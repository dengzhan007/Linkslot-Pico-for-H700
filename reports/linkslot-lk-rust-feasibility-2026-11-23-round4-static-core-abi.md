# Linkslot 1.0.36 static Libretro core registry ABI — round 4

- Date: 2026-11-23
- Branch commit under test: `26e25ef2cb12550dbc984e0d84dc1ca5c3441aeb`.
- PR: [#16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16), draft; not merged.
- GitHub Actions: [run 45107173305](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45107173305), conclusion **SUCCESS**.
- Artifact: `linkslot-lk-rust-feasibility-45107173305`, ID `14787947224`.
- Rust/Cargo: 1.96.0.
- Targets: AArch32 `armv7a-none-eabi`, AArch64 `aarch64-unknown-none`.

## What was built

A temporary no_std Rust static library declares a function table matching the current field order and C ABI in Linkslot 1.0.36 `crates/slot-retro/src/ffi.rs::Api`. The table contains all function pointers that Linkslot's current wrapper loads, including optional cheat reset/set pointers.

Two C mock cores provide distinct namespaced Libretro exports:
- `mgba_retro_*`
- `gpsp_retro_*`

Rust exports `linkslot_static_core_get(core_id)`, returning the corresponding static API table. The probe does not use `libloading`, `dlopen`, `dlsym`, or any runtime dynamic symbol lookup.

## Results

| Validation | Result |
|---|---:|
| AArch32 Rust static API table + two C mock cores | PASS; ELF32/ARM; no undefined symbols |
| AArch64 Rust static API table + two C mock cores | PASS; ELF64/AArch64; no undefined symbols |
| Linker maps | Generated for both target ELFs |
| x86_64 host executable | PASS; exit code 0 |

The host executable called both static tables and verified:
- Libretro API version;
- environment callback setup and core init;
- video refresh callback (frame dimensions, pitch and pixels);
- audio sample and batch callbacks;
- input poll and input-state callbacks;
- controller port/device call;
- ROM data handoff and load/unload;
- frame run;
- optional cheat reset/set;
- serialize/unserialize round trip;
- save-memory data pointer and length;
- clean deinit.

The mock cores have different timing, state seeds and memory bytes so the test also proves that the core-ID registry selects the intended implementation rather than accidentally dispatching both IDs to one set of symbols.

## What this establishes

The current Linkslot Libretro API boundary can be represented as a static function table while preserving the callback and lifecycle shape. Multiple static cores can coexist if all their exported Libretro entry points and any internally referenced symbol names are consistently namespaced at compile time.

That gives a credible replacement for the current `libloading::Library` boundary:
1. compile an actual core as static objects/archive;
2. namespace its Libretro exports;
3. provide one static API table for that core;
4. have Linkslot select the table by core ID, without dynamic loading.

## What this does not establish

- The two mock cores are **not** mGBA or gpSP.
- No actual emulator source was compiled or linked.
- ROM execution, CPU/JIT code, graphics/audio correctness, save-format compatibility and performance were not tested.
- No H700 device or LK runtime was involved.
- This does not solve Linkslot's broader `std` dependencies, TLS active-host callback context, storage, UI or hardware backends.
- It does not certify memory addresses or allocate any H700 RAM region.

## Next real-core step

For the first GB/GBC/GBA milestone, build a real mGBA core as a namespaced AArch32 static archive and connect the registry to Linkslot's existing `RetroCore` lifecycle/callback adapter. The upstream model registry already permits mGBA for GBA and GB/GBC, so this gives one real core to validate across all three target systems. Only after that should gpSP be added as an optional second GBA core, with its own prefix and compatibility tests.

## Overall validation boundary

The full workflow run also passed the previous AArch32/AArch64 Rust candidate, static i18n, heap-header ABI, and A32 BL33 entry/header layout checks. No firmware was built, no SD/boot media was written, and no payload was executed.
