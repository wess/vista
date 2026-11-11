//! Resolves every catalog entry against its publisher. Network-bound, so it
//! only runs when asked: `cargo test -p vista-kit --test catalog -- --ignored`.

use vista_kit::catalog;

#[tokio::test]
#[ignore]
async fn every_entry_resolves() {
  let client = vista_kit::download::client();
  let mut failures = Vec::new();
  for entry in catalog::all() {
    if entry.id == "windows11" {
      continue;
    }
    match catalog::resolve(&client, &entry).await {
      Ok(found) => println!(
        "{:<18} {:<10} {:>6} MB sha256:{} {}",
        entry.id,
        found.version,
        found.size.unwrap_or(0) / 1_000_000,
        if found.sha256.is_some() { "yes" } else { "no " },
        found.filename
      ),
      Err(err) => failures.push(format!("{}: {err}", entry.id)),
    }
  }
  assert!(failures.is_empty(), "{failures:#?}");
}

#[tokio::test]
#[ignore]
async fn windows_link() {
  let client = vista_kit::download::client();
  let link = vista_kit::windows::download::link(&client).await.unwrap();
  println!("{} {} {:?} {:?}", link.filename, link.version, link.size, link.sha256);
  assert!(link.url.starts_with("https://"));
}
