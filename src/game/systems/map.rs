// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::domain::types;
use crate::domain::world;
use crate::domain::world::mark_visible_area;
use crate::domain::world::CreatureBound;
use crate::domain::world::SpriteReference;
use crate::game::components::creature::CreatureVisual;
use crate::game::components::sprite::{ActorType, AnimatedSprite, TileSprite};
use crate::game::resources::sprite_assets::SpriteAssets;
use crate::game::resources::GlobalState;
use crate::game::resources::TerrainMaterialCache;
use crate::game::systems::actor::spawn_actor_sprite;
use crate::game::systems::creature_spawning::spawn_creature;
use crate::game::systems::furniture_rendering::resolve_furniture_fields;
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

/// Message requesting immediate despawn of a recruitable visual at a specific map tile.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct DespawnRecruitableVisual {
    /// Map ID containing the recruitable visual.
    pub map_id: types::MapId,
    /// Tile position of the recruitable visual to remove.
    pub position: types::Position,
    /// Character ID of the recruitable visual to remove.
    pub character_id: String,
}

/// Component tagging an entity as a visual marker for a map encounter.
///
/// Despawned by `cleanup_encounter_visuals` when the backing `MapEvent::Encounter`
/// is removed from the map data (e.g. after the party wins combat against it).
#[derive(bevy::prelude::Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncounterVisualMarker {
    /// Map ID this entity belongs to.
    pub map_id: types::MapId,
    /// Tile position of the originating `MapEvent::Encounter`.
    pub position: types::Position,
}

