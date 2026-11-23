//! Flat folder browsing for the cartridge shelf.
//!
//! The scanner already walks `Games/<platform>/` recursively.  This module turns
//! those carts into the shelf's browse sets without touching the full library.
//! Folder mode deliberately lists **only real subdirectories**, never the
//! synthetic `全部` or `根目录` sets:
//!
//! ```text
//! 动作
//! 动作/横版
//! 角色扮演
//! ```
//!
//! Every group counts only the ROMs directly inside its own directory, so one
//! cartridge is never shown twice under two different groups.  ROMs sitting
//! directly in `Games/<platform>/` stay reachable from the normal shelf; they
//! do not create a browse group.

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Component, Path};

use crate::{scan::sort_key, Cart, Platform};

/// One real directory under `Games/<platform>/` that directly contains ROMs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderGroup {
    pub platform: Platform,
    /// Relative directory path, e.g. `动作/横版`.  Never empty.
    pub path: String,
    /// Carts directly inside this directory.
    pub carts: Vec<Cart>,
}

/// Every browse set, ordered by [`Platform::ALL`] and then by directory path.
#[derive(Clone, Debug, Default)]
pub struct FolderCatalog {
    pub groups: Vec<FolderGroup>,
}

impl FolderCatalog {
    /// Build from the card root, so a directory name is taken from the real
    /// `Games/<platform>/` prefix.
    pub fn build(root: &Path, carts: &[Cart]) -> FolderCatalog {
        Self::collect(carts, Some(root))
    }

    /// Build without a root.  Used by tests and by `App::new`, where the card
    /// root is not known yet; paths are located by their `Games/<platform>`
    /// components instead.
    pub fn build_from_paths(carts: &[Cart]) -> FolderCatalog {
        Self::collect(carts, None)
    }

    fn collect(carts: &[Cart], root: Option<&Path>) -> FolderCatalog {
        let mut groups = Vec::new();
        for platform in Platform::ALL {
            let platform_carts: Vec<Cart> = carts
                .iter()
                .filter(|cart| cart.platform == platform)
                .cloned()
                .collect();
            if platform_carts.is_empty() {
                continue;
            }
            groups.extend(platform_groups(platform, &platform_carts, root));
        }
        FolderCatalog { groups }
    }

    /// The contiguous range of groups belonging to one platform.
    pub fn range_for(&self, platform: Platform) -> Option<Range<usize>> {
        let start = self
            .groups
            .iter()
            .position(|group| group.platform == platform)?;
        let len = self.groups[start..]
            .iter()
            .take_while(|group| group.platform == platform)
            .count();
        Some(start..start + len)
    }

    pub fn groups_for(&self, platform: Platform) -> &[FolderGroup] {
        match self.range_for(platform) {
            Some(range) => &self.groups[range],
            None => &[],
        }
    }

    /// A platform is browsable when it has at least one real ROM subdirectory.
    pub fn is_browsable(&self, platform: Platform) -> bool {
        self.range_for(platform).is_some()
    }

    /// The first directory of a platform, used when the remembered directory
    /// has disappeared or the selected cart lives in the platform root.
    pub fn first_index(&self, platform: Platform) -> Option<usize> {
        Some(self.range_for(platform)?.start)
    }

    pub fn find(&self, platform: Platform, path: &str) -> Option<usize> {
        let range = self.range_for(platform)?;
        range
            .clone()
            .find(|index| self.groups[*index].path == path)
    }

    /// The next browsable group in one direction, wrapping across platforms.
    pub fn step(&self, from: usize, direction: i32) -> Option<usize> {
        let count = self.groups.len();
        if count == 0 {
            return None;
        }
        let from = from.min(count - 1) as i32;
        Some((from + direction.signum()).rem_euclid(count as i32) as usize)
    }

    /// The group index whose directory directly contains `cart.rom`.
    ///
    /// `None` for a cart in the platform root (or outside `Games/<platform>/`);
    /// the caller then opens the platform's first real directory.
    pub fn group_of_cart(&self, cart: &Cart, root: Option<&Path>) -> Option<usize> {
        let path = relative_folder(cart, root)?;
        if path.is_empty() {
            return None;
        }
        self.find(cart.platform, &path)
    }

    /// Which group to open for `cart`, falling back to the platform's first
    /// real directory.
    pub fn group_or_first(&self, cart: &Cart, root: Option<&Path>) -> Option<usize> {
        self.group_of_cart(cart, root)
            .or_else(|| self.first_index(cart.platform))
    }
}

fn platform_groups(platform: Platform, carts: &[Cart], root: Option<&Path>) -> Vec<FolderGroup> {
    let mut directories: BTreeMap<String, Vec<Cart>> = BTreeMap::new();
    for cart in carts {
        match relative_folder(cart, root) {
            // Root ROMs are normal-shelf material, not a browse group.
            Some(path) if !path.is_empty() => directories.entry(path).or_default().push(cart.clone()),
            // A path outside `Games/<platform>/` cannot be placed in a
            // directory group either; the normal shelf still owns it.
            _ => {}
        }
    }
    let mut directories: Vec<(String, Vec<Cart>)> = directories.into_iter().collect();
    directories.sort_by(|(left, _), (right, _)| {
        sort_key(left)
            .cmp(&sort_key(right))
            .then_with(|| left.cmp(right))
    });
    directories
        .into_iter()
        .map(|(path, carts)| FolderGroup {
            platform,
            path,
            carts,
        })
        .collect()
}

