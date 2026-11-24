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

Two independently maintained RG34XX-specific device trees (Knulli and MustardOS) map `cpu@0`'s `cpu-supply` phandle to AXP2202 **DCDC1**. In those references, DCDC1 is named `axp2202-dcdc1`, has a 500000–1540000 µV range, 250 µs ramp delay and 1000 µs enable-ramp delay. This is strong board-specific cross-check evidence, but the current project's live-FDT summary is a reduced report and does not include the complete `cpu-supply` / regulator phandle subtree. Treat DCDC1 as the leading RG34XX CPU-rail candidate, not yet as a proven live readback for this exact board revision.

The matching Allwinner AXP2202 U-Boot reference driver describes DCDC1 voltage selector register `0x83` (mask `0x7f`), the DCDC enable register `0x80` (bit 0), 10 mV steps up to 1.2 V and 20 mV steps above 1.2 V, with a declared 0.5–1.54 V control range. These are register-format and driver-range references only; they do not authorize writes or establish the actual regulator voltage left by the stock boot chain. Never copy AXP717 descriptors or regulator limits from RG35XX-2024 into the RG34XX board profile.

The same RG34XX device-tree reference exposes the SID speed cell at `/sunxi-sid-ng@03006000/speed@00` (offset 0, 2 bytes) and thermal calibration cell at `/sunxi-sid-ng@03006000/calib@14` (offset `0x14`, 8 bytes). The device's current kernel log reports the H700 thermal sensor at `0x05070400` with **avcc is not calibrated**. A temperature sensor exists, but current calibration evidence is insufficient for closed-loop automatic DVFS.

The current H700 handoff audit also leaves CPU1–CPU3 state unknown at BL31→BL33. CPU frequency is a shared cluster clock domain, not a per-Linkslot-core setting. Changing the shared CPU clock while an unaccounted secondary PE is running is unsafe. Keep the CPU service write path disabled until the single-core handoff invariant is positively proven.

## 3. What the reference GammaOS Pico does—and what must not be copied

The reference LK has an in-game CPU Clock row and code paths that request named frequencies. However, its MT6785 implementation rewrites the MediaTek ARM-PLL PCW and explicitly does **not** scale CPU voltage. Its core-porting notes describe instability when sustained clocks exceed the fixed boot-voltage operating point. This is useful as a UI/policy reference, not as an H700 DVFS implementation.

Reference source anchors:
- platform/mt6785/pll.c: ayaneo_set_cpu_mhz() / ayaneo_get_cpu_mhz()
- emu/gba/gba_driver.c: CPU Clock menu OPP grid and run-ahead clock tiers
- documentation/development/core-porting-notes.md: fixed boot Vproc warning and stable menu clock discussion

Do not port the raw MTK PLL register sequence, the 600–2000 MHz menu list, or its assumption that PLL-only changes are safe. Those values belong to another SoC and another voltage setup.

## 4. H700 OPP/DVFS evidence

Mainline H700 support is useful source material:
- Linux's `sun50i-cpufreq-nvmem` driver matches both `allwinner,sun50i-h616` and `allwinner,sun50i-h700`. It uses the H616 speed-bin decoder for this family and reads a two-byte NVMEM cell.
- The H616/H700 decoder examines raw speed-bin values in the low 16 bits. Known values map to bins 0–5, including a silicon-revision-dependent mapping for `0x2400` / `0x7400` / `0x2c00` / `0x7c00`. Linux falls back to bin 0 for unknown values. This project will **not** inherit that permissive fallback: unknown raw values or unavailable silicon-revision data disable elevated OPPs and report the bin as unknown.
- The RG34XX vendor DTS contains its own `opp_l_table` from 480 MHz upwards, with per-speed/silicon variants. These are vendor configuration references, not a validated frequency-voltage table for the current device at BL33 entry.
- The mainline H616 CPU OPP table includes points up to 1.512 GHz with speed-bin-specific voltages. Those values are useful for comparing the Linux driver model only; they are not a release OPP grid for this RG34XX.
- The RG34XX DTS DCDC1 range/ramp metadata and AXP2202 selector register definitions are documented in section 2 above. The AXP717-based RG35XX-2024 1.16 V configuration is not the RG34XX voltage policy.

