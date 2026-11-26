#!/usr/bin/env bash
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CC="${CC:-cc}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' 0

# Normal LK bootstrap: heap_init() precedes target_init().
"$CC" -std=c11 -Wall -Wextra -Werror -DH700_RG34XX_PI12_EARLY_ACCESS_ENABLE=1 \
    -I"$ROOT/platform/h700/runtime" \
    -I"$ROOT/platform/h700/runtime/tests/freestanding_include" \
    "$ROOT/platform/h700/runtime/lk_heap_service.c" \
    "$ROOT/target/rg34xx_h700/init.c" \
    "$ROOT/platform/h700/runtime/tests/lk_heap_service_test.c" \
    -o "$TMP_DIR/lk_heap_service_test"
"$TMP_DIR/lk_heap_service_test"

# Alternate NAND-write bootstrap calls target_init() without heap_init().
"$CC" -std=c11 -Wall -Wextra -Werror -DH700_RG34XX_PI12_EARLY_ACCESS_ENABLE=1 -DENABLE_NANDWRITE=1 \
    -I"$ROOT/platform/h700/runtime" \
    -I"$ROOT/platform/h700/runtime/tests/freestanding_include" \
    "$ROOT/platform/h700/runtime/lk_heap_service.c" \
    "$ROOT/target/rg34xx_h700/init.c" \
    "$ROOT/platform/h700/runtime/tests/lk_heap_service_test.c" \
    -o "$TMP_DIR/lk_heap_service_nandwrite_test"
"$TMP_DIR/lk_heap_service_nandwrite_test"
