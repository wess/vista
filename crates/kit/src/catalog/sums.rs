//! Checksum files, in the three shapes distributions publish them.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SumFormat {
  /// `HASH  filename` (GNU coreutils), optionally `*filename`.
  Gnu,
  /// `SHA256 (filename) = HASH` (BSD, Fedora, openSUSE).
  Bsd,
  /// A file for one image: just `HASH`, or `HASH filename`.
  Single,
}

/// Find the SHA-256 for `file` in a checksum listing. PGP-clearsigned files
/// parse as-is: the signature lines simply don't match.
pub fn parse_sums(text: &str, file: &str, format: SumFormat) -> Option<String> {
  let is_hash = |s: &str| s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit());
  for line in text.lines().map(str::trim) {
    match format {
      SumFormat::Gnu | SumFormat::Single => {
        let mut parts = line.split_whitespace();
        let Some(hash) = parts.next().filter(|h| is_hash(h)) else {
          continue;
        };
        let name = parts.next().map(|n| n.trim_start_matches('*'));
        if format == SumFormat::Single || name == Some(file) {
          return Some(hash.to_lowercase());
        }
      }
      SumFormat::Bsd => {
        let Some(rest) = line.strip_prefix("SHA256 (") else {
          continue;
        };
        let Some((name, hash)) = rest.split_once(") = ") else {
          continue;
        };
        if name == file && is_hash(hash.trim()) {
          return Some(hash.trim().to_lowercase());
        }
      }
    }
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;

  const H: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

  #[test]
  fn gnu() {
    let text = format!("{H}  other.iso\n{H} *ubuntu.iso\n");
    assert_eq!(
      parse_sums(&text, "ubuntu.iso", SumFormat::Gnu).as_deref(),
      Some(H)
    );
    assert_eq!(parse_sums(&text, "nope.iso", SumFormat::Gnu), None);
  }

  #[test]
  fn bsd_inside_a_signature() {
    let text = format!(
      "-----BEGIN PGP SIGNED MESSAGE-----\nHash: SHA256\n\n# Fedora.iso: 1 bytes\nSHA256 (Fedora.iso) = {H}\n-----BEGIN PGP SIGNATURE-----\n"
    );
    assert_eq!(
      parse_sums(&text, "Fedora.iso", SumFormat::Bsd).as_deref(),
      Some(H)
    );
  }

  #[test]
  fn single() {
    assert_eq!(
      parse_sums(H, "x.iso", SumFormat::Single).as_deref(),
      Some(H)
    );
    let text = format!("{H}  x.iso");
    assert_eq!(
      parse_sums(&text, "x.iso", SumFormat::Single).as_deref(),
      Some(H)
    );
  }
}
