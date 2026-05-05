// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Deterministic vegetation placement helpers.
//!
//! This module builds render-layer placement plans for trees, shrubs, and grass
//! exclusion zones. The helpers keep vegetation composition deterministic by
//! deriving every anchor from map ID, tile position, metadata, and stable salts.

use bevy::prelude::Vec2;

use crate::domain::types::{MapId, Position};
use crate::domain::world::{
    GrassDensity, TerrainType, Tile, TileVisualMetadata, TreeType, WallType,
};

/// Offset from integer tile coordinates to the visual tile center.
pub const TILE_CENTER_OFFSET: f32 = 0.5;

/// Stable salt used for tile-level vegetation plans.
pub const TILE_PLAN_SALT: u64 = 0xA17A_4E5E_771A_7101;

/// Stable salt used for shrub anchor generation.
pub const SHRUB_ANCHOR_SALT: u64 = 0x5A7B_5A7B_51E7_0001;

/// Stable salt used for grass exclusion generation.
pub const GRASS_EXCLUSION_SALT: u64 = 0x67A5_5EED_0000_0001;

/// Safety margin between a tree trunk footprint and shrub stems.
pub const SHRUB_TRUNK_SAFETY_MARGIN: f32 = 0.04;

/// Safety margin used when grass clumps avoid tree and shrub stems.
pub const GRASS_STEM_SAFETY_MARGIN: f32 = 0.07;

/// Default low shrub footprint radius in world units.
pub const DEFAULT_SHRUB_RADIUS: f32 = 0.09;

/// Default tree fallback trunk exclusion radius in world units.
pub const DEFAULT_TREE_RADIUS: f32 = 0.24;

/// Describes the type of vegetation represented by an anchor.
///
/// # Examples
///
/// ```
/// use antares::game::systems::vegetation_placement::VegetationKind;
///
/// assert_eq!(VegetationKind::Tree.label(), "tree");
/// assert_eq!(VegetationKind::Shrub.label(), "shrub");
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VegetationKind {
    /// A full-size tree or tree-like variant.
    Tree,
    /// A shrub, bush, or low clump of woody vegetation.
    Shrub,
}

impl VegetationKind {
    /// Returns a lowercase label for diagnostics and tests.
    pub fn label(self) -> &'static str {
        match self {
            Self::Tree => "tree",
            Self::Shrub => "shrub",
        }
    }
}

/// A deterministic vegetation anchor inside one map tile.
///
/// The `center` is in world X/Z coordinates, where `center.x` maps to Bevy X
/// and `center.y` maps to Bevy Z.
///
/// # Examples
///
/// ```
/// use antares::game::systems::vegetation_placement::{VegetationAnchor, VegetationKind};
/// use bevy::prelude::Vec2;
///
/// let anchor = VegetationAnchor::new(VegetationKind::Tree, Vec2::new(1.5, 2.5), 0.3, 0.3, 0.0, 0.0);
/// assert_eq!(anchor.kind, VegetationKind::Tree);
/// assert_eq!(anchor.max_radius(), 0.3);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VegetationAnchor {
    /// The vegetation category represented by this anchor.
    pub kind: VegetationKind,
    /// World-space X/Z center.
    pub center: Vec2,
    /// Footprint radius on the X axis.
    pub radius_x: f32,
    /// Footprint radius on the Z axis.
    pub radius_z: f32,
    /// Vertical offset inherited from tile metadata.
    pub y_offset: f32,
    /// Rotation around the Y axis in radians.
    pub rotation_y_radians: f32,
}

impl VegetationAnchor {
    /// Creates a new deterministic vegetation anchor.
    pub fn new(
        kind: VegetationKind,
        center: Vec2,
        radius_x: f32,
        radius_z: f32,
        y_offset: f32,
        rotation_y_radians: f32,
    ) -> Self {
        Self {
            kind,
            center,
            radius_x: radius_x.max(0.0),
            radius_z: radius_z.max(0.0),
            y_offset,
            rotation_y_radians,
        }
    }

    /// Returns the largest footprint radius.
    pub fn max_radius(self) -> f32 {
        self.radius_x.max(self.radius_z)
    }

