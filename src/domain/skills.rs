// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Skill system — level-scaled numeric character capabilities.
//!
//! Skills are distinct from proficiencies:
//!
//! - **Proficiencies** are binary item-use permissions (`simple_weapon`, `light_armor`, …).
//! - **Skills** are numeric values (`perception: 7`, `disarm_traps: 3`) that can grow
//!   with character level, class grants, race grants, and paid NPC training.
//!
//! # Module Layout
//!
//! | Item | Kind | Purpose |
//! |------|------|---------|
//! | [`SkillId`] | type alias | Stable campaign-authored identifier |
//! | [`SkillRank`] | type alias | Numeric rank value (0–65535) |
//! | [`SkillCategory`] | enum | UI grouping |
//! | [`SkillScalingMode`] | enum | How a skill's auto-rank grows with level |
//! | [`SkillGrantSource`] | enum | Which system produced a skill bonus |
//! | [`PartySkillScope`] | enum | Party scope for skill-gated dialogue conditions |
//! | [`SkillGrant`] | struct | Data-driven bonus attached to class/race |
//! | [`CharacterSkillRanks`] | struct | Persistent character-owned skill ranks |
//! | [`SkillBreakdown`] | struct | Full rank derivation with source breakdown |
//! | [`SkillDefinition`] | struct | A single campaign-authored skill |
//! | [`SkillDatabase`] | struct | Loaded, validated collection of definitions |
//! | [`SkillError`] | enum | All error conditions for this module |
//! | [`rank_for_level`] | fn | Pure scaling computation |
//! | [`rank_for_level_with_bonus`] | fn | Scaling + signed flat bonus |
//! | [`validate_skill_id`] | fn | Enforce `lowercase_snake_case` identifiers |
//! | [`validate_skill_rank`] | fn | Assert rank does not exceed cap |
//!
//! # Examples
//!
//! ```
//! use antares::domain::skills::{
//!     SkillDefinition, SkillCategory, SkillScalingMode, SkillDatabase, rank_for_level,
//! };
//!
//! let perception = SkillDefinition {
//!     id: "perception".to_string(),
//!     name: "Perception".to_string(),
//!     category: SkillCategory::Exploration,
//!     description: "Awareness of hidden objects and traps.".to_string(),
//!     scaling: SkillScalingMode::Linear { base: 0, per_level: 1 },
//!     max_rank: 50,
//!     is_trainable: true,
//! };
//!
//! assert_eq!(rank_for_level(&perception, 1), 0);
//! assert_eq!(rank_for_level(&perception, 5), 4);
//! assert_eq!(rank_for_level(&perception, 10), 9);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// ===== Error Types =====

/// Errors produced by the skill system.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SkillError {
    /// A skill ID was looked up but not found in the database.
    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    /// The skill data file could not be read from disk.
    #[error("Failed to load skills from file: {0}")]
    LoadError(String),

    /// The RON data could not be parsed.
    #[error("Failed to parse skill data: {0}")]
    ParseError(String),

    /// A skill definition failed structural validation.
    #[error("Skill validation error: {0}")]
    ValidationError(String),

    /// Two or more skill definitions share the same `id`.
    #[error("Duplicate skill ID: {0}")]
    DuplicateId(String),

    /// The character's class ID was not found in the class database.
    #[error("Class not found: {0}")]
    ClassNotFound(String),

    /// The character's race ID was not found in the race database.
    #[error("Race not found: {0}")]
    RaceNotFound(String),

    /// A skill grant references a skill ID that does not exist in the database.
    #[error("Invalid skill reference in grant: {0}")]
    InvalidSkillReference(String),
}

// ===== Type Aliases =====

/// Stable campaign-authored skill identifier.
///
/// Must be lowercase `snake_case` (e.g. `"disarm_traps"`, `"item_lore"`).
/// Use [`validate_skill_id`] to enforce this constraint.
pub type SkillId = String;

/// Numeric rank value for a skill.
///
/// `0` means untrained; higher values represent greater capability.
/// The hard cap is defined per skill via [`SkillDefinition::max_rank`].
pub type SkillRank = u16;

// ===== Enums =====

/// Grouping category for UI display and filtering.
///
/// # Examples
///
/// ```
/// use antares::domain::skills::SkillCategory;
///
/// let cat = SkillCategory::Exploration;
/// assert_eq!(cat, SkillCategory::Exploration);
/// assert_ne!(cat, SkillCategory::Combat);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SkillCategory {
    /// Combat-adjacent tactical skills (e.g. leadership, tactics).
    Combat,
    /// World traversal, traps, perception, and discovery.
    #[default]
    Exploration,
    /// Lore, item identification, and arcane/divine knowledge.
    Knowledge,
    /// Diplomacy, intimidation, and bargaining.
    Social,
    /// General non-combat utility (e.g. athletics, swimming, climbing).
    Utility,
}

/// Describes how a skill's effective rank grows automatically with character level.
///
/// The computed rank is always clamped to `0..=SkillDefinition::max_rank`.
///
/// | Variant | Growth rule |
/// |---------|-------------|
/// | `Flat` | Never increases; rank is always 0 from auto-scaling |
/// | `Linear` | `base + per_level * (level - 1)` |
/// | `Step` | `base + ((level - 1) / per_levels) * amount` |
/// | `Table` | Explicit rank lookup by level, clamped to last entry |
///
/// # Examples
///
/// ```
/// use antares::domain::skills::SkillScalingMode;
///
/// // A skill that gains 1 rank every other level starting at 0.
/// let mode = SkillScalingMode::Step { base: 0, per_levels: 2, amount: 1 };
///
/// // A skill that never auto-scales.
/// let flat = SkillScalingMode::Flat;
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillScalingMode {
    /// Rank never increases automatically. Auto-scaled contribution is always 0.
    Flat,

    /// `base + per_level * (level - 1)`.
    ///
    /// At level 1 the rank equals `base`.
    Linear {
        /// Rank at level 1.
        base: SkillRank,
        /// Rank gained per additional level beyond level 1.
        per_level: u16,
    },

    /// `base + ((level - 1) / per_levels) * amount`.
    ///
    /// Increases by `amount` every `per_levels` levels.
    Step {
        /// Rank at level 1.
        base: SkillRank,
        /// Number of levels between each rank increase. Must be > 0.
        per_levels: u16,
        /// Rank gained at each step.
        amount: u16,
    },

    /// Explicit rank lookup by level.
    ///
    /// `ranks_by_level[0]` is the rank at level 1.
    /// Levels beyond the last entry clamp to the last value.
    /// Must not be empty.
    Table {
        /// Explicit rank values indexed by `level - 1`.
        ranks_by_level: Vec<SkillRank>,
    },
}

