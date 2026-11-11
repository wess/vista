# Windows

Windows 11 for Arm is **experimental** in Vista. Apple's Virtualization
framework doesn't support Windows. Vista gets as far as the framework allows
and is honest about the rest: the download and the install disk are built and
tested; a Windows install running to completion on this framework has not
been verified.

## What Vista does

1. **Gets the ISO from Microsoft.** Vista uses the same download service as
   Microsoft's [Windows 11 for Arm download page](https://www.microsoft.com/en-us/software-download/windows11arm64),
   including the anti-fraud handshake the page performs, and picks the
   English (United States) multi-edition ISO. Links last 24 hours; the file is
   about 8.5 GB and resumes if interrupted.
2. **Fetches the Arm64 virtio drivers** (virtio-win, from the Fedora
   project), once, for every Windows machine.
3. **Builds an install disk.** The framework has no CD drive, and Windows only
   reads ISO 9660 from one, so the ISO can't be plugged in as it is. Vista
   builds a USB disk with macOS's own tools (`hdiutil`, `diskutil`, `ditto`):
   - a FAT32 partition the firmware boots from, holding everything on the ISO
     except the system image;
   - an exFAT partition holding `install.wim`, which is too big for FAT32;
   - `autounattend.xml`, which lifts Setup's TPM, Secure Boot, CPU, RAM and
     storage checks;
   - `vista\install.cmd`, which partitions the disk, applies Windows 11 Pro
     with DISM, adds the drivers and makes the disk bootable — Setup's own
     disk step is reported to fail on this framework's disks, so Vista
     doesn't use it;
   - `vista\unattend.xml`, which creates your local account, signs it in, and
     skips the out-of-box screens and the Microsoft account requirement;
   - the drivers under `$WinPEDriver$`, where Setup finds them on its own.
4. **Boots it with the switches Windows needs.** Windows Boot Manager needs
   the CPU's performance counters and fine-grained trap handling emulated, and
   a linear framebuffer to draw on. The framework has these but doesn't
   publish them; Vista turns each on only when the host answers it.

When the install script finishes it shuts the machine down. Vista sees a
power-off after more than 3 GB was written, removes the install disk, and
boots Windows from its own disk, where the answer file takes over.

## What to expect

- It may not boot on every Mac or macOS version: the switches above are
  private to Apple's framework and can change without notice.
- The display can stay on the firmware framebuffer until Windows loads the
  virtio display driver.
- There's no TPM, so BitLocker and anything else that needs one won't work.
- Windows isn't activated: Vista installs Windows 11 Pro without a product
  key. Activate it with your own.
- [Vista Tools](guesttools.md) doesn't run in Windows. Agents drive Windows
  through the screen, keyboard and pointer tools.
- Microsoft blocks the download service for some networks, including many
  VPNs. If Vista can't get a link, download the ISO from Microsoft's page in a
  browser and use **Use an Installer on This Mac…**; Vista builds the install
  disk the same way.

## The account

The username and password you give when creating the machine become a local
administrator account that signs in automatically after setup. Without them,
Vista uses `vista` / `vista` — change that password once you're in.
