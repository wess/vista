# Sharing

A machine's **Sharing** tab. Changes save at once and apply at the machine's
next start.

## Shared folders

**Share a Folder…** picks a folder on the Mac; ✕ stops sharing it. New
machines share your Downloads folder unless you turn that off (Settings, or
when creating the machine).

Folders are shared with virtio-fs, so the guest reads and writes them
directly — no network share, no syncing.

### In macOS guests

They appear by themselves in **/Volumes/My Shared Files**, one folder per
share.

### In Linux guests

All shared folders are under one virtio-fs tag, `vista`:

```sh
sudo mkdir -p /mnt/vista
sudo mount -t virtiofs vista /mnt/vista
ls /mnt/vista            # Downloads/  …
```

Installing [Vista Tools](guesttools.md) adds this mount to `/etc/fstab`, so
it's there on every boot.

### Read-only shares

A share can be read-only: the guest can see the files but not change them.
Agents set this with `configure_machine` (`add_shares` with
`readonly: true`); folders added from the tab are read-write.

### The Vista Tools share

Every machine also sees a read-only share holding [Vista Tools](guesttools.md):
the virtio-fs tag `vistatools` on Linux, and **Vista Tools** inside
/Volumes/My Shared Files on macOS. It isn't listed in the tab.

## Clipboard

Linux guests share the clipboard with the Mac through the SPICE agent
channel. The guest needs `spice-vdagent`, which most desktop distributions
install already; if copying doesn't work, install it:

```sh
sudo apt install spice-vdagent       # Debian, Ubuntu, Kali
sudo dnf install spice-vdagent       # Fedora, CentOS, Rocky, AlmaLinux
sudo zypper install spice-vdagent    # openSUSE
```

The Virtualization framework doesn't offer clipboard sharing for macOS
guests. Use a shared folder, or Vista Tools' file tools if you're an agent.

## Rosetta (Linux)

With **Rosetta** on, a Linux guest can run x86-64 Linux programs through
Apple's Rosetta. It needs Rosetta installed on the Mac (macOS offers to
install it the first time any Intel program runs, or
`softwareupdate --install-rosetta`).

Inside the guest, mount it and register it as the handler for x86-64
binaries:

```sh
sudo mkdir -p /media/rosetta
sudo mount -t virtiofs rosetta /media/rosetta
sudo /usr/sbin/update-binfmts --install rosetta /media/rosetta/rosetta \
  --magic "\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\x3e\x00" \
  --mask "\xff\xff\xff\xff\xff\xfe\xfe\x00\xff\xff\xff\xff\xff\xff\xff\xff\xfe\xff\xff\xff" \
  --credentials yes --preserve no --fix-binary yes
```

(Distributions without `update-binfmts` use `systemd-binfmt` with the same
magic and mask.)

## Agent control

**Agent control** decides whether coding agents may control this machine
through Vista's MCP server. Off, every agent tool refuses this machine and
says why; `list_machines` still shows it. Settings → Agents turns agent
control off for the whole Mac. See [Agents](agents.md).
