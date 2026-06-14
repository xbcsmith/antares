// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Wind configuration Bevy resource.
//!
//! [`WindConfig`] wraps the campaign-loaded [`CampaignWindConfig`] as a Bevy
//! resource so wind shader parameters are accessible to rendering systems.

use bevy::prelude::*;

use crate::domain::world::wind::CampaignWindConfig;

/// Bevy resource that holds the active campaign wind configuration.
///
/// Inserted at campaign load time alongside [`crate::game::resources::GameDataResource`].
/// A missing `data/wind.ron` results in the default (no wind animation).
///
/// # Examples
///
/// ```
/// use antares::game::resources::wind_config::WindConfig;
/// use antares::domain::world::wind::{CampaignWindConfig, WindSystemKind};
///
/// let res = WindConfig::default();
/// assert_eq!(res.0.wind_system, WindSystemKind::None);
/// ```
#[derive(Resource, Clone, Debug, Default)]
pub struct WindConfig(pub CampaignWindConfig);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::world::wind::WindSystemKind;

    /// `WindConfig::default()` wraps `CampaignWindConfig::default()`.
    #[test]
    fn test_wind_config_default_is_no_wind() {
        let res = WindConfig::default();
        assert_eq!(res.0.wind_system, WindSystemKind::None);
    }

    /// `WindConfig` can be constructed from any `CampaignWindConfig`.
    #[test]
    fn test_wind_config_wraps_config() {
        use crate::domain::world::wind::CampaignWindConfig;
        let cfg = CampaignWindConfig {
            wind_system: WindSystemKind::Sine,
            ..Default::default()
        };
        let res = WindConfig(cfg.clone());
        assert_eq!(res.0, cfg);
    }
}
