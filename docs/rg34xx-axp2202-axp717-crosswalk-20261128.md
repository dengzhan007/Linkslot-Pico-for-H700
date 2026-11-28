# RG34XX AXP2202 / AXP717 SPL crosswalk — 2026-11-28

## Executive correction

The AXP2202 versus AXP717 naming difference must not be treated as evidence of two incompatible PMICs.

Two independent sources explain the naming:

- Linux upstream AXP717 support author Andre Przywara states that Allwinner BSP code often calls the same chip AXP2202, and that a Boot0 AXP2202 driver successfully drove a chip physically labelled AXP717:
  https://lkml.iu.edu/2403.1/01550.html
- Allwinner's Tina PMU guide explicitly says AXP717 uses the AXP2202 software framework name in kernel configuration and DTS compatible strings:
  https://tina.100ask.net/SdkModule/Linux_PMU_DevelopmentGuide-02/

This makes the KNULLI RG34XX DTS compatible string x-powers,axp2202 and the SenkOS / upstream U-Boot AXP717 naming consistent with a legacy BSP alias. It removes the prior assumption of a PMIC-family incompatibility.

## Allwinner AXP2202 SPL driver reference

Inspected Allwinner-authored source in sam-yangjj/tina-v821-v1.3-brandy, commit 34809037526678ccda1720cb4b4ec7ee32272c38:

- brandy-2.0/spl/drivers/power/axp2202.c
  https://github.com/sam-yangjj/tina-v821-v1.3-brandy/blob/34809037526678ccda1720cb4b4ec7ee32272c38/brandy-2.0/spl/drivers/power/axp2202.c
- brandy-2.0/spl/include/arch/axp2202_reg.h
  https://github.com/sam-yangjj/tina-v821-v1.3-brandy/blob/34809037526678ccda1720cb4b4ec7ee32272c38/brandy-2.0/spl/include/arch/axp2202_reg.h
- brandy-2.0/spl/drivers/power/axp.c
  https://github.com/sam-yangjj/tina-v821-v1.3-brandy/blob/34809037526678ccda1720cb4b4ec7ee32272c38/brandy-2.0/spl/drivers/power/axp.c

