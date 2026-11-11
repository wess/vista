# Architecture

## Processes

```text
                    ┌──────────────┐        ┌────────────────┐
  you ─────────────▶│ vista        │        │ vistamcp       │◀──── agent (stdio)
                    │ window, menu │        │ MCP server     │
                    │ bar, jobs    │        │ + `job <id>`   │
                    └──────┬───────┘        └───────┬────────┘
                           │  JSON lines over       │
                           │  /tmp/vista<uid>/*.sock│
               ┌───────────┴───────────┬────────────┴─────────┐
               ▼                       ▼                      ▼
        ┌──────────────┐        ┌──────────────┐       ┌──────────────┐
        │ vistamachine │        │ vistamachine │  …    │ vistamachine │
        │ one machine, │        │              │       │              │
        │ its window   │        │              │       │              │
        └──────┬───────┘        └──────────────┘       └──────────────┘
               │ virtio-vsock :7000
               ▼
        ┌──────────────┐
        │ vistatools   │  inside the guest
        └──────────────┘
```

- **`vista`** is the library window and the menu bar item. It never runs a
  machine itself: it starts `vistamachine` processes and talks to them. It
  runs creation jobs on its own tokio runtime.
- **`vistamachine`** runs exactly one machine. It owns the
  `VZVirtualMachine`, its window, and a control socket. It's started detached
  in its own session, so quitting Vista (or the agent that started it) leaves
  the machine running, and a crash takes down one machine and nothing else.
  When the machine stops, the process exits.
- **`vistamcp`** is the MCP server an agent launches. It reads the same
  library and speaks the same socket protocol as Vista. `vistamcp job <id>`
  is the same binary working a creation job in the background.
- **`vistatools`** runs inside Linux and macOS guests and answers on vsock.

Nothing is a daemon. A machine's process exists while the machine runs;
everything else exists while someone has it open.

## Crates

| Directory | Package | What it is |
| --- | --- | --- |
| `crates/kit` | `vista-kit` | Everything every process agrees on: the machine model, bundles and snapshots, the SQLite library, the installer catalog and resolver, downloads, creation jobs, Windows install media, launching machines, the control protocol and its client and server. No AppKit, no gpui. |
| `crates/guest` | `vista-guest` | The Vista Tools protocol, on its own so `vistatools` builds without SQLite or TLS. |
| `crates/vz` | `vista-vz` | The bridge to Apple's Virtualization framework through objc2: a bundle to a `VZVirtualMachineConfiguration`, macOS installation, and the macOS 26/27 additions (Secure Boot, guest provisioning) reached by selector after checking the host answers. |
| `crates/machine` | `vista-machine` | `vistamachine`. |
| `crates/mcp` | `vista-mcp` | `vistamcp`. |
| `crates/tools` | `vista-tools` | `vistatools`, built as a static musl binary for Linux and a native one for macOS guests. |
| `crates/app` | `vista-app` | `vista`, in gpui with [guise](https://github.com/wess/guise) components. |

### Inside `vistamachine`

The machine, its view and every framework callback live on the main thread,
where the machine was created. The control socket is served by a tokio
runtime on a background thread; a request that touches the machine hops to
the main queue, and the answer comes back over a oneshot channel.

- `lifecycle.rs` — boot (installing macOS first if needed), start or
  restore, stop, pause, suspend, restart, live snapshots, what happens when
  the guest powers off.
- `window.rs` — the machine's window and the macOS install progress window.
- `capture.rs` — screenshots with ScreenCaptureKit's per-process content,
  which needs no Screen Recording permission. The title bar is cropped so the
  image is exactly the guest's screen.
- `input.rs` — keyboard and pointer events built with Core Graphics and
  handed to the `VZVirtualMachineView`. Modifier keys need the
  device-dependent bits real keyboards set (left shift, left command…), or
  the view ignores them.
- `guest.rs` — connects to vsock port 7000 through the machine's socket
  device and relays one request to Vista Tools.

### Inside `vista`

- `state/` — the `Library` entity: machines, jobs, settings, what's selected.
  It reads the database and asks every machine's socket for its state once a
  second, on tokio, and fetches a screen thumbnail of the selected machine
  every three seconds. The first read happens before the window opens, so the
  first frame is the real library.
- `ui/` — the window: toolbar, sidebar, the machine page and its tabs, the
  New Machine flow, Settings.
- `statusbar/` — the menu bar item. gpui has no status item API, so the item
  is AppKit through objc2; its click is forwarded over a channel to a gpui
  task that opens the quick view, a gpui pop-up window.

## The machine bundle

A machine is a folder ending in `.vista`:

```text
Ubuntu Dev.vista/
  config.json          the machine's hardware — the source of truth
  disk.img             the system disk: a sparse raw image
  nvram                UEFI variables (Linux, BSD, Windows)
  auxiliary            boot storage (macOS)
  hardwaremodel        the Mac hardware model macOS was installed for
  machineid            the platform's machine identifier
  suspended.vzvmsave   memory, while suspended
  install.img          the Windows install disk, while installing
  machine.log          the machine process's log
  serial.log           the serial console (Linux, BSD)
  snapshots/<id>/      copy-on-write clones of the above + snapshot.json
```

`config.json` carries everything needed to rebuild the hardware: id, name,
OS and family, processors, memory (MiB), disks, display, network mode and MAC
address, shared folders, sound, microphone, Rosetta, clipboard, Secure Boot,
nested virtualization, autostart, agent control, and the installer media
still attached. It has a `format` number; a Vista that finds a newer format
refuses to open the machine rather than misread it.

## The library database

`~/Library/Application Support/Vista/vista.db`, SQLite through sqlx:

| Table | Holds |
| --- | --- |
| `machines` | Where each bundle is, its name, OS and family (for drawing the sidebar without opening every bundle), sidebar order, last run. |
| `settings` | One row per preference, values as JSON, so a newer Vista can add a setting without a migration. |
| `images` | Downloaded installers kept for reuse: catalog entry, version, URL, path, size, SHA-256. |

The database is an index, not the truth: a bundle copied in from another Mac
is a whole machine, and bundles found in the machines location are added at
launch. Schema steps are embedded in the binary and applied in order,
tracked by SQLite's `user_version`.

## Creation jobs

A job is a JSON file in `~/Library/Application Support/Vista/jobs/`,
rewritten as it moves through its phases (queued, resolving, downloading,
verifying, preparing, creating, starting, then done, failed or cancelled).
The app and `vistamcp` run jobs with the same code and read the same files,
which is how a job an agent started shows in the sidebar. Cancelling writes a
marker file next to the job; the worker notices within a second.

## Starting a machine

`kit::launch::start` finds `vistamachine` — `$VISTA_MACHINE`, then next to
the current executable, then `../Helpers/VistaMachine.app` inside the app
bundle — and spawns it detached (`setsid`), with output appended to the
machine's `machine.log`. `--hidden` starts it without a window.
