# RG34XX non-SP input controller source audit

**Status:** source-derived reference profile; not yet confirmed against the current device's active live FDT or electrical pin readback. No input MMIO driver is enabled.

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

## 2. Candidate GPIO map

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

All 17 GPIO cells in both reference DTS files use mux select 0, pull value 1,
drive level 2 and active-low flag 1. The actual electrical state must still be
confirmed on the tested device; these are declarations, not pin readbacks.
The two reference poll intervals differ by 5x, so neither is selected as the
LK scan interval yet.

## 3. ADC and PMIC key paths

Both reference DTS files also declare `allwinner,sunxi-gpadc` and
`allwinner,keyboard_1350mv` nodes as disabled (`disabled` / `disable`).
This makes the GPIO-key path the leading candidate for the 17 mapped inputs,
but it does not prove that the BaseOS active FDT uses the same node status.

The power key is a separate PMIC input and is not part of the 17-pin table.
Its active compatible and event behavior must be read from the tested device's
live FDT and input-device inventory. Do not infer it from a sibling boot DTS
or mix it into the GPIO scan table.

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

## 5. Conditions before implementing the LK scanner

1. Re-run the existing read-only RG34XX inventory on the target and inspect the
   new `input_devices` block from the actual `live-fdt.json`. Confirm the
   active `gpio-keys` child map, PIO phandle, status and PMIC power-key node.
2. Capture `/proc/bus/input/devices` and `getevent -lp` (when present) to
   confirm which kernel input devices actually bind. The inventory extension
   does not synthesize key events or press buttons.
3. Read pinmux/pinconf debugfs state if already exported. Do not mount debugfs,
   change mux/pull, request new GPIO ownership or write PIO registers during
   this source-verification step.
4. Only after the active FDT and pin ownership agree should an LK GPIO scanner
   be implemented. Keep it cooperative/polled initially; do not add EINT/GIC
   setup until a separate interrupt design and PPI mapping are verified.
5. Keep PMIC power-key handling separate. The current runtime event ABI can
   receive a power event later, but this audit does not authorize PMIC access.

The H700 input queue / C ABI and Rust gesture adapter already provide the
upper-layer event boundary. This audit adds board-source evidence and live-FDT
visibility only; it does not connect GPIO hardware to that boundary.
