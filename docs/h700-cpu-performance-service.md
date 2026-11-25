# H700 CPU performance modes and DVFS service

**Target:** RG34XX non-SP / Allwinner H700 / Cortex-A53 quad-core  
**Status:** architecture decision and evidence gates only. No CPU PLL or PMIC voltage writes are implemented or approved.

## 1. Decision

The port should support user-selectable CPU performance modes and a future automatic policy. The feature belongs to a platform-owned **H700 CPU Performance Service**:

- C HAL owns CCU/CPU PLL programming, PMIC transactions, voltage sequencing, thermal caps and hardware readback.
- Rust no_std / Linkslot owns policy selection, settings UI and requested profile. It must not write CCU/PMIC registers directly.
- The service returns the actual operating point and the reason for any clamp (silicon bin, board limit, thermal limit, power limit, unavailable regulator, or hardware gate).
- The initial implementation is read-only capability discovery and host-tested policy logic. Hardware writes remain disabled until the handoff, clock, regulator and thermal evidence gates are closed.

This is independent of upstream Linkslot support. A small H700 C service ABI can exist first; a later Linkslot CPU Clock / Performance setting can consume it without changing the hardware implementation.

## 2. Target-board facts and important distinction

The RG34XX live inventory identifies four Cortex-A53 CPUs and **AXP2202** as the PMIC. The live Linux log identifies it on I²C-5 at address **0x34**:
- `reports/docs/rg34xx-live-inventory-2026-11-22.md`
- `reports/rg34xx-inventory-20261122-214708/live-fdt.json`
- `reports/rg34xx-inventory-20261122-214708/dmesg-hardware-lines.txt`

Two independently maintained RG34XX-specific device trees (Knulli and MustardOS) map `cpu@0`'s `cpu-supply` phandle to AXP2202 **DCDC1**. In those references, DCDC1 is named `axp2202-dcdc1`, has a 500000–1540000 µV range, 250 µs ramp delay and 1000 µs enable-ramp delay. This is strong board-specific cross-check evidence, but it is not a live regulator readback for this exact unit/revision.

The first 2026-11-22 inventory's `live-fdt.json` was a reduced summary and did not contain CPU nodes or the full regulator subtree. The updated FDT inspector now resolves CPU `cpu-supply`, `operating-points-v2`, referenced regulator nodes, NVMEM cell phandles/offset metadata and thermal-sensor phandles including provider arguments. The 2026-11-25 read-only capture [reports/rg34xx-inventory-20261125-cpux](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux) now resolves CPU0 `cpu-supply` phandle `0x4f` to AXP2202 DCDC1, and Linux regulator debugfs reports `cpu0 @1160mV`. This confirms the CPU rail mapping in the captured live FDT/runtime pairing; it is still Linux regulator-framework evidence, not a raw PMIC I²C register read at BL33 entry or a guarantee across all board revisions.

The new FDT report is intentionally explicit that `opp-hz`/`opp-microvolt` values are static DT declarations. They do not prove the currently selected OPP, PMIC voltage readback, silicon bin, regulator enable state or safety of applying the points. The runtime collector reads only the two exact SID cells declared by the pinned RG34XX-family DTS (speed offset 0, 2 bytes; thermal calibration offset 0x14, 8 bytes), and only after the NVMEM provider `of_node` exactly matches `sunxi-sid-ng@03006000`. The pinned provider driver maps its NVMEM view to SID hardware window offset `0x200`; the collector uses NVMEM-relative cell offsets, not guessed MMIO. It never dumps the complete NVMEM region.

The matching Allwinner AXP2202 U-Boot reference driver describes DCDC1 voltage selector register `0x83` (mask `0x7f`), the DCDC enable register `0x80` (bit 0), 10 mV steps up to 1.2 V and 20 mV steps above 1.2 V, with a declared 0.5–1.54 V control range. These are register-format and driver-range references only; they do not authorize writes or establish the actual regulator voltage left by the stock boot chain. Never copy AXP717 descriptors or regulator limits from RG35XX-2024 into the RG34XX board profile.

The same RG34XX device-tree reference exposes the SID speed cell at `/sunxi-sid-ng@03006000/speed@00` (offset 0, 2 bytes) and thermal calibration cell at `/sunxi-sid-ng@03006000/calib@14` (offset `0x14`, 8 bytes). The 2026-11-25 bounded capture reads speed raw `0x6c00` and thermal bytes `181113ab191b16bb`; the mainline diagnostic classifier calls `0x6c00` candidate bin 5, but vendor source trees differ: two 4.9.170 candidates map it to mask 1 or default mask 4, while one 4.9.191 candidate maps it to mask 1 with a safer length-checked read. None is proven to match the running image, so OPP authorization stays `not_approved`. The kernel log reports the H700 thermal sensor at `0x05070400` with **avcc is not calibrated**. The calibration bytes exist, but the driver did not accept AVCC calibration; temperatures remain diagnostic only and cannot drive closed-loop automatic DVFS. Full source comparison and parser limitation: [RG34XX CPUX live read-only audit](rg34xx-cpux-live-audit-2026-11-25.md) §1.5.

The current H700 handoff audit also leaves CPU1–CPU3 state unknown at BL31→BL33. CPU frequency is a shared cluster clock domain, not a per-Linkslot-core setting. Changing the shared CPU clock while an unaccounted secondary PE is running is unsafe. Keep the CPU service write path disabled until the single-core handoff invariant is positively proven.

## 3. What the reference GammaOS Pico does—and what must not be copied

The reference LK has an in-game CPU Clock row and code paths that request named frequencies. However, its MT6785 implementation rewrites the MediaTek ARM-PLL PCW and explicitly does **not** scale CPU voltage. Its core-porting notes describe instability when sustained clocks exceed the fixed boot-voltage operating point. This is useful as a UI/policy reference, not as an H700 DVFS implementation.

Reference source anchors:
- platform/mt6785/pll.c: ayaneo_set_cpu_mhz() / ayaneo_get_cpu_mhz()
- emu/gba/gba_driver.c: CPU Clock menu OPP grid and run-ahead clock tiers
- documentation/development/core-porting-notes.md: fixed boot Vproc warning and stable menu clock discussion

Do not port the raw MTK PLL register sequence, the 600–2000 MHz menu list, or its assumption that PLL-only changes are safe. Those values belong to another SoC and another voltage setup.

## 4. H700 OPP/DVFS evidence

The pinned vendor SID driver also offers `sunxi_get_soc_ver()`, and `drivers/char/sunxi-sysinfo/sunxi-sysinfo.c` exposes it as `sunxi_batchno` in `/sys/class/sunxi_info/sys_info`. Do **not** use that path for a strictly read-only collector on this H700 target: for `CONFIG_ARCH_SUN50IW9`, `sunxi-sid.c` does not define `SUNXI_SOC_ID_IN_SID`, so `sid_soc_ver_init()` calls `sid_rd_soc_ver_from_ce()`; if CE is not already enabled it temporarily writes CCU clock/reset registers, then restores them. Reading the full `sys_info` class file also emits chip serial/chipid data. The collector therefore probes only generic `/sys/devices/soc0/{machine,family,soc_id,revision}` attributes when present and does not invoke the vendor sys_info or ioctl path. The offline `sid-cell-report.json` preserves those strings under `generic_soc_identity`, but labels them as generic SoC-bus metadata and keeps `opp_authorization=not_approved`; they are not assumed to be a complete replacement for the vendor SID's SoC revision semantics.

