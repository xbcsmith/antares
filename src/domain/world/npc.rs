// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! NPC (Non-Player Character) definitions and placement
//!
//! This module provides data structures for defining NPCs and placing them on maps.
//! NPCs are separated into:
//! - `NpcDefinition`: The reusable NPC data (name, portrait, dialogue, etc.)
//! - `NpcPlacement`: A reference to an NPC definition at a specific map location
//!
//! This design allows the same NPC to appear on multiple maps without duplicating data.
//!
//! # Examples
//!
//! ```
//! use antares::domain::world::npc::{NpcDefinition, NpcPlacement};
//! use antares::domain::types::Position;
//!
//! // Define an NPC
//! let npc = NpcDefinition {
//!     id: "village_elder".to_string(),
//!     name: "Elder Theron".to_string(),
//!     description: "The wise village elder".to_string(),
//!     portrait_id: "elder".to_string(),
//!     sprite: None,
//!     dialogue_id: Some(1),
//!     quest_ids: vec![1, 2],
//!     faction: Some("Village Council".to_string()),
//!     is_merchant: false,
//!     is_innkeeper: false,
//! };
//! ```
//!
//! // Place the NPC on a map
//! let placement = NpcPlacement {
//!     npc_id: "village_elder".to_string(),
//!     position: Position::new(10, 15),
//!     facing: None,
//!     dialogue_override: None,
//! };
//! ```

use crate::domain::dialogue::DialogueId;
use crate::domain::quest::QuestId;
use crate::domain::types::{Direction, Position};
use crate::domain::world::SpriteReference;
use serde::{Deserialize, Serialize};

/// NPC identifier
///
/// Uses human-readable string IDs for better debugging and editor UX.
/// Examples: "village_elder", "merchant_tom", "high_priestess"
pub type NpcId = String;

/// NPC definition containing all reusable NPC data
///
/// This struct defines an NPC's identity, appearance, and behavior.
/// NPC definitions are stored in `npcs.ron` files and referenced by ID
/// from map placements.
///
/// # Examples
///
/// ```
/// use antares::domain::world::npc::NpcDefinition;
///
/// let merchant = NpcDefinition {
///     id: "merchant_tom".to_string(),
///     name: "Tom the Merchant".to_string(),
///     description: "A friendly traveling merchant".to_string(),
///     portrait_id: "assets/portraits/merchant.png".to_string(),
///     sprite: None,
///     dialogue_id: Some(5),
///     quest_ids: vec![],
///     faction: Some("Merchants Guild".to_string()),
///     is_merchant: true,
///     is_innkeeper: false,
/// };
///
/// assert_eq!(merchant.id, "merchant_tom");
/// assert!(merchant.is_merchant);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NpcDefinition {
    /// Unique identifier (e.g., "village_elder", "merchant_tom")
    pub id: NpcId,

    /// Display name shown to player
    pub name: String,

    /// Description for tooltips and inspection
    #[serde(default)]
    pub description: String,

    /// Path to portrait image (required)
    pub portrait_id: String,

    /// Optional default dialogue id for NPC interactions
    #[serde(default)]
    pub dialogue_id: Option<DialogueId>,

    /// Optional sprite reference for this NPC's visual representation.
    ///
    /// When `Some`, the NPC will use the specified sprite sheet and index.
    /// When `None`, falls back to `DEFAULT_NPC_SPRITE_PATH` (placeholder).
    ///
    /// Backward compatibility: Old RON files without this field will deserialize
    /// with `sprite = None` via `#[serde(default)]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc::NpcDefinition;
    /// use antares::domain::world::SpriteReference;
    ///
    /// let npc = NpcDefinition {
    ///     id: "guard_001".to_string(),
    ///     name: "City Guard".to_string(),
    ///     description: "A vigilant guard".to_string(),
    ///     portrait_id: "guard.png".to_string(),
    ///     dialogue_id: None,
    ///     quest_ids: vec![],
    ///     faction: None,
    ///     is_merchant: false,
    ///     is_innkeeper: false,
    ///     sprite: None,
    /// };
    /// ```
    #[serde(default)]
    pub sprite: Option<SpriteReference>,

    /// Quests this NPC gives or is involved with
    #[serde(default)]
    pub quest_ids: Vec<QuestId>,

    /// Optional faction affiliation
    #[serde(default)]
    pub faction: Option<String>,

    /// If true, this NPC can open shop interface
    #[serde(default)]
    pub is_merchant: bool,

    /// If true, this NPC can rest party (inn/tavern)
    #[serde(default)]
    pub is_innkeeper: bool,
}

