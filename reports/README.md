# RG34XX read-only inventory reports

## Index

### 2026-11-27 — RG34XX fixed DRAM-source PFN metadata (read-only)

- [`rg34xx-dram-pfn-20261127/`](rg34xx-dram-pfn-20261127/) — `scripts/collect-rg34xx-dram-pfn-readonly.sh` (porting branch `3f61baa`) run as root (`adb shell id -u` = 0; no escalation, no boot-argument change) against the page holding the stock BL31 DRAM source window `[0x40000000, 0x40000140)`. Page size 4096, PFN `0x40000` (262144), `kpageflags` at byte offset 2 097 152. `/proc/kpageflags` and `/proc/kpagecount` were both readable (raw `6858000000000000` and `0100000000000000`), `page_owner` not present; `CONFIG_PROC_PAGE_MONITOR=y` and `CONFIG_DEBUG_VM=y`. The output deliberately leaves flag decoding deferred until the matching 4.9.170 flag definitions are checked, and the script itself notes that `H700_MEMORY_LAYOUT_APPROVED` remains unset, so no ownership approval is implied. No DRAM contents, SMC, MMIO or suspend operation. Ships the verbatim output and `SHA256SUMS.txt`; no serial, MAC or host path.

### 2026-11-26 — RG34XX kernel-module / MMIO preflight (read-only)

- [`rg34xx-kmod-preflight-20261126/`](rg34xx-kmod-preflight-20261126/) — what would be needed to read `SYS_CFG` from kernel context, now that the userspace `/dev/mem` path fails with `EFAULT` on this build. `CONFIG_MODULES=y`, `CONFIG_MODULE_UNLOAD=y`, `CONFIG_KALLSYMS=y`, `CONFIG_DEVMEM=y`, `CONFIG_STRICT_DEVMEM` not set — but `/lib/modules/4.9.170` has **no `build` link, no `Makefile`, no `Module.symvers`**, and `/usr/src/linux*` is absent, so an out-of-tree module cannot be compiled against the running kernel from this image. The `syscon` platform driver is present, which is the relevant accessor for a `SYS_CFG` regmap, but `/sys/kernel/debug/regmap` currently exposes only `5-0034`, `5070400.thermal_sensor`, `5096000.codec`, `5097000.ahub_dam_plat`. Filtered `T`/`t` symbols present: `__ioremap`, `devm_ioremap_resource` (no exact `ioremap`/`iounmap`/`readl`/`regmap_init_mmio` names). `/proc/iomem` reserves only the pinctrl sub-range `0300b000-0300b3ff` and other peripherals — the `SYS_CFG` block at `0x03000000` itself has **no reserved resource entry**, so a kernel accessor would map it explicitly. Ships the verbatim 64-line output, the exact script, and `SHA256SUMS.txt`; no full `/proc/kallsyms`, serial, MAC or host path.

### 2026-11-26 — RG34XX RVBAR selector probe (read-only, no JSON: `/dev/mem` EFAULT)

- [`rg34xx-rvbar-selector-20261126/`](rg34xx-rvbar-selector-20261126/) — `scripts/read-h700-rvbar-selector-readonly.py` (repo `fbab663`, md5 `2739be35…2d73`) was pushed to `/tmp` and run as root; the image has no `python3` on PATH, so the portable CPython 3.12.16 bundled with the LEI-V4 frontend was used. The script's single 32-bit `os.pread()` of `/dev/mem` at `0x03000024` returned **`[Errno 14] Bad address`**, exit 2, **0 bytes of stdout — no JSON exists and none was fabricated**. An independent `dd` read at the same address reproduces `EFAULT`, consistent with the offset-independent `/dev/mem` failure documented in `rg34xx-pio-input-capture-20261126/`; `CONFIG_STRICT_DEVMEM` is not set, so the usual devmem gate is not the cause. Read-only alternatives were enumerated but not substituted (debugfs has no sunxi register dump; `regmap` exposes only four unrelated devices), and no mount/module/boot-arg/register write was attempted. Publishing the verbatim run log and a note on what would unblock the read (in-kernel `SYS_CFG` read, or a pre-BL33 capture — a post-boot sample cannot prove CPU reset/RVBAR state by design).

### 2026-11-26 — RG34XX full inventory with FDT interrupt/timer declarations (read-only)

- [`rg34xx-inventory-20261126-timer/`](rg34xx-inventory-20261126-timer/) — complete output of `scripts/run-rg34xx-inventory.ps1 -OutputDirectory .\build\rg34xx-inventory-20261126-timer` at repo `8e7f8db` (41 captured files + `REDACTION.md` + `SHA256SUMS.txt` = 43). Read-only `/dev/mmcblk0` capture, TOC1 SHA-256 `e6c20e3a…4604` and all four images verified; both boot0 copies parse as LPDDR4 672 MHz. The expanded FDT inspector now preserves interrupt-controller and timer declarations: 5 controllers (NMI, wakeupgen, GIC, and both pinctrl nodes) and 2 timers — `allwinner,sun4i-a10-timer` at `0x03009000` (IRQ specifier `0,48,4` via wakeupgen) and `arm,armv8-timer` with four PPIs via the GIC. `live-fdt.dtb` (143 936 B), `live-fdt.json`, `interrupts.txt` and `summary.txt` are all present, together with the full boot-chain, partition, memory, display and input captures. **Published redacted:** the device serial occurred twice in `cmdline.txt`, twice in `live-fdt.json`'s boot args, and twice inside `live-fdt.dtb` (`snum=` / `androidboot.serialno=` at byte offsets 94984 and 95105); all six occurrences were replaced by equal-length `X` strings, local absolute Windows paths in three JSON `file`/`source` fields were made relative, and the JSON's DTB hash was refreshed to match the masked DTB. `REDACTION.md` records every original hash and the resulting DTB hash `620b36d8…42e`; the masked DTB re-parses to the same 573 nodes / 5 controllers / 2 timers. Byte-level rescan of all published files: 0 serial, 0 MAC, 0 workstation path or user name. `SHA256SUMS.txt` covers 42 files, all verified against the committed blobs. No register or block-device write.

### 2026-11-26 — RG34XX physical key test via gpiolib debugfs (read-only, completed)

- [`rg34xx-gpio-debugfs-20261126b/`](rg34xx-gpio-debugfs-20261126b/) — after raw `/dev/mem` PIO sampling proved impossible on this kernel (see the entry below), the 17-key test was completed against the already-mounted `/sys/kernel/debug/gpio` view using `scripts/collect-rg34xx-gpio-debugfs-readonly.sh` (repo `3ce6bb3`). Root ADB (`id -u` 0), 1200 samples over a 126.0 s `/proc/uptime` window, `cat` only — no debugfs mount, no GPIO export/request, no register/pinmux/GPIO/EINT/GIC write. All 17 physical keys produced one clean `hi → lo → hi` transition each (34 transitions, 17 distinct gpiolib lines, no duplicates and no simultaneous lines), power key excluded. The **X/Y silkscreen swap** is confirmed a third time: physical X drives `gpio-2` (`GPIO Key y` / DT `keyY` / `PA2` / evdev `307`) and physical Y drives `gpio-3` (`GPIO Key x` / DT `keyX` / `PA3` / evdev `306`). PE0–PE3 map to `gpio-128…131` and carry Down / Vol+ / Vol− / Menu, again contradicting the `pinmux-pins` claim that PE0/PE2/PE3 are `csi1`. A first 600-sample attempt covered only 12 keys and was discarded (kept locally, not published); everything here is one uninterrupted 1200-sample run. Ships the three requested device artifacts (`preflight-gpio.txt`, `gpio-debugfs-samples.txt`, `summary.txt`), plus `GPIO-DEBUGFS-TEST.md` with the per-key sample/uptime table and `SHA256SUMS.txt`. No serial, MAC, host user name or host path in any published file.

