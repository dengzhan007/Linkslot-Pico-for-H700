# Kernel #19 PLL factor-table consistency audit

**Branch:** `porting/linkslot-1.0.45-feasibility`  
**Inputs:** existing Build-ID-matched `build/rg34xx-kernel.elf`, `reports/kernel19-display-reverse-audit/ADDENDUM.md`, and current `verification.json`  
**Purpose:** resolve the remaining internal inconsistency in the PLL table summary before using the stated maximum or quantization in any 120-Hz design calculation.

## Hard constraints

- Offline ELF and static code analysis only. No device connection, sysfs/debugfs/vendor getter, MMIO, `/dev/mem`, `sunxi_dump`, register/clock/PMIC writes, p4 re-extraction, boot writes or game execution.
- ELF GNU Build ID must be exactly `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68`; otherwise stop.
- Use an in-memory `.rela.dyn` relocation overlay only. Do not write a relocated ELF to disk.
- Preserve the first-round immutable files; write a new addendum and update only the current verification/manifest as required.

## 1. Reconcile the table size, range and endpoint

The current `ADDENDUM.md` and `verification.json` contain conflicting arithmetic:

- They claim `factor_pllvideo0x4_tbl` has 187 entries, 288–2520 MHz, step 12 MHz.
- They also identify the last word as `0x00006400`, N=100, D2=0.
- Under the same audited formula `24 MHz × (N+1)/(D2+1)`, that endpoint is 2,424 MHz, not 2,520 MHz.
- 187 evenly spaced entries from 288 MHz in 12-MHz steps would end at 2,520 MHz.

Do not choose one statement by assumption. Re-read the actual table through the relocation overlay and emit the **complete 187-row table** (or prove the actual entry count differs). For every row include:

`index, table_address, factor_word, N, D2, calc_rate_hz, stored_rate_hz, delta_hz`

Then report:
- exact first and last entry addresses;
- exact entry count and array bounds;
- actual minimum/maximum stored rate;
- whether rows are sorted, unique and uniformly spaced;
- all non-12-MHz deltas, duplicate rates or anomalous entries;
- whether the `0x6400` row is truly the final entry and whether `2424 MHz` is the maximum.

Cross-check the table symbol boundary and adjacent descriptor at `0xffffff800903deb0`; do not count the descriptor as a table entry.

## 2. Prove the rate-selection/rounding behavior

Recover the full control flow of `get_factors_pll_video0`, its helper(s), and the `clk_set_rate` path for the active `tcon_lcd` clock. Do not infer "rounds up" from the first/last table entries alone.

Show the selected factor word and actual table rate for:
- baseline request 286 MHz (`26 MHz × divider 11`);
- 120-Hz arithmetic request 572 MHz (`52 MHz × divider 11`);
- nearest candidate table entries below and above each request.

Clarify whether the clock framework/helper selects floor, ceil, nearest, or another constrained rate. If a request maps to 576 MHz, identify the exact row and factor word. If that cannot be proven, label it unresolved.

## 3. Correct the published verification

The top-level portion of `reports/kernel19-display-reverse-audit/verification.json` still contains superseded first-round claims (e.g. `lcd_type` indexing, old 288–336-MHz range, old PLL discrepancy, and obsolete unresolved items). Update it so a reader consuming JSON alone sees the corrected state. Keep a clearly labelled `first_round_snapshot` only if historical preservation is needed.

Update:
- PLL table count/range/step and all endpoint examples;
- 120-Hz evidence and missing items (do not state PLL is a blocker if the exact table proves a sufficient rate is supported);
- parent/mux conclusion (candidate list recovered; active parent remains runtime-readback evidence; driver captures/reapplies the same parent);
- divider source (`lcd_if` table plus `lcd_dclk_freq == 26` override);
- current unresolved list (clk handle acquisition, LCD_OPEN_FUNC executor, panel_info+0x118 dead field, PLL descriptor side-field semantics, and `lcd_type=old` linkage).

The report's `generated_at` for the addendum currently reads `2026-11-26T00:20:00+08:00`, later than the reports-branch commit timestamp on 2026-11-25 at 22:49 +08:00. Correct the timestamp from the actual generation/commit metadata or omit it; do not leave a future timestamp.

## 4. Deliverables

Add `reports/kernel19-display-reverse-audit/PLL-TABLE.md` and `pll-factor-table.json` with the full data and algorithm analysis. Update `verification.json`, `ADDENDUM.md` (new section or second addendum), and `SHA256SUMS.txt`. Keep the first-round `README.md`, `symbols.json`, and `bounded-disassembly.txt` untouched unless a separately versioned correction is necessary.

Also update `reports/README.md`, `docs/PROJECT-STATUS.md` and `docs/roadmap.md` with the resolved facts, remaining risks, and accurate CI state. The Linkslot 1.0.49 complete feasibility workflow `45631521531` is known to have completed successfully (71/71 steps); do not describe it as still running.

No 120-Hz writer or mode enablement is authorized by this task.
