//! Disks, and the installer media attached while a machine is being set up.

use std::path::Path;

use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_foundation::NSArray;
use objc2_virtualization::*;
use vista_kit::bundle::Bundle;
use vista_kit::machine::{Bus, Installer};

use crate::util::{describe, url};

unsafe fn attachment(
  path: &Path,
  readonly: bool,
) -> Result<Retained<VZStorageDeviceAttachment>, String> {
  let attachment =
    VZDiskImageStorageDeviceAttachment::initWithURL_readOnly_cachingMode_synchronizationMode_error(
      VZDiskImageStorageDeviceAttachment::alloc(),
      &url(path),
      readonly,
      VZDiskImageCachingMode::Automatic,
      VZDiskImageSynchronizationMode::Full,
    )
    .map_err(|err| format!("Couldn't open {}: {}", path.display(), describe(&err)))?;
  Ok(Retained::into_super(attachment))
}

unsafe fn device(
  attachment: &VZStorageDeviceAttachment,
  bus: Bus,
) -> Retained<VZStorageDeviceConfiguration> {
  match bus {
    Bus::Virtio => Retained::into_super(VZVirtioBlockDeviceConfiguration::initWithAttachment(
      VZVirtioBlockDeviceConfiguration::alloc(),
      attachment,
    )),
    Bus::Nvme => Retained::into_super(
      VZNVMExpressControllerDeviceConfiguration::initWithAttachment(
        VZNVMExpressControllerDeviceConfiguration::alloc(),
        attachment,
      ),
    ),
    Bus::Usb => Retained::into_super(VZUSBMassStorageDeviceConfiguration::initWithAttachment(
      VZUSBMassStorageDeviceConfiguration::alloc(),
      attachment,
    )),
  }
}

pub unsafe fn apply(
  vm: &VZVirtualMachineConfiguration,
  bundle: &Bundle,
  install: bool,
) -> Result<(), String> {
  let mut devices = Vec::new();
  for disk in &bundle.config.disks {
    let path = bundle.file(&disk.file);
    devices.push(device(&*attachment(&path, disk.readonly)?, disk.bus));
  }
  // Installer media rides on USB, the way a person would plug in a stick:
  // every guest's firmware boots from it, and Windows Setup scans removable
  // drives for its answer file.
  if install {
    if let Some(Installer::Iso { path, extras }) = &bundle.config.installer {
      for media in std::iter::once(path).chain(extras.iter()) {
        let media = Path::new(media);
        if !media.exists() {
          return Err(format!("The installer {} is missing.", media.display()));
        }
        let writable = bundle.config.os == vista_kit::machine::Os::Windows
          && media == bundle.file("install.img");
        devices.push(device(&*attachment(media, !writable)?, Bus::Usb));
      }
    }
  }
  vm.setStorageDevices(&NSArray::from_retained_slice(&devices));
  Ok(())
}
