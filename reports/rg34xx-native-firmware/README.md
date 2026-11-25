# p4 + 40 MiB 原生固件包拆分与核对（host-only）

本目录只放 `firmware-manifest.json`（工具输出，未改动）和这份核对说明。拆出的固件二进制留在
采集机本地 `build/rg34xx-native-firmware/`，未上传（合计 287,232 B 固件包 + 4 个组件）。

- 时间：2026-11-24 07:24 +08:00
- 输入：本地 64 MiB p4 dump，SHA-256 `41cfaa8f9ade4a5e05a89188abb7db154243e38b8e19c53b730f0a45ff325516`
- 性质：host-only，`device_io_performed: false`；`not_installable`、`not_hardware_validated`
- 工具：`tools/h700_boot_image.py`，分支 `porting/linkslot-1.0.36-feasibility`，提交 `f0371a8`，
  SHA-256 `b063bfa092805215c043ecb0ce63834f66c13d51f7d257b9f8f52eac643eec47`

执行命令（本机无 `py` 启动器，改用同一解释器 `python` 3.13.14）：

```text
python tools/h700_boot_image.py extract-firmware \
  --partition build/rg34xx-inventory-20261122-214708/boot-partition.bin \
  --active-image build/rg34xx-active-inputs/active-boot.img \
  --output-dir build/rg34xx-native-firmware
```

## 1. 固件包身份

| 项 | 值 | 判定 |
| --- | --- | --- |
| 偏移 | `0x02800000`（+40 MiB） | 与审计记录一致 |
| 长度 | `0x00046200`（287,232 B） | 与钉住的 `RG34XX_FIRMWARE_BYTES` 一致 |
| SHA-256 | `dc9dc3ae66b30244911e7054ca66136991626cabb0d59c796c4b253a126f28e9` | 与审计记录一致 |
| FNV-1a32 | `0xaf0018cb` | 与审计记录一致 |
| 区间末端 | `0x02846200` < 64 MiB | 未越界 |
| 与 dump 对应区间 | 逐字节相等 | 是 |

"长度"指固件包本身 287,232 B，不是 40 MiB；**+40 MiB 是它在 p4 中的偏移**。

## 2. 四段边界与头部

四段首尾相接、无空洞无重叠，末段结束正好等于包长 `0x46200`：

| 段 | 文件偏移 | extent | payload | 载入地址 | 头/边界判定 |
| --- | --- | ---: | ---: | --- | --- |
| `monitor` | `0x00000` | `0x1A400` | `0x1A2D0` | `0x48000000` | AArch64，首条 `b 0x480000f8`；`0x4` 起 ASCII `monitor`；尾部 304 B 全零 |
| `dtbo` | `0x1A400` | `0x00A00` | `0x00920` | `0x4A100000` | FDT magic `d00dfeed`，`totalsize 2336 = payload`；尾部 224 B 全零 |
| `dtb` | `0x1AE00` | `0x23400` | `0x2222E`（139,822） | `0x4A200000` | FDT magic，`totalsize 139,822` 在 144,384 B extent 内；尾部 4,562 B 全零 |
| `loader` | `0x3E200` | `0x08000` | `0x08000` | `0x4A000000` | AArch32，首条 `b 0x4A001000`，`+4` 为 magic `uboot`；无填充 |

补充说明（只写已观察到的事实，不推断字段语义）：

- `monitor` 不是 AArch32。`0x1400003E` 按 AArch64 解码是 `b #0x480000f8`；跳到 `0x480000f8`
  后是 BL31 的 `_start` 前导：`mov x20,#0; mov x21,#0; mov w20,w0; mov w21,w1; mov x1,#0x100a;
  mrs x0,sctlr_el3; orr; msr sctlr_el3; isb; bl 0x48015804`。段内另有
  `v1.0(debug):f30a720`、`Built : 16:02:57, 2024-06-26`、`NOTICE:  BL3-1: ...` 等字符串，
  因此该段是 ARM Trusted Firmware BL31 镜像。
- `loader` 是 U-Boot 头结构：`+0x00 0xEA0003FE`（A32 `b`）、`+0x04 "uboot"`、`+0x0c 0xa24c7482`、
  `+0x10 0x00004000`、`+0x14/0x18 0x00100000`、`+0x1c "4.0."`、`+0x24 "2.0"`、`+0x2c 0x4a000000`。
  除 magic、分支目标与载入地址外，其余字段含义未验证，不做断言。

