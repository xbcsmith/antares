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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionDuration {
    /// Instantaneous effect (e.g., immediate damage/healing)
    Instant,
    /// Lasts for a specific number of combat rounds
    Rounds(u16),
    /// Lasts for a specific number of minutes (exploration turns)
    Minutes(u16),
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
    pub fn new(condition_id: ConditionId, duration: ConditionDuration) -> Self {
        Self {
            condition_id,
            duration,
            magnitude: 1.0,
        }
    }

    pub fn with_magnitude(mut self, magnitude: f32) -> Self {
        self.magnitude = magnitude;
        self
    }

    /// Decrements duration for round-based conditions
    /// Returns true if the condition has expired
    pub fn tick_round(&mut self) -> bool {
        match &mut self.duration {
            ConditionDuration::Rounds(rounds) => {
                if *rounds > 0 {
                    *rounds -= 1;
                }
                *rounds == 0
            }
            ConditionDuration::Instant => true,
            _ => false,
        }
    }

    /// Decrements duration for minute-based conditions
    /// Returns true if the condition has expired
    pub fn tick_minute(&mut self) -> bool {
        match &mut self.duration {
            ConditionDuration::Minutes(minutes) => {
                if *minutes > 0 {
                    *minutes -= 1;
                }
                *minutes == 0
            }
            ConditionDuration::Rounds(_) => true, // Rounds expire fast in minutes
            ConditionDuration::Instant => true,
            _ => false,
        }
    }
}