### 2026-11-26 — RG34XX raw PIO input capture (read-only, FAILED at first read)

- [`rg34xx-pio-input-capture-20261126/`](rg34xx-pio-input-capture-20261126/) — attempt to sample `PA_DAT` (`0x0300b010`) and `PE_DAT` (`0x0300b0a0`) while pressing each of the 17 gamepad keys once, using `scripts/collect-rg34xx-pio-input-readonly.sh` (repo `be8523e`). The collector ran exactly as specified — `adb shell id -u` returned `0`, so the root branch was used, `/dev/mem` was readable at the inode level, the DTB pinctrl resource `0300b000-0300b3ff : /soc@03000000/pinctrl@0300b000` was present in `/proc/iomem` — but the **first** 4-byte read returned `dd: /dev/mem: Bad address` (EFAULT) and the script failed closed with `EXIT=1` at sample 0. `pio-samples.csv` therefore holds only its header and **no keys were pressed**: the 17-key capture did not happen. A read-only probe matrix shows the refusal is offset-independent — offsets `0`, `4`, `0x03000000`, `0x0300b000`, `0x0300b010`, `0x0300b0a0`, a whole page and a 1-byte read all fail identically — so it is a property of this kernel build's `/dev/mem` translation path, not of the two registers. The usual gates are excluded by the device's own config: `CONFIG_STRICT_DEVMEM` is **not set** (which would have yielded `EPERM`, not `EFAULT`), `selinux=0`, the range lies outside the `System RAM` node that starts at `0x40000000`, and `dmesg` shows no `ioremap` warning, oops or panic. No workaround was attempted: no kernel protection disabled, no alternative memory node substituted, no module load, no boot-argument change, no register/pinmux/GPIO/EINT/GIC write — `hardware_applied = 0`. Ships the verbatim device artifacts (`summary.txt`, header-only `pio-samples.csv`, `pa-read-error.txt`), the probe matrix, the config grep, the `/proc/iomem` view, the dmesg tail and `PIO-CAPTURE-FAILURE.md` with reproduce steps; the kernel-side counterpart `platform/h700/runtime/h700_rg34xx_input_scan.c` from the same commit range is noted as the viable next step and was not run.

### 2026-11-26 — RG34XX input runtime + physical key event test (read-only)

- [`rg34xx-input-runtime-20261126/`](rg34xx-input-runtime-20261126/) — collector `input-runtime.txt` from a fresh inventory re-run plus a full physical key test against `/dev/input/event1` (`ANBERNIC-keys`, `gpio-keys-polled`, handlers `kbd js0 event1`). `getevent` does not exist on this image, so the capture is `adb shell "cat /dev/input/event1"` into a local binary (byte transparency first proven by an md5 comparison on `/bin/sh`), decoded offline as 24-byte `struct input_event`. All 17 device-tree keys produced real events: the 13 `EV_KEY` codes 304–315/114/115 and both `EV_ABS` axes 16/17 in both directions with −1/+1 press and 0 release — so the d-pad must be handled as two hat axes, not four buttons. One extra R1 group sits between START and MENU (operator confirmed an accidental press). The X/Y ordering anomaly in the first pass was re-tested head-to-head in [`XY-REPEAT.md`](rg34xx-input-runtime-20261126/XY-REPEAT.md) and resolved as a **label swap, not an ordering mistake**: the key silkscreened X reports `307` (DT `keyY`/`PA2`) and Y reports `306` (DT `keyX`/`PA3`), 6/6 presses, so any keymap assuming `306`=X / `307`=Y is inverted on this unit. The test also surfaces three static-vs-runtime mismatches: `pinmux-pins` reports `PE0/PE2/PE3` as `function csi1` although `keyDOWN`/`keyvol-`/`keyMenu` are declared there and all three still emit events; `PA0–PA12` read `(MUX UNCLAIMED)` while `pinconf-pins` shows `input bias disabled`; and a third device `dierct-keys-polled` (`input2`/`event2`) exists outside the device-tree `gpio_keys` node and was not touched. Read-only throughout — no `EVIOCGRAB`, no keymap change, no `/dev/input` write, no register write, power key excluded.

### 2026-11-26 — RG34XX live-FDT `input_devices` (expanded FDT inspector, read-only)

- [`rg34xx-input-fdt-20261126/`](rg34xx-input-fdt-20261126/) — re-run of `scripts/run-rg34xx-inventory.ps1` against the connected RG34XX with root ADB, using the expanded `tools/inspect_h700_fdt.py` that now emits an `input_devices` section. The shipped input path is the polled `gpio-keys-polled` node `/soc@03000000/gpio_keys` with 17 keys on `active_low`, unpulled, drive 2, mux 0 inputs through `&pio` (phandle `0x4b`, `#gpio-cells = 6`); both analog key controllers are inactive (`gpadc` `disabled`, `keyboard` `disable`), so there is no ADC key path. `keyMenu` is `linux,code = 312` (`BTN_TL2`) on `PE3`, which independently confirms the earlier evdev finding that the Menu/exit key is 312 and not `BTN_MODE` (316). The d-pad is two `EV_ABS` axes (code 16 and 17, values -1/+1), not four `EV_KEY` buttons, and `keyL2`/`keyR2` sit on 314/315, so those codes must not be read by their nominal kernel names. Ships `INPUT-DEVICES.md`, the `input_devices`-only extract `live-fdt-input-devices.json` (DTB SHA-256 `c17f896f…`, 143 936 B, 573 nodes), `sensitivity-scan.txt` and `SHA256SUMS.txt`. `chosen_bootargs` (carrying `snum=` and `androidboot.serialno=`) and the local build path were dropped before publishing; the scan records 0 serial / 0 MAC / 0 workstation-path hits in every published file. Static device-tree declarations only — no key injection, no `/proc/bus/input/devices`, no register write.

### 2026-11-26 — Slot 120 Hz runtime check (ADB, read-only)

- [`slot-refresh-audit/`](slot-refresh-audit/) — two read-only ADB captures of the same RG34xx (BaseOS 1.5.3, Linux `4.9.170 #19`), UI then a running GBA core, 12 s IRQ windows in each. The panel block is byte-identical in both phases: `rg34xx_v1` 720x480, `lcd_ht` 820 / `lcd_vt` 536 / `lcd_dclk_freq` 26 MHz / `lcd_if` 0, so the tree carries only the 60 Hz timing set. IRQ 293 measures 59.667 Hz in UI and 59.583 Hz in game; `disp` reports `de_rate` 696 MHz, `ref_fps` 59 and `fps:60.2` in both. The PLL factor word at `0x03001040` reads `0x88001703` in both phases (N=23, D2=1, 288 MHz), i.e. no 120 Hz programming, and `0x06511044` answers `I/O error` from the dump node, so the TCON divider has no runtime readback. `slot` keeps PID 2301 across phases and only dlopens `armgba_libretro.so`, so no display re-init happens on game entry; `high_refresh=1` in `slot.state` in both phases with no effect on the display stack. Verdict: 120 Hz is not applied, and both phases sit at the 60 Hz path. Ships `SUMMARY.md` plus both raw reports and the collector script; no `sunxi_dump` write, no register write, no `adb root`, no process kill and no input injection.
- [`slot-refresh-audit/binary-identity/`](slot-refresh-audit/binary-identity/) — provenance check of the binaries actually running on that unit. The official `v1.16.0-pvaibhav.2-h700.zip` matches its published SHA-256 (`1f9bdaa1…`), and the device copies of `System/slot` (`8f0fd40d…`) and `System/armgba_libretro.so` (`4ebae098…`) are byte-identical to the archive; three other cores and two launcher scripts match as well, all carrying the archive's own mtime, so nothing was rebuilt locally. The armgba core traces to `armGBA` @ `df6b2631…`, built `-mcpu=cortex-a53` with a single patch whose SHA-256 equals the one its `.meta` declares. Both ELFs keep `.symtab` (37 201 symbols in `slot`, 4 960 in the core), so attribution is by name rather than heuristic; the core's 47 gba-named symbols are all LOCAL and none is exported, while its 24 libretro entry points are defined in both tables, and `slot` exports none of them and resolves them at runtime. Runtime side: PID 2301, stdout and stderr both land in `Logs/slot.log` (same inode, identical md5), `SLOT_TRACE_LATENCY` is unset and `System/trace-latency.on` does not exist, so latency tracing was off and was left off. An addendum disassembles the `high_refresh` gate: `set_high_refresh` returns 0 without touching the panel while the sentinel field at `self+0xb8` is set, which is consistent with `high_refresh=1` producing no visible change. No `readelf` exists on the host or the device, so the four requested views were reproduced with pyelftools 0.33 and the raw dumps are included. Read-only throughout: no `/proc/PID/mem`, no trace marker created, no restart, no config change, no register write.

