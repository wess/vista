//! Vista — virtual machines for Apple silicon.
//!
//! This is the library window and the menu bar item. Machines themselves run
//! in their own `vistamachine` processes, so quitting Vista leaves them
//! running, and the MCP server (`vistamcp`) can drive them with or without
//! this window.

mod actions;
mod runtime;
mod state;
mod statusbar;
mod theme;
mod ui;

use std::cell::RefCell;

use gpui::prelude::*;
use gpui::{
  point, px, size, App, Application, Bounds, TitlebarOptions, WindowBounds, WindowHandle,
  WindowOptions,
};

use crate::actions::{CloseWindow, Fullscreen, Minimize, Zoom};
use crate::ui::root::Root;

thread_local! {
  static MAIN: RefCell<Option<WindowHandle<Root>>> = const { RefCell::new(None) };
}

/// Whether Rosetta for Linux is installed on this Mac.
pub fn rosetta() -> bool {
  std::path::Path::new("/Library/Apple/usr/libexec/oah/libRosettaRuntime").exists()
}

/// Open the library window, or bring it forward.
pub fn open_main(cx: &mut App) {
  cx.activate(true);
  let existing = MAIN.with(|m| *m.borrow());
  if let Some(handle) = existing {
    if handle
      .update(cx, |_, window, _| window.activate_window())
      .is_ok()
    {
      return;
    }
  }
  let dimensions = std::env::var("VISTA_SIZE")
    .ok()
    .and_then(|value| {
      let (width, height) = value.split_once('x')?;
      Some(size(
        px(width.parse::<f32>().ok()?.max(920.)),
        px(height.parse::<f32>().ok()?.max(600.)),
      ))
    })
    .unwrap_or_else(|| size(px(1180.), px(760.)));
  let bounds = Bounds::centered(None, dimensions, cx);
  let options = WindowOptions {
    window_bounds: Some(WindowBounds::Windowed(bounds)),
    window_min_size: Some(size(px(920.), px(600.))),
    titlebar: Some(TitlebarOptions {
      title: Some("Vista".into()),
      appears_transparent: true,
      traffic_light_position: Some(point(px(16.), px(17.))),
    }),
    app_id: Some("io.wess.vista".into()),
    ..Default::default()
  };
  let handle = cx.open_window(options, |window, cx| {
    theme::watch(window, cx);
    let root = cx.new(Root::new);
    let focus = root.read(cx).focus.clone();
    window.focus(&focus);
    window.on_window_should_close(cx, |_, _| {
      MAIN.with(|m| *m.borrow_mut() = None);
      true
    });
    root
  });
  if let Ok(handle) = handle {
    let _ = handle.update(cx, |root, window, cx| {
      let _ = root;
      wire_window(window, cx);
    });
    MAIN.with(|m| *m.borrow_mut() = Some(handle));
  }
}

/// `VISTA_FLOAT=1` keeps the window above others: gpui stops drawing a
/// window that's fully covered, which makes automated screenshots stale.
fn float(window: &gpui::Window) {
  use raw_window_handle::{HasWindowHandle, RawWindowHandle};
  if std::env::var_os("VISTA_FLOAT").is_none() {
    return;
  }
  let Ok(handle) = HasWindowHandle::window_handle(window) else {
    return;
  };
  if let RawWindowHandle::AppKit(handle) = handle.as_raw() {
    let view = handle.ns_view.as_ptr() as *mut objc2::runtime::AnyObject;
    unsafe {
      let native: *mut objc2::runtime::AnyObject = objc2::msg_send![view, window];
      if !native.is_null() {
        let _: () = objc2::msg_send![native, setLevel: 3isize];
      }
    }
  }
}

fn wire_window(window: &mut gpui::Window, cx: &mut App) {
  float(window);
  cx.on_action(|_: &CloseWindow, cx| {
    if let Some(handle) = MAIN.with(|m| m.borrow_mut().take()) {
      let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
  });
  cx.on_action(|_: &Minimize, cx| {
    if let Some(handle) = MAIN.with(|m| *m.borrow()) {
      let _ = handle.update(cx, |_, window, _| window.minimize_window());
    }
  });
  cx.on_action(|_: &Zoom, cx| {
    if let Some(handle) = MAIN.with(|m| *m.borrow()) {
      let _ = handle.update(cx, |_, window, _| window.zoom_window());
    }
  });
  cx.on_action(|_: &Fullscreen, cx| {
    if let Some(handle) = MAIN.with(|m| *m.borrow()) {
      let _ = handle.update(cx, |_, window, _| window.toggle_fullscreen());
    }
  });
}

/// Machines marked "Start with Vista" come up in the background.
fn autostart(cx: &mut App) {
  let library = state::library(cx);
  let Some(store) = library.read(cx).store.clone() else {
    return;
  };
  runtime::spawn(async move {
    for entry in store.machines().await.unwrap_or_default() {
      let Ok(bundle) = vista_kit::bundle::load(&entry.path()) else {
        continue;
      };
      if bundle.config.autostart && vista_kit::ipc::status(&entry.id).await.is_none() {
        let _ = vista_kit::launch::start(&bundle.path, false);
      }
    }
  });
}

fn main() {
  let _ = vista_kit::paths::ensure();
  runtime::get();
  let app = Application::new();
  app.on_reopen(open_main);
  app.run(|cx: &mut App| {
    let library = state::Library::install(cx);
    let settings = library.read(cx).settings.clone();
    theme::install(settings.appearance, cx);
    cx.bind_keys(actions::keys());
    cx.set_menus(actions::menus());
    actions::register(cx);
    statusbar::install(cx);
    statusbar::set_visible(settings.menubar);
    autostart(cx);
    open_main(cx);
    if std::env::var_os("VISTA_PANEL").is_some() {
      statusbar::toggle(cx);
    }
  });
}
