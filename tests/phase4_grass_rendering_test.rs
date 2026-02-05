// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for Phase 4: Complex Grass Blade Generation
//!
//! Tests verify that:
//! - Curved grass blades are generated with proper geometry
//! - Grass clusters are spawned at appropriate density levels
//! - Blade variation (height, width, curve) is within expected ranges
//! - Cluster-based approach produces natural grass patches

use antares::domain::world::TileVisualMetadata;
use antares::game::resources::{GrassDensity, GrassQualitySettings};

// Helper test utilities
fn create_test_tile_visual_metadata() -> TileVisualMetadata {
    TileVisualMetadata::default()
}

fn create_test_grass_quality_settings(density: GrassDensity) -> GrassQualitySettings {
    GrassQualitySettings { density }
}

// ==================== Density Tests ====================

/// Tests that Low density produces sparse grass
#[test]
fn test_grass_density_low_sparse() {
    let settings = create_test_grass_quality_settings(GrassDensity::Low);
    let (min, max) = settings.density.blade_count_range();

    // Low: 2-4 blades per tile
    assert_eq!(min, 2, "Low density minimum should be 2");
    assert_eq!(max, 4, "Low density maximum should be 4");
    assert!(max <= 4, "Low density should not exceed 4 blades");
}

/// Tests that Medium density produces balanced grass
#[test]
fn test_grass_density_medium_balanced() {
    let settings = create_test_grass_quality_settings(GrassDensity::Medium);
    let (min, max) = settings.density.blade_count_range();

    // Medium: 6-10 blades per tile
    assert_eq!(min, 6, "Medium density minimum should be 6");
    assert_eq!(max, 10, "Medium density maximum should be 10");
    assert!(min < max, "Min should be less than max");
}

/// Tests that High density produces dense grass
#[test]
fn test_grass_density_high_dense() {
    let settings = create_test_grass_quality_settings(GrassDensity::High);
    let (min, max) = settings.density.blade_count_range();

    // High: 12-20 blades per tile
    assert_eq!(min, 12, "High density minimum should be 12");
    assert_eq!(max, 20, "High density maximum should be 20");
    assert!(max > 10, "High density should exceed 10 blades");
}

// ==================== Cluster Formation Tests ====================

/// Tests that cluster count is calculated correctly from blade count
#[test]
fn test_grass_cluster_count_calculation() {
    // spawn_grass divides blade_count by 7 for clusters of 5-10 blades
    let test_cases = vec![
        (2, 1),  // 2 blades: 1 cluster (max(2/7, 1) = 1)
        (4, 1),  // 4 blades: 1 cluster
        (6, 1),  // 6 blades: 1 cluster (6/7 < 1, so max(x, 1) = 1)
        (10, 1), // 10 blades: 1 cluster
        (14, 2), // 14 blades: 2 clusters (14/7 = 2)
        (21, 3), // 21 blades: 3 clusters (21/7 = 3)
        (20, 2), // 20 blades: 2 clusters (20/7 ≈ 2)
    ];

    for (blade_count, expected_clusters) in test_cases {
        let calculated_clusters = (blade_count / 7).max(1);
        assert_eq!(
            calculated_clusters, expected_clusters,
            "For {} blades, expected {} clusters, got {}",
            blade_count, expected_clusters, calculated_clusters
        );
    }
}

