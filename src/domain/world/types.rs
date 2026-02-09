// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Core world data structures
//!
//! This module contains the fundamental types for the world system including
//! tiles, maps, NPCs, and the overall world structure.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.2 for complete specifications.

use crate::domain::types::{Direction, MapId, Position};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ===== Tile Types =====

/// Wall type for tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WallType {
    /// No wall
    None,
    /// Normal wall
    Normal,
    /// Door (can be opened)
    Door,
    /// Torch (light source)
    Torch,
}

/// Terrain type for tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TerrainType {
    /// Normal walkable ground
    Ground,
    /// Grass terrain
    Grass,
    /// Water (may need special ability to cross)
    Water,
    /// Lava (damages party)
    Lava,
    /// Swamp (slows movement)
    Swamp,
    /// Stone floor
    Stone,
    /// Dirt path
    Dirt,
    /// Forest
    Forest,
    /// Mountain (blocked)
    Mountain,
}

// ===== Terrain-Specific Features =====

/// Grass density levels for terrain visualization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GrassDensity {
    /// No grass blades (bare dirt)
    None,
    /// 10-20 blades per tile
    Low,
    /// 40-60 blades per tile
    #[default]
    Medium,
    /// 80-120 blades per tile
    High,
    /// 150+ blades per tile
    VeryHigh,
}

/// Tree visual variants for forest tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TreeType {
    /// Deciduous tree (broad leaves)
    #[default]
    Oak,
    /// Coniferous tree (needle leaves)
    Pine,
    /// Dead/bare tree
    Dead,
    /// Palm tree
    Palm,
    /// Willow tree
    Willow,
    /// Birch tree
    Birch,
    /// Shrub/Bush formation
    Shrub,
}

/// Rock visual variants for mountain/hill tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RockVariant {
    /// Smooth rounded boulders
    #[default]
    Smooth,
    /// Jagged sharp rocks
    Jagged,
    /// Layered sedimentary
    Layered,
    /// Crystalline formation
    Crystal,
}

/// Water flow direction for river/stream tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WaterFlowDirection {
    /// Still water (no flow)
    #[default]
    Still,
    /// Flowing north
    North,
    /// Flowing south
    South,
    /// Flowing east
    East,
    /// Flowing west
    West,
}

// ===== Sprite System =====

/// Reference to a sprite in a sprite sheet (texture atlas)
///
/// # Examples
///
/// ```
/// use antares::domain::world::SpriteReference;
///
/// let sprite = SpriteReference {
///     sheet_path: "sprites/walls.png".to_string(),
///     sprite_index: 3,
///     animation: None,
///     material_properties: None,
/// };
/// assert_eq!(sprite.sprite_index, 3);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpriteReference {
    /// Path to sprite sheet image (relative to campaign or global assets)
    /// Example: "sprites/walls.png" or "textures/npcs_town.png"
    pub sheet_path: String,

    /// Index within texture atlas grid (0-indexed, row-major order)
    /// For 4x4 grid: index 0 = top-left, index 3 = top-right, index 15 = bottom-right
    pub sprite_index: u32,

    /// Optional animation configuration
    #[serde(default)]
    pub animation: Option<SpriteAnimation>,

    /// Material property overrides for this sprite (Phase 6)
    /// Allows per-sprite customization of emissive color, alpha, metallic, roughness
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_properties: Option<SpriteMaterialProperties>,
}

/// Animation configuration for sprite frames
///
/// # Examples
///
/// ```
/// use antares::domain::world::SpriteAnimation;
///
/// let anim = SpriteAnimation {
///     frames: vec![0, 1, 2, 1], // Ping-pong animation
///     fps: 8.0,
///     looping: true,
/// };
/// assert_eq!(anim.frames.len(), 4);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpriteAnimation {
    /// Frame indices in animation sequence (refers to sprite_index values)
    pub frames: Vec<u32>,

    /// Frames per second (default: 8.0)
    #[serde(default = "default_animation_fps")]
    pub fps: f32,

    /// Whether animation loops (default: true)
    #[serde(default = "default_animation_looping")]
    pub looping: bool,
}

/// Default FPS for sprite animations
fn default_animation_fps() -> f32 {
    8.0
}

/// Default looping behavior for sprite animations
fn default_animation_looping() -> bool {
    true
}

// ===== Phase 6: Advanced Sprite Features =====

/// Material property overrides for sprites
///
/// Allows per-sprite customization of PBR material properties.
/// All fields are optional; None means use default material properties.
///
/// # Examples
///
/// ```
/// use antares::domain::world::SpriteMaterialProperties;
///
/// let props = SpriteMaterialProperties {
///     emissive: Some([0.0, 1.0, 0.0]),
///     alpha: None,
///     metallic: None,
///     roughness: None,
/// };
/// assert_eq!(props.emissive, Some([0.0, 1.0, 0.0]));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SpriteMaterialProperties {
    /// Emissive color (RGB, 0.0-1.0 range)
    /// Creates a glowing effect independent of lighting
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emissive: Option<[f32; 3]>,

    /// Alpha/transparency override (0.0 = fully transparent, 1.0 = fully opaque)
    /// Overrides alpha channel from sprite sheet
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha: Option<f32>,

    /// Metallic factor for PBR (0.0 = non-metallic, 1.0 = fully metallic)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metallic: Option<f32>,

    /// Roughness factor for PBR (0.0 = polished, 1.0 = rough)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roughness: Option<f32>,
}

/// Sprite layer depth for layering multiple sprites per tile
///
/// Used for sprite layering system (Phase 6) to order sprites
/// in Z-order without requiring separate Z coordinates.
///
/// # Examples
///
/// ```
/// use antares::domain::world::SpriteLayer;
///
/// let layer = SpriteLayer::Foreground;
/// assert!(layer > SpriteLayer::Midground);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SpriteLayer {
    /// Background layer (rendered first, behind everything)
    /// Z-depth: 0
    Background = 0,
    /// Midground layer (default, most sprites here)
    /// Z-depth: 1
    Midground = 1,
    /// Foreground layer (rendered last, in front)
    /// Z-depth: 2
    Foreground = 2,
}

/// Sprite with layer information for multi-layered tiles
///
/// Represents a single sprite in a layered sprite stack.
/// Multiple LayeredSprites can be applied to one tile
/// for complex visual effects.
///
/// # Examples
///
/// ```
/// use antares::domain::world::{LayeredSprite, SpriteLayer, SpriteReference};
///
/// let sprite = LayeredSprite {
///     sprite: SpriteReference {
///         sheet_path: "terrain.png".to_string(),
///         sprite_index: 0,
///         animation: None,
///         material_properties: None,
///     },
///     layer: SpriteLayer::Background,
///     offset_y: 0.0,
/// };
/// assert_eq!(sprite.layer, SpriteLayer::Background);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayeredSprite {
    /// Sprite reference (sheet path, index, animation)
    pub sprite: SpriteReference,

    /// Layer depth (controls rendering order)
    pub layer: SpriteLayer,

    /// Vertical Y-axis offset from base position
    /// Positive = raised, negative = sunken
    #[serde(default)]
    pub offset_y: f32,
}

/// Procedural sprite selection rule for automatic sprite variation
///
/// Allows tiles to automatically select different sprites based on
/// context (fixed, randomized, or auto-tiling rules).
///
/// # Examples
///
/// ```
/// use antares::domain::world::SpriteSelectionRule;
///
/// let rule = SpriteSelectionRule::Random {
///     sheet_path: "grass.png".to_string(),
///     sprite_indices: vec![0, 1, 2, 3],
///     seed: Some(42),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SpriteSelectionRule {
    /// Fixed sprite (no variation)
    Fixed {
        /// Path to sprite sheet
        sheet_path: String,
        /// Index of sprite to use
        sprite_index: u32,
    },
    /// Random variation from a list of sprite indices
    /// Deterministic if seed is provided
    Random {
        /// Path to sprite sheet
        sheet_path: String,
        /// Available sprite indices for random selection
        sprite_indices: Vec<u32>,
        /// Optional seed for deterministic random selection
        /// If None, uses tile position as seed for consistency
        seed: Option<u64>,
    },
    /// Autotile selection based on neighbor tiles
    /// Useful for seamless tiling (grass, water, walls)
    Autotile {
        /// Path to sprite sheet
        sheet_path: String,
        /// Mapping from 4-bit neighbor bitmask to sprite index
        /// Bits: [North, East, South, West]
        /// E.g., 0b0011 = North and East neighbors = index 5
        rules: std::collections::HashMap<u8, u32>,
    },
}

/// Configuration for individual grass blade appearance (Phase 3)
///
/// Controls the visual properties of grass blades spawned on a tile.
/// All multipliers are clamped to safe ranges during conversion.
///
/// # Examples
///
/// ```
/// use antares::domain::world::GrassBladeConfig;
///
/// let tall_grass = GrassBladeConfig {
///     length: 1.5,
///     width: 0.8,
///     tilt: 0.4,
///     curve: 0.5,
///     color_variation: 0.3,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GrassBladeConfig {
    /// Blade length multiplier (0.5-2.0, default 1.0)
    /// Applied to base blade height
    pub length: f32,

    /// Blade width multiplier (0.5-2.0, default 1.0)
    /// Applied to base blade width
    pub width: f32,

    /// Blade tilt angle in radians (0.0-0.5, default 0.3)
    /// Controls how much blades lean from vertical
    pub tilt: f32,

    /// Blade curvature amount (0.0-1.0, default 0.3)
    /// Higher values create more curved blades
    pub curve: f32,

    /// Color variation (0.0-1.0, default 0.2)
    /// 0.0 = uniform color, 1.0 = high variation
    pub color_variation: f32,
}

impl Default for GrassBladeConfig {
    fn default() -> Self {
        Self {
            length: 1.0,
            width: 1.0,
            tilt: 0.3,
            curve: 0.3,
            color_variation: 0.2,
        }
    }
}

/// Visual rendering properties for a tile
///
/// All dimensions in world units (1 unit ≈ 10 feet).
/// All fields are optional to maintain backward compatibility.
/// When None, defaults are determined by terrain/wall type.
///
/// # Examples
///
/// ```
/// use antares::domain::world::{TileVisualMetadata, TerrainType, WallType};
///
/// let mut metadata = TileVisualMetadata::default();
/// metadata.height = Some(1.5); // Custom 15-foot wall
/// assert_eq!(metadata.effective_height(TerrainType::Ground, WallType::Normal), 1.5);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TileVisualMetadata {
    /// Height of wall/terrain feature (Y-axis dimension)
    /// Default: wall=2.5, mountain=3.0, tree=2.2, door=2.5
    pub height: Option<f32>,

    /// Width in X-axis (default: 1.0 for full tile)
    pub width_x: Option<f32>,

    /// Depth in Z-axis (default: 1.0 for full tile)
    pub width_z: Option<f32>,

    /// Color tint (RGB, 0.0-1.0 range)
    /// Applied multiplicatively to base material color
    pub color_tint: Option<(f32, f32, f32)>,

    /// Scale multiplier (default: 1.0)
    /// Applied uniformly to all dimensions
    pub scale: Option<f32>,

    /// Vertical offset from ground (default: 0.0)
    /// Positive = raised, negative = sunken
    pub y_offset: Option<f32>,

    /// Rotation around Y-axis in degrees (default: 0.0)
    /// Useful for angled walls, rotated props, diagonal features
    /// Positive = counter-clockwise when viewed from above
    pub rotation_y: Option<f32>,

    /// Optional sprite reference for texture-based rendering
    /// When set, replaces default 3D mesh with billboarded sprite
    #[serde(default)]
    pub sprite: Option<SpriteReference>,

    /// Multiple sprite layers for complex visuals (Phase 6)
    /// Allows stacking sprites (background, midground, foreground)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sprite_layers: Vec<LayeredSprite>,

    /// Procedural sprite selection rule (Phase 6)
    /// If set, overrides the fixed `sprite` field
    /// Useful for random variation or autotiling
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite_rule: Option<SpriteSelectionRule>,

    /// Grass density for grassland/plains tiles (default: Medium)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grass_density: Option<GrassDensity>,

    /// Tree type for forest tiles (default: Oak)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree_type: Option<TreeType>,

    /// Rock variant for mountain/hill tiles (default: Smooth)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rock_variant: Option<RockVariant>,

    /// Water flow direction for water tiles (default: Still)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub water_flow_direction: Option<WaterFlowDirection>,

    /// Foliage density multiplier (0.0 to 2.0, default: 1.0)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foliage_density: Option<f32>,

    /// Snow coverage percentage (0.0 to 1.0, default: 0.0)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snow_coverage: Option<f32>,

    /// Grass blade configuration for customized appearance (Phase 3)
    /// Controls blade dimensions, curvature, tilt, and color variation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grass_blade_config: Option<GrassBladeConfig>,
}

impl TileVisualMetadata {
    /// Get effective height for this tile based on terrain/wall type
    ///
    /// Falls back to hardcoded defaults if not specified.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{TileVisualMetadata, TerrainType, WallType};
    ///
    /// let metadata = TileVisualMetadata::default();
    /// assert_eq!(metadata.effective_height(TerrainType::Ground, WallType::Normal), 2.5);
    /// assert_eq!(metadata.effective_height(TerrainType::Mountain, WallType::None), 3.0);
    /// ```
    pub fn effective_height(&self, terrain: TerrainType, wall_type: WallType) -> f32 {
        if let Some(h) = self.height {
            return h;
        }

        // Default heights matching current hardcoded values
        match wall_type {
            WallType::Normal | WallType::Door | WallType::Torch => 2.5,
            WallType::None => match terrain {
                TerrainType::Mountain => 3.0,
                TerrainType::Forest => 2.2,
                _ => 0.0, // Flat terrain has no height
            },
        }
    }

    /// Get effective width_x (defaults to 1.0)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::TileVisualMetadata;
    ///
    /// let metadata = TileVisualMetadata::default();
    /// assert_eq!(metadata.effective_width_x(), 1.0);
    /// ```
    pub fn effective_width_x(&self) -> f32 {
        self.width_x.unwrap_or(1.0)
    }

    /// Get effective width_z (defaults to 1.0)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::TileVisualMetadata;
    ///
    /// let metadata = TileVisualMetadata::default();
    /// assert_eq!(metadata.effective_width_z(), 1.0);
    /// ```
    pub fn effective_width_z(&self) -> f32 {
        self.width_z.unwrap_or(1.0)
    }

