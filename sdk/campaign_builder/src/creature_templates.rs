// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Creature template generators for the campaign builder
//!
//! Provides pre-built creature templates using primitive shapes to help
//! content creators get started quickly.

use crate::primitive_generators::{
    generate_cone, generate_cube, generate_cylinder, generate_sphere,
};
use crate::template_metadata::{
    Complexity, TemplateCategory, TemplateGenerator, TemplateMetadata, TemplateRegistry,
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

/// Returns a list of all available creature templates (legacy)
///
/// # Returns
///
/// A vector of tuples containing (template_name, generator_function)
///
/// # Note
///
/// This function is deprecated. Use `initialize_template_registry()` instead
/// to get a registry with full metadata support.
pub fn available_templates() -> Vec<(&'static str, TemplateGenerator)> {
    vec![
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
    ]
}

/// Initializes a template registry with all available creature templates
///
/// This function populates a `TemplateRegistry` with all built-in creature
/// templates, including rich metadata for browsing, filtering, and searching.
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
/// assert!(registry.len() > 0);
///
/// // Search for templates
/// let humanoids = registry.search("humanoid");
/// assert!(!humanoids.is_empty());
/// ```
pub fn initialize_template_registry() -> TemplateRegistry {
    let mut registry = TemplateRegistry::new();

    // Humanoid template
    let humanoid_metadata = TemplateMetadata {
        id: "humanoid_basic".to_string(),
        name: "Humanoid".to_string(),
        category: TemplateCategory::Humanoid,
        complexity: Complexity::Beginner,
        mesh_count: 6,
        description: "Basic bipedal humanoid with body, head, arms, and legs. Perfect starting point for knights, mages, and NPCs.".to_string(),
        tags: vec!["humanoid".to_string(), "biped".to_string(), "basic".to_string()],
    };
    let humanoid_example = generate_humanoid_template("Example Humanoid", 0);
    registry.register(
        humanoid_metadata,
        humanoid_example,
        generate_humanoid_template as TemplateGenerator,
    );

    // Quadruped template
    let quadruped_metadata = TemplateMetadata {
        id: "quadruped_basic".to_string(),
        name: "Quadruped".to_string(),
        category: TemplateCategory::Creature,
        complexity: Complexity::Beginner,
        mesh_count: 6,
        description: "Four-legged creature with elongated body. Great for wolves, bears, dogs, and other animals.".to_string(),
        tags: vec!["quadruped".to_string(), "animal".to_string(), "four-legged".to_string()],
    };
    let quadruped_example = generate_quadruped_template("Example Quadruped", 0);
    registry.register(
        quadruped_metadata,
        quadruped_example,
        generate_quadruped_template as TemplateGenerator,
    );

    // Flying creature template
    let flying_metadata = TemplateMetadata {
        id: "flying_basic".to_string(),
        name: "Flying Creature".to_string(),
        category: TemplateCategory::Creature,
        complexity: Complexity::Intermediate,
        mesh_count: 4,
        description: "Winged creature with body, beak, and wings. Ideal for birds, bats, and flying monsters.".to_string(),
        tags: vec!["flying".to_string(), "winged".to_string(), "bird".to_string()],
    };
    let flying_example = generate_flying_template("Example Bird", 0);
    registry.register(
        flying_metadata,
        flying_example,
        generate_flying_template as TemplateGenerator,
    );

    // Slime template
    let slime_metadata = TemplateMetadata {
        id: "slime_basic".to_string(),
        name: "Slime/Blob".to_string(),
        category: TemplateCategory::Creature,
        complexity: Complexity::Beginner,
        mesh_count: 3,
        description: "Simple blob creature with squashed sphere body and eyes. Perfect for slimes, oozes, and amorphous monsters.".to_string(),
        tags: vec!["slime".to_string(), "blob".to_string(), "ooze".to_string(), "simple".to_string()],
    };
    let slime_example = generate_slime_template("Example Slime", 0);
    registry.register(
        slime_metadata,
        slime_example,
        generate_slime_template as TemplateGenerator,
    );

    // Dragon template
    let dragon_metadata = TemplateMetadata {
        id: "dragon_basic".to_string(),
        name: "Dragon".to_string(),
        category: TemplateCategory::Creature,
        complexity: Complexity::Advanced,
        mesh_count: 11,
        description: "Complex dragon creature with body, head, horns, wings, tail, and legs. Advanced template for large boss monsters.".to_string(),
        tags: vec!["dragon".to_string(), "boss".to_string(), "winged".to_string(), "complex".to_string()],
    };
    let dragon_example = generate_dragon_template("Example Dragon", 0);
    registry.register(
        dragon_metadata,
        dragon_example,
        generate_dragon_template as TemplateGenerator,
    );

    registry
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

    #[test]
    fn test_initialize_template_registry() {
        let registry = initialize_template_registry();
        assert_eq!(registry.len(), 5);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_registry_contains_all_templates() {
        let registry = initialize_template_registry();

        assert!(registry.get("humanoid_basic").is_some());
        assert!(registry.get("quadruped_basic").is_some());
        assert!(registry.get("flying_basic").is_some());
        assert!(registry.get("slime_basic").is_some());
        assert!(registry.get("dragon_basic").is_some());
    }

    #[test]
    fn test_registry_templates_by_category() {
        let registry = initialize_template_registry();

        let humanoids = registry.by_category(TemplateCategory::Humanoid);
        assert_eq!(humanoids.len(), 1);

        let creatures = registry.by_category(TemplateCategory::Creature);
        assert_eq!(creatures.len(), 4);
    }

    #[test]
    fn test_registry_templates_by_complexity() {
        let registry = initialize_template_registry();

        let beginner = registry.by_complexity(Complexity::Beginner);
        assert_eq!(beginner.len(), 3); // humanoid, quadruped, slime

        let intermediate = registry.by_complexity(Complexity::Intermediate);
        assert_eq!(intermediate.len(), 1); // flying

        let advanced = registry.by_complexity(Complexity::Advanced);
        assert_eq!(advanced.len(), 1); // dragon
    }

    #[test]
    fn test_registry_search() {
        let registry = initialize_template_registry();

        let results = registry.search("humanoid");
        assert_eq!(results.len(), 1);

        let results = registry.search("dragon");
        assert_eq!(results.len(), 1);

        let results = registry.search("winged");
        assert_eq!(results.len(), 2); // flying and dragon
    }

    #[test]
    fn test_registry_generate_creature() {
        let registry = initialize_template_registry();

        let result = registry.generate("humanoid_basic", "Test Knight", 42);
        assert!(result.is_ok());

        let creature = result.unwrap();
        assert_eq!(creature.name, "Test Knight");
        assert_eq!(creature.id, 42);
        assert_eq!(creature.meshes.len(), 6);
    }

    #[test]
    fn test_registry_metadata_accuracy() {
        let registry = initialize_template_registry();

        let humanoid = registry.get("humanoid_basic").unwrap();
        assert_eq!(
            humanoid.metadata.mesh_count,
            humanoid.example_creature.meshes.len()
        );

        let dragon = registry.get("dragon_basic").unwrap();
        assert_eq!(
            dragon.metadata.mesh_count,
            dragon.example_creature.meshes.len()
        );
    }
}
