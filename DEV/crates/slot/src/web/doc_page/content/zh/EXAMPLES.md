# 示例：manual + HTTP / 冲突选择

```lua
-- actions/sync/main.lua（manual）
local base = ctx.mod.config and ctx.mod.config.base_url
if not base then
  mods.report{ status="error", error="config", detail="请先在管理页填写服务地址" }
  return
end

local response = http.get({"api", "saves"}, { game = ctx.game.name })
if response.status ~= 200 then
  mods.report{ status="error", error="server",
               detail="服务返回 HTTP " .. tostring(response.status) }
  return
end

local choice = mods.choose{
  key = "conflict",
  title = "存档冲突",
  body = "本地和服务端都修改了「" .. ctx.game.name .. "」。",
  default = "remote",
  options = {
    { id="remote", label="用服务端覆盖本地" },
    { id="local",  label="用本地覆盖服务端" },
  },
}
files.write(ctx.game.sav_path, response.body)
mods.report{ status="ok", direction="download",
             detail="已同步（" .. choice .. "）" }
```

示例只展示 API 组合方式；实际 mod 应校验长度、哈希、备份和错误分类。GBAStation / SaveDeck 两个完整示例在仓库 `workdoc/examples/`。
