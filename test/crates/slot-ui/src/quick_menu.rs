use slot_i18n::Language;
use slot_store::{Device, GbPalette};

use slot_store::{Platform, ResumeKind};

use crate::draw::{Draw, TexId, OUT_H, OUT_W};
use crate::plate::{hint_face, hint_face_ink, UndoFace};
use crate::power_menu::{MENU_H, MENU_PAD, MENU_PX};
use crate::switch_style;
use crate::text;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum MenuIcon {
    Clock,
    Globe,
    Calendar,
    Grid,
    Update,
    Info,
    Monitor,
    Shader,
    Palette,
    Mask,
    Theme,
    FastForward,
    Rewind,
    Speaker,
    Rumble,
    Save,
    Load,
    Keyboard,
    Map,
    Layout,
    Home,
    Turbo,
    Wifi,
    Web,
    Cloud,
    Transfer,
    Hotspot,
    Star,
    Prev,
    Next,
    Reset,
    Text,
    Cover,
    Arrows,
    Cart,
    Gamepad,
    DPad,
    Shoulder,
    Gear,
    Sun,
    Moon,
    Link,
    /// Colour temperature: a slider, not a second sun.
    Colour,
    /// Recently played: a clock with a backward arrow.
    History,
    /// Cloud sync content: a cloud with a list.
    Content,
    /// The network entry: a router with its signal, not the Wi-Fi fan.
    Router,
    /// Screen and brightness: a monitor with a sun.
    MonitorSun,
    /// GBA / GB shader: wide vs tall screen, same scanline family.
    ShaderGba,
    ShaderGb,
}

/// The small 6-9px badge drawn at the bottom-right of the 22x22 glyph.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
enum Badge {
    #[default]
    None,
    Keycap,
    Up,
    Down,
    Left,
    Right,
    CartGba,
    CartGb,
    CartGbc,
    ScreenFc,
    ScreenGb,
    ScreenGbc,
    ScreenSfc,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    LetterA,
    LetterB,
    LetterL,
    LetterR,
    Select,
    Start,
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

    /// One shipped theme.  `ThemeDefault`/`ThemeWeb` remain as compatibility
    /// discriminants, but no page can select them any more.
    pub const THEMES: &[QuickRow] = &[QuickRow::ThemeWarm];

    pub const SHELF: [QuickRow; 2] = [QuickRow::ShelfLook, QuickRow::ShelfKeys];

    pub const SHELF_LOOK: [QuickRow; 2] = [QuickRow::RomTitle, QuickRow::Recents];

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

