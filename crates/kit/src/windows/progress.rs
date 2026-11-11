use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct Installation {
  pub result: Option<bool>,
  pub fraction: f64,
  pub message: String,
}

pub fn parse(log: &str, result: &str) -> Installation {
  let result = match result.trim() {
    "OK" => Some(true),
    "FAILED" => Some(false),
    _ => None,
  };
  let percentage = regex::Regex::new(r"([0-9]+(?:\.[0-9]+)?)%")
    .expect("installer percentage")
    .captures_iter(log)
    .last()
    .and_then(|capture| capture[1].parse::<f64>().ok())
    .unwrap_or(0.)
    .clamp(0., 100.);
  let (fraction, message) = if result == Some(true) {
    (1., "Windows installation complete".into())
  } else if result == Some(false) {
    let last = log
      .lines()
      .rev()
      .find(|line| {
        let line = line.trim();
        !line.is_empty() && !line.contains("Installation failed")
      })
      .unwrap_or("See the installation log for details.")
      .trim();
    (
      percentage / 125.,
      format!("Windows installation failed: {last}"),
    )
  } else if log.contains("Installing virtual hardware drivers") {
    (0.9, "Installing Windows hardware drivers".into())
  } else if log.contains("Applying Windows 11 Pro") {
    (
      percentage / 125.,
      format!("Applying Windows 11 Pro · {percentage:.0}%"),
    )
  } else {
    (0., "Starting Windows Setup".into())
  };
  Installation {
    result,
    fraction,
    message,
  }
}

struct Fat {
  file: File,
  size: u64,
  fat: u64,
  data: u64,
  cluster: u64,
  root: u32,
}

impl Fat {
  fn bytes(&mut self, offset: u64, size: usize) -> Option<Vec<u8>> {
    if offset.checked_add(size as u64)? > self.size {
      return None;
    }
    self.file.seek(SeekFrom::Start(offset)).ok()?;
    let mut bytes = vec![0; size];
    self.file.read_exact(&mut bytes).ok()?;
    Some(bytes)
  }

  fn chain(&mut self, mut cluster: u32, limit: usize) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    let mut seen = std::collections::HashSet::new();
    while (2..0x0ffffff8).contains(&cluster) && output.len() < limit && seen.insert(cluster) {
      let offset = self
        .data
        .checked_add((cluster as u64 - 2).checked_mul(self.cluster)?)?;
      output.extend(self.bytes(offset, (self.cluster as usize).min(limit - output.len()))?);
      let next = self.bytes(self.fat.checked_add(cluster as u64 * 4)?, 4)?;
      cluster = u32::from_le_bytes(next.try_into().ok()?) & 0x0fffffff;
    }
    Some(output)
  }

  fn entry(&mut self, directory: u32, name: &[u8; 11]) -> Option<(u32, usize)> {
    let entries = self.chain(directory, 128 * 1024)?;
    for entry in entries.chunks_exact(32) {
      if entry[0] == 0 {
        break;
      }
      if entry[0] == 229 || entry[11] == 15 || &entry[..11] != name {
        continue;
      }
      let cluster = ((u16::from_le_bytes(entry[20..22].try_into().ok()?) as u32) << 16)
        | u16::from_le_bytes(entry[26..28].try_into().ok()?) as u32;
      return Some((
        cluster,
        u32::from_le_bytes(entry[28..32].try_into().ok()?) as usize,
      ));
    }
    None
  }

  fn read(&mut self, name: &[u8; 11]) -> Option<String> {
    let (directory, _) = self.entry(self.root, b"VISTA      ")?;
    let (cluster, size) = self.entry(directory, name)?;
    if size == 0 {
      return Some(String::new());
    }
    let bytes = self.chain(cluster, size.min(512 * 1024))?;
    Some(
      String::from_utf8_lossy(&bytes)
        .trim_end_matches('\0')
        .to_string(),
    )
  }
}

