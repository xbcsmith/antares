// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Core world data structures
//!
//! This module contains the fundamental types for the world system including
//! tiles, maps, NPCs, and the overall world structure.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.2 for complete specifications.

use crate::domain::types::{Direction, MapId, Position};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ===== Tile Types =====

/// Wall type for tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WallType {
    /// No wall
    None,
    /// Normal wall
    Normal,
    /// Door (can be opened)
    Door,
    /// Torch (light source)
    Torch,
}

/// Terrain type for tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainType {
    /// Normal walkable ground
    Ground,
    /// Grass terrain
    Grass,
    /// Water (may need special ability to cross)
    Water,
    /// Lava (damages party)
    Lava,
    /// Swamp (slows movement)
    Swamp,
    /// Stone floor
    Stone,
    /// Dirt path
    Dirt,
    /// Forest
    Forest,
    /// Mountain (blocked)
    Mountain,
}

/// A single tile in the game world
///
/// # Examples
///
/// ```
/// use antares::domain::world::{Tile, TerrainType, WallType};
///
/// let tile = Tile::new(0, 0, TerrainType::Ground, WallType::None);
/// assert!(!tile.blocked);
/// assert!(!tile.visited);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    /// Terrain type
    pub terrain: TerrainType,
    /// Wall type (None, Normal, Door, Torch)
    pub wall_type: WallType,
    /// Whether movement is blocked
    pub blocked: bool,
    /// Special tile (for events)
    pub is_special: bool,
    /// Dark area (requires light)
    pub is_dark: bool,
    /// Has been visited by party
    pub visited: bool,
    /// X coordinate
    pub x: i32,
    /// Y coordinate
    pub y: i32,
}

impl Tile {
    /// Creates a new tile with the given terrain and wall type
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{Tile, TerrainType, WallType};
    ///
    /// let tile = Tile::new(0, 0, TerrainType::Ground, WallType::None);
    /// assert_eq!(tile.terrain, TerrainType::Ground);
    /// assert_eq!(tile.wall_type, WallType::None);
    /// ```
    pub fn new(x: i32, y: i32, terrain: TerrainType, wall_type: WallType) -> Self {
        let blocked = matches!(terrain, TerrainType::Mountain | TerrainType::Water)
            || matches!(wall_type, WallType::Normal);

        Self {
            x,
            y,
            terrain,
            wall_type,
            blocked,
            is_special: false,
            is_dark: false,
            visited: false,
        }
    }

    /// Returns true if the tile blocks movement
    pub fn is_blocked(&self) -> bool {
        self.blocked
    }

    /// Returns true if the tile is a door
    pub fn is_door(&self) -> bool {
        self.wall_type == WallType::Door
    }

    /// Returns true if the tile has a light source (torch)
    pub fn has_light(&self) -> bool {
        self.wall_type == WallType::Torch
    }

    /// Marks the tile as visited
    pub fn mark_visited(&mut self) {
        self.visited = true;
    }
}

// ===== Map Event System =====

/// Map event types
///
/// Events are triggered when the party moves to specific tiles or interacts
/// with the environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MapEvent {
    /// Random monster encounter
    Encounter {
        /// Event name
        #[serde(default)]
        name: String,
        /// Event description
        #[serde(default)]
        description: String,
        /// Monster group IDs
        monster_group: Vec<u8>,
    },
    /// Treasure chest
    Treasure {
        /// Event name
        #[serde(default)]
        name: String,
        /// Event description
        #[serde(default)]
        description: String,
        /// Loot table or item IDs
        loot: Vec<u8>,
    },
    /// Teleport to another location
    Teleport {
        /// Event name
        #[serde(default)]
        name: String,
        /// Event description
        #[serde(default)]
        description: String,
        /// Destination position
        destination: Position,
        /// Target map ID
        map_id: MapId,
    },
    /// Trap that triggers
    Trap {
        /// Event name
        #[serde(default)]
        name: String,
        /// Event description
        #[serde(default)]
        description: String,
        /// Damage amount
        damage: u16,
        /// Status effect
        effect: Option<String>,
    },
    /// Sign with text
    Sign {
        /// Event name
        #[serde(default)]
        name: String,
        /// Event description
        #[serde(default)]
        description: String,
        /// Message text
        text: String,
    },
    /// NPC dialogue trigger
    NpcDialogue {
        /// Event name
        #[serde(default)]
        name: String,
        /// Event description
        #[serde(default)]
        description: String,
        /// NPC identifier
        npc_id: u16,
    },
}

// ===== NPC =====

/// Non-player character in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Npc {
    /// NPC identifier
    pub id: u16,
    /// NPC name
    pub name: String,
    /// NPC description
    #[serde(default)]
    pub description: String,
    /// Position on the map
    pub position: Position,
    /// Dialogue/interaction text
    pub dialogue: String,
}

