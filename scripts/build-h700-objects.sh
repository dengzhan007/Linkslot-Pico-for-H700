#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
H700_ARCH="${H700_ARCH:-aarch64}"
OUT_DIR="${OUT_DIR:-$ROOT/build/h700-arch-check/$H700_ARCH}"
CC="${H700_CC:-${AARCH64_CC:-clang}}"
CXX="${H700_CXX:-${AARCH64_CXX:-clang++}}"
OBJDUMP="${LLVM_OBJDUMP:-llvm-objdump}"

for tool in "$CC" "$CXX" "$OBJDUMP"; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "error: required tool '$tool' not found" >&2
        exit 1
    fi
done

mkdir -p "$OUT_DIR"

case "$H700_ARCH" in
    aarch64)
        TARGET_FLAGS=(
            --target=aarch64-none-elf
            -mcpu=cortex-a53
            -mgeneral-regs-only
        )
        EXPECTED_ARCH="aarch64"
        C_PROBE="$ROOT/platform/h700/arch/compile_probe.c"
        ASM_PROBE="$ROOT/platform/h700/arch/compile_probe.S"
        CPP_PROBE="$ROOT/platform/h700/arch/compile_probe.cpp"
        ;;
    aarch32)
        # Match the GammaOS LK A32 BL33 ABI: ARM state, Cortex-A53, NEON/VFPv4,
        # and the base AAPCS float calling convention (softfp).
        TARGET_FLAGS=(
            --target=armv7a-none-eabi
            -mcpu=cortex-a53
            -marm
            -mfpu=neon-vfpv4
            -mfloat-abi=softfp
        )
        EXPECTED_ARCH="arm"
        C_PROBE="$ROOT/platform/h700/arch/compile_probe_a32.c"
        ASM_PROBE="$ROOT/platform/h700/arch/compile_probe_a32.S"
        CPP_PROBE="$ROOT/platform/h700/arch/compile_probe_a32.cpp"
        ;;
    *)
        echo "error: H700_ARCH must be 'aarch64' or 'aarch32' (got '$H700_ARCH')" >&2
        exit 2
        ;;
esac

