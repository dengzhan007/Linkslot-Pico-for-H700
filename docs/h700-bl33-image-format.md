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
upper size limit. This is incorrect: the standalone packager enforces the active 32 KiB loader extent; 0xe0000 is only a separate researched link/runtime arena bound, not the package file-size allowance. It copies only reference bytes \`[+4,+0x640)\`; it never
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

## 4. Matched native SPL / firmware rebuild for a small LK diagnostic

The deployed p4+40 MiB native firmware manifest on the reports branch records
the loader extent as offset `0x3e200`, length `0x8000` (32 KiB), destination
`0x4a000000`. This is a stricter limit than the current BL33 image packager's
`0xe0000` host policy: a payload may pass the BL33 wrapper and still be too
large for the *current native firmware extent*.

The Linkslot repository now provides
`scripts/patch-baseos-native-lk-override.py`. It patches only
`experiments/h700-boot-speed/native/build.py` in a clean BaseOS-MultiFrontend
checkout pinned at `cbe253a43f46cf551cc9a6c79a84015bde3686cf`. The patch is
explicitly opt-in through `BASEOS_LK_BL33_RAW` and:

- accepts only a raw, header-free A32 LK image no larger than 32 KiB;
- verifies the first A32 branch, all eight A32 vector branches, and that
  reserved header bytes `[+4,+0x640)` are zero;
- lets the existing native builder copy the matching prepared TOC header
  bytes into that reserved window;
- pads the result to exactly 32 KiB, preserving loader extent length, all
  following firmware offsets and the native SPL extent table;
- appends the raw input to the native build input list, so the generation
  digest and `persistent_source_hashes` bind the exact LK payload;
- rebuilds `firmware.bin` and lets the native builder regenerate
  `BASEOS_FIRMWARE_BYTES`, `BASEOS_FIRMWARE_FNV`, the generated header hash
  and manifest. The paired `build-early-mmc.py` step then compiles the native
  SPL against those new constants.
- after the builder has consumed the original `loader.map` trampoline check,
  the discarded native loader's `loader.elf` and `loader.map` are renamed to
  `native-loader-template.elf/.map`; the raw replacement is separately saved
  as `lk-bl33-override.raw`. The manifest explicitly records that this ELF
  does not correspond to the final `loader.bin`. Do not use the template map
  as the LK memory map; a separate LK target ELF/map is still required.
- `--restore` writes the saved source backup as raw bytes, preserving original
  line endings, and refuses to overwrite a builder file changed after apply.

Apply the patch from the Linkslot checkout (this edits a *local* BaseOS
checkout, not the GitHub BaseOS repository):

```sh
python3 scripts/patch-baseos-native-lk-override.py \
  --baseos-root /path/to/BaseOS-MultiFrontend --apply
python3 scripts/patch-baseos-native-lk-override.py \
  --baseos-root /path/to/BaseOS-MultiFrontend --check
```

Put the raw LK image in a path inside the BaseOS checkout (it is included in
the generation/source hashes), then run from the BaseOS repository root:

```sh
mkdir -p build/rg34xx-lk-bl33-input
cp /path/to/lk-no-mtk-header.bin build/rg34xx-lk-bl33-input/lk-no-mtk-header.bin
BASEOS_LK_BL33_RAW=build/rg34xx-lk-bl33-input/lk-no-mtk-header.bin \
  python3 experiments/h700-boot-speed/native/build-release.py rg34xx \
  build/rg34xx-lk-bl33-release
```

The output directory must not already exist. The command performs a host build
only; it does not install or write Boot0/TOC1/SD. Check the resulting
`native/manifest.json` for `lk_bl33_override.loader_extent_bytes == 32768`,
`loader_extent_preserved == true`, and the expected raw payload SHA-256. The
native SPL manifest, firmware FNV, generation and capsule must all identify
the same build. If any check fails, discard the output.

The patch creates a byte-exact backup beside `native/build.py`. Restore only
when no subsequent edits were made:

```sh
python3 scripts/patch-baseos-native-lk-override.py \
  --baseos-root /path/to/BaseOS-MultiFrontend --restore
