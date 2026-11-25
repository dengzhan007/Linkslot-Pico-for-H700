# Linkslot 1.0.36 Rust no_std / LK ABI feasibility

- Date: 2026-11-23
- Scope: host-only cross-compilation and static linking on GitHub Actions; no device execution, SD writes, boot-media changes, or payload execution.
- Repository source baseline: `upstream/linkslot` @ `8590d6ff1b2caa26118051069c975106fbb14bd3` (user-supplied 1.0.36 archive snapshot).
- Test branch commit: `3cc4bd9917ec77c8730cd42e6d41ea7186bd1f40`.
- Rust/Cargo: `1.96.0`.
- GitHub Actions run: [45102606482](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45102606482), conclusion: success.
- Build artifact: `linkslot-lk-rust-feasibility-45102606482` (artifact ID `14785544761`).

## Summary

| Test | AArch32 `armv7a-none-eabi` | AArch64 `aarch64-unknown-none` |
|---|---:|---:|
| `slot-input` isolated `no_std + alloc` candidate | PASS | PASS |
| `slot-model` isolated `no_std + alloc` candidate | PASS | PASS |
| Rust `staticlib` + freestanding C shim + allocator ABI stub | PASS | PASS |
| Final ELF machine type | ELF32 / ARM | ELF64 / AArch64 |
| Undefined external symbols in final ELF | none | none |
| Linker map generated | yes | yes |

Existing project H700 AArch64 compile checks also passed: C11, C++17 and AArch64 assembly objects were emitted and each object was verified as AArch64.

## What was compiled

1. `slot-input`: an isolated copy was adapted with `#![no_std]`, `alloc::Vec` / `alloc::vec!`, and `core::mem::take`. Both targets passed `cargo check`.
2. `slot-model`: an isolated copy retained static machine/core/layout/power domain data. The filesystem-dependent `PathBuf` root resolver and `Path` extension helper were omitted from this first probe. Both targets passed `cargo check`.
3. An independent minimal Rust `staticlib` exported `extern "C" fn linkslot_probe(u32) -> u32`, used `alloc::Vec`, and supplied a minimal `GlobalAlloc` wrapper forwarding to C functions `linkslot_lk_alloc/free`. A freestanding C shim called the Rust export, supplied an aligned test arena, and linked into static ELF files for both target triples.

The two final ELF files were independently checked after downloading the Actions artifact:
- `abi-armv7a-none-eabi.elf`: ELF32, little-endian, ARM; entry point present; `nm -u` empty.
- `abi-aarch64-unknown-none.elf`: ELF64, little-endian, AArch64; entry point present; `nm -u` empty.

Both linker maps are included in the workflow artifact.

## AArch32 EHABI marker

The Rust 1.96 AArch32 `alloc` static library carries `__aeabi_unwind_cpp_pr0` references in `.ARM.exidx`. Inspection with `readelf -Wr` confirmed all references in this probe are `R_ARM_NONE` metadata relocations, not code-call relocations or `PREL31` relocations. For this `panic=abort` / no-unwind ABI probe only, the linker resolves that metadata-only symbol to absolute zero using `--defsym=__aeabi_unwind_cpp_pr0=0`.

This is **not** an EHABI unwinder implementation. Any future use of C++ exceptions, Rust unwinding, or stack unwinding requires a real and verified EHABI runtime; the zero-valued metadata resolution must not be reused for that purpose.

## Important limits

- The two Linkslot crates were tested as temporary isolated copies; the upstream snapshot was not modified.
- These tests do not prove that all of `slot-model` or `slot-input` is integrated with LK, nor that the complete Linkslot workspace builds without `std`.
- The allocator test uses an aligned host-side bump arena. It does not call the real LK `heap_alloc` / `heap_free` APIs and does not verify their alignment, free/coalescing, locking or failure semantics.
- The C shim validates the generic calling/link ABI and final ELF closure. It does not validate the actual LK entry ABI, stack, EL, MMU, exceptions, interrupts, scheduler, or H700 hardware drivers.
- No Linkslot UI, Libretro emulator core, display/audio/input backend, storage layer or firmware image was built.
- No payload was executed and no device storage or boot sectors were written.

## Next technical gates

1. Select AArch32 LK or AArch64 runtime based on the independently audited H700 boot handoff; Rust target compilation alone does not decide this.
2. Replace the generic allocator stub with a compile/link test against the actual pinned LK heap declarations and symbols; check allocator contract and concurrency.
3. Expand C/Rust ABI probes to cover structs, alignment, callbacks, enums, panic-abort and stack use.
4. Preserve upstream behavior/tests while splitting filesystem helpers away from pure `slot-model` logic.
5. Keep device payload execution blocked until the boot handoff and RAM ownership NO-GO findings are closed.

No firmware, bootable payload, or hardware readiness claim follows from this report.
