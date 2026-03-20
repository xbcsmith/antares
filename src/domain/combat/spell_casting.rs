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

use crate::domain::combat::engine::{
    apply_condition_to_character_by_id, apply_condition_to_monster_by_id, CombatState,
};
use crate::domain::combat::types::CombatantId;
use crate::domain::magic::casting as magic_casting;
use crate::domain::magic::types::{Spell, SpellError, SpellResult, SpellTarget};
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
    let (caster_intellect, mut result) = {
        let caster_ref_mut = combat_state
            .get_combatant_mut(&caster)
            .ok_or(SpellCastError::InvalidContext)?;
        if let Combatant::Player(pc) = caster_ref_mut {
            let intellect_now = pc.stats.intellect.current;
            // Consume SP / gems
            let res = magic_casting::cast_spell(pc, spell);
            (intellect_now, res)
        } else {
            return Err(SpellCastError::InvalidContext);
        }
    };

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
                        let _ = mon.take_damage(dmg);

                        // We already have the target index (idx) for single-target spells
                        affected.push(idx);
                        total_damage += dmg as i32;

                        // Apply conditions (best-effort)
                        for def in &cond_defs {
                            let _ = apply_condition_to_monster_by_id(mon, &def.id, content);
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

                        let _ = mon.take_damage(dmg);
                        affected.push(i);
                        total_damage += dmg as i32;

                        for cond in &spell.applied_conditions {
                            let _ = apply_condition_to_monster_by_id(mon.as_mut(), cond, content);
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
                                let _ =
                                    apply_condition_to_character_by_id(pc.as_mut(), cond, content);
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
                            let _ = apply_condition_to_character_by_id(pc.as_mut(), cond, content);
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
                        let _ = apply_condition_to_character_by_id(pc.as_mut(), cond, content);
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
    content: &ContentDatabase,
    rng: &mut R,
) -> Result<SpellResult, SpellCastError> {
    let spell = content
        .spells
        .get_spell(spell_id)
        .ok_or(SpellCastError::SpellNotFound(spell_id))?;
    execute_spell_cast_with_spell(combat_state, caster, spell, target, content, rng)
}

#[cfg(test)]
mod tests {
    use super::*;
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
        c.sp.current = sp;
        c.gems = 10;
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
        c.sp.current = sp;
        c.gems = 5;
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
}
