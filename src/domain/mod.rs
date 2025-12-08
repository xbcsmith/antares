// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Domain layer - Core game logic and data structures
//!
//! This module contains all the core game entities and business logic,
//! independent of any infrastructure concerns like I/O or rendering.
//!
//! # Module Organization
//!
//! - `types`: Core type aliases and supporting types (Position, Direction, etc.)
//! - `character`: Character system (stats, inventory, equipment, party)
//! - `world`: World system (maps, tiles, locations)
//! - `combat`: Combat system (monsters, attacks, combat state)
//! - `magic`: Magic system (spells, spell books, effects)
//! - `items`: Item system (weapons, armor, consumables)

pub mod character;
pub mod character_definition;
pub mod classes;
pub mod combat;
pub mod conditions;
pub mod dialogue;
pub mod items;
pub mod magic;
pub mod proficiency;
pub mod progression;
pub mod quest;
pub mod races;
pub mod resources;
pub mod types;
pub mod world;

// Re-export commonly used types from submodules
pub use character_definition::{
    BaseStats, CharacterDatabase, CharacterDefinition, CharacterDefinitionError,
    CharacterDefinitionId, StartingEquipment,
};
pub use dialogue::{DialogueId, NodeId};
pub use quest::QuestId;
pub use types::{CharacterId, EventId, ItemId, MapId, MonsterId, RaceId, SpellId, TownId};
pub use types::{DiceRoll, Direction, GameTime, Position};

// Re-export proficiency types
pub use proficiency::{
    has_proficiency_union, is_item_compatible_with_race, ProficiencyCategory, ProficiencyDatabase,
    ProficiencyDefinition, ProficiencyError, ProficiencyId,
};
