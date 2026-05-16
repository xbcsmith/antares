// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Application-layer service for NPC skill training.
//!
//! This module provides [`perform_skill_training_service`], the authoritative
//! entry point for all NPC skill-training interactions.  It enforces every
//! precondition atomically — no state is modified until every check passes.
//!
//! # Skill Training Flow
//!
//! 1. Player approaches an NPC with `is_skill_trainer == true`.
//! 2. UI opens [`SkillTrainingState`](crate::application::skill_training_state::SkillTrainingState)
//!    listing eligible party members and the NPC's `trainable_skill_ids`.
//! 3. Player selects a party member and a skill, then confirms.
//! 4. `perform_skill_training_service` is called.
//! 5. On success: gold is deducted and the character's persistent skill rank
//!    is incremented by 1.
//!
//! # Fee Formula
//!
//! ```text
//! fee = floor(base * multiplier * (current_persistent_rank + 1))
//! ```
//!
//! Where `base` and `multiplier` come from the NPC override (if set) or the
//! campaign-level constants [`SKILL_TRAINING_FEE_BASE_DEFAULT`] and
//! [`SKILL_TRAINING_FEE_MULTIPLIER_DEFAULT`].

use crate::application::GameState;
use crate::domain::skill_resolver::SkillResolver;
use crate::domain::skills::SkillRank;
use crate::sdk::database::ContentDatabase;
use thiserror::Error;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Default base gold fee per skill training session when the NPC has no override.
///
/// A character with persistent rank 0 training to rank 1 costs this amount
/// (before any multiplier). Multiply by `(current_rank + 1)` for higher ranks.
pub const SKILL_TRAINING_FEE_BASE_DEFAULT: u32 = 100;

/// Default per-rank fee multiplier applied during skill training when the NPC
/// has no override.
pub const SKILL_TRAINING_FEE_MULTIPLIER_DEFAULT: f32 = 1.0;

// ── Error type ────────────────────────────────────────────────────────────────

/// Errors returned by [`perform_skill_training_service`].
#[derive(Error, Debug, Clone, PartialEq)]
pub enum SkillTrainingError {
    /// The specified NPC does not exist or is not flagged as a skill trainer.
    #[error("NPC '{0}' is not a skill trainer")]
    NotASkillTrainer(String),

    /// The NPC exists and is a skill trainer, but does not offer this skill.
    #[error("NPC does not offer skill '{0}'")]
    SkillNotOffered(String),

    /// The skill definition does not exist or has `is_trainable == false`.
    #[error("Skill '{0}' is not trainable")]
    SkillNotTrainable(String),

    /// No living party member exists at the requested index.
    #[error("No party member at index {0}")]
    CharacterNotFound(usize),

    /// The character's effective skill rank is already at the training cap.
    #[error("Character's skill rank is already at maximum")]
    SkillRankAtMaximum,

    /// The party does not have enough gold to pay the training fee.
    #[error("Insufficient gold: need {need}, have {have}")]
    InsufficientGold { need: u32, have: u32 },

    /// An error occurred while resolving the character's effective skill rank.
    #[error("Skill resolution failed: {0}")]
    SkillResolutionFailed(String),
}

// ── Service function ──────────────────────────────────────────────────────────

