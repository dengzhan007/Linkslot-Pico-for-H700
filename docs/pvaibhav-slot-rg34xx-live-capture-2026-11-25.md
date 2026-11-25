# pvaibhav/slot RG34XX non-SP live capture (2026-11-25)

## Executive result

The two read-only captures show the same RG34XX non-SP device scanning at approximately **59.6 Hz** in both the pvaibhav/slot UI and an active GBA game. No 120-Hz mode, UI-to-game refresh transition, or game-to-UI transition was observed.

The device has `slot.state: high_refresh=1`, but that preference is not proof that the hardware mode was applied. The observed IRQ rates, display-driver status and clock tree remained at the same approximately-60-Hz state in both phases.

## Captures and provenance

The complete report and files are on the `reports` branch:

- [Report README and comparison](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/pvaibhav-slot-runtime/README.md)
- [UI capture, 2026-11-25 18:44:06 +08](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/pvaibhav-slot-runtime/20261125-184406-ui)
- [GAME capture, 2026-11-25 18:45:08 +08](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/pvaibhav-slot-runtime/20261125-184508-game)
- [Packaged archive](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/pvaibhav-slot-runtime/pvaibhav-slot-runtime-20261125.zip)
- Reports-branch commit: [`3bedb6212639f1913187996d49b4fddc15671434`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/3bedb6212639f1913187996d49b4fddc15671434)

Same device, same OS and same Slot process PID (`2842`) were used. UI captured 27 samples across 29.624 s; GAME captured 23 samples across 29.749 s. The user manually entered a game between captures.

The command environment required a local execution copy with fixes. The report lists five defects in the first collector version: ADB `shell sh -c` argument handling (causing remote stdin hangs), CRLF in multiline shell payloads, PowerShell `$Matches` clobbering in IRQ parsing, nested arrays from the IRQ parser call site, and incomplete process/FPS matching. The production script was not changed by that run. The checked-out project branch did not yet contain `tools/collect-pvaibhav-slot-runtime.ps1`; this tool must be added and regression-checked before it is reused.

## Live display observations

| Measurement | UI | GAME |
|---|---:|---:|
| Samples / elapsed | 27 / 29.624 s | 23 / 29.749 s |
| IRQ 292 (`dispaly`) | 59.648 Hz | 59.632 Hz |
| IRQ 293 (`dispaly`) | 59.614 Hz | 59.632 Hz |
| Driver-reported `fps` | 60.2 (23 samples), 59.8 (4) | 60.2 (20), 59.8 (3) |
| Driver `ref_fps` | 59 | 59 |
| DE rate | 696 MHz | 696 MHz |
| Framebuffer mode | 720x480 | 720x480 |

Display clock readback was identical in both phases: `tcon_lcd=288 MHz`, `pll_video0x4=288 MHz`, `pll_video0=72 MHz`, `display_top=200 MHz`, and `tcon_tv/pll_video2=297 MHz`. These clock-summary entries are not themselves the panel pixel clock.

The live FDT identifies:
- SoC/model: `sun50iw9`; compatible includes `allwinner,h616` and `arm,sun50iw9p1`.
- Panel node: `/soc@03000000/lcd0@01c0c000`.
- Panel driver: `rg34xx_v1`.
- Active geometry: 720x480.
- The capture README records the live FDT timing cells as DCLK 26 MHz, HT 820, VT 536, HBP 46, HSPW 2, VBP 20, VSPW 5, interface `lcd_if=0`.

**Artifact caveat:** the published `live-fdt.json` files do not contain the `interface` or `timing` subobjects. They include panel driver and geometry only. The raw `live-fdt.dtb` was retained locally and not uploaded. Therefore the 26 MHz / 820x536 values are recorded in the report README as extracted values, but cannot be independently re-parsed from the published binary in this commit. Re-pull the live FDT once or provide the retained DTB, run the current `tools/inspect_h700_fdt.py`, and publish a JSON whose `panels[0].timing` values are populated. Do not silently treat the report README as a substitute for the parser artifact.

The two IRQ counters are scanout/vblank proxies. Their approximately-59.6-Hz slopes agree across the full UI and GAME intervals, and the display status, clock tree and FDT are unchanged. This establishes that no refresh change was observed during these captures; it does not measure panel optical response or UI present rate.

## Slot process and configuration

The runtime process scan correctly found PID `2842` with:
- `comm=ld-linux-aarch6` (truncated process comm);
- command line `/lib/ld-linux-aarch64.so.1 /mnt/sdcard/System/slot`;
- GAME maps additionally contain `/mnt/sdcard/SYSTEM/armgba_libretro.so`.

The executable path is a BaseOS multi-frontend mapped runtime path; it is not named `slot-pvaibhav` and is not under `.system/slot-pvaibhav`. Future process discovery must therefore match the actual `/System/slot` executable path in addition to frontend labels.

The effective slot-state candidates under `/mnt/SDCARD/Config/slot.state`, `/mnt/SDCARD/config/slot.state` and `/mnt/sdcard/Config/slot.state` all report `high_refresh=1`. The GAME capture confirms `cart_platform=gba` and `armgba_libretro.so` in process maps. It does **not** include a committed-frame counter, so no game FPS is available. The UI capture similarly contains no UI present counter; neither capture establishes UI FPS.

