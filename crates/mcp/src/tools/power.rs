//! Power and windows.

use std::time::Duration;

use serde_json::{json, Value};
use vista_kit::ipc::{Request, Response};

use super::{ask, flag, machine_arg, tool, Context};
use crate::output::Output;

pub fn defs() -> Vec<Value> {
  let one = |name: &str, title: &str, description: &str, destructive: bool| {
    tool(
      name,
      title,
      description,
      json!({"machine": machine_arg()}),
      &["machine"],
      false,
      destructive,
    )
  };
  vec![
    tool(
      "start_machine",
      "Start a machine",
      "Power on a machine (or resume it if suspended). Each machine runs in its own process; this returns once that process answers, with the machine's state. The guest OS still needs time to boot — take a screenshot to see where it is.",
      json!({
        "machine": machine_arg(),
        "visible": {"type": "boolean", "description": "Open its window (default true). Screenshots and pointer tools need a visible window; a hidden machine can be shown later with show_machine."}
      }),
      &["machine"],
      false,
      false,
    ),
    tool(
      "stop_machine",
      "Stop a machine",
      "Shut a machine down. By default this presses the guest's power button so it shuts down cleanly (the guest decides; macOS may ask about unsaved work). force=true pulls the plug immediately, like holding the power button — unsaved data in the guest is lost. wait=true (default) waits up to 60 seconds for it to finish.",
      json!({
        "machine": machine_arg(),
        "force": {"type": "boolean", "description": "Stop immediately without asking the guest (default false)."},
        "wait": {"type": "boolean", "description": "Wait for the machine to be off (default true)."}
      }),
      &["machine"],
      false,
      true,
    ),
    one("restart_machine", "Restart a machine", "Ask the guest to shut down, then start it again with its current saved configuration (so configure_machine changes take effect). Falls back to a hard restart if the guest can't be asked.", false),
    one("pause_machine", "Pause a machine", "Freeze a running machine in place: its CPUs stop, memory stays. resume_machine continues exactly where it was.", false),
    one("resume_machine", "Resume a machine", "Continue a paused machine.", false),
    one("suspend_machine", "Suspend a machine", "Save the machine's memory to disk and quit its process. The next start_machine resumes it exactly where it was, in seconds. Not every configuration supports it.", false),
    one("show_machine", "Show a machine's window", "Bring a running machine's window to the front (opening it if the machine was started hidden). Needed before screenshots of a hidden machine.", false),
    one("hide_machine", "Hide a machine's window", "Hide a running machine's window; it keeps running in the background.", false),
  ]
}

pub async fn call(context: &Context, name: &str, args: &Value) -> Option<Result<Output, String>> {
  let request = match name {
    "start_machine" => return Some(start(context, args).await),
    "stop_machine" => return Some(stop(context, args).await),
    "restart_machine" => Request::Restart,
    "pause_machine" => Request::Pause,
    "resume_machine" => Request::Resume,
    "suspend_machine" => Request::Suspend,
    "show_machine" => Request::Show,
    "hide_machine" => Request::Hide,
    _ => return None,
  };
  Some(simple(context, args, request, name).await)
}

async fn simple(
  context: &Context,
  args: &Value,
  request: Request,
  name: &str,
) -> Result<Output, String> {
  let bundle = context.machine(args).await?;
  ask(&bundle, &request).await?;
  let verb = match name {
    "restart_machine" => "is restarting",
    "pause_machine" => "is paused",
    "resume_machine" => "is running again",
    "suspend_machine" => "is suspended; start_machine resumes it",
    "show_machine" => "is in front",
    _ => "is hidden",
  };
  Ok(Output::text(format!("{} {verb}.", bundle.config.name)))
}

async fn start(context: &Context, args: &Value) -> Result<Output, String> {
  let bundle = context.machine(args).await?;
  let id = bundle.config.id.clone();
  let visible = flag(args, "visible").unwrap_or(true);
  if let Some(live) = vista_kit::ipc::status(&id).await {
    if visible && !live.visible {
      let _ = vista_kit::ipc::call(&id, &Request::Show).await;
    }
    return Ok(Output::data(
      format!("{} is already running.", bundle.config.name),
      serde_json::to_value(live).unwrap_or(Value::Null),
    ));
  }
  vista_kit::launch::start(&bundle.path, visible).map_err(|e| e.to_string())?;
  let store = context.store().await?;
  let _ = store.touch(&id).await;
  let live = vista_kit::launch::wait_ready(&id, Duration::from_secs(30))
    .await
    .ok_or_else(|| {
      format!(
        "{} didn't come up. Its log is {}/machine.log.",
        bundle.config.name,
        bundle.path.display()
      )
    })?;
  if live.state == vista_kit::machine::State::Error {
    return Err(format!(
      "{} failed to start: {}",
      bundle.config.name,
      live.message.unwrap_or_default()
    ));
  }
  Ok(Output::data(
    format!(
      "{} is {}. Give the guest a moment to boot, then take a screenshot.",
      bundle.config.name,
      live.state.label().to_lowercase()
    ),
    serde_json::to_value(live).unwrap_or(Value::Null),
  ))
}

async fn stop(context: &Context, args: &Value) -> Result<Output, String> {
  let bundle = context.machine(args).await?;
  let id = bundle.config.id.clone();
  let force = flag(args, "force").unwrap_or(false);
  let request = if force { Request::Kill } else { Request::Stop };
  match ask(&bundle, &request).await {
    Ok(_) => {}
    Err(err) if err.contains("isn't running") => {
      return Ok(Output::text(format!(
        "{} is already off.",
        bundle.config.name
      )))
    }
    Err(err) => return Err(err),
  }
  if flag(args, "wait").unwrap_or(true) {
    for _ in 0..120 {
      if vista_kit::ipc::status(&id).await.is_none() {
        return Ok(Output::text(format!("{} is off.", bundle.config.name)));
      }
      tokio::time::sleep(Duration::from_millis(500)).await;
    }
    return Ok(Output::text(format!(
      "Asked {} to shut down, but it's still running after 60 seconds (the guest may be showing a dialog — take a screenshot). stop_machine with force=true stops it at once.",
      bundle.config.name
    )));
  }
  let _ = Response::Ok;
  Ok(Output::text(format!(
    "Asked {} to shut down.",
    bundle.config.name
  )))
}
