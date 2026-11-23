# RG34XX BOL4 / Android boot v2 打包工具

本工具为 RG34XX 当前 BaseOS native loader 的启动镜像格式提供纯主机侧打包与校验。它只处理文件，不访问设备或存储介质。

## 已确认的目标启动链

reports 分支的 active-loader provenance 报告记录的当前运行链为：Boot0 → AArch64 native SPL → vendor BL31 → AArch32 BL33 loader → AArch64 Linux-style Image。A32 loader 从 Android boot v2 镜像读取 kernel component；该 component 是 BOL4 frame，包含 raw LZ4 block。解压目的地为 0x40080000，随后 loader 准备 FDT（0x44000000）并通过既有 vendor Linux-start SMC 进入 AArch64。

0x8000ff04 是已经观察到的 Linux 启动交接契约，不是通用的“跳转到任意裸机入口”接口。本工具不改变 SMC 参数、Image 入口、FDT 交接或其他固件阶段。

## BOL4 格式

BOL4 的 16 字节 little-endian header 为：

| 偏移 | 长度 | 含义 |
|---:|---:|---|
| 0 | 4 | ASCII BOL4 |
| 4 | 4 | 后续 raw LZ4 block 的字节数 |
| 8 | 4 | 解压后 AArch64 Image 的字节数 |
| 12 | 4 | raw LZ4 block 的 FNV-1a 32-bit checksum |
| 16 | 可变 | raw LZ4 block；不带额外 LZ4 frame/size prefix |

解压器输出上限沿用 native loader 的 0x01f80000 字节。打包器使用纯 Python raw LZ4 block encoder，并在输出前用有界 decoder 回读验证 header、packed length、FNV 与解压长度。

## Android boot v2 容器

工具接受一个经过核验的 2048-byte Android boot v2 header template，并按其组件顺序生成：

1. kernel：BOL4 frame；
2. ramdisk：可选；
3. second：空；
4. recovery DTBO：空；
5. DTB：必需。

每个组件按 2048 字节页对齐。工具检查 ANDROID! magic、page size 2048、header version 2、header_size 1660，以及 RG34XX 当前 loader 使用的 kernel/ramdisk/second/tags 地址；重算组件长度与 Android image ID（SHA-1）。默认完整 Android image 上限为 16 MiB，与当前 BaseOS native build policy 一致。initrd 不得超过 loader 的 1 MiB 限制。

## 从本地 RG34XX boot 分区提取真实输入

reports 中的 active-loader provenance 已将这份 RG34XX capture 的活动镜像定位为 p4/boot 分区偏移 \`0x03000000\`（+48 MiB），Android v2 镜像范围 \`0x829000\` 字节。原始 64 MiB \`boot-partition.bin\` 没有上传仓库，保存在采集端本地；因此仓库 CI 只能做合成媒体回归，不能自行重建这份真实输入。

在持有**同一份已审计 RG34XX p4/boot 分区 dump** 的电脑上执行：

~~~sh
python3 tools/h700_boot_image.py extract-inputs \
  --partition boot-partition.bin \
  --offset 0x03000000 \
  --size 0x829000 \
  --output-dir build/rg34xx-active-inputs
~~~

该命令只读取本地文件，要求输出目录为空，并提取：
\`active-boot.img\`、\`android-v2-header.bin\`、\`kernel.BOL4\`、\`kernel.Image\`、\`initrd.img\`、\`dtb.bin\` 和 \`extracted-manifest.json\`。manifest 记录源分区、活动镜像及各组件 SHA-256。脚本不会修改原始分区 dump，也不会触碰 SD 卡。

随后可从提取出的**相互匹配**的 kernel、header、DTB 和 initrd 重建 host candidate：

~~~sh
python3 tools/h700_boot_image.py build \
  --kernel build/rg34xx-active-inputs/kernel.Image \
  --header build/rg34xx-active-inputs/android-v2-header.bin \
  --dtb build/rg34xx-active-inputs/dtb.bin \
  --initrd build/rg34xx-active-inputs/initrd.img \
  --output build/rg34xx-linkslot-boot.img

python3 tools/h700_boot_image.py inspect build/rg34xx-linkslot-boot.img
~~~

不要把别的固件版本、其他机型或不同启动链的 header/DTB 与这份 kernel 混用。这里的 \`0x03000000 / 0x829000\` 是该次 RG34XX 采集的固定审计事实，不是任意 RG34XX 固件版本的通用猜测值；更换来源前需重新核对 reports 与镜像头部。

## 用法

如果已有独立、经过核验的文件，也可直接执行：

~~~sh
python3 tools/h700_boot_image.py build \
  --kernel Image \
  --header rg34xx-android-v2-header.bin \
  --dtb rg34xx.dtb \
  --initrd initrd.img \
  --output build/rg34xx-linkslot-boot.img

python3 tools/h700_boot_image.py inspect build/rg34xx-linkslot-boot.img
~~~

initrd 和 command line 可省略；默认保留 header template 中的 command line。构建产生 Android v2 image、独立 .bol4 component 和 JSON manifest。输出路径不得覆盖输入文件。


~~~sh
python3 tools/h700_boot_image.py build \
  --kernel Image \
  --header rg34xx-android-v2-header.bin \
  --dtb rg34xx.dtb \
  --initrd initrd.img \
  --output build/rg34xx-linkslot-boot.img

python3 tools/h700_boot_image.py inspect build/rg34xx-linkslot-boot.img
~~~

initrd 和 command line 可省略；默认保留 header template 中的 command line。构建产生 Android v2 image、独立 .bol4 component 和 JSON manifest。输出路径不得覆盖输入文件。

header template、DTB、initrd 和 kernel 必须来自同一套已核验 RG34XX 构建输入；工具不会从不匹配的 stock image 自动拼凑板级文件。测试中的合成 Image/FDT 只用于格式回归，并非可启动固件。

## 验证与限制

CI 的 unittest discover 会运行此工具的 host tests，覆盖本地 p4 dump 的有界提取、raw LZ4、BOL4 FNV/长度/损坏输入、AArch64 Image magic、Android v2 layout、image ID、load-address policy、16 MiB image budget 和 round-trip。

**该工具不是完整固件构建器，也不构成实机启动许可。** 它不会生成/修改 Boot0、TOC1、BL31、BL33、SPL、分区表或安装包；不会写 SD/Boot 区；不会执行 payload。Android v2 文件通过结构校验，只代表封装与当前 loader 的文件格式相符，不代表内存归属、SMC 目标地址、watchdog 恢复、UART/显示观察通道或 RG34XX 冷启动已经通过。a64 hardware readiness 报告中的 hardware probe 仍为 NO-GO；未知保留区 [0x7ec00000,0x7fc00000) 及 RAM 顶部碎片继续排除。

因此，这项交付只关闭“缺少可重复的 BOL4 / Android v2 host pack-and-verify primitive”这一工程缺口，不把项目状态提升为可刷写或可启动。
