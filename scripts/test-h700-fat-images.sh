#!/usr/bin/env bash
# Exercise the imported filesystem code against real FAT32 and exFAT disk images.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/platform/h700/storage/fat"
OUT="${TMPDIR:-/tmp}/h700-fat-image-tests"
MNT="$OUT/mnt"
REF="$OUT/ref"
mkdir -p "$OUT" "$MNT" "$REF"
cleanup() {
  if mountpoint -q "$MNT"; then sudo umount "$MNT" || true; fi
}
trap cleanup EXIT

for tool in cc mkfs.fat mcopy mmd mkfs.exfat fsck.exfat; do
  command -v "$tool" >/dev/null || { echo "missing required tool: $tool" >&2; exit 2; }
done

cc -std=c11 -O2 -Wall -Wextra -Werror -I "$SRC" \
  "$SRC/fat_ro.c" "$SRC/fat_exfat.c" "$SRC/tests/fat_ro_test.c" -o "$OUT/fat_ro_test"
cc -std=c11 -O2 -Wall -Wextra -Werror -I "$SRC" \
  "$SRC/fat_ro.c" "$SRC/fat_wr.c" "$SRC/fat_exfat.c" \
  "$SRC/tests/fat_exfat_rw_test.c" -o "$OUT/fat_exfat_rw_test"

echo "== FAT32 real-image read regression =="
FATIMG="$OUT/fat32.img"
truncate -s 128M "$FATIMG"
mkfs.fat -F 32 "$FATIMG" >/dev/null
mmd -i "$FATIMG" ::/roms
mmd -i "$FATIMG" ::/roms/gba
python3 - "$OUT" <<'PY'
import pathlib, sys
p = pathlib.Path(sys.argv[1])
(p/"gba_bios.bin").write_bytes(b"GBA_BIOS_16KB_CONTENT_MARKER" + bytes(13))
(p/"Pokemon Emerald (USA).gba").write_bytes(bytes(range(41)))
(p/"Zelda.gba").write_bytes(b"ZELDA1234")
(p/"Big.gba").write_bytes(bytes((i * 13 + (i >> 7)) & 255 for i in range(300000)))
PY
mcopy -i "$FATIMG" "$OUT/gba_bios.bin" ::/gba_bios.bin
mcopy -i "$FATIMG" "$OUT/Pokemon Emerald (USA).gba" "::/roms/gba/Pokemon Emerald (USA).gba"
mcopy -i "$FATIMG" "$OUT/Zelda.gba" ::/roms/gba/Zelda.gba
mcopy -i "$FATIMG" "$OUT/Big.gba" ::/roms/gba/Big.gba
"$OUT/fat_ro_test" "$FATIMG"

echo "== exFAT real-image read/write regression =="
for CLUSTER in 4K 128K; do
  IMG="$OUT/exfat-$CLUSTER.img"
  REF="$OUT/ref-$CLUSTER"
  mkdir -p "$REF"
  rm -f "$REF"/*
  truncate -s 256M "$IMG"
  mkfs.exfat -c "$CLUSTER" "$IMG" >/dev/null
  sudo mount -t exfat -o loop "$IMG" "$MNT"
  sudo mkdir -p "$MNT/roms/gba"
  sudo dd if=/dev/urandom of="$MNT/roms/gba/small.gba" bs=1K count=3 status=none
  sudo dd if=/dev/urandom of="$MNT/roms/gba/Big Rom Name (USA, Europe) [!].gba" bs=1M count=3 status=none
  sudo dd if=/dev/urandom of="$MNT/roms/gba/odd size.gba" bs=1 count=70001 status=none
  sudo cp "$MNT"/roms/gba/* "$REF/"
  sudo sync
  sudo umount "$MNT"
  "$OUT/fat_exfat_rw_test" "$IMG" "$REF"
  fsck.exfat -n "$IMG"
done
echo "H700 FAT32/exFAT image regressions PASS"
