// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Game resources module
//!
//! Contains global game resources and asset management systems.

use crate::application::GameState;
use crate::domain::types::{ItemId, MapId, Position};
use bevy::prelude::*;
use std::collections::HashMap;

pub mod game_data;
pub mod grass_quality_settings;
pub mod performance;
pub mod sprite_assets;
pub mod terrain_material_cache;

// Re-export commonly used types
pub use game_data::GameDataResource;
pub use grass_quality_settings::{GrassPerformanceLevel, GrassQualitySettings};
pub use performance::{LodAutoTuning, MeshCache, PerformanceMetrics};
pub use terrain_material_cache::TerrainMaterialCache;

/// Global game state resource wrapper
#[derive(Resource)]
pub struct GlobalState(pub GameState);

/// Registry mapping a dropped item's world key to its Bevy entity.
///
/// The key is `(map_id, tile_x, tile_y, item_id)`.  A single tile may hold at
/// most one dropped instance of a given item ID.  If the same item is dropped
/// twice on the same tile the second drop replaces the first entry (the first
/// entity is expected to have been despawned already, or the caller should
/// check before inserting).
///
/// # Examples
///
/// ```
/// use antares::game::resources::DroppedItemRegistry;
///
/// let registry = DroppedItemRegistry::default();
/// assert!(registry.entries.is_empty());
/// ```
#[derive(Resource, Default, Debug)]
pub struct DroppedItemRegistry {
    /// Maps `(map_id, tile_x, tile_y, item_id)` → the Bevy `Entity` that holds
    /// the dropped item mesh hierarchy.
    pub entries: HashMap<(MapId, i32, i32, ItemId), Entity>,
}

impl DroppedItemRegistry {
    /// Inserts or replaces an entry in the registry.
    ///
    /// # Arguments
    ///
    /// * `map_id`  - ID of the map where the item was dropped.
    /// * `tile_x`  - X coordinate of the tile.
    /// * `tile_y`  - Y coordinate of the tile.
    /// * `item_id` - Logical item ID from the item database.
    /// * `entity`  - The Bevy entity that owns the item mesh.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::DroppedItemRegistry;
    /// use bevy::prelude::*;
    ///
    /// let mut registry = DroppedItemRegistry::default();
    /// let fake_entity = Entity::from_bits(42);
    /// registry.insert(1, 5, 7, 42, fake_entity);
    /// assert_eq!(registry.entries.len(), 1);
    /// ```
    pub fn insert(
        &mut self,
        map_id: MapId,
        tile_x: i32,
        tile_y: i32,
        item_id: ItemId,
        entity: Entity,
    ) {
        self.entries
            .insert((map_id, tile_x, tile_y, item_id), entity);
    }

    /// Looks up the entity for a given drop key.
    ///
    /// Returns `None` if no such entry exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::DroppedItemRegistry;
    /// use bevy::prelude::*;
    ///
    /// let mut registry = DroppedItemRegistry::default();
    /// let fake_entity = Entity::from_bits(43);
    /// registry.insert(1, 3, 4, 10, fake_entity);
    /// assert_eq!(registry.get(1, 3, 4, 10), Some(fake_entity));
    /// assert_eq!(registry.get(1, 3, 4, 99), None);
    /// ```
    pub fn get(&self, map_id: MapId, tile_x: i32, tile_y: i32, item_id: ItemId) -> Option<Entity> {
        self.entries
            .get(&(map_id, tile_x, tile_y, item_id))
            .copied()
    }

    /// Removes the entry for a given drop key and returns the entity, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::DroppedItemRegistry;
    /// use bevy::prelude::*;
    ///
    /// let mut registry = DroppedItemRegistry::default();
    /// let fake_entity = Entity::from_bits(44);
    /// registry.insert(1, 0, 0, 5, fake_entity);
    /// let removed = registry.remove(1, 0, 0, 5);
    /// assert_eq!(removed, Some(fake_entity));
    /// assert!(registry.entries.is_empty());
    /// ```
    pub fn remove(
        &mut self,
        map_id: MapId,
        tile_x: i32,
        tile_y: i32,
        item_id: ItemId,
    ) -> Option<Entity> {
        self.entries.remove(&(map_id, tile_x, tile_y, item_id))
    }
}

