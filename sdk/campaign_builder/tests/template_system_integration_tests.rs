// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for Phase 3: Template System Integration
//!
//! Tests the complete workflow of:
//! - Template registry initialization
//! - Template browser UI state management
//! - Template search and filtering
//! - Template application workflow
//! - Integration with creature templates

use campaign_builder::creature_templates::initialize_template_registry;
use campaign_builder::template_browser::{TemplateBrowserAction, TemplateBrowserState, ViewMode};
use campaign_builder::template_metadata::{Complexity, TemplateCategory};

#[test]
fn test_template_registry_initialization() {
    let registry = initialize_template_registry();

    // Verify all expected templates are registered
    assert_eq!(registry.len(), 24, "Expected 24 built-in templates");

    // Verify original templates still exist
    assert!(registry.get("humanoid_basic").is_some());
    assert!(registry.get("quadruped_basic").is_some());
    assert!(registry.get("flying_basic").is_some());
    assert!(registry.get("slime_basic").is_some());
    assert!(registry.get("dragon_basic").is_some());

    // Verify new humanoid variants
    assert!(registry.get("humanoid_fighter").is_some());
    assert!(registry.get("humanoid_mage").is_some());
    assert!(registry.get("humanoid_cleric").is_some());
    assert!(registry.get("humanoid_rogue").is_some());
    assert!(registry.get("humanoid_archer").is_some());

    // Verify new creature variants
    assert!(registry.get("quadruped_wolf").is_some());
    assert!(registry.get("spider_basic").is_some());
    assert!(registry.get("snake_basic").is_some());

    // Verify undead templates
    assert!(registry.get("skeleton_basic").is_some());
    assert!(registry.get("zombie_basic").is_some());
    assert!(registry.get("ghost_basic").is_some());

    // Verify robot templates
    assert!(registry.get("robot_basic").is_some());
    assert!(registry.get("robot_advanced").is_some());
    assert!(registry.get("robot_flying").is_some());

    // Verify primitive templates
    assert!(registry.get("primitive_cube").is_some());
    assert!(registry.get("primitive_sphere").is_some());
    assert!(registry.get("primitive_cylinder").is_some());
    assert!(registry.get("primitive_cone").is_some());
    assert!(registry.get("primitive_pyramid").is_some());
}

#[test]
fn test_template_metadata_accuracy() {
    let registry = initialize_template_registry();

    // Verify humanoid template metadata
    let humanoid = registry.get("humanoid_basic").unwrap();
    assert_eq!(humanoid.metadata.name, "Humanoid");
    assert_eq!(humanoid.metadata.category, TemplateCategory::Humanoid);
    assert_eq!(humanoid.metadata.complexity, Complexity::Beginner);
    assert_eq!(humanoid.metadata.mesh_count, 6);
    assert!(!humanoid.metadata.tags.is_empty());
    assert!(!humanoid.metadata.description.is_empty());

    // Verify dragon template metadata
    let dragon = registry.get("dragon_basic").unwrap();
    assert_eq!(dragon.metadata.name, "Dragon");
    assert_eq!(dragon.metadata.category, TemplateCategory::Creature);
    assert_eq!(dragon.metadata.complexity, Complexity::Advanced);
    assert_eq!(dragon.metadata.mesh_count, 11);
}

#[test]
fn test_template_mesh_count_matches_actual() {
    let registry = initialize_template_registry();

    for entry in registry.all_templates() {
        assert_eq!(
            entry.metadata.mesh_count,
            entry.example_creature.meshes.len(),
            "Template '{}' metadata mesh_count doesn't match actual meshes",
            entry.metadata.name
        );
    }
}

#[test]
fn test_template_category_filtering() {
    let registry = initialize_template_registry();

    // Filter by Humanoid category
    let humanoids = registry.by_category(TemplateCategory::Humanoid);
    assert_eq!(humanoids.len(), 6); // basic + fighter + mage + cleric + rogue + archer
    for entry in &humanoids {
        assert_eq!(entry.metadata.category, TemplateCategory::Humanoid);
    }

    // Filter by Creature category
    let creatures = registry.by_category(TemplateCategory::Creature);
    assert_eq!(creatures.len(), 7); // quadruped, flying, slime, dragon, wolf, spider, snake

    // Verify undead category
    let undead = registry.by_category(TemplateCategory::Undead);
    assert_eq!(undead.len(), 3); // skeleton, zombie, ghost

    // Verify robot category
    let robots = registry.by_category(TemplateCategory::Robot);
    assert_eq!(robots.len(), 3); // basic, advanced, flying

    // Verify primitive category
    let primitives = registry.by_category(TemplateCategory::Primitive);
    assert_eq!(primitives.len(), 5); // cube, sphere, cylinder, cone, pyramid
}

