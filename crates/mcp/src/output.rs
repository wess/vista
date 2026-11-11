//! What a tool hands back: text for the agent to read, structured data it
//! can rely on, and images when it asked to see the screen.

use serde_json::{json, Value};

#[derive(Default)]
pub struct Output {
  content: Vec<Value>,
  structured: Option<Value>,
  error: bool,
}

impl Output {
  pub fn text(text: impl Into<String>) -> Self {
    Output {
      content: vec![json!({"type": "text", "text": text.into()})],
      ..Output::default()
    }
  }

  /// Structured data, also rendered as pretty JSON for clients that only
  /// read text.
  pub fn data(summary: impl Into<String>, data: Value) -> Self {
    let pretty = serde_json::to_string_pretty(&data).unwrap_or_default();
    Output {
      content: vec![json!({"type": "text", "text": format!("{}\n\n{pretty}", summary.into())})],
      structured: Some(data),
      error: false,
    }
  }

  pub fn error(text: impl Into<String>) -> Self {
    Output {
      content: vec![json!({"type": "text", "text": text.into()})],
      structured: None,
      error: true,
    }
  }

  pub fn with_image(mut self, png: String) -> Self {
    self
      .content
      .push(json!({"type": "image", "data": png, "mimeType": "image/png"}));
    self
  }

  pub fn into_json(self) -> Value {
    let mut out = json!({"content": self.content, "isError": self.error});
    if let Some(data) = self.structured {
      // structuredContent must be an object.
      out["structuredContent"] = if data.is_object() {
        data
      } else {
        json!({"result": data})
      };
    }
    out
  }
}

impl From<Result<Output, String>> for Output {
  fn from(result: Result<Output, String>) -> Self {
    result.unwrap_or_else(Output::error)
  }
}
