//! Snapshots: copy-on-write clones of a bundle's disks and firmware state.
//!
//! A snapshot of a stopped machine is its files. A snapshot of a running one
//! also carries the machine's memory (`suspended.vzvmsave`), written by the
//! runner while the guest is paused, so reverting resumes exactly where the
//! snapshot was taken.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::files::*;
use super::{clone_path, Bundle};
use crate::{Error, Result};

const META: &str = "snapshot.json";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
  pub id: String,
  pub name: String,
  pub created: i64,
  /// Taken while running: reverting resumes the saved memory.
  #[serde(default)]
  pub live: bool,
}

/// The files a snapshot captures besides the disks.
const STATE: [&str; 5] = [CONFIG, NVRAM, AUXILIARY, HARDWARE, MACHINEID];

pub struct Snapshots;

impl Snapshots {
  pub fn dir(bundle: &Bundle) -> PathBuf {
    bundle.file(SNAPSHOTS)
  }

  pub fn list(bundle: &Bundle) -> Vec<Snapshot> {
    let Ok(entries) = std::fs::read_dir(Self::dir(bundle)) else {
      return Vec::new();
    };
    let mut list: Vec<Snapshot> = entries
      .flatten()
      .filter_map(|entry| std::fs::read_to_string(entry.path().join(META)).ok())
      .filter_map(|text| serde_json::from_str(&text).ok())
      .collect();
    list.sort_by_key(|s| std::cmp::Reverse(s.created));
    list
  }

  /// Reserve a directory for a new snapshot and return it. A live snapshot's
  /// runner writes the saved state into it before calling [`Self::capture`].
  pub fn reserve(bundle: &Bundle) -> Result<(String, PathBuf)> {
    let id = uuid::Uuid::new_v4().to_string();
    let dir = Self::dir(bundle).join(crate::paths::short(&id));
    std::fs::create_dir_all(&dir)?;
    Ok((id, dir))
  }

  /// Clone the bundle's disks and state into `dir`. The machine must not be
  /// writing to its disks: stopped, or paused by the runner.
  pub fn capture(bundle: &Bundle, id: &str, dir: &Path, name: &str) -> Result<Snapshot> {
    let mut files: Vec<String> = bundle.config.disks.iter().map(|d| d.file.clone()).collect();
    files.extend(STATE.iter().map(|s| s.to_string()));
    for file in files {
      let from = bundle.file(&file);
      if from.exists() {
        clone_path(&from, &dir.join(&file))?;
      }
    }
    let snapshot = Snapshot {
      id: id.to_string(),
      name: if name.trim().is_empty() {
        format!("Snapshot {}", Self::list(bundle).len() + 1)
      } else {
        name.trim().to_string()
      },
      created: crate::machine::now(),
      live: dir.join(SAVED).exists(),
    };
    std::fs::write(dir.join(META), serde_json::to_string_pretty(&snapshot)?)?;
    Ok(snapshot)
  }

  /// Snapshot a stopped machine.
  pub fn take(bundle: &Bundle, name: &str) -> Result<Snapshot> {
    let (id, dir) = Self::reserve(bundle)?;
    Self::capture(bundle, &id, &dir, name).inspect_err(|_| {
      let _ = std::fs::remove_dir_all(&dir);
    })
  }

  fn find(bundle: &Bundle, id: &str) -> Result<PathBuf> {
    let dir = Self::dir(bundle).join(crate::paths::short(id));
    if dir.join(META).exists() {
      Ok(dir)
    } else {
      Err(Error::msg("That snapshot no longer exists."))
    }
  }

  /// Put the machine back the way the snapshot found it. The machine must be
  /// stopped; a live snapshot leaves it suspended, ready to resume.
  pub fn revert(bundle: &Bundle, id: &str) -> Result<()> {
    let dir = Self::find(bundle, id)?;
    let _ = std::fs::remove_file(bundle.file(SAVED));
    for entry in std::fs::read_dir(&dir)?.flatten() {
      let name = entry.file_name();
      if name == META {
        continue;
      }
      let target = bundle.path.join(&name);
      if target.exists() {
        std::fs::remove_file(&target)?;
      }
      clone_path(&entry.path(), &target)?;
    }
    Ok(())
  }

  pub fn delete(bundle: &Bundle, id: &str) -> Result<()> {
    let dir = Self::find(bundle, id)?;
    std::fs::remove_dir_all(dir)?;
    Ok(())
  }

  pub fn rename(bundle: &Bundle, id: &str, name: &str) -> Result<()> {
    let dir = Self::find(bundle, id)?;
    let text = std::fs::read_to_string(dir.join(META))?;
    let mut snapshot: Snapshot = serde_json::from_str(&text)?;
    snapshot.name = name.trim().to_string();
    std::fs::write(dir.join(META), serde_json::to_string_pretty(&snapshot)?)?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::bundle::create;
  use crate::machine::{Config, Os};

  #[test]
  fn take_and_revert() {
    let dir = std::env::temp_dir().join(format!("vistasnap{}", uuid::Uuid::new_v4()));
    let mut config = Config::new("Snap", Os::Linux);
    config.disks[0].size = 4096;
    let bundle = create(&dir, &config).unwrap();
    std::fs::write(bundle.file("disk.img"), b"before").unwrap();
    let snap = Snapshots::take(&bundle, "").unwrap();
    assert_eq!(snap.name, "Snapshot 1");
    assert!(!snap.live);

    std::fs::write(bundle.file("disk.img"), b"after").unwrap();
    Snapshots::revert(&bundle, &snap.id).unwrap();
    assert_eq!(std::fs::read(bundle.file("disk.img")).unwrap(), b"before");

    assert_eq!(Snapshots::list(&bundle).len(), 1);
    Snapshots::delete(&bundle, &snap.id).unwrap();
    assert!(Snapshots::list(&bundle).is_empty());
    std::fs::remove_dir_all(dir).unwrap();
  }
}
