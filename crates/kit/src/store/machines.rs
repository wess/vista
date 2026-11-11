use std::path::{Path, PathBuf};

use tokio_rusqlite::rusqlite::params;

use super::Store;
use crate::bundle::Bundle;
use crate::Result;

/// A row in the library: where a machine's bundle is, and enough about it to
/// draw the sidebar without opening every bundle.
#[derive(Clone, Debug, PartialEq)]
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
    let rows = self
      .connection()
      .call(|connection| {
        let mut statement = connection.prepare(
          "SELECT id, path, name, os, family, position, created, lastrun
           FROM machines ORDER BY position, created",
        )?;
        let rows = statement
          .query_map([], |row| {
            Ok(Entry {
              id: row.get(0)?,
              path: row.get(1)?,
              name: row.get(2)?,
              os: row.get(3)?,
              family: row.get(4)?,
              position: row.get(5)?,
              created: row.get(6)?,
              lastrun: row.get(7)?,
            })
          })?
          .collect();
        rows
      })
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
    let id = config.id.clone();
    let path = bundle.path.to_string_lossy().to_string();
    let name = config.name.clone();
    let family = config.family.clone();
    let created = config.created;
    self
      .connection()
      .call(move |connection| {
        let next: i64 = connection.query_row(
          "SELECT COALESCE(MAX(position), -1) + 1 FROM machines",
          [],
          |row| row.get(0),
        )?;
        connection.execute(
          "INSERT INTO machines (id, path, name, os, family, position, created)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(id) DO UPDATE SET
           path = excluded.path, name = excluded.name,
           os = excluded.os, family = excluded.family",
          params![id, path, name, os, family, next, created],
        )?;
        Ok(())
      })
      .await?;
    Ok(())
  }

  pub async fn forget(&self, id: &str) -> Result<()> {
    let id = id.to_string();
    self
      .connection()
      .call(move |connection| {
        connection.execute("DELETE FROM machines WHERE id = ?1", [id])?;
        Ok(())
      })
      .await?;
    Ok(())
  }

  pub async fn touch(&self, id: &str) -> Result<()> {
    let id = id.to_string();
    let now = crate::machine::now();
    self
      .connection()
      .call(move |connection| {
        connection.execute(
          "UPDATE machines SET lastrun = ?2 WHERE id = ?1",
          params![id, now],
        )?;
        Ok(())
      })
      .await?;
    Ok(())
  }

  pub async fn reorder(&self, ids: &[String]) -> Result<()> {
    let ids = ids.to_vec();
    self
      .connection()
      .call(move |connection| {
        let transaction = connection.transaction()?;
        for (position, id) in ids.iter().enumerate() {
          transaction.execute(
            "UPDATE machines SET position = ?2 WHERE id = ?1",
            params![id, position as i64],
          )?;
        }
        transaction.commit()?;
        Ok(())
      })
      .await?;
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