/// Component tagging a spawned entity as the visual padlock marker for a locked door.
///
/// Despawned by `cleanup_locked_door_markers` when the corresponding door is
/// successfully unlocked (bash, pick, or key) by `lock_action_system`.
///
/// # Examples
///
/// ```
/// use antares::game::systems::map::LockedDoorMarker;
///
/// let marker = LockedDoorMarker { lock_id: "gate_01".to_string() };
/// assert_eq!(marker.lock_id, "gate_01");
/// ```
#[derive(bevy::prelude::Component, Debug, Clone, PartialEq, Eq)]
pub struct LockedDoorMarker {
    /// Matches `MapEvent::LockedDoor::lock_id` so it can be despawned
    /// after the door is unlocked.
    pub lock_id: String,
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

/// Plugin responsible for dynamic map management (spawning/despawning marker
/// entities and event triggers when the current map changes).
pub struct MapManagerPlugin;

impl Plugin for MapManagerPlugin {
    fn build(&self, app: &mut App) {
        // Register the map change message and the handler + spawn systems
        app.add_message::<MapChangeEvent>()
            .add_message::<DespawnRecruitableVisual>()
            // Phase 3: register SetFacing message and proximity/facing systems
            .add_plugins(crate::game::systems::facing::FacingPlugin)
            // Phase 2 (locks): seed lock_states for every map present at startup.
            // This runs once before the first frame so that the split
            // exploration-interaction input flow can find lock entries immediately.
            .add_systems(Startup, init_map_lock_states_system)
            // Process explicit map change requests first, then let the marker
            // spawner observe the changed world state and spawn/despawn accordingly.
            .add_systems(
                Update,
                (
                    map_change_handler,
                    spawn_map_markers,
                    handle_despawn_recruitable_visual,
                    cleanup_recruitable_visuals,
                    cleanup_encounter_visuals,
                    cleanup_locked_door_markers,
                ),
            );
    }
}

/// Despawns encounter visual entities when their backing `MapEvent::Encounter` is no longer
/// present on the map (e.g. after the party defeats the monsters in that encounter).
///
/// Mirrors the pattern used by `cleanup_recruitable_visuals`.
fn cleanup_encounter_visuals(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    query: Query<(Entity, &EncounterVisualMarker)>,
) {
    let game_state = &global_state.0;
    for (entity, marker) in query.iter() {
        let Some(map) = game_state.world.get_map(marker.map_id) else {
            // Map no longer loaded — despawn the visual.
            commands.entity(entity).despawn();
            continue;
        };

        // Despawn if the backing encounter event is gone.
        let event_present = matches!(
            map.get_event(marker.position),
            Some(world::MapEvent::Encounter { .. })
        );
        if !event_present {
            commands.entity(entity).despawn();
        }
    }
}

/// Handles explicit recruitable-visual despawn requests triggered by gameplay systems.
///
/// This provides an immediate despawn path when a recruitable joins the party,
/// while `cleanup_recruitable_visuals` remains as a fallback safety net.
fn handle_despawn_recruitable_visual(
    mut commands: Commands,
    mut ev_reader: MessageReader<DespawnRecruitableVisual>,
    query: Query<(Entity, &MapEntity, &TileCoord, &RecruitableVisualMarker)>,
) {
    for ev in ev_reader.read() {
        for (entity, map_entity, tile_coord, marker) in query.iter() {
            if map_entity.0 == ev.map_id
                && tile_coord.0 == ev.position
                && marker.character_id == ev.character_id
            {
                commands.entity(entity).despawn();
            }
        }
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

/// Despawns locked-door marker entities when their corresponding door has been
/// unlocked (i.e. the `LockedDoor` event is gone from the map or the lock state
/// shows the door is no longer locked).
///
/// This mirrors the pattern used by `cleanup_recruitable_visuals`.
fn cleanup_locked_door_markers(
    mut commands: Commands,
    global_state: Res<GlobalState>,
    query: Query<(Entity, &LockedDoorMarker, &TileCoord, &MapEntity)>,
) {
    let game_state = &global_state.0;
    let Some(current_map) = game_state.world.get_current_map() else {
        return;
    };

    for (entity, marker, tile_coord, map_entity) in query.iter() {
        if map_entity.0 != current_map.id {
            continue;
        }

        // Despawn if the LockedDoor event has been removed or the door is now unlocked.
        let should_keep = matches!(
            current_map.get_event(tile_coord.0),
            Some(crate::domain::world::MapEvent::LockedDoor { lock_id, .. })
                if lock_id == &marker.lock_id
        ) && current_map
            .lock_states
            .get(marker.lock_id.as_str())
            .map(|ls| ls.is_locked)
            .unwrap_or(false);

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
                // Load terrain textures/materials before map spawn runs so the
                // cache is populated when spawn_map_system executes.
                (
                    super::terrain_materials::load_terrain_materials_system,
                    register_sprite_sheets_system,
                    spawn_map_system,
                )
                    .chain(),
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
    terrain_cache: Res<TerrainMaterialCache>,
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
        &terrain_cache,
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

/// Resolves a creature id for an encounter marker from a monster group.
///
/// Uses the first monster entry that has a configured `creature_id`.
fn resolve_encounter_creature_id(
    monster_group: &[types::MonsterId],
    content: &crate::application::resources::GameContent,
) -> Option<types::CreatureId> {
    for monster_id in monster_group {
        if let Some(monster_def) = content.0.monsters.get_monster(*monster_id) {
            if let Some(creature_id) = monster_def.creature_id() {
                return Some(creature_id);
            }
        }
    }

    None
}

/// System that handles explicit MapChangeEvent messages by updating the world
/// current map and party position. Invalid map ids are ignored (no panic).
/// Startup system that initialises `lock_states` for every map loaded into the
/// world at game start.
///
/// Calling [`Map::init_lock_states`] is idempotent — it skips any lock whose
/// `lock_id` is already present in `lock_states` — so it is safe to call
/// multiple times. However, it must run before any player interaction so that
/// the split exploration-interaction input flow can look up lock entries
/// immediately.
///
/// # Examples
///
/// The system is registered as a [`Startup`] system in [`MapManagerPlugin`].
/// For map *transitions* the same initialisation call is made inside
/// [`map_change_handler`].
fn init_map_lock_states_system(mut global_state: ResMut<GlobalState>) {
    let map_count = global_state.0.world.maps.len();
    for map in global_state.0.world.maps.values_mut() {
        map.init_lock_states();
    }
    info!(
        "Lock states initialised for {} map(s) at startup",
        map_count
    );
}

fn map_change_handler(
    mut ev_reader: MessageReader<MapChangeEvent>,
    mut global_state: ResMut<GlobalState>,
) {
    for ev in ev_reader.read() {
        if global_state.0.world.get_map(ev.target_map).is_some() {
            global_state.0.world.set_current_map(ev.target_map);
            global_state.0.world.set_party_position(ev.target_pos);
            mark_visible_area(
                &mut global_state.0.world,
                ev.target_pos,
                crate::domain::world::VISIBILITY_RADIUS,
            );
            // Phase 2 (locks): seed lock_states for the newly active map so that
            // any LockedDoor / LockedContainer events on it are registered before
            // the player can interact with them.  init_lock_states is idempotent —
            // it skips entries that already exist — so previously-unlocked doors
            // keep their state after a map transition.
            if let Some(map) = global_state.0.world.get_current_map_mut() {
                map.init_lock_states();
            }
            // Each map transition (teleport, dungeon entrance, town portal, etc.)
            // costs time. Advance after confirming the map actually exists so that
            // invalid/no-op events do not tick the clock.
            global_state.0.advance_time(
                crate::domain::resources::TIME_COST_MAP_TRANSITION_MINUTES,
                None,
            );
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
    terrain_cache: Option<Res<TerrainMaterialCache>>,
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
        // Use the cached resource when available; fall back to an empty default
        // so tests that don't insert TerrainMaterialCache still work correctly.
        let default_cache = TerrainMaterialCache::default();
        let cache_ref: &TerrainMaterialCache = terrain_cache.as_deref().unwrap_or(&default_cache);

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
            cache_ref,
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
fn terrain_material_with_optional_tint(
    terrain: world::TerrainType,
    tint: Option<(f32, f32, f32)>,
    terrain_cache: &TerrainMaterialCache,
    source_material: Option<StandardMaterial>,
    materials_mut: &mut Assets<StandardMaterial>,
    fallback_base_color: Color,
    fallback_roughness: f32,
) -> Handle<StandardMaterial> {
    let cached_handle = terrain_cache.get(terrain).cloned();

    match (cached_handle, tint, source_material) {
        (Some(handle), None, _) => handle,
        (Some(_handle), Some((r, g, b)), Some(mut source_material)) => {
            let source_srgba = source_material.base_color.to_srgba();
            source_material.base_color = Color::srgba(
                source_srgba.red * r,
                source_srgba.green * g,
                source_srgba.blue * b,
                source_srgba.alpha,
            );
            source_material.perceptual_roughness = fallback_roughness;
            materials_mut.add(source_material)
        }
        (Some(_), Some((r, g, b)), None) | (None, Some((r, g, b)), _) => {
            let fallback_srgba = fallback_base_color.to_srgba();
            materials_mut.add(StandardMaterial {
                base_color: Color::srgba(
                    fallback_srgba.red * r,
                    fallback_srgba.green * g,
                    fallback_srgba.blue * b,
                    fallback_srgba.alpha,
                ),
                perceptual_roughness: fallback_roughness,
                ..default()
            })
        }
        (None, None, _) => materials_mut.add(StandardMaterial {
            base_color: fallback_base_color,
            perceptual_roughness: fallback_roughness,
            ..default()
        }),
    }
}

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
    terrain_cache: &TerrainMaterialCache,
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
        let water_rgb = (0.2_f32, 0.4_f32, 0.8_f32); // Blue
        let mountain_rgb = (0.5_f32, 0.5_f32, 0.5_f32); // Gray rock
        let forest_rgb = (0.2_f32, 0.6_f32, 0.2_f32); // Green
        let grass_rgb = (0.3_f32, 0.5_f32, 0.2_f32); // Darker green floor
        let stone_rgb = (0.5_f32, 0.5_f32, 0.55_f32);
        let dirt_rgb = (0.4_f32, 0.3_f32, 0.2_f32);

        let floor_color = Color::srgb(floor_rgb.0, floor_rgb.1, floor_rgb.2);
        let wall_base_color = Color::srgb(wall_base_rgb.0, wall_base_rgb.1, wall_base_rgb.2);
        let water_color = Color::srgb(water_rgb.0, water_rgb.1, water_rgb.2);
        let mountain_color = Color::srgb(mountain_rgb.0, mountain_rgb.1, mountain_rgb.2);
        let _forest_color = Color::srgb(forest_rgb.0, forest_rgb.1, forest_rgb.2);
        let grass_color = Color::srgb(grass_rgb.0, grass_rgb.1, grass_rgb.2);

        // Look up cached textured materials, falling back to flat-colour
        // materials if the cache is not yet populated (e.g. in tests that do
        // not run the terrain-materials startup system).
        let floor_material = terrain_cache
            .get(world::TerrainType::Ground)
            .cloned()
            .unwrap_or_else(|| {
                materials.add(StandardMaterial {
                    base_color: floor_color,
                    perceptual_roughness: 0.95,
                    ..default()
                })
            });

        let water_material = terrain_cache
            .get(world::TerrainType::Water)
            .cloned()
            .unwrap_or_else(|| {
                materials.add(StandardMaterial {
                    base_color: water_color,
                    perceptual_roughness: 0.10,
                    ..default()
                })
            });

        let grass_material = terrain_cache
            .get(world::TerrainType::Grass)
            .cloned()
            .unwrap_or_else(|| {
                materials.add(StandardMaterial {
                    base_color: grass_color,
                    perceptual_roughness: 0.90,
                    ..default()
                })
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

                            // Apply color tint if specified while preserving
                            // the cached textured source material when
                            // available. Cached material assets are never
                            // mutated in place.
                            let source_material = terrain_cache
                                .get(world::TerrainType::Mountain)
                                .and_then(|handle| materials.get(handle))
                                .cloned();
                            let material = terrain_material_with_optional_tint(
                                world::TerrainType::Mountain,
                                tile.visual.color_tint,
                                terrain_cache,
                                source_material,
                                &mut materials,
                                mountain_color,
                                0.85,
                            );

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
                                        crate::game::systems::advanced_trees::TreeType::Palm
                                    }
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
                                        &asset_server,
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
                                    &asset_server,
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
                                &asset_server,
                                pos,
                                map.id,
                                Some(&tile.visual),
                                tile.visual.color_tint,
                                &quality_settings,
                            );
                        }
                        _ => {
                            // Spawn regular floor for Ground, Stone, Dirt, Lava,
                            // Swamp and any future terrain types.  Use the cached
                            // textured material when available; fall back to the
                            // flat-colour floor material otherwise.
                            let tile_material = terrain_cache
                                .get(tile.terrain)
                                .cloned()
                                .unwrap_or_else(|| floor_material.clone());

                            commands.spawn((
                                Mesh3d(floor_mesh.clone()),
                                MeshMaterial3d(tile_material),
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

        // Phase 2: Build a facing-override map from NpcDialogue events so that
        // an event-level `facing` field can override the NpcPlacement.facing for
        // the same NPC.  Only entries where `facing` is `Some` are stored.
        let npc_dialogue_facing: std::collections::HashMap<
            String,
            crate::domain::types::Direction,
        > = map
            .events
            .values()
            .filter_map(|ev| {
                if let world::MapEvent::NpcDialogue { npc_id, facing, .. } = ev {
                    facing.map(|d| (npc_id.clone(), d))
                } else {
                    None
                }
            })
            .collect();

        // Phase 3/4: Build a proximity-facing map from NpcDialogue events so that
        // entities with `proximity_facing: true` get a `ProximityFacing` component,
        // carrying the optional `rotation_speed` for Phase 4 smooth rotation.
        let npc_dialogue_proximity: std::collections::HashMap<String, Option<f32>> = map
            .events
            .values()
            .filter_map(|ev| {
                if let world::MapEvent::NpcDialogue {
                    npc_id,
                    proximity_facing,
                    rotation_speed,
                    ..
                } = ev
                {
                    if *proximity_facing {
                        Some((npc_id.clone(), *rotation_speed))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

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
                        Vec3::new(
                            x + TILE_CENTER_OFFSET,
                            creature_def.foot_ground_offset(),
                            y + TILE_CENTER_OFFSET,
                        ),
                        None,
                        None,
                        // Phase 2: event-level NpcDialogue.facing overrides NpcPlacement.facing
                        npc_dialogue_facing
                            .get(&resolved_npc.npc_id)
                            .copied()
                            .or(resolved_npc.facing),
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

                    // Phase 3/4: insert ProximityFacing when the NpcDialogue event
                    // has proximity_facing: true for this NPC.
                    if let Some(rotation_speed) = npc_dialogue_proximity.get(&resolved_npc.npc_id) {
                        commands.entity(entity).insert(
                            crate::game::systems::facing::ProximityFacing {
                                trigger_distance: 2,
                                rotation_speed: *rotation_speed,
                            },
                        );
                    }

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

            // Phase 2: apply facing rotation to the sprite fallback entity and
            // attach FacingComponent so runtime systems can query/change it.
            {
                use crate::domain::types::Direction;
                use crate::game::components::creature::FacingComponent;
                // Event-level NpcDialogue.facing overrides NpcPlacement.facing
                let resolved_facing = npc_dialogue_facing
                    .get(&resolved_npc.npc_id)
                    .copied()
                    .or(resolved_npc.facing);
                let effective_dir = resolved_facing.unwrap_or(Direction::North);
                let yaw = effective_dir.direction_to_yaw_radians();
                commands.entity(entity).insert((
                    FacingComponent::new(effective_dir),
                    Transform::from_translation(Vec3::new(
                        x + TILE_CENTER_OFFSET,
                        0.9,
                        y + TILE_CENTER_OFFSET,
                    ))
                    .with_rotation(Quat::from_rotation_y(yaw)),
                ));
            }

            // Attach map tags and NPC marker to the spawned actor entity
            commands.entity(entity).insert((
                MapEntity(map.id),
                TileCoord(resolved_npc.position),
                NpcMarker {
                    npc_id: resolved_npc.npc_id.clone(),
                },
                Visibility::default(),
            ));

            // Phase 3/4: insert ProximityFacing on the sprite-fallback entity too.
            if let Some(rotation_speed) = npc_dialogue_proximity.get(&resolved_npc.npc_id) {
                commands
                    .entity(entity)
                    .insert(crate::game::systems::facing::ProximityFacing {
                        trigger_distance: 2,
                        rotation_speed: *rotation_speed,
                    });
            }
        }

        // Spawn procedural event markers and recruitable character visuals.
        for (position, event) in map.events.iter() {
            // Get tile visual metadata for rotation (if tile exists)
            let rotation_y = map
                .get_tile(*position)
                .and_then(|tile| tile.visual.rotation_y);

            match event {
                world::MapEvent::Sign { name, facing, .. } => {
                    procedural_meshes::spawn_sign(
                        &mut commands,
                        &mut materials,
                        &mut meshes,
                        *position,
                        name.clone(),
                        map.id,
                        procedural_cache,
                        rotation_y,
                        *facing, // Phase 2: cardinal facing from map event
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
                    furniture_id,
                    furniture_type,
                    rotation_y,
                    scale,
                    material,
                    flags,
                    color_tint,
                    key_item_id,
                    ..
                } => {
                    // Resolve final furniture properties: database template defaults
                    // merged with per-instance inline overrides from the map event.
                    let (
                        resolved_type,
                        resolved_material,
                        resolved_scale,
                        resolved_flags,
                        resolved_tint,
                    ) = resolve_furniture_fields(
                        *furniture_id,
                        *furniture_type,
                        *material,
                        *scale,
                        flags,
                        *color_tint,
                        &content.0.furniture,
                    );

                    procedural_meshes::spawn_furniture(
                        &mut commands,
                        &mut materials,
                        &mut meshes,
                        *position,
                        map.id,
                        resolved_type,
                        *rotation_y,
                        resolved_scale,
                        resolved_material,
                        &resolved_flags,
                        resolved_tint,
                        *key_item_id,
                        procedural_cache,
                    );
                }
                world::MapEvent::Encounter {
                    monster_group,
                    facing,
                    proximity_facing,
                    rotation_speed,
                    ..
                } => {
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
                                Vec3::new(
                                    x + TILE_CENTER_OFFSET,
                                    creature_def.foot_ground_offset(),
                                    y + TILE_CENTER_OFFSET,
                                ),
                                None,
                                None,
                                *facing, // Phase 2: wire Encounter.facing
                            );

                            commands.entity(entity).insert((
                                CreatureVisual {
                                    creature_id,
                                    scale_override: None,
                                },
                                MapEntity(map.id),
                                TileCoord(*position),
                                EncounterVisualMarker {
                                    map_id: map.id,
                                    position: *position,
                                },
                                Visibility::default(),
                            ));

                            // Phase 3/4: insert ProximityFacing when the event flag is set,
                            // forwarding rotation_speed for smooth rotation support.
                            if *proximity_facing {
                                commands.entity(entity).insert(
                                    crate::game::systems::facing::ProximityFacing {
                                        trigger_distance: 2,
                                        rotation_speed: *rotation_speed,
                                    },
                                );
                            }

                            continue;
                        }
                    }

                    warn!(
                        "Encounter at ({}, {}) has no resolvable creature visual; no marker spawned",
                        position.x, position.y
                    );
                }
                world::MapEvent::RecruitableCharacter {
                    character_id,
                    name,
                    facing,
                    ..
                } => {
                    let x = position.x as f32;
                    let y = position.y as f32;

                    if let Some(creature_id) = content
                        .0
                        .characters
                        .get_character(character_id)
                        .and_then(|def| def.creature_id())
                    {
                        if let Some(creature_def) = content.0.creatures.get_creature(creature_id) {
                            let entity = spawn_creature(
                                &mut commands,
                                creature_def,
                                &mut meshes,
                                &mut materials,
                                Vec3::new(
                                    x + TILE_CENTER_OFFSET,
                                    creature_def.foot_ground_offset(),
                                    y + TILE_CENTER_OFFSET,
                                ),
                                None,
                                None,
                                *facing, // Phase 2: wire RecruitableCharacter.facing
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
                world::MapEvent::LockedDoor { lock_id, .. } => {
                    // Spawn a small amber padlock marker when the door is still locked.
                    let is_locked = map
                        .lock_states
                        .get(lock_id.as_str())
                        .map(|ls| ls.is_locked)
                        .unwrap_or(true);

                    if is_locked {
                        let x = position.x as f32;
                        let y = position.y as f32;
                        /// Deep amber tint to distinguish locked doors at a glance.
                        const LOCKED_DOOR_MARKER_COLOR: [f32; 3] = [0.8, 0.5, 0.1];
                        let amber = Color::srgb(
                            LOCKED_DOOR_MARKER_COLOR[0],
                            LOCKED_DOOR_MARKER_COLOR[1],
                            LOCKED_DOOR_MARKER_COLOR[2],
                        );
                        let marker_mesh = meshes.add(Cuboid::new(0.2, 0.2, 0.2));
                        let marker_mat = materials.add(StandardMaterial {
                            base_color: amber,
                            perceptual_roughness: 0.6,
                            ..default()
                        });
                        commands.spawn((
                            Mesh3d(marker_mesh),
                            MeshMaterial3d(marker_mat),
                            Transform::from_xyz(
                                x + TILE_CENTER_OFFSET,
                                1.5,
                                y + TILE_CENTER_OFFSET,
                            ),
                            GlobalTransform::default(),
                            Visibility::default(),
                            MapEntity(map.id),
                            TileCoord(*position),
                            LockedDoorMarker {
                                lock_id: lock_id.clone(),
                            },
                        ));
                    }
                }
                // LockedContainer events have no visual marker — opened exclusively via UI.
                world::MapEvent::LockedContainer { .. } => {}
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
    use crate::domain::types::Position;
    use crate::domain::world::SpriteAnimation;
    use crate::game::components::dialogue::NpcDialogue;
    use crate::game::resources::GlobalState;

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
    fn test_starting_tile_marked_on_map_load() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChangeEvent>();
        app.add_systems(Update, map_change_handler);

        let mut game_state = crate::application::GameState::new();
        let map = crate::domain::world::Map::new(
            1,
            "Test Map".to_string(),
            "Visibility test map".to_string(),
            5,
            5,
        );
        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(GlobalState(game_state));

        let start = Position::new(2, 2);
        app.world_mut().write_message(MapChangeEvent {
            target_map: 1,
            target_pos: start,
        });
        app.update();

        let global_state = app.world().resource::<GlobalState>();
        let current_map = global_state.0.world.get_current_map().unwrap();

        assert!(current_map.get_tile(start).unwrap().visited);
        assert!(current_map.get_tile(Position::new(1, 1)).unwrap().visited);
        assert!(current_map.get_tile(Position::new(3, 3)).unwrap().visited);
        assert!(!current_map.get_tile(Position::new(0, 0)).unwrap().visited);
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
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
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
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
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

            assert_eq!(sheet_path, expected_sheet);
            assert_eq!(sprite_index, expected_index);
        }
    }

    #[test]
    fn test_terrain_material_with_optional_tint_returns_cached_handle_when_tint_none() {
        let mut cache = TerrainMaterialCache::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let texture_handle = Handle::<Image>::default();
        let cached_handle = materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.5, 0.5),
            base_color_texture: Some(texture_handle),
            perceptual_roughness: 0.85,
            ..default()
        });
        cache.set(world::TerrainType::Mountain, cached_handle.clone());

        let result = terrain_material_with_optional_tint(
            world::TerrainType::Mountain,
            None,
            &cache,
            None,
            &mut materials,
            Color::srgb(0.5, 0.5, 0.5),
            0.85,
        );

        assert_eq!(result, cached_handle);
    }

    #[test]
    fn test_terrain_material_with_optional_tint_returns_new_handle_when_tinted() {
        let mut cache = TerrainMaterialCache::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let texture_handle = Handle::<Image>::default();
        let cached_handle = materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.5, 0.5),
            base_color_texture: Some(texture_handle),
            perceptual_roughness: 0.85,
            ..default()
        });
        let source_material = materials
            .get(&cached_handle)
            .expect("cached material should exist")
            .clone();
        cache.set(world::TerrainType::Mountain, cached_handle.clone());

        let result = terrain_material_with_optional_tint(
            world::TerrainType::Mountain,
            Some((0.8, 0.7, 0.6)),
            &cache,
            Some(source_material),
            &mut materials,
            Color::srgb(0.5, 0.5, 0.5),
            0.85,
        );

        assert_ne!(result, cached_handle);
    }

    #[test]
    fn test_terrain_material_with_optional_tint_preserves_base_color_texture() {
        let mut cache = TerrainMaterialCache::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let texture_handle = Handle::<Image>::default();
        let cached_handle = materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.5, 0.5),
            base_color_texture: Some(texture_handle.clone()),
            perceptual_roughness: 0.85,
            ..default()
        });
        let source_material = materials
            .get(&cached_handle)
            .expect("cached material should exist")
            .clone();
        cache.set(world::TerrainType::Mountain, cached_handle);

        let result = terrain_material_with_optional_tint(
            world::TerrainType::Mountain,
            Some((0.8, 0.7, 0.6)),
            &cache,
            Some(source_material),
            &mut materials,
            Color::srgb(0.5, 0.5, 0.5),
            0.85,
        );

        let tinted_material = materials
            .get(&result)
            .expect("tinted material handle should resolve");
        assert_eq!(tinted_material.base_color_texture, Some(texture_handle));
    }

    #[test]
    fn test_terrain_material_with_optional_tint_fallback_works_when_cache_empty() {
        let cache = TerrainMaterialCache::default();
        let mut materials = Assets::<StandardMaterial>::default();

        let result = terrain_material_with_optional_tint(
            world::TerrainType::Mountain,
            Some((0.8, 0.7, 0.6)),
            &cache,
            None,
            &mut materials,
            Color::srgb(0.5, 0.5, 0.5),
            0.85,
        );

        let material = materials
            .get(&result)
            .expect("fallback material handle should resolve");
        assert!(material.base_color_texture.is_none());
        assert_eq!(material.perceptual_roughness, 0.85);
    }

    #[test]
    fn test_terrain_material_with_optional_tint_does_not_mutate_cached_material() {
        let mut cache = TerrainMaterialCache::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let texture_handle = Handle::<Image>::default();
        let cached_handle = materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.5, 0.5),
            base_color_texture: Some(texture_handle.clone()),
            perceptual_roughness: 0.85,
            ..default()
        });
        let original_cached_material = materials
            .get(&cached_handle)
            .expect("cached material should exist")
            .clone();
        let source_material = original_cached_material.clone();
        cache.set(world::TerrainType::Mountain, cached_handle.clone());

        let _ = terrain_material_with_optional_tint(
            world::TerrainType::Mountain,
            Some((0.8, 0.7, 0.6)),
            &cache,
            Some(source_material),
            &mut materials,
            Color::srgb(0.5, 0.5, 0.5),
            0.85,
        );

        let cached_material_after = materials
            .get(&cached_handle)
            .expect("cached material should still exist");
        assert_eq!(
            cached_material_after.base_color,
            original_cached_material.base_color
        );
        assert_eq!(
            cached_material_after.base_color_texture,
            original_cached_material.base_color_texture
        );
        assert_eq!(
            cached_material_after.perceptual_roughness,
            original_cached_material.perceptual_roughness
        );
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
            creature_id: Some(7),
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
            creature_id: None,
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
            creature_id: Some(99),
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
        old_gareth.creature_id = Some(58);
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
                character_id: "old_gareth".to_string(),
                dialogue_id: None,
                time_condition: None,
                facing: None,
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
        assert_eq!(marker.npc_id, "old_gareth");
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
        old_gareth.creature_id = Some(58);
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
                character_id: "old_gareth".to_string(),
                dialogue_id: None,
                time_condition: None,
                facing: None,
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
            assert_eq!(results[0].0.character_id, "old_gareth");
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
    fn test_explicit_recruitable_visual_despawn_message_removes_matching_visual() {
        use crate::domain::types::Position;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<DespawnRecruitableVisual>();
        app.add_systems(Update, handle_despawn_recruitable_visual);

        let recruitable_pos = Position::new(4, 4);
        let other_pos = Position::new(6, 6);

        app.world_mut().spawn((
            MapEntity(1),
            TileCoord(recruitable_pos),
            RecruitableVisualMarker {
                character_id: "whisper".to_string(),
            },
        ));

        app.world_mut().spawn((
            MapEntity(1),
            TileCoord(other_pos),
            RecruitableVisualMarker {
                character_id: "old_gareth".to_string(),
            },
        ));

        {
            let mut messages = app
                .world_mut()
                .resource_mut::<Messages<DespawnRecruitableVisual>>();
            messages.write(DespawnRecruitableVisual {
                map_id: 1,
                position: recruitable_pos,
                character_id: "whisper".to_string(),
            });
        }

        app.update();

        let world_ref = app.world_mut();
        let mut query = world_ref.query::<(&RecruitableVisualMarker, &TileCoord, &MapEntity)>();
        let results: Vec<_> = query.iter(&*world_ref).collect();

        assert_eq!(
            results.len(),
            1,
            "Exactly one non-target recruitable visual should remain after explicit despawn",
        );
        assert_eq!(results[0].0.character_id, "old_gareth");
        assert_eq!(results[0].1 .0, other_pos);
        assert_eq!(results[0].2 .0, 1);
    }

    /// T4-E4: Spawn an `EncounterVisualMarker` entity, remove the backing event,
    /// run `cleanup_encounter_visuals`, and assert the entity no longer exists.
    #[test]
    fn test_encounter_visual_despawned_when_event_removed() {
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<bevy::prelude::Image>();
        // Register MapManagerPlugin so cleanup_encounter_visuals is scheduled.
        // spawn_map_markers (also registered by the plugin) requires these resources.
        app.add_plugins(MapManagerPlugin);

        let map_id: crate::domain::types::MapId = 1;
        let encounter_pos = Position::new(3, 4);

        // Build a map with a live encounter event.
        let mut map = Map::new(map_id, "Test".to_string(), "Desc".to_string(), 10, 10);
        map.add_event(
            encounter_pos,
            MapEvent::Encounter {
                name: "Goblins".to_string(),
                description: "Goblins lurk here".to_string(),
                monster_group: vec![1],
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
                combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
            },
        );

        let mut game_state = crate::application::GameState::new();
        game_state.world.add_map(map);
        game_state.world.set_current_map(map_id);
        app.insert_resource(crate::game::resources::GlobalState(game_state));

        // Required resources for spawn_map_markers (called each frame by MapManagerPlugin).
        app.insert_resource(crate::application::resources::GameContent::new(
            crate::sdk::database::ContentDatabase::new(),
        ));
        app.insert_resource(crate::game::resources::sprite_assets::SpriteAssets::default());
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.insert_resource(crate::game::resources::GrassQualitySettings::default());

        // Spawn an EncounterVisualMarker entity manually to simulate what spawn_map does.
        let marker_entity = app
            .world_mut()
            .spawn(EncounterVisualMarker {
                map_id,
                position: encounter_pos,
            })
            .id();

        // First frame: event is present — entity should survive.
        app.update();
        assert!(
            app.world().get_entity(marker_entity).is_ok(),
            "Entity should still exist while the encounter event is present"
        );

        // Remove the encounter event from the map (simulating victory removal).
        {
            let mut gs = app
                .world_mut()
                .resource_mut::<crate::game::resources::GlobalState>();
            let map = gs.0.world.get_map_mut(map_id).expect("map must exist");
            map.remove_event(encounter_pos);
        }

        // Next frame: cleanup system should despawn the visual.
        app.update();
        assert!(
            app.world().get_entity(marker_entity).is_err(),
            "Entity must be despawned after the backing encounter event is removed"
        );
    }

    /// T4-E5: Spawn an `EncounterVisualMarker` entity, leave the backing event
    /// intact, run `cleanup_encounter_visuals`, and assert the entity still exists.
    #[test]
    fn test_encounter_visual_kept_when_event_present() {
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<bevy::prelude::Image>();
        app.add_plugins(MapManagerPlugin);

        let map_id: crate::domain::types::MapId = 1;
        let encounter_pos = Position::new(6, 2);

        let mut map = Map::new(map_id, "Test".to_string(), "Desc".to_string(), 10, 10);
        map.add_event(
            encounter_pos,
            MapEvent::Encounter {
                name: "Trolls".to_string(),
                description: "Trolls guard this tile".to_string(),
                monster_group: vec![2],
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
                combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
            },
        );

        let mut game_state = crate::application::GameState::new();
        game_state.world.add_map(map);
        game_state.world.set_current_map(map_id);
        app.insert_resource(crate::game::resources::GlobalState(game_state));

        // Required resources for spawn_map_markers.
        app.insert_resource(crate::application::resources::GameContent::new(
            crate::sdk::database::ContentDatabase::new(),
        ));
        app.insert_resource(crate::game::resources::sprite_assets::SpriteAssets::default());
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.insert_resource(crate::game::resources::GrassQualitySettings::default());

        // Spawn an EncounterVisualMarker entity — backing event is still present.
        let marker_entity = app
            .world_mut()
            .spawn(EncounterVisualMarker {
                map_id,
                position: encounter_pos,
            })
            .id();

        // Run cleanup: entity must NOT be despawned.
        app.update();
        assert!(
            app.world().get_entity(marker_entity).is_ok(),
            "Entity must remain when the backing encounter event is still present"
        );
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

    // ===== Phase 2: Static Map-Time Facing tests =====

    /// Helper: build a minimal single-mesh CreatureDefinition with the given id.
    fn make_creature_def(
        id: crate::domain::types::CreatureId,
    ) -> crate::domain::visual::CreatureDefinition {
        crate::domain::visual::CreatureDefinition {
            id,
            name: format!("Creature_{}", id),
            meshes: vec![crate::domain::visual::MeshDefinition {
                name: None,
                vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]],
                indices: vec![0, 1, 2],
                normals: None,
                uvs: None,
                color: [1.0, 1.0, 1.0, 1.0],
                lod_levels: None,
                lod_distances: None,
                material: None,
                texture_path: None,
            }],
            mesh_transforms: vec![crate::domain::visual::MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        }
    }

    /// Helper: build a minimal Bevy App wired up for spawn_map integration tests.
    fn make_spawn_app(db: crate::sdk::database::ContentDatabase) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<bevy::prelude::Image>();
        app.add_plugins(MapRenderingPlugin);
        app.insert_resource(crate::application::resources::GameContent::new(db));
        app.insert_resource(crate::game::resources::sprite_assets::SpriteAssets::default());
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app
    }

    #[test]
    fn test_npc_facing_applied_at_spawn() {
        // Integration test: NPC placement with facing: Some(East) should produce
        // a CreatureVisual entity whose FacingComponent stores Direction::East.
        use crate::domain::types::{Direction, Position};
        use crate::domain::world::npc::{NpcDefinition, NpcPlacement};
        use crate::game::components::creature::FacingComponent;

        let mut db = crate::sdk::database::ContentDatabase::new();

        let mut npc_def = NpcDefinition::new("npc_test_facing", "Test NPC", "portrait");
        npc_def.creature_id = Some(10);
        db.npcs.add_npc(npc_def).expect("add npc");
        db.creatures
            .add_creature(make_creature_def(10))
            .expect("add creature");

        let mut app = make_spawn_app(db);

        let mut game_state = crate::application::GameState::new();
        let mut map = crate::domain::world::Map::new(1, "T".to_string(), "D".to_string(), 10, 10);

        let npc_pos = Position::new(3, 3);
        let placement = NpcPlacement::with_facing("npc_test_facing", npc_pos, Direction::East);
        map.npc_placements.push(placement);

        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(crate::game::resources::GlobalState(game_state));

        app.update();

        let world_ref = app.world_mut();
        let mut query = world_ref.query::<(&FacingComponent, &NpcMarker)>();
        let results: Vec<_> = query.iter(&*world_ref).collect();
        assert_eq!(results.len(), 1, "expected exactly one NPC entity");
        assert_eq!(
            results[0].0.direction,
            Direction::East,
            "FacingComponent must store the placement's facing direction"
        );
    }

    #[test]
    fn test_facing_component_on_npc() {
        // After spawn, querying FacingComponent on the NPC entity gives East.
        use crate::domain::types::{Direction, Position};
        use crate::domain::world::npc::{NpcDefinition, NpcPlacement};
        use crate::game::components::creature::FacingComponent;

        let mut db = crate::sdk::database::ContentDatabase::new();

        let mut npc_def = NpcDefinition::new("npc_facing_east", "Facing East NPC", "portrait");
        npc_def.creature_id = Some(11);
        db.npcs.add_npc(npc_def).expect("add npc");
        db.creatures
            .add_creature(make_creature_def(11))
            .expect("add creature");

        let mut app = make_spawn_app(db);

        let mut game_state = crate::application::GameState::new();
        let mut map = crate::domain::world::Map::new(1, "T".to_string(), "D".to_string(), 10, 10);

        let npc_pos = Position::new(5, 5);
        let placement = NpcPlacement::with_facing("npc_facing_east", npc_pos, Direction::East);
        map.npc_placements.push(placement);

        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(crate::game::resources::GlobalState(game_state));
        app.update();

        let world_ref = app.world_mut();
        let mut query = world_ref.query::<(&FacingComponent, &NpcMarker)>();
        let found: Vec<_> = query.iter(&*world_ref).collect();
        assert_eq!(found.len(), 1, "one NPC entity expected");
        assert_eq!(
            found[0].0.direction,
            Direction::East,
            "FacingComponent direction should be East"
        );
    }

    #[test]
    fn test_map_event_encounter_facing() {
        // Integration test: Encounter event with facing: Some(West) must produce a
        // creature entity whose FacingComponent stores Direction::West.
        use crate::domain::character::{AttributePair, AttributePair16};
        use crate::domain::combat::database::MonsterDefinition;
        use crate::domain::combat::monster::{LootTable as MDLootTable, MonsterResistances};
        use crate::domain::types::{Direction, Position};
        use crate::domain::world::MapEvent;
        use crate::game::components::creature::FacingComponent;

        let mut db = crate::sdk::database::ContentDatabase::new();

        // Build a MonsterDefinition with creature_id pre-set so add_monster converts it correctly.
        let monster_def = MonsterDefinition {
            id: 33,
            name: "Test Monster".to_string(),
            stats: crate::domain::character::Stats::new(10, 10, 10, 10, 10, 10, 5),
            hp: AttributePair16::new(10),
            ac: AttributePair::new(5),
            attacks: vec![],
            flee_threshold: 0,
            special_attack_threshold: 0,
            resistances: MonsterResistances::new(),
            can_regenerate: false,
            can_advance: false,
            is_undead: false,
            magic_resistance: 0,
            loot: MDLootTable::new(0, 0, 0, 0, 0),
            creature_id: Some(20),
            conditions: crate::domain::combat::monster::MonsterCondition::Normal,
            active_conditions: vec![],
            has_acted: false,
        };
        db.monsters.add_monster(monster_def).expect("add monster");
        db.creatures
            .add_creature(make_creature_def(20))
            .expect("add creature");

        let mut app = make_spawn_app(db);

        let mut game_state = crate::application::GameState::new();
        let mut map = crate::domain::world::Map::new(1, "T".to_string(), "D".to_string(), 10, 10);

        let enc_pos = Position::new(4, 4);
        map.events.insert(
            enc_pos,
            MapEvent::Encounter {
                name: "Test Encounter".to_string(),
                description: "desc".to_string(),
                monster_group: vec![33],
                time_condition: None,
                facing: Some(Direction::West),
                proximity_facing: false,
                rotation_speed: None,
                combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
            },
        );

        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(crate::game::resources::GlobalState(game_state));
        app.update();

        let world_ref = app.world_mut();
        let mut query = world_ref.query::<(&FacingComponent, &EncounterVisualMarker)>();
        let results: Vec<_> = query.iter(&*world_ref).collect();
        assert_eq!(results.len(), 1, "one encounter visual expected");
        assert_eq!(
            results[0].0.direction,
            Direction::West,
            "Encounter FacingComponent must store West"
        );
    }

    #[test]
    fn test_map_event_sign_facing() {
        // Integration test: Sign event with facing: Some(South) must produce an entity
        // whose FacingComponent stores Direction::South.
        use crate::domain::types::{Direction, Position};
        use crate::domain::world::MapEvent;
        use crate::game::components::creature::FacingComponent;

        let db = crate::sdk::database::ContentDatabase::new();
        let mut app = make_spawn_app(db);

        let mut game_state = crate::application::GameState::new();
        let mut map = crate::domain::world::Map::new(1, "T".to_string(), "D".to_string(), 10, 10);

        let sign_pos = Position::new(2, 2);
        map.events.insert(
            sign_pos,
            MapEvent::Sign {
                name: "South Sign".to_string(),
                description: "desc".to_string(),
                text: "Facing South".to_string(),
                time_condition: None,
                facing: Some(Direction::South),
            },
        );

        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(crate::game::resources::GlobalState(game_state));
        app.update();

        let world_ref = app.world_mut();
        let mut query = world_ref.query::<(&FacingComponent, &TileCoord)>();
        let results: Vec<_> = query
            .iter(&*world_ref)
            .filter(|(_, tc)| tc.0 == sign_pos)
            .collect();
        assert_eq!(results.len(), 1, "one sign entity at sign_pos expected");
        assert_eq!(
            results[0].0.direction,
            Direction::South,
            "Sign FacingComponent must store South"
        );
    }

    #[test]
    fn test_map_event_ron_round_trip() {
        // Serialize a RecruitableCharacter event with facing: Some(North) to RON and parse back.
        use crate::domain::types::Direction;
        use crate::domain::world::MapEvent;

        let event = MapEvent::RecruitableCharacter {
            name: "Round-Trip NPC".to_string(),
            description: "desc".to_string(),
            character_id: "npc_round_trip".to_string(),
            dialogue_id: Some(42),
            time_condition: None,
            facing: Some(Direction::North),
        };

        let ron_str = ron::to_string(&event).expect("serialize to RON");
        let parsed: MapEvent = ron::from_str(&ron_str).expect("parse from RON");

        assert_eq!(
            event, parsed,
            "RON round-trip must preserve all fields including facing"
        );
    }

    #[test]
    fn test_map_event_ron_round_trip_no_facing() {
        // Existing RON without a facing field must deserialize with facing: None (backward compat).
        use crate::domain::world::MapEvent;

        // Minimal RON without `facing` key — serde(default) must supply None.
        let ron_str = r#"RecruitableCharacter(
            name: "Legacy NPC",
            description: "no facing",
            character_id: "npc_legacy",
            dialogue_id: None,
            time_condition: None,
        )"#;

        let parsed: MapEvent = ron::from_str(ron_str).expect("parse from RON");
        match parsed {
            MapEvent::RecruitableCharacter { facing, .. } => {
                assert_eq!(
                    facing, None,
                    "Missing facing field must default to None for backward compat"
                );
            }
            other => panic!("expected RecruitableCharacter, got {:?}", other),
        }
    }

    #[test]
    fn test_map_event_sign_ron_backward_compat_no_facing() {
        // Existing Sign RON without facing must still parse correctly.
        use crate::domain::world::MapEvent;

        let ron_str = r#"Sign(
            name: "Old Sign",
            description: "desc",
            text: "Hello",
            time_condition: None,
        )"#;

        let parsed: MapEvent = ron::from_str(ron_str).expect("parse from RON");
        match parsed {
            MapEvent::Sign { facing, .. } => {
                assert_eq!(facing, None, "Sign missing facing must default to None");
            }
            other => panic!("expected Sign, got {:?}", other),
        }
    }

