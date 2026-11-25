# RG34XX / H700 stock boot partition — deep read-only audit

Scope: `build/rg34xx-inventory-20261122-214708/boot-partition.bin` (67,108,864 B,
partition `mmcblk0p4`), captured read-only on a single authorized ADB device
(ADB serial redacted). Target: confirm the stock Linux boot image layout,
kernel version/build, DTB provenance and the AArch64 hand-off facts needed for a
GammaOS Pico / LK bare-metal port. All analysis was performed on the **local
copy** of the image; no writes to the device, boot chain or SD card were made.
The full `boot-partition.bin` and all other binaries remain local-only and are
**not** uploaded to this branch.

Evidence level notation used below:
- **A** = byte-exact match against a primary artifact (local binary or cited upstream source).
- **B** = strong structural/named-symbol correlation, not a whole-file identity.
- **C** = inference from surrounding evidence; explicitly not proof.

---

## 1. Image structure

`boot-partition.bin` is **not a single boot image**. It contains two `ANDROID!`
images. Both use page size **0x800 (2048)** and the classic Android boot header
(name `sun50i_arm64`, load addresses kernel `0x40080000` / ramdisk `0x42000000`
/ second `0x40f00000` / tags `0x40000100`).

### 1.1 Primary image at partition offset 0x0 (the one observed live)

| Item | Offset (in partition) | Length (B) | Format / detail |
| --- | --- | --- | --- |
| Android header | 0x00000000 | 0x800 | `ANDROID!` v0/v1; `name=sun50i_arm64`, header[0x28]=2 |
| cmdline @0x40 | 0x00000040 | 65 | `loglevel=1 rcupdate.rcu_expedited=1 rcupdate.rcu_normal_after_boot=1` (**A** byte-exact match to BaseOS `CMDLINE`) |
| kernel | 0x00000800 | 7,018,071 (0x6B1657) | gzip (`1f 8b`), decompresses to 17,461,256 B AArch64 Image, SHA-256 `39cf8c3e…21d84b7` |
| ramdisk | 0x006B2000 | 512 (0x200) | newc cpio containing only `TRAILER!!!`, padded to 512 B (**A** byte-exact match to BaseOS `EMPTY_CPIO`) |
| second | — | 0 | header declares `second_size=0` |
| DTB (undeclared in classic fields) | 0x006B2800 | FDT `totalsize=137,770` | magic `d00dfeed`, model `sun50iw9`, 573 nodes; cropped SHA-256 `e2f3ec2a9f…` — **different** from TOC1 DTB |

Notes:
- The primary image cmdline is the BaseOS-patched one; the stock header cmdline
  was empty. `kernel_addr + kernel_size = 0x40080000 + 0x6B1657 =
  0x40731657 < 0x42000000` (ramdisk load), so the decompressed kernel does not
  collide with the ramdisk. (**A**/address arithmetic)
- The kernel is gzip-compressed by the build; the original vendor kernel in the
  second image is not a plain gzip Android Image (see 1.2).

### 1.2 Secondary image at partition offset 0x3000000

| Item | Offset | Length (B) | Format / detail |
| --- | --- | --- | --- |
| Android header | 0x03000000 | 0x800 | `ANDROID!`, same name and load addresses, empty cmdline |
| "kernel" component | 0x03000800 | 8,406,862 (0x80474E) | Raw compressed stream beginning `42 4f 4c 34` (`BOL4`), **not** gzip/`ARM\x64`; decoder/tooling not available in this environment — format left **unresolved** |
| ramdisk | 0x03805000 | 512 | same `EMPTY_CPIO` bytes as primary |
| DTB | 0x03805800 | FDT `totalsize=139,822` | cropped SHA-256 `7017a8f970…`, **byte-identical to TOC1 `dtb.bin`** (**A**) |

