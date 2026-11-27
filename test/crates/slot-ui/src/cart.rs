use std::sync::OnceLock;

use slot_store::gb::Class;
use slot_store::{Cart, CartModel, Outline, Platform};

use crate::art;
use crate::plate::UndoFace;
use crate::shell::{shell_for, Finish, Shell};
use crate::silhouette::{
    cart_depth, cart_mask, detail_mask, fc_cart_depth, fc_cart_mask, fc_detail_mask,
    gb_cart_depth, gb_cart_mask, gb_detail_mask, Detail, GbShell,
};
use crate::switch_style as sw;
use crate::text;

pub const CART_W: u32 = 360;
/// The reference project's Advance face is 360x202; the old vector shell was
/// 360x212.  Matching the art's native aspect is what removes the subtle
/// vertical stretch people notice on the cart.
pub const CART_H: u32 = 202;

pub const GB_CART_W: u32 = 240;
pub const GB_CART_H: u32 = 276;

/// The reference project's Famicom cassette and its 240-wide seated footprint.
/// The face is 360x299 (1.20) so the 1.46 label window of a real Famicom cart
/// label fits without cropping.
pub const FC_BASE_W: u32 = 240;
/// The seated footprint keeps the cart's own 1.20 aspect: 240 / (360/299).
pub const FC_BASE_H: u32 = 199;
pub const SFC_CART_W: u32 = 360;
pub const SFC_CART_H: u32 = 252;
/// The reference Mega Drive cassette: 360x244, with its tall printed label.
pub const MD_CART_W: u32 = 360;
pub const MD_CART_H: u32 = 244;

pub const FC_CART_W: u32 = 360;
pub const FC_CART_H: u32 = 299;

/// The NES Game Pak uses the same 360x299 display canvas as the Famicom
/// cassette.  The embedded art is the upstream 300x334 `66SLOT_CART_NES.png`
/// scaled into that canvas; its magenta label window is this rectangle.
pub const NES_LABEL_X: u32 = 154;
pub const NES_LABEL_Y: u32 = 2;
pub const NES_LABEL_W: u32 = 121;
pub const NES_LABEL_H: u32 = 200;
const NES_MAX_PX: f32 = NES_LABEL_H as f32 / (3.0 * 1.36);
/// The Advance label window in the reference art: x 57..303, y 46..175.
pub const fn label_panel(w: u32, h: u32) -> (u32, u32, u32, u32) {
    (
        (w * 57 + 180) / 360,
        (h * 46 + 100) / 202,
        (w * 303 + 180) / 360,
        (h * 175 + 100) / 202,
    )
}

pub const LABEL_X: u32 = label_panel(CART_W, CART_H).0;
pub const LABEL_Y: u32 = label_panel(CART_W, CART_H).1;
pub const LABEL_W: u32 = label_panel(CART_W, CART_H).2 - LABEL_X;
pub const LABEL_H: u32 = label_panel(CART_W, CART_H).3 - LABEL_Y;

/// The reference Game Boy label window: x 27..213, y 77..243 on 240x276.
pub const fn gb_label_panel(w: u32, h: u32) -> (u32, u32, u32, u32) {
    (
        (w * 27 + 120) / 240,
        (h * 77 + 138) / 276,
        (w * 213 + 120) / 240,
        (h * 243 + 138) / 276,
    )
}

/// The reference Game Boy Color label window sits lower than the DMG one.
pub const fn gbc_label_panel(w: u32, h: u32) -> (u32, u32, u32, u32) {
    (
        (w * 27 + 120) / 240,
        (h * 89 + 138) / 276,
        (w * 213 + 120) / 240,
        (h * 255 + 138) / 276,
    )
}

pub const GB_LABEL_X: u32 = gb_label_panel(GB_CART_W, GB_CART_H).0;
pub const GB_LABEL_Y: u32 = gb_label_panel(GB_CART_W, GB_CART_H).1;
pub const GB_LABEL_W: u32 = gb_label_panel(GB_CART_W, GB_CART_H).2 - GB_LABEL_X;
pub const GB_LABEL_H: u32 = gb_label_panel(GB_CART_W, GB_CART_H).3 - GB_LABEL_Y;

/// The reference Famicom label window in the 360x299 art: 31..327 across,
/// 31..233 down (296x202 = 1.47, the shape of a real cart label).
pub const fn fc_label_panel(w: u32, h: u32) -> (u32, u32, u32, u32) {
    (
        (w * 31 + 180) / 360,
        (h * 31 + 150) / 299,
        (w * 327 + 180) / 360,
        (h * 233 + 150) / 299,
    )
}

pub const FC_LABEL_X: u32 = fc_label_panel(FC_CART_W, FC_CART_H).0;
pub const FC_LABEL_Y: u32 = fc_label_panel(FC_CART_W, FC_CART_H).1;
pub const FC_LABEL_W: u32 = fc_label_panel(FC_CART_W, FC_CART_H).2 - FC_LABEL_X;
pub const FC_LABEL_H: u32 = fc_label_panel(FC_CART_W, FC_CART_H).3 - FC_LABEL_Y;

const PAD: u32 = 10;
const MAX_LINES: usize = 3;
const MAX_PX: f32 = LABEL_H as f32 / (MAX_LINES as f32 * 1.36);
const GB_MAX_PX: f32 = (GB_LABEL_H - 2 * PAD) as f32 / MAX_LINES as f32;
const MIN_PX: f32 = 10.0;
const FC_MAX_PX: f32 = FC_LABEL_H as f32 / (MAX_LINES as f32 * 1.36);

const RIM: u32 = (4 * CART_W + SEATED_W / 2) / SEATED_W;
const GB_RIM: u32 = (7 * GB_CART_W + SEATED_W / 2) / SEATED_W;

pub struct CartFace {
    pub rgba: Vec<u8>,
    pub w: u32,
    pub h: u32,
}

struct Spec {
    w: u32,
    h: u32,
    label: (u32, u32, u32, u32),
    mask: &'static [u8],
    depth: &'static [u8],
    detail: &'static Detail,
    max_px: f32,
    max_h: f32,
    rim: u32,
    board: Board,
}

struct Board {
    x: (f32, f32),
    top: f32,
    pins: (f32, f32),
    contacts_from: f32,
    traces_from: f32,
}

#[derive(Clone, Copy)]
enum Shape {
    Gba,
    Gb(GbShell),
    /// Famicom / NES.  The reference project's orange 360x241 cassette; the
    /// embedded art supplies the plastic and its moulding.
    Fc,
    /// NES Game Pak: a vertical grey cartridge on the same display canvas.
    Nes,
    /// Super Famicom / SNES: the grey 360x252 cassette with a wide label well.
    Sfc,
    /// Mega Drive / Genesis: the reference 360x244 cassette with its tall
    /// printed label.
    Md,
}

pub fn gb_shell_of(cart: &Cart) -> Option<GbShell> {
    match cart.platform {
        Platform::Gba | Platform::Fc | Platform::Sfc | Platform::Md => None,
        Platform::Gb | Platform::Gbc => Some(match cart.shell.map(|c| c.outline) {
            Some(Outline::Notched) => GbShell::Notched,
            Some(Outline::Rounded) => GbShell::Rounded,
            _ => match slot_store::gb::class(&cart.rom) {
                Class::ColourOnly => GbShell::Rounded,
                Class::Original | Class::DualMode => GbShell::Notched,
            },
        }),
    }
}

