#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT/build/h700-arch-check}"
CC="${AARCH64_CC:-clang}"
CXX="${AARCH64_CXX:-clang++}"
OBJDUMP="${LLVM_OBJDUMP:-llvm-objdump}"

for tool in "$CC" "$CXX" "$OBJDUMP"; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "error: required tool '$tool' not found" >&2
        exit 1
    fi
done

mkdir -p "$OUT_DIR"

COMMON_FLAGS=(
    --target=aarch64-none-elf
    -mcpu=cortex-a53
    -mgeneral-regs-only
    -ffreestanding
    -fno-builtin
    -fno-stack-protector
    -fno-pic
    -fno-pie
    -fno-unwind-tables
    -fno-asynchronous-unwind-tables
    -Wall
    -Wextra
    -Werror
)

"$CC" "${COMMON_FLAGS[@]}" -std=c11 \
    -c "$ROOT/platform/h700/arch/compile_probe.c" \
    -o "$OUT_DIR/compile_probe.o"

"$CC" "${COMMON_FLAGS[@]}" -std=c11 \
    -I"$ROOT/platform/h700/storage/mmc" \
    -c "$ROOT/platform/h700/storage/mmc/h700_mmc_board_profile.c" \
    -o "$OUT_DIR/h700_mmc_board_profile.o"

"$CC" "${COMMON_FLAGS[@]}" -std=c11 \
    -I"$ROOT/platform/h700/storage/mmc" \
    -c "$ROOT/platform/h700/storage/mmc/sunxi_mmc_clock.c" \
    -o "$OUT_DIR/sunxi_mmc_clock.o"

"$CC" "${COMMON_FLAGS[@]}" -std=c11 \
    -I"$ROOT/platform/h700/storage/mmc" \
    -c "$ROOT/platform/h700/storage/mmc/sunxi_mmc_module_clock.c" \
    -o "$OUT_DIR/sunxi_mmc_module_clock.o"

"$CC" "${COMMON_FLAGS[@]}" -std=c11 \
    -I"$ROOT/platform/h700/storage/mmc" \
    -c "$ROOT/platform/h700/storage/mmc/sunxi_mmc_pio_read.c" \
    -o "$OUT_DIR/sunxi_mmc_pio_read.o"

"$CC" "${COMMON_FLAGS[@]}" -std=c11 \
    -I"$ROOT/platform/h700/storage/gpt/tests/freestanding_include" \
    -I"$ROOT/platform/h700/storage/mmc" \
    -c "$ROOT/platform/h700/storage/mmc/sunxi_sd_card_init.c" \
    -o "$OUT_DIR/sunxi_sd_card_init.o"

"$CC" "${COMMON_FLAGS[@]}" -std=c11 \
    -I"$ROOT/platform/h700/storage/mmc" \
    -c "$ROOT/platform/h700/storage/mmc/sunxi_sd_csd.c" \
    -o "$OUT_DIR/sunxi_sd_csd.o"

"$CC" "${COMMON_FLAGS[@]}" -std=c11 \
    -I"$ROOT/platform/h700/storage/gpt/tests/freestanding_include" \
    -I"$ROOT/platform/h700/storage/gpt" \
    -c "$ROOT/platform/h700/storage/gpt/h700_gpt_readonly.c" \
    -o "$OUT_DIR/h700_gpt_readonly.o"

"$CC" "${COMMON_FLAGS[@]}" -c \
    "$ROOT/platform/h700/arch/compile_probe.S" \
    -o "$OUT_DIR/compile_probe_asm.o"

"$CXX" "${COMMON_FLAGS[@]}" -nostdinc++ -fno-exceptions -fno-rtti \
    -fno-threadsafe-statics -fno-use-cxa-atexit -std=c++17 \
    -c "$ROOT/platform/h700/arch/compile_probe.cpp" \
    -o "$OUT_DIR/compile_probe_cpp.o"

for object in "$OUT_DIR/compile_probe.o" "$OUT_DIR/compile_probe_asm.o" "$OUT_DIR/compile_probe_cpp.o" "$OUT_DIR/h700_mmc_board_profile.o" "$OUT_DIR/sunxi_mmc_clock.o" "$OUT_DIR/sunxi_mmc_module_clock.o" "$OUT_DIR/sunxi_mmc_pio_read.o" "$OUT_DIR/sunxi_sd_csd.o" "$OUT_DIR/sunxi_sd_card_init.o" "$OUT_DIR/h700_gpt_readonly.o"; do
    format="$("$OBJDUMP" -f "$object")"
    if ! grep -q "architecture: aarch64" <<<"$format"; then
        echo "error: $object is not an AArch64 object" >&2
        printf '%s\n' "$format" >&2
        exit 1
    fi
    echo "$object: AArch64 object OK"
done

echo "H700 compile-only architecture check passed."
echo "Note: this does not provide a C/C++ runtime, produce a bootable payload, or validate a U-Boot handoff."
