//! One-click export of every embedded mod-development document.
//!
//! The files are compiled into the binary so the export works offline and
//! always matches the running build.  The archive is intentionally small and
//! can be handed directly to an AI together with a feature request.

use std::io::{Cursor, Write};

use crate::web::http::{self, Response};

const README: &str = include_str!("doc_page/content/zh/README.md");
const AI_PROMPT: &str = include_str!("doc_page/content/zh/AI-PROMPT.md");
const API: &str = include_str!("doc_page/content/zh/API.md");
const WEB_BRIDGE: &str = include_str!("doc_page/content/zh/WEB-BRIDGE.md");
const CAPABILITIES: &str = include_str!("doc_page/content/zh/CAPABILITIES.md");
const EXAMPLES: &str = include_str!("doc_page/content/zh/EXAMPLES.md");
const INTERNAL_API: &str = include_str!("doc_page/content/zh/INTERNAL-API.md");
const QUICK: &str = include_str!("doc_page/content/zh/QUICK.md");
const TEMPLATE_MOD: &str = include_str!("static/mod-docs/templates/basic/mod.json");
const TEMPLATE_ACTION: &str =
    include_str!("static/mod-docs/templates/basic/actions/demo/action.json");
const TEMPLATE_LUA: &str =
    include_str!("static/mod-docs/templates/basic/actions/demo/main.lua");
const TEMPLATE_README: &str = include_str!("static/mod-docs/templates/basic/README.md");
const TEMPLATE_HTML: &str = include_str!("static/mod-docs/templates/basic/web/index.html");
const TEMPLATE_JS: &str = include_str!("static/mod-docs/templates/basic/web/app.js");
const TEMPLATE_CONFIG: &str =
    include_str!("static/mod-docs/templates/basic/config.example.json");

const FILES: &[(&str, &str)] = &[
    ("README.md", README),
    ("AI-PROMPT.md", AI_PROMPT),
    ("API.md", API),
    ("WEB-BRIDGE.md", WEB_BRIDGE),
    ("CAPABILITIES.md", CAPABILITIES),
    ("EXAMPLES.md", EXAMPLES),
    ("INTERNAL-API.md", INTERNAL_API),
    ("QUICK.md", QUICK),
    ("templates/basic/mod.json", TEMPLATE_MOD),
    ("templates/basic/actions/demo/action.json", TEMPLATE_ACTION),
    ("templates/basic/actions/demo/main.lua", TEMPLATE_LUA),
    ("templates/basic/README.md", TEMPLATE_README),
    ("templates/basic/web/index.html", TEMPLATE_HTML),
    ("templates/basic/web/app.js", TEMPLATE_JS),
    ("templates/basic/config.example.json", TEMPLATE_CONFIG),
];

pub(crate) fn export(_shared: &crate::web::Shared, _request: &crate::web::http::Request) -> Response {
    let mut cursor = Cursor::new(Vec::new());
    // The page fills {{version}} from /api/status; an exported prompt is read
    // outside the device, so bake the build version in instead.
    let ai_prompt = AI_PROMPT.replace("{{version}}", env!("CARGO_PKG_VERSION"));
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, content) in FILES {
            let content = if *name == "AI-PROMPT.md" {
                ai_prompt.as_str()
            } else {
                content
            };
            if zip.start_file(*name, options).is_err()
                || zip.write_all(content.as_bytes()).is_err()
            {
                return Response::error(500, "无法生成 mod 开发文档包");
            }
        }
        if zip.finish().is_err() {
            return Response::error(500, "无法生成 mod 开发文档包");
        }
    }
    Response::new(200, "application/zip", cursor.into_inner())
        .with_header(
            "Content-Disposition",
            http::content_disposition("attachment", "linkslot-mod-docs.zip"),
        )
        .with_header("Cache-Control", "no-store")
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPORT_README: &str = include_str!("doc_page/content/zh/README.md");
    const EXPORT_API: &str = include_str!("doc_page/content/zh/API.md");
    const EXPORT_BRIDGE: &str = include_str!("doc_page/content/zh/WEB-BRIDGE.md");
    const EXPORT_AI: &str = include_str!("doc_page/content/zh/AI-PROMPT.md");

    #[test]
    fn the_docs_archive_contains_the_ai_prompt_and_api() {
        let dir = tempfile::tempdir().unwrap();
        let shared = crate::web::test_support::test_shared(dir.path().to_path_buf());
        let request = crate::web::test_support::request(
            crate::web::http::Method::Get,
            "/api/mods/docs.zip",
            "",
        );
        let response = export(&shared, &request);
        assert_eq!(response.status, 200);
        let mut archive = zip::ZipArchive::new(Cursor::new(response.body))
            .expect("mod docs archive is not a zip");
        let mut names: Vec<String> = (0..archive.len())
            .map(|index| archive.by_index(index).unwrap().name().to_string())
            .collect();
        names.sort();
        assert!(names.contains(&"AI-PROMPT.md".to_string()));
        assert!(names.contains(&"API.md".to_string()));
        assert!(names.contains(&"WEB-BRIDGE.md".to_string()));
        assert!(names.contains(&"templates/basic/actions/demo/main.lua".to_string()));
        use std::io::Read;
        let mut text = String::new();
        archive
            .by_name("AI-PROMPT.md")
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        assert!(text.contains("Mod API 1.1"));
        assert!(
            !text.contains("{{version}}"),
            "the exported AI prompt must not contain a page-only placeholder"
        );
        assert!(text.contains(env!("CARGO_PKG_VERSION")));
    }

    /// The embedded docs page and the export bundle are edited separately
    /// today.  This is the cheap guard that keeps the important capability
    /// list and the bridge contract from drifting apart before the two are
    /// generated from one Markdown source.
    #[test]
    fn the_docs_page_and_export_agree_on_key_capabilities() {
        let page = crate::web::assets::MOD_DOCS_HTML;
        for needle in ["crypto:use", "rom:manage", "LinkSlotMod.request"] {
            assert!(EXPORT_API.contains(needle), "API.md is missing {needle}");
            assert!(page.contains(needle), "the docs page is missing {needle}");
        }
        assert!(EXPORT_BRIDGE.contains("LinkSlotMod.request"));
        assert!(EXPORT_AI.contains("Mod API 1.1"));
    }

    #[test]
    fn the_exported_docs_do_not_reference_removed_files_or_fixed_versions() {
        assert!(!EXPORT_README.contains("templates/web-ui"));
        for (name, text) in [
            ("README.md", EXPORT_README),
            ("API.md", EXPORT_API),
            ("WEB-BRIDGE.md", EXPORT_BRIDGE),
            ("AI-PROMPT.md", EXPORT_AI),
        ] {
            for stale in ["0.6.79", "1.0.26"] {
                assert!(!text.contains(stale), "{name} still mentions {stale}");
            }
        }
    }
}
