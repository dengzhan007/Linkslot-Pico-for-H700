#!/bin/sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
OUT="/tmp/h700-fat-partition-bounds-$$"
trap 'rm -rf "$OUT"' EXIT HUP INT TERM
mkdir -p "$OUT"
cc -std=c11 -O2 -Wall -Wextra -Werror \
    -I"$ROOT/platform/h700/storage/fat" \
    "$ROOT/platform/h700/storage/fat/fat_ro.c" \
    "$ROOT/platform/h700/storage/fat/fat_exfat.c" \
    "$ROOT/platform/h700/storage/fat/tests/fat_partition_bounds_test.c" \
    -o "$OUT/test-fat-partition-bounds"
"$OUT/test-fat-partition-bounds"
