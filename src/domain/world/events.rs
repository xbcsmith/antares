// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Map event handling system
//!
//! This module handles the processing and resolution of map events such as
//! encounters, treasure, teleports, traps, signs, and NPC dialogue.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.2 for world system specifications.

use super::types::{MapEvent, World};
use crate::domain::types::Position;
use thiserror::Error;

/// Result of triggering an event
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventResult {
    /// No event at the position
    None,
    /// Monster encounter triggered
    Encounter {
        /// IDs of monsters in the encounter
        monster_group: Vec<u8>,
    },
    /// Treasure found
    Treasure {
        /// Item IDs in the treasure
        loot: Vec<u8>,
    },
    /// Party teleported to new location
    Teleported {
        /// New position
        position: Position,
        /// New map ID
        map_id: u16,
    },
    /// Trap triggered
    Trap {
        /// Damage dealt to party
        damage: u16,
        /// Optional status effect
        effect: Option<String>,
    },
    /// Sign read
    Sign {
        /// Text displayed
        text: String,
    },
    /// NPC dialogue initiated
    NpcDialogue {
        /// NPC identifier
        npc_id: u16,
    },
}

/// Errors that can occur during event processing
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum EventError {
    /// Event position is out of bounds
    #[error("Event position ({0}, {1}) is out of bounds")]
    OutOfBounds(i32, i32),

    /// Current map not found
    #[error("Current map with ID {0} not found")]
    MapNotFound(u16),

    /// Invalid event data
    #[error("Invalid event data: {0}")]
    InvalidEvent(String),
}