fn shape_of(cart: &Cart) -> Shape {
    match gb_shell_of(cart) {
        None if cart.platform == Platform::Fc && cart.model == CartModel::Nes => Shape::Nes,
        None if cart.platform == Platform::Fc => Shape::Fc,
        None if cart.platform == Platform::Sfc => Shape::Sfc,
        None if cart.platform == Platform::Md => Shape::Md,
        None => Shape::Gba,
        Some(shell) => Shape::Gb(shell),
    }
}

fn spec(shape: Shape) -> Spec {
    match shape {
        Shape::Gba => gba_spec(),
        // Its own body, label box and moulding: a Famicom cassette, not the
        // Advance's landscape cart.
        Shape::Fc => fc_spec(),
        // NES uses the same display canvas, but a portrait label window.
        Shape::Nes => nes_spec(),
        // The embedded `sfc_cart.png` supplies the real shell; the spec is a
        // fallback for a missing asset.
        Shape::Sfc => fc_spec(),
        // The embedded `md_cart.png` supplies the real shell; the spec is a
        // fallback for a missing asset.
        Shape::Md => fc_spec(),
        Shape::Gb(shell) => gb_spec(shell),
    }
}

fn fc_spec() -> Spec {
    Spec {
        w: FC_CART_W,
        h: FC_CART_H,
        label: (FC_LABEL_X, FC_LABEL_Y, FC_LABEL_W, FC_LABEL_H),
        mask: fc_cart_mask(),
        depth: fc_cart_depth(),
        detail: fc_detail_mask(),
        max_px: FC_MAX_PX,
        max_h: f32::INFINITY,
        rim: RIM,
        board: Board {
            x: (0.135, 0.875),
            top: FC_LABEL_Y as f32 / FC_CART_H as f32,
            pins: (0.17, 0.84),
            contacts_from: 0.85,
            traces_from: 0.76,
        },
    }
}

fn nes_spec() -> Spec {
    Spec {
        w: FC_CART_W,
        h: FC_CART_H,
        label: (NES_LABEL_X, NES_LABEL_Y, NES_LABEL_W, NES_LABEL_H),
        mask: fc_cart_mask(),
        depth: fc_cart_depth(),
        detail: fc_detail_mask(),
        max_px: NES_MAX_PX,
        max_h: NES_LABEL_H as f32,
        rim: RIM,
        board: Board {
            x: (0.18, 0.82),
            top: NES_LABEL_Y as f32 / FC_CART_H as f32,
            pins: (0.22, 0.78),
            contacts_from: 0.86,
            traces_from: 0.80,
        },
    }
}

fn gba_spec() -> Spec {
    Spec {
        w: CART_W,
        h: CART_H,
        label: (LABEL_X, LABEL_Y, LABEL_W, LABEL_H),
        mask: cart_mask(),
        depth: cart_depth(),
        detail: detail_mask(),
        max_px: MAX_PX,
        max_h: f32::INFINITY,
        rim: RIM,
        board: Board {
            x: (0.135, 0.875),
            top: LABEL_Y as f32 / CART_H as f32,
            pins: (0.17, 0.84),
            contacts_from: 0.85,
            traces_from: 0.76,
        },
    }
}

fn gb_spec(shell: GbShell) -> Spec {
    Spec {
        w: GB_CART_W,
        h: GB_CART_H,
        label: (GB_LABEL_X, GB_LABEL_Y, GB_LABEL_W, GB_LABEL_H),
        mask: gb_cart_mask(shell),
        depth: gb_cart_depth(shell),
        detail: gb_detail_mask(shell),
        max_px: GB_MAX_PX,
        max_h: (GB_LABEL_H - 2 * PAD) as f32,
        rim: GB_RIM,
        board: Board {
            x: (0.075, 0.925),
            top: 0.07,
            pins: (0.1, 0.9),
            contacts_from: 0.905,
            traces_from: 0.84,
        },
    }
}

pub const SEATED_W: u32 = 240;
const SEATED_H: u32 = 135;
const GB_SEATED_H: u32 = GB_CART_H;
const FC_SEATED_H: u32 = FC_BASE_H;
/// The SFC cassette is 360x252; seated at 240 wide it is 168 tall.
const SFC_SEATED_H: u32 = SFC_CART_H * SEATED_W / SFC_CART_W;
/// The MD cassette is 360x244; seated at 240 wide it is 162 tall.
const MD_SEATED_H: u32 = MD_CART_H * SEATED_W / MD_CART_W;

pub fn seated_box(platform: Platform) -> (u32, u32) {
    match platform {
        Platform::Gba => (SEATED_W, SEATED_H),
        Platform::Fc => (SEATED_W, FC_SEATED_H),
        Platform::Sfc => (SEATED_W, SFC_SEATED_H),
        Platform::Md => (SEATED_W, MD_SEATED_H),
        Platform::Gb | Platform::Gbc => (SEATED_W, GB_SEATED_H),
    }
}

/// The cartridge's own art size, before the on-screen display scale.  FC art
/// is drawn at 360x299 but shown at 85%, so anything laid out in art pixels
/// (the intro's back label) must use this instead of [`cart_box`].
pub fn cart_art_box(platform: Platform) -> (u32, u32) {
    let mut s = spec(match platform {
        Platform::Gba => Shape::Gba,
        Platform::Fc => Shape::Fc,
        Platform::Sfc => Shape::Sfc,
        Platform::Md => Shape::Md,
        Platform::Gb | Platform::Gbc => Shape::Gb(GbShell::Notched),
    });
    reference_spec(&mut s, platform);
    (s.w, s.h)
}

pub fn cart_box(platform: Platform) -> (u32, u32) {
    let mut s = spec(match platform {
        Platform::Gba => Shape::Gba,
        Platform::Fc => Shape::Fc,
        Platform::Sfc => Shape::Sfc,
        Platform::Md => Shape::Md,
        Platform::Gb | Platform::Gbc => Shape::Gb(GbShell::Notched),
    });
    reference_spec(&mut s, platform);
    match platform {
        // The FC face had to become 1.20 for a real cart label.  Its art is
        // still drawn at that size, but on screen it is shown a little smaller
        // so a 1.20 cassette does not dwarf the 1.49 Advance one.
        Platform::Fc => (s.w * FC_DISPLAY / 100, s.h * FC_DISPLAY / 100),
        _ => (s.w, s.h),
    }
}

/// How much of its own art the FC cartridge is shown at, in percent.
pub const FC_DISPLAY: u32 = 85;

pub fn cart_shadow() -> CartFace {
    shadow(CART_W, CART_H, cart_mask())
}

pub fn gb_cart_shadow(shell: GbShell) -> CartFace {
    shadow(GB_CART_W, GB_CART_H, gb_cart_mask(shell))
}