This source is a different Allwinner platform lineage (the Tina guide's example is sun50iw10p1), not the exact RG34XX/H700 Boot0 source. It is therefore a register/driver reference, not proof of the target call sequence.

### Register and bus crosswalk

| Function | Allwinner AXP2202 SPL | Pinned U-Boot AXP717 SPL |
|---|---|---|
| DCDC3 voltage register | 0x85 | 0x85 |
| DCDC3 enable register | 0x80 | 0x80 |
| DCDC3 enable bit | bit 2 | bit 2 |
| DCDC3 voltage field | mask 0x7f | mask 0x7f |
| Voltage range | 500–1840 mV | 500–1840 mV |
| Step encoding | 500–1200 mV by 10 mV; 1220–1840 mV by 20 mV | same split encoding |
| Default runtime address | 0x34 | 0x34 |
| Address fallback | 0x35 | no equivalent fallback in the inspected SPL path |
| Bus speed | 400 kHz under CFG_SUNXI_TWI | 400 kHz in inspected H700 defconfig |
| PMIC identity probe | CHIP_ID_EXT register 0x0e, expects value 0x02 | inspected SPL ID mask is zero; no chip-ID read |

The Allwinner driver exposes axp2202_set_ddr_voltage(set_vol), which calls pmu_set_vol("dcdc3", set_vol, 1). Its pmu_set_vol path reads DCDC3 voltage register 0x85, encodes the requested voltage, writes 0x85, reads output-control register 0x80, sets bit 2, and writes 0x80. Bus read/write failures are returned as errors.

The generic Allwinner axp.c dispatcher routes set_ddr_voltage() to axp2202_set_ddr_voltage() when the detected PMU is AXP2202. This establishes that the vendor SPL framework has a DCDC3-as-DDR-voltage operation. It does not prove that the RG34XX board.c calls it, at what voltage, or before which DRAM initialization function.

The AXP2202 axp_init() path initializes a 400 kHz TWI bus, first probes address 0x34 and can fall back to 0x35, reads CHIP_ID_EXT 0x0e, and identifies the expected AXP2202 PMU value 0x02. This is a more complete identity-detection path than the inspected upstream U-Boot AXP717 SPL helper, whose ID mask is zero.

### Other initialization writes are not automatically DRAM prerequisites

The inspected AXP2202 axp2202_set_necessary_reg() helper writes VBUS current limit, ADC enable, VSYS minimum, DCDC1 undervoltage-protection, and module-enable state. These are general PMU initialization actions. The source does not establish that all of them are required for DRAM training, and none should be copied into the project's Profile A without target call-path evidence.

## RG34XX target evidence correlation

Target-specific evidence now includes:

- RG34XX non-SP stock Boot0 UART log: PMU identified as AXP2202; DRAM_VCC 1100 mV; LPDDR4 type 8; 672 MHz; 1 GiB; DRAM simple test passed.
- RG34XX read-only runtime rail captures: DCDC2 940 mV and DCDC3 1100 mV enabled.
- RG34XX live FDT and KNULLI RG34XX boot-package DTS: compatible x-powers,axp2202 at I2C address 0x34 under twi5 / R-TWI0 base 0x07081400.
- Pinned H700 U-Boot v2026.10 defconfig: AXP717_POWER, R-I2C, SPL legacy I2C, 400 kHz, address 0x34, DCDC2 940 mV, DCDC3 1100 mV.
- Pinned U-Boot AXP717 SPL: DCDC3 register/enable map matches the Allwinner AXP2202 SPL reference above.

These observations are mutually consistent with the AXP2202 BSP label referring to AXP717-compatible silicon/registers. They do not prove that the RG34XX stock Boot0 was built from the inspected Tina v821 source or that its PMIC writes exactly match either reference implementation.

## RG34XX Boot0 PMU-probe log clue

The target Boot0 UART log shows two generic [pmu]: bus read error messages before PMU: AXP2202. The captured primary Boot0 binary contains these PMU-related strings (file offsets):

| Offset | String |
|---:|---|
| 0xD15C | ic cant match axp, please check... |
| 0xE657 | [pmu]: bad split val(%d) for %s |
| 0xE678 | [pmu]: unable to set %s |
| 0xE6A3 | [pmu]: bus init error |
| 0xE6BA | [pmu]: bus read error |
| 0xE6D1 | PMU: AXP858 |
| 0xE6DE | unknow PMU |
| 0xE702 | PMU: AXP2202 |
| 0xE716 | PMU: AXP1530 |
| 0xE736 | PMU: AXP806 |

This is consistent with a generic multi-PMU probe framework. In the inspected Tina v821 AXP2202 driver, a failed read at 0x34 logs one bus-read error and retries 0x35; if that second read also fails, the AXP2202 probe returns failure and cannot then print PMU: AXP2202. Therefore the two Boot0 errors followed by a successful AXP2202 identification are unlikely to be explained by that one AXP2202 address-fallback path alone; they may be failed probes of other PMU families. The exact Boot0 probe order is not recovered from strings.

### Stronger probe-order hypothesis from the generic dispatcher

The inspected Tina v821 generic axp.c probe order is AXP858, AXP81X, AXP806, AXP809, AXP2101, AXP2202, then AXP152/AXP1530 and later families. The AXP858 and AXP806 probe functions each call pmic_bus_read() for their PMU type and emit the same generic bus-read error when that read fails. The Boot0 binary contains PMU labels for AXP858, AXP806, AXP2202 and AXP1530; it does not contain the AXP81X/AXP809/AXP2101 labels in the extracted PMU string set.

If the target Boot0 uses this dispatcher ordering and has AXP858, AXP806, AXP2202 and AXP1530 enabled, the two initial bus-read errors are consistent with failed AXP858 and AXP806 probes, followed by successful AXP2202 detection. This is currently the best source-backed explanation of the observed log order.

It is still a hypothesis: the inspected Tina v821 source is not proven to be the exact source revision used to build the RG34XX Boot0 commit 749c1f9a-dirty. The target binary's exact compile-time PMU set and call order must be established by disassembly or an exact source match. The later U-Boot stage's AXP152/AXP1530 failures are a separate stage and must not be substituted for Boot0 probe identities.

Source references:
- Generic probe order: https://github.com/sam-yangjj/tina-v821-v1.3-brandy/blob/34809037526678ccda1720cb4b4ec7ee32272c38/brandy-2.0/spl/drivers/power/axp.c
- AXP858 failed-read probe: https://github.com/sam-yangjj/tina-v821-v1.3-brandy/blob/34809037526678ccda1720cb4b4ec7ee32272c38/brandy-2.0/spl/drivers/power/axp858.c
- AXP806 failed-read probe: https://github.com/sam-yangjj/tina-v821-v1.3-brandy/blob/34809037526678ccda1720cb4b4ec7ee32272c38/brandy-2.0/spl/drivers/power/axp806.c
- AXP2202 0x34 to 0x35 fallback and ID check: https://github.com/sam-yangjj/tina-v821-v1.3-brandy/blob/34809037526678ccda1720cb4b4ec7ee32272c38/brandy-2.0/spl/drivers/power/axp2202.c


The later RG34XX U-Boot stage in the same public log explicitly reports failed AXP152 and AXP1530 probes before PMU: AXP2202. That supports the existence of a multi-family probe sequence on this device, but it is a later boot stage and does not prove Boot0's exact probe order.

The same Boot0 log places DRAM_VCC set to 1100 mv after chip-id check and before the DRAM DST training loop. The Allwinner AXP2202 SPL reference exposes set_ddr_voltage() -> axp2202_set_ddr_voltage() -> DCDC3, while the target's post-boot read-only runtime captures show DCDC3 at 1100 mV. This makes DCDC3 the strongest candidate for the logged DRAM_VCC rail. It remains a candidate until the exact RG34XX Boot0 call site and write sequence are recovered; do not infer a PMIC write from the log text alone.

## Revised blocker classification

The AXP2202/AXP717 name difference is **not an incompatibility blocker**. It is a legacy BSP naming and source-lineage issue with strong evidence of shared PMIC silicon/register semantics.

The remaining blocker is narrower and target-specific:

1. Recover the actual RG34XX stock Boot0 AXP2202 code path, by disassembly and/or matching source.
2. Identify the actual TWI controller and slave address selected in that path, including any 0x34/0x35 fallback.
3. Recover the exact PMIC register writes and their error handling.
4. Determine whether DCDC3 is changed to 1100 mV before DRAM training, whether the rail is already configured by power-on defaults, or whether the log simply reports an existing voltage.
5. Establish which writes are necessary for DRAM training and which are unrelated general PMU setup.
6. Keep SenkOS GPIO strap mapping and its board-specific DRAM tables reference-only until independently matched to RG34XX.

## Safety state

No PMIC write, TWI transaction, MMIO/SMC, physical I/O, DRAM content access, Boot0/TOC1/TF write, firmware flash, or payload execution was performed.

- H700_MEMORY_LAYOUT_APPROVED=false
- hardware_applied=0
- candidate_eligible=false
- hardware_qualified=false
- M1 preflight remains PMIC/TWI-write-disabled.
- Profile A remains the approved policy, but no DRAM-capable SPL implementation or hardware action is authorized by this audit.