### 2026-11-25 — RG34XX kernel #19 display-path static reverse audit (offline)

- [`kernel19-display-reverse-audit/`](kernel19-display-reverse-audit/) — Build-ID matched stock ELF, GNU Build ID `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68`; bounded PLL/TCON disassembly resolves the live 288-MHz / divider-11 / ~59.57-Hz baseline. No 120-Hz mode table or runtime refresh-switch path found. The first pass left the `tcon_lcd` code-side mux, the full panel callback lifecycle and the field at `+0x80` unresolved. No device access or hardware writes.
- [`kernel19-display-reverse-audit/ADDENDUM.md`](kernel19-display-reverse-audit/ADDENDUM.md) — second round, offline only; the three first-round evidence files remain unchanged. A 91,760-entry in-memory `.rela.dyn` overlay recovers the panel and clock pointer tables. Confirms `panel_info+0x80=lcd_dclk_freq`, `lcd_if` table indexing, `lcd_dclk_freq==26` → divider 11, the `0xf` TCON upper bits from `tcon0_open`, the 17-entry panel callback table (`fog_fj035fhd05_v1_panel` registered as `rg34xx_v1`), and parent capture/re-apply in the LCD clock path. It corrects the first-round mechanism and recovers the lifecycle/rate flow. A third round then exported every PLL factor row and resolved that summary conflict: the table really is 187 rows from 288 to 2520 MHz in 12-MHz steps, and `0x6400` is interior row 178 (2424 MHz), not the final row. The 120-Hz verdict remains not verified / do not enable. The report records 82 self-checks passing; no device access or hardware writes.


### 2026-11-25 — RG34XX stock kernel identity + CPUFreq NVMEM probe reverse engineering (offline)

- `rg34xx-boot-p4-verified.json` confirms the uncompressed stock kernel SHA-256 `39cf8c3eb9a75ff0eda51b4074cd65860c5aa5386093bcaf69339819421d84b7` matches the public BaseOS survey prefix; GNU Build ID `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68` matches the live kernel.
- `rg34xx-cpufreq-driver-disasm.txt`, `rg34xx-cpufreq-full-cfg.txt/.json`, and `rg34xx-opp-core-disasm.txt` document the Build-ID-matched AArch64 ELF analysis: `sun50i_cpufreq_nvmem_probe` has 287 instructions / 69 strict basic blocks / 106 edges and ten explicit speed comparisons.
- Speed `0x6c00/0x7c00` takes the `b0/b1` prop-name path, keeps supported-hw mask at zero, and skips `dev_pm_opp_set_supported_hw`; suffix is `(sunxi_get_soc_ver() & 7) ? 1 : 0`. The live `soc_ver&7` and actual NVMEM over-read high halfword remain unverified. The probe reads a 2-byte cell buffer with a 32-bit load; treat this as a logical bounds-risk, not as proof of a runtime fault.
- Entire exercise is offline/static. No device-side vendor getter, `/dev/mem`, MMIO, PMIC/PLL writes, SD/boot writes or new hardware authorization.

### 2026-11-25 — pvaibhav/slot RG34XX UI vs GAME runtime capture (read-only, 30 s each)
- [`kernel19-display-reverse-audit/PLL-TABLE.md`](kernel19-display-reverse-audit/PLL-TABLE.md) — third round, offline only; all 187 factor rows are emitted with index, address, factor word, N, D2, computed rate, stored rate and delta, plus the recovered selection algorithm. Entry count is confirmed three independent ways (symbol gap `0x903deb0-0x903d8d8 = 187x8`, the `mov x4, #0xbb` immediate in `get_factors_pll_video0`, and the 1496-byte sibling `factor_pllvideo1_tbl`). Every row satisfies `24 MHz x (N+1)/(D2+1)` with zero deviation, the ladder is uniform 12 MHz, and N/D2 are **not** monotonic because the table alternates `D2=0`/`D2=1` encodings from row 14 on. The earlier claim that `0x00006400` was the final entry is corrected: it is row 178 at 2424 MHz, the real last row is `0x00006800` at 2520 MHz. Rate selection is clamp + nearest neighbour with ties going low. 286 MHz selects row 0 (288 MHz). The 572→576 MHz / divider-11 result is a forced-divider counterfactual, not the stock path when `lcd_dclk_freq=52` (stock divider=6, 312 MHz, 118.31 Hz). A separate forced-divider-10 / 528-MHz candidate yields ~120.13 Hz but is not panel-validated or stock-driver output. The 120-Hz verdict stays not verified / do not enable, with the clock source no longer the blocker. Full data in `pll-factor-table.json`; 47 self-checks pass. No device access or hardware writes.

- `pvaibhav-slot-runtime/` — two read-only runtime captures of the same RG34xx (`sun50iw9p1`, Linux `4.9.170`)
  taken back to back: `20261125-184406-ui` (27 samples / 29.6 s) and `20261125-184508-game` (23 samples /
  29.7 s). Display scanout IRQ rates are indistinguishable between phases — IRQ 292 `wakeupgen 88 dispaly`
  59.648 Hz (UI) vs 59.632 Hz (GAME); IRQ 293 59.614 vs 59.632 Hz. Driver-reported `fps` toggles between
  `60.2` and `59.8` in *both* phases, so it does not track the phase. `mgr0` mode line, `de_rate` 696 MHz and
  the live FDT blob (`6d1301e6…`) are identical in both rounds; panel is `rg34xx_v1` 720x480,
  `lcd_dclk_freq` 26 MHz, `ht` 820 / `vt` 536, `hbp` 46 / `hspw` 2 / `vbp` 20 / `vspw` 5. `high_refresh=1`
  in `slot.state` in both phases, with **no** observed mode change. GAME round additionally maps
  `armgba_libretro.so` + `libz.so.1` (PID 2842 unchanged; 895 vs 863 `maps` lines). No game FPS was measured —
  IRQ deltas are a scanout proxy only. Includes per-round `samples.json`, `interrupt-samples/`,
  `proc-2842/*` and a packaged zip; `live-fdt.dtb` and the serial stay local/redacted, and no `adb root`,
  mode-set, MMIO, `sunxi_dump` write or input injection was performed.

### 2026-11-25 — RG34XX/H700 read-only inventory (v5, display timing snapshot)

