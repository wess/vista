use std::path::PathBuf;
use std::time::Duration;

use super::{build, save, Job, Phase};
use crate::download::{fetch, Phase as Fetch, Transfer};
use crate::machine::Os;
use crate::store::{Image, Store};
use crate::{Error, Result};

pub(super) fn cancel_marker(id: &str) -> PathBuf {
  crate::paths::jobs().join(format!("{}.cancel", crate::paths::short(id)))
}

/// Ask a running job to stop. It notices within a second.
pub fn cancel(id: &str) {
  let _ = std::fs::write(cancel_marker(id), b"");
  if let Some(mut job) = super::load(id) {
    if !job.phase.is_finished() && (job.pid == 0 || job.is_orphaned()) {
      job.transition(Phase::Cancelled, "Cancelled.".into());
      let _ = save(&job);
    }
  }
}

fn update(job: &mut Job, phase: Phase, message: Option<String>) {
  job.transition(phase, message.unwrap_or_else(|| phase.label().to_string()));
  let _ = save(job);
}

/// Work a job to the end. The job file is updated at every step; the return
/// value is the finished job.
pub async fn run(store: &Store, mut job: Job) -> Job {
  if super::load(&job.id).is_some_and(|stored| stored.phase.is_finished()) {
    return super::load(&job.id).unwrap_or(job);
  }
  job.pid = std::process::id();
  let _ = save(&job);
  let id = job.id.clone();
  let outcome = interrupt(cancel_marker(&id), work(store, &mut job)).await;
  match outcome {
    Ok(()) => {
      if let Some(total) = job.total {
        job.received = total;
      }
      update(&mut job, Phase::Done, Some("Ready".into()))
    }
    Err(err) => {
      let cancelled = cancel_marker(&job.id).exists();
      update(
        &mut job,
        if cancelled {
          Phase::Cancelled
        } else {
          Phase::Failed
        },
        Some(err.to_string()),
      );
    }
  }
  if job.phase != Phase::Cancelled {
    let _ = std::fs::remove_file(cancel_marker(&job.id));
  }
  job
}

async fn work(store: &Store, job: &mut Job) -> Result<()> {
  let client = crate::download::client();
  let settings = store.settings().await?;
  let spec = job.spec.clone();

  // 1. The installer: a local file, a kept download, or a fresh one.
  let (entry, release, installer) = if let Some(file) = &spec.file {
    let path = PathBuf::from(file);
    if !path.exists() {
      return Err(Error::msg(format!("{file} doesn't exist")));
    }
    (None, String::new(), path)
  } else {
    let id = spec
      .system
      .as_deref()
      .ok_or_else(|| Error::msg("Say which system to install, or give an installer file."))?;
    let entry = crate::catalog::find(id).ok_or_else(|| {
      Error::msg(format!(
        "Vista doesn't know a system called \"{id}\". list_systems shows the ones it does."
      ))
    })?;
    update(
      job,
      Phase::Resolving,
      Some(format!("Finding {}", entry.title())),
    );
    let resolved = crate::catalog::resolve_with_status(&client, &entry, &mut |message| {
      update(job, Phase::Resolving, Some(message.to_string()))
    })
    .await
    .map_err(|error| {
      Error::msg(format!(
        "Could not find {}: {error}. Try again, or choose an installer already on your Mac.",
        entry.title()
      ))
    })?;
    let kept = store.images().await?.into_iter().find(|image| {
      image.url.split('?').next() == resolved.url.split('?').next()
        || (image.catalog == entry.id
          && image.version == resolved.version
          && entry.os == Os::Windows)
    });
    let path = match kept {
      Some(image) => PathBuf::from(image.path),
      None => {
        let dest = crate::paths::images().join(&resolved.filename);
        job.total = resolved.size;
        update(
          job,
          Phase::Downloading,
          Some(format!("Downloading {} from its publisher.", entry.title())),
        );
        let transfer = Transfer::new();
        let mut progress = transfer.progress.clone();
        let download = fetch(
          &client,
          &resolved.url,
          &dest,
          resolved.sha256.as_deref(),
          &transfer,
        );
        let mut sampled = std::time::Instant::now();
        let mut previous = None;
        tokio::pin!(download);
        let fetched = loop {
          tokio::select! {
            result = &mut download => break result,
            changed = progress.changed() => {
              if changed.is_err() { continue; }
              let now = progress.borrow().clone();
              if let Some(bytes) = previous {
                let seconds = sampled.elapsed().as_secs_f64();
                if seconds >= 1.0 {
                  job.rate = Some(now.received.saturating_sub(bytes) as f64 / seconds);
                  sampled = std::time::Instant::now();
                  previous = None;
                }
              }
              if previous.is_none() { previous = Some(now.received); }
              job.received = now.received;
              job.total = now.total.or(job.total);
              if now.phase == Fetch::Verifying && job.phase != Phase::Verifying {
                update(job, Phase::Verifying, Some("Checking the installer checksum.".into()));
              } else { let _ = save(job); }
            }
          }
        };
        let path = fetched?;
        if let Some(expected) = resolved.sha1.clone() {
          update(
            job,
            Phase::Verifying,
            Some("Checking the installer against Microsoft’s catalog checksum.".into()),
          );
          let file = path.clone();
          tokio::task::spawn_blocking(move || -> Result<()> {
            use std::io::Read;
            let mut input = std::fs::File::open(&file)?;
            let mut digest = sha1_smol::Sha1::new();
            let mut buffer = vec![0; 4 * 1024 * 1024];
            loop {
              let count = input.read(&mut buffer)?;
              if count == 0 {
                break;
              }
              digest.update(&buffer[..count]);
            }
            if digest.digest().to_string() != expected {
              std::fs::remove_file(&file)?;
              return Err(Error::msg(
                "The installer checksum does not match Microsoft’s catalog. Please try again.",
              ));
            }
            Ok(())
          })
          .await
          .map_err(|error| Error::msg(error.to_string()))??;
        }
        job.received = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        job.total = Some(job.received);
        if settings.keepinstallers {
          store
            .keep_image(&Image {
              id: uuid::Uuid::new_v4().to_string(),
              catalog: entry.id.to_string(),
              name: entry.title(),
              version: resolved.version.clone(),
              url: resolved.url.clone(),
              path: path.to_string_lossy().to_string(),
              size: std::fs::metadata(&path)
                .map(|m| m.len() as i64)
                .unwrap_or(0),
              sha256: resolved.sha256.clone(),
              fetched: crate::machine::now(),
            })
            .await?;
        }
        path
      }
    };
    (Some(entry), resolved.version, path)
  };

  // 2. The bundle.
  update(job, Phase::Creating, Some("Creating the machine".into()));
  let mut config = build::configure(&spec, entry.as_ref(), &release, &installer);
  let location = spec
    .location
    .clone()
    .unwrap_or_else(|| settings.location.clone());
  let mut bundle = crate::bundle::create(std::path::Path::new(&location), &config)?;
  job.machine = Some(config.id.clone());
  let _ = save(job);

  if config.os == Os::Windows {
    update(
      job,
      Phase::Preparing,
      Some("Preparing Windows Setup".into()),
    );
    let account = spec
      .account
      .clone()
      .unwrap_or_else(|| crate::machine::Account {
        fullname: "Vista".into(),
        username: "vista".into(),
        password: "vista".into(),
        autologin: true,
        ssh: false,
      });
    let media = crate::windows::media::prepare(
      &client,
      &installer,
      &bundle.path,
      &config.name,
      &account,
      Some(cancel_marker(&job.id)),
    )
    .await?;
    config.installer = Some(crate::machine::Installer::Iso {
      path: media.to_string_lossy().to_string(),
      extras: Vec::new(),
    });
    crate::bundle::save(&bundle.path, &config)?;
    bundle.config = config.clone();
  }
  store.register(&bundle).await?;

  // 3. Power on.
  if spec.start {
    update(job, Phase::Starting, Some("Starting".into()));
    crate::launch::start(&bundle.path, spec.visible)?;
    store.touch(&config.id).await?;
    match crate::launch::wait_ready(&config.id, Duration::from_secs(30)).await {
      Some(live) if live.state == crate::machine::State::Error => {
        return Err(Error::msg(
          live
            .message
            .unwrap_or_else(|| "The machine failed to start.".into()),
        ))
      }
      Some(_) => {}
      None => {
        return Err(Error::msg(
          "The machine did not start. Check its machine.log for details.",
        ))
      }
    }
  }
  Ok(())
}

