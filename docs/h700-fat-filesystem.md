# H700 FAT16/FAT32 and exFAT storage layer

## Source and license provenance

The initial H700 storage candidate is copied from the GammaOS Pico LK reference:

- Repository: https://github.com/TheGammaSqueeze/lk-mt6785
- Pinned commit: `9bf1419a49e0af6dba8b05916df382c9c9c6d0a8`
- Original paths: `emu/gba/fat_ro.*`, `emu/gba/fat_wr.*`, `emu/gba/fat_exfat.*`
- Host test sources: `emu/gba/fat_ro_test.c`, `emu/gba/fba_exfat_rw_test.c` (copied test source path corrected to `fat_exfat_rw_test.c`)

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
