// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Spell effect dispatcher — routes spells to their correct game-state mutations
//!
//! This module is the **central routing layer** for spell effect resolution.
//! Both the combat casting system (`domain::combat::spell_casting`) and the
//! exploration casting system (Phase 3) delegate to the functions here for
//! healing, buff, condition-cure, utility, and composite spell effects.
//!
//! # Design
//!
//! Each effect category has a focused helper function that performs a single,
//! well-defined state mutation:
//!
//! | Helper | Mutation |
//! |--------|----------|
//! | [`apply_healing_spell`] | `character.hp.current += amount` (clamped to base) |
//! | [`apply_buff_spell`] | `active_spells.{field} = duration` |
//! | [`apply_cure_condition`] | `character.remove_condition(id)` + bitfield clear |
//! | [`apply_utility_spell`] | returns food amount / logs teleport / info |
//!
//! The top-level [`apply_spell_effect`] function dispatches to the above
//! helpers based on the spell's [`SpellEffectType`] (explicit or inferred).
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 5.3 and the Spell System
//! Updates Implementation Plan Phase 1 for complete specifications.

use crate::application::ActiveSpells;
use crate::domain::character::Character;
use crate::domain::magic::types::{
    BuffField, Spell, SpellEffectType, TeleportDestination, UtilityType,
};
use crate::domain::types::DiceRoll;
use rand::Rng;

// ===== Result Types =====

/// Result of applying a healing spell to a single character.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::effect_dispatch::HealResult;
///
/// let result = HealResult { hp_restored: 12, already_at_max: false };
/// assert_eq!(result.hp_restored, 12);
/// assert!(!result.already_at_max);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealResult {
    /// HP actually restored after clamping to `character.hp.base`
    pub hp_restored: u16,
    /// Whether the target was already at maximum HP before healing
    pub already_at_max: bool,
}

/// Result of applying a buff spell to the party's [`ActiveSpells`] tracker.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::effect_dispatch::BuffResult;
/// use antares::domain::magic::types::BuffField;
///
/// let result = BuffResult { buff_field: BuffField::Bless, duration_set: 30 };
/// assert_eq!(result.duration_set, 30);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuffResult {
    /// Which `ActiveSpells` field was modified
    pub buff_field: BuffField,
    /// Duration value written to the field
    pub duration_set: u8,
}

/// Result of curing a condition from a character.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::effect_dispatch::CureConditionResult;
///
/// let result = CureConditionResult {
///     condition_id: "paralyzed".to_string(),
///     was_present: true,
/// };
/// assert!(result.was_present);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CureConditionResult {
    /// The condition ID that was targeted
    pub condition_id: String,
    /// Whether the condition was actually present (and therefore removed)
    pub was_present: bool,
}

/// Result of a utility spell effect.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::effect_dispatch::UtilityResult;
/// use antares::domain::magic::types::UtilityType;
///
/// let result = UtilityResult {
///     utility_type: UtilityType::CreateFood { amount: 5 },
///     food_created: 5,
///     message: "Create Food produces 5 food rations.".to_string(),
///     teleport_destination: None,
/// };
/// assert_eq!(result.food_created, 5);
/// assert!(result.teleport_destination.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtilityResult {
    /// Sub-type of utility effect that was triggered
    pub utility_type: UtilityType,
    /// Food ration units produced (zero for non-food utilities)
    pub food_created: u32,
    /// Human-readable description of the effect
    pub message: String,
    /// Teleport destination if this was a [`UtilityType::Teleport`] spell.
    ///
    /// The exploration layer reads this field and applies the corresponding
    /// world-state change (party position / current map).  `None` for all
    /// non-teleport utility spells.
    pub teleport_destination: Option<TeleportDestination>,
}

/// Aggregate result returned by [`apply_spell_effect`].
///
/// Collects all mutations that occurred during a single dispatch call so the
/// caller (combat or exploration system) can incorporate them into feedback
/// messages and `SpellResult` construction.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::effect_dispatch::SpellEffectResult;
///
/// let result = SpellEffectResult::no_op("Nothing happened.");
/// assert!(result.success);
/// assert_eq!(result.total_hp_healed, 0);
/// assert!(result.buff_applied.is_none());
/// assert!(result.condition_cured.is_none());
/// assert_eq!(result.food_created, 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellEffectResult {
    /// Whether the spell effect succeeded
    pub success: bool,
    /// Human-readable description of what happened
    pub message: String,
    /// Total HP healed across all affected targets (0 if not a healing spell)
    pub total_hp_healed: i32,
    /// Buff applied to `ActiveSpells` (if this was a buff spell)
    pub buff_applied: Option<BuffResult>,
    /// Condition cured from the target (if this was a cure spell)
    pub condition_cured: Option<CureConditionResult>,
    /// Food ration units created (non-zero only for `CreateFood` utility spells)
    pub food_created: u32,
    /// Indices of character targets whose HP was modified (caller supplies indices)
    pub affected_targets: Vec<usize>,
}

impl SpellEffectResult {
    /// Creates an empty, successful no-op result with a custom message.
    pub fn no_op(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            total_hp_healed: 0,
            buff_applied: None,
            condition_cured: None,
            food_created: 0,
            affected_targets: Vec::new(),
        }
    }
}

// ===== Core Dispatch Helpers =====

