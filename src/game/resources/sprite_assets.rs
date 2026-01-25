// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Sprite asset management using native Bevy PBR
//!
//! This module provides sprite sheet loading, material caching, and UV transform
//! calculation for billboard-based sprite rendering using Bevy's PBR system.
//!
//! # Architecture
//!
//! - Uses `StandardMaterial` with alpha blending for sprite textures
//! - Caches materials per sprite sheet to minimize draw calls
//! - Provides UV transforms for texture atlas sprite selection
//! - No external dependencies - native Bevy only
//!
//! # Examples
//!
//! ```no_run
//! use bevy::prelude::*;
//! use antares::game::resources::sprite_assets::SpriteAssets;
//!
//! fn setup_sprites(
//!     mut sprite_assets: ResMut<SpriteAssets>,
//!     asset_server: Res<AssetServer>,
//!     mut materials: ResMut<Assets<StandardMaterial>>,
//!     mut meshes: ResMut<Assets<Mesh>>,
//! ) {
//!     // Load sprite sheet material
//!     let material = sprite_assets.get_or_load_material(
//!         "sprites/walls.png",
//!         &asset_server,
//!         &mut materials,
//!     );
//!
//!     // Create quad mesh for sprite
//!     let mesh = sprite_assets.get_or_load_mesh((1.0, 2.0), &mut meshes);
//!
//!     // Get UV transform for sprite at index 5 in 4x4 grid
//!     let (offset, scale) = sprite_assets.get_sprite_uv_transform("walls", 5);
//! }
//! ```

use bevy::prelude::*;
use std::collections::HashMap;

/// Configuration for a sprite sheet (texture atlas)
///
/// # Examples
///
/// ```
/// use antares::game::resources::sprite_assets::SpriteSheetConfig;
///
/// let config = SpriteSheetConfig {
///     texture_path: "sprites/walls.png".to_string(),
///     tile_size: (128.0, 256.0),
///     columns: 4,
///     rows: 4,
///     sprites: vec![
///         (0, "stone_wall".to_string()),
///         (1, "brick_wall".to_string()),
///     ],
/// };
///
/// assert_eq!(config.columns, 4);
/// assert_eq!(config.rows, 4);
/// ```
#[derive(Debug, Clone)]
pub struct SpriteSheetConfig {
    /// Path to sprite sheet texture (relative to assets/)
    pub texture_path: String,

    /// Size of each sprite in pixels (width, height)
    pub tile_size: (f32, f32),

    /// Number of columns in sprite grid
    pub columns: u32,

    /// Number of rows in sprite grid
    pub rows: u32,

    /// Named sprite mappings (index, name)
    pub sprites: Vec<(u32, String)>,
}

/// Resource managing sprite materials and meshes for billboard rendering
///
/// # Architecture
///
/// - **Materials**: Cached `StandardMaterial` per sprite sheet (texture + alpha blend)
/// - **Meshes**: Cached `Rectangle` meshes per sprite size
/// - **Configs**: Sprite sheet configurations for UV calculations
///
/// # Performance
///
/// - Material caching reduces draw calls (one material per sprite sheet)
/// - Mesh caching avoids duplicate geometry (one mesh per size)
/// - UV transforms calculated on-demand (no runtime overhead)
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::resources::sprite_assets::SpriteAssets;
///
/// fn sprite_system(sprite_assets: Res<SpriteAssets>) {
///     if let Some(config) = sprite_assets.get_config("npcs_town") {
///         println!("NPC sprite sheet: {}x{} grid", config.columns, config.rows);
///     }
/// }
/// ```
#[derive(Resource)]
pub struct SpriteAssets {
    /// Cached materials per sprite sheet path
    materials: HashMap<String, Handle<StandardMaterial>>,

    /// Cached meshes per sprite size (key: "widthxheight")
    meshes: HashMap<String, Handle<Mesh>>,

    /// Sprite sheet configurations (key: sheet identifier)
    configs: HashMap<String, SpriteSheetConfig>,
}

impl SpriteAssets {
    /// Create new empty sprite assets resource
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::sprite_assets::SpriteAssets;
    ///
    /// let sprite_assets = SpriteAssets::new();
    /// ```
    pub fn new() -> Self {
        Self {
            materials: HashMap::new(),
            meshes: HashMap::new(),
            configs: HashMap::new(),
        }
    }