    /// Number of rows on page `index`; the header prints this as `SETTINGS / 06`.
    pub fn page_len_at(index: usize) -> usize {
        QuickRow::ALL
            .iter()
            .find(|row| row.page_index() == index)
            .map_or(0, |row| row.page().len())
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn page(self) -> &'static [QuickRow] {
        match self {
            QuickRow::DisplayPanel
            | QuickRow::DisplayEffects
            | QuickRow::Masks
            | QuickRow::Theme
            | QuickRow::ThemeWeb => &QuickRow::DISPLAY,
            QuickRow::Brightness | QuickRow::BlueLight => &QuickRow::DISPLAY_PANEL,
            QuickRow::GbaShader
            | QuickRow::GbShader
            | QuickRow::ColourCorrection
            | QuickRow::GbPalettes => &QuickRow::DISPLAY_EFFECTS,
            QuickRow::FcMask | QuickRow::GbMask | QuickRow::GbcMask | QuickRow::SfcMask => {
                &QuickRow::MASKS
            }
            QuickRow::ShelfLook | QuickRow::ShelfKeys => &QuickRow::SHELF,
            QuickRow::ThemeDefault | QuickRow::ThemeWarm => &QuickRow::THEMES,
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
            QuickRow::ThemeDefault | QuickRow::ThemeWarm => 15,
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

    fn menu_icon(self) -> MenuIcon {
        match self {
            QuickRow::FastForward | QuickRow::ShortcutFastForward => MenuIcon::FastForward,
            QuickRow::FastForwardSound => MenuIcon::Speaker,
            QuickRow::Screen => MenuIcon::Monitor,
            QuickRow::DisplayPanel => MenuIcon::MonitorSun,
            QuickRow::Rumble => MenuIcon::Rumble,
            QuickRow::DateTime => MenuIcon::Clock,
            QuickRow::Recents => MenuIcon::History,
            QuickRow::Mods => MenuIcon::Grid,
            QuickRow::About => MenuIcon::Info,
            QuickRow::SystemUpgrade => MenuIcon::Update,
            QuickRow::GbaShader => MenuIcon::ShaderGba,
            QuickRow::GbShader => MenuIcon::ShaderGb,
            QuickRow::ShortcutShaderNext | QuickRow::ShortcutShaderBack => MenuIcon::Shader,
            QuickRow::GbPalettes
            | QuickRow::ShortcutPaletteBack
            | QuickRow::ShortcutPaletteNext
            | QuickRow::DisplayEffects => MenuIcon::Palette,
            QuickRow::ColourCorrection | QuickRow::ShortcutColorCorrection => MenuIcon::Colour,
            QuickRow::Masks
            | QuickRow::FcMask
            | QuickRow::GbMask
            | QuickRow::GbcMask
            | QuickRow::SfcMask => MenuIcon::Mask,
            QuickRow::Brightness | QuickRow::ShortcutBrightnessUp | QuickRow::ShortcutBrightnessDown => {
                MenuIcon::Sun
            }
            QuickRow::BlueLight
            | QuickRow::ShortcutBlueLightDown
            | QuickRow::ShortcutBlueLightUp => MenuIcon::Moon,
            QuickRow::Game => MenuIcon::Cart,
            QuickRow::ButtonSettings
            | QuickRow::ControlA
            | QuickRow::ControlB
            | QuickRow::ControlSelect
            | QuickRow::ControlStart => MenuIcon::Gamepad,
            QuickRow::SaveSettings
            | QuickRow::SaveGba
            | QuickRow::SaveGb
            | QuickRow::SaveGbc
            | QuickRow::DefaultSave
            | QuickRow::ShortcutSaveState => MenuIcon::Save,
            QuickRow::LoadGba
            | QuickRow::LoadGb
            | QuickRow::LoadGbc
            | QuickRow::DefaultLoad
            | QuickRow::ShortcutLoadState => MenuIcon::Load,
            QuickRow::Turbo | QuickRow::ControlTurboA | QuickRow::ControlTurboB => MenuIcon::Turbo,
            QuickRow::Rewind | QuickRow::ShortcutRewind => MenuIcon::Rewind,
            QuickRow::ButtonLayout | QuickRow::ControlPreset => MenuIcon::Layout,
            QuickRow::RomTitle => MenuIcon::Text,
            QuickRow::FallbackCover | QuickRow::ShelfLook => MenuIcon::Cover,
            QuickRow::LidTimeout => MenuIcon::Calendar,
            QuickRow::SystemSettings => MenuIcon::Gear,
            QuickRow::Shortcuts => MenuIcon::Keyboard,
            QuickRow::Network => MenuIcon::Router,
            QuickRow::Wifi => MenuIcon::Wifi,
            QuickRow::Web => MenuIcon::Web,
            QuickRow::SyncStorage => MenuIcon::Cloud,
            QuickRow::SyncContent => MenuIcon::Content,
            QuickRow::ShelfKeys => MenuIcon::Arrows,
            QuickRow::ShelfPrev => MenuIcon::Prev,
            QuickRow::ShelfNext => MenuIcon::Next,
            QuickRow::ShelfFav => MenuIcon::Star,
            QuickRow::ShortcutMap => MenuIcon::Map,
            QuickRow::ShortcutReset => MenuIcon::Reset,
            QuickRow::ControlUp
            | QuickRow::ControlDown
            | QuickRow::ControlLeft
            | QuickRow::ControlRight => MenuIcon::DPad,
            QuickRow::ControlL | QuickRow::ControlR => MenuIcon::Shoulder,
            QuickRow::CartTransfer => MenuIcon::Transfer,
            QuickRow::Theme
            | QuickRow::ThemeDefault
            | QuickRow::ThemeWeb
            | QuickRow::ThemeWarm => MenuIcon::Theme,
            QuickRow::Shelf => MenuIcon::Home,
            QuickRow::HotspotBand => MenuIcon::Hotspot,
            QuickRow::Language => MenuIcon::Globe,
            QuickRow::SplitLink => MenuIcon::Link,
        }
    }

    fn menu_badge(self) -> Badge {
        match self {
            QuickRow::SaveGba | QuickRow::LoadGba => Badge::CartGba,
            QuickRow::SaveGb | QuickRow::LoadGb => Badge::CartGb,
            QuickRow::SaveGbc | QuickRow::LoadGbc => Badge::CartGbc,
            QuickRow::FcMask => Badge::ScreenFc,
            QuickRow::GbMask => Badge::ScreenGb,
            QuickRow::GbcMask => Badge::ScreenGbc,
            QuickRow::SfcMask => Badge::ScreenSfc,
            QuickRow::ControlUp => Badge::DpadUp,
            QuickRow::ControlDown => Badge::DpadDown,
            QuickRow::ControlLeft => Badge::DpadLeft,
            QuickRow::ControlRight => Badge::DpadRight,
            QuickRow::ControlA => Badge::LetterA,
            QuickRow::ControlB => Badge::LetterB,
            QuickRow::ControlSelect => Badge::Select,
            QuickRow::ControlStart => Badge::Start,
            QuickRow::ControlL => Badge::LetterL,
            QuickRow::ControlR => Badge::LetterR,
            QuickRow::ControlTurboA => Badge::LetterA,
            QuickRow::ControlTurboB => Badge::LetterB,
            QuickRow::ShortcutSaveState
            | QuickRow::ShortcutLoadState
            | QuickRow::ShortcutColorCorrection
            | QuickRow::ShortcutFastForward
            | QuickRow::ShortcutRewind
            | QuickRow::ShortcutReset => Badge::Keycap,
            QuickRow::ShortcutBrightnessUp => Badge::Up,
            QuickRow::ShortcutBrightnessDown => Badge::Down,
            QuickRow::ShortcutBlueLightDown => Badge::Down,
            QuickRow::ShortcutBlueLightUp => Badge::Up,
            QuickRow::ShortcutPaletteBack => Badge::Left,
            QuickRow::ShortcutPaletteNext => Badge::Right,
            QuickRow::ShortcutShaderNext => Badge::Right,
            QuickRow::ShortcutShaderBack => Badge::Left,
            _ => Badge::None,
        }
    }

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
            | QuickRow::SaveSettings => Some(QuickRow::Game),
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
        let page = self.page();
        page.iter().position(|row| *row == self).unwrap_or(0)
    }

    pub fn up(self) -> QuickRow {
        let page = self.page();
        page[(self.position() + page.len() - 1) % page.len()]
    }

    pub fn down(self) -> QuickRow {
        let page = self.page();
        page[(self.position() + 1) % page.len()]
    }

    pub fn resume(self) -> Option<(Platform, ResumeKind)> {
        match self {
            QuickRow::LoadGba => Some((Platform::Gba, ResumeKind::Load)),
            QuickRow::SaveGba => Some((Platform::Gba, ResumeKind::Save)),
            QuickRow::LoadGb => Some((Platform::Gb, ResumeKind::Load)),
            QuickRow::SaveGb => Some((Platform::Gb, ResumeKind::Save)),
            QuickRow::LoadGbc => Some((Platform::Gbc, ResumeKind::Load)),
            QuickRow::SaveGbc => Some((Platform::Gbc, ResumeKind::Save)),
            _ => None,
        }
    }

    /// The two global defaults at the top of the save page.
    pub fn resume_default(self) -> Option<ResumeKind> {
        match self {
            QuickRow::DefaultLoad => Some(ResumeKind::Load),
            QuickRow::DefaultSave => Some(ResumeKind::Save),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum QuickValue {
    Speed2,
    Speed3,
    Speed4,
    Speed6,
    On,
    Off,
    Level0,
    Level1,
    Level2,
    Level3,
    Level4,
    Level5,
    Level6,
    Level7,
    Level8,
    Level9,
    Palette(GbPalette),
    Control(Device),
    Shortcut(Device),
    MenuShortcut(Device),
    PlainShortcut(Device),
    Lcd3x,
    Grid,
    Dot,
    Simpletex,
    FourPass,
    SwapShoulders,
    XyShoulders,
    Both,
    Minutes3,
    Minutes10,
    Minutes30,
    Never,
    SyncAll,
    SyncState,
    SyncSav,
    /// A per-platform resume switch that follows the global default.
    Follow,
    /// A shortcut that was turned off: no chording will fire it.
    Unbound,
    /// The palette that is already selected in the theme list.
    ThemeCurrent,
    /// A palette that can be selected in the theme list.
    ThemePick,
    /// 系统 → 语言 → 中文.
    LanguageZh,
    /// 系统 → 语言 → English.
    LanguageEn,
}

impl QuickValue {
    pub const ALL: [QuickValue; 37] = [
        QuickValue::Speed2,
        QuickValue::Speed3,
        QuickValue::Speed4,
        QuickValue::Speed6,
        QuickValue::On,
        QuickValue::Off,
        QuickValue::Level0,
        QuickValue::Level1,
        QuickValue::Level2,
        QuickValue::Level3,
        QuickValue::Level4,
        QuickValue::Level5,
        QuickValue::Level6,
        QuickValue::Level7,
        QuickValue::Level8,
        QuickValue::Level9,
        QuickValue::Lcd3x,
        QuickValue::Grid,
        QuickValue::Dot,
        QuickValue::Simpletex,
        QuickValue::FourPass,
        QuickValue::SwapShoulders,
        QuickValue::XyShoulders,
        QuickValue::Both,
        QuickValue::Minutes3,
        QuickValue::Minutes10,
        QuickValue::Minutes30,
        QuickValue::Never,
        QuickValue::SyncAll,
        QuickValue::SyncState,
        QuickValue::SyncSav,
        QuickValue::Follow,
        QuickValue::Unbound,
        QuickValue::ThemeCurrent,
        QuickValue::ThemePick,
        QuickValue::LanguageZh,
        QuickValue::LanguageEn,
    ];

    pub fn index(self) -> usize {
        match self {
            QuickValue::Palette(p) => Self::ALL.len() + p.index(),
            QuickValue::Control(d) => {
                Self::ALL.len()
                    + GbPalette::COUNT
                    + Device::ALL.iter().position(|x| *x == d).unwrap_or(0)
            }
            QuickValue::Shortcut(d) => {
                Self::ALL.len()
                    + GbPalette::COUNT
                    + Device::ALL.len()
                    + Device::ALL.iter().position(|x| *x == d).unwrap_or(0)
            }
            QuickValue::MenuShortcut(d) => {
                Self::ALL.len()
                    + GbPalette::COUNT
                    + Device::ALL.len() * 2
                    + Device::ALL.iter().position(|x| *x == d).unwrap_or(0)
            }
            QuickValue::PlainShortcut(d) => {
                Self::ALL.len()
                    + GbPalette::COUNT
                    + Device::ALL.len() * 3
                    + Device::ALL.iter().position(|x| *x == d).unwrap_or(0)
            }
            other => Self::ALL.iter().position(|v| *v == other).unwrap_or(0),
        }
    }

    pub fn palette_values() -> impl Iterator<Item = QuickValue> {
        GbPalette::all().map(QuickValue::Palette)
    }

    pub fn control_values() -> impl Iterator<Item = QuickValue> {
        Device::ALL.into_iter().map(QuickValue::Control)
    }

    pub fn shortcut_values() -> impl Iterator<Item = QuickValue> {
        Device::ALL.into_iter().map(QuickValue::Shortcut)
    }

    pub fn menu_shortcut_values() -> impl Iterator<Item = QuickValue> {
        Device::ALL.into_iter().map(QuickValue::MenuShortcut)
    }

    pub fn plain_shortcut_values() -> impl Iterator<Item = QuickValue> {
        Device::ALL.into_iter().map(QuickValue::PlainShortcut)
    }

    /// The value's text in `lang`.
    ///
    /// The two language names are translated even though they read the same in
    /// both languages — that is what keeps the row legible to someone who
    /// switched by accident.
    pub fn text(self, lang: Language) -> &'static str {
        match self {
            QuickValue::LanguageZh => slot_i18n::t(lang, "device.quick.value.language_zh"),
            QuickValue::LanguageEn => slot_i18n::t(lang, "device.quick.value.language_en"),
            QuickValue::Speed2 => "2×",
            QuickValue::Speed3 => "3×",
            QuickValue::Speed4 => "4×",
            QuickValue::Speed6 => "6×",
            QuickValue::On => slot_i18n::t(lang, "device.quick.value.on"),
            QuickValue::Off => slot_i18n::t(lang, "device.quick.value.off"),
            QuickValue::Level0 => "0",
            QuickValue::Level1 => "1",
            QuickValue::Level2 => "2",
            QuickValue::Level3 => "3",
            QuickValue::Level4 => "4",
            QuickValue::Level5 => "5",
            QuickValue::Level6 => "6",
            QuickValue::Level7 => "7",
            QuickValue::Level8 => "8",
            QuickValue::Level9 => "9",
            QuickValue::Palette(p) => palette_name(p, lang),
            QuickValue::Control(d) => d.label(),
            QuickValue::Shortcut(d) => shortcut_chord(d),
            QuickValue::MenuShortcut(d) => menu_shortcut_chord(d),
            QuickValue::PlainShortcut(d) => d.label(),
            QuickValue::Lcd3x => "LCD3x",
            QuickValue::Grid => slot_i18n::t(lang, "device.quick.value.grid"),
            QuickValue::Dot => slot_i18n::t(lang, "device.quick.value.dot"),
            QuickValue::Simpletex => slot_i18n::t(lang, "device.quick.value.simpletex"),
            QuickValue::FourPass => slot_i18n::t(lang, "device.quick.value.four_pass"),
            QuickValue::SwapShoulders => slot_i18n::t(lang, "device.quick.value.swap_shoulders"),
            QuickValue::XyShoulders => slot_i18n::t(lang, "device.quick.value.xy_shoulders"),
            QuickValue::Both => slot_i18n::t(lang, "device.quick.value.both"),
            QuickValue::Minutes3 => slot_i18n::t(lang, "device.quick.value.minutes_3"),
            QuickValue::Minutes10 => slot_i18n::t(lang, "device.quick.value.minutes_10"),
            QuickValue::Minutes30 => slot_i18n::t(lang, "device.quick.value.minutes_30"),
            QuickValue::Never => slot_i18n::t(lang, "device.quick.value.never"),
            QuickValue::SyncAll => slot_i18n::t(lang, "device.quick.value.sync_all"),
            QuickValue::SyncState => slot_i18n::t(lang, "device.quick.value.sync_state"),
            QuickValue::SyncSav => slot_i18n::t(lang, "device.quick.value.sync_sav"),
            QuickValue::Follow => slot_i18n::t(lang, "device.quick.value.follow"),
            QuickValue::Unbound => slot_i18n::t(lang, "device.quick.value.unbound"),
            QuickValue::ThemeCurrent => slot_i18n::t(lang, "device.quick.value.theme_current"),
            QuickValue::ThemePick => slot_i18n::t(lang, "device.quick.value.theme_pick"),
        }
    }

    pub fn speed(frames: u8) -> Option<QuickValue> {
        match frames {
            0 => Some(QuickValue::Off),
            2 => Some(QuickValue::Speed2),
            3 => Some(QuickValue::Speed3),
            4 => Some(QuickValue::Speed4),
            6 => Some(QuickValue::Speed6),
            _ => None,
        }
    }

    pub fn flag(on: bool) -> QuickValue {
        if on {
            QuickValue::On
        } else {
            QuickValue::Off
        }
    }

    pub fn level(n: u8) -> QuickValue {
        [
            QuickValue::Level0,
            QuickValue::Level1,
            QuickValue::Level2,
            QuickValue::Level3,
            QuickValue::Level4,
            QuickValue::Level5,
            QuickValue::Level6,
            QuickValue::Level7,
            QuickValue::Level8,
            QuickValue::Level9,
        ][usize::from(n).min(9)]
    }
}

/// A palette's name in `lang`.
///
/// Chinese uses the fork's own name; English uses the mGBA name, which is
/// already English (`Grayscale`, `DMG Green`, …) and is what the state file
/// stores, so nothing new has to be translated for 48 palettes.
pub fn palette_name(palette: GbPalette, lang: Language) -> &'static str {
    match lang {
        Language::ZhCn => palette.label(),
        Language::EnUs => palette.core_name(),
    }
}

/// The Switch-style face for a Select chord, e.g. `SEL+UP`.
fn menu_shortcut_chord(device: Device) -> &'static str {
    match device {
        Device::Up => "MENU+UP",
        Device::Down => "MENU+DOWN",
        Device::Left => "MENU+LEFT",
        Device::Right => "MENU+RIGHT",
        Device::A => "MENU+A",
        Device::B => "MENU+B",
        Device::X => "MENU+X",
        Device::Y => "MENU+Y",
        Device::L1 => "MENU+L1",
        Device::R1 => "MENU+R1",
        Device::L2 => "MENU+L2",
        Device::R2 => "MENU+R2",
        Device::Select => "MENU+SELECT",
        Device::Start => "MENU+START",
    }
}

fn shortcut_chord(device: Device) -> &'static str {
    match device {
        Device::Up => "SEL+UP",
        Device::Down => "SEL+DOWN",
        Device::Left => "SEL+LEFT",
        Device::Right => "SEL+RIGHT",
        Device::A => "SEL+A",
        Device::B => "SEL+B",
        Device::X => "SEL+X",
        Device::Y => "SEL+Y",
        Device::L1 => "SEL+L1",
        Device::R1 => "SEL+R1",
        Device::L2 => "SEL+L2",
        Device::R2 => "SEL+R2",
        Device::Select => "SEL+SELECT",
        Device::Start => "SEL+START",
    }
}