/// What the chooser pastes on the mini cartridge's label.
///
/// `Text` is the built-in cloud sticker: dark plate, cloud icon, white
/// wordmark and red sync rail. `TextOnly` is the same plate without the cloud,
/// for a mod that supplies no art. `Art` is a mod's own sticker PNG, already
/// decoded and fitted by the caller. `None` keeps the old printed label look.
/// The fixed sync cartridge: a gunmetal Advance shell. Solid, not
/// translucent, so the pressed title and the sticker stay clean at card size.
pub const SYNC_SHELL: Shell = Shell {
    colour: [0x46, 0x47, 0x4e],
    finish: Finish::Solid,
};

pub enum CartSticker<'a> {
    Text,
    TextOnly,
    Art {
        rgba: &'a [u8],
        w: u32,
        h: u32,
    },
}

pub fn labelled_cart_face(
    platform: Platform,
    gb_shell: Option<GbShell>,
    shell: Shell,
    label: &str,
) -> CartFace {
    labelled_cart_face_with_sticker(platform, gb_shell, shell, label, None)
}

pub fn labelled_cart_face_with_sticker(
    platform: Platform,
    gb_shell: Option<GbShell>,
    shell: Shell,
    label: &str,
    sticker: Option<CartSticker<'_>>,
) -> CartFace {
    let shape = card_shape(platform, gb_shell);
    let mut s = spec(shape);
    reference_spec(&mut s, platform);
    labelled_face(
        &s,
        matches!(shape, Shape::Fc),
        shell,
        label,
        sticker,
        Some(platform),
    )
}

/// The chooser's fixed sync cartridge: an Advance-shaped charcoal card whose
/// shell is pressed with `联网卡带`, not the machine name. Keeping one fixed
/// body gives every sticker the same label canvas.
pub fn sync_cart_face_with_sticker(
    shell: Shell,
    label: &str,
    sticker: Option<CartSticker<'_>>,
) -> CartFace {
    let mut s = gba_spec();
    s.detail = crate::silhouette::sync_detail_mask();
    let mut face = labelled_face(&s, false, shell, label, sticker, None);
    paint_sync_title(&mut face, shell);
    face
}

/// A printed title on the sync shell: the emboss alone disappears on a case
/// this flat, and this is the one place the sync card should still read
/// 联网卡带 at a glance. The ink flips with the shell's lightness.
fn paint_sync_title(face: &mut CartFace, shell: Shell) {
    let mask = crate::silhouette::sync_lettering(face.w, face.h);
    let luma = 0.2126 * shell.colour[0] as f32
        + 0.7152 * shell.colour[1] as f32
        + 0.0722 * shell.colour[2] as f32;
    let ink: [u8; 3] = if luma > 150.0 {
        [0x46, 0x44, 0x3f]
    } else {
        [0xd9, 0xd6, 0xce]
    };
    for (px, m) in face.rgba.chunks_exact_mut(4).zip(mask) {
        let a = m as u32 * 225 / 255;
        if a == 0 {
            continue;
        }
        for c in 0..3 {
            px[c] = ((ink[c] as u32 * a + px[c] as u32 * (255 - a) + 127) / 255) as u8;
        }
    }
}

fn labelled_face(
    s: &Spec,
    is_fc: bool,
    shell: Shell,
    label: &str,
    sticker: Option<CartSticker<'_>>,
    platform: Option<Platform>,
) -> CartFace {
    let label = match sticker {
        Some(CartSticker::Text) => generated_sticker(s, label, true),
        Some(CartSticker::TextOnly) => generated_sticker(s, label, false),
        Some(CartSticker::Art { rgba, w, h }) => sticker_art(s, rgba, w, h),
        None => generated_label(s, label, platform.unwrap_or(Platform::Gba)),
    };
    if let Some(platform) = platform {
        if reference_art(platform).is_some() {
            return reference_face(s, platform, CartModel::Famicom, &label, None);
        }
    }
    let mut face = shell_face(s, &shell);
    if is_fc {
        shade_fc(&mut face);
    }
    mould_detail(s, &mut face);
    if is_fc {
        fc_label_rim(s, &mut face);
    } else {
        recess_label(s, &mut face, &shell);
    }
    paste_label(s, &mut face, &label);
    clip_to_silhouette(s, &mut face);
    face
}

/// The drop shadow for [`labelled_cart_face`] cards.
pub fn labelled_cart_shadow(platform: Platform, gb_shell: Option<GbShell>) -> CartFace {
    if let Some(art) = reference_art(platform) {
        let mask: Vec<u8> = art.rgba.chunks_exact(4).map(|p| p[3]).collect();
        return shadow(art.w, art.h, &mask);
    }
    match card_shape(platform, gb_shell) {
        Shape::Gb(shell) => gb_cart_shadow(shell),
        _ => cart_shadow(),
    }
}

fn card_shape(platform: Platform, gb_shell: Option<GbShell>) -> Shape {
    match gb_shell {
        Some(shell) => Shape::Gb(shell),
        None => match platform {
            Platform::Gb | Platform::Gbc => Shape::Gb(GbShell::Notched),
            Platform::Fc => Shape::Fc,
            Platform::Sfc => Shape::Sfc,
            Platform::Md => Shape::Md,
            Platform::Gba => Shape::Gba,
        },
    }
}

fn shadow(w: u32, h: u32, mask: &[u8]) -> CartFace {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for cover in mask {
        rgba.extend_from_slice(&[0, 0, 0, *cover]);
    }
    CartFace { rgba, w, h }
}

pub fn cart_face(cart: &Cart) -> CartFace {
    let (lw, lh) = label_size(cart);
    let art = cart.label.as_deref().and_then(|p| art::contain(p, lw, lh));
    cart_face_with(cart, art)
}

/// The card face the reference project ships in `System/linkSlot/assets/carts/`.  These are
/// embedded so the packaged frontend looks the same on a fresh card; the
/// magenta label window in each PNG is replaced with the generated/cover
/// label at runtime.
struct ReferenceArt {
    rgba: Vec<u8>,
    w: u32,
    h: u32,
    /// x, y, width, height (not x0/y0/x1/y1).
    label: (u32, u32, u32, u32),
}

static REF_GBA: OnceLock<Option<ReferenceArt>> = OnceLock::new();
static REF_GB: OnceLock<Option<ReferenceArt>> = OnceLock::new();
static REF_GBC: OnceLock<Option<ReferenceArt>> = OnceLock::new();
static REF_FC: OnceLock<Option<ReferenceArt>> = OnceLock::new();
static REF_NES: OnceLock<Option<ReferenceArt>> = OnceLock::new();
static REF_SFC: OnceLock<Option<ReferenceArt>> = OnceLock::new();
static REF_MD: OnceLock<Option<ReferenceArt>> = OnceLock::new();

fn reference_art_model(platform: Platform, model: CartModel) -> Option<&'static ReferenceArt> {
    if platform == Platform::Fc && model == CartModel::Nes {
        return REF_NES
            .get_or_init(|| {
                art::decode_bytes(include_bytes!("../assets/nes_cart.png")).map(|(rgba, w, h)| {
                    ReferenceArt {
                        rgba,
                        w,
                        h,
                        label: (NES_LABEL_X, NES_LABEL_Y, NES_LABEL_W, NES_LABEL_H),
                    }
                })
            })
            .as_ref();
    }
    reference_art(platform)
}