    /// Get grass density with fallback to default
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{TileVisualMetadata, GrassDensity};
    ///
    /// let metadata = TileVisualMetadata::default();
    /// assert_eq!(metadata.grass_density(), GrassDensity::Medium);
    /// ```
    pub fn grass_density(&self) -> GrassDensity {
        self.grass_density.unwrap_or_default()
    }

    /// Get tree type with fallback to default
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{TileVisualMetadata, TreeType};
    ///
    /// let metadata = TileVisualMetadata::default();
    /// assert_eq!(metadata.tree_type(), TreeType::Oak);
    /// ```
    pub fn tree_type(&self) -> TreeType {
        self.tree_type.unwrap_or_default()
    }

    /// Get rock variant with fallback to default
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{TileVisualMetadata, RockVariant};
    ///
    /// let metadata = TileVisualMetadata::default();
    /// assert_eq!(metadata.rock_variant(), RockVariant::Smooth);
    /// ```
    pub fn rock_variant(&self) -> RockVariant {
        self.rock_variant.unwrap_or_default()
    }

    /// Get water flow direction with fallback to default
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{TileVisualMetadata, WaterFlowDirection};
    ///
    /// let metadata = TileVisualMetadata::default();
    /// assert_eq!(metadata.water_flow_direction(), WaterFlowDirection::Still);
    /// ```
    pub fn water_flow_direction(&self) -> WaterFlowDirection {
        self.water_flow_direction.unwrap_or_default()
    }

    /// Get foliage density with fallback to 1.0
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::TileVisualMetadata;
    ///
    /// let metadata = TileVisualMetadata::default();
    /// assert_eq!(metadata.foliage_density(), 1.0);
    /// ```
    pub fn foliage_density(&self) -> f32 {
        self.foliage_density.unwrap_or(1.0)
    }

    /// Get snow coverage with fallback to 0.0
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::TileVisualMetadata;
    ///
    /// let metadata = TileVisualMetadata::default();
    /// assert_eq!(metadata.snow_coverage(), 0.0);
    /// ```
    pub fn snow_coverage(&self) -> f32 {
        self.snow_coverage.unwrap_or(0.0)
    }

    /// Check if metadata has any terrain-specific overrides
    ///
    /// Returns true if any of the terrain-specific fields are set (Some).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{TileVisualMetadata, GrassDensity};
    ///
    /// let mut metadata = TileVisualMetadata::default();
    /// assert!(!metadata.has_terrain_overrides());
    ///
    /// metadata.grass_density = Some(GrassDensity::High);
    /// assert!(metadata.has_terrain_overrides());
    /// ```
    pub fn has_terrain_overrides(&self) -> bool {
        self.grass_density.is_some()
            || self.tree_type.is_some()
            || self.rock_variant.is_some()
            || self.water_flow_direction.is_some()
            || self.foliage_density.is_some()
            || self.snow_coverage.is_some()
    }

    /// Get effective scale (defaults to 1.0)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::TileVisualMetadata;
    ///
    /// let metadata = TileVisualMetadata::default();
    /// assert_eq!(metadata.effective_scale(), 1.0);
    /// ```
    pub fn effective_scale(&self) -> f32 {
        self.scale.unwrap_or(1.0)
    }

    /// Get effective y_offset (defaults to 0.0)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::TileVisualMetadata;
    ///
    /// let metadata = TileVisualMetadata::default();
    /// assert_eq!(metadata.effective_y_offset(), 0.0);
    /// ```
    pub fn effective_y_offset(&self) -> f32 {
        self.y_offset.unwrap_or(0.0)
    }

    /// Calculate mesh dimensions (width_x, height, width_z) with scale applied
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{TileVisualMetadata, TerrainType, WallType};
    ///
    /// let metadata = TileVisualMetadata::default();
    /// let (x, h, z) = metadata.mesh_dimensions(TerrainType::Ground, WallType::Normal);
    /// assert_eq!((x, h, z), (1.0, 2.5, 1.0));
    /// ```
    pub fn mesh_dimensions(&self, terrain: TerrainType, wall_type: WallType) -> (f32, f32, f32) {
        let scale = self.effective_scale();
        (
            self.effective_width_x() * scale,
            self.effective_height(terrain, wall_type) * scale,
            self.effective_width_z() * scale,
        )
    }

    /// Get effective rotation_y in degrees (defaults to 0.0)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::TileVisualMetadata;
    ///
    /// let mut metadata = TileVisualMetadata::default();
    /// assert_eq!(metadata.effective_rotation_y(), 0.0);
    ///
    /// let metadata = TileVisualMetadata { rotation_y: Some(45.0), ..Default::default() };
    /// assert_eq!(metadata.effective_rotation_y(), 45.0);
    /// ```
    pub fn effective_rotation_y(&self) -> f32 {
        self.rotation_y.unwrap_or(0.0)
    }

    /// Get rotation_y in radians (converts from degrees)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::TileVisualMetadata;
    ///
    /// let metadata = TileVisualMetadata { rotation_y: Some(180.0), ..Default::default() };
    /// assert!((metadata.rotation_y_radians() - std::f32::consts::PI).abs() < 0.001);
    /// ```
    pub fn rotation_y_radians(&self) -> f32 {
        self.effective_rotation_y().to_radians()
    }

    /// Calculate Y-position for mesh center
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{TileVisualMetadata, TerrainType, WallType};
    ///
    /// let metadata = TileVisualMetadata::default();
    /// assert_eq!(metadata.mesh_y_position(TerrainType::Ground, WallType::Normal), 1.25);
    /// ```
    pub fn mesh_y_position(&self, terrain: TerrainType, wall_type: WallType) -> f32 {
        let height = self.effective_height(terrain, wall_type);
        let scale = self.effective_scale();
        (height * scale / 2.0) + self.effective_y_offset()
    }

    /// Check if sprite rendering is enabled
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{TileVisualMetadata, SpriteReference};
    ///
    /// let mut metadata = TileVisualMetadata::default();
    /// assert!(!metadata.uses_sprite());
    ///
    /// metadata.sprite = Some(SpriteReference {
    ///     sheet_path: "sprites/walls.png".to_string(),
    ///     sprite_index: 0,
    ///     animation: None,
    ///     material_properties: None,
    /// });
    /// assert!(metadata.uses_sprite());
    /// ```
    pub fn uses_sprite(&self) -> bool {
        self.sprite.is_some()
    }

    /// Get sprite sheet path if sprite is configured
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{TileVisualMetadata, SpriteReference};
    ///
    /// let mut metadata = TileVisualMetadata::default();
    /// assert_eq!(metadata.sprite_sheet_path(), None);
    ///
    /// metadata.sprite = Some(SpriteReference {
    ///     sheet_path: "sprites/walls.png".to_string(),
    ///     sprite_index: 0,
    ///     animation: None,
    ///     material_properties: None,
    /// });
    /// assert_eq!(metadata.sprite_sheet_path(), Some("sprites/walls.png"));
    /// ```
    pub fn sprite_sheet_path(&self) -> Option<&str> {
        self.sprite.as_ref().map(|s| s.sheet_path.as_str())
    }

    /// Get sprite index if sprite is configured
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{TileVisualMetadata, SpriteReference};
    ///
    /// let mut metadata = TileVisualMetadata::default();
    /// assert_eq!(metadata.sprite_index(), None);
    ///
    /// metadata.sprite = Some(SpriteReference {
    ///     sheet_path: "sprites/walls.png".to_string(),
    ///     sprite_index: 42,
    ///     animation: None,
    ///     material_properties: None,
    /// });
    /// assert_eq!(metadata.sprite_index(), Some(42));
    /// ```
    pub fn sprite_index(&self) -> Option<u32> {
        self.sprite.as_ref().map(|s| s.sprite_index)
    }

    /// Check if sprite has animation configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{TileVisualMetadata, SpriteReference, SpriteAnimation};
    ///
    /// let mut metadata = TileVisualMetadata::default();
    /// assert!(!metadata.has_animation());
    ///
    /// metadata.sprite = Some(SpriteReference {
    ///     sheet_path: "sprites/walls.png".to_string(),
    ///     sprite_index: 0,
    ///     animation: Some(SpriteAnimation {
    ///         frames: vec![0, 1, 2],
    ///         fps: 8.0,
    ///         looping: true,
    ///     }),
    ///     material_properties: None,
    /// });
    /// assert!(metadata.has_animation());
    /// ```
    pub fn has_animation(&self) -> bool {
        self.sprite
            .as_ref()
            .and_then(|s| s.animation.as_ref())
            .is_some()
    }
}

/// A single tile in the game world
///
/// # Examples
///
/// ```
/// use antares::domain::world::{Tile, TerrainType, WallType};
///
/// let tile = Tile::new(0, 0, TerrainType::Ground, WallType::None);
/// assert!(!tile.blocked);
/// assert!(!tile.visited);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    /// Terrain type
    pub terrain: TerrainType,
    /// Wall type (None, Normal, Door, Torch)
    pub wall_type: WallType,
    /// Whether movement is blocked
    pub blocked: bool,
    /// Special tile (for events)
    pub is_special: bool,
    /// Dark area (requires light)
    pub is_dark: bool,
    /// Has been visited by party
    pub visited: bool,
    /// X coordinate
    pub x: i32,
    /// Y coordinate
    pub y: i32,

    /// Optional visual rendering metadata
    #[serde(default)]
    pub visual: TileVisualMetadata,
}

impl Tile {
    /// Creates a new tile with the given terrain and wall type
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{Tile, TerrainType, WallType};
    ///
    /// let tile = Tile::new(0, 0, TerrainType::Ground, WallType::None);
    /// assert_eq!(tile.terrain, TerrainType::Ground);
    /// assert_eq!(tile.wall_type, WallType::None);
    /// ```
    pub fn new(x: i32, y: i32, terrain: TerrainType, wall_type: WallType) -> Self {
        let blocked = matches!(terrain, TerrainType::Mountain | TerrainType::Water)
            || matches!(wall_type, WallType::Normal);

        Self {
            x,
            y,
            terrain,
            wall_type,
            blocked,
            is_special: false,
            is_dark: false,
            visited: false,
            visual: TileVisualMetadata::default(),
        }
    }

    /// Returns true if the tile blocks movement
    pub fn is_blocked(&self) -> bool {
        self.blocked
    }

    /// Returns true if the tile is a door
    pub fn is_door(&self) -> bool {
        self.wall_type == WallType::Door
    }

    /// Returns true if the tile has a light source (torch)
    pub fn has_light(&self) -> bool {
        self.wall_type == WallType::Torch
    }

    /// Marks the tile as visited
    pub fn mark_visited(&mut self) {
        self.visited = true;
    }

    /// Sets a custom height for this tile
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{Tile, TerrainType, WallType};
    ///
    /// let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal)
    ///     .with_height(1.5);
    /// assert_eq!(tile.visual.height, Some(1.5));
    /// ```
    pub fn with_height(mut self, height: f32) -> Self {
        self.visual.height = Some(height);
        self
    }

    /// Sets custom dimensions for this tile
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{Tile, TerrainType, WallType};
    ///
    /// let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal)
    ///     .with_dimensions(0.8, 1.5, 0.8);
    /// assert_eq!(tile.visual.width_x, Some(0.8));
    /// assert_eq!(tile.visual.height, Some(1.5));
    /// assert_eq!(tile.visual.width_z, Some(0.8));
    /// ```
    pub fn with_dimensions(mut self, width_x: f32, height: f32, width_z: f32) -> Self {
        self.visual.width_x = Some(width_x);
        self.visual.height = Some(height);
        self.visual.width_z = Some(width_z);
        self
    }

    /// Sets a color tint for this tile
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{Tile, TerrainType, WallType};
    ///
    /// let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal)
    ///     .with_color_tint(1.0, 0.5, 0.5);
    /// assert_eq!(tile.visual.color_tint, Some((1.0, 0.5, 0.5)));
    /// ```
    pub fn with_color_tint(mut self, r: f32, g: f32, b: f32) -> Self {
        self.visual.color_tint = Some((r, g, b));
        self
    }

    /// Sets a scale multiplier for this tile
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{Tile, TerrainType, WallType};
    ///
    /// let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal)
    ///     .with_scale(1.5);
    /// assert_eq!(tile.visual.scale, Some(1.5));
    /// ```
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.visual.scale = Some(scale);
        self
    }

    /// Set static sprite for this tile
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{Tile, TerrainType, WallType};
    /// use antares::domain::types::Position;
    ///
    /// let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal)
    ///     .with_sprite("sprites/walls.png", 5);
    ///
    /// assert!(tile.visual.uses_sprite());
    /// assert_eq!(tile.visual.sprite_index(), Some(5));
    /// ```
    pub fn with_sprite(mut self, sheet_path: &str, sprite_index: u32) -> Self {
        self.visual.sprite = Some(SpriteReference {
            sheet_path: sheet_path.to_string(),
            sprite_index,
            animation: None,
            material_properties: None,
        });
        self
    }

    /// Set animated sprite for this tile
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{Tile, TerrainType, WallType};
    /// use antares::domain::types::Position;
    ///
    /// let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal)
    ///     .with_animated_sprite("sprites/water.png", vec![0, 1, 2, 3], 4.0, true);
    ///
    /// assert!(tile.visual.uses_sprite());
    /// assert!(tile.visual.has_animation());
    /// ```
    pub fn with_animated_sprite(
        mut self,
        sheet_path: &str,
        frames: Vec<u32>,
        fps: f32,
        looping: bool,
    ) -> Self {
        self.visual.sprite = Some(SpriteReference {
            sheet_path: sheet_path.to_string(),
            sprite_index: frames[0], // First frame is base sprite_index
            animation: Some(SpriteAnimation {
                frames,
                fps,
                looping,
            }),
            material_properties: None,
        });
        self
    }
}

// ===== Furniture Types =====

/// Material types for furniture rendering
///
/// Each material has different visual properties (PBR parameters) that affect
/// how the furniture appears in the game world.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum FurnitureMaterial {
    /// Wood material (default)
    #[default]
    Wood,
    /// Stone material
    Stone,
    /// Metal material
    Metal,
    /// Gold material
    Gold,
}

