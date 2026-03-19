// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Pure-domain consumable effect application
//!
//! This module provides the single authoritative implementation for applying
//! [`ConsumableEffect`] variants to a [`Character`]. Both the combat path
//! ([`execute_item_use_by_slot`]) and the exploration/menu path share this
//! implementation, ensuring there is no duplicated match logic.
//!
//! # Two Entry Points
//!
//! The module exposes two public functions, each suited to a different game
//! context:
//!
//! - **[`apply_consumable_effect`]** — the *combat* entry point.  Handles all
//!   five effect variants.  For `BoostResistance`, it always uses the permanent
//!   path (mutating `character.resistances` directly) because resistances in
//!   combat are typically granted by party spells already tracked in
//!   [`crate::application::ActiveSpells`].
//!
//! - **[`apply_consumable_effect_exploration`]** — the *exploration* entry
//!   point.  Identical to the combat path for every effect except timed
//!   `BoostResistance`: when `ConsumableData::duration_minutes` is `Some(n >
//!   0)`, the resistance boost is written to the corresponding field in
//!   [`crate::application::ActiveSpells`] instead of directly mutating
//!   `character.resistances`.  This lets the protection expire automatically
//!   via [`crate::application::GameState::advance_time`].
//!
//! # Timed vs. Permanent Boosts
//!
//! `ConsumableData::duration_minutes` controls whether a boost is timed or
//! permanent:
//!
//! | `duration_minutes` | `BoostAttribute` behaviour | `BoostResistance` (exploration) |
//! | --- | --- | --- |
//! | `None` | Mutates `stats.<attr>.current` directly (permanent) | Mutates `resistances.<field>.current` directly (permanent) |
//! | `Some(0)` | Treated as `None` via `normalize_duration` (permanent) | Treated as `None` (permanent) |
//! | `Some(n)` | Registers a [`crate::domain::character::TimedStatBoost`]; reversed after `n` minutes | Writes `n` (clamped to `u8`) into `ActiveSpells.<field>` |
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
//! use antares::domain::items::types::{ConsumableData, ConsumableEffect, AttributeType, ResistanceType};
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
//! let data = ConsumableData {
//!     effect: ConsumableEffect::HealHp(50),
//!     is_combat_usable: true,
//!     duration_minutes: None,
//! };
//! let result = apply_consumable_effect(&mut hero, &data);
//! assert_eq!(hero.hp.current, 30); // capped at base
//! assert_eq!(result.healing, 20);  // only the delta is reported
//! ```

use crate::domain::character::Character;
use crate::domain::items::types::{ConsumableData, ConsumableEffect, ResistanceType};

