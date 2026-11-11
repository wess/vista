//! Linux and the BSDs. Every source here was checked against its publisher on
//! 2026-11-10; resolution happens at create time, so point releases are
//! picked up without a Vista update.

use super::{Checksum, Entry, Source, SumFormat};
use crate::machine::Os;

#[allow(clippy::too_many_arguments)]
const fn entry(
  id: &'static str,
  os: Os,
  family: &'static str,
  name: &'static str,
  version: &'static str,
  edition: &'static str,
  source: Source,
  checksum: Option<Checksum>,
  desktop: bool,
  notes: &'static str,
) -> Entry {
  Entry {
    id,
    os,
    family,
    name,
    version,
    edition,
    source,
    checksum,
    desktop,
    notes,
  }
}

const fn listing(index: &'static str, pattern: &'static str) -> Source {
  Source::Listing { index, pattern }
}

const fn sum(url: &'static str, format: SumFormat) -> Option<Checksum> {
  Some(Checksum { url, format })
}

use Os::{Linux as L, Other as O};
use SumFormat::{Bsd, Gnu, Single};

pub const LINUX: &[Entry] = &[
  entry(
    "ubuntudesktop2604", L, "ubuntu", "Ubuntu", "26.04 LTS", "Desktop",
    listing("https://cdimage.ubuntu.com/ubuntu/releases/26.04/release/", r"^ubuntu-26\.04(\.\d+)*-desktop-arm64\.iso$"),
    sum("SHA256SUMS", Gnu), true,
    "GNOME desktop with a graphical installer.",
  ),
  entry(
    "ubuntuserver2604", L, "ubuntu", "Ubuntu", "26.04 LTS", "Server",
    listing("https://cdimage.ubuntu.com/ubuntu/releases/26.04/release/", r"^ubuntu-26\.04(\.\d+)*-live-server-arm64\.iso$"),
    sum("SHA256SUMS", Gnu), false,
    "Text installer, no desktop. Offers to install OpenSSH.",
  ),
  entry(
    "ubuntudesktop2404", L, "ubuntu", "Ubuntu", "24.04 LTS", "Desktop",
    listing("https://cdimage.ubuntu.com/ubuntu/releases/24.04/release/", r"^ubuntu-24\.04(\.\d+)*-desktop-arm64\.iso$"),
    sum("SHA256SUMS", Gnu), true,
    "GNOME desktop with a graphical installer.",
  ),
  entry(
    "ubuntuserver2404", L, "ubuntu", "Ubuntu", "24.04 LTS", "Server",
    listing("https://cdimage.ubuntu.com/ubuntu/releases/24.04/release/", r"^ubuntu-24\.04(\.\d+)*-live-server-arm64\.iso$"),
    sum("SHA256SUMS", Gnu), false,
    "Text installer, no desktop.",
  ),
  entry(
    "fedoraworkstation", L, "fedora", "Fedora", "", "Workstation",
    listing("https://dl.fedoraproject.org/pub/fedora/linux/releases/{n}/Workstation/aarch64/iso/", r"^Fedora-Workstation-Live-\d+-[\d.]+\.aarch64\.iso$"),
    sum(r"^Fedora-Workstation-\d+-[\d.]+-aarch64-CHECKSUM$", Bsd), true,
    "Live GNOME desktop; install from the desktop. Choose \"Start Fedora\" at the boot menu to skip the media check.",
  ),
  entry(
    "fedoraeverything", L, "fedora", "Fedora", "", "Everything (network)",
    listing("https://dl.fedoraproject.org/pub/fedora/linux/releases/{n}/Everything/aarch64/iso/", r"^Fedora-Everything-netinst-aarch64-\d+-[\d.]+\.iso$"),
    sum(r"^Fedora-Everything-\d+-[\d.]+-aarch64-CHECKSUM$", Bsd), true,
    "Graphical network installer; pick any edition or desktop.",
  ),
  entry(
    "debian", L, "debian", "Debian", "13", "Netinst",
    Source::Sums { sums: "https://cdimage.debian.org/debian-cd/current/arm64/iso-cd/SHA256SUMS", pattern: r"^debian-\d+\.\d+\.\d+-arm64-netinst\.iso$" },
    sum("SHA256SUMS", Gnu), false,
    "Choose \"Graphical install\" at the boot menu for the mouse-driven installer.",
  ),
  entry(
    "debiandvd", L, "debian", "Debian", "13", "DVD (offline)",
    Source::Sums { sums: "https://cdimage.debian.org/debian-cd/current/arm64/iso-dvd/SHA256SUMS", pattern: r"^debian-\d+\.\d+\.\d+-arm64-DVD-1\.iso$" },
    sum("SHA256SUMS", Gnu), false,
    "Installs without a network connection.",
  ),
  entry(
    "kali", L, "kali", "Kali Linux", "", "Installer",
    listing("https://kali.download/base-images/current/", r"^kali-linux-\d{4}\.\d+[a-z]?-installer-arm64\.iso$"),
    sum("SHA256SUMS", Gnu), false,
    "Installs the Xfce desktop. Choose \"Graphical install\" at the boot menu.",
  ),
  entry(
    "centos", L, "centos", "CentOS Stream", "10", "Network install",
    listing("https://mirror.stream.centos.org/10-stream/BaseOS/aarch64/iso/", r"^CentOS-Stream-10-\d{8}\.\d+-aarch64-boot\.iso$"),
    sum("SHA256SUM", Bsd), true,
    "Graphical Anaconda installer over the network.",
  ),
  entry(
    "rockyworkstation", L, "rocky", "Rocky Linux", "10", "Workstation",
    listing("https://download.rockylinux.org/pub/rocky/10/live/aarch64/", r"^Rocky-10\.\d+-Workstation-aarch64-\d{8}\.\d+\.iso$"),
    sum("CHECKSUM", Bsd), true,
    "Live GNOME desktop with an installer.",
  ),
  entry(
    "rocky", L, "rocky", "Rocky Linux", "10", "Network install",
    listing("https://download.rockylinux.org/pub/rocky/10/isos/aarch64/", r"^Rocky-10\.\d+-aarch64-boot\.iso$"),
    sum("CHECKSUM", Bsd), true,
    "Graphical Anaconda installer over the network.",
  ),
  entry(
    "almagnome", L, "alma", "AlmaLinux", "10", "Live GNOME",
    listing("https://repo.almalinux.org/almalinux/10/live/aarch64/", r"^AlmaLinux-10\.\d+-aarch64-Live-GNOME\.iso$"),
    sum("CHECKSUM", Gnu), true,
    "Live GNOME desktop with an installer.",
  ),
  entry(
    "alma", L, "alma", "AlmaLinux", "10", "Network install",
    listing("https://repo.almalinux.org/almalinux/10/isos/aarch64/", r"^AlmaLinux-10\.\d+-aarch64-boot\.iso$"),
    sum("CHECKSUM", Bsd), true,
    "Graphical Anaconda installer over the network.",
  ),
  entry(
    "tumbleweed", L, "opensuse", "openSUSE Tumbleweed", "", "Live GNOME",
    listing("https://download.opensuse.org/ports/aarch64/tumbleweed/iso/", r"^openSUSE-Tumbleweed-GNOME-Live-aarch64-Snapshot\d{8}-Media\.iso$"),
    sum("{file}.sha256", Single), true,
    "Rolling release. Live GNOME desktop with an installer.",
  ),
  entry(
    "tumbleweednet", L, "opensuse", "openSUSE Tumbleweed", "", "Network install",
    listing("https://download.opensuse.org/ports/aarch64/tumbleweed/iso/", r"^openSUSE-Tumbleweed-NET-aarch64-Snapshot\d{8}-Media\.iso$"),
    sum("{file}.sha256", Single), true,
    "Rolling release. YaST installer over the network.",
  ),
  entry(
    "popos", L, "popos", "Pop!_OS", "24.04", "COSMIC",
    Source::Pop { api: "https://api.pop-os.org/builds/24.04/generic?arch=arm64" },
    None, true,
    "COSMIC desktop. Renders in software: there's no 3D acceleration for Linux guests.",
  ),
  entry(
    "nixos", L, "nixos", "NixOS", "26.05", "Graphical",
    Source::Direct { url: "https://channels.nixos.org/nixos-26.05/latest-nixos-graphical-aarch64-linux.iso" },
    sum("{file}.sha256", Single), true,
    "GNOME with the Calamares installer.",
  ),
  entry(
    "nixosminimal", L, "nixos", "NixOS", "26.05", "Minimal",
    Source::Direct { url: "https://channels.nixos.org/nixos-26.05/latest-nixos-minimal-aarch64-linux.iso" },
    sum("{file}.sha256", Single), false,
    "Console installer.",
  ),
  entry(
    "alpine", L, "alpine", "Alpine Linux", "", "Virtual",
    listing("https://dl-cdn.alpinelinux.org/alpine/latest-stable/releases/aarch64/", r"^alpine-virt-\d+\.\d+\.\d+-aarch64\.iso$"),
    sum("{file}.sha256", Single), false,
    "Tiny. Log in as root and run setup-alpine.",
  ),
  entry(
    "alpinestandard", L, "alpine", "Alpine Linux", "", "Standard",
    listing("https://dl-cdn.alpinelinux.org/alpine/latest-stable/releases/aarch64/", r"^alpine-standard-\d+\.\d+\.\d+-aarch64\.iso$"),
    sum("{file}.sha256", Single), false,
    "Log in as root and run setup-alpine.",
  ),
  entry(
    "freebsd", O, "freebsd", "FreeBSD", "", "Installer",
    listing("https://download.freebsd.org/releases/arm64/aarch64/ISO-IMAGES/{n}/", r"^FreeBSD-\d+\.\d+-RELEASE-arm64-aarch64-disc1\.iso$"),
    sum(r"^CHECKSUM\.SHA256-FreeBSD-[\d.]+-RELEASE-arm64-aarch64$", Bsd), false,
    "Text installer. Experimental on this framework.",
  ),
  entry(
    "openbsd", O, "openbsd", "OpenBSD", "", "Installer",
    listing("https://cdn.openbsd.org/pub/OpenBSD/{n}/arm64/", r"^install\d{2}\.iso$"),
    sum("SHA256", Bsd), false,
    "Text installer. Experimental on this framework.",
  ),
  entry(
    "netbsd", O, "netbsd", "NetBSD", "", "Installer",
    listing("https://cdn.netbsd.org/pub/NetBSD/images/{n}/", r"^NetBSD-\d+\.\d+-evbarm-aarch64\.iso$"),
    None, false,
    "Text installer. Experimental on this framework.",
  ),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ids_are_unique_and_patterns_compile() {
    let mut seen = std::collections::HashSet::new();
    for entry in LINUX {
      assert!(seen.insert(entry.id), "duplicate id {}", entry.id);
      assert!(
        super::super::family_name(entry.family).is_some(),
        "{}",
        entry.family
      );
      if let Source::Listing { pattern, .. } | Source::Sums { pattern, .. } = entry.source {
        regex::Regex::new(pattern).unwrap();
      }
    }
  }
}
