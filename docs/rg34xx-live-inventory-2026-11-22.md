# RG34XX 首轮实机只读 Inventory（2026-11-22）

## 1. 采集范围与结论

目标设备为 Anbernic RG34XX（非 SP）。采集时间为 2026-11-22 18:15:51（设备本地 UTC+8），源块设备为 /dev/mmcblk0。主机侧报告目录为 build/rg34xx-inventory-20261122-181713。

本次只读采集没有向 SD 卡发送写命令；Boot0、TOC1、运行时 FDT、内核内存映射及 GPT 均来自本次现场读回。设备块节点的 read-only 标志为 0，表示内核没有把该块设备强制设置成只读；这不改变本次采集命令只执行读取的事实。

**结论：** 已确认这台 RG34XX 实机使用 4 核 Cortex-A53、rg34xx_v1 720×480 面板、AXP2202 PMIC，运行时系统识别到 1 GiB RAM。TOC1 内的 vendor monitor 与 BaseOS RG34XX 已固定的 monitor 指纹完全一致；U-Boot 和 DTB 呈现 BaseOS gzip 启动优化的特征，但整个现场 TOC1 不是已知准备镜像的精确指纹。Boot0 两个位置读出的镜像长度、debug 字段和可执行主体也不同。当前还不能确定 BROM 实际采用哪一份 Boot0、U-Boot 环境如何解释分区差异、monitor 的安全内存边界或可复用的裸机 AArch64 入口 ABI。

## 2. 设备与运行时平台

| 项目 | 现场证据 |
|---|---|
| SoC / DT compatible | DT root model 为 sun50iw9；compatible 为 allwinner,h616、arm,sun50iw9p1。这是 H700 所属的通用 SoC 家族标识，不是 RG34XX SKU 名称 |
| 机型识别 | LCD 节点 lcd_driver_name 为 rg34xx_v1；非 SP 面板驱动名与 720×480 时序和 RG34XX 非 SP 基线吻合 |
| CPU | 4 个 CPU；CPU architecture 8，CPU part 0xd03（Cortex-A53） |
| PMIC | FDT compatible 包含 x-powers,axp2202；内核日志注册 axp2202-pek 与 axp2202_battery |
| Linux | 4.9.170，aarch64；构建时间 2026-05-26 13:58:08 CST |
| U-Boot 标识 | 内核命令行 uboot_message 为 2018.05，构建时间 2026-05-26 13:55:09 |
| SD | /dev/mmcblk0，30,375,936 个 512-byte sectors，15,552,479,232 bytes；内核日志识别为 14.5 GiB SDHC |

注意：root DT compatible 的 allwinner,h616 / sun50iw9p1 不能单独证明产品型号。识别 RG34XX 非 SP 主要依赖 rg34xx_v1 面板节点；该结论与已知硬件基线一致，但不等价于读取主板丝印或硬件 revision。

## 3. DRAM 与运行时内存地图

| 来源 | 结果 |
|---|---|
| TOC1 内静态 DTB 的 /memory | 0x40000000 起，大小 0x20000000（512 MiB） |
| 内核收到的 live FDT 的 /memory | 0x40000000 起，大小 0x40000000（1 GiB） |
| /proc/iomem | System RAM 0x40000000–0x7fffffff（1 GiB） |
| /proc/meminfo | MemTotal 996700 kB；MemFree 927436 kB；MemAvailable 941056 kB |
| CMA | cma=64M；CmaTotal 65536 kB，CmaFree 64416 kB |
| 显示预留参数 | bootargs 含 `disp_reserve=1382400,0x7bf24800` |
| Live FDT boot_fb0 | `/soc@03000000/disp@01000000` 节点值 `7bf24800,2d0,1e0,20,b40,0,0,2d0,1e0`，即 base 0x7bf24800、720×480、32 bpp、stride 2880 bytes |
| 显示 framebuffer 物理范围 | stride × height = 1,382,400 bytes = 0x151800；范围 `[0x7bf24800, 0x7c076000)`，与 `disp_reserve=size,base` 参数完全吻合 |
| FDT reserved-memory | live FDT 与 TOC1 静态 DTB 均未发现标准 `/reserved-memory` 节点；但 live FDT 的 `boot_fb0` 和 bootargs 已明确给出显示 framebuffer 保留区 |

静态 TOC1 DTB 与 live FDT 的 RAM size 不同；live FDT、/proc/iomem 和 MemTotal 相互印证 1 GiB 运行时容量。可以确认内核阶段看到的是 1 GiB RAM，但目前没有证据定位是哪一个启动阶段把静态 DTB 的 512 MiB 修改为 1 GiB。

这不是可直接使用的裸机 RAM map。现在已能明确排除 live FDT 标记的 framebuffer 区间 `[0x7bf24800, 0x7c076000)`，以及当前 Linux 的 kernel code/data 子范围；`CmaTotal=64 MiB` 只说明 CMA 总量，第三轮尚未取得其物理 base，因此不能声称已排除某个 64 MiB CMA 地址区间。monitor/secure world、CMA 实际位置、其他 DMA/carveout 和 SPL/loader 自身空间仍未完整确认。没有标准 `/reserved-memory` 节点也不代表这些区域不存在。任何 payload 链接地址都必须等到完整占用图和 monitor 约束查明后再确定。

## 4. Boot0 两个位置的现场读回

采集器从 LBA 16 和 LBA 512 各读取 256 sectors（128 KiB）。解析器按每份镜像头部的 image_size 字段截取有效镜像后再验证 Allwinner checksum。

| 项目 | LBA 16（primary capture） | LBA 512（secondary capture） |
|---|---|---|
| 原始采集长度 | 128 KiB | 128 KiB |
| 头部声明有效长度 | 128 KiB | 64 KiB |
| checksum | 通过 | 通过 |
| debug byte（0x34） | 3 | 0 |
| 有效镜像 SHA-256 | bf886995745a0eb9b5030296f9f6163035e164b3a82183716e67ff876319289f | 4701a85e4db7e8339b91b92f691a83cc954ec63a09369087c2e41d80eafa9f5d |
| 有效镜像首 512 字节之后 SHA-256 | ff847db96640d09d45c32b1babd77a60b270589aa015160d2740924c9ede4b50 | 1bdc26d91460255019a4b06bee804c4d63cca2a94b15a3e366c31d10146a5608 |
| 0x38 主参数 | LPDDR4，672 MHz | LPDDR4，672 MHz |
| 0x9f4 候选 | 全零 | LPDDR3，672 MHz |
| 0xa74 候选 | 全零 | LPDDR4，672 MHz |

