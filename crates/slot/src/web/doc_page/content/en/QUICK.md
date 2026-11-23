-- Host context (read only)
ctx.api / ctx.timing / ctx.unit / ctx.unit_path
ctx.game       -- {name, platform, core, rom_sha256, state_dir, sav_path, srm_path}
ctx.mod        -- {id, dir, config, config_error}
ctx.action     -- {id, name, dir}
ctx.choices / ctx.webdav / ctx.transfer

-- Local files: States/, Saves/ and the current mod only
files.read(rel) / files.write(rel, data) / files.list(rel)
files.stat(rel) / files.exists(rel) / files.mkdir(rel)

-- Current game metadata (manual timing only)
library.exists("intro"|"guide") / library.write("intro"|"guide", text)

-- Cloud storage, defaulting to ctx.webdav.active
webdav.get([profile,] path) / webdav.put([profile,] path, data)
webdav.put_atomic([profile,] path, data) / webdav.list([profile,] path)
webdav.stat([profile,] path) / webdav.mkdir([profile,] path)

-- Restricted HTTP: only the base_url from this mod's config.json
http.get(segments [, query]) / http.post(segments, body, content_type [, query])
http.request(method, segments, opts)   -- GET/POST/PUT

-- Results, dialogs, progress, helpers
mods.report{status="ok"|"noop"|"error", direction=..., error=..., detail="one line"}
mods.choose{key,title,body,options,default}  -- manual only
ui.confirm{title,body,yes,no} / ui.message{title,body,button}  -- manual only
mods.progress{value=0..1,label} or {current,total,label}
json.encode/decode / hash.sha256(_file) / zip.list/read/build
log.info/warn/error/debug / print(...)
