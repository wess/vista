//! `config.json` — everything needed to rebuild a machine's hardware.
//!
//! The file lives in the machine's bundle, not the library database, so a
//! bundle copied to another Mac is still a whole machine. The database only
//! remembers where bundles are.

use serde::{Deserialize, Serialize};

use super::Os;

/// Bumped when a field changes meaning. Readers accept anything at or below.
pub const FORMAT: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
  #[serde(default = "format")]
  pub format: u32,
  pub id: String,
  pub name: String,
  pub os: Os,
  /// The distribution or product, for the icon and the subtitle: `ubuntu`,
  /// `fedora`, `macos`, `windows`, `freebsd`…
  #[serde(default)]
  pub family: String,
  /// Human version, e.g. "26.04 LTS" or "27.2.1".
  #[serde(default)]
  pub release: String,
  pub cpus: u32,
  /// MiB.
  pub memory: u64,
  pub disks: Vec<Disk>,
  #[serde(default)]
  pub display: Display,
  #[serde(default)]
  pub network: Network,
  /// Locally administered MAC address, fixed for the machine's life so its
  /// DHCP lease (and so its IP address) survives restarts.
  pub mac: String,
  #[serde(default)]
  pub shares: Vec<Share>,
  #[serde(default = "yes")]
  pub audio: bool,
  #[serde(default)]
  pub microphone: bool,
  /// Linux only: expose Rosetta to the guest for x86-64 binaries.
  #[serde(default)]
  pub rosetta: bool,
  /// Linux only: a SPICE agent port so the guest's `spice-vdagent` can share
  /// the clipboard.
  #[serde(default = "yes")]
  pub clipboard: bool,
  /// Generic EFI machines only (macOS 27 hosts): UEFI Secure Boot with the
  /// default signature databases.
  #[serde(default)]
  pub secureboot: bool,
  #[serde(default)]
  pub nested: bool,
  /// Start in the background when Vista launches at login.
  #[serde(default)]
  pub autostart: bool,
  /// Coding agents (through Vista's MCP server) may control this machine.
  #[serde(default = "yes")]
  pub agents: bool,
  /// Media still attached for installation. Cleared once the guest is set up.
  #[serde(default)]
  pub installer: Option<Installer>,
  /// Seconds since the epoch.
  #[serde(default)]
  pub created: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Disk {
  /// File name inside the bundle.
  pub file: String,
  /// Bytes.
  pub size: u64,
  #[serde(default)]
  pub bus: Bus,
  #[serde(default)]
  pub readonly: bool,
}

/// How a disk is presented to the guest. Windows has no inbox virtio-blk
/// driver, so its system disk goes on NVMe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Bus {
  #[default]
  Virtio,
  Nvme,
  Usb,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Display {
  pub width: u32,
  pub height: u32,
  /// Pixels per inch, for macOS guests (Retina is ~220).
  pub ppi: u32,
}

impl Default for Display {
  fn default() -> Self {
    Display {
      width: 1920,
      height: 1200,
      ppi: 144,
    }
  }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum Network {
  /// The Mac shares its connection; the guest gets a 192.168.64.x address.
  #[default]
  Shared,
  /// The guest sits on the Mac's physical network. Needs the
  /// `com.apple.vm.networking` entitlement, which Apple grants per developer.
  Bridged {
    interface: String,
  },
  None,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Share {
  pub path: String,
  /// The name the folder has inside the guest.
  pub name: String,
  #[serde(default)]
  pub readonly: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Installer {
  /// A macOS restore image, installed by the Virtualization framework before
  /// first boot.
  Ipsw {
    path: String,
    /// macOS 27 hosts can create the first account and skip Setup Assistant.
    #[serde(default)]
    account: Option<Account>,
  },
  /// A bootable ISO attached as a USB drive, plus any helper images (the
  /// Windows answer file, drivers).
  Iso {
    path: String,
    #[serde(default)]
    extras: Vec<String>,
  },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
  pub fullname: String,
  pub username: String,
  pub password: String,
  #[serde(default = "yes")]
  pub autologin: bool,
  #[serde(default)]
  pub ssh: bool,
}

fn format() -> u32 {
  FORMAT
}

fn yes() -> bool {
  true
}

impl Config {
  pub fn new(name: impl Into<String>, os: Os) -> Self {
    let (cpus, memory, disk) = os.defaults();
    let bus = if os == Os::Windows {
      Bus::Nvme
    } else {
      Bus::Virtio
    };
    Config {
      format: FORMAT,
      id: uuid::Uuid::new_v4().to_string(),
      name: name.into(),
      os,
      family: String::new(),
      release: String::new(),
      cpus,
      memory,
      disks: vec![Disk {
        file: "disk.img".into(),
        size: disk * 1024 * 1024 * 1024,
        bus,
        readonly: false,
      }],
      display: Display::default(),
      network: Network::Shared,
      mac: random_mac(),
      shares: Vec::new(),
      audio: true,
      microphone: false,
      rosetta: false,
      clipboard: true,
      secureboot: false,
      nested: false,
      autostart: false,
      agents: true,
      installer: None,
      created: now(),
    }
  }

  /// The system disk, if the machine has one.
  pub fn system(&self) -> Option<&Disk> {
    self.disks.first()
  }

  pub fn summary(&self) -> String {
    let disk = self
      .system()
      .map(|d| format!(" · {} GB", d.size / (1024 * 1024 * 1024)))
      .unwrap_or_default();
    format!(
      "{} CPU · {} GB{}",
      self.cpus,
      (self.memory as f64 / 1024.0).round() as u64,
      disk
    )
  }

  pub fn title(&self) -> String {
    let family = crate::catalog::family_name(&self.family).unwrap_or(self.os.label());
    if self.release.is_empty() {
      family.to_string()
    } else {
      format!("{family} {}", self.release)
    }
  }
}

pub fn now() -> i64 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_secs() as i64)
    .unwrap_or(0)
}

/// A random, locally administered, unicast MAC address.
pub fn random_mac() -> String {
  let bytes = *uuid::Uuid::new_v4().as_bytes();
  let first = (bytes[0] & 0b1111_1100) | 0b0000_0010;
  format!(
    "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
    first, bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn macs_are_local_and_unicast() {
    for _ in 0..64 {
      let mac = random_mac();
      let first = u8::from_str_radix(&mac[..2], 16).unwrap();
      assert_eq!(first & 0b10, 0b10, "locally administered");
      assert_eq!(first & 0b01, 0, "unicast");
    }
  }

  #[test]
  fn config_round_trips() {
    let config = Config::new("Test", Os::Linux);
    let json = serde_json::to_string(&config).unwrap();
    let back: Config = serde_json::from_str(&json).unwrap();
    assert_eq!(config, back);
  }

  #[test]
  fn windows_boots_from_nvme() {
    let config = Config::new("Win", Os::Windows);
    assert_eq!(config.system().unwrap().bus, Bus::Nvme);
  }
}
