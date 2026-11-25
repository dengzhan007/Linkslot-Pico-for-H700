# Linkslot 1.0.49 / 1.1.0 三分支更新审查

**审查日期：** 2026-11-25  
**归档分支：** `upstream/linkslot`  
**归档 HEAD：** `241e7c1974926fa22552b200852914ceaccbfd42`  
**移植工作分支：** `porting/linkslot-1.0.45-feasibility`

## 1. 来源与版本确认

依据归档说明 `UPSTREAM-SNAPSHOT.md`、各自的 `Cargo.toml`、`CHANGELOG.md` 和 `SHA256SUMS.txt`：

| 快照 | 原始来源分支 | 原始来源 commit | 版本 | 源码文件 |
|---|---|---|---:|---:|
| 根目录 | main | `b84f37f1a6259a625e9095279dd132f5ef2116af` | 1.0.49 | 851 |
| `DEV/` | DEV | `a800adb2e3a51d05ffe80abeae2e1ec52247b951` | 1.1.0 | 872 |
| `test/` | test | `8c7a37055eb68f71170320d4a17cdb9251785186` | 1.0.49 | 851 |

来源为私有 GitCode / AtomGit 仓库 `luguiyi691/linkslot`，同步时间 2026-11-25 20:40（UTC+8）。原始来源 commit 是快照说明记录的真实 commit；`241e7c...` 是本项目的归档提交，二者不能混用。

## 2. 文件树比较

通过 Git Trees API 对归档 HEAD 的递归树按路径去掉 `DEV/` / `test/` 前缀后逐 blob SHA 比较：

- main 和 test 的 851 个源码文件逐 blob 一致；test 不包含根目录的 `UPSTREAM-SNAPSHOT.md` 归档说明，代码与清单内容相同。
- DEV 相对 main：新增 21、删除 1、修改 46。差异不是只改版本号。
- main 与 test 的相同源码基线意味着首期只需对 main 1.0.49 做一次移植回归；test 用来交叉核对，不另开一套适配。

## 3. main/test 1.0.49 的主要影响

1.0.49 changelog 的近期变化包括：

- 1.0.46–1.0.47：armGBA netpacket 联机能力接入及配对/跟随主机修复；核心联机会话能否运行仍取决于 H700 网络后端，源码支持不代表本项目已经支持。
- 1.0.48：Web 日志清理与日志级别配置维护功能。
- 1.0.49：云同步与卡带传输界面重做，进度展示改为真实阶段/字节数据语义，不再展示伪造百分比。

首期功能范围仍是 GB/GBC/GBA。需要保留 armGBA core metadata 与 netpacket ABI 语义；Web 管理、云同步、卡带传输和复杂联网功能等依赖完整文件/网络/持久化服务的能力，按 HAL 完成情况分阶段接入。

## 4. DEV 1.1.0 的结构变化

DEV 的首条 changelog 为 1.1.0，核心增量是 schema 2 / `kind=platform` LSP 平台包：

- 增加 Arcade、NeoGeo 两个可选机种；它们不在 `Platform::BUILTIN`，只有有效平台包激活后才扫描和显示。
- 增加 FBNeo 核心元数据、Arcade/NeoGeo DAT 扫描、街机 PCB 机柜/投币/检修门状态与 UI。
- 新增 `slot-store::platform_packs`、`arcade_dat`，以及 `slot::arcade`、`slot-ui::cabinet` 等模块。
- 平台包有独立的文件路径白名单、单包/单文件/总负载上限、已登记文件卸载和损坏登记文件回退规则；FBNeo 具有单独的 Non-commercial 许可文件，未来采用时必须保留许可与源码义务。

对 H700 的设计结论：保留可扩展机种/核心模型，但不在首期实现平台包管理、Arcade、NeoGeo、FBNeo 或 DAT 数据链。未来若移植平台包，需要独立设计只读验证、事务安装/卸载和损坏恢复，不能照搬 Linux 文件操作。

