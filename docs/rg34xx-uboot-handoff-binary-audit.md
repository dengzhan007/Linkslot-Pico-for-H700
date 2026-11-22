# RG34XX 现场 U-Boot AArch32 → AArch64 handoff 二进制审计

## 1. 输入身份

本报告审查的是当前 RG34XX（非 SP）测试卡 TOC1 内实际 U-Boot，不是 BaseOS prepared binary 的偏移移植。

| 项目 | 值 |
|---|---|
| TOC1 SHA-256 | `e6c20e3a54c51b9336c798e3a4fa1ca6c45b28c6f70456e2db9c252aa4ba4604` |
| U-Boot size | 1,048,576 bytes |
| U-Boot SHA-256 | `4d60a22ff13eac00dba89e0ee7df52a49852c0d9d856bec545ce40126191d127` |
| Monitor SHA-256 | `676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31` |
| U-Boot build family | 2018.05 |

TOC1 是报告分支 `rg34xx-inventory-20261122-195053.zip` 中的现场读回；静态扫描在 GitHub Actions 的 `RG34XX vendor U-Boot static scan` 工作流内完成，Capstone 同时按 Thumb-2 和 A32 解码。所有偏移均为从 U-Boot flat binary 起点计算的文件偏移，不是运行时绝对地址。

## 2. 发现：当前 U-Boot 确实调用 vendor A64 handoff SMC

当前 U-Boot 在 Thumb 代码中引用 `0x8000FF04`，并通过 interworking `BLX` 进入 A32 `SMC #0` wrapper。不是仅凭一个孤立 SMC 指令或 BaseOS 模拟 trace 作出的推测。

| U-Boot 文件偏移 | 指令/数据 | 解释 |
|---|---|---|
| `0x1474` | Thumb 函数入口 | 与 U-Boot `boot_jump_linux(bootm_headers_t *images, int flag)` 形态一致 |
| `0x1476` | `mov r5, r0` | 保存 `images` 结构指针 |
| `0x1478` | `ldr r6, [r0, #0x60]` | 读取结构字段，后续作为 kernel entry 使用 |
| `0x14A2` | `ldr r2, [r5, #0x70]` | 读取 FDT 长度字段 |
| `0x14A6` | `ldr r1, [r5, #0x6c]` | 读取 FDT 源地址字段 |
| `0x14A8` | `mov.w r5, #0x44000000` | 设定最终 FDT 目标地址 |
| `0x14AC` | `mov.w r0, #0x44000000` | memcpy 目标地址 |
| `0x14B0` | `bl 0x1080` | 调用内存复制辅助函数 |
| `0x14DC` | `movs r3, #1` | 准备 SMC 第四参数 |
| `0x14DE` | `mov r2, r5` | SMC `r2` = `0x44000000` |
| `0x14E0` | `mov r1, r6` | SMC `r1` = `images.ep` |
| `0x14E2` | `ldr r0, [pc, #0x2c]` | 从 literal pool 读取 Function ID |
| `0x14E4` | `blx 0xB58` | 从 Thumb 切到 A32 SMC wrapper |
| `0x1510` | literal `0x8000FF04` | Allwinner vendor Linux A64 handoff service ID |
| `0xB58` | A32 wrapper 入口 | 保存通用寄存器，随后执行 SMC |
| `0xB5C` | `smc #0` | 真正进入 vendor monitor |

### 2.1 `images.ep`、`images.ft_addr`、`images.ft_len` 的字段对应

