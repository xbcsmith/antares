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
/// Contains gameplay tuning parameters and custom rules.
///
/// # Examples
///
/// ```
/// use antares::domain::campaign::CampaignConfig;
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
/// };
///
/// assert_eq!(config.max_party_level, Some(20));
/// assert!(!config.permadeath);
/// assert!(config.unconscious_before_death);
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
}

/// Serde helper: returns `true` as the default value for boolean fields that
/// default to `true` rather than `false`.
fn default_true() -> bool {
    true
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
}
