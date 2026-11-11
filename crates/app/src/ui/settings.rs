//! Settings: how Vista looks, where machines live, downloads, and agents.

use gpui::prelude::*;
use gpui::{div, px, App, ClipboardItem, Context, Entity, IntoElement, PathPromptOptions, Window};
use guise::prelude::*;
use vista_kit::store::{Appearance, Settings};

use crate::state::{library, Library};
use crate::ui::detail::row;
use crate::ui::format;

pub struct SettingsPage {
  library: Entity<Library>,
  downloads: u64,
}

/// `vistamcp`, next to this executable.
pub fn mcp_path() -> String {
  std::env::current_exe()
    .ok()
    .and_then(|exe| exe.parent().map(|d| d.join("vistamcp")))
    .map(|p| p.to_string_lossy().to_string())
    .unwrap_or_else(|| "vistamcp".into())
}

fn chips(options: &[(&str, bool)], on_pick: impl Fn(usize, &mut App) + 'static, cx: &App) -> impl IntoElement {
  let options: Vec<(String, bool)> = options.iter().map(|(l, s)| (l.to_string(), *s)).collect();
  crate::ui::detail::chips("settings-chips", options, move |i, _, cx| on_pick(i, cx), cx)
}

impl SettingsPage {
  pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
    let library = library(cx);
    cx.observe(&library, |_, _, cx| cx.notify()).detach();
    SettingsPage {
      library,
      downloads: vista_kit::bundle::usage(&vista_kit::paths::images()),
    }
  }

  fn update(&self, cx: &mut App, change: impl FnOnce(&mut Settings)) {
    self.library.update(cx, |l, cx| {
      let mut settings = l.settings.clone();
      change(&mut settings);
      l.save_settings(settings, cx);
    });
  }
}

fn heading(title: &str, cx: &App) -> impl IntoElement {
  div()
    .pt(px(24.))
    .pb(px(4.))
    .text_size(px(12.))
    .font_weight(gpui::FontWeight::SEMIBOLD)
    .text_color(theme(cx).dimmed().hsla())
    .child(title.to_uppercase())
}

impl Render for SettingsPage {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let dimmed = t.dimmed().hsla();
    let surface = t.surface().hsla();
    let border = t.border().hsla();
    let settings = self.library.read(cx).settings.clone();
    let entity = cx.entity();
    let mcp = mcp_path();
    let command = mcp.clone();
    let json = format!("{{\n  \"mcpServers\": {{\n    \"vista\": {{ \"command\": \"{mcp}\" }}\n  }}\n}}");

    let set = |change: fn(&mut Settings, bool), value: bool| {
      let entity = entity.clone();
      move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut App| {
        entity.update(cx, |this, cx| {
          this.update(cx, |s| change(s, !value));
        });
      }
    };

    let appearance = {
      let entity = entity.clone();
      chips(
        &[
          ("System", settings.appearance == Appearance::System),
          ("Light", settings.appearance == Appearance::Light),
          ("Dark", settings.appearance == Appearance::Dark),
        ],
        move |i, cx| {
          entity.update(cx, |this, cx| {
            this.update(cx, |s| {
              s.appearance = [Appearance::System, Appearance::Light, Appearance::Dark][i];
            })
          });
        },
        cx,
      )
    };

    let location = {
      let entity = entity.clone();
      Button::new("location", "Change…").size(Size::Sm).variant(Variant::Default).on_click(move |_, _, cx| {
        let picked = cx.prompt_for_paths(PathPromptOptions {
          files: false,
          directories: true,
          multiple: false,
          prompt: Some("Use".into()),
        });
        let entity = entity.clone();
        cx.spawn(async move |cx| {
          if let Ok(Ok(Some(paths))) = picked.await {
            if let Some(path) = paths.into_iter().next() {
              let _ = entity.update(cx, |this, cx| {
                this.update(cx, |s| s.location = path.to_string_lossy().to_string());
              });
            }
          }
        })
        .detach();
      })
    };

