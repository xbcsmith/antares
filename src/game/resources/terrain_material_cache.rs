// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! `TerrainMaterialCache` — Bevy resource that caches one `StandardMaterial`
//! handle per terrain type.
//!
//! Caching prevents the map-spawn system from creating redundant material
//! allocations every time a map is loaded.  All nine [`TerrainType`] variants
//! are represented; a field is `None` until the startup system has loaded the
//! corresponding texture and created the material.

use crate::domain::world::TerrainType;
use bevy::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// Resource
// ─────────────────────────────────────────────────────────────────────────────

/// Caches one `StandardMaterial` handle per terrain type.
///
/// Created and populated by [`crate::game::systems::terrain_materials::load_terrain_materials_system`]
/// during the Bevy `Startup` schedule.  Once fully loaded every field is
/// `Some`; before that any field may be `None`.
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
    /// Cached material handle for [`TerrainType::Ground`].
    pub ground: Option<Handle<StandardMaterial>>,
    /// Cached material handle for [`TerrainType::Grass`].
    pub grass: Option<Handle<StandardMaterial>>,
    /// Cached material handle for [`TerrainType::Stone`].
    pub stone: Option<Handle<StandardMaterial>>,
    /// Cached material handle for [`TerrainType::Mountain`].
    pub mountain: Option<Handle<StandardMaterial>>,
    /// Cached material handle for [`TerrainType::Dirt`].
    pub dirt: Option<Handle<StandardMaterial>>,
    /// Cached material handle for [`TerrainType::Water`].
    pub water: Option<Handle<StandardMaterial>>,
    /// Cached material handle for [`TerrainType::Lava`].
    pub lava: Option<Handle<StandardMaterial>>,
    /// Cached material handle for [`TerrainType::Swamp`].
    pub swamp: Option<Handle<StandardMaterial>>,
    /// Cached material handle for [`TerrainType::Forest`] floor tiles.
    pub forest_floor: Option<Handle<StandardMaterial>>,
}

impl TerrainMaterialCache {
    /// Returns the cached handle for `terrain`, or `None` if not yet loaded.
    ///
    /// # Arguments
    ///
    /// * `terrain` - The terrain type whose material handle to look up.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::TerrainMaterialCache;
    /// use antares::domain::world::TerrainType;
    ///
    /// let cache = TerrainMaterialCache::default();
    /// assert!(cache.get(TerrainType::Grass).is_none());
    /// ```
    pub fn get(&self, terrain: TerrainType) -> Option<&Handle<StandardMaterial>> {
        match terrain {
            TerrainType::Ground => self.ground.as_ref(),
            TerrainType::Grass => self.grass.as_ref(),
            TerrainType::Stone => self.stone.as_ref(),
            TerrainType::Mountain => self.mountain.as_ref(),
            TerrainType::Dirt => self.dirt.as_ref(),
            TerrainType::Water => self.water.as_ref(),
            TerrainType::Lava => self.lava.as_ref(),
            TerrainType::Swamp => self.swamp.as_ref(),
            TerrainType::Forest => self.forest_floor.as_ref(),
        }
    }

    /// Inserts or replaces the cached handle for `terrain`.
    ///
    /// # Arguments
    ///
    /// * `terrain` - The terrain type to associate with `handle`.
    /// * `handle`  - The `StandardMaterial` handle to cache.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::TerrainMaterialCache;
    /// use antares::domain::world::TerrainType;
    /// use bevy::prelude::*;
    ///
    /// let mut cache = TerrainMaterialCache::default();
    /// // In real usage the handle comes from `Assets::<StandardMaterial>::add`.
    /// // Here we use a weak handle purely to demonstrate the API contract.
    /// let handle = Handle::<StandardMaterial>::default();
    /// cache.set(TerrainType::Water, handle);
    /// assert!(cache.get(TerrainType::Water).is_some());
    /// ```
    pub fn set(&mut self, terrain: TerrainType, handle: Handle<StandardMaterial>) {
        match terrain {
            TerrainType::Ground => self.ground = Some(handle),
            TerrainType::Grass => self.grass = Some(handle),
            TerrainType::Stone => self.stone = Some(handle),
            TerrainType::Mountain => self.mountain = Some(handle),
            TerrainType::Dirt => self.dirt = Some(handle),
            TerrainType::Water => self.water = Some(handle),
            TerrainType::Lava => self.lava = Some(handle),
            TerrainType::Swamp => self.swamp = Some(handle),
            TerrainType::Forest => self.forest_floor = Some(handle),
        }
    }

