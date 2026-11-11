//! What a machine is: its configuration on disk, and its live state while a
//! `vistamachine` process runs it.

mod config;
mod live;
mod os;

pub use config::{now, random_mac, Account, Bus, Config, Disk, Display, Installer, Network, Share, FORMAT};
pub use live::{Live, State};
pub use os::Os;