Pinned `sun50iw9p1` source is useful for mapping the hardware data path, but it exposes a speed-bin consumer defect that blocks direct reuse:

- The pinned `sun50iw9p1.dtsi` declares `speed@00` with a two-byte cell (`reg = <0x0 2>`) and `calib@14` with an eight-byte cell (`reg = <0x14 8>`).
- The pinned `drivers/nvmem/core.c` implementation allocates exactly `cell->bytes` with `kzalloc(cell->bytes)`, and returns `*len = cell->bytes`. Thus the speed cell read returns a two-byte buffer.
- The pinned `drivers/cpufreq/sun50i-cpufreq-nvmem.c` stores that return value in a `u32 *` and evaluates `switch (*efuse_value)` without validating `len`. For a two-byte allocation this is a potential out-of-bounds 32-bit read. Its default branch also maps unrecognized values to a supported-hardware mask. **This project's code must not copy this consumer or trust its raw-value-to-OPP mask.** The collector captures only the two exact speed-cell bytes; the SID decoder reports the little-endian u16 and any literal match for diagnostics, but hard-codes OPP authorization to `not_approved`.
- The pinned `sunxi-thermal-ng.c` consumer reads the eight-byte calibration cell as four `u16` words, interprets the factory-test temperature plus sensor calibration words, and warns when a derived calibration value is outside its 12-bit range. That size/layout is consistent with the DTS cell declaration; our offline decoder reports the raw fields only and does not infer that the live sensor has passed calibration.

