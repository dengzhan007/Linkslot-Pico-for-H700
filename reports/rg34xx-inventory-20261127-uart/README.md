# RG34XX read-only inventory + UART0 runtime view — 2026-11-27

Complete output of `scripts/run-rg34xx-inventory.ps1` from the
`porting/linkslot-1.0.45-feasibility` branch at `e87de1e`, run against the connected
RG34XX (non-SP) over ADB. Local directory:
`build/rg34xx-inventory-20261127-185937/`, 42 files, 20 MB.

```
powershell -ExecutionPolicy Bypass -File .\scripts\run-rg34xx-inventory.ps1
ADB device: <serial>
Block device: /dev/mmcblk0
Read-only inventory completed.
No write was issued to /dev/mmcblk0.
```

All 42 files are published. The device-side collector reads only `/dev/mmcblk0`, `/proc`,
`/sys` and already-mounted debugfs text views; nothing was mounted, written, or loaded.
`hardware_applied = 0`.

## Capture identity

| Item | Value |
|---|---|
| kernel | `Linux rg34xx 4.9.170 #19 SMP PREEMPT Tue May 26 13:58:08 CST 2026 aarch64` |
| captured_at | `Fri Nov 27 10:59:36 UTC 2026` |
| block device | `/dev/mmcblk0`, 15 552 479 232 B, logical sector 512 B |
| boot0 primary | SHA-256 `bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f` (body `ff847db9…`) |
| boot0 secondary | SHA-256 `5f143679e0d2c5293d22fee47a47ea6c17120c7a19e6007247c8b13710aa5ef9` (body `1bdc26d9…`) |
| TOC1 package | SHA-256 `e6c20e3a54c51b9336c798e3a4fa1ca6c45b28c6f70456e2db9c252aa4ba4604`, checksum `0x5923eb81`, 4 items |
| TOC1 `monitor` | 107 216 B, SHA-256 `676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31` |
| TOC1 `dtb` | 144 384 B, SHA-256 `c28c87380ffab00148bf4dc457e70b8c6c576d6470dcfbc53df64addddf89603a1` |
| live FDT | 143 936 B, 573 nodes, `sun50iw9`, 5 interrupt controllers, 2 timer nodes |
| env partition | complete, 16 777 216 B, CRC OK, 25 variables |

Both TOC1 hashes and the `monitor`/`dtb` sizes match the earlier captures
(`rg34xx-inventory-20261126-timer`), so this run re-confirms the same boot chain.
The live FDT hash differs from the 11-26 capture because `chosen/bootargs` carries
per-boot values (`bootreason=button`, build-time strings); the tree itself is unchanged
at 573 nodes.

## `uart-runtime.txt` — what it establishes

This is the new file in this inventory. It is a **post-boot Linux view only**, explicitly
not BL33-entry state (`evidence_scope=Linux post-boot runtime only`), and it was produced
without `/dev/mem`, without MMIO reads, without pinmux/CCU writes and without mounting
debugfs (`no_devmem=1`, `no_mmio_reads_or_writes=1`, `no_debugfs_mount=1`).

| Item | Value | Read from |
|---|---|---|
| early console | `earlyprintk=sunxi-uart,0x05000000` | `/proc/cmdline` (filtered) |
| console | `console=ttyS0,115200` | `/proc/cmdline` (filtered) |
| `ttyS0` backing node | `/sys/devices/platform/soc/uart0`, dev `248:0` | `/sys/class/tty/ttyS0` |
| `ttyS1` backing node | `/sys/devices/platform/soc/uart1`, dev `248:1` | `/sys/class/tty/ttyS1` |
| PH0/PH1 function | `uart0`, group `PH0`/`PH1` | `pio/pinmux-pins` |
| PH0/PH1 config | input bias pull-up, drive strength 20 mA | `pio/pinconf-pins` |
| UART0 clock | `24 000 000 Hz`, parent `apb2` = 24 MHz | `clk_summary` |
| `r_pio` | no PH0/PH1 match | `r_pio/pinmux-pins`, `pinconf-pins` |

