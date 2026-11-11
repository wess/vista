#![no_std]
#![no_main]

use core::{ffi::c_void, ptr};
use efi::protocols::graphics_output as gop;
use vistafirmware::{efi, protocol};

const LIMIT: usize = 8;
const WIDTH: usize = 1920;
const HEIGHT: usize = 1200;
const CAPACITY: usize = WIDTH * HEIGHT * 4;
static mut SERVICES: *mut efi::BootServices = ptr::null_mut();
static mut HANDLES: [*mut gop::Protocol; LIMIT] = [ptr::null_mut(); LIMIT];
static mut QUERIES: [Option<gop::ProtocolQueryMode>; LIMIT] = [None; LIMIT];
static mut MODES: [Option<gop::ProtocolSetMode>; LIMIT] = [None; LIMIT];
static mut COUNT: usize = 0;
static mut DEVICE: *mut gop::Protocol = ptr::null_mut();
static mut DEVICE_BLT: Option<gop::ProtocolBlt> = None;
static mut BUFFER: u64 = 0;
static mut ACTIVE: bool = true;
static mut BUSY: bool = false;
static mut TIMER: efi::Event = ptr::null_mut();

unsafe fn index(device: *mut gop::Protocol) -> Option<usize> {
  (0..COUNT).find(|index| HANDLES[*index] == device)
}

unsafe fn linear(info: *mut gop::ModeInformation) {
  (*info).pixel_format = gop::PIXEL_BLUE_GREEN_RED_RESERVED_8_BIT_PER_COLOR;
  (*info).pixels_per_scan_line = (*info).horizontal_resolution;
}

unsafe fn publish(device: *mut gop::Protocol) {
  let mode = (*device).mode;
  linear((*mode).info);
  (*mode).frame_buffer_base = BUFFER;
  (*mode).frame_buffer_size = ((*(*mode).info).horizontal_resolution as usize)
    * ((*(*mode).info).vertical_resolution as usize)
    * 4;
}

unsafe extern "efiapi" fn query(
  device: *mut gop::Protocol,
  mode: u32,
  size: *mut usize,
  info: *mut *mut gop::ModeInformation,
) -> efi::Status {
  let Some(index) = index(device) else {
    return efi::Status::UNSUPPORTED;
  };
  let status = QUERIES[index].unwrap()(device, mode, size, info);
  if status.is_error() {
    return status;
  }
  if (**info).horizontal_resolution as usize > WIDTH
    || (**info).vertical_resolution as usize > HEIGHT
  {
    ((*SERVICES).free_pool)((*info).cast());
    return efi::Status::UNSUPPORTED;
  }
  linear(*info);
  efi::Status::SUCCESS
}

unsafe extern "efiapi" fn set(device: *mut gop::Protocol, mode: u32) -> efi::Status {
  let Some(index) = index(device) else {
    return efi::Status::UNSUPPORTED;
  };
  let old = ((*SERVICES).raise_tpl)(efi::TPL_NOTIFY);
  let status = MODES[index].unwrap()(device, mode);
  if !status.is_error() {
    let info = (*(*device).mode).info;
    if (*info).horizontal_resolution as usize > WIDTH
      || (*info).vertical_resolution as usize > HEIGHT
    {
      ((*SERVICES).restore_tpl)(old);
      return efi::Status::UNSUPPORTED;
    }
    publish(device);
  }
  ((*SERVICES).restore_tpl)(old);
  status
}

unsafe extern "efiapi" fn flush(_: efi::Event, _: *mut c_void) {
  if !ACTIVE || BUSY || DEVICE.is_null() {
    return;
  }
  BUSY = true;
  let info = &*(*(*DEVICE).mode).info;
  if let Some(blt) = DEVICE_BLT {
    blt(
      DEVICE,
      BUFFER as *mut gop::BltPixel,
      gop::BLT_BUFFER_TO_VIDEO,
      0,
      0,
      0,
      0,
      info.horizontal_resolution as usize,
      info.vertical_resolution as usize,
      info.pixels_per_scan_line as usize * 4,
    );
  }
  BUSY = false;
}

