// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! GLB scene pre-loader startup system.
//!
//! Populates [`GlbHandleCache`] at game startup so that all GLB-backed assets
//! have warm handles before any map spawn systems run.
//!
//! The system iterates all three databases that may contain `glb_path` entries:
//! 1. `ContentDatabase::creatures` — combat and world creature visuals.
//! 2. `ContentDatabase::object_meshes` — the unified runtime database that
//!    already contains merged landscape and furniture mesh entries.
//! 3. `ContentDatabase::item_meshes` — dropped item and in-world item visuals.
//!
//! For each unique path, the Bevy scene-fragment asset key is constructed via
//! `GltfAssetLabel::Scene(index).from_asset(path)` and `asset_server.load(…)`
//! is called, placing the resulting `Handle<WorldAsset>` in the cache.

use crate::application::resources::GameContent;
use crate::game::resources::GlbHandleCache;
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use std::collections::HashSet;

/// Startup system that pre-loads every GLB-backed asset into [`GlbHandleCache`].
///
/// Iterates all three content databases (`creatures`, `object_meshes`,
/// `item_meshes`), constructs the Bevy scene-fragment asset key for each
/// unique `glb_path`, and stores the resulting `Handle<WorldAsset>` in the
/// cache.  Duplicate paths across databases are deduplicated so each GLB is
/// loaded at most once.
///
/// The system must run **before** `spawn_map_system` in the `Startup` schedule
/// so that cached handles are available when map entities are spawned.
///
/// # Registration
///
/// Registered as a `Startup` system in `src/bin/antares.rs` and also placed
/// immediately before `spawn_map_system` in the `MapRenderingPlugin` startup
/// chain to guarantee correct execution order.
///
/// # Examples
///
/// ```ignore
/// // Registered in antares.rs:
/// app.init_resource::<antares::game::resources::GlbHandleCache>();
/// app.add_systems(Startup, antares::game::systems::glb_scene_loader::glb_scene_loader_system);
/// ```
pub fn glb_scene_loader_system(
    content: Option<Res<GameContent>>,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<GlbHandleCache>,
) {
    let Some(content) = content else {
        warn!(
            "glb_scene_loader_system: GameContent not yet available; \
             skipping GLB pre-load (no assets will use the GLB render path)"
        );
        return;
    };

    // Collect owned paths first (releases borrow on content) so that the
    // owned Strings can satisfy the `'static` bound required by
    // `GltfAssetLabel::Scene(n).from_asset(path)`.
    let creature_paths: Vec<(String, usize)> = content
        .db()
        .creatures
        .all_creatures()
        .filter_map(|d| d.glb_path.as_ref().map(|p| (p.clone(), d.glb_scene_index)))
        .collect();

    let object_paths: Vec<String> = content
        .db()
        .object_meshes
        .glb_paths()
        .map(str::to_string)
        .collect();

    let item_paths: Vec<String> = content
        .db()
        .item_meshes
        .glb_paths()
        .map(str::to_string)
        .collect();

    // content borrow ends here — now use owned Strings for asset loading.
    let mut seen: HashSet<String> = HashSet::new();

    // 1. Creature database: preserve per-entry glb_scene_index.
    for (path, idx) in creature_paths {
        if seen.insert(path.clone()) {
            let handle: Handle<WorldAsset> =
                asset_server.load(GltfAssetLabel::Scene(idx).from_asset(path.clone()));
            debug!(
                "GlbHandleCache: pre-loaded creature GLB '{}' (Scene{})",
                path, idx
            );
            cache.scenes.insert(path, handle);
        }
    }

    // 2. Object mesh database: scene_index defaults to 0 for this database's
    //    API; the primary use-case is single-scene GLBs.
    for path in object_paths {
        if seen.insert(path.clone()) {
            let handle: Handle<WorldAsset> =
                asset_server.load(GltfAssetLabel::Scene(0).from_asset(path.clone()));
            debug!(
                "GlbHandleCache: pre-loaded object mesh GLB '{}' (Scene0)",
                path
            );
            cache.scenes.insert(path, handle);
        }
    }

    // 3. Item mesh database: scene_index defaults to 0.
    for path in item_paths {
        if seen.insert(path.clone()) {
            let handle: Handle<WorldAsset> =
                asset_server.load(GltfAssetLabel::Scene(0).from_asset(path.clone()));
            debug!(
                "GlbHandleCache: pre-loaded item mesh GLB '{}' (Scene0)",
                path
            );
            cache.scenes.insert(path, handle);
        }
    }

    info!(
        "GlbHandleCache populated: {} unique GLB asset(s) pre-loaded",
        cache.scenes.len()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::visual::CreatureDefinition;
    use crate::sdk::database::ContentDatabase;

    /// Builds a minimal `ContentDatabase` with one GLB entry in `creatures`.
    fn make_db_with_glb_entries() -> ContentDatabase {
        let mut db = ContentDatabase::new();

        // Creature GLB entry
        let creature_def = CreatureDefinition {
            id: 9001,
            name: "GlbCreature".to_string(),
            meshes: vec![],
            mesh_transforms: vec![],
            scale: 1.0,
            color_tint: None,
            glb_path: Some("assets/meshes/creatures/test_creature.glb".to_string()),
            glb_scene_index: 0,
        };
        db.creatures
            .add_creature(creature_def)
            .expect("add GLB creature");

        db
    }

    /// After running `glb_scene_loader_system`, the cache must contain a handle
    /// for the GLB creature entry.
    ///
    /// Uses a headless Bevy App with `MinimalPlugins` + `AssetPlugin`.
    #[test]
    fn test_glb_scene_loader_populates_cache_from_all_databases() {
        use crate::application::resources::GameContent;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        // WorldAsset must be registered before asset_server.load::<WorldAsset>() is called.
        app.init_asset::<WorldAsset>();

        // Build a ContentDatabase with a GLB entry in creatures.
        let db = make_db_with_glb_entries();
        app.insert_resource(GameContent::new(db));
        app.init_resource::<GlbHandleCache>();

        // Run the system manually.
        app.add_systems(Startup, glb_scene_loader_system);
        app.update();

        let cache = app.world().resource::<GlbHandleCache>();
        // The creature GLB path must be in the cache.
        assert!(
            cache
                .scenes
                .contains_key("assets/meshes/creatures/test_creature.glb"),
            "creature GLB path missing from cache"
        );
    }

    /// When `GameContent` is not available, the system warns and returns early
    /// without panicking — cache remains empty.
    #[test]
    fn test_glb_scene_loader_no_content_does_not_panic() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        // Do NOT insert GameContent
        app.init_resource::<GlbHandleCache>();
        app.add_systems(Startup, glb_scene_loader_system);
        app.update();

        let cache = app.world().resource::<GlbHandleCache>();
        assert!(
            cache.scenes.is_empty(),
            "cache must be empty when no content"
        );
    }
}
