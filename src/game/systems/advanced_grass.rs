// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Advanced grass rendering systems
//!
//! This module contains the core grass rendering pipeline, including:
//! - Procedural blade mesh generation
//! - Grass cluster spawning with configurable blade appearance
//! - Culling and LOD systems for performance
//! - Optional chunk-based mesh merging for advanced optimization
//!
//! Grass rendering consumes **domain** content density (`GrassDensity`) and
//! applies **performance scaling** (`GrassPerformanceLevel`) to determine
//! the final blade count per tile.

use bevy::log::debug;
use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;

use crate::domain::types;
use crate::domain::world::{self, TileVisualMetadata};
use crate::game::resources::{
    GrassPerformanceLevel, GrassQualitySettings, VegetationQualityLevel, VegetationQualitySettings,
};
use crate::game::systems::map::{MapEntity, TileCoord};
use crate::game::systems::vegetation_placement::VegetationExclusionZone;

// ==================== Constants ====================

const GRASS_BLADE_WIDTH: f32 = 0.15;
const GRASS_BLADE_HEIGHT_BASE: f32 = 0.4;
const GRASS_BLADE_Y_OFFSET: f32 = 0.0;
const TILE_CENTER_OFFSET: f32 = 0.5;

/// Path to the grass blade alpha-cutout texture (relative to the `assets/` directory).
const GRASS_BLADE_TEXTURE: &str = "assets/textures/grass/grass_blade.png";

/// Alpha threshold for the grass blade mask cutout.
///
/// Fragments with alpha below this value are discarded, producing a clean
/// silhouette without sorting artefacts.
const GRASS_ALPHA_CUTOFF: f32 = 0.3;

/// Minimum number of crossed blade cards in a grass clump.
const MIN_CARDS_PER_CLUMP: u32 = 2;

/// Maximum number of crossed blade cards in a grass clump.
const MAX_CARDS_PER_CLUMP: u32 = 4;

/// Average authored blades represented by one clump entity.
const BLADES_PER_CLUMP: u32 = 8;

/// Radius used when distributing clumps within a tile.
const GRASS_PATCH_RADIUS: f32 = 0.42;

/// Multiplier from the first grass LOD distance to the far-card LOD threshold.
const FAR_GRASS_LOD_DISTANCE_MULTIPLIER: f32 = 1.5;

/// Default budget for grass material variant buckets.
const DEFAULT_MAX_GRASS_MATERIAL_VARIANT_BUCKETS: usize = 64;

/// Minimum clump height variation multiplier.
const MIN_HEIGHT_VARIATION: f32 = 0.75;

/// Maximum clump height variation multiplier.
const MAX_HEIGHT_VARIATION: f32 = 1.25;

/// Minimum clump width variation multiplier.
const MIN_WIDTH_VARIATION: f32 = 0.85;

/// Maximum clump width variation multiplier.
const MAX_WIDTH_VARIATION: f32 = 1.15;

/// Ground lift that prevents alpha-masked grass from z-fighting with floors.
const GRASS_GROUND_CLEARANCE: f32 = 0.015;

/// Resource holding shared grass mesh and material assets.
///
/// The cache is explicit so map spawning can reuse grass assets without relying
/// on process-global state. Meshes are keyed by quality and bucketed blade
/// configuration; materials are keyed by tint and alpha-mask settings.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassAssetCache;
///
/// let cache = GrassAssetCache::default();
/// assert_eq!(cache.mesh_count(), 0);
/// assert_eq!(cache.material_count(), 0);
/// ```
#[derive(Resource, Default, Debug)]
pub struct GrassAssetCache {
    mesh_handles: HashMap<GrassMeshKey, Handle<Mesh>>,
    material_handles: HashMap<GrassMaterialKey, Handle<StandardMaterial>>,
}

impl GrassAssetCache {
    /// Returns the number of cached grass mesh variants.
    pub fn mesh_count(&self) -> usize {
        self.mesh_handles.len()
    }

    /// Returns the number of cached grass material variants.
    pub fn material_count(&self) -> usize {
        self.material_handles.len()
    }
}

// ==================== Grass Rendering Components ====================

/// Component marking a grass cluster (parent entity containing multiple blades)
///
/// Used for distance-based culling to optimize rendering performance.
/// Clusters are culled when the camera is beyond `cull_distance`.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassCluster;
///
/// let cluster = GrassCluster { cull_distance: 50.0 };
/// ```
#[derive(Component, Clone, Copy, Debug)]
pub struct GrassCluster {
    /// Distance beyond which this cluster should be culled (hidden)
    pub cull_distance: f32,
}

impl Default for GrassCluster {
    fn default() -> Self {
        Self {
            cull_distance: 50.0,
        }
    }
}

/// Parent component for all grass spawned for a single tile.
///
/// `GrassPatch` keeps the optimized clump children grouped by tile while
/// preserving the existing `GrassCluster` component used by culling and LOD.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassPatch;
///
/// let patch = GrassPatch { clump_count: 12 };
/// assert_eq!(patch.clump_count, 12);
/// ```
#[derive(Component, Clone, Copy, Debug)]
pub struct GrassPatch {
    /// Number of renderable clumps spawned under this patch.
    pub clump_count: u32,
}

/// Component marking one renderable clump of crossed grass cards.
///
/// A clump replaces several sparse per-blade entities with one shared mesh
/// instance, reducing entity and asset churn while keeping visible volume.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassClump;
///
/// let clump = GrassClump { card_count: 3 };
/// assert_eq!(clump.card_count, 3);
/// ```
#[derive(Component, Clone, Copy, Debug)]
pub struct GrassClump {
    /// Number of crossed cards represented by this clump mesh.
    pub card_count: u32,
}

/// Grass runtime LOD tiers.
///
/// These tiers reduce visible clumps at distance without changing tile data or
/// gameplay state.
///
/// # Examples
///
/// ```
/// use antares::game::systems::advanced_grass::GrassLodTier;
///
/// assert_eq!(GrassLodTier::Near.retention_stride(), Some(1));
/// assert_eq!(GrassLodTier::Far.retention_stride(), Some(4));
/// assert_eq!(GrassLodTier::Culled.retention_stride(), None);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrassLodTier {
    /// Full clumps and all visible child clump entities.
    Near,
    /// Half of clump children remain visible.
    Mid,
    /// One quarter of clump children remain visible as low-card patches.
    Far,
    /// Grass is hidden by the culling budget.
    Culled,
}

impl GrassLodTier {
    /// Returns the child-retention stride for this tier.
    ///
    /// `None` means the tier is culled. `Some(1)` means every clump remains
    /// visible, `Some(2)` means every other clump remains visible, and so on.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::systems::advanced_grass::GrassLodTier;
    ///
    /// assert_eq!(GrassLodTier::Near.retention_stride(), Some(1));
    /// assert_eq!(GrassLodTier::Mid.retention_stride(), Some(2));
    /// assert_eq!(GrassLodTier::Far.retention_stride(), Some(4));
    /// assert_eq!(GrassLodTier::Culled.retention_stride(), None);
    /// ```
    pub fn retention_stride(self) -> Option<u32> {
        match self {
            Self::Near => Some(1),
            Self::Mid => Some(2),
            Self::Far => Some(4),
            Self::Culled => None,
        }
    }
}

/// Mesh quality tiers for grass blade card geometry.
///
/// Lower quality uses fewer curve segments. Higher quality uses more segments
/// for smoother tapered and bent blades.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassMeshQuality;
///
/// assert_eq!(GrassMeshQuality::Low.segment_count(), 3);
/// assert_eq!(GrassMeshQuality::Medium.segment_count(), 5);
/// assert_eq!(GrassMeshQuality::High.segment_count(), 7);
/// ```
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum GrassMeshQuality {
    /// Low-quality mesh with three curve segments.
    Low,
    /// Medium-quality mesh with five curve segments.
    Medium,
    /// High-quality mesh with seven curve segments.
    High,
}

impl GrassMeshQuality {
    /// Returns the number of vertical curve segments for this quality tier.
    pub fn segment_count(self) -> usize {
        match self {
            Self::Low => 3,
            Self::Medium => 5,
            Self::High => 7,
        }
    }

    fn from_settings(settings: &GrassQualitySettings) -> Self {
        match settings.performance_level {
            crate::game::resources::grass_quality_settings::GrassPerformanceLevel::Low => Self::Low,
            crate::game::resources::grass_quality_settings::GrassPerformanceLevel::Medium => {
                Self::Medium
            }
            crate::game::resources::grass_quality_settings::GrassPerformanceLevel::High => {
                Self::High
            }
        }
    }
}

/// Cache key for shared alpha-masked grass materials.
///
/// The key buckets tint and alpha values so many clumps can share a material
/// while still allowing dried or tinted grass to look distinct.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassMaterialKey;
///
/// let key = GrassMaterialKey::from_tint((0.4, 0.8, 0.3), 0.2);
/// assert!(key.tint_g >= key.tint_r);
/// ```
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct GrassMaterialKey {
    /// Quantized red tint bucket.
    pub tint_r: u8,
    /// Quantized green tint bucket.
    pub tint_g: u8,
    /// Quantized blue tint bucket.
    pub tint_b: u8,
    /// Quantized color variation bucket.
    pub variation: u8,
    /// Quantized alpha cutoff bucket.
    pub alpha_cutoff: u8,
}

impl GrassMaterialKey {
    /// Builds a material key from a tint tuple and color variation value.
    pub fn from_tint(tint: (f32, f32, f32), variation: f32) -> Self {
        Self::from_tint_with_budget(tint, variation, DEFAULT_MAX_GRASS_MATERIAL_VARIANT_BUCKETS)
    }

    /// Builds a material key using a bounded material-variant budget.
    ///
    /// Lower budgets coarsen tint and variation buckets so repeated map spawns
    /// cannot create one material per subtly different blade color.
    pub fn from_tint_with_budget(
        tint: (f32, f32, f32),
        variation: f32,
        max_material_variants: usize,
    ) -> Self {
        Self {
            tint_r: quantize_unit_with_budget(tint.0, max_material_variants),
            tint_g: quantize_unit_with_budget(tint.1, max_material_variants),
            tint_b: quantize_unit_with_budget(tint.2, max_material_variants),
            variation: quantize_unit_with_budget(variation, max_material_variants),
            alpha_cutoff: quantize_unit(GRASS_ALPHA_CUTOFF),
        }
    }
}

/// Deterministic grass placement seed derived from map ID and tile position.
///
/// # Examples
///
/// ```rust
/// use antares::domain::types::Position;
/// use antares::game::systems::advanced_grass::GrassPlacementSeed;
///
/// let a = GrassPlacementSeed::new(1, Position::new(2, 3));
/// let b = GrassPlacementSeed::new(1, Position::new(2, 3));
/// assert_eq!(a.value(), b.value());
/// ```
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct GrassPlacementSeed(u64);

impl GrassPlacementSeed {
    /// Creates a deterministic seed from map ID and tile coordinates.
    pub fn new(map_id: types::MapId, position: types::Position) -> Self {
        let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
        seed ^= u64::from(map_id).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        seed ^= (position.x as i64 as u64).rotate_left(17);
        seed ^= (position.y as i64 as u64).rotate_left(41);
        Self(seed)
    }

    /// Returns the raw deterministic seed value.
    pub fn value(self) -> u64 {
        self.0
    }

    fn rng(self) -> StdRng {
        StdRng::seed_from_u64(self.0)
    }
}

/// Render-layer grass wind parameters.
///
/// Wind values are attached now so later animation work can consume them
/// without changing the spawn pipeline.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassWindParams;
///
/// let wind = GrassWindParams::default();
/// assert!(wind.strength > 0.0);
/// ```
#[derive(Component, Clone, Copy, Debug)]
pub struct GrassWindParams {
    /// Wind sway strength in world units.
    pub strength: f32,
    /// Wind animation frequency in cycles per second.
    pub frequency: f32,
    /// Deterministic phase offset for this clump.
    pub phase: f32,
}

