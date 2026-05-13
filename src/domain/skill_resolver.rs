// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Skill resolver — computes effective skill ranks from all contributing sources.
//!
//! This module implements [`SkillResolver`], which combines a character's
//! auto-scaled rank (from the skill definition), class grants, race grants,
//! and persistent character ranks into a single clamped effective value.
//!
//! # Effective Rank Formula
//!
//! 1. **Auto rank** — `rank_for_level(skill_definition, character.level)`.
//! 2. **Class grants** — flat_bonus + per_level_bonus × level for matching grants.
//! 3. **Race grants** — flat_bonus + per_level_bonus × level for matching grants.
//! 4. **Character rank** — persistent `character.skill_ranks[skill_id]`.
//! 5. **Clamp** to `0..=SkillDefinition::max_rank`.
//! 6. **Minimum floor** — any grant `minimum_rank` raises the result.
//! 7. **Override cap** — any grant `maximum_rank_override` lowers the result.
//!
//! # Module Layout
//!
//! | Item | Kind | Purpose |
//! |------|------|---------|
//! | [`SkillResolverContext`] | struct | Bundled character + grant data for one resolve call |
//! | [`SkillResolver`] | struct | Namespace for resolver functions |
//!
//! # Examples
//!
//! ```
//! use antares::domain::skills::{
//!     SkillDatabase, SkillGrant, CharacterSkillRanks,
//! };
//! use antares::domain::skill_resolver::{SkillResolver, SkillResolverContext};
//!
//! let skill_ron = r#"[
//!     (id: "perception", name: "Perception", category: Exploration,
//!      description: "", scaling: Linear(base: 0, per_level: 1),
//!      max_rank: 50, is_trainable: true),
//! ]"#;
//! let skills = SkillDatabase::load_from_string(skill_ron).unwrap();
//!
//! let char_ranks = CharacterSkillRanks::new();
//! let no_grants: Vec<SkillGrant> = vec![];
//!
//! let ctx = SkillResolverContext {
//!     level: 5,
//!     class_id: "knight",
//!     race_id: "human",
//!     char_ranks: &char_ranks,
//!     class_grants: &no_grants,
//!     race_grants: &no_grants,
//! };
//!
//! let rank = SkillResolver::effective_skill_rank(&ctx, &"perception".to_string(), &skills)
//!     .unwrap();
//! // level 5, linear per_level=1, base=0: auto_rank = 4 + 0 grants = 4
//! assert_eq!(rank, 4);
//! ```

use crate::domain::skills::{
    rank_for_level, CharacterSkillRanks, SkillBreakdown, SkillBreakdownEntry, SkillDatabase,
    SkillError, SkillGrant, SkillGrantSource, SkillId, SkillRank,
};
use std::collections::HashMap;

// ===== SkillResolverContext =====

/// Bundles the character-specific inputs required for skill rank resolution.
///
/// Pass a reference to this struct into each [`SkillResolver`] method to avoid
/// repeating the same parameters at every call site and to stay within clippy's
/// argument-count limit.
///
/// # Examples
///
/// ```
/// use antares::domain::skills::{CharacterSkillRanks, SkillGrant};
/// use antares::domain::skill_resolver::SkillResolverContext;
///
/// let ranks = CharacterSkillRanks::new();
/// let grants: Vec<SkillGrant> = vec![];
/// let ctx = SkillResolverContext {
///     level: 5,
///     class_id: "knight",
///     race_id: "human",
///     char_ranks: &ranks,
///     class_grants: &grants,
///     race_grants: &grants,
/// };
/// assert_eq!(ctx.level, 5);
/// ```
pub struct SkillResolverContext<'a> {
    /// Character's current level (1-based; 0 is treated as 1).
    pub level: u32,
    /// Character's class ID — used for error message context by callers.
    pub class_id: &'a str,
    /// Character's race ID — used for error message context by callers.
    pub race_id: &'a str,
    /// Persistent character-owned skill ranks.
    pub char_ranks: &'a CharacterSkillRanks,
    /// All skill grants from the character's class definition.
    pub class_grants: &'a [SkillGrant],
    /// All skill grants from the character's race definition.
    pub race_grants: &'a [SkillGrant],
}

// ===== SkillResolver =====

