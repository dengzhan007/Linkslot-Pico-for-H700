# pvaibhav/slot runtime capture (ui)

- Serial: [REDACTED]
- Phase: ui
- Samples: 27
- Requested duration: 30 seconds
- Slot PID(s) from /proc scan: 2842

## Display IRQ rates

| IRQ | label | delta | seconds | estimated Hz |
|---:|---|---:|---:|---:|
| 292 | wakeupgen  88 Level     dispaly | 1767 | 29.624 | 59.648 |
| 293 | wakeupgen  64 Level     dispaly | 1766 | 29.624 | 59.614 |

IRQ rates are kernel interrupt-counter deltas (scanout/vblank proxies), not UI present FPS or emulator committed game FPS.
The vendor `fps` and `ref_fps` values are driver reports; `ref_fps` may be stale.
Process discovery matches comm, cmdline and executable path, including generic `slot` binaries launched from `.system/slot-pvaibhav`.
No mode-set, MMIO, `sunxi_dump` write, input injection, ROM/save write, or boot-package write is performed.
