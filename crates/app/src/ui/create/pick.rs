//! Step one: which system.

use gpui::prelude::*;
use gpui::{div, px, App, Context, IntoElement, PathPromptOptions, SharedString, Window};
use guise::prelude::*;
use vista_kit::catalog::{self, Entry};
use vista_kit::machine::Os;

use super::Create;
use crate::ui::glyph;

/// The Linux and BSD families, in the order they're offered.
const FAMILIES: [&str; 14] = [
  "ubuntu", "fedora", "debian", "kali", "centos", "rocky", "alma", "opensuse", "popos", "nixos", "alpine",
  "freebsd", "openbsd", "netbsd",
];

fn card(id: String, family: &str, title: String, subtitle: String, selected: bool, cx: &App) -> gpui::Stateful<gpui::Div> {
  let t = theme(cx);
  let border = if selected { t.primary().hsla() } else { t.border().hsla() };
  let surface = t.surface().hsla();
  let hover = t.surface_hover().hsla();
  let dimmed = t.dimmed().hsla();
  div()
    .id(SharedString::from(id))
    .flex()
    .items_center()
    .gap(px(12.))
    .w(px(214.))
    .p(px(12.))
    .rounded(px(12.))
    .border_1()
    .border_color(border)
    .bg(surface)
    .hover(move |s| s.bg(hover))
    .cursor_pointer()
    .child(glyph::badge(family, 36.))
    .child(
      div()
        .flex()
        .flex_col()
        .min_w(px(0.))
        .child(div().text_size(px(13.)).font_weight(gpui::FontWeight::MEDIUM).child(title))
        .child(div().text_size(px(11.)).text_color(dimmed).overflow_hidden().whitespace_nowrap().text_ellipsis().child(subtitle)),
    )
}

fn section(title: &str, cx: &App) -> impl IntoElement {
  div()
    .pt(px(22.))
    .pb(px(10.))
    .text_size(px(12.))
    .font_weight(gpui::FontWeight::SEMIBOLD)
    .text_color(theme(cx).dimmed().hsla())
    .child(title.to_uppercase())
}

impl Create {
  pub fn pick(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let all = catalog::all();
    let entity = cx.entity();
    let dimmed = theme(cx).dimmed().hsla();

    let choose = |entry: Entry| {
      let entity = entity.clone();
      move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut App| {
        entity.update(cx, |this, cx| this.choose(Some(entry), None, cx));
      }
    };

    let macs = all.iter().filter(|e| e.os == Os::Macos).map(|entry| {
      let title = if entry.version.is_empty() { "macOS".to_string() } else { format!("macOS {}", entry.version) };
      let subtitle = if entry.version.is_empty() { "Latest from Apple".to_string() } else { "From Apple".to_string() };
      card(format!("pick-{}", entry.id), "macos", title, subtitle, false, cx).on_click(choose(*entry))
    });

    let windows = all.iter().filter(|e| e.os == Os::Windows).map(|entry| {
      card(format!("pick-{}", entry.id), "windows", entry.title(), "Experimental".into(), false, cx)
        .on_click(choose(*entry))
    });

    let open = self.family;
    let families = FAMILIES.iter().filter_map(|family| {
      let editions: Vec<Entry> = all.iter().filter(|e| e.family == *family).copied().collect();
      let first = *editions.first()?;
      let name = first.name.to_string();
      let subtitle = if editions.len() == 1 {
        first.edition.to_string()
      } else {
        format!("{} editions", editions.len())
      };
      let entity = entity.clone();
      let family = *family;
      Some(
        card(format!("family-{family}"), family, name, subtitle, open == Some(family), cx).on_click(move |_, _, cx| {
          entity.update(cx, |this, cx| {
            if editions.len() == 1 {
              this.choose(Some(editions[0]), None, cx);
            } else {
              this.family = if this.family == Some(family) { None } else { Some(family) };
              cx.notify();
            }
          });
        }),
      )
    });

    let editions = open.map(|family| {
      let list: Vec<Entry> = all.iter().filter(|e| e.family == family).copied().collect();
      div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .mt(px(12.))
        .p(px(14.))
        .rounded(px(12.))
        .bg(theme(cx).surface().hsla())
        .border_1()
        .border_color(theme(cx).border().hsla())
        .child(div().text_size(px(12.)).text_color(dimmed).child("Choose an edition"))
        .children(list.into_iter().map(|entry| {
          let label = match (entry.version, entry.edition) {
            ("", edition) => edition.to_string(),
            (version, edition) => format!("{version} · {edition}"),
          };
          div()
            .id(SharedString::from(format!("edition-{}", entry.id)))
            .flex()
            .flex_col()
            .px(px(10.))
            .py(px(8.))
            .rounded(px(8.))
            .cursor_pointer()
            .hover(|s| s.bg(theme(cx).surface_hover().hsla()))
            .on_click(choose(entry))
            .child(div().text_size(px(13.)).font_weight(gpui::FontWeight::MEDIUM).child(label))
            .child(div().text_size(px(12.)).text_color(dimmed).child(entry.notes))
        }))
    });

    let file = {
      let entity = entity.clone();
      Button::new("pick-file", "Use an Installer on This Mac…")
        .variant(Variant::Default)
        .left_section(Icon::new(IconName::Disc3).size(Size::Sm))
        .on_click(move |_, _, cx| {
          let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Use".into()),
          });
          let entity = entity.clone();
          cx.spawn(async move |cx| {
            if let Ok(Ok(Some(paths))) = picked.await {
              if let Some(path) = paths.into_iter().next() {
                let _ = entity.update(cx, |this, cx| this.choose(None, Some(path), cx));
              }
            }
          })
          .detach();
        })
    };

    div()
      .flex()
      .flex_col()
      .child(Title::new("Choose a system").order(3))
      .child(
        div()
          .pt(px(4.))
          .text_size(px(13.))
          .text_color(dimmed)
          .child("Vista downloads the installer from its publisher, checks it, and keeps it for next time."),
      )
      .child(section("macOS", cx))
      .child(div().flex().flex_wrap().gap(px(12.)).children(macs))
      .child(section("Windows", cx))
      .child(div().flex().flex_wrap().gap(px(12.)).children(windows))
      .child(section("Linux and BSD", cx))
      .child(div().flex().flex_wrap().gap(px(12.)).children(families))
      .children(editions)
      .child(section("Something else", cx))
      .child(
        div()
          .flex()
          .items_center()
          .gap(px(12.))
          .child(file)
          .child(div().text_size(px(12.)).text_color(dimmed).child("Any Arm64 ISO, or a macOS restore image (.ipsw).")),
      )
  }
}
