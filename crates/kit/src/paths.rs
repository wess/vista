//! Where Vista keeps things.
//!
//! Everything lives under `~/Library/Application Support/Vista` unless the
//! person points machines somewhere else in Settings. Sockets get their own
//! short directory because a Unix socket path is capped at 104 bytes, and a
//! machine bundle's path can be longer than that on its own.

use std::path::PathBuf;

pub fn home() -> PathBuf {
  std::env::var_os("HOME")
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from("/tmp"))
}

/// `~/Library/Application Support/Vista`, or `$VISTA_SUPPORT` (for
/// development and tests).
pub fn support() -> PathBuf {
  if let Some(dir) = std::env::var_os("VISTA_SUPPORT") {
    return PathBuf::from(dir);
  }
  home()
    .join("Library")
    .join("Application Support")
    .join("Vista")
}

/// The default home for machine bundles.
pub fn machines() -> PathBuf {
  support().join("machines")
}

/// Downloaded installers: ISOs, IPSWs, and the drivers Windows needs.
pub fn images() -> PathBuf {
  support().join("images")
}

/// Machine creation jobs: one JSON file each, shared by the app and the MCP
/// server.
pub fn jobs() -> PathBuf {
  support().join("jobs")
}

/// The library database.
pub fn database() -> PathBuf {
  support().join("vista.db")
}

/// Control sockets for running machines, one per machine.
pub fn run() -> PathBuf {
  PathBuf::from("/tmp").join(format!("vista{}", unsafe { libc::getuid() }))
}

pub fn socket(id: &str) -> PathBuf {
  run().join(format!("{}.sock", short(id)))
}

/// The first eight characters of a machine id: unique enough for a socket
/// name, short enough to stay well under the socket path limit.
pub fn short(id: &str) -> &str {
  let end = id
    .char_indices()
    .nth(8)
    .map(|(i, _)| i)
    .unwrap_or(id.len());
  &id[..end]
}

pub fn ensure() -> std::io::Result<()> {
  std::fs::create_dir_all(support())?;
  std::fs::create_dir_all(machines())?;
  std::fs::create_dir_all(images())?;
  std::fs::create_dir_all(jobs())?;
  std::fs::create_dir_all(run())?;
  use std::os::unix::fs::PermissionsExt;
  std::fs::set_permissions(run(), std::fs::Permissions::from_mode(0o700))?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sockets_stay_under_the_path_limit() {
    let id = "0f8fad5b-d9cb-469f-a165-70867728950e";
    assert!(socket(id).as_os_str().len() < 104);
    assert_eq!(short(id), "0f8fad5b");
  }
}
