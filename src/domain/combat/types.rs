// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Combat type definitions
//!
//! This module defines the core types used in combat, including attack types,
//! special effects, handicap system, combat status tracking, and the
//! `CombatEventType` that governs how each encounter begins and behaves.
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
    /// True when this attack is ranged (bow, crossbow, thrown).
    ///
    /// Used by `perform_attack_action_with_rng` to reject ranged weapons
    /// in the melee path, and by `perform_monster_turn_with_rng` to prefer
    /// ranged monster attacks in Ranged combat events.
    #[serde(default)]
    pub is_ranged: bool,
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
            is_ranged: false,
        }
    }

    /// Creates a basic physical attack
    ///
    /// The `is_ranged` field is set to `false`.
    pub fn physical(damage: DiceRoll) -> Self {
        Self::new(damage, AttackType::Physical, None)
    }

    /// Creates a ranged physical attack
    ///
    /// Sets `is_ranged` to `true`. Use this constructor when building
    /// an `Attack` for a ranged weapon (bow, crossbow, thrown weapon).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::combat::types::Attack;
    /// use antares::domain::types::DiceRoll;
    ///
    /// let arrow_shot = Attack::ranged(DiceRoll::new(1, 6, 0));
    /// assert!(arrow_shot.is_ranged);
    /// ```
    pub fn ranged(damage: DiceRoll) -> Self {
        Self {
            damage,
            attack_type: AttackType::Physical,
            special_effect: None,
            is_ranged: true,
        }
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
    /// Physical melee attack
    Attack,
    /// Ranged attack using a bow or other ranged weapon with ammo.
    ///
    /// Requires the acting character to have a [`WeaponClassification::MartialRanged`]
    /// weapon equipped **and** at least one compatible ammo item in their
    /// inventory.  Consumes one ammo item on use.  Only available during
    /// `CombatEventType::Ranged` encounters or whenever the character has a
    /// ranged weapon equipped.
    RangedAttack,
    /// Defend (temporary AC/defense bonus)
    Defend,
    /// Attempt to flee the encounter
    Flee,
    /// Cast a spell (select a spell and target)
    CastSpell,
    /// Use an item from inventory
    UseItem,
    /// Internal-only: skip this combatant's turn without any action.
    ///
    /// Used during round 1 of an ambush encounter (the party is surprised and
    /// cannot act) and for incapacitated combatants whose turn must be
    /// auto-advanced. This variant is **never** shown in the player UI action
    /// menu; it is only dispatched programmatically by the combat engine.
    Skip,
}

// ===== CombatEventType =====

/// The type of combat event that determines how a battle begins and what
/// special mechanics apply throughout.
///
/// Campaign authors set this in `map.ron` per-encounter or on the
/// `EncounterTable` for random encounters.  The game engine uses it to
/// configure `CombatState` before `start_combat()` is called.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::types::CombatEventType;
///
/// let t = CombatEventType::Ambush;
/// assert!(t.gives_monster_advantage());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CombatEventType {
    /// Party sees the monster before combat begins. Normal initiative order.
    /// No special mechanics.
    #[default]
    Normal,

    /// Party does not see the monster. Monsters act first in round 1 only
    /// (MonsterAdvantage handicap for round 1, then Even from round 2).
    /// The party's actions are suppressed during their first turn of round 1.
    Ambush,

    /// Party and monster can exchange ranged attacks before closing to melee.
    /// Combatants with a ranged weapon or ranged-capable attack gain an
    /// additional "Ranged Attack" action option. Normal initiative order.
    Ranged,

    /// Monster uses magic as its primary attack vector. The "Cast Spell"
    /// action button is highlighted and placed first in the action menu.
    /// Normal initiative order.
    Magic,

    /// Monster is a boss with special abilities and enhanced stats at runtime.
    /// Bosses: advance every round, may regenerate, cannot be bribed,
    /// cannot be surrendered to. Normal initiative order.
    Boss,
}

impl CombatEventType {
    /// Returns `true` if this event type gives monsters the first-round advantage.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::combat::types::CombatEventType;
    ///
    /// assert!(CombatEventType::Ambush.gives_monster_advantage());
    /// assert!(!CombatEventType::Normal.gives_monster_advantage());
    /// assert!(!CombatEventType::Boss.gives_monster_advantage());
    /// ```
    pub fn gives_monster_advantage(&self) -> bool {
        matches!(self, CombatEventType::Ambush)
    }

    /// Returns `true` if this event type enables the dedicated Ranged Attack action.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::combat::types::CombatEventType;
    ///
    /// assert!(CombatEventType::Ranged.enables_ranged_action());
    /// assert!(!CombatEventType::Normal.enables_ranged_action());
    /// ```
    pub fn enables_ranged_action(&self) -> bool {
        matches!(self, CombatEventType::Ranged)
    }

    /// Returns `true` if this event type highlights the Cast Spell action.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::combat::types::CombatEventType;
    ///
    /// assert!(CombatEventType::Magic.highlights_magic_action());
    /// assert!(!CombatEventType::Normal.highlights_magic_action());
    /// ```
    pub fn highlights_magic_action(&self) -> bool {
        matches!(self, CombatEventType::Magic)
    }

    /// Returns `true` if this event type applies boss mechanics to all monsters.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::combat::types::CombatEventType;
    ///
    /// assert!(CombatEventType::Boss.applies_boss_mechanics());
    /// assert!(!CombatEventType::Normal.applies_boss_mechanics());
    /// ```
    pub fn applies_boss_mechanics(&self) -> bool {
        matches!(self, CombatEventType::Boss)
    }

