//! A machine's badge: a rounded tile in its system's color with a monogram,
//! so a list of machines reads at a glance without anyone's logo.

use gpui::{div, px, Hsla, IntoElement, ParentElement, Styled};
use guise::prelude::*;

pub fn tint(family: &str) -> Hsla {
  match family {
    "macos" => color!("#5E6470"),
    "windows" => color!("#0F6CBD"),
    "ubuntu" => color!("#E95420"),
    "fedora" => color!("#3C6EB4"),
    "debian" => color!("#C70036"),
    "kali" => color!("#367BF0"),
    "centos" => color!("#932279"),
    "rocky" => color!("#10B981"),
    "alma" | "almalinux" => color!("#0F4266"),
    "opensuse" => color!("#73BA25"),
    "alpine" => color!("#0D597F"),
    "nixos" => color!("#5277C3"),
    "popos" => color!("#48B9C7"),
    "freebsd" => color!("#AB2B28"),
    "openbsd" => color!("#D4A72C"),
    "netbsd" => color!("#F26711"),
    _ => color!("#6741D9"),
  }
}

pub fn monogram(family: &str) -> &'static str {
  match family {
    "macos" => "",
    "windows" => "W",
    "ubuntu" => "U",
    "fedora" => "F",
    "debian" => "D",
    "kali" => "K",
    "centos" => "C",
    "rocky" => "R",
    "alma" | "almalinux" => "A",
    "opensuse" => "S",
    "alpine" => "Al",
    "nixos" => "N",
    "popos" => "P",
    "freebsd" => "Fb",
    "openbsd" => "Ob",
    "netbsd" => "Nb",
    _ => "L",
  }
}

/// The badge, `size` points square.
pub fn badge(family: &str, size: f32) -> impl IntoElement {
  let mark = monogram(family);
  let tile = div()
    .flex()
    .items_center()
    .justify_center()
    .flex_none()
    .size(px(size))
    .rounded(px(size * 0.26))
    .bg(tint(family))
    .text_color(color!("#ffffff"))
    .text_size(px(size * 0.42))
    .font_weight(gpui::FontWeight::SEMIBOLD);
  if family == "macos" {
    tile.child(Icon::new(IconName::Apple).size(if size > 30.0 { Size::Md } else { Size::Sm }))
  } else if family == "windows" {
    tile.child(Icon::new(IconName::AppWindow).size(if size > 30.0 { Size::Md } else { Size::Sm }))
  } else {
    tile.child(mark)
  }
}

/// A dot in the color of a machine state.
pub fn state_color(state: vista_kit::machine::State) -> Hsla {
  use vista_kit::machine::State::*;
  match state {
    Running => color!("#40C057"),
    Paused | Suspended => color!("#FAB005"),
    Error => color!("#FA5252"),
    Stopped => color!("#868E96"),
    _ => color!("#4C6EF5"),
  }
}

pub fn dot(state: vista_kit::machine::State) -> impl IntoElement {
  div().size(px(8.)).rounded_full().bg(state_color(state)).flex_none()
}
