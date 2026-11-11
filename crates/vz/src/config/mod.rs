//! A bundle, as hardware.

mod devices;
mod platform;
mod storage;

pub use devices::{bridgeable, rosetta_installed};

use objc2::rc::Retained;
use objc2_virtualization::VZVirtualMachineConfiguration;
use vista_kit::bundle::Bundle;

use crate::util::describe;

/// A validated configuration and what it allows.
pub struct Built {
  pub configuration: Retained<VZVirtualMachineConfiguration>,
  /// The machine's memory can be saved to disk (suspend, live snapshots).
  pub can_save: bool,
}

/// Build and validate the hardware for a bundle. `install` attaches the
/// pending installer media.
pub fn build(bundle: &Bundle, install: bool) -> Result<Built, String> {
  let config = &bundle.config;
  unsafe {
    let vm = VZVirtualMachineConfiguration::new();
    let cpus = (config.cpus as usize).clamp(
      VZVirtualMachineConfiguration::minimumAllowedCPUCount(),
      VZVirtualMachineConfiguration::maximumAllowedCPUCount(),
    );
    vm.setCPUCount(cpus);
    let memory = (config.memory * 1024 * 1024).clamp(
      VZVirtualMachineConfiguration::minimumAllowedMemorySize(),
      VZVirtualMachineConfiguration::maximumAllowedMemorySize(),
    );
    vm.setMemorySize(memory - memory % (1024 * 1024));

    platform::apply(&vm, bundle)?;
    storage::apply(&vm, bundle, install)?;
    devices::apply(&vm, bundle)?;

    vm.validateWithError()
      .map_err(|err| format!("This machine's hardware isn't valid: {}", describe(&err)))?;
    let can_save = vm.validateSaveRestoreSupportWithError().is_ok();
    Ok(Built {
      configuration: vm,
      can_save,
    })
  }
}
