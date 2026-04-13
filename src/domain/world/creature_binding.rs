// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! `CreatureBound` trait — unified creature asset binding for definition types
//!
//! This module defines the [`CreatureBound`] trait, which is implemented by every
//! definition type that may carry a reference to a [`CreatureDefinition`] in the
//! creature registry.  Centralising the accessor behind a trait means rendering
//! systems can be written generically (or at least consistently) instead of
//! reaching into each definition type's fields directly.
//!
//! # Implementing Types
//!
//! | Type | Field | Source module |
//! |---|---|---|
//! | [`Monster`]            | `creature_id: Option<CreatureId>` | `domain::combat::monster`   |
//! | [`MonsterDefinition`]  | `creature_id: Option<CreatureId>` | `domain::combat::database`  |
//! | [`NpcDefinition`]      | `creature_id: Option<CreatureId>` | `domain::world::npc`        |
//! | [`CharacterDefinition`]| `creature_id: Option<CreatureId>` | `domain::character_definition` |
//!
//! Note: The SDK's `ContentDatabase` stores runtime [`Monster`] instances (converted
//! from [`MonsterDefinition`] via `to_monster()`), so the trait must be implemented
//! for both the definition type and the runtime type.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/unified_creature_asset_binding_implementation_plan.md`
//! for the design rationale.

use crate::domain::character_definition::CharacterDefinition;
use crate::domain::combat::database::MonsterDefinition;
use crate::domain::combat::monster::Monster;
use crate::domain::types::CreatureId;
use crate::domain::world::npc::NpcDefinition;

// ===== Trait Definition =====

/// Implemented by any definition type that may carry a reference to a
/// `CreatureDefinition` in the creature registry.
///
/// The trait provides a single accessor, [`creature_id`](CreatureBound::creature_id),
/// which returns the optional [`CreatureId`] that links the definition to a mesh
/// asset.  Rendering systems should call this method rather than accessing the
/// underlying field directly so that the call-sites remain consistent and
/// refactoring the field name in the future requires only one change.
///
/// # Examples
///
/// ```
/// use antares::domain::world::creature_binding::CreatureBound;
/// use antares::domain::combat::database::MonsterDefinition;
/// use antares::domain::combat::monster::{LootTable, MonsterCondition, MonsterResistances};
/// use antares::domain::character::{AttributePair, AttributePair16, Stats};
///
/// let def = MonsterDefinition {
///     id: 1,
///     name: "Goblin".to_string(),
///     stats: Stats::new(8, 6, 6, 8, 10, 8, 6),
///     hp: AttributePair16::new(12),
///     ac: AttributePair::new(5),
///     attacks: vec![],
///     flee_threshold: 5,
///     special_attack_threshold: 0,
///     resistances: MonsterResistances::new(),
///     can_regenerate: false,
///     can_advance: true,
///     is_undead: false,
///     magic_resistance: 0,
///     loot: LootTable::default(),
///     creature_id: Some(7),
///     conditions: MonsterCondition::Normal,
///     active_conditions: vec![],
///     has_acted: false,
/// };
///
/// assert_eq!(def.creature_id(), Some(7));
/// ```
pub trait CreatureBound {
    /// Returns the optional [`CreatureId`] that links this definition to a mesh
    /// asset in the creature registry.
    ///
    /// Returns `None` when no visual binding has been set for this definition.
    fn creature_id(&self) -> Option<CreatureId>;
}

// ===== Implementations =====

/// [`CreatureBound`] for [`MonsterDefinition`]
///
/// # Examples
///
/// ```
/// use antares::domain::world::creature_binding::CreatureBound;
/// use antares::domain::combat::database::MonsterDefinition;
/// use antares::domain::combat::monster::{LootTable, MonsterCondition, MonsterResistances};
/// use antares::domain::character::{AttributePair, AttributePair16, Stats};
///
/// let def = MonsterDefinition {
///     id: 42,
///     name: "Troll".to_string(),
///     stats: Stats::new(18, 6, 6, 16, 8, 8, 4),
///     hp: AttributePair16::new(40),
///     ac: AttributePair::new(4),
///     attacks: vec![],
///     flee_threshold: 10,
///     special_attack_threshold: 0,
///     resistances: MonsterResistances::new(),
///     can_regenerate: true,
///     can_advance: true,
///     is_undead: false,
///     magic_resistance: 0,
///     loot: LootTable::default(),
///     creature_id: Some(3),
///     conditions: MonsterCondition::Normal,
///     active_conditions: vec![],
///     has_acted: false,
/// };
///
/// assert_eq!(def.creature_id(), Some(3));
/// ```
impl CreatureBound for MonsterDefinition {
    fn creature_id(&self) -> Option<CreatureId> {
        self.creature_id
    }
}

