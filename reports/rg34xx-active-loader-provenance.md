# RG34XX active boot-loader provenance — read-only verification

Scope: determine which boot chain is **actually running** on the RG34XX/H700
unit represented by `build/rg34xx-inventory-20261122-214708/`, using the local
read-only capture, the two previous reports on the `reports` branch, and the
`dengzhan007/BaseOS-MultiFrontend` source at commit
`cbe253a43f46cf551cc9a6c79a84015bde3686cf`.

All operations were read-only. Nothing was written to the device, SD card,
Boot0, TOC1, GPT, U-Boot, monitor or any device node. No payload was executed,
no SMC was issued by this audit, no ION ioctl was performed, and no binary,
serial number, MAC or local path is included in this report. All raw firmware
and FDT binaries remain local-only.

Evidence levels (same convention as the preceding reports):

- **A** = byte-exact comparison against a primary artifact or a cited source expression.
- **B** = strong structural/named-symbol correlation, not whole-file identity.
- **C** = inference; explicitly not proof.

---

## 0. Headline result

The unit is **not** booting through the stock TOC1 U-Boot as its active path.
It is running **path B: BaseOS native SPL → vendor BL31 (`monitor.bin`) →
BaseOS AArch32 BL33 loader → BaseOS BOL4/LZ4 Android image → AArch64 kernel**.

The decisive evidence is not the presence of BaseOS strings in U-Boot, nor the
fact that two images share one kernel. It is the combination of:

1. three `/chosen` properties that only `native/loader.c` writes (**A**);
2. a `bootargs` string byte-identical to the template produced by
   `native/build.py` from the on-device `setargs_mmc` environment (**A**);
3. `/dram` values byte-identical to the trained 32-word Boot0 block, and a
   1 GiB `/memory` node produced exactly as `loader.c` writes it (**A**);
4. a 40 MiB-offset firmware package containing an AArch32 BL33 whose single
   `SMC` instruction is the `0x8000ff04` kernel-entry service (**A**);
5. a 48 MiB-offset Android image in **BaseOS's own BOL4 frame format**, which
   is the only image the native SPL is coded to read and verify (**A**).

The stock chain (path A) is **still present as a compatibility/fallback path**
on the media and in TOC1, but no evidence shows it executed in the captured
boots. The previous reports' attribution of the 1 GiB fixup to U-Boot, and the
characterisation of Option B as a future target, are therefore corrected in
section 6.

---

## 1. Current active boot flow

```
Boot0 (stock, on eMMC boot0 area; DRAM training)
  |
  | loads/enters native mainline SPL at EL3 in AArch64
  v
BaseOS native SPL  [AArch64 EL3, from SRAM; source spl/native-boot.c]
  |  reads p4 + 40 MiB firmware package into 0x42e00000
  |  verifies FNV-1a, copies extents:
  |      monitor       -> 0x48000000  (vendor BL31, 16 MiB window)
  |      dtbo          -> 0x4a100000
  |      dtb           -> 0x4a200000
  |      A32 loader    -> 0x4a000000  (BL33 slot, 1 MiB arena)
  |  reads p4 + 48 MiB Android image into 0x45000000, verifies FNV-1a
  |  patches BL33 header with trained DRAM parameters / board bytes
  |  publishes 240-byte native record at 0x4a300000
  |  cleans caches, disables MMU, programs RVBAR=0x48000000, RMR_EL3 reset
  v
Vendor BL31 / monitor  [AArch64 EL3 at 0x48000000, unmodified binary]
  |  default next image: AArch32 SVC BL33 at 0x4a000000, SPSR=0x1d3
  v
BaseOS A32 loader  [AArch32/SVC at 0x4a000000; source native/loader.c]
  |  validates record and Android v2 header
  |  LZ4-decodes BOL4 kernel directly to 0x40080000 (up to 0x01f80000)
  |  copies 512-byte empty initrd to 0x42000000
  |  opens TOC1 DTB at 0x44000000, writes 24 /dram words, /memory reg,
  |  bootargs, initrd, /chosen/baseos,* and display properties
  |  dsb/isb; cache clean + SCTLR.M/C/I off; TLB/I-cache invalidate
  |  r0=0x8000ff04, r1=0x40080000, r2=0x44000000, r3=1; SMC #0
  v
AArch64 Linux kernel  [EL1h, SPSR=0x3c5, x0=DTB, entry 0x40080000]
```

