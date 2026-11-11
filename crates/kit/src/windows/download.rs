//! Microsoft's software download service, the same one the Windows 11 for Arm
//! download page uses. The page's own script does a fraud-check handshake
//! (`ov-df.microsoft.com`) before it may ask for a link; Vista does the same
//! three requests, on one fresh session, and gets the official ISO link.
//! Links last 24 hours.

use regex::Regex;
use serde::Deserialize;

use crate::{Error, Result};

const PAGE: &str = "https://www.microsoft.com/en-us/software-download/windows11arm64";
const API: &str = "https://www.microsoft.com/software-download-connector/api";
const PROFILE: &str = "606624d44113";
const INSTANCE: &str = "560dc9f3-1aa5-4a2f-b63c-9e18f8d0e175";
/// The service only answers browsers.
const AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15";

#[derive(Clone, Debug, PartialEq)]
pub struct Link {
  pub url: String,
  pub filename: String,
  pub version: String,
  pub sha256: Option<String>,
  pub size: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Skus {
  #[serde(default)]
  skus: Vec<Sku>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Sku {
  id: String,
  language: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Links {
  #[serde(default)]
  product_download_options: Vec<Option_>,
  #[serde(default)]
  errors: Vec<Problem>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Option_ {
  name: String,
  uri: String,
  download_type: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Problem {
  key: String,
  value: String,
}

fn capture(pattern: &str, text: &str) -> Option<String> {
  Regex::new(pattern)
    .ok()?
    .captures(text)
    .map(|c| c[1].to_string())
}

fn browser() -> Result<reqwest::Client> {
  Ok(
    reqwest::Client::builder()
      .user_agent(AGENT)
      .cookie_store(true)
      .connect_timeout(std::time::Duration::from_secs(20))
      .build()?,
  )
}

/// Ask Microsoft for a fresh Windows 11 for Arm link (English, US).
pub async fn link(_client: &reqwest::Client) -> Result<Link> {
  let http = browser()?;
  let page = http.get(PAGE).send().await?.error_for_status()?.text().await?;
  let edition = capture(r#"<option value="([0-9]+)">Windows 11"#, &page)
    .ok_or_else(|| Error::msg("Microsoft's download page has changed; choose a downloaded ISO instead."))?;
  let sha256 = capture(r"(?s)English 64-bit.{0,300}?([A-Fa-f0-9]{64})", &page).map(|h| h.to_lowercase());

  let mut last = String::new();
  for attempt in 0..3 {
    if attempt > 0 {
      tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    match session(&http, &edition).await {
      Ok((url, name)) => {
        let filename = url
          .split('?')
          .next()
          .and_then(|u| u.rsplit('/').next())
          .unwrap_or("Windows11_arm64.iso")
          .to_string();
        let version = capture(r"(\d\dH\d)", &name).unwrap_or_default();
        let size = http
          .head(&url)
          .send()
          .await
          .ok()
          .and_then(|r| {
            r.headers()
              .get(reqwest::header::CONTENT_LENGTH)
              .and_then(|v| v.to_str().ok())
              .and_then(|v| v.parse().ok())
          });
        return Ok(Link {
          url,
          filename,
          version,
          sha256,
          size,
        });
      }
      Err(err) => last = err.to_string(),
    }
  }
  Err(Error::msg(format!(
    "Microsoft wouldn't hand out a download link ({last}). VPNs and some networks are blocked; \
     download the ISO from {PAGE} and choose it instead."
  )))
}

async fn session(http: &reqwest::Client, edition: &str) -> Result<(String, String)> {
  let sid = uuid::Uuid::new_v4().to_string();

  let mdt = http
    .get(format!(
      "https://ov-df.microsoft.com/mdt.js?instanceId={INSTANCE}&PageId=si&session_id={sid}"
    ))
    .send()
    .await?
    .text()
    .await?;
  let w = capture(r"[?&]w=([A-F0-9]+)", &mdt).ok_or_else(|| Error::msg("handshake script changed"))?;
  let rticks = capture(r#"rticks="?\+?([0-9]+)"#, &mdt).unwrap_or_default();
  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_millis())
    .unwrap_or(0);
  http
    .get(format!(
      "https://ov-df.microsoft.com/?session_id={sid}&CustomerId={INSTANCE}&PageId=si&w={w}&mdt={now}&rticks={rticks}"
    ))
    .send()
    .await?;

  let skus: Skus = http
    .get(format!(
      "{API}/getskuinformationbyproductedition?profile={PROFILE}&ProductEditionId={edition}&SKU=undefined&friendlyFileName=undefined&Locale=en-US&sessionID={sid}"
    ))
    .send()
    .await?
    .json()
    .await?;
  let sku = skus
    .skus
    .into_iter()
    .find(|s| s.language == "English")
    .ok_or_else(|| Error::msg("no English (US) edition listed"))?;

  let links: Links = http
    .get(format!(
      "{API}/GetProductDownloadLinksBySku?profile={PROFILE}&ProductEditionId=undefined&SKU={}&friendlyFileName=undefined&Locale=en-US&sessionID={sid}",
      sku.id
    ))
    .header(reqwest::header::REFERER, PAGE)
    .send()
    .await?
    .json()
    .await?;
  if let Some(problem) = links.errors.first() {
    return Err(Error::msg(format!("{}: {}", problem.key, problem.value)));
  }
  links
    .product_download_options
    .into_iter()
    .find(|o| o.download_type == 2)
    .map(|o| (o.uri, o.name))
    .ok_or_else(|| Error::msg("no Arm64 download offered"))
}
