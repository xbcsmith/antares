// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Monster definitions and conditions
//!
//! This module defines monster data structures, resistances, and condition tracking
//! for combat encounters.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.4 for complete specifications.

use crate::domain::character::{AttributePair, AttributePair16, Stats};
use crate::domain::combat::types::Attack;
use crate::domain::types::MonsterId;
use serde::{Deserialize, Serialize};

// ===== MonsterResistances =====

/// Monster resistances to various damage types and effects
///
/// Each resistance is a boolean flag indicating immunity.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::monster::MonsterResistances;
///
/// let undead_resistances = MonsterResistances {
///     physical: false,
///     fire: false,
///     cold: true,
///     electricity: false,
///     energy: false,
///     paralysis: true,
///     fear: true,
///     sleep: true,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonsterResistances {
    /// Immune to physical damage
    pub physical: bool,
    /// Immune to fire damage
    pub fire: bool,
    /// Immune to cold damage
    pub cold: bool,
    /// Immune to electricity damage
    pub electricity: bool,
    /// Immune to energy/magic damage
    pub energy: bool,
    /// Immune to paralysis effects
    pub paralysis: bool,
    /// Immune to fear effects
    pub fear: bool,
    /// Immune to sleep effects
    pub sleep: bool,
}

impl MonsterResistances {
    /// Creates a new MonsterResistances with no immunities
    pub fn new() -> Self {
        Self {
            physical: false,
            fire: false,
            cold: false,
            electricity: false,
            energy: false,
            paralysis: false,
            fear: false,
            sleep: false,
        }
    }

    /// Creates resistances for undead creatures
    pub fn undead() -> Self {
        Self {
            physical: false,
            fire: false,
            cold: true,
            electricity: false,
            energy: false,
            paralysis: true,
            fear: true,
            sleep: true,
        }
    }

    /// Creates resistances for elemental creatures
    pub fn elemental() -> Self {
        Self {
            physical: true,
            fire: false,
            cold: false,
            electricity: false,
            energy: false,
            paralysis: true,
            fear: true,
            sleep: true,
        }
    }
}

impl Default for MonsterResistances {
    fn default() -> Self {
        Self::new()
    }
}

// ===== MonsterCondition =====

/// Conditions that can affect monsters during combat
///
/// # Examples
///
/// ```
/// use antares::domain::combat::monster::MonsterCondition;
///
/// let condition = MonsterCondition::Normal;
/// assert!(matches!(condition, MonsterCondition::Normal));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MonsterCondition {
    /// Monster is operating normally
    #[default]
    Normal,
    /// Monster is paralyzed and cannot act
    Paralyzed,
    /// Monster is webbed and cannot move
    Webbed,
    /// Monster is held by magic
    Held,
    /// Monster is asleep
    Asleep,
    /// Monster is immune to mind-affecting spells
    Mindless,
    /// Monster cannot cast spells
    Silenced,
    /// Monster has reduced accuracy
    Blinded,
    /// Monster has reduced effectiveness
    Afraid,
    /// Monster is dead
    Dead,
}

impl MonsterCondition {
    /// Returns true if the monster can act
    pub fn can_act(&self) -> bool {
        matches!(
            self,
            MonsterCondition::Normal
                | MonsterCondition::Mindless
                | MonsterCondition::Silenced
                | MonsterCondition::Blinded
                | MonsterCondition::Afraid
        )
    }

    /// Returns true if the monster is incapacitated
    pub fn is_incapacitated(&self) -> bool {
        !self.can_act()
    }

    /// Returns true if the monster is dead
    pub fn is_dead(&self) -> bool {
        matches!(self, MonsterCondition::Dead)
    }
}

// ===== LootTable =====

/// Loot table for monsters
///
/// Defines what rewards a monster drops when defeated.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::monster::LootTable;
///
/// let loot = LootTable {
///     gold_min: 10,
///     gold_max: 50,
///     gems_min: 0,
///     gems_max: 2,
///     items: vec![],
///     experience: 100,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LootTable {
    /// Minimum gold dropped
    pub gold_min: u32,
    /// Maximum gold dropped
    pub gold_max: u32,
    /// Minimum gems dropped
    pub gems_min: u8,
    /// Maximum gems dropped
    pub gems_max: u8,
    /// Possible item drops (probability, item_id)
    pub items: Vec<(f32, u8)>,
    /// Experience points awarded
    pub experience: u32,
}

impl LootTable {
    /// Creates a new loot table
    pub fn new(gold_min: u32, gold_max: u32, gems_min: u8, gems_max: u8, experience: u32) -> Self {
        Self {
            gold_min,
            gold_max,
            gems_min,
            gems_max,
            items: Vec::new(),
            experience,
        }
    }

