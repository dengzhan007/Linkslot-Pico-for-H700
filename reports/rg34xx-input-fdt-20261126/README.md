# RG34XX (H700) live-FDT input_devices — 2026-11-26

Sanitised publication of the `input_devices` section produced by the expanded
`tools/inspect_h700_fdt.py` on the live device tree of the connected RG34XX.

## Why this directory exists

`scripts/run-rg34xx-inventory.ps1` was re-run against the connected RG34XX with root ADB
(the script hard-fails unless `id -u` returns `0`). The expanded FDT inspector now emits an
`input_devices` object alongside the previous panel/PMIC/USB sections, so this run finally
gives a device-tree-level statement of which input nodes exist and how each key is wired.
Only that section is published here.

Because the full `live-fdt.json` also carries `chosen_bootargs` with `snum=` and
`androidboot.serialno=` (the device serial) plus the local build path in `file`, it is
**not** published. This directory is the reviewed subset.

## Files

| file | content |
| --- | --- |
| `INPUT-DEVICES.md` | Human-readable report: 17 gpio-keys entries with evdev codes and pins, d-pad axis semantics, disabled analog key controllers, GPIO controllers, AXP2101 power key, scope limits, redaction log. |
| `live-fdt-input-devices.json` | Machine-readable extract: non-identifying provenance + the `input_devices` object verbatim. |
| `sensitivity-scan.txt` | Actual grep counts for the serial, MAC-shaped strings and workstation identifiers, over the raw capture (reference) and over both published files. All published counts are 0. |
| `SHA256SUMS.txt` | SHA-256 of the three files above, excluding itself. |

## Capture provenance

- Command: `powershell -ExecutionPolicy Bypass -File .\scripts\run-rg34xx-inventory.ps1`
- One authorised device in `device` state; `adb shell id -u` = `0`.
- Root block device derived from `root=/dev/mmcblk0p5` → `/dev/mmcblk0`; the collector only
  reads it (`dd` to `/tmp`), no write was issued.
- Local output directory: `build/rg34xx-inventory-20261126-123232`.
- `live-fdt.dtb` SHA-256 `c17f896f60e2f50eada76c3db6005ea05e149238bf225379a9dc517212088449`,
  143936 bytes, 573 nodes, model `sun50iw9`, compatible `allwinner,h616` + `arm,sun50iw9p1`.

## Findings that matter downstream

1. The shipped input path is the polled `gpio-keys-polled` node `/soc@03000000/gpio_keys`
   with 17 keys. Both analog key controllers are inactive (`gpadc` `disabled`,
   `keyboard` `disable`), so there is no ADC key path on this build.
2. `keyMenu` is `linux,code = 312` (`BTN_TL2`) on `PE3`. This independently confirms the
   earlier ADB/evdev finding that the Menu/exit key is 312, not `BTN_MODE` (316).
3. The d-pad is declared as two `EV_ABS` axes (code 16 and 17, values -1/+1), not as four
   independent `EV_KEY` buttons.
4. Several codes do not carry their nominal kernel meaning: `keyL2`/`keyR2` are 314/315,
   which the kernel names `BTN_SELECT`/`BTN_START`, while the physical select/start keys are
   310/311 (`BTN_TL`/`BTN_TR`). Consumers must key off the DT labels and observed evdev
   codes, not the nominal kernel button names.
5. Every key resolves through `&pio` (`allwinner,sun50iw9p1-pinctrl`, phandle `0x4b`,
   `#gpio-cells = 6`), active-low, unpulled, drive level 2, mux 0 (GPIO input).

## What this does not prove

Device-tree declarations are static. They do not demonstrate runtime driver binding, live pin
mux/pull readback, electrical polarity as measured, key repeat behaviour, evdev handler
numbers, or that a key press actually reaches a frontend. No key-injection test was run for
this report, no `/proc/bus/input/devices` snapshot is included, and no hardware register,
PLL, TCON, CCU or PMIC value was written.

## Redaction log

Removed before publishing:

- `chosen_bootargs` — contained `snum=<redacted>`, `androidboot.serialno=<redacted>`, and
  empty `mac_addr=` / `wifi_mac=` / `bt_mac=`.
- `file` — contained the local Windows build path including the workstation user name.

Retained: DTB SHA-256, byte size, `totalsize`, `node_count`, `model`, `compatible`, and the
whole `input_devices` object.

Verified afterwards with `sensitivity-scan.txt`: 0 hits for the device serial, 0 hits for
MAC-shaped strings, 0 hits for the workstation user name or repo path, in both published
files.
