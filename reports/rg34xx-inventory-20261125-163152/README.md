# RG34XX/H700 只读盘点（2026-11-25 16:31，新增 display timing）

本目录是 `scripts/run-rg34xx-inventory.ps1` 在仓库根目录（无 `-OutputDirectory`，默认落 `build/rg34xx-inventory-<时间戳>`）
一次完整只读盘点的**文本 / JSON 侧产物**。原始二进制捕获（`boot0-primary/secondary.bin`、`toc1-package.bin`、
`env-partition.bin`、`live-fdt.dtb` 及 `toc1-extracted/items/*`，合计约 19 MB）留在采集机本地，未上传。

- 采集时间：设备本地 `2026-11-25 08:30:25 UTC`（= `2026-11-25 16:30:25 +08:00`），远程输出目录
  `/tmp/rg34xx-inventory-20261125-163152`
- 设备：`RG34xx` / `sun50iw9p1`（`compatible = allwinner,h616, arm,sun50iw9p1`），Linux `4.9.170 #19 SMP PREEMPT
  Tue May 26 13:58:08 CST 2026`，Linaro GCC 5.3-2016.05，ADB shell `uid=0`，单设备（序列号已脱敏）
- 采集链：`porting/linkslot-1.0.45-feasibility` @ `d7edaad`（`docs: map Allwinner vendor display mode-setting path`），
  采集器 `collect-rg34xx-readonly-inventory.sh` blob `8b6852d0…`（974 行），runner `run-rg34xx-inventory.ps1`
  blob `bf79922e…`
- 盘：`/dev/mmcblk0`，15,552,479,232 B，逻辑扇区 512，`root=/dev/mmcblk0p5`，`gpt=1`
- 性质：**只读**。设备端回执 `No write was issued to /dev/mmcblk0`；runner 汇总
  `The device-side collector only reads the source block device; output files were written under /tmp.`
  未调用厂商 ioctl/SMC，未写 PMIC/PLL/CCU/regulator/sysfs 节点。`not_installable`、`not_hardware_validated`

## 1. 产物（31 个文本 / JSON）

| 文件 | 字节 | SHA-256（前 16） |
| --- | ---: | --- |
| `cpu-power-runtime.txt` | 59,415 | `7496614cb63c34b5` |
| `live-fdt.json` | 38,941 | `cdb9d55351c43e55`（已脱敏） |
| `toc1-extracted/dtb-report.json` | 37,839 | `248bfceb6d447f4e` |
| `memory-map-report.json` | 16,774 | `dd64f67c2b40c33e` |
| `dmesg-hardware-lines.txt` | 11,156 | `2f58ca62aade8289` |
| `kernel-driver-provenance.txt` | 6,980 | `aac9e1551ed940f9` |
| `tee-ion-runtime.txt` | 4,858 | `a962619a3cb77876` |
| `interrupts.txt` | 4,386 | `ea09a609719931aa` |
| `boot0-primary.json` / `boot0-secondary.json` | 3,872 / 3,872 | `0e0c1f1ce04ab241` / `4370a68ae9e8bb40` |
| `env-report.json` | 3,684 | `437a08660ef2d0f2` |
| `sid-cell-report.json` | 3,003 | `5438d2bcc57a9168` |
| `debugfs-memory-reservations.txt` | 2,664 | `6d256713f030dd60` |
| `mounts.txt` | 2,462 | `03941a64aca1042f` |
| **`display-runtime.txt`** | **1,975** | `eb750634dcafc89d`（本轮新增） |
| `device-and-block-layout.txt` | 1,660 | `60b59c3275872230` |
| `toc1-extracted/manifest.json` | 1,498 | `d22b1c3ab53f02a1` |
| `meminfo.txt` | 1,009 | `cf8994fc85223922` |
| `iomem.txt` | 814 | `5eda970ab6d10be8` |
| `partitions.txt` | 823 | `6e9436ad6cb54ede` |
| `raw-sha256.txt` | 737 | `6d6f2162d83cb05e` |
| `cmdline.txt` | 683 | `63bad10808078b9a`（已脱敏） |
| `cpuinfo.txt` | 692 | `310475455c2bb599` |
| `gpt-fdisk.txt` | 690 | `0645d3e536c0879b` |
| `summary.txt` | 375 | `5341699f4d62c4d4` |
| `env-dd.txt` | 98 | `fe91a259b026b618` |
| `toc1-package-dd.txt` | 94 | `bb98776fc18fa194` |
| `boot0-primary-dd.txt` / `boot0-secondary-dd.txt` | 93 / 93 | `44cf653f2c208290` / `ac5a96a9c9c6aadf` |
| `page-size-source.txt` | 16 | `a2a558c8d2f4509d` |
| `page-size.txt` | 5 | `25ac87b7309b7946` |

## 2. 与 v1–v4 的可复现性

五个分区级捕获与 v1–v4 **逐字节相同**（`raw-sha256.txt`）：

