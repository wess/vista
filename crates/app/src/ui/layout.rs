use gpui::{div, px, App, Div, FontWeight, IntoElement, ParentElement, Styled};
use guise::prelude::*;

pub fn heading(title: &str, subtitle: &str, cx: &App) -> impl IntoElement {
  div()
    .flex()
    .flex_col()
    .flex_none()
    .gap(px(7.))
    .mb(px(28.))
    .child(
      div()
        .text_size(px(28.))
        .font_weight(FontWeight::BOLD)
        .child(title.to_string()),
    )
    .child(
      div()
        .text_size(px(13.))
        .line_height(px(20.))
        .text_color(theme(cx).dimmed().hsla())
        .child(subtitle.to_string()),
    )
}

pub fn group(cx: &App) -> Div {
  let t = theme(cx);
  div()
    .flex()
    .flex_col()
    .flex_none()
    .w_full()
    .min_w(px(0.))
    .px(px(18.))
    .rounded(px(12.))
    .border_1()
    .border_color(t.border().hsla())
    .bg(t.surface().hsla())
}

pub fn section(title: &str, cx: &App) -> impl IntoElement {
  div()
    .flex_none()
    .pt(px(24.))
    .pb(px(10.))
    .text_size(px(13.))
    .font_weight(FontWeight::SEMIBOLD)
    .text_color(theme(cx).text().hsla())
    .child(title.to_string())
}
