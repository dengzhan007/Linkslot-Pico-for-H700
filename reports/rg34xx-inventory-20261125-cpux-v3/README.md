# RG34XX/H700 CPU 时钟、电源轨、SID、OPP 与内存预留只读盘点 v3（2026-11-25）

本目录是 `scripts/run-rg34xx-inventory.ps1 -OutputDirectory .\build\rg34xx-inventory-20261125-cpux-v3`
一次完整只读盘点的文本侧产物与核对说明。原始二进制捕获（`boot0-primary/secondary.bin`、
`toc1-package.bin`、`env-partition.bin`、`live-fdt.dtb`，以及 `toc1-extracted/items/*`，合计约 20 MB）
留在采集机 `build/rg34xx-inventory-20261125-cpux-v3/`，未上传。

- 采集时间：设备本地 `2026-11-25 03:19:48 UTC`（= `2026-11-25 11:19:48 +08:00`），远程输出目录
  `/tmp/rg34xx-inventory-20261125-112116`
- 设备：`RG34xx` / `sun50iw9p1`（`model = sun50iw9`，`compatible = allwinner,h616, arm,sun50iw9p1`），
  Linux `4.9.170` aarch64，ADB shell `uid=0`，单设备 `2c00…bd2`（序列号已脱敏）
- 采集链：`porting/linkslot-1.0.45-feasibility` @ `d9da2e4`（含 2026-11-25 上午的 H700 更新）；采集器
  `collect-rg34xx-readonly-inventory.sh` blob `f386360e…`（比 v2 的 `94cce9b7…` 新增
  **cpufreq getter 对账**段）；`tools/inspect_h700_fdt.py` blob `33f8e470…`
- 盘：`/dev/mmcblk0`，15,552,479,232 B，`root=/dev/mmcblk0p5`
- 性质：**只读**。采集器只对源设备执行 `dd if=`，未向任何分区、sysfs 控制文件或 PMIC/时钟寄存器写入；
  设备端回执 `No write was issued to /dev/mmcblk0`。`not_installable`、`not_hardware_validated`

## 1. 产物

| 文件 | 字节 | SHA-256（前 16） |
| --- | ---: | --- |
| `cpu-power-runtime.txt` | 59,388 | `8a1263b88b6a6a4a` |
| `live-fdt.json` | 38,933 | `3f46b7db29dd26c7`（已脱敏，见 §8） |
| `dtb-report.json` | 37,831 | `bd6802d86504b96d`（打包 DTB 解析，来自 `toc1-extracted/`） |
| `memory-map-report.json` | 16,774 | `dd64f67c2b40c33e` |
| `dmesg-hardware-lines.txt` | 11,156 | `2f58ca62aade8289` |
| `sid-cell-report.json` | 3,003 | `9e4564549b3ab346` |
| `debugfs-memory-reservations.txt` | 2,664 | `6d256713f030dd60` |
| `meminfo.txt` / `partitions.txt` / `gpt-fdisk.txt` | 1,009 / 823 / 690 | `14c81674…` / `6e9436ad…` / `0645d3e5…` |
| `raw-sha256.txt` | 737 | `d3f074b6b238e152` |
| `cmdline.txt` | 682 | `24b0148a159d0166`（已脱敏） |
| `summary.txt` | 375 | `8230c867519930f0` |
| `page-size.txt` | 5 | `25ac87b7309b7946` |

## 2. 与 v1/v2 的可复现性：五个分区级捕获逐字节相同

| 捕获 | SHA-256 | 与 v1/v2 一致 |
| --- | --- | --- |
| `boot0-primary.bin` | `bf886995745a0eb9…` | ✅ |
| `boot0-secondary.bin` | `5f143679e0d2c529…` | ✅ |
| `toc1-package.bin` | `e6c20e3a54c51b93…` | ✅ |
| `env-partition.bin` | `8dd964ef0d525071…` | ✅ |
| `live-fdt.dtb` | `cbb0bf254f366a86…` | ✅ |

