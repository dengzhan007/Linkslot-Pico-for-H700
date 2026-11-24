# 上游源码基线

本文记录可核验的上游来源、用户提供的源码快照和正式导入规则。**源码快照 commit 与原始上游 commit 必须分开记录**：没有原始 Git 元数据时，不得推断或伪造上游 commit/tag。

| 来源 | Ref / commit | 在本项目中的用途 |
|---|---|---|
| [linkSlot 1.0.36 用户源码快照](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/upstream/linkslot) | 本仓库 `upstream/linkslot` @ `8590d6ff1b2caa26118051069c975106fbb14bd3` | 当前 Linkslot UI/业务/数据格式审查基线；输入为无 .git 的 ZIP，归档 commit 不是上游 commit |
| [luguiyi/linkslot 公共仓库](https://github.com/luguiyi/linkslot) | 查询记录：`main` @ `9275419ca99846fe6738027b2ade72e1804fdc08` | 公开仓库基线；该版本 Cargo version 为 1.0.0，不等同于用户提供的 1.0.36 |
| [GammaOS LK](https://github.com/TheGammaSqueeze/lk-mt6785) | `lk-gba-emu-sd-card-kpa` @ `9bf1419a49e0af6dba8b05916df382c9c9c6d0a8` | 裸机运行时/Core ABI 参考；该分支当前是 AArch32 LK，不是 H700 AArch64 port |
| [armGBA-pico](https://github.com/TheGammaSqueeze/armGBA-pico) | `main` @ `75dbb3108b16d7fa72e60e523a0e4ec3b14af122` | GBA 核心候选；A32 固定地址 blob、Core ABI、过滤和 cheats 研究参考 |
| [gambatte-pico](https://github.com/TheGammaSqueeze/gambatte-pico) | GammaOS LK submodule @ `7d6a1d3def653b71f2ed03c02829277868511932` | GB/GBC 裸机集成参考；首选核心候选 |
| [gpsp-pico](https://github.com/TheGammaSqueeze/gpsp-pico) | GammaOS LK submodule @ `c3c909554a1b9a968c527b1f8153bba2fc57b0d1` | GBA 备选核心与 Pico Core ABI 适配参考 |

## Linkslot 1.0.36 快照身份

快照信息见 [UPSTREAM-SNAPSHOT.md](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/upstream/linkslot/UPSTREAM-SNAPSHOT.md)：

- Cargo version：`1.0.36`；构建日期：2026-11-22。
- Rust channel / rust-version：`1.96`。
- 838 个源码文件，加 `UPSTREAM-SNAPSHOT.md` 和 `SHA256SUMS.txt`；无 submodule / Git LFS。
- 原始输入 ZIP 不包含 `.git`，原始上游 commit/tag **不可得**。
- SHA-256 清单覆盖除清单自身外的所有普通文件。

当前用户提供的快照比公开 `luguiyi/linkslot/main`（上表的 1.0.0 公开基线）更新。不能把 1.0.36 归档提交 `8590d6ff...` 表述为 luguiyi/linkslot 的真实源码 commit。

## 正式基线锁定规则

1. 当前先以 `upstream/linkslot` 的 1.0.36 归档树作为静态审查基线，不修改该只读快照。
2. 完成逐 crate 复用审计、LK ABI/工具链可行性验证和用户功能/数据格式回归计划。
3. 若未来取得带原始 Git 历史的 1.0.36 或更新上游仓库，记录 tag、真实 commit SHA、发布时间与差异，再决定 subtree 同步。
4. 若仍只有 ZIP 快照，必须继续明确标记为 archive snapshot；可以按文件树/hash 做版本比较，但不能宣称保留了原始上游 Git 历史。
5. 只有经过审计后才决定将快照导入 `third_party/linkslot`。不得先把整个 workspace 无差别并入 H700 根 workspace。

## 特别说明

- `lk-mt6785` 中的 MTK CONNSYS、PMIC、GPIO、显示、USB PHY、DMA 和电源寄存器配置不得直接用于 Allwinner H700。
- 当前 GammaOS LK 参考分支只有 AArch32 `arch/arm` 与 x86 架构目录；本项目已有 AArch64 freestanding 编译 scaffold，两者的入口/ABI选择必须先解决。
- `armGBA-pico` 的现成 blob 面向 32-bit ARM LK host，不能直接由 H700 AArch64 host 加载；要另行选择 AArch32 interworking、独立 AArch32 payload 或重新构建 AArch64 core ABI。
- armGBA-pico README 明确声明与 gpSP 共用 `/saves/gba` 的 128 KiB 电池存档缓冲区；没有明确声明与 Linkslot mGBA 字节兼容。
- Linkslot 1.0.36 当前有 mGBA / gpSP Libretro 核心构建流程；未来 armgba 尚未出现在本快照的正式 GBA core manifest 中。
- 本表记录的是研究来源，不代表所有上游许可、子模块和构建产物已经完成发行审查。
