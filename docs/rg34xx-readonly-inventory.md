# RG34XX 只读启动链信息采集

## 用途

这个采集器为 Anbernic RG34XX（非 SP）收集现场启动链证据，供后续核对 BaseOS 的准备固件和启动交接研究。它不改写启动区，不安装 loader，不改变 U-Boot 环境，也不重启设备。

默认输出放在设备的 /tmp，避免把采集文件写进用于启动的 FAT 分区。输出目录可以通过第二个参数指定；指定其他目录会在该文件系统创建普通报告文件。

## 采集内容

| 文件 | 内容 |
|---|---|
| boot0-primary.bin | 原始 SD LBA 16 起采集的 128 KiB Boot0 区域；解析器按头部声明的 64/128 KiB 镜像长度截取 |
| boot0-secondary.bin | 原始 SD LBA 512 起采集的 128 KiB Boot0 区域；解析器按头部声明的 64/128 KiB 镜像长度截取 |
| toc1-package.bin | 原始 SD LBA 32800 起的 0x140000 字节 TOC1 包 |
| env-partition.bin | 可选：通过 sysfs 的精确 GPT PARTNAME=env 动态发现后读取完整分区（最大 64 MiB），不假设固定分区号 |
| live-fdt.dtb | 内核提供 /sys/firmware/fdt 时保存的 live FDT |
| live-fdt.json / toc1-extracted/dtb-report.json | 自动解析 live FDT 与 TOC1 静态 DTB 的 RAM、reserved-memory、面板、PMIC、USB 状态、bootargs、display framebuffer 的 boot_fb0 物理范围及 hash |
| env-report.json | 解析 key=value 变量并扫描 CRC32 头偏移/sector-aligned 环境长度；报告 CRC 覆盖长度与实际 GPT 分区大小。CRC 有效证明该映像内部一致，但不单独证明 U-Boot 已选中它 |
| cmdline.txt / cpuinfo.txt / meminfo.txt | 运行时内核启动参数、CPU 与内存信息 |
| iomem.txt / interrupts.txt | 内存资源范围与中断状态 |
| partitions.txt / mounts.txt | 块设备分区及挂载状态 |
| device-and-block-layout.txt | 设备树 model/compatible、块设备 sysfs 与分区标签 |
| gpt-sfdisk.txt 或 gpt-fdisk.txt | 系统提供 sfdisk 或 fdisk 时生成分区表文本 |
| dmesg-hardware-lines.txt | 与 DRAM、面板、PMIC、MMC、串口、保留内存相关的内核日志行 |
| raw-sha256.txt | 原始启动区文件 SHA-256 与两份 Boot0 比较结果 |

LBA 16、512、32800 和 TOC1 长度 0x140000 来自 BaseOS-MultiFrontend 的 H700 启动链采集/装载代码。采集器只适用于与这些布局一致的 RG34XX/BaseOS SD 镜像；如果现场读取结果和固件清单不一致，应先解释布局差异，不要直接把镜像视为损坏。

需要特别区分两种已记录的 TOC1 指纹：

- RG34XX 原始厂商包：BaseOS 的兼容性调查记录 SHA-256 为 56371d984e29e74db63009ce27f829eb0a55ce9007ee5e40c52479b22418af3。这个值对应 BaseOS H700 native-handoff/compatibility 研究中的厂商 TOC1 输入。
- BaseOS RG34XX gzip 启动优化的输入 TOC1：manifest/kernel-gzip.json 中的 package_sha256 为 6c380adb6e4f16aa535da25f665706b0a6118b542947859aad5931f4baa9ac28。该值是 tools/kernel_gzip.py 在任何修改之前要求匹配的原始输入包 hash，不是脚本生成的 gzip-patched 输出包 hash。脚本随后会修改 U-Boot 的 Android 压缩选择指令、相关 checksum，以及 DTB 中的 debug_mode 和五个 USB controller status；生成的包必须与对应 boot image 配套安装。

