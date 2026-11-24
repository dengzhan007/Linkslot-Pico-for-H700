# H700 CPUX / sun50iw9 clock-source audit

**Target:** Anbernic RG34XX (non-SP) / Allwinner H700 / Cortex-A53  
**Audit date:** 2026-11-25  
**Audit type:** pinned vendor-source review; **not** physical register readback, silicon validation, or approval for DVFS writes.  
**Decision:** retain the CPU performance service in policy-only / unavailable mode. No PLL, CCU, PMIC, or regulator write code is authorized by this audit.

## 1. Source provenance and applicability

The review is pinned to the following vendor tree revision:

- Repository: [qiaoweibiao/T507_Kernel](https://github.com/qiaoweibiao/T507_Kernel)
- Commit: [`98fcc7d3f112e51b0edfb71536da89cb2115106f`](https://github.com/qiaoweibiao/T507_Kernel/commit/98fcc7d3f112e51b0edfb71536da89cb2115106f)
- SoC source family: `sun50iw9p1` / `sun50iw9`

Reviewed files at that immutable commit:

| File | Blob SHA | Relevant finding |
|---|---|---|
| `arch/arm64/boot/dts/sunxi/sun50iw9p1.dtsi` | `32ddb772c25aff3bd945bf35271dfdbbd70dade0` | Four Cortex-A53 CPU nodes reference the same CPU PLL and `opp-shared` |
| `arch/arm64/boot/dts/sunxi/sun50iw9p1-clk.dtsi` | `7708d3083bc6c2f3295aa1d110fc8103088b5b6c` | CPUX clock-init window declared at `0x03001000` |
| `drivers/clk/sunxi/clk-sun50iw9.h` | `579185376c9271449e02a445d8f5840286f95d42` | PLL_CPU / CPU_CFG offsets |
| `drivers/clk/sunxi/clk-sun50iw9.c` | `f29be57a194848955260ddcc7afb18682508f9b4` | PLL fields, parent list, factor registration, CPU mux configuration |
| `drivers/clk/sunxi/clk-cpu.c` | `a6cd887d36a0a9153a48c0f4df5bbc994631a2fa` | CPU clock pre-rate / post-rate parent-switch notifier |
| `drivers/clk/sunxi/clk-factors.c` | `5acb3d3c39720adce465e57a81bf2da1b3c84849` | Factor programming and lock polling implementation |
| `drivers/clk/sunxi/clk-factors.h` | `628a36048044e3326a87ff431ae75513a361263f` | Register-field / lock-mode structures and macros |
| `drivers/clk/sunxi/clk-sun50iw9_tbl.c` | `cf0c122aec7bed3ed9000f98abb987514ecce43b` | PLL factor lookup tables; explicitly marked unreliable |

`sun50iw9p1` is a useful, more specific vendor-family reference than treating a generic H616 table as an H700 implementation. Nevertheless, this repository/commit has not been proven byte-for-byte identical to the RG34XX shipping boot firmware or the exact silicon revision in the test unit. Every result below is therefore a **source-level candidate**, not a board-level hardware fact.

## 2. Register and clock-tree facts in the pinned source

The source defines offsets relative to the CPUX CCU mapping:

| Symbol | Offset | Source-level meaning |
|---|---:|---|
| `PLL_CPU` | `0x0000` | CPU PLL factor/control register |
| `CPU_CFG` | `0x0500` | CPU clock parent mux and CPU bus divider fields |

The vendor clock-init DTS declares the first CCU resource as the CPUX window at physical candidate base `0x03001000` (size `0x1000`). The driver maps that resource with `of_iomap(node, 0)` and then adds the offsets above. **Do not turn this source DTS address into an LK MMIO pointer yet**: the actual RG34XX live FDT, BL33 address mapping, security/firewall state, and handoff register values have not been read back.

The `sun50iw9` clock driver declares:

- CPU mux at `CPU_CFG + 0x00`, shift 24, width 3.
- CPU parents, in order: `hosc`, `losc`, `iosc`, `pll_cpu`, `pll_periph0`.
- CPU alternate parent used by its notifier: `pll_periph0`; normal parent: `pll_cpu`.
- AXI divider field: `CPU_CFG`, shift 0, width 2.
- CPUAPB divider field: `CPU_CFG`, shift 8, width 2.

Those fields establish which parts of the clock tree the vendor driver models. They do not provide the live mux selection, actual `pll_periph0` rate, divider limits for the current board, timer source continuity, or proof that a direct transition is safe in BL33.

## 3. PLL factor layout and the enable-bit safety rule

The `SUNXI_CLK_FACTORS(pll_cpu, ...)` declaration configures these fields in `PLL_CPU`:

| Field | Shift | Width | Meaning |
|---|---:|---:|---|
| N | 8 | 8 | Integer multiplier |
| M | 0 | 2 | M divider |
| P | 16 | 2 | Power-of-two P divider |
| output gate / configured enable field | 27 | 1 | PLL output gating used by this driver |

The source's rate calculator uses the form:

`rate = parent_rate × (N + 1) / ((M + 1) × 2^P)`

with the normal oscillator parent represented as 24 MHz in this code path. This equation describes the source's factor model only; no factor tuple or frequency from its lookup table is an approved RG34XX OPP.

**Critical source comment:** the vendor driver says the PLL enable bit 31 should be enabled at boot and should not be changed dynamically; it uses bit 27 for PLL output gating instead. Accordingly:

- Do not implement a runtime sequence that clears/re-sets bit 31 as a generic “PLL off/on” operation.
- Do not assume bit 27, bit 31, or the lock indication can be modified independently without the exact SoC revision's register specification and readback.
- Preserve unrelated register fields with a verified read-modify-write design only after the hardware path is approved.
- The source comment is a strong safety warning, not a complete PLL programming specification.

## 4. Parent-switch and lock sequence found in the Linux driver

The vendor driver registers a clock notifier on `pll_cpu`. Its flow is:

1. On `PRE_RATE_CHANGE`, switch the CPU clock parent to the alternate parent `pll_periph0`.
2. The factor clock changes the `pll_cpu` N/M/P fields.
3. The PLL lock helper checks the lock state.
4. On `POST_RATE_CHANGE`, switch the CPU clock parent back to `pll_cpu`.

This establishes that the vendor implementation does **not** simply rewrite the active CPU PLL while the CPU mux remains on it. It deliberately moves the CPU clock to another parent before changing PLL factors and restores the normal parent after the rate change.

The same driver uses `PLL_LOCK_NEW_MODE` for `pll_cpu`, with the lock-control register at offset `0x0000` and lock-enable field at bit 29. The helper:

- clears lock-enable bit 29 and mode bit 28 before a rate change;
- enables mode bit 28 and then bit 29 when checking lock;
- polls the configured lock-status register/bit, with up to 5,000 loop iterations, 1 µs delay per unsuccessful iteration and a 20 µs settling delay after the lock bit is observed;
- clears the temporary lock-control bits after polling.

There is a material build-configuration ambiguity: the source defines `LOCKBIT(x)` as bit 31 for non-EVB builds, but as bit 28 for `CONFIG_EVB_PLATFORM`. The PLL descriptor passes `LOCKBIT(28)` for the lock-status bit. Consequently, the lock-status bit used by the built driver depends on its compile-time platform configuration; it must not be guessed from this source file alone. The same register also carries the boot PLL enable and new-lock-mode control semantics, so the runtime configuration and silicon manual must be reconciled before any port.

The factor setter uses its configured field layout and lock helper; it does not clear PLL enable bit 31 in its normal rate-change path. The source also contains other generic PLL enable/disable paths. Do not copy those paths without isolating and proving their exact behavior for the CPU PLL.

### Required items before translating this sequence into C HAL

- Confirm that the exact RG34XX live FDT and silicon revision match the pinned `sun50iw9` clock model.
- Confirm the physical CCU base and access permission at BL33 entry; validate the current `PLL_CPU` and `CPU_CFG` readbacks.
- Identify the actual `pll_periph0` rate and prove it remains available and safe during the transition.
- Confirm CPU mux, AXI/CPUAPB divider values, and all downstream bus constraints.
- Confirm the selected lock mode, lock-status bit and timeout against the actual build configuration and SoC documentation.
- Prove timer/monotonic-clock continuity across both parent switches and PLL relock.
- Prove CPU1–CPU3 are parked or otherwise safely coordinated before changing the shared CPUX clock.
- Implement bounded timeout, rollback/fallback, readback and fault reporting only after those checks pass.

## 5. Shared cluster clock and OPP implications

In the pinned `sun50iw9p1.dtsi`, CPU0–CPU3 are Cortex-A53 nodes that reference the same `clk_pll_cpu` and carry `opp-shared`. This supports treating the CPU frequency control as a **shared cluster policy**, not four independent per-core MHz controls, in the architecture and UI.

This is still not live confirmation of the RG34XX BL31→BL33 handoff state. The project remains single-core by policy until CPU1–CPU3 state is proven. Do not write the shared PLL while secondary-core execution is unknown.

## 6. The vendor CPU PLL factor table is explicitly not reliable

The first comment in `clk-sun50iw9_tbl.c` says:

> the clk table is not right, please fix me when ic come back

This warning invalidates the table as an OPP authority. It also contains only clock-factor/frequency data, not a verified RG34XX CPU-rail voltage relationship. Therefore:

- Do not extract a supported MHz list from `factor_pllcpu_tbl` for the RG34XX runtime profile.
- Do not combine these factor tuples with AXP2202 DCDC1 voltages inferred from another DTS.
- Do not use this table to approve Performance or Boost, even if a displayed rate appears plausible.
- Use only a board-specific OPP grid whose frequency/voltage pairs, speed-bin mask, thermal ceiling, PMIC readback and stability are independently established.

The pinned H616 mainline OPP table remains a comparative software reference, not a replacement for an RG34XX approved OPP list.

## 7. Current project impact

This source review narrows the clock-control investigation and replaces “unknown sequence” with a **documented vendor-driver candidate sequence**. It does not close the hardware gate.

| Item | Result after this audit |
|---|---|
| CPUX PLL and CPU mux offsets | Source-level references identified: `0x0000` and `0x0500` |
| CPU parent-switch behavior | Vendor pre-rate alternate-parent / post-rate restore flow identified |
| PLL field model | N/M/P shifts and output-gating field identified |
| Lock protocol | New-mode flow identified; status bit remains build-config dependent |
| PLL enable bit | Source explicitly says boot-enable bit 31 must not be dynamically changed |
| CPU shared-domain semantics | `opp-shared` and shared `clk_pll_cpu` in reference DTS |
| Usable OPP frequency/voltage grid | **Not established**; factor table is explicitly marked incorrect |
| Actual RG34XX register values / clock parent | **Not read from the target** |
| CPU1–CPU3 BL33 state | **Not proven** |
| DVFS write authorization | **Not approved** |

No code path under `platform/h700/power` was changed by this audit. The C service remains `POLICY_ONLY`, `hardware_applied = 0`, and the Linkslot CPU Performance QuickRow remains visibly unavailable and inert.

## 8. Next work

1. Regenerate the RG34XX read-only inventory and capture the complete live FDT plus CPUFreq/regulator/thermal/clock state.
2. Add only source- and permission-verified read-only register capture for `PLL_CPU` / `CPU_CFG`; first establish the live CCU mapping and a safe read method. Do not add MMIO writes.
3. Extend the BL31→BL33 audit to prove CPU1–CPU3 state and the timer/monotonic-clock path.
4. Reconcile lock-bit behavior with the exact shipping kernel/U-Boot configuration and authoritative H700 register documentation.
5. Only after all evidence is complete, design the voltage/clock transition state machine and its host failure-injection tests. Keep the runtime unavailable until board OPP approval.