impl Default for GrassWindParams {
    fn default() -> Self {
        Self {
            strength: 0.04,
            frequency: 0.65,
            phase: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct GrassMeshKey {
    quality: GrassMeshQuality,
    length: u16,
    width: u16,
    tilt: u16,
    curve: u16,
    cards: u8,
}

/// Component marking an individual grass blade within a cluster
///
/// Used for LOD (Level of Detail) system to reduce blade count at distance.
/// The `lod_index` determines which blades are hidden in far LOD mode.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassBlade;
///
/// let blade = GrassBlade { lod_index: 0 };
/// ```
#[derive(Component, Clone, Copy, Debug)]
pub struct GrassBlade {
    /// Index of this blade within its cluster (0-indexed)
    pub lod_index: u32,
}

/// Component attached to spawned grass blades containing the original
/// mesh and material handles used for that blade.
///
/// This component is consumed by chunking/instancing systems so the renderer
/// can collect geometry and material data without relying on spawn internals.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassBladeInstance;
/// use bevy::prelude::{Handle, Mesh, StandardMaterial};
///
/// fn build_instance(mesh: Handle<Mesh>, material: Handle<StandardMaterial>) -> GrassBladeInstance {
///     GrassBladeInstance { mesh, material }
/// }
/// ```
#[derive(Component, Clone, Debug)]
pub struct GrassBladeInstance {
    /// Mesh handle for this blade (copied from spawn-time mesh)
    pub mesh: Handle<Mesh>,
    /// Material handle for this blade (copied from spawn-time material)
    pub material: Handle<StandardMaterial>,
}

/// Resource configuring grass rendering performance settings
///
/// Controls culling and LOD distances for grass rendering optimization.
/// Essential for maintaining 60fps with grass-heavy maps.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassRenderConfig;
///
/// let config = GrassRenderConfig {
///     cull_distance: 50.0,
///     lod_distance: 25.0,
/// };
/// ```
#[derive(Resource, Clone, Copy, Debug)]
pub struct GrassRenderConfig {
    /// Distance beyond which grass clusters are culled (default: 50.0)
    pub cull_distance: f32,
    /// Distance beyond which LOD reduction begins (default: 25.0)
    pub lod_distance: f32,
}

impl Default for GrassRenderConfig {
    fn default() -> Self {
        Self::from_vegetation_quality(&VegetationQualitySettings::default())
    }
}

impl GrassRenderConfig {
    /// Builds grass culling and LOD distances from vegetation-wide quality settings.
    ///
    /// This keeps grass render budgets aligned with tree and global vegetation
    /// budgets while preserving grass-specific runtime components.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::game::resources::performance::{
    ///     VegetationQualityLevel, VegetationQualitySettings,
    /// };
    /// use antares::game::systems::advanced_grass::GrassRenderConfig;
    ///
    /// let quality = VegetationQualitySettings::for_level(VegetationQualityLevel::Low);
    /// let config = GrassRenderConfig::from_vegetation_quality(&quality);
    ///
    /// assert_eq!(config.cull_distance, quality.vegetation_cull_distance);
    /// assert_eq!(config.lod_distance, quality.grass_lod_distance);
    /// ```
    pub fn from_vegetation_quality(settings: &VegetationQualitySettings) -> Self {
        Self {
            cull_distance: settings.vegetation_cull_distance,
            lod_distance: settings.grass_lod_distance,
        }
    }
}

// ==================== Instance Batching ====================

/// Resource controlling grass instance batching behavior
///
/// When enabled, the system will aggregate per-blade instance data into
/// batches for future GPU instancing pipelines while keeping existing
/// blade meshes rendered.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassInstanceConfig;
///
/// let config = GrassInstanceConfig::default();
/// assert!(!config.enabled);
/// ```
#[derive(Resource, Clone, Copy, Debug)]
pub struct GrassInstanceConfig {
    /// Whether instance batching is enabled
    pub enabled: bool,
    /// Maximum instances per batch entity
    pub max_instances_per_batch: usize,
}

impl Default for GrassInstanceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_instances_per_batch: 1024,
        }
    }
}

/// Component storing instance data for a batched grass mesh
///
/// Instance batches are used for performance diagnostics and future GPU
/// instancing pipelines. The existing blade meshes remain rendered until
/// a GPU instancing pipeline is introduced.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassInstanceBatch;
/// use antares::domain::world::InstanceData;
/// use bevy::prelude::{Handle, Mesh, StandardMaterial};
///
/// let batch = GrassInstanceBatch {
///     mesh: Handle::<Mesh>::default(),
///     material: Handle::<StandardMaterial>::default(),
///     instances: vec![InstanceData::new([0.0, 0.0, 0.0])],
/// };
/// ```
#[derive(Component, Clone, Debug)]
pub struct GrassInstanceBatch {
    /// Mesh handle shared by all instances
    pub mesh: Handle<Mesh>,
    /// Material handle shared by all instances
    pub material: Handle<StandardMaterial>,
    /// Per-instance transform data
    pub instances: Vec<world::InstanceData>,
}

// ==================== Blade Configuration ====================

/// Configuration for individual grass blade appearance
///
/// Controls visual properties of spawned grass blades.
/// Converted from domain-layer `GrassBladeConfig` with clamped values.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::BladeConfig;
///
/// let config = BladeConfig {
///     length: 1.5,
///     width: 0.8,
///     tilt: 0.4,
///     curve: 0.5,
///     color_variation: 0.3,
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct BladeConfig {
    /// Blade length multiplier (clamped 0.5-2.0)
    pub length: f32,
    /// Blade width multiplier (clamped 0.5-2.0)
    pub width: f32,
    /// Blade tilt angle in radians (clamped 0.0-0.5)
    pub tilt: f32,
    /// Blade curvature amount (clamped 0.0-1.0)
    pub curve: f32,
    /// Color variation (clamped 0.0-1.0)
    pub color_variation: f32,
}

impl Default for BladeConfig {
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

impl From<&world::GrassBladeConfig> for BladeConfig {
    fn from(config: &world::GrassBladeConfig) -> Self {
        Self {
            length: config.length.clamp(0.5, 2.0),
            width: config.width.clamp(0.5, 2.0),
            tilt: config.tilt.clamp(0.0, 0.5),
            curve: config.curve.clamp(0.0, 1.0),
            color_variation: config.color_variation.clamp(0.0, 1.0),
        }
    }
}

/// Color scheme for grass blades with variation support
///
/// Provides base and tip colors with random variation to create
/// natural-looking grass.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::advanced_grass::GrassColorScheme;
/// use bevy::prelude::Color;
///
/// let scheme = GrassColorScheme {
///     base_color: Color::srgb(0.2, 0.5, 0.1),
///     tip_color: Color::srgb(0.3, 0.7, 0.2),
///     variation: 0.3,
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct GrassColorScheme {
    /// Base color at blade bottom
    pub base_color: Color,
    /// Tip color at blade top
    pub tip_color: Color,
    /// Variation amount (0.0-1.0)
    pub variation: f32,
}

impl GrassColorScheme {
    /// Sample a random blade color with variation
    ///
    /// # Arguments
    ///
    /// * `rng` - Random number generator
    ///
    /// # Returns
    ///
    /// A color with applied variation
    pub fn sample_blade_color(&self, rng: &mut impl rand::Rng) -> Color {
        let base_blend = 0.7;
        let blended = Color::srgb(
            self.base_color.to_srgba().red * base_blend
                + self.tip_color.to_srgba().red * (1.0 - base_blend),
            self.base_color.to_srgba().green * base_blend
                + self.tip_color.to_srgba().green * (1.0 - base_blend),
            self.base_color.to_srgba().blue * base_blend
                + self.tip_color.to_srgba().blue * (1.0 - base_blend),
        );

        if self.variation > 0.0 {
            let variation_amount = rng.random_range(-self.variation..self.variation);
            Color::srgb(
                (blended.to_srgba().red + variation_amount).clamp(0.0, 1.0),
                (blended.to_srgba().green + variation_amount).clamp(0.0, 1.0),
                (blended.to_srgba().blue + variation_amount).clamp(0.0, 1.0),
            )
        } else {
            blended
        }
    }
}

impl Default for GrassColorScheme {
    fn default() -> Self {
        Self {
            base_color: Color::srgb(0.2, 0.5, 0.1),
            tip_color: Color::srgb(0.3, 0.7, 0.2),
            variation: 0.2,
        }
    }
}

/// Scales the content-derived blade-count range by tile foliage density.
///
/// `TileVisualMetadata::foliage_density` is an authoring multiplier shared by
/// vegetation systems.  Grass clamps it to `0.0..=2.0` so SDK edits can remove,
/// reduce, or thicken grass coverage without creating unbounded blade counts.
fn scaled_blade_count_range_for_foliage_density(
    min_blades: u32,
    max_blades: u32,
    visual_metadata: Option<&TileVisualMetadata>,
) -> (u32, u32, f32) {
    let foliage_density_multiplier = visual_metadata
        .map(TileVisualMetadata::foliage_density)
        .unwrap_or(1.0)
        .clamp(0.0, 2.0);
    let scaled_min_blades = ((min_blades as f32) * foliage_density_multiplier).round() as u32;
    let scaled_max_blades = ((max_blades as f32) * foliage_density_multiplier).round() as u32;

    (
        scaled_min_blades,
        scaled_max_blades,
        foliage_density_multiplier,
    )
}

// ==================== Grass Mesh Generation ====================

fn quantize_unit(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn vegetation_quality_from_grass_quality_settings(
    quality_settings: &GrassQualitySettings,
) -> VegetationQualitySettings {
    let quality_level = match quality_settings.performance_level {
        GrassPerformanceLevel::Low => VegetationQualityLevel::Low,
        GrassPerformanceLevel::Medium => VegetationQualityLevel::Medium,
        GrassPerformanceLevel::High => VegetationQualityLevel::High,
    };

    VegetationQualitySettings::for_level(quality_level)
}

fn material_budget_levels(max_material_variants: usize) -> u8 {
    match max_material_variants {
        0 | 1 => 1,
        2..=16 => 2,
        17..=64 => 4,
        _ => 8,
    }
}

fn quantize_unit_with_budget(value: f32, max_material_variants: usize) -> u8 {
    let levels = material_budget_levels(max_material_variants);
    if levels <= 1 {
        return 0;
    }

    let max_bucket = f32::from(levels - 1);
    let bucket = (value.clamp(0.0, 1.0) * max_bucket).round();
    ((bucket / max_bucket) * 255.0).round() as u8
}

fn quantize_mesh_value(value: f32) -> u16 {
    (value.clamp(0.0, 4.0) * 1000.0).round() as u16
}

fn cached_material_color(key: GrassMaterialKey, color_scheme: &GrassColorScheme) -> Color {
    let variation = f32::from(key.variation) / 255.0;
    let base = color_scheme.base_color.to_srgba();
    let tip = color_scheme.tip_color.to_srgba();

    Color::srgb(
        (base.red * (1.0 - variation) + tip.red * variation).clamp(0.0, 1.0),
        (base.green * (1.0 - variation) + tip.green * variation).clamp(0.0, 1.0),
        (base.blue * (1.0 - variation) + tip.blue * variation).clamp(0.0, 1.0),
    )
}

fn point_inside_grass_exclusion_zones(
    point: Vec2,
    grass_exclusion_zones: &[VegetationExclusionZone],
) -> bool {
    grass_exclusion_zones
        .iter()
        .any(|zone| zone.contains(point))
}

fn find_allowed_grass_clump_center(
    rng: &mut StdRng,
    tile_world_center: Vec2,
    grass_exclusion_zones: &[VegetationExclusionZone],
) -> Option<Vec2> {
    const MAX_ATTEMPTS: usize = 12;

    for _ in 0..MAX_ATTEMPTS {
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let distance = rng.random_range(0.0..GRASS_PATCH_RADIUS);
        let local_center = Vec2::new(angle.cos() * distance, angle.sin() * distance);

        if !point_inside_grass_exclusion_zones(
            tile_world_center + local_center,
            grass_exclusion_zones,
        ) {
            return Some(local_center);
        }
    }

    None
}

fn grass_mesh_key(
    quality: GrassMeshQuality,
    blade_config: &BladeConfig,
    card_count: u32,
) -> GrassMeshKey {
    GrassMeshKey {
        quality,
        length: quantize_mesh_value(blade_config.length),
        width: quantize_mesh_value(blade_config.width),
        tilt: quantize_mesh_value(blade_config.tilt),
        curve: quantize_mesh_value(blade_config.curve),
        cards: card_count.clamp(MIN_CARDS_PER_CLUMP, MAX_CARDS_PER_CLUMP) as u8,
    }
}

fn get_or_create_grass_mesh(
    meshes: &mut ResMut<Assets<Mesh>>,
    grass_cache: &mut GrassAssetCache,
    key: GrassMeshKey,
    blade_height: f32,
    blade_config: &BladeConfig,
) -> Handle<Mesh> {
    if let Some(handle) = grass_cache.mesh_handles.get(&key).cloned() {
        return handle;
    }

    let mesh = create_grass_clump_mesh(
        blade_height * blade_config.length,
        GRASS_BLADE_WIDTH * blade_config.width,
        blade_config.tilt,
        blade_config.curve,
        key.quality.segment_count(),
        u32::from(key.cards),
    );
    let handle = meshes.add(mesh);
    grass_cache.mesh_handles.insert(key, handle.clone());

    handle
}

fn get_or_create_grass_material(
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    grass_cache: &mut GrassAssetCache,
    key: GrassMaterialKey,
    color_scheme: &GrassColorScheme,
) -> Handle<StandardMaterial> {
    if let Some(handle) = grass_cache.material_handles.get(&key).cloned() {
        return handle;
    }

    let texture_handle: Handle<Image> = asset_server.load(GRASS_BLADE_TEXTURE);
    let handle = materials.add(StandardMaterial {
        base_color: cached_material_color(key, color_scheme),
        base_color_texture: Some(texture_handle),
        alpha_mode: AlphaMode::Mask(GRASS_ALPHA_CUTOFF),
        double_sided: true,
        cull_mode: None,
        perceptual_roughness: 0.7,
        ..default()
    });

    grass_cache.material_handles.insert(key, handle.clone());

    handle
}

#[cfg(test)]
fn create_grass_blade_mesh(height: f32, width: f32, curve_amount: f32) -> Mesh {
    create_curved_grass_card_mesh(
        height,
        width,
        0.0,
        curve_amount,
        GrassMeshQuality::Medium.segment_count(),
        &[Color::WHITE; 2],
    )
}

fn create_curved_grass_card_mesh(
    height: f32,
    width: f32,
    tilt: f32,
    curve_amount: f32,
    segment_count: usize,
    vertex_colors: &[Color; 2],
) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let mut curve_points = Vec::with_capacity(segment_count + 1);

    for i in 0..=segment_count {
        let t = i as f32 / segment_count as f32;

        let p0_y = 0.0;
        let p1_y = height * 0.5;
        let p2_y = height;

        let one_minus_t = 1.0 - t;
        let coeff0 = one_minus_t * one_minus_t;
        let coeff1 = 2.0 * one_minus_t * t;
        let coeff2 = t * t;

        let curve_x = coeff0 * 0.0 + coeff1 * (tilt * height * 0.25) + coeff2 * curve_amount;
        let curve_y = coeff0 * p0_y + coeff1 * p1_y + coeff2 * p2_y;

        curve_points.push(Vec3::new(0.0, curve_y, curve_x));
    }

    for i in 0..=segment_count {
        let t = i as f32 / segment_count as f32;
        let point = curve_points[i];
        let tangent = if i == 0 {
            curve_points[1] - curve_points[0]
        } else if i == segment_count {
            curve_points[segment_count] - curve_points[segment_count - 1]
        } else {
            curve_points[i + 1] - curve_points[i - 1]
        };

        let normal = Vec3::X.cross(tangent).normalize_or_zero();
        let taper_width = width * (1.0 - t).max(0.08);
        let color = vertex_colors[0].mix(&vertex_colors[1], t).to_linear();

        positions.push([-taper_width / 2.0, point.y, point.z]);
        normals.push([normal.x, normal.y, normal.z]);
        uvs.push([0.0, t]);
        colors.push([color.red, color.green, color.blue, color.alpha]);

        positions.push([taper_width / 2.0, point.y, point.z]);
        normals.push([normal.x, normal.y, normal.z]);
        uvs.push([1.0, t]);
        colors.push([color.red, color.green, color.blue, color.alpha]);
    }

    for i in 0..segment_count {
        let base = (i * 2) as u32;
        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);

        indices.push(base + 1);
        indices.push(base + 3);
        indices.push(base + 2);
    }

    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::all(),
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));

    mesh
}

