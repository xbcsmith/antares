// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Magic system types - Spells, schools, contexts, and targets
//!
//! This module defines the core types for the magic system including spell
//! definitions, casting contexts, target types, and spell schools.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 5.3 for complete specifications.

use crate::domain::types::SpellId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur during spell casting
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SpellError {
    #[error("Not enough spell points (need {needed}, have {available})")]
    NotEnoughSP { needed: u16, available: u16 },

    #[error("Not enough gems (need {needed}, have {available})")]
    NotEnoughGems { needed: u32, available: u32 },

    #[error("Character class {0:?} cannot cast {1:?} spells")]
    WrongClass(String, SpellSchool),

    #[error("Character level {level} is too low (need level {required})")]
    LevelTooLow { level: u32, required: u32 },

    #[error("Spell can only be cast in combat")]
    CombatOnly,

    #[error("Spell cannot be cast in combat")]
    NonCombatOnly,

    #[error("Spell can only be cast outdoors")]
    OutdoorsOnly,

    #[error("Spell can only be cast indoors")]
    IndoorsOnly,

    #[error("Magic is forbidden in this area")]
    MagicForbidden,

    #[error("Character is silenced and cannot cast spells")]
    Silenced,

    #[error("Character is unconscious and cannot cast spells")]
    Unconscious,

    #[error("Spell not found: {0}")]
    SpellNotFound(SpellId),

    #[error("Invalid target for this spell")]
    InvalidTarget,
}

// ===== Spell School =====

/// Spell school identifier - two separate magic systems
///
/// In Might and Magic 1, there are two completely separate spell schools:
/// - Cleric spells (divine magic)
/// - Sorcerer spells (arcane magic)
///
/// # Examples
///
/// ```
/// use antares::domain::magic::types::SpellSchool;
///
/// let divine = SpellSchool::Cleric;
/// let arcane = SpellSchool::Sorcerer;
/// assert_ne!(divine, arcane);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellSchool {
    /// Divine magic - healing, protection, support, turn undead
    Cleric,
    /// Arcane magic - offense, debuffs, utility, transformation
    Sorcerer,
}

// ===== Spell Context =====

/// Spell casting context restrictions
///
/// Defines when and where a spell can be cast based on game state.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::types::SpellContext;
///
/// let combat_spell = SpellContext::CombatOnly;
/// let utility_spell = SpellContext::Anytime;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellContext {
    /// Can cast in or out of combat
    Anytime,
    /// Only during combat
    CombatOnly,
    /// Only outside combat
    NonCombatOnly,
    /// Only in outdoor areas
    OutdoorOnly,
    /// Only in indoor areas
    IndoorOnly,
    /// Combat in outdoor areas only
    OutdoorCombat,
}

// ===== Spell Target =====

/// Spell target type
///
/// Defines what entities a spell can target.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::types::SpellTarget;
///
/// let healing = SpellTarget::SingleCharacter;
/// let fireball = SpellTarget::AllMonsters;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellTarget {
    /// Caster only
    Self_,
    /// One party member
    SingleCharacter,
    /// Entire party
    AllCharacters,
    /// One enemy
    SingleMonster,
    /// Multiple enemies (up to N)
    MonsterGroup,
    /// All enemies
    AllMonsters,
    /// Subset based on type (e.g., undead)
    SpecificMonsters,
}

// ===== Spell Definition =====

/// Complete spell definition
///
/// Contains all static information about a spell including costs,
/// restrictions, and description.
///
/// # Architecture Compliance
///
/// This struct matches the architecture specification in Section 5.3.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::types::{Spell, SpellSchool, SpellContext, SpellTarget};
///
/// let cure_wounds = Spell {
///     id: 0x0101, // Cleric school, spell 1
///     name: "Cure Wounds".to_string(),
///     school: SpellSchool::Cleric,
///     level: 1,
///     sp_cost: 2,
///     gem_cost: 0,
///     context: SpellContext::Anytime,
///     target: SpellTarget::SingleCharacter,
///     description: "Heals 8 hit points".to_string(),
///     damage: None,
///     duration: 0,
///     saving_throw: false,
/// };
///
/// assert_eq!(cure_wounds.level, 1);
/// assert_eq!(cure_wounds.sp_cost, 2);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spell {
    /// Spell identifier (high byte = school, low byte = spell number)
    pub id: SpellId,
    /// Display name
    pub name: String,
    /// Spell school (Cleric or Sorcerer)
    pub school: SpellSchool,
    /// Spell level (1-7)
    pub level: u8,
    /// Base spell point cost
    pub sp_cost: u16,
    /// Gem cost (0 if none)
    pub gem_cost: u16,
    /// When/where the spell can be cast
    pub context: SpellContext,
    /// Who/what the spell affects
    pub target: SpellTarget,
    /// Human-readable description of effects
    pub description: String,
    /// Damage dice (if applicable)
    pub damage: Option<crate::domain::types::DiceRoll>,
    /// Duration in rounds (0 = Instant)
    pub duration: u16,
    /// Whether a saving throw is allowed
    pub saving_throw: bool,
}

