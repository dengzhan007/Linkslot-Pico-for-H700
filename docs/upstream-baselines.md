# 上游源码基线

本文记录本项目的参考来源和正式导入规则。Linkslot 即将发布较大版本更新，因此当前记录的 Linkslot commit 仅用于前期源码研究，不作为后续移植的正式导入基线。

| 来源 | Ref / commit | 在本项目中的用途 |
|---|---|---|
| [linkSlot](https://github.com/luguiyi/linkslot) | 研究快照：`main` @ `9275419ca99846fe6738027b2ade72e1804fdc08` | 仅供了解现有 crate、UI、同步和 MOD 的历史实现；等待即将发布的大版本后重新锁定正式基线 |
| [GammaOS LK](https://github.com/TheGammaSqueeze/lk-mt6785) | `lk-gba-emu-sd-card-kpa` @ `9bf1419a49e0af6dba8b05916df382c9c9c6d0a8` | 裸机架构参考；注意它是 MT6785/32-bit ARM，不是 Allwinner H700 / RG34XX 移植目标 |
| [armGBA-pico](https://github.com/TheGammaSqueeze/armGBA-pico) | `main` @ `75dbb3108b16d7fa72e60e523a0e4ec3b14af122` | GBA 核心候选；A32 固定地址 blob、Core ABI、过滤和 cheats 的研究参考 |
| [gambatte-pico](https://github.com/TheGammaSqueeze/gambatte-pico) | GammaOS LK submodule @ `7d6a1d3def653b71f2ed03c02829277868511932` | GB/GBC 裸机集成参考；首选核心候选 |
| [gpsp-pico](https://github.com/TheGammaSqueeze/gpsp-pico) | GammaOS LK submodule @ `c3c909554a1b9a968c527b1f8153bba2fc57b0d1` | GBA 备选核心与 Pico Core ABI 适配参考 |

## Linkslot 正式基线锁定规则

1. 等待用户确认的 Linkslot 大版本正式发布。
2. 记录 release tag、对应 commit SHA、发布时间和主要变更。
3. 与上述研究快照比较，确认 crate 结构、运行时依赖、存档格式、MOD API、Web API 和云同步协议是否变化。
4. 以新版本作为 subtree 导入和逐 crate 审计的起点；不要为了保持旧 commit 而回移大版本中的变更。
5. 新基线通过编译和兼容性审查后，更新本文件并记录未移植或不兼容的功能。

## 特别说明

- `lk-mt6785` 中的 MTK CONNSYS、PMIC、GPIO、显示、USB PHY、DMA 和电源寄存器配置不得直接用于 Allwinner H700。
- `armGBA-pico` 的现成 blob 面向 32-bit ARM LK host，不能直接由 H700 AArch64 host 加载；要另行选择 AArch32 interworking、独立 AArch32 payload 或重新构建 AArch64 core ABI。
- armGBA-pico README 明确声明与 gpSP 共用 `/saves/gba` 的 128 KiB 电池存档缓冲区；没有明确声明与 Linkslot mGBA 字节兼容。
- 本表记录的是研究来源，不代表所有上游许可、子模块和构建产物已经完成发行审查。
