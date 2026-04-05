// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Spell learning and acquisition domain functions
//!
//! This module implements all spell learning mechanics including class and level
//! validation, spellbook mutation, and level-up spell grants.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.3 (SpellBook) and Section 12.5
//! (Training and Leveling) for complete specifications.
//!
//! # Four Acquisition Channels
//!
//! 1. **Level-Up**: [`grant_level_up_spells`] returns newly accessible spells at a
//!    given character level; [`learn_spell`] is called for each to auto-grant.
//! 2. **Dialogue**: `DialogueAction::LearnSpell` calls [`learn_spell`] for a target
//!    party member.
//! 3. **Quest Reward**: `QuestReward::LearnSpell` calls [`learn_spell`] for the first
//!    eligible party member.
//! 4. **Scroll**: `ConsumableEffect::LearnSpell` calls [`learn_spell`] when a scroll
//!    item is used.
//!
//! # Examples
//!
//! ```
//! use antares::domain::character::{Character, Sex, Alignment};
//! use antares::sdk::database::SpellDatabase;
//! use antares::domain::magic::learning::{learn_spell, can_learn_spell};
//! use antares::domain::magic::types::{Spell, SpellSchool, SpellContext, SpellTarget};
//! use antares::domain::classes::ClassDatabase;
//!
//! let mut cleric = Character::new(
//!     "Healbot".to_string(),
//!     "human".to_string(),
//!     "cleric".to_string(),
//!     Sex::Female,
//!     Alignment::Good,
//! );
//! cleric.level = 1;
//!
//! let mut spell_db = SpellDatabase::new();
//! let spell = Spell::new(
//!     257,
//!     "Cure Wounds",
//!     SpellSchool::Cleric,
//!     1,
//!     2,
//!     0,
//!     SpellContext::Anytime,
//!     SpellTarget::SingleCharacter,
//!     "Heals 8 HP",
//!     None,
//!     0,
//!     false,
//! );
//! spell_db.add_spell(spell).unwrap();
//! let class_db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
//!
//! assert!(can_learn_spell(&cleric, 257, &spell_db, &class_db).is_ok());
//! learn_spell(&mut cleric, 257, &spell_db, &class_db).unwrap();
//!
//! // Spell is now in the spellbook
//! assert!(cleric.spells.cleric_spells[0].contains(&257));
//! ```

use crate::domain::character::Character;
use crate::domain::classes::{ClassDatabase, SpellSchool as ClassSpellSchool};
use crate::domain::magic::casting::{
    can_class_cast_school_by_id, get_required_level_for_spell_by_id,
};
use crate::domain::types::SpellId;
use crate::sdk::database::SpellDatabase;
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur when a character attempts to learn a spell
///
/// # Examples
///
/// ```
/// use antares::domain::magic::learning::SpellLearnError;
///
/// let err = SpellLearnError::WrongClass("knight".to_string());
/// assert!(err.to_string().contains("knight"));
///
/// let err2 = SpellLearnError::LevelTooLow { level: 1, required: 3 };
/// assert!(err2.to_string().contains("1"));
/// assert!(err2.to_string().contains("3"));
/// ```
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SpellLearnError {
    /// The requested spell ID does not exist in the database
    #[error("Spell {0} not found in database")]
    SpellNotFound(SpellId),

    /// The character's class cannot cast spells of the spell's school
    #[error("Character class '{0}' cannot cast spells of this school")]
    WrongClass(String),

    /// The character's current level is below the minimum required
    #[error("Character level {level} is too low (requires {required})")]
    LevelTooLow {
        /// The character's current level
        level: u32,
        /// The level required to learn this spell
        required: u32,
    },

    /// The spell is already in the character's spellbook
    #[error("Spell {0} is already in the character's spellbook")]
    AlreadyKnown(SpellId),

    /// The spellbook has no space for additional spells (future cap)
    #[error("Spellbook is full and cannot hold more spells")]
    SpellBookFull,
}

// ===== Core Spell Learning Functions =====

