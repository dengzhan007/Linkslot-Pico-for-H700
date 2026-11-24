#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
OUT="/tmp/h700-mmc-board-profile-test-$$"
trap 'rm -rf "$OUT"' EXIT HUP INT TERM
mkdir -p "$OUT"

CC=cc
"$CC" -std=c11 -Wall -Wextra -Werror \
    -I"$ROOT/platform/h700/storage/mmc" \
    "$ROOT/platform/h700/storage/mmc/h700_mmc_board_profile.c" \
    "$ROOT/platform/h700/storage/mmc/tests/h700_mmc_board_profile_test.c" \
    -o "$OUT/test-h700-mmc-board-profile"

"$OUT/test-h700-mmc-board-profile"