因此，现场 hash 应先与设备当前安装的 BaseOS 启动方案比对。仅仅不匹配其中一个值不能判定损坏；更高版本或其他启动方案可能有独立指纹。2026-11-22 首轮 RG34XX 实机采集结果和组件级解释见 [现场只读 Inventory 报告](rg34xx-live-inventory-2026-11-22.md)。具体修改边界见 BaseOS 的 tools/kernel_gzip.py。

## Windows 一键采集

在项目仓库根目录打开 PowerShell，连接 RG34XX 的 ADB 数据线后执行：

    powershell -ExecutionPolicy Bypass -File .\scripts\run-rg34xx-inventory.ps1

脚本会确认只连接了一台已授权的 ADB 设备、确认 ADB shell 为 root、读取 /proc/cmdline 并校验整盘设备与 root 分区一致，然后自动 push 采集器、运行只读采集、pull 报告、解析两份 Boot0、解包 TOC1，并用 FDT 检查器分别输出 live-fdt.json 与 TOC1 内 dtb-report.json。采集器会优先通过 sysfs 中的精确 PARTNAME=env 查找环境分区并读取完整分区（限制 64 MiB），不硬编码 p3；若找不到且存在 by-partlabel/env 链接，才使用该链接作为兼容回退。Windows runner 会自动运行 env 解析器并输出 env-report.json。默认结果保存在 build/rg34xx-inventory-时间戳/。多个 ADB 设备同时连接时用 -DeviceSerial 指定；root 参数不是 /dev 块设备时，在确认真实启动 SD 整盘后用 -BlockDevice 显式指定。

该快捷脚本要求 Windows PowerShell 5.1 或 PowerShell 7、Android platform-tools 和 Python 3。ADB shell 必须为 root；若设备 ADB 不是 root，请按下方方法在 root SSH shell 执行采集器。

## 在设备上运行

优先使用 ADB 把脚本临时传入 /tmp：

    adb push scripts/collect-rg34xx-readonly-inventory.sh /tmp/rg34xx-inventory.sh

设备 ADB shell 具有 root 权限时：

    adb shell "sh /tmp/rg34xx-inventory.sh /dev/mmcblk0 /tmp/rg34xx-readonly-inventory"

然后把整个输出目录拉回电脑：

    adb pull /tmp/rg34xx-readonly-inventory ./rg34xx-readonly-inventory

也可以从 root SSH shell 运行同一脚本：

    sh /tmp/rg34xx-inventory.sh /dev/mmcblk0 /tmp/rg34xx-readonly-inventory

如果不提供第一个参数，脚本会尝试从 /proc/cmdline 的 root=/dev/... 推导整盘设备；若系统使用 PARTUUID 等 root 形式，必须显式传入整盘设备，且先人工确认它确实是当前 RG34XX 启动 SD 卡。

## 在电脑上解析 Boot0 与 TOC1

分别检查两份 Boot0 副本：

    python3 tools/inspect_h700_boot0.py ./rg34xx-readonly-inventory/boot0-primary.bin -o ./rg34xx-readonly-inventory/boot0-primary.json
    python3 tools/inspect_h700_boot0.py ./rg34xx-readonly-inventory/boot0-secondary.bin -o ./rg34xx-readonly-inventory/boot0-secondary.json

工具会验证 eGON.BT0 标记、头部声明的镜像长度及 Allwinner checksum。采集器固定读取 128 KiB；解析器按头部声明长度截取并计算 checksum，当前测试覆盖 64 KiB 和 128 KiB 两种镜像。随后输出 0x38、0x9f4、0xa74 三组 32-word DRAM 参数候选。0x38 主候选位于 Boot0 第一扇区，可能在 DRAM training 时变化，因此同时报告整段 SHA-256、第一扇区 SHA-256 和第一扇区之后的主体 SHA-256。0x9f4 和 0xa74 位于第一扇区之后；若它们在两份副本间不同，不能仅凭“首扇区会变化”解释，必须结合两份 JSON 的主体 SHA-256 与原始参数 words 单独审查。工具不会仅凭参数内容断言实际训练成功，也不会据此自动生成 DRAM 设置。


