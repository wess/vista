use regex::Regex;
use std::process::Command;

use super::download::Link;
use crate::{Error, Result};

const CATALOG: &str = "https://go.microsoft.com/fwlink/?LinkId=2156292";

fn value(xml: &str, name: &str) -> Option<String> {
  Regex::new(&format!(r"(?s)<{name}>(.*?)</{name}>"))
    .ok()?
    .captures(xml)
    .map(|capture| capture[1].trim().replace("&amp;", "&"))
}

pub(super) fn parse(xml: &str) -> Result<Link> {
  let files = Regex::new(r#"(?s)<File\s[^>]*>(.*?)</File>"#).expect("catalog files");
  files
    .captures_iter(xml)
    .filter_map(|capture| {
      let file = &capture[1];
      if value(file, "Architecture")?.to_uppercase() != "ARM64"
        || !value(file, "LanguageCode")?.eq_ignore_ascii_case("en-us")
        || value(file, "Edition")? != "Professional"
      {
        return None;
      }
      let mut url = reqwest::Url::parse(&value(file, "FilePath")?).ok()?;
      let host = url.host_str()?;
      if !(host == "microsoft.com" || host.ends_with(".microsoft.com")) {
        return None;
      }
      if !matches!(url.scheme(), "http" | "https") {
        return None;
      }
      // Microsoft's catalog still emits HTTP URLs for its own delivery host.
      // The same path is available over TLS; never send the installer in clear.
      url.set_scheme("https").ok()?;
      let filename = value(file, "FileName")?;
      if filename.contains('/') || filename.contains('\\') || !filename.ends_with(".esd") {
        return None;
      }
      let sha1 = value(file, "Sha1")?.to_lowercase();
      if sha1.len() != 40 || !sha1.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
      }
      Some(Link {
        url: url.into(),
        version: filename.split('.').take(2).collect::<Vec<_>>().join("."),
        filename,
        sha256: None,
        sha1: Some(sha1),
        size: value(file, "Size").and_then(|size| size.parse().ok()),
      })
    })
    .max_by_key(|link| {
      link
        .version
        .split('.')
        .filter_map(|part| part.parse::<u64>().ok())
        .collect::<Vec<_>>()
    })
    .ok_or_else(|| {
      Error::msg("Microsoft’s catalog has no English Windows 11 Pro installer for Arm64")
    })
}

pub async fn link(client: &reqwest::Client, status: &mut (dyn FnMut(&str) + Send)) -> Result<Link> {
  status("Reading Microsoft’s Windows installer catalog.");
  let response = client.get(CATALOG).send().await?.error_for_status()?;
  let data = response.bytes().await?;
  if data.len() > 16 * 1024 * 1024 {
    return Err(Error::msg(
      "Microsoft’s installer catalog is unexpectedly large",
    ));
  }
  status("Finding Windows 11 Pro for Arm64 in Microsoft’s catalog.");
  let xml = tokio::task::spawn_blocking(move || -> Result<String> {
    let path = std::env::temp_dir().join(format!("vistacatalog{}", uuid::Uuid::new_v4().simple()));
    std::fs::write(&path, &data)?;
    let output = Command::new("/usr/bin/tar")
      .args(["-xOf"])
      .arg(&path)
      .arg("products.xml")
      .output();
    let _ = std::fs::remove_file(&path);
    let output = output?;
    if !output.status.success() {
      return Err(Error::msg("Could not read Microsoft’s installer catalog"));
    }
    String::from_utf8(output.stdout).map_err(|error| Error::msg(error.to_string()))
  })
  .await
  .map_err(|error| Error::msg(error.to_string()))??;
  let link = parse(&xml)?;
  status(&format!(
    "Found Windows 11 Pro for Arm64, build {}.",
    link.version
  ));
  Ok(link)
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn selects_arm64_english_pro_and_requires_microsoft_source_and_checksum() {
    let file = |arch, lang, host| {
      format!("<File id=\"\"><Architecture>{arch}</Architecture><LanguageCode>{lang}</LanguageCode><Edition>Professional</Edition><FileName>26100.4349.a.esd</FileName><FilePath>http://{host}/a.esd</FilePath><Sha1>{}</Sha1><Size>1234</Size></File>", "a".repeat(40))
    };
    assert!(parse(&file("x64", "en-us", "dl.delivery.mp.microsoft.com")).is_err());
    assert!(parse(&file("ARM64", "fr-fr", "dl.delivery.mp.microsoft.com")).is_err());
    assert!(parse(&file("ARM64", "en-us", "example.com")).is_err());
    let missing_hash =
      file("ARM64", "en-us", "dl.delivery.mp.microsoft.com").replace(&"a".repeat(40), "");
    assert!(parse(&missing_hash).is_err());
    let found = parse(&file("ARM64", "en-us", "dl.delivery.mp.microsoft.com")).unwrap();
    assert_eq!(found.version, "26100.4349");
    assert_eq!(found.size, Some(1234));
    assert!(found
      .url
      .starts_with("https://dl.delivery.mp.microsoft.com/"));
  }
}
