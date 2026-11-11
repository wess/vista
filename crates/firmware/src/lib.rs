#![no_std]

use core::{ffi::c_void, ptr};
pub use r_efi::efi;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
  loop {
    core::hint::spin_loop();
  }
}

pub unsafe fn protocol<T>(
  services: *mut efi::BootServices,
  handle: efi::Handle,
  mut guid: efi::Guid,
) -> *mut T {
  let mut interface = ptr::null_mut();
  if ((*services).handle_protocol)(handle, &mut guid, &mut interface).is_error() {
    ptr::null_mut()
  } else {
    interface.cast()
  }
}

pub unsafe fn load(
  services: *mut efi::BootServices,
  parent: efi::Handle,
  volume: efi::Handle,
  path: &str,
) -> Result<efi::Handle, efi::Status> {
  let original = protocol::<efi::protocols::device_path::Protocol>(
    services,
    volume,
    efi::protocols::device_path::PROTOCOL_GUID,
  );
  if original.is_null() {
    return Err(efi::Status::NOT_FOUND);
  }
  let mut prefix = 0usize;
  let mut node = original;
  while (*node).r#type != 0x7f {
    let length = u16::from_le_bytes((*node).length) as usize;
    if length < 4 || prefix + length > 2048 {
      return Err(efi::Status::INVALID_PARAMETER);
    }
    prefix += length;
    node = original.cast::<u8>().add(prefix).cast();
  }
  let units = path.encode_utf16().count() + 1;
  let file_length = 4 + units * 2;
  let mut allocated: *mut c_void = ptr::null_mut();
  let status =
    ((*services).allocate_pool)(efi::LOADER_DATA, prefix + file_length + 4, &mut allocated);
  if status.is_error() {
    return Err(status);
  }
  let bytes = allocated.cast::<u8>();
  ptr::copy_nonoverlapping(original.cast::<u8>(), bytes, prefix);
  let file = bytes.add(prefix);
  *file = 4;
  *file.add(1) = 4;
  ptr::write_unaligned(file.add(2).cast::<u16>(), file_length as u16);
  for (index, unit) in path.encode_utf16().chain(core::iter::once(0)).enumerate() {
    ptr::write_unaligned(file.add(4 + index * 2).cast::<u16>(), unit);
  }
  ptr::copy_nonoverlapping([0x7f, 0xff, 4, 0].as_ptr(), file.add(file_length), 4);
  let mut image = ptr::null_mut();
  let result = ((*services).load_image)(
    efi::Boolean::FALSE,
    parent,
    allocated.cast(),
    ptr::null_mut(),
    0,
    &mut image,
  );
  ((*services).free_pool)(allocated);
  if result.is_error() {
    Err(result)
  } else {
    Ok(image)
  }
}

pub unsafe fn say(table: *mut efi::SystemTable, message: &str) {
  let mut text = [0u16; 200];
  for (index, unit) in message.encode_utf16().take(199).enumerate() {
    text[index] = unit;
  }
  let output = (*table).con_out;
  if !output.is_null() {
    ((*output).output_string)(output, text.as_mut_ptr());
  }
}
