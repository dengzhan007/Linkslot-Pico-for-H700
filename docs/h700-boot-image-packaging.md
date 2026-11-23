# RG34XX BOL4 / Android boot v2 打包与提取工具

本工具为 RG34XX 当前 BaseOS native loader 的启动镜像格式提供纯主机侧提取、打包和检查。它只处理普通文件，不访问设备或存储介质。

## 已确认的活动启动镜像

reports 分支的 active-loader provenance 已确认，当前 RG34XX 采集的活动启动链为：

Boot0 → AArch64 native SPL → vendor BL31 → AArch32 BL33 loader → BaseOS BOL4/LZ4 Android image → AArch64 Linux-style Image。

活动 Android 镜像位于 p4/boot 分区偏移 48 MiB（字节偏移 0x03000000），本次采集的镜像总长度为 0x829000 字节。A32 loader 从 Android v2 kernel component 读取 BOL4 frame，校验 packed block 的 FNV-1a 后，以 raw LZ4 解压到 0x40080000；随后准备 FDT 并通过已有 vendor Linux-start SMC 进入 AArch64。

0x8000ff04 是当前 Linux 启动交接契约，不是通用的“跳转到任意裸机入口”接口。本工具不改变 SMC 参数、Image 入口、FDT 交接或其他固件阶段。

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

## 从本地 RG34XX boot 分区提取真实输入

原始 64 MiB boot-partition.bin 只保存在本地采集目录，没有上传仓库。仓库 CI 因此只能执行合成媒体回归，不能从文本摘要伪造原始二进制输入。

在持有与 reports 中该次采集相同的 RG34XX p4/boot 分区 dump 的电脑上执行：

~~~sh
python3 tools/h700_boot_image.py extract-inputs \
  --partition boot-partition.bin \
  --offset 0x03000000 \
  --size 0x829000 \
  --output-dir build/rg34xx-active-inputs
~~~

该命令只读取本地文件，要求输出目录为空，并按显式给定的 offset/size 提取：

- active-boot.img
- android-v2-header.bin
- kernel.BOL4
- kernel.Image
- initrd.img
- dtb.bin
- extracted-manifest.json

manifest 记录源分区 hash、活动镜像 hash、每个拆分组件的长度和 SHA-256。脚本不会修改原始分区 dump，也不会触碰 SD 卡。offset 和 size 必须显式提供，不会扫描分区或猜测哪个 Android image 是活动镜像。对于文档给出的已审计 offset/size，工具还会核对整张 image 的 FNV-1a（0x04762a5f）以及 BOL4 component length/packed length/unpacked length/FNV（0x80474e / 0x80473e / 0x010a7008 / 0xb4c47d39）；任意一项不符都会拒绝提取。

注意：0x03000000 / 0x829000 是这次 RG34XX 现场采集报告记录的固定审计值，不是所有固件版本的通用常量。更换设备、固件或 boot 分区来源前，必须重新核对对应报告和镜像头部。

## 由提取输入构建 host candidate

使用同一目录提取的、彼此匹配的 kernel、header、DTB 和 initrd：

~~~sh
python3 tools/h700_boot_image.py build \
  --kernel build/rg34xx-active-inputs/kernel.Image \
  --header build/rg34xx-active-inputs/android-v2-header.bin \
  --dtb build/rg34xx-active-inputs/dtb.bin \
  --initrd build/rg34xx-active-inputs/initrd.img \
  --output build/rg34xx-linkslot-boot.img

python3 tools/h700_boot_image.py inspect build/rg34xx-linkslot-boot.img
~~~

这生成一个新的 host-side Android v2 文件、独立 BOL4 component 和 JSON manifest。它不是可刷写安装包，也不修改原始 boot 分区。

