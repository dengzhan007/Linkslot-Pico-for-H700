# RG34XX stock Boot0 DRAM.ext audit — 2026-11-28

## Scope and evidence

This is a read-only static inspection of the two RG34XX non-SP stock Boot0 captures in the reports branch:

- reports/rg34xx-inventory-20261127-uart/boot0-primary.bin
- reports/rg34xx-inventory-20261127-uart/boot0-secondary.bin

No device operation, register access, firmware write, DRAM access, or payload execution was performed.

Raw capture hashes from raw-sha256.txt:

| Capture | Raw capture bytes | Raw capture SHA-256 | eGON declared image bytes | SHA-256 of declared image |
|---|---:|---|---:|---|
| primary | 131072 | bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f | 131072 | same |
| secondary | 131072 | 5f143679e0d2c5293d22fee47a47ea6c17120c7a19e6007247c8b13710aa5ef9 | 65536 | 4701a85e4db7e8339b91b92f691a83cc954ec63a09369087c2e41d80eafa9f5d |

Both declared eGON images pass the Allwinner checksum validation. The secondary capture is 128 KiB long, but its own header declares a 64 KiB Boot0 image; the second 64 KiB must not be treated as part of that Boot0 image.

## Active header parameter block

Both copies contain a 32-word parameter block at offset 0x38:

- DRAM clock field: 672 MHz (0x2a0)
- DRAM type field: 8 (LPDDR4)
- DX ODT: 0x08080808
- DX DRI: 0x0e0e0e0e
- CA DRI: 0x00000e0e
- PARA0: 0x7887bbbb
- PARA1: 0x000030fa
- PARA2: 0x04000000
- TPR6: primary 0x3e808080; secondary 0x40808080
- TPR10: 0x402f6633
- TPR11: 0x24272824
- TPR12: primary 0x06070706; secondary 0x06070707
- TPR13: 0x02008c60
- TPR14: 0

The first-sector parameter block is mutable according to the existing inspector's evidence notes. The two captures are not byte-identical; this audit does not infer why the first-sector and body differences exist.

## DRAM.ext discovery and parser correction

The prior tools/inspect_h700_boot0.py reported only three fixed offsets: 0x38, 0x9f4, and 0xa74. That was insufficient for the 128 KiB primary image.

A full-image scan for the literal DRAM.ext marker, followed by the H616/H700 32-word profile layout and conservative structural checks, finds:

| Capture | DRAM.ext marker | Parameter records start | Structurally plausible contiguous records |
|---|---:|---:|---:|
| primary | 0x348 | 0x374 | 1 |
| primary | 0x1348 | 0x1374 | 15 |
| secondary | 0x348 | 0x374 | 15 |

The 15-record tables at primary 0x1348 and secondary 0x348 agree on the following broad pattern:

- Entries 0–12: DDR3, 792 MHz.
- Entry 13: LPDDR3, 672 MHz.
- Entry 14: LPDDR4, 672 MHz.

Therefore the LPDDR3/LPDDR4 records in the primary image are at 0x19f4 and 0x1a74, not the old fixed offsets 0x9f4 and 0xa74. The old parser consequently showed zero-filled “alternates” for the primary image even though its later DRAM.ext region contains the 15-profile table.

The primary marker at 0x348 yields one structurally plausible DDR3 record before the following bytes stop matching the profile shape. It is retained as a one-record discovery, not assumed to be the runtime-selected table. A further DRAM.ext string in the diagnostic text area does not yield a plausible profile table.

### Parser changes

tools/inspect_h700_boot0.py now:

1. Preserves the legacy fixed-offset fields for compatibility with existing reports.
2. Scans every DRAM.ext marker in the declared eGON image.
3. Locates the first 32-word profile 44 bytes after the marker, following the H616/H700 parsing convention.
4. Reports contiguous profiles only while the conservative checks pass: clock 100–1200 MHz, recognized DRAM type, and valid high-nibble shape for DX ODT/DRI.
5. Emits named fields and the source offset for each discovered profile.
6. Explicitly states that profile extraction does not prove Boot0 runtime selection, PMIC writes, or the physical DRAM map.

