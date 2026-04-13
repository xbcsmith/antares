// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Campaign system for modular content packaging
//!
//! This module provides campaign definitions and metadata structures
//! for organizing game content into campaigns.
//!
//! # Examples
//!
//! ```
//! use antares::domain::campaign::{Campaign, CampaignConfig};
//! use antares::domain::types::{MapId, Position};
//! use std::collections::BTreeMap;
//!
//! let campaign = Campaign {
//!     id: "tutorial".to_string(),
//!     name: "Tutorial Campaign".to_string(),
//!     version: "1.0.0".to_string(),
//!     description: "A tutorial campaign for new players".to_string(),
//!     author: "Antares Team".to_string(),
//!     starting_map: MapId::from(1u16),
//!     starting_position: Position { x: 0, y: 0 },
//!     starting_facing: antares::domain::types::Direction::North,
//!     starting_innkeeper: Some("innkeeper_1".to_string()),
//!     required_data_version: "1.0.0".to_string(),
//!     dependencies: vec![],
//!     content_overrides: BTreeMap::new(),
//! };
//! ```
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::domain::types::{Direction, MapId, Position};

/// Controls how characters advance in level.
///
/// This enum is stored in [`CampaignConfig`] and governs whether characters
/// level up automatically upon reaching the XP threshold, or whether they
/// must seek out and pay a trainer NPC to apply their accumulated levels.
///
/// # Examples
///
/// ```
/// use antares::domain::campaign::LevelUpMode;
///
/// let mode = LevelUpMode::Auto;
/// assert_eq!(mode, LevelUpMode::default());
///
/// let trainer_mode = LevelUpMode::NpcTrainer;
/// assert_ne!(trainer_mode, LevelUpMode::Auto);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum LevelUpMode {
    /// Characters level up automatically the moment the XP threshold is reached.
    #[default]
    Auto,
    /// Characters must visit a trainer NPC and pay a gold fee to level up.
    NpcTrainer,
}

/// Complete campaign definition
///
/// Represents a campaign with all metadata and configuration.
///
/// # Examples
///
/// ```
/// use antares::domain::campaign::Campaign;
/// use antares::domain::types::{MapId, Position};
/// use std::collections::BTreeMap;
///
/// let campaign = Campaign {
///     id: "tutorial".to_string(),
///     name: "Tutorial".to_string(),
///     version: "1.0.0".to_string(),
///     description: "Tutorial campaign".to_string(),
///     author: "Developer".to_string(),
///     starting_map: MapId::from(1u16),
///     starting_position: Position { x: 5, y: 5 },
///     starting_facing: antares::domain::types::Direction::North,
///     starting_innkeeper: None,
///     required_data_version: "1.0.0".to_string(),
///     dependencies: vec![],
///     content_overrides: BTreeMap::new(),
/// };
///
/// assert_eq!(campaign.id, "tutorial");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    /// Unique campaign identifier
    pub id: String,

    /// Display name of the campaign
    pub name: String,

    /// Campaign version (semantic versioning)
    pub version: String,

    /// Campaign description
    pub description: String,

    /// Campaign author(s)
    pub author: String,

    /// Starting map ID
    pub starting_map: MapId,

    /// Starting position on the map
    pub starting_position: Position,

    /// Starting facing direction
    pub starting_facing: Direction,

    /// Optional starting innkeeper identifier
    pub starting_innkeeper: Option<String>,

    /// Required data version for compatibility
    pub required_data_version: String,

    /// Other campaigns required as dependencies
    pub dependencies: Vec<String>,

    /// File path overrides (file -> override path)
    pub content_overrides: BTreeMap<String, String>,
}

