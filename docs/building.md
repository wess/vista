# Building and contributing

## What you need

- A Mac with Apple silicon, macOS 15 or later, and Xcode's command line
  tools (for `codesign`, `hdiutil`, `iconutil` and `swift`).
- Rust 1.94 or later.
- The Linux target for Vista Tools: `rustup target add aarch64-unknown-linux-musl`.
  No cross toolchain is needed: `.cargo/config.toml` links it with `rust-lld`
  and the target's own C runtime.

## Building

```sh
cargo build                  # every crate, debug
scripts/tools.sh             # Vista Tools for Linux and macOS guests -> target/tools
scripts/sign.sh              # sign target/debug binaries (see below)
target/debug/vista           # run Vista
```

Debug builds compile dependencies at `opt-level = 2`, because gpui's layout
and text shaping are too slow unoptimized. Only the first build pays for it.

### Signing

The Virtualization framework refuses to create a machine in a process that
isn't signed with `com.apple.security.virtualization`. `scripts/sign.sh`
signs the debug binaries ad hoc with `assets/vista.entitlements`; run it after
every build that relinks `vistamachine` (`scripts/sign.sh release` for
`target/release`).

### The app

```sh
scripts/bundle.sh            # release build -> dist/Vista.app
scripts/dmg.sh               # dist/Vista.app -> dist/Vista.dmg
```

`bundle.sh` builds the three binaries and Vista Tools, lays out the bundle
(see [Installing Vista](install.md#what-is-in-the-app)), writes both
`Info.plist` files, and signs inside out: Vista Tools, the helper's binary,
the helper app, `vistamcp`, `vista`, the app. With `CODESIGN_IDENTITY` set to
a Developer ID it signs with the hardened runtime and a timestamp, ready for
notarization; otherwise ad hoc.

The release profile uses fat LTO and strips symbols. Build scripts and proc
macros are exempt from stripping: macOS 27 refuses to load stripped proc
macro libraries.

`scripts/icon.sh` redraws `assets/icon.icns` from `scripts/icon.swift`.

## Tests

```sh
cargo test --workspace
```

Unit tests cover the parts that are easy to get subtly wrong: bundle create,
clone and snapshot/revert on real files, the SQLite store, settings merging,
the control socket round trip, checksum parsing in every format, version
ordering in directory listings, MAC addresses, Windows answer files and
names, key combination parsing, DHCP lease lookup.

Two tests go to the network and are run by hand:

```sh
cargo test -p vista-kit --test catalog -- --ignored --nocapture
```

`every_entry_resolves` asks every publisher for its current installer and
prints what it found; `windows_link` gets a real download link from
Microsoft. Run them after touching the catalog or the resolver.

## Development switches

| Variable | Effect |
| --- | --- |
| `VISTA_SUPPORT=<dir>` | Use `<dir>` instead of `~/Library/Application Support/Vista` — a scratch library for testing, in every Vista process. |
| `VISTA_MACHINE=<path>` | The `vistamachine` binary to launch machines with. |
| `VISTA_TOOLS=<dir>` | The Vista Tools folder to share into guests. |
| `VISTA_PAGE=create\|settings\|welcome` | Open the window on that page. |
| `VISTA_TAB=hardware\|sharing\|snapshots` | Open a machine on that tab. |
| `VISTA_PANEL=1` | Open the menu bar quick view at launch. |
| `VISTA_FLOAT=1` | Keep the window above others. |

gpui stops drawing a window that's completely covered or on a locked screen,
so screenshots of such a window show its first frame only. `VISTA_PAGE`,
`VISTA_TAB` and `VISTA_PANEL` exist so a page can be checked from its first
frame.

A machine can be driven by hand over its socket:

```sh
printf '%s\n' '{"op":"type","text":"uname -a\n"}' | nc -U /tmp/vista$(id -u)/<id8>.sock
```

## Conventions

- Format with `cargo fmt` (two-space indent, 100 columns — `rustfmt.toml`).
  `cargo clippy --workspace` should be clean.
- File and directory names are lowercase, with no spaces, dashes or
  underscores. Group with directories (`ui/detail/hardware.rs`), never with
  compound names.
- Small files with one job each.
- Functions and data over classes. The few Objective-C classes Vista defines
  (`define_class!`) exist because AppKit requires a target or a delegate.
- Comments say why, not what.
- `vista-kit` never depends on AppKit or gpui; `vista-guest` depends on serde
  only.
- Read every color and size from the theme (`theme(cx)`), so light and dark
  switch for free.
- Framework APIs newer than the published objc2 bindings are called by
  selector, after `respondsToSelector:`, so Vista still runs on a host
  without them.

## Releasing

Bump `version` in the workspace `Cargo.toml`, build with a Developer ID
(`CODESIGN_IDENTITY=… scripts/bundle.sh`), notarize and staple the app,
then `scripts/dmg.sh`.

Windows media packaging requires `wimlib` on the build Mac (`brew install wimlib`).
`scripts/windows.sh` copies its executable, library, licenses and source link into
`target/windows`, rewrites library paths, and checks that no Homebrew library
paths remain. The bundle includes these tools; users do not need Homebrew.

`scripts/firmware.sh` builds Vista’s Windows UEFI boot loader and display bridge
from `crates/firmware` for `aarch64-unknown-uefi`. It uses the Apache-compatible
`r-efi` protocol bindings and Rust’s UEFI linker. Windows packaging runs this
step and includes both `.efi` files in the app’s Windows resources.
