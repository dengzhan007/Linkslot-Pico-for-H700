# RG34XX BL31 PIO / CCU access-candidate audit

Date: 2026-11-26  
Target: RG34XX non-SP / Allwinner H700 / Cortex-A53  
Scope: matched vendor BL31 binary; static host-only inspection.

## Address provenance

| Classifier | Physical interval | Evidence and limit |
|---|---|---|
| `CPUX_CCU` | `[0x03001000, 0x03002000)` | 4 KiB CPUX clock/CCU resource from the pinned `sun50iw9p1-clk.dtsi` source audit. It is not a live handoff readback. |
| `PIO` | `[0x0300b000, 0x0300b400)` | Exact `reg` interval from the sanitized active live FDT node `/soc@03000000/pinctrl@0300b000`; size 0x400. |
| `SID_SYS` | `[0x03000000, 0x03010000)` fallback | Broad SYS_CFG/SID/system-control candidate region. The narrower CCU and PIO labels take precedence. |

The BaseOS native SPL audit separately found PIO accesses before BL31:
PI15/PI14 at `0x0300b124` / `0x0300b130`, and PE6 reads at
`0x0300b090`, `0x0300b0ac`, and `0x0300b0a0`. That confirms pre-BL31
use only; it does not prove the post-BL31 state.

## Method

`scripts/audit-rg34xx-bl31-full-image-cpu-mmio.py` checks the exact monitor
payload by size and SHA-256 before scanning all aligned overlapping AArch64
decode windows. It keeps local register-constant tracking and literal-word
candidates, then classifies a hit into the narrowest declared range.
`scripts/test-audit-rg34xx-bl31-full-image-cpu-mmio.py` checks the CCU/PIO
range boundaries as well as the existing CPUCFG synthetic cases.

## Interpretation boundary

- The scanner is a heuristic, not a complete CFG, call graph, MMIO trace, or
  semantic proof of every indirect/secure-world path.
- A hit must be reviewed against its instruction context and reachable
  control flow before attributing it to an executed BL31 path.
- No hit does not prove no access; register indirection, data tables,
  unaligned instruction paths, secure services, or hidden ROM behavior may
  evade the scan.
- The classifier does not read or write hardware and says nothing about
  APB1 clock/reset state, PIO access permissions, or register values at
  BL33 entry.
- PI12 stays default-off; no device MMIO, boot-media write, flash, module
  load, or payload execution is authorized by this audit.

Next: inspect any CCU/PIO candidate rows emitted by CI in the saved disassembly
context. Regardless of hit count, retain the independent BL31→BL33 handoff,
memory-ownership, and secondary-A53 gates.


## Manual review of BL31 PIO candidate (2026-11-26)

