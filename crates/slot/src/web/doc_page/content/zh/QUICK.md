-- 上下文（只读全局 ctx）
ctx.api                 -- 1
ctx.timing              -- before_upload / after_upload / before_download / after_download / after_sync / after_boot / manual
ctx.unit                -- state / sav / boot；after_sync 为 nil；manual 为 sav；after_boot 为 boot
ctx.unit_path           -- state=目录；sav=现有 .sav/.srm；after_sync / after_boot=nil
ctx.game                -- {name, platform, core, rom_sha256, state_dir, sav_path, srm_path}；after_sync / after_boot 为 nil
ctx.mod.id / ctx.mod.dir
ctx.mod.config          -- 当前 mod config.json 解析结果（不存在为 nil）
ctx.mod.config_error    -- config.json 读取/解析错误（无错误为 nil）
ctx.action.id / ctx.action.name / ctx.action.dir
ctx.choices             -- { key = option }，已确定的 mods.choose 答案
ctx.webdav.active       -- 当前同步方式 id；没有兼容根时为 nil
ctx.webdav.profiles     -- { id, name, remote_root } 数组
ctx.transfer            -- after_upload/after_download：{ direction, result, version }

-- 本地文件（相对卡根；只允许 States/、Saves/、当前 mods/<id>/）
files.read(rel)         -- string 或 nil
files.write(rel, data)  -- true；States 只允许 .state/.png，Saves 只允许 .sav/.srm
files.list(rel)         -- { name, is_dir, size, mtime } 数组
files.stat(rel)         -- { name, is_dir, size, mtime } 或 nil
files.exists(rel)       -- boolean
files.mkdir(rel)        -- true

-- 敏感字符串加密（需要 crypto:use）
local token = crypto.encrypt("secret" [, key])
local text = crypto.decrypt(token [, key])

-- 当前游戏资料（仅 timing=manual；不影响 ROM/存档/数据库）
library.exists("intro"|"guide")       -- boolean
library.write("intro"|"guide", text)  -- true，写入 Info/Guides 的约定路径

-- 兼容根（省略 profile = ctx.webdav.active）
webdav.get([profile,] path)         -- string 或 nil（404）
webdav.put([profile,] path, data)
webdav.put_atomic([profile,] path, data)
webdav.list([profile,] path)        -- { name, is_dir, size, modified } 或 nil
webdav.stat([profile,] path)        -- 条目或 nil
webdav.mkdir([profile,] path)

-- 受控 HTTP：只能访问本 mod config.json 的 base_url 来源
http.get(segments [, query])                    -- { status, body }
http.post(segments, body, content_type [, query])
http.request(method, segments, opts)            -- GET/POST/PUT；opts: body/content_type/query/headers

-- 手动动作结果（仅 timing=manual）
mods.report{ status="ok"|"noop"|"error", direction="upload"|"download",
             error="net"|"config"|"server"|"fs"|"verify"|"timeout",
             detail="单行中文" }

-- 通用选择弹窗（仅 timing=manual；A 确认 / B 取消）
local choice = mods.choose{
  key="conflict", title="存档冲突", body="本地和 GBAStation 都改过",
  default="remote",
  options={
    { id="remote", label="用 GBAStation 覆盖本地" },
    { id="local",  label="用本地覆盖 GBAStation" },
  },
}

-- 辅助
json.encode(v) / json.decode(text)
hash.sha256(data) / hash.sha256_file(rel)
zip.list(data) / zip.read(data, index_or_name) / zip.build({ {name=, data=}, ... })
log.info / log.warn / log.error / log.debug
print(...)              -- 写入日志，不写 stdout
