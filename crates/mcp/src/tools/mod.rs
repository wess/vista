//! The tool catalog and dispatch.

pub mod guest;
pub mod jobs;
pub mod library;
pub mod power;
pub mod screen;
pub mod snapshots;

use serde_json::{json, Value};
use tokio::sync::OnceCell;
use vista_kit::bundle::Bundle;
use vista_kit::store::Store;

use crate::output::Output;

pub struct Context {
  store: OnceCell<Store>,
}

impl Context {
  pub fn new() -> Self {
    Context {
      store: OnceCell::new(),
    }
  }

  pub async fn store(&self) -> Result<&Store, String> {
    self
      .store
      .get_or_try_init(|| async { Store::open().await.map_err(|e| e.to_string()) })
      .await
  }

  /// The machine named in `args["machine"]`, if agents may touch it.
  pub async fn machine(&self, args: &Value) -> Result<Bundle, String> {
    let store = self.store().await?;
    allowed(store).await?;
    let (_, bundle) = crate::machine::find(store, &text(args, "machine").unwrap_or_default()).await?;
    if !bundle.config.agents {
      return Err(format!(
        "Agent control is turned off for {}. The person at this Mac can turn it on in the machine's settings in Vista.",
        bundle.config.name
      ));
    }
    Ok(bundle)
  }
}

pub async fn allowed(store: &Store) -> Result<(), String> {
  let settings = store.settings().await.map_err(|e| e.to_string())?;
  if settings.agents {
    Ok(())
  } else {
    Err("Agent control is turned off in Vista's Settings (Agents). Ask the person at this Mac to turn it on.".into())
  }
}

pub fn text(args: &Value, key: &str) -> Option<String> {
  args.get(key).and_then(|v| match v {
    Value::String(s) => Some(s.clone()),
    Value::Number(n) => Some(n.to_string()),
    _ => None,
  })
}

pub fn number(args: &Value, key: &str) -> Option<f64> {
  args.get(key).and_then(|v| match v {
    Value::Number(n) => n.as_f64(),
    Value::String(s) => s.parse().ok(),
    _ => None,
  })
}

pub fn flag(args: &Value, key: &str) -> Option<bool> {
  args.get(key).and_then(|v| match v {
    Value::Bool(b) => Some(*b),
    Value::String(s) => Some(s == "true"),
    _ => None,
  })
}

pub fn required(args: &Value, key: &str) -> Result<String, String> {
  text(args, key)
    .filter(|s| !s.trim().is_empty())
    .ok_or_else(|| format!("Missing \"{key}\"."))
}

/// The `machine` argument every per-machine tool takes.
pub fn machine_arg() -> Value {
  json!({
    "type": "string",
    "description": "The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them."
  })
}

/// A tool definition.
pub fn tool(name: &str, title: &str, description: &str, properties: Value, required: &[&str], read_only: bool, destructive: bool) -> Value {
  json!({
    "name": name,
    "title": title,
    "description": description,
    "inputSchema": {
      "type": "object",
      "properties": properties,
      "required": required,
      "additionalProperties": false
    },
    "annotations": {
      "title": title,
      "readOnlyHint": read_only,
      "destructiveHint": destructive,
      "idempotentHint": read_only,
      "openWorldHint": false
    }
  })
}

pub fn list() -> Vec<Value> {
  let mut all = Vec::new();
  all.extend(library::defs());
  all.extend(jobs::defs());
  all.extend(power::defs());
  all.extend(snapshots::defs());
  all.extend(screen::defs());
  all.extend(guest::defs());
  all
}

pub async fn call(context: &Context, name: &str, args: Value) -> Output {
  let result = if let Some(out) = library::call(context, name, &args).await {
    out
  } else if let Some(out) = jobs::call(context, name, &args).await {
    out
  } else if let Some(out) = power::call(context, name, &args).await {
    out
  } else if let Some(out) = snapshots::call(context, name, &args).await {
    out
  } else if let Some(out) = screen::call(context, name, &args).await {
    out
  } else if let Some(out) = guest::call(context, name, &args).await {
    out
  } else {
    Err(format!("Vista has no tool called \"{name}\"."))
  };
  result.into()
}

/// A running machine's answer to `request`, or an error saying it's off.
pub async fn ask(bundle: &Bundle, request: &vista_kit::ipc::Request) -> Result<vista_kit::ipc::Response, String> {
  if vista_kit::ipc::status(&bundle.config.id).await.is_none() {
    return Err(format!("{} isn't running. start_machine starts it.", bundle.config.name));
  }
  vista_kit::ipc::call(&bundle.config.id, request)
    .await
    .map_err(|e| e.to_string())?
    .into_result()
    .map_err(|e| e.to_string())
}
