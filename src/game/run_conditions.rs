// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared Bevy run-condition helpers for `GameMode`-gated systems.
//!
//! Use these with `.run_if(...)` in plugin `build` methods to avoid
//! repeating `if !matches!(mode, …) { return; }` prologues in system bodies.

use crate::application::GameMode;
use crate::game::resources::GlobalState;
use bevy::prelude::Res;

/// Returns `true` while the game is in any [`GameMode::Combat`] state.
///
/// # Examples
///
/// ```
/// use bevy::prelude::World;
/// use antares::game::resources::GlobalState;
/// use antares::application::GameState;
///
/// let mut world = World::new();
/// world.insert_resource(GlobalState(GameState::new()));
/// assert!(!world.run_system_cached(antares::game::run_conditions::in_combat_mode).unwrap());
/// ```
pub fn in_combat_mode(state: Res<GlobalState>) -> bool {
    matches!(state.0.mode, GameMode::Combat(_))
}

/// Returns `true` while the game is in any [`GameMode::Dialogue`] state.
///
/// # Examples
///
/// ```
/// use bevy::prelude::World;
/// use antares::game::resources::GlobalState;
/// use antares::application::GameState;
///
/// let mut world = World::new();
/// world.insert_resource(GlobalState(GameState::new()));
/// assert!(!world.run_system_cached(antares::game::run_conditions::in_dialogue_mode).unwrap());
/// ```
pub fn in_dialogue_mode(state: Res<GlobalState>) -> bool {
    matches!(state.0.mode, GameMode::Dialogue(_))
}

/// Returns `true` while the game is in any [`GameMode::CharacterSheet`] state.
///
/// # Examples
///
/// ```
/// use bevy::prelude::World;
/// use antares::game::resources::GlobalState;
/// use antares::application::GameState;
///
/// let mut world = World::new();
/// world.insert_resource(GlobalState(GameState::new()));
/// assert!(!world.run_system_cached(antares::game::run_conditions::in_character_sheet_mode).unwrap());
/// ```
pub fn in_character_sheet_mode(state: Res<GlobalState>) -> bool {
    matches!(state.0.mode, GameMode::CharacterSheet(_))
}

/// Returns `true` while the full-screen automap overlay is open.
///
/// # Examples
///
/// ```
/// use bevy::prelude::World;
/// use antares::game::resources::GlobalState;
/// use antares::application::GameState;
///
/// let mut world = World::new();
/// world.insert_resource(GlobalState(GameState::new()));
/// assert!(!world.run_system_cached(antares::game::run_conditions::in_automap_mode).unwrap());
/// ```
pub fn in_automap_mode(state: Res<GlobalState>) -> bool {
    matches!(state.0.mode, GameMode::Automap)
}
