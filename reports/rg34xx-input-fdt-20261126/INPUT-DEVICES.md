# RG34XX (H700) live-FDT input_devices report — 2026-11-26

Expanded `tools/inspect_h700_fdt.py` output for the live device tree dumped on the connected RG34XX, extracted from `live-fdt.json` produced by `scripts/run-rg34xx-inventory.ps1`. Only the `input_devices` section is published; see the redaction note below.

## 1. Capture provenance

| field | value |
| --- | --- |
| live-fdt.dtb SHA-256 | `c17f896f60e2f50eada76c3db6005ea05e149238bf225379a9dc517212088449` |
| dtb bytes / totalsize | 143936 / 143936 |
| node count | 573 |
| model | `sun50iw9` |
| compatible | `allwinner,h616, arm,sun50iw9p1` |
| collector | `scripts/run-rg34xx-inventory.ps1` (read-only, root ADB) |
| block device read | `/dev/mmcblk0` (read only, no write issued) |
| extraction time (UTC) | 2026-11-26T04:34:21Z |

## 2. GPIO key devices

`/soc@03000000/gpio_keys`, compatible `gpio-keys-polled`, status `okay`, 17 keys.

### 2.1 Event-key entries (EV_KEY)

| node | label | type (code) | linux,code | value | pin | active_low | drive | mux |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `keyvol-` | GPIO Key vol down | 1 (EV_KEY) | 114 | 1 | `PE2` | yes | 2 | 0 |
| `keyVol+` | GPIO Key Vol up | 1 (EV_KEY) | 115 | 1 | `PE1` | yes | 2 | 0 |
| `keyA` | GPIO Key a | 1 (EV_KEY) | 304 | 1 | `PA0` | yes | 2 | 0 |
| `keyB` | GPIO Key b | 1 (EV_KEY) | 305 | 1 | `PA1` | yes | 2 | 0 |
| `keyX` | GPIO Key x | 1 (EV_KEY) | 306 | 1 | `PA3` | yes | 2 | 0 |
| `keyY` | GPIO Key y | 1 (EV_KEY) | 307 | 1 | `PA2` | yes | 2 | 0 |
| `keyL1` | GPIO Key L1 | 1 (EV_KEY) | 308 | 1 | `PA10` | yes | 2 | 0 |
| `keyR1` | GPIO Key R1 | 1 (EV_KEY) | 309 | 1 | `PA12` | yes | 2 | 0 |
| `keySEl` | GPIO Key select | 1 (EV_KEY) | 310 | 1 | `PA5` | yes | 2 | 0 |
| `keyStart` | GPIO Key start | 1 (EV_KEY) | 311 | 1 | `PA4` | yes | 2 | 0 |
| `keyMenu` | GPIO Key Menu | 1 (EV_KEY) | 312 | 1 | `PE3` | yes | 2 | 0 |
| `keyL2` | GPIO Key L2 | 1 (EV_KEY) | 314 | 1 | `PA11` | yes | 2 | 0 |
| `keyR2` | GPIO Key R2 | 1 (EV_KEY) | 315 | 1 | `PA7` | yes | 2 | 0 |

### 2.2 Analog-axis entries (EV_ABS)

| node | label | type (code) | linux,code | value | pin | active_low | drive | mux |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `keyLeft` | GPIO Key left | 3 (EV_ABS) | 16 | -1 | `PA8` | yes | 2 | 0 |
| `keyRight` | GPIO Key right | 3 (EV_ABS) | 16 | 1 | `PA9` | yes | 2 | 0 |
| `keyDOWN` | GPIO Key Down | 3 (EV_ABS) | 17 | 1 | `PE0` | yes | 2 | 0 |
| `keyUP` | GPIO Key Up | 3 (EV_ABS) | 17 | -1 | `PA6` | yes | 2 | 0 |

The d-pad is declared as two `EV_ABS` axes: `keyLeft`/`keyRight` both use code 16 with values -1/+1, `keyUP`/`keyDOWN` both use code 17 with values -1/+1. The rows must be read as axis directions rather than as independent buttons.

