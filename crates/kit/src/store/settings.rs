//! App preferences. Each field is its own row so a newer Vista can add one
//! without a migration, and an older one ignores keys it doesn't know.

use serde::{Deserialize, Serialize};

use super::Store;
use crate::Result;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
  #[default]
  System,
  Light,
  Dark,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
  pub appearance: Appearance,
  /// Where new machines are created.
  pub location: String,
  /// Show Vista in the menu bar.
  pub menubar: bool,
  /// Keep downloaded installers for the next machine of the same system.
  pub keepinstallers: bool,
  /// Share the Mac's Downloads folder with new machines.
  pub sharedownloads: bool,
  /// Coding agents on this Mac may manage machines through the MCP server.
  pub agents: bool,
}

impl Default for Settings {
  fn default() -> Self {
    Settings {
      appearance: Appearance::System,
      location: crate::paths::machines().to_string_lossy().to_string(),
      menubar: true,
      keepinstallers: true,
      sharedownloads: true,
      agents: true,
    }
  }
}

impl Store {
  pub async fn setting<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
    let row: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?1")
      .bind(key)
      .fetch_optional(self.pool())
      .await?;
    Ok(row.and_then(|value| serde_json::from_str(&value).ok()))
  }

  pub async fn set_setting<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
    sqlx::query(
      "INSERT INTO settings (key, value) VALUES (?1, ?2)
       ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(serde_json::to_string(value)?)
    .execute(self.pool())
    .await?;
    Ok(())
  }

  /// Every preference, falling back to the default for any key not stored.
  pub async fn settings(&self) -> Result<Settings> {
    let defaults = serde_json::to_value(Settings::default())?;
    let mut merged = serde_json::Map::new();
    if let serde_json::Value::Object(fields) = defaults {
      for (key, fallback) in fields {
        let stored: Option<serde_json::Value> = self.setting(&key).await?;
        merged.insert(key, stored.unwrap_or(fallback));
      }
    }
    Ok(serde_json::from_value(serde_json::Value::Object(merged)).unwrap_or_default())
  }

  pub async fn save_settings(&self, settings: &Settings) -> Result<()> {
    if let serde_json::Value::Object(fields) = serde_json::to_value(settings)? {
      for (key, value) in fields {
        self.set_setting(&key, &value).await?;
      }
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn settings_merge_defaults() {
    let store = Store::memory().await.unwrap();
    assert_eq!(store.settings().await.unwrap(), Settings::default());
    store.set_setting("menubar", &false).await.unwrap();
    let settings = store.settings().await.unwrap();
    assert!(!settings.menubar);
    assert!(settings.keepinstallers);
  }
}