上游 U-Boot v2018.05 的 `bootm_headers_t` 在 `image_info_t os` 后定义 `ulong ep`，随后是 `rd_start/rd_end`、`ft_addr` 和 `ft_len`。该字段顺序与现场函数的 `+0x60`、`+0x6c`、`+0x70` 访问吻合。比较源码：[include/image.h at v2018.05](https://github.com/u-boot/u-boot/blob/v2018.05/include/image.h)，入口控制流：[arch/arm/lib/bootm.c at v2018.05](https://github.com/u-boot/u-boot/blob/v2018.05/arch/arm/lib/bootm.c)。

这不是声称现场 vendor tree 与上游源码逐字节相同；关键结论由目标 binary 自身的读字段/调用行为建立，并由同版本结构定义辅助命名。

### 2.2 `0x1080` 是 FDT copy helper

0x14B0 的调用采用标准 `memcpy(dst, src, len)` 寄存器布局：

- `r0 = 0x44000000`（目标）；
- `r1 = images.ft_addr`（源）；
- `r2 = images.ft_len`（长度）。

0x1080 的 Thumb 反汇编先比较源/目的地址，处理相同地址与对齐差异，随后使用 LDM/STM 批量复制和字节尾部处理；其行为与优化版 memcpy 相符。因此，在 SMC 前，U-Boot 将 `images.ft_addr` 指向的 FDT 内容复制到固定目标 `0x44000000`，再把该目标地址传给 monitor。

**已确认最终 FDT 指针的准备方式；尚未确认 `images.ft_addr` 最初来自哪个输入 blob，以及 U-Boot 对该树做过哪些 fixup。**不得据此把 p4 appended DTB、TOC1 DTB 和 live FDT 视为相同文件。

### 2.3 AArch32 与 AArch64 两条出口交叉验证

同一函数在 `0x14DA` 有条件分支进入 `0x14F4` 的传统 A32 入口路径。该路径把 `r2 = r5`、从栈恢复 machine ID 到 `r1`，再 `BLX r6`；由于分支由 `cbz r0` 进入，`r0` 保持为 0。这符合传统 AArch32 Linux 入口 `(r0=0, r1=machine-id, r2=FDT)`，从独立路径证明 `r6` 是 kernel entry、`r5` 是 FDT 地址。

另一条路径将同一个 `r6` 作为 `r1`、FDT 目标地址作为 `r2`，并以 `r3=1` 调用 Function ID `0x8000FF04`。它与 BaseOS H700 native handoff 研究记录的参数顺序 `r0=0x8000ff04, r1=kernel_entry, r2=FDT, r3=1` 精确吻合。现在这个服务调用已直接在 RG34XX 当前 U-Boot binary 中找到。

### 2.4 进入 SMC 前的 cache / CPU 状态清理

handoff 函数在真正选择 SMC/A32 分支前还执行了 `0x14D0 -> 0x0C7C`。现场 binary 的直接控制流为：

1. `0x0C7C` 设置 `r0=3` 并跳入 `0x0C3E`。
2. `0x0C3E` 调用 `0x1600`；该 helper 通过 `MSR CPSR_c` 将 CPSR 的 I/F mask bits (`0xC0`) 置位，因此这一路径会屏蔽 IRQ/FIQ。
3. 由于此处 `r0=3`，`0x0C3E` 对参数最高位的测试为 false，不会进入 `0x0C4A` 的另一条分支。那条未走分支会调用 `0x195A`，在满足 SCTLR.C 条件时 clean D-cache 后清除 SCTLR.M/C；**当前 `r0=3` 路径没有执行该分支**。
4. 实际走的 `0x0C66` 分支先调用 `0x18D6`，清 SCTLR.I（临时关闭 I-cache）；再调用 `0x0B64`，执行 I-cache invalidate (`MCR p15,c7,c5,0`) 和 branch predictor invalidate (`MCR p15,c7,c5,6`)，带 DSB/ISB。
5. `0x0B7E` 通过 `BLX` 进入 A32 helper `0x0A5C`。该 helper读取 CLIDR/CCSIDR cache geometry 并遍历 set/way，执行 `MCR p15,c7,c14,2`，即 D-cache clean by set/way。
6. 再次调用 `0x0B64` 失效 I-cache/branch predictor，最后 `0x18C4` 设置 SCTLR.I 恢复 I-cache。

所以这个具体 handoff 路径的静态指令证据是：IRQ/FIQ mask 被置位；D-cache 执行 set/way clean；I-cache/branch predictor 被失效后重新启用。因为 `r0=3`，不能把这条路径描述成关闭 MMU 或关闭 D-cache。也不能只凭这段 helper 推断调用 SMC 时 SCTLR.M/C 的全部现场值；`0x2BC0` 等其它前置函数、U-Boot relocation 和 monitor 服务自身仍需纳入最终入口状态确认。

### 2.5 SMC 与 AArch32 fallback 的选择条件

`0x14D4` 先检查传入 `flag` 的 `0x200` 位；同版本 U-Boot `BOOTM_STATE_OS_FAKE_GO` 正是 `0x00000200`。该位为 1 时，函数清理栈并返回，不执行实际 OS handoff。对于正常非 fake-go 路径，`0x14D6` 调用 `0x27DC`；该 helper 从全局基址 `0x4A000000` 附近读取 byte offset `0x4E9`，返回其是否等于 1。返回非零时进入 `0x14DC..0x14E4` 的 vendor SMC 路径；返回零时进入 `0x14F4..0x14F8` 的传统 AArch32 fallback（`r0=0`、`r1=machine-id`、`r2=FDT`、`BLX r6`）。目前只将 `+0x4E9 == 1` 解释为二进制中的路径选择条件；尚无证据给这个字段命名为某个特定设备能力或固件状态。

上述 `BOOTM_STATE_OS_FAKE_GO` 数值对照来自 [U-Boot v2018.05 include/image.h](https://github.com/u-boot/u-boot/blob/v2018.05/include/image.h)，其中 `BOOTM_STATE_OS_FAKE_GO=0x00000200`。

## 3. 与当前 p4 Android boot image 对应

用户上传的当前 p4 boot partition SHA-256 为 `41cfaa8f9ade4a5e05a89188abb7db154243e38b8e19c53b730f0a45ff325516`。其 Android v2 header 字段 `kernel_addr=0x40080000`，`ramdisk_addr=0x42000000`，`dtb_addr=0x44000000`。压缩 kernel 解压后为 `ARM\x64` AArch64 Image，`text_offset=0x80000`，版本为 Linux 4.9.170 `#19`，与现场 runtime 完全匹配。

当前 U-Boot 的压缩选择器位置 `0xF698` 可见 `40f20123`，与 BaseOS 对照 `kernel_gzip.py` 中将 vendor Android handler 从 raw Image selector 改为 gzip selector 的补丁指令相同。现场 p4 kernel component 当前确实是 gzip。现场 U-Boot SHA 与 BaseOS pinned U-Boot 不同，所以只把该指令作为现场 binary 中实际观察到的压缩路径特征，不套用 BaseOS 其他指令偏移。

因此当前最强的静态链路是：

```text
boot_normal = sunxi_flash read 0x45000000 boot; bootm 0x45000000
  -> Android v2 p4 kernel component (gzip)
  -> U-Boot 得到 images.ep（预期与 p4 kernel_addr 0x40080000 对应）
  -> U-Boot 得到 images.ft_addr / images.ft_len
  -> memcpy(images.ft_addr, 0x44000000, images.ft_len)
  -> SMC(0x8000FF04, images.ep, 0x44000000, 1)
  -> vendor monitor 进入 AArch64 kernel/payload entry
```

`images.ep` 从 `bootm_headers` 的字段位置及后续 A32 fallback 直接调用关系确定为 kernel entry 字段；当前 p4 header 的 kernel address 是 `0x40080000`，与 Linux Image `text_offset=0x80000` 相符。因此 `0x40080000` 是 stock kernel-slot 路线的强候选入口地址，不是从空闲 RAM 推导出来的猜测地址。实际 decompressor destination / `images.ep` 值仍需通过目标 U-Boot load path 或入口 trace 做最后验证。

### 3.1 Android kernel compression selector 与真实 load/entry 地址

现场 U-Boot 的 Android kernel handler 在文件偏移 `0xF698` 有如下 Thumb-2 序列：

```asm
0xF698  movw    r3, #0x0201
0xF69E  strh.w  r3, [r4, #0x5c]
0xF6A2  movs    r3, #5
0xF6A4  strb.w  r3, [r4, #0x5e]
```

这组字节与 BaseOS 固定固件工具 `tools/kernel_gzip.py` 的 selector patch 完全相同：它把原 selector bytes `4f f4 00 73` 替换为 `40 f2 01 23`；脚本注释明确说明原 vendor Android handler 强制 raw Image，patch 将 compression selector 改为 gzip。当前 RG34XX 现场 U-Boot 在 `0xF698` 已是 patched bytes `40 f2 01 23`。结合 U-Boot v2018.05 `include/image.h` 的 enum：`IH_COMP_GZIP=1`、`IH_TYPE_KERNEL=2`、`IH_OS_LINUX=5`，`0x0201` 加随后 byte `5` 对应 GZIP/KERNEL/LINUX。参考：[BaseOS kernel_gzip.py](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/tools/kernel_gzip.py)、[U-Boot image.h](https://github.com/u-boot/u-boot/blob/v2018.05/include/image.h)。

在 U-Boot v2018.05 `common/bootm.c` 的 Android image 分支，`images.os.load = android_image_get_kload(os_hdr)`，并 `images.ep = images.os.load`；`android_image_get_kload()` 返回 Android header 的 `kernel_addr`（仅在 Android 默认占位地址时才改用 kernel 数据位置）。`bootm_load_os()` 再以 `images.os.load` 作为解压目标；`bootm_decomp_image()` 对 `IH_COMP_GZIP` 调用 `gunzip(load_buf, unc_len, image_buf, image_len)`。参考：[common/bootm.c](https://github.com/u-boot/u-boot/blob/v2018.05/common/bootm.c)、[common/image-android.c](https://github.com/u-boot/u-boot/blob/v2018.05/common/image-android.c)。

现场 p4 Android v2 header 的 `kernel_addr=0x40080000`，与该 `images.os.load → images.ep` 路径相符；现场 handoff 函数又将 `images.ep` 读取到 `r6`，随后传入 `0x8000FF04`。因此现在可把 `0x40080000` 定为**原厂 kernel component 的解压目标与 handoff entry**，而不只是地址候选。

解压 buffer 的精确上限（目标编译时 `CONFIG_SYS_BOOTM_LEN`）尚未从当前 U-Boot config/二进制恢复。已知现场原厂 gzip kernel 成功解压为 17,461,256 bytes，因此当前 stock loader 的实际输出能力至少覆盖该长度；还不能据此宣称上限恰好为某个 32 MiB 常量。

主机侧 payload builder 现保守限制新 Image 文件长度不得超过这份已成功启动原厂 kernel 的 17,461,256-byte 解压长度，且 Image header `image_size` 不超过原厂值 `0x1138000`。只有确认 `CONFIG_SYS_BOOTM_LEN` 后，才可考虑放宽这两个保守限制。

## 4. 对裸机移植路线的影响

当前证据支持**优先保留 Boot ROM、Boot0、TOC1、vendor monitor、AArch32 U-Boot、env 和 GPT，只替换 p4 Android boot image 的 kernel component**。第一份 payload 只需要满足现有 U-Boot kernel-slot / gzip / AArch64 Image 入口契约；暂时没有理由为第一份入口诊断 payload 先重写 Boot0 或引入 A64 SPL。

这仍不是可刷写方案。还需要：

1. 已通过目标 U-Boot 的 selector bytes、Android header、`bootm` 数据流确定 gzip 解压目标和 `images.ep` 为 `0x40080000`；仍需恢复精确 `CONFIG_SYS_BOOTM_LEN`，当前 builder 先把新 payload 限制在已成功启动的原 kernel 解压长度内。
2. 确认 `images.ft_addr` 的原始 FDT 来源，以及 TOC1 DTB、p4 appended DTB、live FDT 的具体 fixup 顺序。p4 appended DTB 声明 512 MiB，live FDT 声明 1 GiB；不得用 p4 里的旧树替代 live 树。
3. 用当前相同 hash 的 monitor binary 检查 `0x8000FF04` 服务的实际 ERET 状态、目标 EL/SPSR、cache/MMU/TLB、DAIF 和 x0..x3。BaseOS 的 monitor verifier 以实际 vendor monitor 指令完成了仿真 ERET：`ELR=0x40080000`、`SPSR=0x3c5`、`x0=0x44000000`、`x1..x3=0`；该结果是 emulation/model evidence，不是 RG34XX 现场硬件寄存器捕获，也不能替代当前 U-Boot 最终 CPU 状态核对。
4. 主机侧构建一个带同样 ARM64 Image header、`text_offset=0x80000`、gzip 压缩的入口诊断 payload；仅重建 Android v2 kernel component 和必要的 size/image ID，保留 ramdisk、second、recovery_dtbo、appended DTB、cmdline 与 p4 未使用尾部。
5. 完成入口模拟和恢复评审后，才考虑独立测试卡。没有用户明确批准前，不改 SD/Boot0/TOC1/U-Boot，不试刷 payload。

## 5. 安全边界仍然不变

当前还没有被批准的通用堆或 allocator 区间。特别是 `[0x7ec00000,0x7fc00000)` 16 MiB 未知保留区继续排除；monitor `[0x48000000,0x49000000)`、CMA `[0x77c00000,0x7bc00000)`、framebuffer `[0x7bf24800,0x7c076000)`、kernel/FDT/initrd 和其他 memblock reserved entries 全部继续排除。`0x40080000` 仅是原厂启动链提供的 kernel slot/入口候选，不代表整个 RAM map 已经开放。

没有发起 SMC、ION ioctl、driver rebind、debugfs 写入、SD 写入、Boot0/TOC1/U-Boot 写入或任何设备端代码执行。