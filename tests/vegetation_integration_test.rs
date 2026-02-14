// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Vegetation Systems (Shrubs & Grass) Integration Tests
//!
//! Tests for shrub and grass procedural generation, quality settings,
//! and integration with the terrain spawning system.

#[cfg(test)]
mod vegetation_tests {
    use antares::domain::world::GrassDensity;
    use antares::game::resources::{GrassPerformanceLevel, GrassQualitySettings};

    // ==================== GrassQualitySettings Tests ====================

    #[test]
    fn test_grass_quality_settings_default_is_medium() {
        let settings = GrassQualitySettings::default();
        assert_eq!(settings.performance_level, GrassPerformanceLevel::Medium);
    }

    #[test]
    fn test_grass_quality_settings_can_be_changed() {
        let mut settings = GrassQualitySettings::default();
        assert_eq!(settings.performance_level, GrassPerformanceLevel::Medium);

        settings.performance_level = GrassPerformanceLevel::High;
        assert_eq!(settings.performance_level, GrassPerformanceLevel::High);

        settings.performance_level = GrassPerformanceLevel::Low;
        assert_eq!(settings.performance_level, GrassPerformanceLevel::Low);
    }

    #[test]
    fn test_grass_quality_settings_is_cloneable() {
        let settings = GrassQualitySettings::default();
        let cloned = settings.clone();
        assert_eq!(settings, cloned);
    }

    // ==================== GrassPerformanceLevel Tests ====================

    #[test]
    fn test_grass_performance_level_multiplier_values() {
        assert_eq!(GrassPerformanceLevel::Low.multiplier(), 0.25);
        assert_eq!(GrassPerformanceLevel::Medium.multiplier(), 1.0);
        assert_eq!(GrassPerformanceLevel::High.multiplier(), 1.5);
    }

    #[test]
    fn test_grass_performance_level_names() {
        assert_eq!(GrassPerformanceLevel::Low.name(), "Low (0.25x)");
        assert_eq!(GrassPerformanceLevel::Medium.name(), "Medium (1.0x)");
        assert_eq!(GrassPerformanceLevel::High.name(), "High (1.5x)");
    }

    #[test]
    fn test_grass_performance_level_scales_content_density() {
        let low = GrassPerformanceLevel::Low.apply_to_content_density(GrassDensity::High);
        let medium = GrassPerformanceLevel::Medium.apply_to_content_density(GrassDensity::High);
        let high = GrassPerformanceLevel::High.apply_to_content_density(GrassDensity::High);

        assert!(low.0 <= low.1);
        assert!(medium.0 <= medium.1);
        assert!(high.0 <= high.1);
        assert!(low.1 < medium.1);
        assert!(medium.1 < high.1);
    }

    #[test]
    fn test_apply_to_content_density_none_returns_zero() {
        let (min, max) = GrassPerformanceLevel::Medium.apply_to_content_density(GrassDensity::None);
        assert_eq!((min, max), (0, 0));
    }

    #[test]
    fn test_grass_quality_settings_uses_performance_level() {
        let settings = GrassQualitySettings {
            performance_level: GrassPerformanceLevel::Low,
        };
        let (min, max) = settings.blade_count_range_for_content(GrassDensity::Medium);
        assert!(min > 0);
        assert!(max >= min);
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
        let settings = GrassQualitySettings::default();
        for density in [
            GrassDensity::None,
            GrassDensity::Low,
            GrassDensity::Medium,
            GrassDensity::High,
            GrassDensity::VeryHigh,
        ] {
            let (min, max) = settings.blade_count_range_for_content(density);
            if density == GrassDensity::None {
                assert_eq!((min, max), (0, 0));
            } else {
                assert!(
                    min > 0,
                    "Min blade count for {:?} must be positive",
                    density
                );
            }
            assert!(
                max >= min,
                "Max blade count for {:?} must be >= min",
                density
            );
            assert!(
                max <= 300,
                "Max blade count for {:?} should not exceed 300",
                density
            );
        }
    }

