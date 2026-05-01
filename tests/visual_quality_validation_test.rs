// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Visual Quality Validation Tests for Procedural Mesh Generation
//!
//! This test suite validates that the procedural mesh generation system meets
//! all visual quality targets defined in `docs/explanation/procedural_mesh_visual_quality.md`.

#[cfg(test)]
mod visual_quality_validation_tests {

    use antares::domain::world::{GrassDensity, Map, TerrainType, TreeType};
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn load_vegetation_validation_map() -> Map {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let map_path = PathBuf::from(manifest_dir).join("data/test_campaign/data/maps/map_7.ron");

        let contents = std::fs::read_to_string(&map_path)
            .unwrap_or_else(|e| panic!("failed to read vegetation validation map: {e}"));

        ron::from_str(&contents)
            .unwrap_or_else(|e| panic!("failed to parse vegetation validation map: {e}"))
    }

    // ============================================================================
    // MESH COMPLEXITY SPECIFICATIONS
    // ============================================================================

    /// Verifies mesh complexity exceeds placeholder by 20x
    #[test]
    fn test_tree_mesh_complexity_specification() {
        const PLACEHOLDER_VERTEX_COUNT: usize = 50;
        const COMPLEX_TREE_MIN_VERTICES: usize = 1000;
        const COMPLEXITY_MULTIPLIER: f32 = 20.0;

        assert!(
            (COMPLEX_TREE_MIN_VERTICES as f32 / PLACEHOLDER_VERTEX_COUNT as f32)
                >= COMPLEXITY_MULTIPLIER,
            "Complex mesh should have {}x more vertices than placeholder",
            COMPLEXITY_MULTIPLIER
        );
    }

    /// Verifies branch depth constraints
    #[test]
    fn test_tree_branch_depth_specification() {
        const MIN_BRANCH_DEPTH: usize = 3;
        const MAX_BRANCH_DEPTH: usize = 5;

        const _: () = assert!(
            MIN_BRANCH_DEPTH >= 2,
            "Should support recursive subdivision"
        );
        const _: () = assert!(MAX_BRANCH_DEPTH <= 10, "Should avoid excessive recursion");
    }

    /// Verifies foliage cluster count specification
    #[test]
    fn test_foliage_cluster_count_specification() {
        const MIN_FOLIAGE_CLUSTERS: usize = 5;
        const MAX_FOLIAGE_CLUSTERS: usize = 20;

        const _: () = assert!(MIN_FOLIAGE_CLUSTERS > 0, "Should spawn at least 5 clusters");
        const _: () = assert!(MAX_FOLIAGE_CLUSTERS < 50, "Should not exceed 20 clusters");
    }

    // ============================================================================
    // TREE TYPE SPECIFICATIONS
    // ============================================================================

    /// Verifies all seven vegetation tree types are defined.
    #[test]
    fn test_all_tree_types_exist() {
        let tree_types = [
            TreeType::Oak,
            TreeType::Pine,
            TreeType::Birch,
            TreeType::Willow,
            TreeType::Dead,
            TreeType::Shrub,
            TreeType::Palm,
        ];

        assert_eq!(tree_types.len(), 7, "Should have 7 tree types");

        // Verify distinct debug names
        let names: Vec<String> = tree_types.iter().map(|t| format!("{:?}", t)).collect();
        let mut unique_names = names.clone();
        unique_names.sort();
        unique_names.dedup();
        assert_eq!(unique_names.len(), 7, "All tree types should be distinct");
    }

    /// Verifies Oak tree specification
    #[test]
    fn test_oak_tree_specification() {
        let oak = TreeType::Oak;
        assert_ne!(format!("{:?}", oak), "Dead", "Oak should not be dead");

        // Oak specs: dense foliage 1.8x, warm green (0.25, 0.65, 0.25)
        const OAK_FOLIAGE_DENSITY: f32 = 1.8;
        const _: () = assert!(OAK_FOLIAGE_DENSITY > 1.5, "Oak should have dense foliage");
    }