- `rg34xx-inventory-20261125-163152/` — fifth read-only inventory (`porting/linkslot-1.0.45-feasibility` @ `d7edaad`,
  collector `8b6852d0…`, same device `sun50iw9p1` / Linux `4.9.170 #19`). Adds `display-runtime.txt`:
  read-only vendor display snapshot (`de_rate 696000000`, `720x480 rgb`, `fps:60.2`, fb `stride 2880`,
  `bpp 32`) plus the display clock subtree (`pll_video2`/`tcon_tv` 297 MHz, `pll_video0x4`/`tcon_lcd`
  288 MHz, `display_top` 200 MHz). All five partition-level captures remain byte-identical to v1-v4;
  `kernel-driver-provenance.txt`, `memory-map-report.json` and `dmesg-hardware-lines.txt` are also
  byte-identical. `cpu-power-runtime.txt` differs only in sampled CPU rates (idle `1512000`), and
  `sid-cell-report.json` only in its `source_sha256`. This is a runtime snapshot: it does not prove
  `0x6c00 → b1`, authorize any OPP/clock/PMIC/display write, or close the temperature path. Serial
  redacted; binaries stay local.

### 2026-11-25 — RG34XX/H700 kernel + CPUFreq driver provenance (v4, build-id / OPP source)

- `rg34xx-inventory-20261125-cpux-v4/` — fourth read-only inventory (`porting/linkslot-1.0.45-feasibility`
  @ `b30fac3`, collector `0366c257…`, same device `sun50iw9p1` / Linux `4.9.170`). Adds
  `kernel-driver-provenance.txt`: kernel `4.9.170 #19 SMP PREEMPT 2026-05-26`, Linaro GCC 5.3-2016.05,
  `/sys/kernel/notes` SHA-256 `6065c1a7…` with GNU build-id `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68`,
  `CONFIG_CPUFREQ_DT=y` / `CONFIG_CPU_FREQ_STAT` unset, `cpufreq-dt` + `sun50i-cpufreq-nvmem` platform
  bindings, and filtered kallsyms names (addresses discarded) incl. `sun50i_cpufreq_*` and
  `dev_pm_opp_set_prop_name` / `dev_pm_opp_set_supported_hw`. All five partition-level captures remain
  byte-identical to v1-v3. Provenance narrows candidates but does NOT identify the shipping binary, prove
  `0x6c00 → b1`, or authorize any OPP/clock write. Serial redacted; binaries stay local.

### 2026-11-25 — RG34XX/H700 CPU clock, OPP, SID and BaseOS FDT handoff (v3, getter reconciliation)

- `rg34xx-inventory-20261125-cpux-v3/` — third read-only inventory (`porting/linkslot-1.0.45-feasibility`
  @ `d9da2e4`, collector `f386360e…`, same device `sun50iw9p1` / Linux `4.9.170`). All five partition-level
  captures (`boot0` ×2, `toc1`, `env`, live FDT) remain byte-identical to the 07:37 / 08:41 runs. New collector
  block performs back-to-back cpufreq getter reconciliation: re-reading the SAME sysfs attribute within tens of
  ms flips 1008 → 1104 MHz while `pll_cpu` and dcdc1 already track 1104 MHz, so the v2 "480 MHz request not
  applied" window is reclassified as a read-skew/cache artifact, not a governor clamp or clock-driver rejection.
  OPP table, SID (`0x6c00`, calib 2835/2841/2838/2842), thermal-not-calibrated and BaseOS FDT handoff findings
  are unchanged. Serial redacted; binaries stay local.

### 2026-11-25 — RG34XX/H700 CPU clock, OPP table, SID and BaseOS FDT handoff (v2)

- `rg34xx-inventory-20261125-cpux-v2/` — second read-only inventory (`porting/linkslot-1.0.45-feasibility`
  @ `7f07ea2`, collector `94cce9b7…`, same device `sun50iw9p1` / Linux `4.9.170`). All five partition-level
  captures (`boot0` ×2, `toc1`, `env`, live FDT) are **byte-identical** to the 07:37 run, so the read path is
  reproducible. New collector block samples cpufreq + `clk_summary` + `uptime` back to back: most
  frequency/PLL disagreements are read-skew across the switch, but one window of ≥1.1 s holds
  `scaling_cur_freq` at 480 MHz while `cpuinfo_cur_freq` and `pll_cpu` stay at 1008 MHz and dcdc1 tracks
  1008's voltage — request recorded, clock and regulator not applied. The OPP table is now fully decoded:
  table-level `nvmem-cells = <&speed>` plus per-OPP `opp-microvolt-{a1,b0,b1,c0,c1,c2}`; the five dropped
  OPPs (600/792/1032/1296/1344 MHz) are exactly those left with variant-only entries, so
  `_opp_supported_by_regulators` sees `minuV 0 maxuV 0`. SID values are identical to the earlier run
  (`speed` `0x6c00`, `calib` four 12-bit 2835/2841/2838/2842) and `avcc is not calibrated` still stands.
  Runtime FDT vs packaged DTB: +4,114 B at equal 573 nodes, an extra memreserve at `0x42000000`+512 B, and
  boot-chain-injected `/chosen` properties `baseos,native-boot` / `baseos,native-timing` /
  `baseos,boot-display` plus `boot_disp*` / `boot_fb0` (`7bf24800,2d0,1e0,20,b40,…` = the 720×480
  framebuffer handoff). Serial numbers redacted; binaries stay local.

### 2026-11-25 — RG34XX/H700 CPU clock, CPU rail, SID and memory reservations

- `rg34xx-inventory-20261125-cpux/` — read-only inventory (`porting/linkslot-1.0.45-feasibility` @ `aad78d4`,
  collector `ce88451c…`, device `sun50iw9p1` / kernel `4.9.170`). CPU clock parent is `pll_cpu` (div 1,
  `cpu` rate always equals `pll_cpu`; `cpuapb` = 1/4); `pll_periph0` stays 600 MHz / 4 users and is not a
  CPU parent. CPU rail is `axp2202-dcdc1` (DTS `cpu-supply` phandle `0x4f`, `cpu0 @1160mV`, measured
  950/1000/1160 mV tracking OPP; `axp2202-cpusldo` 900 mV unused). Five OPPs (600/792/1032/1296/1344 MHz)
  are dropped at boot as `not supported by regulators`. SID `speed@0x00` = `0x6c00` (matches no pinned
  vendor literal) and `calib@0x14` = `181113ab191b16bb` → four 12-bit codes 2835/2841/2838/2842, yet
  `sunxi-thermal: avcc is not calibrated` means calibration is **not** applied. `clk_summary` PLL rates and
  cpufreq readings diverge across samples (480 MHz requested vs `pll_cpu` 1008/1104 MHz), so only the
  parent relation is asserted. live FDT is 143,936 B / 573 nodes vs packaged DTB 139,822 B / 573 nodes —
  property-level patching, not a DTBO overlay. Serial numbers redacted; binaries stay local.

### 2026-11-24 — RG34XX active BOL4 payload identity resolved

- `rg34xx-boot-partition-deep-audit.md` addendum — decodes the active p4 +48 MiB BOL4 component using the verified extraction report and matches the 17,461,256-byte output SHA-256 to the primary Android image's gzip-decompressed kernel. Resolves the former BOL4-format / secondary-kernel identity unknown; does not change the firmware-pairing blocker or hardware NO-GO.

### 2026-11-24 — RG34XX native build inputs recovered from CI artifact

- `rg34xx-native-build-inputs-from-0539a7-artifact.md` — extracted read-only from BaseOS-MultiFrontend CI upgrade artifact `14588761000` (build commit `05c39a7`). Recovers the exact pinned `boot-gzip/boot-package.bin` (SHA-256 `56371d98…af3d`), source `boot.img`, Boot0 from the native capsule and the 128 KiB env input. Distinguishes that source TOC1 package from the separate recovery TOC1 (`e6c20e3a…`). Generated firmware/image/generation hashes match prior reports. The prepared 222,298,112-byte `boot-prefix.img` remains unavailable; the composed A/B image prefix is a different byte sequence and is explicitly not substituted.

