#![no_std]

use slot_i18n::Language;
use slot_model::Platform;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ResumeAction {
    Load,
    Save,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum QuickRow {
    FastForward,
    FastForwardSound,
    Screen,
    Rumble,
    DateTime,
    Mods,
    About,
    SystemUpgrade,
    GbaShader,
    GbShader,
    ColourCorrection,
    Brightness,
    BlueLight,
    Game,
    SaveSettings,
    LoadGba,
    SaveGba,
    LoadGb,
    SaveGb,
    LoadGbc,
    SaveGbc,
    Recents,
    Turbo,
    Rewind,
    GbPalettes,
    ButtonLayout,
    RomTitle,
    FallbackCover,
    LidTimeout,
    ButtonSettings,
    SystemSettings,
    Shortcuts,
    Network,
    Wifi,
    Web,
    SyncStorage,
    ShelfKeys,
    ShelfPrev,
    ShelfNext,
    ShelfFav,
    ShortcutMap,
    ShortcutSaveState,
    ShortcutLoadState,
    ShortcutBrightnessUp,
    ShortcutBrightnessDown,
    ShortcutBlueLightDown,
    ShortcutBlueLightUp,
    ShortcutColorCorrection,
    ShortcutPaletteBack,
    ShortcutPaletteNext,
    ShortcutShaderNext,
    ShortcutShaderBack,
    ShortcutFastForward,
    ShortcutRewind,
    ControlUp,
    ControlDown,
    ControlLeft,
    ControlRight,
    ControlA,
    ControlB,
    ControlSelect,
    ControlStart,
    ControlL,
    ControlR,
    ControlTurboA,
    ControlTurboB,
    ControlPreset,
    SyncContent,
    /// 网络设置 → 卡带传输模式.
    CartTransfer,
    /// The shortcut page's last row: put every shortcut back to its default.
    ShortcutReset,
    /// 画面 → 屏幕与亮度 (a second-level page).
    DisplayPanel,
    /// 画面 → 画面效果 (a second-level page).
    DisplayEffects,
    /// 卡带架: the shelf's own settings (a top-level page).
    Shelf,
    /// 卡带架 → 外观与显示 (a second-level page).
    ShelfLook,
    /// 存档设置 top: load resume.state by default on insert.
    DefaultLoad,
    /// 存档设置 top: save resume.state by default on exit.
    DefaultSave,
    /// 画面效果 -> FC: draw the screen overlay from Overlay/FC/*.png.
    FcMask,
    /// 画面效果 -> GB: draw the screen overlay from Overlay/GB/*.png.
    GbMask,
    /// 画面效果 -> GBC: draw the screen overlay from Overlay/GBC/*.png.
    GbcMask,
    /// 画面效果 -> SFC: draw the screen overlay from Overlay/SFC/*.png.
    SfcMask,
    /// 画面 -> 遮罩: the page that holds the screen overlays.
    Masks,
    /// 外观与显示 → 主题: opens the theme list.
    Theme,
    /// 主题 → 默认深色.
    ThemeDefault,
    /// 主题 → 网络青.
    ThemeWeb,
    /// 主题 → 复古暖色.
    ThemeWarm,
    /// 系统 → 语言: the UI language, left/right to switch.
    ///
    /// Appended after every row that already existed, so no stored value or
    /// test index moves.  It sits before `SplitLink` because `SplitLink` stays
    /// the one row outside `QuickRow::ALL`, and `index()` is the enum's
    /// discriminant: keeping `SplitLink` last keeps `Language` inside the value
    /// table.
    HotspotBand,
    /// The system language row.  It stays last in QuickRow::ALL.
    Language,
    /// The link session page: who is on the other end, and start/stop.  This
    /// row is intentionally absent from QuickRow::ALL.
    SplitLink,
}

impl QuickRow {
    pub const ALL: [QuickRow; 87] = [
        QuickRow::FastForward,
        QuickRow::FastForwardSound,
        QuickRow::Screen,
        QuickRow::Rumble,
        QuickRow::DateTime,
        QuickRow::Mods,
        QuickRow::About,
        QuickRow::SystemUpgrade,
        QuickRow::GbaShader,
        QuickRow::GbShader,
        QuickRow::ColourCorrection,
        QuickRow::Brightness,
        QuickRow::BlueLight,
        QuickRow::Game,
        QuickRow::SaveSettings,
        QuickRow::LoadGba,
        QuickRow::SaveGba,
        QuickRow::LoadGb,
        QuickRow::SaveGb,
        QuickRow::LoadGbc,
        QuickRow::SaveGbc,
        QuickRow::Recents,
        QuickRow::Turbo,
        QuickRow::Rewind,
        QuickRow::GbPalettes,
        QuickRow::ButtonLayout,
        QuickRow::RomTitle,
        QuickRow::FallbackCover,
        QuickRow::LidTimeout,
        QuickRow::ButtonSettings,
        QuickRow::SystemSettings,
        QuickRow::Shortcuts,
        QuickRow::Network,
        QuickRow::Wifi,
        QuickRow::Web,
        QuickRow::SyncStorage,
        QuickRow::ShelfKeys,
        QuickRow::ShelfPrev,
        QuickRow::ShelfNext,
        QuickRow::ShelfFav,
        QuickRow::ShortcutMap,
        QuickRow::ShortcutSaveState,
        QuickRow::ShortcutLoadState,
        QuickRow::ShortcutBrightnessUp,
        QuickRow::ShortcutBrightnessDown,
        QuickRow::ShortcutBlueLightDown,
        QuickRow::ShortcutBlueLightUp,
        QuickRow::ShortcutColorCorrection,
        QuickRow::ShortcutPaletteBack,
        QuickRow::ShortcutPaletteNext,
        QuickRow::ShortcutShaderNext,
        QuickRow::ShortcutShaderBack,
        QuickRow::ShortcutFastForward,
        QuickRow::ShortcutRewind,
        QuickRow::ControlUp,
        QuickRow::ControlDown,
        QuickRow::ControlLeft,
        QuickRow::ControlRight,
        QuickRow::ControlA,
        QuickRow::ControlB,
        QuickRow::ControlSelect,
        QuickRow::ControlStart,
        QuickRow::ControlL,
        QuickRow::ControlR,
        QuickRow::ControlTurboA,
        QuickRow::ControlTurboB,
        QuickRow::ControlPreset,
        QuickRow::SyncContent,
        QuickRow::CartTransfer,
        QuickRow::ShortcutReset,
        QuickRow::DisplayPanel,
        QuickRow::DisplayEffects,
        QuickRow::Shelf,
        QuickRow::ShelfLook,
        QuickRow::DefaultLoad,
        QuickRow::DefaultSave,
        QuickRow::FcMask,
        QuickRow::GbMask,
        QuickRow::GbcMask,
        QuickRow::SfcMask,
        QuickRow::Masks,
        QuickRow::Theme,
        QuickRow::ThemeDefault,
        QuickRow::ThemeWeb,
        QuickRow::ThemeWarm,
        QuickRow::HotspotBand,
        QuickRow::Language,
    ];

    pub const MAIN: [QuickRow; 6] = [
        QuickRow::Screen,
        QuickRow::Shelf,
        QuickRow::Game,
        QuickRow::ButtonSettings,
        QuickRow::Network,
        QuickRow::SystemSettings,
    ];

    pub const DISPLAY: [QuickRow; 4] = [
        QuickRow::DisplayPanel,
        QuickRow::DisplayEffects,
        QuickRow::Masks,
        // 主题 lives under 画面: it is a look, like the masks and shaders.
        QuickRow::Theme,
    ];

    pub const DISPLAY_PANEL: [QuickRow; 2] = [QuickRow::Brightness, QuickRow::BlueLight];

    pub const DISPLAY_EFFECTS: [QuickRow; 4] = [
        // Grouped by machine: GBA first, then the Game Boy family, then the
        // one row that applies everywhere.
        QuickRow::GbaShader,
        QuickRow::GbShader,
        QuickRow::GbPalettes,
        QuickRow::ColourCorrection,
    ];

    pub const MASKS: [QuickRow; 4] = [
        QuickRow::FcMask,
        QuickRow::GbMask,
        QuickRow::GbcMask,
        QuickRow::SfcMask,
    ];

    pub const THEMES: [QuickRow; 3] = [
        QuickRow::ThemeDefault,
        QuickRow::ThemeWeb,
        QuickRow::ThemeWarm,
    ];

    pub const SHELF: [QuickRow; 2] = [QuickRow::ShelfLook, QuickRow::ShelfKeys];

    pub const SHELF_LOOK: [QuickRow; 3] = [
        QuickRow::RomTitle,
        QuickRow::FallbackCover,
        QuickRow::Recents,
    ];

    pub const BUTTON_SETTINGS: [QuickRow; 4] = [
        QuickRow::Shortcuts,
        QuickRow::ShortcutMap,
        QuickRow::ButtonLayout,
        // Turbo is a pad mapping switch, so it lives with the pad remap.
        QuickRow::Turbo,
    ];

    /// Order matters: 系统升级 sits right above 关于 at the bottom.
    /// 语言 comes second so it is one press below the date, the row people
    /// reach for first when the screen is in a language they cannot read.
    pub const SYSTEM_SETTINGS: [QuickRow; 6] = [
        QuickRow::DateTime,
        QuickRow::Language,
        QuickRow::LidTimeout,
        QuickRow::Mods,
        QuickRow::SystemUpgrade,
        QuickRow::About,
    ];

    pub const NETWORK: [QuickRow; 6] = [
        QuickRow::Wifi,
        QuickRow::Web,
        QuickRow::SyncStorage,
        QuickRow::SyncContent,
        QuickRow::CartTransfer,
        QuickRow::HotspotBand,
    ];

    pub const SHELF_KEYS: [QuickRow; 3] = [
        QuickRow::ShelfPrev,
        QuickRow::ShelfNext,
        QuickRow::ShelfFav,
    ];

    pub const SHORTCUTS: [QuickRow; 14] = [
        QuickRow::ShortcutSaveState,
        QuickRow::ShortcutLoadState,
        QuickRow::ShortcutBrightnessUp,
        QuickRow::ShortcutBrightnessDown,
        QuickRow::ShortcutBlueLightDown,
        QuickRow::ShortcutBlueLightUp,
        QuickRow::ShortcutColorCorrection,
        QuickRow::ShortcutPaletteBack,
        QuickRow::ShortcutPaletteNext,
        QuickRow::ShortcutShaderNext,
        QuickRow::ShortcutShaderBack,
        QuickRow::ShortcutFastForward,
        QuickRow::ShortcutRewind,
        QuickRow::ShortcutReset,
    ];

    pub const CONTROLS: [QuickRow; 13] = [
        QuickRow::ControlUp,
        QuickRow::ControlDown,
        QuickRow::ControlLeft,
        QuickRow::ControlRight,
        QuickRow::ControlA,
        QuickRow::ControlB,
        QuickRow::ControlSelect,
        QuickRow::ControlStart,
        QuickRow::ControlL,
        QuickRow::ControlR,
        QuickRow::ControlTurboA,
        QuickRow::ControlTurboB,
        QuickRow::ControlPreset,
    ];

    pub const GAME: [QuickRow; 5] = [
        QuickRow::FastForward,
        QuickRow::FastForwardSound,
        QuickRow::Rewind,
        QuickRow::Rumble,
        QuickRow::SaveSettings,
    ];

    pub const SAVES: [QuickRow; 8] = [
        QuickRow::DefaultLoad,
        QuickRow::DefaultSave,
        QuickRow::LoadGba,
        QuickRow::SaveGba,
        QuickRow::LoadGb,
        QuickRow::SaveGb,
        QuickRow::LoadGbc,
        QuickRow::SaveGbc,
    ];

    /// Translation keys for the page titles, indexed by [`QuickRow::page_index`].
    ///
    /// Stored as keys rather than titles: a `&'static [&'static str]` cannot hold
    /// two languages, and one constant per language would duplicate the table the
    /// parity test exists to keep single.
    const PAGE_KEYS: [&'static str; 16] = [
    "device.quick.title.settings",
    "device.quick.title.display",
    "device.quick.title.screen_light",
    "device.quick.title.picture_effects",
    "device.quick.title.masks",
    "device.quick.title.shelf",
    "device.quick.title.shelf_look",
    "device.quick.title.shelf_keys",
    "device.quick.title.game",
    "device.quick.title.controls",
    "device.quick.title.shortcut_map",
    "device.quick.title.button_layout",
    "device.quick.title.network",
    "device.quick.title.system",
    "device.quick.title.save_settings",
    "device.quick.title.theme",
    ];
    pub const QUICK_PAGES: usize = Self::PAGE_KEYS.len();

    /// The page titles in `lang`, indexed like [`QuickRow::page_index`].
    pub fn pages(lang: Language) -> [&'static str; Self::QUICK_PAGES] {
        Self::PAGE_KEYS.map(|key| slot_i18n::t(lang, key))
    }

    /// The title of page `index` in `lang`.
    pub fn page_title_at(index: usize, lang: Language) -> &'static str {
        let index = index.min(Self::QUICK_PAGES - 1);
        slot_i18n::t(lang, Self::PAGE_KEYS[index])
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn page(self) -> &'static [QuickRow] {
        match self {
            QuickRow::DisplayPanel
            | QuickRow::DisplayEffects
            | QuickRow::Masks
            | QuickRow::Theme => &QuickRow::DISPLAY,
            QuickRow::Brightness | QuickRow::BlueLight => &QuickRow::DISPLAY_PANEL,
            QuickRow::GbaShader
            | QuickRow::GbShader
            | QuickRow::ColourCorrection
            | QuickRow::GbPalettes => &QuickRow::DISPLAY_EFFECTS,
            QuickRow::FcMask | QuickRow::GbMask | QuickRow::GbcMask | QuickRow::SfcMask => {
                &QuickRow::MASKS
            }
            QuickRow::ShelfLook | QuickRow::ShelfKeys => &QuickRow::SHELF,
            QuickRow::ThemeDefault | QuickRow::ThemeWeb | QuickRow::ThemeWarm => &QuickRow::THEMES,
            QuickRow::RomTitle | QuickRow::FallbackCover | QuickRow::Recents => {
                &QuickRow::SHELF_LOOK
            }
            QuickRow::FastForward
            | QuickRow::FastForwardSound
            | QuickRow::Rewind
            | QuickRow::Rumble
            | QuickRow::SaveSettings
            => &QuickRow::GAME,
            QuickRow::DefaultLoad
            | QuickRow::DefaultSave
            | QuickRow::LoadGba
            | QuickRow::SaveGba
            | QuickRow::LoadGb
            | QuickRow::SaveGb
            | QuickRow::LoadGbc
            | QuickRow::SaveGbc => &QuickRow::SAVES,
            QuickRow::Wifi
            | QuickRow::Web
            | QuickRow::SyncStorage
            | QuickRow::SyncContent
            | QuickRow::CartTransfer
            | QuickRow::HotspotBand
            | QuickRow::SplitLink => &QuickRow::NETWORK,
            // 按键: shortcuts, the mapping page and the pad remap live together.
            QuickRow::Shortcuts
            | QuickRow::ShortcutMap
            | QuickRow::ButtonLayout
            | QuickRow::Turbo => &QuickRow::BUTTON_SETTINGS,
            QuickRow::DateTime
            | QuickRow::Language
            | QuickRow::LidTimeout
            | QuickRow::Mods
            | QuickRow::SystemUpgrade
            | QuickRow::About => &QuickRow::SYSTEM_SETTINGS,
            QuickRow::ShelfPrev | QuickRow::ShelfNext | QuickRow::ShelfFav => {
                &QuickRow::SHELF_KEYS
            }
            QuickRow::ShortcutSaveState
            | QuickRow::ShortcutLoadState
            | QuickRow::ShortcutBrightnessUp
            | QuickRow::ShortcutBrightnessDown
            | QuickRow::ShortcutBlueLightDown
            | QuickRow::ShortcutBlueLightUp
            | QuickRow::ShortcutColorCorrection
            | QuickRow::ShortcutPaletteBack
            | QuickRow::ShortcutPaletteNext
            | QuickRow::ShortcutShaderNext
            | QuickRow::ShortcutShaderBack
            | QuickRow::ShortcutFastForward
            | QuickRow::ShortcutRewind
            | QuickRow::ShortcutReset => &QuickRow::SHORTCUTS,
            QuickRow::ControlUp
            | QuickRow::ControlDown
            | QuickRow::ControlLeft
            | QuickRow::ControlRight
            | QuickRow::ControlA
            | QuickRow::ControlB
            | QuickRow::ControlSelect
            | QuickRow::ControlStart
            | QuickRow::ControlL
            | QuickRow::ControlR
            | QuickRow::ControlTurboA
            | QuickRow::ControlTurboB
            | QuickRow::ControlPreset => &QuickRow::CONTROLS,
            _ => &QuickRow::MAIN,
        }
    }

    pub fn page_title(self, lang: Language) -> &'static str {
        Self::page_title_at(self.page_index(), lang)
    }
    pub fn page_index(self) -> usize {
        match self.page()[0] {
            QuickRow::DisplayPanel => 1,
            QuickRow::Brightness => 2,
            QuickRow::GbaShader => 3,
            QuickRow::FcMask => 4,
            QuickRow::ShelfLook => 5,
            QuickRow::ThemeDefault => 15,
            QuickRow::RomTitle => 6,
            QuickRow::ShelfPrev => 7,
            QuickRow::FastForward => 8,
            QuickRow::Shortcuts => 9,
            QuickRow::ShortcutSaveState => 10,
            QuickRow::ControlUp => 11,
            QuickRow::Wifi => 12,
            QuickRow::DateTime => 13,
            QuickRow::DefaultLoad => 14,
            _ => 0,
        }
    }

    /// Short label for the fallback icon tile when the symbol font has no
    /// glyph for `glyph()`.
    ///
    /// Takes the language because a single Chinese character is meaningless in
    /// an English menu; WP1 widens this to two-letter English initials, WP0 only
    /// needs the language row itself to read correctly.
    /// Short label for the fallback icon tile when the symbol font has no
    /// glyph for `glyph()`.
    ///
    /// One character per language, so the tile keeps the same weight: a single
    /// Chinese character is meaningless in an English menu, and a full English
    /// word would not fit the tile.
    pub fn icon_text(self, lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => self.icon_text_zh(),
            Language::EnUs => self.icon_text_en(),
        }
    }

    fn icon_text_zh(self) -> &'static str {
        match self {
            QuickRow::Language => "语",
            QuickRow::FastForward => "快",
            QuickRow::FastForwardSound => "音",
            QuickRow::Screen => "画",
            QuickRow::Shelf => "架",
            QuickRow::ShelfLook => "看",
            QuickRow::DisplayPanel => "亮",
            QuickRow::DisplayEffects => "效",
            QuickRow::Rumble => "震",
            QuickRow::DateTime => "时",
            QuickRow::Mods => "M",
            QuickRow::ButtonSettings => "键",
            QuickRow::SystemSettings => "系",
            QuickRow::About => "i",
            QuickRow::SystemUpgrade => "升",
            QuickRow::GbaShader => "色",
            QuickRow::GbShader => "色",
            QuickRow::ColourCorrection => "校",
            QuickRow::Brightness => "亮",
            QuickRow::BlueLight => "蓝",
            QuickRow::Game => "游",
            QuickRow::SaveSettings => "存",
                QuickRow::DefaultLoad => "读",
            QuickRow::LoadGba => "读",
            QuickRow::LoadGb => "读",
            QuickRow::LoadGbc => "读",
            QuickRow::DefaultSave => "存",
            QuickRow::FcMask => "遮",
            QuickRow::GbMask => "遮",
            QuickRow::GbcMask => "遮",
            QuickRow::SfcMask => "遮",
            QuickRow::Masks => "罩",
            QuickRow::SaveGba => "存",
            QuickRow::SaveGb => "存",
            QuickRow::SaveGbc => "存",
            QuickRow::Recents => "近",
            QuickRow::Turbo => "连",
            QuickRow::Rewind => "倒",
            QuickRow::GbPalettes => "调",
            QuickRow::ButtonLayout => "键",
            QuickRow::RomTitle => "题",
            QuickRow::FallbackCover => "图",
            QuickRow::LidTimeout => "盖",
            QuickRow::Shortcuts => "捷",
            QuickRow::Network => "网",
            QuickRow::Wifi => "W",
            QuickRow::Web => "服",
            QuickRow::SyncStorage => "同",
            QuickRow::SyncContent => "云",
            QuickRow::CartTransfer => "传",
            QuickRow::HotspotBand => "段",
            QuickRow::SplitLink => "联",
            QuickRow::Theme => "主",
            QuickRow::ThemeDefault => "色",
            QuickRow::ThemeWeb => "色",
            QuickRow::ThemeWarm => "色",
            QuickRow::ShelfKeys => "架",
            QuickRow::ShelfPrev => "前",
            QuickRow::ShelfNext => "后",
            QuickRow::ShelfFav => "藏",
            QuickRow::ShortcutMap => "映",
            QuickRow::ShortcutReset => "重",
            QuickRow::ShortcutSaveState => "存",
            QuickRow::ShortcutLoadState => "读",
            QuickRow::ShortcutBrightnessUp => "亮",
            QuickRow::ShortcutBrightnessDown => "暗",
            QuickRow::ShortcutBlueLightDown => "蓝",
            QuickRow::ShortcutBlueLightUp => "蓝",
            QuickRow::ShortcutColorCorrection => "校",
            QuickRow::ShortcutPaletteBack => "调",
            QuickRow::ShortcutPaletteNext => "调",
            QuickRow::ShortcutShaderNext => "着",
            QuickRow::ShortcutShaderBack => "着",
            QuickRow::ShortcutFastForward => "快",
            QuickRow::ShortcutRewind => "倒",
            QuickRow::ControlUp => "上",
            QuickRow::ControlDown => "下",
            QuickRow::ControlLeft => "左",
            QuickRow::ControlRight => "右",
            QuickRow::ControlA => "A",
            QuickRow::ControlB => "B",
            QuickRow::ControlSelect => "S",
            QuickRow::ControlStart => "T",
            QuickRow::ControlL => "L",
            QuickRow::ControlR => "R",
            QuickRow::ControlTurboA => "X",
            QuickRow::ControlTurboB => "Y",
            QuickRow::ControlPreset => "肩",
        }
    }

    fn icon_text_en(self) -> &'static str {
        match self {
            QuickRow::Language => "L",
            QuickRow::FastForward => "F",
            QuickRow::FastForwardSound => "S",
            QuickRow::Screen => "D",
            QuickRow::Shelf => "C",
            QuickRow::ShelfLook => "L",
            QuickRow::DisplayPanel => "B",
            QuickRow::DisplayEffects => "E",
            QuickRow::Rumble => "R",
            QuickRow::DateTime => "T",
            QuickRow::Mods => "M",
            QuickRow::ButtonSettings => "K",
            QuickRow::SystemSettings => "S",
            QuickRow::About => "i",
            QuickRow::SystemUpgrade => "U",
            QuickRow::GbaShader => "S",
            QuickRow::GbShader => "S",
            QuickRow::ColourCorrection => "C",
            QuickRow::Brightness => "B",
            QuickRow::BlueLight => "B",
            QuickRow::Game => "G",
            QuickRow::SaveSettings => "S",
                QuickRow::DefaultLoad => "L",
            QuickRow::LoadGba => "L",
            QuickRow::LoadGb => "L",
            QuickRow::LoadGbc => "L",
            QuickRow::DefaultSave => "S",
            QuickRow::FcMask => "M",
            QuickRow::GbMask => "M",
            QuickRow::GbcMask => "M",
            QuickRow::SfcMask => "M",
            QuickRow::Masks => "M",
            QuickRow::SaveGba => "S",
            QuickRow::SaveGb => "S",
            QuickRow::SaveGbc => "S",
            QuickRow::Recents => "R",
            QuickRow::Turbo => "T",
            QuickRow::Rewind => "R",
            QuickRow::GbPalettes => "P",
            QuickRow::ButtonLayout => "B",
            QuickRow::RomTitle => "T",
            QuickRow::FallbackCover => "I",
            QuickRow::LidTimeout => "L",
            QuickRow::Shortcuts => "K",
            QuickRow::Network => "N",
            QuickRow::Wifi => "W",
            QuickRow::Web => "S",
            QuickRow::SyncStorage => "M",
            QuickRow::SyncContent => "C",
            QuickRow::CartTransfer => "T",
            QuickRow::HotspotBand => "B",
            QuickRow::SplitLink => "L",
            QuickRow::Theme => "T",
            QuickRow::ThemeDefault => "P",
            QuickRow::ThemeWeb => "P",
            QuickRow::ThemeWarm => "P",
            QuickRow::ShelfKeys => "K",
            QuickRow::ShelfPrev => "<",
            QuickRow::ShelfNext => ">",
            QuickRow::ShelfFav => "F",
            QuickRow::ShortcutMap => "M",
            QuickRow::ShortcutReset => "R",
            QuickRow::ShortcutSaveState => "S",
            QuickRow::ShortcutLoadState => "L",
            QuickRow::ShortcutBrightnessUp => "+",
            QuickRow::ShortcutBrightnessDown => "-",
            QuickRow::ShortcutBlueLightDown => "-",
            QuickRow::ShortcutBlueLightUp => "+",
            QuickRow::ShortcutColorCorrection => "C",
            QuickRow::ShortcutPaletteBack => "P",
            QuickRow::ShortcutPaletteNext => "P",
            QuickRow::ShortcutShaderNext => "S",
            QuickRow::ShortcutShaderBack => "S",
            QuickRow::ShortcutFastForward => "F",
            QuickRow::ShortcutRewind => "R",
            QuickRow::ControlUp => "U",
            QuickRow::ControlDown => "D",
            QuickRow::ControlLeft => "L",
            QuickRow::ControlRight => "R",
            QuickRow::ControlA => "A",
            QuickRow::ControlB => "B",
            QuickRow::ControlSelect => "S",
            QuickRow::ControlStart => "T",
            QuickRow::ControlL => "L",
            QuickRow::ControlR => "R",
            QuickRow::ControlTurboA => "X",
            QuickRow::ControlTurboB => "Y",
            QuickRow::ControlPreset => "S",
        }
    }

    /// Nerd Font glyph used by the Switch-style icon tile. A neutral circle is
    /// substituted by the face builder when a font lacks the glyph.
    pub fn glyph(self) -> char {
        match self {
            QuickRow::FastForward | QuickRow::FastForwardSound => '\u{f04e}',
            QuickRow::Screen => '\u{f108}',
            QuickRow::Shelf => '\u{f07b}',
            QuickRow::ShelfLook => '\u{f03e}',
            QuickRow::DisplayPanel => '\u{f185}',
            QuickRow::DisplayEffects => '\u{f1fc}',
            QuickRow::Rumble => '\u{f0f3}',
            QuickRow::DateTime => '\u{f017}',
            QuickRow::Mods => '\u{f085}',
            QuickRow::ButtonSettings => '\u{f11c}',
            QuickRow::SystemSettings => '\u{f013}',
            QuickRow::About => '\u{f05a}',
            QuickRow::SystemUpgrade => '\u{f019}',
            QuickRow::GbaShader | QuickRow::GbShader | QuickRow::ColourCorrection => '\u{f1de}',
            QuickRow::Brightness => '\u{f185}',
            QuickRow::BlueLight => '\u{f0eb}',
            QuickRow::Game => '\u{f11b}',
                QuickRow::SaveSettings
            | QuickRow::SaveGba
            | QuickRow::SaveGb
            | QuickRow::SaveGbc => '\u{f0c7}',
            QuickRow::DefaultLoad | QuickRow::LoadGba | QuickRow::LoadGb | QuickRow::LoadGbc => '\u{f115}',
            QuickRow::DefaultSave => '\u{f0c7}',
            QuickRow::FcMask | QuickRow::GbMask | QuickRow::GbcMask | QuickRow::SfcMask => {
                '\u{f2d0}'
            }
            QuickRow::Masks => '\u{f0e0}',
            QuickRow::Recents => '\u{f1da}',
            QuickRow::Turbo => '\u{f0e7}',
            QuickRow::Rewind => '\u{f04a}',
            QuickRow::GbPalettes => '\u{f53f}',
            QuickRow::ButtonLayout => '\u{f11c}',
            QuickRow::RomTitle => '\u{f031}',
            QuickRow::FallbackCover => '\u{f03e}',
            QuickRow::LidTimeout => '\u{f186}',
            QuickRow::Shortcuts => '\u{f11c}',
            QuickRow::Network | QuickRow::Wifi => '\u{f1eb}',
            QuickRow::Web => '\u{f0ac}',
            QuickRow::SyncStorage => '\u{f0c2}',
            QuickRow::SyncContent => '\u{f0c2}',
            QuickRow::CartTransfer => '\u{f0c7}',
            QuickRow::HotspotBand => '\u{f1eb}',
            QuickRow::SplitLink => '\u{f0c1}',
            QuickRow::Theme
            | QuickRow::ThemeDefault
            | QuickRow::ThemeWeb
            | QuickRow::ThemeWarm => '\u{f1fc}',
            // fa-language; the tile fallback above covers a font without it.
            QuickRow::Language => '\u{f1ab}',
            QuickRow::ShelfKeys | QuickRow::ShelfPrev | QuickRow::ShelfNext | QuickRow::ShelfFav => {
                '\u{f11c}'
            }
            QuickRow::ShortcutReset => '\u{f021}',
            QuickRow::ShortcutMap
            | QuickRow::ShortcutSaveState
            | QuickRow::ShortcutLoadState
            | QuickRow::ShortcutBrightnessUp
            | QuickRow::ShortcutBrightnessDown
            | QuickRow::ShortcutBlueLightDown
            | QuickRow::ShortcutBlueLightUp
            | QuickRow::ShortcutColorCorrection
            | QuickRow::ShortcutPaletteBack
            | QuickRow::ShortcutPaletteNext
            | QuickRow::ShortcutShaderNext
            | QuickRow::ShortcutShaderBack
            | QuickRow::ShortcutFastForward
            | QuickRow::ShortcutRewind => '\u{f11c}',
            QuickRow::ControlUp
            | QuickRow::ControlDown
            | QuickRow::ControlLeft
            | QuickRow::ControlRight
            | QuickRow::ControlA
            | QuickRow::ControlB
            | QuickRow::ControlSelect
            | QuickRow::ControlStart
            | QuickRow::ControlL
            | QuickRow::ControlR
            | QuickRow::ControlTurboA
            | QuickRow::ControlTurboB => '\u{f11c}',
            QuickRow::ControlPreset => '\u{f11c}',
        }
    }

    pub fn parent(self) -> Option<QuickRow> {
        match self {
            QuickRow::DisplayPanel
            | QuickRow::DisplayEffects
            | QuickRow::Masks
            | QuickRow::Theme => Some(QuickRow::Screen),
            QuickRow::Brightness | QuickRow::BlueLight => Some(QuickRow::DisplayPanel),
            QuickRow::GbaShader
            | QuickRow::GbShader
            | QuickRow::ColourCorrection
            | QuickRow::GbPalettes => Some(QuickRow::DisplayEffects),
            QuickRow::FcMask | QuickRow::GbMask | QuickRow::GbcMask | QuickRow::SfcMask => {
                Some(QuickRow::Masks)
            }
            QuickRow::ShelfLook | QuickRow::ShelfKeys => Some(QuickRow::Shelf),
            QuickRow::ThemeDefault | QuickRow::ThemeWeb | QuickRow::ThemeWarm => {
                Some(QuickRow::Theme)
            }
            QuickRow::RomTitle | QuickRow::FallbackCover | QuickRow::Recents => {
                Some(QuickRow::ShelfLook)
            }
            QuickRow::ShelfPrev | QuickRow::ShelfNext | QuickRow::ShelfFav => {
                Some(QuickRow::ShelfKeys)
            }
            QuickRow::FastForward
            | QuickRow::FastForwardSound
            | QuickRow::Rewind
            | QuickRow::Rumble
            | QuickRow::SaveSettings
            => Some(QuickRow::Game),
            QuickRow::DefaultLoad
            | QuickRow::DefaultSave
            | QuickRow::LoadGba
            | QuickRow::SaveGba
            | QuickRow::LoadGb
            | QuickRow::SaveGb
            | QuickRow::LoadGbc
            | QuickRow::SaveGbc => Some(QuickRow::SaveSettings),
            QuickRow::Shortcuts
            | QuickRow::ShortcutMap
            | QuickRow::ButtonLayout
            | QuickRow::Turbo => Some(QuickRow::ButtonSettings),
            QuickRow::ShortcutSaveState
            | QuickRow::ShortcutLoadState
            | QuickRow::ShortcutBrightnessUp
            | QuickRow::ShortcutBrightnessDown
            | QuickRow::ShortcutBlueLightDown
            | QuickRow::ShortcutBlueLightUp
            | QuickRow::ShortcutColorCorrection
            | QuickRow::ShortcutPaletteBack
            | QuickRow::ShortcutPaletteNext
            | QuickRow::ShortcutShaderNext
            | QuickRow::ShortcutShaderBack
            | QuickRow::ShortcutFastForward
            | QuickRow::ShortcutRewind
            | QuickRow::ShortcutReset => Some(QuickRow::ShortcutMap),
            QuickRow::ControlUp
            | QuickRow::ControlDown
            | QuickRow::ControlLeft
            | QuickRow::ControlRight
            | QuickRow::ControlA
            | QuickRow::ControlB
            | QuickRow::ControlSelect
            | QuickRow::ControlStart
            | QuickRow::ControlL
            | QuickRow::ControlR
            | QuickRow::ControlTurboA
            | QuickRow::ControlTurboB
            | QuickRow::ControlPreset => Some(QuickRow::ButtonLayout),
            QuickRow::Wifi
            | QuickRow::Web
            | QuickRow::SyncStorage
            | QuickRow::SyncContent
            | QuickRow::CartTransfer
            | QuickRow::HotspotBand
            | QuickRow::SplitLink => Some(QuickRow::Network),
            QuickRow::DateTime
            | QuickRow::Language
            | QuickRow::LidTimeout
            | QuickRow::Mods
            | QuickRow::SystemUpgrade
            | QuickRow::About => Some(QuickRow::SystemSettings),
            _ => None,
        }
    }

    pub fn child(self) -> Option<QuickRow> {
        match self {
            QuickRow::Screen => Some(QuickRow::DISPLAY[0]),
            QuickRow::DisplayPanel => Some(QuickRow::DISPLAY_PANEL[0]),
            QuickRow::DisplayEffects => Some(QuickRow::DISPLAY_EFFECTS[0]),
            QuickRow::Masks => Some(QuickRow::MASKS[0]),
            QuickRow::Shelf => Some(QuickRow::SHELF[0]),
            QuickRow::ShelfLook => Some(QuickRow::SHELF_LOOK[0]),
            QuickRow::Theme => Some(QuickRow::THEMES[0]),
            QuickRow::Game => Some(QuickRow::GAME[0]),
            QuickRow::Network => Some(QuickRow::NETWORK[0]),
            QuickRow::ShelfKeys => Some(QuickRow::SHELF_KEYS[0]),
            QuickRow::ShortcutMap => Some(QuickRow::SHORTCUTS[0]),
            QuickRow::ButtonLayout => Some(QuickRow::CONTROLS[0]),
            QuickRow::ButtonSettings => Some(QuickRow::BUTTON_SETTINGS[0]),
            QuickRow::SystemSettings => Some(QuickRow::SYSTEM_SETTINGS[0]),
            QuickRow::SaveSettings => Some(QuickRow::SAVES[0]),
            _ => None,
        }
    }

    pub fn position(self) -> usize {
        self.page().iter().position(|r| *r == self).unwrap_or(0)
    }

    /// The translation key for this row's label.
    ///
    /// One match instead of a literal per language: `label` is then a single
    /// lookup, and the parity test in `slot-i18n` guarantees both files carry
    /// every key this table can return.
    pub fn key(self) -> &'static str {
        match self {
            QuickRow::FastForward => "device.quick.label.fast_forward",
            QuickRow::FastForwardSound => "device.quick.label.fast_forward_sound",
            QuickRow::Screen => "device.quick.label.screen",
            QuickRow::Rumble => "device.quick.label.rumble",
            QuickRow::DateTime => "device.quick.label.date_time",
            QuickRow::Mods => "device.quick.label.mods",
            QuickRow::About => "device.quick.label.about",
            QuickRow::SystemUpgrade => "device.quick.label.system_upgrade",
            QuickRow::GbaShader => "device.quick.label.gba_shader",
            QuickRow::GbShader => "device.quick.label.gb_shader",
            QuickRow::ColourCorrection => "device.quick.label.colour_correction",
            QuickRow::Brightness => "device.quick.label.brightness",
            QuickRow::BlueLight => "device.quick.label.blue_light",
            QuickRow::Game => "device.quick.label.game",
            QuickRow::SaveSettings => "device.quick.label.save_settings",
                QuickRow::LoadGba => "device.quick.label.load_gba",
            QuickRow::SaveGba => "device.quick.label.save_gba",
            QuickRow::LoadGb => "device.quick.label.load_gb",
            QuickRow::SaveGb => "device.quick.label.save_gb",
            QuickRow::LoadGbc => "device.quick.label.load_gbc",
            QuickRow::SaveGbc => "device.quick.label.save_gbc",
            QuickRow::Recents => "device.quick.label.recents",
            QuickRow::Turbo => "device.quick.label.turbo",
            QuickRow::Rewind => "device.quick.label.rewind",
            QuickRow::GbPalettes => "device.quick.label.gb_palettes",
            QuickRow::ButtonLayout => "device.quick.label.button_layout",
            QuickRow::RomTitle => "device.quick.label.rom_title",
            QuickRow::FallbackCover => "device.quick.label.fallback_cover",
            QuickRow::LidTimeout => "device.quick.label.lid_timeout",
            QuickRow::ButtonSettings => "device.quick.label.button_settings",
            QuickRow::SystemSettings => "device.quick.label.system_settings",
            QuickRow::Shortcuts => "device.quick.label.shortcuts",
            QuickRow::Network => "device.quick.label.network",
            QuickRow::Wifi => "device.quick.label.wifi",
            QuickRow::Web => "device.quick.label.web",
            QuickRow::SyncStorage => "device.quick.label.sync_storage",
            QuickRow::ShelfKeys => "device.quick.label.shelf_keys",
            QuickRow::ShelfPrev => "device.quick.label.shelf_prev",
            QuickRow::ShelfNext => "device.quick.label.shelf_next",
            QuickRow::ShelfFav => "device.quick.label.shelf_fav",
            QuickRow::ShortcutMap => "device.quick.label.shortcut_map",
            QuickRow::ShortcutSaveState => "device.quick.label.shortcut_save_state",
            QuickRow::ShortcutLoadState => "device.quick.label.shortcut_load_state",
            QuickRow::ShortcutBrightnessUp => "device.quick.label.shortcut_brightness_up",
            QuickRow::ShortcutBrightnessDown => "device.quick.label.shortcut_brightness_down",
            QuickRow::ShortcutBlueLightDown => "device.quick.label.shortcut_blue_light_down",
            QuickRow::ShortcutBlueLightUp => "device.quick.label.shortcut_blue_light_up",
            QuickRow::ShortcutColorCorrection => "device.quick.label.shortcut_color_correction",
            QuickRow::ShortcutPaletteBack => "device.quick.label.shortcut_palette_back",
            QuickRow::ShortcutPaletteNext => "device.quick.label.shortcut_palette_next",
            QuickRow::ShortcutShaderNext => "device.quick.label.shortcut_shader_next",
            QuickRow::ShortcutShaderBack => "device.quick.label.shortcut_shader_back",
            QuickRow::ShortcutFastForward => "device.quick.label.shortcut_fast_forward",
            QuickRow::ShortcutRewind => "device.quick.label.shortcut_rewind",
            QuickRow::ControlUp => "device.quick.label.control_up",
            QuickRow::ControlDown => "device.quick.label.control_down",
            QuickRow::ControlLeft => "device.quick.label.control_left",
            QuickRow::ControlRight => "device.quick.label.control_right",
            QuickRow::ControlA => "device.quick.label.control_a",
            QuickRow::ControlB => "device.quick.label.control_b",
            QuickRow::ControlSelect => "device.quick.label.control_select",
            QuickRow::ControlStart => "device.quick.label.control_start",
            QuickRow::ControlL => "device.quick.label.control_l",
            QuickRow::ControlR => "device.quick.label.control_r",
            QuickRow::ControlTurboA => "device.quick.label.control_turbo_a",
            QuickRow::ControlTurboB => "device.quick.label.control_turbo_b",
            QuickRow::ControlPreset => "device.quick.label.control_preset",
            QuickRow::SyncContent => "device.quick.label.sync_content",
            QuickRow::CartTransfer => "device.quick.label.cart_transfer",
            QuickRow::HotspotBand => "device.quick.label.hotspot_band",
            QuickRow::ShortcutReset => "device.quick.label.shortcut_reset",
            QuickRow::DisplayPanel => "device.quick.label.display_panel",
            QuickRow::DisplayEffects => "device.quick.label.display_effects",
            QuickRow::Shelf => "device.quick.label.shelf",
            QuickRow::ShelfLook => "device.quick.label.shelf_look",
            QuickRow::DefaultLoad => "device.quick.label.default_load",
            QuickRow::DefaultSave => "device.quick.label.default_save",
            QuickRow::FcMask => "device.quick.label.fc_mask",
            QuickRow::GbMask => "device.quick.label.gb_mask",
            QuickRow::GbcMask => "device.quick.label.gbc_mask",
            QuickRow::SfcMask => "device.quick.label.sfc_mask",
            QuickRow::Masks => "device.quick.label.masks",
            QuickRow::Theme => "device.quick.label.theme",
            QuickRow::ThemeDefault => "device.quick.label.theme_default",
            QuickRow::ThemeWeb => "device.quick.label.theme_web",
            QuickRow::ThemeWarm => "device.quick.label.theme_warm",
            QuickRow::Language => "device.quick.label.language",
            QuickRow::SplitLink => "device.quick.label.split_link",
        }
    }

    /// The row's label in `lang`.
    pub fn label(self, lang: Language) -> &'static str {
        slot_i18n::t(lang, self.key())
    }

    pub fn opens(self) -> bool {
        matches!(
            self,
            QuickRow::Shortcuts
                | QuickRow::DateTime
                | QuickRow::Mods
                | QuickRow::ButtonSettings
                | QuickRow::SystemSettings
                | QuickRow::About
                | QuickRow::SystemUpgrade
                | QuickRow::Screen
                | QuickRow::Shelf
                | QuickRow::ShelfLook
                | QuickRow::Theme
                | QuickRow::DisplayPanel
                | QuickRow::DisplayEffects
                | QuickRow::Masks
                | QuickRow::Game
                | QuickRow::Network
                | QuickRow::SaveSettings
                | QuickRow::ShelfKeys
                | QuickRow::ShortcutMap
                | QuickRow::ButtonLayout
                | QuickRow::Wifi
                | QuickRow::Web
                | QuickRow::SyncStorage
                | QuickRow::CartTransfer
        )
    }

    pub fn up(self) -> QuickRow {
        let page = self.page();
        page[(self.position() + page.len() - 1) % page.len()]
    }

    pub fn down(self) -> QuickRow {
        let page = self.page();
        page[(self.position() + 1) % page.len()]
    }

    pub fn resume(self) -> Option<(Platform, ResumeAction)> {
        match self {
            QuickRow::LoadGba => Some((Platform::Gba, ResumeAction::Load)),
            QuickRow::SaveGba => Some((Platform::Gba, ResumeAction::Save)),
            QuickRow::LoadGb => Some((Platform::Gb, ResumeAction::Load)),
            QuickRow::SaveGb => Some((Platform::Gb, ResumeAction::Save)),
            QuickRow::LoadGbc => Some((Platform::Gbc, ResumeAction::Load)),
            QuickRow::SaveGbc => Some((Platform::Gbc, ResumeAction::Save)),
            _ => None,
        }
    }

    /// The two global defaults at the top of the save page.
    pub fn resume_default(self) -> Option<ResumeAction> {
        match self {
            QuickRow::DefaultLoad => Some(ResumeAction::Load),
            QuickRow::DefaultSave => Some(ResumeAction::Save),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_row_indices_preserve_the_fixed_value_table_contract() {
        assert_eq!(QuickRow::ALL.len(), 87);
        for (index, row) in QuickRow::ALL.iter().enumerate() {
            assert_eq!(row.index(), index, "row index drift: {row:?}");
            assert!(!QuickRow::ALL[..index].contains(row), "duplicate row: {row:?}");
            assert!(row.page().contains(row), "row/page mismatch: {row:?}");
            assert_eq!(row.up().down(), *row);
            assert_eq!(row.down().up(), *row);
        }
        assert_eq!(QuickRow::Language.index(), 86);
        assert_eq!(QuickRow::SplitLink.index(), QuickRow::ALL.len());
        assert!(!QuickRow::ALL.contains(&QuickRow::SplitLink));
    }

    #[test]
    fn page_labels_and_indices_are_stable() {
        assert_eq!(QuickRow::QUICK_PAGES, 16);
        assert_eq!(QuickRow::Language.parent(), Some(QuickRow::SystemSettings));
        assert_eq!(QuickRow::Language.page(), &QuickRow::SYSTEM_SETTINGS[..]);
        assert_eq!(QuickRow::Language.label(Language::ZhCn), "语言");
        assert_eq!(QuickRow::Language.label(Language::EnUs), "Language");
        for index in 0..QuickRow::QUICK_PAGES {
            assert_ne!(QuickRow::page_title_at(index, Language::ZhCn),
                       QuickRow::page_title_at(index, Language::EnUs));
        }
    }

    #[test]
    fn resume_rows_use_host_independent_actions() {
        assert_eq!(QuickRow::LoadGba.resume(), Some((Platform::Gba, ResumeAction::Load)));
        assert_eq!(QuickRow::SaveGb.resume(), Some((Platform::Gb, ResumeAction::Save)));
        assert_eq!(QuickRow::DefaultLoad.resume_default(), Some(ResumeAction::Load));
        assert_eq!(QuickRow::DefaultSave.resume_default(), Some(ResumeAction::Save));
        assert_eq!(QuickRow::SystemSettings.resume(), None);
    }

    #[test]
    fn appended_language_does_not_shift_previous_rows() {
        assert_eq!(QuickRow::ALL[85], QuickRow::HotspotBand);
        assert_eq!(QuickRow::ALL[86], QuickRow::Language);
        assert_eq!(QuickRow::SYSTEM_SETTINGS[0], QuickRow::DateTime);
        assert_eq!(QuickRow::SYSTEM_SETTINGS[1], QuickRow::Language);
    }
}
