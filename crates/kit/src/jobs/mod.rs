//! Making a machine: resolve the system, download its installer, build the
//! bundle, and optionally start it.
//!
//! A job is a JSON file in `paths::jobs()`, rewritten as it moves along, so
//! the app, the menu bar and an agent can all watch the same creation — and
//! one started by an agent shows up in Vista's sidebar with its progress.

mod build;
mod file;
mod run;

pub use build::configure;
pub use file::{list, load, remove, save};
pub use build::guess_os;
pub use run::{cancel, run};

use serde::{Deserialize, Serialize};

use crate::machine::{Account, Os};

/// What to make.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Spec {
  pub name: String,
  /// A catalog id (`ubuntudesktop2604`, `macoslatest`, `windows11`…).
  #[serde(default)]
  pub system: Option<String>,
  /// Or a local installer: an `.iso` or an `.ipsw`.
  #[serde(default)]
  pub file: Option<String>,
  /// With `file`: what it installs. Guessed from the extension otherwise.
  #[serde(default)]
  pub os: Option<Os>,
  #[serde(default)]
  pub cpus: Option<u32>,
  /// MiB.
  #[serde(default)]
  pub memory: Option<u64>,
  /// GiB.
  #[serde(default)]
  pub disk: Option<u64>,
  /// The first account (macOS 27 hosts, Windows).
  #[serde(default)]
  pub account: Option<Account>,
  #[serde(default)]
  pub sharedownloads: bool,
  #[serde(default)]
  pub rosetta: bool,
  /// Start once built; `visible` opens its window.
  #[serde(default)]
  pub start: bool,
  #[serde(default = "yes")]
  pub visible: bool,
  /// Directory to put the bundle in; the configured location otherwise.
  #[serde(default)]
  pub location: Option<String>,
}

fn yes() -> bool {
  true
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
  #[default]
  Queued,
  Resolving,
  Downloading,
  Verifying,
  Preparing,
  Creating,
  Starting,
  Done,
  Failed,
  Cancelled,
}

impl Phase {
  pub fn is_finished(self) -> bool {
    matches!(self, Phase::Done | Phase::Failed | Phase::Cancelled)
  }

  pub fn label(self) -> &'static str {
    match self {
      Phase::Queued => "Waiting",
      Phase::Resolving => "Finding the installer",
      Phase::Downloading => "Downloading",
      Phase::Verifying => "Verifying",
      Phase::Preparing => "Preparing the installer",
      Phase::Creating => "Creating the machine",
      Phase::Starting => "Starting",
      Phase::Done => "Ready",
      Phase::Failed => "Failed",
      Phase::Cancelled => "Cancelled",
    }
  }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Job {
  pub id: String,
  pub spec: Spec,
  pub phase: Phase,
  /// Bytes downloaded and expected.
  #[serde(default)]
  pub received: u64,
  #[serde(default)]
  pub total: Option<u64>,
  #[serde(default)]
  pub message: Option<String>,
  /// The machine's id once the bundle exists.
  #[serde(default)]
  pub machine: Option<String>,
  /// The pid working on the job, so a dead one can be noticed.
  #[serde(default)]
  pub pid: u32,
  #[serde(default)]
  pub created: i64,
  #[serde(default)]
  pub updated: i64,
}

impl Job {
  pub fn new(spec: Spec) -> Job {
    let now = crate::machine::now();
    Job {
      id: uuid::Uuid::new_v4().to_string(),
      spec,
      phase: Phase::Queued,
      pid: std::process::id(),
      created: now,
      updated: now,
      ..Job::default()
    }
  }

  pub fn fraction(&self) -> Option<f64> {
    self
      .total
      .filter(|t| *t > 0)
      .map(|t| (self.received as f64 / t as f64).clamp(0.0, 1.0))
  }

  /// The worker died without finishing.
  pub fn is_orphaned(&self) -> bool {
    !self.phase.is_finished() && self.pid != 0 && unsafe { libc::kill(self.pid as i32, 0) } != 0
  }
}
