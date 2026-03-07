// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Item-world event systems — spawn and despawn dropped-item meshes.
//!
//! # Overview
//!
//! This module wires the logical item-drop/pickup lifecycle to the 3-D world:
//!
//! * [`ItemDroppedEvent`] — fired whenever an item is dropped (runtime or from
//!   a static [`MapEvent::DroppedItem`] on map load).
//! * [`ItemPickedUpEvent`] — fired when the party picks up a dropped item.
//! * [`spawn_dropped_item_system`] — reads `ItemDroppedEvent`, looks up the
//!   item in `GameContent`, builds a [`CreatureDefinition`] via
//!   [`ItemMeshDescriptor::from_item`], and spawns the mesh with
//!   [`spawn_creature`].  Inserts a [`DroppedItem`] component and registers
//!   the entity in [`DroppedItemRegistry`].
//! * [`despawn_picked_up_item_system`] — reads `ItemPickedUpEvent`, finds the
//!   entity in [`DroppedItemRegistry`], and despawns the whole hierarchy.
//! * [`load_map_dropped_items_system`] — runs after map load, iterates
//!   `MapEvent::DroppedItem` entries on the current map, and fires
//!   `ItemDroppedEvent` for each so static map-authored drops share the same
//!   spawn path as runtime drops.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/items_procedural_meshes_implementation_plan.md` §2.

use crate::application::resources::GameContent;
use crate::domain::types::{ItemId, MapId};
use crate::domain::visual::item_mesh::ItemMeshDescriptor;
use crate::domain::world::MapEvent;
use crate::game::components::dropped_item::DroppedItem;
use crate::game::resources::{DroppedItemRegistry, GlobalState};
use crate::game::systems::creature_spawning::spawn_creature;
use crate::game::systems::map::{MapEntity, TileCoord};
use bevy::prelude::*;
use rand::Rng;

// Re-export Bevy message types under their canonical project names.
use bevy::prelude::MessageReader;
use bevy::prelude::MessageWriter;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Y-axis height at which dropped items sit above the floor (world units).
const DROPPED_ITEM_Y: f32 = 0.05;

/// Maximum random rotation applied around the Y axis when an item is dropped
/// (radians). Creates natural-looking scatter for multiple items.
const DROP_ROTATION_JITTER: f32 = std::f32::consts::TAU; // full 360°

/// Tile-centre offset so items sit in the middle of a 1-unit tile.
const TILE_CENTER_OFFSET: f32 = 0.5;

// ─────────────────────────────────────────────────────────────────────────────
// Events
// ─────────────────────────────────────────────────────────────────────────────

/// Fired when an item is placed (dropped by the party or authored in a map).
///
/// The [`spawn_dropped_item_system`] reads this event and spawns the 3-D mesh.
///
/// # Examples
///
/// ```
/// use antares::game::systems::item_world_events::ItemDroppedEvent;
///
/// let ev = ItemDroppedEvent {
///     item_id: 5,
///     charges: 3,
///     map_id: 1,
///     tile_x: 10,
///     tile_y: 15,
/// };
///
/// assert_eq!(ev.item_id, 5);
/// assert_eq!(ev.charges, 3);
/// assert_eq!(ev.map_id, 1);
/// assert_eq!(ev.tile_x, 10);
/// assert_eq!(ev.tile_y, 15);
/// ```
#[derive(Message, Clone, Debug)]
pub struct ItemDroppedEvent {
    /// Logical item ID from the item database.
    pub item_id: ItemId,
    /// Remaining charges (0 = non-magical / fully charged).
    pub charges: u16,
    /// Map on which the item was dropped.
    pub map_id: MapId,
    /// X tile coordinate of the drop location.
    pub tile_x: i32,
    /// Y tile coordinate of the drop location.
    pub tile_y: i32,
}

