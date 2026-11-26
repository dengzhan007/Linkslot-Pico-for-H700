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

## LK heap readiness bridge and startup contract

The heap readiness bridge is part of the H700 platform object list because
target_init() references its lifecycle marker. Its source contract is pinned
to GammaOS LK commit 9bf1419a49e0af6dba8b05916df382c9c9c6d0a8:

- The normal kernel/main.c path calls target_early_init() before heap_init(),
  then initializes threads/DPC/timers, calls platform_init(), then target_init(),
  and only afterward calls apps_init().
- lib/heap/heap.c::heap_get_info() walks the free list inside LK critical
  section enter/exit calls. heap_init() initializes that list before inserting
  the initial free chunk.
- h700_platform_heap_ready() does not query LK before the post-heap marker;
  afterward it reports ready only while both total free bytes and the largest
  free chunk are non-zero. This is a live availability predicate, not a memory
  budget, allocation guarantee or approval of any physical range.
- The alternate ENABLE_NANDWRITE bootstrap calls target_init() without
  heap_init(). The H700 hook therefore does not mark the heap ready for that
  build path; normal and NAND-write host regression cases cover the distinction.
- scripts/test-h700-lk-project-guard.py checks the pinned startup ordering,
  heap API and critical-section behavior, plus the actual H700 OBJS entry.

This bridge does not initialize the heap, select its address/size, install the
newlib adapter, or authorize the Linkslot runtime coordinator. The full LK
target link and memory-layout gate remain closed.

## Read-only Generic Timer time service

`platform/h700/runtime/h700_arch_timer.c` supplies ABI v1 monotonic time from
the ARM Generic Timer virtual counter and firmware-programmed frequency. It
uses AArch32 `MRRC p15,1` / `MRC p15,0` or AArch64
`CNTVCT_EL0` / `CNTFRQ_EL0`, reads only, and starts a relative epoch when
the service is initialized. A zero `CNTFRQ` rejects initialization.

The implementation does not configure counter access controls, CNTVOFF,
timer compare/control registers, GIC/PPI, or periodic interrupts. It does not
implement LK `platform_set_periodic_timer()` and is not called from
`target_init()` or the Linkslot coordinator. Host tests cover conversion,
counter wrap, zero-frequency refusal and descriptor lifetime; AArch32,
Thumb-2 and AArch64 compile checks cover instruction syntax only. Actual
CNTFRQ and counter growth still require device execution.

## LK kernel periodic-timer contract — unresolved boot prerequisite

