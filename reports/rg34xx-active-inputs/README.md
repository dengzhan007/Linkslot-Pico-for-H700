# RG34XX 活动启动镜像拆分与校验清单

本目录只包含**校验清单与拆分记录**，不含任何二进制。所有拆分件与原始活动镜像留在采集机本地
`build/rg34xx-active-inputs/`，其大小与 SHA-256 见下表，可按清单自行核对。

- 采集时间：2026-11-24 06:51 +08:00
- 性质：host-only。全程只读取本地 64 MiB p4 dump，未接触设备、SD 卡、Boot0、TOC1、U-Boot、monitor
  或任何分区写操作（`device_io_performed: false`）。
- 结果：可安装性 `not_installable: true`，硬件验证 `not_hardware_validated: true`。这是取证与
  重建输入提取，不是可刷写镜像。

## 1. 实际执行的命令

```text
python tools/h700_boot_image.py extract-inputs \
  --partition build/rg34xx-inventory-20261122-214708/boot-partition.bin \
  --offset 0x03000000 --size 0x829000 \
  --output-dir build/rg34xx-active-inputs
```

与请求命令的两处差异，均不改变结论：

1. 本机没有安装 `py` 启动器（`py -3` 报 command not found），改用同一解释器 `python`
   （CPython 3.13.14）。
2. `--partition` 用了 dump 的实际相对路径。工具的 manifest 只记录文件基名
   `boot-partition.bin`，字节数与 SHA-256 来自文件内容本身，因此路径写法不影响清单。

## 2. 工具来源

| 项 | 值 |
| --- | --- |
| 文件 | `tools/h700_boot_image.py` |
| 所在分支 | `porting/linkslot-1.0.36-feasibility` |
| 提交 | `b6c3667` |
| SHA-256 | `5c75dc3805ef50117fa19ca0de1896ee82965f4100a68fed0619a0e1ced76c3a` |

该文件不在 `main`、`reports` 或 `audit/rg34xx-ion-drm-followup-20261122` 上，因此从 porting 分支
只读导出到临时目录执行，未修改任何工作树。自带测试实跑 14 项：14 通过、1 跳过（缺少宿主编译器，
跳过的是"C 解码器对照"用例）。

## 3. 输入

| 项 | 值 |
| --- | --- |
| 文件 | `boot-partition.bin`（64 MiB p4 dump） |
| 字节 | 67,108,864 |
| SHA-256 | `41cfaa8f9ade4a5e05a89188abb7db154243e38b8e19c53b730f0a45ff325516` |
| 提取偏移 | `0x03000000` |
| 提取长度 | `0x829000`（8,556,544） |
| 区间末端 | `0x03829000` < 64 MiB，未越界 |

## 4. 拆分件清单

| 文件 | 字节 | SHA-256 |
| --- | ---: | --- |
| `active-boot.img` | 8,556,544 | `d71e5bd87b23fb409d7a837cf90d153ae117af518fe6ee330a96d2c7a5f76fc3` |
| `android-v2-header.bin` | 2,048 | `d924e2f3e73db16f39dbec84c1de3e2216040656e521177799a89e738e7586df` |
| `kernel.BOL4` | 8,406,862 | `df5b391030794dec176a16b50b96efd0881e57f46acbad65a4f15fdbbe2074b8` |
| `kernel.Image` | 17,461,256 | `39cf8c3eb9a75ff0eda51b4074cd65860c5aa5386093bcaf69339819421d84b7` |
| `initrd.img` | 512 | `c0b1f70675c793df96e958306c94b3086375c35b2e91e497ffc3484bb93228d8` |
| `dtb.bin` | 144,384 | `c28c87380ffab00148bf4dc457e70b8c6c576470dcfbc53df64addddf89603a1` |

机器可读版本见同目录 `SHA256SUMS.txt`（含 `extracted-manifest.json` 与 `verification.json` 自身），
`sha256sum -c` 在本地 8/8 全部 OK。

## 5. 交叉校验

`verification.json` 的 21 项检查全部通过（`all_checks_passed: true`）。其中与已知身份相关的关键项：

- 活动镜像长度 `0x829000`、FNV-1a32 `0x04762a5f`——与工具内钉住的读数一致。
- 活动镜像与 dump 的 `[0x03000000, 0x03829000)` 区间逐字节相等。
- `kernel.BOL4` 长度 `0x80474e`，帧内 `packed_len = 0x80473e = 帧长 − 16`，
  `output_len = 0x10a7008`，头部 FNV 字段 `0xb4c47d39` 与对 packed 负载实算的 FNV 一致。
- BOL4 解压结果与 `kernel.Image` **逐字节相等**（不是抽样比对）。
- `kernel.Image` 长度 `0x10a7008`，magic `ARMd`，`text_offset 0x80000`，
  `image_size 0x1138000`，`flags 0xa`。
- `android-v2-header.bin` 与 `active-boot.img` 前 2,048 字节逐字节相等；
  `page_size 2048`、`header_version 2`、`header_size 1660`。
- Android v2 五个组件区长度 `[8406862, 512, 0, 0, 144384]`，即 BOL4 / initrd / recovery_dtbo(空) /
  second(空) / dtb。
- `initrd.img` 是 512 字节的空 cpio newc 归档（`070701` + 仅一个 `TRAILER!!!` 条目），
  与早前"空 ramdisk"的记录一致，但更精确：不是全零填充，而是一个合法空归档。
- `dtb.bin` magic `d00dfeed`，`totalsize 139,822`，在 144,384 字节的组件区之内（尾部为页对齐填充）。

## 6. 复现方式

在有 `tools/h700_boot_image.py` 的分支上，按第 1 节命令执行即可；输出目录必须不存在或为空，
否则工具拒绝执行（保护已有产物）。随后在输出目录运行：

```text
sha256sum active-boot.img android-v2-header.bin kernel.BOL4 kernel.Image initrd.img dtb.bin \
  extracted-manifest.json verification.json > SHA256SUMS.txt
sha256sum -c SHA256SUMS.txt
```

## 7. 边界说明

- 本次拆分不证明该镜像可启动、可刷写或与当前 Boot0/TOC1 配对。工具自身标注
  `requires_matching_native_firmware_rebuild`：native loader 会用编译期钉住的
  `BASEOS_IMAGE_BYTES`/`BASEOS_IMAGE_FNV` 校验记录，替换镜像必须与配对的固件一起重建。
- 未分析 `kernel.Image` 的符号、配置或驱动行为；未解码 `dtb.bin` 内容，只校验容器结构。
- 二进制（合计约 34.4 MB）按本仓库惯例不上传，只上传清单。