impl Npc {
    /// Creates a new NPC
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::Npc;
    /// use antares::domain::types::Position;
    ///
    /// let npc = Npc::new(1, "Merchant".to_string(), "A friendly merchant".to_string(), Position::new(5, 5), "Welcome!".to_string());
    /// assert_eq!(npc.name, "Merchant");
    /// ```
    pub fn new(
        id: u16,
        name: String,
        description: String,
        position: Position,
        dialogue: String,
    ) -> Self {
        Self {
            id,
            name,
            description,
            position,
            dialogue,
        }
    }
}

// ===== Resolved NPC =====

/// Resolved NPC combining placement and definition data
///
/// This struct merges an NPC placement (position, facing, overrides) with
/// the NPC definition (name, portrait, dialogue, etc.) from the database.
/// It's used at runtime after loading maps and resolving NPC references.
///
/// # Examples
///
/// ```
/// use antares::domain::world::ResolvedNpc;
/// use antares::domain::world::npc::{NpcDefinition, NpcPlacement};
/// use antares::domain::types::Position;
///
/// let definition = NpcDefinition::new("merchant_1", "Bob", "merchant.png");
/// let placement = NpcPlacement::new("merchant_1", Position::new(10, 15));
/// let resolved = ResolvedNpc::from_placement_and_definition(&placement, &definition);
///
/// assert_eq!(resolved.npc_id, "merchant_1");
/// assert_eq!(resolved.name, "Bob");
/// assert_eq!(resolved.position, Position::new(10, 15));
/// ```
#[derive(Debug, Clone)]
pub struct ResolvedNpc {
    /// NPC ID from definition
    pub npc_id: String,
    /// NPC name from definition
    pub name: String,
    /// NPC description from definition
    pub description: String,
    /// Portrait path from definition
    pub portrait_path: String,
    /// Position from placement
    pub position: Position,
    /// Facing direction from placement
    pub facing: Option<Direction>,
    /// Effective dialogue ID (placement override or definition default)
    pub dialogue_id: Option<crate::domain::dialogue::DialogueId>,
    /// Quest IDs from definition
    pub quest_ids: Vec<crate::domain::quest::QuestId>,
    /// Faction from definition
    pub faction: Option<String>,
    /// Whether NPC is a merchant
    pub is_merchant: bool,
    /// Whether NPC is an innkeeper
    pub is_innkeeper: bool,
}

impl ResolvedNpc {
    /// Creates a ResolvedNpc from a placement and definition
    ///
    /// The dialogue_id uses the placement's override if present, otherwise
    /// falls back to the definition's default dialogue.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::ResolvedNpc;
    /// use antares::domain::world::npc::{NpcDefinition, NpcPlacement};
    /// use antares::domain::types::Position;
    ///
    /// let definition = NpcDefinition {
    ///     id: "guard".to_string(),
    ///     name: "City Guard".to_string(),
    ///     description: "A vigilant guard".to_string(),
    ///     portrait_path: "guard.png".to_string(),
    ///     dialogue_id: Some(10),
    ///     quest_ids: vec![],
    ///     faction: Some("City Watch".to_string()),
    ///     is_merchant: false,
    ///     is_innkeeper: false,
    /// };
    ///
    /// let placement = NpcPlacement::new("guard", Position::new(5, 5));
    ///
    /// let resolved = ResolvedNpc::from_placement_and_definition(&placement, &definition);
    /// assert_eq!(resolved.dialogue_id, Some(10));
    /// ```
    pub fn from_placement_and_definition(
        placement: &crate::domain::world::npc::NpcPlacement,
        definition: &crate::domain::world::npc::NpcDefinition,
    ) -> Self {
        Self {
            npc_id: definition.id.clone(),
            name: definition.name.clone(),
            description: definition.description.clone(),
            portrait_path: definition.portrait_path.clone(),
            position: placement.position,
            facing: placement.facing,
            dialogue_id: placement.dialogue_override.or(definition.dialogue_id),
            quest_ids: definition.quest_ids.clone(),
            faction: definition.faction.clone(),
            is_merchant: definition.is_merchant,
            is_innkeeper: definition.is_innkeeper,
        }
    }
}

// ===== Map =====

