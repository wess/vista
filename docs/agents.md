# Agents

Vista includes an MCP server, `vistamcp`, so a coding agent can do with
machines what you can: create them, start and stop them, snapshot them, look
at their screens, type and click, and run commands and move files inside
them. It works with any agent that speaks the Model Context Protocol over
stdio.

## Connecting an agent

The server is a command; there's nothing to start or keep running. Point the
agent at:

```text
/Applications/Vista.app/Contents/MacOS/vistamcp
```

Agents configured with JSON take:

```json
{
  "mcpServers": {
    "vista": { "command": "/Applications/Vista.app/Contents/MacOS/vistamcp" }
  }
}
```

Settings → Agents shows the exact path for your copy of Vista, with buttons
to copy either form. For a development build the command is
`target/debug/vistamcp` (signed with `scripts/sign.sh`).

When an agent connects, the server sends it a page of instructions —
what Vista is, the three ways in, and how to work well — so a new agent
doesn't need prompting to use it sensibly.

## Turning it off

- **Settings → Agents → Let coding agents manage machines** turns agent
  control off for every machine. Every tool then answers that it's off.
- **A machine's Sharing tab → Agent control** turns it off for one machine.

Agents can't change either switch.

## Three ways in

### Outside the machine

Library, power, configuration, snapshots and creation jobs: everything on the
machine page, and more (`configure_machine` can set a bridged network or a
read-only share). Machines are named by name (any case), id, or an 8+
character id prefix.

Creating a machine downloads gigabytes, so `create_machine` starts a
**background job** and answers at once with its id. The job runs in its own
process (`vistamcp job <id>`), outlives the agent's session, and shows up in
Vista's sidebar. `get_job` with `wait_seconds` blocks until it's done or the
time runs out.

### Through the screen

`screenshot` returns the guest's display as a PNG. Its size in pixels is the
coordinate space for every pointer tool, with the origin at the top-left. The
pointer and keyboard tools send input through the machine's virtual keyboard
and pointer — exactly what a person's would — so they work with every guest,
before an OS is even installed, with nothing to install in the guest.

- `click`, `double_click`, `right_click`, `move_mouse`, `drag`, `scroll` —
  at screenshot coordinates.
- `type_text` — one keystroke per character on a US layout; newlines press
  Return. Characters without a US key are reported back, not dropped
  silently.
- `press_keys` — shortcuts: `enter`, `escape`, `cmd+c`, `cmd+shift+t`,
  `ctrl+alt+delete`, `alt+f4`, `f5`, `pagedown`. Several, space-separated,
  run in order: `"cmd+a backspace"`. `cmd` (also `win`, `super`) is the
  platform key: Command in macOS, the Windows key elsewhere.

Every input tool takes `screenshot: true` to answer with a fresh screenshot
after acting, and `settle_ms` to wait for the guest first. Screens change;
an agent should look again rather than assume a click landed.

Screenshots need the machine's window on screen; a machine started in the
background is shown first.

### Inside the guest

With [Vista Tools](guesttools.md) installed in a Linux or macOS guest:

- `run_command` — through `/bin/sh -c`, returning the exit code, stdout and
  stderr (each up to 1 MB), with a timeout that kills the whole process
  group. Runs as the guest's console user, or as root with `admin: true`.
- `read_file`, `write_file`, `list_directory`, `file_info`,
  `make_directory`, `delete_path` — paths in the guest.
- `upload_file`, `download_file` — Mac ↔ guest, any size, in chunks.
- `guest_info` — OS, kernel, architecture, hostname, user.

When Vista Tools isn't answering, these tools say so and say how to install
it — which an agent can do itself with `type_text` in a terminal.

## Patterns that work

**A disposable Linux box**

```text
create_machine   system: "ubuntuserver2604", name: "Scratch", wait_seconds: 600
screenshot       machine: "Scratch"           # walk the installer with click / type_text
…
take_snapshot    machine: "Scratch", name: "clean"
run_command      machine: "Scratch", command: "sudo apt-get install -y nginx", admin: true
revert_snapshot  machine: "Scratch", snapshot: "clean", stop_first: true, start: true
```

**Trying something risky.** `take_snapshot` before, `revert_snapshot` after.
Snapshots of a running machine include its memory, so reverting lands back in
the same session.

**Driving a GUI.** `screenshot`, decide, act with `screenshot: true`, repeat.
Prefer keyboard shortcuts to hunting for buttons when the guest has them.

**Reaching a server in the guest.** `get_machine` reports the guest's address
on the shared network; the Mac can reach any port on it.

## What an agent can do

Treat access to `vistamcp` as access to your account. An agent with it can:

- run any command, as root, inside a guest with Vista Tools;
- share any folder on your Mac into a guest (`configure_machine`
  `add_shares`) and so read and change what's in it from inside;
- read any file on your Mac it names (`upload_file`, `installer_path`) and
  write wherever it names (`download_file`);
- revert and delete snapshots, and delete machines (`delete_machine` needs
  `confirm: true` and moves the machine to the Trash).

Give it to agents you'd let use your terminal, and use the switches above
for machines an agent shouldn't have.

The full list of tools and their arguments is the
[MCP tool reference](tools.md).
