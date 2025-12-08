// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Spell casting logic - Validation and execution
//!
//! This module implements the core spell casting mechanics including
//! validation of casting requirements and execution of spell effects.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 5.3 for complete specifications.
//!
//! # Data-Driven Functions
//!
//! This module provides ID-based (data-driven) functions for spell casting:
//! - `can_class_cast_school` - Check if a class can cast from a spell school
//! - `get_required_level_for_spell` - Get required level for a spell
//! - `calculate_spell_points` - Calculate spell points for a character
//!
//! Database variants (`*_by_id`) use `ClassDatabase` lookups for full extensibility.

use crate::domain::character::Character;
use crate::domain::classes::{ClassDatabase, SpellSchool as ClassSpellSchool, SpellStat};
use crate::domain::magic::types::{Spell, SpellError, SpellResult, SpellSchool};
use crate::domain::types::GameMode;

// ===== Spell Casting Validation =====

/// Checks if a character can cast a specific spell
///
/// Validates all requirements for casting including:
/// - Class restrictions (Cleric vs Sorcerer spells)
/// - Character level requirements
/// - Spell point availability
/// - Gem availability
/// - Game mode context (combat vs exploration)
/// - Character conditions (silenced, unconscious)
///
/// # Arguments
///
/// * `character` - The character attempting to cast the spell
/// * `spell` - The spell to be cast
/// * `game_mode` - Current game mode (combat, exploration, etc.)
/// * `in_combat` - Whether the party is currently in combat
/// * `is_outdoor` - Whether the party is in an outdoor area
///
/// # Returns
///
/// Returns `Ok(())` if the character can cast the spell, otherwise returns
/// a `SpellError` describing why the spell cannot be cast.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::magic::types::{Spell, SpellSchool, SpellContext, SpellTarget};
/// use antares::domain::magic::casting::can_cast_spell;
/// use antares::domain::types::GameMode;
///
/// let mut cleric = Character::new(
///     "Healer".to_string(),
///     "human".to_string(),
///     "cleric".to_string(),
///     Sex::Female,
///     Alignment::Good,
/// );
/// cleric.level = 5;
/// cleric.sp.current = 10;
///
/// let cure_wounds = Spell::new(
///     0x0101,
///     "Cure Wounds",
///     SpellSchool::Cleric,
///     1,
///     2,
///     0,
///     SpellContext::Anytime,
///     SpellTarget::SingleCharacter,
///     "Heals 8 hit points",
///     None,
///     0,
///     false,
/// );
///
/// assert!(can_cast_spell(&cleric, &cure_wounds, &GameMode::Exploration, false, false).is_ok());
/// ```
pub fn can_cast_spell(
    character: &Character,
    spell: &Spell,
    _game_mode: &GameMode,
    in_combat: bool,
    is_outdoor: bool,
) -> Result<(), SpellError> {
    // Check if character is conscious
    if character.conditions.is_unconscious() {
        return Err(SpellError::Unconscious);
    }

    // Check if character is silenced
    if character.conditions.is_silenced() {
        return Err(SpellError::Silenced);
    }

    // Check class restrictions
    if !can_class_cast_school(&character.class_id, spell.school) {
        return Err(SpellError::WrongClass(
            character.class_id.clone(),
            spell.school,
        ));
    }

    // Check level requirements (including delayed access for Paladins/Archers)
    let required_level = get_required_level_for_spell(&character.class_id, spell);
    if character.level < required_level {
        return Err(SpellError::LevelTooLow {
            level: character.level,
            required: required_level,
        });
    }

    // Check spell points
    if character.sp.current < spell.sp_cost {
        return Err(SpellError::NotEnoughSP {
            needed: spell.sp_cost,
            available: character.sp.current,
        });
    }

    // Check gems
    if spell.gem_cost > 0 && (character.gems as u16) < spell.gem_cost {
        return Err(SpellError::NotEnoughGems {
            needed: spell.gem_cost as u32,
            available: character.gems,
        });
    }

    // Check context restrictions
    use crate::domain::magic::types::SpellContext;
    match spell.context {
        SpellContext::CombatOnly => {
            if !in_combat {
                return Err(SpellError::CombatOnly);
            }
        }
        SpellContext::NonCombatOnly => {
            if in_combat {
                return Err(SpellError::NonCombatOnly);
            }
        }
        SpellContext::OutdoorOnly => {
            if !is_outdoor {
                return Err(SpellError::OutdoorsOnly);
            }
        }
        SpellContext::IndoorOnly => {
            if is_outdoor {
                return Err(SpellError::IndoorsOnly);
            }
        }
        SpellContext::OutdoorCombat => {
            if !in_combat || !is_outdoor {
                return Err(SpellError::CombatOnly);
            }
        }
        SpellContext::Anytime => {
            // No restrictions
        }
    }

    Ok(())
}