    /// Returns the distance between this anchor and another anchor.
    pub fn distance_to(self, other: Self) -> f32 {
        self.center.distance(other.center)
    }
}

/// An elliptical grass exclusion zone.
///
/// Grass clumps should not be spawned inside these zones because they represent
/// trunk, shrub stem, or prop footprints.
///
/// # Examples
///
/// ```
/// use antares::game::systems::vegetation_placement::VegetationExclusionZone;
/// use bevy::prelude::Vec2;
///
/// let zone = VegetationExclusionZone::new(Vec2::new(0.5, 0.5), 0.25, 0.25);
/// assert!(zone.contains(Vec2::new(0.5, 0.5)));
/// assert!(!zone.contains(Vec2::new(0.9, 0.5)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VegetationExclusionZone {
    /// World-space X/Z center.
    pub center: Vec2,
    /// Exclusion radius on the X axis.
    pub radius_x: f32,
    /// Exclusion radius on the Z axis.
    pub radius_z: f32,
}

impl VegetationExclusionZone {
    /// Creates a new grass exclusion zone.
    pub fn new(center: Vec2, radius_x: f32, radius_z: f32) -> Self {
        Self {
            center,
            radius_x: radius_x.max(0.0),
            radius_z: radius_z.max(0.0),
        }
    }

    /// Creates an exclusion zone from a vegetation anchor plus a safety margin.
    pub fn from_anchor(anchor: VegetationAnchor, safety_margin: f32) -> Self {
        Self::new(
            anchor.center,
            anchor.radius_x + safety_margin.max(0.0),
            anchor.radius_z + safety_margin.max(0.0),
        )
    }

    /// Returns true when `point` is inside this elliptical zone.
    pub fn contains(self, point: Vec2) -> bool {
        if self.radius_x <= f32::EPSILON || self.radius_z <= f32::EPSILON {
            return false;
        }

        let delta = point - self.center;
        let normalized_x = delta.x / self.radius_x;
        let normalized_z = delta.y / self.radius_z;

        normalized_x * normalized_x + normalized_z * normalized_z <= 1.0
    }

    /// Returns the largest exclusion radius.
    pub fn max_radius(self) -> f32 {
        self.radius_x.max(self.radius_z)
    }
}

/// A deterministic vegetation composition plan for one tile.
///
/// # Examples
///
/// ```
/// use antares::domain::types::Position;
/// use antares::domain::world::{Tile, TerrainType, WallType};
/// use antares::game::systems::vegetation_placement::tile_vegetation_plan;
///
/// let tile = Tile::new(0, 0, TerrainType::Forest, WallType::None);
/// let plan = tile_vegetation_plan(&tile, 1, Position::new(0, 0));
/// assert!(plan.tree_anchor.is_some());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct TileVegetationPlan {
    /// Stable plan seed derived from map ID and tile position.
    pub seed: u64,
    /// Main tree anchor, if this tile should contain a full-size tree.
    pub tree_anchor: Option<VegetationAnchor>,
    /// Shrub anchors, already placed outside any tree exclusion radius.
    pub shrub_anchors: Vec<VegetationAnchor>,
    /// Grass clump exclusion zones generated from tree and shrub footprints.
    pub grass_exclusion_zones: Vec<VegetationExclusionZone>,
    /// Grass coverage multiplier derived from metadata.
    pub grass_coverage_multiplier: f32,
    /// Whether the tree was inferred from forest terrain rather than explicit metadata.
    pub uses_default_forest_tree: bool,
    /// Explicit tree type from metadata, if present.
    pub explicit_tree_type: Option<TreeType>,
}

impl TileVegetationPlan {
    /// Returns true when a grass clump can occupy the given world X/Z point.
    pub fn allows_grass_clump_at(&self, point: Vec2) -> bool {
        !self
            .grass_exclusion_zones
            .iter()
            .any(|zone| zone.contains(point))
    }

