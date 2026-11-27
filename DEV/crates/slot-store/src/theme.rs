use std::path::Path;

pub const THEME_FILE: &str = "theme.txt";

/// The two built-in cabinets.  Old theme files are still accepted through
/// [`ThemePreset::parse`]; their colours are remapped onto one of these.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ThemePreset {
    /// Warm charcoal, paper cards, brass accents.  The only shipped theme.
    #[default]
    Warm,
    /// Kept as a data fallback for old theme files; mapped to Warm.
    Paper,
}

impl ThemePreset {
    pub const ALL: [ThemePreset; 2] = [Self::Warm, Self::Paper];

    pub fn label(self) -> &'static str {
        match self {
            Self::Warm => "掌机平面",
            Self::Paper => "纸本",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Warm => "warm",
            Self::Paper => "paper",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            // One shipped theme: every historic name lands on Warm.
            "paper" | "ivory" | "light" | "default" | "web" | "teal" | "cyan"
            | "warm" | "dark" | "retro" | "amber" => Some(Self::Warm),
            _ => None,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Warm => 0,
            Self::Paper => 1,
        }
    }

    pub fn at(index: usize) -> Self {
        Self::ALL[index.min(Self::ALL.len() - 1)]
    }

    pub fn next(self) -> Self {
        Self::at((self.index() + 1) % Self::ALL.len())
    }

    pub fn prev(self) -> Self {
        Self::at((self.index() + Self::ALL.len() - 1) % Self::ALL.len())
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Theme {
    pub preset: ThemePreset,
    // The cartridge slot / chassis.
    pub housing: [u8; 3],
    pub recess: [u8; 3],
    pub opening: [u8; 3],
    pub edge: [u8; 3],
    // Shared UI palette.
    pub bg: [u8; 4],
    pub bg_top: [u8; 4],
    pub surface: [u8; 4],
    pub surface_border: [u8; 4],
    pub selected: [u8; 4],
    pub selected_border: [u8; 4],
    pub ink: [u8; 3],
    pub ink_selected: [u8; 3],
    pub ink_dim: [u8; 3],
    pub accent: [u8; 3],
    pub accent_dim: [u8; 3],
    // Document layer: the same paper in both cabinets, with a red stamp that
    // is deliberately not the primary brand colour.
    pub paper: [u8; 4],
    pub paper_ink: [u8; 3],
    pub paper_muted: [u8; 3],
    pub paper_edge: [u8; 4],
    pub stamp: [u8; 3],
    /// Metallic highlight for stars and small marks.  On paper this is the
    /// darker brass that stays readable; on warm ink it is the bright brass.
    pub gold: [u8; 3],
    pub bar: [u8; 4],
    pub bar_edge: [u8; 4],
}

impl Default for Theme {
    fn default() -> Self {
        Self::preset(ThemePreset::default())
    }
}

impl Theme {
    pub fn preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::Warm => Theme {
                preset,
                housing: [0x16, 0x16, 0x19],
                recess: [0x26, 0x26, 0x2b],
                opening: [0x08, 0x08, 0x0a],
                edge: [0x3a, 0x3a, 0x42],
                bg: [0x1b, 0x1b, 0x1e, 0xff],
                bg_top: [0x1b, 0x1b, 0x1e, 0xff],
                surface: [0x26, 0x26, 0x2b, 0xff],
                surface_border: [0x3a, 0x3a, 0x42, 0xff],
                selected: [0xf6, 0xf6, 0xf3, 0xff],
                selected_border: [0xf6, 0xf6, 0xf3, 0xff],
                ink: [0xf4, 0xf4, 0xf1],
                ink_selected: [0x1b, 0x1b, 0x1e],
                ink_dim: [0xa6, 0xa6, 0xad],
                accent: [0xc4, 0x3b, 0x32],
                accent_dim: [0x8c, 0x2a, 0x24],
                paper: [0xf0, 0xe8, 0xdb, 0xff],
                paper_ink: [0x1f, 0x1f, 0x22],
                paper_muted: [0x7a, 0x6a, 0x5a],
                paper_edge: [0xd8, 0xc9, 0xb5, 0xff],
                stamp: [0xc4, 0x3b, 0x32],
                gold: [0xc4, 0x3b, 0x32],
                bar: [0x1b, 0x1b, 0x1e, 0xff],
                bar_edge: [0x3a, 0x3a, 0x42, 0xff],
            },
            ThemePreset::Paper => Theme {
                preset,
                housing: [0x2a, 0x20, 0x18],
                recess: [0x1b, 0x15, 0x10],
                opening: [0x0c, 0x0a, 0x08],
                edge: [0xb9, 0xa9, 0x8e],
                bg: [0xef, 0xe9, 0xdc, 0xff],
                bg_top: [0xf8, 0xf3, 0xe7, 0xff],
                surface: [0xf8, 0xf3, 0xe7, 0xff],
                surface_border: [0xd3, 0xc6, 0xb0, 0xff],
                selected: [0x24, 0x1f, 0x1a, 0xff],
                selected_border: [0x3a, 0x2e, 0x24, 0xff],
                ink: [0x24, 0x1f, 0x1a],
                ink_selected: [0xf8, 0xf3, 0xe7],
                ink_dim: [0x6e, 0x63, 0x54],
                accent: [0x7a, 0x56, 0x20],
                accent_dim: [0x5e, 0x42, 0x18],
                paper: [0xf8, 0xf3, 0xe7, 0xff],
                paper_ink: [0x24, 0x1f, 0x1a],
                paper_muted: [0x6e, 0x63, 0x54],
                paper_edge: [0xd3, 0xc6, 0xb0, 0xff],
                stamp: [0xa8, 0x34, 0x2f],
                gold: [0x9a, 0x74, 0x34],
                bar: [0xef, 0xe9, 0xdc, 0xf2],
                bar_edge: [0xd3, 0xc6, 0xb0, 0xff],
            },
        }
    }

    pub fn read(root: &Path) -> Self {
        match std::fs::read_to_string(root.join("linkSlot/Config").join(THEME_FILE)) {
            Ok(text) => Self::parse(&text),
            Err(_) => Theme::default(),
        }
    }

    pub fn write_preset(root: &Path, preset: ThemePreset) -> std::io::Result<()> {
        let dir = root.join("linkSlot/Config");
        std::fs::create_dir_all(&dir)?;
        std::fs::write(
            dir.join(THEME_FILE),
            format!("preset {}\n", preset.as_str()),
        )
    }

    pub fn parse(text: &str) -> Self {
        let mut preset = ThemePreset::default();
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            let mut parts = line.split_whitespace();
            if let (Some(name), Some(value)) = (parts.next(), parts.next()) {
                if parts.next().is_none()
                    && matches!(name.to_ascii_lowercase().as_str(), "preset" | "theme")
                {
                    if let Some(found) = ThemePreset::parse(value) {
                        preset = found;
                    }
                }
            }
        }
        let mut theme = Theme::preset(preset);
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            let mut parts = line.split_whitespace();
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                continue;
            };
            if parts.next().is_some() {
                continue;
            }
            let Some(rgb) = hex(value) else {
                continue;
            };
            match name.to_ascii_lowercase().as_str() {
                "housing" => theme.housing = rgb,
                "recess" => theme.recess = rgb,
                "opening" => theme.opening = rgb,
                "edge" => theme.edge = rgb,
                "bg" => theme.bg = rgba(rgb),
                "bg_top" => theme.bg_top = rgba(rgb),
                "surface" => theme.surface = rgba(rgb),
                "surface_border" => theme.surface_border = rgba(rgb),
                "selected" => theme.selected = rgba(rgb),
                "selected_border" => theme.selected_border = rgba(rgb),
                "ink" => theme.ink = rgb,
                "ink_selected" => theme.ink_selected = rgb,
                "ink_dim" => theme.ink_dim = rgb,
                "accent" => theme.accent = rgb,
                "accent_dim" => theme.accent_dim = rgb,
                "paper" => theme.paper = rgba(rgb),
                "paper_ink" => theme.paper_ink = rgb,
                "paper_muted" => theme.paper_muted = rgb,
                "paper_edge" => theme.paper_edge = rgba(rgb),
                "stamp" => theme.stamp = rgb,
                "gold" => theme.gold = rgb,
                "bar" => theme.bar = rgba(rgb),
                "bar_edge" => theme.bar_edge = rgba(rgb),
                _ => {}
            }
        }
        theme
    }
}

fn rgba(rgb: [u8; 3]) -> [u8; 4] {
    [rgb[0], rgb[1], rgb[2], 0xff]
}

fn hex(value: &str) -> Option<[u8; 3]> {
    let digits = value.strip_prefix('#').unwrap_or(value);
    if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}