The rest of the 64 MiB partition is filler `0xff` apart from these regions
(e.g. 0x02800000 area contains monitor-associated strings; 0x3900000..0x3fffff
is all `0xff`). The two-image layout, empty cp io and patched cmdline match
BaseOS `kernel_gzip.py` exactly (**A** based on the cited source, section 6).

---

## 2. Kernel

Source: gzip kernel from the **primary** image, decompressed locally to
`kernel.Image` (17,461,256 B). AArch64 arm64 Image header: `text_offset=0x80000`,
`image_size=0x1138000`, `magic @0x38 = 0x644d5241` (`ARM\x64`) — **AArch64 is confirmed** (**A**).

- `Linux version 4.9.170 (flower@flower-B85M-D2V) (gcc version 5.3.1 20160412
  (Linaro GCC 5.3-2016.05)) #19 SMP PREEMPT Tue May 26 13:58:08 CST 2026` —
  matches the live `/proc/cpuinfo` banner from the reports exactly (**A**).
- Embedded IKCONFIG (`IKCFG_ST … IKCFG_ED`, gzip) was extracted successfully;
  110,173 B of `.config`. Confirmed options (**A**):
  `CONFIG_ARM64=y`, `CONFIG_IKCONFIG=y`, `CONFIG_IKCONFIG_PROC=y`,
  `CONFIG_KALLSYMS=y`, `CONFIG_ION=y`, `CONFIG_ION_SUNXI=y`,
  `CONFIG_ION_OF=y`, `CONFIG_TEE=y`, `CONFIG_OPTEE=y`, `CONFIG_CMA=y`.
  `CONFIG_DRM` is **not set**. This corrects the earlier `CONFIG_SUNXI_ION`
  grep miss (wrong symbol name) in report `…212049`.
- The build tree is a vendor Allwinner-derived 4.9.170, not a mainline release;
  `CONFIG_LOCALVERSION=""`, no `+` localversion suffix in the banner.

### 2.1 Symbol / DRM SMC path evidence

Strings in the binary (**A**):
- `sunxi_ion_probe`, `sunxi_ion_ioctl`, `sunxi_ion_client_create`,
  `sunxi_ion_probe_drm_info` (named symbols present).
- `drm config service not available: %X`, `drm_base=0x%x`, `drm_size=0x%x`,
  `tee_base=0x%x` — all four present, adjacent to `optee: initialized driver`
  and `&optee->call_queue.mutex`, i.e. in the OP-TEE driver string pool.
- Live kallsyms (report `…212049`) already showed `optee_probe_drm_configure`,
  `sunxi_ion_probe`, `ion_secure_heap_create` (addresses redacted there), so the
  compiled image and the running kernel agree on symbol names.

SMC instruction scan: only 8 `smc` opcodes found on 4-byte boundaries in the
decompressed Image (0xd678, 0xd7b4, 0xd864, 0xd8f0, 0x10150, 0x8f7888,
0x8f78ac, 0xab9010). The OP-TEE invoke function is selected at runtime from
`/firmware/optee` `method = "smc"` (**B**: count alone does not prove which SMC
is `OPTEE_SMC_GET_DRM_INFO`; see section 6 for source-level identification).

**Conclusion on the runtime path**: the successful-SMC log strings
(`drm_base=…`, `drm_size=…`, `tee_base=…`) and the failure string
(`drm config service not available`) are both compiled in, but **neither** appeared
in the captured `tee-ion-runtime.txt` dmesg extract. Therefore the captured boot
did not print either outcome. This is consistent with either (a) the call not
being executed during this boot, or (b) execution with output suppressed below
the effective log level. It is **not** evidence that secure memory is absent.
No SMC was issued by this audit.

---

## 3. Ramdisk / boot chain

- Both images' ramdisk is the 512-byte `EMPTY_CPIO` (only `TRAILER!!!`) — a
  BaseOS build artifact, byte-exact to source (**A**). There is **no init,
  no startup script and no DTB** inside the Android ramdisk.
