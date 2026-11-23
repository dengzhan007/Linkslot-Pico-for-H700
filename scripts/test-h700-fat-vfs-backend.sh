#!/bin/sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
OUT="/tmp/h700-fat-vfs-backend-$$"
trap 'rm -rf "$OUT"' EXIT HUP INT TERM
mkdir -p "$OUT"

SOURCES="
$ROOT/platform/h700/storage/mmc/h700_fat_vfs_backend.c
$ROOT/platform/h700/storage/mmc/h700_sunxi_mmc_fat.c
$ROOT/platform/h700/storage/mmc/h700_sunxi_mmc_block_device.c
$ROOT/platform/h700/storage/mmc/sunxi_mmc_pio_read.c
$ROOT/platform/h700/storage/fat/fat_ro.c
$ROOT/platform/h700/storage/fat/fat_exfat.c
$ROOT/platform/h700/storage/mmc/tests/h700_fat_vfs_backend_test.c
"

# shellcheck disable=SC2086
cc -std=c11 -O2 -Wall -Wextra -Werror \
    -I"$ROOT/platform/h700/runtime/include" \
    -I"$ROOT/platform/h700/storage/mmc" \
    -I"$ROOT/platform/h700/storage/gpt" \
    -I"$ROOT/platform/h700/storage/fat" \
    $SOURCES -o "$OUT/test-h700-fat-vfs-backend"
"$OUT/test-h700-fat-vfs-backend"

# shellcheck disable=SC2086
if cc -std=c11 -O1 -g -Wall -Wextra -Werror \
    -fsanitize=address,undefined -fno-omit-frame-pointer \
    -I"$ROOT/platform/h700/runtime/include" \
    -I"$ROOT/platform/h700/storage/mmc" \
    -I"$ROOT/platform/h700/storage/gpt" \
    -I"$ROOT/platform/h700/storage/fat" \
    $SOURCES -o "$OUT/test-h700-fat-vfs-backend-sanitized" >/dev/null 2>&1; then
    ASAN_OPTIONS=detect_leaks=0 "$OUT/test-h700-fat-vfs-backend-sanitized"
else
    echo "SKIP: compiler does not provide AddressSanitizer/UndefinedBehaviorSanitizer"
fi
