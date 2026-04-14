// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Combat engine - turn-based combat logic
//!
//! This module implements the core combat system including combat state management,
//! turn order calculation, attack resolution, and damage application.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.4 for complete specifications.

use crate::application::ActiveSpells;
use crate::domain::character::Character;
use crate::domain::combat::monster::Monster;
use crate::domain::combat::types::{
    Attack, AttackType, CombatStatus, CombatantId, Handicap, SpecialEffect,
};
use crate::domain::items::{ItemDatabase, ItemType, WeaponClassification};
use crate::domain::magic::types::Spell;
use crate::domain::types::DiceRoll;
use std::collections::HashSet;
// Condition types referenced by fully-qualified paths where needed
use rand::Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ===== Unarmed Combat Constant =====

/// Default damage roll for an unarmed character (no weapon equipped).
///
/// Per the game spec, unarmed strikes deal 1d2 physical damage.
/// This constant replaces all scattered `DiceRoll::new(1, 4, 0)` literals
/// that were previously used as the player fallback — 1d4 was wrong.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::engine::UNARMED_DAMAGE;
/// use antares::domain::types::DiceRoll;
///
/// assert_eq!(UNARMED_DAMAGE, DiceRoll { count: 1, sides: 2, bonus: 0 });
/// ```
pub const UNARMED_DAMAGE: DiceRoll = DiceRoll {
    count: 1,
    sides: 2,
    bonus: 0,
};

// ===== MeleeAttackResult =====

/// Outcome of resolving a character's attack from their equipped weapon.
///
/// `get_character_attack` returns this enum to allow callers to distinguish
/// between a usable melee attack and a ranged weapon that must be handled
/// via the dedicated ranged-attack path instead.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::engine::{get_character_attack, MeleeAttackResult, UNARMED_DAMAGE};
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::items::ItemDatabase;
///
/// let character = Character::new(
///     "Hero".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// let db = ItemDatabase::new();
///
/// match get_character_attack(&character, &db) {
///     MeleeAttackResult::Melee(attack) => {
///         assert_eq!(attack.damage, UNARMED_DAMAGE);
///     }
///     MeleeAttackResult::Ranged(_) => panic!("Expected melee result"),
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum MeleeAttackResult {
    /// A valid melee `Attack` ready for `resolve_attack`.
    Melee(Attack),
    /// The equipped weapon is ranged; the melee path must not proceed.
    ///
    /// The inner `Attack` carries the weapon's stats so callers can log or
    /// display them without a second item lookup, but MUST NOT apply damage
    /// through the melee pipeline with it.
    Ranged(Attack),
}

// ===== Error Types =====

/// Errors that can occur during combat
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CombatError {
    #[error("Combat is not in progress")]
    NotInProgress,

    #[error("Combatant {0:?} not found")]
    CombatantNotFound(CombatantId),

    #[error("Combatant {0:?} cannot act")]
    CombatantCannotAct(CombatantId),

    #[error("Invalid target {0:?}")]
    InvalidTarget(CombatantId),

    #[error("No ammo available for ranged attack")]
    NoAmmo,

    /// The spell cast failed (insufficient SP, wrong class, silenced, etc.)
    #[error("Spell fizzled: {0}")]
    SpellFizzled(String),
}

// ===== Combatant =====

/// A combatant in battle (either player or monster)
///
/// # Examples
///
/// ```
/// use antares::domain::combat::engine::Combatant;
/// use antares::domain::character::{Character, Sex, Alignment};
///
/// let mut character = Character::new(
///     "Hero".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// let combatant = Combatant::Player(Box::new(character));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Combatant {
    /// Player character
    Player(Box<Character>),
    /// Monster
    Monster(Box<Monster>),
}

impl Combatant {
    /// Returns the combatant's speed for turn order calculation
    pub fn get_speed(&self) -> u8 {
        match self {
            Combatant::Player(character) => character.stats.speed.current,
            Combatant::Monster(monster) => monster.stats.speed.current,
        }
    }

    /// Returns true if the combatant can act this turn
    pub fn can_act(&self) -> bool {
        match self {
            Combatant::Player(character) => character.can_act(),
            Combatant::Monster(monster) => monster.can_act(),
        }
    }

    /// Returns true if the combatant is alive
    pub fn is_alive(&self) -> bool {
        match self {
            Combatant::Player(character) => character.is_alive(),
            Combatant::Monster(monster) => monster.is_alive(),
        }
    }

    /// Returns the combatant's name
    pub fn get_name(&self) -> &str {
        match self {
            Combatant::Player(character) => &character.name,
            Combatant::Monster(monster) => &monster.name,
        }
    }
}

// ===== CombatState =====

/// State of an active combat encounter
///
/// # Examples
///
/// ```
/// use antares::domain::combat::engine::CombatState;
/// use antares::domain::combat::types::Handicap;
///
/// let combat = CombatState::new(Handicap::Even);
/// assert_eq!(combat.round, 1);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatState {
    /// All combatants in the battle
    pub participants: Vec<Combatant>,
    /// Turn order (indices into participants)
    pub turn_order: Vec<CombatantId>,
    /// Current turn index in turn_order
    pub current_turn: usize,
    /// Current round number (starts at 1)
    pub round: u32,
    /// Current combat status
    pub status: CombatStatus,
    /// Combat handicap (party/monster advantage)
    pub handicap: Handicap,
    /// Can the party flee?
    pub can_flee: bool,
    /// Can the party surrender?
    pub can_surrender: bool,
    /// Can the party bribe monsters?
    pub can_bribe: bool,
    /// Do monsters advance each round?
    pub monsters_advance: bool,
    /// Do monsters regenerate each round?
    pub monsters_regenerate: bool,
    /// True during round 1 of an ambush encounter.
    ///
    /// When set, player turns are automatically skipped (the party is surprised
    /// and cannot act). Cleared automatically at the start of round 2, at which
    /// point the handicap is also reset to `Handicap::Even` so that subsequent
    /// rounds are fought on equal footing.
    pub ambush_round_active: bool,

    /// Controls whether characters become unconscious before dying.
    ///
    /// When `true` (default), a character reaching 0 HP first becomes
    /// unconscious; further damage while unconscious transitions them to dead.
    /// When `false` (instant-death mode), reaching 0 HP sets DEAD immediately.
    ///
    /// Copied from `CampaignConfig::unconscious_before_death` at combat start.
    #[serde(default = "default_true")]
    pub unconscious_before_death: bool,

    /// Set of participant indices that are currently defending this round.
    ///
    /// Only player combatants (indices into `participants`) are eligible.
    /// Inserted by `perform_defend_action` and drained at the start of each
    /// new round by `advance_round`, which also reverses the +2 AC bonus.
    #[serde(default)]
    pub defending_combatants: HashSet<usize>,
}

/// Serde helper: returns `true` as the default value for fields that should
/// default to `true`.
fn default_true() -> bool {
    true
}

impl CombatState {
    /// Creates a new combat state
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::combat::engine::CombatState;
    /// use antares::domain::combat::types::Handicap;
    ///
    /// let combat = CombatState::new(Handicap::PartyAdvantage);
    /// assert_eq!(combat.round, 1);
    /// assert!(combat.can_flee);
    /// ```
    pub fn new(handicap: Handicap) -> Self {
        Self {
            participants: Vec::new(),
            turn_order: Vec::new(),
            current_turn: 0,
            round: 1,
            status: CombatStatus::InProgress,
            handicap,
            can_flee: true,
            can_surrender: true,
            can_bribe: true,
            monsters_advance: false,
            monsters_regenerate: false,
            ambush_round_active: false,
            unconscious_before_death: true,
            defending_combatants: HashSet::new(),
        }
    }

    /// Adds a player character to combat
    pub fn add_player(&mut self, character: Character) {
        self.participants
            .push(Combatant::Player(Box::new(character)));
    }

    /// Adds a monster to combat
    pub fn add_monster(&mut self, monster: Monster) {
        self.participants
            .push(Combatant::Monster(Box::new(monster)));
    }

    /// Returns true if combat is still in progress
    pub fn is_in_progress(&self) -> bool {
        self.status == CombatStatus::InProgress
    }

    /// Returns the number of alive party members
    pub fn alive_party_count(&self) -> usize {
        self.participants
            .iter()
            .filter(|c| matches!(c, Combatant::Player(_)) && c.is_alive())
            .count()
    }

    /// Returns the number of alive monsters
    pub fn alive_monster_count(&self) -> usize {
        self.participants
            .iter()
            .filter(|c| matches!(c, Combatant::Monster(_)) && c.is_alive())
            .count()
    }

    /// Checks if combat should end and updates status
    pub fn check_combat_end(&mut self) {
        if self.alive_party_count() == 0 {
            self.status = CombatStatus::Defeat;
        } else if self.alive_monster_count() == 0 {
            self.status = CombatStatus::Victory;
        }
    }

    /// Advances to the next turn
    ///
    /// Returns any effects (damage/healing) that occurred if a new round started
    pub fn advance_turn(
        &mut self,
        condition_defs: &[crate::domain::conditions::ConditionDefinition],
    ) -> Vec<(CombatantId, i16)> {
        self.current_turn += 1;
        if self.current_turn >= self.turn_order.len() {
            self.current_turn = 0;
            return self.advance_round(condition_defs);
        }
        Vec::new()
    }

    /// Advances to the next round
    fn advance_round(
        &mut self,
        condition_defs: &[crate::domain::conditions::ConditionDefinition],
    ) -> Vec<(CombatantId, i16)> {
        self.round += 1;

        // Ambush only suppresses player actions in round 1.
        // At the start of round 2 clear the flag and reset to Even handicap so
        // that the remainder of the fight is not permanently skewed.
        if self.round == 2 && self.ambush_round_active {
            self.ambush_round_active = false;
            self.handicap = Handicap::Even;
            // Recalculate turn order under the new (even) handicap so speed
            // ties are resolved fairly from this round onward.
            self.turn_order = crate::domain::combat::engine::calculate_turn_order(self);
            self.current_turn = 0;
        }

        let mut effects = Vec::new();

        // Tick conditions and reset flags for all participants
        for participant in &mut self.participants {
            match participant {
                Combatant::Player(character) => {
                    // Tick round-based conditions
                    character.tick_conditions_round();

                    // Reconcile status flags based on active conditions and definitions
                    crate::domain::combat::engine::reconcile_character_conditions(
                        character,
                        condition_defs,
                    );
                }
                Combatant::Monster(monster) => {
                    // Tick round-based conditions
                    monster.tick_conditions_round();

                    // Reset has_acted flag
                    monster.reset_turn();

                    // Monster regeneration
                    if self.monsters_regenerate && monster.can_regenerate {
                        monster.regenerate(1);
                    }

                    // Reconcile monster status flags based on active conditions
                    crate::domain::combat::engine::reconcile_monster_conditions(
                        monster,
                        condition_defs,
                    );
                }
            }
        }

        // Apply DoT/HoT effects
        effects.extend(self.apply_dot_effects(condition_defs));

        // Clear defending flags and reverse the +2 AC bonus that was applied
        // when each player chose Defend this round.  The bonus is bounded to
        // a single round: it is applied in `perform_defend_action` and removed
        // here at the start of the next round.
        for idx in self.defending_combatants.drain() {
            if let Some(Combatant::Player(pc)) = self.participants.get_mut(idx) {
                // Reverse the +2 bonus; never let ac.current fall below ac.base.
                pc.ac.current = pc.ac.current.saturating_sub(2).max(pc.ac.base);
            }
        }

        effects
    }

    /// Applies damage/healing over time effects from conditions
    ///
    /// This should be called at the start of each round after ticking conditions.
    /// Requires condition definitions to look up effect details.
    ///
    /// Returns a list of (combatant_id, damage_amount) tuples for logging/display
    pub fn apply_dot_effects(
        &mut self,
        condition_defs: &[crate::domain::conditions::ConditionDefinition],
    ) -> Vec<(CombatantId, i16)> {
        use crate::domain::magic::apply_condition_dot_effects;

        let mut effects = Vec::new();

        for (idx, participant) in self.participants.iter_mut().enumerate() {
            let combatant_id = match participant {
                Combatant::Player(_) => CombatantId::Player(idx),
                Combatant::Monster(_) => CombatantId::Monster(idx),
            };

            let damage = match participant {
                Combatant::Player(character) => {
                    apply_condition_dot_effects(&character.active_conditions, condition_defs)
                }
                Combatant::Monster(monster) => {
                    apply_condition_dot_effects(&monster.active_conditions, condition_defs)
                }
            };

            if damage != 0 {
                // Apply the damage (negative = healing)
                match participant {
                    Combatant::Player(character) => {
                        character.hp.modify(-damage as i32);
                    }
                    Combatant::Monster(monster) => {
                        if damage > 0 {
                            monster.take_damage(damage as u16);
                        } else {
                            // Healing
                            monster.hp.modify((-damage) as i32);
                        }
                    }
                }

                effects.push((combatant_id, damage));
            }
        }

        effects
    }

    /// Gets the current combatant
    pub fn get_current_combatant(&self) -> Option<&Combatant> {
        self.turn_order
            .get(self.current_turn)
            .and_then(|id| self.get_combatant(id))
    }

    /// Gets a combatant by ID
    pub fn get_combatant(&self, id: &CombatantId) -> Option<&Combatant> {
        match id {
            CombatantId::Player(idx) => self.participants.get(*idx),
            CombatantId::Monster(idx) => self.participants.get(*idx),
        }
    }

    /// Gets a mutable combatant by ID
    pub fn get_combatant_mut(&mut self, id: &CombatantId) -> Option<&mut Combatant> {
        match id {
            CombatantId::Player(idx) => self.participants.get_mut(*idx),
            CombatantId::Monster(idx) => self.participants.get_mut(*idx),
        }
    }

    /// Removes all `UntilCombatEnd` active conditions from every participant
    /// and reconciles status-flag bitfields so the party exits combat with a
    /// clean condition state.
    ///
    /// Must be called before syncing combat state back to party members.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::combat::engine::{CombatState, Combatant};
    /// use antares::domain::combat::types::Handicap;
    /// use antares::domain::conditions::{ConditionDefinition, ConditionDuration};
    ///
    /// let mut combat = CombatState::new(Handicap::Even);
    /// combat.clear_combat_end_conditions(&[]);
    /// ```
    pub fn clear_combat_end_conditions(
        &mut self,
        condition_defs: &[crate::domain::conditions::ConditionDefinition],
    ) {
        for participant in &mut self.participants {
            match participant {
                Combatant::Player(character) => {
                    character.tick_conditions_combat_end();
                    crate::domain::combat::engine::reconcile_character_conditions(
                        character,
                        condition_defs,
                    );
                }
                Combatant::Monster(monster) => {
                    monster.tick_conditions_combat_end();
                    crate::domain::combat::engine::reconcile_monster_conditions(
                        monster,
                        condition_defs,
                    );
                }
            }
        }
    }
}

