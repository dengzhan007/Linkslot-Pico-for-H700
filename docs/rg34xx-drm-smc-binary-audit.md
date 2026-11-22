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

The embedded version string exactly matches the live RG34XX runtime identity in the 21:20 `tee-ion-runtime.txt` report. The analyzed binary is therefore the kernel image associated with the current device's running `#19` build, not a substitute kernel from another H700 model.

## 2. Address and offset convention

All offsets below are **byte offsets from the beginning of the decompressed 17,461,256-byte Image**. They were verified by AArch64 instruction decoding and direct branch-target calculation against the raw Image bytes. They are not runtime virtual addresses, physical load addresses, Kallsyms addresses, or payload entry addresses.

An earlier draft of this report used incorrect offsets for the caller and wrapper. Those values (`0x666244`, `0x66636c`, `0x666380`, `0x675098`) were wrong and are superseded by this section. Do not use them.

| Code | Raw Image file offset |
|---|---:|
| Probe-like ION heap initialization routine | `0x661244` |
| heap type comparison (`cmp w1, #6`) | `0x66136c` |
| direct BL from that routine to DRM helper | `0x661380` |
| DRM-info SMC helper body | `0x67663c` |
| SMC argument/result wrapper | `0x676098` |
| AArch64 `smc #0` stub | `0x010120` |

Do not map these raw file offsets directly to Kallsyms virtual addresses. A section-aware ELF reconstruction is required to establish that mapping; this audit intentionally reports only byte offsets that can be checked against the decompressed Image.

## 3. Static call path in the target #19 binary

The routine beginning at Image offset `0x661244` has the structure of the Allwinner ION platform heap initialization/probe path: it allocates device state, establishes an ION device, parses the device-tree heap definitions, allocates the heap array, loops over the parsed heap records, and then creates/adds ION heaps. This sequence matches the shape of the comparison implementation in `sunxi_ion_probe()` from the Allwinner 4.9.170 reference tree.

Within that heap loop:

1. At `0x661368`, the routine loads the heap record's type field.
2. At `0x66136c`, it executes `cmp w1, #6`.
3. At `0x661370`, a non-6 type branches around the DRM helper.
4. For type 6, it prepares output pointers to the heap base, heap size and a stack slot for `tee_base`.
5. At `0x661380`, it executes a direct `BL` to `0x67663c`.
6. After the helper returns, control continues into the heap-creation path. There is no conditional branch on the helper's return value before continuing.

The helper at `0x67663c` independently decodes as follows:

- It constructs `x0 = 0xB200000F` (`mov x0, #15`; `movk x0, #0xB200, lsl #16`), the Allwinner `GET_DRM_INFO` fast-call ID.
- It calls the wrapper at `0x676098`.
- The wrapper calls the instruction at `0x010120`, which is literally AArch64 `smc #0`; the following instructions save returned `x0..x3` to the caller's result structure.
- The helper tests returned `a0`. A nonzero status follows the failure branch, references the string `drm config service not available`, and returns `-EINVAL`.
- A zero status copies returned `a1`, `a2`, and `a3` to the three output pointers, then references the format strings `drm_base=0x%x`, `drm_size=0x%x`, and `tee_base=0x%x`, before returning 0.

The strings are present in the same decompressed Image at offsets `0xbf1bbb`, `0xbf1be0`, `0xbf1bef`, and `0xbf1bfe`, respectively. Both success and failure output paths are encoded in the target binary.

## 4. Runtime correlation

The 21:20:49 read-only runtime report independently shows:

- `CONFIG_ION_SUNXI=y`, `CONFIG_ION_OF=y`, `CONFIG_TEE=y`, `CONFIG_OPTEE=y`;
- the `ion-sunxi` platform device bound at `/sys/bus/platform/drivers/ion-sunxi/ion`;
- live FDT `/ion/heap_secure@0` with `compatible = allwinner,secure`, `id = 6`, and `type = ion_secure`;
- `/sys/kernel/debug/ion/heaps/secure` exists, although its current allocation total is zero;
- the captured dmesg has neither the success fields (`drm_base`, `drm_size`, `tee_base`) nor the failure text (`drm config service not available`).

The matching Allwinner 4.9.170 comparison source maps `allwinner,secure` to `ION_HEAP_TYPE_SECURE`, whose enum value is 6. It also shows the same heap-type test and DRM helper call in the `sunxi_ion_probe()` path. References: [sunxi_ion.c](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/staging/android/ion/sunxi/sunxi_ion.c), [uapi/ion.h](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/staging/android/uapi/ion.h), and [OP-TEE core.c](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/tee/optee/core.c). That reference source is comparative evidence, not a claim that the target #19 source tree is byte-identical.

The FDT heap type 6, bound ION device and existing `secure` heap make the type-6 branch applicable to the live configuration. The static call path is therefore highly relevant to the device's ION setup. However, the present capture does not prove the exact runtime execution of that branch, nor the SMC return value.

## 5. What remains unresolved

The target binary contains a direct call from its ION heap initialization path to a helper that issues `0xB200000F` through `SMC #0`. It contains distinct success and failure output paths. But neither output appears in the captured dmesg, so the actual SMC return status and the values of `drm_base`, `drm_size`, and `tee_base` remain **unknown**.

The helper's return value is not checked by the caller before heap creation. Therefore, an empty `secure` heap can exist after either a successful or failed SMC; heap presence and zero allocations cannot distinguish the two.

The missing messages are a runtime logging/execution discrepancy to resolve, not evidence that the SMC succeeded, failed, or returned a zero-sized region. The standard OP-TEE API `api uid mismatch` is a separate UID/revision/capability probe and is not the vendor DRM-info SMC return status.

## 6. Porting relevance and memory safety

This DRM audit is a boot-safety dependency, not the porting deliverable. It does not itself decide the boot architecture or approve a payload address. The port still needs a minimal AArch64 freestanding entry, an owned stack/vector/logging contract, a proven usable-memory policy, and then the H700 HAL.

No physical address is approved by this audit. In particular, `[0x7ec00000,0x7fc00000)` remains an owner-unknown 16 MiB reserved interval and must stay excluded. The monitor, CMA, framebuffer, FDT/initrd, other memblock reservations and top-of-RAM fragments also remain excluded. Do not infer available payload RAM by subtracting memblock reservations from total RAM.

No SMC, ioctl, driver rebind, debugfs control write, boot-media write, or on-device instrumentation was performed.