impl FurnitureMaterial {
    /// Returns all material variants
    pub fn all() -> &'static [FurnitureMaterial] {
        &[
            FurnitureMaterial::Wood,
            FurnitureMaterial::Stone,
            FurnitureMaterial::Metal,
            FurnitureMaterial::Gold,
        ]
    }

    /// Returns human-readable name for the material
    pub fn name(self) -> &'static str {
        match self {
            FurnitureMaterial::Wood => "Wood",
            FurnitureMaterial::Stone => "Stone",
            FurnitureMaterial::Metal => "Metal",
            FurnitureMaterial::Gold => "Gold",
        }
    }

    /// Returns base color in RGB format (0.0-1.0 range) for PBR rendering
    ///
    /// # Returns
    ///
    /// `[f32; 3]` representing RGB color values
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::FurnitureMaterial;
    ///
    /// let wood_color = FurnitureMaterial::Wood.base_color();
    /// assert_eq!(wood_color, [0.6, 0.4, 0.2]); // Brown
    ///
    /// let gold_color = FurnitureMaterial::Gold.base_color();
    /// assert_eq!(gold_color, [1.0, 0.84, 0.0]); // Gold
    /// ```
    pub fn base_color(self) -> [f32; 3] {
        match self {
            FurnitureMaterial::Wood => [0.6, 0.4, 0.2],  // Brown
            FurnitureMaterial::Stone => [0.5, 0.5, 0.5], // Gray
            FurnitureMaterial::Metal => [0.7, 0.7, 0.8], // Silver
            FurnitureMaterial::Gold => [1.0, 0.84, 0.0], // Gold
        }
    }

    /// Returns metallic property (0.0-1.0) for PBR rendering
    ///
    /// Indicates how metallic the material surface is:
    /// - 0.0: Non-metallic (plastic, wood, stone)
    /// - 1.0: Fully metallic (polished metal, gold)
    ///
    /// # Returns
    ///
    /// Metallic value between 0.0 (non-metallic) and 1.0 (fully metallic)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::FurnitureMaterial;
    ///
    /// assert_eq!(FurnitureMaterial::Wood.metallic(), 0.0);
    /// assert_eq!(FurnitureMaterial::Gold.metallic(), 1.0);
    /// ```
    pub fn metallic(self) -> f32 {
        match self {
            FurnitureMaterial::Wood => 0.0,
            FurnitureMaterial::Stone => 0.1,
            FurnitureMaterial::Metal => 0.9,
            FurnitureMaterial::Gold => 1.0,
        }
    }

    /// Returns roughness property (0.0-1.0) for PBR rendering
    ///
    /// Indicates how rough the material surface is:
    /// - 0.0: Smooth, mirror-like surface
    /// - 1.0: Rough, matte surface
    ///
    /// # Returns
    ///
    /// Roughness value between 0.0 (smooth) and 1.0 (rough)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::FurnitureMaterial;
    ///
    /// assert_eq!(FurnitureMaterial::Gold.roughness(), 0.2); // Shiny
    /// assert_eq!(FurnitureMaterial::Stone.roughness(), 0.9); // Dull
    /// ```
    pub fn roughness(self) -> f32 {
        match self {
            FurnitureMaterial::Wood => 0.8,
            FurnitureMaterial::Stone => 0.9,
            FurnitureMaterial::Metal => 0.3,
            FurnitureMaterial::Gold => 0.2,
        }
    }
}

/// Furniture-specific state flags
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FurnitureFlags {
    /// Torch is lit (emissive)
    pub lit: bool,
    /// Chest is locked
    pub locked: bool,
    /// Furniture blocks movement
    pub blocking: bool,
}

impl FurnitureFlags {
    /// Creates a new FurnitureFlags with all flags set to false
    pub fn new() -> Self {
        FurnitureFlags::default()
    }

    /// Sets the lit flag and returns self for chaining
    pub fn with_lit(mut self, lit: bool) -> Self {
        self.lit = lit;
        self
    }

    /// Sets the locked flag and returns self for chaining
    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// Sets the blocking flag and returns self for chaining
    pub fn with_blocking(mut self, blocking: bool) -> Self {
        self.blocking = blocking;
        self
    }
}

/// Furniture appearance customization preset
///
/// Presets allow quick application of common material, scale, and color combinations.
#[derive(Clone, Debug, PartialEq)]
pub struct FurnitureAppearancePreset {
    /// Human-readable name for this preset
    pub name: &'static str,
    /// Material variant to apply
    pub material: FurnitureMaterial,
    /// Scale multiplier to apply
    pub scale: f32,
    /// Optional color tint (RGB, 0.0-1.0)
    pub color_tint: Option<[f32; 3]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum FurnitureType {
    /// Ornate throne for rulers
    Throne,
    /// Simple bench seating
    Bench,
    /// Dining or working table
    Table,
    /// Single seat chair
    Chair,
    /// Mounted torch for light
    Torch,
    /// Storage bookshelf
    Bookshelf,
    /// Wooden barrel for storage
    Barrel,
    /// Lockable chest for treasure
    Chest,
}

impl FurnitureType {
    /// Returns all furniture type variants
    pub fn all() -> &'static [FurnitureType] {
        &[
            FurnitureType::Throne,
            FurnitureType::Bench,
            FurnitureType::Table,
            FurnitureType::Chair,
            FurnitureType::Torch,
            FurnitureType::Bookshelf,
            FurnitureType::Barrel,
            FurnitureType::Chest,
        ]
    }

    /// Returns human-readable name for the furniture type
    pub fn name(self) -> &'static str {
        match self {
            FurnitureType::Throne => "Throne",
            FurnitureType::Bench => "Bench",
            FurnitureType::Table => "Table",
            FurnitureType::Chair => "Chair",
            FurnitureType::Torch => "Torch",
            FurnitureType::Bookshelf => "Bookshelf",
            FurnitureType::Barrel => "Barrel",
            FurnitureType::Chest => "Chest",
        }
    }

    /// Returns an emoji icon representing the furniture type
    pub fn icon(self) -> &'static str {
        match self {
            FurnitureType::Throne => "👑",
            FurnitureType::Bench => "🪑",
            FurnitureType::Table => "🪵",
            FurnitureType::Chair => "💺",
            FurnitureType::Torch => "🔥",
            FurnitureType::Bookshelf => "📚",
            FurnitureType::Barrel => "🛢️",
            FurnitureType::Chest => "📦",
        }
    }

    /// Returns the category for this furniture type
    pub fn category(self) -> FurnitureCategory {
        match self {
            FurnitureType::Throne | FurnitureType::Bench | FurnitureType::Chair => {
                FurnitureCategory::Seating
            }
            FurnitureType::Chest | FurnitureType::Barrel | FurnitureType::Bookshelf => {
                FurnitureCategory::Storage
            }
            FurnitureType::Torch => FurnitureCategory::Lighting,
            FurnitureType::Table => FurnitureCategory::Utility,
        }
    }

    /// Returns default appearance presets for this furniture type
    ///
    /// Each furniture type has one or more predefined appearance configurations
    /// combining material, scale, and color tint settings.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{FurnitureType, FurnitureMaterial};
    ///
    /// let throne_presets = FurnitureType::Throne.default_presets();
    /// assert!(throne_presets.len() >= 3); // Wooden, Stone, Golden thrones
    ///
    /// let torch_presets = FurnitureType::Torch.default_presets();
    /// assert!(torch_presets.len() >= 2); // Wooden torch, Metal sconce
    /// ```
    pub fn default_presets(self) -> Vec<FurnitureAppearancePreset> {
        match self {
            FurnitureType::Throne => vec![
                FurnitureAppearancePreset {
                    name: "Wooden Throne",
                    material: FurnitureMaterial::Wood,
                    scale: 1.2,
                    color_tint: None,
                },
                FurnitureAppearancePreset {
                    name: "Stone Throne",
                    material: FurnitureMaterial::Stone,
                    scale: 1.3,
                    color_tint: None,
                },
                FurnitureAppearancePreset {
                    name: "Golden Throne",
                    material: FurnitureMaterial::Gold,
                    scale: 1.5,
                    color_tint: None,
                },
            ],
            FurnitureType::Torch => vec![
                FurnitureAppearancePreset {
                    name: "Wooden Torch",
                    material: FurnitureMaterial::Wood,
                    scale: 1.0,
                    color_tint: Some([1.0, 0.6, 0.2]), // Orange flame
                },
                FurnitureAppearancePreset {
                    name: "Metal Sconce",
                    material: FurnitureMaterial::Metal,
                    scale: 0.8,
                    color_tint: Some([0.6, 0.8, 1.0]), // Blue flame
                },
            ],
            _ => vec![FurnitureAppearancePreset {
                name: "Default",
                material: FurnitureMaterial::Wood,
                scale: 1.0,
                color_tint: None,
            }],
        }
    }
}

/// Furniture categories for palette organization
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum FurnitureCategory {
    /// Seating furniture (Throne, Bench, Chair)
    Seating,
    /// Storage furniture (Chest, Barrel, Bookshelf)
    Storage,
    /// Decorative furniture (Statue, Fountain, Altar)
    Decoration,
    /// Lighting furniture (Torch)
    Lighting,
    /// Utility furniture (Table, Crate)
    Utility,
}

impl FurnitureCategory {
    /// Returns human-readable name for the category
    pub fn name(self) -> &'static str {
        match self {
            FurnitureCategory::Seating => "Seating",
            FurnitureCategory::Storage => "Storage",
            FurnitureCategory::Decoration => "Decoration",
            FurnitureCategory::Lighting => "Lighting",
            FurnitureCategory::Utility => "Utility",
        }
    }

    /// Returns all category variants
    pub fn all() -> &'static [FurnitureCategory] {
        &[
            FurnitureCategory::Seating,
            FurnitureCategory::Storage,
            FurnitureCategory::Decoration,
            FurnitureCategory::Lighting,
            FurnitureCategory::Utility,
        ]
    }
}

// ===== Architectural Structure Components =====

/// Types of architectural structure components for dungeons and areas
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructureType {
    /// Vertical support column
    Column,
    /// Arched opening
    Arch,
    /// Wall segment
    WallSegment,
    /// Door frame
    DoorFrame,
    /// Safety railing
    Railing,
}

impl StructureType {
    /// Returns all structure type variants
    pub fn all() -> &'static [StructureType] {
        &[
            StructureType::Column,
            StructureType::Arch,
            StructureType::WallSegment,
            StructureType::DoorFrame,
            StructureType::Railing,
        ]
    }

    /// Returns human-readable name for the structure type
    pub fn name(self) -> &'static str {
        match self {
            StructureType::Column => "Column",
            StructureType::Arch => "Arch",
            StructureType::WallSegment => "Wall Segment",
            StructureType::DoorFrame => "Door Frame",
            StructureType::Railing => "Railing",
        }
    }
}

/// Architectural styles for columns
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColumnStyle {
    /// Plain cylindrical column
    Plain,
    /// Classical Doric style with simple capital
    Doric,
    /// Classical Ionic style with scroll capital
    Ionic,
}

impl ColumnStyle {
    /// Returns all column style variants
    pub fn all() -> &'static [ColumnStyle] {
        &[ColumnStyle::Plain, ColumnStyle::Doric, ColumnStyle::Ionic]
    }

    /// Returns human-readable name for the column style
    pub fn name(self) -> &'static str {
        match self {
            ColumnStyle::Plain => "Plain",
            ColumnStyle::Doric => "Doric",
            ColumnStyle::Ionic => "Ionic",
        }
    }
}

/// Configuration for column generation
#[derive(Clone, Debug)]
pub struct ColumnConfig {
    /// Height of the column (default: 3.0)
    pub height: f32,
    /// Radius of the column shaft (default: 0.3)
    pub radius: f32,
    /// Architectural style
    pub style: ColumnStyle,
}

impl Default for ColumnConfig {
    fn default() -> Self {
        Self {
            height: 3.0,
            radius: 0.3,
            style: ColumnStyle::Plain,
        }
    }
}

/// Configuration for arch generation
#[derive(Clone, Debug)]
pub struct ArchConfig {
    /// Width of the arch opening (default: 2.0)
    pub width: f32,
    /// Height to the top of the arch (default: 3.0)
    pub height: f32,
    /// Thickness of the arch structure (default: 0.3)
    pub thickness: f32,
}

impl Default for ArchConfig {
    fn default() -> Self {
        Self {
            width: 2.0,
            height: 3.0,
            thickness: 0.3,
        }
    }
}

/// Configuration for wall segment generation
#[derive(Clone, Debug)]
pub struct WallSegmentConfig {
    /// Length of the wall segment (default: 2.0)
    pub length: f32,
    /// Height of the wall segment (default: 2.5)
    pub height: f32,
    /// Thickness of the wall (default: 0.2)
    pub thickness: f32,
    /// Whether the wall has a window opening
    pub has_window: bool,
}

impl Default for WallSegmentConfig {
    fn default() -> Self {
        Self {
            length: 2.0,
            height: 2.5,
            thickness: 0.2,
            has_window: false,
        }
    }
}

/// Configuration for door frame generation
#[derive(Clone, Debug)]
pub struct DoorFrameConfig {
    /// Width of the door opening (default: 1.0)
    pub width: f32,
    /// Height of the door opening (default: 2.5)
    pub height: f32,
    /// Thickness of the frame (default: 0.15)
    pub frame_thickness: f32,
}

impl Default for DoorFrameConfig {
    fn default() -> Self {
        Self {
            width: 1.0,
            height: 2.5,
            frame_thickness: 0.15,
        }
    }
}

/// Configuration for railing generation
#[derive(Clone, Debug)]
pub struct RailingConfig {
    /// Length of the railing (default: 2.0)
    pub length: f32,
    /// Height of the railing (default: 1.0)
    pub height: f32,
    /// Radius of the posts (default: 0.08)
    pub post_radius: f32,
    /// Number of posts (default: 4)
    pub post_count: usize,
}

impl Default for RailingConfig {
    fn default() -> Self {
        Self {
            length: 2.0,
            height: 1.0,
            post_radius: 0.08,
            post_count: 4,
        }
    }
}

// ===== Performance & Polish Types (Phase 5) =====