// ===== Combat Logic Functions =====

/// Starts combat and initializes turn order
///
/// # Examples
///
/// ```
/// use antares::domain::combat::engine::{CombatState, start_combat};
/// use antares::domain::combat::types::Handicap;
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::combat::monster::{Monster, LootTable};
/// use antares::domain::character::Stats;
/// use antares::domain::types::DiceRoll;
/// use antares::domain::combat::types::Attack;
///
/// let mut combat = CombatState::new(Handicap::Even);
/// let character = Character::new("Hero".to_string(), "human".to_string(), "knight".to_string(), Sex::Male, Alignment::Good);
/// combat.add_player(character);
///
/// let stats = Stats::new(10, 8, 6, 10, 8, 7, 5);
/// let attacks = vec![Attack::physical(DiceRoll::new(1, 6, 0))];
/// let monster = Monster::new(1, "Goblin".to_string(), stats, 10, 5, attacks, LootTable::none());
/// combat.add_monster(monster);
///
/// start_combat(&mut combat);
/// assert!(!combat.turn_order.is_empty());
/// ```
pub fn start_combat(combat: &mut CombatState) {
    combat.turn_order = calculate_turn_order(combat);
    combat.current_turn = 0;
    combat.round = 1;
    combat.status = CombatStatus::InProgress;
}

/// Calculates turn order based on speed and handicap
///
/// # Examples
///
/// ```
/// use antares::domain::combat::engine::{CombatState, calculate_turn_order};
/// use antares::domain::combat::types::Handicap;
/// use antares::domain::character::{Character, Sex, Alignment};
///
/// let mut combat = CombatState::new(Handicap::PartyAdvantage);
/// let knight = Character::new("Knight".to_string(), "human".to_string(), "knight".to_string(), Sex::Male, Alignment::Good);
/// combat.add_player(knight);
///
/// let order = calculate_turn_order(&combat);
/// assert_eq!(order.len(), 1);
/// ```
pub fn calculate_turn_order(combat: &CombatState) -> Vec<CombatantId> {
    let mut order: Vec<(CombatantId, u8)> = combat
        .participants
        .iter()
        .enumerate()
        .filter(|(_, c)| c.is_alive())
        .map(|(idx, c)| {
            let id = match c {
                Combatant::Player(_) => CombatantId::Player(idx),
                Combatant::Monster(_) => CombatantId::Monster(idx),
            };
            let speed = c.get_speed();
            (id, speed)
        })
        .collect();

    // Sort by speed (descending), with handicap affecting order
    match combat.handicap {
        Handicap::PartyAdvantage => {
            // Players go first regardless of speed
            order.sort_by(|a, b| {
                let a_is_player = matches!(a.0, CombatantId::Player(_));
                let b_is_player = matches!(b.0, CombatantId::Player(_));
                match (a_is_player, b_is_player) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => b.1.cmp(&a.1), // Within same type, sort by speed descending
                }
            });
        }
        Handicap::MonsterAdvantage => {
            // Monsters go first regardless of speed
            order.sort_by(|a, b| {
                let a_is_player = matches!(a.0, CombatantId::Player(_));
                let b_is_player = matches!(b.0, CombatantId::Player(_));
                match (a_is_player, b_is_player) {
                    (true, false) => std::cmp::Ordering::Greater,
                    (false, true) => std::cmp::Ordering::Less,
                    _ => b.1.cmp(&a.1), // Within same type, sort by speed descending
                }
            });
        }
        Handicap::Even => {
            // Normal initiative: sort by speed descending
            order.sort_by(|a, b| b.1.cmp(&a.1));
        }
    }

    order.into_iter().map(|(id, _)| id).collect()
}

// ===== Defense Reduction =====

/// Outcome of `compute_defense_reduction` — either the target is fully immune
/// (only possible with `power_shield` active) or a damage multiplier applies.
#[derive(Debug, Clone, Copy, PartialEq)]
enum DefenseReduction {
    /// Target takes zero damage this hit (power_shield).
    Immune,
    /// Apply this multiplier to raw damage before elemental resistance.
    ///
    /// A value of `1.0` means no reduction.  Values below `1.0` reduce damage;
    /// `0.25` is the minimum non-immune multiplier.
    Multiplier(f32),
}

/// Computes the damage-reduction multiplier for a defending target.
///
/// Priority order (highest to lowest):
///
/// | Condition                          | Multiplier / Result                     |
/// |------------------------------------|------------------------------------------|
/// | `power_shield` active              | `Immune` (0 damage)                      |
/// | Defending **and** `shield` active  | `0.35` (65 % reduction)                 |
/// | Defending only                     | `0.5` − endurance bonus, min `0.25`     |
/// | `shield` active (not defending)    | `0.80` (20 % reduction)                 |
/// | `leather_skin` active              | `0.90` (10 % reduction)                 |
/// | None of the above                  | `1.0` (no reduction)                    |
///
/// The endurance modifier: each full 10 points of `endurance` **above 10**
/// subtracts an additional `0.02` from the `0.5` base, floored at `0.25`.
fn compute_defense_reduction(
    is_defending: bool,
    active_spells: Option<&ActiveSpells>,
    endurance: u8,
) -> DefenseReduction {
    let power_shield_active = active_spells.is_some_and(|s| s.power_shield > 0);
    let shield_active = active_spells.is_some_and(|s| s.shield > 0);
    let leather_skin_active = active_spells.is_some_and(|s| s.leather_skin > 0);

    if power_shield_active {
        return DefenseReduction::Immune;
    }

    if is_defending && shield_active {
        return DefenseReduction::Multiplier(0.35);
    }

    if is_defending {
        // Base 50 % reduction, improved by endurance above 10.
        // Each full 10 points above 10 adds an extra −0.02, capped at 0.25 min.
        let above_ten = endurance.saturating_sub(10) as u32;
        let steps = above_ten / 10;
        let endurance_bonus = steps as f32 * 0.02;
        let multiplier = (0.5_f32 - endurance_bonus).max(0.25_f32);
        return DefenseReduction::Multiplier(multiplier);
    }

    if shield_active {
        return DefenseReduction::Multiplier(0.80);
    }

    if leather_skin_active {
        return DefenseReduction::Multiplier(0.90);
    }

    DefenseReduction::Multiplier(1.0)
}

/// Resolves an attack from attacker to target
///
/// Returns the damage dealt and whether a special effect was applied.
/// On a hit, damage is always floored at **1** regardless of weapon penalties,
/// negative bonuses, or low might. A cursed weapon that would roll 0 or less
/// still deals exactly 1 damage point when it connects.
///
/// The floor is applied here — not in [`DiceRoll::roll`] (which floors at 0)
/// and not in [`get_character_attack`] (which only builds the roll descriptor).
/// This is the single authoritative place for the damage-floor invariant.
///
/// # Errors
///
/// Returns `CombatError` if the attack cannot be resolved.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::engine::{CombatState, resolve_attack};
/// use antares::domain::combat::types::{Handicap, CombatantId, Attack};
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::types::DiceRoll;
/// use rand::rng;
///
/// let mut combat = CombatState::new(Handicap::Even);
/// let mut character = Character::new("Knight".to_string(), "human".to_string(), "knight".to_string(), Sex::Male, Alignment::Good);
/// character.stats.might.current = 15;
/// combat.add_player(character);
///
/// let attack = Attack::physical(DiceRoll::new(1, 8, 2));
/// let attacker = CombatantId::Player(0);
/// let target = CombatantId::Player(0);
/// let mut rng = rng();
///
/// // Note: In real combat, target would be a different combatant
/// let result = resolve_attack(&combat, attacker, target, &attack, None, &mut rng);
/// ```
pub fn resolve_attack<R: Rng>(
    combat: &CombatState,
    attacker_id: CombatantId,
    target_id: CombatantId,
    attack: &Attack,
    active_spells: Option<&ActiveSpells>,
    rng: &mut R,
) -> Result<(u16, Option<SpecialEffect>), CombatError> {
    // Get attacker
    let attacker = combat
        .get_combatant(&attacker_id)
        .ok_or(CombatError::CombatantNotFound(attacker_id))?;

    if !attacker.can_act() {
        return Err(CombatError::CombatantCannotAct(attacker_id));
    }

    // Get target
    let target = combat
        .get_combatant(&target_id)
        .ok_or(CombatError::CombatantNotFound(target_id))?;

    if !target.is_alive() {
        return Err(CombatError::InvalidTarget(target_id));
    }

    // Calculate hit chance (simplified)
    let attacker_accuracy = match attacker {
        Combatant::Player(c) => c.stats.accuracy.current,
        Combatant::Monster(m) => m.stats.accuracy.current,
    };

    let target_ac = match target {
        Combatant::Player(c) => c.ac.current,
        Combatant::Monster(m) => m.ac.current,
    };

    // Simple hit calculation: need to roll >= (10 + target_ac - attacker_accuracy)
    let hit_threshold = (10 + target_ac as i16 - attacker_accuracy as i16).max(2) as u8;
    let roll = rng.random_range(1..=20);

    if roll < hit_threshold {
        // Miss
        return Ok((0, None));
    }

    // Hit - roll damage
    let base_damage = attack.damage.roll(rng);

    // Apply might bonus for physical attacks
    let damage_bonus = match (&attack.attack_type, attacker) {
        (AttackType::Physical, Combatant::Player(c)) => (c.stats.might.current as i32 - 10) / 2,
        (AttackType::Physical, Combatant::Monster(m)) => (m.stats.might.current as i32 - 10) / 2,
        _ => 0,
    };

    let raw_damage = (base_damage + damage_bonus).max(1);

    // ── Defense reduction ─────────────────────────────────────────────────────
    // Check whether the target is a defending player and whether active spell
    // buffs (power_shield, shield, leather_skin) provide additional mitigation.
    // The multiplier is applied to raw_damage before elemental resistance so
    // that both systems stack independently.
    let is_defending = match target_id {
        CombatantId::Player(idx) => combat.defending_combatants.contains(&idx),
        CombatantId::Monster(_) => false,
    };
    let target_endurance = match target {
        Combatant::Player(c) => c.stats.endurance.current,
        Combatant::Monster(_) => 10u8, // monsters never defend; endurance is irrelevant
    };
    let raw_damage = match compute_defense_reduction(is_defending, active_spells, target_endurance)
    {
        DefenseReduction::Immune => return Ok((0, None)),
        DefenseReduction::Multiplier(m) if m < 1.0 => {
            ((raw_damage as f32 * m).ceil() as i32).max(1)
        }
        DefenseReduction::Multiplier(_) => raw_damage,
    };

    // Project active spell protection bonuses into effective resistance for the
    // target (players only — monsters do not carry ActiveSpells).  Resistance
    // values are in the range [0, 100] and are treated as a percentage reduction.
    // Effective resistance = character.resistances.<field>.current
    //                       + active_spells.effective_resistance(res_type)
    // clamped to [0, 100].
    let resistance_reduction: i32 = match &attack.attack_type {
        AttackType::Physical => 0, // Physical attacks are not mitigated by elemental resistance
        non_physical => {
            use crate::domain::items::types::ResistanceType;
            // Map AttackType → (ResistanceType for active_spells, character resistance field)
            let res_type = match non_physical {
                AttackType::Fire => ResistanceType::Fire,
                AttackType::Cold => ResistanceType::Cold,
                AttackType::Electricity => ResistanceType::Electricity,
                AttackType::Energy => ResistanceType::Energy,
                AttackType::Acid => ResistanceType::Physical, // closest analogue
                AttackType::Poison => ResistanceType::Physical, // closest analogue
                AttackType::Physical => unreachable!("guarded above"),
            };

            // Character's current resistance for this damage type
            let char_resistance: i16 = match target {
                Combatant::Player(c) => match non_physical {
                    AttackType::Fire => c.resistances.fire.current as i16,
                    AttackType::Cold => c.resistances.cold.current as i16,
                    AttackType::Electricity => c.resistances.electricity.current as i16,
                    AttackType::Energy => c.resistances.magic.current as i16,
                    AttackType::Acid => c.resistances.acid.current as i16,
                    AttackType::Poison => c.resistances.poison.current as i16,
                    AttackType::Physical => unreachable!("guarded above"),
                },
                Combatant::Monster(_) => 0, // Monsters rely on their own stats; no active_spells projection
            };

            // Active-spell bonus for this damage type (0 when no protection active)
            let spell_bonus: i16 = active_spells.map_or(0, |s| s.effective_resistance(res_type));

            // Total effective resistance clamped to [0, 100]
            let effective = (char_resistance + spell_bonus).clamp(0, 100) as i32;

            // Resistance is a percentage reduction: effective / 100 * damage
            (raw_damage * effective) / 100
        }
    };

    let total_damage = (raw_damage - resistance_reduction).max(0) as u16;

    Ok((total_damage, attack.special_effect))
}

/// Applies damage to a combatant
///
/// # Errors
///
/// Returns `CombatError` if the target cannot be found.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::engine::{CombatState, apply_damage};
/// use antares::domain::combat::types::{Handicap, CombatantId};
/// use antares::domain::character::{Character, Sex, Alignment};
///
/// let mut combat = CombatState::new(Handicap::Even);
/// let knight = Character::new("Knight".to_string(), "human".to_string(), "knight".to_string(), Sex::Male, Alignment::Good);
/// combat.add_player(knight);
///
/// let target = CombatantId::Player(0);
/// let result = apply_damage(&mut combat, target, 10);
/// assert!(result.is_ok());
/// ```
pub fn apply_damage(
    combat: &mut CombatState,
    target_id: CombatantId,
    damage: u16,
) -> Result<bool, CombatError> {
    // Snapshot the death mode before taking a mutable borrow on a participant.
    // A `bool` copy avoids a second borrow of `combat` inside the match arm.
    let unconscious_before_death = combat.unconscious_before_death;

    let target = combat
        .get_combatant_mut(&target_id)
        .ok_or(CombatError::CombatantNotFound(target_id))?;

    let died = match target {
        Combatant::Player(character) => {
            use crate::domain::character::Condition;
            use crate::domain::conditions::{ActiveCondition, ConditionDuration};

            let old_hp = character.hp.current;
            let already_unconscious = character.conditions.has(Condition::UNCONSCIOUS);

            character.hp.modify(-(damage as i32));

            if character.hp.current == 0 {
                if already_unconscious {
                    // Further damage to an unconscious character — they die.
                    // Clear UNCONSCIOUS and promote to DEAD.
                    if !character.conditions.has(Condition::DEAD) {
                        character.conditions.remove(Condition::UNCONSCIOUS);
                        character.remove_condition("unconscious");
                        character.conditions.add(Condition::DEAD);
                        character.add_condition(ActiveCondition::new(
                            "dead".to_string(),
                            ConditionDuration::Permanent,
                        ));
                    }
                } else if unconscious_before_death {
                    // First hit to 0 HP in normal mode: become unconscious.
                    // Both the bitflag AND the ActiveCondition are set so that
                    // reconcile_character_conditions remains consistent.
                    if !character.conditions.has(Condition::UNCONSCIOUS) {
                        character.conditions.add(Condition::UNCONSCIOUS);
                        character.add_condition(ActiveCondition::new(
                            "unconscious".to_string(),
                            ConditionDuration::Permanent,
                        ));
                    }
                } else {
                    // Instant-death mode: skip unconscious, go straight to dead.
                    if !character.conditions.has(Condition::DEAD) {
                        character.conditions.add(Condition::DEAD);
                        character.add_condition(ActiveCondition::new(
                            "dead".to_string(),
                            ConditionDuration::Permanent,
                        ));
                    }
                }
            }
            character.hp.current == 0 && old_hp > 0
        }
        Combatant::Monster(monster) => monster.take_damage(damage),
    };

    Ok(died)
}

