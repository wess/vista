//! Making a machine: pick a system, set it up, create. Creation itself is a
//! job (download, verify, build) that shows in the sidebar.

mod configure;
mod pick;

use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, Window};
use guise::prelude::*;
use vista_kit::catalog::Entry;
use vista_kit::machine::Os;

use crate::state::{library, Library};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Step {
  Pick,
  Configure,
}

pub struct Create {
  pub library: Entity<Library>,
  pub step: Step,
  /// A family whose editions are showing.
  pub family: Option<&'static str>,
  pub entry: Option<Entry>,
  pub file: Option<PathBuf>,
  pub name: Entity<TextInput>,
  pub cpus: Entity<NumberInput>,
  pub memory: Entity<NumberInput>,
  pub disk: Entity<NumberInput>,
  pub username: Entity<TextInput>,
  pub password: Entity<TextInput>,
  pub share: bool,
  pub rosetta: bool,
  pub ssh: bool,
  pub start: bool,
}

impl Create {
  pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
    let library = library(cx);
    let share = library.read(cx).settings.sharedownloads;
    let host = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8) as f64;
    let user = std::env::var("USER").unwrap_or_default();
    Create {
      library,
      step: Step::Pick,
      family: None,
      entry: None,
      file: None,
      name: cx.new(|cx| TextInput::new(cx).label("Name").placeholder("My Machine")),
      cpus: cx.new(|cx| NumberInput::new(cx).label("Processors").min(1.0).max(host).step(1.0).value(4.0)),
      memory: cx.new(|cx| NumberInput::new(cx).label("Memory (GB)").min(1.0).max(1024.0).step(1.0).value(8.0)),
      disk: cx.new(|cx| NumberInput::new(cx).label("Disk (GB)").min(8.0).max(8192.0).step(8.0).value(64.0)),
      username: cx.new(|cx| TextInput::new(cx).label("Account name").value(&user)),
      password: cx.new(|cx| TextInput::new(cx).label("Password").password(true)),
      share,
      rosetta: false,
      ssh: false,
      start: true,
    }
  }

  pub fn os(&self) -> Os {
    match (&self.entry, &self.file) {
      (Some(entry), _) => entry.os,
      (None, Some(file)) => vista_kit::jobs::guess_os(file),
      _ => Os::Linux,
    }
  }

  /// Move to the form with defaults for what was picked.
  pub fn choose(&mut self, entry: Option<Entry>, file: Option<PathBuf>, cx: &mut Context<Self>) {
    self.entry = entry;
    self.file = file;
    let os = self.os();
    let (cpus, memory, disk) = os.defaults();
    let host = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8) as u32;
    let name = match (&self.entry, &self.file) {
      (Some(entry), _) => entry.title(),
      (None, Some(file)) => file
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Machine".into()),
      _ => "Machine".into(),
    };
    self.name.update(cx, |input, cx| input.set_text(&name, cx));
    self.cpus.update(cx, |input, cx| input.set_value(cpus.min(host.max(1)) as f64, cx));
    self.memory.update(cx, |input, cx| input.set_value((memory / 1024) as f64, cx));
    self.disk.update(cx, |input, cx| {
      input.set_min(os.minimum_disk() as f64, cx);
      input.set_value(disk as f64, cx);
    });
    self.rosetta = os == Os::Linux && crate::rosetta();
    self.step = Step::Configure;
    cx.notify();
  }
}

impl Render for Create {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let body = match self.step {
      Step::Pick => self.pick(window, cx).into_any_element(),
      Step::Configure => self.configure(window, cx).into_any_element(),
    };
    div()
      .id("create")
      .size_full()
      .overflow_y_scroll()
      .child(div().p(px(28.)).max_w(px(980.)).child(body))
  }
}