impl Spell {
    /// Creates a new spell definition
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::magic::types::{Spell, SpellSchool, SpellContext, SpellTarget};
    /// use antares::domain::types::DiceRoll;
    ///
    /// let spell = Spell::new(
    ///     0x0201,
    ///     "Fireball",
    ///     SpellSchool::Sorcerer,
    ///     3,
    ///     5,
    ///     2,
    ///     SpellContext::CombatOnly,
    ///     SpellTarget::MonsterGroup,
    ///     "Deals 3d6 fire damage to multiple enemies",
    ///     Some(DiceRoll::new(3, 6, 0)),
    ///     0,
    ///     true,
    /// );
    ///
    /// assert_eq!(spell.name, "Fireball");
    /// assert_eq!(spell.level, 3);
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: SpellId,
        name: impl Into<String>,
        school: SpellSchool,
        level: u8,
        sp_cost: u16,
        gem_cost: u16,
        context: SpellContext,
        target: SpellTarget,
        description: impl Into<String>,
        damage: Option<crate::domain::types::DiceRoll>,
        duration: u16,
        saving_throw: bool,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            school,
            level,
            sp_cost,
            gem_cost,
            context,
            target,
            description: description.into(),
            damage,
            duration,
            saving_throw,
        }
    }

    /// Returns the minimum character level required to cast this spell
    ///
    /// Spell access by level:
    /// - Level 1: Can cast level 1 spells
    /// - Level 3: Can cast level 2 spells
    /// - Level 5: Can cast level 3 spells
    /// - Level 7: Can cast level 4 spells
    /// - Level 9: Can cast level 5 spells
    /// - Level 11: Can cast level 6 spells
    /// - Level 13+: Can cast level 7 spells
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::magic::types::{Spell, SpellSchool, SpellContext, SpellTarget};
    ///
    /// let spell = Spell::new(
    ///     0x0103,
    ///     "Raise Dead",
    ///     SpellSchool::Cleric,
    ///     5,
    ///     15,
    ///     10,
    ///     SpellContext::Anytime,
    ///     SpellTarget::SingleCharacter,
    ///     "Resurrects a dead character",
    ///     None,
    ///     0,
    ///     false,
    /// );
    ///
    /// assert_eq!(spell.required_level(), 9);
    /// ```
    pub fn required_level(&self) -> u32 {
        match self.level {
            1 => 1,
            2 => 3,
            3 => 5,
            4 => 7,
            5 => 9,
            6 => 11,
            7 => 13,
            _ => 1,
        }
    }

    /// Returns true if the spell is a combat-only spell
    pub fn is_combat_only(&self) -> bool {
        matches!(
            self.context,
            SpellContext::CombatOnly | SpellContext::OutdoorCombat
        )
    }

    /// Returns true if the spell is a non-combat-only spell
    pub fn is_non_combat_only(&self) -> bool {
        self.context == SpellContext::NonCombatOnly
    }

    /// Returns true if the spell has a gem cost
    pub fn requires_gems(&self) -> bool {
        self.gem_cost > 0
    }
}

// ===== Spell Result =====

/// Result of casting a spell
///
/// Contains information about the spell's effects and which targets
/// were affected.
///
/// # Examples
///
/// ```
/// use antares::domain::magic::types::SpellResult;
///
/// let result = SpellResult {
///     success: true,
///     effect_message: "The party is healed!".to_string(),
///     damage: None,
///     healing: Some(20),
///     affected_targets: vec![0, 1, 2],
/// };
///
/// assert!(result.success);
/// assert_eq!(result.healing, Some(20));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellResult {
    /// Whether the spell succeeded
    pub success: bool,
    /// Human-readable effect message
    pub effect_message: String,
    /// Damage dealt (if applicable)
    pub damage: Option<i32>,
    /// Healing done (if applicable)
    pub healing: Option<i32>,
    /// Indices of affected targets (party members or monsters)
    pub affected_targets: Vec<usize>,
}

