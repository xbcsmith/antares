/*
SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
SPDX-License-Identifier: Apache-2.0
*/

//! Combat-oriented item usage helpers
//!
//! This module provides combat-facing helpers for validating and executing
//! item usage (Phase 8: Item Usage System).
//!
//! ## Responsibilities
//!
//! - Validate that an inventory slot contains a usable item (consumable,
//!   combat-usable flag, alignment, race/proficiency restrictions as needed)
//!   via [`validate_item_use_slot`].
//! - Execute consumable effects against a target in [`CombatState`] via
//!   [`execute_item_use_by_slot`].
//! - Consume inventory charges (decrement or remove slot).
//! - Advance the combat turn and apply round/condition ticks.
//!
//! ## Effect Application — Shared Helper
//!
//! All five [`ConsumableEffect`] variants (`HealHp`, `RestoreSp`,
//! `CureCondition`, `BoostAttribute`, `BoostResistance`) are **not** matched
//! here. Instead, [`execute_item_use_by_slot`] delegates effect application to
//! [`apply_consumable_effect`] in
//! [`crate::domain::items::consumable_usage`].  This ensures there is exactly
//! one authoritative implementation shared between the combat path and the
//! exploration/menu path introduced in Phase 3.
//!
//! ## Design Notes
//!
//! - Consumables are the primary item type supported in combat for this phase.
//! - Only minimal behaviour is implemented for effects (no durations on boosts).
//! - Validation only queries class/race definitions when required (e.g., when an
//!   item has a proficiency requirement or tags to validate); this keeps
//!   tests lightweight for common consumables which typically have no profs/tags.

use crate::domain::combat::engine::CombatState;
use crate::domain::combat::types::CombatantId;
use crate::domain::items::consumable_usage::apply_consumable_effect;
use crate::domain::items::types::{ConsumableData, ConsumableEffect, ItemType};
use crate::domain::types::ItemId;
use crate::sdk::database::ContentDatabase;
use rand::Rng;
use thiserror::Error;

/// Action to use an inventory item in combat
///
/// This is a small, serializable-friendly data structure that mirrors the data
/// produced by UI layers when a player chooses to use an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemUseAction {
    /// Who is using the item (combatant id)
    pub user: CombatantId,
    /// Inventory slot index in the user's backpack
    pub inventory_index: usize,
    /// Target for the item (meaning depends on effect)
    pub target: CombatantId,
}

/// Result of applying an item effect in combat
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemUseResult {
    /// Whether the use succeeded
    pub success: bool,
    /// Human-readable effect message
    pub effect_message: String,
    /// Damage dealt (if applicable)
    pub damage: Option<i32>,
    /// Healing done (if applicable)
    pub healing: Option<i32>,
    /// Indices of affected targets (participant indices)
    pub affected_targets: Vec<usize>,
    /// Applied condition identifiers (if any)
    pub applied_conditions: Vec<String>,
}

impl ItemUseResult {
    /// Create a successful outcome with a message
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            effect_message: message.into(),
            damage: None,
            healing: None,
            affected_targets: Vec::new(),
            applied_conditions: Vec::new(),
        }
    }

    /// Attach damage information
    pub fn with_damage(mut self, amount: i32, affected: Vec<usize>) -> Self {
        self.damage = Some(amount);
        self.affected_targets = affected;
        self
    }

    /// Attach healing information
    pub fn with_healing(mut self, amount: i32, affected: Vec<usize>) -> Self {
        self.healing = Some(amount);
        self.affected_targets = affected;
        self
    }

    /// Attach applied condition ids
    pub fn with_applied_conditions(mut self, ids: Vec<String>) -> Self {
        self.applied_conditions = ids;
        self
    }
}

/// Errors that can occur when validating or executing item usage
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ItemUseError {
    #[error("Item not found in database: {0}")]
    ItemNotFound(ItemId),

    #[error("Inventory slot invalid: {0}")]
    InventorySlotInvalid(usize),

    #[error("Item is not a consumable and cannot be used this way")]
    NotConsumable,

    #[error("Item cannot be used in combat")]
    NotUsableInCombat,

    #[error("Character's alignment cannot use this item")]
    AlignmentRestriction,

    #[error("Character class cannot use this item (missing required proficiency)")]
    ClassRestriction,

    #[error("Character race cannot use this item (incompatible tag)")]
    RaceRestriction,

    #[error("No charges left to use")]
    NoCharges,

    #[error("Invalid target for item")]
    InvalidTarget,

    #[error("Other item use error: {0}")]
    Other(String),
}