    /// Returns `true` when all nine terrain variants have a cached handle.
    ///
    /// Used by tests and diagnostics to confirm that the startup loading
    /// system has completed successfully.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::TerrainMaterialCache;
    /// use antares::domain::world::TerrainType;
    /// use bevy::prelude::*;
    ///
    /// let mut cache = TerrainMaterialCache::default();
    /// assert!(!cache.is_fully_loaded());
    ///
    /// for terrain in [
    ///     TerrainType::Ground, TerrainType::Grass, TerrainType::Stone,
    ///     TerrainType::Mountain, TerrainType::Dirt, TerrainType::Water,
    ///     TerrainType::Lava, TerrainType::Swamp, TerrainType::Forest,
    /// ] {
    ///     cache.set(terrain, Handle::default());
    /// }
    /// assert!(cache.is_fully_loaded());
    /// ```
    pub fn is_fully_loaded(&self) -> bool {
        self.ground.is_some()
            && self.grass.is_some()
            && self.stone.is_some()
            && self.mountain.is_some()
            && self.dirt.is_some()
            && self.water.is_some()
            && self.lava.is_some()
            && self.swamp.is_some()
            && self.forest_floor.is_some()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Convenience: all nine TerrainType variants in a fixed order.
    fn all_terrain_types() -> [TerrainType; 9] {
        [
            TerrainType::Ground,
            TerrainType::Grass,
            TerrainType::Stone,
            TerrainType::Mountain,
            TerrainType::Dirt,
            TerrainType::Water,
            TerrainType::Lava,
            TerrainType::Swamp,
            TerrainType::Forest,
        ]
    }

    // §1.4 — test_terrain_material_cache_default_all_none
    /// `TerrainMaterialCache::default()` must have `None` for all nine fields.
    #[test]
    fn test_terrain_material_cache_default_all_none() {
        let cache = TerrainMaterialCache::default();

        assert!(cache.ground.is_none(), "ground should be None by default");
        assert!(cache.grass.is_none(), "grass should be None by default");
        assert!(cache.stone.is_none(), "stone should be None by default");
        assert!(
            cache.mountain.is_none(),
            "mountain should be None by default"
        );
        assert!(cache.dirt.is_none(), "dirt should be None by default");
        assert!(cache.water.is_none(), "water should be None by default");
        assert!(cache.lava.is_none(), "lava should be None by default");
        assert!(cache.swamp.is_none(), "swamp should be None by default");
        assert!(
            cache.forest_floor.is_none(),
            "forest_floor should be None by default"
        );
    }

    // §1.4 — test_terrain_material_cache_is_fully_loaded_false_when_empty
    /// `is_fully_loaded()` returns `false` on a default (empty) cache.
    #[test]
    fn test_terrain_material_cache_is_fully_loaded_false_when_empty() {
        let cache = TerrainMaterialCache::default();
        assert!(
            !cache.is_fully_loaded(),
            "is_fully_loaded() should be false when no handles are set"
        );
    }

    // §1.4 — test_terrain_material_cache_set_get_roundtrip
    /// `set(TerrainType::Grass, handle)` then `get(TerrainType::Grass)` returns
    /// the same handle.
    #[test]
    fn test_terrain_material_cache_set_get_roundtrip() {
        let mut cache = TerrainMaterialCache::default();
        let handle = Handle::<StandardMaterial>::default();

        cache.set(TerrainType::Grass, handle.clone());

        let retrieved = cache
            .get(TerrainType::Grass)
            .expect("handle should be present after set");

        assert_eq!(
            *retrieved, handle,
            "get() should return the handle that was set"
        );
    }

    // §1.4 — test_terrain_material_cache_is_fully_loaded_true_when_all_set
    /// After nine `set()` calls (one per terrain), `is_fully_loaded()` returns `true`.
    #[test]
    fn test_terrain_material_cache_is_fully_loaded_true_when_all_set() {
        let mut cache = TerrainMaterialCache::default();

        for terrain in all_terrain_types() {
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

    /// `get()` must return `None` for a terrain type that has not been set.
    #[test]
    fn test_get_returns_none_for_unset_terrain() {
        let mut cache = TerrainMaterialCache::default();
        // Set all except Water
        for terrain in all_terrain_types() {
            if terrain != TerrainType::Water {
                cache.set(terrain, Handle::default());
            }
        }

        assert!(
            cache.get(TerrainType::Water).is_none(),
            "Water handle should still be None"
        );
        // All others should be Some
        for terrain in all_terrain_types() {
            if terrain != TerrainType::Water {
                assert!(
                    cache.get(terrain).is_some(),
                    "{terrain:?} handle should be Some"
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

        cache.set(TerrainType::Stone, first);
        cache.set(TerrainType::Stone, second.clone());

        let retrieved = cache
            .get(TerrainType::Stone)
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
        cache.set(TerrainType::Lava, Handle::default());

        for terrain in all_terrain_types() {
            if terrain == TerrainType::Lava {
                assert!(cache.get(terrain).is_some());
            } else {
                assert!(
                    cache.get(terrain).is_none(),
                    "Setting Lava must not populate {terrain:?}"
                );
            }
        }
    }

    /// `is_fully_loaded()` must return `false` if only eight of nine types are set.
    #[test]
    fn test_is_fully_loaded_false_with_eight_of_nine() {
        let all = all_terrain_types();
        // Omit the last variant (Forest) to verify that is_fully_loaded stays false.
        let mut cache = TerrainMaterialCache::default();
        for terrain in &all[..8] {
            cache.set(*terrain, Handle::default());
        }
        assert!(
            !cache.is_fully_loaded(),
            "is_fully_loaded() should be false when one variant is missing"
        );
    }

    /// `get()` must handle every terrain variant without panicking.
    #[test]
    fn test_get_covers_all_variants() {
        let cache = TerrainMaterialCache::default();
        for terrain in all_terrain_types() {
            // Should not panic; just returns None for each.
            let _ = cache.get(terrain);
        }
    }

    /// `set()` + `get()` round-trip must work for every terrain variant.
    #[test]
    fn test_set_get_roundtrip_all_variants() {
        let mut cache = TerrainMaterialCache::default();
        for terrain in all_terrain_types() {
            let handle = Handle::<StandardMaterial>::default();
            cache.set(terrain, handle.clone());
            let got = cache
                .get(terrain)
                .unwrap_or_else(|| panic!("Expected Some for {terrain:?}"));
            assert_eq!(*got, handle);
        }
    }
}
