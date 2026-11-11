//! Finding a guest's address on the shared network.
//!
//! Shared (NAT) networking hands out addresses with macOS's own DHCP server,
//! which records every lease in `/var/db/dhcpd_leases`. Each machine keeps a
//! fixed MAC address, so its lease is easy to find.

use std::path::Path;

const LEASES: &str = "/var/db/dhcpd_leases";

/// The newest leased IPv4 address for `mac`.
pub fn lease(mac: &str) -> Option<String> {
  let text = std::fs::read_to_string(Path::new(LEASES)).ok()?;
  find(&text, mac)
}

/// The lease file writes MAC bytes without leading zeros (`1:2:3a:…`).
fn normalize(mac: &str) -> String {
  mac
    .split(':')
    .map(|part| {
      let trimmed = part.trim_start_matches('0');
      if trimmed.is_empty() { "0" } else { trimmed }.to_lowercase()
    })
    .collect::<Vec<_>>()
    .join(":")
}

pub fn find(text: &str, mac: &str) -> Option<String> {
  let want = normalize(mac);
  let mut best: Option<(i64, String)> = None;
  for block in text.split('}') {
    let mut ip = None;
    let mut hw = None;
    let mut stamp = 0i64;
    for line in block.lines().map(str::trim) {
      if let Some(value) = line.strip_prefix("ip_address=") {
        ip = Some(value.to_string());
      } else if let Some(value) = line.strip_prefix("hw_address=") {
        hw = value.split_once(',').map(|(_, mac)| normalize(mac));
      } else if let Some(value) = line.strip_prefix("lease=") {
        stamp = i64::from_str_radix(value.trim_start_matches("0x"), 16).unwrap_or(0);
      }
    }
    if let (Some(ip), Some(hw)) = (ip, hw) {
      if hw == want && best.as_ref().map(|(s, _)| stamp > *s).unwrap_or(true) {
        best = Some((stamp, ip));
      }
    }
  }
  best.map(|(_, ip)| ip)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn finds_the_newest_lease() {
    let text = "{\n\tname=ubuntu\n\tip_address=192.168.64.3\n\thw_address=1,2:a:3c:4:5:6\n\tidentifier=1,2:a:3c:4:5:6\n\tlease=0x6a000000\n}\n{\n\tname=ubuntu\n\tip_address=192.168.64.9\n\thw_address=1,2:a:3c:4:5:6\n\tlease=0x6b000000\n}\n";
    assert_eq!(
      find(text, "02:0a:3c:04:05:06").as_deref(),
      Some("192.168.64.9")
    );
    assert_eq!(find(text, "02:0a:3c:04:05:07"), None);
  }
}
