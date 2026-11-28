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



## 8. R-TWI logical-bus mapping and SPL ordering closure

The remaining upstream I2C adapter ambiguity is resolved for the pinned H700 defconfig:

1. The H700 defconfig enables `CONFIG_R_I2C_ENABLE=y` and `CONFIG_SYS_I2C_MVTWSI=y`, without enabling ordinary TWI0/TWI1 adapters.
2. `arch/arm/include/asm/arch-sunxi/i2c.h` maps `CFG_I2C_MVTWSI_BASE2` to `SUNXI_R_TWI_BASE`.
3. `drivers/i2c/mvtwsi.c` registers the adapter as `twsi2` with hardware adapter number 2. The adapter's linked-list position is the logical I2C bus index; because it is the only adapter in this configuration, logical bus 0 resolves to this hardware adapter 2.
4. `drivers/i2c/i2c_core.c` routes `i2c_init()`, `i2c_read()` and `i2c_write()` through the current logical adapter. `i2c_init()` uses the default current bus index, 0.
5. `arch/arm/mach-sunxi/board.c:board_init_f()` calls `i2c_init_board()` and `i2c_init(CONFIG_SYS_I2C_SPEED, CONFIG_SYS_I2C_SLAVE)` before calling `sunxi_board_init()`.
6. The H616 `i2c_init_board()` branch enables TWI channel 5 and configures GPL0/GPL1 for R-TWI. The target DTS's `twi5` controller at `0x07081400` is the same R-TWI0 base.
7. `sunxi_board_init()` calls `axp_init()`, then the configured DCDC2/DCDC3 setters, and only afterwards prints/executes `sunxi_dram_init()`. In this upstream SPL reference, the PMIC rail programming therefore precedes DRAM initialization.

The PMIC access path for this configuration is consequently statically traceable as:

`board_init_f → i2c_init_board → i2c_init(logical bus 0) → sunxi_board_init → axp_init → pmic_bus_init → i2c_read/write(0x34) → twsi2(hardware adapter 2) → SUNXI_R_TWI_BASE`

The DCDC3 write path is:

`axp_set_dcdc3(1100) → pmic_bus_write(0x85, encoded_voltage) → pmic_bus_setbits(0x80, BIT(2))`

This closes the generic upstream U-Boot reference's controller-selection and pre-DRAM ordering. It does **not** prove that the target stock Boot0 binary used this exact source implementation or that the PMIC silicon behind the legacy `x-powers,axp2202` binding is identical to the AXP717 part assumed by upstream U-Boot. The AXP717 SPL driver's zero chip-ID mask makes that distinction especially important.

### Revised blocker status

- **Upstream U-Boot AXP717 → R-TWI0 → address 0x34 → DCDC3 register sequence:** statically traced.
- **Upstream U-Boot PMIC-before-DRAM ordering:** statically traced.
- **RG34XX target-specific physical PMIC identity and vendor AXP2202 register-map equivalence:** not yet proven.
- **Exact stock Boot0 implementation and equivalence to the upstream sequence:** not yet proven.
- **RG34XX DRAM strap values and Boot0 `DRAM.ext` selection:** not yet proven from target-specific strap evidence.

This means the former broad "PMIC initialization path unknown" blocker is narrowed to a **stock Boot0 provenance / physical PMIC identity equivalence** blocker. No PMIC writes are enabled in M1 preflight.


## 9. Target-device applicability correction (RG34XX, not RG35XX)

The project target is **Anbernic RG34XX**. SenkOS's H700 implementation and the inspected generic H700 defconfig are reference implementations from the RG35XX/H700 family, not target-specific RG34XX evidence. The prior sections must be read with this distinction.

### Evidence classification

**RG34XX-specific evidence**
- KNULLI's RG34XX boot-package DTS path and RG34XX-specific `sun50i-h700-anbernic-rg34xx.dts` are target-specific software descriptions.
- The public RG34XX Boot0 log identifies the target's reported PMU/DRAM operating point.
- The project's own read-only RG34XX runtime captures are target-specific observations.
- RG34XX-specific DTB selection is distinct from RG35XX H / RG35XX 2024 DTBs.

**H700-family reference evidence only**
- SenkOS SPL strap-selection patch and its PH6/PH7/PH8/PH10 mapping.
- SenkOS's DRAM parameter tables and LPDDR3 branch.
- The inspected `anbernic_rg35xx_h700_defconfig` values and AXP717 selection.
- The generic upstream H700 board initialization sequence and its PMIC-before-DRAM ordering.

These reference sources establish that an implementation pattern exists; they do not prove that RG34XX has the same PCB strap wiring, DRAM package, PMIC silicon/register map, voltage requirements, or Boot0 call sequence.

### Required RG34XX-only proof before using family references

1. Identify the exact RG34XX PCB/board revision and match its stock Boot0/boot-package lineage.
2. Establish RG34XX PH6/PH7/PH8/PH10 strap values from target-specific evidence; do not import SenkOS's index mapping by assumption.
3. Match the exact RG34XX DRAM package/type/geometry and stock Boot0 `dram_para`/`DRAM.ext` data before considering any timing-table reuse.
4. Use the upstream Linux maintainer and Allwinner Tina guide evidence that AXP2202 is the legacy BSP name/framework for AXP717-compatible silicon. Then recover the exact RG34XX Boot0 PMU ID probe, bus/address fallback, register writes and DRAM-init ordering. The DT compatible and shared address alone do not prove the target call path.
5. Trace the RG34XX-specific boot package / stock Boot0 PMIC access path independently. The RG35XX H700 defconfig's TWI and DCDC sequence remains a comparison reference only.

### Corrected conclusion

The SenkOS and upstream U-Boot findings narrow the search space and supply useful source-level hypotheses. They do **not** close the RG34XX PMIC/DRAM blockers. The project's target evidence remains the authority. No SenkOS strap values, voltage policy, DRAM timing table, or PMIC register writes are approved for RG34XX by this comparison.


### 10. AXP2202 / AXP717 naming evidence strengthened by upstream and Allwinner sources

The Linux upstream AXP717 support author states that Allwinner BSP code often calls the same chip AXP2202 and that a Boot0 AXP2202 driver successfully drove a chip labelled AXP717: https://lkml.iu.edu/2403.1/01550.html. Allwinner's Tina PMU guide explicitly says AXP717 uses the AXP2202 software framework name in kernel/DTS configuration: https://tina.100ask.net/SdkModule/Linux_PMU_DevelopmentGuide-02/.

An Allwinner-authored Tina v821 AXP2202 SPL reference (sun50iw10p1 lineage, not the RG34XX/H700 target source) and the pinned U-Boot AXP717 SPL use the same DCDC3 voltage register 0x85, output-enable register 0x80 bit 2, mask 0x7f, and 500–1840 mV split-step encoding. The AXP2202 reference additionally probes CHIP_ID_EXT 0x0e (expected value 0x02), initializes a 400 kHz TWI bus, and supports address fallback 0x34 -> 0x35. Its generic dispatcher exposes set_ddr_voltage() -> axp2202_set_ddr_voltage() -> DCDC3. These findings make the AXP717 SPL driver a credible register-family reference, not an incompatible-chip substitute.

The exact RG34XX Boot0 call site, PMIC read/write sequence, fallback selection, and DCDC3-before-DRAM ordering remain unproven. See [RG34XX AXP2202/AXP717 crosswalk](rg34xx-axp2202-axp717-crosswalk-20261128.md).