The AArch32 loader replaces the **vendor U-Boot image in RAM** at `0x4a000000`;
it keeps a U-Boot-derived header region so the vendor BL31 image parser still
accepts it. This is why a U-Boot-looking header/BL33 slot exists without stock
U-Boot code executing. The static U-Boot strings about `setargs_nand`, `bootcmd`
and `sunxi_flash` are **not execution evidence**.

---

## 2. p4 layout and component mapping

`boot-partition.bin` is the 64 MiB `mmcblk0p4` image, SHA-256
`41cfaa8f…325516`. Three regions matter; everything else is `0xff` filler.

| Region | File offset | Derived length | SHA-256 (prefix) | Role |
| --- | --- | --- | --- | --- |
| Primary gzip image | `0x00000000` | `0x6d4800` (7,161,856) | archive not hashed separately in this report; kernel gzip `1f8b…` | stock-U-Boot compatibility / fallback image |
| BaseOS firmware package | `0x02800000` (+40 MiB) | `0x46200` (287,232) | `dc9dc3ae…126f28e9` | native SPL input |
| BaseOS BOL4 image | `0x03000000` (+48 MiB) | `0x829000` (8,556,544) | `d71e5bd8…5f76fc3` | **active** kernel/ramdisk/DTB image |

### 2.1 Firmware package at p4 + 40 MiB

The package is a plain concatenation produced by `native/build.py`; the SPL
reads `BASEOS_FIRMWARE_SECTOR = boot_partition_lba + 40*2048` and checks
FNV-1a over `BASEOS_FIRMWARE_BYTES`.

Computed package fingerprint: **0x46200 bytes, SHA-256
`dc9dc3ae66b30244911e7054ca66136991626cabb0d59c796c4b253a126f28e9`,
FNV-1a `0xaf0018cb`**.

| Extent | Offset in package | Bytes (incl. padding) | Destination | SHA-256 comparison |
| --- | --- | --- | --- | --- |
| monitor | `0x0` | `0x1a400` (0x1a2d0 payload + zeros) | `0x48000000` | payload **byte-identical** to TOC1 `monitor.bin` (`676bc020…696c31`) (**A**) |
| dtbo | `0x1a400` | `0xa00` (0x920 payload + zeros) | `0x4a100000` | payload **byte-identical** to TOC1 `dtbo.bin` (`4bee2817…e73ec2`) (**A**) |
| dtb | `0x1ae00` | `0x23400` | `0x4a200000` | **byte-identical** to TOC1 `dtb.bin` (`c28c8738…89603a1`) (**A**) |
| A32 loader | `0x3e200` | `0x8000` | `0x4a000000` | `48b2a837…3412dd8590`; no TOC1 counterpart (replaces U-Boot) |

The monitor carries the Allwinner `CHKv1.0` header (`eGON`-style branch word
`0x1400003e`, names `monitor`, ASCII checksum tag). Its checksum tag was not
rederived; identity with the TOC1 item is established by whole-file SHA-256.

The loader is AArch32: first word `0xea0003fe` is `b 0x4a001000`, followed by
the retained `uboot` magic. A single `SMC #0` sits at loader offset `0x1144`;
the disassembly immediately before it is:

```
mcr p15,0,r0,c1,c0,0   ; SCTLR write-back
isb sy
mov r0,#0
mcr p15,0,r0,c8,c7,#0  ; TLB invalidate
mcr p15,0,r0,c7,c5,#0  ; I-cache invalidate
dsb sy; isb sy
movw r0,#0xff04
movt r0,#0x8000        ; r0 = 0x8000ff04
ldr  r1,[pc,#0x20]     ; literal 0x40080000
mov  r2,#0x44000000
mov  r3,#1
smc #0
wfi                    ; never expected to return
```

