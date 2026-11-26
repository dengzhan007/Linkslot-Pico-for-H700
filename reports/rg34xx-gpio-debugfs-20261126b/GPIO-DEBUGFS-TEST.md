# RG34XX — physical key test against gpiolib debugfs (read-only)

**Date:** 2026-11-26, collector start `Thu Nov 26 06:16:37 UTC` (host 14:16 CST).
**Collector:** `scripts/collect-rg34xx-gpio-debugfs-readonly.sh` (repo `3ce6bb3`, script md5 `2dc2498a9bd70cb6beacd66b3ec9a54e`, identical on device and host).
**Result:** **17/17 physical game keys produced a clean `hi → lo → hi` transition. No extra, missing or simultaneous key events.**
**Writes:** none. No debugfs mount, no GPIO export/request, no pinmux/pinconf/GPIO/EINT/GIC write. `hardware_applied = 0`.

---

## 1. Method and preconditions

Commands, in the local repository root:

```
adb push scripts/collect-rg34xx-gpio-debugfs-readonly.sh /data/local/tmp/gpio-debugfs-readonly.sh
adb shell id -u                       # 0
adb shell sh /data/local/tmp/gpio-debugfs-readonly.sh /data/local/tmp/rg34xx-gpio-debugfs-20261126b 1200
adb pull /data/local/tmp/rg34xx-gpio-debugfs-20261126b ./rg34xx-gpio-debugfs-20261126b
```

`id -u` returned `0`, so the root branch was taken. `/sys/kernel/debug/gpio` was already mounted and readable (2449 bytes, `gpiochip1: GPIOs 0-287, parent: platform/pio, pio`). The collector only `cat`s that existing file; it does **not** mount debugfs, export/request a GPIO line, or touch any register. `preflight-gpio.txt` is a separate one-shot read taken before the timed capture; `gpio-debugfs-samples.txt` holds the timed samples.

Captured file hashes, device and host (identical):

| file | MD5 | bytes |
|---|---|---|
| `preflight-gpio.txt` | `314af28d210bee104a330603c3cab13a` | 2449 |
| `gpio-debugfs-samples.txt` | `5e3cbb7be69311347e481f81948f46cc` | 2 982 090 |
| `summary.txt` | `406658f5b1507abb98bdb7722f73857d` | 589 |

Timed window: 1200 samples, `/proc/uptime` 301.83 s → 427.83 s (**126.0 s**), nominal 100 ms interval (actual ~105 ms, recorded per sample in the file). The first key was pressed at uptime 309.11 s, i.e. ≈7.3 s of idle lead-in, as requested.

## 2. Observed presses

All keys were driven from idle, one at a time, in the instructed order. `hi → lo` is the press (all lines are declared `active_low` in the device tree); `lo → hi` is the release. Times are device `/proc/uptime` in seconds; hold durations are quantised by the ~105 ms sampling step.

| # | Key as instructed (silkscreen) | gpiolib line / label | press (sample, uptime) | release (sample, uptime) | hold |
|---|---|---|---|---|---|
| 1 | A | `gpio-0` / GPIO Key a | 69, 309.11 | 70, 309.21 | 0.10 |
| 2 | B | `gpio-1` / GPIO Key b | 83, 310.58 | 84, 310.68 | 0.10 |
| 3 | X | `gpio-2` / GPIO Key y | 99, 312.26 | 100, 312.36 | 0.10 |
| 4 | Y | `gpio-3` / GPIO Key x | 133, 315.83 | 134, 315.93 | 0.10 |
| 5 | L1 | `gpio-10` / GPIO Key L1 | 149, 317.51 | 151, 317.72 | 0.21 |
| 6 | R1 | `gpio-12` / GPIO Key R1 | 165, 319.19 | 166, 319.30 | 0.11 |
| 7 | SELECT | `gpio-5` / GPIO Key select | 189, 321.71 | 191, 321.92 | 0.21 |
| 8 | START | `gpio-4` / GPIO Key start | 207, 323.60 | 208, 323.71 | 0.11 |
| 9 | MENU | `gpio-131` / GPIO Key Menu | 231, 326.13 | 232, 326.23 | 0.10 |
| 10 | L2 | `gpio-11` / GPIO Key L2 | 255, 328.65 | 257, 328.86 | 0.21 |
| 11 | R2 | `gpio-7` / GPIO Key R2 | 273, 330.54 | 274, 330.64 | 0.10 |
| 12 | D-pad Up | `gpio-6` / GPIO Key Up | 288, 332.12 | 289, 332.22 | 0.10 |
| 13 | D-pad Down | `gpio-128` / GPIO Key Down | 301, 333.48 | 303, 333.69 | 0.21 |
| 14 | D-pad Left | `gpio-8` / GPIO Key left | 327, 336.21 | 328, 336.32 | 0.11 |
| 15 | D-pad Right | `gpio-9` / GPIO Key right | 343, 337.90 | 344, 338.01 | 0.11 |
| 16 | Volume + | `gpio-129` / GPIO Key Vol up | 373, 341.05 | 375, 341.26 | 0.21 |
| 17 | Volume − | `gpio-130` / GPIO Key vol down | 389, 342.73 | 391, 342.95 | 0.22 |

