//! Keyboard and pointer input, synthesized and handed to the machine's view
//! exactly as if a person had used them. Works with any guest: no software
//! inside the machine is needed.
//!
//! Coordinates are screenshot pixels (origin top-left). Keys are named the
//! way people write shortcuts: `cmd+shift+t`, `ctrl+alt+delete`, `enter`.
//! `cmd`/`win`/`super` are the same key — the guest sees the platform key.

use std::time::Duration;

use objc2::rc::Retained;
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType};
use objc2_core_graphics::{CGEvent, CGEventFlags, CGEventType, CGScrollEventUnit};
use objc2_foundation::NSPoint;
use objc2_virtualization::VZVirtualMachineView;
use vista_kit::ipc::{MouseAction, Response};

use crate::control::main_sync;
use crate::state::with;

const SHIFT: u64 = 1 << 17;
const CONTROL: u64 = 1 << 18;
const OPTION: u64 = 1 << 19;
const COMMAND: u64 = 1 << 20;

/// macOS virtual key codes (ANSI layout) for each modifier.
const MODIFIERS: [(u64, u16); 4] = [(SHIFT, 56), (CONTROL, 59), (OPTION, 58), (COMMAND, 55)];

/// The key code and whether shift is needed for a character.
pub fn char_key(c: char) -> Option<(u16, bool)> {
  let lower = c.to_ascii_lowercase();
  let base = match lower {
    'a' => 0, 's' => 1, 'd' => 2, 'f' => 3, 'h' => 4, 'g' => 5, 'z' => 6, 'x' => 7,
    'c' => 8, 'v' => 9, 'b' => 11, 'q' => 12, 'w' => 13, 'e' => 14, 'r' => 15,
    'y' => 16, 't' => 17, 'o' => 31, 'u' => 32, 'i' => 34, 'p' => 35, 'l' => 37,
    'j' => 38, 'k' => 40, 'n' => 45, 'm' => 46,
    _ => u16::MAX,
  };
  if base != u16::MAX {
    return Some((base, c.is_ascii_uppercase()));
  }
  Some(match c {
    '1' => (18, false), '2' => (19, false), '3' => (20, false), '4' => (21, false),
    '5' => (23, false), '6' => (22, false), '7' => (26, false), '8' => (28, false),
    '9' => (25, false), '0' => (29, false),
    '!' => (18, true), '@' => (19, true), '#' => (20, true), '$' => (21, true),
    '%' => (23, true), '^' => (22, true), '&' => (26, true), '*' => (28, true),
    '(' => (25, true), ')' => (29, true),
    '-' => (27, false), '_' => (27, true), '=' => (24, false), '+' => (24, true),
    '[' => (33, false), '{' => (33, true), ']' => (30, false), '}' => (30, true),
    '\\' => (42, false), '|' => (42, true), ';' => (41, false), ':' => (41, true),
    '\'' => (39, false), '"' => (39, true), ',' => (43, false), '<' => (43, true),
    '.' => (47, false), '>' => (47, true), '/' => (44, false), '?' => (44, true),
    '`' => (50, false), '~' => (50, true),
    ' ' => (49, false), '\n' => (36, false), '\t' => (48, false),
    _ => return None,
  })
}

/// A named key: `enter`, `f5`, `pagedown`, or a single character.
pub fn named_key(name: &str) -> Option<(u16, bool)> {
  let code = match name {
    "enter" | "return" => 36,
    "tab" => 48,
    "space" => 49,
    "backspace" | "delete" => 51,
    "escape" | "esc" => 53,
    "forwarddelete" | "del" => 117,
    "home" => 115,
    "end" => 119,
    "pageup" => 116,
    "pagedown" => 121,
    "left" => 123,
    "right" => 124,
    "down" => 125,
    "up" => 126,
    "f1" => 122, "f2" => 120, "f3" => 99, "f4" => 118, "f5" => 96, "f6" => 97,
    "f7" => 98, "f8" => 100, "f9" => 101, "f10" => 109, "f11" => 103, "f12" => 111,
    "capslock" => 57,
    _ => {
      let mut chars = name.chars();
      return match (chars.next(), chars.next()) {
        (Some(c), None) => char_key(c),
        _ => None,
      };
    }
  };
  Some((code, false))
}

fn modifier(name: &str) -> Option<u64> {
  Some(match name {
    "cmd" | "command" | "super" | "win" | "windows" | "meta" => COMMAND,
    "ctrl" | "control" => CONTROL,
    "alt" | "option" | "opt" => OPTION,
    "shift" => SHIFT,
    _ => return None,
  })
}

