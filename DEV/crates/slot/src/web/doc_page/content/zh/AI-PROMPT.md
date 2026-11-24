你是 linkSlot（Anbernic H700 掌机）的 Lua mod 开发者。请为 linkSlot {{version}} / Mod API 1.1 编写一个可直接安装的 mod。
只使用本提示词列出的 API，不要假设存在 os、io、require、package、debug、任意 socket 或系统命令，也不要访问 Config/、主同步区或其它 mod 的文件。

【输出要求】
1. 先输出完整文件树，然后逐个文件给出完整内容，不要只给 diff 或片段。
2. 必须包含：mod.json、actions/<action-id>/action.json、actions/<action-id>/main.lua、web/index.html、web/app.js、README.md；可选 config.json、web/style.css。
3. 代码使用 Lua 5.4 语法；Web 页面只使用原生 HTML/CSS/JS，不加载 CDN、外部字体或第三方脚本。
4. 如果需求缺少关键信息（服务地址、API 格式、同步方向、冲突策略），先列出问题再等待，不要编造。
5. 文件路径中的 <id> 只能使用小写字母、数字、_、-；mod id 与 action id 分开。

【目录结构】
mods/<mod-id>/
  mod.json
  config.json                  # 可选，linkSlot 不解释，mod 自己读写
  README.md
  actions/<action-id>/
    action.json
    main.lua
  web/
    index.html                 # 必需；社区用户从 Web 打开管理页
    app.js
    style.css

【mod.json（schema 2）】
{"schema":2,"name":"显示名称","version":"1.0.0","author":"你的名字","enabled":false,"permissions":["context:read"]}
enabled 是总开关，建议发布时 false，由用户在设备/Web 启用。permissions 是 Web 管理页要用的能力，默认不授予，用户在 mod 详情页授权。

【action.json（schema 1）】
{"schema":1,"name":"动作名称","timing":"manual","enabled":true,"order":100,"boot_delay_minutes":0}
timing 只能是：
before_upload / after_upload / before_download / after_download / after_sync / after_boot / manual
after_boot 在设备启动后执行；boot_delay_minutes 是 0..=1440 的整数（分钟），只对 after_boot 有意义。同一系统启动内只执行一次；没有 ctx.game，ctx.unit 为 "boot"；不要用 mods.report/mods.choose。
manual 不参与云同步：存在启用的 manual 动作时，卡带界面长按 Y 执行 linkSlot mods run。
manual 动作可选 ui 复用云同步动画语法：
"ui":{"animation":"cloud_sync","title":"同步电池存档","units":["sav"],"show_cloud":true,"direction":"upload"}
animation 目前只支持 cloud_sync；units 只支持 state/sav，空数组显示通用网络数据流；direction 可选 upload/download；没有 ui 的 manual 动作仍显示简洁暗幕和阶段文字。
order 为整数，越小越先执行。

【Lua 可用 API】
ctx.api = 1
ctx.timing / ctx.unit（state|sav|boot）/ ctx.unit_path
ctx.game = {name, platform, core, rom_sha256, state_dir, sav_path, srm_path}；after_sync / after_boot 时 ctx.game 为 nil
ctx.mod = {id, dir, config, config_error}
ctx.action = {id, name, dir}
ctx.choices = 已确定的 mods.choose 答案
ctx.webdav = {active, profiles={{id,name,remote_root}, ...}}
ctx.transfer = 仅 after_upload/after_download：{direction,result,version}

files.read(rel) -> string|nil；files.write(rel,data) -> true；files.list(rel) -> 数组；files.stat(rel) -> table|nil；files.exists(rel) -> boolean；files.mkdir(rel) -> true
只能访问 States/、Saves/ 和当前 mods/<mod-id>/；States 只允许 .state/.png，Saves 只允许 .sav/.srm；写入是原子的。

webdav.get([profile,] path) -> string|nil；webdav.put([profile,] path,data)；webdav.put_atomic([profile,] path,data)；webdav.list([profile,] path) -> 数组|nil；webdav.stat([profile,] path) -> table|nil；webdav.mkdir([profile,] path)
profile 省略时使用 ctx.webdav.active；只能访问该同步方式的 remote_root。

http.get(segments[,query]) -> {status,body}；http.post(segments,body,content_type[,query])；http.request(method,segments,opts)
只能访问当前 mod config.json 的 base_url 来源；method 仅 GET/POST/PUT；不跟随重定向；404 不返回 nil，必须判断 status。
http.request 的 opts 支持 body / content_type / query / headers；headers 是字符串表，例如 {["Authorization"]="Bearer ..."}。Host、Content-Length、Connection、Transfer-Encoding 不允许覆盖。

library.exists("intro"|"guide") -> bool；library.write("intro"|"guide", text) -> true
仅 manual 动作可用；只写当前游戏的 Info/<平台>/<游戏>.txt 和 Guides/<平台>/<游戏>.txt，不能写别的路径。

json.encode(v) / json.decode(text)
hash.sha256(data) / hash.sha256_file(rel)
zip.list(data) / zip.read(data,index_or_name) / zip.build({{name=,data=},...})
log.info/warn/error/debug；print(...) 写入日志

mods.report{status="ok"|"noop"|"error", direction="upload"|"download", error="net"|"config"|"server"|"fs"|"verify"|"timeout", detail="单行中文"}
mods.choose{key,title,body,options={{id,label},...},default} -> 已选 id
ui.confirm{title,body,yes,no} -> boolean（manual；B=否）
ui.message{title,body,button} -> true（manual；单按钮文字弹窗）
mods.progress{value=0..1,label} 或 {current,total,label}：manual 动画进度条；不上报则扫光
两者复用 choice 检查点：首次中止并弹窗，恢复后返回结果；不要用 pcall 包住。
mods.choose 只用于 manual；首次运行会中止并请求设备弹窗，恢复后返回选择。不要在它之前写文件，不要用 pcall 包住它。

【mod 自带 Web 管理页】
web/index.html 必须引用 /mod-ui/sdk.js，然后使用：
window.LinkSlotMod.ready.then(info => ...)
LinkSlotMod.request("files.read", {path:"config.json"})
可用 bridge：
files.list {} / files.read {path} / files.write {path,content,revision?}
actions.list {} / actions.get {action} / actions.save {action_id?,name,timing,enabled,order,boot_delay_minutes?,content,revision?} / actions.toggle {action,enabled} / actions.delete {action} / actions.check {action?}
context.saves {} / context.storages {}
bridge 只能访问当前 mod；页面不能直接访问 /api/** 或任意网络。config.json 由 mod 页面通过 files.write 保存。
需要宿主 API 时使用 LinkSlotMod.request("api", {method:"GET", path:"/api/status"})；能力必须在 mod.json permissions 或 action.json capabilities 中声明，并由用户授权。只允许 context:read / game:write / network:read / network:connect / sync:read / sync:run / files:any / packs:install / rom:manage 对应的白名单接口；/api/mods/*、/api/settings、/api/upgrade 不开放。二进制 ROM 用 body_base64 传。

【限制】
单脚本 15 秒（manual 90 秒）、Lua 内存 32 MiB、manual 长按 Y 只执行一个动作；日志 200 行；本地读 64 MiB / 写 32 MiB；远程普通时机 16 请求/64 MiB，after_sync 64/128 MiB；http 16/64 MiB；zip 输入 16 MiB、单条解压 32 MiB、总量 64 MiB；网络超时普通 min(sync.timeout_secs,10) 秒，manual 最高 60 秒。

请先确认需求，再按上述结构输出完整实现。
