# RG34XX input runtime — collector output + physical key event test (2026-11-26)

Two artifacts from the same powered-on session on the connected RG34XX, both read-only:

1. `input-runtime.txt` — the input block of the re-run inventory collector
   (`scripts/collect-rg34xx-readonly-inventory.sh`, via `scripts/run-rg34xx-inventory.ps1`):
   `/proc/bus/input/devices`, a `getevent -lp` attempt, and the already-mounted pinctrl debug views.
2. `key-events.*` — the raw `input_event` stream from `/dev/input/event1` (`ANBERNIC-keys`) while all
   17 game keys were pressed one by one, plus its decoded forms.

## Files

| file | content |
| --- | --- |
| `input-runtime.txt` | Verbatim collector output: 3 input devices, `NOT_PRESENT: getevent`, `pinmux-pins` / `pinconf-pins` for PA0–PA12 and PE0–PE3. |
| `key-events.bin` | Raw capture, 1728 bytes = 72 × 24-byte `struct input_event`. Bit-exact source for everything else. |
| `key-events.hexdump.txt` | The same bytes as a hexdump, one record per line with its index and decoded header. |
| `key-events.csv` | One row per record: index, epoch seconds, ISO-8601 UTC, relative seconds, type, code, value with names. |
| `key-events-summary.json` | Machine-readable roll-up: window, counts, observed codes, the one extra press group. |
| `KEY-EVENT-TEST.md` | The test itself: method, full press sequence with pins and hold times, DT-vs-runtime coverage table, deviations, and the three static/runtime inconsistencies. |
| `SHA256SUMS.txt` | SHA-256 of the files above, excluding itself. |

## Why `getevent` was not used

`getevent` does not exist on this image (`/system/bin`, `/system/xbin` both absent; the collector logged
`NOT_PRESENT: getevent`). `adb exec-out` on this adbd returns `error: closed`, so the capture uses
`adb shell "cat /dev/input/event1" > key-events.bin`. Byte transparency was verified first by comparing
`dd if=/bin/sh bs=32 count=1 | md5sum` on the device with the local file (`6fbeca97…5442`), so the
`adb shell` path does not insert CR/LF on this device.

Decoding is done offline on the host with `struct.unpack('<qqHHi')`. Nothing was compiled, pushed to
`/dev/input`, or executed on the device beyond `cat`.

## Result in one line

All 17 device-tree keys produced real evdev events, in the expected order except that X and Y were
pressed in the opposite order and one extra R1 press was recorded between START and MENU (operator
confirmed it as an accidental press while reaching for MENU). See `KEY-EVENT-TEST.md` §3–§4.

## Sensitive data

`input-runtime.txt` contains no serial or MAC: every `U: Uniq=` field is empty, and the sysfs paths are
generic kernel paths. The key event capture contains only timestamps, types, codes and values. No device
serial, MAC address or host path is present in any file in this directory.

## Scope

Read-only throughout: no `EVIOCGRAB`, no keymap change, no `/dev/input` write, no `sendevent`, no GPIO or
pinctrl register write, no debugfs mount. The power key (`axp2202-pek`, `input0`) was excluded by request.
This proves the key → evdev leg only; it does not prove that Slot or the frontend consumes these events
correctly, nor that the libretro key mapping is right.
