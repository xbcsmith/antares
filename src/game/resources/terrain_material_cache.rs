// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! `TerrainMaterialCache` — Bevy resource that caches one `StandardMaterial`
//! handle per terrain definition, keyed by [`TerrainId`].
//!
//! Caching prevents the map-spawn system from creating redundant material
//! allocations every time a map is loaded. The cache is populated by
//! [`crate::game::systems::terrain_materials::load_terrain_materials_system`]
//! during the Bevy `Startup` schedule.

use crate::domain::types::TerrainId;
use crate::domain::world::terrain::{
    TERRAIN_DIRT, TERRAIN_FOREST, TERRAIN_GRASS, TERRAIN_GROUND, TERRAIN_LAVA, TERRAIN_MOUNTAIN,
    TERRAIN_STONE, TERRAIN_SWAMP, TERRAIN_WATER,
};
use bevy::prelude::*;
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// Resource
// ─────────────────────────────────────────────────────────────────────────────

/// Caches one `StandardMaterial` handle per terrain ID.
///
/// Created and populated by [`crate::game::systems::terrain_materials::load_terrain_materials_system`]
/// during the Bevy `Startup` schedule. Once all nine built-in terrain variants
/// are loaded, [`is_fully_loaded`](Self::is_fully_loaded) returns `true`.
///
/// # Examples
///
/// ```
/// use antares::game::resources::TerrainMaterialCache;
///
/// let cache = TerrainMaterialCache::default();
/// assert!(!cache.is_fully_loaded());
/// ```
#[derive(Resource, Default, Debug)]
pub struct TerrainMaterialCache {
    /// Material handles keyed by terrain ID.
    items: HashMap<TerrainId, Handle<StandardMaterial>>,
}

impl TerrainMaterialCache {
    /// Returns the cached handle for `terrain`, or `None` if not yet loaded.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::TerrainMaterialCache;
    /// use antares::domain::world::terrain::TERRAIN_GRASS;
    ///
    /// let cache = TerrainMaterialCache::default();
    /// assert!(cache.get(TERRAIN_GRASS).is_none());
    /// ```
    pub fn get(&self, terrain: TerrainId) -> Option<&Handle<StandardMaterial>> {
        self.items.get(&terrain)
    }

    /// Inserts or replaces the cached handle for `terrain`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::TerrainMaterialCache;
    /// use antares::domain::world::terrain::TERRAIN_WATER;
    /// use bevy::prelude::*;
    ///
    /// let mut cache = TerrainMaterialCache::default();
    /// let handle = Handle::<StandardMaterial>::default();
    /// cache.set(TERRAIN_WATER, handle);
    /// assert!(cache.get(TERRAIN_WATER).is_some());
    /// ```
    pub fn set(&mut self, terrain: TerrainId, handle: Handle<StandardMaterial>) {
        self.items.insert(terrain, handle);
    }

