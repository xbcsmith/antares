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
pub const TEXTURE_GROUND: &str = "assets/textures/terrain/ground.png";
/// Asset-server path for the grass terrain texture.
pub const TEXTURE_GRASS: &str = "assets/textures/terrain/grass.png";
/// Asset-server path for the stone terrain texture.
pub const TEXTURE_STONE: &str = "assets/textures/terrain/stone.png";
/// Asset-server path for the mountain terrain texture.
pub const TEXTURE_MOUNTAIN: &str = "assets/textures/terrain/mountain.png";
/// Asset-server path for the dirt terrain texture.
pub const TEXTURE_DIRT: &str = "assets/textures/terrain/dirt.png";
/// Asset-server path for the water terrain texture.
pub const TEXTURE_WATER: &str = "assets/textures/terrain/water.png";
/// Asset-server path for the lava terrain texture.
pub const TEXTURE_LAVA: &str = "assets/textures/terrain/lava.png";
/// Asset-server path for the swamp terrain texture.
pub const TEXTURE_SWAMP: &str = "assets/textures/terrain/swamp.png";
/// Asset-server path for the forest-floor terrain texture.
pub const TEXTURE_FOREST_FLOOR: &str = "assets/textures/terrain/forest_floor.png";

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
/// assert_eq!(texture_path_for(TerrainType::Grass), "assets/textures/terrain/grass.png");
/// assert_eq!(texture_path_for(TerrainType::Water), "assets/textures/terrain/water.png");
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
/// 1. **Synchronously** reads and decodes the PNG texture from disk (via
///    [`load_terrain_image_sync`]) so the image is immediately present in
///    `Assets<Image>` before the first render frame.  This prevents the
///    intermittent startup corruption where the render world's bindless
///    material pipeline prepares a bind group before large terrain PNG files
///    finish their async decode, permanently locking a wrong texture slot into
///    the material's GPU buffer.
/// 2. Creates a [`StandardMaterial`] with `base_color_texture` set and the
///    per-terrain `perceptual_roughness` value from the implementation plan.
/// 3. Stores the material handle in [`TerrainMaterialCache`].
///
/// The cache is then accessible to [`spawn_map`](crate::game::systems::map) so
/// that floor tiles receive a textured material rather than a flat colour.
///
/// # System Parameters
///
/// * `commands`     - Used to insert the finished cache as a world resource.
/// * `asset_server` - Bevy asset server; used as async-load fallback only.
/// * `materials`    - Mutable access to the `Assets<StandardMaterial>` storage.
/// * `images`       - Mutable access to `Assets<Image>` for synchronous insertion.
pub fn load_terrain_materials_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut cache = TerrainMaterialCache::default();

    // Determine the asset root that the AssetPlugin was configured to use.
    // This mirrors the logic in `main()` where BEVY_ASSET_ROOT is set to the
    // campaign root before the app is built.
    let asset_root = std::env::var("BEVY_ASSET_ROOT").unwrap_or_default();

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
        let texture_handle =
            load_terrain_image_sync(texture_path, &asset_root, &asset_server, &mut images);

        let material_handle = materials.add(StandardMaterial {
            base_color_texture: Some(texture_handle),
            perceptual_roughness: roughness_for(terrain),
            ..default()
        });

        cache.set(terrain, material_handle);
    }

    commands.insert_resource(cache);
}

