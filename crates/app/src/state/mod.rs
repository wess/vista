//! The library: every machine, every creation job, the settings, and what's
//! selected. One entity, shared by the main window and the menu bar panel,
//! refreshed once a second from the database and the machines' sockets.

mod commands;
mod machine;

pub use machine::Machine;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use gpui::{App, AppContext, Context, Entity, EventEmitter, Global, Image, ImageFormat};
use vista_kit::jobs::Job;
use vista_kit::store::{Settings, Store};

use crate::runtime;

/// What the content area shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
  Machine(String),
  Create,
  Settings,
  Welcome,
}

/// Something to tell the person, as a toast.
#[derive(Clone, Debug)]
pub enum Notice {
  Info(String),
  Failed(String),
}

pub struct Library {
  pub store: Option<Store>,
  pub machines: Vec<Machine>,
  pub jobs: Vec<Job>,
  pub settings: Settings,
  pub page: Page,
  /// Latest screen of each running, visible machine.
  pub screens: HashMap<String, Arc<Image>>,
  pub loaded: bool,
}

impl EventEmitter<Notice> for Library {}

/// The app-wide handle to the library.
pub struct Shared(pub Entity<Library>);
impl Global for Shared {}

pub fn library(cx: &App) -> Entity<Library> {
  cx.global::<Shared>().0.clone()
}

impl Library {
  pub fn install(cx: &mut App) -> Entity<Library> {
    let store = runtime::block_on(Store::open()).ok();
    let settings = match &store {
      Some(store) => {
        let _ = runtime::block_on(store.adopt(std::path::Path::new(
          &runtime::block_on(store.settings()).unwrap_or_default().location,
        )));
        runtime::block_on(store.settings()).unwrap_or_default()
      }
      None => Settings::default(),
    };
    // The first read happens before any window opens, so the first frame
    // shows the real library rather than an empty one.
    let machines = match &store {
      Some(store) => {
        let store = store.clone();
        runtime::block_on(async move { machine::gather(&store).await })
      }
      None => Vec::new(),
    };
    let page = match std::env::var("VISTA_PAGE").as_deref() {
      Ok("create") => Page::Create,
      Ok("settings") => Page::Settings,
      Ok("welcome") => Page::Welcome,
      _ => machines
        .first()
        .map(|m| Page::Machine(m.id().to_string()))
        .unwrap_or(Page::Welcome),
    };
    let entity = cx.new(|cx| {
      let library = Library {
        store,
        machines,
        jobs: vista_kit::jobs::list(),
        settings,
        page,
        screens: HashMap::new(),
        loaded: true,
      };
      Self::poll(cx);
      Self::watch_screens(cx);
      library
    });
    cx.set_global(Shared(entity.clone()));
    entity
  }

  fn poll(cx: &mut Context<Self>) {
    cx.spawn(async move |this, cx| loop {
      let Ok(store) = this.read_with(cx, |lib, _| lib.store.clone()) else {
        break;
      };
      if let Some(store) = store {
        let gathered = runtime::spawn(async move {
          let machines = machine::gather(&store).await;
          let jobs = vista_kit::jobs::list();
          (machines, jobs)
        })
        .await;
        if let Ok((machines, jobs)) = gathered {
          if this.update(cx, |lib, cx| lib.apply(machines, jobs, cx)).is_err() {
            break;
          }
        }
      }
      cx.background_executor().timer(Duration::from_millis(1000)).await;
    })
    .detach();
  }

  fn apply(&mut self, machines: Vec<Machine>, jobs: Vec<Job>, cx: &mut Context<Self>) {
    let first = !self.loaded;
    self.loaded = true;
    // A job that just finished brings its machine into view.
    for job in &jobs {
      let before = self.jobs.iter().find(|j| j.id == job.id);
      if let (Some(before), Some(id)) = (before, &job.machine) {
        if !before.phase.is_finished() && job.phase == vista_kit::jobs::Phase::Done {
          self.page = Page::Machine(id.clone());
        }
        if !before.phase.is_finished() && job.phase == vista_kit::jobs::Phase::Failed {
          cx.emit(Notice::Failed(job.message.clone().unwrap_or_else(|| "Creating the machine failed.".into())));
        }
      }
    }
    self.machines = machines;
    self.jobs = jobs
      .into_iter()
      .filter(|j| !(j.phase == vista_kit::jobs::Phase::Done && crate::state::age(j) > 10))
      .collect();
    let exists = |id: &str| self.machines.iter().any(|m| m.id() == id);
    match &self.page {
      Page::Machine(id) if !exists(id) => self.page = self.default_page(),
      Page::Welcome if first || !self.machines.is_empty() => self.page = self.default_page(),
      _ => {}
    }
    self.screens.retain(|id, _| self.machines.iter().any(|m| m.id() == id && m.is_running()));
    cx.notify();
  }

  fn default_page(&self) -> Page {
    self
      .machines
      .first()
      .map(|m| Page::Machine(m.id().to_string()))
      .unwrap_or(Page::Welcome)
  }

  /// Keep a live thumbnail of the selected machine's screen.
  fn watch_screens(cx: &mut Context<Self>) {
    cx.spawn(async move |this, cx| loop {
      let target = this
        .read_with(cx, |lib, _| match &lib.page {
          Page::Machine(id) => lib
            .machine(id)
            .filter(|m| m.live.as_ref().map(|l| l.visible && l.state == vista_kit::machine::State::Running).unwrap_or(false))
            .map(|m| m.id().to_string()),
          _ => None,
        })
        .ok()
        .flatten();
      if let Some(id) = target {
        let wanted = id.clone();
        let shot = runtime::spawn(async move {
          match vista_kit::ipc::call(&wanted, &vista_kit::ipc::Request::Screenshot).await {
            Ok(vista_kit::ipc::Response::Image { png, .. }) => {
              use base64::Engine;
              base64::engine::general_purpose::STANDARD.decode(png).ok()
            }
            _ => None,
          }
        })
        .await
        .ok()
        .flatten();
        if let Some(bytes) = shot {
          let image = Arc::new(Image::from_bytes(ImageFormat::Png, bytes));
          if this
            .update(cx, |lib, cx| {
              lib.screens.insert(id, image);
              cx.notify();
            })
            .is_err()
          {
            break;
          }
        }
      }
      cx.background_executor().timer(Duration::from_secs(3)).await;
    })
    .detach();
  }

  pub fn machine(&self, id: &str) -> Option<&Machine> {
    self.machines.iter().find(|m| m.id() == id)
  }

  pub fn selected(&self) -> Option<&Machine> {
    match &self.page {
      Page::Machine(id) => self.machine(id),
      _ => None,
    }
  }

  pub fn select(&mut self, page: Page, cx: &mut Context<Self>) {
    self.page = page;
    cx.notify();
  }
}

/// Seconds since a job last changed.
pub fn age(job: &Job) -> i64 {
  vista_kit::machine::now() - job.updated
}
