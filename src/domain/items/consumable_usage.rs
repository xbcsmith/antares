// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Pure-domain consumable effect application
//!
//! This module provides the single authoritative implementation for applying
//! [`ConsumableEffect`] variants to a [`Character`]. Both the combat path
//! ([`execute_item_use_by_slot`]) and the future exploration/menu path share
//! this implementation, ensuring there is no duplicated match logic.
//!
//! # Design Contract
//!
//! - **Self-mutation only.** `apply_consumable_effect` mutates only the
//!   `character` reference it receives. Callers are responsible for selecting
//!   the correct target.
//! - **Cap behaviour.** `HealHp` and `RestoreSp` clamp `current` to `base`
//!   after the modification so over-healing is impossible.
//! - **`CureCondition` scope.** Only `character.conditions` (the `Condition`
//!   bitflag newtype) is modified. `character.active_conditions` is never
//!   touched; that vec is managed by the data-driven condition tick system.
//! - **`BoostAttribute` / `BoostResistance`.** Modify `current` via
//!   `AttributePair::modify` / `AttributePair16::modify`; `base` is always
//!   preserved.
//! - **`IsFood`.** Food items are never applied via this path (the rest system
//!   handles them separately). Calling `apply_consumable_effect` with
//!   `ConsumableEffect::IsFood` is a no-op that returns a zeroed result.
//!
//! # Examples
//!
//! ```
//! use antares::domain::character::{Character, Sex, Alignment, Condition};
//! use antares::domain::items::types::{ConsumableEffect, AttributeType, ResistanceType};
//! use antares::domain::items::consumable_usage::apply_consumable_effect;
//!
//! let mut hero = Character::new(
//!     "Aria".to_string(),
//!     "human".to_string(),
//!     "cleric".to_string(),
//!     Sex::Female,
//!     Alignment::Good,
//! );
//! hero.hp.base = 30;
//! hero.hp.current = 10;
//!
//! let result = apply_consumable_effect(&mut hero, ConsumableEffect::HealHp(50));
//! assert_eq!(hero.hp.current, 30); // capped at base
//! assert_eq!(result.healing, 20);  // only the delta is reported
//! ```

use crate::domain::character::Character;
use crate::domain::items::types::{AttributeType, ConsumableEffect, ResistanceType};

/// Describes what `apply_consumable_effect` actually changed on the character.
///
/// All fields are zero when the corresponding effect did not apply (e.g.
/// `healing == 0` for a `BoostAttribute` potion). Callers can use this to
/// compose player-visible feedback messages without re-deriving the deltas.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::items::types::ConsumableEffect;
/// use antares::domain::items::consumable_usage::apply_consumable_effect;
///
/// let mut hero = Character::new(
///     "Brom".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// hero.sp.base = 20;
/// hero.sp.current = 5;
///
/// let result = apply_consumable_effect(&mut hero, ConsumableEffect::RestoreSp(100));
/// assert_eq!(result.sp_restored, 15); // only the delta up to base
/// assert_eq!(result.healing, 0);      // HP field untouched
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConsumableApplyResult {
    /// HP actually restored (0 if no heal occurred)
    pub healing: i32,
    /// SP actually restored (0 if no SP restore occurred)
    pub sp_restored: i32,
    /// Bitflags that were cleared from `character.conditions` (0 if none)
    pub conditions_cleared: u8,
    /// Stat change applied via `BoostAttribute` (0 if none)
    pub attribute_delta: i16,
    /// Resistance change applied via `BoostResistance` (0 if none)
    pub resistance_delta: i16,
}

