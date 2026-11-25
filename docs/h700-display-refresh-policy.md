# RG34XX H700 display refresh policy and timing profiles

## 1. Decision and scope

The RG34XX policy is:

- UI scene default: **120 Hz class** (source package reports 119.947 Hz).
- Game scene: **fixed 60 Hz class** (source package reports 60.006 Hz).
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

## 3. C policy API

Files:
- `platform/h700/display/h700_display_refresh_policy.h`
- `platform/h700/display/h700_display_refresh_policy.c`
- `platform/h700/display/tests/h700_display_refresh_policy_test.c`

The versioned C API provides:
- `h700_display_refresh_policy_init_v1()`: initializes UI preference to 120 Hz and scene to UI.
- `h700_display_refresh_policy_set_ui_preference_v1(60|120)`: future settings toggle.
- `h700_display_refresh_policy_set_scene_v1(UI|GAME)`: game always resolves to 60 Hz; UI resolves to the stored UI preference.
- `h700_display_refresh_policy_get_state_v1()`: returns current preference, scene, requested profile, generation and `hardware_applied`.
- `h700_display_refresh_get_rg34xx_timing_v1(60|120)`: returns the fixed RG34XX timing candidate above.

No Frontend/Linkslot interface, event hook, persistent configuration format, or user-visible settings row is created yet. The policy can be unit-tested and linked into the H700 build graph without binding it to an emulator or game frontend.

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

Host tests verify the state rules and timing constants. Verified on code commit [`440a5cb393a5c0a4d7aa17502f1edf94ec24b3f6`](https://github.com/dengzhan007/Linkslot-Pico-for-H700/commit/440a5cb393a5c0a4d7aa17502f1edf94ec24b3f6): [H700 AArch32/AArch64 compile check #45569849692](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45569849692) passed both the host-regression job (including this policy test) and the freestanding AArch64, AArch32 ARM-state and AArch32 Thumb-2 compile job. This is not a full LK target link, hardware timing application, or physical 120/60 Hz display test.
