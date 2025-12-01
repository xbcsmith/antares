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

use crate::domain::character::Character;
use crate::domain::combat::monster::Monster;
use crate::domain::combat::types::{
    Attack, AttackType, CombatStatus, CombatantId, Handicap, SpecialEffect,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
}

// ===== Combatant =====

/// A combatant in battle (either player or monster)
///
/// # Examples
///
/// ```
/// use antares::domain::combat::engine::Combatant;
/// use antares::domain::character::{Character, Race, Class, Sex, Alignment};
///
/// let character = Character::new(
///     "Hero".to_string(),
///     Race::Human,
///     Class::Knight,
///     Sex::Male,
///     Alignment::Good,
/// );
/// let combatant = Combatant::Player(Box::new(character));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

        let mut effects = Vec::new();

        // Tick conditions and reset flags for all participants
        for participant in &mut self.participants {
            match participant {
                Combatant::Player(character) => {
                    // Tick round-based conditions
                    character.tick_conditions_round();
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
                }
            }
        }

        // Apply DoT/HoT effects
        effects.extend(self.apply_dot_effects(condition_defs));

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
}

// ===== Combat Logic Functions =====

/// Starts combat and initializes turn order
///
/// # Examples
///
/// ```
/// use antares::domain::combat::engine::{CombatState, start_combat};
/// use antares::domain::combat::types::Handicap;
/// use antares::domain::character::{Character, Race, Class, Sex, Alignment};
/// use antares::domain::combat::monster::{Monster, LootTable};
/// use antares::domain::character::Stats;
/// use antares::domain::types::DiceRoll;
/// use antares::domain::combat::types::Attack;
///
/// let mut combat = CombatState::new(Handicap::Even);
/// let character = Character::new("Hero".to_string(), Race::Human, Class::Knight, Sex::Male, Alignment::Good);
/// combat.add_player(character);
///
/// let stats = Stats::new(10, 8, 6, 10, 8, 7, 5);
/// let attacks = vec![Attack::physical(DiceRoll::new(1, 6, 0))];
/// let monster = Monster::new(1, "Goblin".to_string(), stats, 10, 5, attacks, LootTable::none(), 25);
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
/// use antares::domain::character::{Character, Race, Class, Sex, Alignment};
///
/// let mut combat = CombatState::new(Handicap::PartyAdvantage);
/// let character = Character::new("Hero".to_string(), Race::Human, Class::Knight, Sex::Male, Alignment::Good);
/// combat.add_player(character);
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

/// Resolves an attack from attacker to target
///
/// Returns the damage dealt and whether a special effect was applied.
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
/// use antares::domain::character::{Character, Race, Class, Sex, Alignment};
/// use antares::domain::types::DiceRoll;
/// use rand::rng;
///
/// let mut combat = CombatState::new(Handicap::Even);
/// let mut character = Character::new("Hero".to_string(), Race::Human, Class::Knight, Sex::Male, Alignment::Good);
/// character.stats.might.current = 15;
/// combat.add_player(character);
///
/// let attack = Attack::physical(DiceRoll::new(1, 8, 2));
/// let attacker = CombatantId::Player(0);
/// let target = CombatantId::Player(0);
/// let mut rng = rng();
///
/// // Note: In real combat, target would be a different combatant
/// let result = resolve_attack(&combat, attacker, target, &attack, &mut rng);
/// ```
pub fn resolve_attack<R: Rng>(
    combat: &CombatState,
    attacker_id: CombatantId,
    target_id: CombatantId,
    attack: &Attack,
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

    let total_damage = (base_damage + damage_bonus).max(1) as u16;

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
/// use antares::domain::character::{Character, Race, Class, Sex, Alignment};
///
/// let mut combat = CombatState::new(Handicap::Even);
/// let character = Character::new("Hero".to_string(), Race::Human, Class::Knight, Sex::Male, Alignment::Good);
/// combat.add_player(character);
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
    let target = combat
        .get_combatant_mut(&target_id)
        .ok_or(CombatError::CombatantNotFound(target_id))?;

    let died = match target {
        Combatant::Player(character) => {
            let old_hp = character.hp.current;
            character.hp.modify(-(damage as i32));
            character.hp.current == 0 && old_hp > 0
        }
        Combatant::Monster(monster) => monster.take_damage(damage),
    };

    Ok(died)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, Class, Race, Sex, Stats};
    use crate::domain::combat::monster::LootTable;
    use crate::domain::types::DiceRoll;
    use rand::rng;

    fn create_test_character(name: &str, speed: u8) -> Character {
        let mut character = Character::new(
            name.to_string(),
            Race::Human,
            Class::Knight,
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
            25,
        )
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
}
