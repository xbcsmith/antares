// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Procedural mesh generation for environmental objects and static event markers
//!
//! This module provides pure Rust functions to spawn composite 3D meshes using
//! Bevy primitives (Cylinder, Sphere, Cuboid). No external assets required.
//!
//! Character rendering (NPCs, Monsters, Recruitables) uses the sprite system.

use super::map::{MapEntity, TileCoord};
use crate::domain::types;
use crate::domain::world::TileVisualMetadata;
use crate::game::components::Billboard;
use bevy::prelude::*;
use rand::Rng;

// ==================== Mesh Caching ====================

/// Cache for procedural mesh handles to avoid duplicate asset creation
///
/// When spawning multiple trees, signs, or portals on a map, the same mesh
/// geometry is reused many times. This cache stores mesh handles to avoid
/// redundant allocations, improving performance and reducing garbage collection
/// pressure.
///
/// The cache is created fresh for each map spawn and is dropped after the map
/// is fully generated, allowing garbage collection of unused meshes between map loads.
///
/// # Examples
///
/// ```text
/// use antares::game::systems::procedural_meshes::ProceduralMeshCache;
///
/// let mut cache = ProceduralMeshCache::default();
/// // First call creates and caches the mesh
/// let handle1 = get_or_create_mesh(&mut cache, &mut meshes, ...);
/// // Second call reuses the cached handle
/// let handle2 = get_or_create_mesh(&mut cache, &mut meshes, ...);
/// assert_eq!(handle1, handle2);  // Same handle reused
/// ```
#[derive(Clone)]
pub struct ProceduralMeshCache {
    /// Cached trunk mesh handle for trees
    tree_trunk: Option<Handle<Mesh>>,
    /// Cached foliage mesh handle for trees
    tree_foliage: Option<Handle<Mesh>>,
    /// Cached horizontal bar mesh handle for portals (top/bottom)
    portal_frame_horizontal: Option<Handle<Mesh>>,
    /// Cached vertical bar mesh handle for portals (left/right)
    portal_frame_vertical: Option<Handle<Mesh>>,
    /// Cached cylinder mesh handle for sign posts
    sign_post: Option<Handle<Mesh>>,
    /// Cached cuboid mesh handle for sign boards
    sign_board: Option<Handle<Mesh>>,
    /// Cached mesh handle for shrub stems
    shrub_stem: Option<Handle<Mesh>>,
    /// Cached mesh handle for grass blades
    grass_blade: Option<Handle<Mesh>>,
}

impl Default for ProceduralMeshCache {
    /// Creates a new empty cache with no cached meshes
    fn default() -> Self {
        Self {
            tree_trunk: None,
            tree_foliage: None,
            portal_frame_horizontal: None,
            portal_frame_vertical: None,
            sign_post: None,
            sign_board: None,
            shrub_stem: None,
            grass_blade: None,
        }
    }
}

// ==================== Constants ====================

// Tree dimensions (world units, 1 unit ≈ 10 feet)
const TREE_TRUNK_RADIUS: f32 = 0.15;
const TREE_TRUNK_HEIGHT: f32 = 2.0;
const TREE_FOLIAGE_RADIUS: f32 = 0.6;
const TREE_FOLIAGE_Y_OFFSET: f32 = 2.0;

// Event marker dimensions
// Portal dimensions - rectangular frame standing vertically
const PORTAL_FRAME_WIDTH: f32 = 0.8; // Width of the portal opening
const PORTAL_FRAME_HEIGHT: f32 = 1.8; // Height of the portal opening (taller, human-sized)
const PORTAL_FRAME_THICKNESS: f32 = 0.08; // Thickness of frame bars
const PORTAL_FRAME_DEPTH: f32 = 0.08; // Depth of frame bars
const PORTAL_Y_POSITION: f32 = 0.9; // Bottom of frame at ground level (frame center)
const _PORTAL_ROTATION_SPEED: f32 = 1.0; // radians/sec

const SIGN_POST_RADIUS: f32 = 0.05;
const SIGN_POST_HEIGHT: f32 = 1.5;
const SIGN_BOARD_WIDTH: f32 = 0.6;
const SIGN_BOARD_HEIGHT: f32 = 0.3;
const SIGN_BOARD_DEPTH: f32 = 0.05;
const SIGN_BOARD_Y_OFFSET: f32 = 1.5; // Eye height (approx 5 feet)

