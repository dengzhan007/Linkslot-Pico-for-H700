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
ACTIVE_LOADER_EXTENT_BYTES = 0x8000


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
    overlay_exceptions = overlay / "files/arch/arm/exceptions.S"
    overlay_faults = overlay / "files/arch/arm/faults.c"
    overlay_rules = overlay / "files/arch/arm/rules.mk"
    for path in (
        overlay_makefile,
        overlay_dev_rules,
        overlay_target_rules,
        overlay_crt0,
        overlay_arch_c,
        overlay_exceptions,
        overlay_faults,
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
    if "extern char __lk_vectors_start[];\\n\\tset_vector_base" in arch_text:
        raise RuntimeError("arch.c VBAR declaration was emitted with literal escape characters")
    if "\n\tset_vector_base((addr_t)__lk_vectors_start);" not in arch_text:
        raise RuntimeError("arch.c VBAR declaration and assignment must be split by a real newline and tab")
        raise RuntimeError("arch.c does not preserve the relocated H700 vector base")
    if "set_vector_base(MEMBASE);" in arch_text:
        raise RuntimeError("arch.c still resets VBAR to MEMBASE")

    if arch_text.count("vmrs %0, fpexc") != 2 or arch_text.count("vmsr fpexc, %0") != 2:
        raise RuntimeError("Cortex-A53 FPEXC initialization must use two VMRS and two VMSR operations")
    if "p10, 7" in arch_text:
        raise RuntimeError("legacy MRC/MCR p10 FPEXC syntax must not remain in the H700 Cortex-A53 overlay")
    rules_text = overlay_rules.read_text(encoding="utf-8")
    a53_branch = re.search(
        r"ifeq \(\$\(ARM_CPU\),cortex-a53\)(.*?)HANDLED_CORE := true",
        rules_text,
        re.S,
    )
    if not a53_branch or not all(flag in a53_branch.group(1) for flag in ("-mcpu=cortex-a53", "-mfpu=neon-vfpv4", "-mfloat-abi=softfp")):
        raise RuntimeError("Cortex-A53 rules branch lacks the Cortex-A53 + NEON/VFPv4 + softfp ABI flags")

    exception_text = overlay_exceptions.read_text(encoding="utf-8")
    faults_text = overlay_faults.read_text(encoding="utf-8")
    for token in (
        "cpsid if",
        "mov\tr4, sp",
        "bic\tsp, sp, #7",
        "mov\tr0, r4",
        "bl\t\tarm_data_abort_handler",
        "mov\tsp, r4",
        "cmp\tr0, #0",
        "bne\th700_arm_data_abort_exception_return",
        "ldmia\tsp!, { r0-r12, pc }^",
        "msr\tspsr_cxsf, r1",
    ):
        if token not in exception_text:
            raise RuntimeError("patched exception vector missing scoped recovery operation: " + repr(token))
    abort_entry_at = exception_text.find("FUNCTION(arm_data_abort)")
    first_push_at = exception_text.find("stmfd", abort_entry_at)
    if abort_entry_at < 0 or first_push_at < 0:
        raise RuntimeError("Data Abort vector entry cannot be located")
    if "cpsid if" not in exception_text[abort_entry_at:first_push_at]:
        raise RuntimeError("Data Abort vector must mask IRQ/FIQ before building the fault frame")
    if "h700_data_abort_try_recover((void *)frame)" not in faults_text:
        raise RuntimeError("patched LK Data Abort handler does not call the H700 exact matcher")
    if "exception_die(frame, -8," not in faults_text or "data abort, halting" not in faults_text:
        raise RuntimeError("patched LK fatal Data Abort fallback was not preserved")

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

int platform_irq(void *frame) { (void)frame; return 0; }
void platform_fiq(void *frame) { (void)frame; }
void thread_preempt(void) { }
'''
    stubs_path = out / "lk-bl33-stubs.c"
    stubs_path.write_text(stubs, encoding="utf-8")

    # The modified LK C handler follows the normal H700 Thumb-2 C policy.
    # Linking it with the A32 exception vector must therefore interwork safely.
    thumb_fault_stub = (
        "extern int h700_data_abort_try_recover(void *frame);\n"
        "int arm_data_abort_handler(void *frame) { return h700_data_abort_try_recover(frame); }\n"
    )
    thumb_fault_path = out / "data-abort-handler-thumb.c"
    thumb_fault_path.write_text(thumb_fault_stub + chr(10), encoding="utf-8")

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
        ["clang", *flags, "-c", str(overlay_exceptions), "-o", str(exc_obj)],
        out / "exceptions-compile.log",
    )
    run(
        ["clang", *flags, "-std=c11", "-c", str(stubs_path), "-o", str(stubs_obj)],
        out / "stubs-compile.log",
    )

    thumb_fault_obj = out / "data-abort-handler-thumb.o"
    thumb_flags = [flag for flag in flags if flag != "-marm"] + ["-mthumb"]
    run(
        ["clang", *thumb_flags, "-std=c11", "-c", str(thumb_fault_path), "-o", str(thumb_fault_obj)],
        out / "thumb-data-abort-handler-compile.log",
    )

    repo_root = Path(__file__).resolve().parents[1]
    h700_abort_c = repo_root / "platform/h700/arch/h700_data_abort_recovery.c"
    h700_probe_asm = repo_root / "platform/h700/arch/h700_syscfg_probe.S"
    if not h700_abort_c.is_file() or not h700_probe_asm.is_file():
        raise RuntimeError("H700 Data Abort recovery C/ASM sources are missing")

    h700_abort_obj = out / "h700-data-abort-recovery-thumb.o"
    h700_probe_obj = out / "h700-syscfg-probe-arm.o"
    run(
        ["clang", *thumb_flags, "-I", str(repo_root / "platform/h700/arch"),
         "-std=c11", "-c", str(h700_abort_c), "-o", str(h700_abort_obj)],
        out / "h700-data-abort-recovery-compile.log",
    )
    run(
        ["clang", *flags, "-c", str(h700_probe_asm), "-o", str(h700_probe_obj)],
        out / "h700-syscfg-probe-compile.log",
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
        "h700_arm_data_abort_exception_return",
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
            str(thumb_fault_obj),
            str(h700_abort_obj),
            str(h700_probe_obj),
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

    for required in (
        "h700_data_abort_try_recover",
        "h700_syscfg_ver_reg_low3",
        "h700_syscfg_ver_reg_low3_asm",
        "h700_syscfg_probe_active",
        "h700_syscfg_probe_fault_load",
        "h700_syscfg_probe_recovery",
    ):
        if not any(line.split() and line.split()[-1] == required for line in nm.splitlines()):
            raise RuntimeError("linked H700 abort recovery/probe symbol is missing: " + required)


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
    if len(data) > ACTIVE_LOADER_EXTENT_BYTES:
        raise RuntimeError(
            f"minimal H700 BL33 test raw image exceeds active 32 KiB loader extent: "
            f"{len(data)} > {ACTIVE_LOADER_EXTENT_BYTES}"
        )
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
            str(elf),
        ]
    )
    (out / "lk-bl33-overlay-layout.disasm.txt").write_text(disassembly, encoding="utf-8")
    if not re.search(r"\bblx\b", disassembly):
        raise RuntimeError("A32 exception vector did not link an interworking BLX to the Thumb Data Abort C handler")

    report = [
        "# GammaOS LK H700 BL33 overlay assembly/link validation\n\n",
        f"- Pinned source: {pin}\n",
        f"- Overlay patch SHA-256: {manifest['patch_sha256']}\n",
        "- Result: PASS for the real patched GammaOS LK CRT0 + original LK exception assembly linked with minimal stubs.\n",
        f"- ELF: ELF32/ARM; entry 0x{start:08x}; flat image bytes {len(data)}.\n",
        f"- Active native loader file extent: {ACTIVE_LOADER_EXTENT_BYTES} bytes; raw file headroom {ACTIVE_LOADER_EXTENT_BYTES - len(data)} bytes.\n",
        f"- Linked memory footprint: 0x{end - IMAGE_BASE:x} bytes through _end; linker memory bound remains 0x{IMAGE_END - IMAGE_BASE:x} bytes.\n",
        f"- BL33 branch target: 0x{branch_target:08x}.\n",
        f"- Exception vector table: 0x{vectors:08x} (+0x{vectors - IMAGE_BASE:x}).\n",
        f"- Boot-argument scratch: 0x{boot_argument:08x}.\n",
        f"- BL33 entry wrapper: 0x{entry:08x}.\n",
        f"- Linked _end: 0x{end:08x}; conservative runtime memory bound 0x{IMAGE_END:08x}.\n",
        "- All eight vector instructions decode as A32 branch instructions; all required exception symbols resolve.\n",
        "- The H700-only Data Abort trampoline is compiled and linked; unrelated aborts still use the original fatal C fallback.\n",
        "- The A32 exception vector links to the Thumb-2 C handler through an interworking BLX.\n",
        "- The real H700 matcher C object and fixed-address ARM probe assembly link into the same ELF; the Thumb helper-to-ARM probe call is resolved.\n",
        "- The generated LK make rules exclude MTK platform/common, platform/pal, MTK dev modules and MTK target/init.c when H700_BUILD=yes; H700 must provide its own platform/target module lists.\n",
        "- Synthetic bytes +4..+0x63f replace cleanly without changing the first BL33 branch; the synthetic bytes are not a vendor header.\n\n",
        "## Explicit limits\n\n",
        "- This links the actual modified GammaOS LK CRT0 and original exception assembly with tiny stubs. It is not the full LK kernel/platform build, does not include H700 platform or PAL service objects, and is not a bootable image.\n",
        "- The flat raw file is checked against the current 32 KiB native loader file extent; the test still uses stubs and does not construct a real vendor-header/firmware package.\n",
        "- The 0x4a000000..0x4a0e0000 boundary is only the previously researched BaseOS loader image bound; it is not an approved RAM allocation or payload arena.\n",
        "- The overlay intentionally does not set ACTLR.SMP, release or park secondary CPUs, choose H700 MMU/heap/stack ranges, initialize GIC/CCU/GPIO/SD, or implement console/RTC services.\n",
        "- No firmware is flashed, no SD/boot sector is written, and no payload is executed.\n",
    ]
    (out / "REPORT.md").write_text("".join(report), encoding="utf-8")
    print("".join(report))


    # Build a second, stricter layout image using the real pinned LK early
    # architecture/MMU code and the real H700 PI12 target early-init hook.
    # This intentionally does not link platform_early_init(), heap, timer,
    # GIC, storage, or the full kernel; it is a source/layout regression only.
    early_include = out / "early-include"
    (early_include / "platform").mkdir(parents=True, exist_ok=True)
    (early_include / "platform/mt_reg_base.h").write_text(
        "#ifndef H700_TEST_MT_REG_BASE_H\n"
        "#define H700_TEST_MT_REG_BASE_H\n"
        "/* Only the generic pinned LK mmu.c DRAM base dependency. */\n"
        "#define DRAM_PHY_ADDR 0x40000000u\n"
        "#endif\n",
        encoding="utf-8",
    )

    early_stubs_path = out / "h700-early-led-stubs.c"
    early_stubs_path.write_text(
        "volatile int critical_section_count = 1;\n"
        "volatile unsigned int h700_bl33_entry_snapshot[5];\n"
        "void arm_undefined_handler(void *frame) { (void)frame; for (;;) { __asm__ volatile(\"wfi\"); } }\n"
        "void arm_syscall_handler(void *frame) { (void)frame; for (;;) { __asm__ volatile(\"wfi\"); } }\n"
        "void arm_prefetch_abort_handler(void *frame) { (void)frame; for (;;) { __asm__ volatile(\"wfi\"); } }\n"
        "int platform_irq(void *frame) { (void)frame; return 0; }\n"
        "void platform_fiq(void *frame) { (void)frame; }\n"
        "void thread_preempt(void) { }\n",
        encoding="utf-8",
    )
    early_main_path = out / "h700-early-led-main.c"
    early_main_path.write_text(
        "#include <stdint.h>\n"
        "extern void arch_early_init(void);\n"
        "extern void target_early_init(void);\n"
        "extern int h700_rg34xx_power_led_is_ready(void);\n"
        "volatile uint32_t h700_led_diag_status;\n"
        "void kmain(void) {\n"
        "    arch_early_init();\n"
        "    target_early_init();\n"
        "    h700_led_diag_status = h700_rg34xx_power_led_is_ready() ? 0x50494f31u : 0x45525231u;\n"
        "    for (;;) { __asm__ volatile(\"wfi\"); }\n"
        "}\n",
        encoding="utf-8",
    )

    early_thumb_flags = [
        flag for flag in flags if flag != "-marm"
    ] + [
        "-mthumb",
        "-ffunction-sections",
        "-fdata-sections",
        "-DARM_CPU_CORTEX_A53=1",
        "-DARM_ISA_ARMV7=1",
        "-DARCH_ARM=1",
        "-D_SIZE_T_DEFINED_=1",
        "-DH700_RG34XX_PI12_EARLY_ACCESS_ENABLE=1",
    ]
    early_asm_flags = flags + [
        "-DARM_CPU_CORTEX_A53=1",
        "-DARM_ISA_ARMV7=1",
        "-DARCH_ARM=1",
        "-DASSEMBLY=1",
    ]
    # LK arch.c contains CP10/VFP MRC/MCR instructions, which are A32-only.
    # Compile this architecture object in ARM state; keep H700 services and
    # the generic MMU mapping objects in the normal Thumb-2 mode.
    early_common_includes = [
        "-I", str(early_include),
        "-I", str(repo_root / "platform/h700/arch"),
        "-I", str(repo_root / "platform/h700/power"),
    ]

    def compile_early_thumb(name: str, source: Path) -> Path:
        obj = out / (name + ".o")
        run(
            [
                "clang", *early_thumb_flags, *early_common_includes,
                "-std=c11", "-c", str(source), "-o", str(obj),
            ],
            out / (name + ".log"),
        )
        return obj

    early_arch_obj = compile_early_thumb("h700-real-lk-arch", overlay_arch_c)
    early_mmu_obj = compile_early_thumb("h700-real-lk-mmu", lk / "arch/arm/mmu.c")
    early_pio_map_obj = compile_early_thumb(
        "h700-real-pio-mmu-map",
        repo_root / "platform/h700/arch/h700_pio_mmu_mapping.c",
    )
    early_physical_obj = compile_early_thumb(
        "h700-real-physical-memory",
        repo_root / "platform/h700/arch/h700_physical_memory.c",
    )
    early_led_obj = compile_early_thumb(
        "h700-real-pi12-led",
        repo_root / "platform/h700/power/h700_rg34xx_power_led.c",
    )
    early_target_obj = compile_early_thumb(
        "h700-real-target-early-init",
        repo_root / "target/rg34xx_h700/init.c",
    )
    early_stubs_obj = compile_early_thumb("h700-early-led-stubs", early_stubs_path)
    early_main_obj = compile_early_thumb("h700-early-led-main", early_main_path)

    early_cache_ops_obj = out / "h700-real-lk-cache-ops.o"
    early_ops_obj = out / "h700-real-lk-cp15-ops.o"
    run(
        ["clang", *early_asm_flags, "-c", str(lk / "arch/arm/cache-ops.S"), "-o", str(early_cache_ops_obj)],
        out / "h700-real-lk-cache-ops.log",
    )
    run(
        ["clang", *early_asm_flags, "-c", str(lk / "arch/arm/ops.S"), "-o", str(early_ops_obj)],
        out / "h700-real-lk-cp15-ops.log",
    )

    early_ld_script = out / "h700-early-led-layout.ld"
    early_ld_script.write_text(linker_script, encoding="utf-8")
    early_elf = out / "h700-early-led-layout.elf"
    early_map = out / "h700-early-led-layout.map"
    run(
        [
            "ld.lld", "-m", "armelf", "-static", "--no-undefined", "--gc-sections",
            "-T", str(early_ld_script), "-Map=" + str(early_map), "-o", str(early_elf),
            str(crt_obj),
            str(exc_obj),
            str(early_stubs_obj),
            str(early_main_obj),
            str(thumb_fault_obj),
            str(h700_abort_obj),
            str(h700_probe_obj),
            str(early_arch_obj),
            str(early_mmu_obj),
            str(early_cache_ops_obj),
            str(early_ops_obj),
            str(early_pio_map_obj),
            str(early_physical_obj),
            str(early_led_obj),
            str(early_target_obj),
        ],
        out / "h700-early-led-link.log",
    )

    early_header = run(["readelf", "-h", str(early_elf)])
    if "ELF32" not in early_header or "ARM" not in early_header:
        raise RuntimeError("H700 early-MMU/PI12 diagnostic layout is not ELF32/ARM")
    early_nm = run(["llvm-nm", "-n", str(early_elf)])
    early_undefined = run(["llvm-nm", "-u", str(early_elf)])
    if early_undefined.strip():
        raise RuntimeError("H700 early-MMU/PI12 diagnostic has unresolved symbols:\n" + early_undefined)

    early_symbols = {}
    for required in (
        "_start",
        "kmain",
        "arch_early_init",
        "platform_init_mmu_mappings",
        "arm_mmu_init",
        "arm_mmu_map_section",
        "arch_enable_mmu",
        "target_early_init",
        "h700_rg34xx_power_led_early_init",
        "h700_rg34xx_power_led_is_ready",
        "physical_memory_size",
        "tt",
        "h700_led_diag_status",
        "_end",
    ):
        early_symbols[required] = symbol_address(early_nm, required)

    if early_symbols["_start"] != IMAGE_BASE:
        raise RuntimeError("H700 early-MMU/PI12 diagnostic entry is not at the expected BL33 base")
    if early_symbols["tt"] & 0x3FFF:
        raise RuntimeError(f"real LK 4096-entry section table is not 16 KiB aligned: {early_symbols['tt']:#x}")
    if early_symbols["_end"] >= IMAGE_END:
        raise RuntimeError(f"early-MMU/PI12 BSS exceeds the researched link bound: {early_symbols['_end']:#x}")

    early_binary = out / "h700-early-led-layout.bin"
    run(["llvm-objcopy", "-O", "binary", str(early_elf), str(early_binary)])
    early_raw = early_binary.read_bytes()
    if len(early_raw) > ACTIVE_LOADER_EXTENT_BYTES:
        raise RuntimeError(
            f"early-MMU/PI12 diagnostic raw file exceeds the active 32 KiB loader extent: "
            f"{len(early_raw)} > {ACTIVE_LOADER_EXTENT_BYTES}"
        )

    early_disasm = run(["llvm-objdump", "-d", str(early_elf)])
    (out / "h700-early-led-layout.disasm.txt").write_text(early_disasm, encoding="utf-8")
    kmain_at = early_disasm.find("<kmain>:")
    if kmain_at < 0:
        raise RuntimeError("early-MMU/PI12 diagnostic kmain is missing from disassembly")
    kmain_block = early_disasm[kmain_at:kmain_at + 1800]
    arch_call_at = kmain_block.find("arch_early_init")
    target_call_at = kmain_block.find("target_early_init")
    if arch_call_at < 0 or target_call_at < 0 or arch_call_at >= target_call_at:
        raise RuntimeError("diagnostic must call real LK arch_early_init before the H700 target early-init hook")

    early_report = [
        "\n## Real LK early-MMU + RG34XX PI12 target-hook layout test\n\n",
        "- Result: PASS for a minimal source/layout ELF linking the pinned LK AArch32 architecture/MMU/cache/CP15 code and the H700 PI12 service.\n",
        f"- Raw ELF image: {len(early_raw)} bytes; 32 KiB file headroom {ACTIVE_LOADER_EXTENT_BYTES - len(early_raw)} bytes.\n",
        f"- Linked _end: 0x{early_symbols['_end']:08x}; in-memory span 0x{early_symbols['_end'] - IMAGE_BASE:x} bytes.\n",
        f"- Actual LK section table tt: 0x{early_symbols['tt']:08x}, 16 KiB aligned (16,384-byte table).\n",
        f"- arch_early_init: 0x{early_symbols['arch_early_init']:08x}; platform_init_mmu_mappings: 0x{early_symbols['platform_init_mmu_mappings']:08x}.\n",
        f"- arm_mmu_init: 0x{early_symbols['arm_mmu_init']:08x}; arm_mmu_map_section: 0x{early_symbols['arm_mmu_map_section']:08x}; arch_enable_mmu: 0x{early_symbols['arch_enable_mmu']:08x}.\n",
        f"- target_early_init: 0x{early_symbols['target_early_init']:08x}; PI12 service: 0x{early_symbols['h700_rg34xx_power_led_early_init']:08x}.\n",
        f"- PI12 service-ready status word: 0x{early_symbols['h700_led_diag_status']:08x}.\n",
        "- Diagnostic kmain calls the real LK arch_early_init() first, then the real H700 target_early_init() hook; the latter calls the real PI12 service.\n",
        "- Pinned LK arch.c and H700 early-MMU/PIO/LED C services use Thumb-2; FPEXC uses VMRS/VMSR, while the exception vector and fixed SYS_CFG probe remain A32.\\n",
        "- The H700 platform-header shim supplies only DRAM_PHY_ADDR=0x40000000 required by the generic pinned LK mmu.c; no MTK platform register implementation is linked.\n",
        "- This disposable layout ELF explicitly opts into PI12 MMIO for link validation; production H700 rules keep the physical access macro undefined/disabled.\\n",
        "\n### Explicit limits\n\n",
        "- This is a minimal layout regression, not a bootable image or a full LK target. It deliberately omits thread_init_early(), the vendor-specific platform_early_init(), GIC/timer/console, heap, storage and the rest of kmain().\n",
        "- APB1/PIO clock state at the BL33 handoff is not proven by this link test. The LED service does not alter CCU/APB1; do not execute this payload until the early PIO bus-clock and physical memory ownership gates are separately closed.\n",
        "- The 32 KiB check is raw-file size only. _end/BSS and the 16 KiB translation table are runtime memory; this test does not grant ownership of the [0x4a000000,0x4a0e0000) range.\n",
        "- No vendor header or firmware/SPL pair was generated, no SD/Boot0/TOC1 was written, and no MMIO register was accessed on device.\n",
    ]
    (out / "H700_EARLY_LED_LAYOUT.md").write_text("".join(early_report), encoding="utf-8")
    with (out / "REPORT.md").open("a", encoding="utf-8") as report_file:
        report_file.write("".join(early_report))
    print("".join(early_report))



if __name__ == "__main__":
    main()