// ===== Skill Grant Types =====

/// Identifies the originating source of a [`SkillGrant`].
///
/// Used in [`SkillBreakdown`] to show players (and developers) which system
/// contributed each bonus to a character's effective skill rank.
///
/// # Examples
///
/// ```
/// use antares::domain::skills::SkillGrantSource;
///
/// let source = SkillGrantSource::Class;
/// assert_eq!(source, SkillGrantSource::Class);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillGrantSource {
    /// Bonus comes from the character's class definition.
    Class,
    /// Bonus comes from the character's race definition.
    Race,
    /// Bonus comes from explicit persistent character ranks.
    Character,
    /// Bonus comes from paid NPC training (future use).
    Training,
    /// Bonus is temporary (e.g., spell effect). Not yet implemented.
    Temporary,
}

/// Scope for party-wide skill checks in dialogue conditions.
///
/// Determines which party members' skill ranks are considered when evaluating
/// a [`crate::domain::dialogue::DialogueCondition::SkillCheck`] condition.
///
/// # Examples
///
/// ```
/// use antares::domain::skills::PartySkillScope;
///
/// let scope = PartySkillScope::AnyMember;
/// assert_eq!(scope, PartySkillScope::AnyMember);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartySkillScope {
    /// At least one party member meets the minimum rank threshold.
    AnyMember,
    /// The party member currently leading dialogue meets the threshold.
    ActiveSpeaker,
    /// Average rank across living members meets the threshold.
    PartyAverage,
    /// Sum of ranks across living members meets the threshold.
    PartyTotal,
}

/// A data-driven bonus to a single skill, attached to a class or race definition.
///
/// All bonuses are additive. `per_level_bonus` is multiplied by the
/// character's level before adding. Both can be negative (for penalties).
///
/// # Examples
///
/// ```
/// use antares::domain::skills::{SkillGrant, SkillGrantSource};
///
/// let grant = SkillGrant {
///     skill_id: "perception".to_string(),
///     flat_bonus: 2,
///     per_level_bonus: 0,
///     minimum_rank: None,
///     maximum_rank_override: None,
/// };
/// assert_eq!(grant.skill_id, "perception");
/// assert_eq!(grant.flat_bonus, 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillGrant {
    /// The skill this grant applies to.
    pub skill_id: SkillId,

    /// Flat bonus added regardless of level.
    pub flat_bonus: i16,

    /// Additional bonus per character level. Defaults to 0.
    #[serde(default)]
    pub per_level_bonus: i16,

    /// Optional floor for the effective rank from this source.
    ///
    /// Applied after all additive bonuses and the global max-rank clamp.
    pub minimum_rank: Option<SkillRank>,

    /// Optional grant-specific cap that overrides the global `max_rank`.
    ///
    /// Applied as the very last step, so it can only reduce the result.
    pub maximum_rank_override: Option<SkillRank>,
}

/// Persistent, character-owned skill ranks keyed by [`SkillId`].
///
/// Represents explicit ranks a character has accrued through NPC training or
/// manual assignment. Auto-derived class/race grants are computed on demand
/// by [`SkillResolver`] and are NOT stored here.
///
/// Supports `#[serde(default)]` at the field site via the `Default` derive.
///
/// # Examples
///
/// ```
/// use antares::domain::skills::{CharacterSkillRanks, SkillRank};
///
/// let mut ranks = CharacterSkillRanks::new();
/// ranks.set("perception".to_string(), 5);
/// assert_eq!(ranks.get(&"perception".to_string()), Some(5));
/// assert!(ranks.contains(&"perception".to_string()));
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CharacterSkillRanks(pub HashMap<SkillId, SkillRank>);

impl CharacterSkillRanks {
    /// Creates a new, empty ranks map.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Looks up the persistent rank for `id`.
    ///
    /// Returns `None` if the skill has not been explicitly ranked.
    pub fn get(&self, id: &SkillId) -> Option<SkillRank> {
        self.0.get(id).copied()
    }

    /// Inserts or updates the persistent rank for `id`.
    pub fn set(&mut self, id: SkillId, rank: SkillRank) {
        self.0.insert(id, rank);
    }

    /// Increments the persistent rank for `id` by 1.
    ///
    /// Inserts at rank `1` if the skill was previously absent.
    pub fn increment(&mut self, id: &SkillId) {
        let entry = self.0.entry(id.clone()).or_insert(0);
        *entry = entry.saturating_add(1);
    }

    /// Removes the persistent rank entry for `id`.
    ///
    /// No-op if the skill was not present.
    pub fn remove(&mut self, id: &SkillId) {
        self.0.remove(id);
    }

    /// Returns `true` if an explicit persistent rank exists for `id`.
    pub fn contains(&self, id: &SkillId) -> bool {
        self.0.contains_key(id)
    }
}

/// One contribution line in a [`SkillBreakdown`].
///
/// Used for UI display and debugging to show which source contributed
/// how many ranks to the effective total.
///
/// # Examples
///
/// ```
/// use antares::domain::skills::{SkillBreakdownEntry, SkillGrantSource};
///
/// let entry = SkillBreakdownEntry { source: SkillGrantSource::Class, bonus: 3 };
/// assert_eq!(entry.bonus, 3);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillBreakdownEntry {
    /// Which system produced this bonus.
    pub source: SkillGrantSource,
    /// The signed rank contribution (can be negative).
    pub bonus: i32,
}

/// Full breakdown of how a character's effective skill rank was computed.
///
/// Returned by [`SkillResolver::effective_skill_breakdown`] for UI display
/// and debugging. The `final_rank` is authoritative; `entries` explain it.
///
/// # Examples
///
/// ```
/// use antares::domain::skills::{SkillBreakdown, SkillBreakdownEntry, SkillGrantSource};
///
/// let breakdown = SkillBreakdown {
///     skill_id: "perception".to_string(),
///     auto_rank: 4,
///     entries: vec![
///         SkillBreakdownEntry { source: SkillGrantSource::Class, bonus: 2 },
///     ],
///     character_rank: 0,
///     final_rank: 6,
///     applied_minimum_rank: None,
///     applied_maximum_rank_override: None,
/// };
/// assert_eq!(breakdown.final_rank, 6);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SkillBreakdown {
    /// The skill this breakdown is for.
    pub skill_id: SkillId,
    /// Rank from the skill's auto-scaling formula at the character's level.
    pub auto_rank: SkillRank,
    /// Individual grant contributions from class, race, etc.
    pub entries: Vec<SkillBreakdownEntry>,
    /// Persistent character-owned rank contribution.
    pub character_rank: SkillRank,
    /// Final effective rank after clamping, floors, and overrides.
    pub final_rank: SkillRank,
    /// The minimum_rank floor applied, if any.
    pub applied_minimum_rank: Option<SkillRank>,
    /// The maximum_rank_override cap applied, if any.
    pub applied_maximum_rank_override: Option<SkillRank>,
}