This is a source audit of the immutable GammaOS LK pin
[`9bf1419a49e0af6dba8b05916df382c9c9c6d0a8`](https://github.com/TheGammaSqueeze/lk-mt6785/tree/9bf1419a49e0af6dba8b05916df382c9c9c6d0a8):

- `kernel/main.c` calls `target_early_init()`, then `heap_init()`,
  `thread_init()`, `dpc_init()`, and `timer_init()`. Only after the
  bootstrap thread starts does LK call `platform_init()`, `target_init()`,
  and `apps_init()`.
- `kernel/timer.c::timer_init()` unconditionally calls
  `platform_set_periodic_timer(timer_tick, NULL, 10)` on the normal build
  (some specific Konkr builds use 1 ms). The timer subsystem also calls
  `current_time()` when scheduling software timers. The scheduler therefore
  requires a real H700 timer backend before `platform_init()` and
  `target_init()`; the Linkslot runtime coordinator is too late.
- The pinned platform contract declares `platform_set_periodic_timer()`,
  `mdelay()`, `udelay()`, and `platform_tick_rate()` in
  `include/platform/timer.h`, and `current_time()` /
  `current_time_hires()` in `include/platform.h`. The H700 read-only
  `h700_arch_timer.c` currently exports an ABI descriptor only; it does
  not implement the LK platform hooks or a periodic scheduler tick.
- The pinned MT6785 implementation in `platform/mt6785/timer.c` is based
  on GPT5 and MediaTek IRQ/GIC programming. It is not an H700 implementation
  and must not be reused. The H700 overlay intentionally omits MTK
  platform/PAL objects.
- The H700 heap readiness marker runs from `target_init()`, which is later
  than `timer_init()`; it cannot be used to qualify the first kernel tick.
- This is an unresolved full-link/boot prerequisite. The current build
  feasibility workflow compiles selected overlays and probes; it has not
  linked a complete H700 LK target. Do not satisfy the missing timer contract
  with a no-op success stub.

The early phase is an additional prerequisite, not just a detail of
the later scheduler callback:

- `kernel/main.c` calls `platform_early_init()` after
  `arch_early_init()`, but before `target_early_init()`, `heap_init()`,
  and `timer_init()`.
- The pinned generic `platform/init.c` implementation of
  `platform_early_init()` is an empty weak fallback. The MT6785 reference
  overrides it and calls `platform_init_interrupts()` before
  `platform_early_init_timer()`.
- The MT6785 early delay timebase is GPT4; its scheduler tick later uses GPT5
  and the MTK IRQ path. Its `dev/gic/mt_gic_v2.c` driver uses MTK-specific
  register bases and IRQ helpers. The pinned `dev/gic/rules.mk` selects it
  via `MTK_GIC_VERSION`, and the H700 overlay excludes the `gic` module.
- H700 currently has neither a strong `platform_early_init()` override nor
  an H700 GIC/timer provider. The empty generic fallback cannot initialize
  the H700 interrupt controller or timebase.
- The archived Linux interrupt snapshot reports `GIC-0 27 arch_timer` and
  `wakeupgen 48 sun4i_timer0`; these are Linux runtime mappings, not BL33
  PPI ownership. The checked-in FDT summaries omit interrupt-controller
  nodes, and the raw live DTB was not uploaded. The GIC compatible, register
  ranges, interrupt-cell layout and timer PPI therefore remain unconfirmed
  from repository evidence.

CI now runs `scripts/audit-h700-lk-timer-prerequisites.py` against the
immutable LK pin. This prevents drift in the audited upstream contract; it
does not prove the H700 hardware handoff or implement the missing provider.

Before implementing an H700 timer backend, obtain device evidence for the
BL31→BL33 Generic Timer access state (including EL2 counter traps/offset and
`CNTFRQ/CNTVCT` readback), interrupt-controller type/register layout and distributor/CPU-interface
handoff (if GICv2), and the correct architected timer PPI/ownership. Linux IRQ 27 activity is evidence
for the Linux runtime only; it does not authorize enabling the same interrupt
from BL33. After those gates close, implement and test the LK platform time,
periodic timer, and delay contracts as a coherent provider. Until then, keep
the read-only Generic Timer service unregistered and do not program timer
compare/control, GIC/PPI, or IRQ state.

## Live FDT interrupt-controller and timer inventory

The read-only FDT inspector now emits `interrupt_controllers` and
`timer_nodes`. It preserves compatible/status, phandle, register ranges
decoded using the parent bus cell counts, `#interrupt-cells`, inherited
`interrupt-parent`, raw `interrupts` specifiers grouped by the provider's
cell count, and `interrupts-extended` references.

The parser deliberately does not translate these cells into Linux virtual
IRQ numbers or assert SPI/PPI meaning. This is static active-DTB declaration
evidence only, not proof that a firmware stage enabled or handed an interrupt
to BL33. The synthetic FDT tests cover GIC register ranges and phandle
resolution, inherited interrupt-parent, timer specifiers and extended
references. Linux/Windows host-tool tests passed in H700 boot-audit
`#45772025184`; AArch64/AArch32 compile and storage regressions passed in
`#45772025265`.

The previous archived inventory contains `live-fdt.json` only; it did not
retain `live-fdt.dtb`. Consequently, the new fields still need to be
generated from a fresh RG34XX read-only inventory. The collector already
copies `/sys/firmware/fdt` when the kernel exports it. Re-run the inventory,
retain the raw DTB and JSON side by side, and inspect the actual controller
compatible/reg/cell declarations before designing the H700 GIC provider.

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


The pinned LK implementation stores `tt[4096]` as a static, 16-KiB-aligned
section table in BSS. That table is part of LK's linked image footprint; its
address and the rest of BSS/stack/heap still require the explicit memory
ownership review. The mapping function does not allocate a replacement table
or dynamic DRAM memory.

## RG34XX PI12 power LED startup

The RG34XX non-SP vendor boot DTS in the pinned [KNULLI RG34XX boot package](https://github.com/knulli-cfw/knulli-linux/blob/57afa662009641d181f57241bcc480119b131530/package/boot/uboot-h700/rg34xx/boot_package/rg34xx.dts) maps `work_led` to PIO bank I / PI12 and `low_pwr_led` to PI11. [MustardOS RG34XX-H DTS](https://github.com/MustardOS/internal/blob/aef38ed9a870419a0fe4f351dac1bac58a84ba5e/device/rg34xx-h/package/sunxi.dts) has the same mapping. PI11 remains untouched. The boot DTS marks UART3 disabled; the active boot console is UART0.

The driver uses PIO base `0x0300b000`, bank stride `0x24`, PI12 CFG1/MUX1 offset `0x124` (function 1 / GPIO output) and DATA offset `0x130` bit 12. It preloads the active-high on-level before pinmux transition and reads both fields back. Mainline H700 RG35XX-2024 configuration is only sibling-board polarity corroboration; exact RG34XX electrical polarity and power-key release timing still require a cold-boot test. See [power LED lifecycle](h700-power-led-lifecycle.md).



### PIO bus clock boundary

The H616-family reference PIO node binds to `CLK_APB1` and has no separate
PIO reset property; the reference CCU models APB1 as a mux/divider clock, not
as a PIO-specific gate. See
[`sun50i-h616.dtsi`](https://github.com/torvalds/linux/blob/master/arch/arm64/boot/dts/allwinner/sun50i-h616.dtsi)
and
[`ccu-sun50i-h616.c`](https://github.com/torvalds/linux/blob/master/drivers/clk/sunxi-ng/ccu-sun50i-h616.c).
RG34XX v2 Linux runtime reports APB1 at 100 MHz with a PIO consumer. This is
Linux runtime/reference-family evidence only; BL33-entry APB1 parent/rate is
not directly read. The PI12 service does not modify CCU/APB1.

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

### Pinned LK header/API cross-check

The MMU hook includes the actual pinned LK headers
[`arch/arm/arm/mmu.h`](https://github.com/TheGammaSqueeze/lk-mt6785/blob/9bf1419a49e0af6dba8b05916df382c9c9c6d0a8/arch/arm/include/arch/arm/mmu.h)
and
[`include/arch/ops.h`](https://github.com/TheGammaSqueeze/lk-mt6785/blob/9bf1419a49e0af6dba8b05916df382c9c9c6d0a8/include/arch/ops.h):
the pinned source declares `arm_mmu_init()`, `arm_mmu_map_section()` and
`arch_enable_mmu()`, and provides the exact normal-write-back, shared-Device,
read/write and XN attribute constants used by the board mapping. The current
cross-compile script compiles this object against freestanding test headers;
the LK project guard separately verifies the source call/attribute contract.
A full LK target link against this board object is still blocked on the
independent approved image/BSS/stack/heap memory layout.

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
