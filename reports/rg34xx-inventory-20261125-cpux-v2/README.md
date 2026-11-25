# RG34XX/H700 CPU 时钟、电源轨、SID、OPP 与内存预留只读盘点 v2（2026-11-25）

本目录是 `scripts/run-rg34xx-inventory.ps1 -OutputDirectory .\build\rg34xx-inventory-20261125-cpux-v2`
一次完整只读盘点的文本侧产物与核对说明。原始二进制捕获（`boot0-primary/secondary.bin`、
`toc1-package.bin`、`env-partition.bin`、`live-fdt.dtb`，合计约 18.5 MB）留在采集机
`build/rg34xx-inventory-20261125-cpux-v2/`，未上传。

- 采集时间：设备本地 `2026-11-25 00:41:21 UTC`（= `2026-11-25 08:41:21 +08:00`），采集器在设备端耗时 **3.5 s**
- 设备：`RG34xx` / `sun50iw9p1`（`model = sun50iw9`，`compatible = allwinner,h616, arm,sun50iw9p1`），
  Linux `4.9.170` aarch64，ADB shell `uid=0`，单设备
- 采集链：`porting/linkslot-1.0.45-feasibility` @ `7f07ea2`；采集器
  `collect-rg34xx-readonly-inventory.sh` blob `94cce9b7a0a90a28a7e427cfb84d2a9f6be540b8`
  （比上一版 `ce88451c…` 新增"同步采样"段）；`tools/inspect_h700_fdt.py` blob `361d8ec6…`
  （新增 `opp-microvolt-<变体>` 与 OPP 表 `nvmem-cells` 解析）
- 盘：`/dev/mmcblk0`，15,552,479,232 B，`root=/dev/mmcblk0p5`
- 性质：**只读**。采集器只对源设备执行 `dd if=`，未向任何分区、sysfs 控制文件或 PMIC/时钟寄存器写入。
  `not_installable`、`not_hardware_validated`

## 1. 产物

| 文件 | 字节 | SHA-256（前 16） |
| --- | ---: | --- |
| `cpu-power-runtime.txt` | 42,630 | `c4632005eab49731…` |
| `live-fdt.json` | 30,680 | `18acd82337afd13c…`（已脱敏，见 §11） |
| `dtb-report.json` | 29,578 | `de6e61f9adb6dcb3…` |
| `memory-map-report.json` | 16,774 | `dd64f67c2b40c33e…` |
| `dmesg-hardware-lines.txt` | 11,156 | `2f58ca62aade8289…` |
| `sid-cell-report.json` | 3,003 | `03700cee092e418a…` |
| `debugfs-memory-reservations.txt` | 2,664 | `6d256713f030dd60…` |
| `meminfo.txt` / `partitions.txt` / `gpt-fdisk.txt` | 1,009 / 823 / 690 | `daebe182…` / `6e9436ad…` / `0645d3e5…` |
| `raw-sha256.txt` | 737 | `3fa868f7235f4b1e…` |
| `cmdline.txt` | 682 | `24b0148a159d0166…`（已脱敏） |
| `summary.txt` | 375 | `b4e408e4d4c77819…` |
| `page-size.txt` | 5 | `25ac87b7309b7946…` |

## 2. 与上一次盘点（`rg34xx-inventory-20261125-cpux`）的可复现性

五个分区级捕获与 live FDT **逐字节相同**，说明设备状态与采集路径稳定：

| 捕获 | SHA-256 | 与 v1 一致 |
| --- | --- | --- |
| `boot0-primary.bin` | `bf886995745a0eb9…` | ✅ |
| `boot0-secondary.bin` | `5f143679e0d2c529…` | ✅ |
| `toc1-package.bin` | `e6c20e3a54c51b93…` | ✅ |
| `env-partition.bin` | `8dd964ef0d525071…` | ✅ |
| `live-fdt.dtb` | `cbb0bf254f366a86…` | ✅ |

同时 `memory-map-report.json`、`debugfs-memory-reservations.txt`、`meminfo.txt` 等也完全相同
（连 SHA 都一样）；差异只出现在运行态读数（频率、电压、温度）与新增的同步采样段。

## 3. CPU 父时钟：`pll_cpu`（分频 1）