- Live `chosen.bootargs` (from `live-fdt.json`) shows
  `root=/dev/mmcblk0p5 rootwait quiet splash init=/init` plus the unusual
  `cma=64M` and `partitions=…` list; live `initrd` range is
  `0x42000000..0x42000200` (512 B), which equals `ramdisk_addr` plus the header
  ramdisk size of this same 512-byte archive (**A**: numeric identity). The
  Linux boot therefore consumes the (empty) Android ramdisk as initrd, and the
  real root filesystem comes from `mmcblk0p5` (ext4, per BaseOS build scripts).
- The actual init lives in the root filesystem, not in `boot`. This audit did
  not image the rootfs; the loader path for `/init` and any runtime DTBO patching
  are **out of scope / unresolved**.

**DTB source conclusion**: the boot image carries its own DTB component, but the
one living boot used (live FDT) is a **modified** FDT, not byte-identical to any
of the on-disk DTBs (section 4). At least stage hand-off is:
Boot0 → TOC1 U-Boot → U-Boot selects Android boot image → kernel +
empty initrd + DTB. Whether U-Boot passed the primary-image DTB
(`e2f3ec2a…`) or the TOC1 DTB (`7017a8f9…`) before runtime edits cannot be
decided from the captured artifacts alone (**C**).

---

## 4. DTB cross-comparison

All reports below were produced with the repository's own
`tools/inspect_h700_fdt.py`. Hashes are of the FDT `totalsize` crop (not the
256 KiB read padding).

| Source | totalsize | cropped SHA-256 (prefix) | /memory 0 | initrd | disp_reserve bootarg |
| --- | --- | --- | --- | --- | --- |
| TOC1 `dtb.bin` | 139,822 | `7017a8f970234f75…` | 0x40000000–0x60000000 (512 MiB) | 0/0 | none |
| Primary boot image DTB @0x6B2800 | 137,770 | `e2f3ec2aa9ad78ce5a…` | 512 MiB | 0/0 | none |
| Secondary boot image DTB @0x3805800 | 139,822 | `7017a8f970234f75…` (**identical to TOC1**) | 512 MiB | 0/0 | none |
| Live FDT (`live-fdt.dtb` / `live-fdt.json`) | 143,936 | `e1560a7f5eba68b21…` | 0x40000000–0x80000000 (1 GiB) | 0x42000000..0x42000200 (512 B) | base 0x7bf24800, size 1,382,400 |

Common across all four: `model=sun50iw9`, `compatible=allwinner,h616 /
arm,sun50iw9p1`, 573 nodes, panel `rg34xx_v1 720x480`, three ION heaps
(`sys_user` id0, `cma` id4, `secure` id6; all base/size 0 in DTS), and
`/firmware/optee` with `method=smc`. None declares a `/reserved-memory` node.

### 4.1 Explaining 512 MiB → 1 GiB

- The three on-disk DTBs declare `/memory@40000000 = 0x20000000` (512 MiB,
  DDR from 0x40000000 to 0x5fffffff).
- Live `/proc/iomem` / memblock shows **System RAM 0x40000000–0x7fffffff
  (1 GiB)** and live FDT memory 0x40000000–0x7fffffff (**A**:
  `memory-map-report.json`, `iomem.txt`, `live-fdt.json`).
- Therefore the extra 512 MiB (0x60000000–0x7fffffff) was **not** added by
  editing the on-disk DTB text; it was injected at boot (U-Boot fixup or a
  vendor memory-probe path) before the kernel parsed the FDT. The live FDT also
  carries the added `disp_reserve` bootarg and updated initrd range, consistent
  with a U-Boot/bootloader fixup stage. The exact code performing the expansion
  was not identified in the cited source (**C**).
- Consequence for LK: sizing/rules that assume the hardware always has 1 GiB
  must not rely on the on-disk DTB's 512 MiB value, but must also not claim the
  upper 512 MiB blindly — verify actual DDR geometry from Boot0 DRAM parameters
  (boot0 report: LPDDR4 672 MHz) at hand-off.

