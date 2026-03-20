// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Application-level resources exposed to Bevy systems
//!
//! This module defines a thin resource wrapper around the SDK's
//! `ContentDatabase`, making campaign content available to ECS systems
//! via Bevy's resource mechanism.
//!
//! It also contains application-layer enforcement helpers for campaign rules
//! such as permadeath that must be checked before domain operations are
//! performed.

use crate::domain::campaign::CampaignConfig;
use crate::sdk::database::ContentDatabase;
use bevy::prelude::*;

/// Wrapper resource exposing campaign content as a Bevy resource.
///
/// Systems can fetch this resource to query items, spells, maps, and
/// other campaign data loaded by the SDK.
///
/// # Examples
///
/// ```no_run
/// use antares::application::resources::GameContent;
/// use antares::sdk::database::ContentDatabase;
///
/// let db = ContentDatabase::new();
/// let content = GameContent::new(db);
/// assert_eq!(content.db().classes.all_classes().count(), 0);
/// ```
#[derive(Resource, Debug, Clone)]
pub struct GameContent(pub ContentDatabase);

impl GameContent {
    /// Create a new `GameContent` resource from a `ContentDatabase`.
    pub fn new(db: ContentDatabase) -> Self {
        Self(db)
    }

    /// Immutable access to the underlying `ContentDatabase`.
    pub fn db(&self) -> &ContentDatabase {
        &self.0
    }

    /// Mutable access to the underlying `ContentDatabase`.
    pub fn db_mut(&mut self) -> &mut ContentDatabase {
        &mut self.0
    }
}

/// Returns `Ok(())` if the campaign allows resurrection, or an `Err` string
/// when permadeath is enabled.
///
/// This helper must be called by any application-layer or game-system code
/// that is about to apply a `ConsumableEffect::Resurrect` or cast a spell
/// with `resurrect_hp: Some(_)`.  The domain layer does **not** check
/// permadeath — enforcement is the caller's responsibility.
///
/// # Arguments
///
/// * `config` — the active campaign's [`CampaignConfig`].
///
/// # Errors
///
/// Returns `Err(String)` with a human-readable message when
/// `config.permadeath == true`.
///
/// # Examples
///
/// ```
/// use antares::application::resources::check_permadeath_allows_resurrection;
/// use antares::domain::campaign::CampaignConfig;
///
/// // Default config has permadeath == false → resurrection allowed.
/// let config = CampaignConfig::default();
/// assert!(check_permadeath_allows_resurrection(&config).is_ok());
///
/// // Permadeath config → resurrection blocked.
/// let mut pd_config = CampaignConfig::default();
/// pd_config.permadeath = true;
/// assert!(check_permadeath_allows_resurrection(&pd_config).is_err());
/// ```
pub fn check_permadeath_allows_resurrection(config: &CampaignConfig) -> Result<(), String> {
    if config.permadeath {
        Err("Resurrection is not allowed in this campaign (permadeath enabled).".to_string())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::campaign::CampaignConfig;

    #[test]
    fn test_game_content_new() {
        let db = ContentDatabase::new();
        let resource = GameContent::new(db);
        // Basic smoke test: empty content database has zero classes
        assert_eq!(resource.db().classes.all_classes().count(), 0);
    }

    /// `check_permadeath_allows_resurrection` must return `Ok` when permadeath
    /// is disabled (the default).
    #[test]
    fn test_permadeath_allows_resurrection_by_default() {
        let config = CampaignConfig::default();
        assert!(
            check_permadeath_allows_resurrection(&config).is_ok(),
            "resurrection must be allowed when permadeath == false"
        );
    }

    /// `check_permadeath_allows_resurrection` must return `Err` when permadeath
    /// is enabled.
    #[test]
    fn test_permadeath_blocks_resurrection() {
        let config = CampaignConfig {
            permadeath: true,
            ..CampaignConfig::default()
        };
        let result = check_permadeath_allows_resurrection(&config);
        assert!(
            result.is_err(),
            "resurrection must be blocked when permadeath == true"
        );
        assert!(
            result.unwrap_err().contains("permadeath"),
            "error message must mention permadeath"
        );
    }
}
