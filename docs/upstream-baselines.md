# 上游源码基线

本文记录可核验的上游来源、用户提供的源码快照和正式导入规则。**源码快照归档 commit 与原始上游 commit 必须分开记录**；归档分支中的文件树不是原始 Git 历史。

| 来源 | Ref / commit | 在本项目中的用途 |
|---|---|---|
| [linkSlot 三分支最新快照](https://github.com/dengzhan007/Linkslot-Pico-for-H700/tree/upstream/linkslot) | 本仓库归档分支 HEAD `6122c8f2357ecae43295fdb1a158ff5ef7488325`；原始 GitCode commit 见下表 | 当前 Linkslot 业务、数据格式和功能的只读来源 |
| [luguiyi/linkslot 公共仓库](https://github.com/luguiyi/linkslot) | 查询记录：`main` @ `9275419ca99846fe6738027b2ade72e1804fdc08` | 公开仓库基线；当前公开 main 的 Cargo version 为 1.0.0，不等同于用户提供的私有 GitCode 快照 |
| [GammaOS LK](https://github.com/TheGammaSqueeze/lk-mt6785) | `lk-gba-emu-sd-card-kpa` @ `9bf1419a49e0af6dba8b05916df382c9c9c6d0a8` | 裸机运行时/Core ABI 参考；该分支当前是 AArch32 LK，不是 H700 AArch64 port |
| [armGBA-pico](https://github.com/TheGammaSqueeze/armGBA-pico) | `main` @ `75dbb3108b16d7fa72e60e523a0e4ec3b14af122` | GBA 核心候选；A32 固定地址 blob、Core ABI、过滤和 cheats 研究参考 |
| [gambatte-pico](https://github.com/TheGammaSqueeze/gambatte-pico) | GammaOS LK submodule @ `7d6a1d3def653b71f2ed03c02829277868511932` | GB/GBC 裸机集成参考；首选核心候选 |
| [gpsp-pico](https://github.com/TheGammaSqueeze/gpsp-pico) | GammaOS LK submodule @ `c3c909554a1b9a968c527b1f8153bba2fc57b0d1` | GBA 备选核心与 Pico Core ABI 适配参考 |

## Linkslot 三分支快照身份（2026-11-27）

快照说明文件：[UPSTREAM-SNAPSHOT.md](https://github.com/dengzhan007/Linkslot-Pico-for-H700/blob/upstream/linkslot/UPSTREAM-SNAPSHOT.md)。本次来源为 GitCode 私有仓库 `luguiyi691/linkslot`，同步时间 **2026-11-27 20:47（UTC+8）**。已枚举远端 `main`、`DEV`、`test` 三个分支。访问令牌仅用于镜像读取，未写入仓库、脚本或清单。

| 本地快照路径 | 原始来源分支 | 原始来源 commit | 来源提交时间（UTC+8） | Cargo 版本 | 源码文件数 |
|---|---|---|---|---:|---:|
| `upstream/linkslot/` 根目录 | `main` | `b84f37f1a6259a625e9095279dd132f5ef2116af` | 2026-11-25 10:39 | 1.0.49 | 851 |
| `upstream/linkslot/DEV/` | `DEV` | `ebc6094890dd02210720b79a94fe64e3f11d30fc` | 2026-11-26 09:13 | 1.1.0 | 875 |
| `upstream/linkslot/test/` | `test` | `b1342bed9deca313ae3954f0022c941af4e0b031` | 2026-11-27 20:39 | 1.0.60 | 854 |

以上 SHA 是 GitCode 源仓库提交，不是本仓库归档提交。本仓库只读归档分支 HEAD 为 `6122c8f2357ecae43295fdb1a158ff5ef7488325`。

### 相对上一版快照的变化

上一版同步于 2026-11-25 20:40（UTC+8），其 `main` 为 `b84f37f`（1.0.49）、`DEV` 为 `a800adb`（1.1.0，872 文件）、`test` 为 `8c7a370`（1.0.49，851 文件）。

| 分支 | 上一版 | 本次 | 相对上一版 |
|---|---|---|---|
| `main` | `b84f37f`（1.0.49） | `b84f37f`（1.0.49） | **无变化**：0 个文件变更，源码树逐字节相同 |
| `DEV` | `a800adb`（1.1.0，872 文件） | `ebc6094`（1.1.0，875 文件） | 57 个文件变更，+3035 / −2287；3 新增、0 删除 |
| `test` | `8c7a370`（1.0.49，851 文件） | `b1342be`（1.0.60，854 文件） | 85 个文件变更，+9042 / −6455；3 新增、0 删除 |

新增文件：

- `DEV/`：`crates/slot-ui/assets/SlotSimplifiedChinese.ttf`、`crates/slot-ui/src/paper.rs`、`crates/slot-ui/src/shelf_header.rs`
- `test/`：`crates/slot-ui/assets/SlotSimplifiedChinese.ttf`、`crates/slot-ui/src/shelf_header.rs`、`small-draw-calls.txt`

### 三分支差异与稳定基线判断

- **`main` 仍是首期移植的稳定对照基线：** 版本 1.0.49、851 个源码文件；本次相对上一版快照无任何文件变化。此前已围绕 main 1.0.49 建立的 H700 移植结论不会因为这次源快照同步而自动失效。
- **`test` 已不再与 `main` 相同：** 当前为 1.0.60，有 85 个文件变化和 3 个新增文件。其最新 changelog 包括 Web 批量导入/导出与菜单图标细化；v1.0.50–1.0.60 还包含较大范围的 UI/HUD/通知/传输界面调整。它是需要审查的上游候选，不是当前稳定主线，不应直接替换 main 基线。
- **`DEV` 仍是独立的下一代功能分支：** 当前 1.1.0、875 个文件；相对上次快照 57 个文件变化，新增内容集中于 Arcade/NeoGeo/FBNeo、platform-pack 和相关 UI/QA。它不是首期 GB/GBC/GBA 垂直切片的直接升级目标。
- 三个分支均按各自来源 commit 的树原样归档，没有在同步时进行分支合并、挑选或移植。每份快照均带基于 Git blob 字节生成的 `SHA256SUMS.txt`；文件 mode 按源树保留。

### 对当前 H700 移植的直接影响

- 首期继续以 **main 1.0.49** 为业务/API 与数据格式基线；本次确认 main 未变化，因此无需因快照刷新而重做所有 1.0.49 兼容性验证。
- 将 `test` 1.0.60 作为差异审查来源，优先检查对 GB/GBC/GBA 主界面、游戏选择、输入、渲染和状态持久化有影响的改动；Web 批量导入/导出及视觉资源不自动进入裸机首期。
- `DEV` 1.1.0 的机种/核心模型及平台包数据契约继续作为后续扩展参考；首期不提前引入 Arcade、NeoGeo、FBNeo 或动态平台包机制。
- `crates/slot-retro/src/ffi.rs` 的旧 ABI 兼容结论仍需以各分支当前 blob SHA 分别核验；不得因目录名或版本号相近，将旧 patcher 锚点命中当作新源码兼容证明。
- `slot-model/src/core.rs` 与 `crates/slot/src/app.rs` 等关键业务入口应继续做精确 blob/差异审查。尤其 `test` 1.0.60 的 85 文件更新不能只凭 changelog 概括为“仅清理警告”。

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
