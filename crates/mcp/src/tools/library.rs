//! The library: what machines exist, what they are, and changing them.

use serde_json::{json, Value};
use vista_kit::bundle::{self, Bundle, Snapshots};
use vista_kit::machine::{Installer, Live, Network, Share, State};

use super::{flag, machine_arg, number, required, text, tool, Context};
use crate::output::Output;

pub fn defs() -> Vec<Value> {
  vec![
    tool(
      "list_machines",
      "List machines",
      "Every virtual machine in the Vista library with its state (running, stopped, suspended, …), operating system, hardware (CPUs, memory, disk size and the disk space it really uses on the Mac), IP address when running, and whether agent control is allowed. Start here to learn names and ids.",
      json!({}),
      &[],
      true,
      false,
    ),
    tool(
      "get_machine",
      "Get machine details",
      "Everything about one machine: full configuration (CPU, memory, disks, display, network mode and MAC address, shared folders, audio, Rosetta, clipboard, Secure Boot), live state (state, uptime, IP, window visible, install progress), its snapshots, whether an installer is still attached, the bundle path on the Mac, and how to reach the guest.",
      json!({"machine": machine_arg()}),
      &["machine"],
      true,
      false,
    ),
    tool(
      "list_systems",
      "List installable systems",
      "The operating systems Vista can download and install in one step, with the id to pass to create_machine. Includes macOS (latest and earlier majors, from Apple), Windows 11 for Arm (from Microsoft; experimental), and Linux/BSD distributions (Ubuntu, Fedora, Debian, Kali, CentOS Stream, Rocky, AlmaLinux, openSUSE, Pop!_OS, NixOS, Alpine, FreeBSD, OpenBSD, NetBSD). Each entry says whether it boots to a desktop and any tips for its installer. Optionally filter by os (macos, windows, linux, other) or a search word.",
      json!({
        "os": {"type": "string", "enum": ["macos", "windows", "linux", "other"], "description": "Only systems of this kind."},
        "search": {"type": "string", "description": "Only systems whose name, family or edition contains this (case-insensitive)."}
      }),
      &[],
      true,
      false,
    ),
    tool(
      "configure_machine",
      "Change machine settings",
      "Change a machine's hardware and options. Any field left out is unchanged. Changes to a running machine are saved and take effect at its next start (restart_machine applies them). Disks can only grow. Returns the new configuration.",
      json!({
        "machine": machine_arg(),
        "name": {"type": "string", "description": "New display name."},
        "cpus": {"type": "integer", "minimum": 1, "description": "Virtual CPU cores."},
        "memory_gb": {"type": "number", "minimum": 1, "description": "Memory in GB."},
        "disk_gb": {"type": "integer", "description": "Grow the system disk to this many GB (the guest must then extend its partition)."},
        "display_width": {"type": "integer", "description": "Display width in pixels (the window still resizes it while running)."},
        "display_height": {"type": "integer", "description": "Display height in pixels."},
        "network": {"type": "string", "description": "\"shared\" (NAT through the Mac, default), \"none\", or \"bridged:<interface>\" e.g. \"bridged:en0\" (needs Apple's bridging entitlement)."},
        "audio": {"type": "boolean", "description": "Sound output."},
        "microphone": {"type": "boolean", "description": "Give the guest the Mac's microphone."},
        "rosetta": {"type": "boolean", "description": "Linux only: share Rosetta so the guest can run x86-64 Linux binaries (mount with `mount -t virtiofs rosetta /media/rosetta`, then register binfmt)."},
        "clipboard": {"type": "boolean", "description": "Linux only: clipboard sharing through spice-vdagent."},
        "nested": {"type": "boolean", "description": "Nested virtualization (M3 or later)."},
        "autostart": {"type": "boolean", "description": "Start in the background when Vista opens."},
        "add_shares": {
          "type": "array",
          "description": "Mac folders to share into the guest.",
          "items": {"type": "object", "properties": {
            "path": {"type": "string", "description": "Absolute folder path on the Mac."},
            "name": {"type": "string", "description": "Name inside the guest (defaults to the folder name)."},
            "readonly": {"type": "boolean"}
          }, "required": ["path"]}
        },
        "remove_shares": {"type": "array", "items": {"type": "string"}, "description": "Names of shared folders to stop sharing."},
        "eject_installer": {"type": "boolean", "description": "Detach the installer media (ISO) at the next start."}
      }),
      &["machine"],
      false,
      false,
    ),
    tool(
      "import_machine",
      "Import a machine",
      "Add an existing Vista machine bundle (a .vista folder, e.g. copied from another Mac) to the library.",
      json!({"path": {"type": "string", "description": "Absolute path to the .vista folder."}}),
      &["path"],
      false,
      false,
    ),
    tool(
      "clone_machine",
      "Clone a machine",
      "Make a copy of a stopped machine. On APFS the copy shares blocks with the original, so it's instant and uses no space until either changes. The clone gets a new id, MAC address and machine identity; snapshots aren't copied.",
      json!({
        "machine": machine_arg(),
        "name": {"type": "string", "description": "Name for the copy (default: \"<name> copy\")."},
        "start": {"type": "boolean", "description": "Start the clone right away."}
      }),
      &["machine"],
      false,
      false,
    ),
    tool(
      "delete_machine",
      "Delete a machine",
      "Move a stopped machine's bundle (disks, snapshots, everything) to the Trash and remove it from the library. Irreversible from Vista's side. Requires confirm=true.",
      json!({
        "machine": machine_arg(),
        "confirm": {"type": "boolean", "description": "Must be true."}
      }),
      &["machine", "confirm"],
      false,
      true,
    ),
  ]
}

