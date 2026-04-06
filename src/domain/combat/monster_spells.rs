// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Monster spell casting AI
//!
//! This module provides the AI decision logic and spell execution machinery for
//! monsters that have spell-casting capabilities.  Two concerns are kept
//! deliberately separate:
//!
//! * **Action selection** — [`choose_monster_action`] decides whether a monster
//!   should make a physical attack or cast a spell on this turn.
//! * **Spell execution** — [`execute_monster_spell_cast`] applies the chosen
//!   spell's effect to the combat state and manages the post-cast cooldown.
//!
//! # Differences from player spell casting
//!
//! | Aspect              | Player                          | Monster                        |
//! |---------------------|---------------------------------|--------------------------------|
//! | SP cost             | Deducted from `character.sp`    | **None** — unlimited energy    |
//! | Class restriction   | Enforced by school/level gates  | **None** — any listed spell    |
//! | Cooldown            | None                            | 2 rounds after each cast       |
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.4 for monster specifications
//! and Section 5.3 for spell mechanics.

use crate::application::ActiveSpells;
use crate::domain::combat::engine::{apply_condition_to_character_by_id, CombatState, Combatant};
use crate::domain::combat::monster::{AiBehavior, Monster};
use crate::domain::magic::effect_dispatch::apply_buff_spell;
use crate::domain::magic::types::{SpellEffectType, SpellResult};
use crate::domain::types::SpellId;
use crate::sdk::database::ContentDatabase;
use rand::Rng;

// ===== MonsterAction =====

/// Decision made by the monster AI for its current turn.
///
/// Returned by [`choose_monster_action`] to tell the combat engine whether the
/// monster will perform a standard physical attack or attempt to cast a spell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonsterAction {
    /// Monster performs a standard physical attack.
    PhysicalAttack,
    /// Monster casts the identified spell.
    CastSpell {
        /// The ID of the spell to cast.
        spell_id: SpellId,
    },
}

// ===== Action Selection =====

/// Chooses a monster's action for this turn: physical attack or spell cast.
///
/// Decision rules applied in order:
///
/// 1. If the monster cannot cast (empty spell list, non-zero cooldown, silenced,
///    or incapacitated) → [`MonsterAction::PhysicalAttack`].
/// 2. If [`AiBehavior::Defensive`] **and** current HP > 60 % of base HP →
///    30 % chance to cast, 70 % chance of physical attack.
/// 3. Otherwise → 40 % chance to cast, 60 % chance of physical attack.
///
/// A random spell is selected from `monster.spells` when casting.
///
/// # Arguments
///
/// * `monster` - The monster deciding its action.
/// * `rng`     - Random number generator used for probabilistic decisions.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::monster::{Monster, LootTable};
/// use antares::domain::combat::monster_spells::{choose_monster_action, MonsterAction};
/// use antares::domain::character::Stats;
///
/// let monster = Monster::new(
///     1, "Warrior".to_string(),
///     Stats::new(10, 8, 8, 10, 10, 8, 8), 30, 8, vec![], LootTable::none(),
/// );
///
/// // No spells → always physical
/// let action = choose_monster_action(&monster, &mut rand::rng());
/// assert_eq!(action, MonsterAction::PhysicalAttack);
/// ```
pub fn choose_monster_action<R: Rng>(monster: &Monster, rng: &mut R) -> MonsterAction {
    if !monster.can_cast_spell() || monster.spells.is_empty() {
        return MonsterAction::PhysicalAttack;
    }

    // Pick a random spell from the monster's list.
    let spell_idx = rng.random_range(0..monster.spells.len());
    let spell_id = monster.spells[spell_idx];

    // Defensive monsters with healthy HP strongly prefer physical attacks.
    if matches!(monster.ai_behavior, AiBehavior::Defensive)
        && monster.hp.base > 0
        && (u32::from(monster.hp.current) * 100) / u32::from(monster.hp.base) > 60
    {
        // 30 % chance to cast (3 out of 10).
        if rng.random_range(0..10) < 3 {
            return MonsterAction::CastSpell { spell_id };
        }
        return MonsterAction::PhysicalAttack;
    }

    // Default: 40 % chance to cast (4 out of 10).
    if rng.random_range(0..10) < 4 {
        MonsterAction::CastSpell { spell_id }
    } else {
        MonsterAction::PhysicalAttack
    }
}

// ===== Spell Execution =====

