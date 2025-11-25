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

use crate::domain::character::{Character, Class};
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
/// use antares::domain::character::{Character, Class, Race, Sex, Alignment};
/// use antares::domain::magic::types::{Spell, SpellSchool, SpellContext, SpellTarget};
/// use antares::domain::magic::casting::can_cast_spell;
/// use antares::domain::types::GameMode;
///
/// let mut cleric = Character::new(
///     "Healer".to_string(),
///     Race::Human,
///     Class::Cleric,
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
    if !can_class_cast_school(character.class, spell.school) {
        return Err(SpellError::WrongClass(
            format!("{:?}", character.class),
            spell.school,
        ));
    }

    // Check level requirements (including delayed access for Paladins/Archers)
    let required_level = get_required_level_for_spell(character.class, spell);
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
/// use antares::domain::character::{Character, Class, Race, Sex, Alignment};
/// use antares::domain::magic::types::{Spell, SpellSchool, SpellContext, SpellTarget};
/// use antares::domain::magic::casting::cast_spell;
///
/// let mut sorcerer = Character::new(
///     "Mage".to_string(),
///     Race::Elf,
///     Class::Sorcerer,
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
}

// ===== Helper Functions =====

/// Checks if a character class can cast spells from a given school
///
/// # Examples
///
/// ```
/// use antares::domain::character::Class;
/// use antares::domain::magic::types::SpellSchool;
/// use antares::domain::magic::casting::can_class_cast_school;
///
/// assert!(can_class_cast_school(Class::Cleric, SpellSchool::Cleric));
/// assert!(can_class_cast_school(Class::Sorcerer, SpellSchool::Sorcerer));
/// assert!(!can_class_cast_school(Class::Knight, SpellSchool::Cleric));
/// ```
pub fn can_class_cast_school(class: Class, school: SpellSchool) -> bool {
    matches!(
        (class, school),
        (Class::Cleric, SpellSchool::Cleric)
            | (Class::Paladin, SpellSchool::Cleric)
            | (Class::Sorcerer, SpellSchool::Sorcerer)
            | (Class::Archer, SpellSchool::Sorcerer)
    )
}

