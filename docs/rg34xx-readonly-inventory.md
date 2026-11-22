# RG34XX 只读启动链信息采集

## 用途

这个采集器为 Anbernic RG34XX（非 SP）收集现场启动链证据，供后续核对 BaseOS 的准备固件和启动交接研究。它不改写启动区，不安装 loader，不改变 U-Boot 环境，也不重启设备。

默认输出放在设备的 /tmp，避免把采集文件写进用于启动的 FAT 分区。输出目录可以通过第二个参数指定；指定其他目录会在该文件系统创建普通报告文件。

## 采集内容

| 文件 | 内容 |
|---|---|
| boot0-primary.bin | 原始 SD LBA 16 起的 64 KiB Boot0 副本 |
| boot0-secondary.bin | 原始 SD LBA 512 起的 64 KiB Boot0 副本 |
| toc1-package.bin | 原始 SD LBA 32800 起的 0x140000 字节 TOC1 包 |
| env-partition-first64k.bin | 可选：GPT 分区标签为 env 的分区起始 64 KiB |
| live-fdt.dtb | 内核提供 /sys/firmware/fdt 时保存的 live FDT |
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
- BaseOS gzip 启动包：manifest/kernel-gzip.json 记录 SHA-256 为 6c380adb6e4f16aa535da25f665706b0a6118b542947859aad5931f4baa9ac28。它不是原始厂商包的另一个别名；kernel_gzip.py 会修改 U-Boot 的 Android 压缩选择指令、TOC1 checksum，以及 DTB 中的 debug_mode 和 USB controller status，并要求 boot package 与 boot image 配套安装。

因此，现场 hash 应先与设备当前安装的 BaseOS 启动方案比对。仅仅不匹配其中一个值不能判定损坏；更高版本或其他启动方案可能有独立指纹。具体修改边界见 BaseOS 的 docs/12-kernel-gzip.md 和 tools/kernel_gzip.py。

## Windows 一键采集

在项目仓库根目录打开 PowerShell，连接 RG34XX 的 ADB 数据线后执行：

    powershell -ExecutionPolicy Bypass -File .\scripts\run-rg34xx-inventory.ps1

脚本会确认只连接了一台已授权的 ADB 设备、确认 ADB shell 为 root、读取 /proc/cmdline 并校验整盘设备与 root 分区一致，然后自动 push 采集器、运行只读采集、pull 报告、解析两份 Boot0 并解包 TOC1。默认结果保存在 build/rg34xx-inventory-时间戳/。多个 ADB 设备同时连接时用 -DeviceSerial 指定；root 参数不是 /dev 块设备时，在确认真实启动 SD 整盘后用 -BlockDevice 显式指定。

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

工具会验证 64 KiB eGON.BT0 标记、镜像长度及 Allwinner checksum，并输出 0x38、0x9f4、0xa74 三组 32-word DRAM 参数候选。0x38 主候选位于 Boot0 第一扇区，可能在 DRAM training 时变化，因此同时报告整段 SHA-256、第一扇区 SHA-256 和第一扇区之后的主体 SHA-256。它不会仅凭参数内容断言实际训练成功，也不会据此自动生成 DRAM 设置。


在项目仓库根目录执行：

    python3 tools/parse_h700_toc1.py ./rg34xx-readonly-inventory/toc1-package.bin -o ./rg34xx-readonly-inventory/toc1-extracted

程序会验证 TOC1 item 数量、表格边界、名称、偏移、长度、重叠与整体 SHA-256，并在 items/ 下提取各组件。manifest.json 会记录整个包与每个组件的 SHA-256。当前解析器验证 Allwinner 包头 magic、包长度、checksum stamp、MIE; 表标记、四个组件名、每项 368 字节、名称 64 字节、offset/size/encryption/type 字段和 IIE; 项尾标记；并验证 payload 不越界、不重叠。

它不验证厂商签名、不解析 Boot0 DRAM training 的语义，也不证明 payload handoff ABI。提取成功只表示容器布局通过边界检查。

## 对比与解释规则

1. TOC1 整包及内部 u-boot、monitor、dtb、dtbo 的 SHA-256 与 BaseOS 准备固件清单比对。
2. Boot0 的 primary/secondary 副本不要简单要求整段逐字节相同。BaseOS 研究记录指出 Boot0 会在启动时改写训练相关头部字段和 checksum；需要按 Boot0 的已知可变区域解释差异，同时核对可执行主体。
3. env 采集仅包含标签为 env 的分区起始 64 KiB，不代表完整分区镜像。后续如需更完整的环境分析，应先从 GPT 证据确定实际分区边界，再另行制定只读采集范围。
4. live FDT、/proc/iomem、/proc/meminfo 和设备树内存保留区共同用于判断安全 RAM 区域。不得只凭某个 Linux 入口地址推定空闲内存。
5. 当前 prepared hashes 是仓库准备输入指纹，不是本次测试机 live readback。只有采集报告与这些指纹完成差异解释后，才能更新目标机启动链基线。

## 安全边界

- 对启动 SD 的原始块设备只使用 dd if=... of=报告文件；不会向块设备发起写命令。
- 不执行 fw_setenv、flashcp、nand、sunxi-fel、reboot、poweroff 或任何 Boot0/TOC1/U-Boot 写入。
- 不尝试进入 Boot ROM 下载模式，也不改变设备环境变量。
- 脚本会在报告输出目录创建文件。默认 /tmp；不要把输出目录指定到 boot 分区或其他只读/重要分区。
- 若任何原始区间短读或输出路径已存在，脚本会报错退出，不会自动猜测替代偏移。

## 验证

主机 CI 对采集脚本运行 sh -n，静态检查所有 dd 输出都指向报告目录，并用纯合成数据测试 Boot0 checksum/DRAM 候选解析和 TOC1 的有效包、Allwinner checksum、错误 magic、项目数、重复名称、越界、重叠、非法名称及 IIE; 尾标记处理。它不需要真实设备固件，也不会向硬件写入。
