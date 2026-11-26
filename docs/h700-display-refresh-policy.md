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


### Direct implementation comparison and nominal-rate precision

The original repository's rg34xx branch is a boot-package mod, not a Linux/Slot runtime component. Its 60hz and 120hz folders package the same U-Boot, monitor and DTBO with a different sunxi.fex DTB; its build manifest places those pieces into a signed Allwinner boot package. The two timing profiles both request 54 MHz and differ in lcd_ht (1679 for 60 Hz, 840 for 120 Hz). Its flash scripts write the complete package to raw /dev/mmcblk0 offset 0x1004000 and reboot. The upstream README states stock-firmware testing only. This project must not run those scripts or flash that package on BaseOS.

The README-reported rates (60.006 / 119.947 Hz) are not exactly the nominal arithmetic from the integer DT properties:
- 54 MHz / (1679 × 536) = 60.0037 Hz.
- 54 MHz / (840 × 536) = 119.9360 Hz.

The difference is small (less than 0.011 Hz), but should remain visible in documentation: refresh_millihz retains the upstream README-reported values; get_period_ns derives a nominal period from the integer DCLK and totals. The repo does not include a raw IRQ/clock readback trace to independently resolve the effective pixel clock.

This differs from pvaibhav/slot: that fork retains the 26-MHz / 820×536 live tuple and attempts to write divider/PLL values at app startup/wake. Its 120-Hz candidate is divider 10 + PLL 528 MHz, not the original boot package's 54-MHz / HT=840 profile. Both reference implementations select a global physical mode; neither implements automatic UI-120 / GAME-60 scene switching.


### Kernel #19 divider/PLL implication for the original 54-MHz package

The matched stock kernel #19 panel audit shows the live RGB/HV path (`lcd_if=0`) defaults to divider 6; the divider-11 override is specific to `lcd_dclk_freq=26`. Thus the original repository's 54-MHz FDT profile, if consumed by this Linux panel-init path, would request 54×6=**324 MHz**, an exact supported PLL factor-table rate, and return a nominal 54-MHz DCLK. Its 60/120 timing distinction is then the horizontal total (1679 vs 840). This differs from pvaibhav/slot's runtime candidate of PLL 528 MHz plus forced divider 10 while preserving HT=820.

This is an inference from the byte-matched kernel #19 code and the original DT profile; it is not a live test of that boot package on BaseOS. The original boot-package FDT label and U-Boot-to-Linux handoff still need qualification against `rg34xx_v1 + lcd_type=old`. Do not program 324/528 MHz, change the DT, or flash the package based on this arithmetic alone.

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

The later [FDT reparse report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/pvaibhav-slot-runtime/fdt-reparse) now publishes the live panel timing/interface fields: `rg34xx_v1`, RGB666, 720×480, DCLK property 26 MHz, HT/VT 820×536, H porches 46/2/52, V porches 20/5/31, with `lcd_type=old`. The same-boot [TCON/PLL readback](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/pvaibhav-slot-runtime/tcon-readback) is `0xf000000b` / `0x88001703`. If `tcon_lcd=288 MHz` directly feeds divider 11, derived DCLK is 26.1818 MHz and refresh is 59.569 Hz; this reconciles the 59.155-Hz integer-DTB calculation with the ~59.6-Hz IRQ proxy, but it is not an independent physical pixel-clock measurement. The MustardOS DTS remains an external reference, not a replacement for the live profile.
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

The follow-up [FDT reparse report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/pvaibhav-slot-runtime/fdt-reparse) now includes byte-matched raw DTB and JSON. It confirms `rg34xx_v1` / RGB666 / 720×480, DCLK property 26 MHz, HT/VT 820×536, H porch 46/2/52 and V porch 20/5/31, deriving 59.155 Hz from the DTB. The same bootargs contain `lcd_type=old`. Same-boot TCON/PLL readback subsequently confirmed divider 11 and raw PLL `0x88001703`; the pinned public sun50iw9 formula estimates 288-MHz parent, 26.1818-MHz DCLK and 59.569 Hz, reconciling the ~59.63-Hz IRQ proxy within about 0.1%. Exact kernel #19 PLL layout and runtime parent path still require binary disassembly. The static Knulli RG34XX DTS matches 26 MHz/820×536; MustardOS' 54 MHz/1679×536 DTS is an alternate configuration and not the live profile.

The fork's target `rg34xx` path expects `fog_fj035fhd05_v1`; this device identifies as `rg34xx_v1`. Its exact panel gate therefore rejects the device. The timing tuple alone does not authorize the fork's PLL/divider writes. The `sunxi_dump` files exist and are root-writable, but no writes were performed. Do not bypass the model/panel gate, copy register constants, or enable our HAL writer. Retain `hardware_applied=0`.

