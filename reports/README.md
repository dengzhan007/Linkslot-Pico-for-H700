# RG34XX read-only inventory reports

## Index

- `rg34xx-inventory-20261122-195053.zip` — full read-only inventory archive (`main` @ `acc5e09`).
- `debugfs-memory-reservations.txt` — standalone extract from that archive.
- `rg34xx-inventory-20261122-204742/` — sanitized ION/TEE runtime subset (`main` @ `9b68302`).
- `rg34xx-inventory-20261122-210352/` — sanitized Allwinner DRM-info SMC follow-up
  (`audit/rg34xx-ion-drm-followup-20261122` @ `00fe17d`); see `DRM-SMC-finding.md`.
- `rg34xx-inventory-20261122-212049/` — sanitized ION config-symbol + kallsyms capture
  (`audit/rg34xx-ion-drm-followup-20261122` @ `b84fff0`); see `ION-symbol-finding.md`.
  Confirms `CONFIG_ION_SUNXI=y`, `CONFIG_ION_OF=y` and all three key functions in kallsyms.
- `rg34xx-inventory-20261122-214708/` — sanitized text/JSON subset of the boot-partition
  capture (`audit/rg34xx-ion-drm-followup-20261122` @ `c6da96d`); see
  `boot-partition-local-only.md`. The 64 MiB `boot-partition.bin` and all other binaries
  are kept local only, NOT uploaded.
- `rg34xx-boot-partition-deep-audit.md` — deep read-only analysis of the local
  `boot-partition.bin`: two Android images, AArch64 4.9.170 #19 kernel with
  extracted IKCONFIG, empty ramdisk, DTB cross-comparison (512 MiB on-disk vs
  1 GiB live), DRM `GET_DRM_INFO` source match to `qiaoweibiao/T507_Kernel`,
  memory-region attribution and LK hand-off recommendations. No binaries included.
- `rg34xx-bootloader-handoff-followup.md` — deep read-only follow-up: AArch32 U-Boot boot flow
  and image selection, full DTBO fragment decode (PMIC only — not the cause of 512 MiB → 1 GiB),
  U-Boot `fdt_fixup_memory_banks`/`update_fdt_dram_para` fixup path, the NAND-env vs MMC-root
  contradiction explained by `boot_type`, vendor monitor identified as AArch64 BL31 with the
  A32→A64 SMC contract (`0x8000ff04` / SPSR `0x3c5`), recovered kernel kallsyms addresses for
  the DRM path, **BOL4 decoded as LZ4 holding a byte-identical kernel**, and an A/B/C comparison
  of LK take-over architectures. No binaries, serial numbers or local paths included.
- `rg34xx-active-loader-provenance.md` — read-only provenance check establishing that the
  unit is **already running** the BaseOS native chain (native SPL → vendor BL31 → A32
  loader → BOL4 image), not TOC1 U-Boot: p4 + 40 MiB firmware extent map, p4 + 48 MiB
  BOL4 frame verification, exact bootargs-template match, live FDT generation/DRAM/timing
  cross-check, active-vs-fallback image distinction, ABI table, corrections to the two
  earlier reports, and unresolved items. No binaries, serial numbers or local paths.
- `rg34xx-stage0-and-a64-probe.md` — stage-0 identity closure plus a host-only AArch64 entry
  probe. Establishes that LBA 16 holds a BaseOS 128 KiB **raw capsule** (dispatcher +
  contract vendor Boot0 at `+0x1000` + 48 KiB BaseOS SPL at `+0x11000`), that LBA 512 is a
  *different* vendor Boot0 revision, and binds the boot area to the running system by
  `FNV-1a32(generation) == capsule cookie 0xf7bed1c8`. Publishes the deployed SPL fingerprint
  (`cdfc6d48…`) and the three extents' hashes, explains why the generation cannot be rebuilt
  (missing pinned `boot-package.bin` and build-time GPT prefix), resolves the `p6/p7` label
  drift against `rcS`/`fstab`, and documents a 12,288-byte freestanding AArch64 probe that was
  built and verified on the host only. No binaries, serial numbers or local paths.
