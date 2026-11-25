# ION config symbols and kallsyms — 2026-11-22 21:20

Read-only inventory captured on branch `audit/rg34xx-ion-drm-followup-20261122`
(`b84fff0`, test: assert correct ION symbols and redacted kallsyms). Collector
only reads files/nodes; no SMC, no ION ioctl, no SD write, no boot-area change.
Device side confirmed no write to `/dev/mmcblk0`.

## 1. Config symbols: both enabled

`tee-ion-runtime.txt` now exports from `/proc/config.gz`:

```
CONFIG_CMA=y
CONFIG_DMA_CMA=y
CONFIG_OF_RESERVED_MEM=y
CONFIG_ION=y
CONFIG_ION_SUNXI=y
CONFIG_ION_OF=y
CONFIG_TEE=y
CONFIG_OPTEE=y
```

**This corrects the 21:03 report.** That report said the config grep did not
surface a SUNXI ION symbol and could not prove the driver was built in. The
older collector grepped `CONFIG_SUNXI_ION`, but the matching Allwinner 4.9.170
Kconfig/Makefile uses `CONFIG_ION_SUNXI` (and `CONFIG_ION_OF` for DT heap
parsing). The symbol name was wrong; the driver is in fact configured in.

## 2. kallsyms: all three key functions present

New section `relevant built-in ION / DRM symbols (names only)`:

```
[address redacted] ion_secure_heap_create
[address redacted] sunxi_ion_probe
[address redacted] optee_probe_drm_configure
MODULE_SYSFS_NOT_PRESENT: ion_sunxi
MODULE_SYSFS_PRESENT: optee
```

Addresses are intentionally dropped by the collector; only symbol names are
kept. All three functions are built into the running `#19` kernel. Together
with `CONFIG_ION_SUNXI=y` and `CONFIG_TEE=y`, this establishes that the vendor
`GET_DRM_INFO` call chain (`sunxi_ion_probe` -> `optee_probe_drm_configure`)
is compiled into this kernel.

## 3. What is still unknown

The dmesg section still has **no** `drm_base` / `drm_size` / `tee_base` line and
**no** `drm config service not available` line. Only the standard OP-TEE path
appears:

```
[    1.040792] optee: probing for conduit method from DT.
[    1.040801] optee: api uid mismatch
```

Symbol presence proves the code exists, not that it ran or that the SMC
succeeded. Per branch audit doc section 13.1, the next step is to obtain the
source/config or kernel image matching Linux `#19` and check whether
`optee_probe_drm_configure()` is actually called, its log level/output path,
and the secure-heap creation order. Do not issue SMC from userspace to force a
return value and do not touch the unknown reserved memory.

Runtime state is otherwise unchanged from 21:03: `/dev/ion` present,
`/dev/tee0`/`/dev/teepriv0` `NOT_PRESENT`, `ion-sunxi` bound, ION heaps
`sys_user`/`cma`/`secure` exported with secure heap empty, live FDT secure heap
base/size `0x0`.

## 4. Memory safety

`[0x7ec00000,0x7fc00000)` (16 MiB) still has no owner; it cannot be matched to
a DRM/TEE carveout without `drm_base`/`drm_size`. CMA
`[0x77c00000,0x7bc00000)`, framebuffer `[0x7bf24800,0x7c076000)`, and all other
unowned reserved entries remain excluded. No payload load/entry address is
approved.

## Files in this directory

- `tee-ion-runtime.txt` — verbatim (config symbols + kallsyms names; no serial present)
- `dmesg-hardware-lines.txt` — `usb_serial_number` redacted
- `cmdline.txt`, `live-fdt.json` — `snum=` / `androidboot.serialno=` redacted
- `iomem.txt`, `debugfs-memory-reservations.txt`, `meminfo.txt`,
  `memory-map-report.json` — verbatim (diagnostic addresses preserved)
- `ION-symbol-finding.md` — this note

No firmware images or environment partition were uploaded.
