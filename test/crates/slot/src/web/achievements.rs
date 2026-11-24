//! Phase 2 achievement data for the Web page.
//!
//! `/api/achievements` lists installed definitions (the game picker);
//! `/api/achievements/game?platform=FC&stem=...` returns one game's rows and
//! unlocks. Both are read-only: definitions are user data and the handlers
//! never create or rewrite them.

use std::collections::{BTreeSet, HashMap};

use serde_json::json;

use crate::cheevos;
use crate::common_achievements;
use crate::web::http::{self, Request, Response};
use crate::web::Shared;

pub fn list(shared: &Shared, _request: &Request) -> Response {
    let games: Vec<serde_json::Value> = cheevos::installed_games(&shared.root)
        .into_iter()
        .map(|game| {
            json!({
                "platform": game.platform,
                "stem": game.stem,
                "title": game.title,
                "sha256": game.sha256,
                "total": game.total,
                "unlocked": game.unlocked,
                "points": game.points,
            })
        })
        .collect();
    Response::json(
        200,
        json!({
            "schema": cheevos::SCHEMA,
            "games": games,
        }),
    )
}

/// `GET /api/achievements/public` — the five built-in per-ROM achievements,
/// each with the games that unlocked it.  Built-in unlocks share the normal
/// `games/<platform>/<sha>.json` records, so no extra storage is needed.
pub fn public(shared: &Shared, _request: &Request) -> Response {
    let language = slot_store::read_language(&shared.root);
    let mut games_by_id: HashMap<&'static str, Vec<serde_json::Value>> = HashMap::new();
    let mut covered: BTreeSet<(String, String)> = BTreeSet::new();
    let metas = slot_store::cart_meta::read_all(&shared.root);
    let by_sha: HashMap<(String, String), &slot_store::cart_meta::CartMeta> = metas
        .values()
        .filter_map(|meta| {
            let sha = meta.sha256.as_ref()?;
            Some(((meta.platform.clone(), sha.to_ascii_lowercase()), meta))
        })
        .collect();

    let games_dir = shared.root.join(cheevos::GAMES_DIR);
    if let Ok(platforms) = std::fs::read_dir(&games_dir) {
        for platform in platforms.flatten() {
            if !platform.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                continue;
            }
            let platform_name = platform.file_name().to_string_lossy().into_owned();
            let Some(platform_id) = slot_model::Platform::ALL
                .iter()
                .copied()
                .find(|platform| platform.dir_name().eq_ignore_ascii_case(&platform_name))
            else {
                continue;
            };
            let Ok(files) = std::fs::read_dir(platform.path()) else {
                continue;
            };
            for file in files.flatten() {
                if !file.file_type().map(|kind| kind.is_file()).unwrap_or(false) {
                    continue;
                }
                let Some(sha) = file
                    .file_name()
                    .to_string_lossy()
                    .strip_suffix(".json")
                    .map(|sha| sha.to_ascii_lowercase())
                else {
                    continue;
                };
                if sha.len() != 64 || !sha.chars().all(|ch| ch.is_ascii_hexdigit()) {
                    continue;
                }
                let Some(meta) = by_sha.get(&(platform_name.clone(), sha.clone())) else {
                    continue;
                };
                let unlocks = cheevos::load_unlocks(&shared.root, &platform_name, &sha);
                let stem = meta
                    .rom
                    .rsplit('/')
                    .next()
                    .and_then(|name| name.rsplit_once('.').map(|(stem, _)| stem))
                    .unwrap_or(&meta.rom)
                    .to_string();
                let mut any = false;
                for definition in common_achievements::DEFINITIONS {
                    let Some(unlocked_at) = unlocks.unlocks.get(definition.id).copied() else {
                        continue;
                    };
                    any = true;
                    games_by_id.entry(definition.id).or_default().push(json!({
                        "platform": platform_id.dir_name(),
                        "sha256": sha,
                        "stem": stem,
                        "rom": meta.rom,
                        "title": meta.title.clone().unwrap_or_else(|| stem.clone()),
                        "unlocked_at": unlocked_at,
                    }));
                }
                if any {
                    covered.insert((platform_name.clone(), sha.clone()));
                }
            }
        }
    }

    let achievements: Vec<serde_json::Value> = common_achievements::DEFINITIONS
        .iter()
        .map(|definition| {
            let mut games = games_by_id.remove(definition.id).unwrap_or_default();
            games.sort_by(|left, right| {
                right["unlocked_at"]
                    .as_u64()
                    .cmp(&left["unlocked_at"].as_u64())
                    .then_with(|| left["title"].as_str().cmp(&right["title"].as_str()))
            });
            json!({
                "id": definition.id,
                "title": slot_i18n::t(language, definition.title_key),
                "description": slot_i18n::t(language, definition.desc_key),
                "points": definition.points,
                "badge": format!("/api/achievements/public/badge?id={}", definition.id),
                "games": games,
            })
        })
        .collect();
    let total_points: u32 = common_achievements::DEFINITIONS
        .iter()
        .map(|definition| definition.points)
        .sum();
    let earned_points: u32 = achievements
        .iter()
        .filter(|achievement| {
            achievement["games"]
                .as_array()
                .is_some_and(|games| !games.is_empty())
        })
        .filter_map(|achievement| achievement["points"].as_u64())
        .sum::<u64>() as u32;
    Response::json(
        200,
        json!({
            "schema": 1,
            "summary": {
                "total": common_achievements::DEFINITIONS.len(),
                "games": covered.len(),
                "points": earned_points,
                "total_points": total_points,
            },
            "achievements": achievements,
        }),
    )
}