`memory-map-report.json`、`debugfs-memory-reservations.txt`、`gpt-fdisk.txt`、`partitions.txt`、
`page-size.txt`、`cmdline.txt`（脱敏后）的 SHA 也与 v2 完全相同——设备状态与采集路径在三次盘点间稳定。
差异只来自：① 运行态读数（频率、电压、温度、meminfo 计数）；② 新版采集器新增的 getter 对账段，
使 `cpu-power-runtime.txt` 由 42,630 B 增至 59,388 B；③ 新版 FDT 工具输出更完整，
使 `live-fdt.json` / `dtb-report.json` 体积增大（原始 `.dtb` 未变）。

## 3. 核心结论更新：v2 的"480 MHz 请求未生效窗口"确认为读取瞬态，不是真实钳制

v2 记录了一个悬而未决的现象：连续 ≥1.1 s 内 `scaling_cur_freq` 恒为 480 MHz，而
`cpuinfo_cur_freq` 与 `pll_cpu` 恒为 1008 MHz，怀疑 governor 被钳制或时钟驱动拒绝该速率。
本次采集器新增的 **cpufreq getter 对账段**（同一时刻对 `scaling_cur_freq` / `cpuinfo_cur_freq`
做"读—再读—再读"，中间穿插 `clk_summary` 与 dcdc1）直接给出了答案。

8 组对账采样（uptime 从 14398.16 到 14399.58，约 0.2 s 间隔）：

| uptime | `scaling` 读前 | `cpuinfo` 读1 | `scaling` 读后 | `cpuinfo` 读2 | `scaling` 再读后 | `pll_cpu` | dcdc1 |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 14398.16 | 1416 | 1416 | 1416 | 1416 | 1416 | 1416 | 1150 mV |
| 14398.36 | 1008 | 1104 | **1104** | 1104 | 1104 | 1104 | 1000 mV |
| 14398.56 | 1104 | 1104 | 1104 | 1104 | 1104 | 1104 | 1000 mV |
| 14398.77 | 1008 | 1104 | **1104** | 1104 | 1104 | 1200 | 1050 mV |
| 14398.97 | 1008 | 1008 | 1008 | 1008 | **1104** | 1104 | 1000 mV |
| 14399.17 | 1008 | 1008 | 1008 | **1104** | 1104 | 1104 | 1000 mV |
| 14399.38 | 1008 | 1104 | **1104** | 1104 | 1104 | 1104 | 1000 mV |
| 14399.58 | 1008 | 1104 | **1104** | 1104 | 1104 | 1104 | 1000 mV |

判据：

1. **同一个 sysfs getter 在几十毫秒内两次读取就发生变化**（样本 2/4：`scaling` 1008→1104；
   样本 5：`scaling` 第 3 次读 1008→1104；样本 6：`cpuinfo` 第 2 次读 1008→1104）。
   若 governor 真的把频率钳在 480/1008 MHz，连续读同一属性不可能自行跳到 1104 MHz。
2. 每次读到 1104 MHz 时，`pll_cpu` 与 dcdc1（1000 mV = 1104 MHz 档）同步到位，说明
   **时钟与调压器都已切换完成**，只是 cpufreq sysfs 的缓存/回读值滞后一拍。
3. 据此，v2 的"480 MHz 持续窗口"应重新解释为：cpufreq 属性回读的滞后/采样竞态，
   **不是** governor/policy 侧钳制，也**不是**时钟驱动拒绝速率。本次证据不支持"频率请求未生效"
   这一故障判断；同时样本 4 出现 `pll_cpu = 1200 MHz` 而回读 1104 MHz，进一步印证多属性
   非原子读取是主因。

保留 v2 的两层结论：**读取非原子**始终成立；但"超出读取时差的真实窗口"这一条在 v3 证据下
**被推翻**，不再作为异常记录。`/sys/.../cpufreq/stats/` 在本内核不存在，仍无法用 `time_in_state` 交叉验证。

## 4. CPU 父时钟与电源轨：与 v2 一致

- 父时钟 `pll_cpu → cpu → cpuapb/axi`，`cpu` 速率恒等于 `pll_cpu`，`cpuapb` = 1/4、`axi` = 1/2；
  CPU **不**挂 `pll_periph0`（后者恒 600 MHz / 4 users）。
- `scaling_driver = cpufreq-dt`，`scaling_governor = schedutil`，
  `scaling_min_freq = 480000`，`scaling_max_freq = 1512000`；
  可用频率 `480 720 936 1008 1104 1200 1320 1416 1512 MHz`（9 档）。
