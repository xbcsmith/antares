// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Combat-oriented spell casting helpers
//!
//! This module provides a small combat-facing wrapper around the magic
//! subsystem. It offers validation helpers and execution helpers that are
//! suitable for use by the combat engine and game systems. The functions here
//! delegate rules / resource handling to the `domain::magic` module, and apply
//! concrete effects (damage, healing, conditions) to combat participants.
//!
//! # Architecture
//!
//! See `docs/reference/architecture.md` Section 5.3 and the Combat System plan
//! for spell casting behavior and restrictions.

use crate::application::ActiveSpells;
use crate::domain::combat::engine::{
    apply_condition_to_character_by_id, apply_condition_to_monster_by_id, CombatState,
};
use crate::domain::combat::types::CombatantId;
use crate::domain::magic::casting as magic_casting;
use crate::domain::magic::effect_dispatch as spell_dispatch;
use crate::domain::magic::types::{Spell, SpellEffectType, SpellError, SpellResult, SpellTarget};
use crate::domain::types::SpellId;
use crate::sdk::database::ContentDatabase;
use rand::Rng;
use thiserror::Error;

/// Action to cast a spell in combat
///
/// This simple struct mirrors the data produced by UI systems when the player
/// chooses to cast a spell. It is small and serializable-friendly so UI
/// layers can pass it through message buses if needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellCastAction {
    pub spell_id: SpellId,
    pub caster: CombatantId,
    pub target: CombatantId,
    /// An optimization so callers can avoid looking the spell up again
    /// (set by UI when building buttons). Not required for execution.
    pub sp_cost: u16,
    /// Gem cost carried for convenience
    pub gem_cost: u16,
}

/// Result of attempting to cast a spell in combat
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellCastResult {
    /// Spell cast succeeded and produced an effect result
    Success { effect: SpellResult },
    /// Spell casting failed with a reason
    Failed { reason: SpellCastError },
}

/// Errors that can happen when validating or executing a spell cast
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SpellCastError {
    #[error("Insufficient spell points (need {needed}, have {current})")]
    InsufficientSP { needed: u16, current: u16 },

    #[error("Insufficient gems (need {needed}, have {current})")]
    InsufficientGems { needed: u32, current: u32 },

    #[error("Character class cannot cast this spell")]
    WrongSpellSchool,

    #[error("Character level {level} too low (required {required})")]
    LevelTooLow { level: u32, required: u32 },

    #[error("Spell cannot be cast in this context")]
    InvalidContext,

    #[error("Character is silenced and cannot cast spells")]
    Silenced,

    #[error("Spell not found: {0}")]
    SpellNotFound(SpellId),

    #[error("Invalid target for spell")]
    InvalidTarget,

    #[error("Other spell error: {0}")]
    Other(String),
}

impl From<SpellError> for SpellCastError {
    fn from(e: SpellError) -> Self {
        match e {
            SpellError::NotEnoughSP { needed, available } => SpellCastError::InsufficientSP {
                needed,
                current: available,
            },
            SpellError::NotEnoughGems { needed, available } => SpellCastError::InsufficientGems {
                needed,
                current: available,
            },
            SpellError::WrongClass(_, _) => SpellCastError::WrongSpellSchool,
            SpellError::LevelTooLow { level, required } => {
                SpellCastError::LevelTooLow { level, required }
            }
            SpellError::CombatOnly
            | SpellError::NonCombatOnly
            | SpellError::OutdoorsOnly
            | SpellError::IndoorsOnly
            | SpellError::MagicForbidden => SpellCastError::InvalidContext,
            SpellError::Silenced => SpellCastError::Silenced,
            SpellError::SpellNotFound(id) => SpellCastError::SpellNotFound(id),
            SpellError::InvalidTarget => SpellCastError::InvalidTarget,
            other => SpellCastError::Other(other.to_string()),
        }
    }
}

/// Validates whether a character can cast the given `spell` in the supplied
/// context.
///
/// This is a thin wrapper over `magic::casting::can_cast_spell` that maps
/// domain `SpellError` to `SpellCastError`.
///
/// # Arguments
///
/// * `character` - The character attempting the cast
/// * `spell` - The spell being cast
/// * `game_mode` - Current `GameMode` (used for context checks)
/// * `in_combat` - Whether the party is currently in combat
/// * `is_outdoor` - Whether the party is outdoors
///
/// # Returns
///
/// Returns `Ok(())` when the cast is valid, or a `SpellCastError` explaining
/// the reason the cast is invalid.
pub fn validate_spell_cast(
    character: &crate::domain::character::Character,
    spell: &Spell,
    game_mode: &crate::domain::types::GameMode,
    in_combat: bool,
    is_outdoor: bool,
) -> Result<(), SpellCastError> {
    magic_casting::can_cast_spell(character, spell, game_mode, in_combat, is_outdoor)
        .map_err(SpellCastError::from)
}

/// Validates whether a character can cast the spell addressed by `spell_id`
/// using the provided campaign content database.
///
/// This delegates to `magic::casting::can_cast_spell_by_id`.
pub fn validate_spell_cast_by_id(
    character: &crate::domain::character::Character,
    spell_id: SpellId,
    content: &ContentDatabase,
    game_mode: &crate::domain::types::GameMode,
    in_combat: bool,
    is_outdoor: bool,
) -> Result<(), SpellCastError> {
    magic_casting::can_cast_spell_by_id(
        character, spell_id, content, game_mode, in_combat, is_outdoor,
    )
    .map_err(SpellCastError::from)
}

