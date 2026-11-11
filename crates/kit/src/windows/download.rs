//! Official Windows 11 Arm media from Microsoft’s catalog, with the ISO service as a fallback.

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
  pub sha1: Option<String>,
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
      .timeout(std::time::Duration::from_secs(30))
      .build()?,
  )
}

/// Ask Microsoft for a fresh Windows 11 for Arm link (English, US).
pub async fn link(client: &reqwest::Client) -> Result<Link> {
  link_with_status(client, &mut |_| {}).await
}

pub async fn link_with_status(
  client: &reqwest::Client,
  status: &mut (dyn FnMut(&str) + Send),
) -> Result<Link> {
  tokio::time::timeout(std::time::Duration::from_secs(85), async {
    match super::catalog::link(client, status).await {
      Ok(link) => Ok(link),
      Err(error) => {
        status(&format!("Catalog lookup failed ({error}). Trying Microsoft’s ISO service."));
        discover(status).await
      }
    }
  }).await
    .map_err(|_| Error::msg(format!("Microsoft did not provide a download link within 85 seconds. Download an Arm64 ISO from {PAGE} and choose it instead")))?
}

async fn discover(status: &mut (dyn FnMut(&str) + Send)) -> Result<Link> {
  let http = browser()?;
  status("Opening Microsoft’s Windows 11 Arm download page.");
  let page = http
    .get(PAGE)
    .send()
    .await?
    .error_for_status()?
    .text()
    .await?;
  let edition = capture(r#"<option value="([0-9]+)">Windows 11"#, &page).ok_or_else(|| {
    Error::msg("Microsoft's download page has changed; choose a downloaded ISO instead.")
  })?;
  let sha256 =
    capture(r"(?s)English 64-bit.{0,300}?([A-Fa-f0-9]{64})", &page).map(|h| h.to_lowercase());

  let mut last = String::new();
  for attempt in 0..3 {
    if attempt > 0 {
      status(&format!(
        "Retrying Microsoft’s download service (attempt {} of 3).",
        attempt + 1
      ));
      tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    match session(&http, &edition, status).await {
      Ok((url, name)) => {
        let filename = url
          .split('?')
          .next()
          .and_then(|u| u.rsplit('/').next())
          .unwrap_or("Windows11_arm64.iso")
          .to_string();
        let version = capture(r"(\d\dH\d)", &name).unwrap_or_default();
        status("Checking the Windows installer’s download size.");
        let size = http.head(&url).send().await.ok().and_then(|r| {
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
          sha1: None,
          size,
        });
      }
      Err(err) => last = err.to_string(),
    }
  }
  Err(Error::msg(format!(
    "Microsoft wouldn't hand out a download link ({last}). Try again or \
     download the ISO from {PAGE} and choose it instead."
  )))
}

async fn session(
  http: &reqwest::Client,
  edition: &str,
  status: &mut (dyn FnMut(&str) + Send),
) -> Result<(String, String)> {
  let sid = uuid::Uuid::new_v4().to_string();

  status("Starting Microsoft’s download-session verification.");
  let mdt = http
    .get(format!(
      "https://ov-df.microsoft.com/mdt.js?instanceId={INSTANCE}&PageId=si&session_id={sid}"
    ))
    .send()
    .await?
    .text()
    .await?;
  let w =
    capture(r"[?&]w=([A-F0-9]+)", &mdt).ok_or_else(|| Error::msg("handshake script changed"))?;
  let rticks = capture(r#"rticks="?\+?([0-9]+)"#, &mdt).unwrap_or_default();
  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_millis())
    .unwrap_or(0);
  status("Completing Microsoft’s download-session verification.");
  http
    .get(format!(
      "https://ov-df.microsoft.com/?session_id={sid}&CustomerId={INSTANCE}&PageId=si&w={w}&mdt={now}&rticks={rticks}"
    ))
    .send()
    .await?;

  status("Requesting available Windows languages.");
  let skus: Skus = http
    .get(format!(
      "{API}/getskuinformationbyproductedition?profile={PROFILE}&ProductEditionId={edition}&SKU=undefined&friendlyFileName=undefined&Locale=en-US&sessionID={sid}"
    ))
    .send()
    .await?
    .error_for_status()?
    .json()
    .await?;
  let sku = skus
    .skus
    .into_iter()
    .find(|s| s.language == "English")
    .ok_or_else(|| Error::msg("no English (US) edition listed"))?;

  status("Requesting the Windows 11 Arm installer link.");
  let links: Links = http
    .get(format!(
      "{API}/GetProductDownloadLinksBySku?profile={PROFILE}&ProductEditionId=undefined&SKU={}&friendlyFileName=undefined&Locale=en-US&sessionID={sid}",
      sku.id
    ))
    .header(reqwest::header::REFERER, PAGE)
    .send()
    .await?
    .error_for_status()?
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
