# H700 card path containment and storage-backend audit — round 14

Date: 2026-11-23  
Feature commit: 4bb58d1c530bae6d732673b081c04f7f62e1f5ac  
Feasibility workflow: [45130158723](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45130158723) — **PASS**  
Artifact: [14797343239](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45130158723) — 380,223,927 bytes; SHA-256 `c8e7837b099d3cd5e5ffc1652d22ff98a041e3d5f8a826069e8b25fc9ed81b33`.

## Implementation

Added `platform/h700/runtime/storage_path.rs`, a standalone Rust lexical containment layer for the future card-backed Libretro VFS provider.

The API accepts:
- the exact mount prefix exposed to Linkslot (configured by the platform; not guessed by this module);
- a caller-provided allowlist of card-root directories;
- an explicit base directory for relative paths, or no base for absolute paths.

It returns a normalized card-relative path only after checking the mount boundary and every component. It rejects parent/current-directory traversal, repeated or trailing separators, control characters, backslashes, drive/URI prefixes, paths outside the mount, and roots not in the caller's allowlist. It deliberately has no process-CWD fallback.

Added `scripts/test-h700-storage-path.py`; CI compiles the module directly with `rustc --test` and runs its host unit tests. The path-policy step passed **9/9** tests in the complete feasibility workflow.

## Broader CI result

The full Linkslot feasibility workflow passed on this commit. In addition to the new path-policy tests, the run passed the slot static-mGBA selection/override regression, Rust static API binding, VFS v3 integration, actual mGBA VDir fake-provider tests, AArch32/newlib builds, real mGBA archive symbol binding and final link diagnosis, and the synthetic LK-as-BL33 layout audit.

This is a host/CI feasibility result. The new path resolver is not yet called by `linkslot_vfs.rs`, and it does not implement block I/O, filesystem operations, file/directory handles, durable flush, atomic replacement or callback synchronization.

## Storage implementation candidate discovered in pinned LK reference

The pinned GammaOS LK source (`TheGammaSqueeze/lk-mt6785` @ `9bf1419a49e0af6dba8b05916df382c9c9c6d0a8`) contains a useful separation in `emu/gba/`:

- `fat_ro.c/.h`: block-callback-driven FAT16/FAT32 reader, MBR or superfloppy mount, subdirectories and VFAT long names.
- `fat_wr.c/.h`: FAT16/FAT32 create/replace and directory creation.
- `fat_exfat.c/.h`: exFAT read, directory traversal and file replacement.
- `fat_ro_test.c` and `fat_exfat_rw_test.c`: host-side test harnesses backed by image files.
- `sd_fat.c/.h`: device-specific SD/MMC adapter and ROM catalogue logic.

The repository's `LICENSES.md` explicitly lists GammaOS Pico additions under `emu/` as MIT. If these generic FAT modules are reused, preserve the upstream MIT notice and source attribution.

**Do not copy `sd_fat.c` as an H700 driver.** It calls MediaTek-specific `mmc_wrap_bread/bwrite`, `mmc_legacy_init`, `msdc_ext_sd_power_on`, MTK watchdog hooks and an MTK storage mutex. The pinned LK platform is MT6785-oriented; those functions and its MSDC numbering/power sequencing are not valid H700 hardware evidence.

## Next implementation gate

1. Port/adapt the generic FAT and exFAT modules behind a clean 512-byte block-device callback, preserving their MIT notices.
2. Re-run and extend host image tests for FAT32 and exFAT, including large files, partial reads, replacement, allocation failure and interrupted-write behavior.
3. Implement a separate Allwinner H700 SD/MMC block provider only after locating and validating an H700-compatible controller driver, clocks, pinmux, power and card-detect path.
4. Connect the block/filesystem provider to the path resolver and Libretro VFS callbacks only after both layers have host regression coverage.

No H700 SD controller driver, card mount, FAT/exFAT write, or persistent save path is claimed by this round.

## Safety boundary

No firmware, SD card, boot partition or device state was written. No payload was executed. Boot/handoff remains **NO-GO** pending the independent LK entry, memory ownership and multicore safety gates.
