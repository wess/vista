//! Hardware: processors, memory, disk, display, network, sound.

use gpui::prelude::*;
use gpui::{div, px, App, Context, IntoElement, SharedString, Window};
use guise::prelude::*;
use vista_kit::machine::{Network, Os};

use super::Detail;
use crate::state::Machine;

const DISPLAYS: [(u32, u32, &str); 4] = [
  (1440, 900, "1440 × 900"),
  (1920, 1200, "1920 × 1200"),
  (2560, 1600, "2560 × 1600"),
  (2880, 1800, "2880 × 1800"),
];

pub fn row(label: &str, hint: &str, control: impl IntoElement, cx: &App) -> impl IntoElement {
  let t = theme(cx);
  div()
    .flex()
    .items_center()
    .justify_between()
    .gap(px(16.))
    .py(px(10.))
    .border_b_1()
    .border_color(t.border().hsla())
    .child(
      div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w(px(0.))
        .gap(px(2.))
        .child(div().text_size(px(13.)).child(label.to_string()))
        .when(!hint.is_empty(), |d| {
          d.child(
            div()
              .text_size(px(12.))
              .text_color(t.dimmed().hsla())
              .overflow_hidden()
              .text_ellipsis()
              .whitespace_nowrap()
              .child(hint.to_string()),
          )
        }),
    )
    .child(div().flex_none().child(control))
}

pub fn chips(
  id: &str,
  options: Vec<(String, bool)>,
  on_pick: impl Fn(usize, &mut gpui::Window, &mut App) + 'static,
  cx: &App,
) -> impl IntoElement {
  let t = theme(cx);
  let on_pick = std::rc::Rc::new(on_pick);
  let border = t.border().hsla();
  let active = t.primary().alpha(0.16);
  let primary = t.primary().hsla();
  div().flex().gap(px(6.)).children(options.into_iter().enumerate().map(|(i, (label, selected))| {
    let on_pick = on_pick.clone();
    div()
      .id(SharedString::from(format!("{id}-{i}")))
      .px(px(10.))
      .py(px(5.))
      .rounded(px(7.))
      .border_1()
      .text_size(px(12.))
      .cursor_pointer()
      .when(selected, |d| d.bg(active).border_color(primary).text_color(primary))
      .when(!selected, |d| d.border_color(border))
      .on_click(move |_, window, cx| on_pick(i, window, cx))
      .child(label)
  }))
}

impl Detail {
  pub fn hardware(&mut self, machine: &Machine, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let dimmed = t.dimmed().hsla();
    let Some(draft) = self.draft.clone() else {
      return div().child("This machine's files are missing.").into_any_element();
    };
    let running = machine.is_running();
    let entity = cx.entity();

    let display_options: Vec<(String, bool)> = DISPLAYS
      .iter()
      .map(|(w, h, label)| (label.to_string(), draft.display.width == *w && draft.display.height == *h))
      .collect();
    let pick_display = {
      let entity = entity.clone();
      move |i: usize, _: &mut Window, cx: &mut App| {
        entity.update(cx, |this, cx| {
          if let Some(draft) = &mut this.draft {
            draft.display.width = DISPLAYS[i].0;
            draft.display.height = DISPLAYS[i].1;
          }
          cx.notify();
        });
      }
    };
    let network_options = vec![
      ("Shared".to_string(), draft.network == Network::Shared),
      ("Off".to_string(), draft.network == Network::None),
    ];
    let pick_network = {
      let entity = entity.clone();
      move |i: usize, _: &mut Window, cx: &mut App| {
        entity.update(cx, |this, cx| {
          if let Some(draft) = &mut this.draft {
            draft.network = if i == 0 { Network::Shared } else { Network::None };
          }
          cx.notify();
        });
      }
    };
    let toggle = |id: &'static str, value: bool, set: fn(&mut vista_kit::machine::Config, bool)| {
      let entity = entity.clone();
      Switch::new(id).checked(value).on_change(move |_, _, cx| {
        entity.update(cx, |this, cx| {
          if let Some(draft) = &mut this.draft {
            set(draft, !value);
          }
          cx.notify();
        });
      })
    };

    let save = {
      let entity = entity.clone();
      Button::new("save-hardware", "Save Changes").on_click(move |_, _, cx| {
        entity.update(cx, |this, cx| {
          let name = this.name.read(cx).text();
          let cpus = this.cpus.read(cx).value_f64();
          let memory = this.memory.read(cx).value_f64();
          let disk = this.disk.read(cx).value_f64();
          // A running guest's disk can't change size under it.
          let running = this.machine(cx).map(|m| m.is_running()).unwrap_or(true);
          if let Some(draft) = &mut this.draft {
            if !name.trim().is_empty() {
              draft.name = name.trim().to_string();
            }
            if let Some(cpus) = cpus {
              draft.cpus = cpus.max(1.0) as u32;
            }
            if let Some(memory) = memory {
              draft.memory = (memory.max(1.0) * 1024.0) as u64;
            }
            if let (Some(disk), false, Some(system)) = (disk, running, draft.disks.first_mut()) {
              system.size = system.size.max(disk as u64 * 1024 * 1024 * 1024);
            }
          }
          let Some(config) = this.merged(cx) else { return };
          let disk_grew = this
            .machine(cx)
            .and_then(|m| m.bundle.as_ref())
            .and_then(|b| b.config.system().map(|d| (d.size, b.path.join(&d.file))));
          if let (Some((old, file)), Some(new), false) = (disk_grew, config.system().map(|d| d.size), running) {
            if new > old {
              let _ = std::fs::OpenOptions::new().write(true).open(file).and_then(|f| f.set_len(new));
            }
          }
          this.library.update(cx, |l, cx| l.save_config(config, cx));
        });
      })
    };

    div()
      .flex()
      .flex_col()
      .max_w(px(720.))
      .when(running, |d| {
        d.child(
          div()
            .mb(px(12.))
            .text_size(px(12.))
            .text_color(dimmed)
            .child("The machine is running. Changes are saved now and apply when it next starts; the disk can only grow while it's off."),
        )
      })
      .child(div().pb(px(12.)).child(self.name.clone()))
      .child(
        div()
          .flex()
          .gap(px(12.))
          .pb(px(8.))
          .child(div().flex_1().child(self.cpus.clone()))
          .child(div().flex_1().child(self.memory.clone()))
          .child(div().flex_1().child(self.disk.clone())),
      )
      .child(row("Display", "The window resizes the display while running.", chips("display", display_options, pick_display, cx), cx))
      .child(row("Network", "Shared uses the Mac's connection; the guest gets its own address.", chips("network", network_options, pick_network, cx), cx))
      .child(row("Sound", "", toggle("audio", draft.audio, |c, v| c.audio = v), cx))
      .child(row("Microphone", "Let the guest use the Mac's microphone.", toggle("mic", draft.microphone, |c, v| c.microphone = v), cx))
      .when(draft.os != Os::Macos, |d| {
        d.child(row("Nested virtualization", "Run VMs inside this machine (M3 or later).", toggle("nested", draft.nested, |c, v| c.nested = v), cx))
      })
      .child(row("Start with Vista", "Start in the background when Vista opens.", toggle("autostart", draft.autostart, |c, v| c.autostart = v), cx))
      .child(div().pt(px(18.)).child(save))
      .into_any_element()
  }
}