/// Synchronously reads and decodes a terrain PNG texture into [`Assets<Image>`],
/// returning a strong handle.
///
/// This is the core of the startup-race fix: by inserting the decoded image
/// directly into [`Assets<Image>`] during `Startup`, the image is present in
/// the main world before the render world's first `PrepareAssets` pass.  The
/// render world therefore prepares the [`StandardMaterial`] bind group with the
/// correct texture slot on the very first frame, instead of a stale fallback
/// slot that would never be corrected.
///
/// Falls back to [`AssetServer::load`] (async) if the file cannot be read or
/// decoded — this preserves a working path in unusual launch contexts (e.g.
/// tests, missing campaign directory) while still emitting a warning.
fn load_terrain_image_sync(
    texture_path: &str,
    asset_root: &str,
    asset_server: &AssetServer,
    images: &mut Assets<Image>,
) -> Handle<Image> {
    let full_path = if asset_root.is_empty() {
        std::path::PathBuf::from(texture_path)
    } else {
        std::path::Path::new(asset_root).join(texture_path)
    };

    match std::fs::read(&full_path) {
        Ok(bytes) => {
            match Image::from_buffer(
                &bytes,
                bevy::image::ImageType::Extension("png"),
                bevy::image::CompressedImageFormats::NONE,
                true,
                bevy::image::ImageSampler::default(),
                bevy::asset::RenderAssetUsages::default(),
            ) {
                Ok(image) => {
                    tracing::debug!(
                        "terrain-load: synchronously decoded '{}'",
                        full_path.display()
                    );
                    images.add(image)
                }
                Err(e) => {
                    tracing::warn!(
                        "terrain-load: failed to decode '{}' ({e}); \
                         falling back to async AssetServer load",
                        full_path.display()
                    );
                    asset_server.load(texture_path.to_string())
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                "terrain-load: failed to read '{}' ({e}); \
                 falling back to async AssetServer load",
                full_path.display()
            );
            asset_server.load(texture_path.to_string())
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Diagnostics
// ─────────────────────────────────────────────────────────────────────────────

/// Diagnostic `PreStartup` system: allocates N never-used 1×1 images before
/// any Startup asset loads, to test whether early `Assets<Image>` allocations
/// alone (an asset-index shift) corrupt material texture bindings.
///
/// Controlled by `ANTARES_DIAG_DUMMY_IMAGES=<n>`; no-op when unset.
pub fn debug_allocate_dummy_images_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok(value) = std::env::var("ANTARES_DIAG_DUMMY_IMAGES") else {
        return;
    };
    let Ok(count) = value.parse::<usize>() else {
        return;
    };
    #[derive(Resource)]
    struct DummyImages(#[allow(dead_code)] Vec<Handle<Image>>);
    let mut handles = Vec::new();
    for _ in 0..count {
        let image = Image::new_fill(
            bevy::render::render_resource::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            &[255, 255, 255, 255],
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::default(),
        );
        handles.push(images.add(image));
    }
    tracing::info!("terrain-diag: allocated {count} dummy images at PreStartup");
    commands.insert_resource(DummyImages(handles));
}

/// Update system that logs which image asset each terrain material's
/// `base_color_texture` handle resolves to, a few seconds after startup.
///
/// Used to diagnose intermittent wrong-texture-on-terrain reports: for every
/// [`TerrainType`] it logs the texture's asset path, load state, and pixel
/// dimensions, and it also logs every `Image` asset that has **no** asset path
/// (runtime-generated canvases such as the mini-map). Comparing the two lists
/// identifies exactly which image a corrupted tile is bound to.
pub fn debug_terrain_texture_bindings_system(
    mut commands: Commands,
    mut frames: Local<u32>,
    terrain_cache: Option<Res<TerrainMaterialCache>>,
    materials: Res<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
    asset_server: Res<AssetServer>,
    global_state: Option<ResMut<crate::game::resources::GlobalState>>,
) {
    *frames += 1;
    // All diagnostics are opt-in: without ANTARES_DIAG_SCREENSHOT set the
    // system is a frame-counter no-op, so normal runs stay quiet.
    if std::env::var("ANTARES_DIAG_SCREENSHOT").is_err() {
        return;
    }
    // Automated screenshot for diagnosing visual-only bugs: capture frame 350
    // to the given file path. The party is turned to face West (toward the
    // tutorial-town water pool) and the automap is briefly opened so both
    // dynamic canvases hold real content when the frame is captured.
    {
        match *frames {
            // Open the automap so its canvas gets painted, close it again,
            // then face the water pool and capture the frame.
            200 => {
                if let Some(mut gs) = global_state {
                    gs.0.world
                        .set_party_facing(crate::domain::types::Direction::West);
                    gs.0.mode = crate::application::GameMode::Automap;
                }
            }
            280 => {
                if let Some(mut gs) = global_state {
                    gs.0.mode = crate::application::GameMode::Exploration;
                }
            }
            350 => {
                if let Ok(path) = std::env::var("ANTARES_DIAG_SCREENSHOT") {
                    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
                    commands
                        .spawn(Screenshot::primary_window())
                        .observe(save_to_disk(std::path::PathBuf::from(path)));
                }
            }
            _ => {}
        }
    }
    // Log twice: shortly after map spawn and again once async loads settle.
    if *frames != 120 && *frames != 600 {
        return;
    }
    let Some(cache) = terrain_cache else {
        tracing::info!("terrain-diag: TerrainMaterialCache not present");
        return;
    };

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
        let Some(material_handle) = cache.get(terrain) else {
            tracing::info!("terrain-diag: {terrain:?}: no cached material");
            continue;
        };
        let Some(material) = materials.get(material_handle) else {
            tracing::info!("terrain-diag: {terrain:?}: material asset missing");
            continue;
        };
        match &material.base_color_texture {
            None => tracing::info!("terrain-diag: {terrain:?}: no base_color_texture"),
            Some(texture_handle) => {
                let id = texture_handle.id();
                let path = asset_server.get_path(id);
                let state = asset_server.get_load_state(id);
                let dims = images.get(id).map(|image| image.texture_descriptor.size);
                tracing::info!(
                    "terrain-diag: {terrain:?}: frame={} id={id:?} path={path:?} state={state:?} dims={dims:?}",
                    *frames
                );
            }
        }
    }

    // Runtime-generated images have no asset path; list them for comparison.
    for (id, image) in images.iter() {
        if asset_server.get_path(id).is_none() {
            tracing::info!(
                "terrain-diag: pathless image id={id:?} dims={:?} frame={}",
                image.texture_descriptor.size,
                *frames
            );
        }
    }
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
                constant.starts_with("assets/textures/terrain/"),
                "Texture path constant '{constant}' must start with 'assets/textures/terrain/'"
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
