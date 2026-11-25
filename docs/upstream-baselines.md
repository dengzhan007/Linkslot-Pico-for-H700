# 上游源码基线

本文记录可核验的上游来源、用户提供的源码快照和正式导入规则。**源码快照归档 commit 与原始上游 commit 必须分开记录**；归档分支中的文件树不是原始 Git 历史。

| 来源 | Ref / commit | 在本项目中的用途 |
|---|---|---|
| [linkSlot 三分支最新快照](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/upstream/linkslot) | 本仓库归档分支 HEAD `241e7c1974926fa22552b200852914ceaccbfd42`；原始 GitCode commit 见下表 | 当前 Linkslot 业务、数据格式和功能的只读来源 |
| [luguiyi/linkslot 公共仓库](https://github.com/luguiyi/linkslot) | 查询记录：`main` @ `9275419ca99846fe6738027b2ade72e1804fdc08` | 公开仓库基线；当前公开 main 的 Cargo version 为 1.0.0，不等同于用户提供的私有 GitCode 快照 |
| [GammaOS LK](https://github.com/TheGammaSqueeze/lk-mt6785) | `lk-gba-emu-sd-card-kpa` @ `9bf1419a49e0af6dba8b05916df382c9c9c6d0a8` | 裸机运行时/Core ABI 参考；该分支当前是 AArch32 LK，不是 H700 AArch64 port |
| [armGBA-pico](https://github.com/TheGammaSqueeze/armGBA-pico) | `main` @ `75dbb3108b16d7fa72e60e523a0e4ec3b14af122` | GBA 核心候选；A32 固定地址 blob、Core ABI、过滤和 cheats 研究参考 |
| [gambatte-pico](https://github.com/TheGammaSqueeze/gambatte-pico) | GammaOS LK submodule @ `7d6a1d3def653b71f2ed03c02829277868511932` | GB/GBC 裸机集成参考；首选核心候选 |
| [gpsp-pico](https://github.com/TheGammaSqueeze/gpsp-pico) | GammaOS LK submodule @ `c3c909554a1b9a968c527b1f8153bba2fc57b0d1` | GBA 备选核心与 Pico Core ABI 适配参考 |

## Linkslot 最新三分支快照身份（2026-11-25）

快照说明文件：[UPSTREAM-SNAPSHOT.md](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/upstream/linkslot/UPSTREAM-SNAPSHOT.md)。本轮来源是 GitCode / AtomGit 私有仓库 `luguiyi691/linkslot`，同步时间 2026-11-25 20:40（UTC+8）。令牌未进入项目仓库、脚本或清单。

| 本地快照路径 | 原始来源分支 | 原始来源 commit | 来源提交时间（UTC+8） | Cargo 版本 | 源码文件数 |
|---|---|---|---|---:|---:|
| `upstream/linkslot/` 根目录 | `main` | `b84f37f1a6259a625e9095279dd132f5ef2116af` | 2026-11-25 10:39 | 1.0.49 | 851 |
| `upstream/linkslot/DEV/` | `DEV` | `a800adb2e3a51d05ffe80abeae2e1ec52247b951` | 2026-11-25 11:40 | 1.1.0 | 872 |
| `upstream/linkslot/test/` | `test` | `8c7a37055eb68f71170320d4a17cdb9251785186` | 2026-11-25 10:37 | 1.0.49 | 851 |

以上三个 SHA 是原始来源仓库提交，不是本仓库归档 commit。归档分支 HEAD 为 `241e7c1974926fa22552b200852914ceaccbfd42`。三个版本均附各自的 `SHA256SUMS.txt`；快照说明记录了复制的源文件清单、blob 字节哈希和文件 mode。

### 三分支差异核验

- `main` 与 `test` 的 851 个源码文件内容完全一致；test 快照仅不含根目录归档说明文件 `UPSTREAM-SNAPSHOT.md`。因此当前稳定基线选择 `main 1.0.49`，`test 1.0.49` 作为同内容交叉核对，不需要重复移植。
- `DEV 1.1.0` 相对 main：文件树比较为新增 21、删除 1、修改 46。新增内容集中于平台包、Arcade/NeoGeo、FBNeo、DAT 目录和检修门；修改覆盖 `slot-model`、`slot-store`、`slot-ui`、`slot`、翻译、打包及扩展包管理。
- DEV 的主要结构变化包括新增 `Platform::Arcade` / `Platform::Neogeo`，增加 FBNeo 核心，并通过 schema 2 / `kind=platform` 的 LSP 平台包按安装状态激活机种。它不是简单的 1.0.49 bugfix 分支。

### 对当前 H700 移植的直接影响

- 1.0.49 是首期 GB/GBC/GBA 移植的稳定对照基线；main 与 test 无需分别适配。
- DEV 1.1.0 作为下一代架构差异来源单独保留。当前裸机首期仍只实现 GB/GBC/GBA，不提前引入 Arcade、NeoGeo、FBNeo 或动态平台包机制；但机种/核心表与存储布局不得设计成无法向后扩展。
- `crates/slot-retro/src/ffi.rs` 在旧 1.0.45、main 1.0.49、DEV 1.1.0 三份快照中的 blob SHA 均为 `2e7b1b0bb3e1665de01f6458b6dfc7fe65a61f34`（7,649 bytes）。此前 21 个 mGBA namespaced exports 的静态 API 链接验证仍有明确的 ABI 兼容依据，但仍需在新快照上重新执行完整 patch/build workflow。
- `slot-model/src/core.rs` 已发生变化：1.0.45 blob `d7fe752e7968d00dafaa3dbadacf2a749580a64e`（4,750 bytes），main 1.0.49 为 `cd877949a3b30c62cf178fd91add3e258a09c66d`（5,761 bytes），DEV 1.1.0 为 `51dd6ed87d1d3c2e80e18d6759ad687b22b5c481`（6,199 bytes）。H700 持久化 no_std 核心表需要从 1.0.45 重新对齐到 1.0.49，保留本地 no_std/alloc 边界和已验证的核心元数据。
- `crates/slot/src/app.rs` 从 1.0.45 到 main 1.0.49、DEV 1.1.0 均有大幅变化（725,712 → 760,471 → 830,714 bytes）。不能把旧 patcher 的锚点命中或旧版本编译通过视为新版本兼容证明。
- 1.0.49 changelog 包含 armGBA netpacket 联机修复、Web 日志维护和云同步/卡带传输 UI 更新。armGBA 的 netpacket 语义应保留；当前 H700 尚未完成的网络后端不应因此被假定已支持联机。

## 历史快照

- Linkslot 1.0.36 旧快照曾归档于本仓库 `upstream/linkslot` @ `8590d6ff1b2caa26118051069c975106fbb14bd3`。
- Linkslot 1.0.45 旧快照 pin 为 `3b06d96adadd0375f071555ecb3cc20c3a5b53db`，相关适配记录保留在 [1.0.45 portability update](linkslot-1.0.45-portability-update.md)。两者均为历史对照，不再是最新导入目标。

## 正式基线锁定规则

1. 新快照分支 `upstream/linkslot` 继续只读；不得在其中写入 H700 补丁。
2. 先以 main 1.0.49 重新验证 H700 的 no_std 业务切片、renderer overlay、mGBA 静态 ABI、newlib/VFS 及 patcher 锚点。
3. 同时审查 DEV 1.1.0 的机种/核心模型和 platform-pack 数据契约，但不把 Arcade/NeoGeo 功能并入首期垂直切片。
4. 若取得原始 Git 历史，记录原始分支、tag、commit SHA 与时间；若只有源码快照，继续标记为 archive snapshot，不伪造 subtree 历史。
5. 在完成版本差异、许可、功能和数据格式审查前，不把整个 workspace 无差别并入 H700 根 workspace。

## 特别说明

- `lk-mt6785` 中的 MTK CONNSYS、PMIC、GPIO、显示、USB PHY、DMA 和电源寄存器配置不得直接用于 Allwinner H700。
- 当前 GammaOS LK 参考分支只有 AArch32 `arch/arm` 与 x86 架构目录；本项目已有 AArch64 freestanding 编译 scaffold，两者的入口/ABI选择必须先解决。
- `armGBA-pico` 的现成 blob 面向 32-bit ARM LK host，不能直接由 H700 AArch64 host 加载；要另行选择 AArch32 interworking、独立 AArch32 payload 或重新构建 AArch64 core ABI。
- armGBA-pico README 明确声明与 gpSP 共用 `/saves/gba` 的 128 KiB 电池存档缓冲区；没有明确声明与 Linkslot mGBA 字节兼容。
- 本表记录的是研究来源，不代表所有上游许可、子模块和构建产物已经完成发行审查。
