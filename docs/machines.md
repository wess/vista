# Creating and running machines

## Making a machine

Choose **New Machine** — in the toolbar, from
the menu bar panel, or with ⌘N.

### 1. Choose a system

- **macOS** — the latest release, or the newest of an earlier major (26, 15,
  14, 13). Restore images come from Apple.
- **Windows 11** — experimental; see [Windows](windows.md).
- **Linux and BSD** — one card per distribution. A distribution with several
  editions (Ubuntu Desktop and Server, Fedora Workstation and the network
  installer, …) opens a list of them with a line about each. The full list is
  on [Systems](systems.md).
- **Use an Installer on This Mac…** — any Arm64 `.iso`, or a macOS `.ipsw`.
  Vista works out what it installs from the file: `.ipsw` is macOS, a name
  containing "win" is Windows, one containing "bsd" is a BSD, anything else is
  treated as Linux.

### 2. Set it up

| Field | Notes |
| --- | --- |
| Name | Shown in the sidebar, the menu bar and the machine's window. |
| Processors, Memory, Disk | Defaults depend on the system (below). The disk is sparse, so a generous size costs nothing until the guest writes. Each system has a minimum disk size its installer needs. |
| First account | macOS and Windows. On a macOS 27 host, Vista creates this account in a new macOS machine and skips Setup Assistant; leave the password empty to go through Setup Assistant yourself. Windows always installs unattended with this account. |
| Share Downloads | The Mac's Downloads folder appears inside the guest. The default comes from Settings. |
| Rosetta | Linux: lets the guest run x86-64 Linux programs. On by default when Rosetta is installed on the Mac. |
| Remote Login | macOS: turns on SSH for the first account. |
| Start when ready | Boot the installer as soon as the machine is built. |

| System | Processors | Memory | Disk | Smallest disk |
| --- | --- | --- | --- | --- |
| macOS | 4 | 8 GB | 96 GB | 64 GB |
| Windows | 4 | 8 GB | 80 GB | 64 GB |
| Linux | 4 | 4 GB | 64 GB | 16 GB |
| BSD and others | 2 | 2 GB | 32 GB | 4 GB |

A macOS restore image can require more processors or memory than you chose;
Vista raises them to the image's minimum when it installs.

### 3. Create

**Create Machine** starts a job, which appears under **Creating** in the
sidebar with what it's doing and how far along it is:

1. **Finding the installer** — asks the publisher for the current file (see
   [Systems](systems.md)).
2. **Downloading** — resumable. A download interrupted by a quit, a crash or a
   lost connection continues from where it stopped next time.
3. **Verifying** — the SHA-256 the publisher lists, where there is one.
4. **Preparing** — Windows only: building the install disk.
5. **Creating** — the machine's folder and its sparse disk.
6. **Starting** — if you asked for it.

The ✕ on a job cancels it; on a failed job it dismisses it. A failed job keeps
its message until you dismiss it. When a job finishes, Vista selects the new
machine.

Installers are kept (Settings → Downloads), so a second machine of the same
system skips the download.

Jobs run in their own task and write their progress to
`~/Library/Application Support/Vista/jobs/`. A job an agent started shows
up in the sidebar exactly like one you started.

## Installing the system

### macOS

The machine process installs macOS from the restore image before the first
boot, showing a progress window (*Installing macOS… 42%*). This takes
several minutes. When it finishes the machine boots, and on a macOS 27 host
with a first account given, it signs straight in.

### Linux and the BSDs

The ISO is plugged in as a USB drive and the machine boots from it, so the
distribution's own installer runs in the machine's window. Each system's
note in the picker says what to expect at its boot menu.

When an installer finishes it powers the machine off or reboots it — which
on this framework is the same thing. If the guest wrote more than 3 GB to its
disk while the installer was attached, Vista takes that as a finished install:
it ejects the ISO and boots from the disk. Below that, the machine simply
stops and keeps the installer, so a live session you shut down can be booted
again.

