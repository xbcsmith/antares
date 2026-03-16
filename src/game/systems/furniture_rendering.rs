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

use crate::domain::types::{self, FurnitureId};
use crate::domain::visual::CreatureDefinition;
use crate::domain::world::furniture::FurnitureDatabase;
use crate::domain::world::{DoorFrameConfig, FurnitureFlags, FurnitureMaterial, FurnitureType};
use crate::game::components::{DoorState, FurnitureEntity, Interactable, InteractionType};
use crate::game::systems::procedural_meshes::{
    spawn_bench, spawn_chair, spawn_chest, spawn_door_with_frame, spawn_table, spawn_throne,
    spawn_torch, BenchConfig, ChairConfig, ChestConfig, DoorConfig, ProceduralMeshCache,
    TableConfig, ThroneConfig, TorchConfig,
};

/// Resolves effective furniture properties by merging a [`FurnitureDatabase`]
/// definition with per-instance inline overrides from a `MapEvent::Furniture`.
///
/// ## Resolution rules
///
/// | `furniture_id` | DB lookup | Result |
/// |---|---|---|
/// | `None` | n/a | All inline values used as-is (backward compatible) |
/// | `Some(id)` | found | `base_type`, `material`, `scale`, `flags` from DB definition; `color_tint` from inline if `Some`, else from DB |
/// | `Some(id)` | not found | Warning logged; falls back to all inline values |
///
/// `rotation_y` is always taken from the inline field (it is positional, not a
/// template property) and is therefore **not** a parameter of this function.
///
/// # Arguments
///
/// * `furniture_id` — optional definition ID from `MapEvent::Furniture`
/// * `inline_type` — inline `furniture_type` field
/// * `inline_material` — inline `material` field
/// * `inline_scale` — inline `scale` field
/// * `inline_flags` — inline `flags` field
/// * `inline_color_tint` — inline `color_tint` field
/// * `db` — the campaign's loaded [`FurnitureDatabase`]
///
/// # Returns
///
/// A tuple `(FurnitureType, FurnitureMaterial, f32, FurnitureFlags, Option<[f32; 3]>)`
/// representing the resolved type, material, scale, flags, and color tint.
///
/// # Examples
///
/// ```
/// use antares::game::systems::furniture_rendering::resolve_furniture_fields;
/// use antares::domain::world::furniture::FurnitureDatabase;
/// use antares::domain::world::{FurnitureType, FurnitureMaterial, FurnitureFlags};
///
/// let db = FurnitureDatabase::new();
///
/// // No furniture_id → inline values returned unchanged
/// let (ft, mat, scale, flags, tint) = resolve_furniture_fields(
///     None,
///     FurnitureType::Bench,
///     FurnitureMaterial::Stone,
///     1.5,
///     &FurnitureFlags { lit: false, locked: false, blocking: true },
///     Some([0.8, 0.8, 0.8]),
///     &db,
/// );
/// assert_eq!(ft, FurnitureType::Bench);
/// assert_eq!(mat, FurnitureMaterial::Stone);
/// assert_eq!(tint, Some([0.8, 0.8, 0.8]));
/// ```
pub fn resolve_furniture_fields(
    furniture_id: Option<FurnitureId>,
    inline_type: FurnitureType,
    inline_material: FurnitureMaterial,
    inline_scale: f32,
    inline_flags: &FurnitureFlags,
    inline_color_tint: Option<[f32; 3]>,
    db: &FurnitureDatabase,
) -> (
    FurnitureType,
    FurnitureMaterial,
    f32,
    FurnitureFlags,
    Option<[f32; 3]>,
) {
    let Some(id) = furniture_id else {
        // No id — pure inline fields, full backward compatibility
        return (
            inline_type,
            inline_material,
            inline_scale,
            inline_flags.clone(),
            inline_color_tint,
        );
    };

    let Some(def) = db.get_by_id(id) else {
        warn!(
            "furniture_id {} not found in FurnitureDatabase; falling back to inline fields",
            id
        );
        return (
            inline_type,
            inline_material,
            inline_scale,
            inline_flags.clone(),
            inline_color_tint,
        );
    };

    // Definition found: use its base values.
    // Inline `color_tint` acts as a per-instance override when it is `Some`.
    let resolved_color = inline_color_tint.or(def.color_tint);

    (
        def.base_type,
        def.material,
        def.scale,
        def.flags.clone(),
        resolved_color,
    )
}

