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
        if name == "slot-input":
            run(["cargo", "test", "--manifest-path", str(dst / "Cargo.toml"),
                 "--target", "x86_64-unknown-linux-gnu"],
                log=out / "logs" / "slot-input-no-std-host-tests.log")
            report.append("- PASS: upstream gesture integration tests against the no_std candidate on host\n")
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

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--source", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    source, out = args.source.resolve(), args.out.resolve()
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
        report.append("\n## Result\n\nAll configured host-only compile/link gates passed. This only demonstrates compiler support, selected logic compiling with core+alloc, C-callable staticlib linkage, and a linkable allocator shim. It does not prove real LK heap ABI, startup state, memory map, exception setup, or device execution.\n")
    except Exception as exc:
        report.append(f"\n## FAILURE\n\n{type(exc).__name__}: {exc}\n")
        (out / "REPORT.md").write_text("".join(report), encoding="utf-8")
        raise
    (out / "REPORT.md").write_text("".join(report), encoding="utf-8")
    print((out / "REPORT.md").read_text(encoding="utf-8"))

if __name__ == "__main__":
    main()
