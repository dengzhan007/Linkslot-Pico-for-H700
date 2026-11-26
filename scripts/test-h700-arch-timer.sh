#!/bin/sh
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CC="${CC:-cc}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' 0

"$CC" -std=c11 -Wall -Wextra -Werror \
    -DH700_ARCH_TIMER_HOST_TEST=1 \
    -I"$ROOT/platform/h700/runtime" \
    "$ROOT/platform/h700/runtime/h700_arch_timer.c" \
    "$ROOT/platform/h700/runtime/tests/arch_timer_host_test.c" \
    -o "$TMP_DIR/h700_arch_timer_test"

"$TMP_DIR/h700_arch_timer_test"
