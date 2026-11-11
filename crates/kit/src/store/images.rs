use sqlx::FromRow;

use super::Store;
use crate::Result;

/// A downloaded installer kept on disk.
#[derive(Clone, Debug, PartialEq, FromRow)]
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
    let rows = sqlx::query_as::<_, Image>(
      "SELECT id, catalog, name, version, url, path, size, sha256, fetched
       FROM images ORDER BY fetched DESC",
    )
    .fetch_all(self.pool())
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
    sqlx::query(
      "INSERT OR REPLACE INTO images (id, catalog, name, version, url, path, size, sha256, fetched)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )
    .bind(&image.id)
    .bind(&image.catalog)
    .bind(&image.name)
    .bind(&image.version)
    .bind(&image.url)
    .bind(&image.path)
    .bind(image.size)
    .bind(&image.sha256)
    .bind(image.fetched)
    .execute(self.pool())
    .await?;
    Ok(())
  }

  pub async fn forget_image(&self, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM images WHERE id = ?1")
      .bind(id)
      .execute(self.pool())
      .await?;
    Ok(())
  }

  /// Delete an image's file and its row.
  pub async fn remove_image(&self, image: &Image) -> Result<()> {
    let _ = std::fs::remove_file(&image.path);
    self.forget_image(&image.id).await
  }
}