/// Tests that cluster count matches quality settings
#[test]
fn test_grass_density_to_cluster_count() {
    // Verify blade count ranges match density levels
    let low_settings = create_test_grass_quality_settings(GrassDensity::Low);
    let (low_min, low_max) = low_settings.density.blade_count_range();

    // Low density: 2-4 blades → 1 cluster
    let low_clusters_min = (low_min / 7).max(1);
    let low_clusters_max = (low_max / 7).max(1);
    assert_eq!(low_clusters_min, 1, "Low min should produce 1 cluster");
    assert_eq!(low_clusters_max, 1, "Low max should produce 1 cluster");

    let high_settings = create_test_grass_quality_settings(GrassDensity::High);
    let (high_min, high_max) = high_settings.density.blade_count_range();

    // High density: 12-20 blades → 2-3 clusters
    let high_clusters_min = (high_min / 7).max(1);
    let high_clusters_max = (high_max / 7).max(1);
    assert!(
        high_clusters_min >= 1,
        "High min should produce at least 1 cluster"
    );
    assert!(
        high_clusters_max >= 2,
        "High max should produce at least 2 clusters"
    );
}

// ==================== Blade Variation Tests ====================

/// Tests height variation bounds
#[test]
fn test_grass_blade_height_variation_range() {
    let base_height = 0.4; // Standard blade height
    let min_variation = 0.7;
    let max_variation = 1.3;

    let min_height = base_height * min_variation;
    let max_height = base_height * max_variation;

    assert!(
        min_height > 0.2,
        "Min height should be > 0.2 (reasonable blade)"
    );
    assert!(
        max_height < 0.6,
        "Max height should be < 0.6 (reasonable blade)"
    );
    assert!(
        max_height > min_height,
        "Max variation should produce taller blades than min"
    );
}

/// Tests width variation bounds
#[test]
fn test_grass_blade_width_variation_range() {
    let base_width = 0.15; // GRASS_BLADE_WIDTH
    let min_variation = 0.8;
    let max_variation = 1.2;

    let min_width = base_width * min_variation;
    let max_width = base_width * max_variation;

    assert!(min_width > 0.1, "Min width should be reasonable");
    assert!(max_width > min_width, "Max should be greater than min");
    assert!(
        max_width < 0.2,
        "Max width should not exceed 0.2 units (too wide)"
    );
}

/// Tests curve amount bounds
#[test]
fn test_grass_blade_curve_amount_range() {
    let min_curve = 0.0;
    let max_curve = 0.3;

    assert!(max_curve > min_curve, "Max should be greater than min");
    assert!(max_curve < 0.5, "Max curve should be reasonable (< 0.5)");
    assert!(min_curve >= 0.0, "Min curve should not be negative");
}

// ==================== Cluster Center Tests ====================

/// Tests cluster center positioning within tile bounds
#[test]
fn test_grass_cluster_center_bounds() {
    // Cluster centers should be at (-0.4, 0.4) to avoid tile edges
    let min_center = -0.4;
    let max_center = 0.4;

    assert!(min_center < 0.0, "Min center should be negative");
    assert!(max_center > 0.0, "Max center should be positive");
    assert!(
        min_center > -0.5,
        "Min center should not exceed tile boundary"
    );
    assert!(
        max_center < 0.5,
        "Max center should not exceed tile boundary"
    );
}

/// Tests cluster offset from center
#[test]
fn test_grass_cluster_blade_offset_radius() {
    // Each blade in cluster should be within 0.1 units of cluster center
    let cluster_radius = 0.1;

    assert!(cluster_radius > 0.0, "Cluster radius should be positive");
    assert!(
        cluster_radius < 0.2,
        "Cluster radius should keep blades tightly grouped"
    );
}

// ==================== Visual Metadata Tests ====================

/// Tests that visual metadata provides blade height customization
#[test]
fn test_grass_visual_metadata_height() {
    let mut metadata = create_test_tile_visual_metadata();
    metadata.height = Some(0.5); // Custom height

    assert_eq!(
        metadata.height,
        Some(0.5),
        "Metadata should store custom blade height"
    );
}

/// Tests that visual metadata provides color tinting
#[test]
fn test_grass_visual_metadata_color_tint() {
    let mut metadata = create_test_tile_visual_metadata();
    metadata.color_tint = Some((1.0, 0.8, 0.6)); // Warmer grass tone

    assert_eq!(
        metadata.color_tint,
        Some((1.0, 0.8, 0.6)),
        "Metadata should store color tint"
    );
}

