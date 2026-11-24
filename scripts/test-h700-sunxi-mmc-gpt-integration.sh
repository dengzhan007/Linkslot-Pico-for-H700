#!/bin/sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
OUT="/tmp/h700-sunxi-mmc-gpt-e2e-$$"
trap 'rm -rf "$OUT"' EXIT HUP INT TERM
mkdir -p "$OUT"
cc -std=c11 -O2 -Wall -Wextra -Werror \
  -I"$ROOT/platform/h700/storage/mmc" \
  -I"$ROOT/platform/h700/storage/gpt" \
  "$ROOT/platform/h700/storage/mmc/h700_sunxi_mmc_block_device.c" \
  "$ROOT/platform/h700/storage/mmc/sunxi_mmc_pio_read.c" \
  "$ROOT/platform/h700/storage/gpt/h700_gpt_readonly.c" \
  "$ROOT/platform/h700/storage/mmc/tests/h700_sunxi_mmc_gpt_integration_test.c" \
  -o "$OUT/test-gpt-through-cmd17"
"$OUT/test-gpt-through-cmd17"
if cc -std=c11 -O1 -g -Wall -Wextra -Werror -fsanitize=address,undefined \
  -fno-omit-frame-pointer \
  -I"$ROOT/platform/h700/storage/mmc" \
  -I"$ROOT/platform/h700/storage/gpt" \
  "$ROOT/platform/h700/storage/mmc/h700_sunxi_mmc_block_device.c" \
  "$ROOT/platform/h700/storage/mmc/sunxi_mmc_pio_read.c" \
  "$ROOT/platform/h700/storage/gpt/h700_gpt_readonly.c" \
  "$ROOT/platform/h700/storage/mmc/tests/h700_sunxi_mmc_gpt_integration_test.c" \
  -o "$OUT/test-gpt-through-cmd17-sanitized" >/dev/null 2>&1; then
  ASAN_OPTIONS=detect_leaks=0 "$OUT/test-gpt-through-cmd17-sanitized"
else
  echo "SKIP: compiler does not provide AddressSanitizer/UndefinedBehaviorSanitizer"
fi
