# RG34XX gpiolib debugfs key test — 2026-11-26 (completed)

Read-only, timed `cat` of the existing `/sys/kernel/debug/gpio` view while the operator pressed
each of the 17 physical game keys once. Collector `scripts/collect-rg34xx-gpio-debugfs-readonly.sh`
(repo `3ce6bb3`), run as root (`id -u` = 0) with 1200 samples over a 126.0 s device-uptime window.

**Result: 17/17 keys, 34 clean `hi → lo → hi` transitions, no duplicates and no simultaneous lines.**
Power key was not pressed. No debugfs mount, no GPIO export/request, no register write.
`hardware_applied = 0`.

## Requested files

| File | Content |
|---|---|
| `preflight-gpio.txt` | Verbatim one-shot read of `/sys/kernel/debug/gpio` taken before the timed capture (2449 B). |
| `gpio-debugfs-samples.txt` | Verbatim timed capture: 1200 numbered blocks, each with `/proc/uptime` and the full gpiolib GPIO dump (2 982 090 B). |
| `summary.txt` | Verbatim device-produced run header and totals. |

## Added analysis

| File | Content |
|---|---|
| `GPIO-DEBUGFS-TEST.md` | Per-key press/release sample number, uptime, gpiolib line and label; confirms the X/Y silkscreen swap and the PA/PE pin split; records the discarded incomplete first attempt; read-only and sensitivity notes. |
| `SHA256SUMS.txt` | SHA-256 of the four files above, excluding itself. |

## Note on the first attempt

A 600-sample attempt covered only 12 keys before the operator stopped, so it was **not** used.
It was kept locally outside the published directory; the device directory was removed and the
capture re-run from scratch. Everything published here comes from one uninterrupted 1200-sample
window (`/proc/uptime` 301.83 → 427.83 s).

## Sensitivity

The device serial in `/proc/cmdline` (`snum=`, `androidboot.serialno=`) was never written into
these files. Scans of the three requested files found 0 serial hits, 0 colon-hex MAC strings,
0 host user name and 0 host path.
