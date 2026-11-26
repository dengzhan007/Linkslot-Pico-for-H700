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

## 5. Current assessment

| Core | Static source evidence | Confidence |
|---|---|---|
| CPU0 | Native SPL programs the primary RVBAR; BL31 has a CPU0 mode setup path | High for primary handoff |
| CPU1 | Not passed to Boot0 reset helper; H616 reset default holds it in reset | Probable, not target-verified |
| CPU2 | Boot0 may execute a helper that clears its reset bit; with H616 POR default that bit is already zero | Probable, gate bit and live register not measured |
| CPU3 | Same as CPU2 | Probable, gate bit and live register not measured |

This supports a CPU0-only boot hypothesis but does **not** close the single-core
startup gate. The exact target H700 register semantics, bit29 feature-gate
value, pre-BL31 mutations, and state at BL33 entry remain unmeasured.

## 6. Required proof before executing diagnostic BL33

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