pub fn summary(entry_path: &str, bundle: &Bundle, live: Option<&Live>) -> Value {
  let config = &bundle.config;
  let state = live.map(|l| l.state).unwrap_or(if bundle.is_suspended() {
    State::Suspended
  } else {
    State::Stopped
  });
  json!({
    "id": config.id,
    "name": config.name,
    "os": config.os,
    "system": config.title(),
    "state": state,
    "cpus": config.cpus,
    "memory_gb": config.memory as f64 / 1024.0,
    "disk_gb": config.system().map(|d| d.size / 1_000_000_000).unwrap_or(0),
    "disk_used_gb": (bundle::usage(&bundle.path) as f64 / 1e9 * 10.0).round() / 10.0,
    "ip": live.and_then(|l| l.ip.clone()),
    "uptime_seconds": live.map(|l| l.uptime),
    "window_visible": live.map(|l| l.visible),
    "install_progress": live.and_then(|l| l.progress),
    "message": live.and_then(|l| l.message.clone()),
    "agents_allowed": config.agents,
    "path": entry_path,
  })
}

pub async fn call(context: &Context, name: &str, args: &Value) -> Option<Result<Output, String>> {
  Some(match name {
    "list_machines" => list(context).await,
    "get_machine" => get(context, args).await,
    "list_systems" => Ok(systems(args)),
    "configure_machine" => configure(context, args).await,
    "clone_machine" => clone(context, args).await,
    "import_machine" => import(context, args).await,
    "delete_machine" => delete(context, args).await,
    _ => return None,
  })
}

async fn list(context: &Context) -> Result<Output, String> {
  let store = context.store().await?;
  super::allowed(store).await?;
  let mut machines = Vec::new();
  for entry in store.machines().await.map_err(|e| e.to_string())? {
    let Ok(bundle) = bundle::load(&entry.path()) else {
      machines
        .push(json!({"id": entry.id, "name": entry.name, "state": "missing", "path": entry.path}));
      continue;
    };
    let live = vista_kit::ipc::status(&bundle.config.id).await;
    machines.push(summary(&entry.path, &bundle, live.as_ref()));
  }
  let count = machines.len();
  Ok(Output::data(
    format!("{count} machine{}.", if count == 1 { "" } else { "s" }),
    json!({"machines": machines}),
  ))
}

