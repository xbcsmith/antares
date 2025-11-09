// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Antares - A classic turn-based RPG inspired by Might and Magic 1
//!
//! This library provides the core game engine, including character management,
//! combat systems, world exploration, and magic systems.
//!
//! # Architecture
//!
//! The game follows a layered architecture:
//! - **Domain Layer**: Core game logic and data structures
//! - **Application Layer**: Game state management and orchestration
//! - **Infrastructure Layer**: I/O, rendering, and persistence (future phases)
//!
/// # Example
///
/// ```
/// use antares::domain::types::{Position, Direction};
///
/// let start_pos = Position::new(5, 10);
/// let direction = Direction::North;
/// let new_pos = direction.forward(start_pos);
/// assert_eq!(new_pos, Position::new(5, 9));
/// ```
pub mod application;
pub mod domain;

// Re-export commonly used types for convenience
pub use domain::types::{CharacterId, EventId, ItemId, MapId, MonsterId, SpellId, TownId};
pub use domain::types::{DiceRoll, Direction, GameTime, Position};