    let clear = {
      let entity = entity.clone();
      Button::new("clear-downloads", "Delete Downloads")
        .size(Size::Sm)
        .variant(Variant::Default)
        .disabled(self.downloads == 0)
        .on_click(move |_, _, cx| {
          let store = entity.read(cx).library.read(cx).store.clone();
          crate::runtime::spawn(async move {
            if let Some(store) = store {
              for image in store.images().await.unwrap_or_default() {
                let _ = store.remove_image(&image).await;
              }
            }
            if let Ok(entries) = std::fs::read_dir(vista_kit::paths::images()) {
              for entry in entries.flatten() {
                let _ = std::fs::remove_file(entry.path());
              }
            }
          });
          entity.update(cx, |this, cx| {
            this.downloads = 0;
            cx.notify();
          });
        })
    };

    let copy_command = {
      let command = command.clone();
      Button::new("copy-mcp", "Copy Path")
        .size(Size::Sm)
        .variant(Variant::Default)
        .left_section(Icon::new(IconName::Copy).size(Size::Sm))
        .on_click(move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(command.clone())))
    };
    let copy_json = {
      let json = json.clone();
      Button::new("copy-json", "Copy JSON")
        .size(Size::Sm)
        .variant(Variant::Default)
        .left_section(Icon::new(IconName::Copy).size(Size::Sm))
        .on_click(move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(json.clone())))
    };
    let code = |text: String| {
      div()
        .p(px(12.))
        .rounded(px(8.))
        .bg(surface)
        .border_1()
        .border_color(border)
        .font_family("Menlo")
        .text_size(px(12.))
        .child(text)
    };

    div()
      .id("settings")
      .size_full()
      .overflow_y_scroll()
      .child(
        div()
          .p(px(28.))
          .max_w(px(720.))
          .flex()
          .flex_col()
          .child(heading("General", cx))
          .child(row("Appearance", "", appearance, cx))
          .child(row(
            "Show in menu bar",
            "Quick view of every machine, with start and stop, from the menu bar.",
            Switch::new("menubar").checked(settings.menubar).on_change(set(|s, v| s.menubar = v, settings.menubar)),
            cx,
          ))
          .child(heading("Machines", cx))
          .child(row("Location", &format::tilde(&settings.location), location, cx))
          .child(row(
            "Share Downloads with new machines",
            "",
            Switch::new("sharedownloads")
              .checked(settings.sharedownloads)
              .on_change(set(|s, v| s.sharedownloads = v, settings.sharedownloads)),
            cx,
          ))
          .child(heading("Downloads", cx))
          .child(row(
            "Keep installers",
            "Reuse a downloaded ISO or restore image for the next machine of the same system.",
            Switch::new("keepinstallers")
              .checked(settings.keepinstallers)
              .on_change(set(|s, v| s.keepinstallers = v, settings.keepinstallers)),
            cx,
          ))
          .child(row(
            "Downloaded installers",
            &format!("{} in {}", format::bytes(self.downloads), format::tilde(&vista_kit::paths::images().to_string_lossy())),
            clear,
            cx,
          ))
          .child(heading("Agents", cx))
          .child(row(
            "Let coding agents manage machines",
            "Vista's MCP server lets agents create, run, see and drive machines, and run commands inside them.",
            Switch::new("agents").checked(settings.agents).on_change(set(|s, v| s.agents = v, settings.agents)),
            cx,
          ))
          .child(div().pt(px(12.)).text_size(px(12.)).text_color(dimmed).child("The MCP server is a stdio command. Point your agent at it:"))
          .child(div().pt(px(6.)).child(code(command)))
          .child(div().pt(px(12.)).text_size(px(12.)).text_color(dimmed).child("Or add this to the agent's MCP configuration:"))
          .child(div().pt(px(6.)).child(code(json)))
          .child(div().flex().gap(px(8.)).pt(px(10.)).child(copy_command).child(copy_json))
          .child(heading("About", cx))
          .child(row(
            "Vista",
            &format!("Version {} · Apache 2.0 licensed · no accounts, no tracking", env!("CARGO_PKG_VERSION")),
            Button::new("github", "GitHub")
              .size(Size::Sm)
              .variant(Variant::Default)
              .left_section(Icon::new(IconName::ExternalLink).size(Size::Sm))
              .on_click(|_, _, cx| cx.open_url("https://github.com/wess/vista")),
            cx,
          )),
      )
  }
}
