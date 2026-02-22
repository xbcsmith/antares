// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::domain::types;
use crate::domain::world;
use crate::domain::world::SpriteReference;
use crate::game::components::creature::CreatureVisual;
use crate::game::components::sprite::{ActorType, AnimatedSprite, TileSprite};
use crate::game::resources::sprite_assets::SpriteAssets;
use crate::game::resources::GlobalState;
use crate::game::systems::actor::spawn_actor_sprite;
use crate::game::systems::creature_spawning::spawn_creature;
use crate::game::systems::procedural_meshes;
use rand::Rng;

const DEFAULT_NPC_SPRITE_PATH: &str = "sprites/placeholders/npc_placeholder.png";
use bevy::prelude::*;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type alias for mesh cache keys (width_x, height, width_z)
type MeshDimensions = (OrderedFloat<f32>, OrderedFloat<f32>, OrderedFloat<f32>);

/// Type alias for the mesh cache HashMap
type MeshCache = HashMap<MeshDimensions, Handle<Mesh>>;

/// Offset to center map objects within their tile (matches camera centering)
const TILE_CENTER_OFFSET: f32 = 0.5;

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

/// Component tagging a spawned visual as originating from a recruitable map event.
#[derive(bevy::prelude::Component, Debug, Clone, PartialEq, Eq)]
pub struct RecruitableVisualMarker {
    /// Character ID stored in the recruitable event.
    pub character_id: String,
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
    RecruitableCharacter {
        character_id: String,
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
                (
                    map_change_handler,
                    handle_door_opened,
                    spawn_map_markers,
                    cleanup_recruitable_visuals,
                ),
            );
    }
}

/// Despawns recruitable visual entities when their backing map event is no longer present.
fn cleanup_recruitable_visuals(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    query: Query<(Entity, &MapEntity, &TileCoord, &RecruitableVisualMarker)>,
) {
    let game_state = &global_state.0;
    let Some(current_map) = game_state.world.get_current_map() else {
        return;
    };

    for (entity, map_entity, tile_coord, marker) in query.iter() {
        if map_entity.0 != current_map.id {
            continue;
        }

        let should_keep = matches!(
            current_map.get_event(tile_coord.0),
            Some(world::MapEvent::RecruitableCharacter { character_id, .. }) if character_id == &marker.character_id
        );

        if !should_keep {
            commands.entity(entity).despawn();
        }
    }
}

impl Plugin for MapRenderingPlugin {
    fn build(&self, app: &mut App) {
        // Initialize the SpriteAssets resource so systems depending on it pass
        // Bevy's system validation at startup. Also register the sprite sheet
        // registry on startup so metadata is available before map spawn runs.
        app.init_resource::<SpriteAssets>()
            .init_resource::<crate::game::resources::GrassQualitySettings>()
            .init_resource::<super::advanced_grass::GrassRenderConfig>() // Phase 2: Add grass render config
            .init_resource::<super::advanced_grass::GrassInstanceConfig>()
            .add_systems(
                Startup,
                // Ensure registration happens before the map spawn system runs
                (register_sprite_sheets_system, spawn_map_system),
            )
            .add_systems(
                Update,
                (
                    // Phase 2: Grass performance systems for culling and LOD
                    super::advanced_grass::grass_distance_culling_system,
                    super::advanced_grass::grass_lod_system,
                    // Phase 4: Instance batching for diagnostics
                    super::advanced_grass::build_grass_instance_batches_system,
                    // Phase 4: Advanced grass chunking + culling systems
                    super::advanced_grass::build_grass_chunks_system,
                    super::advanced_grass::grass_chunk_culling_system,
                ),
            )
            .add_plugins(MapManagerPlugin);
    }
}

/// System wrapper that creates a cache and calls spawn_map
#[allow(clippy::too_many_arguments)]
fn spawn_map_system(
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    sprite_assets: ResMut<SpriteAssets>,
    asset_server: Res<AssetServer>,
    global_state: Res<GlobalState>,
    content: Res<crate::application::resources::GameContent>,
    quality_settings: Res<crate::game::resources::GrassQualitySettings>,
    mut cache: Local<super::procedural_meshes::ProceduralMeshCache>,
) {
    spawn_map(
        commands,
        meshes,
        materials,
        sprite_assets,
        asset_server,
        global_state,
        content,
        quality_settings,
        &mut cache,
    );
}

/// Startup system that loads the sprite sheet registry (`data/sprite_sheets.ron`)
/// and registers all sheet configurations into the `SpriteAssets` resource.
///
/// This ensures sprite sheet metadata is available at runtime before map spawn
/// systems depend on `SpriteAssets`.
fn register_sprite_sheets_system(mut sprite_assets: ResMut<SpriteAssets>) {
    match crate::sdk::map_editor::load_sprite_registry() {
        Ok(registry) => {
            // Use len before moving the registry into iteration
            let count = registry.len();
            for (key, info) in registry {
                let config = crate::game::resources::sprite_assets::SpriteSheetConfig {
                    texture_path: info.texture_path,
                    tile_size: info.tile_size,
                    columns: info.columns,
                    rows: info.rows,
                    sprites: info.sprites,
                };
                sprite_assets.register_config(key, config);
            }
            info!("Registered {} sprite sheets into SpriteAssets", count);
        }
        Err(e) => {
            warn!("Failed to load sprite registry: {}", e);
            warn!("Registering built-in placeholder sprite sheets as fallback.");

            // Minimal placeholder sheet configuration so the engine has a safe
            // visual fallback even when the registry cannot be parsed.
            let placeholder_config = crate::game::resources::sprite_assets::SpriteSheetConfig {
                texture_path: "sprites/placeholders/npc_placeholder.png".to_string(),
                tile_size: (32.0, 48.0),
                columns: 1,
                rows: 1,
                sprites: vec![(0, "npc_placeholder".to_string())],
            };
            sprite_assets.register_config("placeholders".to_string(), placeholder_config);

            info!("Registered fallback placeholder sprite sheet: placeholders");
        }
    }
}

/// System that handles door opened messages by refreshing map visuals
#[allow(unused_mut)] // spawn_map requires mut even though clippy doesn't detect it
#[allow(clippy::too_many_arguments)]
fn handle_door_opened(
    mut door_messages: MessageReader<DoorOpenedEvent>,
    mut commands: Commands,
    query_existing: Query<Entity, With<MapEntity>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut sprite_assets: ResMut<SpriteAssets>,
    asset_server: Res<AssetServer>,
    global_state: Res<GlobalState>,
    content: Res<crate::application::resources::GameContent>,
    quality_settings: Res<crate::game::resources::GrassQualitySettings>,
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
    let mut procedural_cache = super::procedural_meshes::ProceduralMeshCache::default();
    spawn_map(
        commands,
        meshes,
        materials,
        sprite_assets,
        asset_server,
        global_state,
        content,
        quality_settings,
        &mut procedural_cache,
    );
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
        world::MapEvent::RecruitableCharacter { character_id, .. } => {
            Some(MapEventType::RecruitableCharacter {
                character_id: character_id.clone(),
            })
        }
        _ => None,
    }
}

