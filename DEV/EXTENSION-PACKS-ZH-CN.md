# LSP 扩展包

扩展包用于给已发布的 linkSlot 增加**可选核心**、**成就定义/徽章**，以及
**街机 / NeoGeo 平台**（平台包），不需要重新刷整包，也不需要手动拷卡。支持
`kind = "core"`、`kind = "achievements"` 和 `kind = "platform"`。

| 扩展包 | 机种 | 说明 |
|---|---|---|
| `linkslot-pack-supafaust-v1.0.0.lsp` | SFC | 第二个 SFC 核心，兼容性选项 |
| `linkslot-pack-nestopia-v1.0.0.lsp` | FC | 高兼容 FC 核心，适合汉化/特殊 mapper 游戏 |
| `linkslot-pack-armgba-v1.0.0.lsp` | GBA | 第三个 GBA 核心（AArch64 重编译器）；netpacket 联机（Link Mode Separate，电缆/无线） |
| `linkslot-pack-contra-1447-v1.0.0.lsp` | FC | 《魂斗罗》RA 中文化成就 + 徽章 |
| `linkslot-pack-arcade-neogeo-v1.0.0.lsp` | Arcade + NeoGeo | 平台包：一次激活两台机种，带一个 FBNeo |

> `armGBA` 的 libretro 核心通过 netpacket 提供联机（`Link Mode = Separate`，
> 电缆或无线适配器）；linkSlot 的 2 人会话走这条接口（Together 不接，两端需同一构建）。

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
| `schema` | 固定 `1`；平台包固定 `2` |
| `kind` | `core`、`achievements` 或 `platform` |
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
- 可重复 `--payload` 添加多个文件（核心包目前只支持一个 `.so`；平台包必须
  带核心 `.so`，DAT/资产可多份）；
- 平台包（schema 2）示例，DAT 由 Phase 3 的生成器产在 `dist/platforms/`：

```bash
python3 build/pack_lsp.py \
  --meta packs/arcade-neogeo/pack.json \
  --payload System/linkSlot/cores/fbneo/fbneo_libretro.so=target/cores/fbneo_libretro.so \
  --payload System/linkSlot/extensions/platforms/arcade-neogeo/metadata/arcade.dat.json=dist/platforms/arcade.dat.json \
  --payload System/linkSlot/extensions/platforms/arcade-neogeo/metadata/neogeo.dat.json=dist/platforms/neogeo.dat.json \
  --out dist/packs/linkslot-pack-arcade-neogeo-v1.0.0.lsp
```
- 固定 `SOURCE_DATE_EPOCH` 时，同样的输入产生同样的 `.lsp`；
- 发布时把 `.lsp` 作为独立附件，不放进卡包；
- `armGBA` 是 GPL-2.0：发布它的 `.lsp` 时，同时附上
  `cores/armgba/build.sh` 生成的 `armGBA-<commit>.tar.gz` 与 `licenses/armgba-*`
  （对应源码与许可证）。

## 平台包（`kind = "platform"`）

平台包是**唯一能让 Arcade / NeoGeo 出现**的东西：没有合法的平台包记录，
`Games/Arcade` 与 `Games/NeoGeo` 完全不扫描（连目录都不会被读取），书架上也
没有街机入口。手动把 `fbneo_libretro.so` 拷进卡里不算安装。

一个包同时激活两台机种（`arcade` + `neogeo`），共用一个 FBNeo；卸载一次两台
一起消失，ROM / BIOS / 存档 / 记录都保留。

### 格式：schema 2 + 多文件

```text
linkslot-pack-arcade-neogeo-v1.0.0.lsp
├─ pack.json
├─ System/linkSlot/cores/fbneo/fbneo_libretro.so
└─ System/linkSlot/extensions/platforms/arcade-neogeo/
   ├─ metadata/arcade.dat.json
   ├─ metadata/neogeo.dat.json
   └─ assets/...                      # 可选，静态图片
```

`pack.json`（模板见 `packs/arcade-neogeo/pack.json`）：

| 字段 | 说明 |
|---|---|
| `schema` | 固定 `2`（`core` / `achievements` 仍是 `1`） |
| `kind` | 固定 `platform` |
| `extension_api` | 固定 `1`；app 不认识就拒装并提示升级 |
| `min_app` | **必填**，格式 `x.y.z` |
| `platforms[]` | 至少一项；每项 `id`（`arcade` / `neogeo`）、`folder`、`rom_extensions`、`scanner`、`ui`、`ui_profile`、`input`、`core_ids`、`default_core`、`bios` |
| `cores[]` | v1.1.0 **只允许一条**；`id` 必须是已知核心，`dylib` 必须是 `<id>_libretro.so`，`license` 必填 |
| `files[]` | 与其它 kind 相同：逐个校验 `size` / `sha256` |

模板 id 只能点名 app 已实现的那一套（不接受包自带代码）：

- `scanner` 只能是 `arcade_zip_v1`；
- `ui` 只能是 `arcade_cabinet_v1`，`ui_profile` 必须等于平台 id；
- `input` 只能是 `arcade_6button_v1`；
- `bios` 只允许 `neogeo.zip`，且只有 `neogeo` 可以非空（BIOS 由用户放在
  `BIOS/fbneo/neogeo.zip`，包不携带）。

