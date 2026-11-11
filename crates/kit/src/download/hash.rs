use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

/// SHA-256 of a file, as lowercase hex. Reads in 4 MiB chunks so a 25 GB
/// restore image doesn't need 25 GB of memory.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
  let mut file = std::fs::File::open(path)?;
  let mut hasher = Sha256::new();
  let mut buffer = vec![0u8; 4 << 20];
  loop {
    let n = file.read(&mut buffer)?;
    if n == 0 {
      break;
    }
    hasher.update(&buffer[..n]);
  }
  Ok(hex(&hasher.finalize()))
}

pub(crate) fn hex(bytes: &[u8]) -> String {
  bytes.iter().map(|b| format!("{b:02x}")).collect()
}