/// Level of detail for procedurally generated objects
///
/// Distance-based visual simplification to improve performance on large maps.
/// Objects fade between detail levels as the camera moves away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DetailLevel {
    /// Full quality: complete branch graphs, all foliage, detailed geometry
    /// Distance: < 10 tiles from camera
    Full,
    /// Simplified: fewer branches, clustered foliage, reduced vertices
    /// Distance: 10-30 tiles from camera
    Simplified,
    /// Billboard: flat impostor sprite, no geometry
    /// Distance: > 30 tiles from camera
    Billboard,
}

impl DetailLevel {
    /// Get the squared distance threshold for this detail level (in world units)
    /// Used to avoid repeated sqrt calculations in distance checks
    ///
    /// # Returns
    ///
    /// Squared distance in world units (1 unit ≈ 10 feet, so 10 tiles ≈ 3.33 units)
    pub fn distance_threshold_squared(self) -> f32 {
        match self {
            DetailLevel::Full => 100.0,       // 10 tiles squared
            DetailLevel::Simplified => 900.0, // 30 tiles squared
            DetailLevel::Billboard => f32::INFINITY,
        }
    }

    /// Get the maximum distance for this detail level
    pub fn max_distance(self) -> f32 {
        match self {
            DetailLevel::Full => 10.0,
            DetailLevel::Simplified => 30.0,
            DetailLevel::Billboard => f32::INFINITY,
        }
    }

    /// Select the appropriate detail level for a given distance
    ///
    /// # Arguments
    ///
    /// * `distance` - Distance from camera to object in world units
    ///
    /// # Returns
    ///
    /// The recommended detail level for this distance
    pub fn from_distance(distance: f32) -> Self {
        if distance < 10.0 {
            DetailLevel::Full
        } else if distance < 30.0 {
            DetailLevel::Simplified
        } else {
            DetailLevel::Billboard
        }
    }
}

/// Configuration for GPU mesh instancing
///
/// Stores transform data for multiple instances of the same mesh to be drawn
/// in a single draw call, significantly reducing GPU overhead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceData {
    /// World position (x, y, z)
    pub position: [f32; 3],
    /// Scale (uniform)
    pub scale: f32,
    /// Rotation in radians around Y-axis
    pub rotation_y: f32,
}

impl InstanceData {
    /// Create a new instance at the specified position
    ///
    /// # Arguments
    ///
    /// * `position` - World coordinates [x, y, z]
    ///
    /// # Examples
    ///
    /// ```text
    /// use antares::domain::world::types::InstanceData;
    ///
    /// let instance = InstanceData::new([1.0, 0.0, 2.0]);
    /// assert_eq!(instance.position, [1.0, 0.0, 2.0]);
    /// assert_eq!(instance.scale, 1.0);
    /// assert_eq!(instance.rotation_y, 0.0);
    /// ```
    pub fn new(position: [f32; 3]) -> Self {
        Self {
            position,
            scale: 1.0,
            rotation_y: 0.0,
        }
    }

    /// Set the scale
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Set the rotation
    pub fn with_rotation(mut self, rotation_y: f32) -> Self {
        self.rotation_y = rotation_y;
        self
    }
}

/// Async mesh generation task identifier
///
/// Used to track background mesh generation tasks and retrieve their results
/// without blocking the main game loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AsyncMeshTaskId(pub u64);

impl AsyncMeshTaskId {
    /// Create a new task ID from a raw u64
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Configuration for async mesh generation
///
/// Controls how procedural meshes are generated on background threads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncMeshConfig {
    /// Maximum number of concurrent mesh generation tasks
    pub max_concurrent_tasks: usize,
    /// Whether to prioritize closer objects
    pub prioritize_by_distance: bool,
    /// Timeout in milliseconds for mesh generation
    pub generation_timeout_ms: u64,
}

impl Default for AsyncMeshConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 4,
            prioritize_by_distance: true,
            generation_timeout_ms: 5000,
        }
    }
}

// ===== Map Event System =====

/// Map events are special occurrences that can happen at specific tile locations.
///
/// Events are triggered when the party moves to a tile containing an event,
/// or when the party explicitly interacts with the environment. Each event type
/// has specific properties and effects on gameplay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MapEvent {
    /// Random monster encounter
    Encounter {
        /// Event name
        #[serde(default)]
        name: String,
        /// Event description
        #[serde(default)]
        description: String,
        /// Monster group IDs
        monster_group: Vec<u8>,
    },
    /// Treasure chest
    Treasure {
        /// Event name
        #[serde(default)]
        name: String,
        /// Event description
        #[serde(default)]
        description: String,
        /// Loot table or item IDs
        loot: Vec<u8>,
    },
    /// Teleport to another location
    Teleport {
        /// Event name
        #[serde(default)]
        name: String,
        /// Event description
        #[serde(default)]
        description: String,
        /// Destination position
        destination: Position,
        /// Target map ID
        map_id: MapId,
    },
    /// Trap that triggers
    Trap {
        /// Event name
        #[serde(default)]
        name: String,
        /// Event description
        #[serde(default)]
        description: String,
        /// Damage amount
        damage: u16,
        /// Status effect
        effect: Option<String>,
    },
    /// Sign with text
    Sign {
        /// Event name
        #[serde(default)]
        name: String,
        /// Event description
        #[serde(default)]
        description: String,
        /// Message text
        text: String,
    },
    /// NPC dialogue trigger
    NpcDialogue {
        /// Event name
        #[serde(default)]
        name: String,
        /// Event description
        #[serde(default)]
        description: String,
        /// NPC identifier (string-based ID for NPC database lookup)
        npc_id: crate::domain::world::NpcId,
    },
    /// Recruitable character encounter
    RecruitableCharacter {
        /// Event name
        #[serde(default)]
        name: String,
        /// Event description
        #[serde(default)]
        description: String,
        /// Character definition ID for recruitment
        character_id: String,
        /// Optional dialogue tree for recruitment interaction
        #[serde(default)]
        dialogue_id: Option<crate::domain::dialogue::DialogueId>,
    },
    /// Enter an inn for party management
    EnterInn {
        /// Event name
        #[serde(default)]
        name: String,
        /// Event description
        #[serde(default)]
        description: String,
        /// Innkeeper NPC identifier (must exist in NPC database with is_innkeeper=true)
        innkeeper_id: crate::domain::world::NpcId,
    },
    /// Furniture or prop placement event
    Furniture {
        /// Event name for editor display
        #[serde(default)]
        name: String,
        /// Type of furniture to spawn
        furniture_type: FurnitureType,
        /// Optional Y-axis rotation in degrees (0-360)
        #[serde(default)]
        rotation_y: Option<f32>,
        /// Scale multiplier (0.5-2.0, default 1.0)
        #[serde(default = "default_furniture_scale")]
        scale: f32,
        /// Material variant (Wood, Stone, Metal, Gold)
        #[serde(default)]
        material: FurnitureMaterial,
        /// Furniture-specific flags
        #[serde(default)]
        flags: FurnitureFlags,
        /// Optional color tint for customization (RGB, 0.0-1.0 range)
        #[serde(default)]
        color_tint: Option<[f32; 3]>,
    },
}

/// Default scale for furniture events (1.0x)
fn default_furniture_scale() -> f32 {
    1.0
}

/// Default dialogue ID for recruitment events when none specified
#[allow(dead_code)]
pub const DEFAULT_RECRUITMENT_DIALOGUE_ID: crate::domain::dialogue::DialogueId = 1000;

/// Encounter table definition for random encounters configured per map
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EncounterTable {
    /// Base chance per step (0.0 - 1.0) of triggering an encounter on this map
    #[serde(default = "default_encounter_rate")]
    pub encounter_rate: f32,

    /// Monster groups available in this area (each entry is a monster_group Vec<u8>)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<Vec<u8>>,

    /// Terrain-based modifiers to multiply the base encounter rate
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub terrain_modifiers: HashMap<TerrainType, f32>,
}

// ===== Resolved NPC =====

/// Resolved NPC combining placement and definition data
///
/// This struct merges an NPC placement (position, facing, overrides) with
/// the NPC definition (name, portrait, dialogue, etc.) from the database.
/// It's used at runtime after loading maps and resolving NPC references.
///
/// # Examples
///
/// ```
/// use antares::domain::world::ResolvedNpc;
/// use antares::domain::world::npc::{NpcDefinition, NpcPlacement};
/// use antares::domain::types::Position;
///
/// let definition = NpcDefinition::new("merchant_1", "Bob", "merchant.png");
/// let placement = NpcPlacement::new("merchant_1", Position::new(10, 15));
/// let resolved = ResolvedNpc::from_placement_and_definition(&placement, &definition);
///
/// assert_eq!(resolved.npc_id, "merchant_1");
/// assert_eq!(resolved.name, "Bob");
/// assert_eq!(resolved.position, Position::new(10, 15));
/// ```
#[derive(Debug, Clone)]
pub struct ResolvedNpc {
    /// NPC ID from definition
    pub npc_id: String,
    /// NPC name from definition
    pub name: String,
    /// NPC description from definition
    pub description: String,
    /// Portrait path from definition
    pub portrait_id: String,
    /// Optional sprite reference from NPC definition.
    /// When `Some`, runtime spawning prefers this over default placeholder.
    pub sprite: Option<SpriteReference>,
    /// Position from placement
    pub position: Position,
    /// Facing direction from placement
    pub facing: Option<Direction>,
    /// Effective dialogue ID (placement override or definition default)
    pub dialogue_id: Option<crate::domain::dialogue::DialogueId>,
    /// Quest IDs from definition
    pub quest_ids: Vec<crate::domain::quest::QuestId>,
    /// Faction from definition
    pub faction: Option<String>,
    /// Whether NPC is a merchant
    pub is_merchant: bool,
    /// Whether NPC is an innkeeper
    pub is_innkeeper: bool,
}

impl ResolvedNpc {
    /// Creates a ResolvedNpc from a placement and definition
    ///
    /// The dialogue_id uses the placement's override if present, otherwise
    /// falls back to the definition's default dialogue.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::ResolvedNpc;
    /// use antares::domain::world::npc::{NpcDefinition, NpcPlacement};
    /// use antares::domain::types::Position;
    ///
    /// let definition = NpcDefinition {
    ///     id: "guard".to_string(),
    ///     name: "City Guard".to_string(),
    ///     description: "A vigilant guard".to_string(),
    ///     portrait_id: "guard.png".to_string(),
    ///     sprite: None,
    ///     dialogue_id: Some(10),
    ///     quest_ids: vec![],
    ///     faction: Some("City Watch".to_string()),
    ///     is_merchant: false,
    ///     is_innkeeper: false,
    /// };
    ///
    /// let placement = NpcPlacement::new("guard", Position::new(5, 5));
    ///
    /// let resolved = ResolvedNpc::from_placement_and_definition(&placement, &definition);
    /// assert_eq!(resolved.dialogue_id, Some(10));
    /// ```
    pub fn from_placement_and_definition(
        placement: &crate::domain::world::npc::NpcPlacement,
        definition: &crate::domain::world::npc::NpcDefinition,
    ) -> Self {
        Self {
            npc_id: definition.id.clone(),
            name: definition.name.clone(),
            description: definition.description.clone(),
            portrait_id: definition.portrait_id.clone(),
            sprite: definition.sprite.clone(),
            position: placement.position,
            facing: placement.facing,
            dialogue_id: placement.dialogue_override.or(definition.dialogue_id),
            quest_ids: definition.quest_ids.clone(),
            faction: definition.faction.clone(),
            is_merchant: definition.is_merchant,
            is_innkeeper: definition.is_innkeeper,
        }
    }
}

// ===== Map =====

/// A map in the game world
///
/// Maps are 2D grids of tiles with events and NPCs.
///
/// # Examples
///
/// ```
/// use antares::domain::world::{Map, Tile, TerrainType, WallType};
///
/// let map = Map::new(1, "Test Map".to_string(), "Description".to_string(), 20, 20);
/// assert_eq!(map.width, 20);
/// assert_eq!(map.height, 20);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Map {
    /// Map identifier
    pub id: MapId,
    /// Map width in tiles
    pub width: u32,
    /// Map height in tiles
    pub height: u32,
    /// Map name
    #[serde(default = "default_map_name")]
    pub name: String,
    /// Map description
    #[serde(default)]
    pub description: String,
    /// 2D grid of tiles (row-major order: y * width + x)
    pub tiles: Vec<Tile>,
    /// Events at specific positions
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub events: HashMap<Position, MapEvent>,

    /// Optional random encounter table for this map
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encounter_table: Option<EncounterTable>,

    /// Whether random encounters are allowed on this map (towns/inns should set to false)
    #[serde(default = "default_allow_random_encounters")]
    pub allow_random_encounters: bool,

    /// NPC placements (references to NPC definitions)
    #[serde(default)]
    pub npc_placements: Vec<crate::domain::world::npc::NpcPlacement>,
}

fn default_map_name() -> String {
    "Unnamed Map".to_string()
}

fn default_encounter_rate() -> f32 {
    0.05 // Default 5% chance per step
}

fn default_allow_random_encounters() -> bool {
    true
}

