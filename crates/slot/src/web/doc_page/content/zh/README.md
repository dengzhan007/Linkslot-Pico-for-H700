# linkSlot Mod 开发文档包

适用版本：linkSlot 1.x / Mod API 1.1（capabilities）

这个包可以直接交给 AI，要求它按文档开发一个可安装的 linkSlot mod。

## 文件

| 文件 | 内容 |
|---|---|
| `AI-PROMPT.md` | 可直接发给 AI 的开发提示词 |
| `API.md` | 完整 mod 清单、Lua API、执行时机、限制和 Web Bridge |
| `WEB-BRIDGE.md` | Web 管理页能力授权与 `LinkSlotMod.request('api')` |
| `CAPABILITIES.md` | 能力、授权与 bridge 速查 |
| `EXAMPLES.md` | manual + HTTP / 冲突选择示例 |
| `INTERNAL-API.md` | linkSlot 管理页使用的内部 API |
| `QUICK.md` | Lua API 速查 |
| `templates/basic/` | 最小可安装 mod 模板（含 `web/index.html` 管理页示例） |

## 快速开始

1. 把整个目录交给 AI；
2. 在 `AI-PROMPT.md` 末尾补充你的需求；
3. 要求 AI 输出完整的 `mods/<mod-id>/` 文件树；
4. 把生成的文件复制到存储卡 `linkSlot/mods/<mod-id>/`；
5. 在设备 Web 的 mods 页面导入或启用，先运行 `mods check`。

## 注意

- 不要假设存在 `os`、`io`、`require`、`package`、`debug`、任意 socket 或系统命令。
- Lua 文件、网络和 WebDAV 访问都受沙箱限制。
- Web 管理页运行在 sandbox iframe 中，只能通过 `LinkSlotMod.request` bridge 操作。
- 新增的 Web API 能力默认不授予，用户必须在 mod 详情页授权。
- 不要复用其他 mod 的 ID，不要写入 `Config/` 或别人的 mod 目录。
