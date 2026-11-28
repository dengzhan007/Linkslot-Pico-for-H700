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
