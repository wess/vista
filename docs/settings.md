# Settings and shortcuts

## Settings

Open with ⌘, or the gear in the toolbar.

| Section | Setting | What it does |
| --- | --- | --- |
| General | Appearance | System, Light or Dark. System follows macOS as it changes. |
| General | Show in menu bar | The [quick view](menubar.md) icon. |
| Machines | Location | Where new machines are created. Existing machines stay where they are. |
| Machines | Share Downloads with new machines | The default for the option of the same name when creating a machine. |
| Downloads | Keep installers | Reuse a downloaded ISO or restore image for the next machine of the same system. |
| Downloads | Downloaded installers | How much space kept installers use; **Delete Downloads** removes them. |
| Agents | Let coding agents manage machines | Turns the MCP server's control of every machine on or off. Each machine also has its own switch (Sharing tab). |
| Agents | MCP server | The command to point an agent at, and a JSON snippet for agents configured that way. See [Agents](agents.md). |

Settings are stored in the library database, one row per setting.

## Menus

| Menu | Items |
| --- | --- |
| Vista | About Vista, Settings…, Services, Hide Vista, Hide Others, Show All, Quit Vista |
| File | New Machine…, Add Existing Machine…, Close Window |
| Edit | Undo, Redo, Cut, Copy, Paste, Select All |
| Machine | Start, Show Window, Pause, Resume, Suspend, Take Snapshot, Shut Down, Force Stop — acting on the machine selected in the sidebar |
| Window | Minimize, Zoom, Enter Full Screen |
| Help | Vista on GitHub, Report an Issue |

A machine's own window has different menus; see
[The machine window](machines.md#the-machine-window).

## Keyboard shortcuts

| Keys | Action |
| --- | --- |
| ⌘N | New Machine |
| ⌘O | Add Existing Machine |
| ⌘R | Start the selected machine |
| ⇧⌘S | Take a snapshot of the selected machine |
| ⌘, | Settings |
| ⌘W | Close the window (Vista keeps running in the menu bar) |
| ⌘M | Minimize |
| ⌃⌘F | Full screen |
| ⌘H / ⌥⌘H | Hide Vista / Hide Others |
| ⌘Q | Quit Vista (machines keep running) |

Inside a machine's window, shortcuts go to the guest.