/// `GET /api/achievements/public/badge?id=builtin:*` — built-in badge PNG,
/// served from the binary so it works without an achievement pack.
pub fn public_badge(_shared: &Shared, request: &Request) -> Response {
    let id = match query(request, "id") {
        Ok(Some(value)) => value,
        Ok(None) => return Response::error(400, "缺少 id 参数"),
        Err(response) => return response,
    };
    let Some(bytes) = common_achievements::png(&id) else {
        return Response::error(404, "没有该公共成就徽章");
    };
    Response::new(200, "image/png", bytes.to_vec())
        .with_header("Cache-Control", "public, max-age=86400")
}

pub fn badge(shared: &Shared, request: &Request) -> Response {
    let platform = match query(request, "platform") {
        Ok(Some(value)) => value.to_ascii_uppercase(),
        Ok(None) => return Response::error(400, "缺少 platform 参数"),
        Err(response) => return response,
    };
    let sha256 = match query(request, "sha256") {
        Ok(Some(value)) => value.to_ascii_lowercase(),
        Ok(None) => return Response::error(400, "缺少 sha256 参数"),
        Err(response) => return response,
    };
    let id = match query(request, "id") {
        Ok(Some(value)) => value,
        Ok(None) => return Response::error(400, "缺少 id 参数"),
        Err(response) => return response,
    };
    if platform.is_empty()
        || !platform.chars().all(|character| character.is_ascii_alphanumeric())
        || sha256.len() != 64
        || !sha256.chars().all(|character| character.is_ascii_hexdigit())
        || id.is_empty()
        || id.len() > 10
        || !id.chars().all(|character| character.is_ascii_digit())
    {
        return Response::error(400, "成就图标参数不合法");
    }
    let path = shared
        .root
        .join("linkSlot/Achievements/badges")
        .join(&platform)
        .join(&sha256)
        .join(format!("{id}.png"));
    match std::fs::read(path) {
        Ok(bytes) => Response::new(200, "image/png", bytes)
            .with_header("Cache-Control", "public, max-age=86400"),
        Err(_) => Response::error(404, "没有该成就图标"),
    }
}

fn valid_stem(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && value != "."
        && value != ".."
        && !value.contains(['/', '\\', '\0'])
        && !value.chars().any(char::is_control)
}

