// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Game data resource for Bevy ECS
//!
//! This module provides a Bevy resource that holds all loaded game data
//! including creatures, items, spells, and other campaign content.
//!
//! # Examples
//!
//! ```
//! use antares::game::resources::game_data::GameDataResource;
//! use antares::domain::campaign_loader::GameData;
//!
//! let resource = GameDataResource::new(GameData::new());
//! assert!(resource.data().creatures.is_empty());
//! ```

use bevy::prelude::*;

use crate::domain::campaign_loader::GameData;
use crate::domain::types::CreatureId;
use crate::domain::visual::CreatureDefinition;

/// Bevy resource holding all loaded game data
///
/// This resource provides access to the game's creature database,
/// and in the future will hold items, spells, monsters, etc.
///
/// # Examples
///
/// ```
/// use antares::game::resources::game_data::GameDataResource;
/// use antares::domain::campaign_loader::GameData;
///
/// let data = GameData::new();
/// let resource = GameDataResource::new(data);
/// ```
#[derive(Resource, Debug, Clone)]
pub struct GameDataResource {
    data: GameData,
}

impl GameDataResource {
    /// Creates a new GameDataResource
    ///
    /// # Arguments
    ///
    /// * `data` - The game data to wrap
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::game_data::GameDataResource;
    /// use antares::domain::campaign_loader::GameData;
    ///
    /// let resource = GameDataResource::new(GameData::new());
    /// assert!(resource.data().creatures.is_empty());
    /// ```
    pub fn new(data: GameData) -> Self {
        Self { data }
    }

    /// Gets a reference to the game data
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::game_data::GameDataResource;
    /// use antares::domain::campaign_loader::GameData;
    ///
    /// let resource = GameDataResource::new(GameData::new());
    /// let data = resource.data();
    /// assert!(data.creatures.is_empty());
    /// ```
    pub fn data(&self) -> &GameData {
        &self.data
    }

    /// Gets a mutable reference to the game data
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::game_data::GameDataResource;
    /// use antares::domain::campaign_loader::GameData;
    ///
    /// let mut resource = GameDataResource::new(GameData::new());
    /// let data = resource.data_mut();
    /// ```
    pub fn data_mut(&mut self) -> &mut GameData {
        &mut self.data
    }

    /// Gets a creature by ID from the creature database
    ///
    /// # Arguments
    ///
    /// * `id` - The creature ID to look up
    ///
    /// # Returns
    ///
    /// Returns `Some(&CreatureDefinition)` if found, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::game_data::GameDataResource;
    /// use antares::domain::campaign_loader::GameData;
    /// use antares::domain::types::CreatureId;
    ///
    /// let resource = GameDataResource::new(GameData::new());
    /// let creature = resource.get_creature(CreatureId::from(1));
    /// assert!(creature.is_none());
    /// ```
    pub fn get_creature(&self, id: CreatureId) -> Option<&CreatureDefinition> {
        self.data.creatures.get_creature(id)
    }

    /// Checks if a creature exists in the database
    ///
    /// # Arguments
    ///
    /// * `id` - The creature ID to check
    ///
    /// # Returns
    ///
    /// Returns `true` if the creature exists, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::game_data::GameDataResource;
    /// use antares::domain::campaign_loader::GameData;
    /// use antares::domain::types::CreatureId;
    ///
    /// let resource = GameDataResource::new(GameData::new());
    /// assert!(!resource.has_creature(CreatureId::from(1)));
    /// ```
    pub fn has_creature(&self, id: CreatureId) -> bool {
        self.data.creatures.has_creature(id)
    }

    /// Gets the number of creatures in the database
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::game_data::GameDataResource;
    /// use antares::domain::campaign_loader::GameData;
    ///
    /// let resource = GameDataResource::new(GameData::new());
    /// assert_eq!(resource.creature_count(), 0);
    /// ```
    pub fn creature_count(&self) -> usize {
        self.data.creatures.count()
    }
}

impl Default for GameDataResource {
    fn default() -> Self {
        Self::new(GameData::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_resource() {
        let resource = GameDataResource::new(GameData::new());
        assert!(resource.data().creatures.is_empty());
    }

    #[test]
    fn test_default_resource() {
        let resource = GameDataResource::default();
        assert!(resource.data().creatures.is_empty());
    }

    #[test]
    fn test_get_creature_not_found() {
        let resource = GameDataResource::new(GameData::new());
        assert!(resource.get_creature(1).is_none());
    }

    #[test]
    fn test_has_creature_false() {
        let resource = GameDataResource::new(GameData::new());
        assert!(!resource.has_creature(1));
    }

    #[test]
    fn test_creature_count_zero() {
        let resource = GameDataResource::new(GameData::new());
        assert_eq!(resource.creature_count(), 0);
    }

    #[test]
    fn test_data_access() {
        let resource = GameDataResource::new(GameData::new());
        let data = resource.data();
        assert!(data.creatures.is_empty());
    }

    #[test]
    fn test_data_mut_access() {
        let mut resource = GameDataResource::new(GameData::new());
        let _data = resource.data_mut();
        // Just testing we can get mutable access
    }
}
