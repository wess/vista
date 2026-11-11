use serde::{Deserialize, Serialize};

use crate::bundle::Snapshot;
use crate::machine::Live;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum Request {
  Status,
  /// Bring the machine's window forward, opening it if the machine was
  /// started in the background.
  Show,
  Hide,
  /// Ask the guest to shut itself down, as if its power button were pressed.
  Stop,
  /// Pull the plug.
  Kill,
  Pause,
  Resume,
  /// Save memory to disk and quit; the next start resumes.
  Suspend,
  Restart,
  Snapshot {
    name: String,
  },
  /// The guest's screen as a PNG.
  Screenshot,
  /// Press a key combination: `"enter"`, `"cmd+c"`, `"ctrl+alt+delete"`.
  Key {
    keys: String,
  },
  /// Type text as keystrokes.
  Type {
    text: String,
  },
  /// A pointer action at screenshot pixel coordinates.
  Mouse {
    action: MouseAction,
    x: f64,
    y: f64,
    /// The end point of a drag.
    #[serde(default)]
    to: Option<(f64, f64)>,
  },
  Scroll {
    x: f64,
    y: f64,
    dx: f64,
    dy: f64,
  },
  /// Forward a request to Vista Tools inside the guest.
  Guest {
    request: crate::guest::Request,
  },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseAction {
  Click,
  Double,
  Right,
  Move,
  Down,
  Up,
  Drag,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "lowercase")]
pub enum Response {
  Ok,
  Status(Live),
  Snapshot(Snapshot),
  Image {
    /// Base64 PNG.
    png: String,
    width: u32,
    height: u32,
  },
  /// Vista Tools' answer, in its own field: both enums are tagged on
  /// `result`, so flattening one into the other would collide.
  Guest {
    answer: crate::guest::Response,
  },
  Error {
    message: String,
  },
}

impl Response {
  pub fn error(message: impl Into<String>) -> Self {
    Response::Error {
      message: message.into(),
    }
  }

  pub fn into_result(self) -> crate::Result<Response> {
    match self {
      Response::Error { message } => Err(crate::Error::Message(message)),
      other => Ok(other),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wire_format_is_tagged() {
    let json = serde_json::to_string(&Request::Snapshot { name: "a".into() }).unwrap();
    assert_eq!(json, r#"{"op":"snapshot","name":"a"}"#);
    let json = serde_json::to_string(&Request::Status).unwrap();
    assert_eq!(json, r#"{"op":"status"}"#);
  }
}