```

**Limit:** this is a route for a compact proof-of-life diagnostic that fits in
the existing 32 KiB raw-file extent. It is not yet a route for the full LK
kernel, Linkslot or mGBA.

The latest source/link regression (final code commit
`545f944936549044dcb36d57d8b86b77f812b700`, feasibility artifact
[15123653120](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280/artifacts/15123653120))
links the patched A32 CRT0/exception vectors, real H700 matcher C object and
fixed-address probe assembly with minimal stubs. Its raw flat image is
5,804 bytes, leaving 26,964 bytes in the 32 KiB native loader file extent.
The link map's in-memory `_end` is `0x4a0036d0` (offset `0x36d0`,
14,032 bytes from the image base); the BSS includes the 8 KiB Abort stack and
the probe-active word. This is a separate runtime-memory measurement against
the conservative `0x4a0e0000` link bound, not the 32 KiB file-size ceiling.

This is still a stub-linked layout test, not a diagnostic payload with a
visible result signal, a full LK kernel/platform build, or a vendor-header /
native-firmware package. The regression only establishes that this test
image's raw bytes fit and that an exactly-32-KiB input is accepted by the
override validator; it does not establish a bootable image. A larger raw LK
image requires a separately qualified loader-extent redesign and a new
SPL/firmware pairing review; do not increase the extent silently.

## 5. Scope and remaining gates

The \`scripts/package-h700-bl33.py\` regression uses synthetic input
only to test byte-preservation, branch and vector checks. It is not evidence
that a synthetic header is valid for hardware. That standalone CLI accepts
only the real SHA-pinned reference loader.

The standalone BL33 wrapper does not build GammaOS LK, choose
\`MEMBASE\`/\`MEMSIZE\`, or rebuild the native SPL. Section 4 describes the
separate opt-in BaseOS native-builder patch that can pair a small raw LK
payload with rebuilt firmware/SPL outputs. Neither route installs the result,
writes SD/Boot0/TOC1, or validates monitor handoff on hardware. Any produced
capsule or image remains non-installable until a complete LK target
ELF/map/raw image, reviewed memory map, native firmware pairing, live
BL31-to-LK handoff and recovery path are separately validated.

## References

- Pinned LK \`make/build.mk\` and \`img_hdr_lk.cfg\`
- \`scripts/apply-h700-lk-overlay.py\`
- \`scripts/package-h700-bl33.py\`
- BaseOS \`experiments/h700-boot-speed/native/build.py\`
- BaseOS \`experiments/h700-boot-speed/spl/native-boot.c\`
- Reports \`reports/rg34xx-native-firmware/README.md\` and manifest


### 32 KiB boundary test status (2026-11-26)

The overlay layout regression now rejects a raw flat image larger than the
active 32 KiB loader file extent. The BaseOS override regression separately
accepts exactly 32,768 bytes and rejects 32,769 bytes. Both passed within
full feasibility [#45834208280](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280)
(81/81; artifact [15123653120](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280/artifacts/15123653120)).
The 5,804-byte stub-linked raw file and its 14,032-byte in-memory footprint
remain different measurements. No vendor-header/firmware pair was generated
from this test ELF and no device ran it.


### Real LK early-MMU + PI12 raw layout result (2026-11-26)

The latest full feasibility [#45843270056](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45843270056) passed 81/81, zero failures and zero skipped (artifact [15131735148](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45843270056/artifacts/15131735148)). Its additional minimal source/layout ELF links the pinned LK's real `arch_early_init()`, `arm_mmu_init()`, `arm_mmu_map_section()`, cache/CP15 objects, H700 physical DRAM provider and SoC Device map, and the real RG34XX `target_early_init()`/PI12 LED service.

| Measurement | Result |
|---|---:|
| Raw flat ELF | 7,912 bytes |
| Active native loader raw extent | 32,768 bytes |
| Raw file headroom | 24,856 bytes |
| LK section table `tt` | `0x4a008000` (16 KiB aligned) |
| Linked `_end` | `0x4a00c024` |
| Runtime span from image base | `0xc024` bytes |

The raw extent and runtime footprint are distinct. This minimal layout links real MMU and PI12 code but omits `thread_init_early()`, vendor `platform_early_init()`, GIC/timer/console, heap, storage and scheduler. It is not a complete LK target or a bootable image. APB1/PIO early handoff state and runtime RAM ownership remain unproven; no vendor header/firmware/SPL package was generated, no media was written and no device executed the payload.
