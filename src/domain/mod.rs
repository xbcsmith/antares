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
pub mod combat;
pub mod types;
pub mod world;

// Re-export commonly used types from submodules
pub use types::{CharacterId, EventId, ItemId, MapId, MonsterId, SpellId, TownId};
pub use types::{DiceRoll, Direction, GameTime, Position};
