#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT/build/h700-arch-check}"
CC="${AARCH64_CC:-clang}"
CXX="${AARCH64_CXX:-clang++}"
OBJDUMP="${LLVM_OBJDUMP:-llvm-objdump}"
OBJCOPY="${LLVM_OBJCOPY:-llvm-objcopy}"
NM="${LLVM_NM:-llvm-nm}"
LD="${AARCH64_LD:-ld.lld}"
PYTHON="${PYTHON:-python3}"

for tool in "$CC" "$CXX" "$OBJDUMP" "$OBJCOPY" "$NM" "$LD" "$PYTHON"; do
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

"$CC" "${COMMON_FLAGS[@]}" -c \
    "$ROOT/platform/h700/arch/compile_probe.S" \
    -o "$OUT_DIR/compile_probe_asm.o"

"$CXX" "${COMMON_FLAGS[@]}" -nostdinc++ -fno-exceptions -fno-rtti \
    -fno-threadsafe-statics -fno-use-cxa-atexit -std=c++17 \
    -c "$ROOT/platform/h700/arch/compile_probe.cpp" \
    -o "$OUT_DIR/compile_probe_cpp.o"

for object in "$OUT_DIR/compile_probe.o" "$OUT_DIR/compile_probe_asm.o" "$OUT_DIR/compile_probe_cpp.o"; do
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

# Build a passive, Image-contained AArch64 entry probe. This is a host-side
# static artifact only; it is not inserted into an Android boot image and is
# never executed on the handheld by this workflow.
BOOTPROBE_DIR="$ROOT/platform/h700/bootprobe"
PROBE_OBJ="$OUT_DIR/entry-probe.o"
PROBE_ELF="$OUT_DIR/entry-probe.elf"
PROBE_BODY="$OUT_DIR/entry-probe.body.bin"
PROBE_IMAGE="$OUT_DIR/entry-probe.Image"
PROBE_MAP="$OUT_DIR/entry-probe.map"

"$CC" --target=aarch64-none-elf -mcpu=cortex-a53 \
    -mgeneral-regs-only -ffreestanding -fno-builtin -fno-stack-protector \
    -fno-pic -fno-pie -fno-unwind-tables -fno-asynchronous-unwind-tables \
    -c "$BOOTPROBE_DIR/entry.S" -o "$PROBE_OBJ"

"$LD" -m aarch64elf -T "$BOOTPROBE_DIR/payload.ld" \
    -Map="$PROBE_MAP" --build-id=none "$PROBE_OBJ" -o "$PROBE_ELF"

elf_format="$("$OBJDUMP" -f "$PROBE_ELF")"
if ! grep -q "architecture: aarch64" <<<"$elf_format"; then
    echo "error: entry probe ELF is not AArch64" >&2
    printf '%s\n' "$elf_format" >&2
    exit 1
fi

start_symbol="$("$NM" -n "$PROBE_ELF" | awk '$3 == "_start" { print $1; exit }')"
if [[ -z "$start_symbol" ]]; then
    echo "error: _start symbol missing from entry probe ELF" >&2
    exit 1
fi
entry_offset=$((0x$start_symbol - 0x40080000))
if (( entry_offset < 64 || entry_offset % 4 != 0 )); then
    echo "error: invalid Image header branch target offset: $entry_offset" >&2
    exit 1
fi

"$OBJCOPY" -O binary \
    --only-section=.text \
    --only-section=.rodata \
    --only-section=.data \
    --only-section=.payload_storage \
    "$PROBE_ELF" "$PROBE_BODY"

"$PYTHON" "$ROOT/tools/pack_aarch64_image.py" "$PROBE_BODY" \
    --entry-offset "$entry_offset" \
    --output "$PROBE_IMAGE" \
    --report "$OUT_DIR/entry-probe.Image.json"

PYTHONPATH="$ROOT/tools" "$PYTHON" - "$PROBE_IMAGE" <<'PY'
import sys
from pathlib import Path
import build_rg34xx_bootimg_payload as builder

image = Path(sys.argv[1]).read_bytes()
builder.validate_payload_image(
    image,
    slot_bytes=0x42000000 - 0x40080000,
    max_output_bytes=17_461_256,
    max_image_size=0x1138000,
)
print(f"{sys.argv[1]}: RG34XX AArch64 Image header and size limits OK ({len(image)} bytes)")
PY

echo "Passive entry probe Image was built for static review only."
echo "It has not been executed, inserted into p4, or written to any device."

