//! Lexical path containment for the H700 card-backed storage provider.
//!
//! This module does not access a filesystem. It converts a Libretro path into
//! a card-relative path only after proving that it is beneath an explicitly
//! configured mount prefix and one of the frontend's allowed top-level roots.

extern crate alloc;
use alloc::string::String;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathError {
    Empty, InvalidCharacter, InvalidMountPrefix, InvalidRelativeBase,
    OutsideMount, Traversal, EmptyComponent, UnsupportedPrefix, DisallowedRoot,
}

fn valid_config_path(path: &str) -> bool {
    !path.is_empty() && !path.starts_with('/') && !path.ends_with('/')
        && !path.contains('\\') && !path.contains(':')
        && path.split('/').all(|c| !c.is_empty() && c != "." && c != "..")
}

fn validate_request_chars(path: &str) -> Result<(), PathError> {
    if path.is_empty() { return Err(PathError::Empty); }
    if path.bytes().any(|b| b == 0 || b < 0x20 || b == 0x7f) || path.contains('\\') {
        return Err(PathError::InvalidCharacter);
    }
    if path.contains(':') { return Err(PathError::UnsupportedPrefix); }
    Ok(())
}

/// Resolve a core-supplied path into a card-relative slash-separated path.
/// Relative paths require an explicit base; there is no process-CWD fallback.
/// This is lexical containment only, not a filesystem or handle implementation.
pub fn resolve_card_path(
    request: &str,
    mount_prefix: &str,
    allowed_roots: &[&str],
    relative_base: Option<&str>,
) -> Result<String, PathError> {
    validate_request_chars(request)?;
    if mount_prefix.is_empty() || !mount_prefix.starts_with('/') || mount_prefix == "/"
        || mount_prefix.ends_with('/') || mount_prefix.contains('\\') || mount_prefix.contains(':')
        || mount_prefix.split('/').skip(1).any(|c| c.is_empty() || c == "." || c == "..")
    {
        return Err(PathError::InvalidMountPrefix);
    }
    if request.starts_with('/') {
        let rest = request.strip_prefix(mount_prefix).ok_or(PathError::OutsideMount)?;
        if !rest.starts_with('/') { return Err(PathError::OutsideMount); }
        validate_card_relative(&rest[1..], allowed_roots)
    } else {
        let base = relative_base.ok_or(PathError::OutsideMount)?;
        if !valid_config_path(base) { return Err(PathError::InvalidRelativeBase); }
        let mut combined = String::from(base);
        combined.push('/');
        combined.push_str(request);
        validate_card_relative(&combined, allowed_roots)
    }
}

fn validate_card_relative(path: &str, allowed_roots: &[&str]) -> Result<String, PathError> {
    if path.is_empty() { return Err(PathError::Empty); }
    let mut first = None;
    let mut out = String::with_capacity(path.len());
    for (index, component) in path.split('/').enumerate() {
        if component.is_empty() { return Err(PathError::EmptyComponent); }
        if component == "." || component == ".." { return Err(PathError::Traversal); }
        if component.contains(':') || component.contains('\\') { return Err(PathError::UnsupportedPrefix); }
        if index == 0 { first = Some(component); }
        if !out.is_empty() { out.push('/'); }
        out.push_str(component);
    }
    let root = first.ok_or(PathError::Empty)?;
    if !allowed_roots.iter().any(|allowed| *allowed == root) {
        return Err(PathError::DisallowedRoot);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    const ROOTS: &[&str] = &["Games", "BIOS", "Saves", "States", "System", "linkSlot"];

    #[test] fn accepts_absolute_path_inside_exact_mount() {
        assert_eq!(resolve_card_path("/mnt/sdcard/Games/GBA/title.gba", "/mnt/sdcard", ROOTS, None),
            Ok(String::from("Games/GBA/title.gba")));
    }
    #[test] fn rejects_sibling_mount_prefix() {
        assert_eq!(resolve_card_path("/mnt/sdcard2/Games/a.gba", "/mnt/sdcard", ROOTS, None),
            Err(PathError::OutsideMount));
    }
    #[test] fn rejects_paths_outside_mount() {
        assert_eq!(resolve_card_path("/etc/passwd", "/mnt/sdcard", ROOTS, None),
            Err(PathError::OutsideMount));
    }
    #[test] fn rejects_parent_and_dot_components() {
        for path in ["/mnt/sdcard/Games/../BIOS/bios.bin", "/mnt/sdcard/Games/./title.gba",
                     "/mnt/sdcard/Games/a/../../etc/passwd"] {
            assert_eq!(resolve_card_path(path, "/mnt/sdcard", ROOTS, None), Err(PathError::Traversal));
        }
    }
    #[test] fn rejects_repeated_and_trailing_separators() {
        for path in ["/mnt/sdcard//Games/a.gba", "/mnt/sdcard/Games//a.gba", "/mnt/sdcard/Games/a.gba/"] {
            assert_eq!(resolve_card_path(path, "/mnt/sdcard", ROOTS, None), Err(PathError::EmptyComponent));
        }
    }
    #[test] fn rejects_unknown_roots_and_uri_or_drive_prefixes() {
        assert_eq!(resolve_card_path("content://provider/game", "/mnt/sdcard", ROOTS, Some("Games")),
            Err(PathError::UnsupportedPrefix));
        assert_eq!(resolve_card_path("C:/Games/a.gba", "/mnt/sdcard", ROOTS, Some("Games")),
            Err(PathError::UnsupportedPrefix));
        assert_eq!(resolve_card_path("/mnt/sdcard/private/a", "/mnt/sdcard", ROOTS, None),
            Err(PathError::DisallowedRoot));
    }
    #[test] fn relative_paths_require_an_explicit_base() {
        assert_eq!(resolve_card_path("title.gba", "/mnt/sdcard", ROOTS, None), Err(PathError::OutsideMount));
        assert_eq!(resolve_card_path("title.gba", "/mnt/sdcard", ROOTS, Some("Games/GBA")),
            Ok(String::from("Games/GBA/title.gba")));
    }
    #[test] fn rejects_invalid_configured_roots() {
        assert_eq!(resolve_card_path("title.gba", "/mnt/sdcard", ROOTS, Some("../Games")),
            Err(PathError::InvalidRelativeBase));
        assert_eq!(resolve_card_path("Games/a.gba", "/mnt//sdcard", ROOTS, Some("Games")),
            Err(PathError::InvalidMountPrefix));
    }
    #[test] fn rejects_control_characters_and_backslashes() {
        assert_eq!(resolve_card_path("/mnt/sdcard/Games/a\n.gba", "/mnt/sdcard", ROOTS, None),
            Err(PathError::InvalidCharacter));
        assert_eq!(resolve_card_path("/mnt/sdcard\\Games\\a.gba", "/mnt/sdcard", ROOTS, None),
            Err(PathError::InvalidCharacter));
    }
}
