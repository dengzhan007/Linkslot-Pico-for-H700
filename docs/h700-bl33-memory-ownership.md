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

### 2026-11-25 16:30 +08:00 ION/TEE read-only follow-up

The latest capture at [`reports/rg34xx-inventory-20261125-163152/tee-ion-runtime.txt`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261125-163152/tee-ion-runtime.txt) records:

- `CONFIG_CMA=y`, `CONFIG_DMA_CMA=y`, `CONFIG_OF_RESERVED_MEM=y`, `CONFIG_ION=y`, `CONFIG_TEE=y`, and `CONFIG_OPTEE=y`.
- `/dev/ion` exists; `/dev/tee0` and `/dev/teepriv0` are absent. The kernel log reports `optee: api uid mismatch`; this describes the standard OP-TEE probe and does not establish whether a separate vendor DRM SMC service exists.
- ION debugfs exposes `sys_user`, `cma`, and `secure` heaps. The sampled `secure` heap has zero current allocations; `sys_user` reports 7,397,376 bytes of current allocations (including a 7,372,800-byte `ion_disp2` client). These are runtime allocation snapshots, not address ownership maps.
- The CMA heap reports `[0x77c00000,0x7bc00000)`, total 64 MiB and approximately 62 MiB free. The FDT ION secure heap base/size fields remain zero.

This narrows the runtime evidence but does **not** identify the separate `[0x7ec00000,0x7fc00000)` 16 MiB memblock reservation. Do not infer that it is or is not a secure/DRM carveout from the OP-TEE UID mismatch, empty ION secure heap, or zero FDT heap base/size. No vendor SMC, ioctl, secure monitor call, or memory read was issued. The range remains unknown and excluded from every LK allocation candidate; `H700_MEMORY_LAYOUT_APPROVED` remains unset.

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


### Active native SPL and later A32-loader memory accesses

A direct source trace at the pinned BaseOS-MultiFrontend commit
cbe253a43f46cf551cc9a6c79a84015bde3686cf narrows the pre-BL31 ownership
question. In spl/native-boot.c, the native SPL reads the p4 firmware package
into 0x42e00000, copies its four firmware extents to the monitor/DTBO/DTB/BL33
destinations listed above, reads the BOL4 image into 0x45000000, patches the
BL33 and monitor headers, and publishes the record at 0x4a300000. It then
cleans caches, programs RVBAR to 0x48000000 and requests the EL3 reset. The
pinned native-handoff.h sets the BOL4 staging capacity to 0x01000000 (16 MiB)
at 0x45000000, the final FDT capacity to 0x00100000 at 0x44000000, and the
native handoff record to 0x4a300000. No explicit read/copy/clear target in
the audited native-boot.c handoff path is the candidate interval
[0x40080000,0x42000000).

The subsequent A32 loader is a separate stage. Its entry32.S sets SP to
0x4a0f0000 and uses [0x4a0f8000,0x4a0fc000) for its 4096-entry translation
table. native/loader.c later decodes the BOL4 kernel into
[0x40080000,0x42000000) with a maximum output length of 0x01f80000, copies
the initrd at 0x42000000, and opens the final FDT at 0x44000000. Therefore the
candidate application region is demonstrably used by the existing Linux path
after BL33 entry; a replacement LK that enters directly at BL33 would need to
bypass the A32 Linux loader and never execute this decode/initrd path.

This source trace is only a partial lifetime proof. It shows no direct native
SPL destination in the candidate interval in the pinned source, and identifies
the later Linux-loader writes, but it does not show every possible access by
the opaque vendor BL31 binary or its secure-world services. The monitor's
runtime scratch allocations, cache aliases, DMA access, and any hidden
reservation below 0x48000000 have not been proven absent. The live Linux
memblock map is not a substitute for that pre-Linux BL31 audit. Accordingly,
[0x40080000,0x42000000) remains a candidate only; it is not approved for LK,
Linkslot, mGBA, heap, stack or DMA.

Source references:
- BaseOS native SPL source at the pinned commit:
  https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/cbe253a43f46cf551cc9a6c79a84015bde3686cf/experiments/h700-boot-speed/spl/native-boot.c