/// Apply a single [`ConsumableEffect`] variant to `character` and return a
/// [`ConsumableApplyResult`] describing what changed.
///
/// This is the **single authoritative implementation** for all five consumable
/// effect variants. Both the combat executor and the exploration/menu path
/// delegate here to avoid duplicated match logic.
///
/// # Arguments
///
/// * `character` - The character receiving the effect (mutated in place).
/// * `effect`    - The consumable effect to apply.
///
/// # Returns
///
/// A [`ConsumableApplyResult`] with the actual deltas applied. All fields that
/// do not correspond to the active effect variant are zero.
///
/// # Cap Behaviour
///
/// - `HealHp`:   `hp.current` is clamped to `hp.base` after modification.
/// - `RestoreSp`: `sp.current` is clamped to `sp.base` after modification.
/// - All other variants use `AttributePair::modify` / `AttributePair16::modify`
///   which saturates at the type boundary (`u8::MAX` / `u16::MAX`).
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment, Condition};
/// use antares::domain::items::types::{ConsumableEffect, AttributeType, ResistanceType};
/// use antares::domain::items::consumable_usage::apply_consumable_effect;
///
/// // --- HealHp capped at base ---
/// let mut ch = Character::new(
///     "Hero".to_string(), "human".to_string(), "knight".to_string(),
///     Sex::Male, Alignment::Good,
/// );
/// ch.hp.base = 30;
/// ch.hp.current = 10;
/// let r = apply_consumable_effect(&mut ch, ConsumableEffect::HealHp(50));
/// assert_eq!(ch.hp.current, 30);
/// assert_eq!(r.healing, 20);
///
/// // --- CureCondition removes only the matching bits ---
/// ch.conditions.add(Condition::POISONED);
/// ch.conditions.add(Condition::BLINDED);
/// let r2 = apply_consumable_effect(&mut ch, ConsumableEffect::CureCondition(Condition::POISONED));
/// assert!(!ch.conditions.has(Condition::POISONED));
/// assert!(ch.conditions.has(Condition::BLINDED));
/// assert_eq!(r2.conditions_cleared, Condition::POISONED);
///
/// // --- BoostAttribute modifies current, not base ---
/// let r3 = apply_consumable_effect(&mut ch, ConsumableEffect::BoostAttribute(AttributeType::Might, 5));
/// assert_eq!(ch.stats.might.current, 15); // default base=10, current=10+5
/// assert_eq!(ch.stats.might.base, 10);
/// assert_eq!(r3.attribute_delta, 5);
/// ```
pub fn apply_consumable_effect(
    character: &mut Character,
    effect: ConsumableEffect,
) -> ConsumableApplyResult {
    let mut result = ConsumableApplyResult::default();

    match effect {
        ConsumableEffect::HealHp(amount) => {
            let pre = character.hp.current as i32;
            character.hp.modify(amount as i32);
            // Clamp to base — over-healing is not allowed
            if character.hp.current > character.hp.base {
                character.hp.current = character.hp.base;
            }
            let post = character.hp.current as i32;
            let healed = post - pre;
            if healed > 0 {
                result.healing = healed;
            }
        }

        ConsumableEffect::RestoreSp(amount) => {
            let pre = character.sp.current as i32;
            character.sp.modify(amount as i32);
            // Clamp to base — over-restoring is not allowed
            if character.sp.current > character.sp.base {
                character.sp.current = character.sp.base;
            }
            let post = character.sp.current as i32;
            let restored = post - pre;
            if restored > 0 {
                result.sp_restored = restored;
            }
        }

        ConsumableEffect::CureCondition(flags) => {
            // Only the bitflag Condition newtype is cleared.
            // active_conditions (data-driven) is intentionally untouched.
            character.conditions.remove(flags);
            result.conditions_cleared = flags;
        }

        ConsumableEffect::BoostAttribute(attr, amount) => {
            match attr {
                AttributeType::Might => character.stats.might.modify(amount as i16),
                AttributeType::Intellect => character.stats.intellect.modify(amount as i16),
                AttributeType::Personality => character.stats.personality.modify(amount as i16),
                AttributeType::Endurance => character.stats.endurance.modify(amount as i16),
                AttributeType::Speed => character.stats.speed.modify(amount as i16),
                AttributeType::Accuracy => character.stats.accuracy.modify(amount as i16),
                AttributeType::Luck => character.stats.luck.modify(amount as i16),
            }
            result.attribute_delta = amount as i16;
        }

        ConsumableEffect::BoostResistance(res_type, amount) => {
            match res_type {
                ResistanceType::Physical => {
                    // No dedicated physical field on character Resistances;
                    // use magic as the closest analogue (matches combat path).
                    character.resistances.magic.modify(amount as i16);
                }
                ResistanceType::Fire => {
                    character.resistances.fire.modify(amount as i16);
                }
                ResistanceType::Cold => {
                    character.resistances.cold.modify(amount as i16);
                }
                ResistanceType::Electricity => {
                    character.resistances.electricity.modify(amount as i16);
                }
                ResistanceType::Energy => {
                    // Energy maps to magic resistance (matches combat path).
                    character.resistances.magic.modify(amount as i16);
                }
                ResistanceType::Paralysis => {
                    // Paralysis maps to psychic resistance (matches combat path).
                    character.resistances.psychic.modify(amount as i16);
                }
                ResistanceType::Fear => {
                    character.resistances.fear.modify(amount as i16);
                }
                ResistanceType::Sleep => {
                    // Sleep maps to psychic resistance (matches combat path).
                    character.resistances.psychic.modify(amount as i16);
                }
            }
            result.resistance_delta = amount as i16;
        }

        ConsumableEffect::IsFood(_) => {
            // Food items are consumed by the rest system, not this path.
            // Silently return the zeroed result so callers can detect no-op.
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, Character, Condition, Sex};
    use crate::domain::items::types::{AttributeType, ConsumableEffect, ResistanceType};

    /// Build a basic character for tests with explicit hp/sp bases set.
    fn make_character(hp_base: u16, hp_current: u16, sp_base: u16, sp_current: u16) -> Character {
        let mut ch = Character::new(
            "Tester".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        ch.hp.base = hp_base;
        ch.hp.current = hp_current;
        ch.sp.base = sp_base;
        ch.sp.current = sp_current;
        ch
    }

    // -----------------------------------------------------------------------
    // HealHp
    // -----------------------------------------------------------------------

    #[test]
    fn test_heal_hp_restores_up_to_base() {
        let mut ch = make_character(30, 10, 0, 0);
        let result = apply_consumable_effect(&mut ch, ConsumableEffect::HealHp(50));
        // current must be capped at base
        assert_eq!(ch.hp.current, 30, "hp.current should be capped at hp.base");
        // only the actual delta (20) is reported
        assert_eq!(result.healing, 20, "healing delta should be 20, not 50");
    }

    #[test]
    fn test_heal_hp_already_full_is_noop() {
        let mut ch = make_character(30, 30, 0, 0);
        let result = apply_consumable_effect(&mut ch, ConsumableEffect::HealHp(10));
        assert_eq!(ch.hp.current, 30, "full-health character should not change");
        assert_eq!(result.healing, 0, "no actual healing should be reported");
    }

    #[test]
    fn test_heal_hp_partial_restore() {
        let mut ch = make_character(100, 50, 0, 0);
        let result = apply_consumable_effect(&mut ch, ConsumableEffect::HealHp(30));
        assert_eq!(ch.hp.current, 80, "current should increase by 30");
        assert_eq!(result.healing, 30);
    }

    // -----------------------------------------------------------------------
    // RestoreSp
    // -----------------------------------------------------------------------

    #[test]
    fn test_restore_sp_capped_at_base() {
        let mut ch = make_character(30, 30, 20, 5);
        let result = apply_consumable_effect(&mut ch, ConsumableEffect::RestoreSp(100));
        assert_eq!(ch.sp.current, 20, "sp.current must be capped at sp.base");
        assert_eq!(result.sp_restored, 15, "only the delta to base is reported");
    }

    #[test]
    fn test_restore_sp_already_full_is_noop() {
        let mut ch = make_character(30, 30, 10, 10);
        let result = apply_consumable_effect(&mut ch, ConsumableEffect::RestoreSp(10));
        assert_eq!(ch.sp.current, 10, "full SP character should not change");
        assert_eq!(result.sp_restored, 0);
    }

    #[test]
    fn test_restore_sp_partial() {
        let mut ch = make_character(30, 30, 20, 0);
        let result = apply_consumable_effect(&mut ch, ConsumableEffect::RestoreSp(8));
        assert_eq!(ch.sp.current, 8);
        assert_eq!(result.sp_restored, 8);
    }

    // -----------------------------------------------------------------------
    // CureCondition
    // -----------------------------------------------------------------------

    #[test]
    fn test_cure_condition_clears_flags() {
        let mut ch = make_character(30, 30, 0, 0);
        ch.conditions.add(Condition::POISONED);
        ch.conditions.add(Condition::BLINDED);

        let result = apply_consumable_effect(
            &mut ch,
            ConsumableEffect::CureCondition(Condition::POISONED),
        );

        assert!(
            !ch.conditions.has(Condition::POISONED),
            "POISONED should be cleared"
        );
        assert!(
            ch.conditions.has(Condition::BLINDED),
            "BLINDED should remain"
        );
        assert_eq!(
            result.conditions_cleared,
            Condition::POISONED,
            "result should record cleared flags"
        );
    }

    #[test]
    fn test_cure_condition_does_not_touch_active_conditions() {
        use crate::domain::conditions::ActiveCondition;

        let mut ch = make_character(30, 30, 0, 0);
        ch.conditions.add(Condition::PARALYZED);

        // Populate active_conditions with a sentinel
        ch.active_conditions.push(ActiveCondition::new(
            "test_paralysis".to_string(),
            crate::domain::conditions::ConditionDuration::Rounds(3),
        ));

        apply_consumable_effect(
            &mut ch,
            ConsumableEffect::CureCondition(Condition::PARALYZED),
        );

        // active_conditions must be untouched
        assert_eq!(
            ch.active_conditions.len(),
            1,
            "active_conditions must not be modified by CureCondition"
        );
        assert_eq!(
            ch.active_conditions[0].condition_id, "test_paralysis",
            "active_conditions entry should be unchanged"
        );
    }

    #[test]
    fn test_cure_condition_noop_when_already_clear() {
        let mut ch = make_character(30, 30, 0, 0);
        // POISONED is not set — cure should be a harmless no-op
        let result = apply_consumable_effect(
            &mut ch,
            ConsumableEffect::CureCondition(Condition::POISONED),
        );
        // flags field still set to the requested value (the remove is idempotent)
        assert_eq!(result.conditions_cleared, Condition::POISONED);
        assert!(ch.conditions.is_fine());
    }

    // -----------------------------------------------------------------------
    // BoostAttribute
    // -----------------------------------------------------------------------

    #[test]
    fn test_boost_attribute_modifies_current_not_base() {
        let mut ch = make_character(30, 30, 0, 0);
        let original_base = ch.stats.might.base;
        let original_current = ch.stats.might.current;

        let result = apply_consumable_effect(
            &mut ch,
            ConsumableEffect::BoostAttribute(AttributeType::Might, 5),
        );

        assert_eq!(
            ch.stats.might.current,
            original_current.saturating_add(5),
            "current should increase by 5"
        );
        assert_eq!(ch.stats.might.base, original_base, "base must not change");
        assert_eq!(result.attribute_delta, 5);
    }

    #[test]
    fn test_boost_attribute_all_stats() {
        // Ensure every AttributeType arm compiles and routes to the right field
        type StatAccessor = fn(&Character) -> (u8, u8);
        let cases: &[(AttributeType, StatAccessor)] = &[
            (AttributeType::Might, |c| {
                (c.stats.might.base, c.stats.might.current)
            }),
            (AttributeType::Intellect, |c| {
                (c.stats.intellect.base, c.stats.intellect.current)
            }),
            (AttributeType::Personality, |c| {
                (c.stats.personality.base, c.stats.personality.current)
            }),
            (AttributeType::Endurance, |c| {
                (c.stats.endurance.base, c.stats.endurance.current)
            }),
            (AttributeType::Speed, |c| {
                (c.stats.speed.base, c.stats.speed.current)
            }),
            (AttributeType::Accuracy, |c| {
                (c.stats.accuracy.base, c.stats.accuracy.current)
            }),
            (AttributeType::Luck, |c| {
                (c.stats.luck.base, c.stats.luck.current)
            }),
        ];

        for (attr, get) in cases {
            let mut ch = make_character(30, 30, 0, 0);
            let (base_before, current_before) = get(&ch);
            apply_consumable_effect(&mut ch, ConsumableEffect::BoostAttribute(*attr, 3));
            let (base_after, current_after) = get(&ch);
            assert_eq!(base_before, base_after, "base must not change for {attr:?}");
            assert_eq!(
                current_after,
                current_before.saturating_add(3),
                "current should increase by 3 for {attr:?}"
            );
        }
    }

    #[test]
    fn test_boost_attribute_negative_debuffs_current() {
        let mut ch = make_character(30, 30, 0, 0);
        let base_before = ch.stats.speed.base;

        apply_consumable_effect(
            &mut ch,
            ConsumableEffect::BoostAttribute(AttributeType::Speed, -3),
        );

        assert_eq!(ch.stats.speed.base, base_before, "base must not change");
        assert_eq!(ch.stats.speed.current, base_before.saturating_sub(3));
    }

    // -----------------------------------------------------------------------
    // BoostResistance
    // -----------------------------------------------------------------------

    #[test]
    fn test_boost_resistance_modifies_current_not_base() {
        let mut ch = make_character(30, 30, 0, 0);
        let base_before = ch.resistances.fire.base;
        let current_before = ch.resistances.fire.current;

        let result = apply_consumable_effect(
            &mut ch,
            ConsumableEffect::BoostResistance(ResistanceType::Fire, 10),
        );

        assert_eq!(
            ch.resistances.fire.current,
            current_before.saturating_add(10),
            "current should increase by 10"
        );
        assert_eq!(
            ch.resistances.fire.base, base_before,
            "base must not change"
        );
        assert_eq!(result.resistance_delta, 10);
    }

    #[test]
    fn test_boost_resistance_all_types() {
        // Verify every ResistanceType arm routes to a field (compilation + runtime check)
        type ResAccessor = fn(&Character) -> u8;
        let cases: &[(ResistanceType, ResAccessor)] = &[
            // Physical → magic
            (ResistanceType::Physical, |c| c.resistances.magic.current),
            (ResistanceType::Fire, |c| c.resistances.fire.current),
            (ResistanceType::Cold, |c| c.resistances.cold.current),
            (ResistanceType::Electricity, |c| {
                c.resistances.electricity.current
            }),
            // Energy → magic
            (ResistanceType::Energy, |c| c.resistances.magic.current),
            // Paralysis → psychic
            (ResistanceType::Paralysis, |c| c.resistances.psychic.current),
            (ResistanceType::Fear, |c| c.resistances.fear.current),
            // Sleep → psychic
            (ResistanceType::Sleep, |c| c.resistances.psychic.current),
        ];

        for (res_type, get_current) in cases {
            let mut ch = make_character(30, 30, 0, 0);
            let before = get_current(&ch);
            apply_consumable_effect(&mut ch, ConsumableEffect::BoostResistance(*res_type, 5));
            let after = get_current(&ch);
            assert_eq!(
                after,
                before.saturating_add(5),
                "resistance field should increase by 5 for {res_type:?}"
            );
        }
    }

    #[test]
    fn test_boost_resistance_base_preserved_all_types() {
        let res_types = [
            ResistanceType::Physical,
            ResistanceType::Fire,
            ResistanceType::Cold,
            ResistanceType::Electricity,
            ResistanceType::Energy,
            ResistanceType::Paralysis,
            ResistanceType::Fear,
            ResistanceType::Sleep,
        ];

        for res_type in res_types {
            let mut ch = make_character(30, 30, 0, 0);
            let bases_before = (
                ch.resistances.magic.base,
                ch.resistances.fire.base,
                ch.resistances.cold.base,
                ch.resistances.electricity.base,
                ch.resistances.acid.base,
                ch.resistances.fear.base,
                ch.resistances.poison.base,
                ch.resistances.psychic.base,
            );

            apply_consumable_effect(&mut ch, ConsumableEffect::BoostResistance(res_type, 20));

            let bases_after = (
                ch.resistances.magic.base,
                ch.resistances.fire.base,
                ch.resistances.cold.base,
                ch.resistances.electricity.base,
                ch.resistances.acid.base,
                ch.resistances.fear.base,
                ch.resistances.poison.base,
                ch.resistances.psychic.base,
            );

            assert_eq!(
                bases_before, bases_after,
                "no resistance base should change for {res_type:?}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // IsFood (no-op)
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_food_is_noop() {
        let mut ch = make_character(30, 20, 10, 5);
        let hp_before = ch.hp.current;
        let sp_before = ch.sp.current;

        let result = apply_consumable_effect(&mut ch, ConsumableEffect::IsFood(1));

        assert_eq!(ch.hp.current, hp_before, "IsFood must not change HP");
        assert_eq!(ch.sp.current, sp_before, "IsFood must not change SP");
        assert_eq!(
            result,
            ConsumableApplyResult::default(),
            "IsFood result must be zeroed"
        );
    }

    // -----------------------------------------------------------------------
    // ConsumableApplyResult default / zero checks
    // -----------------------------------------------------------------------

    #[test]
    fn test_result_default_is_all_zero() {
        let r = ConsumableApplyResult::default();
        assert_eq!(r.healing, 0);
        assert_eq!(r.sp_restored, 0);
        assert_eq!(r.conditions_cleared, 0);
        assert_eq!(r.attribute_delta, 0);
        assert_eq!(r.resistance_delta, 0);
    }
}