#[test]
fn test_template_complexity_filtering() {
    let registry = initialize_template_registry();

    // Filter by Beginner complexity
    let beginner = registry.by_complexity(Complexity::Beginner);
    // humanoid_basic, fighter, mage, cleric, rogue, archer = 6 humanoids
    // quadruped_basic, slime_basic, wolf, snake = 4 creatures
    // skeleton, zombie, ghost = 3 undead
    // robot_basic = 1 robot
    // cube, sphere, cylinder, cone, pyramid = 5 primitives
    // Total = 19
    assert_eq!(beginner.len(), 19);

    // Filter by Intermediate complexity
    let intermediate = registry.by_complexity(Complexity::Intermediate);
    // flying_basic, spider_basic, robot_advanced, robot_flying = 4
    assert_eq!(intermediate.len(), 4);

    // Filter by Advanced complexity
    let advanced = registry.by_complexity(Complexity::Advanced);
    assert_eq!(advanced.len(), 1); // dragon

    // No Expert templates
    let expert = registry.by_complexity(Complexity::Expert);
    assert_eq!(expert.len(), 0);
}

#[test]
fn test_template_search_by_name() {
    let registry = initialize_template_registry();

    // "humanoid" appears in tags/descriptions for multiple humanoid templates
    let results = registry.search("humanoid");
    assert!(!results.is_empty(), "humanoid search should return results");
    assert!(results.iter().any(|e| e.metadata.name == "Humanoid"));

    let results = registry.search("dragon");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.name, "Dragon");
}

#[test]
fn test_template_search_by_tags() {
    let registry = initialize_template_registry();

    // Search for "winged" should find flying creature and dragon only
    let results = registry.search("winged");
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|e| e.metadata.name == "Flying Creature"));
    assert!(results.iter().any(|e| e.metadata.name == "Dragon"));

    // Search for "biped" should find all humanoid templates (all tagged "biped")
    let results = registry.search("biped");
    assert_eq!(results.len(), 6);
    for entry in &results {
        assert_eq!(entry.metadata.category, TemplateCategory::Humanoid);
    }
}

#[test]
fn test_template_search_case_insensitive() {
    let registry = initialize_template_registry();

    let lower = registry.search("dragon");
    let upper = registry.search("DRAGON");
    let mixed = registry.search("DrAgOn");

    assert_eq!(lower.len(), upper.len());
    assert_eq!(lower.len(), mixed.len());
    assert_eq!(lower[0].metadata.id, upper[0].metadata.id);
}

#[test]
fn test_template_generation() {
    let registry = initialize_template_registry();

    // Generate a humanoid creature
    let result = registry.generate("humanoid_basic", "Test Knight", 42);
    assert!(result.is_ok());

    let creature = result.unwrap();
    assert_eq!(creature.name, "Test Knight");
    assert_eq!(creature.id, 42);
    assert_eq!(creature.meshes.len(), 6);
    assert_eq!(creature.mesh_transforms.len(), 6);

    // Test invalid template ID
    let result = registry.generate("nonexistent_template", "Test", 1);
    assert!(result.is_err());
}

#[test]
fn test_template_browser_state_initialization() {
    let state = TemplateBrowserState::new();

    assert_eq!(state.selected_template, None);
    assert_eq!(state.search_query, "");
    assert_eq!(state.category_filter, None);
    assert_eq!(state.complexity_filter, None);
    assert_eq!(state.view_mode, ViewMode::Grid);
    assert!(state.show_preview);
}

#[test]
fn test_browser_state_category_filter() {
    let mut browser = TemplateBrowserState::new();

    // Default is no filter
    assert_eq!(browser.category_filter, None);

    // Can set category filter
    browser.category_filter = Some(TemplateCategory::Humanoid);
    assert_eq!(browser.category_filter, Some(TemplateCategory::Humanoid));
}

