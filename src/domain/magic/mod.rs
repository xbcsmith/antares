// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Magic system module - Spells, casting, and spell effects
//!
//! This module implements the complete magic system including spell definitions,
//! casting validation, and spell effect resolution.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 5.3 for complete specifications.
//!
//! # Module Organization
//!
//! - `types` - Spell definitions, schools, contexts, and targets
//! - `casting` - Spell casting logic and validation
//!
//! # Examples
//!
//! ```
//! use antares::domain::character::{Character, Class, Race, Sex, Alignment};
//! use antares::domain::magic::types::{Spell, SpellSchool, SpellContext, SpellTarget};
//! use antares::domain::magic::casting::{can_cast_spell, cast_spell};
//! use antares::domain::types::GameMode;
//!
//! // Create a cleric character
//! let mut cleric = Character::new(
//!     "Healer".to_string(),
//!     Race::Human,
//!     Class::Cleric,
//!     Sex::Female,
//!     Alignment::Good,
//! );
//! cleric.level = 5;
//! cleric.sp.current = 20;
//!
//! // Define a healing spell
//! let cure_wounds = Spell::new(
//!     0x0101,
//!     "Cure Wounds",
//!     SpellSchool::Cleric,
//!     1,
//!     2,
//!     0,
//!     SpellContext::Anytime,
//!     SpellTarget::SingleCharacter,
//!     "Heals 8 hit points",
//!     None,
//!     0,
//!     false,
//! );
//!
//! // Check if the spell can be cast
//! let can_cast = can_cast_spell(&cleric, &cure_wounds, &GameMode::Exploration, false, false);
//! assert!(can_cast.is_ok());
//!
//! // Cast the spell
//! let result = cast_spell(&mut cleric, &cure_wounds);
//! assert!(result.success);
//! assert_eq!(cleric.sp.current, 18); // 20 - 2
//! ```

pub mod casting;
pub mod database;
pub mod spell_effects;
pub mod types;

// Re-export commonly used types
pub use casting::{
    calculate_spell_points, calculate_spell_points_by_id, can_cast_spell, can_class_cast_school,
    can_class_cast_school_by_id, cast_spell, get_required_level_for_spell,
    get_required_level_for_spell_by_id,
};
pub use database::{SpellDatabase, SpellDatabaseError};
pub use spell_effects::{
    apply_condition_dot_effects, apply_spell_conditions_to_character,
    apply_spell_conditions_to_monster,
};
pub use types::{Spell, SpellContext, SpellError, SpellResult, SpellSchool, SpellTarget};
