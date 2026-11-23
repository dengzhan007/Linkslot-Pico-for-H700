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

## H700 CMD17 read-only bridge status

The H700-facing glue now includes `h700_sunxi_mmc_fat.c`:

- `h700_sunxi_mmc_fat_read()` implements the imported `fat_read_fn` callback by issuing one CMD17 per 512-byte sector. It always uses an explicitly aligned bounce sector before copying into the caller's buffer, since the FAT code's byte-oriented local arrays do not promise 4-byte alignment.
- `h700_sunxi_mmc_fat_mount_gpt_partition()` accepts an already-validated GPT entry, validates the full partition extent against card capacity and the 32-bit FAT LBA limit, then calls `fat_mount_partition()`. FAT16/FAT32/exFAT mount validation now checks the declared volume size, FAT capacity, heap/data region and root cluster against that extent.
- `scripts/test-h700-sunxi-mmc-fat.sh` covers aligned-bounce reads, sequential CMD17 sector reads and a read-only FAT32 p7 mount. `scripts/test-h700-sunxi-mmc-fat-scan.sh` goes further: a synthetic FAT32 disk is mounted through GPT/CMD17, its ROM directory is enumerated, and ROM/BIOS file content is read back through the PIO path.

All of these are host-side register/media simulations. Round 16 independently confirms that the live system/game volume is connected to sdc0 at `0x04020000`, with p7 mounted by Linux at `/mnt/sdcard`; this mapping is not inferred from the `mmcblk0` suffix. The H700/LK driver itself has still not read any physical VBR/FAT sector. The sdc0 top-level bring-up API requires target MMIO mappings, PIO/APB1 enablement and card power; it has not been run against live hardware.

## Read-only Libretro VFS v3 backend
A higher-level `h700_fat_vfs_scan_partition_and_install()` now packages startup ordering for a caller: run the read-only sdc0 bring-up/SD identification/primary GPT scan, locate one exact GPT name (for the RG34XX inventory, `primary`), recreate the CMD17 block-device state in caller-owned persistent memory, mount that partition with its exact extent, and install the VFS v3 backend. The caller supplies a persistent provider object, a persistent block-device object, and a 128-entry GPT scratch array, and must call this before Libretro `retro_init()`. For the current RG34XX capture, the exact GPT name passed is `primary` (p7, mounted by Linux at `/mnt/sdcard`); the bootargs `UDISK` string is not the p7 GPT `PARTNAME`. The bootstrap does not power the card or enable/map PIO/APB1; those remain board preconditions.


`h700_fat_vfs_backend.c` installs a concrete `linkslot_vfs_backend` table through the existing `linkslot_vfs_install()` ABI. It uses fixed file and directory handle tables (no heap allocation), calls the CMD17-backed FAT reader for file and directory operations, and preserves the original absolute path in file handles. It accepts only absolute paths under the exact configured mount prefix and restricts the first path component to `Games`, `BIOS`, `Saves`, `States`, `System`, or `linkSlot`. The virtual mount root enumerates only real directories from that allowlist. The list is a subset of Linkslot 1.0.36's `crates/slot/src/root.rs::DIRS` (which creates `Games`, `BIOS`, `Saves`, `States`, `System`, and `linkSlot` among its card directories); the BIOS path used by the frontend is `BIOS/gba_bios.bin`. Both the C backend and Rust lexical path guard now compare allowed root names using FAT-compatible ASCII case-insensitive matching.

The backend is intentionally read-only: `open` accepts only `FILE_ACCESS_READ`; write, flush, remove, rename, truncate and mkdir callbacks are present for VFS v3 ABI completeness but always fail. It rejects relative paths, traversal, control characters, backslashes, colon/URI prefixes, sibling mount prefixes, and paths outside the allowed roots. The static provider object, mounted volume and MMC device must remain alive for process lifetime. VFS/MMC calls currently assume serialized callbacks; no SMP lock or reader/writer synchronization is added.

