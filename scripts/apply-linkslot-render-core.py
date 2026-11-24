#!/usr/bin/env python3
"""Apply the H700 software-render contract to an isolated Linkslot 1.0.36 tree.

The immutable upstream snapshot is never edited in the repository. CI applies
this patch to a temporary checkout and later exports the resulting source diff.
"""
from __future__ import annotations

import argparse
import hashlib
import shutil
import subprocess
from pathlib import Path


EXPECTED_SOURCE = "8590d6ff1b2caa26118051069c975106fbb14bd3"


def run_git(source: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(source), *args],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return result.stdout.strip()


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one match, found {count}")
    return text.replace(old, new, 1)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True,
                        help="temporary checkout of upstream/linkslot")
    parser.add_argument("--crate", type=Path, required=True,
                        help="repository's integration/linkslot/h700-render-core")
    args = parser.parse_args()

    source = args.source.resolve()
    crate = args.crate.resolve()
    if run_git(source, "rev-parse", "HEAD") != EXPECTED_SOURCE:
        raise SystemExit("refusing to patch an unexpected Linkslot source revision")

    root_manifest = source / "Cargo.toml"
    if 'version = "1.0.36"' not in root_manifest.read_text(encoding="utf-8"):
        raise SystemExit("source tree is not the pinned Linkslot 1.0.36 snapshot")

    if not (crate / "Cargo.toml").is_file() or not (crate / "src/lib.rs").is_file():
        raise SystemExit("render-core crate is incomplete")

    target_crate = source / "crates/h700-render-core"
    if target_crate.exists():
        raise SystemExit(f"refusing to overwrite existing {target_crate}")
    shutil.copytree(crate, target_crate)

    gfx_manifest = source / "crates/slot-gfx/Cargo.toml"
    manifest_text = gfx_manifest.read_text(encoding="utf-8")
    manifest_text = replace_once(
        manifest_text,
        'test-support = []',
        'test-support = ["h700-render-core/test-support"]\nh700-software = []',
        "slot-gfx test-support and H700 software features",
    )
    manifest_text = replace_once(
        manifest_text,
        "[dependencies]\n",
        '[dependencies]\nh700-render-core = { path = "../h700-render-core" }\n',
        "slot-gfx render-core dependency",
    )
    gfx_manifest.write_text(manifest_text, encoding="utf-8")

    draw_file = source / "crates/slot-gfx/src/draw.rs"
    draw_text = draw_file.read_text(encoding="utf-8")
    start_marker = '#[derive(Copy, Clone, PartialEq, Eq, Debug)]\npub struct TexId(usize);'
    end_marker = 'pub struct Sprites {'
    start = draw_text.find(start_marker)
    end = draw_text.find(end_marker, start)
    if start < 0 or end < 0:
        raise SystemExit("slot-gfx Draw/TexId declarations do not match the audited baseline")
    if "pub enum Draw {" not in draw_text[start:end]:
        raise SystemExit("Draw enum is missing from the audited declaration block")
    draw_text = (
        draw_text[:start]
        + "pub use h700_render_core::{Draw, TexId};\n\n"
        + draw_text[end:]
    )

    # The local tuple-struct constructor is now private to the shared crate.
    # Route the GL backend's new texture handle through its public constructor.
    sc = "TexId(self.textures.len() - 1)"
    if draw_text.count(sc) != 1:
        raise SystemExit("expected one GL texture-handle constructor")
    draw_text = draw_text.replace(
        sc, "TexId::from_raw(self.textures.len() - 1)", 1
    )

    for field, expected in (("id.0", 3), ("tex.0", 2)):
        actual = draw_text.count(field)
        if actual != expected:
            raise SystemExit(f"expected {expected} occurrences of {field}, found {actual}")
        draw_text = draw_text.replace(field, field[:-2] + ".index()")

    draw_file.write_text(draw_text, encoding="utf-8")

    # Add an object-safe renderer boundary so the real Frontend can accept
    # either the original GL compositor or a future H700 software backend.
    renderer_rs = source / "crates/slot-gfx/src/renderer.rs"
    if renderer_rs.exists():
        raise SystemExit(f"refusing to overwrite {renderer_rs}")
    renderer_rs.write_text("""use crate::{Compositor, Draw, ScreenEffect, TexId};

/// Renderer contract consumed by Linkslot's frontend. It contains only
/// immediate rendering, texture and display-state operations; platform
/// input, audio and frame pacing remain outside this interface.
pub trait Renderer {
    fn create_texture(&mut self, w: u32, h: u32, rgba: &[u8]) -> TexId;
    fn create_texture_nearest(&mut self, w: u32, h: u32, rgba: &[u8]) -> TexId;
    fn update_texture(&mut self, id: TexId, w: u32, h: u32, rgba: &[u8]);
    fn delete_texture(&mut self, id: TexId);
    fn draw_list(&mut self, items: &[Draw]);
    fn begin_frame(&mut self);
    fn end_frame(&mut self, window: (u32, u32));
    fn fit(&mut self, window: (u32, u32));
    fn set_bezel(&mut self, w: u32, h: u32, rgba: &[u8]);
    fn set_blue_light(&mut self, step: u8);
    fn set_frame_lift(&mut self, t: f32);
    fn set_frame_size(&mut self, w: u32, h: u32);
    fn set_frame_split(&mut self, canvas_y: Option<f32>);
    fn set_game_source_rect(&mut self, rect: [f32; 4]);
    fn set_paper(&mut self, size: u32, gray: &[u8]);
    fn set_picture(&mut self, rect: [f32; 4]);
    fn set_screen_effect(&mut self, effect: ScreenEffect);
    fn set_screen_fit(&mut self, fit: bool);
    fn set_screen_power(&mut self, t: f32);
    fn set_shake(&mut self, dx: f32);
    fn upload_game(&mut self, xrgb8888: &[u8], w: u32, h: u32);
}

impl Renderer for Compositor {
    fn create_texture(&mut self, w: u32, h: u32, rgba: &[u8]) -> TexId {
        Compositor::create_texture(self, w, h, rgba)
    }
    fn create_texture_nearest(&mut self, w: u32, h: u32, rgba: &[u8]) -> TexId {
        Compositor::create_texture_nearest(self, w, h, rgba)
    }
    fn update_texture(&mut self, id: TexId, w: u32, h: u32, rgba: &[u8]) {
        Compositor::update_texture(self, id, w, h, rgba)
    }
    fn delete_texture(&mut self, id: TexId) {
        Compositor::delete_texture(self, id)
    }
    fn draw_list(&mut self, items: &[Draw]) {
        Compositor::draw_list(self, items)
    }
    fn begin_frame(&mut self) { Compositor::begin_frame(self) }
    fn end_frame(&mut self, window: (u32, u32)) { Compositor::end_frame(self, window) }
    fn fit(&mut self, window: (u32, u32)) { Compositor::fit(self, window) }
    fn set_bezel(&mut self, w: u32, h: u32, rgba: &[u8]) {
        Compositor::set_bezel(self, w, h, rgba)
    }
    fn set_blue_light(&mut self, step: u8) { Compositor::set_blue_light(self, step) }
    fn set_frame_lift(&mut self, t: f32) { Compositor::set_frame_lift(self, t) }
    fn set_frame_size(&mut self, w: u32, h: u32) { Compositor::set_frame_size(self, w, h) }
    fn set_frame_split(&mut self, canvas_y: Option<f32>) {
        Compositor::set_frame_split(self, canvas_y)
    }
    fn set_game_source_rect(&mut self, rect: [f32; 4]) {
        Compositor::set_game_source_rect(self, rect)
    }
    fn set_paper(&mut self, size: u32, gray: &[u8]) {
        Compositor::set_paper(self, size, gray)
    }
    fn set_picture(&mut self, rect: [f32; 4]) { Compositor::set_picture(self, rect) }
    fn set_screen_effect(&mut self, effect: ScreenEffect) {
        Compositor::set_screen_effect(self, effect)
    }
    fn set_screen_fit(&mut self, fit: bool) { Compositor::set_screen_fit(self, fit) }
    fn set_screen_power(&mut self, t: f32) { Compositor::set_screen_power(self, t) }
    fn set_shake(&mut self, dx: f32) { Compositor::set_shake(self, dx) }
    fn upload_game(&mut self, xrgb8888: &[u8], w: u32, h: u32) {
        Compositor::upload_game(self, xrgb8888, w, h)
    }
}
""", encoding="utf-8")

    gfx_lib = source / "crates/slot-gfx/src/lib.rs"
    gfx_lib_text = gfx_lib.read_text(encoding="utf-8")
    if "mod renderer;" in gfx_lib_text or "pub use renderer::Renderer;" in gfx_lib_text:
        raise SystemExit("slot-gfx renderer module already exists")
    gfx_lib_text = replace_once(
        gfx_lib_text, "mod quad;\n", "mod quad;\nmod renderer;\n#[cfg(feature = \"h700-software\")]\nmod software;\n",
        "slot-gfx renderer and software modules",
    )
    gfx_lib_text = replace_once(
        gfx_lib_text, "pub use power::{screen_brightness, screen_scale, screen_width};\n",
        "pub use power::{screen_brightness, screen_scale, screen_width};\npub use renderer::Renderer;\n#[cfg(feature = \"h700-software\")]\npub use software::SoftwareCompositor;\n",
        "slot-gfx Renderer and SoftwareCompositor exports",
    )
    gfx_lib.write_text(gfx_lib_text, encoding="utf-8")

    slot_manifest = source / "crates/slot/Cargo.toml"
    slot_manifest_text = slot_manifest.read_text(encoding="utf-8")
    slot_manifest_text = replace_once(
        slot_manifest_text,
        'device = ["slot-gfx/device"]',
        'device = ["slot-gfx/device"]\nh700-software = ["device", "slot-gfx/h700-software"]',
        "slot H700 software feature",
    )
    slot_manifest.write_text(slot_manifest_text, encoding="utf-8")

    slot_software = source / "crates/slot-gfx/src/software.rs"
    if slot_software.exists():
        raise SystemExit(f"refusing to overwrite {slot_software}")
    slot_software.write_text("""use h700_render_core::{Draw, PixelFormat, RenderError, SoftwareRenderer, TexId};
use crate::{blue_light_gain, Renderer, ScreenEffect};

/// Bits reported by SoftwareCompositor::unsupported_features.
pub const UNSUPPORTED_SCREEN_EFFECT: u32 = 1 << 0;
pub const UNSUPPORTED_PAPER_TEXTURE: u32 = 1 << 1;
pub const UNSUPPORTED_PICTURE_RECT: u32 = 1 << 2;
pub const UNSUPPORTED_FRAME_SPLIT: u32 = 1 << 3;
pub const UNSUPPORTED_FRAME_LIFT: u32 = 1 << 4;

/// Direct-to-framebuffer software compositor for the H700 port.
///
/// The caller supplies a validated framebuffer slice. This type never maps
/// physical addresses, assumes a scanout location, performs cache maintenance,
/// or swaps buffers. The H700 display HAL owns those operations.
pub struct SoftwareCompositor<'a> {
    raster: SoftwareRenderer<'a>,
    game_size: (u32, u32),
    fit: bool,
    power: f32,
    shake: f32,
    blue_gain: [f32; 3],
    bezel: Option<TexId>,
    last_error: Option<RenderError>,
    unsupported: u32,
}

impl<'a> SoftwareCompositor<'a> {
    pub fn new(
        width: u32,
        height: u32,
        stride: usize,
        format: PixelFormat,
        framebuffer: &'a mut [u8],
    ) -> Result<Self, RenderError> {
        let raster = SoftwareRenderer::new(width, height, stride, format, framebuffer)?;
        Ok(Self {
            raster,
            game_size: (0, 0),
            fit: false,
            power: 1.0,
            shake: 0.0,
            blue_gain: [1.0, 1.0, 1.0],
            bezel: None,
            last_error: None,
            unsupported: 0,
        })
    }

    pub fn framebuffer(&self) -> &[u8] {
        self.raster.framebuffer()
    }

    /// Allocation and upload errors that cannot be returned through the
    /// legacy infallible Linkslot Renderer trait.
    pub fn take_error(&mut self) -> Option<RenderError> {
        self.last_error.take()
    }

    /// Query effects that this baseline backend does not implement.
    pub const fn unsupported_features(&self) -> u32 {
        self.unsupported
    }

    fn invalid_texture() -> TexId {
        TexId::from_raw(usize::MAX)
    }

    fn update_game_rect(&mut self) {
        let (src_w, src_h) = self.game_size;
        let (canvas_w, canvas_h) = (self.raster.width(), self.raster.height());
        if src_w == 0 || src_h == 0 {
            self.raster.set_game_rect(0.0, 0.0, canvas_w as f32, canvas_h as f32);
            return;
        }

        let sx = canvas_w as f32 / src_w as f32;
        let sy = canvas_h as f32 / src_h as f32;
        let scale = if self.fit {
            sx.min(sy)
        } else {
            sx.floor().min(sy.floor()).max(1.0)
        };
        let base_w = src_w as f32 * scale;
        let base_h = src_h as f32 * scale;

        // Keep Linkslot's short power-on/off collapse envelope for the game
        // picture. UI draws remain visible above it.
        let p = self.power.clamp(0.0, 1.0);
        let left = 1.0 - (p / 0.12).min(1.0);
        let width_factor = 1.0 - (1.0 - 2.0 / canvas_w as f32) * left;
        let height_factor = 1.0 - (1.0 - 2.0 / canvas_h as f32) * (1.0 - p).powi(2);
        let w = base_w * width_factor;
        let h = base_h * height_factor;
        let x = (canvas_w as f32 - w) * 0.5 + self.shake;
        let y = (canvas_h as f32 - h) * 0.5;
        self.raster.set_game_rect(x, y, w, h);
    }

    fn mark_unsupported(&mut self, feature: u32) {
        self.unsupported |= feature;
    }
}

impl Renderer for SoftwareCompositor<'_> {
    fn create_texture(&mut self, w: u32, h: u32, rgba: &[u8]) -> TexId {
        match self.raster.create_texture(w, h, rgba) {
            Ok(id) => id,
            Err(error) => {
                self.last_error = Some(error);
                Self::invalid_texture()
            }
        }
    }

    fn create_texture_nearest(&mut self, w: u32, h: u32, rgba: &[u8]) -> TexId {
        match self.raster.create_texture_nearest(w, h, rgba) {
            Ok(id) => id,
            Err(error) => {
                self.last_error = Some(error);
                Self::invalid_texture()
            }
        }
    }

    fn update_texture(&mut self, id: TexId, w: u32, h: u32, rgba: &[u8]) {
        if let Err(error) = self.raster.update_texture(id, w, h, rgba) {
            self.last_error = Some(error);
        }
    }

    fn delete_texture(&mut self, id: TexId) {
        self.raster.delete_texture(id);
        if self.bezel == Some(id) {
            self.bezel = None;
        }
    }

    fn draw_list(&mut self, items: &[Draw]) {
        self.raster.draw_list(items);
    }

    fn begin_frame(&mut self) {
        self.raster.clear([0, 0, 0, 255]);
    }

    fn end_frame(&mut self, _window: (u32, u32)) {
        if let Some(tex) = self.bezel {
            let w = self.raster.width() as f32;
            let h = self.raster.height() as f32;
            self.raster.draw_list(&[Draw::Tex {
                x: 0.0, y: 0.0, w, h, tex, alpha: 1.0,
            }]);
        }
        let p = self.power.clamp(0.0, 1.0);
        let brightness = 1.0 + 0.6 * (1.0 - p).powi(2);
        self.raster.apply_rgb_gain([
            self.blue_gain[0] * brightness,
            self.blue_gain[1] * brightness,
            self.blue_gain[2] * brightness,
        ]);
    }

    fn fit(&mut self, _window: (u32, u32)) {
        // The caller-provided framebuffer dimensions are fixed at construction.
    }

    fn set_bezel(&mut self, w: u32, h: u32, rgba: &[u8]) {
        let new_tex = match self.raster.create_texture(w, h, rgba) {
            Ok(id) => id,
            Err(error) => {
                self.last_error = Some(error);
                return;
            }
        };
        if let Some(old) = self.bezel.replace(new_tex) {
            self.raster.delete_texture(old);
        }
    }

    fn set_blue_light(&mut self, step: u8) {
        self.blue_gain = blue_light_gain(step);
    }

    fn set_frame_lift(&mut self, t: f32) {
        if (t - 1.0).abs() > 0.0001 {
            self.mark_unsupported(UNSUPPORTED_FRAME_LIFT);
        }
    }

    fn set_frame_size(&mut self, w: u32, h: u32) {
        self.game_size = (w, h);
        self.update_game_rect();
    }

    fn set_frame_split(&mut self, canvas_y: Option<f32>) {
        if canvas_y.is_some() {
            self.mark_unsupported(UNSUPPORTED_FRAME_SPLIT);
        }
    }

    fn set_game_source_rect(&mut self, rect: [f32; 4]) {
        self.raster.set_game_source_rect(rect);
    }

    fn set_paper(&mut self, size: u32, gray: &[u8]) {
        if size != 0 && !gray.is_empty() {
            self.mark_unsupported(UNSUPPORTED_PAPER_TEXTURE);
        }
    }

    fn set_picture(&mut self, rect: [f32; 4]) {
        if rect != [0.0, 0.0, 1.0, 1.0] {
            self.mark_unsupported(UNSUPPORTED_PICTURE_RECT);
        }
    }

    fn set_screen_effect(&mut self, effect: ScreenEffect) {
        if effect != ScreenEffect::None {
            self.mark_unsupported(UNSUPPORTED_SCREEN_EFFECT);
        }
    }

    fn set_screen_fit(&mut self, fit: bool) {
        self.fit = fit;
        self.update_game_rect();
    }

    fn set_screen_power(&mut self, t: f32) {
        self.power = t.clamp(0.0, 1.0);
        self.update_game_rect();
    }

    fn set_shake(&mut self, dx: f32) {
        self.shake = dx;
        self.update_game_rect();
    }

    fn upload_game(&mut self, xrgb8888: &[u8], w: u32, h: u32) {
        if (w, h) != self.game_size {
            self.game_size = (w, h);
            self.update_game_rect();
        }
        if let Err(error) = self.raster.upload_game_bgra(w, h, xrgb8888) {
            self.last_error = Some(error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_draws_to_caller_framebuffer_and_reports_unimplemented_effects() {
        let mut framebuffer = [0u8; 4 * 2];
        let mut renderer = SoftwareCompositor::new(
            2, 1, 8, PixelFormat::Rgba8888, &mut framebuffer,
        ).unwrap();
        Renderer::begin_frame(&mut renderer);
        Renderer::set_frame_size(&mut renderer, 1, 1);
        Renderer::upload_game(&mut renderer, &[30, 20, 10, 255], 1, 1);
        Renderer::draw_list(&mut renderer, &[Draw::Game]);
        Renderer::set_screen_effect(&mut renderer, ScreenEffect::Grid);
        Renderer::end_frame(&mut renderer, (2, 1));
        assert_eq!(renderer.framebuffer()[0..4], [10, 20, 30, 255]);
        assert_ne!(renderer.unsupported_features() & UNSUPPORTED_SCREEN_EFFECT, 0);
        assert_eq!(renderer.take_error(), None);
    }
}
""", encoding="utf-8")

    frontend_file = source / "crates/slot/src/frontend.rs"
    frontend_text = frontend_file.read_text(encoding="utf-8")
    frontend_text = replace_once(
        frontend_text,
        "use slot_gfx::{Compositor, Draw, ScreenEffect, TexId, OUT_H, OUT_W};",
        "use slot_gfx::{Draw, Renderer, ScreenEffect, TexId, OUT_H, OUT_W};",
        "Frontend renderer import",
    )
    frontend_count = frontend_text.count("&mut Compositor")
    if frontend_count < 20:
        raise SystemExit(f"expected at least 20 Frontend Compositor parameters, found {frontend_count}")
    frontend_text = frontend_text.replace("&mut Compositor", "&mut dyn Renderer")
    frontend_file.write_text(frontend_text, encoding="utf-8")

    # CartFaces is called by Frontend::render/compose and also uploads the
    # textures consumed by its draw list, so it must use the same backend seam.
    cart_faces_file = source / "crates/slot/src/cart_faces.rs"
    cart_faces_text = cart_faces_file.read_text(encoding="utf-8")
    cart_faces_text = replace_once(
        cart_faces_text,
        "use slot_gfx::{Compositor, TexId};",
        "use slot_gfx::{Renderer, TexId};",
        "CartFaces renderer import",
    )
    cart_compositor_count = cart_faces_text.count("&mut Compositor")
    if cart_compositor_count != 2:
        raise SystemExit(
            f"expected two CartFaces Compositor parameters, found {cart_compositor_count}"
        )
    cart_faces_text = cart_faces_text.replace("&mut Compositor", "&mut dyn Renderer")
    cart_faces_file.write_text(cart_faces_text, encoding="utf-8")

    if "pub struct TexId" in draw_text or "pub enum Draw" in draw_text:
        raise SystemExit("local duplicate Draw/TexId types remain in slot-gfx")
    if "pub use h700_render_core::{Draw, TexId};" not in draw_text:
        raise SystemExit("slot-gfx did not re-export the shared render contract")

    run_git(source, "diff", "--check")
    changed = run_git(source, "status", "--short")
    print("Applied shared H700 render contract to temporary Linkslot 1.0.36 tree.")
    print(f"Base: {EXPECTED_SOURCE}")
    print("Changed files:")
    print(changed)
    for path in (target_crate / "Cargo.toml", target_crate / "src/lib.rs",
                 gfx_manifest, draw_file):
        print(f"SHA256 {sha256(path)}  {path.relative_to(source)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
