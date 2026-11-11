//! Running commands.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use vista_guest::Exec;

const LIMIT: usize = 1 << 20;

fn collect(mut pipe: impl Read + Send + 'static) -> std::thread::JoinHandle<String> {
  std::thread::spawn(move || {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 64 * 1024];
    while let Ok(n) = pipe.read(&mut chunk) {
      if n == 0 {
        break;
      }
      if buffer.len() < LIMIT {
        buffer.extend_from_slice(&chunk[..n.min(LIMIT - buffer.len())]);
      }
    }
    let mut text = String::from_utf8_lossy(&buffer).to_string();
    if buffer.len() >= LIMIT {
      text.push_str("\n… (truncated at 1 MB)");
    }
    text
  })
}

pub fn run(
  command: &str,
  shell: bool,
  cwd: Option<String>,
  env: BTreeMap<String, String>,
  stdin: Option<String>,
  timeout: u64,
  admin: bool,
) -> Result<Exec, String> {
  let mut process = if shell {
    let mut c = Command::new("/bin/sh");
    c.arg("-c").arg(command);
    c
  } else {
    let mut parts = command.split_whitespace();
    let program = parts.next().ok_or("empty command")?;
    let mut c = Command::new(program);
    c.args(parts);
    c
  };
  let user = if admin { None } else { crate::user::console() };
  match &user {
    Some(user) => {
      process.uid(user.uid).gid(user.gid);
      process.env("HOME", &user.home).env("USER", &user.name).env("LOGNAME", &user.name);
      process.current_dir(cwd.as_deref().unwrap_or(&user.home));
    }
    None => {
      process.env("HOME", "/root");
      process.current_dir(cwd.as_deref().unwrap_or("/root"));
    }
  }
  if std::env::var_os("PATH").is_none() {
    process.env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/opt/homebrew/bin");
  }
  process.envs(env);
  process
    .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
  // Its own process group, so a timeout kills everything it started.
  process.process_group(0);

  let started = Instant::now();
  let mut child = process.spawn().map_err(|e| format!("couldn't run: {e}"))?;
  if let (Some(text), Some(mut pipe)) = (stdin, child.stdin.take()) {
    std::thread::spawn(move || {
      let _ = pipe.write_all(text.as_bytes());
    });
  }
  let stdout = collect(child.stdout.take().ok_or("no stdout")?);
  let stderr = collect(child.stderr.take().ok_or("no stderr")?);
  let limit = Duration::from_secs(timeout.max(1));
  let mut timedout = false;
  let status = loop {
    match child.try_wait() {
      Ok(Some(status)) => break Some(status),
      Ok(None) if started.elapsed() > limit => {
        timedout = true;
        unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) };
        break child.wait().ok();
      }
      Ok(None) => std::thread::sleep(Duration::from_millis(20)),
      Err(_) => break None,
    }
  };
  Ok(Exec {
    code: status.and_then(|s| s.code()),
    stdout: stdout.join().unwrap_or_default(),
    stderr: stderr.join().unwrap_or_default(),
    timedout,
    elapsed: started.elapsed().as_millis() as u64,
  })
}