/// [`CreatureBound`] for the runtime [`Monster`] struct
///
/// The SDK's `ContentDatabase` converts `MonsterDefinition` into `Monster`
/// instances at load time (via `to_monster()`).  The `monster_def.creature_id`
/// value is copied across during that conversion, so the trait accessor works
/// identically on both the definition type and the runtime type.
///
/// # Examples
///
/// ```
/// use antares::domain::world::creature_binding::CreatureBound;
/// use antares::domain::combat::monster::{Monster, LootTable, MonsterCondition, MonsterResistances, AiBehavior};
/// use antares::domain::character::{AttributePair, AttributePair16, Stats};
///
/// let monster = Monster {
///     id: 5,
///     name: "Skeleton".to_string(),
///     stats: Stats::new(10, 4, 4, 10, 8, 8, 4),
///     hp: AttributePair16::new(10),
///     ac: AttributePair::new(5),
///     attacks: vec![],
///     loot: LootTable::default(),
///     flee_threshold: 0,
///     special_attack_threshold: 0,
///     resistances: MonsterResistances::undead(),
///     can_regenerate: false,
///     can_advance: false,
///     is_undead: true,
///     magic_resistance: 0,
///     ai_behavior: AiBehavior::default(),
///     creature_id: Some(5),
///     conditions: MonsterCondition::Normal,
///     active_conditions: vec![],
///     has_acted: false,
/// };
///
/// assert_eq!(monster.creature_id(), Some(5));
/// ```
impl CreatureBound for Monster {
    fn creature_id(&self) -> Option<CreatureId> {
        self.creature_id
    }
}

/// [`CreatureBound`] for [`NpcDefinition`]
///
/// # Examples
///
/// ```
/// use antares::domain::world::creature_binding::CreatureBound;
/// use antares::domain::world::npc::NpcDefinition;
///
/// let def = NpcDefinition {
///     id: "village_elder".to_string(),
///     name: "Elder Theron".to_string(),
///     description: "The wise village elder".to_string(),
///     portrait_id: "elder.png".to_string(),
///     dialogue_id: None,
///     creature_id: Some(1000),
///     sprite: None,
///     quest_ids: vec![],
///     faction: None,
///     is_merchant: false,
///     is_innkeeper: false,
///     is_priest: false,
///     stock_template: None,
///     service_catalog: None,
///     economy: None,
/// };
///
/// assert_eq!(def.creature_id(), Some(1000));
/// ```
impl CreatureBound for NpcDefinition {
    fn creature_id(&self) -> Option<CreatureId> {
        self.creature_id
    }
}

