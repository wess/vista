//! What a running machine reports about itself over its control socket.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
  #[default]
  Stopped,
  /// Stopped, with memory saved to disk: the next start resumes.
  Suspended,
  Starting,
  Running,
  Pausing,
  Paused,
  Resuming,
  Stopping,
  Saving,
  Restoring,
  /// The macOS installer is writing the system.
  Installing,
  Error,
}

impl State {
  pub fn label(self) -> &'static str {
    match self {
      State::Stopped => "Stopped",
      State::Suspended => "Suspended",
      State::Starting => "Starting",
      State::Running => "Running",
      State::Pausing => "Pausing",
      State::Paused => "Paused",
      State::Resuming => "Resuming",
      State::Stopping => "Shutting down",
      State::Saving => "Saving",
      State::Restoring => "Restoring",
      State::Installing => "Installing",
      State::Error => "Error",
    }
  }

  /// A process exists for the machine.
  pub fn is_live(self) -> bool {
    !matches!(self, State::Stopped | State::Suspended | State::Error)
  }

  pub fn is_busy(self) -> bool {
    matches!(
      self,
      State::Starting
        | State::Pausing
        | State::Resuming
        | State::Stopping
        | State::Saving
        | State::Restoring
        | State::Installing
    )
  }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Live {
  pub state: State,
  /// 0.0–1.0 while installing.
  #[serde(default)]
  pub progress: Option<f64>,
  /// The last thing worth telling a person: an install step, an error.
  #[serde(default)]
  pub message: Option<String>,
  /// The `vistamachine` process.
  #[serde(default)]
  pub pid: u32,
  /// Seconds since the guest started.
  #[serde(default)]
  pub uptime: u64,
  /// The guest's address on the shared network, once it has leased one.
  #[serde(default)]
  pub ip: Option<String>,
  #[serde(default)]
  pub visible: bool,
}
