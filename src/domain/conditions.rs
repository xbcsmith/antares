// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Condition system types
//!
//! This module defines the core types for the condition system, allowing for
//! data-driven status effects, buffs, and debuffs.

use crate::domain::types::DiceRoll;
use serde::{Deserialize, Serialize};

/// Unique identifier for a condition definition
pub type ConditionId = String;

/// Duration of a condition
///
/// Determines how and when an [`ActiveCondition`] expires.  The tick
/// methods on `ActiveCondition` inspect this variant to decide whether the
/// condition has run its course:
///
/// | Variant           | Expires when…                                     |
/// |-------------------|---------------------------------------------------|
/// | `Instant`         | Immediately after being applied (single tick)     |
/// | `Rounds(n)`       | `n` combat rounds have elapsed                   |
/// | `Minutes(n)`      | `n` exploration minutes have elapsed              |
/// | `UntilCombatEnd`  | The current combat ends (victory, defeat, flee)   |
/// | `UntilRest`       | The party takes a full rest                       |
/// | `Permanent`       | Never — must be explicitly removed                |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionDuration {
    /// Instantaneous effect (e.g., immediate damage/healing)
    Instant,
    /// Lasts for a specific number of combat rounds
    Rounds(u16),
    /// Lasts for a specific number of minutes (exploration turns)
    Minutes(u16),
    /// Expires when the current combat ends (victory, defeat, or flee).
    UntilCombatEnd,
    /// Expires when the party takes a full rest.
    UntilRest,
    /// Lasts until cured or removed
    Permanent,
}

/// Type of effect a condition applies
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionEffect {
    /// Modifies a primary attribute (Might, Intellect, etc.)
    AttributeModifier {
        attribute: String, // "might", "ac", "speed", etc.
        value: i16,
    },
    /// Applies a status flag (Blind, Sleep, Paralyzed)
    StatusEffect(String),
    /// Deals damage over time
    DamageOverTime {
        damage: DiceRoll,
        element: String, // "fire", "poison", etc.
    },
    /// Heals over time
    HealOverTime { amount: DiceRoll },
}

/// Definition of a condition (static data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionDefinition {
    /// Unique ID
    pub id: ConditionId,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Effects applied by this condition
    pub effects: Vec<ConditionEffect>,
    /// Default duration
    pub default_duration: ConditionDuration,
    /// Icon or visual indicator ID
    pub icon_id: Option<String>,
}

/// An active instance of a condition on an entity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveCondition {
    /// ID of the condition definition
    pub condition_id: ConditionId,
    /// Remaining duration
    pub duration: ConditionDuration,
    /// Magnitude multiplier (default 1.0)
    pub magnitude: f32,
}

impl ActiveCondition {
    /// Creates a new active condition with the given ID and duration.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::conditions::{ActiveCondition, ConditionDuration};
    ///
    /// let ac = ActiveCondition::new("bless".to_string(), ConditionDuration::Rounds(3));
    /// assert_eq!(ac.condition_id, "bless");
    /// assert_eq!(ac.magnitude, 1.0);
    /// ```
    pub fn new(condition_id: ConditionId, duration: ConditionDuration) -> Self {
        Self {
            condition_id,
            duration,
            magnitude: 1.0,
        }
    }

    /// Sets a custom magnitude multiplier for the condition.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::conditions::{ActiveCondition, ConditionDuration};
    ///
    /// let ac = ActiveCondition::new("poison".to_string(), ConditionDuration::Permanent)
    ///     .with_magnitude(2.0);
    /// assert_eq!(ac.magnitude, 2.0);
    /// ```
    pub fn with_magnitude(mut self, magnitude: f32) -> Self {
        self.magnitude = magnitude;
        self
    }