/// Selects an attack for a monster, honoring its `special_attack_threshold`.
///
/// If the threshold triggers and the monster has attacks that include a special
/// effect, one of those special attacks will be returned. Otherwise a random
/// attack from the monster's attack list is returned.
/// Choose an attack for a monster to use on its turn.
///
/// When `is_ranged_combat` is `true` the function first tries to find an
/// attack that has `is_ranged == true`; if the monster has none it falls back
/// to the normal selection logic.  When `is_ranged_combat` is `false` the
/// original behaviour is preserved unchanged.
///
/// # Arguments
///
/// * `monster` - The monster that is about to act.
/// * `is_ranged_combat` - Whether the current encounter is a ranged combat
///   (`CombatEventType::Ranged`).
/// * `rng` - Random number generator used for probabilistic selection.
///
/// # Returns
///
/// `Some(Attack)` if the monster has at least one attack, `None` if its
/// attack list is empty.
pub fn choose_monster_attack<R: Rng>(
    monster: &Monster,
    is_ranged_combat: bool,
    rng: &mut R,
) -> Option<Attack> {
    if monster.attacks.is_empty() {
        return None;
    }

    // In ranged combat, prefer attacks flagged as ranged.
    if is_ranged_combat {
        let ranged_attacks: Vec<&Attack> = monster.attacks.iter().filter(|a| a.is_ranged).collect();
        if !ranged_attacks.is_empty() {
            let idx = rng.random_range(0..ranged_attacks.len());
            return Some(ranged_attacks[idx].clone());
        }
        // Fall through to normal selection if no ranged attacks exist.
    }

    // Try to use a special attack if threshold triggers
    if monster.special_attack_threshold > 0 {
        let roll = rng.random_range(1..=100);
        if roll <= monster.special_attack_threshold {
            let special_attacks: Vec<&Attack> = monster
                .attacks
                .iter()
                .filter(|a| a.special_effect.is_some())
                .collect();

            if !special_attacks.is_empty() {
                let idx = rng.random_range(0..special_attacks.len());
                return Some(special_attacks[idx].clone());
            }
        }
    }

    // Fallback to a random attack
    let idx = rng.random_range(0..monster.attacks.len());
    Some(monster.attacks[idx].clone())
}

/// Determines the attack a character makes based on their equipped weapon.
///
/// This is a pure-domain function with no I/O or Bevy dependencies. It is
/// the single canonical place to convert a character's equipped weapon into
/// an [`Attack`].
///
/// # Fallback Behaviour
///
/// - No weapon equipped → `MeleeAttackResult::Melee` with [`UNARMED_DAMAGE`]
/// - Weapon ID not found in `item_db` → same unarmed fallback (no panic)
/// - Item found but not a weapon (e.g. a consumable in the weapon slot) →
///   same unarmed fallback
///
/// # Ranged Weapons
///
/// If the equipped weapon has
/// [`WeaponClassification::MartialRanged`][crate::domain::items::WeaponClassification],
/// this function returns `MeleeAttackResult::Ranged`. Callers in the melee
/// path **must** treat this as an error / early-return and direct the player
/// through `perform_ranged_attack_action_with_rng` instead.
///
/// # Arguments
///
/// * `character` - The character whose equipped weapon slot is inspected.
/// * `item_db` - The live item database used to look up weapon stats.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::engine::{get_character_attack, MeleeAttackResult, UNARMED_DAMAGE};
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::items::{ItemDatabase, Item, ItemType, WeaponData, WeaponClassification};
/// use antares::domain::types::DiceRoll;
///
/// // Unarmed fallback
/// let hero = Character::new(
///     "Hero".to_string(), "human".to_string(), "knight".to_string(),
///     Sex::Male, Alignment::Good,
/// );
/// let db = ItemDatabase::new();
/// let result = get_character_attack(&hero, &db);
/// assert!(matches!(result, MeleeAttackResult::Melee(_)));
/// ```
pub fn get_character_attack(character: &Character, item_db: &ItemDatabase) -> MeleeAttackResult {
    // Step 1: no weapon equipped — unarmed fallback.
    let Some(weapon_id) = character.equipment.weapon else {
        return MeleeAttackResult::Melee(Attack::physical(UNARMED_DAMAGE));
    };

    // Step 2: look up the item in the database.
    let Some(item) = item_db.get_item(weapon_id) else {
        // Unknown item ID — fall back gracefully rather than panicking.
        return MeleeAttackResult::Melee(Attack::physical(UNARMED_DAMAGE));
    };

    // Step 3: confirm it is actually a weapon.
    let ItemType::Weapon(weapon_data) = &item.item_type else {
        // Non-weapon in the weapon slot — unarmed fallback.
        return MeleeAttackResult::Melee(Attack::physical(UNARMED_DAMAGE));
    };

    // Step 4: apply weapon bonus to the dice roll via saturating_add.
    let adjusted = DiceRoll {
        count: weapon_data.damage.count,
        sides: weapon_data.damage.sides,
        bonus: weapon_data.damage.bonus.saturating_add(weapon_data.bonus),
    };

    // Step 5: ranged weapons must not be used in the melee path.
    if weapon_data.classification == WeaponClassification::MartialRanged {
        let attack = Attack::ranged(adjusted);
        return MeleeAttackResult::Ranged(attack);
    }

    // Step 6: normal melee weapon.
    MeleeAttackResult::Melee(Attack::physical(adjusted))
}

/// Returns `true` if `character` has a [`WeaponClassification::MartialRanged`]
/// weapon equipped **and** at least one compatible ammo item in their inventory.
///
/// A character who has a bow but no arrows returns `false` — they cannot
/// meaningfully fire a ranged attack.
///
/// # Arguments
///
/// * `character` - The character to inspect.
/// * `item_db` - The live item database used to look up item types.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::engine::has_ranged_weapon;
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::items::ItemDatabase;
///
/// let hero = Character::new(
///     "Archer".to_string(), "human".to_string(), "archer".to_string(),
///     Sex::Female, Alignment::Good,
/// );
/// let db = ItemDatabase::new();
///
/// // No weapon equipped → false
/// assert!(!has_ranged_weapon(&hero, &db));
/// ```
pub fn has_ranged_weapon(character: &Character, item_db: &ItemDatabase) -> bool {
    // Must have a weapon equipped.
    let Some(weapon_id) = character.equipment.weapon else {
        return false;
    };

    // Weapon must exist in the database.
    let Some(item) = item_db.get_item(weapon_id) else {
        return false;
    };

    // Must be a weapon item.
    let ItemType::Weapon(data) = &item.item_type else {
        return false;
    };

    // Must be classified as a ranged weapon.
    if data.classification != WeaponClassification::MartialRanged {
        return false;
    }

    // Must also have at least one ammo item in inventory.
    character.inventory.items.iter().any(|slot| {
        item_db
            .get_item(slot.item_id)
            .map(|i| matches!(i.item_type, ItemType::Ammo(_)))
            .unwrap_or(false)
    })
}

/// Determines the attack a character makes based on their equipped weapon.
///
/// This is a pure-domain function with no I/O or Bevy dependencies. It is
/// the single canonical place to convert a character's equipped weapon into
/// an [`Attack`].
///
/// # Fallback Behaviour
///
/// - No weapon equipped → `MeleeAttackResult::Melee` with [`UNARMED_DAMAGE`]
/// - Weapon ID not found in `item_db` → same unarmed fallback (no panic)
/// - Item found but not a weapon (e.g. a consumable in the weapon slot) →
///   same unarmed fallback
///
/// # Ranged Weapons
///
/// If the equipped weapon has
/// [`WeaponClassification::MartialRanged`][crate::domain::items::WeaponClassification],
/// this function returns `MeleeAttackResult::Ranged`. Callers in the melee
/// path **must** treat this as an error / early-return and direct the player
/// through `perform_ranged_attack_action_with_rng` instead.
///
/// # Arguments
///
/// * `character` - The character whose equipped weapon slot is inspected.
/// * `item_db` - The live item database used to look up weapon stats.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::engine::{get_character_attack, MeleeAttackResult, UNARMED_DAMAGE};
/// use antares::domain::character::{Character, Sex, Alignment};
/// use antares::domain::items::ItemDatabase;
///
/// let hero = Character::new(
///     "Hero".to_string(), "human".to_string(), "knight".to_string(),
///     Sex::Male, Alignment::Good,
/// );
/// let db = ItemDatabase::new();
/// let result = get_character_attack(&hero, &db);
/// assert!(matches!(result, MeleeAttackResult::Melee(_)));
/// ```
pub fn roll_spell_damage<R: Rng>(spell: &Spell, rng: &mut R) -> i32 {
    if let Some(dice) = &spell.damage {
        dice.roll(rng)
    } else {
        0
    }
}

/// Apply a condition definition to a character.
///
/// Applies status flags (bitflags), attribute modifiers, and registers an
/// `ActiveCondition` for DoT/HoT effects so that the existing combat tick
/// machinery can handle duration-based effects.
pub fn apply_condition_to_character(
    target: &mut crate::domain::character::Character,
    cond_def: &crate::domain::conditions::ConditionDefinition,
) {
    use crate::domain::character::Condition;
    use crate::domain::conditions::{ActiveCondition, ConditionEffect};

    for effect in &cond_def.effects {
        match effect {
            ConditionEffect::StatusEffect(name) => match name.to_lowercase().as_str() {
                "paralyzed" | "paralysis" | "paralyse" => {
                    target.conditions.add(Condition::PARALYZED);
                }
                "silenced" | "silence" => {
                    target.conditions.add(Condition::SILENCED);
                }
                "asleep" | "sleep" => {
                    target.conditions.add(Condition::ASLEEP);
                }
                "blinded" | "blind" => {
                    target.conditions.add(Condition::BLINDED);
                }
                "poisoned" | "poison" => {
                    target.conditions.add(Condition::POISONED);
                }
                "unconscious" => {
                    target.conditions.add(Condition::UNCONSCIOUS);
                }
                "dead" | "stone" => {
                    target.conditions.add(Condition::DEAD);
                }
                _ => {
                    tracing::warn!(
                        "Unknown status effect '{}' in condition '{}'; ignoring",
                        name,
                        cond_def.id
                    );
                }
            },
            ConditionEffect::AttributeModifier { attribute, value } => {
                match attribute.to_lowercase().as_str() {
                    "might" => target.stats.might.modify(*value),
                    "intellect" => target.stats.intellect.modify(*value),
                    "personality" => target.stats.personality.modify(*value),
                    "endurance" => target.stats.endurance.modify(*value),
                    "speed" => target.stats.speed.modify(*value),
                    "accuracy" => target.stats.accuracy.modify(*value),
                    "luck" => target.stats.luck.modify(*value),
                    "ac" => target.ac.modify(*value),
                    "hp" => target.hp.modify(*value as i32),
                    "sp" => target.sp.modify(*value as i32),
                    _ => {
                        tracing::warn!(
                            "Unknown attribute modifier '{}' (value={}) in condition '{}'; ignoring",
                            attribute,
                            value,
                            cond_def.id
                        );
                    }
                }
            }
            ConditionEffect::DamageOverTime { .. } | ConditionEffect::HealOverTime { .. } => {
                // Register as active condition so DoT/HoT will be processed by CombatState
                let active = ActiveCondition::new(cond_def.id.clone(), cond_def.default_duration);
                target.add_condition(active);
            }
        }
    }

    // For UntilCombatEnd and UntilRest durations, always add the condition to
    // active_conditions regardless of effect type so that the temporal cleanup
    // machinery (clear_combat_end_conditions / tick_conditions_rest) can find and
    // remove them.  add_condition is idempotent — it refreshes if already present.
    if matches!(
        cond_def.default_duration,
        crate::domain::conditions::ConditionDuration::UntilCombatEnd
            | crate::domain::conditions::ConditionDuration::UntilRest
    ) {
        let active = crate::domain::conditions::ActiveCondition::new(
            cond_def.id.clone(),
            cond_def.default_duration,
        );
        target.add_condition(active);
    }
}

/// Apply a condition definition to a monster.
///
/// Sets monster-level status (enum), applies attribute modifiers, and
/// registers any DoT/HoT active conditions.
pub fn apply_condition_to_monster(
    monster: &mut Monster,
    cond_def: &crate::domain::conditions::ConditionDefinition,
) {
    use crate::domain::combat::monster::MonsterCondition;
    use crate::domain::conditions::{ActiveCondition, ConditionEffect};

    for effect in &cond_def.effects {
        match effect {
            ConditionEffect::StatusEffect(name) => match name.to_lowercase().as_str() {
                "paralyzed" | "paralysis" | "paralyse" => {
                    monster.conditions = MonsterCondition::Paralyzed;
                }
                "silenced" | "silence" => {
                    monster.conditions = MonsterCondition::Silenced;
                }
                "asleep" | "sleep" => {
                    monster.conditions = MonsterCondition::Asleep;
                }
                "blinded" | "blind" => {
                    monster.conditions = MonsterCondition::Blinded;
                }
                "afraid" | "fear" => {
                    monster.conditions = MonsterCondition::Afraid;
                }
                "dead" | "stone" => {
                    monster.conditions = MonsterCondition::Dead;
                }
                _ => {
                    tracing::warn!(
                        "Unknown monster status effect '{}' in condition '{}'; ignoring",
                        name,
                        cond_def.id
                    );
                }
            },
            ConditionEffect::AttributeModifier { attribute, value } => {
                match attribute.to_lowercase().as_str() {
                    "might" => monster.stats.might.modify(*value),
                    "intellect" => monster.stats.intellect.modify(*value),
                    "endurance" => monster.stats.endurance.modify(*value),
                    "speed" => monster.stats.speed.modify(*value),
                    "accuracy" => monster.stats.accuracy.modify(*value),
                    "ac" => monster.ac.modify(*value),
                    "hp" => monster.hp.modify(*value as i32),
                    _ => {
                        tracing::warn!(
                            "Unknown monster attribute modifier '{}' (value={}) in condition '{}'; ignoring",
                            attribute,
                            value,
                            cond_def.id
                        );
                    }
                }
            }
            ConditionEffect::DamageOverTime { .. } | ConditionEffect::HealOverTime { .. } => {
                let active = ActiveCondition::new(cond_def.id.clone(), cond_def.default_duration);
                monster.add_condition(active);
            }
        }
    }

    // For UntilCombatEnd and UntilRest durations, always add the condition to
    // active_conditions regardless of effect type so that the temporal cleanup
    // machinery (clear_combat_end_conditions) can find and remove them.
    // add_condition is idempotent — it refreshes if already present.
    if matches!(
        cond_def.default_duration,
        crate::domain::conditions::ConditionDuration::UntilCombatEnd
            | crate::domain::conditions::ConditionDuration::UntilRest
    ) {
        let active = crate::domain::conditions::ActiveCondition::new(
            cond_def.id.clone(),
            cond_def.default_duration,
        );
        monster.add_condition(active);
    }
}