pub const QUICK_PITCH: f32 = 54.0;
pub const QUICK_TOP: f32 = 96.0;
pub const QUICK_EDGE: f32 = 24.0;
pub const QUICK_CARD_W: u32 = 672;
pub const QUICK_CARD_H: u32 = 48;
const CARD_Y: f32 = 96.0;
const ICON_BOX: f32 = 30.0;
const ICON_X: f32 = 40.0;
const LABEL_X: f32 = 78.0;
const RIGHT_PAD: f32 = 16.0;
const CARET_PX: f32 = 12.0;
const LEGEND_Y: f32 = 440.0;
pub const QUICK_SPLIT: f32 = LEGEND_Y - 2.0;

/// Console Flat menu: only the selected row has a face.  Unselected rows are
/// the flat page background; the face stays transparent.
pub fn quick_card_face(selected: bool) -> UndoFace {
    if selected {
        switch_style::rounded_face(
            QUICK_CARD_W,
            QUICK_CARD_H,
            11.0,
            switch_style::selected(),
            None,
        )
    } else {
        UndoFace {
            rgba: vec![0u8; (QUICK_CARD_W * QUICK_CARD_H * 4) as usize],
            w: QUICK_CARD_W,
            h: QUICK_CARD_H,
        }
    }
}

pub fn quick_label_face(row: QuickRow, lang: Language) -> UndoFace {
    quick_label_face_state(row, false, lang)
}

