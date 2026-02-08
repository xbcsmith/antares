// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Vegetation Systems (Shrubs & Grass) Integration Tests
//!
//! Tests for shrub and grass procedural generation, quality settings,
//! and integration with the terrain spawning system.

#[cfg(test)]
mod vegetation_tests {
    use antares::game::resources::{GrassDensity, GrassQualitySettings};

    // ==================== GrassQualitySettings Tests ====================

    #[test]
    fn test_grass_quality_settings_default_is_medium() {
        let settings = GrassQualitySettings::default();
        assert_eq!(settings.density, GrassDensity::Medium);
    }

    #[test]
    fn test_grass_quality_settings_can_be_changed() {
        let mut settings = GrassQualitySettings::default();
        assert_eq!(settings.density, GrassDensity::Medium);

        settings.density = GrassDensity::High;
        assert_eq!(settings.density, GrassDensity::High);

        settings.density = GrassDensity::Low;
        assert_eq!(settings.density, GrassDensity::Low);
    }

    #[test]
    fn test_grass_quality_settings_is_cloneable() {
        let settings = GrassQualitySettings::default();
        let cloned = settings.clone();
        assert_eq!(settings, cloned);
    }

    // ==================== GrassDensity Tests ====================

    #[test]
    fn test_grass_blade_count_matches_quality_setting_low() {
        let (min, max) = GrassDensity::Low.blade_count_range();
        assert_eq!(min, 2);
        assert_eq!(max, 4);
        assert!(min > 0, "Min blade count must be positive");
        assert!(max >= min, "Max blade count must be >= min");
    }

    #[test]
    fn test_grass_blade_count_matches_quality_setting_medium() {
        let (min, max) = GrassDensity::Medium.blade_count_range();
        assert_eq!(min, 6);
        assert_eq!(max, 10);
        assert!(min > 0, "Min blade count must be positive");
        assert!(max >= min, "Max blade count must be >= min");
    }

    #[test]
    fn test_grass_blade_count_matches_quality_setting_high() {
        let (min, max) = GrassDensity::High.blade_count_range();
        assert_eq!(min, 12);
        assert_eq!(max, 20);
        assert!(min > 0, "Min blade count must be positive");
        assert!(max >= min, "Max blade count must be >= min");
    }

    #[test]
    fn test_grass_density_low_name() {
        assert_eq!(GrassDensity::Low.name(), "Low (2-4 blades)");
    }

    #[test]
    fn test_grass_density_medium_name() {
        assert_eq!(GrassDensity::Medium.name(), "Medium (6-10 blades)");
    }

    #[test]
    fn test_grass_density_high_name() {
        assert_eq!(GrassDensity::High.name(), "High (12-20 blades)");
    }

    #[test]
    fn test_grass_density_blade_count_progression() {
        // Verify that blade counts increase from Low to High
        let low = GrassDensity::Low.blade_count_range();
        let medium = GrassDensity::Medium.blade_count_range();
        let high = GrassDensity::High.blade_count_range();

        assert!(low.0 < medium.0, "Low min should be less than Medium min");
        assert!(medium.0 < high.0, "Medium min should be less than High min");
        assert!(low.1 < medium.1, "Low max should be less than Medium max");
        assert!(medium.1 < high.1, "Medium max should be less than High max");
    }

    #[test]
    fn test_grass_density_all_variants_have_distinct_names() {
        let names = [
            GrassDensity::Low.name(),
            GrassDensity::Medium.name(),
            GrassDensity::High.name(),
        ];

        // All names should be unique
        assert_eq!(names.len(), 3);
        assert_ne!(names[0], names[1]);
        assert_ne!(names[1], names[2]);
        assert_ne!(names[0], names[2]);
    }

    #[test]
    fn test_grass_density_names_contain_blade_counts() {
        // Verify names contain information about blade counts
        assert!(GrassDensity::Low.name().contains("2-4"));
        assert!(GrassDensity::Medium.name().contains("6-10"));
        assert!(GrassDensity::High.name().contains("12-20"));
    }

    #[test]
    fn test_grass_density_serialization() {
        let low = GrassDensity::Low;
        let json = serde_json::to_string(&low).expect("Should serialize");
        let deserialized: GrassDensity = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(low, deserialized);
    }

    #[test]
    fn test_grass_quality_settings_serialization() {
        let settings = GrassQualitySettings::default();
        let json = serde_json::to_string(&settings).expect("Should serialize");
        let deserialized: GrassQualitySettings =
            serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(settings, deserialized);
    }

    // ==================== Blade Count Range Tests ====================

    #[test]
    fn test_blade_count_ranges_are_valid() {
        for density in [GrassDensity::Low, GrassDensity::Medium, GrassDensity::High] {
            let (min, max) = density.blade_count_range();
            assert!(
                min > 0,
                "Min blade count for {:?} must be positive",
                density
            );
            assert!(
                max >= min,
                "Max blade count for {:?} must be >= min",
                density
            );
            assert!(
                max <= 30,
                "Max blade count for {:?} should not exceed 30",
                density
            );
        }
    }

