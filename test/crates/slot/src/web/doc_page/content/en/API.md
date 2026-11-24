# API reference

## Package layout and lifecycle

```text
mods/<mod-id>/
  mod.json              # required, schema 2
  config.json           # optional, owned by the mod
  README.md
  actions/<action-id>/
    action.json         # schema 1: timing, order, enabled, boot_delay_minutes
    main.lua            # Lua 5.4, one sandbox per action
  web/index.html        # required for the mod's own management page
  web/app.js
```

Install a complete ZIP from **Mods & Packs → Import**; the ZIP must contain `mods/<id>/` or the mod folder itself. A mod without `web/index.html` is listed but cannot be enabled. Files can be browsed on the Files page, but edits belong to the mod's own page.

## mod.json and action.json

```json
// mod.json (schema 2)
{"schema":2,"name":"Example mod","version":"1.0.0","author":"you","enabled":false,
 "permissions":["context:read"],"category":"sync"}

// actions/sync/action.json (schema 1)
{"schema":1,"name":"Sync","timing":"manual","enabled":true,"order":100,"boot_delay_minutes":0}
```

`timing` is one of `manual`, `before_upload`, `after_upload`, `before_download`, `after_download`, `after_sync`, `after_boot`. `after_boot` accepts `boot_delay_minutes` 0..=1440. `category` is `sync` / `info` / `tool` / `theme`; the list infers it when omitted. Set `enabled:false` when publishing.

## Limits and troubleshooting

| Item | Limit / behaviour |
|---|---|
| Script | 15 s (manual 90 s), 32 MiB Lua memory, 200 log lines. |
| Local files | Read 64 MiB, write 32 MiB; `States/`, `Saves/` and this mod. |
| WebDAV | 16 calls / 64 MiB normally; `after_sync` 64 / 128 MiB; timeout min(sync.timeout_secs, 10 s). |
| HTTP | Only the configured `base_url`; 16 calls / 64 MiB; no redirects; manual up to 60 s. |
| ZIP | 16 MiB input, 256 entries, 32 MiB per entry, 64 MiB total. |
| Boundary | No `Config/`, main sync area, other mods, shell or arbitrary URLs. |

A manual action does not run: check the mod switch, the action switch and `timing=manual`, then run `linkSlot mods check --id <id> --action <action>`. The page does not open: confirm `web/index.html` exists under `mods/<id>/`.
