// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Skill check API — deterministic skill rank comparisons used by game mechanics.
//!
//! Skill checks compare a character's effective skill rank against a numeric
//! difficulty threshold. All Phase 3 integrations use deterministic checks
//! (`evaluate_skill_check_without_roll`) unless a specific mechanic requires
//! a randomized outcome.
//!
//! # Module Layout
//!
//! | Item | Kind | Purpose |
//! |------|------|---------|
//! | [`SkillCheckDifficulty`] | type alias | Numeric difficulty threshold |
//! | [`SkillCheckRequest`] | struct | Skill ID, difficulty, and optional modifiers |
//! | [`SkillCheckResult`] | struct | Success flag, rank, optional roll, margin |
//! | [`SkillCheckError`] | enum | All error conditions for skill checks |
//! | [`evaluate_skill_check_without_roll`] | fn | Pure deterministic rank-vs-difficulty |
//! | [`evaluate_party_skill_scope`] | fn | Aggregate party ranks then evaluate |
//! | [`skill_check_for_character`] | fn | Resolve rank via resolver then evaluate |
//!
//! # Examples
//!
//! ```
//! use antares::domain::skill_checks::{
//!     SkillCheckDifficulty, SkillCheckResult, evaluate_skill_check_without_roll,
//! };
//! use antares::domain::skills::SkillRank;
//!
//! let rank: SkillRank = 7;
//! let difficulty: SkillCheckDifficulty = 5;
//! let result = evaluate_skill_check_without_roll(rank, difficulty);
//! assert!(result.success);
//! assert_eq!(result.margin, 2);
//! ```

use crate::domain::skill_resolver::{SkillResolver, SkillResolverContext};
use crate::domain::skills::{PartySkillScope, SkillDatabase, SkillError, SkillId, SkillRank};
use thiserror::Error;

// ===== Type Aliases =====

/// Numeric difficulty threshold for a skill check.
///
/// A character's effective rank must meet or exceed this value to succeed.
/// Defined as a type alias over [`SkillRank`] so callers can name the
/// semantic role without a wrapper type.
pub type SkillCheckDifficulty = SkillRank;

// ===== Error Type =====

/// Errors produced when performing a skill check.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SkillCheckError {
    /// The requested skill is not present in the skill database.
    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    /// The character's class ID was not found in the class database.
    #[error("Class not found: {0}")]
    ClassNotFound(String),

    /// The character's race ID was not found in the race database.
    #[error("Race not found: {0}")]
    RaceNotFound(String),

    /// The difficulty value is not valid for this check.
    #[error("Invalid difficulty: {0}")]
    InvalidDifficulty(String),
}

impl From<SkillError> for SkillCheckError {
    fn from(e: SkillError) -> Self {
        match e {
            SkillError::SkillNotFound(id) => SkillCheckError::SkillNotFound(id),
            SkillError::ClassNotFound(id) => SkillCheckError::ClassNotFound(id),
            SkillError::RaceNotFound(id) => SkillCheckError::RaceNotFound(id),
            other => SkillCheckError::SkillNotFound(other.to_string()),
        }
    }
}

// ===== Request / Result Types =====

/// A request to evaluate a skill check for a single character or party.
///
/// Build this struct and pass it to [`skill_check_for_character`] or
/// [`evaluate_party_skill_scope`] depending on the scope required.
///
/// # Examples
///
/// ```
/// use antares::domain::skill_checks::SkillCheckRequest;
///
/// let req = SkillCheckRequest {
///     skill_id: "perception".to_string(),
///     difficulty: 5,
///     modifiers: vec![],
/// };
/// assert_eq!(req.skill_id, "perception");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillCheckRequest {
    /// The skill to check (must exist in the [`SkillDatabase`]).
    pub skill_id: SkillId,

    /// Minimum rank needed for success.
    pub difficulty: SkillCheckDifficulty,

    /// Optional flat signed bonuses or penalties applied to the resolved rank.
    ///
    /// Each element is added to the resolved rank before the comparison.
    /// Negative values are penalties. Positive values are bonuses.
    pub modifiers: Vec<i16>,
}

