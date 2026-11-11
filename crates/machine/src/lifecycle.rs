//! Starting, stopping, suspending, snapshotting. All on the main thread; the
//! framework calls back on the main queue, which is where the machine was
//! created.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use block2::RcBlock;
use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::AnyThread;
use objc2_foundation::NSError;
use objc2_virtualization::VZVirtualMachine;
use vista_kit::bundle::{self, Snapshot, Snapshots, AUXILIARY, HARDWARE, SAVED};
use vista_kit::machine::{Account, Installer, State};
use vista_vz::util::{describe, url};

use crate::state::{exit, with, After};
use crate::window;

pub type Reply = Box<dyn FnOnce(Result<(), String>)>;
pub type SnapshotReply = Box<dyn FnOnce(Result<Snapshot, String>)>;

/// A guest that wrote this much to its system disk while booted from an
/// installer has been installed: when it next powers off, the installer comes
/// out and the machine boots from disk.
const INSTALLED: u64 = 3 * 1024 * 1024 * 1024;

/// Run `f` on the main queue after `delay`.
pub fn later(delay: Duration, f: impl FnOnce() + Send + 'static) {
  std::thread::spawn(move || {
    std::thread::sleep(delay);
    DispatchQueue::main().exec_async(f);
  });
}

/// Exit once a reply has had time to reach the socket.
fn exit_soon(code: i32) {
  later(Duration::from_millis(400), move || exit(code));
}

/// Wrap a one-shot callback as the `Fn` block the framework wants.
fn once(reply: impl FnOnce(Option<String>) + 'static) -> RcBlock<dyn Fn(*mut NSError)> {
  let cell = std::cell::RefCell::new(Some(reply));
  RcBlock::new(move |err: *mut NSError| {
    let err = unsafe { Retained::retain(err) }.map(|e| describe(&e));
    if let Some(reply) = cell.borrow_mut().take() {
      reply(err);
    }
  })
}

fn machine() -> Option<Retained<VZVirtualMachine>> {
  with(|runner| runner.machine.clone()).flatten()
}

fn set_state(state: State) {
  with(|runner| runner.state = state);
}

pub fn boot(visible: bool) {
  let ipsw = with(|runner| {
    let installed = runner.bundle.file(AUXILIARY).exists() && runner.bundle.file(HARDWARE).exists();
    match &runner.bundle.config.installer {
      Some(Installer::Ipsw { path, .. }) if !installed => Some(PathBuf::from(path)),
      _ => None,
    }
  })
  .flatten();
  match ipsw {
    Some(ipsw) => install(ipsw, visible),
    None => launch(visible, None),
  }
}

fn install(ipsw: PathBuf, visible: bool) {
  let Some(bundle) = with(|runner| {
    runner.state = State::Installing;
    runner.message = Some("Installing macOS".into());
    if visible {
      window::installing(runner);
    }
    runner.bundle.clone()
  }) else {
    return;
  };
  vista_vz::install_macos(
    bundle,
    ipsw,
    |progress| {
      with(|runner| runner.install = Some(progress));
      watch_install();
    },
    move |result| match result {
      Ok(()) => {
        let account = with(|runner| {
          runner.install = None;
          window::installed(runner);
          // `prepare` may have raised CPU and memory; read them back.
          if let Ok(fresh) = bundle::load(&runner.bundle.path) {
            runner.bundle = fresh;
          }
          let account = match runner.bundle.config.installer.take() {
            Some(Installer::Ipsw { account, .. }) => account,
            _ => None,
          };
          let _ = bundle::save(&runner.bundle.path, &runner.bundle.config);
          runner.message = None;
          account
        })
        .flatten();
        launch(visible, account);
      }
      Err(err) => {
        with(|runner| {
          runner.install = None;
          runner.fail(err);
        });
        later(Duration::from_secs(120), || exit(1));
      }
    },
  );
}

/// Keep the install window's progress current.
fn watch_install() {
  let going = with(|runner| {
    window::progress(runner);
    runner.install.is_some()
  })
  .unwrap_or(false);
  if going {
    later(Duration::from_millis(500), watch_install);
  }
}

/// Build the machine and power it on, resuming saved memory when there is
/// some.
fn launch(visible: bool, account: Option<Account>) {
  let prepared = with(|runner| -> Result<(Retained<VZVirtualMachine>, PathBuf, bool), String> {
    let installing = matches!(runner.bundle.config.installer, Some(Installer::Iso { .. }));
    let built = vista_vz::build(&runner.bundle, installing)?;
    runner.can_save = built.can_save;
    runner.installing = installing;
    let machine = unsafe {
      VZVirtualMachine::initWithConfiguration(VZVirtualMachine::alloc(), &built.configuration)
    };
    unsafe { machine.setDelegate(Some(ProtocolObject::from_ref(&*runner.delegate))) };
    window::attach(runner, &machine);
    if visible {
      window::show(runner);
    }
    runner.machine = Some(machine.clone());
    runner.state = State::Starting;
    runner.message = None;
    Ok((machine, runner.bundle.file(SAVED), runner.can_save))
  });
  match prepared {
    Some(Ok((machine, saved, can_save))) => {
      if saved.exists() && can_save {
        restore(machine, saved);
      } else {
        let _ = std::fs::remove_file(&saved);
        cold_start(machine, account);
      }
    }
    Some(Err(err)) => {
      with(|runner| runner.fail(err));
      later(Duration::from_secs(120), || exit(1));
    }
    None => {}
  }
}

