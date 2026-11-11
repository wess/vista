//! The overview: what the machine looks like right now and the numbers worth
//! knowing, plus the things you do to a machine rather than in it.

use gpui::prelude::*;
use gpui::{div, img, px, App, ClipboardItem, Context, IntoElement, ObjectFit, Window};
use guise::prelude::*;
use vista_kit::machine::{Installer, Network, State};

use super::Detail;
use crate::state::Machine;
use crate::ui::{format, glyph};

fn stat(label: &str, value: String, cx: &App) -> impl IntoElement {
  let t = theme(cx);
  div()
    .flex()
    .flex_col()
    .gap(px(4.))
    .p(px(14.))
    .rounded(px(10.))
    .bg(t.surface().hsla())
    .border_1()
    .border_color(t.border().hsla())
    .min_w(px(140.))
    .min_h(px(84.))
    .flex_1()
    .child(
      div()
        .text_size(px(11.))
        .text_color(t.dimmed().hsla())
        .child(label.to_string()),
    )
    .child(
      div()
        .text_size(px(17.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .child(value),
    )
}

impl Detail {
  fn preview(&self, machine: &Machine, cx: &App) -> impl IntoElement {
    let t = theme(cx);
    let screen = self.library.read(cx).screens.get(machine.id()).cloned();
    let state = machine.state();
    let frame = div()
      .w_full()
      .h(px(280.))
      .flex_none()
      .rounded(px(12.))
      .overflow_hidden()
      .bg(color!("#111114"))
      .border_1()
      .border_color(t.border().hsla())
      .flex()
      .items_center()
      .justify_center();
    match screen {
      Some(image) if machine.is_running() => {
        frame.child(img(image).size_full().object_fit(ObjectFit::Contain))
      }
      _ => {
        let message = match (state, &machine.live) {
          (State::Installing, Some(live)) => format!(
            "Installing macOS — {:.0}%",
            live.progress.unwrap_or(0.0) * 100.0
          ),
          (State::Running, Some(live)) if !live.visible => "Running in the background".into(),
          (State::Running, _) => "Starting…".into(),
          (State::Suspended, _) => "Suspended — resumes where it left off".into(),
          (State::Error, Some(live)) => live
            .message
            .clone()
            .unwrap_or_else(|| "Stopped with an error".into()),
          _ => "Powered off".into(),
        };
        frame.child(
          div()
            .flex()
            .flex_col()
            .items_center()
            .p(px(24.))
            .gap(px(12.))
            .child(glyph::badge(machine.family(), 56.))
            .child(
              div()
                .text_size(px(13.))
                .text_center()
                .text_color(color!("#A6A7AB"))
                .child(message),
            )
            .when(state == State::Installing, |d| {
              let value = machine
                .live
                .as_ref()
                .and_then(|l| l.progress)
                .unwrap_or(0.0) as f32
                * 100.0;
              d.child(div().w(px(260.)).child(Progress::new(value)))
            }),
        )
      }
    }
  }

  pub fn overview(
    &mut self,
    machine: &Machine,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> impl IntoElement {
    let t = theme(cx);
    let dimmed = t.dimmed().hsla();
    let config = machine.bundle.as_ref().map(|b| b.config.clone());
    let live = machine.live.clone();
    let ip = live.as_ref().and_then(|l| l.ip.clone());
    let disk_size = config
      .as_ref()
      .and_then(|c| c.system())
      .map(|d| d.size)
      .unwrap_or(0);

    let mut stats = div().flex().flex_none().flex_wrap().gap(px(10.));
    stats = stats
      .child(stat(
        "Uptime",
        live
          .as_ref()
          .filter(|l| l.uptime > 0)
          .map(|l| format::duration(l.uptime))
          .unwrap_or_else(|| "—".into()),
        cx,
      ))
      .child(stat(
        "Address",
        match (&ip, config.as_ref().map(|c| &c.network)) {
          (Some(ip), _) => ip.clone(),
          (None, Some(Network::None)) => "No network".into(),
          (None, _) if machine.is_running() => "Waiting for lease…".into(),
          _ => "—".into(),
        },
        cx,
      ))
      .child(stat(
        "Disk",
        format!(
          "{} of {}",
          format::bytes(machine.usage),
          format::gib(disk_size)
        ),
        cx,
      ))
      .child(stat(
        "Memory",
        config
          .as_ref()
          .map(|c| format!("{} GB", (c.memory as f64 / 1024.0).round()))
          .unwrap_or_default(),
        cx,
      ))
      .child(stat(
        "Processors",
        config
          .as_ref()
          .map(|c| c.cpus.to_string())
          .unwrap_or_default(),
        cx,
      ));

    let id = machine.id().to_string();
    let lib = self.library.clone();
    let mut notes = div().flex().flex_col().gap(px(10.));
    if let Some(Installer::Iso { path, .. }) = config.as_ref().and_then(|c| c.installer.clone()) {
      let file = std::path::Path::new(&path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or(path.clone());
      let lib = lib.clone();
      let id = id.clone();
      let t = theme(cx);
      notes = notes.child(
        div()
          .flex()
          .items_center()
          .justify_between()
          .gap(px(12.))
          .p(px(12.))
          .rounded(px(10.))
          .bg(t.primary().alpha(0.08))
          .border_1()
          .border_color(t.primary().alpha(0.3))
          .child(
            div()
              .flex()
              .flex_1()
              .min_w(px(0.))
              .items_center()
              .gap(px(10.))
              .child(Icon::new(IconName::Disc3).size(Size::Sm))
              .child(
                div()
                  .flex_1()
                  .min_w(px(0.))
                  .text_size(px(12.))
                  .child(format!(
                "{file} is attached. Vista ejects it when the guest powers off after installing."
              )),
              ),
          )
          .child(
            Button::new("eject", "Eject Now")
              .size(Size::Xs)
              .variant(Variant::Default)
              .on_click(move |_, _, cx| {
                let id = id.clone();
                lib.update(cx, |l, cx| {
                  if let Some(mut config) = l
                    .machine(&id)
                    .and_then(|m| m.bundle.as_ref())
                    .map(|b| b.config.clone())
                  {
                    config.installer = None;
                    l.save_config(config, cx);
                  }
                });
              }),
          ),
      );
    }
    if let (Some(ip), true) = (ip.clone(), machine.is_running()) {
      notes = notes.child(
        div()
          .flex()
          .items_center()
          .gap(px(8.))
          .text_size(px(12.))
          .text_color(dimmed)
          .child(format!("Reach the guest from this Mac at {ip}"))
          .child(
            ActionIcon::new("copy-ip", IconName::Copy)
              .label("Copy address")
              .size(Size::Sm)
              .variant(Variant::Subtle)
              .on_click(move |_, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(ip.clone()))
              }),
          ),
      );
    }

    let path = machine.bundle.as_ref().map(|b| b.path.clone());
    let running = machine.is_running();
    let overlays = self.overlays.clone();
    let name = machine.name().to_string();
    let manage = {
      let lib_clone = lib.clone();
      let lib_delete = lib.clone();
      let id_clone = id.clone();
      let id_delete = id.clone();
      div()
        .flex()
        .flex_wrap()
        .gap(px(8.))
        .child(
          Button::new("clone", "Clone")
            .size(Size::Sm)
            .variant(Variant::Default)
            .disabled(running)
            .left_section(Icon::new(IconName::Copy).size(Size::Sm))
            .on_click(move |_, _, cx| {
              let id = id_clone.clone();
              lib_clone.update(cx, |l, cx| l.duplicate(&id, cx));
            }),
        )
        .child(
          Button::new("reveal", "Show in Finder")
            .size(Size::Sm)
            .variant(Variant::Default)
            .left_section(Icon::new(IconName::FolderOpen).size(Size::Sm))
            .on_click(move |_, _, cx| {
              if let Some(path) = &path {
                cx.reveal_path(path);
              }
            }),
        )
        .child(
          Button::new("delete", "Delete…")
            .size(Size::Sm)
            .variant(Variant::Subtle)
            .color(ColorName::Red)
            .disabled(running)
            .left_section(Icon::new(IconName::Trash2).size(Size::Sm))
            .on_click(move |_, window, cx| {
              let id = id_delete.clone();
              let lib = lib_delete.clone();
              super::confirm::ask(
                &overlays,
                &format!("Delete {name}?"),
                "Its disks and snapshots move to the Trash.",
                "Delete",
                window,
                cx,
                move |cx| lib.update(cx, |l, cx| l.delete(&id, cx)),
              );
            }),
        )
    };
    let _ = window;

    div()
      .flex()
      .flex_col()
      .w_full()
      .min_w(px(0.))
      .gap(px(18.))
      .child(self.preview(machine, cx))
      .child(stats)
      .child(notes)
      .child(Divider::new())
      .child(manage)
  }
}