impl SpellResult {
    /// Creates a successful spell result
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            effect_message: message.into(),
            damage: None,
            healing: None,
            affected_targets: Vec::new(),
        }
    }

    /// Creates a failed spell result
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            effect_message: message.into(),
            damage: None,
            healing: None,
            affected_targets: Vec::new(),
        }
    }

    /// Creates a damage spell result
    pub fn with_damage(mut self, damage: i32, targets: Vec<usize>) -> Self {
        self.damage = Some(damage);
        self.affected_targets = targets;
        self
    }

    /// Creates a healing spell result
    pub fn with_healing(mut self, healing: i32, targets: Vec<usize>) -> Self {
        self.healing = Some(healing);
        self.affected_targets = targets;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spell_creation() {
        let spell = Spell::new(
            0x0101,
            "Cure Wounds",
            SpellSchool::Cleric,
            1,
            2,
            0,
            SpellContext::Anytime,
            SpellTarget::SingleCharacter,
            "Heals 8 hit points",
            None,
            0,
            false,
        );

        assert_eq!(spell.name, "Cure Wounds");
        assert_eq!(spell.school, SpellSchool::Cleric);
        assert_eq!(spell.level, 1);
        assert_eq!(spell.sp_cost, 2);
        assert_eq!(spell.gem_cost, 0);
    }

    #[test]
    fn test_required_level() {
        let level1 = Spell::new(
            1,
            "Test",
            SpellSchool::Cleric,
            1,
            1,
            0,
            SpellContext::Anytime,
            SpellTarget::Self_,
            "Test",
            None,
            0,
            false,
        );
        assert_eq!(level1.required_level(), 1);

        let level3 = Spell::new(
            2,
            "Test",
            SpellSchool::Cleric,
            3,
            3,
            0,
            SpellContext::Anytime,
            SpellTarget::Self_,
            "Test",
            None,
            0,
            false,
        );
        assert_eq!(level3.required_level(), 5);

        let level7 = Spell::new(
            3,
            "Test",
            SpellSchool::Sorcerer,
            7,
            10,
            5,
            SpellContext::Anytime,
            SpellTarget::Self_,
            "Test",
            None,
            0,
            false,
        );
        assert_eq!(level7.required_level(), 13);
    }

    #[test]
    fn test_spell_context_checks() {
        let combat_spell = Spell::new(
            1,
            "Attack",
            SpellSchool::Sorcerer,
            1,
            2,
            0,
            SpellContext::CombatOnly,
            SpellTarget::SingleMonster,
            "Attack spell",
            Some(crate::domain::types::DiceRoll::new(1, 6, 0)),
            0,
            true,
        );
        assert!(combat_spell.is_combat_only());
        assert!(!combat_spell.is_non_combat_only());

        let utility_spell = Spell::new(
            2,
            "Town Portal",
            SpellSchool::Cleric,
            3,
            5,
            2,
            SpellContext::NonCombatOnly,
            SpellTarget::Self_,
            "Teleport to town",
            None,
            0,
            false,
        );
        assert!(!utility_spell.is_combat_only());
        assert!(utility_spell.is_non_combat_only());
    }

    #[test]
    fn test_spell_requires_gems() {
        let free_spell = Spell::new(
            1,
            "Test",
            SpellSchool::Cleric,
            1,
            1,
            0,
            SpellContext::Anytime,
            SpellTarget::Self_,
            "Test",
            None,
            0,
            false,
        );
        assert!(!free_spell.requires_gems());

        let gem_spell = Spell::new(
            2,
            "Test",
            SpellSchool::Sorcerer,
            5,
            10,
            5,
            SpellContext::Anytime,
            SpellTarget::Self_,
            "Test",
            None,
            0,
            false,
        );
        assert!(gem_spell.requires_gems());
    }

    #[test]
    fn test_spell_result_success() {
        let result = SpellResult::success("Spell cast successfully");
        assert!(result.success);
        assert_eq!(result.effect_message, "Spell cast successfully");
        assert_eq!(result.damage, None);
        assert_eq!(result.healing, None);
    }

    #[test]
    fn test_spell_result_with_damage() {
        let result = SpellResult::success("Fireball!").with_damage(30, vec![0, 1, 2]);

        assert!(result.success);
        assert_eq!(result.damage, Some(30));
        assert_eq!(result.affected_targets, vec![0, 1, 2]);
    }

    #[test]
    fn test_spell_result_with_healing() {
        let result = SpellResult::success("Party healed!").with_healing(20, vec![0, 1]);

        assert!(result.success);
        assert_eq!(result.healing, Some(20));
        assert_eq!(result.affected_targets, vec![0, 1]);
    }
}
