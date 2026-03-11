// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Terrain material loading startup system.
//!
//! This module creates one [`StandardMaterial`] per terrain type by loading a
//! PNG texture from `assets/textures/terrain/` via Bevy's [`AssetServer`].
//! All nine handles are stored in the [`TerrainMaterialCache`] resource that
//! is inserted into the world so that [`spawn_map`](crate::game::systems::map)
//! can look up a cached handle instead of creating redundant allocations for
//! every tile.
//!
//! The system is registered as a `Startup` system inside
//! [`MapRenderingPlugin::build`](crate::game::systems::map::MapRenderingPlugin).

use crate::domain::world::TerrainType;
use crate::game::resources::TerrainMaterialCache;
use bevy::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// Texture path constants
// ─────────────────────────────────────────────────────────────────────────────

/// Asset-server path for the ground terrain texture.
pub const TEXTURE_GROUND: &str = "textures/terrain/ground.png";
/// Asset-server path for the grass terrain texture.
pub const TEXTURE_GRASS: &str = "textures/terrain/grass.png";
/// Asset-server path for the stone terrain texture.
pub const TEXTURE_STONE: &str = "textures/terrain/stone.png";
/// Asset-server path for the mountain terrain texture.
pub const TEXTURE_MOUNTAIN: &str = "textures/terrain/mountain.png";
/// Asset-server path for the dirt terrain texture.
pub const TEXTURE_DIRT: &str = "textures/terrain/dirt.png";
/// Asset-server path for the water terrain texture.
pub const TEXTURE_WATER: &str = "textures/terrain/water.png";
/// Asset-server path for the lava terrain texture.
pub const TEXTURE_LAVA: &str = "textures/terrain/lava.png";
/// Asset-server path for the swamp terrain texture.
pub const TEXTURE_SWAMP: &str = "textures/terrain/swamp.png";
/// Asset-server path for the forest-floor terrain texture.
pub const TEXTURE_FOREST_FLOOR: &str = "textures/terrain/forest_floor.png";

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Returns the asset-server texture path for each [`TerrainType`].
///
/// # Examples
///
/// ```
/// use antares::domain::world::TerrainType;
/// use antares::game::systems::terrain_materials::texture_path_for;
///
/// assert_eq!(texture_path_for(TerrainType::Grass), "textures/terrain/grass.png");
/// assert_eq!(texture_path_for(TerrainType::Water), "textures/terrain/water.png");
/// ```
pub fn texture_path_for(terrain: TerrainType) -> &'static str {
    match terrain {
        TerrainType::Ground => TEXTURE_GROUND,
        TerrainType::Grass => TEXTURE_GRASS,
        TerrainType::Stone => TEXTURE_STONE,
        TerrainType::Mountain => TEXTURE_MOUNTAIN,
        TerrainType::Dirt => TEXTURE_DIRT,
        TerrainType::Water => TEXTURE_WATER,
        TerrainType::Lava => TEXTURE_LAVA,
        TerrainType::Swamp => TEXTURE_SWAMP,
        TerrainType::Forest => TEXTURE_FOREST_FLOOR,
    }
}