impl NpcDefinition {
    /// Creates a new basic NPC definition
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the NPC
    /// * `name` - Display name
    /// * `portrait_id` - Path to portrait image
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc::NpcDefinition;
    ///
    /// let npc = NpcDefinition::new(
    ///     "guard_001",
    ///     "City Guard",
    ///     "assets/portraits/guard.png"
    /// );
    ///
    /// assert_eq!(npc.id, "guard_001");
    /// assert_eq!(npc.name, "City Guard");
    /// assert!(!npc.is_merchant);
    /// ```
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        portrait_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            portrait_id: portrait_id.into(),
            sprite: None,
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
        }
    }

    /// Creates a merchant NPC
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc::NpcDefinition;
    ///
    /// let merchant = NpcDefinition::merchant(
    ///     "merchant_bob",
    ///     "Bob's Goods",
    ///     "assets/portraits/merchant.png"
    /// );
    ///
    /// assert!(merchant.is_merchant);
    /// assert!(!merchant.is_innkeeper);
    /// ```
    pub fn merchant(
        id: impl Into<String>,
        name: impl Into<String>,
        portrait_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            portrait_id: portrait_id.into(),
            sprite: None,
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
        }
    }

    /// Creates an innkeeper NPC
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc::NpcDefinition;
    ///
    /// let innkeeper = NpcDefinition::innkeeper(
    ///     "innkeeper_mary",
    ///     "Mary's Inn",
    ///     "assets/portraits/innkeeper.png"
    /// );
    ///
    /// assert!(innkeeper.is_innkeeper);
    /// assert!(!innkeeper.is_merchant);
    /// ```
    pub fn innkeeper(
        id: impl Into<String>,
        name: impl Into<String>,
        portrait_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            portrait_id: portrait_id.into(),
            sprite: None,
            dialogue_id: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: true,
        }
    }

    /// Checks if this NPC has a dialogue
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc::NpcDefinition;
    ///
    /// let mut npc = NpcDefinition::new("test", "Test NPC", "test.png");
    /// assert!(!npc.has_dialogue());
    ///
    /// npc.dialogue_id = Some(1);
    /// assert!(npc.has_dialogue());
    /// ```
    pub fn has_dialogue(&self) -> bool {
        self.dialogue_id.is_some()
    }

    /// Checks if this NPC gives quests
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc::NpcDefinition;
    ///
    /// let mut npc = NpcDefinition::new("test", "Test NPC", "test.png");
    /// assert!(!npc.gives_quests());
    ///
    /// npc.quest_ids.push(1);
    /// assert!(npc.gives_quests());
    /// ```
    pub fn gives_quests(&self) -> bool {
        !self.quest_ids.is_empty()
    }

    /// Sets the sprite reference for this NPC (builder pattern).
    ///
    /// # Arguments
    ///
    /// * `sprite` - The sprite reference to use for this NPC
    ///
    /// # Returns
    ///
    /// Self with sprite field set
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc::NpcDefinition;
    /// use antares::domain::world::SpriteReference;
    ///
    /// let sprite = SpriteReference {
    ///     sheet_path: "sprites/actors/npcs_town.png".to_string(),
    ///     sprite_index: 2,
    ///     animation: None,
    ///     material_properties: None,
    /// };
    ///
    /// let npc = NpcDefinition::new("Guard", "City Guard", "guard.png")
    ///     .with_sprite(sprite);
    /// ```
    pub fn with_sprite(mut self, sprite: SpriteReference) -> Self {
        self.sprite = Some(sprite);
        self
    }
}

/// NPC placement on a map
///
/// This lightweight struct references an NPC definition and specifies where
/// it appears on a specific map. Multiple placements can reference the same
/// NPC definition.
///
/// # Examples
///
/// ```
/// use antares::domain::world::npc::NpcPlacement;
/// use antares::domain::types::{Position, Direction};
///
/// let placement = NpcPlacement {
///     npc_id: "village_elder".to_string(),
///     position: Position::new(10, 15),
///     facing: Some(Direction::South),
///     dialogue_override: None,
/// };
///
/// assert_eq!(placement.npc_id, "village_elder");
/// assert_eq!(placement.position.x, 10);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NpcPlacement {
    /// Reference to NpcDefinition.id
    pub npc_id: NpcId,

    /// Position on the map
    pub position: Position,

    /// Optional direction the NPC faces
    #[serde(default)]
    pub facing: Option<Direction>,

    /// Override default dialogue for this specific placement
    #[serde(default)]
    pub dialogue_override: Option<DialogueId>,
}

