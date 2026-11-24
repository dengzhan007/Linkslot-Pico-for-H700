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
        'test-support = ["h700-render-core/test-support"]',
        "slot-gfx test-support feature",
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
        gfx_lib_text, "mod quad;\n", "mod quad;\nmod renderer;\n",
        "slot-gfx renderer module",
    )
    gfx_lib_text = replace_once(
        gfx_lib_text, "pub use power::{screen_brightness, screen_scale, screen_width};\n",
        "pub use power::{screen_brightness, screen_scale, screen_width};\npub use renderer::Renderer;\n",
        "slot-gfx Renderer export",
    )
    gfx_lib.write_text(gfx_lib_text, encoding="utf-8")

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
