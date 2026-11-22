# RG34XX 原厂 boot.img 内核槽位裸机 payload 路径评估

## 1. 目的与项目定位

本文件评估一个优先于替换 Boot0/U-Boot 的低改动启动路径：保留 RG34XX 当前 Boot ROM、Boot0、TOC1、vendor monitor、AArch32 U-Boot、环境分区和 GPT，只研究把 p4 Android boot image 的 kernel component 替换成一个符合 AArch64 Linux Image 容器头部的 freestanding payload。

这不是 GammaOS Pico 前端移植，也不是最终启动方案决定。产品目标仍是 GammaOS Pico/LK 裸机运行架构 + Linkslot 唯一前端，首期 GB/GBC/GBA。当前没有可刷写固件。本阶段只做静态分析、主机侧容器验证和可回滚的实验设计；未获明确批准前不写测试 SD。

**选择该方向的原因：** 当前 U-Boot 已经能读取 p4、解压 gzip kernel 并把 AArch64 Image 交给后续启动路径。若能够证明它会按可复用的 A64 入口契约交接一个 freestanding Image，我们就不必为了第一份诊断 payload 先替换 Boot0、引入 mainline SPL 或实现自有 SD/DRAM 初始化。

## 2. 当前设备实际 boot 分区证据

用户提供的完整 p4 boot 分区只读镜像：

- 大小：67,108,864 bytes（64 MiB）。
- SHA-256：`41cfaa8f9ade4a5e05a89188abb7db154243e38b8e19c53b730f0a45ff325516`。
- Android header：v2，`ANDROID!`，page size 2048，header size 1660，image ID 校验通过。
- boot image 的 `kernel_addr=0x40080000`、`ramdisk_addr=0x42000000`、`dtb_addr=0x44000000`。
- kernel component 位于文件偏移 `0x800`，gzip 压缩长度 7,018,071 bytes；解压得到 17,461,256-byte AArch64 Image，SHA-256 `39cf8c3eb9a75ff0eda51b4074cd65860c5aa5386093bcaf69339819421d84b7`。
- 解压后 Image header magic 为 `ARM\x64`，`text_offset=0x80000`，`image_size=0x1138000`；版本串精确匹配现场 `Linux 4.9.170 #19`（2026-05-26 13:58:08 CST）。
- Android boot image 还包含 512-byte ramdisk、空 second/recovery_dtbo 和 137,770-byte appended DTB。五个组件及 SHA-1 image ID 按 Android v2 规则验证通过。

主机工具 [inspect_android_bootimg.py](../tools/inspect_android_bootimg.py) 可重复检查 header、component offsets、长度、SHA-256、image ID、gzip kernel、AArch64 Image header 和 appended FDT；它只读输入，不访问块设备。

## 3. 三份 DTB 必须区分

当前已读回的三份树不同，不能直接互换：

| 来源 | 大小 | SHA-256 | 观察到的内容 |
|---|---:|---|---|
| p4 Android boot.img appended DTB | 137,770 B | `e2f3ec2aa9ad78ce5ab553dc9f75fc04070fdb7b6cc405ba3f3af79b9fd9d3c0` | `memory@40000000` 声明 512 MiB，chosen bootargs 为旧模板 |
| TOC1 DTB | 144,384 B | `c28c87380ffab00148bf4dc457e70b8c6c576470dcfbc53df64addddf89603a1` | 与当前设备 H700/`rg34xx_v1` 固件配置对应；BaseOS 对照工具会修改其中的 debug_mode 与五个 USB host controller status |
| Linux 实际收到的 live FDT | 143,936 B | `ccaf4cd9fe973c17e376f773876486bd479f46e85e18ac25ce3dc33920c4c57f` | 声明 1 GiB RAM；bootargs 带动态 root/partitions/cma/serial 等字段 |

BaseOS-MultiFrontend 的 `tools/kernel_gzip.py` 明确按 Android v2 五组件顺序解析 p4 image，并保留其中原有 DTB；同时它单独解析、修改 TOC1 `dtb`，注释说明 U-Boot 从 package DTB 读取早期启动配置。其 boot chain 文档也记录 U-Boot 从 p4 boot image 取 kernel/ramdisk、从环境与 GPT 构造 bootargs。该设计是“TOC1 DTB 是 U-Boot 工作树、boot.img appended DTB 不一定是最终 FDT”的强线索，但并非对本机当前 U-Boot 二进制的直接执行证明。

因此：任何替换 boot image 的构建器必须按原样保留 appended DTB、ramdisk、header cmdline 和未知组件；payload 不应自行构造一个新的 FDT，也不能假定 Linux 原来接收的是 appended DTB。先静态确定当前 U-Boot 实际使用的 FDT 来源。

## 4. 低改动候选启动流程

```text
Boot ROM
  -> 原厂 Boot0（不变）
  -> 原厂 TOC1 / vendor monitor（不变）
  -> 原厂 AArch32 U-Boot（不变）
  -> sunxi_flash read 45000000 boot
  -> bootm 45000000：验证 Android v2、解压 gzip kernel 到原 kernel slot
  -> 原厂 monitor / A32-to-A64 handoff（实际 ABI 待静态确认）
  -> AArch64 freestanding payload
```

