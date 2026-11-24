# RG34XX H700 A32 BL33 image packaging

This document describes the host-side image wrapper required to turn the
GammaOS LK H700 raw binary into the layout used by the currently deployed
RG34XX native A32 BL33 slot. It does not claim the resulting image boots.

## 1. GammaOS LK has an MTK wrapper

The pinned GammaOS LK \`make/build.mk\` first runs \`objcopy -O binary\`,
saves the flat output as \`lk-no-mtk-header.bin\`, then invokes
\`scripts/mkimage\` using \`img_hdr_lk.cfg\` (\`NAME = lk\`) and replaces
\`lk.img\` with the result. The H700 wrapper consumes \`lk-no-mtk-header.bin\`,
not the normal MTK-header \`lk.img\`.

The H700 CRT0 overlay places an unconditional A32 branch at image offset
\`+0\`, reserves \`+4..+0x63f\` for the vendor header, and places the A32
exception vector table at \`+0x1000\`. The first branch targets the LK BL33
entry wrapper, which sets VBAR before entering the original LK reset path.

## 2. Active RG34XX reference header

The reports branch contains a host-only extraction of the active native
firmware package from p4 \`+40 MiB\`:

| Item | Value |
|---|---|
| Firmware package | 287,232 bytes; SHA-256 \`dc9dc3ae66b30244911e7054ca66136991626cabb0d59c796c4b253a126f28e9\` |
| Firmware offset | \`0x02800000\` |
| A32 loader extent | 32,768 bytes |
| A32 loader SHA-256 | \`48b2a837ac5d8daad8f524c976f2b3911d26a838f8b68fa695e3283412dd8590\` |
| Load address | \`0x4a000000\` |
| First instruction | A32 branch to \`0x4a001000\` |
| Header magic | ASCII \`uboot\` at \`+4\` |

The extracted report records additional observed fields, but their semantics
are not established here. The packager preserves them byte-for-byte rather
than guessing their meanings.

The BaseOS native loader builder copies bytes \`package[0x804:0xe40]\` from
its matching U-Boot TOC payload into loader offsets \`[+4,+0x640)\`. At
runtime, native SPL updates these fields in the loaded BL33 header:

| Image offset | Native SPL action |
|---|---|
| \`+0x30..+0xaf\` | Copy selected 128-byte DRAM parameter block |
| \`+0xe4..+0xe7\` | Store \`1\` |
| \`+0x4e8..+0x4ea\` | Store bytes \`00 01 00\` |
| \`+0x4f4..+0x4f7\` | Store \`0x00140000\` |
| \`+0x4f8..+0x4fb\` | Store detected RAM size in MiB |

These writes are visible in the deployed native SPL source. The remaining
header bytes are opaque and are copied exactly from the known-working
reference loader.

## 3. Host wrapper

\`scripts/package-h700-bl33.py\` accepts:

- \`lk-no-mtk-header.bin\` from a complete H700 GammaOS LK build;
- \`loader.bin\` extracted from the currently deployed p4+40 MiB native
  firmware package.

The packager requires the reference loader to be exactly 32 KiB and match
the audited SHA-256 above. It verifies the \`uboot\` magic, the raw LK first
A32 non-link branch, all eight A32 vector branches, the reserved/zero raw
header window, 512-byte output alignment, and the observed \`0xe0000\`
upper size limit. It copies only reference bytes \`[+4,+0x640)\`; it never
copies the reference loader's first branch or code.

Example after performing the read-only firmware extraction locally:

\`\`\`sh
python tools/h700_boot_image.py extract-firmware --partition build/rg34xx-inventory-20261122-214708/boot-partition.bin --active-image build/rg34xx-active-inputs/active-boot.img --output-dir build/rg34xx-native-firmware
python scripts/package-h700-bl33.py --lk-raw build-rg34xx_h700/lk-no-mtk-header.bin --reference-loader build/rg34xx-native-firmware/loader.bin --output build-rg34xx_h700/lk-h700-bl33.bin
\`\`\`

The current reference SHA pin intentionally refuses arbitrary U-Boot
binaries or unverified headers. If Boot0, BL31, native SPL or the p4+40 MiB
firmware package changes, re-extract and re-qualify the header before
changing the pin.

## 4. Scope and remaining gates

The packaging regression uses synthetic input only to test the code's
byte-preservation, branch and vector checks. It is not evidence that a
synthetic header is valid for hardware. The CLI accepts only the real
SHA-pinned reference loader.

This wrapper does not build GammaOS LK, choose \`MEMBASE\`/\`MEMSIZE\`, create
a matching native SPL/monitor firmware package, install the result, write
SD/Boot0/TOC1, or validate monitor handoff on hardware. A produced image is
not installable until a complete LK target ELF/map/raw image, reviewed
memory map, native firmware pairing, live BL31-to-LK handoff and recovery
path are separately validated.

## References

- Pinned LK \`make/build.mk\` and \`img_hdr_lk.cfg\`
- \`scripts/apply-h700-lk-overlay.py\`
- \`scripts/package-h700-bl33.py\`
- BaseOS \`experiments/h700-boot-speed/native/build.py\`
- BaseOS \`experiments/h700-boot-speed/spl/native-boot.c\`
- Reports \`reports/rg34xx-native-firmware/README.md\` and manifest
