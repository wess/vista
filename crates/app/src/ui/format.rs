//! Numbers as people read them.

pub fn bytes(n: u64) -> String {
  let n = n as f64;
  if n >= 1e12 {
    format!("{:.1} TB", n / 1e12)
  } else if n >= 1e9 {
    format!("{:.1} GB", n / 1e9)
  } else if n >= 1e6 {
    format!("{:.0} MB", n / 1e6)
  } else if n >= 1e3 {
    format!("{:.0} KB", n / 1e3)
  } else {
    format!("{n:.0} B")
  }
}

/// A path with the home folder written as `~`.
pub fn tilde(path: &str) -> String {
  let home = vista_kit::paths::home().to_string_lossy().to_string();
  match path.strip_prefix(&home) {
    Some(rest) => format!("~{rest}"),
    None => path.to_string(),
  }
}

/// Disk sizes the way the configuration states them: binary gigabytes.
pub fn gib(bytes: u64) -> String {
  format!("{} GB", bytes / (1024 * 1024 * 1024))
}

pub fn duration(seconds: u64) -> String {
  let (d, h, m, s) = (
    seconds / 86400,
    seconds / 3600 % 24,
    seconds / 60 % 60,
    seconds % 60,
  );
  if d > 0 {
    format!("{d}d {h}h")
  } else if h > 0 {
    format!("{h}h {m}m")
  } else if m > 0 {
    format!("{m}m {s}s")
  } else {
    format!("{s}s")
  }
}

pub fn ago(epoch: i64) -> String {
  let delta = (vista_kit::machine::now() - epoch).max(0) as u64;
  if delta < 60 {
    "just now".into()
  } else {
    format!("{} ago", duration(delta).split(' ').next().unwrap_or(""))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn readable() {
    assert_eq!(bytes(1_500_000_000), "1.5 GB");
    assert_eq!(bytes(2_000_000), "2 MB");
    assert_eq!(duration(3725), "1h 2m");
    assert_eq!(duration(42), "42s");
  }
}
