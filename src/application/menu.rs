// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Menu state for the application layer
//!
//! This module defines the core `MenuState` used to represent the in-game
//! menu (pause/menu) system. It is intentionally a small, pure-data
//! structure used by application logic and UI systems.
//!
//! # Examples
//!
//! ```
/// use antares::application::menu::{MenuState, MenuType};
/// use antares::application::GameMode;
///
/// let ms = MenuState::new(GameMode::Exploration);
/// assert!(matches!(ms.get_resume_mode(), GameMode::Exploration));
/// assert_eq!(ms.current_submenu, MenuType::Main);
/// ```
use crate::application::GameMode;
use serde::{Deserialize, Serialize};

/// State for the in-game menu system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuState {
    /// The game mode that was active before opening the menu
    ///
    /// Stored as a `Box<GameMode>` to break recursive size dependency
    /// between `MenuState` and `GameMode::Menu(MenuState)`.
    pub previous_mode: Box<GameMode>,

    /// Current submenu being displayed
    pub current_submenu: MenuType,

    /// Selected option index in current submenu (0-based)
    pub selected_index: usize,

    /// Cached list of save files (populated when SaveLoad submenu opens)
    #[serde(default)]
    pub save_list: Vec<SaveGameInfo>,
}

/// Menu screen types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuType {
    /// Main menu with Resume/Save/Load/Settings/Quit
    Main,

    /// Save/Load game screen
    SaveLoad,

    /// Settings configuration screen
    Settings,
}

/// Information about a save file for display in save/load UI
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveGameInfo {
    /// Filename without path
    pub filename: String,

    /// Human-readable timestamp
    pub timestamp: String,

    /// Names of characters in the party at time of save
    pub character_names: Vec<String>,

    /// Current location description
    pub location: String,

    /// Save file version
    pub game_version: String,
}

impl MenuState {
    /// Create a new `MenuState`, storing the previous game mode.
    ///
    /// # Arguments
    ///
    /// * `previous_mode` - The `GameMode` that was active before opening the menu.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::menu::MenuState;
    /// use antares::application::GameMode;
    ///
    /// let ms = MenuState::new(GameMode::Exploration);
    /// assert!(matches!(ms.get_resume_mode(), GameMode::Exploration));
    /// ```
    pub fn new(previous_mode: GameMode) -> Self {
        Self {
            previous_mode: Box::new(previous_mode),
            current_submenu: MenuType::Main,
            selected_index: 0,
            save_list: Vec::new(),
        }
    }

    /// Get the `GameMode` to return to when closing the menu.
    ///
    /// This returns a cloned `GameMode` previously stored.
    pub fn get_resume_mode(&self) -> GameMode {
        (*self.previous_mode).clone()
    }

    /// Switch to a different submenu and reset selection index.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::application::menu::{MenuState, MenuType};
    /// use antares::application::GameMode;
    ///
    /// let mut ms = MenuState::new(GameMode::Exploration);
    /// ms.selected_index = 3;
    /// ms.set_submenu(MenuType::Settings);
    /// assert_eq!(ms.current_submenu, MenuType::Settings);
    /// assert_eq!(ms.selected_index, 0);
    /// ```
    pub fn set_submenu(&mut self, submenu: MenuType) {
        self.current_submenu = submenu;
        self.selected_index = 0;
    }

    /// Move selection up (with wrapping).
    ///
    /// * If `item_count == 0` this is a no-op.
    /// * If the selection is at `0` it wraps to `item_count - 1`.
    pub fn select_previous(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }
        if self.selected_index == 0 {
            self.selected_index = item_count - 1;
        } else {
            self.selected_index -= 1;
        }
    }

    /// Move selection down (with wrapping).
    ///
    /// * If `item_count == 0` this is a no-op.
    /// * Selection wraps using modulo arithmetic.
    pub fn select_next(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }
        self.selected_index = (self.selected_index + 1) % item_count;
    }
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            previous_mode: Box::new(GameMode::Exploration),
            current_submenu: MenuType::Main,
            selected_index: 0,
            save_list: Vec::new(),
        }
    }
}
