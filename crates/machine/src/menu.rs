//! The runner's menu bar. No key equivalents on purpose: with system keys
//! captured, ⌘-shortcuts belong to the guest.

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{sel, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
use objc2_foundation::NSString;

fn item(
  mtm: MainThreadMarker,
  title: &str,
  action: Option<Sel>,
  target: Option<&AnyObject>,
) -> Retained<NSMenuItem> {
  let item = unsafe {
    NSMenuItem::initWithTitle_action_keyEquivalent(
      NSMenuItem::alloc(mtm),
      &NSString::from_str(title),
      action,
      &NSString::from_str(""),
    )
  };
  unsafe { item.setTarget(target) };
  item
}

fn submenu(
  mtm: MainThreadMarker,
  title: &str,
  items: Vec<Retained<NSMenuItem>>,
) -> Retained<NSMenuItem> {
  let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(title));
  for entry in items {
    menu.addItem(&entry);
  }
  let holder = item(mtm, title, None, None);
  holder.setSubmenu(Some(&menu));
  holder
}

pub fn install(app: &NSApplication, mtm: MainThreadMarker) {
  let target = crate::state::with(|runner| runner.delegate.clone());
  let target: Option<&AnyObject> = target.as_deref().map(|d| d.as_ref());
  let name = crate::state::with(|runner| runner.bundle.config.name.clone()).unwrap_or_default();

  let main = NSMenu::new(mtm);
  main.addItem(&submenu(
    mtm,
    &name,
    vec![
      item(mtm, "Open Vista", Some(sel!(showVista:)), target),
      NSMenuItem::separatorItem(mtm),
      item(mtm, "Hide", Some(sel!(hide:)), None),
      NSMenuItem::separatorItem(mtm),
      item(mtm, "Suspend and Close", Some(sel!(quitMachine:)), target),
    ],
  ));
  main.addItem(&submenu(
    mtm,
    "Machine",
    vec![
      item(mtm, "Pause", Some(sel!(pauseMachine:)), target),
      item(mtm, "Resume", Some(sel!(resumeMachine:)), target),
      NSMenuItem::separatorItem(mtm),
      item(mtm, "Take Snapshot", Some(sel!(snapshotMachine:)), target),
      item(mtm, "Eject Installer", Some(sel!(ejectInstaller:)), target),
      NSMenuItem::separatorItem(mtm),
      item(mtm, "Restart", Some(sel!(restartMachine:)), target),
      item(mtm, "Suspend", Some(sel!(suspendMachine:)), target),
      item(mtm, "Shut Down", Some(sel!(shutDownMachine:)), target),
      item(mtm, "Force Stop", Some(sel!(forceStopMachine:)), target),
    ],
  ));
  main.addItem(&submenu(
    mtm,
    "Window",
    vec![
      item(mtm, "Minimize", Some(sel!(performMiniaturize:)), None),
      item(mtm, "Zoom", Some(sel!(performZoom:)), None),
      item(
        mtm,
        "Enter Full Screen",
        Some(sel!(toggleFullScreen:)),
        None,
      ),
    ],
  ));
  app.setMainMenu(Some(&main));
}
