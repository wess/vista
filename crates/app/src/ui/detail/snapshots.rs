//! Snapshots: take one, go back to one.

use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement, SharedString, Window};
use guise::prelude::*;

use super::Detail;
use crate::state::Machine;
use crate::ui::format;

impl Detail {
  pub fn snapshots(&mut self, machine: &Machine, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let dimmed = t.dimmed().hsla();
    let border = t.border().hsla();
    let surface = t.surface().hsla();
    let id = machine.id().to_string();
    let running = machine.is_running();
    let lib = self.library.clone();
    let input = self.snapshot.clone();

    let take = {
      let lib = lib.clone();
      let id = id.clone();
      let input = input.clone();
      Button::new("take-snapshot", "Take Snapshot")
        .left_section(Icon::new(IconName::Camera).size(Size::Sm))
        .on_click(move |_, _, cx| {
          let name = input.read(cx).text();
          input.update(cx, |i, cx| i.set_text("", cx));
          let id = id.clone();
          lib.update(cx, |l, cx| l.snapshot(&id, name, cx));
        })
    };

    let rows = machine.snapshots.iter().map(|snapshot| {
      let lib_revert = lib.clone();
      let lib_delete = lib.clone();
      let overlays = self.overlays.clone();
      let overlays2 = self.overlays.clone();
      let id_revert = id.clone();
      let id_delete = id.clone();
      let snap_revert = snapshot.id.clone();
      let snap_delete = snapshot.id.clone();
      let name = snapshot.name.clone();
      let name2 = snapshot.name.clone();
      div()
        .flex()
        .items_center()
        .gap(px(12.))
        .px(px(14.))
        .py(px(10.))
        .border_b_1()
        .border_color(border)
        .child(Icon::new(IconName::History).size(Size::Sm))
        .child(
          div()
            .flex()
            .flex_col()
            .flex_1()
            .child(div().text_size(px(13.)).child(snapshot.name.clone()))
            .child(div().text_size(px(11.)).text_color(dimmed).child(format::ago(snapshot.created))),
        )
        .when(snapshot.live, |d| d.child(Badge::new("With memory").size(Size::Xs).variant(Variant::Light).color(ColorName::Indigo)))
        .child(
          Button::new(SharedString::from(format!("revert-{}", snapshot.id)), "Revert")
            .size(Size::Xs)
            .variant(Variant::Default)
            .on_click(move |_, window, cx| {
              let lib = lib_revert.clone();
              let id = id_revert.clone();
              let snap = snap_revert.clone();
              super::confirm::ask(
                &overlays,
                &format!("Revert to \u{201c}{name}\u{201d}?"),
                if running {
                  "The machine is stopped first. Everything since the snapshot is lost."
                } else {
                  "Everything since the snapshot is lost."
                },
                "Revert",
                window,
                cx,
                move |cx| lib.update(cx, |l, cx| l.revert(&id, snap.clone(), cx)),
              );
            }),
        )
        .child(
          ActionIcon::new(SharedString::from(format!("delete-{}", snapshot.id)), IconName::Trash2)
            .label("Delete snapshot")
            .size(Size::Sm)
            .variant(Variant::Subtle)
            .on_click(move |_, window, cx| {
              let lib = lib_delete.clone();
              let id = id_delete.clone();
              let snap = snap_delete.clone();
              super::confirm::ask(
                &overlays2,
                &format!("Delete \u{201c}{name2}\u{201d}?"),
                "The machine itself isn't affected.",
                "Delete",
                window,
                cx,
                move |cx| lib.update(cx, |l, cx| l.delete_snapshot(&id, snap.clone(), cx)),
              );
            }),
        )
    });

    div()
      .flex()
      .flex_col()
      .gap(px(12.))
      .max_w(px(680.))
      .child(
        div()
          .text_size(px(12.))
          .text_color(dimmed)
          .child("Snapshots are copy-on-write: instant, and they only use space as the machine changes. Taken while running, they keep the machine's memory too."),
      )
      .child(div().flex().gap(px(8.)).items_end().child(div().flex_1().child(input)).child(take))
      .child(
        div()
          .rounded(px(10.))
          .border_1()
          .border_color(border)
          .bg(surface)
          .overflow_hidden()
          .when(machine.snapshots.is_empty(), |d| {
            d.child(div().p(px(14.)).text_size(px(12.)).text_color(dimmed).child("No snapshots yet."))
          })
          .children(rows),
      )
  }
}
