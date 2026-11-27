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

## 3. RG34XX 当前实际启动链与 stock fallback

2026-11-22 的 RG34XX 非 SP 现场 provenance 审计已将本节原先的“stock U-Boot → Linux”描述修正为实际执行链。当前启动路径（证据等级 A）：

```text
Allwinner Boot ROM
  └─ stock eGON.BT0 Boot0（DRAM training）
       └─ BaseOS native SPL（AArch64 EL3，SRAM）
            ├─ 从 p4 + 40 MiB 读取 native firmware package
            │    ├─ vendor BL31 / monitor → 0x48000000
            │    ├─ DTBO → 0x4a100000
            │    ├─ TOC1 DTB → 0x4a200000
            │    └─ AArch32 SVC BL33 loader → 0x4a000000
            ├─ 从 p4 + 48 MiB 读取 BOL4 Android v2 image → staging 0x45000000
            ├─ 校验镜像长度/FNV，并更新 BL33 header / native handoff record
            └─ 进入 vendor BL31 / monitor
                 └─ ERET 到 AArch32 SVC BL33（SPSR=0x1d3）
                      └─ BaseOS A32 loader
                           ├─ BOL4 raw-LZ4 → AArch64 Image @ 0x40080000
                           ├─ 准备 initrd @ 0x42000000 与 live FDT @ 0x44000000
                           └─ SMC #0：r0=0x8000ff04, r1=0x40080000,
                                      r2=0x44000000, r3=1
                                └─ vendor monitor ERET → AArch64 Linux
                                     （SPSR=0x3c5，x0=0x44000000）
```

当前介质中仍存在 TOC1 AArch32 U-Boot 和 p4 偏移 0 的 gzip Android image，但它们是 stock compatibility/fallback path；现场的 `/chosen/baseos,*` markers、Boot0 trained DRAM → live `/dram` 对应关系、live bootargs 与 loader 生成模板匹配、p4+40 MiB loader/firmware package 和 p4+48 MiB BOL4 image 的身份共同证明本次启动执行的是 native path B，而不是 stock U-Boot path A。详细逐项证据见 [活动 loader provenance 报告](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-active-loader-provenance.md)。

**版本字符串解释：** live FDT 中 `uboot_message=2018.05(05/26/2026-13:55:09)` 不表示 stock TOC1 U-Boot 在本次启动中运行。固定 BaseOS `native/build.py::vendor_version_message()` 会从 TOC1 `u-boot` fallback 项提取版本/日期/时间字面量，并将其写入生成的 bootargs；本机活动路径仍是 A32 loader 替换 BL33 slot。固定 BaseOS 源包含 `rg34xx` prepared profile（compatibility/vendor-handoff），但 `verify-fleet.py` 的 live hardware target 仅为 RG34XX SP / RG40XX V；这表示 `rg34xx` 有 host prepared-source 支持，不等于这台非 SP 的部署 generation 已被精确重建或该实验有同机硬件资格化。

这对 Linkslot 移植的直接含义：GammaOS LK 的 H700 candidate 是替换当前 A32 BL33 loader 的候选程序。vendor BL31 → A32 SVC BL33 是启动入口，不等于 A32 BL33 必须继续执行 Linux-start SMC。若 LK 直接作为独立 A32 裸机系统运行，应留在 AArch32；只有兼容原有 Linux 启动时才复刻 loader 的 BOL4/FDT/SMC 逻辑。不能把 `0x8000ff04` 改写成未经证明的通用 A64 payload jump。

Boot0 负责 DRAM training；不得在 Linkslot 主程序里重新初始化 DRAM，也不能把某台设备训练出的参数写死到同型号所有设备。当前 BL33 slot 的装载地址是活动链事实，但不代表其 1 MiB 研究 arena 全部可供 LK 任意使用；必须结合 monitor、native SPL、reserved-memory 和链接 map 审核实际尺寸与覆盖。

参考：
- [活动 loader provenance](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-active-loader-provenance.md)
- [BaseOS 原生 SPL 设计与交接 ABI](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/experiments/h700-boot-speed/DESIGN.md)
- [BaseOS 当前原生启动实验状态](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/experiments/h700-boot-speed/HANDOFF.md)

## 4. BaseOS native SPL / BL31 / A32 BL33 的已部署交接结构

该结构已经不只是其他设备的研究候选：前述 RG34XX 非 SP 现场 provenance 将 Boot0 → native SPL → vendor BL31 → A32 SVC BL33 确认为当前实际活动链。BaseOS 的深入研究没有采用“让 AArch32 U-Boot 直接跳 AArch64”这种未经验证的捷径，而是形成并在本机运行了两段式交接：

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

目标 RG34XX（非 SP）的活动 native boot path 已有 A 级现场证据，不再把它描述为“仅在其他 H700 设备运行”：

