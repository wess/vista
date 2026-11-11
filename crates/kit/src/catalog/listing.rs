//! Reading directory indexes: the newest file that matches a pattern.

use regex::Regex;

use crate::{Error, Result};

/// Every `href` in an HTML page, decoded enough to compare file names.
pub fn links(html: &str) -> Vec<String> {
  let href = Regex::new(r#"(?i)href\s*=\s*["']([^"'#?]+)["']"#).expect("href regex");
  href
    .captures_iter(html)
    .map(|c| c[1].replace("%21", "!").replace("%20", " "))
    .map(|link| {
      link
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string()
    })
    .filter(|name| !name.is_empty() && name != ".." && name != ".")
    .collect()
}

fn numbers(text: &str) -> Vec<u64> {
  text
    .split(|c: char| !c.is_ascii_digit())
    .filter(|part| !part.is_empty())
    .map(|part| part.parse::<u64>().unwrap_or(0))
    .collect()
}

/// Order names by version: the first dotted number in the name decides
/// (`ubuntu-26.04.1-…-arm64` beats `ubuntu-26.04-…-arm64`, `44` beats `9`),
/// and every number in the name breaks ties (dated snapshots).
pub fn version_key(name: &str) -> (Vec<u64>, Vec<u64>) {
  let first = name
    .find(|c: char| c.is_ascii_digit())
    .map(|start| {
      let rest = &name[start..];
      let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(rest.len());
      numbers(rest[..end].trim_end_matches('.'))
    })
    .unwrap_or_default();
  (first, numbers(name))
}

pub fn newest<'a>(names: impl IntoIterator<Item = &'a String>, pattern: &Regex) -> Option<String> {
  names
    .into_iter()
    .filter(|name| pattern.is_match(name))
    .max_by(|a, b| version_key(a).cmp(&version_key(b)))
    .cloned()
}

/// Fetch a page, retrying server errors: some mirrors (Debian's) answer 500
/// a few times in ten.
pub async fn page(client: &reqwest::Client, url: &str) -> Result<String> {
  let mut wait = std::time::Duration::from_millis(400);
  for attempt in 0..4 {
    let response = client.get(url).send().await?;
    let status = response.status();
    if status.is_success() {
      return Ok(response.text().await?);
    }
    if !status.is_server_error() || attempt == 3 {
      return Err(Error::msg(format!("{url} answered {status}")));
    }
    tokio::time::sleep(wait).await;
    wait *= 2;
  }
  unreachable!()
}

/// The candidates for `{n}` in an index URL, newest first: numbered
/// directories of the path before it. A URL without `{n}` is its own only
/// candidate.
pub async fn candidates(client: &reqwest::Client, index: &str) -> Result<Vec<String>> {
  let Some(at) = index.find("{n}") else {
    return Ok(vec![index.to_string()]);
  };
  let parent = &index[..at];
  let html = page(client, parent).await?;
  let numbered = Regex::new(r"^\d+(\.\d+)*$").expect("numbered regex");
  let mut names: Vec<String> = links(&html)
    .into_iter()
    .filter(|name| numbered.is_match(name))
    .collect();
  names.sort_by_key(|name| std::cmp::Reverse(version_key(name)));
  names.dedup();
  if names.is_empty() {
    return Err(Error::msg(format!("no releases listed at {parent}")));
  }
  Ok(
    names
      .into_iter()
      .map(|n| index.replacen("{n}", &n, 1))
      .collect(),
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn picks_the_newest_point_release() {
    let html = r#"<a href="ubuntu-26.04.9-desktop-arm64.iso">x</a>
      <a href="ubuntu-26.04.10-desktop-arm64.iso">x</a>
      <a href="ubuntu-26.04.10-desktop-amd64.iso">x</a>
      <a href="../">up</a>"#;
    let names = links(html);
    let pattern = Regex::new(r"^ubuntu-.*-desktop-arm64\.iso$").unwrap();
    assert_eq!(
      newest(names.iter(), &pattern).as_deref(),
      Some("ubuntu-26.04.10-desktop-arm64.iso")
    );
  }

  #[test]
  fn arch_suffixes_dont_outrank_point_releases() {
    let names = [
      "ubuntu-26.04-desktop-arm64.iso".to_string(),
      "ubuntu-26.04.1-desktop-arm64.iso".to_string(),
    ];
    let pattern = Regex::new(r"^ubuntu-26\.04(\.\d+)*-desktop-arm64\.iso$").unwrap();
    assert_eq!(
      newest(names.iter(), &pattern).as_deref(),
      Some("ubuntu-26.04.1-desktop-arm64.iso")
    );
  }

  #[test]
  fn numbered_directories() {
    let html = r#"<a href="9/">9/</a><a href="44/">44/</a><a href="test/">t</a>"#;
    let names = links(html);
    let numbered = Regex::new(r"^\d+(\.\d+)*$").unwrap();
    assert_eq!(newest(names.iter(), &numbered).as_deref(), Some("44"));
  }
}