/// A map in the game world
///
/// Maps are 2D grids of tiles with events and NPCs.
///
/// # Examples
///
/// ```
/// use antares::domain::world::{Map, Tile, TerrainType, WallType};
///
/// let map = Map::new(1, "Test Map".to_string(), "Description".to_string(), 20, 20);
/// assert_eq!(map.width, 20);
/// assert_eq!(map.height, 20);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Map {
    /// Map identifier
    pub id: MapId,
    /// Map width in tiles
    pub width: u32,
    /// Map height in tiles
    pub height: u32,
    /// Map name
    #[serde(default = "default_map_name")]
    pub name: String,
    /// Map description
    #[serde(default)]
    pub description: String,
    /// 2D grid of tiles (row-major order: y * width + x)
    pub tiles: Vec<Tile>,
    /// Events at specific positions
    pub events: HashMap<Position, MapEvent>,
    /// NPCs on this map (legacy - use npc_placements)
    #[serde(default)]
    pub npcs: Vec<Npc>,
    /// NPC placements (references to NPC definitions)
    #[serde(default)]
    pub npc_placements: Vec<crate::domain::world::npc::NpcPlacement>,
}

fn default_map_name() -> String {
    "Unnamed Map".to_string()
}

impl Map {
    /// Creates a new map with the given given dimensions
    ///
    /// All tiles are initialized to ground terrain with no walls.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::Map;
    ///
    /// let map = Map::new(1, "Test Map".to_string(), "Description".to_string(), 10, 10);
    /// assert_eq!(map.width, 10);
    /// assert_eq!(map.height, 10);
    /// assert_eq!(map.tiles.len(), 100);
    /// ```
    pub fn new(id: MapId, name: String, description: String, width: u32, height: u32) -> Self {
        let mut tiles = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                tiles.push(Tile::new(
                    x as i32,
                    y as i32,
                    TerrainType::Ground,
                    WallType::None,
                ));
            }
        }

        Self {
            id,
            name,
            description,
            width,
            height,
            tiles,
            events: HashMap::new(),
            npcs: Vec::new(),
            npc_placements: Vec::new(),
        }
    }

    /// Gets a tile at the specified position
    ///
    /// Returns `None` if the position is out of bounds.
    pub fn get_tile(&self, pos: Position) -> Option<&Tile> {
        if self.is_valid_position(pos) {
            let index = (pos.y as usize * self.width as usize) + pos.x as usize;
            Some(&self.tiles[index])
        } else {
            None
        }
    }

    /// Gets a mutable reference to a tile at the specified position
    ///
    /// Returns `None` if the position is out of bounds.
    pub fn get_tile_mut(&mut self, pos: Position) -> Option<&mut Tile> {
        if self.is_valid_position(pos) {
            let index = (pos.y as usize * self.width as usize) + pos.x as usize;
            Some(&mut self.tiles[index])
        } else {
            None
        }
    }

    /// Returns true if the position is within map bounds
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::Map;
    /// use antares::domain::types::Position;
    ///
    /// let map = Map::new(1, "Test Map".to_string(), "Description".to_string(), 10, 10);
    /// assert!(map.is_valid_position(Position::new(5, 5)));
    /// assert!(!map.is_valid_position(Position::new(10, 10)));
    /// assert!(!map.is_valid_position(Position::new(-1, 5)));
    /// ```
    pub fn is_valid_position(&self, pos: Position) -> bool {
        pos.x >= 0 && pos.y >= 0 && pos.x < self.width as i32 && pos.y < self.height as i32
    }

    /// Returns true if the tile at the position is blocked
    ///
    /// This checks both tile blocking (walls, terrain) and NPC blocking.
    /// NPCs are considered blocking obstacles - the party cannot move through them.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{Map, Npc};
    /// use antares::domain::world::npc::NpcPlacement;
    /// use antares::domain::types::Position;
    ///
    /// let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
    ///
    /// // Position is not blocked initially
    /// assert!(!map.is_blocked(Position::new(5, 5)));
    ///
    /// // Add NPC placement at position
    /// map.npc_placements.push(NpcPlacement::new("guard", Position::new(5, 5)));
    ///
    /// // Now the position is blocked by the NPC
    /// assert!(map.is_blocked(Position::new(5, 5)));
    /// ```
    pub fn is_blocked(&self, pos: Position) -> bool {
        // Check tile blocking first
        if self.get_tile(pos).is_none_or(|tile| tile.is_blocked()) {
            return true;
        }

        // Check if any NPC placement occupies this position
        if self.npc_placements.iter().any(|npc| npc.position == pos) {
            return true;
        }

        // Check legacy NPCs (for backward compatibility)
        if self.npcs.iter().any(|npc| npc.position == pos) {
            return true;
        }

        false
    }

    /// Adds an event at the specified position
    pub fn add_event(&mut self, pos: Position, event: MapEvent) {
        self.events.insert(pos, event);
    }

    /// Gets an event at the specified position
    pub fn get_event(&self, pos: Position) -> Option<&MapEvent> {
        self.events.get(&pos)
    }

    /// Removes and returns an event at the specified position
    pub fn remove_event(&mut self, pos: Position) -> Option<MapEvent> {
        self.events.remove(&pos)
    }

    /// Gets the event at a specific position, if one exists
    ///
    /// # Arguments
    ///
    /// * `position` - The position to check for events
    ///
    /// # Returns
    ///
    /// Returns `Some(&MapEvent)` if an event exists at the position, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{Map, MapEvent};
    /// use antares::domain::types::Position;
    ///
    /// let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
    /// let pos = Position::new(5, 5);
    /// let event = MapEvent::Sign {
    ///     name: "Test".to_string(),
    ///     description: "A sign".to_string(),
    ///     text: "Hello!".to_string(),
    /// };
    /// map.add_event(pos, event);
    ///
    /// assert!(map.get_event_at_position(pos).is_some());
    /// assert!(map.get_event_at_position(Position::new(0, 0)).is_none());
    /// ```
    pub fn get_event_at_position(&self, position: Position) -> Option<&MapEvent> {
        self.get_event(position)
    }

    /// Adds an NPC to the map
    pub fn add_npc(&mut self, npc: Npc) {
        self.npcs.push(npc);
    }

    /// Resolves NPC placements using the NPC database
    ///
    /// This method takes the NPC placements on the map and resolves them
    /// against the NPC database to create `ResolvedNpc` instances that
    /// combine placement data (position, facing) with definition data
    /// (name, portrait, dialogue, etc.).
    ///
    /// NPCs that reference IDs not found in the database are skipped with
    /// a warning (in production, consider logging).
    ///
    /// # Arguments
    ///
    /// * `npc_db` - Reference to the NPC database
    ///
    /// # Returns
    ///
    /// Returns a vector of `ResolvedNpc` instances
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{Map, ResolvedNpc};
    /// use antares::domain::world::npc::{NpcDefinition, NpcPlacement};
    /// use antares::domain::types::Position;
    /// use antares::sdk::database::NpcDatabase;
    ///
    /// let mut npc_db = NpcDatabase::new();
    /// let npc_def = NpcDefinition::new("merchant_1", "Bob", "merchant.png");
    /// npc_db.add_npc(npc_def).unwrap();
    ///
    /// let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
    /// map.npc_placements.push(NpcPlacement::new("merchant_1", Position::new(5, 5)));
    ///
    /// let resolved = map.resolve_npcs(&npc_db);
    /// assert_eq!(resolved.len(), 1);
    /// assert_eq!(resolved[0].name, "Bob");
    /// ```
    pub fn resolve_npcs(&self, npc_db: &crate::sdk::database::NpcDatabase) -> Vec<ResolvedNpc> {
        self.npc_placements
            .iter()
            .filter_map(|placement| {
                if let Some(definition) = npc_db.get_npc(&placement.npc_id) {
                    Some(ResolvedNpc::from_placement_and_definition(
                        placement, definition,
                    ))
                } else {
                    // NPC definition not found in database
                    // In production, this should log a warning
                    eprintln!(
                        "Warning: NPC '{}' not found in database on map {}",
                        placement.npc_id, self.id
                    );
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod map_npc_resolution_tests {
    use super::*;
    use crate::domain::world::npc::{NpcDefinition, NpcPlacement};
    use crate::sdk::database::NpcDatabase;

    #[test]
    fn test_resolve_npcs_with_single_npc() {
        // Arrange
        let mut npc_db = NpcDatabase::new();
        let npc_def = NpcDefinition::new("merchant_bob", "Bob the Merchant", "merchant.png");
        npc_db.add_npc(npc_def).expect("Failed to add NPC");

        let mut map = Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);
        map.npc_placements
            .push(NpcPlacement::new("merchant_bob", Position::new(5, 5)));

        // Act
        let resolved = map.resolve_npcs(&npc_db);

        // Assert
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].npc_id, "merchant_bob");
        assert_eq!(resolved[0].name, "Bob the Merchant");
        assert_eq!(resolved[0].position, Position::new(5, 5));
        assert_eq!(resolved[0].portrait_path, "merchant.png");
    }

    #[test]
    fn test_resolve_npcs_with_multiple_npcs() {
        // Arrange
        let mut npc_db = NpcDatabase::new();
        npc_db
            .add_npc(NpcDefinition::merchant(
                "merchant_1",
                "Merchant Shop",
                "merchant.png",
            ))
            .unwrap();
        npc_db
            .add_npc(NpcDefinition::innkeeper(
                "innkeeper_1",
                "Friendly Inn",
                "innkeeper.png",
            ))
            .unwrap();

        let mut map = Map::new(1, "Town".to_string(), "Town map".to_string(), 20, 20);
        map.npc_placements
            .push(NpcPlacement::new("merchant_1", Position::new(5, 5)));
        map.npc_placements
            .push(NpcPlacement::new("innkeeper_1", Position::new(10, 10)));

        // Act
        let resolved = map.resolve_npcs(&npc_db);

        // Assert
        assert_eq!(resolved.len(), 2);
        assert!(resolved.iter().any(|n| n.npc_id == "merchant_1"));
        assert!(resolved.iter().any(|n| n.npc_id == "innkeeper_1"));
        assert!(resolved.iter().any(|n| n.is_merchant));
        assert!(resolved.iter().any(|n| n.is_innkeeper));
    }

    #[test]
    fn test_resolve_npcs_with_missing_definition() {
        // Arrange
        let npc_db = NpcDatabase::new(); // Empty database

        let mut map = Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);
        map.npc_placements
            .push(NpcPlacement::new("nonexistent_npc", Position::new(5, 5)));

        // Act
        let resolved = map.resolve_npcs(&npc_db);

        // Assert
        assert_eq!(resolved.len(), 0, "Missing NPCs should be skipped");
    }

    #[test]
    fn test_resolve_npcs_with_dialogue_override() {
        // Arrange
        let mut npc_db = NpcDatabase::new();
        let npc_def = NpcDefinition {
            id: "guard".to_string(),
            name: "City Guard".to_string(),
            description: "Vigilant guard".to_string(),
            portrait_path: "guard.png".to_string(),
            dialogue_id: Some(10),
            quest_ids: vec![],
            faction: Some("City Watch".to_string()),
            is_merchant: false,
            is_innkeeper: false,
        };
        npc_db.add_npc(npc_def).unwrap();

        let mut map = Map::new(1, "City".to_string(), "City map".to_string(), 20, 20);
        let mut placement = NpcPlacement::new("guard", Position::new(5, 5));
        placement.dialogue_override = Some(99); // Override dialogue
        map.npc_placements.push(placement);

        // Act
        let resolved = map.resolve_npcs(&npc_db);

        // Assert
        assert_eq!(resolved.len(), 1);
        assert_eq!(
            resolved[0].dialogue_id,
            Some(99),
            "Should use placement override"
        );
    }

    #[test]
    fn test_resolve_npcs_with_quest_givers() {
        // Arrange
        let mut npc_db = NpcDatabase::new();
        let npc_def = NpcDefinition {
            id: "quest_giver".to_string(),
            name: "Elder".to_string(),
            description: "Village elder".to_string(),
            portrait_path: "elder.png".to_string(),
            dialogue_id: Some(5),
            quest_ids: vec![1, 2, 3],
            faction: Some("Village".to_string()),
            is_merchant: false,
            is_innkeeper: false,
        };
        npc_db.add_npc(npc_def).unwrap();

        let mut map = Map::new(1, "Village".to_string(), "Village map".to_string(), 15, 15);
        map.npc_placements
            .push(NpcPlacement::new("quest_giver", Position::new(7, 7)));

        // Act
        let resolved = map.resolve_npcs(&npc_db);

        // Assert
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].quest_ids, vec![1, 2, 3]);
        assert_eq!(resolved[0].faction, Some("Village".to_string()));
    }

    #[test]
    fn test_resolved_npc_from_placement_and_definition() {
        // Arrange
        let definition = NpcDefinition {
            id: "test_npc".to_string(),
            name: "Test NPC".to_string(),
            description: "A test NPC".to_string(),
            portrait_path: "test.png".to_string(),
            dialogue_id: Some(42),
            quest_ids: vec![1],
            faction: Some("Test Faction".to_string()),
            is_merchant: true,
            is_innkeeper: false,
        };

        let placement = NpcPlacement {
            npc_id: "test_npc".to_string(),
            position: Position::new(3, 4),
            facing: Some(Direction::North),
            dialogue_override: None,
        };

        // Act
        let resolved = ResolvedNpc::from_placement_and_definition(&placement, &definition);

        // Assert
        assert_eq!(resolved.npc_id, "test_npc");
        assert_eq!(resolved.name, "Test NPC");
        assert_eq!(resolved.description, "A test NPC");
        assert_eq!(resolved.portrait_path, "test.png");
        assert_eq!(resolved.position, Position::new(3, 4));
        assert_eq!(resolved.facing, Some(Direction::North));
        assert_eq!(resolved.dialogue_id, Some(42));
        assert_eq!(resolved.quest_ids, vec![1]);
        assert_eq!(resolved.faction, Some("Test Faction".to_string()));
        assert!(resolved.is_merchant);
        assert!(!resolved.is_innkeeper);
    }

    #[test]
    fn test_resolved_npc_uses_dialogue_override() {
        // Arrange
        let definition = NpcDefinition {
            id: "npc".to_string(),
            name: "NPC".to_string(),
            description: "".to_string(),
            portrait_path: "npc.png".to_string(),
            dialogue_id: Some(10),
            quest_ids: vec![],
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
        };

        let placement = NpcPlacement {
            npc_id: "npc".to_string(),
            position: Position::new(0, 0),
            facing: None,
            dialogue_override: Some(20),
        };

        // Act
        let resolved = ResolvedNpc::from_placement_and_definition(&placement, &definition);

        // Assert
        assert_eq!(
            resolved.dialogue_id,
            Some(20),
            "Should use placement override, not definition default"
        );
    }

    #[test]
    fn test_resolve_npcs_empty_placements() {
        // Arrange
        let mut npc_db = NpcDatabase::new();
        npc_db
            .add_npc(NpcDefinition::new("npc1", "NPC 1", "npc1.png"))
            .unwrap();

        let map = Map::new(1, "Empty".to_string(), "No NPCs".to_string(), 10, 10);
        // No placements added

        // Act
        let resolved = map.resolve_npcs(&npc_db);

        // Assert
        assert_eq!(resolved.len(), 0);
    }
}

