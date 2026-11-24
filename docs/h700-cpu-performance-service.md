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

The RG34XX live inventory identifies four Cortex-A53 CPUs and **AXP2202** as the PMIC (x-powers,axp2202). This is the actual RG34XX evidence:
- reports/docs/rg34xx-live-inventory-2026-11-22.md

Do not substitute the PMIC configuration from a different H700 handheld. For example, the mainline sun50i-h700-anbernic-rg35xx-2024.dts describes an AXP717-based board. Its vdd-cpu DCDC1 range must not be copied into this RG34XX port. The RG34XX board's exact CPU supply rail, PMIC regulator channel, voltage constraints, sequencing and board revision still need to be established from its own live FDT/vendor source and PMIC readback.

The current H700 handoff audit also leaves CPU1–CPU3 state unknown at BL31→BL33. CPU frequency is a cluster-level clock domain, not a per-Linkslot-core setting. Changing the shared CPU clock while an unaccounted secondary PE is running is unsafe. Keep the CPU service write path disabled until the single-core handoff invariant is positively proven.

## 3. What the reference GammaOS Pico does—and what must not be copied

The reference LK has an in-game CPU Clock row and code paths that request named frequencies. However, its MT6785 implementation rewrites the MediaTek ARM-PLL PCW and explicitly does **not** scale CPU voltage. Its core-porting notes describe instability when sustained clocks exceed the fixed boot-voltage operating point. This is useful as a UI/policy reference, not as an H700 DVFS implementation.

Reference source anchors:
- platform/mt6785/pll.c: ayaneo_set_cpu_mhz() / ayaneo_get_cpu_mhz()
- emu/gba/gba_driver.c: CPU Clock menu OPP grid and run-ahead clock tiers
- documentation/development/core-porting-notes.md: fixed boot Vproc warning and stable menu clock discussion

Do not port the raw MTK PLL register sequence, the 600–2000 MHz menu list, or its assumption that PLL-only changes are safe. Those values belong to another SoC and another voltage setup.

## 4. H700 OPP/DVFS evidence

Mainline H700 support is useful source material:
- The Linux sun50i-cpufreq-nvmem driver identifies an H700 silicon speed bin using NVMEM/eFuse data.
- The H700 OPP patch added support for 1.008 GHz, 1.032 GHz and 1.512 GHz points after earlier testing had found some of them unstable; the patch states that 1.512 GHz needs 1.16 V for its speed-bin-5 entry.
- The related RG35XX-2024 DTS raises the AXP717 vdd-cpu regulator ceiling to 1.16 V. That is evidence for that board, **not** a voltage instruction for RG34XX/AXP2202.

Source references:
- https://lists.infradead.org/pipermail/linux-arm-kernel/2024-June/933097.html
- https://lists.infradead.org/pipermail/linux-arm-kernel/2024-June/933098.html
- https://lists.infradead.org/pipermail/linux-arm-kernel/2024-June/933099.html
- https://lists.infradead.org/pipermail/linux-arm-kernel/2024-June/933100.html
- https://github.com/torvalds/linux/blob/master/drivers/cpufreq/sun50i-cpufreq-nvmem.c
- https://github.com/torvalds/linux/blob/master/drivers/clk/sunxi-ng/ccu-sun50i-h616.c
- https://github.com/torvalds/linux/blob/master/drivers/regulator/axp20x-regulator.c

The OPP table for this port must be generated/selected only after we know:
1. the exact RG34XX board revision and CPU supply rail/regulator channel;
2. the H700 SID speed-bin result, with unknown bins failing closed;
3. the board's permitted minimum/maximum CPU voltage and reliable regulator range;
4. the exact frequency/voltage pairs approved for that silicon bin and this board;
5. thermal sensor availability and safe sustained temperature/power limits.

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

## 8. Required hardware and safety gates

- [ ] Extract the exact RG34XX live FDT regulator subtree and identify which AXP2202 rail supplies the CPU cluster; corroborate with board-specific vendor DTS/source or non-destructive PMIC readback.
- [ ] Identify AXP2202 PMIC bus/address, regulator channel, selector encoding, ramp/settling behaviour and safe operating range from the matching datasheet/driver. Do not infer the CPU rail from regulator numbering.
- [ ] Read/translate the H700 SID speed bin using the same semantics as the Linux NVMEM driver; unknown or unsupported bins disable elevated OPPs.
- [ ] Identify H700 CPUX PLL/CCU sequence and verify safe clock-parent switching, lock polling, divider constraints and timer continuity from matching H616/H700 sources.
- [ ] Resolve CPU1–CPU3 state at BL31→BL33. Until then, CPU PLL writes remain disabled.
- [ ] Implement/read the H700 thermal sensor path and validate sensor calibration; define throttling and emergency fallback temperatures.
- [ ] Establish cold-boot voltage/clock readback and a conservative board-approved OPP table before adding any write path.
- [ ] Add host tests for OPP selection, speed-bin filtering, request clamping, voltage-before-frequency ordering on upshift, clock-before-voltage ordering on downshift, PLL timeout, regulator failure, thermal clamp and unknown-hardware fail-closed behaviour.
- [ ] Add on-device sustained-load, cold/warm boot, battery/charger, audio-underrun, frame-pacing and thermal soak tests before enabling Performance/Boost in release builds.

## 9. Current project status

The H700 scaffold does not yet contain a working CPU DVFS driver. The existing runtime has a monotonic clock ABI and a single-core cooperative event loop, but there is no approved CPU OPP table, no verified AXP2202 CPU-supply mapping, no H700 CPU PLL transition implementation, no thermal policy, and no secondary-core handoff proof.

Therefore:
- the **architecture and UI capability can be prepared now**;
- read-only CPU frequency / thermal telemetry can be added after its registers are verified;
- actual frequency/voltage writes and high-performance modes remain disabled until the gates above are closed.
