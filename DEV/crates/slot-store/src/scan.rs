use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::gba::header_title_code;
use crate::Platform;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cart {
    pub platform: Platform,
    pub stem: String,
    pub rom: PathBuf,
    pub label: Option<PathBuf>,
    pub title: String,
    pub code: String,
    pub shell: Option<crate::ShellChoice>,
    /// Which physical Famicom/NES cartridge shell this game uses.
    pub model: crate::CartModel,
    /// The selected custom front-face overlay, if one is available.
    pub front: Option<PathBuf>,
}

impl Cart {
    pub fn read_header(&mut self) {
        match self.platform {
            Platform::Gba => {
                if let Some((title, code)) = header_title_code(&self.rom) {
                    self.title = title;
                    self.code = code;
                }
            }
            Platform::Gb | Platform::Gbc => {
                if let Some(title) = crate::gb::title(&self.rom) {
                    self.title = title;
                }
            }
            // A .nes file has no printable title header; feeding it to the
            // Game Boy reader would only invent mojibake. The shelf falls
            // back to the file name.
            Platform::Fc | Platform::Sfc | Platform::Md => {}
            // 街机 / NeoGeo 的"ROM"是 ZIP 压缩包，不是卡带：里面没有可打印的
            // 标题头，解压去猜游戏名只会写出乱码。名字一律取文件名。
            Platform::Arcade | Platform::Neogeo => {}
        }
    }
}

#[derive(Debug)]
pub enum StoreError {
    Io(std::io::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        StoreError::Io(e)
    }
}

pub fn scan(root: &Path) -> Result<Vec<Cart>, StoreError> {
    let started = Instant::now();
    let carts = collect(root)?;
    let collected = started.elapsed();
    eprintln!(
        "slot: scan: {} games under {}",
        carts.len(),
        root.join("Games").display()
    );
    // The report promises what the scan looks for. Deriving both lines from the
    // same platform list the collector walks keeps that promise true when a
    // machine is added, where a hand-written list silently goes stale.
    //
    // 街机 / NeoGeo 只有装了平台包才在这里出现：日志必须如实反映"没扫描"，
    // 否则用户会以为游戏丢在 Games/Arcade 里而扫描器视而不见。
    let scanned = crate::platform_packs::available_platforms(root);
    let expected: Vec<String> = scanned
        .iter()
        .map(|platform| format!("Games/{}", platform.dir_name()))
        .collect();
    let mut extensions: Vec<&str> = Vec::new();
    for platform in &scanned {
        for extension in platform.extensions() {
            if !extensions.contains(extension) {
                extensions.push(extension);
            }
        }
    }
    let supported: Vec<String> = extensions
        .iter()
        .map(|extension| format!(".{extension}"))
        .collect();
    let mut report = format!(
        "Slot R6 library scan: {} games\nExpected: {} (including subfolders).\nSupported: {}, case-insensitive. Archives are not ROM files.\n",
        carts.len(),
        expected.join(", "),
        supported.join(" / ")
    );
    for c in &carts {
        report.push_str(&format!(
            "{}\t{}\t{}\n",
            c.platform.dir_name(),
            initial(&c.stem),
            c.rom.strip_prefix(root).unwrap_or(&c.rom).display()
        ));
    }
    let report_started = Instant::now();
    let log_dir = root.join("linkSlot/Logs");
    let log_path = log_dir.join("slot-library.log");
    if let Err(e) = std::fs::create_dir_all(&log_dir)
        .and_then(|_| std::fs::write(&log_path, report))
    {
        eprintln!("slot: library report: {e}");
    }
    eprintln!(
        "slot: scan: collect {:.1} ms, report {:.1} ms",
        collected.as_secs_f64() * 1e3,
        report_started.elapsed().as_secs_f64() * 1e3
    );
    Ok(carts)
}

/// Scan the library without writing the report log.
///
/// The Web game library is a read-only view and must not touch the card merely
/// because someone opened the page.  The cartridge and label rules stay shared
/// with [`scan`] through [`collect`].
pub fn scan_quiet(root: &Path) -> Result<Vec<Cart>, StoreError> {
    collect(root)
}

