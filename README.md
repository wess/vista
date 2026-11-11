# Vista

Virtual machines for Apple silicon, on Apple's own Virtualization framework.
Vista downloads and installs macOS, Linux and the BSDs (and, experimentally,
Windows 11 for Arm) for you, runs them side by side, and lets coding agents
drive them — from the outside and from the inside — through a built-in MCP
server.

No accounts, no licence keys, no telemetry, no splash screens. MIT licensed.

![Vista](assets/icon.png)

## What it does

- **One-click systems.** Pick a system and Vista resolves the current
  installer from its publisher at that moment, downloads it (resumable),
  checks its SHA-256, keeps it for next time, builds the machine and boots it.
  - macOS: latest, 26, 15, 14, 13 — restore images from Apple. On a macOS 27
    host Vista creates the first account and skips Setup Assistant.
  - Linux: Ubuntu (26.04 / 24.04, Desktop and Server), Fedora, Debian, Kali,
    CentOS Stream, Rocky, AlmaLinux, openSUSE Tumbleweed, Pop!_OS, NixOS,
    Alpine.
  - BSD: FreeBSD, OpenBSD, NetBSD.
  - Windows 11 for Arm (experimental): the ISO comes from Microsoft's own
    download service; Vista builds an install disk and installs unattended
    with a local account. See [Windows](#windows).
  - Or any Arm64 ISO, or a `.ipsw`, already on your Mac.
- **Machines that behave.** Each machine runs in its own process
  (`vistamachine`), so a crash takes down one machine and quitting Vista
  leaves them running. Suspend writes memory to disk and resumes in seconds.
  Shared folders (virtio-fs), clipboard (spice-vdagent), Rosetta for Linux,
  sound and microphone, nested virtualization, NAT networking with the
  guest's IP shown, Secure Boot on macOS 27 hosts.
- **Snapshots and clones** are APFS copy-on-write: instant, and they take
  space only as the machine changes. A snapshot of a running machine keeps
  its memory, so reverting resumes exactly there.
- **Menu bar quick view** of every machine, with start and stop.
- **Agents.** `vistamcp` is an MCP server with 43 tools — see below.

## Requirements

- A Mac with Apple silicon.
- macOS 15 or later. macOS 27 adds Secure Boot and automatic first-account
  setup for macOS guests.
- Rust 1.94+ to build.

## Building

```sh
cargo build                 # all crates, debug
scripts/tools.sh            # Vista Tools for Linux and macOS guests -> target/tools
scripts/sign.sh             # sign target/debug binaries with the virtualization entitlement
target/debug/vista          # run

scripts/bundle.sh           # release build -> dist/Vista.app (ad-hoc signed)
scripts/dmg.sh              # dist/Vista.dmg
CODESIGN_IDENTITY="Developer ID Application: …" scripts/bundle.sh   # for distribution
```

The Virtualization framework refuses to run unsigned code: anything that
creates a machine must carry `com.apple.security.virtualization`
(`assets/vista.entitlements`). `scripts/sign.sh` does that for debug builds.

Vista Tools for Linux guests is a static musl binary; it cross-compiles on
the Mac with `rust-lld` and no extra toolchain (`.cargo/config.toml`). It
needs the `aarch64-unknown-linux-musl` target:
`rustup target add aarch64-unknown-linux-musl`.

`cargo test --workspace` runs the unit tests. Two network tests check every
catalog entry and the Windows download against the live publishers:

```sh
cargo test -p vista-kit --test catalog -- --ignored --nocapture
```

For development, `VISTA_SUPPORT=<dir>` points every Vista process at a
scratch library instead of `~/Library/Application Support/Vista`.

## Agents (MCP)

Vista ships an MCP server, `vistamcp` (stdio). Point any MCP-capable agent at
the binary — inside the app bundle that's
`/Applications/Vista.app/Contents/MacOS/vistamcp` — or add it to the agent's
configuration:

```json
{
  "mcpServers": {
    "vista": { "command": "/Applications/Vista.app/Contents/MacOS/vistamcp" }
  }
}
```

Settings → Agents shows the exact path and turns agent control off for the
whole Mac; each machine's Sharing tab turns it off for that machine.

**Outside the machine**

| | |
| --- | --- |
| Library | `list_machines`, `get_machine`, `list_systems`, `create_machine`, `import_machine`, `clone_machine`, `configure_machine`, `delete_machine` |
| Creation jobs | `get_job` (can block with `wait_seconds`), `list_jobs`, `cancel_job` |
| Power | `start_machine`, `stop_machine` (graceful or forced), `restart_machine`, `pause_machine`, `resume_machine`, `suspend_machine`, `show_machine`, `hide_machine` |
| Snapshots | `take_snapshot` (live, with memory), `list_snapshots`, `revert_snapshot`, `rename_snapshot`, `delete_snapshot` |

**Inside, through the screen** — works with every guest, nothing to install:
`screenshot`, `click`, `double_click`, `right_click`, `move_mouse`, `drag`,
`scroll`, `type_text`, `press_keys`. Pointer coordinates are screenshot
pixels; every input tool can return a fresh screenshot after acting.

**Inside, through Vista Tools** — Linux and macOS guests:
`guest_info`, `run_command` (exit code, stdout, stderr, timeout, as the
console user or root), `read_file`, `write_file`, `list_directory`,
`file_info`, `make_directory`, `delete_path`, `upload_file`,
`download_file`.

Vista Tools is a small agent that listens on virtio-vsock inside the guest.
Every machine sees it on a read-only share; install it once:

```sh
# Linux (systemd, OpenRC, or neither)
sudo mount -t virtiofs vistatools /mnt && sudo /mnt/install.sh
# macOS: open "/Volumes/My Shared Files/Vista Tools/install.command"
```

The Linux installer also mounts Vista's shared folders at `/mnt/vista` on
every boot.

Machine creation runs as a background job (`vistamcp job <id>`) that outlives
the agent session, and shows up in Vista's sidebar with its progress.

## How it's built

Rust, [gpui](https://github.com/zed-industries/zed) and
[guise](https://github.com/wess/guise) for the interface, tokio for
everything off the main thread, sqlx + SQLite for the library index, objc2
for the Virtualization framework and AppKit.

```text
crates/
  kit/      vista-kit      machine model, bundles, snapshots, SQLite store,
                           installer catalog + resolver, downloads, jobs,
                           Windows media, control protocol (no AppKit, no gpui)
  guest/    vista-guest    the protocol between Vista and Vista Tools
  vz/       vista-vz       bundle -> VZVirtualMachineConfiguration, macOS
                           install, macOS 26/27 additions by selector
  machine/  vista-machine  `vistamachine`: runs one machine, its window, its
                           control socket, screenshots and input injection
  mcp/      vista-mcp      `vistamcp`: the MCP server
  tools/    vista-tools    `vistatools`: Vista Tools, inside the guest
  app/      vista-app      `vista`: the library window and menu bar item
```

A machine is a `.vista` folder: `config.json`, a sparse `disk.img`, firmware
state, and `snapshots/`. Copy one to another Mac and add it with
File → Add Existing Machine. The SQLite database only remembers where
bundles are, plus settings and the installer cache.

Each `vistamachine` listens on a Unix socket in `/tmp/vista<uid>/`; the app,
the menu bar and `vistamcp` all speak the same JSON-lines protocol to it.

## Limits worth knowing

- **Windows** <a id="windows"></a> is experimental. Apple's framework doesn't
  support Windows; booting it relies on unpublished framework switches (CPU
  counter and trap emulation, a linear framebuffer), applied only when the
  host has them. The ISO download and install disk are built and tested; a
  full install on this framework has not been verified. Vista Tools isn't
  available inside Windows; agents drive it through the screen tools.
- **Bridged networking** needs Apple's `com.apple.vm.networking` entitlement,
  which Apple grants per developer. Shared (NAT) networking always works.
- A guest **reboot** powers the machine off (the framework reports both the
  same way). While an installer ISO is attached, Vista treats that power-off
  as the end of installation, ejects the ISO and boots from disk.
- **Screenshots** need the machine's window on screen (a hidden machine is
  shown first). macOS allows at most **two macOS guests** at once.
