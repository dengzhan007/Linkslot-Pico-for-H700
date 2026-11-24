#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::vec::Vec;

/// Stable handle for a CPU-side RGBA texture. Deleted handles are not reused.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TexId(usize);

/// Renderer-independent drawing commands matching Linkslot's current draw vocabulary.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Draw {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        colour: [f32; 4],
    },
    Tex {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        tex: TexId,
        alpha: f32,
    },
    Turned {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        tex: TexId,
        alpha: f32,
        turn: f32,
    },
    Game,
    Shot {
        tex: TexId,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PixelFormat {
    /// Bytes in memory are R, G, B, A.
    Rgba8888,
    /// Bytes in memory are B, G, R, A.
    Bgra8888,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RenderError {
    ZeroDimension,
    DimensionOverflow,
    StrideTooSmall,
    FramebufferTooSmall,
    TextureTooLarge,
    InvalidTextureData,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum TextureFilter { Linear, Nearest }

struct Texture {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    filter: TextureFilter,
}

/// A deterministic CPU rasterizer. The framebuffer is supplied by the caller:
/// this crate never chooses or maps a physical address.
///
/// Textures are stored as tightly packed RGBA8888. The caller-selected
/// framebuffer format is converted at each pixel access, keeping the software
/// renderer independent of the board's eventual scanout byte order.
pub struct SoftwareRenderer<'a> {
    width: u32,
    height: u32,
    stride: usize,
    format: PixelFormat,
    framebuffer: &'a mut [u8],
    textures: Vec<Option<Texture>>,
    game: Option<Texture>,
    game_rect: [f32; 4],
}

impl<'a> SoftwareRenderer<'a> {
    pub fn new(
        width: u32,
        height: u32,
        stride: usize,
        format: PixelFormat,
        framebuffer: &'a mut [u8],
    ) -> Result<Self, RenderError> {
        if width == 0 || height == 0 {
            return Err(RenderError::ZeroDimension);
        }
        let row_bytes = (width as usize)
            .checked_mul(4)
            .ok_or(RenderError::DimensionOverflow)?;
        if stride < row_bytes {
            return Err(RenderError::StrideTooSmall);
        }
        let required = stride
            .checked_mul(height as usize)
            .ok_or(RenderError::DimensionOverflow)?;
        if framebuffer.len() < required {
            return Err(RenderError::FramebufferTooSmall);
        }

        let mut renderer = Self {
            width,
            height,
            stride,
            format,
            framebuffer,
            textures: Vec::new(),
            game: None,
            game_rect: [0.0, 0.0, width as f32, height as f32],
        };
        renderer.clear([0, 0, 0, 255]);
        Ok(renderer)
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub const fn stride(&self) -> usize {
        self.stride
    }

    pub const fn pixel_format(&self) -> PixelFormat {
        self.format
    }

    pub fn framebuffer(&self) -> &[u8] {
        self.framebuffer
    }

    pub fn rgba_at(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let offset = y as usize * self.stride + x as usize * 4;
        Some(read_pixel(self.framebuffer, offset, self.format))
    }

    pub fn clear(&mut self, rgba: [u8; 4]) {
        for y in 0..self.height {
            for x in 0..self.width {
                let offset = y as usize * self.stride + x as usize * 4;
                write_pixel(self.framebuffer, offset, self.format, rgba);
            }
        }
    }

    /// Match Linkslot's default GL_LINEAR texture creation.
    pub fn create_texture(
        &mut self, width: u32, height: u32, rgba: &[u8],
    ) -> Result<TexId, RenderError> {
        self.create_texture_with_filter(width, height, rgba, TextureFilter::Linear)
    }

    /// Match Linkslot's explicit GL_NEAREST texture creation.
    pub fn create_texture_nearest(
        &mut self, width: u32, height: u32, rgba: &[u8],
    ) -> Result<TexId, RenderError> {
        self.create_texture_with_filter(width, height, rgba, TextureFilter::Nearest)
    }

    fn create_texture_with_filter(
        &mut self, width: u32, height: u32, rgba: &[u8], filter: TextureFilter,
    ) -> Result<TexId, RenderError> {
        let bytes = texture_bytes(width, height)?;
        if rgba.len() < bytes {
            return Err(RenderError::InvalidTextureData);
        }
        let mut pixels = Vec::new();
        pixels.try_reserve_exact(bytes).map_err(|_| RenderError::TextureTooLarge)?;
        pixels.extend_from_slice(&rgba[..bytes]);
        let id = TexId(self.textures.len());
        self.textures.push(Some(Texture { width, height, rgba: pixels, filter }));
        Ok(id)
    }

    pub fn update_texture(
        &mut self,
        id: TexId,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<bool, RenderError> {
        let bytes = texture_bytes(width, height)?;
        if rgba.len() < bytes {
            return Err(RenderError::InvalidTextureData);
        }
        let Some(slot) = self.textures.get_mut(id.0) else {
            return Ok(false);
        };
        let Some(existing) = slot.as_ref() else { return Ok(false); };
        let filter = existing.filter;
        let mut pixels = Vec::new();
        pixels.try_reserve_exact(bytes).map_err(|_| RenderError::TextureTooLarge)?;
        pixels.extend_from_slice(&rgba[..bytes]);
        *slot = Some(Texture { width, height, rgba: pixels, filter });
        Ok(true)
    }

    pub fn delete_texture(&mut self, id: TexId) -> bool {
        let Some(slot) = self.textures.get_mut(id.0) else {
            return false;
        };
        slot.take().is_some()
    }

    /// Linkslot's current GL backend uploads XRGB8888 input as GL_BGRA.
    /// This helper accepts the same byte layout and converts it to RGBA.
    pub fn upload_game_bgra(
        &mut self,
        width: u32,
        height: u32,
        bgra: &[u8],
    ) -> Result<(), RenderError> {
        let bytes = texture_bytes(width, height)?;
        if bgra.len() < bytes {
            return Err(RenderError::InvalidTextureData);
        }
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(bytes).map_err(|_| RenderError::TextureTooLarge)?;
        for px in bgra[..bytes].chunks_exact(4) {
            rgba.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
        }
        self.game = Some(Texture { width, height, rgba, filter: TextureFilter::Nearest });
        Ok(())
    }

    pub fn upload_game_rgba(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), RenderError> {
        let bytes = texture_bytes(width, height)?;
        if rgba.len() < bytes {
            return Err(RenderError::InvalidTextureData);
        }
        let mut pixels = Vec::new();
        pixels.try_reserve_exact(bytes).map_err(|_| RenderError::TextureTooLarge)?;
        pixels.extend_from_slice(&rgba[..bytes]);
        self.game = Some(Texture { width, height, rgba: pixels, filter: TextureFilter::Nearest });
        Ok(())
    }

    /// Set the destination rectangle used by Draw::Game and Draw::Shot.
    pub fn set_game_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.game_rect = [x, y, width, height];
    }

    pub fn draw_list(&mut self, items: &[Draw]) {
        for item in items {
            match *item {
                Draw::Rect {
                    x,
                    y,
                    w,
                    h,
                    colour,
                } => self.draw_rect(x, y, w, h, colour),
                Draw::Tex {
                    x,
                    y,
                    w,
                    h,
                    tex,
                    alpha,
                } => self.draw_texture(tex, x, y, w, h, alpha, 0.0),
                Draw::Turned {
                    x,
                    y,
                    w,
                    h,
                    tex,
                    alpha,
                    turn,
                } => self.draw_texture(tex, x, y, w, h, alpha, turn),
                Draw::Game => self.draw_game(),
                Draw::Shot { tex } => {
                    let r = self.game_rect;
                    self.draw_texture(tex, r[0], r[1], r[2], r[3], 1.0, 0.0);
                }
            }
        }
    }

    fn draw_rect(&mut self, x: f32, y: f32, w: f32, h: f32, colour: [f32; 4]) {
        let rgba = [
            unit_to_byte(colour[0]),
            unit_to_byte(colour[1]),
            unit_to_byte(colour[2]),
            unit_to_byte(colour[3]),
        ];
        fill_rect(
            self.framebuffer,
            self.width,
            self.height,
            self.stride,
            self.format,
            x,
            y,
            w,
            h,
            rgba,
        );
    }

    fn draw_texture(
        &mut self,
        id: TexId,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        alpha: f32,
        turn: f32,
    ) {
        let Some(texture) = self.textures.get(id.0).and_then(Option::as_ref) else {
            return;
        };
        let width = texture.width;
        let height = texture.height;
        let pixels = texture.rgba.as_slice();
        let filter = texture.filter;
        blit_texture(
            self.framebuffer,
            self.width,
            self.height,
            self.stride,
            self.format,
            pixels,
            width,
            height,
            filter,
            x,
            y,
            w,
            h,
            alpha,
            turn,
        );
    }

    fn draw_game(&mut self) {
        let Some(texture) = self.game.as_ref() else {
            return;
        };
        let width = texture.width;
        let height = texture.height;
        let pixels = texture.rgba.as_slice();
        let filter = texture.filter;
        let r = self.game_rect;
        blit_texture(
            self.framebuffer,
            self.width,
            self.height,
            self.stride,
            self.format,
            pixels,
            width,
            height,
            filter,
            r[0],
            r[1],
            r[2],
            r[3],
            1.0,
            0.0,
        );
    }
}

fn texture_bytes(width: u32, height: u32) -> Result<usize, RenderError> {
    if width == 0 || height == 0 {
        return Err(RenderError::ZeroDimension);
    }
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(RenderError::TextureTooLarge)
}

fn unit_to_byte(value: f32) -> u8 {
    let v = if !value.is_finite() {
        0.0
    } else if value < 0.0 {
        0.0
    } else if value > 1.0 {
        1.0
    } else {
        value
    };
    libm::roundf(v * 255.0) as u8
}

fn bounds(start: f32, extent: f32, limit: u32) -> Option<(u32, u32)> {
    if !start.is_finite() || !extent.is_finite() || extent <= 0.0 {
        return None;
    }
    let end = start + extent;
    if !end.is_finite() {
        return None;
    }
    let lo = libm::floorf(start);
    let hi = libm::ceilf(end);
    let clamp = |v: f32| -> u32 {
        if v <= 0.0 {
            0
        } else if v >= limit as f32 {
            limit
        } else {
            v as u32
        }
    };
    let a = clamp(lo);
    let b = clamp(hi);
    (a < b).then_some((a, b))
}

fn fill_rect(
    framebuffer: &mut [u8],
    width: u32,
    height: u32,
    stride: usize,
    format: PixelFormat,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    rgba: [u8; 4],
) {
    let Some((x0, x1)) = bounds(x, w, width) else {
        return;
    };
    let Some((y0, y1)) = bounds(y, h, height) else {
        return;
    };
    for py in y0..y1 {
        for px in x0..x1 {
            let offset = py as usize * stride + px as usize * 4;
            blend_pixel(framebuffer, offset, format, rgba);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn blit_texture(
    framebuffer: &mut [u8],
    fb_width: u32,
    fb_height: u32,
    stride: usize,
    format: PixelFormat,
    texture: &[u8],
    tex_width: u32,
    tex_height: u32,
    filter: TextureFilter,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    alpha: f32,
    turn: f32,
) {
    if tex_width == 0
        || tex_height == 0
        || !w.is_finite()
        || !h.is_finite()
        || w <= 0.0
        || h <= 0.0
    {
        return;
    }
    let global_alpha = unit_to_byte(alpha);
    if global_alpha == 0 {
        return;
    }

    let angle = if turn.is_finite() { turn } else { 0.0 };
    let (sin, cos) = if angle == 0.0 {
        (0.0, 1.0)
    } else {
        (libm::sinf(angle), libm::cosf(angle))
    };
    let cx = x + w * 0.5;
    let cy = y + h * 0.5;
    if !cx.is_finite() || !cy.is_finite() {
        return;
    }

    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for (dx, dy) in [(-w * 0.5, -h * 0.5), (w * 0.5, -h * 0.5), (-w * 0.5, h * 0.5), (w * 0.5, h * 0.5)] {
        let tx = cx + dx * cos - dy * sin;
        let ty = cy + dx * sin + dy * cos;
        min_x = min_x.min(tx);
        min_y = min_y.min(ty);
        max_x = max_x.max(tx);
        max_y = max_y.max(ty);
    }
    let Some((x0, x1)) = bounds(min_x, max_x - min_x, fb_width) else {
        return;
    };
    let Some((y0, y1)) = bounds(min_y, max_y - min_y, fb_height) else {
        return;
    };

    let tex_len = (tex_width as usize)
        .checked_mul(tex_height as usize)
        .and_then(|n| n.checked_mul(4));
    let Some(tex_len) = tex_len else {
        return;
    };
    if texture.len() < tex_len {
        return;
    }

    for py in y0..y1 {
        for px in x0..x1 {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            // Inverse rotation maps framebuffer coordinates back to the
            // unrotated destination rectangle.
            let local_x = dx * cos + dy * sin;
            let local_y = -dx * sin + dy * cos;
            let u = (local_x + w * 0.5) / w;
            let v = (local_y + h * 0.5) / h;
            if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                continue;
            }
            let mut rgba = match filter {
                TextureFilter::Nearest => sample_nearest(texture, tex_width, tex_height, u, v),
                TextureFilter::Linear => sample_linear(texture, tex_width, tex_height, u, v),
            };
            rgba[3] = ((rgba[3] as u32 * global_alpha as u32 + 127) / 255) as u8;
            if rgba[3] == 0 { continue; }
            let offset = py as usize * stride + px as usize * 4;
            blend_pixel(framebuffer, offset, format, rgba);
        }
    }
}

fn sample_nearest(texture: &[u8], width: u32, height: u32, u: f32, v: f32) -> [u8; 4] {
    let x = ((u * width as f32) as u32).min(width - 1);
    let y = ((v * height as f32) as u32).min(height - 1);
    let i = (y as usize * width as usize + x as usize) * 4;
    [texture[i], texture[i + 1], texture[i + 2], texture[i + 3]]
}

/// Normalized GL_LINEAR sampling with CLAMP_TO_EDGE and half-texel centres.
fn sample_linear(texture: &[u8], width: u32, height: u32, u: f32, v: f32) -> [u8; 4] {
    let fx = u * width as f32 - 0.5;
    let fy = v * height as f32 - 0.5;
    let ix = libm::floorf(fx) as i32;
    let iy = libm::floorf(fy) as i32;
    let tx = fx - ix as f32;
    let ty = fy - iy as f32;
    let cx = |x: i32| x.max(0).min(width as i32 - 1) as u32;
    let cy = |y: i32| y.max(0).min(height as i32 - 1) as u32;
    let at = |x: u32, y: u32| {
        let i = (y as usize * width as usize + x as usize) * 4;
        [texture[i], texture[i + 1], texture[i + 2], texture[i + 3]]
    };
    let p00 = at(cx(ix), cy(iy));
    let p10 = at(cx(ix + 1), cy(iy));
    let p01 = at(cx(ix), cy(iy + 1));
    let p11 = at(cx(ix + 1), cy(iy + 1));
    let mut out = [0u8; 4];
    for channel in 0..4 {
        let top = p00[channel] as f32 + (p10[channel] as f32 - p00[channel] as f32) * tx;
        let bottom = p01[channel] as f32 + (p11[channel] as f32 - p01[channel] as f32) * tx;
        out[channel] = libm::roundf(top + (bottom - top) * ty).max(0.0).min(255.0) as u8;
    }
    out
}

fn read_pixel(framebuffer: &[u8], offset: usize, format: PixelFormat) -> [u8; 4] {
    match format {
        PixelFormat::Rgba8888 => [
            framebuffer[offset],
            framebuffer[offset + 1],
            framebuffer[offset + 2],
            framebuffer[offset + 3],
        ],
        PixelFormat::Bgra8888 => [
            framebuffer[offset + 2],
            framebuffer[offset + 1],
            framebuffer[offset],
            framebuffer[offset + 3],
        ],
    }
}

fn write_pixel(
    framebuffer: &mut [u8],
    offset: usize,
    format: PixelFormat,
    rgba: [u8; 4],
) {
    match format {
        PixelFormat::Rgba8888 => framebuffer[offset..offset + 4].copy_from_slice(&rgba),
        PixelFormat::Bgra8888 => {
            framebuffer[offset..offset + 4]
                .copy_from_slice(&[rgba[2], rgba[1], rgba[0], rgba[3]])
        }
    }
}

fn blend_pixel(
    framebuffer: &mut [u8],
    offset: usize,
    format: PixelFormat,
    source: [u8; 4],
) {
    let destination = read_pixel(framebuffer, offset, format);
    let a = source[3] as u32;
    if a == 255 {
        write_pixel(framebuffer, offset, format, source);
        return;
    }
    let inverse = 255 - a;
    let out_alpha = a + (destination[3] as u32 * inverse + 127) / 255;
    if out_alpha == 0 {
        write_pixel(framebuffer, offset, format, [0, 0, 0, 0]);
        return;
    }
    let mut out = [0u8; 4];
    for c in 0..3 {
        let premultiplied = source[c] as u32 * a
            + (destination[c] as u32 * destination[3] as u32 * inverse + 127) / 255;
        out[c] = ((premultiplied + out_alpha / 2) / out_alpha).min(255) as u8;
    }
    out[3] = out_alpha.min(255) as u8;
    write_pixel(framebuffer, offset, format, out);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_framebuffer_layout() {
        let mut small = [0u8; 8];
        assert_eq!(
            SoftwareRenderer::new(2, 2, 4, PixelFormat::Rgba8888, &mut small).err(),
            Some(RenderError::StrideTooSmall)
        );
        assert_eq!(
            SoftwareRenderer::new(2, 2, 8, PixelFormat::Rgba8888, &mut small).err(),
            Some(RenderError::FramebufferTooSmall)
        );
    }

    #[test]
    fn rectangle_is_clipped_and_fills_expected_pixels() {
        let mut fb = [0u8; 4 * 3 * 2];
        let mut r = SoftwareRenderer::new(3, 2, 12, PixelFormat::Rgba8888, &mut fb).unwrap();
        r.draw_list(&[Draw::Rect {
            x: -1.0,
            y: 0.0,
            w: 2.0,
            h: 1.0,
            colour: [1.0, 0.0, 0.0, 1.0],
        }]);
        assert_eq!(r.rgba_at(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(r.rgba_at(1, 0), Some([0, 0, 0, 255]));
        assert_eq!(r.rgba_at(0, 1), Some([0, 0, 0, 255]));
    }

    #[test]
    fn texture_blit_uses_nearest_sampling_and_rgba_order() {
        let mut fb = [0u8; 4 * 4];
        let mut r = SoftwareRenderer::new(4, 1, 16, PixelFormat::Rgba8888, &mut fb).unwrap();
        let tex = r
            .create_texture_nearest(
                2,
                1,
                &[255, 0, 0, 255, 0, 255, 0, 255],
            )
            .unwrap();
        r.draw_list(&[Draw::Tex {
            x: 0.0,
            y: 0.0,
            w: 4.0,
            h: 1.0,
            tex,
            alpha: 1.0,
        }]);
        assert_eq!(r.rgba_at(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(r.rgba_at(1, 0), Some([255, 0, 0, 255]));
        assert_eq!(r.rgba_at(2, 0), Some([0, 255, 0, 255]));
        assert_eq!(r.rgba_at(3, 0), Some([0, 255, 0, 255]));
    }

    #[test]
    fn create_texture_matches_linkslot_linear_default() {
        let mut fb = [0u8; 16];
        let mut r = SoftwareRenderer::new(4, 1, 16, PixelFormat::Rgba8888, &mut fb).unwrap();
        let tex = r.create_texture(2, 1, &[255, 0, 0, 255, 0, 0, 255, 255]).unwrap();
        r.draw_list(&[Draw::Tex { x: 0.0, y: 0.0, w: 4.0, h: 1.0, tex, alpha: 1.0 }]);
        assert_eq!(r.rgba_at(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(r.rgba_at(1, 0), Some([191, 0, 64, 255]));
        assert_eq!(r.rgba_at(2, 0), Some([64, 0, 191, 255]));
        assert_eq!(r.rgba_at(3, 0), Some([0, 0, 255, 255]));
    }

    #[test]
    fn update_texture_preserves_original_filter_mode() {
        let mut fb = [0u8; 16];
        let mut r = SoftwareRenderer::new(4, 1, 16, PixelFormat::Rgba8888, &mut fb).unwrap();
        let tex = r.create_texture_nearest(2, 1, &[255, 0, 0, 255, 0, 255, 0, 255]).unwrap();
        assert!(r.update_texture(tex, 2, 1, &[255, 0, 0, 255, 0, 0, 255, 255]).unwrap());
        r.draw_list(&[Draw::Tex { x: 0.0, y: 0.0, w: 4.0, h: 1.0, tex, alpha: 1.0 }]);
        assert_eq!(r.rgba_at(1, 0), Some([255, 0, 0, 255]));
        assert_eq!(r.rgba_at(2, 0), Some([0, 0, 255, 255]));
    }

    #[test]
    fn alpha_blending_composites_over_the_opaque_clear_colour() {
        let mut fb = [0u8; 4];
        let mut r = SoftwareRenderer::new(1, 1, 4, PixelFormat::Rgba8888, &mut fb).unwrap();
        r.draw_list(&[Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
            colour: [1.0, 0.0, 0.0, 0.5],
        }]);
        assert_eq!(r.rgba_at(0, 0), Some([128, 0, 0, 255]));
    }

    #[test]
    fn turned_texture_rotates_around_its_centre() {
        let mut fb = [0u8; 4 * 4 * 4];
        let mut r = SoftwareRenderer::new(4, 4, 16, PixelFormat::Rgba8888, &mut fb).unwrap();
        let tex = r
            .create_texture_nearest(
                2,
                1,
                &[255, 0, 0, 255, 0, 255, 0, 255],
            )
            .unwrap();
        r.draw_list(&[Draw::Turned {
            x: 1.5,
            y: 1.0,
            w: 2.0,
            h: 1.0,
            tex,
            alpha: 1.0,
            turn: core::f32::consts::FRAC_PI_2,
        }]);
        assert_eq!(r.rgba_at(2, 0), Some([255, 0, 0, 255]));
        assert_eq!(r.rgba_at(2, 1), Some([0, 255, 0, 255]));
    }

    #[test]
    fn game_and_shot_markers_preserve_draw_order() {
        let mut fb = [0u8; 4 * 2];
        let mut r = SoftwareRenderer::new(2, 1, 8, PixelFormat::Rgba8888, &mut fb).unwrap();
        r.upload_game_rgba(1, 1, &[0, 0, 255, 255]).unwrap();
        let shot = r
            .create_texture(1, 1, &[255, 0, 0, 255])
            .unwrap();
        r.draw_list(&[
            Draw::Game,
            Draw::Rect {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
                colour: [1.0, 0.0, 0.0, 1.0],
            },
        ]);
        assert_eq!(r.rgba_at(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(r.rgba_at(1, 0), Some([0, 0, 255, 255]));
        r.draw_list(&[Draw::Shot { tex: shot }]);
        assert_eq!(r.rgba_at(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(r.rgba_at(1, 0), Some([255, 0, 0, 255]));
    }

    #[test]
    fn bgra_framebuffer_is_read_back_as_rgba() {
        let mut fb = [0u8; 4];
        let mut r = SoftwareRenderer::new(1, 1, 4, PixelFormat::Bgra8888, &mut fb).unwrap();
        r.draw_list(&[Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
            colour: [0.0, 0.0, 1.0, 1.0],
        }]);
        assert_eq!(r.framebuffer(), &[255, 0, 0, 255]);
        assert_eq!(r.rgba_at(0, 0), Some([0, 0, 255, 255]));
    }

    #[test]
    fn deleting_a_texture_does_not_recycle_its_id() {
        let mut fb = [0u8; 4];
        let mut r = SoftwareRenderer::new(1, 1, 4, PixelFormat::Rgba8888, &mut fb).unwrap();
        let old = r.create_texture(1, 1, &[1, 2, 3, 255]).unwrap();
        assert!(r.delete_texture(old));
        let new = r.create_texture(1, 1, &[4, 5, 6, 255]).unwrap();
        assert_ne!(old, new);
        assert!(!r.delete_texture(old));
    }

    #[test]
    fn bgra_game_upload_matches_the_existing_gl_upload_byte_order() {
        let mut fb = [0u8; 4];
        let mut r = SoftwareRenderer::new(1, 1, 4, PixelFormat::Rgba8888, &mut fb).unwrap();
        r.upload_game_bgra(1, 1, &[30, 20, 10, 255]).unwrap();
        r.draw_list(&[Draw::Game]);
        assert_eq!(r.rgba_at(0, 0), Some([10, 20, 30, 255]));
    }
}
