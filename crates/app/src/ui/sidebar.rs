//! The machine list, with creation jobs underneath and a button to make one.

use gpui::prelude::*;
use gpui::{div, px, App, Entity, IntoElement, MouseButton, MouseDownEvent, Window};
use guise::prelude::*;
use vista_kit::jobs::{Job, Phase};

use super::format;
use super::glyph;
use super::root::SIDEBAR;
use crate::state::{Library, Machine, Page};

#[derive(Clone)]
pub enum Target {
  Machine(String),
  Job(String),
  Library,
}

fn row(
  machine: &Machine,
  selected: bool,
  library: &Entity<Library>,
  on_menu: impl Fn(Target, &MouseDownEvent, &mut Window, &mut App) + Clone + 'static,
  cx: &App,
) -> impl IntoElement {
  let t = theme(cx);
  let text = t.text().hsla();
  let dimmed = t.dimmed().hsla();
  let hover = t.surface_hover().hsla();
  let active = t.primary().alpha(0.14);
  let state = machine.state();
  let subtitle = match (&machine.live, state) {
    (Some(live), vista_kit::machine::State::Installing) => {
      format!(
        "Installing {}%",
        (live.progress.unwrap_or(0.0) * 100.0).round()
      )
    }
    (Some(live), vista_kit::machine::State::Running) if live.uptime > 0 => {
      format!("Running · {}", format::duration(live.uptime))
    }
    _ if machine.bundle.is_none() => "Missing".into(),
    _ => state.label().to_string(),
  };
  let id = machine.id().to_string();
  let library = library.clone();
  let menu_id = id.clone();
  let control_id = id.clone();
  let control_menu = on_menu.clone();
  div()
    .id(gpui::SharedString::from(format!("machine-{id}")))
    .flex()
    .flex_none()
    .items_center()
    .gap(px(10.))
    .mx(px(8.))
    .px(px(8.))
    .py(px(10.))
    .rounded(px(8.))
    .when(selected, |d| d.bg(active))
    .when(!selected, |d| d.hover(move |s| s.bg(hover)))
    .cursor_pointer()
    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
      if event.modifiers.control {
        cx.stop_propagation();
        control_menu(Target::Machine(control_id.clone()), event, window, cx);
      }
    })
    .on_mouse_down(MouseButton::Right, move |event, window, cx| {
      cx.stop_propagation();
      on_menu(Target::Machine(menu_id.clone()), event, window, cx);
    })
    .on_click(move |_, _, cx| {
      let id = id.clone();
      library.update(cx, |l, cx| l.select(Page::Machine(id), cx));
    })
    .child(glyph::badge(machine.family(), 30.))
    .child(
      div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w(px(0.))
        .child(
          div()
            .text_size(px(13.))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(text)
            .text_ellipsis()
            .overflow_hidden()
            .whitespace_normal()
            .line_clamp(1)
            .child(machine.name().to_string()),
        )
        .child(
          div()
            .flex()
            .items_center()
            .gap(px(5.))
            .text_size(px(11.))
            .text_color(dimmed)
            .child(glyph::dot(state))
            .child(
              div()
                .min_w(px(0.))
                .overflow_hidden()
                .whitespace_normal()
                .line_clamp(1)
                .text_ellipsis()
                .child(subtitle),
            ),
        ),
    )
}