/// Restores HP to a single character by rolling `amount` dice.
///
/// HP is restored by the rolled amount and then clamped to `target.hp.base`
/// (the character's maximum HP) so healing can never exceed the cap.
///
/// # Arguments
///
/// * `amount`  — dice specification for how much HP to restore
/// * `target`  — character to heal (mutated in place)
/// * `rng`     — random number generator for the roll
///
/// # Returns
///
/// A [`HealResult`] describing how many HP were actually restored and whether
/// the character was already at maximum HP.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::effect_dispatch::{apply_healing_spell, HealResult};
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::types::DiceRoll;
///
/// let mut character = Character::new(
///     "Hero".to_string(), "human".to_string(), "knight".to_string(),
///     Sex::Male, Alignment::Good,
/// );
/// character.hp.base = 20;
/// character.hp.current = 5;
///
/// let amount = DiceRoll::new(2, 4, 0); // 2d4 healing
/// let result = apply_healing_spell(amount, &mut character, &mut rand::rng());
/// assert!(result.hp_restored > 0);
/// assert!(character.hp.current >= 5);
/// assert!(character.hp.current <= 20);
/// ```
pub fn apply_healing_spell<R: Rng>(
    amount: DiceRoll,
    target: &mut Character,
    rng: &mut R,
) -> HealResult {
    let already_at_max = target.hp.current >= target.hp.base;
    let roll = amount.roll(rng) as u16;
    let before = target.hp.current;
    // Clamp restored HP to the character's maximum (base HP)
    let new_hp = u16::min(target.hp.current.saturating_add(roll), target.hp.base);
    target.hp.current = new_hp;
    let hp_restored = target.hp.current.saturating_sub(before);
    HealResult {
        hp_restored,
        already_at_max,
    }
}

/// Writes a buff duration into the appropriate [`ActiveSpells`] field.
///
/// The `duration` value directly replaces the current field value — it does
/// not stack.  A duration of 0 effectively removes the buff.
///
/// # Arguments
///
/// * `buff_field`    — which field in `ActiveSpells` to write
/// * `duration`      — duration to set (rounds in combat, minutes in exploration)
/// * `active_spells` — party-wide active spell tracker (mutated in place)
///
/// # Returns
///
/// A [`BuffResult`] echoing the field and duration that were applied.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::effect_dispatch::{apply_buff_spell, BuffResult};
/// use antares::domain::magic::types::BuffField;
/// use antares::application::ActiveSpells;
///
/// let mut active = ActiveSpells::new();
/// assert_eq!(active.bless, 0);
///
/// let result = apply_buff_spell(BuffField::Bless, 30, &mut active);
/// assert_eq!(active.bless, 30);
/// assert_eq!(result.duration_set, 30);
/// assert_eq!(result.buff_field, BuffField::Bless);
/// ```
pub fn apply_buff_spell(
    buff_field: BuffField,
    duration: u8,
    active_spells: &mut ActiveSpells,
) -> BuffResult {
    match buff_field {
        BuffField::FearProtection => active_spells.fear_protection = duration,
        BuffField::ColdProtection => active_spells.cold_protection = duration,
        BuffField::FireProtection => active_spells.fire_protection = duration,
        BuffField::PoisonProtection => active_spells.poison_protection = duration,
        BuffField::AcidProtection => active_spells.acid_protection = duration,
        BuffField::ElectricityProtection => active_spells.electricity_protection = duration,
        BuffField::MagicProtection => active_spells.magic_protection = duration,
        BuffField::Light => active_spells.light = duration,
        BuffField::LeatherSkin => active_spells.leather_skin = duration,
        BuffField::Levitate => active_spells.levitate = duration,
        BuffField::WalkOnWater => active_spells.walk_on_water = duration,
        BuffField::GuardDog => active_spells.guard_dog = duration,
        BuffField::PsychicProtection => active_spells.psychic_protection = duration,
        BuffField::Bless => active_spells.bless = duration,
        BuffField::Invisibility => active_spells.invisibility = duration,
        BuffField::Shield => active_spells.shield = duration,
        BuffField::PowerShield => active_spells.power_shield = duration,
        BuffField::Cursed => active_spells.cursed = duration,
    }
    BuffResult {
        buff_field,
        duration_set: duration,
    }
}

/// Removes a named condition from a character.
///
/// Clears the condition from both `character.active_conditions` (timed/stacked
/// entries) and the `character.conditions` bitfield (for status conditions that
/// map to a bitfield flag such as `PARALYZED`, `POISONED`, etc.).
///
/// This function is a no-op when the character does not have the condition.
///
/// # Arguments
///
/// * `condition_id` — ID string of the condition to remove (e.g. `"paralyzed"`)
/// * `target`       — character to cure (mutated in place)
///
/// # Returns
///
/// A [`CureConditionResult`] indicating whether the condition was present.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::effect_dispatch::{apply_cure_condition, CureConditionResult};
/// use antares::domain::character::{Character, Sex, Alignment, Condition};
/// use antares::domain::conditions::{ActiveCondition, ConditionDuration};
///
/// let mut character = Character::new(
///     "Hero".to_string(), "human".to_string(), "knight".to_string(),
///     Sex::Male, Alignment::Good,
/// );
/// character.conditions.add(Condition::PARALYZED);
/// character.add_condition(ActiveCondition::new(
///     "paralyzed".to_string(),
///     ConditionDuration::Rounds(5),
/// ));
///
/// let result = apply_cure_condition("paralyzed", &mut character);
/// assert!(result.was_present);
/// assert!(!character.conditions.has(Condition::PARALYZED));
/// assert!(character.active_conditions.is_empty());
/// ```
pub fn apply_cure_condition(condition_id: &str, target: &mut Character) -> CureConditionResult {
    use crate::domain::character::Condition;

    let was_present = target
        .active_conditions
        .iter()
        .any(|ac| ac.condition_id == condition_id);

    if was_present {
        // Remove the active condition entry
        target.remove_condition(condition_id);

        // Also clear the matching bitfield flag so status checks (e.g.
        // is_silenced(), can_act()) immediately reflect the cure.
        let flag: Option<u8> = match condition_id.to_lowercase().as_str() {
            "asleep" | "sleep" => Some(Condition::ASLEEP),
            "blinded" | "blind" => Some(Condition::BLINDED),
            "silenced" | "silence" => Some(Condition::SILENCED),
            "diseased" | "disease" => Some(Condition::DISEASED),
            "poisoned" | "poison" => Some(Condition::POISONED),
            "paralyzed" | "paralysis" | "paralyse" => Some(Condition::PARALYZED),
            "unconscious" => Some(Condition::UNCONSCIOUS),
            _ => None,
        };
        if let Some(f) = flag {
            target.conditions.remove(f);
        }
    }

    CureConditionResult {
        condition_id: condition_id.to_string(),
        was_present,
    }
}