/// [`CreatureBound`] for [`CharacterDefinition`]
///
/// # Examples
///
/// ```
/// use antares::domain::world::creature_binding::CreatureBound;
/// use antares::domain::character_definition::CharacterDefinition;
///
/// use antares::domain::character::Sex;
/// use antares::domain::character::Alignment;
///
/// let def = CharacterDefinition::new(
///     "hero_001".to_string(),
///     "Brave Hero".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
///
/// // By default no creature binding is set
/// assert_eq!(def.creature_id(), None);
/// ```
impl CreatureBound for CharacterDefinition {
    fn creature_id(&self) -> Option<CreatureId> {
        self.creature_id
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{AttributePair, AttributePair16, Stats};
    use crate::domain::character_definition::CharacterDefinition;
    use crate::domain::combat::database::MonsterDefinition;
    use crate::domain::combat::monster::{
        AiBehavior, LootTable, Monster, MonsterCondition, MonsterResistances,
    };
    use crate::domain::world::npc::NpcDefinition;

    // ─── helpers ────────────────────────────────────────────────────────────

    fn make_monster_def(creature_id: Option<CreatureId>) -> MonsterDefinition {
        MonsterDefinition {
            id: 1,
            name: "Test Monster".to_string(),
            stats: Stats::new(10, 8, 6, 10, 10, 10, 8),
            hp: AttributePair16::new(20),
            ac: AttributePair::new(5),
            attacks: vec![],
            flee_threshold: 5,
            special_attack_threshold: 0,
            resistances: MonsterResistances::new(),
            can_regenerate: false,
            can_advance: true,
            is_undead: false,
            magic_resistance: 0,
            loot: LootTable::default(),
            creature_id,
            conditions: MonsterCondition::Normal,
            active_conditions: vec![],
            has_acted: false,
        }
    }

    fn make_npc_def(creature_id: Option<CreatureId>) -> NpcDefinition {
        NpcDefinition {
            id: "test_npc".to_string(),
            name: "Test NPC".to_string(),
            description: "A test NPC".to_string(),
            portrait_id: "test.png".to_string(),
            dialogue_id: None,
            creature_id,
            sprite: None,
            quest_ids: vec![],
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            is_trainer: false,
            training_fee_base: None,
            training_fee_multiplier: None,
        }
    }

    fn make_runtime_monster(creature_id: Option<CreatureId>) -> Monster {
        Monster {
            id: 99,
            name: "Runtime Monster".to_string(),
            stats: Stats::new(10, 8, 6, 10, 10, 10, 8),
            hp: AttributePair16::new(20),
            ac: AttributePair::new(5),
            attacks: vec![],
            loot: LootTable::default(),
            flee_threshold: 5,
            special_attack_threshold: 0,
            resistances: MonsterResistances::new(),
            can_regenerate: false,
            can_advance: true,
            is_undead: false,
            magic_resistance: 0,
            ai_behavior: AiBehavior::default(),
            creature_id,
            conditions: MonsterCondition::Normal,
            active_conditions: vec![],
            has_acted: false,
            spells: Vec::new(),
            spell_cooldown: 0,
        }
    }

    fn make_character_def(creature_id: Option<CreatureId>) -> CharacterDefinition {
        let mut def = CharacterDefinition::new(
            "test_char".to_string(),
            "Test Character".to_string(),
            "human".to_string(),
            "knight".to_string(),
            crate::domain::character::Sex::Male,
            crate::domain::character::Alignment::Good,
        );
        def.creature_id = creature_id;
        def
    }

    // ─── Monster (runtime) ──────────────────────────────────────────────────

    /// `Monster::creature_id()` returns the bound id when present.
    #[test]
    fn test_creature_bound_runtime_monster_some() {
        let monster = make_runtime_monster(Some(3));
        assert_eq!(monster.creature_id(), Some(3));
    }

    /// `Monster::creature_id()` returns `None` when no binding exists.
    #[test]
    fn test_creature_bound_runtime_monster_none() {
        let monster = make_runtime_monster(None);
        assert_eq!(monster.creature_id(), None);
    }

    // ─── MonsterDefinition ──────────────────────────────────────────────────

    /// `MonsterDefinition::creature_id()` returns the bound id when present.
    #[test]
    fn test_creature_bound_monster_some() {
        let def = make_monster_def(Some(3));
        assert_eq!(def.creature_id(), Some(3));
    }

    /// `MonsterDefinition::creature_id()` returns `None` when no binding exists.
    #[test]
    fn test_creature_bound_monster_none() {
        let def = make_monster_def(None);
        assert_eq!(def.creature_id(), None);
    }

    // ─── NpcDefinition ──────────────────────────────────────────────────────

    /// `NpcDefinition::creature_id()` returns the bound id when present.
    #[test]
    fn test_creature_bound_npc_some() {
        let def = make_npc_def(Some(1000));
        assert_eq!(def.creature_id(), Some(1000));
    }

    /// `NpcDefinition::creature_id()` returns `None` when no binding exists.
    #[test]
    fn test_creature_bound_npc_none() {
        let def = make_npc_def(None);
        assert_eq!(def.creature_id(), None);
    }

    // ─── CharacterDefinition ────────────────────────────────────────────────

    /// `CharacterDefinition::creature_id()` returns the bound id when present.
    #[test]
    fn test_creature_bound_character_some() {
        let def = make_character_def(Some(2000));
        assert_eq!(def.creature_id(), Some(2000));
    }

    /// `CharacterDefinition::creature_id()` returns `None` when no binding exists.
    #[test]
    fn test_creature_bound_character_none() {
        let def = make_character_def(None);
        assert_eq!(def.creature_id(), None);
    }

    // ─── Cross-type consistency ─────────────────────────────────────────────

    /// All four implementing types with the same `creature_id` value return
    /// identical `Option<CreatureId>` from the trait method.
    ///
    /// This test verifies that the trait accessor is a thin, consistent wrapper
    /// regardless of the concrete implementing type — i.e., there is no
    /// accidental field aliasing or value transformation.  The runtime `Monster`
    /// type is included because the SDK stores converted `Monster` instances
    /// rather than `MonsterDefinition` objects.
    #[test]
    fn test_creature_bound_all_three_types_consistent() {
        const SHARED_CREATURE_ID: CreatureId = 42;

        let monster_def = make_monster_def(Some(SHARED_CREATURE_ID));
        let runtime_monster = make_runtime_monster(Some(SHARED_CREATURE_ID));
        let npc = make_npc_def(Some(SHARED_CREATURE_ID));
        let character = make_character_def(Some(SHARED_CREATURE_ID));

        let monster_def_result = monster_def.creature_id();
        let runtime_monster_result = runtime_monster.creature_id();
        let npc_result = npc.creature_id();
        let character_result = character.creature_id();

        assert_eq!(monster_def_result, Some(SHARED_CREATURE_ID));
        assert_eq!(runtime_monster_result, Some(SHARED_CREATURE_ID));
        assert_eq!(npc_result, Some(SHARED_CREATURE_ID));
        assert_eq!(character_result, Some(SHARED_CREATURE_ID));

        // All four must be identical to each other
        assert_eq!(monster_def_result, runtime_monster_result);
        assert_eq!(runtime_monster_result, npc_result);
        assert_eq!(npc_result, character_result);
    }
}