    #[test]
    fn test_map_event_encounter_ron_backward_compat_no_facing() {
        // Existing Encounter RON without facing must still parse correctly.
        use crate::domain::world::MapEvent;

        let ron_str = r#"Encounter(
            name: "Old Encounter",
            description: "desc",
            monster_group: [1, 2],
            time_condition: None,
        )"#;

        let parsed: MapEvent = ron::from_str(ron_str).expect("parse from RON");
        match parsed {
            MapEvent::Encounter { facing, .. } => {
                assert_eq!(
                    facing, None,
                    "Encounter missing facing must default to None"
                );
            }
            other => panic!("expected Encounter, got {:?}", other),
        }
    }

    #[test]
    fn test_map_event_npc_dialogue_ron_backward_compat_no_facing() {
        // Existing NpcDialogue RON without facing must still parse correctly.
        use crate::domain::world::MapEvent;

        let ron_str = r#"NpcDialogue(
            name: "Old NPC",
            description: "desc",
            npc_id: "some_npc",
            time_condition: None,
        )"#;

        let parsed: MapEvent = ron::from_str(ron_str).expect("parse from RON");
        match parsed {
            MapEvent::NpcDialogue { facing, .. } => {
                assert_eq!(
                    facing, None,
                    "NpcDialogue missing facing must default to None"
                );
            }
            other => panic!("expected NpcDialogue, got {:?}", other),
        }
    }

    #[test]
    fn test_map_event_recruitable_character_facing() {
        // Integration test: RecruitableCharacter event with facing: Some(East) must produce
        // a creature entity whose FacingComponent stores Direction::East.
        use crate::domain::types::{Direction, Position};
        use crate::domain::world::MapEvent;
        use crate::game::components::creature::FacingComponent;

        let mut db = crate::sdk::database::ContentDatabase::new();

        let mut char_def = crate::domain::character_definition::CharacterDefinition::new(
            "facing_test_char".to_string(),
            "Facing Test Character".to_string(),
            "human".to_string(),
            "knight".to_string(),
            crate::domain::character::Sex::Male,
            crate::domain::character::Alignment::Good,
        );
        char_def.creature_id = Some(30);
        char_def.is_premade = true;
        db.characters.add_character(char_def).expect("add char");
        db.creatures
            .add_creature(make_creature_def(30))
            .expect("add creature");

        let mut app = make_spawn_app(db);

        let mut game_state = crate::application::GameState::new();
        let mut map = crate::domain::world::Map::new(1, "T".to_string(), "D".to_string(), 10, 10);

        let rc_pos = Position::new(6, 6);
        map.events.insert(
            rc_pos,
            MapEvent::RecruitableCharacter {
                name: "Facing Test Character".to_string(),
                description: "desc".to_string(),
                character_id: "facing_test_char".to_string(),
                dialogue_id: None,
                time_condition: None,
                facing: Some(Direction::East),
            },
        );

        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(crate::game::resources::GlobalState(game_state));
        app.update();

        let world_ref = app.world_mut();
        let mut query = world_ref.query::<(&FacingComponent, &RecruitableVisualMarker)>();
        let results: Vec<_> = query.iter(&*world_ref).collect();
        assert_eq!(results.len(), 1, "one recruitable visual expected");
        assert_eq!(
            results[0].0.direction,
            Direction::East,
            "RecruitableCharacter FacingComponent must store East"
        );
    }

    #[test]
    fn test_recruitable_spawn_uses_character_def_creature_id() {
        // Build an app with a CharacterDefinition that has creature_id: Some(N)
        // and a matching CreatureDefinition; trigger a RecruitableCharacter map event;
        // assert a CreatureVisual { creature_id: N } entity is spawned.
        use crate::domain::types::Position;
        use crate::game::components::creature::CreatureVisual;

        let mut db = crate::sdk::database::ContentDatabase::new();

        let mut char_def = crate::domain::character_definition::CharacterDefinition::new(
            "char_with_creature".to_string(),
            "Character With Creature".to_string(),
            "human".to_string(),
            "knight".to_string(),
            crate::domain::character::Sex::Male,
            crate::domain::character::Alignment::Good,
        );
        char_def.creature_id = Some(42);
        char_def.is_premade = true;
        db.characters.add_character(char_def).expect("add char");
        db.creatures
            .add_creature(make_creature_def(42))
            .expect("add creature");

        let mut app = make_spawn_app(db);

        let mut game_state = crate::application::GameState::new();
        let mut map = crate::domain::world::Map::new(1, "T".to_string(), "D".to_string(), 10, 10);

        let rc_pos = Position::new(3, 3);
        map.events.insert(
            rc_pos,
            crate::domain::world::MapEvent::RecruitableCharacter {
                name: "Character With Creature".to_string(),
                description: "desc".to_string(),
                character_id: "char_with_creature".to_string(),
                dialogue_id: None,
                time_condition: None,
                facing: None,
            },
        );

        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(crate::game::resources::GlobalState(game_state));
        app.update();

        let world_ref = app.world_mut();
        let mut query = world_ref.query::<(&CreatureVisual, &TileCoord)>();
        let results: Vec<_> = query.iter(&*world_ref).collect();
        assert_eq!(results.len(), 1, "expected one CreatureVisual spawned");
        assert_eq!(results[0].0.creature_id, 42);
        assert_eq!(results[0].1 .0, rc_pos);
    }

    #[test]
    fn test_recruitable_spawn_falls_back_to_sprite_when_no_creature_id() {
        // Same setup but creature_id: None; assert no CreatureVisual is spawned
        // and a RecruitableVisualMarker entity is present instead.
        use crate::domain::types::Position;
        use crate::game::components::creature::CreatureVisual;

        let mut db = crate::sdk::database::ContentDatabase::new();

        let char_def = crate::domain::character_definition::CharacterDefinition::new(
            "char_no_creature".to_string(),
            "Character Without Creature".to_string(),
            "human".to_string(),
            "knight".to_string(),
            crate::domain::character::Sex::Male,
            crate::domain::character::Alignment::Good,
        );
        // creature_id is None by default
        db.characters.add_character(char_def).expect("add char");

        let mut app = make_spawn_app(db);

        let mut game_state = crate::application::GameState::new();
        let mut map = crate::domain::world::Map::new(1, "T".to_string(), "D".to_string(), 10, 10);

        let rc_pos = Position::new(5, 5);
        map.events.insert(
            rc_pos,
            crate::domain::world::MapEvent::RecruitableCharacter {
                name: "Character Without Creature".to_string(),
                description: "desc".to_string(),
                character_id: "char_no_creature".to_string(),
                dialogue_id: None,
                time_condition: None,
                facing: None,
            },
        );

        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(crate::game::resources::GlobalState(game_state));
        app.update();

        let world_ref = app.world_mut();
        // No CreatureVisual should be spawned
        let mut cv_query = world_ref.query::<&CreatureVisual>();
        let cv_results: Vec<_> = cv_query.iter(&*world_ref).collect();
        assert_eq!(
            cv_results.len(),
            0,
            "no CreatureVisual when creature_id is None"
        );

        // A RecruitableVisualMarker sprite entity should be present (sprite fallback)
        let mut rv_query = world_ref.query::<(&RecruitableVisualMarker, &TileCoord)>();
        let rv_results: Vec<_> = rv_query.iter(&*world_ref).collect();
        assert_eq!(rv_results.len(), 1, "expected sprite fallback entity");
        assert_eq!(rv_results[0].1 .0, rc_pos);
    }

    // ===== Phase 3: Runtime Facing Change System tests =====

    /// `test_proximity_facing_inserted_on_encounter_with_flag` – when
    /// `MapEvent::Encounter` has `proximity_facing: true`, the spawned entity
    /// must carry a `ProximityFacing` component with `trigger_distance == 2`.
    #[test]
    fn test_proximity_facing_inserted_on_encounter_with_flag() {
        use crate::domain::types::{Direction, Position};
        use crate::domain::world::MapEvent;
        use crate::game::systems::facing::ProximityFacing;

        let mut db = crate::sdk::database::ContentDatabase::new();
        db.monsters
            .add_monster(crate::domain::combat::database::MonsterDefinition {
                id: 50,
                name: "Proximity Goblin".to_string(),
                stats: crate::domain::character::Stats::new(8, 6, 6, 8, 10, 8, 5),
                hp: crate::domain::character::AttributePair16::new(8),
                ac: crate::domain::character::AttributePair::new(5),
                attacks: vec![],
                flee_threshold: 0,
                special_attack_threshold: 0,
                resistances: crate::domain::combat::monster::MonsterResistances::new(),
                can_regenerate: false,
                can_advance: false,
                is_undead: false,
                magic_resistance: 0,
                loot: crate::domain::combat::monster::LootTable::default(),
                creature_id: Some(55),
                conditions: crate::domain::combat::monster::MonsterCondition::Normal,
                active_conditions: vec![],
                has_acted: false,
            })
            .expect("add monster");
        db.creatures
            .add_creature(make_creature_def(55))
            .expect("add creature");

        let mut app = make_spawn_app(db);

        let mut game_state = crate::application::GameState::new();
        let mut map = crate::domain::world::Map::new(1, "T".to_string(), "D".to_string(), 10, 10);
        let enc_pos = Position::new(3, 3);
        map.events.insert(
            enc_pos,
            MapEvent::Encounter {
                name: "Proximity Goblins".to_string(),
                description: "desc".to_string(),
                monster_group: vec![50],
                time_condition: None,
                facing: Some(Direction::South),
                proximity_facing: true, // Phase 3: enable proximity tracking
                rotation_speed: None,
                combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
            },
        );

        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(crate::game::resources::GlobalState(game_state));
        app.update();

        let world_ref = app.world_mut();
        let mut query = world_ref.query::<(&ProximityFacing, &EncounterVisualMarker)>();
        let results: Vec<_> = query.iter(&*world_ref).collect();
        assert_eq!(
            results.len(),
            1,
            "encounter with proximity_facing:true must have ProximityFacing component"
        );
        assert_eq!(
            results[0].0.trigger_distance, 2,
            "default trigger_distance must be 2"
        );
    }

    /// `test_proximity_facing_not_inserted_when_flag_false` – when
    /// `MapEvent::Encounter` has `proximity_facing: false` (default), no
    /// `ProximityFacing` component must be present.
    #[test]
    fn test_proximity_facing_not_inserted_when_flag_false() {
        use crate::domain::world::MapEvent;
        use crate::game::systems::facing::ProximityFacing;

        let mut db = crate::sdk::database::ContentDatabase::new();
        db.monsters
            .add_monster(crate::domain::combat::database::MonsterDefinition {
                id: 51,
                name: "Static Goblin".to_string(),
                stats: crate::domain::character::Stats::new(8, 6, 6, 8, 10, 8, 5),
                hp: crate::domain::character::AttributePair16::new(8),
                ac: crate::domain::character::AttributePair::new(5),
                attacks: vec![],
                flee_threshold: 0,
                special_attack_threshold: 0,
                resistances: crate::domain::combat::monster::MonsterResistances::new(),
                can_regenerate: false,
                can_advance: false,
                is_undead: false,
                magic_resistance: 0,
                loot: crate::domain::combat::monster::LootTable::default(),
                creature_id: Some(56),
                conditions: crate::domain::combat::monster::MonsterCondition::Normal,
                active_conditions: vec![],
                has_acted: false,
            })
            .expect("add monster");
        db.creatures
            .add_creature(make_creature_def(56))
            .expect("add creature");

        let mut app = make_spawn_app(db);

        let mut game_state = crate::application::GameState::new();
        let mut map = crate::domain::world::Map::new(1, "T".to_string(), "D".to_string(), 10, 10);
        let enc_pos = crate::domain::types::Position::new(4, 4);
        map.events.insert(
            enc_pos,
            MapEvent::Encounter {
                name: "Static Goblins".to_string(),
                description: "desc".to_string(),
                monster_group: vec![51],
                time_condition: None,
                facing: None,
                proximity_facing: false, // default – no component
                rotation_speed: None,
                combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
            },
        );

        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(crate::game::resources::GlobalState(game_state));
        app.update();

        let world_ref = app.world_mut();
        let mut query = world_ref.query::<&ProximityFacing>();
        let results: Vec<_> = query.iter(&*world_ref).collect();
        assert_eq!(
            results.len(),
            0,
            "encounter with proximity_facing:false must NOT have ProximityFacing"
        );
    }

    /// `test_proximity_facing_npc_inserted_when_flag_set` – NPC with an
    /// `NpcDialogue` event whose `proximity_facing: true` must have a
    /// `ProximityFacing` component after spawn.
    #[test]
    fn test_proximity_facing_npc_inserted_when_flag_set() {
        use crate::domain::types::Position;
        use crate::domain::world::{npc::NpcDefinition, MapEvent};
        use crate::game::systems::facing::ProximityFacing;

        let mut db = crate::sdk::database::ContentDatabase::new();
        let mut npc_def = NpcDefinition::new("npc_prox_test", "Proximity NPC", "portrait");
        npc_def.creature_id = Some(60);
        db.npcs.add_npc(npc_def).expect("add npc");
        db.creatures
            .add_creature(make_creature_def(60))
            .expect("add creature");

        let mut app = make_spawn_app(db);

        let mut game_state = crate::application::GameState::new();
        let mut map = crate::domain::world::Map::new(1, "T".to_string(), "D".to_string(), 10, 10);

        // Place NPC via placement
        let npc_pos = Position::new(5, 5);
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "npc_prox_test",
                npc_pos,
            ));

        // Add NpcDialogue event at the same tile with proximity_facing: true
        map.events.insert(
            npc_pos,
            MapEvent::NpcDialogue {
                name: "Proximity NPC".to_string(),
                description: "desc".to_string(),
                npc_id: "npc_prox_test".to_string(),
                time_condition: None,
                facing: None,
                proximity_facing: true, // Phase 3: insert ProximityFacing
                rotation_speed: None,
            },
        );

        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(crate::game::resources::GlobalState(game_state));
        app.update();

        let world_ref = app.world_mut();
        let mut query = world_ref.query::<(&ProximityFacing, &NpcMarker)>();
        let results: Vec<_> = query.iter(&*world_ref).collect();
        assert_eq!(
            results.len(),
            1,
            "NPC with proximity_facing:true must have ProximityFacing component"
        );
        assert_eq!(
            results[0].0.trigger_distance, 2,
            "default NPC trigger_distance must be 2"
        );
        assert_eq!(results[0].1.npc_id, "npc_prox_test");
    }

    /// Backward compatibility: existing RON `MapEvent::Encounter` without
    /// `proximity_facing` must deserialize with `proximity_facing: false`.
    #[test]
    fn test_map_event_encounter_ron_backward_compat_no_proximity_facing() {
        use crate::domain::world::MapEvent;

        let ron_str = r#"Encounter(
            name: "Old Encounter",
            description: "desc",
            monster_group: [1, 2],
            time_condition: None,
        )"#;

        let parsed: MapEvent = ron::from_str(ron_str).expect("parse from RON");
        match parsed {
            MapEvent::Encounter {
                proximity_facing, ..
            } => {
                assert!(
                    !proximity_facing,
                    "Encounter missing proximity_facing must default to false"
                );
            }
            other => panic!("expected Encounter, got {:?}", other),
        }
    }

    /// Backward compatibility: existing RON `MapEvent::NpcDialogue` without
    /// `proximity_facing` must deserialize with `proximity_facing: false`.
    #[test]
    fn test_map_event_npc_dialogue_ron_backward_compat_no_proximity_facing() {
        use crate::domain::world::MapEvent;

        let ron_str = r#"NpcDialogue(
            name: "Old NPC",
            description: "desc",
            npc_id: "some_npc",
            time_condition: None,
        )"#;

        let parsed: MapEvent = ron::from_str(ron_str).expect("parse from RON");
        match parsed {
            MapEvent::NpcDialogue {
                proximity_facing, ..
            } => {
                assert!(
                    !proximity_facing,
                    "NpcDialogue missing proximity_facing must default to false"
                );
            }
            other => panic!("expected NpcDialogue, got {:?}", other),
        }
    }

    /// RON round-trip: `proximity_facing: true` survives serialise/deserialise.
    #[test]
    fn test_map_event_encounter_ron_round_trip_proximity_facing() {
        use crate::domain::world::MapEvent;

        let event = MapEvent::Encounter {
            name: "Proximity Test".to_string(),
            description: "desc".to_string(),
            monster_group: vec![1],
            time_condition: None,
            facing: None,
            proximity_facing: true,
            rotation_speed: None,
            combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
        };

        let ron_str = ron::to_string(&event).expect("serialize to RON");
        let parsed: MapEvent = ron::from_str(&ron_str).expect("parse from RON");

        assert_eq!(
            event, parsed,
            "RON round-trip must preserve proximity_facing: true"
        );
    }

    /// Backward compatibility: existing RON `MapEvent::Encounter` without
    /// `rotation_speed` must deserialize with `rotation_speed: None`.
    #[test]
    fn test_map_event_encounter_ron_backward_compat_no_rotation_speed() {
        use crate::domain::world::MapEvent;

        let ron_str = r#"Encounter(
            name: "Legacy Encounter",
            description: "desc",
            monster_group: [1],
            time_condition: None,
            proximity_facing: true,
        )"#;

        let parsed: MapEvent = ron::from_str(ron_str).expect("parse from RON");
        match parsed {
            MapEvent::Encounter { rotation_speed, .. } => {
                assert!(
                    rotation_speed.is_none(),
                    "Encounter missing rotation_speed must default to None (snap)"
                );
            }
            other => panic!("expected Encounter, got {:?}", other),
        }
    }

    /// Backward compatibility: existing RON `MapEvent::NpcDialogue` without
    /// `rotation_speed` must deserialize with `rotation_speed: None`.
    #[test]
    fn test_map_event_npc_dialogue_ron_backward_compat_no_rotation_speed() {
        use crate::domain::world::MapEvent;

        let ron_str = r#"NpcDialogue(
            name: "Legacy NPC",
            description: "desc",
            npc_id: "old_npc",
            time_condition: None,
            proximity_facing: true,
        )"#;

        let parsed: MapEvent = ron::from_str(ron_str).expect("parse from RON");
        match parsed {
            MapEvent::NpcDialogue { rotation_speed, .. } => {
                assert!(
                    rotation_speed.is_none(),
                    "NpcDialogue missing rotation_speed must default to None (snap)"
                );
            }
            other => panic!("expected NpcDialogue, got {:?}", other),
        }
    }

    /// RON round-trip: `rotation_speed: Some(90.0)` on `Encounter` survives
    /// serialise/deserialise with the exact value preserved.
    #[test]
    fn test_map_event_encounter_ron_round_trip_rotation_speed_some() {
        use crate::domain::world::MapEvent;

        let event = MapEvent::Encounter {
            name: "Smooth Encounter".to_string(),
            description: "desc".to_string(),
            monster_group: vec![1],
            time_condition: None,
            facing: None,
            proximity_facing: true,
            rotation_speed: Some(90.0),
            combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
        };

        let ron_str = ron::to_string(&event).expect("serialize to RON");
        let parsed: MapEvent = ron::from_str(&ron_str).expect("parse from RON");

        match parsed {
            MapEvent::Encounter { rotation_speed, .. } => {
                assert_eq!(
                    rotation_speed,
                    Some(90.0),
                    "RON round-trip must preserve rotation_speed: Some(90.0)"
                );
            }
            other => panic!("expected Encounter, got {:?}", other),
        }
    }

    /// RON round-trip: `rotation_speed: Some(180.0)` on `NpcDialogue` survives
    /// serialise/deserialise with the exact value preserved.
    #[test]
    fn test_map_event_npc_dialogue_ron_round_trip_rotation_speed_some() {
        use crate::domain::world::MapEvent;

        let event = MapEvent::NpcDialogue {
            name: "Smooth NPC".to_string(),
            description: "desc".to_string(),
            npc_id: "smooth_guard".to_string(),
            time_condition: None,
            facing: None,
            proximity_facing: true,
            rotation_speed: Some(180.0),
        };

        let ron_str = ron::to_string(&event).expect("serialize to RON");
        let parsed: MapEvent = ron::from_str(&ron_str).expect("parse from RON");

        match parsed {
            MapEvent::NpcDialogue { rotation_speed, .. } => {
                assert_eq!(
                    rotation_speed,
                    Some(180.0),
                    "RON round-trip must preserve rotation_speed: Some(180.0)"
                );
            }
            other => panic!("expected NpcDialogue, got {:?}", other),
        }
    }

    /// Integration: `rotation_speed: Some(speed)` on a proximity-facing
    /// `MapEvent::Encounter` must be forwarded into the `ProximityFacing`
    /// component on the spawned encounter entity.
    #[test]
    fn test_proximity_facing_rotation_speed_forwarded_on_encounter() {
        use crate::domain::types::{Direction, Position};
        use crate::domain::world::MapEvent;
        use crate::game::systems::facing::ProximityFacing;

        let mut db = crate::sdk::database::ContentDatabase::new();
        db.monsters
            .add_monster(crate::domain::combat::database::MonsterDefinition {
                id: 70,
                name: "Smooth Goblin".to_string(),
                stats: crate::domain::character::Stats::new(8, 6, 6, 8, 10, 8, 5),
                hp: crate::domain::character::AttributePair16::new(8),
                ac: crate::domain::character::AttributePair::new(5),
                attacks: vec![],
                flee_threshold: 0,
                special_attack_threshold: 0,
                resistances: crate::domain::combat::monster::MonsterResistances::new(),
                can_regenerate: false,
                can_advance: false,
                is_undead: false,
                magic_resistance: 0,
                loot: crate::domain::combat::monster::LootTable::default(),
                creature_id: Some(75),
                conditions: crate::domain::combat::monster::MonsterCondition::Normal,
                active_conditions: vec![],
                has_acted: false,
            })
            .expect("add monster");
        db.creatures
            .add_creature(make_creature_def(75))
            .expect("add creature");

        let mut app = make_spawn_app(db);

        let mut game_state = crate::application::GameState::new();
        let mut map = crate::domain::world::Map::new(1, "T".to_string(), "D".to_string(), 10, 10);
        let enc_pos = Position::new(4, 4);
        map.events.insert(
            enc_pos,
            MapEvent::Encounter {
                name: "Smooth Goblins".to_string(),
                description: "desc".to_string(),
                monster_group: vec![70],
                time_condition: None,
                facing: Some(Direction::South),
                proximity_facing: true,
                rotation_speed: Some(90.0), // Phase 4: smooth rotation
                combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
            },
        );

        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(crate::game::resources::GlobalState(game_state));
        app.update();

        let world_ref = app.world_mut();
        let mut query = world_ref.query::<(&ProximityFacing, &EncounterVisualMarker)>();
        let results: Vec<_> = query.iter(&*world_ref).collect();
        assert_eq!(
            results.len(),
            1,
            "encounter with proximity_facing:true must have ProximityFacing component"
        );
        assert_eq!(
            results[0].0.rotation_speed,
            Some(90.0),
            "rotation_speed from MapEvent::Encounter must be forwarded into ProximityFacing"
        );
    }

    /// Integration: `rotation_speed: Some(speed)` on a proximity-facing
    /// `MapEvent::NpcDialogue` must be forwarded into the `ProximityFacing`
    /// component on the spawned NPC entity.
    #[test]
    fn test_proximity_facing_rotation_speed_forwarded_on_npc_dialogue() {
        use crate::domain::types::Position;
        use crate::domain::world::{npc::NpcDefinition, MapEvent};
        use crate::game::systems::facing::ProximityFacing;

        let mut db = crate::sdk::database::ContentDatabase::new();
        let mut npc_def = NpcDefinition::new("npc_smooth_test", "Smooth NPC", "portrait");
        npc_def.creature_id = Some(80);
        db.npcs.add_npc(npc_def).expect("add npc");
        db.creatures
            .add_creature(make_creature_def(80))
            .expect("add creature");

        let mut app = make_spawn_app(db);

        let mut game_state = crate::application::GameState::new();
        let mut map = crate::domain::world::Map::new(1, "T".to_string(), "D".to_string(), 10, 10);

        let npc_pos = Position::new(6, 6);
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "npc_smooth_test",
                npc_pos,
            ));

        map.events.insert(
            npc_pos,
            MapEvent::NpcDialogue {
                name: "Smooth NPC Dialogue".to_string(),
                description: "desc".to_string(),
                npc_id: "npc_smooth_test".to_string(),
                time_condition: None,
                facing: None,
                proximity_facing: true,
                rotation_speed: Some(180.0), // Phase 4: smooth rotation
            },
        );

        game_state.world.add_map(map);
        game_state.world.set_current_map(1);
        app.insert_resource(crate::game::resources::GlobalState(game_state));
        app.update();

        let world_ref = app.world_mut();
        let mut query = world_ref.query::<(&ProximityFacing, &NpcMarker)>();
        let results: Vec<_> = query.iter(&*world_ref).collect();
        assert_eq!(
            results.len(),
            1,
            "NPC with proximity_facing:true must have ProximityFacing component"
        );
        assert_eq!(
            results[0].0.rotation_speed,
            Some(180.0),
            "rotation_speed from MapEvent::NpcDialogue must be forwarded into ProximityFacing"
        );
        assert_eq!(results[0].1.npc_id, "npc_smooth_test");
    }

    /// Phase 1: A map transition via `MapChangeEvent` must advance the in-game
    /// clock by exactly `TIME_COST_MAP_TRANSITION_MINUTES`.
    ///
    /// Strategy: build a minimal Bevy app with `MapManagerPlugin`, wire up two
    /// maps in `GlobalState`, send a `MapChangeEvent` targeting the second map,
    /// run one update frame so `map_change_handler` executes, and assert the
    /// clock advanced by the expected amount.
    #[test]
    fn test_map_transition_advances_time() {
        use crate::domain::resources::TIME_COST_MAP_TRANSITION_MINUTES;
        use crate::domain::types::Position;
        use crate::domain::world::{Map, World};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<bevy::prelude::Image>();
        app.add_plugins(MapManagerPlugin);

        // Required resources for spawn_map_markers (called by MapManagerPlugin).
        app.insert_resource(crate::application::resources::GameContent::new(
            crate::sdk::database::ContentDatabase::new(),
        ));
        app.insert_resource(crate::game::resources::sprite_assets::SpriteAssets::default());
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.insert_resource(crate::game::resources::GrassQualitySettings::default());

        // Build a world with two maps: party starts on map 1.
        let mut world = World::new();
        world.add_map(Map::new(
            1,
            "Map One".to_string(),
            "First".to_string(),
            10,
            10,
        ));
        world.add_map(Map::new(
            2,
            "Map Two".to_string(),
            "Second".to_string(),
            10,
            10,
        ));
        world.set_current_map(1);
        world.set_party_position(Position::new(5, 5));

        let mut gs = crate::application::GameState::new();
        gs.world = world;

        // Record the starting total minutes.
        // Use total_days() so the cumulative-minute baseline is correct across
        // month/year boundaries (day is now 1–30 within-month, not a running total).
        let start_minutes = gs.time.total_days() as u64 * 24 * 60
            + gs.time.hour as u64 * 60
            + gs.time.minute as u64;

        app.insert_resource(crate::game::resources::GlobalState(gs));

        // Send a MapChangeEvent targeting map 2.
        {
            let mut msgs = app
                .world_mut()
                .get_resource_mut::<Messages<MapChangeEvent>>()
                .expect("MapChangeEvent message queue must exist after MapManagerPlugin");
            msgs.write(MapChangeEvent {
                target_map: 2,
                target_pos: Position::new(1, 1),
            });
        }

        // Run one frame — map_change_handler processes the event and advances time.
        app.update();

        let state = app
            .world()
            .resource::<crate::game::resources::GlobalState>();
        let end_minutes = state.0.time.total_days() as u64 * 24 * 60
            + state.0.time.hour as u64 * 60
            + state.0.time.minute as u64;

        assert_eq!(
            end_minutes - start_minutes,
            TIME_COST_MAP_TRANSITION_MINUTES as u64,
            "a map transition must advance the clock by exactly TIME_COST_MAP_TRANSITION_MINUTES ({} min)",
            TIME_COST_MAP_TRANSITION_MINUTES
        );

        // The active map must also have been updated.
        assert_eq!(
            state.0.world.current_map, 2,
            "world.current_map must be updated to the target map id"
        );
        assert_eq!(
            state.0.world.party_position,
            Position::new(1, 1),
            "world.party_position must be updated to the target position"
        );
    }

    /// Phase 1: An invalid MapChangeEvent (targeting a non-existent map) must
    /// NOT advance the in-game clock.
    ///
    /// The handler gracefully ignores unknown map ids, so no time should pass.
    #[test]
    fn test_invalid_map_transition_does_not_advance_time() {
        use crate::domain::types::Position;
        use crate::domain::world::{Map, World};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<bevy::prelude::Image>();
        app.add_plugins(MapManagerPlugin);

        app.insert_resource(crate::application::resources::GameContent::new(
            crate::sdk::database::ContentDatabase::new(),
        ));
        app.insert_resource(crate::game::resources::sprite_assets::SpriteAssets::default());
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.insert_resource(crate::game::resources::GrassQualitySettings::default());

        let mut world = World::new();
        world.add_map(Map::new(
            1,
            "Only Map".to_string(),
            "Only".to_string(),
            10,
            10,
        ));
        world.set_current_map(1);
        world.set_party_position(Position::new(5, 5));

        let mut gs = crate::application::GameState::new();
        gs.world = world;

        app.insert_resource(crate::game::resources::GlobalState(gs));

        let time_before = app
            .world()
            .resource::<crate::game::resources::GlobalState>()
            .0
            .time;

        // Send an event targeting map 999 — does not exist.
        {
            let mut msgs = app
                .world_mut()
                .get_resource_mut::<Messages<MapChangeEvent>>()
                .expect("MapChangeEvent message queue must exist");
            msgs.write(MapChangeEvent {
                target_map: 999,
                target_pos: Position::new(0, 0),
            });
        }

        app.update();

        let state = app
            .world()
            .resource::<crate::game::resources::GlobalState>();
        assert_eq!(
            state.0.time.minute, time_before.minute,
            "invalid map transition must not advance minutes"
        );
        assert_eq!(
            state.0.time.hour, time_before.hour,
            "invalid map transition must not advance hours"
        );
        assert_eq!(
            state.0.time.day, time_before.day,
            "invalid map transition must not advance days"
        );
    }

    #[test]
    fn test_map_event_furniture_ron_backward_compat_no_furniture_id() {
        // Existing Furniture RON without furniture_id must parse with furniture_id: None.
        use crate::domain::world::MapEvent;

        let ron_str = r#"Furniture(
            name: "Old Throne",
            furniture_type: Throne,
            rotation_y: Some(90.0),
            scale: 1.2,
        )"#;

        let parsed: MapEvent = ron::from_str(ron_str).expect("parse from RON");
        match parsed {
            MapEvent::Furniture { furniture_id, .. } => {
                assert_eq!(
                    furniture_id, None,
                    "Missing furniture_id must default to None for backward compat"
                );
            }
            other => panic!("expected Furniture, got {:?}", other),
        }
    }

    #[test]
    fn test_map_event_furniture_ron_round_trip_with_furniture_id() {
        // A Furniture event with furniture_id: Some(3) round-trips through RON correctly.
        use crate::domain::world::{FurnitureMaterial, FurnitureType, MapEvent};

        let event = MapEvent::Furniture {
            name: "Royal Throne".to_string(),
            furniture_id: Some(3),
            furniture_type: FurnitureType::Throne,
            rotation_y: Some(180.0),
            scale: 1.5,
            material: FurnitureMaterial::Gold,
            flags: crate::domain::world::FurnitureFlags {
                lit: false,
                locked: false,
                blocking: true,
            },
            color_tint: None,
            key_item_id: None,
        };

        let serialised = ron::to_string(&event).expect("serialize to RON");
        let parsed: MapEvent = ron::from_str(&serialised).expect("parse from RON");

        match parsed {
            MapEvent::Furniture {
                furniture_id,
                furniture_type,
                name,
                ..
            } => {
                assert_eq!(furniture_id, Some(3), "furniture_id must round-trip");
                assert_eq!(
                    furniture_type,
                    FurnitureType::Throne,
                    "furniture_type must round-trip"
                );
                assert_eq!(name, "Royal Throne", "name must round-trip");
            }
            other => panic!("expected Furniture, got {:?}", other),
        }
    }

    #[test]
    fn test_map_event_furniture_inline_fields_without_furniture_id() {
        // When furniture_id is None, inline fields are used as-is (pure backward compat).
        use crate::domain::world::{FurnitureMaterial, FurnitureType, MapEvent};

        let ron_str = r#"Furniture(
            name: "Stone Bench",
            furniture_type: Bench,
            material: Stone,
            scale: 0.8,
            flags: (lit: false, locked: false, blocking: false),
        )"#;

        let parsed: MapEvent = ron::from_str(ron_str).expect("parse from RON");
        match parsed {
            MapEvent::Furniture {
                furniture_id,
                furniture_type,
                material,
                scale,
                ..
            } => {
                assert_eq!(furniture_id, None, "No furniture_id → None");
                assert_eq!(furniture_type, FurnitureType::Bench);
                assert_eq!(material, FurnitureMaterial::Stone);
                assert!((scale - 0.8).abs() < f32::EPSILON);
            }
            other => panic!("expected Furniture, got {:?}", other),
        }
    }

    /// T4-LD1: Verify a `LockedDoorMarker` entity is spawned when `spawn_map` processes
    /// a map that contains a locked `LockedDoor` event.
    #[test]
    fn test_locked_door_marker_spawned_on_map_load() {
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<bevy::prelude::Image>();
        app.add_plugins(MapRenderingPlugin);

        let db = crate::sdk::database::ContentDatabase::new();
        app.insert_resource(crate::application::resources::GameContent::new(db));

        let map_id: crate::domain::types::MapId = 1;
        let door_pos = Position::new(3, 3);

        let mut map = Map::new(map_id, "Test".to_string(), "Desc".to_string(), 10, 10);
        map.add_event(
            door_pos,
            MapEvent::LockedDoor {
                name: "Castle Gate".to_string(),
                lock_id: "castle_gate".to_string(),
                key_item_id: None,
                initial_trap_chance: 0,
            },
        );
        // Seed lock states so the spawn check sees is_locked = true.
        map.init_lock_states();

        let mut game_state = crate::application::GameState::new();
        game_state.world.add_map(map);
        game_state.world.set_current_map(map_id);
        app.insert_resource(crate::game::resources::GlobalState(game_state));

        app.insert_resource(crate::game::resources::sprite_assets::SpriteAssets::default());
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());

        // First frame: spawn_map_system runs in Startup and should produce a marker.
        app.update();

        let world_ref = app.world_mut();
        let mut query = world_ref.query::<(&LockedDoorMarker, &TileCoord)>();
        let results: Vec<_> = query.iter(&*world_ref).collect();
        assert_eq!(
            results.len(),
            1,
            "Expected exactly one LockedDoorMarker entity"
        );
        assert_eq!(results[0].0.lock_id, "castle_gate");
        assert_eq!(results[0].1 .0, door_pos);
    }

    /// T4-LD2: Verify `cleanup_locked_door_markers` despawns the marker once the
    /// `LockedDoor` event is removed from the map (simulating a successful unlock).
    #[test]
    fn test_locked_door_marker_despawned_after_unlock() {
        use crate::domain::types::Position;
        use crate::domain::world::{Map, MapEvent};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<bevy::prelude::Image>();
        // Use MapManagerPlugin so cleanup_locked_door_markers is scheduled.
        app.add_plugins(MapManagerPlugin);

        let map_id: crate::domain::types::MapId = 1;
        let door_pos = Position::new(2, 5);
        let lock_id = "dungeon_gate".to_string();

        let mut map = Map::new(map_id, "Test".to_string(), "Desc".to_string(), 10, 10);
        map.add_event(
            door_pos,
            MapEvent::LockedDoor {
                name: "Dungeon Gate".to_string(),
                lock_id: lock_id.clone(),
                key_item_id: None,
                initial_trap_chance: 0,
            },
        );
        // Seed lock states so is_locked = true before the first frame.
        map.init_lock_states();

        let mut game_state = crate::application::GameState::new();
        game_state.world.add_map(map);
        game_state.world.set_current_map(map_id);
        app.insert_resource(crate::game::resources::GlobalState(game_state));

        // Resources required by spawn_map_markers (also registered by MapManagerPlugin).
        app.insert_resource(crate::application::resources::GameContent::new(
            crate::sdk::database::ContentDatabase::new(),
        ));
        app.insert_resource(crate::game::resources::sprite_assets::SpriteAssets::default());
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.insert_resource(crate::game::resources::GrassQualitySettings::default());

        // Spawn a LockedDoorMarker entity manually to simulate what spawn_map does.
        let marker_entity = app
            .world_mut()
            .spawn((
                LockedDoorMarker {
                    lock_id: lock_id.clone(),
                },
                TileCoord(door_pos),
                MapEntity(map_id),
            ))
            .id();

        // First frame: event is present and lock is locked — entity must survive.
        app.update();
        assert!(
            app.world().get_entity(marker_entity).is_ok(),
            "Entity should survive while the LockedDoor event is present and locked"
        );

        // Remove the locked door event (simulates a successful unlock action).
        {
            let mut gs = app
                .world_mut()
                .resource_mut::<crate::game::resources::GlobalState>();
            let map = gs.0.world.get_map_mut(map_id).expect("map must exist");
            map.remove_event(door_pos);
        }

        // Next frame: cleanup_locked_door_markers must despawn the marker.
        app.update();
        assert!(
            app.world().get_entity(marker_entity).is_err(),
            "Entity must be despawned after the LockedDoor event is removed"
        );
    }
}