/// Computes the result of a utility spell effect.
///
/// This function does **not** mutate game state directly — it returns a
/// [`UtilityResult`] that the calling layer (combat or exploration) uses to
/// apply the effect.  For example, `CreateFood` returns `food_created: N` and
/// the exploration layer adds `N` food ration items to party inventories.
///
/// # Arguments
///
/// * `utility_type` — the specific utility category to handle
///
/// # Returns
///
/// A [`UtilityResult`] describing the effect.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::effect_dispatch::{apply_utility_spell, UtilityResult};
/// use antares::domain::magic::types::UtilityType;
///
/// let result = apply_utility_spell(UtilityType::CreateFood { amount: 6 });
/// assert_eq!(result.food_created, 6);
/// assert!(result.teleport_destination.is_none());
///
/// use antares::domain::magic::types::TeleportDestination;
/// let portal = apply_utility_spell(UtilityType::Teleport {
///     destination: TeleportDestination::TownPortal,
/// });
/// assert_eq!(portal.teleport_destination, Some(TeleportDestination::TownPortal));
///
/// let info = apply_utility_spell(UtilityType::Information);
/// assert_eq!(info.food_created, 0);
/// assert!(info.teleport_destination.is_none());
/// ```
pub fn apply_utility_spell(utility_type: UtilityType) -> UtilityResult {
    match utility_type {
        UtilityType::CreateFood { amount } => UtilityResult {
            utility_type,
            food_created: amount,
            message: format!("Create Food produces {amount} food rations."),
            teleport_destination: None,
        },
        UtilityType::Teleport { destination } => UtilityResult {
            utility_type: UtilityType::Teleport { destination },
            food_created: 0,
            message: match destination {
                TeleportDestination::Surface => "Returning to the surface...".to_string(),
                TeleportDestination::TownPortal => "Teleporting to town...".to_string(),
                TeleportDestination::Jump => "Jumping forward...".to_string(),
            },
            teleport_destination: Some(destination),
        },
        UtilityType::Information => UtilityResult {
            utility_type,
            food_created: 0,
            message: "Information gathered.".to_string(),
            teleport_destination: None,
        },
    }
}