fn running() {
  with(|runner| {
    runner.state = State::Running;
    runner.started = Some(Instant::now());
    runner.message = None;
  });
}

fn cold_start(machine: Retained<VZVirtualMachine>, account: Option<Account>) {
  let done = once(|err| match err {
    None => running(),
    Some(err) => {
      with(|runner| runner.fail(format!("Couldn't start: {err}")));
      later(Duration::from_secs(120), || exit(1));
    }
  });
  let options = account.and_then(|account| unsafe { vista_vz::modern::provisioning(&account) });
  unsafe {
    match options {
      Some(options) => machine.startWithOptions_completionHandler(&Retained::into_super(options), &done),
      None => machine.startWithCompletionHandler(&done),
    }
  }
}

fn restore(machine: Retained<VZVirtualMachine>, saved: PathBuf) {
  set_state(State::Restoring);
  let again = machine.clone();
  let path = saved.clone();
  let done = once(move |err| {
    // The saved memory is only good once; the guest moves on from here.
    let _ = std::fs::remove_file(&path);
    match err {
      None => {
        let resumed = once(|err| match err {
          None => running(),
          Some(err) => with(|runner| runner.fail(format!("Couldn't resume: {err}"))).unwrap_or(()),
        });
        unsafe { again.resumeWithCompletionHandler(&resumed) };
      }
      Some(err) => {
        eprintln!("vistamachine: saved state unusable ({err}); starting fresh");
        cold_start(again, None);
      }
    }
  });
  unsafe { machine.restoreMachineStateFromURL_completionHandler(&url(&saved), &done) };
}

/// The guest powered off, or the machine failed.
pub fn stopped(error: Option<String>) {
  enum Next {
    Exit(i32),
    Relaunch(bool),
  }
  let next = with(|runner| {
    runner.started = None;
    if let Some(err) = &error {
      runner.fail(err.clone());
      return Next::Exit(1);
    }
    let disk = runner
      .bundle
      .config
      .system()
      .map(|disk| bundle::usage(&runner.bundle.file(&disk.file)))
      .unwrap_or(0);
    if runner.installing && disk > INSTALLED {
      runner.bundle.config.installer = None;
      let _ = bundle::save(&runner.bundle.path, &runner.bundle.config);
      return Next::Relaunch(runner.visible);
    }
    match runner.after {
      After::Restart => {
        runner.after = After::Exit;
        Next::Relaunch(runner.visible)
      }
      After::Exit => {
        runner.state = State::Stopped;
        Next::Exit(0)
      }
    }
  });
  match next {
    Some(Next::Relaunch(visible)) => {
      // Re-read the bundle so settings changed while running apply.
      with(|runner| {
        if let Ok(fresh) = bundle::load(&runner.bundle.path) {
          runner.bundle = fresh;
        }
      });
      launch(visible, None);
    }
    Some(Next::Exit(0)) => exit_soon(0),
    Some(Next::Exit(code)) => later(Duration::from_secs(120), move || exit(code)),
    None => {}
  }
}

pub fn pause(reply: Reply) {
  let Some(machine) = machine() else {
    return reply(Err("The machine isn't running.".into()));
  };
  if !unsafe { machine.canPause() } {
    return reply(Err("The machine can't be paused right now.".into()));
  }
  set_state(State::Pausing);
  let done = once(move |err| {
    set_state(if err.is_none() { State::Paused } else { State::Running });
    reply(err.map_or(Ok(()), Err));
  });
  unsafe { machine.pauseWithCompletionHandler(&done) };
}

pub fn resume(reply: Reply) {
  let Some(machine) = machine() else {
    return reply(Err("The machine isn't running.".into()));
  };
  if !unsafe { machine.canResume() } {
    return reply(Err("The machine isn't paused.".into()));
  }
  set_state(State::Resuming);
  let done = once(move |err| {
    set_state(if err.is_none() { State::Running } else { State::Paused });
    reply(err.map_or(Ok(()), Err));
  });
  unsafe { machine.resumeWithCompletionHandler(&done) };
}

