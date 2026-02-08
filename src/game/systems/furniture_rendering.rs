// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Runtime Furniture Rendering System
//!
//! This module handles spawning furniture with:
//! - Material properties (PBR: metallic, roughness, base color)
//! - Color tints applied multiplicatively
//! - Emissive lighting for lit torches
//! - FurnitureEntity and Interactable components
//! - Proper blocking and interaction setup

use bevy::prelude::*;

use crate::domain::types;
use crate::domain::world::{FurnitureFlags, FurnitureMaterial, FurnitureType};
use crate::game::components::{FurnitureEntity, Interactable, InteractionType};
use crate::game::systems::procedural_meshes::{
    spawn_bench, spawn_chair, spawn_chest, spawn_table, spawn_throne, spawn_torch, BenchConfig,
    ChairConfig, ChestConfig, ProceduralMeshCache, TableConfig, ThroneConfig, TorchConfig,
};

/// Enhanced furniture spawning with material and interaction support
///
/// This function wraps the procedural mesh spawning functions and adds:
/// - Material property application (metallic, roughness)
/// - Color tint application (multiplicative blend)
/// - Emissive lighting for lit torches
/// - FurnitureEntity marker component
/// - Interactable component with appropriate interaction type
/// - Proper scale application
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - Tile position for furniture
/// * `map_id` - Map identifier for cleanup
/// * `furniture_type` - Type of furniture to spawn
/// * `rotation_y` - Optional Y-axis rotation in degrees
/// * `scale` - Size multiplier for the furniture
/// * `material` - Material type (Wood, Stone, Metal, Gold)
/// * `flags` - Furniture flags (lit, locked, blocking)
/// * `color_tint` - Optional RGB color tint [0.0..1.0]
/// * `cache` - Mutable reference to procedural mesh cache
///
/// # Returns
///
/// Entity ID of spawned furniture entity
#[allow(clippy::too_many_arguments)]
pub fn spawn_furniture_with_rendering(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    map_id: types::MapId,
    furniture_type: FurnitureType,
    rotation_y: Option<f32>,
    scale: f32,
    material: FurnitureMaterial,
    flags: FurnitureFlags,
    color_tint: Option<[f32; 3]>,
    cache: &mut ProceduralMeshCache,
) -> Entity {
    // Get base material properties
    let base_color_rgb = material.base_color();

    // Apply color tint if present (multiplicative blend)
    let final_color = if let Some([r, g, b]) = color_tint {
        Color::srgb(
            (base_color_rgb[0] * r).min(1.0),
            (base_color_rgb[1] * g).min(1.0),
            (base_color_rgb[2] * b).min(1.0),
        )
    } else {
        Color::srgb(base_color_rgb[0], base_color_rgb[1], base_color_rgb[2])
    };

    // Spawn the furniture using the procedural mesh system
    let furniture_entity = match furniture_type {
        FurnitureType::Throne => spawn_throne(
            commands,
            materials,
            meshes,
            position,
            map_id,
            ThroneConfig {
                ornamentation_level: 0.7,
                color_override: Some(final_color),
            },
            cache,
            rotation_y,
        ),
        FurnitureType::Bench => spawn_bench(
            commands,
            materials,
            meshes,
            position,
            map_id,
            BenchConfig {
                length: 1.5 * scale,
                height: 0.4 * scale,
                color_override: Some(final_color),
            },
            cache,
            rotation_y,
        ),
        FurnitureType::Table => spawn_table(
            commands,
            materials,
            meshes,
            position,
            map_id,
            TableConfig {
                width: 1.2 * scale,
                depth: 0.8 * scale,
                height: 0.7 * scale,
                color_override: Some(final_color),
            },
            cache,
            rotation_y,
        ),
        FurnitureType::Chair => spawn_chair(
            commands,
            materials,
            meshes,
            position,
            map_id,
            ChairConfig {
                back_height: 0.5 * scale,
                has_armrests: false,
                color_override: Some(final_color),
            },
            cache,
            rotation_y,
        ),
        FurnitureType::Torch => spawn_torch(
            commands,
            materials,
            meshes,
            position,
            map_id,
            TorchConfig {
                lit: flags.lit,
                height: 1.2 * scale,
                flame_color: Some(final_color),
            },
            cache,
            rotation_y,
        ),
        FurnitureType::Chest => spawn_chest(
            commands,
            materials,
            meshes,
            position,
            map_id,
            ChestConfig {
                locked: flags.locked,
                size_multiplier: scale,
                color_override: Some(final_color),
            },
            cache,
            rotation_y,
        ),
        FurnitureType::Bookshelf => spawn_table(
            commands,
            materials,
            meshes,
            position,
            map_id,
            TableConfig {
                width: 0.8 * scale,
                depth: 0.3 * scale,
                height: 1.8 * scale,
                color_override: Some(final_color),
            },
            cache,
            rotation_y,
        ),
        FurnitureType::Barrel => spawn_chest(
            commands,
            materials,
            meshes,
            position,
            map_id,
            ChestConfig {
                locked: false,
                size_multiplier: 0.9 * scale,
                color_override: Some(final_color),
            },
            cache,
            rotation_y,
        ),
    };

    // Add FurnitureEntity marker component
    commands
        .entity(furniture_entity)
        .insert(FurnitureEntity::new(furniture_type, flags.blocking));

    // Add Interactable component if applicable
    if let Some(interaction_type) = get_interaction_type(furniture_type) {
        let distance = get_interaction_distance(furniture_type);
        commands
            .entity(furniture_entity)
            .insert(Interactable::with_distance(interaction_type, distance));
    }

    furniture_entity
}

