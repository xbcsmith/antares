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
use crate::domain::types::{ItemId, MapId, Position};
use crate::domain::visual::item_mesh::ItemMeshDescriptor;
use crate::domain::world::{MapEvent, TerrainType};
use crate::game::components::billboard::Billboard;
use crate::game::components::dropped_item::DroppedItem;
use crate::game::resources::game_data::GameDataResource;
use crate::game::resources::{DroppedItemRegistry, GlobalState};
use crate::game::systems::creature_spawning::spawn_creature;
use crate::game::systems::map::{MapEntity, TileCoord};
use bevy::prelude::*;

// Re-export Bevy message types under their canonical project names.
use bevy::prelude::MessageReader;
use bevy::prelude::MessageWriter;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Minimum Y-axis height (world units) for the dropped-item spawn origin.
///
/// The actual spawn height is computed dynamically in
/// [`spawn_dropped_item_system`] from the item's Z-axis geometry extent.
/// After the upright −π/2 X-rotation, the minimum-Z vertex of each mesh maps
/// to the lowest world-Y point; the spawn origin is raised so that point
/// clears the terrain-appropriate floor clearance constant.  This constant
/// acts as a lower bound so items with no negative-Z geometry (rings,
/// scrolls, …) are still raised high enough to be clearly visible in a
/// first-person view on flat stone or dirt tiles.
const DROPPED_ITEM_MIN_HEIGHT: f32 = 0.3;

/// Minimum gap (world units) between the floor plane and the lowest vertex of
/// a dropped item on **flat terrain** (Ground, Stone, Dirt, Lava, Swamp, …)
/// after the upright tilt is applied.
///
/// 30 cm ensures the pommel / base sits just above a flat tile so the item
/// appears to be resting on the surface rather than floating.  For a typical
/// short-sword (scale 1.5, pommel at local Z = −0.087) this places the pommel
/// at world Y ≈ 0.30 and the blade tip at Y ≈ 0.87.
const DROPPED_ITEM_FLOOR_CLEARANCE: f32 = 0.3;

/// Minimum gap (world units) between the floor plane and the lowest vertex of
/// a dropped item on **grass or forest terrain** after the upright tilt is
/// applied.
///
/// Grass blades are spawned at Y = 0 and can reach up to
/// `GRASS_BLADE_HEIGHT_BASE * max_height_variation = 0.4 * 1.3 = 0.52` world
/// units.  A clearance of 0.6 ensures the pommel clears even the tallest
/// blades, so the full weapon is visible above the grass surface rather than
/// appearing buried.  For a short-sword (scale 1.5) this places the pommel at
/// world Y ≈ 0.60 and the blade tip at Y ≈ 1.17 — well within the first-
/// person camera's field of view (eye height 1.2).
const DROPPED_ITEM_GRASS_FLOOR_CLEARANCE: f32 = 0.6;

/// X-axis rotation (radians) baked into every non-shadow child mesh transform
/// so that items stand upright in the XY plane.
///
/// All item mesh geometry (procedural and RON data-driven) is authored on the
/// XZ plane with normals pointing straight up (+Y).  Viewed edge-on from a
/// first-person camera the flat mesh is invisible.  Applying −π/2 around X to
/// the **child** mesh transforms maps +Z → +Y (blade tip points up) and
/// turns the face normal from +Y to local −Z.  Combined with the
/// [`Billboard`] component on the parent entity the face always points toward
/// the camera regardless of approach direction.
const DROPPED_ITEM_UPRIGHT_TILT: f32 = -std::f32::consts::FRAC_PI_2;

/// Full circle in radians — used to convert the deterministic hash to an angle.
const DROP_ROTATION_FULL_CIRCLE: f32 = std::f32::consts::TAU; // 360°

/// Tile-centre offset so items sit in the middle of a 1-unit tile.
const TILE_CENTER_OFFSET: f32 = 0.5;

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic rotation helper
// ─────────────────────────────────────────────────────────────────────────────

