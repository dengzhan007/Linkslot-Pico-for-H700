-- linkSlot minimal manual action.
-- Replace this file with the real mod logic. Lua 5.4; no os/io/require.

local game = ctx.game
if not game then
  mods.report({ status = "error", detail = "没有当前游戏上下文" })
  return
end

mods.progress({ value = 0.25, label = "准备同步" })
log.info("demo mod: " .. tostring(game.name))

mods.report({
  status = "ok",
  direction = "upload",
  detail = "示例动作完成",
})
