use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::watch;

use super::hash::hex;
use crate::{Error, Result};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
  #[default]
  Connecting,
  Downloading,
  Verifying,
  Done,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Progress {
  pub phase: Phase,
  pub received: u64,
  pub total: Option<u64>,
}

impl Progress {
  pub fn fraction(&self) -> Option<f64> {
    self
      .total
      .filter(|t| *t > 0)
      .map(|t| (self.received as f64 / t as f64).clamp(0.0, 1.0))
  }
}

/// A download in flight: watch its progress, cancel it.
#[derive(Clone)]
pub struct Transfer {
  pub progress: watch::Receiver<Progress>,
  sender: Arc<watch::Sender<Progress>>,
  cancelled: Arc<AtomicBool>,
}

impl Default for Transfer {
  fn default() -> Self {
    Self::new()
  }
}

impl Transfer {
  pub fn new() -> Self {
    let (sender, progress) = watch::channel(Progress::default());
    Transfer {
      progress,
      sender: Arc::new(sender),
      cancelled: Arc::new(AtomicBool::new(false)),
    }
  }

  pub fn cancel(&self) {
    self.cancelled.store(true, Ordering::SeqCst);
  }

  pub fn is_cancelled(&self) -> bool {
    self.cancelled.load(Ordering::SeqCst)
  }

  fn report(&self, progress: Progress) {
    let _ = self.sender.send(progress);
  }
}

pub fn client() -> reqwest::Client {
  reqwest::Client::builder()
    .user_agent(concat!("Vista/", env!("CARGO_PKG_VERSION")))
    .connect_timeout(std::time::Duration::from_secs(20))
    .build()
    .expect("http client")
}

/// Download `url` to `dest`, resuming a `.part` file left by an earlier
/// attempt, and check the SHA-256 when one is known. The finished file only
/// appears at `dest` once it is complete and verified.
pub async fn fetch(
  client: &reqwest::Client,
  url: &str,
  dest: &Path,
  sha256: Option<&str>,
  transfer: &Transfer,
) -> Result<PathBuf> {
  if let Some(dir) = dest.parent() {
    tokio::fs::create_dir_all(dir).await?;
  }
  let part = dest.with_extension(format!(
    "{}.part",
    dest.extension().and_then(|e| e.to_str()).unwrap_or("download")
  ));

  // Hash what is already on disk so the digest covers the whole file.
  let mut hasher = Sha256::new();
  let mut have = 0u64;
  if let Ok(mut file) = tokio::fs::File::open(&part).await {
    let mut buffer = vec![0u8; 4 << 20];
    loop {
      let n = file.read(&mut buffer).await?;
      if n == 0 {
        break;
      }
      hasher.update(&buffer[..n]);
      have += n as u64;
    }
  }

  transfer.report(Progress {
    phase: Phase::Connecting,
    received: have,
    total: None,
  });
  let mut request = client.get(url);
  if have > 0 {
    request = request.header(reqwest::header::RANGE, format!("bytes={have}-"));
  }
  let response = request.send().await?;
  let status = response.status();
  if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
    // The part file is already complete (or bigger than the file): start over.
    let _ = tokio::fs::remove_file(&part).await;
    return Box::pin(fetch(client, url, dest, sha256, transfer)).await;
  }
  if !status.is_success() {
    return Err(Error::msg(format!("{url} answered {status}")));
  }
  let resumed = status == reqwest::StatusCode::PARTIAL_CONTENT;
  if !resumed && have > 0 {
    hasher = Sha256::new();
    have = 0;
  }
  let total = response.content_length().map(|len| len + have);

  let mut file = tokio::fs::OpenOptions::new()
    .create(true)
    .write(true)
    .append(resumed)
    .truncate(!resumed)
    .open(&part)
    .await?;
  let mut received = have;
  let mut stream = response.bytes_stream();
  let mut last = std::time::Instant::now();
  while let Some(chunk) = stream.next().await {
    if transfer.is_cancelled() {
      file.flush().await?;
      return Err(Error::msg("Download cancelled."));
    }
    let chunk = chunk?;
    hasher.update(&chunk);
    file.write_all(&chunk).await?;
    received += chunk.len() as u64;
    if last.elapsed().as_millis() >= 100 {
      last = std::time::Instant::now();
      transfer.report(Progress {
        phase: Phase::Downloading,
        received,
        total,
      });
    }
  }
  file.flush().await?;
  drop(file);

  transfer.report(Progress {
    phase: Phase::Verifying,
    received,
    total,
  });
  if let Some(total) = total {
    if received != total {
      return Err(Error::msg(format!(
        "Download stopped early ({received} of {total} bytes). Try again to resume."
      )));
    }
  }
  if let Some(expected) = sha256 {
    let digest = hex(&hasher.finalize());
    if !digest.eq_ignore_ascii_case(expected.trim()) {
      let _ = tokio::fs::remove_file(&part).await;
      return Err(Error::msg(
        "The download didn't match its published checksum and was discarded.",
      ));
    }
  }
  tokio::fs::rename(&part, dest).await?;
  transfer.report(Progress {
    phase: Phase::Done,
    received,
    total,
  });
  Ok(dest.to_path_buf())
}
