//! Movement and navigation system
//!
//! This module handles party movement through the world, collision detection,
//! and tile event triggering.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.2 for world system specifications.

use super::types::{Map, World};
use crate::domain::types::{Direction, Position};
use thiserror::Error;

/// Errors that can occur during movement
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum MovementError {
    /// Attempted to move into a blocked tile
    #[error("Movement blocked at position ({0}, {1})")]
    Blocked(i32, i32),

    /// Attempted to move outside map boundaries
    #[error("Position ({0}, {1}) is outside map boundaries")]
    OutOfBounds(i32, i32),

    /// Current map not found in world
    #[error("Current map with ID {0} not found")]
    MapNotFound(u16),

    /// Door is locked and requires a key
    #[error("Door at position ({0}, {1}) is locked")]
    DoorLocked(i32, i32),
}

/// Moves the party in the specified direction
///
/// This function attempts to move the party one tile in the given direction.
/// It checks for blocked tiles, map boundaries, and updates the party position
/// if movement is successful.
///
/// # Arguments
///
/// * `world` - The game world containing maps and party position
/// * `direction` - The direction to move
///
/// # Returns
///
/// Returns `Ok(new_position)` if movement succeeds, or a `MovementError` if blocked.
///
/// # Errors
///
/// Returns `MovementError::Blocked` if the target tile is blocked
/// Returns `MovementError::OutOfBounds` if the target is outside map boundaries
/// Returns `MovementError::MapNotFound` if the current map doesn't exist
///
/// # Examples
///
/// ```
/// use antares::domain::world::{World, Map, move_party};
/// use antares::domain::types::{Direction, Position};
///
/// let mut world = World::new();
/// let map = Map::new(1, 20, 20);
/// world.add_map(map);
/// world.set_current_map(1);
/// world.set_party_position(Position::new(10, 10));
///
/// let result = move_party(&mut world, Direction::North);
/// assert!(result.is_ok());
/// assert_eq!(world.party_position, Position::new(10, 9));
/// ```
pub fn move_party(world: &mut World, direction: Direction) -> Result<Position, MovementError> {
    // Get the current map
    let current_map_id = world.current_map;
    let map = world
        .get_current_map()
        .ok_or(MovementError::MapNotFound(current_map_id))?;

    // Calculate new position
    let current_pos = world.party_position;
    let new_pos = direction.forward(current_pos);

    // Check if new position is within map bounds
    if !map.is_valid_position(new_pos) {
        return Err(MovementError::OutOfBounds(new_pos.x, new_pos.y));
    }

    // Check if tile is blocked
    if check_tile_blocked(map, new_pos)? {
        return Err(MovementError::Blocked(new_pos.x, new_pos.y));
    }

    // Update party position
    world.set_party_position(new_pos);

    // Mark tile as visited
    if let Some(tile) = world
        .get_current_map_mut()
        .and_then(|m| m.get_tile_mut(new_pos))
    {
        tile.mark_visited();
    }

    Ok(new_pos)
}

/// Checks if a tile at the given position is blocked
///
/// This function determines whether the party can move to a specific tile.
/// Tiles can be blocked due to terrain (mountains, water), walls, or other obstacles.
///
/// # Arguments
///
/// * `map` - The map to check
/// * `position` - The position to check
///
/// # Returns
///
/// Returns `Ok(true)` if the tile is blocked, `Ok(false)` if passable
///
/// # Errors
///
/// Returns `MovementError::OutOfBounds` if position is outside map boundaries
///
/// # Examples
///
/// ```
/// use antares::domain::world::{Map, check_tile_blocked, TerrainType, WallType, Tile};
/// use antares::domain::types::Position;
///
/// let mut map = Map::new(1, 10, 10);
/// let pos = Position::new(5, 5);
///
/// // Ground is not blocked
/// let result = check_tile_blocked(&map, pos);
/// assert!(result.is_ok());
/// assert!(!result.unwrap());
///
/// // Set up a wall
/// if let Some(tile) = map.get_tile_mut(Position::new(6, 6)) {
///     tile.wall_type = WallType::Normal;
///     tile.blocked = true;
/// }
/// let wall_result = check_tile_blocked(&map, Position::new(6, 6));
/// assert!(wall_result.unwrap());
/// ```
pub fn check_tile_blocked(map: &Map, position: Position) -> Result<bool, MovementError> {
    // Check if position is valid
    if !map.is_valid_position(position) {
        return Err(MovementError::OutOfBounds(position.x, position.y));
    }

    // Get tile at position
    let tile = map
        .get_tile(position)
        .ok_or(MovementError::OutOfBounds(position.x, position.y))?;

    Ok(tile.is_blocked())
}

