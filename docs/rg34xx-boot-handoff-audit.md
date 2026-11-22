# RG34XX 启动交接审计

## 1. 范围与结论

本报告仅针对 **Anbernic RG34XX（非 SP）/ Allwinner H700 / Cortex-A53 / 1 GiB LPDDR4**。RG34XX SP 是独立机型，不可将其 2 GiB 内存、`rg34xxsp_v1` 面板或固件指纹当作本机数据。

启动链架构与交接 ABI 的基础审计来自公开仓库和已提交研究资料。2026-11-22 已补充 RG34XX 非 SP 实机只读 inventory；本项目没有改写 SD 卡、Boot0、TOC1、U-Boot 或 monitor。

**结论：现有 AArch32 U-Boot 不能被视为可直接启动任意 AArch64 裸机程序的入口。** BaseOS-MultiFrontend 已经研究出一条更具体的候选架构：由 AArch64 SPL 负责 SD、DRAM 和加载；保留厂商 monitor；让 monitor 进入一个 RAM 中的 AArch32 SVC BL33 适配器；适配器再通过厂商 SMC 交接到 AArch64。它为 Linkslot Pico 提供了可研究的桥接方式，但其最终 SMC 是按厂商 Linux 启动 ABI 逆向验证的，尚未证明它是通用裸机 payload ABI，更没有在 RG34XX 非 SP 上完成实机验收。

## 2. 证据等级

本文区分三类证据，不能互相替代：

1. **RG34XX 型号固件静态证据**：来自 BaseOS-MultiFrontend 固定的 `rg34xx` prepared 输入、兼容性清单、U-Boot/Boot0 静态分析和模拟器执行。
2. **其他 H700 实验机的实机证据**：BaseOS 原生 SPL 项目记录了 RG34XX SP（2 GiB LPDDR4）及 RG40XX V（1 GiB LPDDR3）实验。它能证明某些共享 H700 monitor/启动机制可工作，但不能代替 RG34XX 非 SP 的 1 GiB LPDDR4 验收。
3. **本项目 RG34XX 现场证据**：已读回本机 Boot0/TOC1/U-Boot/monitor、完整 env、live FDT、GPT、内存映射和分区信息；已验证 p3 内 128 KiB U-Boot env CRC 和完整 p3 prepared hash，亦已从 boot_fb0 与 disp_reserve 交叉确认 framebuffer 物理范围。Boot0 本次实际选择（LBA 16 是高置信候选但未直接观测）、DRAM training 的完整语义、monitor/secure/DMA 剩余内存约束、入口寄存器状态及最小 AArch64 payload 冷启动仍待验证。