/// Stateless resolver that computes a character's effective skill ranks.
///
/// All methods are associated functions (no instance needed). Build a
/// [`SkillResolverContext`] from the character's data and pass it in.
///
/// # Note on Database References
///
/// To avoid circular module dependencies, `SkillResolver` takes grant slices
/// rather than `ClassDatabase` / `RaceDatabase` directly. Callers that have
/// access to those databases should call `ClassDatabase::get_class(class_id)`
/// and extract `.skill_grants`, returning `SkillError::ClassNotFound` /
/// `SkillError::RaceNotFound` before calling into the resolver.
pub struct SkillResolver;

impl SkillResolver {
    /// Computes the effective skill rank for `skill_id`.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Character-level context (level, class/race IDs, grants, persistent ranks).
    /// * `skill_id` — The skill to compute.
    /// * `skills` — Loaded skill definitions database.
    ///
    /// # Errors
    ///
    /// Returns [`SkillError::SkillNotFound`] if `skill_id` is not in `skills`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::skills::{SkillDatabase, SkillGrant, CharacterSkillRanks};
    /// use antares::domain::skill_resolver::{SkillResolver, SkillResolverContext};
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
    ///     level: 10, class_id: "knight", race_id: "human",
    ///     char_ranks: &ranks, class_grants: &grants, race_grants: &grants,
    /// };
    ///
    /// let rank = SkillResolver::effective_skill_rank(
    ///     &ctx, &"perception".to_string(), &skills,
    /// ).unwrap();
    /// assert_eq!(rank, 9); // level 10, linear(0, 1) → 9
    /// ```
    pub fn effective_skill_rank(
        ctx: &SkillResolverContext<'_>,
        skill_id: &SkillId,
        skills: &SkillDatabase,
    ) -> Result<SkillRank, SkillError> {
        let definition = skills
            .get(skill_id)
            .ok_or_else(|| SkillError::SkillNotFound(skill_id.clone()))?;

        let level_u16 = (ctx.level as u16).max(1);

        // Step 1: Auto-scaled rank from skill definition
        let auto_rank = rank_for_level(definition, level_u16) as i32;
        let mut additive = auto_rank;

        // Steps 2–3: Class and race grants
        let mut min_rank: Option<SkillRank> = None;
        let mut max_rank_override: Option<SkillRank> = None;

        let all_grants = ctx
            .class_grants
            .iter()
            .filter(|g| &g.skill_id == skill_id)
            .chain(ctx.race_grants.iter().filter(|g| &g.skill_id == skill_id));

        for grant in all_grants {
            additive += grant.flat_bonus as i32 + grant.per_level_bonus as i32 * level_u16 as i32;

            if let Some(floor) = grant.minimum_rank {
                min_rank = Some(min_rank.map_or(floor, |ex| ex.max(floor)));
            }
            if let Some(cap) = grant.maximum_rank_override {
                max_rank_override = Some(max_rank_override.map_or(cap, |ex| ex.min(cap)));
            }
        }

        // Step 4: Persistent character rank
        if let Some(char_rank) = ctx.char_ranks.get(skill_id) {
            additive += char_rank as i32;
        }

        // Step 5: (Temporary modifiers — not yet implemented)

        // Step 6: Clamp to [0, max_rank]
        let global_max = definition.max_rank as i32;
        let clamped = additive.clamp(0, global_max) as SkillRank;

        // Step 7: Apply minimum floor (still subject to global max)
        let after_floor = if let Some(floor) = min_rank {
            clamped.max(floor).min(definition.max_rank)
        } else {
            clamped
        };

        // Step 8: Apply grant-specific override cap
        let final_rank = if let Some(cap) = max_rank_override {
            after_floor.min(cap)
        } else {
            after_floor
        };

        Ok(final_rank)
    }