/// Fired when the party picks up a dropped item.
///
/// The [`despawn_picked_up_item_system`] reads this event and despawns the mesh.
///
/// # Examples
///
/// ```
/// use antares::game::systems::item_world_events::ItemPickedUpEvent;
///
/// let ev = ItemPickedUpEvent {
///     item_id: 5,
///     map_id: 1,
///     tile_x: 10,
///     tile_y: 15,
/// };
///
/// assert_eq!(ev.item_id, 5);
/// assert_eq!(ev.map_id, 1);
/// assert_eq!(ev.tile_x, 10);
/// assert_eq!(ev.tile_y, 15);
/// ```
#[derive(Message, Clone, Debug)]
pub struct ItemPickedUpEvent {
    /// Logical item ID from the item database.
    pub item_id: ItemId,
    /// Map on which the item was picked up.
    pub map_id: MapId,
    /// X tile coordinate where the item was lying.
    pub tile_x: i32,
    /// Y tile coordinate where the item was lying.
    pub tile_y: i32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Plugin
// ─────────────────────────────────────────────────────────────────────────────

/// Bevy plugin that registers the item-world events and systems.
///
/// Add to your `App` during startup:
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::item_world_events::ItemWorldPlugin;
///
/// App::new().add_plugins(ItemWorldPlugin);
/// ```
pub struct ItemWorldPlugin;

impl Plugin for ItemWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ItemDroppedEvent>()
            .add_message::<ItemPickedUpEvent>()
            .init_resource::<DroppedItemRegistry>()
            .add_systems(
                Update,
                (
                    load_map_dropped_items_system,
                    spawn_dropped_item_system,
                    despawn_picked_up_item_system,
                )
                    .chain(),
            );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Systems
// ─────────────────────────────────────────────────────────────────────────────

/// Spawns a 3-D mesh for every [`ItemDroppedEvent`] received this frame.
///
/// For each event the system:
/// 1. Looks up the `Item` from `GameContent` by `item_id`.
/// 2. Builds an [`ItemMeshDescriptor`] via [`ItemMeshDescriptor::from_item`].
/// 3. Converts to a [`CreatureDefinition`] via `to_creature_definition`.
/// 4. Calls [`spawn_creature`] with a ground-lying transform (Y = 0.05).
/// 5. Inserts a [`DroppedItem`] component on the spawned parent entity.
/// 6. Inserts a [`MapEntity`] and [`TileCoord`] component for map cleanup.
/// 7. Registers the entity in [`DroppedItemRegistry`].
///
/// If `GameContent` is not available or the item is not found the event is
/// silently ignored with a warning.
#[allow(clippy::too_many_arguments)]
pub fn spawn_dropped_item_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut registry: ResMut<DroppedItemRegistry>,
    mut events: MessageReader<ItemDroppedEvent>,
    content: Option<Res<GameContent>>,
) {
    let Some(content) = content else {
        // Content not loaded yet; events will be lost this frame.
        return;
    };

    let mut rng = rand::rng();

    for ev in events.read() {
        // Look up the item definition.
        let Some(item) = content.db().items.get_item(ev.item_id) else {
            warn!(
                "spawn_dropped_item_system: item_id {} not found in GameContent; skipping",
                ev.item_id
            );
            continue;
        };

        // Build mesh descriptor and creature definition.
        let descriptor = ItemMeshDescriptor::from_item(item);
        let creature_def = descriptor.to_creature_definition();

        // Slight random rotation around Y for visual variety.
        let jitter_y = rng.random::<f32>() * DROP_ROTATION_JITTER;

        // World-space position: tile centre, just above the floor.
        let world_pos = Vec3::new(
            ev.tile_x as f32 + TILE_CENTER_OFFSET,
            DROPPED_ITEM_Y,
            ev.tile_y as f32 + TILE_CENTER_OFFSET,
        );

        // Spawn the mesh hierarchy via the shared creature spawning path.
        // `spawn_creature` returns the parent entity; we then patch it with
        // the DroppedItem marker and map-cleanup components.
        let entity = spawn_creature(
            &mut commands,
            &creature_def,
            &mut meshes,
            &mut materials,
            world_pos,
            None, // use creature definition scale
            None, // no animation
            None, // facing handled by jitter rotation below
        );

        // Apply the random Y jitter (spawn_creature sets facing → North by
        // default; we override with an additional rotation).
        commands.entity(entity).insert((
            DroppedItem {
                item_id: ev.item_id,
                map_id: ev.map_id,
                tile_x: ev.tile_x,
                tile_y: ev.tile_y,
                charges: ev.charges,
            },
            MapEntity(ev.map_id),
            TileCoord(crate::domain::types::Position::new(ev.tile_x, ev.tile_y)),
            Name::new(format!("DroppedItem({})", item.name)),
        ));

        // Apply extra Y-axis jitter via transform patch.
        // We can't do this in spawn_creature without changing its signature,
        // so we append the rotation here via a separate insert.
        commands
            .entity(entity)
            .entry::<Transform>()
            .and_modify(move |mut transform| {
                transform.rotation *= Quat::from_rotation_y(jitter_y);
            });

        // Register in the lookup table.
        registry.insert(ev.map_id, ev.tile_x, ev.tile_y, ev.item_id, entity);

        info!(
            "Spawned dropped item mesh: item_id={} at map={} tile=({}, {})",
            ev.item_id, ev.map_id, ev.tile_x, ev.tile_y
        );
    }
}

