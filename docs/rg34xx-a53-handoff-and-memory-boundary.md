# RG34XX H700 A53 handoff and memory boundary

**Target:** Anbernic RG34XX non-SP, Allwinner H700, four Cortex-A53 CPUs  
**Status:** read-only evidence synthesis; no handoff contract or payload memory range is approved.

## 1. Facts established by the target-device reports

### CPU and BL31/BL33 chain

- The live RG34XX inventory identifies four CPUs, architecture 8, CPU part 0xd03 (Cortex-A53). This is a Linux-visible topology observation; it does not prove which PEs are powered, released, or parked at the earlier BL31→BL33 handoff.
- The stock vendor monitor in the current device TOC1 has the same SHA-256 as the monitor pinned by the BaseOS RG34XX evidence. The binary contains Allwinner sun50iw9p1 / H616 platform strings, BL31 context-management paths, and strings naming PSCI CPU_ON, CPU_OFF, and CPU_SUSPEND handlers.
- Those strings prove that code paths/names exist in the binary, not that the corresponding service is registered, working, or invoked on this boot. The handoff report also records the literal diagnostic string "WARNING: Unimplemented PSCI Call"; it must not be treated as a captured runtime log.
- The measured BaseOS native chain is: AArch64 EL3 SPL → vendor BL31 at 0x48000000 → AArch32 SVC BL33 at 0x4a000000, with SPSR=0x1d3. The native SPL prepares the monitor header/DRAM parameters, programs RVBAR=0x48000000, and requests an EL3 reset through RMR_EL3; BL31 then returns to the A32 loader.
- The live secondary-CPU reset/hold state at BL31→BL33 is still unverified. A matched Boot0/SPL/BL31 static audit makes CPU1–CPU3 hold-off probable, not proven: Boot0's helper only targets CPU2/CPU3 and only clears their reset bits (an assert-reset operation under H616 reference semantics); CPU1 is not passed to that helper; the H616 family reset default asserts CPU1–CPU3; pinned native SPL writes only the primary RVBAR pair; and the matched BL31 direct-MMIO scan recovered CPU0 mode setup but no direct secondary reset/RVBAR writes. H700 register equivalence, the helper's SID bit-29 condition, and live C0_RST_CTRL/RVBAR values remain unknown. See [Boot0 CPU reset static audit](../scripts/audit-rg34xx-boot0-cpu-reset.py) and [H700 BL31 audit workflow](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/workflows/h700-bl31-memory-audit.yml).
- The matched BL31 also contains a legacy PSCI platform-ops table whose affinst_on slot reaches a CPU_ON helper. This proves a later CPU_ON-capable path exists in the binary, not that the monitor releases CPU1–CPU3 automatically at initial BL33 handoff or that the path is available to the non-secure caller.

### Runtime DRAM and reservations

The target's live FDT and /proc/iomem agree on 1 GiB DRAM:

\[
[0x40000000,\;0x80000000)
\]

The current BaseOS native loader derives this 1 GiB size from the trained Boot0 DRAM block and writes the live FDT. The on-disk TOC1 DTB's 512 MiB default must not be used as the live capacity.

The captured Linux memblock snapshot reports these ranges:

| Range | Observed size | Evidence / interpretation |
|---|---:|---|
| [0x40000000, 0x80000000) | 1 GiB | Linux-visible memory window |
| [0x40080000, 0x411b5000) | 17,620 KiB | reserved by the running kernel |
| [0x411b6000, 0x411b8000) | 8 KiB | reserved |
| [0x42000000, 0x42001000) | 4 KiB | initrd vicinity; active loader records a 512-byte initrd at 0x42000000 |
| [0x44000000, 0x44023240) | 140 KiB | FDT vicinity; live/TOC FDT is staged at 0x44000000 in the recorded boot chain |
| [0x48000000, 0x49000000) | 16 MiB | vendor BL31 / monitor reservation |
| [0x77c00000, 0x7bc00000) | 64 MiB | reserved CMA window in the memblock snapshot |
| [0x7bf24800, 0x7c076000) | 1,382,400 bytes | display framebuffer from boot_fb0 / disp_reserve |
| [0x7ec00000, 0x7fc00000) | 16 MiB | reserved, owner not established |
| [0x7fd4a000, 0x7fd4b000) | 4 KiB | reserved |
| [0x7fd65000, 0x7ff49000) | 1,936 KiB | reserved |
| additional small ranges near 0x7ff49a00..0x7fffffff | several KiB | reserved firmware/kernel fragments |

