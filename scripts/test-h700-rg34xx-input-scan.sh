#!/bin/sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
CC="${CC:-cc}"
OUT="${TMPDIR:-/tmp}/h700-rg34xx-input-scan-$$"
trap 'rm -rf "$OUT"' EXIT HUP INT TERM
mkdir -p "$OUT"
"$CC" -std=c11 -Wall -Wextra -Werror \
  -I"$ROOT/platform/h700/runtime" \
  "$ROOT/platform/h700/runtime/input_queue.c" \
  "$ROOT/platform/h700/runtime/h700_rg34xx_input_map.c" \
  "$ROOT/platform/h700/runtime/h700_rg34xx_input_scan.c" \
  "$ROOT/platform/h700/runtime/tests/rg34xx_input_scan_host_test.c" \
  -o "$OUT/test-h700-rg34xx-input-scan"
"$OUT/test-h700-rg34xx-input-scan"
