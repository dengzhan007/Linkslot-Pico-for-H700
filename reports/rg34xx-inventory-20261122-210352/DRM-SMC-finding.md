# Allwinner DRM-info SMC follow-up — 2026-11-22 21:03

Read-only inventory captured on branch `audit/rg34xx-ion-drm-followup-20261122`
(`00fe17d`, docs: distinguish OP-TEE probe from Allwinner DRM SMC), device
serial redacted. Collector only reads files/nodes; no SMC, no ION ioctl, no SD
write, no boot-area modification. Device side confirmed no write to `/dev/mmcblk0`.

## Result: neither log class present

`tee-ion-runtime.txt` section "ION / TEE / DRM / secure / CMA kernel log lines"
contains:

```
[    1.040792] optee: probing for conduit method from DT.
[    1.040801] optee: api uid mismatch
```

There is **no** `drm_base` / `drm_size` / `tee_base` success line and **no**
`drm config service not available` failure line. Per the follow-up decision
table this is the third case:

| Reported content | Next step |
| --- | --- |
| `drm_base`/`drm_size`/`tee_base` | compare with the 16 MiB unknown reservation |
| `drm config service not available` | record SMC failure status, keep tracing secure-memory source |
| **neither (this capture)** | **check whether the target kernel executes this vendor call path** |

## Evidence on whether the vendor path ran

Present and consistent with an ION/TEE-capable kernel:

- `/proc/config.gz` exports `CONFIG_CMA=y`, `CONFIG_DMA_CMA=y`,
  `CONFIG_OF_RESERVED_MEM=y`, `CONFIG_ION=y`, `CONFIG_TEE=y`, `CONFIG_OPTEE=y`.
- `/dev/ion` exists; ION debugfs heaps `sys_user`, `cma`, `secure` are all
  exported. `/sys/bus/platform/drivers/ion-sunxi` exists with `ion` bound.
- Live FDT has `/firmware/optee` (`linaro,optee-tz`, `method=smc`) and
  `/ion/heap_secure@0` (`allwinner,secure`, status okay), but heap `base=0x0`,
  `size=0` for all three heaps.

Missing / negative:

- `/dev/tee0` and `/dev/teepriv0` are `NOT_PRESENT`; loaded modules show
  `NO_MATCHING_MODULES` (drivers may be built-in).
- The `secure` heap client list is empty (0 allocations).
- The config grep did **not** surface `CONFIG_SUNXI_ION` (only generic
  `CONFIG_ION`); the kernel still exposes an `ion-sunxi` platform driver, so the
  config symbol name may differ or the driver is built from another option.
- No `GET_DRM_INFO` success/failure line in dmesg at any log level captured.

## Interpretation

The presence of `api uid mismatch` only shows the standard OP-TEE API UID
handshake aborted; it is a different SMC path from Allwinner's
`OPTEE_SMC_GET_DRM_INFO`, so it must not be rewritten as "vendor DRM SMC
failed". Equally, the empty secure heap and zero FDT heap base/size must not be
read as "secure memory is zero-sized".

Because neither vendor log class appears, this capture **cannot confirm** that
the target kernel executes `optee_probe_drm_configure()`, nor that the vendor
SMC was attempted. Kernel config export does not show `CONFIG_SUNXI_ION`
explicitly, so the next check is whether this 4.9.170 `#19` build actually
contains the `sunxi_ion.c` -> `optee_probe_drm_configure()` call (e.g. via the
kernel image / module strings or source config), not an inference from runtime
absence.

## 16 MiB unknown reservation — still unassigned

`debugfs-memory-reservations.txt` keeps reserved entry 5:
`0x7ec00000..0x7fbfffff`, size 16384K (16 MiB), with no owner in `/proc/iomem`.
Without `drm_base`/`drm_size`/`tee_base` there is no basis to match it to the
DRM/TEE carveout. It, the other unknown reserved entries and top-of-RAM
fragments remain excluded from any payload/heap/loader use. The
framebuffer `0x7bf24800..0x7c076000` also stays excluded.

## Files in this directory

- `tee-ion-runtime.txt` — verbatim (no serial present)
- `dmesg-hardware-lines.txt` — `usb_serial_number` redacted
- `cmdline.txt` — `snum=` / `androidboot.serialno=` redacted
- `live-fdt.json` — `snum=` / `androidboot.serialno=` in chosen bootargs redacted
- `iomem.txt`, `debugfs-memory-reservations.txt`, `meminfo.txt`,
  `memory-map-report.json` — verbatim (diagnostic addresses preserved)
- `DRM-SMC-finding.md` — this note

No firmware images or environment partition were uploaded.