The framebuffer range is independently derived from the live boot_fb0 node (720×480, 32 bpp, stride 2880) and matches the disp_reserve=1382400,0x7bf24800 boot argument.

The active BaseOS chain places its A32 BL33 loader at 0x4a000000 in a 1 MiB boot-chain arena; the current loader's SP is 0x4a0f0000, and its 16 KiB translation table begins at 0x4a0f8000. These are observed BaseOS allocation/lifetime facts, not grants to reuse that arena for a replacement LK image, heap, stack, BSS, or Linkslot core.

## 1.1 Boot0 / BL31 secondary-core static evidence (2026-11-28)

The pinned read-only static audit inspects the captured 128 KiB primary Boot0
(SHA-256 bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f),
the exact pinned native SPL source, and the matched BL31 CPU/MMIO scan.

- Boot0's reset helper at 0x231e0 is called for core IDs 2 and 3 only when
  bit 29 of 0x03006248 is set. The helper clears DBG_REG1.C_DBGPWRUPREQ[core],
  ORs a one-hot mask into 0x09010044, then stores C0_RST_CTRL & ~(1 << core).
  Under the H616 reference convention (zero asserts reset), that final
  operation asserts / keeps the selected core in reset; it never deasserts
  the reset bit. CPU1 is not passed to the helper.
- H616 User Manual v1.0 gives C0_RST_CTRL reset default 0x13ff0101: CPU0 bit
  0 is deasserted, CPU1–CPU3 bits 1–3 are asserted. This is a same-family
  reference only; H700 equivalence and the live H700 value are not established.
- The 0x03006248 bit-29 field is not defined in the public H616 register
  table; its H700 meaning/value is unknown. The 0x09010044 write is also
  unresolved because the H616 manual names it CPU0 RVBARADDR0_H, while this
  helper ORs CPU2/CPU3 one-hot masks into it. Do not modify either register
  based on this inference.
- Pinned native SPL writes the primary RVBAR low/high pair only and does not
  directly write per-core reset or secondary RVBAR offsets during its handoff.
- The matched BL31 contains a legacy nine-entry `plat_pm_ops_t` table at
  `0x480165f8`; slot 1 (`affinst_on`) is dispatched through `0x4800255c`
  to CPU_ON helper `0x48002d78` when its CPU_ON context argument is zero.
  The helper calculates per-core addresses, programs a per-core RVBAR and
  performs CPU power/reset sequencing. One SoC variant uses CPUCFG/PRCM
  addresses `0x09010000`, `0x09010010`, `0x07000440`, `0x07000444` and
  `0x090100c0`; another branch references the `0x08100000` region, including
  `0x08100020`. The selected branch depends on `VER_REG 0x03000024 & 7`.
- The 2026-11-26 selector read failed with `EFAULT`, so the active CPU_ON
  variant is unknown. The whole-image fixed-address scanner's empty direct
  reset list does not cover all helper-generated per-core addresses; it must
  not be read as proof that BL31 has no secondary-core control logic.
- This is a PSCI callback path for a later CPU_ON request. No evidence shows
  it is called automatically during the initial BL31→BL33 handoff, nor has
  non-secure availability/permissions been verified. Linux later showing
  CPU0–CPU3 online is consistent with later CPU_ON calls but does not prove
  when or how each secondary core was released.
- Assessment: PROBABLE_NOT_DEVICE_VERIFIED. ROM/secure/indirect paths and
  actual register state at BL33 entry remain outside the static proof.

The 2026-11-26 read-only RVBAR-selector probe did not read back the selector.
It tried only a bounded 32-bit read of /dev/mem at 0x03000024 and failed with
EFAULT (zero bytes; no JSON report). A separate dd attempt at the same
address also returned Bad address. No register was read, no protection was
bypassed, and no write was attempted. The 2026-11-27 Linux inventory showing
CPU0–CPU3 online is a post-kernel observation and cannot reconstruct the
earlier BL31→BL33 reset state.

