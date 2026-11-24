#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
OUT="/tmp/h700-sunxi-mmc-clock-test-$$"
trap 'rm -rf "$OUT"' EXIT HUP INT TERM
mkdir -p "$OUT"

cc -std=c11 -Wall -Wextra -Werror \
    -I"$ROOT/platform/h700/storage/mmc" \
    "$ROOT/platform/h700/storage/mmc/sunxi_mmc_clock.c" \
    "$ROOT/platform/h700/storage/mmc/tests/sunxi_mmc_clock_test.c" \
    -o "$OUT/test-h700-sunxi-mmc-clock"

"$OUT/test-h700-sunxi-mmc-clock"
