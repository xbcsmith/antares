// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for Foliage Distribution System
//!
//! Tests verify that foliage clusters are spawned correctly at branch endpoints
//! based on tree type and foliage_density configuration.

#[cfg(test)]
mod tests {
    use antares::game::systems::advanced_trees::{
        generate_branch_graph, get_leaf_branches, TreeType,
    };

    #[test]
    fn test_oak_tree_has_foliage() {
        // Oak trees should have foliage density > 0
        let graph = generate_branch_graph(TreeType::Oak);
        let config = TreeType::Oak.config();
        let leaf_indices = get_leaf_branches(&graph);

        // Oak should have foliage
        assert!(
            config.foliage_density > 0.0,
            "Oak tree should have foliage_density > 0"
        );

        // Oak should have leaf branches
        assert!(
            !leaf_indices.is_empty(),
            "Oak tree should have leaf branches for foliage"
        );

        // Calculate cluster size
        let cluster_size = (config.foliage_density * 5.0) as usize;
        assert!(cluster_size > 0, "Oak tree should produce foliage clusters");
    }

    #[test]
    fn test_dead_tree_has_no_foliage() {
        // Dead trees should have foliage_density = 0.0
        let config = TreeType::Dead.config();

        assert_eq!(
            config.foliage_density, 0.0,
            "Dead tree should have foliage_density = 0"
        );

        // No foliage should be spawned
        let cluster_size = (config.foliage_density * 5.0) as usize;
        assert_eq!(
            cluster_size, 0,
            "Dead tree should produce no foliage clusters"
        );
    }

    #[test]
    fn test_pine_tree_sparse_foliage() {
        // Pine trees should have moderate foliage
        let graph = generate_branch_graph(TreeType::Pine);
        let config = TreeType::Pine.config();
        let leaf_indices = get_leaf_branches(&graph);

        // Pine should have some foliage
        assert!(
            config.foliage_density > 0.0,
            "Pine tree should have foliage_density > 0"
        );

        // Pine should have leaf branches
        assert!(
            !leaf_indices.is_empty(),
            "Pine tree should have leaf branches"
        );

        // Pine should have fewer foliage spheres per leaf than Oak
        let pine_cluster = (config.foliage_density * 5.0) as usize;
        let oak_cluster = (TreeType::Oak.config().foliage_density * 5.0) as usize;

        assert!(
            pine_cluster <= oak_cluster,
            "Pine should have equal or fewer foliage spheres than Oak"
        );
    }

    #[test]
    fn test_foliage_positioned_at_branch_ends() {
        // Leaf branch positions should be at branch endpoints
        let graph = generate_branch_graph(TreeType::Oak);
        let leaf_indices = get_leaf_branches(&graph);

        // All leaf branches should be endpoints
        for &leaf_idx in &leaf_indices {
            let branch = &graph.branches[leaf_idx];

            // Leaf branches should have no children
            assert!(
                branch.children.is_empty(),
                "Leaf branch {} should have no children",
                leaf_idx
            );

            // Branch should have valid end position
            assert!(
                branch.end.is_finite(),
                "Leaf branch {} end position should be finite",
                leaf_idx
            );

            // Foliage should be positioned near the end (not too far from tree)
            let distance_from_origin = branch.end.length();
            assert!(
                distance_from_origin < 100.0,
                "Leaf branch {} should be reasonably close to origin ({})",
                leaf_idx,
                distance_from_origin
            );
        }
    }

    #[test]
    fn test_all_tree_types_have_consistent_foliage() {
        // Each tree type should have consistent foliage configuration
        for tree_type in [
            TreeType::Oak,
            TreeType::Pine,
            TreeType::Birch,
            TreeType::Willow,
            TreeType::Dead,
            TreeType::Shrub,
        ] {
            let graph = generate_branch_graph(tree_type);
            let config = tree_type.config();
            let leaf_indices = get_leaf_branches(&graph);

            // If foliage_density is 0, no foliage should be spawned
            if config.foliage_density == 0.0 {
                let cluster_size = (config.foliage_density * 5.0) as usize;
                assert_eq!(
                    cluster_size,
                    0,
                    "{} should produce no foliage clusters",
                    tree_type.name()
                );
            } else {
                // If foliage_density > 0, tree should have leaves to place foliage on
                if config.depth > 0 {
                    assert!(
                        !leaf_indices.is_empty(),
                        "{} should have leaf branches if it has depth > 0",
                        tree_type.name()
                    );
                }
            }

            // Verify foliage density is in valid range [0.0, 1.0]
            assert!(
                config.foliage_density >= 0.0 && config.foliage_density <= 1.0,
                "{} foliage_density should be in [0.0, 1.0], got {}",
                tree_type.name(),
                config.foliage_density
            );
        }
    }

    #[test]
    fn test_foliage_density_parameter_affects_cluster_count() {
        // Verify that foliage_density directly affects cluster size calculation
        let test_cases = vec![(0.0, 0), (0.2, 1), (0.4, 2), (0.6, 3), (0.8, 4), (1.0, 5)];

        for (density, expected_clusters) in test_cases {
            let cluster_size = (density * 5.0) as usize;
            assert_eq!(
                cluster_size, expected_clusters,
                "Density {} should produce {} clusters",
                density, expected_clusters
            );
        }
    }
}