async fn cancelled(marker: &std::path::Path) {
  loop {
    if marker.exists() {
      return;
    }
    tokio::time::sleep(Duration::from_millis(250)).await;
  }
}

async fn interrupt<T>(
  marker: PathBuf,
  work: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
  tokio::select! {
    biased;
    _ = cancelled(&marker) => Err(Error::msg("Cancelled.")),
    result = work => result,
  }
}

pub fn spawn(id: &str) -> Result<()> {
  use std::os::unix::process::CommandExt;
  use std::process::{Command, Stdio};
  let executable = std::env::current_exe()?;
  let helper = executable
    .parent()
    .ok_or_else(|| Error::msg("No executable directory"))?
    .join("vistamcp");
  if !helper.is_file() {
    return Err(Error::msg(
      "The creation worker is missing. Reinstall Vista or build vistamcp alongside the app.",
    ));
  }
  let mut command = Command::new(helper);
  command
    .args(["job", id])
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null());
  unsafe {
    command.pre_exec(|| {
      libc::setsid();
      Ok(())
    });
  }
  let mut child = command.spawn()?;
  std::thread::spawn(move || {
    let _ = child.wait();
  });
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn cancellation_interrupts_an_unresponsive_operation() {
    let marker = std::env::temp_dir().join(format!("vistacancel{}", uuid::Uuid::new_v4().simple()));
    let request = marker.clone();
    let trigger = tokio::spawn(async move {
      tokio::time::sleep(Duration::from_millis(20)).await;
      std::fs::write(request, b"").unwrap();
    });
    let result = tokio::time::timeout(
      Duration::from_secs(2),
      interrupt(marker.clone(), std::future::pending::<Result<()>>()),
    )
    .await;
    trigger.await.unwrap();
    let _ = std::fs::remove_file(marker);
    assert_eq!(result.unwrap().unwrap_err().to_string(), "Cancelled.");
  }

  #[tokio::test]
  async fn pending_cancellation_prevents_work_from_starting() {
    let marker = std::env::temp_dir().join(format!("vistacancel{}", uuid::Uuid::new_v4().simple()));
    std::fs::write(&marker, b"").unwrap();
    let mut started = false;
    let result = interrupt(marker.clone(), async {
      started = true;
      Ok(())
    })
    .await;
    let _ = std::fs::remove_file(marker);
    assert!(result.is_err());
    assert!(!started);
  }
}
