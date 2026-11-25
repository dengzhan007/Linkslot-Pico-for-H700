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

The first runtime captures identify:
- SoC/model: `sun50iw9`; compatible includes `allwinner,h616` and `arm,sun50iw9p1`.
- Panel node: `/soc@03000000/lcd0@01c0c000`; driver identity `rg34xx_v1`; 720×480 active geometry.
- Both first-pass runtime captures have `high_refresh=1`, but remained near 59.6 Hz by IRQ counters.

### Re-parsed live FDT (19:09 +08:00, new boot cycle)

The follow-up [FDT reparse artifacts](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/pvaibhav-slot-runtime/fdt-reparse) now include the raw DTB and complete machine-readable JSON. The JSON's internal `sha256` exactly matches the raw DTB:

| Artifact | Size | SHA-256 |
|---|---:|---|
| `live-fdt.dtb` | 143,936 B | `65801950f6d2d3c7139111d35c6cc0dd64cc249f8ba64ffb5b6f1c7fd9c39173` |
| `live-fdt.json` | 38,537 B (LF) | `1ddbb3073fddc83e3633b372bf42168a0652182e46726abcb6cf60b1121e560b` |

The re-parsed `panels[0]` fields are:

| Property | Live FDT value |
|---|---:|
| `lcd_driver_name` | `rg34xx_v1` |
| Interface | `rgb666` (`lcd_if=0`, `lcd_hv_if=0`) |
| Active size | 720×480 |
| Pixel clock property | 26,000,000 Hz (26 MHz) |
| H total / back porch / sync / front porch | 820 / 46 / 2 / 52 |
| V total / back porch / sync / front porch | 536 / 20 / 5 / 31 |
| Derived nominal refresh | 59.155 Hz |
| Physical dimensions | 150×94 mm |
| PWM | channel 0, 50 kHz, polarity 1, max 255 |

The active kernel bootargs also contain `lcd_type=old`. This is a boot-time strap-derived argument, not a direct measurement of panel vendor or optical characteristics. The panel driver can select its initialization sequence using this token, so it must remain part of the hardware profile evidence.

The computed 59.155 Hz is derived from the DTB values: `26,000,000 / (820 × 536)`. It is **not** a measurement of the final pixel clock. The runtime IRQ counters measured 59.61–59.65 Hz, and the vendor driver's instantaneous `fps` alternated between 59.8 and 60.2. The approximately 0.5-Hz difference between the IRQ slope and DTB-derived nominal value remains unexplained. The exported clock summary exposes `tcon_lcd=288 MHz` and `pll_video0x4=288 MHz`, but not a verified final LCD DCLK/divider; do not infer a register value from the parent clock alone.

The live timing is independently consistent with the [Knulli RG34XX boot-package DTS](https://github.com/knulli-cfw/knulli-linux/blob/57afa662009641d181f57241bcc480119b131530/package/boot/uboot-h700/rg34xx/boot_package/rg34xx.dts), which also declares `rg34xx_v1`, 26 MHz and 820×536. A different MustardOS RG34XX-H DTS uses the same driver identity with 54 MHz and 1679×536; that is an alternate source configuration, not the live device's current values.

The README's structural comparison says the 19:09 DTB differs from the 18:44 FDT only in three `/chosen` boot metadata properties; the 573-node tree including the complete LCD node is otherwise identical. The new capture is a later boot cycle, not a panel replacement.

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

1. Compare the installed `System/slot` and `SYSTEM/armgba_libretro.so` hashes against the exact extracted files from the published `v1.16.0-pvaibhav.2` H700 release ZIP; the device hashes are recorded below, but ZIP-to-installed-file identity is not yet established.
2. Trace the exact `rg34xx_v1` panel initialization and `lcd_type=old` strap path, then identify the final LCD DCLK divider and clock parent. Resolve why IRQ counters report about 59.63 Hz while the static DTB calculation gives 59.155 Hz.
3. Continue tracing TCON/DE/CCU clock path, scanout ownership and safe VBlank/blanking boundary from matching sources. Keep hardware writes disabled.
4. Develop separate UI-present and armGBA committed-frame measurements; do not use LCD IRQ as either FPS metric.
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

The report now captures hashes and sizes for the installed Slot executable and armGBA core, but not the build ID/version or an extracted-release comparison. The source-level gate explanation remains pinned to reviewed commit `5206e3fa39aa4a9b702cd9048a07dab3b5ab95da`; byte identity of the deployed binary is still unverified.


## Installed runtime binary identities

The 19:12 read-only capture recorded:

- `/mnt/sdcard/System/slot`: 18,718,728 bytes, SHA-256 `8f0fd40da4ac161e5fbb9a56a99c254acf8b2f31806e2be6bdaa929b5ee28505`.
- `/mnt/sdcard/SYSTEM/armgba_libretro.so`: 2,021,992 bytes, SHA-256 `4ebae098cc4848fb459fc18299c39acbdbbbeb732db5c3f66d1ff3f6e142e889`.

These hashes are new to the project reports. They do not by themselves identify a version or prove equality with the reviewed source. Compare them with the extracted executable/core in the official [`v1.16.0-pvaibhav.2` H700 release](https://github.com/pvaibhav/slot/releases/tag/v1.16.0-pvaibhav.2); the tag points to the reviewed source line, but the installed file's version/build ID has not yet been attested.
