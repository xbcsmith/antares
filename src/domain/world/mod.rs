// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! World system - Maps, tiles, and locations
//!
//! This module contains all world-related data structures and logic including
//! tiles, maps, movement, and event handling.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.2 for complete specifications.
//!
//! # Module Organization
//!
//! - `types`: Core world data structures (Tile, Map, World, etc.)
//! - `movement`: Party movement and navigation logic
//! - `events`: Map event handling system

pub mod blueprint;
mod events;
mod movement;
pub mod npc;
mod types;

pub use blueprint::MapBlueprint;
pub use events::{trigger_event, EventError, EventResult};
pub use movement::{check_tile_blocked, move_party, MovementError};
pub use npc::{NpcDefinition, NpcId, NpcPlacement};
pub use types::{Map, MapEvent, TerrainType, Tile, WallType, World};