/// Validate whether the given `character` can use the item located at
/// `slot_index` in `character.inventory` within the given `content` context.
///
/// This performs:
/// - Existence checks (slot and item)
/// - Consumable check
/// - Combat-use check (when `in_combat == true`)
/// - Alignment restriction check
/// - Optional proficiency / race tag checks when the item has such requirements
///
/// # Arguments
///
/// * `character` - Character performing the use (snapshot; no mutation)
/// * `slot_index` - Index into `character.inventory.items`
/// * `content` - Content DB (items, classes, races)
/// * `in_combat` - Whether this validation is happening in combat
///
/// # Errors
///
/// Returns `ItemUseError` describing the reason the item cannot be used.
pub fn validate_item_use_slot(
    character: &crate::domain::character::Character,
    slot_index: usize,
    content: &ContentDatabase,
    in_combat: bool,
) -> Result<(), ItemUseError> {
    // Inventory slot exists?
    let slot = character
        .inventory
        .items
        .get(slot_index)
        .ok_or(ItemUseError::InventorySlotInvalid(slot_index))?;

    // Item exists?
    let item = content
        .items
        .get_item(slot.item_id)
        .ok_or(ItemUseError::ItemNotFound(slot.item_id))?;

    // Accept the slot if it is either:
    //  (a) a Consumable — original path, or
    //  (b) a non-consumable magical charged item with spell_effect set — new path.
    match &item.item_type {
        ItemType::Consumable(consumable) => {
            // Combat use allowed?
            if in_combat && !consumable.is_combat_usable {
                return Err(ItemUseError::NotUsableInCombat);
            }
        }
        _ => {
            // Non-consumable: only valid if it has a spell effect and charges.
            if item.spell_effect.is_none() || item.max_charges == 0 {
                return Err(ItemUseError::NotConsumable);
            }
            if slot.charges == 0 {
                return Err(ItemUseError::NoCharges);
            }
        }
    }

    // Alignment
    if !item.can_use_alignment(character.alignment) {
        return Err(ItemUseError::AlignmentRestriction);
    }

    // If item requires proficiency, validate it using class/race proficiencies.
    // Many consumables have no proficiency requirement, so we avoid expensive
    // DB lookups when not necessary.
    if let Some(required_prof) = item.required_proficiency() {
        // Lookup class definition (if missing -> treat as restriction)
        let class_def_opt = content.classes.get_class(&character.class_id);
        let race_def_opt = content.races.get_race(&character.race_id);

        let class_profs = class_def_opt
            .map(|c| c.proficiencies.clone())
            .unwrap_or_default();
        let race_profs = race_def_opt
            .map(|r| r.proficiencies.clone())
            .unwrap_or_default();

        if !crate::domain::proficiency::has_proficiency_union(
            Some(&required_prof),
            &class_profs,
            &race_profs,
        ) {
            return Err(ItemUseError::ClassRestriction);
        }
    }

    // If item has tags, ensure race can use it (only validate if tags present)
    if !item.tags.is_empty() {
        if let Some(race_def) = content.races.get_race(&character.race_id) {
            if !race_def.can_use_item(&item.tags) {
                return Err(ItemUseError::RaceRestriction);
            }
        } else {
            // Race not found; treat as restriction
            return Err(ItemUseError::RaceRestriction);
        }
    }

    // All checks passed
    Ok(())
}