/// Central spell effect dispatcher — routes a spell to the correct mutation.
///
/// Inspects [`Spell::effective_effect_type`] and calls the appropriate helper:
///
/// - `Healing` → [`apply_healing_spell`] on `target`
/// - `Buff` → [`apply_buff_spell`] on `active_spells`
/// - `CureCondition` → [`apply_cure_condition`] on `target`
/// - `Utility` → [`apply_utility_spell`] (no direct state mutation)
/// - `Composite` → dispatches each sub-effect in order
/// - `Damage`, `Debuff`, `Resurrection` → returns a no-op (handled by the
///   combat engine's existing paths)
///
/// For party-wide effects (`AllCharacters` target), the **caller** is
/// responsible for iterating over party members and calling this function once
/// per member.  This function operates on a single optional character target.
///
/// # Arguments
///
/// * `spell`         — spell definition (used for effect type and name)
/// * `target`        — optional mutable reference to a single target character
/// * `active_spells` — party-wide active spell tracker
/// * `rng`           — random number generator for healing dice
///
/// # Returns
///
/// A [`SpellEffectResult`] describing what changed.  The caller fills in
/// `affected_targets` indices appropriate for its context.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::effect_dispatch::{apply_spell_effect, SpellEffectResult};
/// use antares::domain::magic::types::{
///     Spell, SpellSchool, SpellContext, SpellTarget, SpellEffectType, BuffField,
/// };
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::application::ActiveSpells;
/// use antares::domain::types::DiceRoll;
///
/// // Build a Bless buff spell
/// let mut bless = Spell::new(
///     0x0102, "Bless", SpellSchool::Cleric, 2, 3, 0,
///     SpellContext::Anytime, SpellTarget::AllCharacters,
///     "Grants a combat bonus", None, 30, false,
/// );
/// bless.effect_type = Some(SpellEffectType::Buff {
///     buff_field: BuffField::Bless,
///     duration: 30,
/// });
///
/// let mut active = ActiveSpells::new();
/// let result = apply_spell_effect(&bless, None, &mut active, &mut rand::rng());
///
/// assert!(result.success);
/// assert!(result.buff_applied.is_some());
/// assert_eq!(active.bless, 30);
/// ```
pub fn apply_spell_effect<R: Rng>(
    spell: &Spell,
    target: Option<&mut Character>,
    active_spells: &mut ActiveSpells,
    rng: &mut R,
) -> SpellEffectResult {
    match spell.effective_effect_type() {
        // ── Healing ──────────────────────────────────────────────────────────
        SpellEffectType::Healing { amount } => {
            if let Some(character) = target {
                let heal = apply_healing_spell(amount, character, rng);
                SpellEffectResult {
                    success: true,
                    message: format!("{} restores {} HP.", spell.name, heal.hp_restored),
                    total_hp_healed: heal.hp_restored as i32,
                    buff_applied: None,
                    condition_cured: None,
                    food_created: 0,
                    affected_targets: Vec::new(),
                }
            } else {
                SpellEffectResult::no_op(format!("{} has no target.", spell.name))
            }
        }

        // ── Buff ─────────────────────────────────────────────────────────────
        SpellEffectType::Buff {
            buff_field,
            duration,
        } => {
            let buff = apply_buff_spell(buff_field, duration, active_spells);
            SpellEffectResult {
                success: true,
                message: format!("{} is active for {} rounds.", spell.name, duration),
                total_hp_healed: 0,
                buff_applied: Some(buff),
                condition_cured: None,
                food_created: 0,
                affected_targets: Vec::new(),
            }
        }

        // ── Cure Condition ───────────────────────────────────────────────────
        SpellEffectType::CureCondition { condition_id } => {
            if let Some(character) = target {
                let cure = apply_cure_condition(&condition_id, character);
                let message = if cure.was_present {
                    format!(
                        "{} cures {} of {}.",
                        spell.name, character.name, condition_id
                    )
                } else {
                    format!(
                        "{} has no effect — condition '{}' not present.",
                        spell.name, condition_id
                    )
                };
                SpellEffectResult {
                    success: true,
                    message,
                    total_hp_healed: 0,
                    buff_applied: None,
                    condition_cured: Some(cure),
                    food_created: 0,
                    affected_targets: Vec::new(),
                }
            } else {
                SpellEffectResult::no_op(format!("{} has no target.", spell.name))
            }
        }

        // ── Utility ──────────────────────────────────────────────────────────
        SpellEffectType::Utility { utility_type } => {
            let util = apply_utility_spell(utility_type);
            let food = util.food_created;
            let msg = util.message.clone();
            SpellEffectResult {
                success: true,
                message: msg,
                total_hp_healed: 0,
                buff_applied: None,
                condition_cured: None,
                food_created: food,
                affected_targets: Vec::new(),
            }
        }

        // ── Composite ────────────────────────────────────────────────────────
        //
        // Two-pass approach:
        //   Pass 1 — non-character effects (Buff, Utility) that don't require
        //             a mutable borrow of `target`.
        //   Pass 2 — character effects (Healing, CureCondition) using the
        //             single mutable character borrow.
        SpellEffectType::Composite(sub_effects) => {
            let mut total_healed = 0i32;
            let mut buff_applied: Option<BuffResult> = None;
            let mut condition_cured: Option<CureConditionResult> = None;
            let mut food_created = 0u32;

            // Pass 1: effects that do not need the target character
            for sub in &sub_effects {
                match sub {
                    SpellEffectType::Buff {
                        buff_field,
                        duration,
                    } => {
                        let br = apply_buff_spell(*buff_field, *duration, active_spells);
                        buff_applied = Some(br);
                    }
                    SpellEffectType::Utility { utility_type } => {
                        let ur = apply_utility_spell(*utility_type);
                        food_created = food_created.saturating_add(ur.food_created);
                    }
                    _ => {}
                }
            }

            // Pass 2: effects that mutate the target character
            if let Some(character) = target {
                for sub in &sub_effects {
                    match sub {
                        SpellEffectType::Healing { amount } => {
                            let hr = apply_healing_spell(*amount, character, rng);
                            total_healed = total_healed.saturating_add(hr.hp_restored as i32);
                        }
                        SpellEffectType::CureCondition { condition_id } => {
                            let cr = apply_cure_condition(condition_id, character);
                            condition_cured = Some(cr);
                        }
                        _ => {}
                    }
                }
            }

            SpellEffectResult {
                success: true,
                message: format!("{} applies multiple effects.", spell.name),
                total_hp_healed: total_healed,
                buff_applied,
                condition_cured,
                food_created,
                affected_targets: Vec::new(),
            }
        }

        // ── DispelMagic ───────────────────────────────────────────────────────
        // Resets every `ActiveSpells` field to 0, clearing all party buffs.
        SpellEffectType::DispelMagic => {
            active_spells.reset();
            SpellEffectResult {
                success: true,
                message: format!("{} dispels all active magic!", spell.name),
                total_hp_healed: 0,
                buff_applied: None,
                condition_cured: None,
                food_created: 0,
                affected_targets: Vec::new(),
            }
        }

        // ── Damage / Debuff / Resurrection ───────────────────────────────────
        // These are handled by the combat engine's existing code paths.
        // Return a no-op so the caller knows the dispatcher did not act.
        SpellEffectType::Damage | SpellEffectType::Debuff | SpellEffectType::Resurrection => {
            SpellEffectResult::no_op(format!("{} effect handled by combat engine.", spell.name))
        }
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ActiveSpells;
    use crate::domain::character::{Alignment, Character, Condition, Sex};
    use crate::domain::conditions::{ActiveCondition, ConditionDuration};
    use crate::domain::magic::types::{
        BuffField, Spell, SpellContext, SpellEffectType, SpellSchool, SpellTarget,
        TeleportDestination, UtilityType,
    };
    use crate::domain::types::DiceRoll;

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn make_character(name: &str, hp_base: u16, hp_current: u16) -> Character {
        let mut c = Character::new(
            name.to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        c.hp.base = hp_base;
        c.hp.current = hp_current;
        c
    }

    fn make_heal_spell(amount: DiceRoll) -> Spell {
        let mut s = Spell::new(
            0x0101,
            "Cure Wounds",
            SpellSchool::Cleric,
            1,
            2,
            0,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Restores HP",
            None,
            0,
            false,
        );
        s.effect_type = Some(SpellEffectType::Healing { amount });
        s
    }

    fn make_buff_spell(buff_field: BuffField, duration: u8) -> Spell {
        let mut s = Spell::new(
            0x0102,
            "Bless",
            SpellSchool::Cleric,
            2,
            3,
            0,
            SpellContext::Anytime,
            SpellTarget::AllCharacters,
            "Grants a combat bonus",
            None,
            duration as u16,
            false,
        );
        s.effect_type = Some(SpellEffectType::Buff {
            buff_field,
            duration,
        });
        s
    }

    fn make_cure_spell(condition_id: &str) -> Spell {
        let mut s = Spell::new(
            0x0103,
            "Cure Paralysis",
            SpellSchool::Cleric,
            2,
            3,
            0,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Removes paralysis",
            None,
            0,
            false,
        );
        s.effect_type = Some(SpellEffectType::CureCondition {
            condition_id: condition_id.to_string(),
        });
        s
    }

    fn make_utility_spell(utility_type: UtilityType) -> Spell {
        let mut s = Spell::new(
            0x0104,
            "Create Food",
            SpellSchool::Cleric,
            3,
            5,
            0,
            SpellContext::NonCombatOnly,
            SpellTarget::AllCharacters,
            "Creates food rations",
            None,
            0,
            false,
        );
        s.effect_type = Some(SpellEffectType::Utility { utility_type });
        s
    }

    // ── apply_healing_spell ───────────────────────────────────────────────────

    /// Single-target healing restores HP without exceeding base maximum.
    #[test]
    fn test_apply_healing_spell_restores_hp() {
        let mut character = make_character("Hero", 20, 5);
        let amount = DiceRoll::new(2, 4, 0); // 2d4 → 2–8
        let result = apply_healing_spell(amount, &mut character, &mut rand::rng());

        assert!(result.hp_restored > 0, "HP should be restored");
        assert!(!result.already_at_max);
        assert!(
            character.hp.current >= 5,
            "HP must not decrease after healing"
        );
        assert!(
            character.hp.current <= 20,
            "HP must not exceed base maximum"
        );
    }

    /// Healing is clamped to the character's base (max) HP.
    #[test]
    fn test_apply_healing_spell_clamps_to_max() {
        let mut character = make_character("Hero", 10, 8);
        // A very large dice (1d100) will almost certainly overflow base 10
        let amount = DiceRoll::new(1, 100, 0);
        let result = apply_healing_spell(amount, &mut character, &mut rand::rng());

        assert_eq!(
            character.hp.current, 10,
            "HP must be clamped to base after overflow heal"
        );
        assert_eq!(
            result.hp_restored, 2,
            "Only the difference to max should be reported"
        );
    }

    /// Healing a character already at full HP reports zero restored.
    #[test]
    fn test_apply_healing_spell_no_op_at_full_hp() {
        let mut character = make_character("Hero", 20, 20);
        let amount = DiceRoll::new(2, 6, 0);
        let result = apply_healing_spell(amount, &mut character, &mut rand::rng());

        assert!(result.already_at_max, "Character was already at max HP");
        assert_eq!(
            result.hp_restored, 0,
            "No HP should be reported when already full"
        );
        assert_eq!(character.hp.current, 20);
    }

    /// Party-wide healing: applying healing to multiple characters independently.
    #[test]
    fn test_apply_healing_spell_party_wide() {
        let mut members = vec![
            make_character("Member1", 20, 5),
            make_character("Member2", 15, 3),
            make_character("Member3", 25, 10),
        ];
        let amount = DiceRoll::new(1, 8, 0); // 1d8
        let mut rng = rand::rng();

        let mut total_healed = 0u32;
        for member in &mut members {
            let result = apply_healing_spell(amount, member, &mut rng);
            total_healed += result.hp_restored as u32;
        }

        // Each member must have gained HP (or been capped)
        assert!(members[0].hp.current > 5 || members[0].hp.current == members[0].hp.base);
        assert!(members[1].hp.current > 3 || members[1].hp.current == members[1].hp.base);
        assert!(members[2].hp.current > 10 || members[2].hp.current == members[2].hp.base);
        assert!(
            total_healed > 0,
            "At least one member should have been healed"
        );
    }

    // ── apply_buff_spell ─────────────────────────────────────────────────────

    /// apply_buff_spell writes the correct duration to ActiveSpells.bless.
    #[test]
    fn test_apply_buff_spell_bless() {
        let mut active = ActiveSpells::new();
        assert_eq!(active.bless, 0);

        let result = apply_buff_spell(BuffField::Bless, 30, &mut active);

        assert_eq!(active.bless, 30);
        assert_eq!(result.buff_field, BuffField::Bless);
        assert_eq!(result.duration_set, 30);
    }

    /// apply_buff_spell writes the correct duration to ActiveSpells.light.
    #[test]
    fn test_apply_buff_spell_light_sets_active_spells_light() {
        let mut active = ActiveSpells::new();
        let result = apply_buff_spell(BuffField::Light, 60, &mut active);

        assert_eq!(active.light, 60, "light field should be set to 60");
        assert_eq!(result.buff_field, BuffField::Light);
        assert_eq!(result.duration_set, 60);
    }

    /// apply_buff_spell covers all ActiveSpells protection fields.
    #[test]
    fn test_apply_buff_spell_all_protection_fields() {
        type FieldGetter = fn(&ActiveSpells) -> u8;
        let cases: &[(BuffField, FieldGetter)] = &[
            (BuffField::FearProtection, |a| a.fear_protection),
            (BuffField::ColdProtection, |a| a.cold_protection),
            (BuffField::FireProtection, |a| a.fire_protection),
            (BuffField::PoisonProtection, |a| a.poison_protection),
            (BuffField::AcidProtection, |a| a.acid_protection),
            (BuffField::ElectricityProtection, |a| {
                a.electricity_protection
            }),
            (BuffField::MagicProtection, |a| a.magic_protection),
            (BuffField::LeatherSkin, |a| a.leather_skin),
            (BuffField::Levitate, |a| a.levitate),
            (BuffField::WalkOnWater, |a| a.walk_on_water),
            (BuffField::GuardDog, |a| a.guard_dog),
            (BuffField::PsychicProtection, |a| a.psychic_protection),
            (BuffField::Invisibility, |a| a.invisibility),
            (BuffField::Shield, |a| a.shield),
            (BuffField::PowerShield, |a| a.power_shield),
            (BuffField::Cursed, |a| a.cursed),
        ];

        let duration = 20u8;
        for (field, getter) in cases {
            let mut active = ActiveSpells::new();
            let result = apply_buff_spell(*field, duration, &mut active);
            assert_eq!(
                getter(&active),
                duration,
                "Field {:?} should be set to {}",
                field,
                duration
            );
            assert_eq!(result.duration_set, duration);
        }
    }

    /// Setting duration to 0 effectively removes the buff.
    #[test]
    fn test_apply_buff_spell_zero_duration_clears_buff() {
        let mut active = ActiveSpells::new();
        active.bless = 15;

        apply_buff_spell(BuffField::Bless, 0, &mut active);
        assert_eq!(active.bless, 0);
    }

    // ── apply_cure_condition ─────────────────────────────────────────────────

    /// Curing paralysis clears active_conditions and the PARALYZED bitflag.
    #[test]
    fn test_apply_cure_condition_paralysis() {
        let mut character = make_character("Frozen", 20, 20);
        character.conditions.add(Condition::PARALYZED);
        character.add_condition(ActiveCondition::new(
            "paralyzed".to_string(),
            ConditionDuration::Rounds(5),
        ));

        let result = apply_cure_condition("paralyzed", &mut character);

        assert!(result.was_present);
        assert_eq!(result.condition_id, "paralyzed");
        assert!(
            !character.conditions.has(Condition::PARALYZED),
            "PARALYZED bitflag must be cleared"
        );
        assert!(
            character.active_conditions.is_empty(),
            "active_conditions must be empty after cure"
        );
    }

    /// Curing poison clears active_conditions and the POISONED bitflag.
    #[test]
    fn test_apply_cure_condition_poison() {
        let mut character = make_character("Venom", 20, 20);
        character.conditions.add(Condition::POISONED);
        character.add_condition(ActiveCondition::new(
            "poisoned".to_string(),
            ConditionDuration::Permanent,
        ));

        let result = apply_cure_condition("poisoned", &mut character);

        assert!(result.was_present);
        assert!(
            !character.conditions.has(Condition::POISONED),
            "POISONED bitflag must be cleared"
        );
        assert!(character.active_conditions.is_empty());
    }

    /// Curing blindness clears active_conditions and the BLINDED bitflag.
    #[test]
    fn test_apply_cure_condition_blindness() {
        let mut character = make_character("Sight", 20, 20);
        character.conditions.add(Condition::BLINDED);
        character.add_condition(ActiveCondition::new(
            "blinded".to_string(),
            ConditionDuration::Rounds(10),
        ));

        let result = apply_cure_condition("blinded", &mut character);

        assert!(result.was_present);
        assert!(!character.conditions.has(Condition::BLINDED));
        assert!(character.active_conditions.is_empty());
    }

    /// Curing a condition that is not present is a safe no-op.
    #[test]
    fn test_apply_cure_condition_not_present_is_noop() {
        let mut character = make_character("Fine", 20, 20);
        // No conditions applied

        let result = apply_cure_condition("paralyzed", &mut character);

        assert!(!result.was_present);
        assert!(character.conditions.is_fine());
    }

    // ── apply_utility_spell ──────────────────────────────────────────────────

    /// Create Food produces the correct number of food units.
    #[test]
    fn test_apply_utility_spell_create_food() {
        let result = apply_utility_spell(UtilityType::CreateFood { amount: 6 });

        assert_eq!(
            result.food_created, 6,
            "Create Food should report 6 ration units"
        );
        assert!(result.message.contains("6"));
    }

    /// Teleport utility returns zero food and a non-empty message.
    #[test]
    fn test_apply_utility_spell_teleport() {
        let result = apply_utility_spell(UtilityType::Teleport {
            destination: TeleportDestination::Surface,
        });

        assert_eq!(result.food_created, 0);
        assert!(!result.message.is_empty());
        assert_eq!(
            result.teleport_destination,
            Some(TeleportDestination::Surface)
        );
    }

    #[test]
    fn test_apply_utility_spell_teleport_town_portal() {
        let result = apply_utility_spell(UtilityType::Teleport {
            destination: TeleportDestination::TownPortal,
        });

        assert_eq!(
            result.teleport_destination,
            Some(TeleportDestination::TownPortal)
        );
        assert!(!result.message.is_empty());
    }

    #[test]
    fn test_apply_utility_spell_teleport_jump() {
        let result = apply_utility_spell(UtilityType::Teleport {
            destination: TeleportDestination::Jump,
        });

        assert_eq!(result.teleport_destination, Some(TeleportDestination::Jump));
    }

    #[test]
    fn test_apply_utility_spell_create_food_no_teleport_destination() {
        let result = apply_utility_spell(UtilityType::CreateFood { amount: 3 });
        assert!(result.teleport_destination.is_none());
    }

    #[test]
    fn test_apply_utility_spell_information_no_teleport_destination() {
        let result = apply_utility_spell(UtilityType::Information);
        assert!(result.teleport_destination.is_none());
    }

    /// Information utility returns zero food and a non-empty message.
    #[test]
    fn test_apply_utility_spell_information() {
        let result = apply_utility_spell(UtilityType::Information);

        assert_eq!(result.food_created, 0);
        assert!(!result.message.is_empty());
    }

    // ── apply_spell_effect — Healing ──────────────────────────────────────────

    /// Dispatching a Healing spell restores target HP and reports correct result.
    #[test]
    fn test_apply_spell_effect_healing_single_target() {
        let mut character = make_character("Hero", 20, 5);
        let spell = make_heal_spell(DiceRoll::new(2, 4, 0));
        let mut active = ActiveSpells::new();

        let result =
            apply_spell_effect(&spell, Some(&mut character), &mut active, &mut rand::rng());

        assert!(result.success);
        assert!(result.total_hp_healed > 0);
        assert!(character.hp.current > 5);
        assert!(character.hp.current <= 20);
    }

    /// Dispatching a Healing spell with no target returns a no-op result.
    #[test]
    fn test_apply_spell_effect_healing_no_target_noop() {
        let spell = make_heal_spell(DiceRoll::new(1, 6, 0));
        let mut active = ActiveSpells::new();

        let result = apply_spell_effect(&spell, None, &mut active, &mut rand::rng());

        assert!(result.success);
        assert_eq!(result.total_hp_healed, 0);
    }

    // ── apply_spell_effect — Buff ─────────────────────────────────────────────

    /// Dispatching a Buff spell writes the correct field on active_spells.
    #[test]
    fn test_apply_spell_effect_buff_bless() {
        let spell = make_buff_spell(BuffField::Bless, 30);
        let mut active = ActiveSpells::new();

        let result = apply_spell_effect(&spell, None, &mut active, &mut rand::rng());

        assert!(result.success);
        assert!(result.buff_applied.is_some());
        assert_eq!(active.bless, 30);
    }

    /// Buff spell sets active_spells.light correctly.
    #[test]
    fn test_apply_spell_effect_buff_light() {
        let spell = make_buff_spell(BuffField::Light, 60);
        let mut active = ActiveSpells::new();

        let result = apply_spell_effect(&spell, None, &mut active, &mut rand::rng());

        assert!(result.success);
        assert_eq!(active.light, 60);
    }

    // ── apply_spell_effect — Cure Condition ───────────────────────────────────

    /// Dispatching a CureCondition spell removes the condition from the target.
    #[test]
    fn test_apply_spell_effect_cure_condition_removes_condition() {
        let mut character = make_character("Victim", 20, 20);
        character.conditions.add(Condition::SILENCED);
        character.add_condition(ActiveCondition::new(
            "silenced".to_string(),
            ConditionDuration::Rounds(3),
        ));

        let spell = make_cure_spell("silenced");
        let mut active = ActiveSpells::new();

        let result =
            apply_spell_effect(&spell, Some(&mut character), &mut active, &mut rand::rng());

        assert!(result.success);
        let cure = result
            .condition_cured
            .as_ref()
            .expect("cure result expected");
        assert!(cure.was_present);
        assert!(!character.conditions.is_silenced());
        assert!(character.active_conditions.is_empty());
    }

    /// CureCondition is a no-op when the condition is not present.
    #[test]
    fn test_apply_spell_effect_cure_condition_noop_when_absent() {
        let mut character = make_character("Fine", 20, 20);
        let spell = make_cure_spell("paralyzed");
        let mut active = ActiveSpells::new();

        let result =
            apply_spell_effect(&spell, Some(&mut character), &mut active, &mut rand::rng());

        assert!(result.success);
        let cure = result
            .condition_cured
            .as_ref()
            .expect("cure result expected");
        assert!(!cure.was_present);
    }

    // ── apply_spell_effect — Utility ──────────────────────────────────────────

    /// Dispatching Create Food reports the correct food amount.
    #[test]
    fn test_apply_spell_effect_utility_create_food() {
        let spell = make_utility_spell(UtilityType::CreateFood { amount: 5 });
        let mut active = ActiveSpells::new();

        let result = apply_spell_effect(&spell, None, &mut active, &mut rand::rng());

        assert!(result.success);
        assert_eq!(result.food_created, 5);
    }

    // ── apply_spell_effect — Composite ───────────────────────────────────────

    /// Composite: heal AND cure condition both apply to the target.
    #[test]
    fn test_apply_spell_effect_composite_heal_and_cure() {
        let mut character = make_character("Wounded", 20, 5);
        character.conditions.add(Condition::POISONED);
        character.add_condition(ActiveCondition::new(
            "poisoned".to_string(),
            ConditionDuration::Permanent,
        ));

        let mut composite_spell = Spell::new(
            0x0110,
            "Cure Wounds + Cure Poison",
            SpellSchool::Cleric,
            3,
            5,
            0,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Heals and cures poison",
            None,
            0,
            false,
        );
        composite_spell.effect_type = Some(SpellEffectType::Composite(vec![
            SpellEffectType::Healing {
                amount: DiceRoll::new(2, 4, 0),
            },
            SpellEffectType::CureCondition {
                condition_id: "poisoned".to_string(),
            },
        ]));

        let mut active = ActiveSpells::new();
        let result = apply_spell_effect(
            &composite_spell,
            Some(&mut character),
            &mut active,
            &mut rand::rng(),
        );

        assert!(result.success);
        // HP should have increased
        assert!(result.total_hp_healed > 0, "composite should have healed");
        assert!(character.hp.current > 5);
        // Poison should be cured
        assert!(
            result.condition_cured.is_some(),
            "composite should have cured condition"
        );
        assert!(!character.conditions.has(Condition::POISONED));
        assert!(character.active_conditions.is_empty());
    }

    /// Composite: buff AND create food both apply.
    #[test]
    fn test_apply_spell_effect_composite_buff_and_food() {
        let mut composite_spell = Spell::new(
            0x0111,
            "Holy Feast",
            SpellSchool::Cleric,
            4,
            6,
            0,
            SpellContext::NonCombatOnly,
            SpellTarget::AllCharacters,
            "Blesses and creates food",
            None,
            0,
            false,
        );
        composite_spell.effect_type = Some(SpellEffectType::Composite(vec![
            SpellEffectType::Buff {
                buff_field: BuffField::Bless,
                duration: 20,
            },
            SpellEffectType::Utility {
                utility_type: UtilityType::CreateFood { amount: 3 },
            },
        ]));

        let mut active = ActiveSpells::new();
        let result = apply_spell_effect(&composite_spell, None, &mut active, &mut rand::rng());

        assert!(result.success);
        assert_eq!(active.bless, 20);
        assert_eq!(result.food_created, 3);
        assert!(result.buff_applied.is_some());
    }

    // ── apply_spell_effect — Damage / Debuff / Resurrection (no-op paths) ────

    /// Damage spells return a no-op from the dispatcher (handled by combat engine).
    #[test]
    fn test_apply_spell_effect_damage_returns_noop() {
        let mut damage_spell = Spell::new(
            0x0201,
            "Fireball",
            SpellSchool::Sorcerer,
            3,
            5,
            0,
            SpellContext::CombatOnly,
            SpellTarget::AllMonsters,
            "3d6 fire damage",
            Some(DiceRoll::new(3, 6, 0)),
            0,
            true,
        );
        // Explicitly set Damage (also inferred from damage field)
        damage_spell.effect_type = Some(SpellEffectType::Damage);

        let mut active = ActiveSpells::new();
        let result = apply_spell_effect(&damage_spell, None, &mut active, &mut rand::rng());

        assert!(result.success);
        assert_eq!(result.total_hp_healed, 0);
        assert!(result.buff_applied.is_none());
    }

    // ── infer_effect_type ─────────────────────────────────────────────────────

    /// Spells with damage dice infer Damage.
    #[test]
    fn test_infer_effect_type_damage() {
        let spell = Spell::new(
            0x0201,
            "Flame Arrow",
            SpellSchool::Sorcerer,
            1,
            1,
            0,
            SpellContext::CombatOnly,
            SpellTarget::SingleMonster,
            "1d6 fire",
            Some(DiceRoll::new(1, 6, 0)),
            0,
            false,
        );
        assert_eq!(spell.infer_effect_type(), SpellEffectType::Damage);
    }

    /// Spells with resurrect_hp infer Resurrection.
    #[test]
    fn test_infer_effect_type_resurrection() {
        let mut spell = Spell::new(
            0x0105,
            "Resurrect",
            SpellSchool::Cleric,
            5,
            15,
            5,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Revives dead",
            None,
            0,
            false,
        );
        spell.resurrect_hp = Some(1);
        assert_eq!(spell.infer_effect_type(), SpellEffectType::Resurrection);
    }

    /// Spells with applied_conditions but no damage infer Debuff.
    #[test]
    fn test_infer_effect_type_debuff() {
        let mut spell = Spell::new(
            0x0202,
            "Sleep",
            SpellSchool::Sorcerer,
            1,
            2,
            0,
            SpellContext::CombatOnly,
            SpellTarget::AllMonsters,
            "Puts enemies to sleep",
            None,
            0,
            false,
        );
        spell.applied_conditions = vec!["asleep".to_string()];
        assert_eq!(spell.infer_effect_type(), SpellEffectType::Debuff);
    }

    /// Spells with no damage/conditions/resurrection infer Utility(Information).
    #[test]
    fn test_infer_effect_type_defaults_to_utility_information() {
        let spell = Spell::new(
            0x0106,
            "Unknown",
            SpellSchool::Cleric,
            1,
            1,
            0,
            SpellContext::Anytime,
            SpellTarget::Self_,
            "Does something",
            None,
            0,
            false,
        );
        assert_eq!(
            spell.infer_effect_type(),
            SpellEffectType::Utility {
                utility_type: UtilityType::Information,
            }
        );
    }

    /// effective_effect_type returns explicit type when set.
    #[test]
    fn test_effective_effect_type_explicit_overrides_inference() {
        let mut spell = Spell::new(
            0x0101,
            "Cure Wounds",
            SpellSchool::Cleric,
            1,
            2,
            0,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Heals HP",
            None,
            0,
            false,
        );
        // Without explicit type, inference returns Utility(Information)
        assert_eq!(
            spell.infer_effect_type(),
            SpellEffectType::Utility {
                utility_type: UtilityType::Information,
            }
        );

        // Set explicit type
        spell.effect_type = Some(SpellEffectType::Healing {
            amount: DiceRoll::new(1, 8, 0),
        });
        assert!(matches!(
            spell.effective_effect_type(),
            SpellEffectType::Healing { .. }
        ));
    }
}