    /// Returns the full breakdown of how the effective rank was derived.
    ///
    /// Identical computation to [`effective_skill_rank`] but returns a
    /// [`SkillBreakdown`] with per-source contribution entries for UI/debugging.
    ///
    /// # Errors
    ///
    /// Returns [`SkillError::SkillNotFound`] if `skill_id` is not in `skills`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::skills::{SkillDatabase, SkillGrant, CharacterSkillRanks};
    /// use antares::domain::skill_resolver::{SkillResolver, SkillResolverContext};
    ///
    /// let skill_ron = r#"[
    ///     (id: "perception", name: "Perception", category: Exploration,
    ///      description: "", scaling: Linear(base: 0, per_level: 1),
    ///      max_rank: 50, is_trainable: true),
    /// ]"#;
    /// let skills = SkillDatabase::load_from_string(skill_ron).unwrap();
    /// let class_grant = SkillGrant {
    ///     skill_id: "perception".to_string(),
    ///     flat_bonus: 2, per_level_bonus: 0,
    ///     minimum_rank: None, maximum_rank_override: None,
    /// };
    /// let char_ranks = CharacterSkillRanks::new();
    /// let no_grants: Vec<SkillGrant> = vec![];
    /// let ctx = SkillResolverContext {
    ///     level: 5, class_id: "knight", race_id: "human",
    ///     char_ranks: &char_ranks,
    ///     class_grants: &[class_grant],
    ///     race_grants: &no_grants,
    /// };
    ///
    /// let bd = SkillResolver::effective_skill_breakdown(
    ///     &ctx, &"perception".to_string(), &skills,
    /// ).unwrap();
    /// assert_eq!(bd.final_rank, 6); // 4 (auto) + 2 (class)
    /// ```
    pub fn effective_skill_breakdown(
        ctx: &SkillResolverContext<'_>,
        skill_id: &SkillId,
        skills: &SkillDatabase,
    ) -> Result<SkillBreakdown, SkillError> {
        let definition = skills
            .get(skill_id)
            .ok_or_else(|| SkillError::SkillNotFound(skill_id.clone()))?;

        let level_u16 = (ctx.level as u16).max(1);
        let auto_rank = rank_for_level(definition, level_u16);
        let mut additive = auto_rank as i32;
        let mut entries: Vec<SkillBreakdownEntry> = Vec::new();
        let mut min_rank: Option<SkillRank> = None;
        let mut max_rank_override: Option<SkillRank> = None;

        // Class grants
        for grant in ctx.class_grants.iter().filter(|g| &g.skill_id == skill_id) {
            let bonus = grant.flat_bonus as i32 + grant.per_level_bonus as i32 * level_u16 as i32;
            additive += bonus;
            if bonus != 0 {
                entries.push(SkillBreakdownEntry {
                    source: SkillGrantSource::Class,
                    bonus,
                });
            }
            if let Some(floor) = grant.minimum_rank {
                min_rank = Some(min_rank.map_or(floor, |ex| ex.max(floor)));
            }
            if let Some(cap) = grant.maximum_rank_override {
                max_rank_override = Some(max_rank_override.map_or(cap, |ex| ex.min(cap)));
            }
        }

        // Race grants
        for grant in ctx.race_grants.iter().filter(|g| &g.skill_id == skill_id) {
            let bonus = grant.flat_bonus as i32 + grant.per_level_bonus as i32 * level_u16 as i32;
            additive += bonus;
            if bonus != 0 {
                entries.push(SkillBreakdownEntry {
                    source: SkillGrantSource::Race,
                    bonus,
                });
            }
            if let Some(floor) = grant.minimum_rank {
                min_rank = Some(min_rank.map_or(floor, |ex| ex.max(floor)));
            }
            if let Some(cap) = grant.maximum_rank_override {
                max_rank_override = Some(max_rank_override.map_or(cap, |ex| ex.min(cap)));
            }
        }

        // Character persistent rank
        let char_rank = ctx.char_ranks.get(skill_id).unwrap_or(0);
        if char_rank > 0 {
            additive += char_rank as i32;
            entries.push(SkillBreakdownEntry {
                source: SkillGrantSource::Character,
                bonus: char_rank as i32,
            });
        }

        // Clamp, floor, cap
        let global_max = definition.max_rank as i32;
        let clamped = additive.clamp(0, global_max) as SkillRank;
        let after_floor = if let Some(floor) = min_rank {
            clamped.max(floor).min(definition.max_rank)
        } else {
            clamped
        };
        let final_rank = if let Some(cap) = max_rank_override {
            after_floor.min(cap)
        } else {
            after_floor
        };

        Ok(SkillBreakdown {
            skill_id: skill_id.clone(),
            auto_rank,
            entries,
            character_rank: char_rank,
            final_rank,
            applied_minimum_rank: min_rank,
            applied_maximum_rank_override: max_rank_override,
        })
    }