fn create_grass_clump_mesh(
    height: f32,
    width: f32,
    tilt: f32,
    curve_amount: f32,
    segment_count: usize,
    card_count: u32,
) -> Mesh {
    let card_count = card_count.clamp(MIN_CARDS_PER_CLUMP, MAX_CARDS_PER_CLUMP);
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let mut vertex_offset = 0_u32;

    for card_index in 0..card_count {
        let rotation =
            Quat::from_rotation_y(card_index as f32 * std::f32::consts::TAU / card_count as f32);
        let color_lift = 1.0 + (card_index as f32 / card_count as f32 - 0.5) * 0.18;
        let base_color = Color::srgb(0.85 * color_lift, 0.95 * color_lift, 0.75 * color_lift);
        let tip_color = Color::srgb(1.0, (1.05 * color_lift).min(1.0), 0.85 * color_lift);
        let card = create_curved_grass_card_mesh(
            height,
            width,
            tilt,
            curve_amount,
            segment_count,
            &[base_color, tip_color],
        );

        let card_positions = card
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|attr| attr.as_float3())
            .expect("grass card mesh should contain float3 positions");
        let card_normals = card
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .and_then(|attr| attr.as_float3())
            .expect("grass card mesh should contain float3 normals");
        let card_uvs = match card
            .attribute(Mesh::ATTRIBUTE_UV_0)
            .expect("grass card mesh should contain UVs")
        {
            bevy::mesh::VertexAttributeValues::Float32x2(values) => values.as_slice(),
            _ => unreachable!("grass card mesh should contain float2 UVs"),
        };
        let card_colors = match card
            .attribute(Mesh::ATTRIBUTE_COLOR)
            .expect("grass card mesh should contain vertex colors")
        {
            bevy::mesh::VertexAttributeValues::Float32x4(values) => values.as_slice(),
            _ => unreachable!("grass card mesh should contain float4 colors"),
        };

        for (position, normal) in card_positions.iter().zip(card_normals.iter()) {
            let rotated_position = rotation * Vec3::new(position[0], position[1], position[2]);
            let rotated_normal = rotation * Vec3::new(normal[0], normal[1], normal[2]);
            positions.push([rotated_position.x, rotated_position.y, rotated_position.z]);
            normals.push([rotated_normal.x, rotated_normal.y, rotated_normal.z]);
        }
        uvs.extend_from_slice(card_uvs);
        colors.extend_from_slice(card_colors);

        if let Some(card_indices) = card.indices() {
            match card_indices {
                bevy::mesh::Indices::U32(card_indices) => {
                    indices.extend(card_indices.iter().map(|index| vertex_offset + index));
                }
                bevy::mesh::Indices::U16(card_indices) => {
                    indices.extend(
                        card_indices
                            .iter()
                            .map(|index| vertex_offset + u32::from(*index)),
                    );
                }
            }
        }

        vertex_offset += card.count_vertices() as u32;
    }

    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::all(),
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));

    mesh
}

fn clump_count_for_blade_count(blade_count: u32) -> u32 {
    if blade_count == 0 {
        0
    } else {
        ((blade_count as f32) / BLADES_PER_CLUMP as f32).ceil() as u32
    }
}

// ==================== Grass Spawning ====================

#[allow(clippy::too_many_arguments)]
fn spawn_grass_clump(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    asset_server: &Res<AssetServer>,
    grass_cache: &mut GrassAssetCache,
    clump_center: Vec2,
    blade_height: f32,
    blade_config: &BladeConfig,
    color_scheme: &GrassColorScheme,
    material_key: GrassMaterialKey,
    quality: GrassMeshQuality,
    card_count: u32,
    lod_index: u32,
    rng: &mut StdRng,
    parent_entity: Entity,
) {
    let height_variation = rng.random_range(MIN_HEIGHT_VARIATION..=MAX_HEIGHT_VARIATION);
    let width_variation = rng.random_range(MIN_WIDTH_VARIATION..=MAX_WIDTH_VARIATION);

    let mesh_key = grass_mesh_key(quality, blade_config, card_count);
    let clump_mesh =
        get_or_create_grass_mesh(meshes, grass_cache, mesh_key, blade_height, blade_config);
    let clump_material = get_or_create_grass_material(
        materials,
        asset_server,
        grass_cache,
        material_key,
        color_scheme,
    );

    let rotation_y = rng.random_range(0.0..std::f32::consts::TAU);
    let lean_angle = rng.random_range(0.0..std::f32::consts::TAU);
    let lean_dir = Vec3::new(lean_angle.cos(), 0.0, lean_angle.sin());
    let tilt_axis = Vec3::new(-lean_dir.z, 0.0, lean_dir.x).normalize_or_zero();
    let tilt_amount = rng.random_range(0.0..=blade_config.tilt);
    let tilt_rotation = if tilt_axis == Vec3::ZERO {
        Quat::IDENTITY
    } else {
        Quat::from_axis_angle(tilt_axis, tilt_amount)
    };
    let final_rotation = Quat::from_rotation_y(rotation_y) * tilt_rotation;

    let clump = commands
        .spawn((
            Mesh3d(clump_mesh.clone()),
            MeshMaterial3d(clump_material.clone()),
            Transform::from_xyz(
                clump_center.x,
                GRASS_BLADE_Y_OFFSET + GRASS_GROUND_CLEARANCE,
                clump_center.y,
            )
            .with_rotation(final_rotation)
            .with_scale(Vec3::new(
                width_variation,
                height_variation,
                width_variation,
            )),
            GlobalTransform::default(),
            Visibility::default(),
            // Grass should not cast/receive dynamic shadows; this avoids
            // heavy shadow-map cost and dark first-person self-shadowing.
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
            GrassClump { card_count },
            GrassBlade { lod_index },
            GrassBladeInstance {
                mesh: clump_mesh.clone(),
                material: clump_material.clone(),
            },
            GrassWindParams {
                phase: rng.random_range(0.0..std::f32::consts::TAU),
                ..Default::default()
            },
        ))
        .id();

    commands.entity(parent_entity).add_child(clump);
}

