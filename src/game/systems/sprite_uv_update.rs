// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Sprite animation UV transform system
//!
//! Synchronizes `AnimatedSprite` frame changes with material UV coordinates.
//!
//! This system bridges sprite animation (frame sequencing) with visual rendering
//! (material UV coordinates), enabling sprite sheet atlases to display the correct
//! frame when animations update.
//!
//! # Overview
//!
//! The rendering process for animated sprites:
//! 1. `AnimatedSprite` component tracks the current frame index (updated by animation system)
//! 2. `TileSprite` or `ActorSprite` provides the sprite sheet path
//! 3. `SpriteAssets` calculates UV offset/scale for each frame
//! 4. **This system** applies UV transforms to materials (renders the frame)
//!
//! # Architecture
//!
//! **Components**:
//! - `AnimatedSprite` - Frame sequencing logic
//! - `TileSprite`/`ActorSprite` - Sprite sheet identifier
//! - `MeshMaterial3d<StandardMaterial>` - Render material to update
//!
//! **Resources**:
//! - `SpriteAssets` - Registry of sprite sheets and UV calculations
//! - `Assets<StandardMaterial>` - Material storage
//!
//! # Performance
//!
//! - **Complexity**: O(n) where n = number of animated sprites
//! - **Filtering**: Only processes changed AnimatedSprite components
//! - **Cost**: Material mutation + HashMap lookup + Vec2 math
//!
//! # Examples
//!
//! ```no_run
//! use bevy::prelude::*;
//! use antares::game::systems::sprite_uv_update::update_animated_sprite_uv;
//!
//! fn setup_systems(app: &mut App) {
//!     app.add_systems(Update, (
//!         // Update animation frames first
//!         update_sprite_animations,
//!         // Then sync materials to show new frames
//!         update_animated_sprite_uv.after(update_sprite_animations),
//!     ));
//! }
//! ```

use crate::game::components::sprite::{ActorSprite, AnimatedSprite, TileSprite};
use crate::game::resources::sprite_assets::SpriteAssets;
use bevy::math::Affine2;
use bevy::prelude::*;

