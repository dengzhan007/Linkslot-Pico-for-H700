# RG34XX Boot0 container and embedded SPL2 audit — 2026-11-28

## Scope and source provenance

This report analyzes the read-only TF-card captures in:

- \`reports/rg34xx-inventory-20261127-uart/boot0-primary.bin\` — read from \`/dev/mmcblk0\`, LBA 16, 256 sectors (128 KiB).
- \`reports/rg34xx-inventory-20261127-uart/boot0-secondary.bin\` — read from \`/dev/mmcblk0\`, LBA 512, 256 sectors (128 KiB capture; declared Boot0 image is 64 KiB).
- \`reports/rg34xx-inventory-20261127-uart/live-fdt.dtb\` — same inventory capture.
- Collector implementation: \`scripts/collect-rg34xx-readonly-inventory.sh\`.

These are the project's RG34XX TF-card captures, not eMMC boot partitions. No hardware writes or payload execution were performed.

## 1. Primary container layout

The 128 KiB primary capture is not a flat single Boot0 executable. It contains three aligned eGON-tagged headers:

| File offset | Declared size | Header word at +0x14 | Interpretation |
|---:|---:|---|---|
| \`0x0000\` | \`0x20000\` (128 KiB) | \`0x30\` | Outer eGON Boot0 container; standard checksum passes |
| \`0x1000\` | \`0x10000\` (64 KiB) | \`0x30\` | Embedded eGON Boot0 component; standard checksum passes |
| \`0x11000\` | \`0xC000\` (48 KiB) | \`SPL\\x02\` | Nonstandard SPL2 component; do not apply the standard Boot0 checksum rule |

The embedded Boot0 component's 15-profile \`DRAM.ext\` table is at relative offset \`0x348\`, therefore absolute file offset \`0x1348\`. The outer container also has a separate marker at \`0x348\` followed by one plausible profile. Those are distinct structures and must not be conflated.

## 2. Primary-inner versus secondary Boot0

The primary's inner Boot0 image (\`primary[0x1000:0x11000]\`) and the secondary standalone image (\`secondary[0:0x10000]\`) are near-identical:

- Primary inner SHA-256: \`b6edbf39af0bf1f35a7e2fd991c8966b04d061e247c81985d2516f695464049b\`
- Secondary declared-image SHA-256: \`4701a85e4db7e8339b91b92f691a83cc954ec63a09369087c2e41d80eafa9f5d\`
- Exactly five bytes differ: offsets \`0x0c\`, \`0x0f\` (checksum), \`0x34\` (debug byte), \`0xa3\` (active \`dram_tpr6\` high byte), and \`0xac\` (active \`dram_tpr12\` low byte).

| Field | Primary inner | Secondary |
|---|---:|---:|
| debug byte | 3 | 0 |
| DRAM type | LPDDR4 (8) | LPDDR4 (8) |
| DRAM clock | 672 MHz | 672 MHz |
| \`dram_tpr6\` | \`0x3e808080\` | \`0x40808080\` |
| \`dram_tpr12\` | \`0x06070706\` | \`0x06070707\` |

The outer primary header's active 32-word DRAM parameter block is byte-for-byte equal to the embedded primary Boot0 block. The 15-profile table is present in both the primary inner Boot0 and secondary standalone Boot0. This is direct evidence that the two Boot0 copies carry a very similar H700 LPDDR4/672 MHz configuration with two small timing-field differences; it does not establish which copy or profile the hardware selected.

## 3. Embedded SPL2 build label

The component at \`0x11000\` has:

- eGON.BT0 magic;
- nonstandard header tag bytes \`SPL\\x02\`;
- declared size \`0xC000\`;
- platform string at component offset \`0x2c\`:

\`allwinner/sun50i-h700-anbernic-rg35xx-2024\`

This is a real build/platform label in the RG34XX TF-card capture. It is not safe to dismiss, but it is also not proof of an incompatible board configuration. Anbernic may deliberately share this H700 SPL across RG34XX and RG35XX 2024, with board differentiation occurring in later device-tree/boot-package stages. That hypothesis must be verified against the SPL's actual hardware assumptions and the TOC1 package.

The embedded SPL2 component's standard eGON checksum is not asserted because its header tag is nonstandard. The current audit records it as a distinct component rather than falsely labelling it a corrupt Boot0 image.

## 4. Candidate Boot0-to-SPL2 handoff correlation

Static inspection of the outer Boot0 entry stub found:

- the instruction at runtime address \`0x20488\` loads PC from a literal at file offset \`0x494\`;
- that literal is \`0x50000\`;
- the SPL2 component begins with ARM word \`0xea000016\`, which branches to component offset \`0x60\`;
- if the SPL2 component is loaded at VMA \`0x50000\`, that branch target is \`0x50060\`.

This address relationship is consistent with the outer Boot0 handing off to the embedded SPL2 component at \`0x50000\`. It is not yet proof of the full copy path, active component selection, or control-flow reachability. The next disassembly pass therefore maps the embedded SPL2 component at candidate VMA \`0x50000\`, separately from the outer and inner Boot0 images.

## 5. Same-device RG34XX identity evidence

The live FDT from this same inventory capture contains:

- root model string \`sun50iw9\`;
- compatibles \`allwinner,h616\` and \`arm,sun50iw9p1\`;
- board marker \`:rg34xx_v1\`;
- PMIC compatible \`x-powers,axp2202\`;
- \`twi@0x07081400\` and \`twi5\`;
- regulator nodes including \`axp2202-dcdc3\`.

Thus the same TF-card inventory contains an RG34XX-v1 live FDT while its embedded SPL2 header advertises an RG35XX-2024 build path. This is a cross-component naming discrepancy on the target device, not a reason to relabel the hardware as RG35XX. The compatibility question is whether the SPL2 code's DRAM/PMIC/strap/power assumptions are shared by RG34XX v1.


## 6. TOC1 package evidence: later stage is RG34XX-v1 labelled

The same reports-branch inventory includes a 1,310,720-byte TOC1 package:

- SHA-256: \`e6c20e3a54c51b9336c798e3a4fa1ca6c45b28c6f70456e2db9c252aa4ba4604\`
- String index contains \`rg34xx_v1\` at offsets \`0xa40d0\` and \`0x12b52b\`; the filtered TOC1 string set contains no \`rg35xx\` label.
- It contains \`U-Boot.armv7\` and \`U-Boot 2018.05 (May 26 2026 - 13:55:09 +0800) Allwinner Technology\`.
- It contains BL31 source-path strings including \`plat/sun50iw9p1/bl31_sunxi_setup.c\`, \`bl31/bl31_main.c\`, and \`bl31/context_mgmt.c\`.
- It contains PMU driver symbols/strings \`pmu_axp2202_probe\`, \`bmu_axp2202_probe\`, \`PMU: AXP2202\`, plus the AXP2202 regulator names.
- Its FDT-related strings include \`x-powers,axp2202\`, \`axp2202-dcdc3\`, and the RG34XX-v1 marker. The live FDT has the same RG34XX-v1 and AXP2202/TWI identity.

This materially narrows the SPL2 naming discrepancy: the embedded SPL2 component is labelled RG35XX-2024, but the later TOC1 package carries RG34XX-v1 board data and AXP2202 PMU support. That is consistent with an intentionally shared H700 SPL followed by a board-specific TOC1/U-Boot/BL31 package. It is not yet proof that the SPL itself is hardware-generic; its DRAM/PMIC/strap assumptions still need to be checked.

The SPL2 string index also shows \`U-Boot SPL 2026.10 (Oct 02 2026)\`, \`spl_load_simple_fit\`, and DRAM failure/unsupported-configuration diagnostics. The SPL2 ARM-mode disassembly reveals AArch64-looking instruction encodings in the component. Therefore the ARM-only disassembly is not sufficient for the main SPL code; the next pass adds a separate AArch64 disassembly from candidate component offset \`0x188\`.

The TOC1 scan now validates one embedded FDT at file offset `0x11b800`, total size 139,822 bytes, with `:rg34xx_v1` and AXP2202 compatible strings. This establishes the presence of RG34XX-v1 board data in the package, but does not prove which FDT is selected at runtime or identify all component boundaries.


## 7. SPL2 AArch64 PMIC regulator path

The SPL2 component is mixed-mode: its early header/entry stub is ARM-state code, while valid AArch64 instruction sequences appear from candidate component offset \`0x188\`. The AArch64 disassembly is therefore required for the main SPL logic; the ARM-only listing is not a reliable decode of that region.

The AArch64 listing provides a target-specific DCDC voltage path:

- At candidate VMA \`0x54cec\`, the code requests 940 mV through wrapper \`0x5a318\` (regulator ID 2 / DCDC2).
- At \`0x54cf8\`, it requests 1100 mV through wrapper \`0x5a324\` (regulator ID 3 / DCDC3).
- It then prints the \`DRAM:\` banner and calls \`0x53608\`, the next DRAM-initialization routine.
- Inside that DRAM path, code around \`0x53720\` reads the DRAM type and selects DCDC3 voltage: LPDDR4 type 8 -> 1100 mV; LPDDR3 type 7 -> 1200 mV; other types -> 1500 mV. It calls the same DCDC3 wrapper and waits 1000 ms.

The regulator table at candidate VMA \`0x5aafc\` (component-relative offset \`0xaafc\`) contains 10-byte entries:

| Regulator ID | Rail | Control register | Enable mask | Voltage register | Voltage mask | Range / step |
|---:|---|---:|---:|---:|---:|---|
| 2 | DCDC2 | \`0x80\` | \`0x02\` | \`0x84\` | \`0x7f\` | 500–1540 mV / 10 mV |
| 3 | DCDC3 | \`0x80\` | \`0x04\` | \`0x85\` | \`0x7f\` | 500–1840 mV / 10 mV |

The voltage setter computes the code from the table's minimum and step. Therefore 940 mV on DCDC2 maps to code \`0x2c\`; 1100 mV on DCDC3 maps to code \`0x3c\`. The SPL2 I2C read/write helpers pass slave address \`0x34\`, one-byte register addresses and one-byte data through a bus callback table. The helper at \`0x50678\` performs a read-modify-write that ORs an enable mask into a control register; however, the call sites recovered so far prove the voltage-setting calls, not the complete DCDC enable sequence.

This register table matches the AXP717 SPL driver's DCDC2/DCDC3 register layout audited earlier, while the same RG34XX inventory's live FDT and TOC1 identify the PMIC as \`x-powers,axp2202\`. This is strong evidence that the stock RG34XX SPL2 uses an AXP717-compatible DCDC register map for its AXP2202-labelled PMIC path. It does not prove the physical silicon identity or complete register-map equivalence.

The bus helpers at \`0x5779c\` and \`0x57804\` dispatch through a descriptor selected from global field \`x18 + 264\`; the descriptor table begins at \`0x5b6b0\`. Descriptor entry 0 stores link-time callback addresses \`0x27d30\` (read) and \`0x27c20\` (write), which relocate by \`+0x30000\` to valid SPL2 AArch64 functions at \`0x57d30\` and \`0x57c20\`. Those functions read the descriptor's controller index at offset \`+56\`. Entry 0 and entry 1 both specify controller index 2. The controller lookup at \`0x57ae0\` returns MMIO base \`0x07081400\` when the index is 2 and reports an unknown bus for other indices. Thus the stock SPL2 contains an R-TWI0 backend at exactly the RG34XX FDT/KNULLI \`twi5\` base. The selector \`x18 + 264\` is read but no explicit assignment was found in the SPL2 code; whether entry 0 or 1 is selected remains open, although both route to controller index 2. The I2C transaction helpers pass slave address \`0x34\` through this descriptor path.

This independently corroborates the LPDDR4 1100 mV / LPDDR3 1200 mV voltage policy seen in SenkOS. The later TOC1 DTB audit below also directly confirms PH6/PH7/PH8/PH10 as RG34XX DRAM-selection GPIOs; the remaining open question is the exact SPL2 sampling/bit-order/fallback path, not whether those pins are configured in the RG34XX package.




## 8. RG34XX TOC1 DTB confirms the SenkOS DRAM strap pins

This section supersedes the earlier statement that SenkOS's PH6/PH7/PH8/PH10 strap mapping had not been confirmed for RG34XX.

The extracted TOC1 package contains four components: \`u-boot\`, \`monitor\`, \`dtbo\`, and \`dtb\`. The package's DTB payload is 144,384 bytes (SHA-256 \`c28c87380ffab00148bf4dc457e70b8c6c576470dcfbc53df64addddf89603a1\`); its valid FDT occupies 139,822 bytes (SHA-256 \`7017a8f970234f7518e7f417fa9f86dd4ad3e46529bc5282c05a31e9f007dbc5\`). It contains the \`:rg34xx_v1\` board marker and the same AXP2202/R-TWI0 topology as the live RG34XX FDT.

The FDT's \`/soc@03000000/dram_select_para\` node contains:

- \`select_mode = 1\`, which Allwinner's sys_config documentation defines as GPIO-based automatic DRAM profile selection (mode 2 is GPADC; mode 3 is GPIO+GPADC).
- \`gpadc_channel = 1\` is present in the configuration, but mode 1 is the GPIO-only mode.
- \`select_gpio0\` through \`select_gpio3\` all reference pinctrl phandle \`0x4b\`, which resolves to \`allwinner,sun50iw9p1-pinctrl\` at \`/soc@03000000/pinctrl@0300b000\`.
- The six GPIO cells after that phandle encode port index 7, pin numbers 6, 7, 8 and 10, GPIO input function 0, pull-up 1, and default drive/data values. Allwinner's port numbering maps index 7 to port H. These are therefore **PH6, PH7, PH8 and PH10**, in the same order as SenkOS's strap patch.

The same DTB contains two relevant profiles:

| Profile node | DRAM type | Type ID | Clock |
|---|---|---:|---:|
| \`dram_para14\` | LPDDR3 | 7 | 672 MHz |
| \`dram_para15\` | LPDDR4 | 8 | 672 MHz |

This is now direct RG34XX package evidence that the four SenkOS strap pins are configured as DRAM-selection inputs and that the package carries LPDDR3/LPDDR4 profiles 14 and 15. It materially validates the pin mapping on RG34XX; SenkOS is no longer merely a source of unverified pin assumptions.

The stock Boot0 Thumb disassembly now closes the runtime sampling-order and profile-index questions. The actual selector reads the Boot0 image's own `DRAM.ext` block; the later FDT node is a matching configuration copy, not the structure directly consumed by this Boot0 routine. The exact Boot0 algorithm and its limits are detailed below.


### Stock Boot0 Thumb-code confirms GPIO sampling and profile lookup

The raw primary Boot0 capture is 128 KiB, SHA-256 \`bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f\`. Its inner 64 KiB image begins at file offset \`0x1000\` and is linked at \`0x20000\`.

The Boot0 \`DRAM.ext\` block begins at inner-image offset \`0x348\` (VMA \`0x20348\`):

- Signature: \`DRAM.ext\`.
- Byte \`+0x350\`: \`select_mode=1\`.
- Byte \`+0x351\`: GPADC channel field \`1\` (not used by the mode-1 GPIO-only branch).
- Four 8-byte GPIO descriptors at \`+0x354 + i*8\`:
  - \`08 06 00 01 ff ff 00 00\`
  - \`08 07 00 01 ff ff 00 00\`
  - \`08 08 00 01 ff ff 00 00\`
  - \`08 0a 00 01 ff ff 00 00\`
- The first descriptor byte is the one-based port value 8; Boot0 subtracts one before applying the \`0x24\`-byte PIO-bank stride. Thus it resolves to port index 7 / PH. The second byte is the pin number. The remaining descriptor bytes encode GPIO mux/pull/drive/data settings; the pull field is 1.

The Thumb code at VMA \`0x2340e\` loads the mode byte at \`DRAM.ext + 0x350\`; VMA \`0x23412\` compares it with 1 and branches to the GPIO sampler at \`0x23418\`. That sampler:

1. Uses the PIO data-register base literal \`0x0300b010\` (inner-image literal at offset \`0x3580\`).
2. For each of four descriptors, reads the port value at descriptor offset \`+0\`, subtracts one, multiplies by \`0x24\`, and reads the GPIO data register.
3. Reads the pin number at descriptor offset \`+1\`, forms \`1 << pin\`, masks and extracts the sampled bit.
4. Shifts the result by a bit position initialized to zero, ORs it into the result, increments the bit position, advances the descriptor pointer by 8 bytes, and repeats until four GPIOs have been sampled.

Therefore the bit mapping is exact and LSB-first:

| DRAM.ext entry | RG34XX pin | Result bit |
|---:|---|---:|
| 0 | PH6 | 0 |
| 1 | PH7 | 1 |
| 2 | PH8 | 2 |
| 3 | PH10 | 3 |

The four-bit result is then used as a **one-based profile number**. The Thumb code at VMA \`0x23482\` computes the selected index. If it is zero, it branches to \`0x234a0\` and scans the profile table; if nonzero, it subtracts one at \`0x23488\`, shifts by seven (128-byte profile stride) at \`0x2348a\`, adds the DRAM.ext base at \`0x2348c\`, and checks the first word at profile-table base offset \`0x374\` at \`0x2348e\`. If the selected entry is empty, it logs the invalid-profile path and falls back to the table scan. A valid selected 128-byte profile is copied to the active DRAM parameter block at inner VMA \`0x20038\` (literal at inner offset \`0x3584\`).

The Boot0 DRAM.ext table has 15 profiles. Profiles 1–13 are DDR3 type 3 at 792 MHz. The two RG34XX-relevant entries are:

| Four-bit selector | Profile | Type ID | DRAM | Clock | TPR6 |
|---:|---:|---:|---|---:|---:|
| \`1110b\` = 14 | 14 | 7 | LPDDR3 | 672 MHz | \`0x2fb08080\` |
| \`1111b\` = 15 | 15 | 8 | LPDDR4 | 672 MHz | \`0x40808080\` |

This directly confirms SenkOS's **14 → LPDDR3** mapping on RG34XX, including the exact pin order. The stock Boot0 implementation is more general than the SenkOS patch's simplified "14 else LPDDR4" rule: it performs a direct profile-index lookup, and selector values 1–13 address DDR3 profiles. Our port must preserve the lookup semantics and fail closed on unexpected/unsupported indices; it must not silently map every non-14 value to LPDDR4.

The public RG34XX Boot0 runtime log reporting LPDDR4 type 8 at 672 MHz is consistent with profile 15, but does not contain the raw GPIO sample. The actual electrical strap level on each pin is therefore not independently measured by this static audit.

The Boot0 DRAM.ext/Thumb sampler regression is now green on both host platforms: H700 audit run #1852 passed Linux and Windows after correcting the profile-zero fallback branch assertion ([run #1852](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46149278892)); the follow-up documentation-state run #1853 also passed both jobs ([run #1853](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46149296929)).



### Extracted later-stage identities

- The U-Boot payload is AArch32 ARM-state code, begins with entry word \`0xea00018e\`, contains \`U-Boot.armv7\`, and identifies itself as \`U-Boot 2018.05 (May 26 2026 - 13:55:09 +0800) Allwinner Technology\`. Its strings include \`pmu_axp2202_probe\` and \`bmu_axp2202_probe\`.
- The monitor payload is AArch64, begins with entry word \`0x1400003e\`, and contains TF-A source paths \`plat/sun50iw9p1/bl31_sunxi_setup.c\` and \`plat/sun50iw9p1/plat_pm.c\`. It includes PSCI CPU on/off/suspend implementation strings such as \`psci_afflvl0_on\`, \`psci_afflvl0_off\` and \`psci_afflvl0_suspend\`.
- The monitor also contains generic TF-A error strings for a platform that has not exported PSCI System Off/System Reset hooks. String presence is not enough to prove those hooks are absent in this binary; the actual platform operation table and call paths must be traced.
- These are extracted vendor-stage references from the RG34XX TOC1 package, not outputs that can be safely executed as project firmware.

A new regression test \`tools/test_rg34xx_stock_toc1_dtb.py\` pins the TOC1/package payload hashes, FDT identity, PMIC/TWI node, GPIO strap tuples, LPDDR3/LPDDR4 profile IDs and regulator voltage ranges. The H700 audit workflow now fetches the read-only reports branch before running the host tests on Linux and Windows.



## 9. SPL2 contains a separate three-candidate DRAM fallback table

A follow-up read of the SPL2 data table and AArch64 control flow reveals a second mechanism that must not be conflated with the DTB's GPIO strap declaration:

- Candidate table VMA: \`0x5a7f4\`; component-relative offset: \`0xa7f4\`.
- Candidate stride: \`0x30\` bytes.
- The DRAM initialization/test loop at VMA \`0x53608\` initializes an index to zero, allows indices 0 through 2, and computes \`base + index * 0x30\`. It calls \`0x538dc\` to save CPU registers/stack context, then calls candidate-specific routines at \`0x527d8\` and \`0x5287c\`.
- The three embedded candidate records are all 672 MHz:
  - Candidate 0: LPDDR4 type 8; \`dram_tpr6=0x3e808080\` (matches the active Boot0 primary header block).
  - Candidate 1: LPDDR3 type 7; \`dram_tpr6=0x2fb08080\` (matches the RG34XX DTB's \`dram_para14\`).
  - Candidate 2: LPDDR4 type 8; \`dram_tpr6=0x40808080\` (matches the RG34XX DTB's \`dram_para15\`).
- The candidate-specific routines call the low-level helper at `0x50980`, which manipulates DRAM-controller MMIO beginning at `0x03001540`; the `0x5287c` path also copies a 64-byte scratch block to/from physical address `0x40000000`, and `0x52984` performs further memory checks. The outer loop branches on the candidate DRAM type and requests DCDC3 1100 mV for type 8, 1200 mV for type 7, and 1500 mV for other types; the 1000 ms delay is visible in the disassembly.

This establishes that the stock SPL2 has a fixed three-entry candidate table and a bounded sequential candidate loop. It does **not** yet establish that the loop is selected by the DTB's \`dram_select_para\` GPIOs, nor that it implements SenkOS's "strap value 14 selects LPDDR3" rule. The GPIO configuration and the SPL2 candidate loop are separate pieces of evidence until the code path joining them is recovered.

A regression test now pins all three candidate records, loop/voltage instructions, controller-helper call sites and the \`0x40000000\` scratch address in the raw RG34XX Boot0 capture. \`0x538dc\` is a context-save helper, not the DRAM initializer. The next analysis should trace \`0x527d8\`, \`0x5287c\`, \`0x50980\`, and the outer caller/global state to determine candidate acceptance/failure semantics and whether strap inputs influence candidate order or selection.



## 10. BL31 PSCI dispatch: reset path found, system-off hook unresolved

The AArch64 monitor contains the TF-A PSCI core and the H700 platform path \`plat/sun50iw9p1/plat_pm.c\`. Static disassembly identifies the following dispatch behavior:

- The PSCI System Off handler around VMA \`0x9eb0\` obtains the primary PSCI platform-ops pointer from global address \`0x3c000 + 4032\`, then checks the System Off callback at structure offset \`+48\`. If the pointer is null, the generic TF-A path emits \`Platform has not exported a PSCI System Off hook.\` The handler also checks a separate platform-action table at global \`0x3c000 + 4040\`, offset \`+56\`, before calling the primary System Off callback.
- The PSCI System Reset handler around \`0x9f08\` checks the primary PSCI platform-ops callback at global \`0x3c000 + 4032\`, structure offset \`+56\`, and calls it indirectly when present. If it is null, it emits the corresponding generic missing-System-Reset-hook message. A separate platform-action callback at global \`0x3c000 + 4040\`, offset \`+64\`, is checked before the primary reset callback is invoked.
- A platform reset routine exists around \`0x2578\`: it logs \`ERROR: Sunxi System Reset\`, calls reset sequence \`0x1ec8\`, executes \`WFI\`, and if execution continues logs \`Sunxi System Reset: operation not handled.\` before entering the fatal path. This is strong evidence of a platform reset implementation in this monitor.
- The reset sequence at \`0x1ec8\` manipulates platform state and calls lower-level helpers; its complete PMU/RTC/WDT effects are not yet resolved.

No corresponding named \`sunxi_system_off\` implementation or platform System Off routine has been identified in the monitor strings/code reviewed so far. The PSCI dispatcher explicitly supports a nullable System Off callback at primary ops offset \`+48\`; the actual ops-table initialization and runtime pointer value have not yet been recovered. Therefore **PSCI_SYSTEM_OFF is unproven and must not be assumed available**; the generic missing-hook message is the null-callback path, not proof that this captured binary takes that path.

The monitor also contains generic PSCI CPU on/off/suspend code and the H700 \`plat_pm.c\` path, including universal-standby strings. This confirms the code family has suspend-related machinery, but it does not establish that the desired Linkslot product should use suspend; the project requirement remains no standby/suspend as a user-facing power mode.

The regression test \`tools/test_rg34xx_stock_toc1_dtb.py\` now pins the monitor's PSCI dispatch field offsets and the platform reset call/WFI sequence. Next work is to recover the actual PSCI platform-ops table initialization, prove whether System Off is exported, and trace the reset routine's PMIC/RTC/WDT dependencies.


## 11. Independent public UART sample remains separate

The published macromorgan RG34XX UART sample reports PMU AXP2202 and DRAM_VCC 1100 mV before DRAM training. That sample is not part of the reports-branch binary inventory, and the captured Boot0 image contains \`BOOT0 commit : %s\` rather than a fixed build hash. Do not match the public sample's displayed \`749c1f9a-dirty\` identifier to this binary or claim that the public log is this exact binary's runtime trace.

The public log remains useful independent RG34XX corroboration. The project's own read-only runtime capture reports DCDC3 at 1100 mV, and the stock SPL2 AArch64 code now provides a direct DCDC3 voltage-setting path. Whether the public UART's `DRAM_VCC` message originates in Boot0 or SPL2 remains unproven.

## 12. Required next static work

1. Trace candidate routines at \`0x527d8\` and \`0x5287c\`, controller helper at \`0x50980\`, and the state that selects/advances the three-entry table; determine whether PH6/PH7/PH8/PH10 strap inputs influence it.
2. Parse the TOC1 header/component table and validate embedded FDT blobs; identify BL31/U-Boot payload boundaries and the selected RG34XX-v1 FDT.
3. Trace the DCDC enable/read-modify-write call path and exact PMU probe sequence; the voltage-setting calls and R-TWI0 backend are already statically identified.
4. Establish whether the RG35XX-2024 SPL2 build label is a shared vendor target or board-specific, and verify its remaining DRAM/PMIC/power assumptions against RG34XX-v1 evidence.
5. Recover BL31 PSCI platform-ops initialization and prove whether System Off is exported; trace the reset routine's PMIC/RTC/WDT dependencies. No writes are authorized.

## 13. Safety state

This was read-only parsing, hashing, string extraction and static ARM/AArch64 disassembly only.

- No Boot0/TOC1/TF write.
- No PMIC/TWI transaction.
- No MMIO/SMC or physical I/O.
- No DRAM content access.
- No payload execution or firmware flashing.
- \`H700_MEMORY_LAYOUT_APPROVED=false\`
- \`hardware_applied=0\`
- \`candidate_eligible=false\`
- \`hardware_qualified=false\`
