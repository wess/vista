use gpui::prelude::*;
use gpui::{div, px, App, ClipboardItem, Entity, IntoElement, SharedString};
use guise::prelude::*;
use vista_kit::jobs::{Job, Phase};

use super::{format, layout};
use crate::state::{Library, Page};

pub fn render(job: &Job, library: &Entity<Library>, cx: &App) -> impl IntoElement {
  let t = theme(cx);
  let dimmed = t.dimmed().hsla();
  let finished = job.phase.is_finished();
  let failed = job.phase == Phase::Failed;
  let now = vista_kit::machine::now();
  let elapsed = (if finished { job.updated } else { now } - job.created).max(0) as u64;
  let message = job
    .message
    .clone()
    .unwrap_or_else(|| job.phase.label().into());
  let source = job
    .spec
    .system
    .as_deref()
    .and_then(vista_kit::catalog::find)
    .map(|entry| entry.title())
    .or_else(|| job.spec.file.clone())
    .unwrap_or_else(|| "Local installer".into());
  let diagnostics = format!("Machine: {}\nSystem: {}\nJob: {}\nStep: {}\nStatus: {}\nElapsed: {}\nDownloaded: {}\nLast update: {}\n{}", job.title(), source, job.id, job.phase.label(), message,
    format::duration(elapsed), format::bytes(job.received), job.updated,
    job.events.iter().map(|event| format!("{}: {}", event.phase.label(), event.message)).collect::<Vec<_>>().join("\n"));
  let copy = Button::new("copy-job", "Copy Details")
    .variant(Variant::Default)
    .left_section(Icon::new(IconName::Copy).size(Size::Sm))
    .on_click(move |_, _, cx| {
      cx.write_to_clipboard(ClipboardItem::new_string(diagnostics.clone()))
    });
  let library_action = library.clone();
  let id = job.id.clone();
  let action = Button::new(
    "job-action",
    if finished {
      "Dismiss"
    } else {
      "Cancel Creation"
    },
  )
  .variant(Variant::Default)
  .on_click(move |_, _, cx| {
    library_action.update(cx, |library, cx| {
      if finished {
        library.dismiss_job(&id, cx);
      } else {
        library.cancel_job(&id, cx);
      }
    })
  });
  let retry_library = library.clone();
  let spec = job.spec.clone();
  let retry = Button::new("retry-job", "Try Again").on_click(move |_, _, cx| {
    retry_library.update(cx, |library, cx| library.create(spec.clone(), cx));
  });
  let installer_library = library.clone();
  let installer = Button::new("local-installer", "Change System or Installer…")
    .variant(Variant::Default)
    .on_click(move |_, _, cx| {
      installer_library.update(cx, |library, cx| library.select(Page::Create, cx))
    });
  let fraction = if matches!(job.phase, Phase::Downloading | Phase::Verifying) {
    job.fraction()
  } else {
    None
  };
  let progress = if let Some(fraction) = fraction {
    format!(
      "{} of {} · {:.0}%",
      format::bytes(job.received),
      format::bytes(job.total.unwrap_or(0)),
      fraction * 100.
    )
  } else if job.phase == Phase::Resolving {
    "Contacting the publisher. Installer discovery is limited to 90 seconds.".into()
  } else if job.phase == Phase::Downloading {
    format!(
      "{} downloaded · the publisher has not supplied a total size",
      format::bytes(job.received)
    )
  } else {
    "".into()
  };
  let speed = job
    .rate
    .filter(|rate| rate.is_finite() && *rate > 0. && now - job.updated < 10)
    .filter(|_| job.phase == Phase::Downloading)
    .map(|rate| {
      let remaining = job
        .total
        .map(|total| {
          format!(
            " · about {} remaining",
            format::duration((total.saturating_sub(job.received) as f64 / rate).ceil() as u64)
          )
        })
        .unwrap_or_default();
      format!("{}/s{remaining}", format::bytes(rate as u64))
    });
  let last_step = job
    .events
    .iter()
    .rev()
    .find(|event| !event.phase.is_finished())
    .map(|event| event.phase);
  let stages = [
    (Phase::Resolving, "Find installer"),
    (Phase::Downloading, "Download"),
    (Phase::Verifying, "Verify"),
    (Phase::Creating, "Create machine"),
    (Phase::Preparing, "Prepare installer"),
    (Phase::Starting, "Start"),
  ];
  let activity = job.events.iter().enumerate().map(|(index, event)| {
    div()
      .id(SharedString::from(format!("activity-{index}")))
      .flex()
      .flex_none()
      .gap(px(14.))
      .py(px(10.))
      .child(
        div()
          .flex_none()
          .w(px(62.))
          .text_size(px(11.))
          .text_color(dimmed)
          .child(format!(
            "+{}",
            format::duration((event.at - job.created).max(0) as u64)
          )),
      )
      .child(
        div()
          .flex_1()
          .min_w(px(0.))
          .text_size(px(12.))
          .child(event.message.clone()),
      )
  });
  div().id(SharedString::from(format!("job-{}", job.id))).size_full().min_h(px(0.)).overflow_y_scroll()
    .child(div().flex().flex_col().w_full().max_w(px(900.)).mx_auto().p(px(32.))
      .child(layout::heading(&job.title(), &source, cx))
      .child(layout::group(cx).py(px(20.)).gap(px(12.))
        .child(div().flex().items_center().justify_between().gap(px(12.))
          .child(div().text_size(px(20.)).font_weight(gpui::FontWeight::SEMIBOLD).child(job.phase.label()))
          .child(div().flex_none().text_size(px(12.)).text_color(dimmed).child(format::duration(elapsed))))
        .child(div().text_size(px(13.)).line_height(px(20.)).when(failed, |d| d.text_color(t.danger().hsla())).child(message))
        .when(!progress.is_empty() && !finished, |d| d.child(div().text_size(px(12.)).text_color(dimmed).child(progress)))
        .children(speed.map(|speed| div().text_size(px(12.)).text_color(dimmed).child(speed)))
        .when(fraction.is_some() && !finished, |d| d.child(Progress::new(fraction.unwrap_or(0.) as f32 * 100.)))
        .when(!finished && fraction.is_none(), |d| d.child(div().flex().items_center().gap(px(8.))
          .child(Loader::new().size(Size::Sm))
          .child(div().text_size(px(12.)).text_color(dimmed).child("Working…")))))
      .child(div().flex().flex_wrap().gap(px(8.)).pt(px(16.)).child(action).child(copy)
        .when(matches!(job.phase, Phase::Failed | Phase::Cancelled), |d| d.child(retry).child(installer)))
      .child(layout::section("Creation steps", cx))
      .child(layout::group(cx).children(stages.into_iter().map(|(phase, label)| {
        let reached = job.events.iter().any(|event| event.phase == phase);
        let current = job.phase == phase;
        div().flex().flex_none().items_center().justify_between().gap(px(12.)).py(px(10.))
          .text_size(px(12.)).text_color(if current { t.primary().hsla() } else { dimmed })
          .child(label).child(if current { "In progress" } else if reached && finished && job.phase != Phase::Done && last_step == Some(phase) { "Stopped" } else if reached { "Completed" } else if job.phase == Phase::Done { "Not required" } else { "Not reached" })
      })))
      .child(layout::section("Activity", cx))
      .child(layout::group(cx).when(job.events.is_empty(), |d| d.child(div().py(px(14.)).text_size(px(12.)).text_color(dimmed)
        .child("This job was created before activity history was available. Its last recorded status is shown above."))).children(activity))
      .child(div().pt(px(14.)).text_size(px(11.)).text_color(dimmed).child(format!("Last update {} · Job {}", format::ago(job.updated), job.id))))
}