/// `None` = outside the platform directory, `Some("")` = platform root,
/// `Some("动作")` = a real subdirectory.
pub fn relative_folder(cart: &Cart, root: Option<&Path>) -> Option<String> {
    let parent = cart.rom.parent()?;
    if let Some(root) = root {
        let dir = root.join("Games").join(cart.platform.dir_name());
        if let Ok(relative) = parent.strip_prefix(&dir) {
            return normalize(relative);
        }
    }
    relative_from_components(&cart.rom, cart.platform)
}

fn relative_from_components(rom: &Path, platform: Platform) -> Option<String> {
    let components: Vec<Component> = rom.components().collect();
    let wanted = platform.dir_name();
    let mut at = None;
    for index in 0..components.len() {
        let is_games = matches!(&components[index], Component::Normal(name) if name.to_string_lossy().eq_ignore_ascii_case("Games"));
        let is_platform = index + 1 < components.len()
            && matches!(&components[index + 1], Component::Normal(name) if name.to_string_lossy().eq_ignore_ascii_case(wanted));
        if is_games && is_platform {
            at = Some(index + 1);
        }
    }
    let at = at.or_else(|| {
        components.iter().position(|component| {
            matches!(component, Component::Normal(name) if name.to_string_lossy().eq_ignore_ascii_case(wanted))
        })
    })?;
    // The last component is the ROM file itself; everything between the
    // platform directory and the file is the folder path.
    let end = components.len().saturating_sub(1).max(at + 1);
    normalize_parts(&components[at + 1..end])
}

fn normalize(path: &Path) -> Option<String> {
    normalize_parts(&path.components().collect::<Vec<_>>())
}