    /// Creates a loot table with no rewards
    pub fn none() -> Self {
        Self {
            gold_min: 0,
            gold_max: 0,
            gems_min: 0,
            gems_max: 0,
            items: Vec::new(),
            experience: 0,
        }
    }
}

impl Default for LootTable {
    fn default() -> Self {
        Self::none()
    }
}

// ===== Monster =====

/// Monster definition for combat encounters
///
/// # Examples
///
/// ```
/// use antares::domain::combat::monster::{Monster, MonsterCondition, MonsterResistances, LootTable};
/// use antares::domain::character::{Stats, AttributePair, AttributePair16};
/// use antares::domain::combat::types::Attack;
/// use antares::domain::types::DiceRoll;
///
/// let goblin = Monster {
///     id: 1,
///     name: "Goblin".to_string(),
///     stats: Stats::new(8, 6, 6, 8, 10, 8, 5),
///     hp: AttributePair16::new(7),
///     ac: AttributePair::new(6),
///     attacks: vec![Attack::physical(DiceRoll::new(1, 6, 0))],
///     loot: LootTable::new(5, 15, 0, 1, 25),
///     flee_threshold: 25,
///     special_attack_threshold: 0,
///     resistances: MonsterResistances::new(),
///     can_regenerate: false,
///     can_advance: false,
///     is_undead: false,
///     magic_resistance: 0,
///     conditions: MonsterCondition::Normal,
///     active_conditions: Vec::new(),
///     has_acted: false,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Monster {
    /// Monster identifier
    pub id: MonsterId,
    /// Monster name
    pub name: String,
    /// Monster stats (might, intellect, etc.)
    pub stats: Stats,
    /// Hit points (current/base)
    pub hp: AttributePair16,
    /// Armor class (higher is better)
    pub ac: AttributePair,
    /// List of attacks this monster can perform
    pub attacks: Vec<Attack>,
    /// Loot dropped when defeated
    pub loot: LootTable,
    /// HP percentage at which monster attempts to flee (0-100)
    #[serde(default)]
    pub flee_threshold: u8,
    /// Percentage chance of special attack per turn (0-100)
    #[serde(default)]
    pub special_attack_threshold: u8,
    /// Damage and effect resistances
    #[serde(default)]
    pub resistances: MonsterResistances,
    /// Can regenerate HP each round
    #[serde(default)]
    pub can_regenerate: bool,
    /// Can move forward in combat formation
    #[serde(default)]
    pub can_advance: bool,
    /// Is undead (affected by Turn Undead, etc.)
    #[serde(default)]
    pub is_undead: bool,
    /// Magic resistance percentage (0-100)
    #[serde(default)]
    pub magic_resistance: u8,
    /// Current condition (paralyzed, asleep, etc.) - runtime state, defaults to Normal
    #[serde(default)]
    pub conditions: MonsterCondition,
    /// Active data-driven conditions - runtime state, defaults to empty
    #[serde(default)]
    pub active_conditions: Vec<crate::domain::conditions::ActiveCondition>,
    /// Has acted this turn (for turn order tracking) - runtime state, defaults to false
    #[serde(default)]
    pub has_acted: bool,
}

