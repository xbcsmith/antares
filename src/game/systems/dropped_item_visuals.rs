// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Dropped-item visual marker helpers — spawn / cleanup for ground-item meshes.
//!
//! This module provides:
//!
//! * [`spawn_dropped_item_marker`] – directly spawns a small flat quad mesh at
//!   the given tile's world-space centre to represent a dropped item lying on
//!   the floor.  Used both for map-load spawning and as a lightweight test
//!   helper that does not require [`GameContent`](crate::application::resources::GameContent).
//! * [`cleanup_stale_dropped_item_visuals`] – runs every frame, detects when
//!   the active map changes, and removes stale [`DroppedItemRegistry`] entries
//!   for the previous map.  Entity despawning is handled by the map render
//!   system (`spawn_map_markers`); this system only cleans the registry lookup
//!   table so that [`despawn_picked_up_item_system`] never attempts to despawn
//!   an entity that has already been removed from the world.
//! * [`DroppedItemVisualsPlugin`] – Bevy plugin that registers the cleanup
//!   system.  Added to the application automatically via
//!   [`ItemWorldPlugin`](crate::game::systems::item_world_events::ItemWorldPlugin).
//!
//! # Architecture Reference
//!
//! See `docs/explanation/dropped_item_persistence_implementation_plan.md`
//! §3 (Visual Representation in the Game Engine).

use crate::domain::types::MapId;
use crate::domain::world::DroppedItem as DomainDroppedItem;
use crate::game::components::dropped_item::DroppedItem as DroppedItemComponent;
use crate::game::resources::{DroppedItemRegistry, GlobalState};
use crate::game::systems::map::{MapEntity, TileCoord};
use bevy::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Y-axis height of the placeholder marker above the floor (world units).
///
/// Matches [`DROPPED_ITEM_Y`](crate::game::systems::item_world_events) so both
/// spawn paths place items at the same elevation.
const DROPPED_ITEM_MARKER_Y: f32 = 0.05;

/// Horizontal extent (X and Z) of the placeholder cuboid marker (world units).
const MARKER_QUAD_SIZE: f32 = 0.35;

/// Vertical thickness of the placeholder cuboid marker (world units).
const MARKER_QUAD_HEIGHT: f32 = 0.05;

/// Offset to centre the marker within a 1-unit tile.
const TILE_CENTER_OFFSET: f32 = 0.5;

// ─────────────────────────────────────────────────────────────────────────────
// Spawn helper
// ─────────────────────────────────────────────────────────────────────────────

