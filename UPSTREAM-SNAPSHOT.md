# Upstream snapshot — linkSlot（GitCode 私有仓库同步）

这是一份**只读归档快照**，用于保存指定时间点的 linkSlot 源码树。它不是开发分支，不接受补丁，
也不代表任何维护承诺。

## 项目

| 项 | 值 |
| --- | --- |
| 项目名称 | linkSlot（Game Boy 系掌机前端，Anbernic H700 系列） |
| 项目版本 | `main` = `1.0.49`，`DEV` = `1.1.0`（各自 `Cargo.toml` 的 `[workspace.package].version`） |
| 版本佐证 | `CHANGELOG.md` 首条：`main` 为 `## v1.0.49（2026-11-25 构建）`，`DEV/` 为 `## v1.1.0（2026-11-24 构建）` |
| 许可证 | GPL-3.0-or-later（`LICENSE`、`NOTICE`、`Cargo.toml` 一致） |
| 版权 | Copyright (C) 2026 luguiyi691（见 `NOTICE`） |
| 上游派生自 | slot v1.6.0（2026-10-10 发布），https://github.com/slot-cfw/slot |

## 本次来源（与上一版快照不同）

| 项 | 值 |
| --- | --- |
| 来源平台 | GitCode / AtomGit（私有仓库） |
| 仓库 | `luguiyi691/linkslot`，分支 `main`、`DEV`、`test` |
| 同步时间 | 2026-11-25 20:40（UTC+8） |
| 读取方式 | 使用账号访问令牌经 HTTPS `git clone --mirror`；令牌未写入本仓库、脚本或清单，克隆完成后已从远端 URL 中移除 |

上一版快照同步于 2026-11-24 17:23，`main` 为 v1.0.45（`25664ec`）。本次三个分支全部前移，
`main` 由 v1.0.45 → **v1.0.49**，`DEV` 由 v1.0.42 → **v1.1.0**，`test` 由 v1.0.45 → **v1.0.49**。

## 各分支快照位置

**根目录是 `main` 分支**，另外两个分支各自归档为子目录：

| 目录 | 源分支 | 原始 commit | 提交时间 | 版本 | 源码文件数 |
| --- | --- | --- | --- | --- | ---: |
| `./`（根目录） | `main` | `b84f37f1a6259a625e9095279dd132f5ef2116af` | 2026-11-25 10:39 +08 | 1.0.49 | 851 |
| `DEV/` | `DEV` | `a800adb2e3a51d05ffe80abeae2e1ec52247b951` | 2026-11-25 11:40 +08 | 1.1.0 | 872 |
| `test/` | `test` | `8c7a37055eb68f71170320d4a17cdb9251785186` | 2026-11-25 10:37 +08 | 1.0.49 | 851 |

原始提交说明：`main` = `!3 merge test into main`；`DEV` = `Merge branch 'test' into DEV`；`test` = `1`。

上一版对应的 commit 为 `main` `25664ec`、`DEV` `3cf0b04`、`test` `fbe9fca`，可直接在 git 历史中对比：

| 分支 | 相对上一版 |
| --- | --- |
| `main` | 35 个文件变更，+5526 / −1786（2 新增、2 删除、31 修改） |
| `DEV` | 94 个文件变更，+21455 / −2278（文件数 851 → 872：新增 `packs/arcade-neogeo/`、`licenses/fbneo-license.txt` 及若干集成测试） |
| `test` | 35 个文件变更，+5526 / −1786 |

三个分支之间**没有做合并、挑选或补丁**：各自按其 commit 的树原样归档。`test` 与 `main` 的变更集规模相同
（`main` 的 tip 就是一次 `merge test into main`），但仍是两个独立 commit，未被压成同一个；也因此本轮
`main` 与 `test` 两份快照的源码树内容完全一致（两份 `SHA256SUMS.txt` 相同），`DEV/` 是唯一不同的分支（v1.1.0）。

与上一版本分支快照相比，本分支共 **200 个路径**发生变化：135 个文件内容修改、27 个新增、6 个删除，
另有 **32 个仅可执行位修正**（上一版快照是用 Python 拷贝工作区再 `git add` 生成的，丢失/误加了 exec 位；
本次直接从 `ls-tree` 取 mode，`build/*.sh` 回到上游的 `100644`，`DEV/cores/**` 等回到 `100755`）。

## 纳入范围与检查

- 完整复制各 commit 的 `git ls-files` 列表：根目录 **851** 个、`DEV/` **872** 个、`test/` **851** 个文件，
  逐字节未修改、未重命名、未"顺手清理"。
- 未包含来源仓库的 `.git` 目录，也未包含任何构建产物、本机配置或临时文件。
- 每份快照各带一份 `SHA256SUMS.txt`，覆盖该目录全部源码文件（路径相对各自目录）；清单自身与
  本说明文件不计入。清单里的哈希取自 **git blob 字节**（不是本机工作区字节），与仓库实际存储的内容一致。
- 文件权限位逐条取自上游 `ls-tree`，未做统一化：`build/*.sh` 为 `100644`，`DEV/cores/**/*.sh` 等可执行脚本为 `100755`。
- 只读安全检查（对三个分支分别执行）：`git ls-files` 中无 `.gitmodules`，`.gitattributes` 无 Git LFS filter，
  最大单文件 1.85 MiB（`crates/slot-ui/assets/SlotSimplifiedChinese.otf`），无超过 20 MiB 的文件；
  对全部源码内容扫描私钥块、AWS/GitHub/GitLab/OpenAI 风格令牌，**0 命中**。
- 许可证文本、NOTICE 与第三方 `licenses/` 目录均随源码原样保留。
- 三份源码树按 git blob 原样存储，未做行尾归一化；原本就是 CRLF 的许可证文本保持 CRLF。

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
