use std::path::Path;

use objc2::rc::Retained;
use objc2_foundation::{NSData, NSError, NSString, NSURL};

pub fn url(path: &Path) -> Retained<NSURL> {
  let path = NSString::from_str(&path.to_string_lossy());
  NSURL::fileURLWithPath(&path)
}

pub fn string(text: &str) -> Retained<NSString> {
  NSString::from_str(text)
}

/// What a person should read when the framework refuses something.
pub fn describe(err: &NSError) -> String {
  let text = err.localizedDescription().to_string();
  let reason = err
    .localizedFailureReason()
    .map(|r| r.to_string())
    .unwrap_or_default();
  if reason.is_empty() || text.contains(&reason) {
    text
  } else {
    format!("{text} {reason}")
  }
}

pub fn read_data(path: &Path) -> Option<Retained<NSData>> {
  std::fs::read(path)
    .ok()
    .map(|bytes| NSData::with_bytes(&bytes))
}

pub fn write_data(path: &Path, data: &NSData) -> std::io::Result<()> {
  std::fs::write(path, data.to_vec())
}