/// Casts a spell and consumes resources
///
/// This function assumes `can_cast_spell` has already been called and passed.
/// It consumes spell points and gems, then returns a result indicating the
/// spell's effect.
///
/// **Note**: This function only handles resource consumption. Actual spell
/// effects (damage, healing, buffs) are applied by the combat or exploration
/// system based on the returned `SpellResult`.
///
/// # Arguments
///
/// * `character` - The character casting the spell (will be modified)
/// * `spell` - The spell being cast
///
/// # Returns
///
/// Returns a `SpellResult` containing information about the spell's effects.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::magic::types::{Spell, SpellSchool, SpellContext, SpellTarget};
/// use antares::domain::magic::casting::cast_spell;
///
/// let mut sorcerer = Character::new(
///     "Mage".to_string(),
///     "elf".to_string(),
///     "sorcerer".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// sorcerer.sp.current = 20;
/// sorcerer.gems = 10;
///
/// let fireball = Spell::new(
///     0x0201,
///     "Fireball",
///     SpellSchool::Sorcerer,
///     3,
///     5,
///     2,
///     SpellContext::CombatOnly,
///     SpellTarget::MonsterGroup,
///     "Deals 3d6 fire damage",
///     None,
///     0,
///     false,
/// );
///
/// let result = cast_spell(&mut sorcerer, &fireball);
/// assert!(result.success);
/// assert_eq!(sorcerer.sp.current, 15); // 20 - 5
/// assert_eq!(sorcerer.gems, 8); // 10 - 2
/// ```
pub fn cast_spell(character: &mut Character, spell: &Spell) -> SpellResult {
    // Consume spell points
    character.sp.current = character.sp.current.saturating_sub(spell.sp_cost);

    // Consume gems
    if spell.gem_cost > 0 {
        character.gems = character.gems.saturating_sub(spell.gem_cost as u32);
    }

    // Return a basic success result
    // Actual spell effects are handled by combat/exploration systems
    SpellResult::success(format!("{} casts {}!", character.name, spell.name))
        .with_conditions(spell.applied_conditions.clone())
}

// ===== Helper Functions =====

/// Checks if a character class can cast spells from a given school
///
/// # Arguments
///
/// * `class_id` - The class identifier string
/// * `school` - The spell school to check
///
/// # Returns
///
/// Returns `true` if the class can cast spells from the given school.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::types::SpellSchool;
/// use antares::domain::magic::casting::can_class_cast_school;
///
/// assert!(can_class_cast_school("cleric", SpellSchool::Cleric));
/// assert!(can_class_cast_school("sorcerer", SpellSchool::Sorcerer));
/// assert!(!can_class_cast_school("knight", SpellSchool::Cleric));
/// ```
pub fn can_class_cast_school(class_id: &str, school: SpellSchool) -> bool {
    matches!(
        (class_id, school),
        ("cleric", SpellSchool::Cleric)
            | ("paladin", SpellSchool::Cleric)
            | ("sorcerer", SpellSchool::Sorcerer)
            | ("archer", SpellSchool::Sorcerer)
    )
}