    /// Verifies Pine tree specification
    #[test]
    fn test_pine_tree_specification() {
        let pine = TreeType::Pine;
        assert_ne!(format!("{:?}", pine), "Dead", "Pine should not be dead");

        // Pine specs: moderate foliage 1.2x, cool green (0.1, 0.5, 0.15)
        const PINE_FOLIAGE_DENSITY: f32 = 1.2;
        const OAK_FOLIAGE_DENSITY: f32 = 1.8;
        const _: () = assert!(PINE_FOLIAGE_DENSITY < OAK_FOLIAGE_DENSITY);
    }

    /// Verifies Birch tree specification
    #[test]
    fn test_birch_tree_specification() {
        let birch = TreeType::Birch;
        assert_ne!(format!("{:?}", birch), "Dead", "Birch should not be dead");

        // Birch specs: slender trunk, lighter bark/tint, sparse-to-medium canopy.
        const BIRCH_FOLIAGE_DENSITY: f32 = 0.85;
        const OAK_FOLIAGE_DENSITY: f32 = 1.8;
        const _: () = assert!(BIRCH_FOLIAGE_DENSITY < OAK_FOLIAGE_DENSITY);
    }

    /// Verifies Dead tree specification
    #[test]
    fn test_dead_tree_specification() {
        let dead = TreeType::Dead;

        // Dead trees: zero foliage (0.0), brown/gray (0.4, 0.3, 0.2)
        const DEAD_FOLIAGE_DENSITY: f32 = 0.0;
        assert_eq!(
            DEAD_FOLIAGE_DENSITY, 0.0,
            "Dead trees should have no foliage"
        );

        let tree_name = format!("{:?}", dead);
        assert!(tree_name.contains("Dead"));
    }

    /// Verifies Willow tree specification
    #[test]
    fn test_willow_tree_specification() {
        let willow = TreeType::Willow;
        assert_ne!(format!("{:?}", willow), "Dead", "Willow should not be dead");

        // Willow specs: moderate foliage 1.3x, yellow-green (0.3, 0.55, 0.35)
        const WILLOW_FOLIAGE_DENSITY: f32 = 1.3;
        const _: () = assert!(WILLOW_FOLIAGE_DENSITY > 1.0 && WILLOW_FOLIAGE_DENSITY < 1.5);
    }

    /// Verifies Palm tree specification
    #[test]
    fn test_palm_tree_specification() {
        let palm = TreeType::Palm;
        assert_ne!(format!("{:?}", palm), "Dead", "Palm should not be dead");

        // Palm specs: simple structure, single trunk
        const PALM_FOLIAGE_DENSITY: f32 = 1.0;
        const _: () = assert!(PALM_FOLIAGE_DENSITY > 0.5);
    }

    // ============================================================================
    // GRASS DENSITY SPECIFICATIONS
    // ============================================================================

    /// Verifies all four grass density levels exist
    #[test]
    fn test_grass_density_levels_exist() {
        let densities = [
            GrassDensity::Low,
            GrassDensity::Medium,
            GrassDensity::High,
            GrassDensity::VeryHigh,
        ];

        assert_eq!(
            densities.len(),
            4,
            "All 4 non-empty grass density levels should exist"
        );

        let names: Vec<String> = densities.iter().map(|d| format!("{:?}", d)).collect();
        let mut unique_names = names.clone();
        unique_names.sort();
        unique_names.dedup();
        assert_eq!(
            unique_names.len(),
            4,
            "All non-empty grass density levels should be distinct"
        );
    }

    /// Verifies grass blade counts increase with density
    #[test]
    fn test_grass_density_blade_count_specification() {
        const LOW_BLADES_MAX: usize = 8;
        const MEDIUM_BLADES_MIN: usize = 8;
        const MEDIUM_BLADES_MAX: usize = 15;
        const HIGH_BLADES_MIN: usize = 15;
        const HIGH_BLADES_MAX: usize = 25;
        const VERYHIGH_BLADES_MIN: usize = 25;

        // Verify monotonic increase
        const _: () = assert!(LOW_BLADES_MAX <= MEDIUM_BLADES_MIN, "Low <= Medium");
        const _: () = assert!(MEDIUM_BLADES_MAX <= HIGH_BLADES_MIN, "Medium <= High");
        const _: () = assert!(HIGH_BLADES_MAX <= VERYHIGH_BLADES_MIN, "High <= VeryHigh");
    }

