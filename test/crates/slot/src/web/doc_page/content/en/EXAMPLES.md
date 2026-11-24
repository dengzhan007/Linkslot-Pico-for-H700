# Example: a manual action with HTTP and a conflict choice

```lua
-- actions/sync/main.lua (manual)
local base = ctx.mod.config and ctx.mod.config.base_url
if not base then
  mods.report{ status="error", error="config", detail="Set the server URL on the mod page first" }
  return
end

local response = http.get({"api", "saves"}, { game = ctx.game.name })
if response.status ~= 200 then
  mods.report{ status="error", error="server",
               detail="Server returned HTTP " .. tostring(response.status) }
  return
end

local choice = mods.choose{
  key = "conflict",
  title = "Save conflict",
  body = "Both the local and remote copies of " .. ctx.game.name .. " changed.",
  default = "remote",
  options = {
    { id="remote", label="Use the remote copy" },
    { id="local",  label="Use the local copy" },
  },
}
files.write(ctx.game.sav_path, response.body)
mods.report{ status="ok", direction="download",
             detail="Synced (" .. choice .. ")" }
```

This only shows how the APIs combine. A real mod should validate length, hashes, backups and error classes. The full GBAStation and SaveDeck examples live under `workdoc/examples/` in the repository.
