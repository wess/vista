//! The protocol between Vista and Vista Tools, the agent that runs inside a
//! guest. Vista connects to it over virtio-vsock on [`PORT`]; requests and
//! responses are single JSON lines, file contents travel as base64.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The vsock port Vista Tools listens on inside the guest.
pub const PORT: u32 = 7000;

/// Largest file read or written in one request. Bigger transfers belong on a
/// shared folder.
pub const MAX_FILE: u64 = 64 * 1024 * 1024;

/// Largest JSON request or response accepted on a control connection. A
/// 64 MiB file expands while base64 encoded, so leave room for its envelope.
pub const MAX_MESSAGE: u64 = 96 * 1024 * 1024;

/// Commands are interactive operations, not unattended jobs. Bounding this
/// keeps a lost client from leaving a guest process behind indefinitely.
pub const MAX_TIMEOUT: u64 = 60 * 60;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum Request {
  /// Who and what the guest is.
  Hello,
  /// Run a command. With `shell`, `command` goes to `/bin/sh -c`; otherwise
  /// it is split on whitespace and run directly.
  Exec {
    command: String,
    #[serde(default = "yes")]
    shell: bool,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    #[serde(default)]
    stdin: Option<String>,
    /// Seconds before the command is killed.
    #[serde(default = "minute")]
    timeout: u64,
    /// Run as root (the agent runs as root; without this it drops to the
    /// console user when it can).
    #[serde(default)]
    admin: bool,
  },
  Read {
    path: String,
    #[serde(default)]
    offset: u64,
    #[serde(default)]
    length: Option<u64>,
  },
  Write {
    path: String,
    /// Base64.
    data: String,
    #[serde(default)]
    append: bool,
    /// Octal permission bits, e.g. "755".
    #[serde(default)]
    mode: Option<String>,
  },
  List {
    path: String,
  },
  Stat {
    path: String,
  },
  Mkdir {
    path: String,
  },
  Remove {
    path: String,
    #[serde(default)]
    recursive: bool,
  },
}

fn yes() -> bool {
  true
}

fn minute() -> u64 {
  60
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hello {
  pub version: String,
  pub os: String,
  pub arch: String,
  pub hostname: String,
  pub user: String,
  pub kernel: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Exec {
  pub code: Option<i32>,
  pub stdout: String,
  pub stderr: String,
  pub timedout: bool,
  /// Milliseconds.
  pub elapsed: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileInfo {
  pub name: String,
  pub path: String,
  /// "file", "dir", "link", "other"
  pub kind: String,
  pub size: u64,
  /// Seconds since the epoch.
  pub modified: i64,
  /// Octal permission bits.
  pub mode: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "lowercase")]
pub enum Response {
  Hello(Hello),
  Exec(Exec),
  Data {
    /// Base64.
    data: String,
    size: u64,
    eof: bool,
  },
  Entries {
    entries: Vec<FileInfo>,
  },
  Info(FileInfo),
  Ok,
  Error {
    message: String,
  },
}