/// Describes what `apply_consumable_effect` actually changed on the character.
///
/// All fields are zero/false when the corresponding effect did not apply (e.g.
/// `healing == 0` for a `BoostAttribute` potion). Callers can use this to
/// compose player-visible feedback messages without re-deriving the deltas.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::items::types::{ConsumableData, ConsumableEffect};
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
/// let data = ConsumableData {
///     effect: ConsumableEffect::RestoreSp(100),
///     is_combat_usable: true,
///     duration_minutes: None,
/// };
/// let result = apply_consumable_effect(&mut hero, &data);
/// assert_eq!(result.sp_restored, 15); // only the delta up to base
/// assert_eq!(result.healing, 0);      // HP field untouched
/// assert!(!result.attribute_boost_is_timed);
/// assert!(!result.resistance_boost_is_timed);
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
    /// True when a `BoostAttribute` was registered as a timed boost
    pub attribute_boost_is_timed: bool,
    /// True when a `BoostResistance` was handled by the caller's timed layer
    pub resistance_boost_is_timed: bool,
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
/// * `data`      - The full [`ConsumableData`] including optional duration.
///
/// # Returns
///
/// A [`ConsumableApplyResult`] with the actual deltas applied. All fields that
/// do not correspond to the active effect variant are zero/false.
///
/// # Cap Behaviour
///
/// - `HealHp`:    `hp.current` is clamped to `hp.base` after modification.
/// - `RestoreSp`: `sp.current` is clamped to `sp.base` after modification.
/// - All other variants use `AttributePair::modify` / `AttributePair16::modify`
///   which saturates at the type boundary (`u8::MAX` / `u16::MAX`).
///
/// # Timed vs Permanent Boosts
///
/// For `BoostAttribute`, if `normalize_duration(data.duration_minutes)` is
/// `Some`, the boost is registered via `apply_timed_stat_boost` and
/// `attribute_boost_is_timed` is set to `true`. Otherwise the permanent path
/// mutates `current` directly.
///
/// `BoostResistance` **always** uses the permanent path in this function (combat
/// context). Use [`apply_consumable_effect_exploration`] to route timed
/// resistance boosts through `ActiveSpells`.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Character, Sex, Alignment, Condition};
/// use antares::domain::items::types::{ConsumableData, ConsumableEffect, AttributeType, ResistanceType};
/// use antares::domain::items::consumable_usage::apply_consumable_effect;
///
/// // --- HealHp capped at base ---
/// let mut ch = Character::new(
///     "Hero".to_string(), "human".to_string(), "knight".to_string(),
///     Sex::Male, Alignment::Good,
/// );
/// ch.hp.base = 30;
/// ch.hp.current = 10;
/// let r = apply_consumable_effect(&mut ch, &ConsumableData {
///     effect: ConsumableEffect::HealHp(50),
///     is_combat_usable: true,
///     duration_minutes: None,
/// });
/// assert_eq!(ch.hp.current, 30);
/// assert_eq!(r.healing, 20);
///
/// // --- CureCondition removes only the matching bits ---
/// ch.conditions.add(Condition::POISONED);
/// ch.conditions.add(Condition::BLINDED);
/// let r2 = apply_consumable_effect(&mut ch, &ConsumableData {
///     effect: ConsumableEffect::CureCondition(Condition::POISONED),
///     is_combat_usable: true,
///     duration_minutes: None,
/// });
/// assert!(!ch.conditions.has(Condition::POISONED));
/// assert!(ch.conditions.has(Condition::BLINDED));
/// assert_eq!(r2.conditions_cleared, Condition::POISONED);
///
/// // --- BoostAttribute modifies current, not base (permanent) ---
/// let r3 = apply_consumable_effect(&mut ch, &ConsumableData {
///     effect: ConsumableEffect::BoostAttribute(AttributeType::Might, 5),
///     is_combat_usable: true,
///     duration_minutes: None,
/// });
/// assert_eq!(ch.stats.might.current, 15); // default base=10, current=10+5
/// assert_eq!(ch.stats.might.base, 10);
/// assert_eq!(r3.attribute_delta, 5);
/// assert!(!r3.attribute_boost_is_timed);
/// ```
pub fn apply_consumable_effect(
    character: &mut Character,
    data: &ConsumableData,
) -> ConsumableApplyResult {
    use crate::domain::items::types::normalize_duration;

    let mut result = ConsumableApplyResult::default();

    match data.effect {
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
            // Revive from unconscious if HP is now above 0.
            crate::domain::resources::revive_from_unconscious(character);
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
            if normalize_duration(data.duration_minutes).is_some() {
                // Timed path: register via apply_timed_stat_boost so the boost
                // is automatically reversed by tick_timed_stat_boosts_minute.
                character.apply_timed_stat_boost(attr, amount, data.duration_minutes);
                result.attribute_delta = amount as i16;
                result.attribute_boost_is_timed = true;
            } else {
                // Permanent path (legacy behaviour): directly mutate current.
                character.apply_attribute_delta(attr, amount as i16);
                result.attribute_delta = amount as i16;
            }
        }

        ConsumableEffect::BoostResistance(res_type, amount) => {
            apply_resistance_to_character(character, res_type, amount);
            result.resistance_delta = amount as i16;
        }

        ConsumableEffect::IsFood(_) => {
            // Food items are consumed by the rest system, not this path.
            // Silently return the zeroed result so callers can detect no-op.
        }
    }

    result
}

/// Applies a resistance boost directly to the character's resistance fields.
///
/// This is the shared permanent-path helper used by both
/// `apply_consumable_effect` (combat) and `apply_consumable_effect_exploration`
/// (exploration fallthrough for permanent resistance items).
fn apply_resistance_to_character(character: &mut Character, res_type: ResistanceType, amount: i8) {
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
}

