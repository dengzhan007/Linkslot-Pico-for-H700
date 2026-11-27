use slot_gfx::{Draw, TexId, OUT_W};
use slot_store::{BLUE_LIGHT_MAX, BRIGHTNESS_MAX, VOLUME_MAX};

use crate::icon::icon_box;
use crate::switch_style as sw;
use crate::{Badge, Icon, Toast};

pub type Millis = u64;

pub const HUD_MS: Millis = 1500;
const FADE_MS: Millis = 250;


pub const PLATE_H: f32 = 40.0;
const HUD_PANEL_W: f32 = 378.0;
/// The volume/brightness/rewind bar's height; the top notification stack uses
/// it to place the rows under each other.
pub const HUD_BAR_H: f32 = 40.0;

pub const HUD_ICON_PX: f32 = 24.0;
/// The control bar's glyph is the small 16px version of the HUD icon.
const BAR_ICON_PX: f32 = 16.0;
pub const HUD_INK: [u8; 3] = [0xf5, 0xf2, 0xef];
const BADGE_MARGIN: f32 = 12.0;
/// The small notification's left marker.
const MARKER_W: f32 = 3.0;
const MARKER_H: f32 = 24.0;
const BAR_X: f32 = 14.0;
const BAR_Y: f32 = 32.0;
const BAR_W: f32 = 350.0;
const BAR_H: f32 = 2.0;

pub fn badge_at(w: f32, h: f32) -> (f32, f32) {
    (OUT_W as f32 - BADGE_MARGIN - w, (PLATE_H - h) / 2.0)
}

pub(crate) const TRACK: [f32; 4] = [1.0, 1.0, 1.0, 0.18];
pub(crate) const FILL: [f32; 4] = [
    HUD_INK[0] as f32 / 255.0,
    HUD_INK[1] as f32 / 255.0,
    HUD_INK[2] as f32 / 255.0,
    1.0,
];

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum HudKind {
    #[default]
    Brightness,
    BlueLight,
    Volume,
    Rewind,
}

impl HudKind {
    pub fn icon(self, value: u8, muted: bool, headphones: bool) -> Icon {
        match self {
            HudKind::Brightness => Icon::Brightness,
            HudKind::BlueLight => Icon::BlueLight,
            HudKind::Volume if headphones && muted => Icon::HeadphonesMuted,
            HudKind::Volume if headphones => Icon::Headphones,
            HudKind::Volume if muted => Icon::VolumeMuted,
            HudKind::Volume if value == 0 => Icon::VolumeZero,
            HudKind::Volume => Icon::Volume,
            HudKind::Rewind => Icon::Rewind,
        }
    }