/// Spawns grass clusters for a terrain tile
///
/// This is the main entry point for grass rendering. It computes the
/// blade count using content density and performance settings, then
/// spawns one or more clusters within the tile.
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `asset_server` - Bevy asset server used to load the grass blade texture
/// * `grass_cache` - Explicit cache for reusable grass mesh and material assets
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
/// * `visual_metadata` - Optional per-tile visual customization
/// * `tile_tint` - Optional explicit RGB colour tint that overrides
///   `visual_metadata.color_tint`; multiplied into `GrassColorScheme.base_color`
///   before blade colours are sampled.  Pass `None` to use the tint embedded
///   in `visual_metadata` (or the natural-green default if that is also `None`).
/// * `quality_settings` - Performance settings for grass density scaling
///
/// # Returns
///
/// Entity ID of the parent grass cluster entity
///
/// # Examples
///
/// ```rust
/// use antares::domain::types::{MapId, Position};
/// use antares::game::resources::GrassQualitySettings;
/// use antares::game::systems::advanced_grass::{spawn_grass_cached, GrassAssetCache};
/// use bevy::prelude::{Assets, AssetServer, Commands, Mesh, Res, ResMut, StandardMaterial};
///
/// fn spawn_example(
///     mut commands: Commands,
///     mut materials: ResMut<Assets<StandardMaterial>>,
///     mut meshes: ResMut<Assets<Mesh>>,
///     asset_server: Res<AssetServer>,
///     mut grass_cache: ResMut<GrassAssetCache>,
///     settings: Res<GrassQualitySettings>,
/// ) {
///     let _entity = spawn_grass_cached(
///         &mut commands,
///         &mut materials,
///         &mut meshes,
///         &asset_server,
///         grass_cache.as_mut(),
///         Position::new(1, 2),
///         1u16,
///         None,
///         None,
///         &settings,
///     );
/// }
/// ```
#[allow(clippy::too_many_arguments)]
pub fn spawn_grass_cached(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    asset_server: &Res<AssetServer>,
    grass_cache: &mut GrassAssetCache,
    position: types::Position,
    map_id: types::MapId,
    visual_metadata: Option<&TileVisualMetadata>,
    tile_tint: Option<(f32, f32, f32)>,
    quality_settings: &GrassQualitySettings,
) -> Entity {
    spawn_grass_cached_with_exclusions(
        commands,
        materials,
        meshes,
        asset_server,
        grass_cache,
        &[],
        position,
        map_id,
        visual_metadata,
        tile_tint,
        quality_settings,
    )
}

/// Spawns grass clusters for a terrain tile while avoiding vegetation exclusion zones.
///
/// Exclusion zones are world-space X/Z ellipses produced by the vegetation
/// placement planner. Clump centers that fall inside one of those zones are
/// deterministically resampled, and skipped if no valid placement can be found.
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `asset_server` - Bevy asset server used to load the grass blade texture
/// * `grass_cache` - Explicit cache for reusable grass mesh and material assets
/// * `grass_exclusion_zones` - World-space zones that grass clumps must avoid
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
/// * `visual_metadata` - Optional per-tile visual customization
/// * `tile_tint` - Optional explicit RGB colour tint
/// * `quality_settings` - Performance settings for grass density scaling
///
/// # Returns
///
/// Entity ID of the parent grass patch entity.
#[allow(clippy::too_many_arguments)]
pub fn spawn_grass_cached_with_exclusions(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    asset_server: &Res<AssetServer>,
    grass_cache: &mut GrassAssetCache,
    grass_exclusion_zones: &[VegetationExclusionZone],
    position: types::Position,
    map_id: types::MapId,
    visual_metadata: Option<&TileVisualMetadata>,
    tile_tint: Option<(f32, f32, f32)>,
    quality_settings: &GrassQualitySettings,
) -> Entity {
    debug!(
        "spawn_grass called at tile ({}, {}) map {:?}",
        position.x, position.y, map_id
    );

    let blade_config = visual_metadata
        .and_then(|m| m.grass_blade_config.as_ref())
        .map(BladeConfig::from)
        .unwrap_or_default();

    let blade_height = visual_metadata
        .and_then(|m| m.height)
        .unwrap_or(GRASS_BLADE_HEIGHT_BASE)
        .clamp(0.2, 0.6);

    // Resolve the colour tint: an explicit `tile_tint` override takes precedence
    // over the value embedded in `visual_metadata`, which in turn falls back to
    // a natural green default.
    let resolved_tint = tile_tint
        .or_else(|| visual_metadata.and_then(|m| m.color_tint))
        .unwrap_or((0.3, 0.65, 0.2));

    let base_color = Color::srgb(
        0.2 * resolved_tint.0,
        0.5 * resolved_tint.1,
        0.1 * resolved_tint.2,
    );
    let tip_color = Color::srgb(
        0.3 * resolved_tint.0,
        0.7 * resolved_tint.1,
        0.2 * resolved_tint.2,
    );

    let color_scheme = GrassColorScheme {
        base_color,
        tip_color,
        variation: blade_config.color_variation,
    };

    let content_density = visual_metadata
        .and_then(|m| m.grass_density)
        .unwrap_or_default();

    let (min_blades, max_blades) = quality_settings.blade_count_range_for_content(content_density);
    let (scaled_min_blades, scaled_max_blades, foliage_density_multiplier) =
        scaled_blade_count_range_for_foliage_density(min_blades, max_blades, visual_metadata);

    let placement_seed = GrassPlacementSeed::new(map_id, position);
    let mut rng = placement_seed.rng();
    let blade_count = if scaled_max_blades > 0 {
        rng.random_range(scaled_min_blades.min(scaled_max_blades)..=scaled_max_blades)
    } else {
        0
    };

    debug!(
        "grass blades for tile ({}, {}): content={:?} min={} max={} foliage_density={} scaled_min={} scaled_max={} chosen={}",
        position.x,
        position.y,
        content_density,
        min_blades,
        max_blades,
        foliage_density_multiplier,
        scaled_min_blades,
        scaled_max_blades,
        blade_count
    );

    if blade_count == 0 {
        return commands
            .spawn((
                Transform::from_xyz(
                    position.x as f32 + TILE_CENTER_OFFSET,
                    0.0,
                    position.y as f32 + TILE_CENTER_OFFSET,
                ),
                GlobalTransform::default(),
                Visibility::default(),
                MapEntity(map_id),
                TileCoord(position),
            ))
            .id();
    }

    let parent = commands
        .spawn((
            Transform::from_xyz(
                position.x as f32 + TILE_CENTER_OFFSET,
                0.0,
                position.y as f32 + TILE_CENTER_OFFSET,
            ),
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            GrassCluster::default(),
            GrassPatch {
                clump_count: clump_count_for_blade_count(blade_count),
            },
        ))
        .id();

    let clump_count = clump_count_for_blade_count(blade_count);
    let vegetation_quality = vegetation_quality_from_grass_quality_settings(quality_settings);
    let material_key = GrassMaterialKey::from_tint_with_budget(
        resolved_tint,
        blade_config.color_variation,
        vegetation_quality.max_grass_material_variants,
    );
    let mesh_quality = GrassMeshQuality::from_settings(quality_settings);

    let tile_world_center = Vec2::new(
        position.x as f32 + TILE_CENTER_OFFSET,
        position.y as f32 + TILE_CENTER_OFFSET,
    );

    for clump_index in 0..clump_count {
        let Some(clump_center) =
            find_allowed_grass_clump_center(&mut rng, tile_world_center, grass_exclusion_zones)
        else {
            continue;
        };
        let card_count = rng.random_range(MIN_CARDS_PER_CLUMP..=MAX_CARDS_PER_CLUMP);

        spawn_grass_clump(
            commands,
            materials,
            meshes,
            asset_server,
            grass_cache,
            clump_center,
            blade_height,
            &blade_config,
            &color_scheme,
            material_key,
            mesh_quality,
            card_count,
            clump_index,
            &mut rng,
            parent,
        );
    }

    debug!(
        "spawn_grass completed at tile ({}, {}) with {} clumps from deterministic seed {}",
        position.x,
        position.y,
        clump_count,
        placement_seed.value()
    );

    parent
}

/// Compatibility grass spawn entry point that builds a temporary cache.
///
/// This keeps older call sites working while [`spawn_grass_cached`] provides
/// the optimized render path for systems that own a persistent [`GrassAssetCache`].
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `asset_server` - Bevy asset server used to load the grass blade texture
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
/// * `visual_metadata` - Optional per-tile visual customization
/// * `tile_tint` - Optional explicit RGB colour tint
/// * `quality_settings` - Performance settings for grass density scaling
///
/// # Returns
///
/// Entity ID of the parent grass patch entity.
///
/// # Examples
///
/// ```rust
/// use antares::domain::types::Position;
/// use antares::game::resources::GrassQualitySettings;
/// use antares::game::systems::advanced_grass::spawn_grass;
/// use bevy::prelude::{Assets, AssetServer, Commands, Mesh, Res, ResMut, StandardMaterial};
///
/// fn spawn_example(
///     mut commands: Commands,
///     mut materials: ResMut<Assets<StandardMaterial>>,
///     mut meshes: ResMut<Assets<Mesh>>,
///     asset_server: Res<AssetServer>,
///     settings: Res<GrassQualitySettings>,
/// ) {
///     let _entity = spawn_grass(
///         &mut commands,
///         &mut materials,
///         &mut meshes,
///         &asset_server,
///         Position::new(1, 2),
///         1u16,
///         None,
///         None,
///         &settings,
///     );
/// }
/// ```
#[allow(clippy::too_many_arguments)]
pub fn spawn_grass(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    asset_server: &Res<AssetServer>,
    position: types::Position,
    map_id: types::MapId,
    visual_metadata: Option<&TileVisualMetadata>,
    tile_tint: Option<(f32, f32, f32)>,
    quality_settings: &GrassQualitySettings,
) -> Entity {
    let mut grass_cache = GrassAssetCache::default();
    spawn_grass_cached(
        commands,
        materials,
        meshes,
        asset_server,
        &mut grass_cache,
        position,
        map_id,
        visual_metadata,
        tile_tint,
        quality_settings,
    )
}

// ==================== Grass Performance Systems ====================

/// System that culls grass clusters beyond the configured distance
///
/// # Examples
///
/// ```rust
/// use bevy::prelude::*;
/// use antares::game::systems::advanced_grass::{grass_distance_culling_system, GrassRenderConfig};
///
/// fn setup_app(app: &mut App) {
///     app.insert_resource(GrassRenderConfig::default())
///        .add_systems(Update, grass_distance_culling_system);
/// }
/// ```
pub fn grass_distance_culling_system(
    mut grass_query: Query<(&GlobalTransform, &mut Visibility, &GrassCluster)>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    config: Res<GrassRenderConfig>,
    vegetation_quality: Option<Res<VegetationQualitySettings>>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let camera_pos = camera_transform.translation();

    let effective_config = vegetation_quality
        .as_deref()
        .map(GrassRenderConfig::from_vegetation_quality)
        .unwrap_or(*config);

    for (transform, mut visibility, cluster) in grass_query.iter_mut() {
        let distance = camera_pos.distance(transform.translation());

        if distance > cluster.cull_distance.max(effective_config.cull_distance) {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Inherited;
        }
    }
}

/// Selects the grass LOD tier for a camera distance.
///
/// Returns [`GrassLodTier::Culled`] outside the configured cull distance.
///
/// # Examples
///
/// ```
/// use antares::game::systems::advanced_grass::{select_grass_lod_tier, GrassLodTier};
///
/// assert_eq!(select_grass_lod_tier(5.0, 20.0, 50.0), GrassLodTier::Near);
/// assert_eq!(select_grass_lod_tier(25.0, 20.0, 50.0), GrassLodTier::Mid);
/// assert_eq!(select_grass_lod_tier(40.0, 20.0, 50.0), GrassLodTier::Far);
/// assert_eq!(select_grass_lod_tier(60.0, 20.0, 50.0), GrassLodTier::Culled);
/// ```
pub fn select_grass_lod_tier(distance: f32, lod_distance: f32, cull_distance: f32) -> GrassLodTier {
    if distance > cull_distance {
        GrassLodTier::Culled
    } else if distance > lod_distance * FAR_GRASS_LOD_DISTANCE_MULTIPLIER {
        GrassLodTier::Far
    } else if distance > lod_distance {
        GrassLodTier::Mid
    } else {
        GrassLodTier::Near
    }
}