// ===== World =====

/// The game world containing all maps
///
/// The world manages multiple maps, tracks the party's current location,
/// and handles map transitions.
///
/// # Examples
///
/// ```
/// use antares::domain::world::{World, Map};
/// use antares::domain::types::{Position, Direction};
///
/// let mut world = World::new();
/// let map = Map::new(1, "Test Map".to_string(), "Description".to_string(), 20, 20);
/// world.add_map(map);
/// world.set_current_map(1);
/// assert_eq!(world.current_map, 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    /// All maps in the world
    pub maps: HashMap<MapId, Map>,
    /// Current map ID
    pub current_map: MapId,
    /// Party position on current map
    pub party_position: Position,
    /// Direction party is facing
    pub party_facing: Direction,
}

impl World {
    /// Creates a new empty world
    pub fn new() -> Self {
        Self {
            maps: HashMap::new(),
            current_map: 0,
            party_position: Position::new(0, 0),
            party_facing: Direction::North,
        }
    }

    /// Adds a map to the world
    pub fn add_map(&mut self, map: Map) {
        let map_id = map.id;
        self.maps.insert(map_id, map);
    }

    /// Gets a reference to a map by ID
    pub fn get_map(&self, map_id: MapId) -> Option<&Map> {
        self.maps.get(&map_id)
    }

