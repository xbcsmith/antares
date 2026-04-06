// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Character progression system - Experience, leveling, and stat growth
//!
//! This module implements character advancement mechanics including
//! experience point awards, level-up checks, and class-based HP increases.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 5.2 for complete specifications.
//!
//! # Data-Driven Functions
//!
//! This module provides ID-based (data-driven) functions for character progression:
//! - `roll_hp_gain` - Roll HP gain based on class_id
//! - `level_up` - Level up a character using class_id
//!
//! Database variants (`*_from_db`) use `ClassDatabase` lookups for full extensibility.

use crate::domain::character::Character;
use crate::domain::classes::{ClassDatabase, ClassError};
use crate::domain::types::{DiceRoll, SpellId};
use crate::sdk::database::SpellDatabase;
use rand::Rng;
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur during character progression
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ProgressionError {
    #[error("Character is at maximum level")]
    MaxLevelReached,

    #[error("Not enough experience to level up (need {needed}, have {current})")]
    NotEnoughExperience { needed: u64, current: u64 },

    #[error("Character is dead and cannot gain experience")]
    CharacterDead,

    #[error("Class error: {0}")]
    ClassError(#[from] ClassError),
}

// ===== Experience Constants =====

/// Maximum character level
pub const MAX_LEVEL: u32 = 200;

/// Base experience for level 2
const BASE_XP: u64 = 1000;

/// Experience multiplier for exponential curve
const XP_MULTIPLIER: f64 = 1.5;

// ===== Experience and Leveling =====

/// Awards experience points to a character
///
/// Adds the specified amount of experience to the character.
/// Does not automatically level up - use `check_level_up` separately.
///
/// # Arguments
///
/// * `character` - The character receiving experience
/// * `amount` - Amount of experience to award
///
/// # Returns
///
/// Returns `Ok(())` if successful, or an error if the character is dead.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::progression::award_experience;
///
/// let mut knight = Character::new(
///     "Sir Lancelot".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
///
/// assert_eq!(knight.experience, 0);
/// award_experience(&mut knight, 500).unwrap();
/// assert_eq!(knight.experience, 500);
/// ```
pub fn award_experience(character: &mut Character, amount: u64) -> Result<(), ProgressionError> {
    // Dead characters cannot gain experience
    if character.conditions.is_fatal() {
        return Err(ProgressionError::CharacterDead);
    }

    character.experience = character.experience.saturating_add(amount);
    Ok(())
}

/// Checks if a character has enough experience to level up
///
/// # Arguments
///
/// * `character` - The character to check
///
/// # Returns
///
/// Returns `true` if the character can level up, `false` otherwise.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::progression::{award_experience, check_level_up};
///
/// let mut character = Character::new(
///     "Hero".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
///
/// assert!(!check_level_up(&character));
///
/// award_experience(&mut character, 10000).unwrap();
/// assert!(check_level_up(&character));
/// ```
pub fn check_level_up(character: &Character) -> bool {
    if character.level >= MAX_LEVEL {
        return false;
    }

    let required = experience_for_level(character.level + 1);
    character.experience >= required
}

/// Levels up a character
///
/// Increases the character's level by 1, rolls for HP gain based on class,
/// and updates spell points.
///
/// # Arguments
///
/// * `character` - The character to level up (will be modified)
/// * `rng` - Random number generator for HP rolls
///
/// # Returns
///
/// Returns `Ok(hp_gained)` with the amount of HP gained, or an error if
/// the character cannot level up.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::progression::{award_experience, level_up};
/// use rand::rng;
///
/// let mut knight = Character::new(
///     "Sir Lancelot".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
///
/// // Give enough XP to level up
/// award_experience(&mut knight, 10000).unwrap();
///
/// let mut rng = rng();
/// let hp_gained = level_up(&mut knight, &mut rng).unwrap();
///
/// assert_eq!(knight.level, 2);
/// assert!(hp_gained > 0);
/// ```
pub fn level_up(character: &mut Character, rng: &mut impl Rng) -> Result<u16, ProgressionError> {
    // Check if character can level up
    if character.level >= MAX_LEVEL {
        return Err(ProgressionError::MaxLevelReached);
    }

    let required = experience_for_level(character.level + 1);
    if character.experience < required {
        return Err(ProgressionError::NotEnoughExperience {
            needed: required,
            current: character.experience,
        });
    }

    // Increase level
    character.level += 1;

    // Roll for HP gain
    let hp_gain = roll_hp_gain(&character.class_id, rng);
    character.hp.base = character.hp.base.saturating_add(hp_gain);
    character.hp.current = character.hp.current.saturating_add(hp_gain);

    // Update spell points (if spellcaster)
    let new_sp = crate::domain::magic::casting::calculate_spell_points(character);
    if new_sp > 0 {
        let sp_gain = new_sp.saturating_sub(character.sp.base);
        character.sp.base = new_sp;
        character.sp.current = character.sp.current.saturating_add(sp_gain);
    }

    Ok(hp_gain)
}

/// Rolls HP gain for a character based on their class_id
///
/// Each class has different HP dice:
/// - Knight: 1d10
/// - Paladin: 1d8
/// - Archer: 1d8
/// - Cleric: 1d6
/// - Sorcerer: 1d4
/// - Robber: 1d6
///
/// # Arguments
///
/// * `class_id` - The character's class identifier
/// * `rng` - Random number generator
///
/// # Returns
///
/// Returns the HP gained (minimum 1).
///
/// # Examples
///
/// ```
/// use antares::domain::progression::roll_hp_gain;
/// use rand::rng;
///
/// let mut rng = rng();
///
/// let knight_hp = roll_hp_gain("knight", &mut rng);
/// assert!(knight_hp >= 1 && knight_hp <= 10);
///
/// let sorcerer_hp = roll_hp_gain("sorcerer", &mut rng);
/// assert!(sorcerer_hp >= 1 && sorcerer_hp <= 4);
/// ```
pub fn roll_hp_gain(class_id: &str, rng: &mut impl Rng) -> u16 {
    let dice = match class_id {
        "knight" => DiceRoll::new(1, 10, 0),  // 1d10
        "paladin" => DiceRoll::new(1, 8, 0),  // 1d8
        "archer" => DiceRoll::new(1, 8, 0),   // 1d8
        "cleric" => DiceRoll::new(1, 6, 0),   // 1d6
        "sorcerer" => DiceRoll::new(1, 4, 0), // 1d4
        "robber" => DiceRoll::new(1, 6, 0),   // 1d6
        _ => DiceRoll::new(1, 6, 0),          // Default: 1d6
    };

    dice.roll(rng).max(1) as u16
}

/// Rolls HP gain for a character based on class definition from database
///
/// This is the data-driven version that uses external class definitions.
/// Use this when working with campaign-specific or modded classes.
///
/// # Arguments
///
/// * `class_id` - The class ID to look up
/// * `class_db` - Reference to the class database
/// * `rng` - Random number generator
///
/// # Returns
///
/// Returns the HP gained (minimum 1), or an error if the class is not found.
///
/// # Examples
///
/// ```
/// use antares::domain::progression::roll_hp_gain_from_db;
/// use antares::domain::classes::ClassDatabase;
/// use rand::rng;
///
/// let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
/// let mut rng = rng();
///
/// let hp = roll_hp_gain_from_db("knight", &db, &mut rng).unwrap();
/// assert!(hp >= 1 && hp <= 10);
/// ```
pub fn roll_hp_gain_from_db(
    class_id: &str,
    class_db: &ClassDatabase,
    rng: &mut impl Rng,
) -> Result<u16, ProgressionError> {
    let class_def = class_db
        .get_class(class_id)
        .ok_or_else(|| ClassError::ClassNotFound(class_id.to_string()))?;

    let hp = class_def.hp_die.roll(rng).max(1) as u16;
    Ok(hp)
}

/// Levels up a character using ClassDatabase for HP calculation
///
/// This is the data-driven version that uses external class definitions.
/// Use this when working with campaign-specific or modded classes.
///
/// Increases the character's level by 1, rolls for HP gain based on class
/// definition from the database, and updates spell points.
///
/// # Arguments
///
/// * `character` - The character to level up (will be modified)
/// * `class_db` - Reference to the class database
/// * `rng` - Random number generator for HP rolls
///
/// # Returns
///
/// Returns `Ok(hp_gained)` with the amount of HP gained, or an error if
/// the character cannot level up or the class is not found.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::progression::{award_experience, level_up_from_db};
/// use antares::domain::classes::ClassDatabase;
/// use rand::rng;
///
/// let mut knight = Character::new(
///     "Sir Lancelot".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
///
/// // Give enough XP to level up
/// award_experience(&mut knight, 10000).unwrap();
///
/// let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
/// let mut rng = rng();
/// let hp_gained = level_up_from_db(&mut knight, &db, &mut rng).unwrap();
///
/// assert_eq!(knight.level, 2);
/// assert!(hp_gained > 0);
/// ```
pub fn level_up_from_db(
    character: &mut Character,
    class_db: &ClassDatabase,
    rng: &mut impl Rng,
) -> Result<u16, ProgressionError> {
    // Check if character can level up
    if character.level >= MAX_LEVEL {
        return Err(ProgressionError::MaxLevelReached);
    }

    let required = experience_for_level(character.level + 1);
    if character.experience < required {
        return Err(ProgressionError::NotEnoughExperience {
            needed: required,
            current: character.experience,
        });
    }

    // Increase level
    character.level += 1;

    // Roll for HP gain using class database
    let hp_gain = roll_hp_gain_from_db(&character.class_id, class_db, rng)?;
    character.hp.base = character.hp.base.saturating_add(hp_gain);
    character.hp.current = character.hp.current.saturating_add(hp_gain);

    // Update spell points using data-driven calculation
    let new_sp = crate::domain::magic::casting::calculate_spell_points_by_id(character, class_db);
    if new_sp > 0 {
        let sp_gain = new_sp.saturating_sub(character.sp.base);
        character.sp.base = new_sp;
        character.sp.current = character.sp.current.saturating_add(sp_gain);
    }

    Ok(hp_gain)
}

/// Levels up a character and auto-grants all newly accessible spells
///
/// This is the full level-up pipeline combining [`level_up_from_db`] with
/// [`crate::domain::magic::learning::grant_level_up_spells`]. After HP and SP
/// are updated, every spell that first becomes accessible at the new level is
/// automatically added to the character's spellbook via
/// [`crate::domain::magic::learning::learn_spell`].
///
/// Non-caster classes (Knight, Robber) receive no spells; the returned
/// `Vec<SpellId>` will be empty.
///
/// # Arguments
///
/// * `character` - The character to level up (will be modified)
/// * `class_db`  - Reference to the class database (for HP dice and spell school)
/// * `spell_db`  - Reference to the spell database (for spell-level lookup)
/// * `rng`       - Random number generator for HP rolls
///
/// # Returns
///
/// Returns `Ok((hp_gained, granted_spells))` on success, where `hp_gained` is
/// the HP rolled this level and `granted_spells` is the list of spell IDs added
/// to the character's spellbook. Returns a [`ProgressionError`] if the
/// character cannot level up.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::classes::ClassDatabase;
/// use antares::domain::magic::database::SpellDatabase;
/// use antares::domain::progression::{award_experience, level_up_and_grant_spells};
/// use rand::rng;
///
/// let mut cleric = Character::new(
///     "Theodora".to_string(),
///     "human".to_string(),
///     "cleric".to_string(),
///     Sex::Female,
///     Alignment::Good,
/// );
/// award_experience(&mut cleric, 10000).unwrap();
///
/// let class_db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
/// let spell_db = SpellDatabase::load_from_file("data/spells.ron").unwrap_or_default();
/// let mut rng = rng();
/// let (hp_gained, new_spells) = level_up_and_grant_spells(&mut cleric, &class_db, &spell_db, &mut rng).unwrap();
///
/// assert_eq!(cleric.level, 2);
/// assert!(hp_gained > 0);
/// // new_spells contains any spells that first became accessible at level 2
/// let _ = new_spells;
/// ```
pub fn level_up_and_grant_spells(
    character: &mut Character,
    class_db: &ClassDatabase,
    spell_db: &SpellDatabase,
    rng: &mut impl Rng,
) -> Result<(u16, Vec<SpellId>), ProgressionError> {
    // Perform standard level-up (HP roll, SP update)
    let hp_gained = level_up_from_db(character, class_db, rng)?;

    // Determine which spells first become accessible at the new level
    let new_level = character.level;
    let newly_accessible = crate::domain::magic::learning::grant_level_up_spells(
        character, new_level, spell_db, class_db,
    );

    // Auto-grant: teach every newly accessible spell; silently skip errors
    // (AlreadyKnown can arise if a scroll was used before training)
    let mut granted: Vec<SpellId> = Vec::new();
    for spell_id in newly_accessible {
        match crate::domain::magic::learning::learn_spell(character, spell_id, spell_db, class_db) {
            Ok(()) => {
                granted.push(spell_id);
            }
            Err(crate::domain::magic::learning::SpellLearnError::AlreadyKnown(_)) => {
                // Character already learned this via scroll; not an error
            }
            Err(e) => {
                // Log unexpected errors but do not abort the level-up
                tracing::warn!(
                    "level_up_and_grant_spells: could not grant spell {} to {}: {}",
                    spell_id,
                    character.name,
                    e
                );
            }
        }
    }

    Ok((hp_gained, granted))
}

/// Calculates the experience required for a given level
///
/// # Examples
///
/// ```
/// use antares::domain::progression::experience_for_level;
///
/// assert_eq!(experience_for_level(1), 0);
/// assert_eq!(experience_for_level(2), 1000);
/// assert!(experience_for_level(10) > experience_for_level(5));
/// ```
pub fn experience_for_level(level: u32) -> u64 {
    if level <= 1 {
        return 0;
    }

    let level_offset = (level - 1) as f64;
    (BASE_XP as f64 * level_offset.powf(XP_MULTIPLIER)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::magic::types::{Spell, SpellContext, SpellSchool, SpellTarget};
    use crate::sdk::database::SpellDatabase;
    use crate::test_helpers::factories::test_character_with_class;
    use rand::rng;

    fn create_test_character(class_id: &str) -> Character {
        test_character_with_class("Test", class_id)
    }

    fn make_class_db() -> crate::domain::classes::ClassDatabase {
        crate::domain::classes::ClassDatabase::load_from_file("data/classes.ron")
            .expect("data/classes.ron must exist")
    }

    fn make_spell_db_with_level1_cleric_and_sorcerer() -> SpellDatabase {
        let mut db = SpellDatabase::new();
        db.add_spell(Spell::new(
            0x0101,
            "Cure Wounds",
            SpellSchool::Cleric,
            1,
            2,
            0,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Heals",
            None,
            0,
            false,
        ))
        .unwrap();
        db.add_spell(Spell::new(
            0x0201,
            "Holy Bolt",
            SpellSchool::Cleric,
            2,
            4,
            0,
            SpellContext::CombatOnly,
            SpellTarget::SingleMonster,
            "Damages",
            None,
            0,
            false,
        ))
        .unwrap();
        db.add_spell(Spell::new(
            0x0501,
            "Magic Arrow",
            SpellSchool::Sorcerer,
            1,
            2,
            0,
            SpellContext::CombatOnly,
            SpellTarget::SingleMonster,
            "Damages",
            None,
            0,
            false,
        ))
        .unwrap();
        db
    }

    #[test]
    fn test_award_experience() {
        let mut character = create_test_character("knight");
        assert_eq!(character.experience, 0);

        award_experience(&mut character, 500).unwrap();
        assert_eq!(character.experience, 500);

        award_experience(&mut character, 300).unwrap();
        assert_eq!(character.experience, 800);
    }

    #[test]
    fn test_experience_for_level() {
        assert_eq!(experience_for_level(1), 0);
        assert_eq!(experience_for_level(2), 1000);

        // Should be exponential
        let level5 = experience_for_level(5);
        let level10 = experience_for_level(10);
        assert!(level10 > level5 * 2);
    }

    #[test]
    fn test_check_level_up() {
        let mut character = create_test_character("knight");
        assert_eq!(character.level, 1);
        assert!(!check_level_up(&character));

        // Add enough XP for level 2
        let required = experience_for_level(2);
        character.experience = required;
        assert!(check_level_up(&character));

        // Not enough for level 3
        character.level = 2;
        assert!(!check_level_up(&character));
    }

    #[test]
    fn test_level_up_increases_level() {
        let mut character = create_test_character("knight");
        let required = experience_for_level(2);
        character.experience = required;

        let mut rng = rng();
        let hp_gained = level_up(&mut character, &mut rng).unwrap();

        assert_eq!(character.level, 2);
        assert!((1..=10).contains(&hp_gained)); // Knight uses 1d10
    }

    #[test]
    fn test_level_up_increases_hp() {
        let mut character = create_test_character("knight");
        let initial_hp = character.hp.base;
        let required = experience_for_level(2);
        character.experience = required;

        let mut rng = rng();
        level_up(&mut character, &mut rng).unwrap();

        assert!(character.hp.base > initial_hp);
        assert_eq!(character.hp.current, character.hp.base);
    }

    #[test]
    fn test_level_up_not_enough_xp() {
        let mut character = create_test_character("knight");
        character.experience = 500; // Not enough for level 2

        let mut rng = rng();
        let result = level_up(&mut character, &mut rng);

        assert!(matches!(
            result,
            Err(ProgressionError::NotEnoughExperience { .. })
        ));
        assert_eq!(character.level, 1);
    }

    #[test]
    fn test_level_up_max_level() {
        let mut character = create_test_character("knight");
        character.level = MAX_LEVEL;
        character.experience = u64::MAX;

        let mut rng = rng();
        let result = level_up(&mut character, &mut rng);

        assert!(matches!(result, Err(ProgressionError::MaxLevelReached)));
    }

    #[test]
    fn test_hp_gain_by_class() {
        let mut rng = rng();

        // Test multiple rolls to ensure ranges are correct
        for _ in 0..20 {
            let knight_hp = roll_hp_gain("knight", &mut rng);
            assert!((1..=10).contains(&knight_hp));

            let sorcerer_hp = roll_hp_gain("sorcerer", &mut rng);
            assert!((1..=4).contains(&sorcerer_hp));

            let cleric_hp = roll_hp_gain("cleric", &mut rng);
            assert!((1..=6).contains(&cleric_hp));
        }
    }

    #[test]
    fn test_dead_character_cannot_gain_xp() {
        let mut character = create_test_character("knight");
        character
            .conditions
            .add(crate::domain::character::Condition::DEAD);

        let result = award_experience(&mut character, 1000);
        assert!(matches!(result, Err(ProgressionError::CharacterDead)));
        assert_eq!(character.experience, 0);
    }

    #[test]
    fn test_spellcaster_gains_sp_on_level() {
        let mut cleric = create_test_character("cleric");
        cleric.stats.personality.base = 15;
        cleric.experience = experience_for_level(2);

        let initial_sp = cleric.sp.base;
        let mut rng = rng();
        level_up(&mut cleric, &mut rng).unwrap();

        assert!(cleric.sp.base > initial_sp);
    }

    #[test]
    fn test_non_spellcaster_no_sp_gain() {
        let mut knight = create_test_character("knight");
        knight.experience = experience_for_level(2);

        let initial_sp = knight.sp.base;
        let mut rng = rng();
        level_up(&mut knight, &mut rng).unwrap();

        assert_eq!(knight.sp.base, initial_sp); // Should still be 0
    }

    #[test]
    fn test_roll_hp_gain_from_db() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut rng = rng();

        // Test multiple rolls for each class
        for _ in 0..10 {
            let knight_hp = roll_hp_gain_from_db("knight", &db, &mut rng).unwrap();
            assert!((1..=10).contains(&knight_hp), "Knight HP out of range");

            let sorcerer_hp = roll_hp_gain_from_db("sorcerer", &db, &mut rng).unwrap();
            assert!((1..=4).contains(&sorcerer_hp), "Sorcerer HP out of range");

            let cleric_hp = roll_hp_gain_from_db("cleric", &db, &mut rng).unwrap();
            assert!((1..=6).contains(&cleric_hp), "Cleric HP out of range");
        }
    }

    #[test]
    fn test_roll_hp_gain_from_db_invalid_class() {
        let db = ClassDatabase::new();
        let mut rng = rng();

        let result = roll_hp_gain_from_db("nonexistent", &db, &mut rng);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ProgressionError::ClassError(ClassError::ClassNotFound(_)))
        ));
    }

    #[test]
    fn test_level_up_from_db() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut character = create_test_character("knight");
        let required = experience_for_level(2);
        character.experience = required;

        let mut rng = rng();
        let hp_gained = level_up_from_db(&mut character, &db, &mut rng).unwrap();

        assert_eq!(character.level, 2);
        assert!((1..=10).contains(&hp_gained)); // Knight uses 1d10
    }

    #[test]
    fn test_level_up_from_db_increases_hp() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut character = create_test_character("knight");
        let initial_hp = character.hp.base;
        let required = experience_for_level(2);
        character.experience = required;

        let mut rng = rng();
        level_up_from_db(&mut character, &db, &mut rng).unwrap();

        assert!(character.hp.base > initial_hp);
        assert_eq!(character.hp.current, character.hp.base);
    }

    #[test]
    fn test_level_up_from_db_not_enough_xp() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut character = create_test_character("knight");
        character.experience = 500; // Not enough for level 2

        let mut rng = rng();
        let result = level_up_from_db(&mut character, &db, &mut rng);

        assert!(matches!(
            result,
            Err(ProgressionError::NotEnoughExperience { .. })
        ));
        assert_eq!(character.level, 1);
    }

    #[test]
    fn test_level_up_from_db_max_level() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut character = create_test_character("knight");
        character.level = MAX_LEVEL;
        character.experience = u64::MAX;

        let mut rng = rng();
        let result = level_up_from_db(&mut character, &db, &mut rng);

        assert!(matches!(result, Err(ProgressionError::MaxLevelReached)));
    }

    #[test]
    fn test_level_up_from_db_spellcaster_gains_sp() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut cleric = create_test_character("cleric");
        cleric.stats.personality.base = 15;
        cleric.experience = experience_for_level(2);

        let initial_sp = cleric.sp.base;
        let mut rng = rng();
        level_up_from_db(&mut cleric, &db, &mut rng).unwrap();

        assert!(cleric.sp.base > initial_sp);
    }

    #[test]
    fn test_level_up_from_db_non_spellcaster_no_sp() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut knight = create_test_character("knight");
        knight.experience = experience_for_level(2);

        let initial_sp = knight.sp.base;
        let mut rng = rng();
        level_up_from_db(&mut knight, &db, &mut rng).unwrap();

        assert_eq!(knight.sp.base, initial_sp); // Should still be 0
    }

    #[test]
    fn test_id_and_db_hp_rolls_same_range() {
        // Verify that both methods produce results in the same range
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut rng = rng();

        for _ in 0..20 {
            let id_hp = roll_hp_gain("knight", &mut rng);
            let db_hp = roll_hp_gain_from_db("knight", &db, &mut rng).unwrap();

            // Both should be in the 1-10 range for Knight
            assert!((1..=10).contains(&id_hp));
            assert!((1..=10).contains(&db_hp));
        }
    }

    // ===== level_up_and_grant_spells tests =====

    #[test]
    fn test_level_up_and_grant_spells_cleric_level_1_grants_level_1_spells() {
        let class_db = make_class_db();
        let spell_db = make_spell_db_with_level1_cleric_and_sorcerer();
        let mut cleric = create_test_character("cleric");
        cleric.experience = experience_for_level(2);

        let mut rng = rng();
        let (hp_gained, granted) =
            level_up_and_grant_spells(&mut cleric, &class_db, &spell_db, &mut rng).unwrap();

        assert_eq!(cleric.level, 2);
        assert!(hp_gained > 0);
        // Level 2 for a cleric does not unlock new spell levels (spell level 2 needs char level 3)
        assert!(granted.is_empty());
    }

    #[test]
    fn test_level_up_and_grant_spells_cleric_level_3_grants_spell_level_2() {
        let class_db = make_class_db();
        let spell_db = make_spell_db_with_level1_cleric_and_sorcerer();

        let mut cleric = create_test_character("cleric");
        // Bring character to level 2 first, then level 3
        cleric.level = 2;
        cleric.experience = experience_for_level(3);

        let mut rng = rng();
        let (hp_gained, granted) =
            level_up_and_grant_spells(&mut cleric, &class_db, &spell_db, &mut rng).unwrap();

        assert_eq!(cleric.level, 3);
        assert!(hp_gained > 0);
        // Spell level 2 spell (0x0201) becomes accessible at character level 3
        assert!(granted.contains(&0x0201));
        // Level 1 spells were already accessible at level 1 — not granted again
        assert!(!granted.contains(&0x0101));
        // Spell appears in spellbook
        assert!(cleric.spells.cleric_spells[1].contains(&0x0201));
    }

    #[test]
    fn test_level_up_and_grant_spells_sorcerer_level_1_grants_sorcerer_level_1_spells() {
        let class_db = make_class_db();
        let spell_db = make_spell_db_with_level1_cleric_and_sorcerer();

        let mut sorc = create_test_character("sorcerer");
        // Start at level 0 equivalent — manually set to level 1 position
        sorc.level = 0;
        sorc.experience = experience_for_level(1);

        let mut rng = rng();
        let (hp_gained, granted) =
            level_up_and_grant_spells(&mut sorc, &class_db, &spell_db, &mut rng).unwrap();

        assert_eq!(sorc.level, 1);
        assert!(hp_gained > 0);
        // Sorcerer level 1 spell becomes accessible at character level 1
        assert!(granted.contains(&0x0501));
        assert!(sorc.spells.sorcerer_spells[0].contains(&0x0501));
        // Cleric spells must not appear for sorcerer
        assert!(!granted.contains(&0x0101));
    }

    #[test]
    fn test_level_up_and_grant_spells_knight_never_grants_spells() {
        let class_db = make_class_db();
        let spell_db = make_spell_db_with_level1_cleric_and_sorcerer();

        let mut knight = create_test_character("knight");
        knight.experience = experience_for_level(2);

        let mut rng = rng();
        let (_hp_gained, granted) =
            level_up_and_grant_spells(&mut knight, &class_db, &spell_db, &mut rng).unwrap();

        assert!(granted.is_empty());
    }

    #[test]
    fn test_level_up_and_grant_spells_paladin_no_spells_at_level_2() {
        let class_db = make_class_db();
        let spell_db = make_spell_db_with_level1_cleric_and_sorcerer();

        let mut paladin = create_test_character("paladin");
        paladin.experience = experience_for_level(2);

        let mut rng = rng();
        let (_hp_gained, granted) =
            level_up_and_grant_spells(&mut paladin, &class_db, &spell_db, &mut rng).unwrap();

        assert_eq!(paladin.level, 2);
        // Paladin has no spell access at levels 1-2
        assert!(granted.is_empty());
    }

    #[test]
    fn test_level_up_and_grant_spells_paladin_gains_at_level_3() {
        let class_db = make_class_db();
        let spell_db = make_spell_db_with_level1_cleric_and_sorcerer();

        let mut paladin = create_test_character("paladin");
        paladin.level = 2;
        paladin.experience = experience_for_level(3);

        let mut rng = rng();
        let (_hp_gained, granted) =
            level_up_and_grant_spells(&mut paladin, &class_db, &spell_db, &mut rng).unwrap();

        assert_eq!(paladin.level, 3);
        // Level 1 cleric spell first becomes accessible to paladin at level 3
        assert!(granted.contains(&0x0101));
        assert!(paladin.spells.cleric_spells[0].contains(&0x0101));
    }

    #[test]
    fn test_level_up_and_grant_spells_already_known_not_duplicated() {
        let class_db = make_class_db();
        let spell_db = make_spell_db_with_level1_cleric_and_sorcerer();

        let mut cleric = create_test_character("cleric");
        // Pre-populate the spell as if learned via scroll before training
        cleric.spells.cleric_spells[1].push(0x0201);
        cleric.level = 2;
        cleric.experience = experience_for_level(3);

        let mut rng = rng();
        let (_hp_gained, granted) =
            level_up_and_grant_spells(&mut cleric, &class_db, &spell_db, &mut rng).unwrap();

        assert_eq!(cleric.level, 3);
        // AlreadyKnown is silently skipped — spell must not be duplicated
        assert!(!granted.contains(&0x0201));
        assert_eq!(
            cleric.spells.cleric_spells[1]
                .iter()
                .filter(|&&id| id == 0x0201)
                .count(),
            1
        );
    }

    #[test]
    fn test_level_up_and_grant_spells_returns_error_when_max_level() {
        let class_db = make_class_db();
        let spell_db = make_spell_db_with_level1_cleric_and_sorcerer();

        let mut cleric = create_test_character("cleric");
        cleric.level = MAX_LEVEL;
        cleric.experience = u64::MAX;

        let mut rng = rng();
        let result = level_up_and_grant_spells(&mut cleric, &class_db, &spell_db, &mut rng);
        assert!(matches!(result, Err(ProgressionError::MaxLevelReached)));
    }

    #[test]
    fn test_level_up_and_grant_spells_hp_gain_is_positive() {
        let class_db = make_class_db();
        let spell_db = make_spell_db_with_level1_cleric_and_sorcerer();

        let mut cleric = create_test_character("cleric");
        cleric.experience = experience_for_level(2);

        let hp_before = cleric.hp.base;
        let mut rng = rng();
        let (hp_gained, _) =
            level_up_and_grant_spells(&mut cleric, &class_db, &spell_db, &mut rng).unwrap();

        assert!(hp_gained > 0);
        assert_eq!(cleric.hp.base, hp_before + hp_gained);
    }
}
