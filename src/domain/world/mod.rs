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

pub mod blueprint;
mod events;
mod movement;
pub mod npc;
pub mod sprite_selection;
mod types;

pub use blueprint::MapBlueprint;
pub use events::{random_encounter, trigger_event, EventError, EventResult};
pub use movement::{check_tile_blocked, move_party, MovementError};
pub use npc::{NpcDefinition, NpcId, NpcPlacement};
pub use types::{
    ArchConfig, AsyncMeshConfig, AsyncMeshTaskId, ColumnConfig, ColumnStyle, DetailLevel,
    DoorFrameConfig, EncounterTable, FurnitureType, InstanceData, LayeredSprite, Map, MapEvent,
    RailingConfig, ResolvedNpc, SpriteAnimation, SpriteLayer, SpriteMaterialProperties,
    SpriteReference, SpriteSelectionRule, StructureType, TerrainType, Tile, TileVisualMetadata,
    WallSegmentConfig, WallType, World,
};
