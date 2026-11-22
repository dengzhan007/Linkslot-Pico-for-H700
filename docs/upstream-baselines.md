# 上游源码基线

本文记录本项目初始研究时锁定的上游提交。更新任何上游版本前，先完成差异审查、许可检查、构建验证和对应设备回归，不直接追随浮动分支。

| 来源 | Ref / commit | 在本项目中的用途 |
|---|---|---|
| [linkSlot](https://github.com/luguiyi/linkslot) | `main` @ `9275419ca99846fe6738027b2ade72e1804fdc08` | UI、用户功能、目录语义、Web API、云同步和 MOD 的行为基线 |
| [GammaOS LK](https://github.com/TheGammaSqueeze/lk-mt6785) | `lk-gba-emu-sd-card-kpa` @ `9bf1419a49e0af6dba8b05916df382c9c9c6d0a8` | 裸机架构参考；注意它是 MT6785/32-bit ARM，不是 H700/RK3566 移植目标 |
| [armGBA-pico](https://github.com/TheGammaSqueeze/armGBA-pico) | `main` @ `75dbb3108b16d7fa72e60e523a0e4ec3b14af122` | GBA 核心候选；A32 固定地址 blob、Core ABI、过滤和 cheats 的研究参考 |
| [gambatte-pico](https://github.com/TheGammaSqueeze/gambatte-pico) | GammaOS LK submodule @ `7d6a1d3def653b71f2ed03c02829277868511932` | GB/GBC 裸机集成参考；首选核心候选 |
| [gpsp-pico](https://github.com/TheGammaSqueeze/gpsp-pico) | GammaOS LK submodule @ `c3c909554a1b9a968c527b1f8153bba2fc57b0d1` | GBA 备选核心与 Pico Core ABI 适配参考 |

## 特别说明

- `lk-mt6785` 中的 MTK CONNSYS、PMIC、GPIO、显示、USB PHY、DMA 和电源寄存器配置不得直接用于 RK3566。
- `armGBA-pico` 的现成 blob 面向 32-bit ARM LK host，不能直接由 H700 AArch64 host 加载；要另行选择 AArch32 interworking、独立 AArch32 payload 或重新构建 AArch64 core ABI。
- armGBA-pico README 明确声明与 gpSP 共用 `/saves/gba` 的 128 KiB 电池存档缓冲区；没有明确声明与 Linkslot mGBA 字节兼容。
- 本表记录的是初始审查基线，不代表所有上游许可、子模块和构建产物已经完成发行审查。