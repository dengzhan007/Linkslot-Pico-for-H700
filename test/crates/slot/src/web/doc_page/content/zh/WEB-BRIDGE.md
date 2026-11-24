# Web Bridge 能力与 `mod.json` 权限

mod 管理页运行在 sandbox iframe 中，不能直接请求 `/api/**`。所有宿主操作通过：

```js
LinkSlotMod.request(method, params)
```

## 权限声明

`mod.json`：

```json
"permissions": ["context:read", "game:write", "network:read"]
```

`action.json`：

```json
"capabilities": ["fs:saves", "fs:states", "fs:mod", "net:base_url", "ui:confirm"]
```

Web 端展示两者的并集。权限默认不授予，用户在 mod 详情页勾选并保存后写入：

```text
linkSlot/Config/mod_permissions.toml
```

这个文件在 mod 目录之外，mod 不能自己批准权限。

## 能力表

| 能力 | 允许的接口 |
|---|---|
| `context:read` | `/api/status`、`/api/roms`、`/api/saves`、`/api/achievements*`、`/api/i18n`、`/api/rom/*` 的 GET |
| `game:write` | 当前游戏资料：封面、简介、视频、颜色、模型、金手指、指南、存档写入 |
| `network:read` | `/api/network`、`/api/network/scan` |
| `network:connect` | `/api/network/connect` |
| `sync:read` | `/api/sync` |
| `sync:run` | `/api/sync/run` |
| `files:any` | `/api/file`、`/api/files*`（高危） |
| `packs:install` | `/api/packs/install`（高危） |
| `rom:manage` | `/api/rom/add`、`/api/rom/rename`、`PUT /api/rom`、`/api/rom/zip/*`（高危） |
| `crypto:use` | `POST /api/crypto/encrypt`、`POST /api/crypto/decrypt` |

动作/Lua 既有能力：

`fs:saves`、`fs:states`、`fs:mod`、`net:base_url`、`webdav:read`、`webdav:write`、`library:write`、`ui:confirm`。

## `api` 方法

```js
const status = await LinkSlotMod.request('api', {
  method: 'GET',
  path: '/api/status'
});

await LinkSlotMod.request('api', {
  method: 'POST',
  path: '/api/network/connect',
  body: { ssid: 'MyWiFi', psk: 'secret' }
});

await LinkSlotMod.request('api', {
  method: 'PUT',
  path: '/api/file?path=Mods/note.txt',
  content_type: 'text/plain;charset=utf-8',
  body: 'hello'
});

// 默认优先使用 Web「设置 → 加密密钥」里的用户密钥，没有时使用设备默认密钥。
// 用户密钥由设备密钥包裹保存在 linkSlot/Config/crypto.user，不是明文。
// 也可以传 key 使用本次调用方密钥；该参数不写盘。
const token = await LinkSlotMod.request('api', {
  method: 'POST',
  path: '/api/crypto/encrypt',
  body: { text: 'secret' }
});
const plain = await LinkSlotMod.request('api', {
  method: 'POST',
  path: '/api/crypto/decrypt',
  body: { token }
});

// Binary ROM replacement / upload (rom:manage)
await LinkSlotMod.request('api', {
  method: 'PUT',
  path: '/api/rom?rom=Games/GBA/Game.gba',
  body_base64: base64FromArrayBuffer(romBytes)
});
```

`api` 会按 `method + path` 查能力表，未授权直接拒绝。它只接受上表中的 `/api/**` 路径；`/api/mods/*`、`/api/settings`、`/api/upgrade`、`/api/packs/remove` 等管理接口不会暴露。

需要外部服务器时，使用 Lua 的 `http`（限制在 `config.json` 的 `base_url`）或 `webdav`。Web 管理页本身不能直接访问外部 URL。