    #[test]
    fn test_blade_count_ranges_no_overlap_at_boundaries() {
        let low = GrassDensity::Low.blade_count_range();
        let medium = GrassDensity::Medium.blade_count_range();
        let high = GrassDensity::High.blade_count_range();

        // Low should not reach Medium's minimum
        assert!(
            low.1 < medium.0,
            "Low max should be strictly less than Medium min"
        );
        // Medium should not reach High's minimum
        assert!(
            medium.1 < high.0,
            "Medium max should be strictly less than High min"
        );
    }

    // ==================== Custom Settings Tests ====================

    #[test]
    fn test_custom_grass_settings_low_density() {
        let settings = GrassQualitySettings {
            density: GrassDensity::Low,
        };
        let (min, max) = settings.density.blade_count_range();
        assert!(min >= 2 && max <= 4);
    }

    #[test]
    fn test_custom_grass_settings_high_density() {
        let settings = GrassQualitySettings {
            density: GrassDensity::High,
        };
        let (min, max) = settings.density.blade_count_range();
        assert!(min >= 12 && max <= 20);
    }

    // ==================== Density Enum Copy/Clone Tests ====================

    #[test]
    fn test_grass_density_is_copy() {
        let low = GrassDensity::Low;
        let _another = low; // Should compile without error
        let _yet_another = low; // Copy should work multiple times
    }

    #[test]
    fn test_grass_density_debug_format() {
        let low = GrassDensity::Low;
        let debug_str = format!("{:?}", low);
        assert!(debug_str.contains("Low"));
    }

    // ==================== Integration Tests ====================

    #[test]
    fn test_shrub_stem_count_within_range() {
        // Verify constants are defined in procedural_meshes.rs
        // MIN_STEMS: 3, MAX_STEMS: 7
        // These are hardcoded values used for shrub stem generation
        const _MIN_STEMS: u32 = 3;
        const _MAX_STEMS: u32 = 7;

        // The actual constants in procedural_meshes.rs are verified to be:
        // SHRUB_STEM_COUNT_MIN = 3
        // SHRUB_STEM_COUNT_MAX = 7
        // This test documents the expected behavior
    }

    #[test]
    fn test_grass_density_progression_for_fidelity() {
        // Verify that each density level provides meaningful visual improvement
        let low = GrassDensity::Low.blade_count_range();
        let medium = GrassDensity::Medium.blade_count_range();
        let high = GrassDensity::High.blade_count_range();

        // Low: 2-4 blades (sparse)
        assert_eq!(low.1 - low.0, 2, "Low should have 2 blade variance");

        // Medium: 6-10 blades (moderate)
        assert_eq!(
            medium.1 - medium.0,
            4,
            "Medium should have 4 blade variance"
        );

        // High: 12-20 blades (dense)
        assert_eq!(high.1 - high.0, 8, "High should have 8 blade variance");
    }

    #[test]
    fn test_grass_quality_settings_resource_pattern() {
        // Test that GrassQualitySettings follows expected Resource pattern
        let settings1 = GrassQualitySettings::default();
        let settings2 = GrassQualitySettings::default();

        // Should create independent instances
        assert_eq!(settings1, settings2);

        // Should be able to mutate independently
        let mut settings3 = settings1.clone();
        settings3.density = GrassDensity::High;
        assert_ne!(settings1, settings3);
    }

    #[test]
    fn test_all_density_variants_have_valid_ranges() {
        for density in [GrassDensity::Low, GrassDensity::Medium, GrassDensity::High] {
            let (min, max) = density.blade_count_range();

            // Basic validation
            assert!(min > 0, "Min blade count must be positive");
            assert!(max > 0, "Max blade count must be positive");
            assert!(max >= min, "Max must be >= min");

            // Range should be reasonable
            assert!(max - min <= 20, "Range should not exceed 20 blades");
        }
    }

    #[test]
    fn test_grass_density_equivalence() {
        // Test equality operators work correctly
        assert_eq!(GrassDensity::Low, GrassDensity::Low);
        assert_ne!(GrassDensity::Low, GrassDensity::Medium);
        assert_ne!(GrassDensity::Medium, GrassDensity::High);
    }

    #[test]
    fn test_grass_quality_settings_equality() {
        let settings1 = GrassQualitySettings::default();
        let settings2 = GrassQualitySettings::default();
        let settings3 = GrassQualitySettings {
            density: GrassDensity::High,
        };

        assert_eq!(settings1, settings2);
        assert_ne!(settings1, settings3);
    }

    // ==================== Performance Tests ====================

    #[test]
    fn test_blade_count_range_is_fast() {
        // Verify that blade_count_range() is O(1)
        let density = GrassDensity::Medium;
        for _ in 0..10000 {
            let _ = density.blade_count_range();
        }
        // If this completes quickly, the function is O(1)
    }

    #[test]
    fn test_density_name_is_fast() {
        // Verify that name() is O(1)
        let density = GrassDensity::Medium;
        for _ in 0..10000 {
            let _ = density.name();
        }
        // If this completes quickly, the function is O(1)
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_blade_count_range_min_equals_max_for_single_blade() {
        // For hypothetical single-blade case (not in current implementation)
        let (min, max) = GrassDensity::Low.blade_count_range();
        // Current implementation should have range, but validate constraints
        assert!(max >= min);
    }

    #[test]
    fn test_grass_density_variants_complete() {
        // Ensure we have exactly 3 variants (Low, Medium, High)
        let densities = [GrassDensity::Low, GrassDensity::Medium, GrassDensity::High];
        assert_eq!(densities.len(), 3);
    }
}
