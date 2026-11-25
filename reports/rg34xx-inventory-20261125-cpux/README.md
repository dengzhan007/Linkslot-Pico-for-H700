# RG34XX/H700 CPU 时钟、电源轨、SID 与内存预留只读盘点（2026-11-25）

本目录是 `scripts/run-rg34xx-inventory.ps1` 一次完整只读盘点的文本侧产物与核对说明。
原始二进制捕获（`boot0-primary/secondary.bin`、`toc1-package.bin`、`env-partition.bin`、
`live-fdt.dtb`，合计约 18.5 MB）留在采集机 `build/rg34xx-inventory-20261125-cpux/`，未上传。

- 采集时间：设备本地 `2026-11-24 23:37:46 UTC`（= `2026-11-25 07:37:46 +08:00`）
- 设备：`RG34xx` / `sun50iw9p1`（`compatible = allwinner,h616, arm,sun50iw9p1`），Linux `4.9.170 #19 SMP PREEMPT aarch64`
- 采集链：`porting/linkslot-1.0.45-feasibility` @ `aad78d4`；采集器 `collect-rg34xx-readonly-inventory.sh`
  SHA-256 `ce88451c9365bb55…`（v1），ADB shell `uid=0`
- 盘：`/dev/mmcblk0`，15,552,479,232 B，`root=/dev/mmcblk0p5`
- 性质：**只读**。采集器只对源设备执行 `dd if=`，未向任何分区、sysfs 控制文件或 PMIC/时钟寄存器写入。
  `not_installable`、`not_hardware_validated`

## 1. 产物

| 文件 | 字节 | SHA-256（前 16） |
| --- | ---: | --- |
| `live-fdt.json` | 25,719 | `c358bf27097bd91f…`（已脱敏，见 §9） |
| `cpu-power-runtime.txt` | 32,189 | `35b0609d500d6641…` |
| `sid-cell-report.json` | 3,003 | `8e52578b2f9c3651…` |
| `debugfs-memory-reservations.txt` | 2,664 | `6d256713f030dd60…` |
| `memory-map-report.json` | 16,774 | `dd64f67c2b40c33e…` |
| `summary.txt` | 375 | `7d0f3eb5d81ad433…` |
| `raw-sha256.txt` | 737 | `a9b3cbf93b3625c4…` |
| `cmdline.txt` | 682 | `24b0148a159d0166…`（已脱敏） |
| `meminfo.txt` / `partitions.txt` / `gpt-fdisk.txt` | 1,009 / 823 / 690 | — |
| `dmesg-hardware-lines.txt` | 11,156 | `2f58ca62aade8289…` |
| `page-size.txt` | 5 | `25ac87b7309b7946…` |

## 2. CPU 时钟：父时钟是 `pll_cpu`，不是 `pll_periph0`

`clk_summary` 中的层级为 `pll_cpu → cpu → cpuapb`，且 `cpu` 速率在每次采样中都**恰好等于**
`pll_cpu`（分频 1）；`cpuapb` 恒为 1/4：

| 采样 | cpufreq `scaling_cur_freq` | `pll_cpu` | `cpu` | `cpuapb` |
| --- | ---: | ---: | ---: | ---: |
| A | 1416 MHz | 1512 MHz | 1512 MHz | 378 MHz |
| B | 1104 MHz | 1104 MHz | 1104 MHz | 276 MHz |
| C | 1008 MHz | 1104 MHz | 1104 MHz | 276 MHz |
| D | 480 MHz | 1008 MHz | 1008 MHz | 252 MHz |

`pll_periph0` 子树（与 CPU 无关，供 APB、PIO、LRADC 等）：

| 时钟 | 速率 | enable/prepare |
| --- | ---: | --- |
| `pll_periph0` | 600 MHz | 4 / 4 |
| `pll_periph0x2` | 1200 MHz | 2 / 2 |
| `pll_periph0x4` | 2400 MHz | 0 / 0 |
| `pll_periph0d6` | 100 MHz | 0 / 0 |
| `pll_periph0div25m` | 25 MHz | 0 / 0 |

**必须同时记录的矛盾**：cpufreq 报 480 MHz 时 `pll_cpu` 仍显示 1008/1104 MHz（连续 3 次采样稳定复现），
电压也在 0.95/1.00/1.16 V 之间跳动。也就是说 `clk_summary` 的 PLL 速率与 cpufreq 视图不同步
（缓存或分频路径差异）。因此本节**只认定父时钟关系**；单次快照不足以断言某一时刻的实际 ARM 频率。
另：`pll_cpu`/`cpu` 的 enable/prepare 计数恒为 0，属厂商驱动对常开 CPU 时钟不记账，不能解读为门控关闭。

