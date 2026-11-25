# RG34XX 120 Hz boot-package modification review

**Reviewed source:** [TheGammaSqueeze/rgxx_boot_package_mods, branch `rg34xx`](https://github.com/TheGammaSqueeze/rgxx_boot_package_mods/tree/rg34xx), HEAD `64300b3b6137f7676b63b2e969e58db2870db48b` (2024-12-21). This is a source/artifact audit only; no package was flashed to the RG34XX test device.

## Executive result

This repository implements a **boot-time, fixed display mode** by replacing the signed Allwinner `boot_package.fex` at raw storage offset `0x1004000`, then rebooting. It does not provide a userspace runtime refresh switch. The 60-Hz and 120-Hz packages contain distinct FDT timing values, so the selected mode persists through UI and gameplay until another package is flashed.

The FDT panel identity is `fog_fj035fhd05_v1`; the test device's live FDT is `rg34xx_v1` with bootarg `lcd_type=old`. Therefore this package is **not a qualified profile for the current non-SP RG34XX**, even though the repository directory/README says RG34XX. Do not flash either package onto the current device.

## 1. Binary-level FDT comparison

Both `120hz/sunxi.fex` and `60hz/sunxi.fex` are 137,728-byte FDT/DTB blobs (FDT magic `d00dfeed`). They were decoded and compared by property path, not judged from README labels alone.

| FDT field at `/soc@03000000/lcd0@01c0c000` | 120-Hz package | 60-Hz package |
|---|---:|---:|
| `lcd_driver_name` | `fog_fj035fhd05_v1` | same |
| `lcd_x` / `lcd_y` | 720 / 480 | same |
| `lcd_if` / `lcd_hv_if` | 0 / 0 | same |
| `lcd_dclk_freq` | 54 MHz | 54 MHz |
| `lcd_ht` | **840** | **1679** |
| `lcd_vt` | 536 | 536 |
| H back porch / sync | 46 / 2 | same |
| V back porch / sync | 20 / 5 | same |

The full parsed property diff between the two FDTs contains exactly one property: `lcd_ht` (840 versus 1679). Every other FDT property is identical.

| FDT artifact | Size | Git blob SHA |
|---|---:|---|
| `120hz/sunxi.fex` | 137,728 B | `25d06f4b27abf23b78b5790f4e455f0e5c8351bd` |
| `60hz/sunxi.fex` | 137,728 B | `22c90d37d9dd672fa23d2a3189e72c4b035e7569` |

The repository README also lists stock timing as 26 MHz / 820 / 536. The patch instead uses 54 MHz for both its fixed modes. The README labels the resulting modes 119.947 Hz and 60.006 Hz; direct arithmetic from the stored FDT integers gives:

- 120-Hz FDT: `54,000,000 / (840 × 536) = 119.936034 Hz`.
- 60-Hz FDT: `54,000,000 / (1679 × 536) = 60.003734 Hz`.
- README's stock values: `26,000,000 / (820 × 536) = 59.155442 Hz` (this is the DT-property calculation, not the final clock readback).

The small difference between README's reported values and the nominal FDT calculation may reflect measured clock/IRQ behavior or rounding; the repository does not include a raw measurement log in this branch, so the stated 119.947/60.006 values remain author-reported rather than independently reproducible from a published trace.

## 2. What is actually replaced

[`120hz/boot_package.cfg`](https://github.com/TheGammaSqueeze/rgxx_boot_package_mods/blob/rg34xx/120hz/boot_package.cfg) lists four package items:

1. `u-boot.fex`
2. `monitor.fex`
3. `p1.dtbo`
4. `sunxi.fex` (the FDT/DTB)

Comparison of the extracted components in the 120-Hz and 60-Hz directories:

| Component | SHA-1/Git blob SHA | Same between modes? |
|---|---|---|
| `u-boot.fex` | `4bdedff96537da04def4b3356adef7a8a4d71585` | yes |
| `monitor.fex` | `40a0476deaf0590f062b0c9fda14b97ee12af1b2` | yes |
| `p1.dtbo` | `aebd4a61c90674f9cb8c6d68b2b74bd150fc3a63` | yes |
| `sunxi.fex` | different (above) | no |

The stock package included for extraction/restore is also identical in both folders (`abf722055f0baa873569ad071eed96e5d90ab6b4`). Both built `boot_package.fex` files are 1,310,720 bytes, but have different hashes. Thus the meaningful mode delta is the LCD `lcd_ht` FDT property; U-Boot, monitor and the overlay are not mode-specific in this branch.

[`build.sh`](https://github.com/TheGammaSqueeze/rgxx_boot_package_mods/blob/rg34xx/120hz/build.sh) invokes `dragonsecboot -pack boot_package.cfg` to package the four components.

## 3. Flash and activation model

The README directs the user to overwrite the disk image at offset `0x1004000` with the contents of `boot_package.fex`. The [120-Hz flash script](https://github.com/TheGammaSqueeze/rgxx_boot_package_mods/blob/rg34xx/flashscript/RG34XX_120Hz.sh) performs these operations:

- hardcodes `DEVICE=/dev/mmcblk0` and `OFFSET=0x1004000`;
- checks that the 14-byte prefix at the target offset contains the `sunxi-package` magic;
- uses `dd ... conv=notrunc` to overwrite the package;
- reboots immediately after `dd` returns success.

There is a separate backup script and restore script. The README says to back up first and explicitly warns that the package scripts were tested on Stock firmware; it does not claim broad CFW compatibility. The flash script itself does not verify the written bytes/hash after `dd`, does not verify the package's full signature/content before writing, and does not enforce that the backup script has been run.

This is a raw boot-package update, not a safe in-place display API. Do not execute it against the current BaseOS device or infer that `/dev/mmcblk0` and the boot-package offset are interchangeable across firmware layouts.

## 4. Difference from pvaibhav/slot

Reviewed Slot source is pinned at [`pvaibhav/slot@5206e3fa39aa4a9b702cd9048a07dab3b5ab95da`](https://github.com/pvaibhav/slot/tree/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da), especially [`crates/slot-power/src/refresh.rs`](https://github.com/pvaibhav/slot/blob/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da/crates/slot-power/src/refresh.rs).

| Dimension | `rgxx_boot_package_mods` | pvaibhav/slot |
|---|---|---|
| Control layer | bootloader package / FDT | Linux userspace at Slot startup and wake |
| Mode selected by | flashing 60-Hz or 120-Hz package, then reboot | `high_refresh` preference, applied through runtime register writes |
| 120-Hz configuration | DCLK 54 MHz, HT 840, VT 536 | TCON divider `0xf000000a`, PLL `0x88002b03` (reference decode 528 MHz / 10) |
| 60-Hz configuration | DCLK 54 MHz, HT 1679, VT 536 | PLL `0x88001703`, divider `0xf000000b` |
| Runtime switching | no; separate package + reboot | startup/wake application; one global mode, not UI/GAME scene switching |
| Panel gate | FDT names `fog_fj035fhd05_v1` | target `rg34xx` requires `fog_fj035fhd05_v1`, 720×480, HT/VT 820×536, DCLK 26 MHz |
| Main failure risk | wrong boot package / raw flash layout | runtime writer applied to a panel/clock profile not independently qualified |

These are **not the same timing implementation** despite both targeting a named RG34XX variant:

- The boot-package method asks the kernel's ordinary panel init to use 54-MHz DCLK and changes only HT between its two fixed modes. Under the matched kernel #19 audit's `lcd_if=0` divider logic, a 54-MHz request does not take the special 26-MHz divider-11 branch; the table default is divider 6. If that same kernel path is applicable, it requests `54 × 6 = 324 MHz`, producing nominal `54 MHz / (840 × 536) = 119.936 Hz` or `54 MHz / (1679 × 536) = 60.004 Hz`. This is a source-derived model, not a live test of the boot-package patch.
- pvaibhav/slot keeps the baseline FDT tuple at 26 MHz / 820×536 and writes its separate 528-MHz / divider-10 candidate at runtime. Under the related pinned sun50iw9 PLL formula, this yields 52.8-MHz DCLK and about 120.131 Hz. Its 60-Hz pair matches the current device's prior readback, but the high mode has not been validated on the current panel.
- Importantly, the Slot probe expects 26 MHz / HT820; the original boot-package FDT uses 54 MHz / HT840 or HT1679. Therefore the original package profile would fail Slot's exact timing gate even on an FJ-named panel. They should not be layered together.

Neither implementation provides the desired policy of UI 120 Hz with GAME 60 Hz by default: the boot-package method fixes one mode across all use until a package is reflashed, while Slot applies one global preference on startup/wake and does not switch mode on game entry.

## 5. Applicability to our RG34XX non-SP

Our live ADB/FDT evidence is:

- `BASEOS_TARGET=rg34xx`
- `lcd_driver_name=rg34xx_v1`
- `lcd_if=0`
- 720×480, DCLK property 26 MHz, HT/VT 820×536
- bootarg `lcd_type=old`
- UI/GAME runtime capture: IRQ293 59.667 / 59.583 Hz; PLL raw `0x88001703` in both.

Both third-party approaches use the FJ panel identity in their intended profile; this device reports `rg34xx_v1`. The base timing also differs from the original boot package's 54-MHz profile. A common U-Boot init symbol containing `fj035fhd05` does not change the live FDT identity and is not evidence that the FJ Linux display profile is interchangeable.

**Decision:** use these repositories as reference designs only. Do not relax panel identity checks, flash their boot packages, or copy their 54-MHz timing / PLL / TCON values into our H700 HAL. Continue with the exact `rg34xx_v1 + lcd_type=old` kernel #19 path, independent UI-present and armGBA committed-frame measurement, and the existing fail-closed policy. Keep all hardware writers disabled and `hardware_applied=0`.

## References

- [Original branch README](https://github.com/TheGammaSqueeze/rgxx_boot_package_mods/blob/rg34xx/README.md)
- [Original 120-Hz FDT](https://github.com/TheGammaSqueeze/rgxx_boot_package_mods/blob/rg34xx/120hz/sunxi.fex)
- [Original 60-Hz FDT](https://github.com/TheGammaSqueeze/rgxx_boot_package_mods/blob/rg34xx/60hz/sunxi.fex)
- [Original 120-Hz package configuration](https://github.com/TheGammaSqueeze/rgxx_boot_package_mods/blob/rg34xx/120hz/boot_package.cfg)
- [Original 120-Hz raw flash script](https://github.com/TheGammaSqueeze/rgxx_boot_package_mods/blob/rg34xx/flashscript/RG34XX_120Hz.sh)
- [pvaibhav Slot refresh writer](https://github.com/pvaibhav/slot/blob/5206e3fa39aa4a9b702cd9048a07dab3b5ab95da/crates/slot-power/src/refresh.rs)