impl Monster {
    /// Creates a new monster with the given parameters
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: MonsterId,
        name: String,
        stats: Stats,
        hp: u16,
        ac: u8,
        attacks: Vec<Attack>,
        loot: LootTable,
    ) -> Self {
        Self {
            id,
            name,
            stats,
            hp: AttributePair16::new(hp),
            ac: AttributePair::new(ac),
            attacks,
            loot,
            flee_threshold: 0,
            special_attack_threshold: 0,
            resistances: MonsterResistances::new(),
            can_regenerate: false,
            can_advance: false,
            is_undead: false,
            magic_resistance: 0,
            conditions: MonsterCondition::Normal,
            active_conditions: Vec::new(),
            has_acted: false,
        }
    }

    /// Returns true if the monster is alive
    pub fn is_alive(&self) -> bool {
        self.hp.current > 0 && !self.conditions.is_dead()
    }

    /// Returns true if the monster can act this turn
    pub fn can_act(&self) -> bool {
        self.is_alive() && self.conditions.can_act() && !self.has_acted
    }

    /// Returns true if the monster should attempt to flee
    pub fn should_flee(&self) -> bool {
        if self.flee_threshold == 0 {
            return false;
        }
        let hp_percent = (self.hp.current as f32 / self.hp.base as f32 * 100.0) as u8;
        hp_percent <= self.flee_threshold
    }

    /// Regenerates HP if capable
    pub fn regenerate(&mut self, amount: u16) {
        if self.can_regenerate && self.is_alive() {
            self.hp.modify(amount as i32);
        }
    }

    /// Applies damage to the monster
    ///
    /// Returns true if the monster died from this damage.
    pub fn take_damage(&mut self, damage: u16) -> bool {
        let old_hp = self.hp.current;
        self.hp.modify(-(damage as i32));

        if self.hp.current == 0 && old_hp > 0 {
            self.conditions = MonsterCondition::Dead;
            true
        } else {
            false
        }
    }

    /// Resets the monster's has_acted flag for a new turn
    pub fn reset_turn(&mut self) {
        self.has_acted = false;
    }

    /// Marks the monster as having acted this turn
    pub fn mark_acted(&mut self) {
        self.has_acted = true;
    }

    /// Adds a condition to the monster
    pub fn add_condition(&mut self, condition: crate::domain::conditions::ActiveCondition) {
        // Check if condition already exists, if so, refresh/overwrite it
        if let Some(existing) = self
            .active_conditions
            .iter_mut()
            .find(|c| c.condition_id == condition.condition_id)
        {
            existing.duration = condition.duration;
        } else {
            self.active_conditions.push(condition);
        }
    }

    /// Removes a condition by ID
    pub fn remove_condition(&mut self, condition_id: &str) {
        self.active_conditions
            .retain(|c| c.condition_id != condition_id);
    }

    /// Updates conditions based on round tick
    pub fn tick_conditions_round(&mut self) {
        self.active_conditions.retain_mut(|c| !c.tick_round());
    }

    /// Updates conditions based on minute tick
    pub fn tick_conditions_minute(&mut self) {
        self.active_conditions.retain_mut(|c| !c.tick_minute());
    }

    /// Calculates the total modifier from active conditions for a given attribute
    pub fn get_condition_modifier(
        &self,
        attribute: &str,
        condition_defs: &[crate::domain::conditions::ConditionDefinition],
    ) -> i16 {
        let mut total_modifier = 0i16;

        for active in &self.active_conditions {
            // Find the definition
            if let Some(def) = condition_defs.iter().find(|d| d.id == active.condition_id) {
                for effect in &def.effects {
                    if let crate::domain::conditions::ConditionEffect::AttributeModifier {
                        attribute: attr,
                        value,
                    } = effect
                    {
                        if attr == attribute {
                            let modified = (*value as f32 * active.magnitude).round() as i16;
                            total_modifier = total_modifier.saturating_add(modified);
                        }
                    }
                }
            }
        }

        total_modifier
    }

    /// Returns true if monster has a specific status effect from conditions
    pub fn has_status_effect(
        &self,
        status: &str,
        condition_defs: &[crate::domain::conditions::ConditionDefinition],
    ) -> bool {
        for active in &self.active_conditions {
            if let Some(def) = condition_defs.iter().find(|d| d.id == active.condition_id) {
                for effect in &def.effects {
                    if let crate::domain::conditions::ConditionEffect::StatusEffect(s) = effect {
                        if s == status {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::DiceRoll;

    #[test]
    fn test_monster_resistances_creation() {
        let resistances = MonsterResistances::new();
        assert!(!resistances.physical);
        assert!(!resistances.fire);
        assert!(!resistances.paralysis);
    }

    #[test]
    fn test_monster_resistances_undead() {
        let resistances = MonsterResistances::undead();
        assert!(resistances.cold);
        assert!(resistances.paralysis);
        assert!(resistances.fear);
        assert!(resistances.sleep);
    }

    #[test]
    fn test_monster_resistances_elemental() {
        let resistances = MonsterResistances::elemental();
        assert!(resistances.physical);
        assert!(resistances.paralysis);
        assert!(resistances.fear);
    }

    #[test]
    fn test_monster_condition_can_act() {
        assert!(MonsterCondition::Normal.can_act());
        assert!(MonsterCondition::Blinded.can_act());
        assert!(!MonsterCondition::Paralyzed.can_act());
        assert!(!MonsterCondition::Asleep.can_act());
        assert!(!MonsterCondition::Dead.can_act());
    }

    #[test]
    fn test_monster_condition_is_incapacitated() {
        assert!(!MonsterCondition::Normal.is_incapacitated());
        assert!(MonsterCondition::Paralyzed.is_incapacitated());
        assert!(MonsterCondition::Webbed.is_incapacitated());
    }

    #[test]
    fn test_monster_condition_is_dead() {
        assert!(!MonsterCondition::Normal.is_dead());
        assert!(!MonsterCondition::Paralyzed.is_dead());
        assert!(MonsterCondition::Dead.is_dead());
    }

    #[test]
    fn test_loot_table_creation() {
        let loot = LootTable::new(10, 50, 0, 2, 100);
        assert_eq!(loot.gold_min, 10);
        assert_eq!(loot.gold_max, 50);
        assert_eq!(loot.gems_min, 0);
        assert_eq!(loot.gems_max, 2);
        assert_eq!(loot.experience, 100);
    }

    #[test]
    fn test_loot_table_none() {
        let loot = LootTable::none();
        assert_eq!(loot.gold_min, 0);
        assert_eq!(loot.gold_max, 0);
        assert_eq!(loot.experience, 0);
    }

    #[test]
    fn test_monster_creation() {
        let stats = Stats::new(10, 8, 6, 10, 8, 7, 5);
        let attacks = vec![Attack::physical(DiceRoll::new(1, 6, 1))];
        let loot = LootTable::new(5, 20, 0, 1, 50);

        let monster = Monster::new(1, "Orc".to_string(), stats, 15, 5, attacks, loot);

        assert_eq!(monster.id, 1);
        assert_eq!(monster.name, "Orc");
        assert_eq!(monster.hp.base, 15);
        assert_eq!(monster.hp.current, 15);
        assert_eq!(monster.ac.base, 5);
        assert_eq!(monster.loot.experience, 50);
        assert!(!monster.has_acted);
    }

    #[test]
    fn test_monster_is_alive() {
        let stats = Stats::new(10, 8, 6, 10, 8, 7, 5);
        let attacks = vec![Attack::physical(DiceRoll::new(1, 6, 0))];
        let loot = LootTable::none();

        let mut monster = Monster::new(1, "Test".to_string(), stats, 20, 5, attacks, loot);
        assert!(monster.is_alive());

        monster.hp.current = 0;
        assert!(!monster.is_alive());
    }

    #[test]
    fn test_monster_can_act() {
        let stats = Stats::new(10, 8, 6, 10, 8, 7, 5);
        let attacks = vec![Attack::physical(DiceRoll::new(1, 6, 0))];
        let loot = LootTable::none();

        let mut monster = Monster::new(1, "Test".to_string(), stats, 20, 5, attacks, loot);
        assert!(monster.can_act());

        monster.has_acted = true;
        assert!(!monster.can_act());

        monster.has_acted = false;
        monster.conditions = MonsterCondition::Paralyzed;
        assert!(!monster.can_act());
    }

    #[test]
    fn test_monster_should_flee() {
        let stats = Stats::new(10, 8, 6, 10, 8, 7, 5);
        let attacks = vec![Attack::physical(DiceRoll::new(1, 6, 0))];
        let loot = LootTable::none();

        let mut monster = Monster::new(1, "Test".to_string(), stats, 20, 5, attacks, loot);
        monster.flee_threshold = 25;

        assert!(!monster.should_flee()); // At 100% HP

        monster.hp.current = 4; // 20% of 20
        assert!(monster.should_flee());
    }

    #[test]
    fn test_monster_regenerate() {
        let stats = Stats::new(10, 8, 6, 10, 8, 7, 5);
        let attacks = vec![Attack::physical(DiceRoll::new(1, 6, 0))];
        let loot = LootTable::none();

        let mut monster = Monster::new(1, "Troll".to_string(), stats, 30, 5, attacks, loot);
        monster.can_regenerate = true;
        monster.hp.current = 20;

        monster.regenerate(5);
        assert_eq!(monster.hp.current, 25);
    }

    #[test]
    fn test_monster_take_damage() {
        let stats = Stats::new(10, 8, 6, 10, 8, 7, 5);
        let attacks = vec![Attack::physical(DiceRoll::new(1, 6, 0))];
        let loot = LootTable::none();

        let mut monster = Monster::new(1, "Test".to_string(), stats, 20, 5, attacks, loot);

        let died = monster.take_damage(10);
        assert!(!died);
        assert_eq!(monster.hp.current, 10);

        let died = monster.take_damage(15);
        assert!(died);
        assert_eq!(monster.hp.current, 0);
        assert!(monster.conditions.is_dead());
    }

    #[test]
    fn test_monster_turn_tracking() {
        let stats = Stats::new(10, 8, 6, 10, 8, 7, 5);
        let attacks = vec![Attack::physical(DiceRoll::new(1, 6, 0))];
        let loot = LootTable::none();

        let mut monster = Monster::new(1, "Test".to_string(), stats, 20, 5, attacks, loot);

        assert!(!monster.has_acted);
        monster.mark_acted();
        assert!(monster.has_acted);
        monster.reset_turn();
        assert!(!monster.has_acted);
    }
}