## 3. CPU 电源轨：`axp2202-dcdc1`

- DTS：`/cpus/cpu@0` 的 `cpu-supply` = phandle `0x4f` = `/soc@03000000/twi@0x07081400/pmu/regulators@0/dcdc1`，
  `axp2202-dcdc1`，500,000–1,540,000 µV，`always_on` + `boot_on`，`status=okay`。
- 运行时：`/sys/kernel/debug/regulator/regulator_summary` 中 `axp2202-dcdc1` 的消费者正是
  `cpu0 @1160mV`；`/sys/class/regulator/regulator.1/microvolts` 实测随 OPP 变化：
  1008 MHz→950 mV、1104 MHz→1000 mV、1416 MHz→1160 mV。
- `axp2202-cpusldo`（900 mV）**没有消费者**，不是 CPU 轨，不要误认。

这些读数与 DTS `opp_l_table` 中 `supported-hw = 0x30`（1008→950 mV、1104→1.00 V）和
`0x20`（1416→1.16 V）的条目吻合，说明生效的 speed-bin 掩码至少含 bit4/bit5。
SID 原始 speed 值与这个掩码的对应关系未验证（见 §4）。

启动日志中另有 5 条被丢弃的 OPP，均为"只有无电压通用条目"的频率：

```text
core: _opp_supported_by_regulators: OPP minuV: 0 maxuV: 0, not supported by regulator
cpu cpu0: _opp_add: OPP not supported by regulators (600000000 / 792000000 / 1032000000 / 1296000000 / 1344000000)
```

这与 `scaling_available_frequencies`（480/720/936/1008/1104/1200/1320/1416/1512 MHz）不含这 5 个频率一致。

## 4. SID 原始值

Provider 命中 `/sys/bus/nvmem/devices/sunxi-sid0`（`of_node = /sys/firmware/devicetree/base/sunxi-sid-ng@03006000`），
仅按 DTS 声明的两个 cell 做有界读取，未 dump 整段 NVMEM：

| cell | 偏移/长度 | 原始字节 | 解码 |
| --- | --- | --- | --- |
| speed | `0x00` / 2 B | `00 6c` | `raw_u16_le = 0x6c00` |
| thermal calib | `0x14` / 8 B | `18 11 13 ab 19 1b 16 bb` | 4×12 bit：2835 / 2841 / 2838 / 2842，出厂测试温度 raw 280（28.0 ℃） |

- `0x6c00` **不匹配**任何钉住的厂商字面量（`0x2400 0x2c00 0x5000 0x5400 0x5c00 0x7400 0x7c00`）。
- mainline 参考解码归为 candidate bin 5，但该路径需要 `SMCCC_ARCH_SOC_ID` revision，
  且明确标注 `opp_authorization: not_approved`——**不构成 RG34XX 的 OPP 批准**。
- `/sys/devices/soc0` 未导出，`generic_soc_identity` 为 `unavailable`；`sunxi_info`/`/dev/sunxi_soc_info`
  按纪律未读（会触发 CE 时钟/复位且有泄露 chipid 的风险）。

## 5. 温度校准：SID 有校准数据，但驱动报告未校准

这是本次最关键的一条：

```text
[    1.019774] sunxi-thermal 5070400.thermal_sensor: avcc is not calibrated
```

即 SID `calib@0x14` 里的四路 12 bit 校准码存在且数值合理，但 `sunxi-thermal` 在 probe 阶段
判定 AVCC 未校准，**没有把这些值用于温度换算**。因此当前温度读数只能当诊断参考：

| zone | 类型 | 读数 |
| --- | --- | ---: |
| 0 | `cpu_thermal_zone` | 45.5 ℃（45,503 m℃） |
| 1 | `gpu_thermal_zone` | 46.4 ℃（46,394 m℃） |
| 2 | `ve_thermal_zone` | 45.5 ℃ |
| 3 | `ddr_thermal_zone` | 45.2 ℃ |
| 4 | `axp2202-battery` | 30.0 ℃ |

zone0 的 trip 为 60/70/115 ℃（passive/passive/critical），其余 zone 未导出 trip 属性；
gpu zone 启动期还有两次 `Error reading temperature for gpu thermal zone: -11`。
结论：**温度链不可用于闭环 DVFS 决策**，`calibration_status = not_certified`。

