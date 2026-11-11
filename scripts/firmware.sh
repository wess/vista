#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
rustup target add aarch64-unknown-uefi
CARGO_TARGET_DIR="$root/target/firmware" cargo build --manifest-path "$root/crates/firmware/Cargo.toml" --release --target aarch64-unknown-uefi
mkdir -p "$root/target/windows"
cp "$root/target/firmware/aarch64-unknown-uefi/release/boot.efi" "$root/target/windows/boot.efi"
cp "$root/target/firmware/aarch64-unknown-uefi/release/display.efi" "$root/target/windows/display.efi"