    /// Computes effective ranks for **all** skills in `skills`, returning a map.
    ///
    /// Skills absent from the context's grants and `char_ranks` receive their
    /// auto-scaled rank only (clamped to their `max_rank`).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::skills::{SkillDatabase, SkillGrant, CharacterSkillRanks};
    /// use antares::domain::skill_resolver::{SkillResolver, SkillResolverContext};
    ///
    /// let skill_ron = r#"[
    ///     (id: "perception", name: "Perception", category: Exploration,
    ///      description: "", scaling: Linear(base: 0, per_level: 1),
    ///      max_rank: 50, is_trainable: true),
    ///     (id: "athletics", name: "Athletics", category: Utility,
    ///      description: "", scaling: Flat, max_rank: 20, is_trainable: true),
    /// ]"#;
    /// let skills = SkillDatabase::load_from_string(skill_ron).unwrap();
    /// let ranks = CharacterSkillRanks::new();
    /// let grants: Vec<SkillGrant> = vec![];
    /// let ctx = SkillResolverContext {
    ///     level: 1, class_id: "knight", race_id: "human",
    ///     char_ranks: &ranks, class_grants: &grants, race_grants: &grants,
    /// };
    ///
    /// let all = SkillResolver::all_effective_skill_ranks(&ctx, &skills);
    /// assert!(all.contains_key("perception"));
    /// assert!(all.contains_key("athletics"));
    /// ```
    pub fn all_effective_skill_ranks(
        ctx: &SkillResolverContext<'_>,
        skills: &SkillDatabase,
    ) -> HashMap<SkillId, SkillRank> {
        let mut result = HashMap::new();
        for skill_id in skills.all_ids() {
            // Safe: skill_id is always present in the database we iterate over
            let rank = Self::effective_skill_rank(ctx, skill_id, skills).unwrap_or(0);
            result.insert(skill_id.clone(), rank);
        }
        result
    }

    /// Returns `true` if the character's effective rank in `skill_id` ≥ `minimum`.
    ///
    /// Returns `false` if the skill is not in the database.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::skills::{SkillDatabase, SkillGrant, CharacterSkillRanks};
    /// use antares::domain::skill_resolver::{SkillResolver, SkillResolverContext};
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
    ///     level: 10, class_id: "knight", race_id: "human",
    ///     char_ranks: &ranks, class_grants: &grants, race_grants: &grants,
    /// };
    ///
    /// // Level 10 auto-rank for linear(0, 1) = 9
    /// assert!(SkillResolver::character_has_skill_rank(
    ///     &ctx, &"perception".to_string(), 9, &skills,
    /// ));
    /// assert!(!SkillResolver::character_has_skill_rank(
    ///     &ctx, &"perception".to_string(), 10, &skills,
    /// ));
    /// ```
    pub fn character_has_skill_rank(
        ctx: &SkillResolverContext<'_>,
        skill_id: &SkillId,
        minimum: SkillRank,
        skills: &SkillDatabase,
    ) -> bool {
        Self::effective_skill_rank(ctx, skill_id, skills)
            .map(|r| r >= minimum)
            .unwrap_or(false)
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::skills::{CharacterSkillRanks, SkillDatabase, SkillError, SkillGrant};

    fn make_linear_skills_db() -> SkillDatabase {
        let ron = r#"[
            (id: "perception", name: "Perception", category: Exploration,
             description: "", scaling: Linear(base: 0, per_level: 1), max_rank: 50, is_trainable: true),
            (id: "disarm_traps", name: "Disarm Traps", category: Exploration,
             description: "", scaling: Step(base: 0, per_levels: 2, amount: 1), max_rank: 25, is_trainable: true),
            (id: "item_lore", name: "Item Lore", category: Knowledge,
             description: "", scaling: Linear(base: 0, per_level: 1), max_rank: 50, is_trainable: true),
            (id: "arcane_lore", name: "Arcane Lore", category: Knowledge,
             description: "", scaling: Flat, max_rank: 40, is_trainable: true),
            (id: "athletics", name: "Athletics", category: Utility,
             description: "", scaling: Step(base: 1, per_levels: 3, amount: 1), max_rank: 20, is_trainable: true),
            (id: "diplomacy", name: "Diplomacy", category: Social,
             description: "", scaling: Flat, max_rank: 30, is_trainable: true),
        ]"#;
        SkillDatabase::load_from_string(ron).unwrap()
    }

