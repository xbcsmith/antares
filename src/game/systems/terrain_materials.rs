// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Terrain material loading startup system.
//!
//! This module creates one [`StandardMaterial`] per terrain definition by
//! loading a PNG texture from `assets/textures/terrain/` via Bevy's
//! [`AssetServer`]. All handles are stored in the [`TerrainMaterialCache`]
//! resource that is inserted into the world so that
//! [`spawn_map`](crate::game::systems::map) can look up a cached handle
//! instead of creating redundant allocations for every tile.
//!
//! Terrain definitions are read from all terrain definitions in the active
//! campaign's [`TerrainDatabase`] (or the built-in 12-entry database when no
//! [`GameContent`] resource is present), making the material-loading system
//! data-driven and automatically extensible as new terrain types are added.
//!
//! The system is registered as a `Startup` system inside
//! [`MapRenderingPlugin::build`](crate::game::systems::map::MapRenderingPlugin).

use crate::application::resources::GameContent;
use crate::domain::types::TerrainId;
use crate::domain::world::terrain::{
    builtin_terrain_db, TERRAIN_DIRT, TERRAIN_FOREST, TERRAIN_GRASS, TERRAIN_GROUND, TERRAIN_LAVA,
    TERRAIN_MOUNTAIN, TERRAIN_STONE, TERRAIN_SWAMP, TERRAIN_WATER,
};
use crate::game::resources::TerrainMaterialCache;
use bevy::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// Startup system
// ─────────────────────────────────────────────────────────────────────────────

/// Bevy `Startup` system that loads terrain textures and inserts a fully
/// populated [`TerrainMaterialCache`] resource into the world.
///
/// For each terrain definition in the active campaign's [`TerrainDatabase`]
/// (falling back to the built-in 12-entry database when no [`GameContent`]
/// resource is present) the system:
///
/// 1. **Synchronously** reads and decodes the PNG texture from disk (via
///    [`load_terrain_image_sync`]) and registers each image with the
///    [`AssetServer`] so its handle ID is reserved before later runtime image
///    allocations. This prevents the intermittent startup corruption where the
///    render world's bindless material pipeline prepares a bind group before
///    large terrain PNG files finish their async decode, permanently locking a
///    wrong texture slot into the material's GPU buffer.
/// 2. Creates a [`StandardMaterial`] with `base_color_texture` set and the
///    per-terrain `perceptual_roughness` value from the terrain definition.
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
/// * `content`      - Optional campaign content; falls back to builtin DB when absent.
pub fn load_terrain_materials_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    content: Option<Res<GameContent>>,
) {
    let mut cache = TerrainMaterialCache::default();

    let terrain_db = content
        .as_deref()
        .map(|c| c.0.terrain.clone())
        .unwrap_or_else(builtin_terrain_db);

    // Determine the asset root that the AssetPlugin was configured to use.
    // This mirrors the logic in `main()` where BEVY_ASSET_ROOT is set to the
    // campaign root before the app is built.
    let asset_root = std::env::var("BEVY_ASSET_ROOT").unwrap_or_default();

    for def in terrain_db.all_definitions() {
        let texture_handle = load_terrain_image_sync(&def.texture_path, &asset_root, &asset_server);

        let material_handle = materials.add(StandardMaterial {
            base_color_texture: Some(texture_handle),
            perceptual_roughness: def.roughness,
            ..default()
        });

        cache.set(def.id, material_handle);
    }

    commands.insert_resource(cache);
}