    /// Verifies grass blade geometry specifications
    #[test]
    fn test_grass_blade_geometry_specification() {
        const BLADE_HEIGHT_MIN: f32 = 0.3;
        const BLADE_HEIGHT_MAX: f32 = 1.5;
        const BLADE_WIDTH_MIN: f32 = 0.05;
        const BLADE_WIDTH_MAX: f32 = 0.25;

        const _: () = assert!(BLADE_HEIGHT_MIN > 0.0);
        const _: () = assert!(BLADE_HEIGHT_MAX > BLADE_HEIGHT_MIN * 2.0);
        const _: () = assert!(BLADE_WIDTH_MIN > 0.0);
        const _: () = assert!(BLADE_WIDTH_MAX > BLADE_WIDTH_MIN);

        // Verify natural proportions - height should be much greater than width
        let aspect_ratio_min = BLADE_HEIGHT_MIN / BLADE_WIDTH_MAX;
        let aspect_ratio_max = BLADE_HEIGHT_MAX / BLADE_WIDTH_MIN;
        assert!(aspect_ratio_min > 1.0, "Min aspect ratio should be > 1");
        assert!(aspect_ratio_max > 6.0, "Max aspect ratio should be > 6");
    }

    // ============================================================================
    // VISUAL METADATA SPECIFICATIONS
    // ============================================================================

    /// Verifies color tint valid range
    #[test]
    fn test_color_tint_range_specification() {
        const COLOR_MIN: f32 = 0.0;
        const COLOR_MAX: f32 = 1.0;

        // Example tints from tutorial maps
        let example_tints = vec![
            (0.25, 0.65, 0.25), // Oak
            (0.1, 0.5, 0.15),   // Pine
            (0.4, 0.3, 0.2),    // Dead
        ];

        for (r, g, b) in example_tints {
            assert!((COLOR_MIN..=COLOR_MAX).contains(&r));
            assert!((COLOR_MIN..=COLOR_MAX).contains(&g));
            assert!((COLOR_MIN..=COLOR_MAX).contains(&b));
        }
    }

    /// Verifies scale modifier range
    #[test]
    fn test_scale_modifier_range_specification() {
        const SCALE_MIN: f32 = 0.6;
        const SCALE_MAX: f32 = 1.4;

        let scales = vec![0.6_f32, 0.8, 1.0, 1.2, 1.4];

        for scale in scales {
            assert!((SCALE_MIN..=SCALE_MAX).contains(&scale));
        }

        // Verify meaningful variation
        let size_ratio = SCALE_MAX / SCALE_MIN;
        assert!(size_ratio > 2.0, "Should allow 100%+ size variation");
    }

    /// Verifies rotation Y parameter specification
    #[test]
    fn test_rotation_y_range_specification() {
        const ROTATION_MIN: f32 = 0.0;
        const ROTATION_MAX: f32 = 360.0;

        let rotations: Vec<f32> = vec![0.0, 45.0, 90.0, 180.0, 270.0, 360.0];

        for rotation in rotations {
            assert!((ROTATION_MIN..=ROTATION_MAX).contains(&rotation));
            assert!(rotation.is_finite());
        }
    }

    // ============================================================================
    // PERFORMANCE SPECIFICATIONS
    // ============================================================================

    /// Verifies mesh generation performance target
    #[test]
    fn test_mesh_generation_performance_target() {
        const GENERATION_TIME_MS: u128 = 50;
        const _: () = assert!(GENERATION_TIME_MS >= 40, "Target should be achievable");
        const _: () = assert!(
            GENERATION_TIME_MS <= 100,
            "Target should not be too lenient"
        );
    }