- CPU 轨 `axp2202-dcdc1`（DTS phandle `0x4f`，500–1540 mV，`always-on`/`boot-on`），
  实测 1512→1150 mV、1104→1000 mV、1416→1150 mV；`axp2202-cpusldo` 900 mV 无消费者。

## 5. OPP 表、SID、温度：与 v2 一致

- `/opp_l_table` 表级 `nvmem-cells = <&speed>` + 每档 `opp-microvolt-{a1,b0,b1,c0,c1,c2}`；
  5 个频率（600/792/1032/1296/1344 MHz）因变体查找未命中、`minuV/maxuV = 0` 被内核丢弃。
- SID `speed@0x00` = `0x6c00`（不匹配任何钉住的厂商字面量，mainline 参考分类 bin 5 仅供参考）；
  `calib@0x14` = `181113ab191b16bb` → 4×12 bit = 2835/2841/2838/2842，出厂测试温度 raw 280（28.0 ℃）。
  两次盘点字节完全相同，读路径稳定。**不**据此批准任何 OPP。
- `sunxi-thermal: avcc is not calibrated` 仍在，校准未被接受。当前读数 cpu 47.7 ℃、gpu 47.9 ℃、
  ve/ddr 47.6 ℃、电池 30.0 ℃，仅作诊断值，**不能**作为闭环 DVFS 依据。

## 6. 运行时 FDT vs 打包 DTB：BaseOS 原生启动交接数据（与 v2 一致）

| 项 | 打包 DTB（TOC1 `dtb`） | 运行时 `/sys/firmware/fdt` |
| --- | ---: | ---: |
| 实际字节 / totalsize | 144,384 / 139,822 | 143,936 / 143,936 |
| 节点数 | 573 | 573 |
| SHA-256 | `c28c8738…89603a1` | `cbb0bf25…94c1f7cc` |

节点数相同、结构块增大约 896 B，差异是引导链对同一棵树的**属性级写入**（非 DTBO overlay）：
`/chosen` 新增 `baseos,native-boot`(32 B)、`baseos,native-timing`(56 B)、
`baseos,boot-display`(80 B)，`/soc@03000000/disp@01000000` 新增
`boot_fb0`(`7bf24800,2d0,1e0,20,b40,0,0,2d0,1e0` = 720×480@32bpp, stride 2880 B)、
`boot_disp`/`boot_disp1`/`boot_disp2`。打包 DTB 的 `/memory@40000000` 为 512 MiB，
运行时为 1 GiB（`0x40000000–0x80000000`）——该差异为运行时 FDT 采用实际容量。
本报告只记录存在与原始字节，**不**推断语义或校验正确性。

## 7. 内存预留与 env

- RAM 1 GiB，`MemTotal 996,700 kB`；memblock 预留 46 段；`/sys/kernel/debug/cma` 不存在，
  CMA 走全局 `cma=64M`；关键预留 `0x48000000`+16 MiB、`0x77c00000`+64 MiB（CMA）。
- framebuffer `0x7bf24800`，1,382,400 B，`matches_disp_reserve = true`。
- env 分区 16 MiB，CRC 校验通过（`crc_valid = true`，存储 CRC `0xe3b6e401`，25 个变量）。

## 8. 边界与脱敏

- `cmdline.txt` 与 `live-fdt.json` 中的设备序列号按既有惯例替换为 `[REDACTED]`（字节级替换，
  各 −18 B = 2×9 B）；脱敏后 `cmdline.txt` 的 SHA 与 v2 完全相同（`24b0148a…`）。
- `memory-map-report.json` 的 `fdt_sha256 = cbb0bf25…` 指向**未脱敏**的原始 FDT，与上传的
  `live-fdt.json`（`3f46b7db…`）不同，这是预期差异。
- `sid-cell-report.json` 的 `source_sha256 = 8a1263b8…` 与上传的 `cpu-power-runtime.txt` 一致
  （该文件不含序列号）。
- 本报告只覆盖一次文本侧捕获。未读取或写入任何 PMIC、PLL、时钟、热控寄存器；未改动 governor、
  频率上下限、电压或分区内容。**不构成**可安装性、硬件适配或 OPP 授权结论。
