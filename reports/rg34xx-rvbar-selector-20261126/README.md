# RG34XX / H700 RVBAR selector probe — no JSON produced (`/dev/mem` EFAULT)

**Date:** 2026-11-26 20:29–20:32 CST (device 12:29–12:32 UTC).
**Script:** `scripts/read-h700-rvbar-selector-readonly.py` at repo `fbab663`, md5 `2739be359c087d9bae318dc461bf2d73` (identical on host and at `/tmp/read-h700-rvbar-selector.py`).
**Result:** **the script exited 2 with `[Errno 14] Bad address` and wrote 0 bytes to stdout. No JSON was generated, so none is published here.** Nothing has been synthesised or reconstructed.

---

## 1. What was run

```
git switch porting/linkslot-1.0.45-feasibility
git pull --ff-only origin porting/linkslot-1.0.45-feasibility      # -> fbab663
adb push scripts/read-h700-rvbar-selector-readonly.py /tmp/read-h700-rvbar-selector.py
adb shell 'id && python3 /tmp/read-h700-rvbar-selector.py --json'
```

`id` returned `uid=0(root) gid=0(root)`, so the root requirement was met.

The first attempt stopped at the shell: **`python3` is not on this image's PATH** (`PATH=/sbin:/usr/sbin:/bin:/usr/bin`; no `python3` applet in busybox). A portable CPython 3.12.16 is shipped with the LEI-V4 frontend at `/mnt/sdcard/Frontends/LEI-V4/System/python3/bin/python3`; running that interpreter works (`Python 3.12.16`), so the probe was re-run with it to reach the script's own read path instead of stopping at a missing interpreter:

```
/mnt/sdcard/Frontends/LEI-V4/System/python3/bin/python3 \
    /tmp/read-h700-rvbar-selector.py --json > /tmp/rvbar-out.json 2> /tmp/rvbar-err.txt
EXIT=2
stdout bytes: 0
stderr: ERROR: read-only VER_REG[2:0] read failed: [Errno 14] Bad address
```

## 2. Why it failed

`read_selector()` performs a single 32-bit `os.pread()` of `/dev/mem` at `0x03000024` with `O_RDONLY`. The kernel rejected it with `EFAULT` (`Errno 14`), which Python surfaces as *Bad address*. The script's own error handling turned that into exit code 2 and an empty stdout — the correct fail-closed behaviour, since it never prints a selector it did not read.

An independent probe at the same address confirms it is not a Python or `pread` artifact:

```
dd if=/dev/mem bs=4 skip=12582921 count=1     # 0x03000024
dd: /dev/mem: Bad address
rc=1
```

This matches the earlier, broader finding in
[`../rg34xx-pio-input-capture-20261126/`](../rg34xx-pio-input-capture-20261126/): on this build every `/dev/mem` read returns `EFAULT`, at every offset including `0`, one byte, and whole pages. `CONFIG_DEVMEM=y` but `CONFIG_STRICT_DEVMEM` is **not set**, so the usual `devmem_is_allowed()` gate is not the cause (that would give `EPERM`, not `EFAULT`).

## 3. Read-only alternatives checked, and why none was used

The goal was the JSON the script generates, so substituting a different data source would not be that JSON. The available read-only interfaces were nevertheless enumerated:

- `debugfs` top level has no `sunxi_dump`-style register node (only `sunxi_gpu`, `sunxi_pinctrl`, `ccudbg`, `dispdbg`, `regmap`, …).
- `regmap` exposes only 4 devices — `5-0034`, `5070400.thermal_sensor`, `5096000.codec`, `5097000.ahub_dam_plat` — none of which is the SYS_CFG block at `0x03000000`.

No debugfs mount, no kernel module, no boot argument, no register write and no attempt to weaken `/dev/mem` protection was made.

## 4. What would unblock it

1. **A kernel-side read** of `SYS_CFG VER_REG` (offset `0x24`) through the driver that already maps the block — the same approach noted for the PIO registers in the `rg34xx-pio-input-capture-20261126` report. That runs in kernel context and is not subject to the userspace `/dev/mem` translation path.
2. **A bootloader/early-boot capture**, which is also the only way to answer the question the probe's own `scope_note` flags: a post-Linux-boot sample cannot prove the pre-BL33 CPU reset/RVBAR state.

Either path is a functional change, not a read-only ADB session, so neither was attempted here.

## 5. Files

| File | Content |
|---|---|
| `README.md` | This note. |
| `rvbar-probe-log.txt` | Verbatim command/exit-code log: device identity, `id`, both-end script md5, interpreter discovery, the exact run with empty stdout and the `Errno 14` stderr, the independent `dd` probe at `0x03000024`, the devmem config, and the debugfs/regmap enumeration. |
| `SHA256SUMS.txt` | SHA-256 of the files above, excluding itself. |

No device serial, MAC address, host user name or host path appears in these files; the script itself only ever reads the low 3 bits and never prints the full register word.
