#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
OUT="/tmp/h700-gpt-readonly-test-$$"
trap 'rm -rf "$OUT"' EXIT HUP INT TERM
mkdir -p "$OUT"

CC=cc
"$CC" -std=c11 -O2 -Wall -Wextra -Werror \
    -I"$ROOT/platform/h700/storage/gpt" \
    "$ROOT/platform/h700/storage/gpt/h700_gpt_readonly.c" \
    "$ROOT/platform/h700/storage/gpt/tests/h700_gpt_readonly_test.c" \
    -o "$OUT/test-h700-gpt-readonly"
"$OUT/test-h700-gpt-readonly"

# Run the same suite with runtime memory/undefined-behavior checks when supported.
if "$CC" -std=c11 -O1 -g -Wall -Wextra -Werror \
    -fsanitize=address,undefined -fno-omit-frame-pointer \
    -I"$ROOT/platform/h700/storage/gpt" \
    "$ROOT/platform/h700/storage/gpt/h700_gpt_readonly.c" \
    "$ROOT/platform/h700/storage/gpt/tests/h700_gpt_readonly_test.c" \
    -o "$OUT/test-h700-gpt-readonly-sanitized" >/dev/null 2>&1; then
    ASAN_OPTIONS=detect_leaks=0 "$OUT/test-h700-gpt-readonly-sanitized"
else
    echo "SKIP: compiler does not provide AddressSanitizer/UndefinedBehaviorSanitizer"
fi