async fn get(context: &Context, args: &Value) -> Result<Output, String> {
  let bundle = context.machine(args).await?;
  let live = vista_kit::ipc::status(&bundle.config.id).await;
  let mut data = summary(&bundle.path.to_string_lossy(), &bundle, live.as_ref());
  let config = &bundle.config;
  data["configuration"] = serde_json::to_value(config).unwrap_or(Value::Null);
  data["snapshots"] = serde_json::to_value(Snapshots::list(&bundle)).unwrap_or(Value::Null);
  data["installer_attached"] = json!(config.installer.is_some());
  data["suspended"] = json!(bundle.is_suspended());
  data["guest_tools"] = json!(match config.os {
    vista_kit::machine::Os::Linux => "Vista Tools: `sudo mount -t virtiofs vistatools /mnt && sudo /mnt/install.sh` inside the guest.",
    vista_kit::machine::Os::Macos => "Vista Tools: open \"/Volumes/My Shared Files/Vista Tools/install.command\" inside the guest.",
    _ => "Vista Tools isn't available for this guest; use screenshot and the pointer/keyboard tools.",
  });
  Ok(Output::data(
    format!("{} — {}", config.name, config.title()),
    data,
  ))
}

fn systems(args: &Value) -> Output {
  let os = text(args, "os").map(|s| s.to_lowercase());
  let search = text(args, "search").map(|s| s.to_lowercase());
  let list: Vec<Value> = vista_kit::catalog::all()
    .into_iter()
    .filter(|e| {
      os.as_ref()
        .map(|o| {
          serde_json::to_value(e.os)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .as_deref()
            == Some(o.as_str())
        })
        .unwrap_or(true)
    })
    .filter(|e| {
      search
        .as_ref()
        .map(|s| {
          format!("{} {} {} {}", e.name, e.family, e.edition, e.version)
            .to_lowercase()
            .contains(s)
        })
        .unwrap_or(true)
    })
    .map(|e| {
      json!({
        "id": e.id,
        "name": e.title(),
        "edition": e.edition,
        "os": e.os,
        "family": e.family,
        "desktop": e.desktop,
        "notes": e.notes,
      })
    })
    .collect();
  Output::data(
    format!(
      "{} installable systems. Pass an id to create_machine as \"system\".",
      list.len()
    ),
    json!({"systems": list}),
  )
}

async fn configure(context: &Context, args: &Value) -> Result<Output, String> {
  let mut bundle = context.machine(args).await?;
  let running = vista_kit::ipc::status(&bundle.config.id).await.is_some();
  let config = &mut bundle.config;
  if let Some(name) = text(args, "name").filter(|n| !n.trim().is_empty()) {
    config.name = name.trim().to_string();
  }
  if let Some(cpus) = number(args, "cpus") {
    config.cpus = (cpus as u32).max(1);
  }
  if let Some(memory) = number(args, "memory_gb") {
    config.memory = ((memory * 1024.0) as u64).max(1024);
  }
  if let Some(width) = number(args, "display_width") {
    config.display.width = (width as u32).max(640);
  }
  if let Some(height) = number(args, "display_height") {
    config.display.height = (height as u32).max(400);
  }
  if let Some(network) = text(args, "network") {
    config.network = match network.as_str() {
      "shared" | "nat" => Network::Shared,
      "none" | "off" => Network::None,
      other => match other.strip_prefix("bridged:") {
        Some(interface) => Network::Bridged {
          interface: interface.to_string(),
        },
        None => {
          return Err(format!(
            "Unknown network \"{other}\": use shared, none or bridged:<interface>."
          ))
        }
      },
    };
  }
  for (key, field) in [
    ("audio", &mut config.audio),
    ("microphone", &mut config.microphone),
    ("rosetta", &mut config.rosetta),
    ("clipboard", &mut config.clipboard),
    ("nested", &mut config.nested),
    ("autostart", &mut config.autostart),
  ] {
    if let Some(value) = flag(args, key) {
      *field = value;
    }
  }
  if let Some(Value::Array(shares)) = args.get("add_shares") {
    for share in shares {
      let path = text(share, "path").ok_or("Each shared folder needs a path.")?;
      if !std::path::Path::new(&path).is_dir() {
        return Err(format!("{path} isn't a folder on this Mac."));
      }
      let name = text(share, "name").unwrap_or_else(|| {
        std::path::Path::new(&path)
          .file_name()
          .map(|n| n.to_string_lossy().to_string())
          .unwrap_or_else(|| "share".into())
      });
      config.shares.retain(|s| s.name != name);
      config.shares.push(Share {
        path,
        name,
        readonly: flag(share, "readonly").unwrap_or(false),
      });
    }
  }
  if let Some(Value::Array(names)) = args.get("remove_shares") {
    let names: Vec<String> = names
      .iter()
      .filter_map(|n| n.as_str().map(str::to_string))
      .collect();
    config.shares.retain(|s| !names.contains(&s.name));
  }
  if flag(args, "eject_installer") == Some(true) {
    if let Some(Installer::Iso { .. }) = config.installer {
      config.installer = None;
    }
  }
  if let Some(disk) = number(args, "disk_gb") {
    let bytes = disk as u64 * 1024 * 1024 * 1024;
    let system = config
      .disks
      .first()
      .cloned()
      .ok_or("This machine has no disk.")?;
    if bytes < system.size {
      return Err("Disks can only grow.".into());
    }
    if running {
      return Err("Shut the machine down before growing its disk.".into());
    }
    std::fs::OpenOptions::new()
      .write(true)
      .open(bundle.path.join(&system.file))
      .and_then(|f| f.set_len(bytes))
      .map_err(|e| e.to_string())?;
    config.disks[0].size = bytes;
  }
  bundle::save(&bundle.path, &bundle.config).map_err(|e| e.to_string())?;
  let store = context.store().await?;
  store.register(&bundle).await.map_err(|e| e.to_string())?;
  Ok(Output::data(
    if running {
      "Saved. The machine is running: changes take effect at its next start (restart_machine)."
    } else {
      "Saved."
    },
    serde_json::to_value(&bundle.config).unwrap_or(Value::Null),
  ))
}

