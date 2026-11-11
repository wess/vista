use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use super::{Request, Response};
use crate::machine::Live;
use crate::{Error, Result};

/// Send one request to a running machine and wait for its answer.
pub async fn call(id: &str, request: &Request) -> Result<Response> {
  let path = crate::paths::socket(id);
  let exchange = async {
    let stream = UnixStream::connect(&path).await?;
    let (read, mut write) = stream.into_split();
    let mut line = serde_json::to_string(request)?;
    line.push('\n');
    write.write_all(line.as_bytes()).await?;
    let mut reader = BufReader::new(read);
    let mut answer = String::new();
    reader.read_line(&mut answer).await?;
    if answer.is_empty() {
      return Err(Error::msg("the machine closed the connection"));
    }
    Ok::<Response, Error>(serde_json::from_str(answer.trim())?)
  };
  // Suspending writes all of the guest's memory to disk before answering.
  let limit = match request {
    Request::Suspend | Request::Snapshot { .. } => Duration::from_secs(300),
    Request::Guest { request } => match request {
      crate::guest::Request::Exec { timeout, .. } => Duration::from_secs(timeout + 15),
      _ => Duration::from_secs(120),
    },
    Request::Type { text } => Duration::from_secs(10 + text.len() as u64 / 10),
    _ => Duration::from_secs(15),
  };
  tokio::time::timeout(limit, exchange)
    .await
    .map_err(|_| Error::msg("the machine didn't answer"))?
}

/// The machine's live state, or `None` when no process is running it.
pub async fn status(id: &str) -> Option<Live> {
  if !crate::paths::socket(id).exists() {
    return None;
  }
  match call(id, &Request::Status).await {
    Ok(Response::Status(live)) => Some(live),
    _ => None,
  }
}
