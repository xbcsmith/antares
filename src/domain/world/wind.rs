// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Per-campaign wind configuration types.
//!
//! [`CampaignWindConfig`] is loaded from `data/wind.ron` inside a campaign
//! directory and drives the grass wind shader. A missing file is silently
//! treated as [`WindSystemKind::None`] (no wind animation).

use serde::{Deserialize, Serialize};

// ── Wind system kind ──────────────────────────────────────────────────────────

/// Selects which wind animation algorithm drives grass sway.
///
/// # Examples
///
/// ```
/// use antares::domain::world::wind::WindSystemKind;
///
/// assert_eq!(WindSystemKind::default(), WindSystemKind::None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum WindSystemKind {
    /// No wind animation; grass is stationary.
    #[default]
    None,
    /// Simple sinusoidal sway controlled by `strength` and `frequency`.
    Sine,
    /// Spatially coherent Perlin noise wind.  Enables `perlin_scale`,
    /// `perlin_octaves`, and `perlin_seed`.
    Perlin,
}

// ── Serde defaults ────────────────────────────────────────────────────────────

fn default_strength() -> f32 {
    0.04
}
fn default_frequency() -> f32 {
    0.65
}
fn default_direction() -> [f32; 2] {
    [1.0, 0.0]
}
fn default_perlin_scale() -> f32 {
    100.0
}
fn default_perlin_octaves() -> u32 {
    4
}
fn default_perlin_seed() -> u32 {
    0
}

// ── CampaignWindConfig ────────────────────────────────────────────────────────

/// Per-campaign wind configuration loaded from `data/wind.ron`.
///
/// All fields carry serde defaults so a minimal RON file containing only
/// `wind_system: Sine` is valid.
///
/// # Examples
///
/// ```
/// use antares::domain::world::wind::{CampaignWindConfig, WindSystemKind};
///
/// let cfg = CampaignWindConfig::default();
/// assert_eq!(cfg.wind_system, WindSystemKind::None);
/// assert!((cfg.strength - 0.04).abs() < f32::EPSILON);
/// assert!((cfg.frequency - 0.65).abs() < f32::EPSILON);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignWindConfig {
    /// Wind animation algorithm.  Default: [`WindSystemKind::None`].
    #[serde(default)]
    pub wind_system: WindSystemKind,

    /// World-units sway amplitude.  Default: `0.04`.
    #[serde(default = "default_strength")]
    pub strength: f32,

    /// Cycles per second.  Default: `0.65`.
    #[serde(default = "default_frequency")]
    pub frequency: f32,

    /// Normalised XZ wind direction.  Default: `[1.0, 0.0]`.
    #[serde(default = "default_direction")]
    pub direction: [f32; 2],

    /// Noise tiling scale in world units (Perlin only).  Default: `100.0`.
    #[serde(default = "default_perlin_scale")]
    pub perlin_scale: f32,

    /// Number of noise octaves, 1–8 (Perlin only).  Default: `4`.
    #[serde(default = "default_perlin_octaves")]
    pub perlin_octaves: u32,

    /// RNG seed for the Perlin noise generator (Perlin only).  Default: `0`.
    #[serde(default = "default_perlin_seed")]
    pub perlin_seed: u32,
}

impl Default for CampaignWindConfig {
    fn default() -> Self {
        Self {
            wind_system: WindSystemKind::None,
            strength: default_strength(),
            frequency: default_frequency(),
            direction: default_direction(),
            perlin_scale: default_perlin_scale(),
            perlin_octaves: default_perlin_octaves(),
            perlin_seed: default_perlin_seed(),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// `WindSystemKind::default()` must be `None`.
    #[test]
    fn test_wind_system_kind_default_is_none() {
        assert_eq!(WindSystemKind::default(), WindSystemKind::None);
    }

    /// `CampaignWindConfig::default()` must produce the documented field values.
    #[test]
    fn test_campaign_wind_config_default_values() {
        let cfg = CampaignWindConfig::default();
        assert_eq!(cfg.wind_system, WindSystemKind::None);
        assert!((cfg.strength - 0.04).abs() < f32::EPSILON);
        assert!((cfg.frequency - 0.65).abs() < f32::EPSILON);
        assert_eq!(cfg.direction, [1.0, 0.0]);
        assert!((cfg.perlin_scale - 100.0).abs() < f32::EPSILON);
        assert_eq!(cfg.perlin_octaves, 4);
        assert_eq!(cfg.perlin_seed, 0);
    }

    /// RON round-trip for the `None` variant.
    #[test]
    fn test_ron_roundtrip_none() {
        let cfg = CampaignWindConfig::default();
        let ron = ron::to_string(&cfg).expect("serialize");
        let back: CampaignWindConfig = ron::from_str(&ron).expect("deserialize");
        assert_eq!(cfg, back);
    }

    /// RON round-trip for the `Sine` variant.
    #[test]
    fn test_ron_roundtrip_sine() {
        let cfg = CampaignWindConfig {
            wind_system: WindSystemKind::Sine,
            strength: 0.04,
            frequency: 0.65,
            direction: [1.0, 0.0],
            ..Default::default()
        };
        let ron = ron::to_string(&cfg).expect("serialize");
        let back: CampaignWindConfig = ron::from_str(&ron).expect("deserialize");
        assert_eq!(cfg, back);
    }

    /// RON round-trip for the `Perlin` variant.
    #[test]
    fn test_ron_roundtrip_perlin() {
        let cfg = CampaignWindConfig {
            wind_system: WindSystemKind::Perlin,
            strength: 0.06,
            frequency: 0.5,
            direction: [0.71, 0.71],
            perlin_scale: 80.0,
            perlin_octaves: 4,
            perlin_seed: 12345,
        };
        let ron = ron::to_string(&cfg).expect("serialize");
        let back: CampaignWindConfig = ron::from_str(&ron).expect("deserialize");
        assert_eq!(cfg, back);
    }

    /// A minimal RON file containing only `wind_system` must deserialize
    /// with serde defaults for all other fields.
    #[test]
    fn test_minimal_ron_deserializes_with_defaults() {
        let minimal = "(wind_system: Sine)";
        let cfg: CampaignWindConfig = ron::from_str(minimal).expect("deserialize minimal");
        assert_eq!(cfg.wind_system, WindSystemKind::Sine);
        assert!((cfg.strength - 0.04).abs() < f32::EPSILON);
        assert!((cfg.frequency - 0.65).abs() < f32::EPSILON);
        assert_eq!(cfg.direction, [1.0, 0.0]);
        assert!((cfg.perlin_scale - 100.0).abs() < f32::EPSILON);
        assert_eq!(cfg.perlin_octaves, 4);
        assert_eq!(cfg.perlin_seed, 0);
    }
}
