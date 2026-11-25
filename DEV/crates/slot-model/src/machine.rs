//! 机种表：`Platform` 枚举 + 8 条 `Machine` 数据。
//!
//! 设计约束：
//! - `Platform` 保留**枚举**（而不是 newtype），这样 `match` 仍然能被编译器检查完整性；
//!   数据统一从 [`MACHINES`] 取，新增机种只需「加变体 + 加一条表项」。
//! - `MACHINES` 的顺序就是历史落盘槽序，**只能追加**（`slot.state` 的旧格式依赖它）。
//! - `Platform::ALL` 与 `MACHINES` 必须一一对应，由 `tests/registry.rs` 守住。
//! - `Platform::BUILTIN` 是「出厂就在卡上、永远是货架」的前六个机种；
//!   Arcade / Neogeo 虽然也是合法机种，但要靠平台包激活才出现，因此不在 BUILTIN 里。

use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Platform {
    #[default]
    Gba,
    Gb,
    Gbc,
    /// Famicom/NES.
    Fc,
    /// Super Famicom / SNES.
    Sfc,
    /// Mega Drive / Genesis.
    Md,
    /// 街机（FBNeo）。追加在最后：`as u8` 是排序与落盘槽位，不能插队。
    ///
    /// 它**默认不注册**：只有卡上存在合法的 `kind=platform` 包记录时才会被
    /// 扫描（见 `slot-store::platform_packs`），所以枚举里有它不等于货架上有它。
    Arcade,
    /// NeoGeo（同一个 FBNeo 核心，独立目录与独立货架）。
    Neogeo,
}

/// 稳定机种标识：落盘、云端路径、SQLite 里使用的那串大写目录名（如 `GBA`）。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct MachineId(pub &'static str);

impl MachineId {
    pub fn as_str(self) -> &'static str {
        self.0
    }

    /// 小写形式，用于 INI key 等场合（`GBA` → `gba`）。
    pub fn key(self) -> String {
        self.0.to_ascii_lowercase()
    }
}

/// 机种美术资源（v1.0.0 仅登记文件名，P3-15 由加载器消费）。
#[derive(Clone, Copy, Debug)]
pub struct MachineArt {
    pub board: &'static str,
    pub cart: &'static str,
    pub label: &'static str,
    pub slot_back: &'static str,
}

/// 快捷菜单里该机种可用的开关。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QuickSwitch {
    LoadState,
    SaveState,
    Shader,
    Palette,
}

/// 一个机种的全部元数据。
#[derive(Debug)]
pub struct Machine {
    pub id: MachineId,
    /// 卡内目录名，也是 `Games/`、`Saves/`、`States/` 的子目录名。
    pub dir: &'static str,
    /// 英文长名（About / 诊断用）。
    pub name: &'static str,
    pub rom_ext: &'static [&'static str],
    /// 渲染表面尺寸。
    pub canvas: (u32, u32),
    /// 封面几何尺寸（机器的形状，不是画面）。
    pub picture: (u32, u32),
    /// 可运行该机种的核心 id（按推荐顺序）。
    pub cores: &'static [&'static str],
    pub default_core: &'static str,
    pub art: MachineArt,
    /// 叠加层目录名（`Overlay/<name>`），没有则为 None。
    pub overlay: Option<&'static str>,
    pub quick: &'static [QuickSwitch],
    /// 是否参与 GB 调色板。
    pub palette: bool,
}

pub const GBA: Machine = Machine {
    id: MachineId("GBA"),
    dir: "GBA",
    name: "Game Boy Advance",
    rom_ext: &["gba"],
    canvas: (240, 160),
    picture: (240, 160),
    cores: &["mgba", "gpsp"],
    default_core: "mgba",
    art: MachineArt {
        board: "board.svg",
        cart: "gba_cart.png",
        label: "sticker.svg",
        slot_back: "slot_back_gba.png",
    },
    overlay: None,
    quick: &[QuickSwitch::LoadState, QuickSwitch::SaveState, QuickSwitch::Shader],
    palette: false,
};

