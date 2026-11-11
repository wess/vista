use std::future::Future;
use std::path::Path;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

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
  let handler = Arc::new(handler);
  loop {
    let (stream, _) = listener.accept().await?;
    let handler = handler.clone();
    tokio::spawn(async move {
      let (read, mut write) = stream.into_split();
      let mut lines = BufReader::new(read).lines();
      while let Ok(Some(line)) = lines.next_line().await {
        let response = match serde_json::from_str::<Request>(&line) {
          Ok(request) => handler(request).await,
          Err(err) => Response::error(format!("bad request: {err}")),
        };
        let Ok(mut text) = serde_json::to_string(&response) else {
          break;
        };
        text.push('\n');
        if write.write_all(text.as_bytes()).await.is_err() {
          break;
        }
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
    let live = crate::ipc::status(&id).await.unwrap();
    assert_eq!(live.state, State::Running);
    let answer = crate::ipc::call(&id, &Request::Pause).await.unwrap();
    assert_eq!(answer, Response::Ok);
    let _ = std::fs::remove_file(path);
  }
}
