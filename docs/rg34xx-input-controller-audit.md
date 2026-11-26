# RG34XX non-SP input controller source audit

**Status:** the active live FDT and physical evdev captures confirm all 17 GPIO keys. Physical X/Y silkscreen labels are reversed relative to DTS child names. A read-only PA_DAT/PE_DAT single-shot sampler is linked but not called from LK main; pinmux/pinconf debug views conflict and are not treated as write authorization.

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

| DTS child label | Pin | Linux input event in active FDT |
|---|---|---|
| Up | PA6 | EV_ABS / code 0x11 (ABS_Y), value -1 |
| Down | PE0 | EV_ABS / code 0x11 (ABS_Y), value +1 |
| Left | PA8 | EV_ABS / code 0x10 (ABS_X), value -1 |
| Right | PA9 | EV_ABS / code 0x10 (ABS_X), value +1 |
| A | PA0 | EV_KEY, code 0x130 |
| B | PA1 | EV_KEY, code 0x131 |
| keyX | PA3 | EV_KEY, code 306 (0x132) |
| keyY | PA2 | EV_KEY, code 307 (0x133) |
| Start | PA4 | EV_KEY, code 0x137 |
| Select | PA5 | EV_KEY, code 0x136 |
| L1 | PA10 | EV_KEY, code 0x134 |
| L2 | PA11 | EV_KEY, code 0x13a |
| R1 | PA12 | EV_KEY, code 0x135 |
| R2 | PA7 | EV_KEY, code 0x13b |
| Menu | PE3 | EV_KEY, code 0x138 |
| Volume Up | PE1 | EV_KEY, code 0x73 |
| Volume Down | PE2 | EV_KEY, code 0x72 |

The table above describes **DTS child labels**, not the physical silkscreen:

| Physical key label | Actual pin | Runtime evdev code | Active-FDT child |
|---|---|---:|---|
| X | PA2 | 307 | `keyY` |
| Y | PA3 | 306 | `keyX` |

The dedicated X/Y re-test pressed each physical key three times; all six presses agreed. The H700 logical map now routes physical X/PA2/code307 to `H700_BTN_X` and physical Y/PA3/code306 to `H700_BTN_Y`.

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

## 5. Runtime input evidence and pinctrl debug-view caveats

The 2026-11-26 same-session capture confirms:

- /proc/bus/input/devices registers ANBERNIC-keys as input1 with handlers
  kbd js0 event1; the platform device uses gpio-keys-polled. The raw event1
  capture contains 72 records (18 press/release groups), and all 17 expected
  physical keys were observed. One additional R1 press was operator-confirmed
  as accidental while reaching for MENU.
- getevent is absent. A read-only cat of /dev/input/event1 was decoded
  offline; no sendevent, keymap change or event-device write was performed.
- Physical X/PA2/code307 and Y/PA3/code306 were confirmed by a dedicated
  six-press retest. DTS child keyX is PA3/code306, and keyY is PA2/code307.
- A third input device, dierct-keys-polled/event2, exists but is not present
  in the 17-key active-FDT node. Its origin/events are unknown; do not merge
  it into the gamepad map. The AXP2101 power-key device/event0 is separate.

The existing pinctrl debug views disagree with the active FDT and each other:
PA0-PA12 and PE1 are shown as MUX UNCLAIMED, PE0/PE2/PE3 as function csi1,
while pinconf-pins reports input bias disabled and 20 mA output drive for all
17 pins. Despite these labels, the kernel input device emits real events from
all 17 keys. Treat the debug views as insufficient to establish pad ownership
or a safe state to rewrite. Do not change MUX/PULL/drive based on these views.

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
transitions. The map uses measured physical X/PA2/code307 and Y/PA3/code306.
AArch64, AArch32 ARM-state and AArch32 Thumb-2 freestanding compiles are part
of the H700 architecture check.

## 7. Read-only PIO input sampler

The scanner reads only two sunxi PIO DATA registers:

| Register | Offset from PIO base | Derivation |
|---|---:|---|
| PA_DAT | 0x10 | Bank A index 0 * 0x24 + DATA offset 0x10 |
| PE_DAT | 0xA0 | Bank E index 4 * 0x24 + DATA offset 0x10 |

PIO base is 0x0300b000. The existing H700 AArch32 MMU hook maps the
0x03000000..0x03100000 peripheral window as Device memory. The scanner's
injectable IO descriptor exposes read32 only; no write callback exists.
The production implementation performs volatile 32-bit reads of PA_DAT and
PE_DAT and passes those two snapshots to the logical edge mapper. It does not
request GPIO ownership, alter mux/pull/drive, access EINT/GIC registers, or
configure a timer.

The scanner is a single-shot call, not a scheduler. Its caller owns cadence;
the live FDT declares 20 ms. No extra debounce has been selected because the
evdev capture does not characterize electrical contact bounce. Host tests
check the exact two register offsets, idle snapshots, physical X/Y edge order,
and fail-closed ABI validation.

This is host/mock and cross-build validation only. The sampler is not called
from target_init(), apps_init() or LK main and has not been executed on the
device. No LK input loop or hardware input latency claim is made.


## 8. Bounded read-only PIO register capture

The Linux inventory report identifies the live PIO resource as
`0300b000-0300b3ff : /soc@03000000/pinctrl@0300b000`. This resource
contains both audited data-register addresses. A separate capture helper,
`scripts/collect-rg34xx-pio-input-readonly.sh`, was added rather than
silently extending the general inventory with an MMIO probe.

The helper is fail-closed. Before its first read it requires a root shell,
a readable character device at `/dev/mem`, a readable `/proc/iomem`, and
the exact expected PIO resource line. It then reads only:

- PA_DAT: physical address `0x0300b010`, one 32-bit word;
- PE_DAT: physical address `0x0300b0a0`, one 32-bit word.

Each `dd` opens `/dev/mem` as input and writes the four-byte result to a
temporary file under the chosen report directory. The helper checks that
exactly four bytes were returned, records `/proc/uptime` with each sample,
and emits `pio-samples.csv`. The default is 240 samples with a nominal
100-ms sleep between samples; the CSV timestamps, not the requested sleep,
are authoritative for observed cadence.

No other PIO offsets are accessed. The helper does not write `/dev/mem`,
change GPIO ownership, alter pinmux/pull/drive, mount debugfs, invoke
`devmem`, or touch EINT/GIC state. If the resource signature, permissions,
or exact read length differ, it exits without broadening the probe.

Run it from a root ADB shell after copying the script to the device:

```sh
adb push scripts/collect-rg34xx-pio-input-readonly.sh /data/local/tmp/pio-readonly.sh
adb shell sh /data/local/tmp/pio-readonly.sh /data/local/tmp/rg34xx-pio-capture 240
adb pull /data/local/tmp/rg34xx-pio-capture ./rg34xx-pio-capture
```

During the capture, press and release one gamepad key at a time, holding
each briefly; do not press the PMIC Power key. Do not infer logical polarity
from FDT alone: compare stable idle samples with the bit changes observed
while each physical key is held. The sample interval may be longer than
100 ms due to per-register `dd` process overhead.

The helper and its static safety regression passed Linux/Windows host CI
[#45762940878](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45762940878)
(89 Python tests on each platform plus shell syntax validation). **No device MMIO read has yet been performed,
and the LK sampler remains disconnected from all runtime entry points.**