fn reference_art(platform: Platform) -> Option<&'static ReferenceArt> {
    let slot = match platform {
        Platform::Gba => &REF_GBA,
        Platform::Gb => &REF_GB,
        Platform::Gbc => &REF_GBC,
        Platform::Fc => &REF_FC,
        Platform::Sfc => &REF_SFC,
        Platform::Md => &REF_MD,
    };
    slot.get_or_init(|| {
        let (bytes, label) = match platform {
            Platform::Gba => (
                &include_bytes!("../assets/gba_cart.png")[..],
                label_panel(360, 202),
            ),
            Platform::Gb => (
                &include_bytes!("../assets/gb_cart.png")[..],
                gb_label_panel(240, 276),
            ),
            Platform::Gbc => (
                &include_bytes!("../assets/gbc_cart.png")[..],
                gbc_label_panel(240, 276),
            ),
            Platform::Fc => (
                &include_bytes!("../assets/fc_cart.png")[..],
                fc_label_panel(FC_CART_W, FC_CART_H),
            ),
            // Measured from the reference card art: (27,15)-(332,121).
            Platform::Sfc => (
                &include_bytes!("../assets/sfc_cart.png")[..],
                ((SFC_CART_W * 27 / 360), (SFC_CART_H * 15 / 252),
                 (SFC_CART_W * 332 / 360), (SFC_CART_H * 121 / 252)),
            ),
            // Measured from the reference MD art: (52,4)-(308,210).
            Platform::Md => (
                &include_bytes!("../assets/md_cart.png")[..],
                ((MD_CART_W * 52 / 360), (MD_CART_H * 4 / 244),
                 (MD_CART_W * 308 / 360), (MD_CART_H * 210 / 244)),
            ),
        };
        let (x0, y0, x1, y1) = label;
        let label = (x0, y0, x1.saturating_sub(x0), y1.saturating_sub(y0));
        art::decode_bytes(bytes).map(|(rgba, w, h)| ReferenceArt { rgba, w, h, label })
    })
    .as_ref()
}

fn reference_spec(s: &mut Spec, platform: Platform) {
    let Some(art) = reference_art(platform) else {
        return;
    };
    s.w = art.w;
    s.h = art.h;
    s.label = art.label;
    s.max_px = (art.label.3 as f32 / (MAX_LINES as f32 * 1.36)).max(MIN_PX);
    s.max_h = art.label.3 as f32;
}

fn reference_spec_model(s: &mut Spec, platform: Platform, model: CartModel) {
    let Some(art) = reference_art_model(platform, model) else {
        return;
    };
    s.w = art.w;
    s.h = art.h;
    s.label = art.label;
    s.max_px = (art.label.3 as f32 / (MAX_LINES as f32 * 1.36)).max(MIN_PX);
    s.max_h = art.label.3 as f32;
}

