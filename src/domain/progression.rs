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
use crate::domain::levels::LevelDatabase;
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

/// Default base XP required to reach level 2.
///
/// This is the fallback value used when no [`crate::domain::campaign::CampaignConfig`]
/// is available (or when the campaign config retains its default).
/// Pass this constant to [`experience_for_level_class`] when you have no config.
pub const DEFAULT_BASE_XP: u64 = 1000;

/// Default XP curve exponent (steepness of the level-up curve).
///
/// This is the fallback value used when no [`crate::domain::campaign::CampaignConfig`]
/// is available. Pass this constant to [`experience_for_level_class`] when you
/// have no config.
pub const DEFAULT_XP_MULTIPLIER: f64 = 1.5;

// Keep private aliases so internal code remains concise.
const BASE_XP: u64 = DEFAULT_BASE_XP;
const XP_MULTIPLIER: f64 = DEFAULT_XP_MULTIPLIER;

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
    check_level_up_with_db(character, None)
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
    level_up_with_level_db(character, class_db, None, None, rng)
}

/// Levels up a character using a `ClassDatabase`, an optional `LevelDatabase`,
/// and an optional campaign-configured maximum level.
///
/// This is the canonical implementation that [`level_up_from_db`] delegates to.
/// Use it directly when you need to supply explicit per-class XP tables or
/// enforce a campaign-specific `max_party_level`.
///
/// # Arguments
///
/// * `character`  - The character to level up (will be modified)
/// * `class_db`   - Reference to the class database (for HP dice and spell school)
/// * `level_db`   - Optional level database for per-class XP thresholds;
///   `None` falls back to the formula in [`experience_for_level`]
/// * `max_level`  - Optional campaign maximum level override;
///   `None` defaults to [`MAX_LEVEL`] (200)
/// * `rng`        - Random number generator for HP rolls
///
/// # Returns
///
/// Returns `Ok(hp_gained)` on success. Returns [`ProgressionError::MaxLevelReached`]
/// when `character.level >= max_level` (or `MAX_LEVEL` when `max_level` is
/// `None`), and [`ProgressionError::NotEnoughExperience`] when XP is
/// insufficient.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::classes::ClassDatabase;
/// use antares::domain::progression::{award_experience, level_up_with_level_db};
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
///
/// // No level database, no max-level override — identical to level_up_from_db
/// let hp = level_up_with_level_db(&mut knight, &db, None, None, &mut rng).unwrap();
/// assert_eq!(knight.level, 2);
/// assert!(hp > 0);
/// ```
pub fn level_up_with_level_db(
    character: &mut Character,
    class_db: &ClassDatabase,
    level_db: Option<&LevelDatabase>,
    max_level: Option<u32>,
    rng: &mut impl Rng,
) -> Result<u16, ProgressionError> {
    let effective_max = max_level.unwrap_or(MAX_LEVEL);

    // Enforce campaign-configured (or global) maximum level
    if character.level >= effective_max {
        return Err(ProgressionError::MaxLevelReached);
    }

    // Use explicit table if available, otherwise fall back to formula
    let required = experience_for_level_class(
        character.level + 1,
        &character.class_id,
        level_db,
        BASE_XP,
        XP_MULTIPLIER,
    );
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
    level_up_and_grant_spells_with_level_db(character, class_db, spell_db, None, None, rng)
}