`clk_summary` 层级恒为 `pll_cpu → cpu → cpuapb`，`cpu` 速率每次都**恰好等于** `pll_cpu`，
`cpuapb` 恒为 1/4，`axi` 恒为 1/2：

| 采样 | `pll_cpu` | `cpu` | `cpuapb` | `axi` |
| --- | ---: | ---: | ---: | ---: |
| 1 | 1416 MHz | 1416 MHz | 354 MHz | 708 MHz |
| 2 | 1104 MHz | 1104 MHz | 276 MHz | 552 MHz |
| 3–6 | 1104 MHz | 1104 MHz | 276 MHz | 552 MHz |

CPU **不**挂在 `pll_periph0` 上。`pll_periph0` 子树（APB、PIO、LRADC 等）在全部采样中保持恒定：

| 时钟 | 速率 | enable/prepare |
| --- | ---: | --- |
| `pll_periph0` | 600 MHz | 4 / 4 |
| `pll_periph0x2` | 1200 MHz | 2 / 2 |
| `pll_periph0x4` | 2400 MHz | 0 / 0 |
| `pll_periph0d6` | 100 MHz | 0 / 0 |
| `pll_periph0div25m` | 25 MHz | 0 / 0 |

`pll_cpu` 自身的 enable/prepare 计数恒为 `0 / 0`（常开，驱动不记账），因此该计数不能用来判断
PLL 是否在用；可用判据只有 `cpu` 与其同速。

## 4. 频率回读 vs 时钟树：多数是读取时差，但 480 MHz 有一段真实窗口

本次采集器新增了 6 组"`uptime` + cpufreq + `clk_summary`"紧邻采样，用来区分"调频瞬态"与
"持续不匹配"。结果：

| 采样 | policy0 `scaling_cur_freq` | policy0 `cpuinfo_cur_freq` | `pll_cpu` |
| --- | ---: | ---: | ---: |
| 1 | 1416 | 1416 | 1416 |
| 2 | 1104 | 1104 | 1104 |
| 3–5 | 1008 | 1104 | 1104 |
| 6 | 1008 | 1008 | 1104 |

再补做 8 组 0.15 s 间隔的原子采样（含 dcdc1 电压）：

| 采样 | `scaling_cur_freq` | `cpuinfo_cur_freq` | dcdc1 |
| --- | ---: | ---: | ---: |
| 1 | 1512 | 1512 | 1160 mV |
| 2 | 480 | 1008 | 950 mV |
| 3–5 | 480 | 1008 | 1000 mV |
| 6 | 480 | 1104 | 1000 mV |
| 7 | 480 | 1008 | 950 mV |
| 8 | 480 | 1008 | 1000 mV |

另做 6 组原始行采样，观察到 `scaling = info = 1200 MHz` 而 `pll_cpu = 1512 MHz`（第 1 组）与
`scaling = info = 1008 MHz` 而 `pll_cpu = 1104 MHz`（第 6 组）——即 sysfs 先更新、时钟后切换。

结论分两层，务必分开引用：

1. **读取并非原子**。cpufreq sysfs、`clk_summary`、`regulator` 三处是分别读的，跨越切换瞬间就会
   出现"请求值/实际值/PLL"三者不一致。表 1、表 2 中绝大多数不一致属于这一类，不能据此断言故障。
2. **但存在超出读取时差的窗口**：连续 ≥1.1 s（8 组、间隔 0.15 s）`scaling_cur_freq` 恒为 480 MHz，
   而 `cpuinfo_cur_freq` 与 `pll_cpu` 恒为 1008 MHz，dcdc1 电压为 950/1000 mV（对应 1008 的两档）。
   也就是说请求被记录下来了，但时钟与调压器都没有跟随。本次采集**不能**区分这是 governor/policy
   侧被钳制（例如 IPA 热策略、`scaling_min_freq` 约束）还是时钟驱动拒绝该速率。
   `/sys/devices/system/cpu/cpu0/cpufreq/stats/` 在本内核上不存在，无法用 `time_in_state` 交叉验证。

电压侧还有一个可解释的滞后现象：`cpuinfo_cur_freq = 1008` 时电压仍是 1000 mV（= 1104 MHz 档位），
说明调压器比时钟慢一步；因此**单点回读不能当作 OPP↔电压表**的判据。

## 5. CPU 电源轨：`axp2202-dcdc1`