pub fn quick_label_face_state(row: QuickRow, lit: bool, lang: Language) -> UndoFace {
    quick_text_face_at(
        row.label(lang),
        if lit {
            [0x1B, 0x1B, 0x1E]
        } else {
            switch_style::ink()
        },
        18.0,
        13.0,
    )
}

/// `text` is already resolved: the clock, a mask file name and a menu value all
/// arrive as strings, so this builder takes the string rather than a value.
pub fn quick_value_face(text: &str, lit: bool) -> UndoFace {
    quick_text_face_at(
        text,
        if lit {
            [0x3A, 0x3A, 0x40]
        } else {
            [0xA6, 0xA6, 0xAD]
        },
        13.0,
        11.0,
    )
}

/// The page header, drawn from the page's index rather than a title string so
/// the language decides the text.
pub fn quick_title_face(index: usize, lang: Language) -> UndoFace {
    quick_text_face_at(
        QuickRow::page_title_at(index, lang),
        switch_style::ink(),
        26.0,
        14.0,
    )
}

/// Right-aligned page coordinate, e.g. `SETTINGS / 06`.
pub fn quick_page_meta_face(count: usize) -> UndoFace {
    quick_text_face_at(
        &format!("SETTINGS / {:02}", count),
        [0x74, 0x74, 0x7C],
        11.0,
        10.0,
    )
}

/// The old menu has no printed row number.
pub fn quick_number_face(number: usize) -> UndoFace {
    let _ = number;
    UndoFace {
        rgba: Vec::new(),
        w: 0,
        h: 0,
    }
}

pub fn quick_icon_face(row: QuickRow, lit: bool, _lang: Language) -> crate::cart::CartFace {
    let size = ICON_BOX as u32;
    let ink = if lit { [0x1B, 0x1B, 0x1E] } else { switch_style::ink() };
    let mut tile = UndoFace {
        rgba: vec![0u8; (22 * 22 * 4) as usize],
        w: 22,
        h: 22,
    };
    draw_menu_icon(&mut tile, row.menu_icon(), row.menu_badge(), ink);
    let tile = crate::plate::upscale_nearest(&tile, size, size);
    crate::cart::CartFace {
        rgba: tile.rgba,
        w: tile.w,
        h: tile.h,
    }
}