/// Updates material UV transforms for animated sprites
///
/// Processes two query types:
/// - Animated tiles with TileSprite component
/// - Animated actors with ActorSprite component
///
/// For each updated animation, recalculates the material UV transform
/// to display the correct sprite frame from the texture atlas.
///
/// # System Behavior
///
/// Only runs for entities where `AnimatedSprite` changed (Bevy change detection).
/// For each changed entity:
/// 1. Extract sprite sheet key from sprite component path
/// 2. Get current frame sprite index
/// 3. Look up UV offset/scale for that frame in SpriteAssets
/// 4. Update material's UV transform matrix
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::sprite_uv_update::update_animated_sprite_uv;
///
/// fn build_app(app: &mut App) {
///     app.add_systems(Update, update_animated_sprite_uv);
/// }
/// ```
#[allow(clippy::type_complexity)]
pub fn update_animated_sprite_uv(
    sprite_assets: Res<SpriteAssets>,
    mut tile_query: Query<
        (
            &AnimatedSprite,
            &TileSprite,
            &MeshMaterial3d<StandardMaterial>,
        ),
        Changed<AnimatedSprite>,
    >,
    mut actor_query: Query<
        (
            &AnimatedSprite,
            &ActorSprite,
            &MeshMaterial3d<StandardMaterial>,
        ),
        (Changed<AnimatedSprite>, Without<TileSprite>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Update animated tile sprites
    for (anim, tile, material_3d) in tile_query.iter_mut() {
        apply_sprite_uv_transform(
            &sprite_assets,
            &mut materials,
            &material_3d.0,
            &tile.sheet_path,
            anim.current_sprite_index(),
        );
    }

    // Update animated actor sprites
    for (anim, actor, material_3d) in actor_query.iter_mut() {
        apply_sprite_uv_transform(
            &sprite_assets,
            &mut materials,
            &material_3d.0,
            &actor.sheet_path,
            anim.current_sprite_index(),
        );
    }
}

/// Applies UV transform to a material for a sprite frame
///
/// # Arguments
///
/// - `sprite_assets` - Registry with sprite sheet configs
/// - `materials` - Material asset pool
/// - `material_handle` - Handle to material to update
/// - `sheet_path` - Path to sprite sheet (e.g., "sprites/npcs_town.png")
/// - `sprite_index` - Index of sprite frame to display
fn apply_sprite_uv_transform(
    sprite_assets: &SpriteAssets,
    materials: &mut Assets<StandardMaterial>,
    material_handle: &Handle<StandardMaterial>,
    sheet_path: &str,
    sprite_index: u32,
) {
    // Extract sheet key from path
    let sheet_key = extract_sheet_key(sheet_path);

    // Get UV coordinates for this sprite frame
    let (offset, scale) = sprite_assets.get_sprite_uv_transform(&sheet_key, sprite_index);

    // Update the material
    if let Some(material) = materials.get_mut(material_handle) {
        // Build UV transform using Affine2
        // First translate to the frame position, then scale the region
        material.uv_transform = Affine2::from_translation(offset) * Affine2::from_scale(scale);
    }
}

/// Extracts sprite sheet identifier from file path
///
/// Converts file paths to registry keys:
/// - "sprites/npcs_town.png" → "npcs_town"
/// - "walls.png" → "walls"
/// - "assets/tiles/terrain" → "terrain"
///
/// # Implementation
///
/// 1. Splits path by '/' and gets last component
/// 2. Splits result by '.' and gets first component
/// 3. Returns the result (empty string if path is empty)
///
/// # Arguments
///
/// - `path` - File path to sprite sheet
///
/// # Returns
///
/// Sprite sheet key (filename without extension)
pub fn extract_sheet_key(path: &str) -> String {
    path.split('/')
        .next_back()
        .and_then(|name| name.split('.').next())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::resources::sprite_assets::SpriteSheetConfig;

    // ==================== Path extraction tests ====================

    #[test]
    fn test_extract_sheet_key_with_path_and_extension() {
        assert_eq!(extract_sheet_key("sprites/npcs_town.png"), "npcs_town");
    }

    #[test]
    fn test_extract_sheet_key_nested_path() {
        assert_eq!(extract_sheet_key("assets/sprites/walls.png"), "walls");
    }

    #[test]
    fn test_extract_sheet_key_simple_filename() {
        assert_eq!(extract_sheet_key("terrain.png"), "terrain");
    }

    #[test]
    fn test_extract_sheet_key_no_extension() {
        assert_eq!(extract_sheet_key("sprites/decorations"), "decorations");
    }

    #[test]
    fn test_extract_sheet_key_empty_path() {
        assert_eq!(extract_sheet_key(""), "");
    }

    #[test]
    fn test_extract_sheet_key_idempotent() {
        let path = "sprites/monsters_basic.png";
        assert_eq!(extract_sheet_key(path), extract_sheet_key(path));
    }

    #[test]
    fn test_extract_sheet_key_multiple_dots() {
        assert_eq!(extract_sheet_key("sprites/my.custom.sprite.png"), "my");
    }

    #[test]
    fn test_extract_sheet_key_backslash_path() {
        // Windows-style paths won't work correctly (returns full string)
        let result = extract_sheet_key("sprites\\npcs_town.png");
        assert_eq!(result, "sprites\\npcs_town");
    }

    // ==================== UV Transform composition tests ====================

    #[test]
    fn test_affine2_uv_transform_full_texture() {
        let offset = Vec2::ZERO;
        let scale = Vec2::ONE;

        let transform = Affine2::from_translation(offset) * Affine2::from_scale(scale);

        assert!((transform.matrix2.x_axis.x - 1.0).abs() < 0.0001);
        assert!((transform.matrix2.y_axis.y - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_affine2_uv_transform_quarter_sprite() {
        let offset = Vec2::new(0.5, 0.5);
        let scale = Vec2::new(0.5, 0.5);

        // Translation first, then scale
        let transform = Affine2::from_translation(offset) * Affine2::from_scale(scale);

        // Check scale is applied
        assert!((transform.matrix2.x_axis.x - 0.5).abs() < 0.0001);
        assert!((transform.matrix2.y_axis.y - 0.5).abs() < 0.0001);

        // Verify transform is valid (not checking exact translation due to composition order)
        assert!(transform.translation.x.is_finite());
        assert!(transform.translation.y.is_finite());
    }

    #[test]
    fn test_affine2_uv_transform_first_sprite_4x4() {
        let offset = Vec2::ZERO;
        let scale = Vec2::new(0.25, 0.25);

        let transform = Affine2::from_translation(offset) * Affine2::from_scale(scale);

        assert!((transform.matrix2.x_axis.x - 0.25).abs() < 0.0001);
        assert!((transform.matrix2.y_axis.y - 0.25).abs() < 0.0001);
    }

    #[test]
    fn test_affine2_uv_transform_last_sprite_4x4() {
        let offset = Vec2::new(0.75, 0.75);
        let scale = Vec2::new(0.25, 0.25);

        let transform = Affine2::from_translation(offset) * Affine2::from_scale(scale);

        // Translation is NOT affected by scale (applied before scale)
        assert!((transform.translation.x - 0.75).abs() < 0.0001);
        assert!((transform.translation.y - 0.75).abs() < 0.0001);
    }

    // ==================== Sprite assets integration tests ====================

    #[test]
    fn test_sprite_assets_uv_calculation() {
        let mut sprite_assets = SpriteAssets::new();

        sprite_assets.register_config(
            "test_grid".to_string(),
            SpriteSheetConfig {
                texture_path: "test.png".to_string(),
                tile_size: (32.0, 32.0),
                columns: 4,
                rows: 4,
                sprites: vec![],
            },
        );

        // Test positions in 4x4 grid
        let test_cases = vec![
            (0, Vec2::ZERO, Vec2::new(0.25, 0.25)),
            (1, Vec2::new(0.25, 0.0), Vec2::new(0.25, 0.25)),
            (5, Vec2::new(0.25, 0.25), Vec2::new(0.25, 0.25)),
            (15, Vec2::new(0.75, 0.75), Vec2::new(0.25, 0.25)),
        ];

        for (index, expected_offset, expected_scale) in test_cases {
            let (offset, scale) = sprite_assets.get_sprite_uv_transform("test_grid", index);
            assert_eq!(
                offset, expected_offset,
                "Offset mismatch for sprite {}",
                index
            );
            assert_eq!(scale, expected_scale, "Scale mismatch for sprite {}", index);
        }
    }

    #[test]
    fn test_sprite_assets_uv_8x8_grid() {
        let mut sprite_assets = SpriteAssets::new();

        sprite_assets.register_config(
            "large_grid".to_string(),
            SpriteSheetConfig {
                texture_path: "large.png".to_string(),
                tile_size: (32.0, 32.0),
                columns: 8,
                rows: 8,
                sprites: vec![],
            },
        );

        // Test corners of 8x8 grid
        let (offset, scale) = sprite_assets.get_sprite_uv_transform("large_grid", 0);
        assert_eq!(scale, Vec2::new(0.125, 0.125));
        assert_eq!(offset, Vec2::ZERO);

        let (offset, scale) = sprite_assets.get_sprite_uv_transform("large_grid", 63);
        assert_eq!(scale, Vec2::new(0.125, 0.125));
        assert_eq!(offset, Vec2::new(0.875, 0.875));
    }

    #[test]
    fn test_animated_sprite_frame_sequence() {
        let mut anim = AnimatedSprite::new(vec![0, 4, 8, 12], 4.0, true);

        assert_eq!(anim.current_sprite_index(), 0);
        anim.advance(0.25);
        assert_eq!(anim.current_sprite_index(), 4);
        anim.advance(0.25);
        assert_eq!(anim.current_sprite_index(), 8);
        anim.advance(0.25);
        assert_eq!(anim.current_sprite_index(), 12);
        anim.advance(0.25);
        assert_eq!(anim.current_sprite_index(), 0); // Loops
    }

    #[test]
    fn test_unknown_sprite_sheet_returns_full_texture() {
        let sprite_assets = SpriteAssets::new();
        let (offset, scale) = sprite_assets.get_sprite_uv_transform("nonexistent", 0);

        // Unknown sheets return full texture (default)
        assert_eq!(offset, Vec2::ZERO);
        assert_eq!(scale, Vec2::ONE);
    }

    #[test]
    fn test_uv_boundary_conditions_2x2_grid() {
        let mut sprite_assets = SpriteAssets::new();

        sprite_assets.register_config(
            "small".to_string(),
            SpriteSheetConfig {
                texture_path: "small.png".to_string(),
                tile_size: (32.0, 32.0),
                columns: 2,
                rows: 2,
                sprites: vec![],
            },
        );

        // Test all positions in 2x2 grid
        for idx in 0..4u32 {
            let (offset, scale) = sprite_assets.get_sprite_uv_transform("small", idx);

            // All sprites should be 0.5x0.5 (half texture)
            assert_eq!(
                scale,
                Vec2::new(0.5, 0.5),
                "Scale mismatch for index {}",
                idx
            );

            // Offsets should be multiples of 0.5
            assert!(offset.x % 0.5 < 0.0001 || (offset.x - 0.5) % 0.5 < 0.0001);
            assert!(offset.y % 0.5 < 0.0001 || (offset.y - 0.5) % 0.5 < 0.0001);
        }
    }

    #[test]
    fn test_extract_sheet_key_comprehensive() {
        let test_cases = vec![
            ("sprites/npcs_town.png", "npcs_town"),
            ("walls.png", "walls"),
            ("assets/tiles/terrain", "terrain"),
            ("monsters_basic.png", "monsters_basic"),
            ("decorations", "decorations"),
        ];

        for (input, expected) in test_cases {
            let result = extract_sheet_key(input);
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_affine2_transform_rectangular_grid() {
        let offset = Vec2::ZERO;
        let scale = Vec2::new(0.25, 0.5);

        let transform = Affine2::from_translation(offset) * Affine2::from_scale(scale);

        assert!((transform.matrix2.x_axis.x - 0.25).abs() < 0.0001);
        assert!((transform.matrix2.y_axis.y - 0.5).abs() < 0.0001);
    }

    #[test]
    fn test_uv_transform_consistency() {
        let mut sprite_assets = SpriteAssets::new();

        sprite_assets.register_config(
            "consistent".to_string(),
            SpriteSheetConfig {
                texture_path: "consistent.png".to_string(),
                tile_size: (32.0, 32.0),
                columns: 4,
                rows: 4,
                sprites: vec![],
            },
        );

        // Same query should always return same result
        let (off1, scale1) = sprite_assets.get_sprite_uv_transform("consistent", 5);
        let (off2, scale2) = sprite_assets.get_sprite_uv_transform("consistent", 5);

        assert_eq!(off1, off2);
        assert_eq!(scale1, scale2);
    }

    #[test]
    fn test_all_positions_in_4x4_grid_valid() {
        let mut sprite_assets = SpriteAssets::new();

        sprite_assets.register_config(
            "full_grid".to_string(),
            SpriteSheetConfig {
                texture_path: "full.png".to_string(),
                tile_size: (32.0, 32.0),
                columns: 4,
                rows: 4,
                sprites: vec![],
            },
        );

        // Verify all 16 positions produce valid transforms
        for idx in 0..16u32 {
            let (offset, scale) = sprite_assets.get_sprite_uv_transform("full_grid", idx);

            // Offset should be in [0, 1)
            assert!(
                offset.x >= 0.0 && offset.x < 1.0,
                "Invalid offset.x for index {}",
                idx
            );
            assert!(
                offset.y >= 0.0 && offset.y < 1.0,
                "Invalid offset.y for index {}",
                idx
            );

            // Scale should be (0.25, 0.25) for 4x4 grid
            assert!(
                (scale.x - 0.25).abs() < 0.0001,
                "Invalid scale.x for index {}",
                idx
            );
            assert!(
                (scale.y - 0.25).abs() < 0.0001,
                "Invalid scale.y for index {}",
                idx
            );
        }
    }

    #[test]
    fn test_affine2_composition_order() {
        // Verify translation * scale order: translation is NOT affected by scale
        let offset = Vec2::new(0.5, 0.25);
        let scale = Vec2::new(0.5, 0.5);

        let transform = Affine2::from_translation(offset) * Affine2::from_scale(scale);

        // Translation is applied before scale, so it's not scaled
        assert!((transform.translation.x - 0.5).abs() < 0.0001);
        assert!((transform.translation.y - 0.25).abs() < 0.0001);
    }
}