Do not repeat userspace /dev/mem probing on this build. The next valid
evidence path is either (a) an early Boot0/BL31 trace/capture of CPUCFG reset
and RVBAR state using a reviewed read-only path, or (b) a read-only kernel
driver path through an already-mapped resource with explicit bounds and
permission review. Until then, keep the LK runtime single-core-only and do
not invoke CPU_ON or release secondary cores.

## 1.2 PSCI CPU_ON is a separate later control path

The separate `PSCI-CPU-ON.md` / `PSCI-CALLBACK-DISASSEMBLY.txt` output in
BL31 static audit #134 shows a real platform `affinst_on` callback and a
low-level helper that can program per-core RVBAR and execute power/reset
operations. Its SoC-specific branch depends on the `VER_REG` low-three-bit
selector at `0x03000024`; `/dev/mem` could not read this selector. This
callback establishes the existence of a secondary-core start mechanism,
not an automatic action at the initial BL31→BL33 transition. The direct
fixed-address whole-image MMIO scanner does not model all calculated
per-core addresses used by this helper.

Do not invoke this path from LK/Linkslot for M1. Keep the runtime CPU0-only
until initial hold-off and non-secure PSCI semantics are independently
verified, and until an SMP-capable runtime is used. The unresolved active
CPU_ON variant and reset/RVBAR values remain hardware gates.
## 2. What remains unknown

1. Whether CPUs 1–3 are still held in reset, parked in a vendor holding pen, executing secure firmware, or released when BL31 enters the current A32 BL33.
2. Whether PSCI AFFINITY_INFO and CPU_ON are implemented and available to non-secure BL33, and what entry/context contract they require.
3. Whether RMR_EL3 reset handling in the current native SPL path implies a single-PE or broader reset sequence on this exact board. Do not infer secondary state from the name of the register or from the single observed BL33 entry.
4. The complete pre-Linux secure-world, monitor, DMA, CMA, loader and framebuffer ownership map, including lifetimes and cacheability/shareability attributes. Linux memblock is a post-handoff snapshot, not the complete BL33 allocation map.
5. The owner and purpose of [0x7ec00000, 0x7fc00000). Its presence in memblock.reserved excludes it from an allocation until proven otherwise; the absence of a named /reserved-memory node does not make it free.

## 3. Required runtime policy until those facts are proven

- Treat the initial H700 LK runtime as **single-core only**.
- Do not release, wake, or schedule work on secondary A53s. Do not add PSCI or Allwinner CPU_ON calls based solely on symbol/string evidence.
- The LK scheduler currently uses global thread/run-queue and critical-section state with local IRQ masking. It is not an SMP scheduler. The Newlib adapter's single_cpu_active() gate must remain false until a board startup path can positively establish the required single-core invariant.
- Keep the project H700_MEMORY_LAYOUT_APPROVED gate closed. Do not set H700_MEMBASE/H700_MEMSIZE for a device build from the current Linux map or from the 1 MiB BaseOS BL33 arena.
- Do not infer that the complement of the memblock reserved ranges is available to LK. A free-at-Linux-runtime interval may have had an earlier SPL/monitor/loader owner or may require special cache/MMU treatment.
- Host-only synthetic header/link probes and CI artifacts are not firmware; do not write them to SD, Boot0, TOC1, or the device.

## 4. Evidence required before changing the policy

- Capture a boot-stage UART trace or a source/firmware-backed BL31 trace that identifies the primary PE and the state of secondary PEs at the actual BL33 handoff.
- Establish whether the non-secure BL33 can query PSCI version/affinity and whether a documented secondary-entry contract exists. A monitor binary string is insufficient.
- Reconcile the live FDT, memblock.reserved, CMA base/size, /proc/iomem, framebuffer ownership, monitor/TEE reservations, and BaseOS SPL/BL33 staging lifetimes into one interval table with explicit owner and release point.
- Only after that review should LK heap, stack, BSS, page tables, framebuffer, and emulator-core memory be assigned.

## 5. Primary evidence

- [RG34XX live inventory and memory map](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/docs/rg34xx-live-inventory-2026-11-22.md)
- [Active loader provenance and measured BL31/BL33 path](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-active-loader-provenance.md)
- [Bootloader/monitor handoff binary audit](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-bootloader-handoff-followup.md)
- [Captured memblock reservations](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/debugfs-memory-reservations.txt)
- [H700 LK project fail-closed memory gate](h700-lk-project.md)