COMMON_FLAGS=(
    "${TARGET_FLAGS[@]}"
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
    -c "$C_PROBE" \
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
    -c "$ROOT/platform/h700/storage/mmc/sunxi_h616_sdc0_platform.c" \
    -o "$OUT_DIR/sunxi_h616_sdc0_platform.o"

"$CC" "${COMMON_FLAGS[@]}" -std=c11 \
    -I"$ROOT/platform/h700/storage/mmc" \
    -c "$ROOT/platform/h700/storage/mmc/sunxi_h616_sdc0_pins.c" \
    -o "$OUT_DIR/sunxi_h616_sdc0_pins.o"

"$CC" "${COMMON_FLAGS[@]}" -std=c11 \
    -I"$ROOT/platform/h700/storage/gpt/tests/freestanding_include" \
    -I"$ROOT/platform/h700/storage/mmc" \
    -I"$ROOT/platform/h700/storage/gpt" \
    -c "$ROOT/platform/h700/storage/mmc/h700_sunxi_sdc0_readonly_scan.c" \
    -o "$OUT_DIR/h700_sunxi_sdc0_readonly_scan.o"

"$CC" "${COMMON_FLAGS[@]}" -std=c11 \
    -I"$ROOT/platform/h700/storage/mmc" \
    -I"$ROOT/platform/h700/storage/gpt" \
    -c "$ROOT/platform/h700/storage/mmc/h700_sunxi_mmc_block_device.c" \
    -o "$OUT_DIR/h700_sunxi_mmc_block_device.o"

"$CC" "${COMMON_FLAGS[@]}" -std=c11 \
    -I"$ROOT/platform/h700/storage/mmc" \
    -c "$ROOT/platform/h700/storage/mmc/sunxi_sd_csd.c" \
    -o "$OUT_DIR/sunxi_sd_csd.o"

"$CC" "${COMMON_FLAGS[@]}" -std=c11 \
    -I"$ROOT/platform/h700/storage/gpt/tests/freestanding_include" \
    -I"$ROOT/platform/h700/storage/gpt" \
    -c "$ROOT/platform/h700/storage/gpt/h700_gpt_readonly.c" \
    -o "$OUT_DIR/h700_gpt_readonly.o"

"$CC" "${COMMON_FLAGS[@]}" -std=c11 \
    -I"$ROOT/platform/h700/storage/gpt/tests/freestanding_include" \
    -I"$ROOT/platform/h700/storage/gpt" \
    -I"$ROOT/platform/h700/storage/mmc" \
    -I"$ROOT/platform/h700/storage/fat" \
    -c "$ROOT/platform/h700/storage/mmc/h700_sunxi_mmc_fat.c" \
    -o "$OUT_DIR/h700_sunxi_mmc_fat.o"

"$CC" "${COMMON_FLAGS[@]}" -std=c11 \
    -I"$ROOT/platform/h700/runtime/include" \
    -I"$ROOT/platform/h700/storage/gpt/tests/freestanding_include" \
    -I"$ROOT/platform/h700/storage/gpt" \
    -I"$ROOT/platform/h700/storage/mmc" \
    -I"$ROOT/platform/h700/storage/fat" \
    -c "$ROOT/platform/h700/storage/mmc/h700_fat_vfs_backend.c" \
    -o "$OUT_DIR/h700_fat_vfs_backend.o"

"$CC" "${COMMON_FLAGS[@]}" -std=c11 \
    -I"$ROOT/platform/h700/storage/fat" \
    -c "$ROOT/platform/h700/storage/fat/fat_ro.c" \
    -o "$OUT_DIR/fat_ro.o"

"$CC" "${COMMON_FLAGS[@]}" -std=c11 \
    -I"$ROOT/platform/h700/storage/fat" \
    -c "$ROOT/platform/h700/storage/fat/fat_exfat.c" \
    -o "$OUT_DIR/fat_exfat.o"

"$CC" "${COMMON_FLAGS[@]}" -c \
    "$ASM_PROBE" \
    -o "$OUT_DIR/compile_probe_asm.o"

"$CXX" "${COMMON_FLAGS[@]}" -nostdinc++ -fno-exceptions -fno-rtti \
    -fno-threadsafe-statics -fno-use-cxa-atexit -std=c++17 \
    -c "$CPP_PROBE" \
    -o "$OUT_DIR/compile_probe_cpp.o"

for object in "$OUT_DIR/compile_probe.o" "$OUT_DIR/compile_probe_asm.o" "$OUT_DIR/compile_probe_cpp.o" "$OUT_DIR/h700_mmc_board_profile.o" "$OUT_DIR/sunxi_mmc_clock.o" "$OUT_DIR/sunxi_mmc_module_clock.o" "$OUT_DIR/sunxi_mmc_pio_read.o" "$OUT_DIR/sunxi_sd_csd.o" "$OUT_DIR/sunxi_sd_card_init.o" "$OUT_DIR/sunxi_h616_sdc0_platform.o" "$OUT_DIR/sunxi_h616_sdc0_pins.o" "$OUT_DIR/h700_sunxi_mmc_block_device.o" "$OUT_DIR/h700_sunxi_sdc0_readonly_scan.o" "$OUT_DIR/h700_gpt_readonly.o" "$OUT_DIR/h700_sunxi_mmc_fat.o" "$OUT_DIR/h700_fat_vfs_backend.o" "$OUT_DIR/fat_ro.o" "$OUT_DIR/fat_exfat.o"; do
    format="$("$OBJDUMP" -f "$object")"
    if ! grep -q "architecture: $EXPECTED_ARCH" <<<"$format"; then
        echo "error: $object does not match the requested H700 architecture ($H700_ARCH)" >&2
        printf '%s\n' "$format" >&2
        exit 1
    fi
    echo "$object: AArch64 object OK"
done

echo "H700 compile-only $H700_ARCH architecture check passed."
echo "Note: this is compile-only. It does not provide a C/C++ runtime, link the full LK/HAL image, establish a BL33 load range, execute a payload, or validate H700 hardware."