    /// Gets a mutable reference to a map by ID
    pub fn get_map_mut(&mut self, map_id: MapId) -> Option<&mut Map> {
        self.maps.get_mut(&map_id)
    }

    /// Gets a reference to the current map
    pub fn get_current_map(&self) -> Option<&Map> {
        self.maps.get(&self.current_map)
    }

    /// Gets a mutable reference to the current map
    pub fn get_current_map_mut(&mut self) -> Option<&mut Map> {
        self.maps.get_mut(&self.current_map)
    }

    /// Sets the current map
    pub fn set_current_map(&mut self, map_id: MapId) {
        self.current_map = map_id;
    }

    /// Sets the party position
    pub fn set_party_position(&mut self, position: Position) {
        self.party_position = position;
    }

    /// Sets the party facing direction
    pub fn set_party_facing(&mut self, direction: Direction) {
        self.party_facing = direction;
    }

    /// Turns the party left
    pub fn turn_left(&mut self) {
        self.party_facing = self.party_facing.turn_left();
    }

    /// Turns the party right
    pub fn turn_right(&mut self) {
        self.party_facing = self.party_facing.turn_right();
    }

    /// Gets the position in front of the party
    pub fn position_ahead(&self) -> Position {
        self.party_facing.forward(self.party_position)
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_creation() {
        let tile = Tile::new(0, 0, TerrainType::Ground, WallType::None);
        assert_eq!(tile.terrain, TerrainType::Ground);
        assert_eq!(tile.wall_type, WallType::None);
        assert!(!tile.blocked);
        assert!(!tile.visited);
        assert_eq!(tile.x, 0);
        assert_eq!(tile.y, 0);

        let wall_tile = Tile::new(1, 1, TerrainType::Ground, WallType::Normal);
        assert!(wall_tile.blocked);
    }

    #[test]
    fn test_tile_door() {
        let door = Tile::new(0, 0, TerrainType::Ground, WallType::Door);
        assert!(door.is_door());
        assert!(!door.has_light());
    }

    #[test]
    fn test_tile_blocked_terrain() {
        let water = Tile::new(0, 0, TerrainType::Water, WallType::None);
        assert!(water.is_blocked());

        let mountain = Tile::new(0, 0, TerrainType::Mountain, WallType::None);
        assert!(mountain.is_blocked());
    }

    #[test]
    fn test_map_bounds() {
        let map = Map::new(1, "Map".to_string(), "Desc".to_string(), 10, 10);
        assert_eq!(map.width, 10);
        assert_eq!(map.height, 10);

        assert!(map.is_valid_position(Position::new(0, 0)));
        assert!(map.is_valid_position(Position::new(9, 9)));
        assert!(!map.is_valid_position(Position::new(10, 10)));
        assert!(!map.is_valid_position(Position::new(-1, 0)));
    }

    #[test]
    fn test_map_tile_access() {
        let map = Map::new(1, "Map".to_string(), "Desc".to_string(), 10, 10);
        let tile = map.get_tile(Position::new(5, 5));
        assert!(tile.is_some());
        assert_eq!(tile.unwrap().terrain, TerrainType::Ground);

        let out_of_bounds = map.get_tile(Position::new(10, 10));
        assert!(out_of_bounds.is_none());
    }

    #[test]
    fn test_map_events() {
        let mut map = Map::new(1, "Map".to_string(), "Desc".to_string(), 10, 10);
        let pos = Position::new(5, 5);
        let event = MapEvent::Sign {
            name: "Sign".to_string(),
            description: "Desc".to_string(),
            text: "Welcome!".to_string(),
        };

        map.add_event(pos, event);
        assert!(map.get_event(pos).is_some());

        let removed = map.remove_event(pos);
        assert!(removed.is_some());
        assert!(map.get_event(pos).is_none());
    }

    #[test]
    fn test_world_map_access() {
        let mut world = World::new();
        let map = Map::new(1, "Map".to_string(), "Desc".to_string(), 20, 20);
        world.add_map(map);

        world.set_current_map(1);
        assert_eq!(world.current_map, 1);
        assert!(world.get_current_map().is_some());
    }

    #[test]
    fn test_world_party_movement() {
        let mut world = World::new();
        world.set_party_position(Position::new(5, 5));
        world.set_party_facing(Direction::North);

        assert_eq!(world.party_position, Position::new(5, 5));
        assert_eq!(world.party_facing, Direction::North);

        let ahead = world.position_ahead();
        assert_eq!(ahead, Position::new(5, 4));
    }

    #[test]
    fn test_world_turn() {
        let mut world = World::new();
        world.set_party_facing(Direction::North);

        world.turn_right();
        assert_eq!(world.party_facing, Direction::East);

        world.turn_left();
        assert_eq!(world.party_facing, Direction::North);
    }

    #[test]
    fn test_npc_creation() {
        let npc = Npc::new(
            1,
            "Merchant".to_string(),
            "Desc".to_string(),
            Position::new(10, 10),
            "Buy something!".to_string(),
        );
        assert_eq!(npc.id, 1);
        assert_eq!(npc.name, "Merchant");
        assert_eq!(npc.position, Position::new(10, 10));
    }

    #[test]
    fn test_map_get_event_at_position_returns_event() {
        // Arrange
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let pos = Position::new(5, 5);
        let event = MapEvent::Sign {
            name: "Test Sign".to_string(),
            description: "A test sign".to_string(),
            text: "Hello, World!".to_string(),
        };
        map.add_event(pos, event.clone());

        // Act
        let result = map.get_event_at_position(pos);

        // Assert
        assert!(result.is_some());
        match result.unwrap() {
            MapEvent::Sign { text, .. } => assert_eq!(text, "Hello, World!"),
            _ => panic!("Expected Sign event"),
        }
    }

    #[test]
    fn test_map_get_event_at_position_returns_none_when_no_event() {
        // Arrange
        let map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let pos = Position::new(5, 5);

        // Act
        let result = map.get_event_at_position(pos);

        // Assert
        assert!(result.is_none());
    }

    // ===== NPC Blocking Tests =====

    #[test]
    fn test_is_blocked_empty_tile_not_blocked() {
        // Arrange
        let map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let pos = Position::new(5, 5);

        // Act & Assert
        assert!(
            !map.is_blocked(pos),
            "Empty ground tile should not be blocked"
        );
    }

    #[test]
    fn test_is_blocked_tile_with_wall_is_blocked() {
        // Arrange
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let pos = Position::new(5, 5);

        // Set tile as blocked (wall)
        if let Some(tile) = map.get_tile_mut(pos) {
            tile.wall_type = WallType::Normal;
            tile.blocked = true;
        }

        // Act & Assert
        assert!(map.is_blocked(pos), "Tile with wall should be blocked");
    }

    #[test]
    fn test_is_blocked_npc_placement_blocks_movement() {
        // Arrange
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let npc_pos = Position::new(5, 5);

        // Add NPC placement
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "guard", npc_pos,
            ));

