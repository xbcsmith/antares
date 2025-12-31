// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::domain::types;
use crate::domain::world;
use crate::game::resources::GlobalState;
use bevy::prelude::*;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type alias for mesh cache keys (width_x, height, width_z)
type MeshDimensions = (OrderedFloat<f32>, OrderedFloat<f32>, OrderedFloat<f32>);

/// Type alias for the mesh cache HashMap
type MeshCache = HashMap<MeshDimensions, Handle<Mesh>>;

/// Plugin that renders the current map using Bevy meshes/materials.
///
/// Note: The visual rendering plugin remains focused on rendering. The map
/// management (spawning/despawning event trigger and marker entities as maps
/// change) is implemented alongside it to enable a dynamic map system.
pub struct MapRenderingPlugin;

/// Component tagging an entity as belonging to a specific map
#[derive(bevy::prelude::Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapEntity(pub types::MapId);

/// Component that stores the position of a spawned tile/entity
#[derive(bevy::prelude::Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileCoord(pub types::Position);

/// Component tagging an entity as an NPC visual marker
#[derive(bevy::prelude::Component, Debug, Clone, PartialEq, Eq)]
pub struct NpcMarker {
    /// NPC ID from the definition
    pub npc_id: String,
}

/// Event trigger component - attached to entities that represent in-world event triggers
#[derive(bevy::prelude::Component, Debug, Clone)]
pub struct EventTrigger {
    /// The trigger's event type
    pub event_type: MapEventType,
    /// Position on the map for which this trigger is placed
    pub position: types::Position,
}

/// Lightweight event type used by ECS triggers (converts from domain MapEvent)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapEventType {
    Teleport {
        target_map: types::MapId,
        target_pos: types::Position,
    },
    NpcDialogue {
        npc_id: String,
    },
    CombatEncounter {
        monster_group_id: u8,
    },
    TreasureChest {
        loot_table_id: u8,
    },
}

/// Message used to request a map change (teleportation, portal, etc.)
#[derive(Message, Clone)]
pub struct MapChangeEvent {
    pub target_map: types::MapId,
    pub target_pos: types::Position,
}

/// Message sent when a door is opened - triggers map visual refresh
#[derive(Message, Clone)]
pub struct DoorOpenedEvent {
    pub position: types::Position,
}

/// Plugin responsible for dynamic map management (spawning/despawning marker
/// entities and event triggers when the current map changes).
pub struct MapManagerPlugin;

impl Plugin for MapManagerPlugin {
    fn build(&self, app: &mut App) {
        // Register the map change message and the handler + spawn systems
        app.add_message::<MapChangeEvent>()
            .add_message::<DoorOpenedEvent>()
            // Process explicit map change requests first, then let the marker
            // spawner observe the changed world state and spawn/despawn accordingly.
            .add_systems(
                Update,
                (map_change_handler, spawn_map_markers, handle_door_opened),
            );
    }
}

impl Plugin for MapRenderingPlugin {
    fn build(&self, app: &mut App) {
        // Keep the visual spawn on startup (original behavior), and add the
        // map manager plugin so dynamic changes are handled at runtime.
        app.add_systems(Startup, spawn_map)
            .add_plugins(MapManagerPlugin);
    }
}

/// System that handles door opened messages by refreshing map visuals
#[allow(unused_mut)] // spawn_map requires mut even though clippy doesn't detect it
fn handle_door_opened(
    mut door_messages: MessageReader<DoorOpenedEvent>,
    mut commands: Commands,
    query_existing: Query<Entity, With<MapEntity>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    global_state: Res<GlobalState>,
    content: Res<crate::application::resources::GameContent>,
) {
    // Only refresh if a door was actually opened
    if door_messages.read().count() == 0 {
        return;
    }

    info!("Door opened - refreshing map visuals");

    // Despawn all existing map entities
    for entity in query_existing.iter() {
        commands.entity(entity).despawn();
    }

    // Respawn the map with updated door states
    spawn_map(commands, meshes, materials, global_state, content);
}