    /// Get or create PBR material for a sprite sheet
    ///
    /// Materials are cached per sprite sheet path to minimize draw calls.
    ///
    /// # Material Configuration
    ///
    /// - `base_color_texture`: Loaded from `sheet_path`
    /// - `alpha_mode`: `AlphaMode::Blend` (supports transparency)
    /// - `unlit`: `false` (uses PBR lighting)
    /// - `perceptual_roughness`: `0.9` (slightly rough for depth)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use bevy::prelude::*;
    /// use antares::game::resources::sprite_assets::SpriteAssets;
    ///
    /// fn load_sprite_material(
    ///     mut sprite_assets: ResMut<SpriteAssets>,
    ///     asset_server: Res<AssetServer>,
    ///     mut materials: ResMut<Assets<StandardMaterial>>,
    /// ) {
    ///     let material = sprite_assets.get_or_load_material(
    ///         "sprites/npcs_town.png",
    ///         &asset_server,
    ///         &mut materials,
    ///     );
    /// }
    /// ```
    pub fn get_or_load_material(
        &mut self,
        sheet_path: &str,
        asset_server: &AssetServer,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        if let Some(handle) = self.materials.get(sheet_path) {
            return handle.clone();
        }

        let texture_handle = asset_server.load::<Image>(sheet_path.to_string());
        let material = StandardMaterial {
            base_color_texture: Some(texture_handle),
            alpha_mode: AlphaMode::Blend,
            unlit: false, // Use PBR lighting for depth
            perceptual_roughness: 0.9,
            ..default()
        };

        let handle = materials.add(material);
        self.materials
            .insert(sheet_path.to_string(), handle.clone());
        handle
    }

    /// Get or create quad mesh for sprite rendering
    ///
    /// Meshes are cached per size to avoid duplicate geometry.
    ///
    /// # Arguments
    ///
    /// * `sprite_size` - (width, height) in world units (1 unit ≈ 1 meter)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use bevy::prelude::*;
    /// use antares::game::resources::sprite_assets::SpriteAssets;
    ///
    /// fn create_sprite_mesh(
    ///     mut sprite_assets: ResMut<SpriteAssets>,
    ///     mut meshes: ResMut<Assets<Mesh>>,
    /// ) {
    ///     // Create 1m x 2m quad for character sprite
    ///     let mesh = sprite_assets.get_or_load_mesh((1.0, 2.0), &mut meshes);
    /// }
    /// ```
    pub fn get_or_load_mesh(
        &mut self,
        sprite_size: (f32, f32),
        meshes: &mut Assets<Mesh>,
    ) -> Handle<Mesh> {
        let key = format!("{}x{}", sprite_size.0, sprite_size.1);

        if let Some(handle) = self.meshes.get(&key) {
            return handle.clone();
        }

        let mesh = Rectangle::new(sprite_size.0, sprite_size.1);
        let handle = meshes.add(mesh);
        self.meshes.insert(key, handle.clone());
        handle
    }

    /// Calculate UV transform for sprite at index in atlas
    ///
    /// Returns (offset, scale) for UV coordinates.
    /// Sprites are indexed in row-major order (left-to-right, top-to-bottom).
    ///
    /// # Arguments
    ///
    /// * `sheet_key` - Sprite sheet identifier (registered config)
    /// * `sprite_index` - Sprite index (0-indexed, row-major order)
    ///
    /// # Returns
    ///
    /// * `(offset, scale)` - UV offset and scale as Vec2
    /// * `(Vec2::ZERO, Vec2::ONE)` - If sheet not found (full texture)
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use antares::game::resources::sprite_assets::{SpriteAssets, SpriteSheetConfig};
    ///
    /// let mut sprite_assets = SpriteAssets::new();
    ///
    /// // Register 4x4 sprite sheet
    /// sprite_assets.register_config("walls".to_string(), SpriteSheetConfig {
    ///     texture_path: "sprites/walls.png".to_string(),
    ///     tile_size: (128.0, 128.0),
    ///     columns: 4,
    ///     rows: 4,
    ///     sprites: vec![],
    /// });
    ///
    /// // Get UV transform for sprite at index 5 (row 1, col 1)
    /// let (offset, scale) = sprite_assets.get_sprite_uv_transform("walls", 5);
    ///
    /// assert_eq!(scale, Vec2::new(0.25, 0.25)); // 1/4 of texture
    /// assert_eq!(offset, Vec2::new(0.25, 0.25)); // Second column, second row
    /// ```
    pub fn get_sprite_uv_transform(&self, sheet_key: &str, sprite_index: u32) -> (Vec2, Vec2) {
        if let Some(config) = self.configs.get(sheet_key) {
            let col = sprite_index % config.columns;
            let row = sprite_index / config.columns;

            let u_scale = 1.0 / config.columns as f32;
            let v_scale = 1.0 / config.rows as f32;

            let u_offset = col as f32 * u_scale;
            let v_offset = row as f32 * v_scale;

            (Vec2::new(u_offset, v_offset), Vec2::new(u_scale, v_scale))
        } else {
            // Default: use full texture
            (Vec2::ZERO, Vec2::ONE)
        }
    }

