// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::domain::types::MapId;
use crate::domain::types::Position;
use crate::domain::types::TerrainId;
use crate::domain::world::terrain::{
    TerrainDatabase, TERRAIN_DIRT, TERRAIN_FOREST, TERRAIN_GRASS, TERRAIN_GROUND, TERRAIN_LAVA,
    TERRAIN_MOUNTAIN, TERRAIN_STONE, TERRAIN_SWAMP, TERRAIN_WATER,
};
use crate::domain::world::{Map, MapEvent, NpcPlacement, Tile, WallType};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub struct MapBlueprint {
    pub id: MapId,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub width: u32,
    pub height: u32,
    pub environment: EnvironmentType,
    pub tiles: Vec<TileBlueprint>,
    pub events: Vec<MapEventBlueprint>,
    #[serde(default)]
    pub npc_placements: Vec<NpcPlacementBlueprint>,
    #[serde(default)]
    pub exits: Vec<ExitBlueprint>,
    pub starting_position: Position,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TileBlueprint {
    /// Simple, compact blueprint form using a TileCode to select terrain/wall combination
    Code { x: i32, y: i32, code: TileCode },

    /// Full tile form: accepts the complete `Tile` structure from RON files.
    /// This allows older engine-style map files that serialize full `Tile` records.
    Full(Box<Tile>),
}

#[derive(Debug, Deserialize)]
pub enum EnvironmentType {
    Outdoor,
    Indoor,
    Dungeon,
    Cave,
}

/// Authoring shorthand for built-in terrain only. Custom campaign terrain
/// must be authored via full map RON (e.g. `terrain: 13105`), not via
/// `TileCode`.
#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum TileCode {
    Floor,
    Wall,
    Door,
    Forest,
    Grass,
    Water,
    Lava,
    Swamp,
    Stone,
    Dirt,
    Mountain,
    Torch,
}

#[derive(Debug, Deserialize)]
pub struct MapEventBlueprint {
    pub position: Position,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub event_type: BlueprintEventType,
}

#[derive(Debug, Deserialize)]
pub enum BlueprintEventType {
    Text(String),
    Treasure(Vec<LootItem>),
    Combat(Vec<MonsterSpawn>),
    Teleport { map_id: u16, x: i32, y: i32 },
    Trap { damage: u16, effect: Option<String> },
    NpcDialogue(String),
}

#[derive(Debug, Deserialize)]
pub struct MonsterSpawn {
    pub monster_id: u16,
    pub count: u16,
}

#[derive(Debug, Deserialize)]
pub struct LootItem {
    pub item_id: u8,
    pub quantity: u16,
}

/// Blueprint for NPC placement (references NPC definition by ID)
///
/// This is the new format for placing NPCs on maps. Instead of inline NPC data,
/// it references an NPC definition from the NPC database.
#[derive(Debug, Deserialize)]
pub struct NpcPlacementBlueprint {
    /// ID of the NPC definition in the database
    pub npc_id: String,
    /// Position on the map
    pub position: Position,
    /// Optional facing direction
    #[serde(default)]
    pub facing: Option<crate::domain::types::Direction>,
    /// Optional dialogue override (uses NPC's default if None)
    #[serde(default)]
    pub dialogue_override: Option<crate::domain::dialogue::DialogueId>,
}

#[derive(Debug, Deserialize)]
pub struct ExitBlueprint {
    pub map_id: u16,
    pub position: Position,
    pub target_position: Position,
}