#[test]
fn test_browser_state_complexity_filter() {
    let mut browser = TemplateBrowserState::new();

    // Default is no filter
    assert_eq!(browser.complexity_filter, None);

    // Can set complexity filter
    browser.complexity_filter = Some(Complexity::Beginner);
    assert_eq!(browser.complexity_filter, Some(Complexity::Beginner));
}

#[test]
fn test_browser_state_search_query() {
    let mut browser = TemplateBrowserState::new();

    // Default is empty
    assert_eq!(browser.search_query, "");

    // Can set search query
    browser.search_query = "dragon".to_string();
    assert_eq!(browser.search_query, "dragon");
}

#[test]
fn test_browser_state_filters_combination() {
    let mut browser = TemplateBrowserState::new();

    // Can set multiple filters
    browser.category_filter = Some(TemplateCategory::Creature);
    browser.complexity_filter = Some(Complexity::Beginner);
    browser.search_query = "test".to_string();

    assert_eq!(browser.category_filter, Some(TemplateCategory::Creature));
    assert_eq!(browser.complexity_filter, Some(Complexity::Beginner));
    assert_eq!(browser.search_query, "test");
}

#[test]
fn test_browser_action_variants() {
    // Test ApplyToCurrent action
    let action = TemplateBrowserAction::ApplyToCurrent("humanoid_basic".to_string());
    match action {
        TemplateBrowserAction::ApplyToCurrent(id) => {
            assert_eq!(id, "humanoid_basic");
        }
        _ => panic!("Expected ApplyToCurrent variant"),
    }

    // Test CreateNew action
    let action = TemplateBrowserAction::CreateNew("dragon_basic".to_string());
    match action {
        TemplateBrowserAction::CreateNew(id) => {
            assert_eq!(id, "dragon_basic");
        }
        _ => panic!("Expected CreateNew variant"),
    }
}

#[test]
fn test_template_application_workflow() {
    let registry = initialize_template_registry();

    // Step 1: User selects a template from browser
    let template_id = "humanoid_basic";
    let template = registry.get(template_id);
    assert!(template.is_some());

    // Step 2: User clicks "Apply to Current" or "Create New"
    let _action = TemplateBrowserAction::ApplyToCurrent(template_id.to_string());

    // Step 3: Generate creature from template
    let result = registry.generate(template_id, "Generated Knight", 99);
    assert!(result.is_ok());

    let creature = result.unwrap();
    assert_eq!(creature.name, "Generated Knight");
    assert_eq!(creature.id, 99);

    // Verify creature has expected structure
    let template_entry = template.unwrap();
    assert_eq!(creature.meshes.len(), template_entry.metadata.mesh_count);
}

#[test]
fn test_available_categories_list() {
    let registry = initialize_template_registry();
    let categories = registry.available_categories();

    // Should have at least Humanoid and Creature
    assert!(categories.contains(&TemplateCategory::Humanoid));
    assert!(categories.contains(&TemplateCategory::Creature));

    // Should not have duplicates
    let unique_count = categories.len();
    let mut sorted = categories.clone();
    sorted.sort_by_key(|c| format!("{:?}", c));
    sorted.dedup();
    assert_eq!(unique_count, sorted.len());
}

#[test]
fn test_available_tags_list() {
    let registry = initialize_template_registry();
    let tags = registry.available_tags();

    // Should have common tags
    assert!(tags.contains(&"humanoid".to_string()));
    assert!(tags.contains(&"winged".to_string()));

    // Should not have duplicates
    let unique_count = tags.len();
    let mut sorted = tags.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(unique_count, sorted.len());
}

#[test]
fn test_complexity_levels_correctly_assigned() {
    let registry = initialize_template_registry();

    // Beginner templates should have simple mesh counts
    for entry in registry.by_complexity(Complexity::Beginner) {
        assert!(
            entry.metadata.mesh_count <= 10,
            "Beginner template '{}' has too many meshes: {}",
            entry.metadata.name,
            entry.metadata.mesh_count
        );
    }

    // Advanced templates should have more meshes
    for entry in registry.by_complexity(Complexity::Advanced) {
        assert!(
            entry.metadata.mesh_count >= 10,
            "Advanced template '{}' should have more meshes: {}",
            entry.metadata.name,
            entry.metadata.mesh_count
        );
    }
}