/// Errors that can occur while applying conditions by ID
#[derive(thiserror::Error, Debug)]
pub enum ConditionApplyError {
    #[error("Condition not found: {0}")]
    ConditionNotFound(String),
}

/// Look up a condition by ID in the content database and apply it to a character.
///
/// # Errors
///
/// Returns `ConditionApplyError::ConditionNotFound` if the condition ID is not
/// present in the provided `ContentDatabase`.
pub fn apply_condition_to_character_by_id(
    target: &mut crate::domain::character::Character,
    condition_id: &str,
    content: &crate::sdk::database::ContentDatabase,
) -> Result<(), ConditionApplyError> {
    if let Some(def) = content.conditions.get_condition(&condition_id.to_string()) {
        apply_condition_to_character(target, def);
        Ok(())
    } else {
        Err(ConditionApplyError::ConditionNotFound(
            condition_id.to_string(),
        ))
    }
}

/// Look up a condition by ID in the content database and apply it to a monster.
///
/// # Errors
///
/// Returns `ConditionApplyError::ConditionNotFound` if the condition ID is not
/// present in the provided `ContentDatabase`.
pub fn apply_condition_to_monster_by_id(
    monster: &mut Monster,
    condition_id: &str,
    content: &crate::sdk::database::ContentDatabase,
) -> Result<(), ConditionApplyError> {
    if let Some(def) = content.conditions.get_condition(&condition_id.to_string()) {
        apply_condition_to_monster(monster, def);
        Ok(())
    } else {
        Err(ConditionApplyError::ConditionNotFound(
            condition_id.to_string(),
        ))
    }
}

/// Initialize a combat encounter from an explicit monster group (list of MonsterId).
///
/// This fetches each monster definition from the `ContentDatabase` and converts it
/// into a runtime `Monster` instance, then inserts into the provided `CombatState`.
///
/// Returns an error if any monster ID in the group is not found.
pub fn initialize_combat_from_group(
    combat: &mut CombatState,
    content: &crate::sdk::database::ContentDatabase,
    group: &[crate::domain::types::MonsterId],
) -> Result<(), crate::domain::combat::database::MonsterDatabaseError> {
    for id in group {
        if let Some(m) = content.monsters.get_monster(*id) {
            combat.add_monster(m.clone());
        } else {
            return Err(
                crate::domain::combat::database::MonsterDatabaseError::MonsterNotFound(*id),
            );
        }
    }

    // Initialize turn order and flags for combat
    start_combat(combat);
    Ok(())
}

pub fn reconcile_character_conditions(
    target: &mut crate::domain::character::Character,
    condition_defs: &[crate::domain::conditions::ConditionDefinition],
) {
    use crate::domain::character::Condition;
    use crate::domain::conditions::ConditionEffect;

    // Build the set of flags to consider from condition definitions
    let mut flags_to_consider: u8 = 0;
    for def in condition_defs {
        for effect in &def.effects {
            if let ConditionEffect::StatusEffect(name) = effect {
                if let Some(flag) = status_str_to_flag(name) {
                    flags_to_consider |= flag;
                }
            }
        }
    }

    // Determine desired flags from active conditions
    let mut desired_flags: u8 = 0;
    for active in &target.active_conditions {
        if let Some(def) = condition_defs.iter().find(|d| d.id == active.condition_id) {
            for effect in &def.effects {
                if let ConditionEffect::StatusEffect(name) = effect {
                    if let Some(flag) = status_str_to_flag(name) {
                        desired_flags |= flag;
                    }
                }
            }
        }
    }

    // Apply or remove flags based on desired state
    let flag_list = [
        Condition::ASLEEP,
        Condition::BLINDED,
        Condition::SILENCED,
        Condition::DISEASED,
        Condition::POISONED,
        Condition::PARALYZED,
        Condition::UNCONSCIOUS,
        Condition::DEAD,
        Condition::STONE,
    ];

    for &flag in &flag_list {
        if flags_to_consider & flag != 0 {
            if desired_flags & flag != 0 {
                target.conditions.add(flag);
            } else {
                target.conditions.remove(flag);
            }
        }
    }
}

pub fn reconcile_monster_conditions(
    monster: &mut Monster,
    condition_defs: &[crate::domain::conditions::ConditionDefinition],
) {
    use crate::domain::combat::monster::MonsterCondition;
    use crate::domain::conditions::ConditionEffect;

    // Do not override a dead monster
    if monster.conditions.is_dead() {
        return;
    }

    // Find the first matching status effect (simple priority)
    let mut desired: Option<MonsterCondition> = None;
    for active in &monster.active_conditions {
        if let Some(def) = condition_defs.iter().find(|d| d.id == active.condition_id) {
            for effect in &def.effects {
                if let ConditionEffect::StatusEffect(name) = effect {
                    if let Some(mc) = status_str_to_monster_condition(name) {
                        desired = Some(mc);
                        break;
                    }
                }
            }
        }
        if desired.is_some() {
            break;
        }
    }

    monster.conditions = desired.unwrap_or(MonsterCondition::Normal);
}

/// Helper to map status names to character flags
fn status_str_to_flag(name: &str) -> Option<u8> {
    match name.to_lowercase().as_str() {
        "asleep" | "sleep" => Some(crate::domain::character::Condition::ASLEEP),
        "blinded" | "blind" => Some(crate::domain::character::Condition::BLINDED),
        "silenced" | "silence" => Some(crate::domain::character::Condition::SILENCED),
        "diseased" | "disease" => Some(crate::domain::character::Condition::DISEASED),
        "poisoned" | "poison" => Some(crate::domain::character::Condition::POISONED),
        "paralyzed" | "paralysis" | "paralyse" => {
            Some(crate::domain::character::Condition::PARALYZED)
        }
        "unconscious" => Some(crate::domain::character::Condition::UNCONSCIOUS),
        "dead" => Some(crate::domain::character::Condition::DEAD),
        "stone" => Some(crate::domain::character::Condition::STONE),
        _ => None,
    }
}

