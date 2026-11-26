# RG34XX / H700 single-core startup static audit

Date: 2026-11-26

Status: **STATIC EVIDENCE SUPPORTS CPU0-ONLY HANDOFF; NOT YET DEVICE-VERIFIED.**
This is a host-only review of the exact uploaded Boot0 image, pinned BaseOS SPL
source, matched BL31 monitor payload, and the H616 register manual as a family
reference. It is not a live CPUCFG readback and does not authorize executing a
new BL33 payload.

## 1. Inputs and reproducibility

Primary Boot0 capture:

- File: `reports/rg34xx-inventory-20261126-timer/boot0-primary.bin`
- Size: 131,072 bytes
- SHA-256: `bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f`
- eGON checksum: valid
- Load address used for disassembly: `0x00020000`

The file named `boot0-secondary.bin` is a second Boot0 image copy/slot, not
CPU1 firmware. The two 128 KiB captures should not be confused with per-core
firmware images.

The exact static audit is automated by:

- `scripts/audit-rg34xx-boot0-cpu-reset.py`
- `scripts/audit-rg34xx-bl31-memory.py`
- `scripts/test-audit-rg34xx-bl31-memory.py`

CI: [#45808401081](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45808401081)
(commit `fc5901ee49b502eab298a52b7e38add8f9611a66`), all steps passed.
Artifacts:
- Boot0 source capture: [15109809483](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45808401081/artifacts/15109809483)
- BL31 / Boot0 audit report and reachable disassembly:
  [15110218249](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45808401081/artifacts/15110218249)

## 2. Boot0 Thumb routine

Boot0 is loaded at `0x20000`. The relevant routine and caller are:

- Reset helper: PC `0x231e0`, file offset `0x31e0`
- Caller / feature gate: PC `0x23224`
- Gate register: `0x03006248`, inside the H616 SID peripheral address window
  `[0x03006000,0x03007000)` at offset `0x248`.
- Gate instruction sequence: load word, `LSLS r3, r3, #2`, then `BPL`.

The shift moves original bit 29 into N. If bit 29 is clear, the branch skips
the two helper calls. If bit 29 is set, the caller invokes the helper with
`r0=2` and `r0=3`. The helper is not called with `r0=1`. The public H616
manual register tables do not define the word at SID offset `0x248`, so its
bit29 meaning is unknown. The audit does not dump or interpret the full SID
block; a future pre-BL33 device snapshot should expose only this single masked
bit (`(readl(0x03006248) >> 29) & 1`) to avoid collecting unrelated chip data.

The helper computes the one-hot mask `1 << r0` and performs these operations:

1. Read/modify/write `0x090100c4`: clear the selected bit.
2. Read/modify/write `0x09010044`: OR the selected one-hot bit into the word.
3. Read/modify/write `0x09010000`: write `value & ~(1 << r0)`.

For the H616 reference semantics of `C0_RST_CTRL[3:0]`, zero asserts a core reset and one deasserts it. Therefore step 3 **clears** the selected core reset bit; it does not release the core. If the register still has the H616 power-on default low nibble `0x1`, CPU1-CPU3 remain reset and CPU0 is released. The Boot0 code does not establish the live pre-helper value of the register, so this remains a conditional inference, not a measured result.

## 3. Register-reference limits and anomalies

The H616 User Manual v1.0 describes the cluster register block at
`C0_CPUX_CFG = 0x09010000`:

| Address | H616 register name |
|---|---|
| `0x09010000` | `C0_RST_CTRL` |
| `0x09010010` | `C0_CTRL_REG0` |
| `0x09010040` / `+0x44` | CPU0 RVBAR low / high |
| `0x09010048` / `+0x4c` | CPU1 RVBAR low / high |
| `0x09010050` / `+0x54` | CPU2 RVBAR low / high |
| `0x09010058` / `+0x5c` | CPU3 RVBAR low / high |
| `0x09010080` | `C0_CPU_STATUS` |
| `0x090100c0` / `+0xc4` | `DBG_REG0` / `DBG_REG1` |

Two target-specific discrepancies must remain open:

- Boot0 ORs the core2/core3 mask into `0x09010044`. In the H616 manual
  this is `RVBARADDR0_H`, the **CPU0** reset-vector high word; CPU2 and CPU3
  have separate high registers at `0x09010054` and `0x0901005c`. Do not
  interpret the write to `0x09010044` as per-core RVBAR programming. Its
  purpose on the RG34XX H700 has not been identified, and it must not be
  modified based on the H616 register naming alone.
- The helper writes `DBG_REG1.C_DBGPWRUPREQ[3:0]`, but the H616 manual marks
  those bits read-only. The H700 behavior may differ; the effective result of
  this write is not established.

The same H616 reference gives `C0_RST_CTRL` reset default
`0x13ff0101`: low bits [3:0] are `0001b`; zero asserts reset and one
deasserts reset. This is useful family-level evidence, not proof that the H700
register default or the device's live value is identical.

Reference: H616 User Manual v1.0, §3.1 (SID window) and
§3.2.4–3.2.5.17, pp.30, 35–41:
https://linux-sunxi.org/images/1/1d/H616_User_Manual_V1.0.pdf

## 4. Native SPL and BL31 cross-check

Pinned BaseOS source:
`dengzhan007/BaseOS-MultiFrontend` commit
`cbe253a43f46cf551cc9a6c79a84015bde3686cf`.

The native SPL handoff selects and writes one primary RVBAR low/high pair:

- `writel(0x48000000, (void *)rvbar)`
- `writel(0, (void *)(rvbar + 4))`

The selected base is `0x09010040` for the H616 branch, or the alternate
`0x08100040` branch. The reviewed handoff function contains no direct write
to `C0_RST_CTRL` and no writes to the other per-core RVBAR offsets.

The matched BL31 control-flow audit recovers these CPU-related MMIO writes:

- `0x09010010` (`C0_CTRL_REG0`) at `0x48002fb4`
- `0x08100020` (CPU subsystem control) at `0x48002fc8`

It does not recover direct references to `C0_RST_CTRL`, `DBG_REG1`, or
CPU1-CPU3 RVBAR address registers. The `C0_CTRL_REG0` path sets the CPU0
AArch64 mode bit when its board-selection branch is taken. This is bounded
static evidence only; indirect address calculation, secure firmware services,
other exception paths and runtime CPU-control behavior are not fully modeled.


## 5. BL31 PSCI CPU_ON callback (confirmed in the matched binary)

A separate exact-binary audit now follows the platform operation table and its
indirect dispatch, rather than relying only on the instructions reachable by
direct branches from BL31's first entry.

- The platform ops table is at `0x480165f8`. Its nine 64-bit entries are:
  `0x48002510, 0x4800254c, 0x4800264c, 0x48002878, 0x48002680,
  0x480026c4, 0x480025a0, 0x48002574, 0x48002544`.
- Setup code at `0x480029bc` stores this table pointer through its output
  pointer; the registration callsite is `0x48009dec`.
- The global string `psci_plat_pm_ops` is present. Dispatch code loads the
  global pointer at `0x4801bfc0`, loads offset `+8` (the second function
  pointer), moves it to `x5`, and branches through `x5` at
  `0x48008640`. A second path repeats the slot selection at
  `0x480086a0..0x480086dc`.
- In the standard TF-A `plat_psci_ops_t` layout, the second member is
  `pwr_domain_on`. The selected callback is `0x4800254c`; its BL at
  `0x4800255c` reaches `0x48002d78`.
- Table slot +56, the standard `pwr_domain_on_finish` member, is also
  non-null at `0x48002574`. Standard TF-A PSCI setup advertises
  `PSCI_CPU_ON_AARCH64`, FID `0xc4000003`, when both power-on callbacks
  are non-null. The matched table satisfies that condition, subject to
  the exact vendor implementation retaining the standard TF-A setup rule.
- The low-level routine receives a target core index in `W0` and a
  non-secure entry address in `X1`. It programs a per-core RVBAR, then
  executes a SoC-selected power/reset sequence. One branch accesses
  `C0_RST_CTRL=0x09010000`, `C0_CTRL_REG0=0x09010010`,
  `AON_POWERON_RESET=0x07000440`, `AON_CLAMP=0x07000444` and
  `DBG_REG0=0x090100c0`.
- The selector register `0x03000024` is SYS_CFG VER_REG (H616 family
  reference), **not** the SID window. The H616 manual lists SYS_CFG base
  `0x03000000` and VER_REG offset `0x24` (page 133). Its public field
  table documents bits [13:8] but leaves [7:0] undefined (page 134); the
  firmware's low-bit die selector is therefore not described by that manual.
- BL31 reads VER_REG and tests `VER_REG & 7`. The decoded RVBAR helper
  selects:
  - low3 = 0: `0x09010040 + core * 8`
  - low3 = 1..7: `0x08100040 + core * 8`
- The pinned BaseOS native SPL at commit
  `cbe253a43f46cf551cc9a6c79a84015bde3686cf` uses
  `(readl(0x03000024) & 7) == 2` to select the alternate bank
  `0x08100040`; otherwise it selects `0x09010040`.
- **This is a real selector-rule mismatch.** The two implementations agree
  only for VER_REG low3 values `0` and `2`; they diverge for
  `1, 3, 4, 5, 6, 7`. If the target value is one of those six values,
  the initial SPL CPU0 RVBAR bank and the later BL31 PSCI per-core RVBAR
  bank would differ. No claim is made that the target currently has one of
  those values.
- The H616-oriented linux-sunxi tools database independently associates
  H616 with main RVBAR bank `0x09010040`, alternate bank `0x08100040`,
  and VER_REG `0x03000024`. Its FEL helper treats a nonzero low byte as
  the alternate die. Upstream U-Boot's sunxi Kconfig likewise describes
  SRAM_VER_REG[7:0] as the H616 variant selector and uses the alternate
  bank when nonzero. These corroborate the *existence* of a die selector,
  but they do not resolve the different equality test in this project's
  pinned SPL or establish H700-specific semantics.
- The target selector value has not been read. A trusted read-only probe
  needs only `readl(0x03000024) & 7`; do not dump the full SYS_CFG word.
  Do not change the SPL or BL31 selector based on H616 naming alone. First
  establish the target value and intended H700 mapping, then make both
  stages use one identical tested rule.
- Added `scripts/read-h700-rvbar-selector-readonly.py`: run it as root on
  the running RG34XX/H700 Linux system with
  `python3 scripts/read-h700-rvbar-selector-readonly.py --json`. It opens
  `/dev/mem` with `O_RDONLY`, performs one 32-bit read at
  `0x03000024`, and emits only the masked low-three-bit value plus each
  stage's calculated bank and match status. It never prints the raw word,
  reads no other register, and performs no MMIO write.
- A Linux-runtime selector sample is useful because VER_REG is a version
  selector, but it is not a pre-BL33 readback of CPU reset/status state.
  The latter remains a separate early-handoff gate.

References:
- H616 User Manual v1.0, SYS_CFG/VER_REG register list and field table,
  pages 133–134:
  https://linux-sunxi.org/images/1/1d/H616_User_Manual_V1.0.pdf
- linux-sunxi RVBAR/die-variant database:
  https://github.com/linux-sunxi/sunxi-tools/blob/master/soc_info.c
- linux-sunxi FEL alternate-RVBAR selection:
  https://github.com/linux-sunxi/sunxi-tools/blob/master/fel.c
- Upstream U-Boot H616 alternate RVBAR Kconfig explanation:
  https://qemu.googlesource.com/u-boot/+/c7aafb20ce9937f1e178ded46d8f22742f54c982/arch/arm/mach-sunxi/Kconfig

This is strong static evidence that the matched vendor BL31 **can start
secondary CPUs on a PSCI power-on request**. It is not evidence that BL31
automatically starts them during its initial handoff to BL33. The current
diagnostic BL33 must not call PSCI CPU_ON unless SMP/coherency and per-core
initialization are deliberately implemented and reviewed.

Reproducible CI:
[#45816600012](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45816600012)
passed. The exact vendor source revision and callback ABI have not been
recovered; the mapping to `pwr_domain_on` uses the standard TF-A operation
table layout plus the matched binary's registration/dispatch structure.
Reference interface:
https://github.com/ARM-software/arm-trusted-firmware/blob/master/include/lib/psci/psci.h

## 6. Current assessment

| Core | Static source evidence | Confidence |
|---|---|---|
| CPU0 | Native SPL programs the primary RVBAR; BL31 has a CPU0 mode setup path | High for primary handoff |
| CPU1 | Boot0 helper does not target it; BL31's PSCI callback can power it on on request | Initial hold-off probable; live state unknown |
| CPU2 | Boot0 may assert reset when the undocumented gate is set; BL31 PSCI callback can release it on request | Initial hold-off probable; gate/live state unknown |
| CPU3 | Same as CPU2 | Initial hold-off probable; gate/live state unknown |

The combined evidence supports a CPU0-only **initial handoff** hypothesis,
but BL31 contains a callable PSCI CPU_ON path that can later release a
secondary core. This does **not** close the single-core startup gate. The
exact target H700 register semantics, bit29 feature-gate value, pre-BL31
mutations, and state at BL33 entry remain unmeasured.

## 7. Required proof before executing diagnostic BL33

Collect a read-only snapshot from a trusted **pre-BL33** environment (not from
normal Linux after the kernel has brought CPUs online):

- `C0_RST_CTRL`: `0x09010000`
- `C0_CPU_STATUS`: `0x09010080`
- `RVBARADDR0_L/H`: `0x09010040`, `0x09010044`
- `RVBARADDR1_L/H`: `0x09010048`, `0x0901004c`
- `RVBARADDR2_L/H`: `0x09010050`, `0x09010054`
- `RVBARADDR3_L/H`: `0x09010058`, `0x0901005c`
- `DBG_REG0/1`: `0x090100c0`, `0x090100c4`
- `CPU_SUBSYS_CTRL` relevant status: base `0x08100000`

For H616, `C0_CPU_STATUS[27:24]` reports per-core AMP/SMP mode
(`0=AMP`, `1=SMP`) and `[19:16]` reports per-core WFI standby; these
fields help interpret a snapshot but do not replace reading `C0_RST_CTRL`.
H700 field equivalence must be confirmed before decoding the live values.

Do not enable SMP, assign `MEMBASE/MEMSIZE`, set
`H700_MEMORY_LAYOUT_APPROVED`, write Boot0/TOC1/SD, or execute a new BL33
payload until the live handoff state and recovery path have been reviewed.
No hardware MMIO read/write, boot-media write, flash, or payload execution
was performed during this audit.


## Runtime selector probe result — /dev/mem is unusable on this kernel (2026-11-26)

The read-only collector was run on the connected RG34XX and reached the
actual register-read operation, but **no selector value was obtained**.

- Report commit:
  `8c084fc21ca29b9afa341adca944c7d553417e29`; report folder:
  [reports/rg34xx-rvbar-selector-20261126](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-rvbar-selector-20261126).
- Target: Linux `4.9.170 #19`, shell identity `uid=0(root)`. The image has
  no system `python3` in PATH, so the portable CPython 3.12.16 bundled with
  the LEI-V4 frontend was used. The script copy matched the host MD5
  `2739be359c087d9bae318dc461bf2d73`.
- One read was attempted: `os.pread(fd, 4, 0x03000024)` on an
  `O_RDONLY` `/dev/mem` descriptor. It returned `[Errno 14] Bad address`,
  exit 2, and zero stdout bytes. A separate `dd` of the same address
  returned the same EFAULT.
- The earlier PIO capture probe independently found the same
  offset-independent failure at offset 0, a single byte, and a full page
  of low RAM. `CONFIG_DEVMEM=y`; `CONFIG_STRICT_DEVMEM` is not set.
  Therefore this is not a selector-address-specific failure or a Python
  `pread` incompatibility, and the common strict-devmem `EPERM` gate is
  not the explanation.
- The device report found no SYS_CFG register node in the available
  debugfs register views: no sunxi register-dump node, and
  `debugfs/regmap` lists only four unrelated devices. No debugfs mount,
  module load, boot-argument modification, kernel-protection weakening,
  MMIO write or boot-media write was attempted.

**Interpretation:** the target's `VER_REG[2:0]` remains unknown. The failure
must not be converted into a guessed value or inferred from H616-family
references. Stop all userspace `/dev/mem` retries on this kernel. A future
runtime sample requires a reviewed kernel-side mapping/read interface for
SYS_CFG; a pre-BL33 instrumented capture is additionally required to prove
the CPU reset/RVBAR state at initial handoff. Neither path has been built or
run. The SPL/BL31 selector mismatch is still open, and no selector code
change is justified.

`hardware_applied=0`; the matched diagnostic BL33 has not been executed or
flashed, and SMP remains disabled.


## Kernel-module preflight and LK-side MMU alternative (2026-11-26)

The latest read-only device preflight is committed on the reports branch at
`2963c8c479486a98c5202dad86e14dbe6e0f466f`:

- Report: [rg34xx-kmod-preflight-20261126](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-kmod-preflight-20261126).
- The running Linux `4.9.170 #19` has `CONFIG_MODULES=y`,
  `CONFIG_MODULE_UNLOAD=y`, `CONFIG_KALLSYMS=y`, `CONFIG_DEVMEM=y`,
  and `CONFIG_STRICT_DEVMEM` unset.
- It has no `/lib/modules/4.9.170/build`, kernel `Makefile`,
  `Module.symvers`, or `/usr/src/linux*`. Thus a version-matched
  out-of-tree module cannot be built from the installed image.
- `syscon` is registered, but debugfs/regmap exposes only four unrelated
  devices; there is no SYS_CFG regmap view. `/proc/iomem` lists the PIO
  sub-range but no separate resource for SYS_CFG at `0x03000000`.
- No module was built or loaded and no register was accessed. The
  `/dev/mem` EFAULT is not a selector value. Stop userspace retries.

### Existing LK MMU coverage

The H700 LK hook `platform/h700/arch/h700_pio_mmu_mapping.c` calls
`arm_mmu_map_section(0x03000000, 0x03000000, DEVICE_SHARED | RW | XN)`
before `arch_enable_mmu()`. The 1 MiB AArch32 section is
`[0x03000000, 0x03100000)`, so it contains both:

- SYS_CFG VER_REG at `0x03000024`;
- RG34XX PIO at `0x0300b000`.

The mapping is installed by `arch_early_init` before
`target_early_init`. Compile-time assertions and the host fake-MMU regression
now pin both addresses to this section. This closes a *page-table coverage*
question only; it does not prove that BL31 leaves SYS_CFG readable from the
eventual BL33 security state, that the peripheral bus responds, or that the
register read is fault-free.

### Next safe experiment

The preferred next step is no longer an on-device kernel module. It is a
narrow LK-only diagnostic design:

1. Review the pinned LK AArch32 Data Abort vector and exception frame contract.
2. Add a recovery-contained accessor that attempts exactly one aligned
   32-bit read at `0x03000024`; on abort it records a fault status and resumes
   without retrying. No writes, other addresses, or broad register dump.
3. Decide how the masked value `VER_REG & 7` / fault status is returned without
   allocating memory or changing the boot-media contract.
4. Only after the source path and recovery are host-tested should a diagnostic
   BL33 execution be considered, with the existing recovery/rollback plan.

A post-entry LK read can establish a later VER_REG value, but **cannot**
prove the reset/RVBAR registers' state at the earlier BL31→BL33 handoff.
That remains a distinct pre-BL33 capture gate. The selector-rule mismatch
between the pinned SPL and BL31 remains unresolved; no code change is
authorized by a guessed value.

`hardware_applied=0`; no MMIO read/write was performed by this report.
