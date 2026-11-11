//! The platform and boot loader: a Mac for macOS guests, a generic EFI
//! machine for everything else.

use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_foundation::NSData;
use objc2_virtualization::*;
use vista_kit::bundle::{Bundle, AUXILIARY, HARDWARE, MACHINEID, NVRAM};

use crate::util::{describe, read_data, url, write_data};

pub unsafe fn apply(vm: &VZVirtualMachineConfiguration, bundle: &Bundle) -> Result<(), String> {
  if bundle.config.os.is_mac() {
    mac(vm, bundle)
  } else {
    generic(vm, bundle)
  }
}

unsafe fn mac(vm: &VZVirtualMachineConfiguration, bundle: &Bundle) -> Result<(), String> {
  let platform = VZMacPlatformConfiguration::new();

  let model = read_data(&bundle.file(HARDWARE))
    .and_then(|data| VZMacHardwareModel::initWithDataRepresentation(VZMacHardwareModel::alloc(), &data))
    .ok_or("This Mac machine has no hardware model. Was macOS installed?")?;
  if !model.isSupported() {
    return Err("This Mac can't run the macOS version installed in this machine.".into());
  }
  platform.setHardwareModel(&model);

  let aux = bundle.file(AUXILIARY);
  if !aux.exists() {
    return Err("This Mac machine is missing its boot storage.".into());
  }
  let storage = VZMacAuxiliaryStorage::initWithURL(VZMacAuxiliaryStorage::alloc(), &url(&aux));
  platform.setAuxiliaryStorage(Some(&storage));

  let identifier = mac_identifier(bundle);
  platform.setMachineIdentifier(&identifier);

  vm.setPlatform(&platform);
  vm.setBootLoader(Some(&VZMacOSBootLoader::new()));
  Ok(())
}

/// The machine identifier, minted on first use and kept in the bundle.
unsafe fn mac_identifier(bundle: &Bundle) -> Retained<VZMacMachineIdentifier> {
  let path = bundle.file(MACHINEID);
  if let Some(identifier) = read_data(&path)
    .and_then(|data| VZMacMachineIdentifier::initWithDataRepresentation(VZMacMachineIdentifier::alloc(), &data))
  {
    return identifier;
  }
  let identifier = VZMacMachineIdentifier::new();
  let _ = write_data(&path, &identifier.dataRepresentation());
  identifier
}

unsafe fn generic(vm: &VZVirtualMachineConfiguration, bundle: &Bundle) -> Result<(), String> {
  let platform = VZGenericPlatformConfiguration::new();
  let path = bundle.file(MACHINEID);
  let identifier = read_data(&path)
    .and_then(|data: Retained<NSData>| {
      VZGenericMachineIdentifier::initWithDataRepresentation(VZGenericMachineIdentifier::alloc(), &data)
    })
    .unwrap_or_else(|| {
      let identifier = VZGenericMachineIdentifier::new();
      let _ = write_data(&path, &identifier.dataRepresentation());
      identifier
    });
  platform.setMachineIdentifier(&identifier);
  if bundle.config.os == vista_kit::machine::Os::Windows {
    crate::modern::windows_platform(&platform);
  }
  if bundle.config.nested && VZGenericPlatformConfiguration::isNestedVirtualizationSupported() {
    platform.setNestedVirtualizationEnabled(true);
  }
  vm.setPlatform(&platform);

  let nvram = bundle.file(NVRAM);
  let store = if nvram.exists() {
    VZEFIVariableStore::initWithURL(VZEFIVariableStore::alloc(), &url(&nvram))
  } else {
    let store = VZEFIVariableStore::initCreatingVariableStoreAtURL_options_error(
      VZEFIVariableStore::alloc(),
      &url(&nvram),
      VZEFIVariableStoreInitializationOptions::empty(),
    )
    .map_err(|err| format!("Couldn't create the machine's firmware storage: {}", describe(&err)))?;
    if bundle.config.secureboot {
      crate::modern::enable_secure_boot(&store)?;
    }
    store
  };
  let loader = VZEFIBootLoader::new();
  loader.setVariableStore(Some(&store));
  vm.setBootLoader(Some(&loader));
  Ok(())
}
