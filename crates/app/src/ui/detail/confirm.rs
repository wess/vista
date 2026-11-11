//! A confirm-or-cancel dialog over the window.

use std::rc::Rc;

use gpui::{App, Entity, IntoElement, Window};
use guise::prelude::*;

pub fn ask(
  overlays: &Entity<OverlayHost>,
  title: &str,
  message: &str,
  action: &str,
  window: &mut Window,
  cx: &mut App,
  then: impl Fn(&mut App) + 'static,
) {
  let title = title.to_string();
  let message = message.to_string();
  let action = action.to_string();
  let then = Rc::new(then);
  overlays.update(cx, |host, cx| {
    host.open_modal(window, cx, move |close, _window, _cx| {
      let confirm = close.clone();
      let then = then.clone();
      ConfirmModal::new()
        .title(title.clone())
        .message(message.clone())
        .confirm_label(action.clone())
        .danger()
        .on_confirm(move |_, window, cx| {
          confirm(window, cx);
          then(cx);
        })
        .on_cancel(move |_, window, cx| close(window, cx))
        .into_any_element()
    });
  });
}
