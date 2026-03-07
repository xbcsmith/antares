// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! `DroppedItem` ECS component — marks an entity as an item lying on the ground.
//!
//! Each entity that represents a dropped item in the world carries exactly one
//! `DroppedItem` component.  The component stores the minimal information needed
//! to correlate the visual entity with a logical item and to locate it by tile
//! coordinate for pickup.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/items_procedural_meshes_implementation_plan.md` §2.2.

use crate::domain::types::{ItemId, MapId};
use bevy::prelude::*;

/// Marker component for an item entity lying on the ground.
///
/// Attached to the **parent** entity produced by the item-world spawn system.
/// The registry maps `(map_id, tile_x, tile_y, item_id)` → `Entity` for
/// fast despawn on pickup.
///
/// # Examples
///
/// ```
/// use antares::game::components::dropped_item::DroppedItem;
///
/// let comp = DroppedItem {
///     item_id: 42,
///     map_id: 1,
///     tile_x: 5,
///     tile_y: 7,
///     charges: 3,
/// };
///
/// assert_eq!(comp.item_id, 42);
/// assert_eq!(comp.map_id, 1);
/// assert_eq!(comp.tile_x, 5);
/// assert_eq!(comp.tile_y, 7);
/// assert_eq!(comp.charges, 3);
/// ```
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct DroppedItem {
    /// Logical item identifier (matches `Item::id` in the item database).
    pub item_id: ItemId,
    /// Map on which the item was dropped.
    pub map_id: MapId,
    /// Tile X coordinate of the drop location.
    pub tile_x: i32,
    /// Tile Y coordinate of the drop location.
    pub tile_y: i32,
    /// Remaining charges at the time of the drop (0 for non-magical items).
    pub charges: u16,
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper that builds a canonical test component.
    fn make_dropped_item() -> DroppedItem {
        DroppedItem {
            item_id: 7,
            map_id: 2,
            tile_x: 10,
            tile_y: 15,
            charges: 5,
        }
    }

    // §2.8 — test_dropped_item_component_fields
    /// Verifies that `DroppedItem` stores all five fields correctly.
    #[test]
    fn test_dropped_item_component_fields() {
        let comp = make_dropped_item();
        assert_eq!(comp.item_id, 7);
        assert_eq!(comp.map_id, 2);
        assert_eq!(comp.tile_x, 10);
        assert_eq!(comp.tile_y, 15);
        assert_eq!(comp.charges, 5);
    }

    // §2.8 — test_dropped_item_clone
    /// Verifies that `DroppedItem` implements `Clone` and produces an equal copy.
    #[test]
    fn test_dropped_item_clone() {
        let original = make_dropped_item();
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    // §2.8 — test_dropped_item_debug
    /// Verifies that `DroppedItem` implements `Debug` and that the output is
    /// non-empty (actual format is an implementation detail).
    #[test]
    fn test_dropped_item_debug() {
        let comp = make_dropped_item();
        let debug_str = format!("{:?}", comp);
        assert!(!debug_str.is_empty());
        // The debug string should mention the type name for clarity.
        assert!(debug_str.contains("DroppedItem"));
    }

    /// Verifies that two `DroppedItem` components with the same fields compare equal.
    #[test]
    fn test_dropped_item_equality() {
        let a = make_dropped_item();
        let b = make_dropped_item();
        assert_eq!(a, b);
    }

    /// Verifies that two `DroppedItem` components with different `item_id` are not equal.
    #[test]
    fn test_dropped_item_inequality_item_id() {
        let a = make_dropped_item();
        let b = DroppedItem {
            item_id: 99,
            ..a.clone()
        };
        assert_ne!(a, b);
    }

    /// Verifies that two `DroppedItem` components with different `map_id` are not equal.
    #[test]
    fn test_dropped_item_inequality_map_id() {
        let a = make_dropped_item();
        let b = DroppedItem {
            map_id: 99,
            ..a.clone()
        };
        assert_ne!(a, b);
    }

    /// Verifies that two `DroppedItem` components with different tile coords differ.
    #[test]
    fn test_dropped_item_inequality_tile_coords() {
        let a = make_dropped_item();
        let b = DroppedItem {
            tile_x: 0,
            tile_y: 0,
            ..a.clone()
        };
        assert_ne!(a, b);
    }

    /// Zero charges is valid for non-magical items.
    #[test]
    fn test_dropped_item_zero_charges() {
        let comp = DroppedItem {
            item_id: 1,
            map_id: 1,
            tile_x: 0,
            tile_y: 0,
            charges: 0,
        };
        assert_eq!(comp.charges, 0);
    }

    /// Maximum charges value (`u16::MAX`) is accepted without overflow.
    #[test]
    fn test_dropped_item_max_charges() {
        let comp = DroppedItem {
            item_id: 1,
            map_id: 1,
            tile_x: 0,
            tile_y: 0,
            charges: u16::MAX,
        };
        assert_eq!(comp.charges, u16::MAX);
    }
}
