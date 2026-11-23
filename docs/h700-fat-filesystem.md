# H700 FAT16/FAT32 and exFAT storage layer

## Source and license provenance

The initial H700 storage candidate is copied from the GammaOS Pico LK reference:

- Repository: https://github.com/TheGammaSqueeze/lk-mt6785
- Pinned commit: `9bf1419a49e0af6dba8b05916df382c9c9c6d0a8`
- Original paths: `emu/gba/fat_ro.*`, `emu/gba/fat_wr.*`, `emu/gba/fat_exfat.*`
- Host test sources: `emu/gba/fat_ro_test.c`, `emu/gba/fat_exfat_rw_test.c` (copied test source path corrected to `fat_exfat_rw_test.c`)

The pinned repository's `LICENSES.md` explicitly places GammaOS Pico additions under `emu/` under its MIT license. The imported files retain their original source headers. Preserve the upstream MIT copyright and permission notice in any redistribution.

## Separation

The imported code is the generic filesystem layer only:

- `fat_ro.c/.h`: FAT16/FAT32 mount and read, MBR or superfloppy, subdirectories and VFAT long names.
- `fat_wr.c/.h`: FAT16/FAT32 directory creation and whole-file create/replace.
- `fat_exfat.c/.h`: exFAT mount, directory traversal, file reads and write operations.
- All sector access is supplied by `fat_read_fn` / `fat_write_fn` callbacks in `fat_vol`.

Do **not** import the adjacent `sd_fat.c/.h` as an H700 driver. It depends on MediaTek `mmc_wrap_bread/bwrite`, `mmc_legacy_init`, `msdc_ext_sd_power_on`, MTK watchdog and mutex services, and MTK-specific SD host numbering/power sequencing. None is evidence of H700 controller support.

## Current validation

- `scripts/test-h700-fat-compile.py` compiles all three production C files and the host test harnesses with C11, `-Wall -Wextra -Werror`. It also forces the `__aarch64__` preprocessor branch in `fat_wr.c` against narrow LK mutex/critical-section declarations, so the AArch64 writer-lock branch is not silently compiled out.
- Full feasibility workflow [45133921000](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45133921000) passed. Its real-image test generated a 128 MiB FAT32 image and 256 MiB exFAT images (4 KiB and 128 KiB clusters). FAT32 reader checks passed for VFAT long-name enumeration, multi-cluster reads and odd-offset reads. exFAT checks passed for long-name ROM reads, partial reads, mkdir, create/replace, 32 KiB and 1 MiB writes, readback, and `fsck.exfat -n` on both cluster sizes.
- The successful run artifact is [14798564036](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45133921000), 380,273,334 bytes; SHA-256 `17a71fdada4af8d4b1e3eb651a1aba112c1688b81570037d6e35f81a6b53f8cf`.
- A further FAT32 writer regression now covers VFAT long-name create/replace/delete, empty files and invalid path rejection; it was added after that run and is awaiting the next CI result. Do not count it as passed until the current run completes.

This is host-image validation, not hardware validation. The test callbacks use ordinary host file I/O. They do not exercise the H700 SD/MMC controller, cache flush/barrier semantics, actual card power loss, or Linkslot's VFS handle table. Failure-injection and write-order/power-cut tests are still required before exposing writes to Linkslot.

## Concurrency audit

The imported FAT writer originally guarded its write critical section only under `__arm__`. H700 is AArch64, so that condition left `wr_lock()` / `wr_unlock()` as no-ops on the target. The H700 port now enables the same LK mutex path for both `__arm__` and `__aarch64__`. This addresses writer-vs-writer serialization only; readers are not yet coordinated with writers, and the lazy mutex initialization / full runtime behavior still require target-LK integration validation.

## Concurrency audit

The imported FAT writer originally guarded its write critical section only under `__arm__`. H700 is AArch64, so that condition left `wr_lock()` / `wr_unlock()` as no-ops on the target. The H700 port now enables the LK mutex path for both `__arm__` and `__aarch64__`; the AArch64 preprocessor branch compiles in CI against stub declarations matching the pinned LK API. This is not yet a target-LK link or runtime test. It serializes writer-vs-writer operations only: readers are not coordinated with writers, and lazy mutex initialization via local critical-section masking is not an SMP-safety proof.

## Not implemented

- H700 SD/MMC host driver, clocks, pinmux, card power and card-detect.
- Partition selection and mount policy.
- VFS file/directory handle table and path resolver wiring.
- Durable block flush/barrier, power-loss-safe save/state replacement.
- Concurrency protection across frontend, core callbacks and storage.
- Device testing. No card writes or boot writes are authorized by this port.

## Compatibility constraints discovered in Linkslot 1.0.36

Linkslot save/state naming is not limited to 8.3 names. `slot-store::StateRing` creates paths such as `States/<platform>/<core>/<game-stem>/<timestamp>.state`, `resume.state`, and matching PNG thumbnails. `slot::persist::write_sav` writes `Saves/<platform>/<game-stem>.sav`; `slot-store::atomic_write` writes a temporary sibling, calls file sync, renames it over the destination, then syncs the parent directory.

The imported FAT16/32 writer explicitly supports only 8.3 file and directory names. It is therefore **not compatible as-is** with Linkslot save/state paths. The exFAT writer supports UTF-8/UTF-16 long names, but its current callback contract has no explicit durable flush/barrier operation and its rename semantics do not yet provide the full `atomic_write` contract. Do not expose FAT32/exFAT writes to Linkslot until long-name support, temporary-file rename, flush ordering and power-cut recovery are designed and tested. Read-only FAT/exFAT support can be evaluated separately.