/// Converts a domain MapEvent into a lightweight MapEventType (if supported)
fn map_event_to_event_type(ev: &world::MapEvent) -> Option<MapEventType> {
    match ev {
        world::MapEvent::Teleport {
            destination,
            map_id,
            ..
        } => Some(MapEventType::Teleport {
            target_map: *map_id,
            target_pos: *destination,
        }),
        world::MapEvent::NpcDialogue { npc_id, .. } => Some(MapEventType::NpcDialogue {
            npc_id: npc_id.clone(),
        }),
        world::MapEvent::Encounter { monster_group, .. } => {
            // For the lightweight form we store the primary group id (if any)
            let gid = *monster_group.first().unwrap_or(&0);
            Some(MapEventType::CombatEncounter {
                monster_group_id: gid,
            })
        }
        world::MapEvent::Treasure { loot, .. } => {
            let lid = *loot.first().unwrap_or(&0);
            Some(MapEventType::TreasureChest { loot_table_id: lid })
        }
        _ => None,
    }
}

/// System that handles explicit MapChangeEvent messages by updating the world
/// current map and party position. Invalid map ids are ignored (no panic).
fn map_change_handler(
    mut ev_reader: MessageReader<MapChangeEvent>,
    mut global_state: ResMut<GlobalState>,
) {
    for ev in ev_reader.read() {
        if global_state.0.world.get_map(ev.target_map).is_some() {
            global_state.0.world.set_current_map(ev.target_map);
            global_state.0.world.set_party_position(ev.target_pos);
        } else {
            // Gracefully ignore invalid map changes
            warn!(
                "MapChangeEvent target_map {} not found; ignoring",
                ev.target_map
            );
        }
    }
}

/// System that observes the world's current map and spawns marker entities
/// (tiles and event triggers) for the active map. When the active map changes
/// it despawns previously spawned map entities.
fn spawn_map_markers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    global_state: Res<GlobalState>,
    content: Res<crate::application::resources::GameContent>,
    query_existing: Query<Entity, With<MapEntity>>,
    mut last_map: Local<Option<types::MapId>>,
) {
    let current = global_state.0.world.current_map;

    // If map hasn't changed, nothing to do
    if Some(current) == *last_map {
        return;
    }

    // If this is the first time this system runs and there are already map
    // entities present (spawned by `spawn_map` in `Startup`), don't
    // despawn them and don't spawn duplicate markers either. Instead, just
    // record the current map and exit.
    if should_skip_marker_spawn(&last_map, query_existing.iter().next().is_some()) {
        *last_map = Some(current);
        debug!(
            "spawn_map_markers: existing map entities present on first run; leaving visuals intact"
        );
        return;
    } else {
        // We have a previously recorded map and it changed; despawn old map entities
        for entity in query_existing.iter() {
            commands.entity(entity).despawn();
        }
    }

    // Spawn markers for the new map (if it exists)
    if let Some(map) = global_state.0.world.get_current_map() {
        let map_id = map.id;

        // Spawn a lightweight marker entity for every tile (useful for logic & tests)
        for y in 0..map.height {
            for x in 0..map.width {
                let pos = types::Position::new(x as i32, y as i32);
                commands.spawn((MapEntity(map_id), TileCoord(pos)));
            }
        }

        // Spawn EventTrigger entities for each map event that we support
        for (pos, event) in map.events.iter() {
            if let Some(evt_type) = map_event_to_event_type(event) {
                commands.spawn((
                    MapEntity(map_id),
                    EventTrigger {
                        event_type: evt_type,
                        position: *pos,
                    },
                ));
            }
        }

        // Spawn NPC visual markers for the new map (Phase 2: NPC Visual Representation)
        let resolved_npcs = map.resolve_npcs(&content.0.npcs);
        let npc_color = Color::srgb(0.0, 1.0, 1.0); // Cyan
        let npc_material = materials.add(StandardMaterial {
            base_color: npc_color,
            perceptual_roughness: 0.5,
            ..default()
        });

        // Vertical plane representing NPC (billboard-like)
        // 1.0 wide, 1.8 tall (human height ~6 feet), 0.1 depth
        let npc_mesh = meshes.add(Cuboid::new(1.0, 1.8, 0.1));

        for resolved_npc in resolved_npcs.iter() {
            let x = resolved_npc.position.x as f32;
            let y = resolved_npc.position.y as f32;

            // Center the NPC marker at y=0.9 (bottom at 0, top at 1.8)
            commands.spawn((
                Mesh3d(npc_mesh.clone()),
                MeshMaterial3d(npc_material.clone()),
                Transform::from_xyz(x, 0.9, y),
                GlobalTransform::default(),
                Visibility::default(),
                MapEntity(map_id),
                TileCoord(resolved_npc.position),
                NpcMarker {
                    npc_id: resolved_npc.npc_id.clone(),
                },
            ));
        }
    } else {
        // Current map id is set to an unknown map - leave the world empty
        warn!("Current map {} not present in world", current);
    }

    *last_map = Some(current);
}

