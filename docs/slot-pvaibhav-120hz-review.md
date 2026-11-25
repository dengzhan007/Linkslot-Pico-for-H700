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

The same fork gate further requires 26 MHz / 820×536. The current uploaded live-FDT JSON predates our new panel-timing parser fields, so it does **not** establish our device's live `lcd_dclk_freq/lcd_ht/lcd_vt`. A separate MustardOS RG34XX-H DTS reference has `rg34xx_v1`, 54 MHz and 1679×536 totals; that is corroborating source data, not a replacement for a new read-only capture from our device. The 120-Hz candidate `lcd_ht=840` in our policy likewise remains unapproved for `rg34xx_v1`.

## Decision for our H700 port

**Reuse the architecture; do not port the register values.**

1. Keep our versioned C policy: UI preference 60/120 Hz plus independent `game_120hz_enabled`. The GAME toggle defaults off (60 Hz); enabling it requests 120 Hz for measurement. Returning to UI restores the separate UI preference.
2. Add a separate C H700 display HAL beneath that policy. It must identify the live board/panel/timing tuple, expose capability only for a fully qualified profile, apply the complete timing/clock route, read back the relevant state, and report `hardware_applied=1` only after successful verification.
3. Treat write ordering, readback, explicit failure status and rollback as mandatory design requirements. Do not assume the two-register writer covers our `rg34xx_v1` panel.
4. Before any hardware writer exists, rerun the read-only inventory with the enhanced FDT parser and verify the live `lcd_*` timing cells. Then trace the exact `rg34xx_v1` initialization path, PE6/`lcd_type` handling, TCON/DE/CCU clock tree and register ownership.
5. Our UI/GAME physical transition additionally needs a proven safe scanout boundary (VBlank/TE or an explicitly blanked transition) and framebuffer/cache ownership rules. The fork's static startup/wake-only mode application does not establish that runtime transition safety.

Until those conditions are met: retain `hardware_applied=0`, do not write `sunxi_dump`, MMIO, PLL, TCON or boot-package data, and do not mark 120 Hz supported for `rg34xx_v1`.

## Immediate next step

A read-only UI/GAME capture and a complete live-FDT reparse are now published; see [the live capture analysis](pvaibhav-slot-rg34xx-live-capture-2026-11-25.md) and [the reparse artifacts](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/pvaibhav-slot-runtime/fdt-reparse). The live panel is `rg34xx_v1`, RGB666, 720×480, DCLK 26 MHz, HT/VT 820×536, and bootargs include `lcd_type=old`. The numeric timing cells match the pvaibhav gate, but exact panel identity does not: the fork requires `fog_fj035fhd05_v1`. IRQ remains near 59.6 Hz; DTB-derived nominal refresh is 59.155 Hz, and final DCLK/divider remains unmeasured. The `rg34xx_v1` panel and `lcd_type`-dependent init sequence need their own source evidence; do not alias this panel to `fog_fj035fhd05_v1`, weaken the gate or try its register writes.


### New profile evidence after FDT reparse

The live machine-readable FDT now confirms `rg34xx_v1`, RGB666, 720×480, 26 MHz DCLK property, HT/VT 820×536 and `lcd_type=old` bootarg. The BaseOS H700 variant survey independently maps the standard `rg34xx` target to `rg34xx_v1`; `fog_fj035fhd05_v1` belongs to other target profiles. The reviewed fork's gate is therefore not a valid match for this standard non-SP RG34XX. Capability work must qualify the tuple including `lcd_type`, not only the driver string and geometry. Actual final DCLK/divider remains unmeasured; IRQ rate (~59.63 Hz) differs from DTB-derived nominal 59.155 Hz. No register writes are authorized.