/// Campaign metadata and configuration
///
/// Contains gameplay tuning parameters and custom rules, including the XP
/// curve formula parameters and the level-up progression mode.
///
/// All fields use `#[serde(default)]` so that existing `config.ron` files
/// that predate any field addition continue to deserialize without errors.
///
/// # Examples
///
/// ```
/// use antares::domain::campaign::{CampaignConfig, LevelUpMode};
/// use std::collections::BTreeMap;
///
/// let config = CampaignConfig {
///     max_party_level: Some(20),
///     difficulty_multiplier: 1.0,
///     experience_rate: 1.0,
///     gold_rate: 1.0,
///     random_encounter_rate: 1.0,
///     rest_healing_rate: 1.0,
///     custom_rules: BTreeMap::new(),
///     permadeath: false,
///     unconscious_before_death: true,
///     base_xp: 1000,
///     xp_multiplier: 1.5,
///     level_up_mode: LevelUpMode::Auto,
///     training_fee_base: 500,
///     training_fee_multiplier: 1.0,
/// };
///
/// assert_eq!(config.max_party_level, Some(20));
/// assert!(!config.permadeath);
/// assert!(config.unconscious_before_death);
/// assert_eq!(config.base_xp, 1000);
/// assert_eq!(config.level_up_mode, LevelUpMode::Auto);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignConfig {
    /// Maximum party level allowed
    pub max_party_level: Option<u32>,

    /// Difficulty multiplier (1.0 = normal)
    pub difficulty_multiplier: f32,

    /// Experience gain rate (1.0 = normal)
    pub experience_rate: f32,

    /// Gold gain rate (1.0 = normal)
    pub gold_rate: f32,

    /// Random encounter rate (1.0 = normal)
    pub random_encounter_rate: f32,

    /// Rest healing rate (1.0 = normal)
    pub rest_healing_rate: f32,

    /// Custom campaign-specific rules
    pub custom_rules: BTreeMap<String, String>,

    /// If true, dead characters cannot be resurrected by any means.
    /// Campaign creators set this for permadeath runs. Default: false.
    #[serde(default)]
    pub permadeath: bool,

    /// If true, a character that reaches 0 HP becomes unconscious first and
    /// only dies if they receive further damage while unconscious.
    /// If false, reaching 0 HP sets DEAD immediately (instant death mode).
    /// Default: true (unconscious before death, classic RPG behavior).
    #[serde(default = "default_true")]
    pub unconscious_before_death: bool,

    /// Base XP required to reach level 2 (default: 1000).
    ///
    /// Used in the XP threshold formula:
    /// `base_xp * (level - 1) ^ xp_multiplier`
    ///
    /// This replaces the private `BASE_XP` constant and allows per-campaign
    /// customisation of the XP curve steepness.
    #[serde(default = "default_base_xp")]
    pub base_xp: u64,

    /// Exponent controlling the steepness of the XP curve (default: 1.5).
    ///
    /// Higher values make each successive level require significantly more XP.
    /// Combined with [`Self::base_xp`], this drives the formula:
    /// `base_xp * (level - 1) ^ xp_multiplier`
    ///
    /// This replaces the private `XP_MULTIPLIER` constant.
    #[serde(default = "default_xp_multiplier")]
    pub xp_multiplier: f64,

    /// Controls whether characters level up automatically or require a trainer.
    ///
    /// - [`LevelUpMode::Auto`]: Characters level up instantly when XP threshold
    ///   is reached (default, classic behaviour).
    /// - [`LevelUpMode::NpcTrainer`]: Characters accumulate levels internally
    ///   but must pay a trainer NPC to apply them.
    #[serde(default)]
    pub level_up_mode: LevelUpMode,

    /// Base gold fee charged per level-up when using [`LevelUpMode::NpcTrainer`]
    /// (default: 500).
    ///
    /// The total fee for a given character level is:
    /// `training_fee_base * level * training_fee_multiplier`
    #[serde(default = "default_training_fee_base")]
    pub training_fee_base: u32,

    /// Per-level fee multiplier applied on top of [`Self::training_fee_base`]
    /// when paying a trainer NPC (default: 1.0).
    ///
    /// Values greater than 1.0 make higher-level training progressively more
    /// expensive; values below 1.0 offer a discount for higher levels.
    #[serde(default = "default_training_fee_multiplier")]
    pub training_fee_multiplier: f32,
}

/// Serde helper: returns `true` as the default value for boolean fields that
/// default to `true` rather than `false`.
fn default_true() -> bool {
    true
}

/// Serde default: base XP for the level-up formula (1000).
fn default_base_xp() -> u64 {
    1000
}

/// Serde default: XP curve exponent (1.5).
fn default_xp_multiplier() -> f64 {
    1.5
}

/// Serde default: base gold fee per level for NPC trainer mode (500).
fn default_training_fee_base() -> u32 {
    500
}

/// Serde default: per-level fee multiplier for NPC trainer mode (1.0).
fn default_training_fee_multiplier() -> f32 {
    1.0
}

