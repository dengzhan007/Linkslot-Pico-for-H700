# Capabilities and the restricted API bridge

A mod page cannot fetch `/api/**` directly. Declare capabilities in `mod.json` `permissions` or `action.json` `capabilities`; the user grants them on the mod detail page. Only granted capabilities reach `LinkSlotMod.request('api', ...)`. Records live in `linkSlot/Config/mod_permissions.toml`, outside the mod directory.

```text
context:read      /api/status /api/roms /api/saves /api/achievements* /api/i18n /api/rom/* GET
game:write        /api/rom/{cover,intro,video,color,model,cheat,guide} /api/play/save PUT
network:read      /api/network /api/network/scan
network:connect   /api/network/connect
sync:read         /api/sync
sync:run          /api/sync/run
files:any         /api/file /api/files* (high risk)
packs:install     /api/packs/install (high risk)
rom:manage        /api/rom/add /api/rom/rename PUT /api/rom /api/rom/zip/* (high risk)
crypto:use        /api/crypto/encrypt /api/crypto/decrypt
```
