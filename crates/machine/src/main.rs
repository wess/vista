//! `vistamachine` — runs one virtual machine.
//!
//! Vista starts one of these per machine (`vistamachine <bundle> [--hidden]`).
//! It owns the machine, its window, and a control socket that Vista, the
//! menu bar and the MCP server talk to. When the machine stops, the process
//! exits: a crash here takes down one machine and nothing else.

mod capture;
mod control;
mod delegate;
mod guest;
mod input;
mod lifecycle;
mod menu;
mod state;
mod window;

use std::path::PathBuf;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

fn main() {
  let mut path: Option<PathBuf> = None;
  let mut hidden = false;
  for arg in std::env::args().skip(1) {
    match arg.as_str() {
      "--hidden" => hidden = true,
      other => path = Some(PathBuf::from(other)),
    }
  }
  let Some(path) = path else {
    eprintln!("usage: vistamachine <machine.vista> [--hidden]");
    std::process::exit(2);
  };
  let bundle = match vista_kit::bundle::load(&path) {
    Ok(bundle) => bundle,
    Err(err) => {
      eprintln!("vistamachine: {err}");
      std::process::exit(1);
    }
  };
  if !vista_vz::supported() {
    eprintln!("vistamachine: this Mac doesn't support virtualization");
    std::process::exit(1);
  }

  let mtm = MainThreadMarker::new().expect("vistamachine runs on the main thread");
  let app = NSApplication::sharedApplication(mtm);
  app.setActivationPolicy(if hidden {
    NSApplicationActivationPolicy::Accessory
  } else {
    NSApplicationActivationPolicy::Regular
  });

  let id = bundle.config.id.clone();
  eprintln!(
    "vistamachine: {} ({}) starting, pid {}",
    bundle.config.name,
    id,
    std::process::id()
  );
  control::listen(id);
  state::install(bundle, mtm);
  menu::install(&app, mtm);
  lifecycle::boot(!hidden);
  app.run();
}
