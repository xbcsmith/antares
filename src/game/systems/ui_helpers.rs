// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared UI helper functions and constants for Bevy UI text styling and image
//! creation.
//!
//! These helpers reduce boilerplate across combat, HUD, menu, rest, and game
//! log systems where identical [`TextFont`] / [`TextColor`] patterns are
//! repeated many times.
//!
//! # Text Style Helper
//!
//! The [`text_style`] function returns a `(TextFont, TextColor)` tuple that
//! can be placed directly inside a Bevy `spawn((...))` call alongside other
//! components.  Bevy accepts nested tuples as bundles, so the returned pair
//! merges seamlessly:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use antares::game::systems::ui_helpers::{text_style, BODY_FONT_SIZE};
//! # fn example(mut commands: Commands) {
//! commands.spawn((
//!     Text::new("Hello"),
//!     text_style(BODY_FONT_SIZE, Color::WHITE),
//! ));
//! # }
//! ```
//!
//! # Image Helper
//!
//! [`create_blank_rgba_image`] produces a square, fully-transparent RGBA8
//! texture suitable for the mini-map and automap backing images.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Standard body-text font size (16 px).
///
/// Used across settings labels, combat victory/defeat summaries, turn-order
/// text, and other general-purpose UI text.
pub const BODY_FONT_SIZE: f32 = 16.0;

/// Standard label / small-text font size (14 px).
///
/// Used in automap legend entries, combat enemy-name cards, action-button
/// labels, and the game-log header.
pub const LABEL_FONT_SIZE: f32 = 14.0;

/// Creates a ([`TextFont`], [`TextColor`]) bundle pair with the given size and
/// color.
///
/// Because Bevy bundles accept nested tuples, the returned pair can be placed
/// directly inside a `spawn((...))` call alongside other components:
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use antares::game::systems::ui_helpers::text_style;
/// # fn example(mut commands: Commands) {
/// commands.spawn((
///     Text::new("hello"),
///     text_style(16.0, Color::WHITE),
/// ));
/// # }
/// ```
///
/// # Arguments
///
/// * `font_size` — Font size in logical pixels.
/// * `color`     — Text color applied via [`TextColor`].
///
/// # Returns
///
/// A `(TextFont, TextColor)` tuple ready for insertion into an entity.
pub fn text_style(font_size: f32, color: Color) -> (TextFont, TextColor) {
    (
        TextFont {
            font_size,
            ..default()
        },
        TextColor(color),
    )
}

/// Creates a square RGBA8 image filled with transparent black pixels.
///
/// The returned [`Image`] uses [`TextureFormat::Rgba8UnormSrgb`] and is
/// flagged for all render-asset usages so it can be written to by CPU-side
/// map-painting logic and simultaneously displayed by the GPU.
///
/// This is used by the mini-map and automap initialisation systems to create
/// the backing texture that is later painted by the map rendering logic.
///
/// # Arguments
///
/// * `size` — Width **and** height of the square image in pixels.
///
/// # Returns
///
/// A new [`Image`] of dimensions `size × size` with every pixel set to
/// `[0, 0, 0, 0]`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::ui_helpers::create_blank_rgba_image;
///
/// let img = create_blank_rgba_image(64);
/// assert_eq!(img.width(), 64);
/// assert_eq!(img.height(), 64);
/// let data = img.data.as_ref().expect("image data should be present");
/// assert_eq!(data.len(), 64 * 64 * 4);
/// ```
pub fn create_blank_rgba_image(size: u32) -> Image {
    Image::new_fill(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &vec![0u8; (size * size * 4) as usize],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_style_returns_correct_font_size() {
        let (font, _color) = text_style(20.0, Color::WHITE);
        assert!((font.font_size - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_text_style_returns_correct_color() {
        let (_font, color) = text_style(14.0, Color::WHITE);
        assert_eq!(color.0, Color::WHITE);
    }

    #[test]
    fn test_body_font_size_value() {
        assert!((BODY_FONT_SIZE - 16.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_label_font_size_value() {
        assert!((LABEL_FONT_SIZE - 14.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_create_blank_rgba_image_dimensions() {
        let img = create_blank_rgba_image(32);
        assert_eq!(img.width(), 32);
        assert_eq!(img.height(), 32);
    }

    #[test]
    fn test_create_blank_rgba_image_data_length() {
        let img = create_blank_rgba_image(16);
        let data = img.data.as_ref().expect("image data should be present");
        // 16 * 16 pixels * 4 bytes (RGBA)
        assert_eq!(data.len(), 16 * 16 * 4);
    }

    #[test]
    fn test_create_blank_rgba_image_all_zeros() {
        let img = create_blank_rgba_image(8);
        let data = img.data.as_ref().expect("image data should be present");
        assert!(data.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_create_blank_rgba_image_size_one() {
        let img = create_blank_rgba_image(1);
        let data = img.data.as_ref().expect("image data should be present");
        assert_eq!(data.len(), 4);
        assert_eq!(data.as_slice(), &[0u8, 0, 0, 0]);
    }
}