unsafe extern "efiapi" fn finish(_: efi::Event, _: *mut c_void) {
  ACTIVE = false;
  ((*SERVICES).set_timer)(TIMER, efi::TIMER_CANCEL, 0);
}

#[no_mangle]
pub unsafe extern "efiapi" fn efi_main(
  _: efi::Handle,
  table: *mut efi::SystemTable,
) -> efi::Status {
  SERVICES = (*table).boot_services;
  let services = SERVICES;
  let mut guid = gop::PROTOCOL_GUID;
  let mut count = 0;
  let mut handles = ptr::null_mut();
  let status = ((*services).locate_handle_buffer)(
    efi::BY_PROTOCOL,
    &mut guid,
    ptr::null_mut(),
    &mut count,
    &mut handles,
  );
  if status.is_error() {
    return status;
  }
  let status = ((*services).allocate_pages)(
    efi::ALLOCATE_ANY_PAGES,
    efi::RESERVED_MEMORY_TYPE,
    CAPACITY.div_ceil(4096),
    &raw mut BUFFER,
  );
  if status.is_error() {
    return status;
  }
  ptr::write_bytes(BUFFER as *mut u8, 0, CAPACITY);
  for handle in core::slice::from_raw_parts(handles, count) {
    if COUNT == LIMIT {
      break;
    }
    let device = protocol::<gop::Protocol>(services, *handle, gop::PROTOCOL_GUID);
    if device.is_null() || (*device).mode.is_null() {
      continue;
    }
    let info = (*(*device).mode).info;
    if info.is_null() {
      continue;
    }
    if (*info).horizontal_resolution as usize > WIDTH
      || (*info).vertical_resolution as usize > HEIGHT
    {
      let mut selected = false;
      for number in 0..(*(*device).mode).max_mode {
        let mut size = 0;
        let mut candidate = ptr::null_mut();
        if ((*device).query_mode)(device, number, &mut size, &mut candidate).is_error() {
          continue;
        }
        let fits = (*candidate).horizontal_resolution as usize <= WIDTH
          && (*candidate).vertical_resolution as usize <= HEIGHT;
        ((*services).free_pool)(candidate.cast());
        if fits && !((*device).set_mode)(device, number).is_error() {
          selected = true;
          break;
        }
      }
      if !selected {
        continue;
      }
    }
    let path = protocol::<efi::protocols::device_path::Protocol>(
      services,
      *handle,
      efi::protocols::device_path::PROTOCOL_GUID,
    );
    if DEVICE.is_null() || !path.is_null() {
      DEVICE = device;
      DEVICE_BLT = Some((*device).blt);
    }
    HANDLES[COUNT] = device;
    QUERIES[COUNT] = Some((*device).query_mode);
    MODES[COUNT] = Some((*device).set_mode);
    COUNT += 1;
  }
  ((*services).free_pool)(handles.cast());
  if COUNT == 0 {
    return efi::Status::NOT_FOUND;
  }
  for index in 0..COUNT {
    let device = HANDLES[index];
    publish(device);
    (*device).query_mode = query;
    (*device).set_mode = set;
  }
  let status = ((*services).create_event)(
    efi::EVT_TIMER | efi::EVT_NOTIFY_SIGNAL,
    efi::TPL_CALLBACK,
    Some(flush),
    ptr::null_mut(),
    &raw mut TIMER,
  );
  if status.is_error() {
    return status;
  }
  ((*services).set_timer)(TIMER, efi::TIMER_PERIODIC, 400_000);
  let mut exit = ptr::null_mut();
  ((*services).create_event)(
    efi::EVT_SIGNAL_EXIT_BOOT_SERVICES,
    efi::TPL_NOTIFY,
    Some(finish),
    ptr::null_mut(),
    &mut exit,
  )
}
