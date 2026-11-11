//! Seeing and driving the guest's screen. Nothing needs to be installed in
//! the guest: input goes through the machine's virtual keyboard and pointer.

use std::time::Duration;

use serde_json::{json, Value};
use vista_kit::bundle::Bundle;
use vista_kit::ipc::{MouseAction, Request, Response};

use super::{ask, flag, machine_arg, number, required, tool, Context};
use crate::output::Output;

fn after() -> Value {
  json!({"type": "boolean", "description": "Return a screenshot after acting (default false)."})
}

fn settle() -> Value {
  json!({"type": "integer", "minimum": 0, "maximum": 10000, "description": "With screenshot: milliseconds to wait before taking it, so the guest can react (default 600)."})
}

fn point(extra: Value) -> Value {
  let mut props = json!({
    "machine": machine_arg(),
    "x": {"type": "number", "description": "Pixels from the left edge of the screenshot."},
    "y": {"type": "number", "description": "Pixels from the top edge of the screenshot."},
    "screenshot": after(),
    "settle_ms": settle()
  });
  if let (Some(props), Value::Object(extra)) = (props.as_object_mut(), extra) {
    props.extend(extra);
  }
  props
}

pub fn defs() -> Vec<Value> {
  vec![
    tool(
      "screenshot",
      "Take a screenshot",
      "A PNG of the machine's display, exactly as a person would see it. The image's pixel size is the coordinate space for click/drag/scroll (origin top-left). The window is shown first if the machine runs hidden. Take one after starting a machine, and after acting, to check what happened.",
      json!({"machine": machine_arg()}),
      &["machine"],
      true,
      false,
    ),
    tool(
      "click",
      "Click",
      "Move the pointer to (x, y) in screenshot pixels and click the left button.",
      point(json!({})),
      &["machine", "x", "y"],
      false,
      false,
    ),
    tool(
      "double_click",
      "Double-click",
      "Double-click the left button at (x, y), e.g. to open a file or select a word.",
      point(json!({})),
      &["machine", "x", "y"],
      false,
      false,
    ),
    tool(
      "right_click",
      "Right-click",
      "Click the right (secondary) button at (x, y) to open a context menu.",
      point(json!({})),
      &["machine", "x", "y"],
      false,
      false,
    ),
    tool(
      "move_mouse",
      "Move the pointer",
      "Move the pointer to (x, y) without clicking — to hover menus and tooltips.",
      point(json!({})),
      &["machine", "x", "y"],
      false,
      false,
    ),
    tool(
      "drag",
      "Drag",
      "Press the left button at (x, y), move to (to_x, to_y) and release: dragging windows, files, sliders, or selecting text.",
      point(json!({
        "to_x": {"type": "number", "description": "End point, pixels from the left."},
        "to_y": {"type": "number", "description": "End point, pixels from the top."}
      })),
      &["machine", "x", "y", "to_x", "to_y"],
      false,
      false,
    ),
    tool(
      "scroll",
      "Scroll",
      "Scroll at (x, y). dy > 0 scrolls down (content moves up), dy < 0 up; dx scrolls sideways. Units are pixels; 300–600 is roughly a page of text.",
      point(json!({
        "dx": {"type": "number", "description": "Horizontal pixels (default 0)."},
        "dy": {"type": "number", "description": "Vertical pixels (default 0)."}
      })),
      &["machine", "x", "y"],
      false,
      false,
    ),
    tool(
      "type_text",
      "Type text",
      "Type text into whatever has keyboard focus in the guest, one keystroke per character on a US layout. Newlines press Return and tabs press Tab. Characters with no key on a US keyboard (é, emoji) are skipped and reported. Click a field first if needed.",
      json!({
        "machine": machine_arg(),
        "text": {"type": "string", "description": "What to type."},
        "screenshot": after(),
        "settle_ms": settle()
      }),
      &["machine", "text"],
      false,
      false,
    ),
    tool(
      "press_keys",
      "Press keys",
      "Press a key or shortcut. Examples: \"enter\", \"escape\", \"tab\", \"backspace\", \"delete\" (backspace), \"forwarddelete\", \"up\", \"pagedown\", \"f5\", \"cmd+c\", \"cmd+shift+t\", \"ctrl+alt+delete\", \"alt+f4\", \"cmd+space\". Modifiers: cmd (Command on macOS, Windows/Super key elsewhere), ctrl, alt/option, shift. Several presses separated by spaces run in order: \"cmd+a backspace\".",
      json!({
        "machine": machine_arg(),
        "keys": {"type": "string", "description": "Keys to press, e.g. \"cmd+s\" or \"ctrl+c enter\"."},
        "screenshot": after(),
        "settle_ms": settle()
      }),
      &["machine", "keys"],
      false,
      false,
    ),
  ]
}