fn draw_menu_icon(tile: &mut UndoFace, icon: MenuIcon, badge: Badge, ink: [u8; 3]) {
    let side = tile.w as i32;
    let mut pen = |x: i32, y: i32| {
        // Draw on the native 22px grid, then enlarge the finished tile by
        // nearest-neighbour below.  Scaling point-by-point would turn
        // diagonals into dashes.
        if (0..side).contains(&x) && (0..side).contains(&y) {
            let at = ((y as u32 * tile.w + x as u32) * 4) as usize;
            tile.rgba[at..at + 3].copy_from_slice(&ink);
            tile.rgba[at + 3] = 255;
        }
    };
    let line = |pen: &mut dyn FnMut(i32, i32), x0: i32, y0: i32, x1: i32, y1: i32| {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (x0, y0);
        loop {
            pen(x, y);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    };
    let arc = |pen: &mut dyn FnMut(i32, i32), cx: i32, cy: i32, r: i32, from: i32, to: i32| {
        let steps = (to - from).abs().max(1);
        for step in 0..=steps {
            let angle = (from + (to - from) * step / steps) as f32 * std::f32::consts::PI / 180.0;
            pen(
                cx + (angle.cos() * r as f32).round() as i32,
                cy + (angle.sin() * r as f32).round() as i32,
            );
        }
    };
    let poly = |pen: &mut dyn FnMut(i32, i32), pts: &[(i32, i32)]| {
        for pair in pts.windows(2) {
            line(pen, pair[0].0, pair[0].1, pair[1].0, pair[1].1);
        }
    };
    match icon {
        MenuIcon::Clock => {
            circle(&mut pen, 11, 11, 7);
            line(&mut pen, 11, 11, 11, 7);
            line(&mut pen, 11, 11, 14, 13);
        }
        MenuIcon::Globe => {
            circle(&mut pen, 11, 11, 7);
            line(&mut pen, 4, 11, 18, 11);
            line(&mut pen, 11, 4, 11, 18);
            arc(&mut pen, 11, 11, 7, 40, 140);
            arc(&mut pen, 11, 11, 7, 220, 320);
        }
        MenuIcon::Calendar => {
            rounded_rect(&mut pen, 4, 6, 14, 12);
            line(&mut pen, 4, 9, 18, 9);
            line(&mut pen, 7, 4, 7, 7);
            line(&mut pen, 15, 4, 15, 7);
        }
        MenuIcon::Grid => {
            for (x, y) in [(4, 4), (12, 4), (4, 12), (12, 12)] {
                rect(&mut pen, x, y, 5, 5);
            }
        }
        MenuIcon::Update => {
            line(&mut pen, 11, 4, 11, 14);
            line(&mut pen, 7, 10, 11, 14);
            line(&mut pen, 15, 10, 11, 14);
            line(&mut pen, 5, 18, 17, 18);
        }
        MenuIcon::Info => {
            circle(&mut pen, 11, 11, 7);
            line(&mut pen, 11, 10, 11, 15);
            line(&mut pen, 11, 7, 11, 7);
        }
        MenuIcon::Monitor => {
            rounded_rect(&mut pen, 3, 5, 16, 11);
            line(&mut pen, 8, 19, 14, 19);
            line(&mut pen, 11, 16, 11, 19);
        }
        MenuIcon::Shader => {
            rounded_rect(&mut pen, 3, 5, 16, 12);
            line(&mut pen, 6, 9, 9, 12);
            line(&mut pen, 9, 12, 12, 9);
            line(&mut pen, 12, 9, 15, 12);
            line(&mut pen, 8, 19, 14, 19);
            line(&mut pen, 11, 17, 11, 19);
        }
        MenuIcon::Palette => {
            circle(&mut pen, 11, 11, 7);
            arc(&mut pen, 11, 11, 4, 300, 40);
            for (x, y) in [(8, 8), (12, 7), (15, 10)] {
                line(&mut pen, x, y, x, y);
            }
        }
        MenuIcon::Mask => {
            rounded_rect(&mut pen, 3, 6, 16, 10);
            line(&mut pen, 6, 10, 9, 10);
            line(&mut pen, 13, 10, 16, 10);
            line(&mut pen, 8, 14, 14, 14);
        }
        MenuIcon::Theme => {
            circle(&mut pen, 11, 11, 7);
            line(&mut pen, 11, 4, 11, 18);
            arc(&mut pen, 11, 11, 4, 90, 270);
        }
        MenuIcon::FastForward => {
            poly(&mut pen, &[(4, 5), (10, 11), (4, 17), (4, 5)]);
            poly(&mut pen, &[(11, 5), (17, 11), (11, 17), (11, 5)]);
        }
        MenuIcon::Rewind => {
            poly(&mut pen, &[(18, 5), (12, 11), (18, 17), (18, 5)]);
            poly(&mut pen, &[(11, 5), (5, 11), (11, 17), (11, 5)]);
        }
        MenuIcon::Speaker => {
            poly(&mut pen, &[(3, 9), (7, 9), (11, 5), (11, 17), (7, 13), (3, 13), (3, 9)]);
            arc(&mut pen, 12, 11, 3, -60, 60);
            arc(&mut pen, 13, 11, 6, -55, 55);
        }
        MenuIcon::Rumble => {
            rounded_rect(&mut pen, 6, 3, 10, 16);
            line(&mut pen, 9, 7, 9, 10);
            line(&mut pen, 13, 7, 13, 10);
            line(&mut pen, 9, 12, 9, 15);
            line(&mut pen, 13, 12, 13, 15);
        }
        MenuIcon::Save => {
            rounded_rect(&mut pen, 4, 3, 14, 16);
            line(&mut pen, 7, 3, 7, 8);
            line(&mut pen, 7, 8, 14, 8);
            line(&mut pen, 14, 3, 14, 8);
            line(&mut pen, 7, 15, 15, 15);
        }
        MenuIcon::Load => {
            rounded_rect(&mut pen, 4, 3, 14, 16);
            line(&mut pen, 7, 3, 7, 8);
            line(&mut pen, 7, 8, 14, 8);
            line(&mut pen, 11, 10, 11, 16);
            line(&mut pen, 8, 13, 11, 16);
            line(&mut pen, 14, 13, 11, 16);
        }
        MenuIcon::Keyboard => {
            rounded_rect(&mut pen, 2, 5, 18, 12);
            for x in [5, 8, 11, 14, 17] {
                line(&mut pen, x, 8, x, 8);
            }
            for x in [5, 8, 11, 14, 17] {
                line(&mut pen, x, 11, x, 11);
            }
            line(&mut pen, 7, 14, 15, 14);
        }
        MenuIcon::Map => {
            poly(&mut pen, &[(3, 6), (8, 4), (14, 6), (19, 4), (19, 18), (14, 16), (8, 18), (3, 16), (3, 6)]);
            line(&mut pen, 8, 4, 8, 18);
            line(&mut pen, 14, 6, 14, 20);
        }
        MenuIcon::Layout => {
            rounded_rect(&mut pen, 2, 5, 18, 12);
            line(&mut pen, 8, 5, 8, 17);
            line(&mut pen, 14, 5, 14, 17);
        }
        MenuIcon::Home => {
            poly(&mut pen, &[(3, 11), (11, 4), (19, 11)]);
            line(&mut pen, 5, 10, 5, 18);
            line(&mut pen, 17, 10, 17, 18);
            line(&mut pen, 5, 18, 17, 18);
            line(&mut pen, 9, 18, 9, 13);
            line(&mut pen, 9, 13, 13, 13);
            line(&mut pen, 13, 13, 13, 18);
        }
        MenuIcon::Turbo => {
            line(&mut pen, 3, 6, 12, 6);
            line(&mut pen, 3, 11, 15, 11);
            line(&mut pen, 3, 16, 12, 16);
            line(&mut pen, 14, 8, 18, 11);
            line(&mut pen, 14, 14, 18, 11);
        }
        MenuIcon::Wifi => {
            arc(&mut pen, 11, 15, 9, 215, 325);
            arc(&mut pen, 11, 15, 6, 215, 325);
            arc(&mut pen, 11, 15, 3, 215, 325);
            line(&mut pen, 11, 17, 11, 17);
        }
        MenuIcon::Web => {
            circle(&mut pen, 11, 11, 7);
            line(&mut pen, 4, 11, 18, 11);
            line(&mut pen, 11, 4, 11, 18);
            arc(&mut pen, 11, 11, 7, 40, 140);
            arc(&mut pen, 11, 11, 7, 220, 320);
        }
        MenuIcon::Cloud => {
            arc(&mut pen, 8, 11, 4, 180, 360);
            arc(&mut pen, 14, 11, 5, 0, 180);
            line(&mut pen, 4, 11, 4, 15);
            line(&mut pen, 19, 15, 19, 11);
            line(&mut pen, 4, 15, 8, 18);
            line(&mut pen, 8, 18, 16, 18);
            line(&mut pen, 16, 18, 19, 15);
            line(&mut pen, 11, 12, 11, 18);
            line(&mut pen, 8, 15, 11, 18);
            line(&mut pen, 14, 15, 11, 18);
        }
        MenuIcon::Transfer => {
            line(&mut pen, 3, 7, 18, 7);
            line(&mut pen, 15, 4, 18, 7);
            line(&mut pen, 15, 10, 18, 7);
            line(&mut pen, 19, 15, 4, 15);
            line(&mut pen, 7, 12, 4, 15);
            line(&mut pen, 7, 18, 4, 15);
        }
        MenuIcon::Hotspot => {
            circle(&mut pen, 11, 12, 2);
            arc(&mut pen, 11, 12, 5, 210, 330);
            arc(&mut pen, 11, 12, 8, 210, 330);
            arc(&mut pen, 11, 12, 5, 30, 150);
            arc(&mut pen, 11, 12, 8, 30, 150);
        }
        MenuIcon::Star => {
            poly(&mut pen, &[(11, 3), (13, 8), (18, 9), (14, 12), (15, 18), (11, 15), (7, 18), (8, 12), (4, 9), (9, 8), (11, 3)]);
        }
        MenuIcon::Prev => {
            line(&mut pen, 14, 4, 7, 11);
            line(&mut pen, 7, 11, 14, 18);
        }
        MenuIcon::Next => {
            line(&mut pen, 7, 4, 14, 11);
            line(&mut pen, 14, 11, 7, 18);
        }
        MenuIcon::Reset => {
            arc(&mut pen, 11, 11, 7, 40, 320);
            line(&mut pen, 4, 8, 4, 12);
            line(&mut pen, 4, 12, 8, 12);
        }
        MenuIcon::Text => {
            line(&mut pen, 4, 5, 18, 5);
            line(&mut pen, 11, 5, 11, 18);
            line(&mut pen, 8, 18, 14, 18);
        }
        MenuIcon::Cover => {
            rounded_rect(&mut pen, 3, 4, 16, 14);
            line(&mut pen, 3, 14, 8, 10);
            line(&mut pen, 8, 10, 12, 13);
            line(&mut pen, 12, 13, 15, 11);
            line(&mut pen, 15, 11, 19, 15);
        }
        MenuIcon::Arrows => {
            line(&mut pen, 3, 8, 19, 8);
            line(&mut pen, 6, 5, 3, 8);
            line(&mut pen, 6, 11, 3, 8);
            line(&mut pen, 19, 15, 3, 15);
            line(&mut pen, 16, 12, 19, 15);
            line(&mut pen, 16, 18, 19, 15);
        }
        MenuIcon::Cart => {
            rounded_rect(&mut pen, 4, 3, 14, 16);
            line(&mut pen, 7, 6, 15, 6);
            line(&mut pen, 7, 9, 15, 9);
            line(&mut pen, 7, 15, 15, 15);
        }
        MenuIcon::Gamepad => {
            poly(&mut pen, &[(6, 6), (16, 6), (20, 11), (18, 17), (14, 17), (12, 14), (10, 14), (8, 17), (4, 17), (2, 11), (6, 6)]);
            line(&mut pen, 6, 10, 6, 13);
            line(&mut pen, 4, 11, 8, 11);
            line(&mut pen, 15, 10, 15, 10);
            line(&mut pen, 17, 12, 17, 12);
        }
        MenuIcon::DPad => {
            poly(&mut pen, &[(9, 3), (13, 3), (13, 9), (19, 9), (19, 13), (13, 13), (13, 19), (9, 19), (9, 13), (3, 13), (3, 9), (9, 9), (9, 3)]);
        }
        MenuIcon::Shoulder => {
            poly(&mut pen, &[(3, 11), (5, 6), (9, 4), (15, 4), (19, 6), (20, 12), (20, 18), (15, 18), (15, 12), (7, 12), (7, 18), (3, 18), (3, 11)]);
        }
        MenuIcon::Gear => {
            circle(&mut pen, 11, 11, 3);
            for (dx, dy) in [(0, -8), (0, 8), (-8, 0), (8, 0), (-6, -6), (6, 6), (6, -6), (-6, 6)] {
                line(&mut pen, 11 + dx * 3 / 4, 11 + dy * 3 / 4, 11 + dx, 11 + dy);
            }
        }
        MenuIcon::Sun => {
            circle(&mut pen, 11, 11, 3);
            for (dx, dy) in [(0, -8), (0, 8), (-8, 0), (8, 0), (-6, -6), (6, 6), (6, -6), (-6, 6)] {
                line(&mut pen, 11 + dx * 3 / 4, 11 + dy * 3 / 4, 11 + dx, 11 + dy);
            }
        }
        MenuIcon::Moon => {
            arc(&mut pen, 13, 11, 7, 45, 270);
            arc(&mut pen, 17, 7, 6, 90, 260);
        }
        MenuIcon::Link => {
            line(&mut pen, 8, 13, 13, 8);
            poly(&mut pen, &[(5, 10), (3, 12), (3, 16), (6, 19), (10, 19), (12, 17)]);
            poly(&mut pen, &[(17, 12), (19, 10), (19, 6), (16, 3), (12, 3), (10, 5)]);
        }
        MenuIcon::Colour => {
            // Colour temperature: a slider, not a second sun.
            line(&mut pen, 3, 11, 19, 11);
            rounded_rect(&mut pen, 8, 8, 6, 7);
            line(&mut pen, 11, 8, 11, 14);
        }
        MenuIcon::History => {
            // Recently played: the clock with a backward arrow.
            circle(&mut pen, 11, 11, 7);
            line(&mut pen, 11, 11, 11, 7);
            line(&mut pen, 11, 11, 14, 13);
            arc(&mut pen, 11, 11, 7, 180, 260);
            line(&mut pen, 3, 4, 6, 7);
            line(&mut pen, 3, 4, 3, 8);
        }
        MenuIcon::Content => {
            // Cloud sync content: the cloud with a three-line list.
            arc(&mut pen, 8, 11, 4, 180, 360);
            arc(&mut pen, 14, 11, 5, 0, 180);
            line(&mut pen, 4, 11, 4, 15);
            line(&mut pen, 19, 15, 19, 11);
            line(&mut pen, 4, 15, 8, 18);
            line(&mut pen, 8, 18, 16, 18);
            line(&mut pen, 16, 18, 19, 15);
            line(&mut pen, 6, 14, 16, 14);
            line(&mut pen, 6, 17, 16, 17);
            line(&mut pen, 6, 20, 12, 20);
        }
        MenuIcon::Router => {
            // The network entry: router body + antennas + signal.
            rounded_rect(&mut pen, 3, 14, 16, 5);
            line(&mut pen, 7, 14, 7, 9);
            line(&mut pen, 15, 14, 15, 9);
            arc(&mut pen, 11, 9, 5, 200, 340);
            arc(&mut pen, 11, 9, 8, 200, 340);
        }
        MenuIcon::MonitorSun => {
            // Screen and brightness: the monitor plus a sun.
            rounded_rect(&mut pen, 2, 8, 16, 10);
            line(&mut pen, 10, 18, 10, 20);
            line(&mut pen, 7, 20, 13, 20);
            circle(&mut pen, 17, 4, 2);
            line(&mut pen, 17, 0, 17, 1);
            line(&mut pen, 17, 7, 17, 8);
            line(&mut pen, 13, 4, 14, 4);
            line(&mut pen, 20, 4, 21, 4);
        }
        MenuIcon::ShaderGba => {
            // Wide screen = GBA, same scanline family.
            rounded_rect(&mut pen, 2, 5, 18, 10);
            line(&mut pen, 5, 9, 8, 12);
            line(&mut pen, 8, 12, 11, 9);
            line(&mut pen, 11, 9, 14, 12);
            line(&mut pen, 14, 12, 17, 9);
            line(&mut pen, 11, 15, 11, 17);
            line(&mut pen, 8, 19, 14, 19);
        }
        MenuIcon::ShaderGb => {
            // Tall screen = GB, same scanline family.
            rounded_rect(&mut pen, 5, 3, 12, 14);
            line(&mut pen, 7, 7, 9, 10);
            line(&mut pen, 9, 10, 11, 7);
            line(&mut pen, 11, 7, 13, 10);
            line(&mut pen, 13, 10, 15, 7);
            line(&mut pen, 11, 17, 11, 18);
            line(&mut pen, 8, 20, 14, 20);
        }
    }
    // The small badge at the bottom-right; the base glyph stays untouched.
    let letter = |pen: &mut dyn FnMut(i32, i32), x: i32, y: i32, which: char| {
        let rows: [&str; 5] = match which {
            'A' => [".#.", "#.#", "###", "#.#", "#.#"],
            'B' => ["##.", "#.#", "##.", "#.#", "##."],
            'L' => ["#..", "#..", "#..", "#..", "###"],
            'R' => ["##.", "#.#", "##.", "#.#", "#.#"],
            _ => ["...", "...", "...", "...", "..."],
        };
        for (ry, row) in rows.iter().enumerate() {
            for (cx, c) in row.chars().enumerate() {
                if c == '#' {
                    pen(x + cx as i32, y + ry as i32);
                }
            }
        }
    };
    match badge {
        Badge::None => {}
        Badge::Keycap => rounded_rect(&mut pen, 13, 14, 7, 6),
        Badge::Up => {
            line(&mut pen, 17, 19, 17, 14);
            line(&mut pen, 15, 16, 17, 14);
            line(&mut pen, 19, 16, 17, 14);
        }
        Badge::Down => {
            line(&mut pen, 17, 14, 17, 19);
            line(&mut pen, 15, 17, 17, 19);
            line(&mut pen, 19, 17, 17, 19);
        }
        Badge::Left => {
            line(&mut pen, 19, 17, 14, 17);
            line(&mut pen, 16, 15, 14, 17);
            line(&mut pen, 16, 19, 14, 17);
        }
        Badge::Right => {
            line(&mut pen, 14, 17, 19, 17);
            line(&mut pen, 17, 15, 19, 17);
            line(&mut pen, 17, 19, 19, 17);
        }
        Badge::CartGba => {
            rounded_rect(&mut pen, 11, 15, 9, 5);
            line(&mut pen, 14, 15, 14, 17);
        }
        Badge::CartGb => {
            rounded_rect(&mut pen, 15, 11, 5, 9);
            line(&mut pen, 15, 14, 17, 14);
        }
        Badge::CartGbc => {
            rounded_rect(&mut pen, 13, 13, 7, 7);
            line(&mut pen, 15, 15, 18, 15);
        }
        Badge::ScreenFc => {
            rounded_rect(&mut pen, 11, 14, 9, 5);
            line(&mut pen, 13, 19, 13, 21);
            line(&mut pen, 18, 19, 18, 21);
            line(&mut pen, 14, 16, 16, 16);
        }
        Badge::ScreenGb => {
            rounded_rect(&mut pen, 15, 11, 5, 8);
            line(&mut pen, 16, 14, 18, 14);
        }
        Badge::ScreenGbc => {
            rounded_rect(&mut pen, 13, 13, 7, 6);
            line(&mut pen, 15, 16, 18, 16);
        }
        Badge::ScreenSfc => {
            rounded_rect(&mut pen, 10, 15, 10, 4);
            pen(13, 17);
            pen(16, 17);
        }
        Badge::DpadUp => rect(&mut pen, 9, 3, 4, 6),
        Badge::DpadDown => rect(&mut pen, 9, 13, 4, 6),
        Badge::DpadLeft => rect(&mut pen, 3, 9, 6, 4),
        Badge::DpadRight => rect(&mut pen, 13, 9, 6, 4),
        Badge::LetterA | Badge::LetterB => {
            let (x, y) = if matches!(icon, MenuIcon::Turbo) {
                (14, 7)
            } else {
                (14, 8)
            };
            letter(&mut pen, x, y, if badge == Badge::LetterA { 'A' } else { 'B' });
        }
        Badge::LetterL => letter(&mut pen, 9, 7, 'L'),
        Badge::LetterR => letter(&mut pen, 9, 7, 'R'),
        Badge::Select => rect(&mut pen, 13, 10, 6, 3),
        Badge::Start => poly(&mut pen, &[(14, 8), (14, 13), (18, 10), (14, 8)]),
    }
}

fn rect(pen: &mut dyn FnMut(i32, i32), x: i32, y: i32, w: i32, h: i32) {
    for yy in y..y + h {
        for xx in x..x + w {
            pen(xx, yy);
        }
    }
}

fn rounded_rect(pen: &mut dyn FnMut(i32, i32), x: i32, y: i32, w: i32, h: i32) {
    for xx in x + 2..x + w - 2 {
        pen(xx, y);
        pen(xx, y + h - 1);
    }
    for yy in y + 2..y + h - 2 {
        pen(x, yy);
        pen(x + w - 1, yy);
    }
}

fn circle(pen: &mut dyn FnMut(i32, i32), cx: i32, cy: i32, r: i32) {
    let mut x = r;
    let mut y = 0;
    let mut err = 1 - r;
    while x >= y {
        for (dx, dy) in [
            (x, y),
            (y, x),
            (-y, x),
            (-x, y),
            (-x, -y),
            (-y, -x),
            (y, -x),
            (x, -y),
        ] {
            pen(cx + dx, cy + dy);
        }
        y += 1;
        if err < 0 {
            err += 2 * y + 1;
        } else {
            x -= 1;
            err += 2 * (y - x) + 1;
        }
    }
}

fn quick_text_face_at(label: &str, colour: [u8; 3], px: f32, min_px: f32) -> UndoFace {
    let Some(font) = text::label_font() else {
        return UndoFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    // Long labels shrink instead of being special-cased per language: English
    // strings are routinely wider than the Chinese they replace.
    let layout = text::fit(font, label, OUT_W as f32, 1, px, min_px);
    let set = layout
        .lines
        .iter()
        .map(|l| text::line_width(font, l, layout.px, layout.tracking))
        .fold(0.0, f32::max);
    let w = set.ceil() as u32 + 2 * MENU_PAD;
    let mut rgba = vec![0u8; (w * MENU_H * 4) as usize];
    text::draw_centred(&mut rgba, w, MENU_H, &layout, colour);
    UndoFace { rgba, w, h: MENU_H }
}

pub fn quick_caret_face(right: bool) -> UndoFace {
    quick_caret_face_state(right, false)
}

pub fn quick_caret_face_state(right: bool, lit: bool) -> UndoFace {
    let glyph = if right { '\u{f0da}' } else { '\u{f0d9}' };
    let ink = if lit {
        [0x6C, 0x6C, 0x72]
    } else {
        [0xA6, 0xA6, 0xAD]
    };
    let (Some(symbols), Some(label)) = (crate::icon::symbols_font(), text::label_font()) else {
        return UndoFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let (m, cov) = symbols.rasterize(glyph, CARET_PX);
    let (w, h) = (m.width as u32, MENU_H);
    let centre = match label.horizontal_line_metrics(MENU_PX) {
        Some(v) => {
            (h as f32 - v.new_line_size) / 2.0 + v.ascent
                - label.metrics('H', MENU_PX).height as f32 / 2.0
        }
        None => h as f32 / 2.0,
    };
    let top = (centre - m.height as f32 / 2.0).round() as i32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for gy in 0..m.height {
        let y = top + gy as i32;
        if y < 0 || y >= h as i32 {
            continue;
        }
        for gx in 0..m.width {
            let at = ((y as u32 * w + gx as u32) * 4) as usize;
            let a = cov[gy * m.width + gx];
            rgba[at..at + 4].copy_from_slice(&[ink[0], ink[1], ink[2], a]);
        }
    }
    UndoFace { rgba, w, h }
}

pub fn quick_legend_faces(lang: Language) -> [UndoFace; 3] {
    [
        hint_face("B", slot_i18n::t(lang, "device.quick.legend.back")),
        hint_face("A", slot_i18n::t(lang, "device.quick.legend.open_adjust")),
        hint_face_ink(
            "M",
            slot_i18n::t(lang, "device.quick.legend.close"),
            [0x2A, 0x24, 0x20],
            [0xF2, 0xEC, 0xE2],
            [0xB3, 0xA5, 0x96],
        ),
    ]
}

/// Old warm menu ground: warm charcoal top fading into a darker foot.
pub fn quick_background_face() -> UndoFace {
    let mut face = UndoFace {
        rgba: vec![0u8; (OUT_W * OUT_H * 4) as usize],
        w: OUT_W,
        h: OUT_H,
    };
    for px in face.rgba.chunks_exact_mut(4) {
        px.copy_from_slice(&[0x1B, 0x1B, 0x1E, 0xFF]);
    }
    face
}

pub struct QuickMenuFaces {
    pub background: (TexId, u32, u32),
    pub cards: [(TexId, u32, u32); 2],
    pub labels: Vec<[(TexId, u32, u32); 2]>,
    pub values: Vec<[(TexId, u32, u32); 2]>,
    /// [left, right] × [unselected, selected]
    pub carets: [[(TexId, u32, u32); 2]; 2],
    pub icons: Vec<[(TexId, u32, u32); 2]>,
    /// One title per page, indexed by `QuickRow::page_index`.
    pub titles: [(TexId, u32, u32); QuickRow::QUICK_PAGES],
    /// Per-page `SETTINGS / NN` coordinate.
    pub page_metas: [(TexId, u32, u32); QuickRow::QUICK_PAGES],
    /// 01–07 row index faces.
    pub numbers: Vec<(TexId, u32, u32)>,
    pub legend: [(TexId, u32); 3],
}

pub struct QuickMenu<'a> {
    pub row: QuickRow,
    pub values: [Option<QuickValue>; QuickRow::ALL.len()],
    pub clock: Option<[(TexId, u32, u32); 2]>,
    /// Mask values are file names, so they arrive as ready faces.
    pub fc_mask_value: Option<(TexId, u32, u32)>,
    pub gb_mask_value: Option<(TexId, u32, u32)>,
    pub gbc_mask_value: Option<(TexId, u32, u32)>,
    pub sfc_mask_value: Option<(TexId, u32, u32)>,
    pub mods_value: Option<[(TexId, u32, u32); 2]>,
    pub version_value: Option<[(TexId, u32, u32); 2]>,
    pub faces: Option<&'a QuickMenuFaces>,
}

impl QuickMenu<'_> {
    pub fn draw(&self, out: &mut Vec<Draw>) {
        out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: OUT_W as f32,
            h: OUT_H as f32,
            colour: colour(switch_style::bg()),
        });
        if let Some(faces) = self.faces {
            let (bg_tex, bg_w, bg_h) = faces.background;
            push(out, bg_tex, 0.0, 0.0, bg_w, bg_h);
        }
        out.push(Draw::Rect {
            x: 28.0,
            y: 40.0,
            w: OUT_W as f32 - 56.0,
            h: 1.0,
            colour: colour([0x3A, 0x3A, 0x42, 255]),
        });
        let Some(faces) = self.faces else {
            return;
        };
        let page = self.row.page_index();
        if let Some(&(tex, w, h)) = faces.titles.get(page) {
            push(out, tex, 14.0, 42.0, w, h);
        }
        let (meta_tex, meta_w, meta_h) = faces.page_metas[page];
        push(
            out,
            meta_tex,
            OUT_W as f32 - 32.0 - meta_w as f32,
            43.0,
            meta_w,
            meta_h,
        );
        let card_x = QUICK_EDGE;
        let card_right = card_x + faces.cards[0].1 as f32;
        let page = self.row.page();
        let (offset, take) = page_window(page, self.row.position());
        for (at, &row) in page.iter().enumerate().skip(offset).take(take) {
            let y = CARD_Y + QUICK_PITCH * (at - offset) as f32;
            let lit = row == self.row;
            let state = lit as usize;
            let (card_tex, card_w, card_h) = faces.cards[state];
            if lit {
                push(out, card_tex, card_x, y, card_w, card_h);
                out.push(Draw::Rect {
                    x: card_x - 8.0,
                    y: y + 12.0,
                    w: 3.0,
                    h: 24.0,
                    colour: colour([0xC4, 0x3B, 0x32, 255]),
                });
            }
            if let Some(&(tex, w, h)) = faces.icons.get(row.index()).map(|f| &f[state]) {
                let _ = ICON_BOX;
                push(out, tex, ICON_X, y + (card_h as f32 - h as f32) / 2.0, w, h);
            }
            if let Some(&(tex, w, h)) = faces.labels.get(row.index()).map(|f| &f[state]) {
                push(out, tex, LABEL_X, y + (card_h as f32 - h as f32) / 2.0, w, h);
            }
            let value = match row {
                QuickRow::DateTime => self.clock.map(|c| c[state]),
                QuickRow::FcMask => self.fc_mask_value,
                QuickRow::GbMask => self.gb_mask_value,
                QuickRow::GbcMask => self.gbc_mask_value,
                QuickRow::SfcMask => self.sfc_mask_value,
                QuickRow::Mods => self.mods_value.map(|v| v[state]),
                QuickRow::SystemUpgrade => self.version_value.map(|v| v[state]),
                _ => self.values[row.index()]
                    .and_then(|v| faces.values.get(v.index()))
                    .map(|v| v[state]),
            };
            let (chevron_tex, cw, ch) = faces.carets[1][state];
            let cx = card_right - RIGHT_PAD - cw as f32;
            if let Some((tex, w, h)) = value {
                push(
                    out,
                    tex,
                    cx - 8.0 - w as f32,
                    y + (card_h as f32 - h as f32) / 2.0,
                    w,
                    h,
                );
            }
            push(
                out,
                chevron_tex,
                cx,
                y + (card_h as f32 - ch as f32) / 2.0,
                cw,
                ch,
            );
        }

        // Old warm menu keybar: B 返回 / A 打开·调整 on the left, M 关闭 on
        // the right.  The faces still contain the keycap.
        let legend_y = LEGEND_Y;
        let mut x = 28.0;
        for i in 0..2 {
            let (tex, w) = faces.legend[i];
            let scaled_w = (w as f32 * 0.72) as u32;
            push(out, tex, x, legend_y, scaled_w, 20);
            x += scaled_w.saturating_sub(2) as f32 + 12.0;
        }
        let (close_tex, close_w) = faces.legend[2];
        let close_w = (close_w as f32 * 0.72) as u32;
        push(
            out,
            close_tex,
            OUT_W as f32 - 28.0 - close_w as f32,
            legend_y,
            close_w,
            20,
        );
    }
}