pub const GB: Machine = Machine {
    id: MachineId("GB"),
    dir: "GB",
    name: "Game Boy",
    rom_ext: &["gb", "gbc"],
    canvas: (240, 160),
    picture: (160, 144),
    cores: &["mgba"],
    default_core: "mgba",
    art: MachineArt {
        board: "board_gb.svg",
        cart: "gb_cart.png",
        label: "sticker.svg",
        slot_back: "slot_back_gb.png",
    },
    overlay: None,
    quick: &[QuickSwitch::LoadState, QuickSwitch::SaveState, QuickSwitch::Palette],
    palette: true,
};

pub const GBC: Machine = Machine {
    id: MachineId("GBC"),
    dir: "GBC",
    name: "Game Boy Color",
    rom_ext: &["gb", "gbc"],
    canvas: (240, 160),
    picture: (160, 144),
    cores: &["mgba"],
    default_core: "mgba",
    art: MachineArt {
        board: "board_gbc.svg",
        cart: "gbc_cart.png",
        label: "sticker.svg",
        slot_back: "slot_back_gbc.png",
    },
    overlay: None,
    quick: &[QuickSwitch::LoadState, QuickSwitch::SaveState, QuickSwitch::Palette],
    palette: true,
};

pub const FC: Machine = Machine {
    id: MachineId("FC"),
    dir: "FC",
    name: "Famicom",
    rom_ext: &["nes"],
    canvas: (256, 224),
    picture: (256, 224),
    cores: &["fceumm"],
    default_core: "fceumm",
    art: MachineArt {
        board: "board_fc.svg",
        cart: "fc_cart.png",
        label: "sticker.svg",
        slot_back: "slot_back_fc.png",
    },
    overlay: Some("fc"),
    quick: &[QuickSwitch::LoadState, QuickSwitch::SaveState],
    palette: false,
};

pub const SFC: Machine = Machine {
    id: MachineId("SFC"),
    dir: "SFC",
    name: "Super Famicom",
    rom_ext: &["sfc", "smc", "fig"],
    canvas: (256, 224),
    picture: (256, 224),
    cores: &["snes9x", "supafaust"],
    default_core: "snes9x",
    art: MachineArt {
        board: "board.svg",
        cart: "sfc_cart.png",
        label: "sticker.svg",
        slot_back: "slot_back_sfc.png",
    },
    overlay: None,
    quick: &[QuickSwitch::LoadState, QuickSwitch::SaveState],
    palette: false,
};

pub const MD: Machine = Machine {
    id: MachineId("MD"),
    dir: "MD",
    name: "Mega Drive",
    rom_ext: &["md", "gen", "smd", "bin"],
    canvas: (320, 224),
    picture: (320, 224),
    cores: &["genesis_plus_gx"],
    default_core: "genesis_plus_gx",
    art: MachineArt {
        board: "board_md.svg",
        cart: "md_cart.png",
        label: "sticker.svg",
        slot_back: "slot_back_md.png",
    },
    overlay: None,
    quick: &[QuickSwitch::LoadState, QuickSwitch::SaveState],
    palette: false,
};

/// 街机主板。没有卡带、没有壳色：一个 `Games/Arcade/*.zip` 就是一套 ROM 集。
///
/// 美术资源暂时沿用现有条目（`board.svg` / `gba_cart.png` / `slot_back_gba.png`）：
/// 加载器会按这些名字去卡上找文件，空字符串只会拼出一个空路径。Phase 4 换成
/// 真正的街机 PCB 与柜子贴图。
pub const ARCADE: Machine = Machine {
    id: MachineId("ARCADE"),
    dir: "Arcade",
    name: "Arcade",
    rom_ext: &["zip"],
    canvas: (384, 224),
    // PCB 贴纸的尺寸；Phase 4 换成街机柜子的画面框。
    picture: (240, 163),
    cores: &["fbneo"],
    default_core: "fbneo",
    art: MachineArt {
        board: "board.svg",
        cart: "gba_cart.png",
        label: "sticker.svg",
        slot_back: "slot_back_gba.png",
    },
    overlay: None,
    quick: &[],
    palette: false,
};