This is the exact `native/entry32.S` kernel-entry contract from BaseOS
source (**A**: instruction bytes match the source constants and literals).
It is a vendor Linux-start SMC, not a general bare-metal jump API.

### 2.2 Image at p4 + 48 MiB

Android v0/v1 header: page 2048, name `sun50i_arm64`, load addresses kernel
`0x40080000` / ramdisk `0x42000000` / second `0x40f00000` / tags `0x40000100`.
Header cmdline at `0x40` is byte-identical to the primary image's cmdline.

Derived extents: kernel `0x800..0x80474e` (8,406,862 B), ramdisk
`0x805000..0x805200` (512 B), DTB `0x805800..0x805a400` (144,384 B), total
`0x829000` bytes. Fingerprints: **SHA-256 `d71e5bd8…5f76fc3`, FNV-1a
`0x04762a5f`**.

The kernel stream is not a vendor-compressed kernel and not gzip: it is
BaseOS's own `frame()` format from `fastboot/build.py` —
`struct.pack("<4I", 0x344c4f42, packed_len, unpacked_len, FNV(packed))`
followed by a raw LZ4 block:

| Header field | Value | Verification |
| --- | --- | --- |
| magic | `BOL4` (`0x344c4f42`) | matches `frame()` constant (**A**) |
| packed length | `0x80473e` (8,406,846) | equals blob length − 16 (**A**) |
| unpacked length | `0x010a7008` (17,461,256) | equals AArch64 `Image` size (**A**) |
| FNV-1a of packed block | `0xb4c47d39` | recomputed from the block, exact (**A**) |

Decoding the LZ4 block yields a kernel Image byte-identical (SHA-256
`39cf8c3e…21d84b7`) to the decompressed gzip kernel of the primary image. The
ramdisk is the 512-byte empty cpio; the DTB is byte-identical to TOC1
`dtb.bin`. This image matches the input contract of `loader.c`: Android v2
header checks, bounded LZ4 decode, no external initramfs.

### 2.3 Primary image at offset 0 — fallback, not the active payload

The primary image is also a BaseOS build artifact (patched cmdline + 512-byte
empty cpio + gzip kernel), but it is the image the **stock** `boot_normal`
command reads from the beginning of `boot` into `0x45000000`. Its DTB differs
from TOC1 (`e2f3ec2a…` vs `7017a8f9…`, 137,770 vs 139,822 bytes) and its
kernel is gzip, which the native loader does not decode.

The two images deliberately carry the **same kernel bytes** after
decompression, but that alone does not establish their role. The role is
established by the SPL source (`BASEOS_IMAGE_SECTOR = p4 + 48 MiB`) and by the
live FDT/bootargs proving the A32 loader ran. Therefore:

- **active payload**: p4 + 48 MiB BOL4 image;
- **fallback / stock-U-Boot payload**: p4 + 0 gzip image;
- not resolved: whether the fallback image has ever been used on this unit.

---

## 3. Live FDT provenance and generation

### 3.1 Exact FDT correspondence

`live-fdt.dtb` has 573 nodes, the same node count as TOC1 `dtb.bin`, with only
**28 changed properties**: 17 `/dram` parameters, `/memory@40000000/reg`,
six `/chosen` properties (`bootargs`, both initrd cells and the three
`baseos,*` markers), and four display properties under
`/soc@03000000/disp@01000000`. That is precisely the edit set implemented in
`native/loader.c` lines 295–321. No other structural divergence exists (**A**).

`/chosen` marker values:

- `baseos,native-boot`: 32 bytes, SHA-256 `516e5da8…59674`, generation
  `022f0d8816c861ebbec05f0d0d7f513d6b974d9930661d20fa8ad51b7b4c0fee`.
