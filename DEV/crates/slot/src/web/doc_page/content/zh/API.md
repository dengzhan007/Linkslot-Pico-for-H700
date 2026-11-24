# linkSlot Mod API 参考

适用版本：linkSlot **1.x** / Mod API **1.1**（schema 2 mod）。

- [1. 目录结构](#1-目录结构)
- [2. `mod.json`](#2-modjson)
- [3. `action.json`](#3-actionjson)
- [4. 执行时机](#4-执行时机)
- [5. Lua 运行环境](#5-lua-运行环境)
- [6. Lua API](#6-lua-api)
- [7. 交互与结果协议](#7-交互与结果协议)
- [8. 视觉素材](#8-视觉素材)
- [9. 主题包](#9-主题包)
- [10. 限制与安全](#10-限制与安全)
- [11. 兼容与升级](#11-兼容与升级)

---

## 1. 目录结构

```
linkSlot/mods/<mod-id>/
├── mod.json                     必需，mod 清单（schema 2）
├── README.md                    可选，Web「mod 详情」里的简介
├── web/index.html               可选但强烈建议，mod 自己的管理页面
├── config.json                  可选，mod 自己的配置（Lua 中通过 ctx.mod.config 读取）
├── sticker.png                  可选（预留，当前版本不绘制）
├── sync-chip.png                可选，联网芯片封面
├── theme.txt                    可选，主题调色（主题包）
├── wallpaper.png                可选，壁纸（主题包）
├── font.otf | font.ttf | font.ttc  可选，界面字体（主题包）
├── slot_back_gba.png 等         可选，卡槽铭牌（按机器）
└── actions/<action-id>/
    ├── action.json              必需，动作清单（schema 1）
    ├── main.lua                 必需，动作脚本（UTF-8，≤ 256 KiB）
    └── sticker.png              可选（预留，当前版本不绘制）
```

命名规则：

- `<mod-id>`、`<action-id>`：只能用小写字母、数字、`_`、`-`，且以字母或数字开头。
- 只识别目录；`linkSlot/mods/` 下散落的 `.lua` 文件不会执行。

---

## 2. `mod.json`

```json
{
  "schema": 2,
  "id": "gbastation-battery-sync",
  "name": "GBAStation 电池存档同步",
  "version": "1.3.9",
  "author": "linkSlot",
  "description": "长按 Y 通过局域网 GBAStation 同步电池存档",
  "enabled": true,
  "order": 100,
  "category": "sync"
}
```

| 字段 | 必填 | 说明 |
|---|---|---|
| `schema` | 是 | 固定 `2` |
| `id` | 是 | 必须等于目录名 |
| `name` | 是 | 显示名，≤ 64 字符 |
| `version` | 否 | 默认 `1.0.0`；只允许字母、数字和 `. _ + -`，≤ 32 字符 |
| `author` | 否 | 作者 |
| `description` | 否 | 一句话简介 |
| `enabled` | 否 | 总开关，默认 `false`；关闭后其下所有动作都不执行 |
| `order` | 否 | 优先级，默认 `100`，范围 `-9999..=9999`，**数字越小越优先** |
| `category` | 否 | 分组提示，只能是 `sync` / `info` / `tool` / `theme`。留空时按行为推断：有 `manual` 动作 → `sync`；只带图片（`sync-chip.png` 或 `slot_back_*.png`）→ `theme`；其余 → `other` |

`order` 影响：长按 Y 时网络芯片卡的排列顺序、Web 端「mod 设置」的排序。它**不再**决定「谁抢到长按」——每个在 `action.json` 里声明了 `ui.animation = cloud_sync` 的 manual 动作各占一张卡，由用户选择。

---

## 3. `action.json`

```json
{
  "schema": 1,
  "name": "同步电池存档",
  "timing": "manual",
  "enabled": true,
  "order": 100,
  "boot_delay_minutes": 0,
  "ui": {
    "animation": "cloud_sync",
    "title": "同步 GBAStation",
    "units": ["sav"],
    "show_cloud": true,
    "direction": "upload"
  }
}
```

| 字段 | 必填 | 说明 |
|---|---|---|
| `schema` | 是 | 固定 `1` |
| `name` | 是 | 动作名，≤ 64 字符 |
| `timing` | 是 | 见[执行时机](#4-执行时机) |
| `enabled` | 否 | 默认 `false`；mod 总开关与其共同决定是否执行 |
| `order` | 否 | 默认 `100`，同机时同优先级的排序 |
| `boot_delay_minutes` | 否 | 仅 `after_boot` 用，`0..=1440` 分钟 |
| `ui` | 否 | 长按 Y 时的界面声明，仅 `manual` 有意义 |

### `ui` 子字段

| 字段 | 说明 |
|---|---|
| `animation` | 目前只支持 `"cloud_sync"`，表示沿用云同步的卡片↔云端动画 |
| `title` | 动画里的标题，≤ 24 字符；留空则用动作名 |
| `units` | `"state"` / `"sav"`，最多两个；留空则不画存档节点 |
| `show_cloud` | 是否画云端那一端，默认 `true` |
| `direction` | `"upload"` / `"download"`，留空则不画方向箭头 |

> **重要**：只有声明了 `ui.animation = "cloud_sync"` 的 manual 动作，才会出现在长按 Y 的网络芯片卡列表里。
> 没有 `ui` 的 manual 动作依然可以被 `linkSlot mods run` 调用，但用户在设备上看不到它的卡片。
>
> `ui` 是**声明式**的：mod 只能借用现有的动画语法，不能声明一个不真实的方向或节点。真实进度请用 `mods.progress`。

---

## 4. 执行时机

| `timing` | 何时运行 | 有无游戏上下文 |
|---|---|---|
| `before_upload` | 云同步上传前 | 有 |
| `after_upload` | 云同步上传后 | 有 |
| `before_download` | 云同步下载前 | 有 |
| `after_download` | 云同步下载后 | 有 |
| `after_sync` | 整次 `sync` 结束后（即使全 noop） | 有 |
| `after_boot` | 开机后 `boot_delay_minutes` 分钟 | **无**（`ctx.unit == "boot"`） |
| `manual` | 用户在长按 Y 选卡后执行 `mods run` | 有 |

同一时机有多个动作时，按 `mod.order` → `action.order` → 名称排序。

`manual` 动作**不会**在云同步过程中执行；反之，其它时机的动作也不会出现在长按 Y 菜单里。

---

## 5. Lua 运行环境

- Lua **5.4**，单文件 `main.lua`，UTF-8，≤ 256 KiB。
- 已移除：`dofile`、`loadfile`、`load`、`require`、`package`、`io`、`os`、`debug`。
- 保留：`string`、`table`、`math`、`utf8`、`coroutine`、`pcall` 等标准库。
- `print(...)` 等价于 `log.info`，会写进设备日志。

脚本抛错即视为本次动作失败，错误文本会显示在结果条上。

---

## 6. Lua API

### 6.1 `ctx`：本次执行上下文

```lua
ctx.api          -- 固定 1
ctx.timing       -- "manual" / "after_boot" / "after_sync" / ...
ctx.device_id    -- 设备标识
ctx.app_version  -- linkSlot 版本，例如 "1.2.0"
ctx.choices      -- 表：{ [key] = option }（由 --choice 预答或设备回填）

ctx.unit         -- "boot" / "sync" / "state" / "sav"
ctx.unit_path    -- 当前单元相对存储卡根目录的路径
ctx.game.name        -- 游戏名
ctx.game.platform    -- "GBA" / "GB" / "GBC" / "FC"
ctx.game.core        -- "mgba" / "gpsp" / "fceumm" ...
ctx.game.rom_sha256  -- ROM 摘要（可能为 nil）
ctx.game.state_dir   -- States/... 目录（相对路径）
ctx.game.sav_path    -- 电池存档路径
ctx.game.srm_path    -- .srm 路径

ctx.mod.id, ctx.mod.dir            -- "mymod", "linkSlot/mods/mymod"
ctx.mod.config                     -- config.json 解析后的 Lua 表（无则 nil）
ctx.mod.config_error               -- 读取失败原因（可选）

ctx.action.id, ctx.action.name, ctx.action.dir

ctx.webdav.active                  -- 当前同步方式 id（可能为 nil）
ctx.webdav.profiles                -- 数组：{ {id=, name=, remote_root=}, ... }
```

`after_boot` 动作没有 `ctx.game`，`ctx.unit == "boot"`。

### 6.2 `files`：沙箱文件

```lua
files.read(rel)          -- string | nil
files.write(rel, data)   -- bool（data 为字符串）
files.list(dir)          -- 数组：{ {name=, size=, is_dir=, mtime=}, ... }
files.stat(rel)          -- 同上单个 | nil
files.exists(rel)        -- bool
files.mkdir(rel)         -- bool
```

- 允许的路径只有三处：`States/`、`Saves/`、`linkSlot/mods/<自己的 id>/`。
- 路径按段校验，内部再 canonicalize 一次，防止软链穿越。
- 相对存储卡根目录，例如 `files.read("Saves/GBA/Emerald.sav")`。
- 单次读 ≤ 64 MiB、单次写 ≤ 32 MiB；`files.list` 最多返回 1000 条；路径 ≤ 1024 字节、≤ 32 段。

### 6.3 `json` / `hash`

```lua
json.encode(value)       -- string
json.decode(text)        -- 任意 Lua 值

hash.sha256(text)        -- 十六进制字符串
hash.sha256_file(rel)    -- 十六进制字符串（走沙箱路径校验）
```

### 6.3a `crypto`：字符串加解密

```lua
crypto.encrypt(text[, key])  -- 返回版本化密文
crypto.decrypt(token[, key]) -- 返回明文
```

- 不传 `key` 时优先使用用户在 Web 设置里的自定义密钥；没有自定义密钥时使用设备默认密钥；
- 用户密钥由设备密钥包裹后保存在 `linkSlot/Config/crypto.user`，不是明文；
- 传入 `key` 时使用本次调用方密钥，不会把该参数写入磁盘；
- 使用 AES-256-GCM，随机 nonce，密文带 `v1:` 前缀；
- 解密失败统一返回“密文格式无效或密钥错误”；
- 仅用于避免明文落盘，不应把默认密钥当作绝对安全边界。

### 6.4 `zip`：内存中的 ZIP

```lua
local entries = zip.list(bytes)         -- { {index=, name=, size=, is_dir=}, ... }
local data = zip.read(bytes, "a.txt")   -- string（第二个参数也可以传 zip.list 里的 index）
local packed = zip.build({              -- 返回新的 zip 字节串
  { name = "a.txt", data = "hello" },
})
```

### 6.5 `log` / `print`

```lua
log.info("...")   log.warn("...")   log.error("...")   log.debug("...")
print("...")      -- 等价于 log.info
```

日志会带上 `[mods:<mod-id>/<action-id>]` 前缀写入 `Logs/linkSlot.log`。

### 6.6 `http`（仅 manual 动作）

需要 mod 自己的 `config.json` 里有 `base_url`：

```json
{ "base_url": "http://192.168.1.20:8080" }
```

```lua
-- 路径是相对 base_url 的片段，可以用字符串或片段数组
local res = http.get("/status")                 -- { status = 200, body = "..." }
local res = http.get({ "slot", "1" }, { raw = "1" })   -- 片段 + 查询参数

local res = http.post("/save", body, "application/octet-stream")
local res = http.post("/save", body, nil, { mode = "fast" })

local res = http.request("PUT", "/slot/1", {
  body = "...",
  content_type = "application/octet-stream",
  headers = { ["X-Token"] = "..." },
})
```

- 只允许访问 `base_url` 指向的主机；请求路径作为片段拼在它后面，不能越到别的主机。
- `http.get` / `http.post` 的第三个、第四个参数是 `content_type` 与 `query` 表。
- `http.request` 的签名是 `(method, segments, opts)`；`opts` 支持 `body` / `content_type` / `query` / `headers`，不要写成 `{ method = ..., path = ... }`。
- manual 的 HTTP 超时 60 秒；非 manual 用 `Config` 里的同步超时（1–10 秒）。

### 6.7 `webdav`

```lua
webdav.profiles()               -- 数组：{ {id=, name=, remote_root=}, ... }
webdav.get(path)                -- 文件内容 | nil
webdav.put(path, data)          -- bool
webdav.put_atomic(path, data)   -- 先写临时名再改名
webdav.list(dir)                -- 数组，同 files.list
webdav.stat(path)               -- 单个 | nil
webdav.mkdir(dir)               -- bool
-- 以上每个方法都可写成 (profile, ...) 指定同步方式，省略 profile 时用当前激活的那个
-- 例如 webdav.get(profile.id, "Saves/GBA/Emerald.sav")、webdav.put_atomic(profile.id, path, data)
```

### 6.8 `ui`（仅 manual 动作）

```lua
local ok = ui.confirm({
  title = "覆盖云存档？",          -- 省略则用「确认」
  body = "本地比云端新，继续将覆盖云端。",
  yes = "覆盖",                    -- 省略则用「确定」
  no = "取消",                     -- 省略则用「取消」
})

ui.message({
  title = "完成",
  body = "已同步 3 个文件。",
  button = "知道了",               -- 省略则用「知道了」
})
```

`ui.confirm` **没有自定义 key**：引擎按调用顺序生成 `ui_confirm_1`、`ui_confirm_2`…，`ui.message` 生成 `ui_message_1`…。

- 第一次运行时脚本会**中断**，设备弹出确认框；
- 用户回答后，设备带着 `--choice ui_confirm_1=yes` 重新运行；
- 再次跑到同一位置时，`ui.confirm` 直接返回 `bool`，不再中断。

所以要保证可重入：**同一个脚本里第 N 次 `ui.confirm` 每次都对应同一个问题**，不要把确认放进按条件变化的分支里前后调换顺序。

需要**稳定、自定义**的 key（例如跨版本还想记住答案）时，请用 `mods.choose`。

### 6.9 `mods`：结果、进度、选择

```lua
mods.report({
  status = "ok",            -- ok | noop | error
  direction = "upload",     -- upload | download | nil
  error = "net",            -- 出错分类，见下表；status=error 时可省略（默认 server）
  detail = "已同步电池存档", -- 一行中文，显示在结果条
})

mods.progress({ value = 0.42, label = "正在生成攻略" })   -- value 0..1，节流 100ms

mods.choose({
  key = "conflict",
  title = "存档冲突",
  body = "本地与云端都有改动，选择保留哪一边。",
  options = {
    { id = "remote", label = "用云端" },
    { id = "local",  label = "用本地" },
  },
  default = "remote",
})
```

`error` 可用分类：`net`、`auth`、`cert`、`clock`、`server`、`config`、`fs`、`verify`、`timeout`、`busy`、`no-cache`、`remote-newer`、`forked`、`remote-rollback`、`changed-since`。

`mods.report` 与 `mods.choose` 只允许 manual 动作；非 manual 调用会直接报错。

### 6.10 `library`（仅 manual，且有游戏上下文）

```lua
library.exists("intro")          -- bool
library.write("intro", text)     -- 写入 Info/<平台>/<游戏>.txt
library.write("guide", text)     -- 写入 Guides/<平台>/<游戏>.txt
```

`kind` 只支持 `intro` / `guide`；单次文本上限 1 MiB。写入路径按当前游戏自动推导，脚本不能指定别的文件。

---

## 7. 交互与结果协议

### 7.1 进度

`mods.progress` 会向标准输出写：

```
LINKSLOT_PROGRESS=42
LINKSLOT_PROGRESS_LABEL=正在生成攻略
```

设备侧显示进度条与左侧文案；100ms 内重复调用会被合并。

### 7.2 选择

`mods.choose` / `ui.confirm` / `ui.message` 触发时，本轮脚本以 `LINKSLOT_RESULT=choice` 结束，并附一行 `LINKSLOT_CHOICE={...}`。设备弹出对话框，用户回答后带着 `--choice key=option` 重新启动同一个动作。

- `mods.choose` 用脚本自己给的 `key`；
- `ui.confirm` / `ui.message` 用引擎生成的 `ui_confirm_<n>` / `ui_message_<n>`（按调用顺序）。

因此脚本必须写成**可重入**的：再次运行时，前面已答过的选择会直接返回结果，后续步骤继续。

### 7.3 最终结果

动作结束后，`linkSlot` 输出固定格式的结论（设备据此显示结果条）：

```
LINKSLOT_ACTION=mods
LINKSLOT_RESULT=ok|noop|error|choice
LINKSLOT_DETAIL=已同步电池存档
LINKSLOT_ERROR=none|net|auth|...
```

`mods.report` 不调用时，脚本正常结束视为 `ok`；抛错视为 `error`。

---

## 8. 视觉素材

### 8.1 卡片贴纸 `sticker.png`（预留）

- 文件名仍被识别：动作级 `actions/<action-id>/sticker.png` 与 mod 级 `sticker.png`；
- 长按 Y 使用网络芯片卡，贴纸不再绘制；保留该字段只为兼容旧包；
- 想让长按 Y 有画面，请用下面的 `sync-chip.png`。

### 8.2 联网芯片封面 `sync-chip.png`

- 覆盖放大后的网络芯片正面；
- 画布建议 **3:2（如 540×360）**，与贴纸区域同比例，铺满时不裁切；
- 芯片自身的边框、两颗状态灯由系统绘制，不随封面变化。

### 8.3 卡槽铭牌 `slot_back_<machine>.png`

`<machine>` 可选：`recents`、`favorites`、`gba`、`gb`、`gbc`、`fc`。

优先级：**存储卡上的 `System/linkSlot/assets/carts/slot_back_*.png`** → **启用且 `order` 最小的 mod** → 内置雕刻铭牌。

### 8.4 mod 自己的管理页 `web/index.html`

在设备 Web 的「mods」页点开 mod 时以 iframe 加载，可用桥接 SDK：

```js
const info = await LinkSlotMod.ready;      // { id, name, version }

const files = await LinkSlotMod.request('files.list');                  // mod 目录文件
const text  = await LinkSlotMod.request('files.read', { path: 'config.json' });
await LinkSlotMod.request('files.write', { path: 'config.json', content: '{}' });

const { actions } = await LinkSlotMod.request('actions.list');
await LinkSlotMod.request('actions.save', {
  action_id: 'sync', name: '同步', timing: 'manual', enabled: true, order: 100, content: '...'
});
await LinkSlotMod.request('actions.toggle', { action: 'sync', enabled: false });
await LinkSlotMod.request('actions.delete', { action: 'sync' });
await LinkSlotMod.request('actions.check', { action: 'sync' });

const saves = await LinkSlotMod.request('context.saves');
const { storages, timings } = await LinkSlotMod.request('context.storages');
```

页面运行在 `sandbox="allow-scripts"` 的 iframe 里，**不能**直接调用 `/api/**`；只能使用上表的方法。没有 `web/index.html` 的 mod 在 Web 里打不开管理页。

---

### 8.5 Web Bridge 能力与 `api` 方法

`mod.json` 可以声明 Web 管理页需要的能力：

```json
"permissions": ["context:read", "game:write", "network:read"]
```

动作也可以在 `action.json` 的 `capabilities` 里声明；Web 端展示的是两者的并集：

| 能力 | 允许的接口 |
|---|---|
| `context:read` | `/api/status`、`/api/roms`、`/api/saves`、`/api/achievements*`、`/api/i18n`、`/api/rom/*` 的 GET |
| `game:write` | `/api/rom/*` 的 PUT/POST、`/api/play/save` 的 PUT |
| `network:read` | `/api/network`、`/api/network/scan` |
| `network:connect` | `/api/network/connect` |
| `sync:read` | `/api/sync` |
| `sync:run` | `/api/sync/run` |
| `files:any` | `/api/file`、`/api/files*`（高风险） |
| `packs:install` | `/api/packs/install`（高风险） |
| `rom:manage` | 新增、重命名、替换 ROM，以及 `/api/rom/zip/*`（高风险） |
| `crypto:use` | `/api/crypto/encrypt`、`/api/crypto/decrypt`（默认密钥或调用方密钥） |

能力默认**不授予**。`rom:manage` 允许新增、重命名、替换 ROM 和 ZIP 导入，属于高风险能力。用户在 Web「mod 详情 → 权限」勾选并保存后，`linkSlot/Config/mod_permissions.toml` 记录授权；`mod.json` 只能申请，不能自己批准。

管理页通过一个受能力表约束的通用接口访问：

```js
// GET
const status = await LinkSlotMod.request('api', {
  method: 'GET',
  path: '/api/status'
});

// JSON POST
await LinkSlotMod.request('api', {
  method: 'POST',
  path: '/api/network/connect',
  body: { ssid: 'MyWiFi', psk: 'secret' }
});

// 文本 / 二进制请求
await LinkSlotMod.request('api', {
  method: 'PUT',
  path: '/api/file?path=Mods/note.txt',
  content_type: 'text/plain;charset=utf-8',
  body: 'hello'
});
```

`api` 只接受上表的 `/api/**` 路径；`/api/mods/*`、`/api/settings`、`/api/upgrade`、`/api/packs/remove` 等管理接口不会通过这个桥暴露。mod 仍然不能直接 `fetch('/api/...')`。

## 9. 主题包

一个 mod 只要带了下列任意一个文件，就会被识别成主题包（`category: "theme"`）：

| 文件 | 作用 |
|---|---|
| `theme.txt` | 调色板 |
| `wallpaper.png` | 壁纸 |
| `font.otf` / `font.ttf` / `font.ttc` | 界面与卡带标签字体 |

`theme.txt` 是「键 + 十六进制颜色」，一行一个，`#` 开头为注释：

```ini
# 先选一个基础预设，再覆盖需要的颜色
preset warm

housing         #2a2320
recess          #1b1715
opening         #120f0e
edge            #4a3f39
bg              #14100e
bg_top          #221b18
surface         #2a2320
surface_border  #4a3f39
selected        #3a2f28
selected_border #c8a06a
ink             #f2e6d8
ink_selected    #fff6e8
ink_dim         #a89a8c
accent          #e0b070
accent_dim      #8a6a44
```

`preset` 可取 `default`（或 `dark`）、`web`（或 `teal`/`cyan`）、`warm`（或 `retro`/`amber`）。

优先级：**启用且 `order` 最小的主题包** → 存储卡 `linkSlot/Config/theme.txt` → 内置预设。主题在开机时读取一次。

---

## 10. 限制与安全

| 项 | 上限 |
|---|---|
| mod 数量 | 64 |
| 单 mod 动作数 | 16 |
| 全部动作数 | 64 |
| `main.lua` | 256 KiB（UTF-8） |
| 名称 | 64 字符 |
| 版本号 | 32 字符 |
| `order` | -9999 ..= 9999 |
| `boot_delay_minutes` | 0 ..= 1440 |
| 单张贴纸 | 4 MiB（Web 导入时校验） |
| 脚本运行时长 | manual 90 秒，其余时机 15 秒 |
| Lua 内存 | 32 MiB |
| 单次日志 | 200 行 |
| 单次读 / 写 | 64 MiB / 32 MiB |
| 远程请求 | 每次动作最多 16 次、合计 64 MiB（云同步内 64 次 / 128 MiB） |

沙箱边界：

- 文件只能碰 `States/`、`Saves/`、`linkSlot/mods/<自己>/`；
- 没有 `os` / `io` / `require`，不能起进程、不能读任意路径；
- 网络只有两条受控通道：`http`（`config.json` 的 `base_url` 主机）与 `webdav`（用户在设备上配置的同步方式）；
- `library.write` 只能写当前游戏的 `intro` / `guide`。

> mod 拥有 `Saves/` 与 `States/` 的写权限，**只启用你信任的 mod**。启用前建议先备份存档。

---

## 11. 兼容与升级

- 旧的 `schema=1` 单脚本 mod（根部 `main.lua`、`mod.json` 里写 `timing`）**不再支持**，需要迁移成 `actions/<action-id>/` 结构。
- `mod.json` 的未知字段会被忽略，缺失的 `version` / `order` / `category` 会取默认值，因此旧 mod 目录一般可以直接读。
- 新增字段时以本文件为准；`mods check` 会给出不符合 schema 的具体原因。