- 当前运行链为 stock Boot0 → BaseOS AArch64 native SPL（EL3）→ vendor BL31 → AArch32 SVC BL33 loader → AArch64 Linux。
- 当前 A32 loader 位于 `0x4a000000`，由 native SPL 从 p4+40 MiB package 装入；BL31 交接时 SPSR 为 `0x1d3`（AArch32 SVC）。该 loader 最终使用厂商 Linux-start SMC 进入 A64 Linux。
- 活动 p4+48 MiB Android v2/BOL4 image 的 kernel component 解压后为 AArch64 `ARMd` Image；BOL4 长度、packed/unpacked 长度、FNV、解压结果、DTB 等均完成 byte-level 校验。
- TOC1 A32 U-Boot 与 p4+0 gzip image 仍在设备介质上，但作为 stock fallback；没有证据表明它们在当前 captured boot 中执行。

这项证明的是当前 native boot flow 与 Linux handoff，不是任意 A64 bare-metal payload 的入口 ABI，也不是 GammaOS LK 已经在该机运行。GammaOS LK H700 target scaffold 是 AArch32，应定位为替换活动 BL33 loader 的候选；真实 LK image 尚未完整链接，BL31→LK 的寄存器/缓存/异常环境也尚未实机确认。

仍未关闭：

- deployed native SPL 的精确构建输入/32-byte generation 尚不能完全从已上传仓库重建；当前 provenance 报告指出 build-time inventory prefix 与 Boot0 bytes 缺失。
- vendor monitor 的 `0x8000ff04` 只证明既有 Linux-start 用法，不证明它接受任意 `r1` A64 入口。
- `[0x7ec00000,0x7fc00000)`、RAM 顶部碎片、secure/DRM/DMA 和其他未标名 reservation 的 owner 仍未知。
- 本项目没有完成 GammaOS LK 完整 target link、实机 BL33 替换、冷启动与恢复验收。
- Linkslot UI/runtime 与 mGBA 尚未在目标设备执行。


## 8. 对 Linkslot Pico 的实施建议

1. **以现场已运行的 native path B 为基线。** 当前目标不是从 stock TOC1 U-Boot 重新发明 A64 启动链，而是评估将 AArch32 GammaOS LK 放入现有 A32 BL33 slot，保留 stock Boot0、native SPL 与 vendor BL31。
2. **明确产品执行状态。** 若 Linkslot 直接作为独立裸机系统运行，优先让 A32 LK 接管后停留在 AArch32；不要为了“转成 A64”而调用 `0x8000ff04`。若仍要启动原有 A64 Linux，则必须实现 native loader 的完整 BOL4/header/FNV、DTB/FDT 编辑、native record 和 SMC contract，而不只是复制一条 SMC 指令。
3. 保留活动链已证明的 monitor/BL33/DTB/initrd/image 区间作为占用或交接信息：monitor `[0x48000000,0x49000000)`、BL33 load base `0x4a000000`、DTBO `0x4a100000`、TOC1 DTB `0x4a200000`、handoff record `0x4a300000`、活动 image staging `0x45000000`、Linux Image `0x40080000`、initrd `0x42000000`、live FDT `0x44000000`。这些地址用于描述现有链，不构成 LK heap/stack/payload 的安全分配许可。
4. 完成 reserved-memory ownership 及 monitor/native SPL/BL33 实际覆盖范围审计后，才填写 H700 linker script 的 `MEMBASE/MEMSIZE`、heap、stack、MMU 和 payload ranges。未知 16 MiB 区域 `[0x7ec00000,0x7fc00000)` 与 top-of-RAM fragments 继续排除。
5. 先在主机侧完成完整 LK ELF/map/image size 与 header 检查，再评估目标机最小 A32 BL33 handoff 测试。未证明恢复路径前不改写 Boot0/TOC1/SD 启动区；不要把其他机型实验当作 RG34XX 现场验收。


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

## 11.2 活动 Android v2 / BOL4 镜像的原生拆分与校验（2026-11-24）

用户最近上传的 `reports/rg34xx-active-inputs/` 包含 manifest、independent verification 和 SHA256SUMS；原始二进制仍只保留在本地，未上传。采集来自 RG34XX p4/boot 64 MiB 只读 dump（SHA-256 `41cfaa8f9ade4a5e05a89188abb7db154243e38b8e19c53b730f0a45ff325516`），提取偏移 `0x03000000`、长度 `0x829000`，没有对设备、SD、Boot0 或 TOC1 执行写操作。

21 项独立检查全部通过：

