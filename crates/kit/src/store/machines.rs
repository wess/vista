use std::path::{Path, PathBuf};

use sqlx::FromRow;

use super::Store;
use crate::bundle::Bundle;
use crate::Result;

/// A row in the library: where a machine's bundle is, and enough about it to
/// draw the sidebar without opening every bundle.
#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct Entry {
  pub id: String,
  pub path: String,
  pub name: String,
  pub os: String,
  pub family: String,
  pub position: i64,
  pub created: i64,
  pub lastrun: Option<i64>,
}

impl Entry {
  pub fn path(&self) -> PathBuf {
    PathBuf::from(&self.path)
  }
}

impl Store {
  pub async fn machines(&self) -> Result<Vec<Entry>> {
    let rows = sqlx::query_as::<_, Entry>(
      "SELECT id, path, name, os, family, position, created, lastrun
       FROM machines ORDER BY position, created",
    )
    .fetch_all(self.pool())
    .await?;
    Ok(rows)
  }

  /// Add or refresh a bundle in the library.
  pub async fn register(&self, bundle: &Bundle) -> Result<()> {
    let config = &bundle.config;
    let os = serde_json::to_value(config.os)?
      .as_str()
      .unwrap_or("other")
      .to_string();
    let next: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(position), -1) + 1 FROM machines")
      .fetch_one(self.pool())
      .await?;
    sqlx::query(
      "INSERT INTO machines (id, path, name, os, family, position, created)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
       ON CONFLICT(id) DO UPDATE SET
         path = excluded.path, name = excluded.name,
         os = excluded.os, family = excluded.family",
    )
    .bind(&config.id)
    .bind(bundle.path.to_string_lossy().to_string())
    .bind(&config.name)
    .bind(os)
    .bind(&config.family)
    .bind(next)
    .bind(config.created)
    .execute(self.pool())
    .await?;
    Ok(())
  }

  pub async fn forget(&self, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM machines WHERE id = ?1")
      .bind(id)
      .execute(self.pool())
      .await?;
    Ok(())
  }

  pub async fn touch(&self, id: &str) -> Result<()> {
    sqlx::query("UPDATE machines SET lastrun = ?2 WHERE id = ?1")
      .bind(id)
      .bind(crate::machine::now())
      .execute(self.pool())
      .await?;
    Ok(())
  }

  pub async fn reorder(&self, ids: &[String]) -> Result<()> {
    let mut tx = self.pool().begin().await?;
    for (position, id) in ids.iter().enumerate() {
      sqlx::query("UPDATE machines SET position = ?2 WHERE id = ?1")
        .bind(id)
        .bind(position as i64)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
  }

  /// Find bundles in `location` that the library doesn't know yet — ones
  /// copied in from another Mac, or left behind by a reinstall.
  pub async fn adopt(&self, location: &Path) -> Result<usize> {
    let known: Vec<String> = self.machines().await?.into_iter().map(|e| e.path).collect();
    let Ok(entries) = std::fs::read_dir(location) else {
      return Ok(0);
    };
    let mut added = 0;
    for entry in entries.flatten() {
      let path = entry.path();
      let is_bundle = path
        .extension()
        .map(|ext| ext == crate::bundle::EXTENSION)
        .unwrap_or(false);
      if !is_bundle || known.contains(&path.to_string_lossy().to_string()) {
        continue;
      }
      if let Ok(bundle) = crate::bundle::load(&path) {
        self.register(&bundle).await?;
        added += 1;
      }
    }
    Ok(added)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::machine::{Config, Os};

  #[tokio::test]
  async fn register_list_forget() {
    let store = Store::memory().await.unwrap();
    let a = Bundle {
      path: "/tmp/a.vista".into(),
      config: Config::new("A", Os::Linux),
    };
    let b = Bundle {
      path: "/tmp/b.vista".into(),
      config: Config::new("B", Os::Macos),
    };
    store.register(&a).await.unwrap();
    store.register(&b).await.unwrap();
    let list = store.machines().await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].name, "A");
    assert_eq!(list[1].os, "macos");

    store
      .reorder(&[b.config.id.clone(), a.config.id.clone()])
      .await
      .unwrap();
    assert_eq!(store.machines().await.unwrap()[0].name, "B");

    store.forget(&a.config.id).await.unwrap();
    assert_eq!(store.machines().await.unwrap().len(), 1);
  }
}