impl NpcPlacement {
    /// Creates a new NPC placement
    ///
    /// # Arguments
    ///
    /// * `npc_id` - Reference to an NPC definition
    /// * `position` - Position on the map
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc::NpcPlacement;
    /// use antares::domain::types::Position;
    ///
    /// let placement = NpcPlacement::new("merchant_tom", Position::new(5, 10));
    ///
    /// assert_eq!(placement.npc_id, "merchant_tom");
    /// assert_eq!(placement.position.x, 5);
    /// assert_eq!(placement.position.y, 10);
    /// ```
    pub fn new(npc_id: impl Into<String>, position: Position) -> Self {
        Self {
            npc_id: npc_id.into(),
            position,
            facing: None,
            dialogue_override: None,
        }
    }

    /// Creates a new NPC placement with a facing direction
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc::NpcPlacement;
    /// use antares::domain::types::{Position, Direction};
    ///
    /// let placement = NpcPlacement::with_facing(
    ///     "guard",
    ///     Position::new(0, 0),
    ///     Direction::North
    /// );
    ///
    /// assert_eq!(placement.facing, Some(Direction::North));
    /// ```
    pub fn with_facing(npc_id: impl Into<String>, position: Position, facing: Direction) -> Self {
        Self {
            npc_id: npc_id.into(),
            position,
            facing: Some(facing),
            dialogue_override: None,
        }
    }