impl Map {
    /// Creates a new map with the given given dimensions
    ///
    /// All tiles are initialized to ground terrain with no walls.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::Map;
    ///
    /// let map = Map::new(1, "Test Map".to_string(), "Description".to_string(), 10, 10);
    /// assert_eq!(map.width, 10);
    /// assert_eq!(map.height, 10);
    /// assert_eq!(map.tiles.len(), 100);
    /// ```
    pub fn new(id: MapId, name: String, description: String, width: u32, height: u32) -> Self {
        let mut tiles = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                tiles.push(Tile::new(
                    x as i32,
                    y as i32,
                    TerrainType::Ground,
                    WallType::None,
                ));
            }
        }

        Self {
            id,
            name,
            description,
            width,
            height,
            tiles,
            events: HashMap::new(),
            encounter_table: None,
            allow_random_encounters: default_allow_random_encounters(),
            npc_placements: Vec::new(),
        }
    }

    /// Gets a tile at the specified position
    ///
    /// Returns `None` if the position is out of bounds.
    pub fn get_tile(&self, pos: Position) -> Option<&Tile> {
        if self.is_valid_position(pos) {
            let index = (pos.y as usize * self.width as usize) + pos.x as usize;
            Some(&self.tiles[index])
        } else {
            None
        }
    }

    /// Gets a mutable reference to a tile at the specified position
    ///
    /// Returns `None` if the position is out of bounds.
    pub fn get_tile_mut(&mut self, pos: Position) -> Option<&mut Tile> {
        if self.is_valid_position(pos) {
            let index = (pos.y as usize * self.width as usize) + pos.x as usize;
            Some(&mut self.tiles[index])
        } else {
            None
        }
    }

    /// Returns true if the position is within map bounds
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::Map;
    /// use antares::domain::types::Position;
    ///
    /// let map = Map::new(1, "Test Map".to_string(), "Description".to_string(), 10, 10);
    /// assert!(map.is_valid_position(Position::new(5, 5)));
    /// assert!(!map.is_valid_position(Position::new(10, 10)));
    /// assert!(!map.is_valid_position(Position::new(-1, 5)));
    /// ```
    pub fn is_valid_position(&self, pos: Position) -> bool {
        pos.x >= 0 && pos.y >= 0 && pos.x < self.width as i32 && pos.y < self.height as i32
    }

    /// Returns true if the tile at the position is blocked
    ///
    /// This checks both tile blocking (walls, terrain) and NPC blocking.
    /// NPCs are considered blocking obstacles - the party cannot move through them.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::Map;
    /// use antares::domain::world::npc::NpcPlacement;
    /// use antares::domain::types::Position;
    ///
    /// let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
    ///
    /// // Position is not blocked initially
    /// assert!(!map.is_blocked(Position::new(5, 5)));
    ///
    /// // Add NPC placement at position
    /// map.npc_placements.push(NpcPlacement::new("guard", Position::new(5, 5)));
    ///
    /// // Now the position is blocked by the NPC
    /// assert!(map.is_blocked(Position::new(5, 5)));
    /// ```
    pub fn is_blocked(&self, pos: Position) -> bool {
        // Check tile blocking first
        if self.get_tile(pos).is_none_or(|tile| tile.is_blocked()) {
            return true;
        }

        // Check if any NPC placement occupies this position
        if self.npc_placements.iter().any(|npc| npc.position == pos) {
            return true;
        }

        false
    }

    /// Adds an event at the specified position
    pub fn add_event(&mut self, pos: Position, event: MapEvent) {
        self.events.insert(pos, event);
    }

    /// Gets an event at the specified position
    pub fn get_event(&self, pos: Position) -> Option<&MapEvent> {
        self.events.get(&pos)
    }

    /// Removes and returns an event at the specified position
    pub fn remove_event(&mut self, pos: Position) -> Option<MapEvent> {
        self.events.remove(&pos)
    }

    /// Gets the event at a specific position, if one exists
    ///
    /// # Arguments
    ///
    /// * `position` - The position to check for events
    ///
    /// # Returns
    ///
    /// Returns `Some(&MapEvent)` if an event exists at the position, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::{Map, MapEvent};
    /// use antares::domain::types::Position;
    ///
    /// let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
    /// let pos = Position::new(5, 5);
    /// let event = MapEvent::Sign {
    ///     name: "Test".to_string(),
    ///     description: "A sign".to_string(),
    ///     text: "Hello!".to_string(),
    /// };
    /// map.add_event(pos, event);
    ///
    /// assert!(map.get_event_at_position(pos).is_some());
    /// assert!(map.get_event_at_position(Position::new(0, 0)).is_none());
    /// ```
    pub fn get_event_at_position(&self, position: Position) -> Option<&MapEvent> {
        self.get_event(position)
    }

    /// Resolves NPC placements using the NPC database
    ///
    /// This method takes the NPC placements on the map and resolves them
    /// against the NPC database to create `ResolvedNpc` instances that
    /// combine placement data (position, facing) with definition data
    /// (name, portrait, dialogue, etc.).
    ///
    /// NPCs that reference IDs not found in the database are skipped with
    /// a warning (in production, consider logging).
    ///
    /// # Arguments
    ///
    /// * `npc_db` - Reference to the NPC database
    ///
    /// # Returns
    ///
    /// Returns a vector of `ResolvedNpc` instances
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::Map;
    /// use antares::domain::world::ResolvedNpc;
    /// use antares::domain::world::npc::{NpcDefinition, NpcPlacement};
    /// use antares::domain::types::Position;
    /// use antares::sdk::database::NpcDatabase;
    ///
    /// let mut npc_db = NpcDatabase::new();
    /// let npc_def = NpcDefinition::new("merchant_1", "Bob", "merchant.png");
    /// npc_db.add_npc(npc_def).unwrap();
    ///
    /// let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
    /// map.npc_placements.push(NpcPlacement::new("merchant_1", Position::new(5, 5)));
    ///
    /// let resolved = map.resolve_npcs(&npc_db);
    /// assert_eq!(resolved.len(), 1);
    /// assert_eq!(resolved[0].name, "Bob");
    /// ```
    pub fn resolve_npcs(&self, npc_db: &crate::sdk::database::NpcDatabase) -> Vec<ResolvedNpc> {
        self.npc_placements
            .iter()
            .filter_map(|placement| {
                if let Some(definition) = npc_db.get_npc(&placement.npc_id) {
                    Some(ResolvedNpc::from_placement_and_definition(
                        placement, definition,
                    ))
                } else {
                    // NPC definition not found in database
                    // In production, this should log a warning
                    eprintln!(
                        "Warning: NPC '{}' not found in database on map {}",
                        placement.npc_id, self.id
                    );
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod map_npc_resolution_tests {
    use super::*;
    use crate::domain::world::npc::{NpcDefinition, NpcPlacement};
    use crate::sdk::database::NpcDatabase;

    #[test]
    fn test_resolve_npcs_with_single_npc() {
        // Arrange
        let mut npc_db = NpcDatabase::new();
        let npc_def = NpcDefinition::new("merchant_bob", "Bob the Merchant", "merchant.png");
        npc_db.add_npc(npc_def).expect("Failed to add NPC");

        let mut map = Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);
        map.npc_placements
            .push(NpcPlacement::new("merchant_bob", Position::new(5, 5)));

        // Act
        let resolved = map.resolve_npcs(&npc_db);

        // Assert
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].npc_id, "merchant_bob");
        assert_eq!(resolved[0].name, "Bob the Merchant");
        assert_eq!(resolved[0].position, Position::new(5, 5));
        assert_eq!(resolved[0].portrait_id, "merchant.png");
    }

    #[test]
    fn test_resolve_npcs_with_multiple_npcs() {
        // Arrange
        let mut npc_db = NpcDatabase::new();
        npc_db
            .add_npc(NpcDefinition::merchant(
                "merchant_1",
                "Merchant Shop",
                "merchant.png",
            ))
            .unwrap();
        npc_db
            .add_npc(NpcDefinition::innkeeper(
                "innkeeper_1",
                "Friendly Inn",
                "innkeeper.png",
            ))
            .unwrap();

        let mut map = Map::new(1, "Town".to_string(), "Town map".to_string(), 20, 20);
        map.npc_placements
            .push(NpcPlacement::new("merchant_1", Position::new(5, 5)));
        map.npc_placements
            .push(NpcPlacement::new("innkeeper_1", Position::new(10, 10)));

        // Act
        let resolved = map.resolve_npcs(&npc_db);

        // Assert
        assert_eq!(resolved.len(), 2);
        assert!(resolved.iter().any(|n| n.npc_id == "merchant_1"));
        assert!(resolved.iter().any(|n| n.npc_id == "innkeeper_1"));
        assert!(resolved.iter().any(|n| n.is_merchant));
        assert!(resolved.iter().any(|n| n.is_innkeeper));
    }

    #[test]
    fn test_resolve_npcs_with_missing_definition() {
        // Arrange
        let npc_db = NpcDatabase::new(); // Empty database

        let mut map = Map::new(1, "Test Map".to_string(), "Desc".to_string(), 10, 10);
        map.npc_placements
            .push(NpcPlacement::new("nonexistent_npc", Position::new(5, 5)));

        // Act
        let resolved = map.resolve_npcs(&npc_db);

        // Assert
        assert_eq!(resolved.len(), 0, "Missing NPCs should be skipped");
    }

    #[test]
    fn test_resolve_npcs_with_dialogue_override() {
        // Arrange
        let mut npc_db = NpcDatabase::new();
        let npc_def = NpcDefinition {
            id: "guard".to_string(),
            name: "City Guard".to_string(),
            description: "Vigilant guard".to_string(),
            portrait_id: "guard.png".to_string(),
            dialogue_id: Some(10),
            sprite: None,
            quest_ids: vec![],
            faction: Some("City Watch".to_string()),
            is_merchant: false,
            is_innkeeper: false,
        };
        npc_db.add_npc(npc_def).unwrap();

        let mut map = Map::new(1, "City".to_string(), "City map".to_string(), 20, 20);
        let mut placement = NpcPlacement::new("guard", Position::new(5, 5));
        placement.dialogue_override = Some(99); // Override dialogue
        map.npc_placements.push(placement);

        // Act
        let resolved = map.resolve_npcs(&npc_db);

        // Assert
        assert_eq!(resolved.len(), 1);
        assert_eq!(
            resolved[0].dialogue_id,
            Some(99),
            "Should use placement override"
        );
    }

    #[test]
    fn test_resolve_npcs_with_quest_givers() {
        // Arrange
        let mut npc_db = NpcDatabase::new();
        let npc_def = NpcDefinition {
            id: "quest_giver".to_string(),
            name: "Elder".to_string(),
            description: "Village elder".to_string(),
            portrait_id: "elder.png".to_string(),
            sprite: None,
            dialogue_id: Some(5),
            quest_ids: vec![1, 2, 3],
            faction: Some("Village".to_string()),
            is_merchant: false,
            is_innkeeper: false,
        };
        npc_db.add_npc(npc_def).unwrap();

        let mut map = Map::new(1, "Village".to_string(), "Village map".to_string(), 15, 15);
        map.npc_placements
            .push(NpcPlacement::new("quest_giver", Position::new(7, 7)));

        // Act
        let resolved = map.resolve_npcs(&npc_db);

        // Assert
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].quest_ids, vec![1, 2, 3]);
        assert_eq!(resolved[0].faction, Some("Village".to_string()));
    }

    #[test]
    fn test_resolved_npc_from_placement_and_definition() {
        // Arrange
        let definition = NpcDefinition {
            id: "test_npc".to_string(),
            name: "Test NPC".to_string(),
            description: "A test NPC".to_string(),
            portrait_id: "test.png".to_string(),
            sprite: None,
            dialogue_id: Some(42),
            quest_ids: vec![1],
            faction: Some("Test Faction".to_string()),
            is_merchant: true,
            is_innkeeper: false,
        };

        let placement = NpcPlacement {
            npc_id: "test_npc".to_string(),
            position: Position::new(3, 4),
            facing: Some(Direction::North),
            dialogue_override: None,
        };

        // Act
        let resolved = ResolvedNpc::from_placement_and_definition(&placement, &definition);

        // Assert
        assert_eq!(resolved.npc_id, "test_npc");
        assert_eq!(resolved.name, "Test NPC");
        assert_eq!(resolved.description, "A test NPC");
        assert_eq!(resolved.portrait_id, "test.png");
        assert_eq!(resolved.position, Position::new(3, 4));
        assert_eq!(resolved.facing, Some(Direction::North));
        assert_eq!(resolved.dialogue_id, Some(42));
        assert_eq!(resolved.quest_ids, vec![1]);
        assert_eq!(resolved.faction, Some("Test Faction".to_string()));
        assert!(resolved.is_merchant);
        assert!(!resolved.is_innkeeper);
    }

    #[test]
    fn test_resolved_npc_from_placement_copies_sprite_field_when_present() {
        // Arrange
        let sprite = SpriteReference {
            sheet_path: "sprites/actors/knight.png".to_string(),
            sprite_index: 7,
            animation: None,
            material_properties: None,
        };
        let definition =
            NpcDefinition::new("knight", "Knight", "knight.png").with_sprite(sprite.clone());
        let placement = NpcPlacement::new("knight", Position::new(1, 1));

        // Act
        let resolved = ResolvedNpc::from_placement_and_definition(&placement, &definition);

        // Assert
        assert!(resolved.sprite.is_some());
        assert_eq!(
            resolved.sprite.as_ref().unwrap().sheet_path,
            "sprites/actors/knight.png"
        );
        assert_eq!(resolved.sprite.as_ref().unwrap().sprite_index, 7);
    }

    #[test]
    fn test_resolved_npc_from_placement_sprite_none_when_definition_none() {
        // Arrange
        let definition = NpcDefinition::new("generic", "Generic NPC", "generic.png");
        let placement = NpcPlacement::new("generic", Position::new(2, 2));

        // Act
        let resolved = ResolvedNpc::from_placement_and_definition(&placement, &definition);

        // Assert
        assert!(resolved.sprite.is_none());
    }

    #[test]
    fn test_resolved_npc_uses_dialogue_override() {
        // Arrange
        let definition = NpcDefinition {
            id: "npc".to_string(),
            name: "NPC".to_string(),
            description: "".to_string(),
            portrait_id: "npc.png".to_string(),
            sprite: None,
            dialogue_id: Some(10),
            quest_ids: vec![],
            faction: None,
            is_merchant: false,
            is_innkeeper: false,
        };

        let placement = NpcPlacement {
            npc_id: "npc".to_string(),
            position: Position::new(0, 0),
            facing: None,
            dialogue_override: Some(20),
        };

        // Act
        let resolved = ResolvedNpc::from_placement_and_definition(&placement, &definition);

        // Assert
        assert_eq!(
            resolved.dialogue_id,
            Some(20),
            "Should use placement override, not definition default"
        );
    }

    #[test]
    fn test_resolve_npcs_empty_placements() {
        // Arrange
        let mut npc_db = NpcDatabase::new();
        npc_db
            .add_npc(NpcDefinition::new("npc1", "NPC 1", "npc1.png"))
            .unwrap();

        let map = Map::new(1, "Empty".to_string(), "No NPCs".to_string(), 10, 10);
        // No placements added

        // Act
        let resolved = map.resolve_npcs(&npc_db);

        // Assert
        assert_eq!(resolved.len(), 0);
    }
}

// ===== World =====

