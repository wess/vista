//! The main window: a toolbar across the top, the machine list down the
//! left, and whatever is selected filling the rest.

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, FocusHandle, MouseButton, Window};
use guise::prelude::*;

use super::create::Create;
use super::detail::Detail;
use super::settings::SettingsPage;
use crate::state::{library, Library, Notice, Page};

pub const SIDEBAR: f32 = 248.0;
pub const TOOLBAR: f32 = 48.0;

pub struct Root {
  library: Entity<Library>,
  overlays: Entity<OverlayHost>,
  detail: Option<(String, Entity<Detail>)>,
  create: Option<Entity<Create>>,
  settings: Option<Entity<SettingsPage>>,
  pub focus: FocusHandle,
}

impl Root {
  pub fn new(cx: &mut Context<Self>) -> Self {
    let library = library(cx);
    let overlays = cx.new(OverlayHost::new);
    cx.observe(&library, |_, _, cx| cx.notify()).detach();
    let host = overlays.clone();
    cx.subscribe(&library, move |_, _, notice: &Notice, cx| {
      let notice = notice.clone();
      host.update(cx, |host, cx| match notice {
        Notice::Info(message) => host.toast(message, cx),
        Notice::Failed(message) => host.toast_titled("Something went wrong", message, ColorName::Red, cx),
      });
    })
    .detach();
    Root {
      library,
      overlays,
      detail: None,
      create: None,
      settings: None,
      focus: cx.focus_handle(),
    }
  }

  fn content(&mut self, page: &Page, window: &mut Window, cx: &mut Context<Self>) -> gpui::AnyElement {
    match page {
      Page::Machine(id) => {
        let fresh = !matches!(&self.detail, Some((current, _)) if current == id);
        if fresh {
          let id = id.clone();
          let overlays = self.overlays.clone();
          let detail = cx.new(|cx| Detail::new(id.clone(), overlays, window, cx));
          self.detail = Some((id, detail));
        }
        self.create = None;
        self.detail.as_ref().map(|(_, d)| d.clone().into_any_element()).unwrap_or_else(|| div().into_any_element())
      }
      Page::Create => {
        let create = self
          .create
          .get_or_insert_with(|| cx.new(|cx| Create::new(window, cx)))
          .clone();
        create.into_any_element()
      }
      Page::Settings => {
        self.create = None;
        let settings = self
          .settings
          .get_or_insert_with(|| cx.new(|cx| SettingsPage::new(window, cx)))
          .clone();
        settings.into_any_element()
      }
      Page::Welcome => {
        self.create = None;
        super::welcome::render(cx).into_any_element()
      }
    }
  }
}

impl Render for Root {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let body = t.body().hsla();
    let side = t.surface().hsla();
    let border = t.border().hsla();
    let text = t.text().hsla();
    let dimmed = t.dimmed().hsla();
    let page = self.library.read(cx).page.clone();
    let title: String = match &page {
      Page::Machine(id) => self.library.read(cx).machine(id).map(|m| m.name().to_string()).unwrap_or_default(),
      Page::Create => "New Machine".into(),
      Page::Settings => "Settings".into(),
      Page::Welcome => String::new(),
    };
    let content = self.content(&page, window, cx);
    let lib = self.library.clone();
    let lib2 = self.library.clone();

    let toolbar = div()
      .id("toolbar")
      .flex()
      .flex_row()
      .flex_none()
      .h(px(TOOLBAR))
      .w_full()
      .border_b_1()
      .border_color(border)
      .on_mouse_down(MouseButton::Left, |event, window, _| {
        if event.click_count == 2 {
          window.titlebar_double_click();
        } else {
          window.start_window_move();
        }
      })
      .child(
        div()
          .flex()
          .items_center()
          .w(px(SIDEBAR))
          .h_full()
          .pl(px(84.))
          .bg(side)
          .border_r_1()
          .border_color(border)
          .text_color(dimmed)
          .text_size(px(13.))
          .font_weight(gpui::FontWeight::SEMIBOLD)
          .child("Vista"),
      )
      .child(
        div()
          .flex()
          .flex_1()
          .items_center()
          .justify_between()
          .px(px(16.))
          .bg(body)
          .child(div().text_color(text).text_size(px(14.)).font_weight(gpui::FontWeight::SEMIBOLD).child(title))
          .child(
            Group::new()
              .gap(Size::Xs)
              .child(
                Button::new("new", "New Machine")
                  .size(Size::Sm)
                  .variant(Variant::Light)
                  .left_section(Icon::new(IconName::Plus).size(Size::Sm))
                  .on_click(move |_, _, cx| lib.update(cx, |l, cx| l.select(Page::Create, cx))),
              )
              .child(
                ActionIcon::new("settings", IconName::Settings)
                  .label("Settings")
                  .variant(Variant::Subtle)
                  .on_click(move |_, _, cx| lib2.update(cx, |l, cx| l.select(Page::Settings, cx))),
              ),
          ),
      );

    div()
      .id("root")
      .track_focus(&self.focus)
      .size_full()
      .flex()
      .flex_col()
      .bg(body)
      .text_color(text)
      .font_family(".SystemUIFont")
      .child(toolbar)
      .child(
        div()
          .flex()
          .flex_row()
          .flex_1()
          .min_h(px(0.))
          .child(super::sidebar::render(&self.library, cx))
          .child(div().flex().flex_col().flex_1().min_w(px(0.)).h_full().child(content)),
      )
      .child(self.overlays.clone())
  }
}