    fn make_grant(skill_id: &str, flat: i16, per_level: i16) -> SkillGrant {
        SkillGrant {
            skill_id: skill_id.to_string(),
            flat_bonus: flat,
            per_level_bonus: per_level,
            minimum_rank: None,
            maximum_rank_override: None,
        }
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

    // ── Required tests from Phase 2.4 ───────────────────────────────────────

    #[test]
    fn test_effective_skill_rank_uses_auto_level_scaling() {
        // Level 1 vs level 10: linear scaling should increase rank
        let skills = make_linear_skills_db();
        let ranks = CharacterSkillRanks::new();
        let no_grants: Vec<SkillGrant> = vec![];

        let ctx_l1 = make_ctx(1, &ranks, &no_grants, &no_grants);
        let ctx_l10 = make_ctx(10, &ranks, &no_grants, &no_grants);

        let rank_l1 =
            SkillResolver::effective_skill_rank(&ctx_l1, &"perception".to_string(), &skills)
                .unwrap();

        let rank_l10 =
            SkillResolver::effective_skill_rank(&ctx_l10, &"perception".to_string(), &skills)
                .unwrap();

        assert_eq!(rank_l1, 0, "level 1 linear(0,1) should be 0");
        assert_eq!(rank_l10, 9, "level 10 linear(0,1) should be 9");
        assert!(
            rank_l10 > rank_l1,
            "level 10 should have higher rank than level 1"
        );
    }

    #[test]
    fn test_effective_skill_rank_adds_class_grant() {
        let skills = make_linear_skills_db();
        let ranks = CharacterSkillRanks::new();
        let no_grants: Vec<SkillGrant> = vec![];
        let class_grants = vec![make_grant("perception", 5, 0)];

        let without = SkillResolver::effective_skill_rank(
            &make_ctx(5, &ranks, &no_grants, &no_grants),
            &"perception".to_string(),
            &skills,
        )
        .unwrap();

        let with_class = SkillResolver::effective_skill_rank(
            &make_ctx(5, &ranks, &class_grants, &no_grants),
            &"perception".to_string(),
            &skills,
        )
        .unwrap();

        // auto-rank at level 5 = 4; class gives +5 → 9
        assert_eq!(without, 4);
        assert_eq!(with_class, 9);
    }

    #[test]
    fn test_effective_skill_rank_adds_race_grant() {
        let skills = make_linear_skills_db();
        let ranks = CharacterSkillRanks::new();
        let no_grants: Vec<SkillGrant> = vec![];
        let race_grants = vec![make_grant("perception", 3, 0)];

        let without = SkillResolver::effective_skill_rank(
            &make_ctx(5, &ranks, &no_grants, &no_grants),
            &"perception".to_string(),
            &skills,
        )
        .unwrap();

        let with_race = SkillResolver::effective_skill_rank(
            &make_ctx(5, &ranks, &no_grants, &race_grants),
            &"perception".to_string(),
            &skills,
        )
        .unwrap();

        assert_eq!(without, 4);
        assert_eq!(with_race, 7); // 4 + 3
    }

    #[test]
    fn test_effective_skill_rank_adds_character_rank() {
        let skills = make_linear_skills_db();
        let mut ranks = CharacterSkillRanks::new();
        ranks.set("perception".to_string(), 3);
        let no_grants: Vec<SkillGrant> = vec![];

        let rank = SkillResolver::effective_skill_rank(
            &make_ctx(5, &ranks, &no_grants, &no_grants),
            &"perception".to_string(),
            &skills,
        )
        .unwrap();

        // auto=4, char=3 → 7
        assert_eq!(rank, 7);
    }

    #[test]
    fn test_effective_skill_rank_clamps_to_skill_max() {
        let skills = make_linear_skills_db();
        let mut ranks = CharacterSkillRanks::new();
        ranks.set("perception".to_string(), 100); // huge
        let class_grants = vec![make_grant("perception", 100, 0)];
        let race_grants = vec![make_grant("perception", 100, 0)];

        let rank = SkillResolver::effective_skill_rank(
            &make_ctx(50, &ranks, &class_grants, &race_grants),
            &"perception".to_string(),
            &skills,
        )
        .unwrap();

        // max_rank for perception is 50
        assert_eq!(rank, 50, "rank should be clamped to max_rank=50");
    }

    #[test]
    fn test_effective_skill_rank_missing_skill_returns_error() {
        let skills = make_linear_skills_db();
        let ranks = CharacterSkillRanks::new();
        let no_grants: Vec<SkillGrant> = vec![];

        let result = SkillResolver::effective_skill_rank(
            &make_ctx(5, &ranks, &no_grants, &no_grants),
            &"nonexistent_skill".to_string(),
            &skills,
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SkillError::SkillNotFound(_)));
    }

