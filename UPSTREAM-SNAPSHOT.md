# Upstream snapshot — linkSlot

这是一份**只读归档快照**，用于长期保存某个时间点的 linkSlot 源码树。它不是一个开发分支，
不接受补丁，也不代表任何维护承诺。

## 项目

| 项 | 值 |
| --- | --- |
| 项目名称 | linkSlot（Game Boy 系掌机前端，Anbernic H700 系列） |
| 项目版本 | `1.0.36`（`Cargo.toml` 的 `[workspace.package].version`） |
| 版本佐证 | `CHANGELOG.md` 首条为 `## v1.0.36（2026-11-22 构建）`；同期发行包命名为 `linkslot-v1.0.36-h700.zip` |
| 许可证 | GPL-3.0-or-later（`LICENSE`、`NOTICE`、`Cargo.toml` 三处一致） |
| 版权 | Copyright (C) 2026 luguiyi691（见 `NOTICE`） |
| 上游派生自 | slot v1.6.0（2026-10-10 发布），https://github.com/slot-cfw/slot |

## 快照日期

2026-11-23（本地时间，UTC+8）。

## 上游来源

| 项 | 值 |
| --- | --- |
| 上游项目主页（`NOTICE` 记录） | https://github.com/luguiyi/linkslot |
| 上游基线 | slot v1.6.0，https://github.com/slot-cfw/slot |
| 原始 Git commit | **不可得**。见下节。 |
| 原始 Git tag | **不可得**。 |

### 关于原始 commit / tag

本快照的输入是一份不含 `.git` 的源码归档（zip 导出），因此归档本身不携带 commit id、
分支名或 tag。为避免把推断写成事实，这里不指定 commit。

可供交叉核对的外部事实（2026-11-23 查询 GitHub API）：

- `luguiyi/linkslot` 公开仓库的默认分支 `main` 停在 commit `9275419ca99846fe6738027b2ade72e1804fdc08`
  （2026-11-15，提交说明 `chore: drop the last local-only path and example LAN IP`）；
- 该 commit 的 `Cargo.toml` 版本为 `1.0.0`，而本快照为 `1.0.36`；
- 该仓库没有任何 tag（`/tags` 返回空列表）。

结论：本快照**晚于**公开上游的 `main`，无法对应到任何一个公开 commit 或 tag。它的身份只能由
本文件记录的版本号、日期与 `SHA256SUMS.txt` 共同确定。

## 工具链与构建系统

| 项 | 值 |
| --- | --- |
| 语言 / 版本 | Rust `1.96`（`rust-toolchain.toml` 的 `channel`；`Cargo.toml` 的 `rust-version`；edition `2021`） |
| 工具链组件 | `rustfmt`、`clippy` |
| 构建容器 | `build/container/Dockerfile`，基座 `rust:1.96-bullseye`（设备为 Debian 11 / glibc 2.31） |
| workspace | Cargo workspace，成员为 `crates/*`（14 个 crate） |
| 任务入口 | `taskfile.yml`（Task v3），薄封装到 `build/dist.sh` |
| 出包实现 | `build/dist.sh`（`h700` 整包 / `host` 桌面构建），辅以 `build/package.py` 等 Python 3 脚本 |
| 交叉工具 | `cmake`、`build-essential`（g++/make）、`zip`、`file`、`python3`、`git`、`curl` |

## 本次纳入范围

快照内容 = 输入归档的完整文件集合，未做任何内容修改、重命名或"顺手清理"：
838 个普通文件，连同 `.gitattributes`、`.gitignore`、`Cargo.lock`、构建脚本、
`crates/*/tests` 与 `tests/fixtures`、`card/` 卡内资源、`cores/` 的源码补丁、
`third_party/` 与 `licenses/` 全部保留。

## 本次排除的文件和目录及原因

| 排除项 | 原因 |
| --- | --- |
| `.git/` | 输入归档本身不含版本库元数据；快照使用独立的新历史 |
| `target/`、`target-device/`、`target-verify/`、`target-brick/`、`dist/`、`artifacts/` | 构建产物，可由源码重建；输入归档中本来也不存在 |
| `node_modules/`、`__pycache__/`、`*.rs.bk`、`*.pdb` | 缓存与调试产物；输入归档中本来也不存在 |
| `.cargo/config.toml`、`.idea/`、`.vscode/`、`.zed/` | 本机配置与编辑器目录；输入归档中本来也不存在 |
| `*.log`、`*.orig`、`*.rej`、`Thumbs.db`、`.DS_Store` | 日志与临时文件；输入归档中本来也不存在 |
| 发行包 `linkslot-v1.0.36-h700.zip`、MOD 包、`.lsp` 扩展包 | 属于分发产物而非源码，不纳入源码快照；另行存放在本地分发目录中 |

需要说明的是：除 `.git/` 外，上表其余条目在输入归档中**均不存在**，排除规则是声明性的，
没有实际删除任何文件。快照的 838 个文件与输入归档逐字节一致（见 `SHA256SUMS.txt` 复核方法）。

## submodule / LFS 状态

| 项 | 状态 |
| --- | --- |
| Git submodule | 无。仓库根目录没有 `.gitmodules`，全部依赖为 Cargo 依赖或源码内第三方目录 |
| Git LFS | 无。`.gitattributes` 只包含行尾与二进制属性规则，不含 `filter=lfs`；全树未发现 LFS 指针文件（`git-lfs.github.com/spec`） |
| 大文件 | 最大单文件 1,852,424 B（中文字体子集 `crates/slot-ui/assets/SlotSimplifiedChinese.otf`），无超过 90 MiB 的文件 |

## 第三方组件

`third_party/`（rcheevos、fceumm、gpsp、mgba、snes9x）、`cores/` 的源码补丁、
以及 `crates/slot/src/web/play_data/` 下的 libretro wasm 核心与 EmulatorJS
都是上游源码树的一部分，各自的许可证文本保存在 `licenses/`。
本快照按原样保留，不额外授予任何权利；使用与再分发请遵循 `LICENSE` 与 `licenses/` 中的条款。

## 校验方法

```
sha256sum -c SHA256SUMS.txt
```

`SHA256SUMS.txt` 使用相对路径、按路径排序，覆盖除自身以外的全部普通文件。
