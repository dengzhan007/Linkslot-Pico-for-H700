# Linkslot 1.0.36 → RG34XX/H700 storage candidate and MMC reference (round 15)

Date: 2026-11-23  
Work branch: `porting/linkslot-1.0.36-feasibility`  
Validated head: `53eb7c6e1cafc0d0fe71567be6d82e474a45b2e3`  
PR: [#16 — draft](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16)

## Summary

This round imports the generic FAT16/FAT32 and exFAT implementation from the pinned GammaOS LK reference, establishes real disk-image regressions, fixes the AArch64 writer-lock guard, and adds bounds/overflow hardening. It also records the first useful RG34XX H700 MMC0/MMC2 board wiring references from U-Boot and ROCKNIX.

The work is still a host-tested filesystem candidate, **not an H700 SD driver or Linkslot persistence backend**. No device, SD card, boot partition, or firmware image was written.

## Imported source and license

Reference: `TheGammaSqueeze/lk-mt6785` at `9bf1419a49e0af6dba8b05916df382c9c9c6d0a8`.

Imported under `platform/h700/storage/fat/`:
- `fat_ro.c/.h` — FAT16/FAT32 and exFAT read/dir dispatch.
- `fat_wr.c/.h` — FAT16/FAT32 create/replace/delete/mkdir candidate.
- `fat_exfat.c/.h` — exFAT read/write/mkdir candidate.
- upstream host harnesses `tests/fat_ro_test.c` and `tests/fat_exfat_rw_test.c`.
- new `tests/fat_fat32_rw_test.c`.

GammaOS `LICENSES.md` identifies the `emu/` additions as MIT. Original source headers and license notices are retained. The adjacent `sd_fat.c/.h` is MediaTek-specific and was not imported as an H700 driver.

## Changes in this round

1. **AArch64 lock guard:** changed the FAT writer mutex path to compile for both `__arm__` and `__aarch64__`. The host compile audit forces the AArch64 preprocessor branch against stub declarations matching the pinned LK mutex/critical-section API.
2. **FAT32 LFN clarification:** `fat_wr_put()` writes VFAT long file names; newly created FAT32 directory components in `fat_wr_mkpath()` are still limited to 8.3.
3. **Path bounds:** reject truncated/overlong FAT/exFAT path components. The FAT32 mkdir path previously copied up to 254 bytes into a 64-byte local buffer; it now bounds the short-name component before copying.
4. **Cluster arithmetic:** replace `len + cluster_bytes - 1` ceiling calculations with overflow-safe `1 + (len - 1) / cluster_bytes`; guard exFAT directory length growth against 32-bit overflow.
5. **Host regression runner:** `scripts/test-h700-fat-images.sh` creates real FAT32/exFAT images, runs the read/write harnesses, and invokes filesystem checkers.

## CI evidence

Full workflow [45135115298](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45135115298) completed successfully on validated head `53eb7c6`. The workflow also passed the existing Linkslot no_std/VFS/static-mGBA, AArch32/newlib, and A32 BL33 layout probes.

Artifact:
- ID: `14800060917`
- Size: `380,276,489` bytes
- SHA-256: `6041a12137559f0fdf3f043a755f9d20e60b76e55a870e4a9296ff8a4a563f77`
- [Download artifact](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45135115298)

Filesystem-specific steps:
- Step 18 — host compile: **PASS** for `fat_ro.c`, `fat_wr.c`, `fat_exfat.c`, all three test harnesses, and the AArch64 writer-mutex branch.
- Step 19 — real-image regressions: **PASS**.

### FAT32 image

A 128 MiB FAT32 image was created with `mkfs.fat` and populated using mtools.

Passed:
- VFAT long-name directory enumeration.
- BIOS/ROM file open and multi-cluster ROM read (300,000 bytes).
- Mid-file offset read.
- Long-name save create, replace and delete.
- Empty-file metadata.
- Rejection of an overlong directory component.
- `fsck.fat -n` exited successfully. It reports the FSInfo free-cluster summary as uninitialized because the writer deliberately writes the FAT32-spec unknown sentinel `0xFFFFFFFF` after allocation/free; this is not treated as a filesystem-corruption failure.

### exFAT images

Two 256 MiB images were tested: 4 KiB and 128 KiB clusters.

Passed:
- Long-name ROM reads (3 KiB, 3 MiB, and 70,001 bytes).
- Partial/odd-size reads.
- Directory creation and long-name file operations.
- Create/replace/write/readback through the imported writer harness.
- `fsck.exfat -n` reported both images clean.

These tests use host file callbacks and real on-disk filesystem images. They do not exercise an SD/MMC controller or physical card.

## RG34XX H700 MMC reference findings

Pinned U-Boot reference: `u-boot/u-boot` commit `e578056c766d653270b14557c2a9d8daec414c17`.

From `dts/upstream/src/arm64/allwinner/sun50i-h616.dtsi`:
- MMC0 base `0x04020000`
- MMC1 base `0x04021000`
- MMC2 base `0x04022000`

From `arch/arm/include/asm/arch-sunxi/clock_sun50i_h6.h`:
- MMC module-clock config offsets: `0x830`, `0x834`, `0x838`.

From `drivers/clk/sunxi/clk_h616.c`:
- MMC bus gate/reset register offset `0x84c`
- gate bits 0–2, reset bits 16–18.

From `drivers/mmc/sunxi_mmc.c`: H616 FIFO threshold setup and controller clock sequencing reference. This is U-Boot code only, not an LK port.

ROCKNIX patch `0126-20241018_macroalpha82_rg35xx_add_additional_hardware_support.patch` at distribution commit `1d806accd02e9bbfeca03cb5497179e77c7c18d8` adds the second card slot:
- MMC0 primary slot: 4-bit bus, card detect PF6, CLDO3 supply, no 1.8 V signaling.
- MMC2 second slot: 4-bit bus, card detect PE22, external 3.3 V regulator controlled by PE4, CLDO3 I/O supply, no 1.8 V signaling.
- MMC1 is Wi-Fi SDIO. Linux aliases swap MMC1/MMC2 numbering, so Linux `mmcblkN` must not be equated directly to physical controller number.

ROCKNIX patch `0158-mmc-sunxi-mmc-mask-data0-when-updating-the-clock-on-h616-emmc.patch` explicitly identifies MMC2 as the second SD slot on RG34XX/RG35XX H and adds H616 `mask_data0` handling. The `emmc` compatible is used for a removable SD-slot controller in this design.

This identifies likely board wiring but does **not** prove which slot contains the currently booted/game card on the test unit. The live read-only inventory must bind `/dev/mmcblk*` to its DT `of_node`, card-detect GPIO, pinctrl, regulator and clock state before choosing a controller to initialize in LK.

## Remaining blockers — do not expose writes to Linkslot

- FAT32 can write long file names, but `fat_wr_mkpath()` cannot create new long-name directory components. Linkslot paths such as `States/<platform>/<core>/<game-stem>/` are therefore not fully supported on FAT32.
- Neither writer implements the complete `slot-store::atomic_write` contract: no same-directory temporary-file rename API, no durable file/parent-directory sync or block flush/barrier contract, and no power-cut recovery proof.
- Writer mutex serialization is not reader/writer synchronization. Readers can still race with directory/bitmap/FAT updates. Lazy mutex initialization via local critical-section masking is not an SMP proof and has not been linked/run against target LK.
- No H700 SD/MMC driver, card power/pinmux initialization, partition mount, VFS handle table, or path-resolver integration exists yet.
- No physical-card read test, no device write, no ROM execution, no firmware image, and no boot/handoff approval.

Next engineering gates: add FAT32 LFN directory creation or explicitly keep FAT32 read-only; implement a provider-level reader/writer lock; define a real block flush contract and atomic replacement; then port the H616/H700 SD controller using the verified RG34XX board wiring and read-only live inventory. Boot/handoff remains **NO-GO**.