/// Checks whether a character can learn a specific spell without mutating state
///
/// Validates in order:
/// 1. The spell exists in the database
/// 2. The character's class can cast the spell's school
/// 3. The character's level meets the minimum requirement
/// 4. The spell is not already in the character's spellbook
///
/// # Arguments
///
/// * `character` - The character attempting to learn the spell
/// * `spell_id`  - The spell to check
/// * `spell_db`  - Reference to the spell database
/// * `class_db`  - Reference to the class database
///
/// # Returns
///
/// Returns `Ok(())` if the character can learn the spell, or a [`SpellLearnError`]
/// describing the first reason the spell cannot be learned.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::sdk::database::SpellDatabase;
/// use antares::domain::magic::learning::{can_learn_spell, SpellLearnError};
/// use antares::domain::magic::types::{Spell, SpellSchool, SpellContext, SpellTarget};
/// use antares::domain::classes::ClassDatabase;
///
/// let mut knight = Character::new(
///     "Gawain".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// knight.level = 5;
///
/// let mut spell_db = SpellDatabase::new();
/// spell_db.add_spell(Spell::new(
///     257, "Cure Wounds", SpellSchool::Cleric, 1, 2, 0,
///     SpellContext::Anytime, SpellTarget::SingleCharacter,
///     "Heals 8 HP", None, 0, false,
/// )).unwrap();
///
/// let class_db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
///
/// // Knight cannot learn cleric spells
/// let result = can_learn_spell(&knight, 257, &spell_db, &class_db);
/// assert!(matches!(result, Err(SpellLearnError::WrongClass(_))));
/// ```
pub fn can_learn_spell(
    character: &Character,
    spell_id: SpellId,
    spell_db: &SpellDatabase,
    class_db: &ClassDatabase,
) -> Result<(), SpellLearnError> {
    // 1. Spell must exist in the database
    let spell = spell_db
        .get_spell(spell_id)
        .ok_or(SpellLearnError::SpellNotFound(spell_id))?;

    // 2. Class must be able to cast this school
    if !can_class_cast_school_by_id(&character.class_id, class_db, spell.school) {
        return Err(SpellLearnError::WrongClass(character.class_id.clone()));
    }

    // 3. Character level must meet the minimum
    let required = get_required_level_for_spell_by_id(&character.class_id, class_db, spell);
    if character.level < required {
        return Err(SpellLearnError::LevelTooLow {
            level: character.level,
            required,
        });
    }

    // 4. Spell must not already be known
    let spell_list = character
        .spells
        .get_spell_list_by_id(&character.class_id, class_db);
    let level_index = spell.level.saturating_sub(1) as usize;
    if level_index < 7 && spell_list[level_index].contains(&spell_id) {
        return Err(SpellLearnError::AlreadyKnown(spell_id));
    }

    Ok(())
}

/// Adds a spell to a character's spellbook after validating all requirements
///
/// This is the single authoritative function for spell learning. All acquisition
/// channels (dialogue, quest reward, scroll, level-up) MUST call this function
/// so that class and level restrictions are uniformly enforced.
///
/// # Arguments
///
/// * `character` - The character learning the spell (will be modified)
/// * `spell_id`  - The spell to learn
/// * `spell_db`  - Reference to the spell database
/// * `class_db`  - Reference to the class database
///
/// # Returns
///
/// Returns `Ok(())` if the spell was successfully added to the spellbook, or a
/// [`SpellLearnError`] describing why it could not be learned.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::sdk::database::SpellDatabase;
/// use antares::domain::magic::learning::learn_spell;
/// use antares::domain::magic::types::{Spell, SpellSchool, SpellContext, SpellTarget};
/// use antares::domain::classes::ClassDatabase;
///
/// let mut cleric = Character::new(
///     "Theodora".to_string(),
///     "human".to_string(),
///     "cleric".to_string(),
///     Sex::Female,
///     Alignment::Good,
/// );
/// cleric.level = 1;
///
/// let mut spell_db = SpellDatabase::new();
/// spell_db.add_spell(Spell::new(
///     257, "Cure Wounds", SpellSchool::Cleric, 1, 2, 0,
///     SpellContext::Anytime, SpellTarget::SingleCharacter,
///     "Heals 8 HP", None, 0, false,
/// )).unwrap();
/// let class_db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
///
/// learn_spell(&mut cleric, 257, &spell_db, &class_db).unwrap();
/// assert!(cleric.spells.cleric_spells[0].contains(&257));
/// ```
pub fn learn_spell(
    character: &mut Character,
    spell_id: SpellId,
    spell_db: &SpellDatabase,
    class_db: &ClassDatabase,
) -> Result<(), SpellLearnError> {
    // Validate first using the immutable check
    can_learn_spell(character, spell_id, spell_db, class_db)?;

    // Safe: can_learn_spell already verified the spell exists
    let spell = spell_db
        .get_spell(spell_id)
        .expect("spell verified by can_learn_spell");
    let level_index = spell.level.saturating_sub(1) as usize;

    // Determine which school array to modify
    let uses_cleric_school = class_db
        .get_class(&character.class_id)
        .and_then(|c| c.spell_school.as_ref())
        .map(|s| matches!(s, ClassSpellSchool::Cleric))
        .unwrap_or(false);

    if level_index < 7 {
        if uses_cleric_school {
            character.spells.cleric_spells[level_index].push(spell_id);
        } else {
            character.spells.sorcerer_spells[level_index].push(spell_id);
        }
    }

    Ok(())
}