### 2026-11-24 — RG34XX p4+40 MiB firmware package split

- `rg34xx-native-firmware/` — `firmware-manifest.json` for the 287,232-byte firmware package at
  p4 `+0x02800000` (SHA-256 `dc9dc3ae…f28e9`, FNV `0xaf0018cb`, both matching the audit record),
  split into contiguous `monitor` (0x1A400, ATF BL31 v1.0(debug):f30a720), `dtbo` (0xA00),
  `dtb` (0x23400) and `loader` (0x8000, A32 U-Boot). Firmware DTB is byte-identical to the
  `+48 MiB` active image DTB. The manifest's literal scan reports no length/FNV candidates because
  A32 builds those constants with `movw`+`movt`; an instruction-level scan finds `0x00829000` at
  `0x4A001880/0x4A001888` (tested at `0x4A001890`) and `0x04762a5f` at `0x4A00189C/0x4A0018A0`
  (tested at `0x4A0018A4`), both branching to the same reject path. Cross-check against BaseOS `native-handoff.h` maps loader loads `[r7,#0x94]`/`[r7,#0x98]` (`r7=0x4a300028`) to record fields `image_bytes`/`image_fnv` at `0x4a3000bc`/`0x4a3000c0`. This closes deployed loader ↔ active image identity, not raw-capsule SPL/generation reproduction. Binaries stay local.

### 2026-11-24 — RG34XX active boot inputs extracted and checksummed

- `rg34xx-active-inputs/` — host-only extraction of the active Android v2 / BOL4 image at
  `0x03000000 + 0x829000` from the local 64 MiB p4 dump, with `extracted-manifest.json`,
  `verification.json` (21/21 cross-checks pass) and `SHA256SUMS.txt`. Confirms the image
  FNV-1a32 `0x04762a5f`, BOL4 `0x80474e` bytes whose header FNV `0xb4c47d39` matches the packed
  payload, a `0x10a7008`-byte `kernel.Image` that decompresses byte-identically, page size 2048 /
  header v2, an empty cpio `initrd`, and a 139,822-byte DTB. Binaries stay local; only the
  checksum manifest is published. `not_installable`, `not_hardware_validated`.

### 2026-11-23 — RG34XX H700 live MMC mapping (round 16)

- `linkslot-h700-live-mmc-map-2026-11-23-round16.md` — analyzed the r2 read-only capture and
  decoded the live controller/host relationships. The active boot/system SD is `sdc0@0x04020000` (`sunxi-mmc-v4p1x`, Linux `mmc_host0`): it owns
  `mmcblk0`, the running rootfs p5, and the actual ROM/user-data volume p7. Bootargs designate p7
  as UDISK, and the mount table confirms p7 is mounted at `/mnt/sdcard` with `/mnt/sdcard/Roms`
  and other user paths; p6 is `/data` (ext4) despite its conflicting sysfs `PARTNAME=UDISK`.
  This is the first controller to support for same-card ROM storage; Linux reports SDHC, not eMMC. `sdc2@0x04022000` (`sunxi-mmc-v4p6x`, Linux `mmc_host1`) is the optional
  second-card-slot candidate, with 4-bit bus, GPIO card detect, and `vqmmc-supply` referencing
  AXP2202 ALDO1. `sdc1@0x04021000` (`mmc_host2`) is the SDIO DDR50 device consistent with Wi-Fi.
  The raw `cd-gpios` cells contain port 4 / pin 22 followed by vendor-specific config cells
  (the trailing `0x06` is not the pin); sysfs p6/p7 names also differ from the cmdline aliases.
  Live pinctrl/CCU debugfs: sdc0 uses PF0–PF5 in default state with sdmmc0 module clock 100 MHz and bus/reset gates enabled; sdc1 uses PG0–PG5 with sdmmc1 module clock 200 MHz; sdc2 was in sleep state with its sdmmc2 clock/gates off. Raw pinconf values remain undecoded. The report corrects H616 MASK_DATA0 handling as a shared clock-update requirement for sdc0 too, not sdc2 only. This is topology/config evidence only: sdc2 was empty, no block read/write or LK card test was done.

### 2026-11-23 — RG34XX MMC map, second device capture (r2)

- `h700-mmc-map-r2.tar.gz` — read-only capture after fixing the collector's platform-device
  sweep from `*mmc*` to `sdc*`. Includes the three live nodes, MMC host and block-device
  symlinks, raw controller DT properties, regulator/debugfs summaries, and dmesg. SHA-256:
  `c921136be2439c20d456904607542199f874d82181bd5467977ee12067350adf`. The detailed
  interpretation and remaining unknowns are in round 16. Serial redacted; no CID/CSD identity
  fields and no block-device reads or writes.

### 2026-11-23 — FAT32/exFAT real-image regressions and RG34XX MMC reference (round 15)

- linkslot-h700-fat-exfat-mmc-2026-11-23-round15.md — full feasibility workflow [45136628791](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45135115298) passed on head `20d674c`. Host compile and real-image FAT32/exFAT regressions passed, including FAT32 VFAT long-name file create/replace/delete, empty-file handling, path rejection, and exFAT 4 KiB/128 KiB cluster images checked clean by fsck. Records FAT32 LFN-directory limitation, no atomic rename/flush barrier, AArch64 writer-lock branch, and H700 MMC0/MMC2 register/board wiring references. Artifact `14799704524`, 380,276,600 bytes, SHA-256 `49dda3cc8e12ecf1ac1d13a593fa397ffcf47e9d896f3323fe4b73faaace9929`. This is host-only; no SD/MMC driver or hardware test. A read-only MMC mapping collector (`scripts/collect-h700-mmc-map-readonly.sh`) is now available and syntax-checked for the next device inventory. [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).

### 2026-11-23 — H700 card path containment and FAT/exFAT provider audit (round 14)

- linkslot-h700-storage-path-2026-11-23-round14.md — full workflow 45130158723 passed on `4bb58d1`; standalone Rust path resolver passed 9/9 tests, enforcing exact mount boundaries, explicit relative bases and caller allowlists. Audited pinned GammaOS LK `emu/gba/fat_ro/fat_wr/fat_exfat` as a potential reusable MIT-licensed generic filesystem layer; its `sd_fat.c` glue is MTK-specific and must not be reused for H700. No SD driver or persistent filesystem is implemented. [Workflow](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45130158723); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — Rust static mGBA binding into slot-retro (round 13)

- linkslot-rust-static-mgba-binding-2026-11-23-round13.md — workflow 45128096825 passed. Added a Rust module mapping all 21 namespaced mGBA exports into the real slot-retro::Api type; host C stubs verify mgba_static_api() and the LibretroCore::open_static() init/deinit lifecycle. Static mGBA binding test suite: 63 passed, 0 failed. The next step is slot-level built-in core selection; this report does not claim real game execution or hardware boot. [Workflow 45128096825](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45128096825); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — Real mGBA static API symbol binding (round 12)

- `linkslot-mgba-static-api-link-2026-11-23-round12.md` — main workflow 45126961555 passed on commit `13c8239`. Strict AArch32 API-shaped table references all 21 real namespaced mGBA functions, is combined with H700 newlib adapter, and is linked with the actual mGBA whole archive; newlib-only final ELF32/ARM has no undefined symbols. Artifact ID `14796725457`, 96,415,151 bytes. This is a link probe, not a Rust-runtime or device execution test. [Workflow 45126961555](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45126961555); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — Linkslot static core API entry (round 11)

