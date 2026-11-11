//! Files and directories.

use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use base64::Engine;
use vista_guest::{FileInfo, Response, MAX_FILE};

fn b64() -> base64::engine::GeneralPurpose {
  base64::engine::general_purpose::STANDARD
}

fn error(path: &str, err: std::io::Error) -> String {
  format!("{path}: {err}")
}

pub fn info(path: &Path) -> std::io::Result<FileInfo> {
  let meta = std::fs::symlink_metadata(path)?;
  let kind = if meta.file_type().is_symlink() {
    "link"
  } else if meta.is_dir() {
    "dir"
  } else if meta.is_file() {
    "file"
  } else {
    "other"
  };
  Ok(FileInfo {
    name: path
      .file_name()
      .map(|n| n.to_string_lossy().to_string())
      .unwrap_or_else(|| "/".into()),
    path: path.to_string_lossy().to_string(),
    kind: kind.into(),
    size: meta.len(),
    modified: meta.mtime(),
    mode: format!("{:o}", meta.mode() & 0o7777),
  })
}

pub fn stat(path: &str) -> Result<FileInfo, String> {
  info(Path::new(path)).map_err(|e| error(path, e))
}

pub fn read(path: &str, offset: u64, length: Option<u64>) -> Result<Response, String> {
  let mut file = std::fs::File::open(path).map_err(|e| error(path, e))?;
  let size = file.metadata().map_err(|e| error(path, e))?.len();
  let want = length.unwrap_or(MAX_FILE).min(MAX_FILE);
  file.seek(SeekFrom::Start(offset)).map_err(|e| error(path, e))?;
  let mut buffer = Vec::new();
  file.take(want).read_to_end(&mut buffer).map_err(|e| error(path, e))?;
  let eof = offset + buffer.len() as u64 >= size;
  Ok(Response::Data {
    data: b64().encode(&buffer),
    size,
    eof,
  })
}

pub fn write(path: &str, data: &str, append: bool, mode: Option<String>) -> Result<Response, String> {
  let bytes = b64().decode(data).map_err(|e| format!("bad base64: {e}"))?;
  if let Some(parent) = Path::new(path).parent() {
    std::fs::create_dir_all(parent).map_err(|e| error(path, e))?;
  }
  let mut file = std::fs::OpenOptions::new()
    .create(true)
    .write(true)
    .append(append)
    .truncate(!append)
    .open(path)
    .map_err(|e| error(path, e))?;
  file.write_all(&bytes).map_err(|e| error(path, e))?;
  if let Some(mode) = mode {
    let bits = u32::from_str_radix(&mode, 8).map_err(|_| format!("bad mode {mode}"))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(bits)).map_err(|e| error(path, e))?;
  }
  Ok(Response::Ok)
}

pub fn list(path: &str) -> Result<Response, String> {
  let mut entries: Vec<FileInfo> = std::fs::read_dir(path)
    .map_err(|e| error(path, e))?
    .flatten()
    .filter_map(|entry| info(&entry.path()).ok())
    .collect();
  entries.sort_by(|a, b| a.name.cmp(&b.name));
  Ok(Response::Entries { entries })
}

pub fn mkdir(path: &str) -> Result<Response, String> {
  std::fs::create_dir_all(path).map_err(|e| error(path, e))?;
  Ok(Response::Ok)
}

pub fn remove(path: &str, recursive: bool) -> Result<Response, String> {
  let meta = std::fs::symlink_metadata(path).map_err(|e| error(path, e))?;
  if meta.is_dir() {
    if recursive {
      std::fs::remove_dir_all(path)
    } else {
      std::fs::remove_dir(path)
    }
  } else {
    std::fs::remove_file(path)
  }
  .map_err(|e| error(path, e))?;
  Ok(Response::Ok)
}