在项目仓库根目录执行：

    python3 tools/parse_h700_toc1.py ./rg34xx-readonly-inventory/toc1-package.bin -o ./rg34xx-readonly-inventory/toc1-extracted

程序会验证 TOC1 item 数量、表格边界、名称、偏移、长度、重叠与整体 SHA-256，并在 items/ 下提取各组件。manifest.json 会记录整个包与每个组件的 SHA-256。当前解析器验证 Allwinner 包头 magic、包长度、checksum stamp、MIE; 表标记、四个组件名、每项 368 字节、名称 64 字节、offset/size/encryption/type 字段和 IIE; 项尾标记；并验证 payload 不越界、不重叠。

它不验证厂商签名、不解析 Boot0 DRAM training 的语义，也不证明 payload handoff ABI。提取成功只表示容器布局通过边界检查。

## 对比与解释规则

1. TOC1 整包及内部 u-boot、monitor、dtb、dtbo 的 SHA-256 与 BaseOS 准备固件清单比对。
2. Boot0 的 primary/secondary 副本不要简单要求整段逐字节相同。BaseOS 研究记录指出 Boot0 会在启动时改写训练相关头部字段和 checksum；需要按 Boot0 的已知可变区域解释差异，同时核对可执行主体。
3. env 采集按 GPT PARTNAME=env 动态定位并读取完整分区（最多 64 MiB）；解析器只在标准 CRC32 与候选头偏移/sector-aligned 环境长度匹配时标记 CRC valid。注意 GPT env 分区大小不必等于 CRC 有效环境映像长度；本机实测为 16 MiB p3 内含 128 KiB CRC 有效映像。CRC 有效也不单独证明 U-Boot 已选中它。
4. live FDT、/proc/iomem、/proc/meminfo、boot_fb0/display reserve 和设备树保留区共同用于判断安全 RAM 区域。解析器会比较 boot_fb0 的 base/stride/height 计算范围与 disp_reserve=size,base 参数；不得把 framebuffer 当作 payload 可用内存，也不得只凭某个 Linux 入口地址推定空闲内存。
5. 当前 prepared hashes 是仓库准备输入指纹，不是本次测试机 live readback。只有采集报告与这些指纹完成差异解释后，才能更新目标机启动链基线。

## 安全边界

采集报告中的 /proc/cmdline、live FDT bootargs 和 env-partition.bin 可能包含设备序列号、MAC 地址、私有启动变量或其他设备专属信息。默认只保存在本机 build/ 目录；不要将原始 env/FDT/命令行文件直接提交到公开仓库或公开 Issue。需要公开分享时先检查并脱敏，原始数据仅在可信的私有渠道传递。

- 对启动 SD 的原始块设备只使用 dd if=... of=报告文件；不会向块设备发起写命令。
- 不执行 fw_setenv、flashcp、nand、sunxi-fel、reboot、poweroff 或任何 Boot0/TOC1/U-Boot 写入。
- 不尝试进入 Boot ROM 下载模式，也不改变设备环境变量。
- 脚本会在报告输出目录创建文件。默认 /tmp；不要把输出目录指定到 boot 分区或其他只读/重要分区。
- 若任何原始区间短读或输出路径已存在，脚本会报错退出，不会自动猜测替代偏移。

## 验证

主机 CI 对采集脚本运行 sh -n，Windows runner 进行 PowerShell 语法检查；静态检查所有 dd 输出都指向报告目录，并用纯合成数据测试 Boot0 checksum/DRAM 候选解析、TOC1 容器边界与 checksum、FDT 的内存/reserved-memory/面板/PMIC/USB/bootargs 解析，以及 U-Boot env 的 key=value 提取、CRC 覆盖范围识别和损坏 CRC 拒绝。它不需要真实设备固件，也不会向硬件写入。