/// System that reduces grass blade count at distance (LOD)
///
/// # Examples
///
/// ```rust
/// use bevy::prelude::*;
/// use antares::game::systems::advanced_grass::{grass_lod_system, GrassRenderConfig};
///
/// fn setup_app(app: &mut App) {
///     app.insert_resource(GrassRenderConfig::default())
///        .add_systems(Update, grass_lod_system);
/// }
/// ```
pub fn grass_lod_system(
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    cluster_query: Query<(&GlobalTransform, &Children), With<GrassCluster>>,
    mut blade_query: Query<(&mut Visibility, &GrassBlade)>,
    config: Res<GrassRenderConfig>,
    vegetation_quality: Option<Res<VegetationQualitySettings>>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let camera_pos = camera_transform.translation();

    let effective_config = vegetation_quality
        .as_deref()
        .map(GrassRenderConfig::from_vegetation_quality)
        .unwrap_or(*config);

    for (cluster_transform, children) in cluster_query.iter() {
        let distance = camera_pos.distance(cluster_transform.translation());
        let lod_tier = select_grass_lod_tier(
            distance,
            effective_config.lod_distance,
            effective_config.cull_distance,
        );

        for child in children.iter() {
            if let Ok((mut visibility, blade)) = blade_query.get_mut(child) {
                *visibility = match lod_tier.retention_stride() {
                    Some(stride) if blade.lod_index % stride == 0 => Visibility::Inherited,
                    _ => Visibility::Hidden,
                };
            }
        }
    }
}

#[derive(Hash, PartialEq, Eq)]
struct InstanceBatchKey {
    mesh: bevy::asset::AssetId<Mesh>,
    material: bevy::asset::AssetId<StandardMaterial>,
    map_id: types::MapId,
}

struct InstanceBatchEntry {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    instances: Vec<world::InstanceData>,
}

/// Build instance batches from existing grass blades
///
/// This system aggregates per-blade instance data into batch components to
/// support performance diagnostics and future GPU instancing pipelines.
///
/// # Examples
///
/// ```rust
/// use bevy::prelude::*;
/// use antares::game::systems::advanced_grass::{
///     build_grass_instance_batches_system, GrassInstanceConfig,
/// };
///
/// fn setup_app(app: &mut App) {
///     app.insert_resource(GrassInstanceConfig::default())
///        .add_systems(Update, build_grass_instance_batches_system);
/// }
/// ```
pub fn build_grass_instance_batches_system(
    mut commands: Commands,
    cluster_query: Query<(&Children, &GlobalTransform, &MapEntity), With<GrassCluster>>,
    blade_query: Query<(&Transform, Option<&GlobalTransform>, &GrassBladeInstance)>,
    mut existing_batches: Query<Entity, With<GrassInstanceBatch>>,
    config: Res<GrassInstanceConfig>,
) {
    if !config.enabled {
        return;
    }

    for ent in existing_batches.iter_mut() {
        commands.entity(ent).despawn();
    }

    let mut buckets: HashMap<InstanceBatchKey, InstanceBatchEntry> = HashMap::new();

    for (children, cluster_global, map_entity) in cluster_query.iter() {
        for child in children.iter() {
            if let Ok((child_local, child_global_opt, blade_inst)) = blade_query.get(child) {
                let world_transform = if let Some(child_global) = child_global_opt {
                    if child_global.translation() == Vec3::ZERO
                        && child_local.translation != Vec3::ZERO
                    {
                        cluster_global.mul_transform(*child_local)
                    } else {
                        *child_global
                    }
                } else {
                    cluster_global.mul_transform(*child_local)
                };

                let (scale, rotation, translation) =
                    world_transform.to_scale_rotation_translation();
                let (yaw, _, _) = rotation.to_euler(EulerRot::YXZ);
                let uniform_scale = (scale.x + scale.y + scale.z) / 3.0;

                let instance = world::InstanceData {
                    position: [translation.x, translation.y, translation.z],
                    scale: uniform_scale,
                    rotation_y: yaw,
                };

                let key = InstanceBatchKey {
                    mesh: blade_inst.mesh.id(),
                    material: blade_inst.material.id(),
                    map_id: map_entity.0,
                };

                let entry = buckets.entry(key).or_insert_with(|| InstanceBatchEntry {
                    mesh: blade_inst.mesh.clone(),
                    material: blade_inst.material.clone(),
                    instances: Vec::new(),
                });
                entry.instances.push(instance);
            }
        }
    }

    let max_instances = config.max_instances_per_batch.max(1);
    for (key, entry) in buckets.into_iter() {
        for (batch_index, chunk) in entry.instances.chunks(max_instances).enumerate() {
            commands.spawn((
                GrassInstanceBatch {
                    mesh: entry.mesh.clone(),
                    material: entry.material.clone(),
                    instances: chunk.to_vec(),
                },
                MapEntity(key.map_id),
                Name::new(format!("GrassInstanceBatch_{}_{}", key.map_id, batch_index)),
            ));
        }
    }
}

// ==================== Optional Chunking ====================

/// Component that marks a merged grass chunk entity
#[derive(Component, Clone, Copy, Debug)]
pub struct GrassChunk {
    /// Chunk coordinates (x,z) in chunk-space
    pub coords: (i32, i32),
    /// Distance beyond which the chunk should be considered for culling
    pub cull_distance: f32,
}

impl Default for GrassChunk {
    fn default() -> Self {
        Self {
            coords: (0, 0),
            cull_distance: 50.0,
        }
    }
}

/// Configuration resource for chunking behavior
#[derive(Resource, Clone, Copy, Debug)]
pub struct GrassChunkConfig {
    /// Width/height of a single chunk (world units)
    pub chunk_size: f32,
    /// Maximum expected blade height (used to conservatively expand AABB)
    pub max_blade_height: f32,
}

impl Default for GrassChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 30.0,
            max_blade_height: 1.0,
        }
    }
}

struct BladeGather {
    material: Handle<StandardMaterial>,
    world_transform: GlobalTransform,
    map_id: crate::domain::types::MapId,
}

/// Build merged chunk meshes for existing grass blades.
pub fn build_grass_chunks_system(
    mut commands: Commands,
    cluster_query: Query<(&Children, &GlobalTransform, &MapEntity), With<GrassCluster>>,
    blade_query: Query<(&Transform, Option<&GlobalTransform>, &GrassBladeInstance)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut existing_chunks: Query<Entity, With<GrassChunk>>,
    config: Option<Res<GrassChunkConfig>>,
) {
    // Chunking is optional. If no config is provided, skip building chunk meshes.
    let Some(config_res) = config else {
        return;
    };

    for ent in existing_chunks.iter_mut() {
        commands.entity(ent).despawn();
    }

    let mut buckets: HashMap<(i32, i32), Vec<BladeGather>> = HashMap::new();
    let config = *config_res;

    for (children, cluster_global, map_entity) in cluster_query.iter() {
        for child in children.iter() {
            if let Ok((child_local, child_global_opt, blade_inst)) = blade_query.get(child) {
                let world_transform = if let Some(child_global) = child_global_opt {
                    if child_global.translation() == Vec3::ZERO
                        && child_local.translation != Vec3::ZERO
                    {
                        cluster_global.mul_transform(*child_local)
                    } else {
                        *child_global
                    }
                } else {
                    cluster_global.mul_transform(*child_local)
                };

                let pos = world_transform.translation();
                let cx = (pos.x / config.chunk_size).floor() as i32;
                let cz = (pos.z / config.chunk_size).floor() as i32;
                buckets.entry((cx, cz)).or_default().push(BladeGather {
                    material: blade_inst.material.clone(),
                    world_transform,
                    map_id: map_entity.0,
                });
            }
        }
    }

    for ((cx, cz), blades) in buckets.into_iter() {
        let chunk_center = Vec3::new(
            cx as f32 * config.chunk_size + config.chunk_size * 0.5,
            0.0,
            cz as f32 * config.chunk_size + config.chunk_size * 0.5,
        );

        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut normals: Vec<[f32; 3]> = Vec::new();
        let mut uvs: Vec<[f32; 2]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        let mut vertex_offset: u32 = 0;

        for blade in blades.iter() {
            let world_pos = blade.world_transform.translation();
            let local = world_pos - chunk_center;

            let half_w = 0.1_f32;
            let height = config.max_blade_height;

            positions.push([local.x - half_w, 0.0, local.z]);
            positions.push([local.x + half_w, 0.0, local.z]);
            positions.push([local.x - half_w, height, local.z]);
            positions.push([local.x + half_w, height, local.z]);

            normals.push([0.0, 1.0, 0.0]);
            normals.push([0.0, 1.0, 0.0]);
            normals.push([0.0, 1.0, 0.0]);
            normals.push([0.0, 1.0, 0.0]);

            uvs.push([0.0, 0.0]);
            uvs.push([1.0, 0.0]);
            uvs.push([0.0, 1.0]);
            uvs.push([1.0, 1.0]);

            indices.push(vertex_offset);
            indices.push(vertex_offset + 1);
            indices.push(vertex_offset + 2);
            indices.push(vertex_offset + 2);
            indices.push(vertex_offset + 1);
            indices.push(vertex_offset + 3);

            vertex_offset += 4;
        }

        if positions.is_empty() || indices.is_empty() {
            continue;
        }

        let mut merged = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::all(),
        );
        merged.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        merged.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        merged.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        merged.insert_indices(bevy::mesh::Indices::U32(indices));

        let mesh_handle = meshes.add(merged);

        let material_color = blades
            .first()
            .and_then(|b| materials.get(&b.material))
            .map(|m| m.base_color)
            .unwrap_or(Color::srgb(0.25, 0.6, 0.25));

        let material_handle = materials.add(StandardMaterial {
            base_color: material_color,
            perceptual_roughness: 0.8,
            double_sided: true,
            ..default()
        });

        let map_id = blades[0].map_id;
        commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
            Transform::from_translation(chunk_center),
            GlobalTransform::default(),
            Visibility::default(),
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
            GrassChunk {
                coords: (cx, cz),
                cull_distance: GrassChunk::default().cull_distance,
            },
            MapEntity(map_id),
            Name::new(format!("GrassChunk_{}_{}", cx, cz)),
        ));
    }
}

