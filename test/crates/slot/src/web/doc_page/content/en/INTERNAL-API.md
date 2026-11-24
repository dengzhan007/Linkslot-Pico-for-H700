# Management API (internal)

These `/api/mods/**` endpoints back the manager page; a mod page normally uses the bridge instead. Every write needs the page's `X-Slot-Web: 1` header, and returns 409 while a game or sync is running.

| Endpoint | Notes |
|---|---|
| `GET /api/mods` | List: version, author, action count, enabled, category, UI support. |
| `GET /api/mods/item?id=` | Detail, action summary, declared and granted capabilities. |
| `GET/POST /api/mods/permissions` | Read or save this mod's grants. |
| `GET /api/mods/docs.zip` | API, AI prompt, templates and Web Bridge docs. |
| `GET /api/mods/files?id=` | Read-only file tree. |
| `GET/POST /api/mods/file` | Read or save a text file through the bridge. |
| `POST /api/mods/item` | Create or save name, version and master switch. |
| `POST /api/mods/toggle` | Enable or disable a mod. |
| `POST /api/mods/delete` | Move to `mods/.trash/`. |
| `POST /api/mods/uninstall` | Permanently remove the mod and its actions. |
| `GET /api/mods/action?id=&action=` | Action source and revision. |
| `POST /api/mods/action` | Create or save an action. |
| `POST /api/mods/action/toggle` | Enable or disable an action. |
| `POST /api/mods/test` | `linkSlot mods test --profile` |
| `POST /api/mods/check` | `linkSlot mods check --id [--action]` |
| `GET /api/status` | Writability and device state. |
| `GET/POST /api/clock` | Read device time/zone or set it from the browser. |
| `POST /api/network/save` | Save Wi-Fi credentials without switching. |
| `POST /api/network/connect` | Switch and connect Wi-Fi. |
| `POST /api/network/forget` | Forget a saved Wi-Fi; the connected one is refused. |
