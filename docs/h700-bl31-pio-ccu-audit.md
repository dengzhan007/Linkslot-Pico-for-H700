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
low-power clock/context preservation, but the exact register names, callback
registration, and runtime invocation have not yet been proven. These rows
are not evidence that BL31 exposes CPU DVFS or that Linkslot's CPU-frequency
service can be enabled. No clock or voltage write is approved.

The PI12 production gate, APB1/PIO handoff gate, memory-ownership gate, and
secondary-A53 gate remain closed. No hardware was touched.