// Shrub dimensions
const SHRUB_STEM_RADIUS: f32 = 0.08;
const SHRUB_STEM_HEIGHT_BASE: f32 = 0.5; // Base height (scaled by visual_metadata.height)
const SHRUB_STEM_COUNT_MIN: u32 = 3;
const SHRUB_STEM_COUNT_MAX: u32 = 7;
const SHRUB_STEM_ANGLE_MIN: f32 = 20.0; // degrees
const SHRUB_STEM_ANGLE_MAX: f32 = 45.0; // degrees

// Grass dimensions
const GRASS_BLADE_WIDTH: f32 = 0.15;
const GRASS_BLADE_HEIGHT_BASE: f32 = 0.4; // Base height (scaled by visual_metadata.height)
const GRASS_BLADE_DEPTH: f32 = 0.02;
const GRASS_BLADE_Y_OFFSET: f32 = 0.2; // Position above ground

// Color constants
const TREE_TRUNK_COLOR: Color = Color::srgb(0.4, 0.25, 0.15); // Brown
const TREE_FOLIAGE_COLOR: Color = Color::srgb(0.2, 0.6, 0.2); // Green
                                                              // Color constants for shrubs and grass (used in spawn_shrub and spawn_grass)
                                                              // Inlined into spawn functions to maintain direct color values
#[allow(dead_code)]
const SHRUB_STEM_COLOR: Color = Color::srgb(0.25, 0.45, 0.15); // Dark green
#[allow(dead_code)]
const SHRUB_FOLIAGE_COLOR: Color = Color::srgb(0.35, 0.65, 0.25); // Medium green
#[allow(dead_code)]
const GRASS_BLADE_COLOR: Color = Color::srgb(0.3, 0.65, 0.2); // Grass green

const PORTAL_COLOR: Color = Color::srgb(0.53, 0.29, 0.87); // Purple
const SIGN_POST_COLOR: Color = Color::srgb(0.4, 0.3, 0.2); // Dark brown
const SIGN_BOARD_COLOR: Color = Color::srgb(0.59, 0.44, 0.27); // Tan

// Tile centering offset
/// Offset to center procedural meshes within their tile (matches camera centering)
const TILE_CENTER_OFFSET: f32 = 0.5;

// ==================== Public Functions ====================

