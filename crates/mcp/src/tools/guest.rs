//! Inside the guest, through Vista Tools: commands, files, directories.

use base64::Engine;
use serde_json::{json, Value};
use vista_kit::bundle::Bundle;
use vista_kit::guest::{Request as Guest, Response as Answer, MAX_FILE};
use vista_kit::ipc::{Request, Response};

use super::{ask, flag, machine_arg, number, required, text, tool, Context};
use crate::output::Output;

const CHUNK: u64 = 8 * 1024 * 1024;

pub fn defs() -> Vec<Value> {
  let path = |what: &str| json!({"type": "string", "description": what});
  vec![
    tool(
      "guest_info",
      "Guest information",
      "Ask Vista Tools inside the guest who and what it is: OS, version, kernel, architecture, hostname, and the user it runs commands as. A quick way to check Vista Tools is installed and answering.",
      json!({"machine": machine_arg()}),
      &["machine"],
      true,
      false,
    ),
    tool(
      "run_command",
      "Run a command in the guest",
      "Run a shell command inside the guest and get its exit code, stdout and stderr (each truncated to 1 MB). Runs through /bin/sh -c by default, so pipes, globs and && work. Commands run as the guest's console user unless admin=true (root). Interactive programs that wait for input will hang until the timeout; pass stdin or use non-interactive flags (apt-get -y, DEBIAN_FRONTEND=noninteractive). Needs Vista Tools in the guest (Linux, macOS).",
      json!({
        "machine": machine_arg(),
        "command": {"type": "string", "description": "The command line, e.g. \"uname -a\" or \"cd /srv && ls -la\"."},
        "cwd": path("Working directory (default: the user's home)."),
        "env": {"type": "object", "additionalProperties": {"type": "string"}, "description": "Extra environment variables."},
        "stdin": {"type": "string", "description": "Text to feed to the command's standard input."},
        "timeout": {"type": "integer", "minimum": 1, "maximum": 3600, "description": "Seconds before the command is killed (default 120)."},
        "admin": {"type": "boolean", "description": "Run as root (default false)."},
        "shell": {"type": "boolean", "description": "Run through /bin/sh -c (default true). false splits the command on spaces and runs it directly."}
      }),
      &["machine", "command"],
      false,
      false,
    ),
    tool(
      "read_file",
      "Read a guest file",
      "Read a file inside the guest. Text comes back as text; binary files (or encoding=base64) come back base64-encoded. Up to 64 MB per call; use offset/length to page through bigger files, or download_file to copy one to the Mac.",
      json!({
        "machine": machine_arg(),
        "path": path("Absolute path in the guest."),
        "offset": {"type": "integer", "minimum": 0, "description": "Byte offset to start at."},
        "length": {"type": "integer", "minimum": 1, "description": "Bytes to read."},
        "encoding": {"type": "string", "enum": ["auto", "text", "base64"], "description": "auto (default): text when it's valid UTF-8."}
      }),
      &["machine", "path"],
      true,
      false,
    ),
    tool(
      "write_file",
      "Write a guest file",
      "Create or overwrite a file inside the guest (parent directories are created). content is text unless encoding=base64. append=true adds to the end. mode sets permissions, e.g. \"755\" for a script.",
      json!({
        "machine": machine_arg(),
        "path": path("Absolute path in the guest."),
        "content": {"type": "string", "description": "The file's contents."},
        "encoding": {"type": "string", "enum": ["text", "base64"], "description": "How content is encoded (default text)."},
        "append": {"type": "boolean"},
        "mode": {"type": "string", "description": "Octal permissions, e.g. \"644\" or \"755\"."}
      }),
      &["machine", "path", "content"],
      false,
      false,
    ),
    tool(
      "list_directory",
      "List a guest directory",
      "The entries of a directory inside the guest with kind (file, dir, link), size, permissions and modification time.",
      json!({"machine": machine_arg(), "path": path("Absolute directory path in the guest.")}),
      &["machine", "path"],
      true,
      false,
    ),
    tool(
      "file_info",
      "Guest file info",
      "Whether a path exists inside the guest, and its kind, size, permissions and modification time.",
      json!({"machine": machine_arg(), "path": path("Absolute path in the guest.")}),
      &["machine", "path"],
      true,
      false,
    ),
    tool(
      "make_directory",
      "Make a guest directory",
      "Create a directory (and its parents) inside the guest.",
      json!({"machine": machine_arg(), "path": path("Absolute directory path in the guest.")}),
      &["machine", "path"],
      false,
      false,
    ),
    tool(
      "delete_path",
      "Delete a guest path",
      "Delete a file, or a directory with recursive=true, inside the guest.",
      json!({"machine": machine_arg(), "path": path("Absolute path in the guest."), "recursive": {"type": "boolean"}}),
      &["machine", "path"],
      false,
      true,
    ),
    tool(
      "upload_file",
      "Copy a file into the guest",
      "Copy a file from this Mac into the guest, in chunks (any size). For whole folders, share the folder instead (configure_machine add_shares) or archive it first.",
      json!({"machine": machine_arg(), "host_path": path("Absolute path on the Mac."), "guest_path": path("Absolute destination path in the guest.")}),
      &["machine", "host_path", "guest_path"],
      false,
      false,
    ),
    tool(
      "download_file",
      "Copy a file out of the guest",
      "Copy a file from the guest to this Mac, in chunks (any size).",
      json!({"machine": machine_arg(), "guest_path": path("Absolute path in the guest."), "host_path": path("Absolute destination path on the Mac.")}),
      &["machine", "guest_path", "host_path"],
      false,
      false,
    ),
  ]
}