## 5. 关键 API / 文件兼容性核验

| 文件 | 1.0.45 | main 1.0.49 | DEV 1.1.0 | 结论 |
|---|---|---|---|---|
| `crates/slot-retro/src/ffi.rs` | `2e7b1b0bb3e1665de01f6458b6dfc7fe65a61f34` / 7,649 B | 同 SHA | 同 SHA | 静态 Libretro API 的关键 FFI 定义未改，21 个 mGBA namespaced exports 探针有继续复用基础；仍需新源码重新执行构建 |
| `crates/slot-model/src/core.rs` | `d7fe752e7968d00dafaa3dbadacf2a749580a64e` / 4,750 B | `cd877949a3b30c62cf178fd91add3e258a09c66d` / 5,761 B | `51dd6ed87d1d3c2e80e18d6759ad687b22b5c481` / 6,199 B | core registry 已变化；H700 持久化 no_std 子集不能继续停留在 1.0.45 |
| `crates/slot/src/app.rs` | `92e10628b3cccd476e96e5b00f3449e6f9311b19` / 725,712 B | `14952584a6c6708394d65668ffb18356c1ffa2bf` / 760,471 B | `27cda475638d41aa76a7ea264ab1927bda8f81bf` / 830,714 B | App 改动较大，旧 overlay/patcher anchor 必须重新验证 |

这些比较只证明源码 blob 相同或不同，不等于已经完成 API 语义、编译和行为验证。

## 6. 对现有 H700 代码和 CI 的行动

持久化集成代码已开始迁移：`integration/linkslot/h700-frontend-core/slot-model/src/core.rs` 已按 main 1.0.49 的 blob `cd877949a3b30c62cf178fd91add3e258a09c66d` 同步；本地 crate manifest 已升至 1.0.49，H700 contract test 增加 gpSP/armGBA/mGBA netpacket 断言。提交：`ac0e8e0` / `977519e` / `3e986dc`。

