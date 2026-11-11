# Vista Tools

Vista Tools is a small program inside a guest that runs commands and moves
files when Vista asks. Agents use it through `run_command` and the file
tools; without it they can still work through the screen.

It runs in Linux and macOS guests. Windows isn't supported: the vsock channel
it listens on needs a driver there.

## Installing

Every machine sees Vista Tools on a read-only share. Install once per
machine.

### Linux

```sh
sudo mount -t virtiofs vistatools /mnt && sudo /mnt/install.sh
```

The installer:

- copies `vistatools` to `/usr/local/bin` (a static binary: no dependencies,
  any distribution);
- loads the `vmw_vsock_virtio_transport` kernel module;
- starts it now and at every boot — as a systemd service, an OpenRC service,
  or, on a system with neither, in the background until the next reboot;
- adds `/mnt/vista` to `/etc/fstab`, so [shared folders](sharing.md) mount
  at every boot.

### macOS

Open **/Volumes/My Shared Files/Vista Tools/install.command**. It asks for
your password once, copies `vistatools` to `/usr/local/bin`, and installs a
LaunchDaemon (`io.wess.vista.tools`) that keeps it running.

### Checking it

From Vista's side, `guest_info` answers with the guest's OS, kernel,
hostname and user. Inside the guest:

```sh
systemctl status vistatools          # systemd
rc-service vistatools status         # OpenRC (Alpine)
sudo launchctl print system/io.wess.vista.tools   # macOS
```

### Updating

Run the installer again; it replaces the binary and restarts the service.

## How it works

`vistatools` listens on **virtio-vsock port 7000**. vsock is a socket between
a virtual machine and its host that doesn't go through the network: nothing
on your network, or in another machine, can reach it. Only the
`vistamachine` process running that machine can connect.

Each connection carries one JSON request and one JSON answer; the protocol is
in [Protocols](protocol.md#vista-tools).

It runs as root. Commands run as the guest's console user unless the request
asks for root:

- in Linux, the first regular account (uid 1000 and up) with a login shell;
- in macOS, whoever owns the console (is logged in at the screen);
- root, when there's no such user.

Commands run in their own process group, so a timeout kills everything they
started. Output is kept up to 1 MB per stream. Files move as base64, up to
64 MB per request; `upload_file` and `download_file` chunk bigger files.

## Removing

```sh
# Linux, systemd
sudo systemctl disable --now vistatools && sudo rm /etc/systemd/system/vistatools.service
# Linux, OpenRC
sudo rc-service vistatools stop && sudo rc-update del vistatools && sudo rm /etc/init.d/vistatools
# then
sudo rm /usr/local/bin/vistatools

# macOS
sudo launchctl bootout system/io.wess.vista.tools
sudo rm /Library/LaunchDaemons/io.wess.vista.tools.plist /usr/local/bin/vistatools
```
