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

## SYSTEM_SUSPEND SRAM standby and RTC wake-return path (2026-11-27)

BL31 audit [#51](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45917139830)
passed 20/20 on code HEAD `cb2b836e8fcf70e4a482ce922cfc0a850c927a58`.
Artifact [15162412897](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45917139830/artifacts/15162412897)
contains the JSON report and expanded callback disassembly. Monitor identity is
unchanged: 107,216 bytes, SHA-256
`676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31`.

### Callback path to deep standby

- `get_sys_suspend_power_state` at `0x48002544` returns `0x02010000`.
- `0x48008d98` iterates affinity level 0 through 2 over callback map
  `0x48017d70`: `0x48008b84`, `0x48008ab8`, `0x48008a00`. These wrappers
  dispatch the legacy ops slot 3 `affinst_suspend`.
- Slot 3 system-level CCU/AON save branch returns `-4`; SYSTEM_SUSPEND caller
  compares this sentinel at `0x48009768` and calls special entry
  `0x48001a04` from `0x48009774`.
- Ordinary PSCI powerdown separately contains `DSB SY; WFI; B WFI` at
  `0x48015d70–0x48015d78`. This loop is distinct from the SYSTEM_SUSPEND
  SRAM-standby preparation path.

### EL3 cache teardown and SRAM image staging

The special entry invokes cache teardown through `0x48016128`: it reads
`SCTLR_EL3`, clears M/C bits with mask `0x5`, writes it back and executes
`ISB`. It then uses `CLIDR_EL1` / `CCSIDR_EL1` to select cache levels and
perform set/way maintenance before branching to the relocated SRAM entry.

The byte-copy helper is `0x4800aa80`. The main code/data image is copied as:

| Source | Destination | Length | Meaning / condition |
|---|---|---:|---|
| `[0x4800c220,0x48015630)` | `[0x20000,0x29410)` | `0x9410` | standby image, always |
| `[0x48000030,0x480000b0)` | `[0x20000,0x20080)` | `0x80` | BL31 header/context prefix, always; overlays first 0x80 bytes |
| `[0x4803cb90,0x4803cbc0)` | `[0x20080,0x200b0)` | `0x30` | auxiliary runtime context; source lies outside monitor file payload |
| `[0x4801b308,0x4801b368)` | `[0x200d4,0x20134)` | `0x60` | conditional context block |
| `[0x4801b380,0x4801b400)` | `[0x20134,0x201b4)` | `0x80` | conditional context block |

The copied entry source `0x4800c3d8` is offset `+0x1b8` from the main
copy base and therefore executes at SRAM address `0x201b8`. The copied
stack helper source `0x480140a0` maps to `0x27e80`; its literal stack top
is `0x57ff0`. The entry clears relocated scratch/BSS region
`[0x291f0,0x29349)`, derived by the same source/destination offset mapping.

This layout extends beyond the low SRAM window into higher SRAM. Static copy
instructions prove the intended addresses, but do not prove runtime SRAM C
ownership, retention, clocking or arbitration conditions. Those remain closed
hardware prerequisites, not assumptions.

### RTC standby flag, wake entry and BL31 continuation

The copied path contains WFI instructions at source PCs `0x4800c50c` and
`0x4800c550`. It writes standby flag `0x00010000` to `0x070001f8`, and
writes the saved return address from context offset `+0xc4` to RTC soft-entry
register `0x070001fc`. The stored return address is `0x48015d14`.

On the statically reconstructed wake path, copied code reads context
`+0xc4` and reaches `BLR X22` at `0x4800c6a8`. The BL31 continuation at
`0x48015d14` restores/configures `SCTLR_EL3`, points `VBAR_EL3` to
`0x48019000`, clears the IRQ mask with `DAIFClr #4`, selects `SPSel=0`,
restores CPU/PSCI context and branches to `0x480158c8`.

### CPUX_AXI clock-field operation and wait helper

The standby and resume paths both access candidate register `0x03001500`:

- Standby reads it, applies `AND #0xfcffffff` (clears bits 25:24), writes
  it and calls `0x48011ff8`.
- Resume reads it, applies `ORR #0x03000000` (sets bits 25:24), writes it
  and calls the same helper.
- The H616 reference map labels offset `0x500` as `CPUX_AXI_CFG_REG` and
  identifies `CLK_SRC_SEL[26:24]`. These operations preserve bit 26, so
  exact mux selection cannot be inferred from these instructions alone.
  Do not infer a PLL source, CPU OPP, or DVFS API.
- Wait helper `0x48011ff8` performs `LSL W0,W0,#2` then branches to
  `0x48014078`; that loop executes `SUBS X0,X0,#1; B.HI 0x48014078; RET`.
  Each caller passes 10, yielding 40 decrement iterations. No timer or
  reference-frequency conversion is present in this path; elapsed time is
  unknown.

### Evidence boundary and next work

CI #51 proves these instruction words, BL targets, branch targets, copied
address arithmetic and table entries against the matched static monitor. It
is not evidence that the platform successfully entered standby or woke up.
No PSCI SMC, MMIO access, RTC/PMIC probe, cache operation, storage write or
firmware execution was performed on the device.

Next inspect remaining called helpers (`0x48011b40`, `0x48011ba8`,
`0x48011df8`, `0x48011b58`) and identify exact register semantics/source
provenance. Separately establish SRAM C ownership/clock lifecycle, RTC wake
source, CPUX_AXI live field value, and close BL31/Boot0 memory ownership,
secondary A53 hold-off and timer-PPI prerequisites before any reversible
runtime test.

`hardware_applied=0` remains unchanged.

## PLL/CCU save-restore, AON standby key and wake-source helpers (2026-11-27)

BL31 static audit [#59](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45919948625)
passed 20/20 on code HEAD `6037e706313bd45b8f1ad86129a320bc6411a2bf`.
Artifact [15164072253](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45919948625/artifacts/15164072253)
contains the full callback disassembly and machine report. The matched monitor
identity remains 107,216 bytes, SHA-256
`676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31`.

### CCU context and PLL_CPUX transition

Helper `0x48011ba8` saves masked fields from:

| Register | H616/sun50iw9 family name | Saved mask |
|---|---|---:|
| `0x03001500` | CPUX_AXI_CFG_REG | `0x00000303` |
| `0x03001510` | PSI_AHB1_AHB2_CFG | `0x0003031f` |
| `0x0300151c` | AHB3_CFG | `0x0003031f` |
| `0x03001520` | APB1_CFG | `0x00000303` |
| `0x03001524` | APB2_CFG | `0x00000303` |

The values are staged into PSCI context beginning near `0x48015414`.
The helper also snapshots PLL_CPUX factor fields from `0x03001000` with
mask `0x0003ff03`, clears PLL enable bit31 and lock-enable bit29 before
standby, and sets the high byte of `0x07010244` to `0xa7`.

The resume helper `0x48011df8` clears the high byte at `0x07010244`,
restores PLL_CPUX factor fields, sets enable bit31 and lock-enable bit29,
then polls lock-status bit28 at `0x48011e7c`. After lock, it restores the
saved CCU register fields. This is a suspend/resume clock transition and does
not expose a front-end CPU DVFS profile/API.

### R_PRCM PLL LDO key and RTC XO_CTRL cross references

The static code constructs address `0x07010244` as R_PRCM base
`0x07010000` + offset `0x244`, writes high byte `0xa7` on standby and
clears that high byte on resume. A same-family sun50iw9/H616 suspend analysis
identifies R_PRCM+0x244 as the PLL LDO key:

- [HookedBehemoth/SenkOS suspend notes, pinned commit](https://github.com/HookedBehemoth/SenkOS/blob/802be0c07939ab4f729f901d3c275101754e9e34/docs/suspend.md)

For saved mode word == 2, matched BL31 sets bits25:24 to `10b` in
`0x03001510`, `0x07010000` and `0x07010010`; it also sets bit31 and
clears bit1 at `0x07000160`. The RTC CCU header names offset +0x160
`XO_CTRL_REG`, and the same-family suspend notes document the bit31/bit1
operation:

- [sun50iw9 RTC CCU header, pinned commit](https://github.com/radxa/allwinner-bsp/blob/2045a3ca2a01f088c0314dc924bda59d154e363e/drivers/clk/sunxi-ng/ccu-sun50iw9-rtc.h)
- [H616/sun50iw9 CCU register map](https://github.com/rustsbi/allwinner-hal/blob/d90687bdf2012b251062db77a779350f801c3753/allwinner-hal/src/ccu/register/h616.rs)

These are same-family register-name/sequence cross references. They do not
establish the exact OEM TF-A source commit or live RG34XX register values.
The source does not identify the selected CPUX_AXI mux without bit26 readback;
do not infer a PLL parent, OPP or DVFS capability from the masks alone.

### Wake-source helper chain

The aggregate helper `0x48013ee8` statically calls these subpaths:

| Helper | Behavior established by instruction body | Register/region |
|---|---|---|
| `0x48013190` | Computes word index `irq >> 5`, bit `irq & 31`, reads pending status | GICD_ISPENDR at `0x03021200` |
| `0x480131b8` | Reads IR status, acknowledges low status byte, drains RX FIFO into standby context queue | IR status `0x07040030`, FIFO `0x07040020` |
| `0x48013588` | Parses ASCII GPIO names into bank/pin encoding | GPIO name/context strings |
| `0x480135d8` | Reads PIO DAT bit for requested bank/pin | PIO `0x0300b000`, R_PIO `0x07022000` |
| `0x48013620` | Applies context-supplied mux/pull/drive/data configuration | PIO CFG/DAT/drive/pull register families |

This proves firmware contains wake-source preparation/polling logic, not that
the RG34XX has successfully armed or triggered those sources. No GIC pending
register, IR FIFO, GPIO DAT or pinmux/pinconf register was accessed on target;
the GPIO helper contains write instructions but was not executed.

### Remaining work and evidence boundary

BL31 audit #59 asserts the instruction words, address construction, mode-2
RTC/XO operations, PLL enable/LOCK sequence, wake-source call edges and helper
windows against the exact monitor payload. This remains static evidence only.
No PSCI SMC, MMIO, RTC/PMIC access, cache operation, storage write, flash or
payload execution was performed.

Next inspect checksum/context helpers `0x48011b78`, `0x48011920`,
`0x48011b38`, `0x48011b10` and `0x48011b30`; determine data coverage,
return conditions and failure path. Separately establish SRAM C ownership,
clock/retention prerequisites, wake-source state, BL31/Boot0 memory ownership,
secondary A53 hold-off and timer-PPI handoff before considering any reversible
runtime diagnostic.

`hardware_applied=0` remains unchanged.

## Standby SRAM additive integrity checksum and fail-stop path (2026-11-27)

BL31 audit [#60](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45920492462)
passed 20/20 on code HEAD `74151f7e5347c23b06b82afc09e041266606dd8c`.
Artifact [15164342241](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45920492462/artifacts/15164342241)
contains the expanded disassembly and machine assertions.

The checksum helper at `0x48011b78` loads start pointer and byte length from
context `+0xb0/+0xb4`, initializes a 32-bit sum to zero, adds each 32-bit word
while advancing by four bytes, and returns the wrapped 32-bit additive sum.
It is not CRC or a cryptographic hash.

Standby/resume call flow:

| Stage | PC / field | Behavior |
|---|---|---|
| Optional enable | `0x4800c43c` | Reads context `+0xc0`; zero skips initial checksum |
| Standby checksum | `0x4800c448` | Calls `0x48011b78` |
| Save | `0x4800c44c` | Writes sum to context `+0xb8` |
| Resume checksum | `0x4800c6c4` | Recomputes using context start/length |
| Compare | `0x4800c6d8` | Compares saved `+0xb8` against resumed `+0xbc` |
| Success | `0x4800c6dc` | Branches to resume continuation |
| Failure | `0x4800c6e0–0x4800c6ec` | Builds `0x88000001`, writes via x20 (this path constructs base `0x07000108`), then self-loops |

The mismatch branch is explicit fail-stop behavior: it does not continue
normal resume/restore after a checksum mismatch.

**Range limitation:** helper uses caller-provided context `+0xb0/+0xb4`;
current evidence has not located assignments to those two fields. Therefore
the checked range is configurable and must not yet be described as the full
`0x9410`-byte copied SRAM image. Context `+0xc0` is an enable flag; its
assignment and mode semantics also remain under analysis.

CI #60 pins helper-loop words, both calls, context fields, compare branch,
fail-code construction and fail-stop loop. No hardware checksum execution,
MMIO, PSCI SMC, storage write or suspend/resume test was performed.

`hardware_applied=0` remains unchanged.


## 2026-11-27 BL31 pre-BL33 non-secure peripheral-security setup cross-reference

A separate exact-target audit in
[UART handoff audit](rg34xx-uart-bl31-handoff-audit.md) identifies the
vendor monitor routine at file offset `0x2c78`, reached from the BL33
preparation path at offset `0x21b4`. The target binary constructs and writes
the following before preparing AArch32 SVC BL33:

| Operation | Target address | Value |
|---|---:|---:|
| SPC decode-port set, ports 0–13 | `0x03008004 + 0x10*n` | `0xffffffff` each |
| CCU security switch | `0x03001f00` | `0x00000007` |
| R_PRCM security switch | `0x07010290` | `0x00000007` |
| DMA security mask | `0x03002020` | `0x00000fff` |
| Additional SPC write | `0x030080e0` | `0x00000001` |

The sequence is statically placed in the target BL33-preparation path, not
merely found in an unrelated function. A pinned H616 TF-A reference
(`87cc230066271910096e025cf07376dba6c361d8`) provides the closest public
semantic cross-reference:

- `plat/allwinner/common/sunxi_security.c` describes the intent as setting
  all SPC decode ports and bus/clock/PLL security switches for Non-secure
  access.
- `plat/allwinner/sun50i_h616/include/sunxi_mmap.h` places SPC at
  `0x03008000`, CCU at `0x03001000`, and PIO at `0x0300b000`.
- `plat/allwinner/sun50i_h616/include/sunxi_spc.h` defines 14 decode ports
  and the `+0x04 + 0x10*n` set-register stride.
- The TF-A source explicitly warns that SPC configuration may not take
  effect unless a particular fuse is programmed (the source calls this an
  erratum). The target-specific `0x030080e0` write is not explained by that
  public H616 header.

**What this improves:** we now have positive static ordering evidence that
the target monitor attempts broad peripheral Non-secure security setup before
BL33. This is materially stronger than assuming permissions are inherited
unchanged from SPL.

**What it does not establish:** the exact decode-port-to-peripheral mapping
for this H700 target, the relevant fuse identity/value, whether the fuse
condition makes SPC writes effective, the live SPC/CCU security state at
BL33 entry, or whether APB1/PIO is clocked and out of reset. The ordinary
APB1 clock register `0x03001520` appears in separate BL31 PSCI
save/restore paths; this is not evidence of its value at initial handoff.
The six earlier BL31 PIO scan rows at `0x0300b100/110/118` remain the
conditional PH10 infrared-pin setup, not PI12 activity.

Current RG34XX read-only inventory only captured the NVMEM speed and thermal
calibration cells; it did not capture a secure-mode fuse. Do not infer fuse
state from the A733/H616 family examples or read an unverified SID offset.
A narrowly scoped future fuse-read is permissible only after the exact
RG34XX/H700 fuse index, vendor read path, side effects, and privacy-safe
masked output are pinned to target evidence.

**Disposition:** PI12 physical MMIO remains default-off. This static
security-path evidence narrows the question but does not close M1-S1. No
device MMIO, fuse read, storage write, firmware flash, or payload execution
was performed.
