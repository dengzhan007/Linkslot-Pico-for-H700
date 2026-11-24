#!/bin/sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
OUT="/tmp/h700-sunxi-mmc-fat-scan-$$"
trap 'rm -rf "$OUT"' EXIT HUP INT TERM
mkdir -p "$OUT"

SOURCES="
$ROOT/platform/h700/storage/mmc/h700_sunxi_mmc_fat.c
$ROOT/platform/h700/storage/mmc/h700_sunxi_mmc_block_device.c
$ROOT/platform/h700/storage/mmc/sunxi_mmc_pio_read.c
$ROOT/platform/h700/storage/fat/fat_ro.c
$ROOT/platform/h700/storage/fat/fat_exfat.c
$ROOT/platform/h700/storage/mmc/tests/h700_sunxi_mmc_fat_scan_test.c
"

# shellcheck disable=SC2086
cc -std=c11 -O2 -Wall -Wextra -Werror \
    -I"$ROOT/platform/h700/storage/mmc" \
    -I"$ROOT/platform/h700/storage/gpt" \
    -I"$ROOT/platform/h700/storage/fat" \
    $SOURCES -o "$OUT/test-h700-sunxi-mmc-fat-scan"
"$OUT/test-h700-sunxi-mmc-fat-scan"

# shellcheck disable=SC2086
if cc -std=c11 -O1 -g -Wall -Wextra -Werror \
    -fsanitize=address,undefined -fno-omit-frame-pointer \
    -I"$ROOT/platform/h700/storage/mmc" \
    -I"$ROOT/platform/h700/storage/gpt" \
    -I"$ROOT/platform/h700/storage/fat" \
    $SOURCES -o "$OUT/test-h700-sunxi-mmc-fat-scan-sanitized" >/dev/null 2>&1; then
    ASAN_OPTIONS=detect_leaks=0 "$OUT/test-h700-sunxi-mmc-fat-scan-sanitized"
else
    echo "SKIP: compiler does not provide AddressSanitizer/UndefinedBehaviorSanitizer"
fi