    /// Returns all vegetation anchors in this plan.
    pub fn vegetation_anchors(&self) -> Vec<VegetationAnchor> {
        let mut anchors = Vec::with_capacity(self.shrub_anchors.len() + 1);
        if let Some(tree) = self.tree_anchor {
            anchors.push(tree);
        }
        anchors.extend(self.shrub_anchors.iter().copied());
        anchors
    }
}

/// Creates a stable deterministic seed from map ID, tile position, and salt.
///
/// # Examples
///
/// ```
/// use antares::domain::types::Position;
/// use antares::game::systems::vegetation_placement::vegetation_seed;
///
/// let a = vegetation_seed(1, Position::new(2, 3), 9);
/// let b = vegetation_seed(1, Position::new(2, 3), 9);
/// assert_eq!(a, b);
/// ```
pub fn vegetation_seed(map_id: MapId, position: Position, salt: u64) -> u64 {
    let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
    seed ^= u64::from(map_id).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    seed = mix_u64(seed ^ (position.x as i64 as u64).rotate_left(17));
    seed = mix_u64(seed ^ (position.y as i64 as u64).rotate_left(41));
    mix_u64(seed ^ salt)
}

/// Returns the world-space center of a tile.
pub fn tile_center(position: Position) -> Vec2 {
    Vec2::new(
        position.x as f32 + TILE_CENTER_OFFSET,
        position.y as f32 + TILE_CENTER_OFFSET,
    )
}

/// Returns the main tree anchor for a tile.
///
/// The current metadata model does not expose separate vegetation X/Z offsets,
/// so the main tree remains near the tile center while respecting scale,
/// footprint width/depth overrides, vertical offset, and rotation metadata.
///
/// # Examples
///
/// ```
/// use antares::domain::types::Position;
/// use antares::domain::world::TileVisualMetadata;
/// use antares::game::systems::vegetation_placement::tree_anchor_for_tile;
///
/// let metadata = TileVisualMetadata { scale: Some(1.5), ..Default::default() };
/// let anchor = tree_anchor_for_tile(Position::new(4, 5), &metadata);
/// assert!(anchor.max_radius() > 0.3);
/// ```
pub fn tree_anchor_for_tile(position: Position, metadata: &TileVisualMetadata) -> VegetationAnchor {
    let tree_type = metadata.tree_type.unwrap_or(TreeType::Oak);
    let (radius_x, radius_z) = footprint_radii_for_tree(tree_type, metadata);
    VegetationAnchor::new(
        VegetationKind::Tree,
        tile_center(position),
        radius_x,
        radius_z,
        metadata.effective_y_offset(),
        metadata.rotation_y_radians(),
    )
}

/// Returns deterministic shrub anchors for a tile.
///
/// When `tree_radius` is positive, generated shrubs are placed outside the tree
/// trunk exclusion radius plus a safety margin. When `tree_radius` is zero,
/// the first shrub can occupy the tile center for shrub-only vegetation.
///
/// # Examples
///
/// ```
/// use antares::domain::types::Position;
/// use antares::domain::world::TileVisualMetadata;
/// use antares::game::systems::vegetation_placement::shrub_anchors_for_tile;
///
/// let shrubs = shrub_anchors_for_tile(Position::new(0, 0), 0.35, &TileVisualMetadata::default());
/// assert!(!shrubs.is_empty());
/// ```
pub fn shrub_anchors_for_tile(
    position: Position,
    tree_radius: f32,
    metadata: &TileVisualMetadata,
) -> Vec<VegetationAnchor> {
    shrub_anchors_for_tile_with_map(0, position, tree_radius, metadata)
}