The 44-byte marker-to-profile offset and 15-slot DRAM.ext count follow the constants in apritzel/sunxi-fw main (DRAM_EXT_MAGIC_OFS=210, DRAM_EXT_PARA_OFS=221, DRAM_EXT_PARA_SIZE=32, DRAM_EXT_PARA_CNT=15):
https://github.com/apritzel/sunxi-fw/blob/main/sunxi-boot0.c

The 32-word H616/H700 field layout and DX ODT/DRI plausibility checks are cross-referenced against the H616/H700 parser revision in apritzel/sunxi-fw commit 1ba5f4c:
https://github.com/apritzel/sunxi-fw/blob/1ba5f4c/sunxi-boot0.c

The tool's checks are a format plausibility filter, not a hardware qualification test.

A synthetic regression covers multiple markers, a relocated 15-entry table, and termination at a code-like invalid record. A report-branch regression validates raw capture hashes, declared-image hashes, eGON checksums, active profile fields, and the primary/secondary table locations.

## Runtime log correlation

The public RG34XX Boot0 UART capture reports:

- PMU: AXP2202
- DRAM_VCC set to 1100 mv
- DRAM clock 672 MHz
- DRAM type 8 (LPDDR4)
- DRAM size 1024 MBytes
- successful DRAM simple test / initialization
- vaild para:8 select dram para0

The Boot0 binary also contains PMU identification strings for AXP2202 and other PMU families. Together with the runtime log, this is strong target-specific evidence that the stock RG34XX Boot0 selected its AXP2202 path. It does not yet reveal the exact AXP2202 register writes, their order, or the I2C bus transaction sequence.

DRAM_VCC 1100 mV is retained as the exact log label/value. This audit does not map it to a particular DCDC output or equate it to the upstream U-Boot AXP717 DCDC3 setting.

## AXP2202 / AXP717 naming reconciliation

The Boot0 UART label PMU: AXP2202 is consistent with the AXP717-compatible PMIC family. Linux upstream AXP717 support author Andre Przywara states that Allwinner BSP often calls the same chip AXP2202 and that a Boot0 AXP2202 driver successfully drove a chip labelled AXP717: https://lkml.iu.edu/2403.1/01550.html. Allwinner's Tina PMU guide explicitly says AXP717 uses the AXP2202 software framework name in DTS/kernel configuration: https://tina.100ask.net/SdkModule/Linux_PMU_DevelopmentGuide-02/.

An Allwinner-authored Tina v821 AXP2202 SPL reference (sun50iw10p1 lineage, not the exact RG34XX/H700 source) and pinned U-Boot AXP717 SPL use the same DCDC3 voltage register 0x85 and enable register 0x80 bit 2, with matching mask and split-step range. The Tina driver also exposes a DCDC3 DDR-voltage helper and a 0x34/0x35 PMIC address probe. This removes the assumption that AXP2202 and AXP717 are incompatible, but does not prove that the RG34XX Boot0 binary uses this exact source or sequence. See docs/rg34xx-axp2202-axp717-crosswalk-20261128.md.

## Consequences for the project

1. The target-specific stock Boot0 captures now provide a better basis for the DRAM profile investigation than SenkOS's RG35XX-family strap patch.
2. The primary/secondary parser omission is corrected; previous JSON files remain historical outputs and should not be used as exhaustive descriptions of embedded DRAM profiles.
3. AXP2202 is now a stronger target-specific PMIC-path conclusion than a DTS-compatible-only inference. The AXP717 SPL implementation remains a reference only; do not transplant its register map or voltage policy.
4. The next static task is to recover the exact RG34XX Boot0 AXP2202 branch: identify the PMU bus initialization, PMU ID detection, register address/value writes, and the ordering relative to DRAM initialization. This requires disassembly and/or matching vendor source; strings alone do not prove register writes.
5. The DRAM.ext table does not prove GPIO strap use. PH6/PH7/PH8/PH10 remain unverified for RG34XX; SenkOS's mapping remains reference-only.

## Safety state

Unchanged:

- H700_MEMORY_LAYOUT_APPROVED=false
- hardware_applied=0
- candidate_eligible=false
- hardware_qualified=false
- No payload execution, DRAM destination copy, physical device I/O, MMIO/SMC, TWI transaction, PMIC write, TF/Boot0/TOC1 write, or firmware flashing.
- DRAM source pages 0x40000000 and 0x60000000 remain excluded from scratch use.
- M1 remains preflight-only; PMIC/TWI write paths remain disabled.
