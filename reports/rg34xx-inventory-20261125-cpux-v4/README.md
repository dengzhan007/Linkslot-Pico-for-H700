# RG34XX/H700 内核与 CPUFreq 驱动来源只读盘点 v4（2026-11-25）

本目录是 `scripts/run-rg34xx-inventory.ps1 -OutputDirectory .\build\rg34xx-inventory-20261125-cpux-v4`
一次完整只读盘点的文本侧产物与核对说明。**本次重点是新增的 `kernel-driver-provenance.txt`**——只读采集内核
构建身份、build-id、相关内核配置、platform driver/device 绑定与过滤后的 cpufreq/OPP/NVMEM 符号名。
原始二进制捕获（`boot0-primary/secondary.bin`、`toc1-package.bin`、`env-partition.bin`、`live-fdt.dtb`
及 `toc1-extracted/items/*`，合计约 20 MB）留在采集机 `build/rg34xx-inventory-20261125-cpux-v4/`，未上传。

- 采集时间：设备本地 `2026-11-25 04:05:51 UTC`（= `2026-11-25 12:05:51 +08:00`），远程输出目录
  `/tmp/rg34xx-inventory-20261125-120719`
- 设备：`RG34xx` / `sun50iw9p1`（`model = sun50iw9`，`compatible = allwinner,h616, arm,sun50iw9p1`），
  Linux `4.9.170`，ADB shell `uid=0`，单设备 `2c00…bd2`（序列号已脱敏）
- 采集链：`porting/linkslot-1.0.45-feasibility` @ `b30fac3`（新增 provenance 采集器）；采集器
  `collect-rg34xx-readonly-inventory.sh` blob `0366c257…`（v3 为 `f386360e…`，新增内核/build-id/符号只读段）
- 盘：`/dev/mmcblk0`，15,552,479,232 B，`root=/dev/mmcblk0p5`
- 性质：**只读**。采集器只对源设备执行读取，设备端回执 `No write was issued to /dev/mmcblk0`；
  未调用任何厂商 ioctl/SMC，未触碰 PMIC/PLL/CCU 控制或 sysfs 写节点。`not_installable`、`not_hardware_validated`

## 1. 产物

| 文件 | 字节 | SHA-256（前 16） |
| --- | ---: | --- |
| **`kernel-driver-provenance.txt`** | **6,980** | `aac9e1551ed940f9`（本次重点） |
| `cpu-power-runtime.txt` | 59,386 | `f8f55cc25b58ac68` |
| `live-fdt.json` | 38,933 | `d485e28ce8c241ca`（已脱敏，见 §7） |
| `dtb-report.json` | 37,831 | `0f410d0246864ae5`（打包 DTB 解析，来自 `toc1-extracted/`） |
| `memory-map-report.json` | 16,774 | `dd64f67c2b40c33e` |
| `dmesg-hardware-lines.txt` | 11,156 | `2f58ca62aade8289` |
| `sid-cell-report.json` | 3,003 | `198f163fd932a8a5` |
| `debugfs-memory-reservations.txt` | 2,664 | `6d256713f030dd60` |
| `meminfo.txt` / `partitions.txt` / `gpt-fdisk.txt` | 1,009 / 823 / 690 | `b54cb53a…` / `6e9436ad…` / `0645d3e5…` |
| `raw-sha256.txt` | 737 | `050a2bd72dd63487` |
| `cmdline.txt` | 682 | `24b0148a159d0166`（已脱敏） |
| `summary.txt` | 375 | `fdc5c0223057d6e3` |
| `page-size.txt` | 5 | `25ac87b7309b7946` |

## 2. 与 v1/v2/v3 的可复现性

五个分区级捕获逐字节相同（四次盘点一致）：

| 捕获 | SHA-256（前 16） |
| --- | --- |
| `boot0-primary.bin` | `bf886995745a0eb9…` |
| `boot0-secondary.bin` | `5f143679e0d2c529…` |
| `toc1-package.bin` | `e6c20e3a54c51b93…` |
| `env-partition.bin` | `8dd964ef0d525071…` |
| `live-fdt.dtb` | `cbb0bf254f366a86…` |

`cmdline.txt`（脱敏后）、`memory-map-report.json`、`debugfs-memory-reservations.txt`、`dmesg-hardware-lines.txt`、
`partitions.txt`、`gpt-fdisk.txt`、`page-size.txt` 的 SHA 也与 v3 完全相同。
`live-fdt.json` / `dtb-report.json` 与 v3 的唯一差异是文件内记录的本地输出路径字符串（v3→v4），
解析内容、节点数、字段值全部一致；原始 `.dtb` 未变。`cpu-power-runtime.txt` 与 `meminfo.txt` 的差异仅为运行态读数。

