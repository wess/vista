# Troubleshooting

## Where to look

| File | What's in it |
| --- | --- |
| `<machine>.vista/machine.log` | The machine process: when it started and stopped, errors from the framework, a saved state it couldn't use. |
| `<machine>.vista/serial.log` | Linux and BSD guests' serial console. A kernel that panics or an initramfs that can't find its disk says so here. |
| `~/Library/Application Support/Vista/jobs/*.json` | Each creation job's phase, progress and last message. |

**Show in Finder** on a machine's Overview opens its folder.

## A machine won't start

- **"This machine's hardware isn't valid: …"** — the framework's own reason
  follows. Usually more processors or memory than this Mac can give; lower
  them on the Hardware tab.
- **"This Mac can't run the macOS version installed in this machine"** — the
  guest is newer than the host supports. Update the Mac.
- **A third macOS machine won't start.** Apple allows two macOS guests at a
  time, counting ones other apps run.
- **The machine shows Error.** The header shows the message, and the machine
  process stays up for two minutes so you can read it, then exits. **Close**
  ends it sooner. `machine.log` has the same message.
- **It started fresh instead of resuming.** The saved memory didn't fit the
  machine any more — usually because its hardware changed while it was
  suspended. `machine.log` says "saved state unusable". The guest boots as if
  its power had been cut.
- **A Linux installer boots to a black screen.** Check `serial.log`. Some
  installers send their console to the serial port only; the note for each
  system in the picker says what to expect at its boot menu.

## Downloads

- **"…answered 403/404"** — the publisher moved its files. The entry needs
  updating; meanwhile, download the ISO in a browser and use **Use an
  Installer on This Mac…**.
- **"The download didn't match its published checksum and was discarded."** —
  the file was corrupted on the way. Create the machine again to download it
  again.
- **"Download stopped early … Try again to resume."** — the connection
  dropped. Creating the machine again picks up where it stopped.
- **Windows: "Microsoft wouldn't hand out a download link"** — Microsoft
  blocks some networks, including many VPNs. See [Windows](windows.md).

## Agents

- **"Agent control is turned off…"** — Settings → Agents, or the machine's
  Sharing tab.
- **"<name> isn't running. start_machine starts it."** — exactly that.
- **"The machine is running in the background. Show its window first"** —
  `show_machine`, or let `screenshot` show it (it does by itself).
- **"Vista Tools isn't answering inside this machine"** — install it (see
  [Vista Tools](guesttools.md)), or check its service is running. In Linux,
  `lsmod | grep vsock` should list `vmw_vsock_virtio_transport`.
- **`type_text` skipped characters.** It types on a US layout, and reports
  characters with no key there (accented letters, emoji). Write them with
  `write_file`, or set the guest's keyboard layout to US.
- **No address in `get_machine`.** The guest hasn't asked for one yet (still
  booting, or no network configured), its network is off, or it's bridged:
  Vista only knows addresses on the shared network.

## The app

- **"Vista can't be opened" on another Mac.** A build signed ad hoc only
  opens without asking on the Mac that built it. Control-click → Open, or
  sign with a Developer ID.
- **The library window doesn't update while it's covered.** macOS tells apps
  not to draw windows nobody can see; Vista catches up when the window is
  visible again.
- **A machine is missing from the sidebar.** Its folder moved. Add it again
  with **File → Add Existing Machine…**; machines in the machines location are
  found by themselves at launch.

## Starting over

Quit Vista, shut down every machine, then:

```sh
rm -rf "$HOME/Library/Application Support/Vista"
```

That deletes the library, every machine in the default location, and kept
installers. To keep the machines, move `machines/` somewhere first and add
them back afterwards.

A machine whose process crashed leaves its socket in `/tmp/vista<uid>/`; the
next start of that machine replaces it.
