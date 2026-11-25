# Review: pvaibhav/slot 120 Hz implementation and RG34XX applicability

**Reviewed source:** [`pvaibhav/slot`](https://github.com/pvaibhav/slot), branch `pvaibhav-improvements`, pinned HEAD `5206e3fa39aa4a9b702cd9048a07dab3b5ab95da` (2026-11-20). This review concerns the source at that commit, not a claim that its binary or register writes were tested on our RG34XX.

## Executive finding

This fork is materially ahead of our current H700 display code at the **hardware-control layer**. It has a real Linux-side writer, exact DT allowlisting, ordered PLL/divider writes, register readback, a 60-Hz rollback attempt, wake reapplication, and measurement tooling that verifies scanout via the LCD interrupt rather than relying on the stale `ref_fps` field.

It is **not a drop-in implementation for our RG34XX non-SP**. The fork's code allows `BASEOS_TARGET=rg34xx` only when the DT panel name is `fog_fj035fhd05_v1`, while our latest live RG34XX FDT identifies the panel as `rg34xx_v1`. The fork also requires 26 MHz / 820×536 timing cells. Its own hardware note says the non-SP RG34XX entry has no connected-device live test. Do not weaken these checks or copy its register constants to make the probe pass.

Its product policy also differs from ours: it applies 120 Hz globally on supported devices at startup and wake, and explicitly does not change physical refresh during emulation. It does not implement our per-scene policy or an independent GAME 120-Hz test switch. Our policy now keeps UI preference (60/120 Hz) independent from `game_120hz_enabled` (default off, GAME 60 Hz; opt-in requests 120 Hz). This is a policy request only and awaits HAL/settings integration and controlled measurements.

## What the fork actually implements

Source files at the pinned revision:

- [`crates/slot-power/src/refresh.rs`](https://github.com/pvaibhav/slot/blob/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da/crates/slot-power/src/refresh.rs): panel/DT gate, register writer, readback and rollback.
- [`crates/slot-power/src/device.rs`](https://github.com/pvaibhav/slot/blob/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da/crates/slot-power/src/device.rs): connects the panel controller to the platform power service.
- [`crates/slot-power/src/power.rs`](https://github.com/pvaibhav/slot/blob/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da/crates/slot-power/src/power.rs): wake path calls `resume_display()` before restoring the backlight.
- [`crates/slot/src/app.rs`](https://github.com/pvaibhav/slot/blob/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da/crates/slot/src/app.rs) and [`slot_state.rs`](https://github.com/pvaibhav/slot/blob/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da/crates/slot-store/src/slot_state.rs): startup application and default `high_refresh=true`.
- [`tools/refresh/HARDWARE.md`](https://github.com/pvaibhav/slot/blob/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da/tools/refresh/HARDWARE.md), [`README.md`](https://github.com/pvaibhav/slot/blob/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da/tools/refresh/README.md), and [`RESULTS.md`](https://github.com/pvaibhav/slot/blob/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da/tools/refresh/RESULTS.md): hardware matrix and test methodology.

### Safety gates and register sequence

The runtime gate requires all of the following before exposing the control:

1. An exact `BASEOS_TARGET` allowlist entry.
2. A valid `aliases/lcd0` path within the live device tree, with no conflicting alias entries.
3. An exact `lcd_driver_name` byte string.
4. 720×480 geometry, `lcd_ht=820`, `lcd_vt=536`, and `lcd_dclk_freq=26`.
5. Writable `/sys/class/sunxi_dump/write` and `dump` interfaces.

Its register pair is `DIVIDER=0x06511044` and `PLL=0x03001040`. The 120-Hz path writes the divider first (`0xf000000a`), then the PLL (`0x88002b03`). The 60-Hz path reverses the order: PLL `0x88001703`, then divider `0xf000000b`. Both values are read back. A failed write or readback triggers a best-effort 60-Hz write sequence and verification.

This is a useful **design pattern**, not a hardware recipe for our board. The constants, timings, debugfs control and transition sequence are specific to the source's validated model/panel pairs. The runtime path does not wait for a VBlank/TE edge; the fork avoids changing physical mode during gameplay and reapplies the chosen mode around startup/wake instead.

### What its measurements demonstrate

The committed `rg34xxsp-2026-10-15` probe artifact records target `rg34xxsp`, panel `rg34xxsp_v1`, 1,205 LCD interrupts over 10.03 seconds (120.1396 Hz), with the two register readbacks and a temporary NextUI-menu test. This is SP evidence, not evidence for our non-SP `rg34xx_v1`.

The full repeated resource matrix in `RESULTS.md` is for target `rgsp` / panel `rg34xxsp_v1`. It reports native game speed around 59.7 FPS at both display rates and LCD IRQ scanout around 120.08–120.17 Hz. Its controlled software input-response median improves from 41.26 ms at 60 Hz to 32.76 ms at 120 Hz (about 8.5 ms). The test explicitly excludes physical switch debounce, LCD scanout position and pixel response from that latency metric. These results are useful reference evidence, but they do not validate our panel, PLL values, power draw or battery-life impact.

## The RG34XX compatibility mismatch

Our own latest read-only live capture is [`reports/rg34xx-inventory-20261125-163152`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-163152). Its `live-fdt.json` identifies:

- model target: RG34XX non-SP / H700;
- live panel node: `/soc@03000000/lcd0@01c0c000`;
- `lcd_driver_name=rg34xx_v1`;
- geometry: 720×480.

The fork's `PanelRefresh::probe()` maps target `rg34xx` to `fog_fj035fhd05_v1`, so it will reject our live `rg34xx_v1` before any register write. The fork's own `HARDWARE.md` marks this non-SP entry as source-evidence-only, with no connected RG34XX live test.

This is consistent with the pinned BaseOS fleet survey: [`panel-survey.txt`](https://github.com/pvaibhav/BaseOS/blob/63137afeda1c0bc4ff771be6f03012b7f8147aca/experiments/h700-boot-speed/variant-survey/panel-survey.txt) lists RG34XX as `rg34xx_v1` (720×480), and [`panel-contract.json`](https://github.com/pvaibhav/BaseOS/blob/63137afeda1c0bc4ff771be6f03012b7f8147aca/experiments/h700-boot-speed/native/panel-contract.json) records the RG34XX-specific PE6 strap outcomes. Our 16:30 bootargs snapshot also records `lcd_type=old`.

The same fork gate further requires 26 MHz / 820×536. The current live-FDT reparse now establishes our device's fields as DCLK property 26 MHz, HT/VT 820×536, with `lcd_type=old`; same-boot TCON/PLL readback establishes divider 11 and PLL raw `0x88001703`. The 54 MHz / 1679×536 MustardOS RG34XX-H DTS remains a different profile. The 120-Hz candidate `lcd_ht=840` in our policy likewise remains unapproved for `rg34xx_v1`.

## Decision for our H700 port

**Reuse the architecture; do not port the register values.**

1. Keep our versioned C policy: UI preference 60/120 Hz plus independent `game_120hz_enabled`. The GAME toggle defaults off (60 Hz); enabling it requests 120 Hz for measurement. Returning to UI restores the separate UI preference.
2. Add a separate C H700 display HAL beneath that policy. It must identify the live board/panel/timing tuple, expose capability only for a fully qualified profile, apply the complete timing/clock route, read back the relevant state, and report `hardware_applied=1` only after successful verification.
3. Treat write ordering, readback, explicit failure status and rollback as mandatory design requirements. Do not assume the two-register writer covers our `rg34xx_v1` panel.
4. Before any hardware writer exists, rerun the read-only inventory with the enhanced FDT parser and verify the live `lcd_*` timing cells. Then trace the exact `rg34xx_v1` initialization path, PE6/`lcd_type` handling, TCON/DE/CCU clock tree and register ownership.
5. Our UI/GAME physical transition additionally needs a proven safe scanout boundary (VBlank/TE or an explicitly blanked transition) and framebuffer/cache ownership rules. The fork's static startup/wake-only mode application does not establish that runtime transition safety.

Until those conditions are met: retain `hardware_applied=0`, do not write `sunxi_dump`, MMIO, PLL, TCON or boot-package data, and do not mark 120 Hz supported for `rg34xx_v1`.

## Immediate next step

The live FDT reparse and same-boot TCON/PLL readback are now published. The active 60-Hz register pair is `0xf000000b` / `0x88001703`; with `tcon_lcd=288 MHz` this gives a derived 26.1818-MHz DCLK and 59.569 Hz for the live 820×536 totals. This aligns with the IRQ proxy but remains a clock-tree estimate. The pvaibhav 120-Hz candidate pair is `0xf000000a` / `0x88002b03`; it must not be applied because the fork's strict gate requires `fog_fj035fhd05_v1`, while the device is `rg34xx_v1 + lcd_type=old`. Next: recover the exact kernel #19 panel/PLL path, validate any high-mode clock/timing against that profile, then establish a safe VBlank/TE transition boundary. Keep all hardware writes disabled.


### New profile evidence after FDT reparse

The live machine-readable FDT confirms `rg34xx_v1`, RGB666, 720×480, 26 MHz DCLK property, HT/VT 820×536 and `lcd_type=old` bootarg. The BaseOS H700 variant survey maps the standard `rg34xx` target to `rg34xx_v1`; `fog_fj035fhd05_v1` belongs to other target profiles. The reviewed fork's gate is not a match for this non-SP RG34XX. The active TCON divider is now read back as 11, and PLL raw is `0x88001703`; a pinned public sun50iw9 reference formula decodes that raw value to 288 MHz. At 288/11, estimated DCLK is 26.1818 MHz / 59.569 Hz. This PLL decode and parent relationship still require kernel #19 source confirmation; it replaces the previous claim that the divider is unmeasured. No register writes are authorized.


The vendor U-Boot disassembly nuance is documented in [the live capture analysis](pvaibhav-slot-rg34xx-live-capture-2026-11-25.md): the selected DT panel struct is `rg34xx_v1`; its init function happens to carry a `fj035fhd05` symbol string. Do not confuse this function label with the FDT panel identity required by the fork's Linux-side gate.


## Frame-rate instrumentation findings (source review)

The pinned pvaibhav/slot source provides a low-overhead UI-present counter, gated by its latency trace switch:

- `crates/slot/src/latency.rs`: tracing is enabled by nonzero `SLOT_TRACE_LATENCY` or by `SLOT_ROOT/System/trace-latency.on` (with `SLOT_ROOT` defaulting to `/mnt/sdcard`). The setting is cached at first use, so the flag must exist before Slot starts.
- `crates/slot/src/device_app.rs`: while tracing is enabled, every 600 primary `surface.swap()` calls it logs `presents/s`, nominal frontend frame period and missed game periods. On the one-panel RG34XX FBDEV path this gives an application present rate, not physical panel scanout.

The pinned armGBA source `MagnominusGit/armGBA@fad6c4b0c0176a76e10667f4aa071a8feb4425c0` establishes a direct game-frame counter:

- `libretro/libretro.c` defines a non-static global `struct GBA* gba`.
- `src/gba.h` defines `struct Video { int vcount; uint32_t frameCounter; ... }`, embedded as `struct GBA::video`.
- `src/video/video.c` increments `video->frameCounter` at V-blank frame end; `retro_run()` waits for that counter to advance before publishing the next GBA image.

This is source-level evidence for `gba->video.frameCounter` as the committed emulation-frame counter. It is **not yet safe to use against the deployed `armgba_libretro.so`**: its device SHA is recorded but not matched to this source build/release; symbol visibility, pointer mapping and `offsetof(struct GBA, video)` must be checked against the exact ELF before a `/proc/PID/mem` reader is trusted. Current `pvaibhav/slot/tools/refresh/probe.rs` supports gpSP and mGBA only, not armGBA. Keep Slot `presents/s`, armGBA committed frames/s and LCD IRQ/s as separate metrics.


## PLL register arithmetic cross-check (pinned public BSP reference only)

A source-level decode is now possible without guessing the raw PLL field layout. In [`qiaoweibiao/T507_Kernel` commit `98fcc7d3f112e51b0edfb71536da89cb2115106f`, `clk-sun50iw9.c`](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/clk/sunxi/clk-sun50iw9.c), `SUNXI_CLK_FACTORS(pll_video0x4, ...)` defines N at shift 8 / width 8 and D1 at shift 1 / width 1; `calc_rate_video0()` calculates `24 MHz × (N + 1) / (D1 + 1)`. The matching [`sunxi,clk` DTS`](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/arch/arm64/boot/dts/sunxi/sun50iw9p1-clk.dtsi) assigns `tcon_lcd` to `pll_video0x4`; `pll_video0` is a `/4` fixed-factor child.

| Raw PLL word | N field | D1 field | Reference `pll_video0x4` rate | With TCON divider | Estimated DCLK |
|---|---:|---:|---:|---:|---:|
| `0x88001703` (live) | 0x17 = 23 → N+1=24 | 1 → D1+1=2 | 24 MHz × 24 / 2 = **288 MHz** | 11 | **26.1818 MHz** |
| `0x88002b03` (pvaibhav 120-Hz candidate) | 0x2b = 43 → N+1=44 | 1 → D1+1=2 | 24 MHz × 44 / 2 = **528 MHz** | 10 | **52.8 MHz** |

At our live 820×536 totals, the candidate computes to 52.8 MHz / (820×536) = **120.13 Hz**. The baseline computes to 26.1818 MHz / (820×536) = **59.569 Hz**. This arithmetic is coherent with the observed 60-Hz clock summary and IRQ slope, and makes the upstream 120-Hz pair internally coherent. It still does **not** prove the current kernel uses this exact PLL layout, that the live `tcon_lcd` parent is the same at runtime, that the panel revision tolerates 52.8 MHz, or that pvaibhav's write sequence is safe on `rg34xx_v1 + lcd_type=old`. The report's raw PLL remains recorded as raw hardware evidence; this section is an external-source decode only.

Do not copy the high-mode candidate into our HAL or enable a writer. First match the exact Linux 4.9.170 #19 source/build ID, verify `clk-sun50iw9` factor definitions and runtime parent/mux path, then qualify the panel timing and safe transition boundary. The linked T507 source is a related sun50iw9 reference, not proven identical to the shipping binary.