`keyMenu` is `linux,code = 312` (`BTN_TL2`) on `PE3`. This confirms the earlier ADB finding that the Menu/exit key is 312 and not `BTN_MODE` (316).

Caveat on kernel names: several values here do not carry their nominal `input-event-codes.h` meaning for this pad. `keyL2`/`keyR2` are 314/315, which the kernel names `BTN_SELECT` and `BTN_START`, while the physical select/start keys are 310/311 (`BTN_TL`/`BTN_TR`). Consumers must key off the device-tree labels and the observed evdev codes, never the nominal kernel button names.

## 3. Analog key controllers

| node | compatible | status | key_count | channel_count | channel_select |
| --- | --- | --- | --- | --- | --- |
| `/soc@03000000/gpadc` | `allwinner,sunxi-gpadc` | disabled | 5 | 1 | 1 |
| `/soc@03000000/keyboard` | `allwinner,keyboard_1350mv` | disable | 5 | None | None |

Both analog key controllers (`gpadc` as `disabled`, `keyboard` as `disable`) are inactive, so the shipped RG34XX input path is the polled `gpio_keys` node above, not the ADC keyboard.

## 4. GPIO controllers referenced by the key nodes

| node | compatible | status | gpio_cells | interrupt_cells |
| --- | --- | --- | --- | --- |
| `/soc@03000000/pinctrl@0300b000` | `allwinner,sun50iw9p1-pinctrl` | okay | 6 | 3 |
| `/soc@03000000/pinctrl@07022000` | `allwinner,sun50iw9p1-r-pinctrl` | okay | 6 | 3 |
| `/soc@03000000/twi@0x07081400/pmu/axp_gpio@0` | `-` | okay | 6 | None |

All 17 keys resolve through `&pio` (`allwinner,sun50iw9p1-pinctrl`, phandle `0x4b`, `#gpio-cells = 6`) with **weak pull-up** (`pull_config = 1`), active-low inputs, drive level 2 and mux 0 (GPIO input). The pinned Allwinner kernel header defines `SUNXI_PINCTRL_PULL_UP = 1` and `SUNXI_PINCTRL_NO_PULL = 0`; therefore `pull_config = 1` is not an unpulled input. Bank letters used are `PA` (A/B/X/Y/L1/L2/R1/R2/select/start/dpad) and `PE` (Menu, vol+/vol-, dpad up/down).

## 5. PMIC power key

| node | compatible | status |
| --- | --- | --- |
| `/soc@03000000/twi@0x07081400/pmu/powerkey@0` | `x-powers,axp2101-pek` | okay |

`powerkey@0` under the AXP2101 PMIC parent is the hardware power button path; it is separate from the polled gpio-keys device and is not usable as a gamepad input.

## 6. Scope and limits

- Static device-tree declarations only. These fields do not prove live pin mux/pull readback, electrical polarity, runtime driver binding, physical key operation, or ADC calibration.
- Not covered here: `/proc/bus/input/devices`, evdev handler numbers, runtime key repeat, or any key injection test. This file is a static device-tree declaration dump only.
- No hardware register, PLL, TCON, CCU or PMIC value was written; the collector only reads `/dev/mmcblk0` and `/proc`-level state and writes to `/tmp` on the device.

## 7. Redaction applied before publishing

- Dropped top-level `chosen_bootargs`: it carried `snum=<redacted>` and `androidboot.serialno=<redacted>` (device serial), plus empty `mac_addr=`, `wifi_mac=`, `bt_mac=` placeholders.
- Dropped top-level `file`: it exposed the local workstation build path and user name.
- The published JSON keeps only `input_devices` plus non-identifying provenance (DTB SHA-256, size, model, compatible, node count).
- A post-generation scan for the device serial, MAC-shaped strings and the workstation user name over these two files is recorded in `README.md` in this directory.
