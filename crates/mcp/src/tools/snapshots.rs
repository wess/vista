//! Snapshots.

use serde_json::{json, Value};
use vista_kit::bundle::Snapshots;
use vista_kit::ipc::{Request, Response};

use super::{flag, machine_arg, required, text, tool, Context};
use crate::output::Output;

pub fn defs() -> Vec<Value> {
  let snapshot_arg = json!({"type": "string", "description": "The snapshot's id (or first 8 characters) or its name, from list_snapshots."});
  vec![
    tool(
      "take_snapshot",
      "Take a snapshot",
      "Save the machine's current state so you can come back to it. Works stopped or running: for a running machine the guest is paused for a moment and its memory is saved too, so reverting resumes exactly here (open apps and all). Copy-on-write: instant, and costs disk space only as the machine changes afterwards.",
      json!({"machine": machine_arg(), "name": {"type": "string", "description": "A name, e.g. \"before upgrade\" (default: Snapshot N)."}}),
      &["machine"],
      false,
      false,
    ),
    tool(
      "list_snapshots",
      "List snapshots",
      "A machine's snapshots, newest first, with id, name, when it was taken, and whether it includes memory (taken while running).",
      json!({"machine": machine_arg()}),
      &["machine"],
      true,
      false,
    ),
    tool(
      "revert_snapshot",
      "Revert to a snapshot",
      "Put the machine back exactly as it was when the snapshot was taken, discarding everything since. The machine must be off; stop_first=true force-stops it first. A snapshot taken while running leaves the machine suspended, so start_machine resumes into it. Consider take_snapshot first if the current state may be wanted.",
      json!({"machine": machine_arg(), "snapshot": snapshot_arg, "stop_first": {"type": "boolean", "description": "Force-stop the machine if it's running (default false)."}, "start": {"type": "boolean", "description": "Start the machine after reverting (default false)."}}),
      &["machine", "snapshot"],
      false,
      true,
    ),
    tool(
      "rename_snapshot",
      "Rename a snapshot",
      "Give a snapshot a new name.",
      json!({"machine": machine_arg(), "snapshot": snapshot_arg, "name": {"type": "string"}}),
      &["machine", "snapshot", "name"],
      false,
      false,
    ),
    tool(
      "delete_snapshot",
      "Delete a snapshot",
      "Remove a snapshot and free the space only it was using. The machine's current state is not affected.",
      json!({"machine": machine_arg(), "snapshot": snapshot_arg}),
      &["machine", "snapshot"],
      false,
      true,
    ),
  ]
}

pub async fn call(context: &Context, name: &str, args: &Value) -> Option<Result<Output, String>> {
  Some(match name {
    "take_snapshot" => take(context, args).await,
    "list_snapshots" => list(context, args).await,
    "revert_snapshot" => revert(context, args).await,
    "rename_snapshot" => rename(context, args).await,
    "delete_snapshot" => delete(context, args).await,
    _ => return None,
  })
}

fn pick(
  bundle: &vista_kit::bundle::Bundle,
  reference: &str,
) -> Result<vista_kit::bundle::Snapshot, String> {
  let list = Snapshots::list(bundle);
  list
    .iter()
    .find(|s| s.id == reference || (reference.len() >= 4 && s.id.starts_with(reference)))
    .or_else(|| list.iter().find(|s| s.name.eq_ignore_ascii_case(reference)))
    .cloned()
    .ok_or_else(|| format!("No snapshot \"{reference}\" on {}.", bundle.config.name))
}

async fn take(context: &Context, args: &Value) -> Result<Output, String> {
  let bundle = context.machine(args).await?;
  let name = text(args, "name").unwrap_or_default();
  let snapshot = if vista_kit::ipc::status(&bundle.config.id).await.is_some() {
    match vista_kit::ipc::call(&bundle.config.id, &Request::Snapshot { name })
      .await
      .map_err(|e| e.to_string())?
    {
      Response::Snapshot(snapshot) => snapshot,
      Response::Error { message } => return Err(message),
      _ => return Err("The machine gave an unexpected answer.".into()),
    }
  } else {
    Snapshots::take(&bundle, &name).map_err(|e| e.to_string())?
  };
  Ok(Output::data(
    format!("Took \"{}\" of {}.", snapshot.name, bundle.config.name),
    serde_json::to_value(snapshot).unwrap_or(Value::Null),
  ))
}

async fn list(context: &Context, args: &Value) -> Result<Output, String> {
  let bundle = context.machine(args).await?;
  let list = Snapshots::list(&bundle);
  Ok(Output::data(
    format!("{} has {} snapshot(s).", bundle.config.name, list.len()),
    json!({"snapshots": list}),
  ))
}

async fn revert(context: &Context, args: &Value) -> Result<Output, String> {
  let bundle = context.machine(args).await?;
  let snapshot = pick(&bundle, &required(args, "snapshot")?)?;
  let id = bundle.config.id.clone();
  if vista_kit::ipc::status(&id).await.is_some() {
    if flag(args, "stop_first") != Some(true) {
      return Err(format!(
        "{} is running. Stop it, or pass stop_first=true.",
        bundle.config.name
      ));
    }
    let _ = vista_kit::ipc::call(&id, &Request::Kill).await;
    for _ in 0..40 {
      if vista_kit::ipc::status(&id).await.is_none() {
        break;
      }
      tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
  }
  Snapshots::revert(&bundle, &snapshot.id).map_err(|e| e.to_string())?;
  if flag(args, "start") == Some(true) {
    vista_kit::launch::start(&bundle.path, true).map_err(|e| e.to_string())?;
    vista_kit::launch::wait_ready(&id, std::time::Duration::from_secs(30)).await;
  }
  Ok(Output::text(format!(
    "{} is back at \"{}\"{}.",
    bundle.config.name,
    snapshot.name,
    if snapshot.live {
      " (suspended there: start_machine resumes it)"
    } else {
      ""
    }
  )))
}

async fn rename(context: &Context, args: &Value) -> Result<Output, String> {
  let bundle = context.machine(args).await?;
  let snapshot = pick(&bundle, &required(args, "snapshot")?)?;
  let name = required(args, "name")?;
  Snapshots::rename(&bundle, &snapshot.id, &name).map_err(|e| e.to_string())?;
  Ok(Output::text(format!("Renamed to \"{name}\".")))
}

async fn delete(context: &Context, args: &Value) -> Result<Output, String> {
  let bundle = context.machine(args).await?;
  let snapshot = pick(&bundle, &required(args, "snapshot")?)?;
  Snapshots::delete(&bundle, &snapshot.id).map_err(|e| e.to_string())?;
  Ok(Output::text(format!("Deleted \"{}\".", snapshot.name)))
}