        // Act & Assert
        assert!(
            map.is_blocked(npc_pos),
            "Position with NPC placement should be blocked"
        );
        assert!(
            !map.is_blocked(Position::new(6, 5)),
            "Adjacent position should not be blocked"
        );
    }

    #[test]
    fn test_is_blocked_legacy_npc_blocks_movement() {
        // Arrange
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let npc_pos = Position::new(3, 7);

        // Add legacy NPC
        map.npcs.push(Npc::new(
            1,
            "Merchant".to_string(),
            "A merchant".to_string(),
            npc_pos,
            "Welcome!".to_string(),
        ));

        // Act & Assert
        assert!(
            map.is_blocked(npc_pos),
            "Position with legacy NPC should be blocked"
        );
        assert!(
            !map.is_blocked(Position::new(4, 7)),
            "Adjacent position should not be blocked"
        );
    }

    #[test]
    fn test_is_blocked_multiple_npcs_at_different_positions() {
        // Arrange
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);

        // Add multiple NPC placements
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "guard1",
                Position::new(5, 5),
            ));
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "guard2",
                Position::new(10, 10),
            ));
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "merchant",
                Position::new(15, 15),
            ));

        // Act & Assert
        assert!(
            map.is_blocked(Position::new(5, 5)),
            "First NPC position should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(10, 10)),
            "Second NPC position should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(15, 15)),
            "Third NPC position should be blocked"
        );
        assert!(
            !map.is_blocked(Position::new(7, 7)),
            "Empty position should not be blocked"
        );
    }

    #[test]
    fn test_is_blocked_out_of_bounds_is_blocked() {
        // Arrange
        let map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);

        // Act & Assert
        assert!(
            map.is_blocked(Position::new(-1, 5)),
            "Negative X should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(5, -1)),
            "Negative Y should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(10, 5)),
            "X >= width should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(5, 10)),
            "Y >= height should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(100, 100)),
            "Far out of bounds should be blocked"
        );
    }

    #[test]
    fn test_is_blocked_npc_on_walkable_tile_blocks() {
        // Arrange
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let pos = Position::new(5, 5);

        // Verify tile is walkable first
        assert!(!map.is_blocked(pos), "Tile should be walkable initially");

        // Add NPC placement
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new("npc", pos));

        // Act & Assert
        assert!(
            map.is_blocked(pos),
            "NPC on walkable tile should block movement"
        );
    }

    #[test]
    fn test_is_blocked_wall_and_npc_both_block() {
        // Arrange
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let pos = Position::new(5, 5);

        // Set tile as blocked
        if let Some(tile) = map.get_tile_mut(pos) {
            tile.wall_type = WallType::Normal;
            tile.blocked = true;
        }

        // Also add NPC (unusual case but tests priority)
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new("npc", pos));

        // Act & Assert
        assert!(
            map.is_blocked(pos),
            "Position with wall and NPC should be blocked"
        );
    }

    #[test]
    fn test_is_blocked_boundary_conditions() {
        // Arrange
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);

        // Add NPCs at corners and edges
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "npc1",
                Position::new(0, 0), // Top-left corner
            ));
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "npc2",
                Position::new(9, 9), // Bottom-right corner
            ));
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "npc3",
                Position::new(0, 9), // Bottom-left corner
            ));
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "npc4",
                Position::new(9, 0), // Top-right corner
            ));

        // Act & Assert
        assert!(
            map.is_blocked(Position::new(0, 0)),
            "Top-left corner should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(9, 9)),
            "Bottom-right corner should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(0, 9)),
            "Bottom-left corner should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(9, 0)),
            "Top-right corner should be blocked"
        );
        assert!(
            !map.is_blocked(Position::new(5, 5)),
            "Center should not be blocked"
        );
    }

    #[test]
    fn test_is_blocked_mixed_legacy_and_new_npcs() {
        // Arrange
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 15, 15);

        // Add legacy NPC
        map.npcs.push(Npc::new(
            1,
            "Old NPC".to_string(),
            "Legacy".to_string(),
            Position::new(3, 3),
            "Hello".to_string(),
        ));

        // Add new NPC placement
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "new_npc",
                Position::new(7, 7),
            ));

        // Act & Assert
        assert!(
            map.is_blocked(Position::new(3, 3)),
            "Legacy NPC position should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(7, 7)),
            "New NPC placement position should be blocked"
        );
        assert!(
            !map.is_blocked(Position::new(5, 5)),
            "Empty position should not be blocked"
        );
    }
}
