# 能力、授权与 Web API bridge

Web 管理页不能直接 fetch 外部 URL。mod 在 `mod.json` 的 `permissions` 或 `action.json` 的 `capabilities` 中声明能力，用户在 mod 详情页授权后，才可通过 `LinkSlotMod.request('api', ...)` 调用宿主接口。授权记录在 `linkSlot/Config/mod_permissions.toml`，位于 mod 目录之外。

```text
context:read     /api/status /api/roms /api/saves /api/achievements* /api/i18n /api/rom/* GET
game:write       /api/rom/{cover,intro,video,color,model,cheat,guide} /api/play/save PUT
network:read     /api/network /api/network/scan
network:connect  /api/network/connect
sync:read        /api/sync
sync:run         /api/sync/run
files:any        /api/file /api/files*（高风险）
packs:install    /api/packs/install（高风险）
rom:manage       /api/rom/add /api/rom/rename PUT /api/rom /api/rom/zip/*（高风险）
crypto:use       /api/crypto/encrypt /api/crypto/decrypt（默认用户密钥或设备密钥）

LinkSlotMod.request('api', { method:'GET', path:'/api/status' })
LinkSlotMod.request('api', { method:'POST', path:'/api/network/connect', body:{ssid,psk} })
LinkSlotMod.request('api', { method:'PUT', path:'/api/files?path=...', body:'text', content_type:'text/plain' })
```
