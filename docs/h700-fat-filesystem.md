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

`scripts/test-h700-fat-compile.py` compiles the three production C files and both upstream host test harnesses with C11, `-Wall -Wextra -Werror`. This is only a compile gate. The next gate is to run the host harnesses against generated FAT32 and exFAT images, then add failure-injection and durability-order tests before exposing writes to Linkslot.

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
