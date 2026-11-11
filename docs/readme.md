# Vista documentation

Vista runs virtual machines on Apple silicon with Apple's own Virtualization
framework. It finds and downloads the installer for the system you pick,
builds the machine, and boots it. Each machine runs in its own process, and a
built-in MCP server lets coding agents create machines, look at their screens,
type and click, and run commands inside them.

**New here?** Read [Installing Vista](install.md), then
[Creating and running machines](machines.md).

## Using Vista

| Page | What is on it |
| --- | --- |
| [Installing Vista](install.md) | Requirements, building the app, first launch, where Vista keeps things |
| [Creating and running machines](machines.md) | Picking a system, the creation job, the machine page, the machine window |
| [Systems](systems.md) | Every one-click system, how Vista finds the current installer, checksums, the installer cache |
| [Hardware](hardware.md) | Processors, memory, disks, display, networking, sound, USB |
| [Sharing](sharing.md) | Shared folders, clipboard, Rosetta for Linux |
| [Snapshots, suspend and clones](snapshots.md) | What each one saves, and what reverting does |
| [The menu bar](menubar.md) | The quick view and what you can do from it |
| [Settings and shortcuts](settings.md) | Every preference, menu and key binding |
| [Windows](windows.md) | Windows 11 for Arm: what works, what is experimental, and why |
| [Troubleshooting](troubleshooting.md) | Logs, common errors, starting over |

## Agents

| Page | What is on it |
| --- | --- |
| [Agents](agents.md) | Connecting an agent to Vista's MCP server, what it can do, how to use it well |
| [MCP tool reference](tools.md) | All 43 tools and their arguments |
| [Vista Tools](guesttools.md) | The agent inside a guest: installing it, what it runs as, removing it |

## How Vista is built

| Page | What is on it |
| --- | --- |
| [Architecture](architecture.md) | The crates, the processes, the machine bundle, the library database |
| [Protocols](protocol.md) | The control socket every machine listens on, and the Vista Tools protocol |
| [Building and contributing](building.md) | Building, signing, bundling, tests, conventions |