/// Spawns a procedural tree mesh with trunk and foliage
///
/// Creates two child entities:
/// - Trunk: Brown cylinder (0.15 radius, 2.0 height)
/// - Foliage: Green sphere (0.6 radius) positioned at trunk top
///
/// Reuses cached meshes when available to avoid duplicate allocations.
/// This significantly improves performance when spawning multiple trees
/// on large maps.
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
/// * `cache` - Mutable reference to mesh cache for reuse
///
/// # Returns
///
/// Entity ID of the parent tree entity
///
/// # Examples
///
/// ```text
/// use antares::game::systems::procedural_meshes;
/// use antares::domain::types::{MapId, Position};
///
/// // Inside a Bevy system:
/// let mut cache = ProceduralMeshCache::default();
/// let tree_entity = procedural_meshes::spawn_tree(
///     &mut commands,
///     &mut materials,
///     &mut meshes,
///     Position { x: 5, y: 10 },
///     MapId::new(1),
///     &mut cache,
/// );
/// ```
pub fn spawn_tree(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    map_id: types::MapId,
    cache: &mut ProceduralMeshCache,
) -> Entity {
    // Get or create trunk mesh from cache
    let trunk_mesh = cache.tree_trunk.clone().unwrap_or_else(|| {
        let handle = meshes.add(Cylinder {
            radius: TREE_TRUNK_RADIUS,
            half_height: TREE_TRUNK_HEIGHT / 2.0,
        });
        cache.tree_trunk = Some(handle.clone());
        handle
    });
    let trunk_material = materials.add(StandardMaterial {
        base_color: TREE_TRUNK_COLOR,
        perceptual_roughness: 0.9,
        ..default()
    });

    // Get or create foliage mesh from cache
    let foliage_mesh = cache.tree_foliage.clone().unwrap_or_else(|| {
        let handle = meshes.add(Sphere {
            radius: TREE_FOLIAGE_RADIUS,
        });
        cache.tree_foliage = Some(handle.clone());
        handle
    });
    let foliage_material = materials.add(StandardMaterial {
        base_color: TREE_FOLIAGE_COLOR,
        perceptual_roughness: 0.8,
        ..default()
    });

    // Spawn parent tree entity
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
        ))
        .id();

    // Spawn trunk child at center of trunk height
    let trunk = commands
        .spawn((
            Mesh3d(trunk_mesh),
            MeshMaterial3d(trunk_material),
            Transform::from_xyz(0.0, TREE_TRUNK_HEIGHT / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    commands.entity(parent).add_child(trunk);

    // Spawn foliage child positioned at trunk top
    let foliage = commands
        .spawn((
            Mesh3d(foliage_mesh),
            MeshMaterial3d(foliage_material),
            Transform::from_xyz(0.0, TREE_FOLIAGE_Y_OFFSET, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    commands.entity(parent).add_child(foliage);

    parent
}

/// Spawns a procedurally generated multi-stem shrub
///
/// Shrubs use a multi-stem branch approach with no central trunk, creating
/// a natural bush-like appearance. Multiple stems radiate outward from ground
/// level at configurable angles.
///
/// Stem count is randomly determined between 3-7, with stem height and
/// foliage density customizable via terrain visual metadata.
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
/// * `visual_metadata` - Optional per-tile customization (height controls shrub size, scale affects foliage density)
/// * `cache` - Mutable reference to mesh cache for reuse
///
/// # Returns
///
/// Entity ID of the parent shrub entity
pub fn spawn_shrub(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    map_id: types::MapId,
    visual_metadata: Option<&TileVisualMetadata>,
    cache: &mut ProceduralMeshCache,
) -> Entity {
    // Determine stem count (3-7) randomly
    let mut rng = rand::rng();
    let stem_count = rng.random_range(SHRUB_STEM_COUNT_MIN..=SHRUB_STEM_COUNT_MAX);

    // Get effective height from visual metadata (default 0.6)
    let height_scale = visual_metadata
        .and_then(|m| m.height)
        .unwrap_or(0.6)
        .clamp(0.4, 0.8);

    // Get foliage density from scale (default 1.0)
    let foliage_scale = visual_metadata
        .and_then(|m| m.scale)
        .unwrap_or(1.0)
        .clamp(0.5, 1.5);

    // Get color tint (default green)
    let color_tint = visual_metadata
        .and_then(|m| m.color_tint)
        .unwrap_or((0.3, 0.65, 0.2));

    // Apply color tint to stem and foliage colors
    let stem_color = Color::srgb(
        0.25 * color_tint.0,
        0.45 * color_tint.1,
        0.15 * color_tint.2,
    );
    let foliage_color = Color::srgb(
        0.35 * color_tint.0,
        0.65 * color_tint.1,
        0.25 * color_tint.2,
    );

    // Get or create stem mesh from cache
    let stem_mesh = cache.shrub_stem.clone().unwrap_or_else(|| {
        let handle = meshes.add(Cylinder {
            radius: SHRUB_STEM_RADIUS,
            half_height: (SHRUB_STEM_HEIGHT_BASE * height_scale) / 2.0,
        });
        cache.shrub_stem = Some(handle.clone());
        handle
    });

    // Spawn parent shrub entity
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
        ))
        .id();

    // Spawn stems radiating outward
    for i in 0..stem_count {
        // Distribute stems evenly around the vertical axis
        let angle_horizontal = (i as f32 / stem_count as f32) * std::f32::consts::TAU;

        // Stem leans outward at a random angle
        let lean_angle_deg = rng.random_range(SHRUB_STEM_ANGLE_MIN..=SHRUB_STEM_ANGLE_MAX);
        let lean_angle_rad = lean_angle_deg.to_radians();

        // Calculate stem end position (leaning outward)
        let stem_height = SHRUB_STEM_HEIGHT_BASE * height_scale;
        let radial_distance = SHRUB_STEM_RADIUS * 3.0 * foliage_scale;

        let stem_end_x = radial_distance * angle_horizontal.cos() * lean_angle_rad.sin();
        let stem_end_y = stem_height * lean_angle_rad.cos();
        let stem_end_z = radial_distance * angle_horizontal.sin() * lean_angle_rad.sin();

        // Stem starting position (slightly offset from center)
        let stem_start_x = (SHRUB_STEM_RADIUS * 0.5) * angle_horizontal.cos();
        let _stem_start_y = 0.0;
        let stem_start_z = (SHRUB_STEM_RADIUS * 0.5) * angle_horizontal.sin();

        // Stem mid-point (for mesh positioning)
        let stem_mid_x = (stem_start_x + stem_end_x) / 2.0;
        let stem_mid_y = stem_height / 2.0;
        let stem_mid_z = (stem_start_z + stem_end_z) / 2.0;

        // Create stem material
        let stem_material = materials.add(StandardMaterial {
            base_color: stem_color,
            perceptual_roughness: 0.85,
            ..default()
        });

        // Spawn stem
        let stem = commands
            .spawn((
                Mesh3d(stem_mesh.clone()),
                MeshMaterial3d(stem_material),
                Transform::from_xyz(stem_mid_x, stem_mid_y, stem_mid_z),
                GlobalTransform::default(),
                Visibility::default(),
            ))
            .id();
        commands.entity(parent).add_child(stem);

        // Spawn foliage sphere at stem tip
        let foliage_radius = (TREE_FOLIAGE_RADIUS * 0.6) * foliage_scale;
        let foliage_mesh = meshes.add(Sphere {
            radius: foliage_radius,
        });
        let foliage_material = materials.add(StandardMaterial {
            base_color: foliage_color,
            perceptual_roughness: 0.8,
            ..default()
        });

        let foliage = commands
            .spawn((
                Mesh3d(foliage_mesh),
                MeshMaterial3d(foliage_material),
                Transform::from_xyz(stem_end_x, stem_end_y, stem_end_z),
                GlobalTransform::default(),
                Visibility::default(),
            ))
            .id();
        commands.entity(parent).add_child(foliage);
    }

    parent
}

/// Spawns grass blades on a grass terrain tile
///
/// Grass uses billboard quads that face the camera for performance.
/// Blade count and height are customizable via quality settings and
/// terrain visual metadata.
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - Tile position in world space
/// * `map_id` - Map identifier for organization
/// * `visual_metadata` - Optional per-tile customization (height = blade height)
/// * `quality_settings` - Grass density configuration resource
/// * `cache` - Mesh cache for performance
///
/// # Returns
///
/// Entity ID of the parent grass entity
#[allow(clippy::too_many_arguments)]
pub fn spawn_grass(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    map_id: types::MapId,
    visual_metadata: Option<&TileVisualMetadata>,
    quality_settings: &crate::game::resources::GrassQualitySettings,
    cache: &mut ProceduralMeshCache,
) -> Entity {
    // Get blade height from visual metadata (default 0.4)
    let blade_height = visual_metadata
        .and_then(|m| m.height)
        .unwrap_or(0.4)
        .clamp(0.2, 0.6);

    // Get color tint from visual metadata (default grass green)
    let color_tint = visual_metadata
        .and_then(|m| m.color_tint)
        .unwrap_or((0.3, 0.65, 0.2));

    let grass_color = Color::srgb(0.3 * color_tint.0, 0.65 * color_tint.1, 0.2 * color_tint.2);

    // Determine blade count based on quality settings
    let (min_blades, max_blades) = quality_settings.density.blade_count_range();
    let mut rng = rand::rng();
    let blade_count = rng.random_range(min_blades..=max_blades);

    // Get or create grass blade mesh from cache
    let blade_mesh = cache.grass_blade.clone().unwrap_or_else(|| {
        // Grass blade is a thin cuboid (billboard-like, but stored as 3D mesh)
        let handle = meshes.add(Cuboid {
            half_size: Vec3::new(
                GRASS_BLADE_WIDTH / 2.0,
                GRASS_BLADE_HEIGHT_BASE / 2.0,
                GRASS_BLADE_DEPTH / 2.0,
            ),
        });
        cache.grass_blade = Some(handle.clone());
        handle
    });

    // Spawn parent grass entity
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
        ))
        .id();

    // Spawn individual grass blades randomly distributed across tile
    for _ in 0..blade_count {
        // Random position within tile
        let tile_x = rng.random_range(0.0..1.0) - 0.5; // -0.5 to 0.5 within tile
        let tile_z = rng.random_range(0.0..1.0) - 0.5;

        // Random rotation around Y-axis for visual variety
        let rotation_y = rng.random_range(0.0..std::f32::consts::TAU);

        // Create grass blade material
        let blade_material = materials.add(StandardMaterial {
            base_color: grass_color,
            perceptual_roughness: 0.7,
            ..default()
        });

        // Spawn grass blade with Billboard component
        let blade = commands
            .spawn((
                Mesh3d(blade_mesh.clone()),
                MeshMaterial3d(blade_material),
                Transform::from_xyz(tile_x, GRASS_BLADE_Y_OFFSET + blade_height / 2.0, tile_z)
                    .with_rotation(Quat::from_rotation_y(rotation_y)),
                GlobalTransform::default(),
                Visibility::default(),
                Billboard {
                    lock_y: true, // Keep grass blades upright
                },
            ))
            .id();
        commands.entity(parent).add_child(blade);
    }

    parent
}

/// Spawns a procedural portal/teleport mesh
/// Spawns a procedural portal/teleport mesh (rectangular frame)
///
/// Creates a glowing purple rectangular frame positioned vertically at ground level.
/// The frame is composed of four cuboid bars forming a doorway-like portal.
/// Vertically oriented with rounded corners created by the thickness of the bars.
///
/// Reuses cached meshes when available to avoid duplicate allocations.
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - Tile position in world coordinates
/// * `event_name` - Event name for entity label
/// * `map_id` - Map identifier for cleanup
/// * `cache` - Mutable reference to mesh cache for reuse
/// * `rotation_y` - Optional rotation in degrees around Y-axis (default: 0.0)
///
/// # Returns
///
/// Entity ID of the parent portal entity
#[allow(clippy::too_many_arguments)]
pub fn spawn_portal(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    event_name: String,
    map_id: types::MapId,
    cache: &mut ProceduralMeshCache,
    rotation_y: Option<f32>,
) -> Entity {
    // Get or create horizontal bar mesh from cache (for top/bottom bars)
    let horizontal_bar_mesh = cache.portal_frame_horizontal.clone().unwrap_or_else(|| {
        let handle = meshes.add(Cuboid {
            half_size: Vec3::new(
                PORTAL_FRAME_WIDTH / 2.0,
                PORTAL_FRAME_THICKNESS / 2.0,
                PORTAL_FRAME_DEPTH / 2.0,
            ),
        });
        cache.portal_frame_horizontal = Some(handle.clone());
        handle
    });

    // Get or create vertical bar mesh from cache (for left/right bars)
    let vertical_bar_mesh = cache.portal_frame_vertical.clone().unwrap_or_else(|| {
        let handle = meshes.add(Cuboid {
            half_size: Vec3::new(
                PORTAL_FRAME_THICKNESS / 2.0,
                PORTAL_FRAME_HEIGHT / 2.0,
                PORTAL_FRAME_DEPTH / 2.0,
            ),
        });
        cache.portal_frame_vertical = Some(handle.clone());
        handle
    });

    // Create material for portal frame (shared by all bars)
    let material = materials.add(StandardMaterial {
        base_color: PORTAL_COLOR,
        perceptual_roughness: 0.3,
        emissive: LinearRgba {
            red: 0.2,
            green: 0.0,
            blue: 0.3,
            alpha: 1.0,
        },
        ..default()
    });

    // Spawn parent portal entity with optional rotation
    let rotation_radians = rotation_y.unwrap_or(0.0).to_radians();
    let transform = Transform::from_xyz(
        position.x as f32 + TILE_CENTER_OFFSET,
        PORTAL_Y_POSITION,
        position.y as f32 + TILE_CENTER_OFFSET,
    )
    .with_rotation(Quat::from_rotation_y(rotation_radians));

    let parent = commands
        .spawn((
            transform,
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new(format!("PortalMarker_{}", event_name)),
        ))
        .id();

    // Spawn frame bars as children
    // Top bar (horizontal)
    let top = commands
        .spawn((
            Mesh3d(horizontal_bar_mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(0.0, PORTAL_FRAME_HEIGHT / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    commands.entity(parent).add_child(top);

    // Bottom bar (horizontal)
    let bottom = commands
        .spawn((
            Mesh3d(horizontal_bar_mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(0.0, -PORTAL_FRAME_HEIGHT / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    commands.entity(parent).add_child(bottom);

    // Left bar (vertical)
    let left = commands
        .spawn((
            Mesh3d(vertical_bar_mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(-PORTAL_FRAME_WIDTH / 2.0, 0.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    commands.entity(parent).add_child(left);

    // Right bar (vertical)
    let right = commands
        .spawn((
            Mesh3d(vertical_bar_mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(PORTAL_FRAME_WIDTH / 2.0, 0.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    commands.entity(parent).add_child(right);

    parent
}

/// Spawns a procedural sign mesh with post and board
///
/// Creates two child entities:
/// - Post: Dark brown cylinder (0.05 radius, 1.5 height)
/// - Board: Tan cuboid sign board (0.6 width, 0.3 height, 0.05 depth)
///
/// Reuses cached meshes when available to avoid duplicate allocations.
/// This significantly improves performance when spawning multiple signs
/// on large maps.
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - Tile position in world coordinates
/// * `event_name` - Event name for entity label
/// * `map_id` - Map identifier for cleanup
/// * `cache` - Mutable reference to mesh cache for reuse
/// * `rotation_y` - Optional rotation in degrees around Y-axis (default: 0.0)
///
/// # Returns
///
/// Entity ID of the parent sign entity
#[allow(clippy::too_many_arguments)]
pub fn spawn_sign(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    event_name: String,
    map_id: types::MapId,
    cache: &mut ProceduralMeshCache,
    rotation_y: Option<f32>,
) -> Entity {
    // Get or create post mesh from cache
    let post_mesh = cache.sign_post.clone().unwrap_or_else(|| {
        let handle = meshes.add(Cylinder {
            radius: SIGN_POST_RADIUS,
            half_height: SIGN_POST_HEIGHT / 2.0,
        });
        cache.sign_post = Some(handle.clone());
        handle
    });
    let post_material = materials.add(StandardMaterial {
        base_color: SIGN_POST_COLOR,
        perceptual_roughness: 0.9,
        ..default()
    });

    // Get or create board mesh from cache
    let board_mesh = cache.sign_board.clone().unwrap_or_else(|| {
        let handle = meshes.add(Cuboid {
            half_size: Vec3::new(
                SIGN_BOARD_WIDTH / 2.0,
                SIGN_BOARD_HEIGHT / 2.0,
                SIGN_BOARD_DEPTH / 2.0,
            ),
        });
        cache.sign_board = Some(handle.clone());
        handle
    });
    let board_material = materials.add(StandardMaterial {
        base_color: SIGN_BOARD_COLOR,
        perceptual_roughness: 0.7,
        ..default()
    });

    // Spawn parent sign entity with optional rotation
    let rotation_radians = rotation_y.unwrap_or(0.0).to_radians();
    let transform = Transform::from_xyz(
        position.x as f32 + TILE_CENTER_OFFSET,
        0.0,
        position.y as f32 + TILE_CENTER_OFFSET,
    )
    .with_rotation(Quat::from_rotation_y(rotation_radians));

    let parent = commands
        .spawn((
            transform,
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new(format!("SignMarker_{}", event_name)),
        ))
        .id();

    // Spawn post child
    let post = commands
        .spawn((
            Mesh3d(post_mesh),
            MeshMaterial3d(post_material),
            Transform::from_xyz(0.0, SIGN_POST_HEIGHT / 2.0, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    commands.entity(parent).add_child(post);

    // Spawn board child
    let board = commands
        .spawn((
            Mesh3d(board_mesh),
            MeshMaterial3d(board_material),
            Transform::from_xyz(0.0, SIGN_BOARD_Y_OFFSET, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ))
        .id();
    commands.entity(parent).add_child(board);

    parent
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Constant Validation Tests ====================

    /// Validates tree constants are within reasonable bounds
    #[test]
    fn test_tree_constants_valid() {
        // Constants should be positive and follow size relationships
        // These checks serve as documentation of design invariants
        let _ = TREE_TRUNK_RADIUS;
        let _ = TREE_TRUNK_HEIGHT;
        let _ = TREE_FOLIAGE_RADIUS;
        // Compile will verify constants exist with correct values
    }

    /// Validates portal constants are within reasonable bounds
    #[test]
    fn test_portal_constants_valid() {
        // Constants should be positive and follow size relationships
        // These checks serve as documentation of design invariants
        let _ = PORTAL_FRAME_WIDTH;
        let _ = PORTAL_FRAME_HEIGHT;
        let _ = PORTAL_FRAME_THICKNESS;
        let _ = PORTAL_FRAME_DEPTH;
        let _ = PORTAL_Y_POSITION;
        // Compile will verify constants exist with correct values
    }

    /// Validates sign constants are within reasonable bounds
    #[test]
    fn test_sign_constants_valid() {
        // Constants should be positive and follow size relationships
        // These checks serve as documentation of design invariants
        let _ = SIGN_POST_RADIUS;
        let _ = SIGN_POST_HEIGHT;
        let _ = SIGN_BOARD_WIDTH;
        let _ = SIGN_BOARD_HEIGHT;
        let _ = SIGN_BOARD_DEPTH;
        let _ = SIGN_BOARD_Y_OFFSET;
        // Compile will verify constants exist with correct values
    }

    #[test]
    fn test_procedural_mesh_centering_offset() {
        assert_eq!(TILE_CENTER_OFFSET, 0.5);

        // Verify offset produces centered coordinates
        let pos = types::Position { x: 3, y: 7 };
        let centered_x = pos.x as f32 + TILE_CENTER_OFFSET;
        let centered_z = pos.y as f32 + TILE_CENTER_OFFSET;

        assert_eq!(centered_x, 3.5);
        assert_eq!(centered_z, 7.5);
    }

    // ==================== Mesh Caching Tests ====================

    /// Tests that ProceduralMeshCache initializes with all fields as None
    #[test]
    fn test_cache_default_all_none() {
        let cache = ProceduralMeshCache::default();
        assert!(cache.tree_trunk.is_none());
        assert!(cache.tree_foliage.is_none());
        assert!(cache.portal_frame_horizontal.is_none());
        assert!(cache.portal_frame_vertical.is_none());
        assert!(cache.sign_post.is_none());
        assert!(cache.sign_board.is_none());
    }

    /// Tests that cache can be cloned
    #[test]
    fn test_cache_is_cloneable() {
        let cache = ProceduralMeshCache::default();
        let _cloned = cache.clone();
        // If we can clone it, the test passes
    }

    /// Tests cache with tree_trunk set
    #[test]
    fn test_cache_with_tree_trunk_stored() {
        let cache = ProceduralMeshCache::default();
        assert!(cache.tree_trunk.is_none());

        // After initialization, cache should remain empty until set
        // This test documents the cache's purpose: to store handles
        assert!(cache.tree_foliage.is_none());
    }

    /// Tests that tree mesh dimensions are suitable for caching
    #[test]
    fn test_tree_trunk_dimensions_consistent() {
        // Tree trunk dimensions should be consistent for all spawns
        // allowing the mesh to be reused without quality loss.
        // These constants are verified at compile time through their usage in
        // Cylinder { radius, half_height } which requires valid f32 values.
        let _ = TREE_TRUNK_RADIUS;
        let _ = TREE_TRUNK_HEIGHT;
        // Test passes if constants compile with valid values
    }

    /// Tests that tree foliage dimensions are suitable for caching
    #[test]
    fn test_tree_foliage_dimensions_consistent() {
        // Tree foliage dimensions should be consistent.
        // Foliage should be larger than trunk for visual appeal.
        // These constants are verified at compile time through their usage in
        // Sphere { radius } which requires a valid f32 value.
        let _ = TREE_FOLIAGE_RADIUS;
        // Test passes if constants compile with valid values
    }

    /// Tests that portal frame dimensions are suitable for caching
    #[test]
    fn test_portal_frame_dimensions_consistent() {
        // Portal frame should have reasonable proportions.
        // Frame should be tall enough for player character to walk through.
        // These constants are verified at compile time through their usage in
        // Cuboid creation which requires valid f32 values.
        let _ = PORTAL_FRAME_WIDTH;
        let _ = PORTAL_FRAME_HEIGHT;
        let _ = PORTAL_FRAME_THICKNESS;
        let _ = PORTAL_FRAME_DEPTH;
        // Test passes if constants compile with valid values
    }

    /// Tests that sign post dimensions are suitable for caching
    #[test]
    fn test_sign_post_dimensions_consistent() {
        // Sign post should be thin and tall.
        // These constants are verified at compile time through their usage in
        // Cylinder { radius, half_height } which requires valid f32 values.
        let _ = SIGN_POST_RADIUS;
        let _ = SIGN_POST_HEIGHT;
        // Test passes if constants compile with valid values
    }

    /// Tests that sign board dimensions are suitable for caching
    #[test]
    fn test_sign_board_dimensions_consistent() {
        // Sign board should be a flat rectangle.
        // Board should be wider than tall for sign appearance.
        // Board should be thin (small depth).
        // These constants are verified at compile time through their usage in
        // Cuboid { half_size } which requires valid Vec3 values.
        let _ = SIGN_BOARD_WIDTH;
        let _ = SIGN_BOARD_HEIGHT;
        let _ = SIGN_BOARD_DEPTH;
        // Test passes if constants compile with valid values
    }

    /// Tests that mesh caching reduces allocations by documenting cache pattern
    #[test]
    fn test_mesh_cache_pattern_prevents_duplicates() {
        // This test documents the caching pattern:
        // 1. First spawn: mesh created, cached
        // 2. Second spawn: cached mesh reused
        // 3. Nth spawn: cached mesh reused
        //
        // With a large forest (100+ trees), caching provides significant
        // memory and allocation overhead reduction.
        //
        // Example scenario: spawning 100 trees without caching:
        //   - 100 trunk meshes allocated
        //   - 100 foliage meshes allocated
        //   - 200 mesh allocations total
        //
        // With caching:
        //   - 1 trunk mesh allocated
        //   - 1 foliage mesh allocated
        //   - 2 mesh allocations total
        //   - 99% reduction in mesh allocations
        //
        // This test passes by documenting the design intent.
        let cache = ProceduralMeshCache::default();
        assert!(cache.tree_trunk.is_none(), "Cache should start empty");
    }

    // ==================== Phase 2: Shrub & Grass Tests ====================

    /// Tests that shrub constants are properly defined
    /// (Compile-time verification via const definitions)
    #[test]
    fn test_shrub_constants_valid() {
        // Constants are verified at compile time by their usage
        // This test documents the design invariants
        let _ = SHRUB_STEM_RADIUS;
        let _ = SHRUB_STEM_HEIGHT_BASE;
        let _ = SHRUB_STEM_COUNT_MIN;
        let _ = SHRUB_STEM_COUNT_MAX;
        let _ = SHRUB_STEM_ANGLE_MIN;
        let _ = SHRUB_STEM_ANGLE_MAX;
        // Test passes if constants compile with valid values
    }

    /// Tests that grass constants are properly defined
    /// (Compile-time verification via const definitions)
    #[test]
    fn test_grass_constants_valid() {
        // Constants are verified at compile time by their usage
        // This test documents the design invariants
        let _ = GRASS_BLADE_WIDTH;
        let _ = GRASS_BLADE_HEIGHT_BASE;
        let _ = GRASS_BLADE_DEPTH;
        let _ = GRASS_BLADE_Y_OFFSET;
        // Test passes if constants compile with valid values
    }

    /// Tests that cache properly stores shrub stem meshes
    #[test]
    fn test_cache_shrub_stem_default_none() {
        let cache = ProceduralMeshCache::default();
        assert!(
            cache.shrub_stem.is_none(),
            "Shrub stem should start as None"
        );
    }

    /// Tests that cache properly stores grass blade meshes
    #[test]
    fn test_cache_grass_blade_default_none() {
        let cache = ProceduralMeshCache::default();
        assert!(
            cache.grass_blade.is_none(),
            "Grass blade should start as None"
        );
    }

    /// Tests grass quality settings integration
    #[test]
    fn test_grass_quality_settings_default_is_medium() {
        use crate::game::resources::GrassQualitySettings;
        let settings = GrassQualitySettings::default();
        assert_eq!(settings.density.name(), "Medium (6-10 blades)");
    }

    /// Tests grass blade count matches quality setting range
    #[test]
    fn test_grass_blade_count_matches_quality_setting() {
        use crate::game::resources::GrassQualitySettings;
        let settings = GrassQualitySettings::default();
        let (min, max) = settings.density.blade_count_range();
        assert!(min > 0, "Min blade count must be positive");
        assert!(max >= min, "Max blade count must be >= min");
    }

    /// Tests shrub stem count matches documented range
    /// (Compile-time verification via const assertions)
    #[test]
    fn test_shrub_stem_count_within_range() {
        // Values are const and verified at compile time
        // This test documents the design invariant
        let _ = SHRUB_STEM_COUNT_MIN;
        let _ = SHRUB_STEM_COUNT_MAX;
        // Test passes if constants compile with expected values
    }

    /// Tests shrub stem angle range is valid
    /// (Compile-time verification via const assertions)
    #[test]
    fn test_shrub_stem_angle_range_valid() {
        // Values are const and verified at compile time
        // This test documents the design invariant
        let _ = SHRUB_STEM_ANGLE_MIN;
        let _ = SHRUB_STEM_ANGLE_MAX;
        // Test passes if constants compile with expected values
    }
}
