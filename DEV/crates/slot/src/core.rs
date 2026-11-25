use std::path::{Path, PathBuf};

use slot_retro::{LibretroCore, MockCore, RetroCore};
use slot_store::{Core, GbPalette, Platform};

use crate::root;

const CORE_ENV: &str = "SLOT_CORE";

pub fn dylib_name(core: Core) -> String {
    format!(
        "{}_libretro.{}",
        core.as_str(),
        std::env::consts::DLL_EXTENSION
    )
}

fn candidates(root: &Path, core: Core) -> Vec<PathBuf> {
    if let Some(named) = std::env::var_os(CORE_ENV) {
        return vec![PathBuf::from(named)];
    }
    let name = dylib_name(core);
    let mut paths = vec![root
        .join(crate::root::SYSTEM_CORES)
        .join(core.as_str())
        .join(&name)];
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        paths.push(dir.join(&name));
        paths.push(dir.join("vendor").join(&name));
    }
    paths.push(Path::new("vendor").join(&name));
    paths
}

pub struct Opened {
    pub core: Box<dyn RetroCore>,
    pub named: bool,
}

/// How a core is told to open for a link, before `load`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkSetup {
    /// mGBA's cable: the core needs the player this device drives.
    Mgba { player: u8 },
    /// armGBA's netpacket link: Link Mode Separate and the port in use.
    /// (The core reads both at load; changing either restarts the console.)
    Armgba { wireless: bool },
}

pub fn open_core(
    root: &Path,
    core: Core,
    platform: Platform,
    serial: &str,
    colour: bool,
    palette: Option<GbPalette>,
    link: Option<LinkSetup>,
) -> Opened {
    open_core_with_system_dir(root, core, platform, serial, colour, palette, link, None)
}

/// 与 [`open_core`] 相同，但可以**覆盖 system/BIOS 目录**。
///
/// 街机与 NeoGeo 的 BIOS 按设计放在 `BIOS/fbneo/`（`neogeo.zip` 等），而模拟器
/// 的默认 system 目录是 `BIOS/`。调用方（`session::spawn_core`）用
/// `arcade::launch_plan().system_dir` 把 `BIOS/fbneo` 优先交给核心，其余机种
/// 仍传 `None`，走的还是原来的 `BIOS/`。
pub fn open_core_with_system_dir(
    root: &Path,
    core: Core,
    platform: Platform,
    serial: &str,
    colour: bool,
    palette: Option<GbPalette>,
    link: Option<LinkSetup>,
    system_dir: Option<&Path>,
) -> Opened {
    let paths = candidates(root, core);
    match open_named(
        root,
        core,
        platform,
        serial,
        colour,
        palette,
        link,
        &paths,
        system_dir,
    ) {
        Some(core) => Opened { core, named: true },
        None => {
            report_missing(core, &paths);
            Opened {
                core: Box::new(MockCore::new()),
                named: false,
            }
        }
    }
}

pub fn open_core_for(
    root: &Path,
    core: Core,
    platform: Platform,
    serial: &str,
    colour: bool,
    paths: &[PathBuf],
) -> Box<dyn RetroCore> {
    open_core_for_palette(root, core, platform, serial, colour, None, paths)
}

pub fn open_core_for_palette(
    root: &Path,
    core: Core,
    platform: Platform,
    serial: &str,
    colour: bool,
    palette: Option<GbPalette>,
    paths: &[PathBuf],
) -> Box<dyn RetroCore> {
    open_named(root, core, platform, serial, colour, palette, None, paths, None).unwrap_or_else(|| {
        report_missing(core, paths);
        Box::new(MockCore::new())
    })
}

fn open_named(
    root: &Path,
    core: Core,
    platform: Platform,
    serial: &str,
    colour: bool,
    palette: Option<GbPalette>,
    link: Option<LinkSetup>,
    paths: &[PathBuf],
    system_dir: Option<&Path>,
) -> Option<Box<dyn RetroCore>> {
    let bios = system_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root::bios_dir(root));
    let saves = root::saves_dir(root);
    for path in paths {
        if !path.exists() {
            continue;
        }
        match LibretroCore::open_with(path, &bios, &saves) {
            Ok(mut opened) => {
                // The canvas the core's frames are cut at: 240x160 for the
                // handhelds, 256x224 for a Famicom. Told before `load`, because
                // the first frame is already sized by it.
                let (frame_w, frame_h) = platform.frame();
                opened.set_canvas(frame_w, frame_h);
                opened.set_six_button_pads(six_button_pads(platform));
                apply_core_options(
                    &mut opened,
                    core,
                    serial,
                    root::has_real_bios(root),
                    colour,
                    palette,
                );
                if let Some(link) = link {
                    apply_link_options(&mut opened, core, link);
                }
                eprintln!("slot: core {}", path.display());
                return Some(Box::new(opened));
            }
            Err(e) => eprintln!("slot: {}: {e}", path.display()),
        }
    }
    None
}

