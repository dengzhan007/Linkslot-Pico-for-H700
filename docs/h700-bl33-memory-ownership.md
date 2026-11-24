# RG34XX H700 BL33 and application memory ownership

**Status:** source-backed memory-ownership inventory; **not** a linker-layout approval, hardware GO, or permission to execute/flash a payload.

Target: Anbernic RG34XX non-SP, Allwinner H700 / sun50iw9p1, 1 GiB LPDDR4. This document distinguishes the active boot-chain allocations from candidate memory that might be reused by an AArch32 GammaOS LK system after it deliberately stops following the Linux boot path.

The pinned BaseOS source used for address/lifetime cross-checks is commit cbe253a43f46cf551cc9a6c79a84015bde3686cf. The live values and active-path verdict come from the read-only RG34XX reports on the reports branch. No device I/O, payload execution, or media write was performed for this audit.

## 1. Active chain and handoff

The captured device is running:

```
Boot ROM
  -> stock eGON Boot0 (DRAM training)
  -> BaseOS native AArch64 EL3 SPL
  -> vendor BL31 / monitor at 0x48000000
  -> AArch32 SVC BL33 loader at 0x4a000000 (SPSR 0x1d3)
  -> BOL4 image staged at 0x45000000
  -> AArch64 Linux only after the loader expands the kernel and issues
     the vendor Linux-start SMC
```

The final SMC tuple used by the current loader is r0=0x8000ff04, r1=0x40080000, r2=0x44000000, r3=1. This is proven for the existing Linux-start path only. A standalone LK must remain in AArch32 unless a separate, verified transition ABI is established; it must not call this SMC as a generic payload jump.

## 2. Source-backed allocations and lifetimes

All intervals below are half-open: the start is included and the end is excluded. “Capacity” is not the same as the number of bytes currently populated.

| Interval | Size / observed content | Owner and lifetime | Implication for LK |
|---|---:|---|---|
| [0x40000000, 0x80000000) | 1 GiB | Live DRAM window reported by the RG34XX kernel and FDT | Physical existence is established; this does not make the whole range allocatable. |
| [0x40080000, 0x42000000) | 31.5 MiB | Current A32 loader's Linux kernel destination window. The active decompressed Image is 0x010a7008 bytes and ends at 0x41127008; the Linux memblock reservation extends to 0x411b5000. | Strong candidate for a Linkslot application arena **only if** LK bypasses kernel decompression/Linux handoff and a separate pre-LK/secure/DMA ownership review confirms it. It is not currently free in the observed Linux boot. |
| [0x42000000, 0x42000200) | 512 B | Empty newc initrd copied by the current A32 loader; Linux memblock rounds the page reservation to [0x42000000, 0x42001000). | Do not reuse while preserving the Linux handoff. A standalone LK does not need this initrd, but reuse still requires the handoff path to be disabled. |
| [0x42e00000, 0x42e46200) | 0x46200 B | Temporary native firmware package buffer used by the SPL before it resets into BL31. | Temporary SPL input; not a proposed LK heap or payload region. |
| [0x44000000, 0x44100000) | 1 MiB capacity; live FDT blob is 143,936 B ending at 0x44023240 | Final Linux FDT buffer populated by the A32 loader. | Preserve if Linux is launched. It may be discardable in an LK-only path, but no reuse is approved here. |
| [0x45000000, 0x46000000) | 16 MiB capacity; active BOL4 image is 0x829000 B ending at 0x45829000 | Native SPL stages and verifies the active Android v2/BOL4 image here; the A32 loader reads it to unpack Linux. | It is occupied at BL33 entry. A future LK-only path could reclaim it only after explicitly discarding the BOL4 image and proving no later consumer. |
| [0x48000000, 0x49000000) | 16 MiB | Vendor BL31 / monitor window, corroborated by live memblock and the active loader provenance. | Reserved. Never use for LK, heap, stack, DMA, or Linkslot. |
| [0x4a000000, 0x4a100000) | 1 MiB nominal BL33 slot, inferred from the BL33 load base and the next firmware extent at 0x4a100000 | Native SPL loads the 32 KiB active A32 loader at 0x4a000000; BL31 enters it in AArch32 SVC. | This is the replacement BL33 location, not general-purpose RAM. Its internal runtime layout must be kept explicit. |
| [0x4a100000, 0x4a100a00) | 0xa00 B extent | Native firmware DTBO extent. | Reserved to the firmware layout. |
| [0x4a200000, 0x4a223400) | 0x23400 B extent | Native firmware TOC DTB extent. | Reserved to the firmware layout. |
| [0x4a300000, 0x4a3000f0) | 240 B | Native SPL handoff record read by the A32 loader. | Preserve until LK has consumed any required fields; do not use as scratch. |
| [0x77c00000, 0x7bc00000) | 64 MiB | High-confidence CMA candidate based on cma=64M, CmaTotal, and the matching memblock extent. | Exclude. |
| [0x7bf24800, 0x7c076000) | 1,382,400 B | Framebuffer, independently identified by live FDT boot_fb0 and disp_reserve. | Exclude, even though it is not a named memblock reservation. |
| [0x7ec00000, 0x7fc00000) | 16 MiB | Unassigned live memblock reservation; DRM/TEE association remains unproven. | Unknown means reserved. Exclude. |
| [0x7fd65000, 0x7ff49000) and top fragments | Multiple ranges | Unnamed top-of-RAM reservations in the live memblock snapshot. | Exclude; do not merge fragments into a free span. |

The 0x42e00000 package buffer is populated before the monitor reset. The 0x45000000 image buffer is still live when BL33 is entered. The 0x40080000 kernel destination, initrd and final FDT are populated by the current A32 loader on the way to Linux; they are not written by the SPL's BOL4 staging operation itself.