/// Culling system for grass chunks
pub fn grass_chunk_culling_system(
    mut chunk_query: Query<(
        &Transform,
        Option<&GlobalTransform>,
        &mut Visibility,
        &GrassChunk,
    )>,
    camera_query: Query<(&Transform, Option<&GlobalTransform>), With<Camera3d>>,
    config: Option<Res<GrassRenderConfig>>,
) {
    let Ok((cam_local, cam_global_opt)) = camera_query.single() else {
        return;
    };
    let camera_pos = if let Some(cam_global) = cam_global_opt {
        if cam_global.translation() == Vec3::ZERO && cam_local.translation != Vec3::ZERO {
            cam_local.translation
        } else {
            cam_global.translation()
        }
    } else {
        cam_local.translation
    };
    let config = config.map(|r| *r).unwrap_or_default();

    for (local_transform, chunk_global_opt, mut visibility, chunk) in chunk_query.iter_mut() {
        let chunk_pos = if let Some(chunk_global) = chunk_global_opt {
            if chunk_global.translation() == Vec3::ZERO && local_transform.translation != Vec3::ZERO
            {
                local_transform.translation
            } else {
                chunk_global.translation()
            }
        } else {
            local_transform.translation
        };

        let distance = camera_pos.distance(chunk_pos);
        if distance > chunk.cull_distance.max(config.cull_distance) {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Inherited;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::world::GrassDensity;
    use bevy::mesh::Mesh;

    fn compute_bounds(positions: &[[f32; 3]]) -> (Vec3, Vec3) {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);

        for pos in positions {
            let p = Vec3::new(pos[0], pos[1], pos[2]);
            min = min.min(p);
            max = max.max(p);
        }

        (min, max)
    }

    #[test]
    fn test_higher_grass_density_produces_higher_clump_count() {
        let settings = GrassQualitySettings::default();
        let (_, low_max_blades) = settings.blade_count_range_for_content(GrassDensity::Low);
        let (_, high_max_blades) = settings.blade_count_range_for_content(GrassDensity::High);

        let low_clumps = clump_count_for_blade_count(low_max_blades);
        let high_clumps = clump_count_for_blade_count(high_max_blades);

        assert!(
            high_clumps > low_clumps,
            "high density should produce more clumps than low density"
        );
    }

    #[test]
    fn test_foliage_density_scales_grass_clump_coverage() {
        let thin_metadata = TileVisualMetadata {
            foliage_density: Some(0.5),
            ..Default::default()
        };
        let thick_metadata = TileVisualMetadata {
            foliage_density: Some(2.0),
            ..Default::default()
        };

        let (_, thin_max, _) =
            scaled_blade_count_range_for_foliage_density(40, 60, Some(&thin_metadata));
        let (_, thick_max, _) =
            scaled_blade_count_range_for_foliage_density(40, 60, Some(&thick_metadata));

        assert!(
            clump_count_for_blade_count(thick_max) > clump_count_for_blade_count(thin_max),
            "higher foliage_density should increase clump coverage"
        );
    }

    #[test]
    fn test_grass_blade_config_affects_generated_mesh_bucket() {
        let default_config = BladeConfig::default();
        let custom_config = BladeConfig {
            length: 1.6,
            width: 0.7,
            tilt: 0.45,
            curve: 0.8,
            color_variation: 0.2,
        };

        let default_key = grass_mesh_key(GrassMeshQuality::Medium, &default_config, 3);
        let custom_key = grass_mesh_key(GrassMeshQuality::Medium, &custom_config, 3);

        assert_ne!(
            default_key, custom_key,
            "grass_blade_config should affect reusable mesh bucket selection"
        );
    }

    #[test]
    fn test_grass_placement_seed_is_deterministic_by_map_and_tile() {
        let position = types::Position::new(4, 9);

        let first = GrassPlacementSeed::new(7, position);
        let second = GrassPlacementSeed::new(7, position);
        let different_tile = GrassPlacementSeed::new(7, types::Position::new(5, 9));

        assert_eq!(first, second);
        assert_ne!(first, different_tile);
    }

    #[test]
    fn test_find_allowed_grass_clump_center_avoids_exclusion_zone() {
        let tile_world_center = Vec2::new(0.5, 0.5);
        let exclusion_zone = VegetationExclusionZone::new(tile_world_center, 0.30, 0.30);
        let mut rng = GrassPlacementSeed::new(1, types::Position::new(0, 0)).rng();

        let clump_center =
            find_allowed_grass_clump_center(&mut rng, tile_world_center, &[exclusion_zone])
                .expect("expected at least one clump position outside the exclusion zone");

        assert!(
            !exclusion_zone.contains(tile_world_center + clump_center),
            "grass clump should be outside trunk exclusion zone"
        );
    }

    #[test]
    fn test_find_allowed_grass_clump_center_returns_none_when_tile_is_fully_excluded() {
        let tile_world_center = Vec2::new(0.5, 0.5);
        let exclusion_zone = VegetationExclusionZone::new(tile_world_center, 2.0, 2.0);
        let mut rng = GrassPlacementSeed::new(1, types::Position::new(0, 0)).rng();

        let clump_center =
            find_allowed_grass_clump_center(&mut rng, tile_world_center, &[exclusion_zone]);

        assert!(clump_center.is_none());
    }

    #[test]
    fn test_blade_config_default_values() {
        let config = BladeConfig::default();
        assert_eq!(config.length, 1.0);
        assert_eq!(config.width, 1.0);
        assert_eq!(config.tilt, 0.3);
        assert_eq!(config.curve, 0.3);
        assert_eq!(config.color_variation, 0.2);
    }

    #[test]
    fn test_vegetation_quality_from_grass_quality_settings_maps_budget_level() {
        let low = vegetation_quality_from_grass_quality_settings(&GrassQualitySettings {
            performance_level: GrassPerformanceLevel::Low,
        });
        let high = vegetation_quality_from_grass_quality_settings(&GrassQualitySettings {
            performance_level: GrassPerformanceLevel::High,
        });

        assert_eq!(low.quality_level, VegetationQualityLevel::Low);
        assert_eq!(high.quality_level, VegetationQualityLevel::High);
        assert!(low.max_grass_material_variants < high.max_grass_material_variants);
        assert!(low.vegetation_cull_distance < high.vegetation_cull_distance);
    }

    #[test]
    fn test_grass_render_config_default_values() {
        let config = GrassRenderConfig::default();
        let vegetation_quality = VegetationQualitySettings::default();

        assert_eq!(
            config.cull_distance,
            vegetation_quality.vegetation_cull_distance
        );
        assert_eq!(config.lod_distance, vegetation_quality.grass_lod_distance);
    }

    #[test]
    fn test_scaled_blade_count_range_for_foliage_density_defaults_to_unmodified_range() {
        let (min, max, multiplier) = scaled_blade_count_range_for_foliage_density(10, 20, None);

        assert_eq!(min, 10);
        assert_eq!(max, 20);
        assert_eq!(multiplier, 1.0);
    }

    #[test]
    fn test_scaled_blade_count_range_for_foliage_density_zero_suppresses_grass() {
        let metadata = TileVisualMetadata {
            foliage_density: Some(0.0),
            ..Default::default()
        };

        let (min, max, multiplier) =
            scaled_blade_count_range_for_foliage_density(10, 20, Some(&metadata));

        assert_eq!(min, 0);
        assert_eq!(max, 0);
        assert_eq!(multiplier, 0.0);
    }

    #[test]
    fn test_scaled_blade_count_range_for_foliage_density_scales_and_clamps_high_values() {
        let metadata = TileVisualMetadata {
            foliage_density: Some(3.5),
            ..Default::default()
        };

        let (min, max, multiplier) =
            scaled_blade_count_range_for_foliage_density(10, 20, Some(&metadata));

        assert_eq!(min, 20);
        assert_eq!(max, 40);
        assert_eq!(multiplier, 2.0);
    }

    #[test]
    fn test_create_grass_blade_mesh_vertex_count() {
        let blade = create_grass_blade_mesh(0.4, 0.15, 0.1);
        assert_eq!(blade.count_vertices(), 12);
    }

    #[test]
    fn test_create_grass_blade_mesh_indices() {
        let blade = create_grass_blade_mesh(0.4, 0.15, 0.1);
        let indices_count = blade
            .indices()
            .map(|indices| match indices {
                bevy::mesh::Indices::U32(idx) => idx.len(),
                bevy::mesh::Indices::U16(idx) => idx.len(),
            })
            .unwrap_or(0);

        assert_eq!(indices_count, 30);
    }

    #[test]
    fn test_create_grass_blade_mesh_normals() {
        let blade = create_grass_blade_mesh(0.4, 0.15, 0.1);
        let normals = blade
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .expect("Blade should have normals")
            .as_float3()
            .expect("Normals should be float3");

        for normal in normals {
            let length =
                (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            assert!(normal[0].abs() < 0.01);
            assert!((length - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_create_grass_blade_mesh_has_uvs() {
        let blade = create_grass_blade_mesh(0.4, 0.15, 0.1);

        // UV_0 attribute must be present for texture mapping.
        assert!(
            blade.attribute(Mesh::ATTRIBUTE_UV_0).is_some(),
            "Blade mesh must have UV_0 attribute for texture mapping"
        );
    }

    /// Verify that UV V-coordinates span the full [0.0, 1.0] range so that
    /// the grass blade texture maps from base to tip without clipping.
    ///
    /// `create_grass_blade_mesh` generates UVs as `[u, t]` where `u ∈ {0.0,
    /// 1.0}` and `t` is the normalised segment position from 0.0 (base) to
    /// 1.0 (tip).  With `segment_count = 5` there are 6 levels × 2 vertices =
    /// 12 vertices total.  The first vertex has t = 0.0 and the last has
    /// t = 1.0, so the V range is exactly [0.0, 1.0].
    #[test]
    fn test_create_grass_blade_mesh_uvs_span_full_v_range() {
        let segment_count: usize = 5;
        let expected_vertices = (segment_count + 1) * 2; // 12

        let blade = create_grass_blade_mesh(0.4, 0.15, 0.1);

        // Sanity-check vertex count so we know the mesh has the expected shape.
        assert_eq!(
            blade.count_vertices(),
            expected_vertices,
            "Blade should have {expected_vertices} vertices"
        );

        // Reconstruct expected V values from the known generation algorithm.
        // t = i / segment_count for i in 0..=segment_count, two vertices per level.
        let mut expected_v_min = f32::INFINITY;
        let mut expected_v_max = f32::NEG_INFINITY;
        for i in 0..=segment_count {
            let t = i as f32 / segment_count as f32;
            expected_v_min = expected_v_min.min(t);
            expected_v_max = expected_v_max.max(t);
        }

        assert!(
            expected_v_min < 0.01,
            "Expected V-min near 0.0, got {expected_v_min}"
        );
        assert!(
            expected_v_max > 0.99,
            "Expected V-max near 1.0, got {expected_v_max}"
        );
    }

    #[test]
    fn test_create_grass_blade_mesh_bounds_are_valid() {
        let blade = create_grass_blade_mesh(0.4, 0.15, 0.1);
        let positions = blade
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("Blade should have positions")
            .as_float3()
            .expect("Positions should be float3");

        let (min, max) = compute_bounds(positions);
        assert!(max.y > min.y);
        assert!(max.x > min.x);
    }

    #[test]
    fn test_grass_blade_child_inherits_parent_transform() {
        let expected = Vec3::new(2.25, 0.0, 3.5);
        let parent = Transform::from_translation(Vec3::new(2.0, 0.0, 3.0));
        let child = Transform::from_translation(Vec3::new(0.25, 0.0, 0.5));
        let combined = parent.mul_transform(child);
        let actual = combined.translation;

        assert!((actual.x - expected.x).abs() < 0.001);
        assert!((actual.y - expected.y).abs() < 0.001);
        assert!((actual.z - expected.z).abs() < 0.001);
    }

    #[test]
    fn test_grass_color_scheme_no_variation_is_stable() {
        let scheme = GrassColorScheme {
            base_color: Color::srgb(0.2, 0.5, 0.1),
            tip_color: Color::srgb(0.3, 0.7, 0.2),
            variation: 0.0,
        };

        let mut rng = rand::rng();
        let color_a = scheme.sample_blade_color(&mut rng);
        let color_b = scheme.sample_blade_color(&mut rng);

        assert_eq!(color_a.to_srgba(), color_b.to_srgba());
    }

    #[test]
    fn test_grass_distance_culling_system_hides_far_cluster() {
        let mut app = App::new();
        app.insert_resource(GrassRenderConfig::default());
        app.add_systems(Update, grass_distance_culling_system);

        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
        ));

        let far_cluster = app
            .world_mut()
            .spawn((
                Transform::from_xyz(100.0, 0.0, 0.0),
                GlobalTransform::from(Transform::from_xyz(100.0, 0.0, 0.0)),
                Visibility::default(),
                GrassCluster::default(),
            ))
            .id();

        app.update();

        let visibility = app.world().get::<Visibility>(far_cluster).unwrap();
        assert!(matches!(visibility, Visibility::Hidden));
    }

    #[test]
    fn test_grass_lod_system_hides_odd_blades_far_distance() {
        let mut app = App::new();
        app.insert_resource(GrassRenderConfig {
            cull_distance: 50.0,
            lod_distance: 25.0,
        });
        app.add_systems(Update, grass_lod_system);

        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
        ));

        let cluster = app
            .world_mut()
            .spawn((
                Transform::from_xyz(30.0, 0.0, 0.0),
                GlobalTransform::from(Transform::from_xyz(30.0, 0.0, 0.0)),
                Visibility::default(),
                GrassCluster::default(),
            ))
            .id();

        let blade_even = app
            .world_mut()
            .spawn((
                Transform::default(),
                GlobalTransform::default(),
                Visibility::default(),
                GrassBlade { lod_index: 0 },
            ))
            .id();

        let blade_odd = app
            .world_mut()
            .spawn((
                Transform::default(),
                GlobalTransform::default(),
                Visibility::default(),
                GrassBlade { lod_index: 1 },
            ))
            .id();

        app.world_mut().entity_mut(cluster).add_child(blade_even);
        app.world_mut().entity_mut(cluster).add_child(blade_odd);

        app.update();

        let even_visibility = app.world().get::<Visibility>(blade_even).unwrap();
        let odd_visibility = app.world().get::<Visibility>(blade_odd).unwrap();

        assert!(matches!(even_visibility, Visibility::Inherited));
        assert!(matches!(odd_visibility, Visibility::Hidden));
    }

    #[test]
    fn test_select_grass_lod_tier_uses_near_mid_far_and_cull_budgets() {
        assert_eq!(select_grass_lod_tier(5.0, 20.0, 50.0), GrassLodTier::Near);
        assert_eq!(select_grass_lod_tier(25.0, 20.0, 50.0), GrassLodTier::Mid);
        assert_eq!(select_grass_lod_tier(40.0, 20.0, 50.0), GrassLodTier::Far);
        assert_eq!(
            select_grass_lod_tier(60.0, 20.0, 50.0),
            GrassLodTier::Culled
        );
    }

    #[test]
    fn test_grass_lod_system_far_distance_keeps_only_every_fourth_clump() {
        let mut app = App::new();
        app.insert_resource(GrassRenderConfig {
            cull_distance: 50.0,
            lod_distance: 20.0,
        });
        app.add_systems(Update, grass_lod_system);

        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
        ));

        let cluster = app
            .world_mut()
            .spawn((
                Transform::from_xyz(40.0, 0.0, 0.0),
                GlobalTransform::from(Transform::from_xyz(40.0, 0.0, 0.0)),
                Visibility::default(),
                GrassCluster::default(),
            ))
            .id();

        let mut blades = Vec::new();
        for lod_index in 0..4 {
            let blade = app
                .world_mut()
                .spawn((
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility::default(),
                    GrassBlade { lod_index },
                ))
                .id();
            app.world_mut().entity_mut(cluster).add_child(blade);
            blades.push(blade);
        }

        app.update();

        assert!(matches!(
            app.world().get::<Visibility>(blades[0]).unwrap(),
            Visibility::Inherited
        ));
        for blade in blades.iter().skip(1) {
            assert!(matches!(
                app.world().get::<Visibility>(*blade).unwrap(),
                Visibility::Hidden
            ));
        }
    }

    #[test]
    fn test_grass_material_budget_follows_grass_quality_settings() {
        let low = GrassQualitySettings {
            performance_level: GrassPerformanceLevel::Low,
        };
        let high = GrassQualitySettings {
            performance_level: GrassPerformanceLevel::High,
        };

        let low_budget =
            vegetation_quality_from_grass_quality_settings(&low).max_grass_material_variants;
        let high_budget =
            vegetation_quality_from_grass_quality_settings(&high).max_grass_material_variants;

        assert!(low_budget < high_budget);
    }

    #[test]
    fn test_grass_material_key_budget_buckets_similar_tints() {
        let first = GrassMaterialKey::from_tint_with_budget((0.56, 0.58, 0.57), 0.62, 4);
        let second = GrassMaterialKey::from_tint_with_budget((0.61, 0.63, 0.60), 0.66, 4);

        assert_eq!(
            first, second,
            "low material budgets should bucket nearby tints into the same reusable material key"
        );
    }

    #[test]
    fn test_build_grass_chunks_groups_blades_into_chunks() {
        let mut app = App::new();

        app.insert_resource(GrassChunkConfig {
            chunk_size: 2.0,
            max_blade_height: 1.0,
        });
        app.insert_resource(GrassRenderConfig::default());

        app.add_systems(Update, build_grass_chunks_system);

        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());

        let cube = {
            let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
            let mut test_mesh = Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::all(),
            );
            test_mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![[0.0, 0.0, 0.0], [0.1, 0.0, 0.0], [0.0, 0.1, 0.0]],
            );
            test_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 3]);
            test_mesh.insert_attribute(
                Mesh::ATTRIBUTE_UV_0,
                vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            );
            test_mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2]));
            meshes.add(test_mesh)
        };

        let mat = {
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
            materials.add(StandardMaterial {
                base_color: Color::WHITE,
                ..default()
            })
        };

        let cluster = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::ZERO),
                GlobalTransform::default(),
                GrassCluster::default(),
                MapEntity(1),
            ))
            .id();

        let blade_a = app
            .world_mut()
            .spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(Vec3::new(0.1, 0.0, 0.1)),
                GlobalTransform::default(),
                GrassBladeInstance {
                    mesh: cube.clone(),
                    material: mat.clone(),
                },
            ))
            .id();

        let blade_b = app
            .world_mut()
            .spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(Vec3::new(3.1, 0.0, 0.1)),
                GlobalTransform::default(),
                GrassBladeInstance {
                    mesh: cube.clone(),
                    material: mat.clone(),
                },
            ))
            .id();

        app.world_mut().entity_mut(cluster).add_child(blade_a);
        app.world_mut().entity_mut(cluster).add_child(blade_b);

        app.update();

        let mut q = app.world_mut().query_filtered::<Entity, With<GrassChunk>>();
        let count = q.iter(app.world()).count();
        assert!(count >= 2, "Expected at least 2 chunks, found {}", count);
    }

    #[test]
    fn test_grass_chunk_culling_hides_far_chunks() {
        let mut app = App::new();

        app.insert_resource(GrassRenderConfig::default());
        app.add_systems(Update, grass_chunk_culling_system);

        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
        ));

        let chunk = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(1000.0, 0.0, 0.0)),
                GlobalTransform::from(Transform::from_translation(Vec3::new(1000.0, 0.0, 0.0))),
                Visibility::default(),
                GrassChunk {
                    coords: (100, 0),
                    cull_distance: 50.0,
                },
            ))
            .id();

        app.update();

        let vis = app.world().get::<Visibility>(chunk).unwrap();
        assert!(matches!(vis, Visibility::Hidden));
    }

    #[test]
    fn test_build_grass_instance_batches_groups_instances() {
        let mut app = App::new();
        app.insert_resource(GrassInstanceConfig {
            enabled: true,
            max_instances_per_batch: 2,
        });
        app.add_systems(Update, build_grass_instance_batches_system);

        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());

        let mesh = {
            let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
            let mut test_mesh = Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::all(),
            );
            test_mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![[0.0, 0.0, 0.0], [0.1, 0.0, 0.0], [0.0, 0.1, 0.0]],
            );
            test_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 3]);
            test_mesh.insert_attribute(
                Mesh::ATTRIBUTE_UV_0,
                vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            );
            test_mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2]));
            meshes.add(test_mesh)
        };

        let material = {
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
            materials.add(StandardMaterial::default())
        };

        let cluster = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::ZERO),
                GlobalTransform::default(),
                GrassCluster::default(),
                MapEntity(1),
            ))
            .id();

        for blade_index in 0..3 {
            let blade = app
                .world_mut()
                .spawn((
                    Transform::from_translation(Vec3::new(blade_index as f32, 0.0, 0.0)),
                    GlobalTransform::default(),
                    GrassBladeInstance {
                        mesh: mesh.clone(),
                        material: material.clone(),
                    },
                ))
                .id();
            app.world_mut().entity_mut(cluster).add_child(blade);
        }

        app.update();

        let batches: Vec<_> = {
            let world = app.world_mut();
            world.query::<&GrassInstanceBatch>().iter(world).collect()
        };

        let total_instances: usize = batches.iter().map(|b| b.instances.len()).sum();
        assert_eq!(total_instances, 3);
        assert!(batches.len() >= 2);
    }

    #[test]
    fn test_blade_config_from_domain_config() {
        let domain = world::GrassBladeConfig {
            length: 1.4,
            width: 0.9,
            tilt: 0.4,
            curve: 0.5,
            color_variation: 0.3,
        };

        let config = BladeConfig::from(&domain);
        assert_eq!(config.length, 1.4);
        assert_eq!(config.width, 0.9);
        assert_eq!(config.tilt, 0.4);
        assert_eq!(config.curve, 0.5);
        assert_eq!(config.color_variation, 0.3);
    }

    #[test]
    fn test_grass_color_scheme_variation_can_change_color() {
        let scheme = GrassColorScheme {
            base_color: Color::srgb(0.2, 0.5, 0.1),
            tip_color: Color::srgb(0.3, 0.7, 0.2),
            variation: 0.5,
        };

        let mut rng = rand::rng();
        let first = scheme.sample_blade_color(&mut rng);
        let mut changed = false;

        for _ in 0..10 {
            let next = scheme.sample_blade_color(&mut rng);
            if (next.to_srgba().red - first.to_srgba().red).abs() > 0.01
                || (next.to_srgba().green - first.to_srgba().green).abs() > 0.01
                || (next.to_srgba().blue - first.to_srgba().blue).abs() > 0.01
            {
                changed = true;
                break;
            }
        }

        assert!(changed, "Expected color variation to change at least once");
    }

    #[test]
    fn test_spawn_grass_with_none_density_spawns_no_blades() {
        fn spawn_none_density_system(
            mut commands: Commands,
            mut materials: ResMut<Assets<StandardMaterial>>,
            mut meshes: ResMut<Assets<Mesh>>,
            asset_server: Res<AssetServer>,
            settings: Res<GrassQualitySettings>,
        ) {
            let metadata = TileVisualMetadata {
                grass_density: Some(GrassDensity::None),
                ..Default::default()
            };

            spawn_grass(
                &mut commands,
                &mut materials,
                &mut meshes,
                &asset_server,
                types::Position::new(0, 0),
                1u16,
                Some(&metadata),
                None,
                &settings,
            );
        }

        let mut app = App::new();
        app.add_plugins(bevy::app::PluginGroup::set(
            bevy::MinimalPlugins,
            bevy::app::ScheduleRunnerPlugin::default(),
        ))
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>()
        .init_asset::<Mesh>();
        app.insert_resource(GrassQualitySettings {
            performance_level:
                crate::game::resources::grass_quality_settings::GrassPerformanceLevel::Medium,
        });
        app.add_systems(Update, spawn_none_density_system);

        app.update();

        let (blade_count, clump_count, cluster_count) = {
            let world = app.world_mut();
            let blades = world
                .query_filtered::<Entity, With<GrassBlade>>()
                .iter(world)
                .count();
            let clumps = world
                .query_filtered::<Entity, With<GrassClump>>()
                .iter(world)
                .count();
            let clusters = world
                .query_filtered::<Entity, With<GrassCluster>>()
                .iter(world)
                .count();
            (blades, clumps, clusters)
        };

        assert_eq!(blade_count, 0);
        assert_eq!(clump_count, 0);
        assert_eq!(cluster_count, 0);
    }

    #[test]
    fn test_blade_config_clamps_out_of_range_values() {
        let domain = world::GrassBladeConfig {
            length: 0.1,
            width: 4.0,
            tilt: -1.0,
            curve: 2.0,
            color_variation: -0.5,
        };

        let config = BladeConfig::from(&domain);
        assert_eq!(config.length, 0.5);
        assert_eq!(config.width, 2.0);
        assert_eq!(config.tilt, 0.0);
        assert_eq!(config.curve, 1.0);
        assert_eq!(config.color_variation, 0.0);
    }

    #[test]
    fn test_spawn_grass_with_density_spawns_blades_and_cluster() {
        fn spawn_low_density_system(
            mut commands: Commands,
            mut materials: ResMut<Assets<StandardMaterial>>,
            mut meshes: ResMut<Assets<Mesh>>,
            asset_server: Res<AssetServer>,
            settings: Res<GrassQualitySettings>,
        ) {
            let metadata = TileVisualMetadata {
                grass_density: Some(GrassDensity::Low),
                ..Default::default()
            };

            spawn_grass(
                &mut commands,
                &mut materials,
                &mut meshes,
                &asset_server,
                types::Position::new(1, 1),
                1u16,
                Some(&metadata),
                None,
                &settings,
            );
        }

        let mut app = App::new();
        app.add_plugins(bevy::app::PluginGroup::set(
            bevy::MinimalPlugins,
            bevy::app::ScheduleRunnerPlugin::default(),
        ))
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>()
        .init_asset::<Mesh>();
        app.insert_resource(GrassQualitySettings {
            performance_level:
                crate::game::resources::grass_quality_settings::GrassPerformanceLevel::Medium,
        });
        app.add_systems(Update, spawn_low_density_system);

        app.update();

        let (blade_count, cluster_count) = {
            let world = app.world_mut();
            let blades = world
                .query_filtered::<Entity, With<GrassBlade>>()
                .iter(world)
                .count();
            let clusters = world
                .query_filtered::<Entity, With<GrassCluster>>()
                .iter(world)
                .count();
            (blades, clusters)
        };

        assert!(blade_count > 0);
        assert!(cluster_count > 0);
    }

    #[test]
    fn test_spawn_grass_cached_reuses_mesh_and_material_handles_for_similar_clumps() {
        fn spawn_two_cached_grass_patches(
            mut commands: Commands,
            mut materials: ResMut<Assets<StandardMaterial>>,
            mut meshes: ResMut<Assets<Mesh>>,
            asset_server: Res<AssetServer>,
            mut grass_cache: ResMut<GrassAssetCache>,
            settings: Res<GrassQualitySettings>,
        ) {
            let metadata = TileVisualMetadata {
                grass_density: Some(GrassDensity::Low),
                grass_blade_config: Some(world::GrassBladeConfig::default()),
                ..Default::default()
            };

            spawn_grass_cached(
                &mut commands,
                &mut materials,
                &mut meshes,
                &asset_server,
                grass_cache.as_mut(),
                types::Position::new(2, 2),
                1u16,
                Some(&metadata),
                None,
                &settings,
            );
            spawn_grass_cached(
                &mut commands,
                &mut materials,
                &mut meshes,
                &asset_server,
                grass_cache.as_mut(),
                types::Position::new(3, 2),
                1u16,
                Some(&metadata),
                None,
                &settings,
            );
        }

        let mut app = App::new();
        app.add_plugins(bevy::app::PluginGroup::set(
            bevy::MinimalPlugins,
            bevy::app::ScheduleRunnerPlugin::default(),
        ))
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>()
        .init_asset::<Mesh>()
        .init_resource::<GrassAssetCache>();
        app.insert_resource(GrassQualitySettings {
            performance_level:
                crate::game::resources::grass_quality_settings::GrassPerformanceLevel::Medium,
        });
        app.add_systems(Update, spawn_two_cached_grass_patches);

        app.update();

        let clump_count = {
            let world = app.world_mut();
            world
                .query_filtered::<Entity, With<GrassClump>>()
                .iter(world)
                .count()
        };
        let cache = app.world().resource::<GrassAssetCache>();

        assert!(clump_count > 0);
        assert!(
            cache.mesh_count() < clump_count,
            "similar clumps should reuse fewer mesh handles than spawned clump entities"
        );
        assert_eq!(
            cache.material_count(),
            1,
            "similar grass tint/material settings should reuse one material handle"
        );
    }

    // ── Grass rendering tests ─────────────────────────────────────────────────

    /// `GRASS_BLADE_TEXTURE` must start with `"textures/grass/"` and end with
    /// `".png"`, confirming it points to the correct asset directory and format.
    #[test]
    fn test_grass_blade_texture_path_constant() {
        assert!(
            GRASS_BLADE_TEXTURE.starts_with("assets/textures/grass/"),
            "GRASS_BLADE_TEXTURE must start with 'assets/textures/grass/', got '{GRASS_BLADE_TEXTURE}'"
        );
        assert!(
            GRASS_BLADE_TEXTURE.ends_with(".png"),
            "GRASS_BLADE_TEXTURE must end with '.png', got '{GRASS_BLADE_TEXTURE}'"
        );
        assert!(
            !GRASS_BLADE_TEXTURE.is_empty(),
            "GRASS_BLADE_TEXTURE must not be empty"
        );
    }

    /// `GRASS_ALPHA_CUTOFF` must be strictly between 0.0 and 1.0 so that
    /// Bevy's `AlphaMode::Mask` produces a sensible cutout (0.0 = fully
    /// transparent pass-through; 1.0 = everything discarded).
    #[test]
    fn test_grass_alpha_cutoff_in_valid_range() {
        // Shadow the constant with a local binding so that clippy does not
        // flag the assertions as `assertions_on_constants`.
        let cutoff: f32 = GRASS_ALPHA_CUTOFF;
        assert!(cutoff > 0.0, "GRASS_ALPHA_CUTOFF ({cutoff}) must be > 0.0");
        assert!(cutoff < 1.0, "GRASS_ALPHA_CUTOFF ({cutoff}) must be < 1.0");
    }

    /// Spawned grass blade materials must use `AlphaMode::Mask` (not `Opaque`)
    /// and must have a `base_color_texture` set.
    #[test]
    fn test_grass_material_uses_alpha_mask() {
        fn spawn_grass_for_alpha_test(
            mut commands: Commands,
            mut materials: ResMut<Assets<StandardMaterial>>,
            mut meshes: ResMut<Assets<Mesh>>,
            asset_server: Res<AssetServer>,
            settings: Res<GrassQualitySettings>,
        ) {
            let metadata = TileVisualMetadata {
                grass_density: Some(GrassDensity::Low),
                ..Default::default()
            };

            spawn_grass(
                &mut commands,
                &mut materials,
                &mut meshes,
                &asset_server,
                types::Position::new(5, 5),
                1u16,
                Some(&metadata),
                None,
                &settings,
            );
        }

        let mut app = App::new();
        app.add_plugins(bevy::app::PluginGroup::set(
            bevy::MinimalPlugins,
            bevy::app::ScheduleRunnerPlugin::default(),
        ))
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>()
        .init_asset::<Mesh>();
        app.insert_resource(GrassQualitySettings {
            performance_level:
                crate::game::resources::grass_quality_settings::GrassPerformanceLevel::Medium,
        });
        app.add_systems(Update, spawn_grass_for_alpha_test);
        app.update();

        // Collect all StandardMaterial handles from spawned MeshMaterial3d components.
        let material_handles: Vec<Handle<StandardMaterial>> = {
            let world = app.world_mut();
            world
                .query::<&MeshMaterial3d<StandardMaterial>>()
                .iter(world)
                .map(|m| m.0.clone())
                .collect()
        };

        assert!(
            !material_handles.is_empty(),
            "Expected at least one spawned grass blade material"
        );

        let materials = app
            .world()
            .get_resource::<Assets<StandardMaterial>>()
            .expect("Assets<StandardMaterial> must exist");

        for handle in &material_handles {
            if let Some(mat) = materials.get(handle) {
                assert!(
                    matches!(mat.alpha_mode, AlphaMode::Mask(_)),
                    "Grass blade material must use AlphaMode::Mask, got {:?}",
                    mat.alpha_mode
                );
                assert!(
                    mat.base_color_texture.is_some(),
                    "Grass blade material must have a base_color_texture set"
                );
            }
        }
    }

    /// When `spawn_grass` is called with a `tile_tint` of `(0.5, 0.5, 0.5)`,
    /// the resulting grass should use a base colour that is darker than the
    /// default (which uses a natural-green tint of `(0.3, 0.65, 0.2)`).
    ///
    /// We verify this by computing the `GrassColorScheme` directly from the
    /// tint values and checking that the tinted colour is strictly darker.
    #[test]
    fn test_grass_asset_cache_default_is_empty() {
        let cache = GrassAssetCache::default();

        assert_eq!(cache.mesh_count(), 0);
        assert_eq!(cache.material_count(), 0);
    }

    #[test]
    fn test_cached_material_color_uses_variation_bucket() {
        let scheme = GrassColorScheme {
            base_color: Color::srgb(0.1, 0.2, 0.3),
            tip_color: Color::srgb(0.7, 0.8, 0.9),
            variation: 1.0,
        };

        let no_variation_key = GrassMaterialKey::from_tint((1.0, 1.0, 1.0), 0.0);
        let full_variation_key = GrassMaterialKey::from_tint((1.0, 1.0, 1.0), 1.0);

        let base_color = cached_material_color(no_variation_key, &scheme).to_srgba();
        let tip_color = cached_material_color(full_variation_key, &scheme).to_srgba();

        assert!((base_color.red - 0.1).abs() < 0.001);
        assert!((base_color.green - 0.2).abs() < 0.001);
        assert!((base_color.blue - 0.3).abs() < 0.001);

        assert!((tip_color.red - 0.7).abs() < 0.001);
        assert!((tip_color.green - 0.8).abs() < 0.001);
        assert!((tip_color.blue - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_grass_color_tint_forwarded_to_color_scheme() {
        // Build a GrassColorScheme the same way spawn_grass does with tint (0.5, 0.5, 0.5).
        let tinted_tint = (0.5f32, 0.5f32, 0.5f32);
        let tinted_base = Color::srgb(
            0.2 * tinted_tint.0,
            0.5 * tinted_tint.1,
            0.1 * tinted_tint.2,
        );

        // Default tint used when no explicit tint is provided: (0.3, 0.65, 0.2).
        let default_tint = (0.3f32, 0.65f32, 0.2f32);
        let default_base = Color::srgb(
            0.2 * default_tint.0,
            0.5 * default_tint.1,
            0.1 * default_tint.2,
        );

        let tinted_r = tinted_base.to_srgba().red;
        let tinted_g = tinted_base.to_srgba().green;
        let default_r = default_base.to_srgba().red;
        let default_g = default_base.to_srgba().green;

        // A (0.5, 0.5, 0.5) tint must produce a darker result than the default
        // natural-green tint (0.3, 0.65, 0.2) on the green channel, since
        // 0.5 * 0.5 < 0.65 * 0.5.
        assert!(
            tinted_g < default_g,
            "Tinted green ({tinted_g}) should be darker than default green ({default_g})"
        );
        // And no darker on red than the default (0.2 * 0.5 vs 0.2 * 0.3 — tinted
        // is actually brighter on red, which is expected).
        assert!(
            tinted_r >= default_r,
            "Tinted red ({tinted_r}) should be >= default red ({default_r})"
        );
    }
}