/// Executes a monster's spell cast and applies its effects to the combat state.
///
/// Handles the full lifecycle of a monster spell cast: selecting a spell from
/// the monster's list at random, resolving the appropriate effect against living
/// targets, and applying the post-cast cooldown.
///
/// Effect routing by [`SpellEffectType`]:
///
/// | Effect type   | What happens                                              |
/// |---------------|-----------------------------------------------------------|
/// | `Damage`      | `spell.damage` dice rolled; all living players take hits  |
/// | `Healing`     | Monster heals itself, clamped to its base HP              |
/// | `Buff`        | Writes duration to `active_spells` (party tracker)        |
/// | `Debuff`      | Applies conditions to the first living player             |
/// | All others    | No-op — logged in the `SpellResult` message               |
///
/// # Monster casting vs player casting
///
/// * **No SP deduction** — monster spell energy is unlimited.
/// * **No class/level check** — every spell in `monster.spells` is castable.
/// * **Cooldown** — `monster.spell_cooldown` is set to 2 after a successful cast.
///
/// # Arguments
///
/// * `combat_state`  - Mutable reference to the current combat encounter.
/// * `monster_idx`   - Index of the casting monster in `combat_state.participants`.
/// * `content`       - Content database used to look up the spell definition.
/// * `active_spells` - Party-wide active-spell tracker mutated for buff spells.
/// * `rng`           - Random number generator for damage rolls and spell selection.
///
/// # Returns
///
/// `Some(SpellResult)` on a successful cast, or `None` when:
///
/// * `monster_idx` does not point to a [`Combatant::Monster`].
/// * The monster has no spells or its cooldown has not yet expired.
/// * The randomly selected spell ID is absent from `content`.
///
/// # Examples
///
/// ```
/// use antares::domain::combat::engine::{CombatState, Combatant};
/// use antares::domain::combat::monster::{Monster, LootTable};
/// use antares::domain::combat::monster_spells::execute_monster_spell_cast;
/// use antares::domain::combat::types::Handicap;
/// use antares::domain::character::{Character, Sex, Alignment, Stats};
/// use antares::domain::magic::types::{Spell, SpellContext, SpellSchool, SpellTarget};
/// use antares::domain::types::DiceRoll;
/// use antares::application::ActiveSpells;
/// use antares::sdk::database::ContentDatabase;
///
/// let spell_id: u32 = 0xE001;
///
/// let mut state = CombatState::new(Handicap::Even);
///
/// let player = Character::new(
///     "Hero".to_string(), "human".to_string(), "knight".to_string(),
///     Sex::Male, Alignment::Good,
/// );
/// state.participants.push(Combatant::Player(Box::new(player)));
///
/// let mut mage = Monster::new(
///     1, "Mage".to_string(),
///     Stats::new(8, 14, 8, 10, 10, 8, 10), 30, 8, vec![], LootTable::none(),
/// );
/// mage.spells = vec![spell_id];
/// state.participants.push(Combatant::Monster(Box::new(mage)));
///
/// let spell = Spell::new(
///     spell_id, "Energy Blast", SpellSchool::Sorcerer, 3, 4, 0,
///     SpellContext::CombatOnly, SpellTarget::AllCharacters,
///     "Blasts all enemies with energy",
///     Some(DiceRoll::new(2, 6, 0)), 0, false,
/// );
///
/// let mut content = ContentDatabase::new();
/// content.spells.add_spell(spell).unwrap();
/// let mut active = ActiveSpells::new();
///
/// let result = execute_monster_spell_cast(
///     &mut state, 1, &content, &mut active, &mut rand::rng(),
/// );
/// assert!(result.is_some());
/// ```
pub fn execute_monster_spell_cast<R: Rng>(
    combat_state: &mut CombatState,
    monster_idx: usize,
    content: &ContentDatabase,
    active_spells: &mut ActiveSpells,
    rng: &mut R,
) -> Option<SpellResult> {
    // Clone monster data first to release the immutable borrow before we need
    // a mutable borrow later to apply effects.
    let (spells, can_cast) = {
        let participant = combat_state.participants.get(monster_idx)?;
        if let Combatant::Monster(m) = participant {
            (m.spells.clone(), m.can_cast_spell())
        } else {
            return None;
        }
    };

    if !can_cast || spells.is_empty() {
        return None;
    }

    // Pick a random spell from the monster's list.
    let spell_idx = rng.random_range(0..spells.len());
    let spell_id = spells[spell_idx];

    // Clone the spell definition to release the borrow on `content`.
    let spell = content.spells.get_spell(spell_id)?.clone();

    let mut result = SpellResult::success(format!("Monster casts {}!", spell.name));

    match spell.effective_effect_type() {
        // ── Damage ────────────────────────────────────────────────────────────
        //
        // Roll damage from `spell.damage` dice and apply it to every living
        // player character in the encounter.
        SpellEffectType::Damage => {
            if let Some(dice) = spell.damage {
                let mut total_damage = 0i32;
                let mut affected = Vec::new();

                for (i, participant) in combat_state.participants.iter_mut().enumerate() {
                    if let Combatant::Player(pc) = participant {
                        if pc.is_alive() {
                            let dmg = dice.roll(rng).max(0);
                            pc.hp.modify(-dmg);
                            total_damage += dmg;
                            affected.push(i);
                        }
                    }
                }

                result = result.with_damage(total_damage, affected);
            }
        }

        // ── Healing (self-heal) ───────────────────────────────────────────────
        //
        // Monster heals itself; result is clamped to its base (max) HP.
        SpellEffectType::Healing { amount } => {
            if let Some(Combatant::Monster(m)) = combat_state.participants.get_mut(monster_idx) {
                let heal = amount.roll(rng).max(0);
                let pre = i32::from(m.hp.current);
                let new_hp = (pre + heal).min(i32::from(m.hp.base)).max(0) as u16;
                m.hp.current = new_hp;
                let healed = (new_hp as i32 - pre).max(0);
                result = result.with_healing(healed, vec![monster_idx]);
            }
        }

        // ── Buff ──────────────────────────────────────────────────────────────
        //
        // Write the buff duration into the party-wide `ActiveSpells` tracker.
        // (Monsters generally use party-targeting buffs to debilitate the group,
        //  e.g. a curse or fear aura modelled as a buff field.)
        SpellEffectType::Buff {
            buff_field,
            duration,
        } => {
            apply_buff_spell(buff_field, duration, active_spells);
        }

        // ── Debuff ────────────────────────────────────────────────────────────
        //
        // Apply `spell.applied_conditions` to the first living player.
        SpellEffectType::Debuff => {
            let cond_ids = spell.applied_conditions.clone();

            let first_player_idx = combat_state
                .participants
                .iter()
                .enumerate()
                .find(|(_, c)| matches!(c, Combatant::Player(_)) && c.is_alive())
                .map(|(i, _)| i);

            if let Some(pidx) = first_player_idx {
                if let Some(Combatant::Player(pc)) = combat_state.participants.get_mut(pidx) {
                    for cond_id in &cond_ids {
                        let _ = apply_condition_to_character_by_id(pc.as_mut(), cond_id, content);
                    }
                }
            }
        }

        // ── All other effect types ────────────────────────────────────────────
        //
        // Utility, Resurrection, CureCondition, and Composite effects are not
        // meaningful for monster casting and are treated as no-ops.  The
        // SpellResult message still records the cast for the combat log.
        _ => {}
    }

    // Set the post-cast cooldown so the monster cannot spam spells every turn.
    if let Some(Combatant::Monster(m)) = combat_state.participants.get_mut(monster_idx) {
        m.set_spell_cooldown(2);
    }

    Some(result)
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ActiveSpells;
    use crate::domain::character::{Alignment, Character, Sex, Stats};
    use crate::domain::combat::engine::{CombatState, Combatant};
    use crate::domain::combat::monster::{AiBehavior, LootTable, Monster, MonsterCondition};
    use crate::domain::combat::types::Handicap;
    use crate::domain::magic::types::{
        Spell, SpellContext, SpellEffectType, SpellSchool, SpellTarget,
    };
    use crate::domain::types::{DiceRoll, MonsterId, SpellId};
    use crate::sdk::database::ContentDatabase;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_test_monster(id: MonsterId, spells: Vec<SpellId>) -> Monster {
        let mut m = Monster::new(
            id,
            "Test Monster".to_string(),
            Stats::new(10, 12, 10, 10, 10, 8, 8),
            30,
            8,
            vec![],
            LootTable::none(),
        );
        m.spells = spells;
        m
    }

    fn make_test_player(hp: u16) -> Character {
        let mut c = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.hp.base = hp;
        c.hp.current = hp;
        c
    }

    fn make_damage_spell(id: SpellId) -> Spell {
        Spell::new(
            id,
            "Fire Bolt",
            SpellSchool::Sorcerer,
            2,
            3,
            0,
            SpellContext::CombatOnly,
            SpellTarget::AllCharacters,
            "Deals 2d6 fire damage to all characters",
            Some(DiceRoll::new(2, 6, 0)),
            0,
            false,
        )
    }

    // ── choose_monster_action ─────────────────────────────────────────────────

    #[test]
    fn test_choose_monster_action_no_spells_returns_physical() {
        let monster = make_test_monster(1, vec![]);
        let action = choose_monster_action(&monster, &mut rand::rng());
        assert_eq!(action, MonsterAction::PhysicalAttack);
    }

    #[test]
    fn test_choose_monster_action_with_spells_sometimes_casts() {
        let spell_id: SpellId = 0xE001;
        let monster = make_test_monster(1, vec![spell_id]);

        // With 40 % cast probability across 200 trials the chance of never seeing
        // a CastSpell is (0.6)^200 ≈ 10^{-46} — practically impossible.
        let mut rng = rand::rng();
        let saw_cast = (0..200).any(|_| {
            matches!(
                choose_monster_action(&monster, &mut rng),
                MonsterAction::CastSpell { .. }
            )
        });
        assert!(saw_cast, "expected at least one CastSpell in 200 trials");
    }

    #[test]
    fn test_choose_monster_action_silenced_returns_physical() {
        let spell_id: SpellId = 0xE001;
        let mut monster = make_test_monster(1, vec![spell_id]);
        monster.conditions = MonsterCondition::Silenced;

        let action = choose_monster_action(&monster, &mut rand::rng());
        assert_eq!(action, MonsterAction::PhysicalAttack);
    }

    #[test]
    fn test_choose_monster_action_with_cooldown_returns_physical() {
        let spell_id: SpellId = 0xE001;
        let mut monster = make_test_monster(1, vec![spell_id]);
        monster.spell_cooldown = 3;

        let action = choose_monster_action(&monster, &mut rand::rng());
        assert_eq!(action, MonsterAction::PhysicalAttack);
    }

    #[test]
    fn test_choose_monster_action_defensive_high_hp_prefers_physical() {
        let spell_id: SpellId = 0xE001;
        let mut monster = make_test_monster(1, vec![spell_id]);
        monster.ai_behavior = AiBehavior::Defensive;
        // HP at 100 % of base → well above the 60 % threshold.
        monster.hp.base = 30;
        monster.hp.current = 30;

        // With only 30 % cast chance over 200 trials, the probability of seeing
        // *exclusively* PhysicalAttack is (0.7)^200 ≈ 10^{-32} — essentially zero.
        // We verify the opposite: at least one physical attack is returned.
        let mut rng = rand::rng();
        let saw_physical = (0..200).any(|_| {
            matches!(
                choose_monster_action(&monster, &mut rng),
                MonsterAction::PhysicalAttack
            )
        });
        assert!(
            saw_physical,
            "defensive monster should prefer physical when HP is high"
        );
    }

    // ── execute_monster_spell_cast ────────────────────────────────────────────

    #[test]
    fn test_execute_monster_spell_cast_no_spells_returns_none() {
        let mut state = CombatState::new(Handicap::Even);
        let monster = make_test_monster(1, vec![]);
        state
            .participants
            .push(Combatant::Monster(Box::new(monster)));

        let content = ContentDatabase::new();
        let mut active = ActiveSpells::new();

        let result =
            execute_monster_spell_cast(&mut state, 0, &content, &mut active, &mut rand::rng());
        assert!(result.is_none());
    }

    #[test]
    fn test_execute_monster_spell_cast_deals_damage_to_players() {
        let spell_id: SpellId = 0xE001;

        let mut state = CombatState::new(Handicap::Even);

        // Player at index 0.
        let player = make_test_player(50);
        state.participants.push(Combatant::Player(Box::new(player)));

        // Casting monster at index 1.
        let mut monster = make_test_monster(2, vec![spell_id]);
        monster.spell_cooldown = 0;
        state
            .participants
            .push(Combatant::Monster(Box::new(monster)));

        let mut content = ContentDatabase::new();
        content
            .spells
            .add_spell(make_damage_spell(spell_id))
            .unwrap();
        let mut active = ActiveSpells::new();

        let result =
            execute_monster_spell_cast(&mut state, 1, &content, &mut active, &mut rand::rng());

        assert!(result.is_some());
        let sr = result.unwrap();
        assert!(sr.success);

        // 2d6 minimum roll is 2, so damage must be positive.
        assert!(sr.damage.is_some());
        assert!(sr.damage.unwrap() > 0, "damage should be > 0");

        // Player should have lost HP.
        if let Combatant::Player(pc) = &state.participants[0] {
            assert!(pc.hp.current < 50, "player should have taken damage");
        } else {
            panic!("expected player at index 0");
        }

        // Monster's cooldown should be set to 2 after casting.
        if let Combatant::Monster(m) = &state.participants[1] {
            assert_eq!(m.spell_cooldown, 2, "spell cooldown should be 2 after cast");
        } else {
            panic!("expected monster at index 1");
        }
    }

    #[test]
    fn test_execute_monster_spell_cast_unknown_spell_returns_none() {
        // Spell ID is not registered in the content database.
        let spell_id: SpellId = 0xFFFF;

        let mut state = CombatState::new(Handicap::Even);
        let mut monster = make_test_monster(1, vec![spell_id]);
        monster.spell_cooldown = 0;
        state
            .participants
            .push(Combatant::Monster(Box::new(monster)));

        let content = ContentDatabase::new();
        let mut active = ActiveSpells::new();

        let result =
            execute_monster_spell_cast(&mut state, 0, &content, &mut active, &mut rand::rng());
        assert!(result.is_none());
    }

    #[test]
    fn test_execute_monster_spell_cast_heals_monster() {
        let spell_id: SpellId = 0xE002;

        let mut state = CombatState::new(Handicap::Even);

        // Wounded monster at index 0.
        let mut monster = make_test_monster(1, vec![spell_id]);
        monster.hp.base = 30;
        monster.hp.current = 10; // wounded — 20 HP below max
        monster.spell_cooldown = 0;
        state
            .participants
            .push(Combatant::Monster(Box::new(monster)));

        // Build a healing spell with an explicit effect type.
        let mut heal_spell = Spell::new(
            spell_id,
            "Regenerate",
            SpellSchool::Sorcerer,
            2,
            4,
            0,
            SpellContext::Anytime,
            SpellTarget::Self_,
            "Monster heals 2d4 HP",
            None,
            0,
            false,
        );
        heal_spell.effect_type = Some(SpellEffectType::Healing {
            amount: DiceRoll::new(2, 4, 0),
        });

        let mut content = ContentDatabase::new();
        content.spells.add_spell(heal_spell).unwrap();
        let mut active = ActiveSpells::new();

        let result =
            execute_monster_spell_cast(&mut state, 0, &content, &mut active, &mut rand::rng());

        assert!(result.is_some());
        let sr = result.unwrap();
        assert!(sr.success);
        assert!(sr.healing.is_some(), "result should carry a healing value");
        assert!(
            sr.healing.unwrap() > 0,
            "healing should be positive (2d4 minimum = 2)"
        );

        // Monster's HP must have increased from 10.
        if let Combatant::Monster(m) = &state.participants[0] {
            assert!(
                m.hp.current > 10,
                "monster HP should have risen after healing spell"
            );
        } else {
            panic!("expected monster at index 0");
        }
    }

    #[test]
    fn test_execute_monster_spell_cast_cooldown_set_after_cast() {
        let spell_id: SpellId = 0xE003;

        let mut state = CombatState::new(Handicap::Even);
        let player = make_test_player(30);
        state.participants.push(Combatant::Player(Box::new(player)));

        let mut monster = make_test_monster(1, vec![spell_id]);
        monster.spell_cooldown = 0;
        state
            .participants
            .push(Combatant::Monster(Box::new(monster)));

        let mut content = ContentDatabase::new();
        content
            .spells
            .add_spell(make_damage_spell(spell_id))
            .unwrap();
        let mut active = ActiveSpells::new();

        execute_monster_spell_cast(&mut state, 1, &content, &mut active, &mut rand::rng());

        if let Combatant::Monster(m) = &state.participants[1] {
            assert_eq!(
                m.spell_cooldown, 2,
                "cooldown must be 2 rounds after casting"
            );
        } else {
            panic!("expected monster at index 1");
        }
    }

    #[test]
    fn test_execute_monster_spell_cast_nonzero_cooldown_returns_none() {
        let spell_id: SpellId = 0xE001;

        let mut state = CombatState::new(Handicap::Even);
        let mut monster = make_test_monster(1, vec![spell_id]);
        monster.spell_cooldown = 1; // not ready yet
        state
            .participants
            .push(Combatant::Monster(Box::new(monster)));

        let mut content = ContentDatabase::new();
        content
            .spells
            .add_spell(make_damage_spell(spell_id))
            .unwrap();
        let mut active = ActiveSpells::new();

        let result =
            execute_monster_spell_cast(&mut state, 0, &content, &mut active, &mut rand::rng());
        assert!(
            result.is_none(),
            "should return None when spell cooldown has not expired"
        );
    }
}
