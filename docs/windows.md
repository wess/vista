# Windows

Vista installs Windows 11 Pro for Arm64 using Microsoft's installer catalog,
a prepared USB installer, and an unattended local account. Apple's
Virtualization framework does not officially support Windows; Vista uses
host capabilities and a firmware display bridge to boot it.

## Creation and first start

1. Vista selects English (US), Arm64, Professional media from Microsoft's
   Media Creation Tool catalog. It checks the ESD download against the
   checksum in the HTTPS catalog. The browser ISO download service is a
   fallback; a local Arm64 ISO can also be selected.
2. Bundled wimlib extracts Windows Setup and exports the boot and Pro system
   images. Windows hardware drivers come from the virtio-win distribution.
3. Vista builds a sector-aligned USB disk with a boot partition and an exFAT
   partition for the system image. The boot loader starts Vista's display
   bridge before Windows Boot Manager.
4. Windows PE loads the storage driver, identifies the single empty guest
   disk, partitions it, applies Windows 11 Pro, installs hardware drivers,
   and copies the account settings and boot firmware into the new system.
5. The script writes an explicit installation result and shuts down.
   Vista only detaches Windows installer media after a successful result,
   then starts Windows from its system disk. A partial disk write is not
   treated as a successful installation.
6. The first Windows boot finishes system configuration and creates the
   user profile. This can take several minutes. Later starts use the
   installed system directly.

During Windows PE installation, the library reports the actual image-apply
percentage and driver-installation step from the installation log. The log
and result are in `vista/install.log` and `vista/result` on the installer
boot partition. A copy of the successful log is kept at
`C:\Windows\Panther\vista.log` in the installed system.

## The account

The username and password supplied when creating a machine become its local
administrator account. Automatic sign-in is configured when requested.
Without account details, Vista uses `vista` / `vista`; change that password
once signed in.

## Compatibility

- The CPU emulation switches used for Windows are private framework APIs
  and can change with macOS versions.
- Vista's firmware display bridge supplies the framebuffer needed before
  Windows loads its virtio display driver. Both the installation media and
  the installed system carry the boot loader and bridge.
- Windows is not activated. Activate Windows 11 Pro with your own license.
- There is no virtual TPM. Features requiring one are unavailable.
- Vista Tools currently supports Linux and macOS guests. Agents can control
  Windows through the screen, keyboard and pointer APIs.
- Failed starts expose the error and offer Try Again. Retrying closes the
  failed runner before starting a fresh one.

The catalog's Microsoft delivery address uses HTTP. Vista requires the
checksum supplied in the HTTPS catalog before preparing that downloaded
media. A rejected ISO-service request is an installer-discovery error,
not evidence that the VM hardware or Windows boot is working.