两条 patcher 已更新为归档 HEAD `241e7c1974926fa22552b200852914ceaccbfd42` 与 workspace 1.0.49：`20a3ee9`、`e9faf2d`；runtime workflow 也改为 1.0.49 命名：`d557c54`。首轮 runtime workflow [#45623895416](https://github.com/dengzhan007/Linkslot-Pico-for-H700/actions/runs/45623895416) 正在执行；截至本次记录，持久化 no_std workspace tests、renderer + business overlay 应用、上游 input/model regression 均成功，完整 Frontend 的 `cargo check` 正在运行，尚不能标记整条 workflow 通过。

对当前持久化集成代码做了进一步只读检查，发现一个具体的功能性差异：`integration/linkslot/h700-frontend-core/slot-model/src/core.rs` 仍是 1.0.45 风格的 `CoreSpec`，只有 `frameskip` / `link`，没有 1.0.49 新基线的 `netpacket` 字段；其中 armGBA 注释仍写单人游玩。1.0.49 的上游核心表已经把 gpSP 和 armGBA 的 `netpacket` 声明为 true、mGBA 声明为 false。这与 1.0.46–1.0.47 的 armGBA netpacket 联机修复相对应。**因此 H700 no_std 核心模型目前不只是版本落后，还会丢失 armGBA 联机能力元数据**；迁移时要把字段、所有 CoreSpec 常量和测试一并同步，不能只复制上游一个文件而遗漏依赖字段/测试。

持久化 `machine.rs` 目前仍只有 GBA/GB/GBC/FC/SFC/MD 六个机种。对第一期 GB/GBC/GBA 切片这是有意的范围限制，暂不引入 DEV 的 Arcade/NeoGeo；未来添加机种时必须遵守上游平台枚举与历史 `slot.state` 槽序只能追加的约束。

当前 H700 patcher `scripts/apply-linkslot-h700-frontend-core.py` 与 `scripts/apply-linkslot-render-core.py` 仍硬编码 1.0.45 archive pin `3b06d96adadd0375f071555ecb3cc20c3a5b53db` 和 1.0.45 workspace version。workflow `.github/workflows/h700-linkslot-runtime.yml` 会 checkout `upstream/linkslot` 的最新分支头；现在该头已是包含 main 1.0.49 根目录快照的归档 commit `241e7c1974926fa22552b200852914ceaccbfd42`。因此若现在直接运行该 workflow，旧 patcher 会在严格 SHA/version gate 拒绝新快照。这是根据现有脚本与 ref 的确定性不匹配判断，**不是已观察到的一次失败 Actions 运行**。必须先有计划地升级 pin，再验证 patch anchors。

下一步不能只把 pin 换成归档 HEAD；应：

1. 建立 main 1.0.49 的临时隔离源码树，核对 Cargo features、crate manifests 和编译参数。
2. 对 frontend/core、renderer 和三核心选择器 patcher 分别跑 dry-run，检查每个锚点的唯一性和上下文语义。
3. 更新 H700 持久化 `slot-model` 数据子集，保留 no_std/alloc；同步 `CoreSpec::netpacket` 字段、gpSP/armGBA/mGBA 的元数据及其回归测试；机种表先保持六平台。
4. 在新快照上重新运行 slot-retro VFS v3、21 个 mGBA 静态 API binding、newlib whole-archive 和 AArch32 ELF `--no-undefined`。
5. 保持 AArch32 ARM、Thumb-2 与 AArch64 的已有编译矩阵；运行完整 Linkslot/LK feasibility workflow。旧 1.0.45 通过记录不能移作 1.0.49 通过证明。
6. DEV 1.1.0 只补充独立审查/兼容测试，不更改首期 ROM 机种范围。



当前 H700 patcher `scripts/apply-linkslot-h700-frontend-core.py` 与 `scripts/apply-linkslot-render-core.py` 仍硬编码 1.0.45 archive pin `3b06d96adadd0375f071555ecb3cc20c3a5b53db` 和 1.0.45 workspace version。当前 H700 Linkslot runtime slices 最近通过记录也仍然是以 1.0.45 为输入。

下一步不能只把 pin 换成归档 HEAD；应：

1. 建立 main 1.0.49 的临时隔离源码树，核对 Cargo features、crate manifests 和编译参数。
2. 对 frontend/core、renderer 和三核心选择器 patcher 分别跑 dry-run，检查每个锚点的唯一性和上下文语义。
3. 更新 H700 持久化 `slot-model` 数据子集，保留 no_std/alloc；对 `CoreSpec`、机种映射、存档路径和 armGBA 元数据增加回归。
4. 在新快照上重新运行 slot-retro VFS v3、21 个 mGBA 静态 API binding、newlib whole-archive 和 AArch32 ELF `--no-undefined`。
5. 保持 AArch32 ARM、Thumb-2 与 AArch64 的已有编译矩阵；运行完整 Linkslot/LK feasibility workflow。旧 1.0.45 通过记录不能移作 1.0.49 通过证明。
6. DEV 1.1.0 只补充独立审查/兼容测试，不更改首期 ROM 机种范围。

## 7. 当前结论

- **稳定移植基线：** main 1.0.49。
- **源码交叉核对：** test 1.0.49，与 main 源码逐 blob 一致。
- **未来功能输入：** DEV 1.1.0，重点是可选平台包及 Arcade/NeoGeo；先延后。
- **已确认的兼容点：** Libretro `slot-retro::ffi` blob 与 1.0.45 相同。
- **必须重新验证的部分：** `slot-model`、`slot/src/app.rs`、renderer/Frontend overlay 与所有固定源码锚点。
- **没有声称：** 1.0.49/1.1.0 已在 H700/LK 构建、启动或实机运行；本次是源码归档与文件级审查。
