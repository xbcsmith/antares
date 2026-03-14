// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Dropped item data model for world persistence.
//!
//! This module defines [`DroppedItem`], the domain record for an item that has
//! been placed on the ground in the game world.  Instances are stored in the
//! [`Map::dropped_items`](crate::domain::world::Map) vector and serialised with
//! the rest of the map via RON, so they survive a full save/load round-trip
//! without any additional wiring.
//!
//! Unlike [`MapEvent::DroppedItem`](crate::domain::world::MapEvent), which is a
//! static campaign-authored trigger keyed in the event `HashMap` (one per tile),
//! `DroppedItem` entries live in a plain `Vec` and can stack: any number of them
//! may share the same tile position.

use crate::domain::types::{ItemId, MapId, Position};
use serde::{Deserialize, Serialize};

/// An item lying on the ground at a specific tile position.
///
/// `DroppedItem` is the authoritative domain record for an item that has been
/// removed from a character's inventory and placed in the game world.  Multiple
/// `DroppedItem` entries can share the same tile (items stack on a single
/// position) and they are stored in the owning [`Map`](crate::domain::world::Map)'s
/// `dropped_items` vector so they are serialised and deserialised as part of the
/// normal save-game flow.
///
/// # Fields
///
/// * `item_id`  – Logical item identifier matching `Item::id` in the item database.
/// * `charges`  – Remaining charges; `0` means non-magical or fully consumed.
/// * `position` – Tile coordinate inside the owning map.
/// * `map_id`   – The [`MapId`] of the map that owns this entry (must match `Map::id`).
///
/// # Examples
///
/// ```
/// use antares::domain::world::DroppedItem;
/// use antares::domain::types::Position;
///
/// let item = DroppedItem {
///     item_id: 5,
///     charges: 3,
///     position: Position::new(10, 15),
///     map_id: 2,
/// };
///
/// assert_eq!(item.item_id, 5);
/// assert_eq!(item.charges, 3);
/// assert_eq!(item.position.x, 10);
/// assert_eq!(item.position.y, 15);
/// assert_eq!(item.map_id, 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroppedItem {
    /// Logical item identifier matching `Item::id` in the item database.
    pub item_id: ItemId,

    /// Remaining charges on the item (`0` = non-magical or fully consumed).
    pub charges: u8,

    /// Tile coordinate where the item is lying on the ground.
    pub position: Position,

    /// Identifier of the map that owns this entry (must equal `Map::id`).
    pub map_id: MapId,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Constructs a representative [`DroppedItem`] for use across multiple tests.
    fn sample_item() -> DroppedItem {
        DroppedItem {
            item_id: 7,
            charges: 2,
            position: Position::new(3, 4),
            map_id: 1,
        }
    }

    // ===== Field Access =====

    #[test]
    fn test_dropped_item_fields_are_accessible() {
        let item = sample_item();
        assert_eq!(item.item_id, 7);
        assert_eq!(item.charges, 2);
        assert_eq!(item.position, Position::new(3, 4));
        assert_eq!(item.map_id, 1);
    }

    // ===== Clone / PartialEq =====

    #[test]
    fn test_dropped_item_clone_is_equal() {
        let item = sample_item();
        let cloned = item.clone();
        assert_eq!(item, cloned);
    }

    #[test]
    fn test_dropped_item_different_values_not_equal() {
        let a = sample_item();
        let b = DroppedItem {
            item_id: 99,
            ..a.clone()
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_dropped_item_different_charges_not_equal() {
        let a = sample_item();
        let b = DroppedItem {
            charges: 10,
            ..a.clone()
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_dropped_item_different_position_not_equal() {
        let a = sample_item();
        let b = DroppedItem {
            position: Position::new(99, 99),
            ..a.clone()
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_dropped_item_different_map_id_not_equal() {
        let a = sample_item();
        let b = DroppedItem {
            map_id: 42,
            ..a.clone()
        };
        assert_ne!(a, b);
    }

    // ===== Debug =====

    #[test]
    fn test_dropped_item_debug_contains_item_id() {
        let item = sample_item();
        let debug_str = format!("{item:?}");
        assert!(debug_str.contains("item_id"));
    }

    // ===== Serde (RON) round-trip =====

    #[test]
    fn test_dropped_item_ron_roundtrip() {
        let item = sample_item();
        let serialized = ron::to_string(&item).expect("serialization failed");
        let deserialized: DroppedItem = ron::from_str(&serialized).expect("deserialization failed");
        assert_eq!(item, deserialized);
    }

    #[test]
    fn test_dropped_item_ron_roundtrip_zero_charges() {
        let item = DroppedItem {
            item_id: 1,
            charges: 0,
            position: Position::new(0, 0),
            map_id: 0,
        };
        let serialized = ron::to_string(&item).expect("serialize zero-charges item");
        let back: DroppedItem = ron::from_str(&serialized).expect("deserialize zero-charges item");
        assert_eq!(item, back);
    }

    #[test]
    fn test_dropped_item_ron_roundtrip_max_values() {
        let item = DroppedItem {
            item_id: u8::MAX,
            charges: u8::MAX,
            position: Position::new(i32::MAX, i32::MAX),
            map_id: u16::MAX,
        };
        let serialized = ron::to_string(&item).expect("serialize max-value item");
        let back: DroppedItem = ron::from_str(&serialized).expect("deserialize max-value item");
        assert_eq!(item, back);
    }
}