Reading notes that should not be overstated:

- `pinmux-pins` prints `uart0 (GPIO UNCLAIMED)`. **UNCLAIMED refers to the GPIO request
  state, not to the pin function.** The function is `uart0` on both pins; this is not
  evidence that the UART is unused.
- `pinconf-pins` reports `input bias pull up` on PH0/PH1, which is the pinctrl state the
  kernel left behind. It is not a measurement of the pad in a bare-metal entry path.
- `dmesg` shows five `expect_func as:uart0_jtag, but muxsel(3) is func:jtag/uart0`
  lines at `0.786 s`. That is the vendor pinctrl driver reconciling an `uart0_jtag`
  group against a mux that is partly `jtag`; it does not by itself say which pin is
  which. Treat it as a boot-time pinctrl observation, not a JTAG/UART arbitration result.
- `/proc/tty/driver/serial` is absent, so there is no driver-level line status,
  baud or FIFO readback in this capture.

The net usable statement for the M1 diagnostic payload is narrow and positive:
**UART0 at `0x05000000`, PH0/PH1 muxed to `uart0`, clocked at 24 MHz, with `ttyS0` at
115200 as the console** — i.e. a proof-of-life UART target already exists in the stock
runtime. It does **not** prove that a bare-metal BL33 can drive those pads, nor that the
clock/pinctrl state survives BL31 without re-initialisation.

## Files

| File | Content |
|---|---|
| `summary.txt` | collector summary, block device geometry, env capture result |
| `cmdline.txt` | kernel command line (serial masked, see `REDACTION.md`) |
| `device-and-block-layout.txt`, `partitions.txt`, `gpt-fdisk.txt` | partition table and block layout |
| `boot0-primary.bin/.json`, `boot0-secondary.bin/.json` | 128 KiB Boot0 captures + parsed DRAM parameters |
| `toc1-package.bin`, `toc1-extracted/` | 1.25 MiB TOC1 package and 4 extracted items (`u-boot`, `monitor`, `dtbo`, `dtb`) |
| `env-partition.bin`, `env-report.json` | 16 MiB `env` partition and parsed variables |
| `live-fdt.dtb`, `live-fdt.json` | live device tree and inspector output |
| `memory-map-report.json` | memblock/reserved/CMA/framebuffer report |
| `cpu-power-runtime.txt`, `sid-cell-report.json`, `cpuinfo.txt` | CPUFreq/thermal/regulator/NVMEM/SID read-only view |
| `debugfs-memory-reservations.txt`, `iomem.txt`, `meminfo.txt`, `page-size*` | memory reservations and geometry |
| `display-runtime.txt`, `input-runtime.txt`, `interrupts.txt` | display, input and IRQ runtime views |
| `uart-runtime.txt` | UART0 runtime view (above) |
| `tee-ion-runtime.txt`, `kernel-driver-provenance.txt`, `mounts.txt` | TEE/ION, driver provenance, mounts |
| `dmesg-hardware-lines.txt` | filtered hardware log lines |
| `raw-sha256.txt` | **verbatim** device-side manifest of the unredacted captures |
| `REDACTION.md` | exactly what was masked and why |
| `SHA256SUMS.txt` | SHA-256 of every published file except itself |

## Redaction and sensitivity

Three files carried the device serial and/or a local workstation path and were redacted;
details and before/after hashes are in `REDACTION.md`. A byte-level scan of the published
directory found 0 occurrences of the 19-byte serial, 0 MAC-shaped strings
(`mac_addr=`, `wifi_mac=`, `bt_mac=` are empty in the boot arguments), and 0 occurrences
of the workstation user name, host name or build path.

`raw-sha256.txt` and `memory-map-report.json` still record the **pre-redaction**
`live-fdt.dtb` hash `b761cbcc…45a89`; this is expected and documented, not a checksum
failure of the published DTB.