fn shrub_anchors_for_tile_with_map(
    map_id: MapId,
    position: Position,
    tree_radius: f32,
    metadata: &TileVisualMetadata,
) -> Vec<VegetationAnchor> {
    let count = shrub_count_for_metadata(metadata);
    if count == 0 {
        return Vec::new();
    }

    let shrub_radius = shrub_radius_for_metadata(metadata);
    let center = tile_center(position);
    let rotation = metadata.rotation_y_radians();
    let y_offset = metadata.effective_y_offset();

    let mut anchors = Vec::with_capacity(count);

    for index in 0..count {
        let offset = if tree_radius <= f32::EPSILON && index == 0 {
            Vec2::ZERO
        } else {
            let seed = vegetation_seed(map_id, position, SHRUB_ANCHOR_SALT + index as u64);
            let angle = unit_from_seed(seed, 0) * std::f32::consts::TAU;
            let minimum_radius = if tree_radius > 0.0 {
                tree_radius + shrub_radius + SHRUB_TRUNK_SAFETY_MARGIN
            } else {
                DEFAULT_SHRUB_RADIUS + shrub_radius * index as f32
            };
            let available_radius = (0.46 - minimum_radius).max(0.0);
            let distance = minimum_radius + available_radius * unit_from_seed(seed, 1);
            rotate_vec2(
                Vec2::new(angle.cos() * distance, angle.sin() * distance),
                rotation,
            )
        };

        anchors.push(VegetationAnchor::new(
            VegetationKind::Shrub,
            center + offset,
            shrub_radius,
            shrub_radius,
            y_offset,
            rotation,
        ));
    }

    anchors
}

/// Returns grass exclusion zones for the provided vegetation anchors.
///
/// # Examples
///
/// ```
/// use antares::game::systems::vegetation_placement::{
///     grass_exclusion_zones, VegetationAnchor, VegetationKind,
/// };
/// use bevy::prelude::Vec2;
///
/// let anchors = vec![VegetationAnchor::new(VegetationKind::Tree, Vec2::new(0.5, 0.5), 0.3, 0.3, 0.0, 0.0)];
/// let zones = grass_exclusion_zones(&anchors);
/// assert_eq!(zones.len(), 1);
/// ```
pub fn grass_exclusion_zones(vegetation: &[VegetationAnchor]) -> Vec<VegetationExclusionZone> {
    vegetation
        .iter()
        .copied()
        .map(|anchor| {
            let safety = match anchor.kind {
                VegetationKind::Tree => GRASS_STEM_SAFETY_MARGIN,
                VegetationKind::Shrub => GRASS_STEM_SAFETY_MARGIN * 0.5,
            };
            VegetationExclusionZone::from_anchor(anchor, safety)
        })
        .collect()
}

/// Creates a deterministic vegetation plan for one tile.
///
/// # Examples
///
/// ```
/// use antares::domain::types::Position;
/// use antares::domain::world::{Tile, TerrainType, WallType};
/// use antares::game::systems::vegetation_placement::tile_vegetation_plan;
///
/// let tile = Tile::new(0, 0, TerrainType::Grass, WallType::None);
/// let first = tile_vegetation_plan(&tile, 1, Position::new(0, 0));
/// let second = tile_vegetation_plan(&tile, 1, Position::new(0, 0));
/// assert_eq!(first, second);
/// ```
pub fn tile_vegetation_plan(tile: &Tile, map_id: MapId, position: Position) -> TileVegetationPlan {
    let metadata = &tile.visual;
    let seed = vegetation_seed(map_id, position, TILE_PLAN_SALT);
    let explicit_tree_type = metadata.tree_type;
    let supports_cover = supports_vegetation_cover(tile);
    let uses_default_forest_tree =
        explicit_tree_type.is_none() && supports_cover && tile.terrain == TerrainType::Forest;

    let tree_anchor = match explicit_tree_type {
        Some(TreeType::Shrub) => None,
        Some(_) if supports_cover => Some(tree_anchor_for_tile(position, metadata)),
        None if uses_default_forest_tree => Some(tree_anchor_for_tile(position, metadata)),
        _ => None,
    };

    let shrub_anchors = if explicit_tree_type == Some(TreeType::Shrub) {
        shrub_anchors_for_tile_with_map(map_id, position, 0.0, metadata)
    } else if supports_cover && should_plan_understory_shrubs(tile.terrain, metadata) {
        let tree_radius = tree_anchor.map(VegetationAnchor::max_radius).unwrap_or(0.0);
        shrub_anchors_for_tile_with_map(map_id, position, tree_radius, metadata)
    } else {
        Vec::new()
    };

    let vegetation = {
        let mut anchors = Vec::with_capacity(shrub_anchors.len() + 1);
        if let Some(tree) = tree_anchor {
            anchors.push(tree);
        }
        anchors.extend(shrub_anchors.iter().copied());
        anchors
    };

    let grass_exclusion_zones = grass_exclusion_zones(&vegetation);

    TileVegetationPlan {
        seed,
        tree_anchor,
        shrub_anchors,
        grass_exclusion_zones,
        grass_coverage_multiplier: if supports_cover {
            grass_coverage_multiplier(metadata)
        } else {
            0.0
        },
        uses_default_forest_tree,
        explicit_tree_type,
    }
}

