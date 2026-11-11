//! The control socket. Requests arrive on a background thread with its own
//! tokio runtime; anything that touches the machine hops to the main queue.

use dispatch2::DispatchQueue;
use tokio::sync::oneshot;
use vista_kit::ipc::{Request, Response};

use crate::lifecycle;
use crate::state::with;
use crate::window;

pub fn listen(id: String) {
  std::thread::spawn(move || {
    let runtime = tokio::runtime::Builder::new_multi_thread()
      .worker_threads(2)
      .enable_all()
      .build()
      .expect("control runtime");
    runtime.block_on(async move {
      let path = vista_kit::paths::socket(&id);
      if let Err(err) = vista_kit::ipc::serve(&path, handle).await {
        eprintln!("vistamachine: control socket failed: {err}");
      }
    });
  });
}

async fn handle(request: Request) -> Response {
  match request {
    Request::Screenshot => crate::capture::screenshot().await,
    Request::Key { keys } => crate::input::keys(&keys).await,
    Request::Type { text } => crate::input::type_text(&text).await,
    Request::Mouse { action, x, y, to } => crate::input::mouse(action, x, y, to).await,
    Request::Scroll { x, y, dx, dy } => crate::input::scroll(x, y, dx, dy).await,
    Request::Guest { request } => crate::guest::forward(request).await,
    other => on_main(other).await,
  }
}

async fn on_main(request: Request) -> Response {
  let (tx, rx) = oneshot::channel();
  DispatchQueue::main().exec_async(move || dispatch(request, tx));
  rx.await
    .unwrap_or_else(|_| Response::error("The machine stopped before answering."))
}

fn answer(tx: oneshot::Sender<Response>) -> lifecycle::Reply {
  Box::new(move |result| {
    let _ = tx.send(match result {
      Ok(()) => Response::Ok,
      Err(err) => Response::error(err),
    });
  })
}

fn dispatch(request: Request, tx: oneshot::Sender<Response>) {
  match request {
    Request::Status => {
      let live = with(|runner| runner.live());
      let _ = tx.send(match live {
        Some(live) => Response::Status(live),
        None => Response::error("busy"),
      });
    }
    Request::Show => {
      with(window::show);
      let _ = tx.send(Response::Ok);
    }
    Request::Hide => {
      with(window::hide);
      let _ = tx.send(Response::Ok);
    }
    Request::Stop => answer(tx)(lifecycle::request_stop()),
    Request::Restart => answer(tx)(lifecycle::restart()),
    Request::Kill => lifecycle::kill(answer(tx)),
    Request::Pause => lifecycle::pause(answer(tx)),
    Request::Resume => lifecycle::resume(answer(tx)),
    Request::Suspend => lifecycle::suspend(answer(tx)),
    Request::Snapshot { name } => lifecycle::snapshot(
      name,
      Box::new(move |result| {
        let _ = tx.send(match result {
          Ok(snapshot) => Response::Snapshot(snapshot),
          Err(err) => Response::error(err),
        });
      }),
    ),
    _ => {
      let _ = tx.send(Response::error("unsupported request"));
    }
  }
}

/// Run `f` on the main thread and wait for its result.
pub fn main_sync<R: Send>(f: impl FnOnce() -> R + Send) -> R {
  let mut out = None;
  DispatchQueue::main().exec_sync(|| out = Some(f()));
  out.expect("main queue ran the work")
}