async fn guest(bundle: &Bundle, request: Guest) -> Result<Answer, String> {
  match ask(bundle, &Request::Guest { request }).await? {
    Response::Guest { answer } => Ok(answer),
    _ => Err("Vista Tools gave an unexpected answer.".into()),
  }
}

pub async fn call(context: &Context, name: &str, args: &Value) -> Option<Result<Output, String>> {
  let known = [
    "guest_info",
    "run_command",
    "read_file",
    "write_file",
    "list_directory",
    "file_info",
    "make_directory",
    "delete_path",
    "upload_file",
    "download_file",
  ];
  if !known.contains(&name) {
    return None;
  }
  Some(dispatch(context, name, args).await)
}

async fn dispatch(context: &Context, name: &str, args: &Value) -> Result<Output, String> {
  let bundle = context.machine(args).await?;
  let b64 = base64::engine::general_purpose::STANDARD;
  match name {
    "guest_info" => match guest(&bundle, Guest::Hello).await? {
      Answer::Hello(hello) => Ok(Output::data(
        format!("{} ({}) on {}", hello.os, hello.arch, hello.hostname),
        serde_json::to_value(hello).unwrap_or(Value::Null),
      )),
      _ => Err("unexpected answer".into()),
    },
    "run_command" => {
      let request = Guest::Exec {
        command: required(args, "command")?,
        shell: flag(args, "shell").unwrap_or(true),
        cwd: text(args, "cwd"),
        env: args
          .get("env")
          .and_then(Value::as_object)
          .map(|m| {
            m.iter()
              .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_string())))
              .collect()
          })
          .unwrap_or_default(),
        stdin: text(args, "stdin"),
        timeout: number(args, "timeout").map(|t| t as u64).unwrap_or(120),
        admin: flag(args, "admin").unwrap_or(false),
      };
      match guest(&bundle, request).await? {
        Answer::Exec(exec) => {
          let status = match (exec.code, exec.timedout) {
            (_, true) => "timed out".to_string(),
            (Some(code), _) => format!("exit code {code}"),
            (None, _) => "killed by a signal".to_string(),
          };
          let mut text = format!("{status} after {} ms", exec.elapsed);
          if !exec.stdout.is_empty() {
            text.push_str(&format!("\n\n--- stdout ---\n{}", exec.stdout));
          }
          if !exec.stderr.is_empty() {
            text.push_str(&format!("\n\n--- stderr ---\n{}", exec.stderr));
          }
          let mut out = Output::data(text, serde_json::to_value(&exec).unwrap_or(Value::Null));
          if exec.timedout {
            out = Output::error(format!("The command timed out after {} ms.", exec.elapsed));
          }
          Ok(out)
        }
        _ => Err("unexpected answer".into()),
      }
    }
    "read_file" => {
      let path = required(args, "path")?;
      let request = Guest::Read {
        path: path.clone(),
        offset: number(args, "offset").unwrap_or(0.0) as u64,
        length: number(args, "length").map(|l| l as u64),
      };
      match guest(&bundle, request).await? {
        Answer::Data { data, size, eof } => {
          let bytes = b64.decode(&data).map_err(|e| e.to_string())?;
          let encoding = text(args, "encoding").unwrap_or_else(|| "auto".into());
          let as_text = match encoding.as_str() {
            "base64" => None,
            _ => String::from_utf8(bytes.clone()).ok(),
          };
          let note = format!(
            "{path}: {} of {size} bytes{}",
            bytes.len(),
            if eof {
              ""
            } else {
              " (more remains; read again with offset)"
            }
          );
          Ok(match as_text {
            Some(text) => Output::data(
              note,
              json!({"path": path, "size": size, "eof": eof, "encoding": "text", "content": text}),
            ),
            None => Output::data(
              note,
              json!({"path": path, "size": size, "eof": eof, "encoding": "base64", "content": data}),
            ),
          })
        }
        _ => Err("unexpected answer".into()),
      }
    }
    "write_file" => {
      let path = required(args, "path")?;
      let content = text(args, "content").unwrap_or_default();
      let data = match text(args, "encoding").as_deref() {
        Some("base64") => content,
        _ => b64.encode(content.as_bytes()),
      };
      guest(
        &bundle,
        Guest::Write {
          path: path.clone(),
          data,
          append: flag(args, "append").unwrap_or(false),
          mode: text(args, "mode"),
        },
      )
      .await?;
      Ok(Output::text(format!("Wrote {path}.")))
    }
    "list_directory" => match guest(
      &bundle,
      Guest::List {
        path: required(args, "path")?,
      },
    )
    .await?
    {
      Answer::Entries { entries } => Ok(Output::data(
        format!("{} entries.", entries.len()),
        json!({"entries": entries}),
      )),
      _ => Err("unexpected answer".into()),
    },
    "file_info" => match guest(
      &bundle,
      Guest::Stat {
        path: required(args, "path")?,
      },
    )
    .await
    {
      Ok(Answer::Info(info)) => Ok(Output::data(
        format!("{} is a {}.", info.path, info.kind),
        json!({"exists": true, "info": info}),
      )),
      Ok(_) => Err("unexpected answer".into()),
      Err(err) if err.contains("No such file") || err.contains("not found") => {
        Ok(Output::data("It doesn't exist.", json!({"exists": false})))
      }
      Err(err) => Err(err),
    },
    "make_directory" => {
      let path = required(args, "path")?;
      guest(&bundle, Guest::Mkdir { path: path.clone() }).await?;
      Ok(Output::text(format!("Made {path}.")))
    }
    "delete_path" => {
      let path = required(args, "path")?;
      guest(
        &bundle,
        Guest::Remove {
          path: path.clone(),
          recursive: flag(args, "recursive").unwrap_or(false),
        },
      )
      .await?;
      Ok(Output::text(format!("Deleted {path}.")))
    }
    "upload_file" => {
      let host = required(args, "host_path")?;
      let target = required(args, "guest_path")?;
      let bytes = std::fs::read(&host).map_err(|e| format!("{host}: {e}"))?;
      let mut first = true;
      for chunk in bytes.chunks(CHUNK as usize) {
        guest(
          &bundle,
          Guest::Write {
            path: target.clone(),
            data: b64.encode(chunk),
            append: !first,
            mode: None,
          },
        )
        .await?;
        first = false;
      }
      if bytes.is_empty() {
        guest(
          &bundle,
          Guest::Write {
            path: target.clone(),
            data: String::new(),
            append: false,
            mode: None,
          },
        )
        .await?;
      }
      Ok(Output::text(format!(
        "Copied {} bytes to {target}.",
        bytes.len()
      )))
    }
    "download_file" => {
      let source = required(args, "guest_path")?;
      let host = required(args, "host_path")?;
      let mut out = Vec::new();
      loop {
        let request = Guest::Read {
          path: source.clone(),
          offset: out.len() as u64,
          length: Some(CHUNK.min(MAX_FILE)),
        };
        match guest(&bundle, request).await? {
          Answer::Data { data, eof, .. } => {
            out.extend(b64.decode(&data).map_err(|e| e.to_string())?);
            if eof {
              break;
            }
          }
          _ => return Err("unexpected answer".into()),
        }
      }
      if let Some(dir) = std::path::Path::new(&host).parent() {
        let _ = std::fs::create_dir_all(dir);
      }
      std::fs::write(&host, &out).map_err(|e| format!("{host}: {e}"))?;
      Ok(Output::text(format!(
        "Copied {} bytes to {host}.",
        out.len()
      )))
    }
    _ => Err("unknown".into()),
  }
}
