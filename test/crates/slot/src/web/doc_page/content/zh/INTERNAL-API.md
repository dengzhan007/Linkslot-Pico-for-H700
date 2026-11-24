# linkSlot mods 管理 API（内部）

以下是 linkSlot 自己的管理页使用的 `/api/mods/**` 接口，普通 mod 作者不需要直接调用；mod 自带页面请使用上方文档中的 `LinkSlotMod.request` bridge。

| 接口 | 说明 |
|---|---|
| `GET /api/mods` | mod 列表：版本、作者、动作数、启用状态、`category` 分类、是否有管理页面、运行支持标记 |
| `GET /api/mods/item?id=` | mod 详情、版本、动作摘要、已声明/已授权能力 |
| `GET/POST /api/mods/permissions` | 读取 / 保存当前 mod 的能力授权 |
| `GET /api/mods/docs.zip` | 导出 API、AI 提示词、模板与 Web Bridge 文档包 |
| `GET /api/mods/files?id=` | 当前 mod 的只读目录树 |
| `GET/POST /api/mods/file` | mod bridge 的文本文件读取/保存 |
| `GET /mod-ui/<id>/web/*` | sandbox 中的 mod 自带管理页面 |
| `POST /api/mods/item` | 新建 / 保存 mod 名称、版本与总开关 |
| `POST /api/mods/toggle` | 启用 / 停用 mod |
| `POST /api/mods/delete` | 移动到 `mods/.trash/` |
| `POST /api/mods/uninstall` | 永久卸载 mod 及其全部动作 |
| `GET /api/mods/action?id=&action=` | 动作源码与 revision |
| `POST /api/mods/action` | 新建 / 保存动作 |
| `POST /api/mods/action/toggle` | 启用 / 停用动作 |
| `POST /api/mods/action/delete` | 删除动作到 mod 回收站 |
| `POST /api/mods/test` | `linkSlot mods test --profile` |
| `POST /api/mods/check` | `linkSlot mods check --id [--action]` |
| `POST /api/files/upload?path=` | 原始字节上传；导入 mod 的图片、字体、主题文件走这里 |
| `POST /api/files/mkdir` | 创建目录；导入 mod 前先建 `web/` 等子目录 |
| `GET /api/status` | 设备是否可写、当前阶段等基础状态 |
| `GET/POST /api/clock` | 读取设备时间/时区，或请求用浏览器时间校准 |
| `POST /api/network/save` | 保存 WiFi 凭据，不切换当前网络 |
| `POST /api/network/connect` | 切换并连接 WiFi |
| `POST /api/network/forget` | 忘记已保存的 WiFi；当前连接会被拒绝 |
| `POST /api/crypto/encrypt` | 使用设备密钥或调用方密钥加密文本 |
| `POST /api/crypto/decrypt` | 解密文本；失败统一报告密钥错误 |

所有写接口都需要页面发送的 `X-Slot-Web: 1` 请求头；设备游戏中或同步中时写入返回 409。内置动作编辑器已移除，动作由 mod 自带管理页面通过受限 bridge 管理。