- `baseos,native-timing`: 56 bytes = 7 big-endian counter ticks:
  `6,865,871 / 7,818,870 / 19,212,664 / 20,536,156 / 23,215,260 /
  31,621,794 / 31,621,794`. They are monotonic; `times[5] == times[6]`, as
  `loader.c` writes both at the same point.
- `baseos,boot-display`: 80-byte `struct baseos_native_display`, magic
  `DSP1`, version 1, `bytes=80`, FNV-1a `0x68d633f7` recomputed with the
  checksum field zeroed — exact (**A**). Its embedded 32-byte generation is
  **byte-identical** to `baseos,native-boot` (**A**). `shown_ticks =
  31,503,114`, which is after the SPL `loaded_ticks` (`times[2]`) and before
  the loader timing stamps.

### 3.2 DRAM, RAM size and memory edits

All 24 `/dram` properties written by the loader match the 32-word trained
block at Boot0 offset `0x38` word-for-word (**A**). Word 7
(`dram_mr1 = 0x04000000`) gives
`ram_bytes = (0x04000000 >> 16) << 20 = 0x40000000` = **1 GiB**, and
`/memory@40000000/reg` is `0x40000000..0x80000000`, exactly as
`loader.c` constructs it. The 512 MiB values in all three on-disk DTBs are the
build-time defaults; the live value comes from the SPL/loader path, not from a
stock U-Boot `fdt_fixup_memory_banks` call.

`disp_reserve=1382400,0x7bf24800` equals
`0x80000000 - 68,007,936`; that offset is the pinned
`vendor-handoff.json` `framebuffer_from_ram_end`, another exact match to the
build policy (**A**). Initrd is `0x42000000..0x42000200`, and the FDT memory
reservation map contains exactly two entries: monitor `0x48000000..0x49000000`
(16 MiB) and initrd `0x42000000..0x42000200`.

### 3.3 Bootargs and generation match status

The live bootargs (serial redacted here) is byte-identical to the template
that `native/build.py` generates from the device's own `setargs_mmc`
environment plus its fixed suffix, after substituting `@SERIAL@`,
`@RESERVE@`, `@BOOTREASON@`, `@PANEL@` and `@ANDROID@` (**A**). The template is
619 bytes long and belongs to the locally built variant.

The `partitions=` list embedded in that template names `appfs@mmcblk0p6` and
`UDISK@mmcblk0p7`, whereas the current device GPT has `UDISK` on p6 and
`primary` on p7. This is a build-time-inventory fingerprint: the bootargs were
baked from the partition table used when the image was built, not from the
current GPT. It also explains why prior "U-Boot changed bootcmd at runtime"
speculation is unnecessary.

**Generation exact-match attempt — result: no published build artifact
matches.** The 32-byte generation above was searched against every committed
BaseOS evidence manifest (no hit), and an attempted reconstruction following
`build.py` line 316 was run with all available combinations of local/pinned
package hash, DTB hash, panel contract and both captured Boot0 copies (16
attempts). None reproduced the live generation. The missing inputs are the
**build-time** inventory prefix and Boot0 bytes: the current device's TOC1
package (`e6c20e3a…`) and Boot0 (`bf886995…`) differ from the repository's
pinned `rg34xx` contracts (`56371d98…` / `b6edbf39…`). This is consistent with
the stated workflow — locally built and flashed first, then source pushed —
and with the loader header not matching the current TOC1 U-Boot bytes. The
honest conclusion is:

- **A** the live generation is internally consistent and was produced by the
  documented `build.py` algorithm shape;
- **C** it belongs to a locally built, unpublished variant; an exact
  source-to-generation reproduction is **not available** from the committed
  repository alone.

No raw FDT, bootargs with identifiers, or generation-bearing binary is
uploaded by this report beyond the digest itself.

---

## 4. Three candidate paths and the verdict

