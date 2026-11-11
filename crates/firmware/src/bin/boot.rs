#![no_std]
#![no_main]

use core::ptr;
use vistafirmware::{efi, load, protocol, say};

#[no_mangle]
pub unsafe extern "efiapi" fn efi_main(
  image: efi::Handle,
  table: *mut efi::SystemTable,
) -> efi::Status {
  let services = (*table).boot_services;
  let loaded = protocol::<efi::protocols::loaded_image::Protocol>(
    services,
    image,
    efi::protocols::loaded_image::PROTOCOL_GUID,
  );
  if loaded.is_null() {
    return efi::Status::LOAD_ERROR;
  }
  let volume = (*loaded).device_handle;
  let driver = match load(services, image, volume, "\\EFI\\vista\\display.efi") {
    Ok(driver) => driver,
    Err(status) => {
      say(table, "Vista: the Windows display bridge is missing.\r\n");
      return status;
    }
  };
  let status = ((*services).start_image)(driver, ptr::null_mut(), ptr::null_mut());
  if status.is_error() {
    say(
      table,
      "Vista: the Windows display bridge could not start.\r\n",
    );
    return status;
  }
  for path in [
    "\\EFI\\BOOT\\windows.efi",
    "\\EFI\\Microsoft\\Boot\\windows.efi",
  ] {
    if let Ok(boot) = load(services, image, volume, path) {
      return ((*services).start_image)(boot, ptr::null_mut(), ptr::null_mut());
    }
  }
  say(table, "Vista: Windows Boot Manager was not found.\r\n");
  efi::Status::NOT_FOUND
}
