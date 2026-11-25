# Kernel #19 display audit follow-up task

**Branch:** `porting/linkslot-1.0.45-feasibility`  
**Input:** existing `build/rg34xx-kernel.elf` and the uploaded `reports/kernel19-display-reverse-audit/`  
**Purpose:** close the remaining panel/clock lifecycle questions and correct any over-interpreted field labels in the first report.

## Hard constraints

- Offline ELF/static analysis only. Do not connect to the RG34XX or invoke sysfs/debugfs/vendor getters.
- No MMIO read/write, `/dev/mem`, `sunxi_dump`, TCON/CCU/PLL/PMIC changes, p4 re-extraction, boot writes, or game execution.
- ELF must retain GNU Build ID `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68`; if absent/mismatched, stop.
- Do not copy another panel profile's values. Preserve all existing reports; deliver a versioned addendum.

## 1. Correct the `disp_al_lcd_get_clk_info` interpretation

The first report labels the word at `[x19 + 0x80]` a SoC/version word. Verify that claim before retaining it.

The published disassembly at `0xffffff8008434f94` shows:

- `mov x19, x2` at `0xffffff8008434fd8`;
- all later loads from `[x19]`, `[x19 + 0x30]`, `[x19 + 0x48]`, `[x19 + 0x80]`, and `[x19 + 0x118]` therefore address the function's third argument;
- `disp_al_lcd_cfg` calls it with `x2 = x23`, where `x23` is the panel/config argument passed to `disp_al_lcd_cfg`.

So `[x19 + 0x80]` is in that panel/config object unless a separate pointer indirection is proven. Identify the exact member using the target binary's parser/structure construction, field accessors, and call sites. Do not call it a SoC version register merely because the comparison constant is `0x1a`.

In particular, trace whether `lcd_dclk_freq=26` feeds this field, another part of the divider path, or a later clock-rate calculation. Narrow the existing claim to: “the value is not used as the arithmetic input to the low-level divider write itself” unless the wider path proves more.

## 2. Recover the complete panel lifecycle

Trace and bound the functions/callers for:

- panel driver registration and `lcd_driver_name` selection;
- `lcd_type=old` parsing and the value passed to `disp_al_lcd_get_clk_info`;
- panel power / GPIO / regulator callback;
- panel init / open / enable / disable callbacks;
- `lcd_clk_init`, `lcd_clk_config`, `lcd_clk_enable` or target-kernel equivalents;
- `disp_al_lcd_cfg`, TCON timing/config, source selection, and backlight enable.

Provide the real caller-to-callee order for the active `rg34xx_v1` profile, not only the inner order within `disp_al_lcd_cfg`. If callback-table pointers cannot be reconstructed because the ELF has zeroed relocation fields, show the code evidence and mark the missing edge unresolved.

## 3. Complete clock-parent and PLL audit

- Search all writers/callers for `clk_set_parent`, `clk_set_rate`, mux/source selection, and the `tcon_lcd` clock.
- Establish whether Linux changes the `tcon_lcd` parent/rate during LCD initialization or simply inherits the bootloader-selected clock. If the binary does not prove this, keep the live `clk_summary` observation separate from the code-side conclusion.
- Recover the exact PLL factor descriptor fields where possible: N/M/P/D1/D2, fractional mode, enable/gate bits and offsets. `calc_rate_video0` proves only the fields it actually reads; do not infer bit27 or bit31 semantics from convention.
- Explain the 2x mismatch between `factor_pllvideo0x4_tbl` rates and `calc_rate_video0` only if the descriptor/table relationship can be proven. Otherwise retain it as unresolved.

## 4. 120-Hz conclusion

Re-check whether any alternative timing/mode is explicitly associated with `rg34xx_v1` and `lcd_type=old`. A generic string, a different panel's timing, or an arithmetic candidate is not support. If any required panel timing, clock parent/rate, accepted divider, VBlank/TE transition boundary, panel tolerance evidence, or rollback path is missing, keep the verdict **not verified / do not enable**.

## 5. Deliverables

Add an `ADDENDUM.md` in `reports/kernel19-display-reverse-audit/` and update `verification.json` with:

- verified struct member identity (or explicitly unresolved) for offsets `0x30`, `0x48`, `0x80`, and `0x118`;
- bounded function listings and the panel lifecycle call graph;
- exact PLL/mux findings and remaining uncertainty;
- corrections to any wording in the initial README;
- a fresh `SHA256SUMS.txt` covering README, symbols, bounded disassembly, verification and ADDENDUM (not self-including).

Keep the original first-round evidence traceable. Update the project status only after the addendum passes its own consistency checks. No hardware test is requested or authorized.