/// Performs a skill training session at an NPC skill trainer.
///
/// This is the authoritative application-layer entry point for all NPC
/// skill-training interactions.  All preconditions are checked atomically:
/// if any check fails, **no game state is modified**.
///
/// # Steps performed
///
/// 1. Looks up the NPC by `npc_id` in `db.npcs`. Missing → `NotASkillTrainer`.
/// 2. Verifies `npc.is_skill_trainer == true`. False → `NotASkillTrainer`.
/// 3. Verifies a party member exists at `party_index`. Missing → `CharacterNotFound`.
/// 4. Verifies the member `is_alive()`. Dead → `CharacterNotFound`.
/// 5. Looks up `skill_id` in `db.skills`. Missing → `SkillNotTrainable`.
/// 6. Verifies `skill_def.is_trainable == true`. False → `SkillNotTrainable`.
/// 7. Verifies `npc.trainable_skill_ids` contains `skill_id`. Missing → `SkillNotOffered`.
/// 8. Resolves the character's current effective rank via [`SkillResolver`].
///    Error → `SkillResolutionFailed`.
/// 9. Determines the rank cap: `npc.skill_training_max_rank.unwrap_or(skill_def.max_rank)`.
/// 10. Checks `current_effective_rank < cap`. At cap → `SkillRankAtMaximum`.
/// 11. Computes fee using the NPC's override or campaign defaults.
/// 12. Checks `party.gold >= fee`. Insufficient → `InsufficientGold`.
/// 13. Resolves the would-be post-training effective rank using a cloned
///     character. Error → `SkillResolutionFailed` with no state mutation.
/// 14. **Modifies state**: deducts gold and increments the character's
///     persistent skill rank by 1, then returns `Ok((new_rank, fee_paid))`.
///
/// # Arguments
///
/// * `game_state`   — Mutable game state; gold and character skill ranks are
///   modified on success.
/// * `npc_id`       — String ID of the skill trainer NPC.
/// * `party_index`  — Zero-based index into `game_state.party.members`.
/// * `skill_id`     — String ID of the skill to train (must be in `db.skills`).
/// * `db`           — Content database providing NPC, skill, class, and race data.
///
/// # Returns
///
/// `Ok((new_effective_rank, fee_paid))` on success, where `new_effective_rank`
/// is the character's effective rank after the training session.
///
/// # Errors
///
/// Returns [`SkillTrainingError`] when any precondition fails. The error
/// variant describes exactly which precondition was not met.
///
/// # Examples
///
/// ```no_run
/// use antares::application::GameState;
/// use antares::application::skill_training::perform_skill_training_service;
/// use antares::domain::character::{Alignment, Character, Sex};
/// use antares::domain::world::npc::NpcDefinition;
/// use antares::sdk::database::ContentDatabase;
///
/// let mut state = GameState::new();
/// let mut db = ContentDatabase::new();
///
/// // Set up a skill trainer NPC
/// let mut npc = NpcDefinition::new("perception_sage", "Perception Sage", "sage.png");
/// npc.is_skill_trainer = true;
/// npc.trainable_skill_ids = vec!["perception".to_string()];
/// db.npcs.add_npc(npc).unwrap();
///
/// // Add a party member with enough gold
/// let member = Character::new(
///     "Hero".to_string(),
///     "human".to_string(),
///     "knight".to_string(),
///     Sex::Male,
///     Alignment::Good,
/// );
/// state.party.members.push(member);
/// state.party.gold = 500;
///
/// let result = perform_skill_training_service(&mut state, "perception_sage", 0, "perception", &db);
/// // On full DB setup this would succeed and return Ok((new_rank, fee))
/// ```
pub fn perform_skill_training_service(
    game_state: &mut GameState,
    npc_id: &str,
    party_index: usize,
    skill_id: &str,
    db: &ContentDatabase,
) -> Result<(SkillRank, u32), SkillTrainingError> {
    // Step 1: Look up NPC by id.
    let npc = db
        .npcs
        .get_npc(npc_id)
        .ok_or_else(|| SkillTrainingError::NotASkillTrainer(npc_id.to_string()))?;

    // Step 2: Verify NPC is a skill trainer.
    if !npc.is_skill_trainer {
        return Err(SkillTrainingError::NotASkillTrainer(npc_id.to_string()));
    }

    // Clone what we need from the NPC before the mutable borrow of game_state.
    let npc_clone = npc.clone();

    // Step 3 & 4: Verify party member exists and is alive.
    let member = game_state
        .party
        .members
        .get(party_index)
        .ok_or(SkillTrainingError::CharacterNotFound(party_index))?;

    if !member.is_alive() {
        return Err(SkillTrainingError::CharacterNotFound(party_index));
    }

    // Step 5: Look up skill definition.
    let skill_def = db
        .skills
        .get(skill_id)
        .ok_or_else(|| SkillTrainingError::SkillNotTrainable(skill_id.to_string()))?;

    // Step 6: Verify skill is trainable.
    if !skill_def.is_trainable {
        return Err(SkillTrainingError::SkillNotTrainable(skill_id.to_string()));
    }

    // Step 7: Verify NPC offers this skill.
    if !npc_clone
        .trainable_skill_ids
        .contains(&skill_id.to_string())
    {
        return Err(SkillTrainingError::SkillNotOffered(skill_id.to_string()));
    }

    // Clone skill definition data we need before continuing.
    let skill_max_rank = skill_def.max_rank;

    // Step 8: Resolve current effective rank.
    let current_effective_rank = SkillResolver::effective_skill_rank_for_character(
        member,
        &skill_id.to_string(),
        &db.skills,
        &db.classes,
        &db.races,
    )
    .map_err(|e| SkillTrainingError::SkillResolutionFailed(e.to_string()))?;

    // Step 9: Determine the rank cap.
    let rank_cap = npc_clone.skill_training_max_rank.unwrap_or(skill_max_rank);

    // Step 10: Check that training is still possible.
    if current_effective_rank >= rank_cap {
        return Err(SkillTrainingError::SkillRankAtMaximum);
    }

    // Step 11: Get the character's current *persistent* rank (used for fee calculation).
    let current_persistent_rank = game_state.party.members[party_index]
        .skill_ranks
        .get(&skill_id.to_string())
        .unwrap_or(0);

    // Compute the fee using the NPC overrides or campaign defaults.
    let fee = npc_clone.skill_training_fee(
        current_persistent_rank,
        SKILL_TRAINING_FEE_BASE_DEFAULT,
        SKILL_TRAINING_FEE_MULTIPLIER_DEFAULT,
    );

    // Step 12: Check the party has enough gold.
    let gold = game_state.party.gold;
    if gold < fee {
        return Err(SkillTrainingError::InsufficientGold {
            need: fee,
            have: gold,
        });
    }

    // Step 13: Resolve the would-be new rank using a cloned character before
    // mutating game state, preserving atomic failure semantics.
    let mut trained_member = member.clone();
    trained_member.skill_ranks.increment(&skill_id.to_string());
    let new_effective_rank = SkillResolver::effective_skill_rank_for_character(
        &trained_member,
        &skill_id.to_string(),
        &db.skills,
        &db.classes,
        &db.races,
    )
    .map_err(|e| SkillTrainingError::SkillResolutionFailed(e.to_string()))?;

    // Step 14: Modify state — all checks passed.
    game_state.party.gold -= fee;
    game_state.party.members[party_index]
        .skill_ranks
        .increment(&skill_id.to_string());

    Ok((new_effective_rank, fee))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::GameState;
    use crate::domain::character::{Alignment, Character, Sex};
    use crate::domain::skills::{SkillCategory, SkillDatabase, SkillDefinition, SkillScalingMode};
    use crate::domain::world::npc::NpcDefinition;

    // ── Test helpers ─────────────────────────────────────────────────────────

    /// Minimal RON string for a single trainable "perception" skill.
    const PERCEPTION_SKILL_RON: &str = r#"[
        (
            id: "perception",
            name: "Perception",
            category: Exploration,
            description: "Awareness of the environment.",
            scaling: Linear(base: 0, per_level: 1),
            max_rank: 50,
            is_trainable: true,
        ),
    ]"#;

    /// Builds a `ContentDatabase` with classes, races, a minimal skill
    /// database containing "perception", and a skill trainer NPC.
    fn make_skill_trainer_db() -> ContentDatabase {
        let mut db = ContentDatabase::new();

        db.classes = crate::domain::classes::ClassDatabase::load_from_file("data/classes.ron")
            .expect("data/classes.ron must exist");
        db.races = crate::domain::races::RaceDatabase::load_from_file("data/races.ron")
            .expect("data/races.ron must exist");
        db.skills = SkillDatabase::load_from_string(PERCEPTION_SKILL_RON)
            .expect("inline skill RON must parse");

        let mut npc = NpcDefinition::new("perception_sage", "Perception Sage", "sage.png");
        npc.is_skill_trainer = true;
        npc.trainable_skill_ids = vec!["perception".to_string()];
        db.npcs.add_npc(npc).unwrap();

        db
    }

    /// Builds a `ContentDatabase` identical to [`make_skill_trainer_db`] but
    /// with the NPC's `skill_training_max_rank` capped at `5`.
    fn make_skill_trainer_db_with_rank_cap(cap: u16) -> ContentDatabase {
        let mut db = make_skill_trainer_db();
        // Replace the NPC with one that has a rank cap.
        db.npcs = {
            let mut npcs = crate::sdk::database::NpcDatabase::new();
            let mut npc = NpcDefinition::new("perception_sage", "Perception Sage", "sage.png");
            npc.is_skill_trainer = true;
            npc.trainable_skill_ids = vec!["perception".to_string()];
            npc.skill_training_max_rank = Some(cap);
            npcs.add_npc(npc).unwrap();
            npcs
        };
        db
    }

    /// Creates a living party member with the given class/race.
    fn make_alive_character() -> Character {
        let mut c = Character::new(
            "Hero".to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        );
        c.hp.base = 30;
        c.hp.current = 30;
        c
    }

    // ── Rejection tests (no state mutation) ──────────────────────────────────

    #[test]
    fn test_perform_skill_training_rejects_non_trainer() {
        let mut state = GameState::new();
        let member = make_alive_character();
        state.party.members.push(member);
        state.party.gold = 500;

        // NPC exists but is NOT a skill trainer
        let mut db = ContentDatabase::new();
        let npc = NpcDefinition::new("plain_npc", "Plain NPC", "plain.png");
        db.npcs.add_npc(npc).unwrap();

        let result = perform_skill_training_service(&mut state, "plain_npc", 0, "perception", &db);

        assert!(
            matches!(result, Err(SkillTrainingError::NotASkillTrainer(_))),
            "expected NotASkillTrainer, got {result:?}"
        );
        assert_eq!(state.party.gold, 500, "gold must not change on error");
    }

    #[test]
    fn test_perform_skill_training_rejects_unoffered_skill() {
        let mut state = GameState::new();
        let member = make_alive_character();
        state.party.members.push(member);
        state.party.gold = 500;

        // Build a DB that has both "perception" (offered) and "diplomacy" (trainable
        // in the skill DB but NOT offered by the NPC). This ensures the service
        // reaches the NPC-offer check rather than failing on skill existence.
        let diplomacy_ron = r#"[
            (id: "perception", name: "Perception", category: Exploration,
             description: "Awareness.", scaling: Flat, max_rank: 50, is_trainable: true),
            (id: "diplomacy", name: "Diplomacy", category: Social,
             description: "Persuasion skill.", scaling: Flat, max_rank: 30, is_trainable: true),
        ]"#;
        let mut db = make_skill_trainer_db();
        db.skills = SkillDatabase::load_from_string(diplomacy_ron).expect("inline RON must parse");
        // NPC only offers "perception", NOT "diplomacy"

        let result =
            perform_skill_training_service(&mut state, "perception_sage", 0, "diplomacy", &db);

        assert!(
            matches!(result, Err(SkillTrainingError::SkillNotOffered(_))),
            "expected SkillNotOffered, got {result:?}"
        );
        assert_eq!(state.party.gold, 500, "gold must not change on error");
    }

    #[test]
    fn test_perform_skill_training_rejects_untrainable_skill() {
        let mut state = GameState::new();
        let member = make_alive_character();
        state.party.members.push(member);
        state.party.gold = 500;

        // Build a DB where "athletics" is NOT trainable and the NPC offers it.
        let mut db = make_skill_trainer_db();
        db.skills
            .add(SkillDefinition {
                id: "athletics".to_string(),
                name: "Athletics".to_string(),
                category: SkillCategory::Utility,
                description: String::new(),
                scaling: SkillScalingMode::Flat,
                max_rank: 20,
                is_trainable: false, // explicitly NOT trainable
            })
            .unwrap();
        // Replace NPC so it offers "athletics"
        db.npcs = {
            let mut npcs = crate::sdk::database::NpcDatabase::new();
            let mut npc = NpcDefinition::new("perception_sage", "Perception Sage", "sage.png");
            npc.is_skill_trainer = true;
            npc.trainable_skill_ids = vec!["athletics".to_string()];
            npcs.add_npc(npc).unwrap();
            npcs
        };

        let result =
            perform_skill_training_service(&mut state, "perception_sage", 0, "athletics", &db);

        assert!(
            matches!(result, Err(SkillTrainingError::SkillNotTrainable(_))),
            "expected SkillNotTrainable, got {result:?}"
        );
        assert_eq!(state.party.gold, 500, "gold must not change on error");
    }

    #[test]
    fn test_perform_skill_training_rejects_insufficient_gold() {
        let mut state = GameState::new();
        let member = make_alive_character();
        state.party.members.push(member);
        // rank 0 → 1: fee = 100 * 1.0 * 1 = 100; only 50 available
        state.party.gold = 50;

        let db = make_skill_trainer_db();

        let result =
            perform_skill_training_service(&mut state, "perception_sage", 0, "perception", &db);

        assert!(
            matches!(
                result,
                Err(SkillTrainingError::InsufficientGold {
                    need: 100,
                    have: 50
                })
            ),
            "expected InsufficientGold(need=100, have=50), got {result:?}"
        );
        assert_eq!(state.party.gold, 50, "gold must not change on error");
        assert_eq!(
            state.party.members[0]
                .skill_ranks
                .get(&"perception".to_string()),
            None,
            "skill rank must not change on error"
        );
    }

    #[test]
    fn test_perform_skill_training_rejects_max_rank() {
        let mut state = GameState::new();
        let mut member = make_alive_character();
        // Give the character a persistent rank at the NPC's cap (5).
        // With Linear(base=0, per_level=1) at level 1, auto rank=0;
        // persistent rank 5 → effective rank = 5 = cap.
        member.skill_ranks.set("perception".to_string(), 5);
        state.party.members.push(member);
        state.party.gold = 500;

        // Cap the NPC at rank 5 so this character is already at max.
        let db = make_skill_trainer_db_with_rank_cap(5);

        let result =
            perform_skill_training_service(&mut state, "perception_sage", 0, "perception", &db);

        assert!(
            matches!(result, Err(SkillTrainingError::SkillRankAtMaximum)),
            "expected SkillRankAtMaximum, got {result:?}"
        );
        assert_eq!(state.party.gold, 500, "gold must not change on error");
    }

    #[test]
    fn test_skill_training_service_is_atomic_when_final_resolution_fails() {
        let mut state = GameState::new();
        let member = make_alive_character();
        state.party.members.push(member);
        state.party.gold = 500;

        let mut db = make_skill_trainer_db();
        db.classes = crate::domain::classes::ClassDatabase::default();

        let result =
            perform_skill_training_service(&mut state, "perception_sage", 0, "perception", &db);

        assert!(
            matches!(result, Err(SkillTrainingError::SkillResolutionFailed(_))),
            "expected SkillResolutionFailed, got {result:?}"
        );
        assert_eq!(
            state.party.gold, 500,
            "gold must not change on resolver error"
        );
        assert_eq!(
            state.party.members[0]
                .skill_ranks
                .get(&"perception".to_string()),
            None,
            "persistent rank must not change on resolver error"
        );
    }

    // ── Success tests ─────────────────────────────────────────────────────────

    #[test]
    fn test_perform_skill_training_increments_character_skill_rank() {
        let mut state = GameState::new();
        let member = make_alive_character();
        state.party.members.push(member);
        state.party.gold = 500;

        let db = make_skill_trainer_db();

        let result =
            perform_skill_training_service(&mut state, "perception_sage", 0, "perception", &db);

        assert!(result.is_ok(), "training should succeed: {result:?}");
        let persistent_rank = state.party.members[0]
            .skill_ranks
            .get(&"perception".to_string())
            .unwrap_or(0);
        assert_eq!(
            persistent_rank, 1,
            "persistent rank must be 1 after training"
        );
    }

    #[test]
    fn test_perform_skill_training_deducts_gold() {
        let mut state = GameState::new();
        let member = make_alive_character();
        state.party.members.push(member);
        state.party.gold = 500;

        let db = make_skill_trainer_db();

        // rank 0 → 1: fee = 100 * 1.0 * 1 = 100
        let result =
            perform_skill_training_service(&mut state, "perception_sage", 0, "perception", &db);

        assert!(result.is_ok(), "training should succeed: {result:?}");
        let (_, fee) = result.unwrap();
        assert_eq!(fee, 100, "fee should be 100 for rank 0→1");
        assert_eq!(state.party.gold, 400, "gold should be 500 - 100 = 400");
    }
}