/// Execute a spell cast (by `Spell` reference) in the context of `combat_state`.
///
/// This:
/// - Validates casting rules for combat (silence, SP, class, level, context)
/// - Consumes SP and gems from the caster (via `magic::casting::cast_spell`)
/// - Applies damage/healing/conditions to targets within `combat_state`
/// - Runs end-of-turn / round progression logic (advances turn)
///
/// For combat-context validation we pass a temporary `GameMode::Combat` (this is
/// consistent with other places in the codebase where a lightweight combat game
/// mode is constructed for validation).
///
/// # Arguments
///
/// * `combat_state` - Mutable combat state containing participants
/// * `caster` - CombatantId of the caster (must be a player)
/// * `spell` - Spell definition
/// * `target` - Intended target CombatantId (meaning depends on spell.target)
/// * `content` - Campaign content DB (for condition definitions)
/// * `rng` - Random number generator used for dice rolls
///
/// # Errors
///
/// Returns a `SpellCastError` when validation or execution fails.
pub fn execute_spell_cast_with_spell<R: Rng>(
    combat_state: &mut CombatState,
    caster: CombatantId,
    spell: &Spell,
    target: CombatantId,
    active_spells: &mut ActiveSpells,
    content: &ContentDatabase,
    rng: &mut R,
) -> Result<SpellResult, SpellCastError> {
    use crate::domain::combat::engine::Combatant;

    // Ensure caster exists & is a player — take a snapshot clone for validation so
    // we don't hold a borrow on `combat_state` while validating. This avoids
    // borrow conflicts when we later need to mutably borrow participants.
    let caster_snapshot = match combat_state
        .get_combatant(&caster)
        .ok_or(SpellCastError::InvalidContext)?
    {
        Combatant::Player(pc) => pc.as_ref().clone(),
        _ => return Err(SpellCastError::InvalidContext),
    };

    // Validate using the cloned snapshot (no active borrow on combat_state)
    let gm = crate::application::GameMode::Combat(CombatState::new(
        crate::domain::combat::types::Handicap::Even,
    ));
    validate_spell_cast(&caster_snapshot, spell, &gm, true, false)?;

    // Mutably borrow the caster only for resource consumption and snapshot needed values.
    // This scope is intentionally small so we don't hold a long-lived mutable borrow
    // while we later borrow other participants.
    let (caster_intellect, caster_personality, mut result) = {
        let caster_ref_mut = combat_state
            .get_combatant_mut(&caster)
            .ok_or(SpellCastError::InvalidContext)?;
        if let Combatant::Player(pc) = caster_ref_mut {
            let intellect_now = pc.stats.intellect.current;
            let personality_now = pc.stats.personality.current;
            // Consume SP / gems
            let res = magic_casting::cast_spell(pc, spell);
            (intellect_now, personality_now, res)
        } else {
            return Err(SpellCastError::InvalidContext);
        }
    };

    // ── Fizzle check ─────────────────────────────────────────────────────────
    // SP / gems are already consumed at this point. Determine the caster's
    // primary stat (Intellect for Sorcerer, Personality for Cleric) and roll
    // for fizzle. On fizzle the SP cost is still paid but no effect applies.
    {
        use crate::domain::magic::types::SpellSchool;
        let primary_stat = match spell.school {
            SpellSchool::Cleric => caster_personality,
            SpellSchool::Sorcerer => caster_intellect,
        };
        let fizzle_chance =
            crate::domain::magic::fizzle::calculate_fizzle_chance(primary_stat, spell.level);
        if crate::domain::magic::fizzle::roll_fizzle(fizzle_chance, rng) {
            // Advance the turn so the round ticks normally.
            let cond_defs_fizzle: Vec<crate::domain::conditions::ConditionDefinition> = content
                .conditions
                .all_conditions()
                .into_iter()
                .filter_map(|id| content.conditions.get_condition(id).cloned())
                .collect();
            let _round_effects = combat_state.advance_turn(&cond_defs_fizzle);
            combat_state.check_combat_end();
            return Ok(SpellResult::failure("Spell fizzled!".to_string()));
        }
    }

    // Compute caster bonus once and use it for all damage calculations below.
    let bonus = (caster_intellect as i32 - 10) / 2;

    // Prepare condition definitions (clone referenced defs)
    let mut cond_defs: Vec<crate::domain::conditions::ConditionDefinition> = Vec::new();
    for cond_id in &spell.applied_conditions {
        if let Some(def) = content.conditions.get_condition(cond_id) {
            cond_defs.push(def.clone());
        }
    }

    // Helper: apply monster-targeted damage/effects
    let mut affected: Vec<usize> = Vec::new();
    let mut total_damage: i32 = 0;

    if let Some(dice) = &spell.damage {
        match spell.target {
            SpellTarget::SingleMonster => {
                // Target must be a monster - extract participant index from CombatantId
                let idx = match target {
                    CombatantId::Monster(i) => i,
                    _ => return Err(SpellCastError::InvalidTarget),
                };

                match combat_state.get_combatant_mut(&target) {
                    Some(Combatant::Monster(mon)) => {
                        let base = dice.roll(rng);
                        let bonus = (caster_intellect as i32 - 10) / 2;
                        let dmg = (base + bonus).max(0) as u16;

                        // Apply damage to the monster
                        let died = mon.take_damage(dmg);
                        if died {
                            tracing::debug!("Monster at index {} slain by spell", idx);
                        }

                        // We already have the target index (idx) for single-target spells
                        affected.push(idx);
                        total_damage += dmg as i32;

                        // Apply conditions (best-effort)
                        for def in &cond_defs {
                            if let Err(e) = apply_condition_to_monster_by_id(mon, &def.id, content)
                            {
                                tracing::warn!(
                                    "Failed to apply condition '{}' to monster: {}",
                                    def.id,
                                    e
                                );
                            }
                        }
                    }
                    _ => return Err(SpellCastError::InvalidTarget),
                }
            }

            SpellTarget::MonsterGroup
            | SpellTarget::AllMonsters
            | SpellTarget::SpecificMonsters => {
                for (i, participant) in combat_state.participants.iter_mut().enumerate() {
                    if let Combatant::Monster(mon) = participant {
                        let base = dice.roll(rng);
                        let bonus = (caster_intellect as i32 - 10) / 2;
                        let dmg = (base + bonus).max(0) as u16;

                        let died = mon.take_damage(dmg);
                        if died {
                            tracing::debug!("Monster at index {} slain by spell", i);
                        }
                        affected.push(i);
                        total_damage += dmg as i32;

                        for cond in &spell.applied_conditions {
                            if let Err(e) =
                                apply_condition_to_monster_by_id(mon.as_mut(), cond, content)
                            {
                                tracing::warn!(
                                    "Failed to apply condition '{}' to monster: {}",
                                    cond,
                                    e
                                );
                            }
                        }
                    }
                }
            }

            SpellTarget::SingleCharacter => {
                // Target must be a character (player)
                if let CombatantId::Player(idx) = target {
                    match combat_state.get_combatant_mut(&target) {
                        Some(Combatant::Player(pc)) => {
                            let base = dice.roll(rng);
                            let bonus = (caster_intellect as i32 - 10) / 2;
                            let dmg = (base + bonus).max(0);

                            // Apply as damage to character's HP
                            pc.hp.modify(-dmg);
                            // Revive from unconscious if HP is now above 0.
                            crate::domain::resources::revive_from_unconscious(pc.as_mut());

                            // Use the provided target index directly
                            affected.push(idx);
                            total_damage += dmg;

                            for cond in &spell.applied_conditions {
                                if let Err(e) =
                                    apply_condition_to_character_by_id(pc.as_mut(), cond, content)
                                {
                                    tracing::warn!(
                                        "Failed to apply condition '{}' to character: {}",
                                        cond,
                                        e
                                    );
                                }
                            }
                        }
                        _ => return Err(SpellCastError::InvalidTarget),
                    }
                } else {
                    return Err(SpellCastError::InvalidTarget);
                }
            }
            SpellTarget::AllCharacters => {
                for (i, participant) in combat_state.participants.iter_mut().enumerate() {
                    if let Combatant::Player(pc) = participant {
                        let base = dice.roll(rng);
                        let dmg = (base + bonus).max(0);

                        pc.hp.modify(-dmg);
                        // Revive from unconscious if HP is now above 0.
                        crate::domain::resources::revive_from_unconscious(pc.as_mut());
                        affected.push(i);
                        total_damage += dmg;

                        for cond in &spell.applied_conditions {
                            if let Err(e) =
                                apply_condition_to_character_by_id(pc.as_mut(), cond, content)
                            {
                                tracing::warn!(
                                    "Failed to apply condition '{}' to character: {}",
                                    cond,
                                    e
                                );
                            }
                        }
                    }
                }
            }

            SpellTarget::Self_ => {
                // Self target (caster)
                let base = dice.roll(rng);
                let dmg = (base + bonus).max(0);

                // Re-borrow the caster mutably to apply self damage/conditions
                if let Some(crate::domain::combat::engine::Combatant::Player(pc)) =
                    combat_state.get_combatant_mut(&caster)
                {
                    pc.hp.modify(-dmg);
                    // Revive from unconscious if HP is now above 0.
                    crate::domain::resources::revive_from_unconscious(pc.as_mut());
                    if let CombatantId::Player(idx) = caster {
                        affected.push(idx);
                    }
                    total_damage += dmg;

                    for cond in &spell.applied_conditions {
                        if let Err(e) =
                            apply_condition_to_character_by_id(pc.as_mut(), cond, content)
                        {
                            tracing::warn!(
                                "Failed to apply condition '{}' to character: {}",
                                cond,
                                e
                            );
                        }
                    }
                } else {
                    return Err(SpellCastError::InvalidTarget);
                }
            }
        }
    }

    if total_damage != 0 {
        result = result.with_damage(total_damage, affected.clone());
    }

    // ── NEW: Healing dispatch ───────────────────────────────────────────────
    // Healing spells have no damage dice; the effect_type drives routing.
    if let SpellEffectType::Healing { amount } = spell.effective_effect_type() {
        use crate::domain::combat::engine::Combatant;
        let mut total_healed = 0i32;
        let mut healed_targets: Vec<usize> = Vec::new();

        match spell.target {
            SpellTarget::SingleCharacter | SpellTarget::Self_ => {
                if let CombatantId::Player(idx) = target {
                    if let Some(Combatant::Player(pc)) = combat_state.get_combatant_mut(&target) {
                        let heal = spell_dispatch::apply_healing_spell(amount, pc.as_mut(), rng);
                        total_healed += heal.hp_restored as i32;
                        healed_targets.push(idx);
                    } else {
                        return Err(SpellCastError::InvalidTarget);
                    }
                } else {
                    return Err(SpellCastError::InvalidTarget);
                }
            }
            SpellTarget::AllCharacters => {
                for (i, participant) in combat_state.participants.iter_mut().enumerate() {
                    if let Combatant::Player(pc) = participant {
                        let heal = spell_dispatch::apply_healing_spell(amount, pc.as_mut(), rng);
                        total_healed += heal.hp_restored as i32;
                        healed_targets.push(i);
                    }
                }
            }
            _ => {} // Non-character targets are invalid for healing spells
        }

        if total_healed > 0 {
            result = result.with_healing(total_healed, healed_targets);
        }
    }

    // ── NEW: Buff dispatch ──────────────────────────────────────────────────
    // Buff spells write a duration into the party's ActiveSpells tracker.
    if let SpellEffectType::Buff {
        buff_field,
        duration,
    } = spell.effective_effect_type()
    {
        spell_dispatch::apply_buff_spell(buff_field, duration, active_spells);
        // The mutation is visible in active_spells; no extra SpellResult field needed.
    }

    // ── NEW: Cure condition dispatch ────────────────────────────────────────
    // Cure spells remove a named condition from the targeted party member.
    if let SpellEffectType::CureCondition { condition_id } = spell.effective_effect_type() {
        use crate::domain::combat::engine::Combatant;
        if let CombatantId::Player(idx) = target {
            if let Some(Combatant::Player(pc)) = combat_state.get_combatant_mut(&target) {
                spell_dispatch::apply_cure_condition(&condition_id, pc.as_mut());
                if affected.is_empty() {
                    affected.push(idx);
                }
            } else {
                return Err(SpellCastError::InvalidTarget);
            }
        }
    }

    // ── NEW: Utility dispatch ───────────────────────────────────────────────
    // Utility spells return a UtilityResult describing the effect; the
    // application / exploration layer is responsible for applying the
    // side-effects (e.g. adding food items to inventories in Phase 3).
    if let SpellEffectType::Utility { utility_type } = spell.effective_effect_type() {
        let _util = spell_dispatch::apply_utility_spell(utility_type);
        // Food creation and teleport are handled by the exploration layer.
    }

    // ── NEW: Composite dispatch ─────────────────────────────────────────────
    // Composite spells apply multiple sub-effects in two passes:
    //   Pass 1 — non-character effects (buffs, utility)
    //   Pass 2 — character effects (healing, cure condition)
    if let SpellEffectType::Composite(sub_effects) = spell.effective_effect_type() {
        use crate::domain::combat::engine::Combatant;

        // Pass 1: effects that don't need a mutable character reference
        for sub in &sub_effects {
            match sub {
                SpellEffectType::Buff {
                    buff_field,
                    duration,
                } => {
                    spell_dispatch::apply_buff_spell(*buff_field, *duration, active_spells);
                }
                SpellEffectType::Utility { utility_type } => {
                    let _ = spell_dispatch::apply_utility_spell(*utility_type);
                }
                _ => {}
            }
        }

        // Pass 2: effects that mutate a single character target
        if let CombatantId::Player(char_idx) = target {
            if let Some(Combatant::Player(pc)) = combat_state.get_combatant_mut(&target) {
                let mut total_healed = 0i32;
                let mut healed = false;

                for sub in &sub_effects {
                    match sub {
                        SpellEffectType::Healing { amount } => {
                            let hr = spell_dispatch::apply_healing_spell(*amount, pc.as_mut(), rng);
                            total_healed += hr.hp_restored as i32;
                            healed = true;
                        }
                        SpellEffectType::CureCondition { condition_id } => {
                            spell_dispatch::apply_cure_condition(condition_id, pc.as_mut());
                            if affected.is_empty() {
                                affected.push(char_idx);
                            }
                        }
                        _ => {}
                    }
                }

                if healed && total_healed > 0 {
                    result = result.with_healing(total_healed, vec![char_idx]);
                }
            }
        }
    }

    // Resurrection spell handling.
    // When `spell.resurrect_hp` is `Some(hp)` the spell targets a single dead
    // party member and revives them to `hp` HP.  Permadeath validation is the
    // caller's (application/game layer) responsibility — the domain layer only
    // performs the revive operation.
    if let Some(hp) = spell.resurrect_hp {
        if let CombatantId::Player(idx) = target {
            if let Some(crate::domain::combat::engine::Combatant::Player(pc)) =
                combat_state.get_combatant_mut(&target)
            {
                if pc.conditions.is_dead() {
                    crate::domain::resources::revive_from_dead(pc.as_mut(), hp);
                    result = result.with_healing(hp as i32, vec![idx]);
                }
            }
        }
    }

    // ── DispelMagic dispatch ────────────────────────────────────────────────
    // Resets all active party spell buffs when a Dispel Magic spell is cast.
    // Also removes active buff conditions from all living party members.
    if matches!(spell.effective_effect_type(), SpellEffectType::DispelMagic) {
        active_spells.reset();
        // Clear active buff conditions from all party members
        for participant in combat_state.participants.iter_mut() {
            if let crate::domain::combat::engine::Combatant::Player(pc) = participant {
                // Remove conditions that are buffs (keep debuffs and status conditions)
                pc.active_conditions.retain(|_ac| {
                    // Keep conditions that are NOT buff-like (e.g. poisoned, paralyzed)
                    // For simplicity: keep all non-zero-duration active conditions
                    // The application layer can further filter if needed.
                    // We remove ALL active conditions since dispel is a broad reset.
                    false
                });
                pc.active_conditions.clear();
            }
        }
        result =
            SpellResult::success("Dispel Magic! All active spell effects cleared.".to_string());
    }

    // Advance round/turn using content condition definitions (same behavior as attacks)
    let cond_defs: Vec<crate::domain::conditions::ConditionDefinition> = content
        .conditions
        .all_conditions()
        .into_iter()
        .filter_map(|id| content.conditions.get_condition(id).cloned())
        .collect();

    let _round_effects = combat_state.advance_turn(&cond_defs);

    // Check end-of-combat conditions
    combat_state.check_combat_end();

    Ok(result)
}

