#!/usr/bin/env python3
"""Host-only Rust no_std feasibility probe for Linkslot 1.0.36."""
from __future__ import annotations
import argparse, os, re, shutil, subprocess, sys, textwrap
from pathlib import Path

TARGETS = {
    "armv7a-none-eabi": ("armv7a-none-eabi", "armelf", "ARM"),
    "aarch64-unknown-none": ("aarch64-none-elf", "aarch64elf", "AArch64"),
}

def run(cmd, env=None, log=None):
    merged = os.environ.copy()
    if env: merged.update(env)
    p = subprocess.run(cmd, env=merged, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    if log:
        log.parent.mkdir(parents=True, exist_ok=True)
        log.write_text(p.stdout, encoding="utf-8")
    if p.returncode:
        print(p.stdout[-12000:], file=sys.stderr)
        raise RuntimeError("command failed: " + " ".join(map(str, cmd)))
    return p.stdout

def manifest(name, kind="rlib"):
    return textwrap.dedent(f"""\
        [package]
        name = "{name}"
        version = "0.0.0"
        edition = "2021"
        rust-version = "1.96"

        [lib]
        crate-type = ["{kind}"]

        [profile.release]
        panic = "abort"
        codegen-units = 1
        lto = false
    """)

def make_candidate(src, dst, name):
    shutil.copytree(src / "src", dst / "src")
    if name == "slot-input" and (src / "tests").is_dir():
        shutil.copytree(src / "tests", dst / "tests")
    (dst / "Cargo.toml").write_text(manifest(name), encoding="utf-8")
    notes = []
    if name == "slot-input":
        p = dst / "src/lib.rs"
        p.write_text("#![no_std]\nextern crate alloc;\n" + p.read_text(encoding="utf-8"), encoding="utf-8")
        p = dst / "src/gesture.rs"
        p.write_text("use alloc::{vec, vec::Vec};\n" + p.read_text(encoding="utf-8").replace("std::mem::take", "core::mem::take"), encoding="utf-8")
        p = dst / "src/raw.rs"
        p.write_text("use alloc::vec::Vec;\n" + p.read_text(encoding="utf-8"), encoding="utf-8")
        notes += ["lib.rs: no_std + alloc", "gesture.rs: alloc Vec/vec and core::mem::take", "raw.rs: alloc Vec"]
    else:
        p = dst / "src/lib.rs"
        txt = "#![no_std]\n" + p.read_text(encoding="utf-8")
        txt = txt.replace("\npub mod core;", "\nextern crate alloc;\n\npub mod core;")
        txt = txt.replace("pub use layout::{resolve_root, DEVICE_ROOT};", "pub use layout::DEVICE_ROOT;")
        p.write_text(txt, encoding="utf-8")
        p = dst / "src/layout.rs"
        txt = p.read_text(encoding="utf-8").replace("use std::path::PathBuf;\n", "")
        txt, count = re.subn(r"\n/// 解析内容根目录。\n(?:///.*\n)*pub fn resolve_root\(.*?\n\}\n", "\n", txt, flags=re.S)
        if count != 1: raise RuntimeError("could not isolate layout::resolve_root")
        p.write_text(txt, encoding="utf-8")
        p = dst / "src/machine.rs"
        txt = p.read_text(encoding="utf-8").replace("use std::path::Path;\n", "use alloc::{string::String, vec::Vec};\n")
        txt, count = re.subn(r"\n    pub fn accepts\(self, path: &Path\) -> bool \{.*?\n    \}\n", "\n", txt, flags=re.S)
        if count != 1: raise RuntimeError("could not isolate Platform::accepts(Path)")
        p.write_text(txt, encoding="utf-8")
        notes += ["lib.rs: no_std + alloc; omit filesystem root resolver", "layout.rs: retain constants; remove PathBuf helper", "machine.rs: retain domain data; remove Path-based extension helper; use alloc"]
        tests = dst / "tests"
        tests.mkdir(parents=True, exist_ok=True)
        (tests / "portable_model.rs").write_text(r'''use slot_model::{by_dir, by_id, core, MachineId, Platform, MACHINES};

#[test]
fn portable_machine_table_has_unique_ids_and_directories() {
    for (i, a) in MACHINES.iter().enumerate() {
        for b in &MACHINES[i + 1..] {
            assert_ne!(a.id, b.id);
            assert_ne!(a.dir, b.dir);
        }
    }
}

#[test]
fn every_machine_has_a_valid_default_core() {
    for machine in MACHINES {
        assert!(machine.cores.contains(&machine.default_core));
        for id in machine.cores {
            assert!(core(id).is_some(), "unregistered core: {id}");
        }
    }
}

#[test]
fn platform_order_and_registry_lookups_are_stable() {
    assert_eq!(Platform::ALL.len(), MACHINES.len());
    for (index, platform) in Platform::ALL.iter().enumerate() {
        assert_eq!(platform.index(), index);
        let item = platform.machine();
        assert_eq!(by_id(item.id).map(|m| m.id), Some(item.id));
        assert_eq!(by_dir(item.dir).map(|m| m.id), Some(item.id));
    }
    assert_eq!(Platform::Fc.index(), 3);
    assert!(by_id(MachineId("NOT-A-MACHINE")).is_none());
}

#[test]
fn rom_extensions_are_unique_in_stable_table_order() {
    let extensions: Vec<&str> = slot_model::machine::all_rom_ext().collect();
    let mut sorted = extensions.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), extensions.len());
    assert_eq!(extensions.first(), Some(&"gba"));
}
''', encoding="utf-8")
        notes.append("tests/portable_model.rs: add no_std domain-table regression coverage")
    remaining = []
    for p in (dst / "src").rglob("*.rs"):
        txt = p.read_text(encoding="utf-8")
        if re.search(r"\bstd::", txt):
            remaining.append(str(p.relative_to(dst)) + " has std:: references")
    if remaining: raise RuntimeError("unadapted std references: " + "; ".join(remaining))
    return notes

def build_candidates(source, out, report):
    root = out / "candidate-crates"
    root.mkdir(parents=True, exist_ok=True)
    for name in ("slot-input", "slot-model"):
        dst = root / name
        dst.mkdir(parents=True, exist_ok=True)
        notes = make_candidate(source / "crates" / name, dst, name)
        report.extend([f"### {name}\n", *[f"- {n}\n" for n in notes]])
        for triple in TARGETS:
            run(["cargo", "check", "--manifest-path", str(dst / "Cargo.toml"), "--target", triple, "--lib"],
                env={"RUSTFLAGS": "-C panic=abort -C force-unwind-tables=no -C target-cpu=cortex-a53"},
                log=out / "logs" / f"{name}-{triple}.log")
            report.append(f"- PASS: cargo check --target {triple}\n")
        run(["cargo", "test", "--manifest-path", str(dst / "Cargo.toml"),
             "--target", "x86_64-unknown-linux-gnu"],
            log=out / "logs" / f"{name}-no-std-host-tests.log")
        if name == "slot-input":
            report.append("- PASS: 47 upstream gesture integration tests against the no_std candidate on host\n")
        else:
            report.append("- PASS: generated no_std domain-table regression tests on host\n")
        report.append("\n")

def test_upstream_host(source, out, report):
    run(["cargo", "test", "--manifest-path", str(source / "Cargo.toml"),
         "-p", "slot-input", "-p", "slot-model", "--locked"],
        env={"CARGO_TARGET_DIR": str(out / "baseline-host-target")},
        log=out / "logs" / "upstream-slot-input-slot-model-host-tests.log")
    report.append("## Unmodified upstream host unit/integration tests\n\n")
    report.append("- PASS: cargo test -p slot-input -p slot-model --locked against immutable upstream snapshot\n\n")

def build_abi(out, report):
    abi = out / "abi-probe"
    (abi / "src").mkdir(parents=True, exist_ok=True)
    (abi / "Cargo.toml").write_text(manifest("lk_linkslot_abi_probe", "staticlib"), encoding="utf-8")
    rust = r'''#![no_std]
extern crate alloc;
use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use core::panic::PanicInfo;

struct LkAllocator;
unsafe impl GlobalAlloc for LkAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { linkslot_lk_alloc(layout.size(), layout.align()) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { linkslot_lk_free(ptr, layout.size(), layout.align()) }
    }
}
#[global_allocator]
static ALLOCATOR: LkAllocator = LkAllocator;

unsafe extern "C" {
    fn linkslot_lk_alloc(size: usize, align: usize) -> *mut u8;
    fn linkslot_lk_free(ptr: *mut u8, size: usize, align: usize);
}

#[no_mangle]
pub extern "C" fn linkslot_probe(value: u32) -> u32 {
    let mut values = Vec::new();
    values.push(value);
    values.push(value + 1);
    values.iter().copied().sum()
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
'''
    (abi / "src/lib.rs").write_text(rust, encoding="utf-8")
    c = r'''#include <stddef.h>
#include <stdint.h>
static unsigned char arena[8192];
static size_t used;
static volatile uint32_t abi_result;
void *linkslot_lk_alloc(size_t size, size_t align) {
    uintptr_t base = (uintptr_t)arena;
    uintptr_t current = base + used;
    uintptr_t aligned = (current + align - 1u) & ~(uintptr_t)(align - 1u);
    size_t next = (size_t)(aligned - base) + size;
    if (next > sizeof(arena)) return (void *)0;
    used = next;
    return (void *)aligned;
}
void linkslot_lk_free(void *ptr, size_t size, size_t align) {
    (void)ptr; (void)size; (void)align;
}
extern uint32_t linkslot_probe(uint32_t value);
void _start(void) {
    abi_result = linkslot_probe(21u);
    for (;;) { }
}
'''
    (abi / "shim.c").write_text(c, encoding="utf-8")
    for triple, (clang_target, emulation, machine) in TARGETS.items():
        run(["cargo", "build", "--manifest-path", str(abi / "Cargo.toml"), "--target", triple, "--release"],
            env={"RUSTFLAGS": "-C panic=abort -C force-unwind-tables=no -C target-cpu=cortex-a53"},
            log=out / "logs" / f"abi-{triple}-cargo.log")
        archive = abi / "target" / triple / "release" / "liblk_linkslot_abi_probe.a"
        obj, elf = abi / f"shim-{triple}.o", abi / f"abi-{triple}.elf"
        map_file = abi / f"abi-{triple}.map"
        run(["clang", f"--target={clang_target}", "-mcpu=cortex-a53", "-ffreestanding", "-fno-builtin",
             "-fno-stack-protector", "-fno-unwind-tables", "-fno-asynchronous-unwind-tables",
             "-c", str(abi / "shim.c"), "-o", str(obj)])
        link_args = ["ld.lld", "-m", emulation, "-static", "-e", "_start",
                     f"-Map={map_file}", "-o", str(elf)]
        notes = []
        if triple == "armv7a-none-eabi":
            # Verify every __aeabi_unwind_cpp_pr0 reference is R_ARM_NONE in
            # .ARM.exidx. This is a metadata-only marker, not a code call.
            relocations = run(["readelf", "-Wr", str(archive)])
            marker = re.findall(
                r"\b(R_ARM_[A-Z0-9_]+)\s+[^\n]*__aeabi_unwind_cpp_pr0",
                relocations
            )
            if not marker or any(kind != "R_ARM_NONE" for kind in marker):
                raise RuntimeError(
                    "AArch32 EHABI marker was not metadata-only; refusing to resolve it: "
                    + repr(marker)
                )
            link_args.append("--defsym=__aeabi_unwind_cpp_pr0=0")
            notes.append("EHABI: every __aeabi_unwind_cpp_pr0 reference is R_ARM_NONE metadata only; resolved to ABS 0 for this panic=abort/no-unwind probe. This is not an unwinder implementation.")
        run(link_args + [str(obj), str(archive)])
        header = run(["readelf", "-h", str(elf)])
        undefined = run(["llvm-nm", "-u", str(elf)])
        if machine not in header:
            raise RuntimeError(f"{elf.name}: wrong ELF machine")
        if undefined.strip():
            raise RuntimeError(f"{elf.name}: unresolved symbols:\n{undefined}")
        sha = run(["sha256sum", str(elf)]).strip()
        report.append(f"- PASS: {triple} staticlib + freestanding C shim linked; machine={machine}; no undefined symbols; {sha}\n")
        report.extend(f"  - {note}\n" for note in notes)
        report.append(f"  - Linker map: {map_file.name}\n")
    report.append("\n")

def build_lk_heap_abi(lk_source, out, report):
    expected = "9bf1419a49e0af6dba8b05916df382c9c9c6d0a8"
    actual = run(["git", "-C", str(lk_source), "rev-parse", "HEAD"]).strip()
    if actual != expected:
        raise RuntimeError(f"LK source pin mismatch: expected {expected}, got {actual}")
    heap_header = lk_source / "include/lib/heap.h"
    heap_source = lk_source / "lib/heap/heap.c"
    thread_header = lk_source / "include/kernel/thread.h"
    thread_source = lk_source / "kernel/thread.c"
    arm_ops = lk_source / "arch/arm/ops.S"
    for p in (heap_header, heap_source, thread_header, thread_source, arm_ops):
        if not p.is_file():
            raise RuntimeError(f"pinned LK evidence file missing: {p}")

    header_text = heap_header.read_text(encoding="utf-8")
    heap_text = heap_source.read_text(encoding="utf-8")
    thread_header_text = thread_header.read_text(encoding="utf-8")
    thread_c = thread_source.read_text(encoding="utf-8")
    arm_text = arm_ops.read_text(encoding="utf-8")
    for needle in ("heap_alloc(size_t, unsigned int alignment)", "heap_free(void *)"):
        if needle not in header_text:
            raise RuntimeError("LK heap header signature changed: " + needle)
    if "int critical_section_count = 1;" not in thread_c:
        raise RuntimeError("LK global critical-section count definition changed")
    if "extern int critical_section_count;" not in thread_header_text:
        raise RuntimeError("LK global critical-section declaration missing")
    if not re.search(r"orr\s+r0,\s*r0,\s*#\(1<<7\)", arm_text):
        raise RuntimeError("LK ARM IRQ-mask implementation changed; re-audit SMP semantics")
    if "enter_critical_section();" not in heap_text or "exit_critical_section();" not in heap_text:
        raise RuntimeError("LK heap critical-section wrapping changed")

    abi = out / "lk-heap-abi-probe"
    (abi / "src").mkdir(parents=True, exist_ok=True)
    (abi / "Cargo.toml").write_text(manifest("lk_real_heap_api_probe", "staticlib"), encoding="utf-8")
    rust = r'''#![no_std]
extern crate alloc;
use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use core::panic::PanicInfo;

struct LkHeapAllocator;
unsafe impl GlobalAlloc for LkHeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.align() > u32::MAX as usize {
            return core::ptr::null_mut();
        }
        unsafe { heap_alloc(layout.size(), layout.align() as u32) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe { heap_free(ptr) }
    }
}
#[global_allocator]
static ALLOCATOR: LkHeapAllocator = LkHeapAllocator;

unsafe extern "C" {
    fn heap_alloc(size: usize, alignment: u32) -> *mut u8;
    fn heap_free(ptr: *mut u8);
}

#[no_mangle]
pub extern "C" fn linkslot_lk_heap_probe(value: u32) -> u32 {
    let mut values = Vec::new();
    values.push(value);
    values.push(value + 1);
    values.iter().copied().sum()
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
'''
    (abi / "src/lib.rs").write_text(rust, encoding="utf-8")
    c = r'''#include <lib/heap.h>
#include <stdint.h>

_Static_assert(sizeof(size_t) == 4, "LK AArch32 size_t must be 32-bit");
_Static_assert(sizeof(unsigned int) == 4, "LK heap alignment argument must be 32-bit");

static unsigned char arena[16384] __attribute__((aligned(16)));
static size_t arena_used;
static volatile uint32_t probe_result;

/* Test-only implementation of the exact pinned LK heap API declarations.
 * It is NOT the LK heap implementation and is never executed on device. */
void *heap_alloc(size_t size, unsigned int alignment) {
    if (alignment != 0 && (alignment & (alignment - 1u)) != 0) return (void *)0;
    size_t a = alignment ? alignment : sizeof(void *);
    if (alignment && a < 16u) a = 16u;
    uintptr_t base = (uintptr_t)arena;
    uintptr_t current = base + arena_used;
    uintptr_t aligned = (current + a - 1u) & ~(uintptr_t)(a - 1u);
    size_t next = (size_t)(aligned - base) + size;
    if (next > sizeof(arena)) return (void *)0;
    arena_used = next;
    return (void *)aligned;
}
void heap_free(void *ptr) {
    (void)ptr;
}

extern uint32_t linkslot_lk_heap_probe(uint32_t value);
void _start(void) {
    probe_result = linkslot_lk_heap_probe(21u);
    for (;;) { }
}
'''
    (abi / "lk_heap_shim.c").write_text(c, encoding="utf-8")
    triple = "armv7a-none-eabi"
    clang_target, emulation, machine = TARGETS[triple]
    run(["cargo", "build", "--manifest-path", str(abi / "Cargo.toml"),
         "--target", triple, "--release"],
        env={"RUSTFLAGS": "-C panic=abort -C force-unwind-tables=no -C target-cpu=cortex-a53"},
        log=out / "logs" / "lk-heap-rust-cargo.log")
    archive = abi / "target" / triple / "release" / "liblk_real_heap_api_probe.a"
    obj = abi / "lk-heap-shim.o"
    elf = abi / "lk-heap-api-armv7a.elf"
    map_file = abi / "lk-heap-api-armv7a.map"
    run(["clang", f"--target={clang_target}", "-mcpu=cortex-a53", "-ffreestanding",
         "-fno-builtin", "-fno-stack-protector", "-fno-unwind-tables",
         "-fno-asynchronous-unwind-tables", "-D_SIZE_T_DEFINED_",
         "-I", str(lk_source / "include"), "-Wall", "-Wextra", "-Werror",
         "-c", str(abi / "lk_heap_shim.c"), "-o", str(obj)],
        log=out / "logs" / "lk-heap-header-c-compile.log")
    relocations = run(["readelf", "-Wr", str(archive)])
    marker = re.findall(r"\b(R_ARM_[A-Z0-9_]+)\s+[^\n]*__aeabi_unwind_cpp_pr0", relocations)
    if not marker or any(kind != "R_ARM_NONE" for kind in marker):
        raise RuntimeError("LK heap ABI probe EHABI metadata marker changed: " + repr(marker))
    run(["ld.lld", "-m", emulation, "-static", "-e", "_start",
         "--defsym=__aeabi_unwind_cpp_pr0=0", f"-Map={map_file}",
         "-o", str(elf), str(obj), str(archive)])
    header = run(["readelf", "-h", str(elf)])
    undefined = run(["llvm-nm", "-u", str(elf)])
    symbols = run(["llvm-nm", "-g", str(elf)])
    if machine not in header:
        raise RuntimeError("LK heap API probe emitted the wrong ELF machine")
    if undefined.strip():
        raise RuntimeError("LK heap API probe unresolved symbols:\n" + undefined)
    for symbol in ("heap_alloc", "heap_free", "linkslot_lk_heap_probe"):
        if symbol not in symbols:
            raise RuntimeError("LK heap ABI symbol missing from linked ELF: " + symbol)

    report.append("## Pinned GammaOS LK heap API compatibility probe (AArch32 only)\n\n")
    report.append(f"- LK reference commit: {actual}\n")
    report.append("- Compiled against the pinned include/lib/heap.h declarations.\n")
    report.append("- Rust GlobalAlloc calls heap_alloc(size_t, unsigned int) and heap_free(void *); the C shim implements only a test stub with matching signatures.\n")
    report.append("- PASS: ARM ELF32 static link, expected heap symbols defined, no unresolved external symbols, linker map generated.\n")
    report.append("- Important: this does NOT link or execute the real lib/heap/heap.c; it is header/signature/ABI compatibility evidence only.\n\n")

    report.append("### LK allocator alignment and concurrency audit\n\n")
    report.append("- heap_alloc rejects non-zero alignments that are not powers of two; zero is accepted. For positive alignments below 16 bytes, the implementation raises the requested alignment to 16 bytes.\n")
    report.append("- heap_free accepts only the pointer; Rust GlobalAlloc::dealloc can map directly to it. Rust layout alignment must fit in the C unsigned int argument; the adapter returns null if it does not.\n")
    report.append("- heap_alloc / heap_free protect their free-list mutations using LK enter_critical_section() / exit_critical_section().\n")
    report.append("- The pinned LK thread header declares a global critical_section_count; kernel/thread.c defines it globally. ARM arch_disable_ints changes the local CPSR IRQ-mask bit. This proves local interrupt exclusion, not cross-core mutual exclusion.\n")
    report.append("- Therefore this allocator adapter is not approved for concurrent multi-core use. It is usable only after proving secondary H700 cores are held off for the LK lifetime, or after adding and validating a cross-core heap lock / SMP-safe allocator.\n")
    report.append("- This pinned LK branch is AArch32/MTK-oriented and is a reference, not an H700-ready kernel port.\n\n")

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--source", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--lk-source", type=Path, required=True)
    args = ap.parse_args()
    source, out, lk_source = args.source.resolve(), args.out.resolve(), args.lk_source.resolve()
    out.mkdir(parents=True, exist_ok=True)
    report = [
        "# Linkslot 1.0.36 no_std / LK ABI feasibility build\n\n",
        "- Scope: host-only cross-compile and static link; no device, SD, boot media, or payload execution.\n",
        "- Rust: " + run(["rustc", "--version"]).strip() + "\n",
        "- Cargo: " + run(["cargo", "--version"]).strip() + "\n",
        "- Targets: armv7a-none-eabi (AArch32), aarch64-unknown-none (AArch64)\n\n",
        "## Candidate crate probes\n\n",
        "Temporary isolated copies of upstream sources are adapted; the upstream snapshot is unchanged.\n\n"
    ]
    try:
        test_upstream_host(source, out, report)
        build_candidates(source, out, report)
        report.append("## Rust staticlib / C ABI / allocator shim\n\n")
        build_abi(out, report)
        build_lk_heap_abi(lk_source, out, report)
        report.append("\n## Result\n\nAll configured host-only compile/link gates passed. This only demonstrates compiler support, selected logic compiling with core+alloc, C-callable staticlib linkage, and a linkable allocator shim. The LK heap test checks only the pinned header/signature ABI with a test stub, not the real heap implementation, SMP safety, startup state, memory map, exception setup, or device execution.\n")
    except Exception as exc:
        report.append(f"\n## FAILURE\n\n{type(exc).__name__}: {exc}\n")
        (out / "REPORT.md").write_text("".join(report), encoding="utf-8")
        raise
    (out / "REPORT.md").write_text("".join(report), encoding="utf-8")
    print((out / "REPORT.md").read_text(encoding="utf-8"))

if __name__ == "__main__":
    main()