fn reference_face(
    s: &Spec,
    platform: Platform,
    model: CartModel,
    label: &[u8],
    colour: Option<[u8; 3]>,
) -> CartFace {
    let Some(art) = reference_art_model(platform, model) else {
        return CartFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let mut face = CartFace {
        rgba: art.rgba.clone(),
        w: art.w,
        h: art.h,
    };
    if let Some(colour) = colour {
        tint_reference_shell(&mut face, colour);
    }
    fill_reference_label(s, &mut face);
    paste_label(s, &mut face, label);
    face
}

/// Re-colour the reference plastic while keeping its moulded light and shade.
/// The artwork is a photograph-like render, so a flat fill would erase the
/// bevels; luminance is mapped onto the chosen colour instead.
fn tint_reference_shell(face: &mut CartFace, colour: [u8; 3]) {
    for px in face.rgba.chunks_exact_mut(4) {
        if px[3] == 0 {
            continue;
        }
        let luma = (0.2126 * px[0] as f32
            + 0.7152 * px[1] as f32
            + 0.0722 * px[2] as f32)
            / 255.0;
        let shade = 0.34 + 0.88 * luma;
        for channel in 0..3 {
            px[channel] = (colour[channel] as f32 * shade).clamp(0.0, 255.0) as u8;
        }
    }
}

/// Replace the magenta placeholder with label paper, keeping the rounded
/// corners of the window.  Transparent sides of a no-crop cover then show
/// paper rather than the bright key colour.
fn fill_reference_label(s: &Spec, face: &mut CartFace) {
    let (lx, ly, lw, lh) = s.label;
    let radius = 7u32;
    let paper = [0xef, 0xe7, 0xd2, 255];
    for y in 0..lh {
        for x in 0..lw {
            let nx = x.min(lw - 1 - x);
            let ny = y.min(lh - 1 - y);
            if nx < radius && ny < radius {
                let dx = radius - nx;
                let dy = radius - ny;
                if dx * dx + dy * dy > radius * radius {
                    continue;
                }
            }
            let dst = (((y + ly) * face.w + x + lx) * 4) as usize;
            if dst + 3 < face.rgba.len() {
                face.rgba[dst..dst + 4].copy_from_slice(&paper);
            }
        }
    }
}

pub fn front_size(platform: Platform, model: CartModel) -> (u32, u32) {
    if let Some(art) = reference_art_model(platform, model) {
        return (art.w, art.h);
    }
    let s = spec(shape_of_platform(platform));
    (s.w, s.h)
}

fn shape_of_platform(platform: Platform) -> Shape {
    match platform {
        Platform::Gba => Shape::Gba,
        Platform::Gb | Platform::Gbc => Shape::Gb(GbShell::Notched),
        Platform::Fc => Shape::Fc,
        Platform::Sfc => Shape::Sfc,
        Platform::Md => Shape::Md,
    }
}

/// Normalize an uploaded PNG front to the fixed front canvas.  Transparent
/// padding is kept; the device clips it to the cartridge silhouette.  The Web
/// client converts PNG/JPEG/WebP inputs to PNG before calling this helper.
pub fn fit_front(bytes: &[u8], platform: Platform, model: CartModel) -> Option<(Vec<u8>, u32, u32)> {
    let (src, sw, sh) = art::decode_bytes(bytes)?;
    if sw == 0 || sh == 0 {
        return None;
    }
    let (w, h) = front_size(platform, model);
    Some((art::contain_rgba(&src, sw, sh, w, h), w, h))
}

pub fn label_size(cart: &Cart) -> (u32, u32) {
    if let Some(art) = reference_art_model(cart.platform, cart.model) {
        let (_, _, w, h) = art.label;
        return (w, h);
    }
    let (_, _, lw, lh) = spec(shape_of(cart)).label;
    (lw, lh)
}

/// The printable label window in the cart face's own pixels: `(x, y, w, h)`.
/// Overlays such as the favorite star anchor to this rectangle so they sit on
/// the cover instead of on the plastic shell.
pub fn label_rect(platform: Platform, model: CartModel) -> (u32, u32, u32, u32) {
    if let Some(art) = reference_art_model(platform, model) {
        return art.label;
    }
    let shape = match (platform, model) {
        (Platform::Fc, CartModel::Nes) => Shape::Nes,
        (Platform::Fc, _) => Shape::Fc,
        (Platform::Sfc, _) => Shape::Sfc,
        (Platform::Md, _) => Shape::Md,
        (Platform::Gb | Platform::Gbc, _) => Shape::Gb(GbShell::Notched),
        (Platform::Gba, _) => Shape::Gba,
    };
    spec(shape).label
}

pub fn cart_face_with(cart: &Cart, art: Option<Vec<u8>>) -> CartFace {
    let shape = shape_of(cart);
    let is_fc = matches!(shape, Shape::Fc | Shape::Nes);
    let mut s = spec(shape);
    reference_spec_model(&mut s, cart.platform, cart.model);
    let label = match art {
        Some(rgba) => rgba,
        None => generated_label(&s, &label_text(cart), cart.platform),
    };
    let mut face = if reference_art_model(cart.platform, cart.model).is_some() {
        // Follow the cartridge's actual/default shell colour, not the fixed
        // grey/clear plastic baked into the reference PNG.  The PCB lid uses
        // this same face, so a GB/GBC/FC back must not turn black when the
        // cartridge is opened.
        let shell = shell_for(cart);
        let tint = if cart.model == CartModel::Nes && cart.shell.is_none() {
            // The upstream NES Game Pak is moulded grey; do not tint it with
            // the Famicom's default gold shell.  An explicit per-game colour
            // still wins.
            None
        } else {
            Some(shell.colour)
        };
        reference_face(&s, cart.platform, cart.model, &label, tint)
    } else {
        let shell = shell_for(cart);
        let mut face = shell_face(&s, &shell);
        if is_fc {
            shade_fc(&mut face);
        }
        mould_detail(&s, &mut face);
        if is_fc {
            // Thin pressed edge only: the paper is pasted alpha-on-top, so a
            // transparent pixel shows yellow plastic instead of a black recess.
            fc_label_rim(&s, &mut face);
        } else {
            recess_label(&s, &mut face, &shell);
        }
        paste_label(&s, &mut face, &label);
        clip_to_silhouette(&s, &mut face);
        face
    };
    if let Some(front) = cart.front.as_deref() {
        if let Some((rgba, w, h)) = art::decode(front) {
            overlay_front(&mut face, &rgba, w, h);
        }
    }
    face
}

/// Composite a fixed-size custom front over the finished cartridge face.
/// The base face's alpha is the cartridge silhouette, so only the pixels that
/// belong to the physical cart are touched.
fn overlay_front(face: &mut CartFace, rgba: &[u8], w: u32, h: u32) {
    if w != face.w || h != face.h || rgba.len() < (w as usize * h as usize * 4) {
        return;
    }
    for index in 0..(w * h) as usize {
        let at = index * 4;
        let src = rgba[at + 3] as u32;
        if src == 0 {
            continue;
        }
        let dst = face.rgba[at + 3] as u32;
        if dst == 0 {
            continue;
        }
        let a = (src * dst + 127) / 255;
        for channel in 0..3 {
            face.rgba[at + channel] = ((rgba[at + channel] as u32 * a
                + face.rgba[at + channel] as u32 * (255 - a)
                + 127)
                / 255) as u8;
        }
    }
}

/// A Famicom cassette's flat plastic would read as a paper rectangle; this is
/// the slow top-to-bottom shade that makes the yellow shell look moulded.
fn shade_fc(face: &mut CartFace) {
    let h = face.h.max(1) as f32;
    for y in 0..face.h {
        let t = y as f32 / (h - 1.0).max(1.0);
        let f = 1.12 - 0.28 * t;
        for x in 0..face.w {
            let i = ((y * face.w + x) * 4) as usize;
            for c in 0..3 {
                face.rgba[i + c] =
                    (face.rgba[i + c] as f32 * f).clamp(0.0, 255.0) as u8;
            }
        }
    }
}

/// A 2 px shadow and 1 px highlight around the label window, drawn on the
/// plastic. Nothing fills the window: the label art's own alpha decides what
/// shows through.
fn fc_label_rim(s: &Spec, face: &mut CartFace) {
    let (lx, ly, lw, lh) = s.label;
    let dark = [0x6e, 0x4a, 0x0c];
    let light = [0xf7, 0xe1, 0xa8];
    let put = |face: &mut CartFace, x: u32, y: u32, colour: [u8; 3]| {
        if x < face.w && y < face.h {
            let i = ((y * face.w + x) * 4) as usize;
            face.rgba[i..i + 3].copy_from_slice(&colour);
        }
    };
    let left = lx.saturating_sub(2);
    let right = (lx + lw + 1).min(face.w - 1);
    let top = ly.saturating_sub(2);
    let bottom = (ly + lh + 1).min(face.h - 1);
    for x in left..=right {
        put(face, x, top, dark);
        put(face, x, top + 1, light);
        put(face, x, bottom, dark);
        put(face, x, bottom - 1, light);
    }
    for y in top..=bottom {
        put(face, left, y, dark);
        put(face, left + 1, y, light);
        put(face, right, y, dark);
        put(face, right - 1, y, light);
    }
}

fn clip_to_silhouette(s: &Spec, face: &mut CartFace) {
    for (px, cover) in face.rgba.chunks_exact_mut(4).zip(s.mask) {
        px[3] = ((px[3] as u32 * *cover as u32 + 127) / 255) as u8;
    }
}

pub fn label_colour(title: &str) -> [u8; 3] {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in title.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hsv_to_rgb((h % 360) as f32, 0.52, 0.74)
}

pub fn label_text(cart: &Cart) -> String {
    clean_label(&cart.stem)
}

pub fn clean_label(stem: &str) -> String {
    let mut bare = String::with_capacity(stem.len());
    let mut depth = 0u32;
    for ch in stem.chars() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => bare.push(ch),
            _ => {}
        }
    }

    let mut out = String::with_capacity(bare.len());
    for word in bare.split_whitespace().filter(|w| *w != "-") {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    if out.is_empty() {
        stem.to_string()
    } else {
        out
    }
}

pub fn label_tags(stem: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0u32;
    let mut cur = String::new();
    for ch in stem.chars() {
        match ch {
            '(' | '[' => {
                depth += 1;
                if depth == 1 {
                    cur.clear();
                    continue;
                }
            }
            ')' | ']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let t = cur.trim();
                    if !t.is_empty() {
                        out.push(t.to_string());
                    }
                    continue;
                }
            }
            _ => {}
        }
        if depth >= 1 {
            cur.push(ch);
        }
    }
    out
}

fn shell_face(s: &Spec, shell: &Shell) -> CartFace {
    let mut rgba = Vec::with_capacity((s.w * s.h * 4) as usize);
    let edge = rim_colour(through(shell.colour, shell.colour));
    for (i, depth) in s.depth.iter().enumerate() {
        let (x, y) = (i as u32 % s.w, i as u32 / s.w);
        let c = match shell.finish {
            Finish::Solid => shell.colour,
            Finish::Translucent | Finish::Glitter => {
                let body = through(shell.colour, inside(s, x, y).unwrap_or(shell.colour));
                let c = lerp(edge, body, (*depth as u32).min(s.rim), s.rim);
                if shell.finish == Finish::Glitter && fleck(x, y) {
                    lerp(c, [0xff; 3], 100, 255)
                } else {
                    c
                }
            }
        };
        rgba.extend_from_slice(&[c[0], c[1], c[2], 255]);
    }
    CartFace {
        rgba,
        w: s.w,
        h: s.h,
    }
}