- `linkslot-static-core-api-2026-11-23-round11.md` — full workflow 45126045887 passed on commit `2ebe74b`. Dynamic mode: 61/61 tests; static-only mode: 62/62 tests. Adds `LibretroCore::open_static(Api, ...)`, makes `libloading` optional with dynamic still default, and confirms shared init/deinit lifecycle. Artifact ID `14795752171`, 96,408,534 bytes. Actual namespaced mGBA symbol binding is the next in-progress probe, not yet included as passed. [Workflow 45126045887](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45126045887); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — Linkslot VFS integration and complete mGBA/newlib feasibility chain (round 10)

- `linkslot-lk-rust-feasibility-2026-11-23-round10-vfs-static-newlib.md` — successful workflow 45125097115 on feature head `2ea0b1f`. The real Linkslot 1.0.36 `slot-retro::environment()` callback now passes its VFS v3 host regression (60/60 slot-retro tests); the actual mGBA VDir adapter passes against a fake VFS; Generic and arm-none-eabi/newlib VFS v3 AArch32 archives build; H700 newlib adapter tests pass; whole-archive mGBA + adapter links with newlib-only and newlib-nosys; A32 LK BL33 layout probe passes. Artifact ID `14794884889` (91,104,214 bytes). No SD/FAT backend, static Linkslot core integration, full no_std frontend, hardware boot or device test. [Workflow 45125097115](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45125097115); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — newlib whole-archive link and mGBA VDir regression (round 9)

- linkslot-lk-rust-feasibility-2026-11-23-round9-newlib-link-and-vdir.md — successful workflow 45115290950. The actual mGBA VDir adapter passes host fake-VFS tests for enumeration, type, EOF, rewind (close/reopen), nested directories, file open, delete/mkdir and path traversal rejection. A whole-archive newlib/libm/libgcc link leaves only 11 syscall hooks (_close/_exit/_fstat/_getpid/_gettimeofday/_isatty/_kill/_lseek/_read/_sbrk/_write); adding libnosys closes an ELF32/ARM link but only with dummy system calls and a WFI entry stub. No LK service adapter or device runtime. [Workflow run 45115290950](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45115290950); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — mGBA AArch32 arm-none-eabi/newlib VFS v3 archive (round 8)

- linkslot-lk-rust-feasibility-2026-11-23-round8-arm-none-eabi.md — successful workflow 45114286962. Builds the pinned real mGBA VFS v3 file+directory variant with arm-none-eabi GCC/newlib (not glibc): 2.86 MB archive, 94 objects, ELF32/ARM EABI5 objects, all 25 Libretro exports namespaced, 52 true external references after archive-local resolution, and no POSIX/stdio file-system fallback. This is archive compilation only; no newlib whole-archive final link, no LK integration, no H700 storage backend or device run. [Workflow run 45114286962](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45114286962); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — mGBA Libretro VFS v3 AArch32 static archive (round 7)

- `linkslot-lk-rust-feasibility-2026-11-23-round7-mgba-libretro-vfs3.md` — successful workflow 45113739310. Builds the pinned real mGBA AArch32 archive with a temporary Libretro VFS v3 file+directory backend; archive has 114 objects, all 25 Libretro exports namespaced, and 51 true external symbols after subtracting internal cross-object references. The old `vfs-file.c`/`vfs-dirent.c` objects and POSIX/stdio/host-config filesystem symbols are absent. The 51 remaining dependencies are heap/string, locale/parser/errno, RTC/time, math/EABI and console/logging; no final LK link or H700 runtime. [Workflow run 45113739310](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45113739310); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — Linkslot Libretro VFS v3 ABI host validation (round 6)

- `linkslot-lk-rust-feasibility-2026-11-23-round6-libretro-vfs-v3.md` — successful workflow run 45109949583. Adds a no_std Rust registry for the Libretro VFS v3 C interface, version negotiation via `RETRO_ENVIRONMENT_GET_VFS_INTERFACE`, and a fake in-memory backend. AArch32/ARM and AArch64 final ELFs link with no undefined symbols; the x86_64 harness exercises file operations, directory enumeration, metadata and path rejection with exit 0. The report explicitly limits this to a singleton fake backend; no SD/FAT or Linkslot integration yet. [Workflow run 45109949583](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45109949583); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — Real mGBA AArch32 static archive and Generic VFS comparison (round 5)

- linkslot-lk-rust-feasibility-2026-11-23-round5-mgba-static.md — successful run 45108202490. Builds the Linkslot-pinned real mGBA source as Linux and Generic AArch32 static Libretro archives. Both have 115 object members and all 25 Libretro exports namespaced; Generic reduces external undefined symbols from 81 to 67 by removing Linux-specific file-descriptor/mmap/glibc fortified calls. The report compares the remaining symbols with pinned GammaOS LK libc and records that Linkslot does not yet implement RETRO_ENVIRONMENT_GET_VFS_INTERFACE, while mGBA's Generic directory backend still uses POSIX dirent. No final LK link or device runtime. [Workflow run 45108202490](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45108202490); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — Linkslot 1.0.36 static Libretro core registry ABI (round 4)

- `linkslot-lk-rust-feasibility-2026-11-23-round4-static-core-abi.md` — successful GitHub Actions run 45107173305. Links a no_std Rust table matching Linkslot's current `slot-retro::ffi::Api` shape to two namespaced mock cores (`mgba_retro_*` / `gpsp_retro_*`). AArch32 and AArch64 ELF links have no undefined symbols; x86_64 harness actually exercises environment/video/audio/input callbacks, ROM load/run, save-state, memory, cheats and deinit. This establishes the static function-table pattern only; it does **not** compile mGBA/gpSP. [Workflow run 45107173305](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45107173305); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).


### 2026-11-23 — Linkslot 1.0.36 static i18n and A32 BL33 layout (round 3)

- `linkslot-lk-rust-feasibility-2026-11-23-round3.md` — successful GitHub Actions run 45106548485. Adds `slot-i18n` no_std static-table proof: 6 domains, 1,763 unique translation keys and 1,763 entries per language; upstream 9 unit + 9 translations integration tests pass; generated no_std candidate 13 tests pass; AArch32/AArch64 checks pass. The same workflow reruns `slot-input`, `slot-model`, Rust/C ABI and A32 BL33 host layout probes. Records LK's uniprocessor scheduler, incorrect A53 compiler flag, missing A53 SMP init, and the Libretro `libloading` porting boundary. **No full LK firmware, device test, SD write or payload execution.** [Workflow run 45106548485](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45106548485); [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).



### 2026-11-23 — Linkslot 1.0.36 no_std behavior tests and pinned LK heap ABI

