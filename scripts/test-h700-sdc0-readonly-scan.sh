#!/bin/sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
OUT="/tmp/h700-sdc0-readonly-scan-$$"
trap 'rm -rf "$OUT"' EXIT HUP INT TERM
mkdir -p "$OUT"

SOURCES="
$ROOT/platform/h700/storage/mmc/h700_sunxi_sdc0_readonly_scan.c
$ROOT/platform/h700/storage/mmc/h700_sunxi_mmc_block_device.c
$ROOT/platform/h700/storage/mmc/sunxi_h616_sdc0_pins.c
$ROOT/platform/h700/storage/mmc/sunxi_h616_sdc0_platform.c
$ROOT/platform/h700/storage/mmc/sunxi_mmc_clock.c
$ROOT/platform/h700/storage/mmc/sunxi_mmc_module_clock.c
$ROOT/platform/h700/storage/mmc/sunxi_mmc_pio_read.c
$ROOT/platform/h700/storage/mmc/sunxi_sd_card_init.c
$ROOT/platform/h700/storage/mmc/sunxi_sd_csd.c
$ROOT/platform/h700/storage/gpt/h700_gpt_readonly.c
$ROOT/platform/h700/storage/mmc/tests/h700_sunxi_sdc0_readonly_scan_test.c
"

# shellcheck disable=SC2086
cc -std=c11 -O2 -Wall -Wextra -Werror \
    -I"$ROOT/platform/h700/storage/mmc" \
    -I"$ROOT/platform/h700/storage/gpt" \
    $SOURCES -o "$OUT/test-h700-sdc0-readonly-scan"
"$OUT/test-h700-sdc0-readonly-scan"

# Run the identical integrated path with runtime memory/UB checks when available.
# shellcheck disable=SC2086
if cc -std=c11 -O1 -g -Wall -Wextra -Werror \
    -fsanitize=address,undefined -fno-omit-frame-pointer \
    -I"$ROOT/platform/h700/storage/mmc" \
    -I"$ROOT/platform/h700/storage/gpt" \
    $SOURCES -o "$OUT/test-h700-sdc0-readonly-scan-sanitized" >/dev/null 2>&1; then
    ASAN_OPTIONS=detect_leaks=0 "$OUT/test-h700-sdc0-readonly-scan-sanitized"
else
    echo "SKIP: compiler does not provide AddressSanitizer/UndefinedBehaviorSanitizer"
fi
