# RG34XX 启动交接审计

## 1. 范围与结论

本报告仅针对 **Anbernic RG34XX（非 SP）/ Allwinner H700 / Cortex-A53 / 1 GiB LPDDR4**。RG34XX SP 是独立机型，不可将其 2 GiB 内存、`rg34xxsp_v1` 面板或固件指纹当作本机数据。

本次审计只读取公开仓库和已提交研究资料，没有改写 SD 卡、Boot0、TOC1、U-Boot 或 monitor。

**结论：现有 AArch32 U-Boot 不能被视为可直接启动任意 AArch64 裸机程序的入口。** BaseOS-MultiFrontend 已经研究出一条更具体的候选架构：由 AArch64 SPL 负责 SD、DRAM 和加载；保留厂商 monitor；让 monitor 进入一个 RAM 中的 AArch32 SVC BL33 适配器；适配器再通过厂商 SMC 交接到 AArch64。它为 Linkslot Pico 提供了可研究的桥接方式，但其最终 SMC 是按厂商 Linux 启动 ABI 逆向验证的，尚未证明它是通用裸机 payload ABI，更没有在 RG34XX 非 SP 上完成实机验收。

## 2. 证据等级

本文区分三类证据，不能互相替代：

1. **RG34XX 型号固件静态证据**：来自 BaseOS-MultiFrontend 固定的 `rg34xx` prepared 输入、兼容性清单、U-Boot/Boot0 静态分析和模拟器执行。
2. **其他 H700 实验机的实机证据**：BaseOS 原生 SPL 项目记录了 RG34XX SP（2 GiB LPDDR4）及 RG40XX V（1 GiB LPDDR3）实验。它能证明某些共享 H700 monitor/启动机制可工作，但不能代替 RG34XX 非 SP 的 1 GiB LPDDR4 验收。
3. **本项目 RG34XX 待验证项**：实际掌机读回的 Boot0/TOC1/U-Boot/monitor、主板 revision、DRAM 实际训练结果、入口寄存器状态及最小 AArch64 payload 冷启动。

BaseOS 参考源本次按 `main` 快照审查，commit：[`cbe253a43f46cf551cc9a6c79a84015bde3686cf`](https://github.com/dengzhan007/BaseOS-MultiFrontend/commit/cbe253a43f46cf551cc9a6c79a84015bde3686cf)。

## 3. RG34XX 原厂启动链

已知启动结构：

```text
Allwinner Boot ROM
  └─ SD / eGON.BT0 Boot0（64 KiB）
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

1. 先对用户当前 RG34XX 做只读 inventory：完整 SD 镜像/GPT、Boot0 两份副本、TOC1、U-Boot、monitor、env、live FDT、`/proc/iomem`、`/proc/cmdline`、DRAM 容量与设备树保留区。输出文件和 SHA-256，不写入任何启动区域。
2. 对比 live 读回与 `manifest/prepared/rg34xx.json`、`manifest/kernel-gzip.json`、`compatibility.json`。先解释每个允许变化字段，再判断能否复用 BaseOS 固件证据。
3. 对比两种入口方案：保留原厂链并修改/替换 A32 BL33，或采用 BaseOS 的 A64 SPL + 原厂 monitor + A32 BL33。前者改动面可能更小；后者对 SD/DRAM/恢复路径控制更强，但会接管 Boot0/SPL 职责，验证和恢复复杂度更高。
4. 不论选择哪条路径，先建立一个最小 A32 handoff adapter 和纯 AArch64 payload。payload 只输出早期日志、CurrentEL、寄存器参数和安全范围内的内存信息；不启动 Linkslot、模拟器、Wi-Fi、音频或显示驱动。
5. 先在目标 RG34XX 验证重复冷启动、失败恢复及完整入口契约，再开始 H700 HAL。不得把编译探针、Unicorn 模拟或其他机型的成功记录当成这一步的通过条件。

## 9. 安全边界

本次审查为只读，不涉及刷写或现场设备访问。后续首轮实机审计仍只读；在完成备份、地址冲突证明、入口状态分析和明确恢复方案前，不得试写 Boot0、TOC1、U-Boot 或 monitor。设备无 eMMC，本项目只设计 SD 启动；失败恢复按项目约定通过电脑重写/修复 SD 卡，不能把自动回退到 Linux 当作产品前提。
