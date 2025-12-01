// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Spell effects - Functions for applying spell effects including conditions
//!
//! This module provides helper functions for applying various spell effects,
//! particularly the new condition system.

use crate::domain::character::Character;
use crate::domain::combat::monster::Monster;
use crate::domain::conditions::{ActiveCondition, ConditionDefinition, ConditionEffect};
use crate::domain::magic::types::Spell;
use rand::Rng;

/// Helper to get the element of a condition from its effects
fn get_condition_element(def: &ConditionDefinition) -> Option<&str> {
    for effect in &def.effects {
        if let ConditionEffect::DamageOverTime { element, .. } = effect {
            return Some(element);
        }
    }
    None
}

/// Applies conditions from a spell to a character
///
/// # Arguments
/// * `spell` - The spell being cast
/// * `target` - The target character
/// * `condition_defs` - Available condition definitions to lookup
/// * `allow_saving_throw` - Whether to check for saving throws (e.g. true for offensive spells, false for buffs)
///
/// # Examples
/// ```
/// use antares::domain::magic::spell_effects::apply_spell_conditions_to_character;
/// # use antares::domain::magic::types::*;
/// # use antares::domain::character::*;
/// # use antares::domain::conditions::*;
///
/// let mut character = Character::new("Hero".to_string(), Race::Human, Class::Knight, Sex::Male, Alignment::Good);
/// // ... setup spell and conditions
/// // apply_spell_conditions_to_character(&spell, &mut character, &conditions, true);
/// ```
pub fn apply_spell_conditions_to_character(
    spell: &Spell,
    target: &mut Character,
    condition_defs: &[ConditionDefinition],
    allow_saving_throw: bool,
) {
    for condition_id in &spell.applied_conditions {
        if let Some(def) = condition_defs.iter().find(|d| &d.id == condition_id) {
            // Saving throw check
            if allow_saving_throw && spell.saving_throw {
                let element = get_condition_element(def);
                let resistance = match element {
                    Some("fire") => target.resistances.fire.current,
                    Some("cold") => target.resistances.cold.current,
                    Some("electricity") => target.resistances.electricity.current,
                    Some("acid") => target.resistances.acid.current,
                    Some("poison") => target.resistances.poison.current,
                    Some("fear") => target.resistances.fear.current,
                    Some("psychic") => target.resistances.psychic.current,
                    _ => target.resistances.magic.current,
                };

                // Roll d100 (1-100)
                // If roll <= resistance, the character resists the effect
                let roll = rand::rng().random_range(1..=100);
                if roll <= resistance {
                    continue; // Saved
                }
            }

            let active = ActiveCondition::new(condition_id.clone(), def.default_duration);
            target.add_condition(active);
        }
    }
}

/// Applies conditions from a spell to a monster
///
/// # Arguments
/// * `spell` - The spell being cast
/// * `target` - The target monster
/// * `condition_defs` - Available condition definitions to lookup
/// * `allow_saving_throw` - Whether to check for saving throws
pub fn apply_spell_conditions_to_monster(
    spell: &Spell,
    target: &mut Monster,
    condition_defs: &[ConditionDefinition],
    allow_saving_throw: bool,
) {
    for condition_id in &spell.applied_conditions {
        if let Some(def) = condition_defs.iter().find(|d| &d.id == condition_id) {
            // Saving throw check
            if allow_saving_throw && spell.saving_throw {
                let element = get_condition_element(def);

                // Check immunities first
                let immune = match element {
                    Some("fire") => target.resistances.fire,
                    Some("cold") => target.resistances.cold,
                    Some("electricity") => target.resistances.electricity,
                    Some("fear") => target.resistances.fear,
                    Some("paralysis") => target.resistances.paralysis,
                    Some("sleep") => target.resistances.sleep,
                    Some("physical") => target.resistances.physical,
                    Some("energy") => target.resistances.energy,
                    _ => false,
                };

                if immune {
                    continue;
                }

                // Check magic resistance
                // Monster magic resistance is a percentage chance to resist
                let roll = rand::rng().random_range(1..=100);
                if roll <= target.magic_resistance {
                    continue;
                }
            }

            let active = ActiveCondition::new(condition_id.clone(), def.default_duration);
            target.add_condition(active);
        }
    }
}

/// Applies damage/healing over time effects for all active conditions
///
/// Returns the total damage dealt (negative for healing)
pub fn apply_condition_dot_effects(
    active_conditions: &[ActiveCondition],
    condition_defs: &[ConditionDefinition],
) -> i16 {
    let mut total_damage = 0i16;
    let mut rng = rand::rng();

    for active in active_conditions {
        if let Some(def) = condition_defs.iter().find(|d| d.id == active.condition_id) {
            for effect in &def.effects {
                match effect {
                    crate::domain::conditions::ConditionEffect::DamageOverTime {
                        damage, ..
                    } => {
                        let roll_result = damage.roll(&mut rng) as i16;
                        let scaled = (roll_result as f32 * active.magnitude).round() as i16;
                        total_damage = total_damage.saturating_add(scaled);
                    }
                    crate::domain::conditions::ConditionEffect::HealOverTime { amount } => {
                        let roll_result = amount.roll(&mut rng) as i16;
                        let scaled = (roll_result as f32 * active.magnitude).round() as i16;
                        total_damage = total_damage.saturating_sub(scaled); // Negative damage = healing
                    }
                    _ => {}
                }
            }
        }
    }

    total_damage
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, Character, Class, Race, Sex};
    use crate::domain::conditions::{ConditionDefinition, ConditionDuration};
    use crate::domain::magic::types::{Spell, SpellContext, SpellSchool, SpellTarget};

    #[test]
    fn test_apply_spell_conditions() {
        let mut character = Character::new(
            "TestHero".to_string(),
            Race::Human,
            Class::Knight,
            Sex::Male,
            Alignment::Good,
        );

        let condition_def = ConditionDefinition {
            id: "test_condition".to_string(),
            name: "Test Condition".to_string(),
            description: "A test condition".to_string(),
            effects: vec![],
            default_duration: ConditionDuration::Rounds(3),
            icon_id: None,
        };

        let mut spell = Spell::new(
            1,
            "Test Spell",
            SpellSchool::Cleric,
            1,
            5,
            0,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Test",
            None,
            0,
            false,
        );
        spell.applied_conditions = vec!["test_condition".to_string()];

        apply_spell_conditions_to_character(&spell, &mut character, &[condition_def], false);

        assert_eq!(character.active_conditions.len(), 1);
        assert_eq!(
            character.active_conditions[0].condition_id,
            "test_condition"
        );
    }

    #[test]
    fn test_saving_throw_resists_condition() {
        let mut character = Character::new(
            "TestHero".to_string(),
            Race::Human,
            Class::Knight,
            Sex::Male,
            Alignment::Good,
        );
        // Set 100% magic resistance
        character.resistances.magic.current = 100;

        let condition_def = ConditionDefinition {
            id: "curse".to_string(),
            name: "Curse".to_string(),
            description: "A curse".to_string(),
            effects: vec![],
            default_duration: ConditionDuration::Permanent,
            icon_id: None,
        };

        let mut spell = Spell::new(
            1,
            "Curse",
            SpellSchool::Cleric,
            1,
            5,
            0,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Curses target",
            None,
            0,
            true, // Saving throw allowed
        );
        spell.applied_conditions = vec!["curse".to_string()];

        // Should be resisted
        apply_spell_conditions_to_character(&spell, &mut character, &[condition_def], true);

        assert_eq!(character.active_conditions.len(), 0);
    }
}
