#!/bin/sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
CC="${CC:-cc}"
OUT="${TMPDIR:-/tmp}/h700-data-abort-test-$$"
trap 'rm -rf "$OUT"' EXIT HUP INT TERM
mkdir -p "$OUT"
"$CC" -std=c11 -Wall -Wextra -Werror -DH700_DATA_ABORT_HOST_TEST=1 \
  -I"$ROOT/platform/h700/arch" \
  "$ROOT/platform/h700/arch/h700_data_abort_recovery.c" \
  "$ROOT/platform/h700/arch/tests/h700_data_abort_recovery_test.c" \
  -o "$OUT/test-h700-data-abort-recovery"
"$OUT/test-h700-data-abort-recovery"
