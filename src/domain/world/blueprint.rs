// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::domain::types::{MapId, Position};
use crate::domain::world::{Map, MapEvent, Npc, TerrainType, Tile, WallType};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct MapBlueprint {
    pub id: MapId,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub environment: EnvironmentType,
    pub tiles: Vec<TileCode>,
    pub events: Vec<MapEventBlueprint>,
    #[serde(default)]
    pub npcs: Vec<NpcBlueprint>,
    #[serde(default)]
    pub exits: Vec<ExitBlueprint>,
    pub starting_position: Position,
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
    pub position: Position,
    pub dialogue_id: Option<String>,
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

        for code in bp.tiles {
            let (terrain, wall_type) = match code {
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
            tiles.push(Tile::new(terrain, wall_type));
        }

        let mut events = HashMap::new();
        for bp_event in bp.events {
            let event = match bp_event.event_type {
                BlueprintEventType::Text(text) => MapEvent::Sign { text },
                BlueprintEventType::Treasure(loot) => {
                    let loot_ids: Vec<u8> = loot.iter().map(|l| l.item_id).collect();
                    MapEvent::Treasure { loot: loot_ids }
                }
                BlueprintEventType::Combat(spawns) => {
                    let mut group = Vec::new();
                    for spawn in spawns {
                        for _ in 0..spawn.count {
                            group.push(spawn.monster_id as u8);
                        }
                    }
                    MapEvent::Encounter {
                        monster_group: group,
                    }
                }
                BlueprintEventType::Teleport { map_id, x, y } => MapEvent::Teleport {
                    destination: Position::new(x, y),
                    map_id,
                },
                BlueprintEventType::Trap { damage, effect } => MapEvent::Trap { damage, effect },
                BlueprintEventType::NpcDialogue(id) => MapEvent::NpcDialogue { npc_id: id },
            };
            events.insert(bp_event.position, event);
        }

        let npcs = bp
            .npcs
            .into_iter()
            .map(|bp_npc| Npc {
                id: bp_npc.id,
                name: bp_npc.name,
                position: bp_npc.position,
                dialogue: bp_npc.dialogue_id.unwrap_or_else(|| "...".to_string()),
            })
            .collect();

        Map {
            id: bp.id,
            width: bp.width,
            height: bp.height,
            tiles,
            events,
            npcs,
        }
    }
}
