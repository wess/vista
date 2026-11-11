//! The library database: an index of machine bundles, app settings, and the
//! installer cache. SQLite through sqlx, migrations embedded in the binary.

mod images;
mod machines;
mod settings;

pub use images::Image;
pub use machines::Entry;
pub use settings::{Appearance, Settings};

use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::Result;

#[derive(Clone, Debug)]
pub struct Store {
  pool: SqlitePool,
}

/// Schema steps, applied in order. The database's `user_version` records
/// how many have run, so each runs exactly once. Append; never edit.
const SCHEMA: &[&str] = &[include_str!("schema/one.sql")];

async fn migrate(pool: &SqlitePool) -> Result<()> {
  let applied: i64 = sqlx::query_scalar("PRAGMA user_version").fetch_one(pool).await?;
  for (index, step) in SCHEMA.iter().enumerate().skip(applied.max(0) as usize) {
    let mut tx = pool.begin().await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(*step)).execute(&mut *tx).await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("PRAGMA user_version = {}", index + 1)))
      .execute(&mut *tx)
      .await?;
    tx.commit().await?;
  }
  Ok(())
}

impl Store {
  /// Open (creating if needed) the library at its usual place.
  pub async fn open() -> Result<Store> {
    crate::paths::ensure()?;
    Self::open_at(&crate::paths::database().to_string_lossy()).await
  }

  pub async fn open_at(path: &str) -> Result<Store> {
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{path}"))?
      .create_if_missing(true)
      .journal_mode(SqliteJournalMode::Wal)
      .busy_timeout(std::time::Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
      .max_connections(4)
      .connect_with(options)
      .await?;
    migrate(&pool).await?;
    Ok(Store { pool })
  }

  /// An in-memory library, for tests.
  pub async fn memory() -> Result<Store> {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")?;
    let pool = SqlitePoolOptions::new()
      .max_connections(1)
      .connect_with(options)
      .await?;
    migrate(&pool).await?;
    Ok(Store { pool })
  }

  pub(crate) fn pool(&self) -> &SqlitePool {
    &self.pool
  }
}
