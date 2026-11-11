//! `vistatools` — Vista Tools, running inside a guest.
//!
//! Listens on virtio-vsock port 7000 and answers one JSON request per
//! connection: run a command, read or write a file, list a directory. Runs as
//! root (systemd, OpenRC or launchd starts it) and drops to the console user
//! for commands unless asked for admin.

mod exec;
mod files;
mod listen;
mod user;

use std::io::{BufRead, BufReader, Write};

use vista_guest::{Hello, Request, Response};

fn main() {
  let args: Vec<String> = std::env::args().collect();
  if args.iter().any(|a| a == "--version") {
    println!("vistatools {}", env!("CARGO_PKG_VERSION"));
    return;
  }
  let listener = match listen::bind(vista_guest::PORT) {
    Ok(fd) => fd,
    Err(err) => {
      eprintln!("vistatools: can't listen on vsock port {}: {err}", vista_guest::PORT);
      std::process::exit(1);
    }
  };
  eprintln!("vistatools: listening on vsock port {}", vista_guest::PORT);
  loop {
    match listen::accept(listener) {
      Ok(stream) => {
        std::thread::spawn(move || serve(stream));
      }
      Err(err) => {
        eprintln!("vistatools: accept failed: {err}");
        std::thread::sleep(std::time::Duration::from_millis(200));
      }
    }
  }
}

fn serve(stream: std::fs::File) {
  let Ok(writer) = stream.try_clone() else { return };
  let mut writer = writer;
  let mut reader = BufReader::with_capacity(1 << 20, stream);
  let mut line = String::new();
  if reader.read_line(&mut line).unwrap_or(0) == 0 {
    return;
  }
  let response = match serde_json::from_str::<Request>(line.trim()) {
    Ok(request) => handle(request),
    Err(err) => Response::Error {
      message: format!("bad request: {err}"),
    },
  };
  if let Ok(mut text) = serde_json::to_string(&response) {
    text.push('\n');
    let _ = writer.write_all(text.as_bytes());
    let _ = writer.flush();
  }
}

fn handle(request: Request) -> Response {
  let result = match request {
    Request::Hello => Ok(Response::Hello(hello())),
    Request::Exec {
      command,
      shell,
      cwd,
      env,
      stdin,
      timeout,
      admin,
    } => exec::run(&command, shell, cwd, env, stdin, timeout, admin).map(Response::Exec),
    Request::Read { path, offset, length } => files::read(&path, offset, length),
    Request::Write { path, data, append, mode } => files::write(&path, &data, append, mode),
    Request::List { path } => files::list(&path),
    Request::Stat { path } => files::stat(&path).map(Response::Info),
    Request::Mkdir { path } => files::mkdir(&path),
    Request::Remove { path, recursive } => files::remove(&path, recursive),
  };
  result.unwrap_or_else(|message| Response::Error { message })
}

fn read_trim(path: &str) -> String {
  std::fs::read_to_string(path).map(|s| s.trim().to_string()).unwrap_or_default()
}

fn os_name() -> String {
  if cfg!(target_os = "macos") {
    let output = std::process::Command::new("/usr/bin/sw_vers").output();
    let text = output.map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    let field = |key: &str| {
      text
        .lines()
        .find_map(|l| l.strip_prefix(key).map(|v| v.trim().to_string()))
        .unwrap_or_default()
    };
    format!("{} {} ({})", field("ProductName:"), field("ProductVersion:"), field("BuildVersion:"))
  } else {
    read_trim("/etc/os-release")
      .lines()
      .find_map(|l| l.strip_prefix("PRETTY_NAME="))
      .map(|v| v.trim_matches('"').to_string())
      .unwrap_or_else(|| "Linux".into())
  }
}

fn hello() -> Hello {
  let mut uts: libc::utsname = unsafe { std::mem::zeroed() };
  unsafe { libc::uname(&mut uts) };
  let field = |raw: &[libc::c_char]| unsafe { std::ffi::CStr::from_ptr(raw.as_ptr()) }.to_string_lossy().to_string();
  Hello {
    version: env!("CARGO_PKG_VERSION").into(),
    os: os_name(),
    arch: field(&uts.machine),
    hostname: field(&uts.nodename),
    user: user::console().map(|u| u.name).unwrap_or_else(|| "root".into()),
    kernel: format!("{} {}", field(&uts.sysname), field(&uts.release)),
  }
}