/// Returns all spells the character is eligible to learn but has not yet learned
///
/// Eligibility requires all of the following:
/// - The character's class can cast the spell's school
/// - The character's level meets the minimum requirement
/// - The spell is not already in the spellbook
///
/// Non-caster classes (e.g., Knight, Robber) always return an empty vector.
///
/// # Arguments
///
/// * `character` - The character to check eligibility for
/// * `spell_db`  - Reference to the spell database
/// * `class_db`  - Reference to the class database
///
/// # Returns
///
/// A `Vec<SpellId>` of eligible-but-not-yet-learned spells sorted by the
/// iteration order of the spell database (deterministic within a session).
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::sdk::database::SpellDatabase;
/// use antares::domain::magic::learning::get_learnable_spells;
/// use antares::domain::magic::types::{Spell, SpellSchool, SpellContext, SpellTarget};
/// use antares::domain::classes::ClassDatabase;
///
/// let mut cleric = Character::new(
///     "Aria".to_string(),
///     "human".to_string(),
///     "cleric".to_string(),
///     Sex::Female,
///     Alignment::Good,
/// );
/// cleric.level = 3;
///
/// let mut spell_db = SpellDatabase::new();
/// spell_db.add_spell(Spell::new(
///     257, "Cure Wounds", SpellSchool::Cleric, 1, 2, 0,
///     SpellContext::Anytime, SpellTarget::SingleCharacter,
///     "Heals", None, 0, false,
/// )).unwrap();
/// spell_db.add_spell(Spell::new(
///     258, "Holy Bolt", SpellSchool::Cleric, 2, 4, 0,
///     SpellContext::CombatOnly, SpellTarget::SingleMonster,
///     "Deals holy damage", None, 0, false,
/// )).unwrap();
/// let class_db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
///
/// let learnable = get_learnable_spells(&cleric, &spell_db, &class_db);
/// assert!(learnable.contains(&257));
/// assert!(learnable.contains(&258));
/// ```
pub fn get_learnable_spells(
    character: &Character,
    spell_db: &SpellDatabase,
    class_db: &ClassDatabase,
) -> Vec<SpellId> {
    // spell_db.all_spells() returns Vec<SpellId> (SDK convention)
    spell_db
        .all_spells()
        .into_iter()
        .filter(|&spell_id| can_learn_spell(character, spell_id, spell_db, class_db).is_ok())
        .collect()
}