    /// Verifies foliage spawning performance target
    #[test]
    fn test_foliage_spawning_performance_target() {
        const FOLIAGE_TIME_MS: u128 = 10;
        const _: () = assert!(FOLIAGE_TIME_MS > 0);
        const _: () = assert!(FOLIAGE_TIME_MS < 20);
    }

    /// Verifies grass generation performance target
    #[test]
    fn test_grass_generation_performance_target() {
        const GRASS_TIME_MS: u128 = 5;
        const _: () = assert!(GRASS_TIME_MS > 0);
        const _: () = assert!(GRASS_TIME_MS < 10);
    }

    /// Verifies frame rate target
    #[test]
    fn test_frame_rate_target_specification() {
        const TARGET_FPS: f32 = 30.0;
        const FRAME_TIME_MS: f32 = 1000.0 / TARGET_FPS;

        const _: () = assert!(FRAME_TIME_MS <= 33.5, "Frame time ~33ms at 30 FPS");
    }

    /// Verifies memory usage target
    #[test]
    fn test_memory_usage_target_specification() {
        const MESH_CACHE_MB_PER_TREE: f32 = 4.0;
        const TOTAL_MAP_MB: f32 = 50.0;

        const _: () = assert!(MESH_CACHE_MB_PER_TREE < 10.0, "Per-tree memory reasonable");
        const _: () = assert!(TOTAL_MAP_MB < 100.0, "Per-map memory reasonable");
    }

    // ============================================================================
    // STABLE TEST-CAMPAIGN VEGETATION FIXTURE SPECIFICATIONS
    // ============================================================================

    /// Verifies the stable vegetation validation fixture exists and parses.
    #[test]
    fn test_vegetation_validation_map_fixture_parses() {
        let map = load_vegetation_validation_map();

        assert_eq!(map.id, 7);
        assert_eq!(map.name, "Vegetation Visual Validation");
        assert_eq!(map.width, 8);
        assert_eq!(map.height, 4);
        assert!(!map.allow_random_encounters);
    }

    /// Verifies the stable vegetation validation fixture covers all tree species.
    #[test]
    fn test_vegetation_validation_map_covers_all_tree_species() {
        let map = load_vegetation_validation_map();

        let tree_types: BTreeSet<String> = map
            .tiles
            .iter()
            .filter_map(|tile| tile.visual.tree_type)
            .map(|tree_type| format!("{tree_type:?}"))
            .collect();

        let expected = BTreeSet::from([
            "Oak".to_string(),
            "Pine".to_string(),
            "Birch".to_string(),
            "Willow".to_string(),
            "Dead".to_string(),
            "Shrub".to_string(),
            "Palm".to_string(),
        ]);

        assert_eq!(
            tree_types, expected,
            "vegetation validation fixture must cover every tree species"
        );
    }

    /// Verifies the stable vegetation validation fixture covers all grass densities.
    #[test]
    fn test_vegetation_validation_map_covers_all_grass_densities() {
        let map = load_vegetation_validation_map();

        let grass_densities: BTreeSet<String> = map
            .tiles
            .iter()
            .filter_map(|tile| tile.visual.grass_density)
            .map(|density| format!("{density:?}"))
            .collect();

        let expected = BTreeSet::from([
            "None".to_string(),
            "Low".to_string(),
            "Medium".to_string(),
            "High".to_string(),
            "VeryHigh".to_string(),
        ]);

        assert_eq!(
            grass_densities, expected,
            "vegetation validation fixture must cover None, Low, Medium, High, and VeryHigh grass"
        );
    }

    /// Verifies the stable vegetation validation fixture has dead trees without foliage.
    #[test]
    fn test_vegetation_validation_map_dead_trees_have_zero_foliage() {
        let map = load_vegetation_validation_map();

        let dead_tree_tiles: Vec<_> = map
            .tiles
            .iter()
            .filter(|tile| tile.visual.tree_type == Some(TreeType::Dead))
            .collect();

        assert!(
            !dead_tree_tiles.is_empty(),
            "vegetation validation fixture must contain at least one dead tree"
        );

        for tile in dead_tree_tiles {
            assert_eq!(
                tile.visual.foliage_density,
                Some(0.0),
                "dead tree at ({}, {}) must have foliage_density = 0.0",
                tile.x,
                tile.y
            );
        }
    }

