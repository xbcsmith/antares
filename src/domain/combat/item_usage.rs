/*
SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
SPDX-License-Identifier: Apache-2.0
*/

//! Combat-oriented item usage helpers
//!
//! This module provides combat-facing helpers for validating and executing
//! item usage (Phase 8: Item Usage System).
//!
//! Responsibilities:
//! - Validate that an inventory slot contains a usable item (consumable,
//!   combat-usable flag, alignment, race/proficiency restrictions as needed).
//! - Execute consumable effects (HealHp, RestoreSp, CureCondition, BoostAttribute)
//!   against a target in `CombatState`.
//! - Consume inventory charges (decrement or remove slot).
//! - Advance the combat turn and apply round/condition ticks.
//!
//! Design notes:
//! - Consumables are the primary item type supported in combat for this phase.
//! - Only minimal behavior is implemented for effects (no durations on boosts).
//! - Validation only queries class/race definitions when required (e.g., when an
//!   item has a proficiency requirement or tags to validate); this keeps
//!   tests lightweight for common consumables which typically have no profs/tags.

use crate::domain::combat::engine::CombatState;
use crate::domain::combat::types::CombatantId;
use crate::domain::items::types::{AttributeType, ConsumableEffect, ItemType};
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

    // Consumable?
    let consumable = match &item.item_type {
        ItemType::Consumable(data) => data,
        _ => return Err(ItemUseError::NotConsumable),
    };

    // Combat use allowed?
    if in_combat && !consumable.is_combat_usable {
        return Err(ItemUseError::NotUsableInCombat);
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
/// This:
/// - Validates the use (via `validate_item_use_slot`)
/// - Applies the consumable effect to the target
/// - Consumes inventory charges / removes the slot
/// - Advances the combat turn and ticks round effects
///
/// # Arguments
///
/// * `combat_state` - mutable combat state to apply effects to
/// * `user` - Combatant who is using the item (must be a player)
/// * `inventory_index` - Index into the user's inventory to use
/// * `target` - Target combatant for the item's effect
/// * `content` - Content DB for items and conditions
/// * `rng` - RNG (present for parity with spell APIs; not used by current consumable effects)
///
/// # Returns
///
/// Returns `Ok(ItemUseResult)` on success or `Err(ItemUseError)` on failure.
pub fn execute_item_use_by_slot<R: Rng>(
    combat_state: &mut CombatState,
    user: CombatantId,
    inventory_index: usize,
    target: CombatantId,
    content: &ContentDatabase,
    _rng: &mut R,
) -> Result<ItemUseResult, ItemUseError> {
    use crate::domain::combat::engine::Combatant;

    // Ensure user exists & is a player - take a snapshot for validation
    let user_snapshot = match combat_state.get_combatant(&user) {
        Some(Combatant::Player(pc)) => pc.as_ref().clone(),
        _ => return Err(ItemUseError::InvalidTarget),
    };

    // Validate using snapshot (no active mutable borrow on combat_state)
    validate_item_use_slot(&user_snapshot, inventory_index, content, true)?;

    // Perform the effect by splitting the operation into two phases to avoid
    // simultaneous mutable borrows of `combat_state`:
    //
    // Phase A: Mutably borrow the user only long enough to consume the inventory
    //          charge and capture the consumable's effect.
    // Phase B: Re-borrow the combat state to apply the captured effect to the
    //          target (healing, SP restore, condition cure, or attribute boost).
    let (result_message, effected_indices, damage_amount, healing_amount, applied_conds) = {
        // Phase A: consume charge and capture effect
        let (item_name, effect) = {
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

            // Consume one charge from the slot (or remove it)
            if slot.charges > 1 {
                slot.charges -= 1;
            } else {
                let _ = pc.inventory.remove_item(inventory_index);
            }

            (item.name.clone(), consumable.effect)
        };

        // Phase B: apply captured effect to target (fresh mutable borrows)
        let mut effected_indices: Vec<usize> = Vec::new();
        let total_damage: i32 = 0;
        let mut total_healing: i32 = 0;
        let mut applied_conditions: Vec<String> = Vec::new();

        match effect {
            ConsumableEffect::HealHp(amount) => match combat_state.get_combatant_mut(&target) {
                Some(Combatant::Player(pc_target)) => {
                    if let CombatantId::Player(idx) = target {
                        let pre = pc_target.hp.current as i32;
                        pc_target.hp.modify(amount as i32);
                        if pc_target.hp.current > pc_target.hp.base {
                            pc_target.hp.current = pc_target.hp.base;
                        }
                        let post = pc_target.hp.current as i32;
                        let healed = post - pre;
                        if healed > 0 {
                            total_healing += healed;
                            effected_indices.push(idx);
                        }
                    }
                }
                _ => return Err(ItemUseError::InvalidTarget),
            },

            ConsumableEffect::RestoreSp(amount) => match combat_state.get_combatant_mut(&target) {
                Some(Combatant::Player(pc_target)) => {
                    if let CombatantId::Player(idx) = target {
                        let pre_sp = pc_target.sp.current as i32;
                        pc_target.sp.modify(amount as i32);
                        if pc_target.sp.current > pc_target.sp.base {
                            pc_target.sp.current = pc_target.sp.base;
                        }
                        let post_sp = pc_target.sp.current as i32;
                        let restored = post_sp - pre_sp;
                        if restored > 0 {
                            total_healing += restored;
                            effected_indices.push(idx);
                        }
                    }
                }
                _ => return Err(ItemUseError::InvalidTarget),
            },

            ConsumableEffect::CureCondition(flags) => {
                match combat_state.get_combatant_mut(&target) {
                    Some(Combatant::Player(pc_target)) => {
                        if let CombatantId::Player(idx) = target {
                            pc_target.conditions.remove(flags);
                            effected_indices.push(idx);
                            applied_conditions.push(format!("cleared_flags:{:#X}", flags));
                        }
                    }
                    _ => return Err(ItemUseError::InvalidTarget),
                }
            }

            ConsumableEffect::BoostAttribute(attr, amount) => {
                match combat_state.get_combatant_mut(&target) {
                    Some(Combatant::Player(pc_target)) => {
                        if let CombatantId::Player(idx) = target {
                            match attr {
                                AttributeType::Might => pc_target.stats.might.modify(amount as i16),
                                AttributeType::Intellect => {
                                    pc_target.stats.intellect.modify(amount as i16)
                                }
                                AttributeType::Personality => {
                                    pc_target.stats.personality.modify(amount as i16)
                                }
                                AttributeType::Endurance => {
                                    pc_target.stats.endurance.modify(amount as i16)
                                }
                                AttributeType::Speed => pc_target.stats.speed.modify(amount as i16),
                                AttributeType::Accuracy => {
                                    pc_target.stats.accuracy.modify(amount as i16)
                                }
                                AttributeType::Luck => pc_target.stats.luck.modify(amount as i16),
                            }
                            effected_indices.push(idx);
                        }
                    }
                    _ => return Err(ItemUseError::InvalidTarget),
                }
            }
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
    use crate::domain::items::types::{ConsumableData, ConsumableEffect, Item, ItemType};
    use crate::sdk::database::ContentDatabase;

    /// Helper: create a simple healing potion item
    fn create_healing_potion(id: ItemId, amount: u16, combat_usable: bool) -> Item {
        Item {
            id,
            name: "Healing Potion".to_string(),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHp(amount),
                is_combat_usable: combat_usable,
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
}
