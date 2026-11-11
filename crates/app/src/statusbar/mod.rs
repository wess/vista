//! The menu bar item. A click opens a small panel — drawn with gpui like the
//! rest of Vista — listing every machine with its state and a start or stop
//! button.
//!
//! gpui has no status-item API, so the item itself is AppKit through objc2.
//! Its click is forwarded over a channel to a gpui task, which owns the panel.

mod panel;

use std::cell::RefCell;

use gpui::{px, size, App, AppContext, Bounds, Point, WindowBounds, WindowHandle, WindowKind, WindowOptions};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{define_class, msg_send, sel, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSImage, NSStatusBar, NSStatusItem, NSVariableStatusItemLength};
use objc2_foundation::NSString;
use tokio::sync::mpsc;

pub use panel::Panel;

const WIDTH: f32 = 340.0;

thread_local! {
  static ITEM: RefCell<Option<Retained<NSStatusItem>>> = const { RefCell::new(None) };
  static TARGET: RefCell<Option<Retained<Target>>> = const { RefCell::new(None) };
  static CLICKS: RefCell<Option<mpsc::UnboundedSender<()>>> = const { RefCell::new(None) };
  static OPEN: RefCell<Option<WindowHandle<Panel>>> = const { RefCell::new(None) };
}

define_class!(
  // SAFETY: NSObject has no subclassing requirements; Target has no Drop.
  #[unsafe(super(NSObject))]
  #[thread_kind = MainThreadOnly]
  #[name = "VistaStatusTarget"]
  struct Target;

  impl Target {
    #[unsafe(method(clicked:))]
    fn clicked(&self, _sender: Option<&AnyObject>) {
      CLICKS.with(|c| {
        if let Some(tx) = c.borrow().as_ref() {
          let _ = tx.send(());
        }
      });
    }
  }
);

impl Target {
  fn new(mtm: MainThreadMarker) -> Retained<Self> {
    let this = Self::alloc(mtm).set_ivars(());
    unsafe { msg_send![super(this), init] }
  }
}

/// Where the item sits, in gpui's coordinates (origin top-left of the main
/// display): the panel hangs below it, centred.
fn anchor() -> Option<Point<gpui::Pixels>> {
  ITEM.with(|item| {
    let item = item.borrow();
    let mtm = MainThreadMarker::new()?;
    let button = item.as_ref()?.button(mtm)?;
    let window = button.window()?;
    let frame = window.frame();
    let screen = window.screen()?;
    let screen_frame = screen.frame();
    let x = frame.origin.x - screen_frame.origin.x + frame.size.width / 2.0 - WIDTH as f64 / 2.0;
    let x = x.clamp(8.0, screen_frame.size.width - WIDTH as f64 - 8.0);
    let y = screen_frame.origin.y + screen_frame.size.height - frame.origin.y + 6.0;
    Some(Point::new(px(x as f32), px(y as f32)))
  })
}

pub fn install(cx: &mut App) {
  let Some(mtm) = MainThreadMarker::new() else { return };
  let (tx, mut rx) = mpsc::unbounded_channel();
  CLICKS.with(|c| *c.borrow_mut() = Some(tx));
  let target = Target::new(mtm);
  unsafe {
    let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
    if let Some(button) = item.button(mtm) {
      let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
        &NSString::from_str("square.stack.3d.up.fill"),
        Some(&NSString::from_str("Vista")),
      );
      if let Some(image) = image {
        image.setTemplate(true);
        button.setImage(Some(&image));
      }
      let target_object: &AnyObject = target.as_ref();
      button.setTarget(Some(target_object));
      button.setAction(Some(sel!(clicked:)));
      button.setToolTip(Some(&NSString::from_str("Vista")));
    }
    ITEM.with(|i| *i.borrow_mut() = Some(item));
  }
  TARGET.with(|t| *t.borrow_mut() = Some(target));

  cx.spawn(async move |cx| {
    while rx.recv().await.is_some() {
      let _ = cx.update(toggle);
    }
  })
  .detach();
}

pub fn set_visible(visible: bool) {
  ITEM.with(|item| {
    if let Some(item) = item.borrow().as_ref() {
      item.setVisible(visible);
    }
  });
}

pub fn close(cx: &mut App) {
  if let Some(handle) = OPEN.with(|o| o.borrow_mut().take()) {
    let _ = handle.update(cx, |_, window, _| window.remove_window());
  }
}

pub fn toggle(cx: &mut App) {
  if OPEN.with(|o| o.borrow().is_some()) {
    return close(cx);
  }
  let rows = crate::state::library(cx).read(cx).machines.len().max(1) as f32;
  let height = (118.0 + rows.min(8.0) * 52.0).min(560.0);
  let origin = anchor().unwrap_or_else(|| Point::new(px(1200.), px(30.)));
  let options = WindowOptions {
    window_bounds: Some(WindowBounds::Windowed(Bounds::new(origin, size(px(WIDTH), px(height))))),
    titlebar: None,
    focus: true,
    show: true,
    kind: WindowKind::PopUp,
    is_movable: false,
    is_resizable: false,
    is_minimizable: false,
    ..Default::default()
  };
  if let Ok(handle) = cx.open_window(options, |window, cx| cx.new(|cx| Panel::new(window, cx))) {
    OPEN.with(|o| *o.borrow_mut() = Some(handle));
  }
}

/// Forget the panel when it closes itself.
pub fn closed() {
  OPEN.with(|o| *o.borrow_mut() = None);
}