fn collect(root: &Path) -> Result<Vec<Cart>, StoreError> {
    let read = |file: &str| std::fs::read_to_string(root.join(file)).unwrap_or_default();
    let shells = crate::cart_shell::layered(
        &read(crate::CART_SHELL_FILE),
        &read(crate::LABELS_SHELL_FILE),
    );
    let colours = crate::cart_color::read(root);
    let cached = crate::cart_meta::read_all(root);
    let mut carts = Vec::new();
    let mut nested_listings: HashMap<PathBuf, HashMap<String, String>> = HashMap::new();
    // 平台包的激活门就在这里：`available_platforms` 只在内置六台之外多出
    // 已激活的 Arcade/Neogeo，所以"没装包"连目录都不会被 `read_dir`。
    for platform in crate::platform_packs::available_platforms(root) {
        let dir = root.join("Games").join(platform.dir_name());
        let entries = rom_entries(&dir);
        let labels_dir = root.join("Labels").join(platform.dir_name());
        let labels = listing(&labels_dir);
        // 街机 / NeoGeo 用 DAT 决定"哪一条是游戏"、标题与机板。其它机种根本不
        // 读 DAT（hosted == false），下面的每个分支都会走"没有 entry"的老路径。
        let hosted = crate::platform_packs::is_hosted(platform);
        let catalog = if hosted {
            crate::arcade_dat::Catalog::load(root, platform)
        } else {
            crate::arcade_dat::Catalog::empty(platform)
        };
        // 判断"这个 clone 的母版在不在卡上"必须先知道目录里有哪些 set：
        // 只装了一个 clone 的用户仍然要能玩到它，不能静默吞掉。
        let sets: std::collections::BTreeSet<String> = if catalog.is_empty() {
            std::collections::BTreeSet::new()
        } else {
            entries
                .iter()
                .filter(|rom| !is_hidden(rom) && platform.accepts(rom) && rom.is_file())
                .filter_map(|rom| {
                    rom.file_stem()
                        .and_then(|stem| stem.to_str())
                        .map(|stem| stem.to_ascii_lowercase())
                })
                .collect()
        };
        let mut warned_orphan = false;
        for rom in entries {
            if is_hidden(&rom) || !platform.accepts(&rom) || !rom.is_file() {
                continue;
            }
            let Some(stem) = rom.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            // 卡上的 zip 名大小写不由我们决定（exFAT/Windows 大小写不敏感），
            // 先按 DAT 的原样查，再退回小写。
            let entry = if catalog.is_empty() {
                None
            } else {
                catalog
                    .get(stem)
                    .or_else(|| catalog.get(&stem.to_ascii_lowercase()))
            };
            if let Some(entry) = entry {
                // BIOS 集合不是游戏，不占书架。
                if entry.is_bios {
                    continue;
                }
                if !entry.parent.is_empty() {
                    if sets.contains(&entry.parent.to_ascii_lowercase()) {
                        // 母版在卡上：clone 收进"版本"子菜单，由母版代表。
                        continue;
                    }
                    if !warned_orphan {
                        // 每台机种只提醒一次：一个只拷了 clone 的库可能上千条，
                        // 逐条 eprintln 会把日志刷满。
                        warned_orphan = true;
                        eprintln!(
                            "slot: scan: {}/{} 是 {} 的 clone，但母版不在 Games/{}，按独立条目显示",
                            platform.dir_name(),
                            stem,
                            entry.parent,
                            platform.dir_name()
                        );
                    }
                }
            }
            // 街机的标题来自 DAT；缺 DAT 或缺这一条时退化成文件名，和内置机种
            // "没有标题就显示文件名"的既有行为一致。
            let title = match entry {
                Some(entry) if !entry.title.trim().is_empty() => entry.title.clone(),
                _ if hosted => stem.to_string(),
                _ => String::new(),
            };
            // 贴纸（cart 封面）查找顺序，stem 与 title 走**同一套**规则：
            //   1. `Labels/<dir>/<相对目录>/<stem>.png`
            //   2. `Labels/<dir>/<相对目录>/<title>.png`
            //   3. `Labels/<dir>/<stem>.png`
            //   4. `Labels/<dir>/<title>.png`
            // 同一层级里 stem 优先：两套命名并存时以文件名为准，更可预测。
            let folder = rom.parent().and_then(|parent| parent.strip_prefix(&dir).ok())
                .filter(|rel| !rel.as_os_str().is_empty()).map(|rel| labels_dir.join(rel));
            let mut nested = |name: &str| -> Option<PathBuf> {
                folder.as_ref().and_then(|folder| {
                    // Thousands of ROMs often share one deep Games/ subfolder.
                    // Listing the matching Labels/ subfolder once per folder,
                    // instead of once per ROM, is the difference between a
                    // large exFAT card scanning in seconds and in tens of
                    // seconds.
                    let entries = nested_listings.entry(folder.clone()).or_insert_with(|| listing(folder));
                    entries.get(name).map(|hit| folder.join(hit))
                })
            };
            let top = |name: &str| -> Option<PathBuf> {
                labels.get(name).map(|hit| labels_dir.join(hit))
            };
            let stem_name = format!("{stem}.png").to_lowercase();
            // 街机额外允许按标题命名的贴纸（`<title>.png`）；标题不能当文件名时
            // （含 `/` `\` `:`、过长）根本不查。
            let title_name = (hosted && title != stem && label_title_usable(&title))
                .then(|| format!("{}.png", title.to_lowercase()));
            let mut label = nested(&stem_name);
            if label.is_none() {
                if let Some(name) = &title_name {
                    label = nested(name);
                }
            }
            if label.is_none() {
                label = top(&stem_name);
            }
            if label.is_none() {
                if let Some(name) = &title_name {
                    label = top(name);
                }
            }
            let shell = shells.get(&crate::cart_shell::key(stem)).copied();
            let shell = colours
                .get(&crate::cart_color::key(platform, stem))
                .copied()
                .map(|colour| {
                    let mut choice = shell.unwrap_or(crate::ShellChoice {
                        outline: crate::Outline::Auto,
                        colour,
                        finish: crate::ShellFinish::Solid,
                    });
                    choice.colour = colour;
                    choice
                })
                .or(shell);
            let mut cart = Cart {
                platform,
                stem: stem.to_string(),
                title,
                code: String::new(),
                shell,
                label,
                rom,
                model: crate::CartModel::default(),
                front: None,
            };
            let meta = cached
                .get(&crate::cart_meta::relative(root, &cart.rom))
                .cloned();
            if let Some(meta) = meta {
                if let Some(cover) = meta.cover_path(root) {
                    cart.label = Some(cover);
                }
                if let Some(shell) = meta.shell_choice() {
                    cart.shell = Some(shell);
                }
                if platform == Platform::Fc {
                    cart.model = crate::CartModel::from_meta(&meta.extra);
                }
                cart.front = crate::fronts::selected_path(root, &cart, Some(&meta));
            }
            carts.push(cart);
        }
    }
    carts.sort_by(|a, b| {
        (a.platform as u8, sort_key(&a.stem)).cmp(&(b.platform as u8, sort_key(&b.stem)))
    });
    Ok(carts)
}