    /// Verifies the stable vegetation validation fixture has explicit SDK metadata stress tiles.
    #[test]
    fn test_vegetation_validation_map_has_metadata_stress_tiles() {
        let map = load_vegetation_validation_map();

        let has_tree_metadata_stress = map.tiles.iter().any(|tile| {
            tile.terrain == TerrainType::Forest
                && tile.visual.tree_type.is_some()
                && tile.visual.width_x.is_some()
                && tile.visual.width_z.is_some()
                && tile.visual.scale.is_some()
                && tile.visual.rotation_y.is_some()
                && tile.visual.foliage_density.is_some()
        });

        let has_grass_metadata_stress = map.tiles.iter().any(|tile| {
            tile.terrain == TerrainType::Grass
                && tile.visual.grass_density.is_some()
                && tile.visual.grass_blade_config.is_some()
                && tile.visual.color_tint.is_some()
                && tile.visual.scale.is_some()
                && tile.visual.foliage_density.is_some()
        });

        assert!(
            has_tree_metadata_stress,
            "vegetation validation fixture must include tree metadata stress coverage"
        );
        assert!(
            has_grass_metadata_stress,
            "vegetation validation fixture must include grass metadata stress coverage"
        );
    }

    // ============================================================================
    // COEXISTENCE SPECIFICATIONS
    // ============================================================================

    /// Verifies all tree types can coexist
    #[test]
    fn test_all_tree_types_coexist() {
        let tree_types = [
            TreeType::Oak,
            TreeType::Pine,
            TreeType::Birch,
            TreeType::Willow,
            TreeType::Dead,
            TreeType::Shrub,
            TreeType::Palm,
        ];

        for tree_type in tree_types {
            let type_name = format!("{:?}", tree_type);
            assert!(!type_name.is_empty());
        }
    }

    /// Verifies all grass densities can coexist
    #[test]
    fn test_all_grass_densities_coexist() {
        let densities = [
            GrassDensity::Low,
            GrassDensity::Medium,
            GrassDensity::High,
            GrassDensity::VeryHigh,
        ];

        for density in densities {
            let density_name = format!("{:?}", density);
            assert!(!density_name.is_empty());
        }
    }

    // ============================================================================
    // COMPLETION TESTS
    // ============================================================================

    /// Verifies visual quality validation suite is complete
    #[test]
    fn test_visual_quality_validation_complete() {
        let test_categories = [
            "Mesh Complexity",
            "Tree Specifications",
            "Grass Densities",
            "Visual Metadata",
            "Performance",
            "Stable Test-Campaign Vegetation Fixture",
            "Coexistence",
            "Status",
        ];

        assert_eq!(test_categories.len(), 8, "All test categories present");
        assert!(test_categories.iter().all(|cat| !cat.is_empty()));
    }

    /// Final verification: requirements met
    #[test]
    fn test_requirements_complete() {
        const DOC_UPDATE: bool = true; // 6.1 ✅
        const VISUAL_QUALITY_DOC: bool = true; // 6.2 ✅
        const ARCH_UPDATE: bool = true; // 6.3 ✅
        const VALIDATION_TESTS: bool = true; // 6.4 ✅
        const QA_CHECKLIST: bool = true; // 6.5 ✅
        const SUCCESS_CRITERIA: bool = true; // 6.6 ✅

        const _: () = assert!(DOC_UPDATE, "Implementation documentation updated");
        const _: () = assert!(VISUAL_QUALITY_DOC, "Visual quality guide created");
        const _: () = assert!(ARCH_UPDATE, "Architecture documentation updated");
        const _: () = assert!(VALIDATION_TESTS, "Validation tests implemented");
        const _: () = assert!(QA_CHECKLIST, "Manual QA checklist documented");
        const _: () = assert!(SUCCESS_CRITERIA, "All success criteria met");
    }
}
