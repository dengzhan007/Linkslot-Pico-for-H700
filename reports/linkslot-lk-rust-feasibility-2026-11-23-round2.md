# Linkslot 1.0.36 no_std behavior tests and pinned LK heap ABI probe

- Date: 2026-11-23
- Scope: host-only CI cross-compilation, unit/integration tests and static linking.
- Linkslot snapshot: `upstream/linkslot` @ `8590d6ff1b2caa26118051069c975106fbb14bd3`.
- LK reference: `TheGammaSqueeze/lk-mt6785`, branch `lk-gba-emu-sd-card-kpa`, commit `9bf1419a49e0af6dba8b05916df382c9c9c6d0a8`.
- Test branch commit: `8c0b9c62accd5ee904cd9a8fb69733b0eac25484`.
- Rust/Cargo: `1.96.0`.
- GitHub Actions: [run 45103281417](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45103281417), conclusion SUCCESS.
- Artifact: `linkslot-lk-rust-feasibility-45103281417`, artifact ID `14786665430`.

## Test results

| Test | Result |
|---|---:|
| Original Linkslot `slot-input` host tests | 47 passed |
| Original Linkslot `slot-model` layout + registry host tests | 11 passed |
| `slot-input` isolated `no_std + alloc` AArch32 compile | PASS |
| `slot-input` isolated `no_std + alloc` AArch64 compile | PASS |
| Original gesture tests against the isolated `no_std` `slot-input` candidate | 47 passed |
| `slot-model` isolated `no_std + alloc` AArch32 compile | PASS |
| `slot-model` isolated `no_std + alloc` AArch64 compile | PASS |
| Pure domain-table tests against `no_std` `slot-model` candidate | 4 passed |
| Rust `staticlib` + C shim generic ABI probe, AArch32 | PASS |
| Rust `staticlib` + C shim generic ABI probe, AArch64 | PASS |
| Rust `GlobalAlloc` adapter against pinned LK heap header declarations, AArch32 | PASS |

All of the tests above ran on GitHub Actions; no local workstation build was required. The original Linkslot snapshot branch was not modified.

## What the no_std tests establish

### slot-input

A temporary source copy was adapted to `#![no_std]`, `alloc::Vec` / `alloc::vec!`, and `core::mem::take`. Its original 47 gesture integration tests were copied unchanged and passed under the host test runner while the library itself remained `no_std`. The same candidate also passed `cargo check` for AArch32 and AArch64 freestanding targets.

This is strong evidence that the gesture state machine can remain common logic shared by a future LK port. It does not include H700 button sampling, interrupt handling, debounce or key-matrix drivers.

### slot-model

A temporary source copy retained pure static machine/core/layout/power domain data. The first probe deliberately excluded filesystem-dependent `PathBuf` root resolution and `Path` extension detection. The candidate passed the two freestanding target checks and four added regression tests covering unique machine ids/directories, valid default-core mappings, platform table ordering, lookups and ROM extension uniqueness.

The original upstream `slot-model` test suite also passed unchanged on host (3 layout + 8 registry tests). This means the data-only model can be separated from the OS path layer without losing the tested registry semantics; the filesystem helpers still need an explicit platform backend.

## Pinned LK heap header / ABI probe

The CI job checked out the exact LK commit listed above and compiled against its real `include/lib/heap.h` declaration:

```c
void *heap_alloc(size_t size, unsigned int alignment);
void *heap_realloc(void *ptr, size_t size);
void heap_free(void *ptr);
```

A separate AArch32 Rust static library implemented `GlobalAlloc` by calling the declared `heap_alloc(size_t, unsigned int)` and `heap_free(void *)` functions. The C side included the pinned LK header and supplied a test-only stub with matching function signatures. The Rust/C static link produced an ELF32/ARM image; `heap_alloc`, `heap_free` and the Rust probe symbol were defined in the final ELF; the CI `nm -u` check reported no unresolved external symbols. A linker map was generated.

**This is not a link against the real heap implementation.** The C stub is only an ABI/signature test. The run does not build or execute LK `lib/heap/heap.c`, nor does it validate heap initialization, fragmentation, coalescing, OOM, or concurrency at runtime.

### Alignment contract

Source inspection of the pinned LK heap implementation shows:
- zero alignment is accepted;
- non-zero alignment must be a power of two;
- a positive request below 16 bytes is raised to 16;
- freeing needs only the original pointer.

The Rust adapter passes `Layout::align()` as the C `unsigned int` argument and rejects values that cannot fit in that type.

### Multi-core safety: unresolved and currently a blocker

The pinned LK implementation wraps heap free-list changes in `enter_critical_section()` / `exit_critical_section()`. However:
- `critical_section_count` is declared and defined as one global variable;
- AArch32 `arch_disable_ints` only changes the local CPU's CPSR IRQ-mask bit;
- no cross-core heap lock is established by those facts.

This is local interrupt exclusion, not proof of SMP mutual exclusion. The reference allocator must therefore **not** be used as a shared multi-core GlobalAlloc on H700 unless one of two conditions is established:
1. all secondary Cortex-A53 cores are reliably held off for the entire LK runtime; or
2. the allocator is protected by a real cross-core lock / SMP-safe heap implementation and that path is tested.

The pinned LK branch is MTK-oriented and AArch32; it is a source reference, not an H700-ready kernel.

## Architecture implication

The current boot-chain audit establishes that vendor BL31 enters the current BL33 slot in AArch32 SVC state (SPSR `0x1d3`). That makes an AArch32 LK-as-BL33 approach worth evaluating as a candidate because it aligns with the existing BL33 execution width. It is not yet selected: the precise replacement image/container expectations, CPU secondary-state ownership, hardware drivers, and cold-boot handoff remain unresolved. The AArch64 runtime route remains possible but would require an AArch64 LK architecture port or a separate runtime.

## Still not tested

- No H700 LK image or firmware built.
- No actual `lib/heap/heap.c` binary linked or executed.
- No A32 BL33 replacement, SMC, HVC, payload entry or hardware code executed.
- No framebuffer, LCD, keys, SD, audio, PMIC or emulator core was exercised.
- No SD card, boot sector, Boot0, TOC1, U-Boot or monitor was written.

The boot-handoff/RAM-ownership NO-GO gate remains in force.
