//! Things the person asks for. Each runs on tokio and reports back with a
//! toast; the next poll shows the new state.

use std::future::Future;
use std::path::PathBuf;

use gpui::Context;
use vista_kit::bundle::{self, Snapshots};
use vista_kit::ipc::{Request, Response};
use vista_kit::jobs::{Job, Spec};
use vista_kit::machine::Config;
use vista_kit::store::Settings;

use super::{Library, Notice, Page};
use crate::runtime;

impl Library {
  /// Run `work` on tokio; toast its error, or its success message if any.
  fn task<F>(&mut self, work: F, cx: &mut Context<Self>)
  where
    F: Future<Output = Result<Option<String>, String>> + Send + 'static,
  {
    let handle = runtime::spawn(work);
    cx.spawn(async move |this, cx| {
      let outcome = handle.await.unwrap_or_else(|e| Err(e.to_string()));
      let _ = this.update(cx, |_, cx| match outcome {
        Ok(Some(message)) => cx.emit(Notice::Info(message)),
        Ok(None) => {}
        Err(message) => cx.emit(Notice::Failed(message)),
      });
    })
    .detach();
  }

  fn path_of(&self, id: &str) -> Option<PathBuf> {
    self
      .machine(id)
      .and_then(|m| m.bundle.as_ref())
      .map(|b| b.path.clone())
  }

