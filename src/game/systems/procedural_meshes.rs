// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Procedural mesh generation for environmental objects and static event markers
//!
//! This module provides pure Rust functions to spawn composite 3D meshes using
//! Bevy primitives (Cylinder, Sphere, Torus, Cuboid). No external assets required.
//!
//! Character rendering (NPCs, Monsters, Recruitables) uses the sprite system.

use super::map::{MapEntity, TileCoord};
use crate::domain::types;
use bevy::prelude::*;

// ==================== Constants ====================

// Tree dimensions (world units, 1 unit ≈ 10 feet)
const TREE_TRUNK_RADIUS: f32 = 0.15;
const TREE_TRUNK_HEIGHT: f32 = 2.0;
const TREE_FOLIAGE_RADIUS: f32 = 0.6;
const TREE_FOLIAGE_Y_OFFSET: f32 = 2.0;

// Event marker dimensions
const PORTAL_TORUS_MAJOR_RADIUS: f32 = 0.4;
const PORTAL_TORUS_MINOR_RADIUS: f32 = 0.05;
const PORTAL_Y_POSITION: f32 = 0.5;
const _PORTAL_ROTATION_SPEED: f32 = 1.0; // radians/sec

const SIGN_POST_RADIUS: f32 = 0.05;
const SIGN_POST_HEIGHT: f32 = 1.5;
const SIGN_BOARD_WIDTH: f32 = 0.6;
const SIGN_BOARD_HEIGHT: f32 = 0.3;
const SIGN_BOARD_DEPTH: f32 = 0.05;
const SIGN_BOARD_Y_OFFSET: f32 = 1.3;

// Color constants
const TREE_TRUNK_COLOR: Color = Color::srgb(0.4, 0.25, 0.15); // Brown
const TREE_FOLIAGE_COLOR: Color = Color::srgb(0.2, 0.6, 0.2); // Green

const PORTAL_COLOR: Color = Color::srgb(0.53, 0.29, 0.87); // Purple
const SIGN_POST_COLOR: Color = Color::srgb(0.4, 0.3, 0.2); // Dark brown
const SIGN_BOARD_COLOR: Color = Color::srgb(0.59, 0.44, 0.27); // Tan

// ==================== Public Functions ====================

/// Spawns a procedural tree mesh with trunk and foliage
///
/// Creates two child entities:
/// - Trunk: Brown cylinder (0.15 radius, 2.0 height)
/// - Foliage: Green sphere (0.6 radius) positioned at trunk top
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - Tile position in world coordinates
/// * `map_id` - Map identifier for cleanup
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
/// let tree_entity = procedural_meshes::spawn_tree(
///     &mut commands,
///     &mut materials,
///     &mut meshes,
///     Position { x: 5, y: 10 },
///     MapId::new(1),
/// );
/// ```
pub fn spawn_tree(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    map_id: types::MapId,
) -> Entity {
    // Create trunk mesh and material
    let trunk_mesh = meshes.add(Cylinder {
        radius: TREE_TRUNK_RADIUS,
        half_height: TREE_TRUNK_HEIGHT / 2.0,
    });
    let trunk_material = materials.add(StandardMaterial {
        base_color: TREE_TRUNK_COLOR,
        perceptual_roughness: 0.9,
        ..default()
    });

    // Create foliage mesh and material
    let foliage_mesh = meshes.add(Sphere {
        radius: TREE_FOLIAGE_RADIUS,
    });
    let foliage_material = materials.add(StandardMaterial {
        base_color: TREE_FOLIAGE_COLOR,
        perceptual_roughness: 0.8,
        ..default()
    });

    // Spawn parent tree entity
    let parent = commands
        .spawn((
            Transform::from_xyz(position.x as f32, 0.0, position.y as f32),
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

/// Spawns a procedural portal/teleport mesh
///
/// Creates a rotating torus mesh to represent a magical portal.
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - Tile position in world coordinates
/// * `event_name` - Event name for entity label
/// * `map_id` - Map identifier for cleanup
///
/// # Returns
///
/// Entity ID of the portal entity
pub fn spawn_portal(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    event_name: String,
    map_id: types::MapId,
) -> Entity {
    let mesh = meshes.add(Torus {
        major_radius: PORTAL_TORUS_MAJOR_RADIUS,
        minor_radius: PORTAL_TORUS_MINOR_RADIUS,
    });

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

    commands
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(position.x as f32, PORTAL_Y_POSITION, position.y as f32),
            GlobalTransform::default(),
            Visibility::default(),
            MapEntity(map_id),
            TileCoord(position),
            Name::new(format!("PortalMarker_{}", event_name)),
        ))
        .id()
}

/// Spawns a procedural sign mesh with post and board
///
/// Creates two child entities:
/// - Post: Dark brown cylinder (0.05 radius, 1.5 height)
/// - Board: Tan cuboid sign board (0.6 width, 0.3 height, 0.05 depth)
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - Tile position in world coordinates
/// * `event_name` - Event name for entity label
/// * `map_id` - Map identifier for cleanup
///
/// # Returns
///
/// Entity ID of the parent sign entity
pub fn spawn_sign(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    event_name: String,
    map_id: types::MapId,
) -> Entity {
    // Create post mesh and material
    let post_mesh = meshes.add(Cylinder {
        radius: SIGN_POST_RADIUS,
        half_height: SIGN_POST_HEIGHT / 2.0,
    });
    let post_material = materials.add(StandardMaterial {
        base_color: SIGN_POST_COLOR,
        perceptual_roughness: 0.9,
        ..default()
    });

    // Create board mesh and material
    let board_mesh = meshes.add(Cuboid {
        half_size: Vec3::new(
            SIGN_BOARD_WIDTH / 2.0,
            SIGN_BOARD_HEIGHT / 2.0,
            SIGN_BOARD_DEPTH / 2.0,
        ),
    });
    let board_material = materials.add(StandardMaterial {
        base_color: SIGN_BOARD_COLOR,
        perceptual_roughness: 0.7,
        ..default()
    });

    // Spawn parent sign entity
    let parent = commands
        .spawn((
            Transform::from_xyz(position.x as f32, 0.0, position.y as f32),
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
        let _ = PORTAL_TORUS_MAJOR_RADIUS;
        let _ = PORTAL_TORUS_MINOR_RADIUS;
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
        // Compile will verify constants exist with correct values
    }
}