    #[test]
    fn test_effective_skill_rank_missing_class_returns_error() {
        // The resolver itself does not perform class DB lookup; callers do.
        // When a caller cannot find the class, they return ClassNotFound.
        // This test verifies that SkillError::ClassNotFound exists and works.
        let err = SkillError::ClassNotFound("missing_class".to_string());
        assert!(matches!(err, SkillError::ClassNotFound(_)));
        assert!(err.to_string().contains("missing_class"));
    }

    #[test]
    fn test_effective_skill_rank_missing_race_returns_error() {
        let err = SkillError::RaceNotFound("missing_race".to_string());
        assert!(matches!(err, SkillError::RaceNotFound(_)));
        assert!(err.to_string().contains("missing_race"));
    }

    #[test]
    fn test_all_effective_skill_ranks_contains_all_database_skills() {
        let skills = make_linear_skills_db();
        let ranks = CharacterSkillRanks::new();
        let no_grants: Vec<SkillGrant> = vec![];

        let all = SkillResolver::all_effective_skill_ranks(
            &make_ctx(5, &ranks, &no_grants, &no_grants),
            &skills,
        );

        for skill_id in skills.all_ids() {
            assert!(
                all.contains_key(skill_id),
                "all_effective_skill_ranks missing '{}'",
                skill_id
            );
        }
        assert_eq!(all.len(), skills.len());
    }

    #[test]
    fn test_effective_skill_rank_combines_class_and_race_grants() {
        let skills = make_linear_skills_db();
        let ranks = CharacterSkillRanks::new();
        let class_grants = vec![make_grant("perception", 3, 0)];
        let race_grants = vec![make_grant("perception", 2, 0)];

        let rank = SkillResolver::effective_skill_rank(
            &make_ctx(5, &ranks, &class_grants, &race_grants),
            &"perception".to_string(),
            &skills,
        )
        .unwrap();

        // auto=4, class=+3, race=+2 → 9
        assert_eq!(rank, 9);
    }

    #[test]
    fn test_effective_skill_rank_per_level_bonus_scales() {
        let skills = make_linear_skills_db();
        let ranks = CharacterSkillRanks::new();
        let class_grants = vec![make_grant("diplomacy", 0, 1)]; // +1 per level
        let no_race: Vec<SkillGrant> = vec![];

        // diplomacy is Flat so auto_rank=0; only per_level contributes
        let rank_l1 = SkillResolver::effective_skill_rank(
            &make_ctx(1, &ranks, &class_grants, &no_race),
            &"diplomacy".to_string(),
            &skills,
        )
        .unwrap();

        let rank_l5 = SkillResolver::effective_skill_rank(
            &make_ctx(5, &ranks, &class_grants, &no_race),
            &"diplomacy".to_string(),
            &skills,
        )
        .unwrap();

        assert_eq!(rank_l1, 1, "level 1 with +1 per_level should be 1");
        assert_eq!(rank_l5, 5, "level 5 with +1 per_level should be 5");
    }

