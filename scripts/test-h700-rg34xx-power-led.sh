#!/bin/sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
OUT="${TMPDIR:-/tmp}/h700-rg34xx-power-led-test-$$"
trap 'rm -rf "$OUT"' EXIT HUP INT TERM
mkdir -p "$OUT"
cc -std=c11 -Wall -Wextra -Werror -I"$ROOT/platform/h700/power" \
  "$ROOT/platform/h700/power/h700_rg34xx_power_led.c" \
  "$ROOT/platform/h700/power/tests/h700_rg34xx_power_led_test.c" \
  -o "$OUT/test-h700-rg34xx-power-led"
"$OUT/test-h700-rg34xx-power-led"
