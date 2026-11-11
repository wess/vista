//! Step two: name it, size it, create it.

use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement, Window};
use guise::prelude::*;
use vista_kit::jobs::Spec;
use vista_kit::machine::{Account, Os};

use super::{Create, Step};
use crate::ui::glyph;
use crate::ui::detail::row;

impl Create {
  fn submit(&mut self, cx: &mut Context<Self>) {
    let os = self.os();
    let username = self.username.read(cx).text();
    let password = self.password.read(cx).text();
    let account = (matches!(os, Os::Macos | Os::Windows) && !username.trim().is_empty() && !password.is_empty()).then(|| Account {
      fullname: username.trim().to_string(),
      username: username.trim().to_string(),
      password,
      autologin: true,
      ssh: self.ssh,
    });
    let spec = Spec {
      name: self.name.read(cx).text().trim().to_string(),
      system: self.entry.map(|e| e.id.to_string()),
      file: self.file.as_ref().map(|f| f.to_string_lossy().to_string()),
      os: Some(os),
      cpus: self.cpus.read(cx).value_f64().map(|v| v as u32),
      memory: self.memory.read(cx).value_f64().map(|v| (v * 1024.0) as u64),
      disk: self.disk.read(cx).value_f64().map(|v| v as u64),
      account,
      sharedownloads: self.share,
      rosetta: self.rosetta,
      start: self.start,
      visible: true,
      location: None,
    };
    self.library.update(cx, |l, cx| l.create(spec, cx));
    self.step = Step::Pick;
    self.family = None;
  }

  pub fn configure(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let dimmed = t.dimmed().hsla();
    let os = self.os();
    let (family, title, notes) = match (&self.entry, &self.file) {
      (Some(entry), _) => (entry.family.to_string(), entry.title(), entry.notes.to_string()),
      (None, Some(file)) => (
        match os {
          Os::Macos => "macos".into(),
          Os::Windows => "windows".into(),
          _ => "linux".into(),
        },
        file.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(),
        file.to_string_lossy().to_string(),
      ),
      _ => ("linux".into(), String::new(), String::new()),
    };
    let entity = cx.entity();
    let toggle = |id: &'static str, value: bool, set: fn(&mut Create, bool)| {
      let entity = entity.clone();
      Switch::new(id).checked(value).on_change(move |_, _, cx| {
        entity.update(cx, |this, cx| {
          set(this, !value);
          cx.notify();
        });
      })
    };
    let provisioning = os == Os::Macos;
    let account_hint = match os {
      Os::Macos => "On macOS 27, Vista creates this account and skips Setup Assistant. Leave the password empty to set up by hand.",
      Os::Windows => "Windows installs unattended and signs in to this local account.",
      _ => "",
    };
    let back = {
      let entity = entity.clone();
      Button::new("back", "Back")
        .variant(Variant::Default)
        .on_click(move |_, _, cx| entity.update(cx, |this, cx| {
          this.step = Step::Pick;
          cx.notify();
        }))
    };
    let create = {
      let entity = entity.clone();
      Button::new("create", "Create Machine")
        .left_section(Icon::new(IconName::Sparkles).size(Size::Sm))
        .on_click(move |_, _, cx| entity.update(cx, |this, cx| this.submit(cx)))
    };

    div()
      .flex()
      .flex_col()
      .gap(px(14.))
      .max_w(px(640.))
      .child(
        div()
          .flex()
          .items_center()
          .gap(px(14.))
          .child(glyph::badge(&family, 48.))
          .child(
            div()
              .flex()
              .flex_col()
              .gap(px(2.))
              .child(div().text_size(px(18.)).font_weight(gpui::FontWeight::SEMIBOLD).child(title))
              .child(div().text_size(px(12.)).text_color(dimmed).child(notes)),
          ),
      )
      .when(os == Os::Windows, |d| {
        d.child(
          Alert::new("Windows on Apple's framework relies on unpublished switches and a custom install disk. It may not boot on every Mac.")
            .title("Experimental")
            .color(ColorName::Yellow),
        )
      })
      .child(self.name.clone())
      .child(
        div()
          .flex()
          .gap(px(12.))
          .child(div().flex_1().child(self.cpus.clone()))
          .child(div().flex_1().child(self.memory.clone()))
          .child(div().flex_1().child(self.disk.clone())),
      )
      .when(matches!(os, Os::Macos | Os::Windows), |d| {
        d.child(div().pt(px(6.)).text_size(px(13.)).font_weight(gpui::FontWeight::MEDIUM).child("First account"))
          .child(div().text_size(px(12.)).text_color(dimmed).child(account_hint))
          .child(
            div()
              .flex()
              .gap(px(12.))
              .child(div().flex_1().child(self.username.clone()))
              .child(div().flex_1().child(self.password.clone())),
          )
      })
      .child(
        div()
          .flex()
          .flex_col()
          .child(row("Share Downloads", "The Mac's Downloads folder appears inside the guest.", toggle("share", self.share, |c, v| c.share = v), cx))
          .when(os == Os::Linux, |d| {
            d.child(row("Rosetta", "Run x86-64 Linux programs.", toggle("rosetta", self.rosetta, |c, v| c.rosetta = v), cx))
          })
          .when(provisioning, |d| {
            d.child(row("Remote Login", "Turn on SSH for the first account.", toggle("ssh", self.ssh, |c, v| c.ssh = v), cx))
          })
          .child(row("Start when ready", "Boot the installer as soon as the download finishes.", toggle("start", self.start, |c, v| c.start = v), cx)),
      )
      .child(div().flex().gap(px(8.)).pt(px(8.)).child(back).child(create))
  }
}