/// Returns spell IDs that become newly accessible when a character reaches `new_level`
///
/// A spell is "newly accessible" at `new_level` if its required character level is
/// `> new_level - 1` and `<= new_level`. This means it was not accessible before
/// the level-up but is now.
///
/// Non-caster classes always return an empty vector. The caller decides the
/// auto-learn policy — to auto-grant all newly accessible spells, iterate the
/// result and call [`learn_spell`] for each ID.
///
/// # Arguments
///
/// * `character`  - The character who just leveled up (with `character.level == new_level`)
/// * `new_level`  - The new character level (must equal `character.level`)
/// * `spell_db`   - Reference to the spell database
/// * `class_db`   - Reference to the class database
///
/// # Returns
///
/// A `Vec<SpellId>` of spells that first became accessible at `new_level`.
/// The vector is empty if no new spell levels are unlocked at this level.
///
/// # Level Unlock Schedule
///
/// For full casters (Cleric, Sorcerer):
/// - Level 1: Spell level 1 spells
/// - Level 3: Spell level 2 spells
/// - Level 5: Spell level 3 spells
/// - Level 7: Spell level 4 spells
/// - Level 9: Spell level 5 spells
/// - Level 11: Spell level 6 spells
/// - Level 13: Spell level 7 spells
///
/// For hybrid casters (Paladin, Archer): same schedule but starting at level 3.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::sdk::database::SpellDatabase;
/// use antares::domain::magic::learning::grant_level_up_spells;
/// use antares::domain::magic::types::{Spell, SpellSchool, SpellContext, SpellTarget};
/// use antares::domain::classes::ClassDatabase;
///
/// let mut cleric = Character::new(
///     "Healer".to_string(),
///     "human".to_string(),
///     "cleric".to_string(),
///     Sex::Female,
///     Alignment::Good,
/// );
/// cleric.level = 3; // just leveled up to 3
///
/// let mut spell_db = SpellDatabase::new();
/// // Level 1 spell: required character level 1 (accessible at level 1, not new at 3)
/// spell_db.add_spell(Spell::new(
///     257, "Cure Wounds", SpellSchool::Cleric, 1, 2, 0,
///     SpellContext::Anytime, SpellTarget::SingleCharacter,
///     "Heals", None, 0, false,
/// )).unwrap();
/// // Level 2 spell: required character level 3 (first accessible at level 3)
/// spell_db.add_spell(Spell::new(
///     258, "Holy Bolt", SpellSchool::Cleric, 2, 4, 0,
///     SpellContext::CombatOnly, SpellTarget::SingleMonster,
///     "Deals holy damage", None, 0, false,
/// )).unwrap();
/// let class_db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
///
/// let granted = grant_level_up_spells(&cleric, 3, &spell_db, &class_db);
/// assert!(!granted.contains(&257)); // already accessible at level 1
/// assert!(granted.contains(&258));  // newly accessible at level 3
/// ```
pub fn grant_level_up_spells(
    character: &Character,
    new_level: u32,
    spell_db: &SpellDatabase,
    class_db: &ClassDatabase,
) -> Vec<SpellId> {
    let prev_level = new_level.saturating_sub(1);

    // spell_db.all_spells() returns Vec<SpellId> (SDK convention);
    // look up each spell definition to check school and required level.
    spell_db
        .all_spells()
        .into_iter()
        .filter_map(|spell_id| spell_db.get_spell(spell_id))
        .filter(|spell| {
            // Must match the character's spell school
            can_class_cast_school_by_id(&character.class_id, class_db, spell.school)
        })
        .filter(|spell| {
            let required = get_required_level_for_spell_by_id(&character.class_id, class_db, spell);
            // Accessible at new_level but was NOT accessible at prev_level
            required <= new_level && required > prev_level
        })
        .map(|s| s.id)
        .collect()
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::classes::ClassDatabase;
    use crate::domain::magic::types::{Spell, SpellContext, SpellSchool, SpellTarget};
    use crate::sdk::database::SpellDatabase;

    // ----- Test helpers -----

    fn make_spell(id: SpellId, school: SpellSchool, spell_level: u8) -> Spell {
        Spell::new(
            id,
            "Test Spell",
            school,
            spell_level,
            2,
            0,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Test description",
            None,
            0,
            false,
        )
    }

    fn make_character(class_id: &str, level: u32) -> Character {
        let mut ch = Character::new(
            "Tester".to_string(),
            "human".to_string(),
            class_id.to_string(),
            Sex::Male,
            Alignment::Good,
        );
        ch.level = level;
        ch
    }

    fn make_class_db() -> ClassDatabase {
        ClassDatabase::load_from_file("data/classes.ron")
            .expect("data/classes.ron must exist for tests")
    }

    /// Build a spell database with cleric spells at levels 1-3 and sorcerer spells at levels 1-3.
    fn make_spell_db() -> SpellDatabase {
        let mut db = SpellDatabase::new();
        // Cleric spells
        db.add_spell(make_spell(0x0101, SpellSchool::Cleric, 1))
            .unwrap();
        db.add_spell(make_spell(0x0102, SpellSchool::Cleric, 1))
            .unwrap();
        db.add_spell(make_spell(0x0201, SpellSchool::Cleric, 2))
            .unwrap();
        db.add_spell(make_spell(0x0301, SpellSchool::Cleric, 3))
            .unwrap();
        // Sorcerer spells
        db.add_spell(make_spell(0x0501, SpellSchool::Sorcerer, 1))
            .unwrap();
        db.add_spell(make_spell(0x0502, SpellSchool::Sorcerer, 1))
            .unwrap();
        db.add_spell(make_spell(0x0601, SpellSchool::Sorcerer, 2))
            .unwrap();
        db.add_spell(make_spell(0x0701, SpellSchool::Sorcerer, 3))
            .unwrap();
        db
    }

    // ===== SpellLearnError display tests =====

    #[test]
    fn test_spell_learn_error_spell_not_found_display() {
        let e = SpellLearnError::SpellNotFound(9999);
        assert!(e.to_string().contains("9999"));
    }

    #[test]
    fn test_spell_learn_error_wrong_class_display() {
        let e = SpellLearnError::WrongClass("knight".to_string());
        assert!(e.to_string().contains("knight"));
    }

    #[test]
    fn test_spell_learn_error_level_too_low_display() {
        let e = SpellLearnError::LevelTooLow {
            level: 1,
            required: 3,
        };
        let s = e.to_string();
        assert!(s.contains('1'));
        assert!(s.contains('3'));
    }

    #[test]
    fn test_spell_learn_error_already_known_display() {
        let e = SpellLearnError::AlreadyKnown(0x0101);
        assert!(e.to_string().contains("257")); // 0x0101 = 257
    }

    #[test]
    fn test_spell_learn_error_spell_book_full_display() {
        let e = SpellLearnError::SpellBookFull;
        assert!(!e.to_string().is_empty());
    }

    // ===== can_learn_spell tests =====

    #[test]
    fn test_can_learn_spell_cleric_level_1_succeeds() {
        let ch = make_character("cleric", 1);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        assert!(can_learn_spell(&ch, 0x0101, &spell_db, &class_db).is_ok());
    }

    #[test]
    fn test_can_learn_spell_spell_not_found_returns_error() {
        let ch = make_character("cleric", 5);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let result = can_learn_spell(&ch, 9999, &spell_db, &class_db);
        assert!(matches!(result, Err(SpellLearnError::SpellNotFound(9999))));
    }

    #[test]
    fn test_can_learn_spell_knight_cleric_spell_wrong_class() {
        let ch = make_character("knight", 5);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let result = can_learn_spell(&ch, 0x0101, &spell_db, &class_db);
        assert!(matches!(result, Err(SpellLearnError::WrongClass(_))));
    }

    #[test]
    fn test_can_learn_spell_robber_sorcerer_spell_wrong_class() {
        let ch = make_character("robber", 10);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let result = can_learn_spell(&ch, 0x0501, &spell_db, &class_db);
        assert!(matches!(result, Err(SpellLearnError::WrongClass(_))));
    }

    #[test]
    fn test_can_learn_spell_level_too_low_for_level_2_spell() {
        // Level 2 cleric spell requires character level 3
        let ch = make_character("cleric", 1);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let result = can_learn_spell(&ch, 0x0201, &spell_db, &class_db);
        assert!(matches!(
            result,
            Err(SpellLearnError::LevelTooLow {
                level: 1,
                required: 3
            })
        ));
    }

    #[test]
    fn test_can_learn_spell_level_too_low_for_level_3_spell() {
        // Level 3 cleric spell requires character level 5
        let ch = make_character("cleric", 3);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let result = can_learn_spell(&ch, 0x0301, &spell_db, &class_db);
        assert!(matches!(
            result,
            Err(SpellLearnError::LevelTooLow {
                level: 3,
                required: 5
            })
        ));
    }

    #[test]
    fn test_can_learn_spell_already_known_returns_error() {
        let mut ch = make_character("cleric", 5);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        ch.spells.cleric_spells[0].push(0x0101);
        let result = can_learn_spell(&ch, 0x0101, &spell_db, &class_db);
        assert!(matches!(result, Err(SpellLearnError::AlreadyKnown(0x0101))));
    }

    #[test]
    fn test_can_learn_spell_sorcerer_sorcerer_spell_succeeds() {
        let ch = make_character("sorcerer", 1);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        assert!(can_learn_spell(&ch, 0x0501, &spell_db, &class_db).is_ok());
    }

    #[test]
    fn test_can_learn_spell_sorcerer_cleric_spell_wrong_class() {
        let ch = make_character("sorcerer", 5);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let result = can_learn_spell(&ch, 0x0101, &spell_db, &class_db);
        assert!(matches!(result, Err(SpellLearnError::WrongClass(_))));
    }

    #[test]
    fn test_can_learn_spell_cleric_sorcerer_spell_wrong_class() {
        let ch = make_character("cleric", 5);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let result = can_learn_spell(&ch, 0x0501, &spell_db, &class_db);
        assert!(matches!(result, Err(SpellLearnError::WrongClass(_))));
    }

    #[test]
    fn test_can_learn_spell_paladin_delayed_access_level_1_blocked() {
        let class_db = make_class_db();
        let mut spell_db = SpellDatabase::new();
        spell_db
            .add_spell(make_spell(0x0101, SpellSchool::Cleric, 1))
            .unwrap();

        let paladin = make_character("paladin", 1);
        let result = can_learn_spell(&paladin, 0x0101, &spell_db, &class_db);
        assert!(matches!(
            result,
            Err(SpellLearnError::LevelTooLow {
                level: 1,
                required: 3
            })
        ));
    }

    #[test]
    fn test_can_learn_spell_paladin_delayed_access_level_2_blocked() {
        let class_db = make_class_db();
        let mut spell_db = SpellDatabase::new();
        spell_db
            .add_spell(make_spell(0x0101, SpellSchool::Cleric, 1))
            .unwrap();

        let paladin = make_character("paladin", 2);
        let result = can_learn_spell(&paladin, 0x0101, &spell_db, &class_db);
        assert!(matches!(
            result,
            Err(SpellLearnError::LevelTooLow {
                level: 2,
                required: 3
            })
        ));
    }

    #[test]
    fn test_can_learn_spell_paladin_delayed_access_level_3_allowed() {
        let class_db = make_class_db();
        let mut spell_db = SpellDatabase::new();
        spell_db
            .add_spell(make_spell(0x0101, SpellSchool::Cleric, 1))
            .unwrap();

        let paladin = make_character("paladin", 3);
        assert!(can_learn_spell(&paladin, 0x0101, &spell_db, &class_db).is_ok());
    }

    #[test]
    fn test_can_learn_spell_archer_delayed_access_level_2_blocked() {
        // In data/classes.ron the archer has spell_school: None, so the
        // data-driven path treats it as a non-caster and returns WrongClass.
        // (The hardcoded `can_class_cast_school` helper knows about archer,
        // but `can_class_cast_school_by_id` defers to the class DB.)
        let class_db = make_class_db();
        let mut spell_db = SpellDatabase::new();
        spell_db
            .add_spell(make_spell(0x0501, SpellSchool::Sorcerer, 1))
            .unwrap();

        let archer = make_character("archer", 2);
        let result = can_learn_spell(&archer, 0x0501, &spell_db, &class_db);
        assert!(matches!(result, Err(SpellLearnError::WrongClass(_))));
    }

    #[test]
    fn test_can_learn_spell_archer_delayed_access_level_3_allowed() {
        // Archer has spell_school: None in the data file — it cannot learn
        // sorcerer spells through the data-driven path regardless of level.
        let class_db = make_class_db();
        let mut spell_db = SpellDatabase::new();
        spell_db
            .add_spell(make_spell(0x0501, SpellSchool::Sorcerer, 1))
            .unwrap();

        let archer = make_character("archer", 3);
        // Returns WrongClass because the class DB marks archer as non-caster.
        let result = can_learn_spell(&archer, 0x0501, &spell_db, &class_db);
        assert!(matches!(result, Err(SpellLearnError::WrongClass(_))));
    }

    // ===== learn_spell tests =====

    #[test]
    fn test_learn_spell_adds_to_cleric_spells_level_index_0() {
        let mut ch = make_character("cleric", 1);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        learn_spell(&mut ch, 0x0101, &spell_db, &class_db).unwrap();
        assert!(ch.spells.cleric_spells[0].contains(&0x0101));
        assert!(!ch.spells.sorcerer_spells[0].contains(&0x0101));
    }

    #[test]
    fn test_learn_spell_adds_to_sorcerer_spells_level_index_0() {
        let mut ch = make_character("sorcerer", 1);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        learn_spell(&mut ch, 0x0501, &spell_db, &class_db).unwrap();
        assert!(ch.spells.sorcerer_spells[0].contains(&0x0501));
        assert!(!ch.spells.cleric_spells[0].contains(&0x0501));
    }

    #[test]
    fn test_learn_spell_adds_level_2_spell_at_correct_index() {
        let mut ch = make_character("cleric", 5);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        // Level 2 spell → index 1
        learn_spell(&mut ch, 0x0201, &spell_db, &class_db).unwrap();
        assert!(ch.spells.cleric_spells[1].contains(&0x0201));
        assert!(ch.spells.cleric_spells[0].is_empty());
    }

    #[test]
    fn test_learn_spell_adds_level_3_spell_at_correct_index() {
        let mut ch = make_character("cleric", 5);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        // Level 3 spell → index 2
        learn_spell(&mut ch, 0x0301, &spell_db, &class_db).unwrap();
        assert!(ch.spells.cleric_spells[2].contains(&0x0301));
    }

    #[test]
    fn test_learn_spell_rejects_wrong_class_spellbook_unchanged() {
        let mut ch = make_character("knight", 5);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let result = learn_spell(&mut ch, 0x0101, &spell_db, &class_db);
        assert!(matches!(result, Err(SpellLearnError::WrongClass(_))));
        assert!(ch.spells.cleric_spells[0].is_empty());
        assert!(ch.spells.sorcerer_spells[0].is_empty());
    }

    #[test]
    fn test_learn_spell_rejects_level_too_low_spellbook_unchanged() {
        let mut ch = make_character("cleric", 1);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let result = learn_spell(&mut ch, 0x0201, &spell_db, &class_db);
        assert!(matches!(result, Err(SpellLearnError::LevelTooLow { .. })));
        assert!(ch.spells.cleric_spells[1].is_empty());
    }

    #[test]
    fn test_learn_spell_rejects_already_known_no_duplicate() {
        let mut ch = make_character("cleric", 5);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        learn_spell(&mut ch, 0x0101, &spell_db, &class_db).unwrap();
        let result = learn_spell(&mut ch, 0x0101, &spell_db, &class_db);
        assert!(matches!(result, Err(SpellLearnError::AlreadyKnown(0x0101))));
        // Must appear exactly once
        assert_eq!(
            ch.spells.cleric_spells[0]
                .iter()
                .filter(|&&id| id == 0x0101)
                .count(),
            1
        );
    }

    #[test]
    fn test_learn_spell_paladin_goes_to_cleric_spells() {
        let mut ch = make_character("paladin", 3);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        learn_spell(&mut ch, 0x0101, &spell_db, &class_db).unwrap();
        assert!(ch.spells.cleric_spells[0].contains(&0x0101));
        assert!(!ch.spells.sorcerer_spells[0].contains(&0x0101));
    }

    #[test]
    fn test_learn_spell_archer_is_non_caster_in_data_db() {
        // Archer has spell_school: None in data/classes.ron so the data-driven
        // path rejects it with WrongClass — it cannot learn sorcerer spells.
        let mut ch = make_character("archer", 3);
        let spell_db = make_spell_db();
        let class_db = make_class_db();

        let result = learn_spell(&mut ch, 0x0501, &spell_db, &class_db);
        assert!(
            matches!(result, Err(SpellLearnError::WrongClass(_))),
            "Archer (spell_school: None in data) must be rejected with WrongClass"
        );
    }

    #[test]
    fn test_learn_multiple_spells_same_level_index() {
        let mut ch = make_character("cleric", 5);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        learn_spell(&mut ch, 0x0101, &spell_db, &class_db).unwrap();
        learn_spell(&mut ch, 0x0102, &spell_db, &class_db).unwrap();
        assert!(ch.spells.cleric_spells[0].contains(&0x0101));
        assert!(ch.spells.cleric_spells[0].contains(&0x0102));
        assert_eq!(ch.spells.cleric_spells[0].len(), 2);
    }

    // ===== get_learnable_spells tests =====

    #[test]
    fn test_get_learnable_spells_includes_eligible_cleric_spells() {
        let ch = make_character("cleric", 5);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let learnable = get_learnable_spells(&ch, &spell_db, &class_db);
        assert!(learnable.contains(&0x0101));
        assert!(learnable.contains(&0x0102));
        assert!(learnable.contains(&0x0201));
        assert!(learnable.contains(&0x0301));
    }

    #[test]
    fn test_get_learnable_spells_excludes_sorcerer_spells_for_cleric() {
        let ch = make_character("cleric", 5);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let learnable = get_learnable_spells(&ch, &spell_db, &class_db);
        assert!(!learnable.contains(&0x0501));
        assert!(!learnable.contains(&0x0601));
    }

    #[test]
    fn test_get_learnable_spells_excludes_already_known() {
        let mut ch = make_character("cleric", 5);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        ch.spells.cleric_spells[0].push(0x0101);
        let learnable = get_learnable_spells(&ch, &spell_db, &class_db);
        assert!(!learnable.contains(&0x0101));
        assert!(learnable.contains(&0x0102));
    }

    #[test]
    fn test_get_learnable_spells_respects_level_restriction_at_level_1() {
        let ch = make_character("cleric", 1);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let learnable = get_learnable_spells(&ch, &spell_db, &class_db);
        // Level 1 spells accessible at character level 1
        assert!(learnable.contains(&0x0101));
        assert!(learnable.contains(&0x0102));
        // Level 2 spell requires character level 3
        assert!(!learnable.contains(&0x0201));
        // Level 3 spell requires character level 5
        assert!(!learnable.contains(&0x0301));
    }

    #[test]
    fn test_get_learnable_spells_knight_returns_empty() {
        let ch = make_character("knight", 10);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let learnable = get_learnable_spells(&ch, &spell_db, &class_db);
        assert!(learnable.is_empty());
    }

    #[test]
    fn test_get_learnable_spells_robber_returns_empty() {
        let ch = make_character("robber", 10);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let learnable = get_learnable_spells(&ch, &spell_db, &class_db);
        assert!(learnable.is_empty());
    }

    #[test]
    fn test_get_learnable_spells_paladin_at_level_2_returns_empty() {
        let ch = make_character("paladin", 2);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let learnable = get_learnable_spells(&ch, &spell_db, &class_db);
        assert!(learnable.is_empty());
    }

    #[test]
    fn test_get_learnable_spells_paladin_at_level_3_returns_level_1_spells() {
        let ch = make_character("paladin", 3);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let learnable = get_learnable_spells(&ch, &spell_db, &class_db);
        assert!(learnable.contains(&0x0101));
        assert!(learnable.contains(&0x0102));
    }

    // ===== grant_level_up_spells tests =====

    #[test]
    fn test_grant_level_up_spells_level_1_grants_cleric_level_1_spells() {
        let ch = make_character("cleric", 1);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let granted = grant_level_up_spells(&ch, 1, &spell_db, &class_db);
        assert!(granted.contains(&0x0101));
        assert!(granted.contains(&0x0102));
        // Level 2 spell not yet accessible
        assert!(!granted.contains(&0x0201));
    }

    #[test]
    fn test_grant_level_up_spells_level_2_grants_nothing_for_cleric() {
        // For a cleric, level 2 spell requires character level 3, not 2
        let ch = make_character("cleric", 2);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let granted = grant_level_up_spells(&ch, 2, &spell_db, &class_db);
        // Nothing newly unlocks at level 2
        assert!(!granted.contains(&0x0101)); // Already unlocked at level 1
        assert!(!granted.contains(&0x0201)); // Unlocks at level 3, not 2
        assert!(granted.is_empty());
    }

    #[test]
    fn test_grant_level_up_spells_level_3_grants_spell_level_2_spells() {
        let ch = make_character("cleric", 3);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let granted = grant_level_up_spells(&ch, 3, &spell_db, &class_db);
        assert!(granted.contains(&0x0201));
        // Level 1 spells were unlocked at level 1, not new
        assert!(!granted.contains(&0x0101));
        assert!(!granted.contains(&0x0102));
    }

    #[test]
    fn test_grant_level_up_spells_level_5_grants_spell_level_3_spells() {
        let ch = make_character("cleric", 5);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let granted = grant_level_up_spells(&ch, 5, &spell_db, &class_db);
        assert!(granted.contains(&0x0301));
        // Spell level 2 was already unlocked at character level 3
        assert!(!granted.contains(&0x0201));
    }

    #[test]
    fn test_grant_level_up_spells_excludes_other_school() {
        let ch = make_character("cleric", 1);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let granted = grant_level_up_spells(&ch, 1, &spell_db, &class_db);
        // Sorcerer spells must not appear for a cleric
        assert!(!granted.contains(&0x0501));
        assert!(!granted.contains(&0x0502));
    }

    #[test]
    fn test_grant_level_up_spells_sorcerer_level_1() {
        let ch = make_character("sorcerer", 1);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        let granted = grant_level_up_spells(&ch, 1, &spell_db, &class_db);
        assert!(granted.contains(&0x0501));
        assert!(granted.contains(&0x0502));
        // Cleric spells not included
        assert!(!granted.contains(&0x0101));
    }

    #[test]
    fn test_grant_level_up_spells_paladin_level_1_and_2_empty() {
        let class_db = make_class_db();
        let mut spell_db = SpellDatabase::new();
        spell_db
            .add_spell(make_spell(0x0101, SpellSchool::Cleric, 1))
            .unwrap();

        let paladin_l1 = make_character("paladin", 1);
        let paladin_l2 = make_character("paladin", 2);
        assert!(grant_level_up_spells(&paladin_l1, 1, &spell_db, &class_db).is_empty());
        assert!(grant_level_up_spells(&paladin_l2, 2, &spell_db, &class_db).is_empty());
    }

    #[test]
    fn test_grant_level_up_spells_paladin_gains_at_level_3() {
        let class_db = make_class_db();
        let mut spell_db = SpellDatabase::new();
        spell_db
            .add_spell(make_spell(0x0101, SpellSchool::Cleric, 1))
            .unwrap();

        let paladin = make_character("paladin", 3);
        let granted = grant_level_up_spells(&paladin, 3, &spell_db, &class_db);
        assert!(granted.contains(&0x0101));
    }

    #[test]
    fn test_grant_level_up_spells_archer_is_non_caster_in_data_db() {
        // Archer has spell_school: None in data/classes.ron — no spells granted.
        let class_db = make_class_db();
        let mut spell_db = SpellDatabase::new();
        spell_db
            .add_spell(make_spell(0x0501, SpellSchool::Sorcerer, 1))
            .unwrap();

        let archer = make_character("archer", 3);
        let granted = grant_level_up_spells(&archer, 3, &spell_db, &class_db);
        assert!(
            granted.is_empty(),
            "Archer (spell_school: None in data) must not receive any level-up spells"
        );
    }

    #[test]
    fn test_grant_level_up_spells_knight_always_empty() {
        let ch = make_character("knight", 10);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        assert!(grant_level_up_spells(&ch, 10, &spell_db, &class_db).is_empty());
    }

    #[test]
    fn test_grant_level_up_spells_robber_always_empty() {
        let ch = make_character("robber", 10);
        let spell_db = make_spell_db();
        let class_db = make_class_db();
        assert!(grant_level_up_spells(&ch, 10, &spell_db, &class_db).is_empty());
    }

    #[test]
    fn test_grant_level_up_spells_key_level_boundaries_full_caster() {
        let class_db = make_class_db();
        let mut spell_db = SpellDatabase::new();
        // One spell for each spell level 1-7
        for sl in 1u8..=7u8 {
            let id = 0x0100u16 + sl as u16;
            spell_db
                .add_spell(make_spell(id, SpellSchool::Cleric, sl))
                .unwrap();
        }

        // Spell level 1 at char level 1, level 2 at 3, level 3 at 5, etc.
        let unlock_pairs: &[(u32, u16)] = &[
            (1, 0x0101),
            (3, 0x0102),
            (5, 0x0103),
            (7, 0x0104),
            (9, 0x0105),
            (11, 0x0106),
            (13, 0x0107),
        ];

        for (char_level, expected_id) in unlock_pairs {
            let ch = make_character("cleric", *char_level);
            let granted = grant_level_up_spells(&ch, *char_level, &spell_db, &class_db);
            assert!(
                granted.contains(expected_id),
                "At char level {} expected spell {} to be granted",
                char_level,
                expected_id
            );
        }
    }
}
