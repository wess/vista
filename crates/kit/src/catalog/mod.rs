//! The systems Vista installs in one click, and how to find each one's
//! current installer.
//!
//! Entries don't hardcode point releases. Each names where its files are
//! published and how to recognise the right one, and `resolve` asks the
//! publisher at the moment someone creates a machine — so "Ubuntu 26.04"
//! means 26.04.1 the day 26.04.1 ships, without a Vista update.

mod linux;
mod listing;
mod macos;
mod resolve;
mod sums;

pub use linux::LINUX;
pub use macos::{macos_versions, MacRelease};
pub use resolve::{resolve, resolve_with_status, Resolved};
pub use sums::{parse_sums, SumFormat};

use crate::machine::Os;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
  /// Fetch a directory index and take the newest file matching `pattern`.
  /// A `{n}` in `index` is replaced by the newest numbered subdirectory of
  /// the path before it (Fedora's `releases/44/`, Rocky's `10.1/`).
  Listing {
    index: &'static str,
    pattern: &'static str,
  },
  /// Read file names out of a checksum listing instead of an HTML index —
  /// for mirrors whose directory pages are unreliable (Debian).
  Sums {
    sums: &'static str,
    pattern: &'static str,
  },
  /// A URL that always points at the current file.
  Direct { url: &'static str },
  /// Pop!_OS publishes builds through a JSON API.
  Pop { api: &'static str },
  /// A macOS restore image from Apple; `None` is the newest.
  Macos { major: Option<u32> },
  /// Windows 11 for Arm, from Microsoft's download service.
  Windows,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checksum {
  /// Absolute, or relative to the directory the ISO was found in. `{file}` is
  /// the ISO's name. A value starting with `^` is a pattern matched against
  /// the same directory listing (Fedora versions its checksum file names).
  pub url: &'static str,
  pub format: SumFormat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
  pub id: &'static str,
  pub os: Os,
  pub family: &'static str,
  pub name: &'static str,
  pub version: &'static str,
  pub edition: &'static str,
  pub source: Source,
  pub checksum: Option<Checksum>,
  /// Boots to a graphical installer or live desktop.
  pub desktop: bool,
  pub notes: &'static str,
}

impl Entry {
  pub fn title(&self) -> String {
    if self.version.is_empty() {
      self.name.to_string()
    } else {
      format!("{} {}", self.name, self.version)
    }
  }
}

pub const WINDOWS: Entry = Entry {
  id: "windows11",
  os: Os::Windows,
  family: "windows",
  name: "Windows 11",
  version: "",
  edition: "Pro",
  source: Source::Windows,
  checksum: None,
  desktop: true,
  notes: "Downloaded from Microsoft. Installs unattended with a local account.",
};

/// Every one-click system, macOS first, then Windows, then Linux and the rest.
pub fn all() -> Vec<Entry> {
  let mut list = macos::ENTRIES.to_vec();
  list.push(WINDOWS);
  list.extend(LINUX.iter().copied());
  list
}

pub fn find(id: &str) -> Option<Entry> {
  all().into_iter().find(|entry| entry.id == id)
}

/// Display name for a family id.
pub fn family_name(family: &str) -> Option<&'static str> {
  Some(match family {
    "macos" => "macOS",
    "windows" => "Windows",
    "ubuntu" => "Ubuntu",
    "fedora" => "Fedora",
    "debian" => "Debian",
    "kali" => "Kali Linux",
    "centos" => "CentOS Stream",
    "rocky" => "Rocky Linux",
    "alma" | "almalinux" => "AlmaLinux",
    "opensuse" => "openSUSE",
    "alpine" => "Alpine Linux",
    "nixos" => "NixOS",
    "popos" => "Pop!_OS",
    "arch" => "Arch Linux",
    "freebsd" => "FreeBSD",
    "openbsd" => "OpenBSD",
    "netbsd" => "NetBSD",
    "linux" => "Linux",
    _ => return None,
  })
}
