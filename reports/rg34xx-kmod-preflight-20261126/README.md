# RG34XX kernel-module / MMIO preflight — 2026-11-26

Read-only shell probe run over ADB at 20:45 CST on the connected RG34XX
(`Linux rg34xx 4.9.170 #19 SMP PREEMPT Tue May 26 13:58:08 CST 2026 aarch64`),
checking what would be needed to read `SYS_CFG` from kernel context after the
userspace `/dev/mem` path proved unusable (see `../rg34xx-rvbar-selector-20261126/`).

Nothing was loaded, mounted, built or written. `hardware_applied = 0`.

## Findings

| Item | Value |
|---|---|
| `CONFIG_MODULES` | `y` |
| `CONFIG_MODULE_UNLOAD` | `y` |
| `CONFIG_KALLSYMS` | `y` |
| `CONFIG_DEVMEM` | `y` |
| `CONFIG_STRICT_DEVMEM` | **not set** |
| `/lib/modules/4.9.170` | exists (directory, no `build` link) |
| `/lib/modules/4.9.170/build` | **missing** |
| `/lib/modules/4.9.170/build/Makefile` | **missing** |
| `/lib/modules/4.9.170/build/Module.symvers` | **missing** |
| `/usr/src/linux`, `/usr/src/linux-4.9.170`, `/usr/src/linux-headers-4.9.170` | all **missing** |

Relevant `T`/`t` symbols present in `/proc/kallsyms` (names only, as requested):
`__ioremap`, `devm_ioremap_resource`. No exact `ioremap` / `iounmap` / `readl` /
`regmap_init_mmio` symbol names matched the pattern.

Platform drivers matching `sys|sunxi|ccu|regmap` include **`syscon`**, plus the
sunxi set (`sunxi-reset`, `sunxi-rtc`, `sunxi-twi`, `sunxi-mmc`, `sunxi_pwm`,
`sunxi-init-gpio`, `ion-sunxi`, `eeprom-sunxi-sid`, …). The `syscon` driver being
present is the relevant one for a `SYS_CFG` regmap.

`/sys/kernel/debug/regmap` currently exposes only `5-0034`,
`5070400.thermal_sensor`, `5096000.codec`, `5097000.ahub_dam_plat` — no SYS_CFG
instance.

`/proc/iomem` entries around the block:

```
0300b000-0300b3ff : /soc@03000000/pinctrl@0300b000
04020000-04020fff : /soc@03000000/sdmmc@04020000
04021000-04021fff : /soc@03000000/sdmmc@04021000
04022000-04022fff : /soc@03000000/sdmmc@04022000
05002c00-05002fff : /soc@03000000/twi@0x05002c00
05070400-050707ff : /soc@03000000/thermal_sensor
07000000-070001ff : /soc@03000000/rtc@07000000
07022000-070223ff : /soc@03000000/pinctrl@07022000
07081400-070817ff : /soc@03000000/twi@0x07081400
```

The `SYS_CFG` block at `0x03000000` (and the `0x03000024` `VER_REG` used by the
RVBAR probe) has **no reserved resource entry** of its own — only the pinctrl
sub-range appears — so a kernel-side accessor would have to map it explicitly
rather than inherit an existing reservation.

## Implication

Modules are *supported* by the running kernel, but this image ships **no build
tree and no `Module.symvers`**, so an out-of-tree module cannot be compiled
against it on-device or from the rootfs. A kernel-side `SYS_CFG` read would
therefore need the matching vendor kernel source and a separately built,
version-matched module (or a rebuilt kernel), which is a structural change, not
a read-only ADB action.

## Files

| File | Content |
|---|---|
| `kmod-preflight.txt` | Verbatim output of the probe (64 lines). |
| `kmod-preflight.sh` | The exact script that produced it. |
| `SHA256SUMS.txt` | SHA-256 of the files above, excluding itself. |

No device serial, MAC address, host path, or full `/proc/kallsyms` dump is
included; only the filtered symbol names above.