- BaseOS A32 entry source:
  https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/cbe253a43f46cf551cc9a6c79a84015bde3686cf/experiments/h700-boot-speed/native/entry32.S
- BaseOS A32 loader source:
  https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/cbe253a43f46cf551cc9a6c79a84015bde3686cf/experiments/h700-boot-speed/native/loader.c
- Native handoff address constants:
  https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/cbe253a43f46cf551cc9a6c79a84015bde3686cf/experiments/h700-boot-speed/spl/native-handoff.h


### Host-only A32 BL33 prototype layout (2026-11-26)

The FIQ-safe entry-snapshot change was linked with the pinned GammaOS LK CRT0
and original exception assembly plus minimal host stubs. The probe reported
ELF32/ARM, _start=0x4a000000, vector table 0x4a001000, wrapper 0x4a001024,
boot-argument scratch 0x4a001020, five-word entry snapshot BSS at
0x4a003428..0x4a00343c, _end=0x4a00343c, and 5,148 flat-image bytes. All eight
vector branch targets resolved. Full feasibility #45779682226 passed 76/76;
see [workflow artifact 15098435220](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45779682226/artifacts/15098435220).

This only demonstrates that the prototype sections and snapshot object fit
in the host linker's researched image bound. It does not establish deployed
SPL acceptance, a valid stack/heap/MMU arrangement, free ownership of any
physical RAM, or the correctness of the runtime firmware handoff. The probe
bound 0x4a000000..0x4a0e0000 remains a packaging/source-analysis bound, not an
allocation grant; H700_MEMORY_LAYOUT_APPROVED remains unset.

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


### 2026-11-26 Matched BL31 binary: control-flow-aware memory-reference triage

The exact monitor payload extracted from the 2026-11-26 reports branch was
verified against both the TOC1 inventory and native firmware extent manifest:

- Payload length: **107,216 bytes**.
- SHA-256: `676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31`.
- Load base: `0x48000000`; first-entry branch target: `0x480000f8`.
- Payload end (exclusive): `0x4801a2d0`.

The Capstone control-flow traversal decoded 1,849 reachable instructions and
recorded 268 recoverable address references: 188 inside the payload file range
and 80 in the immediately following runtime/page-table reference range
`[0x4801a2d0,0x48200000)`. It follows direct branch/call paths only; it is
not a complete call graph.

### Recovered EL3 state

- At `0x4800bf60`, BL31 writes `TTBR0_EL3 = 0x4803cb40`.
- At `0x4800bf4c`, BL31 loads the TCR_EL3 configuration from
  `0x4803c8e0`.
- At `0x4801582c`, per-core TPIDR_EL3 context addressing uses base
  `0x4803cd40`, with an MPIDR-derived index shifted by seven bits.

### Translation-map requests decoded from the caller

The binary passes the following parameters to helper `0x4800bcb4`:

| Callsite | Base argument(s) | Length | Raw attr | Condition |
|---|---|---:|---:|---|
| `0x480033a0` | `0x48000000 / 0x48000000` | `0x41000` | `3` | unconditional |
| `0x480033b4` | `0x48000000 / 0x48000000` | `0x1a000` | `1` | unconditional |
| `0x480033c8` | `0x48040000 / 0x48040000` | `0x1000` | `2` | unconditional |
| `0x480033f4` | `0x48200000 / 0x48200000` | `0x1e00000` | `3` | skipped: matched `+0x18 = 0` |
| `0x48003408` | `0x48200000 / 0x48200000` | `0x400000` | `7` | skipped with same zero flag |

Thus the code unconditionally configures mapping requests inside
`[0x48000000,0x48041000)`. The exact matched monitor payload has the
little-endian word `0x00000000` at file offset `+0x18` (runtime
`0x48000018`). The pinned native SPL source copies the extent and only
patches monitor offsets `+0x24`, `+0x30` and `+0xb0`; no write to
`+0x18` is present in that handoff source. Therefore the CBZ at
`0x480033e0` skips both conditional calls for this exact monitor/SPL pair,
subject to the limitation that earlier BootROM/Boot0 stages have not been
fully traced for an indirect mutation.

The conditional `[0x48200000,0x4a000000)` identity-map request is inactive
for the matched pair and should not be counted as an active mapping solely
because its callsite exists. Raw attributes are preserved as evidence but
have not been interpreted as memory type, shareability or security
permissions.