/// Parse `cmd+shift+t` into modifier bits and a key.
pub fn parse_combo(combo: &str) -> Result<(u64, Option<u16>), String> {
  let mut flags = 0u64;
  let mut key = None;
  for original in combo.split('+').map(str::trim) {
    if original.is_empty() {
      continue;
    }
    // Names are case-insensitive; a lone character keeps its case, so "A"
    // means shift+a.
    let part = if original.chars().count() == 1 {
      original.to_string()
    } else {
      original.to_lowercase()
    };
    if let Some(bit) = modifier(&part) {
      flags |= bit;
    } else if let Some((code, shift)) = named_key(&part) {
      if shift {
        flags |= SHIFT;
      }
      key = Some(code);
    } else {
      return Err(format!("Unknown key \"{part}\" in \"{combo}\"."));
    }
  }
  Ok((flags, key))
}

fn view() -> Option<(Retained<VZVirtualMachineView>, isize, f64, f64)> {
  with(|runner| {
    let view = runner.view.clone()?;
    let window = runner.window.as_ref()?;
    let height = window.contentLayoutRect().size.height;
    Some((view, window.windowNumber(), window.backingScaleFactor(), height))
  })
  .flatten()
}

/// Real keyboard events also carry which physical modifier is down (left
/// shift, left command…) in the low bits; the view reads those to tell the
/// guest which key changed. Synthesized events say "the left one".
fn with_device_bits(flags: u64) -> u64 {
  let mut out = flags | 0x100;
  if flags & SHIFT != 0 {
    out |= 0x02;
  }
  if flags & CONTROL != 0 {
    out |= 0x01;
  }
  if flags & OPTION != 0 {
    out |= 0x20;
  }
  if flags & COMMAND != 0 {
    out |= 0x08;
  }
  out
}

fn post_key(code: u16, flags: u64, down: bool, is_modifier: bool) -> Result<(), String> {
  main_sync(move || {
    let (view, ..) = view().ok_or("The machine has no display.")?;
    let event = CGEvent::new_keyboard_event(None, code, down).ok_or("Couldn't create a key event.")?;
    CGEvent::set_flags(Some(&event), CGEventFlags(with_device_bits(flags)));
    if is_modifier {
      CGEvent::set_type(Some(&event), CGEventType::FlagsChanged);
    }
    let event = NSEvent::eventWithCGEvent(&event).ok_or("Couldn't convert a key event.")?;
    if is_modifier {
      view.flagsChanged(&event);
    } else if down {
      view.keyDown(&event);
    } else {
      view.keyUp(&event);
    }
    Ok(())
  })
}

const GAP: Duration = Duration::from_millis(12);

/// Press modifiers, tap the key, release modifiers in reverse.
async fn chord(flags: u64, key: Option<u16>) -> Result<(), String> {
  let mut held = 0u64;
  for (bit, code) in MODIFIERS {
    if flags & bit != 0 {
      held |= bit;
      post_key(code, held, true, true)?;
      tokio::time::sleep(GAP).await;
    }
  }
  if let Some(code) = key {
    post_key(code, held, true, false)?;
    tokio::time::sleep(GAP).await;
    post_key(code, held, false, false)?;
    tokio::time::sleep(GAP).await;
  }
  for (bit, code) in MODIFIERS.iter().rev() {
    if held & bit != 0 {
      held &= !bit;
      post_key(*code, held, false, true)?;
      tokio::time::sleep(GAP).await;
    }
  }
  Ok(())
}

pub async fn keys(combo: &str) -> Response {
  // Several combos can be sent at once, separated by spaces: "cmd+a delete".
  for part in combo.split_whitespace() {
    let result = match parse_combo(part) {
      Ok((flags, key)) => chord(flags, key).await,
      Err(err) => Err(err),
    };
    if let Err(err) = result {
      return Response::error(err);
    }
  }
  Response::Ok
}

pub async fn type_text(text: &str) -> Response {
  let mut skipped: Vec<char> = Vec::new();
  for c in text.chars() {
    match char_key(c) {
      Some((code, shift)) => {
        if let Err(err) = chord(if shift { SHIFT } else { 0 }, Some(code)).await {
          return Response::error(err);
        }
      }
      None => skipped.push(c),
    }
  }
  if skipped.is_empty() {
    Response::Ok
  } else {
    let list: String = skipped.into_iter().collect();
    Response::error(format!(
      "Typed everything except characters with no key on a US keyboard: {list:?}. \
       Use Vista Tools (guest_write_file / guest_exec) or the clipboard for those."
    ))
  }
}

