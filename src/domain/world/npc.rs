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
//!     dialogue_id: Some(1),
//!     creature_id: None,
//!     sprite: None,
//!     quest_ids: vec![1, 2],
//!     faction: Some("Village Council".to_string()),
//!     is_merchant: false,
//!     is_innkeeper: false,
//!     is_priest: false,
//!     is_trainer: false,
//!     stock_template: None,
//!     service_catalog: None,
//!     economy: None,
//!     training_fee_base: None,
//!     training_fee_multiplier: None,
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
use crate::domain::inventory::{NpcEconomySettings, ServiceCatalog};
use crate::domain::quest::QuestId;
use crate::domain::types::{CreatureId, Direction, Position};
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
///     creature_id: None,
///     quest_ids: vec![],
///     faction: Some("Merchants Guild".to_string()),
///     is_merchant: true,
///     is_innkeeper: false,
///     is_priest: false,
///     is_trainer: false,
///     stock_template: Some("general_goods".to_string()),
///     service_catalog: None,
///     economy: None,
///     training_fee_base: None,
///     training_fee_multiplier: None,
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

    /// Optional creature visual reference for procedural mesh rendering.
    ///
    /// When `Some`, the NPC will use the specified creature's procedural mesh definition
    /// for 3D rendering. This integrates NPCs with the same visual system used for monsters.
    /// When `None`, falls back to sprite-based rendering (if sprite is set) or default visuals.
    ///
    /// Backward compatibility: Old RON files without this field will deserialize
    /// with `creature_id = None` via `#[serde(default)]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc::NpcDefinition;
    ///
    /// let npc = NpcDefinition {
    ///     id: "village_elder".to_string(),
    ///     name: "Elder Theron".to_string(),
    ///     description: "The wise village elder".to_string(),
    ///     portrait_id: "elder.png".to_string(),
    ///     dialogue_id: None,
    ///     creature_id: Some(54), // VillageElder creature
    ///     sprite: None,
    ///     quest_ids: vec![],
    ///     faction: None,
    ///     is_merchant: false,
    ///     is_innkeeper: false,
    ///     is_priest: false,
    ///     is_trainer: false,
    ///     stock_template: None,
    ///     service_catalog: None,
    ///     economy: None,
    ///     training_fee_base: None,
    ///     training_fee_multiplier: None,
    /// };
    /// ```
    #[serde(default)]
    pub creature_id: Option<CreatureId>,

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
    ///     creature_id: None,
    ///     sprite: None,
    ///     quest_ids: vec![],
    ///     faction: None,
    ///     is_merchant: false,
    ///     is_innkeeper: false,
    ///     is_priest: false,
    ///     is_trainer: false,
    ///     stock_template: None,
    ///     service_catalog: None,
    ///     economy: None,
    ///     training_fee_base: None,
    ///     training_fee_multiplier: None,
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

    /// If true, this NPC offers priest services (healing, condition curing, restoration)
    #[serde(default)]
    pub is_priest: bool,

    /// Optional ID referencing a merchant stock template in campaign data.
    ///
    /// Used to initialize the NPC's `MerchantStock` at runtime. For example,
    /// `"blacksmith_basic_stock"` would look up the corresponding template in
    /// `npc_stock_templates.ron`. When `None`, the merchant starts with no stock.
    ///
    /// This is static definition data. Runtime mutable stock quantities live in
    /// `NpcRuntimeState`.
    #[serde(default)]
    pub stock_template: Option<String>,

    /// Optional inline service catalog for priest or innkeeper NPCs.
    ///
    /// Defines what paid services (healing, curing, resting, etc.) this NPC
    /// provides and at what cost. When `None`, the NPC offers no services.
    ///
    /// This is static definition data read at game load time.
    #[serde(default)]
    pub service_catalog: Option<ServiceCatalog>,

    /// Optional per-NPC buy/sell rate overrides.
    ///
    /// When `Some`, these rates override the campaign-wide defaults for this
    /// specific NPC. When `None`, the NPC uses the campaign default economy
    /// settings (buy 50%, sell 100%).
    #[serde(default)]
    pub economy: Option<NpcEconomySettings>,

    /// If true, this NPC offers character level-up training for a gold fee.
    #[serde(default)]
    pub is_trainer: bool,

    /// Per-NPC override for the base gold fee charged per training session.
    ///
    /// When `Some`, overrides [`crate::domain::campaign::CampaignConfig::training_fee_base`].
    /// When `None`, the campaign default is used.
    #[serde(default)]
    pub training_fee_base: Option<u32>,

    /// Per-NPC override for the per-level fee multiplier applied during training.
    ///
    /// When `Some`, overrides [`crate::domain::campaign::CampaignConfig::training_fee_multiplier`].
    /// When `None`, the campaign default is used.
    #[serde(default)]
    pub training_fee_multiplier: Option<f32>,
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
            dialogue_id: None,
            creature_id: None,
            sprite: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            is_trainer: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            training_fee_base: None,
            training_fee_multiplier: None,
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
    /// assert!(!merchant.is_priest);
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
            dialogue_id: None,
            creature_id: None,
            sprite: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
            is_priest: false,
            is_trainer: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            training_fee_base: None,
            training_fee_multiplier: None,
        }
    }

    /// Creates a priest NPC
    ///
    /// Priests offer paid services such as healing, condition curing, and
    /// resurrection. Use the `service_catalog` field to define what services
    /// this priest provides.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc::NpcDefinition;
    ///
    /// let priest = NpcDefinition::priest(
    ///     "high_priest_alaric",
    ///     "High Priest Alaric",
    ///     "assets/portraits/priest.png"
    /// );
    ///
    /// assert!(priest.is_priest);
    /// assert!(!priest.is_merchant);
    /// assert!(!priest.is_innkeeper);
    /// ```
    pub fn priest(
        id: impl Into<String>,
        name: impl Into<String>,
        portrait_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            portrait_id: portrait_id.into(),
            dialogue_id: None,
            creature_id: None,
            sprite: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: true,
            is_trainer: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            training_fee_base: None,
            training_fee_multiplier: None,
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
    /// assert!(!innkeeper.is_priest);
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
            dialogue_id: None,
            creature_id: None,
            sprite: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: true,
            is_priest: false,
            is_trainer: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            training_fee_base: None,
            training_fee_multiplier: None,
        }
    }

    /// Creates a trainer NPC that charges a level-up fee.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier
    /// * `name` - Display name
    /// * `portrait_id` - Portrait image path
    /// * `fee_base` - Base gold fee per training session (overrides campaign default)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc::NpcDefinition;
    ///
    /// let trainer = NpcDefinition::trainer(
    ///     "master_swordsman",
    ///     "Master Swordsman",
    ///     "assets/portraits/trainer.png",
    ///     300,
    /// );
    ///
    /// assert!(trainer.is_trainer);
    /// assert!(!trainer.is_merchant);
    /// assert_eq!(trainer.training_fee_base, Some(300));
    /// ```
    pub fn trainer(
        id: impl Into<String>,
        name: impl Into<String>,
        portrait_id: impl Into<String>,
        fee_base: u32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            portrait_id: portrait_id.into(),
            dialogue_id: None,
            creature_id: None,
            sprite: None,
            quest_ids: Vec::new(),
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            is_trainer: true,
            stock_template: None,
            service_catalog: None,
            economy: None,
            training_fee_base: Some(fee_base),
            training_fee_multiplier: None,
        }
    }

    /// Computes the gold fee for training a character from `level` to `level + 1`.
    ///
    /// Uses the NPC's override values when present, otherwise falls back to
    /// the campaign-wide defaults from `campaign_config`.
    ///
    /// Formula: `floor(base * multiplier * level)`
    ///
    /// # Arguments
    ///
    /// * `level` - The character's **current** level (fee is for advancing beyond it)
    /// * `campaign_config` - Campaign configuration supplying defaults when no
    ///   NPC override is set
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc::NpcDefinition;
    /// use antares::domain::campaign::CampaignConfig;
    ///
    /// let config = CampaignConfig::default(); // training_fee_base=500, multiplier=1.0
    ///
    /// // NPC with no override — uses campaign defaults
    /// let trainer = NpcDefinition::trainer("t", "T", "t.png", 500);
    /// assert_eq!(trainer.training_fee_for_level(1, &config), 500);
    /// assert_eq!(trainer.training_fee_for_level(5, &config), 2500);
    ///
    /// // NPC with override
    /// let mut custom = NpcDefinition::trainer("t2", "T2", "t2.png", 300);
    /// custom.training_fee_multiplier = Some(2.0);
    /// assert_eq!(custom.training_fee_for_level(1, &config), 600);
    /// ```
    pub fn training_fee_for_level(
        &self,
        level: u32,
        campaign_config: &crate::domain::campaign::CampaignConfig,
    ) -> u32 {
        let base = self
            .training_fee_base
            .unwrap_or(campaign_config.training_fee_base);
        let multiplier = self
            .training_fee_multiplier
            .unwrap_or(campaign_config.training_fee_multiplier);
        (base as f32 * multiplier * level as f32) as u32
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

    /// Sets the creature visual reference for this NPC (builder pattern).
    ///
    /// # Arguments
    ///
    /// * `creature_id` - The creature ID to use for procedural mesh rendering
    ///
    /// # Returns
    ///
    /// Self with creature_id field set
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc::NpcDefinition;
    ///
    /// let elder = NpcDefinition::new("elder", "Village Elder", "elder.png")
    ///     .with_creature_id(54); // VillageElder creature mesh
    /// assert_eq!(elder.creature_id, Some(54));
    /// ```
    pub fn with_creature_id(mut self, creature_id: CreatureId) -> Self {
        self.creature_id = Some(creature_id);
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
            dialogue_id: Some(1),
            creature_id: None,
            sprite: None,
            quest_ids: vec![1, 2, 3],
            faction: Some("Village".to_string()),
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            is_trainer: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            training_fee_base: None,
            training_fee_multiplier: None,
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
            dialogue_id: Some(10),
            creature_id: None,
            sprite: None,
            quest_ids: vec![1, 2, 3, 4],
            faction: Some("Test Faction".to_string()),
            is_merchant: true,
            is_innkeeper: true,
            is_priest: false,
            is_trainer: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            training_fee_base: None,
            training_fee_multiplier: None,
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

    #[test]
    fn test_npc_definition_with_creature_id() {
        let npc = NpcDefinition::new("elder", "Village Elder", "elder.png").with_creature_id(54);

        assert_eq!(npc.creature_id, Some(54));
        assert_eq!(npc.id, "elder");
        assert_eq!(npc.name, "Village Elder");
    }

    #[test]
    fn test_npc_definition_creature_id_serialization() {
        let npc = NpcDefinition {
            id: "wizard".to_string(),
            name: "Wizard Arcturus".to_string(),
            description: "A powerful wizard".to_string(),
            portrait_id: "wizard.png".to_string(),
            dialogue_id: Some(1),
            creature_id: Some(58), // WizardArcturus creature
            sprite: None,
            quest_ids: vec![],
            faction: Some("Wizards".to_string()),
            is_merchant: false,
            is_innkeeper: false,
            is_priest: false,
            is_trainer: false,
            stock_template: None,
            service_catalog: None,
            economy: None,
            training_fee_base: None,
            training_fee_multiplier: None,
        };

        let serialized = ron::to_string(&npc).expect("Failed to serialize");
        let deserialized: NpcDefinition =
            ron::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(npc, deserialized);
        assert_eq!(deserialized.creature_id, Some(58));
    }

    #[test]
    fn test_npc_definition_deserializes_without_creature_id_defaults_none() {
        let ron_str = r#"
NpcDefinition(
    id: "old_npc",
    name: "Old NPC",
    portrait_id: "portrait.png",
)
"#;
        let npc: NpcDefinition = ron::from_str(ron_str).expect("Failed to deserialize old format");
        assert!(npc.creature_id.is_none());
        assert_eq!(npc.name, "Old NPC");
    }

    #[test]
    fn test_npc_definition_with_both_creature_and_sprite() {
        let sprite = crate::domain::world::SpriteReference {
            sheet_path: "sprites/npcs.png".to_string(),
            sprite_index: 5,
            animation: None,
            material_properties: None,
        };

        let npc = NpcDefinition::new("merchant", "Merchant", "merchant.png")
            .with_creature_id(53) // Merchant creature
            .with_sprite(sprite);

        assert_eq!(npc.creature_id, Some(53));
        assert!(npc.sprite.is_some());
        assert_eq!(npc.sprite.as_ref().unwrap().sprite_index, 5);
    }

    #[test]
    fn test_npc_definition_defaults_have_no_creature_id() {
        let npc = NpcDefinition::new("test", "Test NPC", "test.png");
        assert!(npc.creature_id.is_none());

        let merchant = NpcDefinition::merchant("m1", "Merchant", "m.png");
        assert!(merchant.creature_id.is_none());

        let innkeeper = NpcDefinition::innkeeper("i1", "Innkeeper", "i.png");
        assert!(innkeeper.creature_id.is_none());
    }

    // ----- NpcDefinition field tests -----

    #[test]
    fn test_npc_definition_is_priest_defaults_false() {
        let ron_str = r#"
NpcDefinition(
    id: "old_npc",
    name: "Old NPC",
    portrait_id: "portrait.png",
)
"#;
        let npc: NpcDefinition = ron::from_str(ron_str).expect("Failed to deserialize old format");
        assert!(
            !npc.is_priest,
            "is_priest should default to false for old RON files without the field"
        );
    }

    #[test]
    fn test_npc_definition_priest_constructor() {
        let priest = NpcDefinition::priest(
            "high_priest_alaric",
            "High Priest Alaric",
            "assets/portraits/priest.png",
        );

        assert!(
            priest.is_priest,
            "priest() constructor must set is_priest = true"
        );
        assert!(
            !priest.is_merchant,
            "priest() constructor must set is_merchant = false"
        );
        assert!(
            !priest.is_innkeeper,
            "priest() constructor must set is_innkeeper = false"
        );
        assert_eq!(priest.id, "high_priest_alaric");
        assert_eq!(priest.name, "High Priest Alaric");
    }

    #[test]
    fn test_npc_definition_stock_template_defaults_none() {
        let ron_str = r#"
NpcDefinition(
    id: "merchant_old",
    name: "Old Merchant",
    portrait_id: "merchant.png",
    is_merchant: true,
)
"#;
        let npc: NpcDefinition = ron::from_str(ron_str).expect("Failed to deserialize old format");
        assert!(
            npc.stock_template.is_none(),
            "stock_template should default to None for old RON files without the field"
        );
    }

    #[test]
    fn test_npc_definition_service_catalog_defaults_none() {
        let ron_str = r#"
NpcDefinition(
    id: "priest_old",
    name: "Old Priest",
    portrait_id: "priest.png",
    is_priest: true,
)
"#;
        let npc: NpcDefinition = ron::from_str(ron_str).expect("Failed to deserialize old format");
        assert!(
            npc.service_catalog.is_none(),
            "service_catalog should default to None for old RON files without the field"
        );
    }

    #[test]
    fn test_npc_definition_economy_defaults_none() {
        let ron_str = r#"
NpcDefinition(
    id: "merchant_no_economy",
    name: "Plain Merchant",
    portrait_id: "merchant.png",
    is_merchant: true,
)
"#;
        let npc: NpcDefinition = ron::from_str(ron_str).expect("Failed to deserialize old format");
        assert!(
            npc.economy.is_none(),
            "economy should default to None for old RON files without the field"
        );
    }

    #[test]
    fn test_npc_definition_new_has_priest_false() {
        let npc = NpcDefinition::new("guard", "Guard", "guard.png");
        assert!(!npc.is_priest);
        assert!(npc.stock_template.is_none());
        assert!(npc.service_catalog.is_none());
        assert!(npc.economy.is_none());
    }

    #[test]
    fn test_npc_definition_merchant_has_priest_false() {
        let merchant = NpcDefinition::merchant("m1", "Merchant", "m.png");
        assert!(merchant.is_merchant);
        assert!(!merchant.is_priest);
        assert!(merchant.stock_template.is_none());
        assert!(merchant.economy.is_none());
    }

    #[test]
    fn test_npc_definition_innkeeper_has_priest_false() {
        let innkeeper = NpcDefinition::innkeeper("i1", "Innkeeper", "i.png");
        assert!(innkeeper.is_innkeeper);
        assert!(!innkeeper.is_priest);
        assert!(innkeeper.service_catalog.is_none());
    }

    #[test]
    fn test_npc_definition_priest_with_service_catalog_serialization() {
        use crate::domain::inventory::{ServiceCatalog, ServiceEntry};

        let mut catalog = ServiceCatalog::new();
        catalog
            .services
            .push(ServiceEntry::new("heal_all", 50, "Heal the entire party"));
        catalog.services.push(ServiceEntry::with_gem_cost(
            "resurrect",
            500,
            1,
            "Resurrect a dead character",
        ));

        let priest = NpcDefinition {
            id: "priest_benedictus".to_string(),
            name: "Father Benedictus".to_string(),
            description: "A devoted priest of the light".to_string(),
            portrait_id: "priest.png".to_string(),
            dialogue_id: Some(20),
            creature_id: None,
            sprite: None,
            quest_ids: vec![],
            faction: Some("Temple of Light".to_string()),
            is_merchant: false,
            is_innkeeper: false,
            is_priest: true,
            is_trainer: false,
            stock_template: None,
            service_catalog: Some(catalog),
            economy: None,
            training_fee_base: None,
            training_fee_multiplier: None,
        };

        let serialized = ron::to_string(&priest).expect("Failed to serialize");
        let deserialized: NpcDefinition =
            ron::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(priest, deserialized);
        assert!(deserialized.is_priest);
        let catalog = deserialized.service_catalog.as_ref().unwrap();
        assert!(catalog.has_service("heal_all"));
        assert!(catalog.has_service("resurrect"));
    }

    #[test]
    fn test_npc_definition_merchant_with_stock_template_and_economy_serialization() {
        use crate::domain::inventory::NpcEconomySettings;

        let npc = NpcDefinition {
            id: "blacksmith_greg".to_string(),
            name: "Blacksmith Greg".to_string(),
            description: "A skilled weaponsmith".to_string(),
            portrait_id: "blacksmith.png".to_string(),
            dialogue_id: None,
            creature_id: None,
            sprite: None,
            quest_ids: vec![],
            faction: None,
            is_merchant: true,
            is_innkeeper: false,
            is_priest: false,
            is_trainer: false,
            stock_template: Some("blacksmith_basic_stock".to_string()),
            service_catalog: None,
            economy: Some(NpcEconomySettings::new(0.4, 1.2)),
            training_fee_base: None,
            training_fee_multiplier: None,
        };

        let serialized = ron::to_string(&npc).expect("Failed to serialize");
        let deserialized: NpcDefinition =
            ron::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(npc, deserialized);
        assert_eq!(
            deserialized.stock_template,
            Some("blacksmith_basic_stock".to_string())
        );
        let economy = deserialized.economy.as_ref().unwrap();
        assert_eq!(economy.buy_rate, 0.4);
        assert_eq!(economy.sell_rate, 1.2);
    }

    #[test]
    fn test_npc_definition_is_trainer_defaults_false() {
        let ron_str = r#"
NpcDefinition(
    id: "old_npc",
    name: "Old NPC",
    portrait_id: "portrait.png",
)
"#;
        let npc: NpcDefinition = ron::from_str(ron_str).expect("Failed to deserialize old format");
        assert!(
            !npc.is_trainer,
            "is_trainer should default to false for old RON files without the field"
        );
        assert!(npc.training_fee_base.is_none());
        assert!(npc.training_fee_multiplier.is_none());
    }

    #[test]
    fn test_npc_definition_trainer_constructor() {
        let trainer = NpcDefinition::trainer(
            "master_swordsman",
            "Master Swordsman",
            "assets/portraits/trainer.png",
            300,
        );

        assert!(trainer.is_trainer, "trainer() must set is_trainer = true");
        assert!(!trainer.is_merchant);
        assert!(!trainer.is_innkeeper);
        assert!(!trainer.is_priest);
        assert_eq!(trainer.training_fee_base, Some(300));
        assert!(trainer.training_fee_multiplier.is_none());
        assert_eq!(trainer.id, "master_swordsman");
    }

    #[test]
    fn test_npc_definition_training_fee_for_level_uses_campaign_defaults() {
        let config = crate::domain::campaign::CampaignConfig::default();
        // default training_fee_base=500, multiplier=1.0
        let trainer = NpcDefinition::trainer("t", "T", "t.png", 500);

        assert_eq!(trainer.training_fee_for_level(1, &config), 500);
        assert_eq!(trainer.training_fee_for_level(5, &config), 2500);
        assert_eq!(trainer.training_fee_for_level(10, &config), 5000);
    }

    #[test]
    fn test_npc_definition_training_fee_for_level_npc_override() {
        let config = crate::domain::campaign::CampaignConfig::default();
        let mut trainer = NpcDefinition::trainer("t", "T", "t.png", 200);
        trainer.training_fee_multiplier = Some(2.0);

        assert_eq!(trainer.training_fee_for_level(1, &config), 400); // 200 * 2.0 * 1
        assert_eq!(trainer.training_fee_for_level(3, &config), 1200); // 200 * 2.0 * 3
    }

    #[test]
    fn test_npc_definition_training_fee_falls_back_to_campaign_config() {
        let config = crate::domain::campaign::CampaignConfig::default(); // base=500, mult=1.0
                                                                         // NPC with no overrides uses campaign values
        let mut npc = NpcDefinition::new("guard", "Guard", "guard.png");
        npc.is_trainer = true;

        assert_eq!(npc.training_fee_for_level(1, &config), 500); // 500 * 1.0 * 1
        assert_eq!(npc.training_fee_for_level(4, &config), 2000); // 500 * 1.0 * 4
    }

    #[test]
    fn test_npc_definition_trainer_serialization_roundtrip() {
        let trainer = NpcDefinition::trainer("training_master", "Training Master", "tm.png", 750);

        let serialized = ron::to_string(&trainer).expect("Failed to serialize");
        let deserialized: NpcDefinition =
            ron::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(trainer, deserialized);
        assert!(deserialized.is_trainer);
        assert_eq!(deserialized.training_fee_base, Some(750));
    }

    #[test]
    fn test_npc_definition_new_has_trainer_false() {
        let npc = NpcDefinition::new("guard", "Guard", "guard.png");
        assert!(!npc.is_trainer);
        assert!(npc.training_fee_base.is_none());
        assert!(npc.training_fee_multiplier.is_none());
    }
}
