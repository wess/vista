//! Vista's shared core.
//!
//! Nothing in this crate knows about AppKit, the Virtualization framework, or
//! gpui. It owns the things every Vista process agrees on: what a machine is
//! (`machine`), how one is laid out on disk (`bundle`), where Vista keeps its
//! files (`paths`), the library index (`store`), the installers Vista can fetch
//! (`catalog`, `download`), and the protocol the app speaks to a running
//! machine (`ipc`).

pub mod bundle;
pub mod catalog;
pub mod download;
pub mod error;
pub mod ipc;
pub mod jobs;
pub mod launch;
pub mod machine;
pub mod net;
pub mod paths;
pub mod store;
pub mod windows;

pub use error::{Error, Result};
pub use vista_guest as guest;
