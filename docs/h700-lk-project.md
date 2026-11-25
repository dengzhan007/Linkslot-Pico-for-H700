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
- `target/rg34xx_h700/init.c` supplies the RG34XX target lifecycle hooks. The
  generic `target/init.c` is excluded because it imports MTK-only defaults.
  H700 `platform_init_mmu_mappings()` follows the pinned LK AArch32
  section-table contract: `arm_mmu_init()`, 1 GiB RG34XX DRAM identity map
  as normal write-back memory, 1 MiB PIO/CCU identity map as Device memory,
  then `arch_enable_mmu()`. Only after that does `target_early_init()` turn
  on PI12 before heap/thread startup. `target_init()` retries if early PIO
  access failed.
  Neither hook creates fake fastboot/eMMC or memory-scratch capabilities.
- `platform/h700/rules.mk` selects `ARCH=arm` / `ARM_CPU=cortex-a53` and assigns LK's generated `system-onesegment.ld` as the ELF linker script.
- The H700 platform rules list the policy-only CPU selector/service/runtime owner,
  the RG34XX PI12 LED C service and the early PIO MMU mapping object. CPU
  performance units remain policy-only; the LED service is the only current
  board GPIO MMIO path and does not touch PMIC/CCU clocks or CPU PLL registers.
- MediaTek `drvgen`/DWS generation is disabled for this non-MediaTek target.
  The separate six-file H700 overlay also excludes the incompatible MediaTek
  platform/common, platform/PAL, device and target-init sources. The MTK PAL
  timer uses MTK GPT, and its mutex implementation is only a no-op placeholder;
  H700 must provide its own timer, logging and synchronization services.

The project currently enables one minimal RG34XX board function: the PI12 work/power LED. It is not a general GPIO driver or input scanner. CPU performance objects remain policy-only; CPU DVFS, PMIC, MMC/SD hardware init and other H700 device drivers are not activated. The early MMU hook sets address-translation attributes only; the 1 GiB DRAM mapping does not allocate memory or approve any LK image, heap, stack, BSS, page-table, framebuffer, DMA or payload placement. This remains a scaffold, not a completed or bootable LK port.

## AArch32 early MMU mapping contract

The H700 target uses the pinned GammaOS LK AArch32 section-table implementation
(non-`MTK_LM_MODE`, non-`MTK_3LEVEL_PAGETABLE`). The upstream LK sequence in
`arch/arm/arch.c::arch_early_init()` calls
`platform_init_mmu_mappings()` before `target_early_init()`; the reference
MTK platform hook initializes the table and TTBR, maps DRAM, then enables the
MMU. The RG34XX implementation now follows that order:

1. `arm_mmu_init()` initializes the 4096-entry identity section table and TTBR.
2. The observed RG34XX DRAM range `[0x40000000, 0x80000000)` is remapped as
   normal write-back, read/write memory (1024 one-MiB sections).
3. `[0x03000000, 0x03100000)` is remapped as shared Device, read/write,
   execute-never memory for H700 CCU/PIO registers.
4. `arch_enable_mmu()` enables translation only after all descriptors are set.
5. LK then proceeds to cache setup and `target_early_init()`, where PI12 is
   configured and turned on.

`DRAM_PHY_ADDR=0x40000000` is supplied to LK's architecture code because
`arm_mmu_init()` uses that constant when building its initial section map.
This is not the same as `H700_MEMBASE/H700_MEMSIZE`: it does not select or
approve the replacement LK load address or grant memory ownership. All
secure/monitor, framebuffer, CMA, unknown-reserved and current-loader
lifetimes remain separate memory-safety gates.

A fake-MMU host regression verifies the call order and descriptor attributes:
one table initialization, 1024 DRAM sections, one PIO Device section, then
one MMU enable. H700 AArch64, AArch32 ARM and AArch32 Thumb-2 object builds
also compile this hook. Neither proves a full LK target link or a hardware boot.
See `platform/h700/arch/h700_pio_mmu_mapping.c` and
`scripts/test-h700-pio-mmu-mapping.sh`.

## RG34XX PI12 power LED startup

The RG34XX non-SP vendor boot DTS in the pinned [KNULLI RG34XX boot package](https://github.com/knulli-cfw/knulli-linux/blob/57afa662009641d181f57241bcc480119b131530/package/boot/uboot-h700/rg34xx/boot_package/rg34xx.dts) maps `work_led` to PIO bank I / PI12 and `low_pwr_led` to PI11. [MustardOS RG34XX-H DTS](https://github.com/MustardOS/internal/blob/aef38ed9a870419a0fe4f351dac1bac58a84ba5e/device/rg34xx-h/package/sunxi.dts) has the same mapping. PI11 remains untouched. The boot DTS marks UART3 disabled; the active boot console is UART0.

The driver uses PIO base `0x0300b000`, bank stride `0x24`, PI12 CFG1/MUX1 offset `0x124` (function 1 / GPIO output) and DATA offset `0x130` bit 12. It preloads the active-high on-level before pinmux transition and reads both fields back. Mainline H700 RG35XX-2024 configuration is only sibling-board polarity corroboration; exact RG34XX electrical polarity and power-key release timing still require a cold-boot test. See [power LED lifecycle](h700-power-led-lifecycle.md).

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

## BL33 memory lifetime audit

The source-backed runtime inventory is documented in
[H700 BL33 and application memory ownership](h700-bl33-memory-ownership.md).
It records two additional facts from the pinned BaseOS loader: the current
A32 entry sets SP to 0x4a0f0000, and its 4096-entry (16 KiB) translation table
starts at 0x4a0f8000. These narrow the current BL33 layout, but do not approve
an LK heap, BSS, stack guard, MMU table or application allocation. The
0x40080000 kernel destination and 0x45000000 BOL4 staging window are only
candidate reclaim areas for a standalone LK path that bypasses Linux; neither
is approved for use.

## BL33 image packaging

The full GammaOS build's normal `lk.img` is MTK-header wrapped. A future H700
BL33 image must use `lk-no-mtk-header.bin` and the verified RG34XX reference
loader header, not the standard `lk.img`. The host wrapper and remaining gates
are documented in [H700 BL33 image packaging](h700-bl33-image-format.md).
The output remains non-installable until the complete LK target link, memory
approval, matching native firmware pairing, and live BL31 handoff are proven.

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

The only introduced MMIO path is the narrowly scoped PI12 work LED: one
1 MiB peripheral Device mapping plus PI12 pinmux/data read-modify-write. It
does not alter CCU/APB1 rates or gates, PMIC registers, CPU PLL/OPP state,
storage contents, Boot0/TOC1 or device flash.

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
5. `platform/h700/rules.mk` contains the policy, service and runtime-owner object
   entries and each entry has a corresponding C source file.
6. `target/rg34xx_h700/rules.mk` links its target lifecycle object and the
   source defines exactly one `target_early_init()` and `target_init()`.

The source/build-graph assertions and MMU/PIO fake-register tests do not
execute the full LK target link or prove the object set fits an approved
RG34XX memory map. They verify software call order and descriptor values,
not the BL33 handoff state, physical LED polarity, APB1/PIO gate state or
boot-time behavior on the handheld.

The sentinel values are parser fixtures only; they are never written to the
project defaults or treated as device addresses.

Passing this test does not mean the complete LK target links, boots, or is
safe to run on hardware.