fn should_skip_marker_spawn(
    last_map: &Option<types::MapId>,
    has_existing_map_entities: bool,
) -> bool {
    last_map.is_none() && has_existing_map_entities
}

/// Helper function to get or create a cached mesh with given dimensions
fn get_or_create_mesh(
    meshes: &mut ResMut<Assets<Mesh>>,
    cache: &mut MeshCache,
    width_x: f32,
    height: f32,
    width_z: f32,
) -> Handle<Mesh> {
    let key = (
        OrderedFloat(width_x),
        OrderedFloat(height),
        OrderedFloat(width_z),
    );
    cache
        .entry(key)
        .or_insert_with(|| meshes.add(Cuboid::new(width_x, height, width_z)))
        .clone()
}

/// Spawns visual entities (meshes/materials) for the current map.
/// This is invoked on demand (e.g., when the map changes).
///
/// Note: Spawned entities are tagged with `MapEntity` for lifecycle management
/// but also tags spawned visual entities with `MapEntity` and `TileCoord` so
/// they are part of the dynamic despawn/spawn lifecycle.
fn spawn_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    global_state: Res<GlobalState>,
    content: Res<crate::application::resources::GameContent>,
) {
    debug!("spawn_map system called");
    let game_state = &global_state.0;

    if let Some(map) = game_state.world.get_current_map() {
        debug!(
            "Found current map: {} (size: {}x{})",
            map.name, map.width, map.height
        );

        // Mesh cache: reuse meshes with identical dimensions
        let mut mesh_cache: MeshCache = HashMap::new();

        // Materials (base colors)
        // RGB tuples are kept to allow per-tile tinting of walls based on terrain
        let floor_rgb = (0.3_f32, 0.3_f32, 0.3_f32);
        let wall_base_rgb = (0.6_f32, 0.6_f32, 0.6_f32);
        let door_rgb = (0.4_f32, 0.2_f32, 0.1_f32); // Brown
        let water_rgb = (0.2_f32, 0.4_f32, 0.8_f32); // Blue
        let mountain_rgb = (0.5_f32, 0.5_f32, 0.5_f32); // Gray rock
        let forest_rgb = (0.2_f32, 0.6_f32, 0.2_f32); // Green
        let grass_rgb = (0.3_f32, 0.5_f32, 0.2_f32); // Darker green floor
        let stone_rgb = (0.5_f32, 0.5_f32, 0.55_f32);
        let dirt_rgb = (0.4_f32, 0.3_f32, 0.2_f32);

        let floor_color = Color::srgb(floor_rgb.0, floor_rgb.1, floor_rgb.2);
        let wall_base_color = Color::srgb(wall_base_rgb.0, wall_base_rgb.1, wall_base_rgb.2);
        let door_color = Color::srgb(door_rgb.0, door_rgb.1, door_rgb.2);
        let water_color = Color::srgb(water_rgb.0, water_rgb.1, water_rgb.2);
        let mountain_color = Color::srgb(mountain_rgb.0, mountain_rgb.1, mountain_rgb.2);
        let forest_color = Color::srgb(forest_rgb.0, forest_rgb.1, forest_rgb.2);
        let grass_color = Color::srgb(grass_rgb.0, grass_rgb.1, grass_rgb.2);

        let floor_material = materials.add(StandardMaterial {
            base_color: floor_color,
            perceptual_roughness: 0.9,
            ..default()
        });

        let water_material = materials.add(StandardMaterial {
            base_color: water_color, // Blue
            perceptual_roughness: 0.3,
            ..default()
        });

        let grass_material = materials.add(StandardMaterial {
            base_color: grass_color, // Darker green floor
            perceptual_roughness: 0.9,
            ..default()
        });

        // Standard meshes for flat terrain (no height)
        let floor_mesh = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
        let water_mesh = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));

        // Iterate over tiles
        for y in 0..map.height {
            for x in 0..map.width {
                let pos = types::Position::new(x as i32, y as i32);
                if let Some(tile) = map.get_tile(pos) {
                    // Determine if this is a perimeter tile
                    let is_perimeter =
                        x == 0 || y == 0 || x == map.width - 1 || y == map.height - 1;

                    // Render based on terrain type
                    match tile.terrain {
                        world::TerrainType::Water => {
                            // Render water slightly below at y = -0.1
                            commands.spawn((
                                Mesh3d(water_mesh.clone()),
                                MeshMaterial3d(water_material.clone()),
                                Transform::from_xyz(x as f32, -0.1, y as f32),
                                GlobalTransform::default(),
                                Visibility::default(),
                                MapEntity(map.id),
                                TileCoord(pos),
                            ));
                        }
                        world::TerrainType::Mountain => {
                            // Use per-tile visual metadata for dimensions
                            let (width_x, height, width_z) =
                                tile.visual.mesh_dimensions(tile.terrain, tile.wall_type);
                            let mesh = get_or_create_mesh(
                                &mut meshes,
                                &mut mesh_cache,
                                width_x,
                                height,
                                width_z,
                            );
                            let y_pos = tile.visual.mesh_y_position(tile.terrain, tile.wall_type);

                            // Apply color tint if specified
                            let mut base_color = mountain_color;
                            if let Some((r, g, b)) = tile.visual.color_tint {
                                base_color = Color::srgb(
                                    mountain_rgb.0 * r,
                                    mountain_rgb.1 * g,
                                    mountain_rgb.2 * b,
                                );
                            }

                            let material = materials.add(StandardMaterial {
                                base_color,
                                perceptual_roughness: 0.8,
                                ..default()
                            });

                            // Apply rotation if specified
                            let rotation = bevy::prelude::Quat::from_rotation_y(
                                tile.visual.rotation_y_radians(),
                            );
                            let transform = Transform::from_xyz(x as f32, y_pos, y as f32)
                                .with_rotation(rotation);

                            commands.spawn((
                                Mesh3d(mesh),
                                MeshMaterial3d(material),
                                transform,
                                GlobalTransform::default(),
                                Visibility::default(),
                                MapEntity(map.id),
                                TileCoord(pos),
                            ));
                        }
                        world::TerrainType::Forest => {
                            // Render floor first
                            commands.spawn((
                                Mesh3d(floor_mesh.clone()),
                                MeshMaterial3d(grass_material.clone()),
                                Transform::from_xyz(x as f32, 0.0, y as f32),
                                GlobalTransform::default(),
                                Visibility::default(),
                                MapEntity(map.id),
                                TileCoord(pos),
                            ));

                            // Use per-tile visual metadata for tree dimensions
                            let (width_x, height, width_z) =
                                tile.visual.mesh_dimensions(tile.terrain, tile.wall_type);
                            let mesh = get_or_create_mesh(
                                &mut meshes,
                                &mut mesh_cache,
                                width_x,
                                height,
                                width_z,
                            );
                            let y_pos = tile.visual.mesh_y_position(tile.terrain, tile.wall_type);

                            // Apply color tint if specified
                            let mut base_color = forest_color;
                            if let Some((r, g, b)) = tile.visual.color_tint {
                                base_color = Color::srgb(
                                    forest_rgb.0 * r,
                                    forest_rgb.1 * g,
                                    forest_rgb.2 * b,
                                );
                            }

                            let material = materials.add(StandardMaterial {
                                base_color,
                                perceptual_roughness: 0.9,
                                ..default()
                            });

                            // Apply rotation if specified
                            let rotation = bevy::prelude::Quat::from_rotation_y(
                                tile.visual.rotation_y_radians(),
                            );
                            let transform = Transform::from_xyz(x as f32, y_pos, y as f32)
                                .with_rotation(rotation);

                            commands.spawn((
                                Mesh3d(mesh),
                                MeshMaterial3d(material),
                                transform,
                                GlobalTransform::default(),
                                Visibility::default(),
                                MapEntity(map.id),
                                TileCoord(pos),
                            ));
                        }
                        world::TerrainType::Grass => {
                            // Grass floor
                            commands.spawn((
                                Mesh3d(floor_mesh.clone()),
                                MeshMaterial3d(grass_material.clone()),
                                Transform::from_xyz(x as f32, 0.0, y as f32),
                                GlobalTransform::default(),
                                Visibility::default(),
                                MapEntity(map.id),
                                TileCoord(pos),
                            ));
                        }
                        _ => {
                            // Spawn regular floor for Ground, Stone, etc.
                            commands.spawn((
                                Mesh3d(floor_mesh.clone()),
                                MeshMaterial3d(floor_material.clone()),
                                Transform::from_xyz(x as f32, 0.0, y as f32),
                                GlobalTransform::default(),
                                Visibility::default(),
                                MapEntity(map.id),
                                TileCoord(pos),
                            ));
                        }
                    }

                    // Spawn wall/door based on wall_type or perimeter
                    if is_perimeter && tile.wall_type == world::WallType::None {
                        // Add perimeter walls using per-tile visual metadata
                        let (width_x, height, width_z) = tile
                            .visual
                            .mesh_dimensions(tile.terrain, world::WallType::Normal);
                        let mesh = get_or_create_mesh(
                            &mut meshes,
                            &mut mesh_cache,
                            width_x,
                            height,
                            width_z,
                        );
                        let y_pos = tile
                            .visual
                            .mesh_y_position(tile.terrain, world::WallType::Normal);

                        // Apply color tint if specified
                        let mut base_color = wall_base_color;
                        if let Some((r, g, b)) = tile.visual.color_tint {
                            base_color = Color::srgb(
                                wall_base_rgb.0 * r,
                                wall_base_rgb.1 * g,
                                wall_base_rgb.2 * b,
                            );
                        }

                        let material = materials.add(StandardMaterial {
                            base_color,
                            perceptual_roughness: 0.5,
                            ..default()
                        });

                        // Apply rotation if specified
                        let rotation =
                            bevy::prelude::Quat::from_rotation_y(tile.visual.rotation_y_radians());
                        let transform =
                            Transform::from_xyz(x as f32, y_pos, y as f32).with_rotation(rotation);

                        commands.spawn((
                            Mesh3d(mesh),
                            MeshMaterial3d(material),
                            transform,
                            GlobalTransform::default(),
                            Visibility::default(),
                            MapEntity(map.id),
                            TileCoord(pos),
                        ));
                    } else {
                        match tile.wall_type {
                            world::WallType::Normal => {
                                // Tint/darken the wall material to match the underlying terrain color
                                // so a Forest Normal wall appears greenish while a Stone Normal wall remains grey.
                                let (tr, tg, tb) = match tile.terrain {
                                    world::TerrainType::Ground => floor_rgb,
                                    world::TerrainType::Grass => grass_rgb,
                                    world::TerrainType::Water => water_rgb,
                                    world::TerrainType::Lava => (0.8_f32, 0.3_f32, 0.2_f32),
                                    world::TerrainType::Swamp => (0.35_f32, 0.3_f32, 0.2_f32),
                                    world::TerrainType::Stone => stone_rgb,
                                    world::TerrainType::Dirt => dirt_rgb,
                                    world::TerrainType::Forest => forest_rgb,
                                    world::TerrainType::Mountain => mountain_rgb,
                                };
                                // Darken a bit to make the wall distinct from the floor
                                let darken = 0.6_f32;

                                // Use per-tile visual metadata for dimensions
                                let (width_x, height, width_z) =
                                    tile.visual.mesh_dimensions(tile.terrain, tile.wall_type);
                                let mesh = get_or_create_mesh(
                                    &mut meshes,
                                    &mut mesh_cache,
                                    width_x,
                                    height,
                                    width_z,
                                );
                                let y_pos =
                                    tile.visual.mesh_y_position(tile.terrain, tile.wall_type);

                                // Apply base terrain tint, then per-tile color tint if specified
                                let mut wall_color =
                                    Color::srgb(tr * darken, tg * darken, tb * darken);
                                if let Some((r, g, b)) = tile.visual.color_tint {
                                    wall_color = Color::srgb(
                                        wall_color.to_srgba().red * r,
                                        wall_color.to_srgba().green * g,
                                        wall_color.to_srgba().blue * b,
                                    );
                                }

                                let tile_wall_material = materials.add(StandardMaterial {
                                    base_color: wall_color,
                                    perceptual_roughness: 0.5,
                                    ..default()
                                });

                                // Apply rotation if specified
                                let rotation = bevy::prelude::Quat::from_rotation_y(
                                    tile.visual.rotation_y_radians(),
                                );
                                let transform = Transform::from_xyz(x as f32, y_pos, y as f32)
                                    .with_rotation(rotation);

                                commands.spawn((
                                    Mesh3d(mesh),
                                    MeshMaterial3d(tile_wall_material),
                                    transform,
                                    GlobalTransform::default(),
                                    Visibility::default(),
                                    MapEntity(map.id),
                                    TileCoord(pos),
                                ));
                            }
                            world::WallType::Door => {
                                // Use per-tile visual metadata for dimensions
                                let (width_x, height, width_z) =
                                    tile.visual.mesh_dimensions(tile.terrain, tile.wall_type);
                                let mesh = get_or_create_mesh(
                                    &mut meshes,
                                    &mut mesh_cache,
                                    width_x,
                                    height,
                                    width_z,
                                );
                                let y_pos =
                                    tile.visual.mesh_y_position(tile.terrain, tile.wall_type);

                                // Apply color tint if specified
                                let mut base_color = door_color;
                                if let Some((r, g, b)) = tile.visual.color_tint {
                                    base_color =
                                        Color::srgb(door_rgb.0 * r, door_rgb.1 * g, door_rgb.2 * b);
                                }

                                let material = materials.add(StandardMaterial {
                                    base_color,
                                    perceptual_roughness: 0.5,
                                    ..default()
                                });

                                // Apply rotation if specified
                                let rotation = bevy::prelude::Quat::from_rotation_y(
                                    tile.visual.rotation_y_radians(),
                                );
                                let transform = Transform::from_xyz(x as f32, y_pos, y as f32)
                                    .with_rotation(rotation);

                                commands.spawn((
                                    Mesh3d(mesh),
                                    MeshMaterial3d(material),
                                    transform,
                                    GlobalTransform::default(),
                                    Visibility::default(),
                                    MapEntity(map.id),
                                    TileCoord(pos),
                                ));
                            }
                            world::WallType::Torch => {
                                // Use per-tile visual metadata for dimensions
                                let (width_x, height, width_z) =
                                    tile.visual.mesh_dimensions(tile.terrain, tile.wall_type);
                                let mesh = get_or_create_mesh(
                                    &mut meshes,
                                    &mut mesh_cache,
                                    width_x,
                                    height,
                                    width_z,
                                );
                                let y_pos =
                                    tile.visual.mesh_y_position(tile.terrain, tile.wall_type);

                                // Apply color tint if specified
                                let mut base_color = wall_base_color;
                                if let Some((r, g, b)) = tile.visual.color_tint {
                                    base_color = Color::srgb(
                                        wall_base_rgb.0 * r,
                                        wall_base_rgb.1 * g,
                                        wall_base_rgb.2 * b,
                                    );
                                }

                                let material = materials.add(StandardMaterial {
                                    base_color,
                                    perceptual_roughness: 0.5,
                                    ..default()
                                });

                                // Apply rotation if specified
                                let rotation = bevy::prelude::Quat::from_rotation_y(
                                    tile.visual.rotation_y_radians(),
                                );
                                let transform = Transform::from_xyz(x as f32, y_pos, y as f32)
                                    .with_rotation(rotation);

                                commands.spawn((
                                    Mesh3d(mesh),
                                    MeshMaterial3d(material),
                                    transform,
                                    GlobalTransform::default(),
                                    Visibility::default(),
                                    MapEntity(map.id),
                                    TileCoord(pos),
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Also spawn lightweight event trigger entities for any map events (so the
        // visual + logic are both represented at load time). We only spawn
        // triggers for event types we convert.
        for (pos, event) in map.events.iter() {
            if let Some(evt_type) = map_event_to_event_type(event) {
                commands.spawn((
                    MapEntity(map.id),
                    EventTrigger {
                        event_type: evt_type,
                        position: *pos,
                    },
                ));
            }
        }

        // Spawn NPC visual markers (Phase 2: NPC Visual Representation)
        let resolved_npcs = map.resolve_npcs(&content.0.npcs);
        let npc_color = Color::srgb(0.0, 1.0, 1.0); // Cyan
        let npc_material = materials.add(StandardMaterial {
            base_color: npc_color,
            perceptual_roughness: 0.5,
            ..default()
        });

        // Vertical plane representing NPC (billboard-like)
        // 1.0 wide, 1.8 tall (human height ~6 feet), 0.1 depth
        let npc_mesh = meshes.add(Cuboid::new(1.0, 1.8, 0.1));

        for resolved_npc in resolved_npcs.iter() {
            let x = resolved_npc.position.x as f32;
            let y = resolved_npc.position.y as f32;

            // Center the NPC marker at y=0.9 (bottom at 0, top at 1.8)
            commands.spawn((
                Mesh3d(npc_mesh.clone()),
                MeshMaterial3d(npc_material.clone()),
                Transform::from_xyz(x, 0.9, y),
                GlobalTransform::default(),
                Visibility::default(),
                MapEntity(map.id),
                TileCoord(resolved_npc.position),
                NpcMarker {
                    npc_id: resolved_npc.npc_id.clone(),
                },
            ));
        }

        debug!(
            "Map spawning complete with {} tiles and {} NPCs",
            map.width * map.height,
            resolved_npcs.len()
        );
    } else {
        warn!("No current map found during spawn_map!");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_skip_marker_spawn_first_run_with_entities() {
        assert!(should_skip_marker_spawn(&None, true));
    }

    #[test]
    fn test_should_skip_marker_spawn_first_run_without_entities() {
        assert!(!should_skip_marker_spawn(&None, false));
    }

    #[test]
    fn test_should_not_skip_when_last_map_some() {
        let some_map: Option<types::MapId> = Some(1u16);
        assert!(!should_skip_marker_spawn(&some_map, true));
    }
}
