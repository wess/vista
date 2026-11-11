//! The machine's window: the guest's display, sized to it, remembered between
//! runs. While macOS installs, a small progress window stands in.

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::MainThreadOnly;
use objc2_app_kit::{
  NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSProgressIndicator,
  NSProgressIndicatorStyle, NSScreen, NSTextField, NSView, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use objc2_virtualization::{VZVirtualMachine, VZVirtualMachineView};

use crate::state::Runner;

fn frame(runner: &Runner) -> NSRect {
  let display = runner.bundle.config.display;
  // Retina: two pixels per point.
  let mut width = display.width as f64 / 2.0;
  let mut height = display.height as f64 / 2.0;
  if let Some(screen) = NSScreen::mainScreen(runner.mtm) {
    let visible = screen.visibleFrame();
    let scale = (visible.size.width * 0.9 / width)
      .min(visible.size.height * 0.9 / height)
      .min(1.0);
    width *= scale;
    height *= scale;
  }
  NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width.max(640.0), height.max(400.0)))
}

fn window(runner: &Runner, rect: NSRect, resizable: bool) -> Retained<NSWindow> {
  let mut style =
    NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Miniaturizable;
  if resizable {
    style |= NSWindowStyleMask::Resizable;
  }
  let window = unsafe {
    NSWindow::initWithContentRect_styleMask_backing_defer(
      NSWindow::alloc(runner.mtm),
      rect,
      style,
      NSBackingStoreType::Buffered,
      false,
    )
  };
  unsafe { window.setReleasedWhenClosed(false) };
  window.setTitle(&NSString::from_str(&runner.bundle.config.name));
  window.setDelegate(Some(ProtocolObject::from_ref(&*runner.delegate)));
  window
}

/// Give the machine a view, creating the window the first time.
pub fn attach(runner: &mut Runner, machine: &VZVirtualMachine) {
  if runner.view.is_none() {
    let rect = frame(runner);
    let window = window(runner, rect, true);
    let view = unsafe { VZVirtualMachineView::new(runner.mtm) };
    unsafe {
      view.setCapturesSystemKeys(true);
      view.setAutomaticallyReconfiguresDisplay(true);
    }
    window.setContentView(Some(&view));
    window.center();
    let name = NSString::from_str(&format!("vista.{}", runner.bundle.config.id));
    let _ = window.setFrameAutosaveName(&name);
    runner.view = Some(view);
    runner.window = Some(window);
  }
  if let Some(view) = &runner.view {
    unsafe { view.setVirtualMachine(Some(machine)) };
  }
}

pub fn show(runner: &mut Runner) {
  let app = NSApplication::sharedApplication(runner.mtm);
  app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
  if let Some(window) = &runner.window {
    window.makeKeyAndOrderFront(None);
    if let Some(view) = &runner.view {
      window.makeFirstResponder(Some(view));
    }
  }
  #[allow(deprecated)]
  app.activateIgnoringOtherApps(true);
  runner.visible = true;
}

pub fn hide(runner: &mut Runner) {
  if let Some(window) = &runner.window {
    window.orderOut(None);
  }
  let app = NSApplication::sharedApplication(runner.mtm);
  app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
  runner.visible = false;
}

/// The install window: a label and a bar.
pub fn installing(runner: &mut Runner) {
  let mtm = runner.mtm;
  let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(420.0, 120.0));
  let window = window(runner, rect, false);
  let content = NSView::initWithFrame(NSView::alloc(mtm), rect);

  let label = NSTextField::labelWithString(&NSString::from_str("Installing macOS…"), mtm);
  label.setFrame(NSRect::new(NSPoint::new(24.0, 66.0), NSSize::new(372.0, 22.0)));
  content.addSubview(&label);

  let bar = NSProgressIndicator::initWithFrame(
    NSProgressIndicator::alloc(mtm),
    NSRect::new(NSPoint::new(24.0, 36.0), NSSize::new(372.0, 20.0)),
  );
  bar.setStyle(NSProgressIndicatorStyle::Bar);
  bar.setIndeterminate(false);
  bar.setMinValue(0.0);
  bar.setMaxValue(1.0);
  content.addSubview(&bar);

  window.setContentView(Some(&content));
  window.center();
  window.makeKeyAndOrderFront(None);
  let app = NSApplication::sharedApplication(mtm);
  #[allow(deprecated)]
  app.activateIgnoringOtherApps(true);
  runner.visible = true;
  runner.window = Some(window);
}

/// Refresh the install window from the installer's progress.
pub fn progress(runner: &mut Runner) {
  let (Some(window), Some(install)) = (&runner.window, &runner.install) else {
    return;
  };
  let fraction = install.fraction();
  let Some(content) = window.contentView() else {
    return;
  };
  for sub in content.subviews().iter() {
    if let Some(bar) = sub.downcast_ref::<NSProgressIndicator>() {
      bar.setDoubleValue(fraction);
    } else if let Some(label) = sub.downcast_ref::<NSTextField>() {
      label.setStringValue(&NSString::from_str(&format!(
        "Installing macOS… {}%",
        (fraction * 100.0).round() as u32
      )));
    }
  }
}

/// Swap the install window for the machine's own.
pub fn installed(runner: &mut Runner) {
  if let Some(window) = runner.window.take() {
    window.orderOut(None);
  }
}
