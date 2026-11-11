//! Finding the machine an agent means. Agents may say an id, the first eight
//! characters of one, or a name (any case).

use vista_kit::bundle::{self, Bundle};
use vista_kit::store::{Entry, Store};

pub async fn find(store: &Store, reference: &str) -> Result<(Entry, Bundle), String> {
  let wanted = reference.trim();
  if wanted.is_empty() {
    return Err("Say which machine: its name or id (list_machines shows both).".into());
  }
  let entries = store.machines().await.map_err(|e| e.to_string())?;
  let lower = wanted.to_lowercase();
  let found = entries
    .iter()
    .find(|e| e.id == wanted)
    .or_else(|| entries.iter().find(|e| e.id.starts_with(wanted) && wanted.len() >= 4))
    .or_else(|| entries.iter().find(|e| e.name.to_lowercase() == lower))
    .or_else(|| {
      let partial: Vec<&Entry> = entries
        .iter()
        .filter(|e| e.name.to_lowercase().contains(&lower))
        .collect();
      (partial.len() == 1).then(|| partial[0])
    })
    .cloned()
    .ok_or_else(|| {
      let names: Vec<String> = entries.iter().map(|e| format!("\"{}\"", e.name)).collect();
      format!(
        "No machine called \"{wanted}\". Machines: {}.",
        if names.is_empty() { "none yet".to_string() } else { names.join(", ") }
      )
    })?;
  let bundle = bundle::load(&found.path())
    .map_err(|e| format!("{} is in the library but its files can't be read: {e}", found.name))?;
  Ok((found, bundle))
}
