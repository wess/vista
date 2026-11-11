use std::path::PathBuf;
use std::time::Duration;

use super::{build, save, Job, Phase};
use crate::download::{fetch, Phase as Fetch, Transfer};
use crate::machine::Os;
use crate::store::{Image, Store};
use crate::{Error, Result};

fn cancel_marker(id: &str) -> PathBuf {
  crate::paths::jobs().join(format!("{}.cancel", crate::paths::short(id)))
}

/// Ask a running job to stop. It notices within a second.
pub fn cancel(id: &str) {
  let _ = std::fs::write(cancel_marker(id), b"");
}

fn update(job: &mut Job, phase: Phase, message: Option<String>) {
  job.phase = phase;
  if message.is_some() {
    job.message = message;
  }
  let _ = save(job);
}

/// Work a job to the end. The job file is updated at every step; the return
/// value is the finished job.
pub async fn run(store: &Store, mut job: Job) -> Job {
  job.pid = std::process::id();
  match work(store, &mut job).await {
    Ok(()) => {
      if let Some(total) = job.total {
        job.received = total;
      }
      update(&mut job, Phase::Done, Some("Ready".into()))
    }
    Err(err) => {
      let cancelled = cancel_marker(&job.id).exists();
      update(
        &mut job,
        if cancelled { Phase::Cancelled } else { Phase::Failed },
        Some(err.to_string()),
      );
    }
  }
  let _ = std::fs::remove_file(cancel_marker(&job.id));
  job
}

async fn work(store: &Store, job: &mut Job) -> Result<()> {
  let client = crate::download::client();
  let settings = store.settings().await?;
  let spec = job.spec.clone();

  // 1. The installer: a local file, a kept download, or a fresh one.
  let (entry, release, installer) = if let Some(file) = &spec.file {
    let path = PathBuf::from(file);
    if !path.exists() {
      return Err(Error::msg(format!("{file} doesn't exist")));
    }
    (None, String::new(), path)
  } else {
    let id = spec
      .system
      .as_deref()
      .ok_or_else(|| Error::msg("Say which system to install, or give an installer file."))?;
    let entry = crate::catalog::find(id)
      .ok_or_else(|| Error::msg(format!("Vista doesn't know a system called \"{id}\". list_systems shows the ones it does.")))?;
    update(job, Phase::Resolving, Some(format!("Finding {}", entry.title())));
    let resolved = crate::catalog::resolve(&client, &entry).await?;
    let kept = store
      .images()
      .await?
      .into_iter()
      .find(|image| image.url.split('?').next() == resolved.url.split('?').next() || (image.catalog == entry.id && image.version == resolved.version && entry.os == Os::Windows));
    let path = match kept {
      Some(image) => PathBuf::from(image.path),
      None => {
        let dest = crate::paths::images().join(&resolved.filename);
        job.total = resolved.size;
        update(job, Phase::Downloading, Some(format!("Downloading {}", resolved.filename)));
        let transfer = Transfer::new();
        let watcher = watch(job.clone(), transfer.clone());
        let fetched = fetch(&client, &resolved.url, &dest, resolved.sha256.as_deref(), &transfer).await;
        watcher.abort();
        let path = fetched?;
        job.received = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        job.total = Some(job.received);
        if settings.keepinstallers {
          store
            .keep_image(&Image {
              id: uuid::Uuid::new_v4().to_string(),
              catalog: entry.id.to_string(),
              name: entry.title(),
              version: resolved.version.clone(),
              url: resolved.url.clone(),
              path: path.to_string_lossy().to_string(),
              size: std::fs::metadata(&path).map(|m| m.len() as i64).unwrap_or(0),
              sha256: resolved.sha256.clone(),
              fetched: crate::machine::now(),
            })
            .await?;
        }
        path
      }
    };
    (Some(entry), resolved.version, path)
  };

  // 2. The bundle.
  update(job, Phase::Creating, Some("Creating the machine".into()));
  let mut config = build::configure(&spec, entry.as_ref(), &release, &installer);
  let location = spec
    .location
    .clone()
    .unwrap_or_else(|| settings.location.clone());
  let mut bundle = crate::bundle::create(std::path::Path::new(&location), &config)?;
  job.machine = Some(config.id.clone());
  let _ = save(job);

  if config.os == Os::Windows {
    update(job, Phase::Preparing, Some("Preparing Windows Setup".into()));
    let account = spec.account.clone().unwrap_or_else(|| crate::machine::Account {
      fullname: "Vista".into(),
      username: "vista".into(),
      password: "vista".into(),
      autologin: true,
      ssh: false,
    });
    let media = crate::windows::media::prepare(&client, &installer, &bundle.path, &config.name, &account).await?;
    config.installer = Some(crate::machine::Installer::Iso {
      path: media.to_string_lossy().to_string(),
      extras: Vec::new(),
    });
    crate::bundle::save(&bundle.path, &config)?;
    bundle.config = config.clone();
  }
  store.register(&bundle).await?;

  // 3. Power on.
  if spec.start {
    update(job, Phase::Starting, Some("Starting".into()));
    crate::launch::start(&bundle.path, spec.visible)?;
    store.touch(&config.id).await?;
  }
  Ok(())
}

/// Mirror download progress into the job file, and turn a cancel marker
/// into a cancelled transfer.
fn watch(mut job: Job, transfer: Transfer) -> tokio::task::JoinHandle<()> {
  tokio::spawn(async move {
    let mut progress = transfer.progress.clone();
    loop {
      tokio::select! {
        changed = progress.changed() => {
          if changed.is_err() { break; }
        }
        _ = tokio::time::sleep(Duration::from_secs(1)) => {}
      }
      if cancel_marker(&job.id).exists() {
        transfer.cancel();
      }
      let now = progress.borrow().clone();
      job.received = now.received;
      job.total = now.total.or(job.total);
      job.phase = match now.phase {
        Fetch::Verifying => Phase::Verifying,
        _ => Phase::Downloading,
      };
      let _ = save(&job);
    }
  })
}
