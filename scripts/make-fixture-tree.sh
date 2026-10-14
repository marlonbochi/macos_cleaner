#!/usr/bin/env bash
# Creates a directory tree with known sizes for manual validation (see quickstart.md).
# Files are written with real data (not sparse) so their allocated size matches.
set -euo pipefail

target="${1:?usage: $0 <target-dir>}"
if [[ -e "$target" ]]; then
  echo "refusing to overwrite existing path: $target" >&2
  exit 1
fi

write() { # path size_mb
  mkdir -p "$(dirname "$1")"
  dd if=/dev/zero of="$1" bs=1m count="$2" status=none
}

write "$target/videos/big.mov" 500
write "$target/isos/medium.iso" 200
write "$target/docs/small.pdf" 50
for i in $(seq 1 150); do write "$target/game/assets/chunk$i.pak" 1; done
ln "$target/isos/medium.iso" "$target/isos/medium-hardlink.iso"
ln -s "$target/videos" "$target/videos-symlink"
mkdir -p "$target/locked" && write "$target/locked/secret.bin" 120 && chmod 000 "$target/locked"
mkdir -p "$target/Tool.app/Contents/MacOS" && write "$target/Tool.app/Contents/MacOS/tool" 110

cat <<MSG
Fixture created at $target (threshold suggestion: 100 MB). Expected results:
  - videos/big.mov (500 MB), isos/medium.iso (200 MB) listed; docs/small.pdf (50 MB) not listed
  - medium-hardlink.iso counted once; videos-symlink ignored
  - game/ folder ~150 MB from many small files; Tool.app shown as a package
  - locked/ reported as inaccessible (run: chmod 755 "$target/locked" before deleting)
MSG
