//! One machine as the interface sees it: where it is, how it's configured,
//! and what its process reports.

use vista_kit::bundle::{self, Bundle, Snapshot, Snapshots};
use vista_kit::machine::{Live, State};
use vista_kit::store::{Entry, Store};

#[derive(Clone, Debug)]
pub struct Machine {
  pub entry: Entry,
  /// `None` when the bundle has gone missing (moved, deleted in Finder).
  pub bundle: Option<Bundle>,
  pub live: Option<Live>,
  /// Bytes the bundle really occupies.
  pub usage: u64,
  pub snapshots: Vec<Snapshot>,
}

impl Machine {
  pub fn id(&self) -> &str {
    &self.entry.id
  }

  pub fn name(&self) -> &str {
    self
      .bundle
      .as_ref()
      .map(|b| b.config.name.as_str())
      .unwrap_or(&self.entry.name)
  }

  pub fn family(&self) -> &str {
    self
      .bundle
      .as_ref()
      .map(|b| b.config.family.as_str())
      .unwrap_or(&self.entry.family)
  }

  pub fn state(&self) -> State {
    match &self.live {
      Some(live) => live.state,
      None
        if self
          .bundle
          .as_ref()
          .map(|b| b.is_suspended())
          .unwrap_or(false) =>
      {
        State::Suspended
      }
      None => State::Stopped,
    }
  }

  pub fn is_running(&self) -> bool {
    self.live.is_some()
  }

  pub fn title(&self) -> String {
    self
      .bundle
      .as_ref()
      .map(|b| b.config.title())
      .unwrap_or_else(|| "Missing".into())
  }

  pub fn summary(&self) -> String {
    self
      .bundle
      .as_ref()
      .map(|b| b.config.summary())
      .unwrap_or_else(|| self.entry.path.clone())
  }
}

/// Read everything the interface shows, off the main thread.
pub async fn gather(store: &Store) -> Vec<Machine> {
  let entries = store.machines().await.unwrap_or_default();
  let mut machines = Vec::with_capacity(entries.len());
  for entry in entries {
    let bundle = bundle::load(&entry.path()).ok();
    let live = vista_kit::ipc::status(&entry.id).await;
    let usage = bundle.as_ref().map(|b| bundle::usage(&b.path)).unwrap_or(0);
    let snapshots = bundle.as_ref().map(Snapshots::list).unwrap_or_default();
    machines.push(Machine {
      entry,
      bundle,
      live,
      usage,
      snapshots,
    });
  }
  machines
}
