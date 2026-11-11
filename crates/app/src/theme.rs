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
  theme.primary_color = ColorName::Blue;
  theme = if dark {
    theme
      .with_body(color!("#1C1C1E"))
      .with_surface(color!("#272729"))
      .with_surface_hover(color!("#333336"))
      .with_border(color!("#3A3A3C"))
      .with_text(color!("#F5F5F7"))
      .with_dimmed(color!("#ABABB0"))
      .with_primary(color!("#0A84FF"))
  } else {
    theme
      .with_body(color!("#F5F5F7"))
      .with_surface(color!("#FFFFFF"))
      .with_surface_hover(color!("#EAEAEE"))
      .with_border(color!("#DEDEE3"))
      .with_text(color!("#1D1D1F"))
      .with_dimmed(color!("#6E6E73"))
      .with_primary(color!("#007AFF"))
  };
  theme.line_height = 1.4;
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
