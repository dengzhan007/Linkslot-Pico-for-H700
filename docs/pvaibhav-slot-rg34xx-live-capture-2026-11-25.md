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

Thus the numerical timing tuple now confirmed by the live parser matches the fork's numeric gate, but the panel identity does not. The probe returns unsupported before the writer is exposed. In the fork's app startup path, the refresh setter is called only when `supports_high_refresh()` is true. The configured `high_refresh=1` therefore cannot force the unsupported panel through the gate.

This is also an inconsistency between the fork's target mapping and the pinned BaseOS panel survey: that survey maps target `rg34xx` to `rg34xx_v1` (720×480), while `fog_fj035fhd05_v1` is assigned to other board targets and panel geometries. The live bootargs additionally record `lcd_type=old`; the survey explains that the Linux panel driver uses this token to choose its init sequence. The token is a selected boot argument, not a direct live PE6 electrical read or a panel-vendor identification.

The safest interpretation is that the reviewed fork's non-SP `rg34xx` allowlist does not cover the standard live RG34XX profile tested here. It may be an upstream coverage gap, but this does **not** establish that simply replacing its string with `rg34xx_v1` would make its 120-Hz register sequence safe. The precise panel init variant, `lcd_type` handling and physical panel characteristics must be traced independently.

Do not change the panel allowlist, copy its register values, or write `sunxi_dump` to make this device pass.

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


## Panel-identity cross-check from the pinned BaseOS survey

