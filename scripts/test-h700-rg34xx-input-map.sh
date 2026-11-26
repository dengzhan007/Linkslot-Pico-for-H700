#!/bin/sh
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CC="${CC:-cc}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' 0
"$CC" -std=c11 -Wall -Wextra -Werror -I"$ROOT/platform/h700/runtime" \
 "$ROOT/platform/h700/runtime/input_queue.c" \
 "$ROOT/platform/h700/runtime/h700_rg34xx_input_map.c" \
 "$ROOT/platform/h700/runtime/tests/rg34xx_input_map_host_test.c" \
 -o "$TMP_DIR/h700_rg34xx_input_map_test"
"$TMP_DIR/h700_rg34xx_input_map_test"