The page-table construction path references pool base `0x4803e000`,
4 KiB page indexing and cursor/counter `0x4803cb80`. Its local counter guard
permits no more than two page indices along this particular path; this is not
a complete allocation map or proof of the overall table-pool size.

### Memory-ownership conclusion

The 107,216-byte monitor file length is **not** the BL31 runtime footprint.
For the exact matched monitor/SPL pair, retain the unconditional
BL31-associated mapping range `[0x48000000,0x48041000)` as excluded from
application allocation. The 30 MiB conditional identity-map request
`[0x48200000,0x4a000000)` is skipped because the matched monitor's
`+0x18` flag is zero and the pinned SPL does not overwrite it. This does not
prove the broader gap is free from every indirect or vendor runtime access;
it only means those skipped mapping calls should not be treated as active
reservations. The endpoint `0x4a000000` is the native BL33 loader-slot start.
The unknown reservation `[0x7ec00000,0x7fc00000)` also remains excluded.

No direct reference into proposed application candidate
`[0x40080000,0x42000000)` was recovered. **This does not prove the candidate
free.** Direct control-flow traversal cannot fully model indirect calls or
branches, exception-return paths, runtime-computed pointers, secure-world
services, DMA, cache aliases, secondary-core behavior or all vendor runtime
paths. Static translation-map arguments also do not establish physical
allocation or security ownership.

Keep `H700_MEMORY_LAYOUT_APPROVED` unset. Do not assign
`MEMBASE/MEMSIZE`, run a payload, or write Boot0/TOC1/SD.

The reproducible audit is implemented by
`scripts/audit-rg34xx-bl31-memory.py` with host regression coverage in
`scripts/test-audit-rg34xx-bl31-memory.py`. The refined audit workflow
[#45795120056](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45795120056)
passed and uploaded report/disassembly artifact
[15104447110](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45795120056/artifacts/15104447110).
No hardware I/O, MMIO write, flash or payload execution was performed.


### Boot0 / BL31 single-core startup evidence (static; not device verified)

The read-only primary Boot0 capture (`131072` bytes, SHA-256
`bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f`,
eGON checksum valid) was decoded and cross-checked against the pinned
BaseOS native SPL and the matched BL31 address-reference report.

- Boot0 reset helper at `0x231e0`, called from `0x23224`, is gated by bit 29
  of `0x03006248`. When the gate is set it is called with `r0=2` and
  `r0=3`; it is not called with core 1.
- The helper ends by writing `C0_RST_CTRL & ~(1 << core)` to
  `0x09010000`. Under H616 reference semantics, clearing the bit asserts
  core reset (0=assert, 1=deassert). If the target register begins with the
  H616 POR low nibble `0x1`, CPU1-3 remain reset and CPU0 is released.
  The gate value and live register state at BL33 handoff have not been
  measured.
- Pinned SPL at BaseOS commit
  `cbe253a43f46cf551cc9a6c79a84015bde3686cf` programs the primary RVBAR
  low/high pair only; it does not directly write `C0_RST_CTRL` or
  CPU1-CPU3 RVBAR registers.
- The matched BL31 reachable scan reports CPU-related accesses only at
  `0x09010010` (`C0_CTRL_REG0`, CPU0 AArch64 mode path) and
  `0x08100020` (CPU subsystem control). No direct reachable access was
  recovered to `C0_RST_CTRL`, `DBG_REG1` or CPU1-CPU3 RVBAR registers.
- H616 documentation calls `0x09010044` `RVBARADDR0_H` (CPU0 high word),
  while the Boot0 helper ORs core2/core3 one-hot bits into it. H616
  documentation also marks `DBG_REG1.C_DBGPWRUPREQ[3:0]` read-only while
  Boot0 performs a read-modify-write. These are unresolved H700-vs-H616
  semantic discrepancies; do not infer the purpose or modify those fields.

The result supports a CPU0-only handoff hypothesis, but CPU1-CPU3 hold-off
is **probable, not device-verified**. See
[docs/h700-a53-single-core-startup-audit.md](h700-a53-single-core-startup-audit.md)
and CI
[#45808401081](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45808401081).
