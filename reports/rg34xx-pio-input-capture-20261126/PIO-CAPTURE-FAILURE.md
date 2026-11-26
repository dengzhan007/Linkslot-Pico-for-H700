# RG34XX / H700 raw PIO input capture — `/dev/mem` read refused (EFAULT)

**Date:** 2026-11-26 (device clock 05:47 UTC, host 13:47 UTC+8)
**Collector:** `scripts/collect-rg34xx-pio-input-readonly.sh` (`rg34xx-pio-input-readonly-v1`, repo commit `be8523e`, script md5 `a60b6e4d4c5306c4f49caf322cc161cc`)
**Result:** **capture failed at sample 0. Zero register values were read. The 17-key test was therefore not performed.**
**Hardware writes:** none. `hardware_applied = 0`.

---

## 1. What was run

Preflight, all on the connected RG34XX (`sun50iw9`, `allwinner,h616`, `arm,sun50iw9p1`), ADB shell as root:

```
adb push scripts/collect-rg34xx-pio-input-readonly.sh /data/local/tmp/pio-readonly.sh
adb shell id -u
    0
adb shell ls -l /dev/mem
    crw------- 1 root root 1, 1 /dev/mem
adb shell test -r /dev/mem
    READABLE
adb shell grep -i 'pinctrl@0300b000' /proc/iomem
    0300b000-0300b3ff : /soc@03000000/pinctrl@0300b000
```

Pushed blob verified: device `md5sum` = host `md5sum` = `a60b6e4d4c5306c4f49caf322cc161cc` (5065 bytes).

Since `id -u` returned `0`, the root branch was taken, exactly as instructed:

```
adb shell sh /data/local/tmp/pio-readonly.sh /data/local/tmp/rg34xx-pio-capture 400
```

Output:

```
Capture running: press and release one physical gamepad key at a time.
Do not press Power. Hold each key briefly, release, then move to the next.
error: /dev/mem read failed at word index 12594180; see /data/local/tmp/rg34xx-pio-capture/pa-read-error.txt
EXIT=1
```

`pa-read-error.txt` contains exactly:

```
dd: /dev/mem: Bad address
```

`pio-samples.csv` holds the header only — the loop never completed one iteration, so the capture window never opened and no keys were pressed.

## 2. Why it stopped

`read_mem()` in `drivers/char/mem.c` returns `-EFAULT` → `dd` prints *Bad address* when the kernel cannot translate the physical address into a kernel virtual mapping (`xlate_dev_mem_ptr()` returned `NULL`). The script correctly fails closed on the first failed read (`set -eu`) instead of writing a synthesised sample.

## 3. The failure is not address-specific

A read-only probe matrix (`probe-matrix.txt`) was run to separate "this PIO range is blocked" from "all `/dev/mem` reads are blocked". Every probe used the same read-only form (`dd if=/dev/mem … 2>&1 >/dev/null`), no writes anywhere:

| offset | index / size | result |
|---|---|---|
| `0x00000000` | `bs=4 skip=0` | `dd: /dev/mem: Bad address` |
| `0x00000004` | `bs=4 skip=1` | `dd: /dev/mem: Bad address` |
| `0x03000000` | `bs=4096 skip=12288` | `dd: /dev/mem: Bad address` |
| `0x0300b000` (PIO base) | `bs=4 skip=12582912` | `dd: /dev/mem: Bad address` |
| `0x0300b010` (`PA_DAT`) | `bs=4 skip=12594180` | `dd: /dev/mem: Bad address` |
| `0x0300b0a0` (`PE_DAT`) | `bs=4 skip=12594216` | `dd: /dev/mem: Bad address` |
| `0x00000000`, 1 byte | `bs=1 count=1` | `dd: /dev/mem: Bad address`, `rc=1` |
| streaming | `cat /dev/mem` | `cat: read error: Bad address` |

Offset `0`, one byte, and a whole page of low RAM all fail the same way. So the refusal is a property of this kernel build's `/dev/mem` read path, **not** of the two PIO addresses and **not** of anything the collector does.