The [BaseOS H700 variant survey](https://github.com/pvaibhav/BaseOS/tree/63137afeda1c0bc4ff771be6f03012b7f8147aca/experiments/h700-boot-speed/variant-survey) records the target-to-panel mapping:

- `rg34xx` → `rg34xx_v1`, 720×480, with kernel panel driver `rg34xx_v1`.
- `fog_fj035fhd05_v1` is assigned to other targets (including RG28XX, RG35XXH/Plus/Pro/SP and RG40XX variants); the survey's geometry list is 640×480 for most of those targets and 720×720 for RG CubeXX.
- The same survey says the per-target panel driver consumes `lcd_type=` to choose an initialization sequence. This device's live bootargs show `lcd_type=old`.

This cross-check strengthens the conclusion that the reviewed pvaibhav `rg34xx → fog_fj035fhd05_v1` gate does not describe the standard RG34XX non-SP panel profile. It does not authorize substituting `rg34xx_v1` into that gate: the 120-Hz clock sequence must first be validated against the exact `rg34xx_v1 + lcd_type=old` initialization and actual panel revision.


## Vendor RG34XX U-Boot panel-selection evidence

The pinned BaseOS firmware audit gives a more precise explanation for the live `lcd_type=old` token and the fork's panel-name mismatch:

- Its compatibility contract maps target `rg34xx` to FDT driver `rg34xx_v1` / 720×480 and records the vendor PE6 result as high → `old`, low → `boe`.
- Its panel survey independently lists `rg34xx` → `rg34xx_v1` and the kernel panel driver `rg34xx_v1`; `fog_fj035fhd05_v1` is assigned to other target profiles.
- The RG34XX U-Boot disassembly inventory selects the `rg34xx_v1` panel struct at `0xa38d0` (`selected_by_dt=true`). That struct's open flow calls init function `0x262e4`, whose embedded symbol/string is `lcd_panel_uboot_fj035fhd05_v1_init`. The function name is a shared U-Boot init routine label; it does **not** change the selected DT driver name from `rg34xx_v1` to `fog_fj035fhd05_v1`.
- The disassembly shows a PE-data bit-6 read and branch inside that U-Boot init path. The survey maps PE6 high to `lcd_type=old` and low to `boe`. Our live FDT bootargs contain `lcd_type=old`, so this is consistent with the vendor boot chain taking the PE6-high / `old` branch. It remains an indirect boot-argument observation, not a fresh electrical read of PE6 at Linux runtime.

This makes the pvaibhav fork's `BASEOS_TARGET=rg34xx → fog_fj035fhd05_v1` gate a clear mismatch with the surveyed standard RG34XX firmware contract. A reasonable hypothesis is that the fork's RG34XX allowlist is incomplete or intended for another hardware revision. Do **not** "fix" the mismatch by changing the driver string or borrowing the FJ035 PLL/divider values: the U-Boot init routine name is not proof that the Linux driver identity or 120-Hz register sequence is interchangeable.

This U-Boot evidence covers panel selection/reset/init sequencing only. It does not reveal the Linux final pixel-clock divider, the runtime TCON/DE clock route, or a safe 60↔120-Hz transition boundary.

Source references:

- [BaseOS compatibility contract](https://github.com/pvaibhav/BaseOS/blob/63137afeda1c0bc4ff771be6f03012b7f8147aca/experiments/h700-boot-speed/evidence/compatibility.json)
- [BaseOS target/panel survey](https://github.com/pvaibhav/BaseOS/blob/63137afeda1c0bc4ff771be6f03012b7f8147aca/experiments/h700-boot-speed/variant-survey/panel-survey.txt)
- [RG34XX U-Boot panel-init disassembly](https://github.com/pvaibhav/BaseOS/blob/63137afeda1c0bc4ff771be6f03012b7f8147aca/experiments/h700-boot-speed/uboot-trace-notes/delays/inventory-rg34xx.json)
- [PE6/lcd_type branch summary](https://github.com/pvaibhav/BaseOS/blob/63137afeda1c0bc4ff771be6f03012b7f8147aca/experiments/h700-boot-speed/uboot-trace-notes/delays/summary.md)


## Official release identity for the next binary comparison

GitHub release metadata confirms the reviewed source commit is exactly the commit targeted by tag `v1.16.0-pvaibhav.2`:

- Tag `v1.16.0-pvaibhav.2` resolves to `5206e3fa39aa4a9b702cd9048a07dab3b5ab95da` (annotated tag message: checked against that commit).
- Official H700 bundle: [`slot-v1.16.0-pvaibhav.2-h700.zip`](https://github.com/pvaibhav/slot/releases/download/v1.16.0-pvaibhav.2/slot-v1.16.0-pvaibhav.2-h700.zip), 17,556,056 bytes, release asset SHA-256 `1f9bdaa1110eb4f6006cc9da6a8bb34bca694ea53555c3138dcf656f0dfd1fca`.

The next local verification is to download that ZIP, verify its archive hash, extract the `slot` and `armgba_libretro.so` files, and compare their SHA-256 with the device captures in [`device-sha256.txt`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/pvaibhav-slot-runtime/fdt-reparse/device-sha256.txt). The archive was not downloaded in this environment, so no equality result is claimed yet.


## Related RG34XX-H clock-mismatch report (reference only; not this device)

A November 2, 2026 MustardOS community report describes a different RG34XX-H running a related `sun50iw9` BSP where FDT DCLK 27 MHz / HT 840 / VT 536 nominally computes to 59.97 Hz, but the observed display was about 106.5 Hz. The author attributed the mismatch to a fixed TCON divider of 6 for RGB/HV combined with the BSP's `pll_video0x4` clock table starting at 288 MHz: 288/6=48 MHz, and 48 MHz/(840×536)=106.61 Hz. A later fix changed that RG34XX-H's DCLK to 54 MHz and HT to 1679; the pinned [MustardOS commit `76e454dfa9758a99f85115f80a29121dabb13a01`](https://github.com/MustardOS/internal/commit/76e454dfa9758a99f85115f80a29121dabb13a01) changed the DTS values and screen refresh declaration from 59.968 to 60.004. A second RG34XX-H user reported they could not reproduce the first unit's mismatch, illustrating revision/build differences.

This is **not** a measurement of our RG34XX non-SP and does not prove that our kernel uses the same divider table. It is evidence that Allwinner's requested `lcd_dclk_freq`, parent PLL selection and final TCON DCLK can diverge, and that even the `rg34xx_v1` driver string is not enough to transfer timing values between hardware revisions. For our live 26 MHz / 820×536 profile, the IRQ rate (~59.63 Hz) is close to the DTB-derived 59.155 Hz but still differs by ~0.48 Hz. That divider has now been read back as 11. The remaining hardware-analysis step is to recover the **exact running 4.9.170 #19 display clock path** from the byte-matched stock kernel and validate PLL field layout, parent mux and `rg34xx_v1 + lcd_type=old` initialization against the already captured values. Do not repeat the same register readback unless a new boot/mode creates a distinct measurement need, and do not import the MustardOS 54 MHz / 1679 timing.

References: [MustardOS RG34XX-H clock-mismatch discussion](https://community.muos.dev/t/rg34xx-h-incorrect-display-refresh-rate/1825) and [the DTS correction commit](https://github.com/MustardOS/internal/commit/76e454dfa9758a99f85115f80a29121dabb13a01).

## External sun50iw9 TCON source reference (readback now complete)

The pinned Allwinner sun50iw9 BSP reference at [orangepi-xunlong/linux-orangepi@0cd0547ea405b84b5b60fbc92978ac1bc2b68055](https://github.com/orangepi-xunlong/linux-orangepi/commit/0cd0547ea405b84b5b60fbc92978ac1bc2b68055) gives a concrete way to distinguish the DTB request from the actual panel DCLK. This is a related vendor BSP reference, **not yet proven to be the exact source revision behind the running 4.9.170 #19 binary**.

- The sun50iw9 build selects lowlevel_v33x display code (see the pinned [DE Makefile](https://github.com/orangepi-xunlong/linux-orangepi/blob/0cd0547ea405b84b5b60fbc92978ac1bc2b68055/drivers/video/fbdev/sunxi/disp2/disp/de/Makefile)).
- Its [disp_al_tcon.c](https://github.com/orangepi-xunlong/linux-orangepi/blob/0cd0547ea405b84b5b60fbc92978ac1bc2b68055/drivers/video/fbdev/sunxi/disp2/disp/de/lowlevel_v33x/disp_al_tcon.c) default table maps LCD_IF_HV to tcon_div=6 and lcd_div=1; disp_al_lcd_cfg() passes this divider to tcon0_set_dclk_div().
- The [TCON register definition](https://github.com/orangepi-xunlong/linux-orangepi/blob/0cd0547ea405b84b5b60fbc92978ac1bc2b68055/drivers/video/fbdev/sunxi/disp2/disp/de/lowlevel_v33x/tcon/de_lcd_type.h) places tcon0_dclk at offset 0x44 after tcon0_ctl at 0x40. Its tcon0_dclk_div field is bits [6:0], and tcon0_dclk_en is bits [31:28]. The [getter/setter](https://github.com/orangepi-xunlong/linux-orangepi/blob/0cd0547ea405b84b5b60fbc92978ac1bc2b68055/drivers/video/fbdev/sunxi/disp2/disp/de/lowlevel_v33x/tcon/de_lcd.c) reads/writes that field.
- Its [disp_lcd.c](https://github.com/orangepi-xunlong/linux-orangepi/blob/0cd0547ea405b84b5b60fbc92978ac1bc2b68055/drivers/video/fbdev/sunxi/disp2/disp/de/disp_lcd.c) requests pll_rate = requested_DCLK × tcon_div × lcd_div, reads back the PLL and LCD child rates, and derives the actual DCLK as lcd_rate_set / tcon_div. Its frame-period calculation also uses the read-back clock, not just the DTB property.
- The pvaibhav fork names the TCON DCLK register DIVIDER=0x06511044 and the video PLL register PLL=0x03001040. Its 60-Hz divider word is 0xf000000b (divider field 11); its 120-Hz word is 0xf000000a (divider field 10). We have **not** written either address on this device.

The current same-boot readback has settled the divider question: `0x06511044=0xf000000b` (divider 11), not the related generic BSP's divider 6. The PLL raw word at `0x03001040` is `0x88001703`. Under the public sun50iw9 clock formula below, that raw word decodes to 288 MHz; if that parent directly feeds the divider, the DCLK estimate is 26.1818 MHz and the scanout estimate is 59.569 Hz at 820×536.

### External PLL formula cross-check (not exact-kernel proof)

The pinned public [`clk-sun50iw9.c`](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/clk/sunxi/clk-sun50iw9.c) defines N at bits [15:8], D1 at bit 1, and `pll_video0x4 = 24 MHz × (N+1)/(D1+1)`. Its pinned [sun50iw9p1 clock DTS](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/arch/arm64/boot/dts/sunxi/sun50iw9p1-clk.dtsi) assigns `tcon_lcd` to that parent. Therefore the reference formula maps live `0x88001703` to 288 MHz and candidate `0x88002b03` to 528 MHz. Pairing with dividers 11 and 10 yields estimated DCLKs 26.1818 and 52.8 MHz; the latter computes to 120.13 Hz at 820×536. This is only an external source cross-check. It does not establish that exact kernel #19 uses identical fields or parent mux, nor prove the `rg34xx_v1 + lcd_type=old` panel supports the candidate.

No additional dump request is required for the current boot: both registers are already read. Do not write `/sys/class/sunxi_dump/write` or attempt the candidate high-mode pair.


No further register-read command is required for the current boot: both addresses have already been queried through `/sys/class/sunxi_dump/dump`. The query was read-only; `/sys/class/sunxi_dump/write` was never opened. The remaining work is source/build-ID validation, not repeating the MMIO capture.


## TCON / PLL readback follow-up (2026-11-25 19:59 +08:00)

The subsequent [read-only TCON/PLL capture](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/pvaibhav-slot-runtime/tcon-readback) closes the previous open item about the active divider:

- TCON0 DCLK register `0x06511044`: `0xf000000b` (divider field 11).
- Video PLL register `0x03001040`: `0x88001703` (preserved as raw hardware evidence; its fields are interpreted below using a pinned related BSP reference, not yet the exact kernel #19 source).
- Concurrent `clk_summary`: `tcon_lcd=288 MHz` / `pll_video0x4=288 MHz`.
- Capture method: read request through `/sys/class/sunxi_dump/dump`; `/sys/class/sunxi_dump/write` was never opened; no register, panel, or refresh-mode writes were performed.

The raw pair matches the pvaibhav/slot pinned implementation's 60-Hz register pair exactly. With the 820×536 live FDT totals, 288 MHz / 11 estimates a 26.1818-MHz DCLK and 59.569 Hz scanout, which agrees with the 59.61–59.65-Hz IRQ proxy to roughly 0.1%. This explains the previous 59.155-Hz DTB-vs-IRQ difference as consistent with a rounded 26-MHz FDT property and the observed divider/clock summary; the derived DCLK is still not an independent physical measurement.

The pvaibhav high-mode pair is divider `0xf000000a` then PLL `0x88002b03`. Its `rg34xx` panel gate requires `fog_fj035fhd05_v1`, whereas this unit reports `rg34xx_v1` and `lcd_type=old`. The 120-Hz sequence is therefore still **not approved** for this board. The exact 4.9.170 #19 panel/PLL source, complete clock-parent chain, UI present FPS, and armGBA committed-frame FPS remain outstanding. Keep hardware writes disabled.


## External sun50iw9 PLL formula cross-check (reference only)

The pinned public `qiaoweibiao/T507_Kernel` sun50iw9 clock driver defines PLL-video0x4 N at bits [15:8], D1 at bit 1, and computes `24 MHz × (N+1)/(D1+1)`. Its pinned DTS assigns `tcon_lcd` to `pll_video0x4`. Under that related BSP's layout, the live raw PLL word `0x88001703` decodes to 288 MHz, while pvaibhav's candidate `0x88002b03` decodes to 528 MHz. Paired with DCLK dividers 11 and 10 respectively, those yield 26.1818-MHz and 52.8-MHz pixel-clock estimates; the latter is about 120.13 Hz at 820×536. This makes the candidate internally consistent but **does not identify the exact running kernel #19 PLL layout or prove panel support**. Exact source/build-ID correspondence and runtime parent mux still need verification. No register write is authorized.


## 9. 2026-11-26 follow-up capture and counter-read plan

The new [ADB report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/slot-refresh-audit) includes a second, same-device UI/GAME check (GBA core actively loaded during GAME). IRQ293 measured 59.667 Hz in UI and 59.583 Hz in GAME; the display driver showed `fps=60.2` / `ref_fps=59` / `de_rate=696 MHz` in both. PLL `0x03001040` remained `0x88001703`. The TCON divider read in this specific run returned I/O error, so this run does not provide a new divider readback. Slot PID 2301 persisted across both captures; GAME added the `armgba_libretro.so` mapping but did not restart the display stack. `high_refresh=1` remained set without changing the actual scanout path.

The new report's first version incorrectly treated a second FDT timing node as a prerequisite for runtime refresh switching and called 576 MHz the pvaibhav high-mode PLL value. The correction in [reports/slot-refresh-audit/SUMMARY.md §10](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/slot-refresh-audit/SUMMARY.md) records the exact pinned-source sequence: divider `0xf000000a`, PLL `0x88002b03` (528 MHz under the related sun50iw9 reference formula), which computes to about 120.131 Hz at 820×536. This does not alter the live-device result: the observed PLL stayed at its 288-MHz baseline value and IRQ stayed near 60 Hz.

### UI present rate

At pinned Slot source `5206e3fa39aa4a9b702cd9048a07dab3b5ab95da`, `device_app.rs` emits `presents/s` every 600 surface swaps when `slot::latency::tracing()` is enabled. The switch is nonzero `SLOT_TRACE_LATENCY` or `$SLOT_ROOT/System/trace-latency.on`; its `OnceLock` caches the first lookup, so a marker must exist before Slot starts. Before enabling it, inspect `/proc/2301/fd/2` to establish where stderr goes and compare the installed `System/slot` hash with the official H700 release.

### armGBA committed frames

Pinned armGBA source `MagnominusGit/armGBA@fad6c4b0c0176a76e10667f4aa071a8feb4425c0` increments `gba->video.frameCounter` at VBlank frame end. `struct GBA` has a source-level fixed-size assertion of 788712 bytes and uses eight-byte host-pointer slots, so a C layout probe can calculate the stable offset from the exact pinned source. However, `libretro/link.T` exports only `retro_*` and marks the rest `local: *`: the global `gba` is not a dependable dynamic export. Do not use `dlsym` or `nm -D` as the pointer source. First compare the installed `armgba_libretro.so` SHA with the exact release/build, inspect whether its full ELF symbol table retains `gba`, and verify the object load bias and pointer mapping. Only then may a bounded read-only `/proc/PID/mem` sampler be considered; no blind scanning.

Keep three outputs distinct: Slot surface presents/s, armGBA VBlank-committed frames/s, and LCD IRQ/s. The last is only scanout/vblank proxy. No hardware register writes are authorized.
