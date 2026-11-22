#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT/build/h700-arch-check}"
CC="${AARCH64_CC:-clang}"
OBJDUMP="${LLVM_OBJDUMP:-llvm-objdump}"

if ! command -v "$CC" >/dev/null 2>&1; then
    echo "error: AArch64 compiler '$CC' not found" >&2
    exit 1
fi
if ! command -v "$OBJDUMP" >/dev/null 2>&1; then
    echo "error: LLVM objdump '$OBJDUMP' not found" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"

COMMON_FLAGS=(
    --target=aarch64-none-elf
    -mcpu=cortex-a55
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

"$CC" "${COMMON_FLAGS[@]}" -c \
    "$ROOT/platform/h700/arch/compile_probe.S" \
    -o "$OUT_DIR/compile_probe_asm.o"

for object in "$OUT_DIR/compile_probe.o" "$OUT_DIR/compile_probe_asm.o"; do
    format="$("$OBJDUMP" -f "$object")"
    if ! grep -q "architecture: aarch64" <<<"$format"; then
        echo "error: $object is not an AArch64 object" >&2
        printf '%s\n' "$format" >&2
        exit 1
    fi
    echo "$object: AArch64 object OK"
done

echo "H700 compile-only architecture check passed."
echo "Note: this does not produce a bootable payload or validate a U-Boot handoff."
