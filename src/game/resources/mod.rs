// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Game resources module
//!
//! Contains global game resources and asset management systems.

use crate::application::GameState;
use bevy::prelude::*;

pub mod sprite_assets;

/// Global game state resource wrapper
#[derive(Resource)]
pub struct GlobalState(pub GameState);
