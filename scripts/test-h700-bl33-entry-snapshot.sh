#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
CC="${CC:-cc}"
if [ -n "${OUT_DIR:-}" ]; then
    mkdir -p "$OUT_DIR"
    CLEAN_OUT=0
else
    OUT_DIR="$(mktemp -d)"
    CLEAN_OUT=1
fi
cleanup() {
    if [ "$CLEAN_OUT" -eq 1 ]; then
        rm -rf "$OUT_DIR"
    fi
}
trap cleanup EXIT HUP INT TERM

"$CC" -std=c11 -Wall -Wextra -Werror \
    -I"$ROOT/platform/h700/runtime" \
    "$ROOT/platform/h700/runtime/h700_bl33_entry_snapshot.c" \
    "$ROOT/platform/h700/runtime/tests/test_bl33_entry_snapshot.c" \
    -o "$OUT_DIR/test_bl33_entry_snapshot"

"$OUT_DIR/test_bl33_entry_snapshot"
echo "H700 BL33 entry snapshot host regression passed."