pub fn game(shared: &Shared, request: &Request) -> Response {
    let platform = match query(request, "platform") {
        Ok(Some(value)) => value,
        Ok(None) => return Response::error(400, "缺少 platform 参数"),
        Err(response) => return response,
    };
    let Some(platform) = slot_store::Platform::ALL
        .iter()
        .find(|known| known.dir_name().eq_ignore_ascii_case(&platform))
        .map(|known| known.dir_name().to_string())
    else {
        return Response::error(400, "未知平台");
    };
    let sha256 = match query(request, "sha256") {
        Ok(Some(value)) => Some(value),
        Ok(None) => None,
        Err(response) => return response,
    };
    if let Some(value) = &sha256 {
        if value.len() != 64 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
            return Response::error(400, "sha256 参数不合法");
        }
    }
    let stem = match query(request, "stem") {
        Ok(Some(value)) => Some(value),
        Ok(None) => None,
        Err(response) => return response,
    };
    if let Some(value) = &stem {
        if !valid_stem(value) {
            return Response::error(400, "stem 参数不合法");
        }
    }
    let set = if let Some(sha256) = sha256 {
        match cheevos::load_for_sha(&shared.root, &platform, &sha256) {
            Ok(Some(set)) => set,
            Ok(None) => return Response::error(404, "该游戏没有成就定义"),
            Err(error) => return Response::error(400, error),
        }
    } else if let Some(stem) = stem.as_deref() {
        match cheevos::load_for_stem(&shared.root, &platform, stem) {
            Ok(Some(set)) => set,
            Ok(None) => return Response::error(404, "该游戏没有成就定义"),
            Err(error) => return Response::error(400, error),
        }
    } else {
        return Response::error(400, "缺少 sha256 或 stem 参数");
    };
    let unlocks = cheevos::load_unlocks(&shared.root, &set.game.platform, &set.game.sha256);
    let achievements: Vec<serde_json::Value> = set
        .rules
        .iter()
        .map(|rule| {
            let at = unlocks.unlocks.get(&rule.id).copied();
            let badge = rule
                .id
                .rsplit(':')
                .next()
                .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
                .map(|id| {
                    format!(
                        "/api/achievements/badge?platform={}&sha256={}&id={id}",
                        set.game.platform, set.game.sha256
                    )
                });
            json!({
                "id": rule.id,
                "title": rule.title,
                "description": rule.description,
                "points": rule.points,
                "unlocked": at.is_some(),
                "at": at,
                "badge": badge,
            })
        })
        .collect();
    let total = set.rules.len() as u32;
    let unlocked = set
        .rules
        .iter()
        .filter(|rule| unlocks.unlocks.contains_key(&rule.id))
        .count() as u32;
    let points: u32 = set
        .rules
        .iter()
        .filter(|rule| unlocks.unlocks.contains_key(&rule.id))
        .map(|rule| rule.points)
        .sum();
    let stem_label = stem.unwrap_or_else(|| set.game.sha256.clone());
    Response::json(
        200,
        json!({
            "schema": cheevos::SCHEMA,
            "game": {
                "platform": set.game.platform,
                "stem": stem_label,
                "title": set.game.title.unwrap_or_else(|| stem_label.clone()),
                "sha256": set.game.sha256,
            },
            "summary": {
                "unlocked": unlocked,
                "total": total,
                "points": points,
            },
            "achievements": achievements,
        }),
    )
}

fn query(request: &Request, name: &str) -> Result<Option<String>, Response> {
    http::query_param(request, name).map_err(|error| Response::error(error.status, error.message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::http::Method;
    use crate::web::test_support::{body_json, request, test_shared};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn public_tab_lists_all_five_before_any_game_is_played() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("slot-public-{stamp}"));
        std::fs::create_dir_all(&root).unwrap();
        let shared = test_shared(root.clone());
        let response = public(
            &shared,
            &request(Method::Get, "/api/achievements/public", ""),
        );
        assert_eq!(response.status, 200);
        let body = body_json(&response);
        assert_eq!(body["summary"]["total"], 5);
        assert_eq!(body["summary"]["games"], 0);
        assert_eq!(body["achievements"].as_array().unwrap().len(), 5);
        assert_eq!(body["achievements"][0]["games"].as_array().unwrap().len(), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn game_queries_reject_platform_and_stem_path_components() {
        let shared = test_shared(std::env::temp_dir());
        let bad_platform = game(
            &shared,
            &request(
                Method::Get,
                "/api/achievements/game",
                "platform=..%2F..%2Ftmp&stem=x",
            ),
        );
        assert_eq!(bad_platform.status, 400);

        let bad_stem = game(
            &shared,
            &request(
                Method::Get,
                "/api/achievements/game",
                "platform=GBA&stem=..%2Fsecret",
            ),
        );
        assert_eq!(bad_stem.status, 400);
    }

    #[test]
    fn public_badges_are_served_from_the_binary() {
        let response = public_badge(
            &test_shared(std::env::temp_dir()),
            &request(
                Method::Get,
                "/api/achievements/public/badge",
                "id=builtin%3Afirst-launch",
            ),
        );
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "image/png");
        assert_eq!(&response.body[..8], b"\x89PNG\r\n\x1a\n");
    }
}