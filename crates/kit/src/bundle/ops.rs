use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use super::files::*;
use crate::machine::{random_mac, Config, FORMAT};
use crate::{Error, Result};

/// A machine bundle that has been read from disk.
#[derive(Clone, Debug, PartialEq)]
pub struct Bundle {
  pub path: PathBuf,
  pub config: Config,
}

impl Bundle {
  pub fn file(&self, name: &str) -> PathBuf {
    self.path.join(name)
  }

  pub fn is_suspended(&self) -> bool {
    self.file(SAVED).exists()
  }
}

/// Read `config.json` from a bundle.
pub fn load(path: &Path) -> Result<Bundle> {
  let text = std::fs::read_to_string(path.join(CONFIG))
    .map_err(|err| Error::msg(format!("{}: {err}", path.display())))?;
  let config: Config = serde_json::from_str(&text)?;
  if config.format > FORMAT {
    return Err(Error::msg(format!(
      "{} was made by a newer Vista (format {})",
      config.name, config.format
    )));
  }
  Ok(Bundle {
    path: path.to_path_buf(),
    config,
  })
}

/// Write `config.json` atomically: a reader never sees half a file.
pub fn save(path: &Path, config: &Config) -> Result<()> {
  let text = serde_json::to_string_pretty(config)?;
  let temp = path.join(format!("{CONFIG}.new"));
  std::fs::write(&temp, text)?;
  std::fs::rename(&temp, path.join(CONFIG))?;
  Ok(())
}

/// Pick a bundle path in `location` that doesn't exist yet.
fn fresh(location: &Path, name: &str) -> PathBuf {
  let base: String = name
    .chars()
    .map(|c| if c == '/' || c == ':' { ' ' } else { c })
    .collect::<String>()
    .trim()
    .to_string();
  let base = if base.is_empty() { "Machine".into() } else { base };
  let mut path = location.join(format!("{base}.{EXTENSION}"));
  let mut n = 2;
  while path.exists() {
    path = location.join(format!("{base} {n}.{EXTENSION}"));
    n += 1;
  }
  path
}

/// Make a new bundle with empty sparse disks.
pub fn create(location: &Path, config: &Config) -> Result<Bundle> {
  std::fs::create_dir_all(location)?;
  let path = fresh(location, &config.name);
  std::fs::create_dir_all(&path)?;
  for disk in &config.disks {
    let file = std::fs::File::create(path.join(&disk.file))?;
    // A sparse file: APFS allocates nothing until the guest writes.
    file.set_len(disk.size)?;
  }
  save(&path, config)?;
  Ok(Bundle {
    path,
    config: config.clone(),
  })
}

/// Copy a machine. The copy shares blocks with the original (APFS), gets a new
/// id and MAC address, and drops the platform identity so the runner mints a
/// fresh one — two machines with one identity confuse the guest.
pub fn duplicate(bundle: &Bundle, name: &str) -> Result<Bundle> {
  if bundle.is_suspended() {
    return Err(Error::msg("Shut down the machine before cloning it."));
  }
  let location = bundle
    .path
    .parent()
    .ok_or_else(|| Error::msg("bundle has no parent directory"))?;
  let path = fresh(location, name);
  super::clone_path(&bundle.path, &path)?;
  let _ = std::fs::remove_dir_all(path.join(SNAPSHOTS));
  let _ = std::fs::remove_file(path.join(MACHINEID));
  let _ = std::fs::remove_file(path.join(LOG));
  let mut config = bundle.config.clone();
  config.id = uuid::Uuid::new_v4().to_string();
  config.name = name.to_string();
  config.mac = random_mac();
  config.created = crate::machine::now();
  save(&path, &config)?;
  Ok(Bundle { path, config })
}

/// Move a bundle to the Trash, or delete it when the Trash is on another
/// volume.
pub fn delete(bundle: &Bundle) -> Result<()> {
  let trash = crate::paths::home().join(".Trash");
  let name = bundle
    .path
    .file_name()
    .ok_or_else(|| Error::msg("bundle has no name"))?;
  let mut target = trash.join(name);
  let mut n = 2;
  while target.exists() {
    target = trash.join(format!("{} {n}", name.to_string_lossy()));
    n += 1;
  }
  if std::fs::rename(&bundle.path, &target).is_err() {
    std::fs::remove_dir_all(&bundle.path)?;
  }
  Ok(())
}

/// Bytes the bundle really occupies. Sparse disks report their full size to
/// `len()`; allocated blocks are what the Mac actually spends.
pub fn usage(path: &Path) -> u64 {
  fn walk(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
      return 0;
    };
    if meta.is_dir() {
      std::fs::read_dir(path)
        .map(|entries| entries.flatten().map(|e| walk(&e.path())).sum())
        .unwrap_or(0)
    } else {
      meta.blocks() * 512
    }
  }
  walk(path)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::machine::Os;

  fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vistatest{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
  }

  #[test]
  fn create_load_duplicate() {
    let dir = scratch();
    let mut config = Config::new("Box", Os::Linux);
    config.disks[0].size = 1024 * 1024;
    let bundle = create(&dir, &config).unwrap();
    assert!(bundle.path.ends_with("Box.vista"));
    let loaded = load(&bundle.path).unwrap();
    assert_eq!(loaded.config, config);
    // Sparse: the full size is reported, almost nothing is allocated.
    let disk = bundle.file("disk.img");
    assert_eq!(std::fs::metadata(&disk).unwrap().len(), 1024 * 1024);

    let copy = duplicate(&loaded, "Box").unwrap();
    assert!(copy.path.ends_with("Box 2.vista"));
    assert_ne!(copy.config.id, config.id);
    assert_ne!(copy.config.mac, config.mac);
    std::fs::remove_dir_all(dir).unwrap();
  }
}
