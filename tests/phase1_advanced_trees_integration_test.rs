// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Phase 1: Advanced Tree Generation System - Integration Tests
//!
//! Tests the complete tree spawning workflow including:
//! - Tree type variations (Oak, Pine, Birch, Willow, Dead, Shrub)
//! - Visual metadata application (scale, height, color tint, rotation)
//! - Mesh generation and caching
//! - Entity spawning and component setup

use antares::domain::world::TileVisualMetadata;
use antares::game::systems::advanced_trees::{
    generate_branch_mesh, Branch, BranchGraph, TerrainVisualConfig, TreeConfig, TreeType,
};
use bevy::prelude::*;

#[test]
fn test_branch_graph_construction_and_bounds() {
    // Test: BranchGraph correctly constructs hierarchical tree structure
    let mut graph = BranchGraph::new();

    // Create trunk (root)
    let trunk = Branch {
        start: Vec3::ZERO,
        end: Vec3::new(0.0, 3.5, 0.0),
        start_radius: 0.3,
        end_radius: 0.15,
        children: vec![1, 2], // Two child branches
    };
    let trunk_idx = graph.add_branch(trunk);
    assert_eq!(trunk_idx, 0);

    // Create first branch
    let branch1 = Branch {
        start: Vec3::new(0.0, 2.0, 0.0),
        end: Vec3::new(1.0, 3.0, 0.5),
        start_radius: 0.15,
        end_radius: 0.05,
        children: vec![],
    };
    let branch1_idx = graph.add_branch(branch1);
    assert_eq!(branch1_idx, 1);

    // Create second branch
    let branch2 = Branch {
        start: Vec3::new(0.0, 2.0, 0.0),
        end: Vec3::new(-1.0, 3.0, 0.5),
        start_radius: 0.15,
        end_radius: 0.05,
        children: vec![],
    };
    let branch2_idx = graph.add_branch(branch2);
    assert_eq!(branch2_idx, 2);

    // Update bounds and verify
    graph.update_bounds();
    let (min, max) = graph.bounds;

    assert!(min.x <= -1.0, "Min X should include branch2");
    assert!(max.x >= 1.0, "Max X should include branch1");
    assert!(min.y <= 0.0, "Min Y should be at root start");
    assert!(max.y >= 3.0, "Max Y should include branch endpoints");
}

#[test]
fn test_tree_type_configurations_are_distinct() {
    // Test: Each TreeType produces visually distinct configurations
    let oak = TreeType::Oak.config();
    let pine = TreeType::Pine.config();
    let _birch = TreeType::Birch.config();
    let _willow = TreeType::Willow.config();
    let dead = TreeType::Dead.config();
    let shrub = TreeType::Shrub.config();

    // Verify distinctiveness
    assert_ne!(
        oak.trunk_radius, pine.trunk_radius,
        "Oak and Pine trunk radii should differ"
    );
    assert_ne!(
        oak.height, pine.height,
        "Oak and Pine heights should differ"
    );
    assert_ne!(
        oak.foliage_density, pine.foliage_density,
        "Oak and Pine foliage density should differ"
    );

    // Verify characteristic differences
    assert!(oak.foliage_density > 0.0, "Oak should have foliage");
    assert_eq!(
        dead.foliage_density, 0.0,
        "Dead tree should have no foliage"
    );
    assert!(
        shrub.foliage_density > oak.foliage_density,
        "Shrub should be bushier than Oak"
    );
    assert!(
        shrub.height < oak.height,
        "Shrub should be shorter than Oak"
    );
}