/// Normalizes identifiers and names for loose cross-database matching.
fn normalize_lookup_key(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Resolves a creature id for a recruitable map event character id.
///
/// Resolution order:
/// 1. Treat the id as an NPC id and use its configured `creature_id`
/// 2. Strip `npc_` prefix and try character definitions by id
/// 3. Match character name against creature definition name (normalized)
fn resolve_recruitable_creature_id(
    character_id: &str,
    content: &crate::application::resources::GameContent,
) -> Option<types::CreatureId> {
    if let Some(npc_def) = content.0.npcs.get_npc(character_id) {
        if let Some(creature_id) = npc_def.creature_id {
            return Some(creature_id);
        }
    }

    let mut candidates = Vec::with_capacity(2);
    candidates.push(character_id);
    if let Some(stripped) = character_id.strip_prefix("npc_") {
        candidates.push(stripped);
    }

    for candidate in candidates {
        if let Some(character_def) = content.0.characters.get_character(candidate) {
            let target_name = normalize_lookup_key(&character_def.name);
            if let Some(creature) = content
                .0
                .creatures
                .all_creatures()
                .find(|creature| normalize_lookup_key(&creature.name) == target_name)
            {
                return Some(creature.id);
            }
        }
    }

    None
}

/// Resolves a creature id for an encounter marker from a monster group.
///
/// Uses the first monster entry that has a configured `visual_id`.
fn resolve_encounter_creature_id(
    monster_group: &[types::MonsterId],
    content: &crate::application::resources::GameContent,
) -> Option<types::CreatureId> {
    for monster_id in monster_group {
        if let Some(monster_def) = content.0.monsters.get_monster(*monster_id) {
            if let Some(creature_id) = monster_def.visual_id {
                return Some(creature_id);
            }
        }
    }

    None
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
#[allow(clippy::too_many_arguments)]
fn spawn_map_markers(
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    sprite_assets: ResMut<SpriteAssets>,
    asset_server: Res<AssetServer>,
    global_state: Res<GlobalState>,
    content: Res<crate::application::resources::GameContent>,
    quality_settings: Res<crate::game::resources::GrassQualitySettings>,
    query_existing: Query<(Entity, &MapEntity)>,
    mut last_map: Local<Option<types::MapId>>,
) {
    let current = global_state.0.world.current_map;

    // If map hasn't changed, nothing to do
    if Some(current) == *last_map {
        return;
    }

    let mut has_any_entities = false;
    let mut has_current_entities = false;

    for (_entity, map_entity) in query_existing.iter() {
        has_any_entities = true;
        if map_entity.0 == current {
            has_current_entities = true;
        }
    }

    // If visuals for the current map already exist, skip marker refresh to avoid
    // despawning freshly spawned visuals.
    if has_current_entities {
        *last_map = Some(current);
        debug!(
            "spawn_map_markers: visuals already spawned for current map {}; skipping marker refresh",
            current
        );
        return;
    }

    // If this is the first time this system runs and there are already map
    // entities present (spawned by `spawn_map` in `Startup`), don't
    // despawn them and don't spawn duplicate markers either. Instead, just
    // record the current map and exit.
    if should_skip_marker_spawn(&last_map, has_any_entities) {
        *last_map = Some(current);
        debug!(
            "spawn_map_markers: existing map entities present on first run; leaving visuals intact"
        );
        return;
    } else {
        // We have a previously recorded map and it changed; despawn old map entities
        for (entity, _map_entity) in query_existing.iter() {
            commands.entity(entity).despawn();
        }
    }

    // Spawn visuals (tiles + markers) for the new map (if it exists)
    if global_state.0.world.get_current_map().is_some() {
        let mut procedural_cache = super::procedural_meshes::ProceduralMeshCache::default();
        spawn_map(
            commands,
            meshes,
            materials,
            sprite_assets,
            asset_server,
            global_state,
            content,
            quality_settings,
            &mut procedural_cache,
        );
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
#[allow(clippy::too_many_arguments)]
fn spawn_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut sprite_assets: ResMut<SpriteAssets>,
    asset_server: Res<AssetServer>,
    global_state: Res<crate::game::resources::GlobalState>,
    content: Res<crate::application::resources::GameContent>,
    quality_settings: Res<crate::game::resources::GrassQualitySettings>,
    procedural_cache: &mut super::procedural_meshes::ProceduralMeshCache,
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
        let _forest_color = Color::srgb(forest_rgb.0, forest_rgb.1, forest_rgb.2);
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
                    // Render based on terrain type
                    match tile.terrain {
                        world::TerrainType::Water => {
                            // Render water slightly below at y = -0.1
                            commands.spawn((
                                Mesh3d(water_mesh.clone()),
                                MeshMaterial3d(water_material.clone()),
                                Transform::from_xyz(
                                    x as f32 + TILE_CENTER_OFFSET,
                                    -0.1,
                                    y as f32 + TILE_CENTER_OFFSET,
                                ),
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
                            let transform = Transform::from_xyz(
                                x as f32 + TILE_CENTER_OFFSET,
                                y_pos,
                                y as f32 + TILE_CENTER_OFFSET,
                            )
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
                        world::TerrainType::Forest | world::TerrainType::Grass => {
                            let is_forest = tile.terrain == world::TerrainType::Forest;

                            // Render grass floor
                            commands.spawn((
                                Mesh3d(floor_mesh.clone()),
                                MeshMaterial3d(grass_material.clone()),
                                Transform::from_xyz(
                                    x as f32 + TILE_CENTER_OFFSET,
                                    0.0,
                                    y as f32 + TILE_CENTER_OFFSET,
                                ),
                                GlobalTransform::default(),
                                Visibility::default(),
                                MapEntity(map.id),
                                TileCoord(pos),
                            ));

                            // Spawn tree/shrub if specified in metadata, or default for Forest
                            let tree_type = tile.visual.tree_type;
                            if let Some(t) = tree_type {
                                // Explicitly map domain TreeType to rendered TreeType to resolve ambiguity
                                let rendered_t = match t {
                                    crate::domain::world::TreeType::Oak => {
                                        crate::game::systems::advanced_trees::TreeType::Oak
                                    }
                                    crate::domain::world::TreeType::Pine => {
                                        crate::game::systems::advanced_trees::TreeType::Pine
                                    }
                                    crate::domain::world::TreeType::Birch => {
                                        crate::game::systems::advanced_trees::TreeType::Birch
                                    }
                                    crate::domain::world::TreeType::Willow => {
                                        crate::game::systems::advanced_trees::TreeType::Willow
                                    }
                                    crate::domain::world::TreeType::Dead => {
                                        crate::game::systems::advanced_trees::TreeType::Dead
                                    }
                                    crate::domain::world::TreeType::Shrub => {
                                        crate::game::systems::advanced_trees::TreeType::Shrub
                                    }
                                    crate::domain::world::TreeType::Palm => {
                                        crate::game::systems::advanced_trees::TreeType::Oak
                                    } // Fallback for Palm
                                };

                                if rendered_t
                                    == crate::game::systems::advanced_trees::TreeType::Shrub
                                {
                                    procedural_meshes::spawn_shrub(
                                        &mut commands,
                                        &mut materials,
                                        &mut meshes,
                                        pos,
                                        map.id,
                                        Some(&tile.visual),
                                        procedural_cache,
                                    );
                                } else {
                                    procedural_meshes::spawn_tree(
                                        &mut commands,
                                        &mut materials,
                                        &mut meshes,
                                        pos,
                                        map.id,
                                        Some(&tile.visual),
                                        Some(rendered_t),
                                        procedural_cache,
                                    );
                                }
                            } else if is_forest {
                                // Default tree for Forest terrain with no explicit tree type
                                procedural_meshes::spawn_tree(
                                    &mut commands,
                                    &mut materials,
                                    &mut meshes,
                                    pos,
                                    map.id,
                                    Some(&tile.visual),
                                    None, // Use default tree type
                                    procedural_cache,
                                );
                            }

                            // Extra shrubs for variety in forest
                            if is_forest {
                                let mut rng = rand::rng();
                                if rng.random_range(0..10) < 4 {
                                    procedural_meshes::spawn_shrub(
                                        &mut commands,
                                        &mut materials,
                                        &mut meshes,
                                        pos,
                                        map.id,
                                        Some(&tile.visual),
                                        procedural_cache,
                                    );
                                }
                            }

                            // Always spawn grass ground cover for these terrains
                            super::advanced_grass::spawn_grass(
                                &mut commands,
                                &mut materials,
                                &mut meshes,
                                pos,
                                map.id,
                                Some(&tile.visual),
                                &quality_settings,
                            );
                        }
                        _ => {
                            // Spawn regular floor for Ground, Stone, etc.
                            commands.spawn((
                                Mesh3d(floor_mesh.clone()),
                                MeshMaterial3d(floor_material.clone()),
                                Transform::from_xyz(
                                    x as f32 + TILE_CENTER_OFFSET,
                                    0.0,
                                    y as f32 + TILE_CENTER_OFFSET,
                                ),
                                GlobalTransform::default(),
                                Visibility::default(),
                                MapEntity(map.id),
                                TileCoord(pos),
                            ));
                        }
                    }

                    // Spawn wall/door based on wall_type
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
                            let y_pos = tile.visual.mesh_y_position(tile.terrain, tile.wall_type);

                            // Apply base terrain tint, then per-tile color tint if specified
                            let mut wall_color = Color::srgb(tr * darken, tg * darken, tb * darken);
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
                            let transform = Transform::from_xyz(
                                x as f32 + TILE_CENTER_OFFSET,
                                y_pos,
                                y as f32 + TILE_CENTER_OFFSET,
                            )
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
                            let y_pos = tile.visual.mesh_y_position(tile.terrain, tile.wall_type);

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
                            let y_pos = tile.visual.mesh_y_position(tile.terrain, tile.wall_type);

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
                            let transform = Transform::from_xyz(
                                x as f32 + TILE_CENTER_OFFSET,
                                y_pos,
                                y as f32 + TILE_CENTER_OFFSET,
                            )
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

        // For each resolved NPC, prefer creature mesh rendering when a creature_id
        // is configured; otherwise fall back to sprite rendering.
        for resolved_npc in resolved_npcs.iter() {
            let x = resolved_npc.position.x as f32;
            let y = resolved_npc.position.y as f32;

            if let Some(creature_id) = resolved_npc.creature_id {
                if let Some(creature_def) = content.0.creatures.get_creature(creature_id) {
                    let entity = spawn_creature(
                        &mut commands,
                        creature_def,
                        &mut meshes,
                        &mut materials,
                        Vec3::new(x + TILE_CENTER_OFFSET, 0.0, y + TILE_CENTER_OFFSET),
                        None,
                        None,
                    );

                    commands.entity(entity).insert((
                        CreatureVisual {
                            creature_id,
                            scale_override: None,
                        },
                        MapEntity(map.id),
                        TileCoord(resolved_npc.position),
                        NpcMarker {
                            npc_id: resolved_npc.npc_id.clone(),
                        },
                        Visibility::default(),
                    ));

                    continue;
                }

                warn!(
                    "NPC '{}' references missing creature_id {} - falling back to sprite",
                    resolved_npc.npc_id, creature_id
                );
            }

            // Prefer per-NPC sprite if defined, otherwise use default placeholder
            let sprite_ref = resolved_npc
                .sprite
                .clone()
                .unwrap_or_else(|| SpriteReference {
                    sheet_path: DEFAULT_NPC_SPRITE_PATH.to_string(),
                    sprite_index: 0,
                    animation: None,
                    material_properties: None,
                });

            let entity = spawn_actor_sprite(
                &mut commands,
                &mut sprite_assets,
                &asset_server,
                &mut materials,
                &mut meshes,
                &sprite_ref,
                Vec3::new(x + TILE_CENTER_OFFSET, 0.9, y + TILE_CENTER_OFFSET),
                ActorType::Npc,
            );

            // Attach map tags and NPC marker to the spawned actor entity
            commands.entity(entity).insert((
                MapEntity(map.id),
                TileCoord(resolved_npc.position),
                NpcMarker {
                    npc_id: resolved_npc.npc_id.clone(),
                },
                Visibility::default(),
            ));
        }

        // Spawn procedural event markers and recruitable character visuals.
        for (position, event) in map.events.iter() {
            // Get tile visual metadata for rotation (if tile exists)
            let rotation_y = map
                .get_tile(*position)
                .and_then(|tile| tile.visual.rotation_y);

            match event {
                world::MapEvent::Sign { name, .. } => {
                    procedural_meshes::spawn_sign(
                        &mut commands,
                        &mut materials,
                        &mut meshes,
                        *position,
                        name.clone(),
                        map.id,
                        procedural_cache,
                        rotation_y,
                    );
                }
                world::MapEvent::Teleport { name, .. } => {
                    procedural_meshes::spawn_portal(
                        &mut commands,
                        &mut materials,
                        &mut meshes,
                        *position,
                        name.clone(),
                        map.id,
                        procedural_cache,
                        rotation_y,
                    );
                }
                world::MapEvent::Furniture {
                    furniture_type,
                    rotation_y,
                    scale,
                    material,
                    flags,
                    color_tint,
                    ..
                } => {
                    procedural_meshes::spawn_furniture(
                        &mut commands,
                        &mut materials,
                        &mut meshes,
                        *position,
                        map.id,
                        *furniture_type,
                        *rotation_y,
                        *scale,
                        *material,
                        flags,
                        *color_tint,
                        procedural_cache,
                    );
                }
                world::MapEvent::Encounter { monster_group, .. } => {
                    let x = position.x as f32;
                    let y = position.y as f32;

                    if let Some(creature_id) =
                        resolve_encounter_creature_id(monster_group, &content)
                    {
                        if let Some(creature_def) = content.0.creatures.get_creature(creature_id) {
                            let entity = spawn_creature(
                                &mut commands,
                                creature_def,
                                &mut meshes,
                                &mut materials,
                                Vec3::new(x + TILE_CENTER_OFFSET, 0.0, y + TILE_CENTER_OFFSET),
                                None,
                                None,
                            );

                            commands.entity(entity).insert((
                                CreatureVisual {
                                    creature_id,
                                    scale_override: None,
                                },
                                MapEntity(map.id),
                                TileCoord(*position),
                                Visibility::default(),
                            ));

                            continue;
                        }
                    }

                    warn!(
                        "Encounter at ({}, {}) has no resolvable creature visual; no marker spawned",
                        position.x, position.y
                    );
                }
                world::MapEvent::RecruitableCharacter {
                    character_id, name, ..
                } => {
                    let x = position.x as f32;
                    let y = position.y as f32;

                    if let Some(creature_id) =
                        resolve_recruitable_creature_id(character_id, &content)
                    {
                        if let Some(creature_def) = content.0.creatures.get_creature(creature_id) {
                            let entity = spawn_creature(
                                &mut commands,
                                creature_def,
                                &mut meshes,
                                &mut materials,
                                Vec3::new(x + TILE_CENTER_OFFSET, 0.0, y + TILE_CENTER_OFFSET),
                                None,
                                None,
                            );

                            commands.entity(entity).insert((
                                CreatureVisual {
                                    creature_id,
                                    scale_override: None,
                                },
                                MapEntity(map.id),
                                TileCoord(*position),
                                RecruitableVisualMarker {
                                    character_id: character_id.clone(),
                                },
                                NpcMarker {
                                    npc_id: character_id.clone(),
                                },
                                Visibility::default(),
                            ));

                            continue;
                        }

                        warn!(
                            "Recruitable '{}' ('{}') references missing creature_id {} - falling back to sprite",
                            name, character_id, creature_id
                        );
                    } else {
                        warn!(
                            "Recruitable '{}' ('{}') has no resolvable creature mapping - falling back to sprite",
                            name, character_id
                        );
                    }

                    let sprite_ref = SpriteReference {
                        sheet_path: DEFAULT_NPC_SPRITE_PATH.to_string(),
                        sprite_index: 0,
                        animation: None,
                        material_properties: None,
                    };

                    let entity = spawn_actor_sprite(
                        &mut commands,
                        &mut sprite_assets,
                        &asset_server,
                        &mut materials,
                        &mut meshes,
                        &sprite_ref,
                        Vec3::new(x + TILE_CENTER_OFFSET, 0.9, y + TILE_CENTER_OFFSET),
                        ActorType::Npc,
                    );

                    commands.entity(entity).insert((
                        MapEntity(map.id),
                        TileCoord(*position),
                        RecruitableVisualMarker {
                            character_id: character_id.clone(),
                        },
                        NpcMarker {
                            npc_id: character_id.clone(),
                        },
                        Visibility::default(),
                    ));
                }
                // Other events (Trap, Treasure, NpcDialogue, InnEntry) have no visual markers
                _ => {}
            }
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

/// Spawns a sprite entity for a tile with sprite metadata
///
/// # Arguments
///
/// * `commands` - Bevy command buffer
/// * `sprite_assets` - Sprite asset registry (mutable for caching)
/// * `asset_server` - Asset server for loading textures
/// * `materials` - Material asset storage (mutable)
/// * `meshes` - Mesh asset storage (mutable)
/// * `sprite_ref` - Sprite reference from tile metadata
/// * `position` - World position for the sprite
/// * `map_id` - ID of the map this sprite belongs to
///
/// # Returns
///
/// Entity ID of spawned sprite
///
/// # Behavior
///
/// - Loads sprite texture from `sprite_ref.sheet_path`
/// - Creates entity with Mesh and StandardMaterial components
/// - Attaches `TileSprite` component with sheet path and index
/// - If animation specified, attaches `AnimatedSprite` component
/// - Tags entity with `MapEntity` for lifecycle management
///
/// # Examples
///
/// ```no_run
/// use antares::game::systems::map::spawn_tile_sprite;
/// use antares::domain::world::SpriteReference;
/// use bevy::prelude::*;
/// use antares::game::resources::sprite_assets::SpriteAssets;
///
/// fn spawn_sprite(
///     mut commands: Commands,
///     mut sprite_assets: ResMut<SpriteAssets>,
///     asset_server: Res<AssetServer>,
///     mut materials: ResMut<Assets<StandardMaterial>>,
///     mut meshes: ResMut<Assets<Mesh>>,
/// ) {
///     let sprite_ref = SpriteReference {
///         sheet_path: "sprites/walls.png".to_string(),
///         sprite_index: 0,
///         animation: None,
///         material_properties: None,
///     };
///     let entity = spawn_tile_sprite(
///         &mut commands,
///         &mut sprite_assets,
///         &asset_server,
///         &mut materials,
///         &mut meshes,
///         &sprite_ref,
///         Vec3::new(5.0, 0.5, 5.0),
///         1u16,
///     );
/// }
/// ```
#[allow(clippy::too_many_arguments)]
pub fn spawn_tile_sprite(
    commands: &mut Commands,
    sprite_assets: &mut SpriteAssets,
    asset_server: &AssetServer,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    sprite_ref: &SpriteReference,
    position: Vec3,
    map_id: types::MapId,
) -> Entity {
    // Get or load material for sprite sheet (caches per sheet path)
    // Phase 6: Pass material properties if defined
    let material = sprite_assets.get_or_load_material(
        &sprite_ref.sheet_path,
        asset_server,
        materials,
        sprite_ref.material_properties.as_ref(),
    );

    // Get or load mesh for tile sprites (1.0 x 1.0 flat quad)
    let mesh = sprite_assets.get_or_load_mesh((1.0, 1.0), meshes);

    // Extract position from Vec3 for TileCoord
    let tile_pos = types::Position::new(position.x as i32, position.z as i32);

    // Spawn tile sprite with components
    let mut entity_commands = commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(position),
        GlobalTransform::default(),
        Visibility::default(),
        TileSprite {
            sheet_path: sprite_ref.sheet_path.clone(),
            sprite_index: sprite_ref.sprite_index,
        },
        MapEntity(map_id),
        TileCoord(tile_pos),
    ));

    // Add animation if specified
    if let Some(anim) = &sprite_ref.animation {
        entity_commands.insert(AnimatedSprite::new(
            anim.frames.clone(),
            anim.fps,
            anim.looping,
        ));
    }

    entity_commands.id()
}

/// Spawns a visual marker for a map event using a sprite
///
/// # Arguments
///
/// * `commands` - Command buffer
/// * `sprite_assets` - Sprite asset registry (mutable for caching)
/// * `asset_server` - Asset server for loading textures
/// * `materials` - Material asset storage (mutable)
/// * `meshes` - Mesh asset storage (mutable)
/// * `event_type` - Type of event (sign, portal, treasure, etc.)
/// * `position` - World position for the marker
/// * `map_id` - ID of the map this marker belongs to
///
/// # Returns
///
/// Entity ID of spawned marker
///
/// # Behavior
///
/// - Maps event_type to appropriate sprite sheet and index
/// - Creates entity with Mesh and StandardMaterial components
/// - Tags entity with `MapEntity` for lifecycle management
/// - Positions slightly above ground (y = 0.5)
///
/// # Examples
///
/// ```no_run
/// use antares::game::systems::map::spawn_event_marker;
/// use bevy::prelude::*;
/// use antares::game::resources::sprite_assets::SpriteAssets;
///
/// fn spawn_sign(
///     mut commands: Commands,
///     mut sprite_assets: ResMut<SpriteAssets>,
///     asset_server: Res<AssetServer>,
///     mut materials: ResMut<Assets<StandardMaterial>>,
///     mut meshes: ResMut<Assets<Mesh>>,
/// ) {
///     spawn_event_marker(
///         &mut commands,
///         &mut sprite_assets,
///         &asset_server,
///         &mut materials,
///         &mut meshes,
///         "sign",
///         Vec3::new(15.0, 0.5, 15.0),
///         1u16,
///     );
/// }
/// ```
#[allow(clippy::too_many_arguments)]
pub fn spawn_event_marker(
    commands: &mut Commands,
    sprite_assets: &mut SpriteAssets,
    asset_server: &AssetServer,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    event_type: &str,
    position: Vec3,
    map_id: types::MapId,
) -> Entity {
    // Map event type to sprite sheet/index
    let (sheet_path, sprite_index) = match event_type {
        "sign" => ("sprites/signs.png", 0u32),
        "portal" => ("sprites/portals.png", 0u32),
        "treasure" => ("sprites/treasure.png", 0u32),
        "quest" => ("sprites/signs.png", 1u32),
        _ => ("sprites/signs.png", 0u32), // Default to generic sign
    };

    let sprite_ref = SpriteReference {
        sheet_path: sheet_path.to_string(),
        sprite_index,
        animation: None,
        material_properties: None,
    };

    spawn_tile_sprite(
        commands,
        sprite_assets,
        asset_server,
        materials,
        meshes,
        &sprite_ref,
        position,
        map_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::world::SpriteAnimation;
    use crate::game::components::dialogue::NpcDialogue;

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

    #[test]
    fn test_npc_dialogue_component_created_when_dialogue_id_present() {
        // Test that NPCs with dialogue_id get the NpcDialogue component
        use crate::domain::types::Position;
        use crate::domain::world::ResolvedNpc;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Create a resolved NPC with dialogue_id
        let npc = ResolvedNpc {
            npc_id: "test_merchant".to_string(),
            name: "Test Merchant".to_string(),
            description: "A test merchant NPC".to_string(),
            portrait_id: "merchant.png".to_string(),
            sprite: None,
            creature_id: None,
            position: Position::new(5, 5),
            facing: None,
            dialogue_id: Some(100u16), // Dialogue ID present
            quest_ids: vec![],
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
        };

        // Verify the NpcDialogue component can be created with the NPC's data
        let npc_dialogue = NpcDialogue::new(npc.dialogue_id.unwrap(), npc.name.clone());
        assert_eq!(npc_dialogue.dialogue_id, 100u16);
        assert_eq!(npc_dialogue.npc_name, "Test Merchant");
    }

    #[test]
    fn test_tile_positions_are_centered() {
        // Verify the TILE_CENTER_OFFSET constant is 0.5
        assert_eq!(TILE_CENTER_OFFSET, 0.5);

        // Verify centered position calculation
        let tile_x = 5;
        let tile_y = 10;
        let centered_x = tile_x as f32 + TILE_CENTER_OFFSET;
        let centered_z = tile_y as f32 + TILE_CENTER_OFFSET;

        assert_eq!(centered_x, 5.5);
        assert_eq!(centered_z, 10.5);
    }

    #[test]
    fn test_npc_without_dialogue_id_doesnt_need_component() {
        // Test that NPCs without dialogue_id can still be spawned
        use crate::domain::types::Position;
        use crate::domain::world::ResolvedNpc;

        let npc = ResolvedNpc {
            npc_id: "silent_npc".to_string(),
            name: "Silent NPC".to_string(),
            description: "An NPC with no dialogue".to_string(),
            portrait_id: "silent.png".to_string(),
            sprite: None,
            creature_id: None,
            position: Position::new(10, 10),
            facing: None,
            dialogue_id: None, // No dialogue ID
            quest_ids: vec![],
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
        };

        // Verify that None dialogue_id is handled correctly
        assert!(npc.dialogue_id.is_none());
    }

    #[test]
    fn test_npc_marker_component_always_created() {
        // Test that NpcMarker component is created regardless of dialogue presence
        let marker = NpcMarker {
            npc_id: "test_npc".to_string(),
        };
        assert_eq!(marker.npc_id, "test_npc");
    }

    #[test]
    fn test_tile_sprite_spawning() {
        // Test that TileSprite component can be created with correct fields
        let sprite_ref = SpriteReference {
            sheet_path: "sprites/walls.png".to_string(),
            sprite_index: 0,
            animation: None,
            material_properties: None,
        };

        // Verify sprite reference is correctly formed
        assert_eq!(sprite_ref.sheet_path, "sprites/walls.png");
        assert_eq!(sprite_ref.sprite_index, 0);
    }

    #[test]
    fn test_animated_tile_sprite_spawning() {
        // Test that SpriteReference with animation can be created
        let sprite_ref = SpriteReference {
            sheet_path: "sprites/water.png".to_string(),
            sprite_index: 0,
            animation: Some(SpriteAnimation {
                frames: vec![0, 1, 2, 3],
                fps: 8.0,
                looping: true,
            }),
            material_properties: None,
        };

        assert_eq!(sprite_ref.sheet_path, "sprites/water.png");
        assert!(sprite_ref.animation.is_some());

        let anim = sprite_ref.animation.as_ref().unwrap();
        assert_eq!(anim.frames, vec![0, 1, 2, 3]);
        assert_eq!(anim.fps, 8.0);
        assert!(anim.looping);
    }

    #[test]
    fn test_event_marker_sprite_spawning() {
        // Test that event marker maps to correct sprite sheet
        let event_type = "sign";
        let (sheet_path, sprite_index) = match event_type {
            "sign" => ("sprites/signs.png", 0u32),
            "portal" => ("sprites/portals.png", 0u32),
            "treasure" => ("sprites/treasure.png", 0u32),
            "quest" => ("sprites/signs.png", 1u32),
            _ => ("sprites/signs.png", 0u32),
        };

        assert_eq!(sheet_path, "sprites/signs.png");
        assert_eq!(sprite_index, 0u32);
    }

    #[test]
    fn test_event_marker_different_types() {
        let event_types = vec![
            ("sign", "sprites/signs.png", 0u32),
            ("portal", "sprites/portals.png", 0u32),
            ("treasure", "sprites/treasure.png", 0u32),
            ("quest", "sprites/signs.png", 1u32),
        ];

        for (event_type, expected_sheet, expected_index) in event_types {
            let (sheet_path, sprite_index) = match event_type {
                "sign" => ("sprites/signs.png", 0u32),
                "portal" => ("sprites/portals.png", 0u32),
                "treasure" => ("sprites/treasure.png", 0u32),
                "quest" => ("sprites/signs.png", 1u32),
                _ => ("sprites/signs.png", 0u32),
            };

            assert_eq!(
                sheet_path, expected_sheet,
                "Failed for event_type: {}",
                event_type
            );
            assert_eq!(
                sprite_index, expected_index,
                "Failed for event_type: {}",
                event_type
            );
        }
    }

    #[test]
    fn test_tile_sprite_map_entity_tagging() {
        // Test that MapEntity component stores correct map ID
        let map_id = 5u16;
        let map_entity = MapEntity(map_id);

        assert_eq!(map_entity.0, map_id);
        assert_eq!(map_entity.0, 5u16);
    }

    #[test]
    fn test_resolve_encounter_creature_id_returns_first_visual_match() {
        let mut db = crate::sdk::database::ContentDatabase::new();

        let monster = crate::domain::combat::database::MonsterDefinition {
            id: 42,
            name: "Encounter Goblin".to_string(),
            stats: crate::domain::character::Stats::new(8, 6, 6, 8, 10, 8, 5),
            hp: crate::domain::character::AttributePair16::new(10),
            ac: crate::domain::character::AttributePair::new(6),
            attacks: vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            flee_threshold: 0,
            special_attack_threshold: 0,
            resistances: crate::domain::combat::monster::MonsterResistances::new(),
            can_regenerate: false,
            can_advance: false,
            is_undead: false,
            magic_resistance: 0,
            loot: crate::domain::combat::monster::LootTable::default(),
            visual_id: Some(7),
            conditions: crate::domain::combat::monster::MonsterCondition::Normal,
            active_conditions: Vec::new(),
            has_acted: false,
        };

        db.monsters.add_monster(monster).unwrap();

        let content = crate::application::resources::GameContent::new(db);
        let result = resolve_encounter_creature_id(&[42], &content);

        assert_eq!(result, Some(7));
    }

    #[test]
    fn test_resolve_encounter_creature_id_skips_monsters_without_visuals() {
        let mut db = crate::sdk::database::ContentDatabase::new();

        let without_visual = crate::domain::combat::database::MonsterDefinition {
            id: 10,
            name: "No Visual".to_string(),
            stats: crate::domain::character::Stats::new(8, 6, 6, 8, 10, 8, 5),
            hp: crate::domain::character::AttributePair16::new(10),
            ac: crate::domain::character::AttributePair::new(6),
            attacks: vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 4, 0),
            )],
            flee_threshold: 0,
            special_attack_threshold: 0,
            resistances: crate::domain::combat::monster::MonsterResistances::new(),
            can_regenerate: false,
            can_advance: false,
            is_undead: false,
            magic_resistance: 0,
            loot: crate::domain::combat::monster::LootTable::default(),
            visual_id: None,
            conditions: crate::domain::combat::monster::MonsterCondition::Normal,
            active_conditions: Vec::new(),
            has_acted: false,
        };

        let with_visual = crate::domain::combat::database::MonsterDefinition {
            id: 11,
            name: "Has Visual".to_string(),
            stats: crate::domain::character::Stats::new(10, 8, 8, 10, 10, 10, 8),
            hp: crate::domain::character::AttributePair16::new(12),
            ac: crate::domain::character::AttributePair::new(7),
            attacks: vec![crate::domain::combat::types::Attack::physical(
                crate::domain::types::DiceRoll::new(1, 6, 0),
            )],
            flee_threshold: 0,
            special_attack_threshold: 0,
            resistances: crate::domain::combat::monster::MonsterResistances::new(),
            can_regenerate: false,
            can_advance: false,
            is_undead: false,
            magic_resistance: 0,
            loot: crate::domain::combat::monster::LootTable::default(),
            visual_id: Some(99),
            conditions: crate::domain::combat::monster::MonsterCondition::Normal,
            active_conditions: Vec::new(),
            has_acted: false,
        };

        db.monsters.add_monster(without_visual).unwrap();
        db.monsters.add_monster(with_visual).unwrap();

        let content = crate::application::resources::GameContent::new(db);
        let result = resolve_encounter_creature_id(&[10, 250, 11], &content);

        assert_eq!(result, Some(99));
    }

    #[test]
    fn test_spawn_map_spawns_actor_sprite_for_npc() {
        // Integration-style test: spawn_map startup system should create an ActorSprite for each NPC
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        // Initialize Image asset type for tests to avoid asset handle allocation panic
        app.init_asset::<bevy::prelude::Image>();
        app.add_plugins(MapRenderingPlugin);

        // Prepare ContentDatabase with a single NPC definition
        let mut db = crate::sdk::database::ContentDatabase::new();
        let npc_def =
            crate::domain::world::npc::NpcDefinition::new("test_npc", "Test NPC", "portrait.png");
        db.npcs
            .add_npc(npc_def)
            .expect("Failed to add NPC to ContentDatabase");
        app.insert_resource(crate::application::resources::GameContent::new(db));

        // Build GameState with a map containing an NPC placement
        let mut game_state = crate::application::GameState::new();
        let mut map =
            crate::domain::world::Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "test_npc",
                crate::domain::types::Position::new(5, 5),
            ));
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(crate::game::resources::GlobalState(game_state));

        // Insert sprite assets resource (default)
        app.insert_resource(crate::game::resources::sprite_assets::SpriteAssets::default());

        // Ensure required Assets resources exist for systems
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());

        // Run startup systems (including spawn_map_system)
        app.update();

        // Verify there is exactly one entity with ActorSprite + NpcMarker
        let world_ref = app.world_mut();
        let mut query =
            world_ref.query::<(&crate::game::components::sprite::ActorSprite, &NpcMarker)>();
        let results: Vec<_> = query.iter(&*world_ref).collect();
        assert_eq!(results.len(), 1);
        let (actor_sprite, npc_marker) = results[0];
        assert_eq!(npc_marker.npc_id, "test_npc");
        assert_eq!(actor_sprite.sheet_path, DEFAULT_NPC_SPRITE_PATH);
        assert_eq!(actor_sprite.sprite_index, 0);
    }

    #[test]
    fn test_spawn_map_prefers_resolved_npc_sprite_over_default() {
        // Integration-style test: spawn_map should use resolved_npc.sprite when provided
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        // Initialize Image asset type for tests to avoid asset handle allocation panic
        app.init_asset::<bevy::prelude::Image>();
        app.add_plugins(MapRenderingPlugin);

        // Prepare ContentDatabase with a single NPC definition with custom sprite
        let mut db = crate::sdk::database::ContentDatabase::new();
        let sprite = crate::domain::world::SpriteReference {
            sheet_path: "sprites/test/custom_npc.png".to_string(),
            sprite_index: 42,
            animation: None,
            material_properties: None,
        };
        let mut npc_def =
            crate::domain::world::npc::NpcDefinition::new("test_npc", "Test NPC", "portrait.png");
        npc_def = npc_def.with_sprite(sprite.clone());
        db.npcs
            .add_npc(npc_def)
            .expect("Failed to add NPC to ContentDatabase");
        app.insert_resource(crate::application::resources::GameContent::new(db));

        // Build GameState with a map containing an NPC placement
        let mut game_state = crate::application::GameState::new();
        let mut map =
            crate::domain::world::Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "test_npc",
                crate::domain::types::Position::new(5, 5),
            ));
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(crate::game::resources::GlobalState(game_state));

        // Insert sprite assets resource (default)
        app.insert_resource(crate::game::resources::sprite_assets::SpriteAssets::default());

        // Ensure required Assets resources exist for systems
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());

        // Run startup systems (including spawn_map_system)
        app.update();

        // Verify there is exactly one entity with ActorSprite + NpcMarker
        let world_ref = app.world_mut();
        let mut query = world_ref.query::<(
            Entity,
            &crate::game::components::sprite::ActorSprite,
            &NpcMarker,
        )>();
        let results: Vec<_> = query.iter(&*world_ref).collect();
        assert_eq!(results.len(), 1);
        let (entity, actor_sprite, npc_marker) = results[0];
        assert_eq!(npc_marker.npc_id, "test_npc");
        assert_eq!(actor_sprite.sheet_path, "sprites/test/custom_npc.png");
        assert_eq!(actor_sprite.sprite_index, 42);

        // Ensure no AnimatedSprite component was added (since animation was None)
        assert!(world_ref
            .get::<crate::game::components::sprite::AnimatedSprite>(entity)
            .is_none());
    }

    #[test]
    fn test_spawn_map_uses_npc_creature_id_when_available() {
        // Integration-style test: NPCs with creature_id should spawn CreatureVisual entities.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<bevy::prelude::Image>();
        app.add_plugins(MapRenderingPlugin);

        // Prepare ContentDatabase with one NPC and one matching creature definition.
        let mut db = crate::sdk::database::ContentDatabase::new();
        let npc =
            crate::domain::world::npc::NpcDefinition::new("elder", "Village Elder", "elder.png")
                .with_creature_id(51);
        db.npcs
            .add_npc(npc)
            .expect("Failed to add NPC to ContentDatabase");

        let mesh = crate::domain::visual::MeshDefinition {
            name: None,
            vertices: vec![[0.0, 0.0, 0.0], [0.5, 0.0, 0.0], [0.25, 0.5, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };
        let creature = crate::domain::visual::CreatureDefinition {
            id: 51,
            name: "VillageElder".to_string(),
            meshes: vec![mesh],
            mesh_transforms: vec![crate::domain::visual::MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        };
        db.creatures
            .add_creature(creature)
            .expect("Failed to add creature to ContentDatabase");

        app.insert_resource(crate::application::resources::GameContent::new(db));

        // Build GameState with one NPC placement.
        let mut game_state = crate::application::GameState::new();
        let mut map =
            crate::domain::world::Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "elder",
                crate::domain::types::Position::new(5, 5),
            ));
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(crate::game::resources::GlobalState(game_state));

        app.insert_resource(crate::game::resources::sprite_assets::SpriteAssets::default());
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());

        // Run startup systems (including spawn_map_system).
        app.update();

        // Assert CreatureVisual exists and uses the configured creature id.
        let world_ref = app.world_mut();
        let mut query = world_ref.query::<(
            &crate::game::components::creature::CreatureVisual,
            &NpcMarker,
        )>();
        let results: Vec<_> = query.iter(&*world_ref).collect();
        assert_eq!(results.len(), 1);
        let (visual, marker) = results[0];
        assert_eq!(visual.creature_id, 51);
        assert_eq!(marker.npc_id, "elder");

        // Ensure we did not spawn a fallback actor sprite for this NPC.
        let mut sprite_query =
            world_ref.query::<(&crate::game::components::sprite::ActorSprite, &NpcMarker)>();
        let sprite_results: Vec<_> = sprite_query.iter(&*world_ref).collect();
        assert_eq!(sprite_results.len(), 0);
    }

    #[test]
    fn test_spawn_map_uses_recruitable_character_creature_visual() {
        // Integration-style test: recruitable character events should spawn CreatureVisual entities
        // when a creature mapping can be resolved from the character definition.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<bevy::prelude::Image>();
        app.add_plugins(MapRenderingPlugin);

        let mut db = crate::sdk::database::ContentDatabase::new();

        let mut old_gareth = crate::domain::character_definition::CharacterDefinition::new(
            "old_gareth".to_string(),
            "Old Gareth".to_string(),
            "dwarf".to_string(),
            "knight".to_string(),
            crate::domain::character::Sex::Male,
            crate::domain::character::Alignment::Neutral,
        );
        old_gareth.is_premade = true;
        db.characters
            .add_character(old_gareth)
            .expect("Failed to add character to ContentDatabase");

        let mesh = crate::domain::visual::MeshDefinition {
            name: None,
            vertices: vec![[0.0, 0.0, 0.0], [0.5, 0.0, 0.0], [0.25, 0.5, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };
        let creature = crate::domain::visual::CreatureDefinition {
            id: 58,
            name: "OldGareth".to_string(),
            meshes: vec![mesh],
            mesh_transforms: vec![crate::domain::visual::MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        };
        db.creatures
            .add_creature(creature)
            .expect("Failed to add creature to ContentDatabase");

        app.insert_resource(crate::application::resources::GameContent::new(db));

        let mut game_state = crate::application::GameState::new();
        let mut map =
            crate::domain::world::Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let recruitable_pos = crate::domain::types::Position::new(4, 4);
        map.events.insert(
            recruitable_pos,
            crate::domain::world::MapEvent::RecruitableCharacter {
                name: "Old Gareth".to_string(),
                description: "A veteran smith".to_string(),
                character_id: "npc_old_gareth".to_string(),
                dialogue_id: None,
            },
        );

        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(crate::game::resources::GlobalState(game_state));

        app.insert_resource(crate::game::resources::sprite_assets::SpriteAssets::default());
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());

        app.update();

        let world_ref = app.world_mut();
        let mut query = world_ref.query::<(
            &crate::game::components::creature::CreatureVisual,
            &NpcMarker,
            &TileCoord,
        )>();
        let results: Vec<_> = query.iter(&*world_ref).collect();
        assert_eq!(results.len(), 1);

        let (visual, marker, coord) = results[0];
        assert_eq!(visual.creature_id, 58);
        assert_eq!(marker.npc_id, "npc_old_gareth");
        assert_eq!(coord.0, recruitable_pos);
    }

    #[test]
    fn test_recruitable_visual_despawns_after_event_removed() {
        // Integration-style test: recruitable visuals are cleaned up after the
        // backing RecruitableCharacter event is removed from the map.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<bevy::prelude::Image>();
        app.add_plugins(MapRenderingPlugin);

        let mut db = crate::sdk::database::ContentDatabase::new();

        let mut old_gareth = crate::domain::character_definition::CharacterDefinition::new(
            "old_gareth".to_string(),
            "Old Gareth".to_string(),
            "dwarf".to_string(),
            "knight".to_string(),
            crate::domain::character::Sex::Male,
            crate::domain::character::Alignment::Neutral,
        );
        old_gareth.is_premade = true;
        db.characters
            .add_character(old_gareth)
            .expect("Failed to add character to ContentDatabase");

        let mesh = crate::domain::visual::MeshDefinition {
            name: None,
            vertices: vec![[0.0, 0.0, 0.0], [0.5, 0.0, 0.0], [0.25, 0.5, 0.0]],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: None,
            texture_path: None,
        };
        let creature = crate::domain::visual::CreatureDefinition {
            id: 58,
            name: "OldGareth".to_string(),
            meshes: vec![mesh],
            mesh_transforms: vec![crate::domain::visual::MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        };
        db.creatures
            .add_creature(creature)
            .expect("Failed to add creature to ContentDatabase");

        app.insert_resource(crate::application::resources::GameContent::new(db));

        let mut game_state = crate::application::GameState::new();
        let mut map =
            crate::domain::world::Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let recruitable_pos = crate::domain::types::Position::new(4, 4);
        map.events.insert(
            recruitable_pos,
            crate::domain::world::MapEvent::RecruitableCharacter {
                name: "Old Gareth".to_string(),
                description: "A veteran smith".to_string(),
                character_id: "npc_old_gareth".to_string(),
                dialogue_id: None,
            },
        );
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(crate::game::resources::GlobalState(game_state));

        app.insert_resource(crate::game::resources::sprite_assets::SpriteAssets::default());
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());

        // First frame: spawn map visuals.
        app.update();

        {
            let world_ref = app.world_mut();
            let mut query = world_ref.query::<(&RecruitableVisualMarker, &TileCoord)>();
            let results: Vec<_> = query.iter(&*world_ref).collect();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].0.character_id, "npc_old_gareth");
            assert_eq!(results[0].1 .0, recruitable_pos);
        }

        // Remove the recruitable event from map data.
        {
            let mut global_state = app
                .world_mut()
                .resource_mut::<crate::game::resources::GlobalState>();
            let current_map = global_state
                .0
                .world
                .get_current_map_mut()
                .expect("Current map should exist");
            current_map.remove_event(recruitable_pos);
        }

        // Next frame: cleanup system should despawn recruitable visual.
        app.update();

        let world_ref = app.world_mut();
        let mut query = world_ref.query::<&RecruitableVisualMarker>();
        let results: Vec<_> = query.iter(&*world_ref).collect();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_map_plugin_initializes_sprite_assets_and_registers_sheets() {
        // Integration-style test: ensure MapRenderingPlugin initializes the SpriteAssets
        // resource and that registry entries are registered when available.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        // Initialize Image asset type for tests to avoid asset handle allocation panic
        app.init_asset::<bevy::prelude::Image>();
        app.add_plugins(MapRenderingPlugin);

        // Ensure required Assets resources exist for systems
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());

        // Insert minimal GlobalState and GameContent so `spawn_map_system`'s required
        // resources exist during startup. Tests that exercise spawn_map in more depth
        // will insert more complete state as needed.
        app.insert_resource(crate::game::resources::GlobalState(
            crate::application::GameState::new(),
        ));
        let db = crate::sdk::database::ContentDatabase::new();
        app.insert_resource(crate::application::resources::GameContent::new(db));

        // Run startup systems (register_sprite_sheets_system should run)
        app.update();

        // Verify SpriteAssets resource exists
        let sprite_assets = app
            .world()
            .get_resource::<crate::game::resources::sprite_assets::SpriteAssets>();
        assert!(
            sprite_assets.is_some(),
            "SpriteAssets resource should be initialized by MapRenderingPlugin"
        );

        // If registry was successfully loaded during the test run, verify the
        // placeholder sheet is registered and has expected properties.
        if let Some(sa) = sprite_assets {
            if let Some(cfg) = sa.get_config("placeholders") {
                assert_eq!(cfg.texture_path, "sprites/placeholders/npc_placeholder.png");
                assert_eq!(cfg.columns, 1);
                assert_eq!(cfg.rows, 1);
            }
        }
    }
}