The capture also exposed first-version collector defects (ADB multi-argument `shell sh -c` quoting, multiline CRLF, IRQ regex variable clobbering, nested IRQ arrays, missing `/mnt/sdcard/System/slot` process path and an `fps` regex that matched `ref_fps`). Version and regression-check a corrected collector before the next capture.


### Gate-path verification from the running process

The on-device process environment records `BASEOS_TARGET=rg34xx`, `BASEOS_PLATFORM=h700`, package `slot-pvaibhav` and frontend instance `slot-pvaibhav--default`. Its process runs as UID 0 with effective capabilities. The reviewed fork's `DevicePlatform::new()` reads `BASEOS_TARGET`, then uses `PanelRefresh::probe()`; `supports_high_refresh()` is true only if this probe returns a controller. Its exact `rg34xx` panel string is `fog_fj035fhd05_v1`, while the device reports `rg34xx_v1`. That is the deterministic rejection point; permissions are not the blocker. No `sunxi_dump` or MMIO write was made. The reparse report now records SHA-256 and byte sizes for the installed Slot executable and armGBA core, but these hashes have not yet been compared with files extracted from the official H700 release ZIP. Therefore the gate explanation is pinned to the reviewed source revision and consistent with the observed runtime, but byte-identity of the deployed executable is not yet attested.


### Panel driver identity and lcd_type are part of the qualified tuple

