// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Combat type definitions
//!
//! This module defines the core types used in combat, including attack types,
//! special effects, handicap system, and combat status tracking.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.4 for complete specifications.

use crate::domain::types::DiceRoll;
use serde::{Deserialize, Serialize};

// ===== Attack =====

/// Attack definition for monsters and characters
///
/// # Examples
///
/// ```
/// use antares::domain::combat::types::{Attack, AttackType};
/// use antares::domain::types::DiceRoll;
///
/// let sword_attack = Attack::new(
///     DiceRoll::new(1, 8, 2),
///     AttackType::Physical,
///     None,
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attack {
    /// Damage roll for this attack
    pub damage: DiceRoll,
    /// Type of attack (physical, fire, etc.)
    pub attack_type: AttackType,
    /// Optional special effect (poison, paralysis, etc.)
    pub special_effect: Option<SpecialEffect>,
}

impl Attack {
    /// Creates a new attack
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::combat::types::{Attack, AttackType, SpecialEffect};
    /// use antares::domain::types::DiceRoll;
    ///
    /// let poison_bite = Attack::new(
    ///     DiceRoll::new(1, 4, 0),
    ///     AttackType::Physical,
    ///     Some(SpecialEffect::Poison),
    /// );
    /// ```
    pub fn new(
        damage: DiceRoll,
        attack_type: AttackType,
        special_effect: Option<SpecialEffect>,
    ) -> Self {
        Self {
            damage,
            attack_type,
            special_effect,
        }
    }

    /// Creates a basic physical attack
    pub fn physical(damage: DiceRoll) -> Self {
        Self::new(damage, AttackType::Physical, None)
    }

    /// Returns true if this attack has a special effect
    pub fn has_special_effect(&self) -> bool {
        self.special_effect.is_some()
    }
}

// ===== AttackType =====

/// Type of attack damage
///
/// Different attack types may interact with character resistances.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::types::AttackType;
///
/// let attack_type = AttackType::Fire;
/// assert!(matches!(attack_type, AttackType::Fire));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackType {
    /// Physical melee or ranged attack
    Physical,
    /// Fire-based attack
    Fire,
    /// Cold/ice-based attack
    Cold,
    /// Electrical/lightning attack
    Electricity,
    /// Acid-based attack
    Acid,
    /// Poison damage
    Poison,
    /// Energy/magic damage
    Energy,
}

// ===== SpecialEffect =====

/// Special effects that can be applied by attacks
///
/// # Examples
///
/// ```
/// use antares::domain::combat::types::SpecialEffect;
///
/// let effect = SpecialEffect::Paralysis;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecialEffect {
    /// Apply poison condition
    Poison,
    /// Apply disease condition
    Disease,
    /// Apply paralysis condition
    Paralysis,
    /// Apply sleep condition
    Sleep,
    /// Level or stat drain
    Drain,
    /// Turn to stone
    Stone,
    /// Instant death effect
    Death,
}

// ===== Handicap =====

/// Combat handicap representing relative advantage
///
/// Determines initiative order and affects combat dynamics.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::types::Handicap;
///
/// let handicap = Handicap::PartyAdvantage;
/// assert!(matches!(handicap, Handicap::PartyAdvantage));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Handicap {
    /// Party has advantage (party acts first)
    PartyAdvantage,
    /// Monsters have advantage (monsters act first)
    MonsterAdvantage,
    /// No advantage (normal initiative order)
    #[default]
    Even,
}

// ===== CombatStatus =====

/// Current status of combat
///
/// # Examples
///
/// ```
/// use antares::domain::combat::types::CombatStatus;
///
/// let status = CombatStatus::InProgress;
/// assert!(matches!(status, CombatStatus::InProgress));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CombatStatus {
    /// Combat is ongoing
    #[default]
    InProgress,
    /// Party won the combat
    Victory,
    /// Party was defeated
    Defeat,
    /// Party fled successfully
    Fled,
    /// Combat ended in surrender
    Surrendered,
}

/// High-level actions a combatant can take on their turn.
///
/// This enum represents choices available to players (and AI): Attack,
/// Defend, Flee, Cast a Spell, or Use an Item.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::types::TurnAction;
///
/// let a = TurnAction::Attack;
/// assert!(matches!(a, TurnAction::Attack));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnAction {
    /// Physical or ranged attack
    Attack,
    /// Defend (temporary AC/defense bonus)
    Defend,
    /// Attempt to flee the encounter
    Flee,
    /// Cast a spell (select a spell and target)
    CastSpell,
    /// Use an item from inventory
    UseItem,
}

// ===== CombatantId =====

/// Identifier for a combatant in battle
///
/// Can reference either a party member or a monster.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::types::CombatantId;
///
/// let player_id = CombatantId::Player(0);
/// let monster_id = CombatantId::Monster(3);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CombatantId {
    /// Party member by index in party
    Player(usize),
    /// Monster by index in encounter
    Monster(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attack_creation() {
        let attack = Attack::physical(DiceRoll::new(1, 8, 2));
        assert!(matches!(attack.attack_type, AttackType::Physical));
        assert!(attack.special_effect.is_none());
        assert!(!attack.has_special_effect());
    }

    #[test]
    fn test_attack_with_special_effect() {
        let attack = Attack::new(
            DiceRoll::new(1, 6, 0),
            AttackType::Physical,
            Some(SpecialEffect::Poison),
        );
        assert!(attack.has_special_effect());
        assert_eq!(attack.special_effect, Some(SpecialEffect::Poison));
    }

    #[test]
    fn test_attack_types() {
        let fire = AttackType::Fire;
        let cold = AttackType::Cold;
        assert_ne!(fire, cold);
        assert_eq!(fire, AttackType::Fire);
    }

    #[test]
    fn test_handicap_system() {
        let advantage = Handicap::PartyAdvantage;
        let disadvantage = Handicap::MonsterAdvantage;
        let even = Handicap::Even;

        assert!(matches!(advantage, Handicap::PartyAdvantage));
        assert!(matches!(disadvantage, Handicap::MonsterAdvantage));
        assert!(matches!(even, Handicap::Even));
    }

    #[test]
    fn test_handicap_default() {
        let handicap = Handicap::default();
        assert_eq!(handicap, Handicap::Even);
    }

    #[test]
    fn test_combat_status() {
        let status = CombatStatus::InProgress;
        assert!(matches!(status, CombatStatus::InProgress));

        let victory = CombatStatus::Victory;
        assert!(matches!(victory, CombatStatus::Victory));
    }

    #[test]
    fn test_combat_status_default() {
        let status = CombatStatus::default();
        assert_eq!(status, CombatStatus::InProgress);
    }

    #[test]
    fn test_combatant_id_variants() {
        let player = CombatantId::Player(2);
        let monster = CombatantId::Monster(5);

        assert!(matches!(player, CombatantId::Player(2)));
        assert!(matches!(monster, CombatantId::Monster(5)));
        assert_ne!(player, monster);
    }

    #[test]
    fn test_special_effects() {
        let poison = SpecialEffect::Poison;
        let paralysis = SpecialEffect::Paralysis;
        let death = SpecialEffect::Death;

        assert_eq!(poison, SpecialEffect::Poison);
        assert_ne!(poison, paralysis);
        assert_ne!(paralysis, death);
    }
}
