//! The orientation an agent gets when it connects.

pub const INSTRUCTIONS: &str = r#"Vista runs virtual machines on this Mac with Apple's Virtualization framework (Apple silicon only). Every guest is arm64: macOS, Linux distributions, the BSDs, and Windows 11 for Arm (experimental).

WHAT YOU CAN DO
Outside a machine (always available):
- Library: list_machines, get_machine, list_systems, create_machine, clone_machine, configure_machine, delete_machine.
- Power: start_machine, stop_machine (graceful, or force), restart_machine, pause_machine, resume_machine, suspend_machine (memory saved to disk; next start resumes instantly), show_machine / hide_machine (window).
- Snapshots: take_snapshot (works while running: memory is captured too), list_snapshots, revert_snapshot, rename_snapshot, delete_snapshot. Snapshots are APFS copy-on-write clones: instant and nearly free until the machine writes.
- Jobs: create_machine downloads the installer and builds the machine in the background and returns a job id. Poll get_job (pass wait_seconds to block up to that long). list_jobs, cancel_job.

Inside a machine, through its screen (works for every guest, nothing to install):
- screenshot: a PNG of the guest's display. Its pixel size is the coordinate space for every pointer tool; origin is the top-left corner.
- click, double_click, right_click, move_mouse, drag, scroll: pointer actions at screenshot pixel coordinates.
- type_text: types text as keystrokes (US keyboard layout; characters without a key are reported back).
- press_keys: shortcuts like "enter", "cmd+c", "ctrl+alt+delete", "cmd+shift+4"; several separated by spaces ("cmd+a delete"). "cmd" is the platform key: Command in macOS, the Windows/Super key in Windows and Linux.
Every pointer and keyboard tool takes screenshot=true to return a fresh screenshot after acting, which saves a round trip.

Inside a machine, through Vista Tools (a small agent in the guest, Linux and macOS):
- guest_info, run_command, read_file, write_file, list_directory, file_info, make_directory, delete_path, upload_file (Mac -> guest), download_file (guest -> Mac).
- If these report that Vista Tools isn't answering, install it once in the guest. Linux: `sudo mount -t virtiofs vistatools /mnt && sudo /mnt/install.sh`. macOS: open "/Volumes/My Shared Files/Vista Tools/install.command". You can do that yourself with type_text/press_keys in a terminal.

HOW TO WORK WELL
- Refer to machines by name or id (an 8-character id prefix works).
- After start_machine, a fresh OS needs time to boot: take a screenshot before acting, and again after every few actions. Screens change; never assume a click landed.
- Machines started with visible=false have no window on screen; screenshot shows the window first.
- Prefer run_command over typing into a terminal when Vista Tools is installed: you get stdout, stderr and the exit code.
- Before risky changes, take_snapshot; revert_snapshot rolls back in seconds.
- Shared networking gives guests 192.168.64.x addresses; get_machine reports the IP once the guest has one. The Mac can reach the guest at that address (SSH, web servers).
- New machines share the Mac's Downloads folder (Linux: `sudo mount -t virtiofs vista /mnt/vista`; macOS: /Volumes/My Shared Files).
- macOS guests: at most two run at once (Apple's limit). A macOS 27 host can create the first account for you (create_machine username/password), skipping Setup Assistant.
- Installing from an ISO: the installer runs in the machine's window. Watch it with screenshot and drive it with the pointer and keyboard tools. When the guest powers off after installing, Vista removes the installer and boots from disk.
- The person at the Mac can turn agent control off globally (Vista Settings) or per machine; tools then say so.
"#;
