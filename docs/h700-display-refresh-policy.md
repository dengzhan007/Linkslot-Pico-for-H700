# RG34XX H700 display refresh policy and timing profiles

## 1. Decision and scope

The RG34XX policy is:

- UI scene default: **120 Hz class** (source package reports 119.947 Hz).
- Game scene: **60 Hz by default** (source package reports 60.006 Hz); an independent opt-in GAME 120 Hz switch can request the 120-Hz candidate for controlled testing. The switch defaults off and does not assert that 120 Hz is stable in gameplay.
- UI preference: user-selectable **60 Hz / 120 Hz**. Changing this preference while a game is active does not change the game target; leaving the game restores the selected UI preference.
- The policy is independent of CPU performance policy. No Linkslot Rust/Frontend/App interface is added in this phase.

The implementation in `platform/h700/display/` is a pure C policy/profile module. It has no TCON, DE, CCU, PLL, panel GPIO, backlight, MMIO, boot-package, or SD-card write path. `hardware_applied` remains zero and APIs return `H700_DISPLAY_REFRESH_POLICY_ONLY`: this means a target timing profile was selected, **not** that the panel is already running at that rate.

## 2. RG34XX timing evidence

Reference: [TheGammaSqueeze/rgxx_boot_package_mods, branch `rg34xx`](https://github.com/TheGammaSqueeze/rgxx_boot_package_mods/tree/rg34xx), which publishes separate 119.947 Hz and 60.006 Hz boot packages and states they were tested on stock firmware.

Relevant files:
- [README and timing notes](https://github.com/TheGammaSqueeze/rgxx_boot_package_mods/blob/rg34xx/README.md)
- [120 Hz `sunxi.fex`](https://github.com/TheGammaSqueeze/rgxx_boot_package_mods/blob/rg34xx/120hz/sunxi.fex), blob SHA `25d06f4b27abf23b78b5790f4e455f0e5c8351bd`
- [60 Hz `sunxi.fex`](https://github.com/TheGammaSqueeze/rgxx_boot_package_mods/blob/rg34xx/60hz/sunxi.fex), blob SHA `22c90d37d9dd672fa23d2a3189e72c4b035e7569`

Both DTBs are 137,728 bytes and have the same node/property structure. A direct FDT parse found exactly one property difference:

`/soc@03000000/lcd0@01c0c000: lcd_ht`

| Parameter | 119.947 Hz profile | 60.006 Hz profile |
|---|---:|---:|
| `lcd_dclk_freq` | 54 MHz | 54 MHz |
| `lcd_ht` | **840** | **1679** |
| `lcd_vt` | 536 | 536 |
| `lcd_hbp` | 46 | 46 |
| `lcd_hspw` | 2 | 2 |
| `lcd_vbp` | 20 | 20 |
| `lcd_vspw` | 5 | 5 |
| active resolution | 720×480 | 720×480 |
| README-reported refresh | 119.947 Hz | 60.006 Hz |

The external 120 Hz FDT identifies `lcd_driver_name = "fog_fj035fhd05_v1"`. Our RG34XX live-FDT inventory currently identifies the panel driver as `rg34xx_v1`. Although the model and 720×480 geometry agree, these driver names are not identical. This profile is therefore a **RG34XX reference timing candidate**, not yet a proof that the exact live panel revision in this project accepts the same timing unchanged.

The external repository also documents flashing its signed `boot_package.fex` at disk offset `0x1004000`. This project does **not** copy that flashing process, boot package, or raw disk-writing scripts. Only the timing values and the FDT comparison are used as reference evidence.

### Panel-driver provenance cross-check

The initial name mismatch is now backed by independent RG34XX firmware and
board-profile evidence; it must not be treated as a cosmetic alias:

- The BaseOS [panel survey](https://github.com/pvaibhav/BaseOS/blob/63137afeda1c0bc4ff771be6f03012b7f8147aca/experiments/h700-boot-speed/variant-survey/panel-survey.txt)
  records RG34XX as `lcd_driver_name=rg34xx_v1`, 720×480, with kernel panel
  driver `rg34xx_v1`. The same survey lists `fog_fj035fhd05_v1` for other
  models such as RG28XX, RG35XX H and RG40XX, not RG34XX.
- The corresponding [BaseOS compatibility manifest](https://github.com/pvaibhav/BaseOS/blob/63137afeda1c0bc4ff771be6f03012b7f8147aca/experiments/h700-boot-speed/evidence/compatibility.json)
  also identifies RG34XX as `rg34xx_v1` at 720×480.
- [MustardOS RG34XX-H DTS](https://github.com/MustardOS/internal/blob/aef38ed9a870419a0fe4f351dac1bac58a84ba5e/device/rg34xx-h/package/sunxi.dts)
  uses `rg34xx_v1`, 54 MHz, `lcd_ht=1679`, `lcd_vt=536`, matching the
  candidate 60.006 Hz timing profile. This is useful independent support for
  the 54 MHz / 1679 / 536 baseline on an RG34XX panel-driver identity.
- [KNULLI's RG34XX boot-package DTS](https://github.com/knulli-cfw/knulli-linux/blob/57afa662009641d181f57241bcc480119b131530/package/boot/uboot-h700/rg34xx/boot_package/rg34xx.dts)
  also uses `rg34xx_v1`, but has an older 26 MHz / `lcd_ht=820` /
  `lcd_vt=536` configuration. The external mod README itself calls this the
  stock timing and says its kernel may override the 26 MHz clock.

The external 60/120 Hz package sets the U-Boot-package FDT panel name to
`fog_fj035fhd05_v1` in both variants. Its README says to overwrite the
boot-package region at disk offset `0x1004000` and reports stock-firmware
testing. We do not assume that its U-Boot display FDT, the kernel's live FDT,
and the panel driver's initialization sequence are interchangeable. It is
not safe to flash or copy that package unchanged into this project.

Current evidence supports the **60 Hz 54 MHz / 1679 / 536 profile** on the
`rg34xx_v1` identity through the MustardOS DTS. The 120 Hz `lcd_ht=840`
candidate still needs confirmation that the actual `rg34xx_v1` panel init
sequence and panel revisions accept it. This is now a stricter gate than
simply comparing active resolution or matching the `fog_*` profile.

## 3. C policy API

Files:
- `platform/h700/display/h700_display_refresh_policy.h`
- `platform/h700/display/h700_display_refresh_policy.c`
- `platform/h700/display/tests/h700_display_refresh_policy_test.c`

The versioned C API provides:
- `h700_display_refresh_policy_init_v1()`: initializes UI preference to 120 Hz, sets `game_120hz_enabled=0`, and starts in the UI scene.
- `h700_display_refresh_policy_set_ui_preference_v1(60|120)`: changes only the UI preference; it does not override the GAME switch.
- `h700_display_refresh_policy_set_game_120hz_enabled_v1(0|1)`: independent GAME 120 Hz switch. Default 0 requests 60 Hz; 1 requests the 120-Hz candidate while GAME is active.
- `h700_display_refresh_policy_set_scene_v1(UI|GAME)`: UI resolves to the stored UI preference; GAME resolves to 120 Hz only when `game_120hz_enabled=1`, otherwise it remains at 60 Hz.
- `h700_display_refresh_policy_get_state_v1()`: returns UI preference, game toggle state, scene, requested profile, generation and `hardware_applied`.
- `h700_display_refresh_get_rg34xx_timing_v1(60|120)`: returns the fixed RG34XX timing candidate above.

The C policy switch and host regression are present, but no Linkslot Frontend setting row, event hook, or persistent configuration format is wired yet. The future settings UI should expose this as an independent “120 Hz in games” on/off option and persist it separately from the UI 60/120 Hz preference.

## 4. Slot upstream reference audit

Reviewed [slot-cfw/slot](https://github.com/slot-cfw/slot) at `main` commit
[8fbc6084ece5bc9b4b18a0fb6ef423df21606f49](https://github.com/slot-cfw/slot/commit/8fbc6084ece5bc9b4b18a0fb6ef423df21606f49).

The current source does **not** contain an RG34XX-specific 120 Hz mode write or
a UI/game refresh-mode toggle. The relevant behavior is split by the BaseOS
platform:

- `crates/slot/tests/baseos.rs` uses `BASEOS_PLATFORM=h700` and
  `BASEOS_TARGET=rg34xx` in its RG34XX release fixture.
- `crates/slot/src/device_app.rs` selects `KmsSurface` only when
  `BASEOS_PLATFORM == "rk3568"`; the H700 value selects `FbdevSurface`.
- `crates/slot-gfx/src/fbdev.rs` probes `/sys/class/graphics/fb0/{mode,modes}`,
  visible framebuffer size and virtual size. These paths provide dimensions,
  not a refresh-rate timing. It calls `eglSwapInterval(display, 1)`.
- The common `Surface::refresh_period()` default is `None`. With the H700
  FBDEV surface, `device_app` therefore creates `Pacer::new()`, whose default
  interval is 16,760 µs (about 59.67 presentations/s). This is not a 120 Hz
  frontend frame loop and it does not program the panel mode.
- The separate RK3568 KMS path enumerates DRM connector modes, chooses a mode
  carrying `DRM_MODE_TYPE_PREFERRED` (otherwise the first available), passes
  the selected full mode to `drmModeSetCrtc`, and computes the frame interval
  from `clock × htotal × vtotal`. `device_app` then uses `Pacer::fixed(period)`.
  It follows the mode exported by the display driver; it does not hard-code
  `120` or switch to 60 Hz when a game starts.

This is useful as an architectural reference: **the display driver/HAL owns
mode enumeration and application; the renderer consumes the actual timing to
pace frames**. The Slot Linux FBDEV/EGL/DRM implementation is not portable to
our GammaOS LK bare-metal target. Our requested UI-120/game-60 policy remains
a separate target behavior and must be implemented by a future RG34XX C
display HAL plus a timing-aware scheduler; it is not evidence that Slot itself
performs this scene switch.

The H700 timing helper added in this project follows the same arithmetic as
Slot's KMS `mode_period`: `period_ns = htotal × vtotal × 1000 / clock_MHz`.
For the candidate profiles it yields 8,337,777 ns at 119.947 Hz and
16,665,629 ns at 60.006 Hz. These are calculated periods, not observed
VBlank intervals.

## 5. Runtime switching is not implemented

Changing `lcd_ht` is a panel/TCON timing change, not simply changing an emulator's FPS cap. A future display HAL must establish, at minimum:

1. Exact panel vendor/revision and the relationship between the live `rg34xx_v1` driver and the external `fog_fj035fhd05_v1` profile.
2. Pixel-clock source/rate, relevant CCU/TCON/DE clocks and divider sequence.
3. VBlank/TE synchronization, safe display disable/re-enable ordering, scanout buffer lifetime, stride/pixel format and cache maintenance.
4. Frame pacing and missed/duplicated frame behaviour for 120 Hz UI and 60 Hz gameplay.
5. Readback/timeout/failure handling and a safe return to the known-good 60 Hz/stock mode.

No such hardware transition function exists yet. Do not add register writes, PLL changes, panel power cycling, or automatic boot-package modifications until these gates are supported by RG34XX-specific evidence and tests. The default 120 Hz value in the policy is currently a **requested mode only**.

## 6. Validation

The display policy test script is `scripts/test-h700-display-refresh-policy.sh`. The H700 AArch32/AArch64 build script compiles the policy for AArch64, AArch32 ARM-state and AArch32 Thumb-2; `platform/h700/rules.mk` links the policy object into the target graph. GitHub Actions invokes the host policy test from `.github/workflows/h700-arch-check.yml`.

Host tests verify independent UI/game settings, default-OFF GAME 120 Hz behavior, opt-in/opt-out during gameplay, unchanged-value generation behavior, UI/GAME scene restoration, unchanged v1 structure sizes (48-byte policy and 32-byte state), timing constants, period calculations (8,337,777 ns for the 119.947 Hz candidate and 16,665,629 ns for the 60.006 Hz candidate), and invalid arguments. The helper was added in code commit [`237ee131d5926ee79b049a1d3915b4da7f5c8d0b`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/237ee131d5926ee79b049a1d3915b4da7f5c8d0b); its host test initially missed a local variable declaration, corrected in [`d20550ab2cd6c4e7dbaa758f09e1b5993988878b`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/d20550ab2cd6c4e7dbaa758f09e1b5993988878b). [H700 AArch32/AArch64 compile check #45572079415](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45572079415) passed the host-regression job and the freestanding AArch64, AArch32 ARM-state and AArch32 Thumb-2 compile job. This is not a full LK target link, hardware timing application, or physical 120/60 Hz display test.
## 7. On-device timing readback collection

The read-only inventory script now writes `display-runtime.txt` with:

- `/sys/class/disp/disp/attr/sys` when exported by the vendor H700 display driver (often contains `ref_fps`, current `fps`, and display-engine state);
- `/sys/class/graphics/fb0/{name,mode,modes,virtual_size,stride,pan,bits_per_pixel}` and `/proc/fb` when available;
- display-related lines from `/sys/kernel/debug/clk/clk_summary`, preserving clock-tree indentation.

The capture has no sysfs/debugfs output redirection, mode set, MMIO, ioctl, or boot-package write. Missing paths are reported as unavailable. Run the normal Windows inventory wrapper or the read-only collector and inspect the generated `display-runtime.txt`.

Interpret this evidence in layers: display-engine `ref_fps`/`fps` is the kernel driver's reported scanout timing, framebuffer `mode` may only report geometry/nominal rate, and clock-summary is the provider's current clock readback. None is a photodiode/high-speed-camera measurement of the physical panel. Frontend render FPS remains a separate quantity.

### 7.1 RG34XX live readback (2026-11-25 16:30 +08:00)

The latest read-only capture is [`reports/rg34xx-inventory-20261125-163152`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261125-163152). It identifies the panel node as `rg34xx_v1` (720×480) and records:

- Vendor display-engine status: `de_rate=696000000 Hz`, `ref_fps=59`, current `fps=60.2`, active 720×480 layer.
- Framebuffer: `U:720x480p-59`, virtual size 720×960, stride 2880 bytes, 32 bpp.
- Display clocks: `pll_video2/tcon_tv=297 MHz`, `pll_video0x4/tcon_lcd=288 MHz`, `display_top=200 MHz`.

This is a software readback of the current approximately-60-Hz scanout path. The 288 MHz `tcon_lcd` clock-summary value is **not** the panel pixel clock and must not be substituted for `lcd_dclk_freq`; the divider and clock-parent path matter. The capture does not show a 120-Hz scanout and does not validate the external `lcd_ht=840` candidate. The 120-Hz timing remains policy-only and unapproved for the live `rg34xx_v1` identity.

The captured `live-fdt.json` predates the panel-timing parser enhancement and therefore reports panel identity/geometry but not the `lcd_*` timing cells. The FDT inspector now emits the available `lcd_dclk_freq`, `lcd_ht`, `lcd_hbp`, `lcd_hspw`, `lcd_vt`, `lcd_vbp`, `lcd_vspw`, and interface properties as structured fields. A subsequent read-only inventory capture is required to establish what the live FDT actually supplies; the MustardOS DTS values remain an independent reference, not a substitute for that readback.
## 8. Allwinner sun50iw9 vendor display-driver reference

To separate a reporting sysfs node from a real mode-setting interface, the
Allwinner Linux 4.9 `sun50iw9` display framework was inspected in the pinned
[orangepi-xunlong/linux-orangepi commit `0cd0547ea405b84b5b60fbc92978ac1bc2b68055`](https://github.com/orangepi-xunlong/linux-orangepi/commit/0cd0547ea405b84b5b60fbc92978ac1bc2b68055).
This is a public vendor-BSP reference for the same SoC family, not a claim
that this exact source revision is the binary kernel currently installed on
the test handheld.

- [`dev_disp.c`](https://github.com/orangepi-xunlong/linux-orangepi/blob/0cd0547ea405b84b5b60fbc92978ac1bc2b68055/drivers/video/fbdev/sunxi/disp2/disp/dev_disp.c#L86-L224): `disp_sys_show()` prints display-engine clock rate, driver-reported `ref_fps`, LCD `fps`, resolution and vsync counters. `disp_sys_store()` simply returns `count` and performs no operation. The neighboring `disp` attribute writes the `g_disp` screen-selection variable; it is not a refresh-rate control.
- [`disp_lcd.c`](https://github.com/orangepi-xunlong/linux-orangepi/blob/0cd0547ea405b84b5b60fbc92978ac1bc2b68055/drivers/video/fbdev/sunxi/disp2/disp/de/disp_lcd.c): derives frame rate from `lcd_dclk_freq × 1,000,000 × (interlace + 1) / (lcd_ht × lcd_vt)`. Its LCD clock configuration calculates PLL/LCD/DSI rates from panel parameters and calls the clock framework to set parents and rates.
- [`de_lcd.c`](https://github.com/orangepi-xunlong/linux-orangepi/blob/0cd0547ea405b84b5b60fbc92978ac1bc2b68055/drivers/video/fbdev/sunxi/disp2/disp/de/lowlevel_v33x/tcon/de_lcd.c) programs the TCON horizontal/vertical panel timing fields. The companion [`disp_al_tcon.c`](https://github.com/orangepi-xunlong/linux-orangepi/blob/0cd0547ea405b84b5b60fbc92978ac1bc2b68055/drivers/video/fbdev/sunxi/disp2/disp/de/lowlevel_v33x/disp_al_tcon.c) calculates the clock dividers used by the LCD output path.

Consequences for our target:

1. Do not try to switch refresh rate by writing `ref_fps`, `fps`, or `disp` under `attr`; `sys` is read-only in effect and `disp` only selects a screen index.
2. The actual operation spans panel timing, TCON, DE/display routing and clock parents/dividers. A dynamic UI/game mode switch must either use a proven driver API with explicit synchronization or own the whole LK display path; changing the emulator's frame cap alone only changes content update cadence.
3. Treat the vendor Linux sources as a sequencing/reference map only. Before implementing LK MMIO/clock writes, confirm the exact H700 register map, the live `rg34xx_v1` panel initialization sequence, current scanout ownership, and a safe VBlank transition. Do not copy Linux `clk_set_rate()` calls or guessed register addresses into LK.



## 9. Reference implementation review: pvaibhav/slot

The [`pvaibhav/slot` fork at `5206e3fa39aa4a9b702cd9048a07dab3b5ab95da`](https://github.com/pvaibhav/slot/tree/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da) provides a substantially more complete Linux-side H700 refresh implementation than our current policy-only code: exact DT/model gating, a two-register divider/PLL sequence, register readback, a best-effort 60-Hz rollback, startup/wake reapplication and LCD IRQ measurement. See the [detailed compatibility review](slot-pvaibhav-120hz-review.md).

Do **not** copy its register addresses/values into this project. Its `rg34xx` allowlist expects `fog_fj035fhd05_v1`, 26 MHz and 820×536, while our live RG34XX report identifies `rg34xx_v1`; the fork's non-SP RG34XX support is not backed by a connected-device test. Its global 120-Hz startup/wake policy also does not implement our independent UI preference plus opt-in GAME 120-Hz switch (default off).

Reuse only the engineering pattern: identify the exact live panel tuple, refuse unsupported combinations, separate policy from hardware ownership, verify every applied state, expose failure clearly and attempt a known-safe rollback. Our own C HAL must be based on fresh live `rg34xx_v1` timing readback and target-specific source/register evidence. No writer is approved.


## 10. pvaibhav/slot on-device observation: RG34XX non-SP (2026-11-25)

The [same-device UI/GAME read-only capture](pvaibhav-slot-rg34xx-live-capture-2026-11-25.md) confirms the live display stayed at approximately 60 Hz in both phases:

| IRQ proxy | UI | GAME |
|---|---:|---:|
| display IRQ 292 | 59.648 Hz | 59.632 Hz |
| display IRQ 293 | 59.614 Hz | 59.632 Hz |
| driver `fps` values | 60.2 / 59.8 | 60.2 / 59.8 |

The Slot process was found at `/mnt/sdcard/System/slot` (PID 2842); the GAME capture maps `armgba_libretro.so`. The config contains `high_refresh=1`, but no hardware refresh transition was observed. Neither UI-present FPS nor armGBA committed game FPS was measured. LCD IRQ is a scanout/vblank proxy, not either FPS metric.

The report README records live FDT timing 26 MHz / HT 820 / VT 536, but the uploaded `live-fdt.json` omits the `interface` and `timing` subobjects and the raw DTB remains local. Re-run the current FDT parser and verify the generated JSON before using these values as a reproducible machine-readable record.

The fork's target `rg34xx` path expects `fog_fj035fhd05_v1`; this device identifies as `rg34xx_v1`. Its exact panel gate therefore rejects the device. The timing tuple alone does not authorize the fork's PLL/divider writes. The `sunxi_dump` files exist and are root-writable, but no writes were performed. Do not bypass the model/panel gate, copy register constants, or enable our HAL writer. Retain `hardware_applied=0`.

The capture also exposed first-version collector defects (ADB multi-argument `shell sh -c` quoting, multiline CRLF, IRQ regex variable clobbering, nested IRQ arrays, missing `/mnt/sdcard/System/slot` process path and an `fps` regex that matched `ref_fps`). Version and regression-check a corrected collector before the next capture.