const SURFACE: u32 = 150;

const BOARD: [u8; 3] = [0x2c, 0x96, 0x52];
const TRACE: [u8; 3] = [0x5a, 0xb4, 0x74];
const GOLD: [u8; 3] = [0xe6, 0xb4, 0x46];

fn through(plastic: [u8; 3], behind: [u8; 3]) -> [u8; 3] {
    let filter =
        |a: [u8; 3], b: [u8; 3]| [0, 1, 2].map(|c| (a[c] as u32 * b[c] as u32 / 255) as u8);
    lerp(
        filter(plastic, behind),
        filter(plastic, plastic),
        SURFACE,
        255,
    )
}

fn inside(s: &Spec, x: u32, y: u32) -> Option<[u8; 3]> {
    let b = &s.board;
    let (fx, fy) = (x as f32 / s.w as f32, y as f32 / s.h as f32);
    if !(b.x.0..b.x.1).contains(&fx) || fy < b.top {
        return None;
    }
    let (c0, c1) = b.pins;
    let pitch = (c1 - c0) / 32.0;
    let on_contact = (c0..c1).contains(&fx) && ((fx - c0) % pitch) < pitch * 0.6;
    if on_contact && fy >= b.contacts_from {
        return Some(GOLD);
    }
    if on_contact && ((fx - c0) % pitch) < pitch * 0.2 && fy >= b.traces_from {
        Some(TRACE)
    } else {
        Some(BOARD)
    }
}

fn fleck(x: u32, y: u32) -> bool {
    let mut h = x.wrapping_mul(0x9e37_79b9) ^ y.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h.is_multiple_of(100)
}

fn rim_colour(base: [u8; 3]) -> [u8; 3] {
    let mean = (base[0] as u16 + base[1] as u16 + base[2] as u16) / 3;
    base.map(|c| {
        let grey = (3 * c as u16 + mean) / 4;
        (grey + (255 - grey) * 2 / 5) as u8
    })
}

fn lerp(a: [u8; 3], b: [u8; 3], num: u32, den: u32) -> [u8; 3] {
    let mut out = [0u8; 3];
    for c in 0..3 {
        out[c] = ((a[c] as u32 * (den - num) + b[c] as u32 * num) / den) as u8;
    }
    out
}

const BEVEL: u32 = 3;

fn mould_detail(s: &Spec, face: &mut CartFace) {
    let mix = |px: &mut [u8], to: [u8; 3], a: u32| {
        for c in 0..3 {
            px[c] = ((to[c] as u32 * a + px[c] as u32 * (255 - a) + 127) / 255) as u8;
        }
    };
    let sides = s.detail.shadow.iter().zip(&s.detail.highlight);
    for (px, (shade, light)) in face.rgba.chunks_exact_mut(4).zip(sides) {
        let under = [px[0], px[1], px[2]];
        if *shade > 0 {
            mix(px, under.map(|c| (c as f32 * 0.62) as u8), *shade as u32);
        }
        if *light > 0 {
            mix(
                px,
                under.map(|c| c + ((255 - c) as f32 * 0.24) as u8),
                *light as u32,
            );
        }
    }
}

fn recess_label(s: &Spec, face: &mut CartFace, shell: &Shell) {
    let shade = |c: [u8; 3], f: f32| -> [u8; 3] {
        [
            (c[0] as f32 * f).clamp(0.0, 255.0) as u8,
            (c[1] as f32 * f).clamp(0.0, 255.0) as u8,
            (c[2] as f32 * f).clamp(0.0, 255.0) as u8,
        ]
    };
    let dark = shade(shell.colour, 0.55);
    let lit = shade(shell.colour, 1.45);

    let (lx, ly, lw, lh) = s.label;
    let (w, h) = (s.w, s.h);
    let bevel = (BEVEL * w + SEATED_W / 2) / SEATED_W;
    let (x0, y0) = (lx - bevel, ly - bevel);
    let (x1, y1) = (lx + lw + bevel, ly + lh + bevel);
    let clear = shell.finish != Finish::Solid;
    let mut put = |x: u32, y: u32, upper: bool| {
        if x >= w || y >= h {
            return;
        }
        let d = ((y * w + x) * 4) as usize;
        let c = if clear {
            let under = [face.rgba[d], face.rgba[d + 1], face.rgba[d + 2]];
            shade(under, if upper { 0.7 } else { 1.18 })
        } else if upper {
            dark
        } else {
            lit
        };
        face.rgba[d] = c[0];
        face.rgba[d + 1] = c[1];
        face.rgba[d + 2] = c[2];
    };
    for y in y0..y1 {
        for x in x0..x1 {
            let inside = (lx..lx + lw).contains(&x) && (ly..ly + lh).contains(&y);
            if inside {
                continue;
            }
            let from_top = y.saturating_sub(y0);
            let from_left = x.saturating_sub(x0);
            let from_bottom = y1.saturating_sub(y + 1);
            let from_right = x1.saturating_sub(x + 1);
            let upper = from_top.min(from_left);
            let lower = from_bottom.min(from_right);
            put(x, y, upper <= lower);
        }
    }
}

fn paste_label(s: &Spec, face: &mut CartFace, label: &[u8]) {
    let (lx, ly, lw, lh) = s.label;
    for y in 0..lh {
        for x in 0..lw {
            let src = ((y * lw + x) * 4) as usize;
            let a = label[src + 3] as u32;
            if a == 0 {
                continue;
            }
            let d = (((y + ly) * s.w + x + lx) * 4) as usize;
            for c in 0..3 {
                face.rgba[d + c] =
                    ((label[src + c] as u32 * a + face.rgba[d + c] as u32 * (255 - a) + 127) / 255)
                        as u8;
            }
        }
    }
}