  pub fn start(&mut self, id: &str, visible: bool, cx: &mut Context<Self>) {
    let Some(path) = self.path_of(id) else { return };
    let id = id.to_string();
    let store = self.store.clone();
    self.task(
      async move {
        if let Some(live) = vista_kit::ipc::status(&id).await {
          if live.state == vista_kit::machine::State::Error {
            vista_kit::ipc::call(&id, &Request::Kill)
              .await
              .map_err(|error| error.to_string())?;
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while vista_kit::ipc::status(&id).await.is_some()
              && std::time::Instant::now() < deadline
            {
              tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            if vista_kit::ipc::status(&id).await.is_some() {
              return Err(
                "The previous failed machine has not exited. Close it and try again.".into(),
              );
            }
          } else {
            if visible && !live.visible {
              let _ = vista_kit::ipc::call(&id, &Request::Show).await;
            }
            return Ok(None);
          }
        }
        vista_kit::launch::start(&path, visible).map_err(|e| e.to_string())?;
        if let Some(store) = store {
          let _ = store.touch(&id).await;
        }
        match vista_kit::launch::wait_ready(&id, std::time::Duration::from_secs(30)).await {
          Some(live) if live.state == vista_kit::machine::State::Error => Err(
            live
              .message
              .unwrap_or_else(|| "The machine failed to start.".into()),
          ),
          Some(_) => Ok(None),
          None => Err(format!(
            "The machine didn't start. See {}/machine.log.",
            path.display()
          )),
        }
      },
      cx,
    );
  }

  /// Send a control request to a running machine.
  pub fn control(&mut self, id: &str, request: Request, cx: &mut Context<Self>) {
    let id = id.to_string();
    self.task(
      async move {
        match vista_kit::ipc::call(&id, &request)
          .await
          .map_err(|e| e.to_string())?
        {
          Response::Error { message } => Err(message),
          Response::Snapshot(snapshot) => {
            Ok(Some(format!("Took \u{201c}{}\u{201d}.", snapshot.name)))
          }
          _ => Ok(None),
        }
      },
      cx,
    );
  }

  pub fn snapshot(&mut self, id: &str, name: String, cx: &mut Context<Self>) {
    let running = self.machine(id).map(|m| m.is_running()).unwrap_or(false);
    if running {
      return self.control(id, Request::Snapshot { name }, cx);
    }
    let Some(bundle) = self.machine(id).and_then(|m| m.bundle.clone()) else {
      return;
    };
    self.task(
      async move {
        let snapshot = Snapshots::take(&bundle, &name).map_err(|e| e.to_string())?;
        Ok(Some(format!("Took \u{201c}{}\u{201d}.", snapshot.name)))
      },
      cx,
    );
  }

  pub fn revert(&mut self, id: &str, snapshot: String, cx: &mut Context<Self>) {
    let Some(bundle) = self.machine(id).and_then(|m| m.bundle.clone()) else {
      return;
    };
    let id = id.to_string();
    self.task(
      async move {
        if vista_kit::ipc::status(&id).await.is_some() {
          let _ = vista_kit::ipc::call(&id, &Request::Kill).await;
          for _ in 0..40 {
            if vista_kit::ipc::status(&id).await.is_none() {
              break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
          }
        }
        Snapshots::revert(&bundle, &snapshot).map_err(|e| e.to_string())?;
        Ok(Some("Reverted.".into()))
      },
      cx,
    );
  }

  pub fn delete_snapshot(&mut self, id: &str, snapshot: String, cx: &mut Context<Self>) {
    let Some(bundle) = self.machine(id).and_then(|m| m.bundle.clone()) else {
      return;
    };
    self.task(
      async move {
        Snapshots::delete(&bundle, &snapshot).map_err(|e| e.to_string())?;
        Ok(None)
      },
      cx,
    );
  }

  pub fn save_config(&mut self, config: Config, cx: &mut Context<Self>) {
    let Some(path) = self.path_of(&config.id) else {
      return;
    };
    let store = self.store.clone();
    let running = self
      .machine(&config.id)
      .map(|m| m.is_running())
      .unwrap_or(false);
    self.task(
      async move {
        bundle::save(&path, &config).map_err(|e| e.to_string())?;
        if let Some(store) = store {
          let bundle = bundle::load(&path).map_err(|e| e.to_string())?;
          store.register(&bundle).await.map_err(|e| e.to_string())?;
        }
        Ok(Some(if running {
          "Saved. Changes apply the next time the machine starts.".into()
        } else {
          "Saved.".into()
        }))
      },
      cx,
    );
  }

  pub fn duplicate(&mut self, id: &str, cx: &mut Context<Self>) {
    let Some(bundle) = self.machine(id).and_then(|m| m.bundle.clone()) else {
      return;
    };
    let store = self.store.clone();
    self.task(
      async move {
        let name = format!("{} copy", bundle.config.name);
        let copy = bundle::duplicate(&bundle, &name).map_err(|e| e.to_string())?;
        if let Some(store) = store {
          store.register(&copy).await.map_err(|e| e.to_string())?;
        }
        Ok(Some(format!("Cloned as {name}.")))
      },
      cx,
    );
  }

  pub fn delete(&mut self, id: &str, cx: &mut Context<Self>) {
    let machine = self.machine(id).cloned();
    let Some(machine) = machine else { return };
    let store = self.store.clone();
    self.task(
      async move {
        if machine.is_running() {
          return Err("Shut the machine down before deleting it.".into());
        }
        if let Some(bundle) = &machine.bundle {
          bundle::delete(bundle).map_err(|e| e.to_string())?;
        }
        if let Some(store) = store {
          store
            .forget(machine.id())
            .await
            .map_err(|e| e.to_string())?;
        }
        Ok(Some(if machine.bundle.is_some() {
          format!("Moved {} to the Trash.", machine.name())
        } else {
          format!("Removed {} from the library.", machine.name())
        }))
      },
      cx,
    );
  }

  /// Start building a machine; the job shows in the sidebar.
  pub fn create(&mut self, spec: Spec, cx: &mut Context<Self>) {
    if self.store.is_none() {
      return cx.emit(Notice::Failed(
        "The library database isn't available.".into(),
      ));
    }
    let mut job = Job::new(spec);
    job.transition(
      vista_kit::jobs::Phase::Queued,
      "Waiting for the creation worker.".into(),
    );
    if let Err(err) = vista_kit::jobs::save(&job) {
      return cx.emit(Notice::Failed(err.to_string()));
    }
    self.page = Page::Job(job.id.clone());
    if let Err(error) = vista_kit::jobs::spawn(&job.id) {
      job.transition(vista_kit::jobs::Phase::Failed, error.to_string());
      let _ = vista_kit::jobs::save(&job);
    }
    self.jobs.insert(0, job);
    cx.notify();
  }

  pub fn cancel_job(&mut self, id: &str, cx: &mut Context<Self>) {
    vista_kit::jobs::cancel(id);
    cx.notify();
  }

  pub fn dismiss_job(&mut self, id: &str, cx: &mut Context<Self>) {
    vista_kit::jobs::remove(id);
    self.jobs.retain(|j| j.id != id);
    cx.notify();
  }

  pub fn save_settings(&mut self, settings: Settings, cx: &mut Context<Self>) {
    let changed_appearance = settings.appearance != self.settings.appearance;
    self.settings = settings.clone();
    if changed_appearance {
      crate::theme::apply(settings.appearance, cx);
    }
    crate::statusbar::set_visible(settings.menubar);
    let store = self.store.clone();
    self.task(
      async move {
        if let Some(store) = store {
          store
            .save_settings(&settings)
            .await
            .map_err(|e| e.to_string())?;
        }
        Ok(None)
      },
      cx,
    );
    cx.notify();
  }

  pub fn import(&mut self, path: PathBuf, cx: &mut Context<Self>) {
    let store = self.store.clone();
    self.task(
      async move {
        let bundle = bundle::load(&path).map_err(|e| e.to_string())?;
        if let Some(store) = store {
          store.register(&bundle).await.map_err(|e| e.to_string())?;
        }
        Ok(Some(format!("Added {}.", bundle.config.name)))
      },
      cx,
    );
  }
}
