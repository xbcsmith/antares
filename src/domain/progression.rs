// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Character progression system - Experience, leveling, and stat growth
//!
//! This module implements character advancement mechanics including
//! experience point awards, level-up checks, and class-based HP increases.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 5.2 for complete specifications.

use crate::domain::character::{Character, Class};
use crate::domain::classes::{ClassDatabase, ClassError};
use crate::domain::types::DiceRoll;
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
/// use antares::domain::character::{Character, Class, Race, Sex, Alignment};
/// use antares::domain::progression::award_experience;
///
/// let mut knight = Character::new(
///     "Sir Lancelot".to_string(),
///     Race::Human,
///     Class::Knight,
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
/// use antares::domain::character::{Character, Class, Race, Sex, Alignment};
/// use antares::domain::progression::{award_experience, check_level_up};
///
/// let mut character = Character::new(
///     "Hero".to_string(),
///     Race::Human,
///     Class::Knight,
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
/// use antares::domain::character::{Character, Class, Race, Sex, Alignment};
/// use antares::domain::progression::{award_experience, level_up};
/// use rand::rng;
///
/// let mut knight = Character::new(
///     "Sir Lancelot".to_string(),
///     Race::Human,
///     Class::Knight,
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
    let hp_gain = roll_hp_gain(character.class, rng);
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

/// Rolls HP gain for a character based on their class
///
/// Each class has different HP dice:
/// - Knight: 1d10
/// - Paladin: 1d8
/// - Archer: 1d8
/// - Cleric: 1d6
/// - Sorcerer: 1d4
/// - Thief: 1d6
///
/// # Arguments
///
/// * `class` - The character's class
/// * `rng` - Random number generator
///
/// # Returns
///
/// Returns the HP gained (minimum 1).
///
/// # Examples
///
/// ```
/// use antares::domain::character::Class;
/// use antares::domain::progression::roll_hp_gain;
/// use rand::rng;
///
/// let mut rng = rng();
///
/// let knight_hp = roll_hp_gain(Class::Knight, &mut rng);
/// assert!(knight_hp >= 1 && knight_hp <= 10);
///
/// let sorcerer_hp = roll_hp_gain(Class::Sorcerer, &mut rng);
/// assert!(sorcerer_hp >= 1 && sorcerer_hp <= 4);
/// ```
pub fn roll_hp_gain(class: Class, rng: &mut impl Rng) -> u16 {
    let dice = match class {
        Class::Knight => DiceRoll::new(1, 10, 0),  // 1d10
        Class::Paladin => DiceRoll::new(1, 8, 0),  // 1d8
        Class::Archer => DiceRoll::new(1, 8, 0),   // 1d8
        Class::Cleric => DiceRoll::new(1, 6, 0),   // 1d6
        Class::Sorcerer => DiceRoll::new(1, 4, 0), // 1d4
        Class::Robber => DiceRoll::new(1, 6, 0),   // 1d6
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

/// Calculates the experience required for a given level
///
/// Uses an exponential curve: BASE_XP * (level - 1) ^ XP_MULTIPLIER
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
    use crate::domain::character::{Alignment, Race, Sex};
    use rand::rng;

    fn create_test_character(class: Class) -> Character {
        Character::new(
            "Test".to_string(),
            Race::Human,
            class,
            Sex::Male,
            Alignment::Good,
        )
    }

    #[test]
    fn test_award_experience() {
        let mut character = create_test_character(Class::Knight);
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
        let mut character = create_test_character(Class::Knight);
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
        let mut character = create_test_character(Class::Knight);
        let required = experience_for_level(2);
        character.experience = required;

        let mut rng = rng();
        let hp_gained = level_up(&mut character, &mut rng).unwrap();

        assert_eq!(character.level, 2);
        assert!((1..=10).contains(&hp_gained)); // Knight uses 1d10
    }

    #[test]
    fn test_level_up_increases_hp() {
        let mut character = create_test_character(Class::Knight);
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
        let mut character = create_test_character(Class::Knight);
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
        let mut character = create_test_character(Class::Knight);
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
            let knight_hp = roll_hp_gain(Class::Knight, &mut rng);
            assert!((1..=10).contains(&knight_hp));

            let sorcerer_hp = roll_hp_gain(Class::Sorcerer, &mut rng);
            assert!((1..=4).contains(&sorcerer_hp));

            let cleric_hp = roll_hp_gain(Class::Cleric, &mut rng);
            assert!((1..=6).contains(&cleric_hp));
        }
    }

    #[test]
    fn test_dead_character_cannot_gain_xp() {
        let mut character = create_test_character(Class::Knight);
        character
            .conditions
            .add(crate::domain::character::Condition::DEAD);

        let result = award_experience(&mut character, 1000);
        assert!(matches!(result, Err(ProgressionError::CharacterDead)));
        assert_eq!(character.experience, 0);
    }

    #[test]
    fn test_spellcaster_gains_sp_on_level() {
        let mut cleric = create_test_character(Class::Cleric);
        cleric.stats.personality.base = 15;
        cleric.experience = experience_for_level(2);

        let initial_sp = cleric.sp.base;
        let mut rng = rng();
        level_up(&mut cleric, &mut rng).unwrap();

        assert!(cleric.sp.base > initial_sp);
    }

    #[test]
    fn test_non_spellcaster_no_sp_gain() {
        let mut knight = create_test_character(Class::Knight);
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
}
