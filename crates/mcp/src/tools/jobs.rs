//! Creating machines. Downloads can take a long time (a macOS restore image is
//! ~25 GB), so creation runs as a background job in its own process and the
//! agent polls it.

use std::time::Duration;

use serde_json::{json, Value};
use vista_kit::jobs::{self, Job, Spec};
use vista_kit::machine::{Account, Os};

use super::{flag, number, required, text, tool, Context};
use crate::output::Output;

pub fn defs() -> Vec<Value> {
  vec![
    tool(
      "create_machine",
      "Create a machine",
      "Create a new virtual machine. Give either `system` (an id from list_systems: Vista downloads the installer from its publisher, verifies its checksum and keeps it for next time) or `installer_path` (an .iso or .ipsw already on this Mac). Runs as a background job and returns its id at once; poll get_job (wait_seconds lets you block). With start=true (default) the machine boots into its installer when ready: macOS installs by itself (and with username/password on a macOS 27 host, creates the account and skips Setup Assistant); Linux/BSD installers run in the window — drive them with screenshot, click, type_text and press_keys; Windows 11 (experimental) installs unattended with the given account. Sensible hardware defaults are chosen per OS.",
      json!({
        "name": {"type": "string", "description": "Display name. Defaults to the system's name."},
        "system": {"type": "string", "description": "Catalog id from list_systems, e.g. \"ubuntudesktop2604\", \"macoslatest\", \"fedoraworkstation\", \"windows11\"."},
        "installer_path": {"type": "string", "description": "Absolute path to an .iso or .ipsw on this Mac, instead of `system`."},
        "os": {"type": "string", "enum": ["macos", "linux", "windows", "other"], "description": "With installer_path: what it installs (guessed from the file name otherwise)."},
        "cpus": {"type": "integer", "minimum": 1, "description": "Virtual CPU cores (default 4; 2 for BSDs)."},
        "memory_gb": {"type": "number", "minimum": 1, "description": "Memory in GB (default 8 for macOS/Windows, 4 for Linux)."},
        "disk_gb": {"type": "integer", "minimum": 4, "description": "System disk in GB (default 96 macOS, 80 Windows, 64 Linux). Sparse: only what the guest writes uses space."},
        "username": {"type": "string", "description": "First account's short name (macOS on a macOS 27 host; Windows)."},
        "password": {"type": "string", "description": "First account's password."},
        "fullname": {"type": "string", "description": "First account's full name."},
        "enable_ssh": {"type": "boolean", "description": "macOS: turn on Remote Login for the first account."},
        "share_downloads": {"type": "boolean", "description": "Share the Mac's Downloads folder into the guest (default true)."},
        "rosetta": {"type": "boolean", "description": "Linux: share Rosetta for x86-64 binaries."},
        "start": {"type": "boolean", "description": "Start the machine once it's built (default true)."},
        "visible": {"type": "boolean", "description": "Open its window when started (default true; needed for screenshots)."},
        "wait_seconds": {"type": "integer", "minimum": 0, "maximum": 600, "description": "Wait up to this long for the job to finish before answering (default 0)."}
      }),
      &[],
      false,
      false,
    ),
    tool(
      "get_job",
      "Get creation job",
      "Progress of a create_machine job: its phase (resolving, downloading, verifying, preparing, creating, starting, done, failed, cancelled), bytes downloaded and expected, percentage, a message, and the new machine's id once it exists. Pass wait_seconds to block until the job finishes or the time runs out.",
      json!({
        "job": {"type": "string", "description": "The job id create_machine returned (or its first 8 characters)."},
        "wait_seconds": {"type": "integer", "minimum": 0, "maximum": 600, "description": "Block up to this long for the job to finish."}
      }),
      &["job"],
      true,
      false,
    ),
    tool(
      "list_jobs",
      "List creation jobs",
      "Recent machine creation jobs (from agents and from Vista's own window), newest first, with their phase and progress.",
      json!({}),
      &[],
      true,
      false,
    ),
    tool(
      "cancel_job",
      "Cancel creation job",
      "Stop a running create_machine job. A partly downloaded installer is kept, so creating the same system again resumes the download.",
      json!({"job": {"type": "string", "description": "The job id."}}),
      &["job"],
      false,
      false,
    ),
  ]
}

pub fn describe(job: &Job) -> Value {
  json!({
    "job": job.id,
    "name": job.spec.name,
    "system": job.spec.system,
    "phase": job.phase,
    "status": job.phase.label(),
    "percent": job.fraction().map(|f| (f * 1000.0).round() / 10.0),
    "downloaded_mb": job.received / 1_000_000,
    "total_mb": job.total.map(|t| t / 1_000_000),
    "message": job.message,
    "machine": job.machine,
    "finished": job.phase.is_finished(),
    "worker_alive": !job.phase.is_finished() && !job.is_orphaned(),
    "bytes_per_second": job.rate,
    "elapsed_seconds": ((if job.phase.is_finished() { job.updated } else { vista_kit::machine::now() }) - job.created).max(0),
    "activity": job.events,
  })
}

