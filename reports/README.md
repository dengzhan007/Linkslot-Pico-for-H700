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
- `rg34xx-a64-probe-qualification.md` — fourth round: completes the SRAM usage audit against the
  H616 map (A1 `0x20000`–`0x28000`, SRAM C `0x28000`–`0x58000`), showing the deployed 128 KiB
  capsule does **not** fit in A1 (its content reaches `0x3C753`), that the BROM itself writes
  SRAM C, that the first code to *choose* SRAM C is the dispatcher, and that BROM-time ownership
  of SRAM C stays UNKNOWN even though it is demonstrably usable. Rebuilds the baseline probe with
  a real assembler (`zig cc` 0.17.1 / clang 22.1.8 / lld) and compares all 144 instructions
  against the hand-encoded original: 118 byte-identical, 26 differences classified, one of them a
  value correction. Adds a framebuffer probe gated on eight independent facts from the live tree
  that paints 23,877 pixels inside `0x7BF24800`–`0x7C076000` with zero out-of-range writes, plus
  nine mutation tests proving every refusal writes nothing. States explicitly that painting is
  not the same claim as *being visible*, so a dark panel on hardware must be reported as
  inconclusive. No binaries, serial numbers or local paths.
- `rg34xx-a64-hardware-readiness.md` — fifth round, host-only. Audits the watchdog recovery chain
  from the live FDT (`allwinner,sun50i-wdt` at `0x030090a0`), the matching T507 driver semantics,
  capsule/SPL/BL31/A32-loader writes and the reset→BROM→capsule→STOCK state machine; the remaining
  BL31 reachability and RTC-retention gaps are marked UNKNOWN. Hardens `probe_fb.c` with the loader's
  FDT arena bounds, strict header/token/duplicate checks and passes 228 ASan/UBSan FDT cases.
  Records final hashes (probe64 `3cb5c440…`, framebuffer probe `d77c9ae6…`), all build commands and
  an explicit **NO-GO** conclusion with a shortest unblocking path. No binaries, device writes or
  payload execution.

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