/// 标题能不能直接当文件名用。
///
/// FBNeo 的标题里有 `Street Fighter II: The World Warrior`、`Cadillacs and
/// Dinosaurs` 这种带冒号/斜杠的写法；冒号在 FAT/exFAT 上是非法字符，斜杠会被
/// 当成目录分隔符。这类标题干脆不做贴纸查找，回退到 stem 命名的贴纸。
fn label_title_usable(title: &str) -> bool {
    !title.trim().is_empty()
        && title.len() <= 120
        && !title.contains('/')
        && !title.contains('\\')
        && !title.contains(':')
}

pub fn listing(dir: &Path) -> HashMap<String, String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return HashMap::new();
    };
    entries
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|name| !name.starts_with('.'))
        .map(|name| (name.to_lowercase(), name))
        .collect()
}

/// Collect paths, not `DirEntry`s: a `DirEntry` can keep its parent
/// directory descriptor alive, and a library with thousands of ROMs can then
/// exhaust the process file-descriptor limit (`EMFILE`) before the shelf is
/// even built.
fn rom_entries(dir: &Path) -> Vec<PathBuf> {
    let mut pending = vec![dir.to_path_buf()];
    let mut files = Vec::new();
    while let Some(folder) = pending.pop() {
        let entries = match std::fs::read_dir(&folder) {
            Ok(entries) => entries,
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    eprintln!("slot: scan: {}: {e}", folder.display());
                }
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    eprintln!("slot: scan entry: {e}");
                    continue;
                }
            };
            let path = entry.path();
            if is_hidden(&path) {
                continue;
            }
            match entry.file_type() {
                // Do not follow directory symlinks: avoids cycles and
                // scanning outside Games.
                Ok(kind) if kind.is_dir() => pending.push(path),
                Ok(_) => files.push(path),
                Err(_) => {}
            }
        }
    }
    files
}

/// Sorting and all navigation share the same pinyin/ASCII initial.
pub fn sort_key(stem: &str) -> (u8, String) {
    let letter = initial(stem);
    let group = if letter == '#' { 0 } else { 1 };
    (group, format!("{letter}{}", stem.trim().to_uppercase()))
}

pub fn initial(stem: &str) -> char {
    crate::title_search::initial(stem).unwrap_or('#')
}


pub fn is_hidden(p: &Path) -> bool {
    p.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.'))
}