pub async fn call(context: &Context, name: &str, args: &Value) -> Option<Result<Output, String>> {
  Some(match name {
    "create_machine" => create(context, args).await,
    "get_job" => get(context, args).await,
    "list_jobs" => {
      let jobs: Vec<Value> = jobs::list().iter().take(20).map(describe).collect();
      Ok(Output::data(
        format!("{} jobs.", jobs.len()),
        json!({"jobs": jobs}),
      ))
    }
    "cancel_job" => match required(args, "job") {
      Ok(id) => match find(&id) {
        Some(job) => {
          jobs::cancel(&job.id);
          Ok(Output::text(format!("Cancelling job {}.", job.id)))
        }
        None => Err(format!("No job {id}.")),
      },
      Err(err) => Err(err),
    },
    _ => return None,
  })
}

fn find(id: &str) -> Option<Job> {
  jobs::load(id).or_else(|| jobs::list().into_iter().find(|j| j.id.starts_with(id)))
}

async fn create(context: &Context, args: &Value) -> Result<Output, String> {
  let store = context.store().await?;
  super::allowed(store).await?;
  let system = text(args, "system");
  let file = text(args, "installer_path");
  if system.is_none() && file.is_none() {
    return Err("Give `system` (see list_systems) or `installer_path`.".into());
  }
  if let Some(id) = &system {
    if vista_kit::catalog::find(id).is_none() {
      return Err(format!("No system \"{id}\"; list_systems shows the ids."));
    }
  }
  let os = text(args, "os").and_then(|o| serde_json::from_value::<Os>(json!(o)).ok());
  let account = match (text(args, "username"), text(args, "password")) {
    (Some(username), Some(password)) => Some(Account {
      fullname: text(args, "fullname").unwrap_or_else(|| username.clone()),
      username,
      password,
      autologin: true,
      ssh: flag(args, "enable_ssh").unwrap_or(false),
    }),
    _ => None,
  };
  let spec = Spec {
    name: text(args, "name").unwrap_or_default(),
    system,
    file,
    os,
    cpus: number(args, "cpus").map(|n| n as u32),
    memory: number(args, "memory_gb").map(|g| (g * 1024.0) as u64),
    disk: number(args, "disk_gb").map(|g| g as u64),
    account,
    sharedownloads: flag(args, "share_downloads").unwrap_or(true),
    rosetta: flag(args, "rosetta").unwrap_or(false),
    start: flag(args, "start").unwrap_or(true),
    visible: flag(args, "visible").unwrap_or(true),
    location: None,
  };
  let job = Job::new(spec);
  jobs::save(&job).map_err(|e| e.to_string())?;
  spawn(&job.id)?;
  let waited = wait(&job.id, number(args, "wait_seconds").unwrap_or(0.0)).await;
  let job = waited.unwrap_or(job);
  Ok(Output::data(
    format!(
      "Job {} {}. Poll get_job with this id (wait_seconds blocks).",
      job.id,
      job.phase.label().to_lowercase()
    ),
    describe(&job),
  ))
}

/// Start `vistamcp job <id>` detached, so it outlives this session.
fn spawn(id: &str) -> Result<(), String> {
  jobs::spawn(id).map_err(|error| error.to_string())
}

async fn wait(id: &str, seconds: f64) -> Option<Job> {
  let deadline = tokio::time::Instant::now() + Duration::from_secs_f64(seconds.clamp(0.0, 600.0));
  loop {
    let job = find(id)?;
    if job.phase.is_finished() || tokio::time::Instant::now() >= deadline {
      return Some(job);
    }
    tokio::time::sleep(Duration::from_millis(1000)).await;
  }
}

async fn get(_context: &Context, args: &Value) -> Result<Output, String> {
  let id = required(args, "job")?;
  let job = find(&id).ok_or_else(|| format!("No job {id}."))?;
  let job = wait(&job.id, number(args, "wait_seconds").unwrap_or(0.0))
    .await
    .unwrap_or(job);
  let summary = match (job.phase, job.fraction()) {
    (vista_kit::jobs::Phase::Downloading, Some(f)) => format!("Downloading: {:.1}%", f * 100.0),
    (phase, _) => phase.label().to_string(),
  };
  Ok(Output::data(summary, describe(&job)))
}

/// The background worker: `vistamcp job <id>`.
pub async fn work(id: &str) -> i32 {
  let Some(mut job) = jobs::load(id) else {
    eprintln!("vistamcp: no job {id}");
    return 1;
  };
  job.pid = std::process::id();
  let _ = jobs::save(&job);
  let store = match vista_kit::store::Store::open().await {
    Ok(store) => store,
    Err(err) => {
      job.transition(
        vista_kit::jobs::Phase::Failed,
        format!("Could not open the creation library: {err}"),
      );
      let _ = jobs::save(&job);
      eprintln!("vistamcp: {err}");
      return 1;
    }
  };
  let finished = jobs::run(&store, job).await;
  if finished.phase == vista_kit::jobs::Phase::Done {
    0
  } else {
    1
  }
}
