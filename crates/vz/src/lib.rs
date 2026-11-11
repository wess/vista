//! Vista's bridge to Apple's Virtualization framework.
//!
//! `config` turns a bundle into a `VZVirtualMachineConfiguration`. `install`
//! puts macOS on a fresh disk from a restore image. `modern` reaches the
//! macOS 26 and 27 additions (Secure Boot, guest provisioning) that the
//! published bindings predate, by selector, after checking the host answers.
//!
//! Everything here must run on the main thread: the machine is created on the
//! main queue, so its view and its completion handlers live there too.

pub mod config;
pub mod install;
pub mod modern;
pub mod util;

pub use config::{bridgeable, build, rosetta_installed, Built};
pub use install::{install_macos, InstallProgress};

/// Whether this Mac can run virtual machines at all.
pub fn supported() -> bool {
  unsafe { objc2_virtualization::VZVirtualMachine::isSupported() }
}