pub fn read(path: &Path) -> Option<Installation> {
  let file = File::open(path).ok()?;
  let size = file.metadata().ok()?.len();
  let mut image = Fat {
    file,
    size,
    fat: 0,
    data: 0,
    cluster: 0,
    root: 0,
  };
  let header = image.bytes(512, 92)?;
  if &header[..8] != b"EFI PART" {
    return None;
  }
  let table = u64::from_le_bytes(header[72..80].try_into().ok()?).checked_mul(512)?;
  let count = u32::from_le_bytes(header[80..84].try_into().ok()?).min(128);
  let width = u32::from_le_bytes(header[84..88].try_into().ok()?) as u64;
  if !(128..=1024).contains(&width) {
    return None;
  }
  for index in 0..count {
    let entry = image.bytes(table.checked_add(index as u64 * width)?, 128)?;
    if entry[..16] == [0; 16] {
      continue;
    }
    let start = u64::from_le_bytes(entry[32..40].try_into().ok()?).checked_mul(512)?;
    let boot = image.bytes(start, 512)?;
    if &boot[82..90] != b"FAT32   " {
      continue;
    }
    let sector = u16::from_le_bytes(boot[11..13].try_into().ok()?) as u64;
    let cluster = boot[13] as u64;
    if !(512..=4096).contains(&sector)
      || !sector.is_power_of_two()
      || !cluster.is_power_of_two()
      || cluster > 128
    {
      continue;
    }
    let reserved = u16::from_le_bytes(boot[14..16].try_into().ok()?) as u64;
    let fats = boot[16] as u64;
    let sectors = u32::from_le_bytes(boot[36..40].try_into().ok()?) as u64;
    image.fat = start.checked_add(reserved.checked_mul(sector)?)?;
    image.data = image
      .fat
      .checked_add(fats.checked_mul(sectors)?.checked_mul(sector)?)?;
    image.cluster = cluster * sector;
    image.root = u32::from_le_bytes(boot[44..48].try_into().ok()?);
    if image.entry(image.root, b"VISTA      ").is_some() {
      return Some(parse(
        &image.read(b"INSTALL LOG").unwrap_or_default(),
        &image.read(b"RESULT     ").unwrap_or_default(),
      ));
    }
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn reads_installation_status_from_the_generated_gpt_fat_volume() {
    let path = std::env::temp_dir().join(format!("vistaprogress{}", uuid::Uuid::new_v4().simple()));
    let mut bytes = vec![0u8; 16 * 512];
    bytes[512..520].copy_from_slice(b"EFI PART");
    bytes[584..592].copy_from_slice(&2u64.to_le_bytes());
    bytes[592..596].copy_from_slice(&1u32.to_le_bytes());
    bytes[596..600].copy_from_slice(&128u32.to_le_bytes());
    bytes[1024] = 1;
    bytes[1056..1064].copy_from_slice(&8u64.to_le_bytes());
    let boot = 8 * 512;
    bytes[boot + 11..boot + 13].copy_from_slice(&512u16.to_le_bytes());
    bytes[boot + 13] = 1;
    bytes[boot + 14..boot + 16].copy_from_slice(&1u16.to_le_bytes());
    bytes[boot + 16] = 1;
    bytes[boot + 36..boot + 40].copy_from_slice(&1u32.to_le_bytes());
    bytes[boot + 44..boot + 48].copy_from_slice(&2u32.to_le_bytes());
    bytes[boot + 82..boot + 90].copy_from_slice(b"FAT32   ");
    for cluster in 2..=5 {
      bytes[9 * 512 + cluster * 4..9 * 512 + cluster * 4 + 4]
        .copy_from_slice(&0x0fffffffu32.to_le_bytes());
    }
    fn entry(bytes: &mut [u8], offset: usize, name: &[u8; 11], cluster: u16, size: u32) {
      bytes[offset..offset + 11].copy_from_slice(name);
      bytes[offset + 26..offset + 28].copy_from_slice(&cluster.to_le_bytes());
      bytes[offset + 28..offset + 32].copy_from_slice(&size.to_le_bytes());
    }
    entry(&mut bytes, 10 * 512, b"VISTA      ", 3, 0);
    let log = b"Applying Windows 11 Pro\r\n[ 35.0% ]";
    entry(&mut bytes, 11 * 512, b"INSTALL LOG", 4, log.len() as u32);
    entry(&mut bytes, 11 * 512 + 32, b"RESULT     ", 5, 4);
    bytes[12 * 512..12 * 512 + log.len()].copy_from_slice(log);
    bytes[13 * 512..13 * 512 + 4].copy_from_slice(b"OK\r\n");
    std::fs::write(&path, bytes).unwrap();
    let status = read(&path).unwrap();
    assert_eq!(status.result, Some(true));
    assert_eq!(status.fraction, 1.);
    std::fs::remove_file(path).unwrap();
  }

  #[test]
  fn installation_tracks_real_work_and_requires_explicit_success() {
    let applying = parse("Applying Windows 11 Pro\n[ 35.0% ]", "");
    assert_eq!(applying.fraction, 0.28);
    assert_eq!(applying.result, None);
    assert!(parse("Applying Windows 11 Pro\n100.0%", "")
      .result
      .is_none());
    assert_eq!(
      parse("Installing virtual hardware drivers", "").fraction,
      0.9
    );
    assert_eq!(parse("", "OK\r\n").result, Some(true));
    assert_eq!(
      parse("Error: 0x8007000e\nInstallation failed.", "FAILED").result,
      Some(false)
    );
  }
}
