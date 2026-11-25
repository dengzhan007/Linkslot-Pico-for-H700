# Boot partition capture — local-only (20261122-214708)

## Scope of this upload

This directory contains the **text/JSON subset** of the read-only inventory
captured on `audit/rg34xx-ion-drm-followup-20261122` @ `c6da96d`.
All binary artifacts are intentionally **kept locally and NOT uploaded**:

- `boot-partition.bin` (67,108,864 bytes, 64 MiB)
- `boot0-primary.bin`, `boot0-secondary.bin`, `toc1-package.bin`
- `env-partition.bin`, `live-fdt.dtb`, `toc1-extracted/items/*.bin`

Reason: the full boot image may contain vendor firmware components. The raw
partition image stays in the local `build/rg34xx-inventory-20261122-214708/`
directory only.

## Boot partition identification (from `summary.txt`)

```
boot_partition_capture=complete source=sysfs PARTNAME=boot device=mmcblk0p4 bytes=67108864 sectors=131072
```

Located by exact GPT `PARTNAME=boot` (not a hardcoded partition number), with a
64 MiB size cap. Only `dd` read operations were used; the device-side collector
confirmed no writes to `/dev/mmcblk0`.

SHA-256 of the full boot partition (from `raw-sha256.txt`, image NOT uploaded):

```
41cfaa8f9ade4a5e05a89188abb7db154243e38b8e19c53b730f0a45ff325516  boot-partition.bin
```

The hash is published here solely so the locally retained image can be
verified without distributing the image itself.

## Redactions applied to the uploaded text/JSON

- `cmdline.txt`, `live-fdt.json`: ADB serial replaced with `[REDACTED]`.
- `dmesg-hardware-lines.txt`: `usb_serial_number:<digits>` replaced with
  `usb_serial_number:[REDACTED]`.
- `gpt-fdisk.txt`: disk identifier GUID replaced with `[REDACTED]`.

`env-report.json` contains only the variable reference `${snum}` (no real
serial value), so it is uploaded verbatim. Kernel virtual addresses and
physical MMIO ranges are retained because they carry diagnostic value.

## Files uploaded

28 text/JSON files, including `summary.txt`, `raw-sha256.txt`,
`tee-ion-runtime.txt`, `boot0-primary.json`, `boot0-secondary.json`,
`env-report.json`, `live-fdt.json`, `memory-map-report.json`, and the
`toc1-extracted/dtb-report.json` / `manifest.json` pair.