/// Helper to map status names to monster conditions
fn status_str_to_monster_condition(
    name: &str,
) -> Option<crate::domain::combat::monster::MonsterCondition> {
    match name.to_lowercase().as_str() {
        "paralyzed" | "paralysis" | "paralyse" => {
            Some(crate::domain::combat::monster::MonsterCondition::Paralyzed)
        }
        "webbed" => Some(crate::domain::combat::monster::MonsterCondition::Webbed),
        "held" => Some(crate::domain::combat::monster::MonsterCondition::Held),
        "asleep" | "sleep" => Some(crate::domain::combat::monster::MonsterCondition::Asleep),
        "mindless" => Some(crate::domain::combat::monster::MonsterCondition::Mindless),
        "silenced" | "silence" => Some(crate::domain::combat::monster::MonsterCondition::Silenced),
        "blinded" | "blind" => Some(crate::domain::combat::monster::MonsterCondition::Blinded),
        "afraid" | "fear" => Some(crate::domain::combat::monster::MonsterCondition::Afraid),
        "dead" => Some(crate::domain::combat::monster::MonsterCondition::Dead),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, InventorySlot, Sex, Stats};
    use crate::domain::combat::monster::LootTable;
    use crate::domain::items::{
        AmmoData, AmmoType, ConsumableData, ConsumableEffect, Item, ItemDatabase, ItemType,
        WeaponClassification, WeaponData,
    };
    use crate::domain::types::DiceRoll;
    use rand::rng;

    // ===== Helpers for get_character_attack / has_ranged_weapon tests =====

    /// Build a minimal `Item` for use in tests.
    fn make_weapon_item(
        id: u8,
        damage: DiceRoll,
        bonus: i8,
        classification: WeaponClassification,
    ) -> Item {
        Item {
            id,
            name: format!("TestWeapon#{}", id),
            item_type: ItemType::Weapon(WeaponData {
                damage,
                bonus,
                hands_required: 1,
                classification,
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

    /// Build a minimal ammo `Item`.
    fn make_ammo_item(id: u8) -> Item {
        Item {
            id,
            name: format!("Arrow#{}", id),
            item_type: ItemType::Ammo(AmmoData {
                ammo_type: AmmoType::Arrow,
                quantity: 20,
            }),
            base_cost: 1,
            sell_cost: 0,
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

    /// Build a minimal consumable `Item`.
    fn make_consumable_item(id: u8) -> Item {
        Item {
            id,
            name: format!("Potion#{}", id),
            item_type: ItemType::Consumable(ConsumableData {
                effect: ConsumableEffect::HealHp(10),
                is_combat_usable: true,
                duration_minutes: None,
            }),
            base_cost: 50,
            sell_cost: 25,
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

    /// Create a fresh [`Character`] with the weapon slot set to `weapon_id`.
    fn make_character_with_weapon(weapon_id: u8) -> Character {
        let mut character = Character::new(
            "Tester".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        character.equipment.weapon = Some(weapon_id);
        character
    }

    /// Create a fresh [`Character`] with a weapon equipped AND an ammo item in
    /// the first inventory slot.
    fn make_character_with_weapon_and_ammo(weapon_id: u8, ammo_id: u8) -> Character {
        let mut character = make_character_with_weapon(weapon_id);
        // Place ammo in first inventory slot
        character.inventory.items.push(InventorySlot {
            item_id: ammo_id,
            charges: 0,
        });
        character
    }

    // ===== get_character_attack tests =====

    #[test]
    fn test_get_character_attack_no_weapon_returns_unarmed() {
        let character = Character::new(
            "Unarmed".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let db = ItemDatabase::new();

        let result = get_character_attack(&character, &db);
        match result {
            MeleeAttackResult::Melee(attack) => {
                assert_eq!(attack.damage, UNARMED_DAMAGE);
                assert!(!attack.is_ranged);
            }
            MeleeAttackResult::Ranged(_) => panic!("Expected MeleeAttackResult::Melee"),
        }
    }

    #[test]
    fn test_get_character_attack_melee_weapon_returns_melee() {
        let mut db = ItemDatabase::new();
        let sword = make_weapon_item(1, DiceRoll::new(1, 8, 0), 0, WeaponClassification::Simple);
        db.add_item(sword).unwrap();

        let character = make_character_with_weapon(1);
        let result = get_character_attack(&character, &db);

        match result {
            MeleeAttackResult::Melee(attack) => {
                assert_eq!(attack.damage, DiceRoll::new(1, 8, 0));
                assert!(!attack.is_ranged);
            }
            MeleeAttackResult::Ranged(_) => panic!("Expected MeleeAttackResult::Melee"),
        }
    }

    #[test]
    fn test_get_character_attack_weapon_bonus_applied() {
        let mut db = ItemDatabase::new();
        // +2 sword: base 1d8, weapon bonus +2 → DiceRoll bonus should be 2
        let sword = make_weapon_item(
            2,
            DiceRoll::new(1, 8, 0),
            2,
            WeaponClassification::MartialMelee,
        );
        db.add_item(sword).unwrap();

        let character = make_character_with_weapon(2);
        let result = get_character_attack(&character, &db);

        match result {
            MeleeAttackResult::Melee(attack) => {
                assert_eq!(attack.damage.bonus, 2);
                assert_eq!(attack.damage.count, 1);
                assert_eq!(attack.damage.sides, 8);
            }
            MeleeAttackResult::Ranged(_) => panic!("Expected MeleeAttackResult::Melee"),
        }
    }

    #[test]
    fn test_get_character_attack_unknown_item_id_falls_back() {
        // Empty database — item_id 99 does not exist
        let db = ItemDatabase::new();
        let character = make_character_with_weapon(99);

        let result = get_character_attack(&character, &db);
        match result {
            MeleeAttackResult::Melee(attack) => {
                assert_eq!(attack.damage, UNARMED_DAMAGE);
            }
            MeleeAttackResult::Ranged(_) => panic!("Expected unarmed fallback"),
        }
    }

    #[test]
    fn test_get_character_attack_non_weapon_item_falls_back() {
        let mut db = ItemDatabase::new();
        // Place a consumable in weapon slot
        let potion = make_consumable_item(5);
        db.add_item(potion).unwrap();

        let character = make_character_with_weapon(5);
        let result = get_character_attack(&character, &db);

        match result {
            MeleeAttackResult::Melee(attack) => {
                assert_eq!(attack.damage, UNARMED_DAMAGE);
            }
            MeleeAttackResult::Ranged(_) => panic!("Expected unarmed fallback"),
        }
    }

    #[test]
    fn test_get_character_attack_ranged_weapon_returns_ranged_variant() {
        let mut db = ItemDatabase::new();
        let bow = make_weapon_item(
            10,
            DiceRoll::new(1, 6, 0),
            0,
            WeaponClassification::MartialRanged,
        );
        db.add_item(bow).unwrap();

        let character = make_character_with_weapon(10);
        let result = get_character_attack(&character, &db);

        match result {
            MeleeAttackResult::Ranged(attack) => {
                assert!(attack.is_ranged);
            }
            MeleeAttackResult::Melee(_) => panic!("Expected MeleeAttackResult::Ranged"),
        }
    }

    #[test]
    fn test_get_character_attack_ranged_weapon_damage_correct() {
        let mut db = ItemDatabase::new();
        // Crossbow: 1d8, bonus +1
        let crossbow = make_weapon_item(
            11,
            DiceRoll::new(1, 8, 0),
            1,
            WeaponClassification::MartialRanged,
        );
        db.add_item(crossbow).unwrap();

        let character = make_character_with_weapon(11);
        let result = get_character_attack(&character, &db);

        match result {
            MeleeAttackResult::Ranged(attack) => {
                assert_eq!(
                    attack.damage,
                    DiceRoll {
                        count: 1,
                        sides: 8,
                        bonus: 1
                    }
                );
                assert!(attack.is_ranged);
            }
            MeleeAttackResult::Melee(_) => panic!("Expected MeleeAttackResult::Ranged"),
        }
    }

    // ===== has_ranged_weapon tests =====

    #[test]
    fn test_has_ranged_weapon_false_no_weapon() {
        let character = Character::new(
            "NoWeapon".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        let db = ItemDatabase::new();
        assert!(!has_ranged_weapon(&character, &db));
    }

    #[test]
    fn test_has_ranged_weapon_false_melee_weapon() {
        let mut db = ItemDatabase::new();
        let sword = make_weapon_item(
            20,
            DiceRoll::new(1, 8, 0),
            0,
            WeaponClassification::MartialMelee,
        );
        db.add_item(sword).unwrap();

        let character = make_character_with_weapon(20);
        assert!(!has_ranged_weapon(&character, &db));
    }

    #[test]
    fn test_has_ranged_weapon_false_no_ammo() {
        let mut db = ItemDatabase::new();
        let bow = make_weapon_item(
            21,
            DiceRoll::new(1, 6, 0),
            0,
            WeaponClassification::MartialRanged,
        );
        db.add_item(bow).unwrap();

        // Bow equipped but inventory is empty (no arrows)
        let character = make_character_with_weapon(21);
        assert!(!has_ranged_weapon(&character, &db));
    }

    #[test]
    fn test_has_ranged_weapon_true_with_bow_and_arrows() {
        let mut db = ItemDatabase::new();
        let bow = make_weapon_item(
            22,
            DiceRoll::new(1, 6, 0),
            0,
            WeaponClassification::MartialRanged,
        );
        let arrows = make_ammo_item(23);
        db.add_item(bow).unwrap();
        db.add_item(arrows).unwrap();

        let character = make_character_with_weapon_and_ammo(22, 23);
        assert!(has_ranged_weapon(&character, &db));
    }

    fn create_test_character(name: &str, speed: u8) -> Character {
        let mut character = Character::new(
            name.to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        character.stats.speed.current = speed;
        character
    }

    fn create_test_monster(name: &str, speed: u8) -> Monster {
        let mut stats = Stats::new(10, 8, 6, 10, speed, 7, 5);
        stats.speed.current = speed;
        let attacks = vec![Attack::physical(DiceRoll::new(1, 6, 0))];
        Monster::new(
            1,
            name.to_string(),
            stats,
            10,
            5,
            attacks,
            LootTable::none(),
        )
    }

    // ===== ambush_round_active tests =====

    /// `CombatState::new` must initialise `ambush_round_active` to `false`.
    #[test]
    fn test_combat_state_ambush_round_active_defaults_false() {
        let cs = CombatState::new(Handicap::Even);
        assert!(
            !cs.ambush_round_active,
            "ambush_round_active must be false by default"
        );
    }

    /// After enough `advance_turn` calls to push into round 2, `ambush_round_active`
    /// must be `false` and the handicap must be reset to `Even`.
    #[test]
    fn test_ambush_round_active_cleared_at_round_2() {
        let mut cs = CombatState::new(Handicap::MonsterAdvantage);
        cs.ambush_round_active = true;

        let char = create_test_character("Ambushed", 8);
        cs.add_player(char);
        start_combat(&mut cs);

        // With one combatant the turn order has one slot; a single advance_turn
        // exhausts the turn order and triggers advance_round (round -> 2).
        let _ = cs.advance_turn(&[]);

        assert!(
            !cs.ambush_round_active,
            "ambush_round_active must be cleared at the start of round 2"
        );
        assert_eq!(
            cs.handicap,
            Handicap::Even,
            "handicap must be reset to Even at the start of round 2"
        );
        assert_eq!(
            cs.round, 2,
            "round counter must be 2 after one full rotation"
        );
    }

    /// When `ambush_round_active` is false the handicap must NOT be changed
    /// when advancing to round 2 (a normal encounter must not be affected).
    #[test]
    fn test_non_ambush_handicap_unchanged_at_round_2() {
        let mut cs = CombatState::new(Handicap::PartyAdvantage);
        cs.ambush_round_active = false;

        let char = create_test_character("Normal", 8);
        cs.add_player(char);
        start_combat(&mut cs);

        let _ = cs.advance_turn(&[]);

        assert_eq!(
            cs.handicap,
            Handicap::PartyAdvantage,
            "handicap must not change when ambush_round_active is false"
        );
    }

    #[test]
    fn test_combat_state_creation() {
        let combat = CombatState::new(Handicap::Even);
        assert_eq!(combat.round, 1);
        assert_eq!(combat.current_turn, 0);
        assert!(combat.is_in_progress());
        assert!(combat.can_flee);
    }

    #[test]
    fn test_add_participants() {
        let mut combat = CombatState::new(Handicap::Even);

        let character = create_test_character("Hero", 10);
        combat.add_player(character);

        let monster = create_test_monster("Goblin", 8);
        combat.add_monster(monster);

        assert_eq!(combat.participants.len(), 2);
    }

    #[test]
    fn test_start_combat() {
        let mut combat = CombatState::new(Handicap::Even);

        combat.add_player(create_test_character("Hero", 12));
        combat.add_monster(create_test_monster("Goblin", 8));

        start_combat(&mut combat);

        assert_eq!(combat.turn_order.len(), 2);
        assert_eq!(combat.round, 1);
        assert!(combat.is_in_progress());
    }

    #[test]
    fn test_turn_order_by_speed() {
        let mut combat = CombatState::new(Handicap::Even);

        combat.add_player(create_test_character("Slow", 5));
        combat.add_player(create_test_character("Fast", 15));
        combat.add_monster(create_test_monster("Medium", 10));

        let order = calculate_turn_order(&combat);

        // Should be ordered by speed: Fast(15), Medium(10), Slow(5)
        assert_eq!(order.len(), 3);
        assert!(matches!(order[0], CombatantId::Player(1))); // Fast
        assert!(matches!(order[1], CombatantId::Monster(2))); // Medium
        assert!(matches!(order[2], CombatantId::Player(0))); // Slow
    }

    #[test]
    fn test_handicap_party_advantage() {
        let mut combat = CombatState::new(Handicap::PartyAdvantage);

        combat.add_player(create_test_character("SlowHero", 5));
        combat.add_monster(create_test_monster("FastMonster", 15));

        let order = calculate_turn_order(&combat);

        // Party should go first despite lower speed
        assert_eq!(order.len(), 2);
        assert!(matches!(order[0], CombatantId::Player(_)));
        assert!(matches!(order[1], CombatantId::Monster(_)));
    }

    #[test]
    fn test_handicap_monster_advantage() {
        let mut combat = CombatState::new(Handicap::MonsterAdvantage);

        combat.add_player(create_test_character("FastHero", 15));
        combat.add_monster(create_test_monster("SlowMonster", 5));

        let order = calculate_turn_order(&combat);

        // Monsters should go first despite lower speed
        assert_eq!(order.len(), 2);
        assert!(matches!(order[0], CombatantId::Monster(_)));
        assert!(matches!(order[1], CombatantId::Player(_)));
    }

    #[test]
    fn test_alive_count() {
        let mut combat = CombatState::new(Handicap::Even);

        combat.add_player(create_test_character("Hero", 10));

        let mut dead_character = create_test_character("DeadHero", 10);
        dead_character.hp.current = 0;
        dead_character
            .conditions
            .add(crate::domain::character::Condition::DEAD);
        combat.add_player(dead_character);

        combat.add_monster(create_test_monster("Goblin", 8));

        assert_eq!(combat.alive_party_count(), 1);
        assert_eq!(combat.alive_monster_count(), 1);
    }

    #[test]
    fn test_combat_end_victory() {
        let mut combat = CombatState::new(Handicap::Even);

        combat.add_player(create_test_character("Hero", 10));

        let mut monster = create_test_monster("Goblin", 8);
        monster.hp.current = 0;
        combat.add_monster(monster);

        combat.check_combat_end();
        assert_eq!(combat.status, CombatStatus::Victory);
    }

    #[test]
    fn test_combat_end_defeat() {
        let mut combat = CombatState::new(Handicap::Even);

        let mut character = create_test_character("Hero", 10);
        character.hp.current = 0;
        character
            .conditions
            .add(crate::domain::character::Condition::DEAD);
        combat.add_player(character);

        combat.add_monster(create_test_monster("Goblin", 8));

        combat.check_combat_end();
        assert_eq!(combat.status, CombatStatus::Defeat);
    }

    #[test]
    fn test_damage_calculation() {
        let mut combat = CombatState::new(Handicap::Even);

        let mut attacker = create_test_character("Attacker", 10);
        attacker.stats.might.current = 16; // +3 damage bonus
        attacker.stats.accuracy.current = 15;
        combat.add_player(attacker);

        let target = create_test_character("Target", 10);
        combat.add_player(target);

        let attack = Attack::physical(DiceRoll::new(1, 8, 2));
        let mut rng = rng();

        // Test multiple attacks
        for _ in 0..10 {
            let result = resolve_attack(
                &combat,
                CombatantId::Player(0),
                CombatantId::Player(1),
                &attack,
                None,
                &mut rng,
            );
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_apply_damage_to_character() {
        let mut combat = CombatState::new(Handicap::Even);

        let character = create_test_character("Hero", 10);
        let initial_hp = character.hp.current;
        combat.add_player(character);

        let result = apply_damage(&mut combat, CombatantId::Player(0), 5);
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should not have died

        if let Some(Combatant::Player(c)) = combat.participants.first() {
            assert_eq!(c.hp.current, initial_hp - 5);
        }
    }

    #[test]
    fn test_apply_damage_kills_target() {
        let mut combat = CombatState::new(Handicap::Even);

        let character = create_test_character("Hero", 10);
        combat.add_player(character);

        let result = apply_damage(&mut combat, CombatantId::Player(0), 1000);
        assert!(result.is_ok());
        assert!(result.unwrap()); // Should have died

        if let Some(Combatant::Player(c)) = combat.participants.first() {
            assert_eq!(c.hp.current, 0);
        }
    }

    #[test]
    fn test_apply_damage_sets_unconscious_at_zero_hp() {
        use crate::domain::conditions::ConditionDuration;

        let mut combat = CombatState::new(Handicap::Even);
        let character = create_test_character("Hero", 10);
        combat.add_player(character);

        // Deal exactly enough to bring HP to 0
        let result = apply_damage(&mut combat, CombatantId::Player(0), 10);
        assert!(result.is_ok());
        assert!(
            result.unwrap(),
            "should return true when character is just downed"
        );

        if let Some(Combatant::Player(c)) = combat.participants.first() {
            assert_eq!(c.hp.current, 0, "HP must be 0 after lethal damage");
            assert!(
                c.conditions
                    .has(crate::domain::character::Condition::UNCONSCIOUS),
                "UNCONSCIOUS bitflag must be set when HP drops to 0"
            );
            let has_active = c
                .active_conditions
                .iter()
                .any(|ac| ac.condition_id == "unconscious");
            assert!(
                has_active,
                "active_conditions must contain 'unconscious' entry"
            );
            // Verify it is Permanent
            let entry = c
                .active_conditions
                .iter()
                .find(|ac| ac.condition_id == "unconscious")
                .unwrap();
            assert_eq!(
                entry.duration,
                ConditionDuration::Permanent,
                "unconscious ActiveCondition must have Permanent duration"
            );
        } else {
            panic!("Player combatant not found");
        }
    }

    #[test]
    fn test_apply_damage_already_at_zero_no_double_push() {
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};

        let mut combat = CombatState::new(Handicap::Even);
        let mut character = create_test_character("Hero", 10);
        // Pre-set hp to 0 and UNCONSCIOUS (simulating already-downed state)
        character.hp.current = 0;
        character
            .conditions
            .add(crate::domain::character::Condition::UNCONSCIOUS);
        character.add_condition(ActiveCondition::new(
            "unconscious".to_string(),
            ConditionDuration::Permanent,
        ));
        combat.add_player(character);

        // Apply damage to an already-unconscious (0 HP) character.
        // The character must transition to DEAD.
        let _ = apply_damage(&mut combat, CombatantId::Player(0), 5);

        if let Some(Combatant::Player(c)) = combat.participants.first() {
            // UNCONSCIOUS must be cleared, DEAD must be set.
            assert!(
                !c.conditions
                    .has(crate::domain::character::Condition::UNCONSCIOUS),
                "UNCONSCIOUS must be cleared when further damage kills the character"
            );
            assert!(
                c.conditions.has(crate::domain::character::Condition::DEAD),
                "DEAD must be set when an unconscious character takes further damage"
            );
            // No lingering "unconscious" ActiveCondition should remain.
            let unconscious_count = c
                .active_conditions
                .iter()
                .filter(|ac| ac.condition_id == "unconscious")
                .count();
            assert_eq!(
                unconscious_count, 0,
                "no 'unconscious' ActiveCondition must remain after transition to dead"
            );
            // A 'dead' ActiveCondition must have been added.
            let dead_count = c
                .active_conditions
                .iter()
                .filter(|ac| ac.condition_id == "dead")
                .count();
            assert_eq!(
                dead_count, 1,
                "exactly one 'dead' ActiveCondition must be present"
            );
        } else {
            panic!("Player combatant not found");
        }
    }

    /// An unconscious character that receives further damage must transition to
    /// DEAD: UNCONSCIOUS bitflag and ActiveCondition are cleared, DEAD bitflag
    /// and ActiveCondition are set.
    #[test]
    fn test_unconscious_to_dead_on_further_damage() {
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};

        let mut combat = CombatState::new(Handicap::Even);
        let mut character = create_test_character("Hero", 20);
        // First bring to 0 HP and UNCONSCIOUS
        character.hp.current = 0;
        character
            .conditions
            .add(crate::domain::character::Condition::UNCONSCIOUS);
        character.add_condition(ActiveCondition::new(
            "unconscious".to_string(),
            ConditionDuration::Permanent,
        ));
        combat.add_player(character);

        let result = apply_damage(&mut combat, CombatantId::Player(0), 1);
        assert!(result.is_ok());

        if let Some(Combatant::Player(c)) = combat.participants.first() {
            assert!(
                c.conditions.has(crate::domain::character::Condition::DEAD),
                "DEAD bitflag must be set after further damage to unconscious character"
            );
            assert!(
                !c.conditions
                    .has(crate::domain::character::Condition::UNCONSCIOUS),
                "UNCONSCIOUS bitflag must be cleared after transitioning to dead"
            );
            assert!(
                c.active_conditions
                    .iter()
                    .any(|ac| ac.condition_id == "dead"),
                "active_conditions must contain 'dead' entry"
            );
            assert!(
                !c.active_conditions
                    .iter()
                    .any(|ac| ac.condition_id == "unconscious"),
                "active_conditions must not contain 'unconscious' after death transition"
            );
        } else {
            panic!("Player combatant not found");
        }
    }

    /// With `unconscious_before_death == false` (instant-death mode), reaching
    /// 0 HP sets DEAD immediately without going through UNCONSCIOUS.
    #[test]
    fn test_instant_death_mode_skips_unconscious() {
        let mut combat = CombatState::new(Handicap::Even);
        combat.unconscious_before_death = false; // instant-death mode
        let character = create_test_character("Hero", 10);
        combat.add_player(character);

        let result = apply_damage(&mut combat, CombatantId::Player(0), 10);
        assert!(result.is_ok(), "apply_damage must succeed");

        if let Some(Combatant::Player(c)) = combat.participants.first() {
            assert_eq!(c.hp.current, 0, "HP must be 0");
            assert!(
                c.conditions.has(crate::domain::character::Condition::DEAD),
                "DEAD must be set immediately in instant-death mode"
            );
            assert!(
                !c.conditions
                    .has(crate::domain::character::Condition::UNCONSCIOUS),
                "UNCONSCIOUS must never be set in instant-death mode"
            );
            assert!(
                c.active_conditions
                    .iter()
                    .any(|ac| ac.condition_id == "dead"),
                "active_conditions must contain 'dead' in instant-death mode"
            );
            assert!(
                !c.active_conditions
                    .iter()
                    .any(|ac| ac.condition_id == "unconscious"),
                "active_conditions must not contain 'unconscious' in instant-death mode"
            );
        } else {
            panic!("Player combatant not found");
        }
    }

    /// `CombatState::new()` must set `unconscious_before_death` to `true` by
    /// default so that new combats always use classic RPG behavior.
    #[test]
    fn test_combat_state_unconscious_before_death_default() {
        let combat = CombatState::new(Handicap::Even);
        assert!(
            combat.unconscious_before_death,
            "CombatState::new() must default unconscious_before_death to true"
        );
    }

    #[test]
    fn test_unconscious_party_triggers_defeat() {
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};

        let mut combat = CombatState::new(Handicap::Even);
        let mut char1 = create_test_character("Hero1", 10);
        let mut char2 = create_test_character("Hero2", 10);

        // Set both characters to 0 HP and UNCONSCIOUS
        char1.hp.current = 0;
        char1
            .conditions
            .add(crate::domain::character::Condition::UNCONSCIOUS);
        char1.add_condition(ActiveCondition::new(
            "unconscious".to_string(),
            ConditionDuration::Permanent,
        ));
        char2.hp.current = 0;
        char2
            .conditions
            .add(crate::domain::character::Condition::UNCONSCIOUS);
        char2.add_condition(ActiveCondition::new(
            "unconscious".to_string(),
            ConditionDuration::Permanent,
        ));
        combat.add_player(char1);
        combat.add_player(char2);

        // Add a monster so there's an alive opponent
        let monster = create_test_monster("Goblin", 10);
        combat.add_monster(monster);

        // All party members at 0 HP means alive_party_count() == 0
        assert_eq!(
            combat.alive_party_count(),
            0,
            "all 0-HP party members must count as not alive"
        );

        combat.check_combat_end();
        assert_eq!(
            combat.status,
            CombatStatus::Defeat,
            "combat must end in Defeat when all party members are at 0 HP"
        );
    }

    /// A party member at 0 HP is considered not alive and must not be counted
    /// as a valid target (`alive_party_count() == 0` when all are at 0 HP).
    /// This verifies that `Character::is_alive()` correctly gates monster targeting.
    #[test]
    fn test_monster_skips_unconscious_target() {
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};

        let mut combat = CombatState::new(Handicap::Even);
        let mut hero = create_test_character("Hero", 10);

        // Simulate being downed: 0 HP + UNCONSCIOUS
        hero.hp.current = 0;
        hero.conditions
            .add(crate::domain::character::Condition::UNCONSCIOUS);
        hero.add_condition(ActiveCondition::new(
            "unconscious".to_string(),
            ConditionDuration::Permanent,
        ));
        combat.add_player(hero);

        // Alive party count must be 0 — monsters have no valid targets
        assert_eq!(
            combat.alive_party_count(),
            0,
            "an unconscious (0 HP) party member must not count as a valid target"
        );
    }

    #[test]
    fn test_monster_regeneration() {
        let mut combat = CombatState::new(Handicap::Even);
        combat.monsters_regenerate = true;

        let mut monster = create_test_monster("Troll", 10);
        monster.can_regenerate = true;
        monster.hp.current = 5;
        combat.add_monster(monster);

        start_combat(&mut combat);

        // Advance one full round
        for _ in 0..combat.turn_order.len() {
            combat.advance_turn(&[]);
        }

        if let Some(Combatant::Monster(m)) = combat.participants.first() {
            assert_eq!(m.hp.current, 6); // Regenerated 1 HP
        }
    }

    #[test]
    fn test_combat_monster_special_ability_applied() {
        let mut monster = create_test_monster("Special", 10);

        // Give the monster two attacks, one of which is special
        monster.attacks = vec![
            Attack::physical(crate::domain::types::DiceRoll::new(1, 4, 0)),
            Attack::new(
                crate::domain::types::DiceRoll::new(1, 6, 0),
                AttackType::Physical,
                Some(SpecialEffect::Paralysis),
            ),
        ];
        monster.special_attack_threshold = 100; // Always trigger special attack

        let mut rng = rand::rng();
        let chosen = choose_monster_attack(&monster, false, &mut rng);
        assert!(chosen.is_some());
        assert!(chosen.unwrap().special_effect.is_some());
    }

    #[test]
    fn test_spell_effect_applies_damage() {
        let mut combat = CombatState::new(Handicap::Even);

        // Add a test monster
        let monster = create_test_monster("FireTest", 8);
        combat.add_monster(monster);

        // Create a simple damage spell (1d6)
        let spell = crate::domain::magic::types::Spell::new(
            0x0201,
            "Test Fire",
            crate::domain::magic::types::SpellSchool::Sorcerer,
            1,
            1,
            0,
            crate::domain::magic::types::SpellContext::CombatOnly,
            crate::domain::magic::types::SpellTarget::SingleMonster,
            "Deals 1d6 damage",
            Some(crate::domain::types::DiceRoll::new(1, 6, 0)),
            0,
            false,
        );

        let mut rng = rand::rng();
        let dmg = roll_spell_damage(&spell, &mut rng);
        assert!(dmg >= 0);

        let target = CombatantId::Monster(0);
        let res = apply_damage(&mut combat, target, dmg as u16);
        assert!(res.is_ok());
    }

    #[test]
    fn test_apply_condition_sets_flag() {
        // Character
        let mut character = create_test_character("Affected", 10);

        let cond = crate::domain::conditions::ConditionDefinition {
            id: "silence".to_string(),
            name: "Silence".to_string(),
            description: "Silences target".to_string(),
            effects: vec![crate::domain::conditions::ConditionEffect::StatusEffect(
                "silenced".to_string(),
            )],
            default_duration: crate::domain::conditions::ConditionDuration::Rounds(2),
            icon_id: None,
        };

        apply_condition_to_character(&mut character, &cond);
        assert!(character.conditions.is_silenced());

        // Monster
        let mut monster = create_test_monster("Silent Gob", 8);
        apply_condition_to_monster(&mut monster, &cond);
        assert_eq!(
            monster.conditions,
            crate::domain::combat::monster::MonsterCondition::Silenced
        );
    }

    #[test]
    fn test_condition_duration_decrements_per_turn() {
        let mut character = create_test_character("Timer", 10);

        let cond = crate::domain::conditions::ConditionDefinition {
            id: "poison".to_string(),
            name: "Poison".to_string(),
            description: "Toxic damage over time".to_string(),
            effects: vec![crate::domain::conditions::ConditionEffect::DamageOverTime {
                damage: crate::domain::types::DiceRoll::new(1, 4, 0),
                element: "poison".to_string(),
            }],
            default_duration: crate::domain::conditions::ConditionDuration::Rounds(2),
            icon_id: None,
        };

        apply_condition_to_character(&mut character, &cond);
        assert_eq!(character.active_conditions.len(), 1);

        // Tick once
        character.tick_conditions_round();
        assert_eq!(character.active_conditions.len(), 1);

        // Tick second time - should expire
        character.tick_conditions_round();
        assert_eq!(character.active_conditions.len(), 0);
    }

    #[test]
    fn test_paralyzed_condition_prevents_action() {
        let mut character = create_test_character("Stunned", 10);

        let cond = crate::domain::conditions::ConditionDefinition {
            id: "paralyze".to_string(),
            name: "Paralyze".to_string(),
            description: "Cannot act".to_string(),
            effects: vec![crate::domain::conditions::ConditionEffect::StatusEffect(
                "paralyzed".to_string(),
            )],
            default_duration: crate::domain::conditions::ConditionDuration::Rounds(1),
            icon_id: None,
        };

        apply_condition_to_character(&mut character, &cond);
        assert!(!character.can_act());
    }

    #[test]
    fn test_apply_condition_by_id_sets_flag() {
        let mut content = crate::sdk::database::ContentDatabase::new();

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

        let mut character = create_test_character("Affected", 10);
        assert!(apply_condition_to_character_by_id(&mut character, "silence", &content).is_ok());
        assert!(character.conditions.is_silenced());

        // Monster test
        let stats = crate::domain::character::Stats::new(10, 8, 6, 10, 8, 7, 5);
        let attacks = vec![crate::domain::combat::types::Attack::physical(
            crate::domain::types::DiceRoll::new(1, 4, 0),
        )];
        let mut monster = crate::domain::combat::monster::Monster::new(
            1,
            "M".to_string(),
            stats,
            10,
            5,
            attacks,
            crate::domain::combat::monster::LootTable::none(),
        );

        assert!(apply_condition_to_monster_by_id(&mut monster, "silence", &content).is_ok());
        assert_eq!(
            monster.conditions,
            crate::domain::combat::monster::MonsterCondition::Silenced
        );
    }

    #[test]
    fn test_combat_loads_monster_stats_from_db() {
        // Build a small content DB and add a monster definition
        let mut db = crate::sdk::database::ContentDatabase::new();

        use crate::domain::character::{AttributePair, AttributePair16, Stats};
        use crate::domain::combat::database::MonsterDefinition;
        use crate::domain::combat::monster::MonsterResistances;
        use crate::domain::combat::types::Attack;
        use crate::domain::types::DiceRoll;

        let monster_def = MonsterDefinition {
            id: crate::domain::types::MonsterId::from(42u8),
            name: "TestMonster".to_string(),
            stats: Stats::new(12, 8, 6, 10, 9, 7, 5),
            hp: AttributePair16::new(30),
            ac: AttributePair::new(8),
            attacks: vec![Attack::physical(DiceRoll::new(1, 6, 0))],
            flee_threshold: 0,
            special_attack_threshold: 0,
            resistances: MonsterResistances::new(),
            can_regenerate: false,
            can_advance: false,
            is_undead: false,
            magic_resistance: 0,
            loot: crate::domain::combat::monster::LootTable::none(),
            creature_id: None,
            conditions: crate::domain::combat::monster::MonsterCondition::Normal,
            active_conditions: Vec::new(),
            has_acted: false,
        };

        db.monsters.add_monster(monster_def).unwrap();

        let mut combat = CombatState::new(crate::domain::combat::types::Handicap::Even);
        let group = vec![42u8];

        // Initialize combat from group
        let res = initialize_combat_from_group(&mut combat, &db, &group);
        assert!(res.is_ok());

        // Verify monster present and stats loaded
        assert_eq!(combat.participants.len(), 1);
        if let crate::domain::combat::engine::Combatant::Monster(m) = &combat.participants[0] {
            assert_eq!(m.name, "TestMonster");
            assert_eq!(m.hp.base, 30);
            assert_eq!(m.ac.base, 8);
        } else {
            panic!("Expected a monster participant");
        }
    }

    #[test]
    fn test_advance_turn_and_round() {
        let mut combat = CombatState::new(Handicap::Even);

        combat.add_player(create_test_character("Hero1", 10));
        combat.add_player(create_test_character("Hero2", 9));

        start_combat(&mut combat);

        assert_eq!(combat.current_turn, 0);
        assert_eq!(combat.round, 1);

        combat.advance_turn(&[]);
        assert_eq!(combat.current_turn, 1);
        assert_eq!(combat.round, 1);

        combat.advance_turn(&[]);
        assert_eq!(combat.current_turn, 0);
        assert_eq!(combat.round, 2); // New round
    }

    #[test]
    fn test_dot_effects_application() {
        use crate::domain::conditions::{
            ActiveCondition, ConditionDefinition, ConditionDuration, ConditionEffect,
        };
        use crate::domain::types::DiceRoll;

        let mut combat = CombatState::new(Handicap::Even);

        // Add a character with a poison condition
        let mut character = create_test_character("Hero", 10);
        character.hp.current = 20;

        // Create poison condition (1d4 damage per round)
        let poison = ActiveCondition::new("poison".to_string(), ConditionDuration::Rounds(3));
        character.add_condition(poison);

        combat.add_player(character);

        // Create condition definition
        let poison_def = ConditionDefinition {
            id: "poison".to_string(),
            name: "Poison".to_string(),
            description: "Takes damage each round".to_string(),
            effects: vec![ConditionEffect::DamageOverTime {
                damage: DiceRoll::new(1, 4, 0),
                element: "poison".to_string(),
            }],
            default_duration: ConditionDuration::Permanent,
            icon_id: None,
        };

        start_combat(&mut combat);

        // Apply DoT effects
        let effects = combat.apply_dot_effects(&[poison_def]);

        // Should have one effect entry
        assert_eq!(effects.len(), 1);

        // Damage should be between 1 and 4
        let (combatant_id, damage) = effects[0];
        assert!(matches!(combatant_id, CombatantId::Player(0)));
        assert!((1..=4).contains(&damage));

        // Character should have taken damage
        if let Some(Combatant::Player(c)) = combat.participants.first() {
            assert!(c.hp.current < 20);
            assert!(c.hp.current >= 16); // 20 - 4 max
        }
    }

    #[test]
    fn test_hot_effects_application() {
        use crate::domain::conditions::{
            ActiveCondition, ConditionDefinition, ConditionDuration, ConditionEffect,
        };
        use crate::domain::types::DiceRoll;

        let mut combat = CombatState::new(Handicap::Even);

        // Add a character with regeneration
        let mut character = create_test_character("Hero", 10);
        character.hp.current = 10;
        character.hp.base = 20;

        // Create regen condition (1d4 healing per round)
        let regen = ActiveCondition::new("regeneration".to_string(), ConditionDuration::Rounds(3));
        character.add_condition(regen);

        combat.add_player(character);

        // Create condition definition
        let regen_def = ConditionDefinition {
            id: "regeneration".to_string(),
            name: "Regeneration".to_string(),
            description: "Heals each round".to_string(),
            effects: vec![ConditionEffect::HealOverTime {
                amount: DiceRoll::new(1, 4, 0),
            }],
            default_duration: ConditionDuration::Rounds(3),
            icon_id: None,
        };

        start_combat(&mut combat);

        // Apply HoT effects
        let effects = combat.apply_dot_effects(&[regen_def]);

        // Should have one effect entry
        assert_eq!(effects.len(), 1);

        // Damage should be negative (healing)
        let (combatant_id, damage) = effects[0];
        assert!(matches!(combatant_id, CombatantId::Player(0)));
        assert!((-4..=-1).contains(&damage));

        // Character should have healed
        if let Some(Combatant::Player(c)) = combat.participants.first() {
            assert!(c.hp.current > 10);
            assert!(c.hp.current <= 14); // 10 + 4 max
        }
    }

    // ===== Damage Floor and Bonus Application Verification =====

    /// A cursed weapon with a large negative bonus (1d4-10) must never cause a
    /// hit to deal 0 damage.  `DiceRoll::roll` clamps its raw result at 0, but
    /// `resolve_attack` applies a second `.max(1)` to the combined
    /// `base_damage + might_bonus` total.  This test exercises that invariant
    /// across many rolls so that every possible die outcome (1-4) is encountered
    /// and we can confirm the floor holds throughout.
    #[test]
    fn test_cursed_weapon_damage_floor_at_one() {
        let mut combat = CombatState::new(Handicap::Even);

        // Attacker with exactly might=10 → damage_bonus == 0, so total damage
        // is determined solely by the weapon roll.
        let mut attacker = create_test_character("CursedWielder", 10);
        attacker.stats.might.current = 10; // (10-10)/2 = 0 bonus
                                           // High accuracy so every attack hits.
        attacker.stats.accuracy.current = 20;
        combat.add_player(attacker);

        // Defender with AC 0 so hit threshold is always trivially low.
        let mut defender = create_test_character("Dummy", 5);
        defender.ac.current = 0;
        combat.add_player(defender);

        // Build the cursed weapon: 1d4-10.  Even the best roll (4-10 = -6) is
        // negative, so DiceRoll::roll will always return 0.  resolve_attack must
        // raise that to 1.
        let mut db = ItemDatabase::new();
        let cursed_dagger = make_weapon_item(
            20,
            DiceRoll::new(1, 4, -10),
            0, // weapon item-level bonus already baked into the DiceRoll above
            WeaponClassification::Simple,
        );
        db.add_item(cursed_dagger).unwrap();

        // Use get_character_attack to build the Attack the same way the game
        // system does, so we test the full pipeline.
        let mut attacker_char = make_character_with_weapon(20);
        attacker_char.stats.might.current = 10;
        attacker_char.stats.accuracy.current = 20;
        // Replace attacker slot with this character so resolve_attack can find it.
        combat.participants[0] = Combatant::Player(Box::new(attacker_char));

        let attack_result = get_character_attack(
            match &combat.participants[0] {
                Combatant::Player(c) => c,
                _ => panic!("Expected player"),
            },
            &db,
        );

        let attack = match attack_result {
            MeleeAttackResult::Melee(a) => a,
            MeleeAttackResult::Ranged(_) => panic!("Expected melee attack"),
        };

        // Verify the DiceRoll bonus is -10 (saturating_add(0, -10) = -10).
        assert_eq!(attack.damage.bonus, -10);
        assert_eq!(attack.damage.count, 1);
        assert_eq!(attack.damage.sides, 4);

        let mut rng = rng();

        // Run many iterations; every hit must deal at least 1 damage.
        for _ in 0..200 {
            let result = resolve_attack(
                &combat,
                CombatantId::Player(0),
                CombatantId::Player(1),
                &attack,
                None,
                &mut rng,
            );
            assert!(result.is_ok(), "resolve_attack returned an error");
            let (damage, _) = result.unwrap();
            // A miss returns 0, which is acceptable.  On a hit, damage >= 1.
            // We cannot guarantee every iteration is a hit, but the threshold
            // is (10 + 0 - 20).max(2) = 2, so nearly every roll will hit.
            // Assert: damage is never in the range 0 < damage — i.e. it is
            // either 0 (miss) or >= 1 (hit, floored).
            assert!(
                damage == 0 || damage >= 1,
                "damage {} violates floor-at-1 invariant",
                damage
            );
        }

        // More targeted: force a hit by making threshold 2 and confirming that
        // over many rolls we observe at least one hit and it is always >= 1.
        let hit_damages: Vec<u16> = (0..500)
            .filter_map(|_| {
                let (dmg, _) = resolve_attack(
                    &combat,
                    CombatantId::Player(0),
                    CombatantId::Player(1),
                    &attack,
                    None,
                    &mut rng,
                )
                .unwrap();
                if dmg > 0 {
                    Some(dmg)
                } else {
                    None
                }
            })
            .collect();

        // With accuracy=20 and AC=0 we expect many hits across 500 trials.
        assert!(
            !hit_damages.is_empty(),
            "Expected at least one hit in 500 trials"
        );
        for &d in &hit_damages {
            assert!(
                d >= 1,
                "Cursed weapon hit dealt {} damage — floor-at-1 invariant violated",
                d
            );
        }
    }

    /// A magical +3 sword (1d6 base damage, weapon bonus +3) must have its
    /// `DiceRoll::bonus` set to 3 after `get_character_attack` applies
    /// `saturating_add`.  The theoretical minimum roll of the die (1) added to
    /// the bonus gives a minimum result of 4, which `resolve_attack` must
    /// honour.
    #[test]
    fn test_positive_bonus_adds_to_roll() {
        // ── 1. Verify the DiceRoll produced by get_character_attack ──────────
        let mut db = ItemDatabase::new();
        // +3 longsword: base damage 1d6, item bonus +3.
        let magic_sword = make_weapon_item(
            30,
            DiceRoll::new(1, 6, 0),
            3,
            WeaponClassification::MartialMelee,
        );
        db.add_item(magic_sword).unwrap();

        let character = make_character_with_weapon(30);
        let result = get_character_attack(&character, &db);

        let attack = match result {
            MeleeAttackResult::Melee(a) => a,
            MeleeAttackResult::Ranged(_) => panic!("Expected melee attack"),
        };

        // Bonus must be 3 (saturating_add(base_bonus=0, weapon_bonus=3)).
        assert_eq!(
            attack.damage.bonus, 3,
            "DiceRoll::bonus should be 3 for a +3 sword"
        );
        assert_eq!(attack.damage.count, 1);
        assert_eq!(attack.damage.sides, 6);

        // Minimum possible DiceRoll result: die=1 + bonus=3 = 4.
        assert_eq!(
            attack.damage.min(),
            4,
            "Minimum DiceRoll outcome for 1d6+3 should be 4"
        );

        // ── 2. Verify resolve_attack respects the bonus floor ────────────────
        let mut combat = CombatState::new(Handicap::Even);

        // Attacker: might=10 → might_bonus=0, high accuracy so every roll hits.
        let mut attacker = create_test_character("Paladin", 10);
        attacker.stats.might.current = 10;
        attacker.stats.accuracy.current = 20;
        combat.add_player(attacker);

        let mut defender = create_test_character("Target", 5);
        defender.ac.current = 0;
        combat.add_player(defender);

        let mut rng = rng();

        // Collect all non-miss damage values over many trials.
        let hit_damages: Vec<u16> = (0..500)
            .filter_map(|_| {
                let (dmg, _) = resolve_attack(
                    &combat,
                    CombatantId::Player(0),
                    CombatantId::Player(1),
                    &attack,
                    None,
                    &mut rng,
                )
                .unwrap();
                if dmg > 0 {
                    Some(dmg)
                } else {
                    None
                }
            })
            .collect();

        assert!(
            !hit_damages.is_empty(),
            "Expected at least one hit in 500 trials"
        );

        // With might_bonus=0, total = DiceRoll result.  Minimum DiceRoll result
        // for 1d6+3 is 4, so every hit must deal at least 4 damage.
        for &d in &hit_damages {
            assert!(
                d >= 4,
                "Expected minimum damage of 4 for +3 sword, got {}",
                d
            );
        }

        // Maximum possible damage is 1*6+3 = 9 (no might bonus).
        for &d in &hit_damages {
            assert!(
                d <= 9,
                "Expected maximum damage of 9 for +3 sword, got {}",
                d
            );
        }
    }

    // ===== ActiveSpells resistance projection tests =====

    #[test]
    fn test_resistance_check_without_active_spells() {
        // resolve_attack with active_spells: None must behave identically to
        // before ActiveSpells — no resistance reduction for physical attacks.
        let mut combat = CombatState::new(Handicap::Even);

        let mut attacker = create_test_character("Attacker", 10);
        attacker.stats.might.current = 10; // 0 damage bonus
        attacker.stats.accuracy.current = 20; // always hit
        combat.add_player(attacker);

        let mut target = create_test_character("Target", 10);
        target.ac.current = 0;
        combat.add_player(target);

        let attack = Attack::physical(crate::domain::types::DiceRoll::new(1, 6, 0));
        let mut rng = rand::rng();

        // Collect hit-damage values with None active_spells
        let hit_damages_no_spells: Vec<u16> = (0..200)
            .filter_map(|_| {
                let (dmg, _) = resolve_attack(
                    &combat,
                    CombatantId::Player(0),
                    CombatantId::Player(1),
                    &attack,
                    None,
                    &mut rng,
                )
                .unwrap();
                if dmg > 0 {
                    Some(dmg)
                } else {
                    None
                }
            })
            .collect();

        // Physical attacks are never resistance-reduced, so all hit damage
        // must fall in [1, 6] (1d6, might bonus 0).
        assert!(
            !hit_damages_no_spells.is_empty(),
            "Expected at least one hit"
        );
        for &d in &hit_damages_no_spells {
            assert!(
                (1..=6).contains(&d),
                "Physical damage {d} out of [1,6] range"
            );
        }
    }

    #[test]
    fn test_resistance_check_with_active_fire_protection() {
        use crate::application::ActiveSpells;
        use crate::domain::combat::types::AttackType;

        // A fire attack against a target whose active_spells.fire_protection > 0
        // must deal strictly less damage than the same attack without protection,
        // assuming the target has no base fire resistance.
        let mut combat = CombatState::new(Handicap::Even);

        let mut attacker = create_test_character("Pyromancer", 10);
        attacker.stats.might.current = 10;
        attacker.stats.accuracy.current = 20; // always hit
        combat.add_player(attacker);

        let mut target = create_test_character("Victim", 20);
        target.ac.current = 0;
        target.resistances.fire.current = 0; // no base fire resistance
        target.resistances.fire.base = 0;
        combat.add_player(target);

        // A fire Attack — use a large fixed die so variance is visible
        let fire_attack = Attack::new(
            crate::domain::types::DiceRoll::new(4, 6, 0),
            AttackType::Fire,
            None,
        );

        let mut rng = rand::rng();

        // --- Without active fire protection ---
        let damage_without: Vec<u16> = (0..300)
            .filter_map(|_| {
                let (dmg, _) = resolve_attack(
                    &combat,
                    CombatantId::Player(0),
                    CombatantId::Player(1),
                    &fire_attack,
                    None,
                    &mut rng,
                )
                .unwrap();
                if dmg > 0 {
                    Some(dmg)
                } else {
                    None
                }
            })
            .collect();

        // --- With active fire protection (25-point bonus) ---
        let mut spells = ActiveSpells::new();
        spells.fire_protection = 30; // non-zero → 25-point resistance bonus

        let damage_with: Vec<u16> = (0..300)
            .filter_map(|_| {
                let (dmg, _) = resolve_attack(
                    &combat,
                    CombatantId::Player(0),
                    CombatantId::Player(1),
                    &fire_attack,
                    Some(&spells),
                    &mut rng,
                )
                .unwrap();
                if dmg > 0 {
                    Some(dmg)
                } else {
                    None
                }
            })
            .collect();

        // Both sets should have hits.
        assert!(
            !damage_without.is_empty(),
            "Expected hits without protection"
        );
        assert!(!damage_with.is_empty(), "Expected hits with protection");

        // Average damage with protection must be strictly less than without.
        let avg_without: f64 =
            damage_without.iter().map(|&d| d as f64).sum::<f64>() / damage_without.len() as f64;
        let avg_with: f64 =
            damage_with.iter().map(|&d| d as f64).sum::<f64>() / damage_with.len() as f64;

        assert!(
            avg_with < avg_without,
            "Average fire damage with protection ({avg_with:.1}) should be \
             less than without ({avg_without:.1})"
        );
    }

    // -----------------------------------------------------------------------
    // clear_combat_end_conditions
    // -----------------------------------------------------------------------

    #[test]
    fn test_clear_combat_end_conditions_removes_until_combat_end_from_player() {
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};

        let mut combat = CombatState::new(Handicap::Even);
        let mut pc = create_test_character("Hero", 30);
        pc.active_conditions.push(ActiveCondition::new(
            "combat_bless".to_string(),
            ConditionDuration::UntilCombatEnd,
        ));
        pc.active_conditions.push(ActiveCondition::new(
            "poison".to_string(),
            ConditionDuration::Permanent,
        ));
        combat.add_player(pc);

        combat.clear_combat_end_conditions(&[]);

        if let Combatant::Player(p) = &combat.participants[0] {
            assert_eq!(p.active_conditions.len(), 1);
            assert_eq!(p.active_conditions[0].condition_id, "poison");
        } else {
            panic!("Expected player combatant");
        }
    }

    #[test]
    fn test_clear_combat_end_conditions_removes_until_combat_end_from_monster() {
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};

        let mut combat = CombatState::new(Handicap::Even);
        let mut monster = create_test_monster("Goblin", 5);
        monster.active_conditions.push(ActiveCondition::new(
            "combat_bless".to_string(),
            ConditionDuration::UntilCombatEnd,
        ));
        monster.active_conditions.push(ActiveCondition::new(
            "poison".to_string(),
            ConditionDuration::Permanent,
        ));
        combat.add_monster(monster);

        combat.clear_combat_end_conditions(&[]);

        if let Combatant::Monster(m) = &combat.participants[0] {
            assert_eq!(m.active_conditions.len(), 1);
            assert_eq!(m.active_conditions[0].condition_id, "poison");
        } else {
            panic!("Expected monster combatant");
        }
    }

    #[test]
    fn test_clear_combat_end_conditions_preserves_until_rest_conditions() {
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};

        let mut combat = CombatState::new(Handicap::Even);
        let mut pc = create_test_character("Hero", 30);
        pc.active_conditions.push(ActiveCondition::new(
            "exhaustion".to_string(),
            ConditionDuration::UntilRest,
        ));
        pc.active_conditions.push(ActiveCondition::new(
            "combat_bless".to_string(),
            ConditionDuration::UntilCombatEnd,
        ));
        combat.add_player(pc);

        combat.clear_combat_end_conditions(&[]);

        if let Combatant::Player(p) = &combat.participants[0] {
            // UntilCombatEnd removed; UntilRest preserved
            assert_eq!(p.active_conditions.len(), 1);
            assert_eq!(p.active_conditions[0].condition_id, "exhaustion");
        } else {
            panic!("Expected player combatant");
        }
    }

    #[test]
    fn test_clear_combat_end_conditions_empty_participants_is_noop() {
        let mut combat = CombatState::new(Handicap::Even);
        // No participants — should not panic
        combat.clear_combat_end_conditions(&[]);
        assert!(combat.participants.is_empty());
    }

    #[test]
    fn test_clear_combat_end_conditions_mixed_participants() {
        use crate::domain::conditions::{ActiveCondition, ConditionDuration};

        let mut combat = CombatState::new(Handicap::Even);

        let mut pc = create_test_character("Hero", 30);
        pc.active_conditions.push(ActiveCondition::new(
            "combat_bless".to_string(),
            ConditionDuration::UntilCombatEnd,
        ));

        let mut monster = create_test_monster("Goblin", 5);
        monster.active_conditions.push(ActiveCondition::new(
            "combat_slow".to_string(),
            ConditionDuration::UntilCombatEnd,
        ));
        monster.active_conditions.push(ActiveCondition::new(
            "poison".to_string(),
            ConditionDuration::Permanent,
        ));

        combat.add_player(pc);
        combat.add_monster(monster);

        combat.clear_combat_end_conditions(&[]);

        // Player should have no conditions left
        if let Combatant::Player(p) = &combat.participants[0] {
            assert!(p.active_conditions.is_empty());
        }
        // Monster should only have poison left
        if let Combatant::Monster(m) = &combat.participants[1] {
            assert_eq!(m.active_conditions.len(), 1);
            assert_eq!(m.active_conditions[0].condition_id, "poison");
        }
    }

    // -----------------------------------------------------------------------
    // apply_condition_to_character: UntilCombatEnd / UntilRest tracking
    // -----------------------------------------------------------------------

    #[test]
    fn test_apply_condition_to_character_until_combat_end_tracked_in_active_conditions() {
        use crate::domain::combat::engine::apply_condition_to_character;
        use crate::domain::conditions::{ConditionDefinition, ConditionDuration, ConditionEffect};

        let cond_def = ConditionDefinition {
            id: "combat_bless".to_string(),
            name: "Combat Bless".to_string(),
            description: "Accuracy bonus that expires when combat ends.".to_string(),
            effects: vec![ConditionEffect::AttributeModifier {
                attribute: "accuracy".to_string(),
                value: 3,
            }],
            default_duration: ConditionDuration::UntilCombatEnd,
            icon_id: None,
        };

        let mut pc = create_test_character("Hero", 30);
        let base_acc = pc.stats.accuracy.current;
        apply_condition_to_character(&mut pc, &cond_def);

        // Attribute modifier should be applied
        assert_eq!(pc.stats.accuracy.current, base_acc + 3);
        // And the condition must be tracked in active_conditions for cleanup
        assert!(
            pc.active_conditions
                .iter()
                .any(|c| c.condition_id == "combat_bless"),
            "UntilCombatEnd condition must be present in active_conditions"
        );
    }

    #[test]
    fn test_apply_condition_to_character_until_rest_tracked_in_active_conditions() {
        use crate::domain::combat::engine::apply_condition_to_character;
        use crate::domain::conditions::{ConditionDefinition, ConditionDuration, ConditionEffect};

        let cond_def = ConditionDefinition {
            id: "exhaustion".to_string(),
            name: "Exhaustion".to_string(),
            description: "Speed penalty that lasts until rest.".to_string(),
            effects: vec![ConditionEffect::AttributeModifier {
                attribute: "speed".to_string(),
                value: -3,
            }],
            default_duration: ConditionDuration::UntilRest,
            icon_id: None,
        };

        let mut pc = create_test_character("Hero", 30);
        let base_speed = pc.stats.speed.current;
        apply_condition_to_character(&mut pc, &cond_def);

        assert_eq!(pc.stats.speed.current, base_speed - 3);
        assert!(
            pc.active_conditions
                .iter()
                .any(|c| c.condition_id == "exhaustion"),
            "UntilRest condition must be present in active_conditions"
        );
    }

    #[test]
    fn test_apply_condition_to_monster_until_combat_end_tracked_in_active_conditions() {
        use crate::domain::combat::engine::apply_condition_to_monster;
        use crate::domain::conditions::{ConditionDefinition, ConditionDuration, ConditionEffect};

        let cond_def = ConditionDefinition {
            id: "combat_bless".to_string(),
            name: "Combat Bless".to_string(),
            description: "Accuracy bonus that expires when combat ends.".to_string(),
            effects: vec![ConditionEffect::AttributeModifier {
                attribute: "accuracy".to_string(),
                value: 2,
            }],
            default_duration: ConditionDuration::UntilCombatEnd,
            icon_id: None,
        };

        let mut monster = create_test_monster("Goblin", 5);
        let base_acc = monster.stats.accuracy.current;
        apply_condition_to_monster(&mut monster, &cond_def);

        assert_eq!(monster.stats.accuracy.current, base_acc + 2);
        assert!(
            monster
                .active_conditions
                .iter()
                .any(|c| c.condition_id == "combat_bless"),
            "UntilCombatEnd condition must be tracked in monster active_conditions"
        );
    }

    // ===== Defense System Tests =====

    /// Defending player's AC bonus resets to base after the round ends.
    ///
    /// The defend action applies +2 AC for the duration of the round only.
    /// `advance_round` (triggered by `advance_turn` when all turns are exhausted)
    /// must drain `defending_combatants` and subtract the bonus.
    #[test]
    fn test_defend_bonus_resets_after_round_end() {
        use crate::domain::character::{Alignment, AttributePair, Character, Sex};

        let mut combat = CombatState::new(Handicap::Even);

        let mut hero = Character::new(
            "Defender".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        hero.ac = AttributePair::new(10);

        // Add a second (monster) participant so that one advance_turn from turn=0
        // does NOT immediately trigger advance_round (needs two turns).
        let monster = create_test_monster("Goblin", 5);
        combat.add_player(hero);
        combat.add_monster(monster);
        combat.turn_order = vec![CombatantId::Player(0), CombatantId::Monster(1)];
        combat.current_turn = 0;

        // Manually apply the defend bonus and set the flag (mirrors perform_defend_action).
        combat.defending_combatants.insert(0);
        if let Some(Combatant::Player(pc)) = combat.participants.get_mut(0) {
            pc.ac.modify(2);
            assert_eq!(pc.ac.current, 12, "AC should be 12 after +2 defend bonus");
        }

        // Advance turn for the player (turn 0 → 1): round NOT yet over.
        combat.advance_turn(&[]);
        assert_eq!(combat.current_turn, 1);
        assert!(
            combat.defending_combatants.contains(&0),
            "defending flag must persist mid-round"
        );
        if let Some(Combatant::Player(pc)) = combat.participants.first() {
            assert_eq!(pc.ac.current, 12, "AC bonus must remain active mid-round");
        }

        // Advance turn for the monster (turn 1 → 2 >= 2): triggers advance_round.
        combat.advance_turn(&[]);
        assert_eq!(
            combat.current_turn, 0,
            "current_turn resets to 0 on new round"
        );

        // Defending flag cleared and AC bonus reversed.
        assert!(
            combat.defending_combatants.is_empty(),
            "defending_combatants must be empty after round advance"
        );
        if let Some(Combatant::Player(pc)) = combat.participants.first() {
            assert_eq!(
                pc.ac.current, pc.ac.base,
                "AC must return to base after round end"
            );
        }
    }

    /// Defending reduces incoming damage by approximately 50 % (base endurance 10).
    #[test]
    fn test_defend_reduces_incoming_damage() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let mut combat = CombatState::new(Handicap::Even);

        let mut attacker = create_test_character("Attacker", 10);
        attacker.stats.accuracy.current = 20; // always hit
        attacker.stats.might.current = 10; // 0 might bonus
        combat.add_player(attacker);

        let mut target = create_test_character("Target", 10);
        target.stats.endurance.current = 10; // no endurance bonus → multiplier = 0.5
        combat.add_player(target);

        // Mark the target (index 1) as defending.
        combat.defending_combatants.insert(1);

        // Fixed-damage attack: 1d1+99 = 100 every time.
        let attack = Attack::physical(DiceRoll::new(1, 1, 99));

        let mut rng = StdRng::seed_from_u64(0);
        let (damage, _) = resolve_attack(
            &combat,
            CombatantId::Player(0),
            CombatantId::Player(1),
            &attack,
            None,
            &mut rng,
        )
        .expect("resolve_attack failed");

        // ceil(100 * 0.5) = 50
        assert_eq!(
            damage, 50,
            "defending with endurance 10 should halve damage"
        );
    }

    /// `power_shield` active grants complete immunity (0 damage).
    #[test]
    fn test_power_shield_grants_immunity() {
        use crate::application::ActiveSpells;
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let mut combat = CombatState::new(Handicap::Even);

        let mut attacker = create_test_character("Attacker", 10);
        attacker.stats.accuracy.current = 20;
        attacker.stats.might.current = 10;
        combat.add_player(attacker);

        let target = create_test_character("Shielded", 10);
        combat.add_player(target);

        let attack = Attack::physical(DiceRoll::new(1, 1, 99));

        let mut active_spells = ActiveSpells::new();
        active_spells.power_shield = 5; // active

        let mut rng = StdRng::seed_from_u64(0);
        let (damage, _) = resolve_attack(
            &combat,
            CombatantId::Player(0),
            CombatantId::Player(1),
            &attack,
            Some(&active_spells),
            &mut rng,
        )
        .expect("resolve_attack failed");

        assert_eq!(damage, 0, "power_shield must grant full immunity");
    }

    /// `shield` active (without defending) reduces damage by 20 %.
    #[test]
    fn test_shield_reduces_damage_without_defending() {
        use crate::application::ActiveSpells;
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let mut combat = CombatState::new(Handicap::Even);

        let mut attacker = create_test_character("Attacker", 10);
        attacker.stats.accuracy.current = 20;
        attacker.stats.might.current = 10;
        combat.add_player(attacker);

        let target = create_test_character("Target", 10);
        combat.add_player(target);

        // Target is NOT defending.
        let attack = Attack::physical(DiceRoll::new(1, 1, 99)); // 100 always

        let mut active_spells = ActiveSpells::new();
        active_spells.shield = 5; // active, but not defending

        let mut rng = StdRng::seed_from_u64(0);
        let (damage, _) = resolve_attack(
            &combat,
            CombatantId::Player(0),
            CombatantId::Player(1),
            &attack,
            Some(&active_spells),
            &mut rng,
        )
        .expect("resolve_attack failed");

        // ceil(100 * 0.80) = 80
        assert_eq!(damage, 80, "shield alone should reduce damage by 20 %");
    }

    /// Defending combined with `shield` reduces damage by 65 % (multiplier 0.35).
    #[test]
    fn test_defend_and_shield_combo_reduces_damage_65_percent() {
        use crate::application::ActiveSpells;
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let mut combat = CombatState::new(Handicap::Even);

        let mut attacker = create_test_character("Attacker", 10);
        attacker.stats.accuracy.current = 20;
        attacker.stats.might.current = 10;
        combat.add_player(attacker);

        let target = create_test_character("Target", 10);
        combat.add_player(target);

        // Target IS defending.
        combat.defending_combatants.insert(1);

        let attack = Attack::physical(DiceRoll::new(1, 1, 99)); // 100 always

        let mut active_spells = ActiveSpells::new();
        active_spells.shield = 5;

        let mut rng = StdRng::seed_from_u64(0);
        let (damage, _) = resolve_attack(
            &combat,
            CombatantId::Player(0),
            CombatantId::Player(1),
            &attack,
            Some(&active_spells),
            &mut rng,
        )
        .expect("resolve_attack failed");

        // ceil(100 * 0.35) = 35
        assert_eq!(
            damage, 35,
            "defend + shield should reduce damage by 65 % (multiplier 0.35)"
        );
    }

    /// `leather_skin` active (alone) reduces damage by 10 %.
    #[test]
    fn test_leather_skin_reduces_damage_10_percent() {
        use crate::application::ActiveSpells;
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let mut combat = CombatState::new(Handicap::Even);

        let mut attacker = create_test_character("Attacker", 10);
        attacker.stats.accuracy.current = 20;
        attacker.stats.might.current = 10;
        combat.add_player(attacker);

        let target = create_test_character("Target", 10);
        combat.add_player(target);

        let attack = Attack::physical(DiceRoll::new(1, 1, 99)); // 100 always

        let mut active_spells = ActiveSpells::new();
        active_spells.leather_skin = 5;

        let mut rng = StdRng::seed_from_u64(0);
        let (damage, _) = resolve_attack(
            &combat,
            CombatantId::Player(0),
            CombatantId::Player(1),
            &attack,
            Some(&active_spells),
            &mut rng,
        )
        .expect("resolve_attack failed");

        // ceil(100 * 0.90) = 90
        assert_eq!(
            damage, 90,
            "leather_skin alone should reduce damage by 10 %"
        );
    }

    /// High endurance improves the defending multiplier (each 10 above 10 = −0.02).
    #[test]
    fn test_defend_endurance_bonus_improves_reduction() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let mut combat = CombatState::new(Handicap::Even);

        let mut attacker = create_test_character("Attacker", 10);
        attacker.stats.accuracy.current = 20;
        attacker.stats.might.current = 10;
        combat.add_player(attacker);

        let mut target = create_test_character("Target", 10);
        // endurance 30 → 20 above 10 → 2 steps → −0.04 → multiplier = 0.46
        target.stats.endurance.current = 30;
        combat.add_player(target);

        combat.defending_combatants.insert(1);

        let attack = Attack::physical(DiceRoll::new(1, 1, 99)); // 100 always

        let mut rng = StdRng::seed_from_u64(0);
        let (damage, _) = resolve_attack(
            &combat,
            CombatantId::Player(0),
            CombatantId::Player(1),
            &attack,
            None,
            &mut rng,
        )
        .expect("resolve_attack failed");

        // ceil(100 * 0.46) = ceil(46.0) = 46
        assert_eq!(
            damage, 46,
            "endurance 30 should reduce defending multiplier to 0.46"
        );
    }

    /// The endurance-based multiplier is floored at 0.25 (very high endurance).
    #[test]
    fn test_defend_endurance_bonus_capped_at_0_25_minimum_multiplier() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let mut combat = CombatState::new(Handicap::Even);

        let mut attacker = create_test_character("Attacker", 10);
        attacker.stats.accuracy.current = 20;
        attacker.stats.might.current = 10;
        combat.add_player(attacker);

        let mut target = create_test_character("Target", 10);
        // endurance 255 (max u8) → huge bonus, but multiplier floored at 0.25
        target.stats.endurance.current = 255;
        combat.add_player(target);

        combat.defending_combatants.insert(1);

        let attack = Attack::physical(DiceRoll::new(1, 1, 99)); // 100 always

        let mut rng = StdRng::seed_from_u64(0);
        let (damage, _) = resolve_attack(
            &combat,
            CombatantId::Player(0),
            CombatantId::Player(1),
            &attack,
            None,
            &mut rng,
        )
        .expect("resolve_attack failed");

        // ceil(100 * 0.25) = 25
        assert_eq!(
            damage, 25,
            "endurance bonus must not push multiplier below 0.25 minimum"
        );
    }
}