    /// Register sprite sheet configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::sprite_assets::{SpriteAssets, SpriteSheetConfig};
    ///
    /// let mut sprite_assets = SpriteAssets::new();
    ///
    /// sprite_assets.register_config("walls".to_string(), SpriteSheetConfig {
    ///     texture_path: "sprites/walls.png".to_string(),
    ///     tile_size: (128.0, 256.0),
    ///     columns: 4,
    ///     rows: 4,
    ///     sprites: vec![(0, "stone_wall".to_string())],
    /// });
    /// ```
    pub fn register_config(&mut self, key: String, config: SpriteSheetConfig) {
        self.configs.insert(key, config);
    }

    /// Get sprite sheet configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::sprite_assets::{SpriteAssets, SpriteSheetConfig};
    ///
    /// let mut sprite_assets = SpriteAssets::new();
    /// sprite_assets.register_config("walls".to_string(), SpriteSheetConfig {
    ///     texture_path: "sprites/walls.png".to_string(),
    ///     tile_size: (128.0, 256.0),
    ///     columns: 4,
    ///     rows: 4,
    ///     sprites: vec![],
    /// });
    ///
    /// let config = sprite_assets.get_config("walls").unwrap();
    /// assert_eq!(config.columns, 4);
    /// ```
    pub fn get_config(&self, key: &str) -> Option<&SpriteSheetConfig> {
        self.configs.get(key)
    }
}

impl Default for SpriteAssets {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprite_assets_new() {
        let assets = SpriteAssets::new();
        assert_eq!(assets.materials.len(), 0);
        assert_eq!(assets.meshes.len(), 0);
        assert_eq!(assets.configs.len(), 0);
    }

    #[test]
    fn test_register_and_get_config() {
        let mut assets = SpriteAssets::new();

        let config = SpriteSheetConfig {
            texture_path: "test.png".to_string(),
            tile_size: (32.0, 32.0),
            columns: 4,
            rows: 4,
            sprites: vec![],
        };

        assets.register_config("test".to_string(), config);

        let retrieved = assets.get_config("test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().columns, 4);
    }

    #[test]
    fn test_uv_transform_4x4_grid() {
        let mut assets = SpriteAssets::new();

        assets.register_config(
            "test".to_string(),
            SpriteSheetConfig {
                texture_path: "test.png".to_string(),
                tile_size: (32.0, 32.0),
                columns: 4,
                rows: 4,
                sprites: vec![],
            },
        );

        // Test sprite at index 0 (top-left)
        let (offset, scale) = assets.get_sprite_uv_transform("test", 0);
        assert_eq!(offset, Vec2::new(0.0, 0.0));
        assert_eq!(scale, Vec2::new(0.25, 0.25));

        // Test sprite at index 5 (row 1, col 1)
        let (offset, scale) = assets.get_sprite_uv_transform("test", 5);
        assert_eq!(offset, Vec2::new(0.25, 0.25));
        assert_eq!(scale, Vec2::new(0.25, 0.25));

        // Test sprite at index 15 (bottom-right)
        let (offset, scale) = assets.get_sprite_uv_transform("test", 15);
        assert_eq!(offset, Vec2::new(0.75, 0.75));
        assert_eq!(scale, Vec2::new(0.25, 0.25));
    }

    #[test]
    fn test_uv_transform_unknown_sheet() {
        let assets = SpriteAssets::new();

        // Should return default (full texture)
        let (offset, scale) = assets.get_sprite_uv_transform("unknown", 0);
        assert_eq!(offset, Vec2::ZERO);
        assert_eq!(scale, Vec2::ONE);
    }
}

#[cfg(test)]
mod asset_loading_tests {
    use super::*;

