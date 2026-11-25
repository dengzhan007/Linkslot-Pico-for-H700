# Linkslot static mGBA API symbol binding and final link — round 12

Date: 2026-11-23  
Feature branch: `porting/linkslot-1.0.36-feasibility`  
Validated commit: `13c82392e05dd7ecb0c74899697736f812a93a8b`  
Workflow: [45126961555](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45126961555)  
Immutable Linkslot snapshot: `upstream/linkslot` @ `8590d6ff1b2caa26118051069c975106fbb14bd3`

## Result

**PASS — the actual AArch32 mGBA archive now participates in a static-API-shaped whole-archive link with the H700 newlib adapter.**

The workflow completed successfully. It re-ran all prior gates, including VFS v3 production callback integration and both dynamic/static Linkslot loader-mode tests.

### New probe

Added `scripts/probe-mgba-static-api-link.py`, executed after the H700 newlib adapter has been built.

It compiles a freestanding C representation matching the field order and callback signatures of `slot-retro::ffi::Api` in Linkslot 1.0.36. The table references all 21 functions:

- cheat reset and set;
- init, deinit and API version;
- system AV information;
- environment, video, audio and input callback registration;
- controller device selection;
- load/unload/run;
- serialize size, serialize and unserialize;
- memory data and memory size.

All references use the actual namespaced `mgba_retro_*` exported symbols. Strict AArch32 compilation with `-Wall -Wextra -Werror` passed, and all 21 expected undefined references were present in the generated ELF32/ARM object.

The probe then combines the static API table object with the H700 newlib adapter using `arm-none-eabi-ld -r`. The following whole-archive link step now uses that combined object, not the old adapter object.

### Final link result

Against the actual mGBA AArch32 VFS v3 archive:

| Link configuration | Result |
|---|---|
| Whole archive + static API symbol table + H700 newlib adapter + newlib | **PASS**, ELF32/ARM, no undefined symbols |
| Same link with `libnosys` as an additional diagnostic library | **PASS**, ELF32/ARM, no undefined symbols |

The `newlib-only` result is the meaningful closure proof for the tested symbol set. The `libnosys` result remains diagnostic only: its syscalls are dummy stubs and do not implement LK services. The link entry is a synthetic `_start`, not LK startup code.

### Workflow and artifact

- Workflow: [45126961555](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45126961555)
- Conclusion: success
- Commit: `13c82392e05dd7ecb0c74899697736f812a93a8b`
- Artifact: `linkslot-lk-rust-feasibility-45126961555`
- Artifact ID: `14796725457`
- Size: 96,415,151 bytes
- SHA-256: `e8a00812d8bf59488a5bd5346e0bc0a832d390278b35a8387be65c1194e3e11b`
- Retention expiry: 2026-12-23

### Prior Linkslot static entry results repeated

The same workflow re-applied the VFS patch and static API patch to a clean temporary checkout of the immutable 1.0.36 snapshot:

- Dynamic-default `slot-retro` test mode: 61 passed, 0 failed.
- Static-only `--no-default-features` mode: 62 passed, 0 failed.
- The static lifecycle regression opens a mock API table with `LibretroCore::open_static()`, checks system/save path registration, calls init and verifies deinit on drop.
- The VFS regression calls the actual patched `slot-retro::environment()` callback.

The C static table is a symbol-binding/link probe shaped like the Rust `Api` type. It is not itself the Rust `Api` instance and does not call the real mGBA through `LibretroCore::open_static()` at runtime. This distinction is intentional and is not claimed as a runtime core test.

## Remaining blockers

1. The Rust `LibretroCore::open_static()` path is tested with a mock API; there is not yet a Rust module that declares and maps the 21 real namespaced mGBA symbols into that Rust struct.
2. `slot-retro` still uses `std`; disabling dynamic loading only removes `libloading`, not `std`, allocation, threads or host synchronization.
3. The broader `slot`, `slot-ui`, `slot-store` and `slot-device` crates are not yet an LK application. In particular `slot-store` relies on host filesystem/SQLite semantics.
4. No H700 SD block driver, FAT32/exFAT provider, path-root resolver or durable save/state implementation exists.
5. No real LK heap/UART/RTC binding, proven single-core hold-off or SMP-safe allocator exists. The LK heap check uses a signature-compatible test stub.
6. No H700 memory-ownership map, BL33 handoff proof, A53 vector/startup/SMP setup or complete LK build exists. Boot/handoff remains **NO-GO**.
7. No boot image, SD/boot-area write, payload execution or RG34XX device ROM test occurred.
8. armGBA is not present in the user-supplied 1.0.36 snapshot and is not part of this round.

## Next implementation step

Add a H700-specific Rust module that declares the namespaced mGBA symbols and constructs the actual `slot-retro::Api` Rust struct, under a Cargo feature that keeps the generic dynamic host path unchanged. Test its `LibretroCore::open_static()` lifecycle with host C stubs, then use the cross-compiled symbol/link probe to confirm that these same symbol names resolve against the real AArch32 mGBA archive. After that, add a narrowly scoped static-mGBA selection path in `slot::core`; keep the full LK/no_std and hardware-storage gates separate.

## References

- [Draft PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16)
- [Feature branch](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/porting/linkslot-1.0.36-feasibility)
- [Round 11 static API entry report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/linkslot-static-core-api-2026-11-23-round11.md)
