// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Item system - Weapons, armor, consumables, and item database
//!
//! This module provides the complete item system for the game, including:
//! - Item type definitions (weapons, armor, accessories, consumables, ammo, quest items)
//! - Item database loading and querying from RON files
//! - Equipment management and restrictions
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.5 and 7.2 for complete specifications.
//!
//! # Examples
//!
//! ```no_run
//! use antares::domain::items::{ItemDatabase, Item, ItemType, WeaponData};
//! use antares::domain::types::DiceRoll;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load item database from RON file
//! let items = ItemDatabase::load_from_file("data/items.ron")?;
//!
//! // Query item by ID
//! if let Some(item) = items.get_item(1) {
//!     println!("Found: {}", item.name);
//!     if item.is_weapon() {
//!         println!("This is a weapon");
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod database;
pub mod types;

// Re-export main types for convenience
pub use database::{ItemDatabase, ItemDatabaseError};
pub use types::{
    AccessoryData, AccessorySlot, AmmoData, AmmoType, ArmorData, AttributeType, Bonus,
    BonusAttribute, ConsumableData, ConsumableEffect, Disablement, Item, ItemType, QuestData,
    WeaponData,
};
