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

use super::types::{EncounterGroup, MapEvent, World};
use crate::domain::combat::types::CombatEventType;
use crate::domain::types::{GameTime, ItemId, Position};
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
        /// Type of combat event — governs how the battle begins and what
        /// special mechanics apply.  Forwarded from [`MapEvent::Encounter`]
        /// or from the selected [`EncounterGroup`] in the encounter table.
        combat_event_type: CombatEventType,
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
    /// Enter a merchant shop
    EnterMerchant {
        /// Merchant NPC identifier (NpcId string)
        npc_id: crate::domain::world::NpcId,
    },
    /// Furniture or props event triggered
    Furniture {
        /// Type of furniture
        furniture_type: crate::domain::world::FurnitureType,
    },
    /// Open an interactive container (chest, barrel, crate, etc.)
    EnterContainer {
        /// Unique identifier for this container instance (matches `MapEvent::Container::id`).
        container_event_id: String,
        /// Display name shown in the container inventory right-panel header.
        container_name: String,
        /// Current item contents of the container.
        items: Vec<crate::domain::character::InventorySlot>,
    },
    /// A dropped item is lying on the ground at this position and may be picked up.
    ///
    /// Returned by [`trigger_event`] when `map.dropped_items_at(position)` is
    /// non-empty and no other [`MapEvent`] is registered at that tile.  Only the
    /// first item (FIFO insertion order) is surfaced per call; the caller must
    /// re-trigger the event to surface additional stacked items.
    PickupItem {
        /// Logical item identifier from the item database.
        item_id: ItemId,
        /// Remaining charges on the item (`0` = non-magical / fully consumed).
        charges: u8,
        /// Tile coordinate where the item is lying.
        position: Position,
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

/// Triggers an event at the specified position, gated by the current game time.
///
/// This function checks if there's an event at the given position on the current map
/// and processes it, returning the result. Events are typically one-time occurrences
/// and are removed from the map after being triggered (except for repeatable events
/// like signs and NPC dialogues).
///
/// If the event carries a `time_condition`, it is evaluated against `game_time`.
/// When the condition is not satisfied the function returns
/// [`EventResult::None`] without consuming the event — the event remains in place
/// and will be re-evaluated on the next visit.
///
/// # Arguments
///
/// * `world`     - The game world containing maps and events
/// * `position`  - The position to check for events
/// * `game_time` - The current in-game clock, used to evaluate [`TimeCondition`]
///   gates on applicable event variants (`Encounter`, `Sign`, `NpcDialogue`,
///   `RecruitableCharacter`).
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
/// use antares::domain::types::{GameTime, Position};
///
/// let mut world = World::new();
/// let mut map = Map::new(1, "Test Map".to_string(), "Description".to_string(), 20, 20);
///
/// // Add a sign event (no time condition — always fires)
/// let pos = Position::new(10, 10);
/// map.add_event(pos, MapEvent::Sign {
///     name: "Dungeon Sign".to_string(),
///     description: "A weathered sign".to_string(),
///     text: "Welcome to the dungeon!".to_string(),
///     time_condition: None,
///     facing: None,
/// });
///
/// world.add_map(map);
/// world.set_current_map(1);
///
/// // Trigger the event — pass the current game time
/// let game_time = GameTime::new(1, 12, 0);
/// let result = trigger_event(&mut world, pos, &game_time);
/// assert!(result.is_ok());
/// match result.unwrap() {
///     EventResult::Sign { text } => assert_eq!(text, "Welcome to the dungeon!"),
///     _ => panic!("Expected Sign event"),
/// }
/// ```
pub fn trigger_event(
    world: &mut World,
    position: Position,
    game_time: &GameTime,
) -> Result<EventResult, EventError> {
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

    // Check for dropped items BEFORE the event HashMap lookup.
    //
    // When items are lying on the ground at this tile and no static map event
    // is registered here, surface the first item (FIFO insertion order) for
    // pickup.  If a static event IS present it takes priority; the party will
    // encounter the event first and can interact with dropped items afterwards.
    let dropped_at = map.dropped_items_at(position);
    if !dropped_at.is_empty() && map.get_event(position).is_none() {
        let first = dropped_at[0];
        return Ok(EventResult::PickupItem {
            item_id: first.item_id,
            charges: first.charges,
            position,
        });
    }

    // Check if there's an event at this position
    let event = match map.get_event(position) {
        Some(event) => event.clone(),
        None => return Ok(EventResult::None),
    };

    // Evaluate optional time condition before processing the event.
    // If the condition is not met we return None without consuming the event so
    // that it can fire on a future visit when the time is right.
    let time_condition_met = match &event {
        MapEvent::Encounter {
            time_condition: Some(tc),
            ..
        } => tc.is_met(game_time),
        MapEvent::Sign {
            time_condition: Some(tc),
            ..
        } => tc.is_met(game_time),
        MapEvent::NpcDialogue {
            time_condition: Some(tc),
            ..
        } => tc.is_met(game_time),
        MapEvent::RecruitableCharacter {
            time_condition: Some(tc),
            ..
        } => tc.is_met(game_time),
        // All other variants, and any variant whose time_condition is None,
        // are unconditionally allowed.
        _ => true,
    };

    if !time_condition_met {
        return Ok(EventResult::None);
    }

    // Process the event based on its type
    let result = match event {
        MapEvent::Encounter {
            monster_group,
            combat_event_type,
            time_condition: _,
            ..
        } => EventResult::Encounter {
            monster_group,
            combat_event_type,
        },

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

        MapEvent::Sign {
            text,
            time_condition: _,
            ..
        } => {
            // Signs are repeatable - don't remove
            EventResult::Sign { text }
        }

        MapEvent::NpcDialogue {
            npc_id,
            time_condition: _,
            ..
        } => {
            // NPC dialogues are repeatable - don't remove
            EventResult::NpcDialogue {
                npc_id: npc_id.clone(),
            }
        }

        MapEvent::RecruitableCharacter {
            character_id,
            time_condition: _,
            ..
        } => {
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

        MapEvent::Furniture { furniture_type, .. } => {
            // Furniture events are repeatable - don't remove
            EventResult::Furniture { furniture_type }
        }

        MapEvent::Container {
            id, name, items, ..
        } => {
            // Container events are repeatable (persist partial takes via write-back on close)
            EventResult::EnterContainer {
                container_event_id: id.clone(),
                container_name: name.clone(),
                items: items.clone(),
            }
        }

        MapEvent::DroppedItem { .. } => {
            // DroppedItem events are handled by `load_map_dropped_items_system`
            // at map load time.  Triggering the event at runtime (e.g. party
            // steps on the tile) is a no-op here — pickup is handled via a
            // dedicated action in a later phase.
            EventResult::None
        }
    };

    Ok(result)
}

/// Roll for a random encounter based on the world's current map encounter table
/// and the terrain modifier at the party position.  Returns `Some(EncounterGroup)`
/// when an encounter should occur — the group contains both the monster list and
/// the [`CombatEventType`] for that group.  Returns `None` when no encounter fires.
///
/// `R` is the RNG implementation used by the project's random helper (e.g. `rand::rng()`).
///
/// # Examples
///
/// ```
/// use antares::domain::world::{World, Map, random_encounter};
/// use antares::domain::world::{EncounterGroup, EncounterTable};
/// use antares::domain::combat::types::CombatEventType;
/// use antares::domain::types::Position;
///
/// let mut world = World::new();
/// let mut map = Map::new(1, "Forest".to_string(), "Dark forest".to_string(), 20, 20);
/// map.allow_random_encounters = true;
/// map.encounter_table = Some(EncounterTable {
///     encounter_rate: 1.0, // guaranteed
///     groups: vec![EncounterGroup::with_type(vec![1], CombatEventType::Ambush)],
///     terrain_modifiers: Default::default(),
/// });
/// world.add_map(map);
/// world.set_current_map(1);
///
/// let mut rng = rand::rng();
/// let result = random_encounter(&world, &mut rng);
/// assert!(result.is_some());
/// let group = result.unwrap();
/// assert_eq!(group.combat_event_type, CombatEventType::Ambush);
/// ```
pub fn random_encounter<R: rand::Rng>(world: &World, rng: &mut R) -> Option<EncounterGroup> {
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
    use crate::domain::character::InventorySlot;
    use crate::domain::types::TimeOfDay;
    use crate::domain::world::types::EncounterTable;
    use crate::domain::world::{Map, TimeCondition};

    /// Convenience: a neutral daytime clock for tests that do not care about time.
    fn noon() -> GameTime {
        GameTime::new(1, 12, 0)
    }

    /// A night-time clock (22:00) for time-condition tests.
    fn night() -> GameTime {
        GameTime::new(1, 22, 0)
    }

    #[test]
    fn test_no_event() {
        let mut world = World::new();
        let map = Map::new(1, "Test Map".to_string(), "Description".to_string(), 20, 20);
        world.add_map(map);
        world.set_current_map(1);

        let pos = Position::new(10, 10);
        let result = trigger_event(&mut world, pos, &noon());
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
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
                combat_event_type: CombatEventType::Normal,
            },
        );

        world.add_map(map);
        world.set_current_map(1);

        let result = trigger_event(&mut world, pos, &noon());
        assert!(result.is_ok());
        match result.unwrap() {
            EventResult::Encounter {
                monster_group,
                combat_event_type,
            } => {
                assert_eq!(monster_group, vec![1, 2, 3]);
                assert_eq!(combat_event_type, CombatEventType::Normal);
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

        let result = trigger_event(&mut world, pos, &noon());
        assert!(result.is_ok());
        match result.unwrap() {
            EventResult::Treasure { loot } => {
                assert_eq!(loot, vec![10, 20, 30]);
            }
            _ => panic!("Expected Treasure event"),
        }

        // Treasure should be removed after collection
        let result2 = trigger_event(&mut world, pos, &noon());
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

        let result = trigger_event(&mut world, pos, &noon());
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

        let result = trigger_event(&mut world, pos, &noon());
        assert!(result.is_ok());
        match result.unwrap() {
            EventResult::Trap { damage, effect } => {
                assert_eq!(damage, 25);
                assert_eq!(effect, Some("Poisoned".to_string()));
            }
            _ => panic!("Expected Trap event"),
        }

        // Trap should be removed after triggering
        let result2 = trigger_event(&mut world, pos, &noon());
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
                time_condition: None,
                facing: None,
            },
        );

        world.add_map(map);
        world.set_current_map(1);

        let result = trigger_event(&mut world, pos, &noon());
        assert!(result.is_ok());
        match result.unwrap() {
            EventResult::Sign { text } => {
                assert_eq!(text, sign_text);
            }
            _ => panic!("Expected Sign event"),
        }
    }

    // ── TimeCondition integration tests ──────────────────────────────────────

    /// An Encounter with DuringPeriods([Night]) must fire at night.
    #[test]
    fn test_time_condition_night_fires_at_night() {
        let mut world = World::new();
        let mut map = Map::new(1, "Map".to_string(), "Desc".to_string(), 20, 20);
        let pos = Position::new(5, 5);
        map.add_event(
            pos,
            MapEvent::Encounter {
                name: "Night Ambush".to_string(),
                description: String::new(),
                monster_group: vec![1, 2],
                time_condition: Some(TimeCondition::DuringPeriods(vec![TimeOfDay::Night])),
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
                combat_event_type: CombatEventType::Normal,
            },
        );
        world.add_map(map);
        world.set_current_map(1);

        // hour=23 → Night → condition met → Encounter result
        let result = trigger_event(&mut world, pos, &GameTime::new(1, 23, 0));
        assert!(
            matches!(result, Ok(EventResult::Encounter { .. })),
            "night-only encounter must fire at hour 23, got {:?}",
            result
        );
    }

    /// The same Encounter must return None at noon (Afternoon period).
    #[test]
    fn test_time_condition_night_skips_at_noon() {
        let mut world = World::new();
        let mut map = Map::new(1, "Map".to_string(), "Desc".to_string(), 20, 20);
        let pos = Position::new(5, 5);
        map.add_event(
            pos,
            MapEvent::Encounter {
                name: "Night Ambush".to_string(),
                description: String::new(),
                monster_group: vec![1, 2],
                time_condition: Some(TimeCondition::DuringPeriods(vec![TimeOfDay::Night])),
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
                combat_event_type: CombatEventType::Normal,
            },
        );
        world.add_map(map);
        world.set_current_map(1);

        // hour=12 → Afternoon → condition NOT met → None
        let result = trigger_event(&mut world, pos, &GameTime::new(1, 12, 0));
        assert_eq!(
            result.unwrap(),
            EventResult::None,
            "night-only encounter must return None at noon"
        );
    }

    /// AfterDay(5) fires on day 10, does not fire on day 3.
    #[test]
    fn test_time_condition_after_day_fires() {
        let mut world = World::new();
        let mut map = Map::new(1, "Map".to_string(), "Desc".to_string(), 20, 20);
        let pos = Position::new(3, 3);
        map.add_event(
            pos,
            MapEvent::Sign {
                name: "Ancient Warning".to_string(),
                description: String::new(),
                text: "Only after the fifth day shall this be revealed.".to_string(),
                time_condition: Some(TimeCondition::AfterDay(5)),
                facing: None,
            },
        );
        world.add_map(map);
        world.set_current_map(1);

        // Day 10 > 5 → fires
        let ok = trigger_event(&mut world, pos, &GameTime::new(10, 12, 0));
        assert!(
            matches!(ok, Ok(EventResult::Sign { .. })),
            "AfterDay(5) must fire on day 10"
        );

        // Day 3 is NOT > 5 → None
        let skip = trigger_event(&mut world, pos, &GameTime::new(3, 12, 0));
        assert_eq!(
            skip.unwrap(),
            EventResult::None,
            "AfterDay(5) must not fire on day 3"
        );

        // Day 5 is NOT > 5 (strict) → None
        let boundary = trigger_event(&mut world, pos, &GameTime::new(5, 12, 0));
        assert_eq!(
            boundary.unwrap(),
            EventResult::None,
            "AfterDay(5) must not fire on day 5 itself"
        );
    }

    /// BetweenHours fires at from/to boundaries but not outside.
    #[test]
    fn test_time_condition_between_hours() {
        let mut world = World::new();
        let mut map = Map::new(1, "Map".to_string(), "Desc".to_string(), 20, 20);
        let pos = Position::new(7, 7);
        map.add_event(
            pos,
            MapEvent::NpcDialogue {
                name: "Day Merchant".to_string(),
                description: String::new(),
                npc_id: "merchant_01".to_string(),
                time_condition: Some(TimeCondition::BetweenHours { from: 8, to: 18 }),
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
            },
        );
        world.add_map(map);
        world.set_current_map(1);

        // At the from boundary (hour=8) → fires
        assert!(
            matches!(
                trigger_event(&mut world, pos, &GameTime::new(1, 8, 0)),
                Ok(EventResult::NpcDialogue { .. })
            ),
            "BetweenHours{{8,18}} must fire at hour 8 (from boundary)"
        );

        // Mid-range (hour=13) → fires
        assert!(
            matches!(
                trigger_event(&mut world, pos, &GameTime::new(1, 13, 0)),
                Ok(EventResult::NpcDialogue { .. })
            ),
            "BetweenHours{{8,18}} must fire at hour 13"
        );

        // At the to boundary (hour=18) → fires
        assert!(
            matches!(
                trigger_event(&mut world, pos, &GameTime::new(1, 18, 0)),
                Ok(EventResult::NpcDialogue { .. })
            ),
            "BetweenHours{{8,18}} must fire at hour 18 (to boundary)"
        );

        // Just outside (hour=7) → None
        assert_eq!(
            trigger_event(&mut world, pos, &GameTime::new(1, 7, 0)).unwrap(),
            EventResult::None,
            "BetweenHours{{8,18}} must return None at hour 7"
        );

        // Just outside (hour=19) → None
        assert_eq!(
            trigger_event(&mut world, pos, &GameTime::new(1, 19, 0)).unwrap(),
            EventResult::None,
            "BetweenHours{{8,18}} must return None at hour 19"
        );
    }

    /// An event with no time_condition must always fire regardless of the clock.
    #[test]
    fn test_no_time_condition_always_fires() {
        let mut world = World::new();
        let mut map = Map::new(1, "Map".to_string(), "Desc".to_string(), 20, 20);
        let pos = Position::new(4, 4);
        map.add_event(
            pos,
            MapEvent::Sign {
                name: "Eternal Sign".to_string(),
                description: String::new(),
                text: "Always visible".to_string(),
                time_condition: None,
                facing: None,
            },
        );
        world.add_map(map);
        world.set_current_map(1);

        for hour in 0u8..24 {
            let result = trigger_event(&mut world, pos, &GameTime::new(1, hour, 0));
            assert!(
                matches!(result, Ok(EventResult::Sign { .. })),
                "unconditional Sign must fire at every hour; failed at hour {hour}"
            );
        }
    }

    /// A time-gated event that fails its condition must NOT be consumed —
    /// it must still be present on the map after the failed trigger.
    #[test]
    fn test_time_condition_not_met_does_not_consume_event() {
        let mut world = World::new();
        let mut map = Map::new(1, "Map".to_string(), "Desc".to_string(), 20, 20);
        let pos = Position::new(2, 2);
        map.add_event(
            pos,
            MapEvent::Encounter {
                name: "Night Only".to_string(),
                description: String::new(),
                monster_group: vec![5],
                time_condition: Some(TimeCondition::DuringPeriods(vec![TimeOfDay::Night])),
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
                combat_event_type: CombatEventType::Normal,
            },
        );
        world.add_map(map);
        world.set_current_map(1);

        // Fails at noon — event must not be removed
        let _ = trigger_event(&mut world, pos, &noon());
        // Now succeeds at night
        let result = trigger_event(&mut world, pos, &night());
        assert!(
            matches!(result, Ok(EventResult::Encounter { .. })),
            "encounter must still be present after a failed (noon) trigger"
        );
    }

    // ===== Phase 1: CombatEventType threading tests =====

    #[test]
    fn test_combat_event_type_default_is_normal() {
        assert_eq!(CombatEventType::default(), CombatEventType::Normal);
    }

    #[test]
    fn test_map_event_encounter_ron_round_trip() {
        use crate::domain::world::types::MapEvent;
        let event = MapEvent::Encounter {
            name: "Test".to_string(),
            description: String::new(),
            monster_group: vec![1, 2],
            time_condition: None,
            facing: None,
            proximity_facing: false,
            rotation_speed: None,
            combat_event_type: CombatEventType::Ambush,
        };
        let serialized = ron::to_string(&event).expect("serialize");
        let deserialized: MapEvent = ron::from_str(&serialized).expect("deserialize");
        match deserialized {
            MapEvent::Encounter {
                combat_event_type, ..
            } => assert_eq!(combat_event_type, CombatEventType::Ambush),
            _ => panic!("Expected Encounter"),
        }
    }

    #[test]
    fn test_map_event_encounter_ron_backward_compat() {
        // Old RON without combat_event_type — must default to Normal.
        let ron_str = r#"Encounter(
            name: "Goblins",
            description: "",
            monster_group: [1, 2],
        )"#;
        let event: crate::domain::world::types::MapEvent =
            ron::from_str(ron_str).expect("backward compat deserialize");
        match event {
            crate::domain::world::types::MapEvent::Encounter {
                combat_event_type, ..
            } => assert_eq!(combat_event_type, CombatEventType::Normal),
            _ => panic!("Expected Encounter"),
        }
    }

    #[test]
    fn test_event_result_encounter_carries_type() {
        let mut world = World::new();
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        let pos = Position::new(5, 5);
        map.add_event(
            pos,
            crate::domain::world::types::MapEvent::Encounter {
                name: "Ambush".to_string(),
                description: String::new(),
                monster_group: vec![3],
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
                combat_event_type: CombatEventType::Ambush,
            },
        );
        world.add_map(map);
        world.set_current_map(1);

        let result = trigger_event(&mut world, pos, &noon()).expect("trigger_event");
        match result {
            EventResult::Encounter {
                combat_event_type, ..
            } => assert_eq!(combat_event_type, CombatEventType::Ambush),
            _ => panic!("Expected Encounter result"),
        }
    }

    #[test]
    fn test_encounter_group_ron_round_trip() {
        use crate::domain::world::types::EncounterGroup;
        let group = EncounterGroup::with_type(vec![1, 2, 3], CombatEventType::Ranged);
        let serialized = ron::to_string(&group).expect("serialize");
        let deserialized: EncounterGroup = ron::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized.monster_group, vec![1, 2, 3]);
        assert_eq!(deserialized.combat_event_type, CombatEventType::Ranged);
    }

    #[test]
    fn test_random_encounter_returns_group_type() {
        use crate::domain::world::types::EncounterGroup;

        let mut world = World::new();
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        map.allow_random_encounters = true;
        map.encounter_table = Some(EncounterTable {
            encounter_rate: 1.0, // guaranteed
            groups: vec![EncounterGroup::with_type(vec![5], CombatEventType::Ambush)],
            terrain_modifiers: Default::default(),
        });
        world.add_map(map);
        world.set_current_map(1);

        // Run enough times to get a hit given rate == 1.0
        let mut rng = rand::rng();
        let result = random_encounter(&world, &mut rng);
        assert!(result.is_some(), "Expected encounter at rate 1.0");
        let group = result.unwrap();
        assert_eq!(group.combat_event_type, CombatEventType::Ambush);
        assert_eq!(group.monster_group, vec![5]);
    }

    #[test]
    fn test_random_encounter_triggers_standalone() {
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
        let table = crate::domain::world::EncounterTable {
            encounter_rate: 1.0,
            groups: vec![crate::domain::world::types::EncounterGroup::new(vec![
                1u8, 2u8, 3u8,
            ])],
            ..Default::default()
        };

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
        let group = result.unwrap();
        assert_eq!(group.monster_group, vec![1u8, 2u8, 3u8]);
        assert_eq!(
            group.combat_event_type,
            CombatEventType::Normal,
            "default group type should be Normal"
        );
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

        let table = crate::domain::world::EncounterTable {
            encounter_rate: 1.0,
            groups: vec![crate::domain::world::types::EncounterGroup::new(vec![1u8])],
            ..Default::default()
        };

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
        let result = trigger_event(&mut world, pos, &noon());
        assert!(result.is_ok());

        // Assert: EnterInn event contains the correct innkeeper_id
        match result.unwrap() {
            EventResult::EnterInn { innkeeper_id: id } => {
                assert_eq!(id, innkeeper_id);
            }
            _ => panic!("Expected EnterInn event"),
        }

        // EnterInn events are repeatable; triggering again should yield the same result
        let result2 = trigger_event(&mut world, pos, &noon());
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
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
            },
        );

        world.add_map(map);
        world.set_current_map(1);

        let result = trigger_event(&mut world, pos, &noon());
        assert!(result.is_ok());
        match result.unwrap() {
            EventResult::NpcDialogue { npc_id } => {
                assert_eq!(npc_id, "test_npc");
            }
            _ => panic!("Expected NpcDialogue event"),
        }

        // NPC dialogue should still be there (repeatable)
        let result2 = trigger_event(&mut world, pos, &noon());
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
        let result = trigger_event(&mut world, pos, &noon());
        assert!(result.is_err());
        assert!(matches!(result, Err(EventError::OutOfBounds(25, 25))));
    }

    #[test]
    fn test_event_map_not_found() {
        let mut world = World::new();
        world.set_current_map(99); // Non-existent map

        let pos = Position::new(10, 10);
        let result = trigger_event(&mut world, pos, &noon());
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
                time_condition: None,
                facing: None,
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
        let r1 = trigger_event(&mut world, pos1, &noon());
        assert!(matches!(r1, Ok(EventResult::Sign { .. })));

        let r2 = trigger_event(&mut world, pos2, &noon());
        assert!(matches!(r2, Ok(EventResult::Treasure { .. })));

        let r3 = trigger_event(&mut world, pos3, &noon());
        assert!(matches!(r3, Ok(EventResult::Trap { .. })));

        // Verify one-time events are removed
        assert_eq!(
            trigger_event(&mut world, pos2, &noon()).unwrap(),
            EventResult::None
        );
        assert_eq!(
            trigger_event(&mut world, pos3, &noon()).unwrap(),
            EventResult::None
        );

        // Verify repeatable event remains
        assert!(matches!(
            trigger_event(&mut world, pos1, &noon()),
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

        let result = trigger_event(&mut world, pos, &noon());
        assert!(result.is_ok());
        match result.unwrap() {
            EventResult::EnterInn { innkeeper_id } => {
                assert_eq!(innkeeper_id, "cozy_inn".to_string());
            }
            _ => panic!("Expected EnterInn event"),
        }

        // Inn entrance should still be there (repeatable)
        let result2 = trigger_event(&mut world, pos, &noon());
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
        let r1 = trigger_event(&mut world, pos1, &noon());
        assert!(
            matches!(r1, Ok(EventResult::EnterInn { innkeeper_id }) if innkeeper_id == "cozy_inn")
        );

        let r2 = trigger_event(&mut world, pos2, &noon());
        assert!(
            matches!(r2, Ok(EventResult::EnterInn { innkeeper_id }) if innkeeper_id == "dragons_rest")
        );

        let r3 = trigger_event(&mut world, pos3, &noon());
        assert!(
            matches!(r3, Ok(EventResult::EnterInn { innkeeper_id }) if innkeeper_id == "wayfarers_lodge")
        );

        // Verify all inn entrances are repeatable
        assert!(matches!(
            trigger_event(&mut world, pos1, &noon()),
            Ok(EventResult::EnterInn { innkeeper_id }) if innkeeper_id == "cozy_inn"
        ));
        assert!(matches!(
            trigger_event(&mut world, pos2, &noon()),
            Ok(EventResult::EnterInn { innkeeper_id }) if innkeeper_id == "dragons_rest"
        ));
        assert!(matches!(
            trigger_event(&mut world, pos3, &noon()),
            Ok(EventResult::EnterInn { innkeeper_id }) if innkeeper_id == "wayfarers_lodge"
        ));
    }

    #[test]
    fn test_container_event_returns_enter_container_result() {
        // Arrange
        let mut world = World::new();
        let mut map = Map::new(1, "Test Map".to_string(), "Desc".to_string(), 20, 20);

        let pos = Position::new(3, 3);
        let items = vec![
            InventorySlot {
                item_id: 1,
                charges: 0,
            },
            InventorySlot {
                item_id: 2,
                charges: 3,
            },
        ];
        map.add_event(
            pos,
            MapEvent::Container {
                id: "test_chest_001".to_string(),
                name: "Old Chest".to_string(),
                description: "A dusty old chest".to_string(),
                items: items.clone(),
            },
        );

        world.add_map(map);
        world.set_current_map(1);

        // Act
        let result = trigger_event(&mut world, pos, &noon());
        assert!(result.is_ok());

        // Assert
        match result.unwrap() {
            EventResult::EnterContainer {
                container_event_id,
                container_name,
                items: result_items,
            } => {
                assert_eq!(container_event_id, "test_chest_001");
                assert_eq!(container_name, "Old Chest");
                assert_eq!(result_items.len(), 2);
                assert_eq!(result_items[0].item_id, 1);
                assert_eq!(result_items[1].item_id, 2);
                assert_eq!(result_items[1].charges, 3);
            }
            _ => panic!("Expected EnterContainer result"),
        }
    }

    #[test]
    fn test_container_event_is_repeatable() {
        // Container events must NOT be removed after triggering so re-interacting works.
        let mut world = World::new();
        let mut map = Map::new(1, "Test Map".to_string(), "Desc".to_string(), 20, 20);

        let pos = Position::new(5, 5);
        map.add_event(
            pos,
            MapEvent::Container {
                id: "barrel_42".to_string(),
                name: "Old Barrel".to_string(),
                description: "".to_string(),
                items: vec![InventorySlot {
                    item_id: 10,
                    charges: 0,
                }],
            },
        );

        world.add_map(map);
        world.set_current_map(1);

        // First trigger
        let r1 = trigger_event(&mut world, pos, &noon());
        assert!(matches!(r1, Ok(EventResult::EnterContainer { .. })));

        // Second trigger: event must still be present
        let r2 = trigger_event(&mut world, pos, &noon());
        assert!(
            matches!(r2, Ok(EventResult::EnterContainer { .. })),
            "Container event must be repeatable"
        );
    }

    // ===== PickupItem tests =====

    /// `trigger_event` returns `PickupItem` when dropped items exist and no
    /// static map event is registered at that tile.
    #[test]
    fn test_trigger_event_returns_pickup_when_item_present() {
        use crate::domain::world::{DroppedItem, Map};

        let mut world = World::new();
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        let pos = crate::domain::types::Position::new(5, 5);

        map.add_dropped_item(DroppedItem {
            item_id: 7,
            charges: 3,
            position: pos,
            map_id: 1,
        });

        world.add_map(map);
        world.set_current_map(1);

        let game_time = GameTime::new(1, 12, 0);
        let result = trigger_event(&mut world, pos, &game_time).expect("trigger must succeed");

        match result {
            EventResult::PickupItem {
                item_id,
                charges,
                position,
            } => {
                assert_eq!(item_id, 7);
                assert_eq!(charges, 3);
                assert_eq!(position, pos);
            }
            other => panic!("expected PickupItem, got {:?}", other),
        }
    }

    /// `trigger_event` returns the static map event (not `PickupItem`) when
    /// both a dropped item and a static event exist at the same tile.
    #[test]
    fn test_trigger_event_static_event_takes_priority_over_dropped_item() {
        use crate::domain::world::{DroppedItem, Map, MapEvent};

        let mut world = World::new();
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        let pos = crate::domain::types::Position::new(3, 3);

        // Both a sign and a dropped item at the same tile
        map.add_event(
            pos,
            MapEvent::Sign {
                name: "A sign".to_string(),
                description: "desc".to_string(),
                text: "Hello".to_string(),
                time_condition: None,
                facing: None,
            },
        );
        map.add_dropped_item(DroppedItem {
            item_id: 9,
            charges: 1,
            position: pos,
            map_id: 1,
        });

        world.add_map(map);
        world.set_current_map(1);

        let game_time = GameTime::new(1, 12, 0);
        let result = trigger_event(&mut world, pos, &game_time).expect("trigger must succeed");

        // Sign takes priority
        assert!(
            matches!(result, EventResult::Sign { .. }),
            "expected Sign, got {:?}",
            result
        );
    }

    /// `trigger_event` returns `None` when there are no dropped items and no
    /// static map event at the queried position.
    #[test]
    fn test_trigger_event_none_when_no_event_and_no_dropped_items() {
        use crate::domain::world::Map;

        let mut world = World::new();
        let map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        world.add_map(map);
        world.set_current_map(1);

        let pos = crate::domain::types::Position::new(7, 7);
        let game_time = GameTime::new(1, 12, 0);
        let result = trigger_event(&mut world, pos, &game_time).expect("trigger must succeed");

        assert_eq!(result, EventResult::None);
    }

    /// `trigger_event` returns the first stacked dropped item (FIFO).
    #[test]
    fn test_trigger_event_pickup_fifo_ordering() {
        use crate::domain::world::{DroppedItem, Map};

        let mut world = World::new();
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);
        let pos = crate::domain::types::Position::new(2, 2);

        // Two items stacked on the same tile; item_id 4 was added first
        map.add_dropped_item(DroppedItem {
            item_id: 4,
            charges: 0,
            position: pos,
            map_id: 1,
        });
        map.add_dropped_item(DroppedItem {
            item_id: 8,
            charges: 1,
            position: pos,
            map_id: 1,
        });

        world.add_map(map);
        world.set_current_map(1);

        let game_time = GameTime::new(1, 12, 0);
        let result = trigger_event(&mut world, pos, &game_time).expect("trigger must succeed");

        // Should surface item_id 4 first (FIFO)
        match result {
            EventResult::PickupItem { item_id, .. } => assert_eq!(item_id, 4),
            other => panic!("expected PickupItem, got {:?}", other),
        }
    }

    #[test]
    fn test_container_event_empty_items() {
        // A container with no items still produces EnterContainer (not None).
        let mut world = World::new();
        let mut map = Map::new(1, "Map".to_string(), "Desc".to_string(), 20, 20);

        let pos = Position::new(2, 2);
        map.add_event(
            pos,
            MapEvent::Container {
                id: "empty_crate".to_string(),
                name: "Empty Crate".to_string(),
                description: "".to_string(),
                items: vec![],
            },
        );

        world.add_map(map);
        world.set_current_map(1);

        let result = trigger_event(&mut world, pos, &noon());
        match result.unwrap() {
            EventResult::EnterContainer { items, .. } => {
                assert!(items.is_empty());
            }
            _ => panic!("Expected EnterContainer for empty container"),
        }
    }
}