- DTS：`/cpus/cpu@0` 的 `cpu-supply` = phandle `0x4f` =
  `/soc@03000000/twi@0x07081400/pmu/regulators@0/dcdc1`（`always-on`、`boot-on`，500–1540 mV）
- `regulator_summary`：`axp2202-dcdc1` 电压 1160 mV，消费者 `cpu0 @1160mV`（另一个 `reg-virt-consumer.0.auto` 是占位）
- 实测随 OPP 变化：1512 → 1160 mV、1104 → 1000 mV、1008 → 950/1000 mV
- `axp2202-cpusldo` 900 mV，`use=0`，**无消费者**，不是 CPU 轨

## 6. OPP 表结构：表级 `nvmem-cells` + 每档 `opp-microvolt-<变体>`

`/opp_l_table`（`allwinner,sun50i-operating-points`，`opp-shared`）这次被完整解析出来：

- `nvmem-cells = <&speed>`，`nvmem-cell-names = "speed"`，指向 `/sunxi-sid-ng@03006000/speed@00`（phandle `0xe0`）
- 同一频率有多条 OPP 节点，分成两类：
  - 带 `supported-hw = <N>` + 明文 `opp-microvolt = <...>`
  - `supported-hw = <>` + `opp-microvolt-a1/b0/b1/c0/c1/c2`（按速度档位给电压）

启动日志显示 5 个频率被丢弃，且都是"取不到电压"：

```text
core: _opp_supported_by_regulators: OPP minuV: 0 maxuV: 0, not supported by regulator
cpu cpu0: _opp_add: OPP not supported by regulators (600000000 / 792000000 / 1032000000 / 1296000000 / 1344000000)
```

被丢弃的 5 个频率，正好是"在本机生效的 `supported-hw` 掩码下只剩变体条目"的那些；内核取到的
`minuV/maxuV` 都是 0，说明变体查找没有命中，条目因无电压被移除。最终生效集合为：

```text
480000 720000 936000 1008000 1104000 1200000 1320000 1416000 1512000
```

`scaling_governor = schedutil`，`scaling_driver = cpufreq-dt`，`scaling_min_freq = 480000`，
`scaling_max_freq = 1512000`，`thermal_zone0` 策略为 `power_allocator`（IPA）。

⚠️ 本报告**不**由 `speed@0x00` 反推速度档位，也**不**据此批准任何 OPP：变体名与 SID 原始值的
对应关系在本机上未被验证，且变体条目在本机全部查找失败，本身就是"该对应关系不可用"的证据。

## 7. SID 原始值：两次盘点完全一致

| cell | DTS 偏移 | 长度 | 原始字节 | 解码 |
| --- | --- | ---: | --- | --- |
| `speed` | `0x00` | 2 | `00 6c` | LE u16 = `0x6c00`，**不匹配**任何钉住的厂商字面量（`0x2400/0x2c00/0x5000/0x5400/0x5c00/0x7400/0x7c00`） |
| `calib` | `0x14` | 8 | `18 11 13 ab 19 1b 16 bb` | 4×12 bit = 2835 / 2841 / 2838 / 2842；出厂测试温度 raw 280（28.0 ℃） |

两次盘点（07:37 与 08:41）读到的字节完全相同，说明读路径稳定，不是偶发采样错误。
`0x6c00` 在 Linux mainline 参考解码下落在 candidate bin 5，但这只是**参考分类**，不构成 RG34XX
的 OPP 授权（mainline 与该厂商树的速度档语义未对齐）。

## 8. 温度校准：仍未生效

```text
[    1.019774] sunxi-thermal 5070400.thermal_sensor: avcc is not calibrated
```

当前读数（`thermal_zone0..4`）：cpu 46.6 ℃、gpu 47.1 ℃、ve 46.6 ℃、ddr 47.2 ℃、电池 30.0 ℃，
数值均在合理范围；启动期另有两处 `Error reading temperature for gpu thermal zone: -11`。

但 SID 里有校准数据、驱动却报 `avcc is not calibrated` —— 即**校准未被接受**。因此这些温度只能
当诊断值看，**不能**作为闭环 DVFS / 降频保护的依据。

## 9. 运行时 FDT vs 打包 DTB：多了 BaseOS 原生启动交接数据

