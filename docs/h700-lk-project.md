# RG34XX H700 LK project scaffold

## What is in this scaffold

This adds a real GammaOS LK project/target/platform selection path for the
**Anbernic RG34XX non-SP, Allwinner H700, Cortex-A53**:

- `project/rg34xx_h700.mk` selects the H700 build, the `arm-none-eabi-`
  toolchain, and the LK Thumb-2 C/C++ default (startup and exception assembly
  remain AArch32 ARM state).
- `target/rg34xx_h700/rules.mk` selects the H700 platform and refuses to
  continue unless an explicit memory-layout approval and both memory values
  are supplied.
- `platform/h700/rules.mk` selects `ARCH=arm` / `ARM_CPU=cortex-a53` and assigns LK's generated `system-onesegment.ld` as the ELF linker script.
- MediaTek `drvgen`/DWS generation is disabled for this non-MediaTek target.
  The separate six-file H700 overlay also excludes the incompatible MediaTek
  platform/common, platform/PAL, device and target-init sources. The MTK PAL
  timer uses MTK GPT, and its mutex implementation is only a no-op placeholder;
  H700 must provide its own timer, logging and synchronization services.

The project currently enables **no H700 device drivers**. It is a build-system
entry point and a memory-guard test, not a completed LK port.

## Memory approval is deliberately fail-closed

A normal invocation such as:

```sh
make PROJECT=rg34xx_h700
```

must stop in `target/rg34xx_h700/rules.mk` with
`H700_MEMORY_LAYOUT_APPROVED` unset. It must not silently select a RAM address
or emit a firmware image.

A future real build will require all three explicit operator-supplied values:

```sh
H700_MEMORY_LAYOUT_APPROVED=yes
H700_MEMBASE=0x...
H700_MEMSIZE=0x...
```

The ellipses are intentionally **not example addresses**. Do not copy a
candidate address from another board, the Linux boot image, the monitor
reservation, the frame buffer, or the host-only BL33 linker probe. Setting the
approval variable is not a substitute for the separate evidence and review;
it is a gate that makes this decision explicit and auditable.

The make rules validate that the values are hexadecimal and that base and
size are non-zero. They do not prove that a range is physically free, large enough,
aligned, outside secure/DMA carveouts, or compatible with the vendor monitor.

## What the RG34XX evidence actually says

The reports branch contains read-only evidence for this exact non-SP device:

- Live Linux reports DRAM as `[0x40000000, 0x80000000)` (1 GiB).
- The active BaseOS chain uses an AArch32 BL33 loader at `0x4a000000`, within
  a 1 MiB BL33 arena in that boot layout. The same chain places vendor BL31 at
  `0x48000000` and its monitor reservation occupies
  `[0x48000000, 0x49000000)`.
- The captured Linux memblock map also lists reservations at
  `[0x40080000, 0x411b5000)`, `[0x42000000, 0x42001000)`,
  `[0x44000000, 0x44023240)`, the 64 MiB CMA area
  `[0x77c00000, 0x7bc00000)`, the framebuffer
  `[0x7bf24800, 0x7c076000)`, and an **unassigned** 16 MiB region
  `[0x7ec00000, 0x7fc00000)`, plus small top-of-RAM reservations.
- The 16 MiB unknown reservation has not been proven to be DRM/TEE, and the
  absence of a named owner does not make it free. It remains excluded.
- The live Linux memblock snapshot is evidence about the running kernel's
  reservations; it is not, by itself, a complete description of every
  pre-Linux secure-world, monitor, DMA, loader or handoff lifetime.

Primary evidence:
- [Active loader provenance (reports branch)](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-active-loader-provenance.md)
- [RG34XX boot handoff audit](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/docs/rg34xx-boot-handoff-audit.md)
- [Live memblock reservations](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261122-214708/debugfs-memory-reservations.txt)
- [DRM/TEE reservation follow-up](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261122-210352/DRM-SMC-finding.md)

**Important:** `0x4a000000` is proven as the current BaseOS BL33 loader's
location, not yet approved as the address/size for a replacement LK image,
heap, stack or Linkslot payload. The 1 MiB arena is a boot-chain allocation
fact, not a free-memory grant. The host-only BL33 layout check is also not an
allocation approval.

## Handoff and runtime limits

The vendor monitor's `0x8000ff04` service is evidenced as the Linux-start
handoff used by the current chain. Its measured contract targets the Linux
entry and FDT convention; it is **not** a generic public
`jump_to_aarch64(payload)` service. No arbitrary AArch64 payload entry has
been proven on this RG34XX.

The pinned LK scheduler/critical-section/heap model is not yet a validated
SMP runtime for this Cortex-A53 device. The secondary-core state at the actual
BL31-to-BL33 handoff remains unknown. The Newlib adapter therefore remains
gated on an explicit heap-ready and single-active-CPU condition.

No addresses are assigned here for:
- LK image, stack, heap, BSS or scratch areas;
- the Linkslot/mGBA core payload or its load/entry contract;
- MMU page tables, DMA buffers or framebuffer ownership;
- secondary-core parking or A53 coherency state.

No H700 MMIO drivers, clock changes, pinmux writes, PMIC operations, storage
writes, Boot0/TOC1 changes or device flash operations are introduced.

## Validation scope

`scripts/test-h700-lk-project-guard.py` applies the pinned six-file LK
overlay in a disposable worktree and proves that:

1. the project refuses to parse without explicit memory approval;
2. it separately refuses missing base, missing size, malformed hexadecimal
   values, zero base and zero size;
3. with test-only sentinel values, GNU Make can dry-run the linker-script
   target without creating a build directory or firmware file;
4. the platform's `LINKER_SCRIPT` variable resolves to
   `./build-rg34xx_h700/system-onesegment.ld`, the script consumed by LK's
   `make/build.mk` ELF link rule.

The sentinel values are parser fixtures only; they are never written to the
project defaults or treated as device addresses.

Passing this test does not mean the complete LK target links, boots, or is
safe to run on hardware.
