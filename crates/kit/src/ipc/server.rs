use std::future::Future;
use std::path::Path;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::Semaphore;

use super::{Request, Response};

/// Listen on `path` and answer each request line with `handler`. Runs until
/// the process exits. A stale socket from a crashed run is replaced.
pub async fn serve<F, Fut>(path: &Path, handler: F) -> std::io::Result<()>
where
  F: Fn(Request) -> Fut + Send + Sync + 'static,
  Fut: Future<Output = Response> + Send,
{
  if let Some(dir) = path.parent() {
    std::fs::create_dir_all(dir)?;
  }
  let _ = std::fs::remove_file(path);
  let listener = UnixListener::bind(path)?;
  use std::os::unix::fs::PermissionsExt;
  std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
  let handler = Arc::new(handler);
  let connections = Arc::new(Semaphore::new(32));
  loop {
    let (stream, _) = listener.accept().await?;
    let Ok(permit) = connections.clone().try_acquire_owned() else {
      continue;
    };
    let handler = handler.clone();
    tokio::spawn(async move {
      let _permit = permit;
      let (read, mut write) = stream.into_split();
      let mut reader = BufReader::new(read).take(crate::guest::MAX_MESSAGE + 1);
      let mut line = String::new();
      if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
        return;
      }
      let response = if line.len() as u64 > crate::guest::MAX_MESSAGE {
        Response::error("request is too large")
      } else {
        match serde_json::from_str::<Request>(line.trim()) {
          Ok(request) => handler(request).await,
          Err(err) => Response::error(format!("bad request: {err}")),
        }
      };
      let Ok(mut text) = serde_json::to_string(&response) else {
        return;
      };
      text.push('\n');
      if text.len() as u64 <= crate::guest::MAX_MESSAGE {
        let _ = write.write_all(text.as_bytes()).await;
      }
    });
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::machine::{Live, State};

  #[tokio::test]
  async fn request_round_trip() {
    use std::os::unix::fs::PermissionsExt;

    let id = format!("t{}", &uuid::Uuid::new_v4().simple().to_string()[..7]);
    let path = crate::paths::socket(&id);
    tokio::spawn({
      let path = path.clone();
      async move {
        serve(&path, |request| async move {
          match request {
            Request::Status => Response::Status(Live {
              state: State::Running,
              ..Live::default()
            }),
            _ => Response::Ok,
          }
        })
        .await
      }
    });
    for _ in 0..50 {
      if path.exists() {
        break;
      }
      tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(
      std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
      0o600
    );
    let live = crate::ipc::status(&id).await.unwrap();
    assert_eq!(live.state, State::Running);
    let answer = crate::ipc::call(&id, &Request::Pause).await.unwrap();
    assert_eq!(answer, Response::Ok);
    let _ = std::fs::remove_file(path);
  }
}
