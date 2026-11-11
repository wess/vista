# The menu bar

Vista puts a stacked-windows icon in the menu bar. Click it for the quick
view: every machine at a glance, without opening the library window.

```text
┌──────────────────────────────────────┐
│ Vista                      1 running │
│ ▢ Ubuntu Dev                      ⏻  │
│   ● Running · 2h 14m · 192.168.64.3  │
│ ▢ macOS 27                        ▶  │
│   ● Stopped · 4 CPU · 8 GB · 96 GB   │
├──────────────────────────────────────┤
│ Open Vista   New Machine        Quit │
└──────────────────────────────────────┘
```

Each row shows the machine's state and, while it runs, its uptime and
address (or install progress).

- **▶** starts a stopped machine with its window; **⏻** asks a running one to
  shut down.
- **Clicking a row** brings a running machine's window forward, or opens the
  library window on a stopped one.
- **Open Vista** and **New Machine** open the library window.
- **Quit** quits Vista. Running machines keep running.

The quick view closes when you click anywhere else.

To remove the icon, turn off **Show in menu bar** in Settings.
