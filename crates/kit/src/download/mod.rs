//! Fetching installers: resumable, verified, cancellable.

mod fetch;
mod hash;

pub use fetch::{client, fetch, Phase, Progress, Transfer};
pub use hash::sha256_file;
