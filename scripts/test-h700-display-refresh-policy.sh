#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
CC="${CC:-cc}"
OUT_DIR="${OUT_DIR:-$(mktemp -d)}"
KEEP_OUT="${KEEP_OUT:-0}"

if [ "$KEEP_OUT" != "1" ]; then
    trap 'rm -rf "$OUT_DIR"' EXIT HUP INT TERM
else
    mkdir -p "$OUT_DIR"
fi

"$CC" -std=c11 -O2 -Wall -Wextra -Werror \
    -I"$ROOT/platform/h700/display" \
    "$ROOT/platform/h700/display/h700_display_refresh_policy.c" \
    "$ROOT/platform/h700/display/tests/h700_display_refresh_policy_test.c" \
    -o "$OUT_DIR/h700_display_refresh_policy_test"

"$OUT_DIR/h700_display_refresh_policy_test"
