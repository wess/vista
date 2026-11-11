# Hardware

A machine's **Hardware** tab. Changes to a stopped machine take effect when it
next starts. Changes to a running machine are saved at once and apply the
next time it starts — **Machine → Restart** in its window (or
`restart_machine` for agents) does that. Settings that live in a machine's
`config.json` but aren't in this tab yet are noted below.

## Processors and memory

Up to the number of cores in your Mac, and as much memory as you can spare.
macOS guests have a floor set by their restore image; Vista raises a machine
to it when installing. Memory is reserved for the guest while it runs, so
leave enough for macOS and your other apps.

## Disk

The system disk is a sparse raw image (`disk.img` in the machine's folder):
it takes only the space the guest has written. **Disk (GB)** can grow it —
only while the machine is off — and never shrink it. After growing, extend the
partition from inside the guest (Disk Utility in macOS, `growpart` and
`resize2fs` or the distribution's partition tool on Linux).

Disks attach as virtio block devices. Installer media attach as USB drives.

## Display

| Preset | Notes |
| --- | --- |
| 1440 × 900 | |
| 1920 × 1200 | Default for Linux, BSD and Windows |
| 2560 × 1600 | |
| 2880 × 1800 | Default for macOS (at 220 pixels per inch, so it looks like 1440 × 900 on a Retina screen) |

The preset is the size a machine starts with. While it runs, resizing its
window resizes the guest's display to match, where the guest supports it
(macOS does, Linux with a virtio-gpu driver does).

macOS guests get Apple's paravirtualized graphics, with Metal on your Mac's
GPU. Linux and BSD guests get virtio-gpu, without 3D acceleration.

## Network

| Mode | What the guest gets |
| --- | --- |
| **Shared** (default) | The Mac's connection, through NAT. The guest gets a `192.168.64.x` address from macOS's own DHCP server; Vista reads it from the lease and shows it on the Overview. The Mac can reach the guest at that address (SSH, web servers); other devices on your network can't. |
| **Off** | No network device at all. |
| Bridged | The guest joins your physical network like another computer. Set by agents (`configure_machine` with `network: "bridged:en0"`) or in `config.json`. Apple only lets apps with its `com.apple.vm.networking` entitlement do this, which it grants per developer; a self-built Vista without it can't start a bridged machine. |

Every machine has a fixed, locally administered MAC address, so its DHCP lease
— and so its address — survives restarts.

## Sound

**Sound** gives the guest an output device playing through the Mac.
**Microphone** gives it the Mac's microphone too; macOS asks permission the
first time.

## Nested virtualization

Linux, BSD and Windows guests can run virtual machines of their own, on a Mac
with an M3 or later. Vista turns it on only where the host supports it.

## Start with Vista

The machine starts in the background whenever Vista opens.

## USB

Every machine has a USB (XHCI) controller; installer media hang off it.

On a macOS 27 host, passing a USB device on your Mac to a virtual machine is
a system feature: macOS's own **Virtual Machine Accessories** menu bar item
offers each device to running virtual machines, and needs the machine to have
a USB controller, which every Vista machine does. Vista has no USB menu of its
own, and this path hasn't been tested with Vista yet.

## Keyboard and pointer

macOS guests get Apple's virtual keyboard and a trackpad (gestures work),
plus a pointer for older guests. Everything else gets a USB keyboard and an
absolute-position pointer, so the pointer moves in and out of the machine's
window with no capture.

## Firmware and boot

macOS guests boot Apple's own way, from boot storage made at install time.
Everything else boots UEFI, with its variables in the machine's `nvram` file.

**Secure Boot** (macOS 27 hosts, `secureboot` in `config.json`, off by
default and not yet in this tab) enrolls Apple's default platform key and the
default signature databases when the machine's firmware storage is first
created.

Linux and BSD machines also write their serial console to `serial.log` in the
machine's folder: the first place to look when a guest won't boot (see
[Troubleshooting](troubleshooting.md)).
