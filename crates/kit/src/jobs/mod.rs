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
pub use build::guess_os;
pub use file::{list, load, remove, save};
pub use run::{cancel, run, spawn};

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
  pub phase: Phase,
  pub message: String,
  pub at: i64,
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
  pub rate: Option<f64>,
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
  #[serde(default)]
  pub events: Vec<Event>,
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

  pub fn transition(&mut self, phase: Phase, message: String) {
    self.phase = phase;
    self.message = Some(message.clone());
    self.updated = crate::machine::now();
    self.events.push(Event {
      phase,
      message,
      at: self.updated,
    });
    if self.events.len() > 100 {
      self.events.remove(0);
    }
  }

  pub fn title(&self) -> String {
    if !self.spec.name.trim().is_empty() {
      return self.spec.name.clone();
    }
    self
      .spec
      .system
      .as_deref()
      .and_then(crate::catalog::find)
      .map(|entry| entry.title())
      .unwrap_or_else(|| "New Machine".into())
  }

  pub fn fraction(&self) -> Option<f64> {
    self
      .total
      .filter(|t| *t > 0)
      .map(|t| (self.received as f64 / t as f64).clamp(0.0, 1.0))
  }

  pub(super) fn recover(&mut self, cancelled: bool) -> bool {
    let grace = self.phase == Phase::Queued && crate::machine::now() - self.created < 10;
    let stalled = self.phase == Phase::Queued && crate::machine::now() - self.created > 30;
    if grace || (!self.is_orphaned() && !stalled) {
      return false;
    }
    self.transition(if cancelled { Phase::Cancelled } else { Phase::Failed },
      if cancelled { "Cancelled." } else if stalled { "The creation worker did not start within 30 seconds. Try again." } else { "Creation was interrupted because its worker exited. Try again, or choose an installer already on your Mac." }.into());
    true
  }

  /// The worker died without finishing.
  pub fn is_orphaned(&self) -> bool {
    !self.phase.is_finished()
      && self.pid != 0
      && unsafe { libc::kill(self.pid as i32, 0) } != 0
      && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn interrupted() -> Job {
    let mut job = Job::new(Spec::default());
    job.pid = i32::MAX as u32;
    job.phase = Phase::Resolving;
    job
  }

  #[test]
  fn exited_worker_becomes_actionable_failure() {
    let mut job = interrupted();
    assert!(job.recover(false));
    assert_eq!(job.phase, Phase::Failed);
    assert!(job.message.as_deref().unwrap().contains("worker exited"));
    assert_eq!(job.events.last().unwrap().phase, Phase::Failed);
    assert!(!job.recover(false));
  }

  #[test]
  fn cancellation_of_exited_worker_finishes_immediately() {
    let mut job = interrupted();
    assert!(job.recover(true));
    assert_eq!(job.phase, Phase::Cancelled);
  }

  #[test]
  fn live_worker_and_startup_grace_are_preserved() {
    let mut job = interrupted();
    job.pid = std::process::id();
    assert!(!job.recover(false));
    job.pid = i32::MAX as u32;
    job.phase = Phase::Queued;
    assert!(!job.recover(false));
    job.created -= 11;
    assert!(job.recover(false));
  }

  #[test]
  fn queued_jobs_cannot_wait_forever_even_if_the_parent_is_alive() {
    let mut job = Job::new(Spec::default());
    job.created -= 31;
    assert!(job.recover(false));
    assert_eq!(job.phase, Phase::Failed);
    assert!(job.message.unwrap().contains("did not start"));
  }

  #[test]
  fn older_job_files_still_load() {
    let job = Job::new(Spec::default());
    let mut json = serde_json::to_value(&job).unwrap();
    json.as_object_mut().unwrap().remove("events");
    let loaded: Job = serde_json::from_value(json).unwrap();
    assert!(loaded.events.is_empty());
  }
}