/// Marks every cached terrain material as changed after startup image
/// allocation has settled.
///
/// Terrain textures are synchronously decoded during [`load_terrain_materials_system`],
/// then other startup systems can still allocate runtime images before the
/// first render extraction. Touching each cached [`StandardMaterial`] in
/// `PostStartup` forces Bevy to rebuild the material bindings from the final
/// image set, preventing stale bindless texture indices from surviving into the
/// first rendered frame.
///
/// # System Parameters
///
/// * `terrain_cache` - Cached terrain material handles created at startup.
/// * `materials`     - Mutable access to the terrain [`StandardMaterial`] assets.
/// * `content`       - Optional campaign content; falls back to builtin DB when absent.
pub(crate) fn refresh_terrain_materials_after_startup_allocations_system(
    terrain_cache: Option<Res<TerrainMaterialCache>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    content: Option<Res<GameContent>>,
) {
    let Some(terrain_cache) = terrain_cache else {
        return;
    };

    let terrain_db = content
        .as_deref()
        .map(|c| c.0.terrain.clone())
        .unwrap_or_else(builtin_terrain_db);

    for def in terrain_db.all_definitions() {
        let Some(material_handle) = terrain_cache.get(def.id) else {
            continue;
        };
        let Some(mut material) = materials.get_mut(material_handle) else {
            continue;
        };

        // Reassign the existing handle and roughness values. The semantic
        // material does not change, but Assets::get_mut marks it as modified so
        // Bevy rebuilds the render-world material binding after all startup
        // image allocations have occurred.
        if let Some(texture_handle) = material.base_color_texture.clone() {
            material.base_color_texture = Some(texture_handle);
        }
        material.perceptual_roughness = def.roughness;
    }
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
                    asset_server.add(image)
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
/// terrain ID it logs the texture's asset path, load state, and pixel
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

    let terrain_ids: [TerrainId; 9] = [
        TERRAIN_GROUND,
        TERRAIN_GRASS,
        TERRAIN_STONE,
        TERRAIN_MOUNTAIN,
        TERRAIN_DIRT,
        TERRAIN_WATER,
        TERRAIN_LAVA,
        TERRAIN_SWAMP,
        TERRAIN_FOREST,
    ];

    for terrain in terrain_ids {
        let Some(material_handle) = cache.get(terrain) else {
            tracing::info!("terrain-diag: {terrain}: no cached material");
            continue;
        };
        let Some(material) = materials.get(material_handle) else {
            tracing::info!("terrain-diag: {terrain}: material asset missing");
            continue;
        };
        match &material.base_color_texture {
            None => tracing::info!("terrain-diag: {terrain}: no base_color_texture"),
            Some(texture_handle) => {
                let id = texture_handle.id();
                let path = asset_server.get_path(id);
                let state = asset_server.get_load_state(id);
                let dims = images.get(id).map(|image| image.texture_descriptor.size);
                tracing::info!(
                    "terrain-diag: {terrain}: frame={} id={id:?} path={path:?} state={state:?} dims={dims:?}",
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
    use crate::domain::world::terrain::{TERRAIN_ICE, TERRAIN_SAND, TERRAIN_SNOW};

    /// All built-in terrain definitions must have non-empty texture paths pointing
    /// at `assets/textures/terrain/`.
    #[test]
    fn test_builtin_terrain_definitions_have_valid_texture_paths() {
        let db = builtin_terrain_db();
        for def in db.all_definitions() {
            assert!(
                !def.texture_path.is_empty(),
                "terrain {} has an empty texture path",
                def.name
            );
            assert!(
                def.texture_path.starts_with("assets/textures/terrain/"),
                "terrain {} texture path '{}' must start with 'assets/textures/terrain/'",
                def.name,
                def.texture_path
            );
        }
    }

    /// All built-in terrain roughness values must be in the Bevy PBR-valid range [0, 1].
    #[test]
    fn test_builtin_terrain_definitions_have_valid_roughness_values() {
        let db = builtin_terrain_db();
        for def in db.all_definitions() {
            assert!(
                (0.0..=1.0).contains(&def.roughness),
                "terrain {} roughness {} is outside [0.0, 1.0]",
                def.name,
                def.roughness
            );
        }
    }

    /// Sand, Snow, and Ice built-in definitions must be present and have
    /// distinct texture paths (they were added in Phase 3).
    #[test]
    fn test_builtin_sand_snow_ice_have_distinct_texture_paths() {
        let db = builtin_terrain_db();
        let sand = db
            .get_by_id(TERRAIN_SAND)
            .expect("Sand must be in builtin DB");
        let snow = db
            .get_by_id(TERRAIN_SNOW)
            .expect("Snow must be in builtin DB");
        let ice = db
            .get_by_id(TERRAIN_ICE)
            .expect("Ice must be in builtin DB");
        assert_ne!(
            sand.texture_path, snow.texture_path,
            "Sand and Snow must have different textures"
        );
        assert_ne!(
            snow.texture_path, ice.texture_path,
            "Snow and Ice must have different textures"
        );
        assert_ne!(
            sand.texture_path, ice.texture_path,
            "Sand and Ice must have different textures"
        );
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
            cache.is_fully_loaded(&builtin_terrain_db()),
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

        for def in builtin_terrain_db().all_definitions() {
            assert!(
                cache.get(def.id).is_some(),
                "get({}) must return Some after startup system runs",
                def.id
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

        for def in builtin_terrain_db().all_definitions() {
            let handle = cache
                .get(def.id)
                .unwrap_or_else(|| panic!("Expected cached material handle for {}", def.id));
            let material = materials
                .get(handle)
                .unwrap_or_else(|| panic!("Expected material asset for {}", def.id));

            assert!(
                material.base_color_texture.is_some(),
                "Cached material for {} must preserve base_color_texture",
                def.id
            );
        }
    }

    /// The PostStartup refresh must mark cached terrain materials as updated
    /// without replacing their texture handle.
    #[test]
    fn test_refresh_terrain_materials_after_startup_allocations_preserves_texture_handle() {
        let mut app = App::new();

        let mut materials = Assets::<StandardMaterial>::default();
        let texture_handle = Handle::<Image>::default();
        let material_handle = materials.add(StandardMaterial {
            base_color_texture: Some(texture_handle.clone()),
            perceptual_roughness: 1.0,
            ..default()
        });
        let mut cache = TerrainMaterialCache::default();
        cache.set(TERRAIN_WATER, material_handle.clone());

        app.add_plugins(bevy::app::PluginGroup::set(
            bevy::MinimalPlugins,
            bevy::app::ScheduleRunnerPlugin::default(),
        ))
        .insert_resource(materials)
        .insert_resource(cache)
        .add_systems(
            PostStartup,
            refresh_terrain_materials_after_startup_allocations_system,
        );

        app.update();

        let materials = app
            .world()
            .get_resource::<Assets<StandardMaterial>>()
            .expect("Assets<StandardMaterial> should be present");
        let material = materials
            .get(&material_handle)
            .expect("water material should remain present");

        assert_eq!(material.base_color_texture, Some(texture_handle));
        assert!(
            (material.perceptual_roughness
                - builtin_terrain_db()
                    .get_by_id(TERRAIN_WATER)
                    .unwrap()
                    .roughness)
                .abs()
                < f32::EPSILON
        );
    }

    /// The PostStartup refresh must tolerate tests and minimal apps that do not
    /// insert the terrain cache resource.
    #[test]
    fn test_refresh_terrain_materials_after_startup_allocations_no_cache_is_noop() {
        let mut app = App::new();

        app.add_plugins(bevy::app::PluginGroup::set(
            bevy::MinimalPlugins,
            bevy::app::ScheduleRunnerPlugin::default(),
        ))
        .insert_resource(Assets::<StandardMaterial>::default())
        .add_systems(
            PostStartup,
            refresh_terrain_materials_after_startup_allocations_system,
        );

        app.update();

        assert!(
            app.world()
                .get_resource::<Assets<StandardMaterial>>()
                .is_some(),
            "refresh without TerrainMaterialCache should leave material storage intact"
        );
    }

    /// Sand, Snow, and Ice must be present in the cache after the startup system runs.
    #[test]
    fn test_load_terrain_materials_system_loads_sand_snow_ice() {
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
            .expect("TerrainMaterialCache must be inserted");

        // Sand, Snow, and Ice must now be in the cache (Phase 3 added them to builtin DB)
        for id in [TERRAIN_SAND, TERRAIN_SNOW, TERRAIN_ICE] {
            assert!(
                cache.get(id).is_some(),
                "Cache must have handle for built-in terrain ID {id}"
            );
        }
        assert!(cache.is_fully_loaded(&builtin_terrain_db()));
    }
}
