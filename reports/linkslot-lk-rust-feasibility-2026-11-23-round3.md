# Linkslot 1.0.36 no_std candidates, static i18n and A32 BL33 layout — round 3

- Date: 2026-11-23
- Feasibility branch commit under test: `cee3779a546630bfcd39e8d516ae1a0f56db8cfa`.
- PR: [#16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16), draft; not merged.
- GitHub Actions: [run 45106548485](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45106548485), conclusion **SUCCESS**.
- Artifact: `linkslot-lk-rust-feasibility-45106548485`, ID `14788005484`.
- Rust/Cargo: 1.96.0.
- Target triples: `armv7a-none-eabi` (AArch32), `aarch64-unknown-none` (AArch64).
- Immutable Linkslot snapshot: `upstream/linkslot` @ `8590d6ff1b2caa26118051069c975106fbb14bd3`.
- Pinned GammaOS LK reference: `TheGammaSqueeze/lk-mt6785` @ `9bf1419a49e0af6dba8b05916df382c9c9c6d0a8`.

## Round 3 — slot-i18n static no_std candidate

The original crate embeds twelve JSON translation files (six domains × two languages). Its runtime previously used `serde_json`, `HashMap` and `OnceLock`. The host-only candidate generator reads the same JSON files at build/test time and emits read-only Rust translation slices. The generated no_std crate has no runtime JSON parser, HashMap or OnceLock dependency; `t_args`, `placeholders` and `entries` retain `alloc`-based String/Vec operations.

### Source validation

- Six translation domains.
- **1,763 unique keys**, with **1,763 entries in each language**.
- All source JSON checks passed: schema and language metadata; UTF-8/no BOM/LF/final newline; nonempty strings; identical Chinese/English key sets; no duplicate keys across domains; placeholder parity.

### Tests and builds

| Test | Result |
|---|---:|
| Unmodified upstream `slot-i18n` unit tests | 9 passed |
| Unmodified upstream `translations.rs` integration tests | 9 passed |
| Isolated generated no_std candidate unit tests | 13 passed |
| Candidate AArch32 `cargo check` | PASS |
| Candidate AArch64 `cargo check` | PASS |

Candidate's 13 tests include the copied upstream language parsing/lookup/fallback/placeholder tests, complete table parity against all generated JSON pairs, sorted table/lookup checks, and all six domains × two languages being embedded.

This is a host build and behavior result. The table has not been linked into a complete Linkslot UI, its glyph/font resources have not been checked on the H700 display, and no H700 device has run it.

## Previously validated candidates rerun in the same workflow

- Existing H700 AArch64 C11/C++17/ASM scaffold: all three object checks passed.
- `slot-input`: AArch32 and AArch64 no_std+alloc checks passed; 47 upstream gesture tests passed against the temporary no_std candidate.
- `slot-model`: AArch32 and AArch64 no_std+alloc checks passed; four pure model registry/layout regression tests passed. Filesystem path helpers remained omitted from the candidate.
- Original `slot-input` + `slot-model` host baseline command passed in the earlier dedicated round (58 tests total).
- Rust `staticlib` + C shim links passed for AArch32 and AArch64; expected ELF machines and no undefined symbols.
- LK heap header ABI probe passed for AArch32 using the pinned `include/lib/heap.h` and a test-only C shim. It did **not** build or link the real LK `lib/heap/heap.c`.

## AArch32 LK-as-BL33 entry/header layout result

The workflow reran `scripts/audit-lk-bl33-layout.py` against the pinned LK sources.

- A32 entry branch at link address `0x4a000000` points to `0x4a001024`.
- Exception vector table was relocated to `0x4a001000`.
- `BOOT_ARGUMENT_LOCATION` was relocated to `0x4a001020`.
- Temporary stub ELF `_end` was `0x4a0033f8`; flat image size 5,100 bytes.
- The test synthetically overwrote bytes +4..+0x63f to model the BaseOS opaque header overlay and confirmed the first branch was preserved and the vectors/argument slot were outside that area.

Required future source adaptations remain:
- The real LK vectors and boot-argument slot must move beyond the BaseOS header window.
- LK self-relocation needs a literal-pool load after relocating reset code.
- `arch/arm/arch.c::arch_early_init()` must preserve the relocated VBAR rather than resetting it to MEMBASE.

**Critical limitation:** the layout probe uses a stub `kmain` and exception handlers. It is not a full LK build, does not use the actual vendor header, does not confirm full LK image size or platform compatibility, and was not executed.

## LK A53 and SMP blockers confirmed by source audit

The pinned LK branch currently:
- compiles its `cortex-a53` branch with `-mcpu=cortex-a9`;
- only sets ACTLR.SMP in a Cortex-A7 conditional branch, with no A53 path;
- has one global `current_thread`, global run queues and global `critical_section_count`, not a per-CPU scheduler;
- implements `arch_disable_ints` by setting the executing core's CPSR.I bit. This is local IRQ exclusion, not an SMP lock.

Therefore the reference scheduler/allocator is uniprocessor-oriented. RG34XX secondary-core state at BL31→BL33 is still unknown. Until the secondary-core state is established or LK is ported to SMP with per-CPU scheduling and cross-core synchronization, **do not treat this branch as safe for H700 four-core operation**.

## Linkslot Libretro loader implication

Linkslot `slot-retro` currently loads Linux `.so` files through `libloading::Library`, with `std`, filesystem paths and TLS callback context. Those binaries cannot be dropped into LK. A future backend must statically register or explicitly load namespaced core objects while retaining the tested `RetroCore` behavior: frame/video, audio, input, serialize/unserialize, battery save RAM, options, cheats, memory map, rumble and optional link/net callbacks.

GammaOS Pico's versioned `emu/gba/gba_core_abi.h` is only a draft contract (its source says it is not yet wired into the build). Its MTK fixed VMA `0x4e400000` is **not** an H700 address plan and must not be reused.

## Safety and validation boundary

This round performed host-side compilation, link and tests only:
- no SD card or boot sector was written;
- no Boot0, TOC1, U-Boot, BL31 or monitor was modified;
- no payload or ELF was executed on device;
- no flashable image or firmware was produced;
- startup handoff and RAM ownership remain NO-GO.

The workflow result is not evidence of a bootable RG34XX build.