/// Triggers an event at the specified position
///
/// This function checks if there's an event at the given position on the current map
/// and processes it, returning the result. Events are typically one-time occurrences
/// and are removed from the map after being triggered (except for repeatable events
/// like signs and NPC dialogues).
///
/// # Arguments
///
/// * `world` - The game world containing maps and events
/// * `position` - The position to check for events
///
/// # Returns
///
/// Returns `Ok(EventResult)` describing what happened
///
/// # Errors
///
/// Returns `EventError::MapNotFound` if the current map doesn't exist
/// Returns `EventError::OutOfBounds` if position is outside map boundaries
///
/// # Examples
///
/// ```
/// use antares::domain::world::{World, Map, MapEvent, trigger_event, EventResult};
/// use antares::domain::types::Position;
///
/// let mut world = World::new();
/// let mut map = Map::new(1, 20, 20);
///
/// // Add a sign event
/// let pos = Position::new(10, 10);
/// map.add_event(pos, MapEvent::Sign {
///     text: "Welcome to the dungeon!".to_string(),
/// });
///
/// world.add_map(map);
/// world.set_current_map(1);
///
/// // Trigger the event
/// let result = trigger_event(&mut world, pos);
/// assert!(result.is_ok());
/// match result.unwrap() {
///     EventResult::Sign { text } => assert_eq!(text, "Welcome to the dungeon!"),
///     _ => panic!("Expected Sign event"),
/// }
/// ```
pub fn trigger_event(world: &mut World, position: Position) -> Result<EventResult, EventError> {
    // Get current map ID
    let current_map_id = world.current_map;

    // Get current map (immutable check first)
    let map = world
        .get_current_map()
        .ok_or(EventError::MapNotFound(current_map_id))?;

    // Check position is valid
    if !map.is_valid_position(position) {
        return Err(EventError::OutOfBounds(position.x, position.y));
    }

    // Check if there's an event at this position
    let event = match map.get_event(position) {
        Some(event) => event.clone(),
        None => return Ok(EventResult::None),
    };

    // Process the event based on its type
    let result = match event {
        MapEvent::Encounter { monster_group } => EventResult::Encounter { monster_group },

        MapEvent::Treasure { loot } => {
            // Remove treasure event after being collected (one-time)
            world
                .get_current_map_mut()
                .ok_or(EventError::MapNotFound(current_map_id))?
                .remove_event(position);

            EventResult::Treasure { loot }
        }

        MapEvent::Teleport {
            destination,
            map_id,
        } => {
            // Teleport the party
            world.set_current_map(map_id);
            world.set_party_position(destination);

            EventResult::Teleported {
                position: destination,
                map_id,
            }
        }

        MapEvent::Trap { damage, effect } => {
            // Remove trap after being triggered (one-time)
            world
                .get_current_map_mut()
                .ok_or(EventError::MapNotFound(current_map_id))?
                .remove_event(position);

            EventResult::Trap { damage, effect }
        }

        MapEvent::Sign { text } => {
            // Signs are repeatable - don't remove
            EventResult::Sign { text }
        }

        MapEvent::NpcDialogue { npc_id } => {
            // NPC dialogues are repeatable - don't remove
            EventResult::NpcDialogue { npc_id }
        }
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::world::Map;

    #[test]
    fn test_no_event() {
        let mut world = World::new();
        let map = Map::new(1, 20, 20);
        world.add_map(map);
        world.set_current_map(1);

        let pos = Position::new(10, 10);
        let result = trigger_event(&mut world, pos);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), EventResult::None);
    }

    #[test]
    fn test_encounter_event() {
        let mut world = World::new();
        let mut map = Map::new(1, 20, 20);

        let pos = Position::new(10, 10);
        map.add_event(
            pos,
            MapEvent::Encounter {
                monster_group: vec![1, 2, 3],
            },
        );

        world.add_map(map);
        world.set_current_map(1);

        let result = trigger_event(&mut world, pos);
        assert!(result.is_ok());
        match result.unwrap() {
            EventResult::Encounter { monster_group } => {
                assert_eq!(monster_group, vec![1, 2, 3]);
            }
            _ => panic!("Expected Encounter event"),
        }
    }

    #[test]
    fn test_treasure_event() {
        let mut world = World::new();
        let mut map = Map::new(1, 20, 20);

        let pos = Position::new(10, 10);
        map.add_event(
            pos,
            MapEvent::Treasure {
                loot: vec![10, 20, 30],
            },
        );

        world.add_map(map);
        world.set_current_map(1);

        let result = trigger_event(&mut world, pos);
        assert!(result.is_ok());
        match result.unwrap() {
            EventResult::Treasure { loot } => {
                assert_eq!(loot, vec![10, 20, 30]);
            }
            _ => panic!("Expected Treasure event"),
        }

        // Treasure should be removed after collection
        let result2 = trigger_event(&mut world, pos);
        assert_eq!(result2.unwrap(), EventResult::None);
    }

    #[test]
    fn test_teleport_event() {
        let mut world = World::new();
        let mut map1 = Map::new(1, 20, 20);
        let map2 = Map::new(2, 30, 30);

        let pos = Position::new(10, 10);
        let dest = Position::new(15, 15);
        map1.add_event(
            pos,
            MapEvent::Teleport {
                destination: dest,
                map_id: 2,
            },
        );

        world.add_map(map1);
        world.add_map(map2);
        world.set_current_map(1);
        world.set_party_position(pos);

        let result = trigger_event(&mut world, pos);
        assert!(result.is_ok());
        match result.unwrap() {
            EventResult::Teleported { position, map_id } => {
                assert_eq!(position, dest);
                assert_eq!(map_id, 2);
            }
            _ => panic!("Expected Teleported event"),
        }

        // Verify party was actually teleported
        assert_eq!(world.current_map, 2);
        assert_eq!(world.party_position, dest);
    }

    #[test]
    fn test_trap_event_damages_party() {
        let mut world = World::new();
        let mut map = Map::new(1, 20, 20);

        let pos = Position::new(10, 10);
        map.add_event(
            pos,
            MapEvent::Trap {
                damage: 25,
                effect: Some("Poisoned".to_string()),
            },
        );

        world.add_map(map);
        world.set_current_map(1);

        let result = trigger_event(&mut world, pos);
        assert!(result.is_ok());
        match result.unwrap() {
            EventResult::Trap { damage, effect } => {
                assert_eq!(damage, 25);
                assert_eq!(effect, Some("Poisoned".to_string()));
            }
            _ => panic!("Expected Trap event"),
        }

        // Trap should be removed after triggering
        let result2 = trigger_event(&mut world, pos);
        assert_eq!(result2.unwrap(), EventResult::None);
    }

    #[test]
    fn test_sign_event() {
        let mut world = World::new();
        let mut map = Map::new(1, 20, 20);

        let pos = Position::new(10, 10);
        let sign_text = "Beware of dragons!".to_string();
        map.add_event(
            pos,
            MapEvent::Sign {
                text: sign_text.clone(),
            },
        );

        world.add_map(map);
        world.set_current_map(1);

        let result = trigger_event(&mut world, pos);
        assert!(result.is_ok());
        match result.unwrap() {
            EventResult::Sign { text } => {
                assert_eq!(text, sign_text);
            }
            _ => panic!("Expected Sign event"),
        }

        // Sign should still be there (repeatable)
        let result2 = trigger_event(&mut world, pos);
        assert!(result2.is_ok());
        match result2.unwrap() {
            EventResult::Sign { text } => {
                assert_eq!(text, sign_text);
            }
            _ => panic!("Expected Sign event on repeat"),
        }
    }

    #[test]
    fn test_npc_dialogue_event() {
        let mut world = World::new();
        let mut map = Map::new(1, 20, 20);

        let pos = Position::new(10, 10);
        map.add_event(pos, MapEvent::NpcDialogue { npc_id: 42 });

        world.add_map(map);
        world.set_current_map(1);

        let result = trigger_event(&mut world, pos);
        assert!(result.is_ok());
        match result.unwrap() {
            EventResult::NpcDialogue { npc_id } => {
                assert_eq!(npc_id, 42);
            }
            _ => panic!("Expected NpcDialogue event"),
        }

        // NPC dialogue should still be there (repeatable)
        let result2 = trigger_event(&mut world, pos);
        assert!(result2.is_ok());
        matches!(result2.unwrap(), EventResult::NpcDialogue { npc_id: 42 });
    }

    #[test]
    fn test_event_out_of_bounds() {
        let mut world = World::new();
        let map = Map::new(1, 20, 20);
        world.add_map(map);
        world.set_current_map(1);

        let pos = Position::new(25, 25); // Out of bounds
        let result = trigger_event(&mut world, pos);
        assert!(result.is_err());
        assert!(matches!(result, Err(EventError::OutOfBounds(25, 25))));
    }

    #[test]
    fn test_event_map_not_found() {
        let mut world = World::new();
        world.set_current_map(99); // Non-existent map

        let pos = Position::new(10, 10);
        let result = trigger_event(&mut world, pos);
        assert!(result.is_err());
        assert!(matches!(result, Err(EventError::MapNotFound(99))));
    }

    #[test]
    fn test_multiple_events_different_positions() {
        let mut world = World::new();
        let mut map = Map::new(1, 20, 20);

        let pos1 = Position::new(5, 5);
        let pos2 = Position::new(10, 10);
        let pos3 = Position::new(15, 15);

        map.add_event(
            pos1,
            MapEvent::Sign {
                text: "North".to_string(),
            },
        );
        map.add_event(pos2, MapEvent::Treasure { loot: vec![1, 2] });
        map.add_event(
            pos3,
            MapEvent::Trap {
                damage: 10,
                effect: None,
            },
        );

        world.add_map(map);
        world.set_current_map(1);

        // Trigger each event
        let r1 = trigger_event(&mut world, pos1);
        assert!(matches!(r1, Ok(EventResult::Sign { .. })));

        let r2 = trigger_event(&mut world, pos2);
        assert!(matches!(r2, Ok(EventResult::Treasure { .. })));

        let r3 = trigger_event(&mut world, pos3);
        assert!(matches!(r3, Ok(EventResult::Trap { .. })));

        // Verify one-time events are removed
        assert_eq!(trigger_event(&mut world, pos2).unwrap(), EventResult::None);
        assert_eq!(trigger_event(&mut world, pos3).unwrap(), EventResult::None);

        // Verify repeatable event remains
        assert!(matches!(
            trigger_event(&mut world, pos1),
            Ok(EventResult::Sign { .. })
        ));
    }
}