LBA 512 的文件在磁盘上仍是 128 KiB 原始采集；其头部只声明 64 KiB 有效镜像，因此解析器生成的 SHA-256 是对前 64 KiB 计算的。完整 128 KiB 原始文件 SHA-256 为 5f143679e0d2c5293d22fee47a47ea6c17120c7a19e6007247c8b13710aa5ef9。两种 hash 对应不同长度，不应混用。

两份镜像都能通过自身 checksum，0x38 参数的 DRAM 类型与频率相同；但 debug byte、有效长度、首扇区 tuning words 和首扇区之后的主体 hash 均不同。尤其主体 hash 不同，因此不能把两份读回简单解释成“只有 Boot0 training 改写了第一扇区”。LBA 512 的备用候选参数块有有效内容，而 LBA 16 的对应位置为零。尚未通过本机启动阶段 trace 直接观察 BROM 的选择，也不能仅凭候选块推断 active DRAM profile 或训练成功状态；H616 BROM 的公开搜索顺序与这两个偏移的对应关系见下文 4.1。

### 4.1 BROM 的 SD Boot0 候选位置

现场两个有效 Boot0 恰好位于 LBA 16（8 KiB）与 LBA 512（256 KiB）。H616-specific U-Boot SPL 修复记录说明，Allwinner H616 BROM 会检查 SD/eMMC 的 sector 16，也会检查 sector 512（256 KiB）；较早 SoC 的高位候选才是 sector 256（128 KiB）。该代码变更专门针对 `CONFIG_MACH_SUN50I_H616`，与现场 DT compatible 的 `allwinner,h616` 家族标识相符，见 [U-Boot H616 high MMC offset patch](https://lists.denx.de/pipermail/u-boot/2024-May/553283.html)。

KNULLI 的 H700 固件提取说明也记载 stock Boot0 位于 8 KiB，而 256 KiB 是另一种常见位置；其页面覆盖 RG35XX Plus/H/2024，而不是本机型，故作为 H700/H616 位置参考而非 RG34XX 本机源代码证据，见 [H700 firmware extraction](https://knulli.org/guides/firmware-extraction/h700-firmware-extract/)。

据此，现场 LBA 16 的 128 KiB Boot0 是**最可能的正常启动候选**，LBA 512 的 64 KiB Boot0 是 H616 BROM 高位候选位置上的另一份有效映像。两份 checksum 均通过，且头部声明长度/主体 hash 不同；公开 BROM 资料和偏移吻合提高了这个判断的可信度，但本轮没有 BootROM 读取轨迹或启动阶段串口记录，因此仍将“本次启动实际选择 LBA 16”标记为高置信推断而非直接观测。

### 4.2 与 BaseOS RG34XX 固定 DRAM profile 的逐 word 比较

将现场 Boot0 JSON 中的三组 32×32-bit 参数，逐项与 BaseOS-MultiFrontend `experiments/h700-boot-speed/evidence/compatibility.json` 的 `targets.rg34xx.dram_candidates` 比较：

| 现场位置 | 比较对象 | 匹配结果 |
|---|---|---|
| LBA 16，0x38 | RG34XX prepared 主 LPDDR4/672 MHz | **32/32 words 完全一致** |
| LBA 512，0x38 | RG34XX prepared 主 LPDDR4/672 MHz | 30/32 一致；仅 word[26]、word[29] 不同 |
| LBA 512，0x9f4 | RG34XX prepared 备用 LPDDR3/672 MHz | **32/32 words 完全一致** |
| LBA 512，0xa74 | RG34XX prepared 备用 LPDDR4/672 MHz | **32/32 words 完全一致** |

LBA 512 的主参数块与 prepared 主参数块仅有两处差异：

- word[26]：现场 `0x40808080`，prepared `0x3e808080`。
- word[29]：现场 `0x06070707`，prepared `0x06070706`。

这比只判断 LPDDR 类型更有信息量：LBA 16 的主参数块与 RG34XX 固定 profile 完全一致；LBA 512 则保留了两组与该 profile 完全一致的 LPDDR3/LPDDR4 备用 bank，但自身主块的两个 tuning word 不同。结合 H616 BROM 对 LBA 16、LBA 512 的候选搜索位置，LBA 16 作为本机 stock 启动 Boot0 的判断得到进一步支持。

证据边界不变：逐 word 相同证明参数块内容匹配，不直接证明 BROM 这次实际读取了哪份，也不证明 training 最终采用的 bank、rank/width/size 或可供裸机使用的 RAM 边界。目标机启动时的 Boot0 读取轨迹仍未采集。

## 5. TOC1 与组件指纹

现场 TOC1 位于 LBA 32800，大小 0x140000（1,310,720 bytes），Allwinner checksum 与四项目录边界校验通过。

| 组件 | 大小 | SHA-256 |
|---|---:|---|
| u-boot | 1,048,576 | 4d60a22ff13eac00dba89e0ee7df52a49852c0d9d856bec545ce40126191d127 |
| monitor | 107,216 | 676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31 |
| dtbo | 2,336 | 4bee281701610f96dcbdd1d38cb8df28386224d8c68b668b9a7a7a9df8e73ec2 |
| dtb | 144,384 | c28c87380ffab00148bf4dc457e70b8c6c576470dcfbc53df64addddf89603a1 |

TOC1 整包 SHA-256：

e6c20e3a54c51b9336c798e3a4fa1ca6c45b28c6f70456e2db9c252aa4ba4604

### 5.1 Monitor

现场 monitor SHA-256 与 BaseOS RG34XX 固件资料中固定的 vendor monitor SHA-256 完全一致。这是目前最强的组件复用证据；它只确认 monitor 二进制身份，不代表其运行时状态、可调用服务或 payload handoff ABI 已经确认。

现场 monitor 组件还包含如下静态字符串特征：

- 构建标记：Built : 16:02:57, 2024-06-26。
- 平台源文件路径：plat/sun50iw9p1/bl31_sunxi_setup.c；并包含 allwinner,h616、allwinner,t507 平台标识。
- 存在 BL31 image-entry / context management 字符串，以及 PSCI CPU_ON、CPU_OFF、CPU_SUSPEND、System Suspend 等处理路径名称。
- 同一二进制包含 “No TSP provided by BL2 boot loader” 和 “Platform has not exported a PSCI System Off/Reset hook” 等诊断字符串。

这些是 monitor 二进制内部的编译/诊断字符串，支持其属于 Allwinner sun50iw9p1 BL31/ATF monitor 的判断；它们不等于本次启动日志实际打印了这些 warning，也不能仅凭字符串确认 TSP、PSCI hook 的现场运行状态或 vendor SMC 服务表。当前只确认二进制 hash 与已知 BaseOS RG34XX monitor 相同。

### 5.2 U-Boot

现场 U-Boot SHA-256 与 BaseOS manifest/kernel-gzip.json 所记录的 RG34XX 原始 U-Boot 输入 SHA-256 不同。其偏移 63128（0xF698）处包含 40f20123，紧邻上下文与 BaseOS kernel_gzip.py 所修改的压缩选择指令位置一致。将该指令恢复为原始字节并重算 U-Boot checksum 后，U-Boot hash 仍不等于已固定的原始输入 hash。因此只能判断它具有该优化补丁的特征，不能说它是已知准备镜像中的精确 U-Boot 文件。

### 5.3 DTB 与 gzip 优化特征

TOC1 内 DTB 的 debug_mode 为 0；EHCI1、OHCI1、OHCI2、EHCI3、OHCI3 的 status 为 late，EHCI2 仍为 okay。这些字段与 BaseOS kernel_gzip.py 所实施的调整一致。静态 DTB 声明 512 MiB RAM，而 live FDT 声明 1 GiB，进一步说明最终送入内核的 FDT 并非简单原样等同于 TOC1 内 DTB。

### 5.4 必须更正的旧 hash 解释

BaseOS 的 manifest/kernel-gzip.json 中 RG34XX 的 6c380adb6e4f16aa535da25f665706b0a6118b542947859aad5931f4baa9ac28，是 tools/kernel_gzip.py 在开始修改之前要求匹配的**输入 TOC1 包 hash**。脚本会在其基础上改写 U-Boot 压缩选择指令、相关 checksum、DTB debug_mode 和五个 USB controller 状态。因此，6c380... 不能称为 gzip-patched 输出包 hash。

另一个历史记录中的 56371d984e29e74db63009ce27f829eb0a55ce9007ee5e40c52479b22418af3，是 BaseOS 兼容性/原始 vendor package 记录中的指纹。现场 TOC1 的 e6c20... 与这两个 hash 都不相同。现场包 checksum 有效且组件结构完整；指纹不匹配只能说明它是另一份现场固件组合，不能据此判定损坏。

### 5.5 U-Boot 二进制中的默认/维护模式字符串

对提取的 1 MiB U-Boot flat binary 执行只读 strings 扫描，找到以下内嵌字符串（偏移为组件文件内偏移）：

- 0x7df4f：`bootcmd=run distro_bootcmd`
- 0x7df6a：`bootdelay=2`
- 紧邻默认串还出现 `baudrate=115200`
- 0x99fd3 起出现另一组维护/刷机测试相关字符串：`bootdelay=0`、`bootcmd=run sunxi_sprite_test`、`sunxi_sprite_test=sprite_test read`，其上下文包含 sunxi sprite、BOOTPKG、UBOOT 更新/校验提示。

这些字符串能说明当前 U-Boot 二进制包含通用 distro boot 默认项和 sunxi sprite 维护路径，但不能证明启动时实际采用哪组环境。第三轮已确认 p3 中存在 CRC 有效且与 BaseOS prepared env 区域逐字节一致的环境数据；该有效映像长度为 128 KiB。也没有从字符串扫描中恢复出完整 distro_bootcmd 子命令链，因此不能据此确认该 fallback 路径实际被采用。

### 5.6 目标 U-Boot 中的启动介质/分区更新函数线索

对本轮实际读回的 U-Boot 组件（1 MiB，SHA-256 `4d60a22ff13eac00dba89e0ee7df52a49852c0d9d856bec545ce40126191d127`）再次按 NUL 终止 ASCII 字符串扫描，发现以下目标二进制自身的字符串（偏移为 U-Boot 组件内偏移）：

| 偏移 | 字符串 |
|---:|---|
| 0x9a814 | `bootcmd` |
| 0x9a81c | `setargs_mmc` |
| 0x9a828 | `setargs_nand` |
| 0x9a835 | `setargs_nor` |
| 0x9a841 | `update bootcmd` |
| 0x9a851 | `can't find valid mmc %d` |
| 0x9a8d9 | `root_partition` |
| 0x9a90a | `nand0p%d` |
| 0x9a913 | `mtdblock%d` |
| 0x9a91e | `mmcblk0p%d` |
| 0x9a929 | `partition_sets is too long, please reduces partition name` |
| 0x9a964 | `%s@%s:` |
| 0x9a96b | `/dev/%s` |
| 0x9a973 | `set root to %s` |
| 0x9a996 | `mmc_root` |
| 0x9a99f | `update part info` |
| 0x9bb65 | `run setargs_mmc boot_normal` |
| 0x9bb81 | `run setargs_nand boot_normal` |

其中 `update bootcmd`、两套 setargs 字符串和 mmc 存储识别字符串与 Allwinner Brandy 2.0 的 `sunxi_update_bootcmd()` 路径吻合；`partition_sets is too long`、`%s@%s:`、`set root to %s`、`mmc_root` 和 `update part info` 与 `sunxi_update_partinfo()` 的动态别名/root 分区生成路径吻合。目标 U-Boot 的这些字符串比仅引用其他板型的参考代码更直接，说明相关更新逻辑极可能被编译进本机 U-Boot。

结合 CRC 有效的 p3 环境中的 `bootcmd=run setargs_nand boot_normal`、`setargs_mmc` 定义，以及 Linux runtime `root=/dev/mmcblk0p5`、`partitions=...appfs@mmcblk0p6:UDISK@mmcblk0p7`，可以形成一致解释：U-Boot 在 SD 启动时执行启动介质适配，将 `setargs_nand` 路径切换到 `setargs_mmc`，并根据其分区元数据动态构造 root 设备和 `partitions=` 逻辑别名。

**证据边界：** 字符串扫描能证明目标 binary 含有与这些函数相匹配的代码路径/诊断常量；完整 p3 中的环境映像 CRC 也已验证。但仍没有反汇编调用图或串口执行日志证明该次冷启动确实调用了函数。因此这是“目标 binary 静态特征 + CRC 有效的持久化 env + runtime cmdline 相符”的强支持证据，不应写成已经捕获到函数执行 trace。

## 6. 分区表、逻辑分区别名与运行时挂载

本次 sysfs 和 GPT 工具输出一致，当前 GPT 为：

| 分区 | GPT 名称 | 起始 sector | sector 数 | 大小 |
|---|---|---:|---:|---:|
| p1 | special | 73728 | 131072 | 64 MiB |
| p2 | boot-resource | 204800 | 65536 | 32 MiB |
| p3 | env | 270336 | 32768 | 16 MiB |
| p4 | boot | 303104 | 131072 | 64 MiB |
| p5 | rootfs | 1482752 | 1048576 | 512 MiB |
| p6 | UDISK | 2531328 | 262144 | 128 MiB |
| p7 | primary | 2793472 | 27582461 | 约 13.1 GiB |

/proc/cmdline 的 partitions= 字符串把逻辑别名 appfs 映射到 mmcblk0p6、把 UDISK 映射到 mmcblk0p7；运行时挂载表显示 p6 是 ext4 并挂载到 /data，p7 是 vfat 并挂载到 /mnt/sdcard。这个运行时用途与 appfs/UDISK 逻辑分工相符。GPT 的 PARTNAME 字段则分别是 p6=UDISK、p7=primary；GPT 分区名元数据与内核 bootargs 中的逻辑别名并非同一个命名来源，因此不能把这两个字符串的差异直接判定为分区错位或 GPT 损坏。当前 root=/dev/mmcblk0p5 与 p5=rootfs 一致。 Allwinner Brandy 2.0 参考函数 `sunxi_update_partinfo()` 根据启动分区表的 `info.name` 和实际分区索引生成 `partitions=`，并设置 `mmc_root`；这说明逻辑别名是 U-Boot 的分区映射结果，不等同于 GPT header 的 PARTNAME。参考代码见 [board_helper.c 环境与分区更新说明](https://manuals.plus/m/35d20341b018e9f5bc7541f9a23512fbf0ebe3a5ee5a064ecb8dd1b44dee8317.pdf)。该文档对应 T113 Brandy 2.0 SDK，作为同一厂商 BSP 代码路径的参考，不直接证明 RG34XX binary 的每个函数版本。

fdisk 输出的磁盘容量文字为 2544M，与块设备 sysfs 的 30,375,936 sectors（约 14.5 GiB）明显不符；分区 sector 范围与 sysfs 一致。后续容量判断应使用块设备 sector 数和 sysfs，而不要采信这条 fdisk 容量文字。

启动日志还显示系统挂载文件系统前已经发生过 ext4 journal recovery：p5 rootfs 输出 `recovery required on readonly filesystem`、`write access will be enabled during recovery`，随后 `recovery complete` 并挂载；p6 `/data` 也报告 `recovery complete` 后挂载。因此本轮 inventory collector 对源块设备只执行读取，但**不能把设备从冷启动到采集结束的整个过程描述为零写入**：Linux 启动阶段至少对 p5 执行了 journal recovery 写操作；p6 也发生了 journal recovery，日志没有单独说明是否产生了写入。

另外，启动 dmesg 出现 `FAT-fs (mmcblk0p7): Volume was not properly unmounted. Some data may be corrupt. Please run fsck.`。这只能说明该 FAT 卷被内核判断为未正常卸载/dirty 状态，不等于已证明文件内容损坏。本轮没有运行 fsck 或额外写入修复；p7 挂载 `/mnt/sdcard`，若要检查应先备份重要 ROM/用户数据、正常卸载后再从电脑侧进行文件系统检查。

第二轮通过 sysfs 的 PARTNAME=env 采集到 p3 首 64 KiB，文件 SHA-256 为 `0cbb51563bc7dd94827392f42295a7f1623cf87254e056eee9469617a6c27e36`。当时只能读取变量前缀，无法验证完整 CRC。第三轮已完成整个 16 MiB p3 的只读采集，并在下文给出完整 CRC 与 prepared 区域 hash 的核对结果。

### 6.1 第二轮 env 前缀解析（2026-11-22 18:56:51）

第二轮采集成功发现 GPT 的 `PARTNAME=env` 并读取了首 64 KiB。环境区片段以 little-endian CRC 头字段 `0xe3b6e401` 开始，偏移 4 的第一个数据字节为 `0x00`，后面是双 NUL 终止的 key=value 字符串。第三轮确认 CRC 覆盖范围从偏移 4 开始，因此该 `0x00` 是环境数据首字节，不是独立 flags 字段。提取到的变量包括：

- `mmc_root=/dev/mmcblk0p5`、`nand_root=/dev/nand0p4`；
- `setargs_mmc`：以 `mmc_root` 生成 bootargs，包含 `rootwait quiet splash`；
- `setargs_nand`：以 `nand_root` 生成 bootargs；
- `boot_normal=sunxi_flash read 45000000 boot;bootm 45000000`；
- `boot_recovery=sunxi_flash read 45000000 recovery;bootm 45000000`；
- `boot_fastboot=fastboot`；
- `bootdelay=0`；
- `bootcmd=run setargs_nand boot_normal`。

现场 Linux `/proc/cmdline` 的 root、rootwait、quiet、splash 与 `setargs_mmc` 展开结果一致，而非 `setargs_nand`。这与 Allwinner vendor U-Boot 的常见 `sunxi_update_bootcmd()` 行为一致：当启动介质为 SD/eMMC 时，将 bootcmd 中的 `setargs_nand` 替换为 `setargs_mmc`。Allwinner Brandy 2.0 的参考实现见 [board_helper.c 行为说明](https://manuals.plus/m/35d20341b018e9f5bc7541f9a23512fbf0ebe3a5ee5a064ecb8dd1b44dee8317.pdf)。KNULLI 的 [H700 firmware extraction](https://knulli.org/guides/firmware-extraction/h700-firmware-extract/) 页面列出的机型是 RG35XX Plus、RG35XX H、RG35XX 2024，并提供 `mkenvimage -s 0x1000000` 的 16 MiB env.img 构建示例；这只是那些机型的参考 recipe，不能代替 RG34XX 的现场 CRC 检查。第三轮已证明 RG34XX 的 16 MiB p3 分区内实际只有一个 CRC 匹配的 128 KiB 环境映像。目标 U-Boot 仍缺串口执行 trace，因此介质自动切换结论是强静态支持，而非直接执行记录。

`boot_normal` 中的 `0x45000000` 是把名为 `boot` 的启动分区读入 RAM 后交给 `bootm` 处理的 Android boot image 暂存地址，不是 Linkslot 裸机 payload 的入口地址。当前分区 p4 的 GPT PARTNAME 为 boot；不能把这个 staging 地址改作裸机入口。

KNULLI 页面中的 `mkenvimage -s 0x1000000` 是该页面所列 RG35XX Plus/H/2024 机型的 16 MiB env.img 构建示例；该页面没有列出 RG34XX，不能将其当作 RG34XX 的 ENV_SIZE 证据。本机 p3 的 16 MiB 是 GPT 分区尺寸。第三轮完整读回表明两者不能等同：本机 CRC 只在 data offset 4、总映像长度 0x20000（128 KiB）时匹配。

本次采集的 GPT p6 PARTNAME=UDISK、p7 PARTNAME=primary；但 /proc/cmdline partitions= 将逻辑别名 appfs 指向 p6、UDISK 指向 p7。运行时 p6 是 ext4 并挂载 /data，p7 是 vfat 并挂载 /mnt/sdcard，实际用途与 appfs/UDISK 分工相符。GPT PARTNAME 与 U-Boot 生成的逻辑别名不是同一命名字段，因此这不是已经证实的分区错位或 GPT 损坏。Allwinner Brandy 2.0 的参考代码会按分区信息生成 partitions= 逻辑名称，见同一份 [board_helper.c 说明](https://manuals.plus/m/35d20341b018e9f5bc7541f9a23512fbf0ebe3a5ee5a064ecb8dd1b44dee8317.pdf)。

### 6.2 /proc/partitions 单位与 BaseOS prepared GPT 对照

需要区分两种容量单位：

- `/sys/class/block/mmcblk0pN/size` 和 GPT 的 sector 数以 512-byte sector 计数。
- `/proc/partitions` 的 `#blocks` 以 1 KiB block 计数。

因此，`/proc/partitions` 中 p6 的 131072 blocks 对应 262144 个 512-byte sectors，即 128 MiB；这与 GPT fdisk 和 sysfs 读数一致，并非 64 MiB。当前 GPT 表中的 p6 数值为 262144 sectors / 128 MiB。

将本轮 live GPT 与 BaseOS-MultiFrontend 的 [manifest/prepared/rg34xx.json](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/manifest/prepared/rg34xx.json) 对照，p1–p4 的起始位置和长度完全相同，但 p5–p7 几何明显不同：

| 分区 | prepared 起始 sector | prepared sector 数 | live 起始 sector | live sector 数 | live 大小 |
|---|---:|---:|---:|---:|---:|
| p1 special | 73728 | 131072 | 73728 | 131072 | 64 MiB |
| p2 boot-resource | 204800 | 65536 | 204800 | 65536 | 32 MiB |
| p3 env | 270336 | 32768 | 270336 | 32768 | 16 MiB |
| p4 boot | 303104 | 131072 | 303104 | 131072 | 64 MiB |
| p5 rootfs | 434176 | 14673920 | 1482752 | 1048576 | 512 MiB |
| p6 appfs / GPT UDISK | 15108096 | 8388608 | 2531328 | 262144 | 128 MiB |
| p7 UDISK / GPT primary | 23496704 | 1048576 | 2793472 | 27582461 | 约 13.1 GiB |

live GPT 在 p4 结束 sector 434175 与 p5 开始 sector 1482752 之间没有 GPT partition entry，间隔 1048576 sectors（512 MiB）。本次 inventory 没有读取或解释该间隔的原始内容，因此不能称它为安全可用的空闲 RAM/磁盘区域，也不能推断它由哪个阶段使用。

这说明当前设备的前四个启动相关 GPT 分区与 prepared RG34XX 基线相同，但后续 rootfs/data/SD 大分区已经采用另一套几何布局。不能把 prepared manifest 的 p5/p6/p7 起始位置或大小直接套到当前卡；后续任何分区读取/写入/镜像定位都必须以现场 GPT 为准，并保持本阶段只读。该布局差异本身不是损坏证据：当前 p5 rootfs、p6 ext4 /data、p7 vfat /mnt/sdcard 均有对应的 sysfs 与运行时挂载证据。

### 6.3 第三轮完整 env 解析（2026-11-22 19:19:42）

第三轮采集器通过 sysfs 中精确的 `PARTNAME=env` 定位到 `/dev/mmcblk0p3`，成功读取整个 GPT 分区：

| 项目 | 结果 |
|---|---|
| GPT 分区 | p3，起始 LBA 270336，32768 sectors |
| 分区总大小 | 16,777,216 bytes（16 MiB） |
| 完整分区 SHA-256 | `8dd964ef0d525071626040d7cdce6c16925aaa6108d35558a0a5f032592b78a2` |
| CRC32 little-endian 头字段 | `0xe3b6e401` |
| 唯一 CRC 匹配数据偏移 | 4 |
| CRC 匹配的环境映像总长度 | 131,072 bytes（128 KiB，`0x20000`） |
| CRC 输入数据长度 | 131,068 bytes（`0x1fffc`） |
| 环境变量数量 | 25 |
| 解析警告 | 0 |

解析器在整个 16 MiB 分区中只找到一个 CRC 匹配：从数据偏移 4 开始，对 131,068 bytes 计算 CRC32，得到 `0xe3b6e401`。偏移 4 的首字节为 `0x00`，它属于 CRC 覆盖的数据本身；偏移 5 并未产生 CRC 匹配。因此本次现场数据表现为 CRC32 header + data-at-offset-4 的 128 KiB 环境映像，不能把首字节误称为独立的 redundant-env flags。U-Boot 标准 `env_t` 结构中，CRC 覆盖 data；只有启用 `CONFIG_SYS_REDUNDAND_ENVIRONMENT` 时才另有 flags 字段，参见 [U-Boot environment.h](https://github.com/ARM-software/u-boot/blob/master/include/environment.h)。

完整 p3 分区 SHA-256 与 BaseOS-MultiFrontend 的 [manifest/prepared/rg34xx.json](https://github.com/dengzhan007/BaseOS-MultiFrontend/blob/main/manifest/prepared/rg34xx.json) 中 `preserved_regions.env.sha256` 完全一致；该 manifest 对应的 env 区域同样从 LBA 270336 开始、长度 32768 sectors（16 MiB）。因此，**本机整个 env 分区与 BaseOS 已固定的 RG34XX prepared env 分区逐字节一致**，不是只比较了其中的变量文本。

分区中的 CRC 有效环境变量与第二轮前缀解析一致：`mmc_root=/dev/mmcblk0p5`、`nand_root=/dev/nand0p4`、`setargs_mmc`、`setargs_nand`、`boot_normal`、`boot_recovery`、`boot_fastboot`、`bootdelay=0`、`bootcmd=run setargs_nand boot_normal` 等共 25 项。当前 runtime cmdline 与 `setargs_mmc` 结果一致；结合目标 U-Boot 中的 `update bootcmd` / `update part info` 字符串，SD 介质自动切换解释得到进一步支持。仍需注意：有效 CRC 和 prepared hash 只能证明 p3 内容一致且内部校验有效，不能单独证明本次 U-Boot 确实选择并导入了这个环境副本。

分区剩余区域的字节分布也已检查：`[0x20000,0x400000)` 为全零，`[0x400000,0x1000000)` 为全 `0xff`。这只是现场原始内容描述，不据此推断额外启动功能。

## 7. 对启动路线的影响

1. 原厂 monitor 看起来可复用，且与已知 RG34XX monitor 输入完全相同；仍需在后续入口审计中确认 monitor 的实际加载地址、保留内存、SMC 服务与安全世界访问范围。
2. 当前 U-Boot 带有 BaseOS gzip 优化特征，但不是已知精确 pinned U-Boot hash。不要对它盲目应用基于固定二进制偏移的补丁；当前 p3 env 分区则已与 BaseOS prepared RG34XX env 区域完整 hash 匹配。
3. 当前两份 Boot0 在长度、debug 状态和主体内容上存在区别。H616 BROM 公开资料显示它检查 LBA 16 与 LBA 512；结合现场偏移，LBA 16 是最可能的正常启动副本，LBA 512 是高位候选。仍需目标机启动轨迹确认本次实际选择，并核验 DRAM training 结果，再决定是否保留厂商链或接管 SPL。
4. runtime FDT 证明内核可见 1 GiB RAM，但没有提供 monitor/secure/DMA 等完整 carveout。不能把 0x40000000–0x7fffffff 整段当作裸机可用内存。
5. 当前已知 U-Boot 的 boot 分区读取/bootm 路径，但仍没有实机证据证明 AArch32 BL33 如何进入任意 AArch64 payload，或厂商 Linux SMC 可被安全地重新定向。Issue #2 的 payload 地址和入口 ABI 继续保持未定，不构建猜地址的 dummy payload。

### 7.1 BaseOS native SPL 研究布局与本机 1 GiB RAM 的关系

BaseOS-MultiFrontend 的 `experiments/h700-boot-speed/DESIGN.md` 将下列地址定义为其 H700 native SPL / A32 BL33 / Linux loader 的固定研究布局：

| 地址 | BaseOS 实验用途 | 对 Linkslot payload 的结论 |
|---:|---|---|
| `0x40080000` | AArch64 Linux Image | Linux handoff 地址，不是 Linkslot 入口 |
| `0x42000000` | initrd | 由该实验链使用，不可当作空闲区 |
| `0x44000000` | 最终 FDT | Linux 参数区，不是 payload 入口 |
| `0x45000000` | 暂存 Android boot image | 与本机 env 的 `boot_normal` 读入/bootm 路径相对应的 staging 语义，不是裸机入口 |
| `0x48000000` | vendor monitor | BL31/secure monitor 占用 |
| `0x4a000000` | AArch32 SVC BL33 loader，预留 1 MiB slot | 含该 loader 自己的 stack/A32 tables |
| `0x4a100000` | DTBO | 实验链预加载数据 |
| `0x4a200000` | monitor DTB | 实验链预加载数据 |
| `0x4a300000` | handoff data record | 实验链记录区 |
| `0x4a400000` | EL3 临时页表（实验范围至 `0x4a403000`） | secure/EL3 实验占用 |

本机 live FDT 与 `/proc/iomem` 显示的 DRAM 窗口为 `[0x40000000,0x80000000)`。表中地址从数值上都落在这 1 GiB DRAM 窗口内；这只是边界检查，不表示它们对 Linkslot 可分配。它们在 BaseOS 方案中已有明确所有者/用途，不能因当前运行的是 Linux、或者某一范围未出现在简化的 `/proc/iomem` 输出中，就把它认作可复用 payload 区域。

本机 framebuffer 已明确占用 `[0x7bf24800,0x7c076000)`，与上表低地址区间没有数值重叠；但 CMA 的物理 base、monitor/secure world 其余 carveout、DMA 缓冲及 loader 生命周期仍未知。因此目前没有任何 Linkslot payload load address 或 entry address 获得批准。BaseOS 实验设计本身也要求逐项证明不与 monitor 实际读写和保留区重叠，并明确警告不要假设 `0x49000000` 可用。

这套 native SPL / monitor / A32 loader 的物理试验覆盖 BaseOS 实验记录中的 RG34XX SP（2 GiB LPDDR4）和 RG40XX V（1 GiB LPDDR3）；这些结果不能替代本目标 RG34XX 非 SP（1 GiB LPDDR4）的冷启动与 handoff 验收。该项目的目标机地址、入口 EL、栈、cache/MMU/TLB、GIC 和 secure carveout 仍须以本机证据验证。

### 7.2 第四轮 memblock 物理范围已读出

第四轮采集归档和独立文本见 [reports 分支的原始 memblock 快照](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/debugfs-memory-reservations.txt)。该文件 SHA-256 为 `6d256713f030dd6093c6f04770cf20a9561b93db76f288c01f2da179f1603ced`。它报告 CMA debugfs 不存在，但导出了 memblock `memory/reserved`。

**memblock.memory** 只有一段：

```text
[0x40000000, 0x80000000) = 1 GiB
```

**memblock.reserved** 共 46 条。前 9 条较大的连续区间换算为半开区间如下（原文件 end 地址是闭区间，以下 end-exclusive = 原 end + 1）：

| 索引 | 半开物理区间 | 精确长度 | 解释状态 |
|---:|---|---:|---|
| 0 | `[0x40080000, 0x411b5000)` | 17,620 KiB | 与 `/proc/iomem` 的 kernel code/data 范围相符 |
| 1 | `[0x411b6000, 0x411b8000)` | 8 KiB | 紧邻 kernel 区域的小型保留项，具体用途未确认 |
| 2 | `[0x42000000, 0x42001000)` | 4 KiB | 地址与 BaseOS 实验的 initrd 起始地址相同，但仅凭一页大小不能认定为 initrd |
| 3 | `[0x44000000, 0x44023240)` | 143,936 bytes | 起始地址与 BaseOS 实验最终 FDT 地址相同，疑似 FDT/启动数据；当前没有对象级标记 |
| 4 | `[0x48000000, 0x49000000)` | 16 MiB | 与 BaseOS native SPL 设计中的 vendor monitor 起始地址及 16 MiB 窗口一致；强烈支持 monitor 在该区域保留，但仍应通过目标 monitor/loader 的实际访问范围做最终核对 |
| 5 | `[0x77c00000, 0x7bc00000)` | 64 MiB | 唯一一段恰好 64 MiB 的连续保留区，与 `CmaTotal=65536 kB` 完全相等；当前最高可信 CMA 物理区间候选 |
| 6 | `[0x7ec00000, 0x7fc00000)` | 16 MiB | 高地址连续保留区，来源/用途未知，必须排除 |
| 7 | `[0x7fd4a000, 0x7fd4b000)` | 4 KiB | 小型高地址保留项，用途未知 |
| 8 | `[0x7fd65000, 0x7ff49000)` | 1,982,464 bytes | 高地址保留区，约 1.89 MiB，来源/用途未知 |

索引 9–45 还有 37 条彼此分离的小型保留区，分布在 `0x7ff49a00` 至 `0x7ffffffe` 之间。它们不是一个连续 42 KiB 区间，不能将其中的空隙合并后当作某个经证明可用的载荷区。原文件的 `size:...K` 是整数 KiB 显示；例如 FDT 候选区的精确长度为 140.5625 KiB，文件显示 `140K`。地址端点才是精确计算区间的依据。

#### CMA 区间判断

启动参数含 `cma=64M`，`/proc/meminfo` 的 `CmaTotal=65536 kB`；memblock.reserved 中恰有一段 64 MiB 的连续区间 `[0x77c00000, 0x7bc00000)`。Linux CMA 的标准初始化路径会从早期内存分配器（memblock/bootmem）保留连续区域；这与本机观测到的“CMA 总量 64 MiB + 唯一 64 MiB memblock reserved 区间”相吻合。参见 Linux DMA contiguous CMA 的早期保留流程说明：[dma-contiguous.c](https://cos.googlesource.com/third_party/kernel/%2B/c570b82c5e6ac78be35b4e72594c0f1b1888fce1/drivers/base/dma-contiguous.c)。

因此现在可以把 `[0x77c00000, 0x7bc00000)` 作为**高置信 CMA 候选物理范围**纳入保守内存排除表；但本机没有导出 `/sys/kernel/debug/cma` 的 `base_pfn/count`，memblock 条目本身也没有区域名称，尚不能把“64 MiB 数量完全吻合”提升为 CMA base 的直接接口确认。CMA 页面在 Linux 内核中可能用于可迁移页并供 DMA 连续分配，不能因为它属于 CMA 就视为 Linkslot loader 可随意覆盖的 RAM。

#### 与 framebuffer 的关系

此前 live FDT 的 `boot_fb0` 和 bootargs `disp_reserve` 已独立确认 framebuffer 为 `[0x7bf24800, 0x7c076000)`，长度 1,382,400 bytes。它**没有出现在本次 memblock.reserved 的 46 条记录中**。这不是解除 framebuffer 排除的理由：FDT/bootargs 已经给出明确显示缓冲区范围，应继续无条件排除。

framebuffer 起点比 64 MiB CMA 候选区末端 `0x7bc00000` 高 `0x324800`（3,291,136 bytes），两者不重叠；framebuffer 也不与高地址 16 MiB 区间 `[0x7ec00000,0x7fc00000)` 重叠。这个不重叠关系只描述地址几何，不说明两者之间的空隙可安全用于 payload。

#### 对 payload 内存布局的影响

- `[0x48000000, 0x49000000)` 必须视为 vendor monitor 保留区；不得把 `0x48000000` 或其内部地址用作 payload。
- `[0x77c00000, 0x7bc00000)` 暂按 CMA 候选保留，不能用于 payload。
- `[0x7ec00000, 0x7fc00000)`、索引 7–45 的高地址区间全部保持排除，等待归属确认。
- kernel 区域、FDT/启动数据候选范围以及 framebuffer 继续排除。
- memblock.reserved 未列出 framebuffer，且用户态运行阶段的驱动/DMA 分配不一定完整反映在这份早期 memblock 表里。因此，`memblock.memory - memblock.reserved` **不能直接作为裸机可用内存清单**。

本轮已从“没有物理 reservation 数据”推进到“已读出并确认若干实际保留区间”；但 16 MiB 高地址区、top-of-RAM 碎片区、monitor 的实际访问上界、secure world、DMA/显示控制器缓冲及 loader 自身使用范围仍未全部归属。还没有任何 Linkslot payload load/entry address 获得批准。


## 8. 下一步

完整 env 分区已读回且 CRC/hash 验证通过；第四轮已读出 46 条 memblock reserved entries，并得到一个 64 MiB CMA 高置信候选和一个与 BaseOS monitor 地址匹配的 16 MiB 保留区。下一阶段重点是归属未知的高地址保留区/secure/DMA 范围、核对 monitor 实际读写边界，并获取 Boot0 当前选择/DRAM training 证据；在此之前不批准任何 payload 地址或 A32→A64 handoff 实现，继续只读。

在 U-Boot environment 和 monitor/DRAM 安全边界审查完成前，不修改 Boot0、TOC1、U-Boot、monitor 或 GPT，不试刷 AArch64 payload。设备没有 eMMC；失败恢复仍按项目设计由电脑重写/修复 SD 卡，不依赖回退到旧 Linux/BaseOS。


## 9. 第四轮 live FDT、initrd、ION / OP-TEE 与 memblock 交叉复核

第四轮归档仍为 [reports 分支 ZIP](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/reports/reports/rg34xx-inventory-20261122-195053.zip)，SHA-256：`bbfa065e590ecd7f8394f9b251d15468d63ed6c93bbe192e4272a65a700ab5e2`。本节使用归档内的 `live-fdt.dtb`、`dmesg-hardware-lines.txt` 和独立 memblock 文本交叉判断；没有对设备或 SD 卡执行写入。

### 9.1 initrd 保留页得到对象级关联

反编译 live FDT 后，`/chosen` 明确给出：

```text
linux,initrd-start = <0x00 0x42000000>;
linux,initrd-end   = <0x00 0x42000200>;
```

按 FDT 的 end-exclusive 语义，initrd 区间为 `[0x42000000,0x42000200)`，长度 512 bytes。第四轮 memblock 的第 2 条是 `[0x42000000,0x42001000)`，恰好覆盖该 initrd 数据所在的 4 KiB 页。因此原先“与 initrd 实验基址相同但 owner 未明”的判断已升级为：**该 4 KiB memblock reservation 与 live FDT 声明的 initrd 页高度吻合**。不要把这页当作空闲地址；initrd 的实际内容和用途仍应单独分析。

### 9.2 FDT blob 与 0x44000000 保留段高度吻合

本次 live FDT 文件：
- SHA-256：`dbe276ea70afff2a57e29b62c29cec8fa0db5a0c4d94986168f9dc37b14d06d5`
- `totalsize/file_bytes = 143,936` bytes

memblock 第 3 条为 `[0x44000000,0x44023240)`，长度同样是 143,936 bytes；其起点又与 BaseOS native SPL 研究中的最终 FDT 基址 `0x44000000` 相同。**地址起点、精确字节长度和 BaseOS 研究布局三者同时吻合，强烈支持该 reservation 就是交给 Linux 的 live FDT blob。**但 memblock 输出没有 owner 标签，也没有直接捕获 U-Boot 传入的 FDT 指针，因此文档仍将它标为“高置信 FDT 区间匹配”，而不是声称已经获得指针级 trace。

### 9.3 OP-TEE 节点存在，但 SMC 身份校验失败

live FDT 反编译得到：
```dts
/firmware/optee {
    compatible = "linaro,optee-tz";
    method = "smc";
};
```

同一份 live FDT 的 bootargs 包含 `androidboot.secure_os_exist=0`。第四轮 `dmesg-hardware-lines.txt` 共 148 行，其中 OP-TEE 相关日志为：
```text
[0.981049] optee: probing for conduit method from DT.
[0.981060] optee: api uid mismatch
```

Linux OP-TEE 驱动在通过 DT 选择 SMC conduit 后，会调用 OP-TEE API UID 检查；UID 不匹配时直接以 `-EINVAL` 退出 probe，尚未走到版本、capabilities、共享内存池和 TEE 设备注册阶段（参考 [Linux OP-TEE core.c](https://code.googlesource.com/linux/torvalds/linux/+/c6e169bc146a76d5ccbf4d3825f705414352bd03/drivers/tee/optee/core.c)）。

因此当前准确结论是：**设备树声明了 OP-TEE SMC conduit，但本次 Linux 启动没有通过 OP-TEE API UID 身份校验；vendor bootargs 同时声明 secure OS 不存在。**这不等于 BL31/vendor monitor 未运行，也不能证明 CPU 安全世界完全不存在，或某些地址没有被 EL3/固件保留。OP-TEE 驱动的身份校验失败不能被改写成“没有 secure world”。

### 9.4 ION heap 声明没有给出固定物理范围

live FDT 还声明了 Allwinner ION heap：
- `/ion/heap_cma@0`：`compatible="allwinner,cma"`，`heap-name="cma"`，`heap-base=0`，`heap-size=0`，`heap-type="ion_cma"`。
- `/ion/heap_secure@0`：`compatible="allwinner,secure"`，`heap-name="secure"`，`heap-base=0`，`heap-size=0`，`heap-type="ion_secure"`。

这两个节点是 heap 类型/名称声明，但 base 和 size 都为零；它们没有提供可据以绘制的固定物理区间。尤其不能仅凭 `heap-name="secure"` 就把 memblock 的 `[0x7ec00000,0x7fc00000)`（16 MiB）指定为 secure heap。live FDT 也没有标准根节点 `/reserved-memory`。这只说明 DT 没有通过标准 reserved-memory 子节点列出 carveout，不说明所有固件/monitor/DMA 保留都不存在。

### 9.5 对 memblock 表的修订与剩余边界

| 区间（半开） | 本轮交叉证据 | 当前归属 |
|---|---|---|
| `[0x42000000,0x42001000)` | live FDT initrd `[0x42000000,0x42000200)` 被该 4 KiB 页覆盖 | 高置信 initrd backing/reservation |
| `[0x44000000,0x44023240)` | live FDT blob 长度恰为 143,936 bytes，起点吻合 | 高置信 live FDT blob reservation |
| `[0x48000000,0x49000000)` | 16 MiB 范围与 BaseOS vendor monitor 研究窗口一致 | monitor 保留区，仍需核对实际访问上界 |
| `[0x77c00000,0x7bc00000)` | `cma=64M`、`CmaTotal=65536 kB` 与唯一 64 MiB memblock reservation 相符 | 高置信 CMA 候选，未获 CMA debugfs base_pfn/count 直接确认 |
| `[0x7ec00000,0x7fc00000)` | ION secure heap 节点 base/size 均为零；OP-TEE probe UID mismatch；secure_os_exist=0 | **仍未知，不能标注为 secure heap，也不能释放** |
| 顶部 37 条碎片 | 本轮 FDT/dmesg 未提供 owner 对应关系 | 全部继续排除 |

148 行硬件日志中没有 CMA base、memblock owner、BL31/ATF carveout 或 secure-memory 范围的直接输出；这是对该筛选日志文件的结论，不是对整个内核所有日志通道的否定。framebuffer `[0x7bf24800,0x7c076000)` 仍须独立排除。

**当前没有批准任何 Linkslot payload load/entry address。**下一步应优先获取设备上的 ION heap/debug 接口（若该内核实际导出）、TEE 设备节点和更完整的启动日志，并审查 monitor 的 SMC/内存访问边界；对未知 16 MiB 段和 top-of-RAM 碎片继续按保留区处理。