`scripts/test-h700-fat-vfs-backend.sh` exercises the C ABI table with a fake `linkslot_vfs_install`, mounts a synthetic GPT p7 through CMD17, opens and reads a ROM, seeks, stats, enumerates directories, verifies the virtual root allowlist, and confirms all mutation callbacks fail without issuing an SD command. The existing Rust `linkslot_vfs.rs` test separately validates the production Libretro VFS v3 bridge. These are two-sided host tests, not a whole-Linkslot runtime/device integration. A target startup caller still needs to hold static provider/MMC state and call `h700_fat_vfs_scan_partition_and_install()` with GPT name `primary` before `retro_init()`. The current Linkslot upstream source is not yet changed to invoke this H700 bootstrap; the CI host test validates the ABI bridge/patch, not target startup wiring.


- `scripts/test-h700-fat-compile.py` compiles all three production C files and the host test harnesses with C11, `-Wall -Wextra -Werror`. It also forces the `__aarch64__` preprocessor branch in `fat_wr.c` against narrow LK mutex/critical-section declarations, so the AArch64 writer-lock branch is not silently compiled out.
- Full feasibility workflow [45135115298](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45135115298) passed on head `53eb7c6e1cafc0d0fe71567be6d82e474a45b2e3`. The real-image test generated a 128 MiB FAT32 image and 256 MiB exFAT images (4 KiB and 128 KiB clusters). FAT32 reader checks passed for VFAT long-name enumeration, multi-cluster reads and odd-offset reads. FAT32 writer checks passed for VFAT long-name create/replace/delete, empty files and overlong directory-component rejection. `fsck.fat -n` returned success; it reports the FSInfo free-cluster count as uninitialized because the writer deliberately sets the spec-defined unknown sentinel `0xFFFFFFFF` after allocation/free. exFAT checks passed for long-name ROM reads, partial reads, mkdir, create/replace, 32 KiB and 1 MiB writes, readback, and `fsck.exfat -n` on both cluster sizes.
- The successful run artifact is [14800060917](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45135115298), 380,276,489 bytes; SHA-256 `6041a12137559f0fdf3f043a755f9d20e60b76e55a870e4a9296ff8a4a563f77`.
- The path hardening now rejects truncated/overlong FAT/exFAT path components, prevents the FAT32 directory-component stack-buffer overrun, and uses overflow-safe cluster-count arithmetic.

This is host-image validation, not hardware validation. The test callbacks use ordinary host file I/O. They do not exercise the H700 SD/MMC controller, cache flush/barrier semantics, actual card power loss, or Linkslot's VFS handle table. Failure-injection and write-order/power-cut tests are still required before exposing writes to Linkslot.

## Concurrency audit

The imported FAT writer originally guarded its write critical section only under `__arm__`. H700 is AArch64, so that condition left `wr_lock()` / `wr_unlock()` as no-ops on the target. The H700 port now enables the same LK mutex path for both `__arm__` and `__aarch64__`. This addresses writer-vs-writer serialization only; readers are not yet coordinated with writers, and the lazy mutex initialization / full runtime behavior still require target-LK integration validation.

## Concurrency audit

The imported FAT writer originally guarded its write critical section only under `__arm__`. H700 is AArch64, so that condition left `wr_lock()` / `wr_unlock()` as no-ops on the target. The H700 port now enables the LK mutex path for both `__arm__` and `__aarch64__`; the AArch64 preprocessor branch compiles in CI against stub declarations matching the pinned LK API. This is not yet a target-LK link or runtime test. It serializes writer-vs-writer operations only: readers are not coordinated with writers, and lazy mutex initialization via local critical-section masking is not an SMP-safety proof.

## H700 SD/MMC controller reference (not an implementation)

A pinned U-Boot reference is available at `u-boot/u-boot` commit `e578056c766d653270b14557c2a9d8daec414c17`:

- `dts/upstream/src/arm64/allwinner/sun50i-h616.dtsi` defines MMC0/MMC1/MMC2 at `0x04020000`, `0x04021000`, and `0x04022000`; their clock and reset names are `CLK_BUS_MMCx`, `CLK_MMCx`, and `RST_BUS_MMCx`.
- `arch/arm/include/asm/arch-sunxi/clock_sun50i_h6.h` gives MMC module-clock config offsets `0x830/0x834/0x838`.
- `drivers/clk/sunxi/clk_h616.c` gives the MMC bus gate/reset register at offset `0x84c`: gate bits 0–2 and reset bits 16–18.
- `drivers/mmc/sunxi_mmc.c` is the H616-capable controller reference, including its H616 FIFO threshold setup. It is U-Boot code, not an LK driver.

