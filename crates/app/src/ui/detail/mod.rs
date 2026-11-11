//! One machine: its header and four tabs.

mod confirm;
mod hardware;

pub use hardware::{chips, row};
mod header;
mod overview;
mod sharing;
mod snapshots;

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, SharedString, Window};
use guise::prelude::*;
use vista_kit::machine::Config;

use crate::state::{library, Library, Machine};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
  Overview,
  Hardware,
  Sharing,
  Snapshots,
}

impl Tab {
  const ALL: [Tab; 4] = [Tab::Overview, Tab::Hardware, Tab::Sharing, Tab::Snapshots];

  fn label(self) -> &'static str {
    match self {
      Tab::Overview => "Overview",
      Tab::Hardware => "Hardware",
      Tab::Sharing => "Sharing",
      Tab::Snapshots => "Snapshots",
    }
  }
}

pub struct Detail {
  pub id: String,
  pub library: Entity<Library>,
  pub overlays: Entity<OverlayHost>,
  pub tab: Tab,
  /// The configuration being edited on the Hardware and Sharing tabs.
  pub draft: Option<Config>,
  pub name: Entity<TextInput>,
  pub cpus: Entity<NumberInput>,
  pub memory: Entity<NumberInput>,
  pub disk: Entity<NumberInput>,
  pub snapshot: Entity<TextInput>,
}

impl Detail {
  pub fn new(id: String, overlays: Entity<OverlayHost>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
    let library = library(cx);
    cx.observe(&library, |_, _, cx| cx.notify()).detach();
    let config = library
      .read(cx)
      .machine(&id)
      .and_then(|m| m.bundle.as_ref())
      .map(|b| b.config.clone());
    let host_cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8) as f64;
    let (name, cpus, memory, disk) = match &config {
      Some(c) => (
        c.name.clone(),
        c.cpus as f64,
        (c.memory as f64 / 1024.0).round(),
        c.system().map(|d| (d.size / (1024 * 1024 * 1024)) as f64).unwrap_or(64.0),
      ),
      None => (String::new(), 2.0, 4.0, 64.0),
    };
    Detail {
      id,
      library,
      overlays,
      tab: match std::env::var("VISTA_TAB").as_deref() {
        Ok("hardware") => Tab::Hardware,
        Ok("sharing") => Tab::Sharing,
        Ok("snapshots") => Tab::Snapshots,
        _ => Tab::Overview,
      },
      draft: config,
      name: cx.new(|cx| TextInput::new(cx).value(&name).label("Name")),
      cpus: cx.new(|cx| NumberInput::new(cx).value(cpus).min(1.0).max(host_cpus).step(1.0).label("Processors")),
      memory: cx.new(|cx| NumberInput::new(cx).value(memory).min(1.0).max(1024.0).step(1.0).label("Memory (GB)")),
      disk: cx.new(|cx| NumberInput::new(cx).value(disk).min(disk).max(8192.0).step(8.0).label("Disk (GB)")),
      snapshot: cx.new(|cx| TextInput::new(cx).placeholder("Snapshot name")),
    }
  }

  /// The machine's current configuration with this page's edits applied.
  /// Everything the page doesn't edit (installer media, identity, disks
  /// other than the system disk's size) comes from disk as it is now, so a
  /// stale draft can't undo what the machine did meanwhile.
  pub fn merged(&self, cx: &gpui::App) -> Option<Config> {
    let draft = self.draft.as_ref()?;
    let mut config = self.machine(cx)?.bundle.as_ref()?.config.clone();
    config.name = draft.name.clone();
    config.cpus = draft.cpus;
    config.memory = draft.memory;
    if let (Some(current), Some(edited)) = (config.disks.first_mut(), draft.disks.first()) {
      current.size = current.size.max(edited.size);
    }
    config.display = draft.display;
    config.network = draft.network.clone();
    config.audio = draft.audio;
    config.microphone = draft.microphone;
    config.nested = draft.nested;
    config.autostart = draft.autostart;
    config.shares = draft.shares.clone();
    config.clipboard = draft.clipboard;
    config.rosetta = draft.rosetta;
    config.agents = draft.agents;
    Some(config)
  }

  pub fn machine<'a>(&self, cx: &'a gpui::App) -> Option<&'a Machine> {
    self.library.read(cx).machine(&self.id)
  }

  fn tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let text = t.text().hsla();
    let dimmed = t.dimmed().hsla();
    let active_bg = t.surface_hover().hsla();
    let current = self.tab;
    div().flex().gap(px(4.)).children(Tab::ALL.iter().map(|tab| {
      let tab = *tab;
      let active = tab == current;
      div()
        .id(SharedString::from(format!("tab-{}", tab.label())))
        .px(px(12.))
        .py(px(5.))
        .rounded(px(7.))
        .text_size(px(13.))
        .cursor_pointer()
        .when(active, |d| d.bg(active_bg).text_color(text).font_weight(gpui::FontWeight::MEDIUM))
        .when(!active, |d| d.text_color(dimmed).hover(move |s| s.text_color(text)))
        .on_click(cx.listener(move |this, _, _, cx| {
          this.tab = tab;
          cx.notify();
        }))
        .child(tab.label())
    }))
  }
}

impl Render for Detail {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let Some(machine) = self.machine(cx).cloned() else {
      return div().into_any_element();
    };
    let border = theme(cx).border().hsla();
    let body = match self.tab {
      Tab::Overview => self.overview(&machine, window, cx).into_any_element(),
      Tab::Hardware => self.hardware(&machine, window, cx).into_any_element(),
      Tab::Sharing => self.sharing(&machine, window, cx).into_any_element(),
      Tab::Snapshots => self.snapshots(&machine, window, cx).into_any_element(),
    };
    div()
      .flex()
      .flex_col()
      .size_full()
      .child(self.header(&machine, window, cx))
      .child(
        div()
          .px(px(24.))
          .pb(px(10.))
          .border_b_1()
          .border_color(border)
          .child(self.tabs(cx)),
      )
      .child(
        div()
          .id("detail-body")
          .flex_1()
          .overflow_y_scroll()
          .p(px(24.))
          .child(body),
      )
      .into_any_element()
  }
}