    /// Decrements duration for round-based conditions.
    ///
    /// Returns `true` if the condition has expired and should be removed.
    ///
    /// - [`ConditionDuration::Rounds`]: decrements the counter; expires at 0.
    /// - [`ConditionDuration::Instant`]: always expires.
    /// - [`ConditionDuration::UntilCombatEnd`], [`ConditionDuration::UntilRest`],
    ///   [`ConditionDuration::Minutes`], [`ConditionDuration::Permanent`]: not
    ///   expired by round ticks — use [`tick_combat_end`](Self::tick_combat_end)
    ///   or [`tick_rest`](Self::tick_rest) for the respective variants.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::conditions::{ActiveCondition, ConditionDuration};
    ///
    /// let mut ac = ActiveCondition::new("bless".to_string(), ConditionDuration::Rounds(1));
    /// assert!(ac.tick_round());  // expires after one round
    ///
    /// let mut ac2 = ActiveCondition::new("combat_bless".to_string(), ConditionDuration::UntilCombatEnd);
    /// assert!(!ac2.tick_round()); // not expired by rounds
    /// ```
    pub fn tick_round(&mut self) -> bool {
        match &mut self.duration {
            ConditionDuration::Rounds(rounds) => {
                if *rounds > 0 {
                    *rounds -= 1;
                }
                *rounds == 0
            }
            ConditionDuration::Instant => true,
            // UntilCombatEnd, UntilRest, Minutes, Permanent — not expired by rounds
            _ => false,
        }
    }

    /// Decrements duration for minute-based conditions (exploration ticks).
    ///
    /// Returns `true` if the condition has expired and should be removed.
    ///
    /// - [`ConditionDuration::Minutes`]: decrements the counter; expires at 0.
    /// - [`ConditionDuration::Rounds`]: expires immediately (rounds are
    ///   coarser than exploration minutes).
    /// - [`ConditionDuration::Instant`]: always expires.
    /// - [`ConditionDuration::UntilCombatEnd`], [`ConditionDuration::UntilRest`],
    ///   [`ConditionDuration::Permanent`]: not expired by minute ticks.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::conditions::{ActiveCondition, ConditionDuration};
    ///
    /// let mut ac = ActiveCondition::new("haste".to_string(), ConditionDuration::Minutes(1));
    /// assert!(ac.tick_minute());  // expires after one minute tick
    ///
    /// let mut ac2 = ActiveCondition::new("exhaustion".to_string(), ConditionDuration::UntilRest);
    /// assert!(!ac2.tick_minute()); // not expired by minutes
    /// ```
    pub fn tick_minute(&mut self) -> bool {
        match &mut self.duration {
            ConditionDuration::Minutes(minutes) => {
                if *minutes > 0 {
                    *minutes -= 1;
                }
                *minutes == 0
            }
            ConditionDuration::Rounds(_) => true, // Rounds expire fast in exploration
            ConditionDuration::Instant => true,
            // UntilCombatEnd, UntilRest, Permanent — not expired by minutes
            _ => false,
        }
    }

    /// Returns `true` if this condition should expire when combat ends.
    ///
    /// Only [`ConditionDuration::UntilCombatEnd`] returns `true`.  All other
    /// variants return `false` and are unaffected by the combat-end event.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::conditions::{ActiveCondition, ConditionDuration};
    ///
    /// let ac = ActiveCondition::new("combat_bless".to_string(), ConditionDuration::UntilCombatEnd);
    /// assert!(ac.tick_combat_end());
    ///
    /// let ac2 = ActiveCondition::new("poison".to_string(), ConditionDuration::Permanent);
    /// assert!(!ac2.tick_combat_end());
    /// ```
    pub fn tick_combat_end(&self) -> bool {
        matches!(self.duration, ConditionDuration::UntilCombatEnd)
    }

