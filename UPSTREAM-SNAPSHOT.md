# Upstream snapshot — linkSlot（GitCode 私有仓库同步）

这是一份**只读归档快照**，用于保存指定时间点的 linkSlot 源码树。它不是开发分支，不接受补丁，
也不代表任何维护承诺。

## 项目

| 项 | 值 |
| --- | --- |
| 项目名称 | linkSlot（Game Boy 系掌机前端，Anbernic H700 系列） |
| 项目版本 | `1.0.45`（`Cargo.toml` 的 `[workspace.package].version`） |
| 版本佐证 | `CHANGELOG.md` 首条为 `## v1.0.45（2026-11-24 构建）` |
| 许可证 | GPL-3.0-or-later（`LICENSE`、`NOTICE`、`Cargo.toml` 一致） |
| 版权 | Copyright (C) 2026 luguiyi691（见 `NOTICE`） |
| 上游派生自 | slot v1.6.0（2026-10-10 发布），https://github.com/slot-cfw/slot |

## 本次来源（与上一版快照不同）

| 项 | 值 |
| --- | --- |
| 来源平台 | GitCode / AtomGit（私有仓库） |
| 仓库 | `luguiyi691/linkslot`，分支 `main` |
| 原始 commit | `25664ecd2a712279dfa840fbc1ea896ebc0ef0ba` |
| 提交时间 | 2026-11-24T16:18:02+08:00 |
| 提交说明 | `!2 merge test into main` |
| 同步时间 | 2026-11-24（UTC+8） |
| 读取方式 | 使用账号访问令牌经 HTTPS 克隆；令牌未写入本仓库、脚本或清单 |

上一版快照（父提交）来自一份不含 `.git` 的 v1.0.36 zip 归档，公开上游 `luguiyi/linkslot`
只到 v1.0.0，因此当时无法给出 commit id。本次来源是带完整 git 元数据的私有仓库，版本为
v1.0.45，源码树已整体替换为该 commit 的内容；旧快照仍保留在父提交中，可随时对比。

## 纳入范围与检查

- 完整复制该 commit 的 `git ls-files` 列表：**851 个文件**，逐字节未修改、未重命名、未"顺手清理"。
- 未包含来源仓库的 `.git` 目录，也未包含任何构建产物、本机配置或临时文件。
- `SHA256SUMS.txt` 覆盖全部 851 个源码文件；本说明文件与清单自身不计入。
- 只读安全检查：`git ls-files` 中无 `.gitmodules`，`.gitattributes` 无 Git LFS filter，
  最大单文件约 1.85 MiB（`crates/slot-ui/assets/SlotSimplifiedChinese.otf`），
  无超过 90 MiB 的文件；文件名与源码内容扫描未发现私钥、访问令牌或云凭据模式。
- 许可证文本、NOTICE 与第三方 licenses/ 目录均随源码原样保留。

## 复现与校验

```sh
sha256sum -c SHA256SUMS.txt
```

本快照仅作为归档与来源对照；不包含可刷写固件、设备侧测试结果或硬件验证结论。