### 路径白名单（只有这三类）

| 卡内路径 | 说明 |
|---|---|
| `System/linkSlot/cores/<core>/<core>_libretro.so` | 必须是声明的那个核心 |
| `System/linkSlot/extensions/platforms/<包 id>/metadata/<平台>.dat.json` | 必须是声明的平台 |
| `System/linkSlot/extensions/platforms/<包 id>/assets/<1–4 层>` | 只接受 `png/jpg/jpeg/webp`，单张 ≤ 4 MiB |

`bin/`、`Config/`、别的包 id、别的核心目录一律拒绝。

### DAT（set → 标题 / 机板）

`metadata/<平台>.dat.json` 由 `build/fbneo_dat.py` 从 FBNeo 的 ClrMamePro XML DAT
转换而来（Arcade 一份 8039 条、NeoGeo 一份 708 条）：

```json
{
  "schema": 1,
  "platform": "arcade",
  "generated_from": {
    "dat": "FinalBurn Neo (ClrMame Pro XML, Arcade only).dat",
    "version": "1.0.0.03"
  },
  "sets": [
    {
      "set": "sf2",
      "title": "Street Fighter II: The World Warrior (World 910522)",
      "system": "CPS-1",
      "year": "1991",
      "parent": "",
      "is_bios": false
    }
  ]
}
```

| 字段 | 说明 |
|---|---|
| `schema` | 固定 `1` |
| `platform` | `arcade` / `neogeo`；与文件所在平台一致 |
| `generated_from.dat` / `.version` | 来源文件名与 DAT 版本，仅用于溯源 |
| `sets[].set` | **必填**，等于 `Games/<平台>/<set>.zip` 的文件名（不含扩展名） |
| `sets[].title` | 展示名；空则回退到 `set` |
| `sets[].system` | 机板名（`CPS-1` / `CPS-2` / `NEOGEO` / `Konami` …）；来自 DAT 的 `sourcefile` |
| `sets[].year` | 年份原样字符串（`1991` / `199?`），可为空 |
| `sets[].players` | 人数，缺省 1 |
| `sets[].orientation` | `horizontal`（缺省）/ `vertical` |
| `sets[].parent` | 该 set 的母版；空表示它自己就是母版（来自 DAT 的 `cloneof`） |
| `sets[].is_bios` | BIOS 集合：不进书架，但运行时可能被别的 set 依赖 |

**扫描怎么用它**：`is_bios` 跳过；`parent` 的 zip 也在卡上时跳过 clone（收进
"版本"子菜单，由母版代表）；`parent` 不在卡上时**保留**该 clone 并记一行警告
（只装了一个改版的用户仍然能玩到）。

生成方式：

```bash
python3 build/fbneo_dat.py \
  --dat "vendor/fbneo/dats/FinalBurn Neo (ClrMame Pro XML, Arcade only).dat" \
  --platform arcade --out dist/platforms/arcade.dat.json
python3 build/fbneo_dat.py \
  --dat "vendor/fbneo/dats/FinalBurn Neo (ClrMame Pro XML, Arcade only).dat" \
  --platform neogeo --out dist/platforms/neogeo.dat.json
```

同一份 DAT 出两个平台：`arcade` 排除 `sourcefile` 以 `neogeo/` 开头的条目，
`neogeo` 只保留它们。

**缺 DAT / 坏 DAT 的兜底**：DAT 只做增强，不做门。

- 没有 DAT、JSON 解析失败、字段类型不对、`sets` 超过 200 000 条 → 当成"没有
  DAT"，记一行日志，不出错、不清库；
- DAT 缺失或损坏不会阻止扫描：标题回退为文件名，BIOS/clone 过滤降级（BIOS
  集合可能作为普通游戏出现）。包作者应始终随包提供两份 DAT；
- 某个 set 不在 DAT 里 → 照常出现在书架，标题用文件名；
- 因此 DAT 可以后补：装好平台包先玩，DAT 到了再刷新一次库就有标题和机板。

### 限制与规则

- 单文件 ≤ 96 MiB、解压总量 ≤ 128 MiB（FBNeo 的 aarch64 `.so` 是 79.6 MiB）；
  其中 96 MiB 只用于扩展包；128 MiB 与升级包的解压总量上限是同一个数值，
  但两者的校验入口独立，Web 上传设置里的默认 32 MB 不受影响；
- **一个平台只能属于一个包**：第二个声明 `arcade` 的包会被拒（否则卸载任一个
  都会删掉另一个的核心）；要换包请先卸载旧的；
- 安装/卸载后立刻重扫游戏库，不需要重启；
- 卸载只删记录里的文件与因此变空的目录，绝不碰 `Games/`、`BIOS/`、`Saves/`、
  `States/`、`Cheats/`、`selected_core.txt`。

## 后续扩展

新增 `kind` 时同步修改三处：

1. `crates/slot/src/packs.rs` 的 `KINDS` 与路径白名单；
2. 安装目标目录（例如主题包写 `System/linkSlot/assets/`）；
3. Web 关于页的提示文案（`translations/web.*.json` 的 `web.about.packs*`）。