The `/sys/class/sunxi_dump/write` and `dump` nodes exist and were listed as root-writable in the capture. No writes were issued.

## Why pvaibhav/slot remained at 60 Hz

The pinned fork implementation reviewed for this project (`5206e3fa39aa4a9b702cd9048a07dab3b5ab95da`) has an exact panel gate in `crates/slot-power/src/refresh.rs`:

- For `BASEOS_TARGET=rg34xx`, it requires panel bytes `fog_fj035fhd05_v1\\0`.
- It also requires 720x480, HT 820, VT 536 and DCLK 26 MHz.
- The live device reports `rg34xx_v1`, not `fog_fj035fhd05_v1`.

Thus the numerical timing tuple recorded by the report matches the fork's expected timing values, but the panel identity does not. The probe returns unsupported before the writer is exposed. In the fork's app startup path, the refresh setter is called only when `supports_high_refresh()` is true. The configured `high_refresh=1` therefore cannot force the unsupported panel through the gate.

This is a useful confirmation of the fork's fail-closed behavior, not evidence that its 120-Hz register sequence is compatible with this panel. Do not change the panel allowlist, copy its register values, or write `sunxi_dump` to make this device pass.

## FPS measurement boundary

The captures establish only:
1. kernel display IRQ rate (approximately 59.6 Hz);
2. vendor display status (`fps` and `ref_fps`);
3. the Slot process and loaded core.

They do not measure:
- UI frame submissions / presents;
- armGBA committed game frames per second;
- audio underruns or gameplay frame pacing;
- input-to-photon latency.

A follow-up game probe must be armGBA-specific; the fork's existing sample probe handles gpSP and mGBA accessors but does not provide an armGBA committed-frame counter. UI pacing likewise needs a frontend present/render counter or a controlled instrumentation point. Do not use the LCD IRQ slope as either FPS metric.

## Next steps

1. Re-parse the retained live DTB or re-pull it and publish the missing `panels[0].timing` fields; keep the raw DTB local if the report policy requires it, but retain its SHA-256.
2. Add and regression-check a corrected PowerShell collector. It must normalize multiline payloads, quote the `adb shell sh -c` payload as one remote command, avoid PowerShell `$Matches` clobbering, return a flat IRQ row array, distinguish `fps` from `ref_fps`, and discover `/mnt/sdcard/System/slot`.
3. Trace the exact `rg34xx_v1` panel initialization, TCON/DE/CCU clock path, actual pixel-clock divider, scanout ownership and safe VBlank/blanking boundary from matching sources. Keep hardware writes disabled.
4. Develop separate UI-present and armGBA committed-frame measurements; then use 60-Hz baseline and only source-approved refresh candidates for controlled comparison.
5. Preserve the C policy contract: UI preference and GAME 120-Hz opt-in are separate; GAME 120 Hz remains default-off. `hardware_applied=0` until an RG34XX-specific HAL proves the applied mode by readback.

**Current conclusion:** the device is observed at approximately 60 Hz in both UI and gameplay. No 120-Hz support or UI/GAME FPS result is claimed.


## Runtime identity and gate confirmation

Both captured `proc-2842/selected-environ.txt` files show the same multi-frontend environment:

- `BASEOS_TARGET=rg34xx`, `BASEOS_PLATFORM=h700`;
- `BASEOS_PACKAGE_ID=slot-pvaibhav`, `BASEOS_FRONTEND_ID=slot-pvaibhav--default`;
- `SLOT_ROOT=/mnt/sdcard`, `BASEOS_LAUNCHER=/mnt/sdcard/.baseos/compat/slot-pvaibhav/launch.sh`;
- `BASEOS_FRONTEND_PROGRAM_ROOT=/mnt/sdcard/Frontends/slot-pvaibhav`.

The running process status is `Uid: 0 0 0 0` with effective capabilities `0000003fffffffff`. Thus root permissions do not explain the absence of a mode change. The `sunxi_dump` nodes are root-owned and root-writable; they were not written.

The reviewed pvaibhav source confirms the exact decision chain: `DevicePlatform::new()` reads `BASEOS_TARGET`; `DevicePlatform::probe()` calls `PanelRefresh::probe()`; `supports_high_refresh()` is `self.refresh.is_some()`; and `app.rs::set_power()` calls `set_high_refresh()` only when that capability is true. The fork's `rg34xx` panel gate requires the exact bytes `fog_fj035fhd05_v1\\0`, while this live panel reports `rg34xx_v1`. Therefore the reviewed source path rejects this board before any hardware write and skips the startup setter even though the stored preference is 1.

The report did not capture a SHA-256 or version string for the installed Slot executable/core, so the deployed binary cannot be proven byte-for-byte identical to reviewed commit `5206e3fa39aa4a9b702cd9048a07dab3b5ab95da`. The environment and runtime observations match the expected target, but the source-level explanation is pinned to that reviewed source revision rather than an independently verified installed binary.