A separate, explicitly diagnostic reference classifier is now implemented in `tools/inspect_h700_sid_cells.py`, pinned to upstream Linux commit [`8d7fe9885c62fc9a95bf55d9db244d9672a8aadc`](https://github.com/torvalds/linux/blob/8d7fe9885c62fc9a95bf55d9db244d9672a8aadc/drivers/cpufreq/sun50i-cpufreq-nvmem.c):

- Direct reference candidates: `0x2000 → bin 0`; `0x5000/0x5400/0x6000 → bin 3`; `0x5c00 → bin 4`; `0x5d00 → bin 0`; `0x6c00 → bin 5`.
- Ambiguous group: `0x2400/0x7400/0x2c00/0x7c00` requires the **ARM SMCCC ARCH_SOC_ID revision**. Upstream selects bin 1 when SMCCC is supported and revision ≤ 1; it selects bin 2 for later revisions, and also when SMCCC reports `NOT_SUPPORTED` (implicitly treating that as C+). This project deliberately does **not** copy that unsupported-revision fallback: without a trustworthy revision it reports `candidate_bin=null` and the candidate set `[1, 2]`.
- Unknown raw values are reported as unsupported; the upstream driver's permissive default-to-bin-0 behavior is deliberately not copied.
- Generic Linux `soc0/revision` strings are not accepted as SMCCC revision. The report uses the term `mainline_reference_decode` and keeps `opp_authorization=not_approved` for every path. A reference candidate bin is not an OPP mask, voltage table, hardware setting or approval.
- Upstream's qualitative bin order comment is `4 → 0/2 → 3 → 1` (0 and 2 appear equivalent in the upstream OPP tables). This ordering is diagnostic only and does not define RG34XX power/performance guarantees.

This classifier does not change the C policy service, supply an H700 board profile or enable DVFS. A real RG34XX OPP table still requires a board-specific validated frequency/voltage relationship, silicon revision proof, PMIC readback, calibrated temperature and secondary-core quiescence evidence.
- The RG34XX vendor DTS contains its own `opp_l_table` from 480 MHz upwards, with per-speed/silicon variants. These are vendor configuration references, not a validated frequency-voltage table for the current device at BL33 entry.
- The mainline H616 CPU OPP table includes points up to 1.512 GHz with speed-bin-specific voltages. Those values are useful for comparing the Linux driver model only; they are not a release OPP grid for this RG34XX.
- The RG34XX DTS DCDC1 range/ramp metadata and AXP2202 selector register definitions are documented in section 2 above. The AXP717-based RG35XX-2024 1.16 V configuration is not the RG34XX voltage policy.

A pinned vendor `sun50iw9` clock-driver audit now supplements the earlier mainline H616-family reference. It identifies source-level `PLL_CPU` offset `0x0000`, `CPU_CFG` offset `0x0500`, the `pll_periph0` alternate-parent switch before PLL rate changes, the post-change restore to `pll_cpu`, and a new-mode PLL-lock polling path. It also finds a critical source comment that PLL enable bit 31 is boot-enabled and must not be changed dynamically, while bit 27 is used for PLL output gating. The lock-status bit is compile-configuration dependent (`LOCKBIT(x)` differs for EVB and non-EVB builds), so it cannot be hardcoded from a generic source review. The vendor CPU factor table is explicitly marked incorrect and is not an OPP source. See [H700 CPUX / sun50iw9 clock-source audit](h700-cpux-clock-source-audit.md) for pinned file hashes, sequence details and remaining gates. The 2026-11-25 Linux snapshot now observes `pll_cpu → cpu`, `pll_periph0=600 MHz`, but this is framework clock-summary evidence, not direct CCU readback or the BL33-entry state. Timer continuity and secondary-core state remain unverified. No raw MMIO implementation is approved.

Source references:
- https://github.com/knulli-cfw/knulli-linux/blob/57afa662009641d181f57241bcc480119b131530/package/boot/uboot-h700/rg34xx/boot_package/rg34xx.dts
- https://github.com/MustardOS/internal/blob/bb67c448c4e2ab73ac9797d257e1492dc49cff45/device/rg34xx-h/package/sunxi.dts
- https://github.com/CrealityTech/sonic_pad_os/blob/7f37a7fbf4ad59907034715e3d967b9177de1cd4/lichee/brandy-2.0/u-boot-2018/drivers/sunxi_power/pmu_axp2202.c
- https://github.com/CrealityTech/sonic_pad_os/blob/7f37a7fbf4ad59907034715e3d967b9177de1cd4/lichee/brandy-2.0/u-boot-2018/include/sunxi_power/pmu_axp2202.h
- https://github.com/torvalds/linux/blob/master/drivers/cpufreq/sun50i-cpufreq-nvmem.c
- https://github.com/torvalds/linux/blob/8d7fe9885c62fc9a95bf55d9db244d9672a8aadc/drivers/cpufreq/sun50i-cpufreq-nvmem.c
- https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/arch/arm64/boot/dts/sunxi/sun50iw9p1.dtsi
- https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/nvmem/core.c
- https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/nvmem/sun50i_sid.c
- https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/cpufreq/sun50i-cpufreq-nvmem.c
- https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/thermal/sunxi-thermal-ng.c
- https://github.com/torvalds/linux/blob/master/drivers/clk/sunxi-ng/ccu-sun50i-h616.c
- https://github.com/torvalds/linux/blob/master/arch/arm64/boot/dts/allwinner/sun50i-h616-cpu-opp.dtsi

The OPP table for this port must be generated/selected only after we know:
1. the exact RG34XX board revision and live cpu-supply / regulator mapping;
2. the actual H700 SID speed-bin value and silicon revision, with unknown bins failing closed;
3. the board's permitted minimum/maximum CPU voltage and reliable regulator range;
4. the exact frequency/voltage pairs approved for that silicon bin and this board;
5. calibrated thermal sensor data and safe sustained temperature/power limits;
6. CPU1–CPU3 state and a safe BL31→BL33 single-core clock-control contract.

Never infer that a frequency is safe just because it appears in an H616/H700 OPP table, another handheld DTS, a vendor boot clock or an online overclock setting.

## 5. User-visible mode model

Expose modes as **policies**, not arbitrary frequency entry:

| Mode | Intended behaviour | Initial policy |
|---|---|---|
| Power Saver | prioritize battery life and low heat | lowest board-approved OPP that still meets the active frame/audio deadlines |
| Balanced | default everyday mode | conservative validated OPP, modest boost on short workload spikes |
| Performance | sustain demanding emulation/rendering | highest continuously validated OPP for this board and silicon bin |
| Boost | short high-performance burst | disabled until voltage, thermal and stability gates explicitly approve a short-duration boost |
| Auto | adapt to workload | use frame-time/deadline pressure, temperature and power headroom with hysteresis |

A mode name does not guarantee a fixed MHz value. The service may clamp a request and must report the actual clock and reason. The UI should display “Auto / Balanced / Performance” plus actual MHz, not claim a target was applied when the service rejected it.

There is one shared H700 CPU cluster clock; the UI should not pretend to set independent CPU0/CPU1/CPU2/CPU3 frequencies. Per-core values can be displayed only as telemetry if a verified counter is available.

## 6. Safe transition contract

After all hardware gates are closed, the C HAL transition must be a single serialized operation:

### Raising performance

1. Validate requested OPP against the detected H700 speed bin, board voltage table, thermal ceiling and policy cap.
2. Raise the verified CPU supply voltage first using the identified RG34XX PMIC/regulator path.
3. Wait for the regulator's documented settling/ramp time and verify the programmed/read-back voltage.
4. Move the CPU clock to a verified safe parent/divider if required by the H700 CCU sequence.
5. Program the CPU PLL and divider, wait for the documented lock/stability condition, then switch the CPU clock back.
6. Read back the effective clock and report the actual OPP; on any failure, return to a known safe clock/voltage state or remain at the existing safe OPP.

### Lowering performance

1. Move the CPU clock to the lower approved OPP first.
2. Verify the effective clock.
3. Lower the CPU supply voltage only after the lower clock is confirmed.
4. Report the actual OPP.

Never lower voltage before reducing the clock. Never change the shared CPU PLL while secondary CPU state is unknown. Do not perform PLL relocks on every frame; mode changes should be event-driven, with minimum dwell/hysteresis.

## 7. Automatic policy and workload integration

The first policy should be intentionally simple:
- choose the selected mode when entering menu/game;
- allow a bounded temporary boost for clearly CPU-bound work such as software rendering, ROM/archive decompression or save-state compression;
- restore the previous policy after the operation;
- only add frame-time-driven adjustment after panel pacing and audio DMA metrics are real and measured.

Later, Linkslot's run-ahead depth can inform a performance request because each extra prediction depth increases work per displayed frame. However, the mapping from run-ahead depth to an OPP must be measured on the selected static core and must never bypass thermal or silicon caps.

Use hysteresis and a minimum dwell interval to avoid frequency oscillation. Do not continuously change PLL settings within a frame loop.

### 7.1 Versioned C service ABI v1 (policy-only implementation)

The first stable C boundary is now implemented under `platform/h700/power`:

- `h700_cpu_performance_service.h` defines ABI v1 configuration, request, telemetry and state structures with explicit version/size fields.
- `h700_cpu_performance_service.c` owns a caller-allocated, fixed-size OPP table and delegates mode selection to the existing pure C policy selector. No heap, worker thread, MMIO or PMIC access is used.
- `h700_cpu_perf_service_request_mode_v1()` returns `H700_CPU_PERF_SERVICE_POLICY_ONLY` after selecting an OPP. That return value means only that a target was planned; it never means a clock or voltage change was applied.
- The state reports `planned_frequency_hz/planned_voltage_uv` separately from read-only `actual_frequency_hz/actual_voltage_uv/temperature_mc` telemetry. `hardware_applied` remains zero in ABI v1.
- Runtime limits can be refreshed; when a prior mode has a valid plan, the service re-evaluates it against the new voltage/thermal limits.
- `scripts/test-h700-cpu-performance-service.sh` tests ABI rejection, missing evidence gates, mode selection, telemetry separation, thermal clamping, Boost gating and malformed OPP tables.
- `integration/linkslot/h700-frontend-core/slot-cpu-performance` is the typed Rust `no_std` client. It treats the C service pointer as opaque, maps mode/error codes, checks the 64-byte ABI v1 state layout, and exposes planned OPP separately from optional actual telemetry. It does not own or initialize the C service.

The C facade is intentionally not wired to a PLL/PMIC actuator. A static C runtime owner and its Rust `CpuPerformanceRuntime` token now provide the lifetime/acquisition boundary, but the H700 target hooks remain no-op and no approved board profile is loaded. Neither path is wired into Linkslot `App`/settings. Until a real readback backend exists, actual clock/voltage must remain unavailable. Do not show planned MHz as measured MHz and do not expose an applied/success state based only on `POLICY_ONLY`.

#### 7.1.1 Static H700 runtime owner (fail-closed, no board profile)

The platform-owned lifetime boundary now exists in `h700_cpu_performance_runtime.h/.c`:

- One zero-initialized, static `h700_cpu_perf_service_v1_t` is owned by the H700 C runtime. The accessor returns `NULL` until initialization succeeds; the owner is intended to be initialized once during single-core startup, before the Linkslot event loop.
- Initialization requires a board-owned profile with ABI/size, RG34XX board ID, non-zero board revision, an explicit approval marker, zero reserved fields, a structurally valid OPP table and all mandatory CPU evidence gates. It preflights Power Saver, Balanced, Performance and Auto against the policy selector before publishing the service pointer.
- The approval marker is an assertion by trusted platform code, **not** a hardware proof or security boundary. The profile must be immutable board-owned source/configuration and must not be provided by user data, a ROM, an untrusted file or the Linkslot UI.
- The Rust client now has a `CpuPerformanceRuntime` token that calls the C runtime status/getter ABI and returns a distinct `RuntimeUnavailable` error when the C owner is not ready. Its unsafe acquisition contract requires successful platform initialization, no owner reinitialization for the token lifetime and exclusive mutation; temporary `CpuPerformanceClient` access is obtained by mutable borrow.
- Every initialization attempt first revokes prior readiness. A missing, rejected or incomplete profile leaves the runtime unavailable and does not preserve a stale service handle. The underlying service ABI also revokes its prior initialized state before validating a full-size reinitialization request.
- Host regression tests use clearly labelled synthetic data only. No RG34XX profile has been checked in, because the real board OPP table and evidence gates are still missing. The H700 `target_init()` hook exists but is intentionally empty; it does not call this owner. Linkslot App/settings wiring is also not present. Consequently the real RG34XX runtime remains unavailable; the CPU Performance QuickRow is present only as a display-only `Unavailable` row and is inert (no mode selection or persistence).
- The owner performs no MMIO, PMIC read/write, PLL operation or telemetry sampling. Successful initialization only exposes the policy-only service; it does not apply hardware performance changes.
- The pinned LK `kernel/main.c` calls `target_early_init()` and `target_init()` unconditionally. The H700 target now supplies both hooks in `target/rg34xx_h700/init.c`; they intentionally remain no-op. When a real board-owned profile and all prerequisites are approved, the intended initialization point is `target_init()`, after `platform_init()` and before `apps_init()`. No current target hook calls the CPU owner.
- The three C objects (policy, service facade and runtime owner) are now listed in `platform/h700/rules.mk` for the future LK target link. The LK project guard asserts the object entries and matching source files. This is only build-graph registration: the full GammaOS LK H700 target has not linked, and the board memory approval gate still prevents a real image build. No startup code calls the owner yet.

The new host test covers unavailable-by-default state, NULL/unapproved/wrong-board/missing-gate/missing-OPP rejection, stable static ownership, policy-only reporting and revocation of a previously borrowed handle. The C owner and target lifecycle object passed H700 freestanding AArch64, AArch32 ARM-state and AArch32 Thumb-2 compile checks. The end-to-end Linkslot LK feasibility run #45393436818 completed 69/69 steps successfully; this is still not a full approved device image or real hardware boot.

### 7.2 Linkslot 1.0.45 settings integration contract

The upstream 1.0.45 UI extension point was reviewed in the immutable snapshot. The H700 overlay now adds a deliberately **display-only / unavailable** CPU row while preserving every pre-existing discriminant:

- `QuickRow::SYSTEM_SETTINGS` grows from six to seven entries; the new `CpuPerformance` row is appended after About. `QuickRow::ALL` grows 87→88; Language remains index 86, CPU Performance is index 87, and the non-ALL `SplitLink` remains the next discriminant at 88. No existing row index moves.
- The H700 overlay adds `QuickValue::CpuPerformanceUnavailable` at the end of the value enum/table; `QuickValue::ALL` grows 37→38 and the new value has index 37. Existing language and other quick-value indices are unchanged.
- The visible row is **CPU 性能 / CPU Performance**. Its current value is **暂不可用 / Unavailable**. The row has a microchip glyph and text fallback; its bilingual label/value are supplied by the H700 overlay without modifying the immutable upstream translation catalogues.
- `App::quick_value()` always returns the unavailable value for this row. `App::change_setting()` returns before any state change or `persist()`, and `open_quick_row()` treats the row as inert (no subpage). This is intentional: a user cannot choose a mode that currently has no backing runtime service.
- The C service owner remains unavailable because there is no approved RG34XX board OPP/profile and the H700 `target_init()` hook remains a no-op. The typed Rust `CpuPerformanceRuntime` client is **not yet connected** to `App`; actual CPU mode choices, policy persistence, frequency/voltage/temperature telemetry and hardware transitions remain disabled.
- Once an approved profile, initialized C owner and real readback path exist, replace the placeholder with **Power Saver / Balanced / Performance / Auto** selection. Keep Boost hidden until voltage, thermal and sustained-load evidence passes. Persist only a validated requested policy; never represent a planned OPP as measured MHz. Show actual telemetry separately and keep “policy not applied” explicit until a real hardware backend confirms it.
- The patcher `scripts/apply-linkslot-h700-frontend-core.py` injects two App unit tests for the unavailable QuickValue/index, left/right no-op behavior and no settings persistence. Both workflows now execute them with `cargo test -p slot --lib ... h700_cpu_performance_quick_row_tests`: runtime-slices [#45428654903](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428654903) passed **20/20 steps** (2 tests passed, 0 failed), and full Linkslot feasibility [#45428664653](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428664653) passed **71/71 steps** with artifact [14937338188](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45428664653/artifacts/14937338188). The test command is scoped with `--lib` so unrelated legacy `slot/tests/core_picker.rs` integration-test API errors are not built as part of this focused App regression.

## 8. Required hardware and safety gates

- [x] Export and parse the RG34XX live-FDT CPU `cpu-supply` relationship: CPU0 phandle `0x4f` resolves to `axp2202-dcdc1` (500000–1540000 µV, always_on/boot_on). Linux regulator debugfs independently reports consumer `cpu0 @1160mV` in the 2026-11-25 capture. This confirms the rail mapping for this captured FDT/runtime pairing; it is not raw PMIC register readback at BL33 entry.
- [x] Confirm the current Linux runtime PMIC observation: AXP2202 on I²C-5, address `0x34`. Match generic AXP2202 reference DCDC1 selector/control fields (`0x83` / `0x80`) without enabling writes.
- [ ] Read-only PMIC voltage and enable-state readback at BL33 entry; verify that the stock firmware left the expected rail at the expected voltage before any transition is designed.
- [x] Read the exact bounded SID speed cell (two bytes): raw `0x6c00`; thermal calibration cell (eight bytes): `181113ab191b16bb`. Mainline diagnostic decode says candidate bin 5, but two 4.9.170 vendor source candidates disagree (mask 4 versus mask 1), generic SoC identity is unavailable, and `opp_authorization` remains `not_approved`. The raw read is complete; exact running-source identity and active OPP mask remain unresolved.
- [x] Audit a pinned `sun50iw9` vendor CPUX source candidate: `PLL_CPU=0x0000`, `CPU_CFG=0x0500`, temporary `pll_periph0` parent before PLL rate change, restore `pll_cpu` afterward, new lock-mode polling; bit31 must not be toggled dynamically and lock-status bit is build-config dependent. This is source-level only; actual BL33 CCU base/access, lock configuration, timer continuity and transition safety remain pending.
- [ ] Resolve CPU1–CPU3 state at BL31→BL33. Until then, CPU PLL writes remain disabled.
- [ ] Implement/read the H700 thermal sensor path and validate AVCC calibration; the current RG34XX kernel capture reports “avcc is not calibrated”, so automatic thermal DVFS is blocked.
- [ ] Establish cold-boot BL33-entry voltage/clock readback and a conservative board-approved OPP table before adding any write path.
- [x] Implement a pure C OPP policy selector with no MMIO/PMIC access, strict speed-bin masks, board-voltage and thermal caps, and fail-closed evidence gates; add host regression tests and AArch64/AArch32 compile coverage.
- [x] GitHub Actions [H700 CPU OPP / AArch32-AArch64 regression #45381349400](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45381349400) passed: policy host regressions plus AArch64, AArch32 ARM-state and AArch32 Thumb-2 freestanding object compilation.
- [x] GitHub Actions [H700 Linkslot 1.0.45 runtime slices #45387192778](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45387192778) passed: the `slot-cpu-performance` Rust workspace tests, `no_std` AArch32/AArch64 checks and full Linkslot renderer/business overlay compile passed.
- [x] GitHub Actions [H700 AArch32/AArch64 compile check #45387293421](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45387293421) passed after adding C `_Static_assert` checks for the ABI v1 state size/field offsets.
- [ ] Add transition-ordering tests for voltage-before-frequency on upshift and frequency-before-voltage on downshift, plus PLL timeout and PMIC failure handling when a hardware transition layer is eventually introduced.
- [ ] Add on-device sustained-load, cold/warm boot, battery/charger, audio-underrun, frame-pacing and thermal-soak tests before enabling Performance/Boost in release builds.

### 8.2 Linux 4.9.170 cpufreq/OPP source variants and parser-v1 limits (2026-11-25)

The device reports Linux `4.9.170`, but no running-kernel build ID or image hash ties it to an exact public source commit. The reviewed branches disagree on raw SID `0x6c00`:

| Candidate | Version / driver blob | `0x6c00` handling | Read safety |
|---|---|---|---|
| [qiaoweibiao/T507_Kernel@98fcc7d3f112e51b0edfb71536da89cb2115106f](https://github.com/qiaoweibiao/T507_Kernel/tree/98fcc7d3f112e51b0edfb71536da89cb2115106f) | 4.9.170 / `e121a4ddf28adb55e0b6bf78b450bd783b8b27bc` | No case; default `supported_hw=0x4` | Unsafe `u32 *` dereference of a two-byte cell without validating `len` |
| [lineage-on-allwinner/android_kernel_allwinner_4.9@ce0c8395b269a6748a10f4917a0e1d06bd4978d0](https://github.com/lineage-on-allwinner/android_kernel_allwinner_4.9/tree/ce0c8395b269a6748a10f4917a0e1d06bd4978d0) | 4.9.170 / `9adc22b6782589b80e58ed86e5f97ff231cfda59` | Explicit `0x6c00 → supported_hw=0x1` | Same unsafe `u32 *` / missing-length-check pattern |
| [CrealityTech/sonic_pad_os@7f37a7fbf4ad59907034715e3d967b9177de1cd4](https://github.com/CrealityTech/sonic_pad_os/tree/7f37a7fbf4ad59907034715e3d967b9177de1cd4) | 4.9.191 / `b524aa53e1bc047624d0bf62ce8504f959095fe7` | Explicit `0x6c00 → supported_hw=0x1` | Length-checked `u8 *` read, byte-wise little-endian assembly; not the target's reported version |

The matching version number is therefore insufficient to identify the actual SID mapping. The two 4.9.170 candidates are both unsafe to copy. The 4.9.191 driver is a useful safety reference only, not evidence that it is the running target implementation.

The Creality 4.9.191 source also demonstrates the named-voltage path: for some speed-cell groups it reads a separate `Icpu` cell and calls `dev_pm_opp_set_prop_name()` with a suffix such as `a0/a1/a2` or `b0/b1/b2`; the OPP core then looks for `opp-microvolt-<suffix>`. In that candidate, `0x6c00` uses the version-mask path, not the named suffix path. This behavior must not be assumed for the RG34XX binary.

#### Parser-v1 gap now fixed in source

The uploaded live-FDT JSON was generated by parser v1. That parser emitted only base `opp-microvolt` and did not export:
- `opp-microvolt-a1/b0/c0/c1/c2...` named voltage properties;
- table-level OPP `nvmem-cells` and `nvmem-cell-names`.

An RG34XX-H device-tree reference ([MustardOS DTS](https://github.com/MustardOS/internal/blob/aef38ed9a870419a0fe4f351dac1bac58a84ba5e/device/rg34xx-h/package/sunxi.dts), blob `64cad2f59b9596dfe6d31e0b52d4f682b1081b4c`) contains precisely these named voltage properties and a table-level `speed` NVMEM reference. This is family-level reference evidence, not proof of the exact live FDT content.

The project parser now emits `voltage_microvolt_variants`, `nvmem_cells` and `nvmem_cell_names`; synthetic tests cover them and the H700 workflow has been wired to execute the new test. The existing report cannot be retroactively enriched from the old JSON—the original `live-fdt.dtb` was not uploaded. A new on-device inventory must regenerate `live-fdt.json` with parser v2 before the OPP table can be reconciled.

#### OPP conclusion

Earlier comparisons of candidate masks against parser-v1's base `opp-microvolt` entries are preliminary. They do not cover all named-voltage OPP variants or prove the table-level SID provider configuration. Neither the T507 default mask `0x4`, the Lineage mask `0x1`, nor mainline candidate bin 5 can be used to generate the RG34XX runtime OPP profile. The exact running cpufreq source/build, parser-v2 live OPP table, voltage-property selection and runtime frequency list must all be reconciled first.

The same-version cpufreq-dt source candidate says `cpuinfo_cur_freq` reads `clk_get_rate(policy->clk)`, while `scaling_cur_freq` uses cached `policy->cur`. Inventory v1 read these fields and the later `clk_summary` sequentially. The collector now performs six uptime-bracketed samples, 200 ms apart, but a new device report is still required. Full Python/Actions verification of the parser/sampler changes is pending.

Do not infer an OPP mask from either vendor candidate or the mainline diagnostic decode. Keep all real DVFS paths disabled.

## 9. Current project status

The H700 scaffold still has **no hardware DVFS driver and no register-write path**. The pure C policy selector under `platform/h700/power` consumes externally supplied, board-approved OPP data and evidence gates. A static runtime owner now exists, but it remains unavailable on the target because no validated RG34XX profile is checked in and no LK startup call site is wired. Host fixtures use synthetic values only and are not the RG34XX operating-point table. The host regression and AArch64/AArch32 ARM/Thumb-2 compile jobs passed in [workflow #45381349400](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45381349400). This validates the policy-selection layer only; it does not validate a physical OPP, PMIC/PLL transition, thermal response or device runtime.

Hardware status remains:
- AXP2202 on I²C-5 / `0x34` is confirmed by the live Linux log.
- CPU0's live-FDT `cpu-supply` phandle `0x4f` is confirmed to resolve to AXP2202 DCDC1. Linux regulator debugfs identifies the `cpu0` consumer and reports 1160 mV in the 2026-11-25 sample; this is framework state, not raw PMIC register readback.
- The SID speed cell is read from the exact bounded NVMEM range: raw `0x6c00`. The mainline reference decoder labels it candidate bin 5; a T507 4.9.170 vendor source falls back to mask 4, a Lineage 4.9.170 source explicitly maps it to mask 1, and a Creality 4.9.191 source maps it to mask 1 using a length-checked read. The parser's earlier “vendor literal match” only referred to the T507 candidate and is not a universal vendor interpretation. No mapping is authorized; `opp_authorization` stays `not_approved`.
- The pinned `sun50iw9` vendor-source audit provides CPUX offsets and a candidate parent-switch/lock sequence. Linux `clk_summary` observes `pll_cpu → cpu` and `pll_periph0=600 MHz`. The first inventory read cpufreq and `clk_summary` at different times; pinned cpufreq-dt source shows `cpuinfo_cur_freq` reads `clk_get_rate()` while `scaling_cur_freq` is cached `policy->cur`, so a schedutil transition may explain the apparent discrepancy. A six-sample uptime-bracketed collector has been added; synchronized runtime capture and exact BL33 CCU readback are still pending.
- The H700 thermal sensor is present; raw calibration fields exist, but the kernel says `AVCC is not calibrated`. Current temperature values are diagnostic, not calibrated control input.
- Linux has CPUs 0–3 online in one shared cpufreq policy. CPU1–CPU3 state at BL31→BL33 remains unknown.

Therefore:
- the **mode model, pure C policy selector, policy-only C service ABI v1 and typed Rust `no_std` client are implemented and host/architecture CI-validated** (see `docs/PROJECT-STATUS.md` for run links);
- the board OPP table, runtime telemetry, PMIC/PLL transition sequence and automatic thermal response remain pending;
- actual frequency/voltage writes and high-performance modes remain disabled until every required evidence gate is closed.



The H700 scaffold does not yet contain a working CPU DVFS driver. The existing runtime has a monotonic clock ABI and a single-core cooperative event loop. The live inventory now confirms the CPU0 DCDC1 rail mapping and the Linux CPU clock-parent relationship, but there is no approved RG34XX OPP table, no direct BL33-entry PMIC/CCU readback, no verified thermal calibration, no CPU PLL transition implementation, and no secondary-core handoff proof.

Therefore:
- the **architecture and UI capability can be prepared now**;
- read-only CPU frequency / thermal telemetry can be added after its registers are verified;
- actual frequency/voltage writes and high-performance modes remain disabled until the gates above are closed.


## 8. Fixed vendor CPUX clock-source audit (2026-11-25)

The additional pinned `sun50iw9` vendor-source review is recorded in [`docs/h700-cpux-clock-source-audit.md`](h700-cpux-clock-source-audit.md). This narrows the investigation to the CPU PLL register, CPU mux/divider fields, `pll_periph0` alternate-parent sequence and lock-mode implementation. It does **not** prove that the exact RG34XX unit exposes these settings identically at BL33 entry.

Important constraints now explicit in the design record:

- Treat all CPU0–CPU3 frequency changes as shared-cluster operations; the reference DTS uses one `clk_pll_cpu` and `opp-shared`.
- Do not toggle PLL enable bit 31 dynamically; the vendor source says it is enabled during boot and uses bit 27 for output gating.
- Do not hardcode the PLL lock-status bit. The vendor `LOCKBIT(x)` macro changes by EVB build configuration, and the exact shipping build setting is not established.
- Do not use `factor_pllcpu_tbl` as an OPP list: the source itself marks that table incorrect, and it contains no validated CPU voltage pairs.
- Do not switch to `pll_periph0` until its actual rate, divider tree and timer continuity are verified on the target.

This audit does not change the policy-only service ABI, runtime owner or QuickRow. Actual DVFS remains unavailable; no PLL/CCU/PMIC writes were added.

### 8.1 Live RG34XX inventory result (2026-11-25)

The full evidence and caveats are recorded in [RG34XX CPUX live read-only audit](rg34xx-cpux-live-audit-2026-11-25.md):

- Linux reports CPU0–CPU3 online, with all four in the same cpufreq policy. Available frequencies are 480–1512 MHz, but this list does not itself prove the approved OPP subset.
- The live FDT maps CPU0's supply to AXP2202 DCDC1; the regulator framework reports the CPU0 consumer at 1160 mV in this sample. This is materially stronger than the earlier two-DTS cross-check but is not direct PMIC register evidence.
- The Linux clock summary reports a current CPU parent chain of pll_cpu → cpu → cpuapb, with pll_cpu/cpu=1512 MHz, cpuapb=378 MHz, axi=756 MHz; pll_periph0 is a separate 600 MHz tree. cpufreq reports 1416 MHz on per-CPU nodes and 1320 MHz on a later policy0 read, so the physical CPU rate is not yet settled.
- The static OPP table has voltage/speed-mask variants; a 1416 MHz / 1160 mV variant numerically matches the regulator sample, but SID raw `0x6c00` has differing vendor candidate interpretations and no approved OPP mask. The parser-v1 JSON also omitted named voltage properties. Do not turn this correlation into an approved OPP.
- Thermal calibration raw bytes are present, but the kernel explicitly rejects AVCC calibration. The temperature telemetry is not certified for Auto/Boost or closed-loop control.
- Linux later reports all four cores online; this does not prove the earlier BL31→BL33 secondary-core state. The shared PLL write gate remains closed.

The Linux capture confirms useful runtime properties but does not authorize any PLL/CCU/PMIC writes.

The read-only inventory collector now also emits a focused, indentation-preserving excerpt for `pll_cpu`, `pll_periph0`, `cpux` and `cpu` entries from Linux `/sys/kernel/debug/clk/clk_summary`. This can make Linux clock-tree/rate evidence easier to review when debugfs exports it, but it is not a direct `PLL_CPU`/`CPU_CFG` MMIO read and does not prove the BL33-entry clock state. Its static safety regression is wired into `.github/workflows/h700-arch-check.yml`; the new push run has not yet been confirmed.

### 8.2 Parser-v2 and synchronized CPUX samples (2026-11-25)

The second read-only report is archived at [reports/rg34xx-inventory-20261125-cpux-v2](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux-v2). It regenerates the live FDT JSON with parser-v2 and adds six uptime-bracketed CPUFreq/clock-tree samples plus eight 150 ms samples including DCDC1 voltage.

- The live `/opp_l_table` now exposes its table-level NVMEM `speed` reference and named `opp-microvolt-a1/b0/b1/c0/c1/c2` variants. The kernel drops 600/792/1032/1296/1344 MHz because selected voltage-property lookup yields `minuV=maxuV=0`. The final Linux table remains 480/720/936/1008/1104/1200/1320/1416/1512 MHz.
- Historical v2 observation: `scaling_cur_freq` appeared to remain at 480 MHz while `cpuinfo_cur_freq` reported 1008/1104 MHz. The v3 eight-sample getter reconciliation supersedes this as a stale/non-atomic cache observation: repeated reads show the cached scaling value moving from 1008 to 1104 MHz after getter reads. Do not classify the old v2 window as a governor clamp or clock-driver rejection.
- CPU0–CPU3 remain in one shared cpufreq policy; CPU supply is AXP2202 DCDC1. Thermal AVCC remains uncalibrated. Raw SID `0x6c00` still does not authorize a speed-bin mask or voltage suffix.

The report is read-only and does not authorize OPP expansion or any PLL/CCU/PMIC write. Keep the service `POLICY_ONLY`, `hardware_applied=0`; continue to block Auto/Boost and CPU voltage/frequency writes.

### 8.3 cpufreq cached-frequency reconciliation and thermal cooling diagnostics (2026-11-25)

The v2 collector's ordered pair `scaling_cur_freq` then `cpuinfo_cur_freq` is
not sufficient to conclude that the clock driver rejected a request. The pinned
candidate `qiaoweibiao/T507_Kernel@98fcc7d3f112e51b0edfb71536da89cb2115106f`
shows why:

- `cpufreq-dt` has `.target_index = set_target` and `.get = cpufreq_generic_get`.
  The target callback calls `dev_pm_opp_set_rate()`; the getter reads
  `clk_get_rate(policy->clk)`.
- In the 4.9 cpufreq core, a target-based driver without `setpolicy` exposes
  `scaling_cur_freq` from cached `policy->cur`.
- `cpuinfo_cur_freq` calls `__cpufreq_get()`. If that driver getter returns a
  non-zero rate different from cached `policy->cur`, the core calls
  `cpufreq_out_of_sync()`, emits a synthetic transition update, assigns the
  returned rate to `policy->cur`, and schedules policy work.

Therefore a read sequence that first sees `scaling_cur_freq=480/1008 MHz` and
then reads `cpuinfo_cur_freq=1008/1104 MHz` can itself reconcile the cached
policy value. The v2 output does not capture `scaling_cur_freq` again after the
getter, so the claimed 1.1-second cached-value mismatch is **not evidence of a
persistent hardware clock failure**. The driver candidate is a pinned public
source reference, not a proven match to the shipping 4.9.170 binary.

The same capture reports CPU thermal zone0 at 46.637°C, with passive trips at
60°C and 70°C and critical at 115°C, policy `power_allocator`. In the pinned
candidate power_allocator implementation, the governor clears passive mode and
allows maximum power when temperature is below its switch-on passive trip. That
makes active CPU-zone power_allocator throttling less likely at this particular
sample, but the report did not include thermal cooling-device states/bindings,
and exact source correspondence is unproven. Other cooling devices, policy
limits or non-thermal drivers are not ruled out.

The collector has therefore been extended again (commit
`b421b077e5f27c95bf61679762b70a86d66131cb`) to capture, read-only:

- resolved path identities of `policy0` and `cpu0/cpufreq`, plus transition
  counters where exported;
- thermal `cooling_deviceN` `type/cur_state/max_state` and each thermal
  zone's `cdevN` binding;
- eight 150 ms samples that read cached scaling frequency, invoke the
  `cpuinfo_cur_freq` getter, read cached scaling frequency again, repeat the
  getter sequence, then capture CPUX clock tree, DCDC1 voltage and uptime.

The new host shell/Python regression and Windows PowerShell workflow are
wired in. H700 compile/regression run
[#45526953772](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45526953772)
and boot audit tools
[#45526953774](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45526953774)
passed. The v3 on-device capture is now archived under
[`reports/rg34xx-inventory-20261125-cpux-v3`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux-v3)
and analyzed in §8.6. The getter sequence shows that cached `scaling_cur_freq`
can reconcile after `cpuinfo_cur_freq`; cooling devices are at 0/8 and 0/5
in this snapshot. This supersedes the v2 uncertainty, but does not prove the
shipping driver binary or certify AVCC. Keep the service POLICY_ONLY,
`hardware_applied=0`; no governor/limit/PMIC/PLL/OPP writes are approved.


### 8.4 Runtime OPP-list fingerprint points to a conditional `b1` property path

A new static comparison of the parser-v2 live OPP entries against the device's `scaling_available_frequencies` list found an exact match under this **conditional** Linux 4.9 OPP-core path:

1. no `supported_hw` array is registered for the OPP table;
2. the selected OPP voltage property name is `b1`;
3. the `-ab` OPP nodes have no `opp-supported-hw` property;
4. only an OPP with a valid selected `opp-microvolt-b1` (or a base fallback) survives the CPU regulator voltage filter.

Under that model, the untagged `b1` nodes produce exactly 480/720/936/1008/1104/1200/1320/1416/1512 MHz. The untagged `-ab` nodes at 600/792/1032/1296/1344 MHz have no `b1` and no base voltage, matching the five OPPs that the boot log says were rejected with `minuV=maxuV=0`.

This is a much closer structural match than the previously examined stock-style 4.9.170 `supported_hw` mask candidates: those candidates do not reproduce the runtime list when their masks and voltage lookups are applied to the parsed table. It suggests the shipping kernel may use a vendor-specific property-name selection path rather than the simple SID-to-mask path, but the exact running driver/source is not identified.

**Important ambiguity in parser-v2:** an absent `opp-supported-hw` property and a present zero-cell `opp-supported-hw` property were both serialized as an empty `supported_hw_cells` array. The Linux OPP core treats them differently when no hardware mask is registered: absent means the OPP may be accepted; present (even with zero cells) means unsupported. The parser was therefore extended in commit `284f48e71ad93145c15bbf034d88d2f4c404789e` to emit `supported_hw_present`, `supported_hw_cell_count`, `voltage_microvolt_present`, and `voltage_microvolt_variant_names`. Synthetic regression covers absent and empty-present properties. H700 Linux/Windows validation passed in [#45529626035](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45529626035) and [#45529626080](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45529626080).

The parser-v3 report is now available and confirms the property-presence condition. The corrected analyzer finds one exact structural fingerprint—no table-level `supported_hw` array plus the `b1` voltage suffix—against the nine runtime frequencies. This identifies a matching OPP-selection model, not the exact shipping kernel code path; it does not decode SID raw `0x6c00`, approve any OPP or authorize frequency/voltage writes. Keep the runtime `POLICY_ONLY`, `hardware_applied=0`, and all actual DVFS controls disabled.

### 8.5 Read-only OPP candidate enumerator and current verification (2026-11-25)

The repository now includes `tools/analyze_h700_opp_candidates.py` and `tools/test_analyze_h700_opp_candidates.py`. The tool consumes parser-v3 `live-fdt.json` and the paired `cpu-power-runtime.txt`, then evaluates:

- no `supported_hw` array registered, where the presence of any `opp-supported-hw` property rejects the OPP, including a zero-cell property;
- a one-cell table-level `supported_hw` model: all 32 atomic u32 bits, unique non-zero masks observed in OPP properties, and one exact set-cover witness when a mask reproduces the runtime set. OPP `opp-supported-hw` arrays are treated as alternative versions, where any one-cell version may intersect the active table mask;
- every voltage suffix in the OPP table, applying Linux 4.9 semantics: a selected named property is used when present; the base property is used only when the named property is absent.

It reports exact frequency-list matches and nearest candidates. The mask probes are bounded hypotheses, not an exhaustive enumeration of all 2^32 table masks; a set-cover search can derive one mathematical witness for an exact match without interpreting it as SID data. Voltage parsing checks property shape and min/target/max ordering only; regulator selector tables and `regulator_is_supported_voltage()` are not simulated. It does not read SID, identify the exact running driver, authorize an OPP or touch hardware. Synthetic tests cover the nine-rate `b1` fingerprint, five dropped rates, high-bit masks, multiple OPP version cells and voltage triplet ordering.

The initial CI run exposed two implementation defects (an over-escaped frequency-block regex and a malformed test replacement); both were corrected. A source-model review then found that the first diagnostic probed only low-byte masks and considered only the first OPP version cell. The current implementation probes all 32 atomic bits, masks present in OPP properties, supports alternative version cells and derives a witness mask by set-cover search when an exact frequency match exists. The first witness-fixture run then showed the test itself allowed an unintended low-bit solution; its excluded-frequency fixture was corrected. Current commit [`da41806e47dbd2e0d57e23e9ff6ff405a5514e29`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/da41806e47dbd2e0d57e23e9ff6ff405a5514e29) passes:
- [H700 AArch32/AArch64 compile + host regression #45537478198](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45537478198), including AArch64, AArch32 ARM-state, AArch32 Thumb-2, and storage/CPU/OPP/MMU/PIO host tests;
- [H700 boot audit tools #45537478202](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45537478202), including Linux and Windows Python unit tests and PowerShell syntax validation.

Parser-v3 is now uploaded and analyzed. The 38-entry live OPP table and five identical runtime frequency lists yield one exact candidate: no table-level `supported_hw` array plus the `b1` named-voltage property. Every `-ab` node has `supported_hw_present=false`; all nine runtime rates have a `b1` variant, while the five boot-rejected rates do not. No one-cell table mask produces an exact witness. Runtime samples at 1416/1150 mV, 1104/1000 mV and 1200/1050 mV also match `b1` voltage declarations. This narrows the property-selection model but does not prove the shipping binary selects `b1` or that SID `0x6c00` maps to it. Keep `opp_authorization=not_approved`, `POLICY_ONLY`, `hardware_applied=0`, and every DVFS write disabled.

### 8.6 Parser-v3: unique no-mask + b1 structural fingerprint and getter reconciliation
### 8.7 Public `sun50iw9` cpufreq-NVMEM source candidates do not match the live v3 fingerprint

The v3 result was compared with four public driver variants. The two sources explicitly labelled Linux 4.9.170 (`qiaoweibiao/T507_Kernel` and `lineage-on-allwinner/android_kernel_allwinner_4.9`) both read only the `speed` cell, but map raw `0x6c00` to different one-cell masks (`0x4` vs `0x1`) and do not select a named voltage property. A Creality 4.9.191 variant does call `dev_pm_opp_set_prop_name()`, but it requires both `speed` and `Icpu`; its `0x6c00` path still selects mask 1, while its `b1` suffix is generated for other speed-cell groups using Icpu data. The RG34XX live OPP table exposes only the `speed` NVMEM reference. A Radxa BSP variant also reads speed only, but maps `0x6c00` to `a0` plus a supported-hw mask.

None of these candidates reproduces the measured no-mask + `b1` table/runtime-list combination. This makes a different or locally customized vendor driver/build more likely than a direct match to the public candidates, but does not identify the actual implementation. The target's running kernel still has no captured build ID or matching source commit. Keep the runtime unavailable and write-free until that identity and all hardware safety gates are established.

The v3 field capture is archived at [reports/rg34xx-inventory-20261125-cpux-v3](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-cpux-v3), captured 2026-11-25 03:19:48 UTC. The original live FDT is hash-identical to v1/v2; parser-v3 preserves `supported_hw_present`, `supported_hw_cell_count`, named voltage variants and the table's `speed` NVMEM reference. The runtime file includes matching CPU0–CPU3 and policy0 lists plus eight uptime-bracketed getter/clock/DCDC1 samples.

- The live `/opp_l_table` has 38 entries; all 14 `-ab` nodes explicitly have no `opp-supported-hw` property.
- Five `scaling_available_frequencies` blocks agree exactly on 480/720/936/1008/1104/1200/1320/1416/1512 MHz.
- The corrected analyzer returns one exact candidate only: no table-level `supported_hw` array + voltage suffix `b1`. Base voltage and other named suffixes fail exact comparison; no one-cell mask model produces an exact set-cover witness.
- The nine `b1` voltages are 900/900/900/950/1000/1050/1100/1150/1160 mV; the five dropped frequencies do not have a matching `b1` voltage on their untagged `-ab` entries.
- Runtime clock / regulator-framework pairs at 1416/1150, 1104/1000 and 1200/1050 MHz/mV numerically match `b1`; these are not raw CCU/PMIC readbacks and the attributes are not atomic.

This is now a unique structural fingerprint, not merely the earlier conditional hypothesis. The exact Linux 4.9.170 binary/source path that selects the property name remains unproven. Do not claim that SID raw `0x6c00` maps to `b1`. The analyzer does not model PMIC selector tables or `regulator_is_supported_voltage()`, and no real OPP is approved.

**cpufreq getter result:** Sample 2 shows cached scaling 1008 → `cpuinfo_cur_freq` 1104 → cached scaling 1104; `pll_cpu` is 1104 MHz and DCDC1 framework voltage is 1000 mV. Sample 4's getter reads 1104 while the later clock tree reads 1200 and DCDC1 1050 mV, proving the reads are non-atomic. Samples 5/6 update the cache only later in the second getter/read sequence; samples 7/8 repeat 1008 → 1104. The v2 480 MHz window is therefore a stale/non-atomic sysfs observation, not evidence of a persistent clamp or failed clock transition.

CPU zone0 is 47.69 °C with passive trips at 60/70 °C and critical at 115 °C. `thermal-cpufreq-0` is 0/8 and `thermal-devfreq-0` is 0/5, both bound to CPU zone0; no active cooling-state throttling is visible in this snapshot. AVCC calibration remains rejected. The 1 GiB RAM window, 46 memblock reservations and 64 MiB CMA are unchanged; `[0x7ec00000,0x7fc00000)` remains un-attributed and excluded.

Next gates: run the v4 read-only provenance collector to capture kernel build notes/build-id, cpufreq platform binding and filtered OPP/NVMEM symbol names; match the actual shipping cpufreq/OPP binary/source path and explain `b1` selection; prove BL31→BL33 CPU1–CPU3 state and timer continuity; obtain safe read-only raw PMIC/CCU evidence; certify an independent temperature path. Keep the CPU service unavailable and write-free.
### 8.8 Read-only kernel / CPUFreq driver provenance collector

The collector now emits `kernel-driver-provenance.txt` using read-only sources: `/proc/version`, `/proc/sys/kernel/osrelease`, `/sys/kernel/notes` (SHA-256 and a bounded 256-byte prefix), optional kernel config exports, cpufreq-related platform driver/device `driver` / `device` / `of_node` symlink targets, module names, and filtered OPP/cpufreq/NVMEM kallsyms **symbol names only** (addresses are discarded). It does not call vendor SoC helpers, load modules, invoke ioctl/SMC, or write sysfs/PMIC/PLL/CCU controls.

Validation: [H700 boot audit tools #45543371548](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45543371548) passed on Linux and Windows, including the inventory write-path safety checks and PowerShell wrapper syntax. [H700 AArch32/AArch64 compile + host regression #45543371507](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45543371507) passed storage/CPU/LED/MMU/PIO host regressions plus AArch64, AArch32 ARM-state and AArch32 Thumb-2 object compilation.

Next collection: run `scripts/run-rg34xx-inventory.ps1` with output directory `build/rg34xx-inventory-20261125-cpux-v4`, then upload the text report folder to `reports/rg34xx-inventory-20261125-cpux-v4/`. No additional device-side test or control write is requested. The resulting provenance may still be incomplete if the running kernel does not export `/sys/kernel/notes`, config or kallsyms; absence must be recorded as unavailable, not inferred.