fn job_row(
  job: &Job,
  library: &Entity<Library>,
  on_menu: impl Fn(Target, &MouseDownEvent, &mut Window, &mut App) + Clone + 'static,
  cx: &App,
) -> impl IntoElement {
  let t = theme(cx);
  let dimmed = t.dimmed().hsla();
  let text = t.text().hsla();
  let name = if job.spec.name.is_empty() {
    job
      .spec
      .system
      .as_deref()
      .and_then(vista_kit::catalog::find)
      .map(|e| e.title())
      .unwrap_or_else(|| "New machine".into())
  } else {
    job.spec.name.clone()
  };
  let family = job
    .spec
    .system
    .as_deref()
    .and_then(vista_kit::catalog::find)
    .map(|e| e.family)
    .unwrap_or("linux");
  let status = match (job.phase, job.fraction()) {
    (Phase::Downloading, Some(f)) => {
      format!(
        "{} of {}",
        format::bytes(job.received),
        format::bytes(job.total.unwrap_or(0))
      )
      .replace(&format!(" of {}", format::bytes(0)), "")
        + &format!(" · {:.0}%", f * 100.0)
    }
    (phase, _) => job
      .message
      .clone()
      .filter(|_| matches!(phase, Phase::Failed | Phase::Cancelled))
      .unwrap_or_else(|| phase.label().to_string()),
  };
  let finished = job.phase.is_finished();
  let fraction = job.fraction().unwrap_or(0.0) as f32;
  let id = job.id.clone();
  let open = library.clone();
  let open_id = id.clone();
  let library = library.clone();
  let menu_id = id.clone();
  let control_id = id.clone();
  let control_menu = on_menu.clone();
  div()
    .id(gpui::SharedString::from(format!("job-row-{id}")))
    .on_mouse_down(MouseButton::Right, move |event, window, cx| {
      cx.stop_propagation();
      on_menu(Target::Job(menu_id.clone()), event, window, cx);
    })
    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
      if event.modifiers.control {
        cx.stop_propagation();
        control_menu(Target::Job(control_id.clone()), event, window, cx);
      }
    })
    .cursor_pointer()
    .rounded(px(8.))
    .when(library.read(cx).page == Page::Job(id.clone()), |d| {
      d.bg(t.primary().alpha(0.14))
    })
    .hover(|style| style.bg(t.surface_hover().hsla()))
    .on_click(move |_, _, cx| {
      open.update(cx, |library, cx| {
        library.select(Page::Job(open_id.clone()), cx)
      })
    })
    .flex()
    .flex_col()
    .flex_none()
    .gap(px(6.))
    .mx(px(8.))
    .px(px(8.))
    .py(px(8.))
    .child(
      div()
        .flex()
        .items_center()
        .gap(px(10.))
        .child(div().opacity(0.6).child(glyph::badge(family, 30.)))
        .child(
          div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.))
            .child(
              div()
                .text_size(px(13.))
                .text_color(text)
                .overflow_hidden()
                .whitespace_normal()
                .line_clamp(1)
                .text_ellipsis()
                .child(name),
            )
            .child(
              div()
                .text_size(px(11.))
                .text_color(dimmed)
                .overflow_hidden()
                .whitespace_normal()
                .line_clamp(1)
                .text_ellipsis()
                .child(status),
            ),
        )
        .child(
          ActionIcon::new(gpui::SharedString::from(format!("job-{id}")), IconName::X)
            .label(if finished { "Dismiss" } else { "Cancel" })
            .variant(Variant::Subtle)
            .size(Size::Sm)
            .on_click(move |_, _, cx| {
              cx.stop_propagation();
              let id = id.clone();
              library.update(cx, |l, cx| {
                if finished {
                  l.dismiss_job(&id, cx)
                } else {
                  l.cancel_job(&id, cx)
                }
              });
            }),
        ),
    )
    .when(!finished && job.total.is_some(), |d| {
      d.child(Progress::new(fraction * 100.0).size(Size::Xs))
    })
}

fn heading(label: &str, cx: &App) -> impl IntoElement {
  let dimmed = theme(cx).dimmed().hsla();
  div()
    .px(px(16.))
    .pt(px(14.))
    .pb(px(6.))
    .text_size(px(11.))
    .font_weight(gpui::FontWeight::SEMIBOLD)
    .text_color(dimmed)
    .child(label.to_string())
}

pub fn render(
  library: &Entity<Library>,
  on_menu: impl Fn(Target, &MouseDownEvent, &mut Window, &mut App) + Clone + 'static,
  cx: &App,
) -> impl IntoElement {
  let t = theme(cx);
  let side = t.surface().hsla();
  let border = t.border().hsla();
  let lib = library.read(cx);
  let selected = match &lib.page {
    Page::Machine(id) => Some(id.clone()),
    _ => None,
  };
  let jobs: Vec<Job> = lib
    .jobs
    .iter()
    .filter(|j| j.phase != Phase::Done || lib.page == Page::Job(j.id.clone()))
    .cloned()
    .collect();
  let rows: Vec<gpui::AnyElement> = lib
    .machines
    .iter()
    .map(|m| {
      row(
        m,
        selected.as_deref() == Some(m.id()),
        library,
        on_menu.clone(),
        cx,
      )
      .into_any_element()
    })
    .collect();
  let job_rows: Vec<gpui::AnyElement> = jobs
    .iter()
    .map(|j| job_row(j, library, on_menu.clone(), cx).into_any_element())
    .collect();
  let empty = rows.is_empty();
  let settings = library.clone();
  let background_menu = on_menu.clone();
  div()
    .id("sidebar")
    .on_mouse_down(MouseButton::Right, move |event, window, cx| {
      cx.stop_propagation();
      background_menu(Target::Library, event, window, cx);
    })
    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
      if event.modifiers.control {
        cx.stop_propagation();
        on_menu(Target::Library, event, window, cx);
      }
    })
    .flex()
    .flex_col()
    .flex_none()
    .w(px(SIDEBAR))
    .h_full()
    .bg(side)
    .border_r_1()
    .border_color(border)
    .child(
      div()
        .id("machines")
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.))
        .overflow_y_scroll()
        .child(heading("Machines", cx))
        .when(empty, |d| {
          d.child(
            div()
              .px(px(16.))
              .text_size(px(12.))
              .text_color(theme(cx).dimmed().hsla())
              .child("No machines yet."),
          )
        })
        .children(rows)
        .when(!job_rows.is_empty(), |d| {
          d.child(heading("Creation jobs", cx)).children(job_rows)
        }),
    )
    .child(
      div()
        .flex_none()
        .p(px(12.))
        .border_t_1()
        .border_color(border)
        .child(
          Button::new("sidebar-settings", "Settings")
            .full_width(true)
            .variant(if matches!(lib.page, Page::Settings) {
              Variant::Light
            } else {
              Variant::Subtle
            })
            .left_section(Icon::new(IconName::Settings).size(Size::Sm))
            .on_click(move |_, _, cx| settings.update(cx, |l, cx| l.select(Page::Settings, cx))),
        ),
    )
}