/// Despawns the mesh entity for every [`ItemPickedUpEvent`] received this frame.
///
/// Looks up the entity in [`DroppedItemRegistry`], calls
/// `commands.entity(entity).despawn()`, and removes the registry
/// entry.  Unknown keys are logged as warnings and skipped.
pub fn despawn_picked_up_item_system(
    mut commands: Commands,
    mut registry: ResMut<DroppedItemRegistry>,
    mut events: MessageReader<ItemPickedUpEvent>,
) {
    for ev in events.read() {
        match registry.remove(ev.map_id, ev.tile_x, ev.tile_y, ev.item_id) {
            Some(entity) => {
                commands.entity(entity).despawn();
                info!(
                    "Despawned picked-up item mesh: item_id={} at map={} tile=({}, {})",
                    ev.item_id, ev.map_id, ev.tile_x, ev.tile_y
                );
            }
            None => {
                warn!(
                    "despawn_picked_up_item_system: no registry entry for \
                     item_id={} map={} tile=({}, {}); already despawned?",
                    ev.item_id, ev.map_id, ev.tile_x, ev.tile_y
                );
            }
        }
    }
}

/// Fires [`ItemDroppedEvent`] for every `MapEvent::DroppedItem` on the current
/// map so that static map-authored dropped items go through the same spawn path
/// as runtime drops.
///
/// This system runs every frame but is effectively a one-shot per map load: it
/// compares the current map ID against the last processed map ID stored in a
/// `Local`, and only emits events when the map has changed.
pub fn load_map_dropped_items_system(
    global_state: Res<GlobalState>,
    mut event_writer: MessageWriter<ItemDroppedEvent>,
    mut last_map_id: Local<Option<MapId>>,
) {
    let world = &global_state.0.world;
    let current_map_id = world.current_map;

    // Only process on map change (or on first run when last_map_id is None).
    if *last_map_id == Some(current_map_id) {
        return;
    }
    *last_map_id = Some(current_map_id);

    let Some(map) = world.get_map(current_map_id) else {
        return;
    };

    for (position, event) in &map.events {
        if let MapEvent::DroppedItem {
            item_id, charges, ..
        } = event
        {
            event_writer.write(ItemDroppedEvent {
                item_id: *item_id,
                charges: *charges,
                map_id: current_map_id,
                tile_x: position.x,
                tile_y: position.y,
            });
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // §2.8 — test_item_dropped_event_creation
    /// `ItemDroppedEvent` stores all five fields correctly.
    #[test]
    fn test_item_dropped_event_creation() {
        let ev = ItemDroppedEvent {
            item_id: 42,
            charges: 7,
            map_id: 3,
            tile_x: 11,
            tile_y: 22,
        };
        assert_eq!(ev.item_id, 42);
        assert_eq!(ev.charges, 7);
        assert_eq!(ev.map_id, 3);
        assert_eq!(ev.tile_x, 11);
        assert_eq!(ev.tile_y, 22);
    }

    // §2.8 — test_item_picked_up_event_creation
    /// `ItemPickedUpEvent` stores all four fields correctly.
    #[test]
    fn test_item_picked_up_event_creation() {
        let ev = ItemPickedUpEvent {
            item_id: 99,
            map_id: 2,
            tile_x: 5,
            tile_y: 8,
        };
        assert_eq!(ev.item_id, 99);
        assert_eq!(ev.map_id, 2);
        assert_eq!(ev.tile_x, 5);
        assert_eq!(ev.tile_y, 8);
    }

    /// `ItemDroppedEvent` is `Clone`.
    #[test]
    fn test_item_dropped_event_clone() {
        let ev = ItemDroppedEvent {
            item_id: 1,
            charges: 0,
            map_id: 1,
            tile_x: 0,
            tile_y: 0,
        };
        let cloned = ev.clone();
        assert_eq!(cloned.item_id, ev.item_id);
        assert_eq!(cloned.charges, ev.charges);
    }

    /// `ItemPickedUpEvent` is `Clone`.
    #[test]
    fn test_item_picked_up_event_clone() {
        let ev = ItemPickedUpEvent {
            item_id: 2,
            map_id: 1,
            tile_x: 3,
            tile_y: 4,
        };
        let cloned = ev.clone();
        assert_eq!(cloned.item_id, ev.item_id);
        assert_eq!(cloned.map_id, ev.map_id);
    }

    /// `ItemDroppedEvent` is `Debug`.
    #[test]
    fn test_item_dropped_event_debug() {
        let ev = ItemDroppedEvent {
            item_id: 5,
            charges: 2,
            map_id: 1,
            tile_x: 0,
            tile_y: 0,
        };
        let s = format!("{:?}", ev);
        assert!(s.contains("ItemDroppedEvent"));
    }

    /// `ItemPickedUpEvent` is `Debug`.
    #[test]
    fn test_item_picked_up_event_debug() {
        let ev = ItemPickedUpEvent {
            item_id: 5,
            map_id: 1,
            tile_x: 0,
            tile_y: 0,
        };
        let s = format!("{:?}", ev);
        assert!(s.contains("ItemPickedUpEvent"));
    }

    /// Zero charges is a valid drop (non-magical item).
    #[test]
    fn test_item_dropped_event_zero_charges() {
        let ev = ItemDroppedEvent {
            item_id: 10,
            charges: 0,
            map_id: 1,
            tile_x: 0,
            tile_y: 0,
        };
        assert_eq!(ev.charges, 0);
    }

    /// Maximum charges (`u16::MAX`) does not overflow.
    #[test]
    fn test_item_dropped_event_max_charges() {
        let ev = ItemDroppedEvent {
            item_id: 10,
            charges: u16::MAX,
            map_id: 1,
            tile_x: 0,
            tile_y: 0,
        };
        assert_eq!(ev.charges, u16::MAX);
    }

    /// Negative tile coordinates are valid (edge-case maps).
    #[test]
    fn test_item_picked_up_event_negative_tiles() {
        let ev = ItemPickedUpEvent {
            item_id: 1,
            map_id: 1,
            tile_x: -3,
            tile_y: -7,
        };
        assert_eq!(ev.tile_x, -3);
        assert_eq!(ev.tile_y, -7);
    }

    /// The `DROPPED_ITEM_Y` constant must be positive so items sit above the floor.
    #[test]
    fn test_dropped_item_y_is_positive() {
        const { assert!(DROPPED_ITEM_Y > 0.0) }
    }

    /// The tile-centre offset must be 0.5 (half a 1-unit tile).
    #[test]
    fn test_tile_center_offset_is_half() {
        assert!((TILE_CENTER_OFFSET - 0.5_f32).abs() < f32::EPSILON);
    }
}