fn supports_vegetation_cover(tile: &Tile) -> bool {
    !tile.blocked
        && tile.wall_type == WallType::None
        && matches!(tile.terrain, TerrainType::Forest | TerrainType::Grass)
}

fn should_plan_understory_shrubs(terrain: TerrainType, metadata: &TileVisualMetadata) -> bool {
    terrain == TerrainType::Forest && metadata.foliage_density().clamp(0.0, 2.0) > 0.0
}

fn shrub_count_for_metadata(metadata: &TileVisualMetadata) -> usize {
    let density = metadata.foliage_density().clamp(0.0, 2.0);
    if density <= 0.0 {
        0
    } else if density < 0.75 {
        1
    } else if density < 1.5 {
        2
    } else {
        3
    }
}

fn grass_coverage_multiplier(metadata: &TileVisualMetadata) -> f32 {
    let density = metadata.foliage_density().clamp(0.0, 2.0);
    match metadata.grass_density {
        Some(GrassDensity::None) => 0.0,
        _ => density,
    }
}

fn footprint_radii_for_tree(tree_type: TreeType, metadata: &TileVisualMetadata) -> (f32, f32) {
    let base_radius = trunk_radius_for_tree_type(tree_type);
    let scale = metadata.effective_scale().clamp(0.1, 4.0);

    let radius_x = metadata
        .width_x
        .map(|width| (width.abs() * scale * 0.5).max(base_radius * scale))
        .unwrap_or(base_radius * scale);

    let radius_z = metadata
        .width_z
        .map(|width| (width.abs() * scale * 0.5).max(base_radius * scale))
        .unwrap_or(base_radius * scale);

    (radius_x, radius_z)
}

fn shrub_radius_for_metadata(metadata: &TileVisualMetadata) -> f32 {
    let scale = metadata.effective_scale().clamp(0.1, 4.0);
    let base = DEFAULT_SHRUB_RADIUS * scale;

    let width_radius = metadata
        .width_x
        .zip(metadata.width_z)
        .map(|(x, z)| (x.abs().max(z.abs()) * scale * 0.25).max(base));

    width_radius.unwrap_or(base)
}

fn trunk_radius_for_tree_type(tree_type: TreeType) -> f32 {
    match tree_type {
        TreeType::Oak => DEFAULT_TREE_RADIUS,
        TreeType::Pine => 0.18,
        TreeType::Dead => DEFAULT_TREE_RADIUS,
        TreeType::Palm => 0.16,
        TreeType::Willow => 0.28,
        TreeType::Birch => 0.14,
        TreeType::Shrub => 0.05,
    }
}

fn rotate_vec2(value: Vec2, radians: f32) -> Vec2 {
    let (sin, cos) = radians.sin_cos();
    Vec2::new(value.x * cos - value.y * sin, value.x * sin + value.y * cos)
}

fn unit_from_seed(seed: u64, stream: u64) -> f32 {
    let mixed = mix_u64(seed ^ stream.wrapping_mul(0xD6E8_FEB8_6659_FD93));
    let value = (mixed >> 40) as u32;
    value as f32 / 16_777_215.0
}

