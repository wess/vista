//! The control protocol between Vista and a running machine.
//!
//! Each `vistamachine` process listens on a Unix socket named for its machine
//! (see `paths::socket`). Requests and responses are single JSON lines. A
//! machine with no socket, or one nobody answers on, is not running.

mod client;
mod protocol;
mod server;

pub use client::{call, status};
pub use protocol::{MouseAction, Request, Response};
pub use server::serve;