The ROCKNIX H700 patch `0126-20241018_macroalpha82_rg35xx_add_additional_hardware_support.patch` (distribution commit `1d806accd02e9bbfeca03cb5497179e77c7c18d8`) adds the second SD slot on MMC2 for the RG35XX family; RG34XX's ROCKNIX DTS inherits the RG35XX Plus DTS. Its board wiring is:

- MMC0 (primary slot): 4-bit bus, card-detect PF6, `vmmc-supply = reg_cldo3`, write-protect disabled, 1.8 V signaling disabled.
- MMC2 (second slot): 4-bit bus, card-detect PE22, external 3.3 V regulator enabled by PE4, `vmmc-supply = reg_vcc3v3_mmc2`, `vqmmc-supply = reg_cldo3`, write-protect disabled, 1.8 V signaling disabled.
- MMC1 is the Wi-Fi SDIO controller on RG35XX Plus; the patch's Linux aliases deliberately swap MMC1/MMC2 numbering (`mmc1 = &mmc2`, `mmc2 = &mmc1`). Therefore Linux `mmcblkN` numbering must not be treated as the physical controller number.
- The ROCKNIX patch `0158-mmc-sunxi-mmc-mask-data0-when-updating-the-clock-on-h616-emmc.patch` explicitly says MMC2 drives the second SD slot on RG34XX and RG35XX H, and adds `mask_data0` to the H616 eMMC controller configuration. Despite the `emmc` compatible, this is a removable SD-slot use case on those boards.

The round-16 live capture now resolves which slot holds the currently booted/game card: `sdc0` / MMIO `0x04020000` owns `mmcblk0`, the running SDHC medium, and the p7 volume mounted at `/mnt/sdcard`. The optional empty second slot candidate is `sdc2` / `0x04022000`; `sdc1` is SDIO Wi-Fi. The U-Boot/Linux implementations remain reference material only; their initialization and power sequencing have not been ported or hardware-tested in LK.

## Not implemented

- Physical H700 SD/MMC operation: the current sdc0 code is host-tested only; real MMIO mapping, PIO/APB1 clock, card power, card-detect and a live CMD17 read remain unvalidated on RG34XX.
- Physical-card partition scan. The read-only GPT scanner and exact-name `primary` selector are implemented and host-tested, but the H700/LK driver has not yet scanned the physical card. The Linux capture confirms p7 is mounted at `/mnt/sdcard`; it does not substitute for an H700 CMD17/GPT read.
- Target startup wiring: the C VFS provider and unified bootstrap API are host-tested, but the actual H700 Linkslot startup code still needs to provide persistent state and call `h700_fat_vfs_scan_partition_and_install()` before `retro_init()`.
- Durable block flush/barrier, power-loss-safe save/state replacement.
- Concurrency protection across frontend, core callbacks and storage.
- Device testing. No card writes or boot writes are authorized by this port.

## Compatibility constraints discovered in Linkslot 1.0.36

Linkslot save/state naming is not limited to 8.3 names. `slot-store::StateRing` creates paths such as `States/<platform>/<core>/<game-stem>/<timestamp>.state`, `resume.state`, and matching PNG thumbnails. `slot::persist::write_sav` writes `Saves/<platform>/<game-stem>.sav`; `slot-store::atomic_write` writes a temporary sibling, calls file sync, renames it over the destination, then syncs the parent directory.

The imported FAT16/32 writer explicitly supports only 8.3 file and directory names. It is therefore **not compatible as-is** with Linkslot save/state paths. The exFAT writer supports UTF-8/UTF-16 long names, but its current callback contract has no explicit durable flush/barrier operation and its rename semantics do not yet provide the full `atomic_write` contract. Do not expose FAT32/exFAT writes to Linkslot until long-name support, temporary-file rename, flush ordering and power-cut recovery are designed and tested. Read-only FAT/exFAT support can be evaluated separately.