/// The game world containing all maps
///
/// The world manages multiple maps, tracks the party's current location,
/// and handles map transitions.
///
/// # Examples
///
/// ```
/// use antares::domain::world::{World, Map};
/// use antares::domain::types::{Position, Direction};
///
/// let mut world = World::new();
/// let map = Map::new(1, "Test Map".to_string(), "Description".to_string(), 20, 20);
/// world.add_map(map);
/// world.set_current_map(1);
/// assert_eq!(world.current_map, 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    /// All maps in the world
    pub maps: HashMap<MapId, Map>,
    /// Current map ID
    pub current_map: MapId,
    /// Party position on current map
    pub party_position: Position,
    /// Direction party is facing
    pub party_facing: Direction,
}

impl World {
    /// Creates a new empty world
    pub fn new() -> Self {
        Self {
            maps: HashMap::new(),
            current_map: 0,
            party_position: Position::new(0, 0),
            party_facing: Direction::North,
        }
    }

    /// Adds a map to the world
    pub fn add_map(&mut self, map: Map) {
        let map_id = map.id;
        self.maps.insert(map_id, map);
    }

    /// Gets a reference to a map by ID
    pub fn get_map(&self, map_id: MapId) -> Option<&Map> {
        self.maps.get(&map_id)
    }

    /// Gets a mutable reference to a map by ID
    pub fn get_map_mut(&mut self, map_id: MapId) -> Option<&mut Map> {
        self.maps.get_mut(&map_id)
    }

    /// Gets a reference to the current map
    pub fn get_current_map(&self) -> Option<&Map> {
        self.maps.get(&self.current_map)
    }

    /// Gets a mutable reference to the current map
    pub fn get_current_map_mut(&mut self) -> Option<&mut Map> {
        self.maps.get_mut(&self.current_map)
    }

    /// Sets the current map
    pub fn set_current_map(&mut self, map_id: MapId) {
        self.current_map = map_id;
    }

    /// Sets the party position
    pub fn set_party_position(&mut self, position: Position) {
        self.party_position = position;
    }

    /// Sets the party facing direction
    pub fn set_party_facing(&mut self, direction: Direction) {
        self.party_facing = direction;
    }

    /// Turns the party left
    pub fn turn_left(&mut self) {
        self.party_facing = self.party_facing.turn_left();
    }

    /// Turns the party right
    pub fn turn_right(&mut self) {
        self.party_facing = self.party_facing.turn_right();
    }

    /// Gets the position in front of the party
    pub fn position_ahead(&self) -> Position {
        self.party_facing.forward(self.party_position)
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== TileVisualMetadata Tests =====

    #[test]
    fn test_tile_visual_metadata_default() {
        let metadata = TileVisualMetadata::default();
        assert_eq!(metadata.height, None);
        assert_eq!(metadata.width_x, None);
        assert_eq!(metadata.width_z, None);
        assert_eq!(metadata.color_tint, None);
        assert_eq!(metadata.scale, None);
        assert_eq!(metadata.y_offset, None);
    }

    #[test]
    fn test_effective_height_wall() {
        let metadata = TileVisualMetadata::default();
        assert_eq!(
            metadata.effective_height(TerrainType::Ground, WallType::Normal),
            2.5
        );
    }

    #[test]
    fn test_effective_height_door() {
        let metadata = TileVisualMetadata::default();
        assert_eq!(
            metadata.effective_height(TerrainType::Ground, WallType::Door),
            2.5
        );
    }

    #[test]
    fn test_effective_height_torch() {
        let metadata = TileVisualMetadata::default();
        assert_eq!(
            metadata.effective_height(TerrainType::Ground, WallType::Torch),
            2.5
        );
    }

    #[test]
    fn test_effective_height_mountain() {
        let metadata = TileVisualMetadata::default();
        assert_eq!(
            metadata.effective_height(TerrainType::Mountain, WallType::None),
            3.0
        );
    }

    #[test]
    fn test_effective_height_forest() {
        let metadata = TileVisualMetadata::default();
        assert_eq!(
            metadata.effective_height(TerrainType::Forest, WallType::None),
            2.2
        );
    }

    #[test]
    fn test_effective_height_flat_terrain() {
        let metadata = TileVisualMetadata::default();
        assert_eq!(
            metadata.effective_height(TerrainType::Ground, WallType::None),
            0.0
        );
        assert_eq!(
            metadata.effective_height(TerrainType::Grass, WallType::None),
            0.0
        );
    }

    #[test]
    fn test_effective_height_custom() {
        let metadata = TileVisualMetadata {
            height: Some(5.0),
            ..Default::default()
        };
        assert_eq!(
            metadata.effective_height(TerrainType::Ground, WallType::Normal),
            5.0
        );
        assert_eq!(
            metadata.effective_height(TerrainType::Mountain, WallType::None),
            5.0
        );
    }

    #[test]
    fn test_mesh_dimensions_default() {
        let metadata = TileVisualMetadata::default();
        let (x, h, z) = metadata.mesh_dimensions(TerrainType::Ground, WallType::Normal);
        assert_eq!((x, h, z), (1.0, 2.5, 1.0));
    }

    #[test]
    fn test_mesh_dimensions_custom() {
        let metadata = TileVisualMetadata {
            width_x: Some(0.8),
            height: Some(1.5),
            width_z: Some(0.6),
            ..Default::default()
        };
        let (x, h, z) = metadata.mesh_dimensions(TerrainType::Ground, WallType::Normal);
        assert_eq!((x, h, z), (0.8, 1.5, 0.6));
    }

    #[test]
    fn test_mesh_dimensions_with_scale() {
        let metadata = TileVisualMetadata {
            scale: Some(2.0),
            ..Default::default()
        };
        let (x, h, z) = metadata.mesh_dimensions(TerrainType::Ground, WallType::Normal);
        assert_eq!((x, h, z), (2.0, 5.0, 2.0)); // 1.0*2.0, 2.5*2.0, 1.0*2.0
    }

    #[test]
    fn test_map_event_enter_inn_ron_serialization() {
        // Verify that MapEvent::EnterInn with an innkeeper_id string
        // round-trips correctly through RON serialization/deserialization.
        let event = MapEvent::EnterInn {
            name: "Cozy Inn Entrance".to_string(),
            description: "A welcoming inn".to_string(),
            innkeeper_id: "cozy_inn".to_string(),
        };

        let ron_str = ron::ser::to_string_pretty(&event, Default::default())
            .expect("Failed to serialize MapEvent::EnterInn to RON");

        let parsed: MapEvent =
            ron::de::from_str(&ron_str).expect("Failed to deserialize MapEvent::EnterInn from RON");

        match parsed {
            MapEvent::EnterInn {
                name,
                description,
                innkeeper_id,
            } => {
                assert_eq!(name, "Cozy Inn Entrance".to_string());
                assert_eq!(description, "A welcoming inn".to_string());
                assert_eq!(innkeeper_id, "cozy_inn".to_string());
            }
            _ => panic!("Expected EnterInn event"),
        }
    }

    #[test]
    fn test_mesh_dimensions_custom_with_scale() {
        let metadata = TileVisualMetadata {
            width_x: Some(0.5),
            height: Some(1.0),
            width_z: Some(0.5),
            scale: Some(2.0),
            ..Default::default()
        };
        let (x, h, z) = metadata.mesh_dimensions(TerrainType::Ground, WallType::Normal);
        assert_eq!((x, h, z), (1.0, 2.0, 1.0)); // 0.5*2.0, 1.0*2.0, 0.5*2.0
    }

    #[test]
    fn test_mesh_y_position_wall() {
        let metadata = TileVisualMetadata::default();
        assert_eq!(
            metadata.mesh_y_position(TerrainType::Ground, WallType::Normal),
            1.25
        ); // 2.5 / 2.0
    }

    #[test]
    fn test_mesh_y_position_mountain() {
        let metadata = TileVisualMetadata::default();
        assert_eq!(
            metadata.mesh_y_position(TerrainType::Mountain, WallType::None),
            1.5
        ); // 3.0 / 2.0
    }

    #[test]
    fn test_mesh_y_position_forest() {
        let metadata = TileVisualMetadata::default();
        assert_eq!(
            metadata.mesh_y_position(TerrainType::Forest, WallType::None),
            1.1
        ); // 2.2 / 2.0
    }

    #[test]
    fn test_mesh_y_position_custom_offset() {
        let metadata = TileVisualMetadata {
            y_offset: Some(0.5),
            ..Default::default()
        };
        assert_eq!(
            metadata.mesh_y_position(TerrainType::Ground, WallType::Normal),
            1.75
        ); // (2.5 / 2.0) + 0.5
    }

    #[test]
    fn test_mesh_y_position_with_scale() {
        let metadata = TileVisualMetadata {
            scale: Some(2.0),
            ..Default::default()
        };
        assert_eq!(
            metadata.mesh_y_position(TerrainType::Ground, WallType::Normal),
            2.5
        ); // (2.5 * 2.0) / 2.0
    }

    #[test]
    fn test_mesh_y_position_scale_and_offset() {
        let metadata = TileVisualMetadata {
            scale: Some(2.0),
            y_offset: Some(1.0),
            ..Default::default()
        };
        assert_eq!(
            metadata.mesh_y_position(TerrainType::Ground, WallType::Normal),
            3.5
        ); // ((2.5 * 2.0) / 2.0) + 1.0
    }

    #[test]
    fn test_effective_width_x_default() {
        let metadata = TileVisualMetadata::default();
        assert_eq!(metadata.effective_width_x(), 1.0);
    }

    #[test]
    fn test_effective_width_x_custom() {
        let metadata = TileVisualMetadata {
            width_x: Some(0.5),
            ..Default::default()
        };
        assert_eq!(metadata.effective_width_x(), 0.5);
    }

    #[test]
    fn test_effective_width_z_default() {
        let metadata = TileVisualMetadata::default();
        assert_eq!(metadata.effective_width_z(), 1.0);
    }

    #[test]
    fn test_effective_width_z_custom() {
        let metadata = TileVisualMetadata {
            width_z: Some(0.7),
            ..Default::default()
        };
        assert_eq!(metadata.effective_width_z(), 0.7);
    }

    #[test]
    fn test_effective_scale_default() {
        let metadata = TileVisualMetadata::default();
        assert_eq!(metadata.effective_scale(), 1.0);
    }

    #[test]
    fn test_effective_scale_custom() {
        let metadata = TileVisualMetadata {
            scale: Some(1.5),
            ..Default::default()
        };
        assert_eq!(metadata.effective_scale(), 1.5);
    }

    #[test]
    fn test_effective_y_offset_default() {
        let metadata = TileVisualMetadata::default();
        assert_eq!(metadata.effective_y_offset(), 0.0);
    }

    #[test]
    fn test_effective_y_offset_custom() {
        let metadata = TileVisualMetadata {
            y_offset: Some(-0.5),
            ..Default::default()
        };
        assert_eq!(metadata.effective_y_offset(), -0.5);
    }

    #[test]
    fn test_tile_builder_with_height() {
        let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal).with_height(3.0);
        assert_eq!(tile.visual.height, Some(3.0));
    }

    #[test]
    fn test_tile_builder_with_dimensions() {
        let tile =
            Tile::new(0, 0, TerrainType::Ground, WallType::Normal).with_dimensions(0.8, 2.0, 0.9);
        assert_eq!(tile.visual.width_x, Some(0.8));
        assert_eq!(tile.visual.height, Some(2.0));
        assert_eq!(tile.visual.width_z, Some(0.9));
    }

    #[test]
    fn test_tile_builder_with_color_tint() {
        let tile =
            Tile::new(0, 0, TerrainType::Ground, WallType::Normal).with_color_tint(1.0, 0.5, 0.25);
        assert_eq!(tile.visual.color_tint, Some((1.0, 0.5, 0.25)));
    }

