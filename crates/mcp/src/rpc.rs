//! JSON-RPC 2.0 over stdio, one message per line, as MCP specifies.

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

pub async fn serve() {
  let mut lines = BufReader::new(tokio::io::stdin()).lines();
  let mut stdout = tokio::io::stdout();
  let context = crate::tools::Context::new();
  while let Ok(Some(line)) = lines.next_line().await {
    if line.trim().is_empty() {
      continue;
    }
    let message: Value = match serde_json::from_str(&line) {
      Ok(message) => message,
      Err(err) => {
        let reply = json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": err.to_string()}});
        write(&mut stdout, &reply).await;
        continue;
      }
    };
    // Batches were dropped from the protocol, but answer them anyway.
    let messages = match message {
      Value::Array(items) => items,
      single => vec![single],
    };
    for message in messages {
      if let Some(reply) = handle(&context, message).await {
        write(&mut stdout, &reply).await;
      }
    }
  }
}

async fn write(stdout: &mut tokio::io::Stdout, value: &Value) {
  let mut text = value.to_string();
  text.push('\n');
  let _ = stdout.write_all(text.as_bytes()).await;
  let _ = stdout.flush().await;
}

async fn handle(context: &crate::tools::Context, message: Value) -> Option<Value> {
  let id = message.get("id").cloned();
  let method = message.get("method").and_then(Value::as_str).unwrap_or("");
  let params = message.get("params").cloned().unwrap_or(Value::Null);
  // Notifications get no answer.
  let id = id?;
  let result = match method {
    "initialize" => {
      let asked = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(VERSIONS[0]);
      let version = VERSIONS
        .iter()
        .find(|v| **v == asked)
        .unwrap_or(&VERSIONS[0]);
      Ok(json!({
        "protocolVersion": version,
        "capabilities": {"tools": {"listChanged": false}},
        "serverInfo": {
          "name": "vista",
          "title": "Vista",
          "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": crate::docs::INSTRUCTIONS
      }))
    }
    "ping" => Ok(json!({})),
    "tools/list" => Ok(json!({"tools": crate::tools::list()})),
    "tools/call" => {
      let name = params.get("name").and_then(Value::as_str).unwrap_or("");
      let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
      Ok(crate::tools::call(context, name, args).await.into_json())
    }
    "resources/list" => Ok(json!({"resources": []})),
    "prompts/list" => Ok(json!({"prompts": []})),
    _ => Err(json!({"code": -32601, "message": format!("Unknown method {method}")})),
  };
  Some(match result {
    Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
    Err(error) => json!({"jsonrpc": "2.0", "id": id, "error": error}),
  })
}
