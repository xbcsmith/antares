// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Furniture-related Bevy components for runtime rendering and interactions

use crate::domain::world::FurnitureType;
use bevy::prelude::*;

/// Marker component for furniture entities spawned from MapEvent::Furniture
///
/// This component identifies an entity as a furniture piece and tracks
/// its type and blocking behavior for collision systems.
///
/// # Examples
///
/// ```text
/// // When spawning a bench from a MapEvent
/// let furniture_entity = commands
///     .spawn(PbrBundle { /* ... */ })
///     .insert(FurnitureEntity {
///         furniture_type: FurnitureType::Bench,
///         blocking: true,
///     });
/// ```
#[derive(Component, Clone, Debug)]
pub struct FurnitureEntity {
    /// The type of furniture (Chair, Table, Torch, etc.)
    pub furniture_type: FurnitureType,
    /// Whether this furniture blocks movement/pathing
    pub blocking: bool,
}

impl FurnitureEntity {
    /// Creates a new furniture entity marker
    ///
    /// # Arguments
    ///
    /// * `furniture_type` - The type of furniture
    /// * `blocking` - Whether the furniture is blocking
    ///
    /// # Examples
    ///
    /// ```text
    /// let chair = FurnitureEntity::new(FurnitureType::Chair, false);
    /// let chest = FurnitureEntity::new(FurnitureType::Chest, true);
    /// ```
    pub fn new(furniture_type: FurnitureType, blocking: bool) -> Self {
        Self {
            furniture_type,
            blocking,
        }
    }
}

/// Component marking an entity as interactable with the player
///
/// Furniture and other entities can be interacted with by pressing
/// the interact key when the player is within interaction_distance.
/// The interaction_type determines what happens when interaction occurs.
///
/// # Examples
///
/// ```text
/// // Interactable chest
/// .insert(Interactable {
///     interaction_type: InteractionType::OpenChest,
///     interaction_distance: 2.0,
/// })
///
/// // Interactable torch
/// .insert(Interactable {
///     interaction_type: InteractionType::LightTorch,
///     interaction_distance: 1.5,
/// })
/// ```
#[derive(Component, Clone, Debug)]
pub struct Interactable {
    /// Type of interaction this entity supports
    pub interaction_type: InteractionType,
    /// Maximum distance from player for interaction (in world units)
    pub interaction_distance: f32,
}

impl Interactable {
    /// Creates a new interactable with default distance
    ///
    /// # Arguments
    ///
    /// * `interaction_type` - Type of interaction supported
    ///
    /// # Examples
    ///
    /// ```text
    /// let interactable = Interactable::new(InteractionType::SitOnChair);
    /// ```
    pub fn new(interaction_type: InteractionType) -> Self {
        Self {
            interaction_type,
            interaction_distance: Self::DEFAULT_DISTANCE,
        }
    }

    /// Creates a new interactable with custom distance
    ///
    /// # Arguments
    ///
    /// * `interaction_type` - Type of interaction supported
    /// * `distance` - Custom interaction distance
    pub fn with_distance(interaction_type: InteractionType, distance: f32) -> Self {
        Self {
            interaction_type,
            interaction_distance: distance,
        }
    }

    /// Default interaction distance (in world units)
    pub const DEFAULT_DISTANCE: f32 = 2.0;
}

/// Types of interactions supported by furniture
///
/// Each interaction type represents a different action that can be
/// performed on furniture when the player is nearby and presses
/// the interact key.
///
/// # Examples
///
/// ```text
/// match interaction_type {
///     InteractionType::OpenChest => { /* open chest */ }
///     InteractionType::SitOnChair => { /* play sitting animation */ }
///     InteractionType::LightTorch => { /* toggle torch lit state */ }
///     InteractionType::ReadBookshelf => { /* show book list */ }
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InteractionType {
    /// Open a chest or container to access inventory
    OpenChest,
    /// Sit on a chair (visual/animation only)
    SitOnChair,
    /// Light/extinguish a torch (toggles lit state)
    LightTorch,
    /// Read from a bookshelf (display book list)
    ReadBookshelf,
}

impl InteractionType {
    /// Get a human-readable name for the interaction
    pub fn name(self) -> &'static str {
        match self {
            Self::OpenChest => "Open Chest",
            Self::SitOnChair => "Sit on Chair",
            Self::LightTorch => "Light Torch",
            Self::ReadBookshelf => "Read Bookshelf",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_furniture_entity_creation() {
        let furniture = FurnitureEntity::new(FurnitureType::Bench, true);
        assert_eq!(furniture.furniture_type, FurnitureType::Bench);
        assert!(furniture.blocking);
    }

    #[test]
    fn test_furniture_entity_non_blocking() {
        let furniture = FurnitureEntity::new(FurnitureType::Torch, false);
        assert_eq!(furniture.furniture_type, FurnitureType::Torch);
        assert!(!furniture.blocking);
    }

    #[test]
    fn test_interactable_default_distance() {
        let interactable = Interactable::new(InteractionType::OpenChest);
        assert_eq!(
            interactable.interaction_distance,
            Interactable::DEFAULT_DISTANCE
        );
        assert_eq!(interactable.interaction_type, InteractionType::OpenChest);
    }

    #[test]
    fn test_interactable_custom_distance() {
        let interactable = Interactable::with_distance(InteractionType::LightTorch, 1.5);
        assert_eq!(interactable.interaction_distance, 1.5);
        assert_eq!(interactable.interaction_type, InteractionType::LightTorch);
    }

    #[test]
    fn test_interaction_type_names() {
        assert_eq!(InteractionType::OpenChest.name(), "Open Chest");
        assert_eq!(InteractionType::SitOnChair.name(), "Sit on Chair");
        assert_eq!(InteractionType::LightTorch.name(), "Light Torch");
        assert_eq!(InteractionType::ReadBookshelf.name(), "Read Bookshelf");
    }
}