/// Full level-up pipeline with optional per-class XP tables and campaign max level.
///
/// Combines [`level_up_with_level_db`] with automatic spell-granting. After HP
/// and SP are updated, every spell that first becomes accessible at the new
/// level is added to the character's spellbook.
///
/// Non-caster classes (Knight, Robber) receive no spells; the returned
/// `Vec<SpellId>` will be empty.
///
/// # Arguments
///
/// * `character`  - The character to level up (will be modified)
/// * `class_db`   - Reference to the class database
/// * `spell_db`   - Reference to the spell database
/// * `level_db`   - Optional level database for per-class XP thresholds;
///   `None` falls back to [`experience_for_level`]
/// * `max_level`  - Optional campaign maximum level; `None` defaults to [`MAX_LEVEL`]
/// * `rng`        - Random number generator for HP rolls
///
/// # Returns
///
/// Returns `Ok((hp_gained, granted_spells))` on success.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::classes::ClassDatabase;
/// use antares::domain::magic::database::SpellDatabase;
/// use antares::domain::progression::{award_experience, level_up_and_grant_spells_with_level_db};
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
///
/// // No level database, no max-level override
/// let (hp, spells) =
///     level_up_and_grant_spells_with_level_db(&mut cleric, &class_db, &spell_db, None, None, &mut rng)
///         .unwrap();
///
/// assert_eq!(cleric.level, 2);
/// assert!(hp > 0);
/// let _ = spells;
/// ```
pub fn level_up_and_grant_spells_with_level_db(
    character: &mut Character,
    class_db: &ClassDatabase,
    spell_db: &SpellDatabase,
    level_db: Option<&LevelDatabase>,
    max_level: Option<u32>,
    rng: &mut impl Rng,
) -> Result<(u16, Vec<SpellId>), ProgressionError> {
    // Perform level-up (HP roll, SP update) using explicit table or formula
    let hp_gained = level_up_with_level_db(character, class_db, level_db, max_level, rng)?;

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
                    "level_up_and_grant_spells_with_level_db: could not grant spell {} to {}: {}",
                    spell_id,
                    character.name,
                    e
                );
            }
        }
    }

    Ok((hp_gained, granted))
}

/// Internal parametric XP formula: `base_xp * (level - 1) ^ xp_multiplier`.
///
/// This is the shared implementation used by both [`experience_for_level`]
/// (with the module-default constants) and [`experience_for_level_class`]
/// (with per-campaign or per-call values).
fn experience_for_level_parametric(level: u32, base_xp: u64, xp_multiplier: f64) -> u64 {
    if level <= 1 {
        return 0;
    }
    let level_offset = (level - 1) as f64;
    (base_xp as f64 * level_offset.powf(xp_multiplier)) as u64
}

/// Calculates the experience required for a given level using the default curve.
///
/// Uses the module-default constants [`DEFAULT_BASE_XP`] = 1000 and
/// [`DEFAULT_XP_MULTIPLIER`] = 1.5. For campaign-configured curves use
/// [`experience_for_level_with_config`] instead.
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
    experience_for_level_parametric(level, BASE_XP, XP_MULTIPLIER)
}

/// Calculates the XP required for a given level, consulting an optional
/// per-class [`LevelDatabase`] before falling back to the parametric formula.
///
/// This is the low-level entry point for XP threshold lookups. Callers that
/// have a [`crate::domain::campaign::CampaignConfig`] should prefer the
/// [`experience_for_level_with_config`] convenience wrapper.
///
/// # Arguments
///
/// * `level`         - The target character level (1-based)
/// * `class_id`      - The character's class identifier (e.g. `"knight"`)
/// * `db`            - Optional level database; `None` always uses the formula
/// * `base_xp`       - Base XP for the formula fallback; pass [`DEFAULT_BASE_XP`]
///   when no campaign config is available
/// * `xp_multiplier` - Curve exponent for the formula fallback; pass
///   [`DEFAULT_XP_MULTIPLIER`] when no campaign config is available
///
/// # Returns
///
/// - If `db` is `Some` **and** the class has an explicit table: returns the
///   table value (possibly extrapolated via cap behaviour).
/// - Otherwise: returns `experience_for_level_parametric(level, base_xp, xp_multiplier)`.
///
/// # Examples
///
/// ```
/// use antares::domain::progression::{
///     experience_for_level, experience_for_level_class,
///     DEFAULT_BASE_XP, DEFAULT_XP_MULTIPLIER,
/// };
///
/// // Without a database — identical to experience_for_level
/// assert_eq!(
///     experience_for_level_class(1, "knight", None, DEFAULT_BASE_XP, DEFAULT_XP_MULTIPLIER),
///     0
/// );
/// assert_eq!(
///     experience_for_level_class(2, "knight", None, DEFAULT_BASE_XP, DEFAULT_XP_MULTIPLIER),
///     experience_for_level(2)
/// );
///
/// // Unknown class with db — falls back to formula without panic
/// use antares::domain::levels::LevelDatabase;
/// let db = LevelDatabase::new();
/// assert_eq!(
///     experience_for_level_class(2, "unknown_class", Some(&db), DEFAULT_BASE_XP, DEFAULT_XP_MULTIPLIER),
///     experience_for_level(2),
/// );
/// ```
pub fn experience_for_level_class(
    level: u32,
    class_id: &str,
    db: Option<&LevelDatabase>,
    base_xp: u64,
    xp_multiplier: f64,
) -> u64 {
    if let Some(db) = db {
        if let Some(xp) = db.threshold_for_class(class_id, level) {
            return xp;
        }
    }
    experience_for_level_parametric(level, base_xp, xp_multiplier)
}