    #[test]
    fn test_tile_builder_with_scale() {
        let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal).with_scale(1.5);
        assert_eq!(tile.visual.scale, Some(1.5));
    }

    #[test]
    fn test_tile_builder_chain() {
        let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal)
            .with_height(2.0)
            .with_scale(1.5)
            .with_color_tint(0.8, 0.8, 1.0);
        assert_eq!(tile.visual.height, Some(2.0));
        assert_eq!(tile.visual.scale, Some(1.5));
        assert_eq!(tile.visual.color_tint, Some((0.8, 0.8, 1.0)));
    }

    #[test]
    fn test_serde_backward_compat() {
        // Old format without visual field should deserialize with default
        let ron_data = r#"(
            terrain: Ground,
            wall_type: Normal,
            blocked: true,
            is_special: false,
            is_dark: false,
            visited: false,
            x: 5,
            y: 10,
        )"#;
        let tile: Tile = ron::from_str(ron_data).expect("Failed to deserialize");
        assert_eq!(tile.x, 5);
        assert_eq!(tile.y, 10);
        assert_eq!(tile.visual, TileVisualMetadata::default());
    }

    #[test]
    fn test_serde_with_visual() {
        // New format with visual field should round-trip correctly
        let tile = Tile::new(3, 7, TerrainType::Mountain, WallType::None)
            .with_height(4.0)
            .with_color_tint(0.5, 0.5, 0.5);

        let serialized = ron::to_string(&tile).expect("Failed to serialize");
        let deserialized: Tile = ron::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(deserialized.x, tile.x);
        assert_eq!(deserialized.y, tile.y);
        assert_eq!(deserialized.terrain, tile.terrain);
        assert_eq!(deserialized.visual.height, Some(4.0));
        assert_eq!(deserialized.visual.color_tint, Some((0.5, 0.5, 0.5)));
    }

    // ===== Existing Tile Tests =====

    #[test]
    fn test_tile_creation() {
        let tile = Tile::new(0, 0, TerrainType::Ground, WallType::None);
        assert_eq!(tile.terrain, TerrainType::Ground);
        assert_eq!(tile.wall_type, WallType::None);
        assert!(!tile.blocked);
        assert!(!tile.visited);
        assert_eq!(tile.x, 0);
        assert_eq!(tile.y, 0);

        let wall_tile = Tile::new(1, 1, TerrainType::Ground, WallType::Normal);
        assert!(wall_tile.blocked);
    }

    #[test]
    fn test_tile_door() {
        let door = Tile::new(0, 0, TerrainType::Ground, WallType::Door);
        assert!(door.is_door());
        assert!(!door.has_light());
    }

    #[test]
    fn test_tile_blocked_terrain() {
        let water = Tile::new(0, 0, TerrainType::Water, WallType::None);
        assert!(water.is_blocked());

        let mountain = Tile::new(0, 0, TerrainType::Mountain, WallType::None);
        assert!(mountain.is_blocked());
    }

    #[test]
    fn test_map_bounds() {
        let map = Map::new(1, "Map".to_string(), "Desc".to_string(), 10, 10);
        assert_eq!(map.width, 10);
        assert_eq!(map.height, 10);

        assert!(map.is_valid_position(Position::new(0, 0)));
        assert!(map.is_valid_position(Position::new(9, 9)));
        assert!(!map.is_valid_position(Position::new(10, 10)));
        assert!(!map.is_valid_position(Position::new(-1, 0)));
    }

    #[test]
    fn test_map_tile_access() {
        let map = Map::new(1, "Map".to_string(), "Desc".to_string(), 10, 10);
        let tile = map.get_tile(Position::new(5, 5));
        assert!(tile.is_some());
        assert_eq!(tile.unwrap().terrain, TerrainType::Ground);

        let out_of_bounds = map.get_tile(Position::new(10, 10));
        assert!(out_of_bounds.is_none());
    }

    #[test]
    fn test_map_events() {
        let mut map = Map::new(1, "Map".to_string(), "Desc".to_string(), 10, 10);
        let pos = Position::new(5, 5);
        let event = MapEvent::Sign {
            name: "Sign".to_string(),
            description: "Desc".to_string(),
            text: "Welcome!".to_string(),
        };

        map.add_event(pos, event);
        assert!(map.get_event(pos).is_some());

        let removed = map.remove_event(pos);
        assert!(removed.is_some());
        assert!(map.get_event(pos).is_none());
    }

    #[test]
    fn test_world_map_access() {
        let mut world = World::new();
        let map = Map::new(1, "Map".to_string(), "Desc".to_string(), 20, 20);
        world.add_map(map);

        world.set_current_map(1);
        assert_eq!(world.current_map, 1);
        assert!(world.get_current_map().is_some());
    }

    #[test]
    fn test_world_party_movement() {
        let mut world = World::new();
        world.set_party_position(Position::new(5, 5));
        world.set_party_facing(Direction::North);

        assert_eq!(world.party_position, Position::new(5, 5));
        assert_eq!(world.party_facing, Direction::North);

        let ahead = world.position_ahead();
        assert_eq!(ahead, Position::new(5, 4));
    }

    #[test]
    fn test_world_turn() {
        let mut world = World::new();
        world.set_party_facing(Direction::North);

        world.turn_right();
        assert_eq!(world.party_facing, Direction::East);

        world.turn_left();
        assert_eq!(world.party_facing, Direction::North);
    }

    #[test]
    fn test_map_get_event_at_position_returns_event() {
        // Arrange
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let pos = Position::new(5, 5);
        let event = MapEvent::Sign {
            name: "Test Sign".to_string(),
            description: "A test sign".to_string(),
            text: "Hello, World!".to_string(),
        };
        map.add_event(pos, event.clone());

        // Act
        let result = map.get_event_at_position(pos);

        // Assert
        assert!(result.is_some());
        match result.unwrap() {
            MapEvent::Sign { text, .. } => assert_eq!(text, "Hello, World!"),
            _ => panic!("Expected Sign event"),
        }
    }

    #[test]
    fn test_map_get_event_at_position_returns_none_when_no_event() {
        // Arrange
        let map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let pos = Position::new(5, 5);

        // Act
        let result = map.get_event_at_position(pos);

        // Assert
        assert!(result.is_none());
    }

    // ===== NPC Blocking Tests =====

    #[test]
    fn test_is_blocked_empty_tile_not_blocked() {
        // Arrange
        let map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let pos = Position::new(5, 5);

        // Act & Assert
        assert!(
            !map.is_blocked(pos),
            "Empty ground tile should not be blocked"
        );
    }

    #[test]
    fn test_is_blocked_tile_with_wall_is_blocked() {
        // Arrange
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let pos = Position::new(5, 5);

        // Set tile as blocked (wall)
        if let Some(tile) = map.get_tile_mut(pos) {
            tile.wall_type = WallType::Normal;
            tile.blocked = true;
        }

        // Act & Assert
        assert!(map.is_blocked(pos), "Tile with wall should be blocked");
    }

    #[test]
    fn test_is_blocked_npc_placement_blocks_movement() {
        // Arrange
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let npc_pos = Position::new(5, 5);

        // Add NPC placement
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "guard", npc_pos,
            ));

        // Act & Assert
        assert!(
            map.is_blocked(npc_pos),
            "Position with NPC placement should be blocked"
        );
        assert!(
            !map.is_blocked(Position::new(6, 5)),
            "Adjacent position should not be blocked"
        );
    }

    #[test]
    fn test_is_blocked_multiple_npcs_at_different_positions() {
        // Arrange
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 20, 20);

        // Add multiple NPC placements
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "guard1",
                Position::new(5, 5),
            ));
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "guard2",
                Position::new(10, 10),
            ));
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "merchant",
                Position::new(15, 15),
            ));

        // Act & Assert
        assert!(
            map.is_blocked(Position::new(5, 5)),
            "First NPC position should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(10, 10)),
            "Second NPC position should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(15, 15)),
            "Third NPC position should be blocked"
        );
        assert!(
            !map.is_blocked(Position::new(7, 7)),
            "Empty position should not be blocked"
        );
    }

    #[test]
    fn test_is_blocked_out_of_bounds_is_blocked() {
        // Arrange
        let map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);

        // Act & Assert
        assert!(
            map.is_blocked(Position::new(-1, 5)),
            "Negative X should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(5, -1)),
            "Negative Y should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(10, 5)),
            "X >= width should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(5, 10)),
            "Y >= height should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(100, 100)),
            "Far out of bounds should be blocked"
        );
    }

    #[test]
    fn test_is_blocked_npc_on_walkable_tile_blocks() {
        // Arrange
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let pos = Position::new(5, 5);

        // Verify tile is walkable first
        assert!(!map.is_blocked(pos), "Tile should be walkable initially");

        // Add NPC placement
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new("npc", pos));

        // Act & Assert
        assert!(
            map.is_blocked(pos),
            "NPC on walkable tile should block movement"
        );
    }

    #[test]
    fn test_is_blocked_wall_and_npc_both_block() {
        // Arrange
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);
        let pos = Position::new(5, 5);

        // Set tile as blocked
        if let Some(tile) = map.get_tile_mut(pos) {
            tile.wall_type = WallType::Normal;
            tile.blocked = true;
        }

        // Also add NPC (unusual case but tests priority)
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new("npc", pos));

        // Act & Assert
        assert!(
            map.is_blocked(pos),
            "Position with wall and NPC should be blocked"
        );
    }

    #[test]
    fn test_is_blocked_boundary_conditions() {
        // Arrange
        let mut map = Map::new(1, "Test".to_string(), "Desc".to_string(), 10, 10);

        // Add NPCs at corners and edges
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "npc1",
                Position::new(0, 0), // Top-left corner
            ));
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "npc2",
                Position::new(9, 9), // Bottom-right corner
            ));
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "npc3",
                Position::new(0, 9), // Bottom-left corner
            ));
        map.npc_placements
            .push(crate::domain::world::npc::NpcPlacement::new(
                "npc4",
                Position::new(9, 0), // Top-right corner
            ));

        // Act & Assert
        assert!(
            map.is_blocked(Position::new(0, 0)),
            "Top-left corner should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(9, 9)),
            "Bottom-right corner should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(0, 9)),
            "Bottom-left corner should be blocked"
        );
        assert!(
            map.is_blocked(Position::new(9, 0)),
            "Top-right corner should be blocked"
        );
        assert!(
            !map.is_blocked(Position::new(5, 5)),
            "Center should not be blocked"
        );
    }

    #[test]
    fn test_rotation_y_default() {
        let metadata = TileVisualMetadata::default();
        assert_eq!(metadata.rotation_y, None);
        assert_eq!(metadata.effective_rotation_y(), 0.0);
    }

    #[test]
    fn test_rotation_y_custom_value() {
        let metadata = TileVisualMetadata {
            rotation_y: Some(45.0),
            ..Default::default()
        };
        assert_eq!(metadata.effective_rotation_y(), 45.0);
    }

    #[test]
    fn test_rotation_y_radians_conversion() {
        let metadata0 = TileVisualMetadata {
            rotation_y: Some(0.0),
            ..Default::default()
        };
        assert!((metadata0.rotation_y_radians() - 0.0).abs() < 0.001);

        let metadata90 = TileVisualMetadata {
            rotation_y: Some(90.0),
            ..Default::default()
        };
        assert!((metadata90.rotation_y_radians() - std::f32::consts::FRAC_PI_2).abs() < 0.001);

        let metadata180 = TileVisualMetadata {
            rotation_y: Some(180.0),
            ..Default::default()
        };
        assert!((metadata180.rotation_y_radians() - std::f32::consts::PI).abs() < 0.001);
    }

    #[test]
    fn test_rotation_serialization() {
        let metadata = TileVisualMetadata {
            rotation_y: Some(45.0),
            ..Default::default()
        };

        let serialized = ron::to_string(&metadata).unwrap();
        let deserialized: TileVisualMetadata = ron::from_str(&serialized).unwrap();
        assert_eq!(deserialized.rotation_y, Some(45.0));
    }

    #[test]
    fn test_tile_with_rotation() {
        let mut tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal);
        tile.visual.rotation_y = Some(90.0);

        assert_eq!(tile.visual.rotation_y, Some(90.0));
        assert_eq!(tile.visual.effective_rotation_y(), 90.0);
    }

    // ===== Sprite Reference Tests =====

    #[test]
    fn test_sprite_reference_serialization() {
        let sprite = SpriteReference {
            sheet_path: "sprites/walls.png".to_string(),
            sprite_index: 3,
            animation: None,
            material_properties: None,
        };

        let ron_str = ron::to_string(&sprite).unwrap();
        let deserialized: SpriteReference = ron::from_str(&ron_str).unwrap();

        assert_eq!(sprite, deserialized);
    }

    #[test]
    fn test_sprite_animation_defaults() {
        let ron_str = r#"SpriteAnimation(frames: [0, 1, 2])"#;
        let anim: SpriteAnimation = ron::from_str(ron_str).unwrap();

        assert_eq!(anim.fps, 8.0);
        assert!(anim.looping);
    }

    #[test]
    fn test_tile_visual_uses_sprite() {
        let mut metadata = TileVisualMetadata::default();
        assert!(!metadata.uses_sprite());

        metadata.sprite = Some(SpriteReference {
            sheet_path: "sprites/walls.png".to_string(),
            sprite_index: 0,
            animation: None,
            material_properties: None,
        });
        assert!(metadata.uses_sprite());
    }

    #[test]
    fn test_tile_visual_no_sprite() {
        let metadata = TileVisualMetadata::default();
        assert!(!metadata.uses_sprite());
        assert_eq!(metadata.sprite_sheet_path(), None);
        assert_eq!(metadata.sprite_index(), None);
        assert!(!metadata.has_animation());
    }

    #[test]
    fn test_sprite_sheet_path_accessor() {
        let metadata = TileVisualMetadata {
            sprite: Some(SpriteReference {
                sheet_path: "sprites/walls.png".to_string(),
                sprite_index: 0,
                animation: None,
                material_properties: None,
            }),
            ..Default::default()
        };

        assert_eq!(metadata.sprite_sheet_path(), Some("sprites/walls.png"));
    }

    #[test]
    fn test_tile_with_sprite_builder() {
        let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal)
            .with_sprite("sprites/test.png", 10);

        assert!(tile.visual.uses_sprite());
        assert_eq!(tile.visual.sprite_index(), Some(10));
        assert!(!tile.visual.has_animation());
    }

    #[test]
    fn test_tile_with_animated_sprite_builder() {
        let tile = Tile::new(0, 0, TerrainType::Ground, WallType::Normal).with_animated_sprite(
            "sprites/water.png",
            vec![0, 1, 2, 3],
            4.0,
            true,
        );

        assert!(tile.visual.uses_sprite());
        assert!(tile.visual.has_animation());

        let anim = tile
            .visual
            .sprite
            .as_ref()
            .unwrap()
            .animation
            .as_ref()
            .unwrap();
        assert_eq!(anim.frames, vec![0, 1, 2, 3]);
        assert_eq!(anim.fps, 4.0);
        assert!(anim.looping);
    }

    #[test]
    fn test_backward_compat_no_sprite_field() {
        // Old RON format without sprite field
        let ron_str = r#"
            TileVisualMetadata(
                height: Some(2.5),
                width_x: None,
                width_z: None,
                color_tint: None,
                scale: None,
                y_offset: None,
                rotation_y: None,
            )
        "#;

        let metadata: TileVisualMetadata = ron::from_str(ron_str).unwrap();

        assert_eq!(metadata.height, Some(2.5));
        assert!(!metadata.uses_sprite());
        assert_eq!(metadata.sprite, None);
    }

    // ===== Phase 6: Advanced Features Tests =====

    #[test]
    fn test_sprite_layer_ordering() {
        assert!(SpriteLayer::Background < SpriteLayer::Midground);
        assert!(SpriteLayer::Midground < SpriteLayer::Foreground);
        assert_eq!(SpriteLayer::Background as u8, 0);
        assert_eq!(SpriteLayer::Midground as u8, 1);
        assert_eq!(SpriteLayer::Foreground as u8, 2);
    }

    #[test]
    fn test_layered_sprite_creation() {
        let sprite = LayeredSprite {
            sprite: SpriteReference {
                sheet_path: "terrain.png".to_string(),
                sprite_index: 0,
                animation: None,
                material_properties: None,
            },
            layer: SpriteLayer::Background,
            offset_y: 0.0,
        };

        assert_eq!(sprite.layer, SpriteLayer::Background);
        assert_eq!(sprite.offset_y, 0.0);
        assert_eq!(sprite.sprite.sprite_index, 0);
    }

    #[test]
    fn test_layered_sprite_with_offset() {
        let sprite = LayeredSprite {
            sprite: SpriteReference {
                sheet_path: "decoration.png".to_string(),
                sprite_index: 5,
                animation: None,
                material_properties: None,
            },
            layer: SpriteLayer::Foreground,
            offset_y: 0.5,
        };

        assert_eq!(sprite.layer, SpriteLayer::Foreground);
        assert_eq!(sprite.offset_y, 0.5);
    }

    #[test]
    fn test_sprite_material_properties_emissive() {
        let props = SpriteMaterialProperties {
            emissive: Some([1.0, 0.0, 0.0]),
            alpha: None,
            metallic: None,
            roughness: None,
        };

        assert_eq!(props.emissive, Some([1.0, 0.0, 0.0]));
        assert_eq!(props.alpha, None);
    }

    #[test]
    fn test_sprite_material_properties_alpha() {
        let props = SpriteMaterialProperties {
            emissive: None,
            alpha: Some(0.5),
            metallic: None,
            roughness: None,
        };

        assert_eq!(props.alpha, Some(0.5));
    }

    #[test]
    fn test_sprite_material_properties_metallic_roughness() {
        let props = SpriteMaterialProperties {
            emissive: None,
            alpha: None,
            metallic: Some(0.8),
            roughness: Some(0.3),
        };

        assert_eq!(props.metallic, Some(0.8));
        assert_eq!(props.roughness, Some(0.3));
    }

    #[test]
    fn test_sprite_material_properties_all_fields() {
        let props = SpriteMaterialProperties {
            emissive: Some([0.5, 0.5, 0.5]),
            alpha: Some(0.8),
            metallic: Some(0.6),
            roughness: Some(0.4),
        };

        assert_eq!(props.emissive, Some([0.5, 0.5, 0.5]));
        assert_eq!(props.alpha, Some(0.8));
        assert_eq!(props.metallic, Some(0.6));
        assert_eq!(props.roughness, Some(0.4));
    }

    #[test]
    fn test_sprite_material_properties_default() {
        let props = SpriteMaterialProperties::default();

        assert_eq!(props.emissive, None);
        assert_eq!(props.alpha, None);
        assert_eq!(props.metallic, None);
        assert_eq!(props.roughness, None);
    }

    #[test]
    fn test_sprite_reference_with_material_properties() {
        let sprite = SpriteReference {
            sheet_path: "portal.png".to_string(),
            sprite_index: 0,
            animation: None,
            material_properties: Some(SpriteMaterialProperties {
                emissive: Some([0.0, 1.0, 0.0]),
                alpha: None,
                metallic: None,
                roughness: None,
            }),
        };

        assert!(sprite.material_properties.is_some());
        assert_eq!(
            sprite.material_properties.unwrap().emissive,
            Some([0.0, 1.0, 0.0])
        );
    }

    #[test]
    fn test_sprite_selection_rule_fixed() {
        let rule = SpriteSelectionRule::Fixed {
            sheet_path: "walls.png".to_string(),
            sprite_index: 3,
        };

        match rule {
            SpriteSelectionRule::Fixed {
                sheet_path,
                sprite_index,
            } => {
                assert_eq!(sheet_path, "walls.png");
                assert_eq!(sprite_index, 3);
            }
            _ => panic!("Expected Fixed rule"),
        }
    }

    #[test]
    fn test_sprite_selection_rule_random() {
        let rule = SpriteSelectionRule::Random {
            sheet_path: "grass.png".to_string(),
            sprite_indices: vec![0, 1, 2, 3],
            seed: Some(42),
        };

        match rule {
            SpriteSelectionRule::Random {
                sheet_path,
                sprite_indices,
                seed,
            } => {
                assert_eq!(sheet_path, "grass.png");
                assert_eq!(sprite_indices.len(), 4);
                assert_eq!(seed, Some(42));
            }
            _ => panic!("Expected Random rule"),
        }
    }

    #[test]
    fn test_sprite_selection_rule_autotile() {
        let mut rules = HashMap::new();
        rules.insert(0b0011, 5);

        let rule = SpriteSelectionRule::Autotile {
            sheet_path: "terrain.png".to_string(),
            rules,
        };

        match rule {
            SpriteSelectionRule::Autotile {
                sheet_path,
                rules: rule_map,
            } => {
                assert_eq!(sheet_path, "terrain.png");
                assert_eq!(rule_map.get(&0b0011), Some(&5));
            }
            _ => panic!("Expected Autotile rule"),
        }
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_tile_visual_metadata_with_sprite_layers() {
        let mut metadata = TileVisualMetadata::default();
        metadata.sprite_layers = vec![
            LayeredSprite {
                sprite: SpriteReference {
                    sheet_path: "bg.png".to_string(),
                    sprite_index: 0,
                    animation: None,
                    material_properties: None,
                },
                layer: SpriteLayer::Background,
                offset_y: 0.0,
            },
            LayeredSprite {
                sprite: SpriteReference {
                    sheet_path: "fg.png".to_string(),
                    sprite_index: 1,
                    animation: None,
                    material_properties: None,
                },
                layer: SpriteLayer::Foreground,
                offset_y: 0.2,
            },
        ];

        assert_eq!(metadata.sprite_layers.len(), 2);
        assert_eq!(metadata.sprite_layers[0].layer, SpriteLayer::Background);
        assert_eq!(metadata.sprite_layers[1].layer, SpriteLayer::Foreground);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_tile_visual_metadata_with_sprite_rule() {
        let mut metadata = TileVisualMetadata::default();
        metadata.sprite_rule = Some(SpriteSelectionRule::Random {
            sheet_path: "grass.png".to_string(),
            sprite_indices: vec![0, 1, 2, 3],
            seed: None,
        });

        assert!(metadata.sprite_rule.is_some());
    }

    #[test]
    fn test_backward_compat_no_sprite_layers() {
        // Old format without sprite_layers should deserialize
        let ron_str = r#"
            TileVisualMetadata(
                height: Some(1.0),
                width_x: None,
                width_z: None,
                color_tint: None,
                scale: None,
                y_offset: None,
                rotation_y: None,
                sprite: None,
            )
        "#;

        let metadata: TileVisualMetadata = ron::from_str(ron_str).unwrap();
        assert_eq!(metadata.sprite_layers.len(), 0);
        assert_eq!(metadata.sprite_rule, None);
    }

    #[test]
    fn test_sprite_layers_serialization() {
        let layers = vec![LayeredSprite {
            sprite: SpriteReference {
                sheet_path: "test.png".to_string(),
                sprite_index: 0,
                animation: None,
                material_properties: None,
            },
            layer: SpriteLayer::Background,
            offset_y: 0.0,
        }];

        // Should be serializable to RON
        let ron_str = ron::to_string(&layers).unwrap();
        assert!(ron_str.contains("Background"));

        // Should be deserializable back
        let deserialized: Vec<LayeredSprite> = ron::from_str(&ron_str).unwrap();
        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized[0].layer, SpriteLayer::Background);
    }

    #[test]
    fn test_sprite_material_properties_serialization() {
        let props = SpriteMaterialProperties {
            emissive: Some([1.0, 0.5, 0.0]),
            alpha: Some(0.8),
            metallic: Some(0.5),
            roughness: Some(0.6),
        };

        let ron_str = ron::to_string(&props).unwrap();
        let deserialized: SpriteMaterialProperties = ron::from_str(&ron_str).unwrap();

        assert_eq!(deserialized.emissive, props.emissive);
        assert_eq!(deserialized.alpha, props.alpha);
        assert_eq!(deserialized.metallic, props.metallic);
        assert_eq!(deserialized.roughness, props.roughness);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_grass_density_serialization() {
        let mut meta = TileVisualMetadata::default();
        meta.grass_density = Some(GrassDensity::High);

        let ron = ron::to_string(&meta).unwrap();
        assert!(ron.contains("grass_density"));
        assert!(ron.contains("High"));

        let deserialized: TileVisualMetadata = ron::from_str(&ron).unwrap();
        assert_eq!(deserialized.grass_density, Some(GrassDensity::High));
    }

    #[test]
    fn test_grass_density_default_not_serialized() {
        let meta = TileVisualMetadata::default();
        let ron = ron::to_string(&meta).unwrap();
        assert!(!ron.contains("grass_density"));
    }

    #[test]
    fn test_tree_type_accessor_defaults_to_oak() {
        let meta = TileVisualMetadata::default();
        assert_eq!(meta.tree_type(), TreeType::Oak);
    }

    #[test]
    fn test_has_terrain_overrides_returns_false_for_default() {
        let meta = TileVisualMetadata::default();
        assert!(!meta.has_terrain_overrides());
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_has_terrain_overrides_returns_true_when_set() {
        let mut meta = TileVisualMetadata::default();
        meta.grass_density = Some(GrassDensity::Low);
        assert!(meta.has_terrain_overrides());
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_foliage_density_clamps_in_valid_range() {
        let mut meta = TileVisualMetadata::default();
        meta.foliage_density = Some(1.5);
        assert_eq!(meta.foliage_density(), 1.5);
    }

    #[test]
    fn test_water_flow_direction_default_is_still() {
        let meta = TileVisualMetadata::default();
        assert_eq!(meta.water_flow_direction(), WaterFlowDirection::Still);
    }

    #[test]
    fn test_grass_density_default_is_medium() {
        assert_eq!(GrassDensity::default(), GrassDensity::Medium);
    }

    #[test]
    fn test_tree_type_default_is_oak() {
        assert_eq!(TreeType::default(), TreeType::Oak);
    }

    #[test]
    fn test_rock_variant_default_is_smooth() {
        assert_eq!(RockVariant::default(), RockVariant::Smooth);
    }

    #[test]
    fn test_water_flow_default_is_still() {
        assert_eq!(WaterFlowDirection::default(), WaterFlowDirection::Still);
    }

    #[test]
    fn test_grass_density_serializes_to_ron() {
        let density = GrassDensity::High;
        let ron = ron::to_string(&density).unwrap();
        assert_eq!(ron.trim(), "High");

        let deserialized: GrassDensity = ron::from_str(&ron).unwrap();
        assert_eq!(deserialized, GrassDensity::High);
    }

    #[test]
    fn test_tree_type_deserializes_from_ron() {
        let ron_str = "Pine";
        let tree: TreeType = ron::from_str(ron_str).unwrap();
        assert_eq!(tree, TreeType::Pine);
    }

    #[test]
    fn test_rock_variant_round_trip_serialization() {
        let original = RockVariant::Crystal;
        let ron_str = ron::to_string(&original).unwrap();
        let deserialized: RockVariant = ron::from_str(&ron_str).unwrap();
        assert_eq!(deserialized, original);
    }

    #[test]
    fn test_water_flow_all_variants_serialize() {
        let variants = vec![
            WaterFlowDirection::Still,
            WaterFlowDirection::North,
            WaterFlowDirection::South,
            WaterFlowDirection::East,
            WaterFlowDirection::West,
        ];

        for variant in variants {
            let ron = ron::to_string(&variant).unwrap();
            let deserialized: WaterFlowDirection = ron::from_str(&ron).unwrap();
            assert_eq!(deserialized, variant);
        }
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_metadata_with_grass_density_serializes() {
        let mut meta = TileVisualMetadata::default();
        meta.height = Some(2.0);
        meta.grass_density = Some(GrassDensity::Medium);

        let ron = ron::to_string(&meta).unwrap();
        let deserialized: TileVisualMetadata = ron::from_str(&ron).unwrap();

        assert_eq!(deserialized.height, meta.height);
        assert_eq!(deserialized.grass_density, meta.grass_density);
    }

    #[test]
    fn test_metadata_without_terrain_fields_is_minimal() {
        let meta = TileVisualMetadata::default();
        let ron = ron::to_string(&meta).unwrap();

        // Default metadata should have minimal serialization
        assert!(!ron.contains("grass_density"));
        assert!(!ron.contains("tree_type"));
        assert!(!ron.contains("rock_variant"));
        assert!(!ron.contains("water_flow_direction"));
        assert!(!ron.contains("foliage_density"));
        assert!(!ron.contains("snow_coverage"));
    }

    #[test]
    fn test_metadata_accessors_return_defaults() {
        let meta = TileVisualMetadata::default();

        assert_eq!(meta.grass_density(), GrassDensity::Medium);
        assert_eq!(meta.tree_type(), TreeType::Oak);
        assert_eq!(meta.rock_variant(), RockVariant::Smooth);
        assert_eq!(meta.water_flow_direction(), WaterFlowDirection::Still);
        assert_eq!(meta.foliage_density(), 1.0);
        assert_eq!(meta.snow_coverage(), 0.0);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_has_terrain_overrides_detects_grass_density() {
        let mut meta = TileVisualMetadata::default();
        assert!(!meta.has_terrain_overrides());

        meta.grass_density = Some(GrassDensity::Medium);
        assert!(meta.has_terrain_overrides());
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_has_terrain_overrides_detects_tree_type() {
        let mut meta = TileVisualMetadata::default();
        meta.tree_type = Some(TreeType::Pine);
        assert!(meta.has_terrain_overrides());
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_has_terrain_overrides_detects_all_fields() {
        let mut meta = TileVisualMetadata::default();

        meta.grass_density = Some(GrassDensity::High);
        assert!(meta.has_terrain_overrides());

        meta.grass_density = None;
        meta.rock_variant = Some(RockVariant::Jagged);
        assert!(meta.has_terrain_overrides());

        meta.rock_variant = None;
        meta.foliage_density = Some(1.5);
        assert!(meta.has_terrain_overrides());

        meta.foliage_density = None;
        meta.snow_coverage = Some(0.5);
        assert!(meta.has_terrain_overrides());
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_foliage_density_bounds() {
        let mut meta = TileVisualMetadata::default();

        // Test minimum
        meta.foliage_density = Some(0.0);
        assert_eq!(meta.foliage_density(), 0.0);

        // Test maximum
        meta.foliage_density = Some(2.0);
        assert_eq!(meta.foliage_density(), 2.0);

        // Test intermediate
        meta.foliage_density = Some(1.5);
        assert_eq!(meta.foliage_density(), 1.5);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_snow_coverage_bounds() {
        let mut meta = TileVisualMetadata::default();

        // Test minimum
        meta.snow_coverage = Some(0.0);
        assert_eq!(meta.snow_coverage(), 0.0);

        // Test maximum
        meta.snow_coverage = Some(1.0);
        assert_eq!(meta.snow_coverage(), 1.0);

        // Test intermediate
        meta.snow_coverage = Some(0.5);
        assert_eq!(meta.snow_coverage(), 0.5);
    }
}
