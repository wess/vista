use gpui::prelude::*;
use gpui::{div, px, App, IntoElement};
use guise::prelude::*;

use crate::state::{library, Page};

pub fn render(cx: &App) -> impl IntoElement {
  let t = theme(cx);
  let dimmed = t.dimmed().hsla();
  let lib = library(cx);
  let screen = div()
    .w(px(272.))
    .h(px(172.))
    .flex_none()
    .rounded(px(14.))
    .border_1()
    .border_color(t.border().hsla())
    .bg(t.surface().hsla())
    .shadow_lg()
    .overflow_hidden()
    .child(
      div()
        .flex()
        .items_center()
        .gap(px(5.))
        .h(px(26.))
        .px(px(10.))
        .border_b_1()
        .border_color(t.border().hsla())
        .children(["#FF6058", "#FEBC2E", "#28C840"].map(|c| {
          div()
            .flex_none()
            .size(px(6.))
            .rounded_full()
            .bg(gpui::rgb(u32::from_str_radix(&c[1..], 16).unwrap()))
        })),
    )
    .child(
      div()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(14.))
        .h(px(144.))
        .bg(t.primary().alpha(0.06))
        .child(super::glyph::badge("macos", 54.))
        .child(super::glyph::badge("ubuntu", 54.))
        .child(super::glyph::badge("windows", 54.)),
    );
  div()
    .id("welcome")
    .size_full()
    .min_h(px(0.))
    .overflow_y_scroll()
    .child(
      div().flex().flex_col().items_center().justify_center().min_h_full().p(px(40.)).gap(px(24.))
        .child(screen)
        .child(div().flex().flex_col().items_center().gap(px(10.)).mt(px(8.))
          .child(div().text_size(px(28.)).font_weight(gpui::FontWeight::BOLD).text_center().child("More possibilities. One Mac."))
          .child(div().max_w(px(390.)).text_center().text_size(px(14.)).line_height(px(22.)).text_color(dimmed)
            .child("A fresh macOS workspace. A Linux development box. A place to try something new. Make it yours in a few clicks.")))
        .child(Button::new("welcome-new", "Create a Machine")
          .left_section(Icon::new(IconName::Plus).size(Size::Sm))
          .on_click(move |_, _, cx| lib.update(cx, |l, cx| l.select(Page::Create, cx))))
        .child(div().text_size(px(11.)).text_color(dimmed).child("Built on Apple Virtualization. Everything stays on your Mac.")),
    )
}