/// Convenience wrapper: calculates XP threshold using campaign-configured curve.
///
/// This is the preferred call site for any game system that has access to a
/// [`crate::domain::campaign::CampaignConfig`]. It reads `base_xp` and
/// `xp_multiplier` from the config and delegates to [`experience_for_level_class`].
///
/// # Arguments
///
/// * `level`    - The target character level (1-based)
/// * `class_id` - The character's class identifier (e.g. `"knight"`)
/// * `config`   - Campaign config supplying `base_xp` and `xp_multiplier`
/// * `level_db` - Optional per-class level database; `None` uses the formula
///
/// # Examples
///
/// ```
/// use antares::domain::campaign::CampaignConfig;
/// use antares::domain::progression::{experience_for_level, experience_for_level_with_config};
///
/// let config = CampaignConfig::default(); // base_xp=1000, xp_multiplier=1.5
///
/// // Level 1 is always 0
/// assert_eq!(experience_for_level_with_config(1, "knight", &config, None), 0);
/// // Level 2 with default config matches the standard formula
/// assert_eq!(
///     experience_for_level_with_config(2, "knight", &config, None),
///     experience_for_level(2),
/// );
/// ```
pub fn experience_for_level_with_config(
    level: u32,
    class_id: &str,
    config: &crate::domain::campaign::CampaignConfig,
    level_db: Option<&LevelDatabase>,
) -> u64 {
    experience_for_level_class(
        level,
        class_id,
        level_db,
        config.base_xp,
        config.xp_multiplier,
    )
}