- `h700-mmc-map-r2.tar.gz` — second read-only MMC map capture. Fixes the r1 collector
  defect where the platform-device sweep globbed `/sys/bus/platform/devices/*mmc*` and
  therefore matched nothing on this SoC (sunxi names the nodes `sdc0`/`sdc1`/`sdc2`);
  the r2 sweep uses `sdc*` and returns the live DT properties for all three controllers.
  Serial redacted; no CID/CSD identity fields, no block-device reads.

This branch is dedicated to field reports captured from the RG34XX handheld. Source code
lives on `main`; report archives are kept here because `build/` is gitignored there.

## Archives

| Archive | Captured | Source commit | Notes |
| --- | --- | --- | --- |
| `rg34xx-inventory-20261122-195053.zip` | 2026-11-22 19:50:53 +08:00 | `acc5e09` | Full read-only inventory incl. memory-map / debugfs markers |

SHA-256 of `rg34xx-inventory-20261122-195053.zip`:
`bbfa065e590ecd7f8394f9b251d15468d63ed6c93bbe192e4272a65a700ab5e2`

## Standalone extracts

- `debugfs-memory-reservations.txt`: verbatim copy of the same-named file inside the archive above
  (2,664 bytes, SHA-256 `6d256713f030dd6093c6f04770cf20a9561b93db76f288c01f2da179f1603ced`).

## 2026-11-22 20:47 capture — ION / TEE runtime focus

Directory `rg34xx-inventory-20261122-204742/` contains a sanitized subset from the read-only
inventory captured at `9b68302` (device ADB serial redacted here as well):

- `tee-ion-runtime.txt`
- `dmesg-hardware-lines.txt` — one occurrence of `usb_serial_number:` redacted; all other bytes verbatim
- `iomem.txt`
- `debugfs-memory-reservations.txt`
- `meminfo.txt`

Sanitization: only the device USB serial number was replaced with `[REDACTED]`; kernel virtual
addresses and physical MMIO ranges are kept because they are diagnostic, not unique identifiers.
No firmware images or environment partition were uploaded.

Key findings in `tee-ion-runtime.txt`:

- Kernel config exports `CONFIG_ION=y`, `CONFIG_TEE=y`, `CONFIG_OPTEE=y`;
  `/dev/ion` exists, `/dev/tee0` and `/dev/teepriv0` are `NOT_PRESENT`.
- ION heaps: `sys_user` (`ion_vin`, `ion_disp2`), `cma` (`nextui.elf`, 4,153,344 bytes), and an
  empty `secure` heap. No orphaned allocations.
- CMA area `0x77c00000..0x7bc00000`, 64 MiB total / 60,360 KiB free (~58 MiB), consistent with
  `CmaTotal`/`CmaFree` in `meminfo.txt`.
- Kernel log shows `optee: probing for conduit method from DT` followed by
  `optee: api uid mismatch` — TEE conduit probed but UID did not match.
- `NO_MATCHING_MODULES` for ION/TEE drivers (built into the kernel rather than loadable modules).

## Capture conditions

- Device: `[REDACTED]`, block device `/dev/mmcblk0`
- Collector: `scripts/collect-rg34xx-readonly-inventory.sh` (read-only; device side confirmed no writes)
- Environment: ADB platform-tools from Uotan Toolbox, Python 3.13.14

## Key results in the 20261122-195053 report

- Boot0 primary and secondary: `eGON.BT0` valid, header-declared size matched captured 128 KiB;
  SHA-256 `bf886995…319289f` / `4701a85e…fa9f5d` (unchanged across all captures)
- TOC1: SHA-256 `e6c20e3a…aa4604`, 4 items (u-boot, monitor, dtbo, dtb)
- `env-partition.bin`: full 16 MiB (16,777,216 bytes), CRC valid `0xe3b6e401`, 25 variables
- `live-fdt.json`: framebuffer `0x7bf24800..0x7c076000`, 720x480x32bpp, matches `disp_reserve`
- `debugfs-memory-reservations.txt`: CMA node `NOT_PRESENT: /sys/kernel/debug/cma`;
  memblock node exported and captured — node absence is not treated as "no reserved memory"