- `linkslot-lk-rust-feasibility-2026-11-23-round2.md` — second CI round. Original `slot-input` / `slot-model` host suite: 58 passed; 47 original gesture tests also pass against the isolated `no_std` `slot-input` candidate; 4 new pure-domain tests pass against the `no_std` `slot-model` candidate. Builds Rust `GlobalAlloc` against the pinned GammaOS LK `heap.h` function signatures and links the C test stub for AArch32. Documents the actual heap alignment contract and the important finding that current LK critical sections do not prove cross-core mutual exclusion. This is **header/ABI compatibility only**, not the real LK heap implementation or a hardware port. [Workflow run 45103281417](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45103281417); see also [PR #16](https://github.com/dengzhan007/Linkslot-Pico-for-H700/pull/16).

### 2026-11-23 — Linkslot 1.0.36 Rust no_std / LK ABI feasibility

- `linkslot-lk-rust-feasibility-2026-11-23.md` — GitHub Actions host-only cross-build. Verifies temporary `slot-input` and `slot-model` no_std+alloc candidates on AArch32 and AArch64, Rust staticlib ↔ freestanding C shim linking, final ELF architecture and undefined-symbol closure. Includes the AArch32 EHABI metadata-only marker caveat and explicitly excludes real LK heap/handoff/device execution. [Workflow run 45102606482](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45102606482); source/test branch is PR #16.


- `rg34xx-inventory-20261122-195053.zip` — full read-only inventory archive (`main` @ `acc5e09`).
- `debugfs-memory-reservations.txt` — standalone extract from that archive.
- `rg34xx-inventory-20261122-204742/` — sanitized ION/TEE runtime subset (`main` @ `9b68302`).
- `rg34xx-inventory-20261122-210352/` — sanitized Allwinner DRM-info SMC follow-up
  (`audit/rg34xx-ion-drm-followup-20261122` @ `00fe17d`); see `DRM-SMC-finding.md`.
- `rg34xx-inventory-20261122-212049/` — sanitized ION config-symbol + kallsyms capture
  (`audit/rg34xx-ion-drm-followup-20261122` @ `b84fff0`); see `ION-symbol-finding.md`.
  Confirms `CONFIG_ION_SUNXI=y`, `CONFIG_ION_OF=y` and all three key functions in kallsyms.
- `rg34xx-inventory-20261122-214708/` — sanitized text/JSON subset of the boot-partition
  capture (`audit/rg34xx-ion-drm-followup-20261122` @ `c6da96d`); see
  `boot-partition-local-only.md`. The 64 MiB `boot-partition.bin` and all other binaries
  are kept local only, NOT uploaded.
- `rg34xx-boot-partition-deep-audit.md` — deep read-only analysis of the local
  `boot-partition.bin`: two Android images, AArch64 4.9.170 #19 kernel with
  extracted IKCONFIG, empty ramdisk, DTB cross-comparison (512 MiB on-disk vs
  1 GiB live), DRM `GET_DRM_INFO` source match to `qiaoweibiao/T507_Kernel`,
  memory-region attribution and LK hand-off recommendations. No binaries included.
- `rg34xx-bootloader-handoff-followup.md` — deep read-only follow-up: AArch32 U-Boot boot flow
  and image selection, full DTBO fragment decode (PMIC only — not the cause of 512 MiB → 1 GiB),
  U-Boot `fdt_fixup_memory_banks`/`update_fdt_dram_para` fixup path, the NAND-env vs MMC-root
  contradiction explained by `boot_type`, vendor monitor identified as AArch64 BL31 with the
  A32→A64 SMC contract (`0x8000ff04` / SPSR `0x3c5`), recovered kernel kallsyms addresses for
  the DRM path, **BOL4 decoded as LZ4 holding a byte-identical kernel**, and an A/B/C comparison
  of LK take-over architectures. No binaries, serial numbers or local paths included.
- `rg34xx-active-loader-provenance.md` — read-only provenance check establishing that the
  unit is **already running** the BaseOS native chain (native SPL → vendor BL31 → A32
  loader → BOL4 image), not TOC1 U-Boot: p4 + 40 MiB firmware extent map, p4 + 48 MiB
  BOL4 frame verification, exact bootargs-template match, live FDT generation/DRAM/timing
  cross-check, active-vs-fallback image distinction, ABI table, corrections to the two
  earlier reports, and unresolved items. No binaries, serial numbers or local paths.
- `rg34xx-stage0-and-a64-probe.md` — stage-0 identity closure plus a host-only AArch64 entry
  probe. Establishes that LBA 16 holds a BaseOS 128 KiB **raw capsule** (dispatcher +
  contract vendor Boot0 at `+0x1000` + 48 KiB BaseOS SPL at `+0x11000`), that LBA 512 is a
  *different* vendor Boot0 revision, and binds the boot area to the running system by
  `FNV-1a32(generation) == capsule cookie 0xf7bed1c8`. Publishes the deployed SPL fingerprint
  (`cdfc6d48…`) and the three extents' hashes, explains why the generation cannot be rebuilt
  (missing pinned `boot-package.bin` and build-time GPT prefix), resolves the `p6/p7` label
  drift against `rcS`/`fstab`, and documents a 12,288-byte freestanding AArch64 probe that was
  built and verified on the host only. No binaries, serial numbers or local paths.
- `rg34xx-a64-probe-qualification.md` — fourth round: completes the SRAM usage audit against the
  H616 map (A1 `0x20000`–`0x28000`, SRAM C `0x28000`–`0x58000`), showing the deployed 128 KiB
  capsule does **not** fit in A1 (its content reaches `0x3C753`), that the BROM itself writes
  SRAM C, that the first code to *choose* SRAM C is the dispatcher, and that BROM-time ownership
  of SRAM C stays UNKNOWN even though it is demonstrably usable. Rebuilds the baseline probe with
  a real assembler (`zig cc` 0.17.1 / clang 22.1.8 / lld) and compares all 144 instructions
  against the hand-encoded original: 118 byte-identical, 26 differences classified, one of them a
  value correction. Adds a framebuffer probe gated on eight independent facts from the live tree
  that paints 23,877 pixels inside `0x7BF24800`–`0x7C076000` with zero out-of-range writes, plus
  nine mutation tests proving every refusal writes nothing. States explicitly that painting is
  not the same claim as *being visible*, so a dark panel on hardware must be reported as
  inconclusive. No binaries, serial numbers or local paths.
- `rg34xx-a64-hardware-readiness.md` — fifth round, host-only. Audits the watchdog recovery chain
  from the live FDT (`allwinner,sun50i-wdt` at `0x030090a0`), the matching T507 driver semantics,
  capsule/SPL/BL31/A32-loader writes and the reset→BROM→capsule→STOCK state machine; the remaining
  BL31 reachability and RTC-retention gaps are marked UNKNOWN. Hardens `probe_fb.c` with the loader's
  FDT arena bounds, strict header/token/duplicate checks and passes 228 ASan/UBSan FDT cases.
  Records final hashes (probe64 `3cb5c440…`, framebuffer probe `d77c9ae6…`), all build commands and
  an explicit **NO-GO** conclusion with a shortest unblocking path. No binaries, device writes or
  payload execution.

This branch is dedicated to field reports captured from the RG34XX handheld. Source code
lives on `main`; report archives are kept here because `build/` is gitignored there.

## Archives

| Archive | Captured | Source commit | Notes |
| --- | --- | --- | --- |
| `rg34xx-inventory-20261122-195053.zip` | 2026-11-22 19:50:53 +08:00 | `acc5e09` | Full read-only inventory incl. memory-map / debugfs markers |

SHA-256 of `rg34xx-inventory-20261122-195053.zip`:
`bbfa065e590ecd7f8394f9b251d15468d63ed6c93bbe192e4272a65a700ab5e2`

## Standalone extracts

- `debugfs-memory-reservations.txt`: verbatim copy of the same-named file inside the archive above
  (2,664 bytes, SHA-256 `6d256713f030dd6093c6f04770cf20a9561b93db76f288c01f2da179f1603ced`).

## 2026-11-22 20:47 capture — ION / TEE runtime focus

Directory `rg34xx-inventory-20261122-204742/` contains a sanitized subset from the read-only
inventory captured at `9b68302` (device ADB serial redacted here as well):

- `tee-ion-runtime.txt`
- `dmesg-hardware-lines.txt` — one occurrence of `usb_serial_number:` redacted; all other bytes verbatim
- `iomem.txt`
- `debugfs-memory-reservations.txt`
- `meminfo.txt`

Sanitization: only the device USB serial number was replaced with `[REDACTED]`; kernel virtual
addresses and physical MMIO ranges are kept because they are diagnostic, not unique identifiers.
No firmware images or environment partition were uploaded.

Key findings in `tee-ion-runtime.txt`:

- Kernel config exports `CONFIG_ION=y`, `CONFIG_TEE=y`, `CONFIG_OPTEE=y`;
  `/dev/ion` exists, `/dev/tee0` and `/dev/teepriv0` are `NOT_PRESENT`.
- ION heaps: `sys_user` (`ion_vin`, `ion_disp2`), `cma` (`nextui.elf`, 4,153,344 bytes), and an
  empty `secure` heap. No orphaned allocations.
- CMA area `0x77c00000..0x7bc00000`, 64 MiB total / 60,360 KiB free (~58 MiB), consistent with
  `CmaTotal`/`CmaFree` in `meminfo.txt`.
- Kernel log shows `optee: probing for conduit method from DT` followed by
  `optee: api uid mismatch` — TEE conduit probed but UID did not match.
- `NO_MATCHING_MODULES` for ION/TEE drivers (built into the kernel rather than loadable modules).

## Capture conditions

- Device: `[REDACTED]`, block device `/dev/mmcblk0`
- Collector: `scripts/collect-rg34xx-readonly-inventory.sh` (read-only; device side confirmed no writes)
- Environment: ADB platform-tools from Uotan Toolbox, Python 3.13.14

## Key results in the 20261122-195053 report

- Boot0 primary and secondary: `eGON.BT0` valid, header-declared size matched captured 128 KiB;
  SHA-256 `bf886995…319289f` / `4701a85e…fa9f5d` (unchanged across all captures)
- TOC1: SHA-256 `e6c20e3a…aa4604`, 4 items (u-boot, monitor, dtbo, dtb)
- `env-partition.bin`: full 16 MiB (16,777,216 bytes), CRC valid `0xe3b6e401`, 25 variables
- `live-fdt.json`: framebuffer `0x7bf24800..0x7c076000`, 720x480x32bpp, matches `disp_reserve`
- `debugfs-memory-reservations.txt`: CMA node `NOT_PRESENT: /sys/kernel/debug/cma`;
  memblock node exported and captured — node absence is not treated as "no reserved memory"
- `page-size.txt`: `NOT_PRESENT: getconf unavailable`; raw debugfs snapshot preserved without
  PFN-to-byte conversion

## 2026-11-23 19:22 capture — MMC map r2 (collector fix)

Archive `h700-mmc-map-r2.tar.gz` (8,313 bytes, SHA-256
`c921136be2439c20d456904607542199f874d82181bd5467977ee12067350adf`), captured at `c6da96d` with
`collect-h700-mmc-map-readonly.sh` (SHA-256 `5472cdb1…1152556`). Contains `mmc-map.txt`
(2,974 lines) and `README.txt`.

Collector defect fixed between r1 and r2: the platform-device sweep used
`/sys/bus/platform/devices/*mmc*`, which matches nothing here because sunxi registers these
controllers as `sdc0`/`sdc1`/`sdc2`. The whole "platform MMC devices and live DT properties"
section was therefore empty in r1. r2 globs `sdc*` and returns all three nodes. No other
collector logic changed, and the run stayed read-only (no block-device access, no writes).

Live DT / topology (properties appear as raw big-endian bytes in the capture):

- `sdc0` @ `0x04020000` — `allwinner,sunxi-mmc-v4p1x`, `bus-width = 4`, no `cd-gpios`, no
  `vmmc-supply`/`vqmmc-supply`. This is the eMMC: `mmcblk0` = 30,375,936 sectors (~14.48 GiB),
  `removable = 0`, 7 partitions (p1 `special` 64 MiB, p2 `boot-resource` 32 MiB, p3 `env` 16 MiB,
  p4 `boot` 64 MiB, p5 `rootfs` 512 MiB, p6 `appfs` 128 MiB, p7 `UDISK` remainder).
- `sdc2` @ `0x04022000` — `allwinner,sunxi-mmc-v4p6x`, `bus-width = 4`, has `cd-gpios`
  (port 4, pin 6, active low) and `vqmmc-supply` (phandle `0x54`).
- `sdc1` @ `0x04021000` — `allwinner,sunxi-mmc-v4p1x`, `bus-width = 4`, no card-detect property.
- Only `sdc0` has a card: `mmc0:1234` under `mmc_host/mmc0`. `mmc1` (sdc2) and `mmc2` (sdc1) are
  registered hosts with no child device, i.e. no card present at capture time; there is no
  `mmcblk1`. sdc2 is the only controller with card detect, so it is the likely removable slot,
  but this capture alone does not prove which physical connector each controller serves.
- All three bind to `sunxi-mmc`; `resets`/`reset-names` are absent on all three nodes, so reset
  control is not described in the live tree.
- DT alias numbering does not match kernel host numbering: aliases say `mmc0 = sdmmc@04020000`
  and `mmc2 = sdmmc@04022000` (`mmc1` alias absent), while the kernel class exposes
  `mmc0 → sdc0`, `mmc1 → sdc2`, `mmc2 → sdc1`. A mapping inferred from DT aliases alone
  is wrong for the two non-eMMC controllers.
- Boot cmdline corroborates the layout: `root=/dev/mmcblk0p5 rootwait`, `gpt=1`, `boot_type=1`,
  `secure_os_exist=0`, `disp_reserve=1382400,0x7bf24800`, `earlyprintk=sunxi-uart,0x05000000`,
  `console=ttyS0,115200`, `uboot_message=2018.05(05/26/2026-13:55:09)`.

Sanitization: the device serial number appeared twice on the `cmdline=` line and was replaced
with `[REDACTED]`; every other byte is exactly what the device produced. The unsanitized archive
is kept local only (SHA-256 `8856e75b2942763cd4799ae647feb6364ec85680d25d5b42f0a324eafff56503`),
so the two archives differ only by those two replacements. No CID/CSD identity fields and no
partition contents are included. No SD/MMC driver work and no hardware test are implied.


**Correction (2026-11-26):** The pvaibhav pinned source does not require a second FDT timing node for its runtime register switch. Its actual 120-Hz pair is TCON divider `0xf000000a` + PLL `0x88002b03` (528 MHz / 10 ≈ 120.131 Hz); `0x88001701` is a different 576-MHz encoding. This corrects the interpretation in the runtime audit summary without changing the measured result: the device's PLL remained `0x88001703` and IRQ stayed near 59.6 Hz in both captures. See the erratum at the end of [slot-refresh-audit/SUMMARY.md](slot-refresh-audit/SUMMARY.md).


**Binary identity follow-up (2026-11-26):** [ADDENDUM-2026-11-26.md](slot-refresh-audit/binary-identity/ADDENDUM-2026-11-26.md) closes the source-level high-refresh gate: this unit's live panel name `rg34xx_v1` fails the official Slot `rg34xx` requirement `fog_fj035fhd05_v1`, so `DevicePlatform.refresh=None` and the stored `high_refresh=1` setting never reaches the writer. The addendum also records the exact official-release armGBA layout probe (commit `df6b263...`, combined frame-counter offset 11,948 bytes; dedicated CI step passed), while preserving the no-`/proc/PID/mem` boundary.


## 2026-11-26 armGBA runtime mapping

The official H700 release's armGBA core now has a read-only runtime mapping report at
`slot-refresh-audit/binary-identity/runtime-mapping/`. It documents the three-way PT_LOAD/load-bias
cross-check, the primary GBA pointer versus the link-cable cache, and the exact-source
`frameCounter` offset. No process memory was read and no frame-rate sampling is claimed.
Start with [SUMMARY.md](slot-refresh-audit/binary-identity/runtime-mapping/SUMMARY.md).


## 2026-11-26 RG34XX FPS sampling

The first full Slot/armGBA runtime rate capture is under [slot-refresh-audit/fps-sampling](slot-refresh-audit/fps-sampling/README.md). Independent arithmetic and evidence-boundary review: [ANALYSIS.md](slot-refresh-audit/fps-sampling/ANALYSIS.md).