| 捕获 | SHA-256 |
| --- | --- |
| `boot0-primary.bin` (131,072 B) | `bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f` |
| `boot0-secondary.bin` (131,072 B) | `5f143679e0d2c5293d22fee47a47ea6c17120c7a19e6007247c8b13710aa5ef9` |
| `toc1-package.bin` (1,310,720 B) | `e6c20e3a54c51b9336c798e3a4fa1ca6c45b28c6f70456e2db9c252aa4ba4604` |
| `env-partition.bin` (16,777,216 B) | `8dd964ef0d525071626040d7cdce6c16925aaa6108d35558a0a5f032592b78a2` |
| `live-fdt.dtb` (143,936 B) | `cbb0bf254f366a864a34c562602674813c6432832626f5005bab998494c1f7cc` |

`Boot0 copy comparison`：`different (may include documented training/header mutations)` —— 与历次一致，
两份 boot0 副本差异属既有已知现象，不是本次采集异常。

逐字节一致的文本产物：`kernel-driver-provenance.txt`、`memory-map-report.json`、`dmesg-hardware-lines.txt`、
`debugfs-memory-reservations.txt`、`partitions.txt`、`gpt-fdisk.txt`、`page-size.txt`（5 B）、
`boot0-primary.json`、`boot0-secondary.json`、`toc1-extracted/manifest.json`。

有差异的产物及其原因（均为预期，非采集失败）：

- `cpu-power-runtime.txt`（59,415 vs v4 的 59,386）：采样瞬间 CPU 频点不同（本轮多次读到 `1512000`，
  v4 读到 `1200000` / `1320000`），属运行时状态，不是驱动行为变化。
- `sid-cell-report.json`：仅 `source_sha256` 字段随上条变化，cell 内容与解析结论不变。
- `live-fdt.json` / `cmdline.txt`：除序列号脱敏外，`live-fdt.json` 体积随字符串表/脱敏占位变化。
- `env-report.json`、`*-dd.txt`、`interrupts.txt`、`meminfo.txt`：含 dd 计时行与运行时计数（IRQ 数、内存水位）。
- `toc1-extracted/dtb-report.json`：与 v4 相差 8 字节，属解析输出中的路径/时间字段。

`page-size.txt` 本轮为 `5` 字节正常值，来源 `page-size-source.txt = proc-self-smaps`（`getconf` 在
该 userspace 不可用时的既有回退），因此 `memory-map-report.json` 正常生成（上一轮 `page-size` 降级
导致 memory-map 被跳过的情形本轮未出现）。

## 3. 本轮新增：`display-runtime.txt`

上游 `d7edaad` 新增只读 display 采集段，内容为三条来源的原文摘要（未做任何写操作）：

- `/sys/class/disp/disp/attr/sys`：`de_rate 696000000 hz, ref_fps:59`；`mgr0: 720x480 fmt[rgb] cs[0x204]
  range[full] eotf[0x4] bits[8bits] iommu[1]`；`lcd output backlight(0) fps:60.2 720x480`；layer
  `fb[720,960] crop[0,0,720,480]`，`addr[ff800000,…]`。
- framebuffer 几何：`fb0/mode`、`modes = U:720x480p-59`、`virtual_size 720,960`、`stride 2880`、
  `bits_per_pixel 32`、`/proc/fb`。
- display 相关时钟摘要（保留缩进）：`pll_video2 / tcon_tv @297 MHz`、`pll_video0x4 @288 MHz /
  tcon_lcd`、`display_top @200 MHz`、`pll_video1`/`tcon_tv1`/`tcon_lcd1` 为 0。

这些是**运行时显示状态快照**，可作为后续 display/DRM 侧只读核对的基线；不构成任何时序、时钟或
背光写入的依据。

## 4. 脱敏

- 设备序列号在 `cmdline.txt`（`snum`、`androidboot.serialno`）与 `live-fdt.json`（内嵌 cmdline 字符串）
  中替换为 `[REDACTED]`，与 v2–v4 处理方式一致；脱敏后副本的字节数/SHA 见 §1。
- 未读取 `/sys/class/sunxi_info/sys_info` 或 `/dev/sunxi_soc_info`（采集器显式跳过，避免 chipid/序列号
  与潜在 CE 时钟/复位副作用）；`mac_addr` / `wifi_mac` / `bt_mac` 在 cmdline 中为空。
- 二进制捕获与恢复产物全部留在本地，仓库内只有有界文本与 JSON。

## 5. 边界

本轮只证实"这台设备当前的分区内容、FDT、boot0/TOC1、env、内存布局、CPUFreq/OPP 驱动来源指纹与
显示运行时状态"，**不**证明：`0x6c00 → b1` 的 OPP 授权、任何频率/电压/PMIC/CCU/PLL 写入安全、
BL31→BL33 多核与定时器连续性、独立温度通路。CPU 性能服务保持不可用、只读（`POLICY_ONLY`、
`hardware_applied=0`）。

## 6. 仍未闭合

- live `sunxi_get_soc_ver() & 7` 的实际取值（需独立只读路径，不从 `/sys` 触发）。
- 38 节点 OPP 表的 DTB 字面位图逐节点核对（需独立只读 DTB dump 与节点级比对）。
- 显示侧仅取得 vendor 快照，未与上游 DRM/TCON 路径做符号级比对。