- `page-size.txt`: `NOT_PRESENT: getconf unavailable`; raw debugfs snapshot preserved without
  PFN-to-byte conversion

## 2026-11-23 19:22 capture — MMC map r2 (collector fix)

Archive `h700-mmc-map-r2.tar.gz` (8,313 bytes, SHA-256
`c921136be2439c20d456904607542199f874d82181bd5467977ee12067350adf`), captured at
`c6da96d` with `collect-h700-mmc-map-readonly.sh` (SHA-256 `5472cdb1…1152556`).
Contains `mmc-map.txt` (2,974 lines) and `README.txt`.

Collector defect fixed between r1 and r2: the platform-device sweep used
`/sys/bus/platform/devices/*mmc*`, which matches nothing here because sunxi registers these
controllers as `sdc0`/`sdc1`/`sdc2`. The whole "platform MMC devices and live DT properties"
section was therefore empty in r1. r2 globs `sdc*` and returns all three nodes. No other
collector logic changed, and the run remained read-only (no block-device access, no writes).

Live DT / topology (properties shown as raw big-endian bytes in the capture):

- `sdc0` @ `0x04020000` — `allwinner,sunxi-mmc-v4p1x`, `bus-width = 4`, no `cd-gpios`,
  no `vmmc-supply`/`vqmmc-supply`. This is the eMMC: `mmcblk0` = 30,375,936 sectors
  (~14.48 GiB), `removable = 0`, 7 partitions (p1 `special` 64 MiB, p2 `boot-resource` 32 MiB,
  p3 `env` 16 MiB, p4 `boot` 64 MiB, p5 `rootfs` 512 MiB, p6 `appfs` 128 MiB, p7 `UDISK` remainder).
- `sdc2` @ `0x04022000` — `allwinner,sunxi-mmc-v4p6x`, `bus-width = 4`, **has** `cd-gpios`
  (port 4, pin 6, active low) and `vqmmc-supply` (phandle `0x54`). No child device under
  `mmc1` in this capture.
- `sdc1` @ `0x04021000` — `allwinner,sunxi-mmc-v4p1x`, `bus-width = 4`, no card-detect
  property. No child device under `mmc2`; no `mmcblk1` exists.
- Only `sdc0` has a card: `mmc0:1234` under `mmc_host/mmc0`. `mmc1` (sdc2) and `mmc2`
  (sdc1) are registered hosts with no child device, i.e. no card present at capture time.
  sdc2 is the only controller with card detect, so it is the likely removable slot; the
  capture alone does not prove which physical connector each maps to.
- All three bind to `sunxi-mmc`; `resets`/`reset-names` are absent on all three, so reset
  control is not described in the live tree for these nodes.
- **DT alias numbering does not match kernel host numbering.** Aliases say `mmc0 =
  sdmmc@04020000` and `mmc2 = sdmmc@04022000` (`mmc1` alias is absent), but the kernel class
  exposes `mmc0 → sdc0`, `mmc1 → sdc2`, `mmc2 → sdc1`. Any mapping derived from DT aliases
  alone will be wrong for the two non-eMMC controllers.
- Boot cmdline corroborates the layout: `root=/dev/mmcblk0p5 rootwait`, `gpt=1`,
  `boot_type=1`, `secure_os_exist=0`, `disp_reserve=1382400,0x7bf24800`,
  `earlyprintk=sunxi-uart,0x05000000`, `console=ttyS0,115200`, `uboot_message=2018.05(05/26/2026-13:55:09)`.

Sanitization: the device serial number appeared twice on the `cmdline=` line and was replaced
with `[REDACTED]`; everything else is byte-identical to what the device produced. The
unsanitized archive is kept local only (SHA-256 `8856e75b2942763cd4799ae647feb6364ec85680d25d5b42f0a324eafff56503`),
so the two archives differ only by those two replacements. No CID/CSD identity fields and no
partition contents are included.
