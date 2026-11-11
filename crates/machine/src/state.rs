//! The runner's state. It lives on the main thread, where the machine, its
//! view and every framework callback live too.

use std::cell::RefCell;
use std::time::Instant;

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::NSWindow;
use objc2_virtualization::{VZVirtualMachine, VZVirtualMachineView};
use vista_kit::bundle::Bundle;
use vista_kit::machine::{Live, State};
use vista_vz::InstallProgress;

use crate::delegate::Delegate;

/// What to do once the guest has stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum After {
  Exit,
  Restart,
}

pub struct Runner {
  pub mtm: MainThreadMarker,
  pub bundle: Bundle,
  pub machine: Option<Retained<VZVirtualMachine>>,
  pub view: Option<Retained<VZVirtualMachineView>>,
  pub window: Option<Retained<NSWindow>>,
  pub install: Option<InstallProgress>,
  pub delegate: Retained<Delegate>,
  pub state: State,
  pub message: Option<String>,
  pub can_save: bool,
  pub started: Option<Instant>,
  pub after: After,
  /// Booted with installer media attached.
  pub installing: bool,
  pub visible: bool,
}

thread_local! {
  static RUNNER: RefCell<Option<Runner>> = const { RefCell::new(None) };
}

pub fn install(bundle: Bundle, mtm: MainThreadMarker) {
  let delegate = Delegate::new(mtm);
  RUNNER.with(|cell| {
    *cell.borrow_mut() = Some(Runner {
      mtm,
      bundle,
      machine: None,
      view: None,
      window: None,
      install: None,
      delegate,
      state: State::Starting,
      message: None,
      can_save: false,
      started: None,
      after: After::Exit,
      installing: false,
      visible: false,
    })
  });
}

/// Run `f` against the runner. `None` if it's already borrowed — a framework
/// callback arriving while another one is mid-flight — which callers treat as
/// "try again later" rather than a crash.
pub fn with<R>(f: impl FnOnce(&mut Runner) -> R) -> Option<R> {
  RUNNER.with(|cell| {
    let mut guard = cell.try_borrow_mut().ok()?;
    guard.as_mut().map(f)
  })
}

impl Runner {
  pub fn live(&self) -> Live {
    let progress = self.install.as_ref().map(|install| install.fraction());
    let ip = if self.state.is_live() {
      vista_kit::net::lease(&self.bundle.config.mac)
    } else {
      None
    };
    Live {
      state: self.state,
      progress,
      message: self.message.clone(),
      pid: std::process::id(),
      uptime: self.started.map(|at| at.elapsed().as_secs()).unwrap_or(0),
      ip,
      visible: self.visible,
    }
  }

  pub fn fail(&mut self, message: impl Into<String>) {
    let message = message.into();
    eprintln!("vistamachine: {message}");
    self.state = State::Error;
    self.message = Some(message);
  }
}

/// Leave: remove the socket so nobody waits on a dead machine.
pub fn exit(code: i32) -> ! {
  let id = with(|runner| runner.bundle.config.id.clone()).unwrap_or_default();
  if !id.is_empty() {
    let _ = std::fs::remove_file(vista_kit::paths::socket(&id));
  }
  eprintln!("vistamachine: exiting ({code})");
  std::process::exit(code)
}