/// Computes a deterministic Y-axis rotation (radians) for a dropped item.
///
/// The rotation is derived purely from the drop coordinates so that:
/// - The same item dropped on the same tile always has the same orientation.
/// - Items on different tiles appear at varied orientations for visual variety.
/// - No non-deterministic randomness is introduced (safe for save/load replay).
///
/// # Algorithm
///
/// ```text
/// hash = map_id XOR (tile_x * 31) XOR (tile_y * 17) XOR (item_id * 7)
/// angle_radians = (hash % 360) / 360.0 * TAU
/// ```
///
/// # Examples
///
/// ```
/// use antares::game::systems::item_world_events::deterministic_drop_rotation;
///
/// let r1 = deterministic_drop_rotation(1, 5, 10, 42);
/// let r2 = deterministic_drop_rotation(1, 5, 10, 42);
/// assert_eq!(r1, r2, "same inputs must yield same rotation");
///
/// let r3 = deterministic_drop_rotation(1, 6, 10, 42);
/// assert_ne!(r1, r3, "different tile_x must yield different rotation");
/// ```
pub fn deterministic_drop_rotation(
    map_id: MapId,
    tile_x: i32,
    tile_y: i32,
    item_id: ItemId,
) -> f32 {
    let hash: u64 = (map_id as u64)
        .wrapping_add((tile_x as u64).wrapping_mul(31))
        .wrapping_add((tile_y as u64).wrapping_mul(17))
        .wrapping_add((item_id as u64).wrapping_mul(7));
    let degrees = hash % 360;
    (degrees as f32 / 360.0) * DROP_ROTATION_FULL_CIRCLE
}

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
            // Register map-unload registry cleanup via the visuals plugin.
            .add_plugins(crate::game::systems::dropped_item_visuals::DroppedItemVisualsPlugin)
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
/// 3. Converts to a [`CreatureDefinition`] via
///    [`to_creature_definition_with_charges`](ItemMeshDescriptor::to_creature_definition_with_charges),
///    passing the per-drop charge fraction for the charge-level gem.
/// 4. Calls [`spawn_creature`] with a ground-lying transform (Y = 0.05).
/// 5. Inserts a [`DroppedItem`] component on the spawned parent entity.
/// 6. Inserts a [`MapEntity`] and [`TileCoord`] component for map cleanup.
/// 7. Registers the entity in [`DroppedItemRegistry`].
///
/// # Deterministic Y rotation
///
/// The Y-axis rotation is derived from
/// [`deterministic_drop_rotation`] using `map_id`, `tile_x`, `tile_y`, and
/// `item_id`.  The same item on the same tile always has the same orientation
/// (deterministic, safe for save/load replay), while items on different tiles
/// appear at varied orientations for visual variety.
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
    game_data: Option<Res<GameDataResource>>,
    global_state: Option<Res<GlobalState>>,
) {
    let Some(content) = content else {
        // Content not loaded yet; events will be lost this frame.
        return;
    };

    for ev in events.read() {
        // Look up the item definition.
        let Some(item) = content.db().items.get_item(ev.item_id) else {
            warn!(
                "spawn_dropped_item_system: item_id {} not found in GameContent; skipping",
                ev.item_id
            );
            continue;
        };

        // Compute per-drop charge fraction for the gem indicator.
        let charges_fraction = if item.max_charges > 0 {
            Some(ev.charges as f32 / item.max_charges as f32)
        } else {
            None
        };

        // Resolve the CreatureDefinition for this dropped item.
        //
        // Data-driven path (preferred): if the item carries a `mesh_id` that
        // resolves to an entry in the campaign's `ItemMeshDatabase`, clone that
        // pre-authored `CreatureDefinition` directly.  This lets campaign authors
        // control scale and mesh geometry via RON files without touching Rust.
        //
        // Procedural fallback: if no `mesh_id` is present, or `GameDataResource`
        // is not yet loaded, or the ID is not found in the database, fall back to
        // the procedural `ItemMeshDescriptor::from_item` path.
        let mut creature_def = if let Some(mesh_id) = item.mesh_id {
            if let Some(gd) = game_data.as_ref() {
                if let Some(def) = gd
                    .data()
                    .item_meshes
                    .as_creature_database()
                    .get_creature(mesh_id)
                {
                    def.clone()
                } else {
                    warn!(
                        "spawn_dropped_item_system: mesh_id {} not found in item_meshes for \
                         item_id {}; falling back to procedural mesh",
                        mesh_id, ev.item_id
                    );
                    ItemMeshDescriptor::from_item(item)
                        .to_creature_definition_with_charges(charges_fraction)
                }
            } else {
                ItemMeshDescriptor::from_item(item)
                    .to_creature_definition_with_charges(charges_fraction)
            }
        } else {
            ItemMeshDescriptor::from_item(item)
                .to_creature_definition_with_charges(charges_fraction)
        };

        // Stand every non-shadow child mesh upright.
        //
        // Both the procedural path (shadow_quad + blade + optional gem) and the
        // data-driven RON path author geometry on the XZ plane (all vertices at
        // Y = 0, face normals pointing up).  A first-person camera looking
        // horizontally sees such a mesh exactly edge-on — invisible.
        //
        // Fix: apply −π/2 X rotation to each child MeshTransform except the
        // shadow quad (kept flat so it remains a floor shadow).  The rotation
        // maps the mesh's +Z axis to world +Y (blade tip points up) and turns
        // the face normal from +Y to local −Z.  The Billboard component added
        // below then rotates the parent around Y so local −Z always faces the
        // camera.
        for (i, mt) in creature_def.mesh_transforms.iter_mut().enumerate() {
            let is_shadow =
                creature_def.meshes.get(i).and_then(|m| m.name.as_deref()) == Some("shadow_quad");
            if !is_shadow {
                mt.rotation[0] = DROPPED_ITEM_UPRIGHT_TILT;
            }
        }

        // Deterministic Y-axis rotation derived from tile coords.
        // Used as the initial facing direction; Billboard keeps the item facing
        // the camera every frame so this only affects the starting orientation.
        let jitter_y = deterministic_drop_rotation(ev.map_id, ev.tile_x, ev.tile_y, ev.item_id);

        // Compute the terrain-appropriate floor clearance for this tile.
        //
        // Grass and Forest tiles spawn dense grass blades that can reach up to
        // 0.4 × 1.3 ≈ 0.52 world units above the floor.  Using the standard
        // 0.3-unit clearance on those tiles would bury most short weapons
        // inside the grass.  For all other terrain types (stone, dirt, ground,
        // …) the flat surface means 0.3 units of clearance is sufficient.
        let tile_terrain = global_state
            .as_ref()
            .and_then(|gs| gs.0.world.get_map(ev.map_id))
            .and_then(|m| m.get_tile(Position::new(ev.tile_x, ev.tile_y)))
            .map(|t| t.terrain)
            .unwrap_or(TerrainType::Ground);

        let effective_floor_clearance = match tile_terrain {
            TerrainType::Grass | TerrainType::Forest => DROPPED_ITEM_GRASS_FLOOR_CLEARANCE,
            _ => DROPPED_ITEM_FLOOR_CLEARANCE,
        };

        // Compute the item's dynamic spawn Y so its lowest vertex clears the floor.
        //
        // Item geometry is authored on the XZ plane (all Y ≈ 0).  After the
        // upright tilt (−π/2 around X), a vertex at (x, 0, z) becomes (x, z, 0)
        // in child-local space and is then scaled by `creature_def.scale`.  The
        // world-space Y of the item bottom is therefore:
        //   spawn_y + min_z × scale
        // We want that to be at least effective_floor_clearance:
        //   spawn_y ≥ effective_floor_clearance − min_z × scale
        // Shadow-quad meshes are excluded because they remain flat on the floor
        // and their Z values are irrelevant to the upright geometry.
        let item_min_z = creature_def
            .meshes
            .iter()
            .filter(|m| m.name.as_deref() != Some("shadow_quad"))
            .flat_map(|m| m.vertices.iter().map(|v| v[2]))
            .fold(f32::INFINITY, f32::min);

        let item_spawn_y = if item_min_z.is_finite() && item_min_z < 0.0 {
            // Raise the origin so the lowest vertex clears the floor, but never
            // below DROPPED_ITEM_MIN_HEIGHT (keeps small items visible).
            (effective_floor_clearance - item_min_z * creature_def.scale)
                .max(DROPPED_ITEM_MIN_HEIGHT)
        } else {
            // No negative-Z geometry: use effective clearance directly so even
            // flat items (rings, scrolls) sit above grass blades.
            effective_floor_clearance.max(DROPPED_ITEM_MIN_HEIGHT)
        };

        // World-space position: tile centre at the dynamically computed height.
        let world_pos = Vec3::new(
            ev.tile_x as f32 + TILE_CENTER_OFFSET,
            item_spawn_y,
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

        // Apply the deterministic Y-axis spin and add a Billboard so the item
        // always faces the camera.
        //
        // The upright tilt is already baked into each child MeshTransform
        // (see the loop above), so only a Y rotation is needed here to set the
        // initial facing direction.  The Billboard component (lock_y: true)
        // updates the parent Y rotation every frame so the item face — whose
        // normal is local −Z after the X tilt — always points toward the player
        // regardless of which direction they approach from.
        commands
            .entity(entity)
            .entry::<Transform>()
            .and_modify(move |mut transform| {
                transform.rotation = Quat::from_rotation_y(jitter_y);
            });

        commands.entity(entity).insert(Billboard { lock_y: true });

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

/// Fires [`ItemDroppedEvent`] for every dropped item on the current map so that
/// all dropped items — both static map-authored entries and items dropped at
/// runtime by the party — go through the same visual spawn path.
///
/// Two sources are iterated on each map load:
///
/// 1. **`map.events` (`MapEvent::DroppedItem` variants)** — static drops
///    authored in campaign data files.
/// 2. **`map.dropped_items` (Vec of [`DroppedItem`](crate::domain::world::DroppedItem))** —
///    runtime drops placed in the world by the party via `drop_item()`.  These
///    are stored in the domain `Map` struct and survive save/load round-trips.
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

    // ── Source 1: static map-authored DroppedItem events ─────────────────
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

    // ── Source 2: runtime-dropped items stored in map.dropped_items ───────
    //
    // Items placed in the world by the party at runtime
    // (via `drop_item()`) are stored in `Map::dropped_items`.  Emit an event
    // for each so that `spawn_dropped_item_system` spawns their visual markers,
    // giving dropped items persistent visuals that survive save/load.
    for dropped in &map.dropped_items {
        event_writer.write(ItemDroppedEvent {
            item_id: dropped.item_id,
            charges: dropped.charges as u16,
            map_id: current_map_id,
            tile_x: dropped.position.x,
            tile_y: dropped.position.y,
        });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // §4.2 — test_deterministic_drop_rotation_same_inputs
    /// Same inputs must always produce the same rotation angle.
    #[test]
    fn test_deterministic_drop_rotation_same_inputs() {
        let r1 = deterministic_drop_rotation(1, 5, 10, 42);
        let r2 = deterministic_drop_rotation(1, 5, 10, 42);
        assert_eq!(r1, r2, "same inputs must yield identical rotation");
    }

    // §4.2 — test_deterministic_drop_rotation_different_tiles
    /// Different tile positions must (in practice) yield different angles.
    #[test]
    fn test_deterministic_drop_rotation_different_tiles() {
        let r1 = deterministic_drop_rotation(1, 5, 10, 1);
        let r2 = deterministic_drop_rotation(1, 6, 10, 1);
        let r3 = deterministic_drop_rotation(1, 5, 11, 1);
        // These should differ (they use different primes in the hash)
        assert_ne!(r1, r2, "different tile_x must yield different rotation");
        assert_ne!(r1, r3, "different tile_y must yield different rotation");
    }

    // §4.2 — test_deterministic_drop_rotation_in_range
    /// Rotation must be in [0, TAU).
    #[test]
    fn test_deterministic_drop_rotation_in_range() {
        for tile_x in 0..10_i32 {
            for tile_y in 0..10_i32 {
                let r = deterministic_drop_rotation(1, tile_x, tile_y, 5);
                assert!(
                    (0.0..std::f32::consts::TAU).contains(&r),
                    "rotation {r} must be in [0, TAU) for tile ({tile_x},{tile_y})"
                );
            }
        }
    }

    // §4.2 — test_deterministic_drop_rotation_different_items
    /// Different item IDs on the same tile must yield different rotations.
    #[test]
    fn test_deterministic_drop_rotation_different_items() {
        let r1 = deterministic_drop_rotation(1, 5, 5, 1);
        let r2 = deterministic_drop_rotation(1, 5, 5, 2);
        assert_ne!(r1, r2, "different item_id must yield different rotation");
    }

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

    /// The `DROPPED_ITEM_MIN_HEIGHT` constant must be positive so items sit above the floor.
    #[test]
    fn test_dropped_item_y_is_positive() {
        const { assert!(DROPPED_ITEM_MIN_HEIGHT > 0.0) }
    }

    /// `DROPPED_ITEM_GRASS_FLOOR_CLEARANCE` must exceed the maximum possible
    /// grass blade height (`GRASS_BLADE_HEIGHT_BASE * max_variation = 0.4 * 1.3 = 0.52`)
    /// so weapons on grass tiles are not visually buried inside the grass.
    #[test]
    fn test_dropped_item_grass_clearance_exceeds_max_grass_blade_height() {
        // Grass blades: base 0.4 units × max height variation 1.3 = 0.52 units.
        // See advanced_grass::GRASS_BLADE_HEIGHT_BASE and the spawn_grass_cluster
        // height_variation range (0.7..=1.3).
        const { assert!(DROPPED_ITEM_GRASS_FLOOR_CLEARANCE > 0.52_f32) }
    }

    /// `DROPPED_ITEM_GRASS_FLOOR_CLEARANCE` must be greater than the standard
    /// `DROPPED_ITEM_FLOOR_CLEARANCE` to ensure grass tiles use a higher value.
    #[test]
    fn test_grass_clearance_exceeds_standard_clearance() {
        const { assert!(DROPPED_ITEM_GRASS_FLOOR_CLEARANCE > DROPPED_ITEM_FLOOR_CLEARANCE) }
    }

    /// The tile-centre offset must be 0.5 (half a 1-unit tile).
    #[test]
    fn test_tile_center_offset_is_half() {
        assert!((TILE_CENTER_OFFSET - 0.5_f32).abs() < f32::EPSILON);
    }
}