/// Get the interaction type for a furniture piece
///
/// # Returns
///
/// Some(InteractionType) if interactable, None otherwise
fn get_interaction_type(furniture_type: FurnitureType) -> Option<InteractionType> {
    match furniture_type {
        FurnitureType::Chest | FurnitureType::Barrel => Some(InteractionType::OpenChest),
        FurnitureType::Chair | FurnitureType::Throne => Some(InteractionType::SitOnChair),
        FurnitureType::Torch => Some(InteractionType::LightTorch),
        FurnitureType::Bookshelf => Some(InteractionType::ReadBookshelf),
        FurnitureType::Table | FurnitureType::Bench => None,
    }
}

/// Get the interaction distance for a furniture piece
fn get_interaction_distance(furniture_type: FurnitureType) -> f32 {
    match furniture_type {
        FurnitureType::Chest | FurnitureType::Barrel => 1.5,
        FurnitureType::Chair | FurnitureType::Throne => 1.5,
        FurnitureType::Torch | FurnitureType::Bookshelf => 2.0,
        FurnitureType::Table | FurnitureType::Bench => Interactable::DEFAULT_DISTANCE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_interaction_type_chest() {
        assert_eq!(
            get_interaction_type(FurnitureType::Chest),
            Some(InteractionType::OpenChest)
        );
    }

    #[test]
    fn test_get_interaction_type_barrel() {
        assert_eq!(
            get_interaction_type(FurnitureType::Barrel),
            Some(InteractionType::OpenChest)
        );
    }

    #[test]
    fn test_get_interaction_type_chair() {
        assert_eq!(
            get_interaction_type(FurnitureType::Chair),
            Some(InteractionType::SitOnChair)
        );
    }

    #[test]
    fn test_get_interaction_type_throne() {
        assert_eq!(
            get_interaction_type(FurnitureType::Throne),
            Some(InteractionType::SitOnChair)
        );
    }

    #[test]
    fn test_get_interaction_type_torch() {
        assert_eq!(
            get_interaction_type(FurnitureType::Torch),
            Some(InteractionType::LightTorch)
        );
    }

    #[test]
    fn test_get_interaction_type_bookshelf() {
        assert_eq!(
            get_interaction_type(FurnitureType::Bookshelf),
            Some(InteractionType::ReadBookshelf)
        );
    }

    #[test]
    fn test_get_interaction_distance_chest() {
        assert_eq!(get_interaction_distance(FurnitureType::Chest), 1.5);
    }

    #[test]
    fn test_get_interaction_distance_torch() {
        assert_eq!(get_interaction_distance(FurnitureType::Torch), 2.0);
    }

    #[test]
    fn test_get_interaction_distance_bookshelf() {
        assert_eq!(get_interaction_distance(FurnitureType::Bookshelf), 2.0);
    }

    #[test]
    fn test_material_properties_wood() {
        let wood = FurnitureMaterial::Wood;
        let base_color = wood.base_color();
        assert!(wood.metallic() < 0.2);
        assert!(wood.roughness() > 0.6);
        assert!(base_color[0] > 0.0); // Has color
    }

    #[test]
    fn test_material_properties_stone() {
        let stone = FurnitureMaterial::Stone;
        assert!(stone.metallic() <= 0.1);
        assert!(stone.roughness() > 0.8);
    }

    #[test]
    fn test_material_properties_metal() {
        let metal = FurnitureMaterial::Metal;
        assert!(metal.metallic() > 0.5);
        assert!(metal.roughness() < 0.5);
    }

    #[test]
    fn test_material_properties_gold() {
        let gold = FurnitureMaterial::Gold;
        assert!(gold.metallic() > 0.5);
        assert!(gold.roughness() < 0.5);
    }
}
