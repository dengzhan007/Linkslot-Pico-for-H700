# RG34XX #19 kernel DRM-info SMC binary audit

## 1. Input identity

Source file: user-provided complete GPT `PARTNAME=boot` partition, captured read-only from the current RG34XX test card.

| Item | Value |
|---|---|
| boot partition size | 67,108,864 bytes (64 MiB) |
| boot partition SHA-256 | `41cfaa8f9ade4a5e05a89188abb7db154243e38b8e19c53b730f0a45ff325516` |
| Android boot header | v2, `ANDROID!` |
| page size | 2048 bytes |
| compressed kernel | 7,018,071 bytes, gzip |
| uncompressed kernel Image | 17,461,256 bytes, SHA-256 `39cf8c3eb9a75ff0eda51b4074cd65860c5aa5386093bcaf69339819421d84b7` |
| kernel version string | `Linux version 4.9.170 ... #19 SMP PREEMPT Tue May 26 13:58:08 CST 2026` |
| Image header | `ARM\x64`, image_size `0x1138000`, text_offset `0x80000` |

The embedded version string exactly matches the live RG34XX runtime identity in `tee-ion-runtime.txt`: Linux 4.9.170 `#19`, built 2026-05-26 13:58:08 CST. This makes the extracted payload the correct binary for the runtime audit; no other model's kernel is substituted.

## 2. Address notation

Disassembly was performed on the decompressed AArch64 `Image`, wrapped in a temporary ELF for `llvm-objdump`. Its `.text` VMA was set to `0x01138000` solely to make file offsets directly visible as disassembly labels. **These labels are not claimed to be live kernel virtual addresses or payload entry addresses.** The authoritative locations below are raw byte offsets from the beginning of the uncompressed Image.

| Code | Raw Image file offset | Synthetic disassembly label |
|---|---:|---:|
| ION platform initialization / heap creation routine | `0x666244` | `0x1799244` |
| secure heap type comparison (`cmp w1, #6`) | `0x66636c` | `0x179936c` |
| direct BL from ION routine to DRM helper | `0x666380` | `0x1799380` |
| DRM-info SMC helper body | `0x67663c` | `0x17ae63c` |
| SMC argument/result wrapper | `0x675098` | `0x17ae098` |
| AArch64 `smc #0` result-capture stub | `0x010120` | `0x1148120` |

## 3. Static call chain in the target #19 binary

The disassembled ION initialization routine at Image offset `0x666244` has the structure of the platform driver's probe path: it allocates the driver state, creates the ION device, parses the device-tree heap definitions, allocates the heap array, iterates over each configured heap, creates each ION heap and adds it to the ION device.

Inside that loop, the machine code loads the heap's type field and compares it with integer `6` at offset `0x66636c`. If it is not 6, execution branches around the DRM call. If it is 6, the code prepares the three output pointers and executes a **direct `BL`** at offset `0x666380` to the helper at `0x67663c`.

That helper is not identified merely by a symbol name. Its instructions independently establish its semantics:

1. It constructs `x0 = 0xB200000F` (`mov x0, #15`; `movk x0, #0xB200, lsl #16`).
2. It calls the wrapper at Image offset `0x675098`.
3. The wrapper calls the common AArch64 `smc #0` stub at Image offset `0x010120`; the stub captures `x0..x3` into the result structure.
4. The helper checks result `a0`. A nonzero status follows the failure branch, prints `drm config service not available`, and returns `-EINVAL`.
5. A zero status copies result `a1`, `a2`, and `a3` to the three supplied outputs, prints `drm_base`, `drm_size`, and `tee_base`, and returns 0.

The caller proceeds to the ION heap creation call after the helper returns; it does not branch on the helper's return value. Thus creation of the debugfs `secure` heap is compatible with either an SMC success or an SMC failure. A present but empty secure heap does not reveal the returned DRM range.

## 4. Runtime correlation

The 21:20:49 read-only runtime report independently shows:

- `CONFIG_ION_SUNXI=y`, `CONFIG_ION_OF=y`, `CONFIG_TEE=y`, `CONFIG_OPTEE=y`;
- the platform device is bound under `/sys/bus/platform/drivers/ion-sunxi/ion`;
- live FDT `/ion/heap_secure@0` has `compatible = allwinner,secure`, `id = 6`, and type `ion_secure`;
- `/sys/kernel/debug/ion/heaps/secure` exists, although current allocation is zero;
- the captured dmesg still contains neither the success fields (`drm_base`, `drm_size`, `tee_base`) nor the failure text (`drm config service not available`).

The matching Allwinner 4.9.170 reference implementation maps the `allwinner,secure` heap to `ION_HEAP_TYPE_SECURE`; in its `enum ion_heap_type`, that enum value is 6. The target binary itself compares the parsed heap type with 6 before the direct DRM helper call. Reference sources: [sunxi_ion.c](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/staging/android/ion/sunxi/sunxi_ion.c), [uapi/ion.h](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/staging/android/uapi/ion.h), [OP-TEE core.c](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/tee/optee/core.c). The reference tree is a comparison source, not proof of byte-identical source for the target kernel.

## 5. Interpretation and remaining discrepancy

The exact target binary contains a direct secure-heap probe call to an implementation that issues Allwinner `GET_DRM_INFO` (`0xB200000F`) through `SMC #0`. The live FDT and runtime secure-heap node indicate that the type-6 condition was configured and the ION driver successfully created the secure heap. This is strong evidence that the vendor DRM-info helper should have been reached during ION initialization.

However, the captured dmesg has neither branch's expected `printk` output. Therefore the exact SMC return status and the values of `drm_base`, `drm_size`, and `tee_base` remain **unknown**. Do not state that the SMC succeeded or failed. The missing messages are an unresolved discrepancy between the static call path and captured runtime log; it is not evidence of a zero return value or a zero-sized carveout.

The standard OP-TEE API `api uid mismatch` remains a separate UID/revision/capability probe and must not be used as the vendor DRM-info SMC return status.

## 6. Memory-safety boundary

No physical address is approved by this audit. In particular, `[0x7ec00000,0x7fc00000)` remains an owner-unknown 16 MiB reserved interval and must stay excluded. The monitor, CMA, framebuffer, kernel/FDT/initrd, other memblock reservations and top-of-RAM fragments also remain excluded. Do not infer available payload RAM by subtracting memblock reservations from total RAM.

No SMC, ioctl, driver rebind, debugfs control write, boot-media write, or on-device instrumentation was performed.