BaseOS 参考源本次按 `main` 快照审查，commit：[`cbe253a43f46cf551cc9a6c79a84015bde3686cf`](https://github.com/dengzhan007/BaseOS-MultiFrontend/commit/cbe253a43f46cf551cc9a6c79a84015bde3686cf)。

## 3. RG34XX 原厂启动链

已知启动结构：

```text
Allwinner Boot ROM
  └─ SD / eGON.BT0 Boot0（镜像长度按头部声明，现场读回覆盖 64/128 KiB）
       ├─ 选择 DRAM 参数并完成 training
       └─ 读取 TOC1 启动包
            ├─ vendor monitor（BL31 路径）
            ├─ AArch32 vendor U-Boot（2018.05 系）
            ├─ DTB / DTBO 等板级数据
            └─ 后续由 U-Boot 加载 boot 分区中的 Android boot image
                 └─ 通过 vendor monitor 进入 AArch64 Linux
```

Boot0 会根据候选参数训练 DRAM，并把选择结果传给后续阶段。不能在 Linkslot 主程序中重新初始化 DRAM，也不能把某一台机器训练出来的参数写死给所有同型号设备。

BaseOS 对 RG34XX 的固件记录确认其原厂 U-Boot 是 AArch32；BaseOS 的 U-Boot trace 在模拟器中到达 Linux 跳转点 `0x40080000`，FDT 研究地址为 `0x44000000`。这些地址是 Linux 启动流程中的位置，不是已验证的 Linkslot payload 地址。该模拟器 trace 不是 RG34XX 实机启动验证，也不能单独证明 monitor 的完整现场状态。

参考：
- [BaseOS 启动链与分区说明](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/docs/00-boot-chain-and-partitions.md)
- [H700 Boot0/DRAM/面板差异调查](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/experiments/h700-boot-speed/variant-survey/README.md)
- [U-Boot 模拟 trace 边界](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/experiments/h700-boot-speed/evidence/uboot-trace-notes.md)

## 4. BaseOS 原生 SPL 研究提供的候选交接结构

BaseOS 的深入研究没有采用“让 AArch32 U-Boot 直接跳 AArch64”这种未经验证的捷径，而是形成了两段式交接：

```text
Boot ROM
  └─ mainline H700 AArch64 SPL
       ├─ SD / PMIC / 自动 DRAM 选择
       ├─ 加载原厂 vendor monitor、DTB/DTBO
       └─ 加载 RAM-only AArch32 BL33 适配器
            └─ vendor monitor 以 AArch32 SVC 状态进入 BL33
                 ├─ 建立自己的栈、BSS、页表与运行环境
                 ├─ 解析/准备镜像和最终 FDT
                 └─ 调用 vendor kernel SMC
                      └─ vendor monitor ERET 到 AArch64
```

重要交接证据：

- BaseOS SPL 的 monitor 验证器执行厂商 monitor 的真实指令，并检查其 BL33 返回地址为 `0x4a000000`、SPSR 为 `0x1d3`。这对应 AArch32 SVC，不是 AArch64 BL33。
- monitor 的实验加载地址为 `0x48000000`；Boot0/TOC1 交接结构中，monitor 还需要 DRAM 参数、board bytes 和 DTB 指针等数据。
- A32 loader 的最终 Linux SMC 参数为 `r0=0x8000ff04`、`r1=0x40080000`、`r2=0x44000000`、`r3=1`。实际 monitor 返回到 AArch64 Linux 时，实验记录检查到入口 `0x40080000`、SPSR `0x3c5`、`x0=0x44000000`，`x1..x3=0`。
- loader 在调用前会处理 D-cache clean、MMU/cache disable、TLB/指令缓存维护和中断屏蔽；这些属于入口契约的一部分，不能只复制 SMC 指令。
- BaseOS 研究还检查了 monitor 地址、SPL/BL33 的内存占用、DRAM training 参数、FDT 的 `/dram`、`/memory`、bootargs 和内存预留。

**关键限制：** `0x8000ff04` 是厂商 Linux 启动交接服务的调用 ID/约定，不应直接当作公开的通用 `jump_to_aarch64(payload, args)` API。即使将 `r1` 改为自定义 AArch64 入口，也需要先确认 monitor 对该服务的目标地址、异常级别、寄存器、SPSR、FDT/参数和安全状态要求，并用一个不依赖 Linux 的最小 AArch64 payload 做实机验证。

参考：
- [BaseOS 原生 SPL 设计与交接 ABI](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/experiments/h700-boot-speed/DESIGN.md)
- [BaseOS 当前原生启动实验状态](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/experiments/h700-boot-speed/HANDOFF.md)
- [SPL monitor / BL33 说明](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/experiments/h700-boot-speed/spl/README.md)
- [真实 monitor 指令验证器](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/experiments/h700-boot-speed/spl/verify-monitor.py)
- [A32 loader 入口与 Linux SMC 汇编](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/experiments/h700-boot-speed/native/entry32.S)

## 5. 地址与内存布局：仅作研究线索

BaseOS 原生 SPL 研究中出现过以下实验地址：

| 用途 | 研究地址 |
|---|---:|
| Linux Image | `0x40080000` |
| initrd | `0x42000000` |
| 最终 FDT | `0x44000000` |
| 暂存 Android image | `0x45000000` |
| vendor monitor | `0x48000000` |
| AArch32 BL33 | `0x4a000000` |
| DTBO | `0x4a100000` |
| monitor DTB | `0x4a200000` |
| 启动记录 | `0x4a300000` |
| EL3 临时页表 | `0x4a400000` |

以上是 BaseOS 某代实验构建使用的布局，不是 RG34XX 非 SP 的内存安全证明。真正采用前，必须结合目标机 live FDT、reserved-memory、monitor 实际访问、framebuffer、SPL 自身使用区和 1 GiB RAM 上界重新计算并进行冲突检查。尤其不能仅凭地址落在 DRAM 范围内就认定它未被 secure world、显示引擎或其他 DMA 使用。


### 5.1 RG34XX 非 SP 实机 memblock reserved 读回（2026-11-22）

第四轮现场文件 [debugfs-memory-reservations.txt](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/debugfs-memory-reservations.txt) 已直接提供 memblock 地址表。`memblock.memory` 为 `[0x40000000,0x80000000)`（1 GiB）；`memblock.reserved` 共 46 条。较大的区间为：

| 区间（半开） | 长度 | 审计结论 |
|---|---:|---|
| `[0x40080000,0x411b5000)` | 17,620 KiB | 与 Linux kernel code/data 的物理范围相符 |
| `[0x411b6000,0x411b8000)` | 8 KiB | 邻近 kernel 的小型保留项，来源未明 |
| `[0x42000000,0x42001000)` | 4 KiB | 与 BaseOS 实验 initrd 起始地址相同，但不能仅凭此认定为 initrd |
| `[0x44000000,0x44023240)` | 143,936 bytes | 与 BaseOS 最终 FDT 起始地址相同，疑似 FDT/启动数据 |
| `[0x48000000,0x49000000)` | 16 MiB | 与 BaseOS vendor monitor 的研究装载地址及 16 MiB 区域吻合；必须视为 monitor 保留区 |
| `[0x77c00000,0x7bc00000)` | 64 MiB | 与 `CmaTotal=65536 kB` 数量完全相等，为高置信 CMA 物理候选区 |
| `[0x7ec00000,0x7fc00000)` | 16 MiB | 用途未知，必须排除 |
| `[0x7fd4a000,0x7fd4b000)` | 4 KiB | 用途未知，必须排除 |
| `[0x7fd65000,0x7ff49000)` | 1,982,464 bytes | 用途未知，必须排除 |

索引 9–45 另有 37 条离散小型保留项，位于 RAM 顶部 `0x7ff49a00` 至 `0x7ffffffe`；它们不可合并为空闲连续区。memblock 文件未提供这些区域的名称/owner，所以未知范围一律按保留处理。

CMA 判断仍属于高置信推断而非直接标注：设备 bootargs 有 `cma=64M`，meminfo 显示 `CmaTotal=65536 kB`，memblock 又恰好有唯一 64 MiB 连续 reserved 段。Linux CMA 的常规初始化会从早期 memblock/bootmem allocator 保留一段连续区域，机制与此吻合（参考 [Linux DMA contiguous CMA reserve path](https://cos.googlesource.com/third_party/kernel/%2B/c570b82c5e6ac78be35b4e72594c0f1b1888fce1/drivers/base/dma-contiguous.c)）。但目标内核未导出 CMA debugfs 的 `base_pfn/count`，reserved 项无名称，因此先将 `[0x77c00000,0x7bc00000)` 标作 CMA candidate，不把推断写成直接观测。

live FDT 的 framebuffer `[0x7bf24800,0x7c076000)` 不在这 46 条 memblock reserved entries 中；但 `boot_fb0` 与 `disp_reserve` 已独立证明其为显示缓冲区，仍必须排除。它与 CMA 候选区不重叠，起点比 CMA 候选区末端高 `0x324800` 字节。memblock 表也不能代表 Linux 运行阶段所有驱动/DMA 动态分配，因此不能把 `memblock.memory - memblock.reserved` 作为裸机可用 RAM 清单。

**对启动设计的结论：** BaseOS monitor 区域现在获得目标机 memblock 现场范围支持；CMA 区域获得高置信物理位置候选；但 `0x7ec00000–0x7fc00000`、top-of-RAM 碎片、secure world、DMA/display 与 loader 的完整 owner 仍未收敛。当前仍没有批准的 Linkslot payload 地址或 entry address。

## 6. RG34XX 非 SP 的 DRAM 与固件输入差异

BaseOS 的 `compatibility.json` 为 `rg34xx` 单独记录了以下 Boot0 候选：

- 主候选：offset `0x38`，LPDDR4，672 MHz。
- 备用：offset `0x9f4`，LPDDR3，672 MHz。
- 通用备用：offset `0xa74`，LPDDR4，672 MHz。

RG34XX profile 是 1 GiB LPDDR4。不要因 RG34XX SP 实验也使用 LPDDR4，就认为两者 training bank、rank/width、size、monitor、面板和启动包相同。特别是通用 LPDDR4 bank 的零 tuning words 在 BaseOS 的其他 H700 实验中暴露过 mainline training/size-probe 风险；任何 SPL 复用都必须对 RG34XX 的实际 bank 逐项验证。

准备镜像中的 Boot0 SHA、TOC1 SHA、U-Boot SHA 和 monitor SHA 是构建输入指纹，不代表当前掌机 SD 卡上的现场读回值。Boot0 在启动时会更新训练相关头部数据，因此应采用 BaseOS 已记录的差异规则核验其可执行主体和允许变化区域，不能简单要求每次启动后的整段 SHA 完全相同。

## 7. 实机覆盖与当前状态

BaseOS 的 native SPL / monitor / A32 loader 是当前最有价值的技术参考。其公开实验记录覆盖其他 H700 实验设备，包括 RG34XX SP 与 RG40XX V，并报告了原生链启动、恢复、显示及休眠相关测试。**本次可访问的记录没有证明 RG34XX（非 SP）的 1 GiB LPDDR4 机型已经完成同等的 native SPL 冷启动和完整 Linkslot payload 验收。**

因此，本项目当前可以确认的是：

- H700 的 AArch32 BL33 → vendor monitor → AArch64 Linux 交接机制已被深入分析。
- 存在一条经过其他 H700 实验机验证的 A64 SPL + vendor monitor + A32 BL33 方案。
- 目标 RG34XX 固件的 Boot0 DRAM 候选、monitor 指纹、屏幕驱动名称等已有静态资料。

当前仍不能宣称：

- RG34XX 非 SP 的 live Boot0/TOC1/U-Boot/monitor 与准备镜像完全一致；
- 该机实测的 payload 入口 EL、寄存器、栈、cache/MMU、异常向量和可用 DRAM 区域已确认；
- 厂商 Linux SMC 已被证明可无 Linux 地启动任意 AArch64 裸机程序；
- Linkslot Pico 已在 RG34XX 上启动。

## 8. 对 Linkslot Pico 的实施建议

1. RG34XX 非 SP 三轮只读采集已完成，见[现场报告](rg34xx-live-inventory-2026-11-22.md)。完整 16 MiB env 分区已读取；其中 128 KiB 环境映像 CRC 有效，25 个变量解析无警告，完整 p3 hash 与 BaseOS prepared env 区域一致。
2. 已完成 TOC1、U-Boot、monitor、DTB 和 Boot0 的组件/镜像指纹对比，并将现场 Boot0 三组 DRAM 参数与 BaseOS RG34XX profile 逐 word 对照；LBA16 主块与 LBA512 两个备用块均全量匹配。继续确认 BROM 实际选择及 DRAM training 结果。
3. 对比两种入口方案：保留原厂链并修改/替换 A32 BL33，或采用 BaseOS 的 A64 SPL + 原厂 monitor + A32 BL33。前者改动面可能更小；后者对 SD/DRAM/恢复路径控制更强，但会接管 Boot0/SPL 职责，验证和恢复复杂度更高。
4. 不论选择哪条路径，先建立一个最小 A32 handoff adapter 和纯 AArch64 payload。payload 只输出早期日志、CurrentEL、寄存器参数和安全范围内的内存信息；不启动 Linkslot、模拟器、Wi-Fi、音频或显示驱动。
5. 先在目标 RG34XX 验证重复冷启动、失败恢复及完整入口契约，再开始 H700 HAL。不得把编译探针、Unicorn 模拟或其他机型的成功记录当成这一步的通过条件。

## 9. 安全边界

本轮公开源码审计与 RG34XX 实机采集均为只读，没有写入 SD 卡、Boot0、TOC1、U-Boot 或 monitor。后续仍须先完成备份、地址冲突证明、入口状态分析和明确恢复方案；在这些条件满足前，不得试写任何启动区域。设备无 eMMC，本项目只设计 SD 启动；失败恢复按项目约定通过电脑重写/修复 SD 卡，不能把自动回退到 Linux 当作产品前提。


## 10. RG34XX 非 SP 首轮实机只读 Inventory（2026-11-22）

本轮真实设备采集、原始 hash、Boot0 两个位置的差异、TOC1 组件身份、运行时 FDT/DRAM、framebuffer 固定物理范围和 GPT/bootargs 逻辑命名关系均已单独归档，详见 [RG34XX 首轮实机只读 Inventory 报告](rg34xx-live-inventory-2026-11-22.md)。

本轮关键更新：

- 现场 FDT 的 rg34xx_v1 面板节点与 720×480 分辨率支持目标是 RG34XX 非 SP；root compatible 的 allwinner,h616 / sun50iw9p1 仍是通用家族标识，不能单独作为 SKU 证明。
- live FDT、/proc/iomem 与 MemTotal 一致显示内核可见 1 GiB RAM；TOC1 内静态 DTB 仍声明 512 MiB。live FDT 的 boot_disp/boot_fb0 与 bootargs disp_reserve 精确给出 framebuffer 区间 `[0x7bf24800, 0x7c076000)`。debugfs 已挂载，但第三轮 ZIP 尚未包含 CMA/memblock 物理布局；新采集器已加入只读抓取，并新增 PFN→物理区间的 JSON 解析器。没有标准 /reserved-memory 节点仍不能据此认定 monitor、secure world 或其他 DMA carveout 不存在。
- LBA 16 位置 Boot0 头部声明 128 KiB，LBA 512 位置头部声明 64 KiB；两者 checksum 均通过，但主体 hash 不同。BaseOS 固定 RG34XX profile 逐 word 对照显示 LBA16 主 0x38 块 32/32 匹配，LBA512 的 0x9f4/0xa74 两个备用块也分别 32/32 匹配，LBA512 主块仅 word[26]/[29] 不同。H616 BROM 会检查 LBA16 和 LBA512，故 LBA16 是高置信正常候选，但未获取本机 BROM 读取轨迹。
- TOC1 中 monitor 与 BaseOS RG34XX 固定指纹完全一致；U-Boot/DTB 呈现 gzip 优化特征，但现场 TOC1 和 U-Boot 都不匹配固定输入包/组件指纹。
- GPT p6=UDISK、p7=primary 与 /proc/cmdline 的 appfs@p6、UDISK@p7 使用不同命名层；运行时 p6 ext4=/data、p7 vfat=/mnt/sdcard，逻辑用途与 bootargs 相符。第三轮完整 p3 env CRC 唯一匹配 128 KiB 映像，25 个变量解析无警告，完整 16 MiB p3 hash 与 BaseOS prepared env 区域完全一致。H616-specific BROM 资料支持 LBA 16/LBA 512 为两处候选位置，LBA 16 最可能为本次正常启动来源。目标 U-Boot 含 update bootcmd/part info 路径字符串，runtime cmdline 与 setargs_mmc 相符；尚无实际函数执行 trace。FDT 检查器已解析 framebuffer `[0x7bf24800,0x7c076000)`。

**当前阶段判断：** 三轮现场采集和完整 env CRC/hash 验证已完成；启动链安全边界、Boot0 当前选择及入口 ABI 审查尚未完成。Issue #2 仍不得据猜测地址构建或启动 payload；在明确 Boot0 选择及 monitor/DRAM 占用前，不触碰启动区域。


## 11. live FDT / initrd / ION / OP-TEE 的实机交叉证据（2026-11-22）

第四轮现场 ZIP 反编译的 live DTB 明确声明 `/chosen/linux,initrd-start=0x42000000`、`linux,initrd-end=0x42000200`；memblock `[0x42000000,0x42001000)` 是覆盖这 512-byte initrd 的 4 KiB 页，因此这条 reservation 现有高置信对象归属。live FDT 总长度与文件长度都是 143,936 bytes，memblock `[0x44000000,0x44023240)` 同样长 143,936 bytes，且起点与 BaseOS 实验 FDT 基址相同；这是高置信 live FDT blob 匹配，但 memblock 输出没有 owner 名称或 U-Boot FDT 指针 trace。

live FDT 同时具有 `/firmware/optee`，`compatible="linaro,optee-tz"`、`method="smc"`；bootargs 则包含 `androidboot.secure_os_exist=0`。现场 dmesg 的 OP-TEE probe 随后记录 `api uid mismatch`。Linux OP-TEE 驱动在这一检查失败时返回 `-EINVAL`，不会继续完成 OP-TEE 版本/capability/共享内存/TEE 设备注册流程（见 [Linux OP-TEE core.c](https://code.googlesource.com/linux/torvalds/linux/+/c6e169bc146a76d5ccbf4d3825f705414352bd03/drivers/tee/optee/core.c)）。因此只能说本次 Linux 没有成功建立 OP-TEE 驱动接口；不能据此宣布 BL31、EL3 或安全世界完全不存在。

FDT 的 Allwinner ION heap 声明包括 `ion_system`（`sys_user`）、`ion_cma` 和 `ion_secure` 三类节点；三者的 `heap-base/heap-size` 都是零，没有给出固定物理范围；FDT 也没有标准 `/reserved-memory` 节点。故未知区间 `[0x7ec00000,0x7fc00000)` 不能因为存在名为 secure 的 ION heap 就自动归类为 secure carveout，必须继续排除直至找到 owner/访问证据。硬件日志筛选没有提供 CMA、BL31 或 secure carveout 的地址证据。

对应的 FDT 解析器已扩充为输出 `initrd`、`firmware_nodes` 和 `ion_heaps`，合成测试覆盖 initrd 边界、OP-TEE conduit 和零尺寸 ION heap。**本轮交叉分析仍未批准 payload 地址或入口。**


## 12. ION / TEE runtime 与 Allwinner DRM-info SMC（2026-11-22 20:47）

20:47:42 的只读现场采集见 [runtime 报告](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/reports/reports/rg34xx-inventory-20261122-204742)。运行内核为 Linux 4.9.170 AArch64；导出的配置包含 `CONFIG_ION=y`、`CONFIG_TEE=y`、`CONFIG_OPTEE=y`、`CONFIG_CMA=y` 和 `CONFIG_DMA_CMA=y`。现场有 `/dev/ion`，`ion-sunxi` 平台驱动已注册；`/dev/tee0`、`/dev/teepriv0` 不存在。标准 OP-TEE 探测日志为 `optee: api uid mismatch`。

ION debugfs 的当前分配为：

- `sys_user`：总计 7,397,376 bytes，其中 `ion_disp2` 为 7,372,800 bytes，另有六项各 4,096 bytes 的 `ion_vin` 分配；orphaned allocations 为 0。
- `cma`：`nextui.elf` 当前分配 4,153,344 bytes；对应 CMA 区域为 `[0x77c00000,0x7bc00000)`，总量 64 MiB，空闲 60,360 KiB，与 `/proc/meminfo` 的 `CmaTotal/CmaFree` 相符。
- `secure`：heap 节点存在，但当前分配为 0，orphaned allocations 为 0。该 debugfs 输出不报告 heap 的 base/size，因此不能据此推断该堆容量或物理区间。

### 12.1 标准 OP-TEE API 与 Allwinner DRM-info 是不同的 SMC 路径

对照 [Allwinner 4.9.170 参考树的 Makefile](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/Makefile)，以及其 [OP-TEE core.c](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/tee/optee/core.c)、[sunxi_ion.c](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/staging/android/ion/sunxi/sunxi_ion.c) 和 [optee_smc.h](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/tee/optee/optee_smc.h)，该参考实现具有以下行为：

1. 标准 OP-TEE driver 先调用 UID 检查；UID 不匹配时会中止标准 TEE driver 初始化，因而没有标准 `/dev/tee0` 接口。
2. Allwinner `ion-sunxi` 在创建 `ION_HEAP_TYPE_SECURE` heap 时，另行调用 `optee_probe_drm_configure()`。该函数发出 `OPTEE_SMC_GET_DRM_INFO`（function ID 15）厂商扩展 SMC，成功时返回 `drm_base`、`drm_size`、`tee_base` 并写入内核日志；失败时日志为 `drm config service not available: <status>`。
3. 该 vendor SMC 不等于标准 OP-TEE UID / revision / capabilities 握手。故现场 `api uid mismatch` 只证明标准 Linux OP-TEE driver 没有初始化，**不能单独证明** Allwinner DRM-info SMC 也失败。
4. 参考实现的 `ion-sunxi` 调用处没有用返回值阻止后续 heap 创建；若 SMC 失败且 DT heap base/size 初始为零，仍不能把 debugfs 中出现的空 `secure` heap 当作安全内存已经配置成功。

这棵参考树的 Makefile 版本为 4.9.170，且采用 `sun50iw9p1` Allwinner 设备树；它是与现场内核版本及 SoC 家族匹配的比较源码，不是已证明与 RG34XX `#19` 二进制逐字节对应的完整源代码。因此上述调用流程需要结合本机日志验证，不能直接视为目标二进制的已证事实。

### 12.2 对 16 MiB 未知保留区的处理

现场 `memblock` 区间 `[0x7ec00000,0x7fc00000)`（16 MiB）仍没有可直接观测的 owner。它可能与 Allwinner DRM/TEE carveout 有关，但当前报告没有 `drm_base/drm_size/tee_base`，也没有 `drm config service not available` 结果；因此目前既不能确认匹配，也不能确认 vendor SMC 失败。该 16 MiB 区间、其余未知 reserved entries 和 RAM 顶部离散碎片继续整体排除，不得用于 payload、heap 或 loader。

只读 collector 的 dmesg 筛选已补充 `drm config service`、`drm_base`、`drm_size`、`tee_base` 关键词。下一轮重新运行后，若出现成功日志，需比较返回的 `drm_base/drm_size` 是否与上述 16 MiB 区间吻合，并核对 `tee_base`；若出现失败日志，保留十六进制返回状态；若两类日志都没有，则需检查目标内核是否包含此 vendor 调用路径，不能推断为零尺寸或 SMC 失败。此项完成前，不批准任何 payload 地址或入口。


## 13. 21:03 DRM-info SMC 复采与下一步判定（2026-11-22）

[21:03 只读复采报告](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261122-210352/DRM-SMC-finding.md) 未捕获 `drm_base/drm_size/tee_base` 成功日志，也未捕获 `drm config service not available` 失败日志。因此它没有给出 `GET_DRM_INFO` 的返回值，不能据此认定 SMC 未执行、成功或失败。

该轮同时确认：

- `/sys/bus/platform/drivers/ion-sunxi/ion` 是已绑定设备的 sysfs 链接，而不仅是 driver 目录存在；
- live FDT 中 `allwinner,secure` heap 节点状态为 `okay`，ID 6，base/size 均为零；
- ION debugfs 导出 `secure` heap，但分配总量为零；
- `/dev/tee0` / `/dev/teepriv0` 仍不存在，标准 OP-TEE UID 检查仍失败。

### 13.1 纠正配置符号检查

此前采集器 grep 使用了 `CONFIG_SUNXI_ION`，但同系列 Allwinner 4.9.170 参考树的 [ION Kconfig](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/staging/android/ion/Kconfig)、[ION Makefile](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/staging/android/ion/Makefile) 和 [sunxi 子目录 Makefile](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/staging/android/ion/sunxi/Makefile) 使用的符号实际是 `CONFIG_ION_SUNXI`，设备树解析选项为 `CONFIG_ION_OF`。故 21:03 报告中“config grep 未显示 CONFIG_SUNXI_ION”的说法是采集器过滤错误造成的，不能用作驱动未编入的证据。

采集器现查询 `CONFIG_ION_SUNXI` 与 `CONFIG_ION_OF`，并在 `/proc/kallsyms` 可读时只输出以下相关符号名称（不保留内核地址）：

- `sunxi_ion_probe`
- `optee_probe_drm_configure`
- `ion_secure_heap_create`

同系列参考源码显示，若目标二进制确实包含相同实现，且 CONFIG_TEE 编译分支启用、secure heap DT 节点被解析为 secure 类型，那么 `sunxi_ion_probe()` 创建该 heap 前会调用 `optee_probe_drm_configure()`。当前观察到驱动绑定及 secure heap 节点，使这条路径值得直接核验；但目标 #19 二进制并未与参考源逐字节对应，不能把参考源码的控制流直接当成本机事实。

下一轮采集需要先核对正确的 config 符号及 kallsyms 符号。若函数符号存在但日志仍缺失，后续应取得与 Linux `#19` 对应的源码/配置或内核镜像，核对该函数是否实际调用、日志级别/输出路径以及 secure heap 创建顺序。不得为获得返回值而从 userspace 主动发 SMC，也不得触碰 unknown reserved memory。

### 13.2 内存安全状态

`[0x7ec00000,0x7fc00000)` 仍是 16 MiB 未知保留区；本轮没有新的 owner 或物理地址证据。CMA `[0x77c00000,0x7bc00000)`、framebuffer `[0x7bf24800,0x7c076000)`、monitor 区 `[0x48000000,0x49000000)` 继续按已知用途排除。其余无 owner 的保留区和 RAM 顶部碎片也继续排除。当前没有批准的 payload RAM 区间或入口地址。

## 14. 21:20 ION 配置与目标内核符号确认（2026-11-22）

[21:20:49 只读采集报告](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261122-212049/ION-symbol-finding.md) 修复了前一轮配置过滤符号名错误，并给出以下现场证据：

- /proc/config.gz 明确包含 CONFIG_ION_SUNXI=y、CONFIG_ION_OF=y、CONFIG_ION=y、CONFIG_TEE=y、CONFIG_OPTEE=y、CONFIG_CMA=y 和 CONFIG_DMA_CMA=y。
- /proc/kallsyms 中存在 sunxi_ion_probe、optee_probe_drm_configure、ion_secure_heap_create 三个函数符号。采集器只输出符号名，不输出内核地址。
- /sys/bus/platform/drivers/ion-sunxi/ion 仍是已绑定设备的 sysfs 链接；ION debugfs 的 sys_user、cma、secure 三个 heap 节点均存在。
- /dev/ion 存在；/dev/tee0 与 /dev/teepriv0 不存在；标准 OP-TEE probe 仍记录 api uid mismatch。
- secure heap 当前分配为 0；live FDT 的 secure heap base/size 仍为 0。

这证明目标 #19 内核编入了 Allwinner ION/TEE 相关配置与三个关键函数，且 ION platform device 已绑定；它不证明 optee_probe_drm_configure() 已在 probe 中执行，也不证明 vendor SMC 成功或失败。21:20 的 dmesg 过滤仍未捕获 drm_base、drm_size、tee_base 或 drm config service not available。不得把缺少这两类日志改写成“SMC 没执行”或“SMC 失败”。

同系列参考实现的 [sunxi_ion.c](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/staging/android/ion/sunxi/sunxi_ion.c) 在 secure heap 路径调用 [optee_probe_drm_configure()](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/tee/optee/core.c)；参考函数使用 OPTEE_SMC_GET_DRM_INFO，非成功返回时 printk drm config service not available，成功时打印 DRM/TEE base 和 size。但该参考树并未证明与 RG34XX #19 二进制逐字节匹配，控制流只能用作反汇编比对的假设。

### 14.1 下一步：取得当前设备的 boot 分区镜像

现场 GPT/启动参数将 PARTNAME=boot 对应到 p4；已记录分区范围为起始 LBA 303104、131072 sectors（64 MiB）。collector 现通过 sysfs 精确匹配 PARTNAME=boot，动态发现对应 block node 并读取完整分区至报告目录的 boot-partition.bin（最大 64 MiB），同时计算 SHA-256；不硬编码 /dev/mmcblk0p4 或固定 raw offset。若 PARTNAME 缺失则记录 skipped，不猜分区号；若分区超过上限或出现重复标签则报错退出。下一轮从采集文件验证 Android boot image header（ANDROID!），解析 kernel payload，并检查内嵌版本标识是否为 Linux version 4.9.170 ... #19 ...。该读取只从设备分区读出数据，不执行 SD/Boot0/TOC1/U-Boot 写入。

[KNULLI 的 RG34XX 开发说明](https://knulli.org/devices/anbernic/rg34xx/)明确指出 Anbernic 未发布该设备的 kernel/U-Boot 源码，开发者需要从 stock SD/固件提取二进制。取得匹配的 kernel payload 后，后续只做静态检查：反汇编 sunxi_ion_probe 与 optee_probe_drm_configure 的实际目标代码、确认调用条件及返回路径，并检查日志字符串是否存在。暂不为获取返回值而重新绑定 driver、触发 ION heap 重建、主动调用 SMC 或改变启动参数。

另一个可用的比对来源是 [Anbernic 官方 RG34XX 下载页](https://win.anbernic.com/download/514.html)：页面列出独立的 RG34XX-V1.0.6-EN16GB-260526 与 RG34XXSP-V1.0.6-EN16GB-260526 下载项。现场 U-Boot bootargs 中的构建时间为 2026-05-26 13:55:09，Linux #19 的构建时间为 2026-05-26 13:58:08；两者与 260526 日期吻合，因此 RG34XX 非 SP 的 V1.0.6 包是一个高优先级候选，可先抽取其 boot.img kernel 并与当前 p4 提取物的 hash/版本/关键字符串比较。日期吻合不是二进制相同的证明，且不得误取 SP 包。

### 14.2 内存安全状态不变

未知区间 [0x7ec00000,0x7fc00000) 仍无 owner 证据；当前没有任何 payload RAM 区间或入口地址获批。CMA、monitor、framebuffer、内核/FDT/启动数据以及 RAM 顶部离散 reserved entries 继续排除。