    fn max(self) -> u8 {
        match self {
            HudKind::Brightness => BRIGHTNESS_MAX,
            HudKind::BlueLight => BLUE_LIGHT_MAX,
            HudKind::Volume => VOLUME_MAX,
            HudKind::Rewind => 100,
        }
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum FfState {
    #[default]
    Off,
    Held,
    Latched,
}

pub fn ff_badge(state: FfState) -> Option<Icon> {
    match state {
        FfState::Off => None,
        FfState::Held => Some(Icon::FastForward),
        FfState::Latched => Some(Icon::FastForwardLatched),
    }
}

pub const LINK_HOST_INK: [u8; 3] = [0x8a, 0x74, 0xcf];
pub const LINK_JOIN_INK: [u8; 3] = [0xb2, 0xb2, 0xb8];

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum LinkBadge {
    #[default]
    Off,
    Hosting,
    Joined,
    HostingLost,
    JoinedLost,
}

impl LinkBadge {
    pub const FACES: [LinkBadge; 4] = [
        LinkBadge::Hosting,
        LinkBadge::Joined,
        LinkBadge::HostingLost,
        LinkBadge::JoinedLost,
    ];

    pub fn face_index(self) -> Option<usize> {
        LinkBadge::FACES.iter().position(|b| *b == self)
    }

    pub fn badge(self) -> Option<Badge> {
        match self {
            LinkBadge::Off => None,
            LinkBadge::Hosting | LinkBadge::Joined => Some(Badge::Link),
            LinkBadge::HostingLost | LinkBadge::JoinedLost => Some(Badge::LinkBroken),
        }
    }

    pub fn colour(self) -> Option<[u8; 3]> {
        match self {
            LinkBadge::Off => None,
            LinkBadge::Hosting | LinkBadge::HostingLost => Some(LINK_HOST_INK),
            LinkBadge::Joined | LinkBadge::JoinedLost => Some(LINK_JOIN_INK),
        }
    }
}

#[derive(Default)]
pub struct Hud {
    pub kind: HudKind,
    pub value: u8,
    pub shown_at: Option<Millis>,
    held: bool,
    muted: bool,
    headphones: bool,
    ff: FfState,
    said: Option<(Toast, Millis)>,
    alarm: Option<Millis>,
    icons: Vec<TexId>,
    toasts: Vec<TexId>,
    panel: Option<TexId>,
    alarm_face: Option<TexId>,
    labels: Vec<(TexId, u32, u32)>,
    values: Vec<(TexId, u32, u32)>,
    link: LinkBadge,
    link_faces: Vec<TexId>,
}

impl Hud {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_icons(&mut self, icons: Vec<TexId>) {
        self.icons = icons;
    }

    pub fn set_toasts(&mut self, toasts: Vec<TexId>) {
        self.toasts = toasts;
    }

    pub fn set_panel(&mut self, panel: TexId) {
        self.panel = Some(panel);
    }

    /// The four kind labels (brightness / blue light / volume / rewind), in
    /// [`HudKind`] order; the frontend uploads them once per language.
    pub fn set_labels(&mut self, labels: Vec<(TexId, u32, u32)>) {
        self.labels = labels;
    }

    /// The 0..=100 value faces; the control bar picks the current value's face
    /// instead of drawing four separate digit textures.
    pub fn set_value_faces(&mut self, values: Vec<(TexId, u32, u32)>) {
        self.values = values;
    }

    pub fn toast(&mut self, toast: Toast, now: Millis) {
        self.said = Some((toast, now));
    }

    pub fn set_alarm(&mut self, on: bool, now: Millis) {
        self.alarm = match (on, self.alarm) {
            (true, Some(since)) => Some(since),
            (true, None) => Some(now),
            (false, _) => None,
        };
    }

    pub fn alarmed(&self) -> bool {
        self.alarm.is_some()
    }

    pub fn toast_visible(&self, now: Millis) -> bool {
        self.toast_alpha(now) > 0.0
    }

    pub fn said(&self, now: Millis) -> Option<Toast> {
        self.toast_visible(now)
            .then(|| self.said.map(|(t, _)| t))
            .flatten()
    }

    /// The toast still on screen and its fade alpha.  The top notification
    /// layer (frontend) blits it; the HUD itself only owns the bar/badges and
    /// the storage-card alarm.
    pub fn toast_view(&self, now: Millis) -> Option<(Toast, f32)> {
        let (toast, _) = self.said?;
        let alpha = self.toast_alpha(now);
        (alpha > 0.0).then_some((toast, alpha))
    }

    /// The uploaded face for `toast`, if the boot pass supplied it.
    pub fn toast_face(&self, toast: Toast) -> Option<TexId> {
        self.toasts.get(toast.index()).copied()
    }

    pub fn show(&mut self, kind: HudKind, value: u8, muted: bool, now: Millis) {
        self.kind = kind;
        self.value = value;
        self.muted = muted;
        self.shown_at = Some(now);
        self.held = kind == HudKind::Rewind;
    }

    pub fn release_rewind(&mut self) {
        if self.kind == HudKind::Rewind {
            self.shown_at = None;
        }
        self.held = false;
    }

    pub fn glyph(&self) -> Icon {
        self.kind.icon(self.value, self.muted, self.headphones)
    }

    pub fn set_headphones(&mut self, on: bool) {
        self.headphones = on;
    }

    pub fn badge(&self) -> Option<Icon> {
        ff_badge(self.ff)
    }

    pub fn set_ff(&mut self, ff: FfState) {
        self.ff = ff;
    }

    pub fn set_link(&mut self, badge: LinkBadge) {
        self.link = badge;
    }

    pub fn link(&self) -> LinkBadge {
        self.link
    }

    pub fn set_link_faces(&mut self, faces: Vec<TexId>) {
        self.link_faces = faces;
    }

    pub fn visible(&self, now: Millis) -> bool {
        self.bar_alpha(now).is_some()
    }

    /// The volume/brightness/rewind bar's fade alpha while it is on screen.
    pub fn bar_alpha(&self, now: Millis) -> Option<f32> {
        let alpha = self.alpha(now);
        (alpha > 0.0).then_some(alpha)
    }

    /// The overlay that sits above every screen: only the link and fast-forward
    /// badges stay here.  The bar/alarm are blitted by the caller in the top
    /// notification stack ([`Hud::draw_bar_at`] / [`Hud::draw_alarm_at`]).
    pub fn draw(&self, out: &mut Vec<Draw>) {
        let link = self
            .link
            .face_index()
            .and_then(|i| self.link_faces.get(i).copied());
        let ff = ff_badge(self.ff).and_then(|i| self.icons.get(i.index()).copied());
        if let Some(tex) = link.or(ff) {
            self.place_badge(tex, out);
        }
    }

    /// Draw the volume/brightness/rewind control as the small 378x40
    /// notification shell: a 3x24 marker, a 16px glyph, a 13px kind label, an
    /// 18px value on the right and a 2px track along the bottom.
    pub fn draw_bar_at(&self, alpha: f32, y: f32, out: &mut Vec<Draw>) {
        let x = (OUT_W as f32 - HUD_PANEL_W) / 2.0;
        if let Some(tex) = self.panel {
            out.push(Draw::Tex {
                x,
                y,
                w: HUD_PANEL_W,
                h: HUD_BAR_H,
                tex,
                alpha,
            });
        } else {
            let border = sw::surface_border();
            let surface = sw::surface();
            out.push(Draw::Rect {
                x,
                y,
                w: HUD_PANEL_W,
                h: HUD_BAR_H,
                colour: faded(
                    [
                        border[0] as f32 / 255.0,
                        border[1] as f32 / 255.0,
                        border[2] as f32 / 255.0,
                        1.0,
                    ],
                    alpha,
                ),
            });
            out.push(Draw::Rect {
                x: x + 1.0,
                y: y + 1.0,
                w: HUD_PANEL_W - 2.0,
                h: HUD_BAR_H - 2.0,
                colour: faded(
                    [
                        surface[0] as f32 / 255.0,
                        surface[1] as f32 / 255.0,
                        surface[2] as f32 / 255.0,
                        1.0,
                    ],
                    alpha,
                ),
            });
        }
        // The shared small-notification marker at the left.
        out.push(Draw::Rect {
            x: x + 7.0,
            y: y + 8.0,
            w: MARKER_W,
            h: MARKER_H,
            colour: faded(FILL, alpha),
        });
        if let Some(tex) = self.icon() {
            out.push(Draw::Tex {
                x: x + 14.0,
                y: y + (HUD_BAR_H - BAR_ICON_PX) / 2.0,
                w: BAR_ICON_PX,
                h: BAR_ICON_PX,
                tex,
                alpha,
            });
        }
        if let Some((tex, w, h)) = self.labels.get(self.kind as usize).copied() {
            out.push(Draw::Tex {
                x: x + 38.0,
                y: y + (HUD_BAR_H - h as f32) / 2.0,
                w: w as f32,
                h: h as f32,
                tex,
                alpha,
            });
        }
        if let Some((tex, w, h)) = self.values.get(self.value as usize).copied() {
            out.push(Draw::Tex {
                x: x + HUD_PANEL_W - 16.0 - w as f32,
                y: y + (HUD_BAR_H - h as f32) / 2.0,
                w: w as f32,
                h: h as f32,
                tex,
                alpha,
            });
        }
        out.push(Draw::Rect {
            x: x + BAR_X,
            y: y + BAR_Y,
            w: BAR_W,
            h: BAR_H,
            colour: faded(TRACK, alpha),
        });
        let filled = BAR_W * self.fraction();
        if filled > 0.0 {
            out.push(Draw::Rect {
                x: x + BAR_X,
                y: y + BAR_Y,
                w: filled,
                h: BAR_H,
                colour: faded(FILL, alpha),
            });
        }
    }

    fn place_badge(&self, tex: TexId, out: &mut Vec<Draw>) {
        let (w, h) = icon_box(HUD_ICON_PX);
        let (w, h) = (w as f32, h as f32);
        let (x, y) = badge_at(w, h);
        out.push(Draw::Tex {
            x,
            y,
            w,
            h,
            tex,
            alpha: 1.0,
        });
    }

    /// The persistent storage-card alarm is a large notification card, not a
    /// toast: the frontend uploads its 480x82 face here.
    pub fn set_alarm_face(&mut self, face: TexId) {
        self.alarm_face = Some(face);
    }

    /// Draw the persistent storage-card alarm with its top edge at `y`.
    pub fn draw_alarm_at(&self, y: f32, out: &mut Vec<Draw>) {
        let (w, h) = (crate::notice::NOTICE_W, crate::notice::NOTICE_H);
        let x = (OUT_W as f32 - w) / 2.0;
        if let Some(tex) = self.alarm_face {
            out.push(Draw::Tex {
                x,
                y,
                w,
                h,
                tex,
                alpha: 1.0,
            });
            return;
        }
        let border = sw::surface_border();
        let surface = sw::surface();
        out.push(Draw::Rect {
            x,
            y,
            w,
            h,
            colour: [
                border[0] as f32 / 255.0,
                border[1] as f32 / 255.0,
                border[2] as f32 / 255.0,
                1.0,
            ],
        });
        out.push(Draw::Rect {
            x: x + 1.0,
            y: y + 1.0,
            w: w - 2.0,
            h: h - 2.0,
            colour: [
                surface[0] as f32 / 255.0,
                surface[1] as f32 / 255.0,
                surface[2] as f32 / 255.0,
                1.0,
            ],
        });
    }

    fn icon(&self) -> Option<TexId> {
        self.icons.get(self.glyph().index()).copied()
    }

    fn fraction(&self) -> f32 {
        let max = self.kind.max();
        if max == 0 {
            return 0.0;
        }
        self.value.min(max) as f32 / max as f32
    }

    fn alpha(&self, now: Millis) -> f32 {
        let Some(shown) = self.shown_at else {
            return 0.0;
        };
        if self.held {
            return 1.0;
        }
        fade(shown, now)
    }

    fn toast_alpha(&self, now: Millis) -> f32 {
        match self.said {
            Some((_, shown)) => fade(shown, now),
            None => 0.0,
        }
    }
}

fn fade(shown: Millis, now: Millis) -> f32 {
    let age = now.saturating_sub(shown);
    if age >= HUD_MS {
        return 0.0;
    }
    ((HUD_MS - age) as f32 / FADE_MS as f32).min(1.0)
}

fn faded(colour: [f32; 4], alpha: f32) -> [f32; 4] {
    [colour[0], colour[1], colour[2], colour[3] * alpha]
}
