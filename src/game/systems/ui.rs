// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameLog>();
    }
}

#[derive(Resource, Default)]
pub struct GameLog {
    pub messages: Vec<String>,
}

impl GameLog {
    /// Create a new empty game log
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Add a message to the game log
    pub fn add(&mut self, msg: String) {
        self.messages.push(msg);
        if self.messages.len() > 50 {
            self.messages.remove(0);
        }
    }

    /// Get all log entries
    pub fn entries(&self) -> &[String] {
        &self.messages
    }
}