fn normalize_parts(parts: &[Component]) -> Option<String> {
    let mut out: Vec<String> = Vec::new();
    for part in parts {
        match part {
            Component::CurDir => {}
            Component::Normal(name) => {
                let name = name.to_str()?;
                if name.is_empty() {
                    continue;
                }
                if name.starts_with('.') {
                    return None;
                }
                out.push(name.to_string());
            }
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(out.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn cart(stem: &str, relative: &str) -> Cart {
        Cart {
            platform: Platform::Gba,
            stem: stem.to_string(),
            rom: PathBuf::from(format!("Games/GBA/{relative}/{stem}.gba")),
            label: None,
            title: String::new(),
            code: String::new(),
            shell: None,
            model: Default::default(),
            front: None,
        }
    }

    fn root_cart(stem: &str) -> Cart {
        Cart {
            platform: Platform::Gba,
            stem: stem.to_string(),
            rom: PathBuf::from(format!("Games/GBA/{stem}.gba")),
            label: None,
            title: String::new(),
            code: String::new(),
            shell: None,
            model: Default::default(),
            front: None,
        }
    }

    fn labels(catalog: &FolderCatalog, platform: Platform) -> Vec<String> {
        catalog
            .groups_for(platform)
            .iter()
            .map(|group| group.path.clone())
            .collect()
    }

    #[test]
    fn plans_only_the_real_subdirectories() {
        let carts = vec![
            root_cart("龙珠"),
            cart("阿拉丁", "动作"),
            cart("合金弹头", "动作/横版"),
            cart("黄金太阳", "角色扮演"),
        ];
        let catalog = FolderCatalog::build_from_paths(&carts);
        assert_eq!(
            labels(&catalog, Platform::Gba),
            vec!["动作", "动作/横版", "角色扮演"]
        );
    }

    #[test]
    fn groups_count_only_their_own_directory() {
        let carts = vec![
            root_cart("龙珠"),
            cart("阿拉丁", "动作"),
            cart("合金弹头", "动作/横版"),
            cart("黄金太阳", "角色扮演"),
        ];
        let catalog = FolderCatalog::build_from_paths(&carts);
        let index = |path: &str| catalog.find(Platform::Gba, path).unwrap();
        assert_eq!(catalog.groups[index("动作")].carts.len(), 1);
        assert_eq!(catalog.groups[index("动作/横版")].carts.len(), 1);
        assert_eq!(catalog.groups[index("角色扮演")].carts.len(), 1);
        assert_eq!(catalog.groups.len(), 3, "root/all leaked into the catalog");
    }

    #[test]
    fn a_nested_directory_never_counts_as_its_parent() {
        let carts = vec![cart("合金弹头", "动作/横版"), cart("阿拉丁", "动作")];
        let catalog = FolderCatalog::build_from_paths(&carts);
        let action = catalog.find(Platform::Gba, "动作").unwrap();
        let carts: Vec<&str> = catalog.groups[action]
            .carts
            .iter()
            .map(|cart| cart.stem.as_str())
            .collect();
        assert_eq!(carts, vec!["阿拉丁"]);
    }

    #[test]
    fn root_roms_only_are_not_a_browsable_platform() {
        let carts = vec![root_cart("龙珠"), root_cart("火影")];
        let catalog = FolderCatalog::build_from_paths(&carts);
        assert!(catalog.groups.is_empty());
        assert!(!catalog.is_browsable(Platform::Gba));
        assert_eq!(catalog.first_index(Platform::Gba), None);
    }

    #[test]
    fn a_root_cart_falls_back_to_the_first_real_folder() {
        let carts = vec![root_cart("龙珠"), cart("阿拉丁", "动作")];
        let catalog = FolderCatalog::build_from_paths(&carts);
        assert_eq!(catalog.group_of_cart(&carts[0], None), None);
        assert_eq!(catalog.group_or_first(&carts[0], None), Some(0));
        assert_eq!(catalog.group_of_cart(&carts[1], None), Some(0));
    }

    #[test]
    fn the_first_index_is_the_first_directory_of_that_platform() {
        let mut gb = cart("俄罗斯方块", "益智");
        gb.platform = Platform::Gb;
        gb.rom = PathBuf::from("Games/GB/益智/俄罗斯方块.gb");
        let catalog = FolderCatalog::build_from_paths(&[cart("阿拉丁", "动作"), gb]);
        assert_eq!(catalog.first_index(Platform::Gba), Some(0));
        assert_eq!(catalog.first_index(Platform::Gb), Some(1));
        assert_eq!(catalog.first_index(Platform::Sfc), None);
    }

    #[test]
    fn an_empty_library_has_no_groups() {
        let catalog = FolderCatalog::build_from_paths(&[]);
        assert!(catalog.groups.is_empty());
        assert_eq!(catalog.step(0, 1), None);
    }

    #[test]
    fn the_same_stem_in_two_folders_stays_two_carts() {
        let carts = vec![cart("同名", "动作"), cart("同名", "角色扮演")];
        let catalog = FolderCatalog::build_from_paths(&carts);
        let action = catalog.find(Platform::Gba, "动作").unwrap();
        let role = catalog.find(Platform::Gba, "角色扮演").unwrap();
        assert_eq!(catalog.groups[action].carts.len(), 1);
        assert_eq!(catalog.groups[role].carts.len(), 1);
        assert_ne!(
            catalog.groups[action].carts[0].rom,
            catalog.groups[role].carts[0].rom
        );
        assert_eq!(catalog.groups.len(), 2);
    }

    #[test]
    fn platforms_keep_their_declared_order_and_cycle_across_them() {
        let mut gb = cart("俄罗斯方块", "益智");
        gb.platform = Platform::Gb;
        gb.rom = PathBuf::from("Games/GB/益智/俄罗斯方块.gb");
        let carts = vec![cart("阿拉丁", "动作"), gb];
        let catalog = FolderCatalog::build_from_paths(&carts);
        let gba = catalog.range_for(Platform::Gba).unwrap();
        let gb_range = catalog.range_for(Platform::Gb).unwrap();
        assert!(gba.end <= gb_range.start);
        let last_gba = gba.end - 1;
        assert_eq!(catalog.step(last_gba, 1), Some(gb_range.start));
        assert_eq!(catalog.step(gb_range.start, -1), Some(last_gba));
        assert_eq!(catalog.step(0, -1), Some(catalog.groups.len() - 1));
    }

    #[test]
    fn directories_sort_by_the_shelf_order_not_by_raw_bytes() {
        let carts = vec![cart("A", "zeta"), cart("B", "Alpha"), cart("C", "动作")];
        let catalog = FolderCatalog::build_from_paths(&carts);
        let paths: Vec<&str> = catalog
            .groups
            .iter()
            .map(|group| group.path.as_str())
            .collect();
        assert_eq!(paths, vec!["Alpha", "动作", "zeta"]);
    }

    #[test]
    fn root_and_outside_roms_never_become_groups() {
        let mut outside = root_cart("局外");
        outside.rom = PathBuf::from("Elsewhere/局外.gba");
        let carts = vec![
            cart("阿拉丁", "动作"),
            root_cart("龙珠"),
            outside,
        ];
        let catalog = FolderCatalog::build_from_paths(&carts);
        assert_eq!(catalog.groups.len(), 1);
        assert_eq!(labels(&catalog, Platform::Gba), vec!["动作"]);
    }

    #[test]
    fn a_root_relative_path_normalises_to_the_same_folder() {
        let carts = vec![cart("阿拉丁", "动作")];
        let absolute = FolderCatalog::build_from_paths(&carts);
        let relative = FolderCatalog::build(
            Path::new("."),
            &[Cart {
                rom: PathBuf::from("Games/GBA/动作/阿拉丁.gba"),
                ..carts[0].clone()
            }],
        );
        assert_eq!(absolute.groups, relative.groups);
    }
}