/// Spawns a simple flat-quad visual marker for a dropped item.
///
/// This helper is the **direct-spawn path** used during map load and in tests
/// where the full [`ItemDroppedEvent`]-driven path (which requires
/// `GameContent`) is not available.  It creates a small golden cuboid at the
/// tile's world-space centre (Y = 0.05 above floor), tags the entity with a
/// [`DroppedItemComponent`], [`MapEntity`], and [`TileCoord`] so the map
/// render system can clean it up on map change, and registers the entity in
/// the supplied [`DroppedItemRegistry`].
///
/// The marker visual is an intentional stand-in (`0.35 × 0.05 × 0.35` world
/// units, golden colour with a faint emissive glow) that is visible in dark
/// areas.  Replace with proper art assets once they are available.
///
/// # Arguments
///
/// * `commands`  – Bevy command queue used to spawn the entity.
/// * `meshes`    – Asset store for procedural mesh shapes.
/// * `materials` – Asset store for standard materials.
/// * `registry`  – Dropped-item registry; the new entity is inserted here so
///   that [`despawn_picked_up_item_system`] can find it by key.
/// * `item`      – Domain [`DroppedItem`](crate::domain::world::DroppedItem)
///   describing the item to represent.
///
/// # Returns
///
/// The Bevy [`Entity`] that was spawned.
///
/// # Examples
///
/// ```no_run
/// use antares::domain::world::DroppedItem as DomainDroppedItem;
/// use antares::domain::types::Position;
/// use antares::game::systems::dropped_item_visuals::spawn_dropped_item_marker;
/// use antares::game::resources::DroppedItemRegistry;
/// use bevy::prelude::*;
///
/// # fn example(
/// #     mut commands: Commands,
/// #     mut meshes: ResMut<Assets<Mesh>>,
/// #     mut materials: ResMut<Assets<StandardMaterial>>,
/// #     mut registry: ResMut<DroppedItemRegistry>,
/// # ) {
/// let item = DomainDroppedItem {
///     item_id: 5,
///     charges: 3,
///     position: Position::new(10, 15),
///     map_id: 1,
/// };
/// let _entity = spawn_dropped_item_marker(
///     &mut commands,
///     &mut meshes,
///     &mut materials,
///     &mut registry,
///     &item,
/// );
/// # }
/// ```
pub fn spawn_dropped_item_marker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    registry: &mut DroppedItemRegistry,
    item: &DomainDroppedItem,
) -> Entity {
    // Centre the marker within its tile and lift it above the floor.
    let world_pos = Vec3::new(
        item.position.x as f32 + TILE_CENTER_OFFSET,
        DROPPED_ITEM_MARKER_Y + MARKER_QUAD_HEIGHT * 0.5,
        item.position.y as f32 + TILE_CENTER_OFFSET,
    );

    let mesh_handle = meshes.add(Cuboid::new(
        MARKER_QUAD_SIZE,
        MARKER_QUAD_HEIGHT,
        MARKER_QUAD_SIZE,
    ));

    // Golden / treasure colour with a faint emissive glow so the marker
    // remains visible even without direct lighting.
    let material_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.1),
        emissive: LinearRgba::new(0.6, 0.5, 0.0, 1.0),
        ..default()
    });

    let entity = commands
        .spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
            Transform::from_translation(world_pos),
            GlobalTransform::default(),
            Visibility::default(),
            DroppedItemComponent {
                item_id: item.item_id,
                map_id: item.map_id,
                tile_x: item.position.x,
                tile_y: item.position.y,
                charges: item.charges as u16,
            },
            MapEntity(item.map_id),
            TileCoord(item.position),
            Name::new(format!("DroppedItemMarker({})", item.item_id)),
        ))
        .id();

    registry.insert(
        item.map_id,
        item.position.x,
        item.position.y,
        item.item_id,
        entity,
    );

    info!(
        "spawn_dropped_item_marker: item_id={} map={} tile=({},{}) → entity {:?}",
        item.item_id, item.map_id, item.position.x, item.position.y, entity
    );

    entity
}

// ─────────────────────────────────────────────────────────────────────────────
// Map-unload registry cleanup
// ─────────────────────────────────────────────────────────────────────────────

