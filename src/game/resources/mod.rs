// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Game resources module
//!
//! Contains global game resources and asset management systems.

use crate::application::GameState;
use bevy::prelude::*;

pub mod game_data;
pub mod grass_quality_settings;
pub mod performance;
pub mod sprite_assets;

// Re-export commonly used types
pub use game_data::GameDataResource;
pub use grass_quality_settings::{GrassPerformanceLevel, GrassQualitySettings};
pub use performance::{LodAutoTuning, MeshCache, PerformanceMetrics};

/// Global game state resource wrapper
#[derive(Resource)]
pub struct GlobalState(pub GameState);