    /// Test that placeholder sprite sheets can be loaded and registered
    ///
    /// This test verifies Phase 4 deliverable: placeholder sprites are
    /// properly configured and ready for asset loading.
    #[test]
    fn test_load_placeholder_sprites() {
        let mut sprite_assets = SpriteAssets::new();

        // Register all placeholder sheets (matching data/sprite_sheets.ron)
        sprite_assets.register_config(
            "walls".to_string(),
            SpriteSheetConfig {
                texture_path: "sprites/tiles/walls.png".to_string(),
                tile_size: (128.0, 256.0),
                columns: 4,
                rows: 4,
                sprites: vec![(0, "stone_wall".to_string()), (1, "brick_wall".to_string())],
            },
        );

        sprite_assets.register_config(
            "npcs_town".to_string(),
            SpriteSheetConfig {
                texture_path: "sprites/actors/npcs_town.png".to_string(),
                tile_size: (32.0, 48.0),
                columns: 4,
                rows: 4,
                sprites: vec![(0, "merchant".to_string())],
            },
        );

        sprite_assets.register_config(
            "signs".to_string(),
            SpriteSheetConfig {
                texture_path: "sprites/events/signs.png".to_string(),
                tile_size: (32.0, 64.0),
                columns: 4,
                rows: 2,
                sprites: vec![(0, "wooden_sign".to_string())],
            },
        );

        // Verify configs are stored
        assert!(sprite_assets.get_config("walls").is_some());
        assert!(sprite_assets.get_config("npcs_town").is_some());
        assert!(sprite_assets.get_config("signs").is_some());

        // Verify correct paths are registered
        let walls_config = sprite_assets.get_config("walls").unwrap();
        assert_eq!(walls_config.texture_path, "sprites/tiles/walls.png");

        let npcs_config = sprite_assets.get_config("npcs_town").unwrap();
        assert_eq!(npcs_config.texture_path, "sprites/actors/npcs_town.png");

        let signs_config = sprite_assets.get_config("signs").unwrap();
        assert_eq!(signs_config.texture_path, "sprites/events/signs.png");
    }

    /// Test that placeholder PNG files exist on disk
    ///
    /// This test verifies Phase 4 deliverable: all placeholder sprite sheets
    /// have been created and are available in the assets directory.
    #[test]
    fn test_placeholder_png_files_exist() {
        // List of placeholder PNG files that should exist
        let paths = vec![
            // Tile sprites
            "assets/sprites/tiles/walls.png",
            "assets/sprites/tiles/doors.png",
            "assets/sprites/tiles/terrain.png",
            "assets/sprites/tiles/trees.png",
            "assets/sprites/tiles/decorations.png",
            // Actor sprites
            "assets/sprites/actors/npcs_town.png",
            "assets/sprites/actors/monsters_basic.png",
            "assets/sprites/actors/monsters_advanced.png",
            "assets/sprites/actors/recruitables.png",
            // Event marker sprites
            "assets/sprites/events/signs.png",
            "assets/sprites/events/portals.png",
        ];

        for path in paths {
            let full_path = std::path::Path::new(path);
            assert!(
                full_path.exists(),
                "Missing placeholder PNG file: {} (required for Phase 4)",
                path
            );

            // Verify file has content (not empty)
            let metadata = std::fs::metadata(path)
                .unwrap_or_else(|_| panic!("Cannot read metadata for {}", path));
            assert!(
                metadata.len() > 0,
                "Placeholder PNG file is empty: {} (file size: 0 bytes)",
                path
            );
        }
    }

    /// Test that directory structure is properly organized
    ///
    /// This test verifies Phase 4 deliverable: sprite assets are organized
    /// into logical subdirectories (tiles, actors, events, ui).
    #[test]
    fn test_sprite_directory_structure() {
        let directories = vec![
            "assets/sprites/tiles",
            "assets/sprites/actors",
            "assets/sprites/events",
            "assets/sprites/ui",
        ];

        for dir in directories {
            let path = std::path::Path::new(dir);
            assert!(
                path.is_dir(),
                "Sprite directory structure incomplete: {} directory missing",
                dir
            );
        }
    }
}