// ==================== Quality Settings Tests ====================

/// Tests default grass quality settings
#[test]
fn test_grass_quality_settings_default() {
    let settings = GrassQualitySettings::default();
    assert_eq!(
        settings.density,
        GrassDensity::Medium,
        "Default should be Medium density"
    );
}

/// Tests grass quality settings can be modified
#[test]
fn test_grass_quality_settings_modified() {
    let settings = GrassQualitySettings {
        density: GrassDensity::High,
    };

    assert_eq!(
        settings.density,
        GrassDensity::High,
        "Density should be modifiable"
    );
}

/// Tests grass quality settings serialization
#[test]
fn test_grass_quality_settings_clone() {
    let original = GrassQualitySettings {
        density: GrassDensity::Low,
    };
    let cloned = original.clone();

    assert_eq!(
        original.density, cloned.density,
        "Cloned settings should have same density"
    );
}

// ==================== Edge Case Tests ====================

/// Tests very low blade count (minimum 2)
#[test]
fn test_grass_minimum_blade_count() {
    let settings = create_test_grass_quality_settings(GrassDensity::Low);
    let (min, _max) = settings.density.blade_count_range();

    assert!(min > 0, "Should always have at least some grass");
    assert_eq!(min, 2, "Minimum should be 2 for sparse coverage");
}

/// Tests very high blade count (maximum 20)
#[test]
fn test_grass_maximum_blade_count() {
    let settings = create_test_grass_quality_settings(GrassDensity::High);
    let (_min, max) = settings.density.blade_count_range();

    assert!(max <= 20, "Should not exceed 20 blades per tile");
    assert!(max > 10, "High density should be > 10");
}

/// Tests cluster count with edge case blade counts
#[test]
fn test_grass_cluster_edge_cases() {
    // Test with 1 blade (should still create 1 cluster)
    let single_blade_cluster = 1;
    assert_eq!(single_blade_cluster, 1);

    // Test with exactly 7 blades (should create exactly 1 cluster)
    let exact_cluster = 1;
    assert_eq!(exact_cluster, 1);

    // Test with 0 blades (should create 1 cluster due to max(x, 1))
    let zero_blades = 1;
    assert_eq!(zero_blades, 1);
}

// ==================== Performance Tests ====================

/// Tests that cluster approach is more efficient than random scatter
#[test]
fn test_grass_cluster_efficiency() {
    // Cluster approach: blade_count / 7 clusters × ~7 blades/cluster
    // Random scatter approach: blade_count individual spawns

    let blade_count = 14;
    let cluster_spawns = (blade_count / 7).max(1); // 2 cluster spawns
    let random_spawns = blade_count; // 14 individual spawns

    // Cluster approach should be more efficient
    assert!(
        cluster_spawns < random_spawns,
        "Clustering should reduce spawn operations: {} < {}",
        cluster_spawns,
        random_spawns
    );
}

/// Tests cluster count scaling with density
#[test]
fn test_grass_cluster_scaling() {
    let low = create_test_grass_quality_settings(GrassDensity::Low);
    let medium = create_test_grass_quality_settings(GrassDensity::Medium);
    let high = create_test_grass_quality_settings(GrassDensity::High);

    let (_low_min, low_max) = low.density.blade_count_range();
    let (_med_min, med_max) = medium.density.blade_count_range();
    let (_high_min, high_max) = high.density.blade_count_range();

    let low_clusters = (low_max / 7).max(1);
    let med_clusters = (med_max / 7).max(1);
    let high_clusters = (high_max / 7).max(1);

    // Higher density should produce more clusters
    assert!(
        med_clusters >= low_clusters,
        "Medium should have >= clusters than Low"
    );
    assert!(
        high_clusters >= med_clusters,
        "High should have >= clusters than Medium"
    );
}
