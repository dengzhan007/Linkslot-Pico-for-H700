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
- Exactly five bytes differ: offsets \`0x0c\`, \`0x0d\` (checksum), \`0x34\` (debug byte), \`0xa3\` (active \`dram_tpr6\` high byte), and \`0xac\` (active \`dram_tpr12\` low byte).

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

## 6. Independent public UART sample remains separate

The published macromorgan RG34XX UART sample reports PMU AXP2202 and DRAM_VCC 1100 mV before DRAM training. That sample is not part of the reports-branch binary inventory, and the captured Boot0 image contains \`BOOT0 commit : %s\` rather than a fixed build hash. Do not match the public sample's displayed \`749c1f9a-dirty\` identifier to this binary or claim that the public log is this exact binary's runtime trace.

The public log remains useful independent RG34XX corroboration. The project's own read-only runtime capture independently reports DCDC3 at 1100 mV. Together with the Allwinner AXP2202 \`set_ddr_voltage()\` reference path, this makes DCDC3 the strongest DRAM_VCC candidate, but the exact target Boot0 write/call site remains open.

## 7. Required next static work

1. Disassemble the outer Boot0, embedded 64 KiB Boot0, and SPL2 component separately; map SPL2 at candidate VMA \`0x50000\`.
2. Scan the TOC1 package for board labels, DTB identities, BL31/BL33 component metadata, and PMIC/DRAM strings.
3. Compare the RG35XX-2024-labelled SPL2's DRAM/PMIC/strap/power code against RG34XX-v1 live FDT and target captures.
4. Establish whether the SPL2 label is a shared vendor build target or a board-specific hardware configuration before any reuse in the project.
5. Recover the exact target PMU probe and DCDC3 write call path; no writes are authorized by this audit.

## Safety state

This was read-only parsing, hashing, string extraction and static ARM disassembly only.

- No Boot0/TOC1/TF write.
- No PMIC/TWI transaction.
- No MMIO/SMC or physical I/O.
- No DRAM content access.
- No payload execution or firmware flashing.
- \`H700_MEMORY_LAYOUT_APPROVED=false\`
- \`hardware_applied=0\`
- \`candidate_eligible=false\`
- \`hardware_qualified=false\`
