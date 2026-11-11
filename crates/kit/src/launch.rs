//! Starting a machine: one `vistamachine` process per machine.
//!
//! The process is detached into its own session so quitting Vista (or the
//! agent that started it) leaves the machine running, and a crash in one
//! machine never takes another down.

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::bundle::{self, LOG};
use crate::machine::Live;
use crate::{Error, Result};

const HELPER: &str = "VistaMachine.app/Contents/MacOS/vistamachine";

/// Where the runner binary is: `$VISTA_MACHINE`, then next to this
/// executable (a cargo build), then the helper app inside Vista.app.
pub fn runner() -> Result<PathBuf> {
  if let Some(path) = std::env::var_os("VISTA_MACHINE") {
    return Ok(PathBuf::from(path));
  }
  let exe = std::env::current_exe()?;
  let dir = exe.parent().ok_or_else(|| Error::msg("no executable directory"))?;
  let candidates = [
    dir.join("vistamachine"),
    // Vista.app/Contents/MacOS/<exe> -> Vista.app/Contents/Helpers/…
    dir.join("..").join("Helpers").join(HELPER),
  ];
  candidates
    .into_iter()
    .find(|path| path.exists())
    .ok_or_else(|| Error::msg("Can't find vistamachine next to Vista."))
}

/// The Vista Tools folder shared into every guest: `$VISTA_TOOLS`, then a
/// `tools` folder beside the runner (cargo's `target/tools`), then the app
/// bundle's resources.
pub fn tools() -> Option<PathBuf> {
  if let Some(path) = std::env::var_os("VISTA_TOOLS") {
    return Some(PathBuf::from(path));
  }
  let exe = std::env::current_exe().ok()?;
  let dir = exe.parent()?;
  [
    dir.join("tools"),
    dir.join("..").join("tools"),
    dir.join("..").join("Resources").join("tools"),
  ]
  .into_iter()
  .find(|path| path.join("install.sh").exists())
}

/// Start the machine in `path`. `visible` opens its window; otherwise it runs
/// in the background until something asks to show it.
pub fn start(path: &Path, visible: bool) -> Result<u32> {
  let bundle = bundle::load(path)?;
  if crate::paths::socket(&bundle.config.id).exists()
    && std::os::unix::net::UnixStream::connect(crate::paths::socket(&bundle.config.id)).is_ok()
  {
    return Err(Error::msg(format!("{} is already running.", bundle.config.name)));
  }
  let log = std::fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(bundle.file(LOG))?;
  let mut command = Command::new(runner()?);
  command.arg(path);
  if !visible {
    command.arg("--hidden");
  }
  command
    .stdin(Stdio::null())
    .stdout(log.try_clone()?)
    .stderr(log);
  unsafe {
    command.pre_exec(|| {
      libc::setsid();
      Ok(())
    });
  }
  let child = command.spawn()?;
  Ok(child.id())
}

/// Wait until a just-started machine answers on its socket.
pub async fn wait_ready(id: &str, limit: Duration) -> Option<Live> {
  let deadline = tokio::time::Instant::now() + limit;
  while tokio::time::Instant::now() < deadline {
    if let Some(live) = crate::ipc::status(id).await {
      return Some(live);
    }
    tokio::time::sleep(Duration::from_millis(150)).await;
  }
  None
}