Total: **34 transitions = 17 × (press, release)**, across 17 distinct gpiolib lines. No line toggled twice in the same key group, no two lines changed in the same sample, and the power key was not touched.

## 3. What this independently confirms

- **X/Y silkscreen swap, third independent confirmation.** The physical key marked **X** drives `gpio-2`, whose kernel label is `GPIO Key y` (DT `keyY`, `PA2`, evdev `307`); the physical key marked **Y** drives `gpio-3`, labelled `GPIO Key x` (DT `keyX`, `PA3`, evdev `306`). This matches the evdev re-test in [`../rg34xx-input-runtime-20261126/XY-REPEAT.md`](../rg34xx-input-runtime-20261126/XY-REPEAT.md) and the device-tree labels in [`../rg34xx-input-fdt-20261126/`](../rg34xx-input-fdt-20261126/). The kernel's names follow the DT nodes; the two *physical* silkscreens are swapped relative to them.
- **Bank A / bank E split is right.** PA0–PA12 (`gpio-0…12`) carry A/B/X/Y/L1/R1/SELECT/START/L2/R2 and the d-pad left/right/up; PE0/PE1/PE2/PE3 map to `gpio-128/129/130/131` and carry Down / Vol+ / Vol− / Menu — exactly the pins the FDT report assigned to those keys. The earlier `pinmux-pins` claim that `PE0/PE2/PE3` belong to `csi1` is again contradicted by live behaviour.
- **One-to-one gpiolib view.** No line other than the 17 listed above changed state during the key window, so the keys are wired to the expected gpiolib chip and there is no second key source firing in parallel.
- **Menu is the same key used to exit the emulator** (`gpio-131`, `PE3`), not a separate power/mode key; consistent with evdev `312` / `BTN_TL2`.

## 4. First attempt (incomplete, not used)

A first run with 600 samples (`/proc/uptime` 167.10 → 230.29 s, 63.19 s window) was started and the operator reported before finishing. It covered only the first 12 keys (A, B, Y, X, L1, R1, SELECT, START, MENU, L2, R2, Up) and stopped before Down/Left/Right/Vol+/Vol−. That data was preserved locally for reference and **excluded from this report**; the device output directory was removed and the capture re-run from scratch with 1200 samples. The 1200-sample artifacts published here are a single uninterrupted window.

## 5. Read-only guarantees and sensitivity

- Only `cat /sys/kernel/debug/gpio` and reads of `/proc/uptime`; the collector's own header declares `no_debugfs_mount=1`, `no_gpio_export_or_request=1`, `no_pinmux_pinconf_gpio_eint_gic_writes=1`.
- No `/dev/mem` access in this run, no `dd` to a device node, no keymap or `/dev/input` write, no process restart, no input injection, no boot argument change.
- The device serial appears in `/proc/cmdline` (`snum=`, `androidboot.serialno=`) and was deliberately **not** captured into any published file. Scan of the three published files: 0 hits for the serial, 0 colon-hex MAC strings, 0 host user name, 0 host path.
- Device left untouched apart from the pushed script and the `/data/local/tmp` capture directory; no reboot.
