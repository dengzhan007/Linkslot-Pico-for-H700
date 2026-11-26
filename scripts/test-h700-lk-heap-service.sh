#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CC="${CC:-cc}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

"$CC" -std=c11 -Wall -Wextra -Werror \
    -I"$ROOT/platform/h700/runtime" \
    -I"$ROOT/platform/h700/runtime/tests/freestanding_include" \
    "$ROOT/platform/h700/runtime/lk_heap_service.c" \
    "$ROOT/target/rg34xx_h700/init.c" \
    "$ROOT/platform/h700/runtime/tests/lk_heap_service_test.c" \
    -o "$TMP_DIR/lk_heap_service_test"

"$TMP_DIR/lk_heap_service_test"
