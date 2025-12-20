// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Application-level resources exposed to Bevy systems
//!
//! This module defines a thin resource wrapper around the SDK's
//! `ContentDatabase`, making campaign content available to ECS systems
//! via Bevy's resource mechanism.

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_content_new() {
        let db = ContentDatabase::new();
        let resource = GameContent::new(db);
        // Basic smoke test: empty content database has zero classes
        assert_eq!(resource.db().classes.all_classes().count(), 0);
    }
}