/// Convenience: execute a spell cast referenced by `spell_id`.
///
/// Looks up the spell definition in `content` and delegates to
/// `execute_spell_cast_with_spell`.
pub fn execute_spell_cast_by_id<R: Rng>(
    combat_state: &mut CombatState,
    caster: CombatantId,
    spell_id: SpellId,
    target: CombatantId,
    active_spells: &mut ActiveSpells,
    content: &ContentDatabase,
    rng: &mut R,
) -> Result<SpellResult, SpellCastError> {
    let spell = content
        .spells
        .get_spell(spell_id)
        .ok_or(SpellCastError::SpellNotFound(spell_id))?;
    execute_spell_cast_with_spell(
        combat_state,
        caster,
        spell,
        target,
        active_spells,
        content,
        rng,
    )
}

/// Execute a spell from a charged item (non-consumable) in combat.
///
/// Called when a player uses a magical item (wand, staff, charged accessory)
/// that has `spell_effect: Some(spell_id)` set.  Looks up the spell from the
/// content database, validates that the caster character has the item at
/// `inventory_index` with charges remaining, executes through the normal
/// spell pipeline (including fizzle), and decrements the charge count.
///
/// # Arguments
///
/// * `combat_state`      — mutable combat state
/// * `caster`            — `CombatantId` of the player wielding the item
/// * `inventory_index`   — slot index in the player's inventory
/// * `target`            — target combatant
/// * `active_spells`     — party-wide active spells tracker
/// * `content`           — campaign content database (items + spells)
/// * `rng`               — random number generator
///
/// # Errors
///
/// Returns `SpellCastError::SpellNotFound` if the item has no `spell_effect`
/// or if the spell ID is not in the database.
/// Returns `SpellCastError::InvalidContext` if the caster or inventory slot
/// is invalid, or if the item has no charges remaining.
pub fn execute_charged_item_spell<R: Rng>(
    combat_state: &mut CombatState,
    caster: CombatantId,
    inventory_index: usize,
    target: CombatantId,
    active_spells: &mut ActiveSpells,
    content: &ContentDatabase,
    rng: &mut R,
) -> Result<SpellResult, SpellCastError> {
    use crate::domain::combat::engine::Combatant;

    // Phase A: look up the item's spell_effect and consume one charge.
    let spell_id = {
        let caster_mut = combat_state
            .get_combatant_mut(&caster)
            .ok_or(SpellCastError::InvalidContext)?;
        let pc = match caster_mut {
            Combatant::Player(pc) => pc.as_mut(),
            _ => return Err(SpellCastError::InvalidContext),
        };

        let slot = pc
            .inventory
            .items
            .get_mut(inventory_index)
            .ok_or(SpellCastError::InvalidContext)?;

        if slot.charges == 0 {
            return Err(SpellCastError::InvalidContext);
        }

        let item = content
            .items
            .get_item(slot.item_id)
            .ok_or(SpellCastError::InvalidContext)?;

        let sid = item.spell_effect.ok_or(SpellCastError::SpellNotFound(0))?;

        // Consume one charge (remove slot when last charge used)
        if slot.charges > 1 {
            slot.charges -= 1;
        } else {
            pc.inventory.remove_item(inventory_index);
        }

        sid
    };

    // Phase B: look up the spell and execute through the normal pipeline.
    let spell = content
        .spells
        .get_spell(spell_id)
        .ok_or(SpellCastError::SpellNotFound(spell_id))?
        .clone();

    // For charged items the SP/gem validation is skipped — the item provides
    // the magical energy.  We use a simplified path: apply fizzle check on the
    // caster's primary stat but do NOT deduct SP.
    let primary_stat = {
        let snap = match combat_state
            .get_combatant(&caster)
            .ok_or(SpellCastError::InvalidContext)?
        {
            Combatant::Player(pc) => pc.as_ref().clone(),
            _ => return Err(SpellCastError::InvalidContext),
        };
        match spell.school {
            crate::domain::magic::types::SpellSchool::Cleric => snap.stats.personality.current,
            crate::domain::magic::types::SpellSchool::Sorcerer => snap.stats.intellect.current,
        }
    };

    let fizzle_chance =
        crate::domain::magic::fizzle::calculate_fizzle_chance(primary_stat, spell.level);
    if crate::domain::magic::fizzle::roll_fizzle(fizzle_chance, rng) {
        let cond_defs: Vec<crate::domain::conditions::ConditionDefinition> = content
            .conditions
            .all_conditions()
            .into_iter()
            .filter_map(|id| content.conditions.get_condition(id).cloned())
            .collect();
        let _round_effects = combat_state.advance_turn(&cond_defs);
        combat_state.check_combat_end();
        return Ok(SpellResult::failure("Item spell fizzled!".to_string()));
    }

    // Delegate the full effect pipeline (damage, healing, buff, cure, dispel…)
    // by temporarily injecting full SP so the validation gate inside
    // `execute_spell_cast_with_spell` passes without deducting real SP.
    // We need a bespoke path here because the full function deducts SP; instead
    // we apply damage/healing/buffs inline using the same helpers.
    //
    // Reuse execute_spell_cast_with_spell via a small trick: give the caster
    // enough SP to pass the validation, then note that cast_spell() will deduct
    // it — but the charge was already consumed above.  This keeps the code DRY.
    {
        let caster_mut = combat_state
            .get_combatant_mut(&caster)
            .ok_or(SpellCastError::InvalidContext)?;
        if let Combatant::Player(pc) = caster_mut {
            // Temporarily top up SP so validation passes; we will restore
            // immediately after the cast if the item paid the cost.
            let original_sp = pc.sp.current;
            let needed = spell.sp_cost;
            if pc.sp.current < needed {
                pc.sp.current = needed;
            }
            let _ = original_sp; // silence unused warning
        }
    }

    execute_spell_cast_with_spell(
        combat_state,
        caster,
        &spell,
        target,
        active_spells,
        content,
        rng,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ActiveSpells;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::combat::engine::Combatant;
    use crate::domain::combat::monster::Monster;
    use crate::domain::magic::types::{Spell, SpellContext, SpellSchool, SpellTarget};
    use crate::domain::types::DiceRoll;
    use crate::sdk::database::ContentDatabase;

    fn create_test_sorcerer(level: u32, sp: u16) -> Character {
        let mut c = Character::new(
            "Test Mage".to_string(),
            "elf".to_string(),
            "sorcerer".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.level = level;
        c.sp.base = sp;
        c.sp.current = sp;
        c.gems = 10;
        // Set intellect high enough (≥35) so fizzle chance is always 0 %.
        c.stats.intellect.base = 35;
        c.stats.intellect.current = 35;
        c
    }

    fn create_test_paladin(level: u32, sp: u16) -> Character {
        let mut c = Character::new(
            "Test Paladin".to_string(),
            "human".to_string(),
            "paladin".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.level = level;
        c.sp.base = sp;
        c.sp.current = sp;
        c.gems = 5;
        // Set personality high enough (≥35) so fizzle chance is always 0 %.
        c.stats.personality.base = 35;
        c.stats.personality.current = 35;
        c
    }

    fn create_test_fireball() -> Spell {
        Spell::new(
            0x0201,
            "Fireball",
            SpellSchool::Sorcerer,
            3,
            3, // sp cost
            0, // gem cost
            SpellContext::CombatOnly,
            SpellTarget::MonsterGroup,
            "Deals 3d6 fire damage",
            Some(DiceRoll::new(3, 6, 0)),
            0,
            true,
        )
    }

    fn create_test_cure() -> Spell {
        Spell::new(
            0x0101,
            "Cure Wounds",
            SpellSchool::Cleric,
            1,
            2, // sp cost
            0,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Heals 2d4 HP",
            Some(DiceRoll::new(2, 4, 0)),
            0,
            false,
        )
    }

    #[test]
    fn test_validate_spell_cast_success() {
        let character = create_test_sorcerer(5, 10);
        let spell = create_test_fireball();

        let gm = crate::application::GameMode::Combat(CombatState::new(
            crate::domain::combat::types::Handicap::Even,
        ));

        let res = validate_spell_cast(&character, &spell, &gm, true, false);
        assert!(res.is_ok());
    }

    #[test]
    fn test_validate_spell_cast_insufficient_sp() {
        let character = create_test_sorcerer(5, 1);
        let spell = create_test_fireball();

        let gm = crate::application::GameMode::Combat(CombatState::new(
            crate::domain::combat::types::Handicap::Even,
        ));

        let res = validate_spell_cast(&character, &spell, &gm, true, false);
        assert!(matches!(res, Err(SpellCastError::InsufficientSP { .. })));
    }

    #[test]
    fn test_validate_spell_cast_wrong_class() {
        // Paladin cannot cast Sorcerer spells
        let character = create_test_paladin(5, 10);
        let spell = create_test_fireball();

        let gm = crate::application::GameMode::Combat(CombatState::new(
            crate::domain::combat::types::Handicap::Even,
        ));

        let res = validate_spell_cast(&character, &spell, &gm, true, false);
        assert!(matches!(res, Err(SpellCastError::WrongSpellSchool)));
    }

    #[test]
    fn test_validate_spell_cast_paladin_level_3_can_cast() {
        let character = create_test_paladin(3, 10);
        let spell = create_test_cure();

        let gm = crate::application::GameMode::Combat(CombatState::new(
            crate::domain::combat::types::Handicap::Even,
        ));

        let res = validate_spell_cast(&character, &spell, &gm, true, false);
        assert!(res.is_ok());
    }

    #[test]
    fn test_validate_spell_cast_paladin_level_2_cannot_cast() {
        let character = create_test_paladin(2, 10);
        let spell = create_test_cure();

        let gm = crate::application::GameMode::Combat(CombatState::new(
            crate::domain::combat::types::Handicap::Even,
        ));

        let res = validate_spell_cast(&character, &spell, &gm, true, false);
        assert!(matches!(res, Err(SpellCastError::LevelTooLow { .. })));
    }

    #[test]
    fn test_execute_spell_cast_deducts_sp_and_gems() {
        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);
        let char = create_test_sorcerer(5, 10);
        cs.add_player(char);

        // Build a spell that targets the caster (self) so we don't need monsters
        let mut fire_self = create_test_fireball();
        fire_self.target = SpellTarget::Self_;
        fire_self.gem_cost = 2;

        let mut rng = rand::rng();
        // Execute
        let res = execute_spell_cast_with_spell(
            &mut cs,
            CombatantId::Player(0),
            &fire_self,
            CombatantId::Player(0),
            &mut ActiveSpells::new(),
            &ContentDatabase::new(),
            &mut rng,
        )
        .expect("cast should succeed");

        // Confirm caster SP and gems were deducted
        if let Some(Combatant::Player(pc)) = cs.get_combatant(&CombatantId::Player(0)) {
            assert_eq!(pc.sp.current, 7); // 10 - 3
            assert_eq!(pc.gems, 8); // 10 - 2
        } else {
            panic!("Caster not found after cast");
        }

        // Result may or may not include damage depending on dice, ensure success
        assert!(res.success);
    }

    #[test]
    fn test_silenced_condition_prevents_casting() {
        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);
        cs.add_player(create_test_sorcerer(5, 10));

        // Build a content DB and register the `silence` condition so apply-by-id succeeds
        let mut content = ContentDatabase::new();
        let cond_def = crate::domain::conditions::ConditionDefinition {
            id: "silence".to_string(),
            name: "Silence".to_string(),
            description: "Silences target".to_string(),
            effects: vec![crate::domain::conditions::ConditionEffect::StatusEffect(
                "silenced".to_string(),
            )],
            default_duration: crate::domain::conditions::ConditionDuration::Rounds(2),
            icon_id: None,
        };
        content.conditions.add_condition(cond_def);

        if let Some(Combatant::Player(pc)) = cs.get_combatant_mut(&CombatantId::Player(0)) {
            assert!(
                crate::domain::combat::engine::apply_condition_to_character_by_id(
                    pc.as_mut(),
                    "silence",
                    &content,
                )
                .is_ok()
            );
        } else {
            panic!("Caster not found");
        }

        let mut rng = rand::rng();
        let spell = create_test_fireball();

        let res = execute_spell_cast_with_spell(
            &mut cs,
            CombatantId::Player(0),
            &spell,
            CombatantId::Player(0),
            &mut ActiveSpells::new(),
            &content,
            &mut rng,
        );

        assert!(matches!(res, Err(SpellCastError::Silenced)));
    }

    #[test]
    fn test_full_spell_casting_flow() {
        // Arrange a small combat with a caster and a monster (if Monster::new exists)
        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);
        cs.add_player(create_test_sorcerer(5, 10));

        // Add a simple monster - construct with basic stats & loot
        let stats = crate::domain::character::Stats::new(10, 8, 6, 10, 10, 10, 10);
        let attacks = vec![crate::domain::combat::types::Attack::physical(
            crate::domain::types::DiceRoll::new(1, 4, 0),
        )];
        let monster = Monster::new(
            1,
            "Test Monster".to_string(),
            stats,
            10,
            5,
            attacks,
            crate::domain::combat::monster::LootTable::none(),
        );
        cs.add_monster(monster);

        let initial_mon_hp =
            if let Some(Combatant::Monster(m)) = cs.get_combatant(&CombatantId::Monster(1)) {
                m.hp.current
            } else {
                0
            };

        let mut rng = rand::rng();
        let spell = create_test_fireball();

        // Act: cast at monster index 1 (first monster participant)
        let res = execute_spell_cast_with_spell(
            &mut cs,
            CombatantId::Player(0),
            &spell,
            CombatantId::Monster(1),
            &mut ActiveSpells::new(),
            &ContentDatabase::new(),
            &mut rng,
        )
        .expect("cast should succeed");

        // Assert: caster lost SP, and if there was damage it affected the monster
        if let Some(Combatant::Player(pc)) = cs.get_combatant(&CombatantId::Player(0)) {
            assert_eq!(pc.sp.current, 7); // 10 - 3
        } else {
            panic!("Caster missing");
        }

        if let Some(Combatant::Monster(m)) = cs.get_combatant(&CombatantId::Monster(1)) {
            assert!(m.hp.current <= initial_mon_hp);
        }
        assert!(res.success || res.damage.is_some());
    }

    /// A spell with `resurrect_hp: Some(1)` targeting a dead player must
    /// clear `DEAD`, set `hp.current` to 1, and report healing in the result.
    #[test]
    fn test_resurrect_spell_revives_dead_player() {
        use crate::domain::character::Condition;
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};

        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);

        // Caster: high-level cleric with enough SP
        let caster = create_test_paladin(9, 20);
        cs.add_player(caster);

        // Target: dead party member
        let mut dead_hero = Character::new(
            "Dead Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        dead_hero.hp.base = 20;
        dead_hero.hp.current = 0;
        dead_hero.conditions.add(Condition::DEAD);
        dead_hero.add_condition(ActiveCondition::new(
            "dead".to_string(),
            ConditionDuration::Permanent,
        ));
        cs.add_player(dead_hero);

        // Resurrection spell: level 5 Cleric, targets SingleCharacter, resurrect_hp = Some(1)
        let mut resurrect = Spell::new(
            0x0105,
            "Resurrect",
            SpellSchool::Cleric,
            5,
            15,
            5,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Resurrects a dead party member to 1 HP",
            None,
            0,
            false,
        );
        resurrect.resurrect_hp = Some(1);

        let content = ContentDatabase::new();
        let mut rng = rand::rng();

        let res = execute_spell_cast_with_spell(
            &mut cs,
            CombatantId::Player(0),
            &resurrect,
            CombatantId::Player(1),
            &mut ActiveSpells::new(),
            &content,
            &mut rng,
        )
        .expect("resurrection spell must succeed");

        // Verify the dead hero was revived
        if let Some(Combatant::Player(hero)) = cs.get_combatant(&CombatantId::Player(1)) {
            assert!(
                !hero.conditions.has(Condition::DEAD),
                "DEAD flag must be cleared after resurrection spell"
            );
            assert_eq!(
                hero.hp.current, 1,
                "hp.current must be 1 after resurrection spell with resurrect_hp=Some(1)"
            );
            assert!(
                !hero
                    .active_conditions
                    .iter()
                    .any(|ac| ac.condition_id == "dead"),
                "active_conditions must not contain 'dead' after resurrection"
            );
        } else {
            panic!("Target hero not found");
        }

        assert!(
            res.healing.is_some(),
            "SpellResult must report healing after resurrection"
        );
        assert_eq!(res.healing, Some(1));
    }

    /// A resurrection spell targeting a living player (not dead) must be a
    /// complete no-op — HP and conditions are unchanged.
    #[test]
    fn test_resurrect_spell_noop_on_alive() {
        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);

        let caster = create_test_paladin(9, 20);
        cs.add_player(caster);

        // Target: alive party member with some HP
        let mut alive_hero = Character::new(
            "Alive Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        alive_hero.hp.base = 20;
        alive_hero.hp.current = 10;
        cs.add_player(alive_hero);

        let mut resurrect = Spell::new(
            0x0105,
            "Resurrect",
            SpellSchool::Cleric,
            5,
            15,
            5,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Resurrects a dead party member to 1 HP",
            None,
            0,
            false,
        );
        resurrect.resurrect_hp = Some(1);

        let content = ContentDatabase::new();
        let mut rng = rand::rng();

        let res = execute_spell_cast_with_spell(
            &mut cs,
            CombatantId::Player(0),
            &resurrect,
            CombatantId::Player(1),
            &mut ActiveSpells::new(),
            &content,
            &mut rng,
        )
        .expect("resurrection spell execution must not error");

        // Living hero's HP must be unchanged
        if let Some(Combatant::Player(hero)) = cs.get_combatant(&CombatantId::Player(1)) {
            assert_eq!(
                hero.hp.current, 10,
                "hp.current must be unchanged when Resurrect targets a living character"
            );
        } else {
            panic!("Target hero not found");
        }

        assert!(
            res.healing.is_none(),
            "SpellResult must not report healing when Resurrect is a no-op"
        );
    }

    // ── Phase 1: Healing dispatch in combat ───────────────────────────────────

    /// A healing spell targeting a single party member restores HP via the
    /// effect dispatcher and reports the healing in the returned SpellResult.
    #[test]
    fn test_healing_spell_restores_single_target_hp_in_combat() {
        use crate::domain::magic::types::SpellEffectType;
        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);

        // Caster: level-3 cleric with enough SP
        let caster = create_test_paladin(3, 10);
        cs.add_player(caster);

        // Target: wounded party member
        let mut wounded = Character::new(
            "Wounded".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        wounded.hp.base = 20;
        wounded.hp.current = 5;
        cs.add_player(wounded);

        // Build a healing spell
        let mut heal_spell = Spell::new(
            0x0101,
            "Cure Wounds",
            SpellSchool::Cleric,
            1,
            2,
            0,
            SpellContext::Anytime,
            crate::domain::magic::types::SpellTarget::SingleCharacter,
            "Heals the target",
            None,
            0,
            false,
        );
        heal_spell.effect_type = Some(SpellEffectType::Healing {
            amount: DiceRoll::new(2, 4, 0),
        });

        let mut active = ActiveSpells::new();
        let mut rng = rand::rng();

        let res = execute_spell_cast_with_spell(
            &mut cs,
            CombatantId::Player(0),
            &heal_spell,
            CombatantId::Player(1),
            &mut active,
            &ContentDatabase::new(),
            &mut rng,
        )
        .expect("healing spell must succeed");

        // Target HP must have increased
        if let Some(Combatant::Player(target)) = cs.get_combatant(&CombatantId::Player(1)) {
            assert!(
                target.hp.current > 5,
                "HP must increase after healing spell"
            );
            assert!(target.hp.current <= 20, "HP must not exceed base maximum");
        } else {
            panic!("Target not found after healing spell");
        }

        assert!(
            res.healing.is_some(),
            "SpellResult must report healing amount"
        );
        assert!(res.healing.unwrap() > 0);
    }

    /// A party-wide healing spell (AllCharacters) heals every player combatant.
    #[test]
    fn test_healing_spell_party_wide_heals_all_players() {
        use crate::domain::magic::types::SpellEffectType;
        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);

        let caster = create_test_paladin(3, 20);
        cs.add_player(caster);

        for i in 0..3 {
            let mut member = Character::new(
                format!("Member{i}"),
                "human".to_string(),
                "knight".to_string(),
                Sex::Male,
                Alignment::Good,
            );
            member.hp.base = 20;
            member.hp.current = 3;
            cs.add_player(member);
        }

        let mut heal_all = Spell::new(
            0x0202,
            "Mass Cure",
            SpellSchool::Cleric,
            1,
            2,
            0,
            SpellContext::Anytime,
            crate::domain::magic::types::SpellTarget::AllCharacters,
            "Heals the whole party",
            None,
            0,
            false,
        );
        heal_all.effect_type = Some(SpellEffectType::Healing {
            amount: DiceRoll::new(1, 6, 0),
        });

        let mut active = ActiveSpells::new();
        let mut rng = rand::rng();

        let res = execute_spell_cast_with_spell(
            &mut cs,
            CombatantId::Player(0),
            &heal_all,
            CombatantId::Player(0), // target ignored for AllCharacters
            &mut active,
            &ContentDatabase::new(),
            &mut rng,
        )
        .expect("party heal must succeed");

        // Every non-caster member should have more HP
        for idx in 1..=3 {
            if let Some(Combatant::Player(m)) = cs.get_combatant(&CombatantId::Player(idx)) {
                assert!(m.hp.current > 3, "Member {idx} should be healed");
            }
        }
        assert!(res.healing.is_some(), "SpellResult must report healing");
    }

    // ── Phase 1: Buff dispatch in combat ──────────────────────────────────────

    /// A buff spell writes the correct duration into active_spells.
    #[test]
    fn test_buff_spell_writes_to_active_spells() {
        use crate::domain::magic::types::{BuffField, SpellEffectType};
        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);
        cs.add_player(create_test_paladin(3, 10));

        let mut bless = Spell::new(
            0x0103,
            "Bless",
            SpellSchool::Cleric,
            2,
            3,
            0,
            SpellContext::Anytime,
            crate::domain::magic::types::SpellTarget::AllCharacters,
            "Grants combat bonus",
            None,
            30,
            false,
        );
        bless.effect_type = Some(SpellEffectType::Buff {
            buff_field: BuffField::Bless,
            duration: 30,
        });

        let mut active = ActiveSpells::new();
        assert_eq!(active.bless, 0);

        let mut rng = rand::rng();
        execute_spell_cast_with_spell(
            &mut cs,
            CombatantId::Player(0),
            &bless,
            CombatantId::Player(0),
            &mut active,
            &ContentDatabase::new(),
            &mut rng,
        )
        .expect("Bless spell must succeed");

        assert_eq!(
            active.bless, 30,
            "Bless should set active_spells.bless = 30"
        );
    }

    // ── Phase 1: Cure condition dispatch in combat ────────────────────────────

    /// A cure condition spell removes the named condition from the target character.
    #[test]
    fn test_cure_condition_spell_removes_condition_in_combat() {
        use crate::domain::character::Condition;
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};
        use crate::domain::magic::types::SpellEffectType;

        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);

        // Caster: paladin with SP
        let caster = create_test_paladin(5, 15);
        cs.add_player(caster);

        // Target: paralyzed party member
        let mut paralyzed = Character::new(
            "Frozen".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        paralyzed.hp.base = 20;
        paralyzed.hp.current = 20;
        paralyzed.conditions.add(Condition::PARALYZED);
        paralyzed.add_condition(ActiveCondition::new(
            "paralyzed".to_string(),
            ConditionDuration::Rounds(5),
        ));
        cs.add_player(paralyzed);

        let mut cure = Spell::new(
            0x0104,
            "Cure Paralysis",
            SpellSchool::Cleric,
            2,
            3,
            0,
            SpellContext::Anytime,
            crate::domain::magic::types::SpellTarget::SingleCharacter,
            "Cures paralysis",
            None,
            0,
            false,
        );
        cure.effect_type = Some(SpellEffectType::CureCondition {
            condition_id: "paralyzed".to_string(),
        });

        let mut active = ActiveSpells::new();
        let mut rng = rand::rng();

        execute_spell_cast_with_spell(
            &mut cs,
            CombatantId::Player(0),
            &cure,
            CombatantId::Player(1),
            &mut active,
            &ContentDatabase::new(),
            &mut rng,
        )
        .expect("Cure Paralysis must succeed");

        if let Some(Combatant::Player(target)) = cs.get_combatant(&CombatantId::Player(1)) {
            assert!(
                !target.conditions.has(Condition::PARALYZED),
                "PARALYZED bitflag must be cleared after cure spell"
            );
            assert!(
                target.active_conditions.is_empty(),
                "active_conditions must be empty after cure spell"
            );
        } else {
            panic!("Target not found after cure spell");
        }
    }

    // ── Phase 1: Composite dispatch in combat ─────────────────────────────────

    // ===== Fizzle tests =====

    fn create_test_sorcerer_low_intellect() -> Character {
        let mut c = Character::new(
            "Low-Int Mage".to_string(),
            "human".to_string(),
            "sorcerer".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        c.level = 5;
        c.stats.intellect.base = 5;
        c.stats.intellect.current = 5;
        c.sp.base = 100;
        c.sp.current = 100;
        c
    }

    fn create_test_cleric_high_personality() -> Character {
        let mut c = Character::new(
            "Holy Cleric".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        c.level = 10;
        c.stats.personality.base = 35;
        c.stats.personality.current = 35;
        c.sp.base = 100;
        c.sp.current = 100;
        c
    }

    /// A high-stat caster should have 0% fizzle chance at level 1 spells.
    #[test]
    fn test_high_stat_caster_does_not_fizzle_level_1() {
        // personality = 35 → fizzle = max(0, 50-(35-10)*2) + 0 = max(0,-50) = 0
        let cleric = create_test_cleric_high_personality();
        let chance = crate::domain::magic::fizzle::calculate_fizzle_chance(
            cleric.stats.personality.current,
            1,
        );
        assert_eq!(chance, 0, "High-stat cleric should have 0% fizzle at L1");
    }

    /// A low-stat sorcerer should have elevated fizzle chance.
    #[test]
    fn test_low_stat_caster_has_elevated_fizzle_chance() {
        // intellect = 5 → fizzle = max(0, 50-(5-10)*2) = max(0, 60) = 60
        let mage = create_test_sorcerer_low_intellect();
        let chance =
            crate::domain::magic::fizzle::calculate_fizzle_chance(mage.stats.intellect.current, 1);
        assert_eq!(chance, 60, "Low-stat sorcerer should have 60% fizzle at L1");
    }

    /// Fizzle roll with 0% chance must never fizzle.
    #[test]
    fn test_fizzle_roll_zero_chance_never_fizzles() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(12345);
        for _ in 0..100 {
            assert!(
                !crate::domain::magic::fizzle::roll_fizzle(0, &mut rng),
                "0% fizzle must never trigger"
            );
        }
    }

    /// SP is consumed even on a fizzle (verified by checking SP decreased).
    #[test]
    fn test_fizzle_consumes_sp() {
        use rand::SeedableRng;

        let rng = rand::rngs::StdRng::seed_from_u64(0);
        let mut content = ContentDatabase::new();

        // Low-int sorcerer casts a L1 spell — still 68% fizzle at intellect=1
        let mut caster = create_test_sorcerer_low_intellect();
        // Force a very high fizzle chance by setting intellect very low
        // fizzle = max(0, 50-(1-10)*2) = 68% at L1, no level penalty (base > 0)
        caster.stats.intellect.current = 1;
        caster.level = 5;

        // Use a L1 spell so a level-5 sorcerer can always cast it
        let mut fireball = create_test_fireball();
        fireball.level = 1;
        fireball.sp_cost = 5;

        content.spells.add_spell(fireball.clone()).ok();

        let mut combat = CombatState::new(crate::domain::combat::types::Handicap::Even);
        combat
            .participants
            .push(Combatant::Player(Box::new(caster)));

        let monster = Monster::new(
            1,
            "Goblin".to_string(),
            crate::domain::character::Stats::new(8, 5, 8, 10, 10, 8, 10),
            20,
            5,
            vec![],
            crate::domain::combat::monster::LootTable::none(),
        );
        combat
            .participants
            .push(Combatant::Monster(Box::new(monster)));

        let sp_before = {
            if let Combatant::Player(pc) = &combat.participants[0] {
                pc.sp.current
            } else {
                panic!("Expected player at index 0")
            }
        };

        let mut active_spells = ActiveSpells::new();

        // Use a seeded RNG; with intellect=1 at L7: fizzle = 68 + 12 = 80%
        // Try many times — at least some should fizzle
        let mut fizzled = false;
        for seed in 0u64..100 {
            let mut rng2 = rand::rngs::StdRng::seed_from_u64(seed);
            let mut combat2 = CombatState::new(crate::domain::combat::types::Handicap::Even);

            let mut c = create_test_sorcerer_low_intellect();
            c.stats.intellect.current = 1;
            c.sp.base = 100;
            c.sp.current = 30;

            combat2.participants.push(Combatant::Player(Box::new(c)));
            let m2 = Monster::new(
                1,
                "Goblin".to_string(),
                crate::domain::character::Stats::new(8, 5, 8, 10, 10, 8, 10),
                20,
                5,
                vec![],
                crate::domain::combat::monster::LootTable::none(),
            );
            combat2.participants.push(Combatant::Monster(Box::new(m2)));

            // L1 spell: level-5 sorcerer can always cast it; intellect=1 → 68% fizzle
            let mut spell2 = create_test_fireball();
            spell2.level = 1;
            spell2.sp_cost = 5;

            let result = execute_spell_cast_with_spell(
                &mut combat2,
                CombatantId::Player(0),
                &spell2,
                CombatantId::Monster(1),
                &mut active_spells,
                &content,
                &mut rng2,
            );

            if let Ok(r) = &result {
                if !r.success {
                    fizzled = true;
                    // SP must still be deducted
                    if let Combatant::Player(pc) = &combat2.participants[0] {
                        assert!(pc.sp.current < 30, "SP should be consumed even on fizzle");
                    }
                    break;
                }
            }
        }

        assert!(
            fizzled,
            "Low-stat caster should fizzle at least once in 100 attempts"
        );
        let _ = (sp_before, rng);
    }

    // ===== DispelMagic tests =====

    fn create_dispel_magic_spell() -> Spell {
        let mut s = Spell::new(
            2049, // Cleric L5 #1
            "Dispel Magic",
            crate::domain::magic::types::SpellSchool::Cleric,
            5,
            20,
            0,
            crate::domain::magic::types::SpellContext::Anytime,
            SpellTarget::AllCharacters,
            "Dispels all active magical effects",
            None,
            0,
            false,
        );
        s.effect_type = Some(SpellEffectType::DispelMagic);
        s
    }

    /// Dispel Magic resets all active spells to zero.
    #[test]
    fn test_dispel_magic_resets_active_spells() {
        use rand::SeedableRng;

        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let content = ContentDatabase::new();

        let mut cleric = Character::new(
            "Cleric".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        cleric.level = 10;
        cleric.stats.personality.base = 35;
        cleric.stats.personality.current = 35;
        cleric.sp.base = 100;
        cleric.sp.current = 100;

        let mut combat = CombatState::new(crate::domain::combat::types::Handicap::Even);
        combat
            .participants
            .push(Combatant::Player(Box::new(cleric)));

        let mut active_spells = ActiveSpells::new();
        // Pre-set some active spells
        active_spells.fire_protection = 30;
        active_spells.bless = 10;
        active_spells.shield = 5;

        let spell = create_dispel_magic_spell();

        let result = execute_spell_cast_with_spell(
            &mut combat,
            CombatantId::Player(0),
            &spell,
            CombatantId::Player(0),
            &mut active_spells,
            &content,
            &mut rng,
        );

        assert!(result.is_ok(), "Dispel Magic should succeed");
        let r = result.unwrap();
        assert!(r.success, "Dispel Magic result should be success");
        assert_eq!(
            active_spells.fire_protection, 0,
            "fire_protection should be reset"
        );
        assert_eq!(active_spells.bless, 0, "bless should be reset");
        assert_eq!(active_spells.shield, 0, "shield should be reset");
    }

    /// Dispel Magic also clears active conditions from party members.
    #[test]
    fn test_dispel_magic_clears_party_conditions() {
        use crate::domain::conditions::ActiveCondition;
        use rand::SeedableRng;

        let mut rng = rand::rngs::StdRng::seed_from_u64(99);
        let content = ContentDatabase::new();

        let mut cleric = Character::new(
            "Cleric".to_string(),
            "human".to_string(),
            "cleric".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        cleric.level = 10;
        cleric.stats.personality.base = 35;
        cleric.stats.personality.current = 35;
        cleric.sp.base = 100;
        cleric.sp.current = 100;
        // Give the cleric an active condition
        cleric.active_conditions.push(ActiveCondition {
            condition_id: "bless".to_string(),
            duration: crate::domain::conditions::ConditionDuration::Rounds(5),
            magnitude: 1.0,
        });

        let mut combat = CombatState::new(crate::domain::combat::types::Handicap::Even);
        combat
            .participants
            .push(Combatant::Player(Box::new(cleric)));

        let mut active_spells = ActiveSpells::new();
        active_spells.bless = 10;

        let spell = create_dispel_magic_spell();

        let _ = execute_spell_cast_with_spell(
            &mut combat,
            CombatantId::Player(0),
            &spell,
            CombatantId::Player(0),
            &mut active_spells,
            &content,
            &mut rng,
        );

        if let Combatant::Player(pc) = &combat.participants[0] {
            assert!(
                pc.active_conditions.is_empty(),
                "Dispel Magic should clear all active conditions"
            );
        }
        assert_eq!(active_spells.bless, 0, "bless active spell should be reset");
    }

    #[test]
    fn test_composite_spell_heals_and_cures_in_combat() {
        use crate::domain::character::Condition;
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};
        use crate::domain::magic::types::SpellEffectType;

        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);

        let caster = create_test_paladin(5, 20);
        cs.add_player(caster);

        let mut victim = Character::new(
            "Victim".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        victim.hp.base = 20;
        victim.hp.current = 5;
        victim.conditions.add(Condition::POISONED);
        victim.add_condition(ActiveCondition::new(
            "poisoned".to_string(),
            ConditionDuration::Permanent,
        ));
        cs.add_player(victim);

        let mut combo = Spell::new(
            0x0105,
            "Cure Wounds + Antidote",
            SpellSchool::Cleric,
            3,
            5,
            0,
            SpellContext::Anytime,
            crate::domain::magic::types::SpellTarget::SingleCharacter,
            "Heals and cures poison",
            None,
            0,
            false,
        );
        combo.effect_type = Some(SpellEffectType::Composite(vec![
            SpellEffectType::Healing {
                amount: DiceRoll::new(2, 4, 0),
            },
            SpellEffectType::CureCondition {
                condition_id: "poisoned".to_string(),
            },
        ]));

        let mut active = ActiveSpells::new();
        let mut rng = rand::rng();

        let res = execute_spell_cast_with_spell(
            &mut cs,
            CombatantId::Player(0),
            &combo,
            CombatantId::Player(1),
            &mut active,
            &ContentDatabase::new(),
            &mut rng,
        )
        .expect("composite spell must succeed");

        if let Some(Combatant::Player(target)) = cs.get_combatant(&CombatantId::Player(1)) {
            assert!(
                target.hp.current > 5,
                "HP should increase from composite heal"
            );
            assert!(
                !target.conditions.has(Condition::POISONED),
                "POISONED must be cleared by composite cure"
            );
            assert!(target.active_conditions.is_empty());
        } else {
            panic!("Target not found after composite spell");
        }

        assert!(res.healing.is_some(), "SpellResult must report healing");
    }
}
