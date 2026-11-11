//! Turning a spec and its installer into a machine configuration.

use std::path::Path;

use super::Spec;
use crate::catalog::Entry;
use crate::machine::{Bus, Config, Installer, Os, Share};

/// What the installer at `path` installs, judging by its name.
pub fn guess_os(path: &Path) -> Os {
  match path
    .extension()
    .and_then(|e| e.to_str())
    .map(str::to_lowercase)
    .as_deref()
  {
    Some("ipsw") => Os::Macos,
    _ => {
      let name = path.to_string_lossy().to_lowercase();
      if name.contains("win") {
        Os::Windows
      } else if ["bsd"].iter().any(|k| name.contains(k)) {
        Os::Other
      } else {
        Os::Linux
      }
    }
  }
}

/// Build the configuration for a new machine.
pub fn configure(spec: &Spec, entry: Option<&Entry>, release: &str, installer: &Path) -> Config {
  let os = entry
    .map(|e| e.os)
    .or(spec.os)
    .unwrap_or_else(|| guess_os(installer));
  let mut config = Config::new(spec.name.trim(), os);
  if config.name.is_empty() {
    config.name = entry
      .map(|e| e.title())
      .unwrap_or_else(|| os.label().to_string());
  }
  config.family = entry
    .map(|e| e.family.to_string())
    .unwrap_or_else(|| match os {
      Os::Macos => "macos".into(),
      Os::Windows => "windows".into(),
      Os::Linux => "linux".into(),
      Os::Other => String::new(),
    });
  config.release = release.to_string();
  if let Some(cpus) = spec.cpus {
    config.cpus = cpus.max(1);
  }
  if let Some(memory) = spec.memory {
    config.memory = memory.max(1024);
  }
  if let Some(disk) = spec.disk {
    config.disks[0].size = disk.max(os.minimum_disk()) * 1024 * 1024 * 1024;
  }
  if spec.sharedownloads {
    let downloads = crate::paths::home().join("Downloads");
    if downloads.is_dir() {
      config.shares.push(Share {
        path: downloads.to_string_lossy().to_string(),
        name: "Downloads".into(),
        readonly: false,
      });
    }
  }
  config.rosetta = spec.rosetta && os == Os::Linux;
  config.display.ppi = if os == Os::Macos { 220 } else { 144 };
  if os == Os::Macos {
    config.display.width = 2880;
    config.display.height = 1800;
  }
  if os == Os::Windows {
    // Windows reads its system disk through the viostor driver injected at
    // setup; the experimental Windows path follows the configuration known
    // to boot on this framework.
    config.disks[0].bus = Bus::Virtio;
    config.audio = false;
  }
  let path = installer.to_string_lossy().to_string();
  config.installer = Some(match os {
    Os::Macos => Installer::Ipsw {
      path,
      account: spec.account.clone(),
    },
    _ => Installer::Iso {
      path,
      extras: Vec::new(),
    },
  });
  config
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn guesses() {
    assert_eq!(guess_os(Path::new("/x/UniversalMac_27.ipsw")), Os::Macos);
    assert_eq!(guess_os(Path::new("/x/Win11_arm64.iso")), Os::Windows);
    assert_eq!(guess_os(Path::new("/x/FreeBSD-15.iso")), Os::Other);
    assert_eq!(guess_os(Path::new("/x/ubuntu.iso")), Os::Linux);
  }

  #[test]
  fn spec_overrides() {
    let spec = Spec {
      name: "Box".into(),
      cpus: Some(6),
      memory: Some(6144),
      disk: Some(2),
      ..Spec::default()
    };
    let config = configure(&spec, None, "1.0", Path::new("/x/ubuntu.iso"));
    assert_eq!(config.cpus, 6);
    assert_eq!(config.memory, 6144);
    // Raised to the minimum for the OS.
    assert_eq!(config.disks[0].size, 16 * 1024 * 1024 * 1024);
    assert!(matches!(config.installer, Some(Installer::Iso { .. })));
  }
}