/// The outcome of a skill check evaluation.
///
/// `success` is the authoritative result. `rank`, `roll`, and `margin`
/// provide diagnostic context for logging and UI display.
///
/// # Examples
///
/// ```
/// use antares::domain::skill_checks::{SkillCheckResult, evaluate_skill_check_without_roll};
///
/// let result = evaluate_skill_check_without_roll(8, 8);
/// assert!(result.success);
/// assert_eq!(result.rank, 8);
/// assert_eq!(result.margin, 0);
/// assert!(result.roll.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillCheckResult {
    /// Whether the check succeeded (rank >= difficulty after all modifiers).
    pub success: bool,

    /// The effective rank used in the comparison (after modifiers).
    pub rank: SkillRank,

    /// The dice roll used, or `None` for deterministic checks.
    pub roll: Option<u16>,

    /// Signed difference: `rank as i32 - difficulty as i32`.
    ///
    /// Positive means succeeded with margin; negative means failed by margin.
    pub margin: i32,
}

// ===== Pure Check Functions =====

/// Compares `rank` against `difficulty` without any dice roll.
///
/// Succeeds when `rank >= difficulty`. This is the default check for all
/// Phase 3 integrations. Use randomized checks only when a specific game
/// mechanic explicitly requires non-determinism.
///
/// # Arguments
///
/// * `rank` — The character's effective skill rank (after all modifiers).
/// * `difficulty` — The minimum rank required for success.
///
/// # Examples
///
/// ```
/// use antares::domain::skill_checks::evaluate_skill_check_without_roll;
///
/// // Equal rank and difficulty — success.
/// let r = evaluate_skill_check_without_roll(5, 5);
/// assert!(r.success);
/// assert_eq!(r.margin, 0);
///
/// // One below difficulty — failure.
/// let r = evaluate_skill_check_without_roll(4, 5);
/// assert!(!r.success);
/// assert_eq!(r.margin, -1);
/// ```
pub fn evaluate_skill_check_without_roll(
    rank: SkillRank,
    difficulty: SkillCheckDifficulty,
) -> SkillCheckResult {
    let margin = rank as i32 - difficulty as i32;
    SkillCheckResult {
        success: rank >= difficulty,
        rank,
        roll: None,
        margin,
    }
}

/// Aggregates party member ranks according to `scope` then evaluates the check.
///
/// `member_ranks` is a slice of pre-resolved effective ranks — one element per
/// living party member. The slice must not include dead or incapacitated members
/// that should not contribute.
///
/// # Arguments
///
/// * `member_ranks` — Effective skill ranks for each living party member.
/// * `scope` — How to aggregate the ranks before comparing to `difficulty`.
/// * `active_speaker_rank` — Rank for the active dialogue speaker. Used by
///   [`PartySkillScope::ActiveSpeaker`]; falls back to the first member if `None`.
/// * `difficulty` — The minimum rank required for success.
///
/// # Returns
///
/// Returns a `SkillCheckResult` with `success = false` and `rank = 0` when
/// `member_ranks` is empty.
///
/// # Examples
///
/// ```
/// use antares::domain::skills::PartySkillScope;
/// use antares::domain::skill_checks::evaluate_party_skill_scope;
///
/// // AnyMember: succeeds if at least one member meets the difficulty.
/// let ranks = [2u16, 7, 4];
/// let result = evaluate_party_skill_scope(&ranks, &PartySkillScope::AnyMember, None, 6);
/// assert!(result.success);
/// assert_eq!(result.rank, 7); // reports best rank
///
/// // PartyTotal: sums all ranks.
/// let result = evaluate_party_skill_scope(&ranks, &PartySkillScope::PartyTotal, None, 12);
/// assert!(result.success); // 2+7+4 = 13 >= 12
/// ```
pub fn evaluate_party_skill_scope(
    member_ranks: &[SkillRank],
    scope: &PartySkillScope,
    active_speaker_rank: Option<SkillRank>,
    difficulty: SkillCheckDifficulty,
) -> SkillCheckResult {
    if member_ranks.is_empty() {
        return SkillCheckResult {
            success: false,
            rank: 0,
            roll: None,
            margin: -(difficulty as i32),
        };
    }

    match scope {
        PartySkillScope::AnyMember => {
            let best = member_ranks.iter().copied().max().unwrap_or(0);
            let success = member_ranks.iter().any(|&r| r >= difficulty);
            let margin = best as i32 - difficulty as i32;
            SkillCheckResult {
                success,
                rank: best,
                roll: None,
                margin,
            }
        }
        PartySkillScope::ActiveSpeaker => {
            let rank =
                active_speaker_rank.unwrap_or_else(|| member_ranks.first().copied().unwrap_or(0));
            evaluate_skill_check_without_roll(rank, difficulty)
        }
        PartySkillScope::PartyAverage => {
            let sum: u32 = member_ranks.iter().map(|&r| r as u32).sum();
            let avg = (sum / member_ranks.len() as u32) as SkillRank;
            evaluate_skill_check_without_roll(avg, difficulty)
        }
        PartySkillScope::PartyTotal => {
            let total: u32 = member_ranks.iter().map(|&r| r as u32).sum();
            let rank = total.min(SkillRank::MAX as u32) as SkillRank;
            evaluate_skill_check_without_roll(rank, difficulty)
        }
    }
}

