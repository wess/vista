use regex::Regex;
use serde::Deserialize;

use super::listing::{candidates, links, newest, page};
use super::{parse_sums, Entry, Source};
use crate::{Error, Result};

/// A catalog entry pinned to a concrete file.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
  pub url: String,
  pub filename: String,
  pub version: String,
  pub sha256: Option<String>,
  pub sha1: Option<String>,
  pub size: Option<u64>,
}

fn directory(url: &str) -> &str {
  &url[..url.rfind('/').map(|i| i + 1).unwrap_or(url.len())]
}

fn filename(url: &str) -> String {
  url.rsplit('/').next().unwrap_or("download").to_string()
}

fn regex(pattern: &str) -> Result<Regex> {
  Regex::new(pattern).map_err(|e| Error::msg(e.to_string()))
}

/// A version number out of a file name: "Rocky-10.2-…" → "10.2".
fn version_in(name: &str) -> String {
  let digits = Regex::new(r"\d+(\.\d+)+").expect("version regex");
  digits
    .find(name)
    .map(|m| m.as_str().to_string())
    .unwrap_or_default()
}

#[derive(Deserialize)]
struct PopBuild {
  url: String,
  size: Option<u64>,
  sha_sum: Option<String>,
  #[serde(default)]
  version: Option<String>,
}

/// Find the current installer for an entry.
pub async fn resolve(client: &reqwest::Client, entry: &Entry) -> Result<Resolved> {
  resolve_with_status(client, entry, &mut |_| {}).await
}

pub async fn resolve_with_status(
  client: &reqwest::Client,
  entry: &Entry,
  status: &mut (dyn FnMut(&str) + Send),
) -> Result<Resolved> {
  tokio::time::timeout(
    std::time::Duration::from_secs(90),
    discover(client, entry, status),
  )
  .await
  .map_err(|_| {
    Error::msg(
      "Installer discovery timed out after 90 seconds. The publisher did not respond in time",
    )
  })?
}

async fn discover(
  client: &reqwest::Client,
  entry: &Entry,
  status: &mut (dyn FnMut(&str) + Send),
) -> Result<Resolved> {
  // The directory the file was found in, with its listing, for checksum
  // names given as patterns.
  let mut listed: Option<Vec<String>> = None;
  let mut release: Option<String> = None;
  let url = match entry.source {
    Source::Listing { index, pattern } => {
      status("Reading the publisher’s installer listings.");
      let pattern = regex(pattern)?;
      let mut found = None;
      // Newest release directory first; skip ones with only betas in them.
      for dir in candidates(client, index).await?.into_iter().take(3) {
        let names = links(&page(client, &dir).await?);
        if let Some(file) = newest(names.iter(), &pattern) {
          if let Some(at) = index.find("{n}") {
            release = dir[at..].split('/').next().map(str::to_string);
          }
          let base = if dir.ends_with('/') {
            dir.clone()
          } else {
            format!("{dir}/")
          };
          found = Some(format!("{base}{file}"));
          listed = Some(names);
          break;
        }
      }
      found.ok_or_else(|| {
        Error::msg(format!(
          "{} has no installer published right now",
          entry.title()
        ))
      })?
    }
    Source::Sums { sums, pattern } => {
      status("Reading the publisher’s installer checksums.");
      let pattern = regex(pattern)?;
      let text = page(client, sums).await?;
      let names: Vec<String> = text
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .map(|name| name.trim_start_matches('*').to_string())
        .collect();
      let file = newest(names.iter(), &pattern).ok_or_else(|| {
        Error::msg(format!(
          "{} has no installer published right now",
          entry.title()
        ))
      })?;
      format!("{}{file}", directory(sums))
    }
    Source::Direct { url } => url.to_string(),
    Source::Pop { api } => {
      status("Asking the publisher for the latest release.");
      let build: PopBuild = client
        .get(api)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
      return Ok(Resolved {
        filename: filename(&build.url),
        version: build.version.unwrap_or_else(|| entry.version.to_string()),
        sha256: build.sha_sum,
        sha1: None,
        size: build.size,
        url: build.url,
      });
    }
    Source::Macos { major } => {
      status("Reading Apple’s macOS restore-image catalog.");
      let release = super::macos::release(client, major).await?;
      return Ok(Resolved {
        filename: filename(&release.url),
        url: release.url,
        version: release.version,
        sha256: release.sha256sum,
        sha1: None,
        size: (release.filesize > 0).then_some(release.filesize),
      });
    }
    Source::Windows => {
      let link = crate::windows::download::link_with_status(client, status).await?;
      return Ok(Resolved {
        filename: link.filename.clone(),
        url: link.url,
        version: link.version,
        sha256: link.sha256,
        sha1: link.sha1,
        size: link.size,
      });
    }
  };

  let file = filename(&url);
  let sha256 = match entry.checksum {
    Some(sum) => {
      let location = if sum.url.starts_with('^') {
        let pattern = regex(sum.url)?;
        listed
          .as_ref()
          .and_then(|names| newest(names.iter(), &pattern))
          .map(|name| format!("{}{name}", directory(&url)))
      } else if sum.url.starts_with("http") {
        Some(sum.url.replace("{file}", &file))
      } else {
        Some(format!(
          "{}{}",
          directory(&url),
          sum.url.replace("{file}", &file)
        ))
      };
      match location {
        Some(location) => match page(client, &location).await {
          Ok(text) => parse_sums(&text, &file, sum.format),
          Err(_) => None,
        },
        None => None,
      }
    }
    None => None,
  };
  status("Checking the installer’s download size.");
  let size = client
    .head(&url)
    .send()
    .await
    .ok()
    .filter(|r| r.status().is_success())
    .and_then(|r| {
      r.headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
    })
    .filter(|len| *len > 0);
  let version = if let Some(release) = release {
    release
  } else if entry.version.is_empty() {
    version_in(&file)
  } else {
    entry.version.to_string()
  };
  Ok(Resolved {
    url,
    filename: file,
    version,
    sha256,
    sha1: None,
    size,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn url_parts() {
    let url = "https://example.com/a/b/file.iso";
    assert_eq!(directory(url), "https://example.com/a/b/");
    assert_eq!(filename(url), "file.iso");
    assert_eq!(version_in("Rocky-10.2-aarch64-boot.iso"), "10.2");
    assert_eq!(
      version_in("Fedora-Workstation-Live-45-1.4.aarch64.iso"),
      "1.4"
    );
  }
}
