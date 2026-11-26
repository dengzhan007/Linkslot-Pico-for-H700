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
| CPU shared-domain semantics | Reference DTS has `opp-shared`; live Linux has CPU0–CPU3 in one cpufreq policy |
| Current Linux CPU parent | **Observed:** `pll_cpu → cpu → cpuapb`; `cpu=pll_cpu` in the capture |
| Current Linux backup PLL | **Observed:** `pll_periph0=600 MHz`, 4/4 prepare/enable users; separate tree at capture |
| Synchronized instantaneous CPU rate | **Not yet captured:** inventory v1 read cpufreq and `clk_summary` at different times; pinned cpufreq-dt says `cpuinfo_cur_freq` reads `clk_get_rate()`, while `scaling_cur_freq` is cached policy state |
| CPU supply | **Confirmed in live FDT/runtime framework:** AXP2202 DCDC1, CPU0 consumer at 1160 mV in captured sample |
| Usable OPP frequency/voltage grid | **Not established**; SID raw `0x6c00` is mainline candidate bin 5 only; factor table is explicitly marked incorrect |
| Actual PLL/CCU register values at BL33 | **Not read**; Linux `clk_summary` is not MMIO readback |
| CPU1–CPU3 BL33 state | **Not proven**; all four CPUs are online only in the later Linux runtime capture |
| DVFS write authorization | **Not approved** |

No code path under `platform/h700/power` was changed by this audit. The C service remains `POLICY_ONLY`, `hardware_applied = 0`, and the Linkslot CPU Performance QuickRow remains visibly unavailable and inert.

### 7.1 RG34XX live Linux evidence (2026-11-25)

The new read-only capture is summarized in [RG34XX CPUX live audit](rg34xx-cpux-live-audit-2026-11-25.md), with raw report files under [reports/rg34xx-inventory-20261125-cpux](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux).

The follow-up same-version Linux 4.9.170 cpufreq/OPP source check is recorded in that live audit §1.5. The candidate vendor cpufreq-nvmem source reads the two-byte speed cell as `u32 *` without checking length; `0x6c00` falls through to default `supported_hw=0x4`, which predicts voltage-bearing OPPs at 480/600/792/1008/1200/1344 MHz, not the captured runtime list. Because the running binary has not been matched to this source revision and the predicted set disagrees, this source candidate cannot provide the live OPP mask.

- Live FDT resolves CPU0 `cpu-supply` phandle `0x4f` to `axp2202-dcdc1`, declared 0.5–1.54 V, `always_on` and `boot_on`. Linux regulator debugfs identifies its consumer as `cpu0` and reports 1.16 V in this sample. This is regulator-framework evidence, not raw PMIC-register readback at BL33.
- The captured Linux clock tree is `pll_cpu → cpu → cpuapb`; the reported rates are 1512 / 1512 / 378 MHz, with AXI 756 MHz. `pll_periph0` is a distinct 600 MHz parent tree with four prepare/enable users; it is not the current CPU parent in this snapshot.
- Linux reports CPUs 0–3 online and one cpufreq policy affecting all four. This reinforces the shared-clock design; it does not determine secondary-core state at BL31→BL33.
- cpufreq sysfs reports 1416 MHz on the per-CPU nodes (and 1320 MHz in a later policy0 read), while a later `clk_summary` reports `pll_cpu=1512 MHz`. The collector v1 did not bracket these reads with a shared sample timestamp. A same-version cpufreq-dt source candidate returns `cpuinfo_cur_freq` through `clk_get_rate()` and `scaling_cur_freq` from cached `policy->cur`; schedutil may change the clock between reads. Do not conclude a persistent provider mismatch until the new paired samples are collected.
- The DT declares a shared OPP table with several voltage variants. The raw SID speed cell is `0x6c00`; mainline diagnostic decoding suggests candidate bin 5, but vendor literal matching fails and `opp_authorization` remains `not_approved`. The 1416 MHz / 1160 mV observation is numerically consistent with one DT variant only; it does not prove that the corresponding speed-bin mask was selected.
- The thermal calibration bytes exist, but the kernel reports `avcc is not calibrated`; the temperature readings are diagnostic only and cannot drive closed-loop DVFS.

These observations reduce uncertainty about the running Linux configuration, but they do not replace BL33-entry register/rail readback, the exact Linux 4.9.170 OPP implementation audit, timer continuity, or secondary-core handoff proof.

## 8. Next work

1. Audit the exact shipping Linux 4.9.170 cpufreq/OPP code path against the live DT table and raw SID `0x6c00`; do not treat the mainline candidate bin or voltage correlation as approval.
2. Re-run the inventory's six-sample uptime-bracketed cpufreq/CPUX capture. Compare `cpuinfo_cur_freq` (clock-framework `clk_get_rate()` read in the pinned cpufreq-dt candidate), cached `scaling_cur_freq` and the adjacent `clk_summary` rates; v1 measurements were taken at different times.
3. Extend the BL31→BL33 audit to prove CPU1–CPU3 state, the single-core invariant and timer/monotonic-clock continuity.
4. Reconcile the PLL lock-status bit with the exact shipping kernel/U-Boot configuration and authoritative H700 register documentation; distinguish Linux framework clock data from direct register readback.
5. Resolve AVCC/thermal calibration or establish an independently verified sensor path. Only after all evidence is complete, design the transition state machine and host failure-injection tests; keep the runtime unavailable until board OPP approval.


## Matched BL31 CCU helpers: separate raw PSCI ops-table code regions

BL31 audit #33 confirms a conditional CCU save helper call from the function
entered through raw ops-table slot 3 (`0x48002878 → 0x48001790`, callsite
`0x48002998`) and a conditional CCU restore helper call from the separate
function entered through raw slot 5 (`0x480026c4 → 0x480018d8`, callsite
`0x48002858`). The latter tail-branches at `0x48002874` to raw slot 4
(`0x48002680`).

This is stronger static ownership evidence than isolated BL target hits, but
the exact shipping TF-A callback ABI remains unpinned, so slots 3/4/5 are not
assigned semantic names. These helper paths are not a CPU DVFS interface,
not evidence of a BL33 startup clock transition, and not proof of runtime
suspend/resume execution. Do not enable the Linkslot CPU-frequency service
or write CCU/PLL/PMIC registers based on these findings.
