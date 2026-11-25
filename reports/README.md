# RG34XX read-only inventory reports

## Index

### 2026-11-24 — RG34XX active BOL4 payload identity resolved

- `rg34xx-boot-partition-deep-audit.md` addendum — decodes the active p4 +48 MiB BOL4 component using the verified extraction report and matches the 17,461,256-byte output SHA-256 to the primary Android image's gzip-decompressed kernel. Resolves the former BOL4-format / secondary-kernel identity unknown; does not change the firmware-pairing blocker or hardware NO-GO.

### 2026-11-24 — RG34XX active boot inputs extracted and checksummed

- `rg34xx-active-inputs/` — host-only extraction of the active Android v2 / BOL4 image at
  `0x03000000 + 0x829000` from the local 64 MiB p4 dump, with `extracted-manifest.json`,
  `verification.json` (21/21 cross-checks pass) and `SHA256SUMS.txt`. Confirms the image
  FNV-1a32 `0x04762a5f`, BOL4 `0x80474e` bytes whose header FNV `0xb4c47d39` matches the packed
  payload, a `0x10a7008`-byte `kernel.Image` that decompresses byte-identically, page size 2048 /
  header v2, an empty cpio `initrd`, and a 139,822-byte DTB. Binaries stay local; only the
  checksum manifest is published. `not_installable`, `not_hardware_validated`.

### 2026-11-23 — RG34XX H700 live MMC mapping (round 16)

- `linkslot-h700-live-mmc-map-2026-11-23-round16.md` — analyzed the r2 read-only capture and
  decoded the live controller/host relationships. The active boot/system SD is `sdc0@0x04020000` (`sunxi-mmc-v4p1x`, Linux `mmc_host0`): it owns
  `mmcblk0`, the running rootfs p5, and the actual ROM/user-data volume p7. Bootargs designate p7
  as UDISK, and the mount table confirms p7 is mounted at `/mnt/sdcard` with `/mnt/sdcard/Roms`
  and other user paths; p6 is `/data` (ext4) despite its conflicting sysfs `PARTNAME=UDISK`.
  This is the first controller to support for same-card ROM storage; Linux reports SDHC, not eMMC. `sdc2@0x04022000` (`sunxi-mmc-v4p6x`, Linux `mmc_host1`) is the optional
  second-card-slot candidate, with 4-bit bus, GPIO card detect, and `vqmmc-supply` referencing
  AXP2202 ALDO1. `sdc1@0x04021000` (`mmc_host2`) is the SDIO DDR50 device consistent with Wi-Fi.
  The raw `cd-gpios` cells contain port 4 / pin 22 followed by vendor-specific config cells
  (the trailing `0x06` is not the pin); sysfs p6/p7 names also differ from the cmdline aliases.
  Live pinctrl/CCU debugfs: sdc0 uses PF0–PF5 in default state with sdmmc0 module clock 100 MHz and bus/reset gates enabled; sdc1 uses PG0–PG5 with sdmmc1 module clock 200 MHz; sdc2 was in sleep state with its sdmmc2 clock/gates off. Raw pinconf values remain undecoded. The report corrects H616 MASK_DATA0 handling as a shared clock-update requirement for sdc0 too, not sdc2 only. This is topology/config evidence only: sdc2 was empty, no block read/write or LK card test was done.

### 2026-11-23 — RG34XX MMC map, second device capture (r2)

- `h700-mmc-map-r2.tar.gz` — read-only capture after fixing the collector's platform-device
  sweep from `*mmc*` to `sdc*`. Includes the three live nodes, MMC host and block-device
  symlinks, raw controller DT properties, regulator/debugfs summaries, and dmesg. SHA-256:
  `c921136be2439c20d456904607542199f874d82181bd5467977ee12067350adf`. The detailed
  interpretation and remaining unknowns are in round 16. Serial redacted; no CID/CSD identity
  fields and no block-device reads or writes.

### 2026-11-23 — FAT32/exFAT real-image regressions and RG34XX MMC reference (round 15)

- linkslot-h700-fat-exfat-mmc-2026-11-23-round15.md — full feasibility workflow [45136628791](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45135115298) passed on head `20d674c`. Host compile and real-image FAT32/exFAT regressions passed, including FAT32 VFAT long-name file create/replace/delete, empty-file handling, path rejection, and exFAT 4 KiB/128 KiB cluster images checked clean by fsck. Records FAT32 LFN-directory limitation, no atomic rename/flush barrier, AArch64 writer-lock branch, and H700 MMC0/MMC2 register/board wiring references. Artifact `14799704524`, 380,276,600 bytes, SHA-256 `49dda3cc8e12ecf1ac1d13a593fa397ffcf47e9d896f3323fe4b73faaace9929`. This is host-only; no SD/MMC driver or hardware test. A read-only MMC mapping collector (`scripts/collect-h700-mmc-map-readonly.sh`) is now available and syntax-checked for the next device inventory. [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).