/// Returns the `perceptual_roughness` value for each [`TerrainType`] per the
/// implementation plan (Section 1.3).
///
/// # Examples
///
/// ```
/// use antares::domain::world::TerrainType;
/// use antares::game::systems::terrain_materials::roughness_for;
///
/// assert!((roughness_for(TerrainType::Water) - 0.10).abs() < f32::EPSILON);
/// assert!((roughness_for(TerrainType::Ground) - 0.95).abs() < f32::EPSILON);
/// ```
pub fn roughness_for(terrain: TerrainType) -> f32 {
    match terrain {
        TerrainType::Ground => 0.95,
        TerrainType::Grass => 0.90,
        TerrainType::Stone => 0.75,
        TerrainType::Mountain => 0.85,
        TerrainType::Dirt => 0.92,
        TerrainType::Water => 0.10,
        TerrainType::Lava => 0.60,
        TerrainType::Swamp => 0.88,
        TerrainType::Forest => 0.90,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Startup system
// ─────────────────────────────────────────────────────────────────────────────

/// Bevy `Startup` system that loads terrain textures and inserts a fully
/// populated [`TerrainMaterialCache`] resource into the world.
///
/// For each of the nine [`TerrainType`] variants the system:
///
/// 1. Loads the corresponding PNG texture via the [`AssetServer`].
/// 2. Creates a [`StandardMaterial`] with `base_color_texture` set and the
///    per-terrain `perceptual_roughness` value from the implementation plan.
/// 3. Stores the material handle in [`TerrainMaterialCache`].
///
/// The cache is then accessible to [`spawn_map`](crate::game::systems::map) so
/// that floor tiles receive a textured material rather than a flat colour.
///
/// # System Parameters
///
/// * `commands`  - Used to insert the finished cache as a world resource.
/// * `asset_server` - Bevy asset server used to load PNG textures.
/// * `materials` - Mutable access to the `Assets<StandardMaterial>` storage.
pub fn load_terrain_materials_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut cache = TerrainMaterialCache::default();

    // All nine terrain types in a single loop to avoid repetition.
    let terrain_types = [
        TerrainType::Ground,
        TerrainType::Grass,
        TerrainType::Stone,
        TerrainType::Mountain,
        TerrainType::Dirt,
        TerrainType::Water,
        TerrainType::Lava,
        TerrainType::Swamp,
        TerrainType::Forest,
    ];

    for terrain in terrain_types {
        let texture_path = texture_path_for(terrain);
        let texture_handle: Handle<Image> = asset_server.load(texture_path);

        let material_handle = materials.add(StandardMaterial {
            base_color_texture: Some(texture_handle),
            perceptual_roughness: roughness_for(terrain),
            ..default()
        });

        cache.set(terrain, material_handle);
    }

    commands.insert_resource(cache);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// All nine `TEXTURE_*` constants must be non-empty strings that start
    /// with `"textures/terrain/"`.
    #[test]
    fn test_texture_path_constants_non_empty() {
        let constants = [
            TEXTURE_GROUND,
            TEXTURE_GRASS,
            TEXTURE_STONE,
            TEXTURE_MOUNTAIN,
            TEXTURE_DIRT,
            TEXTURE_WATER,
            TEXTURE_LAVA,
            TEXTURE_SWAMP,
            TEXTURE_FOREST_FLOOR,
        ];

        for constant in constants {
            assert!(
                !constant.is_empty(),
                "Texture path constant must not be empty: got empty string"
            );
            assert!(
                constant.starts_with("textures/terrain/"),
                "Texture path constant '{constant}' must start with 'textures/terrain/'"
            );
        }
    }

    /// All nine `TEXTURE_*` constants must be unique.
    #[test]
    fn test_texture_path_constants_unique() {
        let mut paths = vec![
            TEXTURE_GROUND,
            TEXTURE_GRASS,
            TEXTURE_STONE,
            TEXTURE_MOUNTAIN,
            TEXTURE_DIRT,
            TEXTURE_WATER,
            TEXTURE_LAVA,
            TEXTURE_SWAMP,
            TEXTURE_FOREST_FLOOR,
        ];
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), 9, "All TEXTURE_* constants must be unique");
    }

    /// `texture_path_for` must return the correct constant for every variant.
    #[test]
    fn test_texture_path_for_all_variants() {
        assert_eq!(texture_path_for(TerrainType::Ground), TEXTURE_GROUND);
        assert_eq!(texture_path_for(TerrainType::Grass), TEXTURE_GRASS);
        assert_eq!(texture_path_for(TerrainType::Stone), TEXTURE_STONE);
        assert_eq!(texture_path_for(TerrainType::Mountain), TEXTURE_MOUNTAIN);
        assert_eq!(texture_path_for(TerrainType::Dirt), TEXTURE_DIRT);
        assert_eq!(texture_path_for(TerrainType::Water), TEXTURE_WATER);
        assert_eq!(texture_path_for(TerrainType::Lava), TEXTURE_LAVA);
        assert_eq!(texture_path_for(TerrainType::Swamp), TEXTURE_SWAMP);
        assert_eq!(texture_path_for(TerrainType::Forest), TEXTURE_FOREST_FLOOR);
    }

    /// `roughness_for` must return the values specified in the implementation
    /// plan for every terrain variant.
    #[test]
    fn test_roughness_for_all_variants() {
        let expected: &[(TerrainType, f32)] = &[
            (TerrainType::Ground, 0.95),
            (TerrainType::Grass, 0.90),
            (TerrainType::Stone, 0.75),
            (TerrainType::Mountain, 0.85),
            (TerrainType::Dirt, 0.92),
            (TerrainType::Water, 0.10),
            (TerrainType::Lava, 0.60),
            (TerrainType::Swamp, 0.88),
            (TerrainType::Forest, 0.90),
        ];

        for (terrain, expected_roughness) in expected {
            let actual = roughness_for(*terrain);
            assert!(
                (actual - expected_roughness).abs() < f32::EPSILON,
                "roughness_for({terrain:?}): expected {expected_roughness}, got {actual}"
            );
        }
    }

    /// All roughness values must be in the physically plausible `[0.0, 1.0]`
    /// range that Bevy's PBR pipeline accepts.
    #[test]
    fn test_roughness_for_values_in_valid_range() {
        let variants = [
            TerrainType::Ground,
            TerrainType::Grass,
            TerrainType::Stone,
            TerrainType::Mountain,
            TerrainType::Dirt,
            TerrainType::Water,
            TerrainType::Lava,
            TerrainType::Swamp,
            TerrainType::Forest,
        ];

        for terrain in variants {
            let roughness = roughness_for(terrain);
            assert!(
                (0.0..=1.0).contains(&roughness),
                "roughness_for({terrain:?}) = {roughness} is outside [0.0, 1.0]"
            );
        }
    }

    /// Build a minimal `App`, run the startup system, and assert that
    /// `TerrainMaterialCache` exists and `is_fully_loaded()` returns `true`.
    #[test]
    fn test_load_terrain_materials_system_inserts_cache_resource() {
        let mut app = App::new();

        app.add_plugins(bevy::app::PluginGroup::set(
            bevy::MinimalPlugins,
            bevy::app::ScheduleRunnerPlugin::default(),
        ))
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>()
        .add_systems(Startup, load_terrain_materials_system);

        app.update();

        let cache = app
            .world()
            .get_resource::<TerrainMaterialCache>()
            .expect("TerrainMaterialCache must be inserted by load_terrain_materials_system");

        assert!(
            cache.is_fully_loaded(),
            "TerrainMaterialCache must be fully loaded after the startup system runs"
        );
    }

    /// The cache inserted by the startup system must return `Some` for every
    /// terrain type via `get()`.
    #[test]
    fn test_load_terrain_materials_system_cache_get_all_variants() {
        let mut app = App::new();

        app.add_plugins(bevy::app::PluginGroup::set(
            bevy::MinimalPlugins,
            bevy::app::ScheduleRunnerPlugin::default(),
        ))
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>()
        .add_systems(Startup, load_terrain_materials_system);

        app.update();

        let cache = app
            .world()
            .get_resource::<TerrainMaterialCache>()
            .expect("TerrainMaterialCache must be present");

        let variants = [
            TerrainType::Ground,
            TerrainType::Grass,
            TerrainType::Stone,
            TerrainType::Mountain,
            TerrainType::Dirt,
            TerrainType::Water,
            TerrainType::Lava,
            TerrainType::Swamp,
            TerrainType::Forest,
        ];

        for terrain in variants {
            assert!(
                cache.get(terrain).is_some(),
                "get({terrain:?}) must return Some after startup system runs"
            );
        }
    }

    /// Cache-populated terrain materials must preserve their texture handles for
    /// every terrain variant.
    #[test]
    fn test_load_terrain_materials_system_populates_textured_materials_for_all_variants() {
        let mut app = App::new();

        app.add_plugins(bevy::app::PluginGroup::set(
            bevy::MinimalPlugins,
            bevy::app::ScheduleRunnerPlugin::default(),
        ))
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>()
        .add_systems(Startup, load_terrain_materials_system);

        app.update();

        let cache = app
            .world()
            .get_resource::<TerrainMaterialCache>()
            .expect("TerrainMaterialCache should be inserted by startup system");
        let materials = app
            .world()
            .get_resource::<Assets<StandardMaterial>>()
            .expect("Assets<StandardMaterial> resource should exist");

        let variants = [
            TerrainType::Ground,
            TerrainType::Grass,
            TerrainType::Stone,
            TerrainType::Mountain,
            TerrainType::Dirt,
            TerrainType::Water,
            TerrainType::Lava,
            TerrainType::Swamp,
            TerrainType::Forest,
        ];

        for terrain in variants {
            let handle = cache
                .get(terrain)
                .unwrap_or_else(|| panic!("Expected cached material handle for {terrain:?}"));
            let material = materials
                .get(handle)
                .unwrap_or_else(|| panic!("Expected material asset for {terrain:?}"));

            assert!(
                material.base_color_texture.is_some(),
                "Cached material for {terrain:?} must preserve base_color_texture"
            );
        }
    }
}
