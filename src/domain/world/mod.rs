// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! World system - Maps, tiles, and locations
//!
//! This module contains all world-related data structures and logic including
//! tiles, maps, movement, and event handling.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.2 for complete specifications.
//!
//! # Module Organization
//!
//! - `types`: Core world data structures (Tile, Map, World, etc.)
//! - `movement`: Party movement and navigation logic
//! - `events`: Map event handling system
//! - `sprite_selection`: Procedural sprite selection
//! - `dropped_items`: Domain record for items lying on the ground ([`DroppedItem`])
//! - `furniture`: Data-driven furniture definitions ([`FurnitureDefinition`], [`FurnitureDatabase`])
//! - `landscape`: Data-driven landscape definitions and map placements
//! - `object_mesh`: Unified object mesh registry ([`ObjectMeshDatabase`])
//! - `lock`: Lock state domain types and unlock functions ([`LockState`], [`try_unlock`], etc.)

pub mod blueprint;
pub mod creature_binding;
pub mod dropped_items;
mod events;
pub mod furniture;
pub mod landscape;
pub mod lock;
mod movement;
pub mod npc;
pub mod npc_runtime;
pub mod object_mesh;
pub mod sprite_selection;
mod types;
pub mod wind;

pub use blueprint::MapBlueprint;
pub use creature_binding::CreatureBound;
pub use dropped_items::DroppedItem;
pub use events::{random_encounter, trigger_event, EventError, EventResult, UnlockMethod};
pub use furniture::{
    FurnitureDatabase, FurnitureDatabaseError, FurnitureDefinition, FurnitureMeshDatabase,
};
pub use landscape::{
    LandscapeCategory, LandscapeDatabase, LandscapeDatabaseError, LandscapeDefinition,
    LandscapeFlags, LandscapeMeshDatabase, LandscapePlacement,
};
pub use lock::{
    try_bash, try_lockpick, try_unlock, LockState, UnlockOutcome, BASH_TRAP_INCREMENT,
    LOCKPICK_FAIL_TRAP_INCREMENT, TRAP_CHANCE_MAX,
};
pub use movement::{
    check_tile_blocked, mark_visible_area, move_party, MovementError, VISIBILITY_RADIUS,
};
pub use npc::{NpcDefinition, NpcId, NpcPlacement};
pub use npc_runtime::{
    MerchantStockTemplate, MerchantStockTemplateDatabase, NpcRuntimeState, NpcRuntimeStore,
};
pub use object_mesh::{ObjectMeshDatabase, ObjectMeshError};
pub use types::{
    ArchConfig, AsyncMeshConfig, AsyncMeshTaskId, ColumnConfig, ColumnStyle, DetailLevel,
    DoorFrameConfig, EncounterGroup, EncounterTable, FurnitureAppearancePreset, FurnitureCategory,
    FurnitureFlags, FurnitureMaterial, FurnitureType, GrassBladeConfig, GrassDensity, InstanceData,
    LayeredSprite, Map, MapEvent, PointOfInterest, RailingConfig, ResolvedNpc, RockVariant,
    SkyConfig, SpriteAnimation, SpriteLayer, SpriteMaterialProperties, SpriteReference,
    SpriteSelectionRule, StructureType, TerrainType, Tile, TileVisualMetadata, TimeCondition,
    TreeType, WallSegmentConfig, WallType, WaterFlowDirection, World,
};
pub use wind::{CampaignWindConfig, WindSystemKind};