当前现场环境为 `boot_normal=sunxi_flash read 45000000 boot;bootm 45000000`。更重要的是，已经从当前 RG34XX TOC1 内的 U-Boot binary 找到 `0x8000FF04` → `SMC #0` 的实际调用点及其寄存器准备路径，详见[现场 U-Boot handoff 二进制审计](rg34xx-uboot-handoff-binary-audit.md)。这比 BaseOS prepared U-Boot 的 Unicorn trace 更直接；但 monitor 的最终 ERET 状态、`images.ep` 的实际数值与 FDT 原始来源仍待进一步核验。

该候选的核心 handoff 调用点和参数布局已在现场 U-Boot binary 中确认；剩余门槛为：

1. 静态追踪当前 U-Boot 的 Android gzip 解压路径，确认输出目标、输出长度限制，并将 `images.ep` 实值与 p4 header `kernel_addr=0x40080000` 对应起来。
2. 对与现场相同 hash 的 vendor monitor 检查 `0x8000FF04` 服务的 ERET 行为：CurrentEL、SPSR_EL3/ELR_EL3、x0..x3、DAIF、中断、MMU/cache/TLB 和 FDT 指针。
3. 确认 `images.ft_addr` 的原始 FDT 来源与 fixup 顺序，解释 p4 appended DTB（512 MiB）、TOC1 DTB 和 live FDT（1 GiB）之间的关系。
4. 在上述静态门槛通过后，才生成一个不调用 Linux、不访问未知 RAM 的 AArch64 entry probe；模拟结果不能替代 RG34XX 非 SP 实机验收。


### 4.1 现场 U-Boot 的 vendor A64 handoff 已直接定位

现场 TOC1 的 U-Boot SHA-256 为 `4d60a22ff13eac00dba89e0ee7df52a49852c0d9d856bec545ce40126191d127`。对这个实际 binary 的 Thumb/A32 混合代码反汇编，已找到 `0x8000FF04` → `SMC #0` 的真实调用链，而不是继续依赖 BaseOS prepared U-Boot 的 Unicorn trace。完整字段和偏移见[现场 U-Boot handoff 二进制审计](rg34xx-uboot-handoff-binary-audit.md)。

关键结论：

- `boot_jump_linux` 形态的函数在 `0x1474` 收到 `images` 结构指针，`r6 = [images + 0x60]`，后续作为 kernel entry 使用。
- `images.ft_addr` 与 `images.ft_len` 分别从 `+0x6c`、`+0x70` 读取。
- U-Boot 调用 `memcpy(0x44000000, images.ft_addr, images.ft_len)`，将当前 FDT 复制到 `0x44000000`。
- 然后设置 `r0=0x8000FF04`、`r1=images.ep`、`r2=0x44000000`、`r3=1`，经 `BLX` 调用 A32 SMC wrapper 并执行 `SMC #0`。
- 另一个 AArch32 fallback 分支通过 `BLX r6` 进入 kernel，且传入 `(r0=0, r1=machine-id, r2=FDT)`；这独立确认 `r6` 是 kernel entry、`r5` 保存 FDT 目标地址。
- U-Boot 2018.05 `bootm_headers_t` 的 `ep`、`ft_addr`、`ft_len` 字段顺序与目标二进制 `+0x60/+0x6c/+0x70` 的访问吻合。现场 `ep` 与 Android header `kernel_addr=0x40080000` 对应是强证据；还需核对实际 decompressor/load path 来最终确认运行时 entry。

所以当前已找到原厂启动链使用的真实 A32→A64 monitor handoff 服务和参数布局。保留 Boot0/TOC1/monitor/U-Boot、只替换 p4 kernel component 是当前首选移植路径，不再与重写 SPL 方案并列为同等优先级。

尚未完成的两项不是 handoff 服务号本身，而是：实际 `images.ep`/kernel decompressor 的最终落点核验；`images.ft_addr` 最初来自哪份 DTB，以及它如何从旧的 512 MiB appended tree 变成 Linux 收到的 1 GiB live FDT。

### 4.2 已有主机侧 kernel-slot 构建器（不刷写）



项目现在包含 `tools/build_rg34xx_bootimg_payload.py`。它只在主机上读取一个原始 p4 分区镜像和一个未压缩的 AArch64 Image，并生成**另一个新文件**作为离线候选分区。当前实现强制检查：



- 输入是 Android boot image v2，page size 2048，原 kernel component 为 gzip；

- 当前 RG34XX 头部地址为 kernel `0x40080000`、ramdisk `0x42000000`、FDT `0x44000000`；

- 新 Image 必须有 `ARM\x64` magic、有效 AArch64 branch header、`text_offset=0x80000`、非零 `image_size`，且 `image_size` 不与 ramdisk 槽重叠；

- 输出 kernel component 使用 gzip；只更新 Android header 的 kernel size 和 SHA-1 image ID；

