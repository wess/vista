//! The quick view: every machine at a glance, with start and stop.

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, SharedString, Subscription, Window};
use guise::prelude::*;
use vista_kit::ipc::Request;
use vista_kit::machine::State;

use crate::state::{library, Library, Machine, Page};
use crate::ui::glyph;

pub struct Panel {
  library: Entity<Library>,
  _activation: Subscription,
}

impl Panel {
  pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
    let library = library(cx);
    cx.observe(&library, |_, _, cx| cx.notify()).detach();
    // Close when the person clicks anywhere else.
    let activation = cx.observe_window_activation(window, |_, window, _| {
      if !window.is_window_active() {
        super::closed();
        window.remove_window();
      }
    });
    Panel {
      library,
      _activation: activation,
    }
  }

  fn row(&self, machine: &Machine, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let dimmed = t.dimmed().hsla();
    let hover = t.surface_hover().hsla();
    let state = machine.state();
    let id = machine.id().to_string();
    let lib = self.library.clone();
    let detail = match (&machine.live, state) {
      (Some(live), State::Running) => {
        let mut parts = vec![format!("Running · {}", crate::ui::format::duration(live.uptime))];
        if let Some(ip) = &live.ip {
          parts.push(ip.clone());
        }
        parts.join(" · ")
      }
      (Some(live), State::Installing) => format!("Installing {:.0}%", live.progress.unwrap_or(0.0) * 100.0),
      _ => format!("{} · {}", state.label(), machine.summary()),
    };
    let running = machine.is_running();
    let open = {
      let lib = lib.clone();
      let id = id.clone();
      move |_: &gpui::ClickEvent, window: &mut Window, cx: &mut gpui::App| {
        super::closed();
        window.remove_window();
        let id = id.clone();
        lib.update(cx, |l, cx| {
          if running {
            l.control(&id, Request::Show, cx);
          } else {
            l.select(Page::Machine(id), cx);
          }
        });
        if !running {
          crate::open_main(cx);
        }
      }
    };
    let action = if running {
      let lib = lib.clone();
      let id = id.clone();
      ActionIcon::new(SharedString::from(format!("stop-{id}")), IconName::Power)
        .label("Shut down")
        .variant(Variant::Subtle)
        .on_click(move |_, _, cx| {
          let id = id.clone();
          lib.update(cx, |l, cx| l.control(&id, Request::Stop, cx));
        })
    } else {
      let lib = lib.clone();
      let id = id.clone();
      ActionIcon::new(SharedString::from(format!("start-{id}")), IconName::Play)
        .label("Start")
        .variant(Variant::Light)
        .on_click(move |_, _, cx| {
          let id = id.clone();
          lib.update(cx, |l, cx| l.start(&id, true, cx));
        })
    };
    div()
      .id(SharedString::from(format!("panel-{id}")))
      .flex()
      .items_center()
      .gap(px(10.))
      .px(px(10.))
      .py(px(7.))
      .rounded(px(8.))
      .hover(move |s| s.bg(hover))
      .cursor_pointer()
      .on_click(open)
      .child(glyph::badge(machine.family(), 32.))
      .child(
        div()
          .flex()
          .flex_col()
          .flex_1()
          .min_w(px(0.))
          .child(div().text_size(px(13.)).font_weight(gpui::FontWeight::MEDIUM).overflow_hidden().whitespace_nowrap().text_ellipsis().child(machine.name().to_string()))
          .child(
            div()
              .flex()
              .items_center()
              .gap(px(5.))
              .text_size(px(11.))
              .text_color(dimmed)
              .overflow_hidden()
              .whitespace_nowrap()
              .child(glyph::dot(state))
              .child(detail),
          ),
      )
      .child(action)
  }
}

impl Render for Panel {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let body = t.body().hsla();
    let border = t.border().hsla();
    let dimmed = t.dimmed().hsla();
    let text = t.text().hsla();
    let machines = self.library.read(cx).machines.clone();
    let running = machines.iter().filter(|m| m.is_running()).count();
    let rows: Vec<gpui::AnyElement> = machines.iter().map(|m| self.row(m, cx).into_any_element()).collect();
    let lib = self.library.clone();
    div()
      .size_full()
      .flex()
      .flex_col()
      .bg(body)
      .text_color(text)
      .font_family(".SystemUIFont")
      .border_1()
      .border_color(border)
      .rounded(px(12.))
      .overflow_hidden()
      .child(
        div()
          .flex()
          .items_center()
          .justify_between()
          .px(px(14.))
          .pt(px(12.))
          .pb(px(8.))
          .child(div().text_size(px(13.)).font_weight(gpui::FontWeight::SEMIBOLD).child("Vista"))
          .child(div().text_size(px(12.)).text_color(dimmed).child(if running == 0 {
            "Nothing running".to_string()
          } else {
            format!("{running} running")
          })),
      )
      .child(
        div()
          .id("panel-list")
          .flex()
          .flex_col()
          .flex_1()
          .px(px(4.))
          .overflow_y_scroll()
          .when(rows.is_empty(), |d| {
            d.child(div().p(px(12.)).text_size(px(12.)).text_color(dimmed).child("No machines yet."))
          })
          .children(rows),
      )
      .child(
        div()
          .flex()
          .items_center()
          .justify_between()
          .px(px(10.))
          .py(px(8.))
          .border_t_1()
          .border_color(border)
          .child(
            Group::new()
              .gap(Size::Xs)
              .child(Button::new("panel-open", "Open Vista").size(Size::Xs).variant(Variant::Subtle).on_click(|_, window, cx| {
                super::closed();
                window.remove_window();
                crate::open_main(cx);
              }))
              .child(Button::new("panel-new", "New Machine").size(Size::Xs).variant(Variant::Subtle).on_click(move |_, window, cx| {
                super::closed();
                window.remove_window();
                lib.update(cx, |l, cx| l.select(Page::Create, cx));
                crate::open_main(cx);
              })),
          )
          .child(Button::new("panel-quit", "Quit").size(Size::Xs).variant(Variant::Subtle).on_click(|_, _, cx| cx.quit())),
      )
  }
}
