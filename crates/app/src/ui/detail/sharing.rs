//! Sharing: Mac folders inside the guest, clipboard, Rosetta, and whether
//! coding agents may drive this machine.

use gpui::prelude::*;
use gpui::{div, px, App, Context, IntoElement, PathPromptOptions, SharedString, Window};
use guise::prelude::*;
use vista_kit::machine::{Os, Share};

use super::hardware::row;
use super::Detail;
use crate::state::Machine;

fn mount_hint(os: Os) -> &'static str {
  match os {
    Os::Macos => "Shared folders appear in the guest under /Volumes/My Shared Files.",
    Os::Linux => {
      "In the guest: sudo mount -t virtiofs vista /mnt/vista (Vista Tools adds this to /etc/fstab)."
    }
    _ => "Shared folders use virtio-fs; the guest needs a driver for it.",
  }
}

impl Detail {
  fn persist(&mut self, cx: &mut Context<Self>) {
    if let Some(config) = self.merged(cx) {
      self.library.update(cx, |l, cx| l.save_config(config, cx));
    }
  }

  pub fn sharing(
    &mut self,
    _machine: &Machine,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) -> impl IntoElement {
    let t = theme(cx);
    let dimmed = t.dimmed().hsla();
    let border = t.border().hsla();
    let surface = t.surface().hsla();
    let Some(draft) = self.draft.clone() else {
      return div().into_any_element();
    };
    let entity = cx.entity();

    let folders = draft.shares.iter().enumerate().map(|(i, share)| {
      let entity = entity.clone();
      div()
        .flex()
        .items_center()
        .gap(px(10.))
        .px(px(12.))
        .py(px(9.))
        .border_b_1()
        .border_color(border)
        .child(Icon::new(IconName::Folder).size(Size::Sm))
        .child(
          div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.))
            .child(
              div()
                .text_size(px(13.))
                .overflow_hidden()
                .whitespace_normal()
                .line_clamp(1)
                .text_ellipsis()
                .child(share.name.clone()),
            )
            .child(
              div()
                .text_size(px(11.))
                .text_color(dimmed)
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_normal()
                .line_clamp(1)
                .child(share.path.clone()),
            ),
        )
        .when(share.readonly, |d| {
          d.child(
            Badge::new("Read only")
              .size(Size::Xs)
              .variant(Variant::Light),
          )
        })
        .child(
          ActionIcon::new(SharedString::from(format!("unshare-{i}")), IconName::X)
            .label("Stop sharing")
            .size(Size::Sm)
            .variant(Variant::Subtle)
            .on_click(move |_, _, cx| {
              entity.update(cx, |this, cx| {
                if let Some(draft) = &mut this.draft {
                  if i < draft.shares.len() {
                    draft.shares.remove(i);
                  }
                }
                this.persist(cx);
                cx.notify();
              });
            }),
        )
    });

    let add = {
      let entity = entity.clone();
      Button::new("add-share", "Share a Folder…")
        .size(Size::Sm)
        .variant(Variant::Default)
        .left_section(Icon::new(IconName::Plus).size(Size::Sm))
        .on_click(move |_, _, cx: &mut App| {
          let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Share".into()),
          });
          let entity = entity.clone();
          cx.spawn(async move |cx| {
            if let Ok(Ok(Some(paths))) = picked.await {
              if let Some(path) = paths.into_iter().next() {
                let _ = entity.update(cx, |this, cx| {
                  if let Some(draft) = &mut this.draft {
                    let name = path
                      .file_name()
                      .map(|n| n.to_string_lossy().to_string())
                      .unwrap_or_else(|| "Shared".into());
                    draft.shares.retain(|s| s.name != name);
                    draft.shares.push(Share {
                      path: path.to_string_lossy().to_string(),
                      name,
                      readonly: false,
                    });
                  }
                  this.persist(cx);
                  cx.notify();
                });
              }
            }
          })
          .detach();
        })
    };

    let toggle = |id: &'static str, value: bool, set: fn(&mut vista_kit::machine::Config, bool)| {
      let entity = entity.clone();
      Switch::new(id).checked(value).on_change(move |_, _, cx| {
        entity.update(cx, |this, cx| {
          if let Some(draft) = &mut this.draft {
            set(draft, !value);
          }
          this.persist(cx);
          cx.notify();
        });
      })
    };

    div()
      .flex()
      .flex_col()
      .gap(px(8.))
      .w_full()
      .max_w(px(760.))
      .child(crate::ui::layout::heading(
        "Sharing",
        "Bring your files and tools along.",
        cx,
      ))
      .child(
        div()
          .text_size(px(13.))
          .font_weight(gpui::FontWeight::MEDIUM)
          .child("Shared folders"),
      )
      .child(
        div()
          .text_size(px(12.))
          .text_color(dimmed)
          .child(mount_hint(draft.os)),
      )
      .child(
        div()
          .rounded(px(10.))
          .border_1()
          .border_color(border)
          .bg(surface)
          .overflow_hidden()
          .when(draft.shares.is_empty(), |d| {
            d.child(
              div()
                .p(px(14.))
                .text_size(px(12.))
                .text_color(dimmed)
                .child("No folders shared."),
            )
          })
          .children(folders),
      )
      .child(div().pb(px(12.)).child(add))
      .child(
        crate::ui::layout::group(cx)
          .when(draft.os == Os::Linux, |d| {
            d.child(row(
              "Clipboard",
              "Copy and paste with the Mac (needs spice-vdagent in the guest).",
              toggle("clipboard", draft.clipboard, |c, v| c.clipboard = v),
              cx,
            ))
            .child(row(
              "Rosetta",
              "Run x86-64 Linux programs through Rosetta.",
              toggle("rosetta", draft.rosetta, |c, v| c.rosetta = v),
              cx,
            ))
          })
          .child(row(
            "Agent control",
            "Let coding agents start, see and drive this machine through Vista's MCP server.",
            toggle("agents", draft.agents, |c, v| c.agents = v),
            cx,
          )),
      )
      .into_any_element()
  }
}