- ramdisk、second、recovery_dtbo、appended DTB 必须逐字节不变；新组件不得越过原始 Android component 区域；原 p4 未使用尾部必须逐字节保留；

- 不能覆盖输入文件，也不能覆盖已存在的输出路径；候选 p4 分区长度必须与原分区相同。



对应测试 `tools/test_build_rg34xx_bootimg_payload.py` 使用合成 Android v2 image 验证 round-trip、non-kernel component hash、p4 tail 保留、错误 Image header、错误 text_offset、slot 超限与组件区越界。该工具当前没有生成真实 payload，也没有被用于用户的设备镜像；任何输出都只是 host-side candidate，不是可刷写发行包。

## 5. 第一份 payload 的边界

当前 U-Boot 已确认存在 vendor SMC handoff 调用点和参数准备路径；但 monitor 的 ERET 状态、现场 `images.ep` 实值及 FDT 来源仍待核验。在这些最后门槛完成前，不生成可刷写镜像。之后第一份 payload 只做入口诊断，不实现 allocator、文件系统、HAL 或 Linkslot：

- 采用带 AArch64 Image header 的 freestanding binary，header 中 code0 分支跳到自身 `_start`；保留 `ARM\x64`、text_offset 和 image_size 字段的一致性。
- 先使用 stock kernel slot `0x40080000`，输出尺寸严格受 host-built image_size 限制；代码、静态数据、BSS、诊断记录和临时栈全部由 payload linker script 在自身镜像范围内分配。
- 不将 `[0x7ec00000,0x7fc00000)`、CMA `[0x77c00000,0x7bc00000)`、monitor `[0x48000000,0x49000000)`、framebuffer `[0x7bf24800,0x7c076000)` 或其它 unknown reserved entries 用作堆/栈。
- 入口最先捕获 x0..x3、SP、CurrentEL、DAIF、SCTLR_EL1、TTBR0/1_EL1、MAIR_EL1、VBAR_EL1 等可读状态；x0 是否为 FDT 指针需要检查 magic/totalsize 后才确认。
- 首轮不启用 MMU、不触发 SMC、不重新初始化 DRAM、不改变电源/显示时序；日志优先使用经确认的早期 UART。若 UART/时钟/pinctrl 继承状态不确定，使用无外设访问的内存记录/停机路径，不编造输出。
- 不保留或调用 Linux 入口代码、Linux exception vectors 或 Linux 驱动；这是真正的 freestanding entry probe。

这个诊断阶段只需要在已被原厂 bootloader 读取的 kernel image 槽位内占用有限、显式链接的空间；它**不等于**已建立系统 RAM allocator。要进入 HAL、SD DMA、图形 framebuffer 双缓冲或多核心运行，仍需独立证明可用内存 ownership。

## 6. 实现与验收顺序

1. **离线镜像检查（已完成第一轮）：** 验证 Android v2 header、组件布局、image ID、kernel gzip、AArch64 Image header、appended DTB；工具与合成测试不得涉及设备。
2. **当前 U-Boot handoff 静态审计（核心调用已找到）：** 现场 U-Boot 的 `0x8000FF04` → `SMC #0` 路径、`images.ep` 与 FDT 参数准备已定位；仍需核实 gzip decompressor 输出地址、FDT 原始来源及 monitor ERET 状态。
3. **主机侧 payload builder（第一版已完成）：** `tools/build_rg34xx_bootimg_payload.py` 只生成独立候选分区文件，验证 Image header 和 kernel slot 边界，保持非 kernel 组件及 p4 尾部；通过主机合成测试。还没有真实 AArch64 payload。
4. **入口 probe 与模拟：** 静态门槛通过后，构建最小 AArch64 Image 诊断程序，检查入口参数、EL、异常屏蔽、栈和 FDT。模拟结果不能替代 RG34XX 非 SP 实机验收。
5. **硬件试验评审：** 只有前四步通过后，才单独制作测试卡镜像；原始整卡/Boot0/TOC1/U-Boot 必须有经校验备份，试验不覆盖用户日常卡。此项需要用户明确批准后才执行。
6. **后续移植：** payload 能稳定冷启动并记录入口状态后，再做早期日志、timer/GPIO/input、display/backlight、SD/filesystem、audio、PMIC full poweroff；之后才是 core ABI 和 Linkslot 大版本同步。

## 7. 失败时的回退路线

若当前 U-Boot/monitor 只能接受 Linux 特定入口，或不能安全地交接任意 freestanding A64 Image，则回到项目 `docs/architecture.md` 已保留的备选路线：自有 A32 BL33 handoff adapter，必要时再评估 A64 SPL + vendor monitor。BaseOS-MultiFrontend 的 `native/entry32.S`、`spl/README.md` 和 monitor verifier 是研究参考，但其 RG34XX SP/RG40XXV 结果不等于 RG34XX 非 SP 实机通过。

本评估不改变“不刷 Boot0、不改 TOC1、不写 U-Boot、不试 payload 地址”的当前安全约束。