impl MapBlueprint {
    /// Converts this blueprint into a [`Map`], seeding tile blocked flags from
    /// the supplied [`TerrainDatabase`].
    ///
    /// Pass [`builtin_terrain_db()`] when working with the standard built-in
    /// terrain set.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::blueprint::{MapBlueprint, TileBlueprint, TileCode, EnvironmentType};
    /// use antares::domain::world::terrain::builtin_terrain_db;
    /// use antares::domain::types::Position;
    ///
    /// let bp = MapBlueprint {
    ///     id: 1,
    ///     name: "Test".to_string(),
    ///     description: String::new(),
    ///     width: 2,
    ///     height: 2,
    ///     environment: EnvironmentType::Indoor,
    ///     tiles: vec![],
    ///     events: vec![],
    ///     npc_placements: vec![],
    ///     exits: vec![],
    ///     starting_position: Position::new(0, 0),
    /// };
    /// let db = builtin_terrain_db();
    /// let map = bp.into_map(&db);
    /// assert_eq!(map.id, 1);
    /// ```
    pub fn into_map(self, db: &TerrainDatabase) -> Map {
        let bp = self;
        let mut tiles = Vec::with_capacity((bp.width * bp.height) as usize);

        for tile_bp in bp.tiles {
            match tile_bp {
                TileBlueprint::Code { x, y, code } => {
                    let (terrain, wall_type): (TerrainId, WallType) = match code {
                        TileCode::Floor => (TERRAIN_GROUND, WallType::None),
                        TileCode::Wall => (TERRAIN_GROUND, WallType::Normal),
                        TileCode::Door => (TERRAIN_GROUND, WallType::Door),
                        TileCode::Forest => (TERRAIN_FOREST, WallType::None),
                        TileCode::Grass => (TERRAIN_GRASS, WallType::None),
                        TileCode::Water => (TERRAIN_WATER, WallType::None),
                        TileCode::Lava => (TERRAIN_LAVA, WallType::None),
                        TileCode::Swamp => (TERRAIN_SWAMP, WallType::None),
                        TileCode::Stone => (TERRAIN_STONE, WallType::None),
                        TileCode::Dirt => (TERRAIN_DIRT, WallType::None),
                        TileCode::Mountain => (TERRAIN_MOUNTAIN, WallType::None),
                        TileCode::Torch => (TERRAIN_GROUND, WallType::Torch),
                    };
                    tiles.push(Tile::new(x, y, terrain, wall_type, db));
                }
                TileBlueprint::Full(tile) => {
                    // Full tile provided by blueprint (engine-style map). Use it verbatim.
                    tiles.push(*tile);
                }
            }
        }

        let mut events = BTreeMap::new();
        for bp_event in bp.events {
            let event = match bp_event.event_type {
                BlueprintEventType::Text(text) => MapEvent::Sign {
                    name: bp_event.name,
                    description: bp_event.description,
                    text,
                    time_condition: None,
                    facing: None,
                    mesh_id: None,
                    dialogue_id: None,
                },
                BlueprintEventType::Treasure(loot) => {
                    let loot_ids: Vec<u8> = loot.iter().map(|l| l.item_id).collect();
                    MapEvent::Treasure {
                        name: bp_event.name,
                        description: bp_event.description,
                        loot: loot_ids,
                        mesh_id: None,
                        dialogue_id: None,
                    }
                }
                BlueprintEventType::Combat(spawns) => {
                    let mut group = Vec::new();
                    for spawn in spawns {
                        for _ in 0..spawn.count {
                            group.push(spawn.monster_id as u8);
                        }
                    }
                    MapEvent::Encounter {
                        name: bp_event.name,
                        description: bp_event.description,
                        monster_group: group,
                        time_condition: None,
                        facing: None,
                        proximity_facing: false,
                        rotation_speed: None,
                        combat_event_type: crate::domain::combat::types::CombatEventType::Normal,
                    }
                }
                BlueprintEventType::Teleport { map_id, x, y } => MapEvent::Teleport {
                    name: bp_event.name,
                    description: bp_event.description,
                    destination: Position::new(x, y),
                    map_id,
                },
                BlueprintEventType::Trap { damage, effect } => MapEvent::Trap {
                    name: bp_event.name,
                    description: bp_event.description,
                    damage,
                    effect,
                },
                BlueprintEventType::NpcDialogue(id) => MapEvent::NpcDialogue {
                    name: bp_event.name,
                    description: bp_event.description,
                    npc_id: id,
                    time_condition: None,
                    facing: None,
                    proximity_facing: false,
                    rotation_speed: None,
                },
            };
            events.insert(bp_event.position, event);
        }

        // Convert NPC placements from blueprint format
        let npc_placements = bp
            .npc_placements
            .into_iter()
            .map(|bp_placement| NpcPlacement {
                npc_id: bp_placement.npc_id,
                position: bp_placement.position,
                facing: bp_placement.facing,
                dialogue_override: bp_placement.dialogue_override,
            })
            .collect();

        Map {
            id: bp.id,
            name: bp.name,
            description: bp.description,
            width: bp.width,
            height: bp.height,
            tiles,
            events,
            encounter_table: None,
            allow_random_encounters: true,
            npc_placements,
            landscape_placements: Vec::new(),
            dropped_items: Vec::new(),
            lock_states: std::collections::HashMap::new(),
            is_outdoor: false,
            sky: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::Direction;
    use crate::domain::world::terrain::builtin_terrain_db;

    #[test]
    fn test_npc_placement_blueprint_conversion() {
        // Arrange
        let bp = MapBlueprint {
            id: 1,
            name: "Test Map".to_string(),
            description: "Test description".to_string(),
            width: 10,
            height: 10,
            environment: EnvironmentType::Indoor,
            tiles: vec![TileBlueprint::Code {
                x: 0,
                y: 0,
                code: TileCode::Floor,
            }],
            events: vec![],
            npc_placements: vec![
                NpcPlacementBlueprint {
                    npc_id: "merchant_1".to_string(),
                    position: Position::new(5, 5),
                    facing: Some(Direction::North),
                    dialogue_override: None,
                },
                NpcPlacementBlueprint {
                    npc_id: "guard_1".to_string(),
                    position: Position::new(8, 3),
                    facing: None,
                    dialogue_override: Some(42),
                },
            ],
            exits: vec![],
            starting_position: Position::new(0, 0),
        };

        // Act
        let map = bp.into_map(&builtin_terrain_db());

        // Assert
        assert_eq!(map.npc_placements.len(), 2);
        assert_eq!(map.npc_placements[0].npc_id, "merchant_1");
        assert_eq!(map.npc_placements[0].position, Position::new(5, 5));
        assert_eq!(map.npc_placements[0].facing, Some(Direction::North));
        assert_eq!(map.npc_placements[0].dialogue_override, None);

        assert_eq!(map.npc_placements[1].npc_id, "guard_1");
        assert_eq!(map.npc_placements[1].position, Position::new(8, 3));
        assert_eq!(map.npc_placements[1].facing, None);
        assert_eq!(map.npc_placements[1].dialogue_override, Some(42));
    }

    #[test]
    fn test_empty_npc_placements() {
        // Arrange
        let bp = MapBlueprint {
            id: 1,
            name: "Empty Map".to_string(),
            description: "No NPCs".to_string(),
            width: 5,
            height: 5,
            environment: EnvironmentType::Cave,
            tiles: vec![],
            events: vec![],
            npc_placements: vec![],
            exits: vec![],
            starting_position: Position::new(0, 0),
        };

        // Act
        let map = bp.into_map(&builtin_terrain_db());

        // Assert
        assert_eq!(map.npc_placements.len(), 0);
    }

    #[test]
    fn test_npc_placement_with_all_fields() {
        // Arrange
        let bp = MapBlueprint {
            id: 5,
            name: "Full NPC Test".to_string(),
            description: "Testing all NPC placement fields".to_string(),
            width: 20,
            height: 20,
            environment: EnvironmentType::Indoor,
            tiles: vec![],
            events: vec![],
            npc_placements: vec![NpcPlacementBlueprint {
                npc_id: "innkeeper_mary".to_string(),
                position: Position::new(12, 8),
                facing: Some(Direction::West),
                dialogue_override: Some(100),
            }],
            exits: vec![],
            starting_position: Position::new(1, 1),
        };

        // Act
        let map = bp.into_map(&builtin_terrain_db());

        // Assert
        assert_eq!(map.npc_placements.len(), 1);
        let placement = &map.npc_placements[0];
        assert_eq!(placement.npc_id, "innkeeper_mary");
        assert_eq!(placement.position, Position::new(12, 8));
        assert_eq!(placement.facing, Some(Direction::West));
        assert_eq!(placement.dialogue_override, Some(100));
    }

    #[test]
    fn test_tile_code_maps_to_builtin_terrain_ids() {
        use crate::domain::world::terrain::{
            builtin_terrain_db, TERRAIN_FOREST, TERRAIN_GROUND, TERRAIN_MOUNTAIN, TERRAIN_WATER,
        };
        let db = builtin_terrain_db();
        let make_tile = |code: TileCode| {
            let bp = MapBlueprint {
                id: 1,
                name: "t".to_string(),
                description: String::new(),
                width: 1,
                height: 1,
                environment: EnvironmentType::Indoor,
                tiles: vec![TileBlueprint::Code { x: 0, y: 0, code }],
                events: vec![],
                npc_placements: vec![],
                exits: vec![],
                starting_position: crate::domain::types::Position::new(0, 0),
            };
            bp.into_map(&db).tiles.into_iter().next().unwrap()
        };
        assert_eq!(make_tile(TileCode::Floor).terrain, TERRAIN_GROUND);
        assert_eq!(make_tile(TileCode::Water).terrain, TERRAIN_WATER);
        assert_eq!(make_tile(TileCode::Mountain).terrain, TERRAIN_MOUNTAIN);
        assert_eq!(make_tile(TileCode::Forest).terrain, TERRAIN_FOREST);
        assert!(make_tile(TileCode::Water).blocked);
        assert!(make_tile(TileCode::Mountain).blocked);
        assert!(!make_tile(TileCode::Floor).blocked);
    }

    #[test]
    fn test_integration_npc_blueprint_to_resolution() {
        // 1. Define NPCs in database
        // 2. Create map blueprint with NPC placements
        // 3. Convert blueprint to Map
        // 4. Resolve NPCs against database
        // 5. Verify resolved data is correct

        // Arrange - Create NPC database
        let mut npc_db = crate::sdk::database::NpcDatabase::new();

        let merchant = crate::domain::world::npc::NpcDefinition {
            id: "merchant_bob".to_string(),
            name: "Bob the Merchant".to_string(),
            description: "A friendly merchant".to_string(),
            portrait_id: "merchant.png".to_string(),
            dialogue_id: Some(10),
            creature_id: None,
            sprite: None,
            quest_ids: vec![],
            faction: Some("Merchants Guild".to_string()),
            is_merchant: true,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
            is_skill_trainer: false,
            trainable_skill_ids: vec![],
            skill_training_fee_base: None,
            skill_training_fee_multiplier: None,
            skill_training_max_rank: None,
            combat_switch: None,
            suppress_flag: None,
        };

        let guard = crate::domain::world::npc::NpcDefinition {
            id: "city_guard".to_string(),
            name: "City Guard".to_string(),
            description: "A vigilant guard".to_string(),
            portrait_id: "guard.png".to_string(),
            dialogue_id: Some(20),
            creature_id: None,
            sprite: None,
            quest_ids: vec![],
            faction: Some("City Watch".to_string()),
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
            is_skill_trainer: false,
            trainable_skill_ids: vec![],
            skill_training_fee_base: None,
            skill_training_fee_multiplier: None,
            skill_training_max_rank: None,
            combat_switch: None,
            suppress_flag: None,
        };

        npc_db.add_npc(merchant).unwrap();
        npc_db.add_npc(guard).unwrap();

        // Arrange - Create map blueprint with NPC placements
        let blueprint = MapBlueprint {
            id: 1,
            name: "Test Town".to_string(),
            description: "Integration test map".to_string(),
            width: 20,
            height: 20,
            environment: EnvironmentType::Outdoor,
            tiles: vec![TileBlueprint::Code {
                x: 0,
                y: 0,
                code: TileCode::Grass,
            }],
            events: vec![],
            npc_placements: vec![
                NpcPlacementBlueprint {
                    npc_id: "merchant_bob".to_string(),
                    position: Position::new(5, 5),
                    facing: Some(Direction::South),
                    dialogue_override: None,
                },
                NpcPlacementBlueprint {
                    npc_id: "city_guard".to_string(),
                    position: Position::new(10, 10),
                    facing: Some(Direction::North),
                    dialogue_override: Some(99), // Override guard's default dialogue
                },
            ],
            exits: vec![],
            starting_position: Position::new(0, 0),
        };

        // Act - Convert blueprint to Map
        let map = blueprint.into_map(&builtin_terrain_db());

        // Assert - Map has correct placements
        assert_eq!(map.npc_placements.len(), 2);
        assert_eq!(map.npc_placements[0].npc_id, "merchant_bob");
        assert_eq!(map.npc_placements[1].npc_id, "city_guard");

        // Act - Resolve NPCs against database
        let resolved = map.resolve_npcs(&npc_db);

        // Assert - Resolved NPCs have correct data
        assert_eq!(resolved.len(), 2);

        // Verify merchant
        let merchant_resolved = resolved
            .iter()
            .find(|n| n.npc_id == "merchant_bob")
            .expect("Merchant not found");
        assert_eq!(merchant_resolved.name, "Bob the Merchant");
        assert_eq!(merchant_resolved.position, Position::new(5, 5));
        assert_eq!(merchant_resolved.facing, Some(Direction::South));
        assert_eq!(merchant_resolved.dialogue_id, Some(10)); // Uses default
        assert!(merchant_resolved.is_merchant);
        assert_eq!(
            merchant_resolved.faction,
            Some("Merchants Guild".to_string())
        );

        // Verify guard
        let guard_resolved = resolved
            .iter()
            .find(|n| n.npc_id == "city_guard")
            .expect("Guard not found");
        assert_eq!(guard_resolved.name, "City Guard");
        assert_eq!(guard_resolved.position, Position::new(10, 10));
        assert_eq!(guard_resolved.facing, Some(Direction::North));
        assert_eq!(guard_resolved.dialogue_id, Some(99)); // Uses override, not default 20
        assert!(!guard_resolved.is_merchant);
    }
}
