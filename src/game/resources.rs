// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::application::GameState;
use bevy::prelude::*;

/// Global game state resource wrapper
#[derive(Resource)]
pub struct GlobalState(pub GameState);