| Path | Evidence present | Executed this boot? |
| --- | --- | --- |
| **A** Stock Boot0 + TOC1 U-Boot + monitor + gzip image | Boot0, TOC1 package and gzip image all exist on the device; U-Boot contains `bootcmd`, `setargs_nand/mmc`, `fdt_fixup_memory_banks` strings; `boot_normal` reads p4 offset 0. All **static** existence only. | **No evidence.** No `baseos,*` marker can be produced by that path; the active image is at p4 + 48 MiB, which stock U-Boot does not read. |
| **B** BaseOS native SPL + vendor BL31 + A32 loader + BOL4 image | p4+40 package with four extents; p4+48 BOL4 image; exact bootargs template match; 28-property loader edit set; 32-byte generation reproduced inside two independent FDT properties; loader `SMC #0` with the documented registers; timing/DRAM/display records internally valid. | **Yes — this is the active path** (A-level evidence). |
| **C** Other hybrid/fallback | TOC1 U-Boot remains in the package; p4+0 gzip image is a fallback; device GPT differs from build-time inventory. These are *parts of the supported design*, not evidence of a third active path. | No evidence for a distinct third path. |

Verdict: **B is running now**; A is an installed fallback; C is not supported
by evidence. Note that B itself is a hybrid by design: stock Boot0 and stock
BL31 are retained, while SPL and BL33 are replaced in RAM.

---

## 5. Handoff ABI across the running chain

| Transition | Evidence | Contract |
| --- | --- | --- |
| Boot0 → native SPL | BaseOS `spl/native-boot.c`, RMR entry (`CurrentEL == EL3`, record word [46] = 16) | SPL runs in AArch64 EL3 from SRAM; DRAM trained by native path; no vendor U-Boot needed |
| SPL → BL31 | `native-boot.c` copies extents, patches monitor `+0x30` (32 DRAM words), `+0x24` (8 board bytes), `+0xb0 = 0x4a200000`, then `RVBAR=0x48000000`, `RMR_EL3` with x0=0x48000000, x1=RVBAR+4, x2=3 | Vendor monitor is loaded at 0x48000000; reset, not a branch |
| BL31 → A32 loader | `HANDOFF.md` and `spl/verify-monitor.py` emulation: first ERET `entry=0x4a000000`, `spsr=0x1d3` (AArch32 SVC) | BL33 slot contents are the BaseOS loader, not the TOC1 U-Boot payload |
| A32 loader → A64 kernel | Loader bytes at `0x4a001130..0x4a001148` (disassembled above); BaseOS `DESIGN.md` | `r0=0x8000ff04`, `r1=0x40080000`, `r2=0x44000000`, `r3=1`, `smc #0`; kernel resumes at `0x40080000`, SPSR `0x3c5`, x0=DTB |

The `0x8000ff04` service is the **vendor Linux-start SMC**. Source-level
emulation proves the return state for this exact usage; it does **not** prove
a general-purpose payload jump interface. Do not retarget `r1` to an
unverified address and do not call this SMC from LK or bare metal without a
separate contract proof.

---

## 6. Corrections to previous reports

| Prior statement | Correction |
| --- | --- |
| `rg34xx-boot-partition-deep-audit.md` §1.3: "at least Boot0 → TOC1 U-Boot → U-Boot selects Android boot image" with the DTB choice left unresolved. | **Superseded.** The active chain is section 1 above. TOC1 U-Boot was replaced in RAM by the A32 loader; the DTB source is TOC1 `dtb.bin` opened and edited by `loader.c`. |
| §3 / §4: the 512 MiB → 1 GiB change was attributed to a U-Boot fixup path (`fdt_fixup_memory_banks`/`update_fdt_dram_para`). | **Superseded.** The live `/memory` reg is written by `loader.c` from the SPL-recorded `ram_bytes` derived from Boot0 DRAM word 7. U-Boot contains similarly named strings, but no execution evidence supports attributing this boot's change to them. |
| "Runtime changed `bootcmd` from NAND to MMC" explanation of `root=/dev/mmcblk0p5`. | **Superseded.** The native loader never evaluates `bootcmd`; it constructs `bootargs` directly from the build-time-expanded `setargs_mmc` template. The `bootcmd=run setargs_nand boot_normal` value is static environment content and was not executed in the observed boots. |
| §7: secondary image BOL4 was "unresolved format", "possibly older/vendor retained". | **Corrected.** BOL4 is BaseOS's own 16-byte frame + raw LZ4 block; header fields and FNV were recomputed exactly. It is the **active** image, not an unexplained leftover. The gzip image at offset 0 is the compatibility/fallback image. |
| Previous follow-up report §8: Option B described as a future goal. | **Corrected.** Option B is **already deployed and running**. Forward work is hardening/reproducibility, not first-time adoption. |
| "Missing DRM success/failure logs are consistent with suppressed logging." | **Refined.** Live bootargs contain **two** `loglevel=` values: `loglevel=4` from the env template and a trailing `loglevel=1` appended from the Android image header (`@ANDROID@`). The kernel uses the last one; `/proc/sys/kernel/printk` reads `1 7 1 7`. Absence of DRM logs therefore says nothing about whether the SMC ran. |