// ===== Structures =====

/// A single skill definition, loaded from `skills.ron`.
///
/// Skills define numeric level-scaled capabilities such as trap detection,
/// item lore, or diplomacy. They are separate from binary item proficiencies.
///
/// # Examples
///
/// ```
/// use antares::domain::skills::{SkillDefinition, SkillCategory, SkillScalingMode};
///
/// let skill = SkillDefinition {
///     id: "diplomacy".to_string(),
///     name: "Diplomacy".to_string(),
///     category: SkillCategory::Social,
///     description: "Ability to negotiate and persuade.".to_string(),
///     scaling: SkillScalingMode::Flat,
///     max_rank: 30,
///     is_trainable: true,
/// };
///
/// assert_eq!(skill.id, "diplomacy");
/// assert!(skill.is_trainable);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDefinition {
    /// Unique identifier in `lowercase_snake_case` (e.g. `"disarm_traps"`).
    pub id: SkillId,

    /// Human-readable display name (e.g. `"Disarm Traps"`).
    pub name: String,

    /// Category for UI grouping and filtering.
    pub category: SkillCategory,

    /// Long description shown in tooltips and documentation.
    #[serde(default)]
    pub description: String,

    /// How the auto-scaled rank grows with character level.
    pub scaling: SkillScalingMode,

    /// Hard cap for the effective rank regardless of source.
    pub max_rank: SkillRank,

    /// Whether this skill can be improved through paid NPC training.
    #[serde(default)]
    pub is_trainable: bool,
}

impl SkillDefinition {
    /// Validates this skill definition's structural constraints.
    ///
    /// Checks:
    /// - ID is valid lowercase snake_case and non-empty.
    /// - Name is non-empty.
    /// - `max_rank` is greater than 0.
    /// - `Step.per_levels` is greater than 0.
    /// - `Table.ranks_by_level` is non-empty and all ranks ≤ `max_rank`.
    ///
    /// # Errors
    ///
    /// Returns [`SkillError::ValidationError`] if any constraint is violated.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::skills::{SkillDefinition, SkillCategory, SkillScalingMode};
    ///
    /// let valid = SkillDefinition {
    ///     id: "athletics".to_string(),
    ///     name: "Athletics".to_string(),
    ///     category: SkillCategory::Utility,
    ///     description: String::new(),
    ///     scaling: SkillScalingMode::Flat,
    ///     max_rank: 20,
    ///     is_trainable: false,
    /// };
    /// assert!(valid.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), SkillError> {
        validate_skill_id(&self.id)?;

        if self.name.is_empty() {
            return Err(SkillError::ValidationError(format!(
                "Skill '{}' has an empty name",
                self.id
            )));
        }

        if self.max_rank == 0 {
            return Err(SkillError::ValidationError(format!(
                "Skill '{}' max_rank must be greater than 0",
                self.id
            )));
        }

        match &self.scaling {
            SkillScalingMode::Flat => {}
            SkillScalingMode::Linear { .. } => {
                // `per_level` is `u16`; the type system enforces >= 0.
            }
            SkillScalingMode::Step { per_levels, .. } => {
                if *per_levels == 0 {
                    return Err(SkillError::ValidationError(format!(
                        "Skill '{}' Step.per_levels must be greater than 0",
                        self.id
                    )));
                }
            }
            SkillScalingMode::Table { ranks_by_level } => {
                if ranks_by_level.is_empty() {
                    return Err(SkillError::ValidationError(format!(
                        "Skill '{}' Table.ranks_by_level must not be empty",
                        self.id
                    )));
                }
                for &rank in ranks_by_level {
                    if rank > self.max_rank {
                        return Err(SkillError::ValidationError(format!(
                            "Skill '{}' Table rank {} exceeds max_rank {}",
                            self.id, rank, self.max_rank
                        )));
                    }
                }
            }
        }

        Ok(())
    }
}

/// Loaded, validated collection of [`SkillDefinition`] records.
///
/// Backed by a `HashMap` keyed by [`SkillId`]. Loaded from a RON file whose
/// contents are a list of [`SkillDefinition`] structs.
///
/// # Examples
///
/// ```
/// use antares::domain::skills::{SkillDatabase, SkillDefinition, SkillCategory, SkillScalingMode};
///
/// let ron = r#"[
///     (id: "perception", name: "Perception", category: Exploration,
///      description: "Spot hidden threats.", scaling: Linear(base: 0, per_level: 1),
///      max_rank: 50, is_trainable: true),
/// ]"#;
///
/// let db = SkillDatabase::load_from_string(ron).unwrap();
/// assert!(db.has("perception"));
/// assert_eq!(db.len(), 1);
/// ```
#[derive(Debug, Clone, Default)]
pub struct SkillDatabase {
    skills: HashMap<SkillId, SkillDefinition>,
}

crate::impl_ron_database!(
    SkillDatabase,
    entity: SkillDefinition,
    key: String,
    error: SkillError,
    field: skills,
    id_of: |d: &SkillDefinition| d.id.clone(),
    dup_err: SkillError::DuplicateId,
    read_err: |e| SkillError::LoadError(e.to_string()),
    parse_err: |e| SkillError::ParseError(e.to_string()),
);

impl SkillDatabase {
    /// Creates a new, empty skill database.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::skills::SkillDatabase;
    ///
    /// let db = SkillDatabase::new();
    /// assert!(db.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Returns the skill definition for `id`, or `None` if not found.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::skills::{SkillDatabase, SkillDefinition, SkillCategory, SkillScalingMode};
    ///
    /// let ron = r#"[
    ///     (id: "diplomacy", name: "Diplomacy", category: Social,
    ///      description: "", scaling: Flat, max_rank: 30, is_trainable: true),
    /// ]"#;
    /// let db = SkillDatabase::load_from_string(ron).unwrap();
    ///
    /// assert!(db.get("diplomacy").is_some());
    /// assert!(db.get("nonexistent").is_none());
    /// ```
    pub fn get(&self, id: &str) -> Option<&SkillDefinition> {
        self.skills.get(id)
    }

    /// Returns `true` if a skill with the given `id` exists in the database.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::skills::SkillDatabase;
    ///
    /// let db = SkillDatabase::new();
    /// assert!(!db.has("anything"));
    /// ```
    pub fn has(&self, id: &str) -> bool {
        self.skills.contains_key(id)
    }

    /// Returns all skill definitions as an unordered vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::skills::SkillDatabase;
    ///
    /// let db = SkillDatabase::new();
    /// assert!(db.all().is_empty());
    /// ```
    pub fn all(&self) -> Vec<&SkillDefinition> {
        self.skills.values().collect()
    }

