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
//! - `sprite_selection`: Procedural sprite selection (Phase 6)
//! - `dropped_items`: Domain record for items lying on the ground ([`DroppedItem`])
//! - `furniture`: Data-driven furniture definitions ([`FurnitureDefinition`], [`FurnitureDatabase`])
//! - `lock`: Lock state domain types and unlock functions ([`LockState`], [`try_unlock`], etc.)

pub mod blueprint;
pub mod creature_binding;
pub mod dropped_items;
mod events;
pub mod furniture;
pub mod lock;
mod movement;
pub mod npc;
pub mod npc_runtime;
pub mod sprite_selection;
mod types;

pub use blueprint::MapBlueprint;
pub use creature_binding::CreatureBound;
pub use dropped_items::DroppedItem;
pub use events::{random_encounter, trigger_event, EventError, EventResult, UnlockMethod};
pub use furniture::{
    FurnitureDatabase, FurnitureDatabaseError, FurnitureDefinition, FurnitureMeshDatabase,
};
pub use lock::{
    try_bash, try_lockpick, try_unlock, LockState, UnlockOutcome, BASH_TRAP_INCREMENT,
    LOCKPICK_FAIL_TRAP_INCREMENT, TRAP_CHANCE_MAX,
};
pub use movement::{check_tile_blocked, move_party, MovementError};
pub use npc::{NpcDefinition, NpcId, NpcPlacement};
pub use npc_runtime::{
    MerchantStockTemplate, MerchantStockTemplateDatabase, NpcRuntimeState, NpcRuntimeStore,
};
pub use types::{
    ArchConfig, AsyncMeshConfig, AsyncMeshTaskId, ColumnConfig, ColumnStyle, DetailLevel,
    DoorFrameConfig, EncounterGroup, EncounterTable, FurnitureAppearancePreset, FurnitureCategory,
    FurnitureFlags, FurnitureMaterial, FurnitureType, GrassBladeConfig, GrassDensity, InstanceData,
    LayeredSprite, Map, MapEvent, RailingConfig, ResolvedNpc, RockVariant, SpriteAnimation,
    SpriteLayer, SpriteMaterialProperties, SpriteReference, SpriteSelectionRule, StructureType,
    TerrainType, Tile, TileVisualMetadata, TimeCondition, TreeType, WallSegmentConfig, WallType,
    WaterFlowDirection, World,
};