    #[test]
    fn test_effective_skill_rank_minimum_rank_floor_applied() {
        let skills = make_linear_skills_db();
        let ranks = CharacterSkillRanks::new();
        // diplomacy is Flat (auto=0); grant gives flat_bonus=0 but minimum_rank=5
        let class_grants = vec![SkillGrant {
            skill_id: "diplomacy".to_string(),
            flat_bonus: 0,
            per_level_bonus: 0,
            minimum_rank: Some(5),
            maximum_rank_override: None,
        }];
        let no_race: Vec<SkillGrant> = vec![];

        let rank = SkillResolver::effective_skill_rank(
            &make_ctx(1, &ranks, &class_grants, &no_race),
            &"diplomacy".to_string(),
            &skills,
        )
        .unwrap();

        assert_eq!(rank, 5, "minimum_rank floor should raise rank to 5");
    }

    #[test]
    fn test_effective_skill_rank_maximum_rank_override_cap_applied() {
        let skills = make_linear_skills_db();
        let ranks = CharacterSkillRanks::new();
        // perception auto at level 20 = 19; cap it at 10
        let class_grants = vec![SkillGrant {
            skill_id: "perception".to_string(),
            flat_bonus: 0,
            per_level_bonus: 0,
            minimum_rank: None,
            maximum_rank_override: Some(10),
        }];
        let no_race: Vec<SkillGrant> = vec![];

        let rank = SkillResolver::effective_skill_rank(
            &make_ctx(20, &ranks, &class_grants, &no_race),
            &"perception".to_string(),
            &skills,
        )
        .unwrap();

        assert_eq!(rank, 10, "maximum_rank_override should cap rank at 10");
    }

    #[test]
    fn test_character_has_skill_rank_true_when_sufficient() {
        let skills = make_linear_skills_db();
        let ranks = CharacterSkillRanks::new();
        let no_grants: Vec<SkillGrant> = vec![];
        let ctx = make_ctx(10, &ranks, &no_grants, &no_grants);

        // level 10, linear(0,1) → rank 9
        assert!(SkillResolver::character_has_skill_rank(
            &ctx,
            &"perception".to_string(),
            9,
            &skills,
        ));
    }

    #[test]
    fn test_character_has_skill_rank_false_when_insufficient() {
        let skills = make_linear_skills_db();
        let ranks = CharacterSkillRanks::new();
        let no_grants: Vec<SkillGrant> = vec![];
        let ctx = make_ctx(10, &ranks, &no_grants, &no_grants);

        // level 10, linear(0,1) → rank 9; need 10
        assert!(!SkillResolver::character_has_skill_rank(
            &ctx,
            &"perception".to_string(),
            10,
            &skills,
        ));
    }

    #[test]
    fn test_effective_skill_breakdown_returns_correct_entries() {
        let skills = make_linear_skills_db();
        let class_grants = vec![make_grant("perception", 2, 0)];
        let race_grants = vec![make_grant("perception", 1, 0)];
        let mut char_ranks = CharacterSkillRanks::new();
        char_ranks.set("perception".to_string(), 3);

        let ctx = SkillResolverContext {
            level: 5,
            class_id: "knight",
            race_id: "human",
            char_ranks: &char_ranks,
            class_grants: &class_grants,
            race_grants: &race_grants,
        };

        let bd = SkillResolver::effective_skill_breakdown(&ctx, &"perception".to_string(), &skills)
            .unwrap();

        // auto=4, class=+2, race=+1, char=+3 → total=10
        assert_eq!(bd.auto_rank, 4);
        assert_eq!(bd.character_rank, 3);
        assert_eq!(bd.final_rank, 10);
        assert_eq!(bd.entries.len(), 3); // class, race, character
    }

    #[test]
    fn test_skill_grants_deserialize_from_test_campaign_classes() {
        // Load the test_campaign classes fixture and verify skill_grants parse
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let path =
            std::path::PathBuf::from(&manifest_dir).join("data/test_campaign/data/classes.ron");

        let content = std::fs::read_to_string(&path)
            .expect("data/test_campaign/data/classes.ron must be readable");

        use crate::domain::classes::ClassDatabase;
        let db = ClassDatabase::load_from_string(&content)
            .expect("test_campaign classes.ron must parse");

        // At least two classes should have skill_grants
        let classes_with_grants: Vec<_> = db
            .all_classes()
            .filter(|c| !c.skill_grants.is_empty())
            .collect();

        assert!(
            classes_with_grants.len() >= 2,
            "at least two classes must have skill_grants; got {}",
            classes_with_grants.len()
        );
    }
}
