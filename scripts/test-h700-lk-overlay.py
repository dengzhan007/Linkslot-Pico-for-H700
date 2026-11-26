#!/usr/bin/env python3
"""Compile/link the generated GammaOS LK A32 BL33 overlay with minimal stubs."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import struct
import subprocess
import sys
from pathlib import Path

LK_PIN = "9bf1419a49e0af6dba8b05916df382c9c9c6d0a8"
IMAGE_BASE = 0x4A000000
HEADER_END = 0x640
VECTOR_OFFSET = 0x1000
IMAGE_END = 0x4A0E0000


def run(command: list[str], log: Path | None = None) -> str:
    proc = subprocess.run(
        command, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT
    )
    if log:
        log.parent.mkdir(parents=True, exist_ok=True)
        log.write_text(proc.stdout, encoding="utf-8")
    if proc.returncode:
        print(proc.stdout[-16000:], file=sys.stderr)
        raise RuntimeError("command failed: " + " ".join(command))
    return proc.stdout


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def symbol_address(text: str, symbol: str) -> int:
    for line in text.splitlines():
        fields = line.split()
        if len(fields) >= 3 and fields[-1] == symbol:
            return int(fields[0], 16)
    raise RuntimeError("linked symbol missing: " + symbol)


def decode_arm_branch(word: int, pc: int) -> int:
    if (word & 0x0F000000) != 0x0A000000 or (word & 0x01000000):
        raise RuntimeError(f"expected A32 B instruction, got {word:#010x}")
    imm = word & 0x00FFFFFF
    if imm & 0x00800000:
        imm -= 1 << 24
    return pc + 8 + (imm << 2)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--lk-source", type=Path, required=True)
    parser.add_argument("--overlay", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    lk = args.lk_source.resolve()
    overlay = args.overlay.resolve()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)

    for tool in ("clang", "ld.lld", "llvm-nm", "llvm-objcopy", "llvm-objdump", "readelf", "git"):
        if not shutil.which(tool):
            raise RuntimeError("required tool missing: " + tool)

    pin = run(["git", "-C", str(lk), "rev-parse", "HEAD"]).strip()
    if pin != LK_PIN:
        raise RuntimeError(f"pinned LK mismatch: expected {LK_PIN}, got {pin}")

    manifest_path = overlay / "manifest.json"
    patch_path = overlay / "h700-bl33-overlay.patch"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("source_commit") != LK_PIN or manifest.get("git_apply_check") != "pass":
        raise RuntimeError("overlay manifest does not identify a verified pinned-LK patch")
    if sha256(patch_path.read_bytes()) != manifest.get("patch_sha256"):
        raise RuntimeError("overlay patch SHA-256 does not match its manifest")

    overlay_makefile = overlay / "files/makefile"
    overlay_dev_rules = overlay / "files/dev/rules.mk"
    overlay_target_rules = overlay / "files/target/rules.mk"
    overlay_crt0 = overlay / "files/arch/arm/crt0.S"
    overlay_arch_c = overlay / "files/arch/arm/arch.c"
    overlay_rules = overlay / "files/arch/arm/rules.mk"
    for path in (
        overlay_makefile,
        overlay_dev_rules,
        overlay_target_rules,
        overlay_crt0,
        overlay_arch_c,
        overlay_rules,
    ):
        if not path.is_file():
            raise RuntimeError("overlay replacement file missing: " + str(path))

    makefile_text = overlay_makefile.read_text(encoding="utf-8")
    if not re.search(
        r"ifneq \(\$\(H700_BUILD\),yes\)\s+.*?include platform/common/rules\.mk\s+endif",
        makefile_text,
        re.S,
    ):
        raise RuntimeError("top-level makefile does not isolate the MediaTek platform/common stack")

    if not re.search(
        r"ifneq \(\$\(H700_BUILD\),yes\)\s+.*?include platform/pal/rules\.mk\s+endif",
        makefile_text,
        re.S,
    ):
        raise RuntimeError("top-level makefile still includes MediaTek PAL timer/log/mutex objects for H700")

    if not all(
        item in makefile_text
        for item in (
            "TOOLCHAIN_PREFIX ?= arm-none-eabi-",
            "LD := $(TOOLCHAIN_PREFIX)ld",
            "TOOLCHAIN_PREFIX =gcc/bin/arm-linux-androideabi-",
        )
    ):
        raise RuntimeError("top-level makefile does not select a separate H700 ARM EABI toolchain")

    project_include = makefile_text.find("include project/$(PROJECT).mk")
    early_toolchain = makefile_text.find("TOOLCHAIN_PREFIX ?= arm-none-eabi-")
    arch_include = makefile_text.find("include arch/$(ARCH)/rules.mk")
    if not (0 <= project_include < early_toolchain < arch_include):
        raise RuntimeError("H700 ARM EABI toolchain is selected too late for arch/arm/rules.mk LIBGCC detection")

    dev_text = overlay_dev_rules.read_text(encoding="utf-8")
    if not re.search(
        r"ifneq \(\$\(H700_BUILD\),yes\)\s+MODULES \+=.*?\$\(LOCAL_DIR\)/md_com\s+endif",
        dev_text,
        re.S,
    ):
        raise RuntimeError("dev/rules.mk still adds MediaTek-only device modules to H700")
    if not re.search(
        r"ifneq \(\$\(H700_BUILD\),yes\)\s+include \$\(LOCAL_DIR\)/logo/rules\.mk\s+endif",
        dev_text,
        re.S,
    ):
        raise RuntimeError("dev/rules.mk still includes the MediaTek boot-logo builder for H700")

    target_text = overlay_target_rules.read_text(encoding="utf-8")
    if not re.search(
        r"ifneq \(\$\(H700_BUILD\),yes\)\s+OBJS \+= \$\(LOCAL_DIR\)/init\.o\s+endif\s+OBJS \+= \$\(LOCAL_DIR\)/target_display\.o",
        target_text,
        re.S,
    ):
        raise RuntimeError("target/rules.mk does not exclude MediaTek target/init.c for H700")

    arch_text = overlay_arch_c.read_text(encoding="utf-8")
    if "set_vector_base((addr_t)__lk_vectors_start);" not in arch_text:
        raise RuntimeError("arch.c does not preserve the relocated H700 vector base")
    if "set_vector_base(MEMBASE);" in arch_text:
        raise RuntimeError("arch.c still resets VBAR to MEMBASE")
    rules_text = overlay_rules.read_text(encoding="utf-8")
    a53_branch = re.search(
        r"ifeq \(\$\(ARM_CPU\),cortex-a53\)(.*?)HANDLED_CORE := true",
        rules_text,
        re.S,
    )
    if not a53_branch or not all(flag in a53_branch.group(1) for flag in ("-mcpu=cortex-a53", "-mfpu=neon-vfpv4", "-mfloat-abi=softfp")):
        raise RuntimeError("Cortex-A53 rules branch lacks the Cortex-A53 + NEON/VFPv4 + softfp ABI flags")
    crt0_text = overlay_crt0.read_text(encoding="utf-8")
    if "ARM_CPU_CORTEX_A53" in crt0_text:
        raise RuntimeError("overlay unexpectedly guesses Cortex-A53 ACTLR.SMP setup")
    entry_match = re.search(r"lk_bl33_entry:\n(?P<body>.*?)\nreset:", crt0_text, re.S)
    if entry_match is None:
        raise RuntimeError("BL33 entry wrapper is missing")
    entry_body = entry_match.group("body")
    capture_ops = ("mrs r5, cpsr", "mov r6, sp", "mov r7, lr")
    capture_positions = [entry_body.find(op) for op in capture_ops]
    vbar_position = entry_body.find("mcr p15, 0, r0, c12, c0, 0")
    if min(capture_positions) < 0 or capture_positions != sorted(capture_positions):
        raise RuntimeError("entry SP/LR/r4/CPSR must be captured in order at BL33 entry")
    if vbar_position < 0 or capture_positions[-1] > vbar_position:
        raise RuntimeError("entry register capture must precede the VBAR write")
    publish_ops = (
        "str r6, [r0, #0]",
        "str r7, [r0, #4]",
        "str r4, [r0, #8]",
        "str r5, [r0, #12]",
        "str r1, [r0, #16]",
    )
    if not all(op in crt0_text for op in publish_ops):
        raise RuntimeError("CRT0 must publish the four unbanked entry values and write magic last")
    if "ldr     r0, =BOOT_ARGUMENT_LOCATION" not in crt0_text or "str     r4, [r0]" not in crt0_text:
        raise RuntimeError("CRT0 must preserve the original r4 boot-argument handoff without clobbering snapshot r6")
    if "ldr r0, =_binary_tzbsp_tzbsp_bin_start" not in crt0_text:
        raise RuntimeError("optional TrustZone reference must not clobber snapshot r7")
    if re.search(r"\b(?:r8|r9|r10|r11)\b", entry_body):
        raise RuntimeError("entry snapshot must use unbanked r4-r7, not FIQ-banked r8-r11")
    clear_loop = re.search(r"blt\s+\.L__bss_loop", crt0_text)
    publish_pos = crt0_text.find("ldr r0, =h700_bl33_entry_snapshot")
    kmain_match = re.search(r"\bbl\s+kmain\b", crt0_text)
    if clear_loop is None or publish_pos <= clear_loop.end():
        raise RuntimeError("entry snapshot must be published after the CRT0 BSS clear")
    if kmain_match is None or publish_pos > kmain_match.start():
        raise RuntimeError("entry snapshot must be published before kmain")

    stubs = r'''volatile int critical_section_count = 1;
volatile unsigned int h700_bl33_entry_snapshot[5];
void kmain(void) { for (;;) { __asm__ volatile("wfi"); } }
void arm_undefined_handler(void *frame) { (void)frame; for (;;) { __asm__ volatile("wfi"); } }
void arm_syscall_handler(void *frame) { (void)frame; for (;;) { __asm__ volatile("wfi"); } }
void arm_prefetch_abort_handler(void *frame) { (void)frame; for (;;) { __asm__ volatile("wfi"); } }
void arm_data_abort_handler(void *frame) { (void)frame; for (;;) { __asm__ volatile("wfi"); } }
int platform_irq(void *frame) { (void)frame; return 0; }
void platform_fiq(void *frame) { (void)frame; }
void thread_preempt(void) { }
'''
    stubs_path = out / "lk-bl33-stubs.c"
    stubs_path.write_text(stubs, encoding="utf-8")

    flags = [
        "--target=armv7a-none-eabi",
        "-mcpu=cortex-a53",
        "-marm",
        "-mfpu=neon-vfpv4",
        "-mfloat-abi=softfp",
        "-ffreestanding",
        "-fno-builtin",
        "-fno-pic",
        "-fno-pie",
        "-fno-stack-protector",
        "-fno-unwind-tables",
        "-fno-asynchronous-unwind-tables",
        "-I",
        str(lk / "include"),
        "-I",
        str(lk / "arch/arm/include"),
        "-DARM_WITH_CP15=1",
        "-DARM_WITH_MMU=1",
        "-DARM_ISA_ARMv7=1",
        "-DARM_WITH_VFP=1",
        "-DARM_WITH_NEON=1",
        "-DARM_WITH_THUMB=1",
        "-DARM_WITH_THUMB2=1",
        "-DARM_WITH_CACHE=1",
        "-DARM_WITH_L2=1",
    ]

    crt_obj = out / "crt0-bl33-overlay.o"
    exc_obj = out / "exceptions.o"
    stubs_obj = out / "lk-bl33-stubs.o"
    run(
        ["clang", *flags, "-c", str(overlay_crt0), "-o", str(crt_obj)],
        out / "crt0-compile.log",
    )
    run(
        ["clang", *flags, "-c", str(lk / "arch/arm/exceptions.S"), "-o", str(exc_obj)],
        out / "exceptions-compile.log",
    )
    run(
        ["clang", *flags, "-std=c11", "-c", str(stubs_path), "-o", str(stubs_obj)],
        out / "stubs-compile.log",
    )

    crt_symbols = run(["llvm-nm", "-g", "--defined-only", str(crt_obj)])
    for required in ("_start", "lk_bl33_entry", "__lk_vectors_start", "BOOT_ARGUMENT_LOCATION"):
        if not any(line.split() and line.split()[-1] == required for line in crt_symbols.splitlines()):
            raise RuntimeError("patched CRT0 object missing symbol: " + required)

    exc_symbols = run(["llvm-nm", "-g", "--defined-only", str(exc_obj)])
    for required in (
        "arm_undefined",
        "arm_syscall",
        "arm_prefetch_abort",
        "arm_data_abort",
        "arm_reserved",
        "arm_irq",
        "arm_fiq",
    ):
        if not any(line.split() and line.split()[-1] == required for line in exc_symbols.splitlines()):
            raise RuntimeError("exception object missing vector handler: " + required)

    linker_script = f'''\
OUTPUT_FORMAT("elf32-littlearm", "elf32-littlearm", "elf32-littlearm")
OUTPUT_ARCH(arm)
ENTRY(_start)
SECTIONS
{{
  . = 0x{IMAGE_BASE:08x};
  .text.boot : {{ KEEP(*(.text.boot)) }}
  .text : {{ *(.text .text.* .glue_7* .gnu.linkonce.t.*) }}
  .rodata : {{ *(.rodata .rodata.* .gnu.linkonce.r.*) }}
  __data_start_rom = .;
  __data_start = .;
  .data : {{ *(.data .data.* .gnu.linkonce.d.*) }}
  __data_end = .;
  . = ALIGN(4);
  .bss (NOLOAD) : {{ __bss_start = .; *(.bss .bss.*) *(COMMON) }}
  . = ALIGN(4);
  _end = .;
  _end_of_ram = 0x{IMAGE_END:08x};
  ASSERT(_end <= _end_of_ram, "LK BL33 stub link exceeds the researched image arena")
  /DISCARD/ : {{ *(.ARM.exidx*) *(.ARM.extab*) *(.comment) *(.ARM.attributes) *(.note*) }}
}}
'''
    ld_script = out / "lk-bl33-overlay-layout.ld"
    ld_script.write_text(linker_script, encoding="utf-8")
    elf = out / "lk-bl33-overlay-layout.elf"
    map_path = out / "lk-bl33-overlay-layout.map"
    run(
        [
            "ld.lld",
            "-m",
            "armelf",
            "-static",
            "--no-undefined",
            "-T",
            str(ld_script),
            "-Map=" + str(map_path),
            "-o",
            str(elf),
            str(crt_obj),
            str(exc_obj),
            str(stubs_obj),
        ],
        out / "link.log",
    )

    header = run(["readelf", "-h", str(elf)])
    if "ELF32" not in header or "ARM" not in header:
        raise RuntimeError("linked overlay probe is not ELF32/ARM")
    nm = run(["llvm-nm", "-n", str(elf)])
    undefined = run(["llvm-nm", "-u", str(elf)])
    if undefined.strip():
        raise RuntimeError("overlay probe has unresolved symbols:\n" + undefined)

    start = symbol_address(nm, "_start")
    entry = symbol_address(nm, "lk_bl33_entry")
    vectors = symbol_address(nm, "__lk_vectors_start")
    boot_argument = symbol_address(nm, "BOOT_ARGUMENT_LOCATION")
    end = symbol_address(nm, "_end")
    snapshot = symbol_address(nm, "h700_bl33_entry_snapshot")
    if start != IMAGE_BASE:
        raise RuntimeError(f"unexpected image entry: {start:#x}")
    if vectors != IMAGE_BASE + VECTOR_OFFSET:
        raise RuntimeError(f"unexpected VBAR/vector placement: {vectors:#x}")
    if boot_argument < vectors + 0x20 or entry < boot_argument + 4:
        raise RuntimeError("boot-argument scratch or BL33 entry overlaps the vector table")
    if end >= IMAGE_END:
        raise RuntimeError(f"linked stub exceeds image arena: _end={end:#x}")
    if snapshot < vectors or snapshot + 20 > end:
        raise RuntimeError("five-word BL33 entry snapshot must be contained in the linked post-vector BSS")

    binary = out / "lk-bl33-overlay-layout.bin"
    run(["llvm-objcopy", "-O", "binary", str(elf), str(binary)])
    data = bytearray(binary.read_bytes())
    if len(data) <= max(HEADER_END, VECTOR_OFFSET + 0x20):
        raise RuntimeError("flat overlay probe is too small to contain its header/vector layout")

    first_word = struct.unpack_from("<I", data, 0)[0]
    branch_target = decode_arm_branch(first_word, IMAGE_BASE)
    if branch_target != entry:
        raise RuntimeError(
            f"BL33 first branch targets {branch_target:#x}, expected {entry:#x}"
        )

    vector_words = struct.unpack_from("<8I", data, VECTOR_OFFSET)
    vector_symbols = (
        "reset",
        "arm_undefined",
        "arm_syscall",
        "arm_prefetch_abort",
        "arm_data_abort",
        "arm_reserved",
        "arm_irq",
        "arm_fiq",
    )
    for index, (word, symbol) in enumerate(zip(vector_words, vector_symbols, strict=True)):
        actual_target = decode_arm_branch(word, vectors + index * 4)
        expected_target = symbol_address(nm, symbol)
        if actual_target != expected_target:
            raise RuntimeError(
                f"vector {index} targets {actual_target:#x}, expected {symbol} at {expected_target:#x}"
            )

    # Exercise the vendor-header replacement contract with synthetic bytes.
    header_material = hashlib.sha256(b"GammaOS-LK-H700-BL33-header-layout-test").digest()
    synthetic_header = (header_material * ((HEADER_END - 4 + 31) // 32))[: HEADER_END - 4]
    original_entry = bytes(data[:4])
    data[4:HEADER_END] = synthetic_header
    if bytes(data[:4]) != original_entry or bytes(data[4:HEADER_END]) != synthetic_header:
        raise RuntimeError("synthetic vendor-header overlay changed the BL33 entry branch")
    overlay_bin = out / "lk-bl33-overlay-layout-synthetic-header.bin"
    overlay_bin.write_bytes(data)

    disassembly = run(
        [
            "llvm-objdump",
            "-d",
            "--start-address=0x4a000000",
            "--stop-address=0x4a001080",
            str(elf),
        ]
    )
    (out / "lk-bl33-overlay-layout.disasm.txt").write_text(disassembly, encoding="utf-8")

    report = [
        "# GammaOS LK H700 BL33 overlay assembly/link validation\n\n",
        f"- Pinned source: {pin}\n",
        f"- Overlay patch SHA-256: {manifest['patch_sha256']}\n",
        "- Result: PASS for the real patched GammaOS LK CRT0 + original LK exception assembly linked with minimal stubs.\n",
        f"- ELF: ELF32/ARM; entry 0x{start:08x}; flat image bytes {len(data)}.\n",
        f"- BL33 branch target: 0x{branch_target:08x}.\n",
        f"- Exception vector table: 0x{vectors:08x} (+0x{vectors - IMAGE_BASE:x}).\n",
        f"- Boot-argument scratch: 0x{boot_argument:08x}.\n",
        f"- BL33 entry wrapper: 0x{entry:08x}.\n",
        f"- Linked _end: 0x{end:08x}; upper probe bound 0x{IMAGE_END:08x}.\n",
        "- All eight vector instructions decode as A32 branch instructions; all required exception symbols resolve.\n",
        "- The generated LK make rules exclude MTK platform/common, platform/pal, MTK dev modules and MTK target/init.c when H700_BUILD=yes; H700 must provide its own platform/target module lists.\n",
        "- Synthetic bytes +4..+0x63f replace cleanly without changing the first BL33 branch; the synthetic bytes are not a vendor header.\n\n",
        "## Explicit limits\n\n",
        "- This links the actual modified GammaOS LK CRT0 and original exception assembly with tiny stubs. It is not the full LK kernel/platform build, does not include H700 platform or PAL service objects, and is not a bootable image.\n",
        "- The 0x4a000000..0x4a0e0000 boundary is only the previously researched BaseOS loader image bound; it is not an approved RAM allocation or payload arena.\n",
        "- The overlay intentionally does not set ACTLR.SMP, release or park secondary CPUs, choose H700 MMU/heap/stack ranges, initialize GIC/CCU/GPIO/SD, or implement console/RTC services.\n",
        "- No firmware is flashed, no SD/boot sector is written, and no payload is executed.\n",
    ]
    (out / "REPORT.md").write_text("".join(report), encoding="utf-8")
    print("".join(report))


if __name__ == "__main__":
    main()