pub async fn call(context: &Context, name: &str, args: &Value) -> Option<Result<Output, String>> {
  let result = match name {
    "screenshot" => screenshot(context, args).await,
    "click" => pointer(context, args, MouseAction::Click).await,
    "double_click" => pointer(context, args, MouseAction::Double).await,
    "right_click" => pointer(context, args, MouseAction::Right).await,
    "move_mouse" => pointer(context, args, MouseAction::Move).await,
    "drag" => pointer(context, args, MouseAction::Drag).await,
    "scroll" => scroll(context, args).await,
    "type_text" => keyboard(context, args, true).await,
    "press_keys" => keyboard(context, args, false).await,
    _ => return None,
  };
  Some(result)
}

/// Capture the screen, showing a hidden machine's window first.
pub async fn capture(bundle: &Bundle) -> Result<(String, u32, u32), String> {
  if let Some(live) = vista_kit::ipc::status(&bundle.config.id).await {
    if !live.visible {
      ask(bundle, &Request::Show).await?;
      tokio::time::sleep(Duration::from_millis(900)).await;
    }
  }
  match ask(bundle, &Request::Screenshot).await? {
    Response::Image { png, width, height } => Ok((png, width, height)),
    _ => Err("The machine didn't return an image.".into()),
  }
}

async fn screenshot(context: &Context, args: &Value) -> Result<Output, String> {
  let bundle = context.machine(args).await?;
  let (png, width, height) = capture(&bundle).await?;
  Ok(
    Output::text(format!(
    "{}'s screen: {width}×{height} pixels. Pointer coordinates use these pixels, origin top-left.",
    bundle.config.name
  ))
    .with_image(png),
  )
}

async fn finish(bundle: &Bundle, args: &Value, done: String) -> Result<Output, String> {
  if flag(args, "screenshot") != Some(true) {
    return Ok(Output::text(done));
  }
  let wait = number(args, "settle_ms")
    .unwrap_or(600.0)
    .clamp(0.0, 10_000.0);
  tokio::time::sleep(Duration::from_millis(wait as u64)).await;
  let (png, width, height) = capture(bundle).await?;
  Ok(Output::text(format!("{done} Screen now ({width}×{height}):")).with_image(png))
}

async fn pointer(context: &Context, args: &Value, action: MouseAction) -> Result<Output, String> {
  let bundle = context.machine(args).await?;
  let x = number(args, "x").ok_or("Missing x.")?;
  let y = number(args, "y").ok_or("Missing y.")?;
  let to = match action {
    MouseAction::Drag => Some((
      number(args, "to_x").ok_or("Missing to_x.")?,
      number(args, "to_y").ok_or("Missing to_y.")?,
    )),
    _ => None,
  };
  ask(&bundle, &Request::Mouse { action, x, y, to }).await?;
  let verb = match action {
    MouseAction::Click => "Clicked",
    MouseAction::Double => "Double-clicked",
    MouseAction::Right => "Right-clicked",
    MouseAction::Move => "Moved to",
    MouseAction::Drag => "Dragged from",
    MouseAction::Down => "Pressed at",
    MouseAction::Up => "Released at",
  };
  let mut done = format!("{verb} ({x:.0}, {y:.0})");
  if let Some((tx, ty)) = to {
    done.push_str(&format!(" to ({tx:.0}, {ty:.0})"));
  }
  done.push('.');
  finish(&bundle, args, done).await
}

async fn scroll(context: &Context, args: &Value) -> Result<Output, String> {
  let bundle = context.machine(args).await?;
  let x = number(args, "x").ok_or("Missing x.")?;
  let y = number(args, "y").ok_or("Missing y.")?;
  let dx = number(args, "dx").unwrap_or(0.0);
  let dy = number(args, "dy").unwrap_or(0.0);
  ask(&bundle, &Request::Scroll { x, y, dx, dy }).await?;
  finish(
    &bundle,
    args,
    format!("Scrolled ({dx:.0}, {dy:.0}) at ({x:.0}, {y:.0})."),
  )
  .await
}

async fn keyboard(context: &Context, args: &Value, typing: bool) -> Result<Output, String> {
  let bundle = context.machine(args).await?;
  let (request, done) = if typing {
    let text = super::text(args, "text").ok_or("Missing text.")?;
    let count = text.chars().count();
    (Request::Type { text }, format!("Typed {count} characters."))
  } else {
    let keys = required(args, "keys")?;
    (
      Request::Key { keys: keys.clone() },
      format!("Pressed {keys}."),
    )
  };
  ask(&bundle, &request).await?;
  finish(&bundle, args, done).await
}
