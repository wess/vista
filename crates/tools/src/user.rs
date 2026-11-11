//! Who is at the guest's console: the user commands run as by default.

pub struct User {
  pub name: String,
  pub uid: u32,
  pub gid: u32,
  pub home: String,
}

fn lookup(name: &str) -> Option<User> {
  let cname = std::ffi::CString::new(name).ok()?;
  let entry = unsafe { libc::getpwnam(cname.as_ptr()) };
  if entry.is_null() {
    return None;
  }
  let entry = unsafe { &*entry };
  Some(User {
    name: name.to_string(),
    uid: entry.pw_uid,
    gid: entry.pw_gid,
    home: unsafe { std::ffi::CStr::from_ptr(entry.pw_dir) }.to_string_lossy().to_string(),
  })
}

/// macOS: the owner of /dev/console. Linux: the first regular account
/// (uid 1000+) with a login shell.
pub fn console() -> Option<User> {
  if cfg!(target_os = "macos") {
    use std::os::unix::fs::MetadataExt;
    let uid = std::fs::metadata("/dev/console").ok()?.uid();
    if uid == 0 {
      return None;
    }
    let entry = unsafe { libc::getpwuid(uid) };
    if entry.is_null() {
      return None;
    }
    let name = unsafe { std::ffi::CStr::from_ptr((*entry).pw_name) }.to_string_lossy().to_string();
    return lookup(&name);
  }
  let passwd = std::fs::read_to_string("/etc/passwd").ok()?;
  passwd.lines().find_map(|line| {
    let fields: Vec<&str> = line.split(':').collect();
    let uid: u32 = fields.get(2)?.parse().ok()?;
    let shell = fields.get(6).copied().unwrap_or("");
    if (1000..60000).contains(&uid) && !shell.ends_with("nologin") && !shell.ends_with("false") {
      lookup(fields[0])
    } else {
      None
    }
  })
}