## 4. What is *not* the cause

Two usual suspects are ruled out by the device's own kernel config (`kernel-config-devmode.txt`, read from `/proc/config.gz`):

```
CONFIG_DEVMEM=y
CONFIG_ARCH_HAS_DEVMEM_IS_ALLOWED=y
# CONFIG_STRICT_DEVMEM is not set
```

(`CONFIG_IO_STRICT_DEVMEM` is not present in this config at all — the grep that produced the file included that symbol and it did not match.)

- `CONFIG_STRICT_DEVMEM` is **off**, so `devmem_is_allowed()` is not filtering the range. If it had been the cause, the call would have returned `-EPERM` (*Operation not permitted*), not `-EFAULT`.
- `selinux=0` on the kernel command line, so no LSM is denying the open. The open itself succeeded — only `read()` fails.
- `/proc/iomem` does expose the range as a non-RAM device resource (`iomem-pinctrl.txt`): `0300b000-0300b3ff : /soc@03000000/pinctrl@0300b000`, and the only `System RAM` node starts at `40000000`, so `0x0300b000` is outside RAM.
- `dmesg` shows no `ioremap` warning, no oops and no panic at the time of the reads (`dmesg-tail.txt`), i.e. the kernel rejected the request cleanly rather than faulting.

**Conclusion:** the kernel returns `NULL` from its physical→virtual translation for `/dev/mem` reads in general on this build (`Linux rg34xx 4.9.170 #19 SMP PREEMPT aarch64`, BSP build of 2026-05-26). Determining the exact in-kernel reason needs the vendor kernel source or a debug build; it cannot be established from userspace without touching kernel protections, which was explicitly out of scope.

## 5. What was deliberately not done

Per instruction, no attempt was made to make `/dev/mem` readable:

- no kernel protection was disabled, and no boot argument was changed;
- no `/dev/mem` alternative was substituted (no `/dev/kmem`, no module load, no `debugfs` register poke, no `sunxi_dump`-style vendor node);
- no pinctrl/pinconf/GPIO/EINT/GIC write of any kind;
- no key presses were requested, because the capture window never opened — pressing keys against a dead sampler would have produced an empty CSV and a misleading "test done" impression.

The only device-side side effects are the pushed script and the output directory under `/data/local/tmp`, both left in place for reproducibility.

## 6. Reproducing the failure

```
adb push scripts/collect-rg34xx-pio-input-readonly.sh /data/local/tmp/pio-readonly.sh
adb shell id -u                                  # 0
adb shell sh /data/local/tmp/pio-readonly.sh /data/local/tmp/rg34xx-pio-capture 400
# -> error: /dev/mem read failed at word index 12594180 ... EXIT=1
```

Any non-empty `OUTDIR` is refused by the script, so remove `/data/local/tmp/rg34xx-pio-capture` first.

## 7. Viable next steps (not executed in this run)

1. **In-kernel scanner.** The same commit range (`be8523e`) adds `platform/h700/runtime/h700_rg34xx_input_scan.c`, which reads the identical two registers through an injected `read32` callback against `0x0300b000`. In kernel context the PIO block is `ioremap`-ed normally, so it is not subject to the userspace `/dev/mem` translation path. Requires building and loading that component — a functional change, not a read-only ADB session.
2. **Vendor diagnostic node.** If the BSP ships an `sunxi_dump`-style debugfs node, it would read registers from kernel context, but probing for it is a different (and structural) task, so it was not attempted here.
3. **Keep the userspace collector.** It remains correct and will start working unchanged on any build where `/dev/mem` reads succeed; no code change is needed.

## 8. Bottom line

The PA_DAT/PE_DAT capture did not happen. The collector was invoked exactly as specified, the device was root, `/dev/mem` was `readable` at the inode level, the PIO resource was present in `/proc/iomem`, and the very first 4-byte read returned `EFAULT` — as does every other offset, including `0`. This is a hard limitation of this kernel build's `/dev/mem`, recorded here instead of being worked around.