Static existence, source behaviour, image matching and live execution are kept
distinct throughout: a string in U-Boot or a function in a source file is never
treated as proof that it ran on this unit.

---

## 7. Remaining unknowns

1. No exact reproduction of the 32-byte generation from committed source and
   available inputs; the build-time Boot0/prefix bytes are not published.
2. The build-time U-Boot bytes used for the loader header differ from the
   current TOC1 U-Boot; the exact source package is unavailable.
3. Whether the stock/fallback path has ever been exercised on this unit.
4. Whether the vendor monitor validates the kernel-entry SMC's `r1`/`r2` for
   anything other than the expected kernel/DTB addresses.
5. DRM SMC success or failure during the captured boots (console level 1).
6. Ownership of the 16 MiB `0x7ec00000..0x7fc00000` memblock reservation.
7. Whether the GPT difference (build-time `appfs`/`UDISK` vs current
   `UDISK`/`primary`) is a post-install repartition or a different SKU image.

---

## 8. Recommendations for the next Linkslot Pico stage

1. **Treat path B as the baseline.** Any LK design must either replace the A32
   loader with an equivalent implementation of the full FDT edit set and the
   vendor SMC contract, or boot beneath it. Designing as if TOC1 U-Boot were
   active will produce wrong memory and bootargs assumptions.
2. **Preserve the fixed addresses and reservations**: kernel `0x40080000`,
   initrd `0x42000000`, DTB `0x44000000`, image staging `0x45000000`, monitor
   `0x48000000` (never assume `0x49000000` is free), DTBO `0x4a100000`,
   TOC DTB `0x4a200000`, BL33 `0x4a000000`, records `0x4a300000`.
3. **Make generation reproducible.** Publish the build-time inventory
   (Boot0, GPT prefix, package), the exact manifest containing `generation`,
   `image_fnv`, `firmware_fnv` and source hashes, and a verifier that compares
   them with `/chosen/baseos,native-boot`. This is the missing audit link.
4. **Keep the fallback explicit.** The gzip image at p4+0 and TOC1 U-Boot can
   remain a recovery path, but a loader must detect the `baseos,*` markers
   before claiming which path ran; identical kernel hashes are insufficient.
5. **For debugging, raise the *last* `loglevel=`** in the image header (or use
   a separate diagnostic build). The env's `loglevel=4` is overridden by the
   appended `loglevel=1` from the Android header.
6. **Do not allocate unattributed regions** (`0x7ec00000..0x7fc00000`,
   monitor window, stacks near `0x7fff…`) without separate proof.
7. **Resolve the GPT divergence** before touching `rootfs`/`appfs` layout:
   current `mmcblk0p6` is `UDISK`, but the baked bootargs still names
   `appfs@mmcblk0p6`.

---

### Evidence index (local, read-only)