The mainline H616 CCU source gives an initial clock-tree reference: PLL-CPUX is at CCU offset `0x000` (enable bit 31, lock bit 28; multiplier field in the register's low bits), and the CPUX clock mux is at offset `0x500`, bits 24–26, with oscillator / PLL parents. These are offsets within the CCU block, not a complete H700 register-access contract. The exact RG34XX CCU base, safe parent-switch sequence, PLL programming limits, lock wait rules and timer continuity still require source/board verification. No raw MMIO implementation is approved.

Source references:
- https://github.com/knulli-cfw/knulli-linux/blob/57afa662009641d181f57241bcc480119b131530/package/boot/uboot-h700/rg34xx/boot_package/rg34xx.dts
- https://github.com/MustardOS/internal/blob/bb67c448c4e2ab73ac9797d257e1492dc49cff45/device/rg34xx-h/package/sunxi.dts
- https://github.com/CrealityTech/sonic_pad_os/blob/7f37a7fbf4ad59907034715e3d967b9177de1cd4/lichee/brandy-2.0/u-boot-2018/drivers/sunxi_power/pmu_axp2202.c
- https://github.com/CrealityTech/sonic_pad_os/blob/7f37a7fbf4ad59907034715e3d967b9177de1cd4/lichee/brandy-2.0/u-boot-2018/include/sunxi_power/pmu_axp2202.h
- https://github.com/torvalds/linux/blob/master/drivers/cpufreq/sun50i-cpufreq-nvmem.c
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

The C facade is intentionally not wired to a PLL/PMIC actuator. The typed Rust client exists and is host-tested, but it is not yet wired into Linkslot `App`/settings or a C service initialization path. Until a real readback backend exists, actual clock/voltage must remain unavailable. Do not show planned MHz as measured MHz and do not expose an applied/success state based only on `POLICY_ONLY`.

#### 7.1.1 Static H700 runtime owner (fail-closed, no board profile)

The platform-owned lifetime boundary now exists in `h700_cpu_performance_runtime.h/.c`:

- One zero-initialized, static `h700_cpu_perf_service_v1_t` is owned by the H700 C runtime. The accessor returns `NULL` until initialization succeeds; the owner is intended to be initialized once during single-core startup, before the Linkslot event loop.
- Initialization requires a board-owned profile with ABI/size, RG34XX board ID, non-zero board revision, an explicit approval marker, zero reserved fields, a structurally valid OPP table and all mandatory CPU evidence gates. It preflights Power Saver, Balanced, Performance and Auto against the policy selector before publishing the service pointer.
- The approval marker is an assertion by trusted platform code, **not** a hardware proof or security boundary. The profile must be immutable board-owned source/configuration and must not be provided by user data, a ROM, an untrusted file or the Linkslot UI.
- The Rust client now has a `CpuPerformanceRuntime` token that calls the C runtime status/getter ABI and returns a distinct `RuntimeUnavailable` error when the C owner is not ready. Its unsafe acquisition contract requires successful platform initialization, no owner reinitialization for the token lifetime and exclusive mutation; temporary `CpuPerformanceClient` access is obtained by mutable borrow.
- Every initialization attempt first revokes prior readiness. A missing, rejected or incomplete profile leaves the runtime unavailable and does not preserve a stale service handle. The underlying service ABI also revokes its prior initialized state before validating a full-size reinitialization request.
- Host regression tests use clearly labelled synthetic data only. No RG34XX profile has been checked in, because the real board OPP table and evidence gates are still missing. There is also no GammaOS LK startup hook or Linkslot App wiring calling this owner yet. Consequently the real RG34XX runtime remains unavailable and the UI row remains intentionally absent.
- The owner performs no MMIO, PMIC read/write, PLL operation or telemetry sampling. Successful initialization only exposes the policy-only service; it does not apply hardware performance changes.
- The three C objects (policy, service facade and runtime owner) are now listed in `platform/h700/rules.mk` for the future LK target link. The LK project guard asserts the object entries and matching source files. This is only build-graph registration: the full GammaOS LK H700 target has not linked, and the board memory approval gate still prevents a real image build. No startup code calls the owner yet.

The new host test covers unavailable-by-default state, NULL/unapproved/wrong-board/missing-gate/missing-OPP rejection, stable static ownership, policy-only reporting and revocation of a previously borrowed handle. Local GCC host execution and local AArch64/AArch32 ARM/AArch32 Thumb-2 freestanding compile probes passed for this slice; the corresponding GitHub Actions result for the latest push has not yet been confirmed.

### 7.2 Linkslot 1.0.45 settings integration contract

The upstream UI extension point has been audited in the immutable 1.0.45 snapshot:

- `slot-ui/src/quick_menu.rs` currently has `QuickRow::SYSTEM_SETTINGS: [QuickRow; 6]`: DateTime, Language, LidTimeout, Mods, SystemUpgrade and About.
- `QuickRow::index()` is its enum discriminant and is also used to address quick-value state. `QuickRow::ALL` is a fixed-size table. The existing Language row was deliberately appended at the end of `QuickRow::ALL` and declared immediately before `SplitLink`, so earlier persisted indices do not move.
- `QuickValue::ALL` currently has 37 base values. The Language values are the last two entries. A future CPU mode value family must be appended, not inserted between existing variants.
- The CPU row belongs in System settings, but it should be appended without renumbering existing `QuickRow` or `QuickValue` variants. Update the fixed array lengths, `page()`, `parent()`, `key()`, icon fallback/glyph, value labels, both translation catalogues, App's `quick_value` and `change_setting` matches, and the row/index parity tests together.
- The visible selectable profiles should initially be **Power Saver / Balanced / Performance / Auto**. Do not expose Boost until the board-specific voltage, thermal and sustained-load gates are approved.
- Persist only the requested policy (for example, a validated text value under `linkSlot/Config/`); never persist a synthetic MHz target as an actual frequency. The UI must show the chosen policy separately from measured telemetry and make “frequency telemetry unavailable / policy not applied” explicit when the H700 service is not ready.
- The C service now has a static H700 runtime owner and a fail-closed board-profile validation API. However, no approved RG34XX OPP/profile is checked in and no GammaOS LK startup path calls the owner; the Rust client is not yet wired into Linkslot `App`/settings. Therefore the UI row is **not yet implemented**. Add it only after a real board profile and boot-to-App adapter path exist, or the user could choose modes that have no effect.

## 8. Required hardware and safety gates

- [ ] Export the complete `cpu-supply` / AXP2202 regulator subtree from the exact RG34XX live FDT and confirm the DCDC1 mapping on this unit/board revision. Knulli and MustardOS RG34XX DTSs already cross-check DCDC1 as cpu0 supply, but live-FDT readback is still outstanding.
- [x] Confirm the current Linux runtime PMIC observation: AXP2202 on I²C-5, address `0x34`. Match generic AXP2202 reference DCDC1 selector/control fields (`0x83` / `0x80`) without enabling writes.
- [ ] Read-only PMIC voltage and enable-state readback at BL33 entry; verify that the stock firmware left the expected rail at the expected voltage before any transition is designed.
- [ ] Read the actual H700 SID speed cell at `0x03006000 + 0x00` (two bytes), establish the required SoC revision semantics, and fail closed for unknown raw values.
- [ ] Identify H700 CPUX PLL/CCU sequence and verify safe clock-parent switching, lock polling, divider constraints, exact CCU base and timer continuity from matching H616/H700 sources.
- [ ] Resolve CPU1–CPU3 state at BL31→BL33. Until then, CPU PLL writes remain disabled.
- [ ] Implement/read the H700 thermal sensor path and validate AVCC calibration; the current RG34XX kernel capture reports “avcc is not calibrated”, so automatic thermal DVFS is blocked.
- [ ] Establish cold-boot voltage/clock readback and a conservative board-approved OPP table before adding any write path.
- [x] Implement a pure C OPP policy selector with no MMIO/PMIC access, strict speed-bin masks, board-voltage and thermal caps, and fail-closed evidence gates; add host regression tests and AArch64/AArch32 compile coverage.
- [x] GitHub Actions [H700 CPU OPP / AArch32-AArch64 regression #45381349400](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45381349400) passed: policy host regressions plus AArch64, AArch32 ARM-state and AArch32 Thumb-2 freestanding object compilation.
- [x] GitHub Actions [H700 Linkslot 1.0.45 runtime slices #45387192778](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45387192778) passed: the `slot-cpu-performance` Rust workspace tests, `no_std` AArch32/AArch64 checks and full Linkslot renderer/business overlay compile passed.
- [x] GitHub Actions [H700 AArch32/AArch64 compile check #45387293421](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45387293421) passed after adding C `_Static_assert` checks for the ABI v1 state size/field offsets.
- [ ] Add transition-ordering tests for voltage-before-frequency on upshift and frequency-before-voltage on downshift, plus PLL timeout and PMIC failure handling when a hardware transition layer is eventually introduced.
- [ ] Add on-device sustained-load, cold/warm boot, battery/charger, audio-underrun, frame-pacing and thermal-soak tests before enabling Performance/Boost in release builds.

## 9. Current project status

The H700 scaffold still has **no hardware DVFS driver and no register-write path**. The pure C policy selector under `platform/h700/power` consumes externally supplied, board-approved OPP data and evidence gates. A static runtime owner now exists, but it remains unavailable on the target because no validated RG34XX profile is checked in and no LK startup call site is wired. Host fixtures use synthetic values only and are not the RG34XX operating-point table. The host regression and AArch64/AArch32 ARM/Thumb-2 compile jobs passed in [workflow #45381349400](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45381349400). This validates the policy-selection layer only; it does not validate a physical OPP, PMIC/PLL transition, thermal response or device runtime.

Hardware status remains:
- AXP2202 on I²C-5 / `0x34` is confirmed by the live Linux log.
- DCDC1 as RG34XX CPU supply is corroborated by two dedicated RG34XX DTS references, but the current live-FDT summary does not yet provide the matching regulator phandle tree.
- The H700 SID speed cell location and generic Linux decoding are known from the reference DTS/driver, but the raw value and correct silicon revision are not read from the target unit.
- The CPUX PLL and mux offsets are known from mainline H616 CCU source, but the exact H700 register access/clock switching sequence is not implemented.
- The H700 thermal sensor is present, but current logs say AVCC is not calibrated.
- CPU1–CPU3 handoff state is still unknown at BL31→BL33.

Therefore:
- the **mode model, pure C policy selector, policy-only C service ABI v1 and typed Rust `no_std` client are implemented and host/architecture CI-validated** (see `docs/PROJECT-STATUS.md` for run links);
- the board OPP table, runtime telemetry, PMIC/PLL transition sequence and automatic thermal response remain pending;
- actual frequency/voltage writes and high-performance modes remain disabled until every required evidence gate is closed.



The H700 scaffold does not yet contain a working CPU DVFS driver. The existing runtime has a monotonic clock ABI and a single-core cooperative event loop, but there is no approved CPU OPP table, no verified AXP2202 CPU-supply mapping, no H700 CPU PLL transition implementation, no thermal policy, and no secondary-core handoff proof.

Therefore:
- the **architecture and UI capability can be prepared now**;
- read-only CPU frequency / thermal telemetry can be added after its registers are verified;
- actual frequency/voltage writes and high-performance modes remain disabled until the gates above are closed.
