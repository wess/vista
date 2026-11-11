//! The tokio runtime. gpui owns the main thread and its own executor; the
//! library database, downloads and machine sockets all run on tokio, and
//! their join handles are awaited from gpui tasks.

use std::future::Future;
use std::sync::OnceLock;

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

pub fn get() -> &'static tokio::runtime::Runtime {
  RUNTIME.get_or_init(|| {
    tokio::runtime::Builder::new_multi_thread()
      .worker_threads(4)
      .thread_name("vista-io")
      .enable_all()
      .build()
      .expect("tokio runtime")
  })
}

pub fn spawn<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
  F: Future + Send + 'static,
  F::Output: Send + 'static,
{
  get().spawn(future)
}

pub fn block_on<F: Future>(future: F) -> F::Output {
  get().block_on(future)
}
