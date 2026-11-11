//! Talking to Vista Tools inside the guest, over virtio-vsock.
//!
//! The framework hands back a connected socket; it is duplicated so tokio can
//! own it, and one JSON request/response is exchanged per connection.

use std::os::fd::FromRawFd;
use std::sync::{Arc, Mutex};

use block2::RcBlock;
use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2_foundation::NSError;
use objc2_virtualization::{VZVirtioSocketConnection, VZVirtioSocketDevice};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::oneshot;
use vista_kit::guest::{Request, Response as GuestResponse, PORT};
use vista_kit::ipc::Response;

use crate::state::with;

const MISSING: &str = "Vista Tools isn't answering inside this machine. Install it once: on Linux run \
`sudo mount -t virtiofs vistatools /mnt && sudo /mnt/install.sh`; on macOS open \
\"/Volumes/My Shared Files/Vista Tools/install.command\". Until then, use screenshot, \
type_text, press_keys and the mouse tools, which need nothing inside the guest.";

type Sender = Arc<Mutex<Option<oneshot::Sender<Result<i32, String>>>>>;

fn send(tx: &Sender, value: Result<i32, String>) {
  if let Some(tx) = tx.lock().ok().and_then(|mut t| t.take()) {
    let _ = tx.send(value);
  }
}

fn connect(tx: Sender) {
  let device = with(|runner| {
    let machine = runner.machine.clone()?;
    let devices = unsafe { machine.socketDevices() };
    let first = devices.firstObject()?;
    first.downcast::<VZVirtioSocketDevice>().ok()
  })
  .flatten();
  let Some(device) = device else {
    return send(&tx, Err("The machine isn't running.".into()));
  };
  let done = RcBlock::new(move |connection: *mut VZVirtioSocketConnection, err: *mut NSError| {
    match unsafe { Retained::retain(connection) } {
      Some(connection) => {
        let fd = unsafe { libc::dup(connection.fileDescriptor()) };
        unsafe { connection.close() };
        send(&tx, if fd >= 0 { Ok(fd) } else { Err("Couldn't take the guest connection.".into()) });
      }
      None => {
        let _ = unsafe { Retained::retain(err) };
        send(&tx, Err(MISSING.into()));
      }
    }
  });
  unsafe { device.connectToPort_completionHandler(PORT, &done) };
}

pub async fn forward(request: Request) -> Response {
  match exchange(request).await {
    Ok(GuestResponse::Error { message }) => Response::error(message),
    Ok(answer) => Response::Guest { answer },
    Err(err) => Response::error(err),
  }
}

async fn exchange(request: Request) -> Result<GuestResponse, String> {
  let (tx, rx) = oneshot::channel();
  let tx: Sender = Arc::new(Mutex::new(Some(tx)));
  DispatchQueue::main().exec_async(move || connect(tx));
  let fd = tokio::time::timeout(std::time::Duration::from_secs(10), rx)
    .await
    .map_err(|_| MISSING.to_string())?
    .map_err(|_| MISSING.to_string())??;

  let std_stream = unsafe { std::os::unix::net::UnixStream::from_raw_fd(fd) };
  std_stream.set_nonblocking(true).map_err(|e| e.to_string())?;
  let stream = tokio::net::UnixStream::from_std(std_stream).map_err(|e| e.to_string())?;
  let (read, mut write) = stream.into_split();
  let mut line = serde_json::to_string(&request).map_err(|e| e.to_string())?;
  line.push('\n');
  write.write_all(line.as_bytes()).await.map_err(|e| e.to_string())?;
  let mut reader = BufReader::with_capacity(1 << 20, read);
  let mut answer = String::new();
  reader.read_line(&mut answer).await.map_err(|e| e.to_string())?;
  if answer.trim().is_empty() {
    return Err("Vista Tools closed the connection without answering.".into());
  }
  serde_json::from_str(answer.trim()).map_err(|e| format!("Vista Tools answered nonsense: {e}"))
}
