//! The top of a machine's page: who it is and the buttons that matter now.

use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement, Window};
use guise::prelude::*;
use vista_kit::ipc::Request;
use vista_kit::machine::State;

use super::Detail;
use crate::state::Machine;
use crate::ui::glyph;

impl Detail {
  pub fn header(&mut self, machine: &Machine, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let dimmed = t.dimmed().hsla();
    let state = machine.state();
    let live = machine.live.clone();
    let id = machine.id().to_string();
    let lib = self.library.clone();

    let act = |label: &'static str, icon: IconName, variant: Variant, request: Option<Request>| {
      let lib = lib.clone();
      let id = id.clone();
      Button::new(label, label)
        .size(Size::Sm)
        .variant(variant)
        .left_section(Icon::new(icon).size(Size::Sm))
        .on_click(move |_, _, cx| {
          let id = id.clone();
          let request = request.clone();
          lib.update(cx, |l, cx| match request {
            Some(request) => l.control(&id, request, cx),
            None => l.start(&id, true, cx),
          });
        })
    };

    let mut actions = Group::new().gap(Size::Xs);
    match state {
      State::Stopped | State::Suspended | State::Error if live.is_none() => {
        let label = if state == State::Suspended { "Resume" } else { "Start" };
        actions = actions.child(act(label, IconName::Play, Variant::Filled, None));
        let lib = lib.clone();
        let id = id.clone();
        actions = actions.child(
          ActionIcon::new("start-hidden", IconName::EyeOff)
            .label("Start in the background")
            .variant(Variant::Default)
            .on_click(move |_, _, cx| {
              let id = id.clone();
              lib.update(cx, |l, cx| l.start(&id, false, cx));
            }),
        );
      }
      State::Paused => {
        actions = actions
          .child(act("Resume", IconName::Play, Variant::Filled, Some(Request::Resume)))
          .child(act("Shut Down", IconName::Power, Variant::Default, Some(Request::Stop)));
      }
      State::Error => {
        actions = actions.child(act("Close", IconName::X, Variant::Default, Some(Request::Kill)));
      }
      _ => {
        let visible = live.as_ref().map(|l| l.visible).unwrap_or(false);
        actions = actions.child(if visible {
          act("Hide", IconName::EyeOff, Variant::Default, Some(Request::Hide))
        } else {
          act("Show", IconName::AppWindow, Variant::Filled, Some(Request::Show))
        });
        if state == State::Running {
          actions = actions
            .child(act("Pause", IconName::Pause, Variant::Default, Some(Request::Pause)))
            .child(act("Suspend", IconName::Moon, Variant::Default, Some(Request::Suspend)));
        }
        actions = actions
          .child(act("Shut Down", IconName::Power, Variant::Default, Some(Request::Stop)))
          .child({
            let lib = lib.clone();
            let id = id.clone();
            ActionIcon::new("force-stop", IconName::CircleStop)
              .label("Force stop")
              .variant(Variant::Default)
              .color(ColorName::Red)
              .on_click(move |_, _, cx| {
                let id = id.clone();
                lib.update(cx, |l, cx| l.control(&id, Request::Kill, cx));
              })
          });
      }
    }

    let badge_color = match state {
      State::Running => ColorName::Green,
      State::Paused | State::Suspended => ColorName::Yellow,
      State::Error => ColorName::Red,
      State::Stopped => ColorName::Gray,
      _ => ColorName::Indigo,
    };

    div()
      .flex()
      .items_center()
      .gap(px(14.))
      .px(px(24.))
      .pt(px(20.))
      .pb(px(14.))
      .child(glyph::badge(machine.family(), 46.))
      .child(
        div()
          .flex()
          .flex_col()
          .flex_1()
          .min_w(px(0.))
          .gap(px(4.))
          .child(
            div()
              .flex()
              .items_center()
              .gap(px(10.))
              .child(div().text_size(px(19.)).font_weight(gpui::FontWeight::SEMIBOLD).child(machine.name().to_string()))
              .child(Badge::new(state.label()).variant(Variant::Light).color(badge_color).size(Size::Sm)),
          )
          .child(
            div()
              .text_size(px(12.))
              .text_color(dimmed)
              .child(format!("{} · {}", machine.title(), machine.summary())),
          ),
      )
      .child(actions)
  }
}