/// Checks if a class can cast spells from a given school using ClassDatabase
///
/// This is the data-driven version that looks up class definitions from the database.
/// Use this when working with campaign-specific or modded classes.
///
/// # Arguments
///
/// * `class_id` - The class ID to look up
/// * `class_db` - Reference to the class database
/// * `school` - The spell school to check
///
/// # Returns
///
/// Returns `true` if the class can cast spells from the given school.
/// Returns `false` if the class cannot cast from that school or if the class is not found.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::types::SpellSchool;
/// use antares::domain::magic::casting::can_class_cast_school_by_id;
/// use antares::domain::classes::ClassDatabase;
///
/// let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
///
/// assert!(can_class_cast_school_by_id("cleric", &db, SpellSchool::Cleric));
/// assert!(can_class_cast_school_by_id("sorcerer", &db, SpellSchool::Sorcerer));
/// assert!(!can_class_cast_school_by_id("knight", &db, SpellSchool::Cleric));
/// ```
pub fn can_class_cast_school_by_id(
    class_id: &str,
    class_db: &ClassDatabase,
    school: SpellSchool,
) -> bool {
    let Some(class_def) = class_db.get_class(class_id) else {
        return false;
    };

    let Some(class_school) = &class_def.spell_school else {
        return false;
    };

    // Convert ClassSpellSchool to magic SpellSchool for comparison
    matches!(
        (class_school, school),
        (ClassSpellSchool::Cleric, SpellSchool::Cleric)
            | (ClassSpellSchool::Sorcerer, SpellSchool::Sorcerer)
    )
}

/// Gets the required character level to cast a spell
///
/// Accounts for delayed spell access for Paladins and Archers.
///
/// # Arguments
///
/// * `class_id` - The class identifier string
/// * `spell` - The spell to check requirements for
///
/// # Returns
///
/// Returns the minimum character level required to cast the spell.
/// Returns `u32::MAX` if the class cannot cast spells.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::types::{Spell, SpellSchool, SpellContext, SpellTarget};
/// use antares::domain::magic::casting::get_required_level_for_spell;
///
/// let spell = Spell::new(
///     1,
///     "Test",
///     SpellSchool::Cleric,
///     1,
///     1,
///     0,
///     SpellContext::Anytime,
///     SpellTarget::Self_,
///     "Test spell",
///     None,
///     0,
///     false,
/// );
///
/// // Clerics can cast level 1 spells at character level 1
/// assert_eq!(get_required_level_for_spell("cleric", &spell), 1);
///
/// // Paladins need level 3 minimum for any spell
/// assert_eq!(get_required_level_for_spell("paladin", &spell), 3);
/// ```
pub fn get_required_level_for_spell(class_id: &str, spell: &Spell) -> u32 {
    let base_required = spell.required_level();

    match class_id {
        // Pure casters have immediate access
        "cleric" | "sorcerer" => base_required,

        // Hybrid classes need higher levels for spell access
        "paladin" | "archer" => {
            // Delayed spell access: need at least level 3
            base_required.max(3)
        }

        // Non-casters cannot cast
        _ => u32::MAX,
    }
}

/// Gets the required character level to cast a spell using ClassDatabase
///
/// This is the data-driven version that looks up class definitions from the database.
/// Accounts for delayed spell access for hybrid casters (non-pure casters).
///
/// # Arguments
///
/// * `class_id` - The class ID to look up
/// * `class_db` - Reference to the class database
/// * `spell` - The spell to check requirements for
///
/// # Returns
///
/// Returns the minimum character level required to cast the spell.
/// Returns `u32::MAX` if the class cannot cast spells or is not found.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::types::{Spell, SpellSchool, SpellContext, SpellTarget};
/// use antares::domain::magic::casting::get_required_level_for_spell_by_id;
/// use antares::domain::classes::ClassDatabase;
///
/// let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
///
/// let spell = Spell::new(
///     1,
///     "Test",
///     SpellSchool::Cleric,
///     1,
///     1,
///     0,
///     SpellContext::Anytime,
///     SpellTarget::Self_,
///     "Test spell",
///     None,
///     0,
///     false,
/// );
///
/// // Clerics (pure casters) can cast level 1 spells at character level 1
/// assert_eq!(get_required_level_for_spell_by_id("cleric", &db, &spell), 1);
///
/// // Paladins (hybrid casters) need level 3 minimum for any spell
/// assert_eq!(get_required_level_for_spell_by_id("paladin", &db, &spell), 3);
///
/// // Knights (non-casters) cannot cast
/// assert_eq!(get_required_level_for_spell_by_id("knight", &db, &spell), u32::MAX);
/// ```
pub fn get_required_level_for_spell_by_id(
    class_id: &str,
    class_db: &ClassDatabase,
    spell: &Spell,
) -> u32 {
    let Some(class_def) = class_db.get_class(class_id) else {
        return u32::MAX;
    };

    // Non-casters cannot cast
    if class_def.spell_school.is_none() {
        return u32::MAX;
    }

    let base_required = spell.required_level();

    // Pure casters have immediate access, hybrids need at least level 3
    if class_def.is_pure_caster {
        base_required
    } else {
        base_required.max(3)
    }
}