/// Execute using the item found at `inventory_index` for `user` against `target`.
///
/// Steps performed in order:
///
/// 1. Validate the use via [`validate_item_use_slot`] (checks consumable flag,
///    `is_combat_usable`, alignment, proficiency/race restrictions, charges).
/// 2. Consume one charge from the inventory slot (or remove the slot when the
///    last charge is spent).
/// 3. **Delegate effect application** to
///    [`apply_consumable_effect`][crate::domain::items::consumable_usage::apply_consumable_effect]
///    — the single authoritative implementation for all five
///    [`ConsumableEffect`] variants.  There is no duplicated match here.
/// 4. Advance the combat turn and tick round/condition effects.
///
/// # Arguments
///
/// * `combat_state` - Mutable combat state; the user and target must both
///   be present as `Combatant::Player` entries.
/// * `user` - Combatant identifier of the character using the item
///   (must be a `CombatantId::Player`).
/// * `inventory_index` - Index into the user's `inventory.items` slice.
/// * `target` - Combatant identifier receiving the effect (must be a
///   `CombatantId::Player`; self-target is supported).
/// * `content` - Content DB used for item lookup and condition ticking.
/// * `rng` - Random-number generator (present for API parity with
///   spell helpers; not consumed by current effect variants).
///
/// # Returns
///
/// Returns `Ok(ItemUseResult)` describing what happened on success, or
/// `Err(ItemUseError)` on any validation failure.
///
/// # Errors
///
/// Returns the same [`ItemUseError`] variants as [`validate_item_use_slot`]
/// plus `InvalidTarget` when the combatant IDs cannot be resolved.
pub fn execute_item_use_by_slot<R: Rng>(
    combat_state: &mut CombatState,
    user: CombatantId,
    inventory_index: usize,
    target: CombatantId,
    content: &ContentDatabase,
    rng: &mut R,
) -> Result<ItemUseResult, ItemUseError> {
    use crate::domain::combat::engine::Combatant;

    // Ensure user exists & is a player - take a snapshot for validation
    let user_snapshot = match combat_state.get_combatant(&user) {
        Some(Combatant::Player(pc)) => pc.as_ref().clone(),
        _ => return Err(ItemUseError::InvalidTarget),
    };

    // Validate using snapshot (no active mutable borrow on combat_state)
    validate_item_use_slot(&user_snapshot, inventory_index, content, true)?;

    // ── Charged spell-item path ──────────────────────────────────────────────
    // If the item at `inventory_index` is a non-consumable charged item with a
    // `spell_effect` set, route it through the combat spell pipeline instead of
    // the consumable path below.  The charge is consumed inside
    // `execute_charged_item_spell`.
    {
        let slot_item_id = user_snapshot
            .inventory
            .items
            .get(inventory_index)
            .map(|s| s.item_id);

        if let Some(iid) = slot_item_id {
            if let Some(item) = content.items.get_item(iid) {
                let is_spell_charged = item.spell_effect.is_some()
                    && item.max_charges > 0
                    && !matches!(item.item_type, ItemType::Consumable(_));
                if is_spell_charged {
                    use crate::application::ActiveSpells;
                    // We need an ActiveSpells but the combat path doesn't carry
                    // one; create a temporary one so the spell pipeline compiles.
                    // Callers that want buff tracking must use
                    // `execute_charged_item_spell` directly with their own
                    // `ActiveSpells`.
                    let mut temp_active = ActiveSpells::new();
                    let spell_result =
                        crate::domain::combat::spell_casting::execute_charged_item_spell(
                            combat_state,
                            user,
                            inventory_index,
                            target,
                            &mut temp_active,
                            content,
                            rng,
                        )
                        .map_err(|e| ItemUseError::Other(e.to_string()))?;

                    let msg = spell_result.effect_message.clone();
                    let res = ItemUseResult::success(msg);
                    let final_res = if let Some(dmg) = spell_result.damage {
                        res.with_damage(dmg, spell_result.affected_targets.clone())
                    } else if let Some(hp) = spell_result.healing {
                        res.with_healing(hp, spell_result.affected_targets.clone())
                    } else {
                        res
                    };
                    return Ok(final_res);
                }
            }
        }
    }

    // Perform the effect by splitting the operation into two phases to avoid
    // simultaneous mutable borrows of `combat_state`:
    //
    // Phase A: Mutably borrow the user only long enough to consume the inventory
    //          charge and capture the consumable's effect.
    // Phase B: Re-borrow the combat state to apply the captured effect to the
    //          target (healing, SP restore, condition cure, or attribute boost).
    let (result_message, effected_indices, damage_amount, healing_amount, applied_conds) = {
        // Phase A: consume charge and capture consumable data (full struct, not
        // just the effect) so that duration_minutes is available for Phase B.
        let (item_name, consumable_data) = {
            // Small mutable borrow for the user (only to mutate inventory)
            let user_ref_mut = combat_state
                .get_combatant_mut(&user)
                .ok_or(ItemUseError::InvalidTarget)?;

            let pc = match user_ref_mut {
                Combatant::Player(pc) => pc.as_mut(),
                _ => return Err(ItemUseError::InvalidTarget),
            };

            if inventory_index >= pc.inventory.items.len() {
                return Err(ItemUseError::InventorySlotInvalid(inventory_index));
            }

            // Access the slot mutably to consume a charge (or remove the slot)
            let slot = pc
                .inventory
                .items
                .get_mut(inventory_index)
                .ok_or(ItemUseError::InventorySlotInvalid(inventory_index))?;

            if slot.charges == 0 {
                return Err(ItemUseError::NoCharges);
            }

            let item = content
                .items
                .get_item(slot.item_id)
                .ok_or(ItemUseError::ItemNotFound(slot.item_id))?;

            let consumable = match &item.item_type {
                ItemType::Consumable(data) => data,
                _ => return Err(ItemUseError::NotConsumable),
            };

            // Clone the full ConsumableData so duration_minutes is available
            // after the inventory borrow ends.
            let consumable_data: ConsumableData = *consumable;

            // Consume one charge from the slot (or remove it)
            if slot.charges > 1 {
                slot.charges -= 1;
            } else if pc.inventory.remove_item(inventory_index).is_none() {
                tracing::warn!(
                    "Consumable removal: inventory index {} was already empty",
                    inventory_index
                );
            }

            (item.name.clone(), consumable_data)
        };

        // Phase B: apply captured effect to target via the shared pure-domain
        // helper. This delegates the authoritative ConsumableEffect match so
        // there is exactly one implementation for all five variants.
        //
        // Food items are never usable in combat — the `is_combat_usable` flag
        // on `ConsumableData` blocks them via `validate_item_use_slot`. If one
        // somehow reaches here, `apply_consumable_effect` returns a zeroed
        // result (no-op), and the safety guard below surfaces the error.
        if matches!(consumable_data.effect, ConsumableEffect::IsFood(_)) {
            return Err(ItemUseError::NotUsableInCombat);
        }

        let mut effected_indices: Vec<usize> = Vec::new();
        let total_damage: i32 = 0;
        let mut total_healing: i32 = 0;
        let mut applied_conditions: Vec<String> = Vec::new();

        // ── ConsumableEffect::CastSpell path ────────────────────────────────
        // When the consumable signals a spell cast, route through the full
        // combat spell pipeline (damage, healing, buff, fizzle, etc.).
        // The charge was already consumed in Phase A above.
        if let ConsumableEffect::CastSpell(spell_id) = consumable_data.effect {
            use crate::application::ActiveSpells;
            let mut temp_active = ActiveSpells::new();
            if let Some(spell) = content.spells.get_spell(spell_id).cloned() {
                // Determine the user player index for the result
                if let CombatantId::Player(user_idx) = user {
                    // Re-borrow the caster to give them enough SP for the spell
                    // (the item pays the cost; SP is not consumed from the caster).
                    {
                        if let Some(Combatant::Player(pc)) = combat_state.get_combatant_mut(&user) {
                            let needed = spell.sp_cost;
                            if pc.sp.current < needed {
                                pc.sp.current = needed;
                            }
                        }
                    }
                    let spell_result =
                        crate::domain::combat::spell_casting::execute_spell_cast_with_spell(
                            combat_state,
                            user,
                            &spell,
                            target,
                            &mut temp_active,
                            content,
                            rng,
                        )
                        .map_err(|e| ItemUseError::Other(e.to_string()))?;

                    let msg = spell_result.effect_message.clone();
                    let res = ItemUseResult::success(msg);
                    let final_res = if let Some(dmg) = spell_result.damage {
                        res.with_damage(dmg, spell_result.affected_targets.clone())
                    } else if let Some(hp) = spell_result.healing {
                        res.with_healing(hp, spell_result.affected_targets.clone())
                    } else {
                        res
                    };
                    let _ = user_idx;
                    return Ok(final_res);
                }
            }
            // Spell not found in DB — treat as no-op success
            return Ok(ItemUseResult::success(format!(
                "Item used: spell {} not found",
                spell_id
            )));
        }

        match combat_state.get_combatant_mut(&target) {
            Some(Combatant::Player(pc_target)) => {
                if let CombatantId::Player(idx) = target {
                    let apply_result = apply_consumable_effect(pc_target, &consumable_data);

                    if apply_result.healing > 0 {
                        total_healing += apply_result.healing;
                        effected_indices.push(idx);
                    } else if apply_result.sp_restored > 0 {
                        total_healing += apply_result.sp_restored;
                        effected_indices.push(idx);
                    } else if apply_result.conditions_cleared != 0 {
                        effected_indices.push(idx);
                        applied_conditions.push(format!(
                            "cleared_flags:{:#X}",
                            apply_result.conditions_cleared
                        ));
                    } else if apply_result.attribute_delta != 0
                        || apply_result.resistance_delta != 0
                    {
                        effected_indices.push(idx);
                    }
                }
            }
            _ => return Err(ItemUseError::InvalidTarget),
        }

        // Build a short result message
        let msg = if total_damage > 0 {
            format!("Item used: {} dealt {} damage", item_name, total_damage)
        } else if total_healing > 0 {
            format!("Item used: {} healed {} HP/SP", item_name, total_healing)
        } else if !applied_conditions.is_empty() {
            format!("Item used: {} applied effects", item_name)
        } else {
            format!("Item used: {}", item_name)
        };

        (
            msg,
            effected_indices,
            total_damage,
            total_healing,
            applied_conditions,
        )
    };

    // Advance round/turn using content condition definitions (same behavior as spells)
    let cond_defs: Vec<crate::domain::conditions::ConditionDefinition> = content
        .conditions
        .all_conditions()
        .into_iter()
        .filter_map(|id| content.conditions.get_condition(id).cloned())
        .collect();

    let _round_effects = combat_state.advance_turn(&cond_defs);

    // Check end-of-combat conditions
    combat_state.check_combat_end();

    // Compose ItemUseResult
    let res = ItemUseResult::success(result_message);

    // Attach numeric results if present
    let final_res = if damage_amount != 0 {
        res.with_damage(damage_amount, effected_indices)
    } else if healing_amount != 0 {
        res.with_healing(healing_amount, effected_indices)
    } else if !applied_conds.is_empty() {
        res.with_applied_conditions(applied_conds)
    } else {
        res
    };

    Ok(final_res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::combat::engine::{CombatState, Combatant};
    use crate::domain::items::consumable_usage::apply_consumable_effect;
    use crate::domain::items::types::{
        AttributeType, ConsumableData, ConsumableEffect, Item, ItemType, ResistanceType,
    };
    use crate::sdk::database::ContentDatabase;

    /// Helper: create a simple healing potion item
    fn create_healing_potion(id: ItemId, amount: u16, combat_usable: bool) -> Item {
        Item {
            id,
            name: "Healing Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHp(amount),
                is_combat_usable: combat_usable,
                duration_minutes: None,
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        }
    }

    /// Helper: create a simple cure potion
    fn create_cure_potion(id: ItemId, flags: u8) -> Item {
        Item {
            id,
            name: "Antidote".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::CureCondition(flags),
                is_combat_usable: true,
                duration_minutes: None,
            }),
            base_cost: 8,
            sell_cost: 4,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        }
    }

    #[test]
    fn test_validate_consumable_in_combat_requires_combat_usable_flag() {
        let mut content = ContentDatabase::new();
        let potion = create_healing_potion(200, 5, false);
        content.items.add_item(potion).unwrap();

        let mut ch = Character::new(
            "Test".to_string(),
            "human".to_string(),
            "none".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        // Place item into inventory (1 charge)
        let _ = ch.inventory.add_item(200, 1);

        // In-combat validation should fail because potion is not combat-usable
        let res = validate_item_use_slot(&ch, 0, &content, true);
        assert!(matches!(res, Err(ItemUseError::NotUsableInCombat)));

        // Out-of-combat validation should succeed
        let res2 = validate_item_use_slot(&ch, 0, &content, false);
        assert!(res2.is_ok());
    }

    #[test]
    fn test_use_healing_potion_consumes_and_heals() {
        let mut content = ContentDatabase::new();
        let potion_id = 201;
        let potion = create_healing_potion(potion_id, 20, true);
        content.items.add_item(potion).unwrap();

        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);
        let mut pc = Character::new(
            "Healer".to_string(),
            "human".to_string(),
            "none".to_string(),
            Sex::Female,
            Alignment::Good,
        );

        // Damage the player a bit
        pc.hp.modify(-15);

        // Add potion to inventory
        pc.inventory.add_item(potion_id, 1).unwrap();

        cs.add_player(pc);

        let mut rng = rand::rng();
        // Use the potion on player 0 (slot index 0)
        let res = execute_item_use_by_slot(
            &mut cs,
            CombatantId::Player(0),
            0,
            CombatantId::Player(0),
            &content,
            &mut rng,
        )
        .expect("use should succeed");

        // After use, player should be healed and inventory slot removed
        if let Some(Combatant::Player(pc_after)) = cs.get_combatant(&CombatantId::Player(0)) {
            assert!(pc_after.inventory.items.is_empty());
            // HP should have increased by up to 20 (but not exceed base)
            assert!(pc_after.hp.current > 0);
        } else {
            panic!("player should still be present");
        }

        assert!(res.success);
    }

    #[test]
    fn test_use_cure_condition_clears_flag() {
        use crate::domain::character::Condition;

        let mut content = ContentDatabase::new();
        let potion_id = 202;
        let potion = create_cure_potion(potion_id, Condition::POISONED);
        content.items.add_item(potion).unwrap();

        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);
        let mut pc = Character::new(
            "Victim".to_string(),
            "human".to_string(),
            "none".to_string(),
            Sex::Male,
            Alignment::Good,
        );

        // Apply poisoned condition
        pc.conditions.add(Condition::POISONED);

        // Ensure condition present
        assert!(pc.conditions.has(Condition::POISONED));

        // Add potion to inventory
        pc.inventory.add_item(potion_id, 1).unwrap();

        cs.add_player(pc);

        let mut rng = rand::rng();
        let _ = execute_item_use_by_slot(
            &mut cs,
            CombatantId::Player(0),
            0,
            CombatantId::Player(0),
            &content,
            &mut rng,
        )
        .expect("cure should succeed");

        // Condition should be gone
        if let Some(Combatant::Player(pc_after)) = cs.get_combatant(&CombatantId::Player(0)) {
            assert!(!pc_after.conditions.has(Condition::POISONED));
        } else {
            panic!("player should exist");
        }
    }

    /// Helper: create a BoostResistance potion
    fn create_boost_resistance_potion(
        id: ItemId,
        res_type: crate::domain::items::types::ResistanceType,
        amount: i8,
    ) -> Item {
        Item {
            id,
            name: "Resistance Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::BoostResistance(res_type, amount),
                is_combat_usable: true,
                duration_minutes: None,
            }),
            base_cost: 15,
            sell_cost: 7,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        }
    }

    /// Helper: create a BoostAttribute potion
    fn create_boost_attribute_potion(id: ItemId, attr: AttributeType, amount: i8) -> Item {
        Item {
            id,
            name: "Attribute Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::BoostAttribute(attr, amount),
                is_combat_usable: true,
                duration_minutes: None,
            }),
            base_cost: 12,
            sell_cost: 6,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        }
    }

    /// Regression: verify that `execute_item_use_by_slot` and a direct call to
    /// `apply_consumable_effect` produce identical healing deltas, confirming
    /// the combat executor delegates to the shared helper correctly.
    #[test]
    fn test_execute_item_use_delegates_to_shared_helper() {
        let potion_id: ItemId = 210;
        let heal_amount: u16 = 15;

        // --- Baseline: compute expected delta using the pure helper directly ---
        let mut reference_pc = Character::new(
            "Reference".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        reference_pc.hp.base = 30;
        reference_pc.hp.current = 10;
        let ref_result = apply_consumable_effect(
            &mut reference_pc,
            &ConsumableData {
                effect: ConsumableEffect::HealHp(heal_amount),
                is_combat_usable: true,
                duration_minutes: None,
            },
        );
        let expected_healing = ref_result.healing;
        let expected_hp = reference_pc.hp.current;

        // --- Execute via the full combat path ---
        let mut content = ContentDatabase::new();
        content
            .items
            .add_item(create_healing_potion(potion_id, heal_amount, true))
            .unwrap();

        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);
        let mut pc = Character::new(
            "Fighter".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        pc.hp.base = 30;
        pc.hp.current = 10;
        pc.inventory.add_item(potion_id, 1).unwrap();
        cs.add_player(pc);

        let mut rng = rand::rng();
        let res = execute_item_use_by_slot(
            &mut cs,
            CombatantId::Player(0),
            0,
            CombatantId::Player(0),
            &content,
            &mut rng,
        )
        .expect("execute should succeed");

        // The combat result must match the pure-helper baseline exactly.
        assert!(res.success);
        assert_eq!(
            res.healing,
            Some(expected_healing),
            "combat path healing must match pure-helper baseline"
        );

        if let Some(Combatant::Player(pc_after)) = cs.get_combatant(&CombatantId::Player(0)) {
            assert_eq!(
                pc_after.hp.current, expected_hp,
                "post-combat HP must match pure-helper baseline"
            );
        } else {
            panic!("player should still be present after item use");
        }
    }

    /// Regression: `validate_item_use_slot` with `in_combat = false` must
    /// permit items where `is_combat_usable: false`.
    #[test]
    fn test_validate_out_of_combat_permits_non_combat_usable() {
        let mut content = ContentDatabase::new();
        let potion_id: ItemId = 211;
        content
            .items
            .add_item(create_healing_potion(potion_id, 10, false))
            .unwrap();

        let mut ch = Character::new(
            "Explorer".to_string(),
            "human".to_string(),
            "none".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        ch.inventory.add_item(potion_id, 1).unwrap();

        let result = validate_item_use_slot(&ch, 0, &content, false);
        assert!(
            result.is_ok(),
            "out-of-combat validation must accept non-combat-usable items; got: {result:?}"
        );
    }

    /// Regression: `validate_item_use_slot` with `in_combat = true` must
    /// reject items where `is_combat_usable: false`.
    #[test]
    fn test_validate_in_combat_rejects_non_combat_usable() {
        let mut content = ContentDatabase::new();
        let potion_id: ItemId = 212;
        content
            .items
            .add_item(create_healing_potion(potion_id, 10, false))
            .unwrap();

        let mut ch = Character::new(
            "Fighter".to_string(),
            "human".to_string(),
            "none".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        ch.inventory.add_item(potion_id, 1).unwrap();

        let result = validate_item_use_slot(&ch, 0, &content, true);
        assert!(
            matches!(result, Err(ItemUseError::NotUsableInCombat)),
            "in-combat validation must reject non-combat-usable items; got: {result:?}"
        );
    }

    /// Verify that BoostResistance flows correctly through the combat path
    /// (exercises `create_boost_resistance_potion` helper).
    #[test]
    fn test_execute_item_use_boost_resistance_via_combat_path() {
        use crate::domain::items::types::ResistanceType;

        let potion_id: ItemId = 213;
        let mut content = ContentDatabase::new();
        content
            .items
            .add_item(create_boost_resistance_potion(
                potion_id,
                ResistanceType::Fire,
                20,
            ))
            .unwrap();

        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);
        let mut pc = Character::new(
            "Mage".to_string(),
            "human".to_string(),
            "sorcerer".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        let fire_before = pc.resistances.fire.current;
        let fire_base = pc.resistances.fire.base;
        pc.inventory.add_item(potion_id, 1).unwrap();
        cs.add_player(pc);

        let mut rng = rand::rng();
        let res = execute_item_use_by_slot(
            &mut cs,
            CombatantId::Player(0),
            0,
            CombatantId::Player(0),
            &content,
            &mut rng,
        )
        .expect("BoostResistance use should succeed");

        assert!(res.success);
        if let Some(Combatant::Player(pc_after)) = cs.get_combatant(&CombatantId::Player(0)) {
            assert_eq!(
                pc_after.resistances.fire.current,
                fire_before.saturating_add(20),
                "fire resistance current should increase by 20"
            );
            assert_eq!(
                pc_after.resistances.fire.base, fire_base,
                "fire resistance base must not change"
            );
        } else {
            panic!("player should still be present");
        }
    }

    /// Verify that BoostAttribute flows correctly through the combat path
    /// (exercises `create_boost_attribute_potion` helper).
    #[test]
    fn test_execute_item_use_boost_attribute_via_combat_path() {
        let potion_id: ItemId = 214;
        let mut content = ContentDatabase::new();
        content
            .items
            .add_item(create_boost_attribute_potion(
                potion_id,
                AttributeType::Luck,
                7,
            ))
            .unwrap();

        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);
        let mut pc = Character::new(
            "Rogue".to_string(),
            "human".to_string(),
            "robber".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        let luck_before = pc.stats.luck.current;
        let luck_base = pc.stats.luck.base;
        pc.inventory.add_item(potion_id, 1).unwrap();
        cs.add_player(pc);

        let mut rng = rand::rng();
        let res = execute_item_use_by_slot(
            &mut cs,
            CombatantId::Player(0),
            0,
            CombatantId::Player(0),
            &content,
            &mut rng,
        )
        .expect("BoostAttribute use should succeed");

        assert!(res.success);
        if let Some(Combatant::Player(pc_after)) = cs.get_combatant(&CombatantId::Player(0)) {
            assert_eq!(
                pc_after.stats.luck.current,
                luck_before.saturating_add(7),
                "luck current should increase by 7"
            );
            assert_eq!(
                pc_after.stats.luck.base, luck_base,
                "luck base must not change"
            );
        } else {
            panic!("player should still be present");
        }
    }

    // ------------------------------------------------------------------
    // Phase 4: Cross-mode regression tests
    // ------------------------------------------------------------------

    /// After the Phase 1 refactor, `execute_item_use_by_slot` with an item
    /// whose `is_combat_usable: false` must still return
    /// `Err(ItemUseError::NotUsableInCombat)`.  This regression test confirms
    /// the combat gate is not broken by the delegation to `apply_consumable_effect`.
    #[test]
    fn test_combat_still_rejects_non_combat_usable() {
        let potion_id: ItemId = 215;
        let mut content = ContentDatabase::new();
        // Create a potion that is explicitly NOT usable in combat.
        content
            .items
            .add_item(create_healing_potion(potion_id, 10, false))
            .unwrap();

        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);
        let mut pc = Character::new(
            "Paladin".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        pc.inventory.add_item(potion_id, 1).unwrap();
        cs.add_player(pc);

        let mut rng = rand::rng();
        let result = execute_item_use_by_slot(
            &mut cs,
            CombatantId::Player(0),
            0,
            CombatantId::Player(0),
            &content,
            &mut rng,
        );

        assert!(
            matches!(result, Err(ItemUseError::NotUsableInCombat)),
            "combat path must reject is_combat_usable:false items; got: {result:?}"
        );
    }

    /// After Phase 1, `BoostAttribute` applied via `execute_item_use_by_slot`
    /// must produce the same stat delta as a direct call to
    /// `apply_consumable_effect`.  This verifies the shared helper produces
    /// consistent results on both paths.
    #[test]
    fn test_combat_boost_attribute_via_shared_helper() {
        let potion_id: ItemId = 216;
        let boost: i8 = 8;
        let attr = AttributeType::Intellect;

        // --- Baseline via pure helper ---
        let mut ref_pc = Character::new(
            "Sage".to_string(),
            "human".to_string(),
            "sorcerer".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        let before_intellect = ref_pc.stats.intellect.current;
        let ref_result = apply_consumable_effect(
            &mut ref_pc,
            &ConsumableData {
                effect: ConsumableEffect::BoostAttribute(attr, boost),
                is_combat_usable: true,
                duration_minutes: None,
            },
        );
        let expected_delta = ref_result.attribute_delta;
        let expected_current = ref_pc.stats.intellect.current;

        // --- Via combat path ---
        let mut content = ContentDatabase::new();
        content
            .items
            .add_item(create_boost_attribute_potion(potion_id, attr, boost))
            .unwrap();

        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);
        let mut pc = Character::new(
            "Sage".to_string(),
            "human".to_string(),
            "sorcerer".to_string(),
            Sex::Female,
            Alignment::Good,
        );
        // Ensure both start at the same current value.
        assert_eq!(pc.stats.intellect.current, before_intellect);
        pc.inventory.add_item(potion_id, 1).unwrap();
        cs.add_player(pc);

        let mut rng = rand::rng();
        let res = execute_item_use_by_slot(
            &mut cs,
            CombatantId::Player(0),
            0,
            CombatantId::Player(0),
            &content,
            &mut rng,
        )
        .expect("BoostAttribute use in combat should succeed");

        assert!(res.success);
        assert_eq!(
            expected_delta, boost as i16,
            "attribute_delta sanity: helper must report the requested boost"
        );
        if let Some(Combatant::Player(pc_after)) = cs.get_combatant(&CombatantId::Player(0)) {
            assert_eq!(
                pc_after.stats.intellect.current, expected_current,
                "combat-path intellect must match pure-helper baseline"
            );
        } else {
            panic!("player should still be present after item use");
        }
    }

    /// After Phase 1, `BoostResistance` applied via `execute_item_use_by_slot`
    /// must produce the same resistance delta as a direct call to
    /// `apply_consumable_effect`.  This verifies the shared helper produces
    /// consistent results on both paths.
    #[test]
    fn test_combat_boost_resistance_via_shared_helper() {
        let potion_id: ItemId = 217;
        let boost: i8 = 12;
        let res_type = ResistanceType::Cold;

        // --- Baseline via pure helper ---
        let mut ref_pc = Character::new(
            "Frost".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        let before_cold = ref_pc.resistances.cold.current;
        let ref_result = apply_consumable_effect(
            &mut ref_pc,
            &ConsumableData {
                effect: ConsumableEffect::BoostResistance(res_type, boost),
                is_combat_usable: true,
                duration_minutes: None,
            },
        );
        let expected_delta = ref_result.resistance_delta;
        let expected_cold = ref_pc.resistances.cold.current;

        // --- Via combat path ---
        let mut content = ContentDatabase::new();
        content
            .items
            .add_item(create_boost_resistance_potion(potion_id, res_type, boost))
            .unwrap();

        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);
        let mut pc = Character::new(
            "Frost".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Neutral,
        );
        assert_eq!(pc.resistances.cold.current, before_cold);
        pc.inventory.add_item(potion_id, 1).unwrap();
        cs.add_player(pc);

        let mut rng = rand::rng();
        let res = execute_item_use_by_slot(
            &mut cs,
            CombatantId::Player(0),
            0,
            CombatantId::Player(0),
            &content,
            &mut rng,
        )
        .expect("BoostResistance use in combat should succeed");

        assert!(res.success);
        assert_eq!(
            expected_delta, boost as i16,
            "resistance_delta sanity: helper must report the requested boost"
        );
        if let Some(Combatant::Player(pc_after)) = cs.get_combatant(&CombatantId::Player(0)) {
            assert_eq!(
                pc_after.resistances.cold.current, expected_cold,
                "combat-path cold resistance must match pure-helper baseline"
            );
        } else {
            panic!("player should still be present after item use");
        }
    }

    #[test]
    fn test_invalid_slot_returns_error() {
        let content = ContentDatabase::new();
        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);
        let pc = Character::new(
            "NoItems".to_string(),
            "human".to_string(),
            "none".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        cs.add_player(pc);

        let mut rng = rand::rng();
        let res = execute_item_use_by_slot(
            &mut cs,
            CombatantId::Player(0),
            0,
            CombatantId::Player(0),
            &content,
            &mut rng,
        );

        assert!(matches!(res, Err(ItemUseError::InventorySlotInvalid(0))));
    }

    // -----------------------------------------------------------------------
    // Phase 3 regression tests — timed boosts via combat path
    // -----------------------------------------------------------------------

    /// Combat use of a `BoostAttribute` item with `duration_minutes: Some(30)`
    /// should register a `TimedStatBoost` on the character (timed path).
    #[test]
    fn test_execute_item_use_timed_attribute_in_combat_applies_boost() {
        let mut content = ContentDatabase::new();
        let potion_id: ItemId = 220;
        let potion = Item {
            id: potion_id,
            name: "Swift Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::BoostAttribute(AttributeType::Speed, 5),
                is_combat_usable: true,
                duration_minutes: Some(30),
            }),
            base_cost: 10,
            sell_cost: 5,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        content.items.add_item(potion).unwrap();

        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);
        let mut pc = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "none".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        pc.inventory.add_item(potion_id, 1).unwrap();
        cs.add_player(pc);

        let mut rng = rand::rng();
        let res = execute_item_use_by_slot(
            &mut cs,
            CombatantId::Player(0),
            0,
            CombatantId::Player(0),
            &content,
            &mut rng,
        )
        .unwrap();
        assert!(res.success);

        if let Some(Combatant::Player(pc_after)) = cs.get_combatant(&CombatantId::Player(0)) {
            assert_eq!(
                pc_after.timed_stat_boosts.len(),
                1,
                "timed boost should be registered"
            );
            assert_eq!(
                pc_after.timed_stat_boosts[0].minutes_remaining, 30,
                "minutes_remaining should be 30"
            );
            assert_eq!(
                pc_after.timed_stat_boosts[0].amount, 5,
                "boost amount should be 5"
            );
        } else {
            panic!("player should exist");
        }
    }

    /// Combat use of a `BoostResistance` item with `duration_minutes: Some(60)`
    /// should still mutate resistances directly (not `active_spells`).
    #[test]
    fn test_execute_item_use_resistance_in_combat_is_permanent() {
        let mut content = ContentDatabase::new();
        let potion_id: ItemId = 221;
        let potion = Item {
            id: potion_id,
            name: "Fire Shield Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::BoostResistance(ResistanceType::Fire, 25),
                is_combat_usable: true,
                duration_minutes: Some(60),
            }),
            base_cost: 20,
            sell_cost: 10,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
        };
        content.items.add_item(potion).unwrap();

        let mut cs = CombatState::new(crate::domain::combat::types::Handicap::Even);
        let mut pc = Character::new(
            "Pyro".to_string(),
            "human".to_string(),
            "none".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let fire_before = pc.resistances.fire.current;
        pc.inventory.add_item(potion_id, 1).unwrap();
        cs.add_player(pc);

        let mut rng = rand::rng();
        execute_item_use_by_slot(
            &mut cs,
            CombatantId::Player(0),
            0,
            CombatantId::Player(0),
            &content,
            &mut rng,
        )
        .unwrap();

        if let Some(Combatant::Player(pc_after)) = cs.get_combatant(&CombatantId::Player(0)) {
            assert!(
                pc_after.resistances.fire.current > fire_before,
                "combat path must still permanently mutate fire resistance"
            );
        } else {
            panic!("player should exist");
        }
    }
}