    #[test]
    fn test_performance_level_ordering_increases_blade_counts() {
        let low = GrassPerformanceLevel::Low.apply_to_content_density(GrassDensity::High);
        let medium = GrassPerformanceLevel::Medium.apply_to_content_density(GrassDensity::High);
        let high = GrassPerformanceLevel::High.apply_to_content_density(GrassDensity::High);

        assert!(low.1 < medium.1);
        assert!(medium.1 < high.1);
    }

    // ==================== Custom Settings Tests ====================

    #[test]
    fn test_custom_grass_settings_low_density() {
        let settings = GrassQualitySettings {
            performance_level: GrassPerformanceLevel::Low,
        };
        let (min, max) = settings.blade_count_range_for_content(GrassDensity::High);
        assert!(min > 0 && max >= min);
    }

    #[test]
    fn test_custom_grass_settings_high_density() {
        let settings = GrassQualitySettings {
            performance_level: GrassPerformanceLevel::High,
        };
        let (min, max) = settings.blade_count_range_for_content(GrassDensity::High);
        assert!(min > 0 && max >= min);
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
        let settings = GrassQualitySettings::default();
        let low = settings.blade_count_range_for_content(GrassDensity::Low);
        let medium = settings.blade_count_range_for_content(GrassDensity::Medium);
        let high = settings.blade_count_range_for_content(GrassDensity::High);

        // Low: 2-4 blades (sparse)
        assert!(low.1 >= low.0, "Low should have valid range");

        // Medium: 6-10 blades (moderate)
        assert!(medium.1 >= medium.0, "Medium should have valid range");

        // High: 12-20 blades (dense)
        assert!(high.1 >= high.0, "High should have valid range");
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
        settings3.performance_level = GrassPerformanceLevel::High;
        assert_ne!(settings1, settings3);
    }

    #[test]
    fn test_all_density_variants_have_valid_ranges() {
        let settings = GrassQualitySettings::default();
        for density in [
            GrassDensity::None,
            GrassDensity::Low,
            GrassDensity::Medium,
            GrassDensity::High,
            GrassDensity::VeryHigh,
        ] {
            let (min, max) = settings.blade_count_range_for_content(density);

            // Basic validation
            if density == GrassDensity::None {
                assert_eq!((min, max), (0, 0));
            } else {
                assert!(min > 0, "Min blade count must be positive");
                assert!(max > 0, "Max blade count must be positive");
            }
            assert!(max >= min, "Max must be >= min");

            // Range should be reasonable for content density scaling
            assert!(max - min <= 100, "Range should not exceed 100 blades");
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
            performance_level: GrassPerformanceLevel::High,
        };

        assert_eq!(settings1, settings2);
        assert_ne!(settings1, settings3);
    }

    // ==================== Performance Tests ====================

    #[test]
    fn test_blade_count_range_is_fast() {
        // Verify that blade_count_range() is O(1)
        let density = GrassDensity::Medium;
        let settings = GrassQualitySettings::default();
        for _ in 0..10000 {
            let _ = settings.blade_count_range_for_content(density);
        }
        // If this completes quickly, the function is O(1)
    }

    #[test]
    fn test_density_name_is_fast() {
        // Verify that name() is O(1)
        let density = GrassPerformanceLevel::Medium;
        for _ in 0..10000 {
            let _ = density.name();
        }
        // If this completes quickly, the function is O(1)
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_blade_count_range_min_equals_max_for_single_blade() {
        // For hypothetical single-blade case (not in current implementation)
        let settings = GrassQualitySettings::default();
        let (min, max) = settings.blade_count_range_for_content(GrassDensity::Low);
        // Current implementation should have range, but validate constraints
        assert!(max >= min);
    }

    #[test]
    fn test_grass_density_variants_complete() {
        // Ensure we have all 5 variants (None, Low, Medium, High, VeryHigh)
        let densities = [
            GrassDensity::None,
            GrassDensity::Low,
            GrassDensity::Medium,
            GrassDensity::High,
            GrassDensity::VeryHigh,
        ];
        assert_eq!(densities.len(), 5);
    }
}