**Important**: primary-boot DTB vs TOC1 DTB differ in bytes and in at least the
USB controller status values (`boot_vendor` has ehci1 `okay` while TOC1/secondary
have `late`). They are **not** the same file; similarity of node lists is not
treated as identity.

---

## 5. Memory region attribution evidence

All ranges below are **reserved / excluded from usable RAM** in the live memblock
snapshot (`memory-map-report.json`, derived from debugfs `memblock` on device),
not available memory.

| Range | Size | Evidence / attribution | Level |
| --- | --- | --- | --- |
| `[0x7ec00000,0x7fc00000)` | 16 MiB | memblock reserved index 6, `size:16384K`. Present in memblock but **no** `reserved-memory` DTS node and no CMA area (debugfs `/sys/kernel/debug/cma` = `NOT_PRESENT`). DRM SMC strings compiled in but no success/failure log captured. **Attribution unresolved**: consistent with a secure/DRM carveout but not proven for this boot. | B/C |
| `[0x7fd65000,0x7ff49000)` | 1,982,464 B (~1.89 MiB) | memblock reserved index 8, `size:1936K`; upper stack region near 0x7fff… (indices 9–18) likely firmware/TF-A/optee stacks, but no node names. Live on-disk FDT has no reserved-memory node. | B |
| monitor `[0x48000000,0x49000000)` | 16 MiB | memblock reserved index 4 (`size:16384K`); matches the requested monitor window exactly. The `boot-partition.bin` 0x02800000 area contains monitor-associated strings, but this audit did not disassemble monitor to confirm ownership. | B |
| CMA `[0x77c00000,0x7bc00000)` | 64 MiB | memblock reserved index 5 (`size:65536K`); live `meminfo` reports `CmaTotal` 64 MiB / `CmaFree` ~59 MiB, and bootargs `cma=64M`. ION `cma` heap id4 present in every DTB with base/size 0 (runtime-filled). **This one is well attributed** to contiguous DMA/CMA. | A/B |
| framebuffer `[0x7bf24800,0x7c076000)` | 1,382,400 B | live FDT `disp_reserve` bootarg and `/soc@03000000/disp@01000000` both give `0x7bf24800,720x480x32bpp`, stride 2,880 B, `matches_disp_reserve=true`; 720*480*4 = 1,382,400 = 0x151800, end = base+0x151800 = 0x7C076000. **A** (arithmetic verified). Note this FB starts **after** the 64 MiB CMA range ends (0x7bc00000), with a gap of 0x324800 B; it is not inside CMA and is a separate reserved/display buffer. | A |

None of these regions may be marked free RAM in an LK port without separate
proof. The `debugfs-memory-reservations.txt` snapshot explicitly distinguishes
`NOT_PRESENT: /sys/kernel/debug/cma` and `NOT_PRESENT: getconf` from absence of
reservations; memblock entries above are positive evidence of reservation.

---

## 6. Source matching

### 6.1 Exact/primary build source — BaseOS-MultiFrontend

The machine is running a locally built BaseOS image that was later pushed to
`dengzhan007/BaseOS-MultiFrontend` and built by CI, so source and flashed image
may diverge. The following byte-exact coincidences were nevertheless found
between the cited source (`tools/kernel_gzip.py`, `manifest/kernel-gzip.json`
at `main`) and this boot partition (**A**):
- `CMDLINE` bytes at header 0x40 match exactly.
- `EMPTY_CPIO` (newc trailer padded to 512 B) matches the ramdisk exactly.
- `SIZES = (8,16,24,0x660,0x670)` and header[0x66C]=1660, header[0x28]=2,
  page size 2048, load addresses 0x40080000 / 0x42000000 / 0x40f00000 /
  0x40000100 all match.
- The two-image arrangement is explained by `derive()`: gzip the vendor kernel,
  rebuild the Android image with empty ramdisk and patched cmdline, write it over
  the pinned `boot_partition_start` region, and preserve the tail of p4. The
  secondary `BOL4` image is consistent with the vendor/older image retained after
  the BaseOS patch, but its exact provenance is **not** proven here.

