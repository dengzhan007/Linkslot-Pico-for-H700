//! `/api/packs/**`：LSP 扩展包的列表、安装与卸载。
//!
//! 安装走专用入口而不是通用文件上传：只有 `System/linkSlot/cores/**`
//! 下的核心库会被写入，`System/` 的其余部分仍然写保护。

use serde_json::{json, Value};

use crate::packs;
use crate::web::http::Request;
use crate::web::http::Response;
use crate::web::Shared;

/// `GET /api/packs`：已安装的扩展包和当前应用版本。
pub(crate) fn list(shared: &Shared, _request: &Request) -> Response {
    Response::json(
        200,
        json!({
            "packs": packs::installed(&shared.root),
            "current": slot_ui::PROJECT_VERSION,
        }),
    )
}

/// `POST /api/packs/install`：请求体就是 `.lsp` 文件本身。
pub(crate) fn install(shared: &Shared, request: &Request) -> Response {
    let handled = (|| -> Result<Response, Response> {
        super::api::guard_write(shared, request)?;
        if request.body.len() > shared.upload_limit() {
            let mb = (shared.upload_limit() / (1024 * 1024)).max(1);
            return Err(Response::error(413, format!("扩展包超过 {mb}MB 限制")));
        }
        let installed = packs::install(&shared.root, &request.body)
            .map_err(|message| Response::error(400, message))?;
        Ok(Response::json(200, json!({ "ok": true, "pack": installed })))
    })();
    handled.unwrap_or_else(|response| response)
}

/// `POST /api/packs/remove`：`{"id": "..."}`。
pub(crate) fn remove(shared: &Shared, request: &Request) -> Response {
    let handled = (|| -> Result<Response, Response> {
        super::api::guard_write(shared, request)?;
        let value = super::mods::json_request(request)?;
        let id = value
            .get("id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| Response::error(400, "缺少扩展包 id"))?;
        let removed = packs::uninstall(&shared.root, id)
            .map_err(|message| Response::error(400, message))?;
        Ok(Response::json(200, json!({ "ok": true, "pack": removed })))
    })();
    handled.unwrap_or_else(|response| response)
}