    /// Returns `true` when the nine built-in terrain variants all have handles.
    ///
    /// The nine checked IDs are the original engine terrains (Ground through
    /// Forest, IDs 13000–13008). Sand, Snow, and Ice are not yet required.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::TerrainMaterialCache;
    /// use antares::domain::world::terrain::{
    ///     TERRAIN_DIRT, TERRAIN_FOREST, TERRAIN_GRASS, TERRAIN_GROUND, TERRAIN_LAVA,
    ///     TERRAIN_MOUNTAIN, TERRAIN_STONE, TERRAIN_SWAMP, TERRAIN_WATER,
    /// };
    /// use bevy::prelude::*;
    ///
    /// let mut cache = TerrainMaterialCache::default();
    /// assert!(!cache.is_fully_loaded());
    ///
    /// for terrain in [
    ///     TERRAIN_GROUND, TERRAIN_GRASS, TERRAIN_STONE,
    ///     TERRAIN_MOUNTAIN, TERRAIN_DIRT, TERRAIN_WATER,
    ///     TERRAIN_LAVA, TERRAIN_SWAMP, TERRAIN_FOREST,
    /// ] {
    ///     cache.set(terrain, Handle::default());
    /// }
    /// assert!(cache.is_fully_loaded());
    /// ```
    pub fn is_fully_loaded(&self) -> bool {
        [
            TERRAIN_GROUND,
            TERRAIN_GRASS,
            TERRAIN_STONE,
            TERRAIN_MOUNTAIN,
            TERRAIN_DIRT,
            TERRAIN_WATER,
            TERRAIN_LAVA,
            TERRAIN_SWAMP,
            TERRAIN_FOREST,
        ]
        .iter()
        .all(|id| self.items.contains_key(id))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// All nine built-in terrain IDs in a fixed order.
    fn all_terrain_ids() -> [TerrainId; 9] {
        [
            TERRAIN_GROUND,
            TERRAIN_GRASS,
            TERRAIN_STONE,
            TERRAIN_MOUNTAIN,
            TERRAIN_DIRT,
            TERRAIN_WATER,
            TERRAIN_LAVA,
            TERRAIN_SWAMP,
            TERRAIN_FOREST,
        ]
    }

    /// `TerrainMaterialCache::default()` must have no cached handles.
    #[test]
    fn test_terrain_material_cache_default_all_none() {
        let cache = TerrainMaterialCache::default();
        for id in all_terrain_ids() {
            assert!(cache.get(id).is_none(), "ID {id} should be None by default");
        }
    }

    /// `is_fully_loaded()` returns `false` on a default (empty) cache.
    #[test]
    fn test_terrain_material_cache_is_fully_loaded_false_when_empty() {
        let cache = TerrainMaterialCache::default();
        assert!(
            !cache.is_fully_loaded(),
            "is_fully_loaded() should be false when no handles are set"
        );
    }

    /// `set(TERRAIN_GRASS, handle)` then `get(TERRAIN_GRASS)` returns the same handle.
    #[test]
    fn test_terrain_material_cache_set_get_roundtrip() {
        let mut cache = TerrainMaterialCache::default();
        let handle = Handle::<StandardMaterial>::default();

        cache.set(TERRAIN_GRASS, handle.clone());

        let retrieved = cache
            .get(TERRAIN_GRASS)
            .expect("handle should be present after set");

        assert_eq!(
            *retrieved, handle,
            "get() should return the handle that was set"
        );
    }

    /// After nine `set()` calls (one per built-in terrain), `is_fully_loaded()` returns `true`.
    #[test]
    fn test_terrain_material_cache_is_fully_loaded_true_when_all_set() {
        let mut cache = TerrainMaterialCache::default();

        for terrain in all_terrain_ids() {
            assert!(
                !cache.is_fully_loaded(),
                "is_fully_loaded() should still be false before all variants are set"
            );
            cache.set(terrain, Handle::default());
        }

        assert!(
            cache.is_fully_loaded(),
            "is_fully_loaded() should be true after all nine variants are set"
        );
    }

    /// `get()` must return `None` for a terrain ID that has not been set.
    #[test]
    fn test_get_returns_none_for_unset_terrain() {
        let mut cache = TerrainMaterialCache::default();
        for terrain in all_terrain_ids() {
            if terrain != TERRAIN_WATER {
                cache.set(terrain, Handle::default());
            }
        }

        assert!(
            cache.get(TERRAIN_WATER).is_none(),
            "Water handle should still be None"
        );
        for terrain in all_terrain_ids() {
            if terrain != TERRAIN_WATER {
                assert!(
                    cache.get(terrain).is_some(),
                    "ID {terrain} handle should be Some"
                );
            }
        }
    }

    /// `set()` must overwrite a previously cached handle.
    #[test]
    fn test_set_overwrites_existing_handle() {
        let mut cache = TerrainMaterialCache::default();
        let first = Handle::<StandardMaterial>::default();
        let second = Handle::<StandardMaterial>::default();

        cache.set(TERRAIN_STONE, first);
        cache.set(TERRAIN_STONE, second.clone());

        let retrieved = cache
            .get(TERRAIN_STONE)
            .expect("Stone handle must be present");
        assert_eq!(
            *retrieved, second,
            "set() must overwrite the previous handle"
        );
    }

    /// Setting one terrain type must not affect any other type.
    #[test]
    fn test_set_one_does_not_affect_others() {
        let mut cache = TerrainMaterialCache::default();
        cache.set(TERRAIN_LAVA, Handle::default());

        for terrain in all_terrain_ids() {
            if terrain == TERRAIN_LAVA {
                assert!(cache.get(terrain).is_some());
            } else {
                assert!(
                    cache.get(terrain).is_none(),
                    "Setting Lava must not populate ID {terrain}"
                );
            }
        }
    }

    /// `is_fully_loaded()` must return `false` if only eight of nine types are set.
    #[test]
    fn test_is_fully_loaded_false_with_eight_of_nine() {
        let all = all_terrain_ids();
        let mut cache = TerrainMaterialCache::default();
        for terrain in &all[..8] {
            cache.set(*terrain, Handle::default());
        }
        assert!(
            !cache.is_fully_loaded(),
            "is_fully_loaded() should be false when one variant is missing"
        );
    }

    /// `get()` must handle every terrain ID without panicking.
    #[test]
    fn test_get_covers_all_variants() {
        let cache = TerrainMaterialCache::default();
        for terrain in all_terrain_ids() {
            let _ = cache.get(terrain);
        }
    }

    /// `set()` + `get()` round-trip must work for every terrain ID.
    #[test]
    fn test_set_get_roundtrip_all_variants() {
        let mut cache = TerrainMaterialCache::default();
        for terrain in all_terrain_ids() {
            let handle = Handle::<StandardMaterial>::default();
            cache.set(terrain, handle.clone());
            let got = cache
                .get(terrain)
                .unwrap_or_else(|| panic!("Expected Some for ID {terrain}"));
            assert_eq!(*got, handle);
        }
    }
}