/// Press the guest's power button.
pub fn request_stop() -> Result<(), String> {
  let machine = machine().ok_or("The machine isn't running.")?;
  unsafe {
    if !machine.canRequestStop() {
      return Err("The guest can't be asked to shut down right now.".into());
    }
    machine
      .requestStopWithError()
      .map_err(|err| describe(&err))?;
  }
  set_state(State::Stopping);
  Ok(())
}

/// Pull the plug.
pub fn kill(reply: Reply) {
  let Some(machine) = machine() else {
    reply(Ok(()));
    exit_soon(0);
    return;
  };
  if !unsafe { machine.canStop() } {
    reply(Ok(()));
    exit_soon(0);
    return;
  }
  set_state(State::Stopping);
  let done = once(move |err| {
    reply(err.map_or(Ok(()), Err));
    exit_soon(0);
  });
  unsafe { machine.stopWithCompletionHandler(&done) };
}

pub fn restart() -> Result<(), String> {
  with(|runner| runner.after = After::Restart);
  if request_stop().is_ok() {
    return Ok(());
  }
  let Some(machine) = machine() else {
    return Err("The machine isn't running.".into());
  };
  let done = once(|_| stopped(None));
  unsafe { machine.stopWithCompletionHandler(&done) };
  Ok(())
}

/// Pause, write memory to `path`, and hand back whether it worked. The
/// machine is left paused either way.
fn save_to(path: PathBuf, then: impl FnOnce(Result<(), String>) + 'static) {
  let Some(machine) = machine() else {
    return then(Err("The machine isn't running.".into()));
  };
  let can_save = with(|runner| runner.can_save).unwrap_or(false);
  if !can_save {
    return then(Err(
      "This machine's hardware can't have its memory saved (shut it down instead).".into(),
    ));
  }
  let saving = machine.clone();
  let write = move || {
    set_state(State::Saving);
    let done = once(move |err| then(err.map_or(Ok(()), Err)));
    unsafe { saving.saveMachineStateToURL_completionHandler(&url(&path), &done) };
  };
  if unsafe { machine.canPause() } {
    set_state(State::Pausing);
    let paused = once(move |err| match err {
      None => write(),
      Some(_) => write(),
    });
    unsafe { machine.pauseWithCompletionHandler(&paused) };
  } else {
    write();
  }
}

/// Save memory to the bundle and quit; the next start resumes.
pub fn suspend(reply: Reply) {
  let Some(saved) = with(|runner| runner.bundle.file(SAVED)) else {
    return;
  };
  save_to(saved.clone(), move |result| match result {
    Ok(()) => {
      let machine = machine();
      let done = once(move |_| {
        reply(Ok(()));
        exit_soon(0);
      });
      match machine {
        Some(machine) => unsafe { machine.stopWithCompletionHandler(&done) },
        None => exit_soon(0),
      }
    }
    Err(err) => {
      let _ = std::fs::remove_file(&saved);
      resume(Box::new(|_| {}));
      reply(Err(err));
    }
  });
}

/// Snapshot the machine. While it runs, its memory goes into the snapshot
/// too, so reverting resumes exactly here.
pub fn snapshot(name: String, reply: SnapshotReply) {
  let Some((bundle, was_running)) = with(|runner| (runner.bundle.clone(), runner.state == State::Running)) else {
    return;
  };
  if machine().is_none() {
    return reply(Snapshots::take(&bundle, &name).map_err(|e| e.to_string()));
  }
  let (id, dir) = match Snapshots::reserve(&bundle) {
    Ok(reserved) => reserved,
    Err(err) => return reply(Err(err.to_string())),
  };
  let saved = dir.join(SAVED);
  save_to(saved, move |result| {
    let outcome = result.and_then(|_| {
      Snapshots::capture(&bundle, &id, &dir, &name).map_err(|e| e.to_string())
    });
    if outcome.is_err() {
      let _ = std::fs::remove_dir_all(&dir);
    }
    if was_running {
      resume(Box::new(|_| {}));
    } else {
      set_state(State::Paused);
    }
    reply(outcome);
  });
}

/// Take the installer media out at the next start.
pub fn eject() {
  with(|runner| {
    if matches!(runner.bundle.config.installer, Some(Installer::Iso { .. })) {
      runner.bundle.config.installer = None;
      let _ = bundle::save(&runner.bundle.path, &runner.bundle.config);
      runner.message = Some("The installer will be removed when the machine restarts.".into());
    }
  });
}

/// The window's close button: suspend when memory can be saved, otherwise
/// keep running in the background.
pub fn close_window() {
  let (can_save, live) = with(|runner| (runner.can_save, runner.state.is_live())).unwrap_or((false, false));
  if !live {
    exit(0);
  }
  if can_save {
    suspend(Box::new(|result| {
      if let Err(err) = result {
        eprintln!("vistamachine: suspend failed: {err}");
      }
    }));
  } else {
    with(window::hide);
  }
}