## 3. 重点：`kernel-driver-provenance.txt` 证据与边界

### 3.1 内核构建身份

```text
Linux rg34xx 4.9.170 #19 SMP PREEMPT Tue May 26 13:58:08 CST 2026 aarch64
Linux version 4.9.170 (flower@flower-B85M-D2V) (gcc version 5.3.1 20160412 (Linaro GCC 5.3-2016.05)) #19 SMP PREEMPT Tue May 26 13:58:08 CST 2026
```

- 编译器 **Linaro GCC 5.3-2016.05**、构建机 `flower@flower-B85M-D2V`、构建时间
  `2026-05-26 13:58:08 CST`。该时间与历史报告中 U-Boot `2018.05(05/26/2026-13:55:09)` 的当天构建窗口吻合，
  但二者是不同组件，**不构成同一构建产物的证明**。

### 3.2 build-id 证据

- `/sys/kernel/notes`：SHA-256 `6065c1a7ae894047f0e902551811b39d0c5a11b5afd894083eae46497d56e12d`，
  前 256 字节的前 32 字节为：
  `04 00 00 00 | 14 00 00 00 | 03 00 00 00 | 47 4e 55 00 | a1 d2 11 f8 47 a5 fb 69 fe 98 98 e1 5f bc cd d3 | ea 8d 7b 68`
  - 按 ELF note 格式解：`namesz=4`、`descsz=20`、`type=3`（`NT_GNU_BUILD_ID`）、名字 `GNU\0`，
    随后 20 字节 GNU build-id = `a1d211f847a5fb69fe9898e15fbccdd3ea8d7b68`。
  - 该 build-id 只作为后续与具体 vmlinux/构建产物比对的**指纹**；本次没有可比对的目标二进制，
    因此**不**据此识别 shipping 内核来源。
- `/sys/kernel/build_id` 与 `/sys/kernel/notes/build-id` 均 `NOT_PRESENT`：本内核只从 notes 暴露 build-id。

### 3.3 相关内核配置（来自 `/proc/config.gz`，IKCONFIG 可用）

| 配置 | 值 | 含义 |
| --- | --- | --- |
| `CONFIG_IKCONFIG` / `_PROC` / `CONFIG_KALLSYMS` / `_ALL` | `y` | 配置与全符号可见，是本次只读采集的前提 |
| `CONFIG_CPU_FREQ` / `CONFIG_PM_OPP` / `CONFIG_CPUFREQ_DT` / `_PLATDEV` | `y` | cpufreq-dt 与 OPP 框架编入内核 |
| `CONFIG_CPU_FREQ_GOV_SCHEDUTIL` | `y` | schedutil 可用；但默认 governor 仍是 `performance`（`...DEFAULT_GOV_PERFORMANCE=y`） |
| `CONFIG_CPU_FREQ_STAT` | **未设置** | 印证 v2/v3 的 `time_in_state` 不存在，无法交叉验证频率驻留 |
| `CONFIG_REGULATOR_AXP2101` | `y` | 板级 regulator 驱动；CPU 轨的实际 DTS 绑定仍是 `axp2202-dcdc1`（见 v1/v2） |
| `CONFIG_NVMEM` / `_AXP` / `_SUN50I_SID` | `y` | SID NVMEM 路径编入；`CONFIG_NVMEM_SUNXI_SID` 未设置 |

### 3.4 platform driver/device 绑定与符号

- `/sys/bus/platform/drivers/` 下 `cpufreq-dt` 与 `sun50i-cpufreq-nvmem` 两个 driver 均存在；
  对应 `/sys/bus/platform/devices/` 下同名 device 的 `[driver]` 链接分别指回各自驱动，
  `uevent` 为 `MODALIAS=platform:cpufreq-dt` / `platform:sun50i-cpufreq-nvmem`。
  两个 `uevent` 属性读回 `Permission denied`（`-w-------`），按只读原则未做任何写操作去改变权限。
- `policy0` 目录里 `scaling_governor` / `scaling_max_freq` / `scaling_setspeed` 为可写权限位，但本次**只读**。
- 已加载模块：`NO_MATCHING_MODULES (drivers may be built-in)` —— 不能据此判断驱动是否编入。
- 过滤后的符号名（**地址已按设计丢弃**）包含：
  - `sun50i_cpufreq_init/exit/driver/pdev/match_list/nvmem_probe/nvmem_remove`
  - `dev_pm_opp_set_prop_name`、`dev_pm_opp_set_supported_hw`、`opp_parse_supplies.isra.0`

