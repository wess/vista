# Systems

The systems Vista installs in one step. Agents pass the **id** to
`create_machine`; `list_systems` returns the same list.

## macOS

| id | System | Notes |
| --- | --- | --- |
| `macoslatest` | macOS (newest) | Today, macOS 27 |
| `macos26` | macOS 26 | Newest 26.x |
| `macos15` | macOS 15 | Newest 15.x |
| `macos14` | macOS 14 | Newest 14.x |
| `macos13` | macOS 13 | Newest 13.x |

Restore images (`.ipsw`) come from Apple's CDN. The list of versions comes
from [ipsw.me](https://ipsw.me), which indexes Apple's images for virtual Macs
with their sizes and SHA-256 sums; if it can't be reached, Vista finds the
newest image in Apple's own software update catalog. A host can only run
macOS versions its own macOS supports, and Apple allows at most two macOS
machines running at once.

## Windows

| id | System | Notes |
| --- | --- | --- |
| `windows11` | Windows 11 for Arm, Pro edition | Experimental — see [Windows](windows.md) |

## Linux

| id | System | Edition | Desktop | Notes |
| --- | --- | --- | --- | --- |
| `ubuntudesktop2604` | Ubuntu 26.04 LTS | Desktop | yes | GNOME with a graphical installer |
| `ubuntuserver2604` | Ubuntu 26.04 LTS | Server | no | Text installer; offers to install OpenSSH |
| `ubuntudesktop2404` | Ubuntu 24.04 LTS | Desktop | yes | GNOME with a graphical installer |
| `ubuntuserver2404` | Ubuntu 24.04 LTS | Server | no | Text installer |
| `fedoraworkstation` | Fedora (newest release) | Workstation | yes | Live GNOME; install from the desktop. Choose "Start Fedora" at the boot menu to skip the media check |
| `fedoraeverything` | Fedora (newest release) | Everything (network) | yes | Graphical network installer; pick any edition or desktop |
| `debian` | Debian 13 | Netinst | no | Choose "Graphical install" at the boot menu for the mouse-driven installer |
| `debiandvd` | Debian 13 | DVD (offline) | no | Installs without a network connection |
| `kali` | Kali Linux | Installer | no | Installs Xfce. Choose "Graphical install" at the boot menu |
| `centos` | CentOS Stream 10 | Network install | yes | Graphical Anaconda installer |
| `rockyworkstation` | Rocky Linux 10 | Workstation | yes | Live GNOME with an installer |
| `rocky` | Rocky Linux 10 | Network install | yes | Graphical Anaconda installer |
| `almagnome` | AlmaLinux 10 | Live GNOME | yes | Live GNOME with an installer |
| `alma` | AlmaLinux 10 | Network install | yes | Graphical Anaconda installer |
| `tumbleweed` | openSUSE Tumbleweed | Live GNOME | yes | Rolling release |
| `tumbleweednet` | openSUSE Tumbleweed | Network install | yes | Rolling release, YaST installer |
| `popos` | Pop!_OS 24.04 | COSMIC | yes | Renders in software: Linux guests have no 3D acceleration |
| `nixos` | NixOS 26.05 | Graphical | yes | GNOME with the Calamares installer |
| `nixosminimal` | NixOS 26.05 | Minimal | no | Console installer |
| `alpine` | Alpine Linux | Virtual | no | Tiny. Log in as root and run `setup-alpine` |
| `alpinestandard` | Alpine Linux | Standard | no | Log in as root and run `setup-alpine` |

"Desktop" means the default boot reaches a graphical installer or a live
desktop.

## BSD

| id | System | Notes |
| --- | --- | --- |
| `freebsd` | FreeBSD (newest release) | Text installer. Experimental on this framework |
| `openbsd` | OpenBSD (newest release) | Text installer. Experimental on this framework |
| `netbsd` | NetBSD (newest release) | Text installer. Experimental on this framework |

## Anything else

Any Arm64 ISO that boots with UEFI works through **Use an Installer on This
Mac…** (or `installer_path` for agents). So does any macOS `.ipsw`.

## How Vista finds the current installer

No entry names a point release. Each says where its publisher puts the files
and how to recognise the right one, and Vista asks at the moment you create a
machine. "Ubuntu 26.04" means 26.04.1 the day 26.04.1 is published, with no
update to Vista.

| Kind | Used for | How |
| --- | --- | --- |
| Directory listing | Ubuntu, Fedora, Kali, CentOS, Rocky, AlmaLinux, openSUSE, Alpine, the BSDs | Read the publisher's directory index and take the newest file matching a pattern. "Newest" compares the first version number in the name, then every number (so dated snapshots sort too). A `{n}` in the directory means "the newest numbered release directory", trying the next one down when the newest holds only betas. |
| Checksum listing | Debian | Debian's directory pages are unreliable, so the file names come from its `SHA256SUMS`. |
| Fixed URL | NixOS | The channel's `latest-…` link. |
| JSON API | Pop!_OS | System76's build API, which also gives the SHA-256. |
| Apple | macOS | ipsw.me's index of virtual Mac images; Apple's catalog as the fallback. |
| Microsoft | Windows 11 | Microsoft's software download service, with the handshake its own page performs. |

Pages that answer with a server error are retried with backoff.

### Checksums

Vista checks the SHA-256 the publisher lists and discards a download that
doesn't match. It reads the three shapes distributions publish:
`HASH  file` (GNU), `SHA256 (file) = HASH` (BSD style, including files
wrapped in a PGP signature, as Fedora and AlmaLinux publish them), and
one-file `.sha256` sums. NetBSD publishes only SHA-512, so its downloads are
checked by size alone; so are Windows downloads when Microsoft's page doesn't
list a hash.

### The installer cache

With **Keep installers** on (Settings → Downloads), a finished download is
recorded in the library, and the next machine of the same system reuses it.
**Delete Downloads** in Settings removes them. A partly downloaded file is
kept as `<name>.part` in the images folder and resumed next time.

## Adding a system

Systems live in `crates/kit/src/catalog/`: macOS in `macos.rs`, Linux and
the BSDs in `linux.rs`, Windows in `mod.rs`. A Linux entry names its id,
family, display name, version, edition, source, checksum file and format,
whether it boots to a desktop, and a one-line note for the picker.

Check every entry against the live publishers before sending a change:

```sh
cargo test -p vista-kit --test catalog -- --ignored --nocapture
```

It prints what each entry resolves to today and fails on any that don't.
A new family also needs a display name in `catalog::family_name` and a color
and monogram in `crates/app/src/ui/glyph.rs`.
