# LSP 扩展包

扩展包用于给已发布的 linkSlot 增加**可选核心**、**成就定义/徽章**等资源，不需要重新刷整包，也不需要手动拷卡。支持 `kind = "core"` 和 `kind = "achievements"`。

| 扩展包 | 机种 | 说明 |
|---|---|---|
| `linkslot-pack-supafaust-v1.0.0.lsp` | SFC | 第二个 SFC 核心，兼容性选项 |
| `linkslot-pack-nestopia-v1.0.0.lsp` | FC | 高兼容 FC 核心，适合汉化/特殊 mapper 游戏 |
| `linkslot-pack-armgba-v1.0.0.lsp` | GBA | 第三个 GBA 核心（AArch64 重编译器），单人；不支持 linkSlot 联机 |
| `linkslot-pack-contra-1447-v1.0.0.lsp` | FC | 《魂斗罗》RA 中文化成就 + 徽章 |

> `armGBA` 的联机是 RetroArch netplay 自己的一套，linkSlot 的电缆/netpacket
> 会话不接；要联机请继续用 mGBA / gpSP。

## 使用

1. 下载 `linkslot-pack-<id>-v<version>.lsp`；
2. 设备 Web → **扩展** → **扩展包** → 选择文件 → **上传并安装**；
3. 安装成功后：
   - 核心包：重开对应卡带的插槽选择器，多出一颗芯片；
   - 成就包：刷新 Web「成就」页，即可看到新增定义和徽章。

不需要重启，也不需要解压。卸载在同一张卡片里点「卸载」。

卸载核心包时：

- 卡带的 `selected_core` 记录会保留；核心文件不在时，启动暂时回退到机种默认
  核心并提示一次；重新安装后恢复原来的选择。
- `Saves/`、`States/`、`Cheats/` 下该核心的存档/状态/金手指不会被删除。
- 如果核心选择器正开着，会关闭并提示“扩展包已变更”；重新打开时按实际可用
  核心数显示 2 核或 3 核布局，不会留下已卸载的芯片。

> 只安装可信来源的扩展包：核心是原生代码，会以设备权限运行。设备在游戏中或
> 同步中时，安装/卸载按钮会被拒绝。

## 文件格式

扩展包是一个 ZIP 容器，扩展名 `.lsp`。根目录必须有 `pack.json`，其余条目就是
卡片上的目标路径：

```text
pack.json
System/linkSlot/cores/supafaust/supafaust_libretro.so
```

`pack.json`：

```json
{
  "schema": 1,
  "kind": "core",
  "id": "supafaust",
  "name": "SFC 核心 · supafaust",
  "version": "1.0.0",
  "min_app": "1.0.0",
  "license": "unknown",
  "source": "https://github.com/libretro/supafaust",
  "files": [
    {
      "path": "System/linkSlot/cores/supafaust/supafaust_libretro.so",
      "size": 1138552,
      "sha256": "cba32007baeab16f424ad12edaadb212f36336df78c6f96a4b12b4c974f08bee"
    }
  ]
}
```

字段：

| 字段 | 说明 |
|---|---|
| `schema` | 固定 `1` |
| `kind` | `core` 或 `achievements` |
| `id` | 核心 id，必须是 linkSlot 已知核心（`mgba`/`gpsp`/`fceumm`/`snes9x`/`supafaust`/`genesis_plus_gx`/`nestopia`/`armgba`） |
| `version` | 扩展包版本（字符串，≤ 32 字符） |
| `min_app` | 可选；要求的最低 linkSlot 版本，格式 `x.y.z` |
| `license` / `source` | 可选，便于合规与溯源 |
| `files` | 文件清单，`path` 是卡内相对路径，安装时逐个校验 `size` 与 `sha256` |

安装限制（v1）：

- `core`：只允许写 `System/linkSlot/cores/<id>/<id>_libretro.so`；
- `achievements`：只允许写
  `linkSlot/Achievements/defs/<平台>/<名称>.json` 和
  `linkSlot/Achievements/badges/<平台>/<sha256>/<成就ID>.png`；
- `bin/`、`shell/`、`linkSlot/Config/`、游戏/存档等目录一律拒绝；
- 不允许 `..`、绝对路径、反斜杠、符号链接；
- 压缩包内除 `pack.json` 外不能有多余文件，清单里的文件也不能缺失；
- 安装记录在 `linkSlot/Config/packs/installed.json`，卸载只删除记录中的文件；
- 同一 `id` 重装会覆盖旧文件并更新记录；
- 成就包不包含解锁记录，不会覆盖已有进度。

## 打包

核心包用本仓库的 `build/pack_lsp.py`；**成就包**用独立工具仓库
`linkslot_tool`（命令 `linkslot-ra`）生成，不在本仓库：

```bash
linkslot-ra \
  --patch-json tmp/contra-patch.json \
  --rom "Games/FC/Contra (USA).nes" \
  --badges-dir tmp/badges \
  --out dist/packs/linkslot-pack-contra-1447-v1.0.0.lsp
```

核心包：

```bash
python3 build/pack_lsp.py \
  --meta packs/supafaust/pack.json \
  --payload System/linkSlot/cores/supafaust/supafaust_libretro.so=vendor/supafaust_libretro.so \
  --out dist/packs/linkslot-pack-supafaust-v1.0.0.lsp

# armGBA：用 cores/armgba/build.sh 在 bullseye arm64 容器里编出 .so 后再打包
python3 build/pack_lsp.py \
  --meta packs/armgba/pack.json \
  --payload System/linkSlot/cores/armgba/armgba_libretro.so=target/cores/armgba_libretro.so \
  --out dist/packs/linkslot-pack-armgba-v1.0.0.lsp
```

- `pack.json` 模板里的 `files` 留空，脚本按实际文件填 `size`/`sha256`；
- 可重复 `--payload` 添加多个文件（核心包目前只支持一个 `.so`）；
- 固定 `SOURCE_DATE_EPOCH` 时，同样的输入产生同样的 `.lsp`；
- 发布时把 `.lsp` 作为独立附件，不放进卡包；
- `armGBA` 是 GPL-2.0：发布它的 `.lsp` 时，同时附上
  `cores/armgba/build.sh` 生成的 `armGBA-<commit>.tar.gz` 与 `licenses/armgba-*`
  （对应源码与许可证）。

## 后续扩展

新增 `kind` 时同步修改三处：

1. `crates/slot/src/packs.rs` 的 `KINDS` 与路径白名单；
2. 安装目标目录（例如主题包写 `System/linkSlot/assets/`）；
3. Web 关于页的提示文案（`translations/web.*.json` 的 `web.about.packs*`）。