fn mix_u64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::world::WallType;

    fn forest_tile() -> Tile {
        Tile::new(0, 0, TerrainType::Forest, WallType::None)
    }

    #[test]
    fn test_vegetation_seed_same_tile_same_seed() {
        let position = Position::new(4, 7);

        let first = vegetation_seed(2, position, 11);
        let second = vegetation_seed(2, position, 11);

        assert_eq!(first, second);
    }

    #[test]
    fn test_vegetation_seed_changes_with_tile() {
        let first = vegetation_seed(2, Position::new(4, 7), 11);
        let second = vegetation_seed(2, Position::new(5, 7), 11);

        assert_ne!(first, second);
    }

    #[test]
    fn test_tree_anchor_respects_metadata_scale() {
        let small = TileVisualMetadata {
            scale: Some(1.0),
            ..Default::default()
        };
        let large = TileVisualMetadata {
            scale: Some(2.0),
            ..Default::default()
        };

        let small_anchor = tree_anchor_for_tile(Position::new(0, 0), &small);
        let large_anchor = tree_anchor_for_tile(Position::new(0, 0), &large);

        assert!(large_anchor.max_radius() > small_anchor.max_radius());
    }

    #[test]
    fn test_tree_anchor_respects_width_overrides() {
        let metadata = TileVisualMetadata {
            width_x: Some(1.2),
            width_z: Some(0.8),
            ..Default::default()
        };

        let anchor = tree_anchor_for_tile(Position::new(0, 0), &metadata);

        assert!(anchor.radius_x > anchor.radius_z);
    }

    #[test]
    fn test_tree_anchor_respects_y_offset_and_rotation() {
        let metadata = TileVisualMetadata {
            y_offset: Some(0.25),
            rotation_y: Some(90.0),
            ..Default::default()
        };

        let anchor = tree_anchor_for_tile(Position::new(0, 0), &metadata);

        assert_eq!(anchor.y_offset, 0.25);
        assert!((anchor.rotation_y_radians - std::f32::consts::FRAC_PI_2).abs() < 0.001);
    }

    #[test]
    fn test_shrub_anchors_avoid_tree_trunk_exclusion_radius() {
        let metadata = TileVisualMetadata::default();
        let tree_radius = 0.35;
        let shrubs = shrub_anchors_for_tile(Position::new(0, 0), tree_radius, &metadata);
        let tree_center = tile_center(Position::new(0, 0));

        assert!(!shrubs.is_empty());
        for shrub in shrubs {
            let minimum = tree_radius + shrub.max_radius() + SHRUB_TRUNK_SAFETY_MARGIN - 0.001;
            assert!(
                shrub.center.distance(tree_center) >= minimum,
                "shrub at {:?} should be outside tree exclusion radius {minimum}",
                shrub.center
            );
        }
    }

    #[test]
    fn test_shrub_only_tile_can_use_center_anchor() {
        let metadata = TileVisualMetadata {
            tree_type: Some(TreeType::Shrub),
            ..Default::default()
        };

        let shrubs = shrub_anchors_for_tile(Position::new(2, 3), 0.0, &metadata);

        assert!(!shrubs.is_empty());
        assert_eq!(shrubs[0].center, tile_center(Position::new(2, 3)));
    }

    #[test]
    fn test_grass_exclusion_zone_blocks_trunk_center() {
        let metadata = TileVisualMetadata::default();
        let tree = tree_anchor_for_tile(Position::new(0, 0), &metadata);
        let zones = grass_exclusion_zones(&[tree]);

        assert_eq!(zones.len(), 1);
        assert!(zones[0].contains(tree.center));
    }

    #[test]
    fn test_tile_vegetation_plan_same_tile_is_deterministic() {
        let tile = forest_tile();
        let position = Position::new(1, 2);

        let first = tile_vegetation_plan(&tile, 5, position);
        let second = tile_vegetation_plan(&tile, 5, position);

        assert_eq!(first, second);
    }

    #[test]
    fn test_tile_vegetation_plan_shrub_anchors_vary_by_map_id() {
        let tile = forest_tile();
        let position = Position::new(1, 2);

        let first = tile_vegetation_plan(&tile, 5, position);
        let second = tile_vegetation_plan(&tile, 6, position);

        assert_ne!(first.shrub_anchors, second.shrub_anchors);
    }

    #[test]
    fn test_tile_vegetation_plan_forest_default_tree_and_shrubs_do_not_overlap() {
        let tile = forest_tile();
        let plan = tile_vegetation_plan(&tile, 1, Position::new(0, 0));
        let tree = plan
            .tree_anchor
            .expect("forest tile should have default tree anchor");

        assert!(plan.uses_default_forest_tree);
        assert!(!plan.shrub_anchors.is_empty());

        for shrub in &plan.shrub_anchors {
            let minimum =
                tree.max_radius() + shrub.max_radius() + SHRUB_TRUNK_SAFETY_MARGIN - 0.001;
            assert!(
                tree.distance_to(*shrub) >= minimum,
                "forest shrub should not overlap default tree trunk"
            );
        }
    }

    #[test]
    fn test_tile_vegetation_plan_explicit_shrub_has_no_full_size_default_tree() {
        let mut tile = forest_tile();
        tile.visual.tree_type = Some(TreeType::Shrub);

        let plan = tile_vegetation_plan(&tile, 1, Position::new(0, 0));

        assert!(plan.tree_anchor.is_none());
        assert_eq!(plan.explicit_tree_type, Some(TreeType::Shrub));
        assert!(!plan.shrub_anchors.is_empty());
        assert!(!plan.uses_default_forest_tree);
    }

    #[test]
    fn test_tile_vegetation_plan_blocked_tile_has_no_vegetation() {
        let mut tile = forest_tile();
        tile.blocked = true;

        let plan = tile_vegetation_plan(&tile, 1, Position::new(0, 0));

        assert!(plan.tree_anchor.is_none());
        assert!(plan.shrub_anchors.is_empty());
        assert!(plan.grass_exclusion_zones.is_empty());
        assert_eq!(plan.grass_coverage_multiplier, 0.0);
        assert!(!plan.uses_default_forest_tree);
    }

    #[test]
    fn test_tile_vegetation_plan_wall_tile_has_no_vegetation() {
        let mut tile = forest_tile();
        tile.wall_type = WallType::Normal;

        let plan = tile_vegetation_plan(&tile, 1, Position::new(0, 0));

        assert!(plan.tree_anchor.is_none());
        assert!(plan.shrub_anchors.is_empty());
        assert!(plan.grass_exclusion_zones.is_empty());
        assert_eq!(plan.grass_coverage_multiplier, 0.0);
        assert!(!plan.uses_default_forest_tree);
    }

    #[test]
    fn test_tile_vegetation_plan_grass_clumps_avoid_trunk_exclusion_zone() {
        let tile = forest_tile();
        let plan = tile_vegetation_plan(&tile, 1, Position::new(0, 0));
        let tree = plan
            .tree_anchor
            .expect("forest tile should have default tree anchor");

        assert!(!plan.allows_grass_clump_at(tree.center));
    }

    #[test]
    fn test_tile_vegetation_plan_allows_grass_outside_exclusion_zones() {
        let tile = forest_tile();
        let plan = tile_vegetation_plan(&tile, 1, Position::new(0, 0));
        let outside = tile_center(Position::new(0, 0)) + Vec2::new(0.49, 0.49);

        assert!(plan.allows_grass_clump_at(outside));
    }

    #[test]
    fn test_foliage_density_controls_shrub_count() {
        let sparse = TileVisualMetadata {
            foliage_density: Some(0.5),
            ..Default::default()
        };
        let dense = TileVisualMetadata {
            foliage_density: Some(2.0),
            ..Default::default()
        };

        let sparse_count = shrub_anchors_for_tile(Position::new(0, 0), 0.35, &sparse).len();
        let dense_count = shrub_anchors_for_tile(Position::new(0, 0), 0.35, &dense).len();

        assert!(dense_count > sparse_count);
    }

    #[test]
    fn test_grass_density_none_sets_zero_grass_coverage() {
        let mut tile = forest_tile();
        tile.visual.grass_density = Some(GrassDensity::None);

        let plan = tile_vegetation_plan(&tile, 1, Position::new(0, 0));

        assert_eq!(plan.grass_coverage_multiplier, 0.0);
    }
}