/// Applies a consumable effect in the exploration context.
///
/// Identical to [`apply_consumable_effect`] except that `BoostResistance` with
/// a `duration_minutes` is written to `active_spells` instead of directly
/// mutating `character.resistances`, so the effect expires automatically via
/// `GameState::advance_time`.
///
/// For all other effect variants (including permanent `BoostResistance`),
/// this function delegates to [`apply_consumable_effect`].
///
/// # Arguments
///
/// * `character`     — mutable reference to the consuming character
/// * `active_spells` — mutable reference to the party-wide active spells
/// * `data`          — the full [`ConsumableData`] including optional duration
///
/// # Returns
///
/// A [`ConsumableApplyResult`] with `resistance_boost_is_timed = true` when
/// the resistance effect was routed to `active_spells`.
///
/// # Examples
///
/// ```
/// use antares::application::ActiveSpells;
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::items::types::{ConsumableData, ConsumableEffect, ResistanceType};
/// use antares::domain::items::consumable_usage::apply_consumable_effect_exploration;
///
/// let mut hero = Character::new(
///     "Scout".to_string(),
///     "human".to_string(),
///     "robber".to_string(),
///     Sex::Female,
///     Alignment::Neutral,
/// );
/// let mut active_spells = ActiveSpells::new();
/// let fire_before = hero.resistances.fire.current;
///
/// let data = ConsumableData {
///     effect: ConsumableEffect::BoostResistance(ResistanceType::Fire, 25),
///     is_combat_usable: true,
///     duration_minutes: Some(60),
/// };
/// let result = apply_consumable_effect_exploration(&mut hero, &mut active_spells, &data);
///
/// // Timed resistance routes to active_spells, not character.resistances
/// assert_eq!(active_spells.fire_protection, 60);
/// assert_eq!(hero.resistances.fire.current, fire_before);
/// assert!(result.resistance_boost_is_timed);
/// ```
pub fn apply_consumable_effect_exploration(
    character: &mut Character,
    active_spells: &mut crate::application::ActiveSpells,
    data: &ConsumableData,
) -> ConsumableApplyResult {
    use crate::domain::items::types::normalize_duration;

    if let ConsumableEffect::BoostResistance(res_type, amount) = data.effect {
        if let Some(minutes) = normalize_duration(data.duration_minutes) {
            // Clamp to u8 range (overwrite semantics — last write wins).
            let clamped = u16::min(minutes, u8::MAX as u16) as u8;
            match res_type {
                ResistanceType::Fire => active_spells.fire_protection = clamped,
                ResistanceType::Cold => active_spells.cold_protection = clamped,
                ResistanceType::Electricity => active_spells.electricity_protection = clamped,
                ResistanceType::Energy => active_spells.magic_protection = clamped,
                ResistanceType::Fear => active_spells.fear_protection = clamped,
                ResistanceType::Physical => active_spells.magic_protection = clamped,
                ResistanceType::Paralysis => active_spells.psychic_protection = clamped,
                ResistanceType::Sleep => active_spells.psychic_protection = clamped,
            }
            return ConsumableApplyResult {
                resistance_delta: amount as i16,
                resistance_boost_is_timed: true,
                ..Default::default()
            };
        }
        // duration_minutes is None or Some(0) → fall through to permanent path
    }

    // Delegate all other effects (and permanent BoostResistance) to the
    // shared combat helper.
    apply_consumable_effect(character, data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ActiveSpells;
    use crate::domain::character::{Alignment, Character, Condition, Sex};
    use crate::domain::items::types::{
        AttributeType, ConsumableData, ConsumableEffect, ResistanceType,
    };

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

    /// Convenience constructor for test ConsumableData values.
    fn make_consumable_data(
        effect: ConsumableEffect,
        duration_minutes: Option<u16>,
    ) -> ConsumableData {
        ConsumableData {
            effect,
            is_combat_usable: true,
            duration_minutes,
        }
    }

    // -----------------------------------------------------------------------
    // HealHp
    // -----------------------------------------------------------------------

    #[test]
    fn test_heal_hp_restores_up_to_base() {
        let mut ch = make_character(30, 10, 0, 0);
        let data = make_consumable_data(ConsumableEffect::HealHp(50), None);
        let result = apply_consumable_effect(&mut ch, &data);
        // current must be capped at base
        assert_eq!(ch.hp.current, 30, "hp.current should be capped at hp.base");
        // only the actual delta (20) is reported
        assert_eq!(result.healing, 20, "healing delta should be 20, not 50");
    }

    #[test]
    fn test_heal_hp_already_full_is_noop() {
        let mut ch = make_character(30, 30, 0, 0);
        let data = make_consumable_data(ConsumableEffect::HealHp(10), None);
        let result = apply_consumable_effect(&mut ch, &data);
        assert_eq!(ch.hp.current, 30, "full-health character should not change");
        assert_eq!(result.healing, 0, "no actual healing should be reported");
    }

    #[test]
    fn test_heal_hp_partial_restore() {
        let mut ch = make_character(100, 50, 0, 0);
        let data = make_consumable_data(ConsumableEffect::HealHp(30), None);
        let result = apply_consumable_effect(&mut ch, &data);
        assert_eq!(ch.hp.current, 80, "current should increase by 30");
        assert_eq!(result.healing, 30);
    }

    /// A `HealHp` consumable applied to an unconscious (0 HP) character must
    /// clear the UNCONSCIOUS condition once HP is raised above 0.
    #[test]
    fn test_heal_hp_clears_unconscious() {
        use crate::domain::character::Condition;
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};

        let mut ch = make_character(20, 0, 0, 0);
        ch.conditions.add(Condition::UNCONSCIOUS);
        ch.add_condition(ActiveCondition::new(
            "unconscious".to_string(),
            ConditionDuration::Permanent,
        ));

        let data = make_consumable_data(ConsumableEffect::HealHp(10), None);
        let result = apply_consumable_effect(&mut ch, &data);

        assert_eq!(ch.hp.current, 10, "HP should be 10 after healing");
        assert!(result.healing > 0, "result.healing must be positive");
        assert!(
            !ch.conditions.has(Condition::UNCONSCIOUS),
            "UNCONSCIOUS bitflag must be cleared after HealHp raises HP above 0"
        );
        assert!(
            ch.active_conditions
                .iter()
                .all(|c| c.condition_id != "unconscious"),
            "active_conditions must not contain 'unconscious' after revival"
        );
    }

    /// A `HealHp(0)` that does not raise HP above 0 must leave UNCONSCIOUS set.
    #[test]
    fn test_heal_hp_does_not_clear_when_still_zero() {
        use crate::domain::character::Condition;
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};

        let mut ch = make_character(20, 0, 0, 0);
        ch.conditions.add(Condition::UNCONSCIOUS);
        ch.add_condition(ActiveCondition::new(
            "unconscious".to_string(),
            ConditionDuration::Permanent,
        ));

        // HealHp(0) — does not raise HP
        let data = make_consumable_data(ConsumableEffect::HealHp(0), None);
        apply_consumable_effect(&mut ch, &data);

        assert_eq!(ch.hp.current, 0, "HP must remain 0 after HealHp(0)");
        assert!(
            ch.conditions.has(Condition::UNCONSCIOUS),
            "UNCONSCIOUS must remain set when HP does not rise above 0"
        );
    }

    // -----------------------------------------------------------------------
    // RestoreSp
    // -----------------------------------------------------------------------

    #[test]
    fn test_restore_sp_capped_at_base() {
        let mut ch = make_character(30, 30, 20, 5);
        let data = make_consumable_data(ConsumableEffect::RestoreSp(100), None);
        let result = apply_consumable_effect(&mut ch, &data);
        assert_eq!(ch.sp.current, 20, "sp.current must be capped at sp.base");
        assert_eq!(result.sp_restored, 15, "only the delta to base is reported");
    }

    #[test]
    fn test_restore_sp_already_full_is_noop() {
        let mut ch = make_character(30, 30, 10, 10);
        let data = make_consumable_data(ConsumableEffect::RestoreSp(10), None);
        let result = apply_consumable_effect(&mut ch, &data);
        assert_eq!(ch.sp.current, 10, "full SP character should not change");
        assert_eq!(result.sp_restored, 0);
    }

    #[test]
    fn test_restore_sp_partial() {
        let mut ch = make_character(30, 30, 20, 0);
        let data = make_consumable_data(ConsumableEffect::RestoreSp(8), None);
        let result = apply_consumable_effect(&mut ch, &data);
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

        let data = make_consumable_data(ConsumableEffect::CureCondition(Condition::POISONED), None);
        let result = apply_consumable_effect(&mut ch, &data);

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

        let data =
            make_consumable_data(ConsumableEffect::CureCondition(Condition::PARALYZED), None);
        apply_consumable_effect(&mut ch, &data);

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
        let data = make_consumable_data(ConsumableEffect::CureCondition(Condition::POISONED), None);
        let result = apply_consumable_effect(&mut ch, &data);
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

        let data = make_consumable_data(
            ConsumableEffect::BoostAttribute(AttributeType::Might, 5),
            None,
        );
        let result = apply_consumable_effect(&mut ch, &data);

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
            let data = make_consumable_data(ConsumableEffect::BoostAttribute(*attr, 3), None);
            apply_consumable_effect(&mut ch, &data);
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

        let data = make_consumable_data(
            ConsumableEffect::BoostAttribute(AttributeType::Speed, -3),
            None,
        );
        apply_consumable_effect(&mut ch, &data);

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

        let data = make_consumable_data(
            ConsumableEffect::BoostResistance(ResistanceType::Fire, 10),
            None,
        );
        let result = apply_consumable_effect(&mut ch, &data);

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
            let data = make_consumable_data(ConsumableEffect::BoostResistance(*res_type, 5), None);
            apply_consumable_effect(&mut ch, &data);
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

            let data = make_consumable_data(ConsumableEffect::BoostResistance(res_type, 20), None);
            apply_consumable_effect(&mut ch, &data);

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

        let data = make_consumable_data(ConsumableEffect::IsFood(1), None);
        let result = apply_consumable_effect(&mut ch, &data);

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
        assert!(!r.attribute_boost_is_timed);
        assert!(!r.resistance_boost_is_timed);
    }

    // -----------------------------------------------------------------------
    // Phase 3 — Timed boost tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_apply_timed_attribute_boost_registers_in_timed_list() {
        let mut ch = make_character(30, 30, 0, 0);
        let data = make_consumable_data(
            ConsumableEffect::BoostAttribute(AttributeType::Might, 5),
            Some(30),
        );
        let result = apply_consumable_effect(&mut ch, &data);
        assert_eq!(
            ch.timed_stat_boosts.len(),
            1,
            "timed list should have one entry"
        );
        assert!(result.attribute_boost_is_timed);
        assert_eq!(result.attribute_delta, 5);
    }

    #[test]
    fn test_apply_permanent_attribute_boost_mutates_directly() {
        let mut ch = make_character(30, 30, 0, 0);
        let before = ch.stats.might.current;
        let data = make_consumable_data(
            ConsumableEffect::BoostAttribute(AttributeType::Might, 5),
            None,
        );
        let result = apply_consumable_effect(&mut ch, &data);
        assert!(
            ch.timed_stat_boosts.is_empty(),
            "no timed entry for permanent boost"
        );
        assert_eq!(ch.stats.might.current, before + 5);
        assert!(!result.attribute_boost_is_timed);
    }

    #[test]
    fn test_apply_timed_resistance_routes_to_active_spells() {
        let mut ch = make_character(30, 30, 0, 0);
        let mut active_spells = ActiveSpells::new();
        let fire_before = ch.resistances.fire.current;
        let data = make_consumable_data(
            ConsumableEffect::BoostResistance(ResistanceType::Fire, 25),
            Some(60),
        );
        let result = apply_consumable_effect_exploration(&mut ch, &mut active_spells, &data);
        assert_eq!(
            active_spells.fire_protection, 60,
            "fire_protection should be set to 60"
        );
        assert_eq!(
            ch.resistances.fire.current, fire_before,
            "character resistances must be unchanged"
        );
        assert!(result.resistance_boost_is_timed);
    }

    #[test]
    fn test_apply_permanent_resistance_mutates_character_directly() {
        let mut ch = make_character(30, 30, 0, 0);
        let mut active_spells = ActiveSpells::new();
        let fire_before = ch.resistances.fire.current;
        let data = make_consumable_data(
            ConsumableEffect::BoostResistance(ResistanceType::Fire, 25),
            None,
        );
        let result = apply_consumable_effect_exploration(&mut ch, &mut active_spells, &data);
        assert!(
            ch.resistances.fire.current > fire_before,
            "permanent path must mutate character"
        );
        assert_eq!(
            active_spells.fire_protection, 0,
            "active_spells must be untouched"
        );
        assert!(!result.resistance_boost_is_timed);
    }

    #[test]
    fn test_resistance_stacking_overwrites_duration() {
        let mut ch = make_character(30, 30, 0, 0);
        let mut active_spells = ActiveSpells::new();
        let data1 = make_consumable_data(
            ConsumableEffect::BoostResistance(ResistanceType::Cold, 10),
            Some(30),
        );
        let data2 = make_consumable_data(
            ConsumableEffect::BoostResistance(ResistanceType::Cold, 10),
            Some(90),
        );
        apply_consumable_effect_exploration(&mut ch, &mut active_spells, &data1);
        assert_eq!(active_spells.cold_protection, 30);
        apply_consumable_effect_exploration(&mut ch, &mut active_spells, &data2);
        assert_eq!(
            active_spells.cold_protection, 90,
            "second call should overwrite with 90"
        );
    }

    #[test]
    fn test_resistance_u8_clamping() {
        let mut ch = make_character(30, 30, 0, 0);
        let mut active_spells = ActiveSpells::new();
        let data = make_consumable_data(
            ConsumableEffect::BoostResistance(ResistanceType::Fire, 10),
            Some(300),
        );
        apply_consumable_effect_exploration(&mut ch, &mut active_spells, &data);
        assert_eq!(
            active_spells.fire_protection,
            u8::MAX,
            "300 must be clamped to 255"
        );
    }

    #[test]
    fn test_timed_resistance_all_eight_types_map_correctly() {
        type ActiveSpellsGetter = fn(&ActiveSpells) -> u8;
        let cases: &[(ResistanceType, ActiveSpellsGetter)] = &[
            (ResistanceType::Fire, |a| a.fire_protection),
            (ResistanceType::Cold, |a| a.cold_protection),
            (ResistanceType::Electricity, |a| a.electricity_protection),
            (ResistanceType::Energy, |a| a.magic_protection),
            (ResistanceType::Fear, |a| a.fear_protection),
            (ResistanceType::Physical, |a| a.magic_protection),
            (ResistanceType::Paralysis, |a| a.psychic_protection),
            (ResistanceType::Sleep, |a| a.psychic_protection),
        ];
        for (res_type, get_field) in cases {
            let mut ch = make_character(30, 30, 0, 0);
            let mut active_spells = ActiveSpells::new();
            let data =
                make_consumable_data(ConsumableEffect::BoostResistance(*res_type, 5), Some(50));
            apply_consumable_effect_exploration(&mut ch, &mut active_spells, &data);
            assert_eq!(
                get_field(&active_spells),
                50,
                "active_spells field for {res_type:?} should be 50"
            );
        }
    }

    #[test]
    fn test_combat_resistance_boost_still_permanent() {
        // apply_consumable_effect (not exploration) with BoostResistance + duration
        // must still mutate character.resistances directly (combat path is permanent)
        let mut ch = make_character(30, 30, 0, 0);
        let fire_before = ch.resistances.fire.current;
        let data = make_consumable_data(
            ConsumableEffect::BoostResistance(ResistanceType::Fire, 25),
            Some(60),
        );
        let result = apply_consumable_effect(&mut ch, &data);
        assert!(
            ch.resistances.fire.current > fire_before,
            "combat path must mutate fire resistance"
        );
        assert!(
            !result.resistance_boost_is_timed,
            "combat path must not set resistance_boost_is_timed"
        );
    }
}