### 2026-11-23 — H700 card path containment and FAT/exFAT provider audit (round 14)

- linkslot-h700-storage-path-2026-11-23-round14.md — full workflow 45130158723 passed on `4bb58d1`; standalone Rust path resolver passed 9/9 tests, enforcing exact mount boundaries, explicit relative bases and caller allowlists. Audited pinned GammaOS LK `emu/gba/fat_ro/fat_wr/fat_exfat` as a potential reusable MIT-licensed generic filesystem layer; its `sd_fat.c` glue is MTK-specific and must not be reused for H700. No SD driver or persistent filesystem is implemented. [Workflow](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45130158723); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — Rust static mGBA binding into slot-retro (round 13)

- linkslot-rust-static-mgba-binding-2026-11-23-round13.md — workflow 45128096825 passed. Added a Rust module mapping all 21 namespaced mGBA exports into the real slot-retro::Api type; host C stubs verify mgba_static_api() and the LibretroCore::open_static() init/deinit lifecycle. Static mGBA binding test suite: 63 passed, 0 failed. The next step is slot-level built-in core selection; this report does not claim real game execution or hardware boot. [Workflow 45128096825](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45128096825); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — Real mGBA static API symbol binding (round 12)

- `linkslot-mgba-static-api-link-2026-11-23-round12.md` — main workflow 45126961555 passed on commit `13c8239`. Strict AArch32 API-shaped table references all 21 real namespaced mGBA functions, is combined with H700 newlib adapter, and is linked with the actual mGBA whole archive; newlib-only final ELF32/ARM has no undefined symbols. Artifact ID `14796725457`, 96,415,151 bytes. This is a link probe, not a Rust-runtime or device execution test. [Workflow 45126961555](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45126961555); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — Linkslot static core API entry (round 11)

- `linkslot-static-core-api-2026-11-23-round11.md` — full workflow 45126045887 passed on commit `2ebe74b`. Dynamic mode: 61/61 tests; static-only mode: 62/62 tests. Adds `LibretroCore::open_static(Api, ...)`, makes `libloading` optional with dynamic still default, and confirms shared init/deinit lifecycle. Artifact ID `14795752171`, 96,408,534 bytes. Actual namespaced mGBA symbol binding is the next in-progress probe, not yet included as passed. [Workflow 45126045887](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45126045887); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — Linkslot VFS integration and complete mGBA/newlib feasibility chain (round 10)

- `linkslot-lk-rust-feasibility-2026-11-23-round10-vfs-static-newlib.md` — successful workflow 45125097115 on feature head `2ea0b1f`. The real Linkslot 1.0.36 `slot-retro::environment()` callback now passes its VFS v3 host regression (60/60 slot-retro tests); the actual mGBA VDir adapter passes against a fake VFS; Generic and arm-none-eabi/newlib VFS v3 AArch32 archives build; H700 newlib adapter tests pass; whole-archive mGBA + adapter links with newlib-only and newlib-nosys; A32 LK BL33 layout probe passes. Artifact ID `14794884889` (91,104,214 bytes). No SD/FAT backend, static Linkslot core integration, full no_std frontend, hardware boot or device test. [Workflow 45125097115](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45125097115); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — newlib whole-archive link and mGBA VDir regression (round 9)

- linkslot-lk-rust-feasibility-2026-11-23-round9-newlib-link-and-vdir.md — successful workflow 45115290950. The actual mGBA VDir adapter passes host fake-VFS tests for enumeration, type, EOF, rewind (close/reopen), nested directories, file open, delete/mkdir and path traversal rejection. A whole-archive newlib/libm/libgcc link leaves only 11 syscall hooks (_close/_exit/_fstat/_getpid/_gettimeofday/_isatty/_kill/_lseek/_read/_sbrk/_write); adding libnosys closes an ELF32/ARM link but only with dummy system calls and a WFI entry stub. No LK service adapter or device runtime. [Workflow run 45115290950](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45115290950); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — mGBA AArch32 arm-none-eabi/newlib VFS v3 archive (round 8)