#[test]
fn test_template_descriptions_exist() {
    let registry = initialize_template_registry();

    for entry in registry.all_templates() {
        assert!(
            !entry.metadata.description.is_empty(),
            "Template '{}' missing description",
            entry.metadata.name
        );
        assert!(
            entry.metadata.description.len() >= 20,
            "Template '{}' description too short",
            entry.metadata.name
        );
    }
}

#[test]
fn test_template_tags_exist() {
    let registry = initialize_template_registry();

    for entry in registry.all_templates() {
        assert!(
            !entry.metadata.tags.is_empty(),
            "Template '{}' missing tags",
            entry.metadata.name
        );
    }
}

#[test]
fn test_template_ids_are_unique() {
    let registry = initialize_template_registry();
    let all_templates = registry.all_templates();

    let mut ids = Vec::new();
    for entry in all_templates {
        assert!(
            !ids.contains(&entry.metadata.id),
            "Duplicate template ID: {}",
            entry.metadata.id
        );
        ids.push(entry.metadata.id.clone());
    }
}

#[test]
fn test_template_names_are_unique() {
    let registry = initialize_template_registry();
    let all_templates = registry.all_templates();

    let mut names = Vec::new();
    for entry in all_templates {
        assert!(
            !names.contains(&entry.metadata.name),
            "Duplicate template name: {}",
            entry.metadata.name
        );
        names.push(entry.metadata.name.clone());
    }
}

#[test]
fn test_example_creatures_are_valid() {
    let registry = initialize_template_registry();

    for entry in registry.all_templates() {
        let creature = &entry.example_creature;

        // Verify basic validity
        assert!(
            !creature.name.is_empty(),
            "Template '{}' has empty creature name",
            entry.metadata.name
        );
        assert!(
            !creature.meshes.is_empty(),
            "Template '{}' has no meshes",
            entry.metadata.name
        );
        assert_eq!(
            creature.meshes.len(),
            creature.mesh_transforms.len(),
            "Template '{}' mesh/transform count mismatch",
            entry.metadata.name
        );

        // Verify each mesh has vertices and indices
        for (i, mesh) in creature.meshes.iter().enumerate() {
            assert!(
                !mesh.vertices.is_empty(),
                "Template '{}' mesh {} has no vertices",
                entry.metadata.name,
                i
            );
            assert!(
                !mesh.indices.is_empty(),
                "Template '{}' mesh {} has no indices",
                entry.metadata.name,
                i
            );
        }
    }
}

#[test]
fn test_complexity_from_mesh_count_heuristic() {
    assert_eq!(Complexity::from_mesh_count(1), Complexity::Beginner);
    assert_eq!(Complexity::from_mesh_count(5), Complexity::Beginner);
    assert_eq!(Complexity::from_mesh_count(6), Complexity::Intermediate);
    assert_eq!(Complexity::from_mesh_count(10), Complexity::Intermediate);
    assert_eq!(Complexity::from_mesh_count(11), Complexity::Advanced);
    assert_eq!(Complexity::from_mesh_count(20), Complexity::Advanced);
    assert_eq!(Complexity::from_mesh_count(21), Complexity::Expert);
    assert_eq!(Complexity::from_mesh_count(50), Complexity::Expert);
}

#[test]
fn test_template_browser_default_state() {
    let browser = TemplateBrowserState::new();

    // Verify default state has no filters applied
    assert_eq!(browser.category_filter, None);
    assert_eq!(browser.complexity_filter, None);
    assert_eq!(browser.search_query, "");
}

#[test]
fn test_view_mode_switching() {
    let mut browser = TemplateBrowserState::new();

    // Default is Grid
    assert_eq!(browser.view_mode, ViewMode::Grid);

    // Switch to List
    browser.view_mode = ViewMode::List;
    assert_eq!(browser.view_mode, ViewMode::List);

    // Switch back to Grid
    browser.view_mode = ViewMode::Grid;
    assert_eq!(browser.view_mode, ViewMode::Grid);
}

#[test]
fn test_registry_is_not_empty() {
    let registry = initialize_template_registry();
    assert!(!registry.is_empty());
    assert_eq!(registry.len(), 24);
}