**不能把新生成的 p4+48 MiB 镜像单独替换到现有启动分区。** 固定版本的 BaseOS native loader 在启动时会比较 SPL 生成记录中的 image byte length/FNV 与 loader 编译时的 BASEOS_IMAGE_BYTES / BASEOS_IMAGE_FNV。固定版本的 native/build.py 会从同一份 Android image 生成这两个常量，并把 image 的 SHA-256 计入 generation；配套 p4+40 MiB firmware package（含 native loader）也由该构建过程生成。因此只要 Android image 内容或长度变化，就必须同步重建并校验匹配的 p4+40 MiB firmware/SPL 侧产物，不能保留旧 package 再只换 p4+48 MiB image。

reports 中的 stage-0 audit 已记录，原始 boot-package.bin 和构建期 GPT prefix 没有归档，当前无法仅凭仓库里的文本/JSON 报告重建匹配的 native firmware generation。这个工具只实现 Android/BOL4 host component，不生成配套 firmware package、capsule、Boot0 或完整分区镜像。

## 使用独立输入构建

如果已有经核验的文件，也可直接执行：

~~~sh
python3 tools/h700_boot_image.py build \
  --kernel Image \
  --header rg34xx-android-v2-header.bin \
  --dtb rg34xx.dtb \
  --initrd initrd.img \
  --output build/rg34xx-linkslot-boot.img

python3 tools/h700_boot_image.py inspect build/rg34xx-linkslot-boot.img
~~~

initrd 和 command line 可省略；默认保留 header template 中的 command line。header template、DTB、initrd 和 kernel 必须来自同一套已核验 RG34XX 构建输入，不要混用其他机型或固件版本的板级文件。

## Android boot v2 容器约束

工具按以下组件顺序生成 Android v2 image：

1. kernel：BOL4 frame；
2. ramdisk：可选；
3. second：空；
4. recovery DTBO：空；
5. DTB：必需。

每个组件按 2048 字节页对齐。工具检查 ANDROID! magic、page size 2048、header version 2、header_size 1660，以及 RG34XX 当前 loader 使用的 kernel/ramdisk/second/tags 地址；重算组件长度与 Android image ID（SHA-1）。默认完整 Android image 上限为 16 MiB。initrd 不得超过 loader 的 1 MiB 限制。



C 互操作测试夹具 `tools/vendor/baseos-native-lz4/` 逐字保留 BaseOS-MultiFrontend 的 `experiments/h700-boot-speed/native/lz4-stream.c/.h`，来源固定为 commit `cbe253a43f46cf551cc9a6c79a84015bde3686cf`。测试 harness 仅在主机进程中调用 `baseos_lz4_begin()` / `baseos_lz4_step()`；不会调用 MMIO、启动代码或设备接口。

## 验证与限制

CI 的 unittest discover 运行 14 项 host tests，覆盖 raw LZ4、BOL4 FNV/长度/损坏输入、AArch64 Image magic、Android v2 layout、image ID、load-address policy、16 MiB image budget、本地分区 dump 的有界提取、提取 hash manifest、原始文件不变性、已审计活动镜像 fingerprint，以及与 BaseOS native C decoder 的互操作。Linux runner 会编译并运行固定版本的 native C decoder；Windows runner 运行纯 Python 测试（若没有 host C compiler 则跳过 C 互操作测试）。

**该工具不是完整固件构建器，也不构成实机启动许可。** 它不会生成或修改 Boot0、TOC1、BL31、BL33、SPL、分区表或安装包；不会写 SD/Boot 区；不会执行 payload。Android v2 文件通过结构校验，只代表封装与当前 loader 的文件格式相符，不代表内存归属、SMC 目标地址、watchdog 恢复、UART/显示观察通道或 RG34XX 冷启动已经通过。

a64 hardware readiness 报告中的 hardware probe 仍为 NO-GO；未知保留区 [0x7ec00000,0x7fc00000) 及 RAM 顶部碎片继续排除。由于真实 boot-partition.bin 没有上传，本次会话不能对现场二进制运行提取命令，也不能声称已经产出真实 RG34XX candidate。

这项工作只关闭“缺少可重复的 BOL4 / Android v2 host pack-and-verify primitive”这一工程缺口，不把项目状态提升为可刷写或可启动。
