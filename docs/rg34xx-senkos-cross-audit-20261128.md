# RG34XX / SenkOS cross-project boot-chain audit

Date: 2026-11-28
Scope: public-source comparison only; no firmware execution, device register access, media writes, or hardware application.

## Executive result

SenkOS provides a directly relevant H700 SPL implementation for selecting LPDDR3 versus LPDDR4 from Boot0-compatible board straps, and for setting the DRAM supply through the AXP717 U-Boot PMIC interface. This materially improves the source-comparison path for the current Profile A investigation.

It does **not** yet close the RG34XX PMIC/DRAM provenance gate. SenkOS documents RG35XX-SP as its primary hardware platform and separately includes RG34XX device support. Its strap observations and tested LPDDR3 branch must not be assumed to describe this project's RG34XX board without matching target evidence.

## 1. SenkOS source findings

Repository: https://github.com/HookedBehemoth/SenkOS
Inspected revision: `802be0c07939ab4f729f901d3c275101754e9e34`

Relevant files:
- `docs/straps.md`
- `docs/hardware.md`
- `yocto/meta-anbernic/recipes-bsp/u-boot/files/0008-sunxi-H616-pick-LPDDR3-or-LPDDR4-by-the-boot0-DRAM-straps.patch`
- `yocto/meta-anbernic/recipes-bsp/u-boot/files/0007-sunxi-axp717-no-boot-on-plug-in.patch`
- `yocto/meta-anbernic/recipes-bsp/u-boot/u-boot_%.bbappend`
- `yocto/meta-anbernic/recipes-bsp/u-boot/files/dram-straps.cfg`

The 0008 patch:
- Adds `CONFIG_SUNXI_DRAM_H616_LPDDR3_STRAPS`, dependent on H616 DRAM support and `AXP717_POWER`.
- Reads PH6, PH7, PH8 and PH10 as inputs with pull-ups, waits, samples the four-bit value, then disables the pin configuration.
- Selects LPDDR3 when the sampled index is 14; otherwise selects the LPDDR4 parameter set in this patch.
- Selects type-specific PHY address tables and LPDDR3 timing parameters, and calculates timing from the selected DRAM clock rather than a fixed build-time clock.
- In `sunxi_board_init()`, routes the DCDC3 setting through `axp_set_dcdc3()`: 1200 mV for the LPDDR3 strap branch, otherwise `CONFIG_AXP_DCDC3_VOLT`.
- Expands the DCDC3 regulator range in the DTS to 1.1–1.2 V so later U-Boot/Linux regulator handling can retain the SPL-selected rail voltage.

SenkOS documents Boot0's 32-word `dram_para` block and the `tpr3` DRAM_VCC encoding: 10 mV steps, one byte per memory type, with defaults of 1100 mV for LPDDR4 and 1200 mV for LPDDR3. It also documents Boot0's `DRAM.ext` table and the PH6/PH7/PH8/PH10 selection index.

Important limitation: this is a SenkOS patch and documentation set, not proof that the same strap mapping, timing table, PMIC operation order, or voltage policy is correct for the target RG34XX. The exact stock Boot0 revision and target board strap sample still need to be joined to this evidence.

## 2. PMIC naming and bus reconciliation

The target project's captured RG34XX live-FDT inventory and KNULLI RG34XX boot-package DTS use:
- PMIC compatible: `x-powers,axp2202`
- PMIC I2C address: `0x34`
- Parent controller: `twi5`
- Controller base: `0x07081400` (R-TWI0)

KNULLI source inspected:
https://github.com/knulli-cfw/knulli-linux/blob/knulli-main/package/boot/uboot-h700/rg34xx/boot_package/rg34xx.dts
File blob: `4c5b7c804ffc1ddf847071285ce4960dd12b9750`

SenkOS describes its H700 PMIC as AXP717 at I2C address `0x34`, and its U-Boot integration selects `AXP717_POWER`. Its mainline Linux DTS lineage uses the `x-powers,axp717` binding. KNULLI's older/vendor boot-package DTS uses the `x-powers,axp2202` binding at the same address.

Disposition:
- Treat AXP2202 versus AXP717 first as a **binding/driver-lineage reconciliation question**, not proof of two different physical PMICs.
- The shared address and rail naming are useful cross-checks, but do not alone prove silicon identity or register-level equivalence.
- Before enabling the upstream AXP717 path in the target SPL, prove the physical PMIC identity and map the target's vendor AXP2202 register accesses to the AXP717 driver operations, including bus controller selection and error handling.
- Do not translate DTS regulator ranges into writes, and do not import an unrelated board's voltage values.

