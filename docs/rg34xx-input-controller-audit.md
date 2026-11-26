# RG34XX non-SP input controller source audit

**Status:** the 2026-11-26 active RG34XX live FDT confirms the 17-key GPIO map, PIO provider, active-low flags and 20 ms declared poll interval. Runtime GPIO driver binding and electrical register readback remain unverified; no GPIO scanner or input MMIO driver is enabled.

## 1. Independent board-source comparison

The following non-SP RG34XX device trees both declare a `gpio-keys-polled`
controller with the same 17 GPIO button mappings:

- KNULLI boot-package DTS, pinned at
  [57afa662009641d181f57241bcc480119b131530](https://github.com/knulli-cfw/knulli-linux/blob/57afa662009641d181f57241bcc480119b131530/package/boot/uboot-h700/rg34xx/boot_package/rg34xx.dts#L4619).
  Its `poll-interval` is 20 ms.
- MustardOS RG34XX-H DTS, pinned at
  [8b7787a178548d51bc5376d52583dd2a91f5091f](https://github.com/MustardOS/internal/blob/8b7787a178548d51bc5376d52583dd2a91f5091f/device/rg34xx-h/package/sunxi.dts#L4618).
  Its `poll-interval` is 4 ms.

Both use pinctrl phandle `0x4b`, compatible `allwinner,sun50iw9p1-pinctrl`,
PIO base `0x0300b000`, and `#gpio-cells = <6>`. Their GPIO argument tuple is
`port, pin, mux, pull, drive, flags`. The pinned sun50iw9p1 GPIO provider's
`sunxi_pinctrl_gpio_of_xlate()` confirms those six fields; its GPIO key parser
uses `flags.data & OF_GPIO_ACTIVE_LOW`, and `OF_GPIO_ACTIVE_LOW = 1`.
Consequently the last cell `1` in every listed key is an active-low flag.
The port and pin indices map to the PA/PE names below through the sun50iw9p1
pinctrl pin enumeration.

## 2. GPIO map confirmed by active live FDT

| Input | Pin | Linux input event in reference DTS |
|---|---|---|
| Up | PA6 | EV_ABS / code 0x11 (ABS_Y), value -1 |
| Down | PE0 | EV_ABS / code 0x11 (ABS_Y), value +1 |
| Left | PA8 | EV_ABS / code 0x10 (ABS_X), value -1 |
| Right | PA9 | EV_ABS / code 0x10 (ABS_X), value +1 |
| A | PA0 | EV_KEY, code 0x130 |
| B | PA1 | EV_KEY, code 0x131 |
| X | PA3 | EV_KEY, code 0x132 |
| Y | PA2 | EV_KEY, code 0x133 |
| Start | PA4 | EV_KEY, code 0x137 |
| Select | PA5 | EV_KEY, code 0x136 |
| L1 | PA10 | EV_KEY, code 0x134 |
| L2 | PA11 | EV_KEY, code 0x13a |
| R1 | PA12 | EV_KEY, code 0x135 |
| R2 | PA7 | EV_KEY, code 0x13b |
| Menu | PE3 | EV_KEY, code 0x138 |
| Volume Up | PE1 | EV_KEY, code 0x73 |
| Volume Down | PE2 | EV_KEY, code 0x72 |

The 2026-11-26 active live FDT confirms all 17 children are `okay` and use
mux select 0, pull value 1, drive level 2 and active-low flag 1. In the pinned
sun50iw9p1 kernel header, `SUNXI_PINCTRL_PULL_UP = 1`; this is a weak pull-up
declaration, not "no pull". The live node declares a 20 ms poll interval.
These remain device-tree settings, not electrical register readbacks. The
4 ms MustardOS value is a different reference configuration and is not used
for this device.

## 3. ADC and PMIC key paths

The active live FDT itself marks `/soc@03000000/gpadc` (`allwinner,sunxi-gpadc`)
as `disabled` and `/soc@03000000/keyboard`
(`allwinner,keyboard_1350mv`) as `disable`. The active tree therefore
describes the 17-key polled GPIO device, not an enabled ADC keyboard. This
does not prove that the Linux driver bound or that the physical keys produce
the expected runtime events.

The active tree separately declares `x-powers,axp2101-pek` at
`/soc@03000000/twi@0x07081400/pmu/powerkey@0`, status `okay`. This PMIC
power key is not part of the 17-pin gamepad map.

## 4. Live-FDT parser extension

The existing read-only inventory already captures `/sys/firmware/fdt`.
`tools/inspect_h700_fdt.py` now adds an `input_devices` report containing:

- PIO/GPIO controller compatible, status, register cells and GPIO-cell count;
- `gpio-keys` parent status, poll interval and each child's label, Linux code,
  event type/value, GPIO provider and decoded sun50iw9p1 six-cell arguments;
- GPADC / ADC-keyboard compatible, status, register, IRQ, key count and channel
  metadata;
- separate PMIC power-key nodes.

The parser explicitly labels these as static DT declarations. The host tests
include synthetic GPIO, active-low, EV_ABS, disabled ADC and PMIC-key nodes.
No device register is read or written by this parser.

The 2026-11-26 target capture was extracted from a 143,936-byte live FDT
(573 nodes; compatible `allwinner,h616` / `arm,sun50iw9p1`). Its SHA-256 is
`c17f896f60e2f50eada76c3db6005ea05e149238bf225379a9dc517212088449`.
Only the sanitized `input_devices` section was published:
[report](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-input-fdt-20261126/INPUT-DEVICES.md)
and [JSON extract](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-input-fdt-20261126/live-fdt-input-devices.json).

## 5. Remaining gates before hardware sampling

1. **Kernel binding and event verification:** capture `/proc/bus/input/devices`
   and `getevent -lp` (when available), then perform a physical key-by-key
   event check. The FDT capture alone cannot prove a bound driver or event path.
2. **Pin ownership/electrical state:** inspect only already-exported pinmux /
   pinconf readback. Do not mount debugfs, claim GPIOs, or change mux/pull
   settings during the audit. A live FDT value is not a register readback.
3. **Hardware scanner:** only after binding and ownership evidence agree may a
   cooperative polled scanner be connected to the PIO data registers. Do not
   add EINT/GIC configuration until its routing and lifecycle are separately
   proven. Keep debounce policy explicit and measured.
4. **Power key:** retain AXP2101-PEK as a separate service. Do not add it to the
   17-key GPIO table or infer PMIC register handling from this input audit.

## 6. Data-only logical input adapter

The work branch now contains `h700_rg34xx_input_map.c/.h`, a 17-entry
mapping sourced from the target's active live FDT, and a snapshot-to-edge
converter that feeds the existing bounded `h700_input_queue`. It accepts
caller-supplied Port A / Port E data-level snapshots, interprets the active-low
flags, and produces press/release edges. It also records Linux event metadata
for source traceability.

This component intentionally performs **no MMIO**, pinmux or pull
configuration, GPIO request, debounce, wait, EINT or GIC work. Its first
sample reports keys already held; subsequent calls report only changed
states. The caller must supply stable samples and own polling/debounce. The
FDT's 20 ms poll interval is recorded as metadata, not enforced by this
adapter.

Host tests cover map uniqueness and metadata, idle/press/hold/release,
initially-held inputs, all four directions, unused bits and all-keys
transitions. AArch64, AArch32 ARM-state and AArch32 Thumb-2 freestanding
compiles are part of the H700 architecture check. Passing these tests does
not qualify the GPIO hardware or enable the scanner.