fn generated_sticker(s: &Spec, title: &str, cloud: bool) -> Vec<u8> {
    let (_, _, lw, lh) = s.label;
    let mut face = UndoFace {
        rgba: vec![0u8; (lw * lh * 4) as usize],
        w: lw,
        h: lh,
    };
    if lw < 12 || lh < 12 {
        return face.rgba;
    }
    // Keep the sticker nearly the size of the paper label: the text needs the
    // room at 720p, and the mod stickers already fill their labels this way.
    let inset = ((lh as f32 * 0.035).round() as u32).max(2);
    let inset = inset.min(lw / 8).min(lh / 8);
    let (x, y) = (inset as f32, inset as f32);
    let (w, h) = ((lw - 2 * inset) as f32, (lh - 2 * inset) as f32);
    let pearl = [0xe9, 0xe6, 0xdf, 226];
    let accent = [sw::accent()[0], sw::accent()[1], sw::accent()[2], 235];
    sw::fill_round(&mut face, x + 2.0, y + 3.0, w, h, 10.0, [0, 0, 0, 115]);
    sw::fill_round(&mut face, x, y, w, h, 10.0, pearl);
    sw::fill_round(
        &mut face,
        x + 2.0,
        y + 2.0,
        w - 4.0,
        h - 4.0,
        8.5,
        [0x19, 0x1a, 0x1f, 248],
    );
    sw::fill_round(
        &mut face,
        x + 8.0,
        y + 10.0,
        3.0,
        (h - 20.0).max(1.0),
        1.5,
        accent,
    );

    let mut text_x = x + 22.0;
    if cloud {
        let icon = (h * 0.50).clamp(18.0, 34.0);
        let ix = x + 18.0;
        let iy = y + (h - icon) / 2.0;
        sw::fill_round(&mut face, ix, iy, icon, icon, 6.0, [0x2d, 0x2d, 0x33, 255]);
        sw::fill_round(
            &mut face,
            ix + 1.0,
            iy + 1.0,
            icon - 2.0,
            icon - 2.0,
            5.0,
            [0x4a, 0x4a, 0x54, 180],
        );
        sw::fill_round(
            &mut face,
            ix + 2.0,
            iy + 2.0,
            icon - 4.0,
            icon - 4.0,
            4.0,
            [0x25, 0x26, 0x2c, 255],
        );
        let ink = [0xe9, 0xe6, 0xdf, 245];
        let cx = ix + icon / 2.0;
        let cy = iy + icon * 0.50;
        let r = icon * 0.13;
        sw::fill_round(&mut face, cx - r * 2.2, cy - r, r * 2.0, r * 2.0, r, ink);
        sw::fill_round(
            &mut face,
            cx - r * 0.9,
            cy - r * 1.8,
            r * 2.2,
            r * 2.2,
            r * 1.1,
            ink,
        );
        sw::fill_round(
            &mut face,
            cx + r * 0.7,
            cy - r * 1.2,
            r * 2.0,
            r * 2.0,
            r,
            ink,
        );
        sw::fill_round(
            &mut face,
            cx - r * 2.1,
            cy + r * 0.2,
            r * 4.2,
            r * 1.5,
            r * 0.7,
            ink,
        );
        sw::fill_round(
            &mut face,
            cx + icon * 0.18,
            iy + icon * 0.72,
            icon * 0.20,
            icon * 0.20,
            icon * 0.05,
            accent,
        );
        text_x = ix + icon + 10.0;
    }
    let text_w = (x + w - text_x - 12.0).max(16.0);
    sw::fill_round(
        &mut face,
        text_x,
        y + h - 11.0,
        (x + w - text_x - 14.0).max(1.0),
        1.5,
        0.75,
        [0x9a, 0x97, 0x8e, 110],
    );

    if let Some(font) = text::label_font() {
        let layout = text::fit_box(
            font,
            title,
            text_w,
            (h - 14.0).max(16.0),
            2,
            (h * 0.36).clamp(12.0, 30.0),
            10.0,
        );
        text::draw_left(&mut face.rgba, lw, lh, &layout, sw::ink(), text_x + 4.0);
    }
    face.rgba
}

/// Fit a mod's sticker into the label without cropping its ends. Transparent
/// pixels around the art show the label paper, exactly like the built-in
/// sticker's margin.
fn sticker_art(s: &Spec, src: &[u8], aw: u32, ah: u32) -> Vec<u8> {
    let (_, _, lw, lh) = s.label;
    let mut face = UndoFace {
        rgba: vec![0u8; (lw * lh * 4) as usize],
        w: lw,
        h: lh,
    };
    if aw == 0 || ah == 0 || src.len() < (aw as usize * ah as usize * 4) {
        return face.rgba;
    }
    let scale = (lw as f32 / aw as f32).min(lh as f32 / ah as f32);
    let dw = (aw as f32 * scale).round().max(1.0) as u32;
    let dh = (ah as f32 * scale).round().max(1.0) as u32;
    let ox = (lw - dw) / 2;
    let oy = (lh - dh) / 2;
    for y in 0..dh {
        let sy = (((y as f32 + 0.5) * ah as f32 / dh as f32) as u32).min(ah - 1);
        for x in 0..dw {
            let sx = (((x as f32 + 0.5) * aw as f32 / dw as f32) as u32).min(aw - 1);
            let from = ((sy * aw + sx) * 4) as usize;
            let to = (((oy + y) * lw + ox + x) * 4) as usize;
            face.rgba[to..to + 4].copy_from_slice(&src[from..from + 4]);
        }
    }
    face.rgba
}

fn generated_label(s: &Spec, title: &str, platform: Platform) -> Vec<u8> {
    let (_, _, lw, lh) = s.label;
    let mut rgba = vec![0u8; (lw * lh * 4) as usize];
    paint_warm_cover(&mut rgba, lw, lh, platform);
    let Some(font) = text::label_font() else {
        return rgba;
    };

    let narrow = lw < 140;
    let portrait = matches!(platform, Platform::Gb | Platform::Gbc) || narrow;
    let title_px = if narrow {
        11.0
    } else {
        match platform {
            Platform::Gba => 18.0,
            Platform::Gb | Platform::Gbc => 17.0,
            Platform::Fc => 20.0,
            Platform::Sfc => 15.0,
            Platform::Md => 19.0,
        }
    };
    let title_w = if narrow {
        lw as f32 * 0.88
    } else {
        match platform {
            Platform::Gba => lw as f32 * 0.62,
            Platform::Gb | Platform::Gbc => lw as f32 * 0.78,
            Platform::Fc => lw as f32 * 0.62,
            Platform::Sfc => lw as f32 * 0.56,
            Platform::Md => lw as f32 * 0.60,
        }
    };
    let title_h = if portrait { lh as f32 * 0.40 } else { lh as f32 * 0.48 };
    let title = text::fit_box(font, title, title_w, title_h, 2, title_px, 8.0);
    let line_h = text::line_height(title.px);
    let block_h = line_h * title.lines.len() as f32;

    let cream = [0xff, 0xf4, 0xd9];
    let muted = [0xd8, 0xcb, 0xb4];
    let code = platform_code(platform);
    let code_layout = text::fit(font, code, lw as f32 * 0.28, 1, 6.0, 5.0);
    paint_text(&mut rgba, lw, lh, &code_layout, cream, 13.0, 17.0);
    if !narrow {
        let no_cover = text::fit(font, "NO COVER", lw as f32 * 0.30, 1, 5.0, 4.0);
        let text_w = no_cover
            .lines
            .first()
            .map_or(0.0, |line| text::line_width(font, line, no_cover.px, no_cover.tracking));
        paint_text(
            &mut rgba,
            lw,
            lh,
            &no_cover,
            [0xd8, 0xcb, 0xb4],
            (lw as f32 - 9.0 - text_w).max(8.0),
            16.0,
        );
    }

    if portrait {
        let top = 30.0;
        paint_text(&mut rgba, lw, lh, &title, cream, 13.0, top + line_height_offset(title.px));
        if !narrow {
            let sub = text::fit(font, "GAME TITLE", title_w, 1, 6.0, 5.0);
            paint_text(&mut rgba, lw, lh, &sub, muted, 13.0, top + block_h + 14.0);
        }
    } else {
        let title_top = (lh as f32 - 30.0 - block_h).max(20.0);
        paint_text(
            &mut rgba,
            lw,
            lh,
            &title,
            cream,
            13.0,
            title_top + line_height_offset(title.px),
        );
        let sub = text::fit(font, "GAME TITLE · PLACEHOLDER", lw as f32 * 0.70, 1, 6.0, 5.0);
        paint_text(&mut rgba, lw, lh, &sub, muted, 13.0, lh as f32 - 12.0);
    }
    rgba
}