## 3. Cross-check against RG34XX evidence already in this project

The project's 2026-11-28 status records three read-only runtime captures from 2026-11-25/26/27:
- AXP2202 DCDC3: 1,100 mV, enabled.
- DCDC2: 940 mV, enabled.
- DCDC1: approximately 1.15–1.16 V.

The public RG34XX Boot0 log by macromorgan records:
- Boot0 commit `749c1f9a-dirty`.
- PMU identified by the Boot0 log as AXP2202.
- `DRAM_VCC set to 1100 mv`.
- DRAM clock 672 MHz, type 8 (LPDDR4), size 1024 MiB.
- DRAM training and simple memory test success.

Log: https://gist.github.com/macromorgan/3e12146a63b53c5b61e4e3c7d4547a0e

This is a strong consistency check for the RG34XX LPDDR4 / 1.1 V operating point. It does not identify the exact register write that produced that voltage, nor prove that SenkOS's SPL sequence is identical to stock Boot0.

## 4. Consequences for the current blockers

### Narrowed

1. The PMIC compatible mismatch is better classified as a legacy-vendor-binding versus upstream-driver naming issue. The AXP717 U-Boot driver path is now a concrete candidate to audit, not an automatic substitute.
2. DCDC3 is independently identified by SenkOS as the DRAM supply. The project's RG34XX 1.1 V runtime capture and the RG34XX Boot0 log's 1.1 V message are consistent with LPDDR4.
3. SenkOS provides a practical reference for strap-driven DRAM parameter selection and for keeping the DRAM supply range wide enough to preserve the SPL-selected voltage.

### Still blocked

1. Exact RG34XX PH6/PH7/PH8/PH10 strap values and their correspondence to Boot0's `DRAM.ext` index have not been independently established in the project's captured evidence.
2. The exact RG34XX stock Boot0/SPL PMIC initialization call path, controller, slave/register addresses, values, ordering, NACK handling and necessity are not yet proven.
3. AXP2202 vendor binding to AXP717 upstream driver equivalence is not yet proven at the register-map and silicon level.
4. The timing of the DCDC3 write relative to DRAM training and resume/self-refresh must be traced from the exact target startup lineage; SenkOS's implementation must not be copied as a sequence without this trace.

## 5. Next static tasks

1. Compare the RG34XX Boot0 header's 32-word `dram_para` and `DRAM.ext` data against the published Boot0 log and the project's captured 1 GiB / LPDDR4 evidence. Record each field separately; do not infer missing bytes from a log.
2. Audit SenkOS patch 0008 and the pinned U-Boot v2026.10 AXP717 driver side by side. Extract the exact `axp_set_dcdc3()` call path, TWI controller setup, PMIC address, register encoding, voltage conversion and error propagation.
3. Trace the RG34XX stock Boot0 binary's PMIC identification and DCDC3 programming path, using immutable binary hash and instruction/data-flow evidence. Compare it with the SenkOS path; do not treat string matches as proof of the write.
4. Check whether RG34XX target strap inventory already exists in the reports branch. If not, define a read-only evidence request for the next already-approved diagnostic capture; do not perform register writes or firmware changes.
5. Keep current M1 read-only preflight Kconfig PMIC/TWI-disabled. The future Profile A DRAM-capable SPL is a separate design/audit target.
6. Continue host-only RTC/WDT state-machine regression separately; do not reuse raw-capsule-v2 semantics.

## 6. CI checkpoint

Linkslot feasibility run #1075:
https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46141495743

- Commit: `807b3b2dc566b9fec1f11d4009b07afeea1ed0d7`
- Cross-compile job: completed successfully.
- All 92 primary workflow steps completed successfully, including the actual M1 preflight-only SPL link, reachable-callgraph report, linked-artifact fail-closed contract, Linkslot/LK cross-compilation, storage and FAT regressions, mGBA builds/link probes, and compact artifact upload.
- Compact artifact: `linkslot-lk-rust-feasibility-46141495743`, 551,201 bytes.
- This confirms build/link and static/host regression success only. It does not authorize payload execution, physical device I/O, DRAM access, media writes, or firmware installation.