- linkslot-lk-rust-feasibility-2026-11-23-round8-arm-none-eabi.md — successful workflow 45114286962. Builds the pinned real mGBA VFS v3 file+directory variant with arm-none-eabi GCC/newlib (not glibc): 2.86 MB archive, 94 objects, ELF32/ARM EABI5 objects, all 25 Libretro exports namespaced, 52 true external references after archive-local resolution, and no POSIX/stdio file-system fallback. This is archive compilation only; no newlib whole-archive final link, no LK integration, no H700 storage backend or device run. [Workflow run 45114286962](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45114286962); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — mGBA Libretro VFS v3 AArch32 static archive (round 7)

- `linkslot-lk-rust-feasibility-2026-11-23-round7-mgba-libretro-vfs3.md` — successful workflow 45113739310. Builds the pinned real mGBA AArch32 archive with a temporary Libretro VFS v3 file+directory backend; archive has 114 objects, all 25 Libretro exports namespaced, and 51 true external symbols after subtracting internal cross-object references. The old `vfs-file.c`/`vfs-dirent.c` objects and POSIX/stdio/host-config filesystem symbols are absent. The 51 remaining dependencies are heap/string, locale/parser/errno, RTC/time, math/EABI and console/logging; no final LK link or H700 runtime. [Workflow run 45113739310](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45113739310); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — Linkslot Libretro VFS v3 ABI host validation (round 6)

- `linkslot-lk-rust-feasibility-2026-11-23-round6-libretro-vfs-v3.md` — successful workflow run 45109949583. Adds a no_std Rust registry for the Libretro VFS v3 C interface, version negotiation via `RETRO_ENVIRONMENT_GET_VFS_INTERFACE`, and a fake in-memory backend. AArch32/ARM and AArch64 final ELFs link with no undefined symbols; the x86_64 harness exercises file operations, directory enumeration, metadata and path rejection with exit 0. The report explicitly limits this to a singleton fake backend; no SD/FAT or Linkslot integration yet. [Workflow run 45109949583](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45109949583); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — Real mGBA AArch32 static archive and Generic VFS comparison (round 5)

- linkslot-lk-rust-feasibility-2026-11-23-round5-mgba-static.md — successful run 45108202490. Builds the Linkslot-pinned real mGBA source as Linux and Generic AArch32 static Libretro archives. Both have 115 object members and all 25 Libretro exports namespaced; Generic reduces external undefined symbols from 81 to 67 by removing Linux-specific file-descriptor/mmap/glibc fortified calls. The report compares the remaining symbols with pinned GammaOS LK libc and records that Linkslot does not yet implement RETRO_ENVIRONMENT_GET_VFS_INTERFACE, while mGBA's Generic directory backend still uses POSIX dirent. No final LK link or device runtime. [Workflow run 45108202490](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45108202490); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — Linkslot 1.0.36 static Libretro core registry ABI (round 4)

- `linkslot-lk-rust-feasibility-2026-11-23-round4-static-core-abi.md` — successful GitHub Actions run 45107173305. Links a no_std Rust table matching Linkslot's current `slot-retro::ffi::Api` shape to two namespaced mock cores (`mgba_retro_*` / `gpsp_retro_*`). AArch32 and AArch64 ELF links have no undefined symbols; x86_64 harness actually exercises environment/video/audio/input callbacks, ROM load/run, save-state, memory, cheats and deinit. This establishes the static function-table pattern only; it does **not** compile mGBA/gpSP. [Workflow run 45107173305](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45107173305); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — Linkslot 1.0.36 static i18n and A32 BL33 layout (round 3)

- `linkslot-lk-rust-feasibility-2026-11-23-round3.md` — successful GitHub Actions run 45106548485. Adds `slot-i18n` no_std static-table proof: 6 domains, 1,763 unique translation keys and 1,763 entries per language; upstream 9 unit + 9 translations integration tests pass; generated no_std candidate 13 tests pass; AArch32/AArch64 checks pass. The same workflow reruns `slot-input`, `slot-model`, Rust/C ABI and A32 BL33 host layout probes. Records LK's uniprocessor scheduler, incorrect A53 compiler flag, missing A53 SMP init, and the Libretro `libloading` porting boundary. **No full LK firmware, device test, SD write or payload execution.** [Workflow run 45106548485](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45106548485); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).



### 2026-11-23 — Linkslot 1.0.36 no_std behavior tests and pinned LK heap ABI

- `linkslot-lk-rust-feasibility-2026-11-23-round2.md` — second CI round. Original `slot-input` / `slot-model` host suite: 58 passed; 47 original gesture tests also pass against the isolated `no_std` `slot-input` candidate; 4 new pure-domain tests pass against the `no_std` `slot-model` candidate. Builds Rust `GlobalAlloc` against the pinned GammaOS LK `heap.h` function signatures and links the C test stub for AArch32. Documents the actual heap alignment contract and the important finding that current LK critical sections do not prove cross-core mutual exclusion. This is **header/ABI compatibility only**, not the real LK heap implementation or a hardware port. [Workflow run 45103281417](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45103281417); see also [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).

