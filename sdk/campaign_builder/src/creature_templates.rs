// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Creature template generators for the campaign builder
//!
//! Provides pre-built creature templates using primitive shapes to help
//! content creators get started quickly.

use crate::primitive_generators::{
    generate_cone, generate_cube, generate_cylinder, generate_sphere,
};
use antares::domain::visual::{CreatureDefinition, MeshTransform};

/// Generates a simple humanoid creature using primitives
///
/// Creates a basic biped creature with:
/// - Cube torso
/// - Sphere head
/// - Cylinder limbs
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` representing a humanoid creature
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_humanoid_template;
///
/// let humanoid = generate_humanoid_template("Knight", 1);
/// assert_eq!(humanoid.name, "Knight");
/// assert!(humanoid.meshes.len() > 5); // Body, head, arms, legs
/// ```
pub fn generate_humanoid_template(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Torso (cube)
    meshes.push(generate_cube(1.0, [0.7, 0.7, 0.7, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 1.0, 0.0));

    // Head (sphere)
    meshes.push(generate_sphere(0.35, 12, 12, [0.9, 0.8, 0.7, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 2.0, 0.0));

    // Left arm (cylinder)
    meshes.push(generate_cylinder(0.15, 0.8, 8, [0.7, 0.7, 0.7, 1.0]));
    transforms.push(MeshTransform::translation(-0.65, 1.0, 0.0));

    // Right arm (cylinder)
    meshes.push(generate_cylinder(0.15, 0.8, 8, [0.7, 0.7, 0.7, 1.0]));
    transforms.push(MeshTransform::translation(0.65, 1.0, 0.0));

    // Left leg (cylinder)
    meshes.push(generate_cylinder(0.2, 1.0, 8, [0.6, 0.6, 0.6, 1.0]));
    transforms.push(MeshTransform::translation(-0.3, 0.0, 0.0));

    // Right leg (cylinder)
    meshes.push(generate_cylinder(0.2, 1.0, 8, [0.6, 0.6, 0.6, 1.0]));
    transforms.push(MeshTransform::translation(0.3, 0.0, 0.0));

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

/// Generates a quadruped creature template
///
/// Creates a four-legged creature with:
/// - Elongated cube body
/// - Sphere head
/// - Four cylinder legs
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` representing a quadruped creature
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_quadruped_template;
///
/// let wolf = generate_quadruped_template("Wolf", 2);
/// assert_eq!(wolf.name, "Wolf");
/// ```
pub fn generate_quadruped_template(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Body (elongated cube)
    meshes.push(generate_cube(1.0, [0.6, 0.5, 0.4, 1.0]));
    transforms.push(MeshTransform::scale(1.5, 0.8, 0.6).with_translation(0.0, 0.8, 0.0));

    // Head (sphere)
    meshes.push(generate_sphere(0.3, 12, 12, [0.7, 0.6, 0.5, 1.0]));
    transforms.push(MeshTransform::translation(1.0, 1.0, 0.0));

    // Front left leg
    meshes.push(generate_cylinder(0.12, 0.8, 8, [0.5, 0.4, 0.3, 1.0]));
    transforms.push(MeshTransform::translation(0.5, 0.0, -0.3));

    // Front right leg
    meshes.push(generate_cylinder(0.12, 0.8, 8, [0.5, 0.4, 0.3, 1.0]));
    transforms.push(MeshTransform::translation(0.5, 0.0, 0.3));

    // Back left leg
    meshes.push(generate_cylinder(0.12, 0.8, 8, [0.5, 0.4, 0.3, 1.0]));
    transforms.push(MeshTransform::translation(-0.5, 0.0, -0.3));

    // Back right leg
    meshes.push(generate_cylinder(0.12, 0.8, 8, [0.5, 0.4, 0.3, 1.0]));
    transforms.push(MeshTransform::translation(-0.5, 0.0, 0.3));

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

/// Generates a flying creature template
///
/// Creates a winged creature with:
/// - Sphere body
/// - Cone beak
/// - Flat wing shapes
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` representing a flying creature
pub fn generate_flying_template(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Body (sphere)
    meshes.push(generate_sphere(0.4, 12, 12, [0.8, 0.8, 0.9, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 0.5, 0.0));

    // Beak (cone)
    meshes.push(generate_cone(0.1, 0.3, 8, [1.0, 0.7, 0.0, 1.0]));
    transforms.push(
        MeshTransform::rotation(0.0, 0.0, std::f32::consts::PI / 2.0)
            .with_translation(0.4, 0.5, 0.0),
    );

    // Left wing (flat cube)
    meshes.push(generate_cube(0.6, [0.7, 0.7, 0.8, 1.0]));
    transforms.push(MeshTransform::scale(0.1, 0.6, 1.2).with_translation(-0.5, 0.5, 0.0));

    // Right wing (flat cube)
    meshes.push(generate_cube(0.6, [0.7, 0.7, 0.8, 1.0]));
    transforms.push(MeshTransform::scale(0.1, 0.6, 1.2).with_translation(0.5, 0.5, 0.0));

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

/// Generates a simple slime/blob creature
///
/// Creates a simple blob creature using a flattened sphere
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` representing a slime creature
pub fn generate_slime_template(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Body (squashed sphere)
    meshes.push(generate_sphere(0.6, 16, 12, [0.2, 0.8, 0.3, 0.8]));
    transforms.push(MeshTransform::scale(1.0, 0.6, 1.0).with_translation(0.0, 0.3, 0.0));

    // Eyes (small spheres)
    meshes.push(generate_sphere(0.08, 6, 6, [0.0, 0.0, 0.0, 1.0]));
    transforms.push(MeshTransform::translation(0.2, 0.5, 0.4));

    meshes.push(generate_sphere(0.08, 6, 6, [0.0, 0.0, 0.0, 1.0]));
    transforms.push(MeshTransform::translation(-0.2, 0.5, 0.4));

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: Some([0.2, 0.9, 0.3, 0.9]),
    }
}

/// Generates a simple dragon template
///
/// Creates a dragon-like creature with:
/// - Elongated body
/// - Head with horns
/// - Wings
/// - Tail
/// - Four legs
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` representing a dragon
pub fn generate_dragon_template(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Body (elongated sphere)
    meshes.push(generate_sphere(0.6, 16, 16, [0.8, 0.2, 0.2, 1.0]));
    transforms.push(MeshTransform::scale(2.0, 1.0, 1.0).with_translation(0.0, 1.2, 0.0));

    // Head (sphere)
    meshes.push(generate_sphere(0.4, 12, 12, [0.9, 0.3, 0.3, 1.0]));
    transforms.push(MeshTransform::translation(1.5, 1.5, 0.0));

    // Left horn (cone)
    meshes.push(generate_cone(0.1, 0.4, 8, [0.7, 0.7, 0.1, 1.0]));
    transforms.push(MeshTransform::translation(1.4, 2.0, -0.2));

    // Right horn (cone)
    meshes.push(generate_cone(0.1, 0.4, 8, [0.7, 0.7, 0.1, 1.0]));
    transforms.push(MeshTransform::translation(1.4, 2.0, 0.2));

    // Left wing
    meshes.push(generate_cube(1.0, [0.6, 0.1, 0.1, 1.0]));
    transforms.push(MeshTransform::scale(0.1, 0.8, 2.0).with_translation(-0.7, 1.5, 0.0));

    // Right wing
    meshes.push(generate_cube(1.0, [0.6, 0.1, 0.1, 1.0]));
    transforms.push(MeshTransform::scale(0.1, 0.8, 2.0).with_translation(0.7, 1.5, 0.0));

    // Tail (cylinder)
    meshes.push(generate_cylinder(0.2, 1.5, 8, [0.7, 0.2, 0.2, 1.0]));
    transforms.push(
        MeshTransform::rotation(0.0, 0.0, std::f32::consts::PI / 4.0)
            .with_translation(-1.5, 0.8, 0.0),
    );

    // Legs (simplified)
    for i in 0..4 {
        let x = if i < 2 { 0.8 } else { -0.8 };
        let z = if i % 2 == 0 { -0.4 } else { 0.4 };

        meshes.push(generate_cylinder(0.15, 1.0, 8, [0.6, 0.2, 0.2, 1.0]));
        transforms.push(MeshTransform::translation(x, 0.0, z));
    }

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

/// Returns a list of all available creature templates
///
/// # Returns
///
/// A vector of tuples containing (template_name, generator_function)
pub fn available_templates() -> Vec<(&'static str, fn(&str, u32) -> CreatureDefinition)> {
    vec![
        (
            "Humanoid",
            generate_humanoid_template as fn(&str, u32) -> CreatureDefinition,
        ),
        (
            "Quadruped",
            generate_quadruped_template as fn(&str, u32) -> CreatureDefinition,
        ),
        (
            "Flying Creature",
            generate_flying_template as fn(&str, u32) -> CreatureDefinition,
        ),
        (
            "Slime/Blob",
            generate_slime_template as fn(&str, u32) -> CreatureDefinition,
        ),
        (
            "Dragon",
            generate_dragon_template as fn(&str, u32) -> CreatureDefinition,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_humanoid_template_structure() {
        let creature = generate_humanoid_template("Test Humanoid", 1);
        assert_eq!(creature.name, "Test Humanoid");
        assert_eq!(creature.id, 1);
        assert!(creature.meshes.len() >= 6); // Body, head, 2 arms, 2 legs minimum
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_quadruped_template_structure() {
        let creature = generate_quadruped_template("Test Wolf", 2);
        assert_eq!(creature.name, "Test Wolf");
        assert!(creature.meshes.len() >= 6); // Body, head, 4 legs
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_flying_template_structure() {
        let creature = generate_flying_template("Test Bird", 3);
        assert_eq!(creature.name, "Test Bird");
        assert!(creature.meshes.len() >= 4); // Body, beak, 2 wings
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_slime_template_structure() {
        let creature = generate_slime_template("Test Slime", 4);
        assert_eq!(creature.name, "Test Slime");
        assert!(creature.meshes.len() >= 3); // Body, 2 eyes
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
        assert!(creature.color_tint.is_some());
    }

    #[test]
    fn test_dragon_template_structure() {
        let creature = generate_dragon_template("Test Dragon", 5);
        assert_eq!(creature.name, "Test Dragon");
        assert!(creature.meshes.len() >= 10); // Complex multi-part creature
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_all_templates_validate() {
        for (name, generator) in available_templates() {
            let creature = generator(name, 999);
            assert!(
                creature.validate().is_ok(),
                "Template {} failed validation",
                name
            );
        }
    }

    #[test]
    fn test_available_templates_count() {
        let templates = available_templates();
        assert!(templates.len() >= 5);
    }

    #[test]
    fn test_template_mesh_transform_consistency() {
        for (_, generator) in available_templates() {
            let creature = generator("Test", 1);
            assert_eq!(
                creature.meshes.len(),
                creature.mesh_transforms.len(),
                "Mesh and transform counts must match"
            );
        }
    }
}