    /// Returns an iterator over all skill IDs in the database.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::skills::{SkillDatabase, SkillDefinition, SkillCategory, SkillScalingMode};
    ///
    /// let ron = r#"[
    ///     (id: "item_lore", name: "Item Lore", category: Knowledge,
    ///      description: "", scaling: Flat, max_rank: 50, is_trainable: true),
    /// ]"#;
    /// let db = SkillDatabase::load_from_string(ron).unwrap();
    ///
    /// let ids: Vec<_> = db.all_ids().collect();
    /// assert!(ids.contains(&&"item_lore".to_string()));
    /// ```
    pub fn all_ids(&self) -> impl Iterator<Item = &SkillId> {
        self.skills.keys()
    }

    /// Returns all skill definitions whose `category` matches `filter`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::skills::{SkillDatabase, SkillDefinition, SkillCategory, SkillScalingMode};
    ///
    /// let ron = r#"[
    ///     (id: "perception", name: "Perception", category: Exploration,
    ///      description: "", scaling: Flat, max_rank: 50, is_trainable: true),
    ///     (id: "diplomacy", name: "Diplomacy", category: Social,
    ///      description: "", scaling: Flat, max_rank: 30, is_trainable: true),
    /// ]"#;
    /// let db = SkillDatabase::load_from_string(ron).unwrap();
    ///
    /// let exploration = db.by_category(SkillCategory::Exploration);
    /// assert_eq!(exploration.len(), 1);
    /// assert_eq!(exploration[0].id, "perception");
    /// ```
    pub fn by_category(&self, filter: SkillCategory) -> Vec<&SkillDefinition> {
        self.skills
            .values()
            .filter(|s| s.category == filter)
            .collect()
    }

    /// Validates all skill definitions in the database.
    ///
    /// Calls [`SkillDefinition::validate`] on each entry.
    ///
    /// # Errors
    ///
    /// Returns [`SkillError::ValidationError`] for the first invalid skill found.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::skills::{SkillDatabase, SkillDefinition, SkillCategory, SkillScalingMode};
    ///
    /// let ron = r#"[
    ///     (id: "perception", name: "Perception", category: Exploration,
    ///      description: "", scaling: Linear(base: 0, per_level: 1),
    ///      max_rank: 50, is_trainable: true),
    /// ]"#;
    /// let db = SkillDatabase::load_from_string(ron).unwrap();
    /// assert!(db.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), SkillError> {
        for (key, skill) in &self.skills {
            skill.validate()?;
            if key != &skill.id {
                return Err(SkillError::ValidationError(format!(
                    "Skill map key '{}' does not match definition id '{}'",
                    key, skill.id
                )));
            }
        }
        Ok(())
    }

    /// Adds a skill definition to the database.
    ///
    /// # Errors
    ///
    /// Returns [`SkillError::DuplicateId`] if a skill with the same `id` already exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::skills::{SkillDatabase, SkillDefinition, SkillCategory, SkillScalingMode};
    ///
    /// let mut db = SkillDatabase::new();
    /// let result = db.add(SkillDefinition {
    ///     id: "athletics".to_string(),
    ///     name: "Athletics".to_string(),
    ///     category: SkillCategory::Utility,
    ///     description: String::new(),
    ///     scaling: SkillScalingMode::Flat,
    ///     max_rank: 20,
    ///     is_trainable: false,
    /// });
    /// assert!(result.is_ok());
    /// assert_eq!(db.len(), 1);
    /// ```
    pub fn add(&mut self, definition: SkillDefinition) -> Result<(), SkillError> {
        if self.skills.contains_key(&definition.id) {
            return Err(SkillError::DuplicateId(definition.id));
        }
        self.skills.insert(definition.id.clone(), definition);
        Ok(())
    }

    /// Removes the skill with the given `id` and returns it, or `None` if absent.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::skills::{SkillDatabase, SkillDefinition, SkillCategory, SkillScalingMode};
    ///
    /// let mut db = SkillDatabase::new();
    /// db.add(SkillDefinition {
    ///     id: "test_skill".to_string(),
    ///     name: "Test".to_string(),
    ///     category: SkillCategory::Utility,
    ///     description: String::new(),
    ///     scaling: SkillScalingMode::Flat,
    ///     max_rank: 10,
    ///     is_trainable: false,
    /// }).unwrap();
    ///
    /// assert!(db.remove("test_skill").is_some());
    /// assert!(db.is_empty());
    /// ```
    pub fn remove(&mut self, id: &str) -> Option<SkillDefinition> {
        self.skills.remove(id)
    }

    /// Returns the number of skill definitions in the database.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::skills::SkillDatabase;
    ///
    /// let db = SkillDatabase::new();
    /// assert_eq!(db.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// Returns `true` if the database contains no skill definitions.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::skills::SkillDatabase;
    ///
    /// let db = SkillDatabase::new();
    /// assert!(db.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}

// ===== Pure Scaling Functions =====

/// Computes a character's auto-scaled skill rank for the given `level`.
///
/// This is the contribution from the skill's [`SkillScalingMode`] alone.
/// Class grants, race grants, and trained ranks are added on top by the
/// resolver in Phase 2.
///
/// The result is clamped to `0..=definition.max_rank`.
///
/// # Arguments
///
/// * `definition` — The skill definition containing the scaling rule.
/// * `level` — The character's current level (1-based; level 0 is treated as 1).
///
/// # Examples
///
/// ```
/// use antares::domain::skills::{SkillDefinition, SkillCategory, SkillScalingMode, rank_for_level};
///
/// // Linear: perception gains 1 rank per level after level 1.
/// let perception = SkillDefinition {
///     id: "perception".to_string(),
///     name: "Perception".to_string(),
///     category: SkillCategory::Exploration,
///     description: String::new(),
///     scaling: SkillScalingMode::Linear { base: 0, per_level: 1 },
///     max_rank: 50,
///     is_trainable: true,
/// };
///
/// assert_eq!(rank_for_level(&perception, 1), 0);  // base
/// assert_eq!(rank_for_level(&perception, 2), 1);
/// assert_eq!(rank_for_level(&perception, 10), 9);
///
/// // Flat: diplomacy never auto-scales.
/// let diplomacy = SkillDefinition {
///     id: "diplomacy".to_string(),
///     name: "Diplomacy".to_string(),
///     category: SkillCategory::Social,
///     description: String::new(),
///     scaling: SkillScalingMode::Flat,
///     max_rank: 30,
///     is_trainable: true,
/// };
///
/// assert_eq!(rank_for_level(&diplomacy, 1), 0);
/// assert_eq!(rank_for_level(&diplomacy, 20), 0);
/// ```
pub fn rank_for_level(definition: &SkillDefinition, level: u16) -> SkillRank {
    // Saturating subtraction makes level 0 behave like level 1.
    let level_index = level.saturating_sub(1) as u32;

    let raw: u32 = match &definition.scaling {
        SkillScalingMode::Flat => 0,

        SkillScalingMode::Linear { base, per_level } => {
            *base as u32 + *per_level as u32 * level_index
        }

        SkillScalingMode::Step {
            base,
            per_levels,
            amount,
        } => {
            // Guard against per_levels == 0 (validate() catches this in production,
            // but we must not panic in rank_for_level).
            let steps = if *per_levels > 0 {
                level_index / *per_levels as u32
            } else {
                0
            };
            *base as u32 + steps * *amount as u32
        }

        SkillScalingMode::Table { ranks_by_level } => {
            if ranks_by_level.is_empty() {
                0
            } else {
                let idx = (level_index as usize).min(ranks_by_level.len() - 1);
                ranks_by_level[idx] as u32
            }
        }
    };

    // Clamp to max_rank.
    raw.min(definition.max_rank as u32) as SkillRank
}