## 6. 内存预留

`/sys/kernel/debug/cma` 不存在（Linux 4.9 未导出 CMA debugfs），但 `meminfo` 显示
`CmaTotal 65,536 kB / CmaFree 64,416 kB`，对应 bootargs `cma=64M`；`memblock` 中也能看到
`0x77c00000`+64 MiB 的预留。`memory-map-report.json` 解析结果为 0 个 CMA area、1 个 memory range、
46 个 reserved range（CMA 按 `cma=` 预留而非设备树 `linux,cma` 节点，且无 debugfs 归因）。

| 区间 | 长度 | 判断 |
| --- | ---: | --- |
| `0x40000000–0x7fffffff` | 1 GiB | 唯一 memory range（`MemTotal 996,700 kB`） |
| `0x48000000–0x48ffffff` | 16 MiB | 正是 monitor/BL31 载入地址 `0x48000000` |
| `0x77c00000–0x7bbfffff` | 64 MiB | `cma=64M` 全局 CMA |
| `0x7ec00000–0x7fbfffff` | 16 MiB | 未归因预留 |
| `0x7bf24800`+1,382,400 B | 1.38 MiB | `disp_reserve`，与 framebuffer 720×480×32bpp、stride 2880 完全一致 |

## 7. live FDT 与打包 DTB 不同

| | live `/sys/firmware/fdt` | 打包 `toc1-extracted/items/dtb.bin` |
| --- | ---: | ---: |
| 文件字节 | 143,936 | 144,384 |
| `totalsize` | 143,936 | 139,822 |
| 节点数 | 573 | 573 |
| SHA-256 | `cbb0bf25…` | `c28c8738…` |

节点数、`model=sun50iw9`、`compatible=allwinner,h616/arm,sun50iw9p1` 全部相同，总量 +4,114 B。
**节点数不变而结构块增大，说明是属性级改写（bootargs/serialno 等）而非 DTBO overlay 展开**；
overlay 新增节点会改变节点数。live FDT 无 `reserved-memory` 节点；ION heap 为
`sys_user`/`cma`/`secure`（均未声明固定 base）；面板 `rg34xx_v1` 720×480。

> 注意：因为 §9 的脱敏替换，上传版 `live-fdt.json` 的字节与 SHA 与设备原文件不同，
> `memory-map-report.json` 中的 `fdt_sha256 = cbb0bf25…` 指的是**未脱敏**的原始 FDT。

## 8. 执行环境说明（可复现性）

本次未直接运行 `.ps1`：本机无 `adb`（借用 `platform-tools/adb.exe`）、PowerShell 执行策略禁止脚本文件，
且 `adb push/pull` 的进度输出走 stderr 会触发脚本的 `$ErrorActionPreference = "Stop"`。
因此按脚本的等价步骤手工执行（push → `sh` 采集 → pull → 同一批 Python 工具解析），
解析工具与参数与脚本一致（`inspect_h700_boot0.py`、`parse_h700_toc1.py`、`inspect_h700_fdt.py`、
`inspect_h700_env.py`、`inspect_h700_memory_map.py`、`inspect_h700_sid_cells.py`）。
设备端采集耗时 2.3 s；`/tmp` 为 256 MB tmpfs，采集后我推送的脚本与目录已全部删除。
TOC1 校验通过（`0x5923eb81`，4 项），env 镜像 CRC 通过（25 个变量，16,777,216 B）。

## 9. 脱敏

`cmdline.txt` 与 `live-fdt.json` 中的设备序列号（`snum=` / `androidboot.serialno=` / bootargs 两处）
已替换为 `[REDACTED]`，其余字节与行尾（CRLF）保持原样，替换量恰好为 -18 B（2×9 B）。
`cpu-power-runtime.txt`、`sid-cell-report.json` 等其余文件未做任何改写。

## 10. 边界

- 本文只覆盖一次静态快照；时钟/电压在采样间持续变化，§2 的矛盾说明不能用单点数据推断真实频率。
- SID 原始值、speed-bin 语义、AVCC 校准是否被驱动接受，均**未**由本报告证实。
- 未读取也未写入任何 PMIC 寄存器、CCU 寄存器、OPP 表或 thermal 控制文件。
- 内存区间的用途标注（BL31/CMA/disp_reserve）来自地址与 bootargs 的对应关系，非内核归因输出。
