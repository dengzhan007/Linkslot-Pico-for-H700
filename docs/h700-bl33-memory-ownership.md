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


### Host-only A32 BL33 scoped-abort overlay layout (2026-11-26)

The latest pinned GammaOS LK overlay test (code commit
`545f944936549044dcb36d57d8b86b77f812b700`; feasibility
[#45834208280](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280),
artifact [15123653120](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280/artifacts/15123653120))
links the modified CRT0, original exception assembly, probe-scoped Data Abort
return trampoline, Thumb matcher C, and ARM-state SYS_CFG probe with minimal
LK stubs.

The linked ELF/map reports:

| Item | Value |
|---|---|
| ELF / entry | ELF32/ARM; `_start=0x4a000000` |
| Vector table / wrapper | `0x4a001000` / `0x4a001024` |
| Boot-argument scratch | `0x4a001020` |
| Abort vector | `0x4a0011d0`; `cpsid if` is the first instruction |
| A32→Thumb handler interworking | `BLX` at `0x4a0011f4` |
| Thumb→ARM probe interworking | `BLX` at `0x4a001640` |
| Probe faulting load label | `0x4a00166c`; one `LDR` to `0x03000024` |
| Probe recovery label | `0x4a001688` |
| Abort stack BSS | `0x4a0016ac..0x4a0036ac` (8 KiB) |
| Probe active word / `_end` | `0x4a0036cc` / `0x4a0036d0` |
| Flat raw image | 5,804 bytes |
| Active 32 KiB loader file headroom | 26,964 bytes |

The in-memory footprint through `_end` is `0x36d0` bytes (14,032 bytes),
which is distinct from the 5,804-byte flat raw file. The linker's conservative
runtime memory bound remains `0x4a0e0000`; the active native firmware's
32-KiB extent limits the *input file* and is padded to exactly 32 KiB by the
opt-in native builder.

This is a minimal stub-linked source/layout test only. It does not include
the complete H700 platform, a visible result signal or a full LK kernel; no
vendor header was accepted as valid by this test, no native firmware/SPL
package was built from this binary, and no device ran it. It does not approve
any physical memory range, heap/stack/DMA allocation or bootloader handoff.
`H700_MEMORY_LAYOUT_APPROVED` remains unset.

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
- The original direct-entry reachable scan reports CPU-related accesses
  at `0x09010010` (`C0_CTRL_REG0`, CPU0 AArch64 mode path) and
  `0x08100020` (CPU subsystem control). That scan does not follow indirect
  platform callback dispatch.
- A later exact full-image/function-table audit found a registered PSCI
  platform CPU_ON callback: table `0x480165f8`, slot `+8` ->
  `0x4800254c` -> `0x48002d78`. This low-level path programs a per-core
  RVBAR and performs SoC-specific power/reset operations. Therefore it is
  incorrect to conclude that BL31 has no secondary-core start mechanism.
  The callback is on-demand and its presence does not prove it runs during
  the initial BL31-to-BL33 handoff.
- The active SoC variant value from SYS_CFG VER_REG `0x03000024` and the
  live target CPU reset state remain unknown. There is now a statically
  confirmed bank-selection mismatch to resolve: BL31 selects the alternate
  `0x08100040` RVBAR bank whenever low3 is nonzero, while pinned native SPL
  selects it only when low3 equals 2. The rules agree for values 0 and 2 and
  diverge for 1, 3, 4, 5, 6 or 7. The public H616 manual leaves VER_REG[7:0]
  undefined; open-source H616 tooling documents a die selector, but does not
  establish the H700 value. Read only `VER_REG & 7` in a trusted early
  environment and make SPL/BL31 share one verified target-specific rule
  before treating CPU_ON as safe. The repository now includes
  `scripts/read-h700-rvbar-selector-readonly.py` for an initial Linux-side
  selector sample (one `/dev/mem O_RDONLY` read; outputs only masked low3).
  That confirms the selector if readable but does not establish pre-BL33 CPU
  reset state.
- H616 documentation calls `0x09010044` `RVBARADDR0_H` (CPU0 high word),
  while the Boot0 helper ORs core2/core3 one-hot bits into it. H616
  documentation also marks `DBG_REG1.C_DBGPWRUPREQ[3:0]` read-only while
  Boot0 performs a read-modify-write. These are unresolved H700-vs-H616
  semantic discrepancies; do not infer the purpose or modify those fields.

The combined evidence supports a CPU0-only **initial handoff** hypothesis,
but CPU1-CPU3 hold-off remains **probable, not device-verified**. BL31 can
start a core later on a PSCI CPU_ON request; this is not evidence of automatic
startup during the initial handoff. See
[docs/h700-a53-single-core-startup-audit.md](h700-a53-single-core-startup-audit.md),
Boot0/BL31 CI
[#45808401081](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45808401081),
and PSCI callback CI
[#45816600012](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45816600012)
and RVBAR selector parity CI
[#45817598323](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45817598323).


### 32 KiB file extent and runtime memory are tested separately

The final host checks reject an overlay raw file above 32 KiB and verify
that the native override validator accepts exactly 32,768 bytes but rejects
32,769 bytes. The opt-in builder pads the accepted raw payload to exactly
32 KiB. The minimal overlay's 5,804-byte raw binary, 14,032-byte linked
footprint through `_end`, and conservative `0x4a0e0000` runtime link bound
are three distinct quantities. Full feasibility
[#45834208280](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280) passed
81/81 (artifact [15123653120](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45834208280/artifacts/15123653120)).
This does not approve any physical memory range or establish deployed
firmware pairing.


### 2026-11-26 real LK early-MMU / page-table layout measurement (host link only)

The successful full feasibility run [#45843270056](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45843270056) now links the pinned LK's actual `arm_mmu_init()` and H700's real `platform_init_mmu_mappings()` into a minimal A32 layout ELF. It reports the actual LK 4096-entry section table at `tt=0x4a008000` (16 KiB aligned), with linked `_end=0x4a00c024` and an image-base-to-`_end` span of `0xc024` bytes. Raw flat ELF size is 7,912 bytes and fits the existing 32 KiB loader file extent; this is not evidence that the runtime BSS/table range is physically safe or available.

The linker address is still inside the historically copied `0x4a000000..0x4a0e0000` loader bound. No independent BL31/Boot0 ownership approval is inferred from the successful link. Keep `H700_MEMORY_LAYOUT_APPROVED` unset; do not build an installable package, flash or execute the ELF. Secondary A53 hold-off/SMP and early APB1/PIO handoff access are separate unresolved gates.


### 2026-11-27 LK early stack + page-table non-overlap check (host link only)

Full Linkslot/LK feasibility [#924](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46021131707)
passed 75/75 on HEAD `5e85194cb473ee29ad909cc231128ddba5df87c0`.
Its artifact [15208527176](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46021131707/artifacts/15208527176)
contains the actual `h700-early-led-layout.map` and disassembly for the
minimal source/layout ELF. The stack/page-table assertions were added after
#921 and are first validated by #924.

The map resolves the pinned LK CRT0's 8 KiB initial/abort stack and the real
LK 4096-entry AArch32 translation table as:

| Object | Exact interval | Size |
|---|---|---:|
| `abort_stack` | `[0x4a004000,0x4a006000)` | 8 KiB |
| Initial stack top | `0x4a006000` | 8-byte aligned |
| Unallocated gap | `[0x4a006000,0x4a008000)` | 8 KiB |
| `tt` | `[0x4a008000,0x4a00c000)` | 16 KiB |
| `g_power_led_service` / `_end` | `[0x4a00c000,0x4a00c024)` | 36 B |

The raw flat ELF is 7,940 bytes (24,828 bytes below the 32 KiB input
extent); linked `_end=0x4a00c024`, giving an image-base-to-`_end` span of
`0xc024` bytes. The CI test now obtains `abort_stack`, `abort_stack_top`
and `tt` from the linked symbol table, requires the pinned 8 KiB stack,
checks stack/table containment within the link bound, enforces AAPCS stack
alignment and rejects any stack/table overlap. The current measured gap is
8 KiB.

This closes a **host-side internal-layout consistency check** that was
previously implicit in the map. It does not prove the BL33 slot is free of
opaque Boot0/BL31 consumers, does not approve any stack/table memory for
hardware use, and does not verify APB1/PIO clock/security state. The
production PI12 wrapper remains default-off; no image/package was generated,
no boot media was written and no payload was executed. Keep
`H700_MEMORY_LAYOUT_APPROVED` unset.

### 2026-11-26 BaseOS SPL PIO access is pre-BL31 evidence only

The pinned BaseOS-MultiFrontend source
[`cbe253a43f46`](https://github.com/dengzhan007/BaseOS-MultiFrontend/tree/cbe253a43f46cf551cc9a6c79a84015bde3686cf)
shows the active RG34XX native SPL's `baseos_vendor_handoff()` running the
panel sequence before the monitor reset: PI15/PI14 configuration and output
through `0x0300b124` / `0x0300b130`, plus PE6 configuration/read through
`0x0300b090`, `0x0300b0ac`, and `0x0300b0a0`. The matching RG34XX panel
contract is rule=2 (PE6 strap; high=old, low=boe).

This establishes that the native SPL uses the main PIO block before BL31;
it does not establish that vendor BL31 preserves APB1/PIO clock, reset or
access state until the first A32 BL33 instruction. The Linux clock tree is a
later runtime snapshot. This is not a memory allocation or BL33 access
approval. Keep PI12 physical access disabled until a post-BL31/pre-BL33
state proof, memory ownership review and secondary-core gate are complete.
`H700_MEMORY_LAYOUT_APPROVED` remains unset.


### 2026-11-28 decode of the fixed BL31 source-page PFN (read-only Linux metadata)

The published [2026-11-27 PFN capture](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-dram-pfn-20261127/dram-retention-pfn.txt) was retained as raw bytes pending verification of the Linux 4.9.170 KPF definitions. Decoding its two 8-byte little-endian values against the stable v4.9.170 UAPI/internal definitions used by `scripts/audit-rg34xx-dram-pfn-metadata.py` gives:

- Target PFN `0x40000`, page `[0x40000000,0x40001000)`, containing the BL31 source window `[0x40000000,0x40000140)`.
- Raw bytes `68 58 00 00 00 00 00 00` decode **little-endian** to `kpageflags=0x5868` (not `0x6858`). Set flags include `KPF_UPTODATE`, `KPF_LRU`, `KPF_ACTIVE`, `KPF_RECLAIM`, `KPF_MMAP`, `KPF_ANON`, and `KPF_SWAPBACKED`; `KPF_BUDDY`, `KPF_RESERVED`, and compound/huge-page flags are not set.
- `kpagecount=1`; `KPF_MMAP` and `KPF_ANON` agree with the nonzero mapcount. The decoder classification is `LINUX_ACTIVE_ANON_MAPPED_PAGE`, gate `BLOCKED_LIVE_LINUX_PAGE_REQUIRES_EXACT_PRESERVE_RESTORE`.
- `page_owner` is unavailable and the mapping/process identity is not captured. This is one post-boot Linux metadata sample, not proof of the page's owner at BL33 entry, the BL31 save/restore timing, or whether another CPU/device can access it.

Operational consequence: keep the entire 4 KiB page excluded from LK scratch, stack, heap, page tables, DMA, and emulator allocations. The sample establishes an active anonymous mapped page in the captured Linux runtime, but does not identify the mapping or prove the page's state at the earlier BL31 handoff. The exact v4.9.170 KPF definitions are referenced by the decoder; the exact vendor kernel source/build identity is still not recovered. No DRAM contents were read, no SMC/MMIO was issued, and no suspend/resume was attempted. `H700_MEMORY_LAYOUT_APPROVED` remains false.


The matched BL31 memory audit [#138](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46121129729) now runs both classifiers against the captured reports. Its memblock page classifier reports `UNRESERVED_SYSTEM_RAM` for the 4 KiB page containing `0x40000000` (`BLOCKED_SOURCE_PAGE_NOT_RESERVED`), while the independent KPF decoder reports `LINUX_ACTIVE_ANON_MAPPED_PAGE` with mapcount 1. These findings are not contradictory: memblock's reserved array is not a live allocator/page-owner map. Together they explicitly rule out interpreting “not memblock-reserved” as “free”. The page remains excluded from all LK allocations until the exact mapping owner, BL31 snapshot/restore lifetime, and CPU/DMA/cache interactions are established. CI success validates the host-side decoders against the captured inputs; it does not resolve those ownership questions.


### Matched BL31 DRAM-retention save/restore path (static binary audit #138)

The successful matched-monitor audit [#138](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46121129729), artifact [15253558130](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46121129729/artifacts/15253558130), identifies a specific standby path relevant to the live PFN above:

- The special power-entry handler at `0x48001a04` copies the 0x9410-byte standby image from the monitor payload into SRAM `[0x20000,0x29410)`, with auxiliary/context blocks at offsets `0x80`, `0xd4`, and `0x134` under conditional branches.
- Before DRAM retraining, the matched BL31 copies 160 words (0x280 bytes total) from two 0x140-byte DRAM source windows into SRAM scratch at `0x4801b400`; after retraining/resume, it copies the saved bytes back. The first source is exactly `[0x40000000,0x40000140)`, the page sampled as active anonymous-mapped Linux PFN `0x40000`. The second source is `DRAM_BASE + total_DRAM_capacity/2`; for the captured 1 GiB RAM capacity its candidate address is `0x60000000`, but the actual runtime geometry selector is not captured.
- The second-window selector is derived by helper `0x4800c730` from context DRAM geometry fields (`+0x18`, `+0x1c`, and halfword `+0x7a`), unless context `+0x1c` bit31 is already preset. Controller IDs read from `0x03006200` add a threshold exception. Resume is gated by `0x48011320`; a zero result skips the restore/copy path. When context `+0x4 == 4`, the gate instead reads `[0x40000000]` as a copy-enable predicate and selects alternate strided-copy workers.
- The SRAM scratch pointer is published through RTC GP_DATA1 `0x07000104`. The 0x280-byte scratch lies in the reserved monitor carveout, outside the 107,216-byte monitor file payload. This is a static binary reconstruction; it does not establish live SRAM ownership, actual context values, whether the standby handler is selected, or whether every save/restore branch completes.

This changes the interpretation of the live PFN evidence: the page is not free, but the matched BL31 contains a deliberate snapshot/restore mechanism intended to preserve the two DRAM windows across its retraining path. Do not infer that this mechanism makes the page generally allocatable or proves the mechanism ran on this unit. G2 remains blocked until the active selector/context and exact save-before-retraining / restore-before-Linux-resume sequence, plus CPU/DMA/cache quiescence, are established. No suspend/resume, SMC, MMIO, SRAM/DRAM content read, or hardware execution was performed.


### 2026-11-28 Boot0 DRAM geometry cross-check (host-only)

The 2026-11-27 read-only inventory contains two checksum-valid Boot0 copies. Their primary 32-word DRAM parameter blocks agree on the geometry-bearing fields:

- Both identify DRAM type 8 (LPDDR4), parameter clock field 672 MHz, word[6] `0x000030fa`, and word[7] `0x04000000`.
- The pinned BaseOS native SPL encoder in `experiments/h700-boot-speed/spl/dram-select.c` defines word[6] as `(3 << 12) | (rows << 4) | columns`, and word[7] as `(size_MiB << 16) | (dual_rank ? 0x1000 : 0) | (narrow_bus ? 1 : 0)`. Applying that exact encoding yields 15 rows, 10 columns, one rank, 32-bit bus, and 1024 MiB.
- The resulting 1 GiB capacity independently matches the live Linux memblock window `[0x40000000,0x80000000)`. Thus the matched BL31 midpoint formula `DRAM_BASE + total_DRAM_capacity/2` yields a second source-window candidate `[0x60000000,0x60000140)`.
- A reproducible fail-closed decoder and unit test are now in `scripts/audit-rg34xx-boot0-dram-geometry.py` and `scripts/test-audit-rg34xx-boot0-dram-geometry.py`; the BL31 memory audit workflow executes them against the pinned reports.

Limits: the two Boot0 primary parameter blocks are captured candidate/training metadata; the captures do not prove which Boot0 copy or runtime branch supplied the live trained state. The debug bytes differ (primary 3, secondary 0) and are not interpreted here. The 672 MHz field is not an independently measured live DRAM clock. The matching Linux memblock is post-boot evidence. The midpoint address is now derived from mutually consistent Boot0 geometry and Linux capacity, but the actual BL31 context selector, selected branch, resume gate, and successful save/restore execution remain unobserved. Keep both source windows excluded from LK allocations; `H700_MEMORY_LAYOUT_APPROVED=false`.


The geometry decoder was exercised against the exact pinned reports in BL31 static memory audit [#141](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46122355821), which completed successfully. Artifact [15254247497](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46122355821/artifacts/15254247497) contains `BOOT0-DRAM-GEOMETRY.json` and `BOOT0-DRAM-GEOMETRY.md`; the emitted values are 15 rows, 10 columns, one rank, 32-bit bus, 1024 MiB, and midpoint candidate `0x60000000`. The first two audit attempts (#139/#140) stopped at a Python import-name error in the new test harness; #141 corrected that import and passed all audit steps. Do not treat the failed attempts as hardware or decoder failures.


### Runtime BL31 context pointer: static producer remains unresolved

A full host disassembly of the exact 107,216-byte matched monitor payload confirms that the resume path repeatedly loads a context pointer from the global slot at `0x48014668` (for example, `0x4800c434`, `0x4800c4e0`, `0x4800c63c`, and `0x48013480`). The save/restore helpers then consume fields from that pointed-to object. The retention worker at `0x48011938` receives the pointer in `x1`, preserves it in `x19`, and reads offsets `+0x18`, `+0x1c`, `+0x78`, and `+0x04`.

The monitor file's bytes at file offset `0x14668` are `02 00 00 00 00 00 00 00`, not a usable pointer. The captured image therefore does not disclose the runtime pointer value. A full linear disassembly finds direct reads of the slot but no obvious direct store to the same slot; an indirect writer, external setup, runtime relocation, or changed mapping is not ruled out. Do not substitute the static file qword for live context.

The second-window address calculation is now instruction-level clear: if the context field at `+0x1c` is not already marked with bit31, helper `0x4800c730` derives a selector from context `+0x18`, `+0x1c`, and halfword `+0x7a`; controller IDs `0x5c00` and `0x5d00` take a special branch. The caller may rewrite the upper half of `+0x1c` with the helper result. Later it calculates `0x40000000 + (((context[+0x1c] >> 17) & 0x7fff) << 20)`. Thus `0x60000000` corresponds to a final selector value of 512 MiB, but the context and selector result remain unavailable.

The restore gate at `0x48011320` returns the result of the context helper at `0x480115c0` after clock-restore setup; for context mode `+0x04 == 4`, it also reads the first word at `0x40000000`. The caller at `0x48011974` skips the subsequent copy worker when the gate returns zero. No captured evidence supplies the pointer, field values, controller ID, or gate return. The next valid route is to locate the runtime context setup path in the matched image and any upstream resume stub; a new device probe is not justified.


### 2026-11-28 whole-payload scan of BL31 context slot 0x48014668

A new identity-pinned, host-only overlapping AArch64 scan was run against the exact 107,216-byte monitor (SHA-256 `676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31`). Audit [#142](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46123660057) passed; artifact [15254871932](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46123660057/artifacts/15254871932) contains `BL31-CONTEXT-SLOT-SCAN.json/.md`.

- The scanner recovered 10 candidate reads/references to global slot `0x48014668`, including `ldr x19,[x20,#0x668]` at `0x4800c434`, `ldr x1,[x20,#0x668]` at `0x4800c4e0`, reads at `0x4800c63c/0x4800c654/0x4800c65c`, and later reads around `0x48013408`, `0x48013e1c`, `0x48013e24`, `0x48013f20`, and `0x48013ff0`.
- It recovered **zero candidate direct writes** to the slot. This is not proof of no writer: the scan uses short overlapping decode windows and only tracks ADR/ADRP, ADD/SUB-immediate, and MOV; indirect pointer stores, exception paths, and external initialization remain outside its model.
- This confirms that the resume/retention paths repeatedly consume a slot at this address, but still does not reveal its runtime value. The static file qword at offset `0x14668` is `2`; that cannot be used as a valid runtime context pointer. Treat the slot as runtime-initialized or externally supplied until a complete writer/relocation/handoff path is proven.
- The scan's possible reads are heuristic leads because overlapping decoding may interpret data/literal bytes as instructions. Validate each candidate against the reachable disassembly/function boundaries before assigning a function semantic. No monitor code was executed and no device state was read.

The new scanner is `scripts/audit-rg34xx-bl31-context-slot.py` and is part of `.github/workflows/h700-bl31-memory-audit.yml`. The next static investigation is the initialization of the relevant monitor data region and any Boot0/SPL/secure-entry handoff that can populate it; do not infer the runtime pointer from the file's initial bytes.


### 2026-11-28 validated call chain from context slot to DRAM retention worker

The callback disassembly gives a concrete link between the context slot and the retention worker, rather than relying only on nearby offsets:

- At `0x4800c654`, BL31 loads `x0 = *(u64 *)0x48014668`; at `0x4800c658` it calls `0x48011b58`.
- `0x48011b58` calls `0x48011b10`; the latter forwards its incoming `x0` into `x1`, sets `w0=0`, and calls `0x48011938`.
- `0x48011938` stores `x1` in `x19` and consumes the context object fields `+0x18`, `+0x1c`, `+0x78`, and `+0x04`; it then runs the retention gate and selector/copy path described above.

This confirms that the value loaded from `0x48014668` is passed as the context argument on this call chain. It does not identify who initializes the slot or establish that this branch executes on the target device. The raw file qword at the slot is still `2`, which is not a usable pointer; the runtime value must differ before this path can safely dereference it.

The same matched monitor has a separate, demonstrable runtime-published pointer at `0x48015548`: helper `0x48013ac8` selects a controller object and stores it at `0x48013b14`. That is a distinct slot and does not initialize `0x48014668`; it only demonstrates that this binary uses small initial data values as placeholders for pointers that are later published by explicit setup code.

The whole-payload context-slot scanner was refined to propagate PC-relative literal loads and image-backed loads, then to record direct-call arguments. The final audit [#145](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46124497685) passed; its artifact [15254429553](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46124497685/artifacts/15254429553) still reports 11 candidate reads, zero candidate direct writes, and zero direct-call argument references to the slot. This is still bounded static analysis: it does not resolve stack spills, indirect stores, helper/memcpy semantics, or external initialization. The separate descriptor audit also reports no alias/CFG direct-store candidate to this slot, with the same caveat.

The remaining question is now specific: identify the producer of the runtime value loaded at `0x48014668`, or establish whether a loader/secure-entry path writes it outside the matched monitor. Do not treat the separate `0x48015548` controller pointer as evidence for this slot.


### Boot0/SPL producer-path exclusion: no direct context-slot literal found

A host-only raw-byte review of the exact Boot0 captures from BL31 audit #145 (artifact [15254688500](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46124497685/artifacts/15254688500)) adds a narrow exclusion:

- Primary Boot0: 131,072 bytes, SHA-256 `bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f`. Secondary Boot0 logical image: 65,536 bytes, SHA-256 `4701a85e4db7e8339b91b92f691a83cc954ec63a09369087c2e41d80eafa9f5d` (the captured backing file is 131,072 bytes; only the first 65,536 bytes are the checksum-valid logical image).
- Neither logical Boot0 image contains the direct 32-bit little-endian literals `0x48014668`, `0x48014000`, or `0x00014668`. The primary image contains one `0x48000000` literal at file offset `0x1b7cc`; adjacent words at `0x1b7c8/0x1b7cc` are `0x1a2d0/0x48000000`, matching the exact monitor payload length and load base. The secondary logical image contains no `0x48000000` literal.
- The pinned BaseOS native SPL contract independently verifies that its explicit monitor patches are limited to `+0x24..+0x2b`, `+0x30..+0xaf`, and `+0xb0..+0xb3`; it has no direct patch to `+0x14668`.

This rules out only an obvious direct-literal Boot0/SPL patch to the context slot. It does not rule out computed addresses, generic relocation/copy side effects, code outside the logical Boot0 extent, or runtime/indirect initialization inside BL31. It is not proof that Boot0 never writes the slot. The context value's producer remains unresolved.


### 2026-11-28 Boot0 disassembly: monitor-loader handoff routine identified, copy semantics still under review

The exact primary Boot0 capture was wrapped as a temporary ARM ELF and disassembled offline with LLVM; no device interaction occurred. The eGON entry branch at file offset `0x0` targets code at offset `0x400`. That early path saves CPU state, checks the current execution mode, validates the Boot0 image extent/signature, and then selects a later stage. The initial branch is not itself the DRAM-training or monitor-copy routine.

A later reachable-looking helper at file offset `0x1b7d4` is called from `0x1b6d0`; a nearby literal at `0x1b7cc` is `0x48000000`, with the preceding word `0x0001a2d0`. The latter matches the exact BL31 payload size and the former its load base. This is materially stronger than the earlier raw literal-only check: the address/size pair sits directly beside a function call in the Boot0 code tail and likely participates in a monitor load/copy operation. However, the helper's full control flow and its callers/arguments must be reconstructed before assigning semantics or claiming which source buffer is copied. The literal pair alone does not show a write to `0x48014668`; it indicates the BL31 extent transfer path is now the next concrete reverse-engineering target.

The full disassembly is not committed as a generated dump. The source capture and hashes remain the evidence source. Next inspect the routine at `0x1b7d4`, its caller at `0x1b6d0`, and the code that prepares its arguments; classify every destination write and any relocation/zero-fill performed over the monitor extent. Do not infer the runtime context pointer from the copy operation.


### Correction: Boot0 tail is mixed-ISA data/code; the 0x1b7d4 target is not a monitor-copy helper

A second-pass disassembly with the correct AArch64 decoder invalidates the tentative interpretation above. The ARMv7 decoder had misidentified AArch64 words as ARM instructions, including the apparent `BL` at `0x1b6d0`. Correct AArch64 decoding shows:

- `0x1b6d0` is `cmp x1, x0`, not a call.
- `0x1b6d4` is a conditional branch; `0x1b6f4` begins an AArch64 function that dispatches through a function pointer at descriptor offset `+0x20`.
- `0x1b7d4` is data (`0x00000920`), not executable code.
- The words at `0x1b7c8/0x1b7cc` are a data pair `0x1a2d0/0x48000000`; they match the monitor extent length and load base but are not shown by this disassembly to be arguments to a copy routine.

Therefore the prior paragraph's claim that `0x1b7d4` is a monitor-loader helper and that `0x1b6d0` calls it is withdrawn. The reliable finding is narrower: Boot0 contains an AArch64 code/data region with a descriptor/dispatch function and an adjacent extent-like size/base pair. Its role and reachability from the Boot0 entry are not yet established. No monitor-copy semantics or context-slot writer can be inferred from it.

Next work must first identify the ISA transition and control-flow reachability into this AArch64 region, decode the descriptor layout around `0x1b740..0x1b7f8`, and trace the actual data consumer of the `0x1a2d0/0x48000000` pair. Do not use the previous tentative interpretation as evidence.


### Boot0 AArch64 tail: descriptor-dispatch structure recovered, no BL31 transfer proof

The AArch64 disassembly around `0x1b694..0x1b73c` resolves a small registry mechanism:

- `0x1b694` calculates the number of 0x28-byte records in a table beginning at `0x1c688` and ending at `0x1c6b0` (one record in this capture).
- It compares the requested key against the record's 32-bit field at `+0x08`.
- `0x1b6f4` saves the input and key, looks up the record, reads a callback pointer from `+0x20`, and tail-branches to it.
- The single record's key field is `0x4d`; its callback pointer at `0x1c6a8` is `0x2984c`.

This establishes a generic descriptor/callback dispatch in the AArch64 region. It does **not** identify that region as the monitor loader, nor show it is reached from the eGON Boot0 AArch32 entry. The nearby `0x1b7c8/0x1b7cc` size/base pair remains unconnected to a consumer. The next useful static task is to map the AArch64 region's caller/entry and inspect callback `0x2984c`, then separately locate references to the size/base descriptor. Do not claim BL31 is copied by this path until the transfer destination and source are observed in a validated reachable routine.


### Boot0 callback pointer address-space reconciliation (2026-11-28)

The primary eGON header reports `run_addr=0x20000` and public-header size `0x30`; the image's executable payload therefore begins at the natural candidate address `0x20030`. Reinterpreting the descriptor callback value `0x2984c` against that base gives:

`0x2984c - 0x20030 = 0x981c`

File offset `0x981c` is a coherent Thumb-2 function boundary in the primary Boot0 image. Its prologue saves r4-r11/lr, and its body consumes a structured context (including offsets `+0x20`, `+0x24`, `+0x28`, `+0x48`, `+0x104`), performs controller/register setup, and calls internal helpers. This is a stronger address correlation than treating `0x2984c` as a file offset. The adjacent descriptor callback `0x298b0` maps to file offset `0x9880`, which is inside the same Thumb function, not an independent entry point.

Important unresolved ISA issue: the registry lookup/dispatch sequence at file offsets `0x1b694..0x1b73c` decodes coherently as AArch64 and ends with `br x16`, while the mapped callback target at `0x981c` decodes coherently as Thumb-2. A plain AArch64 `BR` cannot perform an AArch64-to-AArch32/Thumb state transition. The callback's low bit is also clear. Thus the address correlation does not yet prove that this AArch64 dispatcher invokes the Thumb routine; the table/dispatcher may be data or code for a separate execution context, the target may be transformed elsewhere, or the Boot0 image may contain multiple independently used code regions. No reachable cross-ISA edge has been established.

The primary eGON entry itself is AArch32: its initial branch at file offset `0x0` targets `0x400`, where the first-stage code saves CPU state, checks CPSR mode and SCTLR bits, masks interrupts, sets SP to `0x57000`, and calls the routine at `0x4e0`. It then accepts selected return addresses (`0x21000` or `0x31000`) and transfers control through the value at `0x494` (currently `0x50000`). This validates the AArch32 entry path but does not connect it to the AArch64 registry at `0x1b694`.

Next static task: map the AArch32 entry's actual continuation/return path and the code/data boundary around the AArch64 registry; determine whether the descriptor table is consumed in another processor/firmware context or whether a state-switch trampoline exists. Until then, classify `0x2984c` → `0x981c` as a high-confidence address correlation with unresolved execution semantics, not proof of a monitor loader or BL31 copy routine.


### Boot0 AArch32 continuation: SRAM resume trampoline and zeroed context block

The primary Boot0 AArch32 entry path is now traced through its first helper and continuation:

- Entry at file offset `0x400` sets SP to `0x57000`, calls `0x4e0`, and accepts only return values `0x21000` or `0x31000`. It then transfers to the selected address through the literal at `0x494` (whose current value is `0x50000`).
- The helper at `0x4e0` validates the eGON image-size field at runtime address `0x20010` against `0x20000`, compares the eight-byte signature against the literal at `0x720`, computes/checks the image checksum, and performs a sequence of read/write/readback operations through the peripheral base formed as `0x070001f8`. The precise peripheral function and its relationship to the resume path are not inferred here.
- On the accepted path, code at `0x768` copies bytes from `0x20840..0x20884` (68 bytes) to SRAM `0x50000`. It then copies 72 bytes from `0x20498..0x204e0` to `0x52000`; the entire source interval is zero-filled in the captured image, so this operation initializes/clears the 72-byte context block rather than installing a second code stub.
- The 68-byte routine at file offset `0x840`, copied to `0x50000`, is an AArch32 restore trampoline. It copies a caller-supplied byte range back to `0x20000`, restores a saved CPSR and general-register/SP/LR context from `0x52000`, and finally branches to `0x20000`. The code at `0x840` itself is the copy/restore routine; the 72-byte block at `0x52000` is its context area.

This is a real Boot0 SRAM resume/restore mechanism and a useful upstream handoff lead, but it is not the BL31 context at `0x48014668`: it uses SRAM `0x50000/0x52000`, its 72-byte context is explicitly zeroed by this path, and no dataflow from this block to the BL31 slot has been found. It also does not prove that the AArch64 descriptor dispatcher at `0x1b694` is reachable from this AArch32 path. Treat it as a separate Boot0 resume mechanism unless a concrete bridge is discovered.

Next static step: identify the meaning of the peripheral handshake in `0x4e0` and search the Boot0 image for code that populates `0x52000` after the zeroing copy, then determine whether any path transfers the restored state or pointer into BL31. Keep the BL31 context producer unresolved.


### Boot0 resume gate detail: retained-image checksums and mailbox handshake

A closer read of helper `0x4e0` narrows the accepted resume path further:

- It validates the primary eGON header at `0x20000): the declared size at `0x20010` must be `0x20000`, and the eight-byte signature must match the embedded `eGON.BT0` signature at `0x20838`.
- It reads status at `0x070001f8` and branches according to bit `0x10000`.
- The normal branch reads the two words at `0x07000138`/`0x0700013c`, then calculates checksums over candidate retained regions `[0x21000,0x31000)` (64 KiB; expected `0x8fb6d562`) and `[0x31000,0x3d000)` (48 KiB; expected `0xa53ccffb`). It subsequently performs ordered write/readback operations at addresses derived from `0x030090b4`, including offsets `-4`, `0`, and `+4`; the peripheral identity and protocol meaning remain unassigned.
- The accepted result chooses `0x21000` or `0x31000`; the copied restore trampoline then copies the selected region back to `0x20000`, restores the context from `0x52000`, and branches to the Boot0 image base.

This is consistent with a retained-image/resume validation protocol, but it does not identify the peripheral as an RTC, PMIC, mailbox, or PSCI interface. A raw scan finds the `0x50000`, `0x20840`, `0x20498`, `0x070001f8`, and `0x030090b4` literals, but no direct `0x52000`, `0x07000138`, `0x0700013c`, or `0x48014668` literal. The two 0x07000138/13c register addresses are formed by the helper at runtime from a base plus a small offset; the 0x52000 destination is formed as an immediate in the copy loop. These literal-scan absences do not rule out computed aliases or writes from other code.

This path therefore strengthens the existence of a Boot0 retained-image resume protocol, not the claim that it initializes BL31's context slot. The runtime producer for `0x48014668` remains unknown.


### 2026-11-28 correction: Boot0 callback offsets and registry address space

A byte-level recheck of the captured 128 KiB primary Boot0 image corrects the previous callback mapping. The earlier note subtracted an assumed executable base of `0x20030` from callback addresses; that assumption was not established. With the eGON image mapped as a whole at its reported `0x20000` base, file offsets map directly as `runtime = 0x20000 + file_offset`:

- The Thumb-2 prologue is at file offset `0x981c` (runtime `0x2981c`).
- Descriptor callback value `0x2984c` maps to file offset `0x984c`, 0x30 bytes inside that function, not to its prologue.
- Adjacent value `0x298b0` maps to file offset `0x98b0`, also inside the same Thumb-2 routine, not to a separate function entry.

This supersedes the earlier statement that `0x2984c` maps to file offset `0x981c` and that `0x298b0` maps to `0x9880`. The correction weakens the proposed callback-to-function correlation: these table values are interior code addresses, and their low bits are clear. The AArch64 dispatcher ends in `br x16`, which cannot switch into Thumb state. No valid AArch64-to-Thumb call edge is established.

The AArch64-looking registry code at file offset `0x1b694`, interpreted under the same whole-image mapping, computes its descriptor table at runtime `0x3c688` (file offset `0x1c688`). The bytes there do contain the expected 0x28-byte records, key `0x4d`, and callback value `0x2984c`; this makes the code/table relationship internally address-consistent within the Boot0 image, but it does not resolve the ISA-state mismatch or prove that this code is reachable from the AArch32 eGON entry.

A cross-image comparison also rejects treating this as an unmodified copy of the live TOC1 monitor: Boot0 file window `0x1b694` is not byte-identical to monitor offset `0x13694` and has no matching 16-byte anchor in the monitor; the 0x80-byte Boot0 table window at `0x1c688` is not byte-identical to monitor offset `0x14688` (only 54/128 bytes match, largely zero fields). Therefore do not equate the Boot0 registry/table with BL31's `0x48014668` context slot or `0x48014688` table without a separate relocation/copy proof.

This is a static correction only. No Boot0/TOC1 write, payload execution, MMIO/SMC, or device test occurred. G2/G3/M1-S1 remain open.


### 2026-11-28 Boot0 mixed-ISA region: AArch32 resume path does not reach the AArch64 registry

The primary Boot0 image was disassembled in both AArch32 ARM and AArch64 modes around the mixed-ISA tail. The AArch64 sequence is coherent from file offset `0x1b548`: it contains ordinary A64 prologues/returns, A64 register operations, and a descriptor lookup/dispatch routine at `0x1b694..0x1b73c`. Under the whole-image `0x20000` mapping, these offsets correspond to runtime addresses `0x3b548..`. The descriptor table at file offset `0x1c688` is likewise internally consistent with that A64 routine.

The actual eGON primary entry is a separate AArch32 path. At file offset `0x400` it saves AArch32 state, checks CPSR mode and SCTLR bits, sets SP to `0x57000`, and calls the AArch32 helper at `0x4e0`. On its accepted resume path, it executes DSB/ISB and loads PC from the literal at `0x494`, whose captured value is `0x50000`. The 68-byte SRAM routine copied to `0x50000` is also AArch32: it copies the retained image to `0x20000`, restores CPSR and general-register/SP/LR context from `0x52000`, then branches to `0x20000`. This path contains no AArch64 exception return and no demonstrated state transition into the A64 tail.

A raw little-endian word scan of the full 128 KiB image found no direct occurrences of the whole-image runtime addresses for the A64 code entry/candidate routines (`0x3b548`, `0x3b580`, `0x3b640`, `0x3b694`, `0x3b6f4`, `0x3b728`) or its table (`0x3c688`). The AArch32 resume path therefore does not expose a direct-literal reference to the A64 region. This is a narrowing result only: it does not rule out computed pointers, an external processor/firmware consumer, a loader not present in this image, or an indirect handoff through other memory.

The A64 tail's descriptor values `0x2984c` and `0x298b0` still point into the AArch32 Thumb routine at file offsets `0x984c` and `0x98b0`, with low bits clear. The A64 dispatcher's `br x16` does not change execution state, so this remains an unresolved mixed-ISA table relationship rather than a valid callback edge. Do not treat the A64 tail as the primary Boot0 entry path, a BL31 loader, or proof of BL31 context-slot initialization.

Next static work should identify any independent loader/processor contract that consumes the A64 tail, or classify it as an embedded but unreferenced code/data region if stronger whole-image reachability evidence supports that. In parallel, continue BL31 indirect-writer tracing for `0x48014668`. No device execution, MMIO/SMC, payload execution, or firmware/TF write occurred; G2/G3/M1-S1 remain open.


### 2026-11-28 BL31 startup callback slot 0x4803c048: installer identified, distinct from G2 context slot

A fresh full-payload disassembly of the exact pinned monitor (107,216 bytes; SHA-256 `676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31`) resolves the previously open indirect callback load in `0x48007554`:

- `0x480075cc..0x480075d8` loads qword `[0x4803c048]`, tests it for zero, and calls it with `BLR` if nonzero.
- The exact writer is a three-instruction leaf at `0x480075e8`: `ADRP x1, 0x4803c000`; `STR x0, [x1,#0x48]`; `RET`. This is a direct store to `0x4803c048`.
- A caller at `0x4800a134` (function extent at least `0x4800a134..0x4800a1b0`) reaches that writer at `0x4800a1a0`. Immediately before the call it computes `x0 = 0x4800a504`; therefore the value installed in `0x4803c048` on that branch is the code address `0x4800a504`.
- The target `0x4800a504` is a valid AArch64 function prologue. Its body uses MPIDR_EL1, per-CPU indexing from `0x4803cff0`, TPIDR_EL3, and per-CPU state helpers. The callback is thus a BL31-internal AArch64 per-CPU/exception-context routine, not a Boot0 callback.

This resolves the *writer* and value for the separate callback slot `0x4803c048`, but not yet the reachability/registration timing of its installer function `0x4800a134` from the cold-boot initialization path. The startup routine `0x48007554` invokes the slot after its setup helpers; the captured code alone does not prove that `0x4800a134` has run before that first invocation. The region containing `0x4803c048` is within the startup-cleared BSS interval `[0x4801b300,0x4803d8f8)`, so the nonzero callback must be installed after that clear or by an external mechanism. This is a useful example of why a zero-initialized indirect callback can have a real in-image writer that short scans miss.

Do not conflate `0x4803c048` with G2 slot `0x48014668`: the former is a separate callback pointer; the latter is consumed as a DRAM-retention context pointer by the `0x48011938` path. No write to `0x48014668` is established by this finding. Next trace the callers/registration route into `0x4800a134`, then resume indirect writer analysis for `0x48014668` including full-payload stores and stack/helper propagation.

No monitor execution, device access, MMIO/SMC, suspend/wake, or firmware write occurred. G2/G3/M1-S1 remain open.


### 2026-11-28 follow-up: callback installer 0x4800a134 has no direct BL caller in the matched image

The exact monitor was disassembled as a complete AArch64 image with the verified load base 0x48000000. The following details narrow the remaining callback-registration question:

- Function 0x4800a134 is a complete function. It reads MPIDR_EL1, maps the affinity through helper 0x48015630, and calls 0x48002138. It then checks a returned object (including its +8 field); on the initialized branch it computes a per-CPU record at 0x4803cff0 + cpu_index * 0x240, calls 0x48009f60 with that record and flag 1, and installs 0x4800a504 through writer 0x480075e8. Error/skip branches return without installing the callback.
- A whole-image direct-branch scan finds no BL/B immediate targeting 0x4800a134. A raw qword at 0x48018d00 does equal 0x4800a134, but the nearby data is mixed metadata and this pointer alone does not establish a caller or execution path.
- The nearby MIDR dispatch helper 0x48015cd0 explicitly scans [0x48018d10, 0x48018d70) in 0x20-byte records. That range starts after 0x48018d00; therefore it does not consume the qword at 0x48018d00. Do not claim that this known MIDR lookup invokes 0x4800a134.
- Exception entry 0x48015884 uses a separate table-like region based at 0x48018ca8 and an index scaled by 0x20. Its calculated record addresses are base + multiples of 0x20; 0x48018d00 is not one of those aligned lookup addresses. Thus the visible exception-dispatch lookup also does not explain the 0x48018d00 pointer.

Current interpretation: the writer and installed callback value are proven, but the installer function's invocation source remains unresolved. The qword at 0x48018d00 is a data xref lead only. Continue by identifying the owner/consumer of that record and any indirect branch, while avoiding assumptions from raw pointer coincidence. This does not alter the separate G2 finding: no write to 0x48014668 is established.

No monitor execution, device access, MMIO/SMC, suspend/wake, or firmware write occurred. G2/G3/M1-S1 remain open.


### 2026-11-28: 0x4800a134 is an auxiliary field in an exception descriptor, not a proven dispatch target

A byte-accurate decode of the 32-byte records at `0x48018ca8..0x48018d68` refines the earlier raw-qword lead:

- The exception entry at `0x48015884` sets `x11=0x48018ca8`, derives an index from the exception vector, scales it by 0x20, and executes `LDR x15,[x11,w10,uxtw]`. It then invokes only that first qword via `BLR x15` at `0x480158c4`.
- Record `0x48018ce8` is four qwords: `0x4800a29c`, `0x00013f32`, `0x480184e1`, `0x4800a134`. Thus `0x4800a134` is the fourth qword (+0x18) of the record whose first-qword dispatch handler is `0x4800a29c`; the dispatcher does not load or call the fourth qword.
- Adjacent record `0x48018d08` starts with `0x4800a29c` and has different trailing fields. This confirms a structured 0x20-byte exception descriptor table, not a flat array of function pointers.
- Therefore the earlier raw pointer at `0x48018d00` is not evidence that the exception dispatcher invokes `0x4800a134`. It is an auxiliary descriptor field associated with handler `0x4800a29c`. Its consumer, semantics, and any possible indirect use remain unresolved.

This narrows the call-site question: the known exception dispatcher only uses the first qword, so the fourth-qword pointer is not its direct handler target. Do not describe `0x4800a134` as an exception callback or claim it is invoked by this table without finding a separate consumer. The installer function's execution path remains unknown; consequently the actual runtime initialization of `0x4803c048` remains unproven despite the direct writer and callback value being identified.

No G2 writer for `0x48014668` is established by this finding. No monitor execution, device access, MMIO/SMC, suspend/wake, or firmware write occurred. G2/G3/M1-S1 remain open.


### 2026-11-28: G2 direct-store cross-check against the complete BL31 disassembly

The exact monitor image (107,216 bytes; SHA-256 `676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31`) was rechecked as a whole-image AArch64 disassembly, focusing on the effective address `0x48014000 + 0x668 = 0x48014668`.

- The references already identified at `0x4800c434`, `0x4800c48c`, `0x4800c4e0`, `0x4800c560`, `0x4800c63c`, `0x4800c654`, `0x4800c65c`, `0x4800c6c8`, `0x48011cd4`, `0x48011d30`, `0x48013408`, `0x48013e1c`, `0x48013e24`, `0x48013e44`, `0x48013e4c`, `0x48013e8c`, `0x48013f20`, `0x48013f5c`, `0x48013f9c`, `0x48013ff0`, `0x48014010`, and `0x48014028` are load sites. No matching direct `STR/STP/STUR/STLR/STXR/STLXR` operand targeting `[xN,#0x668]` was found in the decoded image.
- The image's initial qword at `0x48014668` is `2`. The surrounding bytes from `0x48014600` onward form a dense structured data region; nearby code at `0x48013ac8` also performs an indexed 64-bit load from a region based at `0x48014628`. This proximity is a data-layout lead only; it does not establish the semantics or runtime owner of the slot.
- This negative direct-store result is narrower than a complete writer proof. It does not exclude a store through a computed alias, a helper/memcpy destination, stack-mediated or register-indexed addressing, exception/secure-call setup outside this image, relocation, or a write performed before/after the captured BL31 execution phase.
- Accordingly, the strongest defensible statement is: **no direct static store to the exact page-base-plus-offset operand was found in this whole-image disassembly; runtime writer and live pointer value remain unknown.** Do not mark G2 resolved.

This was a host-only inspection of the existing captured monitor. No device access, monitor execution, MMIO/SMC, DRAM inspection, suspend/wake test, media write, or firmware operation occurred.


### 2026-11-28 relocation-aware correction: standby context pointer resolves to SRAM 0x20000

**This section supersedes the earlier interpretation in this document that treated source-coordinate address `0x48014668` as the live runtime context slot and its file qword `2` as evidence of an unknown runtime writer.** That interpretation did not account for the standby image being copied and executed from SRAM.

The successful host-only audit #145 artifact `15254429553` includes `TWI-SELECTOR-RELOCATION.md` and `twi-selector-relocation-report.json`. Rechecking those exact outputs establishes:

- The standby image source interval is `[0x4800c220, 0x48015630)`, copied to SRAM starting at `0x20000` (0x9410 bytes).
- The instruction pair at source PCs `0x4800c430/0x4800c434` is `ADRP x20, 0x48014000`; `LDR x19, [x20,#0x668]`. After relocation, the ADRP page is `0x28000`, and the load's runtime effective address is `0x28668`.
- The source-equivalent backing address for runtime `0x28668` is `0x48014888`, not `0x48014668`. Its captured monitor qword is `0x0000000000020000`, so the relocated load obtains context base `0x20000` from the copied standby SRAM image.
- The old source-coordinate qword at `0x48014668` is indeed `2`, but it is not the backing qword consumed by this relocated standby path. Searching for a writer to `0x48014668` therefore targeted the wrong coordinate for this execution path.
- The same relocation audit maps the TWI lookup table to runtime `0x28628` / source-equivalent `0x48014848`. Original TOC1 FDT selector derivation yields `twi5`, selecting R-TWI0 base `0x07081400`. The selector at external context `0x4803cbb4` is staged to copied SRAM context field `0x200a4`. This is static selector provenance only; no PMIC transaction or suspend execution is claimed.

The context-pointer *address mapping and initial backing value* are therefore statically resolved for the copied standby image. This does **not** prove that the standby handler is selected on the physical device, that the context fields hold the expected live DRAM geometry, or that save/restore completes before Linux resumes. The active anonymous-mapped PFN at `0x40000000`, the second DRAM source window, CPU/DMA/cache quiescence, and the exact retention/resume lifecycle remain ownership blockers. Keep the candidate DRAM pages excluded; `H700_MEMORY_LAYOUT_APPROVED=false`.

The legacy whole-image scan at `0x48014668` is retained only as a historical source-coordinate diagnostic. It must not be interpreted as a scan of the relocated runtime effective address. The scanner now emits the relocation-aware mapping and validates source qword `0x48014888 == 0x20000`. No device execution, live memory readback, SMC/MMIO/TWI transaction, suspend/wake, storage write, or firmware operation occurred.


### 2026-11-28 SPL-to-BL31 DRAM parameter provenance and conditional midpoint derivation

The relocation correction identifies the context base, and the pinned BaseOS SPL source identifies how its first 0x80 bytes are populated. This closes another static dataflow gap, but still does not establish the selected candidate or suspend execution on the captured unit.

**Parameter handoff chain:**

1. In pinned BaseOS source `experiments/h700-boot-speed/spl/dram-select.c`, `baseos_selected_vendor_params()` returns `trained_vendor_params[32]`. After a candidate has passed training, size detection, and the memory-hold check, `baseos_normalize_vendor_params()` copies the selected candidate's 32 words, then replaces word[6] with the detected rows/columns encoding and word[7] with the detected capacity/rank/bus encoding. Other candidate fields, including word[30], are preserved.
2. In `experiments/h700-boot-speed/spl/native-boot.c`, `baseos_vendor_handoff()` obtains that pointer and copies exactly 128 bytes to `monitor + 0x30` (also to U-Boot +0x30). This is the source of the monitor's runtime-patched parameter block; the captured monitor file's ASCII bytes at +0x30 are only its unpatched placeholder, not the runtime parameter values.
3. BL31 special power-entry routine `0x48001a04` copies the 0x9410-byte standby image to SRAM `0x20000`, then copies 0x80 bytes from monitor `0x48000030` over SRAM `0x20000..0x20080`. Therefore the context pointer resolved in the previous section points to this parameter block at SRAM `0x20000`.
4. The retention worker reads context +0x18, +0x1c, +0x78 and halfword +0x7a. These map directly to vendor parameter words [6], [7], [30], and the high halfword of [30], respectively. Thus the DRAM geometry inputs are supplied by the SPL's trained parameter block, not by the placeholder bytes in the unpatched monitor dump.

**Conditional 1-GiB calculation using the captured Boot0 primary candidate:**

The read-only Boot0 report `reports/rg34xx-inventory-20261127-uart/boot0-primary.json` records candidate type 8 / LPDDR4, word[6] `0x000030fa`, word[7] `0x04000000`, and word[30] `0x02008c60`. The separately captured Linux DRAM window is 1 GiB. If this candidate is the one selected and the SPL normalization produces the matching 1-GiB geometry (15 rows, 10 columns, one rank, 32-bit bus), the matched helper `0x4800c730` computes:

- `word[6] = 0x30fa`: extracted fields sum to 28; subtracting 18 gives shift count 10.
- `word[30] >> 16 = 0x0200`; its low three bits are zero, so the helper takes the unscaled branch.
- Helper result: `1 << 10 = 1024 (0x400)`.
- Caller `0x48011938` places that result in context +0x1c bits 16..31 while preserving the low 16 bits: `0x04000000 | 0x0040 = 0x04000000`.
- The copy worker computes `0x40000000 + (((0x04000000 >> 17) & 0x7fff) << 20) = 0x60000000` for the second 0x140-byte DRAM source window.
- Context +0x78 bit27 is clear for `0x02008c60`; the fallback controller-configuration branch rejoins the normal copy path because context +0x1c bit0 is clear. The mode field at context +0x04 is candidate type 8, not 4, so the alternate strided-copy branch is not selected under this candidate assumption.

This calculation is a source/binary consistency result, not a live value observation. The primary Boot0 block is explicitly documented as mutable training metadata; the active Boot0 copy, selected candidate, native SPL build, and final runtime parameter bytes have not been captured as one continuous execution. The captured 1-GiB Linux map and Boot0 candidate are consistent with the 0x60000000 midpoint, but they do not prove that BL31 used that address during a real suspend/resume.

The next static/host task is to add a regression that joins the exact Boot0 primary report, pinned SPL normalization logic, monitor patch offsets, and BL31 helper arithmetic. Keep both DRAM windows excluded from LK allocations until a live save/restore lifecycle and CPU/DMA/cache ownership proof exists. No device access, DRAM readback, MMIO/SMC/TWI, suspend/wake, storage write, or payload execution occurred.


### 2026-11-28 parameter-flow regression completed (#150)

Added `scripts/audit-rg34xx-bl31-dram-context.py` and `scripts/test-audit-rg34xx-bl31-dram-context.py`, integrated into `.github/workflows/h700-bl31-memory-audit.yml`. The audit pins the exact monitor identity and instruction words, checks the SPL producer/handoff source fragments, parses the captured Boot0 primary candidate, and reproduces BL31 helper arithmetic. Successful CI [#150](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46134616497) produced artifact [15259340938](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/46134616497/artifacts/15259340938), including `BL31-DRAM-CONTEXT.json/.md`.

The emitted conditional result is: LPDDR4 type 8, 672 MHz, 15 rows, 10 columns, 1,024 MiB; helper result `0x400`; rewritten context word[7] remains `0x04000000`; second save/restore source candidate is `[0x60000000,0x60000140)`. The regression deliberately records `live_parameter_block_captured=false`, `suspend_resume_execution_observed=false`, and `memory_layout_approved=false`.

The first CI attempts (#147–#149) exposed two audit-fixture mistakes: an incorrect expected low 16-bit value for word[7], and one mistyped immediate in the pinned instruction list. Both were corrected; #150 passed the unit test and the full report-generation step. These were host-side audit failures, not hardware failures.

This closes the static parameter-flow regression task, not G2's runtime ownership gate. The selected candidate, patched live parameter block, actual standby branch, save/restore completion, and CPU/DMA/cache quiescence remain unobserved. No device access or payload execution occurred.