/// Computes a character's auto-scaled rank and then applies a signed flat `bonus`.
///
/// The final value is clamped to `0..=definition.max_rank` after the bonus is applied.
/// A negative bonus can reduce the rank below the auto-scaled value, but never below 0.
///
/// # Arguments
///
/// * `definition` — The skill definition containing the scaling rule and max cap.
/// * `level` — The character's current level (1-based).
/// * `bonus` — Signed flat adjustment (positive or negative).
///
/// # Examples
///
/// ```
/// use antares::domain::skills::{SkillDefinition, SkillCategory, SkillScalingMode, rank_for_level_with_bonus};
///
/// let skill = SkillDefinition {
///     id: "perception".to_string(),
///     name: "Perception".to_string(),
///     category: SkillCategory::Exploration,
///     description: String::new(),
///     scaling: SkillScalingMode::Linear { base: 0, per_level: 1 },
///     max_rank: 50,
///     is_trainable: true,
/// };
///
/// // Level 10 auto-rank is 9; +5 bonus gives 14.
/// assert_eq!(rank_for_level_with_bonus(&skill, 10, 5), 14);
///
/// // Large negative bonus clamps to 0.
/// assert_eq!(rank_for_level_with_bonus(&skill, 1, -100), 0);
///
/// // Large positive bonus clamps to max_rank (50).
/// assert_eq!(rank_for_level_with_bonus(&skill, 1, 1000), 50);
/// ```
pub fn rank_for_level_with_bonus(
    definition: &SkillDefinition,
    level: u16,
    bonus: i32,
) -> SkillRank {
    let auto = rank_for_level(definition, level) as i32;
    let with_bonus = auto + bonus;
    with_bonus.clamp(0, definition.max_rank as i32) as SkillRank
}

/// Validates that `id` is a well-formed `lowercase_snake_case` skill identifier.
///
/// Rules enforced:
/// - Must not be empty.
/// - Must start with a lowercase ASCII letter (`a–z`).
/// - All remaining characters must be lowercase ASCII letters, digits, or `_`.
///
/// # Errors
///
/// Returns [`SkillError::ValidationError`] if any rule is violated.
///
/// # Examples
///
/// ```
/// use antares::domain::skills::validate_skill_id;
///
/// assert!(validate_skill_id("disarm_traps").is_ok());
/// assert!(validate_skill_id("item_lore2").is_ok());
/// assert!(validate_skill_id("").is_err());
/// assert!(validate_skill_id("DisarmTraps").is_err());
/// assert!(validate_skill_id("disarm-traps").is_err());
/// assert!(validate_skill_id("_private").is_err());
/// assert!(validate_skill_id("2fast").is_err());
/// ```
pub fn validate_skill_id(id: &str) -> Result<(), SkillError> {
    if id.is_empty() {
        return Err(SkillError::ValidationError(
            "Skill ID cannot be empty".to_string(),
        ));
    }

    let mut chars = id.chars();

    // First character must be lowercase a–z.
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => {
            return Err(SkillError::ValidationError(format!(
                "Skill ID '{}' must start with a lowercase ASCII letter (a–z)",
                id
            )));
        }
    }

    // Remaining characters: a–z, 0–9, or _.
    for c in chars {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '_' {
            return Err(SkillError::ValidationError(format!(
                "Skill ID '{}' contains invalid character '{}'; \
                 IDs must be lowercase snake_case (a–z, 0–9, _)",
                id, c
            )));
        }
    }

    Ok(())
}