Divergence: `manifest/kernel-gzip.json` rg34xx profile pins
`boot_partition_sha256 = bc533339…`, while the captured partition hashes to
`41cfaa8f…`. Likewise the pinned `boot0_sha256 = b6edbf39…` vs live Boot0
`bf886995…`. This confirms the user's caveat: the running image is **not**
byte-identical to the repository's pinned catalog — treat the structure rules
as matching, the exact bytes as not.

### 6.2 Kernel source reference — qiaoweibiao/T507_Kernel

`Makefile` says `VERSION 4 / PATCHLEVEL 9 / SUBLEVEL 170` — same version family
as the live banner (**A** for version number only). The following are byte-exact
string matches between that repo and the local kernel binary (**A**):
- `drivers/tee/optee/core.c:532` `optee_probe_drm_configure()` issues
  `optee_smccc_smc(OPTEE_SMC_GET_DRM_INFO, 0...)` and on non-OK returns prints
  `drm config service not available: %X`; on OK prints `drm_base=0x%x`,
  `drm_size=0x%x`, `tee_base=0x%x` — all four strings are in the live Image.
- `drivers/staging/android/ion/sunxi/sunxi_ion.c` defines
  `sunxi_ion_probe_drm_info()` and, in `sunxi_ion_probe()`, calls
  `optee_probe_drm_configure()` **before** `ion_heap_create()` for the
  `ION_HEAP_TYPE_SECURE` entry when `CONFIG_TEE` is set, storing the returned
  base/size into the secure heap and caching phy/tee addresses for later
  `ion_secure_heap.c` phys translation. The three function names are present in
  the live Image/live kallsyms (**B**: order inferred from this reference source,
  not disassembled locally).

Therefore T507_Kernel is a **close same-series reference**, not a proven exact
build tree for this `#19` image: the vendor build banner
(`flower@flower-B85M-D2V`, Linaro 5.3.1) is not reproduced by that repo, and no
commit hash or localversion ties them. Do not claim exact source identity.

---

## 7. Open questions (blocking for a hand-off design)

1. What exactly owns `[0x7ec00000,0x7fc00000)` in this boot? The DRM/secure
   carveout hypothesis is consistent with compiled SMC strings + runtime heap
   layout but has no success log, no DTS node and no disassembled call site.
2. Which of the two on-disk DTBs did U-Boot pass to the kernel, and which code
   expanded memory 512 MiB → 1 GiB and inserted `disp_reserve`? Need U-Boot
   source/logs or a re-capture with early boot console.
3. The secondary image kernel format (`BOL4…`) is unresolved; its relationship
   to the vendor kernel and whether it is an older BaseOS payload needs the
   corresponding BaseOS release artifact to compare.
4. No extraction of a full symbol table with addresses was possible from the
   binary in this environment (kallsyms tables are compressed/inlined); only
   names were confirmed. Locating the exact SMC call instruction inside
   `optee_probe_drm_configure` in this Image remains future work (needs an
   AArch64 disassembler such as `aarch64-linux-gnu-objdump`/capstone).
5. `/init` and any runtime DTB patching live in `mmcblk0p5` (rootfs), not imaged
   here — the final FDT mutation path inside userspace is unverified.

## 8. Recommendations for LK / GammaOS Pico hand-off

- Preserve the observed hand-off loads: kernel at `0x40080000`, 512 B empty
  initrd at `0x42000000`, DTB passed as the third argument, cmdline with
  `loglevel=1 rcupdate…` (or an intentionally chosen replacement). Never let
  the decompressed kernel overlap `0x42000000`.
- Carry a DTB derived from the **live FDT** geometry (1 GiB DDR, `disp_reserve`
  matching 720x480x32 @ `0x7bf24800`), not the 512 MiB on-disk DTB, and set
  initrd/chosen explicitly. Do not assume the reserved nodes survive in a DTB
  re-generated from the on-disk source.
