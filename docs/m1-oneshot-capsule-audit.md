# M1 one-shot capsule / SPL safety audit

**Status:** design constraint and source audit only. No M1 capsule, M1 SPL, or paired image has been built. No SD write, device MMIO, or payload execution was performed.

**Target:** RG34XX non-SP / H700, replacing only the current AArch32 SVC BL33 loader at 0x4a000000 on a separately prepared TF1 test card.

**Source pin:** BaseOS-MultiFrontend cbe253a43f46cf551cc9a6c79a84015bde3686cf.

## 1. Decision

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
4. Do not call PMIC/RSB/TWI helpers, PMIC scratch operations, or panel/PIO display startup. Required clock, bus, and DRAM setup must be isolated and justified independently; no broad register setup may be copied merely because it is in the production path.
5. Read only the exact, pinned p4 + 40 MiB firmware package. Validate the package hash/FNV and every extent before copying the vendor monitor, DTBO, DTB, and M1 A32 BL33 payload to their audited destinations.
6. Do not load the p4 + 48 MiB BOL4 Linux image, perform kernel decompression, prepare a Linux bootargs/FDT handoff, or issue the Linux-start SMC.
7. Preserve only the vendor BL31 input fields proven necessary for its A32 SVC BL33 entry: selected DRAM parameters, the exact board bytes and DTB pointer. Their provenance and offsets must be tied to the matching package and Boot0.
8. Any required RVBAR/RMR transition must be separately reviewed as part of the handoff ABI; it is not permission to use unrelated PMIC, display, RTC, watchdog, or media-write functions.

### M1 A32 BL33 payload

- Keep the raw payload within the active 32 KiB loader extent and use the exact currently deployed vendor header template in the native package builder.
- Entry/vector, BSS, early stack, page table and _end must be checked from the actual linked ELF/map. The existing host stack/page-table non-overlap test is necessary but does not approve the physical BL33 region.
- The PI12 MMIO probe remains disabled until M1-S1 proves APB1/PIO handoff access, the SPC/fuse condition, the pin mux and polarity, PI11 preservation, and the abort/fail-stop path.
- No UART writes, framebuffer, GIC/timer, heap, DMA, SMP, PMIC/TWI, or application DRAM arena in the first proof payload.

## 4. Host-only verification required before producing an M1 artifact

The M1 builder must emit a manifest with at least:

- hardware_qualified=false
- durable_install_eligible=false
- not_installable=true
- device_io_performed=false
- rtc_access=false
- watchdog_access=false
- block_device_writes=false
- pmic_access=false
- display_gpio_access=false

Tests must include:

1. A source-level call graph and a linked-ELF/disassembly audit of the exact SPL/capsule. A raw byte-string scan is insufficient because A32/A64 immediates can be built with MOVW/MOVT or literal pools.
2. A forbidden-symbol/operation check for RTC, watchdog, PMIC/RSB/TWI, blk_dwrite and all block-device write APIs. Check the final linked ELF, not just C source.
3. Fake MMC backend tests that allow reads only and fail immediately on any write command or write-sector request.
4. Instruction-level audit of all absolute MMIO references and every write site. Distinguish required SD-controller/DRAM setup from forbidden data-media writes and unapproved peripheral side effects.
5. Exact hash/FNV/extent checks for Boot0 source, firmware package, monitor, DTBO, DTB, M1 loader, generated SPL and outer capsule. All must describe one generation.
6. Linker-map checks for the outer capsule and SPL stack, BSS, code/data buffers, firmware staging and final BL33 destinations.
7. A manifest assertion that there is no implicit fallback, no persistent state protocol, and no hidden media write.

Until all checks pass, the output remains NON-INSTALLABLE / NOT FOR DEVICE.

## 5. Recovery model and hardware gate

This mode has **no automatic runtime fallback after the M1 SPL begins execution**. Its only software fallback is the pre-entry selection of the embedded stock Boot0 when static image validation rejects the M1 SPL. If the M1 SPL starts and subsequently hangs, the only proposed recovery is:

1. power the device fully off;
2. remove the dedicated M1 TF1 test card;
3. restore the untouched original system card.

That is a proposed manual recovery procedure, not yet a hardware-qualified recovery guarantee. It assumes the device can be powered off after a failed or stalled handoff, that the original card was never inserted during the test, and that the one-shot capsule cannot write to any persistent SoC state. These assumptions must be reviewed before any hardware test.

The current M1 work remains host-only. M1-S1 (PI12 qualification), G2 (physical BL33 memory ownership), G3 (CPU1–CPU3 state and timer/PPI continuity), and the one-shot capsule recovery gate remain OPEN. Keep H700_MEMORY_LAYOUT_APPROVED=false and hardware_applied=0. Do not write a card, Boot0, TOC1, RTC, watchdog or device MMIO as part of this design work.

## 6. Pinned source locations

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