/// Gets the required character level to cast a spell
///
/// Accounts for delayed spell access for Paladins and Archers.
///
/// # Examples
///
/// ```
/// use antares::domain::character::Class;
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
/// assert_eq!(get_required_level_for_spell(Class::Cleric, &spell), 1);
///
/// // Paladins need level 3 minimum for any spell
/// assert_eq!(get_required_level_for_spell(Class::Paladin, &spell), 3);
/// ```
pub fn get_required_level_for_spell(class: Class, spell: &Spell) -> u32 {
    let base_required = spell.required_level();

    match class {
        // Pure casters have immediate access
        Class::Cleric | Class::Sorcerer => base_required,

        // Hybrid classes need higher levels for spell access
        Class::Paladin | Class::Archer => {
            // Delayed spell access: need at least level 3
            base_required.max(3)
        }

        // Non-casters cannot cast
        _ => u32::MAX,
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
/// use antares::domain::character::{Character, Class, Race, Sex, Alignment};
/// use antares::domain::magic::casting::calculate_spell_points;
///
/// let mut cleric = Character::new(
///     "Healer".to_string(),
///     Race::Human,
///     Class::Cleric,
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

    match character.class {
        Class::Cleric | Class::Paladin => {
            // SP based on Personality
            calculate_sp_from_stat(character.stats.personality.base, level)
        }
        Class::Sorcerer | Class::Archer => {
            // SP based on Intellect
            calculate_sp_from_stat(character.stats.intellect.base, level)
        }
        _ => 0, // Non-spellcasting classes
    }
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
    use crate::domain::character::{Alignment, Race, Sex};
    use crate::domain::magic::types::{SpellContext, SpellTarget};

    fn create_test_character(class: Class, level: u32, sp: u16, gems: u32) -> Character {
        let mut character = Character::new(
            "Test".to_string(),
            Race::Human,
            class,
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
        let cleric = create_test_character(Class::Cleric, 5, 10, 5);
        let spell = create_test_spell(SpellSchool::Cleric, 1, 2, 0, SpellContext::Anytime);

        let result = can_cast_spell(&cleric, &spell, &GameMode::Exploration, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sorcerer_cannot_cast_cleric_spell() {
        let sorcerer = create_test_character(Class::Sorcerer, 5, 10, 5);
        let spell = create_test_spell(SpellSchool::Cleric, 1, 2, 0, SpellContext::Anytime);

        let result = can_cast_spell(&sorcerer, &spell, &GameMode::Exploration, false, false);
        assert!(matches!(result, Err(SpellError::WrongClass(_, _))));
    }

    #[test]
    fn test_cannot_cast_without_sp() {
        let mut cleric = create_test_character(Class::Cleric, 5, 1, 5);
        cleric.sp.current = 1; // Not enough for 5 SP spell
        let spell = create_test_spell(SpellSchool::Cleric, 3, 5, 0, SpellContext::Anytime);

        let result = can_cast_spell(&cleric, &spell, &GameMode::Exploration, false, false);
        assert!(matches!(result, Err(SpellError::NotEnoughSP { .. })));
    }

    #[test]
    fn test_cannot_cast_without_gems() {
        let mut cleric = create_test_character(Class::Cleric, 10, 10, 1);
        cleric.gems = 1; // Not enough for 5 gem spell
        let spell = create_test_spell(SpellSchool::Cleric, 5, 10, 5, SpellContext::Anytime);

        let result = can_cast_spell(&cleric, &spell, &GameMode::Exploration, false, false);
        assert!(matches!(result, Err(SpellError::NotEnoughGems { .. })));
    }

    #[test]
    fn test_combat_only_spell_in_exploration() {
        let sorcerer = create_test_character(Class::Sorcerer, 5, 10, 5);
        let spell = create_test_spell(SpellSchool::Sorcerer, 1, 3, 0, SpellContext::CombatOnly);

        let result = can_cast_spell(&sorcerer, &spell, &GameMode::Exploration, false, false);
        assert!(matches!(result, Err(SpellError::CombatOnly)));
    }

    #[test]
    fn test_combat_only_spell_in_combat() {
        let sorcerer = create_test_character(Class::Sorcerer, 5, 10, 5);
        let spell = create_test_spell(SpellSchool::Sorcerer, 1, 3, 0, SpellContext::CombatOnly);

        let result = can_cast_spell(&sorcerer, &spell, &GameMode::Combat, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_level_too_low() {
        let mut cleric = create_test_character(Class::Cleric, 1, 10, 5);
        cleric.level = 1; // Level 1 cannot cast level 5 spells (need level 9)
        let spell = create_test_spell(SpellSchool::Cleric, 5, 10, 2, SpellContext::Anytime);

        let result = can_cast_spell(&cleric, &spell, &GameMode::Exploration, false, false);
        assert!(matches!(result, Err(SpellError::LevelTooLow { .. })));
    }

    #[test]
    fn test_paladin_delayed_spell_access() {
        let mut paladin = create_test_character(Class::Paladin, 2, 10, 5);
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
        let mut cleric = create_test_character(Class::Cleric, 5, 10, 5);
        let spell = create_test_spell(SpellSchool::Cleric, 3, 4, 2, SpellContext::Anytime);

        let result = cast_spell(&mut cleric, &spell);

        assert!(result.success);
        assert_eq!(cleric.sp.current, 6); // 10 - 4
        assert_eq!(cleric.gems, 3); // 5 - 2
    }

    #[test]
    fn test_spell_point_calculation() {
        let mut cleric = create_test_character(Class::Cleric, 5, 0, 0);
        cleric.stats.personality.base = 15;

        let sp = calculate_spell_points(&cleric);
        // (15 - 10) * 5 / 2 + (5 * 2) = 12 + 10 = 22
        assert_eq!(sp, 22);
    }

    #[test]
    fn test_spell_point_calculation_low_stat() {
        let mut sorcerer = create_test_character(Class::Sorcerer, 3, 0, 0);
        sorcerer.stats.intellect.base = 8; // Below 10

        let sp = calculate_spell_points(&sorcerer);
        // (8 - 10) clamped to 0, so just base: 3 * 2 = 6
        assert_eq!(sp, 6);
    }

    #[test]
    fn test_non_caster_has_zero_sp() {
        let knight = create_test_character(Class::Knight, 10, 0, 0);
        let sp = calculate_spell_points(&knight);
        assert_eq!(sp, 0);
    }

    #[test]
    fn test_can_class_cast_school() {
        assert!(can_class_cast_school(Class::Cleric, SpellSchool::Cleric));
        assert!(can_class_cast_school(Class::Paladin, SpellSchool::Cleric));
        assert!(can_class_cast_school(
            Class::Sorcerer,
            SpellSchool::Sorcerer
        ));
        assert!(can_class_cast_school(Class::Archer, SpellSchool::Sorcerer));

        assert!(!can_class_cast_school(Class::Knight, SpellSchool::Cleric));
        assert!(!can_class_cast_school(Class::Robber, SpellSchool::Sorcerer));
        assert!(!can_class_cast_school(Class::Cleric, SpellSchool::Sorcerer));
        assert!(!can_class_cast_school(Class::Sorcerer, SpellSchool::Cleric));
    }
}