#[test]
fn test_terrain_visual_config_from_metadata() {
    // Test: TerrainVisualConfig correctly converts from TileVisualMetadata
    let meta = TileVisualMetadata {
        height: Some(4.0),
        width_x: None,
        width_z: None,
        color_tint: Some((0.8, 0.6, 0.4)),
        scale: Some(1.5),
        y_offset: None,
        rotation_y: Some(45.0),
        sprite: None,
        sprite_layers: vec![],
        sprite_rule: None,
        grass_density: None,
        tree_type: None,
        rock_variant: None,
        water_flow_direction: None,
        foliage_density: None,
        snow_coverage: None,
    };

    let config = TerrainVisualConfig::from(&meta);
    assert_eq!(config.scale, 1.5);
    assert_eq!(config.height_multiplier, 2.0); // 4.0 / 2.0
    assert_eq!(config.rotation_y, 45.0);
    assert!(config.color_tint.is_some());
}

#[test]
fn test_terrain_visual_config_defaults() {
    // Test: TerrainVisualConfig fills in defaults for missing metadata
    let meta = TileVisualMetadata {
        height: None,
        width_x: None,
        width_z: None,
        color_tint: None,
        scale: None,
        y_offset: None,
        rotation_y: None,
        sprite: None,
        sprite_layers: vec![],
        sprite_rule: None,
        grass_density: None,
        tree_type: None,
        rock_variant: None,
        water_flow_direction: None,
        foliage_density: None,
        snow_coverage: None,
    };

    let config = TerrainVisualConfig::from(&meta);
    assert_eq!(config.scale, 1.0);
    assert_eq!(config.height_multiplier, 1.0);
    assert_eq!(config.rotation_y, 0.0);
    assert!(config.color_tint.is_none());
}

#[test]
fn test_branch_mesh_generation_with_oak_config() {
    // Test: generate_branch_mesh produces valid mesh for Oak tree
    let mut graph = BranchGraph::new();
    graph.add_branch(Branch {
        start: Vec3::ZERO,
        end: Vec3::new(0.0, 3.5, 0.0),
        start_radius: 0.3,
        end_radius: 0.1,
        children: vec![],
    });
    graph.update_bounds();

    let config = TreeType::Oak.config();
    let mesh = generate_branch_mesh(&graph, &config);

    // Verify mesh structure
    assert!(
        mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some(),
        "Mesh should have positions"
    );
    assert!(
        mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some(),
        "Mesh should have normals"
    );
}

#[test]
fn test_branch_mesh_generation_empty_graph_fallback() {
    // Test: generate_branch_mesh handles empty graph gracefully
    let graph = BranchGraph::new();
    let config = TreeConfig::default();
    let mesh = generate_branch_mesh(&graph, &config);

    // Should produce a valid mesh even with empty graph
    assert!(
        mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some(),
        "Empty graph should produce fallback mesh"
    );
}

#[test]
fn test_tree_type_enumeration() {
    // Test: All TreeType variants are enumerated correctly
    let all_types = TreeType::all();
    assert_eq!(all_types.len(), 6, "Should have exactly 6 tree types");

    // Verify all variants present
    let oak_found = all_types.contains(&TreeType::Oak);
    let pine_found = all_types.contains(&TreeType::Pine);
    let birch_found = all_types.contains(&TreeType::Birch);
    let willow_found = all_types.contains(&TreeType::Willow);
    let dead_found = all_types.contains(&TreeType::Dead);
    let shrub_found = all_types.contains(&TreeType::Shrub);

    assert!(oak_found, "Oak should be in enumeration");
    assert!(pine_found, "Pine should be in enumeration");
    assert!(birch_found, "Birch should be in enumeration");
    assert!(willow_found, "Willow should be in enumeration");
    assert!(dead_found, "Dead should be in enumeration");
    assert!(shrub_found, "Shrub should be in enumeration");
}

#[test]
fn test_tree_type_display_names() {
    // Test: All TreeTypes have appropriate display names
    for tree_type in TreeType::all() {
        let name = tree_type.name();
        assert!(!name.is_empty(), "Tree type should have non-empty name");
        assert!(
            !name.contains('\n'),
            "Tree type name should not contain newlines"
        );
    }
}