- Exclude from any LK-claimed free-memory pool: kernel 0x40080000..0x407316xx,
  initrd 0x42000000..0x42000200, monitor 0x48000000..0x49000000,
  CMA 0x77c00000..0x7bc00000, the separate FB 0x7bf24800..0x7c076000,
  the 16 MiB 0x7ec00000..0x7fc00000 and 0x7fd65000..0x7ff49000, and all
  memblock reserved entries around 0x7ff49…–0x7ffff…. Treat 0x7ec00000 windows
  as **unknown/reserved** until proven otherwise.
- Do not emit `OPTEE_SMC_GET_DRM_INFO` or any other SMC from LK without a
  verified interface contract; the stock path calls it from kernel space with
  `method=smc`, and no result was captured. Do not derive a payload address
  from the unproven 16 MiB region.
- If the goal is to reuse the stock kernel, keep its IKCONFIG-verified options
  (`ION_SUNXI`, `ION_OF`, `TEE`, `OPTEE`, `CMA`) and the same `/firmware/optee`
  DT node; replacing the kernel invalidates all SMC/heap conclusions above.

---

### Evidence references (local, all read-only)

- `build/rg34xx-inventory-20261122-214708/raw-sha256.txt`, `summary.txt`,
  `gpt-fdisk.txt`, `device-and-block-layout.txt`
- `build/rg34xx-inventory-20261122-214708/live-fdt.json`, `live-fdt.dtb`,
  `memory-map-report.json`, `iomem.txt`, `meminfo.txt`,
  `debugfs-memory-reservations.txt`, `tee-ion-runtime.txt`
- `build/rg34xx-inventory-20261122-214708/toc1-extracted/dtb.bin` and
  `dtb-report.json`
- `tools/inspect_h700_fdt.py` (repo tool used to generate the DTB reports)
- Reports `…212049` (ION symbols), `…210352` (DRM SMC follow-up),
  `…195053` (full archive)
- Upstream sources read-only via HTTPS:
  `dengzhan007/BaseOS-MultiFrontend` `tools/kernel_gzip.py`,
  `manifest/kernel-gzip.json`; `qiaoweibiao/T507_Kernel`
  `drivers/tee/optee/core.c`, `drivers/staging/android/ion/sunxi/sunxi_ion.c`,
  `drivers/staging/android/ion/ion_secure_heap.c`, `Makefile`.

---

## Addendum — 2026-11-24: active BOL4 payload identity resolved

The original audit above left the secondary Android image's `BOL4` payload unresolved because the decoder was not yet available. The follow-up extraction is recorded in `reports/rg34xx-active-inputs/` (manifest, 21-check verification and checksums; the raw binaries remain local-only).

The extracted active image confirms:

- BOL4 component: 8,406,862 bytes (`0x80474e`); packed raw-LZ4 payload: 8,406,846 bytes (`0x80473e`).
- Declared and decoded output: 17,461,256 bytes (`0x010a7008`); payload FNV-1a32: `0xb4c47d39`.
- The new extractor verifies that BOL4 decoding produces `kernel.Image` byte-for-byte. Its SHA-256 is `39cf8c3eb9a75ff0eda51b4074cd65860c5aa5386093bcaf69339819421d84b7`.
- This length and SHA-256 match the primary Android image's gzip-decompressed kernel recorded in section 2. The active secondary image therefore carries the same AArch64 Linux kernel payload as the primary image; the difference is the packaging/container path, not a different kernel binary.

This resolves the former open question 3 in section 7 about the BOL4 decoder and secondary-kernel identity. It does **not** prove which on-disk image a different firmware build will select, does not rebuild the paired p4+40 MiB native firmware/SPL, and does not change the existing `not_installable` / `not_hardware_validated` verdict. No device I/O or binary upload was performed for this addendum.
