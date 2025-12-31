// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::domain::types::{MapId, Position};
use crate::domain::world::{Map, MapEvent, Npc, NpcPlacement, TerrainType, Tile, WallType};
use serde::Deserialize;
use std::collections::HashMap;

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
    pub npcs: Vec<NpcBlueprint>,
    #[serde(default)]
    pub npc_placements: Vec<NpcPlacementBlueprint>,
    #[serde(default)]
    pub exits: Vec<ExitBlueprint>,
    pub starting_position: Position,
}

#[derive(Debug, Deserialize)]
pub struct TileBlueprint {
    pub x: i32,
    pub y: i32,
    pub code: TileCode,
}

#[derive(Debug, Deserialize)]
pub enum EnvironmentType {
    Outdoor,
    Indoor,
    Dungeon,
    Cave,
}

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
    NpcDialogue(u16),
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

#[derive(Debug, Deserialize)]
pub struct NpcBlueprint {
    pub id: u16,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub position: Position,
    pub dialogue_id: Option<String>,
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

impl From<MapBlueprint> for Map {
    fn from(bp: MapBlueprint) -> Self {
        let mut tiles = Vec::with_capacity((bp.width * bp.height) as usize);

        // Sort tiles by y then x to ensure row-major order if needed,
        // but Map struct expects a flat vector.
        // If the input is sparse or unordered, we might need to be careful.
        // For now, assuming the input vector covers all tiles or we just push them.
        // However, Map::new initializes tiles. Here we are creating a Map from scratch.
        // If bp.tiles is not in order, the index calculation in Map::get_tile might be wrong
        // if we just push them.
        // But Map stores tiles as Vec<Tile>.
        // Let's assume bp.tiles contains all tiles.

        for tile_bp in bp.tiles {
            let (terrain, wall_type) = match tile_bp.code {
                TileCode::Floor => (TerrainType::Ground, WallType::None),
                TileCode::Wall => (TerrainType::Ground, WallType::Normal),
                TileCode::Door => (TerrainType::Ground, WallType::Door),
                TileCode::Forest => (TerrainType::Forest, WallType::None),
                TileCode::Grass => (TerrainType::Grass, WallType::None),
                TileCode::Water => (TerrainType::Water, WallType::None),
                TileCode::Lava => (TerrainType::Lava, WallType::None),
                TileCode::Swamp => (TerrainType::Swamp, WallType::None),
                TileCode::Stone => (TerrainType::Stone, WallType::None),
                TileCode::Dirt => (TerrainType::Dirt, WallType::None),
                TileCode::Mountain => (TerrainType::Mountain, WallType::None),
                TileCode::Torch => (TerrainType::Ground, WallType::Torch),
            };
            tiles.push(Tile::new(tile_bp.x, tile_bp.y, terrain, wall_type));
        }

        let mut events = HashMap::new();
        for bp_event in bp.events {
            let event = match bp_event.event_type {
                BlueprintEventType::Text(text) => MapEvent::Sign {
                    name: bp_event.name,
                    description: bp_event.description,
                    text,
                },
                BlueprintEventType::Treasure(loot) => {
                    let loot_ids: Vec<u8> = loot.iter().map(|l| l.item_id).collect();
                    MapEvent::Treasure {
                        name: bp_event.name,
                        description: bp_event.description,
                        loot: loot_ids,
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
                },
            };
            events.insert(bp_event.position, event);
        }

        let npcs = bp
            .npcs
            .into_iter()
            .map(|bp_npc| Npc {
                id: bp_npc.id,
                name: bp_npc.name,
                description: bp_npc.description,
                position: bp_npc.position,
                dialogue: bp_npc.dialogue_id.unwrap_or_else(|| "...".to_string()),
            })
            .collect();

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
            npcs,
            npc_placements,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::Direction;

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
            tiles: vec![TileBlueprint {
                x: 0,
                y: 0,
                code: TileCode::Floor,
            }],
            events: vec![],
            npcs: vec![],
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
        let map: Map = bp.into();

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
    fn test_legacy_npc_blueprint_conversion() {
        // Arrange
        let bp = MapBlueprint {
            id: 1,
            name: "Legacy Map".to_string(),
            description: "Map with old-style NPCs".to_string(),
            width: 10,
            height: 10,
            environment: EnvironmentType::Outdoor,
            tiles: vec![TileBlueprint {
                x: 0,
                y: 0,
                code: TileCode::Grass,
            }],
            events: vec![],
            npcs: vec![NpcBlueprint {
                id: 1,
                name: "Old NPC".to_string(),
                description: "Legacy NPC".to_string(),
                position: Position::new(3, 3),
                dialogue_id: Some("greeting".to_string()),
            }],
            npc_placements: vec![],
            exits: vec![],
            starting_position: Position::new(0, 0),
        };

        // Act
        let map: Map = bp.into();

        // Assert
        assert_eq!(map.npcs.len(), 1);
        assert_eq!(map.npcs[0].name, "Old NPC");
        assert_eq!(map.npcs[0].position, Position::new(3, 3));
        assert_eq!(map.npc_placements.len(), 0);
    }

    #[test]
    fn test_mixed_npc_formats() {
        // Arrange - map with both legacy NPCs and new placements
        let bp = MapBlueprint {
            id: 1,
            name: "Mixed Map".to_string(),
            description: "Both old and new NPCs".to_string(),
            width: 15,
            height: 15,
            environment: EnvironmentType::Dungeon,
            tiles: vec![TileBlueprint {
                x: 0,
                y: 0,
                code: TileCode::Stone,
            }],
            events: vec![],
            npcs: vec![NpcBlueprint {
                id: 99,
                name: "Legacy NPC".to_string(),
                description: "Old format".to_string(),
                position: Position::new(1, 1),
                dialogue_id: None,
            }],
            npc_placements: vec![NpcPlacementBlueprint {
                npc_id: "new_npc".to_string(),
                position: Position::new(10, 10),
                facing: Some(Direction::South),
                dialogue_override: None,
            }],
            exits: vec![],
            starting_position: Position::new(0, 0),
        };

        // Act
        let map: Map = bp.into();

        // Assert
        assert_eq!(map.npcs.len(), 1, "Should have 1 legacy NPC");
        assert_eq!(map.npc_placements.len(), 1, "Should have 1 new placement");
        assert_eq!(map.npcs[0].name, "Legacy NPC");
        assert_eq!(map.npc_placements[0].npc_id, "new_npc");
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
            npcs: vec![],
            npc_placements: vec![],
            exits: vec![],
            starting_position: Position::new(0, 0),
        };

        // Act
        let map: Map = bp.into();

        // Assert
        assert_eq!(map.npc_placements.len(), 0);
        assert_eq!(map.npcs.len(), 0);
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
            npcs: vec![],
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
        let map: Map = bp.into();

        // Assert
        assert_eq!(map.npc_placements.len(), 1);
        let placement = &map.npc_placements[0];
        assert_eq!(placement.npc_id, "innkeeper_mary");
        assert_eq!(placement.position, Position::new(12, 8));
        assert_eq!(placement.facing, Some(Direction::West));
        assert_eq!(placement.dialogue_override, Some(100));
    }

    #[test]
    fn test_integration_npc_blueprint_to_resolution() {
        // This integration test demonstrates the complete workflow:
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
            portrait_path: "merchant.png".to_string(),
            dialogue_id: Some(10),
            quest_ids: vec![],
            faction: Some("Merchants Guild".to_string()),
            is_merchant: true,
            is_innkeeper: false,
        };

        let guard = crate::domain::world::npc::NpcDefinition {
            id: "city_guard".to_string(),
            name: "City Guard".to_string(),
            description: "A vigilant guard".to_string(),
            portrait_path: "guard.png".to_string(),
            dialogue_id: Some(20),
            quest_ids: vec![],
            faction: Some("City Watch".to_string()),
            is_merchant: false,
            is_innkeeper: false,
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
            tiles: vec![TileBlueprint {
                x: 0,
                y: 0,
                code: TileCode::Grass,
            }],
            events: vec![],
            npcs: vec![],
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
        let map: Map = blueprint.into();

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
        assert_eq!(guard_resolved.faction, Some("City Watch".to_string()));
    }
}
