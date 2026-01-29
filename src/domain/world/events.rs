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
        /// NPC identifier (string-based ID for NPC database lookup)
        npc_id: crate::domain::world::NpcId,
    },
    /// Recruitable character encounter
    RecruitableCharacter {
        /// Character definition ID for recruitment
        character_id: String,
    },
    /// Enter an inn for party management
    EnterInn {
        /// Innkeeper NPC identifier (NpcId string)
        innkeeper_id: crate::domain::world::NpcId,
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
/// let mut map = Map::new(1, "Test Map".to_string(), "Description".to_string(), 20, 20);
///
/// // Add a sign event
/// let pos = Position::new(10, 10);
/// map.add_event(pos, MapEvent::Sign {
///     name: "Dungeon Sign".to_string(),
///     description: "A weathered sign".to_string(),
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
        MapEvent::Encounter { monster_group, .. } => EventResult::Encounter { monster_group },

        MapEvent::Treasure { loot, .. } => {
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
            ..
        } => {
            // Teleport the party
            world.set_current_map(map_id);
            world.set_party_position(destination);

            EventResult::Teleported {
                position: destination,
                map_id,
            }
        }

        MapEvent::Trap { damage, effect, .. } => {
            // Remove trap after being triggered (one-time)
            world
                .get_current_map_mut()
                .ok_or(EventError::MapNotFound(current_map_id))?
                .remove_event(position);

            EventResult::Trap { damage, effect }
        }

        MapEvent::Sign { text, .. } => {
            // Signs are repeatable - don't remove
            EventResult::Sign { text }
        }

        MapEvent::NpcDialogue { npc_id, .. } => {
            // NPC dialogues are repeatable - don't remove
            EventResult::NpcDialogue {
                npc_id: npc_id.clone(),
            }
        }

        MapEvent::RecruitableCharacter { character_id, .. } => {
            // Recruitment encounters are one-time - remove after triggered
            world
                .get_current_map_mut()
                .ok_or(EventError::MapNotFound(current_map_id))?
                .remove_event(position);

            EventResult::RecruitableCharacter {
                character_id: character_id.clone(),
            }
        }

        MapEvent::EnterInn { innkeeper_id, .. } => {
            // Inn entrances are repeatable - don't remove
            EventResult::EnterInn {
                innkeeper_id: innkeeper_id.clone(),
            }
        }
    };

    Ok(result)
}

