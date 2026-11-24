//! Capability grants are deliberately stored outside `linkSlot/mods/`.
//!
//! A mod can edit its own directory through the Web bridge, so a grant file
//! inside that directory would let the mod approve itself.  The device UI and
//! the Web settings page write this sidecar; the bridge only reads it.

use std::path::Path;

use crate::{validate_id, ModError, CAPABILITIES};

pub const PERMISSIONS_REL: &str = "linkSlot/Config/mod_permissions.toml";

fn path(root: &Path) -> std::path::PathBuf {
    root.join(PERMISSIONS_REL)
}

/// Read the granted capabilities for one mod.  Unknown entries are ignored so
/// a future capability rename cannot brick an older mod.
pub fn granted(root: &Path, id: &str) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path(root)) else {
        return Vec::new();
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != id {
            continue;
        }
        return value
            .split(',')
            .map(str::trim)
            .filter(|value| CAPABILITIES.contains(value))
            .map(str::to_string)
            .collect();
    }
    Vec::new()
}

/// Replace one mod's granted list.  `granted` must be a subset of the
/// capabilities the mod actually declares.
pub fn set_granted(root: &Path, id: &str, granted: &[String]) -> Result<(), ModError> {
    validate_id(id).map_err(ModError::Invalid)?;
    let mut unique: Vec<&str> = Vec::new();
    for capability in granted {
        if !CAPABILITIES.contains(&capability.as_str()) {
            return Err(ModError::Invalid(format!(
                "未知能力 {capability:?}"
            )));
        }
        if !unique.contains(&capability.as_str()) {
            unique.push(capability);
        }
    }
    let file = path(root);
    let mut rows: Vec<(String, String)> = Vec::new();
    if let Ok(text) = std::fs::read_to_string(&file) {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                if key.trim() != id {
                    rows.push((key.trim().to_string(), value.trim().to_string()));
                }
            }
        }
    }
    rows.push((id.to_string(), unique.join(",")));
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let body = format!(
        "# linkSlot mod capability grants\n# id=capability,capability\n{}\n",
        rows.iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let dir = file.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)
        .map_err(|error| ModError::Io(format!("无法创建授权目录 {}：{error}", dir.display())))?;
    let tmp = file.with_extension("tmp");
    std::fs::write(&tmp, body.as_bytes())
        .map_err(|error| ModError::Io(format!("无法写入授权文件：{error}")))?;
    std::fs::rename(&tmp, &file)
        .map_err(|error| ModError::Io(format!("无法保存授权文件：{error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grants_are_scoped_and_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        set_granted(
            dir.path(),
            "one",
            &["context:read".into(), "sync:run".into()],
        )
        .unwrap();
        set_granted(dir.path(), "two", &["network:read".into()]).unwrap();
        assert_eq!(
            granted(dir.path(), "one"),
            vec!["context:read".to_string(), "sync:run".to_string()]
        );
        assert_eq!(
            granted(dir.path(), "two"),
            vec!["network:read".to_string()]
        );
        assert!(set_granted(dir.path(), "one", &["unknown".into()]).is_err());
    }
}
