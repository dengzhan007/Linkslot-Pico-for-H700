# Linkslot slot-retro static API entry — round 11

Date: 2026-11-23  
Feature branch: `porting/linkslot-1.0.36-feasibility`  
Validated commit: `2ebe74b5e4783f6ea99eb047c7c78e522669fad5`  
Validation workflow: [45126045887](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45126045887)  
Immutable Linkslot source snapshot: `upstream/linkslot` @ `8590d6ff1b2caa26118051069c975106fbb14bd3`

## Result

**PASS — Linkslot's real slot-retro crate now supports both dynamic and statically supplied Libretro API tables.**

The change is applied to the temporary CI checkout only. The immutable `upstream/linkslot` branch was not modified.

### What changed

1. `libloading` is now an optional Cargo dependency of `slot-retro`.
2. A default `dynamic` feature preserves the original host/frontend behavior, so normal builds still load core .so files as before.
3. The public `Api` function-pointer table and callback/ABI types are re-exported for building a statically supplied table from resident core symbols.
4. `LibretroCore::open_static(api, system_dir, save_dir)` accepts a table of functions linked into the current executable.
5. The dynamic and static entry points share a single `open_api` implementation for API-version checking, environment/video/audio/input callback registration and `retro_init()`.
6. Dynamic library ownership remains pinned in the core object for dynamic loading; static mode uses an explicit static owner variant and does not require `libloading`.
7. A static-only build explicitly rejects the dynamic `open_with()` entry rather than trying to load a shared object.

### Tests

The main feasibility workflow completed successfully on commit `2ebe74b`.

| Test mode | Command | Result |
|---|---|---:|
| Default / dynamic enabled | `cargo test --locked -p slot-retro --lib` | **61 passed, 0 failed** |
| Static only | `cargo test --locked --no-default-features -p slot-retro --lib` | **62 passed, 0 failed** |

New regression coverage:
- Constructs a mock static API table without a dynamic library handle.
- Opens it with `LibretroCore::open_static()`.
- Checks that `retro_init` is called exactly once.
- Checks the configured system and save directory values.
- Drops the core and checks that `retro_deinit` is called exactly once.
- In static-only mode, verifies that dynamic loading returns the explicit Unsupported error.
- Re-runs the VFS v3 integration against the patched production `slot-retro::environment()` callback.

The default run has 61 tests; static-only has 62 because it includes the dynamic-disabled behavior test. Existing Linkslot behavior tests remain green.

### Full workflow and artifact

- Main workflow: [45126045887](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45126045887)
- Conclusion: success
- Artifact: `linkslot-lk-rust-feasibility-45126045887`
- Artifact ID: `14795752171`
- Size: 96,408,534 bytes
- SHA-256: `90bd21039261e2bd7b09b792931147223f840ef4c9c7ed305dd802e29f4b8efa`
- Retention expiry: 2026-12-23

The full workflow also re-ran the existing checks: selected no_std candidates, AArch32/AArch64 ABI probes, VFS v3, Generic and newlib mGBA archives, mGBA VDir fake-provider tests, H700 newlib adapter, whole-archive newlib link diagnosis and synthetic LK BL33 layout probe. All steps completed successfully.

## Important limits

- The static entry is tested against a mock function table. It is not yet connected to the actual namespaced mGBA functions.
- It does not make `slot-retro` no_std: this crate still relies on `std`, Vec/HashMap, thread synchronization and other host facilities.
- The rest of the Linkslot workspace is not a bootable LK application.
- The H700 VFS bridge still forwards into an abstract provider; no SD block driver or FAT32/exFAT backend exists.
- No persistent save/state handling or power-loss-safe flush/rename implementation exists.
- No real LK heap, UART/console or RTC has been integrated; the allocator remains fail-closed pending heap-ready and single-core proofs.
- No safe H700 memory map, BL33 payload-entry contract, secondary-core hold-off, or SMP implementation has been validated. Boot/handoff remains **NO-GO**.
- No firmware, SD or boot-area write, payload execution or device ROM test was performed.

## Next step already prepared

Commit `6311b8f69396e985da9c9f366df0b80a10cf53c6` adds a separate AArch32 probe that compiles all 21 namespaced `mgba_retro_*` function references into a Linkslot-shaped API object, combines it with the H700 newlib adapter and feeds that object into the actual mGBA whole-archive link. This is a symbol/layout link proof only; it does not call the Rust `LibretroCore::open_static()` with real mGBA at runtime. That updated workflow is still running and is not counted as a pass in this report.

## References

- [Draft PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16)
- [Feature branch](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/porting/linkslot-1.0.36-feasibility)
- [Round 10 report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/linkslot-lk-rust-feasibility-2026-11-23-round10-vfs-static-newlib.md)