- `build/rg34xx-inventory-20261122-214708/boot-partition.bin` (local-only)
- `build/rg34xx-inventory-20261122-214708/toc1-extracted/`
- `build/rg34xx-inventory-20261122-214708/live-fdt.dtb`
- `build/rg34xx-inventory-20261122-214708/env-report.json`
- `build/rg34xx-inventory-20261122-214708/gpt-fdisk.txt`
- BaseOS-MultiFrontend commit `cbe253a4`:
  `experiments/h700-boot-speed/spl/native-boot.c`,
  `experiments/h700-boot-speed/native/loader.c`,
  `experiments/h700-boot-speed/native/build.py`,
  `experiments/h700-boot-speed/native/entry32.S`,
  `experiments/h700-boot-speed/fastboot/build.py`,
  `experiments/h700-boot-speed/DESIGN.md`,
  `experiments/h700-boot-speed/HANDOFF.md`,
  `experiments/h700-boot-speed/spl/verify-monitor.py`
- Previous reports: `reports/rg34xx-boot-partition-deep-audit.md`,
  `reports/rg34xx-bootloader-handoff-followup.md`

---

## Addendum — 2026-11-24: p4+40 / p4+48 image-pair identity

The local p4+40 MiB extraction is recorded in `reports/rg34xx-native-firmware/`.
The 287,232-byte package at p4 offset `0x02800000` matches the audited
SHA-256 `dc9dc3ae66b30244911e7054ca66136991626cabb0d59c796c4b253a126f28e9`
and FNV-1a32 `0xaf0018cb`. Its four extents are contiguous and bounded:
monitor, DTBO, DTB and the 32 KiB AArch32 loader.

The p4+40 MiB firmware DTB and the p4+48 MiB active Android v2 image DTB are
both 139,822 bytes and byte-identical (SHA-256
`c28c87380ffab00148bf4dc457e70b8c6c576470dcfbc53df64addddf89603a1`).

### Loader image identity checks

The A32 loader disassembly in the extraction report finds both expected
compile-time image constants in one validation function:

- Image length `0x00829000`: `movw r1, #0x9000` at loader offset
  `0x1880`, `movt r1, #0x82` at `0x1888`, compared at `0x1890`.
- Image FNV-1a32 `0x04762a5f`: `movw r1, #0x2a5f` at `0x189c`,
  `movt r1, #0x476` at `0x18a0`, compared at `0x18a4`.

The report's byte-string literal search returned empty arrays because these
A32 immediates are assembled by `movw` + `movt`; that is a scanner
limitation, not evidence that the constants are absent. The instruction
disassembly is the evidence for the two constants and comparisons.

This also maps directly to the native record ABI in
`experiments/h700-boot-speed/spl/native-handoff.h` at BaseOS-MultiFrontend
commit `cbe253a4`: record base `0x4a300000`; `image_bytes` offset
`0xbc`; `image_fnv` offset `0xc0`. With loader register `r7 =
0x4a300028`, its loads `[r7, #0x94]` and `[r7, #0x98]` address exactly
`0x4a3000bc` and `0x4a3000c0`. Those are the fields populated by
`spl/native-boot.c` from `BASEOS_IMAGE_BYTES` and `BASEOS_IMAGE_FNV`.
The source build derives the constants from the paired Android image;
the SPL source also checks the image FNV after reading the p4+48 MiB extent.

**Conclusion:** the deployed p4+40 MiB loader constants agree with the
deployed p4+48 MiB active image's length and FNV, and their DTB components
are byte-identical. This closes the *deployed loader ↔ active image
identity* check at the available host-evidence level.

### What this does not close

It does not recreate or validate the exact 32-byte build generation, prove
the image-validation function's runtime call path from disassembly alone,
or independently inspect the deployed raw-capsule SPL's compiled constants.
It is not a firmware rebuild or hardware boot test.

Terminology matters here: BaseOS `native/build.py` reads
`work/<target>/boot-gzip/boot-package.bin` as the **TOC1 source package**
used to derive the policy and loader header. That input is not the
287,232-byte p4+40 MiB native firmware output extracted in this round.
The pinned TOC1 source package and build-time `inventory/prefix.bin` remain
unavailable; the live TOC1 package hash differs from the pinned source hash.
Those missing build inputs still block exact generation reproduction.
