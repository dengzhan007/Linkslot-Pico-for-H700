#![no_std]

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum VideoMode {
    #[default]
    Actual,
    /// GB/GBC: crop the padded canvas to the picture and stretch it to fill.
    /// FC/SFC/MD: fill the height with the source aspect, keeping side bars.
    Stretch,
}

impl VideoMode {
    pub fn as_str(self) -> &'static str {
        match self {
            VideoMode::Actual => "actual",
            VideoMode::Stretch => "stretch",
        }
    }

    /// Parse the persisted option without allocating a lowercase String.
    pub fn parse(value: &str) -> Option<VideoMode> {
        let value = value.trim();
        if value.eq_ignore_ascii_case("actual") {
            Some(VideoMode::Actual)
        } else if value.eq_ignore_ascii_case("stretch") {
            Some(VideoMode::Stretch)
        } else {
            None
        }
    }
}

/// Compute the normalized source crop for a renderer.
///
/// The first tuple is the full frame supplied by the core; the second is the
/// picture area inside that frame. Destination fitting remains a separate
/// renderer/layout responsibility.
pub fn source_rect(frame: (u32, u32), picture: (u32, u32), mode: VideoMode) -> [f32; 4] {
    const WHOLE_TEXTURE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
    let (frame_width, frame_height) = frame;
    let (picture_width, picture_height) = picture;
    if mode == VideoMode::Actual
        || (picture_width, picture_height) == (frame_width, frame_height)
    {
        return WHOLE_TEXTURE;
    }
    if frame_width == 0 || frame_height == 0 {
        return WHOLE_TEXTURE;
    }
    let x = frame_width.saturating_sub(picture_width) / 2;
    let y = frame_height.saturating_sub(picture_height) / 2;
    [
        x as f32 / frame_width as f32,
        y as f32 / frame_height as f32,
        picture_width as f32 / frame_width as f32,
        picture_height as f32 / frame_height as f32,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_serialization_and_parse_are_stable() {
        assert_eq!(VideoMode::default(), VideoMode::Actual);
        assert_eq!(VideoMode::Actual.as_str(), "actual");
        assert_eq!(VideoMode::Stretch.as_str(), "stretch");
        assert_eq!(VideoMode::parse("ACTUAL"), Some(VideoMode::Actual));
        assert_eq!(VideoMode::parse(" Stretch "), Some(VideoMode::Stretch));
        assert_eq!(VideoMode::parse("unknown"), None);
    }

    #[test]
    fn actual_mode_always_uses_the_full_source() {
        assert_eq!(
            source_rect((240, 160), (160, 144), VideoMode::Actual),
            [0.0, 0.0, 1.0, 1.0]
        );
    }

    #[test]
    fn stretch_mode_crops_the_picture_area_from_a_padded_frame() {
        let rect = source_rect((240, 160), (160, 144), VideoMode::Stretch);
        assert!((rect[0] - (40.0 / 240.0)).abs() < 0.00001);
        assert!((rect[1] - (8.0 / 160.0)).abs() < 0.00001);
        assert!((rect[2] - (160.0 / 240.0)).abs() < 0.00001);
        assert!((rect[3] - (144.0 / 160.0)).abs() < 0.00001);
    }

    #[test]
    fn a_full_picture_frame_never_adds_a_crop() {
        assert_eq!(
            source_rect((256, 224), (256, 224), VideoMode::Stretch),
            [0.0, 0.0, 1.0, 1.0]
        );
    }

    #[test]
    fn zero_sized_frame_returns_safe_full_source() {
        assert_eq!(
            source_rect((0, 0), (160, 144), VideoMode::Stretch),
            [0.0, 0.0, 1.0, 1.0]
        );
    }

    #[test]
    fn oversized_picture_uses_saturating_origin_like_the_upstream_code() {
        let rect = source_rect((160, 144), (240, 160), VideoMode::Stretch);
        assert_eq!(rect[0], 0.0);
        assert_eq!(rect[1], 0.0);
        assert!(rect[2] > 1.0);
        assert!(rect[3] > 1.0);
    }
}