/// Signals that the player has interacted with a locked object and must
/// choose whether to pick the lock or bash it open.
///
/// This resource is populated by `handle_input` when the player presses `E`
/// in front of a locked door that cannot be opened with a key from the party
/// inventory. The lock UI reads this resource to display the pick-lock
/// or bash choice prompt.
///
/// Clear `lock_id` to `None` when the choice has been resolved or cancelled.
///
/// # Examples
///
/// ```
/// use antares::game::resources::LockInteractionPending;
///
/// let pending = LockInteractionPending::default();
/// assert!(pending.lock_id.is_none());
/// assert!(pending.position.is_none());
/// assert!(!pending.can_lockpick);
/// ```
#[derive(Resource, Default, Debug, Clone)]
pub struct LockInteractionPending {
    /// The `lock_id` string of the locked object awaiting an action choice.
    ///
    /// `None` when no lock interaction is pending.
    pub lock_id: Option<String>,
    /// The tile position of the locked object on the current map.
    ///
    /// `None` when no lock interaction is pending.
    pub position: Option<Position>,
    /// Whether at least one party member has the `pick_lock` special ability.
    ///
    /// This flag enables or disables the "Pick Lock" option in
    /// the choice UI.
    pub can_lockpick: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // §2.8 — test_dropped_item_registry_default_empty
    /// `DroppedItemRegistry::default()` must have no entries.
    #[test]
    fn test_dropped_item_registry_default_empty() {
        let registry = DroppedItemRegistry::default();
        assert!(registry.entries.is_empty());
    }

    // §2.8 — test_registry_insert_and_lookup
    /// After inserting an entry the registry returns the correct entity for
    /// that key and `None` for any other key.
    #[test]
    fn test_registry_insert_and_lookup() {
        let mut registry = DroppedItemRegistry::default();
        let entity = Entity::from_bits(42);
        registry.insert(1, 5, 7, 10, entity);

        assert_eq!(registry.get(1, 5, 7, 10), Some(entity));
        // Different item_id on same tile → None
        assert_eq!(registry.get(1, 5, 7, 99), None);
        // Different tile → None
        assert_eq!(registry.get(1, 0, 0, 10), None);
        // Different map → None
        assert_eq!(registry.get(2, 5, 7, 10), None);
    }

    // §2.8 — test_registry_remove_on_pickup
    /// After `remove`, the key must be absent from the registry.
    #[test]
    fn test_registry_remove_on_pickup() {
        let mut registry = DroppedItemRegistry::default();
        let entity = Entity::from_bits(7);
        registry.insert(3, 1, 2, 5, entity);

        let removed = registry.remove(3, 1, 2, 5);
        assert_eq!(removed, Some(entity));
        assert!(registry.entries.is_empty());
        // Second remove returns None
        assert_eq!(registry.remove(3, 1, 2, 5), None);
    }

    /// Inserting two entries with different keys keeps both.
    #[test]
    fn test_registry_two_entries() {
        let mut registry = DroppedItemRegistry::default();
        let e1 = Entity::from_bits(1);
        let e2 = Entity::from_bits(2);
        registry.insert(1, 0, 0, 1, e1);
        registry.insert(1, 0, 1, 1, e2);
        assert_eq!(registry.entries.len(), 2);
        assert_eq!(registry.get(1, 0, 0, 1), Some(e1));
        assert_eq!(registry.get(1, 0, 1, 1), Some(e2));
    }

    // ── LockInteractionPending ───────────────────────────────────────────────

    #[test]
    fn test_lock_interaction_pending_default() {
        let pending = LockInteractionPending::default();
        assert!(pending.lock_id.is_none());
        assert!(pending.position.is_none());
        assert!(!pending.can_lockpick);
    }

    #[test]
    fn test_lock_interaction_pending_set_fields() {
        use crate::domain::types::Position;

        let pending = LockInteractionPending {
            lock_id: Some("gate_01".to_string()),
            position: Some(Position::new(3, 5)),
            can_lockpick: true,
        };

        assert_eq!(pending.lock_id, Some("gate_01".to_string()));
        assert_eq!(pending.position, Some(Position::new(3, 5)));
        assert!(pending.can_lockpick);
    }

    #[test]
    fn test_lock_interaction_pending_clear() {
        let mut pending = LockInteractionPending {
            lock_id: Some("some_lock".to_string()),
            position: Some(crate::domain::types::Position::new(1, 1)),
            can_lockpick: true,
        };
        // Clearing simulates the UI consuming the pending request.
        pending.lock_id = None;
        pending.position = None;
        pending.can_lockpick = false;

        assert!(pending.lock_id.is_none());
        assert!(pending.position.is_none());
        assert!(!pending.can_lockpick);
    }

    /// A later insert with the same key overwrites the previous entity.
    #[test]
    fn test_registry_insert_overwrites() {
        let mut registry = DroppedItemRegistry::default();
        let old_entity = Entity::from_bits(10);
        let new_entity = Entity::from_bits(20);
        registry.insert(1, 3, 3, 7, old_entity);
        registry.insert(1, 3, 3, 7, new_entity);
        assert_eq!(registry.get(1, 3, 3, 7), Some(new_entity));
        assert_eq!(registry.entries.len(), 1);
    }
}
