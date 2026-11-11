//! A vsock listener with nothing but libc. Linux and macOS lay out
//! `sockaddr_vm` differently (macOS has a length byte), so each gets its own.

use std::io;
use std::os::fd::FromRawFd;

const AF_VSOCK: libc::c_int = 40;
const ANY: u32 = u32::MAX;

#[cfg(target_os = "linux")]
#[repr(C)]
struct Address {
  family: libc::sa_family_t,
  reserved: u16,
  port: u32,
  cid: u32,
  zero: [u8; 4],
}

#[cfg(target_os = "linux")]
fn address(port: u32) -> Address {
  Address {
    family: AF_VSOCK as libc::sa_family_t,
    reserved: 0,
    port,
    cid: ANY,
    zero: [0; 4],
  }
}

#[cfg(not(target_os = "linux"))]
#[repr(C, packed)]
struct Address {
  len: u8,
  family: libc::sa_family_t,
  reserved: u16,
  port: u32,
  cid: u32,
}

#[cfg(not(target_os = "linux"))]
fn address(port: u32) -> Address {
  Address {
    len: std::mem::size_of::<Address>() as u8,
    family: AF_VSOCK as libc::sa_family_t,
    reserved: 0,
    port,
    cid: ANY,
  }
}

pub fn bind(port: u32) -> io::Result<libc::c_int> {
  unsafe {
    let fd = libc::socket(AF_VSOCK, libc::SOCK_STREAM, 0);
    if fd < 0 {
      return Err(io::Error::last_os_error());
    }
    let addr = address(port);
    let rc = libc::bind(
      fd,
      &addr as *const Address as *const libc::sockaddr,
      std::mem::size_of::<Address>() as libc::socklen_t,
    );
    if rc < 0 {
      return Err(io::Error::last_os_error());
    }
    if libc::listen(fd, 16) < 0 {
      return Err(io::Error::last_os_error());
    }
    Ok(fd)
  }
}

pub fn accept(listener: libc::c_int) -> io::Result<std::fs::File> {
  let fd = unsafe { libc::accept(listener, std::ptr::null_mut(), std::ptr::null_mut()) };
  if fd < 0 {
    return Err(io::Error::last_os_error());
  }
  Ok(unsafe { std::fs::File::from_raw_fd(fd) })
}
