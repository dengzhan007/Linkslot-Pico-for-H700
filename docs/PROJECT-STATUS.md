## 2026-11-28 RG34XX Boot0 container and embedded SPL2 finding

- The reports-branch captures are from the RG34XX TF card: primary Boot0 read at \`/dev/mmcblk0\` LBA 16 (128 KiB); secondary Boot0 read at LBA 512 (128 KiB capture, 64 KiB declared image).
- The primary 128 KiB image contains an outer eGON Boot0 at \`0x0\`, an embedded 64 KiB eGON Boot0 at \`0x1000\`, and a 48 KiB nonstandard SPL2 component at \`0x11000\`. Both standard Boot0 checksums pass. The SPL2 header tag is \`SPL\\x02\`; standard eGON checksum validation is not applied to it.
- The primary inner Boot0's 15-profile DRAM.ext table is at relative \`0x348\` (absolute \`0x1348\`). The secondary standalone Boot0 has the same 15-profile table at \`0x348\`. The outer primary container also has a separate marker at \`0x348\` followed by one plausible record; do not confuse it with the inner 15-profile table.
- Primary inner Boot0 SHA-256: \`b6edbf39af0bf1f35a7e2fd991c8966b04d061e247c81985d2516f695464049b\`. Secondary declared-image SHA-256: \`4701a85e4db7e8339b91b92f691a83cc954ec63a09369087c2e41d80eafa9f5d\`. They differ at exactly five byte offsets: \`0x0c\`, \`0x0f\` (checksum), \`0x34\` (debug byte), \`0xa3\` (TPR6), \`0xac\` (TPR12). Both active blocks report LPDDR4 672 MHz; TPR6 is \`0x3e808080\` vs \`0x40808080\`, TPR12 is \`0x06070706\` vs \`0x06070707\`.
- The SPL2 component embeds the build/platform label \`allwinner/sun50i-h700-anbernic-rg35xx-2024\`. The same inventory's live FDT contains \`:rg34xx_v1\`, \`x-powers,axp2202\`, and R-TWI0 \`twi@0x07081400\`. This is a cross-component naming discrepancy on the actual RG34XX capture, not proof of incompatible firmware; the vendor may share this H700 SPL across devices. The SPL's DRAM/PMIC/strap assumptions must be audited before reuse.
- Candidate handoff correlation: the outer Boot0 stub loads PC from literal file offset \`0x494\` with value \`0x50000\`; the SPL2 first word \`0xea000016\` branches to component offset \`0x60\`, yielding candidate code entry \`0x50060\` if loaded at \`0x50000\`. This is consistent with a Boot0-to-SPL2 handoff, but exact copy/selection/reachability is not yet proven.
- Detailed report: docs/rg34xx-boot0-container-spl2-audit-20261128.md. The next CI pass disassembles the inner Boot0 at VMA \`0x20000\`, SPL2 at candidate VMA \`0x50000\`, and scans TOC1 plus the live FDT for board/component strings. No hardware action or payload execution is authorized.

## 2026-11-28 AXP2202 / AXP717 naming reconciliation

- Corrected the earlier PMIC naming concern: Linux upstream AXP717 support author Andre Przywara says Allwinner BSP often calls the same chip AXP2202 and reports that a Boot0 AXP2202 driver drove a chip labelled AXP717. Allwinner's Tina PMU guide explicitly says AXP717 uses the AXP2202 software framework name in DTS/kernel configuration.
- Inspected Allwinner-authored Tina v821 AXP2202 SPL reference source (sun50iw10p1 lineage, not the RG34XX/H700 exact target): DCDC3 voltage register 0x85, output control 0x80 bit 2, mask 0x7f, 500–1840 mV split-step encoding; 400 kHz TWI; runtime address 0x34 with 0x35 fallback; CHIP_ID_EXT 0x0e expected value 0x02; generic set_ddr_voltage() dispatcher to axp2202_set_ddr_voltage().
- These register details match the pinned U-Boot AXP717 SPL DCDC3 register/enable map. AXP2202/AXP717 is therefore a strong legacy BSP naming / same-PMIC-family reconciliation, not a silicon incompatibility blocker.
- This does not prove the inspected Tina source is the exact RG34XX Boot0 source, nor prove the target's call site, fallback use, DCDC3-before-DRAM ordering, or necessity of every PMIC init write. The actual RG34XX Boot0 binary/log remains the target authority.
- Additional target Boot0 clue: the binary contains PMU strings for AXP858, AXP2202, AXP1530 and AXP806; the UART log shows two generic bus-read errors before AXP2202 is identified. The inspected Tina v821 dispatcher order is AXP858 → AXP81X → AXP806 → AXP809 → AXP2101 → AXP2202; if the target enables AXP858/AXP806/AXP2202, the two errors are consistent with failed AXP858 and AXP806 probes followed by AXP2202 success. The AXP2202 0x34→0x35 fallback itself would yield one error then success, or two errors then failure. This is the best current hypothesis, not proven target order. The later RG34XX U-Boot log explicitly shows failed AXP152/AXP1530 probes before AXP2202 in a separate boot stage.
- In that independent public sample, DRAM_VCC 1100 mV is logged after chip-ID check and before DRAM DST training. Together with the Allwinner AXP2202 set_ddr_voltage()→DCDC3 helper and target runtime DCDC3=1100 mV, DCDC3 is now the strongest candidate for DRAM_VCC; the exact RG34XX Boot0 call site/write remains unproven. See docs/rg34xx-axp2202-axp717-crosswalk-20261128.md.
- Detailed crosswalk: docs/rg34xx-axp2202-axp717-crosswalk-20261128.md. Next static task: disassemble the target 128 KiB Boot0 and recover the actual PMU probe order, AXP2202 ID read, 0x34/0x35 path, DCDC3 0x85 / enable 0x80 writes, and the DRAM_VCC setter call site/order relative to training.
- Safety unchanged: no PMIC/TWI write, MMIO/SMC, physical I/O, DRAM content access, Boot0/TOC1/TF write, firmware flash or payload execution. M1 remains preflight-only; H700_MEMORY_LAYOUT_APPROVED=false and hardware_applied=0.

## 2026-11-28 RG34XX stock Boot0 DRAM.ext parser correction

- Audited the two raw RG34XX non-SP stock Boot0 captures on the reports branch. Primary is a 128 KiB eGON image; secondary capture is 128 KiB but its eGON header declares a 64 KiB image. Both declared images pass checksum validation.
- Both active header parameter blocks at 0x38 report LPDDR4 type 8 at 672 MHz. The primary image contains a 15-record structurally plausible H616/H700 DRAM profile table at DRAM.ext marker 0x1348; the secondary image contains its 15-record table at marker 0x348. Each table has DDR3 792 MHz entries 0–12, LPDDR3 672 MHz entry 13, and LPDDR4 672 MHz entry 14.
- The previous tools/inspect_h700_boot0.py fixed-offset scan at 0x9f4/0xa74 missed the primary's relocated entries at 0x19f4/0x1a74. Parser now scans all DRAM.ext markers and reports contiguous structurally plausible H616/H700 32-word records, while retaining legacy fields for compatibility.
- Added synthetic multiple-marker regression and a reports-branch regression that checks raw capture SHA-256, declared-image SHA-256, eGON checksums, active LPDDR4/672 MHz header parameters, and both 15-record table locations. The feasibility workflow now includes these tests and fetches only the reports branch's required blobs.
- Feasibility CI run #1077 [completed successfully](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46143096380) on commit 34f5f4d9e9d2: all 102 recorded job steps passed, including step 35 (RG34XX stock Boot0 DRAM.ext extraction) and all existing cross-build/static checks. Artifact [linkslot-lk-rust-feasibility-46143096380](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46143096380/artifacts/15263037006) is available (551,531 bytes; SHA-256 c5712656170639e592f44c9e21210b233dfac0fe79915e987523070dcb3c5cff). This remains host/cross-build and static evidence only.
- An independent public RG34XX Boot0 UART sample (not the reports-branch binary capture) reports PMU: AXP2202, DRAM_VCC set to 1100 mv, DRAM 672 MHz, type 8 / LPDDR4, 1024 MiB, and successful DRAM simple test. This materially strengthens the target-specific AXP2202 path finding, but exact AXP2202 register writes/bus/order remain unproven. Do not equate the log's DRAM_VCC value to a particular AXP717 DCDC rail.
- Detailed evidence: docs/rg34xx-stock-boot0-dramext-audit-20261128.md. H616/H700 profile field mapping is cross-checked against apritzel/sunxi-fw commit 1ba5f4c, reference-only.
- Next static task: recover the exact RG34XX Boot0 AXP2202 branch and trace PMU bus init, ID detection, register writes and ordering relative to DRAM init. No hardware operation is authorized.
- Safety gates unchanged: H700_MEMORY_LAYOUT_APPROVED=false, hardware_applied=0, candidate_eligible=false, hardware_qualified=false; no payload execution, physical I/O, MMIO/SMC, TWI/PMIC writes, TF/Boot0/TOC1 writes, or firmware flashing. DRAM source pages 0x40000000 and 0x60000000 remain excluded.

## 2026-11-28 RTC/WDT regression and AXP2202 runtime rail evidence

- Added `scripts/test-m1-native-attempt-state.py` and `tests/test_m1_native_attempt_state.c`, wired into Linkslot feasibility CI. The host-only model compiles against pinned BaseOS `src/native-attempt.h` at `cbe253a43f46cf551cc9a6c79a84015bde3686cf`; it covers 19 cases including empty/resume/stale state, ordered commit, dropped first/second writes, health/generation rejection, acknowledgment and torn acknowledgment. The model passed locally and the CI step passed in run `46141495743`.
- Strengthened `scripts/audit-m1-oneshot-baseos-source.py` to assert raw-capsule-v2 watchdog stop/readback, CFG→MODE→CTRL arm/readback ordering, refusal of an already-active WDT before arm, attempt-pair commit after watchdog arm, two-pass read-only GP14/GP15/WDT adoption, and native WDT reload after raw adoption. The new source-order audit passed in run `46141495743`.
- Important test boundary: this host model validates the existing BaseOS v2 attempt-pair semantics using fake registers. It does not model actual watchdog MMIO, and it does not authorize reusing the v2 state machine in M1.
- New read-only RG34XX runtime evidence: three captures (2026-11-25/26/27) report AXP2202 DCDC2=940 mV and DCDC3=1,100 mV (enabled); DCDC1 is about 1.15–1.16 V. Tina's AXP2202 SPL reference exposes a DCDC3 DDR-voltage helper, but no target H700 call site or explicit memory-supply link is proven. Do not copy the separate RG SP DCDC3=1.2 V setting into RG34XX. See [`docs/m1-owner-policy-audit-20261128.md`](m1-owner-policy-audit-20261128.md).
- Linkslot feasibility run #1075 [run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46141495743), commit 807b3b2, is **completed / success**. The cross-compile job completed all 92 primary recorded steps and all post-job steps with zero failures; artifact [15261853911](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46141495743/artifacts/15261853911) is available (551,201 bytes; SHA-256 a53c9f9b470c65c1b90a6e252980231ee067852ee7b05f42f25c994a1dc535e4). This is host/cross-build, static-audit and synthetic-test evidence only; it does not authorize hardware application or payload execution.
- Safety gates remain unchanged: no firmware writes, payload execution, physical register writes, or hardware application.

## 2026-11-28 AXP2202 bus identity refined

- The RG34XX-specific Knulli boot-package DTS (commit `57afa662009641d181f57241bcc480119b131530`) independently confirms `twi5` at `0x07081400`, PMU compatible `x-powers,axp2202`, and I2C address `0x34`; this matches the captured RG34XX live-FDT inventory and BL31 selector path.
- Found an Allwinner-authored AXP2202 SPL driver/register map in Tina v821 source (commit `34809037526678ccda1720cb4b4ec7ee32272c38`). It uses a 400 kHz TWI path and runtime address `0x34` (fallback `0x35`), and exposes DCDC3 as its DDR-voltage helper. However, it is a different Allwinner platform; no call site proving its DDR-voltage helper is required for RG34XX/H700 DRAM training has been found. Its initialization writes are candidates for comparison, not an approved sequence.
- The generic U-Boot `AXP717_POWER` versus target AXP2202 DTS difference is now classified as a legacy BSP naming / upstream binding reconciliation issue. Upstream Linux maintainer and Allwinner guide evidence strongly support the same PMIC family; the exact RG34XX Boot0 call path and register sequence remain open.
- Full source evidence and limits are in [`docs/m1-owner-policy-audit-20261128.md`](m1-owner-policy-audit-20261128.md). Next task is to locate the exact RG34XX vendor Boot0/SPL power sequence and prove whether any AXP2202 regulator write is needed before DRAM training. Profile A remains approved as a policy, but implementation remains blocked pending that proof.

## 2026-11-28 Initial PMIC binding discrepancy (superseded by AXP2202 / AXP717 alias evidence)

- Cross-checked the captured RG34XX live FDT against the public U-Boot v2026.10 H700 defconfig and PMIC driver. The target inventory identifies `x-powers,axp2202` on R-TWI0 `0x07081400`; the generic `anbernic_rg35xx_h700_defconfig` selects `AXP717_POWER`, and upstream `drivers/power/pmic/axp.c` has no literal `x-powers,axp2202` compatible entry. Later upstream Linux and Allwinner documentation clarify that AXP2202 is the legacy BSP name/framework for AXP717-compatible silicon.
- This initially appeared to be a material compatibility gap, but subsequent upstream Linux and Allwinner documentation strongly support AXP2202 as the legacy BSP name/framework for AXP717-compatible silicon. Do not treat the naming difference as incompatibility; nevertheless, do not enable PMIC writes or import voltage policy until the exact RG34XX Boot0 sequence and board-specific requirements are recovered.
- The target FDT regulator ranges are declarations only, not measured register state or a safe initialization recipe. No PMIC writes may be inferred from them.
- Detailed evidence and disposition appended to [`docs/m1-owner-policy-audit-20261128.md`](m1-owner-policy-audit-20261128.md). Current M1 preflight remains PMIC/TWI-disabled; that is intentional because it stops before the DRAM/board-init continuation.
- The static source trace has therefore narrowed the next task: identify the exact vendor AXP2202 implementation for RG34XX, reconcile its R-TWI0 access with BL31's matched static path, and only then build a separate Profile A DRAM-capable SPL audit. Continue RTC/WDT host-state-machine design separately; do not reuse raw-capsule-v2.
## 2026-11-28 PMIC/TWI and RTC/WDT source-trace refinement

- Extended [`docs/m1-owner-policy-audit-20261128.md`](m1-owner-policy-audit-20261128.md) after directly reviewing the pinned BaseOS files at `cbe253a43f46cf551cc9a6c79a84015bde3686cf`.
- **Profile A scope clarification:** the BaseOS builder inserts `baseos_early_mmc()` before `sunxi_board_init()`; this early MMC path is explicitly pre-DRAM. The current M1 overlay stops after read-only MMC/GPT preflight and does not invoke the DRAM/board-init continuation. Its no-PMIC/no-TWI config is therefore intentional and must remain so. Profile A applies to the separate future DRAM-capable SPL board-init profile.
- The pinned builder retains `i2c_init_board()` and `i2c_init(...)` on the non-resume persistent board-init path, while resume skips them. This proves a conditional SPL I2C/TWI initialization stage exists, but does not yet identify its exact PMIC writes or prove which regulators/voltages are necessary.
- The PMIC operations in pinned `native-boot.c` (`pmic_bus_read(0x20)`, reads of `0` and `0xf0`, conditional write `0xf0=0`) are in boot-reason/scratch-byte handling called after `sunxi_board_init()`; they are not evidence for DRAM rail initialization and must not be copied into Profile A power-up logic.
- Exact raw-capsule-v2 state addresses documented: GP14 `0x07000138` cookie; GP15 `0x0700013c` marker; retention `0x070001f8` bit 16 resume; WDT CTRL/CFG/MODE `0x030090b0/0x030090b4/0x030090b8`. Outer capsule owns attempt commit and WDT arm; raw SPL adoption is read-only (two-pass state verification); later native handoff reloads WDT. The pair writes are ordered but non-atomic, with readback/barrier checks. M1 must define a separate test marker/protocol and recovery state machine, not reuse v2.
- Next static work: trace `i2c_init_board()`, `i2c_init()`, AXP2202 bus/register mapping and DRAM regulator requirements from pinned U-Boot/board sources; independently build a host-only RTC/WDT test-state model with torn-write, stale-marker, watchdog-reset and power-loss cases. No physical register writes.

## 2026-11-28 Owner policy decisions and static-audit continuation

- Owner decisions are now recorded: **SPL Profile A** (only source-proven, individually audited PMIC/TWI operations may be retained) and **native RTC/WDT accepted only for an audited dedicated test-TF workflow**. These decisions do not authorize execution or hardware application.
- Detailed policy, evidence boundaries, and next audit requirements: [`docs/m1-owner-policy-audit-20261128.md`](m1-owner-policy-audit-20261128.md).
- Important distinction: the current linked M1 SPL is a **read-only MMC/GPT preflight fixture**, not the future Profile A DRAM/power-init SPL. Its Kconfig intentionally disables SPL I2C/DM-I2C/DM-PMIC/POWER, and the linked contract rejects I2C/PMIC bus symbols. Keep that restriction for this preflight unless a concrete dependency is proven; do not label this artifact Profile A.
- The matched BL31 + original TOC1-DTB static audit resolves suspend-side selector flow to `twi5` / R-TWI0 `0x07081400` and AXP2202 parent node. It does **not** identify a PMIC slave address/register sequence or prove a live transaction. PMIC init provenance remains open.
- Raw-capsule-v2 is stateful (RTC retention/GP14/GP15 attempt pair, watchdog setup/arm, attempt commit, SRAM copy/launch). It is not the dedicated M1 one-shot protocol; do not alter v2 semantics or inherit them implicitly. RTC/WDT ownership, exact consumers, cleanup and reset recovery still require source audit.
- Linkslot/LK feasibility #1073 [run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46138855460), commit `5f46a52`: **completed success**. Compact artifact 551,255 bytes; downloaded and inspected; ZIP SHA-256 matches GitHub digest `dd5f65d63cce41ed240247fe68b397ddf505f2c87574fa31591714d48d4244cb`; all 10 internal SHA256SUMS entries match. Linked contract: BSS 5,032/16,384 bytes, scratch `0x30340`, GPT table `0x30000`, stack `0x52a00`, 160 reachable functions / 759 store-class instructions; all execution/hardware/candidate authorization flags remain false.
- H700 boot audit #1738 [run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46138911358) completed success (Windows and host-tools). The audit triggered by the owner-decision record, run `46139583546`, also completed success (host-tools and Windows; no artifact expected).
- Current follow-up: continue static source tracing for the actual PMIC/DRAM initialization owner and the RTC/GP14/GP15/WDT state machine; add host-only fail-closed regressions before any test implementation. No hardware test is authorized.

## 2026-11-28 Owner decisions — M1 hardware-policy boundaries

The project owner explicitly confirmed the following policy decisions:

1. **SPL Profile A selected.** The isolated M1 SPL may include PMIC/TWI operations only where each operation is traced to the pinned stock initialization path and independently audited (controller, target/address, register, value/mask, ordering, timeout/error handling, and necessity). This is permission to design and statically audit the compatibility profile, **not** blanket authorization for PMIC writes or hardware execution. Do not add speculative voltage changes or broaden PMIC access.
2. **Native RTC/WDT protocol accepted only for an audited dedicated test-TF workflow.** RTC backup/retention register ownership and semantics, GP14/GP15 use, watchdog configuration/arming/reset behavior, stale-state cleanup, timeout and recovery paths must be documented and statically audited before any test implementation can be exercised. Do not enable this protocol in the normal daily boot path by default.
3. **No execution or hardware-application authorization is implied.** Firmware payload execution, DRAM destination writes/copies, physical device tests, TF/Boot0/TOC1 writes, and firmware flashing remain unauthorized. Preserve `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`, and `candidate_eligible=false`. DRAM source pages `0x40000000` and `0x60000000` remain excluded from scratch use.
4. **Next work:** continue source-level PMIC/TWI provenance and side-effect auditing for Profile A; separately map RTC retention/GP14/GP15 and WDT ownership/protocol against the pinned BaseOS capsule and H700 startup sources. Add regression checks for the declared allowlist/denylist and fail-closed behavior. Keep M1 final-link/callgraph/ELF verification as the mainline; do not reinterpret these owner decisions as closure of G2/G3/M1-S1 or permission to advance to M2.

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

## 2026-11-28 M1 source-audit boundary review (continuation)

- Re-read the pinned BaseOS capsule implementation (`capsule.c`, `entry.S`, `capsule/build.py`) alongside `scripts/audit-m1-oneshot-baseos-source.py`. The ordinary raw-capsule v2 path is explicitly stateful: it reads RTC retention/GP14/GP15, evaluates the attempt pair, configures/arms the watchdog, commits the attempt pair, and then copies the saved CPU context plus selected image through the SRAM copy stub. This confirms the existing release route is not an M1 one-shot implementation; generation matching does not change that protocol.
- The current source audit is a **source-contract regression**, not a complete transitive call-graph proof or final-linked-ELF instruction audit. Its own scope statement says it cannot replace final ELF auditing. A green result may establish that the pinned v2 call chain has not drifted; it must not be interpreted as proof that a future M1 implementation is free of RTC/WDT/PMIC/TWI/PIO/media-write side effects.
- The outer capsule builder also enforces fixed 64 KiB stock + 48 KiB native payload extents, reserves the 0x400-byte vendor header, links the copy stub into a bounded prefix, and emits a 128 KiB eGON image. These are useful packaging invariants to reuse as test requirements, but the resulting image remains raw-capsule v2 and is not an M1 candidate.
- CI status correction (verified 2026-11-28): Linkslot/LK feasibility [#947](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46095436940) is **completed / success** (`cross-compile` job; all recorded steps successful). Its run is on commit `6ecf42a6a3e6640ee6b925a5cca0347be01bcc71`; this validates the host/cross-build and source-audit workflow, not hardware handoff, installability, or device boot. Feasibility [#946](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46094599350) is also completed / success. Feasibility [#945](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46094300447) is completed / cancelled by branch concurrency and is not a test failure.
- Next engineering gate: design a separate `BASEOS_M1_ONESHOT` capsule/SPL/native path rather than editing the semantics of `BASEOS_RAW_CAPSULE`; then add fake-MMC read-only tests, a transitive source/call-graph audit, and final linked-ELF MMIO/write-site audit. Keep all generated artifacts host-only and candidate-ineligible until G2/G3/M1-S1 and recovery gates are independently closed.
- Hardware gate unchanged: `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`. No device MMIO/SMC, TWI transaction, DRAM content access, TF/Boot0/TOC1 write, firmware flash, or payload execution was performed in this review.

# 项目当前状态与续接记录

## 2026-11-28 M1 isolated SPL link debugging

The first attempt to link the M1 preflight-only SPL exposed a configuration problem,
not a source-code failure in the read-only GPT/firmware adapters:

- feasibility #1009 reached the isolated SPL link, then failed on unresolved
  full-U-Boot references including `do_bootd`, `env_get`, `ofnode_*`,
  `lmb_*`, `realloc_simple`, `vsnprintf`, and `__start_rodata`.
- Root cause: the stock RG35XX H700 defconfig is a full-featured vendor SPL
  configuration. The preflight build was inheriting unrelated command, FIT,
  PMIC/TWI, and image-relocation paths while only part of their SPL-specific
  providers were selected. This made the first link unsuitable as a measure of
  M1 scratch/BSS integration.
- The isolated builder now explicitly requests DM-backed MMC, SPL block I/O,
  SPL device-tree support, and retains read-only `blk_dread`; it disables
  common command/FIT/serial/I2C/power/LED and vendor PMIC paths, and excludes
  unrelated `membuf.o` / `abuf.o` dynamic-buffer helpers from the temporary
  U-Boot source checkout. The upstream U-Boot pin is not modified in Git.
- This is an isolated compile/link harness, not the stock BaseOS SPL, not a
  deployable image, and not evidence that runtime hardware accesses are safe.
  The harness still has to prove that its actual final ELF contains the M1
  4 KiB scratch and bounded GPT table inside the measured BSS, and that all
  other symbols and side effects meet the source-level contract.

The follow-up feasibility run #1010 (commit `acf920e`) is in progress.
The next decision is based on the actual linker output, not source geometry
alone. If it fails, preserve the full linker diagnostics and correct only the
specific missing SPL providers/configuration. Do not widen the profile back to
the vendor PMIC/TWI path just to obtain a link.


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
 

## 2026-11-28 M1 read-only firmware package preflight

- New host-tested C contract: `platform/h700/boot/m1_firmware_readonly.{h,c}`; host fake-card tests and freestanding AArch32 ELF32 link check: `scripts/test-m1-firmware-readonly.py`, `tests/test_m1_firmware_readonly.c`.
- The planner models the captured active p4+40 MiB package (0x46200 bytes / 561 sectors), validates partition/staging bounds, reads via a read-only callback, verifies the build-supplied FNV-1a and monitor/DTBO/DTB zero padding, and rejects altered extent/destination metadata.
- Feasibility [#950](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46097505586) passed **87/87 recorded steps, 0 failures, 0 skipped**; artifact [15243703880](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46097505586/artifacts/15243703880). H700 AArch32/AArch64 compile [#670](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46097466062) also passed.
- This remains a synthetic host/freestanding preflight, not BaseOS SPL integration or real-card CMD17 verification. No package extent was copied to physical DRAM. No device MMIO/SMC, TWI transaction, DRAM content access, SD/Boot0/TOC1 write, firmware flash, or payload execution; `hardware_applied=0`.
- Next: implement a distinct BaseOS `BASEOS_M1_ONESHOT` early-MMC patch overlay using this read-only contract; add fake-MMC adapter integration and transitive board-init/MMC call-graph checks before final linked A64 SPL ELF audit. Do not alter ordinary or raw-capsule-v2 behavior.


## 2026-11-28 M1 manifest automation continuation

- Code change: [`dcbf73d`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/dcbf73d9ef6d0431f8bd282853366153f646adbc) adds `scripts/audit-m1-a32-artifact.py`. It binds one ELF, its raw objcopy output and linker map by SHA-256; validates ELF32 little-endian ARM ET_EXEC, entry/_start, _end/map agreement, PT_LOAD extents, zero vendor-header reservation, A32 entry/vector branches and the exact 32 KiB raw extent.
- CI wiring: [`714f7e7`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/714f7e7bceb0d5eba496a083cdcf1d8901d26c9f) adds a feasibility workflow step to emit separate manifests for PI12 early-layout, stub overlay and A32 entry stub. Each is permanently tagged host-only, not-installable, hardware-unvalidated and candidate-ineligible. PI12 is explicitly labeled physical-MMIO-enabled disposable layout work.
- A map parser correction is in [`ec37e97`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/ec37e97ab81d6c3a8b1898f6fca1674b1f363b4b), matching the actual LLD map format (hex address followed by symbol assignment).
- Documentation correction: [`67fa2b2`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/67fa2b26d5c318a391be1cdb64532ac9f5696fd5) distinguishes the active 32 KiB package extent from the separate `0xe0000` researched runtime/link bound. The new manifest contract is documented in `m1-candidate-artifact-accounting.md`.
- H700 boot audit [#1608](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46094478292) passed Linux host-tools and Windows PowerShell on documentation HEAD `19e8f05616e334c6fa2540af444ed78aa8c9037f`.
- Linkslot/LK feasibility [#945](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46094300447) was cancelled by branch concurrency before the new manifest step (not a test failure). Replacement [#946](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46094599350) completed successfully: cross-compile 86/86 recorded steps, zero failures. Artifact [15243336147](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46094599350/artifacts/15243336147), 387,988,856 bytes, ZIP SHA-256 `bb40941103f4167fd7933081d5e4b9dbf79e519b6590f59079215bbc538a0691`. The three emitted JSON manifests were extracted and checked; all report byte-identical ELF→raw objcopy, correct map/_end binding, and `candidate_eligible=false`.
- No single prototype is promoted. The new manifests are accounting outputs only; no actual vendor header/native firmware/SPL/Boot0 capsule pairing or TF1 image has been generated. G2/G3/M1-S1/one-shot recovery gates remain open; `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`.

- Manifest results from #946: PI12 raw 7,940 B / runtime span 49,188 B / `_end=0x4a00c024`; stub overlay raw 5,804 B / runtime span 14,032 B / `_end=0x4a0036d0`; A32 entry stub raw 5,148 B / runtime span 13,372 B / `_end=0x4a00343c`. Each has 3 PT_LOAD segments and all raw files are byte-identical to the supplied ELF's `llvm-objcopy` output. The PI12 profile remains explicitly MMIO-enabled disposable layout only.
- Follow-up source-audit hardening: commit [`6ecf42a`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/6ecf42a6a3e6640ee6b925a5cca0347be01bcc71) pins the standard BaseOS release driver's `raw_capsule=True` native/SPL/outer-capsule call chain and its generation-equality assertion. Feasibility [#947](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46095436940) was independently rechecked and completed successfully; its source-contract regression passed, while the standard release route remains explicitly ineligible for M1 one-shot.

- Additional source-trace finding: pinned BaseOS `build-release.py` calls native build, SPL build and outer capsule builder with `raw_capsule=True`. This gives matched generation metadata but retains RTC GP14/GP15 + watchdog v2 behavior; it is not the M1 one-shot build route. The exact call chain and restriction are recorded in [M1 one-shot capsule audit](m1-oneshot-capsule-audit.md#45-standard-release-driver-is-not-an-m1-one-shot-builder-2026-11-28) and the BL33 image-format guide now warns against using its output on TF1. No package was built.


> **每次开启新的移植对话，先读本文。** 本文是当前工作状态的交接入口；详细设计仍以各专项文档为准。每完成一轮代码、测试或仓库审查，都应同步更新本文、路线图及对应专项文档。

**最新 BL31 static memory audit：** [#136](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46039667955) 成功（artifact [15219895068](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46039667955/artifacts/15219895068)）。新增 selector parity regression 穷举 `VER_REG[2:0]` 0–7：native SPL 与 BL31 后续 CPU_ON helper 仅在 0/2 时选择相同 RVBAR bank，1/3/4/5/6/7 不同。设备 selector 尚未读回，因此状态是 `UNRESOLVED_UNTIL_SELECTOR_READBACK`，并非已证实设备处于 mismatch。#134 是此前 reset-bit / CPU_ON 静态审计基线。全部是静态/host 证据，没有设备 MMIO 写入或 payload 执行。

**本轮续接（2026-11-27，audit-only；代码 HEAD `ccfb33da5f9d36c2531ad4e56c817cfd6eba4309`）：** FDT inspector 已增加 serial0 alias、UART reg/clock、pinctrl state/phandle 与 `uart0_jtag` group 解码；H700 boot audit #1501–#1510 的 Linux/Windows jobs 均成功，AArch32/AArch64 compile #660 成功。Linkslot/LK feasibility #898 在本轮初始 HEAD `e2bce305` 上成功，本轮未修改 LK target code。BaseOS native generation reproduction #45301188593 已在 host 侧精确重建 prepared prefix/Boot0/env、generation、A32 loader、firmware、image 和 128 KiB capsule（capsule byte-exact），该构建来源阻塞已关闭；artifact 仍标注 `hardware_qualified=false`、`durable_install_eligible=false`。本轮继续向下追踪 UART：H616 U-Boot v2026.10 的 `board_init_f` 包含 clock/gpio/preloader-console 初始化，H616 console index 1 分支将 PH0/PH1 mux 到 UART0、APB2 配 24 MHz、开 UART gate/解除 reset，并调用 `serial_init`；精确 SPL map 另含 `eserial1_device/eserial1_init`，不含 eserial2–6。结合 legacy serial source 将 COM1 映射至 UART0、Kconfig non-DM SPL 默认 index 1，这是强 build-level inference（生成的 `.config` 未归档），不是直接 config readback。目标 exact BL31 monitor（SHA `676bc020…`）在 BL33 准备前检测 UART gate，已启用时复用控制器、未启用时有 24 MHz/115200 fallback；同一 BL33 准备路径还尝试配置 SPC/CCU/PRCM/DMA security。新增 `tools/audit_rg34xx_bl31_uart.py` 与回归测试，固定校验 monitor SHA、指令调用点、UART reuse/fallback 分支和 14 个 SPC decode-port 写入；H700 boot audit #1510 两端全通过。以上都是 source/target-binary 静态审计，不是设备 MMIO 或 BL33 实时状态。M1-S1 仍 OPEN（SPC fuse/有效权限、PH0/PH1 物理 test-pad/电平/连线与入口 readback 未证明）；G2/G3 仍 OPEN，`H700_MEMORY_LAYOUT_APPROVED=false`、`hardware_applied=0`。详见 [UART handoff audit](rg34xx-uart-bl31-handoff-audit.md)。

**重要地址更正：** 以前将 copied suspend code 的 link-time ADRP 目标直接当作运行时 monitor VA，误把 `0x48014628` / `0x48014668` 当作活动 table/context slot。该代码实际从 `[0x4800c220,0x48015630)` 复制至 SRAM `0x20000`；重定位后 active table / context-pointer source-equivalent 地址分别是 `0x48014848` / `0x48014888`。selector producer 路径为 `0x48002998 → 0x48001790 → 0x480012cc)：根据原始 TOC1 DTB 的 `pmu0` alias 读取 parent `device_type=twi5`，写 selector 5 到 external context `0x4803cbb4`；0x30-byte context staging copy 将其映射到 SRAM `0x200a4 = context +0xa4`。因此静态代码路径已确定选择 R-TWI0，而不是“index 73 条件候选”。仍未证明某次真实 suspend 执行了该路径；PMIC slave address、事务/命令字节与最终 power action 仍未知。详见 [RTC/TWI 专项审计](rg34xx-rtc-soft-entry-twi-audit.md)。

此前 full monitor alias/CFG 审计 #128 的 79 ADRP seeds、38,676 CFG states、83 context-like slot load path hits，以及在其已建模范围内 zero selector/slot stores 仍是准确记录；它没有建模此处通过 FDT alias/parent property 生成 external context、再 staging 到 copied SRAM context 的完整跨区域数据流，不能用其零命中否定本次 producer 链。H700 boot audit #1480 对前次状态/路线图修改成功；本次 TWI 专项与状态更新已触发后续 boot-audit run。完整 Linkslot/LK feasibility **#892** 在较早文档 HEAD `59c7aa9d...` 上成功（[run #892](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45973668270)）；#891 后续已取消，不作为失败构建结论。

独立硬件门槛不变：#94 静态 Boot0 分析显示 CPU1–CPU3 hold-off 仍未取得 H700 实机 reset/RVBAR readback；DRAM retention PFN `0x40000` 是 `LINUX_ACTIVE_ANON_MAPPED_PAGE`，不能视为自由 scratch；BL33 runtime/BSS/page-table/stack 所有权、APB1/PIO permission 与 timer/PPI continuity 仍未闭合。上述结果全是匹配二进制/原始 DTB 的 host/static 审计，不是实机 suspend/wake 或 TWI transaction。继续保持 `H700_MEMORY_LAYOUT_APPROVED=false`、`hardware_applied=0`；未执行设备 SMC/MMIO、TWI transaction、DRAM 内容读写、待机/唤醒、Boot0/TOC1 写入、刷写或 payload execution。
## 2026-11-28 M1 artifact accounting continuation

- Current branch HEAD before this documentation update: `4b2de492e1749e27697a351af13a753a5422e831` (M1 capsule audit now links the new artifact accounting report).
- H700 boot audit [#1602](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46092843983) completed successfully on predecessor HEAD `0469a6e17a3fc88b979ad82621122143f35630dc`: Linux host-tools 7/7 and Windows PowerShell 8/8 steps passed.
- Downloaded and independently inspected Linkslot/LK feasibility [#943](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46086258522), artifact [15239048815](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46086258522/artifacts/15239048815), ZIP SHA-256 `43bee56b2238fc484284eb9ee0677bfe7a9ec9e8f5279dbc07d5358c8a351f89`.
- Three separate A32 prototypes are now byte-accounted: 7,940-byte PI12 early-MMU layout raw, 5,804-byte CRT0/exception overlay raw, and 5,148-byte A32 entry/vector/snapshot raw. The 7,940-byte raw has runtime `_end=0x4a00c024`, span `0xC024` / 49,188 bytes, with 8 KiB stack and 16 KiB translation table; raw file size is not runtime memory footprint.
- The PI12 early-layout ELF explicitly enabled physical PI12 MMIO for disposable link validation. It is not an M1 deployment candidate. The other two are stub-linked prototypes; synthetic header overlays are not the actual vendor header. No firmware/SPL/Boot0-primary capsule/TF1 pair was generated.
- Exact file hashes, LOAD segment values, linker-map intervals, and packaging promotion gates: [M1 candidate artifact accounting](m1-candidate-artifact-accounting.md). M1 capsule audit links the same report.
- Current next action: create a unified, machine-readable audit/manifest for one selected minimal composition only after separating the dormant/physical PI12 path; bind ELF/map/raw hashes and runtime intervals, then pair the exact vendor header and same-generation firmware/SPL/capsule. Do not package the three prototypes together or treat a raw-size pass as memory approval.
- Hardware gates unchanged: `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`; no device MMIO/SMC, TWI transaction, DRAM content access, TF/Boot0/TOC1 write, firmware flash, or payload execution.

## 2026-11-28 最新状态校准（本轮复核）

- 当前工作分支 HEAD：`faeb797c6b13d46b3399bfdff700e540d446e20e`。
- Linkslot/LK feasibility [#943](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46086258522)：`cross-compile` job **85/85 recorded steps success**，0 failures、0 skipped；artifact [15239048815](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46086258522/artifacts/15239048815)，387,984,278 bytes。该 run 覆盖 host/cross-build，不是 M1 设备 payload、native firmware/SPL/capsule 或 TF1 镜像的构建/启动证明。
- H700 boot audit [#1601](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46086304755)：Linux `host-tools` 与 Windows PowerShell 两个 jobs 均成功，HEAD 与当前分支一致。
- 当前唯一近期主线仍为 M1。已完成的 probe-scoped Data Abort trampoline、exact matcher、caller-RAM result channel、linked-ELF 检查均为 host/source 层成果；probe 仍 dormant，禁止为推进而提前调用。
- 当前可推进的下一项是**host-only diagnostic candidate packaging/accounting**：将已验证的 A32 dummy/entry/exception 代码作为输入候选，明确区分 raw payload、32 KiB loader slot、BSS/stack/page-table runtime footprint、vendor header 注入及 firmware/FNV/SPL generation coupling；在未闭合 G2/G3、PI12 M1-S1 和 one-shot capsule 闸门前，只生成不可安装的审计报告/候选产物，不写 TF1、不生成可刷写/可启动发布包。
- PI12 仍 default-off；UART0 不再是首发前置条件。M1 采用无拆机、独立 TF1 路线，但“可换卡恢复”不豁免 RTC/WDT、PMIC、PIO、DRAM ownership 或 CPU reset/RVBAR 风险。
- 全局硬件闸保持：`H700_MEMORY_LAYOUT_APPROVED=false`、`hardware_applied=0`；未执行设备 MMIO/SMC、TWI transaction、DRAM 内容访问、TF/Boot0/TOC1 写入、固件刷写或 payload execution。

**仓库：** `dengzhan007/Linkslot-Pico-for-H700`  
**当前显示结论 / 下一门槛：** kernel #19 的 panel init、PLL bitfield、divider 来源与 TCON parent 继承已完成反向审计；UI/GAME presents、armGBA committed frames 与 LCD IRQ 分别实测约 60/s。live profile 为 `rg34xx_v1 + lcd_type=old`，只读回读匹配 60-Hz pair `0x88001703` / `0xf000000b`。60 Hz 是唯一合格模式。120 Hz 已移入 deferred：没有面板完整料号/Revision 与对应 datasheet 前不再投入主线，不做 PLL/TCON 写入、boot-package 写入或绕过 panel allowlist。当前唯一近期交付目标改为 M1：受控的 AArch32 BL33 诊断启动准备；详细门槛见下方“项目主线纠偏与验收门槛”。`hardware_applied=0`。
**工作分支：** `porting/linkslot-1.0.45-feasibility`

## 2026-11-28 continuation — dormant SYS_CFG probe and packaging gate

- Reconciled the roadmap: the probe-scoped AArch32 Data Abort trampoline, exact matcher, caller-RAM result channel, and linked-ELF checks are already implemented and host-validated; earlier duplicate checklist items are now marked complete.
- Re-audited active call-site owners (`target/rg34xx_h700/init.c`, target rules, BL33 entry snapshot, timer and input scan). No call site to `h700_syscfg_ver_reg_low3` and no raw `0x03000024` read appears in startup, input, timer, or entry code. The probe remains dormant.
- The BaseOS native-loader override validates the 32 KiB raw extent, reserved vendor header bytes, entry/vector address bounds, and fixed-size padding. It explicitly marks `loader_elf_matches_loader_bin=false`; this is a builder contract, not a paired deployable diagnostic image.
- Next gate is a separately packaged, byte-accounted diagnostic candidate within the 32 KiB loader extent, including matched firmware/SPL metadata and memory-ownership/rollback review. Do not call the probe or modify TF media until those gates and pre-BL33 CPU reset/RVBAR evidence are satisfied.
- Latest CI at this review: feasibility #941 passed 85/85; boot audit #1597 and #1598 passed Linux and Windows. Feasibility #942 was superseded/cancelled by the next documentation commit before completion; replacement feasibility #943 is running on the updated roadmap baseline. Boot audit #1599 has Linux passed and Windows job running.
- Hardware remains fail-closed: `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`; no device MMIO, module load, TF/Boot0/TOC1 write, firmware flash, or diagnostic execution.

## 2026-11-28 最新续接：活动启动链、TF 启动介质与 CI 对齐

**复核起始代码 HEAD：** `15d431936247b24430adc912c553eedee24fb6a9`（[commit](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/15d431936247b24430adc912c553eedee24fb6a9)）；reports 分支修正前 HEAD 为 `701167dcd8db1eac9baedde00e84c56c46e36ead`，本轮介质纠错提交为 `fcaab72524c473a06235376221bbed8ddb34d50b`。本轮状态/路线图同步提交包括 `8a16da65732ed21d5a59086eb9802bb4812a5cbe` 与 `de7a63e31855eff7affcc305c69ebc38567dc9c4`。新增只读启动链报告：[rg34xx-active-loader-provenance.md](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-active-loader-provenance.md)。该报告依据 2026-11-22 只读库存及固定 BaseOS source pin，属于设备既有库存的离线/静态来源复核，不是 2026-11-28 新实机测试。

**活动启动路径已纠正为：** Boot ROM 从可移动 TF 启动介质加载 128 KiB BaseOS Boot0-primary 外层 capsule；外层 capsule 在活动路径选择 AArch64 native SPL；随后为 vendor BL31 → AArch32 SVC BL33 loader（`0x4a000000`）→ p4+48 MiB BOL4/LZ4 image → AArch64 Linux。嵌入 capsule 的 64 KiB stock eGON Boot0 是 fallback，不是活动 native-SPL 路径上先行的 DRAM-training 阶段。stock TOC1 U-Boot 没有证据表明在当前捕获启动中执行；它仍是兼容/回退材料。

**介质表述纠错：** RG34XX 没有板载 eMMC，系统由 TF 卡启动。reports 报告原示意图中的 “on eMMC boot0 area” 已更正为 TF 启动介质；精确 Boot ROM lookup sector/LBA 与分区选择方式尚未由该报告证明，不能猜测或写成已知事实。

**M1 目标边界进一步明确：** 当前交付不是从 stock TOC1 U-Boot 开始另造启动链，而是在已运行的 BaseOS native-SPL/BL31 路径中，准备受控替换活动 A32 BL33 loader 的最小诊断 payload。活动 p4+40 MiB firmware package 含 monitor/DTBO/DTB/A32 loader；p4+48 MiB 是 native SPL 实际读取的 BOL4 image。更换 32 KiB loader 后必须重建 firmware size/FNV、匹配 AArch64 SPL 与 Boot0-primary capsule，禁止新旧代混配。loader 使用的 `0x8000ff04` 是现有 Linux kernel-entry SMC，不是通用裸机跳转 ABI，M1 不得把它当作 payload entry service。

**CI 核对（2026-11-28，最新复核）：**
- Linkslot/LK feasibility [#937](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46035530557) 在代码 HEAD `f3a815e` 上完整成功，cross-compile 85/85 recorded steps，0 failure/skip/cancel；artifact [15215748657](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46035530557/artifacts/15215748657)。
- 后续完整 feasibility [#938](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46036640672) 在 HEAD `89a9001` 上成功；最新 [#939](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46038164016) 在 HEAD `35b830c` 上也已完整成功：`cross-compile` 85/85 recorded steps success，0 failures / 0 skipped，artifact [15217948756](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46038164016/artifacts/15217948756)（387,984,337 bytes，SHA-256 `0e6e4a2c30bc53287c31c8c141cae131b0a32e28079ae14110fff43870416d77`）。该完整 matrix 包括 mGBA bare-metal `arm-none-eabi/newlib` VFS build、真实 VDir/newlib adapters、Rust static API link、SD/GPT/FAT/exFAT host regressions；均为 host/cross-build，不是设备启动。
- 最新完整 feasibility [#941](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46040292243) 在代码 HEAD `31f4654` 上成功，85/85 recorded steps、0 failures / 0 skipped；#940 仍是前一轮成功基线。
- 完整 feasibility [#940](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46039195103) 也在代码 HEAD `0affe9aee0701073b7cae6c0851c736c816dfba5` 上成功：`cross-compile` 85/85 recorded steps success，0 failures / 0 skipped；artifact [15219333721](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46039195103/artifacts/15219333721)（387,993,754 bytes，SHA-256 `75c1c12d1da4e6ee9b437abc5483f0ea968d80fa75a235a3b76ff4ade9abc87a`）。这是最新一次完整 host/cross-build 复验，不包含真实设备启动。
- RG34XX BL31 static memory audit [#136](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46039667955) 成功：H616 reset-bit、BL31 PSCI CPU_ON callback，以及 SPL/BL31 RVBAR selector parity 0–7 全值测试均通过；artifact [15219895068](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46039667955/artifacts/15219895068)，132,729 bytes。#134 [基线](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46037484953) 亦成功。
- 当前 PROJECT-STATUS HEAD `5e1045e` 触发的 H700 boot-audit [#1596](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46040538689) 已成功：Linux host-tools 与 Windows PowerShell jobs 均通过；#1595 及同批 #1591–#1594 也成功。
- 文档更新后的 H700 boot-audit #1584 [run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46038163980) 与当前 PROJECT-STATUS HEAD 的 #1585 [run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46038172395) 均成功（Linux host-tools / Windows PowerShell）。
- 较早复核起始 HEAD `15d4319` 的 boot audit [#1564](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46031009923) 以及 AArch32/AArch64 compile [#668](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46031162588) 均成功；所有这些都是 host/cross-build/source audit，不是设备启动验收。

**已关闭的 source/ELF 子项：** pinned LK Data Abort vector 在建帧前屏蔽 IRQ/FIQ、保留 68-byte fault frame、对 C handler 临时栈做 8-byte 对齐；overlay 只允许精确 SYS_CFG VER_REG 只读 probe 命中后恢复。PI12 的 Data Abort 不属于该恢复白名单，任何不匹配故障（包括 PI12 读/写 abort）保留 fatal handler 并进入非返回 WFI fail-stop。该路径已有 host/source guard 与 linked A32 ELF 检查，详见 [H700 LK Data Abort recovery audit](h700-lk-data-abort-recovery-audit.md)；最近完整 feasibility #939 的 “Compile/link actual GammaOS LK H700 BL33 overlay CRT0 and exception vectors” step 成功。**边界：** 这关闭的是源代码/链接产物层的安全控制流审查，不是 Cortex-A53 实机故障注入、PIO 总线权限或 abort 硬件行为验证；PI12 没有通用恢复能力。

**M1 A32 selector ABI 与完整 CI（2026-11-28）：** 新增独立 dummy ABI fixture，使用 `clang --target=arm-none-eabi -mcpu=cortex-a53 -marm -ffreestanding` 将 `m1_oneshot_selection_model.c` 编译并链接为 ELF32/ARM；检查公开 selector/fixture symbols 和 `nm -u` 零 unresolved symbols。完整 feasibility [#937](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46035530557) 在代码 HEAD `f3a815e16658533227e0907c8b4079185153ba9a` 上成功；`cross-compile` job 85/85 recorded steps 成功（含 setup/post-job records），0 failures、0 skipped、0 cancelled。新增 A32 fixture 第 15 步、pinned GammaOS LK CRT0/exception-vector link 第 17 步通过；Linkslot/frontend、Libretro/mGBA、SD/GPT/FAT/exFAT host/cross-build matrix 也全通过。artifact [15215748657](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46035530557/artifacts/15215748657)，387,993,785 bytes，包含 maps/ELFs/logs/reports。本 fixture 不执行、也未接入 Boot0 capsule/SPL，不做 SRAM copy，不验证 eGON checksum/entry，不生成配对 firmware/SPL/capsule；它只证明选择策略 AArch32 ABI 可编译链接。详见 [M1 one-shot capsule audit](m1-oneshot-capsule-audit.md) §4.2。

**CPU1–CPU3 / RVBAR 状态分层（2026-11-28）：** 初始 BL33 交接的静态评估为 PROBABLE_NOT_DEVICE_VERIFIED。128 KiB Boot0 primary（SHA-256 `bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f`）的 reset helper 只对 CPU2/CPU3 条件调用（`0x03006248` bit29），`C0_RST_CTRL & ~(1 << core)` 按 H616 家族参考语义保持相应核在 reset；CPU1 不传入 helper。H616 `C0_RST_CTRL` reset default `0x13ff0101` 对 CPU1–CPU3 为 asserted，但 H700 等价及 live value 未读回。pinned SPL 只设置 primary RVBAR。**新增 selector 兼容性硬门：** SPL 规则为 `low3==2` 选择 `0x08100040`、否则选择 `0x09010040`；BL31 后续 CPU_ON helper 为 `low3==0` 选择 `0x09010040`、否则选择 `0x08100040`。两者只在 0/2 时一致；1/3/4/5/6/7 是潜在 RVBAR bank mismatch。目标 selector 未读回，不能将潜在不兼容说成设备故障。全 8 值回归由 [BL31 audit #136](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46039667955) 通过。**重要补充：** 匹配 BL31 的 legacy PSCI ops slot 1 `affinst_on` 会进入 `0x48002d78` CPU_ON helper；该 helper包含 per-core RVBAR 编程和 CPU power/reset 序列。其 SoC 分支由 `VER_REG 0x03000024 & 7` 决定；审计发现一条 CPUCFG/PRCM 路径（`0x09010000/10`、`0x07000440/444`、`0x090100c0`）及另一条 `0x08100000` 区域路径（含 `0x08100020`），但目标 selector 读回失败，不能确认 RG34XX 实际分支。全镜像扫描的 `cpu_reset_mmio_accesses=[]` 只表示没有恢复到固定地址形式的直接命中，不否定这个经过计算地址的 CPU_ON helper。当前没有证据表明该 callback 在最初 BL31→BL33 handoff 自动执行；它证明的是 monitor 后续具备按需 CPU_ON 路径。Linux 启动后 CPU0–CPU3 online 不能回推最初状态。详见 [A53 handoff/memory boundary](rg34xx-a53-handoff-and-memory-boundary.md)、[CPUX live audit](rg34xx-cpux-live-audit-2026-11-25.md) 和 [最新 BL31 audit #136](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46039667955)。

**RVBAR 实机只读探测失败（2026-11-26）：** root ADB 正常，但 /dev/mem 对 0x03000024 的 32-bit pread 返回 EFAULT，0 字节、无 JSON 输出；独立 dd 也返回 Bad address。没有写 MMIO、绕过保护或更改启动参数。2026-11-27 Linux inventory 的 CPU0–CPU3 online 是内核启动后的状态，不能回推 BL31→BL33 handoff。停止重复 userspace /dev/mem 探测；下一次 readback 必须来自可信早期启动捕获，或已映射且权限经过审查的只读 kernel driver。
**下一步（保持硬件 fail-closed）：** 不再重复扫描 Boot0/SPL/BL31 的已知静态路径；下一项工作是寻找可信的早期只读采样来源，以确认 `VER_REG 0x03000024 & 7`、`C0_RST_CTRL`、`C0_CPU_STATUS`、RVBAR 与 DBG 状态；尤其需要确认 selector 属于 SPL/BL31 规则一致的 0/2 类，还是潜在 mismatch 的 1/3/4/5/6/7 类。已证实 BL31 的 PSCI `affinst_on` 回调具备 per-core CPU_ON 序列，但 M1/LK 禁止调用该路径；pinned LK 是 UP runtime，不能释放 CPU1–CPU3。userspace `/dev/mem` 对 selector 读失败，且当前 `iomem` 没有列出 `0x03000000` SYS_CFG 映射，停止重复此路径；只考虑可信 Boot0/BL31 capture 或经确认的现有 kernel regmap read-only 节点。并行继续审查活动 BL33 slot 的 BL31/Boot0 生命周期及候选 stack/BSS/page-table 冲突；APB1/PIO 时钟状态、SPC decode-port/fuse 条件及 BL33-entry Non-secure access 仍未证明。生产 PI12 MMIO wrapper 继续 default-off；在这些硬件 gate 关闭前，不执行物理 GPIO 读写、不启用灯、不写测试卡、不执行 payload。

**硬件总闸不变：** `H700_MEMORY_LAYOUT_APPROVED=false`、`hardware_applied=0`；G1/G2/G3/G6 维持 OPEN。上述报告和 CI 均不构成实机 BL33 启动、DRAM ownership、CPU1–CPU3 hold-off、PIO 权限或 LED 极性验收。

## 2026-11-27 项目主线纠偏与验收门槛（优先级高于下方专项审计日志）

**审查依据：** reports 分支最新报告 [project-goal-alignment-review-20261127](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/project-goal-alignment-review-20261127)，reports HEAD `1c4c00722096ad3f22cf3d888f0ea69ca7ad9a0a)，审查工作分支 HEAD `afdf9c24fd659d179e64a8afdd72f3a43663cadc`。这是只读计划/历史审查，不是设备验收。
**当前 CI（最终核对：2026-11-27，工作 HEAD `1ef014bee82dadc81b921a85e207760d132d9767`）：** Linkslot/LK Rust feasibility **#915**（[run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46009183966)）**82/82 steps success**，无失败；产物 [artifact 15202637382](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46009183966/artifacts/15202637382)，约 388 MB，包含 maps、ELF、日志、overlay 与报告。关键修复闭环：共享 pin guard 通过；累计 Linkslot 1.0.49 overlay 成功导出；基于 clean immutable snapshot 的 `git apply --check` 成功。随后 host/cross-build 全部通过：no_std Frontend、Renderer/SoftwareCompositor、QuickRow fail-closed、输入 ABI/queue、Libretro VFS v3、mGBA AArch32/newlib archive 与 Rust static API `--no-undefined` 链接、BL33 AArch32 header/vector/layout probe，以及 SD/GPT/FAT/VFS host 只读路径。H700 Linkslot runtime-slices **#134**（[run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46009183796)）两个 jobs 也全部通过（14/14 + 7/7）。H700 boot audit **#1520**（[run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46008996376)）Linux/Windows 两端通过，各运行 **102 tests，OK**；软件 renderer #140（[run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46008198962)）13/13 host tests 通过。此前 #903 唯一失败点（overlay export 脚本残留旧 SHA）已由 shared pin 文件及 final export guard 修复，#915 对该步骤复验成功。**边界：以上全部仍是 host/cross-build；不是完整 LK target image/实机启动或设备验收。**
**基线事实源：** 当前唯一稳定 Linkslot 源码基线为 main 1.0.49；`porting/linkslot-1.0.45-feasibility` 是历史工作分支名，不表示源码仍 pin 在 1.0.45。版本依据见 [upstream-baselines.md](upstream-baselines.md)。

**上游 Linkslot 本轮复核（2026-11-27）：** 只读快照分支 `upstream/linkslot` HEAD 为 `6122c8f2357ecae43295fdb1a158ff5ef7488325`。新 `UPSTREAM-SNAPSHOT.md` 已核实：GitCode `main` 仍为 1.0.49 / `b84f37f`，851 个文件，较上一快照逐字节无变化；`test` 前进至 1.0.60 / `b1342be`，854 个文件，较上一快照 85 个文件变化、3 个新增；`DEV` 仍为 1.1.0 / `ebc6094`，875 个文件，较上一快照 57 个文件变化、3 个新增。main 与 test 不再相同；test 1.0.60 changelog 包含 Web 批量导入/导出和菜单图标变化，1.0.50–1.0.60 还涉及较大范围 UI/HUD/通知/传输界面调整。故 stable baseline 继续锁定 main 1.0.49，不直接将 test 1.0.60 或 DEV 1.1.0 导入首期。已更新 [upstream-baselines.md](upstream-baselines.md) 记录新快照、原始 commit 与文件差异；本次只是来源/基线审查，不是 H700 功能移植或设备验证。

**M1 UART0 只读准备与 CI（工作 HEAD `e5bc9924b2d2f1c331fe44d12be4a5010d0652f7`）：** 新增 UART0 Device/XN translation section 与 host descriptor regression 的提交为 [`3a4279b`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/3a4279b6cd1d20be1e2ed27d725410167905234f)；只读库存采集器新增 `uart-runtime.txt` 的提交为 [`e5bc992`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/e5bc9924b2d2f1c331fe44d12be4a5010d0652f7)。H700 boot-audit #1489（[run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45985687068)）Linux/Windows 两个 job 全部成功；AArch32/AArch64 compile #658（[run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45985687185)）全部成功。完整 feasibility #895（[run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45985179518)）在前一代码 HEAD `3a4279b` 上最后检查为 64/80 steps、无失败、仍运行；这些工作结果全是 host-only，不关闭任何硬件 gate。

### 2026-11-27 无拆机 / TF1 首启路线（当前执行策略）

用户确认 RG34XX 没有板载 eMMC 系统盘，系统由 TF 卡启动。项目不再把拆机找 UART 焊盘作为首次实机验证前提：保留原系统卡，只用另一张独立 TF1 测试卡验证启动。无 eMMC 表示测试卡上的引导内容可通过更换介质恢复；不代表电气操作、SoC fuse/OTP、PMIC、错误电压或任意 MMIO 没有风险。

- **首发 proof-of-life：PI12 工作灯优先，UART0 降为备用。** RG34XX 非 SP boot DTS 将 PMU work_led 映射到 bank I / PI12、low_pwr_led 映射到 PI11；仓库已有 PI12 服务与 fake-register tests，read-modify-write 保留 PI11 和其他位。生产物理 wrapper 仍 default-off；Data Abort 的 source/linked-ELF fatal fail-stop 已有验证，但 BL33-time APB1/PIO 权限及实际 LED 极性仍 OPEN，且没有 A53 实机故障验证。
- **M1-S1 改为 PI12 资格化，不再等待 UART 焊盘。** 不拆机、不焊线、不连接 UART。BL31 的 pre-BL33 security routine 静态上尝试开放 SPC decode ports 与 CCU/R_PRCM Non-secure access，但 H616 TF-A 参考警告 fuse 条件可能使 SPC 写入不生效；目标 fuse、PIO decode-port 对应、APB1 clock 和有效权限仍未知。LCD/framebuffer 延后，因其 ownership 尚未批准。
- **APB1→PIO 源码证据已进一步收敛但不解锁 PI12：** 除 U-Boot `sun50iw9p1` clock provider 外， pinned `qiaoweibiao/T507_Kernel` commit `98fcc7d3…` 的 `sun50iw9p1-pinctrl.dtsi` 使用与目标 live FDT 完全相同的 `allwinner,sun50iw9p1-pinctrl` compatible，并将 PIO 接到逻辑 `pio` clock；同树 pinctrl probe 会 enable 该 clock，clock provider 将 `pio` 连接至 `apb1`。APB1 候选寄存器为 `0x03001520`，该 provider 没有独立 PIO module gate/reset。该 kernel source 仍未证明就是本机 Linux 4.9.170 的精确 build source；目标 BL33-entry APB1 值、SPC fuse 影响及有效 NS access 仍没有读回。保持 PI12 default-off。详见 `h700-bl31-pio-ccu-audit.md`。U-Boot `clock_get_apb1()` 只读该寄存器，`clock_init_uart()` 只配置 APB2/UART，未发现该源码路径对 APB1 的初始化写入；Boot0 是否提前配置仍未知。
- **M1 capsule 独立闸门仍 OPEN。** BaseOS raw-capsule v2 会写 RTC GP14/GP15 并启动 watchdog，watchdog 跨 BL31 RMR3 的有效性及早期 refusal recovery 尚未实机资格化；现有 diagnostic capsule 只验证旧 inner 的外层协议，不是 M1 LK 启动镜像。M1 不默认授权 RTC/WDT 写入，需设计无 RTC/WDT 的专用 one-shot capsule（手动换卡恢复），或另行资格化 v2 协议。 纯主机 one-shot 选择模型已在 #931 通过 6/6：M1 SPL 不合格时选择内嵌 stock Boot0，stock 本身不可信时 fail-stop；模型尚未接入真实 capsule/SPL。 现已新增 [m1-oneshot-capsule-audit.md](m1-oneshot-capsule-audit.md)：确认普通 SPL 路径会写 0x56000/0x56200 并配置 WDT；raw v2 仍依赖 GP14/GP15 + WDT，native handoff 还有 PMIC/CCU/显示 GPIO/WDT 副作用；需要外层、SPL、native handoff 三段独立 M1 专用实现。
- **M1 设备试验采用 BL33-slot-only + CPU0-only 边界。** Loader 输入 extent 为 32 KiB；#921 map 已测得 8 KiB early stack `[0x4a004000,0x4a006000)`、8 KiB gap、16 KiB 页表 `[0x4a008000,0x4a00c000)`、`_end=0x4a00c024`，CI 已验证栈/页表不重叠。它不代表 BL31/Boot0 所有权已批准。A32 BL33 payload 本身不使用外部应用 DRAM、heap、DMA、GIC/timer/IRQ、SMP、PMIC/TWI 或 CPU PLL；但前置 A64 native SPL 现有路径调用 `sunxi_board_init()`，按固定 defconfig 执行 AXP717/TWI、DCDC2=940 mV、DCDC3=1100 mV、LPDDR4 DRAM training 与 CPU PLL setup。严格 no-PMIC/TWI 版本不能跳过它后假设 DRAM 已就绪；较低差异的候选是源/配置不变地复用当前部署的 native-init baseline，并在 manifest 明确标注 PMIC/TWI 访问、无额外电压/PMIC写入。该 profile 尚未选定或获得实机授权。
- 全局 H700_MEMORY_LAYOUT_APPROVED=false 与通用 H700_MEMBASE/H700_MEMSIZE gate 保持不变；M1 局部 slot profile 不批准完整 1 GiB DRAM 或 Linkslot/mGBA arena。
- 仅允许向独立可擦写 TF1 测试卡写入经 hash/manifest/extent/分区/sector-range 检查的 M1-TEST-TF1-ONLY 配对镜像；原系统卡不写。失败后关机换回原卡。SoC fuse/OTP 与未审计的外设访问仍禁止。若采用当前 native-init baseline，必须明确列出既有 AXP717/TWI/DCDC2/DCDC3/DRAM/CPU-PLL 操作并保持与已部署 source/config 完全一致；禁止新增或改变电压/PMIC设置。若要求零 PMIC/TWI，则必须先提供不依赖该序列的经验证 DRAM 路径。
- **配对格式契约：** standalone `package-h700-bl33.py` 输出（已带 vendor header）与 BaseOS `BASEOS_LK_BL33_RAW` 输入（header 保留区必须为零）不可互换。后者重建 p4+40 MiB firmware 后，必须同步重建引用新 firmware size/FNV 的 AArch64 SPL，并与 Boot0 capsule 配对。当前只验证了工具边界和 host CI，未实际重建 M1 firmware/SPL/capsule。
- 完整方案：[no-disassembly-tf-bringup-plan.md](no-disassembly-tf-bringup-plan.md)。当前 M1-T0 的代码布局子项已在 host map/CI 层通过；剩余重点是 PI12 handoff clock/security/firewall、LED 极性、BL31/Boot0 的 slot/lifetime 交叉审计和 CPU1–CPU3 初始状态。不是直接启用生产 PIO wrapper。

**最近完整 CI：** Linkslot/LK feasibility #931 **84/84 steps 成功，0 failures / 0 skipped**（[run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46031541947)，artifact [15215206088](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46031541947/artifacts/15215206088)，约 388 MB；SHA-256 `0f6c6294f43fb734317be550815da2aa5bd2d89451e3c3e66e1b35025c1b4f6a`）。#931 包含固定 BaseOS 副作用审计、32 KiB loader 边界、LK overlay/link、Linkslot/mGBA/VFS/SD host regressions，以及新增的 M1 capsule pre-entry selector 6/6 host-only model。它没有生成真实 M1 capsule/SPL 或可启动测试卡镜像。AArch32/AArch64 compile #668 成功（[run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46031162588)）；boot audit #1564 Linux/Windows 成功（[run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46031009923)）。#924 的实际 map 断言 early stack `[0x4a004000,0x4a006000)`（8 KiB）、页表 `[0x4a008000,0x4a00c000)`（16 KiB）、8 KiB gap 且不重叠；这些仍是 host-side 证据，不代表实机启动或 BL31/Boot0 内存所有权获批。

### 当前验收门槛台账

| ID | 验收项 | 状态 | 当前证据 / 阻塞 |
|---|---|---|---|
| G1 | 完整 H700 LK target 与可验证的 BL33 启动链 | **OPEN** | 仅有 CRT0/异常/MMU 等 host layout/link probes；没有完整 LK 实机启动。32 KiB native-loader patcher 通过 host 测试，但诊断 payload 尚未产出。 |
| G2 | BL33 image/BSS/stack/heap/page-table 与应用 DRAM 所有权 | **OPEN — fail-closed** | BL31/Boot0 生命周期、未知保留区与 cache/DMA 所有权未闭合；不得设置 `H700_MEMORY_LAYOUT_APPROVED`、`H700_MEMBASE` 或 `H700_MEMSIZE`。 |
| G3 | BL31→BL33 时 CPU1–CPU3 reset/hold-off 与 RVBAR 状态 | **OPEN** | 目前只有 Boot0/BL31 静态证据；没有目标设备 pre-BL33 live readback。PSCI CPU_ON callback 存在不代表初始交接已启动或停驻多核。 |
| G4 | BL33 自有显示 scanout/framebuffer 与面板点亮 | **PARTIAL** | Linux 运行态 60 Hz 有只读证据；BL33 framebuffer ownership、scanout 初始化和实机画面尚未验收。 |
| G5 | 物理按键到 H700 HAL 输入事件 | **PARTIAL** | Linux evdev/gpiolib 17/17 键映射已验证；裸机 PIO/IRQ 输入驱动与 Linkslot 接线未完成。 |
| G6 | BL33 GIC/timer/PPI handoff 与 IRQ 调度 | **OPEN** | GIC/timer 静态映射及 host tests 不证明安全态、Group ownership、PPI handoff 或 timer 连续性。 |
| G7 | PCM DMA 音频 | **NOT STARTED** | 尚无 H700 裸机音频 DMA 路径。 |
| G8 | 裸机 SD 读写及断电一致性 | **PARTIAL** | CMD17→GPT→FAT/VFS 为 host-tested；设备侧控制器初始化、实际读写、flush/barrier 与断电一致性未验收。 |
| G9 | PMIC 完全关机（ADR-003） | **DEFERRED / OPEN** | BL31 suspend/TWI 静态链不是设备 transaction；PMIC 命令与完整关机动作未知。按路线图放在基本启动/功能之后。 |
| G10 | 温度传感器校准与可信 thermal input | **OPEN** | AVCC/温度校准未闭合。 |
| G11 | 板级批准的 OPP/电压频率表 | **OPEN** | 不将 Linux DT OPP 或 host CPU policy 当作可安全执行的板级 OPP。 |

**硬件总闸保持：** `H700_MEMORY_LAYOUT_APPROVED=false`，`hardware_applied=0`。未授权任何设备 MMIO/SMC、TWI transaction、DRAM 内容访问、Boot0/TOC1/SD 写入、固件刷写或 payload execution。

### 唯一近期主线：M1 — 小型 AArch32 BL33 诊断启动准备

M1 先完成规格、proof-of-life 信号资格审查和 host-only 产物验证。32 KiB 是 loader 输入文件上限，不是运行时 BSS/stack/heap/page-table 的内存许可。M1 不包含完整 Linkslot/mGBA，不允许通过临时设置内存总闸来“先跑起来”。详见 [M1 BL33 diagnostic specification](m1-bl33-diagnostic.md)。

1. **M1-S0（本轮）：** 固定诊断范围、入口快照、信号资格、构建边界和禁止事项。
2. **M1-S1：** 仅凭匹配 RG34XX 板型的原理图/boot source/只读证据，确认确定性的 proof-of-life 路径。UART0 目前只是候选；不得根据 Linux bootargs 或其他 RG34XX 变体资料直接写 UART/PIO。PI12、LCD 与 SD 也不是默认安全替代方案。
3. **M1-S2：** 信号路径审查通过后，生成最小 A32 诊断 payload，复用现有 BL33 entry snapshot；执行 A32 编译、ELF/map/BSS/stack/layout 和 raw-size 检查。所有结果限 host。
4. **M1-S3：** 在隔离的 pinned BaseOS checkout 中构建匹配 native SPL/firmware/capsule，核验 loader extent/header/FNV/文件长度和来源；输出明确标记为 `NON-INSTALLABLE / NOT FOR DEVICE`。不得写入 SD 或设备。
5. **M1-S4：** 设备运行不是本轮授权事项。只有 G2/G3 与相关 handoff/恢复门槛经证据审查关闭、proof-of-life 信号经过实机接口资格确认，并获得项目所有者对本次具体测试的明确授权后，才可单独规划。


**M1-S1 UART0 候选证据更新（2026-11-27 实机运行态已采集，资格仍未通过）：** 用户已在目标 RG34XX（非 SP）上运行更新后的只读库存采集器，并将完整 42 文件报告上传至 [reports/rg34xx-inventory-20261127-uart](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261127-uart)；reports HEAD 为 `7217907f57408b714c55a872c6afc5cfc34fdac9`。报告确认 TOC1 package SHA `e6c20e3a...`、monitor SHA `676bc020...`、TOC1 DTB SHA `c28c8738...` 与此前采集相同；live FDT 仍为 573 nodes（其 per-boot chosen bootargs/hash 会变化）。新 `uart-runtime.txt` 显示 Linux bootargs 为 `earlyprintk=sunxi-uart,0x05000000` / `console=ttyS0,115200`；`ttyS0` sysfs 指向 `/sys/devices/platform/soc/uart0`（dev 248:0）；现有 debugfs 中 PH0/PH1 均标为 function `uart0`，pinconf 为 pull-up / 20 mA drive；`apb2` 与 `uart0` clock summary 均为 24 MHz。解释边界：`GPIO UNCLAIMED` 仅表示 GPIO request 未被占用，不否定 pinmux function=`uart0`；但 `/proc/tty/driver/serial` 不可读，故没有 line/FIFO/divisor 寄存器级回读；dmesg 六条 `expect_func as:uart0_jtag, but muxsel(3)` 告警的序列为 `jtag,jtag,uart0,jtag,uart0,jtag`，与固定 sun50iw9p1 参考 DTS 中 `sdc0_pins_c` 的 PF0–PF5 / muxsel 3 组合一致（PF0/1/3/5 映射为 `jtag`，PF2/4 映射为 `uart0`）。同族 pinctrl driver 在 DT group 名称与单 pin function 名不一致时发出 warning，但仍按每个 pin 的实际 function 建立 map；因此这更可能是 PF0–PF5 组合组的命名告警，而不是 UART0 PH0/PH1 上的 JTAG/UART 冲突。本轮已直接解析目标机报告中的 redacted `live-fdt.dtb`（573 nodes）：`/aliases/serial0` 指向 `/soc@03000000/uart@05000000`；UART0 node 为 `allwinner,sun50i-uart`、`status=okay`、`reg=0x05000000/0x400`，clock phandle `0x29` 解析至 `/clocks/uart0`。`pinctrl-0`/`pinctrl-1` 分别指向 `uart0@0`（PH0/PH1、function `uart0`、muxsel 2）和 `uart0@1`（PH0/PH1、function `uart0`、muxsel 7）。同一 live FDT 中还直接找到告警来源组 `/soc@03000000/pinctrl@0300b000/sdc0@2`（phandle `0x95`）：PF0–PF5、group function `uart0_jtag`、muxsel 3；这与六条 dmesg warning 的 per-pin function 序列吻合。因此告警的 FDT 组来源已找到，不再只是参考 DTS 推测；但 DT 声明不是 live PIO register readback，也不能证明该组此刻已电气生效。报告明确是 Linux post-boot view，不是 BL31→BL33 handoff snapshot。报告附带 `REDACTION.md` 与 `SHA256SUMS.txt`；发布前扫描记录 0 device serial、0 MAC-shaped value、0 workstation path，redacted DTB 仍可解析为 573 nodes。采集未访问 `/dev/mem`、未做 MMIO、未挂载 debugfs、未写存储，`hardware_applied=0`。因此这次采集实质性补齐了目标设备 Linux 运行态的 UART0/PH0/PH1/24-MHz 证据，但不能证明这些状态在 BL33 entry 时保留，也未确认本机实际 test-pad、电平/连线、BL31 安全/总线访问权限。另对固定 BaseOS source pin `cbe253a43f46cf551cc9a6c79a84015bde3686cf` 的 A32 entry/native loader/SPL handoff 限定文件做了静态复核：未发现 UART 初始化/console/MMIO 引用，`loader.c` 中 `serial` 仅指 SID 序列号字符串；该负面结果不覆盖 U-Boot 公共代码、Boot0/BL31 或无法完全重建的部署 SPL binary。M1-S1 仍 OPEN，禁止新增 UART 写代码、CCU 操作或 pinmux 修改。

**M1-S1 更新（2026-11-27）：** UART0 已取得从 H616 SPL 到 exact vendor BL31 的静态初始化/复用路径，以及 BL31 SPC/CCU/PRCM/DMA security setup 调用路径。此项将 UART0 从“仅 Linux post-boot 有证据”升级为“boot-chain source/binary 静态路径已定位”，但 target fuse/SPC 有效权限、BL33 入口 live register、PCB test-pad 与电气路径均未确认，故 M1-S1 仍 **OPEN / NOT APPROVED**。不能据此开始新增 UART write、PIO 改动或诊断 payload。详见 [UART handoff audit](rg34xx-uart-bl31-handoff-audit.md)。

**物理板级证据补充（2026-11-27）：** 2026-06-17 制造商签署的 FCC model-difference declaration（document 9400819）明确列出 RG 34XX，并声明与 RG34XXSP 等型号共用 PCB/电路/结构/内部；FCC Internal Photos document 9400823 第 3/4 页展示了 SP 样机 H700 主板正反面。但没有 PH0/PH1 test-pad/net 对照，也没有当前非 SP 实机 PCB revision 的直接识别。因此物理 UART 焊盘仍是 M1-S1 的未闭合项；不能直接套用 SP 的焊点说明。详见 [UART handoff audit](rg34xx-uart-bl31-handoff-audit.md)。
**UART pin/function 独立交叉核对（2026-11-27）：** 社区 RG34XX-SP 参考仓库 `fotlim/android_device_anbernic_rg34xxsp_h700_lineage` 的 stock 硬件记录和解码 DT 也将 UART0 映射到 PH0/PH1，serial0=`0x05000000`，115200 8N1。该证据只交叉确认 SP 侧 pin/function/console 定义；仍没有 test-pad、TX/RX/GND 焊点、电平或非 SP 实板连通证明，不改变 M1-S1。详见 [UART handoff audit](rg34xx-uart-bl31-handoff-audit.md)。

### 冻结项与审计停止规则

- **120 Hz：** 移入 deferred。60 Hz 是当前唯一合格模式；只有取得实际 LCD 模组料号/Revision 与对应 datasheet，才重新开启 panel qualification。
- **BL31 PSCI suspend/TWI / PMIC 命令链：** 保留 #132 已取得的静态结论，不继续无期限增加 scanner。待基本 BL33 启动和可观测性建立后，再按 ADR-003 恢复。
- **CPU OPP/DVFS、完整音频 DMA、上层无 I/O 业务扩展：** 暂停新增功能；保留现有 host-tested 代码，不把其 CI 通过计入硬件阶段验收。
- **静态审计停止规则：** 同一未解决子问题连续 3 轮审计没有新增原始证据、没有改变任何 G 门槛状态时，标记为 `DEFERRED / NOT OBSERVABLE`；不得仅为增加命中数而新建 scanner。只有能解除某一明确 gate 的审计才继续。
- **阶段推进规则：** 每轮以一个可验收的垂直切片为单位。纯 host/tooling/documentation 轮次标记为 `audit-only`，不得宣称硬件 bring-up 前进；不为满足提交数量而制造 platform/target 变更。

本节优先级高于下方按日期排列的审计续接日志；后续日志只追加证据，不自动改变 G 门槛或解冻 deferred 项。



## 2026-11-27 最新续接：RTC selector producer 与 SRAM relocation 已闭合（BL31 audit #132）

- [x] 建立 `scripts/audit-rg34xx-bl31-twi-relocation.py`：固定 monitor SHA `676bc020...` 与原始 TOC1 DTB SHA `c28c8738...`，解析 FDT alias、parent device_type、producer code、context staging 与 relocated table.
- [x] 建立 host test 覆盖 FDT parser、twi5 selector、AArch64 ADRP/ADD/BL 解码、standby copy relocation 和 table/context source-equivalent 地址。
- [x] 将测试与真实审计接入 `.github/workflows/h700-bl31-memory-audit.yml`；#132 的两个新 TWI 步骤均成功，完整 BL31 audit job 全通过；artifact [15187681890](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45977266644/artifacts/15187681890)。
- [x] 在精确匹配的原始 TOC1 DTB 中确认 `pmu0` alias 指向 AXP2202 子节点，父控制器 `device_type=twi5`；BL31 selector producer 将末尾数字转换为 5。
- [x] 证明 selector 的生产与复制：PSCI suspend callback `0x48002998 → 0x48001790 → 0x480012cc` 写 external context `0x4803cbb4`；0x30-byte staging copy `0x4803cb90 → 0x20080` 将其复制到 `0x200a4 = SRAM context +0xa4`。
- [x] 正确处理 standby code copy 的 ADRP rebasing：source `0x4800c220..0x48015630` → SRAM `0x20000`；runtime lookup table `0x28628` 对应 source `0x48014848`；index 5 qword `0x48014870` 为 `0x07081400` (R-TWI0)。context pointer slot runtime `0x28668` 对应 source `0x48014888`，其值为 `0x20000`。
- [x] 修正以前把 link-time 0x48014628/0x48014668 当成 copied code 活动地址的错误解释；具体更正见 [RTC/TWI 专项审计](rg34xx-rtc-soft-entry-twi-audit.md) 4F。
- [ ] 静态追踪 R-TWI0 transfer caller 的参数/状态分支，定位 PMIC slave address 与命令字节；继续追踪传输后返回/自循环与 GP_DATA2 consumer。只做静态分析，不发起 TWI transaction。
- [ ] 对 suspend callback 的条件执行、实际 context 运行时值和硬件 suspend/wake 保持未验证标记；不得将静态 selector 当作现场 transaction trace。

`H700_MEMORY_LAYOUT_APPROVED=false`、`hardware_applied=0`；没有执行实机 SMC/MMIO、I2C/TWI transaction、DRAM 内容读写、待机/唤醒或固件刷写。


## 2026-11-27 最新续接：MOVZ/MOVK 与 literal-pool 地址构造覆盖（BL31 audit #126–#127）

- [x] 在 bounded CFG seed 模型中加入 `MOV/MOVZ` 立即数片段、`MOVK` halfword 更新，以及指向 context-like slot/邻近 monitor data page 的 `LDR literal` seed。
- [x] 新增 synthetic AArch64 regression：`MOVZ x20,#0x4668` + `MOVK x20,#0x4801,LSL #16` 拼出 `0x48014668`，随后加载 context pointer 并合成写 `+0xa4`；另测试 literal pool qword 为 `0x48014668` 的同一路径。
- [x] BL31 audit #126 [run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45972875430) 成功；#127 [run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45972935686) 的新增 synthetic tests 与完整 audit 均通过；artifact [15185123383](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45972935686/artifacts/15185123383)。
- [x] 真实 monitor 结果未变化：ADR seeds=0、MOV-immediate seeds=0、literal-load seeds=0，ADRP seeds=79；38,676 CFG states，83 path hits / 22 unique load PCs，selector/global-slot store candidates 仍为 0。
- [ ] 继续检查 MOVN/逻辑立即数、栈 spill/reload、BLR 间接调用、helper/memcpy 复制和其他外部初始化路径；这些尚未被完整建模。
- [ ] 确认 `0x48014668` 作为 lookup index 8 的重叠数据槽，在 runtime 是否会被覆写或由外部初始化；不要把它当作已证明独立变量。
- Hardware remains unchanged: `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`; no SMC/MMIO, DRAM content access, I2C/TWI transaction, suspend/wake, Boot0/TOC1 write, flashing or payload execution.

## 2026-11-27 最新续接：RTC context alias/CFG 写入追踪（BL31 audit #121–#125）

### 扫描范围与结果

- [x] 在固定偏移扫描基础上加入 alias-aware linear pass：追踪 `ADRP/ADD/SUB/MOV/LDR`，保留条件分支 fall-through，报告 15 个 context-like slot load PC；未发现 selector 或 slot store candidate。
- [x] 加入 bounded CFG pass：种子为 ADRP target page `[0x48013000,0x48015000]`，探索条件分支两侧和直接 `BL` callee，传递 `x0–x7` 参数及 `x19–x29` callee-saved aliases，支持 shifted ADD/SUB immediate。
- [x] 最新真实 monitor 报告（#125）：79 seeds、38,676 states、83 条 slot load path hits，对应 22 个唯一指令 PCs；`context +0xa4` alias stores=0，context-like slot stores=0。
- [x] CFG 比先前 fall-through pass 多找到条件分支目标 `0x48013e44` 与 `0x48013f5c` 的读取路径；22 个 unique PCs 是去重后真实读取指令位置，83 是不同 seed/path 的重复命中计数。
- [x] #121 [run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45970984518)、#122 [run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45971150288)、#123 [run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45971482861)、#125 [run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45971835320) 均通过；#125 artifact [15184548909](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45971835320/artifacts/15184548909)。
- [x] #124 [run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45971516613) 仅因 synthetic ADRP fixture 使用 link-at-zero 编码却在 `BASE=0x48000000` 解码而失败；已改为 monitor-base-relative encodings，并由 #125 全量复验成功。

### 重要地址布局修正

- `0x48014668` 的 raw qword 是 `2`，且它恰好等于 lookup origin `0x48014628 + 0x40`，即 signed lookup index 8。
- 因此不要再将它表述成已证明独立于 lookup data 的 global variable；准确名称为 **context-like pointer slot overlapping the lookup-data region**。它在运行时是否被覆写、是否另有结构性用途，目前都未确定。
- signed indices 68–73 的 H616 TWI0–TWI4/R-TWI0 base map 仍在同一数据窗口后段；R-TWI0 index 73 依旧是条件候选，实际 selector 未捕获。

### 结论边界与后续

- bounded CFG 当前覆盖 ADRP 页构造、部分 ADD/SUB、MOV/LDR、条件分支与直接 BL 调用；未覆盖 BLR 间接调用、完整 return-state merge、栈 spill/reload、所有 literal-pool/MOVZ/MOVK 构造、helper/memcpy、register-indexed store 和其他组件写入。
- 因此“没有 writer hit”只是当前扫描模型下的结果，不是不存在 runtime writer 的证明；仍需继续查别名、stack spill、helper copy 和外部 loader contract。
- selector 仍未知；不发起 TWI transaction，不猜 PMIC slave address/command。A53 reset、active retention PFN、BL33 RAM ownership、PIO 权限、timer/PPI 等其他硬件门槛照旧保持关闭。

`H700_MEMORY_LAYOUT_APPROVED=false`；`hardware_applied=0`。

## 2026-11-27 最新续接：RTC selector writer 全 monitor 审计与 TWI base-map（BL31 audit #109–#114）

### 全镜像扫描结果

- [x] 增加逐 4-byte word 独立解码的完整 monitor 扫描，覆盖 26,804 aligned slots，解码 24,943 instruction-shaped words；单个无效/数据 word 不会提前终止扫描。
- [x] 固定四个 RTC context `+0xa4` 直接读取位置：`0x48012520`、`0x480125d4`、`0x48012694`、`0x480126d4`；#114 的精确指令断言和生成报告均覆盖四处。
- [x] 检查所有写入区间与 selector bytes `[+0xa4,+0xa8)` 的重叠。共发现五个 instruction-shaped 候选：`0x4800fce4`、`0x48010704`、`0x4801571c`、`0x48015858`、`0x48015930`。
- [x] 逐函数和 base-register 追踪后，五个候选全部被排除为 RTC selector writers：两处是 X29=SP 的栈帧局部字段；一处是保存 `DACR32_EL2/IFSR32_EL2` 的 CPU architectural-context block；两处是 exception-entry stack saves。未留下未审查命中。
- [x] 对 global context-like slot `[0x48014668,0x48014670)` 的 immediate-offset scan 得到 22 个 load candidates、0 个 overlapping store candidates；raw initial qword 为 2，实际 runtime producer/initializer 仍未知。
- [x] BL31 audit #111（[run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45968594594)）、#112（[run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45968869354)）、#113（[run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45969019644)）、#114（[run](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45969225655)）全部成功；最新 #114 artifact [15183309990](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45969225655/artifacts/15183309990)。

### 解释边界

- 目前没有找到 RTC context `+0xa4` 的真实 writer；这只是当前 exact-matched monitor 的 aligned-word、immediate-offset、已识别基址扫描结论，不证明 indirect/helper/external writer 不存在。
- 目前没有找到 global slot `0x48014668` 的 immediate-offset writer；不能因为 raw initial qword 为 2 就推断实际运行时 pointer 值。
- context selector 仍是未知 signed 32-bit 值。index 73 的 `0x07081400` 与 live-FDT AXP2202 parent 相符，只是强条件候选；没有 selector runtime evidence，就不能宣布实际选中 R-TWI0。
- PMIC slave address、命令序列和最终 power action 仍未知；不做任何 TWI 设备试探。

### 下一步

1. 继续追踪 global context-like slot `0x48014668` 的别名/预计算指针写入路径和可能的 helper-copy/初始化调用；当前 offset-only scan 尚不能证明不存在这些路径。
2. 审计 RTC context producer 是否位于外部 loader、其他 BL31 initialization helper 或调用者提供的数据结构；需要能将 context base 与 `+0xa4` 值明确关联的静态证据。
3. 若静态路径无法闭合，仅考虑已有可信、只读的 pre-suspend capture；不发出 TWI transaction，不读取/写入未知 MMIO。
4. A53 reset、active retention PFN、BL33 RAM ownership、APB1/PIO access 和 timer/PPI continuity 仍是独立硬件门槛。

`H700_MEMORY_LAYOUT_APPROVED=false`；`hardware_applied=0`。

## 2026-11-27 最新续接：H616 TWI0–TWI4/R-TWI0 静态 base-map 收敛（BL31 audit #99–#103）

### 证据

- [x] 对 exact-matched monitor 在 lookup origin `0x48014628` 后的 bounded data window 自动扫描，发现有符号 index 68–73 的连续六个 qword 对应 H616 family TWI0–TWI4 / R-TWI0 controller bases。
- [x] 已建立可重复 H616 family label map：`0x05002000` TWI0、`0x05002400` TWI1、`0x05002800` TWI2、`0x05002c00` TWI3、`0x05003000` TWI4、`0x07081400` R-TWI0。
- [x] 目标 live FDT 报告 `reports/rg34xx-inventory-20261126-timer/live-fdt.json` 的 AXP2202 PMU parent 节点也位于 `twi@0x07081400`，与静态 index 73 候选一致。
- [x] 精确 H616 family 手册 TWI register offsets 与 BL31 访问字段一致：`+0x08 DATA`、`+0x0c CNTR`、`+0x10 STAT`、`+0x14 CCR`、`+0x18 SRST`、`+0x1c EFR`、`+0x20 LCR`。
- [x] BL31 audit [#99](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45966118995)、[#100](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45966317624)、[#101](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45966329671)、[#102](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45966659172)、[#103](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45966678309) 均通过。
- [x] 最新完整映射报告由 #103 产出：[artifact 15182837507](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45966678309/artifacts/15182837507)。H700 boot audit #1463 对映射说明文档通过；#1464 验证本次 TWI 专项文档更新。

### 正确解释与边界

- BL31 使用 `LDR X1,[X1,W0,SXTW #3]`；W0 是从 context `+0xa4` 读取的有符号 32-bit index。`lookup origin + index*8` 决定实际所选地址。
- index 73 的静态 qword 是 `0x07081400`；**仅当运行时 `context +0xa4 == 73` 时才会选中 R-TWI0**。实际 selector 未采集，因此 R-TWI0 是强静态候选，不是已确认运行总线。index 68–72 是同一序列中的标准 TWI controller alternatives。
- 这组映射已从“未知的 selector lookup”收敛到“静态 H616 family TWI controller base map”，但不证明 H700 每个寄存器语义与 H616 完全相同。
- PMIC slave address、寄存器/命令、GP_DATA2 阶段标记消费者和最终 power action 均未确认；禁止发起 TWI transaction。

### 下一步

1. 继续追踪 RTC soft-entry caller 如何构造 context 以及 `+0xa4` 的 producer/赋值来源；搜索 direct/indirect store、拷贝和 runtime mutation，不要仅凭缺少 direct xref 推断没有 writer。
2. 尝试从已有可信只读采集或源代码契约确认 actual signed selector；优先确定是否为 73。不得使用设备侧 I2C/TWI 试探。
3. 在 selector 与 TWI 命令链闭合之前，不能给 PMIC 关机动作定性。
4. A53 reset、active DRAM retention PFN、BL33 RAM ownership、APB1/PIO access、timer/PPI continuity 仍是独立硬件门槛，不能因 TWI map 成立而放行。

`H700_MEMORY_LAYOUT_APPROVED=false`；`hardware_applied=0`。

## 2026-11-27 最新续接：RTC TWI descriptor 初始化边界与 BL31 audit #96

### CI 结果

- [x] BL31 static audit [#96](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45965474745)：审计脚本 HEAD `f4676657ff0864e9ecd01878c59886798ac1ea11`，`audit-matched-bl31` job 成功；新增 TWI test 与 TWI descriptor audit 步骤通过；产物 [15182980313](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45965474745/artifacts/15182980313)。
- [x] H700 boot audit [#1459](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45965447342)：Linux host-tools 与 Windows PowerShell 均通过。
- [x] 完整 Linkslot/LK feasibility [#887](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45964118596)：cross-compile job 成功；该 run 对应 SHA `4934c503...`，之后提交只改审计/CI/文档。
- [x] #95 的新增审计脚本断言错误已修正：`0x480126d4` 的实际指令基址寄存器是 `x0`，不是 `x19`；#96 完整复验成功。

### TWI 初始化边界更新

- 匹配 monitor 的 RTC continuation `0x48012670 → 0x48012070` 最终进入 `0x480129f0 → 0x48013ac8`；`0x48013ac8` 使用 context `+0xa4` 的值执行 `LDR X1,[X1,W0,SXTW #3]`，随后将加载值视作对象地址并发布到 `0x48015548`。
- raw monitor 中 `0x48014628` 的首 8 个 qwords 是 scalar/encoded-looking values，不是可信的 monitor-internal/SoC MMIO pointers；`0x48014668` 与 `0x48015548` 的 raw 初值均为 2。后者在 `0x48013b14` 被显式覆盖；前者在已审查调用路径中被反复作为 context pointer 读取。
- pinned BaseOS native SPL `cbe253a43f46cf551cc9a6c79a84015bde3686cf` 对 monitor 的直接修改 offsets 为 `+0x24`、`+0x30`、`+0xb0`，不含 `+0x14628` 或 `+0x14668`。这排除了当前已审查 SPL 源码中的直接 patch 作为解释，但不排除 BL31 内部间接初始化/运行时 mutation 或其他未审查的 loader contract。
- 专项文档：[rg34xx-rtc-soft-entry-twi-audit.md](rg34xx-rtc-soft-entry-twi-audit.md)。自动审计脚本：[audit-rg34xx-bl31-twi-descriptors.py](../scripts/audit-rg34xx-bl31-twi-descriptors.py)；host test：[test-audit-rg34xx-bl31-twi-descriptors.py](../scripts/test-audit-rg34xx-bl31-twi-descriptors.py)。

### 下一步

1. 静态追踪 BL31 自身对 lookup data `0x48014628` 和 context slot `0x48014668` 的所有初始化/复制/清零路径，尤其是 indirect pointer 和 helper-based memory writes；不能仅凭 direct xref 缺失宣布不存在 writer。
2. 扩展 matched monitor 全镜像的绝对地址/寄存器基址数据流扫描，定位可能为这两个窗口赋值的 copy/fill loops；对找到的写路径建立可复现 host assertion。
3. 仍然不推断 TWI bus/controller、PMIC 地址/命令；不发起 TWI transaction。
4. CPU reset、active retention PFN、BL33 memory ownership、PIO permission 与 timer/PPI handoff 门槛全部保持关闭。

`H700_MEMORY_LAYOUT_APPROVED=false`；`hardware_applied=0`。
 
## 2026-11-27 增量续接：BL31 audit #94、Boot0 CPU hold-off 与 retention source-page 结论

### CI 与审计产物

- [x] BL31 static audit [#94](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45956263984)：HEAD `8477f41a1bce996d36e240e041723cc877962223`，`audit-matched-bl31` job 全部 24 steps 成功；产物 [15178254258](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45956263984/artifacts/15178254258)。
- [x] 同一 run 上传 Boot0 primary/secondary、对应 metadata 与匹配 monitor 的只读 capture：[15178074907](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45956263984/artifacts/15178074907)。
- [x] H700 boot audit [#1452](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45956131864)：Linux host-tools、Windows PowerShell 两 job 成功。
- [x] 记录 #93 的测试失败根因：`system_suspend_helper_48013b98_complete` / `system_suspend_helper_48013b98` 窗口 key 不一致导致 `KeyError`；#94 修正 test key 后复验成功。未涉及固件运行时代码。

### 新证据与边界

- Boot0 helper 位于 `0x231e0`，调用者位于 `0x23224`；SID-window 候选寄存器 `0x03006248` bit29 决定是否以 core index 2、3 调用 helper，core1 未传入。helper 对 `0x09010000` 执行 `C0_RST_CTRL & ~(1 << core)`；H616 family reference 中 0 为 assert reset、1 为 deassert，因此该操作不会释放 CPU2/CPU3。H700 bit29 语义、H700 reset default 和 BL33 入口实时状态仍未读回。
- CPU1–CPU3 保持 reset 的静态结论只能标为 **PROBABLE**。不得启用 SMP；不能将 helper 对 core2/core3 的行为外推为 CPU1 的独立显式 hold-off 证明。寄存器 `0x09010044` 的 CPU0 RVBAR high 命名与 helper 的 bit2/bit3 写入仍有未解释矛盾。
- BL31 snapshot source `[0x40000000,0x40000140)` 落在 PFN `0x40000`。memblock classifier 结果是 `UNRESERVED_SYSTEM_RAM`，但 Linux 4.9.170 PFN capture 给出 `kpageflags=0x5868`、`page_mapcount=1`、`LINUX_ACTIVE_ANON_MAPPED_PAGE`。安全 gate 为 `BLOCKED_LIVE_LINUX_PAGE_REQUIRES_EXACT_PRESERVE_RESTORE`；绝不可当作 free scratch。
- RTC soft-entry continuation `0x48012670` 调用 helper `0x48012070`；后者经 `0x480129f0` 初始化 TWI-like controller，再经 `0x48013b98` 完成分阶段传输。controller descriptor 由 context `+0xa4` 选择，实际 selector、R-TWI bus index、PMIC slave/命令和最终 power action 未确认。保留“PMIC candidate”措辞，不称为已证明的关机/Boot0 跳转流程。

### RTC soft-entry TWI descriptor follow-up (2026-11-27)

Detailed audit: [rg34xx-rtc-soft-entry-twi-audit.md](rg34xx-rtc-soft-entry-twi-audit.md).

- `0x48013ac8` forwards context `+0xa4` into a 64-bit table lookup at `0x48014628`, dereferences the returned controller object, and stores it to global slot `0x48015548`; `0x48013a48` / `0x48013b98` then configure and use the controller.
- The raw monitor payload does not contain plausible pointer values in that table: initial qwords decode as numeric pairs. Likewise raw `0x48014668` is initially `2`, although code loads it as a context pointer. `0x48015548` also starts at `2` but is explicitly overwritten by `0x48013b14` before use. Thus runtime mutation/relocation/loader initialization is a real unresolved boundary; the raw table must not be interpreted as a live controller pointer list.
- A direct static-address scan did not identify direct stores to `0x48014628` or `0x48014668`; this does not exclude indirect writes or external loader/relocation behavior.
- The live-FDT AXP2202 / H616 R-TWI0 family correspondence remains a candidate only. No selected bus, object base, slave address or PMIC command is established.

### 下一步（全部 host/read-only 优先）

1. 追踪 context `+0xa4` 的赋值来源、descriptor table 的运行时初始化/重定位和最终 controller base；在此闭合前不得命名 bus index 或发出 I2C/TWI transaction。
2. 追踪 Boot0 所有 CPUCFG 写路径，解释 `0x09010044` 异常；仅通过已批准的可信只读路径取得 pre-BL33 `C0_RST_CTRL`、`C0_CPU_STATUS`、RVBAR0–3、DBG_REG0/1 与 SID bit29，不能改用裸 devmem/mmap 猜测。
3. 对 PFN `0x40000` 建立精确保存/恢复、cache/DMA/CPU quiescence 与 Linux continuation 的静态证明；在 source page 所有权闭合前禁止任何 suspend 实验。
4. 继续关闭 BL33 runtime/BSS/page-table/stack 所有权、timer-PPI continuity、APB1/PIO permission 与 32 KiB diagnostic loader package/rollback 审查。

`H700_MEMORY_LAYOUT_APPROVED` 继续 false；`hardware_applied=0`。

## 2026-11-27 续接：BL31 legacy PSCI、SYSTEM_SUSPEND 深度待机与唤醒链已静态识别

**当前审计/文档 HEAD：** `09665657646ecd0fa6131e76106fd4f8c518dcb9`。  
**最新 BL31 静态审计：** [#70](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45926919928)，20/20 steps 成功；[审计产物](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45926919928/artifacts/15166259361)。该 run 同时验证了生成的 `PSCI-CPU-ON.md` retention 摘要。底层 retention 指令审计/host assertions 在 [#69](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45926679076) 通过 20/20；[对应产物](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45926679076/artifacts/15167165432)。H700 boot-audit [#1432](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45926919816) 的 Linux host-tools 与 Windows PowerShell 两 job 均通过。#66 也为 20/20。#67–#68 在新增 retention helper host assertions 时因 WORDS 字典漏项失败；已按日志补齐并由 #69 复验。  
**匹配 monitor：** 107,216 bytes；SHA-256 `676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31`；该 payload 与采集到的原生 firmware/TOC1 monitor 匹配。

### PSCI 操作表与槽位映射

该表位于 `0x480165f8`，共有 **9 个 64-bit 函数指针**；通过 #34–#51 的静态审计逐项核对表项值、成员偏移、间接调用、affinity-level callback map 和深度待机路径：

| Slot | 地址 | 已识别语义 | 静态依据 |
|---|---:|---|---|
| 0 | `0x48002510` | `affinst_standby` | SCR_EL3 IRQ 位设置、DSB/WFI、恢复 SCR_EL3 |
| 1 | `0x4800254c` | `affinst_on` | 条件调用 `0x48002d78`，进入 CPU_ON/RVBAR 与 reset/power 控制序列 |
| 2 | `0x4800264c` | `affinst_off` | PSCI power-off dispatcher 从表偏移 `+0x10` 取指针并间接调用 |
| 3 | `0x48002878` | `affinst_suspend` | suspend dispatcher 从 `+0x18` 取指针；条件路径调用 CCU/AON save helper `0x48001790` |
| 4 | `0x48002680` | `affinst_on_finish` | suspend-finish 函数尾跳到此入口；on-finish dispatcher 从 `+0x20` 取指针 |
| 5 | `0x480026c4` | `affinst_suspend_finish` | dispatcher 从 `+0x28` 取指针；条件路径调用 restore helper `0x480018d8`，随后尾跳 slot 4 |
| 6 | `0x480025a0` | `system_off` | 终端系统电源回调入口；调用独立电源控制 helper |
| 7 | `0x48002574` | `system_reset` | 入口引用的嵌入字符串为 `Sunxi System Reset` 与 `Sunxi System Reset: operation not handled` |
| 8 | `0x48002544` | `get_sys_suspend_power_state` | getter 返回 `0x02010000`；SYSTEM_SUSPEND dispatcher 从表偏移 `+0x40` 加载并间接调用 |

前八项的顺序与旧版 TF-A `plat_pm_ops_t` 回调族吻合，第九项是厂商追加的 `get_sys_suspend_power_state`。这不是现代 `plat_psci_ops_t` 的成员顺序。BL31 中同时含有字符串 `psci_plat_pm_ops->get_sys_suspend_power_state` 和 `psci_system_suspend`，与反汇编调用路径互相印证。

### SYSTEM_SUSPEND 的直接证据

- getter 函数 `0x48002544` 的指令是 `mov w0, #0x02010000; ret`。
- 调用者位于 `0x480096f8`；`0x48009718` 取得 `psci_plat_pm_ops` 全局指针，`0x4800971c` 从偏移 `+0x40` 读取 slot 8，`0x48009744` 执行间接调用。
- 返回值在 `0x4800975c` 传入后续 suspend 分发函数 `0x48008d98`。
- slot 3/5 的 CCU save/restore helper 因此已分别归属 `affinst_suspend` 与 `affinst_suspend_finish`；slot 5 返回后尾跳 slot 4 `affinst_on_finish`。

### SYSTEM_SUSPEND 深度待机与唤醒返回路径

- `0x48008d98` 按 affinity level 0→2 遍历 `0x48017d70` callback map；三个 wrapper 分别为 `0x48008b84`、`0x48008ab8`、`0x48008a00`，最终调用 legacy platform-ops slot 3 `affinst_suspend`。system-level getter state 为 `0x02010000`，进入 level-2 条件路径。
- slot 3 的 CCU/AON save 路径在 `0x4800299c` 返回 `-4`。SYSTEM_SUSPEND caller 在 `0x48009768` 比较该 sentinel，并于 `0x48009774` 调用专用 power-entry `0x48001a04`。普通 powerdown 则另有 `0x48015d70: DSB SY; WFI; B 0x48015d74` 循环；两者不能混为同一入口。
- 专用入口清除 `SCTLR_EL3` 的 M/C 位（`0x4801612c–0x48016138`），执行 CLIDR/CCSIDR 驱动的 cache set/way 维护（`0x480160a8`、`0x48015fb8` 等），随后跳转到 SRAM standby image。
- 主代码块由 `0x48001a2c` 调用 byte-copy helper `0x4800aa80`，从 BL31 `[0x4800c220,0x48015630)` 复制 `0x9410` bytes 到 SRAM `[0x20000,0x29410)`。主入口源地址 `0x4800c3d8` 相对源块偏移 `0x1b8`，因此映射到运行地址 `0x201b8`。另有 header/context staging：`[0x48000030,0x480000b0)`→`[0x20000,0x20080)`、外部 runtime window `[0x4803cb90,0x4803cbc0)`→`[0x20080,0x200b0)`，以及两块条件 context 数据 `[0x4801b308,0x4801b368)`→`[0x200d4,0x20134)`、`[0x4801b380,0x4801b400)`→`[0x20134,0x201b4)`。
- 入口代码清零的 scratch 范围按相同重定位关系为 `[0x291f0,0x29349)`（源 `[0x48015410,0x48015569)`）。复制后的 stack setup helper 源 `0x480140a0` 映射到 `0x27e80`，literal stack top 为 `0x57ff0`。这说明 image 与栈都使用了更高 SRAM 地址；真实 SRAM C 所有权/时钟门控仍未运行时验证。
- 复制代码包含 WFI 分支 `0x4800c50c`、`0x4800c550`，并写入 RTC standby flag `0x070001f8=0x00010000`；RTC soft-entry `0x070001fc` 写入 context `+0xc4` 保存的返回地址 `0x48015d14`。唤醒代码从该 context offset 取回地址，经 `0x4800c6a8: BLR X22` 返回到 BL31 continuation。
- return continuation `0x48015d14` 恢复/配置 `SCTLR_EL3`，将 `VBAR_EL3` 指向 `0x48019000`，执行 `DAIFClr #4`、切换 `SPSel`，恢复 CPU/PSCI 上下文，最后 branch 到 `0x480158c8`。
- suspend/resume 两侧都读写 `0x03001500`（H616 reference map 中对应 CPUX_AXI_CFG_REG）：standby 用 `AND #0xfcffffff` 清 bits 25:24，resume 用 `ORR #0x03000000` 置回 bits 25:24；两边都调用 `0x48011ff8`。H616 `CLK_SRC_SEL` 字段定义覆盖 bits 26:24，但此代码保留 bit 26，因此确切 mux source/value 未知。此操作不是 CPU DVFS 证据。
- `0x48011ff8` 执行 `LSL W0,W0,#2` 后 branch 到 `0x48014078`；后者为 `SUBS X0,X0,#1; B.HI 0x48014078; RET`。两处调用者均传入 10，因此每次执行 40 次递减迭代；这是无时基校准的 busy-wait，不能换算成微秒。
- clock-save helper `0x48011ba8` 将五组 CCU 字段保存到 PSCI context：`0x03001500` CPUX_AXI_CFG (`0x303`)、`0x03001510` PSI_AHB1_AHB2_CFG (`0x3031f`)、`0x0300151c` AHB3_CFG (`0x3031f`)、`0x03001520` APB1_CFG (`0x303`)、`0x03001524` APB2_CFG (`0x303`)。这些名称/offset 与 H616/sun50iw9 家族 CCU 头文件对应；不表示 RG34XX live register values 已读回。
- 同一 helper 保存 PLL_CPUX factor bits（mask `0x0003ff03`），待机前清 PLL enable bit31 与 lock-enable bit29；resume helper `0x48011df8` 恢复 factor fields，置回 bit31/bit29，并在 `0x48011e7c` 轮询 lock bit28，再恢复保存的 CCU fields。该链是 suspend/resume 的时钟上下文维护，不是前端 DVFS API。
- `0x07010244` = R_PRCM base `0x07010000` + offset `0x244`。匹配 BL31 待机路径向其高字节写入 `0xA7`，resume 清除高字节；同家族 [SenkOS sun50iw9/H616 suspend 笔记](https://github.com/HookedBehemoth/SenkOS/blob/802be0c07939ab4f729f901d3c275101754e9e34/docs/suspend.md) 将 R_PRCM+0x244 识别为 PLL LDO key。此为同家族交叉识别，不是出厂 TF-A 源码证明或实机读回。
- saved mode==2 分支在 `0x03001510`、`0x07010000`、`0x07010010` 上将 bits25:24 设为 `10b`，并在 `0x07000160` 置 bit31、清 bit1。sun50iw9 RTC CCU 头文件将 offset `0x160` 命名为 `XO_CTRL_REG`；同家族 suspend 笔记记录了相同 bit31/bit1 操作。参见 [RTC CCU header](https://github.com/radxa/allwinner-bsp/blob/2045a3ca2a01f088c0314dc924bda59d154e363e/drivers/clk/sunxi-ng/ccu-sun50iw9-rtc.h)。这只识别寄存器族与代码动作，不推断当前硬件时钟/晶振状态。
- wake-source aggregate `0x48013ee8` 包含四类静态路径：GIC pending 查询 `0x48013190`（读取 GICD_ISPENDR，基址 `0x03021200`）；IR RX FIFO/status drain `0x480131b8`（`0x07040020/0x07040030`，写入 standby context queue）；GPIO 名称解析/电平读取 `0x48013588/0x480135d8`（normal PIO `0x0300b000` / R_PIO `0x07022000`）；GPIO pad config helper `0x48013620`（含 CFG/DAT/drive/pull 写路径，输入来自 context pin-config tuples）。这些 helper 的存在不证明 wake source 实际工作；未执行寄存器访问或 pin 配置。
- 机器报告和 host regression 已锁定上述 PLL/R_PRCM/XO_CTRL 字段、GIC/IR/GPIO 指令字和调用边；BL31 audit #59–#60 均通过 20/20。
- standby SRAM integrity helper `0x48011b78` 读取 context `+0xb0`（起始指针）与 `+0xb4`（字节长度），对范围内 32-bit words 求和。context `+0xc0` 是 enable flag；若启用，standby 在 `0x4800c448` 计算并将和保存到 `+0xb8`，resume 在 `0x4800c6c4` 重算、于 `0x4800c6d8` 比较。失败路径写入 `0x88000001`（x20 所指 scratch，本路径构造 base `0x07000108`）并在 `0x4800c6ec` 自循环。
- 初始 descriptor 已在匹配 payload 中定位：`0x4801a000` 处依次为 start `0x40000000`、length `0x100`、enable `1`；相邻字符串为 `chipid`。`0x48001a58–0x48001a78` 将这三个初始值读入 standby context `+0xc0/+0xb0/+0xb4`。但该默认值不是最终运行值：EL3 `arm_svc` handler 还暴露了可改写 descriptor 的 SMC 入口，见下节。checksum 仅为 32-bit additive sum，不是 CRC/加密哈希，也不覆盖整个 `0x9410`-byte SRAM image。CI #66 已将默认 descriptor、SMC 入口、两次 checksum 调用、比较和 fail-stop 路径纳入 20/20 regression。


### checksum descriptor 的默认值与 SMC 覆盖入口（2026-11-27）

- 默认 descriptor 位于 BL31 payload runtime address `0x4801a000`：`+0x0 = 0x40000000`、`+0x4 = 0x100`、`+0x8 = 1`；即默认校验区间 `[0x40000000,0x40000100)`，256 bytes / 64 个 32-bit words。此值由 `0x48001a58–0x48001a78` 复制到 standby context。邻接 `chipid` 字符串只是静态邻接证据，不据此推断该区域用途。
- 同一 BL31 有 `arm_svc` 字符串/记录：`0x48018c98` 指向 `0x48016ad9`（`arm_svc`），`0x48018ca8` 指向 handler `0x480035d0`。handler 保存 SMC 参数，并对 function ID `0x80000064` 与 `0xc0000064` 做比较；两个分支都汇入 `0x48003b8c–0x48003b94`，再调用 descriptor writer `0x480022fc`。
- writer `0x480022fc` 将调用参数依次写到 descriptor：SMC arg1 → enable byte `+0x8`；arg2 → start `+0x0`；arg3 → length `+0x4`。因此固件默认的 256-byte 区间可以在后续 SYSTEM_SUSPEND 读取 descriptor 前被 SMC 覆盖；最终 start/length/enable 不能仅凭默认常量定论。
- **运行时边界：** 目前没有 RG34XX 的 EL3 SMC trace、调用计数或 descriptor live readback，不能声称设备实际调用过这两个 FID，也不能声称默认 256-byte 范围就是某次待机实际使用的范围。公开检索到的较早 H6 `sunxi-smc.h` 使用另一组 `0xFFFF0000` 系列旧命令；这不是当前 RG34XX 4.9.170 内核未调用上述 FID 的证明。没有执行 SMC、MMIO 读写或待机。
- 新增 host assertions 锁定 handler 记录指针、`arm_svc` 名称、两个 FID 的分支目标、参数转发与 writer store。BL31 audit [#63](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45924971769)、[#64](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45925098616) 与最终 [#66](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45925450827) 均通过 20/20；#65 的 `arm_svc` 字节串断言误将 NUL 终止符写成转义文本，已修复并由 #66 复验。


### DRAM retention snapshot / restore：RTC GP_DATA1 指针与双窗口搬运（2026-11-27）

- SYSTEM_SUSPEND power-entry `0x48001a04` 在 `0x48001aac–0x48001abc` 计算并写入 RTC GP_DATA1 `0x07000104` 的值 `0x4801b400`。该值随后由 helper `0x48011710` 与恢复 helper `0x48011938` 从 GP_DATA1 读出，作为保留区 buffer 指针。
- standby 调用链：`0x4800c490 → 0x48011b40 → 0x48011920 → 0x48011710`。普通路径总计搬运 160 个 32-bit word（`0x280` / 640 bytes）：固定窗口 `[0x40000000,0x40000140)` 保存到 buffer `[0x4801b400,0x4801b540)`；第二窗口从动态 bank base `0x40000000 + (((context[0x1c] >> 17) << 20))` 开始，长度 `0x140`，保存到 `[0x4801b540,0x4801b680)`。
- resume 恢复链：`0x4800c658 → 0x48011b58 → 0x48011b10 → 0x48011938`，normal path 反向写回上述两个 DRAM 地址窗口。保存 helper 在 context `+0x1c` bit0 为 1 时跳过 normal copy；若 context `+0x4 == 4`，会走 `0x480118d0/0x480118f8` 或恢复侧 `0x48011a80/0x48011aa8` 的 alternate strided addressing 分支，不能概括为连续 memcpy。
- buffer 区间 `[0x4801b400,0x4801b680)` 位于匹配 monitor file payload 结束地址 `0x4801a2d0` 之外。RTC GP_DATA1 位于 RTC base `0x07000000` + offset `0x104`；Linux 同系列 `rtc-sun6i.c` 将 GP_DATA 区域映射为 battery-backed NVMEM（[source](https://github.com/torvalds/linux/blob/master/drivers/rtc/rtc-sun6i.c)）。这是同系列寄存器定义交叉参考，不是 RG34XX 实机 GP_DATA1 readback，也不代表 buffer 的 runtime ownership 已获批准。
- checksum 的静态默认范围 `[0x40000000,0x40000100)` 位于第一段 `0x140` 快照窗口内；这只是地址重叠，不能据此推断固件设计意图，尤其 checksum descriptor 可由 `arm_svc` SMC 覆盖。
- CI #69 新增了 buffer pointer、两组 `0x140`-byte 窗口、selector 算术、160-word loop、alternate stride branch 和 resume reverse-copy 的 host assertions，并扩大 pre-copy disassembly window 覆盖 `0x48011710–0x48011920`。#69 通过 20/20；#70 进一步验证了生成的 Markdown 报告包含 retention 摘要。结论依旧是匹配 BL31 的静态 control-flow 证据。
- **边界：** 未读取设备 RTC GP_DATA1、context `+0x1c/+0x4`、`0x4801b400` 内容或 DRAM 窗口；未发起 PSCI SMC、未读写 MMIO、未执行待机/唤醒。内存所有权、RTC backup 实际供电状态与正常路径是否在目标机被选中仍未证明。



### DRAM restore gate、控制器状态初始化与 MSI_CTRL fallback（2026-11-27）

- `0x48011938` 的 resume worker 先调用 gate `0x48011320`；gate 按序执行 `0x480111a0`（DRAM 时钟恢复准备）与 `0x480115c0`（控制器状态 staging），最终返回值在 `0x4801197c` 判零；零值跳过随后的 scratch→DRAM 恢复拷贝。live gate result 尚未采集。
- `0x480111a0` 对 `0x03001540`、`0x0300180c`、`0x03001010`、`0x03001800` 执行 DRAM/MBUS 时钟、reset、PLL enable/lock 的恢复序列，并用 context 参数重建 PLL factor。寄存器块名称与位语义目前借用同系列 H616 CCU map 交叉参考；未对 RG34XX H700 做寄存器 readback，不能把 H616 命名视为目标芯片验证。
- `0x480115c0` 调用 `0x48010f98` 和状态探测 helper `0x48011360`。后者依据 context `+0x6c` bits 19–23 选择若干 probe，单项最多 5 次检查，并有最多 10 次 gate retry；失败路径返回 W0=0。各 bit 与具体 DRAM 子系统的映射、probe 的 live outcome 均未知。
- **地址解码更正：** 下级 helper 的 AArch64 指令为 `MOVK #0x47f, LSL #16`，构造基址是 `0x047f0000`，不是 `0x47f00000`。因此实际访问是 `0x047fa008`、`0x047fb320`、`0x047fd180` 等外设 MMIO 地址，不是 `0x47f0a008` 一类 DRAM RAM 地址。当前 JSON 已把旧字段 `draminternal_ram_addresses` 更正为 `mmio_addresses`，并将 address-construction 指令与 58 项 host assertions 锁定。
- H616 family memory map 将 `[0x047fa000,0x047fb000)` 命名为 MSI_CTRL、`[0x047fb000,0x04800000)` 命名为 DRAM_CTRL；`[0x04800000,0x05000000)` 为 PHY_CTRL。此处仅作同系列 SoC map cross-reference；H700 target 具体 offset/bit 语义没有 readback，也没有 MMIO 操作。
- `0x480119d0` 的 `TBZ W0,#27` 在 context `+0x1c` bit27 清零时分支到 `0x48011ae0`，随后调用完整 fallback helper `0x4800c7a8`。helper 精确窗口 `[0x4800c7a8,0x4800c964)` 包含 `0x4800c960 RET`，对 MSI_CTRL aperture 写入多个初始化常量（例如低 offset `0x00c/0x200/0x210/0x244/0x260/0x270/0x290/0x370/0x3a0/0x3a4/0x460/0x480/0x484`）。它已从旧的误命名 `retention_rtc_fallback_setup_helper` 更名为 `retention_msi_ctrl_setup_fallback_helper`；这不是 RTC 操作，也不是 RAM staging buffer。
- CI #76 锁定 DRAM clock/state helpers 与 lower callees；#78 锁定 58 条 `0x047fxxxx` 地址构造指令；#79 锁定 `0x480119d0` fallback predicate 与 MSI_CTRL 分类；#81 将 `0x4800c9e8` DRAM_CTRL 和 `0x4800dfe8` PHY_CTRL mode-dependent setup helper 纳入完整窗口；#82 将 `0x4800c7a8` fallback helper extent 精确收紧至 `[0x4800c7a8,0x4800c964)`，包含 `0x4800c960 RET`，不再覆盖后续函数。#76/#78/#79/#81/#82 均 20/20；#77 的单条指令 PC 偏移错误由 #78 修复，#80 的字符串闭合错误由 #81 修复。
- **运行时边界：** 没有读取 `context+0x6c`、context bit27、MSI_CTRL/DRAM_CTRL/PHY_CTRL registers 或 DRAM PLL live state；没有调用 PSCI SMC、没有 MMIO/DRAM read/write、没有执行待机/唤醒。不得从静态控制流推断 H700 当前控制器处于何种状态。


### H700 Super Standby 独立参考实现：SenkOS / h700-suspend-stub（2026-11-27）

- 参考项目：[HookedBehemoth/SenkOS](https://github.com/HookedBehemoth/SenkOS)，固定审查 commit `802be0c07939ab4f729f901d3c275101754e9e34`；主技术说明为 [`docs/suspend.md`](https://github.com/HookedBehemoth/SenkOS/blob/802be0c07939ab4f729f901d3c275101754e9e34/docs/suspend.md)。SenkOS 是面向 H700 掌机的完整 Linux 系统，不是 Linkslot 前端/运行时；其价值是 H700 TF-A、SRAM stub、Boot0/SPL、PMIC 与 Linux resume 的可运行参考。
- SenkOS TF-A recipe 固定 `STUB_REV=712653d11230b2450061ddb8e25f31e072da7a48`，并固定 U-Boot `v2026.07`；分别构建 LPDDR4 和 LPDDR3 两份 stub，均使用与 SPL 对应的 DRAM driver/config，BL31 按 DRAM 类型/频率选择匹配 stub。recipe 同时应用 `0001-allwinner-h616-psci-system-suspend.patch`、`0002-allwinner-h616-super-standby.patch` 与 `stub-0001-super-standby.patch`。参考 [`trusted-firmware-a_%.bbappend`](https://github.com/HookedBehemoth/SenkOS/blob/802be0c07939ab4f729f901d3c275101754e9e34/yocto/meta-anbernic/recipes-bsp/trusted-firmware-a/trusted-firmware-a_%25.bbappend)。
- SenkOS 的 stock-firmware 逆向记录：BL31 将 `0x9410` 字节 standby program 搬到 SRAM A1 `0x20000`；写入 DRAM parameters、standby rails、warm-entry `0x48015d54` 和 RTC GP_DATA1 `0x07000104`；super-standby 程序让 DRAM self-refresh、切换低功耗时钟并配置 AXP717 电源轨；Boot0 在冷启动路径重新初始化 DRAM、校验内存范围、从 RTC `0x104` 指向的 backup 恢复前 `0x280` 字节，然后跳回 BL31 warm entry。该描述来自 SenkOS 对 stock 固件的逆向笔记，需与本项目匹配 BL31 的独立指令证据逐项对应，不当作厂商源码。
- 与本项目静态审计的强交叉点：本项目发现 RTC GP_DATA1 `0x07000104` 保存 scratch pointer `0x4801b400`，buffer 总长 `0x280`；两个快照段各 `0x140` 字节。SenkOS 自定义 stub 也用 `SAVE_WORDS=0x50`（每段 `0x140` 字节）分别保存 DRAM base 与 `base + mctl_calc_size()/2`。这支持“两个 0x140-byte 窗口/总 0x280 bytes”的设计交叉验证，但高段地址算法不同：SenkOS 使用 DRAM size/2，本项目 stock BL31 使用 context geometry selector；不得将两者合并为同一地址算法。
- SenkOS 自定义 Super Standby 不是 stock BL31 原路径的简单复用：它对 TF-A 增补 PMIC rail-cut / warm-entry 处理，对 SRAM stub 增补 RTC handshake、DRAM snapshot、checksum 和失败标志，对 SPL 增补 resume-mode DRAM init 与恢复/重启策略，并对 Linux kernel 增补 CCU/R_CCU/PPU/NMI/pinctrl/DMA/timer/watchdog/PWM、IOMMU、显示/音频/USB PHY 等 SoC 状态恢复。只有 Linkslot 目标仍以 Linux 为 BL33 时，这些 Linux driver restore patch 才有直接移植价值；裸机/自有 HAL 目标应提取硬件顺序与寄存器语义，不移植 sysd/Weston/Launcher 等软件栈。
- **不要混淆 stock 与 SenkOS 自定义 RTC 协议。** SenkOS 文档描述的 stock BL31/Boot0 路径使用 RTC `+0x1f8` standby flag、`+0x1fc` warm entry、`+0x104` DRAM backup pointer、`+0x10c` sum、`+0x118/+0x11c` checksum range 等；这与本项目 stock BL31 对 `0x07000104` 的 pointer 发现相呼应。SenkOS 自定义 stub/SPL 路径则另定义 `+0x100` handshake magic、`+0x104` warm entry、`+0x114` DRAM config、`+0x118` save pointer、`+0x11c` sum。两套协议复用 RTC GP_DATA 区但字段语义不同，移植时绝不能将它们交叉套用。
- SenkOS 自定义 SPL patch 将恢复设计成 one-shot：启动恢复先把 magic 改为 `SSTI`，DRAM resume init 成功并恢复两段各 `0x140` 字节后，计算 DRAM base 起始 `0x40000` 字节的 32-bit sum；与 RTC sum 相等才写 `SSTR` 并跳转 warm entry。DRAM init 失败记 `SSTD`，sum 不匹配记 `SSTC`，resume hang 留下 `SSTI`；失败时通过 AXP717 restart 重新上电，而不是带着 Linux 关闭的电源轨继续普通冷启动。该补丁还在 cold boot 释放 RTC `+0x1f4` DRAM pad hold，文档记录 H700 上 bit0 的真实极性与 H616 手册描述相反（set releases）。参考 [`0009-sunxi-H616-resume-from-super-standby.patch`](https://github.com/HookedBehemoth/SenkOS/blob/802be0c07939ab4f729f901d3c275101754e9e34/yocto/meta-anbernic/recipes-bsp/u-boot/files/0009-sunxi-H616-resume-from-super-standby.patch)。
- SenkOS Linux patch 在 syscore suspend 最末端保存 SoC blocks，在 resume 初期、普通设备 driver 恢复前，仅回写发生变化的寄存器。范围包括 CCU/R-CCU、PPU gates、NMI、PIO/R_PIO、syscon、DMA、timers/watchdog、PWM；其他 driver 自己恢复 IOMMU、thermal、DE33/TCON、USB MUSB PHY、codec 等。参考 [`0051-soc-sunxi-restore-the-SoC-state-after-an-H616-super-.patch`](https://github.com/HookedBehemoth/SenkOS/blob/802be0c07939ab4f729f901d3c275101754e9e34/yocto/meta-anbernic/recipes-kernel/linux/files/0051-soc-sunxi-restore-the-SoC-state-after-an-H616-super-.patch)。这解释了为何仅让 DRAM 恢复并跳回 BL31 不足以保证 Linux 外设恢复。
- **重要安全发现：SenkOS pinned stub 的 live DRAM probe 风险。** SenkOS `stub-0001-super-standby.patch` 在 `src/lib.c` 新增 `mctl_check_memory()`：`readl(addr) → writel(~orig, addr) → readl(addr) → writel(orig, addr)`；而 stub 在 DRAM 进入 self-refresh 前通过 `mctl_calc_size()` 调用该检查（`src/main.c` 的 `dram_suspend()` 及 `stub_main()` 均计算 DRAM size）。SenkOS recipe 固定的 stub rev `712653d...` 本身没有 no-write guard；本地 patch 明确重新引入了写探测。
- 该风险来源于上游 U-Boot commit [`c47b636737`](https://github.com/u-boot/u-boot/commit/c47b636737366d9bc017b915ca6fcbcce8706e6e)：`mctl_calc_size()` 为检测 1.5/3-GiB 等非 2 次幂容量，会对 `CFG_SYS_SDRAM_BASE + 3/4 * size` 做反相写/读回/恢复，失败时还探测前一个 cacheline。后续 stub fork commit [`a8d13c9`](https://github.com/mburakov/h700-suspend-stub/commit/a8d13c95ba68b2e2b216147e0146b8cdf7809627) 明确说明 stub 运行时 Linux 仍拥有 DRAM，因此在 freestanding stub 中将 `mctl_check_memory()` 改为始终返回 true，并将 `printf()` 变为空操作。**仅升级上游 stub 到 a8d13c9 不够：SenkOS 的 `stub-0001-super-standby.patch` 也定义了同名写探测函数，移植时必须一并删除/改成 no-write guard。**
- 本项目后续如集成 SRAM suspend stub，强制规则：不得在 Linux/BL33 仍拥有 DRAM 时运行 destructive `mctl_check_memory()`；不得因“写回原值”就认定安全；容量与 geometry 应从已经可信的 SPL handoff/context 获得，或用不触碰在线 DRAM 的方式确认。stub 只有在所有相关 CPU/DMA 已按已验证顺序停机、缓存一致性与 ownership 已证明后，才能执行必要内存操作。
- SenkOS 文档给出的 stock checksum 范围为 kernel 经 SMC 传入的 256-KiB 区间；本项目目前只静态识别到 checksum descriptor 默认值及 `arm_svc` SMC override，runtime descriptor 尚未读到。因此不能将 SenkOS 的 `0x40000` 校验范围直接写成本项目 stock BL31 的结论，也不能以它替代当前固定源页 `[0x40000000,0x40000140)` 的 memblock/PFN ownership gate。
- SenkOS 文档记录 2026-10-07 的 H700 Super Standby 实机验证（多核、Wi-Fi、游戏音频、重复睡眠/唤醒），说明该 SoC 路线在特定板级电源/PMIC/Boot0/内核补丁组合上可实现。它不是本项目 RG34XX 设备已验证结果；SenkOS 使用 AXP717 电源策略、LPDDR3/LPDDR4 双 stub 及其指定 U-Boot/TF-A/kernel patch set。
- **本项目采用方式：** 用 SenkOS 对照硬件 handoff、双 0x140-byte snapshot、Boot0 restore、checksum/RTC handshake、PMIC rails 与失败回退；针对 stock BL31 保留本项目自己的反汇编结论；将 no-live-DRAM-write 设为移植审查 gate；不复制 SenkOS Linux userspace，也不直接复制其 H616-family register semantics 到 H700 未读回寄存器。

### DRAM retention 固定源窗口与 Linux memblock 保留区冲突（2026-11-27，待解决门槛）

- reports 分支采集 `reports/rg34xx-inventory-20261126-timer/memory-map-report.json` 的 SHA-256/blob evidence 对应最新 2026-11-26 memory-map snapshot。`memblock.memory` 声明 `[0x40000000,0x80000000)`；`memblock.reserved` 有 46 条记录，其中 monitor carveout `[0x48000000,0x49000000)`，但没有任何条目覆盖 BL31 retention 固定源窗口 `[0x40000000,0x40000140)`，也没有覆盖其 4-KiB containing page `[0x40000000,0x40001000)`。因此该页处于 memblock System RAM 中，且采集到的 reserved array 未显式保留它。
- retention scratch buffer `[0x4801b400,0x4801b680)` 位于 `[0x48000000,0x49000000)` monitor carveout 内；这与 fixed DRAM source window 的证据状态不同。前者的地址区间有 Linux memblock reserved carveout 覆盖，后者没有。
- 跨 2026-11-22 至 2026-11-26 的 inventory memory-map batches 复核结果一致；检查到的多个不同 JSON blob revision 均显示同样的 memblock.memory 起点、monitor carveout 和 fixed-window reserved-list miss。live FDT memory 区域为 `[0x40000000,0x80000000)`，`reserved_memory_nodes` 为空（FDT SHA-256 `620b36d84fb7e714dcb56891f50802048d83c64ca924222a4a566f340ac7242e`）。
- **更新解释（2026-11-27 PFN 样本）：** 原先缺失的 `/proc/kpageflags` 与 `/proc/kpagecount` 已采集；它们显示 PFN `0x40000` 是 active-LRU anon mapped page，page_mapcount=1。因此它不是可供 Linkslot 分配的空闲 scratch。memblock reserved-list miss 仍然成立，但不应简单通过 memblock 强行 reserve 来“修复”：BL31 的静态路径看起来是保存并恢复这段原始 DRAM 内容。现在的关键门槛是证明保存/恢复在 suspend handoff 中无损且先于 Linux resume 完成，并核对 DMA/cache/CPU ownership；Secure/Non-secure firewall 和 BL31 权限仍未知。保持 `H700_MEMORY_LAYOUT_APPROVED` unset，不执行 SYSTEM_SUSPEND。
- 后续只读页证据已于 2026-11-27 上传并解码：`/proc/kpageflags` 显示 PFN `0x40000` 为 active-LRU anon mapped page，`/proc/kpagecount` 的 page_mapcount=1；page_owner/page_ext 仍未提供分配栈。不要再重复请求同一份 kpageflags/kpagecount 采集；下一门槛转为解释 BL31 snapshot/restore 的运行时条件及 secure/BL31 ownership。
- memblock 区间 classifier 已在 BL31 audit #85（20/20 steps）接入 CI，实际输出 `UNRESERVED_SYSTEM_RAM` / `BLOCKED_SOURCE_PAGE_NOT_RESERVED`；PFN 元数据 decoder 又在 #86 完整通过。两者均为 host/report-side evidence gates，不会自行批准硬件内存布局。`hardware_applied=0`。

### DRAM retention fixed-page memblock classifier（2026-11-27）

- 新增 `scripts/audit-rg34xx-dram-window-ownership.py`：将 stock BL31 固定源窗口 `[0x40000000,0x40000140)` 扩展到完整 containing page，再按半开区间 union coverage 检查 `memblock.memory` 与 `memblock.reserved`；不会将字节级子区间误当成整页保护。
- 新增 `scripts/test-audit-rg34xx-dram-window-ownership.py` host regression，覆盖当前未保留样本、完整保留、部分保留、相邻 reservation union、CMA-only 不能替代 reserved、超出 memblock.memory 与缺少 memblock 数据。
- `h700-bl31-memory-audit.yml` 增加上述测试及 reports 分支 memory-map JSON 分类步骤；生成 `DRAM-WINDOW-OWNERSHIP.json/.md` 并纳入 job summary 与审计 artifact。发现当前风险不会被包装成硬件通过，也不修改运行时代码。
- 当前样本预期：PFN `0x40000` 对应页 `[0x40000000,0x40001000)` 完全位于 `memblock.memory`，但 `memblock.reserved` 覆盖为 0/4096 bytes，分类 `UNRESERVED_SYSTEM_RAM` / `BLOCKED_SOURCE_PAGE_NOT_RESERVED`。报告的 CMA debugfs 为 `not_present`；`CmaTotal=64 MiB` 不是物理 CMA 区间，不能据此声称 PFN 属于 CMA。
- 即使未来 classifier 显示整页 reserved，也只解除“Linux memblock 未保留”这一项疑点；仍不能证明 PFN allocator/page-owner、Secure/Non-secure firewall、BL31 访问权限、DRAM controller 状态或运行时 selector。始终保持 `H700_MEMORY_LAYOUT_APPROVED=false`，直至独立证据闭合。
- BL31 audit #85（run `45934597078`）在 `855b022734693a8c5130b5a0773b8bd1922c0d22` 上完整成功；classifier host regression、reports JSON 分类、原有 BL31/Boot0/PSCI 审计及 artifact 上传步骤全部成功。生成 artifact `15170227641`。实际分类为 `UNRESERVED_SYSTEM_RAM` / `BLOCKED_SOURCE_PAGE_NOT_RESERVED`。#84 是 workflow YAML 语法错误导致的无-job失败，已由 `855b022` 修复并由 #85 复验。 无实机采集、SMC、MMIO、DRAM 读写、suspend/resume 或刷写，`hardware_applied=0`。

### PFN 0x40000 的只读页元数据采集器已准备（2026-11-27）

- 新增 `scripts/collect-rg34xx-dram-pfn-readonly.sh`，只针对 BL31 固定源页 `[0x40000000,0x40001000)`（4-KiB page，PFN `0x40000`）读取 `/proc/kpageflags` 与 `/proc/kpagecount` 的单个 8-byte 条目；同时只检查 page_owner 节点可用性及 `CONFIG_PROC_PAGE_MONITOR/PAGE_OWNER/PAGE_EXTENSION/DEBUG_VM` 配置。
- 采集器要求 root、确认 page size=4096，且只读取 procfs/debugfs 元数据；不 dump `page_owner` 全表、不 mount debugfs、不读写 DRAM 物理内容、不触发 SMC/MMIO/PSCI、不做 suspend/resume，不写 boot media。
- 本地 agent 执行方式：`adb push scripts/collect-rg34xx-dram-pfn-readonly.sh /data/local/tmp/rg34xx-dram-pfn-readonly.sh`，先确认 `adb shell id -u` 返回 `0`，再运行 `adb shell sh /data/local/tmp/rg34xx-dram-pfn-readonly.sh` 并将完整 stdout 保存为 `dram-retention-pfn.txt` 上传 reports 分支。若不是 root，不尝试提权/改启动参数；记录 `BLOCKED_ROOT_REQUIRED` 即可。
- `kpageflags/kpagecount` 已按 Greg KH Linux stable tag `v4.9.170` 的 UAPI 与 `fs/proc/page.c` 解码；目标运行 kernel 是 `4.9.170 #19`、AArch64。目标 build-ID 为 `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68`，精确 vendor source commit 尚未恢复，因此保留 build-source provenance 限制。
- **状态：采集器已在目标 RG34XX 以 UID 0 执行。** Page size=4096，kpageflags raw=`6858000000000000`（KPF u64 `0x5868`），kpagecount raw=`0100000000000000`（page_mapcount=1）；page_owner/page_owner_stacks 不存在。该页采样时为 active-LRU anonymous mapped page，不得作为 Linkslot scratch。单次样本仍不能批准 BL31 与 Linux 并发访问；需结合 snapshot/restore gate、buffer lifetime、DMA/cache 与 secure-world ownership 判断。保持 `H700_MEMORY_LAYOUT_APPROVED` unset，`hardware_applied=0`。

### PFN 0x40000 元数据解码：确认是活动匿名映射页（2026-11-27）

- 新报告位于 `reports/rg34xx-dram-pfn-20261127/`；目标 PFN `0x40000` / 页面 `[0x40000000,0x40001000)`，BL31 固定 source window `[0x40000000,0x40000140)` 完全落在该页内。
- 设备原始 kpageflags bytes `68 58 00 00 00 00 00 00`，按 AArch64 little-endian 得到 KPF u64 `0x5868`；set bits 为 3/5/6/11/12/14。对照 Linux stable `v4.9.170` 的 `include/uapi/linux/kernel-page-flags.h` 和 `fs/proc/page.c`，分别是 `KPF_UPTODATE`、`KPF_LRU`、`KPF_ACTIVE`、`KPF_MMAP`、`KPF_ANON`、`KPF_SWAPBACKED`。
- 关键定义：v4.9.170 `stable_page_flags()` 在非-SLAB 且 `page_mapped(page)` 时置 `KPF_MMAP`；`PageAnon(page)` 置 `KPF_ANON`；`PG_lru/PG_active/PG_swapbacked` 分别导出为上述稳定位。故该样本在采集时是一个**正在映射的匿名页，并处于 active LRU**，不是 free buddy page。原始 kpagecount bytes `01 00 00 00 00 00 00 00` 解为 1；v4.9.170 `fs/proc/page.c` 明确将该字段写成 `page_mapcount(page)`，因此表示一个 page mapping，不是 page refcount，也不能识别具体进程。
- `KPF_BUDDY` bit10、`KPF_COMPOUND_HEAD/TAIL` bits15/16、`KPF_HUGE` bit17 与 kernel-internal `KPF_RESERVED` bit32 均未置位；这与“普通活动匿名页”的解读一致。page_owner 节点没有提供可用的分配栈，不能追溯映射创建者。
- 这**确认页面是 Linux 正在使用的内容**，但不能单独证明 BL31 的保存/恢复会破坏它。BL31 audit #86 的完整静态调用链已确认：EL3 MMU/D-cache teardown 与按 CLIDR/CCSIDR 执行的 DC CISW 先于 SRAM handoff；SRAM 代码在 sleep/wake 前 snapshot 固定 320-byte 窗口，resume 时先恢复 DRAM clock/controller 并经过 gate，再 copy-back，最后才返回 BL31 并恢复 EL3 控制状态。该顺序显示固件设计上意图保全该活动页中的特定数据，但 gate/flags/selector 的设备运行时值尚未知，不能据静态顺序声称实际恢复成功。
- 必须保留的安全约束：不得将这个 PFN 分配给 Linkslot/LK heap、diagnostic loader、DMA 或其他 scratch；不得用写入测试验证它；不得因 kpagecount=1 就推断进程身份。一次快照只证明采集时的状态，页面未来的映射可能改变。
- 新增 `scripts/audit-rg34xx-dram-pfn-metadata.py` 与 host regression，按上述 v4.9.170 KPF ABI 解码 raw report；CI 生成 `DRAM-PFN-METADATA.json/.md` 并将结论附加到 BL31 audit job summary。该工具永不批准内存布局，也不运行设备操作。 BL31 audit #86（run `45951458800`，revision `d2199893`）已完整通过；host regression 与真实报告解码均成功，产物 `15177495643`。实际分类 `LINUX_ACTIVE_ANON_MAPPED_PAGE`，gate `BLOCKED_LIVE_LINUX_PAGE_REQUIRES_EXACT_PRESERVE_RESTORE`。H700 boot audit #1442（run `45951458825`）的 Linux host-tools 和 Windows PowerShell 两 job 均通过。 BL31 audit #87（run `45952554608`，revision `4efff40`）在新增 cache dispatch/barrier assertions 后再次完整通过；H700 boot audit #1445（run `45952554672`）的 host-tools 与 Windows PowerShell 两 job 亦通过。
- 后续分析顺序调整：静态 copy-out/copy-back 调用顺序、cache teardown 与 resume gate 已由 BL31 audit #86 锁定；下一步不是重复证明静态边，而是取得运行时 context flags、selector 与 gate result 的非侵入证据，并核对 RTC buffer / SRAM retention、CPU/DMA quiescence、RTC soft-entry 是否会落入 Boot0/SPL。**不能为了满足 memblock reserved-list 而机械地把这页改为保留区**，因为当前证据显示它承载匿名映射内容，正确策略取决于固件是否按设计无损保全该页。
- 本轮仍无 SMC、MMIO、DRAM 内容读写、suspend/resume 或刷写；`H700_MEMORY_LAYOUT_APPROVED` unset，`hardware_applied=0`。

### BL31 活动 DRAM 页 preservation 顺序：cache handoff、SRAM snapshot 与 gated restore（2026-11-27）

- 本轮从 BL31 audit #90 匹配二进制反汇编工件 PSCI-CALLBACK-DISASSEMBLY.txt 继续复核；cache dispatch、context +0xcc/+0xc8 sleep-mode branch 与 RTC soft-entry continuation/helper-return self-loop assertions 均已通过 #90。当前将 RTC continuation helper `0x48012070` 覆盖扩展到 `[0x48012070,0x48012420)`，并展开 `0x48013a48/0x48013b98/0x48012fa8` 的配置解析/访问 helpers，追踪 RTC GP_DATA2 标记的写入序列与后续调用；新增窗口待下一轮 CI 验证。所有结论均为静态控制流证据，不是设备上的实际执行 trace。
- **进入 SRAM 前的 cache/MMU handoff：** power-entry 在 0x48001abc 将 retention buffer pointer 0x4801b400 写到 RTC GP_DATA1 0x07000104；随后在 0x48001b5c 调用 0x48016128。该 helper 清除 SCTLR_EL3.M 与 SCTLR_EL3.C（mask 0x5），执行 ISB，再从 0x480160a8 读取 CLIDR_EL1 LoC 并按每级 CCSIDR 遍历 sets/ways。入口设置 x0=1，使 dispatch 选择 DC CISW, X11（data-cache clean+invalidate by set/way）；循环前后有 DSB SY，尾部有 ISB。完成后 power-entry 在 0x48001b70 branch 到 SRAM A1 entry 0x201b8。准确来说，这条选路是 CISW，不是 ISW；静态代码存在不等于目标机每级 cache 实际执行完成已被测量。 新增 dispatch assertions 已由 BL31 audit #87 复验：x0=1 位于 0x4801613c，base 0x48016038、stride 0x20，选中块 0x48016058，其实际 DC CISW 位于 0x48016060；报告还锁定 DSB 0x48016014/0x4801602c 与 ISB 0x48016030。CI 只证明匹配 binary 指令和分支目标，不测量设备运行时 CLIDR/CCSIDR 值或 cache 行数量。
- **SRAM 内先 snapshot、后进入 sleep：** relocated entry 0x4800c3d8（SRAM entry 0x201b8）先设置 SRAM stack 并清理 image 内 scratch，然后按顺序调用 0x4800c418（prep + snapshot）、0x4800c4d0（sleep/wake path）、0x4800c618（resume restore）。0x4800c418 可选地先对 context checksum descriptor 计算并保存 sum；随后切换 0x03001500 的 bits25:24 并 busy-wait，再从 0x4800c490 进入 snapshot wrapper 0x48011b40 → 0x48011920 → 0x48011710。snapshot helper 从 RTC GP_DATA1 取 buffer pointer，将固定段 [0x40000000,0x40000140) 的 320 bytes 复制至 [0x4801b400,0x4801b540)，再将动态 midpoint 段（由 context geometry selector 派生）320 bytes 复制至 [0x4801b540,0x4801b680)，普通路径共 160 个 32-bit words / 640 bytes。snapshot 返回后才调用 0x48011ba8 保存时钟上下文。若 context +0x1c bit0 置位，snapshot helper 会跳过 normal copy；若 context +0x4 == 4，使用 alternate strided addressing，而不是连续 memcpy。
- **sleep/wake 与 RTC 分支已精确区分，但运行时模式未知：** `0x4800c4d0` 读取 context `+0xcc`。`bit0=0` 时继续检查 `bit1`：`bit1=0` 进入普通 wake-source 配置与 WFI/poll 循环（`0x48013df0` setup，`0x48013ee8` poll，poll 返回 0 时回到 WFI）；`bit1=1` 跳至 RTC soft-entry，写 standby flag `0x10000` 到 `0x070001f8`、保存 context `+0xc4` 到 `0x070001fc`，并跳转 `0x48012670`。若 `bit0=1`，不走 bit1 分支：context `+0xc8=0` 时执行一次 WFI 后返回；非零时执行 `+0xc8 × 16` 次计数循环后返回，不经过该 WFI。分支原始 TBZ/TBNZ/CBNZ/B target 已加入 host audit/CI regression。运行时 `+0xcc/+0xc8` 尚未采集，不能断言设备采用哪条分支；RTC soft-entry 的存在也不等同于已经证明 Boot0 冷启动。其静态 continuation `0x48012670` 会聚合 context `+0x80/+0x88/+0x8c/+0x90/+0x98/+0xa4`，调用 `0x48012070`，若 helper 返回则在 `0x480126a4` 自循环；helper 的最终平台动作仍未识别，不能据此称为 PMIC power-off 或 Boot0 transfer。 该 continuation call/self-loop 的目标已由 BL31 audit #90 的 host assertions 固定。SenkOS AXP717 power-cut → BROM/Boot0 → SPL resume 仍是另一套流程，不能与 stock BL31 路径混为一谈。
- **恢复顺序：** 0x4800c618 先调用 0x48011df8，随后进入 0x48011b58 → 0x48011b10 → 0x48011938。restore worker 先调用 gate 0x48011320；gate 按序调用 0x480111a0（DRAM clock/PLL restore preparation）与 0x480115c0（DRAM controller state staging/probes）。0x4801197c 检查 gate 返回值：零值 branch 到 0x48011a70 并跳过 copy-back；非零值才从 RTC GP_DATA1 pointer 将 scratch buffer 内容恢复至固定与动态 DRAM 源窗口。context +0x4 == 4 同样选用 alternate strided restore。随后若 checksum descriptor enabled，0x4800c6c4 重新计算 checksum，与 pre-suspend sum 比较；不匹配则在 0x4800c6ec fail-stop。通过/未启用 checksum 后才恢复 CPUX_AXI bits，0x4800c6a8 通过保存的 return PC 间接返回；return continuation 0x48015d14 再恢复 SCTLR_EL3、VBAR_EL3、DAIF 与 stack selection。由此可确认静态代码顺序上 copy-back 位于 BL31 continuation / 后续 Linux 恢复之前，但不能证明本机实际 gate 非零或 copy-back 确实发生。
### RTC soft-entry 的 TWI/PMIC 候选路径（2026-11-27）

- RTC soft-entry continuation `0x48012670` 调用 `0x48012070`；该 helper 在 `0x48012090` 调用初始化包装 `0x480129f0`，再由 `0x480129f8` 调用 `0x48013ac8`。初始化 helper 按 context `+0xa4` 从 descriptor table `0x48014628` 选择对象，并将选中对象写入全局槽 `0x48015548`。运行时 selector 与最终 object pointer 未采集，因此不能指定 bus index。
- `0x48013ac8` 将 `0x33` 写到 R_PIO 基址候选 `0x07022000)，并向 `0x0701019c` 写 `0x1000)、`0x10001`；该 R-CCU 寄存器语义尚未独立确认。传输 helper `0x48013b98` 调用 `0x48013a48` 检查线路状态，再通过选中的 controller object 访问偏移 `+0x08` DATA、`+0x0c` CNTR、`+0x10` STAT、`+0x14` CCR、`+0x18` SRST、`+0x1c` EFR、`+0x20` LCR。它等待 LCR 值 `0x3a`，设置 CNTR START bit5，轮询 INT_FLAG bit3，检查 STAT `0x08`、`0x18`、`0x28`，并将 7-bit 地址左移一位后送入 DATA。其余分支包含重复 START/STOP/read/write 处理；具体交易序列取决于运行时 mode 与参数。
- 目标机 live FDT 报告声明 AXP2202 节点 `/soc@03000000/twi@0x07081400/pmu`，compatible `x-powers,axp2202`，status `okay`。同系列 H616 User Manual Rev 1.0 将 R-TWI0 映射到 `0x07081400)，寄存器偏移与上述访问吻合；H616 Linux DTS 将 R-PIO PL0/PL1 标记为 `s_i2c`，R-TWI0 也位于 `0x07081400`。这是强烈的同系列/FDT 候选对应，但不能证明 context `+0xa4` 选中的 descriptor 正是该 R-TWI0，也未证明目标 PMIC slave address 或交易寄存器。
- 此路径在 RTC GP_DATA2 `0x07000108` 写入阶段值 `0x22000000..0x22000004)，写入 PCs 为 `0x480122a0/0x48012338/0x48012354/0x48012370/0x4801238c`。随后在 `0x48012380` 调用 `0x480137c8)，参数 `w0=0)、`x1=0x1f4`；helper 访问 `0x03009010/0x03009014/0x03009018`，但 timer 单元、参数单位及其与阶段标记的语义尚未确认。GP_DATA2 标记的下游消费者也未知。
- 当前结论：**RTC soft-entry 包含 TWI master 配置/传输路径，并与 RG34XX FDT 中的 AXP2202/R-TWI0 配置相吻合；尚未证明运行时选择 R-TWI0、发出哪组 PMIC 命令，或最终执行 PMIC power-off/Boot0 转交。** 不将它命名为 PMIC 关机函数，也不推测 GP_DATA2 标记含义。
- H616 对照：[H616 User Manual, R-TWI0 base and register offsets](https://www.scs.stanford.edu/~zyedidia/docs/allwinner/h616.pdf)；[H616 DTS, R-PIO PL0/PL1 s_i2c and R-TWI0](https://gbmc.googlesource.com/linux/+/refs/heads/master/arch/arm64/boot/dts/allwinner/sun50i-h616.dtsi)。这些仅用于同系列对照，不代替 H700 原理图/运行时读回。
- 本轮未执行 I2C transaction、MMIO、SMC、PMIC/RTC 写入、DRAM 内容读写或待机；`H700_MEMORY_LAYOUT_APPROVED` 继续 unset，`hardware_applied=0`。

### RTC GP_DATA 与 DDR retention buffer 的供电边界（2026-11-27）

- 目标机 live FDT 报告 `reports/rg34xx-inventory-20261126-timer/live-fdt.json` 声明 PMIC compatible 为 `x-powers,axp2202`，并声明 regulator `axp2202-rtcldo` 为 1.8 V、`boot_on`、`always_on`；同一批 `dmesg-hardware-lines.txt` 显示内核已注册 `sunxi-rtc` 为 rtc1 并打印 `RTC enabled`。这些是现有设备报告中的 DT 配置及启动日志，不是 VRTC/RTCLDO 实测电压，也不证明具体板级走线。
- Tina PMU guide 的 AXP2202 示例将 `axp2202-rtcldo` 标注为固定 1.8 V、always-on regulator；该资料用于交叉核对 DT 配置语义，不是 RG34XX 专用原理图或 PMIC 输出读回。
- Allwinner H616 User Manual Rev 1.0（同系列参考，非 H700 精确封装/板级规格）说明 RTC 在 system power-off 时可由独立电源继续工作，RTC 有 16 个用于 power-off information 的 32-bit GP_DATA registers；GP_DATA 仅在 RTC-VIO 高于 0.7 V 时保证保存。手册还特别说明 H616 package 的 VCC_RTC/RTC_VIO 实现有封装限制，因此不能把该封装电气说明直接套用到 RG34XX H700。
- 对本项目，`0x07000104`（GP_DATA1）保存的是 `0x4801b400` retention buffer 的 pointer；RTC `0x070001f8/0x070001fc` 保存 standby flag 和 soft-entry return PC。**GP_DATA 能保持，不等于 pointer 指向的 640-byte payload 能保持。** `[0x4801b400,0x4801b680)` 位于 DRAM 地址空间 `[0x48000000,0x49000000)` 的 BL31/monitor carveout 内，不是 RTC GP_DATA，也不是 SRAM A1/C。它的字节内容只有在 DDR 保持供电/自刷新或由其他保持介质另行备份时才可能跨该睡眠路径保留。
- 因此 RTC soft-entry 分支将下一个问题分成两个独立的电气/内存保持门槛：A) RTC-VIO/AXP2202 RTCLDO 及 RTC GP_DATA 是否在目标分支期间保持；B) DRAM/monitor carveout 中的 `0x4801b400` 备份数据是否保持，及 DDR training/restore 的实际先后关系。现有报告仅对 RTCLDO 的 DT 声明和 RTC enabled 启动日志提供证据；没有板级原理图、实际 rail readback、DDR self-refresh/power-down state 或 RTC/DRAM 跨待机实测。两项均不得视为已闭合。
- 本轮没有发起 I2C/PMIC read、MMIO、SMC、RTC 写入、DRAM 内容读写或待机测试；`H700_MEMORY_LAYOUT_APPROVED` 继续 unset，`hardware_applied=0`。

- **运行模式分支（新增精确解码）：** context pointer slot 为 `0x48014668`。`0x4800c4ec` 的 TBZ bit0 跳往 `0x4800c530`；bit0 清零才继续检查 bit1。此时 bit1 清零进入普通 WFI/poll：`0x4800c53c → 0x48013df0` 配置 wake source，WFI 位于 `0x4800c550`，`0x4800c568 → 0x48013ee8` 检查唤醒条件，`0x4800c56c` 在 poll 返回 0 时回到 `0x4800c540` 再次 WFI。bit1 置位则由 `0x4800c530` TBNZ 跳到 RTC soft-entry `0x4800c580`：写 `0x10000` 到 `0x070001f8`，从 context `+0xc4` 取返回地址写入 `0x070001fc`，并将多个 context 字段写入 RTC GP_DATA 窗口，最后在 `0x4800c5fc` 跳到 `0x48012670`。若 bit0 置位，则不会检查 bit1，而是读取 context `+0xc8`：零值走单次 WFI `0x4800c50c` 后返回；非零值左移 4 位后由 `0x4800c610` 跳入 `0x48014078` 计数循环，完成后返回且不经过该 WFI。四类路径的 runtime flags/count 均未知；不能从静态存在的分支推断设备使用其中哪一种。
- **对 PFN 0x40000 的正确解释：** Linux 报告证明 [0x40000000,0x40001000) 在采样时是 active anonymous mapped page；BL31 只 snapshot 该页开头 320 bytes，而不是完整 4-KiB page。静态路径因此显示固件设计上似乎保存/恢复该页内的特定数据窗口，这解释了为何源地址落在 Linux System RAM 中并不必然表示固件要长期独占整页。不能由此批准 Linkslot 分配该页，也不能断言保全机制已成功。
- **剩余硬门槛：** 运行时 context +0x1c bit0、+0x4、geometry selector、RTC GP_DATA1 pointer、gate result、optional checksum descriptor 最终值和 buffer 内容均未知；sleep context +0xcc/+0xc8 实际值与选中分支未知；RTC_VIO rail 与 GP_DATA retention、DDR scratch `[0x4801b400,0x4801b680)` 在目标模式下的保持、CPU/DMA quiescence、cache coherency/所有权，以及 SRAM A1/C 保持能力也未闭合。需要这些证据后才可评估实际 suspend 安全性。保持 H700_MEMORY_LAYOUT_APPROVED unset、hardware_applied=0；未发起 SMC、未进行 MMIO/DRAM 内容读写或实机待机/唤醒。

### 证据边界与下一步

- `scripts/audit-rg34xx-bl31-psci-cpu-on.py` 与 host test 现锁定：9 项 legacy platform-ops、affinity callback map、特殊 SYSTEM_SUSPEND 入口、SRAM code/context staging、clock/PLL restore、wake-source helpers、checksum 默认 descriptor 与 arm_svc SMC override、RTC GP_DATA1 retention buffer、双窗口 snapshot/restore、动态 bank selector、restore gate、DRAM clock restore、mode-dependent DRAM_CTRL/PHY_CTRL setup、MSI_CTRL fallback 及 0x047Fxxxx MMIO 地址构造。BL31 audit #76、#78、#79、#81、#82、#83、#85、#86 和 #87 通过；#85 验证 DRAM page classifier，#86 加入真实 PFN kpageflags/kpagecount 解码，#87 锁定 cache teardown 的 DC CISW dispatch 与 DSB/ISB 顺序。另已审查 SenkOS Super Standby 参考实现与其 pinned stub 的 DRAM write-probe 风险。所有结论仍是匹配 binary 的静态证据或独立参考实现对照。
- 关键指令级断言：slot 0 `+0x00` / `0x48009688 → 0x48009690`；slot 1 `+0x08` / `0x48008608 → 0x48008640`；slot 2 `+0x10` / `0x480080f8 → 0x48008118`；slot 3 `+0x18` / `0x48008a84 → 0x48008a90`；slot 4 `+0x20` / `0x4800854c → 0x48008560`；slot 5 `+0x28` / `0x48008918 → 0x4800893c`。slot 6 `system_off` 从 `0x480025a8` 调用 `0x48001ed8`；slot 7 `system_reset` 从 `0x48002588` 调用 `0x48001ec8`，并有 `Sunxi System Reset` 字符串证据。
- slot 8 `get_sys_suspend_power_state` 位于 `0x48002544`，执行 `mov w0, #0x02010000; ret`。SYSTEM_SUSPEND caller `0x480096f8` 从表偏移 `+0x40` 加载并在 `0x48009744` 间接调用，随后在 `0x4800975c` 调用 transition function `0x48008d98`。
- `0x48008d98` affinity 调度、standby image 搬运、RTC wake continuation、PLL CPUX save/relock、R_PRCM PLL LDO key、RTC XO_CTRL 与 GIC/IR/GPIO wake helper均已静态映射。DRAM retention restore gate 也已扩展到 DRAM clock/PLL、MSI_CTRL/DRAM_CTRL MMIO 访问和 fallback controller setup；#86/#87 已锁定 cache teardown、snapshot-before-sleep、gate-before-copyback 的静态顺序及 cache dispatch 选择。仍未知的是 live selector、context flags、probe gate result、RTC soft-entry 分支、H700 register state 与 DMA/CPU/cache ownership。下一阶段应先寻找可用且不触发 SMC 的既有只读数据源，确认这些 runtime 条件；随后核对 SRAM C 生命周期/时钟门控、RTC wake source 与 CPUX_AXI bit26。未满足前不触发 suspend。
- 上述结果是**匹配 BL31 二进制的静态 ABI/控制流证据**，不是设备已成功休眠、唤醒或执行 CPU_ON 的证明。没有执行 PSCI SMC，没有采集运行时 trace，也没有进行 MMIO 读写；没有对 SRAM、RTC、PMIC 或 CCU 做 runtime probe。
- 精确的 Allwinner/vendor TF-A 源码 commit 仍未恢复；目前识别的是二进制有效成员顺序与语义，不声称已经找到出厂构建的逐字源码。
- CPU_ON 回调存在不等于初始 BL31→BL33 handoff 自动启动了 CPU1–CPU3；secondary A53 的 reset/hold-off 状态仍需独立只读验证。
- SYSTEM_SUSPEND getter 返回有效 power-state 也不等于整套 DRAM self-refresh、唤醒源、时钟恢复与 Linux suspend/resume 已在 RG34XX 实机验证。
- CCU helper 属于 suspend/resume 上下文保存候选，不构成 CPU DVFS 接口，也不证明 BL33 启动阶段发生 PLL 切换。
- **下一步：** 不再重复静态 disassembly。先寻找现有只读接口或已有状态快照，确认 context +0x1c bit0、context +0x4、geometry selector、restore-gate result、checksum descriptor final tuple、RTC GP_DATA1 pointer，以及 sleep selector context +0xcc/+0xc8；如果这些值没有现存只读来源，记录为不可观测，不为获取它们而发出 SMC 或增加 MMIO probe。并行审查两条互不替代的 retention contract：RTC-VIO/AXP2202 RTCLDO 是否维持 GP_DATA 元数据，和 DDR 是否在保留 `0x4801b400` scratch payload 时仍处于供电/自刷新状态；同时核对 SRAM A1/C retention、wake source 供电及 CPU/DMA quiescence。进一步对 `0x48012670 → 0x48012070 → 0x480126a4` 终止分支做静态 caller/helper 审查，但不把 helper 名称或调用形状推断成 PMIC power-off。SenkOS 的 Boot0→DRAM resume→restore 0x280 bytes 是另一套 PMIC cold-resume 实现；本项目 stock BL31 已精确区分普通 WFI/poll、RTC soft-entry、single-WFI 与 counted-delay 四条静态路径，但设备实际分支尚未观测。若未来移植自定义 stub，先修复 mctl_check_memory() no-write 问题。`H700_MEMORY_LAYOUT_APPROVED`、实机 suspend 和所有硬件 writer 继续关闭，直到 runtime gate、两类 retention、buffer retention 和回滚条件均有证据。
- 全程未执行实机 MMIO、存储写入、刷写或 payload；`hardware_applied=0`。PI12 production MMIO 仍 default-off，`H700_MEMORY_LAYOUT_APPROVED` 仍 unset。


**2026-11-26 SYS_CFG / RVBAR 只读预检续接：**

- reports 分支最新预检提交为 `2963c8c479486a98c5202dad86e14dbe6e0f466f`，报告 `reports/rg34xx-kmod-preflight-20261126/`。设备为 Linux `4.9.170 #19`；`CONFIG_MODULES=y`、`CONFIG_KALLSYMS=y`、`CONFIG_DEVMEM=y`，但无 `/lib/modules/4.9.170/build`、kernel `Makefile`、`Module.symvers` 或 `/usr/src/linux*`，当前系统无法自行构建匹配的外置模块。debugfs/regmap 没有 SYS_CFG 实例，故不重复尝试用户态 `/dev/mem`。
- **新确认的 LK 源码事实：** `platform/h700/arch/h700_pio_mmu_mapping.c` 在 LK `arch_early_init` 中把物理区间 `[0x03000000, 0x03100000)` identity-map 为 Device/Shared、RW、XN。SYS_CFG `VER_REG=0x03000024` 与 PIO `0x0300b000` 同处该 1 MiB section。新增静态断言与 host regression 明确锁定两者都在该映射内；这只证明页表描述符覆盖，不证明芯片访问权限、总线响应或寄存器值。
- 下一垂直切片是设计一个**带 Data Abort 保护的单寄存器只读诊断**，且先确定最小结果回传方式，再考虑在 LK early hook 中执行。裸 `volatile` 读若触发 abort 可能破坏启动，不能直接加入 target_init。普通 LK 运行阶段读到的 VER_REG 也不能替代 pre-BL33 的 CPU reset/RVBAR 状态快照。
- 没有加载模块、没有 MMIO 读写、没有改变 boot args/Boot0/TOC1，也没有执行新 BL33；`hardware_applied=0`。SMP、内存布局批准与刷写仍保持关闭。

**2026-11-26 实机输入验证推进：** 已完成 Linux evdev 与 gpiolib debugfs 两条独立只读实机验证。reports 分支提交 `a67f12aa0809ed71bbbbbae685fc0fb77022aae9` 上传完整采样：1200 个 `cat /sys/kernel/debug/gpio` 快照，设备 uptime 301.83–427.83 s（126.0 s）；17/17 实体游戏按键分别在唯一 GPIO 行上产生一次 `hi→lo→hi`，共 34 个变化点。对原始文件逐快照解析，未发现非按键 GPIO 变化、同一采样点多条线变化或重复边沿；17 条线均为 active-low 输入，PA0–PA12 与 PE0–PE3 对应关系和实体 X/Y 丝印交换再次确认。采集仅读取已挂载的 debugfs 文件与 `/proc/uptime`，未 mount/export/request、无 MMIO/寄存器写入，`hardware_applied=0`。采样周期实际约 105 ms，不能用于接点 bounce 或 20 ms polling/debounce 定标。`/dev/mem` 的所有探测均 EFAULT，不再尝试。Linux GPIO driver readback 已通过；LK 直接 PA_DAT/PE_DAT 路径、早期 pin ownership、LK input scheduling 及 LK 实机启动仍未验证，sampler 不接入 runtime。

**2026-11-26 RG34XX live-FDT 输入映射与逻辑边沿层：**

- reports 分支提交 `3de7f40d2d9f` 上传脱敏实机 live-FDT `input_devices`。该 FDT 为 143,936 B / 573 nodes；`/soc@03000000/gpio_keys`（`gpio-keys-polled`）状态 `okay`，17 个子节点全部 `okay`，声明 `poll-interval=20 ms`。17 个按键均解析到 `allwinner,sun50iw9p1-pinctrl` phandle `0x4b`、PIO base `0x0300b000`；声明 mux=0、pull=1、drive=2、active-low。
- 固定版本 sunxi 内核头文件确认 pull=1 是弱上拉；上传报告文字中的“unpulled”需要更正。GPADC 与 `keyboard_1350mv` 节点禁用；AXP2101-PEK powerkey 为 `okay`，且独立于 17 个游戏按键。FDT 不证明 Linux driver binding、实际电平或物理按键事件。
- 新增 `h700_rg34xx_input_map.c/.h`：用 17 键实机映射把调用者提供的 PIO bank A/E 电平快照转换成逻辑 press/release edges，送入既有 bounded input queue。纯软件，不含 MMIO、GPIO claim、pinmux/pull 写入、debounce、timer 或 EINT/GIC。
- 新实机报告 `reports/rg34xx-input-runtime-20261126/` 已闭合 Linux evdev 输入路径：`ANBERNIC-keys` 为 `input1/event1`，72 条原始事件记录覆盖全部 17 个实体按键；一次额外 R1 是人工误触。设备没有 `getevent`，因此从 event1 只读采集并离线解码。独立 X/Y 复测 6/6 确认实体 X=PA2/code307、实体 Y=PA3/code306，已修正软件逻辑映射。
- reports 分支 `reports/rg34xx-gpio-debugfs-20261126b/`（提交 `a67f12aa0809ed71bbbbbae685fc0fb77022aae9`）新增 1200 个完整 gpiolib debugfs 快照。逐行解析原始 2,982,090-byte 文件确认 17/17 GPIO lines 各仅有一次 press/release（34 transitions），没有额外或同时变化的 line；physical X→gpio-2/PA2/keyY，physical Y→gpio-3/PA3/keyX。此为 Linux GPIO chip `get()`/debugfs 读回证据，不是 LK raw PIO MMIO readback。详见 [GPIO-DEBUGFS-TEST.md](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-gpio-debugfs-20261126b/GPIO-DEBUGFS-TEST.md)。
- pinmux/pinconf 调试视图不一致：PA0–PA12/PE1 显示 MUX UNCLAIMED，PE0/PE2/PE3 显示 CSI1；全部 pinconf 显示 bias disabled/20mA。真实 evdev 事件证明按键路径正常，但调试视图不能作为写寄存器依据。未知的 `dierct-keys-polled`/event2 不并入游戏按键。
- 新增 `h700_rg34xx_input_scan.c/.h`：单次只读 PA_DAT offset `0x10` 和 PE_DAT offset `0xA0`，接入已修正的 17 键 edge mapper。接口只有 read32，无 write32；不动 pinmux/pull/drive、debounce、timer、EINT/GIC；函数尚未从 LK 启动或主循环调用。
- 新增 mock-PIO regression 校验只读寄存器 offset、X/Y 实际逻辑键边沿与 ABI 错误拒绝。H700 AArch64/AArch32 ARM/Thumb-2 编译及 storage/board host regressions 已通过 [#45761004754](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45761004754)；boot-audit Linux/Windows 验证通过 [#45761004741](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45761004741)。完整 Linkslot feasibility [#45761004736](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45761004736) 仍在运行，不把它计作通过。


**2026-11-26 LK heap readiness 衔接更新（代码提交 `aca39f29c560`；三条关联 CI 工作流全部通过）：**

- 固定 GammaOS LK 9bf1419a49e0af6dba8b05916df382c9c9c6d0a8 的正常启动顺序为 target_early_init → heap_init → thread/DPC/timer → platform_init → target_init → apps_init；heap_get_info() 的 free-list 遍历受 LK critical section 保护。
- 发现 target_init() 已调用 h700_lk_heap_service_mark_target_init_complete()，但 platform/h700/rules.mk 尚未把 runtime/lk_heap_service.o 加入真实 LK OBJS。现补齐对象图，并在 project guard 中检查固定 LK 源码顺序、heap API/临界区和对象清单。
- 特殊路径：pinned LK 的 ENABLE_NANDWRITE bootstrap 会在未调用 heap_init() 时调用 target_init()。H700 marker 现对此路径 fail-closed；normal 与 ENABLE_NANDWRITE=1 两种 host 回归均已通过。H700 编译 CI 确认 heap service object 在 AArch64、AArch32 ARM-state、AArch32 Thumb-2 三种变体均可编译；full feasibility project/memory guard、BL33 CRT0/vector、Linkslot Frontend、no_std crates、VFS/mGBA 静态链均在 #45752237141 通过。artifact: [linkslot-lk-rust-feasibility-45752237141](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45752237141)。
- heap readiness 只表示 LK free-list 当前有非零可用空间，不是内存布局批准、heap 容量保证或完整 newlib/runtime 接入。完整 LK target link、物理内存 ownership 和设备启动仍未完成。



**2026-11-26 H700 Generic Timer 只读 provider：**

- Linux live snapshot `reports/rg34xx-inventory-20261125-163152/interrupts.txt` 显示 GIC-0 IRQ 27 `arch_timer` 在四个 CPU 列均有计数，另有 wakeupgen IRQ 3 `sun4i_timer0`。这证明 Linux 运行态架构计时器在工作；它不单独证明 BL31→LK 的计数器频率、virtual offset 或中断交接状态。
- 按 Arm ARMv7-A/R Generic Timer 访问规则，`CNTVCT` 对 Secure PL1、Non-secure PL1/PL2 可读；`CNTFRQ` 在 Non-secure PL1 为只读。与之相反，Non-secure PL1 读取 `CNTPCT` 在实现 virtualization extensions 时受 `CNTHCTL.PL1PCTEN` 控制。因此 H700 选择 `CNTVCT + CNTFRQ`，不读取物理计数器，也不碰 `CNTKCTL/CNTHCTL`。本项目当前 BL33 入口证据是 AArch32 SVC（PL1）。
- 新增 `platform/h700/runtime/h700_arch_timer.c`：AArch32 使用 `MRRC p15,1` / `MRC p15,0`，AArch64 使用 `CNTVCT_EL0/CNTFRQ_EL0`；service 以初始化时计数值为 epoch，提供相对毫秒单调时间，零频率 fail-closed，并处理 64-bit counter wrap 与转换饱和。
- 该 provider 只读寄存器，不设置 timer compare、timer control、GIC/PPI，也不实现 `platform_set_periodic_timer()`。host tests 验证换算、ABI 生命周期和边界；AArch32/Thumb-2/AArch64 CI 验证指令编译。尚未在 RG34XX LK 中执行读取，所以真实 `CNTFRQ`、计数增长、BL31 handoff 后连续性仍待后续设备验证；先维持单核。
- 不复用 pinned MTK `platform/mt6785/timer.c` 的 GPT5。下一步是先完成只读 provider 的 CI，再从真实 LK/设备验证计数器读回；只有有明确 PPI/IRQ 依据后，才另行实现周期 timer。

**2026-11-26 RG34XX GPIO input source audit / live-FDT parser（代码提交 `a2f93a0ffbfd` + parser fix `e8dd77cf86db` + GPIO list fix `3f1d64f5a2c1`；CI 全部通过）：**

- KNULLI RG34XX boot-package DTS（commit `57afa662009641d181f57241bcc480119b131530`）与 MustardOS RG34XX-H DTS（commit `8b7787a178548d51bc5376d52583dd2a91f5091f`）的 `gpio-keys-polled` 都声明同一组 17 个 GPIO 按键；方向键以 EV_ABS 上报，其余按键以 EV_KEY 上报。两个参考 DTS 的 polling 分别为 20 ms 与 4 ms，不能据此提前决定 LK 扫描周期。
- 对照 sun50iw9p1 pinctrl/GPIO provider，确认 6-cell tuple 为 port / pin / mux / pull / drive / flags；最后一格 bit0 是 `OF_GPIO_ACTIVE_LOW`。这只是 DTS 声明，尚未与本机 active live FDT、pinmux/pinconf readback 或实测按键事件核对。
- 两份参考 DTS 的 GPADC 与 `keyboard_1350mv` 节点均 disabled；PMIC power key 另属一条路径，不并入 17-pin GPIO map。
- 扩展 `tools/inspect_h700_fdt.py` 输出 `input_devices`，解析 GPIO controllers、gpio-keys 子节点/六 cell 参数、GPADC/ADC keyboard 节点及 PMIC power-key 节点；`tools/test_inspect_h700_fdt.py` 增加合成 FDT 回归。首次 CI 暴露两个主机测试问题（PMIC root matcher 把 PEK 子节点也计入；GPIO phandle 解码漏 append）；均已修复。`#45755000872` Linux 与 Windows jobs 均通过，83 项单元测试每个平台通过；`#45755000890` AArch64/AArch32 ARM/Thumb-2 编译与 storage host regression 均通过。本次没有实现硬件 GPIO scanner，也没有 GPIO/PMIC MMIO 读写。
- 下一步用现有只读 inventory 重新采集目标机 live FDT，比较活动节点和输入驱动；只有 active FDT 与 pin ownership 证据闭合后才实现扫描器。

**2026-11-26 最新续接更新：**

- **ADB 显示复测：** reports 分支 `reports/slot-refresh-audit/` 保存 UI/GAME 两份只读原始输出。UI 12 s IRQ 293 Δ716 = 59.667 Hz；GBA/armGBA 12 s Δ715 = 59.583 Hz。两态 live panel 均为 `rg34xx_v1`、720×480、820×536、26-MHz DCLK property；PLL `0x03001040` 均读为 `0x88001703`。TCON divider `0x06511044` 本轮返回 I/O error，未取得直接读数。结论为两态均无 120-Hz 应用证据；IRQ 只是 scanout proxy，不是 UI present 或 game FPS。原始脚本没有访问 sunxi_dump/write。
- **校正高刷寄存器候选：** 固定 pvaibhav 源码 `5206e3fa39aa4a9b702cd9048a07dab3b5ab95da` 的高刷组合是 divider `0xf000000a` + PLL `0x88002b03`；60-Hz 组合是 PLL `0x88001703` + divider `0xf000000b`。ADB SUMMARY 中旧写的 `0x88001701` 不是该 fork 的高刷值，已在 reports/SUMMARY.md 校正。面板名 `rg34xx_v1` 不满足 fork 的 `fog_fj035fhd05_v1` 精确 gate，按该源码路径启动 setter 会被跳过；不能通过改字符串绕过。
- **Linkslot 1.0.49 全量 CI：** 最新 `#45646868006` 在当前检查 HEAD `018e0b30d14965b56003e23e06da5593a94808d6` 上 **71/71 steps 成功**，artifact `15038716689`。H700 boot-audit `#45646868087` 的 host-tools 与 windows-powershell 两个 job 均成功。两项均为 host/cross-build 和工具验证，不是完整可刷写固件或实机启动证明。
- **原始 RG34XX 120Hz 仓库对照：** 已直接复核 [TheGammaSqueeze/rgxx_boot_package_mods 的 rg34xx 分支](https://github.com/TheGammaSqueeze/rgxx_boot_package_mods/tree/rg34xx)，HEAD `64300b3b6137f7676b63b2e969e58db2870db48b`。它以两套 signed boot_package + FDT timing 实现全局模式：60Hz 为 54MHz / HT1679 / VT536，120Hz 为 54MHz / HT840 / VT536；两个 DTB 唯一 property diff 为 `lcd_ht`，U-Boot/monitor/DTBO 相同。其 flashscript 检查 `sunxi-package` 后直接写 `/dev/mmcblk0` 的 `0x1004000` offset 并 reboot；README 说明只在 Stock firmware 测试。它不是 Slot 运行时功能，也没有 UI/GAME 自动切换。与之不同，pvaibhav/slot 保留 26MHz/820×536 live tuple，在启动/唤醒时尝试写 divider10 + PLL528MHz，并做 readback/rollback；它同样是全局高刷偏好，不作场景切换。按匹配 kernel #19 反汇编推导，若 54MHz FDT 进入 Linux panel-init，则 `lcd_if=0` 使用默认 divider6，请求 PLL 324MHz（因子表精确有效档位），与当前 26MHz 特例 divider11/PLL288MHz 不同；这是源码推导，未在 BaseOS 刷包实测。原包名 `fog_fj035fhd05_v1` 与本机 Linux live FDT `rg34xx_v1` 的差异仍未闭合。按源配置名义算术为 54MHz/(840×536)=119.9360Hz，略低于 README 报告的119.947Hz；文档现已区分 source-reported 与 nominal rate。禁止把原始 boot-package 或任一寄存器序列直接写入 BaseOS。详见 [专项对照](slot-pvaibhav-120hz-review.md) 与 [策略/时序文档](h700-display-refresh-policy.md)。
- **下一显示测量：** 1) 先核对设备已安装 `System/slot` SHA `8f0fd40da4ac161e5fbb9a56a99c254acf8b2f31806e2be6bdaa929b5ee28505` 与 `armgba_libretro.so` SHA `4ebae098cc4848fb459fc18299c39acbdbbbeb732db5c3f66d1ff3f6e142e889` 是否匹配官方 H700 release ZIP；2) 若 Slot 二进制与固定源码一致，利用启动前 `SLOT_TRACE_LATENCY` / `System/trace-latency.on` 启用已有 600 次成功 `surface.swap()` 计数，单独测 UI presents/s；3) armGBA 必须读取其专有 `gba->video.frameCounter`，不能复用 pvaibhav `probe.rs` 中指向 mGBA 的 `gba` selector；先验证 ELF symbol/ABI/offset，再做只读进程内存采样。不得用 IRQ 替代 FPS，也不得写显示寄存器。



**Linkslot 1.0.49 runtime-slices 迁移验证（2026-11-25）：** [#45624285300](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45624285300) **成功**。完整 job 通过：H700 no_std workspace host tests、AArch32/AArch64 freestanding checks、renderer + business overlay 应用、上游 input/model regression、Linkslot 1.0.49 完整 Frontend + SoftwareCompositor 的 `cargo check`、CPU Performance QuickRow unavailable/不写存储的 2 项回归，以及 H700 C input ABI/queue job。首次 1.0.49 run [#45623895416](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45623895416) 曾因 H700 `slot-link-kind` 未提供上游新增的 `armgba_link_carried` / `armgba_port` 失败；已补齐 no_std 实现、测试和 re-export，第二轮通过。此结果仍是 host/cross-build，不是 LK 完整镜像或实机运行。

**Linkslot 1.0.49 综合 feasibility CI（2026-11-25）：** 完整 workflow [#45625386428](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45625386428) 在 static mGBA selector test 失败；上游 1.0.49 已将联机参数改为 `Option<LinkSetup>`，旧测试 patcher 仍传 `Option<u8>`，引发两个 E0308 类型错误。已在提交 `d333eb30dc8a21b4e9b9509bc08a3760223c091a` 修复：helper 改用 `Option<LinkSetup>` 并与动态路径一致设置六键手柄。修复后的 workflow 正在重新验证；此前 1–31 步通过，32 步失败，之后步骤被跳过。不得把该 run 标为通过。

**Linkslot 1.0.49 第一轮迁移（2026-11-25）：** 持久化 `slot-model/src/core.rs` 已同步 main 1.0.49；`CoreSpec::netpacket` 已纳入 H700 no_std 模型，gpSP/armGBA=true、mGBA=false，并在 host contract test 中锁定。crate 版本已更新至 1.0.49。Frontend 与 renderer patcher 的 source pin/workspace version 已指向上游归档 HEAD `241e7c1974926fa22552b200852914ceaccbfd42` / 1.0.49；workflow [#45623895416](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45623895416) 中 no_std workspace tests、overlay、upstream input/model regression 已通过，完整 Frontend cargo check 截至记录时仍在运行。未标记完整迁移通过。详见 [更新审查](linkslot-1.0.49-1.1.0-refresh.md)。

**Linkslot 新基线的 H700 代码差异补充（2026-11-25）：** 持久化 `integration/linkslot/h700-frontend-core/slot-model/src/core.rs` 仍是 1.0.45 风格，没有 1.0.49 `CoreSpec::netpacket` 字段，armGBA 仍被注释为单人核心；而 1.0.49 已声明 gpSP/armGBA netpacket=true、mGBA=false。需同步核心元数据与测试，才能保留上游 1.0.47 起的 armGBA 联机语义。H700 `machine.rs` 保留六个首期机种是有意的；DEV Arcade/NeoGeo 不应在首期引入。现有两条 patcher 仍 pin 1.0.45，且 runtime workflow checkout 的 `upstream/linkslot` 已是 1.0.49 归档头；不更新 pin 和锚点前，严格版本门会拒绝新快照。详见 [更新审查](linkslot-1.0.49-1.1.0-refresh.md)。

**Linkslot 上游源码更新（2026-11-25）：** 用户已更新三个来源分支，均已归档在 `upstream/linkslot`：main 1.0.49（原始 commit `b84f37f1a6259a625e9095279dd132f5ef2116af`）、DEV 1.1.0（`a800adb2e3a51d05ffe80abeae2e1ec52247b951`）、test 1.0.49（`8c7a37055eb68f71170320d4a17cdb9251785186`）。本仓库归档 HEAD `241e7c1974926fa22552b200852914ceaccbfd42`。main/test 的 851 个源码文件逐 blob 一致；DEV 比 main 新增 21、删除 1、修改 46。稳定移植基线改为 main 1.0.49，DEV 1.1.0 单独审查，不将 Arcade/NeoGeo/FBNeo 纳入首期 GB/GBC/GBA。关键兼容信息：旧 1.0.45 与三份新快照的 `slot-retro/src/ffi.rs` blob SHA 完全一致（`2e7b1b0bb3e1665de01f6458b6dfc7fe65a61f34`）；但 `slot-model/src/core.rs` 和 `slot/src/app.rs` 已改变，旧 patcher 仍锁 1.0.45，须在 1.0.49 上重跑锚点、overlay、静态 API 和完整 workflow，不得只修改版本号。详情见 [Linkslot 1.0.49/1.1.0 更新审查](linkslot-1.0.49-1.1.0-refresh.md)。  

**最新显示刷新实机结论（2026-11-25）：** pvaibhav/slot 在同一台 RG34XX non-SP 的 UI/GAME 两轮均约 59.6 Hz；未观察到 120 Hz 或场景切换。完整 live FDT 已上传并解析为 `rg34xx_v1` / RGB666 / 720×480 / DCLK property 26 MHz / HT×VT 820×536 / `lcd_type=old`，DTB 派生 nominal 59.155 Hz，显示 IRQ 计数斜率约 59.63 Hz（仅为 scanout/vblank proxy，不等于已确认的面板物理刷新率），TCON0 DCLK divider 已读回为 11（`0xf000000b`），PLL raw value 为 `0x88001703`；实际 DCLK 仍是基于 288 MHz / 11 的推算值，未做独立物理测量。Pinned BaseOS survey 的 vendor profile 是 PE6 high→old、low→boe，目标 rg34xx 使用 `rg34xx_v1`；其 U-Boot panel struct 选择 `rg34xx_v1`，虽调用一个 symbol 名含 `fj035fhd05` 的共享 init routine，但该函数标签不是 FDT 驱动名。pvaibhav fork 的 `rg34xx→fog_fj035fhd05_v1` allowlist 因而不匹配标准目标 profile，不能直接移植 writer。设备 Slot/core 文件 SHA 已记录，但尚未与 release ZIP 解包文件比较。详情见 [实机分析](pvaibhav-slot-rg34xx-live-capture-2026-11-25.md)。硬件 writer 保持禁用、`hardware_applied=0`。  
**当前文档/工具验证：** 最近一次已完成的 H700 boot audit tools [#45617713806](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45617713806) 已成功：Linux host-tools、Windows PowerShell 两个 job 均通过；此前 #45617414175、#45616919646、#45616740899、#45616747528、#45616754133、#45616761051 也成功。完整 Linkslot/LK feasibility 的既有通过记录为 [#45606529437](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45606529437)；较新的再跑 [#45616741294](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45616741294) 后被更新提交触发的并发取消；文档更新触发的 [#45617402177](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45617402177) 在本次检查时仍为 in_progress，未计为通过。
**最近一次完整 feasibility 通过的 HEAD：** `8d44874db413c6bd5be71a293edb2b34682b4ba5`（[workflow #45579608578](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45579608578)，71/71 步骤成功；工件 ID `15005762318`）。
**当前显示刷新策略实现提交：** `40113045a685413fb6887c621d9e0e982aee9303`，新增独立 `game_120hz_enabled` C policy 开关（默认关闭）。对应 AArch64/AArch32 ARM/Thumb-2 编译和 storage host regression [#45581459152](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45581459152) 已通过；Linux/Windows host-tools audit [#45581459275](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45581459275) 已通过，其中 `test-h700-display-refresh-policy.sh` 明确 PASS。完整 Linkslot/LK feasibility [#45581815012](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45581815012) 尚未取得终态；不将新提交标为完整 feasibility 通过。
**本轮 CPU runtime owner 最新代码提交：** `072793f8b4bfce00683aa3d303eee62613fe41ab`（7 个连续提交）  
**本轮 Rust runtime bridge 最新代码提交：** `0d381d7ba66592b5ab64ba67e0b228039bb9eec0`  
**本轮 H700 build graph / guard 最新代码提交：** `6e847c0121431c681062053c52f28824715e78a6`（platform OBJS）与 `ef07a0e06c1d55e2d2f49175d4e976e8337e1ee4`（project guard）。  
**本轮 H700 target lifecycle hooks：** `target/rg34xx_h700/init.c` / `target/rg34xx_h700/rules.mk`；三种目标 ISA 编译由 #45392910394 验证通过。  
**本轮 CPU inventory/FDT parser：** 解析器与 provider argument 测试由 #45396816432 通过。  
**本轮 CPU power collector missing-state 增强：** `273895925554446bff3eb3f61ca1dde093d15d03`；对应 safety regression 提交 `f2ea2b9d5611b5461546bc4f07fe014f9b4d80b2`，两平台 CI 均通过。  
**本轮 SID 窄读取与解析器：** `tools/inspect_h700_sid_cells.py` / `tools/test_inspect_h700_sid_cells.py`；PowerShell runner 输出 `sid-cell-report.json`；Linux/Windows CI #45399902110 成功。  
**本轮 SoC revision 取证边界：** collector 只探测通用 `/sys/devices/soc0/{machine,family,soc_id,revision}`；不调用 vendor `sunxi_get_soc_ver()` / `sunxi_info`，因为固定 `sun50iw9` 驱动路径可能短暂写 CE CCU clock/reset 且 sys_info 暴露 serial/chipid。新 safety regression #45401913670 与最新版 inventory audit #45402146927 均通过。  
**本轮 SID JSON SoC identity：** `sid-cell-report.json` 现附带 generic SoC bus 的 machine/family/soc_id/revision 原始字符串（可用时），仅用于识别，不参与 OPP 放行；相关 host/Windows tests #45402936876、#45402977130 与 inventory audit #45403070519/#45403377522 均通过。  
**本轮 mainline speed-bin reference decode：** 固定上游 Linux `8d7fe9885c62fc9a95bf55d9db244d9672a8aadc` 的候选映射现加入 `sid-cell-report.json`，但所有结果仍 `opp_authorization=not_approved`；#45404785874/#45405024561 已通过。  
**本轮 Linkslot/mGBA 静态闭环：** #45413653400 成功（70/70）；真实 Linkslot 1.0.45 `slot-retro::Api` Rust `no_std` AArch32 staticlib、21 个 mGBA namespaced exports、真实 mGBA VFS v3 archive、H700 newlib adapter 与 `newlib/libm/libgcc` 最终 ELF32/ARM 链接通过；artifact `14929808289`。该探针不执行核心，也不是完整 LK/固件/设备验证。  
**本轮 CPU Performance QuickRow App regression：** 两条 workflow 都实际执行 `h700_cpu_performance_quick_row_tests`，2 passed / 0 failed；runtime-slices #45428654903 成功（20/20），完整 feasibility #45428664653 成功（71/71）。测试验证 unavailable 显示、左右请求不改变值、不创建持久化配置。
**本轮 RG34XX CPUX live inventory（采集于 2026-11-24 23:37:46 UTC）：** 已分析 `reports/rg34xx-inventory-20261125-cpux/`。live FDT phandle `0x4f` 将 CPU0 `cpu-supply` 指向 AXP2202 DCDC1；Linux regulator debugfs 报告 cpu0 1160 mV。CPU0–CPU3 在 Linux 中属于同一个 cpufreq policy 且全部 online；当前 Linux clock tree 为 `pll_cpu → cpu → cpuapb`（1512/1512/378 MHz），AXI 756 MHz，独立 `pll_periph0` 600 MHz。首轮 cpufreq 与 clk_summary 分时读取，不能据此认定 provider 频率不一致。SID speed raw `0x6c00`；mainline candidate bin 5 仅为诊断，不是 OPP 授权。不同厂商 4.9.170 源对 `0x6c00` 分别映射 mask 4 / mask 1，4.9.191 另有长度检查的 mask 1 实现，当前运行内核来源仍未知。温度 AVCC 未校准。完整结论：[RG34XX CPUX live read-only audit](rg34xx-cpux-live-audit-2026-11-25.md)；原始文件位于 [reports 分支](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux)。
**本轮 CPUX vendor-source audit（2026-11-25）：** 固定审查 `qiaoweibiao/T507_Kernel@98fcc7d3f112e51b0edfb71536da89cb2115106f` 的 `sun50iw9` clock driver；源码给出 `PLL_CPU=0x0000`、`CPU_CFG=0x0500`、`pll_periph0` pre-rate parent switch / `pll_cpu` post-rate restore、PLL lock 新模式以及 PLL bit31 boot-enable / bit27 output-gate 约束。`LOCKBIT(x)` 受 EVB build config 影响；厂商 CPU factor table 自身标注不正确，禁止用作 RG34XX OPP。审查报告：[H700 CPUX clock-source audit](h700-cpux-clock-source-audit.md)。**仅源码审计，不是目标板寄存器读回，不关闭 DVFS 硬件门，也没有新增 CI 结果。**
**本轮 Linux 4.9.170 cpufreq/OPP 源码核查：** 两个相同版本的公开 vendor tree 对 raw `0x6c00` 的映射不同：T507 候选无 case、default=`supported_hw=0x4`；Lineage 候选显式映射 `supported_hw=0x1`。两者都以 `u32 *` 读取 2-byte SID cell 且不校验 `len`。Creality 4.9.191 候选映射为 mask 1，但使用长度检查的 byte-wise read，不能证明目标 4.9.170 binary 的实现。版本号不够识别 running source，任何 mask 均未授权。
**历史 parser-v1 限制（已由 parser-v3/v4 采集关闭）：** 最初的 `live-fdt.json` 没有命名电压与 table-level speed NVMEM 信息；parser-v4 inventory 现已包含 38 个 OPP 节点的 property-presence、named-voltage variants 和 speed NVMEM reference。结合 matched shipping binary 的完整 CFG/OPP-core 反汇编，no-mask+b1 是高置信软件选择模型；不需要重新导出 boot 或 DTB。
**本轮只读 clock inventory 变更：** `scripts/collect-rg34xx-readonly-inventory.sh` 在原有完整 `clk_summary` 输出外，新增保留缩进的 `pll_cpu` / `pll_periph0` / `cpux` / `cpu` 重点摘录；`tools/test_inventory_script_safety.py` 增加只读回归，`.github/workflows/h700-arch-check.yml` 将两个文件加入 push/PR 路径并执行该测试。新摘录只读 Linux debugfs 文本，不直接读取 `PLL_CPU` / `CPU_CFG` MMIO。摘录片段单独通过 `sh -n` 语法检查；完整 Python 回归/Actions 尚未取得可核验执行结果。相关提交链：collector 摘录 `db9e190619b149901b817b67d899c0e937891190` → printf 格式修正 `c30dceea131f2a1cf168bdf08beac36884ce52a3` → 静态安全测试 `aad8671ece7f39124a58aa659c54bc3a2c0ff661` → workflow 接入 `96afe6d2fff811eab4edfa87b3f58606bdaf645c`；随后同步 DVFS 文档、路线图与状态记录。
**本轮 FDT OPP 解析器增强：** 旧 `live-fdt.json` 仅包含 base `opp-microvolt`，未解析 `opp-microvolt-*` 命名电压属性，也未输出 `opp_l_table` 自身的 `nvmem-cells` / `nvmem-cell-names`。`tools/inspect_h700_fdt.py` 现新增 `voltage_microvolt_variants`、OPP table NVMEM refs/name；合成回归覆盖 a1/b0 suffix 与 speed-bin phandle。提交：parser `fc84576d0d597abb6317ac64fefd62cf25dd63dc`、test `0ee82a5709418b8f81120cd337a75b8d237052ee`、workflow `9cbceb738ac2a8802784d0eac6ae6f4f2c5e1a5c`、fixture NUL 修正 `c5bd42ca547db2c49ec2018946275cd457ff24ee`。新增 Python/Actions 尚未取得可核验运行结果；须重新运行 inventory 生成 parser-v2 `live-fdt.json`。
**本轮同步频率采样增强：** 采集器现执行六轮、间隔 200 ms 的 uptime-bracketed 采样，分别读取 `policy0/cpu0` 的 `scaling_cur_freq`、`cpuinfo_cur_freq` 及同轮 CPUX `clk_summary` 行。静态安全测试增加该路径无 sysfs 写入的断言；workflow 已包含此 test path/command。采样脚本和回归源码已提交，完整 Python/Actions 结果尚未取得。
**同步采样代码提交：** collector `2726db1f2b4e356e632b958027cb11f9bf42bf49`（blob `94cce9b7a0a90a28a7e427cfb84d2a9f6be540b8`）；新增静态只读测试 `51ed3bae2fc24e88fc803b2e3e143fe5df8ba495`（blob `c320daec1fc6a970b19301aa61b0003b797409ae`）。采样 shell 片段与 Python 测试方法的语法检查通过；GitHub commit combined status 目前未返回 checks，完整测试/Actions 仍待确认。


**本轮 CPUX inventory v2：** 报告已上传至 [reports/rg34xx-inventory-20261125-cpux-v2](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux-v2)，采集时间 2026-11-25 00:41:21 UTC。parser-v2 已解析 `/opp_l_table` 的 table-level SID `speed` 引用及 `opp-microvolt-a1/b0/b1/c0/c1/c2` 命名电压；启动时 600/792/1032/1296/1344 MHz 五档因电压属性未命中被内核丢弃。v2 的 480 MHz cached observation 已由 v3 复读证据重新解释，不再视为真实 clamp；SID raw `0x6c00` 未获 OPP mask 授权，AVCC 热校准仍未通过。
**本轮 CPUX inventory v3（2026-11-25 03:19:48 UTC）：** 报告已上传至 [reports/rg34xx-inventory-20261125-cpux-v3](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux-v3)。原始 live FDT 与五个启动链二进制捕获的哈希与 v1/v2 一致；v3 新增 parser-v3 OPP property presence 和八组 cpufreq getter/clock/DCDC1 对账。五份 `scaling_available_frequencies` 均为 480/720/936/1008/1104/1200/1320/1416/1512 MHz；38 个 OPP 节点的唯一精确结构候选为 no table-level `supported_hw` + `b1` named voltage，全部 one-cell mask 模型没有 exact witness。九档 `b1` 电压为 900/900/900/950/1000/1050/1100/1150/1160 mV。八组 getter 证明 v2 的 480 MHz cached window 是 cpufreq sysfs 非原子/滞后读取，不是已证实的频率钳制；CPU thermal cdev 状态为 0/8、0/5，均绑定 CPU zone0。此结果仍不证明实际驱动源码、SID `0x6c00 → b1`、原始 PMIC/CCU 状态或 OPP 安全授权；AVCC 未校准、CPU1–CPU3 BL33 状态未知，所有 DVFS 写操作继续禁用。
**本轮 CPUFreq/kernel provenance collector（代码 HEAD `9cd001f2caf5d2f083e98abce7117494d09574c9`，安全断言修正 HEAD `e0ab50220939dfbbec95826ea61c15e7ec27c50c`）：** `scripts/collect-rg34xx-readonly-inventory.sh` 新增 `kernel-driver-provenance.txt`，只读采集 `/proc/version`、`/proc/sys/kernel/osrelease`、`/sys/kernel/notes` 的 hash 与前 256 bytes、可用 kernel config、cpufreq 相关 platform driver/device 的 `driver/device/of_node` 绑定、模块名和经筛选的 OPP/cpufreq/NVMEM kallsyms **符号名（丢弃地址）**。不执行 vendor getter、ioctl、SMC、modprobe/insmod，不读写 PMIC/PLL/SID 和 sysfs 控制项。静态测试先因注释中出现 `sunxi_get_soc_ver()` 文字而误报，已改为仅拒绝真实代码调用；同时修正写路径正则后，Linux/Windows 与 H700 3 ISA 矩阵全通过。
**本轮公开 OPP driver source 排除（2026-11-25）：** 新增对照 T507 4.9.170、Lineage Allwinner 4.9.170、Creality 4.9.191 与 Radxa `sun50iw9` 候选。前两者 raw `0x6c00` 分别映射 mask `0x4/0x1`，均无 `b1` named-property setter；Creality 需 `speed+Icpu` 且 `0x6c00` 仍走 mask；Radxa speed-only 候选落入 `a0` + mask。没有一个公开候选匹配 v3 的 no-mask + `b1` 指纹。本机 OPP table 只声明 `speed` cell；实际 kernel build ID/source commit 尚未取得，因此暂判断为未识别的 custom/vendor path，不推断 `0x6c00 → b1`。

**本轮 RG34XX CPUX inventory v4（2026-11-25 04:05:51 UTC）：** 报告已上传至 [reports/rg34xx-inventory-20261125-cpux-v4](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux-v4)。运行内核为 Linux 4.9.170 #19，Linaro GCC 5.3.1，构建时间 2026-05-26 13:58:08 CST；从 `/sys/kernel/notes` 解析出 GNU Build ID `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68`（notes SHA-256 `6065c1a7ae894047f0e902551811b39d0c5a11b5afd894083eae46497d56e12d`）。config、kallsyms 和 sysfs 链接确认内建 `sun50i-cpufreq-nvmem`、`cpufreq-dt` 平台设备均成功绑定；live OPP 表只有 `speed` NVMEM cell，raw SID 为 `0x6c00`。五组运行频率列表严格对应九个带 `b1` named voltage 的 OPP，频率/电压采样亦与若干 `b1` 档位吻合。公开 Creality 4.9.191 候选因要求额外 `Icpu` cell 且将 `0x6c00` 映射 mask 1 而被排除；T507/Lineage 4.9.170 候选亦不匹配。结论更新为：运行时有效 OPP grid 强烈指向 `b1`，但不能因此声称 SID `0x6c00 → b1`，所有 OPP/DVFS 授权仍关闭。已从实机 stock `mmcblk0p4` 只读导出 64 MiB Android boot 分区；解压内核中 GNU Build ID `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68` 与运行内核 `/sys/kernel/notes` 完全相同，首次建立**运行内核与 boot 分区内核二进制的精确身份关联**。完整文件不上传，仅保存[元数据报告](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-boot-p4-metadata.json)；分区 SHA-256 `41cfaa8f9ade4a5e05a89188abb7db154243e38b8e19c53b730f0a45ff325516`，压缩 kernel payload SHA-256 `cc4b5769cb8c9dcf8d013602e57c494a1153eb6714d9786cf847e39e3ecdd3af`。另扫描到 `fe7e68cd7aa14945d71f36e4fbf8412d959f4cc1` 第二个 GNU-note-like ID，尚未定位归属，不作解释。

**RG34XX PI12 电源灯实现（当前状态更正）：** PI12 driver、fake-IO service、Linkslot `Session` 状态转换和 MMU Device mapping 已完成；Playing/Polaroids 的关灯、退出后的点亮仍是未来 opt-in build 的目标行为。由于 BL33 handoff 后 APB1/PIO 状态未知、PIO Data Abort 不属于当前 SYS_CFG-only recovery 范围，现已通过 `H700_RG34XX_PI12_EARLY_ACCESS_ENABLE` 将物理 MMIO wrapper、`target_early_init()`、`target_init()` 和 C ABI 硬件调用默认关闭。production `platform/h700/rules.mk` 不定义此宏；默认 C ABI 返回 `ERR_NOT_READY`。只有一次性 host layout ELF 显式定义该宏来链接/测量真实 PI12 对象，不代表 production 已启用或设备已验证。

**本轮 LK MMU 顺序修正（`ca1d1ce33b588bff85dc4ab9c949971ee2736ff4`）：** 对照 pinned GammaOS LK `arch/arm/arch.c` / `platform/mt6580/platform.c` 发现 `platform_init_mmu_mappings()` 在 `target_early_init()` 前必须建立 section table/TTBR、映射 DRAM 和设备窗口、最后启用 MMU；此前仅添加 PIO descriptor 的实现不完整。现在按 RG34XX live DRAM `[0x40000000,0x80000000)` 建 1024 个 1 MiB normal write-back RW identity sections，再将 `[0x03000000,0x03100000)` 设为 Device RW/XN，最后 `arch_enable_mmu()`。`DRAM_PHY_ADDR=0x40000000` 只供 LK 初始 section map 使用，不是 `H700_MEMBASE/H700_MEMSIZE` 或任何分配授权。fake-MMU 回归通过。

**本轮 CPUFreq 诊断修正（`b421b077e5f27c95bf61679762b70a86d66131cb`）：** pinned Linux 4.9 候选 `cpufreq.c` 显示 `scaling_cur_freq` 对 `cpufreq-dt` target driver 返回缓存 `policy->cur`；读取 `cpuinfo_cur_freq` 会调用 driver getter，若读数不同，`__cpufreq_get()` 会通过 `cpufreq_out_of_sync()` 更新缓存并排队 policy update。旧 v2 先读 scaling 后读 cpuinfo，因此 480 vs 1008/1104MHz 不能证明持续硬件故障。v2 CPU zone0 46.637°C 且 first passive trip 60°C；固定 power_allocator 候选在低于开关温度时允许最大功率，但其他 cdev/策略及 exact shipping binary 仍未知。新只读 collector 增加 cpufreq path identity、getter 前后 scaling reread、thermal cooling_device cur/max state 与 zone binding、DCDC1/CPUX eight-sample capture。

**自动化验证：** MMU/PIO 修正后的 H700 AArch64、AArch32 ARM、AArch32 Thumb-2 compile + host regressions [#45526769732](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45526769732) 成功；最新 collector 诊断测试 H700 compile/regression [#45526953772](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45526953772) 和 Linux/Windows boot-audit tools [#45526953774](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45526953774) 成功。完整 Linkslot/mGBA/LK feasibility [#45526769421](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45526769421) 已成功完成；它覆盖 1.0.45 Rust Frontend、mGBA AArch32 static archive + newlib/VFS v3、真实 C ABI 链接探针、A32 BL33 入口/异常向量 prototype 与 memory fail-closed guard。Build report 明确说明这是 host/cross-build prototype，**不是完整 LK kernel target link、boot image 或实机启动**。

**本轮 PIO clock-boundary 审计：** H616-family 参考 PIO DTS 以 `CLK_APB1` 为主总线时钟且无独立 PIO reset property；参考 CCU `apb1_clk` 为 `SUNXI_CCU_MP_WITH_MUX`（offset `0x520`），无 PIO 专属 gate。RG34XX v2 Linux clk_summary 报告 APB1=100 MHz、PIO consumer=1。该证据支持不添加未经验证的 PIO gate/reset 写入，但 BL33-entry APB1 parent/rate 仍未实测；LED 代码不改 CCU。
**本轮 LK MMU static footprint：** pinned LK `arch/arm/mmu.c` 使用 `tt[4096]` 静态 section table，占 16 KiB、16 KiB 对齐的 BSS footprint。MMU hook 本身不动态申请 DRAM，但这块 BSS 和其余 LK BSS/stack/heap 的实际 placement 仍在内存 ownership 审批范围内。

**下一步 CPU/DVFS 工作：** p4 解压 SHA、公示 survey 前缀比对、live↔p4 GNU Build ID 精确匹配及 `vmlinux-to-elf` 恢复 AArch64 ELF 已完成，不重复采集。先完成匹配 binary 的全量 probe CFG、raw SID→mask/name 分支、两项 OPP setter 的返回处理和 cpufreq-dt 注册条件；随后恢复目标 OPP core 的 supported-hw / named-voltage filtering 逻辑，解释它与九档 runtime list 的冲突。保持完整 boot/ELF 本地，仅上传有限反汇编片段/JSON。再推进安全 BL33-entry 只读 PMIC/CPUX readback、CPU1–CPU3 handoff 与 timer continuity。AVCC 校准未解决前不开放 Auto/Boost；所有 DVFS 写操作保持禁用。

**本轮 OPP 候选分析工具：** 新增 `tools/analyze_h700_opp_candidates.py` 与 `tools/test_analyze_h700_opp_candidates.py`，按 Linux 4.9 OPP core 规则枚举无 `supported_hw` 与所有一单元 mask，准确处理 `opp-supported-hw` 属性缺失/空属性、命名电压属性 fallback，并与实际 `scaling_available_frequencies` 对比。工具要求 parser-v3 presence 字段；输出仅是 source/table fingerprint，不解码 SID、不授权 OPP，也不写硬件。

**本轮 OPP analyzer 自动化验证：** `da41806e47dbd2e0d57e23e9ff6ff405a5514e29` 通过 [H700 AArch32/AArch64 compile + host regression #45537478198](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45537478198) 和 [H700 boot audit tools #45537478202](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45537478202)。新测试覆盖完整 u32 位域、多版本 OPP cell、exact witness 约束和电压 min/target/max 顺序。前一中间提交 `300ae422` 的一项 test fixture 未禁止低位候选，已修正并在 `da41806` 两平台重跑通过。

**最近完成的 RG34XX 设备采集：** parser-v3 inventory 已上传至 `reports/rg34xx-inventory-20261125-cpux-v3/`，并完成 OPP exact-candidate analysis 与 cpufreq getter reconciliation。原始 binary 保留在采集机，未上传；报告可复查文本 JSON 与哈希清单。
**boot 二进制身份核对：** 已通过 [p4 元数据](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-boot-p4-metadata.json) 与 [解压 hash / aligned notes](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-boot-p4-verified.json) 完成 live kernel ↔ stock p4 kernel 精确 Build ID 匹配；解压 SHA-256 与公开 BaseOS survey 前缀核对成功。`vmlinux-to-elf` 和首轮 probe/OPP setter 反汇编也已完成，报告见 [cpufreq disassembly](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-cpufreq-driver-disasm.txt)。
**本轮 RG34XX cpufreq probe 完整 CFG + OPP core（2026-11-25，纯离线静态）：** 仅用本地已符号化 ELF，不接触设备、不读 `/sys`/`/dev/mem`、不调用 `sunxi_get_soc_ver`、不写寄存器、不重导出 p4。产物仅上传有界文本/JSON：[驱动关键函数反汇编](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-cpufreq-driver-disasm.txt) / [full CFG](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-cpufreq-full-cfg.txt) / [full CFG JSON](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-cpufreq-full-cfg.json) / [OPP core disasm](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-opp-core-disasm.txt)。ELF 与 boot 镜像保留本地。
  - `sun50i_cpufreq_nvmem_probe` = `0x…72ccc`–`0x…73148`（不含），287 指令、69 严格基本块、106 edges；`st_size=0`，边界取"闭合 epilogue 的 ret，其后继为不同 FUNC（`find_deepest_state`）且非本函数分支目标"，全程连续块覆盖无缝隙。
  - 十个显式 speed 比较值及分支表：`0x2000`/`0x2800`→mask 0 + 前缀 `a` + 后缀恒 1；`0x5c00`→mask 0 + `c` + 条件后缀 0/1/2；`0x6c00`/`0x7c00`→mask 0 + `b` + 条件后缀 0/1；`0x2400`/`0x5000`/`0x5400`/`0x7400`→mask 1；`0x2c00`→mask 2；其余→mask 8。
  - **b0/b1 确认保持 mask=0 且跳过 `set_supported_hw`**：`0x…72f24` 块不写 mask 槽，唯一 `str` 在 `0x…72f44`，b 路径经 snprintf 后跳 `0x…72f48` 绕过之；`0x…72fd8 cbz w1,[x29,#0x74]` 因此在 b 路径必然跳过 setter。据此 no-mask + `b1` 模型由运货二进制证实（此前仅为 DT/runtime 指纹）。
  - 零值保证：`0x…72fa8 cbz` 在名字空时跳过 `set_prop_name`；`0x…72fd8 cbz` 在 mask=0 时跳过 `set_supported_hw`。故 a/b/c 只设名字、mask 组只设 mask，两 setter 不会同时触发。`set_supported_hw` 失败打印 `"Failed to set supported hardware"` 并 unwind 至 `0x…73070`，不注册 cpufreq-dt；`platform_device_register_full("cpufreq-dt")` 在 `0x…7303c`，仅 CPU 循环结束后到达。
  - OPP core：`_of_add_opp_table_v2`（`0x…84bdac4`–`0x…84bdb78`）中 `supported_hw==NULL` 时凡带 `opp-supported-hw` 节点一律剔除、仅留无属性节点；注册 mask 时无属性节点仍接受且要求某 `(mask[i]&bit[i])!=0`；命名电压回退 `opp-microvolt-<name>`→`opp-microvolt`。parser-v4 的 live `live-fdt.json` 含全部 38 个 OPP entries、property-presence 与 named-voltage 值；b1 变体恰好对应九档 runtime rates，五个被拒频点没有 b1。因此 no-mask + b1 与运行列表完全对账；mask 1/2/8 会多纳入匹配节点而超过九档。
  - NVMEM：`nvmem_cell_read` 按 `cell->bytes`（speed 为 2）分配（含 `__GFP_ZERO`）并读 2 字节；probe `ldr w1,[x20]` 读 4 字节 = 超越声明长度的**逻辑越界读**（忽略 `&len`），多出 2 字节仍在同一 slab 对象内，不必然是故障/缺页；风险是高 2 字节非零会落空比较而走默认 mask=8。不要把该读法抄进新 HAL。
  - 源码家族：与 AvaotaSBC/linux `sun50iw9_nvmem_xlate` 同血缘（SID `0x20`→`<<5`、`0x28`→`&0x3ff`、两 setter），但常量集、`a/b/c` 条件后缀命名、`{0,1,2,8}` 掩码、DEFER 分类与逐 CPU setter 循环均不同，**不是同一实现**；offset 相同不构成源码匹配。
  - 仍开放：speed cell 2-byte allocation 被 32-bit `ldr` 读取时的实际比较高半字、live `sunxi_get_soc_ver()&7`（b1 要求非零）、精确公开源码 commit 与所有硬件安全/授权门槛。CPU 服务保持 `POLICY_ONLY`、`hardware_applied=0`，所有 DVFS 写禁用。

**stock kernel 二进制身份与初步 ELF 摘录（历史结果，已由上面的完整 CFG / OPP 报告取代）：** p4/live GNU Build ID 精确匹配；解压 SHA-256 `39cf8c3eb9a75ff0eda51b4074cd65860c5aa5386093bcaf69339819421d84b7` 与公开 BaseOS survey 的 16-hex 前缀一致；`vmlinux-to-elf` 恢复 89,675 个 symbols。后续不再导出 p4 或重建 ELF，只基于已经上传的有界 CFG / OPP 报告工作。

**本轮显示刷新策略与 Slot 前端审查（2026-11-25）：** 纯 C policy/profile 位于 `platform/h700/display/`：UI 默认请求 120 Hz、UI preference 可选 60/120；GAME 默认请求 60 Hz，独立 `game_120hz_enabled` opt-in 默认 0。该策略仅表达 request，不连接 Frontend、不等待 VBlank、不写硬件；前端设置行和持久化尚未接入。Slot 不主动设置 120 Hz 或切换 UI/GAME 物理模式。最新 live FDT 与同启动周期 TCON/PLL readback 已将当前 60-Hz 基线闭合到 divider 11 + PLL raw `0x88001703`；public sun50iw9 reference formula gives 288-MHz parent and estimated 26.1818-MHz DCLK / 59.569 Hz。pvaibhav 120-Hz candidate gives 528-MHz parent and 52.8-MHz DCLK / 120.13 Hz at live totals, and the offline kernel #19 audit has since closed the field layout, divider source and runtime parent inheritance (see item 4 and `ADDENDUM.md`), but panel support for that mode remains unverified。pvaibhav strict allowlist requires `fog_fj035fhd05_v1`, not our `rg34xx_v1 + lcd_type=old`；our live panel entry is `fog_fj035fhd05_v1_panel` registered under name `rg34xx_v1`, so the label mismatch is a naming rule, not a different init routine。UI present and armGBA committed-frame metrics are not yet measured；the next method is Slot's 600-present latency trace and an armGBA-specific reader only after binary/source/symbol layout matching。Hardware writer stays disabled and `hardware_applied=0`。
 
**本轮 ION/TEE 内存取证（2026-11-25 16:30 +08:00）：** 新报告的 `tee-ion-runtime.txt` 记录 `CONFIG_CMA/DMA_CMA/OF_RESERVED_MEM/ION/TEE/OPTEE=y`、`/dev/ion` 存在；`/dev/tee0` 与 `/dev/teepriv0` 不存在，标准 OP-TEE probe 报 `api uid mismatch`。ION debugfs 的 `secure` heap 当前零分配，`sys_user` 当前分配 7,397,376 bytes（含 `ion_disp2` 7,372,800 bytes）；CMA 为 64 MiB、约 62 MiB free；FDT secure heap base/size 为零。以上是运行时快照，不能证明 vendor DRM SMC 不存在，也不能给 `[0x7ec00000,0x7fc00000)` 16 MiB 保留区定名。该区域继续按未知保留区排除，`H700_MEMORY_LAYOUT_APPROVED` 保持 unset；未执行 SMC/ioctl/安全区读取。

## 1. 项目目标与固定约束

- 目标设备是 **Anbernic RG34XX（非 SP）/ Allwinner H700 / Cortex-A53**，不是 RG353PS/RK3566，也不是 RG34XX SP。
- 主线：**GammaOS LK → H700 AArch32 BL33 → H700 C HAL → Linkslot main 1.0.49**。当前已验证的 H700 前端 overlay/CI 仍基于历史 1.0.45，必须完成新基线重跑后才可把代码和验收状态标记为 1.0.49。
- 长期语言分工：**C/ARM 汇编负责 LK、启动链和 H700 BSP/HAL；Rust `no_std` 负责 Linkslot 前端业务；C/C++ 静态模拟器核心通过窄 ABI 接入。**
- 首期可玩目标只做 GB/GBC/GBA；首期只运行一个活动核心，采用单核协作式事件循环。
- 不把 Linux Linkslot ELF、Linux `std`/线程/动态 `.so` loader、EGL/ALSA/evdev/sysfs 直接移进 LK。
- 不推测物理内存地址，不写 Boot/SD 启动区，不执行未经验证的 payload；硬件寄存器写入必须有精确板级证据和独立测试。
- `upstream/linkslot` 是不可变源快照；移植补丁在隔离工作树生成，不直接修改上游归档。

## 2. 分支与源码基线

| 分支 | 用途 | 规则 |
|---|---|---|
| `main` | 项目默认/整合分支 | 不把未验证的移植实验直接带入 |
| `porting/linkslot-1.0.36-feasibility` | 旧版本历史验证 | 保留作为历史记录，不作为当前开发基线 |
| `porting/linkslot-1.0.45-feasibility` | **当前唯一移植工作分支** | 新代码、测试、路线图和状态记录都提交到这里 |
| `upstream/linkslot` | 上游源码归档 | 保持只读，不向此分支提交移植改动 |

当前唯一稳定移植基线为 **GitCode main 1.0.49**；最新只读归档 HEAD 为 `6122c8f2357ecae43295fdb1a158ff5ef7488325`。`main` 的 851 个源码文件较上一快照逐 blob 不变；`test` 已推进到 1.0.60（85 个文件变化、3 个新增），`DEV` 为 1.1.0（57 个文件变化、3 个新增）。main/test 当前不再相同；test 的变化涉及 `slot-ui`（51 文件）和 `slot` App/Frontend/Web（18 文件），不应把它概括成单纯视觉清理。首期仍固定 main 1.0.49。三个 H700 patcher 和两条 Linkslot workflow 已统一读取 `integration/linkslot/upstream-snapshot.sha`。最新 runtime-slices #134 与 full feasibility #915 均通过；#915 已验证完整 overlay 导出与 clean-baseline patch apply。源码原始 commit、归档 commit 与差异见 [upstream-baselines.md](upstream-baselines.md)。

## 3. 当前总体进展

### 已完成并有 CI/主机证据

- 历史 1.0.45 上游审查、三核心 GBA picker 状态机升级、Renderer overlay 基线更新；1.0.49 的新源代码审查见 [更新报告](linkslot-1.0.49-1.1.0-refresh.md)，适配重跑待完成。
- 多个纯业务 Rust crate 已拆为 `no_std + alloc` 形式；输入手势、核心选择、状态转换等有独立回归测试。
- Linkslot Renderer 抽象和 H700 软件 framebuffer compositor 已可在隔离上游工作树编译；尚未接入已授权的真实 scanout buffer。
- Linkslot `slot-retro` 静态核心接口、Libretro VFS v3 ABI、mGBA AArch32 静态 archive/newlib 构建和 host link probes 已完成；另已把真实 Linkslot Rust `Api` 表与真实 mGBA archive/newlib adapter 链接为无未解析符号的 AArch32 ELF（仅链接探针，不执行核心）。
- H700 SD→CMD17→GPT→FAT→VFS 的合成设备/主机端到端只读路径通过；尚未在 RG34XX 上完成真实 sdc0 MMIO 和实体卡读扇区。
- GammaOS LK H700 BL33 overlay 的部分 CRT0/异常向量、Cortex-A53 编译参数和 VBAR 处理通过 host/cross-build 检查；**不是完整 LK 镜像，也不是实机启动证明**。
- CPU OPP 纯 C 选择器已实现，覆盖 Power Saver、Balanced、Performance、Auto、受门控的 Boost、Speed Bin mask、电压/温度上限和 fail-closed 证据门。
- CPU Performance Service C ABI v1 已实现，细节见第 4 节。

### 仍未完成的主要系统门槛

- 没有完整的 GammaOS LK H700 target link/BL33 镜像，也没有实机 LK 启动。
- BL31→BL33 时 CPU1–CPU3 是否停驻尚未证明；在证明之前不能更改共享 CPUX PLL。
- DRAM/reserved-memory/heap/stack/framebuffer 所有权仍未全部闭合；未知区域继续排除。
- 实际 H700 显示扫描输出、GPIO/ADC 输入 producer、硬件 timer、PCM DMA audio、RTC、关机和真实存储写入服务尚未形成完整运行时。
- 无经 RG34XX 实机验证的 CPU OPP 表、PMIC 调压流程、CPU PLL 切换流程或温控闭环。

### 历史 Linkslot 1.0.45 综合验收（2026-11-24 15:48 UTC；不代表 1.0.49 已通过）

- [H700 Linkslot runtime-slices #45428654903](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428654903)：**成功，20/20 steps**。no_std business + Linkslot integration job 13/13，C input service ABI + queue job 7/7。
- [Linkslot LK Rust feasibility #45428664653](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428664653)：**成功，71/71 steps**，代码 HEAD `0e33da4a50b28583aac8c7e815dac7857b6ef43a`。artifact [14937338188](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428664653/artifacts/14937338188) 已上传，约 387 MB，包含 maps、ELFs、日志和报告。
- 覆盖 GammaOS LK H700 BL33 overlay/CRT0/exception vector、native BL33 header contract、memory/project fail-closed guard、H700 software renderer/SoftwareCompositor、完整 Linkslot 1.0.45 frontend + no_std business crates、Input ABI/queue、slot-retro VFS v3/static API、mGBA static Rust binding/前端选择、SD→GPT→FAT16/32/exFAT→VFS host path，以及 mGBA AArch32/newlib 构建和链接。
- **CPU QuickRow App tests 现已真实执行：** `cargo test --manifest-path .ci/upstream-linkslot/Cargo.toml -p slot --lib --no-default-features --features device,h700-software h700_cpu_performance_quick_row_tests -- --nocapture`；两项测试 `cpu_performance_row_is_visible_but_explicitly_unavailable` 和 `unavailable_cpu_performance_request_does_not_write_to_storage` 均通过（2 passed / 0 failed，486 filtered out）。runtime-slices 与 full feasibility 两条 workflow 都包含此步骤。
- 第一次新增测试的 run [#45428037808](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428037808) 曾失败，因为默认 `cargo test -p slot` 额外编译了上游 `tests/core_picker.rs`，该旧 integration test 与 1.0.45 `CorePicker<C>` 泛型 API 不匹配（E0107/E0308），不是本次 CPU App 测试失败。命令改为 `--lib` 后，[runtime-slices #45428654903](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428654903) 和 [full feasibility #45428664653](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428664653) 均成功。
- 真实 Rust `slot-retro::Api` `no_std` AArch32 staticlib、21 个 mGBA namespaced exports、真实 mGBA VFS v3 archive、H700 newlib adapter 和 `newlib/libm/libgcc` 最终 ELF32/ARM `--no-undefined` 链接通过；BL33 header/vector layout probe 通过。mGBA ELF 仍未执行，BL33 image 仍只是 tiny `kmain`/exception stub 布局探针。
- 这些结果仍是 host/cross-build 验收，不是完整 LK target link、可刷写固件或 RG34XX 实机启动。
### 现阶段仍然未完成

- 没有完整 GammaOS LK H700 target link、可刷写 BL33 image 或实机启动。
- newlib adapter 虽已参与 ABI 级最终链接，但其 LK heap/console/RTC 回调仍未在目标启动路径安装；文件 I/O VFS 与核心启动生命周期尚未接入完整 LK App。
- 没有执行 Rust+mGBA ELF、没有 ROM gameplay、无视频/音频/输入/存档实机验证；当前工作只证明 AArch32 静态符号闭合。
- CPU runtime/真实 DVFS 仍受第 4–6 节的硬件证据门控约束，不因 mGBA/Linkslot CI 成功而改变。
## 4. 当前 CPU 调频切片（本轮重点）

### 已有代码

- `platform/h700/power/h700_cpu_performance_policy.h/.c`：纯策略选择器，不访问 MMIO/PLL/PMIC。
- `platform/h700/power/h700_cpu_performance_service.h/.c`：版本化 C Service ABI v1。
- `docs/h700-cpux-clock-source-audit.md`：固定 `sun50iw9` vendor 时钟驱动审查，记录 PLL/mux 偏移、parent-switch 与 lock 参考序列、bit31/bit27 安全约束和 factor table 不可信限制；只作源码证据，不提供 MMIO 实现。
- `scripts/collect-rg34xx-readonly-inventory.sh` 现额外输出 CPUX clock-summary 摘录；静态安全测试接入 H700 workflow，结果待确认。
- `platform/h700/rules.mk`：已把 policy selector、C service ABI 和 static runtime owner 三个对象加入 LK `OBJS`；`scripts/test-h700-lk-project-guard.py` 检查对象项和对应 C 源文件。该项只建立 build graph，不代表完整 LK target 已链接。
- `target/rg34xx_h700/init.c` / `target/rg34xx_h700/rules.mk`：提供 LK `kernel/main.c` 必须调用的 `target_early_init()` 与 `target_init()`，两者当前为空实现；`scripts/build-h700-objects.sh` 已将其加入 AArch64/AArch32 ARM/Thumb 编译矩阵。H700 overlay 排除 MTK generic `target/init.c`，project guard 验证 MTK boot app 不进入 H700 project。
- `platform/h700/power/tests/h700_cpu_performance_service_test.c`：服务 ABI、策略门控、遥测分离、温度 clamp、Boost gating 和错误 OPP 测试。
- `scripts/test-h700-cpu-performance-service.sh`：主机测试入口。
- `scripts/build-h700-objects.sh`：已将 Service C 源纳入 AArch64、AArch32 ARM-state、AArch32 Thumb-2 编译矩阵。
- `.github/workflows/h700-arch-check.yml`：Push / Pull Request 均纳入服务 host regression 和架构编译。
- `integration/linkslot/h700-frontend-core/slot-cpu-performance`：新建 Rust `no_std` typed client；C service 作为不透明指针传入，模式通过固定枚举映射，C ABI v1 的 64-byte state 结构按 `repr(C)` 解码。
- Rust 客户端新增 `CpuPerformanceRuntime` token：通过 C runtime status/getter ABI 获取静态 owner；在 runtime 尚未 ready 时返回独立的 `RuntimeUnavailable(status)`，并通过 `&mut self` 发放临时 client，减少 Rust 侧重复可变访问。获取 token 是 unsafe 操作，要求 C 启动已成功、生命周期内不得重新初始化 owner，且不得存在其他 C/Rust 写入者。
- `platform/h700/power/h700_cpu_performance_runtime.h/.c`：新增静态 H700 runtime owner，提供单实例生命周期、RG34XX board profile ABI、四种用户策略预检和 fail-closed accessor；无有效 profile 时返回 unavailable/NULL。
- `platform/h700/power/tests/h700_cpu_performance_runtime_test.c`：覆盖默认 unavailable、NULL/未批准/错误 board/缺少 gate/缺少 OPP 拒绝、静态 owner 稳定性、policy-only 状态和服务撤销。
- `tools/inspect_h700_fdt.py`：扩展静态 FDT 解析，输出 CPU `cpu-supply` / `operating-points-v2`、regulator phandle、NVMEM cell offset/size 和 thermal-sensor provider arguments；明确标注这些只是 DT 声明，不是动态读回或安全 OPP 认证。
- `tools/inspect_h700_fdt.py` 另已扩展 OPP parser：每个 OPP 输出 `voltage_microvolt_variants`（`opp-microvolt-<suffix>`）并输出表级 `nvmem_cells` / `nvmem_cell_names`；对应 synthetic test `tools/test_inspect_h700_fdt.py` 与 H700 workflow 已更新。待 Actions 验证和设备重新采集。
- `tools/inspect_h700_sid_cells.py` / `tools/test_inspect_h700_sid_cells.py`：从 `cpu-power-runtime.txt` 验证且解析固定长度 speed 2-byte cell 与 thermal calibration 8-byte cell；speed 仅解码 little-endian raw `u16` 与 vendor literal 匹配标记，输出 `opp_authorization=not_approved`；thermal 拆成四个 `u16` 和诊断字段，输出 `calibration_status=not_certified`。未知值/缺失值不放行 OPP。Windows runner 自动生成 `sid-cell-report.json`。
- SID parser 的 `mainline_reference_decode` 使用 pinned mainline H616/H700 decoder；`0x6c00→candidate bin 5` 仅是主线诊断参考，不是 vendor mask 或 OPP authorization。另已发现两个 4.9.170 vendor candidates 对 `0x6c00` 分别映射 0x4/0x1，进一步证明版本号不足以唯一识别 SID decoder。所有报告仍固定 `opp_authorization=not_approved`。
- `sid-cell-report.json` 同时保留 generic SoC bus 的 machine/family/soc_id/revision 原始值，缺失时 unavailable；这些字符串不作为 vendor SID revision 语义的替代，也不改变 `opp_authorization=not_approved`。
- `scripts/collect-rg34xx-readonly-inventory.sh`：新增 `cpu-power-runtime.txt`，只读采集 Linux CPUFreq、thermal zone/trips、regulator class/debugfs、`clk_summary` 和 NVMEM 元数据；仅在 `of_node` 精确匹配 `sunxi-sid-ng@03006000` 时，读取 DTS 声明的 speed cell（provider offset 0、2 bytes）与 thermal calibration cell（offset `0x14`、8 bytes），不 dump 完整 NVMEM 区域，也不修改 governor、频率、电压或时钟。旧的 2026-11-22 报告未包含该新输出，必须在设备上重新采集。
- 另探测通用 SoC bus 的 `/sys/devices/soc0` 与 `/sys/bus/soc/devices/soc0`，仅读 machine/family/soc_id/revision，不读 serial/chipid。固定 vendor sun50iw9 `sunxi_get_soc_ver()` 可能通过 CE 临时开关 clock/reset，因此明确不调用 `sunxi_info/sys_info` 或 `/dev/sunxi_soc_info` ioctl。
- 该 collector 另探测通用 SoC bus 的 `/sys/devices/soc0` 与 `/sys/bus/soc/devices/soc0`，仅收集 machine/family/soc_id/revision，不收集 serial/chipid。固定 vendor `sunxi_get_soc_ver()` 在 sun50iw9 可能走 CE clock/reset 暂时开关路径，因此 `sunxi_info/sys_info` 与 `/dev/sunxi_soc_info` ioctl 均明确不调用。
- 另只读探测通用 SoC bus 的 `/sys/devices/soc0` 与 `/sys/bus/soc/devices/soc0`，仅收集 machine/family/soc_id/revision，不读取 serial/chipid。固定 vendor `sunxi-sid.c` 的 sun50iw9 分支未定义 `SUNXI_SOC_ID_IN_SID`，`sunxi_get_soc_ver()` 可能调用 `sid_rd_soc_ver_from_ce()` 临时写入/恢复 CCU CE clock/reset；`sunxi-sysinfo` 的 `sys_info` 也会输出 serial/chipid，因此 collector 明确不调用 sys_info 或 `/dev/sunxi_soc_info` ioctl。
- 针对旧 Linux 4.9 sysfs 兼容性，CPUFreq、thermal、regulator、NVMEM 各分组现会明确写 `NOT_PRESENT`（节点类不存在）或 `NO_READABLE_*`（节点存在但没有可读属性）；不会把空列表解释成零频率/零电压。 `tools/test_inventory_script_safety.py` 同步覆盖这些分支。
- 修复 C service 的失败重初始化语义：对完整 service storage，在校验新配置前先撤销旧实例；ready 检查同时验证 ABI struct size，避免无效重初始化仍保留旧服务。
- 已核对 Linkslot 1.0.45 `slot-ui/src/quick_menu.rs` / `slot/src/app.rs` 的 CPU 模式 UI 扩展点；当前 System Settings 为 6 行，`QuickRow`/`QuickValue` 使用固定数组和 enum index。UI 扩展要求在末尾追加新 row/value、同步中英文 key 和 parity tests，不能重排既有设置索引。设计约束见 CPU DVFS 文档 7.2。

### ABI v1 语义

- C 服务持有调用方提供的固定大小 OPP 表；不使用 heap、线程或动态分配。
- 模式请求经纯 C selector 选择目标 OPP。
- 成功返回 `H700_CPU_PERF_SERVICE_POLICY_ONLY`：**仅表示策略目标已选出，不表示真实 CPU 频率/电压已改变。**
- 状态结构把 `planned_frequency_hz/planned_voltage_uv` 与 `actual_frequency_hz/actual_voltage_uv/temperature_mc` 分开。
- `hardware_applied` 在 ABI v1 中固定为 0。没有实际硬件读回时，不得把计划频率显示为实测 MHz。
- 更新温度/电压/频率限制后，当前有效请求会重新经过 policy selector。
- Rust 客户端把 C service pointer 作为 opaque pointer；把 planned OPP、actual clock/voltage/temperature 遥测、policy flags 和错误码转换为 Rust 类型。
- Rust 客户端校验 ABI version、struct size、reserved bits、telemetry valid flags 和 `hardware_applied == 0`；C 侧 `_Static_assert` 固定 state size=64、`hardware_applied` offset=48、reserved offset=52。
- 当前测试使用合成 OPP 值，不是 RG34XX 已批准的运行频点表。静态 runtime owner 已实现，H700 `target_init()` hook 也已存在，但 hook 保持空实现、没有 board-approved RG34XX profile，也未调用 owner；因此目标机 service 仍 unavailable。Rust 客户端尚未接入 Linkslot `App` 设置界面。
- 不同 vendor `sun50i-cpufreq-nvmem.c` 对 SID raw `0x6c00` 的 literal mapping 不一致；两份 4.9.170 候选均有对 2-byte SID cell 的 `u32 *` 长度风险，4.9.191 另有 length-checked byte-wise 实现。不得把任何一个公开分支当成目标运行内核的精确映射。

### 本轮本地验证与 CI 状态（2026-11-24 13:38 UTC）

- [Linkslot LK Rust feasibility #45403377689](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45403377689)：**成功，69/69 steps**；H700 BL33 overlay、memory fail-closed guard、renderer/framebuffer、Linkslot、mGBA、newlib 和只读 MMC/GPT/FAT/exFAT 综合 host/cross-build 通过。
- [Linkslot LK Rust feasibility #45405198205](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45405198205)：**cancelled**，被后续提交触发的并发运行取代；取消时 69 steps 中 34 步完成且没有测试失败，不作为成功或失败证据。
- [Linkslot LK Rust feasibility #45405599463](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45405599463)：截至 13:41 UTC **in progress**，head `b7ac4a5ddae666b475654f36e13c7071c1723d33`；cross-compile job 68 steps 中 11 步完成、未见失败；GammaOS LK H700 BL33 overlay、native BL33 header、H700 project/memory fail-closed guard 已通过，后续 renderer/Linkslot/storage/mGBA 步骤仍在执行。
- [H700 boot audit tools #45405024561](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45405024561)：**成功**；SID parser mainline reference classifier / Windows runner / 7 项 synthetic tests，Linux 6/6、Windows 8/8。
- [H700 boot audit tools #45405197973](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45405197973)：**成功**；roadmap/inventory docs batch，Linux 6/6、Windows 8/8。
- [H700 boot audit tools #45405590516](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45405590516)：**成功**；inventory schema docs batch，Linux 6/6、Windows 8/8。
- [H700 boot audit tools #45405607986](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45405607986)：**成功**；latest PROJECT-STATUS handoff check，Linux 6/6、Windows 8/8。
- [H700 boot audit tools #45405197973](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45405197973)：截至 13:38 UTC 两个平台 audit jobs 仍在运行；Linux host-tools 3/5、Windows 3/7 steps 已完成，未见失败。此 run 触发于 roadmap/inventory 文档更新。
- [H700 boot audit tools #45404785874](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45404785874)：**成功**；mainline speed-bin direct/ambiguous/unsupported reference 分类的 7 项 parser tests 和两平台 wrapper tests 全通过。
- [H700 boot audit tools #45404714998](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45404714998)：**成功**；SID mainline reference decoder implementation 的 initial host/Windows audit 通过。
- [Linkslot LK Rust feasibility #45393436818](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45393436818)：**成功，69/69 steps**；上一轮 H700 LK/Linkslot/mGBA/storage 综合基线。
- [Linkslot LK Rust feasibility #45399902083](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45399902083)：**成功，68/68 steps**；SID collector/parser 集成阶段综合回归。
- [H700 AArch32/AArch64 compile check #45392910394](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45392910394)：**成功**；RG34XX H700 target lifecycle hooks 在 AArch64、AArch32 ARM-state、AArch32 Thumb-2 编译通过。
- 最新 raw decoder 仅作为 Linux mainline reference 诊断。它不修复或信任旧 vendor `u32*` / 2-byte NVMEM cell consumer，不给设备 OPP mask，不生成真实频率/电压表，也不打开 DVFS。
- 当前仍没有 RG34XX 新版实机 `live-fdt.json` / `cpu-power-runtime.txt` / `sid-cell-report.json`；旧 2026-11-22 inventory 未包含新证据项。没有实机 CPU rail readback、完整板级 OPP/voltage policy、可信 SMCCC revision 或 thermal calibration pass，CPU runtime owner 仍 unavailable。
### 本轮 CI 验证

- [H700 boot audit tools #45397765612](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45397765612)：**成功**；collector missing-node `NOT_PRESENT` 状态覆盖测试通过，Linux/Windows jobs 均成功。
- [H700 boot audit tools #45397788581](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45397788581)：**成功**；明确 missing/unreadable CPU power sysfs status 的安全回归通过。
- [H700 boot audit tools #45397828044](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45397828044)：**成功**；inventory documentation and read-only semantics checks passed.

- [H700 boot audit tools #45396816432](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45396816432)：**成功**；Ubuntu host-tools 与 Windows PowerShell jobs 均通过，包含新增 CPU FDT phandle 测试、read-only CPU power inventory 测试。
- [H700 boot audit tools #45396493076](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45396493076)：**成功**；验证 CPU power collector / FDT provider metadata 的主机测试与 Windows wrapper 检查。
- [H700 AArch32/AArch64 compile check #45392910394](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45392910394)：**成功**；包括 H700 target lifecycle hooks 三种 ISA 编译矩阵。
- [Linkslot LK Rust feasibility #45393436818](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45393436818)：**成功，69/69 steps**；本轮 target hook 前一版综合 BL33 overlay/Linkslot/mGBA/storage host-cross-build baseline。

- [H700 AArch32/AArch64 compile check #45386479380](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45386479380)：**成功**。两个 jobs 均成功：
  - `storage-host-regression`：包含 CPU Service v1 host runner 和现有只读存储回归。
  - `compile-freestanding-objects`：包含新 CPU Service C 源的 AArch64、AArch32 ARM-state 和 AArch32 Thumb-2 编译。
- [H700 Linkslot 1.0.45 runtime slices #45387192778](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45387192778)：**成功**。两个 jobs 均成功；新增 Rust CPU performance crate 的 workspace host tests、AArch32/AArch64 `no_std` check 和 Linkslot renderer/business overlay compile 均通过。
- [H700 AArch32/AArch64 compile check #45387293421](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45387293421)：**成功**。包含 C Service ABI v1 状态布局 `_Static_assert`、host regressions 和 AArch64/AArch32 ARM/Thumb-2 compile。
- [H700 boot audit tools #45387673284](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45387673284)：**成功**，覆盖当轮项目状态文档和报告工具检查。
- [Linkslot LK Rust feasibility #45387540498](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45387540498)：**cancelled**，由后续提交触发的并发运行取代；没有报告测试失败，不能当作通过或失败证据。
- [H700 boot audit tools #45388212345](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45388212345)：**成功**，覆盖最新路线图中的 CPU 设置 UI 扩展约束。
- [Linkslot LK Rust feasibility #45388212358](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45388212358)：**成功**，是本轮之前的一次完整 overlay/cross-build 综合基线。
- 初次 runtime-slices 运行 #45387187588 在仅提交 workspace manifest、Rust crate 源文件尚未提交的中间 commit 上失败（找不到 `slot-cpu-performance/src/lib.rs`）；完整 crate 提交后的 #45387192778 已通过，该中间态失败不代表最终 crate 测试失败。
- [此前完整 Linkslot 1.0.45 feasibility #45381590503](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45381590503)：成功；这是旧的 69-step 综合基线，证明 host/cross-build 范围，不证明实机运行。
- [CPU OPP policy 专项 #45381349400](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45381349400)：成功。

## 5. H700 CPU DVFS 硬件证据和安全门槛

已确认/交叉验证：

- RG34XX 现场 Linux 日志识别 PMIC 为 **AXP2202**，I²C-5、地址 `0x34`。
- 2026-11-25 live FDT 已直接确认 CPU0 `cpu-supply` phandle `0x4f` 指向 AXP2202 DCDC1（500000–1540000 µV，always_on/boot_on）；同次 Linux regulator debugfs 显示 `cpu0 @1160mV`。这是本次 live FDT + Linux regulator-framework 证据，不是 BL33-entry PMIC 原始寄存器读回。
- 通用 SoC `soc0` sysfs revision 即使存在，也仅作为 OS 导出的识别线索；vendor `sunxi_get_soc_ver()` 路径当前不调用，因为它在该 H700 配置可能临时切换 CE clock/reset。SID JSON 对 generic revision 值保持原样，不把它映射成 H700 speed-bin 档位。
- 2026-11-25 新版 inventory 已按 DTS 声明限长读取 SID speed cell（2 bytes，raw `0x6c00`）与 thermal calibration cell（8 bytes，raw `181113ab191b16bb`）。主线诊断 decoder 给 candidate bin 5，但公开 vendor source candidates 对 `0x6c00` 的 mask 解释不一致（4.9.170 分别为 0x4/0x1）；generic SoC identity unavailable，所有结果仍 `opp_authorization=not_approved`。
- 现场内核报告 H700 thermal sensor 存在且有 4 个 SID raw calibration fields，但明确提示 **AVCC is not calibrated**；CPU/GPU 当前温度只作诊断，不可用于自动温控升频、Auto 或 Boost。
- 固定 `sun50iw9` vendor-source audit 给出 `PLL_CPU=0x0000`、`CPU_CFG=0x0500` 和 alternate-parent/lock 流程；2026-11-25 Linux `clk_summary` 观察到 `pll_cpu → cpu → cpuapb`、`pll_periph0=600MHz`。同版本 cpufreq-dt 候选源表明 `cpuinfo_cur_freq` 是 clock-framework `clk_get_rate()` 读数，而 `scaling_cur_freq` 是缓存策略值；首轮数据未同步采样，不能认定 clock provider 有永久冲突。六轮 uptime-bracketed read-only sampler 已加入采集器，待重新采集。
- Linux runtime snapshot 显示 CPU0–CPU3 全部 online 且属于一个共享 cpufreq policy，确认 CPU 频率不是按每个模拟器核心独立设置；但 CPU1–CPU3 在更早 BL31→BL33 handoff 时的状态仍未知。

**禁止事项：** 不复制 MT6785 的 PLL 地址/PCW 算法，不把 GammaOS Pico 600–2000 MHz 档位直接套到 H700，不使用 AXP717 参数替代 AXP2202，不把 H616 OPP 表当作 RG34XX 已批准 OPP，不在缺少次级 CPU 状态证明时写 CPUX PLL，不在缺少电压/热证据时升频。

## 6. 下一步任务（按顺序执行）

1. **在真实启动路径调用 owner，但不提供虚构板级配置**：H700 `target_init()` hook 已实现，下一步是在硬件证据齐备后接入 board-profile 加载；当前必须保持 hook 空实现、runtime unavailable。SoC revision 只能来自已验证的只读来源；不要调用会临时改变 CE clock/reset 的 vendor `sunxi_get_soc_ver()` 路径。
2. CPU Performance QuickRow 已完成 display-only/unavailable 接入和 App 回归测试。后续只有在 board-approved RG34XX profile、C owner 初始化和真实只读 telemetry 路径齐备后，才把 `CpuPerformanceRuntime` typed client 接入 App；在此之前继续保持不可选、不可 persist、无计划 MHz 展示。

Linkslot 1.0.45 UI 当前已追加 display-only CPU row：`SYSTEM_SETTINGS` 为 7 行，`QuickRow::ALL` 为 88 项，Language index=86、CPU Performance index=87；`QuickValue::ALL` 为 38 项，新 unavailable value index=37。两项 App unit tests 已在 runtime-slices/full feasibility CI 实际通过；Rust typed client 仍未接入 App。
3. **已完成 RG34XX inventory v4 与 stock CPUFreq binary audit**：[inventory v4](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux-v4)、[驱动关键函数反汇编](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-cpufreq-driver-disasm.txt)、[完整 CFG](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-cpufreq-full-cfg.json) 已确认 live/stock kernel GNU Build ID 精确匹配、probe 287 instructions / 69 blocks / 106 edges、十个 speed 比较值；`0x6c00` 走 mask=0 的 b0/b1 命名属性路径并跳过 `set_supported_hw`。b1 与九档 runtime OPP grid 对账，但 live `sunxi_get_soc_ver()&7` 和 2-byte cell 的 32-bit over-read 高半字尚未取证；不得调用可能写 CE clock/reset 的 getter。无需重复提取 p4、恢复 ELF 或重跑 CPUFreq CFG。
4. **显示路径主干与主要面板生命周期已恢复（kernel #19，离线）**：[首轮报告](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/kernel19-display-reverse-audit) + [第二轮 ADDENDUM](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/kernel19-display-reverse-audit/ADDENDUM.md)，基于 Build-ID `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68` 匹配的 stock ELF。第二轮通过内存中的 `.rela.dyn` overlay 恢复 91,760 个未应用的相对重定位，证明 `disp_panel_para = disp_lcd_private + 0xf0`，`panel_info+0x80 = lcd_dclk_freq`；按 `lcd_if` 选择 divider 表，RG34XX 基线 `lcd_if=0`、`lcd_dclk_freq=26` 触发特例 divider 11。TCON `base+0x44` 低 7 位来自 `tcon0_set_dclk_div`，高 nibble `0xf` 由 `tcon0_open` 设置，故读回 `0xf000000b` 的两个来源都已定位。PLL 请求为 26×11=286 MHz；报告称 PLL 表步进 12 MHz、取到 288 MHz，与同启动 `clk_summary` 一致；第三轮已逐行导出全部 187 行并裁决该矛盾：187 行、288–2520 MHz、步进 12 MHz 成立，`0x6400` 是索引 178 的中间项（2424 MHz），末项为 `0x6800`（2520 MHz）。选频为“夹取 + 最近邻 + 等距取低”：286 MHz 请求落在 288 MHz（`0x1702`），572→576 MHz / divider 11 是外部强制保持 divider=11 的反事实，并非 stock `lcd_dclk_freq=52` 路径（stock divider=6、PLL 312 MHz、DCLK 52 MHz、约 118.31 Hz）。另一个 divider=10 / PLL=528 MHz 的外部候选算得约 120.13 Hz，但未验证面板。详见 [PLL-TABLE.md](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/kernel19-display-reverse-audit/PLL-TABLE.md)。

面板表已恢复为 17 项，RG34XX 命中 `fog_fj035fhd05_v1_panel`（注册名 `rg34xx_v1`）；`disp_lcd_enable` 主顺序、开/关 panel callbacks 及 TCON config 路径已追出。**仍未完全闭合**：`LCD_OPEN_FUNC` 步骤表的运行时 executor、四个 `struct clk *` handle 的取得点、bootarg `lcd_type=old` 与显示 profile/interface 的实际联系。时钟方面，驱动在 init 时用 `clk_get_parent` 保存现有 parent，再在 enable 时 `clk_set_parent` 回写同一 parent；它并非选择一个新的 mux parent，但会按 `lcd_dclk_freq × divider` 调用 `clk_set_rate`。实测当前 `tcon_lcd` parent/rate 为 `pll_video0x4 / 288 MHz`，这是 runtime `clk_summary` 证据。

120 Hz 仍为**未验证/不支持启用**：未找到绑定 `rg34xx_v1` 的第二套 timing 或 runtime refresh switch；面板规格容差、VBlank/TE 切换边界、accepted divider、回滚路径仍缺。**不连接设备、不写 TCON/PLL/CCU/PMIC、不接入硬件 writer**。第二轮自检 82/82、第三轮自检 47/47 均通过；第三轮的检查包含 187 行逐行重算、表末项与 `0x6400` 位置、以及两个请求的选行结果。

**Linkslot 1.0.49 综合 feasibility CI：** [#45631521531](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45631521531) 已在修复 mGBA static selector 的 `Option<LinkSetup>` 类型后 **71/71 步骤成功**；该成功 run 的 head 为 `d333eb30dc8a21b4e9b9509bc08a3760223c091a`。通过内容是 host/cross-build、软件 renderer、VFS/storage regressions、真实 mGBA AArch32 static archive 与 Linkslot API link probes、BL33 layout probe；**不是可刷写完整固件、实机启动或游戏运行验证**。
5. 证明 BL31→BL33 CPU1–CPU3 状态、单核 invariant、备用父时钟切换和 timer continuity；同时解决 AVCC thermal calibration 或确认独立可信温度来源。前述门槛闭合后才设计 C HAL transition（升频先升压、降频先降频后降压、readback/timeout/rollback/hysteresis）。
6. 最后才开放实际 Performance/Boost。不得把 ABI v1 的 `POLICY_ONLY` 当作 DVFS 已完成。
7. **显示 120/60 Hz：** live FDT timing/interface reparse、TCON/PLL 只读读回与 kernel #19 离线逆向均已完成（[审计报告](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/kernel19-display-reverse-audit)）。60-Hz 状态：TCON word divider 11（`0xf000000b`）、PLL raw `0x88001703` 经 `calc_rate_video0` 解为 288 MHz，288/11 → 26.1818 MHz → 820×536 下 59.569 Hz，与 IRQ proxy 约 59.6 Hz 吻合。已确认 divider 11 是常量表按 **`lcd_if`** 索引（`0 → 6`）叠加 **`lcd_dclk_freq == 26` 的特例覆盖**（→ 11）得到的编译期结果，`lcd_type` 与 SoC 版本都不参与该分支；`lcd_dclk_freq` 虽不参与底层分频写入，但参与后续 `clk_set_rate` 目标速率计算（286 → 288 MHz）。镜像内没有 120 Hz 模式表、没有运行时刷新切换路径，因此 120 Hz 仍为**未验证/不支持启用**。下一步：用 Slot present 计数和 armGBA committed-frame 计数建立独立 UI/GAME FPS；在精确 panel/revision、120-Hz clock/timing、VBlank/TE boundary 与 scanout/cache ownership 闭合前，不接入硬件 writer、不把 120-Hz candidate 标成 supported。

## 7. 续接规则

新的工作轮次按以下顺序恢复上下文：

1. 先读取本文和 GitHub 当前分支 HEAD / 最新 Actions 状态；本文的 HEAD 是核对时快照，不应假设之后没有新提交。
2. 当前代码只在 `porting/linkslot-1.0.45-feasibility` 上推进；先检查分支差异与最新 CI，不重复建立已有分支/文件。
3. 依任务读取专项文档：
   - CPU 调频：本文 + [H700 CPU Performance Service / DVFS](h700-cpu-performance-service.md) + [H700 CPUX clock-source audit](h700-cpux-clock-source-audit.md) + [RG34XX CPUX live audit](rg34xx-cpux-live-audit-2026-11-25.md)
- FDT OPP parser: [`tools/inspect_h700_fdt.py`](../tools/inspect_h700_fdt.py) + [`tools/test_inspect_h700_fdt.py`](../tools/test_inspect_h700_fdt.py); parser-v2 needs a new live inventory capture.
- OPP/cpufreq source check details: [RG34XX CPUX live audit §1.5](rg34xx-cpux-live-audit-2026-11-25.md).
   - 总体路线： [roadmap](roadmap.md)
   - 最新 Linkslot 源码差异： [1.0.49 / 1.1.0 refresh](linkslot-1.0.49-1.1.0-refresh.md)；历史 1.0.45 适配记录：[1.0.45 portability update](linkslot-1.0.45-portability-update.md)
   - LK/H700 runtime 架构： [Linkslot H700 runtime architecture](linkslot-h700-runtime-architecture.md)
   - BL33/内存交接： [RG34XX boot handoff audit](rg34xx-boot-handoff-audit.md)
4. 只把 CI 明确通过的提交标为已验证；host compile 不等于实机启动，合成 OPP 不等于真实可用频点。
5. 每轮完成后先更新本文，再更新 roadmap 和对应专项文档；把新 commit SHA、Actions run URL、失败及尚存风险写入本文。


**第三方 H700 120 Hz 实现审查（2026-11-25）：** 已审查 [`pvaibhav/slot` `5206e3fa39aa4a9b702cd9048a07dab3b5ab95da`](https://github.com/pvaibhav/slot/tree/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da)。该 fork 的 Linux 侧实现确实比我们当前 policy-only 完整：`PanelRefresh::probe()` 对 model/panel/DT timing 严格 allowlist，使用 `sunxi_dump` 写 `DIVIDER 0x06511044` / `PLL 0x03001040`，120 Hz 先 divider 后 PLL，60 Hz 先 PLL 后 divider，读回两寄存器并在失败时 best-effort 回滚 60 Hz；启动时应用，唤醒时在开背光前重应用。其测量工具读取 LCD IRQ，而非依赖 stale `ref_fps`。但 **不能直接移植到本机**：fork 对 target `rg34xx` 的 gate 要求 `fog_fj035fhd05_v1 + 720×480 + lcd_dclk_freq=26 + HT/VT=820×536`，而本机最新 live FDT 是 `rg34xx_v1`；fork 自己也注明非 SP RG34XX 没有连接实机做 live test。BaseOS fleet panel survey 同样将 RG34XX 标为 `rg34xx_v1`。此外该 fork 设定为全局默认 120 Hz，启动/唤醒恢复，不提供独立 UI 偏好与 GAME 120 Hz 开关；与我们的 per-scene policy 不同。详细分析见 [`slot-pvaibhav-120hz-review.md`](slot-pvaibhav-120hz-review.md)。结论：借鉴 strict gate、C HAL owner、write ordering、readback、rollback 和 IRQ 验证；不复制其地址/数值或修改 panel allowlist。live FDT timing/interface 与 TCON/PLL 只读读回均已完成；下一项改为匹配 kernel #19 的 `rg34xx_v1 + lcd_type=old` 初始化、PLL 位域与时钟父链，并完成 UI present / armGBA committed-frame 独立计数。硬件 writer 继续关闭，`hardware_applied=0`。


## 8. pvaibhav/slot RG34XX non-SP 实机运行采集（2026-11-25 18:44–18:45 +08:00）

报告已上传至 [reports/pvaibhav-slot-runtime](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/pvaibhav-slot-runtime)，同一台 RG34XX、同一系统分别采集 UI 29.624 s（27 samples）和 GAME 29.749 s（23 samples）。[专项分析](docs/pvaibhav-slot-rg34xx-live-capture-2026-11-25.md)。

- 两个显示 IRQ 均稳定约 59.6 Hz：UI IRQ292/293 = 59.648/59.614 Hz；GAME = 59.632/59.632 Hz。驱动 `fps` 只在 59.8/60.2 间变化，`ref_fps=59`。DE rate 696 MHz；两轮 framebuffer、display status 和 clock summary 相同。没有观测到 UI/GAME 刷新模式切换，当前实机显示扫描仍约 60 Hz。
- 新增 [FDT reparse 报告](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/pvaibhav-slot-runtime/fdt-reparse)：JSON 已脱敏并记录 hash，未公开 raw DTB。解析确认 `rg34xx_v1`、720×480、RGB666、DCLK property 26 MHz、HT/VT 820×536、H porch 46/2/52、V porch 20/5/31，静态派生 nominal 59.155 Hz；bootargs 为 `lcd_type=old`。同启动周期 TCON/PLL 只读回读随后得到 `0xf000000b` / `0x88001703`。若 `tcon_lcd=288 MHz` 直接馈入 divider 11，则 DCLK 推算 26.1818 MHz、refresh 59.569 Hz，与 IRQ 约 59.63 Hz 的差异缩至约 0.1%；这是时钟树假设下的推算，不是独立物理测量。Knulli RG34XX DTS 提供 26 MHz/820×536 配置参考；MustardOS 的 54 MHz/1679×536 是另一配置，不能混用。
- Slot PID 2842 两轮相同，cmdline/executable 映射路径是 `/mnt/sdcard/System/slot`（comm 被 loader 截断为 `ld-linux-aarch6`）。GAME 的 `maps` 确认加载 `armgba_libretro.so`。多前端路径不是进程名 `slot-pvaibhav`；后续 collector 必须按真实 `/System/slot` executable path 匹配。
- `slot.state` 的 `high_refresh=1`，但配置值本身不是硬件模式证明；IRQ/clock/readback 没有显示 120 Hz。UI present FPS 和 armGBA committed game FPS 均未测得，不能从 IRQ 推断游戏 FPS。
- pvaibhav fork 的目标 `rg34xx` gate 要求 `fog_fj035fhd05_v1`；本机实际是 `rg34xx_v1`，虽然 report README 所列的几何和时序数值匹配，但面板 byte identity 不匹配。其 `PanelRefresh::probe()` 会拒绝该设备、不会开放 writer；不可放宽 allowlist 或复制寄存器值。
- `sunxi_dump` write/dump 节点存在且 root 可写，但本次没有写入。保持硬件 writer 关闭、`hardware_applied=0`。

运行环境已确认 `BASEOS_TARGET=rg34xx` / `BASEOS_PLATFORM=h700`，Slot 进程 UID=0 且具备完整 effective capabilities；pvaibhav 的 `DevicePlatform` 以 `BASEOS_TARGET` 选择 panel gate，`supports_high_refresh()` 只在 `PanelRefresh::probe()` 成功时为 true。由于 `rg34xx_v1` 不等于要求的 `fog_fj035fhd05_v1`，reviewed source 会拒绝该面板，`high_refresh=1` 不会触发写入。设备端 `System/slot` 与 `armgba_libretro.so` 的 SHA 已记录，但尚未与 H700 release ZIP 解包文件逐字节比对。新增源码审查结果：Slot 在非零 `SLOT_TRACE_LATENCY` 或 `SLOT_ROOT/System/trace-latency.on` 启用时，每 600 次主 surface swap 输出 `presents/s`；armGBA 的 `gba->video.frameCounter` 在 VBlank 帧结束处递增。当前 probe 不支持 armGBA，需先匹配 installed core 的 release/build identity，再验证 ELF 中 `gba` 符号和 `struct GBA::video` 偏移，最后实现只读采样。UI present、armGBA committed frames 与 LCD IRQ 分开统计。GAME 120 Hz 维持默认关闭。

- 19:12 设备二进制 SHA-256 已记录：`System/slot`（18,718,728 B）`8f0fd40da4ac161e5fbb9a56a99c254acf8b2f31806e2be6bdaa929b5ee28505`；`SYSTEM/armgba_libretro.so`（2,021,992 B）`4ebae098cc4848fb459fc18299c39acbdbbbeb732db5c3f66d1ff3f6e142e889`。尚未与官方 H700 release ZIP 中提取的文件比对，因此安装二进制具体版本仍待确认。


**Panel profile nuance:** the pinned [BaseOS variant survey](https://github.com/pvaibhav/BaseOS/tree/63137afeda1c0bc4ff771be6f03012b7f8147aca/experiments/h700-boot-speed/variant-survey) maps target `rg34xx` to `rg34xx_v1` 720×480; `fog_fj035fhd05_v1` belongs to other targets/geometries. The live bootargs contain `lcd_type=old`, and that survey notes the Linux panel driver uses `lcd_type` to select its init sequence. Thus the reviewed pvaibhav fork's `rg34xx → fog_fj035fhd05_v1` allowlist is not a match for this standard non-SP profile. Do not substitute the string without proving the exact `rg34xx_v1 + lcd_type=old` timing and register sequence.


**RG34XX vendor panel-selection audit:** the pinned BaseOS compatibility contract maps target `rg34xx` to `rg34xx_v1` / 720×480, with vendor PE6 high → `lcd_type=old`, low → `boe`. Its U-Boot inventory selects the `rg34xx_v1` panel struct (`selected_by_dt=true`) but the struct calls a common init function whose symbol string contains `lcd_panel_uboot_fj035fhd05_v1_init`. That routine label is not the DT `lcd_driver_name`. The live `lcd_type=old` is consistent with the vendor PE6-high branch. This confirms the fork's `rg34xx → fog_fj035fhd05_v1` gate does not describe the standard target profile; do not change that allowlist or copy the Linux refresh registers without independent validation. Details and pinned source links are in [the live capture analysis](docs/pvaibhav-slot-rg34xx-live-capture-2026-11-25.md).

**pvaibhav 安装包版本核对入口：** GitHub tag `v1.16.0-pvaibhav.2` 经 annotated tag 解析后精确指向审查源码 `5206e3fa39aa4a9b702cd9048a07dab3b5ab95da`。官方 H700 ZIP `slot-v1.16.0-pvaibhav.2-h700.zip` 为 17,556,056 B，资产 SHA-256 `1f9bdaa1110eb4f6006cc9da6a8bb34bca694ea53555c3138dcf656f0dfd1fca`。下一步由本地 Agent 下载并核验 ZIP，再比较其中 `slot` / `armgba_libretro.so` 与设备哈希；目前没有声称两者相同。


**相关 H700 时钟分频风险参考（非本机证据）：** MustardOS 2026-11-02 的另一台 RG34XX-H 曾因相关 `sun50iw9` BSP 的 RGB/HV TCON 固定 /6 与 PLL 最低 288 MHz，出现 DTB 标称约 59.97 Hz、实测约 106.6 Hz；其 DTS 后来将该机 DCLK/HT 改为 54 MHz/1679，见 [讨论](https://community.muos.dev/t/rg34xx-h-incorrect-display-refresh-rate/1825) 和 [修正提交](https://github.com/MustardOS/internal/commit/76e454dfa9758a99f85115f80a29121dabb13a01)。另一台 RG34XX-H 无法复现，说明不同修订/内核组合有差别。该数据不能用于本机，只支持一个结论：必须从精确 kernel #19 追出最终 TCON DCLK divider，不能只信 DTB property 或复制 54MHz/1679。


**TCON divider readback complete:** the same-boot `sunxi_dump/dump` report records `0x06511044=0xf000000b` (divider 11) and `0x03001040=0x88001703` (PLL raw). The related pinned sun50iw9 BSP's generic HV divider 6 is not the active value on this device. Under the 288-MHz parent assumption, /11 predicts 59.569 Hz; this matches IRQ proxy within ~0.1%. Exact Linux 4.9.170 #19 PLL decode, parent chain and `rg34xx_v1 + lcd_type=old` initialization still require source provenance. No writes were performed.


**报告脱敏与历史状态（2026-11-25）：** 当前 `reports` HEAD `34ea683` 的 FDT JSON 对 bootargs 敏感字段使用 `[REDACTED]`，raw DTB 未公开。该提交说明此前包含未脱敏 blob 的三提交序列已被替换；核对当前 `reports` 分支最近 100 个祖先提交，旧文档列出的两次提交不在当前可达历史中。此项只证明当前分支历史状态，不代表旧克隆、fork、下载缓存或 GitHub 保留对象立即消失；继续避免在日志、聊天和后续报告中复制设备序列标识。


### 8.1 TCON / PLL 只读回读更新（2026-11-25 19:59 +08:00）

最新报告位于 [reports/pvaibhav-slot-runtime/tcon-readback](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/pvaibhav-slot-runtime/tcon-readback)，采集于 Linux 4.9.170 #19、与 FDT 重解析报告相同的启动周期：

- `0x06511044` 原始值 `0xf000000b`；按已审查 TCON 字段定义，低 7 bits 为 divider=11，高 nibble 为 enable。
- `0x03001040` 原始值 `0x88001703`；不解码 PLL N/M 位域。
- `clk_summary` 同时记录 `tcon_lcd=288 MHz` / `pll_video0x4=288 MHz`、`pll_video0=72 MHz`、`display_top=200 MHz`；显示驱动状态 `ref_fps=59`、`fps=60.2`。

pvaibhav/slot 固定源码 `crates/slot-power/src/refresh.rs` 的 60-Hz register pair 正好是 PLL `0x88001703` + DCLK divider `0xf000000b`，与本机只读值完全相同；其 120-Hz 写入候选则是 divider `0xf000000a` + PLL `0x88002b03`。这验证了该 fork 对当前 60-Hz 基线所用寄存器地址/值与本机状态相符，但**不授权移植 120-Hz writer**：该 fork 的 `rg34xx` allowlist 仍要求 `fog_fj035fhd05_v1`，本机真实 panel 是 `rg34xx_v1` 且 bootargs 为 `lcd_type=old`。

以 `tcon_lcd=288 MHz` 直接馈入 divider 为条件，DCLK 推算为 26.1818 MHz；除以 live FDT 的 820×536 totals 得 59.569 Hz，与 UI/GAME IRQ proxy 59.61–59.65 Hz 一致到约 0.1%。这解释了 live FDT 26 MHz 整数属性计算出的 59.155 Hz 与 IRQ proxy 的差距，但仍不是示波器/光学测量，也没有验证 120-Hz 模式。PLL raw 位域、精确 kernel #19 显示驱动源码对应关系以及时钟树完整父链仍待核验。

报告中的 FDT JSON 已脱敏；raw DTB 未发布。所有显示硬件 writer 继续关闭，`hardware_applied=0`。


### 8.2 UI present / armGBA frame-counter source audit (2026-11-25)

- Slot pinned source `pvaibhav/slot@5206e3fa39aa4a9b702cd9048a07dab3b5ab95da` implements a trace gate via nonzero `SLOT_TRACE_LATENCY` or `SLOT_ROOT/System/trace-latency.on` (default root `/mnt/sdcard`, resolved once at first use). With tracing enabled before process startup, `device_app.rs` logs `600 presents` plus `presents/s`, nominal frontend frame period and missed game periods. This is the direct UI/main-loop present metric; it is not LCD scanout frequency.
- armGBA pinned source `MagnominusGit/armGBA@fad6c4b0c0176a76e10667f4aa071a8feb4425c0` defines global `struct GBA* gba`; `struct Video` contains `uint32_t frameCounter`; `src/video/video.c` increments it at V-blank frame end and `retro_run()` waits for it. This is a source-confirmed committed emulation-frame counter.
- The current public `probe.rs` only supports gpSP and mGBA. Before reading armGBA through `/proc/PID/mem`, match the deployed core hash to an exact build/release, verify ELF symbol visibility and pointer mapping, and calculate `offsetof(struct GBA, video)+offsetof(struct Video, frameCounter)` from that exact source. No memory probe has been run yet.
- Required output columns remain separate: `slot_presents_per_s`, `armgba_committed_frames_per_s`, `lcd_irq_per_s`, plus CPU MHz and temperature. No one metric substitutes for another.


### 8.3 Public sun50iw9 PLL formula cross-check (reference only; 2026-11-25)

The related pinned `qiaoweibiao/T507_Kernel@98fcc7d3f112e51b0edfb71536da89cb2115106f` clock driver places the `pll_video0x4` N field at bits [15:8], D1 at bit 1, and calculates `24 MHz × (N+1)/(D1+1)`; its DTS assigns `tcon_lcd` parent `pll_video0x4`. Under that public BSP layout:

- Live raw `0x88001703`: N=23, D1=1 → `pll_video0x4=288 MHz`; `/11` → 26.1818-MHz DCLK → 59.569 Hz at 820×536.
- pvaibhav 120-Hz candidate `0x88002b03`: N=43, D1=1 → `pll_video0x4=528 MHz`; `/10` → 52.8-MHz DCLK → 120.13 Hz at 820×536.

This is a strong internal-consistency check for the reference pair, not a proven decode of shipping kernel #19. The exact binary's PLL layout and live TCON parent/mux still need source matching; panel tolerance and runtime transition safety are unproven. Keep hardware writes disabled and `hardware_applied=0`.


### 8.4 Exact kernel #19 reverse-engineering path (2026-11-25)

KNULLI's [RG34XX developer documentation](https://knulli.org/devices/anbernic/rg34xx/#extracting-the-kernel-and-bootloader) states that Anbernic has not published the RG34XX kernel/U-Boot source; KNULLI's release uses bootloader/kernel binaries extracted from stock firmware. Our `reports/rg34xx-inventory-20261125-cpux-v4` records Linux 4.9.170 #19's GNU Build ID, and the stock `mmcblk0p4` extracted kernel ELF was previously confirmed to have that exact same GNU Build ID. The complete ELF remains local; do not upload the 64-MiB partition or whole ELF. Continue offline using that already symbolized ELF: locate `sunxi_clk_factor_pll_video0x4`, PLL factor/CCU functions, `disp_al_lcd_get_clk_info`, `disp_al_lcd_cfg`, `tcon0_set_dclk_div`, TCON LCD init/driver callbacks and `lcd_type` branches; disassemble bounded function ranges and compare register offsets, masks, divider encoding, PLL N/D1 layout, clock-parent selection and call ordering against the live readback. Publish only bounded disassembly excerpts, symbol names and verification JSON. The public sun50iw9 reference is a hypothesis until this binary-level check closes.


## 9. 2026-11-26 pvaibhav/slot 120 Hz 复测与 FPS 采样下一步

### 新增 ADB 实测结果

最新证据已上传至 [reports/slot-refresh-audit](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/slot-refresh-audit)，reports 分支原始报告提交 `e320277a08f70524853a631881d2bc8bb7fe7745`；报告解释更正随后提交至 reports 分支（见该目录 `SUMMARY.md` §10）。

同一台 RG34XX non-SP、BaseOS 1.5.3 / Linux 4.9.170 #19，通过 ADB 分别在 UI 和实际运行 GBA 的场景采样 12 秒：
- IRQ293：UI **59.667 Hz**，GAME **59.583 Hz**；显示 driver 两态均为 `fps=60.2`、`ref_fps=59`、`de_rate=696 MHz`。
- PLL `0x03001040` 两态均为 `0x88001703`（按已记录的相关 sun50iw9 参考公式解为 288 MHz）；没有观察到 120-Hz PLL 编程。
- 本轮 TCON `0x06511044` 读取返回 I/O error，不能把本轮 divider 当作已读回；另一次独立同设备采集中曾读到 `0xf000000b`。
- 面板 `rg34xx_v1`、720×480、820×536、DCLK property 26 MHz、`lcd_if=0`；Slot PID 2301 在两态相同，GAME 只新增加载 `armgba_libretro.so`，显示栈没有重初始化。两态 `high_refresh=1`，但没有带来模式变化。
- 结论：这次运行 UI/GAME 均处于约 60-Hz 路径。LCD IRQ 只是 scanout/vblank proxy；UI presents 和 GBA core committed frames 仍未测量。

### 报告解释更正

新报告最初把“FDT 只有一套 timing”当成动态 120-Hz 模式的否决证据，并把 576-MHz raw word 写成 pvaibhav 的目标配置；这两处已在 reports 分支 [SUMMARY.md §10](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/slot-refresh-audit/SUMMARY.md) 修正。pinned `pvaibhav/slot@5206e3fa39aa4a9b702cd9048a07dab3b5ab95da` 的真实 high pair 是 divider `0xf000000a` + PLL `0x88002b03`（参考公式 528 MHz / 10，约 120.131 Hz）；动态 writer 不依赖 FDT 第二个 timing 节点。这不改变实测结论：设备仍读到 60-Hz PLL word，IRQ 约 59.6 Hz，而且本机 `rg34xx_v1` 不满足 fork 的 `fog_fj035fhd05_v1` 精确 allowlist。

### FPS 计数器审查与安全边界

- UI：pinned Slot 的 `device_app.rs` 在 tracing 开启时每 600 次主 surface swap 输出 `presents/s`。开关为非零 `SLOT_TRACE_LATENCY` 或 `$SLOT_ROOT/System/trace-latency.on`；`OnceLock` 在首次读取时缓存，所以标记文件必须在 Slot 进程启动前存在。下一轮先只读确认 PID 2301 的 stderr fd 去向和安装的 `System/slot` SHA，再决定如何开启/收集日志。
- GBA：pinned armGBA 源码在 VBlank 帧结束递增 `gba->video.frameCounter`，是 core committed-frame 候选。其 `libretro/link.T` 只导出 `retro_*`、对其余符号使用 `local: *`；源码全局 `gba` 并非可依赖的动态导出符号，禁止假设 `dlsym` / `nm -D` 可取得它。必须先将设备 `armgba_libretro.so` SHA 与确切 release/build 匹配，并检查完整 ELF symbol table。只有符号、load bias、指针映射和由 pinned `gba.h` 计算的 `offsetof(GBA, video)+offsetof(Video, frameCounter)` 全部闭合后，才允许做有界、只读的 `/proc/PID/mem` 采样；否则改为自建带诊断导出的 core，不做盲目内存扫描。
- 三项指标必须分别报告：`slot_presents_per_s`、`armgba_committed_frames_per_s`、`lcd_irq_per_s`。LCD IRQ 不代表 UI FPS 或模拟器 FPS。

### 当前 CI 与后续执行顺序

- Linkslot LK Rust feasibility [#45646868006](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45646868006)：**成功**，最新已核验 70 余步综合 host/cross-build 流程全通过（含完整 H700 BL33 overlay、Linkslot 1.0.49、mGBA VFS v3、存储只读回归）。
- H700 boot audit tools [#45646868087](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45646868087)：**成功**。
- 上述均是 host/cross-build 和工具回归，不是刷写固件或实机启动证据。

下一步先完成 release ZIP 与设备 `System/slot` / `armgba_libretro.so` 的 SHA 对照、ELF 符号表检查、armGBA C 结构偏移编译探针；并只读确认 Slot stderr 日志去向。完成这些前，不启用标记文件、不重启前端、不读进程内存。随后先测 UI presents，再在 GBA 运行时按已验证地址采样 committed frame counter。硬件 writer 继续禁用，`hardware_applied=0`。


## 10. 原始 RG34XX boot-package 120 Hz 方案对照（2026-11-26）

已直接审查 [TheGammaSqueeze/rgxx_boot_package_mods `rg34xx` branch](https://github.com/TheGammaSqueeze/rgxx_boot_package_mods/tree/rg34xx)，HEAD `64300b3b6137f7676b63b2e969e58db2870db48b`。完整解析记录见 [原始 boot-package 方案审计](rgxx-boot-package-120hz-review.md)。

核心发现：
- 这是**刷写后固定模式**，不是 runtime writer。60/120 两个包均包含 U-Boot、monitor、p1.dtbo、sunxi FDT；三项非 FDT 组件哈希一致。两份 137,728-byte DTB 的全属性对比只发现一项差异：`lcd_ht=840`（120）或 `1679`（60）；两者 `lcd_dclk_freq=54`、`lcd_vt=536`、720×480，面板名均为 `fog_fj035fhd05_v1`。对应 FDT nominal 119.936 / 60.004 Hz。
- README 指示在 raw `/dev/mmcblk0` 的 `0x1004000` 偏移覆盖 1,310,720-byte `boot_package.fex` 并重启。脚本只检查 `sunxi-package` magic 后执行 `dd conv=notrunc`，不做写后 hash/readback；README 限定 Stock firmware 测试。当前设备不得执行该脚本。
- pvaibhav Slot 是另一条 Linux runtime register writer 路线：对 stock-like 26 MHz / 820×536 profile 直接写 divider10 + PLL528-MHz pair；boot-package 路线则由内核初始化读取 54-MHz FDT 并使用不同的 HT。Slot probe 的硬条件是 26 MHz / HT820，原始 boot package 是 54 MHz / HT840/1679，因此两者 profile 不兼容，不能叠加。
- 更关键的是，两个第三方方案的 FDT/allowlist panel string 都是 `fog_fj035fhd05_v1`；当前测试设备真实 live panel 为 `rg34xx_v1` + `lcd_type=old`。不得通过改面板名或降低 gate 来套用第三方实现。

两种方式都没有实现本项目想要的 UI 120 Hz / GAME 默认 60 Hz 场景策略：boot package 是跨 UI/GAME 固定模式、刷包重启切换；pvaibhav Slot 是启动/唤醒应用同一个全局偏好，游戏进入不切换。保持本项目硬件 writer 关闭、`hardware_applied=0`。


## 11. armGBA committed-frame probe preparation (2026-11-26)

Pinned source reviewed: `MagnominusGit/armGBA@fad6c4b0c0176a76e10667f4aa071a8feb4425c0`. `src/video/video.c` increments `gba->video.frameCounter` at VBlank frame end; `libretro.c` owns global `struct GBA* gba`. `libretro/link.T` exports only `retro_*` and localizes all remaining symbols, so `gba` cannot be assumed visible via `dlsym` / `nm -D`.

New compile-time probe:
- `tools/armgba-frame-layout-probe.c` prints the host layout and cross-target static-asserts the combined `offsetof(GBA,video)+offsetof(Video,frameCounter)`.
- `scripts/test-armgba-frame-layout.py` checks the exact source commit, verifies upstream `sizeof(struct GBA)==788712`, then compiles the same offsets for host, AArch32 ARM, AArch32 Thumb-2 and AArch64.
- Workflow run [#45722327333](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45722327333) on HEAD `23473143b08af1a087775c6e40499de5980cab44` was later **cancelled by a superseding push after 33/74 steps**; this is not a test failure. Its dedicated **Verify armGBA committed-frame layout across H700 ABIs** step had already completed **success** (all static offset assertions compiled). Replacement run [#45722659918](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45722659918) was also cancelled during LLVM tool installation, before reaching the layout step. The latest full feasibility run [#45722935508](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45722935508) on current HEAD `562288737369f8d898a1b22c27f0a08b5c1cb6fc` is in progress; no aggregate pass is claimed yet. The numeric layout artifact was recovered from GitHub Actions artifact `linkslot-lk-rust-feasibility-45722327333` (artifact ID `15072183552`): `offsetof(GBA, video)=11944`, `offsetof(Video, frameCounter)=4`, combined `gba->video.frameCounter` offset **11948 bytes**. Host + AArch32 ARM + AArch32 Thumb-2 + AArch64 static assertions passed. This establishes the pinned source layout only; do not read device memory until the installed ELF is source-matched and its full symbol table/load bias prove the `gba` pointer location.
- H700 boot audit tools [#45722302580](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45722302580): **success**.

Additional release-source finding: pinned pvaibhav Slot's [`cores/armgba/build.sh`](https://github.com/pvaibhav/slot/blob/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da/cores/armgba/build.sh) accepts a source commit at build time, downloads the matching `MagnominusGit/armGBA` archive, applies `link-stop-queued-once.patch`, builds `armgba_libretro` with `-mcpu=cortex-a53`, and emits a `.meta` file containing the source commit and patch hash. The exact source commit/patch metadata of the published H700 ZIP and the installed device core are not yet matched.

Previously recorded installed core identity remains:
- device `/mnt/sdcard/SYSTEM/armgba_libretro.so`: SHA-256 `4ebae098cc4848fb459fc18299c39acbdbbbeb732db5c3f66d1ff3f6e142e889`;
- device `/mnt/sdcard/System/slot`: SHA-256 `8f0fd40da4ac161e5fbb9a56a99c254acf8b2f31806e2be6bdaa929b5ee28505`;
- H700 release ZIP `slot-v1.16.0-pvaibhav.2-h700.zip`: SHA-256 `1f9bdaa1110eb4f6006cc9da6a8bb34bca694ea53555c3138dcf656f0dfd1fca`.

Next gate: compare ZIP and device hashes, inspect the ZIP's armGBA `.meta`, inspect full ELF `.symtab` versus `.dynsym`, and read the current Slot process stderr FD/log route. No marker file, front-end restart, or `/proc/PID/mem` access until those read-only checks establish the exact build and pointer source.


## 12. 官方二进制溯源闭环与 high_refresh 门控解释（2026-11-26）

### 二进制身份已闭环

最新 ADB 报告已上传至 [reports/slot-refresh-audit/binary-identity](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/slot-refresh-audit/binary-identity)，reports HEAD `9662b6b218fe1e7af0edb21e53f8098520d2cdd3`。

- 官方 H700 ZIP SHA-256 与预期完全一致：`1f9bdaa1110eb4f6006cc9da6a8bb34bca694ea53555c3138dcf656f0dfd1fca`。
- 设备 `System/slot` SHA-256 `8f0fd40d...`、`SYSTEM/armgba_libretro.so` SHA-256 `4ebae098...` 均与 ZIP 内文件完全相同；另外 gpsp/mGBA/Gambatte 三个核心和两个启动脚本也全部匹配。
- armGBA ELF Build ID：`08309a288967a62cf4155e27d1b10ce13a5e33ef`；未 strip，完整 `.symtab` 4,960 项，`.dynsym` 240 项。
- 官方包内 `armgba-*.meta` 记录的真实源码 commit 是 **`df6b2631291f249d54a00dd14f49bc893e438feb`**，设备与 ZIP 相同；编译参数 `-mcpu=cortex-a53`；link patch SHA-256 `6ffb6a9f83b6fdaf77350725e74f0bedbd530256bce3e5e4fb0cc4b9e5e794db`，与实际 patch 一致。
- 重要修正：先前 layout probe 用的 `fad6c4b0c0176a76e10667f4aa071a8feb4425c0` 是另一个较新的上游 checkout，并非 v1.16.0-pvaibhav.2 所包含的源码。此前从 fad6c4b0 得到的 11948-byte 数值在来源未匹配时不能直接使用。已把 CI checkout 与 PINNED_COMMIT 改为官方 `.meta` 所指定的 df6b263...；run [#45724910634](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45724910634) 的 exact-source layout step **成功**，artifact ID `15072724723` 的 JSON 确认同样是 `GBA.video=11944`、`Video.frameCounter=4`、合计 **11948 bytes**。本轮总体 workflow 后续因新文档提交被取消，这不影响已经成功完成的专用 layout step。

### `high_refresh=1` 不生效的原因已由源码与二进制共同解释

pinned Slot 源码 `pvaibhav/slot@5206e3fa39aa4a9b702cd9048a07dab3b5ab95da`：

- `DevicePlatform::new` 从环境变量 `BASEOS_TARGET` 取目标，缺失时才回退读取 `/etc/baseos-release` 的 `BASEOS_TARGET=`。
- `DevicePlatform::probe` 设置 `refresh = target.and_then(|target| PanelRefresh::probe(sysfs, target))`。
- 普通 target `rg34xx` 在 `PanelRefresh::probe` 中要求 FDT `lcd_driver_name=fog_fj035fhd05_v1\0`；同时要求 720×480、HT820、VT536、DCLK26MHz 和 sunxi_dump dump/write 节点可写。
- 现场 FDT 的分辨率与时序数值符合，但驱动名是 **`rg34xx_v1\0`**，与硬编码要求不同。因此 probe 会返回 `None`，即 `DevicePlatform.refresh=None`。
- `supports_high_refresh()` 返回 `self.refresh.is_some()`；`set_high_refresh()` 通过 `self.refresh.as_mut().is_some_and(...)` 调用 writer。故能力 gate 为 false 时，设置文件中的 `high_refresh=1` 不会写 PLL/TCON，也不会使能高刷。离线二进制反汇编的 `self+0xb8` sentinel 路径与此 Option 门控相符。

这已经足以解释设备仍保持约60Hz，不需要 dump Rust 运行时结构，也不需要读 `/proc/PID/mem`。不得通过伪造面板名或放宽 allowlist 绕过该门控。

### armGBA 符号与 UI 日志通道

- `gba` 在 `.dynsym` 中为 0 个，在完整 `.symtab` 中可见；它不可以通过 `dlsym` 获取。
- `.symtab` 有两个同名 `gba` LOCAL OBJECT：`0x4c16a8` 与 `0x4259b8`。源码中同时存在 `libretro/libretro.c` 的主 GBA 指针和 `libretro/link.c` 的文件局部 link 指针。必须先通过 ELF 符号与调用点反汇编把两个地址准确对应，再选取主 GBA 指针；禁止仅按同名符号任选一个。
- Slot PID 2301 的 fd/1 与 fd/2 均指向同一个 `/mnt/sdcard/Logs/slot.log`，stderr 路径已明确。当前 `SLOT_TRACE_LATENCY` 未设置，`/mnt/sdcard/System/trace-latency.on` 不存在，因此尚无 UI present-rate trace。
- 当前报告保持完全只读：无 marker、无 Slot 重启、无 `/proc/PID/mem` 访问、无硬件写入。

### 后续次序

1. **已完成**：按官方 release commit `df6b2631291f249d54a00dd14f49bc893e438feb` 的 layout probe 通过，精确偏移为 11948 bytes，详情见 [armGBA layout 记录](armgba-frame-counter-layout-v1.16.0.md)。
2. 只读采集当前 Slot PID 的 `/proc/PID/maps` 与 ELF `PT_LOAD` program headers，计算 `armgba_libretro.so` load bias；通过 `retro_run` 引用反汇编区分两个同名 `gba` 符号。此步骤不读取进程内存。
3. UI presents：stderr 已落到可读日志文件，但 tracing 必须在 Slot 启动前启用；创建 marker 和重启前端属于设备配置动作，当前仍未执行。
4. armGBA committed-frame counter：精确源码偏移、正确 `gba` 指针、load bias 三项完全闭合后，再单独决定是否进行有界只读进程内存采样。此前用户给 Agent 的采集约束禁止 `/proc/PID/mem`，本轮仍保持不读。

UI presents、armGBA committed frames、LCD IRQ 继续作为三个独立速率指标。所有硬件 writer 保持关闭，`hardware_applied=0`。


## 13. armGBA 运行时映射与主指针裁决（2026-11-26）

最新只读运行时证据已上传至 [reports/slot-refresh-audit/binary-identity/runtime-mapping](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/slot-refresh-audit/binary-identity/runtime-mapping)，完整分析见 [armGBA v1.16.0 runtime mapping](armgba-runtime-mapping-v1.16.0.md)。

- Slot PID 本轮仍为 2301；游戏态核心 4 条 maps 映射，退出后 0 条，PID 未变，确认核心使用 dlopen/dlclose。
- ELF PT_LOAD #0/#1 对应三条文件映射（r-xp/r--p/rw-p）独立解出的 load bias 全部为 `0x7f64070000`，交叉验证通过。
- 主指针已裁决为 `gba@0x4259b8`（runtime `0x7f644959b8`）；`gba@0x4c16a8`（runtime `0x7f645316a8`）是 link cable 模块缓存。证据是主指针赋值源、retro_run 21 处 GOT 读取、linkSetGba 的缓存写入路径、retro_unload_game 先清缓存再清主指针。
- 官方精确源码 `df6b2631291f249d54a00dd14f49bc893e438feb` 中 `video.c` 在 VBlank 帧结束递增 `frameCounter`，`retro_run` 等待该计数器推进。此前精确布局 `gba->video.frameCounter` 偏移 11948 bytes 已与本轮 ELF 来源闭环。
- **尚未读取**主指针的内容或 frameCounter；未使用 `/proc/PID/mem`，未启用 UI tracing、未创建 marker、未重启 Slot、未写硬件。偏移 + 映射地址闭合只代表可以设计有界读取，不等于已经获得帧率实测。

下一步需要明确授权后才执行设备操作：启用 UI trace marker 并重启 Slot；以及在游戏态做有界只读的主指针/计数器采样。在此之前继续保持无进程内存读取、无配置更改、无硬件写入。UI presents、armGBA committed frames、LCD IRQ 三项独立统计，`hardware_applied=0`。


## 14. UI presents 与 armGBA committed-frame 实测闭环（2026-11-26）

已按用户明确授权完成全自动 ADB 采集，报告见 [fps-sampling](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/slot-refresh-audit/fps-sampling)。本节取代前面各节中“marker/重启/进程内存读取尚未执行”的待办表述。

- 预检：单设备 `state=device`，uid=0，`BASEOS_TARGET=rg34xx`，`/mnt/sdcard` 可读写；`/etc/inittab` 有 `::respawn:/sbin/frontend-session`，Slot PPID 链 `slot -> frontend-session -> PID 1` 已确认，因此允许 SIGTERM 后由 init respawn。核心 SHA-256 `4ebae098...e889` 与官方包一致，`SLOT_TRACE_LATENCY` absent，marker 初始不存在，原有 `slot.log`（3624 B，`0d5991ea...350b1b`）先备份再采集，未覆盖。
- `slot_presents_per_s`：临时创建 `$SLOT_ROOT/System/trace-latency.on` 并 SIGTERM 重启后，从新 PID 22514 的 `fd/2`（实测 `/mnt/sdcard/Logs/slot.log`）取得 4 条 pace 记录：58.67 / 59.57 / 59.57 / 59.57，均值 59.345，nominal 60.00 fps，stale 全 0，missed game periods 10。日志行本身无时间戳；host 轮询看到第 2/3 条的时间是 01:09:58Z / 01:10:01Z。
- `armgba_committed_frames_per_s`：同一 PID 22514 在游戏态 `dlopen` 核心（`CT特种部队1.gba`，日志确认 `core /mnt/sdcard/System/armgba_libretro.so`），每轮重扫 PID/maps 并由 r-xp/r--p/rw-p 三行独立解出相同 bias `0x7f55ae0000`；主指针变量 `bias+0x4259b8`，指针 `0x7f1c0ea4d0` 对齐且稳定，计数器 `+0x2eac` 的 4 B 范围每轮验证为 rw 映射。19.825228 s 内 79 个样本、1184 帧，实测 **59.722 committed frames/s**；因 adb 往返实际采样率 3.934 Hz（目标 5 Hz），间隔 0.249–0.266 s，uint32 差分量化误差约 ±3.93 fps。`SLOT_TRACE_LATENCY` 始终 absent。
- `lcd_irq_per_s`：只读 `/proc/interrupts`，IRQ 293 在 20.187913 s 内增加 1203 次，即 **59.590 IRQ/s**；这仍是扫描/中断代理，不能当作前两项 FPS。
- 内存边界：每样本仅 8 B 主指针 + 4 B 计数器，共 **948 B**（预算 4096 B）；无扫描、无 ptrace、无 gdb、无进程内存写入。
- 清理：长按 Menu（BTN_TL2=312）退出游戏后 armgba 映射回到 0；删除本轮 marker 恢复 ABSENT；再次 SIGTERM 并经 init respawn 到 PID 8064，其 `fd/2` 仍为 `/mnt/sdcard/Logs/slot.log`，`presents/s` 行数回到 0，UI 正常，marker 不存在。未重启设备，未改配置/ROM/存档/刷新率。
- 例外与恢复：首次 SIGTERM 后 `frontend-session` 进入 `baseos-control-center --diagnostics` 并阻塞，60 s 内没有新 Slot PID；该失败已如实记录。B 键切到前端选择器，MENU 无效，只对该辅助进程发 SIGTERM 后 init 正常 respawn。用户后来提示在选择器上选中 `slot-pvaibhav` 按 A 也可返回，此时已完成 respawn，故未再操作。

结论：UI presents、armGBA committed frames、LCD IRQ 三项均约 60/s，但这只说明当前都在 60 Hz 路径上，不构成 120 Hz 可用的证据。`hardware_applied=0` 保持不变；PLL/TCON/CCU/PMIC、`/dev/mem`、面板 allowlist 和 120 Hz writer 均未触碰。


## 15. FPS 采样数据独立复核与补充（2026-11-26）

对 [reports/fps-sampling](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/slot-refresh-audit/fps-sampling) 的原始 CSV/日志/JSON 做了第二次独立计算，复核笔记在 [ANALYSIS.md](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/slot-refresh-audit/fps-sampling/ANALYSIS.md)。

- CSV 的 79 行、78 个间隔、首末 counter 11263→12447、逐行 delta 总和 1184 全部一致；1184 / 19.8252277 s = **59.7218866 armGBA committed frames/s**。实际采样频率 **3.934 Hz**（计划 5 Hz），每轮 12 B、共 948 B；瞬时 fps 抖动来自 15/16 帧整数差分量化。
- 完整 `slot-game-launch.log` 比首份 UI 摘要多包含窗口：进入游戏前 25 个 600-swap 窗口均值 **59.503 presents/s**；进入 armGBA 后 31 个窗口均值 **59.561 presents/s**。原始 `slot-ui-trace.log` 中最初 4 个窗口均值 59.345 仍正确，但只代表启动后首段，不是整个 UI/游戏期间的总均值。
- IRQ293 仍为 **59.590 IRQ/s**。游戏态 presents 59.561、核心 committed frames 59.722、IRQ 59.590 三项互相接近，但分别代表前端 swap、核心完成帧、LCD IRQ。
- 报告 bundle 的 9 个原始文件 SHA-256 全部匹配清单。后增 `ANALYSIS.md` 的 SHA 也已加入 fps-sampling 清单。
- 证据完整性补充：原始采样 PID 22514 的 maps 首尾快照后来确认未找回，其 `0x7f55ae0000` bias 仍为摘要级证据。后续已对新 PID 8064 做独立 maps-only 补采，验证相同 ELF 的 RX/R/RW 三行 bias 解算方法；该补采不替代旧 PID 原始证据，详情与边界见第16节。
- 预检的 ADB descriptor 显示 `occam/Nexus_4/mako`，但同一 ADB session 取回的 uname、DT SoC、BaseOS release 均指向 RG34XX/H700；该 descriptor 不能单独推翻设备身份，后续报告继续记录 DT model/compatible 与 CPU info 即可。
- 恢复记录显示第一次 SIGTERM 后前端链进入 diagnostics helper 并阻塞；处理该 helper 后 init respawn 正常。最终 marker ABSENT、tracing 关闭、Slot PID 8064 回到 UI、armgba 映射 0。不要把第一次 respawn 失败从后续自动化报告中省略。

当前三项结果仍全部约 60/s；没有 120-Hz 证据。所有刷新率硬件 writer 继续关闭，`hardware_applied=0`。


## 16. armGBA maps-only 独立补采与证据边界（2026-11-26）

原始游戏帧采样 PID **22514** 的首尾 `/proc/PID/maps` 快照已在本地采集目录、Agent 工作目录与仓库中搜索，确认未找回。旧采样记录的 load bias `0x7f55ae0000` 和 map signature `8f7db48f8bb8ea5f` 仍应明确视为采集摘要级信息；不得用另一个进程的 maps 回填或声称已证明旧 PID 的地址。

已追加独立 maps-only 补采，完整报告位于 [MAPS-SUPPLEMENT.md](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/slot-refresh-audit/fps-sampling/MAPS-SUPPLEMENT.md)：

- 明确 ADB serial 只有一台 `state=device`；设备侧 uname、DT model/compatible 与 `BASEOS_TARGET` 均确认 RG34XX/H700。
- 新 Slot PID **8064**，同一 GBA ROM 与官方 armGBA core。首尾两份完整 `/proc/PID/maps` 各 **425 行**；两次采集的 armGBA 四行映射逐字节相同。
- 三条文件映射独立得出相同新 bias：RX `0x7f339a0000 - 0`、R `0x7f33b25000 - 0x185000`、RW `0x7f33b28000 - 0x188000`，结果均为 **`0x7f339a0000`**。新 bias 与 PID22514 不同属 ASLR 正常现象；本结果验证 PT_LOAD 映射/对齐与 bias 算法，不回溯证明 PID22514 的旧 bias。
- 两份 maps 文件、补充说明以及整个 `fps-sampling/SHA256SUMS.txt` 中列出的 **13/13 文件**均已独立复算 SHA-256，全部匹配。
- 采集后长按 Menu 退出；PID8064 仍存活并回到 UI，armGBA maps=0、trace marker ABSENT、`SLOT_TRACE_LATENCY` absent。未重采 FPS、未读 `/proc/PID/mem`、未 ptrace/gdb、未杀进程或重启 Slot，未改配置/ROM/存档/启动分区。
- 未写 PLL/TCON/CCU/PMIC、未使用 `/dev/mem` 或 `sunxi_dump/write`；`hardware_applied=0`。

报告与分析已同步修正：[fps-sampling README](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/slot-refresh-audit/fps-sampling/README.md)、[独立分析](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/slot-refresh-audit/fps-sampling/ANALYSIS.md)。原 FPS 数值不变：预游戏 presents 59.503/s、游戏态 presents 59.561/s、armGBA committed frames 59.722/s、IRQ293 59.590/s。显示刷新率仍没有 120-Hz 证据；硬件 writer 继续禁用。


## 17. 120-Hz 显示能力的当前判定与下一门槛（2026-11-26）

显示底层分析和刷新率测量已不再是待办：kernel #19 匹配二进制反向审计已闭合 `rg34xx_v1 + lcd_type=old` 的初始化、PLL factor table、TCON divider 与 clock parent；同一设备的 UI/GAME presents、armGBA committed frame 与 IRQ 也均已独立测量，结果约 60/s。当前 live 60-Hz pair 为 PLL `0x88001703`、divider `0xf000000b`。

本项目不能把外部资料中的 120-Hz 数值直接视为 RG34XX 本机支持：

- Anbernic [官方 RG34XX 产品规格](https://win.anbernic.com/product/517.html)仅公布 3.4-inch IPS OCA、720×480，没有 LCD 模组 part number、revision、native refresh 或 maximum pixel clock。
- [Slot 上游 issue #147](https://github.com/BrandonKowalski/slot/issues/147) 中维护者解释，相关面板略低于 60 Hz，120-Hz tweak 是把 display driver 运行到 120 Hz；同一讨论后来因 stutter reports 和实现复杂度决定跳过该 hack。此为维护者/社区背景，不是针对本机模组的 datasheet 或电气验证。
- 本机 live identity 为 `rg34xx_v1 + lcd_type=old`，与 pvaibhav writer 精确 gate 的 `fog_fj035fhd05_v1` 不同。公开的 FOG FJ035FHD05 硬件实测资料对应的是 RG34XX-SP（compatible `anbernic,rg34xx-sp-panel`），不是本项目的非 SP RG34XX；不能把 SP 面板型号迁移到本机。不得改名、放宽 allowlist 或移植其 writer。

**此前显示专项下一步（现已降级为 deferred）：**查明测试机实际 LCD 模组供应商、完整料号及 revision 并取得对应 datasheet；在此证据出现前维持 60 Hz，不推进 120-Hz profile 或任何 PLL/TCON 写入。

原 FPS 数据不变：预游戏 presents 59.503/s、游戏态 presents 59.561/s、armGBA committed frames 59.722/s、IRQ293 59.590/s。全体刷新率 writer 继续禁用，`hardware_applied=0`。


## 18. 将单核 LK event-step 纳入持久化 H700 workspace（2026-11-26）

本轮按 Linkslot H700 runtime architecture §6 第 12 项继续推进，不再停留于纯 input adapter：

- 发现已有 `integration/linkslot/h700-frontend-core/slot-lk-runtime` 实现，但它之前未列入 workspace members，package version 仍为 1.0.36。由此此前 workspace 的 `cargo test/check --workspace` 没有覆盖这个核心运行时组件。
- 已将 `slot-lk-runtime` 加入 `integration/linkslot/h700-frontend-core/Cargo.toml` 的 workspace members，并将 package version 对齐到当前 Linkslot 1.0.49 适配基线。workspace 从 13 个增加为 **14 个** Rust packages。
- 更新 [业务 crates README](../integration/linkslot/h700-frontend-core/README.md) 与 [runtime architecture](linkslot-h700-runtime-architecture.md)：明确 event-step 行为、边界和下一步，避免把 event loop 误写成已完成的 Linkslot LK app。
- 现有 runtime 的明确契约：每步从 C-owned `H700TimeServiceV1` 读取一个单调时间戳；从 C-owned `H700InputServiceV1` 有界抽取事件；复用 Linkslot `Gestures` 生成并按序 dispatch actions；最后以同一个时间戳 tick application handler。其自身不管理线程、heap、GPIO、显示、音频、VFS 或 emulator。
- 现有测试覆盖 action dispatch 顺序、MENU 长按 timer、POWER edge 顺序、无效 time/input ABI table。加入 workspace 后，这些测试会随全 workspace host tests 执行，AArch32 `armv7a-none-eabi` 与 AArch64 `aarch64-unknown-none` 的 `--no-default-features` cross-check 也会包含该 crate。

代码提交：
- workspace member：`184c9ec6970412d24631fcd44323c57f00f8647a`
- runtime package version：`6d489f072e38fee2866a8e64d39f11a56622e0d7`
- crate README：`514371cfc467c596239210a8637cdd6c206becf0`
- runtime architecture：`86de40fb9e38939137e90dff0cdf0f60cb602bcf`

验证状态（截至本节更新）：
- [H700 boot audit tools #45739691039](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45739691039)：Linux host-tools 和 Windows PowerShell 两个 job 均已成功；这是工具链回归，不是 runtime crate 的直接验证。
- [首次验证 #45739691044](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45739691044) 因后续 push 被 supersede/cancel；它没有作为聚合成功结果。专用 runtime-slices run [#45739686476](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45739686476) 首次把 event-step 编入 workspace 后，正确暴露了 `slot-input` 根模块未导出 `H700_BTN_*` / `H700_INPUT_*` 常量的 E0432 编译错误。已在 `slot-input/src/lib.rs` 补全重导出，修复提交 `a544492a428996a5bcbb981f114b3b117e001653`。
- 修复后的专用 runtime-slices run [#45739871351](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45739871351)：截至本节更新，C input service ABI/queue job **success**；Rust business job 的 **persistent workspace tests（host default + no-default）及 AArch32/AArch64 no-default cross-check 已整体通过**，upstream input/model regression 和 overlay application 也通过；完整 Frontend build 正在运行，QuickRow regression 尚待执行。不可提前把整个 run 记为 success。
- 同一修复提交的完整 feasibility run [#45739871348](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45739871348) 仍在运行，当前处于 LLVM cross-link tools 安装阶段；不将其记为完成。
- [H700 Linkslot runtime slices workflow](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/workflows/h700-linkslot-runtime.yml) 的 `integration/linkslot/h700-frontend-core/**` path filter 会因本轮 workspace/manifest 变化触发；须以对应最新 run 的结论为准。

当前进展是 **runtime floor 已持久化并通过专用 host/AArch32/AArch64 CI**，不是可启动 LK App。下一项为 application-owned action router，把 Quick Menu 与 game-library/search 逻辑接到 handler；之后再依次设计 VFS v3/static-core lifecycle、renderer/display service descriptors。不得提前接通未经批准的 framebuffer/scanout、GPIO、PCM 或存储硬件路径；heap/BL33 memory gate与单 active core约束保持 fail-closed。


## 19. Linkslot Quick Menu 应用动作路由抽取（2026-11-26）

按第 18 节完成 runtime floor 后，继续推进 application-owned router。对照 immutable Linkslot 1.0.49 `upstream/linkslot` 的 `crates/slot/src/app.rs`，确认：
- Shelf 的 `Action::QuickMenu` 打开 Quick Menu（初始 `QuickRow::MAIN[0]`）。
- Playing 的同一动作进入 in-game utility/menu 路径；不能全局捕获。
- 只有 `Phase::QuickMenu` 时才调用 `quick_menu_input(row, action)`：Up/Down 移动 `row.up/down()`；Left/Right 调 `change_setting(row, false/true)`；A 调 `open_quick_row(row)`；B 或 MENU 返回 `row.parent()`，无 parent 则回 Shelf。
- `open_quick_row` 负责区分嵌套页、Modal/utility、网络/设置页和无动作 leaf；这些副作用/状态仍归应用本身，路由模块不能擅自推断。

已增加 `integration/linkslot/h700-frontend-core/slot-app-router`（no_std，无第三方依赖），并加入 workspace，包版本 1.0.49：
- `QuickMenuRouter::new/at/row/route` 仅维护 Quick Menu 当前行。
- typed command：`CursorMoved`、`ChangeSetting`、`OpenRow`、`Parent`、`Close`、`Ignore`。
- 调用方需在 Quick Menu phase 中调用；它不拥有 App phase、不接管 MENU 全局含义、不执行配置保存/VFS/显示/音频/硬件操作。
- 6 项单元测试覆盖入口行、上下导航、左右设置代理、A 打开代理、B/MENU 父页/退出、其他动作忽略和 page-local navigation。

代码提交：
- crate manifest：`15b7e398d04a26dc0bf60671af161e1a32070688`
- crate implementation/tests：`0887806f3c10305e63d23db7b7e929672ab131cf`
- 15th workspace member：`4865ee52153abe5834ed21d9dd60e209d97d97bc`

验证：专用 15-member H700 runtime-slices [#45740732818](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45740732818) **success**。两个 host workspace test 模式（default / no-default）、AArch32 `armv7a-none-eabi` 与 AArch64 `aarch64-unknown-none` no-default checks、upstream input/model regression、完整 Linkslot 1.0.49 Frontend overlay build、CPU Performance QuickRow fail-closed test、C input ABI/queue 与两架构 cross-compile 全部通过；新 `slot-app-router` 的 6 项测试在两个 host test 模式均通过。更广的 H700/LK feasibility run [#45740749359](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45740749359) 仍在执行交叉链接编译。当前新 router 只处理 Quick Menu phase；游戏搜索键盘/候选/结果选择是下一切片。仍不是可启动 LK App，无设备写入或实机执行。

下一步：
1. 搜索结果列表已经抽取；剩余搜索 UI 切片为 keyboard cursor intent 与 Pinyin candidate focus/page/commit 的纯状态路由，保留原 UI Cursor 和注入式 pinyin provider。
2. 将 `slot-app-router` QuickMenu/SearchResults commands 接入上层 Linkslot App adapter，调用既有状态变化逻辑，不在 router 中写 VFS/config。
3. 再推进 VFS v3 provider 与静态 Libretro core lifecycle；display/PCM/GPIO 仍独立 gated。


## 20. SearchResults 列表路由抽取与 CI（2026-11-26）

在 Quick Menu 路由后继续对照 immutable upstream Linkslot 1.0.49 `crates/slot/src/app.rs::search_results_input()`，抽取 SearchResults 状态路由至 `slot-app-router`：

- `SEARCH_ROWS=7` 与上游保持一致。
- Up/Down 在 hits 列表内循环选择；Left/Right 按 7 项跳转并 clamp 首尾。
- 选中项越出 viewport 时，`offset` 按上游 `keep_search_result_visible()` 规则更新。
- A/Insert 发出 `SelectHit { index }`；B 发出 `ReturnToKeyboard`（App 保留 query 并重新打开原键盘）；X/MENU 发出 `Close`。
- Router 只保存 hit_count / selected / offset，不复制 query、pinyin composition/candidates 或 SearchHit 数据；真实 shelf/folder 选择仍由 App 完成。
- 新增 6 项测试，覆盖垂直回绕、按 7 项水平跳页及 clamp、viewport 保持、A/Insert、B/X/MENU 和空列表边界。Quick Menu 原 6 项测试保留，因此当前 router 总计 12 项测试。

实现提交：`db325358da13f33a8e4c8661804c41bc248f4191`。
验证：[H700 Linkslot 1.0.49 runtime slices #45741150698](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45741150698) **success**。15-member workspace default/no-default host tests、AArch32/AArch64 freestanding checks、upstream input/model regression、完整 Frontend overlay build、CPU QuickRow fail-closed、C input ABI/queue host + cross compile 全部通过。新 SearchResults 路由代码包含在两个 workspace test 模式中。

下一切片为搜索键盘 cursor intent 与 Pinyin candidate focus / page / commit；渲染 Cursor、拼音表、候选字符串和查询文本仍归 UI/App，不复制资源或索引。随后才接入上层 App adapter 与 VFS/core lifecycle。当前不代表 bootable LK App，也没有设备端执行或硬件写入。


## 21. SearchKeyboard / Pinyin candidate 输入路由抽取与 CI（2026-11-26）

继续对照 immutable upstream Linkslot 1.0.49 `App::game_search_input()`，在 `slot-app-router` 增加 stateless `route_search_keyboard(action, snapshot)`：
- Snapshot 只读取 App 当前 candidate_focus、composition 是否非空、candidate count/page/pages/selected；不复制 keyboard query、拼音 composition、候选字符串或字典。
- Select：有 composition 且候选不空时进入候选焦点；已聚焦时退出焦点。
- 候选焦点下：Up/Down 按 pages 循环；Left/Right 在候选索引循环；A 返回 CommitCandidate；B 返回退出候选焦点。
- 非候选焦点下：方向键返回现有 UI Cursor 移动意图；Y 切换键盘布局；A 激活当前 UI key；B 退格；Start 确认搜索；X/MENU 关闭搜索。
- App 仍负责执行刷新候选、search_push/backspace、commit candidate、confirm hits 与 close flow；PinyinProvider、UI Cursor、SearchHit/shelf/folder 数据与 VFS 全部保持在原有层。

代码提交：`da457481e12cd6317519c9a13c6ed6bfc7dbfaa3`。新增 7 项 host tests；和既有 QuickMenu 6 项、SearchResults 6 项合计 **19 项 app-router tests**。

验证：[H700 Linkslot 1.0.49 runtime slices #45741616363](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45741616363) **success**。workspace 双 host test 模式、AArch32/AArch64 no-default checks、upstream input/model regression、完整 Linkslot 1.0.49 Frontend overlay、CPU Performance QuickRow fail-closed、C input queue + ABI 两架构交叉编译均通过；因此新 SearchKeyboard 路由在 default 和 no-default host tests 中均已执行通过。

下一步不再继续拆单个按键动作，而是实现薄的 application adapter：仅在对应 App Phase 调用三种 router，把 typed commands 映射回现有 QuickRow/Keyboard/SearchResults 状态与 App 方法；所有持久化/VFS/设备副作用留在原有 handler。Adapter 通过后再推进 VFS v3 + 静态 core lifecycle 的只读/内存边界。


## 22. Phase-aware Linkslot App input dispatcher（2026-11-26）

在完成 QuickMenu、SearchResults、SearchKeyboard 三个纯路由后，新增 `route_app_input(owner, action)` 作为组合式 phase-aware dispatcher：

- `Shelf + Action::QuickMenu` => `OpenQuickMenu`；`Shelf + X` => `OpenGameSearch`。
- `Playing + Action::QuickMenu` => `OpenInGameMenu`；保持与 Shelf MENU 语义不同。
- App 已将输入 owner 判定为 QuickMenu / SearchResults / SearchKeyboard 时，dispatcher 调用相应 typed router。
- 其他 owner 返回 `DelegateToApp`，不截断 power、upgrade、sync、popup、wifi、utility、guide 等旧 handler。
- `AppInputOwner` 是调用方提供的当前 owner snapshot；dispatcher 不扫描 App 字段、不决定 overlay 优先级、不重写 phase、不执行任何 side effect。

特别说明：这一步提供了**可集成的 dispatcher seam**，但尚未修改 immutable upstream 的 `App::apply()` 调用点，也不能声称真实 Frontend 输入已经全部通过该 dispatcher。下一步应在隔离的 Linkslot 1.0.49 source overlay 中，按现有 priority order 在正确位置接入 dispatcher，同时保留其他 modal/overlay handlers 原样运行。

实现提交：`8c4efd5de6250d421067e14f7b96df648ad1aaeb`。增加 4 项测试，总 app-router 23 项测试。
验证：[H700 Linkslot 1.0.49 runtime slices #45742086671](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45742086671) **success**：15-member workspace default/no-default host tests、AArch32/AArch64 no-default checks、upstream input/model regressions、完整 Linkslot Frontend overlay 编译、CPU Performance QuickRow fail-closed 及 C input ABI/queue host 与 cross-compile 全部通过。该结果仍是 host/cross-build，不是实机 boot。

下一步：在 CI 的临时 Linkslot checkout 中增加 priority-safe App integration patch 与 regression tests；不能把源 patch 的成功应用等同于真实 LK runtime 已可启动。此后再推进 VFS v3/static-core 的 App wiring。

## 23. Linkslot App phase dispatcher 接入真实 App::apply()（2026-11-26）

第 22 节的 route_app_input(owner, action) 已从接口 seam 前进一步，接入隔离 CI checkout 中的 immutable Linkslot 1.0.49 App::apply()：

- 新增 scripts/apply-linkslot-app-router.py，只接受固定上游快照 241e7c1974926fa22552b200852914ceaccbfd42；将持久化 slot-app-router crate 复制到临时 checkout，添加 path dependency，并应用 App adapter，不修改 upstream/linkslot 分支。
- App adapter 将 QuickMenu typed commands 映射回原有 change_setting()、open_quick_row() 和 Phase 更新；SearchResults 命令保留原 Query/SearchHit 与 shelf 选择流程；SearchKeyboard 命令继续使用 UI Cursor、Pinyin candidate refresh/commit 和原有 search confirm/close handler。
- App::apply() 继续按原顺序先处理 power menu、upgrade、game intro、sync、search overlays、键盘、popup、transfer/Wi-Fi/sync/utility、adjust 和 game menu，再路由基础 App phase；SearchResults 和 GameSearch keyboard 在原有 owner 优先级位置路由，普通虚拟键盘仍走原 handler。
- Shelf dispatcher 受 core_picker.is_none() 限制，避免覆盖 CorePicker；Playing MENU 仍进入原 in-game utility menu；Shelf MENU/X 的语义保持独立。游戏搜索键盘 UI 的 Cancel key 继续只清键盘和 search index，不错误触发 shelf reset。
- 新增 scripts/test-linkslot-app-router-integration.py，检查 dispatcher 命令映射、App 既有 side-effect handler、overlay 顺序、CorePicker gate 与三类 phase 的接线。
- 两个 H700 Linkslot workflow 都已增加 overlay patch 和静态检查步骤；完整 Frontend build 发生在 router 接线之后。

实现提交：
- App integration patcher：1580e309e599c51e84dd19d79a03a3861acca790
- App integration static regression：9ae79a938af1dd90f40bfcffcc81ba4fc705d4d4
- runtime-slices workflow：9806b7aaa4086e0bd3350fe16f1005f994631063
- full feasibility workflow：cf534e5fe5e2f13d97958a7a822fa2cb64eb13b9

验证：
- [H700 Linkslot 1.0.49 runtime slices #45745328247](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45745328247) **success**。两个 job 均成功：15-member workspace default/no-default tests、AArch32/AArch64 no-default checks、upstream input/model regression、router integration static guard、完整 Linkslot Frontend H700 overlay 编译、CPU Performance QuickRow fail-closed、C input ABI/queue host + cross compile 均通过。
- [Linkslot LK Rust feasibility #45745332208](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45745332208) **success**。完整 job 69/69 steps 完成：BL33 CRT0/vector、H700 renderer、15-member workspace、App router integration、完整 Frontend 编译、CPU QuickRow、CMD17/GPT/FAT/VFS host bootstrap、FAT32/exFAT real-image regressions、mGBA AArch32 archive、VFS v3/newlib 构建、static API 绑定与实际 mGBA archive 静态链接、AArch32 LK-as-BL33 layout probe 和 artifact upload 全部 success。

当前结论：**App phase routing 已实际接入 1.0.49 App 源码，并通过专项完整 Frontend 编译；完整 LK feasibility 验证仍以 workflow 最终结论为准。** 所有证据仍是 host/cross-build，不是 RG34XX 实机启动或运行；没有执行 payload、没有写 SD/Boot0/TOC1，也没有开放 VFS 写回。H700 memory gate、单核 fail-closed、显示/PCM/GPIO/电源硬件门槛保持不变。

第24节已完成只读 VFS 多文件/目录句柄独立游标、16/8 槽位容量与失效句柄回归；第25节已完成 VFS backend 在 static mGBA retro_init() 之前安装、并在 retro_init() 内成功协商 VFS v3 的 host 集成验证。下一步转向把这些 C-owned service descriptor 纳入 LK startup owner/lifecycle，同时保持持久化写入关闭，直到 FAT 长文件名、flush/barrier、原子替换和掉电一致性具备证据。

## 24. H700 read-only Libretro VFS 多句柄生命周期回归（2026-11-26）

在第 23 节 App phase routing 接入后，继续补强 C-owned H700 FAT/CMD17 VFS provider 的固定句柄表回归；本轮只增强 host test，不开放写入，也不改硬件 backend：

- 在 platform/h700/storage/mmc/tests/h700_fat_vfs_backend_test.c 增加同一 ROM 的两个同时打开句柄，交错读取并验证各自 position/tell 与 get_path 独立。
- 穷举 H700_FAT_VFS_MAX_FILES（16）个文件槽位：全部可同时分配、地址互异；第 17 个 open fail-closed；单独 close 后 stale handle 在 slot reuse 前被拒绝；释放槽位可重新打开。
- 穷举 H700_FAT_VFS_MAX_DIRS（8）个目录槽位：全部可同时分配、地址互异；第 9 个 opendir fail-closed；两个目录迭代器分别保有当前 entry/EOF 状态；close 后 stale handle 被拒绝，释放槽位可复用。
- 保持只读检查：volume.wr 仍为 0，write/flush/remove/rename/truncate/mkdir 拒绝，拒绝的 mutation 不发出 SD command。

实现提交：3fafc29488b2bfa01ac6785ae6534c0f443e117b。

验证：
- [H700 AArch32/AArch64 compile check #45745912462](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45745912462) **success**。storage-host-regression job（包含 h700_fat_vfs_backend_test.c 与 ASan/UBSan 可用时的 sanitizer run）及 freestanding AArch64 objects job 均通过。
- 旧 full feasibility run [#45745912439](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45745912439) 因后续提交被 supersede/cancelled，不作为验证结果。更新后的 [#45746788491](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45746788491) **success**，完整 job 75/75 steps 成功；包含存储 host regression、AArch64 freestanding objects、完整 Frontend、15-member workspace、H700 CMD17/GPT/FAT/VFS、mGBA 静态归档与 newlib/VFS v3 构建、static API 实际链接和 BL33 layout probes。详情见第25节。

本节只证明 host 模拟 CMD17/FAT media 上多句柄状态和边界；不证明并发 SMP 安全、真实 SD/MMC 读性能、target LK callback threading，也不实现持久化写入。当前 provider 继续要求 serialized callbacks；FAT32 8.3 writer 与 exFAT flush/rename 差距仍不允许向 Linkslot 开放存档写回。

## 25. 静态 mGBA retro_init() 前安装 VFS v3 的生命周期验证（2026-11-26）

继续沿 VFS v3 → static Libretro API → mGBA 的启动顺序推进，新增真实跨 C/Rust callback 的 host lifecycle probe：

- platform/h700/runtime/tests/mgba_static_stub.c 现在提供完整、静态存活的 fake linkslot_vfs_backend，通过 Rust 导出的 linkslot_vfs_install() 安装；C stub 保存 Linkslot environment callback。
- 当测试版 mGBA retro_init() 执行时，主动调用 GET_VFS_INTERFACE，请求 VFS version 3，并记录是否成功取得非空 interface table。
- scripts/test-linkslot-static-mgba-selection.py 在 open_core() 前安装 backend，并要求 static mGBA 初始化后 vfs_request_ok() 为 true；同时继续检查 init/deinit 次数与显式 SLOT_CORE override 不发生意外 static fallback。
- 该测试将真实验证顺序：C-owned VFS backend install → Linkslot static core API 设置 environment callback → retro_init() → Libretro VFS v3 negotiation。它仍使用 fake backend，不接触真实 SD 卡。

代码：
- C static mGBA lifecycle/VFS stub 与安装顺序断言：f438598a1940e571f9102cb70f16453c6372bbd0
- 修正 static selection test harness：dd4c6898f9db16249547d49023b60a11d3a40eff

验证状态：
- H700 AArch32/AArch64 compile check [#45746324107](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45746324107) **success**；storage host regression 与 freestanding AArch64 objects 均通过。
- 首轮完整 feasibility [#45746324138](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45746324138) 的 VFS v3 slot-retro host suite **63/63 passed**。随后 static mGBA selection test harness 因旧逻辑试图重复注入 linkslot_mgba_stub_reset_counters() 而失败；这是测试脚本与新 C stub 的重复注入冲突，不是 Cargo/C ABI/VFS negotiation 失败。该首轮 run 不记为 success。
- 已将脚本改为验证 reset/install/request 三个 stub helper 必须存在，不再注入或改写 stub。修正后的全量 run [#45746788491](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45746788491) **success**，75/75 steps 成功；第35步 static mGBA selection test 通过，mGBA retro_init() 中的 GET_VFS_INTERFACE(version=3) 请求成功并返回非空 interface table。其后 mGBA VFS v3/newlib、真实静态归档绑定与链接、AArch32 LK-as-BL33 layout probe 和 artifact upload 均成功。

本阶段目标是验证软件层 VFS/core 初始化顺序；即使该步骤通过，也不代表可启动 Linkslot LK App。mGBA C stub 没有执行 ROM，VFS provider 仍只读；无实机 SD 访问、无写卡、无 Boot0/TOC1 操作。完整 App 仍未接入可启动 LK main loop，display/scanout、GPIO/ADC、PCM、真实定时器、内存/heap 资格化及 core payload handoff 仍是独立门槛。


产物：[Linkslot LK feasibility artifacts #45746788491](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45746788491/artifacts/15083782750)（约 387.9 MB，SHA-256 `fe734e1cd1e04ab993b73efa6c96cdeccba5d3b861d7f9b2689dfdcafc089c88`，保留至 2026-12-26）。

## 26. H700 runtime service descriptor 与有序启动协调器（2026-11-26）

在第23–25节的 App phase routing、只读 VFS 多句柄和 mGBA VFS-before-retro_init() 验证基础上，新增 slot-lk-runtime 的第一阶段 startup coordinator：

- 新增 C ABI h700_vfs_service_v1_t 与聚合 h700_runtime_services_v1_t（ABI version 1），分别描述 VFS installer 与 time/input/VFS 三张 service table；Rust 对应 H700VfsServiceV1 / H700RuntimeServicesV1。
- 新增 H700RuntimeSession::bootstrap()，顺序固定为：验证聚合 descriptor 与 VFS 版本/size/callback → 构造并验证 time/input event loop → 调用 VFS install callback → 最后调用 open_static_core closure。VFS install 失败时绝不调用 core opener；time/input validation 失败时不会产生 VFS install 副作用。
- H700RuntimeSession 持有 event loop 与 core handle；实际 core handle 必须提供 RAII deinit/drop 语义。测试覆盖 VFS install 顺序、install fail-closed、time/input invalid、aggregate/VFS ABI version/size invalid、missing installer、core open error，以及 session drop 触发 core handle teardown。
- VFS installer callback 契约要求幂等：如果同一进程级 backend 已经安装，应返回 true；底层 VFS registry 是 process-lifetime、不可随单个 core teardown 卸载。Core open 失败后 VFS 仍保持安装，opener 必须清理任何部分初始化的 core。
- 新增 platform/h700/runtime/vfs_service.h 与 runtime_services.h；扩展 scripts/test-h700-input-service-abi.py，用 C _Static_assert 在 AArch32/AArch64 交叉目标核对 time/input/VFS/aggregate descriptor 的大小和字段偏移。
- H700EventLoop::new() 会分配有界 gesture action buffer，因此 caller 必须先完成 LK heap 初始化和资格化；本 coordinator 不初始化 heap，不实现 timer/GPIO/input scanner，也不执行 MMIO。
- 当前 time_service.h、input_service.h 是注入接口；input_queue.c 仍明确是 single-context、非 ISR-safe。lk_bindings.c 所需 platform heap/secondary CPU/console/RTC/fatal callbacks 仍为外部实现，不得用猜测的 LK symbol/MMIO 填充。

提交：
- Runtime descriptor/bootstrap、C headers、ABI probes、workflow path filters：9c771f6f84d972b8d3c93f3c3997404c2c559779
- 修正 Rust 泛型测试推断和 C -Werror unused helper：ae3b1f4183159f497f0844e2541cbdb614c1b333

验证：
- [H700 Linkslot 1.0.49 runtime slices #45748496564](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45748496564) **success**。C input/runtime service ABI + queue job 与 15-member no_std workspace、default/no-default 测试、AArch32/AArch64 no-default checks、完整 H700 SoftwareCompositor Frontend build、QuickRow fail-closed regression 全部通过。
- [Linkslot LK Rust feasibility #45748496582](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45748496582) **success**，完整 job 75/75 steps 成功、零失败。包括 H700 BL33/renderer/frontend、15-member workspace、CMD17/GPT/FAT32/只读 VFS、VFS-before-mGBA retro_init() host test、mGBA VFS v3/newlib 构建、真实 AArch32 static archive 经 Linkslot Rust static API 表链接、AArch32 LK-as-BL33 layout probe 与 artifact upload。

产物：[Linkslot LK feasibility artifacts #45748496582](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45748496582/artifacts/15084841735)（约 387.9 MB，SHA-256 2f1e9a94e1e291df1d5a162e7afcc8af6080ed9583f7521ba9c45aa89cf88816，保留至 2026-12-26）。

当前结论：**runtime bootstrap 的 C/Rust ABI、验证顺序和失败边界已通过 host/cross-build；它仍是 generic coordinator，不是完整 LK 启动实现。** 还没有实际 H700 monotonic timer、GPIO/ADC scanner、LK heap provider、display/scanout、PCM provider 或可运行 slot::App 的 LK main-loop binding；没有执行游戏 ROM、没有真实 SD/MMC 访问、没有存档写回，也没有 Boot0/TOC1/SD 写卡。

下一步先从 pinned GammaOS LK 源码确认真实 monotonic timer、GPIO/ADC 输入与 heap 生命周期 API，再逐项建立 C-owned platform providers 和 target host-independent tests；只有服务证据、内存 arena/stack/BSS 预算和 secondary-core fail-closed gate 齐备后，才把 coordinator 接到 LK main。持续保持 VFS read-only。


## 27. Pinned GammaOS LK kernel timer contract audit（2026-11-26）

固定源码：[`TheGammaSqueeze/lk-mt6785`](https://github.com/TheGammaSqueeze/lk-mt6785/tree/9bf1419a49e0af6dba8b05916df382c9c9c6d0a8)，commit `9bf1419a49e0af6dba8b05916df382c9c9c6d0a8`。

- `kernel/main.c` 的正常启动顺序为 `arch_early_init()` → `platform_early_init()` → `target_early_init()` → `heap_init()` → `thread_init()` → `dpc_init()` → `timer_init()`。随后 bootstrap thread 才调用 `platform_init()` → `target_init()` → `apps_init()`。因此 LK scheduler timer 是 Linkslot coordinator 之前的内核基础设施。
- `kernel/timer.c::timer_init()` 无条件注册 `platform_set_periodic_timer(timer_tick, NULL, 10)`（特定 Konkr 配置为 1 ms）。`timer_set()` 还通过 `current_time()` 计算到期时刻。没有可工作的 H700 platform timer 时，不能把该阶段视为可启动。
- pinned `include/platform/timer.h` 声明 `platform_set_periodic_timer()`、`mdelay()`、`udelay()` 和 `platform_tick_rate()`；`include/platform.h` 声明 `current_time()` / `current_time_hires()`。H700 当前 `h700_arch_timer.c` 只提供独立 ABI v1 的只读 `CNTVCT/CNTFRQ` monotonic service，不是这些 LK platform hooks 的实现，也不触发 scheduler tick。
- pinned `platform/mt6785/timer.c` 的周期计时器使用 GPT5、MTK GPT 寄存器、MTK IRQ/GIC helper；H700 overlay 排除 MTK platform/PAL。禁止将该源文件或 GPT5/MMIO 常量移植进 H700。
- `target_init()` 在 `timer_init()` 之后执行；现有 heap readiness marker 只能在 `target_init()` 后检查 heap，不能用于初始化 LK 首轮 scheduler timer。
- 当前 H700 object list 没有实现 LK periodic timer hooks；完整 LK target link 仍被 memory-layout approval gate 阻止。因此不能声称 LK scheduler timer 已链接或可运行，不能增加返回成功的空 stub 来绕过 `timer_init()`。

- 更早的必需阶段：`kernel/main.c` 在 `arch_early_init()` 后、`target_early_init()` 和 `heap_init()` 前调用 `platform_early_init()`。pinned `platform/init.c` 的通用弱实现为空；MTK 参考 `platform/mt6785/platform.c::platform_early_init()` 则先调用 `platform_init_interrupts()`，再调用 `platform_early_init_timer()`。当前 H700 没有自己的强 `platform_early_init()` / GIC provider，因此不能把 generic weak fallback 当成已完成的早期初始化。
- MTK 参考 `platform/mt6785/timer.c` 以 GPT4 建立早期自由运行 delay timebase，再以 GPT5 + MTK IRQ path 注册周期 tick；`dev/gic/mt_gic_v2.c` 依赖 MTK `GIC_CPU_BASE/GIC_DIST_BASE`、`mt_irq_get_target()` 和 MTK irq headers。pinned `dev/gic/rules.mk` 通过 `MTK_GIC_VERSION` 选择该驱动，H700 overlay 明确排除 `dev/gic` 模块。不得把这套 GIC/GPT 初始化移植为 Allwinner H700 实现。
- reports 的 Linux `interrupts.txt` 同时显示 `GIC-0 27 arch_timer` 与 `wakeupgen 48 sun4i_timer0`；这是 Linux 运行态映射，不是 BL31→BL33 handoff 或 LK 可用 PPI 的证据。当前归档的 `live-fdt.json` / `dtb-report.json` 是摘要格式，没有 interrupt-controller 节点；原始 live DTB 未上传，因此尚不能从仓库确认 GIC compatible、寄存器范围、#interrupt-cells 和实际 timer PPI。
- 新增 `scripts/audit-h700-lk-timer-prerequisites.py`，CI 将固定检查 LK 启动顺序、弱 early-init fallback、MTK GIC/GPT 的平台依赖和 H700 overlay 排除项。该脚本是源码契约审计，不是硬件资格验证，也不代表 H700 timer 已实现。

**下一步门槛：** 在任何 H700 timer writer/IRQ 实现前，先取得 RG34XX 原始 live FDT / `/proc/device-tree` 的 interrupt-controller compatible、reg、interrupt-cells 与 timer 描述，再通过独立 LK 只读诊断核实 BL31→BL33 时的 EL2/Generic Timer counter access controls、`CNTFRQ/CNTVCT` 可读性与持续增长，以及 GIC distributor/CPU-interface handoff 和 architected timer PPI ownership；GIC 版本与地址仍不得推测。Linux 中 IRQ 27 的运行计数只证明 Linux arch_timer 工作，不证明 BL33 可直接启用同一 PPI。只有这些证据闭合后，才实现 `current_time()` 与 LK `platform_set_periodic_timer()`/delay API 的 H700 provider，并通过静态链接与受控实机诊断验证。不得猜 PPI/IRQ、改写 EL2 控制寄存器或开启未知 timer IRQ。

当前保持：`h700_arch_timer_init()` 不进入 `target_init()`；不连接 `timer_init()`；不启用 timer compare、GIC/PPI 或周期 IRQ；输入 PIO sampler 仍不接 normal runtime。

## 28. RG34XX live FDT interrupt-controller / timer extraction（2026-11-26）

- `tools/inspect_h700_fdt.py` 新增 `interrupt_controllers` 与 `timer_nodes` 输出：保留 compatible/status、phandle、父总线地址/size cell 解码后的 `reg` ranges、`#interrupt-cells`、继承的 `interrupt-parent`、原始 `interrupts` cells 分组及 `interrupts-extended` phandle 引用。
- specifier 仅依 provider 的 `#interrupt-cells` 分组；不把 cell 含义解释为 Linux virq、GIC SPI/PPI 编号，也不推断 BL33 IRQ ownership。输出明确标注静态 DT 声明不证明运行态 handoff。
- 合成 FDT 回归覆盖 GIC compatible/reg/#interrupt-cells/phandle、祖先继承的 interrupt-parent、timer interrupts 与 interrupts-extended。H700 boot-audit #45772025184 Linux host-tools 6/6、Windows 8/8 成功；H700 freestanding compile #45772025265 AArch64/AArch32 ARM/Thumb-2 8/8，storage host regression 5/5。
- reports 分支的 `reports/rg34xx-inventory-20261125-163152/` 只保留旧 `live-fdt.json` 摘要，没有原始 `live-fdt.dtb`；旧 JSON 无法补回未导出的 interrupt-controller 节点。本次 parser 尚未在新的设备原始 FDT 上运行。
- **下一步：** 用更新后的 `scripts/run-rg34xx-inventory.ps1` 对 RG34XX 做一次完整只读采集（从 `/sys/firmware/fdt` 捕获 raw DTB），将新的 `live-fdt.dtb` 和 `live-fdt.json` 上传 reports 分支；随后基于真实 compatible/reg/#interrupt-cells/timer declarations 核对 GIC 类型与资源窗口。仍需独立证明 BL31→BL33 counter access 与 timer PPI ownership；Linux IRQ 27 不能代替该证据。


## 2026-11-26 RG34XX live GICv2 / architected timer mapping

报告目录：reports/rg34xx-inventory-20261126-timer/。本轮完整只读采集包含 live-fdt.dtb（143,936 bytes）与 live-fdt.json（573 nodes）。SoC compatible 为 allwinner,h616 + arm,sun50iw9p1。原始 DTB 在发布前只对 chosen bootargs 的两处设备序列号作等长 19-byte 替换；解析结构与所有其他属性保持不变。发布 DTB SHA-256：620b36d84fb7e714dcb56891f50802048d83c64ca924222a4a566f340ac7242e。详见 REDACTION.md 和 SHA256SUMS.txt。

**主中断控制器（设备树静态事实）**
- /interrupt-controller@03020000：compatible arm,cortex-a15-gic + arm,cortex-a9-gic，status okay，phandle 0x8e，#interrupt-cells=3。
- 四个 reg 区域按 ARM GIC v1/v2 binding 顺序为 GICD 0x03021000/0x1000、GICC 0x03022000/0x2000、GICH 0x03024000/0x2000、GICV 0x03026000/0x2000。它们全部位于现有 H700 Device-memory mapping [0x03000000,0x03100000) 内；页表覆盖不等于安全访问权或中断 group ownership。
- GIC 节点 interrupts=<1 9 0xf04> 指向 PPI relative 9 / GIC INTID 25，为 VGIC maintenance interrupt 声明，不是 scheduler tick。

**Architected Timer PPI 映射**
- /timer_arch compatible arm,armv8-timer，interrupt-parent 指向主 GIC。四个原始 specifier 为 <1 13 0xf08>、<1 14 0xf08>、<1 11 0xf08>、<1 10 0xf08>。
- 按 Linux architected-timer binding 顺序分别为 EL1 Secure physical / EL1 Non-secure physical / EL1 Virtual / EL2 physical；GIC INTID 分别 29 / 30 / 27 / 26。0xf08 的 CPU mask 是 0xF（CPU0–CPU3），flags 0x8 表示 active-low level-sensitive。
- Linux /proc/interrupts 的 GIC-0 27 Level arch_timer 在四个 CPU 均有计数（65554 / 21050 / 28197 / 8522），与 EL1 Virtual Timer PPI 11 → GIC INTID 27 对应。这证明 Linux 运行态路径，不证明未来 LK BL33 的 PPI ownership。
- 独立 Allwinner timer 节点 /soc@03000000/timer@03009000：compatible allwinner,sun4i-a10-timer，reg 0x03009000/0x400，父中断控制器 allwinner,sunxi-wakeupgen，specifier <0 48 4>；Linux /proc/interrupts 对应 wakeupgen 48 Level sun4i_timer0。不要与 architected GIC timer 混淆。

**BL31→A32 BL33 入口状态补充**
- reports/rg34xx-active-loader-provenance.md 对当前 BaseOS 链的首轮 BL31→BL33 ERET 记录为 entry=0x4a000000、SPSR=0x1d3，即预期恢复到 AArch32 SVC、ARM state，IRQ/FIQ masked。若 LK 替换当前 A32 loader 且保留相同入口契约，应在第一条 wrapper 指令读取 CPSR 验证这一预期。
- SPSR=0x3c5 是 A32 loader 后续执行 vendor Linux-start SMC 后进入 AArch64 Linux 的状态，不是 LK 初始 BL33 状态。0x1d3 仍来自现有 loader/monitor 的源码/仿真审计模型，不是替换 LK 的实机寄存器采样；Secure/Non-secure world 也不能单由该 SPSR 推断，仍须独立确认。

**被替代的旧结论与下一道门槛**
- 本节替代前文“原始 live DTB 未上传、GIC compatible/reg/PPI 尚无法确认”的旧结论。现在硬件身份、寄存器窗口、PPI 声明已由 live FDT 闭合；BL31→LK 的权限与中断交接仍未闭合。
- pinned LK arch/arm/exceptions.S 的 arm_irq 会调用 platform_irq()，非零返回后 thread_preempt()；H700 目前没有 platform_irq()/platform_fiq() 或 GICv2 provider。pinned dev/gic/mt_gic_v2.c 是 MTK 专用实现，依赖 MTK IRQ helpers / 寄存器，不可复用。
- 下一步只做 LK 目标态只读诊断：记录实际 CPSR/异常模式与安全态；读取 CNTFRQ 并两次读取 CNTVCT 验证单调增长；读取 GICD/GICC/GICH/GICV ID/control 状态；确认 Group0/Group1NS ownership、PPI 27 是否交给 BL33、EL2 counter/virtual timer traps 与 CNTVOFF 状态。CNTHCTL_EL2 的 EL1TVCT/EL1TVT 访问控制受处理器特性约束，不能根据 FDT 或 Linux IRQ 计数假定访问放行。
- 这些状态闭合后，才实现 H700 strong platform_early_init()、GICv2 platform_irq() IAR/EOI、LK current_time()/delay/platform_set_periodic_timer()；初始仍限制单核，不更改安全组、不清空无关中断、不提前写 timer/GIC/PPI。
- 完整 feasibility #45775116287 在 03fbc6c 上 76/76 成功；它仍是 host/cross-build 和 source-contract 验证，不是完整 LK target link 或实机启动。

规范参考：Linux ARM GIC v1/v2 binding https://github.com/torvalds/linux/blob/master/Documentation/devicetree/bindings/interrupt-controller/arm,gic.yaml；Linux architected timer binding https://github.com/torvalds/linux/blob/master/Documentation/devicetree/bindings/timer/arm,arch_timer.yaml；Arm CNTHCTL_EL2 https://documentation-service.arm.com/static/60df1943677cf7536a55ddef。


## 2026-11-26 A32 BL33 first-instruction snapshot（FIQ-safe）

- 新增 h700_bl33_entry_snapshot 五字 BSS 记录和 C-only 只读 getter，保存 BL31→LK 第一条 wrapper 指令时的 SP、LR、r4、CPSR；magic 0x48374553 最后写入，getter 对空指针与未发布状态 fail-closed。
- 入口采集寄存器采用共享的 AArch32 r4-r7：CPSR→r5、SP→r6、LR→r7，r4 保持原始 loader 参数。原因是 FIQ 模式下 r8-r14 为 banked registers；此前 r8-r11 的初稿可能在 CRT0 切换到 SVC 后无法读取。overlay 同时把原 CRT0 的 BOOT_ARGUMENT_LOCATION 临时地址寄存器 r6 改成 r0，把可选 TrustZone linker-retention load 的 r7 改成 r0，避免覆盖入口快照，原 r4 参数保存行为不变。
- 原始值先留在 r4-r7，经过 CRT0 relocation 与异常模式栈配置；.L__bss_loop 完成后、调用 kmain() 前，写入 BSS 五字结构，magic 最后写入。若启动在此之前 fault，BSS 快照不会生成；它不是 pre-BSS crash logger。
- 已接入 H700 平台真实 OBJS 表、AArch64/AArch32 ARM-state/Thumb-2 freestanding compile 列表；新增 C ABI host regression、BL33 overlay capture/ordering/FIQ-safe assertions、linker BSS bounds 检查及 project guard。A32 layout audit probe 同步采用相同寄存器分配与 CRT0 scratch relocation。
- 当前 FIQ-safe 源码快照（aa05e7b）对应 H700 AArch32/AArch64 编译与 host 回归 [#45779682341](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45779682341) 已完整 success：storage-host-regression 和 compile-freestanding-objects 两个 job 均通过；对象覆盖 AArch64、AArch32 ARM-state、AArch32 Thumb-2。
- 完整 feasibility [#45779682226](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45779682226) 最终 **76/76 steps success**，无失败步骤。除 timer source audit、H700 BL33 overlay generation、真实 CRT0/exception assembly compile+link、native header packaging、H700 project/memory fail-closed guard 外，Linkslot/no_std、mGBA AArch32 archive、VFS v3/newlib、Rust static API link 与最后 A32 BL33 layout probe 全部成功。产物 [linkslot-lk-rust-feasibility-45779682226](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45779682226/artifacts/15098435220)，Artifact ID 15098435220，387,919,929 bytes。
- 最新工作分支 HEAD 的 boot-audit [#45780174078](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45780174078) Linux host-tools 6/6、Windows PowerShell 8/8 成功。完整 feasibility 使用 aa05e7b 代码快照；之后至当前 HEAD 2bccc2b 的差异是文档状态记录，不含运行时代码变化。不得把 host/cross-build 结果描述为完整 LK target build 或设备启动。

- A32 layout probe 结果：ELF32/ARM、入口 0x4a000000、向量表 0x4a001000、BL33 wrapper 0x4a001024、boot argument scratch 0x4a001020、快照 BSS 0x4a003428（20 bytes），链接 _end 0x4a00343c，flat image 5,148 bytes；8 条异常向量分支解析正确且无 unresolved symbols。这是最小 CRT0/exception + stub 的 host layout probe，不是完整 LK 映像，0x4a000000..0x4a0e0000 仍不是内存授权。
- 该变更不读取 GIC/Timer/EL2 控制寄存器，不写 MMIO，也未执行设备 payload。CPSR mode bits 不能证明 Secure/Non-secure world、EL2 traps、CNTVOFF 或 GIC Group ownership。当前仍未完整 link/boot LK；H700 memory-ownership gate 保持关闭。
- **下一门槛：** FIQ-safe overlay、A32 layout、H700 AArch64/AArch32/Thumb-2 编译、host 回归、完整 feasibility 76/76、Linux/Windows boot-audit 均已通过。当前推进转向 BL33 内存与固件生命周期审计：已由 pinned BaseOS source 证明 native SPL 直接使用 0x42e00000 包缓冲、0x45000000 BOL4 staging、0x48000000 BL31、0x4a000000 loader、0x4a300000 handoff record；它没有显式写入候选 [0x40080000,0x42000000)。但后续 A32 Linux loader 会把该候选区用于 kernel 解压与 initrd，而 vendor BL31 内部 scratch/secure/DMA 访问仍未知。不得据此批准该区。下一步需单独审计精确配对的 BL31 二进制/secure ownership，并形成 LK code/BSS/stack/page-table/heap 与独立 Linkslot payload 的完整 non-overlap map；在此之前 H700_MEMORY_LAYOUT_APPROVED 保持 unset，不做设备 payload 执行或 GIC/Timer 写入。


## 2026-11-26 Native SPL / A32 loader memory lifetime source trace

- pinned BaseOS-MultiFrontend cbe253a43f46cf551cc9a6c79a84015bde3686cf 的 spl/native-boot.c：native SPL 将 p4+40 MiB firmware package 读到 0x42e00000，按 extent 将 monitor / DTBO / DTB / A32 loader 分别复制至已审计目标，再将 16 MiB BOL4 image 读到 0x45000000，发布 240-byte record 到 0x4a300000，最终把 RVBAR 指向 0x48000000 并触发 EL3 reset。审阅的 native SPL handoff 路径没有显式访问候选 [0x40080000,0x42000000)。
- 同一 pinned source 的 native/entry32.S 将 A32 loader SP 设为 0x4a0f0000，页表为 [0x4a0f8000,0x4a0fc000)。后续 native/loader.c 把 Linux kernel 解压到 [0x40080000,0x42000000)，最大输出 0x01f80000；initrd 从 0x42000000 开始，FDT 位于 0x44000000。故该候选只可能在替换 BL33、完全绕过后续 Linux loader 时被重新审议。
- **证据边界：** “pinned native SPL 没有显式写入该候选区”不等于“BL31 未使用该区域”。精确 monitor 内部 scratch、secure-world、DMA 与 cache alias ownership 仍未审计；Linux memblock map 也不能完整描述 pre-Linux BL31 生命周期。详情见 [h700-bl33-memory-ownership.md](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/porting/linkslot-1.0.45-feasibility/docs/h700-bl33-memory-ownership.md)。
- 该区仍是 candidate only；native SPL、BL31、BL33、Linux loader 各阶段 ownership 不可混为一谈。当前不填入 MEMBASE/MEMSIZE、不打开 H700_MEMORY_LAYOUT_APPROVED，不推断 16 MiB 0x7ec00000 reservation 属于 DRM/TEE，也不对该区域进行硬件探测。


## 2026-11-26 距离首次实机启动的分级评估

这里把“首次实机启动”拆为三个可验收里程碑，避免把 A32 host linker probe 误当成设备已可启动：

### A. BL33 入口诊断（只证明控制权进入新 payload，不等于完整 LK）

当前 p4+40 MiB native firmware package 中的 loader extent 是 32,768 bytes（来自 reports/rg34xx-native-firmware/firmware-manifest.json）。Pinned BaseOS native/build.py 会把 loader blob 按 512-byte 对齐后作为 firmware extent 写入 package，重新计算 firmware 长度/FNV，并生成 native-generated.h 中的 BASEOS_FIRMWARE_BYTES / BASEOS_FIRMWARE_FNV 与 extent table；总 package 有 1 MiB 上限。当前 scripts/package-h700-bl33.py 只负责将 raw A32 LK 与 SHA-pinned header 组合，**尚未把新 LK payload 接入 BaseOS native SPL/package 构建链**。即使入口诊断 payload 保持在现有 32 KiB extent 内，内容改变也会改变 package FNV，必须生成匹配的 native firmware package/SPL；若 payload 超过 32 KiB，还需同步扩大 loader extent 并重新核对 package 上限与 BL33 区域布局。

因此最短的实机入口证明不是“直接烧录当前 host 产物”，而是：先制作尺寸受控的 A32 diagnostic payload（或最小 LK image），接入配对构建流程，保留真实 vendor header，完成 BL33/stack/BSS 不重叠审查、可恢复安装方案，再在设备上验证入口快照/明确的 proof-of-life 信号。该里程碑尚未开始实机执行。

### B. 首次完整 LK kernel 启动（进入 kernel lifecycle / idle）

除 A 外，还必须具备经过审查的 MEMBASE/MEMSIZE、LK image/BSS/stack/heap/page-table 的完整布局，以及 H700 platform early init。pinned LK 的正常启动路径会注册 scheduler periodic tick；H700 目前仍缺少可用于正常 kernel lifecycle 的 GICv2 provider / platform_irq()、architected-timer 驱动和 LK current_time()/platform_set_periodic_timer()。GICv2 地址/PPI 已由 live FDT 识别，但 BL31 handoff 后的 Secure/Non-secure、Group0/Group1NS、EL2 trap 与 PPI ownership 未证明。第一轮应保持单核，不能用 Linux 中断计数代替 LK 权限证明。

### C. 首次进入 Linkslot / mGBA 前端

这还需要单独批准 Linkslot app/payload 的 DRAM arena，并验证 payload loader、cache/MMU 属性、独立 heap/stack、read-only SD → GPT/FAT/exFAT → Libretro VFS、framebuffer/display/input/audio 等真实板级路径。mGBA AArch32 archive 约 1.78 MiB，大于当前 896 KiB BL33 packaging policy，因此 Linkslot+mGBA 必须作为 BL33 之外的独立 payload，不应塞进 LK BL33 image。当前候选 [0x40080000,0x42000000) 仍未获 BL31 scratch/secure/DMA 生命周期审查，不得分配。

**当前判定：** A32 BL33 host 原型与 76/76 feasibility 已完成；实机入口诊断仍被“匹配 SPL/firmware 集成 + BL33 运行内存布局 + 可恢复部署流程”阻塞。完整 LK kernel 还需 GIC/Timer early platform；Linkslot 前端则再增加独立应用内存与设备驱动验证。H700_MEMORY_LAYOUT_APPROVED 保持 unset。不要以当前阶段的 CI 通过率折算实机启动完成度。


## 2026-11-26 Native firmware extent 与 LK override 构建接入

- 已核对 reports/rg34xx-native-firmware/firmware-manifest.json：部署中的 p4+40 MiB native firmware package 为 287,232 bytes；loader extent offset 0x3e200、长度 0x8000（32 KiB）、目标 0x4a000000。这个 extent 比 BL33 packager 的 0xe0000 上限严格得多。
- 新增 scripts/patch-baseos-native-lk-override.py：要求 BaseOS-MultiFrontend 精确 pinned at cbe253a43f46cf551cc9a6c79a84015bde3686cf，且 native/build.py 原本干净；生成一个带备份的本地源码补丁，通过 BASEOS_LK_BL33_RAW 接入 raw header-free A32 image。严格校验 A32 entry branch、8 个向量分支、保留头区零填充、payload ≤32 KiB；由 BaseOS builder复制配对头区并填满原 32 KiB extent，不改变 extent offsets/table。
- 原始 LK binary 被追加到 native/build.py 的 generation inputs / persistent_source_hashes；native builder随后重新生成 loader.bin、firmware.bin、BASEOS_FIRMWARE_FNV、native-generated.h 与 manifest，build-early-mmc 使用匹配 header 构建 native SPL。避免“只替换 firmware.bin 却漏改 SPL FNV”这一配对错误。
- 新增 scripts/test-baseos-native-lk-override.py，覆盖有效 raw image、超 32 KiB、缺失 vector、非零 header、错误 entry branch、patch transformation、输入哈希接入、幂等及 template ELF/map 改名晚于原 map 检查的顺序。首轮 fixture 因未镜像 pinned manifest 缩进失败，已修正。当前完整 feasibility [#45785521254](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45785521254) 的 BaseOS 原生验证步骤已全部通过：pinned checkout、patcher host regression、对真实 native/build.py 执行 apply/check/AST parse/restore，随后 git diff 与 status 均确认工作树恢复干净。完整 workflow 其余 Linkslot/mGBA 步骤仍在执行；这证明补丁可接入并恢复上游源码，不代表已完成 LK image/full SPL release build，也不允许刷写。
- Patcher 输出中的 native-loader-template.elf/.map 明确属于被替换的 BaseOS loader 模板，不代表 LK 最终镜像；原 loader.map 的 trampoline 检查先执行，随后才改名。LK raw image 单独保存在 lk-bl33-override.raw，必须以单独 LK target ELF/map 做内存审计。`--restore` 已改成备份字节原样写回，避免文本换行规范化；CI 对 pinned BaseOS 源码执行 apply/check/restore 后已确认 diff/status clean。
- 该路径只支持当前 extent 内的紧凑型 proof-of-life stub；目前 5,148-byte layout probe 尚无可见硬件 proof-of-life 信号，也不是完整 LK。完整 LK / Linkslot / mGBA 不能假设可塞入 32 KiB；更大的 loader extent 需要单独重构与配对固件审核。无设备写入/无 payload 执行。


## 2026-11-26 H700 DRAM provider linked into LK and BL31 memory triage
### H700 physical DRAM provider

- Added `h700_physical_memory.o` to the real H700 LK `OBJS` list. The pinned
  GammaOS LK `arch/arm/mmu.c` uses the weak ABI
  `uint64_t physical_memory_size(void)`; the RG34XX provider reports
  `0x40000000` (1 GiB).
- Fixed the Make continuation in `platform/h700/rules.mk`: the provider line
  ends in exactly one continuation backslash. The earlier doubled
  backslash was caught by the full LK project-guard dry-run
  (`recipe commences before first target`) and corrected in commit
  `a92a9e502428728c2f3ca7a321b2851b3ecc3b12`.
- Latest H700 workflow
  [#45795119838](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45795119838)
  passed both jobs. It compiled and architecture-checked
  `h700_physical_memory.o` under AArch64, AArch32 ARM-state and AArch32
  Thumb-2; the storage-host-regression job explicitly passed the 1 GiB
  provider test and read-only SD/GPT/FAT/VFS host regressions.
- The 1 GiB value is the physical DRAM capacity ABI only. It does not approve
  allocation of all DRAM or the proposed Linkslot arena.

### Matched BL31 static memory audit

- Verified the actual monitor component against both the TOC1 inventory and
  native firmware extent manifest: 107,216 bytes, SHA-256
  `676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31`,
  loaded at `0x48000000`; its first entry branch targets `0x480000f8`,
  payload end exclusive `0x4801a2d0`.
- Control-flow-aware AArch64 traversal from the entry decoded 1,849 reachable
  instructions and recorded 268 recoverable address references: 188 into the
  payload and 80 into the immediately following runtime/page-table reference
  range `[0x4801a2d0,0x48200000)`. This is not a complete call graph:
  indirect calls/branches, exception returns and runtime-computed pointers are
  not fully modeled.
- Recovered EL3 state: `TTBR0_EL3 = 0x4803cb40` (write at
  `0x4800bf60`); TCR_EL3 configuration load from `0x4803c8e0`
  (`0x4800bf4c`); per-core TPIDR_EL3 context base `0x4803cd40`
  (`0x4801582c`).
- Decoded calls to helper `0x4800bcb4` request unconditional mappings within
  `[0x48000000,0x48041000)`; a second mapping request spans
  `[0x48200000,0x4a000000)` only when the 32-bit word at `0x48000018` is
  nonzero. A nested 4 MiB request is made under the same condition. The raw
  mapping attributes (1, 2, 3, 7) have not been assigned memory-type or
  security semantics.
- The page-table construction path references pool base `0x4803e000`,
  4 KiB page indexing and cursor/counter `0x4803cb80`; its local counter
  guard permits at most two page indices on that path. This is not a complete
  allocation map. The conditional word has not been qualified from live
  runtime state.
- No direct address reference into candidate `[0x40080000,0x42000000)` was
  recovered; this is **not** proof that the region is free. Keep the
  conservative BL31-associated envelope `[0x48000000,0x4a000000)` excluded
  pending runtime ownership, security/DMA, cache-alias and secondary-core
  review. The endpoint `0x4a000000` is the start of the current native BL33
  loader slot; raw map requests are not proof that the entire interval is
  physically allocated.
- Refined audit CI
  [#45795120056](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45795120056)
  passed and uploaded artifact
  [15104447110](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45795120056/artifacts/15104447110).
  No hardware MMIO, boot-media write, flash or payload execution occurred.


### 2026-11-26 Boot0 / BL31 single-core startup audit

- Added reproducible audit script
  `scripts/audit-rg34xx-boot0-cpu-reset.py` and expanded the BL31 scanner to
  classify CPUCFG/CPU subsystem MMIO references. The combined workflow
  [#45808401081](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45808401081)
  passed all steps on commit
  `fc5901ee49b502eab298a52b7e38add8f9611a66`.
- Primary Boot0 input: 128 KiB, SHA-256
  `bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f`,
  eGON checksum valid. Reset helper at PC `0x231e0`, caller at
  `0x23224`. Caller checks bit29 of `0x03006248`; if set it invokes the
  helper for core2 and core3, not core1. Helper clears the selected
  `C0_RST_CTRL` bit, which is reset-assert under H616 reference semantics.
- The gate address `0x03006248` lies inside the documented H616 SID window
  `[0x03006000,0x03007000)`, offset `0x248`; the public manual does not
  define this word or bit29. Treat it as an undocumented SID-window gate,
  not a known feature register. Do not collect a full SID dump; a future
  approved pre-BL33 probe should expose only the masked bit value.
- The pinned native SPL writes the primary RVBAR pair only. The original
  BL31 first-entry/direct-CFG scan recovered CPU-related MMIO at
  `0x09010010` (CPU0 mode path) and `0x08100020` (CPU subsystem control),
  but does not follow indirect platform-operation callbacks. The separate
  PSCI CPU_ON audit below confirms such an indirect callback can start cores
  later on request; these two audit results are complementary, not conflicting.
- H616 manual family-reference default for `C0_RST_CTRL` is `0x13ff0101`:
  low nibble `0x1` (CPU0 reset deasserted, CPU1-3 asserted). H700 live
  semantics/state still require a target readback; this is not a hardware
  measurement.
- Two discrepancies are explicitly unresolved: Boot0 ORs core2/core3 bits
  into `0x09010044`, which H616 documentation names CPU0 RVBARADDR0_H;
  Boot0 also writes `DBG_REG1.C_DBGPWRUPREQ`, which the H616 manual marks
  read-only. Do not infer these writes' H700 semantics or modify these fields.
- The CPU0-only handoff is now a **strong static hypothesis**, not an approved
  runtime guarantee. Keep SMP disabled and the single-core gate open until
  there is a trusted pre-BL33 read-only snapshot of `C0_RST_CTRL`,
  `C0_CPU_STATUS`, RVBARADDR0-3 and DBG_REG0/1, or an equivalent reviewed
  reset-hold proof. Linux userspace reads after SMP bring-up are not a
  substitute for a pre-BL33 snapshot.
- Detailed analysis:
  [h700-a53-single-core-startup-audit.md](h700-a53-single-core-startup-audit.md).
  Combined CI artifacts include the raw Boot0 capture and machine-readable
  CPU reset report.


### 2026-11-26 Matched BL31 PSCI CPU_ON callback audit

- Added scripts/audit-rg34xx-bl31-psci-cpu-on.py and host regression
  scripts/test-audit-rg34xx-bl31-psci-cpu-on.py. The exact matched
  107,216-byte monitor was verified by SHA-256
  676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31.
- Confirmed platform ops table at 0x480165f8, registration at
  0x480029bc, and dispatch via global psci_plat_pm_ops:
  dispatcher loads slot +8, then branches through the selected callback.
  Slot +8 points to 0x4800254c, which calls low-level CPU power-on
  routine 0x48002d78.
- The low-level routine writes a per-core RVBAR and contains SoC-specific
  CPU power/reset paths. One branch uses C0_RST_CTRL, C0_CTRL_REG0,
  AON power-on reset/clamp and DBG_REG0. The selector is SYS_CFG VER_REG
  at 0x03000024 (not SID); its target low3 value has not been read.
- Important selector mismatch: matched BL31 selects RVBAR bank 0x09010040
  when VER_REG[2:0]==0, otherwise 0x08100040. Pinned BaseOS native SPL
  selects 0x08100040 only when VER_REG[2:0]==2, otherwise 0x09010040.
  These rules agree only for values 0 and 2; they diverge for 1 and 3-7.
  The H616 manual documents VER_REG at SYS_CFG +0x24 but leaves [7:0]
  undefined; upstream H616 tools use the low byte as a die selector, but
  that does not establish the H700 value. Read only VER_REG & 7 in a trusted
  early environment and unify the SPL/BL31 rule after confirming target
  semantics.
- Workflow
  [#45816600012](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45816600012)
  passed all steps, including the exact-word/table/branch regression.
- Interpretation: BL31 has a statically confirmed PSCI platform CPU_ON
  callback. This is on-demand capability, not evidence that secondaries
  are automatically released during initial BL31-to-BL33 handoff. Initial
  CPU1-CPU3 reset state is still not device-verified; a diagnostic LK must
  not invoke PSCI CPU_ON before SMP/coherency and per-core initialization
  are deliberately implemented.
- Exact vendor source revision/callback ABI remains unconfirmed; the
  pwr_domain_on mapping is correlated using the standard TF-A
  plat_psci_ops_t slot order and binary registration/dispatch.
- Selector parity CI
  [#45817598323](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45817598323)
  verifies the differing SPL/BL31 rules and reports matching low3 values
  [0, 2] versus mismatching [1, 3, 4, 5, 6, 7]. It does not read the target.
- Added the single-register read-only collector
  `scripts/read-h700-rvbar-selector-readonly.py` and host regression. It
  reads only `0x03000024` using `/dev/mem O_RDONLY`, emits only
  `VER_REG & 7`, and performs no MMIO writes. Host tests cover all eight
  selector values; [#45818695832](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45818695832)
  passed.
- **Device attempt completed but produced no selector value** (reports
  commit `8c084fc21ca29b9afa341adca944c7d553417e29`):
  [raw report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-rvbar-selector-20261126).
  On Linux `4.9.170 #19`, the shell was root. The system had no
  `python3` on PATH, so the bundled CPython 3.12.16 was used; the script
  reached its read path and `os.pread(/dev/mem, 4, 0x03000024)` returned
  `EFAULT` (exit 2, stdout 0 bytes). Independent `dd` at the same offset
  returned the same error. The earlier PIO probe shows the same
  offset-independent failure at offset 0, one byte, and a full page.
  `CONFIG_DEVMEM=y`, `CONFIG_STRICT_DEVMEM` unset: the usual strict-devmem
  permission rejection is not the cause.
- The probe therefore establishes **no selector value**; it is not evidence
  for low3=0, nor any other value. Do not retry `/dev/mem` with alternate
  userspace tools. No module was loaded, no debugfs mount or boot-argument
  change was made, and no register/boot-media write occurred
  (`hardware_applied=0`). Debugfs had no SYS_CFG register-dump node and
  `debugfs/regmap` exposed only four unrelated devices.
- Next path: inspect a supported kernel-side mapping/read interface for
  SYS_CFG, or add a narrowly scoped read-only early-boot/BL33 diagnostic
  capture after reviewing its source and recovery path. A Linux-runtime
  VER_REG read may settle the static bank selector; only a pre-BL33 capture
  can establish CPU reset/RVBAR state at handoff. Do not change the SPL or
  BL31 selector rule until a valid target value and H700 semantics are
  established.
- Updated detailed report:
  [h700-a53-single-core-startup-audit.md](h700-a53-single-core-startup-audit.md).
  CI artifact 15114838248 contains the exact PSCI CPU_ON report and
  disassembly/audit output.


### Historical full feasibility result (74-step baseline)

<badge color="success">74/74 steps passed</badge>

- Full workflow
  [#45795119939](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45795119939)
  completed successfully on source commit
  `a92a9e502428728c2f3ca7a321b2851b3ecc3b12`. All 74 main steps passed;
  artifact `linkslot-lk-rust-feasibility-45795119939` (387,922,330 bytes)
  was uploaded successfully.
- The run passed the real H700 project/memory fail-closed Make dry-run,
  GammaOS LK H700 BL33 CRT0/exception-vector overlay link, BaseOS native
  override apply/restore and header packaging regressions, DRAM provider
  host test, no_std renderer/frontend and business-crate checks, input and
  Libretro VFS v3 tests, read-only SD -> GPT -> FAT32 -> VFS tests, FAT/exFAT
  regressions, mGBA AArch32 namespaced static archive and VFS v3/newlib builds,
  H700 newlib adapter tests, real mGBA export/Rust static-API binding and
  static-link probes, plus final AArch32 LK-as-BL33 entry/vector layout probe.
- The report is explicitly host-only cross-compile/static-link feasibility.
  The AArch32 LK-as-BL33 probe links the real patched GammaOS LK CRT0 and
  exception assembly with tiny platform/kmain stubs; it is 5,148 bytes, with
  entry wrapper at 0x4a001024, vectors at 0x4a001000 and linked _end at
  0x4a00343c. It is not the full LK kernel/platform, does not initialize H700
  GIC/CCU/GPIO/SD or console services, and is not a bootable image.
- The pinned LK baseline is uniprocessor: one global current_thread, one global
  set of run queues and one global critical_section_count. arch_disable_ints()
  masks IRQ on the current core only; it is not an SMP lock. The pinned CRT0
  sets ACTLR.SMP only for ARM_CPU_CORTEX_A7 and has no Cortex-A53 branch.
  The new Boot0 + native SPL + matched BL31 static audit strongly supports
  a CPU0-only handoff, but secondary CPU1-CPU3 hold-off remains PROBABLE and
  not device-verified. Boot0's helper is gated by bit29 of 0x03006248 and,
  when called, applies C0_RST_CTRL &= ~(1 << core) for core2/core3; it is not
  called for core1. Under H616 reference semantics, zero asserts reset. The
  H700 live register state at BL33 entry is still unknown. Before any new
  payload execution, obtain an approved read-only pre-BL33 snapshot or an
  explicitly reviewed hold-off proof; otherwise implement and validate A53
  coherency, SMP scheduler state and cross-core locks. The heap compatibility
  probe also links only against a signature-compatible test stub, not the real
  LK heap implementation.
- No device, SD card, boot media, payload or hardware execution was involved.
  This is not yet a flashed image or first-device-boot result.
- Downloaded build report SHA-256:
  `a20b4c28d9e8eec92cae69ed93da2ce66edaebcf1dbab5bca78ce9855016473c`.
  Full workflow artifact ZIP SHA-256:
  `421fa40fb1f0349cc3824542b71fc014f60774d9eead49cdcf12444fc533d004`.

### Remaining gates

1. Finish paired BL31 runtime-lifetime/ownership review: conditional mapping
   flag, page-table allocation bounds, secure-world, DMA, cache aliases and
   secondary-core state.
2. Produce a complete LK/BL33/BL31/application memory map, including code,
   BSS, heap, stack, page tables, handoff records, framebuffer/PIO, SD/DMA and
   separate Linkslot/mGBA payload footprint.
3. Keep `H700_MEMORY_LAYOUT_APPROVED` unset; do not assign `MEMBASE/MEMSIZE`
   or execute a payload until ownership is actually demonstrated.
4. Only after memory and recovery gates are approved, implement a compact
   diagnostic payload within the active 32 KiB loader extent, with a reviewed
   deterministic proof-of-life signal, then rebuild the matching native
   firmware/SPL pair. The existing 5,148-byte layout probe is not that
   diagnostic and no image has been flashed or run on hardware.


**2026-11-26 LK Data Abort safety finding:** pinned GammaOS LK
`9bf1419a49e0af6dba8b05916df382c9c9c6d0a8` routes `arm_data_abort` to
`arm_data_abort_handler()`, which calls `exception_die(..., -8, ...)` and
halts permanently. The generic vector has no recoverable exception-return
trampoline. See [h700-lk-data-abort-recovery-audit.md](h700-lk-data-abort-recovery-audit.md).
Do not add bare volatile reads to LK startup or the input scanner. A narrowly
matched assembly recovery path, correct saved-context restoration and safe
result channel must be designed and CI-tested first; unrelated Data Aborts must
retain the fatal fallback.


**2026-11-26 probe-scoped Data Abort source implementation:** 已新增
h700_data_abort_recovery.c/.h 与固定地址的 h700_syscfg_probe.S。恢复条件包含
active cookie、精确 A32 fault-load LR_abt、DFAR、短描述符 DFSR 同步读白名单、
SVC 模式和 ARM 状态；排除异步 abort、写侧 fault、debug event 和未知 syndrome。
A32 vector 使用 r4 保存原始 fault frame、对齐 Abort C-call stack，再由
LDM^ trampoline 恢复 SPSR/GPR/PC；非匹配 fault 仍走原 fatal exception_die()。
诊断函数仅显式调用，不在任何启动路径中调用。当前只完成 source/host/assembler/link
验证目标，CI 正在运行；未执行实机寄存器读取，hardware_applied=0。专项细节见
[h700-lk-data-abort-recovery-audit.md](h700-lk-data-abort-recovery-audit.md)。


**异常嵌套保护补充：** H700 overlay 的 Data Abort vector 在保存 68-byte
异常帧前先执行 `cpsid if`，屏蔽 Abort mode 中的 IRQ/FIQ，防止早期复用栈
被嵌套处理器覆盖、或 DFSR/DFAR 在匹配前被其它异常改写。成功的 `LDM^`
异常返回从保存的 SPSR 恢复原中断状态；非匹配 abort 仍保持 fatal/halt。
host/source/link CI 继续验证；没有实机探测。


**2026-11-26 probe-scoped Data Abort 最终 CI 验收：**

- 完整 Linkslot/LK feasibility [#45834208280](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280)：**81/81 steps 成功、无失败、无跳过**；artifact [15123653120](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280/artifacts/15123653120)，387,946,289 bytes。
- H700 AArch64 / AArch32 ARM / AArch32 Thumb-2 freestanding compile 与 host regressions [#45834208274](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208274) 成功；H700 boot audit Linux + Windows [#45834060935](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834060935) 成功。
- pinned LK overlay 真实 CRT0 + exception-vector A32 ELF 链接通过。最终反汇编确认 Data Abort vector 首指令 `cpsid if`、保存 frame、r4 保留 frame base、Abort stack 8-byte 对齐、A32→Thumb handler 的 `BLX`、仅正匹配进入 `LDM^` return trampoline；Thumb matcher 对 A32 probe 的 `BLX` 及 probe 的单条 `LDR` 均已解析。SYS_CFG probe 位于独立显式 API，**没有默认调用点**。
- ELF 为用于布局/符号验证的最小 overlay test image（5,804 bytes，`_end=0x4a0036d0`），含测试 stub；不是完整 LK kernel、不是可刷写固件、也不是设备启动结果。
- 下一门槛不是继续尝试 Linux `/dev/mem`：需单独审查最小诊断 payload 的 vendor header/package 配对、BL33 32 KiB loader extent、BSS/stack ownership、安装回滚方式和实机 proof-of-life；只有这些门槛通过，才考虑明确授权的一次只读 SYS_CFG 诊断。pre-BL33 CPU reset/RVBAR snapshot 仍是独立未闭合项。
- 全程 `hardware_applied=0`：没有实机 MMIO 读写、没有模块加载、没有 boot args/Boot0/TOC1/SD 写入、没有刷写或执行新 BL33。


**2026-11-26 32 KiB loader extent revalidation:** 当前测试提交
`625e9181d493445fcbff77a140830cad428aa44f` 的完整 Linkslot/LK
feasibility [#45834208280](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280)
**81/81 steps 成功、无跳过**；artifact
[15123653120](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280/artifacts/15123653120)
（387,946,289 bytes）。新增的 LK overlay regression 明确断言 raw flat image
不超过 active native loader 32 KiB extent；BaseOS override regression
另外验证恰好 32,768 bytes 接受、32,769 bytes 拒绝。32 KiB raw 文件上限与
`0x4a0e0000` 运行时链接内存上界分开报告。H700 compile #45834208274
及 boot-audit #45834060935 均成功。没有执行诊断函数或实机寄存器读取；
`hardware_applied=0`。


## 2026-11-26 真实 LK early-MMU + RG34XX PI12 layout 验收（当前续接点）

**当前代码/验证基线：** `1396930d314ceb09d0736d1b6cbb5a5dca66e789`。

### 已完成

- 完整 Linkslot/LK feasibility [#45843270056](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45843270056)：**81/81 steps completed，0 failures，0 skipped**；artifact [15131735148](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45843270056/artifacts/15131735148)（387,985,474 bytes）。
- H700 AArch64、AArch32 ARM-state、AArch32 Thumb-2 freestanding compile 与 storage/board host regressions [#45843270266](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45843270266)：success。
- 新增第二个真实 early-init layout ELF：链接 pinned GammaOS LK 的真实 `arch_early_init()`、`arm_mmu_init()`、`arm_mmu_map_section()`、cache/CP15 汇编对象，以及 H700 的 `platform_init_mmu_mappings()`、`physical_memory_size()`、RG34XX `target_early_init()` 和 PI12 LED service。此测试没有 mock MMU 函数。
- 反汇编/符号检查确认 diagnostic `kmain()` 先调用 LK `arch_early_init()`（完成真实 section-table setup、DRAM/SoC MMIO identity mappings、DSB → MMU enable → ISB），再调用 H700 `target_early_init()` 和 PI12 服务。
- raw flat ELF **7,912 bytes**，当前 loader raw-file extent 32,768 bytes，剩余 **24,856 bytes**。真实 LK section table `tt=0x4a008000`，16 KiB 对齐；链接 `_end=0x4a00c024`，image-base 到 `_end` 的内存跨度为 `0xc024` bytes。raw 文件长度与 BSS/page-table runtime footprint 是不同指标。
- LK `arch.c` 与 H700 early-MMU/PIO/LED C 对象按 Thumb-2 编译；FPEXC 访问用 VMRS/VMSR；原始 exception vectors 和固定 SYS_CFG probe assembly 仍为 A32。
- MMU 更新顺序已补齐：写完页表 descriptor/TLBI 后 DSB，调用 `arch_enable_mmu()` 后 ISB。H700 ARM/AArch64 编译与 host regression 通过。

### 当前阻塞 / 未批准事项

- 该 ELF 是**最小 source/layout regression**，省略 `thread_init_early()`、vendor `platform_early_init()`、GIC/timer/console、heap、storage、scheduler 和完整 `kmain()`；不是完整 LK target、不是 vendor-header 镜像、不是可刷写固件，也没有设备启动证据。
- PI12 需要的 APB1/PIO 时钟和访问状态在 BL31→BL33 handoff 时尚未证明；LED 服务不擅自改 CCU/APB1。
- `0x4a000000..0x4a0e0000` 是历史 loader link bound，不构成 BL31/Boot0 对 BL33 text/BSS/page-table/stack 的物理内存所有权批准。真实页表地址 `0x4a008000` 和 `_end=0x4a00c024` 目前只证明链接位置。
- BL31→BL33 时其他 Cortex-A53 核的 reset/hold-off 状态仍未知；pinned LK scheduler/critical-section 是单核设计。未证明 secondary cores held off 或完成 SMP 设计前，不允许多核进入 LK。
- 尚未生成配对 vendor header 的 native firmware/SPL；没有写 SD/Boot0/TOC1、没有 MMIO 读写、没有模块加载、没有刷写或运行 payload，`hardware_applied=0`。

### 下一步（按顺序）

1. 审查 reports 中的 live FDT、BL31 与 Boot0 证据，关闭 BL33 handoff 时 APB1/PIO 的 clock/reset/访问门槛；不能以 Linux 后期时钟状态替代 BL33 入口状态。
2. 完成 BL31/Boot0 对 `0x4a000000` 候选运行区、16 KiB section table、BSS、Abort stack 的冲突与生命周期审计；在证据闭合前保持 `H700_MEMORY_LAYOUT_APPROVED` unset。
3. 确认 secondary A53 cores 的 BL31→BL33 状态；若不能证明 hold-off，先设计并验证 SMP/coherency，而不是直接启动现有单核 LK。
4. 上述门槛全部通过后，才将诊断 raw ELF 接入 BaseOS native builder，复核真实 vendor header / firmware/SPL 配对、32 KiB padding、rollback 与 proof-of-life 路径。
5. 最后才考虑一次明确授权的只读 PI12 LED proof-of-life；SYS_CFG probe 仍保持 dormant，不与 LED 首启诊断捆绑。

本节为后续对话的真实续接点；新的 CI 结果必须以新的 HEAD/run ID 更新本文件，不能沿用历史 81/81 记录。


## 2026-11-26 续接更新：PI12 默认关闭、安全门槛与最新完整 CI

**当前代码 HEAD：** `166e3a97a4a011dff98ea9b33c959d47b643745e`。

### 最新验证结果

- 完整 Linkslot/LK feasibility [#45847816950](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45847816950)：**81/81 steps completed，0 failures，0 skipped**；artifact [15133935777](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45847816950/artifacts/15133935777)（387,985,689 bytes）。
- H700 AArch64 / AArch32 ARM-state / AArch32 Thumb-2 compile、board/storage host regressions [#45847406072](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45847406072)：success。该 run 的 HEAD 为 `a626caa28ad55dec5031bfb0c33a5ee67c8da5b6`，包含 PI12 默认关闭的 C/target 代码；之后只调整 project-guard 的注释过滤正则。
- H700 boot-audit Linux/Windows [#45847165507](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45847165507)：success。
- 当前完整 feasibility 已通过 project/memory fail-closed guard（step 17）、真实 LK CRT0/exception/early-MMU/PI12 layout link（step 13）、Linkslot Frontend/no_std crates、输入、CMD17→GPT→FAT32→Libretro VFS、mGBA VFS v3/newlib archive 和 Rust static API 链接。

### PI12 物理访问改为默认关闭

- `platform/h700/power/h700_rg34xx_power_led.c` 的真实 PIO MMIO wrapper 由 `H700_RG34XX_PI12_EARLY_ACCESS_ENABLE` 控制，默认宏值为 0；未 opt-in 时 early init 和 game-state ABI 均返回 `ERR_NOT_READY`，不会读写 PIO。
- `target/rg34xx_h700/init.c` 的 `target_early_init()` 与 `target_init()` 都受同一宏保护；正常 production `platform/h700/rules.mk` 不定义此宏。
- fake-IO C service 和 Linkslot no_std 状态转换仍可测试；独立 early-MMU/PI12 layout ELF 仅在测试编译参数显式 opt-in，以验证真实 MMU/PI12 对象链接和 raw extent，不改变 production 默认。
- 这样做是必要的：现有 Data Abort matcher 只恢复固定 SYS_CFG `VER_REG=0x03000024` 的受限读；PIO 地址（如 `0x0300b124` / `0x0300b130`）若触发 external Data Abort 仍会进入原 fatal halt，不能把普通 readback error 当成可恢复总线故障。

### 新增 BaseOS SPL 源码证据（不等于 BL33 访问批准）

已审查固定 BaseOS-MultiFrontend commit
[`cbe253a43f46`](https://github.com/dengzhan007/BaseOS-MultiFrontend/tree/cbe253a43f46cf551cc9a6c79a84015bde3686cf)：
`experiments/h700-boot-speed/spl/native-boot.c` 的 RG34XX active panel contract 为 rule=2（PE6 strap，high=old / low=boe）。在 SPL→BL31 之前，`baseos_vendor_handoff()` 会启动 `panel_start()`；该流程写 PIO `0x0300b124` / `0x0300b130` 控制 PI15/PI14，再在 panel sequence 中配置并读取 PE6（`0x0300b090`、`0x0300b0ac`、`0x0300b0a0`）。这证明当前 BaseOS SPL 源码路径在 BL31 前会访问主 PIO；**它不证明 BL31 运行后到 A32 BL33 入口时 APB1/PIO 状态未被改变**。Linux 正常运行期 APB1/PIO clock-summary 也不能替代该 handoff 状态证据。

### 当前未关闭门槛与下一步

1. 反向审查相同设备/monitor 的 BL31 初始化与平台代码，确认 BL31 是否会更改 APB1/PIO 时钟、复位或访问属性；若找不到可证明的 BL33 入口状态，PI12 继续默认关闭。
2. 继续审查 `0x4a000000` 候选 BL33 runtime、真实 LK section table `0x4a008000`、BSS/`_end=0x4a00c024`、Abort stack 的 BL31/Boot0 生命周期与冲突；`H700_MEMORY_LAYOUT_APPROVED` 保持 unset。
3. 查明 BL31→BL33 时 secondary Cortex-A53 cores 的 reset/hold-off 状态，或先设计并验证 SMP/coherency；pinned LK 的 scheduler/critical-section 仍是单核设计。
4. 上述门槛关闭后，才构建匹配 vendor header 的 native firmware/SPL pair，并审查回滚和 proof-of-life。
5. 实机冷启动与 PI12 游戏生命周期验证仍未执行。SYS_CFG probe 保持 dormant；不把 SYS_CFG 诊断和 PI12 诊断捆绑。

全程 `hardware_applied=0`：无实机 MMIO 读/写、无模块加载、无 SD/Boot0/TOC1 写入、无刷写、无 payload execution。


## 2026-11-26 续接：BL31 PIO/CCU 全镜像静态复核

**当前代码 HEAD：** 96e171bd9c298c6f735393670b086270ef2dfc08。  
**当前 reports HEAD：** 2963c8c479486a98c5202dad86e14dbe6e0f466f。

### CI 与报告
- 完整 Linkslot/LK feasibility [#45850518021](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45850518021)（HEAD ae0722f...）：81/81，0 failures，0 skipped。当前 HEAD 在该 run 之后只改动全镜像审计工具/host tests/文档，没有产品启动代码变化。
- 当前 HEAD 的 RG34XX BL31 static memory audit [#45852546186](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45852546186) 全部步骤成功；40-insn scanner、PIO/CCU 区间、PSCI CPU_ON verifier 和 artifact 上传均通过。PIO 寄存器名称补充后仍需等待新提交触发的回归。
- H700 boot-audit Linux/Windows [#45852315408](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45852315408) 成功；H700 AArch64/AArch32 ARM/Thumb-2 编译/host [#45847406072](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45847406072) 成功。

### BL31 全镜像 PIO 结论
- 完整 107,216-byte monitor 身份校验通过；扫描结果 605 个候选行，其中 CPUX_CCU 152、PIO 6、SID_SYS 275、AON_CPUCFG_PRCM 148、C0_CPUX_CFG 20、CPU_SUBSYSTEM 4。计数是重叠反汇编候选行，不是唯一或执行次数。
- PIO 六行是 0x0300b100、0x0300b110、0x0300b118 各一条 load/store。根据 active live-FDT 的 sun50iw9p1 pinctrl compatible 和同 SoC family pinctrl bank layout，这三个 offset 对应 PH10 mux/drive/pull；reference DTSI 的 s_cir0_pins_a 正好配置 PH10 为 IR function/mux3/drive2/pull1。**这不是 PI12 LED**；PI12 的寄存器在 0x0300b124/130/134/13c。
- PH10 setup 经 0x48013df0 的条件 bit-2 分支调用，外围路径访问 SCR_EL3 并执行 WFI。它是 BL31 中一个条件低功耗/红外输入配置路径；入口注册与是否会在 initial handoff 执行仍未知，不能据此证明 BL33 入口 PIO 权限。
- CPUX_CCU 152 行中发现一个非主入口的状态保存/恢复候选：0x48001790 保存 0x03001000、0x03001a78、0x03001a8c，配对恢复路径在 0x480018d8，附近还访问 AON/PRCM 0x07000200。形状符合低功耗上下文保留，但寄存器语义、callback 注册和实际执行尚未完全确认；**这不证明 CPU DVFS 已可用**。
- 详细反汇编和来源边界见 [h700-bl31-pio-ccu-audit.md](h700-bl31-pio-ccu-audit.md)。

### 实机探测与安全门槛
- reports 最新 kernel preflight 没有匹配 4.9.170 build tree、headers 或 Module.symvers；用户态 /dev/mem 所有测试地址均 EFAULT，不再用 devmem/dd/mmap/模块猜测方式重试。
- H700_RG34XX_PI12_EARLY_ACCESS_ENABLE 保持 production default-off；Data Abort 保护只覆盖受限 SYS_CFG load，PIO abort 仍 fatal。
- BL31→BL33 时 secondary A53 状态、PSCI 可用性、1 MiB loader arena/section table/BSS/Abort stack 的生命周期和内存所有权未闭合；H700_MEMORY_LAYOUT_APPROVED 保持 unset。
- 当前没有完整可启动 LK、没有配对 vendor-header firmware/SPL，没有实机启动证据。hardware_applied=0：无 MMIO 读写、无模块加载、无 SD/Boot0/TOC1 写入、无刷写、无 payload execution。

### 下一步
1. 将 BL31 PIO/CCU callbacks 与注册表/PSCI dispatch 对应起来，区分 initial entry 和后续 low-power paths。
2. 从 pinctrl/clock 的确切 vendor 4.9.170 源码或其他可验证资料完成 PH10/CCU 寄存器语义交叉核对；不要把 family reference 当作已锁定的设备 kernel source commit。
3. 继续关闭 BL31/Boot0 内存所有权和 secondary A53 reset/hold-off/SMP 门槛。
4. 门槛关闭后再考虑完整 LK platform init（GIC/timer/console/heap/storage/scheduler）及 firmware/SPL builder；目前不生成可刷写镜像。


## 2026-11-26 续接：BL31 PSCI 表与 CCU helper 关联审计

**当前代码 HEAD：** `7c65f60a416f4e80c5d6290ceee37ccc072b9f8d`。  
**reports HEAD：** `2963c8c479486a98c5202dad86e14dbe6e0f466f`。

### 已完成的 CI
- 完整 Linkslot/LK feasibility [#45854374894](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45854374894)：81/81 steps 成功，0 failures / 0 skipped；这是当前最近的完整集成检查，运行 HEAD 为 `51882e9...`。其后的代码变更仅是 BL31 只读审计和 host tests，不改变产品运行时代码。
- 当前代码 HEAD 的 BL31 静态审计 [#45856177699](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45856177699)：20/20 steps 成功；PSCI dispatch/table、CPU_ON 指令目标、CCU save/restore helper 分支目标测试全部通过。
- H700 boot-audit Linux/Windows [#45854375018](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45854375018) 成功；三 ABI H700 AArch64/AArch32 ARM/Thumb-2 编译与 host 回归 [#45847406072](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45847406072) 成功。

### PSCI / CCU 证据边界
- 匹配 BL31 的 platform-ops 表位于 `0x480165f8`；注册路径在 `0x48009dec → 0x480029bc`。PSCI dispatcher 读取 `psci_plat_pm_ops` 全局指针并通过间接分支调用表项。slot 1 指向 `0x4800254c`，其调用路径到达低层 CPU_ON 序列 `0x48002d78`，包括 per-core RVBAR 与 power/reset 操作。
- 表项原始值：slot 3=`0x48002878`，slot 7=`0x48002574`。**这两个 slot 当前保持未分类**，不把它们直接命名为 `pwr_domain_suspend` 或 `pwr_domain_on_finish`。
- 两条直接 BL 目标已验证：PC `0x48002998 → 0x48001790`（CCU/AON save helper），PC `0x48002858 → 0x480018d8`（restore helper）。这证明两条分支目标存在，不证明它们属于同一个回调、从 slot 3 可达，或会在 initial BL31→BL33 handoff 执行。
- 当前无法为出厂 BL31 匹配确切 TF-A callback ABI。作为版本差异示例，公开 TF-A header commit [`9839bceb...`](https://github.com/ARM-software/arm-trusted-firmware/blob/9839bceb8b720ba09f3bd3d5f95188ea9d75837b/include/lib/psci/psci.h#L335-L372) 已在常见回调之前加入额外成员；不能将该版本的 slot 排列直接套用到设备固件。
- 因此 CCU helper 仍属于 BL31 power-management 候选代码，不是 CPU DVFS 可用证据，也不是开机阶段 PLL/clock 初始化证据。具体 callback ownership、CFG 可达性和运行时调用时机仍待闭合。

### 不变的硬件门槛
- PH10/S_CIR 的 PIO 候选不触及 PI12 电源灯寄存器；PI12 production MMIO default-off。
- `/dev/mem` 全地址 EFAULT、无匹配 4.9.170 module build tree/headers/Module.symvers；不重试 devmem/mmap/模块猜测路径。
- `H700_MEMORY_LAYOUT_APPROVED` 未设置；secondary A53 初始 reset/hold 状态、BL33 内存生命周期和 timer handoff continuity 未证明。
- 未执行 MMIO、模块加载、SD/Boot0/TOC1 写入、刷写或 payload execution；`hardware_applied=0`。

### 下一步
1. 获取与出厂 BL31 同版本的 TF-A/Allwinner 源码或可验证 ABI 证据，精确恢复 platform-ops 结构布局。
2. 对 slot 3/7 及 helper call sites 建立完整函数边界和 CFG，分别确认 CPU suspend / resume / off 回调归属。
3. 区分 cold-boot handoff 与后续 PSCI power-management 路径；在没有运行时 trace/readback 前不认定 helper 已执行。
4. 继续关闭 BL31/Boot0 内存所有权、secondary A53 hold-off、PIO/clock permissions 门槛后，再考虑完整 LK target integration。


## 2026-11-26 续接：PSCI 操作表中 CCU helper 的分项归属

**静态审计代码 HEAD：** `73679e227f520f05e729dcbb03f5f80f05187491`。  
**reports HEAD：** `2963c8c479486a98c5202dad86e14dbe6e0f466f`。

### CI
- 当前 HEAD 的 BL31 静态审计 [#45857603997](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45857603997)：20/20 steps 成功，产物 [15136726031](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45857603997/artifacts/15136726031)。
- 当前文档提交触发的完整 feasibility #880 [#45856568955](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45856568955) 最近一次检查仍在 `Install LLVM cross-link tools`，10/80 steps，未报告失败；不能记为通过。
- H700 boot-audit #1424 [#45856568821](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45856568821)：Linux 与 Windows job 均成功。
- 最近一次已完成的全量 Linkslot/LK feasibility #879 [#45854374894](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45854374894)：81/81 steps 成功；其后变化主要是 BL31 host-only 审计和文档。

### PSCI raw ops table：新增的静态控制流证据
匹配的 BL31 platform-ops 表位于 `0x480165f8`，注册路径为
`0x48009dec → 0x480029bc`。PSCI dispatcher 从
`psci_plat_pm_ops` 全局指针取表项并间接调用。CI #33 的
`PSCI-CALLBACK-DISASSEMBLY.txt` 对所有表项及两个 CCU helper 输出了
AArch64 反汇编窗口，验证：

- **raw slot 3** 指向函数入口 `0x48002878`。该函数在条件路径中于
  `0x48002998` 调用 CCU/AON 保存 helper `0x48001790`；其 epilogue
  在 `0x480029b4` 返回。
- **raw slot 5** 指向函数入口 `0x480026c4`。该函数的一条条件路径在
  `0x48002858` 调用 CCU/AON 恢复 helper `0x480018d8`；之后在
  `0x48002874` 尾跳到 **raw slot 4** 的入口 `0x48002680`。
- CI 同时校验了 slot 3/4/5 原始指针值、两条 BL helper 目标和
  `0x48002874` 的 AArch64 B 目标。反汇编窗口对 slot 5 精确覆盖
  `[0x480026c4,0x48002878)`，不把相邻 slot 3 的入口包含进来。
- 因此可以确认两个 helper 调用分别位于不同 raw ops-table entry 的
  函数代码区间中。**不能**仅据此命名 slot 3/4/5 为 suspend/off/finish
  或 resume：出厂 BL31 对应的 TF-A ABI/源码版本仍未锁定。
- 两个调用都是条件路径。二进制静态存在不证明设备启动、休眠或唤醒时
  实际执行；也不证明 BL33 启动时进行 PLL/CPU 频率切换。

### 仍然关闭的门槛
- CCU helper 的 PSCI 回调语义、实际触发条件和运行时调用尚未全部确认；
  CPU DVFS 不可用，禁止写 PLL/CCU/PMIC。
- PH10/S_CIR 是红外输入路径，不是 PI12 LED；PI12 MMIO 仍 default-off。
- BL31→BL33 PIO/clock/permission、BL31/Boot0 RAM 生命周期、secondary
  A53 初始 reset/hold-off 和 timer continuity 尚未闭合。
- `H700_MEMORY_LAYOUT_APPROVED` 未设置；`hardware_applied=0`。


## 2026-11-28 M1 SPL build-profile audit correction

- The first real isolated H700 SPL build attempt exposed a test-contract error before compilation: the harness attempted to force `SPL_SYS_MALLOC_SIMPLE`, `SPL_USE_TINY_PRINTF`, and a nonexistent `SPL_CMDLINE` symbol off, then required those exact disabled lines after `olddefconfig`. Pinned U-Boot v2026.10 resolves the first two to enabled defaults; `SPL_CMDLINE` is not defined in this source tree. The failure was therefore a stale/invalid Kconfig expectation, not evidence of an SPL source or linker failure.
- Updated `scripts/build-m1-spl-preflight.py` to assert the resolved defaults accurately, remove ineffective disable requests, and record the simple allocator / early malloc configuration in the manifest. The report now explicitly states that the simple-malloc heap high-water mark and exact runtime reservation remain unmeasured; the 4 KiB static BSS scratch check alone does not prove total SPL memory safety.
- Latest pre-correction run [#1039](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46110121736) failed at the SPL build-profile assertion with `resolved SPL config missing contract: # CONFIG_SPL_SYS_MALLOC_SIMPLE is not set`. Its earlier source audits and host-only M1 tests passed. The code correction is a new commit and must pass a fresh run before any SPL link/layout claim is made.
- Current source remains host-only and non-installable. No hardware access, SD/Boot0/TOC1 writes, firmware flash, MMIO/SMC, TWI, DRAM access, or payload execution occurred. G2/G3/M1-S1 remain open; `hardware_applied=0`.

**Next gate:** rerun the isolated SPL build with the corrected Kconfig contract. If linking succeeds, inspect actual `__bss_start/__bss_end`, scratch/table symbols, allocator placement and high-water evidence, stack separation, provider set, and linked-ELF side effects. Do not treat source Kconfig resolution as a completed memory-safety proof.


## 2026-11-28 M1 SPL allocator memory contract refinement

- Follow-up review of pinned U-Boot v2026.10 `include/system-constants.h`, `common/spl/Kconfig`, `drivers/core/Kconfig`, and `common/spl/spl.c` showed that merely removing ARCH_SUNXI's simple-malloc selector is insufficient: driver model requires a suitable allocator, and the full SPL malloc pool is placed at `CONFIG_SPL_BSS_START_ADDR + CONFIG_SPL_BSS_MAX_SIZE` unless a custom start is configured.
- Updated the isolated harness to use full SPL dlmalloc with an explicit **64 KiB** pool at `0x34000..0x44000`, immediately after the reserved BSS window `0x30000..0x34000`. This ends below the configured initial stack pointer `0x52a00`, leaving a static 60,928-byte gap. `SPL_SYS_MALLOC_F` remains enabled with a 8 KiB early pool; simple malloc and tiny printf are explicitly disabled after the disposable ARCH_SUNXI selector patch. The nonexistent `SPL_CMDLINE` symbol remains excluded from assertions.
- The manifest now records the full allocator profile, heap start/size/end and stack gap. It explicitly leaves allocator high-water usage unmeasured. This is a static reservation proposal for host-side link audit only; actual heap use and all hardware access remain untested.
- This supersedes the immediately preceding note's provisional recording of Kconfig-default simple malloc/tiny printf. The final acceptance criterion is the resolved profile above, not a forced guess about upstream defaults.

**Next gate:** fresh CI must prove the resolved Kconfig profile, link the actual H616 SPL ELF, and verify BSS/scratch/GPT-table/heap/stack intervals from the final map and symbols. The 64 KiB pool must remain candidate-ineligible until stack and allocator usage are measured or conservatively bounded. No installable output or hardware operation is authorized.


## 2026-11-28 M1 early SPL allocator initialization

- The M1 hook is deliberately invoked from `board_init_f`, before normal `board_init_r`; pinned U-Boot source shows the standard `CONFIG_SPL_SYS_MALLOC` pool is initialized in `spl_init()` / `board_init_r`, which this non-returning preflight harness never reaches.
- Updated `baseos_m1_preflight.c` to initialize the configured SRAM-C dlmalloc pool explicitly at `SPL_SYS_MALLOC_START` / `SPL_SYS_MALLOC_SIZE` and set `GD_FLG_FULL_MALLOC_INIT` **before** `mmc_initialize()`. This prevents MMC/DM discovery from silently relying on the 8 KiB early malloc-F fallback while claiming a 64 KiB full pool.
- The pool remains `0x34000..0x44000`; the reserved BSS window is `0x30000..0x34000`; configured stack is `0x52a00`. This is a static non-overlap design only. It does not prove allocator high-water, stack high-water, actual SRAM availability at BL33 entry, or BL31/Boot0 ownership.
- CI must now verify this initialization order in the linked SPL call path and validate the final map/ELF. The zero FNV sentinel still rejects the firmware package; this harness cannot hand off to BL33 and remains non-installable.


## 2026-11-28 M1 SPL allocator profile assertion correction

- Feasibility [#1048](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46111154580) reached the isolated SPL build step and failed at Kconfig validation, before compiling/linking. The resolved config correctly had `# CONFIG_SPL_USE_TINY_PRINTF is not set`, but the assertion tuple also incorrectly required `CONFIG_SPL_USE_TINY_PRINTF=y`; the same block redundantly listed the disabled state twice. This was a contradictory test assertion, not a compiler or linker failure.
- Removed the contradictory enabled-state requirement and deduplicated the allocator/printf options. The intended M1 profile is full SPL dlmalloc with a 64 KiB SRAM-C pool, simple malloc disabled, tiny printf disabled, and the 8 KiB early malloc-F region recorded separately. (The early malloc-F pool remains enabled; its address/overlap must be audited independently.)
- The M1 hook now explicitly initializes the full pool and sets `GD_FLG_FULL_MALLOC_INIT` before MMC/DM discovery. This still requires a fresh CI build and final ELF/map audit; no successful SPL link is claimed yet.


**Correction:** the previous assertion cleanup must retain `CONFIG_SPL_SYS_MALLOC_F=y` with `CONFIG_SPL_SYS_MALLOC_F_LEN=0x2000`. This early pool is required by `spl_early_init()` before the explicit full-pool initialization in the M1 hook; it is not disabled. The profile now distinguishes the 8 KiB early allocator arena from the 64 KiB full dlmalloc pool.


## 2026-11-28 M1 SPL link dependency isolation

- Retrieved and inspected the complete #1050 workflow artifact and pinned upstream U-Boot v2026.10 `common/Makefile`, `common/stdio.c`, `common/cli.c`, and `lib/Makefile`. The SPL link failure was caused by upstream XPL Makefile rules that unconditionally add `stdio.o`, `cli.o`, `command.o`, and `s_record.o`. Those objects reference serial console providers, legacy I2C init, CLI buffer/control helpers and serial input even though this M1 path intentionally disables SPL serial/I2C and has no command interpreter.
- In the disposable U-Boot checkout created by the preflight script only, these four objects are now restricted to non-XPL builds. Normal U-Boot sources and existing raw-capsule v2 path are not modified. `SPL_ENV_SUPPORT` is disabled because this fixed read-only p4+40MiB path does not use environment variables; `SPL_SPRINTF` is explicitly enabled to provide the full `sprintf/vsnprintf` implementation required by the MMC and FDT/DM helpers.
- This is dependency isolation, not a workaround by adding UART/I2C/PMIC providers. The next CI must prove the remaining SPL dependency set and inspect linked ELF undefined symbols, relocation/call graph and side-effect sites. If any disabled provider remains reachable, the build must fail rather than silently re-enable it.
- No hardware access or writes were performed. G2/G3/M1-S1 remain open; output remains host-only, non-installable, and candidate-ineligible.


## 2026-11-28 M1 SPL disposable Makefile patch assertion correction

- Feasibility [#1051](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46112268640) completed with failure at the isolated SPL preflight script before U-Boot compilation. The failure was in our patch guard: the Python tuple used the two-character sequence `\\n` instead of an actual newline in the expected Makefile rule, so the exact upstream line was incorrectly reported as unexpected.
- Corrected `scripts/build-m1-spl-preflight.py` to construct each expected rule from an object name and a real newline, while retaining the fail-closed exact-one-match check and the XPL-only exclusion wrapper. This is a harness string-literal defect, not evidence about SPL linker dependencies.
- The separate H700 boot-audit workflow [#1655](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46112269757) completed successfully (both Windows and host-tools jobs). This does not validate the SPL link.
- Next gate: rerun feasibility from the corrected script, then inspect the actual linked SPL ELF/map and reachable provider/side-effect set. No SPL link success is claimed; all hardware gates remain closed and no device operation occurred.


## 2026-11-28 M1 SPL printf Kconfig dependency correction

- Feasibility [#1052](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46112527006) completed with failure at the resolved-config assertion, before SPL compilation/linking. Artifact inspection showed the generated config had `CONFIG_PRINTF=y` but neither `CONFIG_SPL_PRINTF=y` nor `CONFIG_SPL_SPRINTF=y`; the harness had attempted to enable the hidden leaf symbol `SPL_SPRINTF` directly, which Kconfig does not select without its parent.
- Pinned U-Boot v2026.10 `lib/Kconfig` confirms `SPL_PRINTF` selects `SPL_SPRINTF` (and `SPL_STRTO` when tiny printf is disabled). Changed the requested symbol to `SPL_PRINTF` and strengthened the resolved-config contract to require both parent and selected implementation. This is a Kconfig dependency correction; no compiler/linker failure occurred in #1052.
- The #1052 fake-block-device and GPT read-only adapter tests completed before the SPL config gate; they do not establish a successful SPL link.
- Next gate: fresh feasibility run must resolve the full printf profile, then compile/link the actual SPL and validate final ELF/map. All hardware gates remain closed; no device operations occurred.


## 2026-11-28 SPL_PRINTF hidden-symbol Kconfig selection correction

- Linkslot LK feasibility [#1053](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46112854654) failed again at the resolved-config assertion, before the SPL compile/link step. The artifact's final config contains `CONFIG_SPL_LIBCOMMON_SUPPORT=y`, `CONFIG_PRINTF=y`, and `CONFIG_SPRINTF=y`, but neither `CONFIG_SPL_PRINTF` nor `CONFIG_SPL_SPRINTF`.
- Inspection of the pinned U-Boot v2026.10 Kconfig shows `SPL_PRINTF` is a hidden bool with no prompt or default; the only stock selector in this profile was `SPL_SERIAL`. Since the preflight intentionally disables serial, `scripts/config --enable SPL_PRINTF` is ineffective and must not be treated as a valid way to enable a hidden symbol.
- Corrected the disposable-source patch: add an exact-match, fail-closed `select SPL_PRINTF` to the existing `SPL_LIBCOMMON_SUPPORT` Kconfig block in the temporary U-Boot checkout, then let Kconfig resolve `SPL_SPRINTF`. Removed the ineffective direct `scripts/config --enable SPL_PRINTF` request. This preserves the no-UART policy and leaves upstream source untouched.
- Separate H700 boot audit [#1657](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46112872726) completed successfully; it does not validate SPL linking.
- Next gate: verify the resolved config now contains both `CONFIG_SPL_PRINTF=y` and `CONFIG_SPL_SPRINTF=y`; only then proceed to actual SPL compile/link, final ELF/map and undefined-symbol/provider audit. No device operations occurred; G2/G3/M1-S1 remain open, and all outputs remain non-installable, hardware-unvalidated, and candidate-ineligible.


## 2026-11-28 SPL printf Kconfig patch newline-literal correction

- Feasibility [#1054](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46113193508) failed in the preflight script before U-Boot compilation. The exact-match guard reported `unexpected SPL_LIBCOMMON_SUPPORT Kconfig definition`.
- Root cause was another patch-script escaping defect: the Python expected-block string contained literal backslash-plus-`n` / `t` characters instead of newline/tab escapes, so it could not match the pinned U-Boot Kconfig source. The upstream pinned source was checked directly: the definition is `config SPL_LIBCOMMON_SUPPORT` followed by `bool "Support common libraries"`.
- Corrected the Python string literals to represent actual newline and tab characters, retaining the exact-one-match fail-closed guard and the temporary-checkout-only `select SPL_PRINTF` insertion. Code commit: `e030f96cbd3df58854d2ffd726ccf8393d63ce49`.
- H700 boot audit [#1658](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46113215190) completed successfully; this is independent and does not validate the SPL build.
- Next gate: #1055 must pass the Kconfig source patch and resolve `CONFIG_SPL_PRINTF=y` plus `CONFIG_SPL_SPRINTF=y`; only then can the SPL compile/link and linked SRAM BSS audit provide evidence. No device operations occurred; G2/G3/M1-S1 remain open, outputs remain non-installable, hardware-unvalidated, and candidate-ineligible.


## 2026-11-28 M1 SPL printf/serial declaration conflict correction

- Feasibility [#1055](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46113419377) passed the resolved Kconfig contract and began compiling the isolated U-Boot v2026.10 SPL, but failed in `lib/vsprintf.c`: `printf` and `vprintf` were redefined against inline stubs in `include/stdio.h`. This is a compile-time interface mismatch, not a linker or SRAM-layout result.
- Root cause: upstream `include/stdio.h` places the real `printf/vprintf` prototypes in the same conditional branch as serial `putc/puts` providers. The M1 profile intentionally disables serial, so the header supplied inline no-op definitions even though `CONFIG_SPL_PRINTF=y` compiled the real formatter implementation.
- Corrected `scripts/build-m1-spl-preflight.py` in commit `a9a7fe0ccde6097ad0ba78ae73e9729d1f112133`: the disposable U-Boot checkout now separates formatter declarations from console output providers. With serial disabled, `putc/puts/flush` remain silent inline stubs while `printf/vprintf` expose the real formatter declarations. The patch is guarded by an exact-one-match check and does not enable UART, add console I/O, or modify the upstream repository.
- The separate H700 boot audit [#1659](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46113440168) succeeded; it does not validate the SPL build.
- Next gate: fresh feasibility CI must pass the patched SPL compilation/link, then verify final ELF/map BSS, scratch/table bounds, allocator and stack separation, undefined symbols, and linked side-effect sites. No hardware operations occurred; G2/G3/M1-S1 remain open, and outputs remain host-only, non-installable, hardware-unvalidated, and candidate-ineligible.


## 2026-11-28 SPL stdio patch guard robustness correction

- Feasibility [#1056](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46113801244) passed all preceding host-only capsule/GPT tests and the resolved SPL Kconfig checks, then failed before compilation at the new disposable `include/stdio.h` patch guard: `unexpected U-Boot stdio.h stdout declaration block`. The first correction used a byte-for-byte multiline template that did not match the pinned source's exact formatting; no compiler/linker result was produced.
- Replaced the brittle whole-block literal with fail-closed section-boundary matching: require exactly one serial stdout marker and one following formatter-documentation marker, then replace only the bounded stdout declaration section. The replacement continues to keep console sinks silent while exposing the real SPL printf implementation; it does not enable serial/UART. Code commit: `254c02cb9e7a4a9c72976e32089eb59e2d8c017e`.
- H700 boot audit [#1660](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46113829945) succeeded independently; it does not validate SPL compilation.
- Next gate: fresh feasibility run must pass this exact-section guard and reach SPL compile/link. Continue to inspect the final ELF/map only after a real link succeeds. Hardware gates remain closed; no device operations occurred.

## 2026-11-28 M1 isolated SPL link failure #1057

- Feasibility [#1057](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46114050825) completed with **failure** at step 21, “Build isolated M1 preflight-only H700 SPL and audit linked SRAM BSS”. The previous read-only fake-MMC/GPT/package tests and source overlay steps passed. H700 boot audit [#1661](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46114073397) passed independently; it does not validate this SPL link.
- The failure is at the final AArch64 SPL link, not compilation of the M1 adapters and not a device test. Linker diagnostics show unrelated full-U-Boot code is still entering the SPL link. Distinct unresolved symbol families include:
  - sunxi board/full-board hooks: setup_environment, misc_init_r, parse_spl_header, board_mmc_init, and sunxi_mmc_init;
  - full image/boot path: env_get*, env_set*, lmb_alloc_mem, print_size, dm_rng_read, dm_pci_read_bar32, and _image_binary_end;
  - command/console and generic providers: tstc, getchar, crc16_ccitt, hash_progressive_lookup_algo, and bdinfo_print_num_l;
  - linker section bounds: __start_rodata / __end_rodata.
- This demonstrates that the current isolated Kconfig and Makefile pruning is incomplete: disabling FIT/commands/serial at the requested config level did not prevent several full-featured board/image/command objects or their retained sections from reaching the SPL link. The linker script's added rodata bounds also did not satisfy every reference in the final link.
- **Do not paper over this with weak stubs, --unresolved-symbols=ignore-all, or by enabling the whole vendor PMIC/TWI/console stack.** Those approaches would either create false link evidence or widen the hardware side-effect surface. The next fix must identify each retained object from the generated map/link command and its Kconfig/Makefile selection, then prune only objects/functions not reachable from the fixed read-only M1 entry. Keep the DM MMC/FDT providers actually required by the M1 hook.
- The next attempt should preserve the complete linker command, map, resolved .config, and an object-to-unresolved-symbol report even on failure. Only after a genuine fully resolved ELF exists should the BSS/scratch symbol assertions run.
- Evidence boundary unchanged: no SPL executed, no physical MMC read, no SRAM/DRAM access, no MMIO/SMC/TWI transaction, no media write, no firmware flash, and no payload execution. H700_MEMORY_LAYOUT_APPROVED=false, hardware_applied=0; output remains non-installable and candidate-ineligible.

## 2026-11-28 M1 SPL link diagnosis and section-GC correction

- Inspected the uploaded #1057 artifact [15250968308](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46114050825/artifacts/15250968308), including the actual partial SPL linker map. This narrowed the cause beyond the initial unresolved-symbol list: U-Boot's XPL link wraps its built-in archives in --whole-archive, and the map shows unrelated functions such as board/sunxi setup_environment, misc_init_r, boot/image-board, common/xyzModem and bdinfo being retained as separate .text.<function> sections.
- The object code is already emitted into per-function sections. The M1 harness had supplied only -Map=... and had not enabled linker section garbage collection. Therefore the linker retained unrelated functions from whole-archive objects even though the M1 entry does not call them. This explains the broad undefined-reference families without adding their disabled providers.
- Corrective build change: commit [57ccda2](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/57ccda28f47cf7fe619b24fe2ad5066dda9049ec) adds --gc-sections to the isolated SPL link while retaining the map. This preserves the whole-archive mechanism for linker-list discovery but allows unreferenced function/data sections to be discarded. It does not add weak stubs or re-enable console, PMIC, TWI, FIT, or image-loading paths.
- This is a hypothesis-driven link correction, not a pass. The next feasibility run must show a fully resolved SPL ELF, then pass the existing assertions for the 4 KiB aligned scratch, bounded GPT table, BSS ceiling, stack separation, and forbidden write/side-effect symbols. If any unresolved references remain, use the new map to identify their live call path rather than broadening Kconfig.
- The linked map inspection also corrects the earlier diagnosis: the immediate cause is not simply missing Kconfig providers; it is whole-archive retention without --gc-sections. The archive includes full feature objects, but their unrelated function sections should not be part of this M1 preflight image.
- No SPL was executed and no hardware access occurred. H700_MEMORY_LAYOUT_APPROVED=false, hardware_applied=0; all artifacts remain non-installable, hardware-unvalidated, and candidate-ineligible.

## 2026-11-28 Feasibility #1058 script-format regression

- The first section-GC correction in [#1058](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46114757784) did not reach the linker. The Python builder failed immediately with NameError: map_arg is not defined at the SPL make invocation.
- Root cause: the explanatory comment and map_arg assignment had been inserted with literal backslash-n characters, so Python treated the entire inserted block as one comment line. This was a patch-generation formatting error, not an SPL or linker regression.
- Corrected in [1566094](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/15660947d1f7827ab226b09076c18d494b03b6ec): restore actual source newlines so map_arg is assigned and --gc-sections reaches the linker command. The section-GC hypothesis remains unverified until the next run completes the actual SPL link.
- No hardware action occurred. All existing hardware gates and candidate-ineligible status remain unchanged.
## 2026-11-28 Feasibility #1059: section GC works; linker-script symbols remain unresolved

- Feasibility [#1059](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46114934837) reached the actual SPL link after the #1058 Python formatting fix. `--gc-sections` was passed to the linker and reduced the previous large unresolved-symbol set to only two symbol families: `__bss_end` (referenced by the early SPL boot path and the M1 board hook) and `_image_binary_end` (referenced by `fdtdec_setup`).
- Downloaded and inspected artifact [15251217138](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46114934837/artifacts/15251217138), including the generated `spl/u-boot-spl.lds` and link map. The generated linker script text contains assignments for `__bss_end` and `_image_binary_end`, but the actual link map does not report those symbols; it reports `__bss_end__` and `_end` instead. This discrepancy means the next investigation must verify the exact linker command and the linker-script file actually consumed by the SPL link, rather than adding speculative aliases.
- This confirms the section-GC change materially improved link reachability: previous unresolved board/image/command providers disappeared. The current failure is now a linker-script/symbol-contract mismatch, not broad Kconfig provider leakage.
- Next: capture verbose (`V=1`) final SPL link invocation and the exact preprocessed linker script used by that invocation; compare its path/hash with the archived `spl/u-boot-spl.lds`. Determine why the script's declared boundary assignments are absent from the map. Do not alias `__bss_end` to an arbitrary symbol or `_image_binary_end` to `_end`: those boundaries have different memory semantics, especially with `CONFIG_SPL_SEPARATE_BSS`.
- No hardware operation occurred. `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`; no SPL execution, physical MMC read, SRAM/DRAM access, MMIO/SMC/TWI transaction, media write, firmware flash, or payload execution. Artifacts remain non-installable, hardware-unvalidated, and candidate-ineligible.

## 2026-11-28 Follow-up instrumentation for #1059 linker-script mismatch

- Added `V=1` to the isolated SPL make invocation in [617bb46](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/617bb46e8a26b1cbb56da6cb2294b44383d87307). The next CI log will retain the exact final linker command, including the actual `-T` linker-script path, so we can resolve why the archived preprocessed script declares `__bss_end` and `_image_binary_end` while the link map omits them.
- This is diagnostic instrumentation only; it does not alter linker-script symbols, memory placement, or hardware behavior. Do not add aliases until the exact consumed script and symbol semantics are confirmed.

## 2026-11-28 Feasibility #1060: linker-script dropped by command-line variable override

- Feasibility [#1060](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46115534114) reached the final AArch64 SPL link but failed with unresolved `__bss_end` and `_image_binary_end`.
- The verbose linker command provided the decisive evidence: it contained `--gc-sections` and the map output option, but no `-T u-boot-spl.lds`. This was not a missing symbol declaration in the linker script. The harness passed `LDFLAGS_u-boot-spl=...` on the make command line; GNU make command-line variable precedence suppressed the target-specific assignment in U-Boot v2026.10 `scripts/Makefile.xpl` (line 424), which appends the mandatory `-T u-boot-spl.lds`.
- Corrected in [59c5826](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/59c582629b633736a6dea0a729eec0afa7448f3c): move the map and `--gc-sections` flags to `KBUILD_LDFLAGS`, leaving `LDFLAGS_u-boot-spl` under upstream Makefile.xpl control. This preserves the real linker script and its distinct `__bss_end` / `_image_binary_end` boundaries; no speculative aliases were added.
- Next CI must prove the emitted linker command contains both `-T u-boot-spl.lds` and `--gc-sections`, then produce the ELF/map and pass linked-symbol plus SRAM/BSS assertions. No hardware execution or access occurred; `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`; output remains non-installable, hardware-unvalidated, and candidate-ineligible.


## 2026-11-28 Feasibility #1061: isolated M1 SPL link succeeds

- Feasibility [#1061](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46116018997) completed successfully at commit [59c5826](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/59c582629b633736a6dea0a729eec0afa7448f3c). The previous linker-script regression is resolved: the final U-Boot SPL link now retains upstream `-T u-boot-spl.lds` while adding `--gc-sections` and map output through `KBUILD_LDFLAGS`. The generated `spl/u-boot-spl.lds` and linked map are present in the artifact.
- The actual isolated AArch64 SPL ELF linked successfully (1,108,136 bytes; raw image 47,901 bytes). The manifest reports BSS `0x30000..0x313a8` (5,032 bytes within the 16 KiB SRAM-C BSS window), M1 package scratch at `0x30340` (4 KiB, 64-byte aligned, inside BSS), GPT table at `0x30000` (832 bytes, 8 entries, inside BSS), and initial stack at `0x52a00`, leaving 136,792 bytes between BSS end and stack. The configured 64 KiB SPL heap is `0x34000..0x44000`, statically separated from the stack.
- The same run passed the read-only fake-block-device SPL adapter, GPT discovery, fake-MMC/AArch32 package preflight, and the existing Linkslot/Rust, mGBA, VFS, FAT/exFAT, and LK overlay build/test steps. The companion H700 boot-audit workflow [run 46116023011](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46116023011) also completed successfully.
- This closes the specific #1059/#1060 compile-link blocker; it does **not** establish a bootable or installable M1 capsule. The artifact explicitly records `heap_high_water_measured=false`, `firmware_payload_read_at_runtime=false`, `device_io_performed=false`, `hardware_qualified=false`, `installable=false`, and `candidate_eligible=false`. Its zero FNV sentinel intentionally rejects before firmware payload reads. The preflight profile still contains pre-MMC SRAM/TZPC/timer/clock setup and an MMC read path, so it must not be executed on-device.
- Next work: independently audit the linked SPL ELF/map/disassembly for transitive MMIO/SMC/write-capable instructions and verify the real M1 memory ownership contract, then measure/justify allocator high-water usage and complete the one-shot capsule/SPL path. Keep the hardware gates closed: `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`; no SPL execution, physical MMC access, SRAM/DRAM access, MMIO/SMC/TWI transaction, media write, firmware flash, or payload execution occurred.


## 2026-11-28 #1061 linked SPL disassembly: execution remains explicitly blocked

- Performed an independent ELF inspection of artifact [15251508557](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46116018997/artifacts/15251508557), not just the workflow's manifest. ELF identity is ELF64 little-endian AArch64 ET_EXEC, entry `0x20060`; the linked symbol table confirms `__bss_start=0x30000`, `__bss_end=0x313a8`, and `_image_binary_end=0x2ba30`.
- The disassembly confirms the current `board_init_f` path calls `sunxi_sram_init`, `timer_init`, `clock_init`, `spl_early_init`, then branches to `baseos_early_mmc`; that path calls `mmc_initialize`, `find_mmc_device`, `mmc_init`, and obtains the block descriptor. The retained `clock_init_safe` / `clock_h6_set_cpu_pll` code contains direct MMIO loads/stores (including accesses based around `0x03001500` and `0x01710250`). This independently confirms that the linked preflight SPL is **not safe to execute** as-is, even though it has no firmware payload read in its deliberate fail-closed planner path.
- The manifest's absence list covers selected named write APIs, not arbitrary register stores or all transitive call paths. It must not be interpreted as proof of an MMIO-free image. The direct register accesses and early MMC bring-up are retained in the linked ELF and require a separate source/disassembly call-path audit and an explicit execution policy before any device-side test.
- Next implementation step: split the link-only SRAM/BSS geometry fixture from any executable pre-entry path, or make the M1 entry path bypass all generic U-Boot board/clock/SRAM/MMC initialization and prove that bypass in the final linked disassembly. Preserve only the minimal, explicitly reviewed read-only transport required by the one-shot capsule. No device execution, MMIO/SMC/TWI transaction, physical MMC access, SRAM/DRAM access, media write, or firmware flash was performed; all hardware gates remain closed.


## 2026-11-28 M1 SPL follow-up: automate reachable callgraph/store triage

- Added `scripts/audit-m1-spl-disassembly.py` and wired it immediately after the actual SPL link in the feasibility workflow. It starts at linked `board_init_f`, follows direct `BL` calls and direct tail branches to linked function symbols, then emits the reachable function set, direct call edges, and every reachable store-class instruction into `m1-spl-reachable-callgraph.json`.
- The report explicitly records `execution_safe=false` and `execution_authorized=false`. It is a triage artifact to make the current MMIO/initialization surface inspectable and reproducible; it is not a semantic MMIO decoder or a hardware authorization gate. Indirect calls, register dataflow, effective-address classification, SMC/system-register operations, exception paths and memory ownership still require separate review.
- The known unsafe path names (`sunxi_sram_init`, `tzpc_init`, `timer_init`, `clock_init`, `clock_init_safe`, `clock_h6_set_cpu_pll`, `mmc_initialize`, `mmc_init`, `find_mmc_device`, `baseos_early_mmc`) are called out when reachable. This prevents the previous narrow forbidden-symbol list from being mistaken for a full execution-safety proof.
- The workflow change is in [4de5611](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/4de5611c32d064ca5c1f01d5b35bd16d375d0eb6); the audit script is in [9c3e0d3](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/9c3e0d3333478834b64a2da058903ad70034d0fa). The next feasibility run must validate the parser against the linked #1061 disassembly and publish the report. No hardware access or device execution is introduced by this step.
- Next engineering decision remains to split the SRAM/BSS geometry-only link fixture from the executable M1 path, then separately design and audit the minimum required clock/MMC transport. Do not strip required controller setup based only on a static grep, and do not execute the current preflight SPL. All memory ownership and hardware gates remain closed; output remains non-installable and candidate-ineligible.



## 2026-11-28 Feasibility #1062: reachable SPL callgraph audit passes; unsafe entry path quantified

- Feasibility [#1062](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46117175390) completed **successfully** at commit [4de5611](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/4de5611c32d064ca5c1f01d5b35bd16d375d0eb6); all 87 build/test steps completed, including the actual isolated AArch64 SPL link, SRAM/BSS assertions, and the new reachable-callgraph/store audit. Companion H700 boot audit [#1670](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46117217764) also completed successfully.
- Artifact [15252486966](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46117175390/artifacts/15252486966) contains `m1-spl-preflight/m1-spl-reachable-callgraph.json`. Independent inspection confirms root `board_init_f`, **160 reachable functions**, **678 direct call edges**, and **759 reachable store-class instructions**. Nine known initialization/device-I/O paths are reachable: `baseos_early_mmc`, `clock_h6_set_cpu_pll`, `clock_init`, `clock_init_safe`, `find_mmc_device`, `mmc_init`, `mmc_initialize`, `sunxi_sram_init`, and `timer_init`.
- This validates that the parser executes and publishes a useful callgraph report; it does not make the SPL safe. The store list includes stack/frame stores as well as data stores, and the current parser does not resolve effective addresses or decode complete register dataflow. It follows direct `BL` and direct `B` symbol edges, so indirect calls and other control-flow forms can be missed. The report correctly hard-codes `execution_safe=false` and `execution_authorized=false`.
- The #1062 result therefore strengthens the prior #1061 conclusion: the generic `board_init_f` entry path still reaches SRAM/TZPC/timer/clock initialization and the MMC controller path. Do not execute this ELF or interpret successful CI as a hardware qualification.
- Next implementation task: split **link-only SRAM/BSS geometry verification** from the executable M1 pre-entry path. The geometry fixture should link the real M1 scratch/GPT objects and linker symbols but use a minimal fixture entry that performs no generic U-Boot board initialization, no clock/SRAM/TZPC/MMC setup, and no device I/O. Keep the existing executable preflight build as a separate, explicitly unsafe static-analysis target until the minimum required transport and memory ownership are independently reviewed. Do not remove controller initialization from a future real transport based only on grep.
- Safety status unchanged: `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`; no hardware execution, MMIO/SMC, TWI, physical MMC, SRAM/DRAM content access, media write, firmware flash, or payload execution occurred. Artifacts remain non-installable, hardware-unvalidated, and candidate-ineligible.


## 2026-11-28 M1 SPL geometry-only fixture split

- Added a distinct `--geometry-only` mode to `scripts/build-m1-spl-preflight.py`. It builds from a second clean checkout of the same pinned U-Boot v2026.10 commit, so the original executable preflight build and its disassembly audit remain unchanged.
- In geometry-only mode, the replacement `board_init_f` is an inert non-returning branch loop. It does not clear BSS, invoke generic board initialization, configure SRAM/TZPC/timer/clock, initialize MMC, or access a device. This image is a **linker geometry fixture only** and must never be executed.
- Added `h700_m1_spl_geometry_anchor()` to the real GPT adapter translation unit. The geometry link explicitly retains this anchor; its relocations retain the actual 4 KiB `m1_package_scratch` and bounded `m1_boot_partitions` objects while allowing the MMC/GPT execution call graph to be garbage-collected. The builder still verifies their exact linked addresses, sizes, alignment, BSS containment, BSS ceiling and stack separation.
- CI now checks out the same pinned U-Boot source a second time and builds the separate geometry fixture after the existing unsafe preflight SPL/callgraph audit. Its manifest is required to identify `geometry_only=true`, the inert entry behavior, both objects inside BSS, and `candidate_eligible=false`.
- Implementation commits: geometry anchor [23e4e53](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/23e4e5381554f832f7e4ae7848e59f7c09228838), builder mode [5b49f65](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/5b49f65db34c567d40a82e9859924328365b877f), CI integration [33a9259](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/33a925933c709254eec0219fc6468649f8c36db1). The AArch32/AArch64 compile-check workflow passed on the geometry-anchor commit. The full feasibility workflow [run 46118301704](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46118301704) was queued at the time of this status update; earlier intermediate runs were cancelled by the branch concurrency policy as successive commits landed.
- **Acceptance remains pending:** the full CI must prove the geometry-only variant links and its manifest assertions pass. A successful geometry fixture will validate only the linked SRAM/BSS reservation. It will not prove runtime initialization, the M1 capsule protocol, memory ownership, hardware safety, or device boot.
- Safety gates remain unchanged: `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`; no hardware execution, MMIO/SMC, TWI, physical MMC, SRAM/DRAM content access, media write, firmware flash, or payload execution occurred. All outputs remain non-installable and candidate-ineligible.


## 2026-11-28 Geometry fixture CI correction

- The first full run after introducing the geometry fixture, [#1062 follow-up run 46118301704](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46118301704), failed early in the host GPT adapter regression, before reaching either SPL link. The compiler identified a malformed literal backslash-n accidentally inserted into the include directive in `m1_gpt_spl_adapter.c`; this was a source-generation/formatting error, not a GPT regression or geometry-link failure.
- Corrected the include directive in [386590e](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/386590e6752784c6df5b6648663c6975e899d81f). The independent AArch32/AArch64 compile-check and the full feasibility workflow have both been triggered on this correction; at this update they are still running. The geometry-only SPL build itself remains **unverified** until its dedicated CI step executes and passes.


## 2026-11-28 Feasibility #1063 failure: SPL build diagnostics need preservation

- Full feasibility run [#1063](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46118415628), commit [386590e](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/386590e6752784c6df5b6648663c6975e899d81f), failed at step 22, `Build isolated M1 preflight-only H700 SPL and audit linked SRAM BSS`. The failing command was `make ... spl/u-boot-spl.bin`, exit code 2. The independent AArch32/AArch64 compile-check on the same commit passed; H700 boot audit also passed.
- The existing SPL build failure occurred before the new geometry-only link step, so **the geometry-only fixture remains untested**. This is a host-side U-Boot SPL build/link failure, not an RG34XX boot failure and not a hardware test.
- The Actions job log exposed the failing make invocation but did not preserve the useful tail of the captured subprocess output in a readily inspectable form; the uploaded artifact did not contain the SPL-specific failure log. Root cause is therefore **not yet established**. Do not guess at an undefined symbol, linker-script issue, or geometry anchor regression.
- Added failure-diagnostic persistence in [5bdcc0f](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/5bdcc0fc918f1fde56543f6a16cf535794cf8e38): the builder now writes `m1-spl-build-failure.txt` into the artifact and prints the captured diagnostic tail on failure; successful builds preserve `m1-spl-build.log`. Next run must be used to identify the actual compiler/linker diagnostic before applying a functional fix.
- Safety state unchanged: no device execution or hardware I/O; `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`; all generated output remains non-installable and candidate-ineligible.



## 2026-11-28 Feasibility #1068: SPL failure confirmed; diagnostic capture still truncated

- Feasibility [#1068](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46118915460) failed at step 22, `Build isolated M1 preflight-only H700 SPL and audit linked SRAM BSS`, on commit [5bdcc0f](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/5bdcc0fc918f1fde56543f6a16cf535794cf8e38). The independent H700 boot audit [#1674](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46118931220) passed.
- The new artifact now includes `m1-spl-build-failure.txt`, but inspection shows its contents are only the final 12,000 characters of captured make output. That tail contains compile commands and the top-level `make` exit 2, but no compiler/linker diagnostic. The failure remains localized to the host-side U-Boot SPL build; its underlying compiler/linker cause is **not established**. This is not an RG34XX boot failure.
- Corrected the diagnostic truncation in [b3ce9c8](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/b3ce9c835ce5a7a4d0d008b9f5d52656fd4aa2de): the exception now preserves up to 200,000 trailing characters, enough to retain the diagnostic preceding the make summary. The next run must expose the first actionable error before any functional change is attempted.
- Because step 22 failed, callgraph audit and geometry-only fixture steps 23–24 were skipped. The new geometry-only mode remains unverified. The next run should first reveal the failure cause; only then proceed to the independent geometry fixture.
- Safety state unchanged: `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`; no hardware execution, MMIO/SMC, TWI, physical MMC, SRAM/DRAM content access, media write, firmware flash, or payload execution occurred. Outputs remain non-installable, hardware-unvalidated, and candidate-ineligible.



## 2026-11-28 Feasibility #1069: geometry anchor compile failure isolated and corrected

- Feasibility [#1069](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46119388482) failed at step 22 while compiling the generated U-Boot copy of `m1_gpt_spl_adapter.c`. The preserved compiler diagnostic is exact: line 83, `error: expected ')' before '__attribute__'`, at `__attribute__((used, noinline))`; the following line also fails to parse. This is a compile-time attribute/macro collision in the new geometry anchor, not a linker-layout failure, runtime failure, or device boot failure.
- Corrected in [0c96608](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/0c966086d3bd7f98094b2852d4e5e34fed8160ea) by removing the redundant `noinline` attribute and retaining `__attribute__((used))`. The linker explicitly forces the anchor with `--undefined=h700_m1_spl_geometry_anchor`, so `noinline` is unnecessary; the used attribute remains a defensive compiler-retention marker.
- The larger failure log is now working: #1069 exposed the first actionable compiler diagnostic. A new full feasibility run is triggered by the correction; it must prove the original SPL build, callgraph audit, and separate geometry-only link before the geometry split is considered validated.
- Companion H700 audit [#1675](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46119418972) passed on the prior docs commit, but does not validate the SPL correction.
- Safety state unchanged: `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`; no hardware execution or device I/O occurred. Artifacts remain non-installable, hardware-unvalidated, and candidate-ineligible.


## 2026-11-28 Feasibility #1070: M1 SPL build and geometry-only fixture validated by CI

- Full Linkslot LK Rust feasibility workflow [#1070](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46119672687) passed on implementation commit [0c96608](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/0c966086d3bd7f98094b2852d4e5e34fed8160ea). The previously failing host-side U-Boot v2026.10 SPL build now completes; its reachable-callgraph audit and the separate geometry-only SPL link both ran and passed. The independent AArch32/AArch64 compile-check [#695](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46119672706) and H700 boot audit [#1676](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46119698904) also passed.
- The geometry-only manifest confirms a link-only fixture with an inert, non-returning `board_init_f` loop and no initialization/device-I/O path. Against pinned U-Boot v2026.10 commit `5508406582f6f7120dce0c4b3059819a41788db2`, its ELF is 1,062,848 bytes and raw image 43,405 bytes. BSS is `0x30000..0x313a0` (5,024 of 16,384 bytes); the real 4 KiB package scratch is at `0x30340`, 64-byte aligned, and the 832-byte/eight-slot GPT partition table is at `0x30000`. Both objects are verified inside BSS; the stack pointer is `0x52a00`, with 136,800 bytes between BSS end and stack.
- The separate executable preflight SPL also linked successfully (ELF 1,108,296 bytes; raw 47,901 bytes; BSS use 5,032/16,384 bytes). This does **not** authorize execution: its manifest still describes BSS clear plus SRAM/TZPC/timer/clock setup and MMC preflight. The independent reachable-callgraph audit still reports `execution_safe=false` and `execution_authorized=false`; the geometry-only fixture is likewise link/geometry evidence only, not a bootable or executable payload.
- This closes the CI acceptance gap for the geometry-fixture split and the compile regression fixed by [0c96608](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/0c966086d3bd7f98094b2852d4e5e34fed8160ea). It does **not** close G2 BL31-to-BL33 memory ownership, G3 secondary-core reset/RVBAR handoff, or M1-S1 physical LED/proof-of-life gates. `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`; no hardware execution, MMIO/SMC, TWI transaction, physical MMC access, SRAM/DRAM content access, media write, firmware flash, or payload execution occurred. All artifacts remain non-installable, hardware-unvalidated, and candidate-ineligible.
- Next work: use the now-green host-side evidence to continue static analysis and close the G2/G3/M1-S1 evidence gaps before proposing any hardware action. Do not treat a green Actions run as first-boot readiness.


## 2026-11-28 G2 follow-up: BL31 source-page PFN metadata decoded; test corrected

- Revisited the read-only PFN capture published at [reports/rg34xx-dram-pfn-20261127](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-dram-pfn-20261127). The fixed BL31 source window `[0x40000000,0x40000140)` lies in PFN `0x40000`, page `[0x40000000,0x40001000)`. Raw bytes `68 58 00 00 00 00 00 00` are little-endian `kpageflags=0x5868` (not 0x6858); `kpagecount=1`.
- Correct v4.9.170 KPF decode: `KPF_UPTODATE`, `KPF_LRU`, `KPF_ACTIVE`, `KPF_RECLAIM`, `KPF_MMAP`, `KPF_ANON`, and `KPF_SWAPBACKED` are set; `KPF_BUDDY`, `KPF_RESERVED`, and compound/huge flags are clear. The decoder classification is `LINUX_ACTIVE_ANON_MAPPED_PAGE` / `BLOCKED_LIVE_LINUX_PAGE_REQUIRES_EXACT_PRESERVE_RESTORE`. `page_owner` and mapping/process identity are unavailable. This is one Linux post-boot sample, not proof of ownership at BL33 entry or BL31 save/restore lifetime.
- An attempted test correction briefly used the wrong byte order. CI [BL31 memory audit #137](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46121046859) caught the incorrect expected classification before the decoder step ran. The test has now been restored to the little-endian interpretation in [9f17b90](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/9f17b90f6a1affaad78f0de9d439d67aba11c97a); the ownership note was corrected in [1c2effd](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/1c2effdbbb52b0adeee0a67bdcc3ff2e0a73e7fd). Do not use the failed #137 run as validation.
- Keep the full 4 KiB page excluded from LK scratch, stack, heap, page tables, DMA and emulator allocations. This strengthens the case that the page is live in Linux, but does not identify its owner at earlier boot stages. The H700 audit smoke workflow [#1677](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46120467263) passed host tests and RVBAR selector parity for all eight low-three-bit values; the actual selector read at `0x03000024` remains unavailable because the earlier read-only probe returned `EFAULT`.
- Next: let the corrected BL31 memory audit run decode the captured PFN and then trace the matched BL31 snapshot/restore code and Linux mapping source using captured evidence or a reviewed read-only kernel path. Do not repeat userspace `/dev/mem`. Keep G2/G3/M1-S1 gates closed: `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`; no hardware access, media writes, firmware flash, or payload execution.


### G2 audit #138: little-endian PFN decode and memblock cross-check passed

- After correcting the test's byte-order expectation, BL31 static memory audit [#138](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46121129729) passed all steps, including the v4.9.170 KPF unit tests and decoding the real captured PFN report. The emitted result is `LINUX_ACTIVE_ANON_MAPPED_PAGE`, `kpageflags=0x5868`, `page_mapcount=1`, gate `BLOCKED_LIVE_LINUX_PAGE_REQUIRES_EXACT_PRESERVE_RESTORE`.
- The same run's memblock classifier labels the page `UNRESERVED_SYSTEM_RAM` / `BLOCKED_SOURCE_PAGE_NOT_RESERVED`. This does not contradict the PFN result: memblock reserved ranges do not encode current page allocator ownership. The cross-check demonstrates why “not reserved by memblock” cannot be treated as “free”.
- Updated [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md) in [62af8cc](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/62af8cc6976d4e2b78b2f665866dd7e9e514a8d5). Keep the full page excluded from all LK allocations; mapping owner, exact BL31 snapshot/restore lifetime, and CPU/DMA/cache interactions remain unproven. No hardware access or execution was performed.


### G2 refinement from matched BL31 standby disassembly (#138 artifact)

- Artifact [15253558130](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46121129729/artifacts/15253558130) contains the matched BL31 PSCI callback disassembly and report. The special power-entry handler `0x48001a04` copies a 0x9410-byte standby image to SRAM `[0x20000,0x29410)`; before DRAM retraining it snapshots two 0x140-byte DRAM windows (160 words total) into scratch `0x4801b400`, then copies them back after retraining/resume.
- First window is `[0x40000000,0x40000140)`, overlapping the active anonymous-mapped PFN `0x40000`. Second window is derived as `DRAM_BASE + total_DRAM_capacity/2`; 1 GiB capacity implies a candidate `0x60000000`, but actual context/geometry selector is unknown. The scratch buffer lies in the reserved monitor carveout, outside the 107,216-byte monitor file payload.
- Runtime selection remains conditional: geometry context `+0x18/+0x1c/+0x7a`, possible preset bit31, controller-ID threshold logic, context `+0x4 == 4` alternate branch, and resume gate `0x48011320` are not observed on the target. The static audit proves the code path exists, not that this unit took it or that the full save/restore sequence completed.
- Updated [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md) in [877f910](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/877f910573902a292c26877a85d4a25d31741aa0). This narrows the G2 question from “unknown use of page” to “live Linux page covered by a matched BL31 snapshot/restore design, with runtime gates and completion unverified”. Keep the page and the midpoint window excluded from LK allocations; do not use the existence of this preservation path as memory approval.
- Next evidence target: recover actual DRAM geometry/context values and determine the selected suspend/resume branch from already captured traces or a reviewed passive read-only source. Do not invoke PSCI suspend, issue SMC, probe MMIO, or read/write DRAM contents to force the path.


### Documentation CI after G2 audit update

- H700 boot-audit smoke workflows [#1684](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46121505344) and [#1685](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46121541250) both passed after the memory-ownership and BL31 snapshot/restore documentation updates. These validate the host-tool/unit-test and Windows wrapper paths; the substantive BL31 static analysis and PFN decoding result is [#138](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46121129729).
- Current G2 status: matched BL31 binary contains the conditional two-window SRAM snapshot/restore path; one source page is a live anonymous-mapped Linux page in the captured runtime; exact runtime geometry/context, branch selection, and successful save/restore completion remain unknown. G3 still lacks live CPU reset/RVBAR state. M1-S1 still lacks post-BL31/pre-BL33 physical LED path proof. All three gates remain closed.


### 2026-11-28 G2: Boot0 geometry cross-check narrows BL31 midpoint window

- Read-only 2026-11-27 inventory's two checksum-valid Boot0 copies agree on the primary DRAM geometry fields: type 8 (LPDDR4), parameter clock field 672 MHz, word[6] `0x30fa`, word[7] `0x04000000`. The debug byte differs between copies (3 vs 0); its selection semantics are not inferred.
- Cross-checked against the pinned BaseOS native SPL encoder in `experiments/h700-boot-speed/spl/dram-select.c`: word[6] encodes rows/columns and word[7] encodes MiB capacity, rank count, and bus width. The decoded geometry is 15 rows, 10 columns, one rank, 32-bit bus, 1024 MiB. The live Linux memblock independently reports `[0x40000000,0x80000000)`, exactly 1 GiB.
- Therefore the matched BL31 midpoint formula `DRAM_BASE + total_DRAM_capacity/2` yields a specific second-window candidate `[0x60000000,0x60000140)`, not merely a generic 1 GiB assumption. This remains an address derivation, not confirmation of the actual BL31 runtime selector or branch.
- Added fail-closed decoder and unit test: [audit-rg34xx-boot0-dram-geometry.py](../scripts/audit-rg34xx-boot0-dram-geometry.py), [test-audit-rg34xx-boot0-dram-geometry.py](../scripts/test-audit-rg34xx-boot0-dram-geometry.py). Added both to [h700-bl31-memory-audit.yml](../.github/workflows/h700-bl31-memory-audit.yml). Detailed caveats recorded in [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md), commit [f153387](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/f153387a852cb06bfa39602d06cee3c90418d83b).
- Next: verify the new decoder against the pinned report files in CI, then statically trace the BL31 geometry/context producer and exact resume-gate dataflow. Do not infer active Boot0 copy from debug byte, infer live DRAM clock from the parameter clock field, or force suspend/resume/SMC/MMIO. Both 0x40000000 and 0x60000000 windows remain excluded from LK allocation; G2/G3/M1-S1 gates remain closed.


### G2 Boot0 geometry audit CI result

- BL31 static memory audit [#141](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46122355821) passed every step, including the new Boot0 geometry decoder against the pinned 2026-11-27 primary/secondary Boot0 reports and live Linux memblock JSON. Artifact [15254247497](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46122355821/artifacts/15254247497) confirms both copies decode to LPDDR4 / 15 rows / 10 columns / one rank / 32-bit bus / 1024 MiB; candidate second BL31 source window is `[0x60000000,0x60000140)`.
- Initial audits #139 and #140 failed only because the new test attempted to import a hyphenated script filename as a Python module. Test harness now loads it explicitly with `importlib.util`; #141 passed. The production decoder and test assertions both passed.
- Latest H700 boot audit smoke [#1688](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46122306039) also passed. Geometry note updated in [f6e1c9f](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/f6e1c9f9e03224f22754e920859cd05b7bace117).
- Remaining G2 question is now narrowed to runtime BL31 context: resume helper reads context pointer from SRAM global slot `0x48014668`; its geometry fields at context offsets `+0x18`, `+0x1c`, and halfword `+0x7a` feed selector helper `0x4800c730`. The second-window copy address is computed from context `+0x1c` (after selector adjustment) as `0x40000000 + ((value >> 17) << 20)`. The actual context contents and selected branch are not captured; the Boot0 word[7] match does not by itself prove the context field equals `0x04000000`.
- The resume gate helper `0x48011320` calls clock-restore helper `0x480111a0` and context helper `0x480115c0`; when context `+0x04 == 4`, it additionally reads the first word at `0x40000000`. Its return value controls whether the later 0x280-byte DRAM copy loop proceeds. The gate's live return value and whether the branch ran remain unknown.
- Next: trace the SRAM context pointer initialization and context-field producer statically, including how `0x48014668` is populated, and map the `+0x18/+0x1c/+0x7a` fields back to saved controller geometry. Keep both DRAM source pages excluded; no SMC/MMIO/suspend/resume/DRAM access. G2/G3/M1-S1 gates remain closed.


### G2 static follow-up: BL31 runtime context producer not yet found

- Full linear host disassembly of the exact matched 107,216-byte monitor payload confirms resume code loads a context pointer from global slot `0x48014668` at several callsites, including `0x4800c434`, `0x4800c4e0`, `0x4800c63c`, and `0x48013480`. The retention worker `0x48011938` consumes context offsets `+0x18`, `+0x1c`, `+0x78`, and `+0x04`.
- Raw monitor file bytes at offset `0x14668` are `02 00 00 00 00 00 00 00`, not a valid context pointer. Full linear disassembly shows direct reads but no obvious direct store to the same slot. The runtime pointer must be established by a path not recovered by this direct scan, an external setup, runtime relocation, or a different mapping; none is proven. The static file value must not be treated as the live pointer.
- The second-window address expression is `0x40000000 + (((context[+0x1c] >> 17) & 0x7fff) << 20)`. When context bit31 is clear, helper `0x4800c730` computes a selector using context `+0x18/+0x1c/+0x7a`; controller IDs `0x5c00` and `0x5d00` have special branches. The 0x60000000 candidate requires a final selector of 512 MiB, but the context and helper result are not captured.
- Restore gate `0x48011320` calls `0x480111a0` and `0x480115c0`; in context mode `+0x04 == 4`, it additionally reads the first word at `0x40000000`. Caller `0x48011974` skips the copy worker if the gate returns zero. Live context, controller ID, gate return, and branch execution remain unknown.
- Detailed static findings added to [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md) in [52d45b0](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/52d45b05ab0521917c83c258046aa540276b73ba). Next static task: locate how the global context slot is initialized, including any upstream SRAM resume stub or external handoff; no new device probe, SMC, MMIO, or suspend/resume is warranted.


### H700 documentation CI confirmation

- H700 boot audit tools [#1691](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46123014893) and [#1692](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46123047272) both passed after the BL31 context-pointer/selector analysis was recorded. Both Windows PowerShell and host-tools jobs are green; this validates documentation-linked smoke paths, not hardware state.


- H700 boot audit tools [#1693](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46123133209) passed after this status update; both Windows PowerShell and host-tools jobs are green. This is the latest smoke confirmation for the current documentation state.


### G2 follow-up: whole-payload context-slot scan (#142)

- Added [audit-rg34xx-bl31-context-slot.py](../scripts/audit-rg34xx-bl31-context-slot.py), identity-pinned to the matched 107,216-byte BL31 payload, and integrated it into the BL31 memory audit workflow. It scans aligned overlapping AArch64 windows for reads/writes to slot `0x48014668`; results are heuristic candidates, not a call graph.
- BL31 static memory audit [#142](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46123660057) passed all steps. Artifact [15254871932](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46123660057/artifacts/15254871932) reports 10 candidate reads and zero candidate direct writes to the slot.
- Candidate read instructions include `0x4800c434`, `0x4800c4e0`, `0x4800c63c`, `0x4800c654`, `0x4800c65c`, `0x48013408`, `0x48013e1c`, `0x48013e24`, `0x48013f20`, and `0x48013ff0`. These are overlapping-window decode leads; not all have yet been validated as reachable code.
- The scanner cannot resolve indirect stores, loaded-pointer writes, exception paths, runtime relocation, or external initialization. Zero direct-write candidates is not proof that no writer exists. Runtime context pointer remains UNKNOWN; the static qword value 2 at file offset `0x14668` is not a valid substitute.
- Detailed evidence and limitations are recorded in [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md), commit [cce3d67](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/cce3d67a95a812331b9c8a8caa0c4872c510f25b).
- Next static step: identify initialization of the monitor data region and possible Boot0/SPL/secure-entry handoff writes, then validate candidate references against reachable function boundaries. No hardware probe, MMIO/SMC, DRAM access, suspend/resume, or payload execution. G2/G3/M1-S1 remain open; `H700_MEMORY_LAYOUT_APPROVED=false`.


### G2 context-slot call-chain validation and refined scan (#143–#145)

- Static callback disassembly now explicitly ties the slot to the retention worker: `0x4800c654` loads the qword at `0x48014668` into x0; `0x4800c658` calls `0x48011b58`; that calls `0x48011b10`, which forwards x0 as x1 to `0x48011938`. The worker consumes context offsets `+0x18/+0x1c/+0x78/+0x04` and executes the retention gate/selector/copy logic.
- The static slot qword remains `2`, not a valid pointer. The call-chain proves the value is consumed as a context pointer on this path, but does not prove the runtime value, its producer, or path execution.
- The separate controller-object pointer at `0x48015548` is initialized by helper `0x48013ac8` and published at `0x48013b14`. It is a different slot and must not be confused with `0x48014668`.
- Refined [audit-rg34xx-bl31-context-slot.py](../scripts/audit-rg34xx-bl31-context-slot.py) to propagate literal/image-backed loads and report direct-call argument references. Final BL31 static audit [#145](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46124497685) passed. Artifact [15254429553](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46124497685/artifacts/15254429553) reports 11 candidate reads, zero direct-write candidates, and zero direct-call argument references to the slot. The separate descriptor audit's alias and bounded-CFG writer scans also report no direct store candidates. These are bounded scans, not proof that no indirect/helper/external writer exists.
- Full details recorded in [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md), commit [80658f2](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/80658f2387c42061e838c37422d0e261a6cf795a).
- Next: inspect monitor startup/registration data and secure-entry producer paths for a writer or external handoff to `0x48014668`; do not infer from the analogous `0x48015548` store. Keep DRAM windows excluded; no device MMIO/SMC, DRAM content access, suspend/resume or payload execution. G2/G3/M1-S1 remain open and `H700_MEMORY_LAYOUT_APPROVED=false`.


### G2 producer-path narrowing: Boot0/SPL direct-literal check

- Reviewed the exact Boot0 binaries in BL31 audit #145 artifact [15254688500](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46124497685/artifacts/15254688500). Primary logical image: 128 KiB, SHA-256 `bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f`. Secondary logical image: 64 KiB, SHA-256 `4701a85e4db7e8339b91b92f691a83cc954ec63a09369087c2e41d80eafa9f5d` (capture backing file is 128 KiB).
- Neither logical image contains direct little-endian literals `0x48014668`, `0x48014000`, or `0x00014668`. Primary contains a single `0x48000000` literal at offset `0x1b7cc`; adjacent words `0x1a2d0/0x48000000` match the monitor extent length and load base. Secondary has no `0x48000000` literal.
- Pinned BaseOS SPL source audit already limits explicit monitor patches to `+0x24..+0x2b`, `+0x30..+0xaf`, and `+0xb0..+0xb3`; no direct patch to `+0x14668`.
- This excludes only obvious direct-literal Boot0/SPL writes. Computed address construction, generic relocation/copy behavior, indirect writes, and runtime BL31 initialization remain open. Do not call this proof of no Boot0 writer.
- Details added to [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md), commit [dd47238](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/dd4723802d788a670acb8624b266799fd2d58f73). Next static task: inspect Boot0's monitor extent-copy routine and BL31's initialization/registration paths for computed writes or indirect handoff of the context pointer. G2/G3/M1-S1 remain open; no device access or payload execution.


### Boot0 mixed-ISA reverse-engineering correction

- A second-pass disassembly used the correct AArch64 decoder for the Boot0 tail. The earlier ARMv7 decode had misclassified AArch64 words: `0x1b6d0` is `cmp x1,x0`, not `BL`; `0x1b7d4` is data, not a monitor-copy helper.
- The AArch64 function at `0x1b6f4` dispatches through a function pointer stored at descriptor offset `+0x20`. The data pair at `0x1b7c8/0x1b7cc` remains `0x1a2d0/0x48000000`, matching BL31 size/base, but no consumer or copy semantics have yet been established.
- The prior tentative claim that Boot0 calls a monitor-loader helper at `0x1b7d4` is explicitly withdrawn. Full correction and evidence are in [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md), commit [8fe153f](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/8fe153f4f8b69712938a4850f18b95901a145394).
- Next: identify how/if Boot0 reaches the AArch64 region, decode descriptor layout `0x1b740..0x1b7f8`, and find the actual consumer of the size/base pair. No execution or device access. G2/G3/M1-S1 remain open; `H700_MEMORY_LAYOUT_APPROVED=false`.


### Boot0 AArch64 descriptor-dispatch structure

- Correct AArch64 decoding identifies a generic registry at `0x1b694..0x1b73c`: it walks 0x28-byte records from `0x1c688` to `0x1c6b0`, compares the requested key with the record field at `+0x08`, and dispatches through the callback pointer at `+0x20`.
- The captured single record has key `0x4d` and callback pointer `0x2984c`. This is a concrete descriptor-dispatch mechanism, but not yet a BL31 loader. No validated control-flow link from the eGON AArch32 entry into this AArch64 region has been established.
- The `0x1a2d0/0x48000000` size/base pair remains an unconnected data candidate. Next: inspect callback `0x2984c`, locate the AArch64 region's entry/caller, and find actual code references to the size/base descriptor. Do not infer a monitor copy or context-slot writer.
- Full notes are in [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md), commit [6d42e46](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/6d42e46618bef158b5d235019919285675ac7d6d). Hardware safety gates remain unchanged.


### Boot0 callback pointer / eGON address-space reconciliation

- The primary eGON header reports `run_addr=0x20000` and public-header size `0x30`; mapping the registry callback `0x2984c` against executable base `0x20030` gives file offset `0x981c`.
- File offset `0x981c` is a coherent Thumb-2 function entry. It saves r4-r11/lr and consumes a structured context. The adjacent callback value `0x298b0` maps to `0x9880`, inside that same function rather than a separate entry.
- This is a useful address correlation, not a validated call edge. The registry dispatcher at `0x1b694..0x1b73c` decodes as AArch64 and ends in `br x16`, whereas the target bytes decode as Thumb-2 and the pointer low bit is clear. AArch64 BR cannot switch to Thumb; the code/data ownership or state-transition path remains unresolved.
- The actual eGON entry is AArch32: initial branch to `0x400`; early code saves CPU state, checks CPSR/SCTLR, masks interrupts, sets SP `0x57000`, calls `0x4e0`, and later accepts selected return addresses `0x21000`/`0x31000` before indirect transfer through `0x494` (value `0x50000`). This still does not establish a reachable edge to the AArch64 registry.
- Detailed evidence and caveats: [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md), commit [88d1410](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/88d1410e304783e199e7aa4fc9ce5506934656c4).
- Next: trace the AArch32 entry's continuation and the AArch64 registry's code/data boundary; look for a real state-switch or separate execution context. Do not label this a BL31 loader or context-slot producer.


### Boot0 AArch32 SRAM resume path traced

- Traced the eGON AArch32 entry at `0x400` through helper `0x4e0`: validates image size/signature/checksum, performs peripheral read/write/readback checks, and returns only `0x21000` or `0x31000` on accepted paths; the entry then transfers through `0x494` (value `0x50000`).
- The continuation at `0x768` copies a 68-byte AArch32 restore routine from `0x20840..0x20884` to SRAM `0x50000`, then copies 72 zero bytes from `0x20498..0x204e0` to `0x52000`.
- The copied routine restores a caller-supplied memory range to `0x20000`, restores CPSR and general-register/SP/LR state from `0x52000`, and branches to `0x20000`. This is a concrete Boot0 SRAM resume/restore mechanism.
- It is distinct from BL31 context slot `0x48014668`: this path uses SRAM `0x50000/0x52000`, explicitly zeroes its 72-byte context block, and no bridge to the BL31 context pointer is established. Full analysis: [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md), commit [fcfddba](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/fcfddba03f6b6c6897f27f1ff4e32521697ac61d).
- Next: identify the peripheral handshake in `0x4e0`, locate any later population of `0x52000`, and determine whether any state-transfer path reaches BL31. Keep G2/G3/M1-S1 gates closed; no device interaction or payload execution.


### Boot0 retained-image resume gate narrowed

- Helper `0x4e0` validates the eGON image size/signature, reads status `0x070001f8`, reads registers formed from `0x07000138/0x0700013c`, and checks checksums over `[0x21000,0x31000)` (64 KiB, expected `0x8fb6d562`) and `[0x31000,0x3d000)` (48 KiB, expected `0xa53ccffb`).
- It performs ordered write/readback operations through addresses derived from `0x030090b4`, then selects a retained image at `0x21000` or `0x31000`. The SRAM restore trampoline copies that image to `0x20000`, restores the separate 72-byte context at `0x52000`, and jumps to the Boot0 base.
- The peripheral protocol is not identified; do not label it RTC/PMIC/mailbox/PSCI. Literal scans find no direct `0x48014668` or `0x52000` literal; computed address formation and other writers remain possible.
- This is evidence for a Boot0 retained-image resume protocol, not evidence that it initializes BL31's context slot. Full notes: [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md), commit [69e23bb](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/69e23bb757f9a4fec3ded02310999b917801517e).
- The latest preceding CI #1709 passed for commit `03adbe0`; this new documentation commit will trigger another audit. G2/G3/M1-S1 remain open; no device interaction or payload execution.


### Boot0 callback mapping correction (2026-11-28)

- Rechecked the raw 128 KiB primary Boot0 bytes against the eGON whole-image load base `0x20000`. The Thumb function prologue is at file offset `0x981c` / runtime `0x2981c`; descriptor values `0x2984c` and `0x298b0` map to file offsets `0x984c` and `0x98b0`, both interior locations, not independent function entries. This supersedes the earlier `0x20030`-based callback mapping.
- The AArch64-looking registry code at file offset `0x1b694` computes a table at runtime `0x3c688` / file offset `0x1c688`, and that table contains key `0x4d` plus callback `0x2984c`. The address relationship is internally consistent under the Boot0 image mapping, but the dispatcher uses `br x16` and the target region is Thumb code; no valid ISA transition or reachable AArch32-to-AArch64 path is proven.
- The Boot0 registry/code windows are not byte-identical to the corresponding candidate windows in the live TOC1 monitor. Do not identify this registry with BL31's context slot or callback machinery. Full correction: [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md).
- G2/G3/M1-S1 remain open; no device interaction or payload execution.


### Boot0 AArch32-to-AArch64 reachability follow-up (2026-11-28)

- Dual-mode disassembly confirms a coherent AArch64 code tail beginning at Boot0 file offset `0x1b548`; the descriptor lookup/dispatch is at `0x1b694..0x1b73c`, with its table at `0x1c688` (whole-image runtime addresses add `0x20000`).
- The eGON entry at file offset `0x400` and its resume trampoline at `0x50000` are AArch32. The entry validates state, calls helper `0x4e0`, then loads PC from `0x494` (captured value `0x50000`). The trampoline restores CPSR/register/SP/LR and branches back to `0x20000`. No AArch64 exception-return or state-switch path was found in this resume sequence.
- A full-image direct-word scan found no literal pointers to A64 candidate runtime addresses `0x3b548/0x3b580/0x3b640/0x3b694/0x3b6f4/0x3b728` or table `0x3c688`. Computed pointers, external consumers, and indirect loader paths remain unexcluded.
- The A64 descriptor callback values still land inside Thumb code with low bits clear, and `br x16` does not change ISA state. Treat the A64 tail as an unresolved embedded region, not a proven Boot0 handoff or BL31 loader. Full analysis: [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md).
- Continue BL31 indirect-writer tracing for `0x48014668`; G2/G3/M1-S1 remain closed. No hardware interaction or payload execution.


### BL31 indirect callback writer found (2026-11-28): 0x4803c048 is not the G2 slot

- The callback loaded and conditionally invoked by `0x48007554` is stored at `0x4803c048` (`0x480075cc..0x480075d8`). Its direct writer is `0x480075e8`: `ADRP x1,0x4803c000; STR x0,[x1,#0x48]; RET`.
- Caller `0x4800a134` reaches the writer at `0x4800a1a0`, passing `x0=0x4800a504`. That target is an AArch64 per-CPU/exception-context routine using MPIDR_EL1 and TPIDR_EL3. The installer function's cold-boot reachability/timing still needs tracing; the slot is inside the startup-cleared BSS interval.
- This resolves the separate callback slot `0x4803c048`, **not** the G2 DRAM-retention context slot `0x48014668`. No G2 writer is established. Full evidence and disassembly notes: [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md).
- Continue by tracing the entry/callers of `0x4800a134` and expanding full-payload indirect writer analysis for `0x48014668`. G2/G3/M1-S1 remain closed; no device interaction or payload execution.


### BL31 callback installer call-site follow-up (2026-11-28)

- Full-image AArch64 disassembly finds no direct BL/B immediate targeting installer function `0x4800a134`. The function performs MPIDR/per-CPU initialization and installs callback `0x4800a504` into `0x4803c048` only on its initialized branch.
- A raw qword at `0x48018d00` equals `0x4800a134`, but it is not yet tied to a live consumer. Known MIDR lookup `0x48015cd0` scans `[0x48018d10,0x48018d70)`, excluding that qword; known exception dispatch based at `0x48018ca8` indexes 0x20-byte records and likewise does not calculate `0x48018d00`.
- Therefore the callback writer/value remain confirmed, but installer reachability is unresolved. This does not establish a G2 writer for `0x48014668`. Full detail: [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md).
- Next investigate the owner/consumer of the data record at `0x48018d00`, then resume G2 indirect writer tracing. G2/G3/M1-S1 remain closed; no device interaction or payload execution.


### BL31 exception-descriptor table clarified: 0x4800a134 is not the dispatched handler

- The exception entry at `0x48015884` indexes 0x20-byte records rooted at `0x48018ca8`, but loads and calls only each record's first qword (`LDR x15,[x11,w10,uxtw]`; `BLR x15`).
- Record `0x48018ce8` contains qwords `0x4800a29c, 0x13f32, 0x480184e1, 0x4800a134`. The pointer `0x4800a134` is the fourth field (+0x18), not the dispatch target. Its consumer/meaning is still unknown.
- This corrects the prior interpretation: the data reference at `0x48018d00` does not show that the exception dispatcher calls the callback installer. Runtime installation of `0x4803c048` remains unproven; G2 slot `0x48014668` still has no established writer. Full evidence: [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md).
- Continue by finding a separate consumer for the fourth descriptor field and expanding G2 indirect-writer analysis. G2/G3/M1-S1 remain closed; no device interaction or payload execution.


### G2 whole-image direct-store cross-check (2026-11-28)

- Rechecked the exact 107,216-byte BL31 image (SHA-256 `676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31`) for the exact effective address `0x48014000 + 0x668 = 0x48014668`.
- All identified static references to this operand are loads. No direct `STR/STP/STUR/STLR/STXR/STLXR [xN,#0x668]` store was found in the whole-image disassembly. The image qword at the slot is initially `2`; nearby structured data and indexed loads are not sufficient to identify its runtime owner.
- This does not exclude computed aliases, helper/memcpy writes, register-indexed or stack-mediated stores, exception/secure-call setup outside BL31, or writes by another boot stage. G2 remains unresolved; runtime context pointer remains UNKNOWN. See [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md).
- Continue with cross-stage writer provenance and alias/data-flow analysis. No hardware interaction or payload execution occurred.


### 2026-11-28 G2 correction: relocation resolves the standby context pointer

- Revisited the successful BL31 audit #145 artifact `15254429553`, specifically `TWI-SELECTOR-RELOCATION.md` and its JSON. The standby image `[0x4800c220,0x48015630)` is copied to SRAM at `0x20000`; the code at source PC `0x4800c430` is then executed at `0x20210`.
- Its `ADRP x20,0x48014000; LDR x19,[x20,#0x668]` pair is rebased. Runtime ADRP page is `0x28000`, so the actual load address is `0x28668`. Its source-equivalent backing qword is `0x48014888`, whose value is `0x20000`.
- **Correction:** `0x48014668` and its raw qword value `2` were the unrelocated source-coordinate interpretation, not the live effective slot for the copied standby path. The runtime context base is statically resolved to SRAM `0x20000`; the previous “unknown writer for 0x48014668” blocker is superseded for this path.
- The same audit derives FDT `twi5` → R-TWI0 `0x07081400`, and traces selector staging from `0x4803cbb4` to SRAM context+`0xa4` at `0x200a4`. This is static dataflow only; it does not prove a PMIC transaction or that standby/resume executed on hardware.
- Updated `scripts/audit-rg34xx-bl31-context-slot.py` to label its old `0x48014668` hits as unrelocated source-coordinate candidates, validate the relocated slot `0x28668`, and assert the backing qword `0x48014888 == 0x20000`.
- **G2 is narrowed, not cleared:** active retention path selection, context geometry values, DRAM save/restore timing, CPU/DMA/cache quiescence, and ownership through Linux resume remain unproven. Keep DRAM source pages excluded and `H700_MEMORY_LAYOUT_APPROVED=false`. No hardware access or payload execution occurred.

See [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md) for the detailed correction and evidence boundary.


### 2026-11-28 SPL parameter handoff traced into BL31 retention geometry

- The pinned BaseOS SPL source now closes the parameter provenance chain: `baseos_selected_vendor_params()` returns the 32-word `trained_vendor_params` array; `baseos_normalize_vendor_params()` copies the selected candidate and normalizes word[6] (rows/columns) and word[7] (capacity/rank/bus) from the successfully trained configuration and detected size. Word[30] remains from the selected candidate.
- `native-boot.c` copies those 128 bytes to monitor +0x30. BL31 power-entry `0x48001a04` copies monitor +0x30..+0xaf over SRAM context base `0x20000`. The context fields read by the retention worker therefore map to parameter words [6], [7], and [30], not to the unpatched monitor placeholder bytes.
- Conditional calculation for captured Boot0 primary LPDDR4 candidate: word[6]=`0x30fa`, word[7]=`0x04000000`, word[30]=`0x02008c60`. Under the matching 1-GiB trained-geometry assumption, helper `0x4800c730` returns `0x400`; caller produces context +0x1c=`0x04000000`; the second DRAM source window resolves to `0x60000000`. This matches the independent 1-GiB capacity/midpoint evidence.
- **Boundary:** this is a conditional source/binary derivation, not a live trace. The selected Boot0 copy/candidate, actual native SPL training result, final patched 128-byte block, and BL31 suspend execution have not been observed together. G2 remains blocked on live retention selection/timing and CPU/DMA/cache ownership; keep `0x40000000` and `0x60000000` source pages excluded and `H700_MEMORY_LAYOUT_APPROVED=false`.
- Next: add a host regression joining the captured Boot0 parameter report, SPL normalization, BL31 patch/copy, relocation, and selector arithmetic. Full detail: [h700-bl33-memory-ownership.md](h700-bl33-memory-ownership.md).


### BL31 retention-parameter regression completed: CI #150

- Added `scripts/audit-rg34xx-bl31-dram-context.py` plus unit test and integrated both into the BL31 static memory workflow. It checks the exact monitor instruction words, pinned BaseOS SPL parameter producer/copy path, captured Boot0 primary candidate, and helper arithmetic.
- [CI #150 passed](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46134616497); artifact [15259340938](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46134616497/artifacts/15259340938) records the conditional result: helper `0x400`, context word[7] `0x04000000`, second DRAM save/restore window candidate `[0x60000000,0x60000140)`.
- Initial #147–#149 attempts caught incorrect test expectations/instruction pinning; corrected in #150. The successful report explicitly says no live parameter block or suspend/resume execution was captured and memory layout remains unapproved.
- Static provenance/regression is complete. G2 remains open for actual branch selection, live parameter values, save/restore lifetime, and CPU/DMA/cache quiescence. Keep both DRAM source pages excluded; no device interaction or payload execution.

## 2026-11-28 Decision record: M1 primary / G2 safety parallel

### Approved decisions

1. **M1 isolated SPL final-link verification is the primary implementation line.** Complete the real linked ELF, linker-map, symbol, BSS, stack/heap geometry, and reachable-callgraph audit before attempting any executable image integration.
2. **G2 DRAM ownership remains a parallel safety line.** It must independently establish save/restore lifetime and Linux/DMA/cache ownership; it is not allowed to borrow confidence from M1 link success.
3. Planning allocation: **60% M1 / 40% G2**. This is a work-priority split, not a progress percentage or safety waiver.
4. Preserve the milestone gates:
   - **M0 — final SPL ELF/link geometry:** actual ELF + MAP + NM + disassembly; exact BSS/scratch/GPT table/heap/stack intervals; callgraph and store review.
   - **M1 — preflight-only path:** read-only GPT/MMC source path and fail-closed package planner, with no payload execution or media writes.
   - **M2 — H700 BL33/LK minimal boot:** only after M0/M1 evidence and a separately reviewed boot handoff contract.
   - **M3 — Linkslot minimal UI:** only after stable BL33/LK display/input/storage prerequisites.
5. The geometry-only link fixture is evidence about symbol retention and SRAM geometry only. It must never be described as an executable preflight.
6. No change to the normal BaseOS boot path or raw-capsule-v2 semantics is authorized. M1 remains an isolated one-shot path.
7. No DRAM destination copy, payload execution, Boot0/TOC1/TF write, firmware flashing, or installable candidate is authorized by this decision.

### Implementation order

- **M1-A (now):** run and preserve the isolated U-Boot v2026.10 SPL ELF/MAP/NM/disassembly build; make the CI contract assert the manifest's non-installable state, exact linked scratch/GPT symbols, BSS ceiling, stack separation, and absence of named write/PMIC/TWI/normal-board-init symbols. Review reachable stores and indirect-call limitations manually.
- **M1-B:** keep board_init_f limited to BSS clear, required SRAM/TZPC/timer/clock setup, and read-only MMC/GPT preflight. Treat even read-only device access as unvalidated until the final linked path is audited.
- **G2-A (parallel):** extend read-only Linux 4.9.170 ownership evidence around PFN 0x40000 and the corresponding 0x60000000 source window. Obtain repeated/independent page metadata and kernel memory-map evidence; do not infer free memory from one sample.
- **G2-B:** reconstruct BL31 save-before-training / restore-before-Linux-resume timing and prove CPU, DMA, and cache quiescence. Any Linux-mapped source page remains excluded unless exact preservation/restoration is demonstrated.
- Update this status file with every CI result and every change to the safety gate.

### Current evidence and safety interpretation

- The 2026-11-28 PFN audit decoded the captured Linux 4.9.170 sample for PFN 0x40000 as LINUX_ACTIVE_ANON_MAPPED_PAGE, kpageflags=0x5868, page_mapcount=1; its gate is BLOCKED_LIVE_LINUX_PAGE_REQUIRES_EXACT_PRESERVE_RESTORE. This is affirmative evidence that the sampled page was actively mapped by Linux at capture time. It is not evidence that BL31 actually overwrites it, but it rules out treating it as free scratch.
- The same audit classifies the 0x60000000 source page as UNRESERVED_SYSTEM_RAM / BLOCKED_SOURCE_PAGE_NOT_RESERVED; absence of a reserved marker is not ownership proof.
- M1 builder currently has an actual-link path that emits m1-spl-preflight.elf, .bin, .map, .nm.txt, disassembly, and a manifest; it asserts the aligned 4 KiB scratch and bounded GPT table are inside linked BSS, BSS stays within 16 KiB, BSS does not overlap the initial stack, and named write/PMIC/TWI/normal-board-init symbols are absent. The reachable-callgraph audit follows direct BL and direct tail B edges only; indirect calls and complete MMIO/SMC semantics remain manual-review blockers.
- The M1 link result, even when CI passes, is not a hardware-qualified boot result. The builder's sentinel intentionally prevents firmware payload reads; device_io_performed=false, payload_executed=false, installable=false, and candidate_eligible=false remain mandatory.
- Safety flags remain unchanged: H700_MEMORY_LAYOUT_APPROVED=false, hardware_applied=0, candidate_eligible=false. Keep the DRAM source pages 0x40000000 and 0x60000000 excluded.

### Active next tasks

1. Resolve the latest M1 CI build result and preserve the complete ELF/MAP/NM/disassembly artifact set; if the link fails, fix only the isolated disposable U-Boot overlay and add a regression for the exact failure.
2. Add a strict CI post-link contract over the generated manifest and callgraph so a successful compile cannot silently weaken the fail-closed flags or omit audit outputs.
3. For G2, extend the read-only collector/report to capture the relevant memory-map/zone/page allocator evidence for both candidate pages, while keeping the report non-authorizing and avoiding reads of DRAM contents.
4. Do not advance to M2 until M0 and M1 gates are explicitly closed by evidence review.

### 2026-11-28 implementation update: post-link gate and dual-page G2 collector

- Added `scripts/audit-m1-spl-linked-contract.py` and `scripts/test-audit-m1-spl-linked-contract.py`. The post-link gate checks the real (non-geometry-only) M1 ELF/BIN/MAP/NM/disassembly artifacts; exact linked BSS usage and 16 KiB ceiling; aligned 4 KiB scratch and bounded eight-entry GPT table inside BSS; BSS/stack and heap/stack separation; fail-closed manifest flags and sentinel; and non-authorizing callgraph metadata.
- Wired the gate immediately after the actual SPL link and reachable-callgraph audit in `.github/workflows/linkslot-lk-rust-feasibility.yml`. CI run [#1071](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46136315563) has completed the actual M1 SPL link, callgraph audit, and new post-link contract step successfully (step 24). The full workflow remains in progress; this is not yet an overall CI pass.
- Extended `scripts/collect-rg34xx-dram-pfn-readonly.sh` to capture kpageflags/kpagecount for the second source page at 0x60000000 / PFN 0x60000, plus read-only `/proc/iomem`, `/proc/zoneinfo`, `/proc/buddyinfo`, `/proc/pagetypeinfo`, and `/proc/meminfo`. It still does not read DRAM page contents or perform SMC/MMIO/suspend operations.
- Added shell syntax validation for this collector to `.github/workflows/h700-boot-audit-tools.yml`. CI run [#1733](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46136466831) passed the Linux host-tools job including the new shell syntax check; the Windows job was still running at the last check.
- Implementation commits: linked contract `e47be20`, contract tests `a732f9d` / fixture correction `976a4e4`, workflow integration `48b6ce0`; dual-page collector `46175ab`, host workflow integration `3fc2ff8`.
- The M1 gate is now exercised by CI but remains an audit-only build. `candidate_eligible=false`, `hardware_qualified=false`, `payload_executed=false`, `media_write_performed=false`; no hardware action or media write was performed. G2 remains blocked pending new on-device metadata capture and timing/ownership proof.



### 2026-11-28 follow-up: completed M1 CI review and dual-page G2 decoder

- Rechecked CI #1071 (46136315563): the complete cross-compile job and workflow concluded **success**. The real, non-geometry-only M1 SPL link emitted ELF/BIN/MAP/NM/disassembly and the linked-contract JSON. The contract reported BSS use 5,032 bytes of the 16,384-byte ceiling, scratch at 0x30340 (4,096 bytes, 64-byte aligned), GPT table at 0x30000, initial stack pointer 0x52a00, 160 reachable functions and 759 reachable store instructions. The callgraph remains execution_authorized=false, candidate_eligible=false, and hardware_qualified=false.
- The M1 build manifest explicitly identifies this as a compile/link-only preflight profile. Its zero FNV sentinel rejects before firmware payload reads; no device I/O, SRAM access, MMC read, DRAM access, hardware register access, media write, or payload execution occurred. The isolated harness still contains pre-MMC SRAM/TZPC/timer/clock setup and the MMC controller read path, so static transitive-call and hardware-semantics review remains mandatory before any execution authorization.
- CI #1733 (46136466831) completed successfully on both Linux and Windows. CI #1734 (46136531710) also completed successfully.
- Extended scripts/audit-rg34xx-dram-pfn-metadata.py to decode the optional PFN 0x60000 kpageflags/kpagecount pair and record availability of the captured /proc/iomem, zoneinfo, buddyinfo, pagetypeinfo, and meminfo sections. Added regression coverage for dual-page input and missing secondary metadata. The secondary page remains explicitly non-authorizing: ownership_proven=false, free_scratch_eligible=false, gate BLOCKED_OWNERSHIP_NOT_PROVEN.
- The first parser CI attempt failed because the new section regex was over-escaped; corrected it in commit f6b45a8. The subsequent H700 host-tools CI #1736 (46137836309) and BL31 static-memory audit CI #153 (46137836438) both completed successfully.
- The M1 workflow artifact currently contains 84,629 files and is 557,265,595 bytes, exceeding the connected artifact-download limit. The workflow is being narrowed to omit the redundant build/h700-objects/ tree while retaining the M1 audit outputs under build/audit-work/linkslot-1.0.49/.
- Reports branch review: latest committed device evidence still dates to the existing 2026-11-27 inventory and 2026-11-26 timer/PIO/RVBAR captures; no new dual-page PFN collector output is present. G2 therefore remains blocked pending a fresh on-device collector report and independent save/restore timing/ownership proof.
- Safety gates remain unchanged: H700_MEMORY_LAYOUT_APPROVED=false, hardware_applied=0, candidate_eligible=false. Do not execute payloads, write TF/Boot0/TOC1, flash firmware, or advance to M2.


### 2026-11-28 artifact review and owner-decision checkpoint

- CI #1072 (run 46138022018, commit 89b6ab8) completed **success**. The actual M1 SPL link, reachable-callgraph audit, linked-contract gate, Linkslot/LK cross-compilation, and artifact upload all completed. Its artifact was still 552,695,942 bytes despite removing `build/h700-objects/`; therefore that first size-reduction attempt did not solve reviewability.
- Root cause: the upload still included the entire `build/audit-work/linkslot-1.0.49/` tree, which contains unrelated large emulator/build outputs (including static mGBA products). The upload is now changed to stage only the nine required M1 SPL ELF/BIN/MAP/NM/disassembly/manifest/callgraph/contract/build-log files, plus the concise report and SHA256SUMS. New CI #1073 is running to validate this compact artifact path.
- CI #1737 (run 46138019654) completed successfully on both Linux and Windows. Latest parser/static audit runs #1736 and #153 remain successful.
- Reviewed the latest reports-branch direction audit `project-goal-alignment-review-20261128` (audit HEAD 7877b838). It confirms the M1-first / G1-G11 / audit-only / freeze-120Hz-and-suspend direction is aligned, while flagging two unresolved owner decisions before progressing beyond current audit-only SPL work: (1) SPL profile A (stock-init parity, explicitly declare PMIC/TWI access) versus profile B (strict no-PMIC, which requires independent proof DRAM is already usable); (2) whether the dedicated test card may use stock RTC GP14/GP15 and watchdog protocol. The report recommends A and accepting the stock RTC/WDT protocol, but these are owner decisions and are not inferred as approved here.
- The review report is a read-only snapshot at older HEAD 7877b838 and reports zero diagnostic payload at that time; it does not supersede later CI evidence. No newer dual-page PFN capture is present in reports branch HEAD 836fdb6e, so G2 remains blocked.
- Keep M1 as the sole near-term workstream per the already-recorded project decision. Do not begin M1-S3/package execution or hardware tests until the PMIC/TWI and RTC/WDT policy is explicitly selected and separately reviewed. No payload execution, media writes, or firmware flashing are authorized.
- Safety state remains fail-closed: `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`, `candidate_eligible=false`; DRAM source pages `0x40000000` and `0x60000000` remain excluded.


## 2026-11-28 SenkOS cross-project audit and feasibility #1075 closure

- Added [`docs/rg34xx-senkos-cross-audit-20261128.md`](rg34xx-senkos-cross-audit-20261128.md) with pinned SenkOS source review, KNULLI RG34XX DTS comparison, Boot0 log cross-check, PMIC naming disposition, and remaining source-provenance gates.
- SenkOS provides a concrete H700 SPL reference: PH6/PH7/PH8/PH10 straps select LPDDR3 versus LPDDR4; its U-Boot patch selects matching DRAM/PHY timing tables and routes DCDC3 through `axp_set_dcdc3()` (1.2 V for its LPDDR3 branch, configured LPDDR4 voltage otherwise). Its `docs/straps.md` documents Boot0 `dram_para`/`DRAM.ext` and `tpr3` voltage encoding.
- KNULLI's RG34XX boot-package DTS uses `x-powers,axp2202` at I2C 0x34 under `twi5` / R-TWI0 0x07081400. SenkOS's H700 U-Boot/Linux lineage uses AXP717 naming at I2C 0x34. Treat this as a vendor-binding/upstream-driver reconciliation candidate, not yet proof of silicon/register equivalence. No AXP717 path is enabled in the current M1 preflight.
- Cross-check: the public RG34XX Boot0 log reports AXP2202, DRAM_VCC 1100 mV, 672 MHz LPDDR4, 1 GiB and successful training; the project's three read-only runtime captures report DCDC3 1,100 mV. This is consistent evidence, not proof of the exact PMIC write sequence or strap values.
- **Linkslot feasibility #1075 is now verified complete/success** (run 46141495743, commit `807b3b2dc566b9fec1f11d4009b07afeea1ed0d7)). The cross-compile job and all 92 primary steps passed; the compact review artifact is 551,201 bytes. M1 actual SPL link, reachable-store report and fail-closed linked contract passed. This remains compile/link/static/host evidence only.
- Next: compare Boot0 `dram_para`/`DRAM.ext` and target strap evidence; trace SenkOS `axp_set_dcdc3()` into the exact U-Boot AXP717 register/TWI operations; independently trace the RG34XX stock Boot0 PMIC writes and timing relative to DRAM training. Keep the M1 read-only preflight PMIC/TWI-disabled until a concrete dependency is proven.
- Safety gates unchanged: `H700_MEMORY_LAYOUT_APPROVED=false`, `hardware_applied=0`, `candidate_eligible=false`, `hardware_qualified=false`; no payload execution, physical register writes, device I/O, media writes, TF/Boot0/TOC1 writes or flashing. DRAM source pages `0x40000000` and `0x60000000` remain excluded.


## 2026-11-28 Pinned U-Boot AXP717 SPL path cross-check

- Extended [`docs/rg34xx-senkos-cross-audit-20261128.md`](rg34xx-senkos-cross-audit-20261128.md) with a direct source trace against pinned U-Boot v2026.10 commit `5508406582f6f7120dce0c4b3059819a41788db2`.
- The generic H700 defconfig enables `AXP717_POWER`, R-TWI (`R_I2C_ENABLE`), SPL legacy I2C and 400 kHz, with PMIC address default 0x34 and DCDC2=940 mV / DCDC3=1100 mV. These values match the project's RG34XX read-only rail captures; the 1.1 V DCDC3 value also matches the public RG34XX Boot0 log.
- The pinned AXP717 SPL table maps DCDC3 voltage to register 0x85 (mask 0x7f, 500–1840 mV in 10 mV steps) and enable to register 0x80 bit 2. `axp_set_dcdc3()` writes the voltage register then read-modify-writes the enable bit. The AXP717 SPL `axp_init()` does not read a chip ID (its ID mask is zero), so driver selection alone cannot prove silicon identity.
- The H616 board path enables TWI channel 5 and GPL0/GPL1 R-TWI pins; the target DT identifies `twi5` at 0x07081400. This strongly corroborates the AXP717 driver as a candidate for the target's legacy AXP2202 binding, but the active legacy-I2C bus selection and exact target Boot0 call path remain open.
- Corrected disposition: the earlier AXP2202-versus-AXP717 incompatibility is **not established**; upstream Linux and Allwinner documentation strongly support a legacy BSP alias / same-PMIC-family relationship, and the Allwinner AXP2202 SPL register map matches the pinned AXP717 SPL DCDC3 map. The exact RG34XX Boot0 transaction sequence, active bus/address fallback and DCDC3-before-DRAM ordering remain unproven. Do not enable PMIC writes in the M1 preflight; retain the Profile A allowlist gate for a separate DRAM-capable SPL.
- Next static task: close the `i2c_read/i2c_write` → active bus → R-TWI0 path and compare the exact Boot0 DCDC3 register writes with the pinned AXP717 sequence, including failure propagation and timing relative to DRAM training.
- CI #1075 remains completed/success; this documentation-only update does not change its result or any safety gate.


## 2026-11-28 R-TWI adapter mapping and PMIC-before-DRAM ordering

- Extended the cross-audit with the full pinned U-Boot v2026.10 legacy I2C adapter trace.
- In the generic H700 defconfig, R-TWI is the only registered MVTWSI adapter: logical bus 0 maps to adapter `twsi2` / hardware index 2 / `SUNXI_R_TWI_BASE`. H616 `i2c_init_board()` enables TWI channel 5 and configures GPL0/GPL1. The target DTS `twi5` at 0x07081400 matches that controller.
- `board_init_f()` initializes the I2C board pins and bus before `sunxi_board_init()`. The latter initializes the PMIC, applies configured DCDC2/DCDC3 settings, then calls `sunxi_dram_init()`. Thus the upstream reference's PMIC writes are statically ordered before DRAM training.
- The generic upstream source path is now traced end-to-end: logical I2C bus 0 → twsi2 hardware adapter → R-TWI0 → PMIC 0x34; DCDC3 register write 0x85 followed by read-modify-write of enable register 0x80 bit 2.
- Revised blocker: upstream U-Boot PMIC/TWI selection and pre-DRAM ordering are no longer open. Remaining target blockers are the exact stock Boot0 PMIC source path, exact target Boot0 PMIC transaction sequence, bus/address fallback and DCDC3-before-DRAM ordering, and target-specific PH6/PH7/PH8/PH10 strap evidence. AXP717 SPL's lack of chip-ID read means this identity gate cannot be waived.
- No source code or Kconfig was changed; M1 preflight remains PMIC/TWI-disabled. No firmware execution, device register writes, media writes or hardware qualification.


## 2026-11-28 Target applicability correction: RG34XX vs RG35XX

- Explicitly reaffirmed: the project target is **RG34XX**. SenkOS and `anbernic_rg35xx_h700_defconfig` are H700-family reference sources, not RG34XX hardware proof.
- ROCKNIX publishes separate RG34XX, RG35XX 2024 and RG35XX H DTBs; therefore shared H700 SoC does not justify reusing board-specific GPIO, strap, DRAM, PMIC, panel or power-sequencing assumptions.
- Initially reclassified SenkOS PH6/PH7/PH8/PH10 strap mapping as reference-only; this has now been superseded by direct RG34XX stock TOC1 DTB evidence confirming those four GPIO inputs. SenkOS's exact strap-index interpretation still requires SPL2 code-path confirmation. DRAM timing tables, LPDDR3 branch details, AXP717 configuration and generic PMIC ordering remain hypotheses until target-specific code evidence matches.
- RG34XX-specific evidence remains the authority: target-specific KNULLI RG34XX DTS, RG34XX Boot0 log, and this project's read-only RG34XX runtime captures. Even these do not alone prove physical PMIC silicon/register equivalence.
- Corrected blocker status after TOC1 analysis: RG34XX DRAM strap pin identity and GPIO input configuration are now directly confirmed by the stock package DTB; the exact runtime selection algorithm remains open. PMIC physical identity/register provenance and power sequencing remain gated. No PMIC writes or family-reference timing changes are approved.
- Detailed correction added to `docs/rg34xx-senkos-cross-audit-20261128.md`. M1 preflight remains PMIC/TWI-disabled; all safety gates unchanged.


## 2026-11-28 Nested Boot0 parser and CI validation

- \`tools/inspect_h700_boot0.py\` now enumerates aligned nested eGON-tagged components instead of treating the 128 KiB primary capture as one flat Boot0. It recognizes standard zero-tag Boot0 headers and reports the nonstandard \`SPL\\x02\` component separately without applying the wrong checksum rule.
- H700 run #1796 passed an earlier host-test set but predates the final nested-header regression assertions; it should not be treated as final validation. A later run exposed that the synthetic test fixture omitted the real eGON 0x30 header-size word. The fixture was corrected, and H700 host audit run #1807 then passed: [run #1807](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46145458872).
- Linkslot feasibility run #1081 is still in progress on the same parser commit: [run #1081](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46144699555). No result is claimed yet.
- A new H700 audit pass is being prepared to disassemble the primary outer Boot0, embedded inner Boot0, and SPL2 at candidate VMA \`0x50000\`, while scanning the TOC1 package and same-capture live FDT. This is static analysis only; the SPL2 handoff remains a candidate until the control-flow/copy path is recovered.



## 2026-11-28 SPL2 PMIC path and TOC1 identity cross-check

- H700 static disassembly run #1808 passed and produced the dual-mode Boot0/SPL2 bundle: [run #1808](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46145510710). Artifact: \`rg34xx-stock-boot0-spl2-toc1-disassembly\`.
- TOC1 is 1,310,720 bytes (SHA-256 \`e6c20e3a54c51b9336c798e3a4fa1ca6c45b28c6f70456e2db9c252aa4ba4604\`). It contains one validated embedded FDT at offset \`0x11b800\`, size 139,822 bytes, with \`:rg34xx_v1\`, \`x-powers,axp2202\`, and AXP2202 regulator nodes. The live FDT also validates as RG34XX-v1.
- The TOC1 string index contains \`U-Boot 2018.05\`, BL31 source-path strings, \`pmu_axp2202_probe\`, and \`bmu_axp2202_probe\`; no RG35XX board label appears in its filtered board strings. The RG35XX-2024 build label is localized to the embedded SPL2 component.
- SPL2 AArch64 code requests DCDC2 940 mV (regulator ID 2) and DCDC3 1100 mV (ID 3) before calling the next DRAM-init routine. Inside the DRAM path it selects DCDC3 by DRAM type: LPDDR4 type 8 -> 1100 mV; LPDDR3 type 7 -> 1200 mV; other types -> 1500 mV.
- The SPL2 regulator table at component offset \`0xaafc\` maps DCDC2 to control reg \`0x80\` mask \`0x02\`, voltage reg \`0x84\` mask \`0x7f\`; DCDC3 to control reg \`0x80\` mask \`0x04\`, voltage reg \`0x85\` mask \`0x7f\`. The setter uses 500 mV minimum and 10 mV steps; 940 mV maps to \`0x2c\`, 1100 mV to \`0x3c\`.
- SPL2's PMIC read/write helpers pass I2C slave address \`0x34\` and one-byte register/data arguments through a bus callback table. This register layout matches the audited AXP717 DCDC2/DCDC3 map and the RG34XX AXP2202 FDT; it is strong evidence of an AXP717-compatible register path, not proof of physical silicon identity.
- The callback table is selected via \`x18 + 264\`; its stored callback pointer values do not decode as AArch64 code at the corresponding addresses in the captured Boot0 image. Do not claim that they are Boot0 callbacks or that the bus is R-TWI0 yet. Exact callback relocation/descriptor semantics and the mapping to \`twi5\` / \`0x07081400\` remain open.
- This target SPL2 voltage policy independently corroborates SenkOS's LPDDR4 1100 mV / LPDDR3 1200 mV values. It does not validate SenkOS's PH6/PH7/PH8/PH10 strap mapping for RG34XX.
- Linkslot feasibility run #1083 is still in progress; no result is claimed: [run #1083](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46145458855). All hardware safety gates remain unchanged.
- Detailed report: \`docs/rg34xx-boot0-container-spl2-audit-20261128.md\`.



## 2026-11-28 SPL2 I2C callback relocation and R-TWI0 mapping

- The SPL2 callback table at candidate VMA \`0x5b6b0\` stores link-time callback addresses \`0x27d30\` (read) and \`0x27c20\` (write). The SPL2 is linked at \`0x20000\` and loaded at candidate runtime VMA \`0x50000\`, giving relocation delta \`+0x30000\`; the callbacks resolve to valid AArch64 functions at \`0x57d30\` and \`0x57c20\`.
- Both first bus descriptors carry controller index 2 at descriptor offset \`+56\`. Their read/write callbacks call the controller lookup \`0x57ae0\`; index 2 returns base \`0x07081400\`. This matches the RG34XX live FDT's \`twi5\` and KNULLI's RG34XX DTS.
- The PMIC read/write helpers pass I2C slave address \`0x34\`; the path therefore resolves to the R-TWI0 MMIO backend for either of the first two valid descriptors. The global descriptor selector \`x18 + 264\` is read but no explicit assignment was found in the SPL2 code; exact selection of descriptor 0 versus 1 remains open.
- A regression test now pins the SPL2 regulator table, DCDC2/DCDC3 setpoints, PMIC slave address, controller-index mapping, and callback descriptor values in the raw RG34XX Boot0 capture.
- This closes the R-TWI0 base-address identification blocker at static-code level. It does not authorize execution or I/O, and it does not prove the physical PMIC silicon identity.
- Detailed evidence: \`docs/rg34xx-boot0-container-spl2-audit-20261128.md\`.



## 2026-11-28 RG34XX TOC1 confirms DRAM strap pins and later-stage boot components

- H700 audit run #1826 passed and produced the extracted U-Boot/monitor/DTB disassembly bundle: [run #1826](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46146478411). The latest Linkslot feasibility run #1086 also passed: [run #1086](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46146405481).
- The RG34XX TOC1 package is 1,310,720 bytes, SHA-256 \`e6c20e3a54c51b9336c798e3a4fa1ca6c45b28c6f70456e2db9c252aa4ba4604\`. It has four components: \`u-boot\`, \`monitor\`, \`dtbo\`, \`dtb\`.
- The embedded DTB payload contains the \`:rg34xx_v1\` marker and AXP2202/R-TWI0 data. Its validated FDT SHA-256 is \`7017a8f970234f7518e7f417fa9f86dd4ad3e46529bc5282c05a31e9f007dbc5\`.
- **Important blocker closure:** \`/soc@03000000/dram_select_para/select_mode=1\` configures GPIO-based automatic DRAM selection. \`select_gpio0..3\` reference pinctrl phandle \`0x4b\` and encode port 7 pins 6, 7, 8, 10, GPIO-input mux 0 and pull-up 1. The referenced controller is \`allwinner,sun50iw9p1-pinctrl\`; Allwinner port index 7 is PH. This directly confirms **PH6, PH7, PH8, PH10** as the RG34XX package's DRAM strap inputs, matching SenkOS's pin list.
- The same FDT carries \`dram_para14\` = LPDDR3 type 7 at 672 MHz and \`dram_para15\` = LPDDR4 type 8 at 672 MHz. This is target-specific evidence for both the pins and the two DRAM profiles. It does not yet prove SPL2's sample order, bit polarity, code-to-profile mapping or error fallback.
- U-Boot payload is AArch32 and identifies as \`U-Boot 2018.05 (May 26 2026 - 13:55:09 +0800) Allwinner Technology\`; strings include \`pmu_axp2202_probe\` and \`bmu_axp2202_probe\`.
- AArch64 monitor payload contains BL31/TF-A platform paths for \`sun50iw9p1\`, PSCI CPU on/off/suspend code strings, and generic error strings for missing System Off/System Reset hooks. The strings do not alone prove whether the platform hooks are absent; inspect the actual PSCI platform-ops table and power call paths next.
- New regression test \`tools/test_rg34xx_stock_toc1_dtb.py\` pins the package and component hashes, RG34XX DTB marker, AXP2202/TWI5 topology, PH6/7/8/10 GPIO tuples, LPDDR3/LPDDR4 profile IDs and regulator ranges. The H700 audit workflow now fetches the read-only \`reports\` branch before running Linux and Windows host tests.
- This supersedes the earlier statement that SenkOS strap-pin mapping remained wholly unconfirmed on RG34XX. Pin mapping/configuration is now confirmed; runtime selector semantics remain open.
- Next: disassemble/trace SPL2 DRAM selector logic and exact profile selection; trace DCDC enable/read-modify-write and PMIC probe; then map BL31 PSCI platform ops and power-off/reset hooks. No firmware execution, PMIC/TWI writes, DRAM access, media writes or hardware qualification.
- Safety gates unchanged: \`H700_MEMORY_LAYOUT_APPROVED=false\`, \`hardware_applied=0\`, \`candidate_eligible=false\`, \`hardware_qualified=false\`.

Detailed evidence: \`docs/rg34xx-boot0-container-spl2-audit-20261128.md\`.