async fn import(context: &Context, args: &Value) -> Result<Output, String> {
  let store = context.store().await?;
  super::allowed(store).await?;
  let path = required(args, "path")?;
  let bundle = bundle::load(std::path::Path::new(&path)).map_err(|e| e.to_string())?;
  store.register(&bundle).await.map_err(|e| e.to_string())?;
  Ok(Output::data(
    format!("Imported {}.", bundle.config.name),
    json!({"id": bundle.config.id, "name": bundle.config.name}),
  ))
}

async fn clone(context: &Context, args: &Value) -> Result<Output, String> {
  let bundle = context.machine(args).await?;
  if vista_kit::ipc::status(&bundle.config.id).await.is_some() {
    return Err(format!(
      "Shut {} down first (stop_machine); a running disk can't be copied safely.",
      bundle.config.name
    ));
  }
  let name = text(args, "name").unwrap_or_else(|| format!("{} copy", bundle.config.name));
  let copy = bundle::duplicate(&bundle, &name).map_err(|e| e.to_string())?;
  let store = context.store().await?;
  store.register(&copy).await.map_err(|e| e.to_string())?;
  if flag(args, "start") == Some(true) {
    vista_kit::launch::start(&copy.path, true).map_err(|e| e.to_string())?;
    vista_kit::launch::wait_ready(&copy.config.id, std::time::Duration::from_secs(30)).await;
  }
  Ok(Output::data(
    format!("Cloned {} as {}.", bundle.config.name, copy.config.name),
    json!({"id": copy.config.id, "name": copy.config.name, "path": copy.path}),
  ))
}

async fn delete(context: &Context, args: &Value) -> Result<Output, String> {
  if super::flag(args, "confirm") != Some(true) {
    return Err("Pass confirm=true to delete a machine.".into());
  }
  let bundle = context.machine(args).await?;
  required(args, "machine")?;
  if vista_kit::ipc::status(&bundle.config.id).await.is_some() {
    return Err(format!("{} is running. Stop it first.", bundle.config.name));
  }
  bundle::delete(&bundle).map_err(|e| e.to_string())?;
  let store = context.store().await?;
  store
    .forget(&bundle.config.id)
    .await
    .map_err(|e| e.to_string())?;
  Ok(Output::text(format!(
    "Moved {} to the Trash.",
    bundle.config.name
  )))
}