/// Calculates spell points for a character based on their class and stats
///
/// # Formula
///
/// - Cleric/Paladin: Based on Personality
/// - Sorcerer/Archer: Based on Intellect
/// - Formula: (stat - 10) * level / 2 + (level * 2)
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::magic::casting::calculate_spell_points;
///
/// let mut cleric = Character::new(
///     "Healer".to_string(),
///     "human".to_string(),
///     "cleric".to_string(),
///     Sex::Female,
///     Alignment::Good,
/// );
/// cleric.level = 5;
/// cleric.stats.personality.base = 15;
///
/// let sp = calculate_spell_points(&cleric);
/// // (15 - 10) * 5 / 2 + (5 * 2) = 12.5 + 10 = 22
/// assert_eq!(sp, 22);
/// ```
pub fn calculate_spell_points(character: &Character) -> u16 {
    let level = character.level as i16;

    match character.class_id.as_str() {
        "cleric" | "paladin" => {
            // SP based on Personality
            calculate_sp_from_stat(character.stats.personality.base, level)
        }
        "sorcerer" | "archer" => {
            // SP based on Intellect
            calculate_sp_from_stat(character.stats.intellect.base, level)
        }
        _ => 0, // Non-spellcasting classes
    }
}

/// Calculates spell points for a character using ClassDatabase
///
/// This is the data-driven version that looks up class definitions from the database.
/// Use this when working with campaign-specific or modded classes.
///
/// # Formula
///
/// - Uses the class's `spell_stat` to determine which attribute to use
/// - Personality for Cleric school, Intellect for Sorcerer school
/// - Formula: (stat - 10) * level / 2 + (level * 2)
///
/// # Arguments
///
/// * `character` - The character to calculate SP for
/// * `class_db` - Reference to the class database
///
/// # Returns
///
/// Returns the calculated spell points. Returns 0 if the character's class
/// is not found or cannot cast spells.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::magic::casting::calculate_spell_points_by_id;
/// use antares::domain::classes::ClassDatabase;
///
/// let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
///
/// let mut cleric = Character::new(
///     "Healer".to_string(),
///     "human".to_string(),
///     "cleric".to_string(),
///     Sex::Female,
///     Alignment::Good,
/// );
/// cleric.level = 5;
/// cleric.stats.personality.base = 15;
///
/// let sp = calculate_spell_points_by_id(&cleric, &db);
/// // (15 - 10) * 5 / 2 + (5 * 2) = 12 + 10 = 22
/// assert_eq!(sp, 22);
/// ```
pub fn calculate_spell_points_by_id(character: &Character, class_db: &ClassDatabase) -> u16 {
    let Some(class_def) = class_db.get_class(&character.class_id) else {
        return 0;
    };

    let Some(spell_stat) = &class_def.spell_stat else {
        return 0; // Non-spellcasting class
    };

    let level = character.level as i16;

    let stat_value = match spell_stat {
        SpellStat::Personality => character.stats.personality.base,
        SpellStat::Intellect => character.stats.intellect.base,
    };

    calculate_sp_from_stat(stat_value, level)
}