| 项 | 打包 DTB（TOC1 `dtb` 项） | 运行时 `/sys/firmware/fdt` |
| --- | ---: | ---: |
| 实际字节 | 139,822（条目 144,384 B，尾随 4,562 B 补零） | 143,936 |
| 节点数 | 573 | 573 |
| SHA-256 | `c28c8738…89603a1` | `cbb0bf25…94c1f7cc` |
| `size_struct` / `size_strings` | 122,256 / 15,446 | 123,152 / 15,537 |
| memreserve | `0x48000000`+16 MiB | `0x48000000`+16 MiB、**`0x42000000`+512 B** |

节点数相同、结构块 +896 B、字符串表 +91 B、总计 +4,114 B。差异不是 DTBO overlay 叠加，而是引导链
对同一棵树的**属性级写入**，新增字符串 `boot_disp1`、`boot_disp2`、`boot_fb0`、
`baseos,native-boot`、`baseos,native-timing`、`baseos,boot-display`。具体落点：

| 节点 | 属性 | 长度 | 内容要点 |
| --- | --- | ---: | --- |
| `/chosen` | `baseos,native-boot` | 32 | `02 2f0d8816…4c0fee` |
| `/chosen` | `baseos,native-timing` | 56 | 7 组 `(0, t)`：`0x68351b`、`0x76a417`、`0x1248137`、`0x138b47d`、`0x1618ba2`、`0x1e1da13`、`0x1e1da14` |
| `/chosen` | `baseos,boot-display` | 80 | ASCII `DSP1` + `0x01` + 若干字段，尾部是**与 `baseos,native-boot` 完全相同的 32 字节块** |
| `/soc@03000000/disp@01000000` | `boot_fb0` | 36 | 字符串 `7bf24800,2d0,1e0,20,b40,0,0,2d0,1e0` |
| 同上 | `boot_disp` / `boot_disp1` / `boot_disp2` | 4 / 4 / 4 | `0x00000100` / `0x01040000` / `0x00000004` |

`boot_fb0` 与内存侧完全对上：`0x7bf24800` = `disp_reserve` 基址，`0x2d0/0x1e0` = 720×480，
`0x20` = 32 bpp，`0xb40` = 2880 B stride，合计 1,382,400 B。

`/chosen` 里没有 `baseos,native-boot` 之外的解释信息，`cmdline` 也不含 `baseos` 字样
（`console=ttyS0,115200 root=/dev/mmcblk0p5 androidboot.serialno=[REDACTED]`），因此这些属性是
**引导链在启动时注入**的；本报告只记录其存在与原始字节，不推断其语义或校验其正确性。

## 10. 内存预留

- RAM：`0x40000000–0x80000000`（1 GiB），`MemTotal 996,700 kB`
- `memblock` 预留 46 段；`/sys/kernel/debug/cma` **不存在**，CMA 走全局 `cma=64M`：
  `CmaTotal 65,536 kB / CmaFree 64,416 kB`（已用 1,120 kB）
- 关键预留：`0x40080000`+17,620 KiB、`0x42000000`+4 KiB（与 §9 新增的 512 B memreserve 同一地址）、
  `0x48000000`+16 MiB（monitor/BL31 载入区）、`0x77c00000`+64 MiB（CMA）
- framebuffer：`0x7bf24800`，1,382,400 B，`matches_disp_reserve = true`

## 11. 边界与脱敏

- `cmdline.txt` 与 `live-fdt.json` 中的设备序列号按既有惯例替换为 `[REDACTED]`（字节级替换，
  `live-fdt.json` 保留 CRLF；各 −18 B = 2×9 B）。
- `memory-map-report.json` 的 `fdt_sha256 = cbb0bf25…` 指向**未脱敏**的原始 FDT，与上传的
  `live-fdt.json`（`18acd823…`）不同，这是预期差异。
- `sid-cell-report.json` 的 `source_sha256 = c4632005…` 指向未脱敏的 `cpu-power-runtime.txt`
  （该文件不含序列号，因此上传版 SHA 相同）。
- 本报告只覆盖一次文本侧捕获。未读取或写入任何 PMIC、PLL、时钟、热控寄存器；未改动 governor、
  频率上下限、电压或分区内容。**不构成**可安装性、硬件适配或 OPP 授权结论。
