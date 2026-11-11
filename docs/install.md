# Installing Vista

## Requirements

- A Mac with Apple silicon. Apple's Virtualization framework runs Arm guests
  only, so every machine is arm64: macOS, Arm64 Linux, the Arm64 BSDs, and
  Windows 11 for Arm.
- macOS 15 or later.
- macOS 27 adds three things Vista uses when they are there:
  - UEFI Secure Boot for Linux and Windows machines.
  - Creating the first account in a new macOS machine, so Setup Assistant is
    skipped.
  - Passing USB devices to a machine from the system's menu bar (see
    [Hardware](hardware.md#usb)).
- Disk space for the machines themselves. Disks are sparse: a 64 GB disk
  uses only what the guest has written. Installers are large: a macOS restore
  image is about 25 GB, Windows 11 about 8.5 GB, a Linux desktop ISO 2–4 GB.

## Getting the app

Vista is built from source. See [Building and contributing](building.md) for
the details; the short version:

```sh
rustup target add aarch64-unknown-linux-musl   # once: Vista Tools for Linux guests
scripts/bundle.sh                              # -> dist/Vista.app
scripts/dmg.sh                                 # -> dist/Vista.dmg (optional)
```

Drag `dist/Vista.app` to Applications.

A bundle built without a Developer ID is signed ad hoc. It runs on the Mac
that built it. Copied to another Mac, Gatekeeper blocks it the first time:
Control-click the app, choose Open, and confirm. To hand builds to other
people, sign with a Developer ID (`CODESIGN_IDENTITY=… scripts/bundle.sh`)
and notarize.

## First launch

Vista opens its library window and puts an icon in the menu bar. The library
is empty: choose **New Machine** to make one (see
[Creating and running machines](machines.md)).

Nothing needs an account, a licence key or a network service of Vista's own.
Vista goes to the network only to find and download installers:

- the publishers themselves: Apple, Microsoft, and the Linux and BSD
  projects' own mirrors;
- [ipsw.me](https://ipsw.me), a free index of Apple's restore images, for the
  list of macOS versions (the images themselves come from Apple's CDN, and the
  newest is also found in Apple's own catalog if the index is unreachable);
- the Fedora project, for the Arm64 virtio drivers Windows needs.

### Permissions you may be asked for

- **Microphone**, the first time a machine you gave the microphone to starts.
  The prompt names "Vista Machine", the process that runs machines.

Vista does not need Screen Recording or Accessibility. Machine screenshots
for agents use a capture path macOS allows a process to use on its own
windows without asking.

## What is in the app

```text
Vista.app/Contents/
  MacOS/vista                     the library window and the menu bar item
  MacOS/vistamcp                  the MCP server for agents
  Helpers/VistaMachine.app/       runs one machine per process
    Contents/MacOS/vistamachine
    Contents/Resources/tools/     Vista Tools, shared into every guest
```

Quitting Vista leaves running machines running. Each one lives in its own
`vistamachine` process, with its own window and Dock icon.

## Where Vista keeps things

| Path | What |
| --- | --- |
| `~/Library/Application Support/Vista/machines/` | Machine bundles (`.vista` folders), unless you choose another location in Settings |
| `~/Library/Application Support/Vista/images/` | Downloaded installers kept for reuse |
| `~/Library/Application Support/Vista/jobs/` | Machine creation jobs and their progress |
| `~/Library/Application Support/Vista/vista.db` | The library: where bundles are, settings, the installer cache |
| `/tmp/vista<uid>/` | One control socket per running machine |

A machine is entirely its bundle: copy a `.vista` folder to another Mac and
add it with **File → Add Existing Machine**.

## Uninstalling

Quit Vista and shut down your machines, then delete `Vista.app` and
`~/Library/Application Support/Vista`. Machines you keep elsewhere stay
where you put them.