### 2026-11-23 — Linkslot 1.0.36 Rust no_std / LK ABI feasibility

- `linkslot-lk-rust-feasibility-2026-11-23.md` — GitHub Actions host-only cross-build. Verifies temporary `slot-input` and `slot-model` no_std+alloc candidates on AArch32 and AArch64, Rust staticlib ↔ freestanding C shim linking, final ELF architecture and undefined-symbol closure. Includes the AArch32 EHABI metadata-only marker caveat and explicitly excludes real LK heap/handoff/device execution. [Workflow run 45102606482](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45102606482); source/test branch is PR #16.


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

## 2026-11-23 19:22 capture — MMC map r2 (collector fix)

Archive `h700-mmc-map-r2.tar.gz` (8,313 bytes, SHA-256
`c921136be2439c20d456904607542199f874d82181bd5467977ee12067350adf`), captured at `c6da96d` with
`collect-h700-mmc-map-readonly.sh` (SHA-256 `5472cdb1…1152556`). Contains `mmc-map.txt`
(2,974 lines) and `README.txt`.

Collector defect fixed between r1 and r2: the platform-device sweep used
`/sys/bus/platform/devices/*mmc*`, which matches nothing here because sunxi registers these
controllers as `sdc0`/`sdc1`/`sdc2`. The whole "platform MMC devices and live DT properties"
section was therefore empty in r1. r2 globs `sdc*` and returns all three nodes. No other
collector logic changed, and the run stayed read-only (no block-device access, no writes).

Live DT / topology (properties appear as raw big-endian bytes in the capture):

- `sdc0` @ `0x04020000` — `allwinner,sunxi-mmc-v4p1x`, `bus-width = 4`, no `cd-gpios`, no
  `vmmc-supply`/`vqmmc-supply`. This is the eMMC: `mmcblk0` = 30,375,936 sectors (~14.48 GiB),
  `removable = 0`, 7 partitions (p1 `special` 64 MiB, p2 `boot-resource` 32 MiB, p3 `env` 16 MiB,
  p4 `boot` 64 MiB, p5 `rootfs` 512 MiB, p6 `appfs` 128 MiB, p7 `UDISK` remainder).
- `sdc2` @ `0x04022000` — `allwinner,sunxi-mmc-v4p6x`, `bus-width = 4`, has `cd-gpios`
  (port 4, pin 6, active low) and `vqmmc-supply` (phandle `0x54`).
- `sdc1` @ `0x04021000` — `allwinner,sunxi-mmc-v4p1x`, `bus-width = 4`, no card-detect property.
- Only `sdc0` has a card: `mmc0:1234` under `mmc_host/mmc0`. `mmc1` (sdc2) and `mmc2` (sdc1) are
  registered hosts with no child device, i.e. no card present at capture time; there is no
  `mmcblk1`. sdc2 is the only controller with card detect, so it is the likely removable slot,
  but this capture alone does not prove which physical connector each controller serves.
- All three bind to `sunxi-mmc`; `resets`/`reset-names` are absent on all three nodes, so reset
  control is not described in the live tree.
- DT alias numbering does not match kernel host numbering: aliases say `mmc0 = sdmmc@04020000`
  and `mmc2 = sdmmc@04022000` (`mmc1` alias absent), while the kernel class exposes
  `mmc0 → sdc0`, `mmc1 → sdc2`, `mmc2 → sdc1`. A mapping inferred from DT aliases alone
  is wrong for the two non-eMMC controllers.
- Boot cmdline corroborates the layout: `root=/dev/mmcblk0p5 rootwait`, `gpt=1`, `boot_type=1`,
  `secure_os_exist=0`, `disp_reserve=1382400,0x7bf24800`, `earlyprintk=sunxi-uart,0x05000000`,
  `console=ttyS0,115200`, `uboot_message=2018.05(05/26/2026-13:55:09)`.

Sanitization: the device serial number appeared twice on the `cmdline=` line and was replaced
with `[REDACTED]`; every other byte is exactly what the device produced. The unsanitized archive
is kept local only (SHA-256 `8856e75b2942763cd4799ae647feb6364ec85680d25d5b42f0a324eafff56503`),
so the two archives differ only by those two replacements. No CID/CSD identity fields and no
partition contents are included. No SD/MMC driver work and no hardware test are implied.