/// Checks if a character has enough experience to level up, using an optional
/// per-class [`LevelDatabase`] for the XP threshold.
///
/// This is the canonical implementation. [`check_level_up`] is a thin wrapper
/// that calls this function with `db = None`.
///
/// # Arguments
///
/// * `character` - The character to check
/// * `db`        - Optional level database; `None` falls back to the formula
///
/// # Returns
///
/// Returns `true` if `character.experience >= threshold(character.level + 1)`
/// and `character.level < MAX_LEVEL` (or the campaign max level).
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::levels::LevelDatabase;
/// use antares::domain::progression::{award_experience, check_level_up_with_db};
///
/// let mut knight = Character::new(
///     "Sir Lancelot".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
///
/// // No db — uses formula threshold (1000 for level 2)
/// assert!(!check_level_up_with_db(&knight, None));
/// award_experience(&mut knight, 1000).unwrap();
/// assert!(check_level_up_with_db(&knight, None));
///
/// // With an explicit db that requires more XP for knights
/// let ron = r#"(entries: [(class_id: "knight", thresholds: [0, 1200, 3000])])"#;
/// let db = LevelDatabase::load_from_string(ron).unwrap();
/// // 1000 XP is not enough when the table requires 1200
/// assert!(!check_level_up_with_db(&knight, Some(&db)));
/// award_experience(&mut knight, 200).unwrap();
/// assert!(check_level_up_with_db(&knight, Some(&db)));
/// ```
pub fn check_level_up_with_db(character: &Character, db: Option<&LevelDatabase>) -> bool {
    if character.level >= MAX_LEVEL {
        return false;
    }

    let required = experience_for_level_class(
        character.level + 1,
        &character.class_id,
        db,
        BASE_XP,
        XP_MULTIPLIER,
    );
    character.experience >= required
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::levels::LevelDatabase;
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

    /// Builds a small LevelDatabase for testing: knight requires 1200 XP for
    /// level 2 (vs. formula value of 1000), sorcerer requires 800.
    fn make_level_db() -> LevelDatabase {
        let ron = r#"(
            entries: [
                (class_id: "knight",   thresholds: [0, 1200, 3000, 6000]),
                (class_id: "sorcerer", thresholds: [0,  800, 2000, 4000]),
            ],
        )"#;
        LevelDatabase::load_from_string(ron).expect("inline RON must be valid")
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

    // ===== experience_for_level_class tests =====

    #[test]
    fn test_experience_for_level_class_no_db_matches_formula() {
        // With no database, must produce identical results to experience_for_level
        for lvl in [1u32, 2, 3, 5, 10, 50, 200] {
            assert_eq!(
                experience_for_level_class(
                    lvl,
                    "knight",
                    None,
                    DEFAULT_BASE_XP,
                    DEFAULT_XP_MULTIPLIER
                ),
                experience_for_level(lvl),
                "Mismatch at level {lvl}"
            );
        }
    }

    #[test]
    fn test_experience_for_level_class_with_db_found_returns_table_value() {
        let db = make_level_db();

        // Knight level 2 in the table is 1200, formula gives 1000 — must differ
        let table_val = experience_for_level_class(
            2,
            "knight",
            Some(&db),
            DEFAULT_BASE_XP,
            DEFAULT_XP_MULTIPLIER,
        );
        let formula_val = experience_for_level(2);
        assert_eq!(table_val, 1200, "Should use table value for knight level 2");
        assert_ne!(
            table_val, formula_val,
            "Table and formula must differ for this fixture"
        );

        // Sorcerer level 2 in the table is 800, formula gives 1000
        assert_eq!(
            experience_for_level_class(
                2,
                "sorcerer",
                Some(&db),
                DEFAULT_BASE_XP,
                DEFAULT_XP_MULTIPLIER
            ),
            800
        );
    }

    #[test]
    fn test_experience_for_level_class_with_db_not_found_falls_back_to_formula() {
        let db = make_level_db(); // contains knight and sorcerer only

        // "cleric" is absent from the inline DB — must fall back to formula
        let result = experience_for_level_class(
            2,
            "cleric",
            Some(&db),
            DEFAULT_BASE_XP,
            DEFAULT_XP_MULTIPLIER,
        );
        assert_eq!(result, experience_for_level(2));
    }

    #[test]
    fn test_experience_for_level_class_level_1_always_zero() {
        let db = make_level_db();
        assert_eq!(
            experience_for_level_class(1, "knight", None, DEFAULT_BASE_XP, DEFAULT_XP_MULTIPLIER),
            0
        );
        assert_eq!(
            experience_for_level_class(
                1,
                "knight",
                Some(&db),
                DEFAULT_BASE_XP,
                DEFAULT_XP_MULTIPLIER
            ),
            0
        );
        assert_eq!(
            experience_for_level_class(
                1,
                "unknown",
                Some(&db),
                DEFAULT_BASE_XP,
                DEFAULT_XP_MULTIPLIER
            ),
            0
        );
    }

    #[test]
    fn test_experience_for_level_class_empty_db_falls_back_to_formula() {
        let db = LevelDatabase::new();
        for lvl in [2u32, 5, 10] {
            assert_eq!(
                experience_for_level_class(
                    lvl,
                    "knight",
                    Some(&db),
                    DEFAULT_BASE_XP,
                    DEFAULT_XP_MULTIPLIER,
                ),
                experience_for_level(lvl),
                "Empty DB must fall back to formula at level {lvl}"
            );
        }
    }

    // ===== check_level_up_with_db tests =====

    #[test]
    fn test_check_level_up_with_db_no_db_uses_formula() {
        let mut character = create_test_character("knight");

        // Not enough XP yet
        assert!(!check_level_up_with_db(&character, None));

        // Give exactly the formula threshold for level 2 (1000)
        character.experience = experience_for_level(2);
        assert!(check_level_up_with_db(&character, None));
    }

    #[test]
    fn test_check_level_up_with_db_uses_table_threshold() {
        let db = make_level_db(); // knight level 2 = 1200
        let mut knight = create_test_character("knight");

        // 1000 XP satisfies formula but NOT the table (table requires 1200)
        knight.experience = 1000;
        assert!(!check_level_up_with_db(&knight, Some(&db)));

        // 1200 XP satisfies the table
        knight.experience = 1200;
        assert!(check_level_up_with_db(&knight, Some(&db)));
    }

    #[test]
    fn test_check_level_up_with_db_unknown_class_falls_back_to_formula() {
        let db = make_level_db(); // no "cleric" entry
        let mut cleric = create_test_character("cleric");

        // Formula threshold for level 2 is 1000
        cleric.experience = experience_for_level(2);
        assert!(check_level_up_with_db(&cleric, Some(&db)));
    }

    #[test]
    fn test_check_level_up_with_db_max_level_returns_false() {
        let mut character = create_test_character("knight");
        character.level = MAX_LEVEL;
        character.experience = u64::MAX;

        // Should be false regardless of db
        assert!(!check_level_up_with_db(&character, None));
        let db = make_level_db();
        assert!(!check_level_up_with_db(&character, Some(&db)));
    }

    #[test]
    fn test_check_level_up_delegates_to_check_level_up_with_db() {
        // check_level_up must behave identically to check_level_up_with_db(c, None)
        let mut character = create_test_character("knight");
        assert_eq!(
            check_level_up(&character),
            check_level_up_with_db(&character, None)
        );

        character.experience = experience_for_level(2);
        assert_eq!(
            check_level_up(&character),
            check_level_up_with_db(&character, None)
        );
        assert!(check_level_up(&character));
    }

    // ===== level_up_with_level_db tests =====

    #[test]
    fn test_level_up_with_level_db_no_db_behaves_like_level_up_from_db() {
        let class_db = make_class_db();
        let mut character = create_test_character("knight");
        character.experience = experience_for_level(2);

        let mut rng = rng();
        let hp = level_up_with_level_db(&mut character, &class_db, None, None, &mut rng).unwrap();
        assert_eq!(character.level, 2);
        assert!((1..=10).contains(&hp));
    }

    #[test]
    fn test_level_up_with_level_db_uses_table_threshold() {
        let class_db = make_class_db();
        let db = make_level_db(); // knight level 2 = 1200
        let mut knight = create_test_character("knight");

        // 1000 XP is enough for formula but NOT for the explicit table
        knight.experience = 1000;
        let mut rng = rng();
        let result = level_up_with_level_db(&mut knight, &class_db, Some(&db), None, &mut rng);
        assert!(
            matches!(
                result,
                Err(ProgressionError::NotEnoughExperience { needed: 1200, .. })
            ),
            "Expected NotEnoughExperience(needed=1200), got {:?}",
            result
        );
        assert_eq!(knight.level, 1, "Level must not change on failure");

        // Now provide 1200 XP — should succeed
        knight.experience = 1200;
        let hp = level_up_with_level_db(&mut knight, &class_db, Some(&db), None, &mut rng).unwrap();
        assert_eq!(knight.level, 2);
        assert!(hp > 0);
    }

    #[test]
    fn test_level_up_with_level_db_enforces_max_party_level() {
        let class_db = make_class_db();
        let mut character = create_test_character("knight");

        // Campaign max level of 10 — character is already at 10
        character.level = 10;
        character.experience = u64::MAX;

        let mut rng = rng();
        let result = level_up_with_level_db(&mut character, &class_db, None, Some(10), &mut rng);
        assert!(
            matches!(result, Err(ProgressionError::MaxLevelReached)),
            "Expected MaxLevelReached when level == max_level, got {:?}",
            result
        );
    }

    #[test]
    fn test_level_up_with_level_db_max_level_none_uses_global_max() {
        let class_db = make_class_db();
        let mut character = create_test_character("knight");
        character.level = MAX_LEVEL;
        character.experience = u64::MAX;

        let mut rng = rng();
        let result = level_up_with_level_db(&mut character, &class_db, None, None, &mut rng);
        assert!(matches!(result, Err(ProgressionError::MaxLevelReached)));
    }

    #[test]
    fn test_level_up_with_level_db_campaign_max_lower_than_global() {
        let class_db = make_class_db();
        let mut character = create_test_character("knight");
        // Campaign allows up to level 5; character is at level 4
        character.level = 4;
        character.experience = u64::MAX;

        let mut rng = rng();
        // Level 5 (max=5) — character is at 4, not yet at cap — should succeed
        let result = level_up_with_level_db(&mut character, &class_db, None, Some(5), &mut rng);
        assert!(result.is_ok(), "Should be able to reach level 5");
        assert_eq!(character.level, 5);

        // Now at 5 — at cap — should fail
        character.experience = u64::MAX;
        let result2 = level_up_with_level_db(&mut character, &class_db, None, Some(5), &mut rng);
        assert!(matches!(result2, Err(ProgressionError::MaxLevelReached)));
    }

    #[test]
    fn test_level_up_from_db_still_works_as_wrapper() {
        // Ensure the existing thin wrapper produces the same result as before
        let class_db = make_class_db();
        let mut character = create_test_character("knight");
        character.experience = experience_for_level(2);

        let mut rng = rng();
        let hp = level_up_from_db(&mut character, &class_db, &mut rng).unwrap();
        assert_eq!(character.level, 2);
        assert!((1..=10).contains(&hp));
    }

    // ===== level_up_and_grant_spells_with_level_db tests =====

    #[test]
    fn test_level_up_and_grant_spells_with_level_db_no_db_matches_existing() {
        let class_db = make_class_db();
        let spell_db = make_spell_db_with_level1_cleric_and_sorcerer();
        let mut cleric = create_test_character("cleric");
        cleric.experience = experience_for_level(2);

        let mut rng = rng();
        let (hp, spells) = level_up_and_grant_spells_with_level_db(
            &mut cleric,
            &class_db,
            &spell_db,
            None,
            None,
            &mut rng,
        )
        .unwrap();
        assert_eq!(cleric.level, 2);
        assert!(hp > 0);
        // Level 2 grants no new cleric spells (spell level 2 opens at char level 3)
        assert!(spells.is_empty());
    }

    #[test]
    fn test_level_up_and_grant_spells_with_level_db_max_level_enforced() {
        let class_db = make_class_db();
        let spell_db = make_spell_db_with_level1_cleric_and_sorcerer();
        let mut cleric = create_test_character("cleric");
        cleric.level = 5;
        cleric.experience = u64::MAX;

        let mut rng = rng();
        let result = level_up_and_grant_spells_with_level_db(
            &mut cleric,
            &class_db,
            &spell_db,
            None,
            Some(5),
            &mut rng,
        );
        assert!(matches!(result, Err(ProgressionError::MaxLevelReached)));
    }

    // ===== Fixture integration test =====

    #[test]
    fn test_experience_for_level_class_with_fixture_database() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path =
            std::path::PathBuf::from(manifest_dir).join("data/test_campaign/data/levels.ron");

        let db = LevelDatabase::load_from_file(&path).expect("levels.ron fixture must load");

        // Knight level 2 from fixture = 1200, formula = 1000 — must use fixture value
        let xp = experience_for_level_class(
            2,
            "knight",
            Some(&db),
            DEFAULT_BASE_XP,
            DEFAULT_XP_MULTIPLIER,
        );
        assert_eq!(xp, 1200, "Knight level 2 should use fixture value 1200");
        assert_ne!(xp, experience_for_level(2), "Should differ from formula");

        // Sorcerer level 2 from fixture = 800
        let sorcerer_xp = experience_for_level_class(
            2,
            "sorcerer",
            Some(&db),
            DEFAULT_BASE_XP,
            DEFAULT_XP_MULTIPLIER,
        );
        assert_eq!(sorcerer_xp, 800);

        // Unknown class falls back to formula
        let unknown_xp = experience_for_level_class(
            2,
            "unknown_class",
            Some(&db),
            DEFAULT_BASE_XP,
            DEFAULT_XP_MULTIPLIER,
        );
        assert_eq!(unknown_xp, experience_for_level(2));
    }

    #[test]
    fn test_check_level_up_with_db_fixture_database() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path =
            std::path::PathBuf::from(manifest_dir).join("data/test_campaign/data/levels.ron");
        let db = LevelDatabase::load_from_file(&path).expect("levels.ron fixture must load");

        let mut knight = create_test_character("knight");

        // 1000 XP is below the knight table threshold of 1200 → not ready
        knight.experience = 1000;
        assert!(!check_level_up_with_db(&knight, Some(&db)));

        // 1200 XP satisfies the knight table threshold → ready
        knight.experience = 1200;
        assert!(check_level_up_with_db(&knight, Some(&db)));
    }

    // ===== Phase 2: experience_for_level_with_config tests =====

    #[test]
    fn test_experience_for_level_with_config_default_matches_formula() {
        // Default CampaignConfig has base_xp=1000, xp_multiplier=1.5 — must match
        // the standard formula for every level.
        use crate::domain::campaign::CampaignConfig;
        let config = CampaignConfig::default();
        for lvl in [1u32, 2, 3, 5, 10, 50, 100] {
            assert_eq!(
                experience_for_level_with_config(lvl, "knight", &config, None),
                experience_for_level(lvl),
                "Default config must match formula at level {lvl}"
            );
        }
    }

    #[test]
    fn test_experience_for_level_with_config_custom_base_xp() {
        // base_xp=500, xp_multiplier=2.0 → level 5 = 500 * (4^2) = 8000
        use crate::domain::campaign::CampaignConfig;
        let config = CampaignConfig {
            base_xp: 500,
            xp_multiplier: 2.0,
            ..CampaignConfig::default()
        };

        let xp_level2 = experience_for_level_with_config(2, "knight", &config, None);
        // level 2: 500 * (1^2.0) = 500
        assert_eq!(
            xp_level2, 500,
            "level 2 with base_xp=500, mult=2.0 should be 500"
        );

        let xp_level5 = experience_for_level_with_config(5, "knight", &config, None);
        // level 5: 500 * (4^2.0) = 500 * 16 = 8000
        assert_eq!(
            xp_level5, 8000,
            "level 5 with base_xp=500, mult=2.0 should be 8000"
        );
    }

    #[test]
    fn test_experience_for_level_with_config_prefers_db_over_formula() {
        // When the level DB has an entry, it takes priority over the formula,
        // even when the campaign config provides custom base_xp/xp_multiplier.
        use crate::domain::campaign::CampaignConfig;
        let db = make_level_db(); // knight level 2 = 1200 in fixture
        let config = CampaignConfig {
            base_xp: 500,
            xp_multiplier: 2.0, // would give 500 for level 2
            ..CampaignConfig::default()
        };

        let xp = experience_for_level_with_config(2, "knight", &config, Some(&db));
        assert_eq!(xp, 1200, "DB entry must override the parametric formula");
    }

    #[test]
    fn test_experience_for_level_with_config_level_1_always_zero() {
        use crate::domain::campaign::CampaignConfig;
        let config = CampaignConfig {
            base_xp: 9999,
            xp_multiplier: 10.0,
            ..CampaignConfig::default()
        };

        assert_eq!(
            experience_for_level_with_config(1, "knight", &config, None),
            0,
            "Level 1 must always require 0 XP regardless of config"
        );
    }

    #[test]
    fn test_experience_for_level_parametric_matches_known_values() {
        // Verify the parametric function gives expected values at level boundaries.
        // base_xp=1000, xp_multiplier=1.5:
        //   level 1 → 0
        //   level 2 → 1000 * (1^1.5) = 1000
        //   level 3 → 1000 * (2^1.5) ≈ 2828
        assert_eq!(experience_for_level_parametric(1, 1000, 1.5), 0);
        assert_eq!(experience_for_level_parametric(2, 1000, 1.5), 1000);
        let lv3 = experience_for_level_parametric(3, 1000, 1.5);
        assert!(
            (2820..=2840).contains(&lv3),
            "level 3 expected ~2828, got {lv3}"
        );

        // With base_xp=500, xp_multiplier=2.0:
        //   level 5 → 500 * (4^2.0) = 8000
        assert_eq!(experience_for_level_parametric(5, 500, 2.0), 8000);
    }
}