/// Roll for a random encounter based on the world's current map encounter table
/// and the terrain modifier at the party position. Returns `Some(monster_group)`
/// when an encounter should occur, otherwise `None`.
///
/// `R` is the RNG implementation used by the project's random helper (e.g. `rand::rng()`).
pub fn random_encounter<R: rand::Rng>(world: &World, rng: &mut R) -> Option<Vec<u8>> {
    // Get the current map (return None if not present)
    let map = world.get_current_map()?;

    // Respect the map's safe-zone flag
    if !map.allow_random_encounters {
        return None;
    }

    // Must have a configured encounter table
    let table = match &map.encounter_table {
        Some(t) => t,
        None => return None,
    };

    // No groups or zero chance => no encounter
    if table.groups.is_empty() || table.encounter_rate <= 0.0 {
        return None;
    }

    // Compute terrain modifier (defaults to 1.0)
    let terrain_modifier = map
        .get_tile(world.party_position)
        .map(|tile| {
            table
                .terrain_modifiers
                .get(&tile.terrain)
                .cloned()
                .unwrap_or(1.0)
        })
        .unwrap_or(1.0);

    let chance = (table.encounter_rate * terrain_modifier).clamp(0.0, 1.0);

    // Roll for encounter
    if rng.random::<f32>() <= chance {
        let idx = rng.random_range(0..table.groups.len());
        Some(table.groups[idx].clone())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::world::Map;

    #[test]
    fn test_no_event() {
        let mut world = World::new();
        let map = Map::new(1, "Test Map".to_string(), "Description".to_string(), 20, 20);
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
        let mut map = Map::new(1, "Test Map".to_string(), "Description".to_string(), 20, 20);

        let pos = Position::new(10, 10);
        map.add_event(
            pos,
            MapEvent::Encounter {
                name: "Encounter".to_string(),
                description: "Desc".to_string(),
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
        let mut map = Map::new(1, "Test Map".to_string(), "Description".to_string(), 20, 20);

        let pos = Position::new(10, 10);
        map.add_event(
            pos,
            MapEvent::Treasure {
                name: "Treasure".to_string(),
                description: "Desc".to_string(),
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
        let mut map1 = Map::new(1, "Map 1".to_string(), "Desc".to_string(), 20, 20);
        let map2 = Map::new(2, "Map 2".to_string(), "Desc".to_string(), 30, 30);

        let pos = Position::new(10, 10);
        let dest = Position::new(15, 15);
        map1.add_event(
            pos,
            MapEvent::Teleport {
                name: "Teleport".to_string(),
                description: "Desc".to_string(),
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
        let mut map = Map::new(1, "Map".to_string(), "Desc".to_string(), 20, 20);

        let pos = Position::new(10, 10);
        map.add_event(
            pos,
            MapEvent::Trap {
                name: "Trap".to_string(),
                description: "Desc".to_string(),
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
        let mut map = Map::new(1, "Map".to_string(), "Desc".to_string(), 20, 20);

        let pos = Position::new(10, 10);
        let sign_text = "Beware of dragons!".to_string();
        map.add_event(
            pos,
            MapEvent::Sign {
                name: "Sign".to_string(),
                description: "Desc".to_string(),
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
    }

    #[test]
    fn test_random_encounter_triggers() {
        // Arrange: Create a map with an encounter table that guarantees an encounter
        let mut world = World::new();
        let mut map = Map::new(
            1,
            "Encounter Map".to_string(),
            "Random encounter test".to_string(),
            10,
            10,
        );

        // Configure encounter table: 100% base rate and a single group [1,2,3]
        let mut table = crate::domain::world::EncounterTable::default();
        table.encounter_rate = 1.0;
        table.groups.push(vec![1u8, 2u8, 3u8]);

        map.encounter_table = Some(table);
        map.allow_random_encounters = true;

        world.add_map(map);
        world.set_current_map(1);
        world.set_party_position(Position::new(5, 5));

        // Act
        let mut rng = rand::rng();
        let result = random_encounter(&world, &mut rng);

        // Assert
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![1u8, 2u8, 3u8]);
    }

    #[test]
    fn test_safe_zones_prevent_encounters() {
        // Arrange: Map has an encounter table but random encounters are disabled
        let mut world = World::new();
        let mut map = Map::new(
            1,
            "Safe Map".to_string(),
            "No encounters here".to_string(),
            10,
            10,
        );

        let mut table = crate::domain::world::EncounterTable::default();
        table.encounter_rate = 1.0;
        table.groups.push(vec![1u8]);

        map.encounter_table = Some(table);
        map.allow_random_encounters = false; // safe zone

        world.add_map(map);
        world.set_current_map(1);
        world.set_party_position(Position::new(2, 2));

        // Act
        let mut rng = rand::rng();
        let result = random_encounter(&world, &mut rng);

        // Assert: No encounter should be returned in a safe zone
        assert!(result.is_none());
    }

    #[test]
    fn test_enter_inn_event_with_innkeeper_id() {
        // Arrange: create a map with an EnterInn event that references an innkeeper ID
        let mut world = World::new();
        let mut map = Map::new(
            1,
            "Inn Map".to_string(),
            "Cozy inn for testing".to_string(),
            10,
            10,
        );

        let pos = Position::new(3, 3);
        let innkeeper_id = "tutorial_innkeeper_town".to_string();
        map.add_event(
            pos,
            MapEvent::EnterInn {
                name: "Cozy Inn".to_string(),
                description: "A welcoming inn where travelers rest".to_string(),
                innkeeper_id: innkeeper_id.clone(),
            },
        );

        world.add_map(map);
        world.set_current_map(1);

        // Act: trigger the event
        let result = trigger_event(&mut world, pos);
        assert!(result.is_ok());

        // Assert: EnterInn event contains the correct innkeeper_id
        match result.unwrap() {
            EventResult::EnterInn { innkeeper_id: id } => {
                assert_eq!(id, innkeeper_id);
            }
            _ => panic!("Expected EnterInn event"),
        }

        // EnterInn events are repeatable; triggering again should yield the same result
        let result2 = trigger_event(&mut world, pos);
        assert!(result2.is_ok());
        match result2.unwrap() {
            EventResult::EnterInn { innkeeper_id: id } => {
                assert_eq!(id, innkeeper_id);
            }
            _ => panic!("Expected EnterInn event on second trigger as well"),
        }
    }

    #[test]
    fn test_npc_dialogue_event() {
        let mut world = World::new();
        let mut map = Map::new(1, "Map".to_string(), "Desc".to_string(), 20, 20);

        let pos = Position::new(10, 10);
        map.add_event(
            pos,
            MapEvent::NpcDialogue {
                name: "NPC".to_string(),
                description: "Desc".to_string(),
                npc_id: "test_npc".to_string(),
            },
        );

        world.add_map(map);
        world.set_current_map(1);

        let result = trigger_event(&mut world, pos);
        assert!(result.is_ok());
        match result.unwrap() {
            EventResult::NpcDialogue { npc_id } => {
                assert_eq!(npc_id, "test_npc");
            }
            _ => panic!("Expected NpcDialogue event"),
        }

        // NPC dialogue should still be there (repeatable)
        let result2 = trigger_event(&mut world, pos);
        assert!(result2.is_ok());
        matches!(
            result2.unwrap(),
            EventResult::NpcDialogue {
                npc_id: ref id
            } if id == "test_npc"
        );
    }

    #[test]
    fn test_event_out_of_bounds() {
        let mut world = World::new();
        let map = Map::new(1, "Map".to_string(), "Desc".to_string(), 20, 20);
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
        let mut map = Map::new(1, "Map".to_string(), "Desc".to_string(), 20, 20);

        let pos1 = Position::new(5, 5);
        let pos2 = Position::new(10, 10);
        let pos3 = Position::new(15, 15);

        map.add_event(
            pos1,
            MapEvent::Sign {
                name: "Sign".to_string(),
                description: "Desc".to_string(),
                text: "North".to_string(),
            },
        );
        map.add_event(
            pos2,
            MapEvent::Treasure {
                name: "Treasure".to_string(),
                description: "Desc".to_string(),
                loot: vec![1, 2],
            },
        );
        map.add_event(
            pos3,
            MapEvent::Trap {
                name: "Trap".to_string(),
                description: "Desc".to_string(),
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

    #[test]
    fn test_enter_inn_event() {
        let mut world = World::new();
        let mut map = Map::new(1, "Map".to_string(), "Desc".to_string(), 20, 20);

        let pos = Position::new(10, 10);
        map.add_event(
            pos,
            MapEvent::EnterInn {
                name: "Cozy Inn".to_string(),
                description: "A welcoming inn".to_string(),
                innkeeper_id: "cozy_inn".to_string(),
            },
        );

        world.add_map(map);
        world.set_current_map(1);

        let result = trigger_event(&mut world, pos);
        assert!(result.is_ok());
        match result.unwrap() {
            EventResult::EnterInn { innkeeper_id } => {
                assert_eq!(innkeeper_id, "cozy_inn".to_string());
            }
            _ => panic!("Expected EnterInn event"),
        }

        // Inn entrance should still be there (repeatable)
        let result2 = trigger_event(&mut world, pos);
        assert!(result2.is_ok());
        match result2.unwrap() {
            EventResult::EnterInn { innkeeper_id } => {
                assert_eq!(innkeeper_id, "cozy_inn".to_string());
            }
            _ => panic!("Expected EnterInn event"),
        }
    }

    #[test]
    fn test_enter_inn_event_with_different_inn_ids() {
        let mut world = World::new();
        let mut map = Map::new(1, "Map".to_string(), "Desc".to_string(), 20, 20);

        let pos1 = Position::new(5, 5);
        let pos2 = Position::new(10, 10);
        let pos3 = Position::new(15, 15);

        // Add three different inns
        map.add_event(
            pos1,
            MapEvent::EnterInn {
                name: "Cozy Inn".to_string(),
                description: "A warm inn".to_string(),
                innkeeper_id: "cozy_inn".to_string(),
            },
        );
        map.add_event(
            pos2,
            MapEvent::EnterInn {
                name: "Dragon's Rest Inn".to_string(),
                description: "An upscale inn".to_string(),
                innkeeper_id: "dragons_rest".to_string(),
            },
        );
        map.add_event(
            pos3,
            MapEvent::EnterInn {
                name: "Wayfarer's Lodge".to_string(),
                description: "A rustic inn".to_string(),
                innkeeper_id: "wayfarers_lodge".to_string(),
            },
        );

        world.add_map(map);
        world.set_current_map(1);

        // Trigger each inn entrance and verify correct innkeeper_id
        let r1 = trigger_event(&mut world, pos1);
        assert!(
            matches!(r1, Ok(EventResult::EnterInn { innkeeper_id }) if innkeeper_id == "cozy_inn")
        );

        let r2 = trigger_event(&mut world, pos2);
        assert!(
            matches!(r2, Ok(EventResult::EnterInn { innkeeper_id }) if innkeeper_id == "dragons_rest")
        );

        let r3 = trigger_event(&mut world, pos3);
        assert!(
            matches!(r3, Ok(EventResult::EnterInn { innkeeper_id }) if innkeeper_id == "wayfarers_lodge")
        );

        // Verify all inn entrances are repeatable
        assert!(matches!(
            trigger_event(&mut world, pos1),
            Ok(EventResult::EnterInn { innkeeper_id }) if innkeeper_id == "cozy_inn"
        ));
        assert!(matches!(
            trigger_event(&mut world, pos2),
            Ok(EventResult::EnterInn { innkeeper_id }) if innkeeper_id == "dragons_rest"
        ));
        assert!(matches!(
            trigger_event(&mut world, pos3),
            Ok(EventResult::EnterInn { innkeeper_id }) if innkeeper_id == "wayfarers_lodge"
        ));
    }
}
