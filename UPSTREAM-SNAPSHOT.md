# Upstream snapshot — linkSlot（GitCode 私有仓库同步）

这是一份**只读归档快照**，用于保存指定时间点的 linkSlot 源码树。它不是开发分支，不接受补丁，
也不代表任何维护承诺。

## 项目

| 项 | 值 |
| --- | --- |
| 项目名称 | linkSlot（Game Boy 系掌机前端，Anbernic H700 系列） |
| 项目版本 | `main` = `1.0.49`，`DEV` = `1.1.0`，`test` = `1.0.60`（各自 `Cargo.toml` 的 `[workspace.package].version`） |
| 版本佐证 | `CHANGELOG.md` 首条：`main` 为 `## v1.0.49（2026-11-25 构建）`，`DEV/` 为 `## v1.1.0（2026-11-24 构建）`，`test/` 为 `## v1.0.60（2026-11-27 构建）` |
| 许可证 | GPL-3.0-or-later（`LICENSE`、`NOTICE`、`Cargo.toml` 一致） |
| 版权 | Copyright (C) 2026 luguiyi691（见 `NOTICE`） |
| 上游派生自 | slot v1.6.0（2026-10-10 发布），https://github.com/slot-cfw/slot |

## 本次来源

| 项 | 值 |
| --- | --- |
| 来源平台 | GitCode（私有仓库） |
| 仓库 | `luguiyi691/linkslot`，分支 `main`、`DEV`、`test`（本次已核对全部远端分支，无新增分支） |
| 同步时间 | 2026-11-27 20:47（UTC+8） |
| 读取方式 | 使用账号访问令牌经 HTTPS `git clone --mirror`；令牌未写入本仓库、脚本或清单，克隆完成后镜像目录已删除 |

## 各分支快照位置

**根目录是 `main` 分支**，另外两个分支各自归档为子目录：

| 目录 | 源分支 | 原始 commit | 提交时间 | 版本 | 源码文件数 |
| --- | --- | --- | --- | --- | ---: |
| `./`（根目录） | `main` | `b84f37f1a6259a625e9095279dd132f5ef2116af` | 2026-11-25 10:39 +08 | 1.0.49 | 851 |
| `DEV/` | `DEV` | `ebc6094890dd02210720b79a94fe64e3f11d30fc` | 2026-11-26 09:13 +08 | 1.1.0 | 875 |
| `test/` | `test` | `b1342bed9deca313ae3954f0022c941af4e0b031` | 2026-11-27 20:39 +08 | 1.0.60 | 854 |

原始提交说明：`main` = `!3 merge test into main`；`DEV` = `Merge branch 'test' into DEV`；
`test` = `chore(slot): 清理未使用的代码和编译警告`。

## 相对上一版快照

上一版同步于 2026-11-25 20:40，`main` `b84f37f`、`DEV` `a800adb`、`test` `8c7a370`。

| 分支 | 上一版 | 本次 | 相对上一版 |
| --- | --- | --- | --- |
| `main` | `b84f37f`（1.0.49） | `b84f37f`（1.0.49） | **无变化**：0 个文件变更，源码树逐字节相同 |
| `DEV` | `a800adb`（1.1.0，872 文件） | `ebc6094`（1.1.0，875 文件） | 57 个文件变更，+3035 / −2287；3 新增、0 删除 |
| `test` | `8c7a370`（1.0.49，851 文件） | `b1342be`（1.0.60，854 文件） | 85 个文件变更，+9042 / −6455；3 新增、0 删除 |

新增文件：

- `DEV/`：`crates/slot-ui/assets/SlotSimplifiedChinese.ttf`、`crates/slot-ui/src/paper.rs`、`crates/slot-ui/src/shelf_header.rs`
- `test/`：`crates/slot-ui/assets/SlotSimplifiedChinese.ttf`、`crates/slot-ui/src/shelf_header.rs`、`small-draw-calls.txt`

需要注意的三点：

1. **`main` 本次没有前移**，仍是 2026-11-25 的 `!3 merge test into main`；而 `test` 已在 2026-11-27 20:39 前移到 `1.0.60`，因此 `main` 目前落后于 `test`。
2. 上一版 `main` 与 `test` 的源码树完全相同；本次**两者已不同**（`test` 为 1.0.60，`main` 仍为 1.0.49）。
3. `DEV` 的 head 是 2026-11-26 09:13 的 `Merge branch 'test' into DEV`，早于 `test` 的 2026-11-27 提交，因此 `DEV/` 不包含 `test` 最新一轮的清理提交。

三个分支之间仍然**没有做合并、挑选或补丁**：各自按其 commit 的树原样归档，`DEV`/`test` 目录内容与
各自分支 tip 逐 blob 相同。

## 纳入范围与检查

- 完整复制各 commit 的 `git ls-tree -r` 列表：根目录 **851** 个、`DEV/` **875** 个、`test/` **854** 个文件，
  逐字节未修改、未重命名、未"顺手清理"。
- 未包含来源仓库的 `.git` 目录，也未包含任何构建产物、本机配置或临时文件。
- 每份快照各带一份 `SHA256SUMS.txt`，覆盖该目录全部源码文件（路径相对各自目录）；清单自身与
  本说明文件不计入。清单里的哈希取自 **git blob 字节**（不是本机工作区字节），与仓库实际存储的内容一致。
  因 `main` 未变更，根目录 `SHA256SUMS.txt` 与上一版**逐字节相同**，可作为哈希方法的一致性校验。
- 文件权限位逐条取自上游 `ls-tree`，未做统一化：`build/*.sh` 为 `100644`；本版共 **3 个** `100755`
  条目（`cores/armgba/build.sh`、`DEV/cores/armgba/build.sh`、`test/cores/armgba/build.sh`），与上游一致，
  前后对比无权限位变化。
- 只读安全检查（对三个分支分别执行）：`git ls-files` 中无 `.gitmodules`；`.gitattributes` 无 Git LFS
  filter；最大单文件 **7.14 MiB**（`*/crates/slot-ui/assets/SlotSimplifiedChinese.ttf`，本次新增），
  无超过 20 MiB 的文件；对全部源码内容扫描私钥块与常见平台令牌，**0 命中**。
- 许可证文本、NOTICE 与第三方 `licenses/` 目录均随源码原样保留。

## 复现与校验

在根目录校验 `main`：

```sh
sha256sum -c SHA256SUMS.txt
```

在子目录校验其他分支：

```sh
(cd DEV && sha256sum -c SHA256SUMS.txt)
(cd test && sha256sum -c SHA256SUMS.txt)
```

校验前请确认工作区字节与 blob 一致。`*.sh` 由 `.gitattributes` 的 `text eol=lf` 固定为 LF；在
`core.autocrlf=true` 的 Windows 上，其余文本文件检出后会被转成 CRLF，此时应改用

```sh
git -c core.autocrlf=false checkout -- .
sha256sum -c SHA256SUMS.txt
```

或直接在 CI / Linux 上校验。

本快照仅作为归档与来源对照；不包含可刷写固件、设备侧测试结果或硬件验证结论。
