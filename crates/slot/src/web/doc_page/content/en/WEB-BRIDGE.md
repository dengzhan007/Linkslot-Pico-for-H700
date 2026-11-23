# Mod management page (LinkSlotMod bridge)

```html
<script src="/mod-ui/sdk.js"></script>
LinkSlotMod.ready.then(function (info) {
  return LinkSlotMod.request("files.read", { path: "config.json" });
}).then(function (file) { /* file.content, file.revision */ });
```

| bridge | params | notes |
|---|---|---|
| `files.list` | `{}` | Read-only tree of this mod. |
| `files.read` | `{path}` | `{path, content, revision, editable, size}` |
| `files.write` | `{path, content, revision?}` | Atomic write; revision prevents overwriting another page. |
| `actions.list` | `{}` | Actions of this mod. |
| `actions.get` | `{action}` | Action metadata and Lua source. |
| `actions.save` | `{action_id?,name,timing,enabled,order,boot_delay_minutes?,content,revision?}` | Creates or saves an action. |
| `actions.toggle` | `{action, enabled}` | Enable or disable one action. |
| `actions.delete` | `{action}` | Moves the action to the mod trash. |
| `actions.check` | `{action?}` | Runs `linkSlot mods check`. |
| `context.saves` | `{}` | Save and game list, read-only. |
| `context.storages` | `{}` | `{storages, timings}`, read-only. |
| `tianma.start` | `{path, overwrite?}` | Host-side Tianma import; writes the cover to `Labels/GBA` and the text/video to `Info/GBA`. |
| `tianma.status` / `tianma.cancel` | `{}` | Progress or cancellation of that import. |
| `gbastation.*` | `{base_url, keys, overwrite?}` | Only for the gbastation-rom-sync mod. |
| `api` | `{method, path, body?}` | Capability-gated `/api/**` call; `/api/mods/*`, `/api/settings` and `/api/upgrade` are not exposed. |

The page runs with an opaque origin in a sandboxed iframe. Every call is scoped to this mod; it cannot read another mod or the card.