fn post_mouse(kind: NSEventType, x: f64, y: f64, clicks: isize) -> Result<(), String> {
  main_sync(move || {
    let (view, number, scale, height) = view().ok_or("The machine has no display.")?;
    let location = NSPoint::new(x / scale, height - y / scale);
    let event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
      kind,
      location,
      NSEventModifierFlags::empty(),
      0.0,
      number,
      None,
      0,
      clicks,
      if matches!(kind, NSEventType::LeftMouseUp | NSEventType::RightMouseUp | NSEventType::MouseMoved) { 0.0 } else { 1.0 },
    )
    .ok_or("Couldn't create a mouse event.")?;
    match kind {
      NSEventType::LeftMouseDown => view.mouseDown(&event),
      NSEventType::LeftMouseUp => view.mouseUp(&event),
      NSEventType::RightMouseDown => view.rightMouseDown(&event),
      NSEventType::RightMouseUp => view.rightMouseUp(&event),
      NSEventType::LeftMouseDragged => view.mouseDragged(&event),
      _ => view.mouseMoved(&event),
    }
    Ok(())
  })
}

pub async fn mouse(action: MouseAction, x: f64, y: f64, to: Option<(f64, f64)>) -> Response {
  use NSEventType as T;
  let pause = Duration::from_millis(40);
  let result: Result<(), String> = async {
    post_mouse(T::MouseMoved, x, y, 0)?;
    tokio::time::sleep(pause).await;
    match action {
      MouseAction::Move => {}
      MouseAction::Click => {
        post_mouse(T::LeftMouseDown, x, y, 1)?;
        tokio::time::sleep(pause).await;
        post_mouse(T::LeftMouseUp, x, y, 1)?;
      }
      MouseAction::Double => {
        for clicks in 1..=2 {
          post_mouse(T::LeftMouseDown, x, y, clicks)?;
          tokio::time::sleep(Duration::from_millis(20)).await;
          post_mouse(T::LeftMouseUp, x, y, clicks)?;
          tokio::time::sleep(Duration::from_millis(60)).await;
        }
      }
      MouseAction::Right => {
        post_mouse(T::RightMouseDown, x, y, 1)?;
        tokio::time::sleep(pause).await;
        post_mouse(T::RightMouseUp, x, y, 1)?;
      }
      MouseAction::Down => post_mouse(T::LeftMouseDown, x, y, 1)?,
      MouseAction::Up => post_mouse(T::LeftMouseUp, x, y, 1)?,
      MouseAction::Drag => {
        let (tx, ty) = to.ok_or("A drag needs an end point (to_x, to_y).")?;
        post_mouse(T::LeftMouseDown, x, y, 1)?;
        let steps = 12;
        for step in 1..=steps {
          let f = step as f64 / steps as f64;
          tokio::time::sleep(Duration::from_millis(16)).await;
          post_mouse(T::LeftMouseDragged, x + (tx - x) * f, y + (ty - y) * f, 1)?;
        }
        tokio::time::sleep(pause).await;
        post_mouse(T::LeftMouseUp, tx, ty, 1)?;
      }
    }
    Ok(())
  }
  .await;
  match result {
    Ok(()) => Response::Ok,
    Err(err) => Response::error(err),
  }
}

pub async fn scroll(x: f64, y: f64, dx: f64, dy: f64) -> Response {
  if let Err(err) = post_mouse(NSEventType::MouseMoved, x, y, 0) {
    return Response::error(err);
  }
  let result = main_sync(move || -> Result<(), String> {
    let (view, ..) = view().ok_or("The machine has no display.")?;
    let event = CGEvent::new_scroll_wheel_event2(
      None,
      CGScrollEventUnit::Pixel,
      2,
      -dy as i32,
      -dx as i32,
      0,
    )
    .ok_or("Couldn't create a scroll event.")?;
    let event = NSEvent::eventWithCGEvent(&event).ok_or("Couldn't convert a scroll event.")?;
    view.scrollWheel(&event);
    Ok(())
  });
  match result {
    Ok(()) => Response::Ok,
    Err(err) => Response::error(err),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn combos() {
    assert_eq!(parse_combo("cmd+c").unwrap(), (COMMAND, Some(8)));
    assert_eq!(parse_combo("ctrl+alt+del").unwrap(), (CONTROL | OPTION, Some(117)));
    assert_eq!(parse_combo("shift").unwrap(), (SHIFT, None));
    assert_eq!(parse_combo("A").unwrap(), (SHIFT, Some(0)));
    assert!(parse_combo("cmd+nope").is_err());
  }

  #[test]
  fn characters() {
    assert_eq!(char_key('?'), Some((44, true)));
    assert_eq!(char_key('é'), None);
  }
}