impl Default for CampaignConfig {
    fn default() -> Self {
        Self {
            max_party_level: None,
            difficulty_multiplier: 1.0,
            experience_rate: 1.0,
            gold_rate: 1.0,
            random_encounter_rate: 1.0,
            rest_healing_rate: 1.0,
            custom_rules: BTreeMap::new(),
            permadeath: false,
            unconscious_before_death: true,
            base_xp: default_base_xp(),
            xp_multiplier: default_xp_multiplier(),
            level_up_mode: LevelUpMode::Auto,
            training_fee_base: default_training_fee_base(),
            training_fee_multiplier: default_training_fee_multiplier(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_campaign_creation() {
        let campaign = Campaign {
            id: "test".to_string(),
            name: "Test Campaign".to_string(),
            version: "1.0.0".to_string(),
            description: "A test campaign".to_string(),
            author: "Test Author".to_string(),
            starting_map: 1,
            starting_position: Position { x: 5, y: 5 },
            starting_facing: Direction::North,
            starting_innkeeper: Some("innkeeper_1".to_string()),
            required_data_version: "1.0.0".to_string(),
            dependencies: vec![],
            content_overrides: BTreeMap::new(),
        };

        assert_eq!(campaign.id, "test");
        assert_eq!(campaign.name, "Test Campaign");
        assert_eq!(campaign.starting_map, 1);
    }

    #[test]
    fn test_campaign_config_default() {
        let config = CampaignConfig::default();

        assert_eq!(config.difficulty_multiplier, 1.0);
        assert_eq!(config.experience_rate, 1.0);
        assert_eq!(config.gold_rate, 1.0);
        assert!(config.custom_rules.is_empty());
        assert_eq!(config.base_xp, 1000);
        assert_eq!(config.xp_multiplier, 1.5);
        assert_eq!(config.level_up_mode, LevelUpMode::Auto);
        assert_eq!(config.training_fee_base, 500);
        assert_eq!(config.training_fee_multiplier, 1.0);
    }

    /// `CampaignConfig::default()` must have `unconscious_before_death == true`
    /// and `permadeath == false` — the classic RPG behavior.
    #[test]
    fn test_unconscious_before_death_mode_default() {
        let config = CampaignConfig::default();
        assert!(
            config.unconscious_before_death,
            "unconscious_before_death must default to true (classic RPG mode)"
        );
        assert!(!config.permadeath, "permadeath must default to false");
    }

    #[test]
    fn test_campaign_with_dependencies() {
        let campaign = Campaign {
            id: "expansion".to_string(),
            name: "Expansion".to_string(),
            version: "1.0.0".to_string(),
            description: "Expansion campaign".to_string(),
            author: "Author".to_string(),
            starting_map: 100,
            starting_position: Position { x: 0, y: 0 },
            starting_facing: Direction::East,
            starting_innkeeper: None,
            required_data_version: "1.0.0".to_string(),
            dependencies: vec!["base_campaign".to_string()],
            content_overrides: BTreeMap::new(),
        };

        assert_eq!(campaign.dependencies.len(), 1);
        assert_eq!(campaign.dependencies[0], "base_campaign");
    }

    #[test]
    fn test_campaign_serialization() {
        let campaign = Campaign {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            starting_map: 1,
            starting_position: Position { x: 5, y: 5 },
            starting_facing: Direction::North,
            starting_innkeeper: None,
            required_data_version: "1.0.0".to_string(),
            dependencies: vec![],
            content_overrides: BTreeMap::new(),
        };

        let serialized = ron::to_string(&campaign).expect("Serialization failed");
        let deserialized: Campaign = ron::from_str(&serialized).expect("Deserialization failed");

        assert_eq!(campaign.id, deserialized.id);
        assert_eq!(campaign.name, deserialized.name);
    }

    /// `LevelUpMode::Auto` is the default variant and serialises to `"Auto"` in RON.
    #[test]
    fn test_level_up_mode_default_is_auto() {
        assert_eq!(LevelUpMode::default(), LevelUpMode::Auto);
    }

    /// Round-trip `LevelUpMode` through RON to verify both variants survive intact.
    #[test]
    fn test_level_up_mode_serialization_round_trip() {
        let auto_ron = ron::to_string(&LevelUpMode::Auto).expect("serialize Auto");
        let trainer_ron = ron::to_string(&LevelUpMode::NpcTrainer).expect("serialize NpcTrainer");

        let auto_back: LevelUpMode = ron::from_str(&auto_ron).expect("deserialize Auto");
        let trainer_back: LevelUpMode =
            ron::from_str(&trainer_ron).expect("deserialize NpcTrainer");

        assert_eq!(auto_back, LevelUpMode::Auto);
        assert_eq!(trainer_back, LevelUpMode::NpcTrainer);
    }

    /// `training_fee_base` and `training_fee_multiplier` survive a
    /// `CampaignConfig` RON round-trip unchanged.
    #[test]
    fn test_training_fee_fields_round_trip() {
        let config = CampaignConfig {
            training_fee_base: 750,
            training_fee_multiplier: 1.5,
            ..CampaignConfig::default()
        };

        let ron_str = ron::to_string(&config).expect("serialize CampaignConfig");
        let back: CampaignConfig = ron::from_str(&ron_str).expect("deserialize CampaignConfig");

        assert_eq!(back.training_fee_base, 750);
        assert_eq!(back.training_fee_multiplier, 1.5);
    }

    /// `base_xp` and `xp_multiplier` survive a RON round-trip and produce
    /// the expected XP threshold for level 5 when used in the formula.
    #[test]
    fn test_base_xp_and_multiplier_round_trip() {
        let config = CampaignConfig {
            base_xp: 500,
            xp_multiplier: 2.0,
            ..CampaignConfig::default()
        };

        let ron_str = ron::to_string(&config).expect("serialize");
        let back: CampaignConfig = ron::from_str(&ron_str).expect("deserialize");

        assert_eq!(back.base_xp, 500);
        assert_eq!(back.xp_multiplier, 2.0);

        // Formula: base_xp * (level - 1) ^ xp_multiplier = 500 * 4^2 = 8000
        let level_5_threshold = (back.base_xp as f64 * (4_f64).powf(back.xp_multiplier)) as u64;
        assert_eq!(level_5_threshold, 8000);
    }

    /// A `CampaignConfig` RON string that omits all new fields (simulating an
    /// old config file) must still deserialise successfully using defaults.
    #[test]
    fn test_campaign_config_new_fields_default_when_absent() {
        // Minimal RON that only contains the pre-existing fields
        let ron_str = r#"(
            max_party_level: None,
            difficulty_multiplier: 1.0,
            experience_rate: 1.0,
            gold_rate: 1.0,
            random_encounter_rate: 1.0,
            rest_healing_rate: 1.0,
            custom_rules: {},
            permadeath: false,
            unconscious_before_death: true,
        )"#;

        let config: CampaignConfig =
            ron::from_str(ron_str).expect("old CampaignConfig must parse with new default fields");

        assert_eq!(config.base_xp, 1000, "base_xp must default to 1000");
        assert_eq!(
            config.xp_multiplier, 1.5,
            "xp_multiplier must default to 1.5"
        );
        assert_eq!(
            config.level_up_mode,
            LevelUpMode::Auto,
            "level_up_mode must default to Auto"
        );
        assert_eq!(
            config.training_fee_base, 500,
            "training_fee_base must default to 500"
        );
        assert_eq!(
            config.training_fee_multiplier, 1.0,
            "training_fee_multiplier must default to 1.0"
        );
    }

    /// A complete `CampaignConfig` RON block with the new fields explicitly set
    /// must deserialise to those exact values.
    #[test]
    fn test_campaign_config_new_fields_explicit_values() {
        let ron_str = r#"(
            max_party_level: Some(10),
            difficulty_multiplier: 1.0,
            experience_rate: 2.0,
            gold_rate: 1.0,
            random_encounter_rate: 1.0,
            rest_healing_rate: 1.0,
            custom_rules: {},
            permadeath: false,
            unconscious_before_death: true,
            base_xp: 800,
            xp_multiplier: 2.0,
            level_up_mode: NpcTrainer,
            training_fee_base: 250,
            training_fee_multiplier: 0.5,
        )"#;

        let config: CampaignConfig =
            ron::from_str(ron_str).expect("full CampaignConfig must parse");

        assert_eq!(config.base_xp, 800);
        assert_eq!(config.xp_multiplier, 2.0);
        assert_eq!(config.level_up_mode, LevelUpMode::NpcTrainer);
        assert_eq!(config.training_fee_base, 250);
        assert_eq!(config.training_fee_multiplier, 0.5);
        assert_eq!(config.experience_rate, 2.0);
    }
}