/// Spawns a custom registered furniture mesh using the creature-mesh pipeline.
///
/// The furniture mesh registry stores mesh assets in the same `CreatureDefinition`
/// RON format used by imported creature and item meshes. This helper converts the
/// first mesh in that definition into a Bevy `Mesh`, applies furniture PBR
/// properties, and spawns it as the furniture entity root.
///
/// # Arguments
///
/// * `commands` - Bevy Commands for entity spawning
/// * `materials` - Material asset storage
/// * `meshes` - Mesh asset storage
/// * `position` - Tile position for furniture
/// * `map_id` - Map identifier for cleanup
/// * `creature_def` - Imported mesh definition loaded from the furniture mesh registry
/// * `rotation_y` - Optional Y-axis rotation in degrees
/// * `scale` - Size multiplier for the furniture
/// * `material` - Furniture material that provides base color and PBR defaults
/// * `flags` - Furniture flags (lit, locked, blocking)
/// * `color_tint` - Optional RGB color tint [0.0..1.0]
/// * `cache` - Mutable reference to procedural mesh cache
///
/// # Returns
///
/// `Some(entity)` when a custom mesh was spawned, or `None` when the definition
/// had no meshes and the caller should fall back to procedural rendering.
#[allow(clippy::too_many_arguments)]
pub fn spawn_custom_furniture_mesh_with_rendering(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    position: types::Position,
    map_id: types::MapId,
    creature_def: &CreatureDefinition,
    rotation_y: Option<f32>,
    scale: f32,
    material: FurnitureMaterial,
    flags: FurnitureFlags,
    color_tint: Option<[f32; 3]>,
    cache: &mut ProceduralMeshCache,
) -> Option<Entity> {
    let mesh_def = creature_def.meshes.first()?;

    let base_color_rgb = material.base_color();
    let final_color = if let Some([r, g, b]) = color_tint {
        [
            (base_color_rgb[0] * r).min(1.0),
            (base_color_rgb[1] * g).min(1.0),
            (base_color_rgb[2] * b).min(1.0),
            1.0,
        ]
    } else {
        [base_color_rgb[0], base_color_rgb[1], base_color_rgb[2], 1.0]
    };

    let mesh_handle = cache.get_or_create_creature_mesh(creature_def.id, 0, mesh_def, meshes);

    let mut standard_material = StandardMaterial {
        base_color: Color::srgba(
            final_color[0],
            final_color[1],
            final_color[2],
            final_color[3],
        ),
        metallic: material.metallic(),
        perceptual_roughness: material.roughness(),
        ..default()
    };

    if flags.lit {
        standard_material.emissive = Color::srgb(
            final_color[0] * 0.8,
            final_color[1] * 0.8,
            final_color[2] * 0.8,
        )
        .into();
    }

    let material_handle = materials.add(standard_material);

    let translation = Vec3::new(position.x as f32, 0.0, position.y as f32);
    let rotation = Quat::from_rotation_y(rotation_y.unwrap_or(0.0).to_radians());
    let mesh_scale = Vec3::splat(creature_def.scale * scale);

    let furniture_entity = commands
        .spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
            Transform {
                translation,
                rotation,
                scale: mesh_scale,
            },
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            crate::game::systems::map::MapEntity(map_id),
            Name::new(creature_def.name.clone()),
        ))
        .id();

    commands
        .entity(furniture_entity)
        .insert(FurnitureEntity::new(FurnitureType::Table, flags.blocking));

    if let Some(interaction_type) = get_interaction_type(FurnitureType::Table) {
        commands
            .entity(furniture_entity)
            .insert(Interactable::with_distance(interaction_type, 2.0));
    }

    Some(furniture_entity)
}

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
        FurnitureType::Door => {
            let (door_entity, _frame_entity) = spawn_door_with_frame(
                commands,
                materials,
                meshes,
                position,
                map_id,
                DoorConfig {
                    width: 0.9 * scale,
                    height: 2.3 * scale,
                    has_hinges: true,
                    has_studs: true,
                    color_override: Some(final_color),
                    ..Default::default()
                },
                DoorFrameConfig::default(),
                cache,
                rotation_y,
            );
            // Attach DoorState so the interaction system can track open/locked state.
            // base_rotation_y is the initial Y rotation of the door entity (in radians),
            // used to restore the closed visual when the door is shut again.
            let rotation_radians = rotation_y.unwrap_or(0.0).to_radians();
            commands
                .entity(door_entity)
                .insert(DoorState::new(flags.locked, rotation_radians));
            door_entity
        }
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
        FurnitureType::Door => Some(InteractionType::OpenDoor),
        FurnitureType::Table | FurnitureType::Bench => None,
    }
}

