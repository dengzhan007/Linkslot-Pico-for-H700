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
