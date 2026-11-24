//! Per-machine shelf backgrounds uploaded from the Web settings page.

use std::path::PathBuf;

use serde_json::{json, Value};

use crate::web::http::{self, Request, Response};
use crate::web::Shared;

pub const DIR_REL: &str = "linkSlot/Backdrops";
pub const PLATFORMS: [(&str, &str); 6] = [
    ("GBA", "gba.png"),
    ("GB", "gb.png"),
    ("GBC", "gbc.png"),
    ("FC", "fc.png"),
    ("SFC", "sfc.png"),
    ("MD", "md.png"),
];

fn platform_file(platform: &str) -> Option<&'static str> {
    PLATFORMS
        .iter()
        .find(|(id, _)| id.eq_ignore_ascii_case(platform))
        .map(|(_, file)| *file)
}

fn path(shared: &Shared, platform: &str) -> Option<PathBuf> {
    Some(shared.root.join(DIR_REL).join(platform_file(platform)?))
}

pub(crate) fn list(shared: &Shared, _request: &Request) -> Response {
    let rows: Vec<Value> = PLATFORMS
        .iter()
        .map(|(label, file)| {
            let path = shared.root.join(DIR_REL).join(file);
            let meta = std::fs::metadata(&path).ok();
            json!({
                "platform": label,
                "configured": meta.is_some(),
                "size": meta.map(|m| m.len()).unwrap_or(0),
                "path": format!("{DIR_REL}/{file}"),
            })
        })
        .collect();
    Response::json(200, json!({ "platforms": rows }))
}

pub(crate) fn save(shared: &Shared, request: &Request) -> Response {
    let handled = (|| -> Result<Response, Response> {
        super::api::guard_write(shared, request)?;
        let platform = http::query_param(request, "platform")
            .map_err(|error| Response::error(error.status, error.message))?
            .ok_or_else(|| Response::error(400, "缺少平台参数"))?;
        let target = path(shared, &platform)
            .ok_or_else(|| Response::error(400, "不支持的平台"))?;
        if request.body.is_empty() {
            return Err(Response::error(400, "背景图为空"));
        }
        if request.body.len() > shared.upload_limit() {
            let mb = (shared.upload_limit() / (1024 * 1024)).max(1);
            return Err(Response::error(413, format!("背景图超过 {mb}MB 上限")));
        }
        let is_png = request.body.starts_with(b"\x89PNG\r\n\x1a\n");
        let is_jpeg = request.body.starts_with(&[0xff, 0xd8, 0xff]);
        if !is_png && !is_jpeg {
            return Err(Response::error(400, "背景图只支持 PNG 或 JPEG"));
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| Response::error(500, format!("创建背景图目录失败：{error}")))?;
        }
        let temp = target.with_extension("upload");
        std::fs::write(&temp, &request.body)
            .map_err(|error| Response::error(500, format!("写入背景图失败：{error}")))?;
        // Decode once before replacing the live file, so a malformed image
        // cannot leave the device with a broken background.
        if slot_ui::wallpaper_face(&temp).is_none() {
            let _ = std::fs::remove_file(&temp);
            return Err(Response::error(400, "背景图无法解码"));
        }
        std::fs::rename(&temp, &target)
            .map_err(|error| Response::error(500, format!("保存背景图失败：{error}")))?;
        Ok(Response::json(
            200,
            json!({ "ok": true, "platform": platform, "configured": true }),
        ))
    })();
    handled.unwrap_or_else(|response| response)
}

pub(crate) fn delete(shared: &Shared, request: &Request) -> Response {
    let handled = (|| -> Result<Response, Response> {
        super::api::guard_write(shared, request)?;
        let value = http::json_body(request)
            .map_err(|error| Response::error(error.status, error.message))?;
        let platform = http::json_string(&value, "platform")
            .ok_or_else(|| Response::error(400, "缺少平台参数"))?;
        let target = path(shared, &platform)
            .ok_or_else(|| Response::error(400, "不支持的平台"))?;
        match std::fs::remove_file(&target) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Response::error(500, format!("删除背景图失败：{error}"))),
        }
        Ok(Response::json(
            200,
            json!({ "ok": true, "platform": platform, "configured": false }),
        ))
    })();
    handled.unwrap_or_else(|response| response)
}