#[test]
fn test_multiple_tree_types_visual_variety() {
    // Test: Different tree types produce meshes with distinct characteristics
    let mut oak_graph = BranchGraph::new();
    oak_graph.add_branch(Branch {
        start: Vec3::ZERO,
        end: Vec3::new(0.0, 3.5, 0.0),
        start_radius: 0.3,
        end_radius: 0.1,
        children: vec![],
    });
    oak_graph.update_bounds();

    let mut pine_graph = BranchGraph::new();
    pine_graph.add_branch(Branch {
        start: Vec3::ZERO,
        end: Vec3::new(0.0, 5.0, 0.0),
        start_radius: 0.2,
        end_radius: 0.05,
        children: vec![],
    });
    pine_graph.update_bounds();

    let oak_config = TreeType::Oak.config();
    let pine_config = TreeType::Pine.config();

    let oak_mesh = generate_branch_mesh(&oak_graph, &oak_config);
    let pine_mesh = generate_branch_mesh(&pine_graph, &pine_config);

    // Both should produce valid meshes
    assert!(oak_mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
    assert!(pine_mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());

    // Verify distinct characteristics reflected in bounds
    let (_oak_min, oak_max) = oak_graph.bounds;
    let (_pine_min, pine_max) = pine_graph.bounds;

    assert!(oak_max.y < pine_max.y, "Pine should be taller than Oak");
}

#[test]
fn test_terrain_visual_config_scale_application() {
    // Test: Scale parameter affects configuration correctly
    let meta_small = TileVisualMetadata {
        height: None,
        width_x: None,
        width_z: None,
        color_tint: None,
        scale: Some(0.5),
        y_offset: None,
        rotation_y: None,
        sprite: None,
        sprite_layers: vec![],
        sprite_rule: None,
        grass_density: None,
        tree_type: None,
        rock_variant: None,
        water_flow_direction: None,
        foliage_density: None,
        snow_coverage: None,
    };

    let meta_large = TileVisualMetadata {
        height: None,
        width_x: None,
        width_z: None,
        color_tint: None,
        scale: Some(2.0),
        y_offset: None,
        rotation_y: None,
        sprite: None,
        sprite_layers: vec![],
        sprite_rule: None,
        grass_density: None,
        tree_type: None,
        rock_variant: None,
        water_flow_direction: None,
        foliage_density: None,
        snow_coverage: None,
    };

    let small_config = TerrainVisualConfig::from(&meta_small);
    let large_config = TerrainVisualConfig::from(&meta_large);

    assert_eq!(small_config.scale, 0.5);
    assert_eq!(large_config.scale, 2.0);
    assert!(large_config.scale > small_config.scale);
}

#[test]
fn test_terrain_visual_config_height_multiplier() {
    // Test: Height parameter correctly becomes height_multiplier
    let meta1 = TileVisualMetadata {
        height: Some(2.0),
        width_x: None,
        width_z: None,
        color_tint: None,
        scale: None,
        y_offset: None,
        rotation_y: None,
        sprite: None,
        sprite_layers: vec![],
        sprite_rule: None,
        grass_density: None,
        tree_type: None,
        rock_variant: None,
        water_flow_direction: None,
        foliage_density: None,
        snow_coverage: None,
    };

    let meta2 = TileVisualMetadata {
        height: Some(6.0),
        width_x: None,
        width_z: None,
        color_tint: None,
        scale: None,
        y_offset: None,
        rotation_y: None,
        sprite: None,
        sprite_layers: vec![],
        sprite_rule: None,
        grass_density: None,
        tree_type: None,
        rock_variant: None,
        water_flow_direction: None,
        foliage_density: None,
        snow_coverage: None,
    };

    let config1 = TerrainVisualConfig::from(&meta1);
    let config2 = TerrainVisualConfig::from(&meta2);

    assert_eq!(config1.height_multiplier, 1.0); // 2.0 / 2.0
    assert_eq!(config2.height_multiplier, 3.0); // 6.0 / 2.0
}

#[test]
fn test_branch_parent_child_relationships() {
    // Test: BranchGraph correctly maintains parent-child relationships
    let mut graph = BranchGraph::new();

    // Create parent branch with two children
    let parent = Branch {
        start: Vec3::ZERO,
        end: Vec3::new(0.0, 2.0, 0.0),
        start_radius: 0.3,
        end_radius: 0.15,
        children: vec![1, 2], // References to child indices
    };
    let parent_idx = graph.add_branch(parent);

    let child1 = Branch {
        start: Vec3::new(0.0, 2.0, 0.0),
        end: Vec3::new(1.0, 3.0, 0.0),
        start_radius: 0.15,
        end_radius: 0.05,
        children: vec![],
    };
    let child1_idx = graph.add_branch(child1);

    let child2 = Branch {
        start: Vec3::new(0.0, 2.0, 0.0),
        end: Vec3::new(-1.0, 3.0, 0.0),
        start_radius: 0.15,
        end_radius: 0.05,
        children: vec![],
    };
    let child2_idx = graph.add_branch(child2);

    // Verify relationships
    assert_eq!(parent_idx, 0);
    assert_eq!(child1_idx, 1);
    assert_eq!(child2_idx, 2);

    // Verify parent knows about children
    assert_eq!(graph.branches[parent_idx].children, vec![1, 2]);
}

#[test]
fn test_tree_config_foliage_density_range() {
    // Test: All TreeConfigs have reasonable foliage density values (0.0-1.0)
    for tree_type in TreeType::all() {
        let config = tree_type.config();
        assert!(
            config.foliage_density >= 0.0 && config.foliage_density <= 1.0,
            "{} foliage density out of range",
            tree_type.name()
        );
    }
}

#[test]
fn test_tree_config_height_range() {
    // Test: All TreeConfigs have reasonable height values
    for tree_type in TreeType::all() {
        let config = tree_type.config();
        assert!(
            config.height >= 0.5 && config.height <= 8.0,
            "{} height out of range",
            tree_type.name()
        );
    }
}

#[test]
fn test_tree_config_radius_range() {
    // Test: All TreeConfigs have reasonable trunk radius values
    for tree_type in TreeType::all() {
        let config = tree_type.config();
        assert!(
            config.trunk_radius >= 0.01 && config.trunk_radius <= 1.0,
            "{} trunk radius out of range",
            tree_type.name()
        );
    }
}

#[test]
fn test_branch_radius_tapering() {
    // Test: Branches taper correctly (end_radius < start_radius or equal)
    let mut graph = BranchGraph::new();

    let branch = Branch {
        start: Vec3::ZERO,
        end: Vec3::new(0.0, 2.0, 0.0),
        start_radius: 0.3,
        end_radius: 0.1,
        children: vec![],
    };
    graph.add_branch(branch);

    // Verify taper relationship
    assert!(
        graph.branches[0].end_radius <= graph.branches[0].start_radius,
        "Branch should taper (end_radius <= start_radius)"
    );
}

#[test]
fn test_generate_branch_mesh_produces_valid_vertices() {
    // Test: Mesh generation produces meshes with vertices
    let mut graph = BranchGraph::new();

    for i in 0..3 {
        let y_offset = (i as f32) * 1.0;
        graph.add_branch(Branch {
            start: Vec3::new(0.0, y_offset, 0.0),
            end: Vec3::new(0.0, y_offset + 1.0, 0.0),
            start_radius: 0.3 - (i as f32) * 0.1,
            end_radius: 0.15 - (i as f32) * 0.05,
            children: if i < 2 { vec![i + 1] } else { vec![] },
        });
    }
    graph.update_bounds();

    let config = TreeConfig::default();
    let mesh = generate_branch_mesh(&graph, &config);

    // Verify mesh validity
    assert!(
        mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some(),
        "Mesh should have vertex positions"
    );
}