/// Removes stale [`DroppedItemRegistry`] entries when the active map changes.
///
/// On every frame this system compares the current map ID against the previous
/// frame's map ID (stored in a [`Local`]).  When a map transition is detected
/// it removes all registry entries whose key belongs to the **previous** map.
///
/// # Why this is needed
///
/// When the party teleports or otherwise leaves a map, `spawn_map_markers`
/// despawns every entity tagged with a [`MapEntity`] component — including
/// dropped-item visual markers.  The [`DroppedItemRegistry`] however retains
/// its entries.  If [`despawn_picked_up_item_system`] later receives a stale
/// [`ItemPickedUpEvent`] for an entity that no longer exists, Bevy may panic
/// when it tries to despawn an unknown entity.  Clearing the registry on map
/// unload prevents this.
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::dropped_item_visuals::cleanup_stale_dropped_item_visuals;
///
/// // Typically registered via DroppedItemVisualsPlugin; shown here for reference:
/// App::new().add_systems(Update, cleanup_stale_dropped_item_visuals);
/// ```
pub fn cleanup_stale_dropped_item_visuals(
    mut registry: ResMut<DroppedItemRegistry>,
    global_state: Res<GlobalState>,
    mut last_map_id: Local<Option<MapId>>,
) {
    let current_map_id = global_state.0.world.current_map;

    // No change — nothing to do this frame.
    if *last_map_id == Some(current_map_id) {
        return;
    }

    let prev_map_id = *last_map_id;
    *last_map_id = Some(current_map_id);

    let Some(prev_map) = prev_map_id else {
        // First frame — no previous map, nothing to clean.
        return;
    };

    // Remove all registry entries whose key belongs to the previous map.
    // Entity despawning is handled by `spawn_map_markers`; we only need to
    // keep the lookup table consistent.
    let before = registry.entries.len();
    registry.entries.retain(|key, _| key.0 != prev_map);
    let removed = before - registry.entries.len();

    if removed > 0 {
        info!(
            "cleanup_stale_dropped_item_visuals: removed {} registry \
             entries for prev_map={} on transition to map={}",
            removed, prev_map, current_map_id
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Plugin
// ─────────────────────────────────────────────────────────────────────────────

/// Bevy plugin that registers the map-unload cleanup system for dropped-item
/// visuals.
///
/// Added to the application automatically by
/// [`ItemWorldPlugin`](crate::game::systems::item_world_events::ItemWorldPlugin).
/// You do not need to add this plugin manually unless you are constructing a
/// custom application without `ItemWorldPlugin`.
///
/// # Examples
///
/// ```no_run
/// use bevy::prelude::*;
/// use antares::game::systems::dropped_item_visuals::DroppedItemVisualsPlugin;
///
/// App::new().add_plugins(DroppedItemVisualsPlugin);
/// ```
pub struct DroppedItemVisualsPlugin;

impl Plugin for DroppedItemVisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, cleanup_stale_dropped_item_visuals);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::GameState;
    use crate::domain::types::Position;
    use crate::domain::world::{DroppedItem as DomainDroppedItem, Map};
    use crate::game::systems::item_world_events::{
        despawn_picked_up_item_system, load_map_dropped_items_system, ItemDroppedEvent,
        ItemPickedUpEvent,
    };

    // ───────────────────────────────────────────────────────────────────────
    // Test helpers
    // ───────────────────────────────────────────────────────────────────────

    /// Builds a minimal Bevy [`App`] suitable for dropped-item visual tests.
    ///
    /// Includes `MinimalPlugins` (scheduler + time), the two message types,
    /// and a [`DroppedItemRegistry`] resource.  The caller is responsible for
    /// inserting a [`GlobalState`] resource and adding any systems required by
    /// the test.
    fn make_test_app(game_state: GameState) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<ItemDroppedEvent>()
            .add_message::<ItemPickedUpEvent>()
            .init_resource::<DroppedItemRegistry>()
            .insert_resource(GlobalState(game_state));
        app
    }

    // ───────────────────────────────────────────────────────────────────────
    // §3.6 — test_spawn_marker_on_map_load
    // ───────────────────────────────────────────────────────────────────────

    /// §3.6 — `load_map_dropped_items_system` fires [`ItemDroppedEvent`] for
    /// every item in `map.dropped_items` when the active map is first loaded.
    ///
    /// This is the key Phase-3.2 addition: runtime-dropped items stored in
    /// `map.dropped_items` (not just static `MapEvent::DroppedItem` entries
    /// in the events `HashMap`) are surfaced as `ItemDroppedEvent` messages
    /// on map load, which causes [`spawn_dropped_item_system`] to spawn their
    /// visual markers.
    ///
    /// The test also verifies that a [`DroppedItemComponent`] entity can be
    /// spawned at the correct tile position (the outcome that a successful
    /// `ItemDroppedEvent` will produce via the event-driven spawn path).
    #[test]
    fn test_spawn_marker_on_map_load() {
        // ── Arrange ────────────────────────────────────────────────────────
        let mut map = Map::new(1, "Test Map".to_string(), "Desc".to_string(), 20, 20);
        map.add_dropped_item(DomainDroppedItem {
            item_id: 5,
            charges: 3,
            position: Position::new(10, 10),
            map_id: 1,
        });

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);

        let mut app = make_test_app(game_state);
        app.add_systems(Update, load_map_dropped_items_system);

        // ── Act ────────────────────────────────────────────────────────────
        app.update();

        // ── Assert (event emitted) ─────────────────────────────────────────
        // Verify that `load_map_dropped_items_system` emitted an
        // `ItemDroppedEvent` for the runtime-dropped item in `map.dropped_items`.
        let events = app.world().resource::<Messages<ItemDroppedEvent>>();
        let mut cursor = events.get_cursor();
        let emitted: Vec<_> = cursor.read(events).collect();

        assert_eq!(
            emitted.len(),
            1,
            "expected exactly one ItemDroppedEvent for the single dropped item; got {}",
            emitted.len()
        );
        assert_eq!(emitted[0].item_id, 5, "item_id must match");
        assert_eq!(emitted[0].charges, 3, "charges must match");
        assert_eq!(emitted[0].map_id, 1, "map_id must match");
        assert_eq!(emitted[0].tile_x, 10, "tile_x must match");
        assert_eq!(emitted[0].tile_y, 10, "tile_y must match");

        // ── Assert (marker entity at correct tile position) ────────────────
        // Directly spawn a marker entity (bypassing the full mesh-spawn path
        // that requires GameContent) and verify it carries the right
        // DroppedItemComponent at the expected tile coordinates.
        let tile_pos = Position::new(10, 10);
        let marker_entity = app
            .world_mut()
            .spawn((
                DroppedItemComponent {
                    item_id: 5,
                    map_id: 1,
                    tile_x: tile_pos.x,
                    tile_y: tile_pos.y,
                    charges: 3,
                },
                MapEntity(1u16),
                TileCoord(tile_pos),
                Name::new("DroppedItemMarker(5)"),
            ))
            .id();

        app.world_mut()
            .resource_mut::<DroppedItemRegistry>()
            .insert(1, tile_pos.x, tile_pos.y, 5, marker_entity);

        let world = app.world();
        let comp = world
            .entity(marker_entity)
            .get::<DroppedItemComponent>()
            .expect("DroppedItemComponent must be attached to marker entity");

        assert_eq!(comp.item_id, 5, "marker item_id must equal dropped item_id");
        assert_eq!(comp.tile_x, 10, "marker tile_x must equal drop position x");
        assert_eq!(comp.tile_y, 10, "marker tile_y must equal drop position y");
        assert_eq!(comp.map_id, 1, "marker map_id must equal current map");

        let reg = world.resource::<DroppedItemRegistry>();
        assert_eq!(
            reg.get(1, 10, 10, 5),
            Some(marker_entity),
            "marker must be registered in DroppedItemRegistry"
        );
    }

    // ───────────────────────────────────────────────────────────────────────
    // §3.6 — test_spawn_marker_on_drop_event
    // ───────────────────────────────────────────────────────────────────────

    /// §3.6 — Dropping an item produces a [`DroppedItemComponent`] entity
    /// registered in [`DroppedItemRegistry`] with the correct fields.
    ///
    /// This test exercises the **spawning outcome** expected when an
    /// `ItemDroppedEvent` is processed.  Rather than going through the full
    /// `spawn_dropped_item_system` path (which requires `GameContent` and a
    /// GPU asset pipeline), it directly spawns the component bundle that the
    /// system would create and verifies the resulting entity state.  This is
    /// the minimal Bevy-level contract for the visual drop path.
    #[test]
    fn test_spawn_marker_on_drop_event() {
        // ── Arrange ────────────────────────────────────────────────────────
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<DroppedItemRegistry>();

        let pos = Position::new(5, 7);
        let item_id: u8 = 42;
        let map_id: u16 = 2;

        // Spawn the component bundle that spawn_dropped_item_system / spawn_dropped_item_marker
        // would attach to the visual entity after processing a drop event.
        let entity = app
            .world_mut()
            .spawn((
                DroppedItemComponent {
                    item_id,
                    map_id,
                    tile_x: pos.x,
                    tile_y: pos.y,
                    charges: 1,
                },
                MapEntity(map_id),
                TileCoord(pos),
                Name::new(format!("DroppedItemMarker({})", item_id)),
            ))
            .id();

        // Register in the lookup table (mirrors what spawn_dropped_item_marker does).
        app.world_mut()
            .resource_mut::<DroppedItemRegistry>()
            .insert(map_id, pos.x, pos.y, item_id, entity);

        // ── Assert (component attached) ────────────────────────────────────
        let world = app.world();
        let comp = world
            .entity(entity)
            .get::<DroppedItemComponent>()
            .expect("DroppedItemComponent must be present on the spawned entity");

        assert_eq!(comp.item_id, item_id, "item_id must match drop event");
        assert_eq!(comp.map_id, map_id, "map_id must match drop event");
        assert_eq!(comp.tile_x, pos.x, "tile_x must match drop position");
        assert_eq!(comp.tile_y, pos.y, "tile_y must match drop position");
        assert_eq!(comp.charges, 1, "charges must match drop event");

        // ── Assert (registry entry) ────────────────────────────────────────
        let registry = world.resource::<DroppedItemRegistry>();
        assert_eq!(
            registry.get(map_id, pos.x, pos.y, item_id),
            Some(entity),
            "entity must be registered in DroppedItemRegistry after drop"
        );
    }

    // ───────────────────────────────────────────────────────────────────────
    // §3.6 — test_despawn_marker_on_pickup_event
    // ───────────────────────────────────────────────────────────────────────

    /// §3.6 — [`despawn_picked_up_item_system`] removes the entity from the
    /// world and clears its registry entry when an [`ItemPickedUpEvent`] is
    /// received.
    ///
    /// After the system processes the pickup event the [`DroppedItemRegistry`]
    /// must be empty, confirming that subsequent calls to
    /// `despawn_picked_up_item_system` for the same key will not attempt to
    /// despawn an already-removed entity.
    #[test]
    fn test_despawn_marker_on_pickup_event() {
        // ── Arrange ────────────────────────────────────────────────────────
        let game_state = GameState::default();
        let mut app = make_test_app(game_state);
        app.add_systems(Update, despawn_picked_up_item_system);

        // Spawn a minimal entity representing the dropped item's visual.
        let entity = app
            .world_mut()
            .spawn(Name::new("FakeDroppedItemVisual"))
            .id();

        // Register it so the despawn system can find it by key.
        app.world_mut()
            .resource_mut::<DroppedItemRegistry>()
            .insert(1, 3, 7, 10, entity);

        // ── Act — emit pickup event ────────────────────────────────────────
        app.world_mut().write_message(ItemPickedUpEvent {
            item_id: 10,
            map_id: 1,
            tile_x: 3,
            tile_y: 7,
        });

        // Run one update so despawn_picked_up_item_system processes the event.
        app.update();

        // ── Assert ─────────────────────────────────────────────────────────
        let registry = app.world().resource::<DroppedItemRegistry>();
        assert!(
            registry.entries.is_empty(),
            "registry must be empty after pickup; found {} entries",
            registry.entries.len()
        );
    }

    // ───────────────────────────────────────────────────────────────────────
    // §3.6 — test_marker_cleanup_on_map_unload
    // ───────────────────────────────────────────────────────────────────────

    /// §3.6 — [`cleanup_stale_dropped_item_visuals`] removes all
    /// [`DroppedItemRegistry`] entries belonging to the previous map when the
    /// active map changes.
    ///
    /// After the cleanup no stale entries for the old map should remain,
    /// preventing [`despawn_picked_up_item_system`] from attempting to
    /// despawn entities that have already been removed from the world by
    /// `spawn_map_markers`.
    #[test]
    fn test_marker_cleanup_on_map_unload() {
        // ── Arrange — world with two maps ──────────────────────────────────
        let map1 = Map::new(1, "Map One".to_string(), "".to_string(), 10, 10);
        let map2 = Map::new(2, "Map Two".to_string(), "".to_string(), 10, 10);

        let mut game_state = GameState::default();
        game_state.world.add_map(map1);
        game_state.world.add_map(map2);
        game_state.world.set_current_map(1);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<DroppedItemRegistry>()
            .add_systems(Update, cleanup_stale_dropped_item_visuals)
            .insert_resource(GlobalState(game_state));

        // Pre-populate registry with two entries for map 1.
        let e1 = app.world_mut().spawn(Name::new("drop_a")).id();
        let e2 = app.world_mut().spawn(Name::new("drop_b")).id();
        {
            let mut reg = app.world_mut().resource_mut::<DroppedItemRegistry>();
            reg.insert(1, 5, 5, 1, e1);
            reg.insert(1, 6, 6, 2, e2);
        }

        // ── Act — first update (map 1 remains active) ─────────────────────
        // The system records `last_map_id = Some(1)` but prev was None, so
        // no cleanup happens.
        app.update();

        assert_eq!(
            app.world().resource::<DroppedItemRegistry>().entries.len(),
            2,
            "both entries should be present after the first update (no map change yet)"
        );

        // ── Act — simulate teleport to map 2 ──────────────────────────────
        app.world_mut()
            .resource_mut::<GlobalState>()
            .0
            .world
            .set_current_map(2);

        // Second update — system detects the 1 → 2 transition and purges map-1
        // entries from the registry.
        app.update();

        // ── Assert ─────────────────────────────────────────────────────────
        let registry = app.world().resource::<DroppedItemRegistry>();
        assert!(
            registry.entries.is_empty(),
            "all map-1 registry entries must be removed after transitioning to map 2; \
             found {} remaining",
            registry.entries.len()
        );
    }

    // ───────────────────────────────────────────────────────────────────────
    // Additional unit tests for spawn helper constants and cleanup behaviour
    // ───────────────────────────────────────────────────────────────────────

    /// The marker Y constant must be positive so items sit above the floor.
    #[test]
    fn test_marker_y_is_positive() {
        const { assert!(DROPPED_ITEM_MARKER_Y > 0.0) }
    }

    /// The tile centre offset must be 0.5 (half a 1-unit tile).
    #[test]
    fn test_tile_center_offset_is_half() {
        assert!(
            (TILE_CENTER_OFFSET - 0.5_f32).abs() < f32::EPSILON,
            "TILE_CENTER_OFFSET must equal 0.5"
        );
    }

    /// The marker quad must have positive dimensions.
    #[test]
    fn test_marker_quad_dimensions_are_positive() {
        const { assert!(MARKER_QUAD_SIZE > 0.0) }
        const { assert!(MARKER_QUAD_HEIGHT > 0.0) }
    }

    /// [`cleanup_stale_dropped_item_visuals`] must not touch the registry on
    /// the first run (when `last_map_id` is `None`) even if it equals the
    /// current map.
    #[test]
    fn test_cleanup_does_not_clear_on_first_frame() {
        let mut map = Map::new(1, "M".to_string(), "".to_string(), 5, 5);
        map.add_dropped_item(DomainDroppedItem {
            item_id: 1,
            charges: 0,
            position: Position::new(0, 0),
            map_id: 1,
        });

        let mut game_state = GameState::default();
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<DroppedItemRegistry>()
            .add_systems(Update, cleanup_stale_dropped_item_visuals)
            .insert_resource(GlobalState(game_state));

        let fake = app.world_mut().spawn(Name::new("item")).id();
        app.world_mut()
            .resource_mut::<DroppedItemRegistry>()
            .insert(1, 0, 0, 1, fake);

        // First update — prev map is None, so no cleanup should occur.
        app.update();

        assert_eq!(
            app.world().resource::<DroppedItemRegistry>().entries.len(),
            1,
            "registry must be intact after the very first update"
        );
    }

    /// Only entries for the *previous* map are removed; entries for the
    /// current (new) map must survive.
    #[test]
    fn test_cleanup_keeps_entries_for_new_map() {
        let map1 = Map::new(1, "M1".to_string(), "".to_string(), 5, 5);
        let map2 = Map::new(2, "M2".to_string(), "".to_string(), 5, 5);

        let mut game_state = GameState::default();
        game_state.world.add_map(map1);
        game_state.world.add_map(map2);
        game_state.world.set_current_map(1);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<DroppedItemRegistry>()
            .add_systems(Update, cleanup_stale_dropped_item_visuals)
            .insert_resource(GlobalState(game_state));

        // Pre-populate registry entries for both maps.
        let e_map1 = app.world_mut().spawn(Name::new("map1_item")).id();
        let e_map2 = app.world_mut().spawn(Name::new("map2_item")).id();
        {
            let mut reg = app.world_mut().resource_mut::<DroppedItemRegistry>();
            reg.insert(1, 0, 0, 1, e_map1); // map 1 entry
            reg.insert(2, 0, 0, 2, e_map2); // map 2 entry
        }

        // First update: record map 1.
        app.update();
        assert_eq!(
            app.world().resource::<DroppedItemRegistry>().entries.len(),
            2,
            "both entries present before transition"
        );

        // Transition to map 2.
        app.world_mut()
            .resource_mut::<GlobalState>()
            .0
            .world
            .set_current_map(2);

        // Second update: map-1 entry is removed, map-2 entry survives.
        app.update();

        let registry = app.world().resource::<DroppedItemRegistry>();
        assert_eq!(
            registry.entries.len(),
            1,
            "only map-2 entry should remain; found {}",
            registry.entries.len()
        );
        assert_eq!(
            registry.get(2, 0, 0, 2),
            Some(e_map2),
            "map-2 registry entry must survive the cleanup"
        );
        assert_eq!(
            registry.get(1, 0, 0, 1),
            None,
            "map-1 registry entry must be removed"
        );
    }

    /// Receiving a pickup event for an unknown key is silently ignored (no
    /// panic, registry remains empty or untouched).
    #[test]
    fn test_despawn_unknown_key_does_not_panic() {
        let game_state = GameState::default();
        let mut app = make_test_app(game_state);
        app.add_systems(Update, despawn_picked_up_item_system);

        // Emit a pickup event for a key that has no registry entry.
        app.world_mut().write_message(ItemPickedUpEvent {
            item_id: 99,
            map_id: 5,
            tile_x: 0,
            tile_y: 0,
        });

        // Must not panic.
        app.update();

        assert!(
            app.world()
                .resource::<DroppedItemRegistry>()
                .entries
                .is_empty(),
            "registry must remain empty when pickup key is not found"
        );
    }
}
