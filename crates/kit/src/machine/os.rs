use serde::{Deserialize, Serialize};

/// The guest's operating system. It decides the platform (Mac or generic), the
/// boot loader, and which devices the machine gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
  Macos,
  Linux,
  Windows,
  /// Anything else that boots from an ISO on a generic EFI machine: the BSDs,
  /// a distro Vista doesn't know.
  Other,
}

impl Os {
  pub fn label(self) -> &'static str {
    match self {
      Os::Macos => "macOS",
      Os::Linux => "Linux",
      Os::Windows => "Windows",
      Os::Other => "Other",
    }
  }

  pub fn is_mac(self) -> bool {
    self == Os::Macos
  }

  /// Sensible starting hardware for a new machine of this kind.
  pub fn defaults(self) -> (u32, u64, u64) {
    // (cpus, memory MiB, disk GiB)
    match self {
      Os::Macos => (4, 8192, 96),
      Os::Windows => (4, 8192, 80),
      Os::Linux => (4, 4096, 64),
      Os::Other => (2, 2048, 32),
    }
  }

  /// The smallest disk the installer will accept, in GiB.
  pub fn minimum_disk(self) -> u64 {
    match self {
      Os::Macos => 64,
      Os::Windows => 64,
      Os::Linux => 16,
      Os::Other => 4,
    }
  }
}
