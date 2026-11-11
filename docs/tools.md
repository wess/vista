# MCP tool reference

Every tool `vistamcp` offers, with its arguments. It mirrors the server's own `tools/list` answer, so it says exactly what an agent sees; update it when a tool changes. For setup and how the pieces fit, start with [Agents](agents.md).

Every per-machine tool takes `machine`: a name (any case), an id, or the first 8+ characters of an id. Tools that change something are marked destructive where they can lose data.

## Contents

- **Library**: [`list_machines`](#list_machines), [`get_machine`](#get_machine), [`list_systems`](#list_systems), [`import_machine`](#import_machine), [`configure_machine`](#configure_machine), [`clone_machine`](#clone_machine), [`delete_machine`](#delete_machine)
- **Creating machines**: [`create_machine`](#create_machine), [`get_job`](#get_job), [`list_jobs`](#list_jobs), [`cancel_job`](#cancel_job)
- **Power and windows**: [`start_machine`](#start_machine), [`stop_machine`](#stop_machine), [`restart_machine`](#restart_machine), [`pause_machine`](#pause_machine), [`resume_machine`](#resume_machine), [`suspend_machine`](#suspend_machine), [`show_machine`](#show_machine), [`hide_machine`](#hide_machine)
- **Snapshots**: [`take_snapshot`](#take_snapshot), [`list_snapshots`](#list_snapshots), [`revert_snapshot`](#revert_snapshot), [`rename_snapshot`](#rename_snapshot), [`delete_snapshot`](#delete_snapshot)
- **Screen, keyboard and pointer**: [`screenshot`](#screenshot), [`click`](#click), [`double_click`](#double_click), [`right_click`](#right_click), [`move_mouse`](#move_mouse), [`drag`](#drag), [`scroll`](#scroll), [`type_text`](#type_text), [`press_keys`](#press_keys)
- **Inside the guest (Vista Tools)**: [`guest_info`](#guest_info), [`run_command`](#run_command), [`read_file`](#read_file), [`write_file`](#write_file), [`list_directory`](#list_directory), [`file_info`](#file_info), [`make_directory`](#make_directory), [`delete_path`](#delete_path), [`upload_file`](#upload_file), [`download_file`](#download_file)

## Library

### list_machines

*List machines* · read-only

Every virtual machine in the Vista library with its state (running, stopped, suspended, …), operating system, hardware (CPUs, memory, disk size and the disk space it really uses on the Mac), IP address when running, and whether agent control is allowed. Start here to learn names and ids.

No arguments.

### get_machine

*Get machine details* · read-only

Everything about one machine: full configuration (CPU, memory, disks, display, network mode and MAC address, shared folders, audio, Rosetta, clipboard, Secure Boot), live state (state, uptime, IP, window visible, install progress), its snapshots, whether an installer is still attached, the bundle path on the Mac, and how to reach the guest.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |

### list_systems

*List installable systems* · read-only

The operating systems Vista can download and install in one step, with the id to pass to create_machine. Includes macOS (latest and earlier majors, from Apple), Windows 11 for Arm (from Microsoft; experimental), and Linux/BSD distributions (Ubuntu, Fedora, Debian, Kali, CentOS Stream, Rocky, AlmaLinux, openSUSE, Pop!_OS, NixOS, Alpine, FreeBSD, OpenBSD, NetBSD). Each entry says whether it boots to a desktop and any tips for its installer. Optionally filter by os (macos, windows, linux, other) or a search word.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `os` | `macos` \| `windows` \| `linux` \| `other` |  | Only systems of this kind. |
| `search` | string |  | Only systems whose name, family or edition contains this (case-insensitive). |

### import_machine

*Import a machine*

Add an existing Vista machine bundle (a .vista folder, e.g. copied from another Mac) to the library.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `path` | string | yes | Absolute path to the .vista folder. |

### configure_machine

*Change machine settings*

Change a machine's hardware and options. Any field left out is unchanged. Changes to a running machine are saved and take effect at its next start (restart_machine applies them). Disks can only grow. Returns the new configuration.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `add_shares` | array |  | Mac folders to share into the guest. Each item: `name`, `path`, `readonly`. |
| `audio` | boolean |  | Sound output. |
| `autostart` | boolean |  | Start in the background when Vista opens. |
| `clipboard` | boolean |  | Linux only: clipboard sharing through spice-vdagent. |
| `cpus` | integer |  | Virtual CPU cores. |
| `disk_gb` | integer |  | Grow the system disk to this many GB (the guest must then extend its partition). |
| `display_height` | integer |  | Display height in pixels. |
| `display_width` | integer |  | Display width in pixels (the window still resizes it while running). |
| `eject_installer` | boolean |  | Detach the installer media (ISO) at the next start. |
| `memory_gb` | number |  | Memory in GB. |
| `microphone` | boolean |  | Give the guest the Mac's microphone. |
| `name` | string |  | New display name. |
| `nested` | boolean |  | Nested virtualization (M3 or later). |
| `network` | string |  | "shared" (NAT through the Mac, default), "none", or "bridged:<interface>" e.g. "bridged:en0" (needs Apple's bridging entitlement). |
| `remove_shares` | array |  | Names of shared folders to stop sharing. |
| `rosetta` | boolean |  | Linux only: share Rosetta so the guest can run x86-64 Linux binaries (mount with `mount -t virtiofs rosetta /media/rosetta`, then register binfmt). |

### clone_machine

*Clone a machine*

Make a copy of a stopped machine. On APFS the copy shares blocks with the original, so it's instant and uses no space until either changes. The clone gets a new id, MAC address and machine identity; snapshots aren't copied.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `name` | string |  | Name for the copy (default: "<name> copy"). |
| `start` | boolean |  | Start the clone right away. |

### delete_machine

*Delete a machine* · destructive

Move a stopped machine's bundle (disks, snapshots, everything) to the Trash and remove it from the library. Irreversible from Vista's side. Requires confirm=true.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `confirm` | boolean | yes | Must be true. |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |

## Creating machines

### create_machine

*Create a machine*

Create a new virtual machine. Give either `system` (an id from list_systems: Vista downloads the installer from its publisher, verifies its checksum and keeps it for next time) or `installer_path` (an .iso or .ipsw already on this Mac). Runs as a background job and returns its id at once; poll get_job (wait_seconds lets you block). With start=true (default) the machine boots into its installer when ready: macOS installs by itself (and with username/password on a macOS 27 host, creates the account and skips Setup Assistant); Linux/BSD installers run in the window — drive them with screenshot, click, type_text and press_keys; Windows 11 (experimental) installs unattended with the given account. Sensible hardware defaults are chosen per OS.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `cpus` | integer |  | Virtual CPU cores (default 4; 2 for BSDs). |
| `disk_gb` | integer |  | System disk in GB (default 96 macOS, 80 Windows, 64 Linux). Sparse: only what the guest writes uses space. |
| `enable_ssh` | boolean |  | macOS: turn on Remote Login for the first account. |
| `fullname` | string |  | First account's full name. |
| `installer_path` | string |  | Absolute path to an .iso or .ipsw on this Mac, instead of `system`. |
| `memory_gb` | number |  | Memory in GB (default 8 for macOS/Windows, 4 for Linux). |
| `name` | string |  | Display name. Defaults to the system's name. |
| `os` | `macos` \| `linux` \| `windows` \| `other` |  | With installer_path: what it installs (guessed from the file name otherwise). |
| `password` | string |  | First account's password. |
| `rosetta` | boolean |  | Linux: share Rosetta for x86-64 binaries. |
| `share_downloads` | boolean |  | Share the Mac's Downloads folder into the guest (default true). |
| `start` | boolean |  | Start the machine once it's built (default true). |
| `system` | string |  | Catalog id from list_systems, e.g. "ubuntudesktop2604", "macoslatest", "fedoraworkstation", "windows11". |
| `username` | string |  | First account's short name (macOS on a macOS 27 host; Windows). |
| `visible` | boolean |  | Open its window when started (default true; needed for screenshots). |
| `wait_seconds` | integer |  | Wait up to this long for the job to finish before answering (default 0). |

### get_job

*Get creation job* · read-only

Progress of a create_machine job: its phase (resolving, downloading, verifying, preparing, creating, starting, done, failed, cancelled), bytes downloaded and expected, percentage, a message, and the new machine's id once it exists. Pass wait_seconds to block until the job finishes or the time runs out.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `job` | string | yes | The job id create_machine returned (or its first 8 characters). |
| `wait_seconds` | integer |  | Block up to this long for the job to finish. |

### list_jobs

*List creation jobs* · read-only

Recent machine creation jobs (from agents and from Vista's own window), newest first, with their phase and progress.

No arguments.

### cancel_job

*Cancel creation job*

Stop a running create_machine job. A partly downloaded installer is kept, so creating the same system again resumes the download.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `job` | string | yes | The job id. |

## Power and windows

### start_machine

*Start a machine*

Power on a machine (or resume it if suspended). Each machine runs in its own process; this returns once that process answers, with the machine's state. The guest OS still needs time to boot — take a screenshot to see where it is.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `visible` | boolean |  | Open its window (default true). Screenshots and pointer tools need a visible window; a hidden machine can be shown later with show_machine. |

### stop_machine

*Stop a machine* · destructive

Shut a machine down. By default this presses the guest's power button so it shuts down cleanly (the guest decides; macOS may ask about unsaved work). force=true pulls the plug immediately, like holding the power button — unsaved data in the guest is lost. wait=true (default) waits up to 60 seconds for it to finish.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `force` | boolean |  | Stop immediately without asking the guest (default false). |
| `wait` | boolean |  | Wait for the machine to be off (default true). |

### restart_machine

*Restart a machine*

Ask the guest to shut down, then start it again with its current saved configuration (so configure_machine changes take effect). Falls back to a hard restart if the guest can't be asked.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |

### pause_machine

*Pause a machine*

Freeze a running machine in place: its CPUs stop, memory stays. resume_machine continues exactly where it was.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |

### resume_machine

*Resume a machine*

Continue a paused machine.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |

### suspend_machine

*Suspend a machine*

Save the machine's memory to disk and quit its process. The next start_machine resumes it exactly where it was, in seconds. Not every configuration supports it.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |

### show_machine

*Show a machine's window*

Bring a running machine's window to the front (opening it if the machine was started hidden). Needed before screenshots of a hidden machine.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |

### hide_machine

*Hide a machine's window*

Hide a running machine's window; it keeps running in the background.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |

## Snapshots

### take_snapshot

*Take a snapshot*

Save the machine's current state so you can come back to it. Works stopped or running: for a running machine the guest is paused for a moment and its memory is saved too, so reverting resumes exactly here (open apps and all). Copy-on-write: instant, and costs disk space only as the machine changes afterwards.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `name` | string |  | A name, e.g. "before upgrade" (default: Snapshot N). |

### list_snapshots

*List snapshots* · read-only

A machine's snapshots, newest first, with id, name, when it was taken, and whether it includes memory (taken while running).

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |

### revert_snapshot

*Revert to a snapshot* · destructive

Put the machine back exactly as it was when the snapshot was taken, discarding everything since. The machine must be off; stop_first=true force-stops it first. A snapshot taken while running leaves the machine suspended, so start_machine resumes into it. Consider take_snapshot first if the current state may be wanted.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `snapshot` | string | yes | The snapshot's id (or first 8 characters) or its name, from list_snapshots. |
| `start` | boolean |  | Start the machine after reverting (default false). |
| `stop_first` | boolean |  | Force-stop the machine if it's running (default false). |

### rename_snapshot

*Rename a snapshot*

Give a snapshot a new name.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `name` | string | yes |  |
| `snapshot` | string | yes | The snapshot's id (or first 8 characters) or its name, from list_snapshots. |

### delete_snapshot

*Delete a snapshot* · destructive

Remove a snapshot and free the space only it was using. The machine's current state is not affected.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `snapshot` | string | yes | The snapshot's id (or first 8 characters) or its name, from list_snapshots. |

## Screen, keyboard and pointer

### screenshot

*Take a screenshot* · read-only

A PNG of the machine's display, exactly as a person would see it. The image's pixel size is the coordinate space for click/drag/scroll (origin top-left). The window is shown first if the machine runs hidden. Take one after starting a machine, and after acting, to check what happened.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |

### click

*Click*

Move the pointer to (x, y) in screenshot pixels and click the left button.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `x` | number | yes | Pixels from the left edge of the screenshot. |
| `y` | number | yes | Pixels from the top edge of the screenshot. |
| `screenshot` | boolean |  | Return a screenshot after acting (default false). |
| `settle_ms` | integer |  | With screenshot: milliseconds to wait before taking it, so the guest can react (default 600). |

### double_click

*Double-click*

Double-click the left button at (x, y), e.g. to open a file or select a word.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `x` | number | yes | Pixels from the left edge of the screenshot. |
| `y` | number | yes | Pixels from the top edge of the screenshot. |
| `screenshot` | boolean |  | Return a screenshot after acting (default false). |
| `settle_ms` | integer |  | With screenshot: milliseconds to wait before taking it, so the guest can react (default 600). |

### right_click

*Right-click*

Click the right (secondary) button at (x, y) to open a context menu.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `x` | number | yes | Pixels from the left edge of the screenshot. |
| `y` | number | yes | Pixels from the top edge of the screenshot. |
| `screenshot` | boolean |  | Return a screenshot after acting (default false). |
| `settle_ms` | integer |  | With screenshot: milliseconds to wait before taking it, so the guest can react (default 600). |

### move_mouse

*Move the pointer*

Move the pointer to (x, y) without clicking — to hover menus and tooltips.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `x` | number | yes | Pixels from the left edge of the screenshot. |
| `y` | number | yes | Pixels from the top edge of the screenshot. |
| `screenshot` | boolean |  | Return a screenshot after acting (default false). |
| `settle_ms` | integer |  | With screenshot: milliseconds to wait before taking it, so the guest can react (default 600). |

### drag

*Drag*

Press the left button at (x, y), move to (to_x, to_y) and release: dragging windows, files, sliders, or selecting text.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `to_x` | number | yes | End point, pixels from the left. |
| `to_y` | number | yes | End point, pixels from the top. |
| `x` | number | yes | Pixels from the left edge of the screenshot. |
| `y` | number | yes | Pixels from the top edge of the screenshot. |
| `screenshot` | boolean |  | Return a screenshot after acting (default false). |
| `settle_ms` | integer |  | With screenshot: milliseconds to wait before taking it, so the guest can react (default 600). |

### scroll

*Scroll*

Scroll at (x, y). dy > 0 scrolls down (content moves up), dy < 0 up; dx scrolls sideways. Units are pixels; 300–600 is roughly a page of text.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `x` | number | yes | Pixels from the left edge of the screenshot. |
| `y` | number | yes | Pixels from the top edge of the screenshot. |
| `dx` | number |  | Horizontal pixels (default 0). |
| `dy` | number |  | Vertical pixels (default 0). |
| `screenshot` | boolean |  | Return a screenshot after acting (default false). |
| `settle_ms` | integer |  | With screenshot: milliseconds to wait before taking it, so the guest can react (default 600). |

### type_text

*Type text*

Type text into whatever has keyboard focus in the guest, one keystroke per character on a US layout. Newlines press Return and tabs press Tab. Characters with no key on a US keyboard (é, emoji) are skipped and reported. Click a field first if needed.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `text` | string | yes | What to type. |
| `screenshot` | boolean |  | Return a screenshot after acting (default false). |
| `settle_ms` | integer |  | With screenshot: milliseconds to wait before taking it, so the guest can react (default 600). |

### press_keys

*Press keys*

Press a key or shortcut. Examples: "enter", "escape", "tab", "backspace", "delete" (backspace), "forwarddelete", "up", "pagedown", "f5", "cmd+c", "cmd+shift+t", "ctrl+alt+delete", "alt+f4", "cmd+space". Modifiers: cmd (Command on macOS, Windows/Super key elsewhere), ctrl, alt/option, shift. Several presses separated by spaces run in order: "cmd+a backspace".

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `keys` | string | yes | Keys to press, e.g. "cmd+s" or "ctrl+c enter". |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `screenshot` | boolean |  | Return a screenshot after acting (default false). |
| `settle_ms` | integer |  | With screenshot: milliseconds to wait before taking it, so the guest can react (default 600). |

## Inside the guest (Vista Tools)

### guest_info

*Guest information* · read-only

Ask Vista Tools inside the guest who and what it is: OS, version, kernel, architecture, hostname, and the user it runs commands as. A quick way to check Vista Tools is installed and answering.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |

### run_command

*Run a command in the guest*

Run a shell command inside the guest and get its exit code, stdout and stderr (each truncated to 1 MB). Runs through /bin/sh -c by default, so pipes, globs and && work. Commands run as the guest's console user unless admin=true (root). Interactive programs that wait for input will hang until the timeout; pass stdin or use non-interactive flags (apt-get -y, DEBIAN_FRONTEND=noninteractive). Needs Vista Tools in the guest (Linux, macOS).

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `command` | string | yes | The command line, e.g. "uname -a" or "cd /srv && ls -la". |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `admin` | boolean |  | Run as root (default false). |
| `cwd` | string |  | Working directory (default: the user's home). |
| `env` | object |  | Extra environment variables. |
| `shell` | boolean |  | Run through /bin/sh -c (default true). false splits the command on spaces and runs it directly. |
| `stdin` | string |  | Text to feed to the command's standard input. |
| `timeout` | integer |  | Seconds before the command is killed (default 120). |

### read_file

*Read a guest file* · read-only

Read a file inside the guest. Text comes back as text; binary files (or encoding=base64) come back base64-encoded. Up to 64 MB per call; use offset/length to page through bigger files, or download_file to copy one to the Mac.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `path` | string | yes | Absolute path in the guest. |
| `encoding` | `auto` \| `text` \| `base64` |  | auto (default): text when it's valid UTF-8. |
| `length` | integer |  | Bytes to read. |
| `offset` | integer |  | Byte offset to start at. |

### write_file

*Write a guest file*

Create or overwrite a file inside the guest (parent directories are created). content is text unless encoding=base64. append=true adds to the end. mode sets permissions, e.g. "755" for a script.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `content` | string | yes | The file's contents. |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `path` | string | yes | Absolute path in the guest. |
| `append` | boolean |  |  |
| `encoding` | `text` \| `base64` |  | How content is encoded (default text). |
| `mode` | string |  | Octal permissions, e.g. "644" or "755". |

### list_directory

*List a guest directory* · read-only

The entries of a directory inside the guest with kind (file, dir, link), size, permissions and modification time.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `path` | string | yes | Absolute directory path in the guest. |

### file_info

*Guest file info* · read-only

Whether a path exists inside the guest, and its kind, size, permissions and modification time.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `path` | string | yes | Absolute path in the guest. |

### make_directory

*Make a guest directory*

Create a directory (and its parents) inside the guest.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `path` | string | yes | Absolute directory path in the guest. |

### delete_path

*Delete a guest path* · destructive

Delete a file, or a directory with recursive=true, inside the guest.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |
| `path` | string | yes | Absolute path in the guest. |
| `recursive` | boolean |  |  |

### upload_file

*Copy a file into the guest*

Copy a file from this Mac into the guest, in chunks (any size). For whole folders, share the folder instead (configure_machine add_shares) or archive it first.

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `guest_path` | string | yes | Absolute destination path in the guest. |
| `host_path` | string | yes | Absolute path on the Mac. |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |

### download_file

*Copy a file out of the guest*

Copy a file from the guest to this Mac, in chunks (any size).

| Argument | Type | Required | Description |
| --- | --- | --- | --- |
| `guest_path` | string | yes | Absolute path in the guest. |
| `host_path` | string | yes | Absolute destination path on the Mac. |
| `machine` | string | yes | The machine: its name (case-insensitive), its id, or the first 8+ characters of its id. list_machines shows them. |

