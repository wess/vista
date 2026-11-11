//! The library database: an index of machine bundles, app settings, and the
//! installer cache. Migrations are embedded in the binary.

mod images;
mod machines;
mod settings;

pub use images::Image;
pub use machines::Entry;
pub use settings::{Appearance, Settings};

use tokio_rusqlite::Connection;

use crate::Result;

#[derive(Clone, Debug)]
pub struct Store {
  connection: Connection,
}

/// Schema steps, applied in order. The database's `user_version` records
/// how many have run, so each runs exactly once. Append; never edit.
const SCHEMA: &[&str] = &[include_str!("schema/one.sql")];

async fn migrate(connection: &Connection) -> Result<()> {
  connection
    .call(|connection| {
      let applied: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
      for (index, step) in SCHEMA.iter().enumerate().skip(applied.max(0) as usize) {
        let transaction = connection.transaction()?;
        transaction.execute_batch(step)?;
        transaction.pragma_update(None, "user_version", (index + 1) as i64)?;
        transaction.commit()?;
      }
      Ok(())
    })
    .await?;
  Ok(())
}

impl Store {
  /// Open (creating if needed) the library at its usual place.
  pub async fn open() -> Result<Store> {
    crate::paths::ensure()?;
    Self::open_at(&crate::paths::database().to_string_lossy()).await
  }

  pub async fn open_at(path: &str) -> Result<Store> {
    let connection = Connection::open(path).await?;
    connection
      .call(|connection| {
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(())
      })
      .await?;
    migrate(&connection).await?;
    Ok(Store { connection })
  }

  /// An in-memory library, for tests.
  pub async fn memory() -> Result<Store> {
    let connection = Connection::open_in_memory().await?;
    migrate(&connection).await?;
    Ok(Store { connection })
  }

  pub(crate) fn connection(&self) -> &Connection {
    &self.connection
  }
}
