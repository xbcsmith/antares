// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared test fixtures for Antares integration tests.
//!
//! Import this module in integration test files with:
//! ```ignore
//! mod common;
//! ```
//! then call `common::make_test_campaign("id", "Name", "1.0.0")`.

use antares::sdk::campaign_loader::{Campaign, CampaignAssets, CampaignConfig, CampaignData};
use antares::sdk::game_config::GameConfig;
use std::path::PathBuf;

/// Returns a minimal [`Campaign`] suitable for integration tests.
///
/// All fields are set to sensible defaults. Override individual fields using
/// struct-update syntax:
///
/// ```ignore
/// let campaign = Campaign {
///     id: "custom".to_string(),
///     ..common::make_test_campaign("base", "Base", "1.0.0")
/// };
/// ```
#[allow(dead_code)]
pub fn make_test_campaign(id: &str, name: &str, version: &str) -> Campaign {
    Campaign {
        id: id.to_string(),
        name: name.to_string(),
        version: version.to_string(),
        author: "Test Author".to_string(),
        description: "A test campaign for integration testing".to_string(),
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        required_features: vec![],
        config: CampaignConfig::default(),
        data: CampaignData::default(),
        assets: CampaignAssets::default(),
        root_path: PathBuf::from(format!("campaigns/{}", id)),
        game_config: GameConfig::default(),
    }
}
