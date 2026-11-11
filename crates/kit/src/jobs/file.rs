use super::Job;
use crate::Result;

fn path(id: &str) -> std::path::PathBuf {
  crate::paths::jobs().join(format!("{}.json", crate::paths::short(id)))
}

pub fn save(job: &Job) -> Result<()> {
  std::fs::create_dir_all(crate::paths::jobs())?;
  let mut job = job.clone();
  job.updated = crate::machine::now();
  let temp = path(&job.id).with_extension("new");
  std::fs::write(&temp, serde_json::to_string_pretty(&job)?)?;
  std::fs::rename(temp, path(&job.id))?;
  Ok(())
}

pub fn load(id: &str) -> Option<Job> {
  let text = std::fs::read_to_string(path(id)).ok()?;
  serde_json::from_str(&text).ok()
}

/// Every job, newest first.
pub fn list() -> Vec<Job> {
  let Ok(entries) = std::fs::read_dir(crate::paths::jobs()) else {
    return Vec::new();
  };
  let mut jobs: Vec<Job> = entries
    .flatten()
    .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
    .filter_map(|e| std::fs::read_to_string(e.path()).ok())
    .filter_map(|text| serde_json::from_str(&text).ok())
    .collect();
  jobs.sort_by_key(|j: &Job| std::cmp::Reverse(j.created));
  jobs
}

pub fn remove(id: &str) {
  let _ = std::fs::remove_file(path(id));
}