Safety state remains:
- `H700_MEMORY_LAYOUT_APPROVED=false`
- `hardware_applied=0`
- `candidate_eligible=false`
- `hardware_qualified=false`
- No payload execution, DRAM destination copy, TF/Boot0/TOC1 writes, or firmware flashing.
- DRAM source pages `0x40000000` and `0x60000000` remain excluded.


## 7. Pinned U-Boot v2026.10 AXP717 SPL driver trace

Pinned upstream source revision used by the project: `5508406582f6f7120dce0c4b3059819a41788db2`.

Inspected:
- `configs/anbernic_rg35xx_h700_defconfig`
- `drivers/power/axp_spl.c`
- `arch/arm/mach-sunxi/pmic_bus.c`
- `board/sunxi/board.c`
- `arch/arm/include/asm/arch-sunxi/i2c.h`
- `drivers/power/Kconfig`

The generic H700 defconfig is materially closer to the RG34XX runtime evidence than the earlier compatible-string comparison suggested:
- `CONFIG_AXP717_POWER=y`
- `CONFIG_AXP_DCDC2_VOLT=940`
- `CONFIG_AXP_DCDC3_VOLT=1100`
- `CONFIG_R_I2C_ENABLE=y`
- `CONFIG_SPL_I2C=y`
- `CONFIG_SPL_SYS_I2C_LEGACY=y`
- `CONFIG_SYS_I2C_MVTWSI=y`
- `CONFIG_SYS_I2C_SPEED=400000`
- `CONFIG_AXP_I2C_ADDRESS` defaults to `0x34` for this configuration.

These DCDC2/DCDC3 values match the project's read-only RG34XX captures (940 mV / 1,100 mV) and the public RG34XX Boot0 log's 1,100 mV DRAM_VCC report. This is a strong configuration-level match, but the defconfig is named for the RG35XX H700 family and is not, by itself, proof that the exact RG34XX stock boot stage used this source.

The pinned AXP717 SPL driver maps DCDC3 as:
- Voltage register: `0x85`
- Voltage mask: `0x7f`
- Enable register: `0x80`
- Enable bit: `BIT(2)`
- Range: 500–1840 mV, 10 mV steps, split encoding at 70 steps.

The `axp_set_dcdc3(mV)` wrapper calls the common DCDC setter. For nonzero voltage it writes the encoded value to `0x85`, then calls `pmic_bus_setbits(0x80, BIT(2))`. The setbits helper performs a register read, returns without a write if the bit is already set, otherwise writes the read value OR the enable bit. The voltage write itself is not read back by this helper.

The AXP717 SPL branch sets its chip-version register/mask/ID constants to zero; `axp_init()` therefore initializes the bus but does not identify the physical PMIC by reading a chip-ID register. This is a critical limitation: selecting `AXP717_POWER` is not itself proof that an attached device is AXP717-compatible.

The H616 `board/sunxi/board.c` path under `CONFIG_R_I2C_ENABLE` enables TWI channel 5 and configures GPL0/GPL1 for the H616 R-TWI function. The architecture header maps the R-TWI controller as `SUNXI_R_TWI_BASE`. This aligns with the target DT's `twi5` node at `0x07081400` and KNULLI's `0x34` PMIC node.

Remaining bus-path gap: `pmic_bus.c` ultimately calls legacy `i2c_read/i2c_write(CONFIG_AXP_I2C_ADDRESS,...)` when neither P2WI nor RSB is selected. We have not yet closed the full active-bus selection and SPL call ordering proving that these calls reach R-TWI0 on the target RG34XX in the exact build. That must be traced before treating the register sequence as the target's approved allowlist.

### Updated disposition

The previous broad statement that the target AXP2202 compatible versus upstream AXP717 support is a material incompatibility is now **too strong**. The evidence supports a plausible legacy vendor binding / upstream AXP717 driver relationship:
- same target bus address 0x34;
- same R-TWI0/twi5 controller candidate;
- AXP717 SPL DCDC2/DCDC3 configuration values match the target's measured runtime rails;
- RG34XX Boot0 log independently reports the corresponding DRAM voltage.

Classify this as **strongly corroborated candidate; exact silicon identity and active bus call path still unproven**. Do not enable PMIC writes in the M1 preflight. Profile A remains a separate future DRAM-capable SPL audit.

