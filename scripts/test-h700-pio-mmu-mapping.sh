#!/bin/sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
OUT="${TMPDIR:-/tmp}/h700-pio-mmu-test-$$"
trap 'rm -rf "$OUT"' EXIT HUP INT TERM
mkdir -p "$OUT"
cc -std=c11 -Wall -Wextra -Werror \
  -I"$ROOT/platform/h700/arch/tests/freestanding_include" \
  "$ROOT/platform/h700/arch/h700_pio_mmu_mapping.c" \
  "$ROOT/platform/h700/arch/tests/h700_pio_mmu_mapping_test.c" \
  -o "$OUT/test-h700-pio-mmu"

# Keep the real ARM MMU-enable barrier contract visible in the host regression.
grep -q 'dsb sy' "$ROOT/platform/h700/arch/h700_pio_mmu_mapping.c"
grep -q 'isb sy' "$ROOT/platform/h700/arch/h700_pio_mmu_mapping.c"
"$OUT/test-h700-pio-mmu"