## 3. Internal BL33 layout: what is established

The pinned BaseOS A32 entry source performs these operations:

- Sets the initial AArch32 stack pointer to **0x4a0f0000**.
- Clears the loader BSS.
- Calls the C loader.

The loader then uses **0x4a0f8000** as a 4096-entry, 32-bit translation table: 4096 × 4 = 16,384 bytes, ending at **0x4a0fc000**. Its MMU setup points TTBR0 at that table.

The current Linkslot BL33 packager separately enforces a padded image-size ceiling of **0xe0000 (896 KiB)**. That is a conservative host-side packaging rule, not evidence that the deployed SPL has been tested with an image of that size. The arithmetic leaves 64 KiB between the current package ceiling (0x4a0e0000) and the current loader's initial SP (0x4a0f0000). It is a useful candidate separation for code and a downward-growing stack, but the source does not provide a stack guard or prove a maximum stack depth.

| BL33 subrange / point | Evidence | Limit |
|---|---|---|
| Image base 0x4a000000 | Active loader provenance and native SPL destination | Proven load base. |
| Vendor header [base+4, base+0x640) | Active loader header and the SHA-pinned wrapper template | Opaque bytes must be preserved; do not synthesize fields. |
| Vector table at base+0x1000 | Current loader's first branch targets +0x1000; H700 LK overlay places A32 vectors there | Proposed LK image layout; host link test only. |
| Package ceiling base+0xe0000 | Current host packager constant | A project policy, not a measured SPL maximum. |
| Initial SP 0x4a0f0000 | Pinned BaseOS native/entry32.S | Exact current-loader value; not yet proven as LK's required SP. |
| Page table [0x4a0f8000, 0x4a0fc000) | Pinned BaseOS native/loader.c and native/entry32.S | Exact current-loader allocation; LK may need a distinct table. |
| BL33 nominal end 0x4a100000 | Next firmware extent begins at this address | Boundary inference; no claim that every byte below it is free. |

This makes the present boundary clearer, but does **not** yet define LK's MEMBASE/MEMSIZE, BSS, heap, stack guard, page tables, DMA pool or core payload address. LK's own startup/MMU code may have different requirements from the BaseOS loader.

## 4. Consequence for Linkslot and mGBA

The namespaced AArch32 mGBA static archive is approximately 1.78 MiB, larger than the current 896 KiB BL33 package ceiling. It should not be linked into the BL33 image under the current size policy.

The viable architecture to investigate is therefore split:

1. A compact AArch32 GammaOS LK image occupies the BL33 slot and owns early boot, diagnostics and H700 services.
2. Linkslot plus its static mGBA core is built as a separate application/payload, loaded into a separately reviewed DRAM arena.
3. The most promising source-backed candidate for that application arena is the former Linux kernel destination [0x40080000, 0x42000000), because the standalone LK path would not decompress the Linux kernel there. This is a **candidate only**: pre-LK monitor/native-SPL ownership, all memory aliases, MMU attributes, cacheability, stack/heap bounds and peripheral DMA constraints must be checked before assigning it.
4. The 16 MiB BOL4 staging window at 0x45000000 is another possible reclaim candidate only after the LK path explicitly discards the staged Android image. It is not free at BL33 entry.

The archive size alone is not a complete runtime footprint. mGBA also needs writable globals, emulated memory, frontend buffers and stack/heap. Those allocations must be measured from the final linked application map, not estimated from the archive.

## 5. Blocking items before changing the memory gate

Keep H700_MEMORY_LAYOUT_APPROVED unset. Do not fill MEMBASE/MEMSIZE with guessed values and do not run a hardware payload.

Required evidence before a memory-layout approval:

- Trace every consumer and lifetime of the proposed application arena from Boot0 through native SPL, BL31 and the exact BL33 entry.
- Confirm the real BL33 image acceptance limit and whether the SPL/monitor will preserve the full 1 MiB slot or only the active loader extent.
- Define the LK code/BSS/heap/stack/MMU page-table layout and prove no self-overlap with the vendor header, vector table, native handoff record, DTBO/DTB, monitor or staging buffers.
- Prove secondary-core state and keep the pinned LK heap/scheduler single-core fail-closed until a real cross-core design exists.
- Build the separate Linkslot/mGBA application and inspect ELF program headers, link map, BSS, heap, stack, relocations and final payload length.
- Verify the H700 MMU mapping and cache attributes for the BL33 image, application arena, framebuffer and read-only SD path.
- Keep the existing computer-based SD recovery plan; do not write Boot0/TOC1 or flash an image as part of host-only validation.

## 6. Reproducible source references

- [BaseOS native entry32.S at the pinned source commit](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/cbe253a43f46cf551cc9a6c79a84015bde3686cf/experiments/h700-boot-speed/native/entry32.S)
- [BaseOS native loader.c at the pinned source commit](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/cbe253a43f46cf551cc9a6c79a84015bde3686cf/experiments/h700-boot-speed/native/loader.c)
- [BaseOS native handoff ABI at the pinned source commit](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/cbe253a43f46cf551cc9a6c79a84015bde3686cf/experiments/h700-boot-speed/spl/native-handoff.h)
- [BaseOS native SPL at the pinned source commit](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/cbe253a43f46cf551cc9a6c79a84015bde3686cf/experiments/h700-boot-speed/spl/native-boot.c)
- [RG34XX active loader provenance report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-active-loader-provenance.md)
- [RG34XX boot partition and memory audit](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-boot-partition-deep-audit.md)
- [Current H700 BL33 image packaging contract](h700-bl33-image-format.md)