/// NeoGeo。与街机共用 FBNeo，但目录、封面与落盘槽位都独立。
pub const NEOGEO: Machine = Machine {
    id: MachineId("NEOGEO"),
    dir: "NeoGeo",
    name: "NeoGeo",
    rom_ext: &["zip"],
    canvas: (320, 224),
    picture: (240, 163),
    cores: &["fbneo"],
    default_core: "fbneo",
    art: MachineArt {
        board: "board.svg",
        cart: "gba_cart.png",
        label: "sticker.svg",
        slot_back: "slot_back_gba.png",
    },
    overlay: None,
    quick: &[],
    palette: false,
};

/// 机种表。顺序即落盘槽序，**只能追加**。
pub const MACHINES: &[Machine] = &[GBA, GB, GBC, FC, SFC, MD, ARCADE, NEOGEO];

impl Platform {
    /// 顺序必须与 [`MACHINES`] 一致；两者由 `tests/registry.rs` 校验。
    pub const ALL: [Platform; 8] = [
        Platform::Gba,
        Platform::Gb,
        Platform::Gbc,
        Platform::Fc,
        Platform::Sfc,
        Platform::Md,
        Platform::Arcade,
        Platform::Neogeo,
    ];

    /// 出厂即可用的机种（卡上无需任何扩展包）。
    ///
    /// 货架、召回、Web 库这些"永远在"的地方用这个列表；`ALL` 是"编译器要
    /// 穷尽"的完整机种表，包含需要平台包激活的 Arcade / Neogeo。
    pub const BUILTIN: [Platform; 6] = [
        Platform::Gba,
        Platform::Gb,
        Platform::Gbc,
        Platform::Fc,
        Platform::Sfc,
        Platform::Md,
    ];

    pub fn machine(self) -> &'static Machine {
        match self {
            Platform::Gba => &GBA,
            Platform::Gb => &GB,
            Platform::Gbc => &GBC,
            Platform::Fc => &FC,
            Platform::Sfc => &SFC,
            Platform::Md => &MD,
            Platform::Arcade => &ARCADE,
            Platform::Neogeo => &NEOGEO,
        }
    }

    /// 在 [`MACHINES`] 中的位置；历史 `slot.state` 用它做数组下标，不可重排。
    pub fn index(self) -> usize {
        let id = self.machine().id;
        MACHINES
            .iter()
            .position(|m| m.id == id)
            .expect("every Platform must appear in MACHINES")
    }

    pub fn name(self) -> &'static str {
        self.machine().name
    }

    pub fn dir_name(self) -> &'static str {
        self.machine().dir
    }

    pub fn extensions(self) -> &'static [&'static str] {
        self.machine().rom_ext
    }

    /// 封面几何。
    pub fn picture(self) -> (u32, u32) {
        self.machine().picture
    }

    /// 核心帧画布。
    pub fn frame(self) -> (u32, u32) {
        self.machine().canvas
    }

    pub fn accepts(self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                self.extensions()
                    .iter()
                    .any(|k| ext.eq_ignore_ascii_case(k))
            })
    }
}

pub fn machine(platform: Platform) -> &'static Machine {
    platform.machine()
}

pub fn by_id(id: MachineId) -> Option<&'static Machine> {
    MACHINES.iter().find(|m| m.id == id)
}

/// 按目录名查机种（大小写无关）。
pub fn by_dir(dir: &str) -> Option<&'static Machine> {
    MACHINES
        .iter()
        .find(|m| m.dir.eq_ignore_ascii_case(dir))
}

/// 所有机种的 ROM 扩展名（去重后按表顺序）。
pub fn all_rom_ext() -> impl Iterator<Item = &'static str> {
    let mut seen: Vec<&'static str> = Vec::new();
    for m in MACHINES {
        for ext in m.rom_ext {
            if !seen.contains(ext) {
                seen.push(ext);
            }
        }
    }
    seen.into_iter()
}