    /// Human-readable display name used in the Campaign Builder UI.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::combat::types::CombatEventType;
    ///
    /// assert_eq!(CombatEventType::Normal.display_name(), "Normal");
    /// assert_eq!(CombatEventType::Ambush.display_name(), "Ambush");
    /// ```
    pub fn display_name(&self) -> &'static str {
        match self {
            CombatEventType::Normal => "Normal",
            CombatEventType::Ambush => "Ambush",
            CombatEventType::Ranged => "Ranged",
            CombatEventType::Magic => "Magic",
            CombatEventType::Boss => "Boss",
        }
    }

    /// Short description used in Campaign Builder tooltips.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::combat::types::CombatEventType;
    ///
    /// assert!(!CombatEventType::Ambush.description().is_empty());
    /// ```
    pub fn description(&self) -> &'static str {
        match self {
            CombatEventType::Normal => "Party sees the monster. Standard initiative order.",
            CombatEventType::Ambush => {
                "Party is surprised. Monsters act first; party misses round 1."
            }
            CombatEventType::Ranged => "Ranged weapons and ranged monster attacks are available.",
            CombatEventType::Magic => "Monsters use magic. Cast Spell is the primary action.",
            CombatEventType::Boss => {
                "Boss fight. Monsters advance, may regenerate, cannot be bribed or surrendered to."
            }
        }
    }

    /// All variants in display order for UI combo-boxes.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::combat::types::CombatEventType;
    ///
    /// assert_eq!(CombatEventType::all().len(), 5);
    /// assert_eq!(CombatEventType::all()[0], CombatEventType::Normal);
    /// ```
    pub fn all() -> &'static [CombatEventType] {
        &[
            CombatEventType::Normal,
            CombatEventType::Ambush,
            CombatEventType::Ranged,
            CombatEventType::Magic,
            CombatEventType::Boss,
        ]
    }
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

    // ===== CombatEventType tests =====

    #[test]
    fn test_combat_event_type_default_is_normal() {
        let t = CombatEventType::default();
        assert_eq!(t, CombatEventType::Normal);
    }

    #[test]
    fn test_combat_event_type_flags() {
        // gives_monster_advantage
        assert!(CombatEventType::Ambush.gives_monster_advantage());
        assert!(!CombatEventType::Normal.gives_monster_advantage());
        assert!(!CombatEventType::Ranged.gives_monster_advantage());
        assert!(!CombatEventType::Magic.gives_monster_advantage());
        assert!(!CombatEventType::Boss.gives_monster_advantage());

        // enables_ranged_action
        assert!(CombatEventType::Ranged.enables_ranged_action());
        assert!(!CombatEventType::Normal.enables_ranged_action());

        // highlights_magic_action
        assert!(CombatEventType::Magic.highlights_magic_action());
        assert!(!CombatEventType::Normal.highlights_magic_action());

        // applies_boss_mechanics
        assert!(CombatEventType::Boss.applies_boss_mechanics());
        assert!(!CombatEventType::Normal.applies_boss_mechanics());
    }

    #[test]
    fn test_combat_event_type_display_names() {
        assert_eq!(CombatEventType::Normal.display_name(), "Normal");
        assert_eq!(CombatEventType::Ambush.display_name(), "Ambush");
        assert_eq!(CombatEventType::Ranged.display_name(), "Ranged");
        assert_eq!(CombatEventType::Magic.display_name(), "Magic");
        assert_eq!(CombatEventType::Boss.display_name(), "Boss");
    }

    #[test]
    fn test_combat_event_type_descriptions_non_empty() {
        for variant in CombatEventType::all() {
            assert!(
                !variant.description().is_empty(),
                "{:?} description is empty",
                variant
            );
        }
    }

    #[test]
    fn test_combat_event_type_all_has_five_variants() {
        assert_eq!(CombatEventType::all().len(), 5);
        assert_eq!(CombatEventType::all()[0], CombatEventType::Normal);
        assert_eq!(CombatEventType::all()[4], CombatEventType::Boss);
    }

    #[test]
    fn test_combat_event_type_serde_round_trip() {
        use ron;
        for &variant in CombatEventType::all() {
            let serialized = ron::to_string(&variant).expect("serialize");
            let deserialized: CombatEventType = ron::from_str(&serialized).expect("deserialize");
            assert_eq!(variant, deserialized);
        }
    }

    #[test]
    fn test_combat_event_type_default_deserializes_when_missing() {
        // A struct that wraps CombatEventType with serde(default) — simulates
        // what MapEvent::Encounter does.
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Wrapper {
            #[serde(default)]
            combat_event_type: CombatEventType,
        }
        let ron_str = "()"; // no field present
        let w: Wrapper = ron::from_str(ron_str).expect("deserialize with default");
        assert_eq!(w.combat_event_type, CombatEventType::Normal);
    }

    #[test]
    fn test_attack_creation() {
        let attack = Attack::physical(DiceRoll::new(1, 8, 2));
        assert!(matches!(attack.attack_type, AttackType::Physical));
        assert!(attack.special_effect.is_none());
        assert!(!attack.has_special_effect());
        assert!(!attack.is_ranged);
    }

    #[test]
    fn test_attack_physical_constructor_is_ranged_false() {
        let attack = Attack::physical(DiceRoll::new(1, 4, 0));
        assert!(!attack.is_ranged);
    }

    #[test]
    fn test_attack_ranged_constructor_sets_is_ranged_true() {
        let attack = Attack::ranged(DiceRoll::new(1, 6, 0));
        assert!(attack.is_ranged);
        assert!(matches!(attack.attack_type, AttackType::Physical));
        assert!(attack.special_effect.is_none());
    }

    #[test]
    fn test_attack_ranged_damage_preserved() {
        let dice = DiceRoll::new(1, 8, 1);
        let attack = Attack::ranged(dice);
        assert_eq!(attack.damage, dice);
        assert!(attack.is_ranged);
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
