// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Combat system - turn-based combat mechanics
//!
//! This module contains all combat-related data structures and logic including
//! combat state management, monster definitions, attack types, and combat resolution.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.4 for complete specifications.

pub mod database;
pub mod engine;
pub mod monster;
pub mod spell_casting;
pub mod types;

pub use database::{MonsterDatabase, MonsterDatabaseError, MonsterDefinition};
pub use engine::*;
pub use monster::*;
pub use types::*;
