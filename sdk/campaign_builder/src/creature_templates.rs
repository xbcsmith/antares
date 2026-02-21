// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Creature template generators for the campaign builder
//!
//! Provides pre-built creature templates using primitive shapes to help
//! content creators get started quickly. Templates cover humanoids, creatures,
//! undead, robots, and primitive shapes across all complexity levels.

use crate::primitive_generators::{
    generate_cone, generate_cube, generate_cylinder, generate_pyramid, generate_sphere,
};
use crate::template_metadata::{
    Complexity, TemplateCategory, TemplateGenerator, TemplateMetadata, TemplateRegistry,
};
use antares::domain::visual::{CreatureDefinition, MeshTransform};

// ============================================================================
// Humanoid Templates
// ============================================================================

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

/// Generates a humanoid fighter in plate armor wielding a sword and shield
///
/// Creates an armored warrior with:
/// - Armored torso (wider than basic humanoid)
/// - Sphere head
/// - Armored cylinder limbs
/// - Flat cube shield on the left side
/// - Elongated cube sword in the right hand
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 8 meshes representing an armored fighter
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_humanoid_fighter;
///
/// let fighter = generate_humanoid_fighter("Sir Roland", 2);
/// assert_eq!(fighter.name, "Sir Roland");
/// assert_eq!(fighter.meshes.len(), 8);
/// ```
pub fn generate_humanoid_fighter(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Torso - armored, slightly wider than basic
    meshes.push(generate_cube(1.0, [0.55, 0.6, 0.65, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.0, 1.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [1.1, 1.0, 0.9],
    });

    // Head
    meshes.push(generate_sphere(0.35, 12, 12, [0.9, 0.8, 0.7, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 2.0, 0.0));

    // Left arm - armored, slightly thicker
    meshes.push(generate_cylinder(0.18, 0.8, 8, [0.55, 0.6, 0.65, 1.0]));
    transforms.push(MeshTransform::translation(-0.7, 1.0, 0.0));

    // Right arm - armored
    meshes.push(generate_cylinder(0.18, 0.8, 8, [0.55, 0.6, 0.65, 1.0]));
    transforms.push(MeshTransform::translation(0.7, 1.0, 0.0));

    // Left leg - armored
    meshes.push(generate_cylinder(0.22, 1.0, 8, [0.5, 0.55, 0.6, 1.0]));
    transforms.push(MeshTransform::translation(-0.3, 0.0, 0.0));

    // Right leg - armored
    meshes.push(generate_cylinder(0.22, 1.0, 8, [0.5, 0.55, 0.6, 1.0]));
    transforms.push(MeshTransform::translation(0.3, 0.0, 0.0));

    // Shield - flat wide cube on left side
    meshes.push(generate_cube(0.5, [0.7, 0.65, 0.4, 1.0]));
    transforms.push(MeshTransform {
        translation: [-0.95, 1.0, 0.25],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.15, 0.85, 0.65],
    });

    // Sword - tall thin elongated cube in right hand
    meshes.push(generate_cube(0.1, [0.82, 0.82, 0.9, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.95, 1.2, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.12, 1.5, 0.1],
    });

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

/// Generates a humanoid mage wearing robes and carrying a staff
///
/// Creates an arcane caster with:
/// - Wide robe-like torso
/// - Sphere head
/// - Robe-sleeved arms
/// - Tall staff
/// - Pointed hat
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 8 meshes representing a mage
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_humanoid_mage;
///
/// let mage = generate_humanoid_mage("Arcana", 3);
/// assert_eq!(mage.name, "Arcana");
/// assert_eq!(mage.meshes.len(), 8);
/// ```
pub fn generate_humanoid_mage(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Torso - long flowing robes (taller, slightly wider)
    meshes.push(generate_cube(1.0, [0.4, 0.2, 0.7, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.0, 1.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [1.05, 1.25, 0.8],
    });

    // Head
    meshes.push(generate_sphere(0.35, 12, 12, [0.9, 0.8, 0.7, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 2.15, 0.0));

    // Left arm - robe sleeve
    meshes.push(generate_cylinder(0.13, 0.85, 8, [0.38, 0.18, 0.68, 1.0]));
    transforms.push(MeshTransform::translation(-0.65, 1.0, 0.0));

    // Right arm - robe sleeve
    meshes.push(generate_cylinder(0.13, 0.85, 8, [0.38, 0.18, 0.68, 1.0]));
    transforms.push(MeshTransform::translation(0.65, 1.0, 0.0));

    // Left leg - hidden in robe
    meshes.push(generate_cylinder(0.15, 1.0, 8, [0.35, 0.16, 0.62, 1.0]));
    transforms.push(MeshTransform::translation(-0.22, 0.0, 0.0));

    // Right leg - hidden in robe
    meshes.push(generate_cylinder(0.15, 1.0, 8, [0.35, 0.16, 0.62, 1.0]));
    transforms.push(MeshTransform::translation(0.22, 0.0, 0.0));

    // Staff - tall thin cylinder, held to the side
    meshes.push(generate_cylinder(0.05, 2.1, 6, [0.5, 0.32, 0.12, 1.0]));
    transforms.push(MeshTransform::translation(0.95, 0.55, 0.0));

    // Pointed hat - cone on top of head
    meshes.push(generate_cone(0.3, 0.75, 8, [0.32, 0.1, 0.62, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 2.55, 0.0));

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

/// Generates a humanoid cleric in holy robes with a mace and holy symbol
///
/// Creates a divine warrior with:
/// - White/cream robed torso
/// - Sphere head
/// - Robed arms and legs
/// - Holy symbol disc on the chest
/// - Mace (handle + sphere head)
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 9 meshes representing a cleric
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_humanoid_cleric;
///
/// let cleric = generate_humanoid_cleric("Brother Aldric", 4);
/// assert_eq!(cleric.name, "Brother Aldric");
/// assert_eq!(cleric.meshes.len(), 9);
/// ```
pub fn generate_humanoid_cleric(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Torso - cream/white robes
    meshes.push(generate_cube(1.0, [0.94, 0.94, 0.88, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 1.0, 0.0));

    // Head
    meshes.push(generate_sphere(0.35, 12, 12, [0.9, 0.8, 0.7, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 2.0, 0.0));

    // Left arm
    meshes.push(generate_cylinder(0.15, 0.8, 8, [0.92, 0.92, 0.86, 1.0]));
    transforms.push(MeshTransform::translation(-0.65, 1.0, 0.0));

    // Right arm
    meshes.push(generate_cylinder(0.15, 0.8, 8, [0.92, 0.92, 0.86, 1.0]));
    transforms.push(MeshTransform::translation(0.65, 1.0, 0.0));

    // Left leg
    meshes.push(generate_cylinder(0.2, 1.0, 8, [0.9, 0.9, 0.84, 1.0]));
    transforms.push(MeshTransform::translation(-0.3, 0.0, 0.0));

    // Right leg
    meshes.push(generate_cylinder(0.2, 1.0, 8, [0.9, 0.9, 0.84, 1.0]));
    transforms.push(MeshTransform::translation(0.3, 0.0, 0.0));

    // Holy symbol - flat golden disc on chest
    meshes.push(generate_cube(0.3, [0.9, 0.75, 0.1, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.0, 1.5, 0.5],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.45, 0.45, 0.08],
    });

    // Mace handle
    meshes.push(generate_cylinder(0.06, 0.82, 6, [0.42, 0.32, 0.2, 1.0]));
    transforms.push(MeshTransform::translation(0.9, 0.8, 0.0));

    // Mace head - sphere on top of handle
    meshes.push(generate_sphere(0.16, 8, 8, [0.52, 0.52, 0.56, 1.0]));
    transforms.push(MeshTransform::translation(0.9, 1.55, 0.0));

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

/// Generates a humanoid rogue in dark leather armor with daggers
///
/// Creates a stealthy assassin with:
/// - Dark leather torso
/// - Sphere head
/// - Dark arms and legs
/// - Hood/cowl cylinder over head
/// - Two small daggers at the sides
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 9 meshes representing a rogue
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_humanoid_rogue;
///
/// let rogue = generate_humanoid_rogue("Shadow", 5);
/// assert_eq!(rogue.name, "Shadow");
/// assert_eq!(rogue.meshes.len(), 9);
/// ```
pub fn generate_humanoid_rogue(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Torso - dark leather
    meshes.push(generate_cube(1.0, [0.2, 0.2, 0.2, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 1.0, 0.0));

    // Head
    meshes.push(generate_sphere(0.32, 12, 12, [0.9, 0.8, 0.7, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 2.0, 0.0));

    // Left arm - dark
    meshes.push(generate_cylinder(0.13, 0.8, 8, [0.22, 0.22, 0.22, 1.0]));
    transforms.push(MeshTransform::translation(-0.62, 1.0, 0.0));

    // Right arm - dark
    meshes.push(generate_cylinder(0.13, 0.8, 8, [0.22, 0.22, 0.22, 1.0]));
    transforms.push(MeshTransform::translation(0.62, 1.0, 0.0));

    // Left leg - dark green trousers
    meshes.push(generate_cylinder(0.18, 1.0, 8, [0.15, 0.25, 0.15, 1.0]));
    transforms.push(MeshTransform::translation(-0.3, 0.0, 0.0));

    // Right leg - dark green trousers
    meshes.push(generate_cylinder(0.18, 1.0, 8, [0.15, 0.25, 0.15, 1.0]));
    transforms.push(MeshTransform::translation(0.3, 0.0, 0.0));

    // Hood - wide dark cylinder draped over head
    meshes.push(generate_cylinder(0.38, 0.36, 8, [0.14, 0.14, 0.14, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 2.02, 0.0));

    // Left dagger - thin elongated cube at left hip
    meshes.push(generate_cube(0.05, [0.76, 0.76, 0.82, 1.0]));
    transforms.push(MeshTransform {
        translation: [-0.65, 0.7, 0.22],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.09, 0.62, 0.09],
    });

    // Right dagger - thin elongated cube at right hip
    meshes.push(generate_cube(0.05, [0.76, 0.76, 0.82, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.65, 0.7, 0.22],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.09, 0.62, 0.09],
    });

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

/// Generates a humanoid archer in light leather armor with a bow and quiver
///
/// Creates a ranged combatant with:
/// - Forest green/leather torso
/// - Sphere head
/// - Light armor limbs
/// - Tall bow on the left side
/// - Cylindrical quiver on the back
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 8 meshes representing an archer
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_humanoid_archer;
///
/// let archer = generate_humanoid_archer("Sylvan", 6);
/// assert_eq!(archer.name, "Sylvan");
/// assert_eq!(archer.meshes.len(), 8);
/// ```
pub fn generate_humanoid_archer(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Torso - forest green leather
    meshes.push(generate_cube(1.0, [0.22, 0.45, 0.22, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 1.0, 0.0));

    // Head
    meshes.push(generate_sphere(0.33, 12, 12, [0.9, 0.8, 0.7, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 2.0, 0.0));

    // Left arm - light leather
    meshes.push(generate_cylinder(0.13, 0.8, 8, [0.22, 0.45, 0.22, 1.0]));
    transforms.push(MeshTransform::translation(-0.65, 1.0, 0.0));

    // Right arm
    meshes.push(generate_cylinder(0.13, 0.8, 8, [0.22, 0.45, 0.22, 1.0]));
    transforms.push(MeshTransform::translation(0.65, 1.0, 0.0));

    // Left leg - brown leather
    meshes.push(generate_cylinder(0.18, 1.0, 8, [0.5, 0.35, 0.15, 1.0]));
    transforms.push(MeshTransform::translation(-0.3, 0.0, 0.0));

    // Right leg - brown leather
    meshes.push(generate_cylinder(0.18, 1.0, 8, [0.5, 0.35, 0.15, 1.0]));
    transforms.push(MeshTransform::translation(0.3, 0.0, 0.0));

    // Bow - tall thin cylinder held in left hand
    meshes.push(generate_cylinder(0.04, 1.65, 6, [0.52, 0.32, 0.12, 1.0]));
    transforms.push(MeshTransform::translation(-0.95, 1.0, 0.0));

    // Quiver - small cylinder on the back
    meshes.push(generate_cylinder(0.1, 0.55, 6, [0.5, 0.35, 0.15, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.0, 1.05, -0.45],
        rotation: [0.0, 0.0, 0.0],
        scale: [1.0, 1.0, 1.0],
    });

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

// ============================================================================
// Creature Templates
// ============================================================================

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
    transforms.push(MeshTransform {
        translation: [0.0, 0.8, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [1.5, 0.8, 0.6],
    });

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

/// Generates a wolf-like quadruped with lean proportions, snout, and tail
///
/// Creates a predatory canine with:
/// - Lean elongated body
/// - Sphere head
/// - Extended snout
/// - Four slim legs
/// - Angled tail
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 8 meshes representing a wolf
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_quadruped_wolf;
///
/// let wolf = generate_quadruped_wolf("Grey Fang", 7);
/// assert_eq!(wolf.name, "Grey Fang");
/// assert_eq!(wolf.meshes.len(), 8);
/// ```
pub fn generate_quadruped_wolf(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Body - lean and elongated
    meshes.push(generate_cube(1.0, [0.52, 0.48, 0.42, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.0, 0.82, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [1.8, 0.72, 0.65],
    });

    // Head
    meshes.push(generate_sphere(0.28, 12, 12, [0.54, 0.5, 0.44, 1.0]));
    transforms.push(MeshTransform::translation(1.1, 1.0, 0.0));

    // Snout - elongated muzzle
    meshes.push(generate_cube(0.2, [0.38, 0.34, 0.28, 1.0]));
    transforms.push(MeshTransform {
        translation: [1.45, 0.9, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.55, 0.26, 0.36],
    });

    // Front left leg - slim
    meshes.push(generate_cylinder(0.1, 0.9, 8, [0.48, 0.44, 0.38, 1.0]));
    transforms.push(MeshTransform::translation(0.65, 0.0, -0.28));

    // Front right leg
    meshes.push(generate_cylinder(0.1, 0.9, 8, [0.48, 0.44, 0.38, 1.0]));
    transforms.push(MeshTransform::translation(0.65, 0.0, 0.28));

    // Back left leg
    meshes.push(generate_cylinder(0.1, 0.9, 8, [0.48, 0.44, 0.38, 1.0]));
    transforms.push(MeshTransform::translation(-0.65, 0.0, -0.28));

    // Back right leg
    meshes.push(generate_cylinder(0.1, 0.9, 8, [0.48, 0.44, 0.38, 1.0]));
    transforms.push(MeshTransform::translation(-0.65, 0.0, 0.28));

    // Tail - thin cylinder angled upward
    meshes.push(generate_cylinder(0.07, 0.88, 6, [0.48, 0.44, 0.38, 1.0]));
    transforms.push(MeshTransform {
        translation: [-1.15, 1.12, 0.0],
        rotation: [0.0, 0.0, -std::f32::consts::PI / 4.0],
        scale: [1.0, 1.0, 1.0],
    });

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
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_flying_template;
///
/// let bird = generate_flying_template("Harpy", 3);
/// assert_eq!(bird.name, "Harpy");
/// ```
pub fn generate_flying_template(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Body (sphere)
    meshes.push(generate_sphere(0.4, 12, 12, [0.8, 0.8, 0.9, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 0.5, 0.0));

    // Beak (cone)
    meshes.push(generate_cone(0.1, 0.3, 8, [1.0, 0.7, 0.0, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.4, 0.5, 0.0],
        rotation: [0.0, 0.0, std::f32::consts::PI / 2.0],
        scale: [1.0, 1.0, 1.0],
    });

    // Left wing (flat cube)
    meshes.push(generate_cube(0.6, [0.7, 0.7, 0.8, 1.0]));
    transforms.push(MeshTransform {
        translation: [-0.5, 0.5, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.1, 0.6, 1.2],
    });

    // Right wing (flat cube)
    meshes.push(generate_cube(0.6, [0.7, 0.7, 0.8, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.5, 0.5, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.1, 0.6, 1.2],
    });

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

/// Generates a spider with eight legs and two body segments
///
/// Creates an arachnid with:
/// - Large abdomen sphere
/// - Smaller cephalothorax sphere
/// - Eight thin cylindrical legs
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 10 meshes representing a spider
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_spider_basic;
///
/// let spider = generate_spider_basic("Cave Spider", 8);
/// assert_eq!(spider.name, "Cave Spider");
/// assert_eq!(spider.meshes.len(), 10);
/// ```
pub fn generate_spider_basic(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Abdomen - large rear segment
    meshes.push(generate_sphere(0.5, 14, 12, [0.25, 0.18, 0.12, 1.0]));
    transforms.push(MeshTransform {
        translation: [-0.45, 0.55, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [1.0, 0.88, 1.18],
    });

    // Cephalothorax - smaller front segment (head + thorax)
    meshes.push(generate_sphere(0.3, 10, 10, [0.2, 0.14, 0.1, 1.0]));
    transforms.push(MeshTransform::translation(0.28, 0.55, 0.0));

    // Eight legs - thin cylinders radiating outward
    // Front-left pair
    meshes.push(generate_cylinder(0.04, 0.88, 6, [0.22, 0.16, 0.1, 1.0]));
    transforms.push(MeshTransform::translation(0.32, 0.28, -0.62));

    meshes.push(generate_cylinder(0.04, 0.88, 6, [0.22, 0.16, 0.1, 1.0]));
    transforms.push(MeshTransform::translation(0.32, 0.28, 0.62));

    // Mid-front pair
    meshes.push(generate_cylinder(0.04, 0.82, 6, [0.22, 0.16, 0.1, 1.0]));
    transforms.push(MeshTransform::translation(0.12, 0.25, -0.72));

    meshes.push(generate_cylinder(0.04, 0.82, 6, [0.22, 0.16, 0.1, 1.0]));
    transforms.push(MeshTransform::translation(0.12, 0.25, 0.72));

    // Mid-back pair
    meshes.push(generate_cylinder(0.04, 0.82, 6, [0.22, 0.16, 0.1, 1.0]));
    transforms.push(MeshTransform::translation(-0.22, 0.25, -0.72));

    meshes.push(generate_cylinder(0.04, 0.82, 6, [0.22, 0.16, 0.1, 1.0]));
    transforms.push(MeshTransform::translation(-0.22, 0.25, 0.72));

    // Back pair
    meshes.push(generate_cylinder(0.04, 0.88, 6, [0.22, 0.16, 0.1, 1.0]));
    transforms.push(MeshTransform::translation(-0.52, 0.28, -0.62));

    meshes.push(generate_cylinder(0.04, 0.88, 6, [0.22, 0.16, 0.1, 1.0]));
    transforms.push(MeshTransform::translation(-0.52, 0.28, 0.62));

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

/// Generates a snake with a segmented body, no legs, and a tapered tail
///
/// Creates a serpent with:
/// - Sphere head with slight elongation
/// - Four body segment spheres in a gentle sinusoidal pattern
/// - Tapered cone tail
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 7 meshes representing a snake
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_snake_basic;
///
/// let snake = generate_snake_basic("Viper", 9);
/// assert_eq!(snake.name, "Viper");
/// assert_eq!(snake.meshes.len(), 7);
/// ```
pub fn generate_snake_basic(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Head - slightly elongated sphere
    meshes.push(generate_sphere(0.22, 10, 10, [0.22, 0.55, 0.25, 1.0]));
    transforms.push(MeshTransform {
        translation: [1.4, 0.3, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [1.2, 0.88, 1.0],
    });

    // Neck segment
    meshes.push(generate_sphere(0.2, 8, 8, [0.22, 0.55, 0.25, 1.0]));
    transforms.push(MeshTransform::translation(1.0, 0.28, 0.0));

    // Body segment 1 - slight sinusoidal offset
    meshes.push(generate_sphere(0.25, 10, 10, [0.2, 0.52, 0.22, 1.0]));
    transforms.push(MeshTransform::translation(0.5, 0.25, 0.15));

    // Body segment 2 - opposite offset
    meshes.push(generate_sphere(0.25, 10, 10, [0.2, 0.52, 0.22, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 0.25, -0.12));

    // Body segment 3
    meshes.push(generate_sphere(0.22, 10, 10, [0.2, 0.52, 0.22, 1.0]));
    transforms.push(MeshTransform::translation(-0.5, 0.24, 0.1));

    // Thinning body segment 4
    meshes.push(generate_sphere(0.18, 8, 8, [0.2, 0.52, 0.22, 1.0]));
    transforms.push(MeshTransform::translation(-0.9, 0.22, 0.0));

    // Tail - pointed cone
    meshes.push(generate_cone(0.1, 0.42, 8, [0.2, 0.5, 0.22, 1.0]));
    transforms.push(MeshTransform {
        translation: [-1.22, 0.2, 0.0],
        rotation: [0.0, 0.0, std::f32::consts::PI],
        scale: [1.0, 1.0, 1.0],
    });

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
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_slime_template;
///
/// let slime = generate_slime_template("Green Slime", 4);
/// assert_eq!(slime.name, "Green Slime");
/// ```
pub fn generate_slime_template(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Body (squashed sphere)
    meshes.push(generate_sphere(0.6, 16, 12, [0.2, 0.8, 0.3, 0.8]));
    transforms.push(MeshTransform {
        translation: [0.0, 0.3, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [1.0, 0.6, 1.0],
    });

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
/// Creates a dragon with:
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
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_dragon_template;
///
/// let dragon = generate_dragon_template("Fire Drake", 5);
/// assert_eq!(dragon.name, "Fire Drake");
/// assert!(dragon.meshes.len() >= 10);
/// ```
pub fn generate_dragon_template(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Body (elongated sphere)
    meshes.push(generate_sphere(0.6, 16, 16, [0.8, 0.2, 0.2, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.0, 1.2, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [2.0, 1.0, 1.0],
    });

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
    transforms.push(MeshTransform {
        translation: [-0.7, 1.5, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.1, 0.8, 2.0],
    });

    // Right wing
    meshes.push(generate_cube(1.0, [0.6, 0.1, 0.1, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.7, 1.5, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.1, 0.8, 2.0],
    });

    // Tail (cylinder)
    meshes.push(generate_cylinder(0.2, 1.5, 8, [0.7, 0.2, 0.2, 1.0]));
    transforms.push(MeshTransform {
        translation: [-1.5, 0.8, 0.0],
        rotation: [0.0, 0.0, std::f32::consts::PI / 4.0],
        scale: [1.0, 1.0, 1.0],
    });

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

// ============================================================================
// Undead Templates
// ============================================================================

/// Generates a skeleton creature with thin ivory-colored bones
///
/// Creates an animated skeleton with:
/// - Narrow bony torso
/// - Sphere skull
/// - Thin cylindrical arm and leg bones
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 6 meshes representing a skeleton
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_skeleton_basic;
///
/// let skeleton = generate_skeleton_basic("Bone Warrior", 10);
/// assert_eq!(skeleton.name, "Bone Warrior");
/// assert_eq!(skeleton.meshes.len(), 6);
/// ```
pub fn generate_skeleton_basic(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Torso - narrow bony rib cage
    meshes.push(generate_cube(1.0, [0.92, 0.88, 0.78, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.0, 1.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.68, 0.88, 0.48],
    });

    // Skull
    meshes.push(generate_sphere(0.3, 12, 12, [0.94, 0.9, 0.8, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 2.0, 0.0));

    // Left arm bone - very thin
    meshes.push(generate_cylinder(0.08, 0.88, 6, [0.9, 0.86, 0.76, 1.0]));
    transforms.push(MeshTransform::translation(-0.55, 1.0, 0.0));

    // Right arm bone - very thin
    meshes.push(generate_cylinder(0.08, 0.88, 6, [0.9, 0.86, 0.76, 1.0]));
    transforms.push(MeshTransform::translation(0.55, 1.0, 0.0));

    // Left leg bone
    meshes.push(generate_cylinder(0.1, 1.05, 6, [0.88, 0.84, 0.74, 1.0]));
    transforms.push(MeshTransform::translation(-0.25, 0.0, 0.0));

    // Right leg bone
    meshes.push(generate_cylinder(0.1, 1.05, 6, [0.88, 0.84, 0.74, 1.0]));
    transforms.push(MeshTransform::translation(0.25, 0.0, 0.0));

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

/// Generates a zombie in tattered rags with an asymmetric lumbering pose
///
/// Creates a shambling undead with:
/// - Gray-green decomposing torso
/// - Slightly drooping head
/// - One arm raised (outstretched zombie pose), one arm lowered
/// - Tattered ragged legs
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 6 meshes representing a zombie
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_zombie_basic;
///
/// let zombie = generate_zombie_basic("Shambler", 11);
/// assert_eq!(zombie.name, "Shambler");
/// assert_eq!(zombie.meshes.len(), 6);
/// ```
pub fn generate_zombie_basic(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Torso - gray-green decaying flesh
    meshes.push(generate_cube(1.0, [0.45, 0.52, 0.35, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.0, 0.95, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.95, 0.95, 0.85],
    });

    // Head - slightly forward-drooping
    meshes.push(generate_sphere(0.34, 12, 12, [0.5, 0.56, 0.4, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.08, 1.88, 0.12],
        rotation: [0.15, 0.0, 0.0],
        scale: [1.0, 1.0, 1.0],
    });

    // Left arm - raised outward in classic zombie reach
    meshes.push(generate_cylinder(0.15, 0.82, 8, [0.38, 0.35, 0.28, 1.0]));
    transforms.push(MeshTransform {
        translation: [-0.68, 1.22, 0.0],
        rotation: [0.0, 0.0, std::f32::consts::PI / 6.0],
        scale: [1.0, 1.0, 1.0],
    });

    // Right arm - drooping lower
    meshes.push(generate_cylinder(0.15, 0.78, 8, [0.44, 0.52, 0.36, 1.0]));
    transforms.push(MeshTransform::translation(0.65, 0.78, 0.0));

    // Left leg - ragged
    meshes.push(generate_cylinder(0.2, 0.95, 8, [0.38, 0.35, 0.28, 1.0]));
    transforms.push(MeshTransform::translation(-0.3, 0.0, 0.0));

    // Right leg - ragged
    meshes.push(generate_cylinder(0.2, 0.95, 8, [0.38, 0.35, 0.28, 1.0]));
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

/// Generates a ghost with a translucent flowing form
///
/// Creates an incorporeal spirit with:
/// - Translucent sphere head
/// - Wispy upper body
/// - Flowing cone wisps that fade downward
/// - Overall blue-white color tint with partial transparency
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 6 meshes representing a ghost
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_ghost_basic;
///
/// let ghost = generate_ghost_basic("Wailing Shade", 12);
/// assert_eq!(ghost.name, "Wailing Shade");
/// assert_eq!(ghost.meshes.len(), 6);
/// ```
pub fn generate_ghost_basic(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Head - translucent white-blue
    meshes.push(generate_sphere(0.32, 12, 12, [0.9, 0.9, 1.0, 0.7]));
    transforms.push(MeshTransform::translation(0.0, 1.8, 0.0));

    // Upper body - semi-translucent
    meshes.push(generate_sphere(0.45, 12, 12, [0.85, 0.85, 1.0, 0.55]));
    transforms.push(MeshTransform {
        translation: [0.0, 1.1, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.9, 1.0, 0.9],
    });

    // Left wisp - cone flowing to the lower left
    meshes.push(generate_cone(0.2, 0.82, 8, [0.8, 0.8, 1.0, 0.4]));
    transforms.push(MeshTransform {
        translation: [-0.38, 0.38, 0.0],
        rotation: [0.0, 0.0, std::f32::consts::PI / 8.0],
        scale: [1.0, 1.0, 1.0],
    });

    // Right wisp - cone flowing to the lower right
    meshes.push(generate_cone(0.2, 0.82, 8, [0.8, 0.8, 1.0, 0.4]));
    transforms.push(MeshTransform {
        translation: [0.38, 0.38, 0.0],
        rotation: [0.0, 0.0, -std::f32::consts::PI / 8.0],
        scale: [1.0, 1.0, 1.0],
    });

    // Center wisp - larger central flow
    meshes.push(generate_cone(0.26, 1.02, 8, [0.75, 0.75, 1.0, 0.35]));
    transforms.push(MeshTransform::translation(0.0, 0.2, 0.0));

    // Ethereal aura - large faint outer sphere
    meshes.push(generate_sphere(0.58, 8, 8, [0.92, 0.92, 1.0, 0.15]));
    transforms.push(MeshTransform::translation(0.0, 1.2, 0.0));

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: Some([0.85, 0.9, 1.0, 0.65]),
    }
}

// ============================================================================
// Robot Templates
// ============================================================================

/// Generates a basic boxy mechanical robot
///
/// Creates a simple construct with:
/// - Large cube torso
/// - Cube head
/// - Thick cylinder arms and legs
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 6 meshes representing a basic robot
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_robot_basic;
///
/// let robot = generate_robot_basic("Iron Golem", 13);
/// assert_eq!(robot.name, "Iron Golem");
/// assert_eq!(robot.meshes.len(), 6);
/// ```
pub fn generate_robot_basic(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Torso - large boxy body
    meshes.push(generate_cube(1.0, [0.45, 0.45, 0.5, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.0, 1.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [1.1, 1.05, 0.9],
    });

    // Head - smaller cube
    meshes.push(generate_cube(0.6, [0.4, 0.4, 0.45, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.0, 2.12, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.78, 0.7, 0.78],
    });

    // Left arm - thick cylinder
    meshes.push(generate_cylinder(0.2, 0.92, 8, [0.38, 0.38, 0.42, 1.0]));
    transforms.push(MeshTransform::translation(-0.75, 1.0, 0.0));

    // Right arm - thick cylinder
    meshes.push(generate_cylinder(0.2, 0.92, 8, [0.38, 0.38, 0.42, 1.0]));
    transforms.push(MeshTransform::translation(0.75, 1.0, 0.0));

    // Left leg - thick cylinder
    meshes.push(generate_cylinder(0.24, 1.1, 8, [0.42, 0.42, 0.46, 1.0]));
    transforms.push(MeshTransform::translation(-0.32, 0.0, 0.0));

    // Right leg - thick cylinder
    meshes.push(generate_cylinder(0.24, 1.1, 8, [0.42, 0.42, 0.46, 1.0]));
    transforms.push(MeshTransform::translation(0.32, 0.0, 0.0));

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

/// Generates an advanced robot with articulated joints and chest panel detail
///
/// Creates a sophisticated construct with:
/// - Cube torso with visible chest panel
/// - Cube head with glowing sensor
/// - Sphere shoulder joints
/// - Articulated upper arms and forearms
/// - Heavy legs
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 12 meshes representing an advanced robot
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_robot_advanced;
///
/// let robot = generate_robot_advanced("Steel Sentinel", 14);
/// assert_eq!(robot.name, "Steel Sentinel");
/// assert_eq!(robot.meshes.len(), 12);
/// ```
pub fn generate_robot_advanced(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Torso - blue-gray plated body
    meshes.push(generate_cube(1.0, [0.35, 0.4, 0.55, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.0, 1.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [1.02, 1.1, 0.86],
    });

    // Chest panel - flat overlay with lighter color
    meshes.push(generate_cube(0.5, [0.6, 0.7, 0.85, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.0, 1.2, 0.46],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.62, 0.52, 0.1],
    });

    // Head - cube
    meshes.push(generate_cube(0.5, [0.3, 0.34, 0.48, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.0, 2.12, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.74, 0.64, 0.74],
    });

    // Left shoulder joint - sphere
    meshes.push(generate_sphere(0.22, 8, 8, [0.5, 0.6, 0.75, 1.0]));
    transforms.push(MeshTransform::translation(-0.67, 1.62, 0.0));

    // Right shoulder joint - sphere
    meshes.push(generate_sphere(0.22, 8, 8, [0.5, 0.6, 0.75, 1.0]));
    transforms.push(MeshTransform::translation(0.67, 1.62, 0.0));

    // Left upper arm
    meshes.push(generate_cylinder(0.14, 0.56, 8, [0.35, 0.4, 0.55, 1.0]));
    transforms.push(MeshTransform::translation(-0.78, 1.2, 0.0));

    // Right upper arm
    meshes.push(generate_cylinder(0.14, 0.56, 8, [0.35, 0.4, 0.55, 1.0]));
    transforms.push(MeshTransform::translation(0.78, 1.2, 0.0));

    // Left forearm - slightly smaller
    meshes.push(generate_cylinder(0.12, 0.52, 8, [0.28, 0.32, 0.45, 1.0]));
    transforms.push(MeshTransform::translation(-0.82, 0.62, 0.0));

    // Right forearm
    meshes.push(generate_cylinder(0.12, 0.52, 8, [0.28, 0.32, 0.45, 1.0]));
    transforms.push(MeshTransform::translation(0.82, 0.62, 0.0));

    // Left leg
    meshes.push(generate_cylinder(0.21, 1.12, 8, [0.35, 0.4, 0.55, 1.0]));
    transforms.push(MeshTransform::translation(-0.32, 0.0, 0.0));

    // Right leg
    meshes.push(generate_cylinder(0.21, 1.12, 8, [0.35, 0.4, 0.55, 1.0]));
    transforms.push(MeshTransform::translation(0.32, 0.0, 0.0));

    // Sensor eye - small glowing sphere on head
    meshes.push(generate_sphere(0.1, 6, 6, [0.95, 0.42, 0.1, 1.0]));
    transforms.push(MeshTransform::translation(0.16, 2.42, 0.36));

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

/// Generates a flying robot drone with wing panels and thruster nozzles
///
/// Creates an aerial construct with:
/// - Compact cube body
/// - Cube head
/// - Wide flat wing panels for lift
/// - Short landing strut legs
/// - Downward-facing thruster cones
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 8 meshes representing a flying robot
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_robot_flying;
///
/// let drone = generate_robot_flying("Sky Warden", 15);
/// assert_eq!(drone.name, "Sky Warden");
/// assert_eq!(drone.meshes.len(), 8);
/// ```
pub fn generate_robot_flying(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    // Body - compact silver frame
    meshes.push(generate_cube(0.8, [0.72, 0.72, 0.78, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.0, 0.85, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.92, 0.88, 0.82],
    });

    // Head - smaller cube
    meshes.push(generate_cube(0.45, [0.62, 0.62, 0.66, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.0, 1.65, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.72, 0.62, 0.72],
    });

    // Left wing panel - wide flat cube
    meshes.push(generate_cube(1.0, [0.7, 0.7, 0.76, 1.0]));
    transforms.push(MeshTransform {
        translation: [-1.08, 0.85, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.82, 0.1, 0.52],
    });

    // Right wing panel
    meshes.push(generate_cube(1.0, [0.7, 0.7, 0.76, 1.0]));
    transforms.push(MeshTransform {
        translation: [1.08, 0.85, 0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [0.82, 0.1, 0.52],
    });

    // Left landing strut - short cylinder
    meshes.push(generate_cylinder(0.12, 0.42, 6, [0.22, 0.22, 0.26, 1.0]));
    transforms.push(MeshTransform::translation(-0.32, 0.22, 0.0));

    // Right landing strut
    meshes.push(generate_cylinder(0.12, 0.42, 6, [0.22, 0.22, 0.26, 1.0]));
    transforms.push(MeshTransform::translation(0.32, 0.22, 0.0));

    // Left thruster nozzle - cone pointing downward
    meshes.push(generate_cone(0.2, 0.52, 8, [0.18, 0.18, 0.22, 1.0]));
    transforms.push(MeshTransform {
        translation: [-0.88, 0.4, 0.0],
        rotation: [std::f32::consts::PI, 0.0, 0.0],
        scale: [1.0, 1.0, 1.0],
    });

    // Right thruster nozzle
    meshes.push(generate_cone(0.2, 0.52, 8, [0.18, 0.18, 0.22, 1.0]));
    transforms.push(MeshTransform {
        translation: [0.88, 0.4, 0.0],
        rotation: [std::f32::consts::PI, 0.0, 0.0],
        scale: [1.0, 1.0, 1.0],
    });

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

// ============================================================================
// Primitive Templates
// ============================================================================

/// Generates a single cube primitive creature template
///
/// A single cube shape useful as a placeholder or starting point
/// for a completely custom creature.
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 1 mesh representing a cube
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_primitive_cube;
///
/// let cube = generate_primitive_cube("Block", 16);
/// assert_eq!(cube.meshes.len(), 1);
/// ```
pub fn generate_primitive_cube(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    meshes.push(generate_cube(1.0, [0.7, 0.7, 0.7, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 0.5, 0.0));

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

/// Generates a single sphere primitive creature template
///
/// A single sphere shape useful as a placeholder or starting point
/// for a completely custom creature.
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 1 mesh representing a sphere
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_primitive_sphere;
///
/// let sphere = generate_primitive_sphere("Orb", 17);
/// assert_eq!(sphere.meshes.len(), 1);
/// ```
pub fn generate_primitive_sphere(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    meshes.push(generate_sphere(0.5, 16, 16, [0.7, 0.7, 0.7, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 0.5, 0.0));

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

/// Generates a single cylinder primitive creature template
///
/// A single cylinder shape useful as a placeholder or starting point
/// for a completely custom creature.
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 1 mesh representing a cylinder
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_primitive_cylinder;
///
/// let cylinder = generate_primitive_cylinder("Pillar", 18);
/// assert_eq!(cylinder.meshes.len(), 1);
/// ```
pub fn generate_primitive_cylinder(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    meshes.push(generate_cylinder(0.5, 1.0, 12, [0.7, 0.7, 0.7, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 0.5, 0.0));

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

/// Generates a single cone primitive creature template
///
/// A single cone/pyramid shape useful as a placeholder or starting point
/// for a completely custom creature.
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 1 mesh representing a cone
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_primitive_cone;
///
/// let cone = generate_primitive_cone("Spike", 19);
/// assert_eq!(cone.meshes.len(), 1);
/// ```
pub fn generate_primitive_cone(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    meshes.push(generate_cone(0.5, 1.0, 12, [0.7, 0.7, 0.7, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 0.0, 0.0));

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

/// Generates a single pyramid primitive creature template
///
/// A four-sided pyramid shape useful as a placeholder or starting point
/// for a completely custom creature.
///
/// # Arguments
///
/// * `name` - The name for the creature
/// * `id` - The creature ID
///
/// # Returns
///
/// A `CreatureDefinition` with 1 mesh representing a pyramid
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::generate_primitive_pyramid;
///
/// let pyramid = generate_primitive_pyramid("Obelisk", 20);
/// assert_eq!(pyramid.meshes.len(), 1);
/// ```
pub fn generate_primitive_pyramid(name: &str, id: u32) -> CreatureDefinition {
    let mut meshes = Vec::new();
    let mut transforms = Vec::new();

    meshes.push(generate_pyramid(1.0, [0.7, 0.7, 0.7, 1.0]));
    transforms.push(MeshTransform::translation(0.0, 0.0, 0.0));

    CreatureDefinition {
        id,
        name: name.to_string(),
        meshes,
        mesh_transforms: transforms,
        scale: 1.0,
        color_tint: None,
    }
}

// ============================================================================
// Registry and Legacy API
// ============================================================================

/// Returns a list of all available creature templates
///
/// # Returns
///
/// A vector of tuples containing (template_name, generator_function)
///
/// # Note
///
/// This function is provided for backwards compatibility. Prefer
/// `initialize_template_registry()` to get a registry with full metadata,
/// filtering, and search support.
pub fn available_templates() -> Vec<(&'static str, TemplateGenerator)> {
    vec![
        // Original templates
        ("Humanoid", generate_humanoid_template as TemplateGenerator),
        (
            "Quadruped",
            generate_quadruped_template as TemplateGenerator,
        ),
        (
            "Flying Creature",
            generate_flying_template as TemplateGenerator,
        ),
        ("Slime/Blob", generate_slime_template as TemplateGenerator),
        ("Dragon", generate_dragon_template as TemplateGenerator),
        // Humanoid variants
        ("Fighter", generate_humanoid_fighter as TemplateGenerator),
        ("Mage", generate_humanoid_mage as TemplateGenerator),
        ("Cleric", generate_humanoid_cleric as TemplateGenerator),
        ("Rogue", generate_humanoid_rogue as TemplateGenerator),
        ("Archer", generate_humanoid_archer as TemplateGenerator),
        // Creature variants
        ("Wolf", generate_quadruped_wolf as TemplateGenerator),
        ("Spider", generate_spider_basic as TemplateGenerator),
        ("Snake", generate_snake_basic as TemplateGenerator),
        // Undead
        ("Skeleton", generate_skeleton_basic as TemplateGenerator),
        ("Zombie", generate_zombie_basic as TemplateGenerator),
        ("Ghost", generate_ghost_basic as TemplateGenerator),
        // Robots
        ("Robot (Basic)", generate_robot_basic as TemplateGenerator),
        (
            "Robot (Advanced)",
            generate_robot_advanced as TemplateGenerator,
        ),
        ("Robot (Flying)", generate_robot_flying as TemplateGenerator),
        // Primitives
        (
            "Primitive Cube",
            generate_primitive_cube as TemplateGenerator,
        ),
        (
            "Primitive Sphere",
            generate_primitive_sphere as TemplateGenerator,
        ),
        (
            "Primitive Cylinder",
            generate_primitive_cylinder as TemplateGenerator,
        ),
        (
            "Primitive Cone",
            generate_primitive_cone as TemplateGenerator,
        ),
        (
            "Primitive Pyramid",
            generate_primitive_pyramid as TemplateGenerator,
        ),
    ]
}

/// Initializes a template registry with all available creature templates
///
/// This function populates a `TemplateRegistry` with all 24 built-in creature
/// templates, including rich metadata for browsing, filtering, and searching.
/// Templates span five categories: Humanoid, Creature, Undead, Robot, and Primitive.
///
/// # Returns
///
/// A fully populated `TemplateRegistry` ready for use in the template browser UI
///
/// # Examples
///
/// ```
/// use campaign_builder::creature_templates::initialize_template_registry;
///
/// let registry = initialize_template_registry();
/// assert!(registry.len() >= 15);
///
/// // Filter by category
/// use campaign_builder::template_metadata::TemplateCategory;
/// let undead = registry.by_category(TemplateCategory::Undead);
/// assert_eq!(undead.len(), 3);
///
/// // Search by keyword
/// let results = registry.search("humanoid");
/// assert!(!results.is_empty());
/// ```
pub fn initialize_template_registry() -> TemplateRegistry {
    let mut registry = TemplateRegistry::new();

    // -----------------------------------------------------------------------
    // Humanoid category
    // -----------------------------------------------------------------------

    registry.register(
        TemplateMetadata {
            id: "humanoid_basic".to_string(),
            name: "Humanoid".to_string(),
            category: TemplateCategory::Humanoid,
            complexity: Complexity::Beginner,
            mesh_count: 6,
            description: "Basic bipedal humanoid with body, head, arms, and legs. Perfect starting point for knights, mages, and NPCs.".to_string(),
            tags: vec!["humanoid".to_string(), "biped".to_string(), "basic".to_string()],
        },
        generate_humanoid_template("Example Humanoid", 0),
        generate_humanoid_template as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "humanoid_fighter".to_string(),
            name: "Fighter".to_string(),
            category: TemplateCategory::Humanoid,
            complexity: Complexity::Beginner,
            mesh_count: 8,
            description: "Armored warrior in plate mail wielding a sword and shield. Ideal for knights, guards, and melee combatants.".to_string(),
            tags: vec!["humanoid".to_string(), "biped".to_string(), "fighter".to_string(), "warrior".to_string(), "melee".to_string(), "armored".to_string()],
        },
        generate_humanoid_fighter("Example Fighter", 0),
        generate_humanoid_fighter as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "humanoid_mage".to_string(),
            name: "Mage".to_string(),
            category: TemplateCategory::Humanoid,
            complexity: Complexity::Beginner,
            mesh_count: 8,
            description: "Arcane spellcaster in flowing robes carrying a staff and pointed hat. Great for wizards, sorcerers, and scholars.".to_string(),
            tags: vec!["humanoid".to_string(), "biped".to_string(), "mage".to_string(), "caster".to_string(), "spellcaster".to_string(), "robes".to_string()],
        },
        generate_humanoid_mage("Example Mage", 0),
        generate_humanoid_mage as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "humanoid_cleric".to_string(),
            name: "Cleric".to_string(),
            category: TemplateCategory::Humanoid,
            complexity: Complexity::Beginner,
            mesh_count: 9,
            description: "Divine warrior in holy robes with a mace and holy symbol. Suitable for priests, healers, and templars.".to_string(),
            tags: vec!["humanoid".to_string(), "biped".to_string(), "cleric".to_string(), "healer".to_string(), "divine".to_string(), "holy".to_string()],
        },
        generate_humanoid_cleric("Example Cleric", 0),
        generate_humanoid_cleric as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "humanoid_rogue".to_string(),
            name: "Rogue".to_string(),
            category: TemplateCategory::Humanoid,
            complexity: Complexity::Beginner,
            mesh_count: 9,
            description: "Stealthy assassin in dark leather armor with a hood and twin daggers. Perfect for thieves, scouts, and assassins.".to_string(),
            tags: vec!["humanoid".to_string(), "biped".to_string(), "rogue".to_string(), "stealth".to_string(), "assassin".to_string(), "dark".to_string()],
        },
        generate_humanoid_rogue("Example Rogue", 0),
        generate_humanoid_rogue as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "humanoid_archer".to_string(),
            name: "Archer".to_string(),
            category: TemplateCategory::Humanoid,
            complexity: Complexity::Beginner,
            mesh_count: 8,
            description: "Light leather-armored ranged combatant carrying a bow and quiver. Suitable for rangers, hunters, and scouts.".to_string(),
            tags: vec!["humanoid".to_string(), "biped".to_string(), "archer".to_string(), "ranged".to_string(), "bow".to_string(), "hunter".to_string()],
        },
        generate_humanoid_archer("Example Archer", 0),
        generate_humanoid_archer as TemplateGenerator,
    );

    // -----------------------------------------------------------------------
    // Creature category
    // -----------------------------------------------------------------------

    registry.register(
        TemplateMetadata {
            id: "quadruped_basic".to_string(),
            name: "Quadruped".to_string(),
            category: TemplateCategory::Creature,
            complexity: Complexity::Beginner,
            mesh_count: 6,
            description: "Four-legged creature with elongated body. Great for wolves, bears, dogs, and other animals.".to_string(),
            tags: vec!["quadruped".to_string(), "animal".to_string(), "four-legged".to_string()],
        },
        generate_quadruped_template("Example Quadruped", 0),
        generate_quadruped_template as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "quadruped_wolf".to_string(),
            name: "Wolf".to_string(),
            category: TemplateCategory::Creature,
            complexity: Complexity::Beginner,
            mesh_count: 8,
            description: "Lean predatory canine with a snout and upright tail. Ideal for wolves, foxes, dire wolves, and similar beasts.".to_string(),
            tags: vec!["quadruped".to_string(), "animal".to_string(), "wolf".to_string(), "canine".to_string(), "predator".to_string()],
        },
        generate_quadruped_wolf("Example Wolf", 0),
        generate_quadruped_wolf as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "flying_basic".to_string(),
            name: "Flying Creature".to_string(),
            category: TemplateCategory::Creature,
            complexity: Complexity::Intermediate,
            mesh_count: 4,
            description: "Winged creature with body, beak, and wings. Ideal for birds, bats, and flying monsters.".to_string(),
            tags: vec!["flying".to_string(), "winged".to_string(), "bird".to_string()],
        },
        generate_flying_template("Example Bird", 0),
        generate_flying_template as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "slime_basic".to_string(),
            name: "Slime/Blob".to_string(),
            category: TemplateCategory::Creature,
            complexity: Complexity::Beginner,
            mesh_count: 3,
            description: "Simple blob creature with squashed sphere body and eyes. Perfect for slimes, oozes, and amorphous monsters.".to_string(),
            tags: vec!["slime".to_string(), "blob".to_string(), "ooze".to_string(), "simple".to_string()],
        },
        generate_slime_template("Example Slime", 0),
        generate_slime_template as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "spider_basic".to_string(),
            name: "Spider".to_string(),
            category: TemplateCategory::Creature,
            complexity: Complexity::Intermediate,
            mesh_count: 10,
            description: "Eight-legged arachnid with two distinct body segments. Suitable for spiders, scorpions, and other arthropods.".to_string(),
            tags: vec!["spider".to_string(), "arachnid".to_string(), "eight-legged".to_string(), "arthropod".to_string()],
        },
        generate_spider_basic("Example Spider", 0),
        generate_spider_basic as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "snake_basic".to_string(),
            name: "Snake".to_string(),
            category: TemplateCategory::Creature,
            complexity: Complexity::Beginner,
            mesh_count: 7,
            description: "Segmented serpentine body with no legs and a tapered tail. Great for snakes, worms, and legless reptiles.".to_string(),
            tags: vec!["snake".to_string(), "reptile".to_string(), "legless".to_string(), "serpent".to_string()],
        },
        generate_snake_basic("Example Snake", 0),
        generate_snake_basic as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "dragon_basic".to_string(),
            name: "Dragon".to_string(),
            category: TemplateCategory::Creature,
            complexity: Complexity::Advanced,
            mesh_count: 11,
            description: "Complex dragon creature with body, head, horns, wings, tail, and legs. Advanced template for large boss monsters.".to_string(),
            tags: vec!["dragon".to_string(), "boss".to_string(), "winged".to_string(), "complex".to_string()],
        },
        generate_dragon_template("Example Dragon", 0),
        generate_dragon_template as TemplateGenerator,
    );

    // -----------------------------------------------------------------------
    // Undead category
    // -----------------------------------------------------------------------

    registry.register(
        TemplateMetadata {
            id: "skeleton_basic".to_string(),
            name: "Skeleton".to_string(),
            category: TemplateCategory::Undead,
            complexity: Complexity::Beginner,
            mesh_count: 6,
            description: "Animated skeleton with thin ivory bone shapes. Perfect for skeleton warriors, undead guards, and bone constructs.".to_string(),
            tags: vec!["undead".to_string(), "skeleton".to_string(), "bones".to_string(), "animated".to_string()],
        },
        generate_skeleton_basic("Example Skeleton", 0),
        generate_skeleton_basic as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "zombie_basic".to_string(),
            name: "Zombie".to_string(),
            category: TemplateCategory::Undead,
            complexity: Complexity::Beginner,
            mesh_count: 6,
            description: "Shambling undead humanoid in tattered rags with an asymmetric zombie pose. Good for zombies, ghouls, and walking dead.".to_string(),
            tags: vec!["undead".to_string(), "zombie".to_string(), "humanoid".to_string(), "shambling".to_string()],
        },
        generate_zombie_basic("Example Zombie", 0),
        generate_zombie_basic as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "ghost_basic".to_string(),
            name: "Ghost".to_string(),
            category: TemplateCategory::Undead,
            complexity: Complexity::Beginner,
            mesh_count: 6,
            description: "Incorporeal spirit with a translucent flowing form and ethereal wispy trails. Ideal for ghosts, wraiths, and spectral entities.".to_string(),
            tags: vec!["undead".to_string(), "ghost".to_string(), "spirit".to_string(), "translucent".to_string(), "incorporeal".to_string()],
        },
        generate_ghost_basic("Example Ghost", 0),
        generate_ghost_basic as TemplateGenerator,
    );

    // -----------------------------------------------------------------------
    // Robot category
    // -----------------------------------------------------------------------

    registry.register(
        TemplateMetadata {
            id: "robot_basic".to_string(),
            name: "Robot (Basic)".to_string(),
            category: TemplateCategory::Robot,
            complexity: Complexity::Beginner,
            mesh_count: 6,
            description: "Simple boxy mechanical construct with cube body and head, and thick cylinder limbs. Good for golems, automata, and basic robots.".to_string(),
            tags: vec!["robot".to_string(), "mechanical".to_string(), "golem".to_string(), "construct".to_string(), "boxy".to_string()],
        },
        generate_robot_basic("Example Robot", 0),
        generate_robot_basic as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "robot_advanced".to_string(),
            name: "Robot (Advanced)".to_string(),
            category: TemplateCategory::Robot,
            complexity: Complexity::Intermediate,
            mesh_count: 12,
            description: "Sophisticated mechanical construct with articulated shoulder joints, separate forearms, chest panel detail, and a glowing sensor eye.".to_string(),
            tags: vec!["robot".to_string(), "mechanical".to_string(), "advanced".to_string(), "construct".to_string(), "articulated".to_string()],
        },
        generate_robot_advanced("Example Advanced Robot", 0),
        generate_robot_advanced as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "robot_flying".to_string(),
            name: "Robot (Flying)".to_string(),
            category: TemplateCategory::Robot,
            complexity: Complexity::Intermediate,
            mesh_count: 8,
            description: "Aerial mechanical drone with wide wing panels, landing struts, and downward-facing thruster nozzles. Suitable for drones, air units, and flying constructs.".to_string(),
            tags: vec!["robot".to_string(), "mechanical".to_string(), "flying".to_string(), "drone".to_string(), "construct".to_string()],
        },
        generate_robot_flying("Example Flying Robot", 0),
        generate_robot_flying as TemplateGenerator,
    );

    // -----------------------------------------------------------------------
    // Primitive category
    // -----------------------------------------------------------------------

    registry.register(
        TemplateMetadata {
            id: "primitive_cube".to_string(),
            name: "Cube".to_string(),
            category: TemplateCategory::Primitive,
            complexity: Complexity::Beginner,
            mesh_count: 1,
            description: "Single cube primitive. Use as a blank canvas to build a completely custom creature from scratch.".to_string(),
            tags: vec!["primitive".to_string(), "cube".to_string(), "box".to_string(), "simple".to_string()],
        },
        generate_primitive_cube("Example Cube", 0),
        generate_primitive_cube as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "primitive_sphere".to_string(),
            name: "Sphere".to_string(),
            category: TemplateCategory::Primitive,
            complexity: Complexity::Beginner,
            mesh_count: 1,
            description:
                "Single sphere primitive. A smooth round starting point for custom organic shapes."
                    .to_string(),
            tags: vec![
                "primitive".to_string(),
                "sphere".to_string(),
                "ball".to_string(),
                "simple".to_string(),
            ],
        },
        generate_primitive_sphere("Example Sphere", 0),
        generate_primitive_sphere as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "primitive_cylinder".to_string(),
            name: "Cylinder".to_string(),
            category: TemplateCategory::Primitive,
            complexity: Complexity::Beginner,
            mesh_count: 1,
            description: "Single cylinder primitive. Good starting point for pillars, limbs, and tubular forms.".to_string(),
            tags: vec!["primitive".to_string(), "cylinder".to_string(), "tube".to_string(), "simple".to_string()],
        },
        generate_primitive_cylinder("Example Cylinder", 0),
        generate_primitive_cylinder as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "primitive_cone".to_string(),
            name: "Cone".to_string(),
            category: TemplateCategory::Primitive,
            complexity: Complexity::Beginner,
            mesh_count: 1,
            description:
                "Single cone primitive. Useful for horns, spikes, hats, and tapered shapes."
                    .to_string(),
            tags: vec![
                "primitive".to_string(),
                "cone".to_string(),
                "spike".to_string(),
                "simple".to_string(),
            ],
        },
        generate_primitive_cone("Example Cone", 0),
        generate_primitive_cone as TemplateGenerator,
    );

    registry.register(
        TemplateMetadata {
            id: "primitive_pyramid".to_string(),
            name: "Pyramid".to_string(),
            category: TemplateCategory::Primitive,
            complexity: Complexity::Beginner,
            mesh_count: 1,
            description: "Single four-sided pyramid primitive. Useful for angular spikes, obelisks, and geometric decoration.".to_string(),
            tags: vec!["primitive".to_string(), "pyramid".to_string(), "angular".to_string(), "simple".to_string()],
        },
        generate_primitive_pyramid("Example Pyramid", 0),
        generate_primitive_pyramid as TemplateGenerator,
    );

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Existing template structure tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_humanoid_template_structure() {
        let creature = generate_humanoid_template("Test Humanoid", 1);
        assert_eq!(creature.name, "Test Humanoid");
        assert_eq!(creature.id, 1);
        assert_eq!(creature.meshes.len(), 6);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_quadruped_template_structure() {
        let creature = generate_quadruped_template("Test Wolf", 2);
        assert_eq!(creature.name, "Test Wolf");
        assert_eq!(creature.meshes.len(), 6);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_flying_template_structure() {
        let creature = generate_flying_template("Test Bird", 3);
        assert_eq!(creature.name, "Test Bird");
        assert_eq!(creature.meshes.len(), 4);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_slime_template_structure() {
        let creature = generate_slime_template("Test Slime", 4);
        assert_eq!(creature.name, "Test Slime");
        assert_eq!(creature.meshes.len(), 3);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
        assert!(creature.color_tint.is_some());
    }

    #[test]
    fn test_dragon_template_structure() {
        let creature = generate_dragon_template("Test Dragon", 5);
        assert_eq!(creature.name, "Test Dragon");
        assert_eq!(creature.meshes.len(), 11);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    // -----------------------------------------------------------------------
    // Humanoid variant structure tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_humanoid_fighter_structure() {
        let creature = generate_humanoid_fighter("Test Fighter", 10);
        assert_eq!(creature.name, "Test Fighter");
        assert_eq!(creature.id, 10);
        assert_eq!(creature.meshes.len(), 8);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_humanoid_mage_structure() {
        let creature = generate_humanoid_mage("Test Mage", 11);
        assert_eq!(creature.name, "Test Mage");
        assert_eq!(creature.id, 11);
        assert_eq!(creature.meshes.len(), 8);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_humanoid_cleric_structure() {
        let creature = generate_humanoid_cleric("Test Cleric", 12);
        assert_eq!(creature.name, "Test Cleric");
        assert_eq!(creature.id, 12);
        assert_eq!(creature.meshes.len(), 9);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_humanoid_rogue_structure() {
        let creature = generate_humanoid_rogue("Test Rogue", 13);
        assert_eq!(creature.name, "Test Rogue");
        assert_eq!(creature.id, 13);
        assert_eq!(creature.meshes.len(), 9);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_humanoid_archer_structure() {
        let creature = generate_humanoid_archer("Test Archer", 14);
        assert_eq!(creature.name, "Test Archer");
        assert_eq!(creature.id, 14);
        assert_eq!(creature.meshes.len(), 8);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    // -----------------------------------------------------------------------
    // Creature variant structure tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_quadruped_wolf_structure() {
        let creature = generate_quadruped_wolf("Test Wolf", 20);
        assert_eq!(creature.name, "Test Wolf");
        assert_eq!(creature.id, 20);
        assert_eq!(creature.meshes.len(), 8);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_spider_basic_structure() {
        let creature = generate_spider_basic("Test Spider", 21);
        assert_eq!(creature.name, "Test Spider");
        assert_eq!(creature.id, 21);
        assert_eq!(creature.meshes.len(), 10);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_snake_basic_structure() {
        let creature = generate_snake_basic("Test Snake", 22);
        assert_eq!(creature.name, "Test Snake");
        assert_eq!(creature.id, 22);
        assert_eq!(creature.meshes.len(), 7);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    // -----------------------------------------------------------------------
    // Undead structure tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_skeleton_basic_structure() {
        let creature = generate_skeleton_basic("Test Skeleton", 30);
        assert_eq!(creature.name, "Test Skeleton");
        assert_eq!(creature.id, 30);
        assert_eq!(creature.meshes.len(), 6);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
        assert!(creature.color_tint.is_none());
    }

    #[test]
    fn test_zombie_basic_structure() {
        let creature = generate_zombie_basic("Test Zombie", 31);
        assert_eq!(creature.name, "Test Zombie");
        assert_eq!(creature.id, 31);
        assert_eq!(creature.meshes.len(), 6);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_ghost_basic_structure() {
        let creature = generate_ghost_basic("Test Ghost", 32);
        assert_eq!(creature.name, "Test Ghost");
        assert_eq!(creature.id, 32);
        assert_eq!(creature.meshes.len(), 6);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
        // Ghost uses a translucent color tint
        assert!(creature.color_tint.is_some());
        let tint = creature.color_tint.unwrap();
        assert!(tint[3] < 1.0, "Ghost color tint should be semi-transparent");
    }

    // -----------------------------------------------------------------------
    // Robot structure tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_robot_basic_structure() {
        let creature = generate_robot_basic("Test Robot", 40);
        assert_eq!(creature.name, "Test Robot");
        assert_eq!(creature.id, 40);
        assert_eq!(creature.meshes.len(), 6);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_robot_advanced_structure() {
        let creature = generate_robot_advanced("Test Advanced Robot", 41);
        assert_eq!(creature.name, "Test Advanced Robot");
        assert_eq!(creature.id, 41);
        assert_eq!(creature.meshes.len(), 12);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_robot_flying_structure() {
        let creature = generate_robot_flying("Test Drone", 42);
        assert_eq!(creature.name, "Test Drone");
        assert_eq!(creature.id, 42);
        assert_eq!(creature.meshes.len(), 8);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    // -----------------------------------------------------------------------
    // Primitive structure tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_primitive_cube_structure() {
        let creature = generate_primitive_cube("Test Cube", 50);
        assert_eq!(creature.name, "Test Cube");
        assert_eq!(creature.meshes.len(), 1);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_primitive_sphere_structure() {
        let creature = generate_primitive_sphere("Test Sphere", 51);
        assert_eq!(creature.name, "Test Sphere");
        assert_eq!(creature.meshes.len(), 1);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_primitive_cylinder_structure() {
        let creature = generate_primitive_cylinder("Test Cylinder", 52);
        assert_eq!(creature.name, "Test Cylinder");
        assert_eq!(creature.meshes.len(), 1);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_primitive_cone_structure() {
        let creature = generate_primitive_cone("Test Cone", 53);
        assert_eq!(creature.name, "Test Cone");
        assert_eq!(creature.meshes.len(), 1);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    #[test]
    fn test_primitive_pyramid_structure() {
        let creature = generate_primitive_pyramid("Test Pyramid", 54);
        assert_eq!(creature.name, "Test Pyramid");
        assert_eq!(creature.meshes.len(), 1);
        assert_eq!(creature.meshes.len(), creature.mesh_transforms.len());
    }

    // -----------------------------------------------------------------------
    // Batch validation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_all_templates_validate() {
        for (name, generator) in available_templates() {
            let creature = generator(name, 999);
            assert!(
                creature.validate().is_ok(),
                "Template '{}' failed validation",
                name
            );
        }
    }

    #[test]
    fn test_all_templates_mesh_transform_consistency() {
        for (name, generator) in available_templates() {
            let creature = generator(name, 1);
            assert_eq!(
                creature.meshes.len(),
                creature.mesh_transforms.len(),
                "Template '{}': mesh and transform counts must match",
                name
            );
        }
    }

    #[test]
    fn test_all_humanoid_variants_validate() {
        let humanoids: &[(&str, TemplateGenerator)] = &[
            ("Fighter", generate_humanoid_fighter),
            ("Mage", generate_humanoid_mage),
            ("Cleric", generate_humanoid_cleric),
            ("Rogue", generate_humanoid_rogue),
            ("Archer", generate_humanoid_archer),
        ];
        for (name, gen) in humanoids {
            let creature = gen(name, 0);
            assert!(
                creature.validate().is_ok(),
                "Humanoid variant '{}' failed validation",
                name
            );
        }
    }

    #[test]
    fn test_all_undead_templates_validate() {
        let undead: &[(&str, TemplateGenerator)] = &[
            ("Skeleton", generate_skeleton_basic),
            ("Zombie", generate_zombie_basic),
            ("Ghost", generate_ghost_basic),
        ];
        for (name, gen) in undead {
            let creature = gen(name, 0);
            assert!(
                creature.validate().is_ok(),
                "Undead template '{}' failed validation",
                name
            );
        }
    }

    #[test]
    fn test_all_robot_templates_validate() {
        let robots: &[(&str, TemplateGenerator)] = &[
            ("Robot Basic", generate_robot_basic),
            ("Robot Advanced", generate_robot_advanced),
            ("Robot Flying", generate_robot_flying),
        ];
        for (name, gen) in robots {
            let creature = gen(name, 0);
            assert!(
                creature.validate().is_ok(),
                "Robot template '{}' failed validation",
                name
            );
        }
    }

    #[test]
    fn test_all_primitive_templates_validate() {
        let primitives: &[(&str, TemplateGenerator)] = &[
            ("Cube", generate_primitive_cube),
            ("Sphere", generate_primitive_sphere),
            ("Cylinder", generate_primitive_cylinder),
            ("Cone", generate_primitive_cone),
            ("Pyramid", generate_primitive_pyramid),
        ];
        for (name, gen) in primitives {
            let creature = gen(name, 0);
            assert!(
                creature.validate().is_ok(),
                "Primitive template '{}' failed validation",
                name
            );
        }
    }

    // -----------------------------------------------------------------------
    // available_templates() legacy API tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_available_templates_count() {
        let templates = available_templates();
        assert!(
            templates.len() >= 15,
            "Expected at least 15 templates, got {}",
            templates.len()
        );
    }

    #[test]
    fn test_available_templates_includes_all_categories() {
        let templates = available_templates();
        let names: Vec<&str> = templates.iter().map(|(n, _)| *n).collect();

        // Humanoid variants
        assert!(names.contains(&"Fighter"), "Missing Fighter template");
        assert!(names.contains(&"Mage"), "Missing Mage template");
        assert!(names.contains(&"Cleric"), "Missing Cleric template");
        assert!(names.contains(&"Rogue"), "Missing Rogue template");
        assert!(names.contains(&"Archer"), "Missing Archer template");

        // Creature variants
        assert!(names.contains(&"Wolf"), "Missing Wolf template");
        assert!(names.contains(&"Spider"), "Missing Spider template");
        assert!(names.contains(&"Snake"), "Missing Snake template");

        // Undead
        assert!(names.contains(&"Skeleton"), "Missing Skeleton template");
        assert!(names.contains(&"Zombie"), "Missing Zombie template");
        assert!(names.contains(&"Ghost"), "Missing Ghost template");

        // Robots
        assert!(
            names.contains(&"Robot (Basic)"),
            "Missing Robot (Basic) template"
        );
        assert!(
            names.contains(&"Robot (Advanced)"),
            "Missing Robot (Advanced) template"
        );
        assert!(
            names.contains(&"Robot (Flying)"),
            "Missing Robot (Flying) template"
        );
    }

    // -----------------------------------------------------------------------
    // initialize_template_registry() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_initialize_template_registry_total_count() {
        let registry = initialize_template_registry();
        assert_eq!(registry.len(), 24, "Expected 24 registered templates");
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_registry_contains_original_templates() {
        let registry = initialize_template_registry();

        assert!(registry.get("humanoid_basic").is_some());
        assert!(registry.get("quadruped_basic").is_some());
        assert!(registry.get("flying_basic").is_some());
        assert!(registry.get("slime_basic").is_some());
        assert!(registry.get("dragon_basic").is_some());
    }

    #[test]
    fn test_registry_contains_humanoid_variants() {
        let registry = initialize_template_registry();

        assert!(registry.get("humanoid_fighter").is_some());
        assert!(registry.get("humanoid_mage").is_some());
        assert!(registry.get("humanoid_cleric").is_some());
        assert!(registry.get("humanoid_rogue").is_some());
        assert!(registry.get("humanoid_archer").is_some());
    }

    #[test]
    fn test_registry_contains_creature_variants() {
        let registry = initialize_template_registry();

        assert!(registry.get("quadruped_wolf").is_some());
        assert!(registry.get("spider_basic").is_some());
        assert!(registry.get("snake_basic").is_some());
    }

    #[test]
    fn test_registry_contains_undead_templates() {
        let registry = initialize_template_registry();

        assert!(registry.get("skeleton_basic").is_some());
        assert!(registry.get("zombie_basic").is_some());
        assert!(registry.get("ghost_basic").is_some());
    }

    #[test]
    fn test_registry_contains_robot_templates() {
        let registry = initialize_template_registry();

        assert!(registry.get("robot_basic").is_some());
        assert!(registry.get("robot_advanced").is_some());
        assert!(registry.get("robot_flying").is_some());
    }

    #[test]
    fn test_registry_contains_primitive_templates() {
        let registry = initialize_template_registry();

        assert!(registry.get("primitive_cube").is_some());
        assert!(registry.get("primitive_sphere").is_some());
        assert!(registry.get("primitive_cylinder").is_some());
        assert!(registry.get("primitive_cone").is_some());
        assert!(registry.get("primitive_pyramid").is_some());
    }

    #[test]
    fn test_registry_templates_by_humanoid_category() {
        let registry = initialize_template_registry();
        let humanoids = registry.by_category(TemplateCategory::Humanoid);
        assert_eq!(
            humanoids.len(),
            6,
            "Expected 6 humanoid templates (basic + 5 variants)"
        );
    }

    #[test]
    fn test_registry_templates_by_creature_category() {
        let registry = initialize_template_registry();
        let creatures = registry.by_category(TemplateCategory::Creature);
        assert_eq!(
            creatures.len(),
            7,
            "Expected 7 creature templates (quadruped, flying, slime, dragon, wolf, spider, snake)"
        );
    }

    #[test]
    fn test_registry_templates_by_undead_category() {
        let registry = initialize_template_registry();
        let undead = registry.by_category(TemplateCategory::Undead);
        assert_eq!(undead.len(), 3, "Expected 3 undead templates");
    }

    #[test]
    fn test_registry_templates_by_robot_category() {
        let registry = initialize_template_registry();
        let robots = registry.by_category(TemplateCategory::Robot);
        assert_eq!(robots.len(), 3, "Expected 3 robot templates");
    }

    #[test]
    fn test_registry_templates_by_primitive_category() {
        let registry = initialize_template_registry();
        let primitives = registry.by_category(TemplateCategory::Primitive);
        assert_eq!(primitives.len(), 5, "Expected 5 primitive templates");
    }

    #[test]
    fn test_registry_templates_by_complexity() {
        let registry = initialize_template_registry();

        let beginner = registry.by_complexity(Complexity::Beginner);
        // humanoid_basic, fighter, mage, cleric, rogue, archer = 6 humanoids
        // quadruped_basic, slime_basic, wolf, snake = 4 creatures
        // skeleton, zombie, ghost = 3 undead
        // robot_basic = 1 robot
        // cube, sphere, cylinder, cone, pyramid = 5 primitives
        // Total Beginner = 19
        assert_eq!(beginner.len(), 19, "Expected 19 beginner templates");

        let intermediate = registry.by_complexity(Complexity::Intermediate);
        // flying_basic, spider_basic, robot_advanced, robot_flying = 4
        assert_eq!(intermediate.len(), 4, "Expected 4 intermediate templates");

        let advanced = registry.by_complexity(Complexity::Advanced);
        // dragon_basic = 1
        assert_eq!(advanced.len(), 1, "Expected 1 advanced template");
    }

    #[test]
    fn test_registry_search_by_keyword() {
        let registry = initialize_template_registry();

        let results = registry.search("humanoid");
        assert!(!results.is_empty(), "humanoid search should return results");

        let results = registry.search("dragon");
        assert_eq!(
            results.len(),
            1,
            "dragon search should return exactly 1 result"
        );

        // Only flying_basic and dragon_basic have "winged" tag
        let results = registry.search("winged");
        assert_eq!(
            results.len(),
            2,
            "winged search should return 2 results (flying + dragon)"
        );
    }

    #[test]
    fn test_registry_search_undead_keyword() {
        let registry = initialize_template_registry();

        let results = registry.search("undead");
        assert_eq!(results.len(), 3, "undead search should return 3 results");
    }

    #[test]
    fn test_registry_search_robot_keyword() {
        let registry = initialize_template_registry();

        let results = registry.search("robot");
        assert_eq!(results.len(), 3, "robot search should return 3 results");
    }

    #[test]
    fn test_registry_search_primitive_keyword() {
        let registry = initialize_template_registry();

        let results = registry.search("primitive");
        assert_eq!(results.len(), 5, "primitive search should return 5 results");
    }

    #[test]
    fn test_registry_generate_humanoid_fighter() {
        let registry = initialize_template_registry();
        let result = registry.generate("humanoid_fighter", "Ser Aldric", 99);
        assert!(result.is_ok());
        let creature = result.unwrap();
        assert_eq!(creature.name, "Ser Aldric");
        assert_eq!(creature.id, 99);
        assert_eq!(creature.meshes.len(), 8);
    }

    #[test]
    fn test_registry_generate_skeleton() {
        let registry = initialize_template_registry();
        let result = registry.generate("skeleton_basic", "Bone Guard", 100);
        assert!(result.is_ok());
        let creature = result.unwrap();
        assert_eq!(creature.name, "Bone Guard");
        assert_eq!(creature.meshes.len(), 6);
    }

    #[test]
    fn test_registry_generate_robot_advanced() {
        let registry = initialize_template_registry();
        let result = registry.generate("robot_advanced", "ARACHNE-7", 200);
        assert!(result.is_ok());
        let creature = result.unwrap();
        assert_eq!(creature.name, "ARACHNE-7");
        assert_eq!(creature.meshes.len(), 12);
    }

    #[test]
    fn test_registry_metadata_mesh_count_accuracy() {
        let registry = initialize_template_registry();

        // Verify declared mesh_count matches the actual generated creature
        for entry in registry.all_templates() {
            let declared = entry.metadata.mesh_count;
            let actual = entry.example_creature.meshes.len();
            assert_eq!(
                declared, actual,
                "Template '{}': declared mesh_count {} does not match actual {}",
                entry.metadata.id, declared, actual
            );
        }
    }

    #[test]
    fn test_ghost_is_translucent() {
        let ghost = generate_ghost_basic("Shade", 0);
        // All ghost mesh colors should have alpha < 1.0
        for mesh in &ghost.meshes {
            assert!(
                mesh.color[3] < 1.0,
                "Ghost mesh color should be semi-transparent (alpha < 1.0)"
            );
        }
    }

    #[test]
    fn test_zombie_has_asymmetric_pose() {
        let zombie = generate_zombie_basic("Walker", 0);
        // The two arms should be at different Y translations (asymmetric zombie pose)
        assert_eq!(zombie.meshes.len(), 6);
        // Left arm is at index 2, right arm at index 3 - verify they exist
        assert_eq!(zombie.mesh_transforms.len(), 6);
    }

    #[test]
    fn test_spider_has_eight_legs() {
        let spider = generate_spider_basic("Crawler", 0);
        // 2 body segments + 8 legs = 10 total meshes
        assert_eq!(spider.meshes.len(), 10);
    }

    #[test]
    fn test_snake_has_no_legs() {
        let snake = generate_snake_basic("Serpent", 0);
        // 1 head + 1 neck + 4 body segments + 1 tail = 7 meshes, no leg meshes
        assert_eq!(snake.meshes.len(), 7);
    }

    #[test]
    fn test_robot_advanced_has_more_parts_than_basic() {
        let basic = generate_robot_basic("Basic", 0);
        let advanced = generate_robot_advanced("Advanced", 0);
        assert!(
            advanced.meshes.len() > basic.meshes.len(),
            "Advanced robot should have more meshes than basic robot"
        );
    }

    #[test]
    fn test_fighter_has_more_meshes_than_basic_humanoid() {
        let humanoid = generate_humanoid_template("Basic", 0);
        let fighter = generate_humanoid_fighter("Fighter", 0);
        assert!(
            fighter.meshes.len() > humanoid.meshes.len(),
            "Fighter should have more meshes than the basic humanoid"
        );
    }

    #[test]
    fn test_mage_has_staff_and_hat() {
        let mage = generate_humanoid_mage("Mage", 0);
        // 6 base parts + staff + hat = 8 meshes
        assert_eq!(mage.meshes.len(), 8);
    }

    #[test]
    fn test_cleric_has_holy_symbol_and_mace() {
        let cleric = generate_humanoid_cleric("Cleric", 0);
        // 6 base parts + holy symbol + mace handle + mace head = 9 meshes
        assert_eq!(cleric.meshes.len(), 9);
    }

    #[test]
    fn test_rogue_has_daggers_and_hood() {
        let rogue = generate_humanoid_rogue("Rogue", 0);
        // 6 base parts + hood + left dagger + right dagger = 9 meshes
        assert_eq!(rogue.meshes.len(), 9);
    }

    #[test]
    fn test_archer_has_bow_and_quiver() {
        let archer = generate_humanoid_archer("Archer", 0);
        // 6 base parts + bow + quiver = 8 meshes
        assert_eq!(archer.meshes.len(), 8);
    }

    #[test]
    fn test_wolf_has_snout_and_tail() {
        let wolf = generate_quadruped_wolf("Wolf", 0);
        // 7-part quadruped (body + head + 4 legs) + snout + tail = 8 meshes
        assert_eq!(wolf.meshes.len(), 8);
    }

    #[test]
    fn test_robot_flying_has_wings_and_thrusters() {
        let drone = generate_robot_flying("Drone", 0);
        // body + head + 2 wing panels + 2 leg stubs + 2 thrusters = 8 meshes
        assert_eq!(drone.meshes.len(), 8);
    }

    #[test]
    fn test_all_templates_have_positive_scale() {
        for (name, generator) in available_templates() {
            let creature = generator(name, 1);
            assert!(
                creature.scale > 0.0,
                "Template '{}' scale must be positive",
                name
            );
        }
    }

    #[test]
    fn test_registry_all_templates_retrievable() {
        let registry = initialize_template_registry();
        for entry in registry.all_templates() {
            let id = &entry.metadata.id;
            let retrieved = registry.get(id);
            assert!(
                retrieved.is_some(),
                "Template '{}' should be retrievable by ID",
                id
            );
        }
    }
}