To remove the installer yourself, use **Eject Now** on the machine's Overview,
or **Machine → Eject Installer** in the machine's window. It is removed at the
next start.

### Windows

See [Windows](windows.md).

## Creation progress

Click a creation job in the sidebar to see its current step, publisher requests,
elapsed time, download size and percentage, speed and estimated remaining time,
and activity history. Unknown download sizes show an activity indicator instead
of a percentage. Copy Details provides diagnostics without account passwords.

Cancel Creation stops installer discovery and downloads as well as media preparation.
Downloads keep partial files for a later retry. Jobs run in a detached worker,
so closing the library leaves creation running. An exited worker is reported as
an interrupted creation instead of appearing busy forever. Installer discovery
has a 90-second limit; stalled network reads fail after 30 seconds.

Failed or cancelled jobs remain available for inspection, retry, or dismissal.
Changing the system or installer opens the creation picker, where you can also
choose a local installer.

## The machine page

Select a machine in the sidebar. Right-click (or Control-click) a machine
for its context menu: start or resume, show or hide its window, pause,
suspend, shut down, take a snapshot, clone, reveal in Finder, or delete.
Only actions available in the machine’s current state appear. Force stop
and deletion ask for confirmation; a missing machine can be removed from
the library. Right-click a creation job for **Copy Status** and
**Cancel Creation** (or **Dismiss** after it finishes). Right-click empty
sidebar space for **New Machine…** and **Add Existing Machine…**.

The header shows its system, hardware and state, and the buttons that make
sense right now:

| State | Buttons |
| --- | --- |
| Stopped | **Start**, and start in the background (no window) |
| Suspended | **Resume**, and resume in the background |
| Running | **Show** / **Hide** the window, **Pause**, **Suspend**, **Shut Down**, force stop |
| Paused | **Resume**, **Shut Down** |
| Error | **Close** (ends the machine's process) |

**Shut Down** presses the guest's power button and lets the guest shut down
cleanly. Force stop pulls the plug.

The tabs:

- **Overview** — a live view of the screen while the machine runs with its
  window open (refreshed every few seconds), uptime, its address on the shared
  network (with a copy button), disk space really used against the disk's
  size, memory and processors. Below: **Clone**, **Show in Finder**, and
  **Delete…**.
- **Hardware** — see [Hardware](hardware.md).
- **Sharing** — see [Sharing](sharing.md).
- **Snapshots** — see [Snapshots, suspend and clones](snapshots.md).

## The machine window

Each running machine has its own window and its own Dock icon, belonging to
"Vista Machine". The window is sized to the guest's display and remembers
where you put it. Resizing it resizes the guest's display.

The window sends system shortcuts to the guest: ⌘Tab, ⌘Space, ⌘Q and the like
go to the machine, not the Mac. For the same reason the window's own menus
have no keyboard shortcuts:

- **Vista Machine (the machine's name)** — Open Vista, Hide, Suspend and Close.
- **Machine** — Pause, Resume, Take Snapshot, Eject Installer, Restart,
  Suspend, Shut Down, Force Stop.
- **Window** — Minimize, Zoom, Enter Full Screen.

**Closing the window suspends the machine**: its memory is written to disk
and the process quits, and the next start resumes it where it was. A machine
whose hardware can't be saved that way keeps running in the background
instead; show it again from Vista or the menu bar.

A machine started in the background has no window and no Dock icon until
something shows it.

## Other things you can do with a machine

- **Clone** — an instant copy-on-write duplicate of a stopped machine (see
  [Snapshots, suspend and clones](snapshots.md#clones)).
- **Delete…** — moves the machine's folder, with its disks and snapshots, to
  the Trash. The machine must be off.
- **Show in Finder** — its `.vista` folder.
- **Add an existing machine** — **File → Add Existing Machine…** (⌘O) and
  choose a `.vista` folder. Bundles placed in the machines location are also
  picked up when Vista launches.
- **Start with Vista** (Hardware tab) — the machine starts in the background
  whenever Vista opens.
