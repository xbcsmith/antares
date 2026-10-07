// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! GLB asset handle cache resource.
//!
//! Contains the [`GlbHandleCache`] Bevy resource that caches pre-loaded
//! `Handle<WorldAsset>` for every GLB-backed creature, landscape, and item mesh
//! in the campaign.  The cache is populated at startup by
//! [`glb_scene_loader_system`](crate::game::systems::glb_scene_loader::glb_scene_loader_system)
//! so all GLB handles are warm before any map spawns.
//!
//! In Bevy 0.19, GLTF scene handles are typed as `Handle<WorldAsset>` (from
//! `bevy_world_serialization`) and spawned via the `WorldAssetRoot` component.
//! A specific scene within a GLB is addressed using the
//! `GltfAssetLabel::Scene(n)` label syntax.

use bevy::prelude::*;
use std::collections::HashMap;

/// Startup-populated cache of `Handle<WorldAsset>` for every GLB-backed asset in
/// the campaign.
///
/// Maps campaign-relative GLB path → Bevy `Handle<WorldAsset>`.
///
/// Keys are the raw `glb_path` string from `CreatureDefinition`
/// (e.g. `"assets/meshes/creatures/oak_tree.glb"`). The handle is loaded using
/// the GLTF scene-fragment label syntax, e.g.
/// `GltfAssetLabel::Scene(index).from_asset(path)`.
///
/// The cache is populated during game startup by `glb_scene_loader_system` so
/// all handles are available before any map spawning runs.
///
/// # Examples
///
/// ```
/// use antares::game::resources::GlbHandleCache;
///
/// let cache = GlbHandleCache::default();
/// assert!(cache.scenes.is_empty());
/// ```
#[derive(Resource, Default)]
pub struct GlbHandleCache {
    /// Maps campaign-relative GLB path → Bevy `Handle<WorldAsset>`.
    ///
    /// Keys are the raw `glb_path` string from `CreatureDefinition`
    /// (e.g. `"assets/meshes/creatures/oak_tree.glb"`). The Handle is
    /// loaded using the GLTF scene-fragment label:
    /// `GltfAssetLabel::Scene(scene_index).from_asset(glb_path)`.
    pub scenes: HashMap<String, Handle<WorldAsset>>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// A freshly constructed `GlbHandleCache` must contain no entries.
    #[test]
    fn test_glb_handle_cache_default_is_empty() {
        let cache = GlbHandleCache::default();
        assert!(cache.scenes.is_empty());
    }

    /// Inserting an entry into the cache then checking `contains_key` must succeed.
    #[test]
    fn test_glb_handle_cache_insert_and_lookup() {
        let mut cache = GlbHandleCache::default();
        let fake_handle: Handle<WorldAsset> = Handle::default();
        cache
            .scenes
            .insert("assets/meshes/test.glb".to_string(), fake_handle);
        assert!(cache.scenes.contains_key("assets/meshes/test.glb"));
        assert_eq!(cache.scenes.len(), 1);
    }

    /// Two different paths inserted into the cache should both be retrievable.
    #[test]
    fn test_glb_handle_cache_multiple_entries() {
        let mut cache = GlbHandleCache::default();
        cache.scenes.insert(
            "assets/meshes/creatures/goblin.glb".to_string(),
            Handle::default(),
        );
        cache.scenes.insert(
            "assets/meshes/items/sword.glb".to_string(),
            Handle::default(),
        );
        assert_eq!(cache.scenes.len(), 2);
        assert!(cache
            .scenes
            .contains_key("assets/meshes/creatures/goblin.glb"));
        assert!(cache.scenes.contains_key("assets/meshes/items/sword.glb"));
    }
}
