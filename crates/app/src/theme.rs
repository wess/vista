//! Vista's look: a graphite neutral ramp with an indigo accent, following the
//! system's light or dark appearance unless the person picks one.

use gpui::{App, Window};
use guise::prelude::*;
use guise::theme::{Color, Shades};
use vista_kit::store::Appearance;

const GRAPHITE: [&str; 10] = [
  "#C9CBD1", "#AEB1B8", "#93969E", "#6B6E76", "#494C54", "#34363C", "#26282D", "#1C1D21",
  "#151619", "#0F1012",
];

pub const LIGHT: &str = "light";
pub const DARK: &str = "dark";

fn base(dark: bool) -> Theme {
  let mut theme = if dark { Theme::dark() } else { Theme::light() };
  theme
    .palette
    .set_shades(ColorName::Dark, Shades(GRAPHITE.map(Color::hex)));
  theme.primary_color = ColorName::Indigo;
  theme.default_radius = Size::Md;
  theme.font_family = ".SystemUIFont".into();
  theme
}

fn choice(appearance: Appearance) -> ThemeChoice {
  match appearance {
    Appearance::System => ThemeChoice::System,
    Appearance::Light => ThemeChoice::Fixed(LIGHT.into()),
    Appearance::Dark => ThemeChoice::Fixed(DARK.into()),
  }
}

pub fn install(appearance: Appearance, cx: &mut App) {
  ThemeManager::new()
    .with(ThemeEntry::new(LIGHT, base(false)).name("Light"))
    .with(ThemeEntry::new(DARK, base(true)).name("Dark"))
    .choice(choice(appearance))
    .install(cx);
}

pub fn apply(appearance: Appearance, cx: &mut App) {
  ThemeManager::set_choice(cx, choice(appearance));
  cx.refresh_windows();
}

pub fn watch(window: &mut Window, cx: &mut App) {
  ThemeManager::watch(window, cx);
}
