// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Furniture-related Bevy components for runtime rendering and interactions

use crate::domain::types::ItemId;
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
    /// Open or close a door panel
    OpenDoor,
}

impl InteractionType {
    /// Get a human-readable name for the interaction
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::components::InteractionType;
    ///
    /// assert_eq!(InteractionType::OpenDoor.name(), "Open Door");
    /// assert_eq!(InteractionType::OpenChest.name(), "Open Chest");
    /// ```
    pub fn name(self) -> &'static str {
        match self {
            Self::OpenChest => "Open Chest",
            Self::SitOnChair => "Sit on Chair",
            Self::LightTorch => "Light Torch",
            Self::ReadBookshelf => "Read Bookshelf",
            Self::OpenDoor => "Open Door",
        }
    }
}

/// Runtime state component for door entities
///
/// Tracks whether the door is open/closed/locked and stores the base
/// rotation needed to restore the closed-door visual. Attached to door
/// entities when they are spawned by `spawn_door()` (via `spawn_furniture`
/// or `spawn_furniture_with_rendering`).
///
/// When `is_open` transitions from `false` to `true` the door's `Transform`
/// is rotated by +90° around Y from `base_rotation_y`. When it transitions
/// back to `false` the rotation is restored to `base_rotation_y`.
///
/// # Examples
///
/// ```
/// use antares::game::components::DoorState;
///
/// let door = DoorState::new(false, 0.0);
/// assert!(!door.is_open);
/// assert!(!door.is_locked);
/// assert!(door.key_item_id.is_none());
/// ```
#[derive(Component, Clone, Debug)]
pub struct DoorState {
    /// Whether the door panel is currently rotated open (90° from base)
    pub is_open: bool,
    /// Whether the door requires a key item to open
    pub is_locked: bool,
    /// Item ID that unlocks this door; `None` means no key can open it
    pub key_item_id: Option<ItemId>,
    /// Y rotation in radians at spawn time — restored when the door closes
    pub base_rotation_y: f32,
}

impl DoorState {
    /// Creates a new door state, initially closed
    ///
    /// # Arguments
    ///
    /// * `is_locked` - Whether the door starts in a locked state
    /// * `base_rotation_y` - Base Y rotation in radians from the door's initial
    ///   `Transform` (i.e. `rotation_y.unwrap_or(0.0).to_radians()`)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::components::DoorState;
    ///
    /// // Unlocked door, facing default direction
    /// let door = DoorState::new(false, 0.0);
    /// assert!(!door.is_locked);
    /// assert!(!door.is_open);
    ///
    /// // Locked door, rotated 90° at spawn
    /// let locked = DoorState::new(true, std::f32::consts::FRAC_PI_2);
    /// assert!(locked.is_locked);
    /// assert!(!locked.is_open);
    /// ```
    pub fn new(is_locked: bool, base_rotation_y: f32) -> Self {
        Self {
            is_open: false,
            is_locked,
            key_item_id: None,
            base_rotation_y,
        }
    }
}

impl Default for DoorState {
    fn default() -> Self {
        Self::new(false, 0.0)
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
        assert_eq!(InteractionType::OpenDoor.name(), "Open Door");
    }

    #[test]
    fn test_open_door_interaction_is_distinct() {
        // Verify OpenDoor is a unique variant, not equal to other interactions
        assert_ne!(InteractionType::OpenDoor, InteractionType::OpenChest);
        assert_ne!(InteractionType::OpenDoor, InteractionType::SitOnChair);
        assert_ne!(InteractionType::OpenDoor, InteractionType::LightTorch);
        assert_ne!(InteractionType::OpenDoor, InteractionType::ReadBookshelf);
    }

    // ── DoorState tests ────────────────────────────────────────────────────

    #[test]
    fn test_door_state_default_is_closed_and_unlocked() {
        let door = DoorState::default();
        assert!(!door.is_open, "default door must start closed");
        assert!(!door.is_locked, "default door must start unlocked");
        assert!(
            door.key_item_id.is_none(),
            "default door has no key requirement"
        );
        assert_eq!(door.base_rotation_y, 0.0, "default base rotation is zero");
    }

    #[test]
    fn test_door_state_new_unlocked() {
        let door = DoorState::new(false, 0.0);
        assert!(!door.is_locked);
        assert!(!door.is_open);
    }

    #[test]
    fn test_door_state_new_locked() {
        let door = DoorState::new(true, 0.0);
        assert!(door.is_locked);
        assert!(!door.is_open, "locked door starts closed");
    }

    #[test]
    fn test_door_state_base_rotation_preserved() {
        let angle = std::f32::consts::FRAC_PI_2;
        let door = DoorState::new(false, angle);
        assert!((door.base_rotation_y - angle).abs() < 1e-6);
    }

    #[test]
    fn test_door_state_key_item_id_none_by_default() {
        let door = DoorState::new(true, 0.0);
        assert!(
            door.key_item_id.is_none(),
            "new door with locked=true has no key until explicitly set"
        );
    }

    #[test]
    fn test_door_state_key_item_id_can_be_set() {
        let mut door = DoorState::new(true, 0.0);
        door.key_item_id = Some(42);
        assert_eq!(door.key_item_id, Some(42));
    }

    #[test]
    fn test_door_state_toggle_open_changes_blocking_concept() {
        // Simulate what the input system does: toggle is_open and derive blocking
        let mut door = DoorState::new(false, 0.0);
        assert!(!door.is_open);

        // Open the door
        door.is_open = true;
        let blocking_when_open = !door.is_open;
        assert!(!blocking_when_open, "open door must not block");

        // Close the door
        door.is_open = false;
        let blocking_when_closed = !door.is_open;
        assert!(blocking_when_closed, "closed door must block");
    }

    #[test]
    fn test_door_state_open_rotation_offset() {
        // When opened, the rotation should be base + 90°
        let base = std::f32::consts::PI; // 180°
        let door = DoorState::new(false, base);
        let open_angle = door.base_rotation_y + std::f32::consts::FRAC_PI_2;
        let expected = base + std::f32::consts::FRAC_PI_2; // 270°
        assert!((open_angle - expected).abs() < 1e-6);
    }
}
