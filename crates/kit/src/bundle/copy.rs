//! Copy-on-write copies. On APFS `clonefile` makes a copy that shares every
//! block with the original until one of them changes, so a 64 GB disk clones
//! in milliseconds and costs nothing until the guest writes.

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

/// Clone a file or a whole directory tree. Falls back to a plain copy on a
/// volume without clone support.
pub fn clone_path(from: &Path, to: &Path) -> std::io::Result<()> {
  let src = CString::new(from.as_os_str().as_bytes())?;
  let dst = CString::new(to.as_os_str().as_bytes())?;
  // CLONE_NOFOLLOW (1): never clone through a symlink.
  let status = unsafe { libc::clonefile(src.as_ptr(), dst.as_ptr(), 1) };
  if status == 0 {
    return Ok(());
  }
  let err = std::io::Error::last_os_error();
  match err.raw_os_error() {
    Some(libc::ENOTSUP) | Some(libc::EXDEV) => copy_tree(from, to),
    _ => Err(err),
  }
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
  if from.is_dir() {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
      let entry = entry?;
      copy_tree(&entry.path(), &to.join(entry.file_name()))?;
    }
    Ok(())
  } else {
    std::fs::copy(from, to).map(|_| ())
  }
}