    /// Checks if this placement has a dialogue override
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc::NpcPlacement;
    /// use antares::domain::types::Position;
    ///
    /// let mut placement = NpcPlacement::new("test", Position::new(0, 0));
    /// assert!(!placement.has_dialogue_override());
    ///
    /// placement.dialogue_override = Some(42);
    /// assert!(placement.has_dialogue_override());
    /// ```
    pub fn has_dialogue_override(&self) -> bool {
        self.dialogue_override.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npc_definition_new() {
        let npc = NpcDefinition::new("test_npc", "Test NPC", "test.png");

        assert_eq!(npc.id, "test_npc");
        assert_eq!(npc.name, "Test NPC");
        assert_eq!(npc.portrait_id, "test.png");
        assert_eq!(npc.description, "");
        assert_eq!(npc.dialogue_id, None);
        assert_eq!(npc.quest_ids.len(), 0);
        assert_eq!(npc.faction, None);
        assert!(!npc.is_merchant);
        assert!(!npc.is_innkeeper);
    }

    #[test]
    fn test_npc_definition_merchant() {
        let merchant = NpcDefinition::merchant("merchant_1", "Merchant Bob", "merchant.png");

        assert_eq!(merchant.id, "merchant_1");
        assert!(merchant.is_merchant);
        assert!(!merchant.is_innkeeper);
    }

    #[test]
    fn test_npc_definition_innkeeper() {
        let innkeeper = NpcDefinition::innkeeper("inn_1", "Innkeeper Mary", "innkeeper.png");

        assert_eq!(innkeeper.id, "inn_1");
        assert!(!innkeeper.is_merchant);
        assert!(innkeeper.is_innkeeper);
    }

    #[test]
    fn test_npc_definition_has_dialogue() {
        let mut npc = NpcDefinition::new("test", "Test", "test.png");
        assert!(!npc.has_dialogue());

        npc.dialogue_id = Some(1);
        assert!(npc.has_dialogue());
    }

    #[test]
    fn test_npc_definition_gives_quests() {
        let mut npc = NpcDefinition::new("test", "Test", "test.png");
        assert!(!npc.gives_quests());

        npc.quest_ids.push(1);
        assert!(npc.gives_quests());

        npc.quest_ids.push(2);
        assert!(npc.gives_quests());
        assert_eq!(npc.quest_ids.len(), 2);
    }

    #[test]
    fn test_npc_definition_serialization() {
        let npc = NpcDefinition {
            id: "elder".to_string(),
            name: "Village Elder".to_string(),
            description: "Wise elder".to_string(),
            portrait_id: "elder.png".to_string(),
            sprite: None,
            dialogue_id: Some(1),
            quest_ids: vec![1, 2, 3],
            faction: Some("Village".to_string()),
            is_merchant: false,
            is_innkeeper: false,
        };

        let serialized = ron::to_string(&npc).expect("Failed to serialize");
        let deserialized: NpcDefinition =
            ron::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(npc, deserialized);
    }

    #[test]
    fn test_npc_definition_serialization_defaults() {
        let npc = NpcDefinition::new("test", "Test", "test.png");

        let serialized = ron::to_string(&npc).expect("Failed to serialize");
        let deserialized: NpcDefinition =
            ron::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(npc, deserialized);
        assert_eq!(deserialized.description, "");
        assert_eq!(deserialized.dialogue_id, None);
        assert_eq!(deserialized.quest_ids.len(), 0);
    }

    #[test]
    fn test_npc_definition_serializes_with_sprite_field_present() {
        let sprite = crate::domain::world::SpriteReference {
            sheet_path: "sprites/test/custom.png".to_string(),
            sprite_index: 42,
            animation: None,
            material_properties: None,
        };
        let npc = NpcDefinition::new("test_npc", "Test NPC", "test.png").with_sprite(sprite);

        let ron_str = ron::to_string(&npc).expect("Failed to serialize to RON");
        let deserialized: NpcDefinition =
            ron::from_str(&ron_str).expect("Failed to deserialize from RON");

        assert!(deserialized.sprite.is_some());
        assert_eq!(
            deserialized.sprite.as_ref().unwrap().sheet_path,
            "sprites/test/custom.png"
        );
        assert_eq!(deserialized.sprite.as_ref().unwrap().sprite_index, 42);
    }

    #[test]
    fn test_npc_definition_deserializes_without_sprite_field_defaults_none() {
        let ron_str = r#"
NpcDefinition(
    id: "old_npc",
    name: "Old NPC",
    portrait_id: "portrait.png",
)
"#;
        let npc: NpcDefinition = ron::from_str(ron_str).expect("Failed to deserialize old format");
        assert!(npc.sprite.is_none());
        assert_eq!(npc.name, "Old NPC");
    }

    #[test]
    fn test_npc_placement_new() {
        let placement = NpcPlacement::new("test_npc", Position::new(5, 10));

        assert_eq!(placement.npc_id, "test_npc");
        assert_eq!(placement.position.x, 5);
        assert_eq!(placement.position.y, 10);
        assert_eq!(placement.facing, None);
        assert_eq!(placement.dialogue_override, None);
    }

    #[test]
    fn test_npc_placement_with_facing() {
        let placement = NpcPlacement::with_facing("guard", Position::new(0, 0), Direction::North);

        assert_eq!(placement.npc_id, "guard");
        assert_eq!(placement.facing, Some(Direction::North));
    }

    #[test]
    fn test_npc_placement_has_dialogue_override() {
        let mut placement = NpcPlacement::new("test", Position::new(0, 0));
        assert!(!placement.has_dialogue_override());

        placement.dialogue_override = Some(99);
        assert!(placement.has_dialogue_override());
    }

    #[test]
    fn test_npc_placement_serialization() {
        let placement = NpcPlacement {
            npc_id: "merchant".to_string(),
            position: Position::new(15, 20),
            facing: Some(Direction::South),
            dialogue_override: Some(42),
        };

        let serialized = ron::to_string(&placement).expect("Failed to serialize");
        let deserialized: NpcPlacement = ron::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(placement, deserialized);
    }

    #[test]
    fn test_npc_placement_serialization_defaults() {
        let placement = NpcPlacement::new("test", Position::new(1, 2));

        let serialized = ron::to_string(&placement).expect("Failed to serialize");
        let deserialized: NpcPlacement = ron::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(placement, deserialized);
        assert_eq!(deserialized.facing, None);
        assert_eq!(deserialized.dialogue_override, None);
    }

    #[test]
    fn test_npc_id_string_format() {
        let npc = NpcDefinition::new("village_elder", "Elder", "elder.png");
        assert_eq!(npc.id, "village_elder");

        let placement = NpcPlacement::new("village_elder", Position::new(0, 0));
        assert_eq!(placement.npc_id, npc.id);
    }

    #[test]
    fn test_npc_definition_with_all_fields() {
        let npc = NpcDefinition {
            id: "complete_npc".to_string(),
            name: "Complete NPC".to_string(),
            description: "An NPC with all fields set".to_string(),
            portrait_id: "complete.png".to_string(),
            sprite: None,
            dialogue_id: Some(10),
            quest_ids: vec![1, 2, 3, 4],
            faction: Some("Test Faction".to_string()),
            is_merchant: true,
            is_innkeeper: true,
        };

        assert_eq!(npc.id, "complete_npc");
        assert_eq!(npc.name, "Complete NPC");
        assert_eq!(npc.description, "An NPC with all fields set");
        assert_eq!(npc.dialogue_id, Some(10));
        assert_eq!(npc.quest_ids.len(), 4);
        assert_eq!(npc.faction, Some("Test Faction".to_string()));
        assert!(npc.is_merchant);
        assert!(npc.is_innkeeper);
        assert!(npc.has_dialogue());
        assert!(npc.gives_quests());
    }

    #[test]
    fn test_npc_placement_different_positions() {
        let placement1 = NpcPlacement::new("npc", Position::new(0, 0));
        let placement2 = NpcPlacement::new("npc", Position::new(10, 10));

        assert_eq!(placement1.npc_id, placement2.npc_id);
        assert_ne!(placement1.position, placement2.position);
    }
}