/// Validates that `rank` does not exceed `max_rank`.
///
/// # Errors
///
/// Returns [`SkillError::ValidationError`] if `rank > max_rank`.
///
/// # Examples
///
/// ```
/// use antares::domain::skills::validate_skill_rank;
///
/// assert!(validate_skill_rank(5, 50).is_ok());
/// assert!(validate_skill_rank(50, 50).is_ok());
/// assert!(validate_skill_rank(51, 50).is_err());
/// ```
pub fn validate_skill_rank(rank: SkillRank, max_rank: SkillRank) -> Result<(), SkillError> {
    if rank > max_rank {
        return Err(SkillError::ValidationError(format!(
            "Skill rank {} exceeds maximum rank {}",
            rank, max_rank
        )));
    }
    Ok(())
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper builders ─────────────────────────────────────────────────────

    fn make_linear_skill(
        id: &str,
        base: SkillRank,
        per_level: u16,
        max_rank: SkillRank,
    ) -> SkillDefinition {
        SkillDefinition {
            id: id.to_string(),
            name: id.to_string(),
            category: SkillCategory::Exploration,
            description: String::new(),
            scaling: SkillScalingMode::Linear { base, per_level },
            max_rank,
            is_trainable: true,
        }
    }

    fn make_step_skill(
        id: &str,
        base: SkillRank,
        per_levels: u16,
        amount: u16,
        max_rank: SkillRank,
    ) -> SkillDefinition {
        SkillDefinition {
            id: id.to_string(),
            name: id.to_string(),
            category: SkillCategory::Exploration,
            description: String::new(),
            scaling: SkillScalingMode::Step {
                base,
                per_levels,
                amount,
            },
            max_rank,
            is_trainable: true,
        }
    }

    fn make_flat_skill(id: &str, max_rank: SkillRank) -> SkillDefinition {
        SkillDefinition {
            id: id.to_string(),
            name: id.to_string(),
            category: SkillCategory::Social,
            description: String::new(),
            scaling: SkillScalingMode::Flat,
            max_rank,
            is_trainable: true,
        }
    }

    fn make_table_skill(id: &str, ranks: Vec<SkillRank>, max_rank: SkillRank) -> SkillDefinition {
        SkillDefinition {
            id: id.to_string(),
            name: id.to_string(),
            category: SkillCategory::Knowledge,
            description: String::new(),
            scaling: SkillScalingMode::Table {
                ranks_by_level: ranks,
            },
            max_rank,
            is_trainable: true,
        }
    }

    // ── validate_skill_id ───────────────────────────────────────────────────

    #[test]
    fn test_skill_definition_validate_rejects_empty_id() {
        let result = validate_skill_id("");
        assert!(result.is_err(), "Empty ID must fail validation");
        match result.unwrap_err() {
            SkillError::ValidationError(msg) => {
                assert!(
                    msg.contains("empty"),
                    "Error message should mention 'empty': {}",
                    msg
                );
            }
            other => panic!("Expected ValidationError, got {:?}", other),
        }
    }

    #[test]
    fn test_skill_definition_validate_rejects_non_snake_case_id() {
        // CamelCase
        assert!(
            validate_skill_id("CamelCase").is_err(),
            "CamelCase must fail"
        );
        // kebab-case
        assert!(
            validate_skill_id("kebab-case").is_err(),
            "kebab-case must fail"
        );
        // spaces
        assert!(
            validate_skill_id("has space").is_err(),
            "space in ID must fail"
        );
        // starts with underscore
        assert!(
            validate_skill_id("_private").is_err(),
            "leading _ must fail"
        );
        // starts with digit
        assert!(
            validate_skill_id("2fast").is_err(),
            "leading digit must fail"
        );
        // uppercase in middle
        assert!(
            validate_skill_id("disarm_Traps").is_err(),
            "uppercase in middle must fail"
        );
    }

    #[test]
    fn test_validate_skill_id_accepts_valid_ids() {
        assert!(validate_skill_id("perception").is_ok());
        assert!(validate_skill_id("disarm_traps").is_ok());
        assert!(validate_skill_id("item_lore2").is_ok());
        assert!(validate_skill_id("a").is_ok());
        assert!(validate_skill_id("skill_with_many_underscores_123").is_ok());
    }

    // ── validate_skill_rank ─────────────────────────────────────────────────

    #[test]
    fn test_validate_skill_rank_accepts_rank_at_cap() {
        assert!(validate_skill_rank(50, 50).is_ok());
    }

    #[test]
    fn test_validate_skill_rank_accepts_zero() {
        assert!(validate_skill_rank(0, 50).is_ok());
    }

    #[test]
    fn test_validate_skill_rank_rejects_rank_above_cap() {
        let result = validate_skill_rank(51, 50);
        assert!(result.is_err());
    }

    // ── rank_for_level — Flat ───────────────────────────────────────────────

    #[test]
    fn test_rank_for_level_flat_returns_base_rank() {
        let skill = make_flat_skill("diplomacy", 30);
        // Flat always returns 0 for auto-scaling regardless of level.
        assert_eq!(rank_for_level(&skill, 1), 0);
        assert_eq!(rank_for_level(&skill, 10), 0);
        assert_eq!(rank_for_level(&skill, 20), 0);
    }

    // ── rank_for_level — Linear ─────────────────────────────────────────────

    #[test]
    fn test_rank_for_level_linear_scales_by_level() {
        // perception: Linear(base: 0, per_level: 1, max_rank: 50)
        let skill = make_linear_skill("perception", 0, 1, 50);

        assert_eq!(rank_for_level(&skill, 1), 0, "level 1 = base = 0");
        assert_eq!(rank_for_level(&skill, 2), 1, "level 2 = 0 + 1*1 = 1");
        assert_eq!(rank_for_level(&skill, 5), 4, "level 5 = 0 + 1*4 = 4");
        assert_eq!(rank_for_level(&skill, 10), 9, "level 10 = 0 + 1*9 = 9");
    }

    #[test]
    fn test_rank_for_level_linear_with_nonzero_base() {
        let skill = make_linear_skill("strong_start", 5, 2, 100);
        assert_eq!(rank_for_level(&skill, 1), 5, "level 1 = base = 5");
        assert_eq!(rank_for_level(&skill, 2), 7, "level 2 = 5 + 2*1 = 7");
        assert_eq!(rank_for_level(&skill, 10), 23, "level 10 = 5 + 2*9 = 23");
    }

    // ── rank_for_level — Step ───────────────────────────────────────────────

    #[test]
    fn test_rank_for_level_step_scales_at_interval() {
        // disarm_traps: Step(base: 0, per_levels: 2, amount: 1)
        let skill = make_step_skill("disarm_traps", 0, 2, 1, 25);

        assert_eq!(rank_for_level(&skill, 1), 0, "level 1: 0 + (0/2)*1 = 0");
        assert_eq!(rank_for_level(&skill, 2), 0, "level 2: 0 + (1/2)*1 = 0");
        assert_eq!(rank_for_level(&skill, 3), 1, "level 3: 0 + (2/2)*1 = 1");
        assert_eq!(rank_for_level(&skill, 4), 1, "level 4: 0 + (3/2)*1 = 1");
        assert_eq!(rank_for_level(&skill, 5), 2, "level 5: 0 + (4/2)*1 = 2");
        assert_eq!(rank_for_level(&skill, 11), 5, "level 11: 0 + (10/2)*1 = 5");
    }

    #[test]
    fn test_rank_for_level_step_with_base() {
        // athletics: Step(base: 1, per_levels: 3, amount: 1)
        let skill = make_step_skill("athletics", 1, 3, 1, 20);

        assert_eq!(rank_for_level(&skill, 1), 1, "level 1: 1 + (0/3)*1 = 1");
        assert_eq!(rank_for_level(&skill, 3), 1, "level 3: 1 + (2/3)*1 = 1");
        assert_eq!(rank_for_level(&skill, 4), 2, "level 4: 1 + (3/3)*1 = 2");
        assert_eq!(rank_for_level(&skill, 7), 3, "level 7: 1 + (6/3)*1 = 3");
        assert_eq!(rank_for_level(&skill, 10), 4, "level 10: 1 + (9/3)*1 = 4");
    }

    // ── rank_for_level — Table ──────────────────────────────────────────────

    #[test]
    fn test_rank_for_level_table_clamps_after_last_entry() {
        // Table with 3 entries: level 1→0, level 2→3, level 3→7.
        // Levels beyond 3 should all return 7.
        let skill = make_table_skill("table_skill", vec![0, 3, 7], 50);

        assert_eq!(rank_for_level(&skill, 1), 0, "level 1 → table[0] = 0");
        assert_eq!(rank_for_level(&skill, 2), 3, "level 2 → table[1] = 3");
        assert_eq!(rank_for_level(&skill, 3), 7, "level 3 → table[2] = 7");
        assert_eq!(
            rank_for_level(&skill, 4),
            7,
            "level 4 clamps to table[2] = 7"
        );
        assert_eq!(
            rank_for_level(&skill, 100),
            7,
            "level 100 clamps to table[2] = 7"
        );
    }

    // ── rank_for_level — max_rank clamping ──────────────────────────────────

    #[test]
    fn test_rank_for_level_clamps_to_max_rank() {
        // Linear skill with small max_rank that gets exceeded quickly.
        let skill = make_linear_skill("capped_skill", 0, 5, 10);

        // Level 1: 0 + 5*0 = 0 (under cap)
        assert_eq!(rank_for_level(&skill, 1), 0);
        // Level 3: 0 + 5*2 = 10 (at cap)
        assert_eq!(rank_for_level(&skill, 3), 10);
        // Level 10: 0 + 5*9 = 45, but clamped to 10
        assert_eq!(rank_for_level(&skill, 10), 10);
    }

    // ── rank_for_level_with_bonus ───────────────────────────────────────────

    #[test]
    fn test_rank_for_level_with_bonus_applies_positive_bonus() {
        let skill = make_linear_skill("perception", 0, 1, 50);
        // Level 10 auto = 9; +5 → 14
        assert_eq!(rank_for_level_with_bonus(&skill, 10, 5), 14);
    }

    #[test]
    fn test_rank_for_level_with_bonus_clamps_negative_to_zero() {
        let skill = make_flat_skill("diplomacy", 30);
        // Auto = 0; large negative bonus clamps to 0
        assert_eq!(rank_for_level_with_bonus(&skill, 5, -999), 0);
    }

    #[test]
    fn test_rank_for_level_with_bonus_clamps_to_max_rank() {
        let skill = make_linear_skill("perception", 0, 1, 50);
        // Large positive bonus clamps to max_rank (50)
        assert_eq!(rank_for_level_with_bonus(&skill, 1, 10_000), 50);
    }

    // ── level 0 safety ──────────────────────────────────────────────────────

    #[test]
    fn test_rank_for_level_level_zero_treated_as_level_one() {
        let skill = make_linear_skill("perception", 0, 1, 50);
        // level 0 saturates to level 1, index 0 → base = 0
        assert_eq!(rank_for_level(&skill, 0), 0);
    }

    // ── SkillDefinition::validate ───────────────────────────────────────────

    #[test]
    fn test_skill_definition_validate_rejects_zero_max_rank() {
        let mut skill = make_flat_skill("diplomacy", 30);
        skill.max_rank = 0;
        assert!(skill.validate().is_err());
    }

    #[test]
    fn test_skill_definition_validate_rejects_empty_name() {
        let mut skill = make_flat_skill("diplomacy", 30);
        skill.name = String::new();
        assert!(skill.validate().is_err());
    }

    #[test]
    fn test_skill_definition_validate_rejects_step_per_levels_zero() {
        let skill = SkillDefinition {
            id: "bad_step".to_string(),
            name: "Bad Step".to_string(),
            category: SkillCategory::Utility,
            description: String::new(),
            scaling: SkillScalingMode::Step {
                base: 0,
                per_levels: 0,
                amount: 1,
            },
            max_rank: 50,
            is_trainable: false,
        };
        let result = skill.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            SkillError::ValidationError(msg) => {
                assert!(msg.contains("per_levels"), "{}", msg);
            }
            other => panic!("Expected ValidationError, got {:?}", other),
        }
    }

    #[test]
    fn test_skill_definition_validate_rejects_empty_table() {
        let skill = SkillDefinition {
            id: "bad_table".to_string(),
            name: "Bad Table".to_string(),
            category: SkillCategory::Knowledge,
            description: String::new(),
            scaling: SkillScalingMode::Table {
                ranks_by_level: vec![],
            },
            max_rank: 50,
            is_trainable: false,
        };
        assert!(skill.validate().is_err());
    }

    #[test]
    fn test_skill_definition_validate_rejects_table_rank_exceeds_max() {
        let skill = SkillDefinition {
            id: "over_max".to_string(),
            name: "Over Max".to_string(),
            category: SkillCategory::Knowledge,
            description: String::new(),
            scaling: SkillScalingMode::Table {
                ranks_by_level: vec![0, 5, 100],
            },
            max_rank: 50,
            is_trainable: false,
        };
        assert!(skill.validate().is_err());
    }

    // ── SkillDatabase ───────────────────────────────────────────────────────

    #[test]
    fn test_skill_database_rejects_duplicate_id() {
        let ron = r#"[
            (id: "perception", name: "Perception", category: Exploration,
             description: "", scaling: Flat, max_rank: 50, is_trainable: true),
            (id: "perception", name: "Duplicate", category: Combat,
             description: "", scaling: Flat, max_rank: 50, is_trainable: false),
        ]"#;

        let result = SkillDatabase::load_from_string(ron);
        assert!(result.is_err(), "Duplicate IDs must be rejected");
        match result.unwrap_err() {
            SkillError::DuplicateId(id) => assert_eq!(id, "perception"),
            other => panic!("Expected DuplicateId, got {:?}", other),
        }
    }

    #[test]
    fn test_skill_database_load_from_string_succeeds() {
        let ron = r#"[
            (id: "perception", name: "Perception", category: Exploration,
             description: "Spot hidden threats.", scaling: Linear(base: 0, per_level: 1),
             max_rank: 50, is_trainable: true),
            (id: "diplomacy", name: "Diplomacy", category: Social,
             description: "", scaling: Flat, max_rank: 30, is_trainable: true),
        ]"#;

        let db = SkillDatabase::load_from_string(ron).unwrap();
        assert_eq!(db.len(), 2);
        assert!(db.has("perception"));
        assert!(db.has("diplomacy"));
        assert!(!db.has("nonexistent"));
    }

    #[test]
    fn test_skill_database_by_category_filters_correctly() {
        let ron = r#"[
            (id: "perception", name: "Perception", category: Exploration,
             description: "", scaling: Flat, max_rank: 50, is_trainable: true),
            (id: "diplomacy", name: "Diplomacy", category: Social,
             description: "", scaling: Flat, max_rank: 30, is_trainable: true),
            (id: "disarm_traps", name: "Disarm Traps", category: Exploration,
             description: "", scaling: Flat, max_rank: 25, is_trainable: true),
        ]"#;

        let db = SkillDatabase::load_from_string(ron).unwrap();
        let exploration = db.by_category(SkillCategory::Exploration);
        assert_eq!(exploration.len(), 2);

        let social = db.by_category(SkillCategory::Social);
        assert_eq!(social.len(), 1);
        assert_eq!(social[0].id, "diplomacy");

        let combat = db.by_category(SkillCategory::Combat);
        assert!(combat.is_empty());
    }

    #[test]
    fn test_skill_database_validate_detects_invalid_step() {
        let ron = r#"[
            (id: "bad_step", name: "Bad Step", category: Utility,
             description: "", scaling: Step(base: 0, per_levels: 0, amount: 1),
             max_rank: 50, is_trainable: false),
        ]"#;
        // load_from_string succeeds (RON parses fine), but validate() fails
        let db = SkillDatabase::load_from_string(ron).unwrap();
        assert!(db.validate().is_err());
    }

    #[test]
    fn test_skill_database_validate_passes_for_valid_database() {
        let ron = r#"[
            (id: "perception", name: "Perception", category: Exploration,
             description: "Spot hidden threats.", scaling: Linear(base: 0, per_level: 1),
             max_rank: 50, is_trainable: true),
            (id: "athletics", name: "Athletics", category: Utility,
             description: "Physical fitness.", scaling: Step(base: 1, per_levels: 3, amount: 1),
             max_rank: 20, is_trainable: true),
        ]"#;
        let db = SkillDatabase::load_from_string(ron).unwrap();
        assert!(db.validate().is_ok());
    }

    #[test]
    fn test_skill_database_add_rejects_duplicate() {
        let mut db = SkillDatabase::new();
        let skill = make_flat_skill("perception", 50);
        db.add(skill.clone()).unwrap();
        assert!(db.add(skill).is_err());
    }

    #[test]
    fn test_skill_database_remove_returns_definition() {
        let mut db = SkillDatabase::new();
        db.add(make_flat_skill("perception", 50)).unwrap();
        let removed = db.remove("perception");
        assert!(removed.is_some());
        assert!(db.is_empty());
    }

    #[test]
    fn test_skill_database_empty_string_parse_error() {
        let result = SkillDatabase::load_from_string("not valid ron }}}");
        assert!(result.is_err());
        match result.unwrap_err() {
            SkillError::ParseError(_) => {}
            other => panic!("Expected ParseError, got {:?}", other),
        }
    }

    // ── Load from test campaign fixture ────────────────────────────────────
    // RULE: All test fixture paths use data/test_campaign, never the live campaign.

    #[test]
    fn test_skill_database_loads_test_campaign_fixture() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = std::path::Path::new(manifest_dir).join("data/test_campaign/data/skills.ron");

        let db = SkillDatabase::load_from_file(&path)
            .expect("data/test_campaign/data/skills.ron must load without error");

        assert!(
            !db.is_empty(),
            "Test campaign fixture must contain at least one skill"
        );

        // Verify the five canonical Phase 1 skills are present.
        assert!(db.has("perception"), "perception must be in test fixture");
        assert!(
            db.has("disarm_traps"),
            "disarm_traps must be in test fixture"
        );
        assert!(db.has("item_lore"), "item_lore must be in test fixture");
        assert!(db.has("diplomacy"), "diplomacy must be in test fixture");
        assert!(db.has("athletics"), "athletics must be in test fixture");

        // Validate all skills in the fixture.
        db.validate()
            .expect("All test campaign skills must be structurally valid");
    }

    // ── SkillCategory ───────────────────────────────────────────────────────

    #[test]
    fn test_skill_category_default_is_exploration() {
        assert_eq!(SkillCategory::default(), SkillCategory::Exploration);
    }

    #[test]
    fn test_skill_category_equality() {
        assert_eq!(SkillCategory::Combat, SkillCategory::Combat);
        assert_ne!(SkillCategory::Social, SkillCategory::Utility);
    }

    // ── CharacterSkillRanks tests ────────────────────────────────────────────

    #[test]
    fn test_character_skill_ranks_new_is_empty() {
        let ranks = CharacterSkillRanks::new();
        assert!(!ranks.contains(&"perception".to_string()));
        assert_eq!(ranks.get(&"perception".to_string()), None);
    }

    #[test]
    fn test_character_skill_ranks_set_and_get() {
        let mut ranks = CharacterSkillRanks::new();
        ranks.set("perception".to_string(), 5);
        assert_eq!(ranks.get(&"perception".to_string()), Some(5));
    }

    #[test]
    fn test_character_skill_ranks_set_overwrites() {
        let mut ranks = CharacterSkillRanks::new();
        ranks.set("perception".to_string(), 3);
        ranks.set("perception".to_string(), 7);
        assert_eq!(ranks.get(&"perception".to_string()), Some(7));
    }

    #[test]
    fn test_character_skill_ranks_increment_from_zero() {
        let mut ranks = CharacterSkillRanks::new();
        ranks.increment(&"perception".to_string());
        assert_eq!(ranks.get(&"perception".to_string()), Some(1));
    }

    #[test]
    fn test_character_skill_ranks_increment_existing() {
        let mut ranks = CharacterSkillRanks::new();
        ranks.set("perception".to_string(), 4);
        ranks.increment(&"perception".to_string());
        assert_eq!(ranks.get(&"perception".to_string()), Some(5));
    }

    #[test]
    fn test_character_skill_ranks_remove() {
        let mut ranks = CharacterSkillRanks::new();
        ranks.set("perception".to_string(), 5);
        ranks.remove(&"perception".to_string());
        assert!(!ranks.contains(&"perception".to_string()));
        assert_eq!(ranks.get(&"perception".to_string()), None);
    }

    #[test]
    fn test_character_skill_ranks_remove_missing_is_noop() {
        let mut ranks = CharacterSkillRanks::new();
        ranks.remove(&"nonexistent".to_string()); // no panic
    }

    #[test]
    fn test_character_skill_ranks_contains() {
        let mut ranks = CharacterSkillRanks::new();
        assert!(!ranks.contains(&"athletics".to_string()));
        ranks.set("athletics".to_string(), 2);
        assert!(ranks.contains(&"athletics".to_string()));
    }

    #[test]
    fn test_character_skill_ranks_default_is_empty() {
        let ranks = CharacterSkillRanks::default();
        assert!(!ranks.contains(&"any".to_string()));
    }

    #[test]
    fn test_character_skill_ranks_serde_roundtrip() {
        let mut ranks = CharacterSkillRanks::new();
        ranks.set("perception".to_string(), 5);
        ranks.set("athletics".to_string(), 3);
        // Serialize via ron then deserialize back
        let ron_str = ron::to_string(&ranks).expect("serialization failed");
        let restored: CharacterSkillRanks =
            ron::from_str(&ron_str).expect("deserialization failed");
        assert_eq!(restored.get(&"perception".to_string()), Some(5));
        assert_eq!(restored.get(&"athletics".to_string()), Some(3));
    }
}