CI run [#45851421825](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45851421825)
completed successfully. Its initial 16-instruction scanner emitted one PIO
candidate at PC `0x480134c8`, effective address `0x0300b100`.

A direct AArch64 disassembly of the exact matched monitor payload shows that
the code around `0x48013490` constructs addresses in the live PIO interval
and performs loads/stores at `0x0300b100`, `0x0300b110`, and
`0x0300b118`. The code is not just an isolated malformed decode. The local
direct-call chain is:

```
0x4800c404 -> 0x4800c4d0
0x4800c53c -> 0x48013df0  (conditional branch)
0x48013e40 -> 0x48013490
```

The `0x4800c4d0` path includes `SCR_EL3` accesses and `WFI`; it is
consistent with a conditional low-power / power-management path, but the
exact callback identity is not yet established. This chain is absent from
the current primary-entry direct-CFG report, and the registration/invocation
of the `0x4800c3f4` routine remains unresolved. Therefore:

- Confirmed: the exact matched BL31 binary contains code that accesses the
  live PIO resource range.
- Not confirmed: the code executes during initial BL33 startup/handoff.
- Not confirmed: APB1/PIO clock state, reset state, or firewall permissions
  when BL33 begins.
- Not authorized: enabling PI12 MMIO, making any hardware changes, or
  treating the presence of the path as proof of its runtime invocation.

The original 16-instruction local-tracking window only reported the first
PIO load. The scanner is being widened to 40 instructions, with a host
regression for register setup spread over 24 intervening instructions, so
the next CI artifact can report the complete candidate sequence more
reliably. Keep the production PI12 gate closed.


## Expanded scan result and PIO register decode (2026-11-26)

The 40-instruction scanner and its long-setup regression passed in BL31 audit
run [#45852546186](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45852546186)
on code HEAD baf8f4dd02453c1c42772ee4d4782edda4de5855. It reported 605
decoded memory-operation candidates across the full image:

| Range | Candidate rows |
|---|---:|
| CPUX_CCU | 152 |
| PIO | 6 |
| SID_SYS | 275 |
| AON_CPUCFG_PRCM | 148 |
| C0_CPUX_CFG | 20 |
| CPU_SUBSYSTEM | 4 |

These are scanner rows from overlapping decode windows, not 605 unique,
reachable, executed hardware transactions. The six PIO rows are the three
read/modify/write pairs at offsets 0x100, 0x110, and 0x118.

### The PIO sequence is PH10 / S_CIR0, not PI12

The live RG34XX FDT reports the controller at 0x0300b000 with compatible
allwinner,sun50iw9p1-pinctrl. As a source-family cross-reference, the
sun50iw9p1 pinctrl implementation and pinctrl DTSI in
[qiaoweibiao/T507_Kernel at commit
98fcc7d3f112e51b0edfb71536da89cb2115106f](https://github.com/qiaoweibiao/T507_Kernel/tree/98fcc7d3f112e51b0edfb71536da89cb2115106f)
use a 0x24-byte bank stride, MUX offset 0x00, drive-level offset 0x14, and
pull offset 0x1c. PH is bank 7, so its PH_CFG1 / PH_DRV0 / PH_PUL0 offsets
are respectively 0x100, 0x110, and 0x118.

The matched BL31 instructions construct these three addresses and apply:
- PH_CFG1 at 0x0300b100: PH10 mux field set to function 3;
- PH_DRV0 at 0x0300b110: PH10 drive field set to 2;
- PH_PUL0 at 0x0300b118: PH10 pull field set to 1 (pull-up).

That is an exact semantic match for the reference DTSI's s_cir0_pins_a
entry: PH10, function ir, muxsel=3, drive=2, pull=1. The low-level
routine at 0x48013490 is therefore configuring the PH10 infrared-receiver
pin. The relevant address sequence does not touch the RG34XX PI12 work-LED
registers.

For contrast, the same bank formula places PI12's mux/data/drive/pull fields
at 0x0300b124, 0x0300b130, 0x0300b134, and 0x0300b13c. The scanner now adds
source-family register/pin-field labels to matching PIO rows, and host tests
assert both PH10 and PI12 offsets. This remains static source-based
classification; the exact vendor 4.9.170 pinctrl source revision was not
separately pinned.

### Execution-path boundary

The local branch path is still:
```
0x4800c404 -> 0x4800c4d0
0x4800c53c -> 0x48013df0  (conditional)
0x48013e40 -> 0x48013490  (PH10 setup)
```
The enclosing code manipulates SCR_EL3 and executes WFI; the PH10 routine is
called only on a conditional bit-2 path inside 0x48013df0. The entry or
registration of the enclosing routine remains unresolved and the path is
absent from the primary-entry direct-CFG report. The binary therefore proves
the presence of this conditional low-power infrared-pin setup, not that it
executes at every cold boot or establishes PIO access for non-secure BL33.

### CPUX / CCU candidate interpretation

The expanded scan produced 152 CPUX_CCU rows. A manual disassembly review
finds a coherent non-primary-entry save/restore sequence: code around
0x48001790 snapshots 0x03001000, 0x03001a78, and 0x03001a8c, modifies
selected bits, and saves context; the paired 0x480018d8 path restores the
saved values. It is called from a path containing power-state checks and
AON/PRCM accesses (including 0x07000200). This is consistent with vendor
power-management clock/context preservation. The follow-up PSCI audit
verified platform-ops table registration and direct BL branches at PC
0x48002998 -> 0x48001790 (save helper) and PC 0x48002858 -> 0x480018d8
(restore helper). However, the exact vendor TF-A ABI is not pinned, so table
slots 3/7 remain unclassified; the callsites have not been proven to belong
to the same callback or to be reachable from one another. These rows are not
evidence that BL31 exposes CPU DVFS or that Linkslot's CPU-frequency service
can be enabled. No clock or voltage write is approved.

The PI12 production gate, APB1/PIO handoff gate, memory-ownership gate, and
secondary-A53 gate remain closed. No hardware was touched.


## PSCI table correlation: direct branch targets are not callback ownership (2026-11-26)

BL31 static audit [#45856177699](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45856177699)
passed 20/20 steps on branch HEAD `7c65f60a416f4e80c5d6290ceee37ccc072b9f8d`.

The matched monitor has a platform-ops table at `0x480165f8`, registered
through `0x48009dec → 0x480029bc`. PSCI dispatcher code loads the
`psci_plat_pm_ops` pointer and indirect-calls the selected table member.
Slot 1 points to `0x4800254c`; its validated path reaches CPU_ON helper
`0x48002d78`.

The table values at slot 3 (`0x48002878`) and slot 7 (`0x48002574`)
are recorded but intentionally **unclassified**. The shipping monitor's
exact TF-A ABI/version has not been pinned. Public TF-A header revisions
change the callback layout; for example, the pinned public header at
[commit `9839bceb...`](https://github.com/ARM-software/arm-trusted-firmware/blob/9839bceb8b720ba09f3bd3d5f95188ea9d75837b/include/lib/psci/psci.h#L335-L372)
contains additional operations before the suspend callback. This is a
version-drift warning, not evidence that the shipping monitor uses that
specific layout.

The verifier confirms two separate direct branch targets:
- `0x48002998 → 0x48001790`: CCU/AON save helper;
- `0x48002858 → 0x480018d8`: CCU/AON restore helper.

It does **not** yet establish that both call sites belong to the same
callback, that either is reachable from slot 3/7, or that these paths run
during initial BL31→BL33 handoff. The current conclusion is limited to
verified table registration, slot-1 CPU_ON control flow, and the presence
of two BL31 power-management helper edges. Do not infer CPU DVFS, PLL
startup sequencing, or runtime suspend/resume behavior from this audit.


## PSCI CCU helper calls mapped to separate raw ops-table functions (2026-11-26)

BL31 audit #33 [#45857603997](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45857603997)
passed 20/20 steps. Artifact [15136726031](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45857603997/artifacts/15136726031)
contains the expanded `PSCI-CALLBACK-DISASSEMBLY.txt` plus the JSON assertions.

The verified platform-ops raw pointer values are:
- slot 3: `0x48002878`;
- slot 4: `0x48002680`;
- slot 5: `0x480026c4`.

Disassembly from raw slot 3's entry at `0x48002878` shows a conditional
call at `0x48002998` to the CCU/AON save helper `0x48001790`. The same
function restores its frame and returns at `0x480029b4`.

The separate function beginning at raw slot 5's entry `0x480026c4`
contains a conditional call at `0x48002858` to the CCU/AON restore helper
`0x480018d8`. Its epilogue then tail-branches at `0x48002874` to
`0x48002680`, which is the raw slot 4 pointer. The branches at
`0x480026e4` and `0x480026ec` can bypass the restore call, so this is a
conditional path, not an unconditional operation.

The CI now asserts the slot 3/4/5 pointer values, both BL targets, the
AArch64 B target at `0x48002874`, and the exact slot 5 disassembly window
`[0x480026c4, 0x48002878)`. Thus the save and restore helper callsites are
associated with distinct raw ops-table function regions.

**Interpretation limit:** slot 3/4/5 names remain unclassified because the
exact shipping TF-A ABI/source version is unknown. Do not label them as
suspend/off/finish/resume from slot numbers alone. The static code does not
prove runtime execution or cold-boot reachability; it does not establish a
CPU DVFS API or startup PLL change. Hardware remains untouched.

## Legacy PSCI table and SYSTEM_SUSPEND getter (2026-11-27)

BL31 static audit [#39](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45906188892)
passed 20/20 on code HEAD `4afe6788d2af69434aa9f1c36aa7cb1b75e14032`.
Artifact [15157244797](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45906188892/artifacts/15157244797)
contains the generated JSON report and callback disassembly. The matched monitor
remains 107,216 bytes with SHA-256
`676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31`.

### Effective platform-ops layout

The nine 64-bit pointers at `0x480165f8` resolve as:

| Slot | Member | Pointer | Dispatch evidence |
|---:|---|---:|---|
| 0 | `affinst_standby` | `0x48002510` | load `+0x00` at `0x48009688`; indirect call `0x48009690` |
| 1 | `affinst_on` | `0x4800254c` | load `+0x08` at `0x48008608`; indirect branch `0x48008640` |
| 2 | `affinst_off` | `0x4800264c` | load `+0x10` at `0x480080f8`; indirect call `0x48008118` |
| 3 | `affinst_suspend` | `0x48002878` | load `+0x18` at `0x48008a84`; indirect call `0x48008a90` |
| 4 | `affinst_on_finish` | `0x48002680` | load `+0x20` at `0x4800854c`; indirect call `0x48008560` |
| 5 | `affinst_suspend_finish` | `0x480026c4` | load `+0x28` at `0x48008918`; indirect call `0x4800893c` |
| 6 | `system_off` | `0x480025a0` | helper call `0x480025a8 → 0x48001ed8` |
| 7 | `system_reset` | `0x48002574` | reset helper call `0x48002588 → 0x48001ec8`; reset-specific strings |
| 8 | `get_sys_suspend_power_state` | `0x48002544` | getter returns `0x02010000`; caller loads `+0x40` and calls indirectly |

The first eight members follow the legacy Allwinner TF-A `plat_pm_ops_t`
callback family/order; slot 8 is a vendor extension. This layout is not the
modern `plat_psci_ops_t` layout. The exact shipping vendor TF-A source commit
has not been recovered; member naming is based on table order, per-member
dispatch byte offsets, function behavior and embedded PSCI strings.

### SYSTEM_SUSPEND call chain

- Getter address: `0x48002544`; instructions: `mov w0, #0x02010000; ret`.
- Caller region starts at `0x480096f8`; the global ops pointer is loaded at
  `0x48009718`, slot 8 at byte offset `+0x40` is loaded at `0x4800971c`,
  and the function pointer is invoked indirectly at `0x48009744`.
- At `0x4800975c`, the returned state is passed to transition function
  `0x48008d98`.

### Current limits and next work

These checks pin a static binary's callback registration and dispatch, not a
live suspend/resume operation. No PSCI SMC, MMIO read/write, flash, boot-media
write or payload execution was performed. The next analysis target is
`0x48008d98`: reconstruct the affinity-level transition loop, its callback
tables, and the order/conditions for suspend and finish callbacks. The function
pointer-looking data near `0x48017b20` and `0x48017d70` is only a lead until
its consumers and callsites are recovered.

`hardware_applied=0` remains unchanged.
