#!/bin/sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
OUT="/tmp/h700-sunxi-sd-csd-$$"
trap 'rm -rf "$OUT"' EXIT HUP INT TERM
mkdir -p "$OUT"
cc -std=c11 -O2 -Wall -Wextra -Werror \
    -I"$ROOT/platform/h700/storage/mmc" \
    "$ROOT/platform/h700/storage/mmc/sunxi_sd_csd.c" \
    "$ROOT/platform/h700/storage/mmc/tests/sunxi_sd_csd_test.c" \
    -o "$OUT/test-h700-sunxi-sd-csd"
"$OUT/test-h700-sunxi-sd-csd"
if cc -std=c11 -O1 -g -Wall -Wextra -Werror \
    -fsanitize=address,undefined -fno-omit-frame-pointer \
    -I"$ROOT/platform/h700/storage/mmc" \
    "$ROOT/platform/h700/storage/mmc/sunxi_sd_csd.c" \
    "$ROOT/platform/h700/storage/mmc/tests/sunxi_sd_csd_test.c" \
    -o "$OUT/test-h700-sunxi-sd-csd-sanitized" >/dev/null 2>&1; then
    ASAN_OPTIONS=detect_leaks=0 "$OUT/test-h700-sunxi-sd-csd-sanitized"
else
    echo "SKIP: compiler does not provide AddressSanitizer/UndefinedBehaviorSanitizer"
fi
