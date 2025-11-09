//! Core world data structures
//!
//! This module contains the fundamental types for the world system including
//! tiles, maps, NPCs, and the overall world structure.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.2 for complete specifications.

use crate::domain::types::{Direction, EventId, MapId, Position};
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
/// let tile = Tile::new(TerrainType::Ground, WallType::None);
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
    /// Optional event trigger
    pub event_trigger: Option<EventId>,
}

impl Tile {
    /// Creates a new tile with the given terrain and wall type
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{Tile, TerrainType, WallType};
    ///
    /// let tile = Tile::new(TerrainType::Ground, WallType::None);
    /// assert_eq!(tile.terrain, TerrainType::Ground);
    /// assert_eq!(tile.wall_type, WallType::None);
    /// ```
    pub fn new(terrain: TerrainType, wall_type: WallType) -> Self {
        let blocked = matches!(terrain, TerrainType::Mountain | TerrainType::Water)
            || matches!(wall_type, WallType::Normal);

        Self {
            terrain,
            wall_type,
            blocked,
            is_special: false,
            is_dark: false,
            visited: false,
            event_trigger: None,
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
        /// Monster group IDs
        monster_group: Vec<u8>,
    },
    /// Treasure chest
    Treasure {
        /// Loot table or item IDs
        loot: Vec<u8>,
    },
    /// Teleport to another location
    Teleport {
        /// Destination position
        destination: Position,
        /// Target map ID
        map_id: MapId,
    },
    /// Trap that triggers
    Trap {
        /// Damage amount
        damage: u16,
        /// Status effect
        effect: Option<String>,
    },
    /// Sign with text
    Sign {
        /// Message text
        text: String,
    },
    /// NPC dialogue trigger
    NpcDialogue {
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
    /// let npc = Npc::new(1, "Merchant".to_string(), Position::new(5, 5), "Welcome!".to_string());
    /// assert_eq!(npc.name, "Merchant");
    /// ```
    pub fn new(id: u16, name: String, position: Position, dialogue: String) -> Self {
        Self {
            id,
            name,
            position,
            dialogue,
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
/// let map = Map::new(1, 20, 20);
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
    /// 2D grid of tiles [y][x]
    pub tiles: Vec<Vec<Tile>>,
    /// Events at specific positions
    pub events: HashMap<Position, MapEvent>,
    /// NPCs on this map
    pub npcs: Vec<Npc>,
}

impl Map {
    /// Creates a new map with the given dimensions
    ///
    /// All tiles are initialized to ground terrain with no walls.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::Map;
    ///
    /// let map = Map::new(1, 10, 10);
    /// assert_eq!(map.width, 10);
    /// assert_eq!(map.height, 10);
    /// assert_eq!(map.tiles.len(), 10);
    /// assert_eq!(map.tiles[0].len(), 10);
    /// ```
    pub fn new(id: MapId, width: u32, height: u32) -> Self {
        let tiles = (0..height)
            .map(|_| {
                (0..width)
                    .map(|_| Tile::new(TerrainType::Ground, WallType::None))
                    .collect()
            })
            .collect();

        Self {
            id,
            width,
            height,
            tiles,
            events: HashMap::new(),
            npcs: Vec::new(),
        }
    }

    /// Gets a tile at the specified position
    ///
    /// Returns `None` if the position is out of bounds.
    pub fn get_tile(&self, pos: Position) -> Option<&Tile> {
        if self.is_valid_position(pos) {
            Some(&self.tiles[pos.y as usize][pos.x as usize])
        } else {
            None
        }
    }

    /// Gets a mutable reference to a tile at the specified position
    ///
    /// Returns `None` if the position is out of bounds.
    pub fn get_tile_mut(&mut self, pos: Position) -> Option<&mut Tile> {
        if self.is_valid_position(pos) {
            Some(&mut self.tiles[pos.y as usize][pos.x as usize])
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
    /// let map = Map::new(1, 10, 10);
    /// assert!(map.is_valid_position(Position::new(5, 5)));
    /// assert!(!map.is_valid_position(Position::new(10, 10)));
    /// assert!(!map.is_valid_position(Position::new(-1, 5)));
    /// ```
    pub fn is_valid_position(&self, pos: Position) -> bool {
        pos.x >= 0 && pos.y >= 0 && pos.x < self.width as i32 && pos.y < self.height as i32
    }

    /// Returns true if the tile at the position is blocked
    pub fn is_blocked(&self, pos: Position) -> bool {
        self.get_tile(pos).is_none_or(|tile| tile.is_blocked())
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

    /// Adds an NPC to the map
    pub fn add_npc(&mut self, npc: Npc) {
        self.npcs.push(npc);
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
/// let map = Map::new(1, 20, 20);
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
        let tile = Tile::new(TerrainType::Ground, WallType::None);
        assert_eq!(tile.terrain, TerrainType::Ground);
        assert_eq!(tile.wall_type, WallType::None);
        assert!(!tile.blocked);
        assert!(!tile.visited);

        let wall_tile = Tile::new(TerrainType::Ground, WallType::Normal);
        assert!(wall_tile.blocked);
    }

    #[test]
    fn test_tile_door() {
        let door = Tile::new(TerrainType::Ground, WallType::Door);
        assert!(door.is_door());
        assert!(!door.has_light());
    }

    #[test]
    fn test_tile_blocked_terrain() {
        let water = Tile::new(TerrainType::Water, WallType::None);
        assert!(water.is_blocked());

        let mountain = Tile::new(TerrainType::Mountain, WallType::None);
        assert!(mountain.is_blocked());
    }

    #[test]
    fn test_map_bounds() {
        let map = Map::new(1, 10, 10);
        assert_eq!(map.width, 10);
        assert_eq!(map.height, 10);

        assert!(map.is_valid_position(Position::new(0, 0)));
        assert!(map.is_valid_position(Position::new(9, 9)));
        assert!(!map.is_valid_position(Position::new(10, 10)));
        assert!(!map.is_valid_position(Position::new(-1, 0)));
    }

    #[test]
    fn test_map_tile_access() {
        let map = Map::new(1, 10, 10);
        let tile = map.get_tile(Position::new(5, 5));
        assert!(tile.is_some());
        assert_eq!(tile.unwrap().terrain, TerrainType::Ground);

        let out_of_bounds = map.get_tile(Position::new(10, 10));
        assert!(out_of_bounds.is_none());
    }

    #[test]
    fn test_map_events() {
        let mut map = Map::new(1, 10, 10);
        let pos = Position::new(5, 5);
        let event = MapEvent::Sign {
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
        let map = Map::new(1, 20, 20);
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
            Position::new(10, 10),
            "Buy something!".to_string(),
        );
        assert_eq!(npc.id, 1);
        assert_eq!(npc.name, "Merchant");
        assert_eq!(npc.position, Position::new(10, 10));
    }
}