/// Whether this machine's core has to be told its pads carry six buttons.
///
/// Only the Mega Drive: genesis_plus_gx emulates a 3-button pad unless the ROM
/// header advertises the 6-button one, and then X/Y/Z — two of them on the face
/// buttons, X/Z on the shoulders — silently do nothing.
pub fn six_button_pads(platform: Platform) -> bool {
    platform == Platform::Md
}

pub fn palette_for(
    chosen: Option<GbPalette>,
    platform: Platform,
    rom: &Path,
    link: Option<u8>,
) -> Option<GbPalette> {
    // Only the Game Boys have a DMG palette to force. Naming them rather than
    // excluding the Advance keeps a `.nes` from being opened to read a header
    // byte at 0x143 that means nothing to it.
    chosen.filter(|_| {
        link.is_none() && matches!(platform, Platform::Gb | Platform::Gbc) && dmg_only(rom)
    })
}

pub fn dmg_only(rom: &Path) -> bool {
    use std::io::Read;
    let mut header = [0u8; 0x150];
    std::fs::File::open(rom)
        .and_then(|mut f| f.read_exact(&mut header))
        .is_ok_and(|_| header[0x143] & 0x80 == 0)
}

fn report_missing(core: Core, paths: &[PathBuf]) {
    eprintln!(
        "slot: no {} core found, running the mock test pattern instead. Looked in: {}",
        core.as_str(),
        paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

pub fn colour_option(which: Core, on: bool) -> Option<(&'static str, &'static str)> {
    which
        .spec()
        .colour
        .map(|c| (c.key, if on { c.on } else { c.off }))
}

/// armGBA 的 netpacket 联机选项：必须在 `load` 前设好。核心的 Link Mode /
/// GBA Link Port 都是“重启生效”，改了会内部重启主机。
pub fn armgba_link_options(wireless: bool) -> [(&'static str, &'static str); 3] {
    [
        ("armgba_link_mode", "separate"),
        ("armgba_link_players", "2"),
        (
            "armgba_link_port",
            if wireless { "wireless" } else { "cable" },
        ),
    ]
}

pub fn apply_link_options(core: &mut LibretroCore, which: Core, link: LinkSetup) {
    match link {
        LinkSetup::Mgba { player } => {
            // 目前只有 mGBA 声明了这条电缆选项；能力位来自核心表。
            if !which.spec().link {
                return;
            }
            core.set_option("mgba_link", "on");
            core.set_option("mgba_link_player", &player.to_string());
            eprintln!("slot: core: link mode on, player {player}");
        }
        LinkSetup::Armgba { wireless } => {
            if which != Core::Armgba {
                return;
            }
            for (key, value) in armgba_link_options(wireless) {
                core.set_option(key, value);
            }
            eprintln!(
                "slot: core: armGBA link separate, {} port",
                if wireless { "wireless" } else { "cable" }
            );
        }
    }
}

pub fn apply_core_options(
    core: &mut LibretroCore,
    which: Core,
    serial: &str,
    bios: bool,
    colour: bool,
    palette: Option<GbPalette>,
) {
    let spec = which.spec();

    // 只有声明了该选项的核心才收到 `<core>_frameskip = auto`；未声明的核心
    // 会对未知键告警。能力位来自核心表。
    if spec.frameskip {
        core.set_option(&format!("{}_frameskip", spec.dylib), "auto");
    }

    // 与上下文无关的固定选项（overscan 像素数、turbo 开关、渲染场等）。
    for (key, value) in spec.fixed_options {
        core.set_option(key, value);
    }

    // 依赖运行时上下文的选项：调色板、序列号、BIOS 启动模式、颜色校正。
    match which {
        Core::Mgba => {
            match palette {
                Some(p) => {
                    core.set_option("mgba_gb_model", "Game Boy");
                    core.set_option("mgba_gb_colors_preset", "0");
                    core.set_option("mgba_gb_colors", p.core_name());
                }
                None => {
                    core.set_option("mgba_gb_model", "Autodetect");
                    core.set_option("mgba_gb_colors_preset", "1");
                    core.set_option("mgba_gb_colors", "GBC Dark Green →A");
                }
            }
            if let Some((key, value)) = colour_option(which, colour) {
                core.set_option(key, value);
            }
        }
        Core::Gpsp => {
            core.set_option("gpsp_serial", serial);
            if bios {
                core.set_option("gpsp_boot_mode", "bios");
            }
            if let Some((key, value)) = colour_option(which, colour) {
                core.set_option(key, value);
            }
        }
        Core::Armgba => {
            // armGBA 默认从 system 目录找 gba_bios.bin（linkslot 的 BIOS 目录
            // 正好是它要的位置）；找不到时它回退到内置 BIOS。
            if let Some((key, value)) = colour_option(which, colour) {
                core.set_option(key, value);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn only_the_mega_drive_asks_for_six_button_pads() {
        assert!(six_button_pads(Platform::Md));
        for platform in [
            Platform::Gba,
            Platform::Gb,
            Platform::Gbc,
            Platform::Fc,
            Platform::Sfc,
        ] {
            assert!(
                !six_button_pads(platform),
                "{platform:?} asked its core for six-button pads"
            );
        }
    }

    #[test]
    fn each_core_spells_colour_correction_its_own_way() {
        assert_eq!(
            colour_option(Core::Mgba, true),
            Some(("mgba_color_correction", "Auto"))
        );
        assert_eq!(
            colour_option(Core::Mgba, false),
            Some(("mgba_color_correction", "OFF"))
        );
        assert_eq!(
            colour_option(Core::Gpsp, true),
            Some(("gpsp_color_correction", "enabled"))
        );
        assert_eq!(
            colour_option(Core::Gpsp, false),
            Some(("gpsp_color_correction", "disabled"))
        );
        assert_eq!(
            colour_option(Core::Armgba, true),
            Some(("armgba_color_correction", "enabled"))
        );
        assert_eq!(
            colour_option(Core::Armgba, false),
            Some(("armgba_color_correction", "disabled"))
        );
    }

    #[test]
    fn armgba_opens_a_two_player_separate_link_on_the_chosen_port() {
        assert_eq!(
            armgba_link_options(false),
            [
                ("armgba_link_mode", "separate"),
                ("armgba_link_players", "2"),
                ("armgba_link_port", "cable"),
            ]
        );
        assert_eq!(
            armgba_link_options(true),
            [
                ("armgba_link_mode", "separate"),
                ("armgba_link_players", "2"),
                ("armgba_link_port", "wireless"),
            ]
        );
    }

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn candidates_search_the_named_cores_own_filename_only() {
        let _g = lock();
        std::env::remove_var(CORE_ENV);

        let root = Path::new("/root");
        let mgba = candidates(root, Core::Mgba);
        let gpsp = candidates(root, Core::Gpsp);
        assert_ne!(mgba, gpsp, "the two cores searched the same paths");
        assert!(!mgba.is_empty());
        assert!(!gpsp.is_empty());
        for path in &mgba {
            let name = path.file_name().unwrap().to_str().unwrap();
            assert!(name.starts_with("mgba_libretro"), "{name} is not mGBA's");
        }
        for path in &gpsp {
            let name = path.file_name().unwrap().to_str().unwrap();
            assert!(name.starts_with("gpsp_libretro"), "{name} is not gpSP's");
        }
    }

    #[test]
    fn candidates_search_the_roots_own_system_directory() {
        let _g = lock();
        std::env::remove_var(CORE_ENV);
        let root = Path::new("/some/content/root");
        assert_eq!(
            candidates(root, Core::Gpsp)[0],
            root.join(crate::root::SYSTEM_CORES)
                .join("gpsp")
                .join(dylib_name(Core::Gpsp)),
        );
    }

    #[test]
    fn the_env_override_ignores_which_core_was_asked_for() {
        let _g = lock();
        std::env::set_var(CORE_ENV, "/dev/null/named-core");
        let root = Path::new("/root");
        let mgba = candidates(root, Core::Mgba);
        let gpsp = candidates(root, Core::Gpsp);
        std::env::remove_var(CORE_ENV);
        assert_eq!(mgba, vec![PathBuf::from("/dev/null/named-core")]);
        assert_eq!(
            mgba, gpsp,
            "the override stopped winning for one of the cores"
        );
    }
}