## 3. 两处 DTB 是否逐字节相同

相同。`firmware +40 MiB` 的 `dtb` 组件与 `+48 MiB` 活动镜像内的 dtb 组件都是 144,384 B，
SHA-256 同为 `c28c87380ffab00148bf4dc457e70b8c6c576470dcfbc53df64addddf89603a1`，逐字节相等。
这也解释了工具为何在两者不一致时直接报错退出。

## 4. loader 中的长度 / FNV 常量位置

**结论：能找到，而且是指令级比较，不只是字面量池。**

`firmware-manifest.json` 里 `loader_image_length_literal_candidate_offsets` 与
`loader_image_fnv_literal_candidate_offsets` 为空数组，这是 AArch32 下的**假阴性**：工具的扫描只找
4 字节小端字面量，而 A32 用 `movw`+`movt` 两半拼 32 位立即数。改按指令解码后，两处常量都在
`loader.bin` 内，位于同一个校验函数：

```text
4a001850: ldreq r0, [r6, #4]        ; 前置类型检查
4a001854: cmpeq r0, #2
4a001868: ldr   r0, [r6, #8]
4a00186c: cmp   r0, #0xf0
4a001870: ldreq r0, [r6, #0xc]
4a001874: cmpeq r0, #2
4a001878: bne   0x4a00185c          ; 失败出口
4a00187c: movw  r7, #0x28
4a001884: movt  r7, #0x4a30         ; r7 = 0x4a300028
4a001880: movw  r1, #0x9000
4a001888: movt  r1, #0x82           ; r1 = 0x00829000  <-- 活动镜像长度
4a00188c: ldr   r0, [r7, #0x94]
4a001890: cmp   r0, r1
4a001894: bne   0x4a00185c
4a001898: ldr   r0, [r7, #0x98]
4a00189c: movw  r1, #0x2a5f
4a0018a0: movt  r1, #0x476          ; r1 = 0x04762a5f  <-- 活动镜像 FNV-1a32
4a0018a4: cmp   r0, r1
4a0018a8: bne   0x4a00185c
```

| 常量 | 组成指令 | 比较指令 | 判定 |
| --- | --- | --- | --- |
| 长度 `0x00829000` | `0x4A001880` movw `#0x9000` + `0x4A001888` movt `#0x82` | `0x4A001890` `cmp r0, r1`（值来自 `[r7,#0x94]`） | 命中 |
| FNV `0x04762a5f` | `0x4A00189C` movw `#0x2a5f` + `0x4A0018A0` movt `#0x476` | `0x4A0018A4` `cmp r0, r1`（值来自 `[r7,#0x98]`） | 命中 |

两者比较失败都跳到同一个出口 `0x4A00185C`（`mov r0,r5; sub sp,fp,#0x1c; pop {...,pc}`），
即这是同一函数的硬性拒绝路径。基准寄存器 `r7 = 0x4A300028`，读取 `+0x94`/`+0x98` 两个字段。

这印证了早前报告的说法：native loader 会用编译期常量校验镜像长度与 FNV。**但只证明"常量与比较
指令存在"**，不证明该函数在真实启动路径上一定被执行、也不证明校验失败后的具体行为。

作为对照，同一批常量在 monitor / dtbo / dtb 中都没有 4 字节字面量命中。`0x010a7008`（kernel 长度）
在 p4 内有 5 处匹配（`0x6b1e53`、`0xa461ec`、`0xa461f0`、`0xa461f4`、`0x3000808`），`0xb4c47d39`
（BOL4 FNV）唯一命中 `0x0300080c`。`0x03000808`/`0x0300080c` 正是活动镜像内 BOL4 帧头部的
`output_len`/`packed_fnv` 字段（帧从镜像 `0x800` 开始），不是 loader 常量。

## 5. 边界说明

- 本次只做容器级拆分与常量定位，未反汇编 monitor 的加载流程，未验证该固件包与当前 Boot0/TOC1
  的实际配对关系（工具仅校验镜像侧的长度/FNV 常量）。
- `requires_matching_native_firmware_rebuild: true`：替换镜像必须与配对固件一起重建。
- 固件二进制全部留在本地，未上传。