/// Helper to calculate SP from a stat value and character level
fn calculate_sp_from_stat(stat: u8, level: i16) -> u16 {
    // Base SP increases with both stat and level
    // Formula: (stat - 10) * level / 2 + base_per_level
    let stat_bonus = ((stat as i16 - 10).max(0) * level / 2) as u16;
    let base_sp = (level * 2) as u16; // 2 SP per level minimum
    base_sp + stat_bonus
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, Sex};
    use crate::domain::magic::types::{SpellContext, SpellTarget};

    fn create_test_character(class_id: &str, level: u32, sp: u16, gems: u32) -> Character {
        let mut character = Character::new(
            "Test".to_string(),
            "human".to_string(),
            class_id.to_string(),
            Sex::Male,
            Alignment::Good,
        );
        character.level = level;
        character.sp.current = sp;
        character.gems = gems;
        character
    }

    fn create_test_spell(
        school: SpellSchool,
        level: u8,
        sp_cost: u16,
        gem_cost: u16,
        context: SpellContext,
    ) -> Spell {
        Spell::new(
            1,
            "Test Spell",
            school,
            level,
            sp_cost,
            gem_cost,
            context,
            SpellTarget::Self_,
            "Test spell",
            None,
            0,
            false,
        )
    }

    #[test]
    fn test_cleric_can_cast_cleric_spell() {
        let cleric = create_test_character("cleric", 5, 10, 5);
        let spell = create_test_spell(SpellSchool::Cleric, 1, 2, 0, SpellContext::Anytime);

        let result = can_cast_spell(&cleric, &spell, &GameMode::Exploration, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sorcerer_cannot_cast_cleric_spell() {
        let sorcerer = create_test_character("sorcerer", 5, 10, 5);
        let spell = create_test_spell(SpellSchool::Cleric, 1, 2, 0, SpellContext::Anytime);

        let result = can_cast_spell(&sorcerer, &spell, &GameMode::Exploration, false, false);
        assert!(matches!(result, Err(SpellError::WrongClass(_, _))));
    }

    #[test]
    fn test_cannot_cast_without_sp() {
        let mut cleric = create_test_character("cleric", 5, 1, 5);
        cleric.sp.current = 1; // Not enough for 5 SP spell
        let spell = create_test_spell(SpellSchool::Cleric, 3, 5, 0, SpellContext::Anytime);

        let result = can_cast_spell(&cleric, &spell, &GameMode::Exploration, false, false);
        assert!(matches!(result, Err(SpellError::NotEnoughSP { .. })));
    }

    #[test]
    fn test_cannot_cast_without_gems() {
        let mut cleric = create_test_character("cleric", 10, 10, 1);
        cleric.gems = 1; // Not enough for 5 gem spell
        let spell = create_test_spell(SpellSchool::Cleric, 5, 10, 5, SpellContext::Anytime);

        let result = can_cast_spell(&cleric, &spell, &GameMode::Exploration, false, false);
        assert!(matches!(result, Err(SpellError::NotEnoughGems { .. })));
    }

    #[test]
    fn test_combat_only_spell_in_exploration() {
        let sorcerer = create_test_character("sorcerer", 5, 10, 5);
        let spell = create_test_spell(SpellSchool::Sorcerer, 1, 3, 0, SpellContext::CombatOnly);

        let result = can_cast_spell(&sorcerer, &spell, &GameMode::Exploration, false, false);
        assert!(matches!(result, Err(SpellError::CombatOnly)));
    }

    #[test]
    fn test_combat_only_spell_in_combat() {
        let sorcerer = create_test_character("sorcerer", 5, 10, 5);
        let spell = create_test_spell(SpellSchool::Sorcerer, 1, 3, 0, SpellContext::CombatOnly);

        let result = can_cast_spell(&sorcerer, &spell, &GameMode::Combat, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_level_too_low() {
        let mut cleric = create_test_character("cleric", 1, 10, 5);
        cleric.level = 1; // Level 1 cannot cast level 5 spells (need level 9)
        let spell = create_test_spell(SpellSchool::Cleric, 5, 10, 2, SpellContext::Anytime);

        let result = can_cast_spell(&cleric, &spell, &GameMode::Exploration, false, false);
        assert!(matches!(result, Err(SpellError::LevelTooLow { .. })));
    }

    #[test]
    fn test_paladin_delayed_spell_access() {
        let mut paladin = create_test_character("paladin", 2, 10, 5);
        paladin.level = 2; // Paladins need at least level 3
        let spell = create_test_spell(SpellSchool::Cleric, 1, 2, 0, SpellContext::Anytime);

        let result = can_cast_spell(&paladin, &spell, &GameMode::Exploration, false, false);
        assert!(matches!(result, Err(SpellError::LevelTooLow { .. })));

        // At level 3, should work
        paladin.level = 3;
        let result = can_cast_spell(&paladin, &spell, &GameMode::Exploration, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cast_spell_consumes_resources() {
        let mut cleric = create_test_character("cleric", 5, 10, 5);
        let spell = create_test_spell(SpellSchool::Cleric, 3, 4, 2, SpellContext::Anytime);

        let result = cast_spell(&mut cleric, &spell);

        assert!(result.success);
        assert_eq!(cleric.sp.current, 6); // 10 - 4
        assert_eq!(cleric.gems, 3); // 5 - 2
    }

    #[test]
    fn test_spell_point_calculation() {
        let mut cleric = create_test_character("cleric", 5, 0, 0);
        cleric.stats.personality.base = 15;

        let sp = calculate_spell_points(&cleric);
        // (15 - 10) * 5 / 2 + (5 * 2) = 12 + 10 = 22
        assert_eq!(sp, 22);
    }

    #[test]
    fn test_spell_point_calculation_low_stat() {
        let mut sorcerer = create_test_character("sorcerer", 3, 0, 0);
        sorcerer.stats.intellect.base = 8; // Below 10

        let sp = calculate_spell_points(&sorcerer);
        // (8 - 10) clamped to 0, so just base: 3 * 2 = 6
        assert_eq!(sp, 6);
    }

    #[test]
    fn test_non_caster_has_zero_sp() {
        let knight = create_test_character("knight", 10, 0, 0);
        let sp = calculate_spell_points(&knight);
        assert_eq!(sp, 0);
    }

    #[test]
    fn test_can_class_cast_school() {
        assert!(can_class_cast_school("cleric", SpellSchool::Cleric));
        assert!(can_class_cast_school("paladin", SpellSchool::Cleric));
        assert!(can_class_cast_school("sorcerer", SpellSchool::Sorcerer));
        assert!(can_class_cast_school("archer", SpellSchool::Sorcerer));

        assert!(!can_class_cast_school("knight", SpellSchool::Cleric));
        assert!(!can_class_cast_school("robber", SpellSchool::Sorcerer));
        assert!(!can_class_cast_school("cleric", SpellSchool::Sorcerer));
        assert!(!can_class_cast_school("sorcerer", SpellSchool::Cleric));
    }

    #[test]
    fn test_cast_spell_returns_conditions() {
        let mut cleric = create_test_character("cleric", 5, 10, 5);
        let mut spell = create_test_spell(SpellSchool::Cleric, 1, 2, 0, SpellContext::Anytime);
        spell.applied_conditions = vec!["bless".to_string()];

        let result = cast_spell(&mut cleric, &spell);

        assert!(result.success);
        assert_eq!(result.applied_conditions.len(), 1);
        assert_eq!(result.applied_conditions[0], "bless");
    }

    // ===== Data-Driven Function Tests =====

    #[test]
    fn test_can_class_cast_school_by_id_cleric() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();

        assert!(can_class_cast_school_by_id(
            "cleric",
            &db,
            SpellSchool::Cleric
        ));
        assert!(!can_class_cast_school_by_id(
            "cleric",
            &db,
            SpellSchool::Sorcerer
        ));
    }

    #[test]
    fn test_can_class_cast_school_by_id_sorcerer() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();

        assert!(can_class_cast_school_by_id(
            "sorcerer",
            &db,
            SpellSchool::Sorcerer
        ));
        assert!(!can_class_cast_school_by_id(
            "sorcerer",
            &db,
            SpellSchool::Cleric
        ));
    }

    #[test]
    fn test_can_class_cast_school_by_id_paladin() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();

        // Paladin is a hybrid with Cleric spells
        assert!(can_class_cast_school_by_id(
            "paladin",
            &db,
            SpellSchool::Cleric
        ));
        assert!(!can_class_cast_school_by_id(
            "paladin",
            &db,
            SpellSchool::Sorcerer
        ));
    }

    #[test]
    fn test_can_class_cast_school_by_id_non_caster() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();

        // Knight cannot cast any spells
        assert!(!can_class_cast_school_by_id(
            "knight",
            &db,
            SpellSchool::Cleric
        ));
        assert!(!can_class_cast_school_by_id(
            "knight",
            &db,
            SpellSchool::Sorcerer
        ));

        // Robber cannot cast any spells
        assert!(!can_class_cast_school_by_id(
            "robber",
            &db,
            SpellSchool::Cleric
        ));
        assert!(!can_class_cast_school_by_id(
            "robber",
            &db,
            SpellSchool::Sorcerer
        ));
    }

    #[test]
    fn test_can_class_cast_school_by_id_unknown_class() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();

        // Unknown class returns false
        assert!(!can_class_cast_school_by_id(
            "unknown",
            &db,
            SpellSchool::Cleric
        ));
        assert!(!can_class_cast_school_by_id(
            "nonexistent",
            &db,
            SpellSchool::Sorcerer
        ));
    }

    #[test]
    fn test_get_required_level_for_spell_by_id_pure_caster() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let spell = create_test_spell(SpellSchool::Cleric, 1, 2, 0, SpellContext::Anytime);

        // Cleric (pure caster) can cast level 1 spells at character level 1
        assert_eq!(get_required_level_for_spell_by_id("cleric", &db, &spell), 1);

        // Sorcerer (pure caster) can cast level 1 spells at character level 1
        let sorcerer_spell =
            create_test_spell(SpellSchool::Sorcerer, 1, 2, 0, SpellContext::Anytime);
        assert_eq!(
            get_required_level_for_spell_by_id("sorcerer", &db, &sorcerer_spell),
            1
        );
    }

    #[test]
    fn test_get_required_level_for_spell_by_id_hybrid_caster() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let spell = create_test_spell(SpellSchool::Cleric, 1, 2, 0, SpellContext::Anytime);

        // Paladin (hybrid) needs at least level 3 for any spell
        assert_eq!(
            get_required_level_for_spell_by_id("paladin", &db, &spell),
            3
        );
    }

    #[test]
    fn test_get_required_level_for_spell_by_id_non_caster() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let spell = create_test_spell(SpellSchool::Cleric, 1, 2, 0, SpellContext::Anytime);

        // Knight (non-caster) cannot cast
        assert_eq!(
            get_required_level_for_spell_by_id("knight", &db, &spell),
            u32::MAX
        );

        // Robber (non-caster) cannot cast
        assert_eq!(
            get_required_level_for_spell_by_id("robber", &db, &spell),
            u32::MAX
        );
    }

    #[test]
    fn test_get_required_level_for_spell_by_id_unknown_class() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let spell = create_test_spell(SpellSchool::Cleric, 1, 2, 0, SpellContext::Anytime);

        // Unknown class returns MAX
        assert_eq!(
            get_required_level_for_spell_by_id("unknown", &db, &spell),
            u32::MAX
        );
    }

    #[test]
    fn test_get_required_level_for_spell_by_id_higher_level_spells() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();

        // Level 3 spell requires character level 5 for pure casters
        let level3_spell = create_test_spell(SpellSchool::Cleric, 3, 5, 0, SpellContext::Anytime);
        assert_eq!(
            get_required_level_for_spell_by_id("cleric", &db, &level3_spell),
            5
        );

        // Level 3 spell requires max(5, 3) = 5 for hybrid casters
        assert_eq!(
            get_required_level_for_spell_by_id("paladin", &db, &level3_spell),
            5
        );
    }

    #[test]
    fn test_calculate_spell_points_by_id_cleric() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut cleric = create_test_character("cleric", 5, 0, 0);
        cleric.stats.personality.base = 15;

        let sp = calculate_spell_points_by_id(&cleric, &db);
        // (15 - 10) * 5 / 2 + (5 * 2) = 12 + 10 = 22
        assert_eq!(sp, 22);
    }

    #[test]
    fn test_calculate_spell_points_by_id_sorcerer() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut sorcerer = create_test_character("sorcerer", 5, 0, 0);
        sorcerer.stats.intellect.base = 15;

        let sp = calculate_spell_points_by_id(&sorcerer, &db);
        // (15 - 10) * 5 / 2 + (5 * 2) = 12 + 10 = 22
        assert_eq!(sp, 22);
    }

    #[test]
    fn test_calculate_spell_points_by_id_paladin() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut paladin = create_test_character("paladin", 5, 0, 0);
        paladin.stats.personality.base = 14;

        let sp = calculate_spell_points_by_id(&paladin, &db);
        // (14 - 10) * 5 / 2 + (5 * 2) = 10 + 10 = 20
        assert_eq!(sp, 20);
    }

    #[test]
    fn test_calculate_spell_points_by_id_non_caster() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let knight = create_test_character("knight", 10, 0, 0);

        let sp = calculate_spell_points_by_id(&knight, &db);
        assert_eq!(sp, 0);
    }

    #[test]
    fn test_calculate_spell_points_by_id_unknown_class() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut character = create_test_character("knight", 5, 0, 0);
        // Override class_id to an unknown value
        character.class_id = "unknown".to_string();

        let sp = calculate_spell_points_by_id(&character, &db);
        assert_eq!(sp, 0);
    }

    #[test]
    fn test_calculate_spell_points_by_id_low_stat() {
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let mut sorcerer = create_test_character("sorcerer", 3, 0, 0);
        sorcerer.stats.intellect.base = 8; // Below 10

        let sp = calculate_spell_points_by_id(&sorcerer, &db);
        // (8 - 10) clamped to 0, so just base: 3 * 2 = 6
        assert_eq!(sp, 6);
    }

    #[test]
    fn test_id_and_db_spell_points_match() {
        // Verify that both methods produce the same results
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();

        // Test Cleric
        let mut cleric = create_test_character("cleric", 5, 0, 0);
        cleric.stats.personality.base = 15;
        assert_eq!(
            calculate_spell_points(&cleric),
            calculate_spell_points_by_id(&cleric, &db)
        );

        // Test Sorcerer
        let mut sorcerer = create_test_character("sorcerer", 5, 0, 0);
        sorcerer.stats.intellect.base = 15;
        assert_eq!(
            calculate_spell_points(&sorcerer),
            calculate_spell_points_by_id(&sorcerer, &db)
        );

        // Test Paladin
        let mut paladin = create_test_character("paladin", 5, 0, 0);
        paladin.stats.personality.base = 14;
        assert_eq!(
            calculate_spell_points(&paladin),
            calculate_spell_points_by_id(&paladin, &db)
        );

        // Test Knight (non-caster)
        let knight = create_test_character("knight", 5, 0, 0);
        assert_eq!(
            calculate_spell_points(&knight),
            calculate_spell_points_by_id(&knight, &db)
        );
    }

    #[test]
    fn test_id_and_db_can_cast_school_match() {
        // Verify that both methods produce the same results for all classes
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();

        let class_ids = ["cleric", "sorcerer", "paladin", "knight", "robber"];

        for class_id in class_ids {
            assert_eq!(
                can_class_cast_school(class_id, SpellSchool::Cleric),
                can_class_cast_school_by_id(class_id, &db, SpellSchool::Cleric),
                "Cleric school mismatch for {}",
                class_id
            );
            assert_eq!(
                can_class_cast_school(class_id, SpellSchool::Sorcerer),
                can_class_cast_school_by_id(class_id, &db, SpellSchool::Sorcerer),
                "Sorcerer school mismatch for {}",
                class_id
            );
        }
    }

    #[test]
    fn test_id_and_db_required_level_match() {
        // Verify that both methods produce the same results
        let db = ClassDatabase::load_from_file("data/classes.ron").unwrap();
        let spell = create_test_spell(SpellSchool::Cleric, 1, 2, 0, SpellContext::Anytime);

        let class_ids = ["cleric", "paladin", "knight"];

        for class_id in class_ids {
            assert_eq!(
                get_required_level_for_spell(class_id, &spell),
                get_required_level_for_spell_by_id(class_id, &db, &spell),
                "Required level mismatch for {}",
                class_id
            );
        }
    }
}