fn colour(c: [u8; 4]) -> [f32; 4] {
    [
        c[0] as f32 / 255.0,
        c[1] as f32 / 255.0,
        c[2] as f32 / 255.0,
        c[3] as f32 / 255.0,
    ]
}

pub fn row_top(row: QuickRow) -> f32 {
    CARD_Y + QUICK_PITCH * row.position() as f32
}

const MAX_ROWS: usize = 6;

fn page_window(page: &[QuickRow], selected: usize) -> (usize, usize) {
    let take = page.len().min(MAX_ROWS);
    if take == 0 {
        return (0, 0);
    }
    let offset = selected
        .saturating_sub(MAX_ROWS - 1)
        .min(page.len().saturating_sub(take));
    (offset, take)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_has_distinct_menu_icons() {
        let pages: [&[QuickRow]; 16] = [
            &QuickRow::MAIN,
            &QuickRow::DISPLAY,
            &QuickRow::DISPLAY_PANEL,
            &QuickRow::DISPLAY_EFFECTS,
            &QuickRow::MASKS,
            &QuickRow::SHELF,
            &QuickRow::SHELF_LOOK,
            &QuickRow::SHELF_KEYS,
            &QuickRow::GAME,
            &QuickRow::BUTTON_SETTINGS,
            &QuickRow::CONTROLS,
            &QuickRow::SHORTCUTS,
            &QuickRow::NETWORK,
            &QuickRow::SYSTEM_SETTINGS,
            &QuickRow::SAVES,
            QuickRow::THEMES,
        ];
        for (page_index, rows) in pages.iter().enumerate() {
            let mut seen: Vec<(MenuIcon, Badge)> = Vec::new();
            for row in *rows {
                let key = (row.menu_icon(), row.menu_badge());
                assert!(
                    !seen.contains(&key),
                    "page {page_index} repeats {key:?} for {row:?}"
                );
                seen.push(key);
            }
        }
    }

    #[test]
    fn a_thirteen_row_page_scrolls_to_keep_the_selection_on_screen() {
        assert_eq!(page_window(&QuickRow::SHORTCUTS, 0), (0, 6));
        assert_eq!(page_window(&QuickRow::SHORTCUTS, 7), (2, 6));
        assert_eq!(page_window(&QuickRow::SHORTCUTS, 8), (3, 6));
        assert_eq!(page_window(&QuickRow::SHORTCUTS, 12), (7, 6));
    }

    #[test]
    fn every_quick_row_indexes_into_the_value_table() {
        for row in QuickRow::ALL {
            assert!(
                row.index() < QuickRow::ALL.len(),
                "{row:?} is outside the value table"
            );
        }
    }

    #[test]
    fn the_main_page_never_scrolls() {
        assert_eq!(page_window(&QuickRow::MAIN, 5), (0, 6));
        // 主题 moved under 画面, so that page must still fit without scrolling.
        assert_eq!(page_window(&QuickRow::DISPLAY, 3), (0, 4));
    }

    #[test]
    fn the_language_row_is_appended_so_no_earlier_index_moves() {
        assert_eq!(*QuickRow::ALL.last().expect("ALL is not empty"), QuickRow::Language);
        // `index()` is the enum discriminant and the key into the value table:
        // the new row must stay inside it, which is why `Language` is declared
        // before `SplitLink` even though it is last in `ALL`.
        assert!(QuickRow::Language.index() < QuickRow::ALL.len());
        assert_eq!(QuickRow::Language.page(), &QuickRow::SYSTEM_SETTINGS[..]);
        assert!(QuickRow::SYSTEM_SETTINGS.contains(&QuickRow::Language));
        // 日期与时间 must stay first: `page_index()` reads `page()[0]`.
        assert_eq!(QuickRow::SYSTEM_SETTINGS[0], QuickRow::DateTime);
        assert_eq!(
            QuickRow::Language.page_index(),
            QuickRow::DateTime.page_index()
        );
        assert_eq!(QuickRow::Language.parent(), Some(QuickRow::SystemSettings));
        // A value row, not a page: it must not steal the A button.
        assert!(!QuickRow::Language.opens());
    }

    #[test]
    fn the_language_values_are_last_and_read_in_the_requested_language() {
        assert_eq!(*QuickValue::ALL.last().expect("ALL is not empty"), QuickValue::LanguageEn);
        assert_eq!(QuickValue::LanguageZh.text(Language::ZhCn), "中文");
        assert_eq!(QuickValue::LanguageZh.text(Language::EnUs), "中文");
        assert_eq!(QuickValue::LanguageEn.text(Language::ZhCn), "English");
        assert_eq!(QuickValue::LanguageEn.text(Language::EnUs), "English");
        assert_eq!(QuickRow::Language.label(Language::ZhCn), "语言");
        assert_eq!(QuickRow::Language.label(Language::EnUs), "Language");
    }

    #[test]
    fn page_titles_follow_the_language() {
        for (index, key) in QuickRow::PAGE_KEYS.iter().enumerate() {
            let zh = QuickRow::page_title_at(index, Language::ZhCn);
            let en = QuickRow::page_title_at(index, Language::EnUs);
            assert_eq!(zh, slot_i18n::t(Language::ZhCn, key));
            assert_eq!(en, slot_i18n::t(Language::EnUs, key));
            assert_ne!(zh, en, "page title {key} did not switch language");
        }
    }
}

fn push(out: &mut Vec<Draw>, tex: TexId, x: f32, y: f32, w: u32, h: u32) {
    out.push(Draw::Tex {
        x: x.round(),
        y: y.round(),
        w: w as f32,
        h: h as f32,
        tex,
        alpha: 1.0,
    });
}
