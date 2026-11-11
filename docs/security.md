# Security

Vista has three distinct trust boundaries: the Mac app and its local helper
processes, each virtual machine, and optional agent access. It has no account,
cloud service, analytics, or update daemon.

## Process isolation

Each running machine lives in its own `vistamachine` process. A guest crash or
framework failure takes down that process rather than Vista's library window or
another guest. Quitting the library does not kill running machines.

Machine control uses Unix sockets inside `/tmp/vista<uid>/`. The directory is
mode `0700`, each socket is mode `0600`, requests are size-bounded, and the
server admits at most 32 simultaneous connections. These sockets are local to
the signed-in user; they are not TCP listeners.

Vista Tools listens only on the guest's virtio-vsock transport. It does not
open a network port in the guest. Requests and file chunks are bounded, command
output is capped at 1 MiB per stream, and commands time out after at most one
hour. A timeout kills the command's process group.

## Downloads

Catalog traffic uses HTTPS with rustls, a 20-second connection timeout and a
30-second read timeout. Installer downloads are written to a partial file and
only moved into the image library after the transfer completes.

When a publisher provides a SHA-256 checksum, Vista verifies it before using
the image and discards a mismatch. Windows ESD files are checked against the
checksum in Microsoft's catalog. A local ISO or IPSW supplied by the user is
trusted as local input.

Some publishers expose checksums in a PGP-clearsigned text file. Vista reads
the checksum from that file over HTTPS but does not independently validate the
PGP signature. In that case TLS and the publisher's HTTPS endpoint remain part
of the trust chain.

## Guest capabilities

A virtual machine has the devices enabled in its configuration. Shared folders
are explicit and can be read-only. Shared networking uses NAT; bridged
networking requires a separately granted Apple entitlement. Microphone access
requires macOS permission and is off unless enabled for the machine.

Vista Tools runs as root so it can perform an explicitly requested admin
operation, but ordinary commands drop to the guest's console user when one can
be identified. Installing Vista Tools therefore grants the Mac-side Vista
process administrative control inside that guest.

## Agent access

`vistamcp` is a local stdio server: it accepts no inbound network connection.
Global agent access can be disabled in Settings, and individual machines can
be excluded in their Sharing tab. Agents cannot turn either permission back
on.

Access to `vistamcp` should still be treated like terminal access. An allowed
agent can start and stop machines, operate their screens, run root commands in
guests with Vista Tools, and configure a Mac folder as a guest share. Destructive
tools require explicit confirmation where recovery would otherwise be unclear;
machine deletion moves the bundle to the Trash.

## Release integrity

Release builds use a Developer ID Application certificate, hardened runtime,
and timestamp. The app and disk image are notarized separately and have their
tickets stapled. Every GitHub release includes a SHA-256 file for the disk
image; the Homebrew cask pins that same digest.

Dependency advisories are checked in CI with `cargo audit`. Formatting, the
workspace test suite, Clippy with warnings denied, the site verifier, and shell
script analysis are also release gates.
