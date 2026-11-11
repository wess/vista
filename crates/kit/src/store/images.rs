use tokio_rusqlite::rusqlite::params;

use super::Store;
use crate::Result;

/// A downloaded installer kept on disk.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
  pub id: String,
  /// The catalog entry it was fetched for.
  pub catalog: String,
  pub name: String,
  pub version: String,
  pub url: String,
  pub path: String,
  pub size: i64,
  pub sha256: Option<String>,
  pub fetched: i64,
}

impl Store {
  pub async fn images(&self) -> Result<Vec<Image>> {
    let rows: Vec<Image> = self
      .connection()
      .call(|connection| {
        let mut statement = connection.prepare(
          "SELECT id, catalog, name, version, url, path, size, sha256, fetched
         FROM images ORDER BY fetched DESC",
        )?;
        let rows = statement
          .query_map([], |row| {
            Ok(Image {
              id: row.get(0)?,
              catalog: row.get(1)?,
              name: row.get(2)?,
              version: row.get(3)?,
              url: row.get(4)?,
              path: row.get(5)?,
              size: row.get(6)?,
              sha256: row.get(7)?,
              fetched: row.get(8)?,
            })
          })?
          .collect();
        rows
      })
      .await?;
    // A file deleted behind Vista's back is not an image any more.
    let (present, missing): (Vec<Image>, Vec<Image>) = rows
      .into_iter()
      .partition(|image| std::path::Path::new(&image.path).exists());
    for image in missing {
      self.forget_image(&image.id).await?;
    }
    Ok(present)
  }

  /// The kept image downloaded from exactly this URL, if any.
  pub async fn image_for(&self, url: &str) -> Result<Option<Image>> {
    Ok(
      self
        .images()
        .await?
        .into_iter()
        .find(|image| image.url == url),
    )
  }

  pub async fn keep_image(&self, image: &Image) -> Result<()> {
    let image = image.clone();
    self.connection().call(move |connection| {
      connection.execute(
        "INSERT OR REPLACE INTO images (id, catalog, name, version, url, path, size, sha256, fetched)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![image.id, image.catalog, image.name, image.version, image.url,
          image.path, image.size, image.sha256, image.fetched],
      )?;
      Ok(())
    }).await?;
    Ok(())
  }

  pub async fn forget_image(&self, id: &str) -> Result<()> {
    let id = id.to_string();
    self
      .connection()
      .call(move |connection| {
        connection.execute("DELETE FROM images WHERE id = ?1", [id])?;
        Ok(())
      })
      .await?;
    Ok(())
  }

  /// Delete an image's file and its row.
  pub async fn remove_image(&self, image: &Image) -> Result<()> {
    let _ = std::fs::remove_file(&image.path);
    self.forget_image(&image.id).await
  }
}
