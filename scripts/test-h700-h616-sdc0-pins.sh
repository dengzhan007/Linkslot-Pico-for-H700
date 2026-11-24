#!/bin/sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
OUT="/tmp/h700-h616-sdc0-pins-$$"
trap 'rm -rf "$OUT"' EXIT HUP INT TERM
mkdir -p "$OUT"
cc -std=c11 -O2 -Wall -Wextra -Werror \
    -I"$ROOT/platform/h700/storage/mmc" \
    "$ROOT/platform/h700/storage/mmc/sunxi_h616_sdc0_pins.c" \
    "$ROOT/platform/h700/storage/mmc/tests/sunxi_h616_sdc0_pins_test.c" \
    -o "$OUT/test-h700-h616-sdc0-pins"
"$OUT/test-h700-h616-sdc0-pins"
if cc -std=c11 -O1 -g -Wall -Wextra -Werror \
    -fsanitize=address,undefined -fno-omit-frame-pointer \
    -I"$ROOT/platform/h700/storage/mmc" \
    "$ROOT/platform/h700/storage/mmc/sunxi_h616_sdc0_pins.c" \
    "$ROOT/platform/h700/storage/mmc/tests/sunxi_h616_sdc0_pins_test.c" \
    -o "$OUT/test-h700-h616-sdc0-pins-sanitized" >/dev/null 2>&1; then
    ASAN_OPTIONS=detect_leaks=0 "$OUT/test-h700-h616-sdc0-pins-sanitized"
else
    echo "SKIP: compiler does not provide AddressSanitizer/UndefinedBehaviorSanitizer"
fi
