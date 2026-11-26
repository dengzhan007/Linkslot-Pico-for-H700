#!/usr/bin/env python3
"""Host-only AArch32 LK-as-BL33 image-layout feasibility probe."""
from __future__ import annotations
import argparse
import hashlib
import re
import shutil
import struct
import subprocess
import sys
import textwrap
from pathlib import Path

LK_PIN = "9bf1419a49e0af6dba8b05916df382c9c9c6d0a8"
IMAGE_BASE = 0x4A000000
HEADER_END = 0x640
VECTOR_BASE = 0x1000
IMAGE_LIMIT = 0x4A0E0000  # BaseOS A32-loader BSS upper bound, not a general RAM grant.

def run(cmd, log=None):
    p = subprocess.run(cmd, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    if log:
        log.parent.mkdir(parents=True, exist_ok=True)
        log.write_text(p.stdout, encoding="utf-8")
    if p.returncode:
        print(p.stdout[-10000:], file=sys.stderr)
        raise RuntimeError("command failed: " + " ".join(map(str, cmd)))
    return p.stdout

def symbol_address(nm_text, name):
    for line in nm_text.splitlines():
        cols = line.split()
        if len(cols) >= 3 and cols[-1] == name:
            return int(cols[0], 16)
    raise RuntimeError("missing linked symbol: " + name)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--lk-source", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    lk, out = args.lk_source.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    for tool in ("clang", "ld.lld", "llvm-nm", "llvm-objcopy", "llvm-objdump", "readelf"):
        if not shutil.which(tool):
            raise RuntimeError("required tool missing: " + tool)

    actual = run(["git", "-C", str(lk), "rev-parse", "HEAD"]).strip()
    if actual != LK_PIN:
        raise RuntimeError(f"LK pin mismatch: expected {LK_PIN}, got {actual}")

    crt0_path = lk / "arch/arm/crt0.S"
    vectors_path = lk / "arch/arm/exceptions.S"
    arch_c_path = lk / "arch/arm/arch.c"
    rules_path = lk / "arch/arm/rules.mk"
    thread_path = lk / "kernel/thread.c"
    thread_header_path = lk / "include/kernel/thread.h"
    heap_path = lk / "lib/heap/heap.c"
    ops_path = lk / "arch/arm/ops.S"
    for p in (crt0_path, vectors_path, arch_c_path, rules_path, thread_path, thread_header_path, heap_path, ops_path):
        if not p.is_file():
            raise RuntimeError("pinned LK evidence file missing: " + str(p))

    original_crt0 = crt0_path.read_text(encoding="utf-8")
    begin = ".globl _start\n_start:"
    marker = "reset:\n"
    i = original_crt0.find(begin)
    j = original_crt0.find(marker, i)
    if i < 0 or j < 0:
        raise RuntimeError("unexpected LK crt0 startup shape; expected _start followed by reset; found: " + repr(original_crt0[original_crt0.find("_start")-16:original_crt0.find("_start")+48]))

    # The first word must remain executable. BL31's vendor loader reads an
    # opaque header from +4 through +0x63f. Keep the rest of the first page
    # reserved and put the LK exception vectors at +0x1000.
    prefix = """.global _start
_start:
    b lk_bl33_entry
    .space 0xffc, 0

    .balign 32
.global __lk_vectors_start
__lk_vectors_start:
    b reset
    b arm_undefined
    b arm_syscall
    b arm_prefetch_abort
    b arm_data_abort
    b arm_reserved
    b arm_irq
    b arm_fiq

.global BOOT_ARGUMENT_LOCATION
BOOT_ARGUMENT_LOCATION:
    .word 0

.global lk_bl33_entry
lk_bl33_entry:
    mrs r11, cpsr
    mov r8, sp
    mov r9, lr
    mov r10, r4
    ldr r0, =__lk_vectors_start
    mcr p15, 0, r0, c12, c0, 0
    isb
"""
    patched_crt0 = original_crt0[:i] + prefix + original_crt0[j:]
    relocation = "sub\t\tr0, r0, #(.Laddr - _start)"
    if patched_crt0.count(relocation) != 1:
        raise RuntimeError("LK crt0 self-relocation instruction changed; re-audit before adapting")
    # Moving reset behind the +0x1000 vectors makes the offset too large for
    # one ARM data-processing immediate. Load it via the literal pool instead.
    patched_crt0 = patched_crt0.replace(
        relocation,
        "ldr\t\tr1, =(.Laddr - _start)\n\tsub\t\tr0, r0, r1"
    )
    kmain_call = "\tbl\t\tkmain"
    if patched_crt0.count(kmain_call) != 1:
        raise RuntimeError("unexpected pinned crt0.S kmain call; refusing unsafe entry-state publication")
    publish = (
        "    ldr r0, =h700_bl33_entry_snapshot\n"
        "    stmia r0, {r8-r11}\n"
        "    ldr r1, =0x48374553\n"
        "    str r1, [r0, #16]\n"
    )
    patched_crt0 = patched_crt0.replace(kmain_call, publish + kmain_call, 1)
    (out / "crt0-bl33.S").write_text(patched_crt0, encoding="utf-8")

    c_stubs = r'''volatile int critical_section_count = 1;
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
    (out / "lk-bl33-stubs.c").write_text(c_stubs, encoding="utf-8")

    linker = textwrap.dedent(f"""\
        OUTPUT_FORMAT("elf32-littlearm", "elf32-littlearm", "elf32-littlearm")
        OUTPUT_ARCH(arm)
        ENTRY(_start)
        SECTIONS
        {{
          . = 0x{IMAGE_BASE:08x};
          .text.boot : {{ *(.text.boot) }}
          .text : {{ *(.text .text.* .glue_7* .gnu.linkonce.t.*) }}
          .rodata : {{ *(.rodata .rodata.* .gnu.linkonce.r.*) }}
          __data_start_rom = .;
          __data_start = .;
          .data : {{ *(.data .data.* .gnu.linkonce.d.*) }}
          __data_end = .;
          . = ALIGN(4);
          __bss_start = .;
          .bss (NOLOAD) : {{ *(.bss .bss.*) *(COMMON) }}
          . = ALIGN(4);
          __bss_end = .;
          _end = .;
          _end_of_ram = 0x{IMAGE_LIMIT:08x};
          ASSERT(_end <= _end_of_ram, "LK BL33 layout probe exceeds A32 loader image arena")
          /DISCARD/ : {{ *(.ARM.exidx*) *(.ARM.extab*) *(.comment) *(.ARM.attributes) *(.note*) }}
        }}
    """)
    (out / "lk-bl33-layout.ld").write_text(linker, encoding="utf-8")

    flags = [
        "--target=armv7a-none-eabi", "-mcpu=cortex-a53", "-marm",
        "-mfloat-abi=softfp", "-mfpu=neon-vfpv4", "-ffreestanding",
        "-fno-builtin", "-fno-pic", "-fno-pie", "-fno-stack-protector",
        "-fno-unwind-tables", "-fno-asynchronous-unwind-tables",
        "-I", str(lk / "include"), "-I", str(lk / "arch/arm/include"),
        "-DARM_WITH_CP15=1", "-DARM_WITH_MMU=1", "-DARM_ISA_ARMv7=1",
        "-DARM_WITH_VFP=1", "-DARM_WITH_NEON=1", "-DARM_WITH_THUMB=1",
        "-DARM_WITH_THUMB2=1", "-DARM_WITH_CACHE=1", "-DARM_WITH_L2=1",
        "-Wall", "-Wextra", "-Werror"
    ]
    crt_obj = out / "crt0-bl33.o"
    exc_obj = out / "exceptions.o"
    stub_obj = out / "lk-bl33-stubs.o"
    run(["clang", *flags, "-c", str(out / "crt0-bl33.S"), "-o", str(crt_obj)],
        out / "crt0-compile.log")
    run(["clang", *flags, "-c", str(vectors_path), "-o", str(exc_obj)],
        out / "exceptions-compile.log")
    run(["clang", *flags, "-std=c11", "-c", str(out / "lk-bl33-stubs.c"), "-o", str(stub_obj)],
        out / "stubs-compile.log")

    elf = out / "lk-a32-bl33-layout.elf"
    mapfile = out / "lk-a32-bl33-layout.map"
    run(["ld.lld", "-m", "armelf", "-static", "--no-undefined",
         "-T", str(out / "lk-bl33-layout.ld"), "-Map=" + str(mapfile),
         "-o", str(elf), str(crt_obj), str(exc_obj), str(stub_obj)],
        out / "link.log")
    header = run(["readelf", "-h", str(elf)])
    nm = run(["llvm-nm", "-n", str(elf)])
    undefined = run(["llvm-nm", "-u", str(elf)])
    if "ELF32" not in header or "ARM" not in header:
        raise RuntimeError("layout probe is not ELF32/ARM")
    if undefined.strip():
        raise RuntimeError("unresolved symbols remain:\n" + undefined)
    start = symbol_address(nm, "_start")
    vectors = symbol_address(nm, "__lk_vectors_start")
    argument = symbol_address(nm, "BOOT_ARGUMENT_LOCATION")
    entry = symbol_address(nm, "lk_bl33_entry")
    end = symbol_address(nm, "_end")
    snapshot = symbol_address(nm, "h700_bl33_entry_snapshot")
    if start != IMAGE_BASE or vectors != IMAGE_BASE + VECTOR_BASE:
        raise RuntimeError(f"unexpected image/vector placement: start={start:#x}, vectors={vectors:#x}")
    if argument < IMAGE_BASE + HEADER_END or entry < argument + 4:
        raise RuntimeError("LK vector/argument/entry placement overlaps the vendor header window")
    if end >= IMAGE_LIMIT:
        raise RuntimeError(f"LK layout exceeds BaseOS loader arena: end={end:#x}")
    if snapshot < IMAGE_BASE or snapshot + 20 > end:
        raise RuntimeError(f"entry snapshot is outside the linked image/BSS: {snapshot:#x}..{snapshot + 20:#x}")

    binary = out / "lk-a32-bl33-layout.bin"
    run(["llvm-objcopy", "-O", "binary", str(elf), str(binary)])
    data = bytearray(binary.read_bytes())
    if len(data) <= VECTOR_BASE or len(data) > IMAGE_LIMIT - IMAGE_BASE:
        raise RuntimeError("unexpected flat image size")
    first = struct.unpack_from("<I", data, 0)[0]
    if (first & 0x0F000000) != 0x0A000000 or (first & 0x01000000):
        raise RuntimeError(f"first word is not an A32 B instruction: {first:#010x}")
    imm = first & 0x00FFFFFF
    if imm & 0x00800000:
        imm -= 1 << 24
    target = IMAGE_BASE + 8 + (imm << 2)
    if target != entry:
        raise RuntimeError(f"entry branch points to {target:#x}, expected {entry:#x}")

    # Synthetic header only tests the offset/overlay mechanics, not vendor bytes.
    synthetic_header = hashlib.sha256(b"LK-BL33-header-layout-only").digest()
    pattern = (synthetic_header * ((0x63C + len(synthetic_header) - 1) // len(synthetic_header)))[:0x63C]
    before_branch = bytes(data[:4])
    data[4:HEADER_END] = pattern
    if bytes(data[:4]) != before_branch or bytes(data[4:HEADER_END]) != pattern:
        raise RuntimeError("header overlay changed the branch or was not applied exactly")
    overlay = out / "lk-a32-bl33-layout-header-overlay.bin"
    overlay.write_bytes(data)

    disasm = run(["llvm-objdump", "-d", "--start-address=0x4a000000",
                  "--stop-address=0x4a001080", str(elf)])
    (out / "lk-a32-bl33-layout.disasm.txt").write_text(disasm, encoding="utf-8")

    arch_c = arch_c_path.read_text(encoding="utf-8")
    if "set_vector_base(MEMBASE);" not in arch_c:
        raise RuntimeError("LK arch.c vector-base implementation changed; re-audit")
    patched_arch = arch_c.replace(
        "set_vector_base(MEMBASE);",
        "extern char __lk_vectors_start[];\n\tset_vector_base((addr_t)__lk_vectors_start);"
    )
    (out / "arch.c-bl33-vbar-adaptation.c").write_text(patched_arch, encoding="utf-8")

    crt0 = original_crt0
    rules = rules_path.read_text(encoding="utf-8")
    thread = thread_path.read_text(encoding="utf-8")
    thread_header = thread_header_path.read_text(encoding="utf-8")
    heap = heap_path.read_text(encoding="utf-8")
    ops = ops_path.read_text(encoding="utf-8")
    has_a53_mcpu_bug = "ifeq ($(ARM_CPU),cortex-a53)" in rules and "CFLAGS += -mcpu=cortex-a9" in rules
    has_a7_only_smp = "#ifdef ARM_CPU_CORTEX_A7" in crt0 and "ARM_CPU_CORTEX_A53" not in crt0
    global_scheduler = "thread_t *current_thread;" in thread and "static struct list_node run_queue" in thread
    global_critical = "int critical_section_count = 1;" in thread
    local_irq_mask = bool(re.search(r"orr\s+r0,\s*r0,\s*#\(1<<7\)", ops))
    global_critical_transition = ("critical_section_count++;" in thread_header and
                                  "if (critical_section_count == 1)" in thread_header and
                                  "arch_disable_ints();" in thread_header and
                                  "critical_section_count--;" in thread_header and
                                  "arch_enable_ints();" in thread_header)

    report = [
        "# AArch32 LK-as-BL33 host-only layout probe\n\n",
        f"- Pinned LK commit: {actual}\n",
        "- Result: PASS for an A32 BL33 entry-wrapper/layout prototype; this is NOT a full LK build or boot image.\n",
        f"- Link address: 0x{IMAGE_BASE:08x} (researched BaseOS BL33 slot only; not a general payload/RAM grant).\n",
        f"- Entry branch at +0: target 0x{target:08x}.\n",
        f"- Relocated exception vector table: 0x{vectors:08x} (+0x{vectors-IMAGE_BASE:x}).\n",
        f"- Boot argument scratch: 0x{argument:08x}.\n",
        f"- Entry snapshot BSS object: 0x{snapshot:08x} (five words, including publish-last magic).\n",
        "- Entry capture records incoming SP/LR/r4/CPSR before VBAR setup; CRT0 publishes it after BSS clear and before kmain.\n",
        f"- Linked _end: 0x{end:08x}; flat image bytes: {len(data)}.\n",
        "- Synthetic overlay verified: bytes +4..+0x63f can be replaced by opaque header bytes while preserving the first branch and keeping vectors/entry outside the header area. The synthetic header is NOT the vendor header.\n",
        "- ELF32/ARM; no unresolved symbols; linker map and disassembly generated.\n\n",
        "## BL33 prototype adaptations and remaining requirements\n\n",
        "- Upstream crt0.S places vector branches at +4..+0x1f and BOOT_ARGUMENT_LOCATION at +0x20. Both are overwritten by the BaseOS-compatible header overlay at +4..+0x63f. The temporary wrapper moves vectors to +0x1000, preserves the boot-argument word after them, sets VBAR at entry, and then enters the original reset path.\n",
        "- The pinned upstream arch.c resets VBAR to MEMBASE; the generated H700 overlay already replaces that assignment with __lk_vectors_start, and test-h700-lk-overlay.py asserts that the overlay no longer resets VBAR to MEMBASE.\n",
        "- The probe uses a tiny kmain and exception callback stubs. It does not build the full LK kernel, MTK platform, storage, drivers, heap, or scheduler.\n",
        "- Upstream Cortex-A53 rules.mk baseline uses -mcpu=cortex-a9; the generated H700 overlay changes it to -mcpu=cortex-a53 -mfpu=neon-vfpv4 -mfloat-abi=softfp, checked by test-h700-lk-overlay.py.\n",
        f"- A53 SMP-enable audit: {'FOUND: crt0 only sets ACTLR.SMP for ARM_CPU_CORTEX_A7; no A53-specific path' if has_a7_only_smp else 'no A7-only condition found; re-audit'}.\n",
        f"- Scheduler audit: {'global current-thread and run-queue structures are present' if global_scheduler else 'scheduler structures changed; re-audit'}.\n",
        f"- Critical-section audit: {'global nesting counter + local CPSR IRQ mask; one-core only, not a cross-core lock' if global_critical and local_irq_mask and global_critical_transition else 'critical-section implementation changed; re-audit'}.\n",
        "- LK thread.c uses one global current_thread, one set of global run queues and one global critical_section_count; no per-CPU scheduler state was found in this port. This is a uniprocessor design, not an SMP scheduler.\n",
        "- arch_disable_ints only sets CPSR.I on the executing core. The global nesting count is not an atomic cross-core lock. Do not allow secondary cores to enter this runtime without adding an SMP design.\n",
        "- The compiler-target issue above is already corrected in the generated H700 overlay; keep the Cortex-A53 + NEON/VFPv4 + softfp assertion in the overlay regression.\n",
        "- crt0.S enables ACTLR.SMP only when ARM_CPU_CORTEX_A7 is defined; it has no Cortex-A53 branch. The H700 port must explicitly establish A53 coherency/SMP state or deliberately remain single-core under a proven secondary-core hold-off contract.\n",
        "- H700 secondary-core state at BL31->BL33 remains UNKNOWN. Until other Cortex-A53 cores are proven held off or an SMP port is implemented and validated, this LK runtime is not approved for multi-core use.\n",
        "- The linker bound 0x4a0e0000 is copied only from the BaseOS loader BSS limit. It does not certify the full RAM map, grant a heap range, or supersede reserved-memory exclusions.\n",
        "- lk-a32-bl33-layout-header-overlay.bin is a synthetic host-test artifact, not firmware/a bootable payload. Never write it to SD or execute it.\n"
    ]
    (out / "REPORT.md").write_text("".join(report), encoding="utf-8")
    print("".join(report))

if __name__ == "__main__":
    main()
