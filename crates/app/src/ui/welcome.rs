//! What an empty library looks like: a short welcome and the way in.

use gpui::prelude::*;
use gpui::{div, px, App, IntoElement};
use guise::prelude::*;

use crate::state::{library, Page};

pub fn render(cx: &App) -> impl IntoElement {
  let t = theme(cx);
  let dimmed = t.dimmed().hsla();
  let lib = library(cx);
  let families = ["macos", "windows", "ubuntu", "fedora", "debian", "freebsd"];
  div()
    .flex()
    .flex_col()
    .items_center()
    .justify_center()
    .size_full()
    .gap(px(18.))
    .child(
      div()
        .flex()
        .gap(px(8.))
        .children(families.iter().map(|f| super::glyph::badge(f, 40.))),
    )
    .child(Title::new("Your Mac, many machines").order(2))
    .child(
      div()
        .max_w(px(460.))
        .text_center()
        .text_size(px(14.))
        .text_color(dimmed)
        .child("Vista downloads and installs macOS, Windows 11 and Linux for you, then runs them side by side on Apple's own virtualization."),
    )
    .child(
      Button::new("welcome-new", "Create a Machine")
        .left_section(Icon::new(IconName::Plus).size(Size::Sm))
        .on_click(move |_, _, cx| lib.update(cx, |l, cx| l.select(Page::Create, cx))),
    )
}