这些符号证明在本内核里确实存在 cpufreq/OPP 驱动与 NVMEM 探测路径，且与 v2/v3 观察到的
`supported-hw` 与 `opp-microvolt-<变体>` 表结构属于同一功能域。但符号存在**不等于**能确定：

1. 实际运行的 shipping 二进制/源码路径（存在多个候选实现，配置/符号无法唯一指认）；
2. `0x6c00 → b1` 的映射关系——上游 OPP 候选分析在 v3 上得到"唯一 `no-mask + b1` 指纹匹配"，
   那只是选择行为的指纹，**不能**仅凭频率/电压匹配反推 SID 速度档，也不能批准任何 OPP；
3. 驱动是否真的调用了 `dev_pm_opp_set_prop_name("b1")`、如何消费表级 speed NVMEM cell、
   是否注册 `supported_hw`——本次只读采集未覆盖这些调用点，保持 UNKNOWN。

因此本次结论是**收窄了候选空间**：确认驱动域与构建指纹存在，但 shipping 二进制与 `b1` 选择原因仍未闭合。
`getconf`/`cpufreq stats`/原始 PMIC 寄存器等缺口不变。

## 4. CPU 时钟、OPP、SID、温度：与 v3 一致

- `pll_cpu → cpu → cpuapb/axi`，`cpu` 恒等于 `pll_cpu`；`scaling_driver=cpufreq-dt`、
  `scaling_governor=schedutil`、`scaling_min_freq=480000`、`scaling_max_freq=1512000`（9 档）。
- OPP 表仍有 5 档（600/792/1032/1296/1344 MHz）因变体查找未命中被丢弃；本次未新增 OPP 授权证据。
- SID `speed@0x00 = 0x6c00`、`calib@0x14 = 181113ab191b16bb`（4×12 bit 2835/2841/2838/2842），与历次相同；
  `sid-cell-report.json` 的 `source_sha256 = f8f55cc2…` 与上传的 `cpu-power-runtime.txt` 一致（该文件不含序列号）。
- `sunxi-thermal: avcc is not calibrated` 仍在，温度仅作诊断，不作为闭环 DVFS 依据。
- v3 的 getter 对账结论保留：480 MHz 窗口是读取瞬态，非真实钳制。

## 5. 运行时 FDT、内存、env：与 v3 一致

- 运行时 FDT 143,936 B / 573 nodes vs 打包 DTB 139,822 B / 573 nodes；差异为引导链对 `/chosen` 与
  `disp@01000000` 的属性级写入（`baseos,native-*`、`boot_disp*`、`boot_fb0` = 720×480@32bpp, stride 2880）。
- RAM 1 GiB，`MemTotal 996,700 kB`，46 段 memblock 预留，64 MiB CMA（`CmaTotal 65,536 kB`）；
  framebuffer `0x7bf24800`，`matches_disp_reserve = true`。
- env 分区 16 MiB，CRC 通过（`0xe3b6e401`，25 变量）。

## 6. 下一步（未闭合项）

识别 shipping Linux 4.9.170 cpufreq/OPP 二进制/源码并解释 `b1` 选择；确认
`dev_pm_opp_set_prop_name("b1")` 的调用与 speed NVMEM cell 消费方式；证明 BL31→BL33 的 CPU1–CPU3 状态与
定时器连续性；获取安全的只读原始 PMIC/CCU 证据；解决 AVCC/温度校准。CPU 性能服务保持 `POLICY_ONLY`、
`hardware_applied=0`，链接槽 CPU 性能行保持 display-only/unavailable，**不授权任何处理器间频率切换**。

## 7. 边界与脱敏

- `cmdline.txt` 与 `live-fdt.json` 中的设备序列号按既有惯例替换为 `[REDACTED]`（各 −18 B = 2×9 B）；
  脱敏后 `cmdline.txt` 的 SHA 与 v1/v2/v3 完全相同（`24b0148a…`）。
  `kernel-driver-provenance.txt` 经检查不含序列号，原样上传。
- `memory-map-report.json` 的 `fdt_sha256 = cbb0bf25…` 指向未脱敏原始 FDT，与上传的 `live-fdt.json`
  （`d485e28c…`）不同，这是预期差异。
- 本报告只覆盖一次文本侧捕获；未读取/写入任何 PMIC、PLL、时钟、热控寄存器，未改动 governor、频率上下限、
  电压或分区内容。**不构成**可安装性、硬件适配、OPP 授权或 shipping 二进制识别结论。
