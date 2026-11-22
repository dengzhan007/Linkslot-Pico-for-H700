# RG34XX 硬件基线（首台目标设备）

## 1. 目标机型与适用范围

本项目第一台、也是当前唯一目标设备锁定为 **Anbernic RG34XX（横版，非 RG34XX SP）**。

依据：

- 本项目用户确认：BaseOS-MultiFrontend 已在 RG34XX 实机正常运行。
- [BaseOS-MultiFrontend `devices.json`](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/devices.json) 将本机型标识为 `rg34xx`，独立于 `rg34xxsp`。
- [BaseOS RG34XX prepared manifest](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/manifest/prepared/rg34xx.json) 固定了该机型的 stockmod 输入镜像和分区来源。
- [Anbernic 官方产品规格](https://anbernic.com/products/rg34xx) 与 [ROCKNIX RG34XX 硬件清单](https://rocknix.org/devices/anbernic/rg34xx/)列出该机型的 H700、1 GB LPDDR4、720×480 屏幕和无 eMMC。

**不能将 RG34XX SP 的数据直接套用到本机型。** BaseOS 的固件调查已经表明两者的 Boot0 DRAM 参数和面板驱动名称存在差异；RG34XX SP 是 2 GB LPDDR4，RG34XX 是 1 GB LPDDR4。后续支持其他 H700 掌机时，必须分别建立 profile。

## 2. 已确认硬件信息

| 项目 | RG34XX 基线 | 证据与备注 |
|---|---|---|
| SoC | Allwinner H700 | 不是 Rockchip RK3566 |
| CPU | 四核 Arm Cortex-A53，最高约 1.5 GHz | 裸机编译 CPU target 使用 `cortex-a53` |
| GPU | Mali-G31 MP2 | H700 SoC 集成 GPU |
| RAM | 1 GB LPDDR4 | 不应使用 RG34XX SP 的 2 GB profile |
| RAM training | Boot0 首选 LPDDR4 参数，672 MHz；另有 LPDDR3 候选参数 | Boot0 负责训练/选择；不应在应用层盲目重写 DRAM 控制器 |
| LCD | 3.4 英寸 IPS，720×480 | 横向面板；设备 profile 不旋转 |
| Kernel panel driver name | `rg34xx_v1` | 不是 `rg34xxsp_v1` |
| LCD type selection | BaseOS 固件分析记录 PE6 strap 高电平为 `old`，低电平为 `boe` | 需在本机只读确认 strap/实际面板，不把猜测写入驱动 |
| Storage | TF/microSD 启动；无 eMMC | 项目不设计 eMMC 安装或自动回退 |
| Wi-Fi / Bluetooth | 2.4/5 GHz Wi-Fi，Bluetooth 4.2 | BaseOS 使用 RTL8821CS SDIO WLAN 模块和 RTL 蓝牙低功耗支持；固件/供电顺序需按本机验证 |
| USB | H700 USB 2.0 OTG/Host 能力 | SoC 能力不等于本机连接器的具体角色/PHY 路由，需实机探测 |
| PMIC / Power | BaseOS 提供 AXP2202 关机路径及电源状态集成 | 本 profile 暂不把 PMIC 寄存器访问当作已验证的裸机实现；需读取 RG34XX live DT/sysfs/内核日志确认实际绑定器件 |
| Audio | BaseOS 有 RG34XX 专属 speaker amp kernel profile | 可用作 codec/功放行为参考；Linux 内核模块不能直接作为裸机驱动 |

## 3. RG34XX 启动链基线

BaseOS-MultiFrontend 中的固件审计资料确认了 RG34XX 使用 Allwinner H700 vendor boot chain，而非 Rockchip 启动链。

当前可作为调查起点的阶段：

1. Allwinner Boot ROM 从 SD 卡启动。
2. `boot0` 使用 `eGON.BT0` 镜像格式；BaseOS 的目标调查记录了 64 KiB Boot0 镜像及 DRAM 参数候选表。
3. Boot0 根据候选参数完成 DRAM training，并将选择结果交给后续阶段。
4. Boot0 从 TOC1 固件包加载 vendor monitor（BL31 相关阶段）、U-Boot 和设备树数据。
5. RG34XX vendor U-Boot 是 32-bit AArch32 代码；BaseOS 的 U-Boot 静态分析资料标注其基于 vendor U-Boot 2018.05。
6. vendor U-Boot 通过 monitor 路径进入 AArch64 Linux；不能假设它能直接以 AArch64 调用约定跳转到任意裸机 payload。

BaseOS 的模拟/静态分析记录中出现 Linux entry `0x40080000` 和 FDT `0x44000000`。这只作为该 vendor Linux 启动路径的研究线索，**不是已经验证的 Linkslot Pico payload 地址、入口 ABI 或可用内存区间**。不得据此链接并刷写裸机程序。

### 现有 SD 分区启动约定

BaseOS 的 RG34XX 参考镜像保留 vendor GPT 关键布局，U-Boot 按名称读取 `boot` 分区（p4），再处理 Android boot image。BaseOS 的 RG34XX prepared manifest 和 boot-chain 文档包含该构建的分区起点、尺寸、Boot0/U-Boot SHA-256。

这些是 BaseOS 当前镜像的约定，并不要求 Linkslot Pico 沿用它的 rootfs、分区号或更新布局。新项目只需在确认 Boot ROM/U-Boot 的真实读盘要求后，决定自己 SD 卡镜像如何组织。

## 4. Wi-Fi、音频和电源的可移植性边界

### Wi-Fi / Bluetooth

BaseOS 相关代码和文档显示：

- WLAN 驱动使用 `8821cs.ko`，对应 RTL8821CS SDIO。
- 蓝牙低功耗使用 `rtl_btlpm.ko`；控制器初始化通过 `rtk_hciattach` 的 UART H5 路径。
- Wi-Fi 上电时序可能影响 SDIO 枚举。BaseOS 记录了先清理过早枚举的 stale SDIO card、再加载 WLAN driver 的流程；少数无线模组在断电后需要更长的 radio-off 时间才能重新识别。

这些 Linux 模块及脚本只能用来理解芯片、固件和供电时序，不能直接加入 LK。裸机 WLAN driver 要确认 SDIO host、IRQ、regulator/enable GPIO、firmware blob 和 reset sequence。

### Audio

BaseOS 为 RG34XX 建有单独的扬声器功放补丁 profile，并使用 ALSA/厂商 codec 接口管理音频。其 profile 与精确 kernel Image hash 绑定，说明相关私有结构不是稳定 ABI。

裸机移植需要从目标设备实时设备树、内核驱动和测试记录中继续确认：

- codec 型号和寄存器接口；
- I2S/DAudio 时钟、格式和 DMA；
- speaker amplifier GPIO 的极性、上电延时和静音顺序；
- headphone jack 检测与切换；
- PMIC/电源域对音频的影响。

### PMIC / 完全关机

BaseOS 的 `axp-off` helper 实现了 AXP2202 软关机路径（I2C 地址 0x34、寄存器 0x27 的 POWER_OFF 位），并在切电前要求持久化文件系统完成 flush 和只读卸载。但兼容性清单列出了多个 PMIC compatible，不能仅从某一个辅助程序推断所有 RG34XX 主板批次都相同。

裸机阶段先通过设备树、内核启动日志和只读 sysfs 确认 RG34XX 实机绑定的 PMIC，再按其数据手册和实际电源时序实现完全断电。禁止直接把 BaseOS userspace helper 的寄存器写操作照搬到裸机。

## 5. 对 Linkslot Pico 工程的直接影响

### 工具链

当前目标应使用：

```sh
--target=aarch64-none-elf
-mcpu=cortex-a53
```

不得再使用 `-mcpu=cortex-a55`。RG34XX 的 CPU 是 Cortex-A53。C/C++、汇编探针和后续 emulator core build 都必须使用一致的 AArch64/A53 target。

### 启动与异常级别

由于现有 vendor U-Boot 为 AArch32，而最终目标 payload 是 AArch64，M1 的核心问题不只是找到一个 SD 文件加载命令，还包括：

- U-Boot/monitor 是否支持将一个独立镜像交接到 AArch64；
- 进入 payload 时处于 EL1 还是 EL2；
- MMU、D-cache、I-cache、栈、异常向量和 GIC 的入口状态；
- x0/DTB、寄存器、reserved memory 和 secure monitor 的交接约定；
- 哪些 DRAM/ SRAM 区域已被 Boot0、monitor、U-Boot、framebuffer 或 DMA 占用。

保留 vendor Boot0 的 DRAM training 和 monitor，先研究通过现有 SD 启动链进入自定义 AArch64 payload 的最小路径，通常比一开始替换 Boot0/DRAM training 风险更低；是否可行仍由静态审计和实机验证决定。

### Pico 代码移植

GammaOS Pico 当前参考树面向 MT6785/32-bit ARM。其 MTK 驱动和 A32 core blob 不适用于 Allwinner H700。可以参考任务模型、Core ABI、渲染结构和网络协议，但所有 H700 外设驱动需以 Allwinner H700 为目标重做或适配。

### 范围限制

本阶段只承诺 RG34XX：

- 暂不把 RG34XX SP、RG35XX、RG40XX 或其他 H700 设备纳入同一份硬件 profile；
- 不假定不同批次面板、Wi-Fi 模组、PMIC 的 strap/时序一致；
- 每个差异都需要 profile 或实机探测证据。

## 6. 证据来源

### 用户已验证工程

- [BaseOS-MultiFrontend](https://github.com/dengzhan007/BaseOS-MultiFrontend)
- [RG34XX device ID and capabilities](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/devices.json)
- [RG34XX prepared firmware manifest](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/manifest/prepared/rg34xx.json)
- [RG34XX DRAM and panel compatibility survey](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/experiments/h700-boot-speed/evidence/compatibility.json)
- [Vendor boot-chain and DRAM/panel analysis](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/experiments/h700-boot-speed/variant-survey/README.md)
- [Vendor U-Boot trace notes](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/experiments/h700-boot-speed/evidence/uboot-trace-notes.md)
- [BaseOS boot-chain and partition description](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/docs/00-boot-chain-and-partitions.md)
- [BaseOS power, audio and network integration](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/docs/05-runtime-power-network.md)
- [BaseOS USB implementation](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/docs/08-usb-adb-and-otg.md)
- [BaseOS RG34XX speaker amplifier profile](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/src/h700-speaker-amp/profiles.json)

### Manufacturer / device reference

- [Anbernic RG34XX official specifications](https://anbernic.com/products/rg34xx)
- [ROCKNIX RG34XX hardware profile](https://rocknix.org/devices/anbernic/rg34xx/)
- [Allwinner H700 brief](https://www.allwinnertech.com/uploads/pdf/2021070513595227.pdf)

## 7. 未确认项目

以下内容不能从当前代码仓库资料直接宣称裸机已支持：

- RG34XX 实机 U-Boot prompt/命令列表和能否通过特定命令加载独立 AArch64 payload；
- payload 实际入口 EL、寄存器 ABI、栈及 MMU/cache 状态；
- RG34XX 实机 DRAM 可用区间与保留内存地图；
- exact panel vendor/revision 与 PE6 实际电平；
- 该台实机 Wi-Fi 芯片版本、firmware 和 power/reset GPIO；
- PMIC exact device ID、IRQ/GPIO route 与最终 soft-off timing；
- codec/功放具体寄存器、I2S 时钟和 DMA route；
- OTG/USB PHY 端口的精确连线。

这些作为 M1/M2 的探测任务；未经验证不得写入默认驱动配置。
