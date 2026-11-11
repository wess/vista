//! macOS restore images.
//!
//! Apple publishes the current image in its software update catalog (mesu).
//! Older images are still on Apple's CDN; ipsw.me indexes them for the
//! VirtualMac2,1 model with sizes and SHA-256 sums. Either way the bytes come
//! from Apple.

use regex::Regex;
use serde::Deserialize;

use super::{Entry, Source};
use crate::machine::Os;
use crate::{Error, Result};

const MESU: &str =
  "https://mesu.apple.com/assets/macos/com_apple_macOSIPSW/com_apple_macOSIPSW.xml";
const IPSWME: &str = "https://api.ipsw.me/v4/device/VirtualMac2,1?type=ipsw";

const fn mac(id: &'static str, version: &'static str, major: Option<u32>) -> Entry {
  Entry {
    id,
    os: Os::Macos,
    family: "macos",
    name: "macOS",
    version,
    edition: "",
    source: Source::Macos { major },
    checksum: None,
    desktop: true,
    notes: "Restore image from Apple.",
  }
}

pub const ENTRIES: [Entry; 5] = [
  mac("macoslatest", "", None),
  mac("macos26", "26", Some(26)),
  mac("macos15", "15", Some(15)),
  mac("macos14", "14", Some(14)),
  mac("macos13", "13", Some(13)),
];

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct MacRelease {
  pub version: String,
  pub buildid: String,
  pub url: String,
  #[serde(default)]
  pub filesize: u64,
  #[serde(default)]
  pub sha256sum: Option<String>,
}

impl MacRelease {
  pub fn major(&self) -> u32 {
    self
      .version
      .split('.')
      .next()
      .and_then(|m| m.parse().ok())
      .unwrap_or(0)
  }
}

#[derive(Deserialize)]
struct Device {
  firmwares: Vec<MacRelease>,
}

/// Every restore image for virtual Macs, newest first.
pub async fn macos_versions(client: &reqwest::Client) -> Result<Vec<MacRelease>> {
  let device: Device = client
    .get(IPSWME)
    .header("Accept", "application/json")
    .send()
    .await?
    .error_for_status()?
    .json()
    .await?;
  let mut list = device.firmwares;
  list.sort_by(|a, b| {
    super::listing::version_key(&b.version).cmp(&super::listing::version_key(&a.version))
  });
  Ok(list)
}

/// The newest image straight from Apple's catalog: the fallback when the
/// index is unreachable.
async fn latest_from_apple(client: &reqwest::Client) -> Result<MacRelease> {
  let xml = super::listing::page(client, MESU).await?;
  let pattern =
    Regex::new(r"https://[^<\s]+/UniversalMac_([0-9.]+)_([0-9A-Za-z]+)_Restore\.ipsw").unwrap();
  pattern
    .captures_iter(&xml)
    .map(|c| MacRelease {
      version: c[1].to_string(),
      buildid: c[2].to_string(),
      url: c[0].to_string(),
      filesize: 0,
      sha256sum: None,
    })
    .max_by(|a, b| {
      super::listing::version_key(&a.version)
        .cmp(&super::listing::version_key(&b.version))
        // Among builds of one version the plain one has the shortest id.
        .then(b.buildid.len().cmp(&a.buildid.len()))
    })
    .ok_or_else(|| Error::msg("Apple's catalog lists no macOS restore image"))
}

/// The image for a major version, or the newest.
pub async fn release(client: &reqwest::Client, major: Option<u32>) -> Result<MacRelease> {
  match macos_versions(client).await {
    Ok(list) => list
      .into_iter()
      .find(|r| major.map(|m| r.major() == m).unwrap_or(true))
      .ok_or_else(|| Error::msg(format!("No restore image for macOS {}", major.unwrap_or(0)))),
    Err(err) if major.is_none() => latest_from_apple(client).await.map_err(|_| err),
    Err(err) => Err(err),
  }
}