/// Triggers any event associated with a tile
///
/// This function checks if a tile has an event trigger and returns information
/// about it. The actual event handling is done by the event system.
///
/// # Arguments
///
/// * `map` - The map to check for events
/// * `position` - The position to check for events
///
/// # Returns
///
/// Returns `Some(event_id)` if there's an event trigger, `None` otherwise
///
/// # Examples
///
/// ```
/// use antares::domain::world::{Map, trigger_tile_event, Tile, TerrainType, WallType};
/// use antares::domain::types::Position;
///
/// let mut map = Map::new(1, 10, 10);
/// let pos = Position::new(5, 5);
///
/// // Initially no event
/// assert!(trigger_tile_event(&map, pos).is_none());
///
/// // Add event trigger to tile
/// if let Some(tile) = map.get_tile_mut(pos) {
///     tile.event_trigger = Some(42);
/// }
///
/// // Now event should be found
/// assert_eq!(trigger_tile_event(&map, pos), Some(42));
/// ```
pub fn trigger_tile_event(map: &Map, position: Position) -> Option<u16> {
    map.get_tile(position).and_then(|tile| tile.event_trigger)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::world::{TerrainType, WallType};

    #[test]
    fn test_move_party_basic() {
        let mut world = World::new();
        let map = Map::new(1, 20, 20);
        world.add_map(map);
        world.set_current_map(1);
        world.set_party_position(Position::new(10, 10));

        let result = move_party(&mut world, Direction::North);
        assert!(result.is_ok());
        assert_eq!(world.party_position, Position::new(10, 9));
    }

    #[test]
    fn test_move_party_all_directions() {
        let mut world = World::new();
        let map = Map::new(1, 20, 20);
        world.add_map(map);
        world.set_current_map(1);
        world.set_party_position(Position::new(10, 10));

        // North
        assert!(move_party(&mut world, Direction::North).is_ok());
        assert_eq!(world.party_position, Position::new(10, 9));

        // East
        assert!(move_party(&mut world, Direction::East).is_ok());
        assert_eq!(world.party_position, Position::new(11, 9));

        // South
        assert!(move_party(&mut world, Direction::South).is_ok());
        assert_eq!(world.party_position, Position::new(11, 10));

        // West
        assert!(move_party(&mut world, Direction::West).is_ok());
        assert_eq!(world.party_position, Position::new(10, 10));
    }

    #[test]
    fn test_move_blocked_by_wall() {
        let mut world = World::new();
        let mut map = Map::new(1, 20, 20);

        // Place a wall to the north of starting position
        let wall_pos = Position::new(10, 9);
        if let Some(tile) = map.get_tile_mut(wall_pos) {
            tile.wall_type = WallType::Normal;
            tile.blocked = true;
        }

        world.add_map(map);
        world.set_current_map(1);
        world.set_party_position(Position::new(10, 10));

        // Try to move north into wall
        let result = move_party(&mut world, Direction::North);
        assert!(result.is_err());
        assert!(matches!(result, Err(MovementError::Blocked(10, 9))));

        // Position should not have changed
        assert_eq!(world.party_position, Position::new(10, 10));
    }

    #[test]
    fn test_move_blocked_by_water() {
        let mut world = World::new();
        let mut map = Map::new(1, 20, 20);

        // Place water to the east
        let water_pos = Position::new(11, 10);
        if let Some(tile) = map.get_tile_mut(water_pos) {
            tile.terrain = TerrainType::Water;
            tile.blocked = true;
        }

        world.add_map(map);
        world.set_current_map(1);
        world.set_party_position(Position::new(10, 10));

        // Try to move east into water
        let result = move_party(&mut world, Direction::East);
        assert!(result.is_err());
        assert!(matches!(result, Err(MovementError::Blocked(11, 10))));
    }

    #[test]
    fn test_map_boundaries() {
        let mut world = World::new();
        let map = Map::new(1, 20, 20);
        world.add_map(map);
        world.set_current_map(1);

        // Test north boundary
        world.set_party_position(Position::new(10, 0));
        let result = move_party(&mut world, Direction::North);
        assert!(result.is_err());
        assert!(matches!(result, Err(MovementError::OutOfBounds(10, -1))));

        // Test south boundary
        world.set_party_position(Position::new(10, 19));
        let result = move_party(&mut world, Direction::South);
        assert!(result.is_err());
        assert!(matches!(result, Err(MovementError::OutOfBounds(10, 20))));

        // Test west boundary
        world.set_party_position(Position::new(0, 10));
        let result = move_party(&mut world, Direction::West);
        assert!(result.is_err());
        assert!(matches!(result, Err(MovementError::OutOfBounds(-1, 10))));

        // Test east boundary
        world.set_party_position(Position::new(19, 10));
        let result = move_party(&mut world, Direction::East);
        assert!(result.is_err());
        assert!(matches!(result, Err(MovementError::OutOfBounds(20, 10))));
    }

    #[test]
    fn test_door_interaction() {
        let mut world = World::new();
        let mut map = Map::new(1, 20, 20);

        // Place a door (not blocked by default in our implementation)
        let door_pos = Position::new(10, 9);
        if let Some(tile) = map.get_tile_mut(door_pos) {
            tile.wall_type = WallType::Door;
            tile.blocked = false; // Doors can be walked through when open
        }

        world.add_map(map);
        world.set_current_map(1);
        world.set_party_position(Position::new(10, 10));

        // Should be able to move through open door
        let result = move_party(&mut world, Direction::North);
        assert!(result.is_ok());
        assert_eq!(world.party_position, Position::new(10, 9));
    }

    #[test]
    fn test_check_tile_blocked_basic() {
        let map = Map::new(1, 10, 10);
        let pos = Position::new(5, 5);

        let result = check_tile_blocked(&map, pos);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_check_tile_blocked_wall() {
        let mut map = Map::new(1, 10, 10);
        let pos = Position::new(5, 5);

        if let Some(tile) = map.get_tile_mut(pos) {
            tile.wall_type = WallType::Normal;
            tile.blocked = true;
        }

        let result = check_tile_blocked(&map, pos);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_check_tile_blocked_out_of_bounds() {
        let map = Map::new(1, 10, 10);
        let pos = Position::new(10, 10);

        let result = check_tile_blocked(&map, pos);
        assert!(result.is_err());
        assert!(matches!(result, Err(MovementError::OutOfBounds(10, 10))));
    }

    #[test]
    fn test_trigger_tile_event_none() {
        let map = Map::new(1, 10, 10);
        let pos = Position::new(5, 5);

        assert!(trigger_tile_event(&map, pos).is_none());
    }

    #[test]
    fn test_trigger_tile_event_exists() {
        let mut map = Map::new(1, 10, 10);
        let pos = Position::new(5, 5);

        if let Some(tile) = map.get_tile_mut(pos) {
            tile.event_trigger = Some(42);
        }

        assert_eq!(trigger_tile_event(&map, pos), Some(42));
    }

    #[test]
    fn test_tile_visited_after_move() {
        let mut world = World::new();
        let map = Map::new(1, 20, 20);
        world.add_map(map);
        world.set_current_map(1);
        world.set_party_position(Position::new(10, 10));

        // Move north
        let new_pos = Position::new(10, 9);
        assert!(move_party(&mut world, Direction::North).is_ok());

        // Check that tile is marked as visited
        let tile = world.get_current_map().unwrap().get_tile(new_pos).unwrap();
        assert!(tile.visited);
    }

    #[test]
    fn test_move_party_no_map() {
        let mut world = World::new();
        world.set_current_map(99); // Non-existent map

        let result = move_party(&mut world, Direction::North);
        assert!(result.is_err());
        assert!(matches!(result, Err(MovementError::MapNotFound(99))));
    }
}
