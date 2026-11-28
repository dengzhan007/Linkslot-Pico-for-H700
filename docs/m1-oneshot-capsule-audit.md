# M1 one-shot capsule / SPL safety audit

## 2026-11-28 M1 GPT partition discovery and bounded SRAM scratch

- New source finding: U-Boot v2026.10's `SPL_EFI_PARTITION` defaults to disabled for `ARCH_SUNXI`; the RG35XX H700 defconfig does not explicitly enable it. Therefore SPL code must not assume `part_get_info_by_name()` has a GPT driver behind it. The source-layout audit now pins this Kconfig boundary.
- The captured RG34XX non-SP GPT report `reports/rg34xx-inventory-20261127-uart/gpt-fdisk.txt` confirms 512-byte sectors and exactly eight GPT entry slots. GPT slot 4 is uniquely named `boot`, first LBA 303104, last LBA 434175 (131072 sectors). This independently matches the block inventory.
- Added `platform/h700/boot/m1_gpt_spl_adapter.{h,c}`. It adapts U-Boot `blk_dread()` to the existing read-only H700 GPT parser, validates the primary protective MBR/header/entry CRCs and partition bounds, then resolves the exact `boot` name. Duplicate names, bad CRCs, invalid geometry, I/O errors, or more than eight GPT slots fail closed. It does not call U-Boot's partition subsystem or any write API.
- The integrated `h700_m1_spl_preflight_gpt()` path now owns a named 4096-byte, 64-byte-aligned static `m1_package_scratch` buffer plus the bounded eight-entry GPT table. It resolves p4 first, then invokes the existing p4+40 MiB firmware preflight using 8-sector reads.
- Added `tests/test_m1_gpt_spl_adapter.c` and `scripts/test-m1-gpt-spl-adapter.py`. The fake-card regression verifies the captured p4 geometry, invalid GPT CRC, duplicate `boot` label, wrong sector size, read failure, and the complete 6-sector GPT + 71-read firmware path (77 reads total), with zero writes. The linked host test checks the 4096-byte scratch symbol via `nm -S` and rejects any `blk_dwrite` symbol.
- Feasibility [#995](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46104833857) has passed the M1 SRAM source-layout audit, capsule selection model, original SPL block adapter, new GPT-to-preflight adapter, and fake-card/AArch32 package tests. The overall workflow was still running in later Linkslot build steps at the time of this update; do not label the full run complete yet.
- **Evidence boundary:** the 4 KiB symbol is linked in the host adapter test, not yet in the actual U-Boot SPL ELF. No final U-Boot SPL map, BSS headroom, or transitive linked-ELF audit exists yet. The eight-slot limit is target-specific and deliberately fails closed on a different GPT geometry. No physical MMC reads, SRAM access, DRAM access, MMIO/SMC, TWI, media writes, firmware flash, or payload execution occurred.

**Next gate:** build a separate M1 preflight-only U-Boot SPL overlay in CI, link this adapter into the actual H616 SPL, and inspect `__bss_start`, `__bss_end`, `m1_package_scratch`, and `m1_boot_partitions` in the final map/ELF. Keep the generated SPL non-installable and do not modify the ordinary or raw-capsule-v2 path.


## 2026-11-28 M1 SPL SRAM / separate-BSS source-layout audit

- Added `scripts/audit-m1-spl-sram-layout.py` and a feasibility CI step. It checks the public U-Boot v2026.10 tag commit `5508406582f6f7120dce0c4b3059819a41788db2`, the pinned BaseOS builder commit `cbe253a43f46cf551cc9a6c79a84015bde3686cf`, and the builder's separate U-Boot archive SHA-256 pin `f943a88389c8fd70a6b0d061a88267465e143c7ca6ea2b75837ab20f5b60c739`. The Git tag commit and BaseOS archive digest are intentionally recorded as separate provenance pins; this audit does not claim byte identity between them.
- U-Boot H616 Kconfig selects ARM64 and H6-generation support. ARM64 implies `SPL_SEPARATE_BSS`; the sunxi SPL linker places `.bss` in a distinct `.sdram` region. BaseOS sets `SPL_BSS_START_ADDR=0x30000` and `SPL_BSS_MAX_SIZE=0x4000`, giving a configured BSS window `[0x30000,0x34000)`.
- The H616 upstream device tree describes SRAM C as `[0x28000,0x58000)`. The configured 16 KiB BSS window is inside SRAM C. The initial SPL stack pointer defaults to `0x52a00`, leaving `0x1ea00` (125,440 bytes) between the BSS ceiling and the stack pointer. The separate initial RAM window is `[0x20000,0x28000)`; the 4 KiB streaming scratch is not being assigned to that 32 KiB initial-RAM/A1 window.
- BaseOS explicitly clears `__bss_start..__bss_end` before `spl_early_init()`, then inserts `baseos_early_mmc()` before `sunxi_board_init()`. Thus a future static BSS scratch reservation would be initialized before the early-MMC hook.
- **Important limit:** this proves source-level address geometry and a 16 KiB linker ceiling, not actual BSS headroom. No M1 scratch symbol has been linked into SPL, no SPL map/ELF has been inspected, and stack high-water usage is not measured. The report deliberately sets `linked_map_verified=false`, `scratch_symbol_integrated=false`, and `buffer_reservation_approved=false`. A final M1-enabled SPL build must prove actual `__bss_end`, symbol extent/alignment, and non-overlap before this reservation can be accepted.
- Full feasibility [#978](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46101368294) and H700 boot audit [#1641](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46101368298) completed successfully on the prior source HEAD. The new source-layout check runs on the current change and is not yet counted as passed until its own CI completes.
- No SPL binary was built in this audit; no device SRAM/MMC/DRAM access, MMIO/SMC, TWI, media write, firmware flash, or payload execution occurred. `hardware_applied=0`; G2/G3/M1-S1 and one-shot recovery remain open.

**Next gate:** build the pinned M1-only SPL overlay in CI, link a named 4 KiB BSS scratch symbol, and inspect the final map/ELF to measure actual BSS use and audit the complete transitive preflight call path. Do not connect it to the standard or raw-capsule-v2 flow.


## 2026-11-28 M1 runtime firmware provenance and DTB-tail validation

The earlier apparent size discrepancy is resolved by distinguishing the **source
TOC1 container** from the **runtime p4+40 MiB firmware stream**:

- The captured source TOC1 package is 1,310,720 bytes
  (SHA-256 `e6c20e3a54c51b9336c798e3a4fa1ca6c45b28c6f70456e2db9c252aa4ba4604`).
  Its item directory contains U-Boot, monitor, DTBO and DTB. This is not the
  byte stream read at p4+40 MiB.
- Pinned BaseOS `native/build.py` extracts monitor, DTBO and DTB, appends the
  generated 32 KiB A32 loader, and pads each component to 512-byte boundaries.
  The SPL consumer reads this generated `firmware.bin` at p4+40 MiB and checks
  the build-generated `BASEOS_FIRMWARE_BYTES` / `BASEOS_FIRMWARE_FNV`.
- The captured deployed stream manifest
  (`reports/rg34xx-native-firmware/firmware-manifest.json`) confirms 287,232
  bytes / `0x46200` / 561 sectors, SHA-256
  `dc9dc3ae66b30244911e7054ca66136991626cabb0d59c796c4b253a126f28e9`,
  FNV-1a `0xaf0018cb`, and extents monitor `0x00000`, DTBO `0x1a400`,
  DTB `0x1ae00`, loader `0x3e200`. Thus the existing 0x46200 plan is the
  runtime firmware stream, not the 1.31 MiB TOC1 source container.
- The DTB FDT totalsize is 139,822 bytes (`0x2222e`) inside a 144,384-byte
  (`0x23400`) copied component. Its 4,562-byte tail must be zero. The
  preflight now validates monitor, DTBO **and DTB** padding in both full-buffer
  and 4 KiB streaming paths. Tests deliberately inject a nonzero DTB-tail byte
  while recomputing FNV, proving rejection is due to the padding contract.
- The old fixed `H700_M1_PACKAGE_FNV1A` macro was removed. FNV is generated
  from the actual matching firmware build and must be passed from that
  generation's generated metadata; the captured `0xaf0018cb` is a baseline
  fingerprint, not a constant for a new Linkslot loader. FNV is regression
  integrity, not authentication; generation matching and cryptographic
  provenance remain separate requirements.

Added `tests/fixtures/m1-rg34xx-toc1-provenance.json` and
`scripts/test-m1-baseos-firmware-contract.py`. The new CI step checks the
pinned BaseOS source contract, source TOC1 item hashes/sizes, 512-byte packing,
p4+40 MiB generated metadata flow, native SPL consumer bounds/copies, and the
expected 0x46200 extent arithmetic. It is a source/layout audit only; it does
not build or execute BaseOS SPL, read a physical card, copy to DRAM, or qualify
a capsule.

**Next gate:** pass the source/layout test, then make the M1 adapter consume the
same generated `BASEOS_FIRMWARE_BYTES` and `BASEOS_FIRMWARE_FNV` from one
immutable build manifest. Do not freeze the captured FNV into the new loader
generation. SPL integration still waits for proven SRAM scratch placement and
the full call-graph / linked-ELF side-effect audit.


**Status:** design constraint and source audit only. No M1 capsule, M1 SPL, or paired image has been built. No SD write, device MMIO, or payload execution was performed.

**Target:** RG34XX non-SP / H700, replacing only the current AArch32 SVC BL33 loader at 0x4a000000 on a separately prepared TF1 test card.

**Source pin:** BaseOS-MultiFrontend cbe253a43f46cf551cc9a6c79a84015bde3686cf.

## 2026-11-28 M1 read-only firmware-package preflight (host-only)

A first reusable SPL-side package contract now exists in
`platform/h700/boot/m1_firmware_readonly.{h,c}`. It is deliberately a
read-only planning/validation module, not yet connected to the pinned BaseOS
SPL or to physical addresses.

- The plan fixes the captured active firmware-package geometry: p4 + 40 MiB,
  0x46200 bytes / 561 sectors; monitor at offset 0x00000 (payload 0x1a2d0,
  padded 0x1a400, destination 0x48000000); DTBO at 0x1a400 (0x920 / 0xa00,
  destination 0x4a100000); DTB at 0x1ae00 (FDT totalsize 0x2222e, copied/padded component 0x23400, destination 0x4a200000);
  A32 BL33 at 0x3e200 (0x8000, destination 0x4a000000).
- The planner checks p4 boundary arithmetic and staging capacity. The reader
  interface exposes only a `read_blocks` callback; the validation path issues
  bounded 8-sector streaming reads, checks the build-supplied FNV-1a, validates
  monitor/DTBO/DTB zero padding, and rejects changed extent/destination metadata.
- Host fake-card tests cover successful read, exact LBA/count, read failure,
  partition-end and integer-overflow rejection, undersized staging, bad
  fingerprint, nonzero padding, and altered destination metadata. The same
  production C module is compiled and linked as freestanding Cortex-A53
  AArch32 ARM-state ELF32 with no undefined runtime symbols.
- Full feasibility [#950](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46097505586)
  completed successfully: 87/87 recorded steps, zero failures and zero skips.
  Artifact [15243703880](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46097505586/artifacts/15243703880)
  contains the build outputs and report. H700 AArch32/AArch64 compile check [#670](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46097466062)
  also completed successfully.
- The test image is synthetic. The captured reference package fingerprint
  0xaf0018cb is recorded for comparison, but this module does not yet pin the
  device package to a cryptographic digest or establish that a newly built
  generation matches the captured package. FNV-1a is an integrity regression
  field, not authentication.

**Boundary:** this is host/freestanding-compile evidence only. It does not
connect the reader to BaseOS `blk_dread`, perform actual CMD17 reads, copy any
extent to DRAM, establish memory ownership, or execute BL31/BL33. It has no
write callback, but that does not prove the future SPL call graph is write-free.
No device I/O, MMIO, SD write, firmware build, capsule build, or payload
execution occurred; `hardware_applied=0`.

**Next implementation cut:** prepare a separate patch overlay for the pinned
BaseOS `BASEOS_M1_ONESHOT` early-MMC branch, wiring only the read-only
package preflight behind a distinct build define. Before using it, add fake
block-device integration tests against the actual SPL adapter, audit the
transitive board-init/DRAM/MMC dependencies, and inspect the final linked
AArch64 SPL ELF for block-write, RTC, watchdog, PMIC/TWI, and unapproved
MMIO sites. Keep the standard and raw-capsule-v2 paths unchanged.
 
## 1. Decision

## 2026-11-28 RG34XX partition geometry cross-check

The synthetic test geometry has been replaced with the captured RG34XX non-SP
block layout from the `reports` branch inventory
(`reports/rg34xx-inventory-20261127-uart/device-and-block-layout.txt`):

- `mmcblk0p4` is the boot partition, first LBA **303104**, length **131072
  sectors** (512-byte sectors).
- The M1 package offset is p4 + 40 MiB = **81920 sectors**, so its expected
  first LBA is **385024**. The 561-sector package ends at LBA 385584 inclusive,
  within p4's end-exclusive LBA 434176.
- Both host fake-card tests now use this captured p4 start/length and retain a
  boundary-negative case one sector too short.

This verifies arithmetic against the captured partition geometry; it does not
prove that every future TF image has the same GPT or that the bytes currently
at p4+40 MiB match the expected package. The actual package's cryptographic
identity and runtime p4 discovery remain separate requirements.
 

## 2026-11-28 M1 SPL read-only adapter and bounded streaming

- Added `platform/h700/boot/m1_firmware_spl_adapter.c`: a narrow adapter to U-Boot's `blk_dread()` API. It contains no `blk_dwrite()` call or write-capable callback.
- Important SRAM correction: the complete firmware package is 0x46200 bytes (287 KiB), larger than the H700 SRAM A1 window (32 KiB). The initial full-buffer approach therefore must not be used as an SPL SRAM staging design. The SPL adapter now streams the package through an 8-sector / 4 KiB scratch buffer, reading 561 sectors in 71 bounded calls, updating FNV-1a incrementally and checking all three component padding ranges without retaining the full package in SRAM.
- The adapter host fixture compiles the actual adapter C source against a minimal fake U-Boot `blk.h`; it verifies the 71-read sequence, maximum 8 sectors per read, partition-end rejection, read-error handling, insufficient scratch rejection, and zero writes. `scripts/test-m1-spl-adapter.py` also checks the linked test binary for absence of a `blk_dwrite` dependency.
- The streaming routine is a preflight only. It does not load/copy the validated extents into DRAM or execute them. The current test uses a synthetic package and a host shim, not the pinned BaseOS SPL or a physical MMC controller.
- CI feasibility [#959](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46099372148) was cancelled by branch concurrency after 38 steps; both the new adapter test and package fake-card/AArch32 test had completed successfully before cancellation. Replacement feasibility [#960](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46099471409) is running against the documentation HEAD. AArch32/AArch64 compile check [#675](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46099368676) completed successfully, including the storage host regression job. H700 boot audit [#1627](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46099465183) and [#1628](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46099468041) both passed.
- Hardware boundary remains unchanged: no physical SPL integration, no DRAM writes/copies, no MMIO/SMC, no TWI, no TF/Boot0/TOC1 writes, no firmware flash or payload execution; `hardware_applied=0`.

**Next gate:** port this adapter as a separate `BASEOS_M1_ONESHOT` overlay against the pinned U-Boot/BaseOS source, but do not invoke it until the exact p4 extent and a safe 4 KiB SRAM buffer placement are proven from the linked SPL map. After that, add host tests using the actual U-Boot MMC declarations/adapter and audit all transitive init calls plus final linked SPL ELF. DRAM destination writes remain blocked on G2 memory ownership approval.
 

Neither existing BaseOS SPL path can be reused unchanged as an M1 one-shot path.

- The ordinary non-persistent path writes a diagnostic result sector, copies the 64 KiB stock Boot0 to a mirror sector, and programs the hardware watchdog.
- The raw-capsule v2 path disables SPL MMC writes, but it is not stateless: the outer capsule commits RTC GP14/GP15 and arms the watchdog; the SPL validates that state before proceeding. The native handoff also performs PMIC/CCU/display operations and watchdog reload.
- The existing native/capsule/build-diagnostic.py only tests the outer commit protocol with an old, non-adopting inner. It does not launch M1 LK and does not remove the RTC/watchdog dependency.

**Required direction:** a distinct M1-only one-shot outer capsule, SPL early path, and native handoff. It must be purpose-built and statically audited; removing a single attempt_commit() call from v2 is insufficient.

## 2. Exact source findings

All findings below refer to the pinned BaseOS source tree above; they are source-level observations, not proof that a particular code path has executed on the RG34XX non-SP device.

### 2.1 Ordinary SPL early-MMC path has durable media writes

In experiments/h700-boot-speed/spl/early-mmc.c:

- RESULT_SECTOR = 0x56000, SOURCE_SECTOR = 0x56001, MIRROR_SECTOR = 0x56200.
- baseos_checkpoint() writes one sector to RESULT_SECTOR with blk_dwrite(), then reads it back.
- baseos_control_mmc() reads a 64 KiB stock Boot0 from SOURCE_SECTOR, writes it to LBA 16 or MIRROR_SECTOR (default branch), then reads every sector back.
- The non-persistent tail writes watchdog configuration/mode/control at 0x030090b4, 0x030090b8, and 0x030090b0, then waits.

Therefore this path is rejected for M1 even when the test card is disposable: it contains hidden data-sector writes and watchdog programming unrelated to the minimum BL33 proof.

### 2.2 Raw-capsule v2 is not a no-state one-shot

In the same file, BASEOS_RAW_CAPSULE selects a separate implementation:

- baseos_raw_adopt() reads RTC GP14/GP15 at 0x07000138 / 0x0700013c and watchdog CFG/MODE at 0x030090b4 / 0x030090b8; it fails into WFI unless the exact outer-owned pair and watchdog state are present.
- Its checkpoint routine only updates an SRAM structure; it does not call blk_dwrite().
- The SPL builder disables SPL_MMC_WRITE for --raw-capsule.

This avoids the ordinary early-MMC scratch writes, but the protocol is explicitly dependent on RTC/watchdog state owned by the outer capsule. It is not the M1 no-RTC/no-watchdog contract.

### 2.3 The v2 outer capsule itself writes RTC and watchdog

In experiments/h700-boot-speed/native/capsule/capsule.c:

- The outer reads retention state at 0x070001f8 and reads the attempt pair at GP14/GP15.
- arm_watchdog() writes watchdog CFG/MODE/CTRL at 0x030090b4, 0x030090b8, and 0x030090b0.
- The normal path commits the attempt pair through attempt_commit(), which writes the GP14/GP15 protocol words.
- The outer chooses stock or native according to retention and the exact attempt pair. This is a persistent state machine, not a simple fixed dispatch.

In entry.S, refusal ends in an infinite WFI loop. The code does not provide a general software recovery path independent of the watchdog/outer protocol.

### 2.4 Native handoff has further hardware side effects

In experiments/h700-boot-speed/spl/native-boot.c:

- baseos_native_bootreason() reads PMIC bus registers; for one unknown-boot-reason case it writes PMIC scratch register 0xf0.
- baseos_vendor_pll_power() performs read-modify-write operations on nine CCU offsets.
- panel_start() / panel_advance() modify PIO PI15/PI14 and, for a strapped panel, PE6-related fields. They are display startup behavior, not required for an LK proof-of-life.
- The raw-capsule path revalidates RTC/watchdog adoption before the reset handoff.
- The final handoff reloads the watchdog at 0x030090b0, then programs RVBAR and requests an EL3 reset through RMR_EL3.
- The native boot path also reads the full BOL4 image at p4 + 48 MiB and validates its FNV. That image/kernel work is not intrinsically required to enter an independent A32 BL33 diagnostic.

The existing implementation therefore contains multiple functions that an M1-specific build must either omit or replace with reviewed minimal equivalents. It is not enough to compile the current native SPL with a different outer capsule.

### 2.5 Current native SPL entry and its PMIC/DRAM baseline (2026-11-27)

The native boot chain must be described accurately. The deployed 128 KiB
Boot0-primary image is itself the BaseOS A32 outer capsule executed by the
Allwinner Boot ROM. It embeds a 64 KiB stock eGON.BT0 Boot0 as a fallback and
a 48 KiB AArch64 native SPL. On the current native route, the outer capsule
selects and copies the AArch64 SPL; the embedded stock Boot0 is **not** an
earlier DRAM-training stage on that route. The stock Boot0 runs only when the
outer selector chooses the fallback branch.

The pinned BaseOS build script uses U-Boot v2026.10 and invokes
`anbernic_rg35xx_h700_defconfig` for the native SPL. That defconfig enables
LPDDR4 at 672 MHz, `CONFIG_AXP717_POWER`, SPL I2C / legacy I2C, and these
voltage targets:

| Config | Value |
|---|---:|
| `CONFIG_AXP_DCDC2_VOLT` | 940 mV |
| `CONFIG_AXP_DCDC3_VOLT` | 1100 mV |
| `CONFIG_DRAM_CLK` | 672 MHz |
| DRAM type | H616 LPDDR4 |

The U-Boot v2026.10 `arch/arm/mach-sunxi/board.c::board_init_f()` prelude
performs SRAM, TZPC, timer, clock, UART GPIO/console, and (under this
defconfig) I2C initialization. BaseOS inserts its early-MMC hook before the
normal trailing `sunxi_board_init()` call. The custom
`spl/native-boot.c::baseos_vendor_handoff()` then sets
`baseos_dram_power=1` and calls `sunxi_board_init()` before accessing the
firmware package in DRAM.

In the pinned U-Boot source, `board/sunxi/board.c::sunxi_board_init()`:

1. calls `axp_init()` for AXP717;
2. calls `axp_set_dcdc2(940)` and `axp_set_dcdc3(1100)` because the above
   defconfig values are set;
3. runs `sunxi_dram_init()` and publishes the selected DRAM size; and
4. may call `clock_set_pll1()` when the PMIC voltage setup succeeds.

The pinned `drivers/power/axp_spl.c` implements the DCDC setters using
`pmic_bus_write()` followed by `pmic_bus_setbits()`. These are real PMIC
bus transactions, not just metadata or host-side operations.

This means the currently deployed native SPL has a **board-init baseline
that includes AXP717/TWI, DCDC voltage writes, DRAM training and CPU PLL
setup**. It is a source-matched part of the existing native boot design, but
it is not a "no-PMIC/no-TWI" design. A M1 one-shot must choose and declare
one of two profiles:

- **Reuse the exact deployed native-init baseline (lower-delta candidate):**
  preserve the pinned board-init sequence and its exact DCDC2/DCDC3 values
  unchanged; do not add PMIC scratch writes, new rail settings, new clock
  programming, or other peripheral initialization. The build manifest must
  report PMIC/TWI access as present and bind the profile/source pin.
- **Strict no-PMIC/no-TWI profile:** do not call `sunxi_board_init()` or
  the legacy board-init prelude blindly. This profile cannot simply assume
  DRAM is already trained, because the current native capsule route selects
  the A64 SPL directly from the outer capsule. It needs separate evidence
  that usable DRAM is already established, or a separately designed and
  qualified DRAM bring-up path.

Until the project selects and reviews a profile, do not claim
`pmic_access=false` / `twi_access=false` for an M1 artifact. The A32 BL33
diagnostic itself still does not need PMIC/TWI, but the preceding A64 SPL may.

## 3. M1 one-shot contract

The intended one-shot design is a separate build mode with the following fail-closed requirements.

### Outer Boot0 capsule

1. Use a distinct source directory/build target and manifest identity; do not reuse v2's RTC protocol or its diagnostic outer-control builder.
2. Preserve and validate the exact Boot0 source and embedded M1 SPL using pinned hashes, fixed sizes, eGON header/checksum validation, and an explicit copy/entry layout.
3. Dispatch to the M1 SPL deterministically. Do not read or write RTC GP_DATA, retention registers, watchdog registers, PMIC state, or SD/eMMC data sectors.
4. Validate the embedded stock Boot0 and M1 SPL before dispatch. If the M1 SPL header/hash/entry is invalid but the embedded stock Boot0 is valid, select the embedded stock Boot0 using the existing source-select/copy-stub pattern; if the outer capsule or both inner candidates are invalid, stop in a documented fail-stop state.
5. This is a pre-entry validation fallback only. Once the M1 SPL has started, there is no automatic runtime rollback; use the separately reviewed manual card-swap recovery.
6. Do not silently inherit v2 attempt-pair, cookie, pending, or watchdog assumptions.

### AArch64 SPL

1. Add a dedicated BASEOS_M1_ONESHOT path. It must not call the normal baseos_control_mmc(), the persistent path, or baseos_raw_adopt().
2. Do not call any block-device write routine. In particular, there must be no writes to 0x56000, 0x56200, LBA 16, Boot0/TOC1, or any hidden result/journal sector.
3. Do not read or write RTC GP_DATA/retention state or watchdog CFG/MODE/CTRL.
4. Do not add PMIC/RSB/TWI, voltage, or CPU-PLL operations beyond the selected and explicitly declared initialization profile in section 2.5. In particular, exclude `baseos_native_bootreason()` (including PMIC scratch `0xf0`), `baseos_vendor_pll_power()` (the additional nine CCU writes), and panel/PIO display startup. If strict no-PMIC/no-TWI is selected, the implementation must not call `sunxi_board_init()` and must independently prove its DRAM prerequisite; if the exact deployed baseline is reused, preserve its AXP717/DRAM/PLL settings without modification and declare PMIC/TWI access.
5. Read only the exact, pinned p4 + 40 MiB firmware package. Validate the package hash/FNV and every extent before copying the vendor monitor, DTBO, DTB, and M1 A32 BL33 payload to their audited destinations.
6. Do not load the p4 + 48 MiB BOL4 Linux image, perform kernel decompression, prepare a Linux bootargs/FDT handoff, or issue the Linux-start SMC.
7. Preserve only the vendor BL31 input fields proven necessary for its A32 SVC BL33 entry: selected DRAM parameters, the exact board bytes and DTB pointer. Their provenance and offsets must be tied to the matching package and Boot0.
8. Any required RVBAR/RMR transition must be separately reviewed as part of the handoff ABI; it is not permission to use unrelated PMIC, display, RTC, watchdog, or media-write functions.

### M1 A32 BL33 payload

- Keep the raw payload within the active 32 KiB loader extent and use the exact currently deployed vendor header template in the native package builder.
- Entry/vector, BSS, early stack, page table and _end must be checked from the actual linked ELF/map. The existing host stack/page-table non-overlap test is necessary but does not approve the physical BL33 region.
- The PI12 MMIO probe remains disabled until M1-S1 proves APB1/PIO handoff access, the SPC/fuse condition, the pin mux and polarity, PI11 preservation, and the abort/fail-stop path.
- No UART writes, framebuffer, GIC/timer, heap, DMA, SMP, PMIC/TWI, or application DRAM arena in the first proof payload.

## 4. Concrete implementation cut points in the pinned BaseOS tree

The implementation should be a separate build mode and separate source files;
do not edit the meaning of `BASEOS_RAW_CAPSULE` in place.

| Layer | Existing source cut point | M1-specific replacement |
|---|---|---|
| Outer A32 capsule | `native/capsule/entry.S`, `capsule.c`, `build.py` | Keep the bounded A32 context-save/copy-stub path only after independent review; replace `capsule_main()` with static Boot0/M1-SPL header+hash selection. No RTC, retention, watchdog, persistent attempt state or write-side fallback. |
| SPL early entry | `spl/early-mmc.c` `BASEOS_RAW_CAPSULE` branch | Add a distinct `BASEOS_M1_ONESHOT` branch. It may initialise the already-selected TF controller and perform block reads, but has no `baseos_raw_adopt()`, no `blk_dwrite()`, no result/journal sectors and no RTC/WDT reads or writes. |
| Native handoff | `spl/native-boot.c::baseos_vendor_handoff()` | Add a dedicated M1 handoff rather than branching through the production Linux-loader routine. Load and validate only the required native firmware extents; prepare the monitor's required DRAM/board/DTB fields; enter BL31's A32 SVC BL33 path. |
| Build/package | `spl/build-early-mmc.py`, `native/build.py`, capsule builder | Generate a generation-bound M1 manifest and matched SPL/firmware/capsule. Keep the original 32 KiB A32 loader extent and zero header-reserved input contract. No release/upgrade artifact reuse. |

### Explicit functions to exclude from the M1 native handoff unless separately justified

- `baseos_native_bootreason()`: reads PMIC bus state and may write PMIC scratch `0xf0`; boot-reason metadata is unnecessary for a proof-of-life payload.
- `baseos_vendor_pll_power()`: writes nine CCU PLL/gate registers. Its necessity for a minimal BL31→A32 diagnostic is unproven; do not inherit it by convenience.
- `panel_start()`, `panel_advance()`, `panel_publish()`, `load_boot_logo()`: display PIO and logo handling are unrelated to entering LK; exclude them.
- The p4 + 48 MiB BOL4 image read, FNV check, kernel decompression, Linux bootargs/FDT mutation and Linux-start SMC: none is required for an independent A32 BL33.
- Watchdog adoption/reload, RTC markers and diagnostic trace writes: all excluded.

### Dependencies that still need their own call-graph audit

- `sunxi_board_init()` / DRAM training: determine whether this is needed because the selected M1 SPL, unlike the embedded stock Boot0, begins before DRAM is established. If it is required, enumerate its clock, pinmux, PMIC/RSB/TWI, DRAM and MMIO writes; no hidden call is grandfathered in.
- MMC setup: controller/PIO/CCU register operations required to initialise the SD host are distinct from writing SD data sectors. The M1 source may perform the former only after a scoped allowlist; every block write command/data-sector write remains forbidden.
- `baseos_native_cache_on()` / `baseos_native_cache_off()`: retain only the cache/MMU transitions proven necessary for safe image copying and BL31 handoff; audit the final A64 instructions and barriers.
- Monitor input contract: prove which DRAM parameters, board bytes, DTB/DTBO extents, U-Boot header fields and scratch addresses are actually consumed on the BL31→A32 SVC path. Do not preserve Linux-only fields by assumption.

A useful first implementation milestone is therefore **not** a bootable test image. It is an AArch32 capsule dispatcher unit plus an M1-SPL source branch that compile/link with dummy fixtures, followed by fake-MMC read-only tests and a final ELF MMIO/write-site audit. Only after that can the exact prepared bundle be run through the pinned native builder to generate a matched but still NON-INSTALLABLE artifact.

### 4.1 Host-only pre-entry selection model (2026-11-27)

A pure C policy model was added at
`platform/h700/boot/m1_oneshot_selection_model.{h,c}`, with
`tests/test_m1_oneshot_selection_model.c` and
`scripts/test-m1-oneshot-capsule-selection.py`. It is **not** included in
the LK, SPL, or Boot0 capsule build rules.

Feasibility [#931](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46031541947)
completed 84/84 steps successfully (zero failures, zero skipped). The new
workflow step compiled and ran six policy cases:

1. valid embedded stock Boot0 + valid M1 SPL selects M1;
2. M1 SPL FNV mismatch selects stock;
3. M1 SPL eGON magic mismatch selects stock;
4. M1 SPL size mismatch selects stock;
5. invalid embedded stock Boot0 fail-stops, even if M1 SPL is valid; and
6. missing stock candidate fail-stops.

The model uses the existing capsule FNV-1a convention for these host policy
fixtures. FNV is not a cryptographic signature, and the model does not perform
the outer capsule's eGON checksum/entry verification, execute the A32
copy-stub, validate the final SPL ELF, or read hardware. Those remain separate
builder / linked-ELF checks. It proves only the static choice rule.

Artifact: [15215206088](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46031541947/artifacts/15215206088),
SHA-256
`0f6c6294f43fb734317be550815da2aa5bd2d89451e3c3e66e1b35025c1b4f6a`.
No real capsule/SPL was built, no test card was written, and no payload was
executed.

### 4.2 Freestanding AArch32 selector ABI fixture (2026-11-28)

A separate target-ISA fixture now compiles the existing policy model and a
dummy capsule-facing C ABI entry as **freestanding Cortex-A53 AArch32 ARM-state
ELF32**. The fixture is intentionally not part of the Boot0 capsule, SPL, or
LK production object lists.

- `scripts/test-m1-oneshot-capsule-a32.py` compiles
  `platform/h700/boot/m1_oneshot_selection_model.c` together with
  `tests/m1_oneshot_capsule_a32_fixture.c` using `clang --target=arm-none-eabi`,
  `-mcpu=cortex-a53`, `-marm`, `-ffreestanding`, strict warnings and LLD.
- It checks ELF32 / ARM machine identity, requires the public selector and
  dummy dispatch entry symbols, and rejects any unresolved runtime symbol.
  The internal FNV helper is allowed to inline under `-O2`.
- Full feasibility [#937](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46035530557)
  completed successfully on code commit `f3a815e16658533227e0907c8b4079185153ba9a`.
  The `cross-compile` job reports 85/85 recorded steps successful, with zero
  failures, skips, or cancellations (the count includes setup/post-job records).
  The A32 selector fixture and linked GammaOS LK H700 BL33 CRT0/exception-vector
  build both passed. The workflow's complete host/cross-build matrix also passed.
- Artifact [15215748657](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46035530557/artifacts/15215748657)
  (387,993,785 bytes) contains maps, ELF files, logs, and reports. Its
  `h700-lk-overlay-build/REPORT.md` records the pinned LK overlay raw ELF as
  5,804 bytes at entry `0x4a000000`, with the exception vector table at
  `0x4a001000`, wrapper at `0x4a001024`, and `_end=0x4a0036d0`.
- The runner installed LLVM/LLD from Ubuntu packages after the workflow
  disabled the unrelated Microsoft apt feed that had previously returned 403.
  The earlier #934/#936 failures were tooling-feed / over-strict-test issues;
  #937 is the first full green run after those fixes.

**Scope boundary:** this proves the selector C policy is accepted by the
AArch32 compiler/linker and exports the expected fixture ABI. It does not
execute the ELF, check eGON checksum/entry semantics, perform the SRAM copy
stub at `0x50000` / `0x20000`, connect the selector to `capsule_main()`, build
an M1 SPL, regenerate the matched firmware/SPL/capsule generation, or touch a
test card. The outer capsule still needs a separately reviewed no-RTC/no-WDT
dispatcher and copy/handoff path. No hardware I/O or payload execution was
performed; `hardware_applied=0`.

### 4.3 Single-core runtime restriction confirmed in the pinned LK layout report

The artifact's `lk-bl33-layout/REPORT.md` and `h700-lk-overlay-build/REPORT.md`
confirm the current pinned LK reference is **not an H700 SMP runtime**:

- Its scheduler uses one global `current_thread`, one global run-queue set and
  one global critical-section nesting counter. `arch_disable_ints()` masks
  IRQs only on the executing core; it is not a cross-core lock.
- The pinned `crt0.S` sets `ACTLR.SMP` only for the Cortex-A7 path; there is
  no Cortex-A53 enable path in that reference. Matched BL31 source-binary
  audit makes the initial CPU1–CPU3 hold-off **probable, not device-verified**:
  Boot0's helper keeps CPU2/CPU3 reset under H616 reference semantics and
  omits CPU1; SPL configures only the primary RVBAR. However, BL31 also has
  a later PSCI `affinst_on` callback (`0x4800255c`) to CPU_ON helper
  `0x48002d78`, which can program per-core RVBAR and power/reset state.
  The helper's SoC branch depends on selector `0x03000024 & 7`, which has
  not been read. Its existence is not evidence it is auto-called at initial
  BL33 entry; it is also not safe to call from this UP LK runtime.
- Therefore any future M1 diagnostic using this LK runtime must be explicitly
  CPU0-only and gated on separate evidence that CPU1–CPU3 remain held off /
  in reset for the entire runtime. Do not invoke PSCI CPU_ON, release
  secondaries, or treat the global critical-section counter as an SMP-safe lock.

These are pinned-source and synthetic-link findings only. They do not prove
the real SoC's CPU1–CPU3 reset state, ACTLR state, cache coherency, or memory
ownership. The gate remains OPEN and no payload was executed.

### 4.4 SPL / BL31 RVBAR selector parity gate

The pinned SPL and matched BL31 later CPU_ON helper select RVBAR banks
differently. SPL uses `low3 == 2` for alternate bank `0x08100040`, otherwise
primary `0x09010040`. BL31 CPU_ON uses `low3 == 0` for primary, otherwise
alternate. They match only for `VER_REG[2:0]` values 0 and 2; values
1, 3, 4, 5, 6 and 7 mismatch.

The all-values regression [BL31 audit #136](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46039667955)
marks this `UNRESOLVED_UNTIL_SELECTOR_READBACK`. It is a potential cross-stage
incompatibility, not a confirmed mismatch on this device: the only target
read attempt at `0x03000024` failed with EFAULT. M1/LK must not call PSCI
CPU_ON or release CPU1–CPU3. Keep the runtime CPU0-only until selector and
reset/RVBAR state are established through a trusted read-only path and an
SMP-capable runtime is independently qualified.

### 4.4 Diagnostic artifact byte/runtime accounting (2026-11-28)

The Linkslot/LK feasibility #943 artifact was downloaded and independently hashed. Three distinct A32 prototypes are present; none is a deployable M1 image. The PI12 early-MMU layout raw file is 7,940 bytes (24,828 bytes below the 32 KiB loader file limit), but its linked runtime span through `_end=0x4a00c024` is `0xC024` / 49,188 bytes, including an 8 KiB early stack, a 16 KiB translation table, and BSS. Its ELF explicitly opts into PI12 physical MMIO for disposable layout validation and is not authorized for execution. The other two raw prototypes are 5,804 and 5,148 bytes and are separately linked against stubs; their synthetic header companions are not vendor header material.

The exact hashes, LOAD segment accounting, header-range checks, and promotion gates are recorded in [M1 candidate artifact accounting](m1-candidate-artifact-accounting.md). This closes artifact provenance/accounting only; it does not close BL33 memory ownership, PI12 access, CPU reset/RVBAR, or one-shot capsule gates.

## 5. Host-only verification required before producing an M1 artifact

The M1 builder must emit a manifest with at least:

- hardware_qualified=false
- durable_install_eligible=false
- not_installable=true
- device_io_performed=false
- rtc_access=false
- watchdog_access=false
- block_device_writes=false
For the strict no-PMIC profile, assert `pmic_access=false` and
`twi_access=false`. For the deployed-native-baseline reuse profile, assert
`pmic_access=true`, `twi_access=true`, exact source/config pins for AXP717,
DCDC2=940 mV and DCDC3=1100 mV, and no PMIC/voltage write delta beyond that
baseline. Both profiles must assert `display_gpio_access=false`,
`rtc_access=false`, `watchdog_access=false`, and
`block_device_writes=false`.

Tests must include:

1. A source-level call graph and a linked-ELF/disassembly audit of the exact SPL/capsule. A raw byte-string scan is insufficient because A32/A64 immediates can be built with MOVW/MOVT or literal pools.
2. A forbidden-symbol/operation check for RTC, watchdog, PMIC/RSB/TWI, blk_dwrite and all block-device write APIs. Check the final linked ELF, not just C source.
3. Fake MMC backend tests that allow reads only and fail immediately on any write command or write-sector request.
4. Instruction-level audit of all absolute MMIO references and every write site. Distinguish required SD-controller/DRAM setup from forbidden data-media writes and unapproved peripheral side effects.
5. Exact hash/FNV/extent checks for Boot0 source, firmware package, monitor, DTBO, DTB, M1 loader, generated SPL and outer capsule. All must describe one generation.
6. Linker-map checks for the outer capsule and SPL stack, BSS, code/data buffers, firmware staging and final BL33 destinations.
7. A manifest assertion that there is no implicit fallback, no persistent state protocol, and no hidden media write.

Until all checks pass, the output remains NON-INSTALLABLE / NOT FOR DEVICE.

## 6. Recovery model and hardware gate

This mode has **no automatic runtime fallback after the M1 SPL begins execution**. Its only software fallback is the pre-entry selection of the embedded stock Boot0 when static image validation rejects the M1 SPL. If the M1 SPL starts and subsequently hangs, the only proposed recovery is:

1. power the device fully off;
2. remove the dedicated M1 TF1 test card;
3. restore the untouched original system card.

That is a proposed manual recovery procedure, not yet a hardware-qualified recovery guarantee. It assumes the device can be powered off after a failed or stalled handoff, that the original card was never inserted during the test, and that the one-shot capsule cannot write to any persistent SoC state. These assumptions must be reviewed before any hardware test.

The current M1 work remains host-only. M1-S1 (PI12 qualification), G2 (physical BL33 memory ownership), G3 (CPU1–CPU3 state and timer/PPI continuity), and the one-shot capsule recovery gate remain OPEN. Keep H700_MEMORY_LAYOUT_APPROVED=false and hardware_applied=0. Do not write a card, Boot0, TOC1, RTC, watchdog or device MMIO as part of this design work.

## 7. Pinned source locations

All paths below are at BaseOS-MultiFrontend commit cbe253a43f46cf551cc9a6c79a84015bde3686cf:

- experiments/h700-boot-speed/spl/early-mmc.c
- experiments/h700-boot-speed/spl/build-early-mmc.py
- experiments/h700-boot-speed/spl/native-boot.c
- experiments/h700-boot-speed/native/capsule/capsule.c
- experiments/h700-boot-speed/native/capsule/entry.S
- experiments/h700-boot-speed/native/capsule/build.py
- experiments/h700-boot-speed/native/capsule/build-diagnostic.py

The matching RG34XX native-generation provenance is documented in rg34xx-native-generation-provenance.md.
The full M1 diagnostic boundaries remain in m1-bl33-diagnostic.md.

### 4.5 Standard release driver is not an M1 one-shot builder (2026-11-28)

A direct source trace of the pinned BaseOS `experiments/h700-boot-speed/native/build-release.py` closes an important packaging ambiguity:

- `build-release.py` calls the native builder with `raw_capsule=True`.
- Its `spl()` helper calls `build-early-mmc.py` with `raw_capsule=True`.
- It then invokes the existing `native/capsule/build.py` builder for the outer capsule.
- The generated package checks that native manifest and capsule manifest share the same generation. This proves generation pairing, not that the capsule protocol is suitable for M1.
- The pinned raw-capsule path still commits RTC GP14/GP15 attempt state and arms/reads the watchdog; the outer capsule and native handoff have their separately audited side effects.

Therefore, the documented `build-release.py` command in the BL33 image-format guide is a **host build/reproduction route for the existing BaseOS raw-capsule v2 release flow only**. It must not be run or described as producing an M1 one-shot capsule. Even if its firmware, SPL and capsule hashes all match, the result remains ineligible for M1 until the outer capsule, SPL and native handoff are separately implemented for the M1 one-shot contract and audited from the final linked binaries. Do not use its output to write TF1.

This finding is source-level only; no build-release package was generated in this review.

### 4.6 Release-driver path is now regression-pinned

The host source audit `scripts/audit-m1-oneshot-baseos-source.py` now also reads the pinned `native/build-release.py` and asserts that the standard release driver invokes the native builder, SPL builder and outer capsule builder through the raw-capsule v2 path, and that its generation equality check is present. The audit output explicitly records `standard_release_driver.m1_one_shot_eligible=false`. This is a regression guard against accidentally treating generation pairing as protocol qualification.