The pinned BaseOS [H700 variant survey](https://github.com/pvaibhav/BaseOS/tree/63137afeda1c0bc4ff771be6f03012b7f8147aca/experiments/h700-boot-speed/variant-survey) maps `rg34xx` to panel driver `rg34xx_v1` at 720×480, while `fog_fj035fhd05_v1` is used by other targets/geometries. The live bootargs record `lcd_type=old`; the survey notes that the Linux panel driver uses this token to select a panel init sequence. Therefore panel capability qualification must include at least target, exact `lcd_driver_name`, `lcd_type`, and timing/clock profile. The pvaibhav fork's `rg34xx → fog_fj035fhd05_v1` gate does not cover the observed standard RG34XX profile. Do not replace the panel name in its allowlist or reuse its PLL/divider constants until the `rg34xx_v1 + lcd_type=old` path is independently proven.


### Vendor U-Boot selected panel versus init routine label

The pinned BaseOS RG34XX U-Boot inventory selects DT panel struct `rg34xx_v1` (`selected_by_dt=true`), then calls a shared init function with a symbol string containing `lcd_panel_uboot_fj035fhd05_v1_init`. The function label is not the selected DT driver name. The same BaseOS survey maps RG34XX PE6 high → `lcd_type=old`, low → `boe`; the live bootargs show `old`. Therefore the pvaibhav fork's `rg34xx → fog_fj035fhd05_v1` gate mismatches the standard RG34XX firmware profile; it must not be made to pass by aliasing the name. U-Boot sequencing does not prove Linux DCLK divider or runtime mode-switch safety.


### Clock-tree caution from a related RG34XX-H revision (not our live profile)

A MustardOS community report describes another RG34XX-H where a related `sun50iw9` BSP's fixed RGB/HV TCON divider and minimum `pll_video0x4` frequency produced ~106.6 Hz despite DTB nominal ~59.97 Hz. Its later DTS fix changed that unit's DCLK/HT to 54 MHz/1679; another RG34XX-H user could not reproduce the original mismatch. This is not evidence of our non-SP board's actual clock divider, and the 54/1679 values must not be copied. It reinforces the requirement to derive and verify the exact kernel #19 DCLK/PLL/TCON path for our live 26/820×536 profile. See [the forum report](https://community.muos.dev/t/rg34xx-h-incorrect-display-refresh-rate/1825) and [the DTS correction commit](https://github.com/MustardOS/internal/commit/76e454dfa9758a99f85115f80a29121dabb13a01).

## TCON DCLK divider and PLL: read-only readback completed (2026-11-25)

The new [RG34XX readback report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/pvaibhav-slot-runtime/tcon-readback) records a same-boot, read-only request through `/sys/class/sunxi_dump/dump` on Linux 4.9.170 #19. The sibling `/sys/class/sunxi_dump/write` node was never opened or written.

| Address | Register role in the reviewed pvaibhav implementation | Raw readback |
|---|---|---:|
| `0x06511044` | TCON0 DCLK divider | `0xf000000b` |
| `0x03001040` | video PLL | `0x88001703` |

The TCON word has divider field 11 (bits [6:0]) and enabled nibble `0xf`. The same capture's clock summary reports `tcon_lcd=288 MHz` and `pll_video0x4=288 MHz`; the display driver reports `ref_fps=59`, `fps=60.2`. The PLL raw word is intentionally not decoded: its N/M field interpretation has not yet been matched against the exact running kernel #19 source.

### Cross-check against the pinned pvaibhav/slot source

At pinned commit [`5206e3fa39aa4a9b702cd9048a07dab3b5ab95da`](https://github.com/pvaibhav/slot/tree/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da), `crates/slot-power/src/refresh.rs` defines:

| Source mode | Write order | Register values |
|---|---|---|
| 60-Hz baseline | PLL, then divider | PLL `0x88001703`; divider `0xf000000b` |
| 120-Hz candidate | divider, then PLL | divider `0xf000000a`; PLL `0x88002b03` |

The live readback exactly matches both pvaibhav **60-Hz** values. This strengthens confidence that the named addresses and current baseline words correspond to the RG34XX's active 60-Hz state. It does not prove that the 120-Hz pair is safe for this panel.

The pvaibhav fork's own target gate remains `rg34xx → fog_fj035fhd05_v1`; our live FDT is `rg34xx_v1` and bootargs contain `lcd_type=old`. Do not alias the panel name or bypass the gate. The high-mode pair remains source evidence for a different admitted profile, not an approved RG34XX writer.

### Derived current pixel clock

Using the observed `tcon_lcd=288 MHz` as the clock immediately before the DCLK divider (a clock-tree assumption, not an independent measurement):

- DCLK estimate = 288 MHz / 11 = **26.1818 MHz**.
- Refresh estimate = 26.1818 MHz / (820 × 536) = **59.569 Hz**.
- The measured display IRQ proxy was 59.61–59.65 Hz in UI and GAME. The estimate agrees within about 0.1%.

This explains why the integer live-FDT property `lcd_dclk_freq=26` produces a lower nominal calculation (59.155 Hz) than the runtime IRQ slope. It does not turn the IRQ counter into a photodiode measurement or a frontend/game FPS measurement.

### Independent 120-Hz reference and remaining gate

The pinned [ROCKNIX panel firmware table](https://github.com/ROCKNIX/distribution/blob/c17558d2abadebec634100264523aaddc98b3b96/projects/ROCKNIX/packages/linux-firmware/kernel-firmware/extra-firmware/panels/anbernic,rg34xx-panel.panel) advertises 26/52 MHz modes for an `anbernic,rg34xx-panel` firmware profile, with 813×533 totals. This is independent evidence that a 120-Hz mode exists in an RG34XX-related firmware table, but the timing totals differ from our live 820×536 FDT and this does not prove equivalence to our `rg34xx_v1 + lcd_type=old` panel revision.

The active C policy still carries the external 54-MHz / 840 / 536 and 54-MHz / 1679 / 536 timing candidates. These are not the observed live stock timing (26-MHz property / 820 / 536 plus the read-back divider), and must remain tagged as reference candidates rather than the final board profile. Do not use them to program hardware or infer that the current target supports either mode.

### Next work (updated 2026-11-26)

The previous kernel/source and FPS-measurement gates are complete:

1. **Completed — live timing path:** the Build-ID-matched kernel #19 reverse audit closes the `rg34xx_v1 + lcd_type=old` panel init, divider source, PLL factor table and TCON parent inheritance. Live readback is the 60-Hz pair `0x88001703` / `0xf000000b`; the derived current rate is about 59.57 Hz.
2. **Completed — runtime rate accounting:** separate UI presents, game presents, armGBA committed frames and LCD IRQ have been measured. Results remain about 59.50/s pre-game presents, 59.56/s game presents, 59.72/s armGBA frames and 59.59/s IRQ. IRQ is only a scanout proxy. See the FPS report and maps-only supplement.
3. **Open — exact physical panel qualification (the current blocker):** identify the actual LCD module vendor, complete part number and hardware revision fitted to the tested `rg34xx_v1 + lcd_type=old` unit. Obtain a matching panel datasheet or manufacturer confirmation of supported pixel-clock range, horizontal/vertical timing, blanking, refresh limits and overclock tolerance. The public Anbernic product page specifies a 3.4-inch IPS 720×480 display but does not publish the panel part number or refresh-rate limit. Upstream Slot maintainer discussion describes the panels as slightly below 60 Hz and the 120-Hz change as a driver tweak; the same discussion later says the hack was skipped because of reported stutter and implementation complexity. This is useful warning/context, not a datasheet or direct validation of this exact panel revision.
4. **After qualification only:** derive a profile specifically for the identified panel revision; then design and test a safe HAL transition including VBlank/TE boundary, scanout buffer/cache ownership, clock and TCON sequencing, readback, timeouts and rollback. Until then, use the verified 60-Hz baseline and do not experiment with the candidate 528-MHz/divider-10 or boot-package timings.
5. Keep GAME 120-Hz opt-in default-off and all hardware writers disabled. The current C policy remains a target-selection model only; `hardware_applied=0` is mandatory until all panel and transition gates pass.


## PLL register arithmetic cross-check (pinned public BSP reference only)

A source-level decode is now possible without guessing the raw PLL field layout. In [`qiaoweibiao/T507_Kernel` commit `98fcc7d3f112e51b0edfb71536da89cb2115106f`, `clk-sun50iw9.c`](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/clk/sunxi/clk-sun50iw9.c), `SUNXI_CLK_FACTORS(pll_video0x4, ...)` defines N at shift 8 / width 8 and D1 at shift 1 / width 1; `calc_rate_video0()` calculates `24 MHz × (N + 1) / (D1 + 1)`. The matching [`sunxi,clk` DTS`](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/arch/arm64/boot/dts/sunxi/sun50iw9p1-clk.dtsi) assigns `tcon_lcd` to `pll_video0x4`; `pll_video0` is a `/4` fixed-factor child.

| Raw PLL word | N field | D1 field | Reference `pll_video0x4` rate | With TCON divider | Estimated DCLK |
|---|---:|---:|---:|---:|---:|
| `0x88001703` (live) | 0x17 = 23 → N+1=24 | 1 → D1+1=2 | 24 MHz × 24 / 2 = **288 MHz** | 11 | **26.1818 MHz** |
| `0x88002b03` (pvaibhav 120-Hz candidate) | 0x2b = 43 → N+1=44 | 1 → D1+1=2 | 24 MHz × 44 / 2 = **528 MHz** | 10 | **52.8 MHz** |

At our live 820×536 totals, the candidate computes to 52.8 MHz / (820×536) = **120.13 Hz**. The baseline computes to 26.1818 MHz / (820×536) = **59.569 Hz**. This arithmetic is coherent with the observed 60-Hz clock summary and IRQ slope, and makes the upstream 120-Hz pair internally coherent. It still does **not** prove the current kernel uses this exact PLL layout, that the live `tcon_lcd` parent is the same at runtime, that the panel revision tolerates 52.8 MHz, or that pvaibhav's write sequence is safe on `rg34xx_v1 + lcd_type=old`. The report's raw PLL remains recorded as raw hardware evidence; this section is an external-source decode only.

Do not copy the high-mode candidate into our HAL or enable a writer. First match the exact Linux 4.9.170 #19 source/build ID, verify `clk-sun50iw9` factor definitions and runtime parent/mux path, then qualify the panel timing and safe transition boundary. The linked T507 source is a related sun50iw9 reference, not proven identical to the shipping binary.


## 11. Panel specification and upstream 120-Hz hack context (2026-11-26)

The current public hardware specification is insufficient to approve an overclock:

- Anbernic's [official RG34XX specification](https://win.anbernic.com/product/517.html) lists a 3.4-inch IPS full-view OCA display at 720×480, but does not identify the LCD module/part number, hardware revision, native refresh rate or maximum pixel clock.
- In [Slot issue #147, “120 Hz and schedutil governor”](https://github.com/BrandonKowalski/slot/issues/147), the Slot maintainer explains that the supported panels are slightly below 60 Hz and that the tweak runs the display driver at 120 Hz. In the same discussion, after stutter reports and consideration of implementation complexity, the maintainer says the 120-Hz hack will be skipped. This is maintainer/community context, not a panel datasheet and not direct proof about the exact RG34XX unit audited here.
- Our live identity remains `rg34xx_v1 + lcd_type=old`. That is not the `fog_fj035fhd05_v1` profile required by the reviewed pvaibhav/slot writer. A separately published [RG34XX-SP device-tree/bring-up report](https://github.com/fotlim/android_device_anbernic_rg34xxsp_h700_lineage) explicitly identifies its own panel as `FOG FJ035FHD05` with compatible `anbernic,rg34xx-sp-panel`; it is the SP model and is not evidence that the non-SP RG34XX uses the same module. The 120-Hz register pair and the external boot-package FDT therefore remain unqualified references, not usable settings for this device.

**Engineering interpretation:** do not describe 120 Hz as a native capability of the RG34XX panel. Treat it as an attempted display-driver overclock until the exact LCD module and its timing limits are identified. A successful UI/core frame counter near 120 would still not prove panel electrical acceptance or stable scanout.

**Next evidence to seek:** a clearly readable rear-panel/FPC/module marking from the exact test unit (with the device powered off and safely opened only by the owner), a matching BOM/datasheet from Anbernic or the panel supplier, or written vendor confirmation tied to that board revision. If no traceable module identity/specification can be obtained, retain 60 Hz as the only qualified operating mode. Do not infer permission from unrelated RG34XX-SP, RG34XX-H, RG35XX-SP or other panel variants.

Sources:
- [Anbernic RG34XX product specification](https://win.anbernic.com/product/517.html)
- [BrandonKowalski/slot issue #147](https://github.com/BrandonKowalski/slot/issues/147)