/// Get the interaction distance for a furniture piece
fn get_interaction_distance(furniture_type: FurnitureType) -> f32 {
    match furniture_type {
        FurnitureType::Chest | FurnitureType::Barrel => 1.5,
        FurnitureType::Chair | FurnitureType::Throne => 1.5,
        FurnitureType::Torch | FurnitureType::Bookshelf => 2.0,
        FurnitureType::Door => 1.5,
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
    fn test_get_interaction_type_door() {
        assert_eq!(
            get_interaction_type(FurnitureType::Door),
            Some(InteractionType::OpenDoor)
        );
    }

    #[test]
    fn test_get_interaction_distance_door() {
        assert_eq!(get_interaction_distance(FurnitureType::Door), 1.5);
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

    // ===== resolve_furniture_fields tests =====

    use crate::domain::world::furniture::{FurnitureDatabase, FurnitureDefinition};
    use crate::domain::world::FurnitureCategory;

    fn make_test_def(id: u32) -> FurnitureDefinition {
        FurnitureDefinition {
            id,
            name: format!("Test Def {}", id),
            category: FurnitureCategory::Seating,
            base_type: FurnitureType::Throne,
            material: FurnitureMaterial::Gold,
            scale: 2.0,
            color_tint: Some([0.9, 0.8, 0.7]),
            flags: FurnitureFlags {
                lit: true,
                locked: false,
                blocking: true,
            },
            icon: None,
            tags: vec![],
            mesh_id: None,
            description: None,
        }
    }

    #[test]
    fn test_resolve_furniture_fields_no_id_returns_inline() {
        let db = FurnitureDatabase::new();
        let inline_flags = FurnitureFlags {
            lit: false,
            locked: true,
            blocking: false,
        };
        let (ft, mat, scale, flags, tint) = resolve_furniture_fields(
            None,
            FurnitureType::Bench,
            FurnitureMaterial::Stone,
            1.5,
            &inline_flags,
            Some([0.1, 0.2, 0.3]),
            &db,
        );
        assert_eq!(ft, FurnitureType::Bench);
        assert_eq!(mat, FurnitureMaterial::Stone);
        assert!((scale - 1.5).abs() < f32::EPSILON);
        assert!(flags.locked);
        assert_eq!(tint, Some([0.1, 0.2, 0.3]));
    }

    #[test]
    fn test_resolve_furniture_fields_with_id_uses_def_values() {
        let mut db = FurnitureDatabase::new();
        db.add(make_test_def(1)).unwrap();

        let inline_flags = FurnitureFlags::default();
        let (ft, mat, scale, flags, tint) = resolve_furniture_fields(
            Some(1),
            FurnitureType::Bench,    // overridden by def (Throne)
            FurnitureMaterial::Wood, // overridden by def (Gold)
            1.0,                     // overridden by def (2.0)
            &inline_flags,
            None, // no inline override → use def color_tint
            &db,
        );
        assert_eq!(ft, FurnitureType::Throne, "base_type from definition");
        assert_eq!(mat, FurnitureMaterial::Gold, "material from definition");
        assert!((scale - 2.0).abs() < f32::EPSILON, "scale from definition");
        assert!(flags.lit, "flags.lit from definition");
        assert_eq!(
            tint,
            Some([0.9, 0.8, 0.7]),
            "color_tint from definition when inline is None"
        );
    }

    #[test]
    fn test_resolve_furniture_fields_inline_color_tint_overrides_def() {
        let mut db = FurnitureDatabase::new();
        db.add(make_test_def(2)).unwrap();

        let inline_flags = FurnitureFlags::default();
        let inline_tint = Some([0.1, 0.2, 0.3]);
        let (_, _, _, _, tint) = resolve_furniture_fields(
            Some(2),
            FurnitureType::Chair,
            FurnitureMaterial::Wood,
            1.0,
            &inline_flags,
            inline_tint, // explicit inline override
            &db,
        );
        assert_eq!(
            tint,
            Some([0.1, 0.2, 0.3]),
            "inline color_tint wins over definition color_tint"
        );
    }

    #[test]
    fn test_resolve_furniture_fields_missing_id_falls_back_to_inline() {
        let db = FurnitureDatabase::new(); // empty — id 99 not present
        let inline_flags = FurnitureFlags {
            lit: false,
            locked: false,
            blocking: true,
        };
        let (ft, mat, scale, flags, tint) = resolve_furniture_fields(
            Some(99),
            FurnitureType::Chest,
            FurnitureMaterial::Metal,
            0.8,
            &inline_flags,
            Some([0.5, 0.5, 0.5]),
            &db,
        );
        // Falls back to inline because id 99 is not in the db
        assert_eq!(ft, FurnitureType::Chest, "fallback: inline type");
        assert_eq!(mat, FurnitureMaterial::Metal, "fallback: inline material");
        assert!((scale - 0.8).abs() < f32::EPSILON, "fallback: inline scale");
        assert!(flags.blocking, "fallback: inline flags");
        assert_eq!(tint, Some([0.5, 0.5, 0.5]), "fallback: inline color_tint");
    }

    #[test]
    fn test_resolve_furniture_fields_def_color_none_and_inline_none() {
        // When both def.color_tint and inline_color_tint are None, result is None
        let mut db = FurnitureDatabase::new();
        let def = FurnitureDefinition {
            id: 5,
            name: "No Tint".to_string(),
            category: FurnitureCategory::Utility,
            base_type: FurnitureType::Table,
            material: FurnitureMaterial::Wood,
            scale: 1.0,
            color_tint: None,
            flags: FurnitureFlags::default(),
            icon: None,
            tags: vec![],
            mesh_id: None,
            description: None,
        };
        db.add(def).unwrap();

        let (_, _, _, _, tint) = resolve_furniture_fields(
            Some(5),
            FurnitureType::Bench,
            FurnitureMaterial::Stone,
            1.0,
            &FurnitureFlags::default(),
            None,
            &db,
        );
        assert_eq!(tint, None, "both None → resolved tint is None");
    }
}