/// Resolves a character's effective skill rank then evaluates the check deterministically.
///
/// This is the standard single-character check entry point used by engine mechanics.
/// Modifiers from `request.modifiers` are summed and applied to the resolved rank
/// before comparison. The adjusted rank is clamped to `[0, skill.max_rank]`.
///
/// # Arguments
///
/// * `ctx` — Character-level context (level, grants, persistent ranks).
/// * `request` — The check to evaluate (skill ID, difficulty, and modifiers).
/// * `skills` — Loaded skill database.
///
/// # Errors
///
/// * [`SkillCheckError::SkillNotFound`] — `request.skill_id` is not in `skills`.
///
/// # Examples
///
/// ```
/// use antares::domain::skills::{SkillDatabase, SkillGrant, CharacterSkillRanks};
/// use antares::domain::skill_resolver::{SkillResolver, SkillResolverContext};
/// use antares::domain::skill_checks::{SkillCheckRequest, skill_check_for_character};
///
/// let skill_ron = r#"[
///     (id: "perception", name: "Perception", category: Exploration,
///      description: "", scaling: Linear(base: 0, per_level: 1),
///      max_rank: 50, is_trainable: true),
/// ]"#;
/// let skills = SkillDatabase::load_from_string(skill_ron).unwrap();
/// let ranks = CharacterSkillRanks::new();
/// let grants: Vec<SkillGrant> = vec![];
/// let ctx = SkillResolverContext {
///     level: 6, class_id: "knight", race_id: "human",
///     char_ranks: &ranks, class_grants: &grants, race_grants: &grants,
/// };
///
/// let request = SkillCheckRequest {
///     skill_id: "perception".to_string(),
///     difficulty: 5,
///     modifiers: vec![],
/// };
///
/// // Level 6 linear(0, 1) = rank 5; difficulty 5 → success.
/// let result = skill_check_for_character(&ctx, &request, &skills).unwrap();
/// assert!(result.success);
/// assert_eq!(result.rank, 5);
/// assert_eq!(result.margin, 0);
/// ```
pub fn skill_check_for_character(
    ctx: &SkillResolverContext<'_>,
    request: &SkillCheckRequest,
    skills: &SkillDatabase,
) -> Result<SkillCheckResult, SkillCheckError> {
    let definition = skills
        .get(&request.skill_id)
        .ok_or_else(|| SkillCheckError::SkillNotFound(request.skill_id.clone()))?;

    let base_rank = SkillResolver::effective_skill_rank(ctx, &request.skill_id, skills)
        .map_err(SkillCheckError::from)?;

    let modifier_sum: i32 = request.modifiers.iter().map(|&m| m as i32).sum();
    let adjusted =
        (base_rank as i32 + modifier_sum).clamp(0, definition.max_rank as i32) as SkillRank;

    Ok(evaluate_skill_check_without_roll(
        adjusted,
        request.difficulty,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::skill_resolver::SkillResolverContext;
    use crate::domain::skills::{CharacterSkillRanks, PartySkillScope, SkillDatabase, SkillGrant};

    fn make_linear_skill_db() -> SkillDatabase {
        let ron = r#"[
        (id: "perception", name: "Perception", category: Exploration,
         description: "", scaling: Linear(base: 0, per_level: 1),
         max_rank: 50, is_trainable: true),
        (id: "disarm_traps", name: "Disarm Traps", category: Exploration,
         description: "", scaling: Linear(base: 0, per_level: 1),
         max_rank: 25, is_trainable: true),
    ]"#;
        SkillDatabase::load_from_string(ron).unwrap()
    }

    fn make_ctx<'a>(
        level: u32,
        char_ranks: &'a CharacterSkillRanks,
        class_grants: &'a [SkillGrant],
        race_grants: &'a [SkillGrant],
    ) -> SkillResolverContext<'a> {
        SkillResolverContext {
            level,
            class_id: "knight",
            race_id: "human",
            char_ranks,
            class_grants,
            race_grants,
        }
    }

    #[test]
    fn test_evaluate_skill_check_without_roll_success_at_threshold() {
        let result = evaluate_skill_check_without_roll(5, 5);
        assert!(result.success, "Equal rank and difficulty should succeed");
        assert_eq!(result.rank, 5);
        assert_eq!(result.margin, 0);
        assert!(result.roll.is_none());
    }

    #[test]
    fn test_evaluate_skill_check_without_roll_fails_below_threshold() {
        let result = evaluate_skill_check_without_roll(4, 5);
        assert!(!result.success, "Rank below difficulty should fail");
        assert_eq!(result.rank, 4);
        assert_eq!(result.margin, -1);
        assert!(result.roll.is_none());
    }

    #[test]
    fn test_evaluate_skill_check_without_roll_succeeds_above_threshold() {
        let result = evaluate_skill_check_without_roll(10, 3);
        assert!(result.success);
        assert_eq!(result.margin, 7);
    }

    #[test]
    fn test_evaluate_skill_check_without_roll_zero_difficulty_always_succeeds() {
        let result = evaluate_skill_check_without_roll(0, 0);
        assert!(result.success);
        assert_eq!(result.margin, 0);
    }

    #[test]
    fn test_skill_check_condition_any_member_uses_best_matching_member() {
        let ranks = [2u16, 7, 4];
        let result = evaluate_party_skill_scope(&ranks, &PartySkillScope::AnyMember, None, 6);
        assert!(result.success);
        assert_eq!(
            result.rank, 7,
            "AnyMember should report the best (highest) rank"
        );
        assert_eq!(result.margin, 1);
    }

    #[test]
    fn test_any_member_fails_when_no_member_meets_threshold() {
        let ranks = [1u16, 2, 3];
        let result = evaluate_party_skill_scope(&ranks, &PartySkillScope::AnyMember, None, 5);
        assert!(!result.success);
        assert_eq!(result.rank, 3);
    }

    #[test]
    fn test_skill_check_condition_party_total_sums_members() {
        let ranks = [2u16, 7, 4];
        let result = evaluate_party_skill_scope(&ranks, &PartySkillScope::PartyTotal, None, 12);
        assert!(result.success);
        assert_eq!(result.rank, 13);
        assert_eq!(result.margin, 1);
    }

    #[test]
    fn test_party_total_fails_when_sum_below_threshold() {
        let ranks = [1u16, 2, 3];
        let result = evaluate_party_skill_scope(&ranks, &PartySkillScope::PartyTotal, None, 10);
        assert!(!result.success);
        assert_eq!(result.rank, 6);
    }

    #[test]
    fn test_party_average_success() {
        let ranks = [2u16, 8, 6];
        let result = evaluate_party_skill_scope(&ranks, &PartySkillScope::PartyAverage, None, 5);
        assert!(result.success);
        assert_eq!(result.rank, 5);
    }

    #[test]
    fn test_active_speaker_uses_provided_rank() {
        let ranks = [1u16, 2, 3];
        let result =
            evaluate_party_skill_scope(&ranks, &PartySkillScope::ActiveSpeaker, Some(8), 5);
        assert!(result.success);
        assert_eq!(result.rank, 8);
    }

    #[test]
    fn test_active_speaker_falls_back_to_first_member_when_none() {
        let ranks = [6u16, 2, 3];
        let result = evaluate_party_skill_scope(&ranks, &PartySkillScope::ActiveSpeaker, None, 5);
        assert!(result.success);
        assert_eq!(result.rank, 6);
    }

    #[test]
    fn test_empty_party_returns_failure() {
        let result = evaluate_party_skill_scope(&[], &PartySkillScope::AnyMember, None, 5);
        assert!(!result.success);
        assert_eq!(result.rank, 0);
        assert_eq!(result.margin, -5);
    }

    #[test]
    fn test_skill_gated_dialogue_condition_allows_qualified_party() {
        let ranks = [3u16, 5, 2];
        let result = evaluate_party_skill_scope(&ranks, &PartySkillScope::AnyMember, None, 5);
        assert!(result.success, "Party with a qualified member should pass");
    }

    #[test]
    fn test_skill_gated_dialogue_condition_blocks_unqualified_party() {
        let ranks = [1u16, 3, 2];
        let result = evaluate_party_skill_scope(&ranks, &PartySkillScope::AnyMember, None, 5);
        assert!(
            !result.success,
            "Party without a qualified member should fail"
        );
    }

    #[test]
    fn test_skill_check_for_character_success_at_threshold() {
        let skills = make_linear_skill_db();
        let ranks = CharacterSkillRanks::new();
        let grants: Vec<SkillGrant> = vec![];
        let ctx = make_ctx(6, &ranks, &grants, &grants);
        let request = SkillCheckRequest {
            skill_id: "perception".to_string(),
            difficulty: 5,
            modifiers: vec![],
        };
        let result = skill_check_for_character(&ctx, &request, &skills).unwrap();
        assert!(result.success);
        assert_eq!(result.rank, 5);
        assert_eq!(result.margin, 0);
    }

    #[test]
    fn test_skill_check_for_character_failure_below_threshold() {
        let skills = make_linear_skill_db();
        let ranks = CharacterSkillRanks::new();
        let grants: Vec<SkillGrant> = vec![];
        let ctx = make_ctx(5, &ranks, &grants, &grants);
        let request = SkillCheckRequest {
            skill_id: "perception".to_string(),
            difficulty: 5,
            modifiers: vec![],
        };
        let result = skill_check_for_character(&ctx, &request, &skills).unwrap();
        assert!(!result.success);
        assert_eq!(result.rank, 4);
        assert_eq!(result.margin, -1);
    }

    #[test]
    fn test_skill_check_for_character_applies_positive_modifier() {
        let skills = make_linear_skill_db();
        let ranks = CharacterSkillRanks::new();
        let grants: Vec<SkillGrant> = vec![];
        let ctx = make_ctx(5, &ranks, &grants, &grants);
        let request = SkillCheckRequest {
            skill_id: "perception".to_string(),
            difficulty: 5,
            modifiers: vec![2],
        };
        let result = skill_check_for_character(&ctx, &request, &skills).unwrap();
        assert!(result.success);
        assert_eq!(result.rank, 6);
    }

    #[test]
    fn test_skill_check_for_character_applies_negative_modifier() {
        let skills = make_linear_skill_db();
        let ranks = CharacterSkillRanks::new();
        let grants: Vec<SkillGrant> = vec![];
        let ctx = make_ctx(6, &ranks, &grants, &grants);
        let request = SkillCheckRequest {
            skill_id: "perception".to_string(),
            difficulty: 5,
            modifiers: vec![-2],
        };
        let result = skill_check_for_character(&ctx, &request, &skills).unwrap();
        assert!(!result.success);
        assert_eq!(result.rank, 3);
    }

    #[test]
    fn test_skill_check_for_character_returns_error_for_missing_skill() {
        let skills = make_linear_skill_db();
        let ranks = CharacterSkillRanks::new();
        let grants: Vec<SkillGrant> = vec![];
        let ctx = make_ctx(5, &ranks, &grants, &grants);
        let request = SkillCheckRequest {
            skill_id: "nonexistent_skill".to_string(),
            difficulty: 5,
            modifiers: vec![],
        };
        let result = skill_check_for_character(&ctx, &request, &skills);
        assert!(matches!(result, Err(SkillCheckError::SkillNotFound(_))));
    }

    #[test]
    fn test_skill_check_error_display() {
        let e = SkillCheckError::SkillNotFound("perception".to_string());
        assert!(e.to_string().contains("perception"));
        let e2 = SkillCheckError::ClassNotFound("knight".to_string());
        assert!(e2.to_string().contains("knight"));
    }
}