    /// Returns `true` if this condition should expire when the party rests.
    ///
    /// Only [`ConditionDuration::UntilRest`] returns `true`.  All other
    /// variants return `false` and are unaffected by the rest event.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::conditions::{ActiveCondition, ConditionDuration};
    ///
    /// let ac = ActiveCondition::new("exhaustion".to_string(), ConditionDuration::UntilRest);
    /// assert!(ac.tick_rest());
    ///
    /// let ac2 = ActiveCondition::new("poison".to_string(), ConditionDuration::Permanent);
    /// assert!(!ac2.tick_rest());
    /// ```
    pub fn tick_rest(&self) -> bool {
        matches!(self.duration, ConditionDuration::UntilRest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // ConditionDuration variant coverage
    // -----------------------------------------------------------------------

    #[test]
    fn test_condition_duration_until_combat_end_variant_exists() {
        let d = ConditionDuration::UntilCombatEnd;
        assert_eq!(d, ConditionDuration::UntilCombatEnd);
    }

    #[test]
    fn test_condition_duration_until_rest_variant_exists() {
        let d = ConditionDuration::UntilRest;
        assert_eq!(d, ConditionDuration::UntilRest);
    }

    #[test]
    fn test_condition_duration_all_variants_are_copy() {
        // Verifies that all variants implement Copy (no heap allocation surprises)
        let variants = [
            ConditionDuration::Instant,
            ConditionDuration::Rounds(5),
            ConditionDuration::Minutes(3),
            ConditionDuration::UntilCombatEnd,
            ConditionDuration::UntilRest,
            ConditionDuration::Permanent,
        ];
        // Copy: assigning to a second binding should compile without a move
        let _copies: Vec<_> = variants.to_vec();
    }

    // -----------------------------------------------------------------------
    // tick_round
    // -----------------------------------------------------------------------

    #[test]
    fn test_tick_round_rounds_decrements_and_expires_at_zero() {
        let mut ac = ActiveCondition::new("bless".to_string(), ConditionDuration::Rounds(2));
        assert!(
            !ac.tick_round(),
            "should not expire after first tick (1 left)"
        );
        assert!(ac.tick_round(), "should expire after second tick (0 left)");
    }

    #[test]
    fn test_tick_round_rounds_one_expires_immediately() {
        let mut ac = ActiveCondition::new("bless".to_string(), ConditionDuration::Rounds(1));
        assert!(ac.tick_round());
    }

    #[test]
    fn test_tick_round_instant_always_expires() {
        let mut ac = ActiveCondition::new("burst".to_string(), ConditionDuration::Instant);
        assert!(ac.tick_round());
    }

    #[test]
    fn test_tick_round_until_combat_end_not_expired_by_rounds() {
        let mut ac = ActiveCondition::new(
            "combat_bless".to_string(),
            ConditionDuration::UntilCombatEnd,
        );
        assert!(!ac.tick_round());
        assert!(!ac.tick_round());
    }

    #[test]
    fn test_tick_round_until_rest_not_expired_by_rounds() {
        let mut ac = ActiveCondition::new("exhaustion".to_string(), ConditionDuration::UntilRest);
        assert!(!ac.tick_round());
    }

    #[test]
    fn test_tick_round_minutes_not_expired_by_rounds() {
        let mut ac = ActiveCondition::new("haste".to_string(), ConditionDuration::Minutes(5));
        assert!(!ac.tick_round());
    }

    #[test]
    fn test_tick_round_permanent_not_expired_by_rounds() {
        let mut ac = ActiveCondition::new("poison".to_string(), ConditionDuration::Permanent);
        assert!(!ac.tick_round());
    }

    // -----------------------------------------------------------------------
    // tick_minute
    // -----------------------------------------------------------------------

    #[test]
    fn test_tick_minute_minutes_decrements_and_expires_at_zero() {
        let mut ac = ActiveCondition::new("haste".to_string(), ConditionDuration::Minutes(2));
        assert!(
            !ac.tick_minute(),
            "should not expire after first tick (1 left)"
        );
        assert!(ac.tick_minute(), "should expire after second tick (0 left)");
    }

    #[test]
    fn test_tick_minute_minutes_one_expires_immediately() {
        let mut ac = ActiveCondition::new("haste".to_string(), ConditionDuration::Minutes(1));
        assert!(ac.tick_minute());
    }

    #[test]
    fn test_tick_minute_rounds_expire_fast_in_exploration() {
        let mut ac = ActiveCondition::new("bless".to_string(), ConditionDuration::Rounds(99));
        // Rounds-based conditions expire immediately when ticked by minutes
        assert!(ac.tick_minute());
    }

    #[test]
    fn test_tick_minute_instant_always_expires() {
        let mut ac = ActiveCondition::new("burst".to_string(), ConditionDuration::Instant);
        assert!(ac.tick_minute());
    }

    #[test]
    fn test_tick_minute_until_combat_end_not_expired_by_minutes() {
        let mut ac = ActiveCondition::new(
            "combat_bless".to_string(),
            ConditionDuration::UntilCombatEnd,
        );
        assert!(!ac.tick_minute());
    }

    #[test]
    fn test_tick_minute_until_rest_not_expired_by_minutes() {
        let mut ac = ActiveCondition::new("exhaustion".to_string(), ConditionDuration::UntilRest);
        assert!(!ac.tick_minute());
    }

    #[test]
    fn test_tick_minute_permanent_not_expired_by_minutes() {
        let mut ac = ActiveCondition::new("poison".to_string(), ConditionDuration::Permanent);
        assert!(!ac.tick_minute());
    }

    // -----------------------------------------------------------------------
    // tick_combat_end
    // -----------------------------------------------------------------------

    #[test]
    fn test_tick_combat_end_until_combat_end_returns_true() {
        let ac = ActiveCondition::new(
            "combat_bless".to_string(),
            ConditionDuration::UntilCombatEnd,
        );
        assert!(ac.tick_combat_end());
    }

    #[test]
    fn test_tick_combat_end_instant_returns_false() {
        let ac = ActiveCondition::new("burst".to_string(), ConditionDuration::Instant);
        assert!(!ac.tick_combat_end());
    }

    #[test]
    fn test_tick_combat_end_rounds_returns_false() {
        let ac = ActiveCondition::new("bless".to_string(), ConditionDuration::Rounds(5));
        assert!(!ac.tick_combat_end());
    }

    #[test]
    fn test_tick_combat_end_minutes_returns_false() {
        let ac = ActiveCondition::new("haste".to_string(), ConditionDuration::Minutes(10));
        assert!(!ac.tick_combat_end());
    }

    #[test]
    fn test_tick_combat_end_until_rest_returns_false() {
        let ac = ActiveCondition::new("exhaustion".to_string(), ConditionDuration::UntilRest);
        assert!(!ac.tick_combat_end());
    }

    #[test]
    fn test_tick_combat_end_permanent_returns_false() {
        let ac = ActiveCondition::new("poison".to_string(), ConditionDuration::Permanent);
        assert!(!ac.tick_combat_end());
    }

    // -----------------------------------------------------------------------
    // tick_rest
    // -----------------------------------------------------------------------

    #[test]
    fn test_tick_rest_until_rest_returns_true() {
        let ac = ActiveCondition::new("exhaustion".to_string(), ConditionDuration::UntilRest);
        assert!(ac.tick_rest());
    }

    #[test]
    fn test_tick_rest_instant_returns_false() {
        let ac = ActiveCondition::new("burst".to_string(), ConditionDuration::Instant);
        assert!(!ac.tick_rest());
    }

    #[test]
    fn test_tick_rest_rounds_returns_false() {
        let ac = ActiveCondition::new("bless".to_string(), ConditionDuration::Rounds(5));
        assert!(!ac.tick_rest());
    }

    #[test]
    fn test_tick_rest_minutes_returns_false() {
        let ac = ActiveCondition::new("haste".to_string(), ConditionDuration::Minutes(10));
        assert!(!ac.tick_rest());
    }

    #[test]
    fn test_tick_rest_until_combat_end_returns_false() {
        let ac = ActiveCondition::new(
            "combat_bless".to_string(),
            ConditionDuration::UntilCombatEnd,
        );
        assert!(!ac.tick_rest());
    }

    #[test]
    fn test_tick_rest_permanent_returns_false() {
        let ac = ActiveCondition::new("poison".to_string(), ConditionDuration::Permanent);
        assert!(!ac.tick_rest());
    }

    // -----------------------------------------------------------------------
    // Serde round-trip for new variants
    // -----------------------------------------------------------------------

    #[test]
    fn test_condition_duration_until_combat_end_serde_roundtrip() {
        let original = ConditionDuration::UntilCombatEnd;
        let serialized = ron::to_string(&original).expect("serialize UntilCombatEnd");
        let deserialized: ConditionDuration =
            ron::from_str(&serialized).expect("deserialize UntilCombatEnd");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_condition_duration_until_rest_serde_roundtrip() {
        let original = ConditionDuration::UntilRest;
        let serialized = ron::to_string(&original).expect("serialize UntilRest");
        let deserialized: ConditionDuration =
            ron::from_str(&serialized).expect("deserialize UntilRest");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_active_condition_new_sets_magnitude_one() {
        let ac = ActiveCondition::new("test".to_string(), ConditionDuration::UntilCombatEnd);
        assert!((ac.magnitude - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_active_condition_with_magnitude_overrides_default() {
        let ac = ActiveCondition::new("test".to_string(), ConditionDuration::UntilRest)
            .with_magnitude(2.5);
        assert!((ac.magnitude - 2.5).abs() < f32::EPSILON);
    }
}