- 活动镜像与原始 p4 dump 的 `[0x03000000,0x03829000)` 区间逐字节相等，长度 `0x829000`，FNV-1a32 `0x04762a5f`。
- BOL4 frame 长度 `0x80474e`，packed 长度 `0x80473e`，解压长度 `0x010a7008`，packed FNV `0xb4c47d39`；解压输出与 kernel.Image 逐字节相同。
- kernel.Image 长度 `0x010a7008`，magic `ARMd`，Android Image header 字段 `text_offset=0x80000`、`image_size=0x1138000`、`flags=0xa`，确认活动 Linux kernel 是 AArch64 Image。
- Android v2 header 2,048 B，page size 2,048，version 2，header_size 1,660；组件长度按 `[kernel, ramdisk, second, recovery_dtbo, dtb]` 为 `[8406862,512,0,0,144384]`。
- 512-byte initrd 是合法空 `newc` CPIO（含 `TRAILER!!!`），DTB FDT magic 正确，totalsize 139,822 B，小于 144,384 B component extent。
- 本地 SHA256SUMS 包含 8 条记录（六个拆分件及 manifest/verification 自身），本地 `sha256sum -c` 8/8 通过。

工具 `tools/h700_boot_image.py` 来源为当前 porting 分支提交 `b6c3667`，SHA-256 `5c75dc3805ef50117fa19ca0de1896ee82965f4100a68fed0619a0e1ced76c3a`；自测 14 项通过，1 项因宿主缺少 C compiler 跳过。全部属于 host-only 取证。它确认的是活动镜像容器与 kernel ISA，并不验证可刷写性、冷启动、BL33 入口或任意 A64 payload handoff。二进制未上传，因此当前仓库只能复核报告中的长度/hash/校验结果，无法重新对本地原始二进制计算。

报告索引：
- [拆分与验证 README](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-active-inputs/README.md)
- [extracted manifest](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-active-inputs/extracted-manifest.json)
- [21 项 verification](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-active-inputs/verification.json)
- [SHA256SUMS](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-active-inputs/SHA256SUMS.txt)

## 11.3 Allwinner secure-heap / DRM-info SMC 路径的源码线索（2026-11-24）

RG34XX #19 运行时报告确认 `Linux 4.9.170`、`CONFIG_ION_SUNXI=y`、`CONFIG_TEE=y`，并在 kallsyms 名称列表中发现 `sunxi_ion_probe` 与 `optee_probe_drm_configure`；ION secure heap 节点存在但当前 FDT base/size 为零，debugfs secure heap allocations 为 0。现场 dmesg 摘录没有 `drm_base` / `drm_size` / `tee_base` 或 `drm config service not available` 字符串。

另一个可访问的 Allwinner T507 BSP 参考树（`qiaoweibiao/T507_Kernel@98fcc7d3f112e51b0edfb71536da89cb2115106f`，版本同为 Linux 4.9.170，日期 2021-06-03）呈现以下候选代码路径：

1. `sunxi_ion_probe()` 仅在 `CONFIG_TEE` 条件下，遇到 `ION_HEAP_TYPE_SECURE` 时调用 `optee_probe_drm_configure()`。
2. `optee_probe_drm_configure()` 通过 `optee_smccc_smc(OPTEE_SMC_GET_DRM_INFO,...)` 直接执行 SMC；返回 `a0` 非 `OPTEE_SMC_RETURN_OK` 时记录 `drm config service not available`，否则从 `a1/a2/a3` 读出 DRM base、DRM size、TEE base 并打印。
3. 这条 DRM-info SMC 是独立的直接调用路径，不依赖 Linux 标准 OP-TEE 驱动已经通过 `api uid mismatch` 探测。故 `optee: api uid mismatch` 不能证明 DRM-info SMC 已失败。

**证据边界：** 该公开 4.9.170 BSP 不是已确认与 RG34XX #19 完全一致的源码快照；目标机上函数符号存在也只证明代码被编译进内核，不能证明该分支在本次启动中实际执行。活动 provenance 报告还指出 bootargs 最后一个 `loglevel=1`，因此 console capture 对默认级别 `printk` 的可见性有限；没有 success/failure 文本不能作为 SMC 未调用的证明。需要取得匹配 #19 的源码/符号交叉引用，或通过安全、只读的既有日志渠道证明调用和返回值；不得从用户态主动发该 SMC、不得试探未知 carveout。

无论该 SMC 是否执行，`[0x7ec00000,0x7fc00000)` 的 16 MiB reservation 仍无已验证 owner。它不得分配给 LK、heap、stack、mGBA、DMA 或 payload；secure ION heap 的名字和零 base/size 都不构成物理归属证据。参考代码只用于定位待核对的调用链：
- [T507 sunxi_ion.c](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/staging/android/ion/sunxi/sunxi_ion.c)
- [T507 optee core.c](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/tee/optee/core.c)
- [T507 optee_smc.h](https://github.com/qiaoweibiao/T507_Kernel/blob/98fcc7d3f112e51b0edfb71536da89cb2115106f/drivers/tee/optee/optee_smc.h)
