# RG34XX PIO input capture — 2026-11-26 (failed: `/dev/mem` EFAULT)

Bounded, read-only attempt to sample the H700 PIO data registers `PA_DAT` (`0x0300b010`)
and `PE_DAT` (`0x0300b0a0`) while pressing each of the 17 gamepad keys once, using
`scripts/collect-rg34xx-pio-input-readonly.sh`.

**Outcome: the capture did not run.** The collector was invoked exactly as specified
(`/data/local/tmp/rg34xx-pio-capture 400`, ADB shell as root), the target PIO resource was
present in `/proc/iomem`, `/dev/mem` was readable at the inode level — and the very first
4-byte `dd` read returned `Bad address` (EFAULT). The script fails closed, so it exited at
sample 0 with `EXIT=1`, wrote no data rows, and no keys were pressed.

Every other offset fails identically, including offset `0`, a single byte, and a full page
of low RAM, so this is a property of this kernel build's `/dev/mem` read path, not of the
two registers. `CONFIG_STRICT_DEVMEM` is not set and `selinux=0`, so the usual devmem gate
and LSM denial are both ruled out.

No workaround was attempted: no kernel protection was disabled, no alternative memory
node was substituted, and no register, pinmux, GPIO, EINT or GIC write was performed.
`hardware_applied = 0`.

Full reasoning, evidence and viable next steps: **`PIO-CAPTURE-FAILURE.md`**.

## Contents

| File | What it is |
|---|---|
| `PIO-CAPTURE-FAILURE.md` | The analysis: what was run, exit codes, the offset-independent probe matrix, why the usual causes are excluded, what was deliberately not done, reproduce steps, next steps. |
| `summary.txt` | Verbatim device-produced header from the failed run (collector id, uid, kernel, requested sample count, PIO addresses, matching `/proc/iomem` line). |
| `pio-samples.csv` | Verbatim device output — **header row only**, 30 bytes. No samples exist. |
| `pa-read-error.txt` | Verbatim `dd` error: `dd: /dev/mem: Bad address`. |
| `probe-matrix.txt` | The read-only offset/size probe matrix proving the failure is not address-specific. |
| `kernel-config-devmode.txt` | `CONFIG_DEVMEM` / `CONFIG_STRICT_DEVMEM` / arm64 settings read from `/proc/config.gz`. |
| `iomem-pinctrl.txt` | `/proc/iomem` view: the `0300b000-0300b3ff` pinctrl resource and the `System RAM` node boundary. |
| `dmesg-tail.txt` | Kernel log tail: no `ioremap` warning, no oops, no panic around the reads. |
| `SHA256SUMS.txt` | SHA-256 of the files above, excluding itself. |

## Read-only guarantees

- Only `dd if=/dev/mem … 2>&1 >/dev/null` style reads and `/proc`/`sysfs`/config reads were issued.
- The script's own safety properties hold: `/dev/mem` is only ever an input; the only outputs are four-byte temporary files inside the output directory.
- No key injection, no `EVIOCGRAB`, no keymap change, no write to `/dev/input`, no MMIO write, no pinctrl/pinconf change, no module load, no boot argument change.
- Device state after the run: script and output directory remain under `/data/local/tmp`; the device was not rebooted.

## Sensitivity

The device serial appears in `/proc/cmdline` (`snum=`, `androidboot.serialno=`) and was
deliberately **not** published; the cmdline is not reproduced in any file here. No MAC
address, no host path and no host user name is present in the published files. The probe
outputs were captured with `2>&1 >/dev/null` and contain only the `dd` diagnostics.