fn line_height_offset(px: f32) -> f32 {
    text::line_height(px) * 0.78
}

fn platform_code(platform: Platform) -> &'static str {
    match platform {
        Platform::Gba => "GBA",
        Platform::Gb => "GB",
        Platform::Gbc => "GBC",
        Platform::Fc => "FC",
        Platform::Sfc => "SFC",
        Platform::Md => "MD",
    }
}

fn paint_text(
    dst: &mut [u8],
    dw: u32,
    dh: u32,
    layout: &text::Layout,
    colour: [u8; 3],
    left: f32,
    baseline: f32,
) {
    let Some(font) = text::label_font() else {
        return;
    };
    let Some(vm) = font.horizontal_line_metrics(layout.px) else {
        return;
    };
    let line_h = vm.new_line_size;
    let mut baseline = baseline;
    for line in &layout.lines {
        let mut pen = left;
        for ch in line.chars() {
            let (m, cov) = font.rasterize(ch, layout.px);
            let gx = (pen + m.xmin as f32).round() as i32;
            let gy = (baseline - (m.height as f32 + m.ymin as f32)).round() as i32;
            for y in 0..m.height as i32 {
                for x in 0..m.width as i32 {
                    let a = cov[(y * m.width as i32 + x) as usize];
                    if a == 0 {
                        continue;
                    }
                    let px = gx + x;
                    let py = gy + y;
                    if px < 0 || py < 0 || px as u32 >= dw || py as u32 >= dh {
                        continue;
                    }
                    let at = ((py as u32 * dw + px as u32) * 4) as usize;
                    blend_rgb(dst, at, a, colour);
                }
            }
            pen += m.advance_width + layout.tracking;
        }
        baseline += line_h;
    }
}

fn blend_rgb(dst: &mut [u8], at: usize, alpha: u8, colour: [u8; 3]) {
    let a = alpha as u32;
    let inv = 255 - a;
    for c in 0..3 {
        dst[at + c] = ((colour[c] as u32 * a + dst[at + c] as u32 * inv + 127) / 255) as u8;
    }
    dst[at + 3] = 255;
}

fn paint_warm_cover(dst: &mut [u8], w: u32, h: u32, platform: Platform) {
    let narrow = w < 140;
    let portrait = matches!(platform, Platform::Gb | Platform::Gbc) || narrow;
    let top = [0x16, 0x46, 0x6f];
    let bottom = [0x07, 0x1a, 0x30];
    let sun_rx = if portrait { w as f32 * 0.56 } else { w as f32 * 0.29 };
    let sun_ry = if portrait { h as f32 * 0.46 } else { h as f32 * 0.62 };
    let sun_cx = if portrait { w as f32 * 0.64 } else { w as f32 * 0.78 };
    let sun_cy = if portrait { h as f32 * 1.05 } else { h as f32 * 0.92 };
    for y in 0..h {
        for x in 0..w {
            let t = (x as f32 / w.max(1) as f32 * 0.25 + y as f32 / h.max(1) as f32 * 0.75)
                .clamp(0.0, 1.0);
            let mut c = [
                mix(top[0], bottom[0], t),
                mix(top[1], bottom[1], t),
                mix(top[2], bottom[2], t),
            ];
            let dx = (x as f32 - sun_cx) / sun_rx;
            let dy = (y as f32 - sun_cy) / sun_ry;
            let d = dx * dx + dy * dy;
            if d < 1.0 {
                let sy = ((y as f32 - (sun_cy - sun_ry)) / (2.0 * sun_ry)).clamp(0.0, 1.0);
                let sun = if sy < 0.28 {
                    [0xff, 0xe9, 0xa6]
                } else if sy < 0.72 {
                    [0xf2, 0xc8, 0x5b]
                } else {
                    [0xd1, 0x8d, 0x2f]
                };
                c = sun;
            } else if d < 2.2 {
                let glow = (1.0 - (d - 1.0) / 1.2).max(0.0) * 0.35;
                c = [
                    mix(c[0], 0xf2, glow),
                    mix(c[1], 0xc8, glow),
                    mix(c[2], 0x5b, glow),
                ];
            }
            let at = ((y * w + x) * 4) as usize;
            dst[at..at + 3].copy_from_slice(&c);
            dst[at + 3] = 255;
        }
    }

    let ground_h = if portrait { (h as f32 * 0.15) as u32 } else { (h as f32 * 0.23) as u32 };
    let ground_top = h - ground_h;
    for y in ground_top..h {
        for x in 0..w {
            let t = (y - ground_top) as f32 / ground_h.max(1) as f32;
            let c = [
                mix(0x2c, 0x14, t),
                mix(0x7f, 0x50, t),
                mix(0xa5, 0x6f, t),
            ];
            let at = ((y * w + x) * 4) as usize;
            dst[at..at + 3].copy_from_slice(&c);
        }
    }
    if ground_top < h {
        for x in 0..w {
            let at = ((ground_top * w + x) * 4) as usize;
            let c = [0x9a, 0xc4, 0xcf];
            dst[at..at + 3].copy_from_slice(&c);
        }
    }

    // Sparse printed stars, deterministic so every rebuild is identical.
    for i in 0..8 {
        let x = (i as u32 * 47 + 19) % w;
        let y = (i as u32 * 31 + 13) % (h / 2).max(1);
        let at = ((y * w + x) * 4) as usize;
        dst[at..at + 3].copy_from_slice(&[0xd8, 0xcb, 0xb4]);
    }

    // A quiet double arc in the upper right.
    let cx = (w as f32 * 0.78) as i32;
    let cy = (h as f32 * 0.30) as i32;
    for r in [w as i32 / 4, w as i32 / 3] {
        let mut x = r;
        let mut y = 0;
        let mut err = 1 - r;
        while x >= y {
            for (dx, dy) in [(x, y), (y, x), (-y, x), (-x, y), (-x, -y), (-y, -x), (y, -x), (x, -y)] {
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
                    let at = ((py as u32 * w + px as u32) * 4) as usize;
                    let c = [0x6b, 0x8e, 0xa5];
                    dst[at..at + 3].copy_from_slice(&c);
                }
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

    // Left/bottom darkening keeps the title readable over the picture.
    for y in 0..h {
        for x in 0..w {
            let left = (1.0 - x as f32 / (w as f32 * 0.62)).max(0.0);
            let low = (y as f32 / h as f32 - 0.55).max(0.0) * 1.4;
            let shade = (left * 0.16 + low * 0.26).min(0.36);
            if shade > 0.0 {
                let at = ((y * w + x) * 4) as usize;
                for c in 0..3 {
                    dst[at + c] = (dst[at + c] as f32 * (1.0 - shade)) as u8;
                }
            }
        }
    }
}

fn mix(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t.clamp(0.0, 1.0)).round() as u8
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [
        ((r + m) * 255.0).round() as u8,
        ((g + m) * 255.0).round() as u8,
        ((b + m) * 255.0).round() as u8,
    ]
}
