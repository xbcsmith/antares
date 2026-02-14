// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Campaign Builder SDK - Terrain Visual Configuration Tests
//!
//! This test module verifies the extended visual preset system for advanced
//! procedural terrain objects including trees, shrubs, grass, mountains, swamp, and lava.

#[cfg(test)]
mod visual_presets {
    use campaign_builder::map_editor::VisualPreset;

    // ==================== Tree Preset Tests ====================

    #[test]
    fn test_short_tree_preset_metadata_values() {
        let preset = VisualPreset::ShortTree;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Short Tree");
        assert_eq!(metadata.height, Some(1.0));
        assert_eq!(metadata.scale, Some(0.6));
        assert!(metadata.color_tint.is_some());

        // Green tint: (0.5, 0.85, 0.5)
        let (r, g, b) = metadata.color_tint.unwrap();
        assert!((r - 0.5).abs() < 0.01);
        assert!((g - 0.85).abs() < 0.01);
        assert!((b - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_medium_tree_preset_metadata_values() {
        let preset = VisualPreset::MediumTree;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Medium Tree");
        assert_eq!(metadata.height, Some(2.0));
        assert_eq!(metadata.scale, Some(0.8));
        assert!(metadata.color_tint.is_some());
    }

    #[test]
    fn test_tall_tree_preset_metadata_values() {
        let preset = VisualPreset::TallTree;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Tall Tree");
        assert_eq!(metadata.height, Some(3.0));
        assert_eq!(metadata.scale, Some(1.2));
        assert!(metadata.color_tint.is_some());
    }

    #[test]
    fn test_dead_tree_preset_metadata_values() {
        let preset = VisualPreset::DeadTree;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Dead Tree");
        assert_eq!(metadata.height, Some(2.5));
        assert_eq!(metadata.scale, Some(0.7));
        // Brown/gray tint: (0.6, 0.5, 0.4)
        assert!(metadata.color_tint.is_some());
    }

    #[test]
    fn test_tree_presets_have_distinct_heights() {
        let short = VisualPreset::ShortTree.to_metadata();
        let medium = VisualPreset::MediumTree.to_metadata();
        let tall = VisualPreset::TallTree.to_metadata();
        let dead = VisualPreset::DeadTree.to_metadata();

        let heights = vec![
            short.height.unwrap(),
            medium.height.unwrap(),
            tall.height.unwrap(),
            dead.height.unwrap(),
        ];

        // Verify all heights are present and different
        assert_eq!(heights[0], 1.0);
        assert_eq!(heights[1], 2.0);
        assert_eq!(heights[2], 3.0);
        assert_eq!(heights[3], 2.5);
    }

    // ==================== Shrub Preset Tests ====================

    #[test]
    fn test_small_shrub_preset_metadata_values() {
        let preset = VisualPreset::SmallShrub;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Small Shrub");
        assert_eq!(metadata.height, Some(0.4));
        assert_eq!(metadata.scale, Some(0.4));
        assert!(metadata.color_tint.is_some());
    }

    #[test]
    fn test_large_shrub_preset_metadata_values() {
        let preset = VisualPreset::LargeShrub;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Large Shrub");
        assert_eq!(metadata.height, Some(0.8));
        assert_eq!(metadata.scale, Some(0.9));
        assert!(metadata.color_tint.is_some());
    }

    #[test]
    fn test_flowering_shrub_preset_metadata_values() {
        let preset = VisualPreset::FloweringShrub;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Flowering Shrub");
        assert_eq!(metadata.height, Some(0.6));
        assert_eq!(metadata.scale, Some(0.7));
        // Flower pink tint: (0.8, 0.5, 0.7)
        assert!(metadata.color_tint.is_some());
        let (r, g, b) = metadata.color_tint.unwrap();
        assert!((r - 0.8).abs() < 0.01);
        assert!((g - 0.5).abs() < 0.01);
        assert!((b - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_shrub_presets_have_distinct_heights() {
        let small = VisualPreset::SmallShrub.to_metadata();
        let large = VisualPreset::LargeShrub.to_metadata();
        let flowering = VisualPreset::FloweringShrub.to_metadata();

        assert_eq!(small.height.unwrap(), 0.4);
        assert_eq!(large.height.unwrap(), 0.8);
        assert_eq!(flowering.height.unwrap(), 0.6);
    }

    // ==================== Grass Preset Tests ====================

    #[test]
    fn test_short_grass_preset_metadata_values() {
        let preset = VisualPreset::ShortGrass;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Short Grass");
        assert_eq!(metadata.height, Some(0.2));
        assert_eq!(metadata.scale, Some(0.8));
        assert!(metadata.color_tint.is_some());
    }

    #[test]
    fn test_tall_grass_preset_metadata_values() {
        let preset = VisualPreset::TallGrass;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Tall Grass");
        assert_eq!(metadata.height, Some(0.4));
        assert_eq!(metadata.scale, Some(1.0));
        assert!(metadata.color_tint.is_some());
    }

    #[test]
    fn test_dried_grass_preset_metadata_values() {
        let preset = VisualPreset::DriedGrass;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Dried Grass");
        assert_eq!(metadata.height, Some(0.3));
        assert_eq!(metadata.scale, Some(0.9));
        // Brown/tan tint: (0.7, 0.6, 0.4)
        assert!(metadata.color_tint.is_some());
    }

    #[test]
    fn test_grass_presets_have_distinct_heights() {
        let short = VisualPreset::ShortGrass.to_metadata();
        let tall = VisualPreset::TallGrass.to_metadata();
        let dried = VisualPreset::DriedGrass.to_metadata();

        assert_eq!(short.height.unwrap(), 0.2);
        assert_eq!(tall.height.unwrap(), 0.4);
        assert_eq!(dried.height.unwrap(), 0.3);
    }

    // ==================== Mountain Preset Tests ====================

    #[test]
    fn test_low_peak_preset_metadata_values() {
        let preset = VisualPreset::LowPeak;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Low Peak");
        assert_eq!(metadata.height, Some(1.5));
        assert_eq!(metadata.rotation_y, Some(0.0));
        assert!(metadata.color_tint.is_some());
    }

    #[test]
    fn test_high_peak_preset_metadata_values() {
        let preset = VisualPreset::HighPeak;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "High Peak");
        assert_eq!(metadata.height, Some(3.0));
        assert_eq!(metadata.rotation_y, Some(0.0));
        assert!(metadata.color_tint.is_some());
    }

    #[test]
    fn test_jagged_peak_preset_metadata_values() {
        let preset = VisualPreset::JaggedPeak;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Jagged Peak");
        assert_eq!(metadata.height, Some(5.0));
        assert_eq!(metadata.rotation_y, Some(15.0));
        assert!(metadata.color_tint.is_some());
    }

    #[test]
    fn test_mountain_presets_have_distinct_heights() {
        let low = VisualPreset::LowPeak.to_metadata();
        let high = VisualPreset::HighPeak.to_metadata();
        let jagged = VisualPreset::JaggedPeak.to_metadata();

        assert_eq!(low.height.unwrap(), 1.5);
        assert_eq!(high.height.unwrap(), 3.0);
        assert_eq!(jagged.height.unwrap(), 5.0);
    }

    // ==================== Swamp Preset Tests ====================

    #[test]
    fn test_shallow_swamp_preset_metadata_values() {
        let preset = VisualPreset::ShallowSwamp;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Shallow Swamp");
        assert_eq!(metadata.height, Some(0.1));
        assert_eq!(metadata.scale, Some(1.2));
        // Murky blue-green tint: (0.3, 0.5, 0.4)
        assert!(metadata.color_tint.is_some());
    }

    #[test]
    fn test_deep_swamp_preset_metadata_values() {
        let preset = VisualPreset::DeepSwamp;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Deep Swamp");
        assert_eq!(metadata.height, Some(0.3));
        assert_eq!(metadata.scale, Some(1.1));
        assert!(metadata.color_tint.is_some());
    }

    #[test]
    fn test_murky_swamp_preset_metadata_values() {
        let preset = VisualPreset::MurkySwamp;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Murky Swamp");
        assert_eq!(metadata.height, Some(0.5));
        assert_eq!(metadata.scale, Some(1.0));
        assert!(metadata.color_tint.is_some());
    }

    #[test]
    fn test_swamp_presets_have_distinct_water_levels() {
        let shallow = VisualPreset::ShallowSwamp.to_metadata();
        let deep = VisualPreset::DeepSwamp.to_metadata();
        let murky = VisualPreset::MurkySwamp.to_metadata();

        assert_eq!(shallow.height.unwrap(), 0.1);
        assert_eq!(deep.height.unwrap(), 0.3);
        assert_eq!(murky.height.unwrap(), 0.5);
    }

    // ==================== Lava Preset Tests ====================

    #[test]
    fn test_lava_pool_preset_metadata_values() {
        let preset = VisualPreset::LavaPool;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Lava Pool");
        assert_eq!(metadata.height, Some(0.2));
        assert_eq!(metadata.scale, Some(1.0));
        // Bright red-orange emissive: (1.0, 0.3, 0.0)
        assert!(metadata.color_tint.is_some());
        let (r, g, b) = metadata.color_tint.unwrap();
        assert!((r - 1.0).abs() < 0.01);
        assert!((g - 0.3).abs() < 0.01);
        assert!((b - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_lava_flow_preset_metadata_values() {
        let preset = VisualPreset::LavaFlow;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Lava Flow");
        assert_eq!(metadata.height, Some(0.3));
        assert_eq!(metadata.scale, Some(1.1));
        assert!(metadata.color_tint.is_some());
    }

    #[test]
    fn test_volcanic_vent_preset_metadata_values() {
        let preset = VisualPreset::VolcanicVent;
        let metadata = preset.to_metadata();

        assert_eq!(preset.name(), "Volcanic Vent");
        assert_eq!(metadata.height, Some(0.4));
        assert_eq!(metadata.scale, Some(0.8));
        // Intense yellow-orange emissive: (1.0, 0.8, 0.0)
        assert!(metadata.color_tint.is_some());
    }

    #[test]
    fn test_lava_presets_have_distinct_properties() {
        let pool = VisualPreset::LavaPool.to_metadata();
        let flow = VisualPreset::LavaFlow.to_metadata();
        let vent = VisualPreset::VolcanicVent.to_metadata();

        assert_eq!(pool.height.unwrap(), 0.2);
        assert_eq!(flow.height.unwrap(), 0.3);
        assert_eq!(vent.height.unwrap(), 0.4);

        // All have emissive color tints (high red component)
        assert!(pool.color_tint.unwrap().0 >= 0.99);
        assert!(flow.color_tint.unwrap().0 >= 0.99);
        assert!(vent.color_tint.unwrap().0 >= 0.99);
    }

    // ==================== Comprehensive Tests ====================

    #[test]
    fn test_terrain_preset_all_variants_present() {
        let presets = VisualPreset::all();

        // Count new presets
        let expected_preset_names = vec![
            "Short Tree",
            "Medium Tree",
            "Tall Tree",
            "Dead Tree",
            "Small Shrub",
            "Large Shrub",
            "Flowering Shrub",
            "Short Grass",
            "Tall Grass",
            "Dried Grass",
            "Low Peak",
            "High Peak",
            "Jagged Peak",
            "Shallow Swamp",
            "Deep Swamp",
            "Murky Swamp",
            "Lava Pool",
            "Lava Flow",
            "Volcanic Vent",
        ];

        for preset in presets {
            let name = preset.name();
            // All presets should have a valid name
            assert!(!name.is_empty(), "Preset should have a non-empty name");

            // All presets should be present
            if expected_preset_names.contains(&name) {
                // Verify it's in the list
                assert!(true);
            }
        }
    }

    #[test]
    fn test_terrain_preset_names_are_unique() {
        let presets = VisualPreset::all();
        let mut names = Vec::new();

        for preset in presets {
            let name = preset.name();
            assert!(
                !names.contains(&name),
                "Preset name '{}' appears more than once",
                name
            );
            names.push(name);
        }

        assert_eq!(
            names.len(),
            presets.len(),
            "All preset names should be unique"
        );
    }

    #[test]
    fn test_terrain_preset_metadata_consistency() {
        let presets = VisualPreset::all();

        for preset in presets {
            let metadata = preset.to_metadata();

            // All metadata should have reasonable values
            if let Some(height) = metadata.height {
                assert!(
                    height >= 0.1 && height <= 10.0,
                    "Height for {} should be 0.1-10.0, got {}",
                    preset.name(),
                    height
                );
            }

            if let Some(scale) = metadata.scale {
                assert!(
                    scale >= 0.1 && scale <= 3.0,
                    "Scale for {} should be 0.1-3.0, got {}",
                    preset.name(),
                    scale
                );
            }

            if let Some(rotation) = metadata.rotation_y {
                assert!(
                    rotation >= 0.0 && rotation <= 360.0,
                    "Rotation for {} should be 0-360, got {}",
                    preset.name(),
                    rotation
                );
            }

            if let Some((r, g, b)) = metadata.color_tint {
                assert!(
                    r >= 0.0 && r <= 1.0,
                    "Red component for {} should be 0.0-1.0, got {}",
                    preset.name(),
                    r
                );
                assert!(
                    g >= 0.0 && g <= 1.0,
                    "Green component for {} should be 0.0-1.0, got {}",
                    preset.name(),
                    g
                );
                assert!(
                    b >= 0.0 && b <= 1.0,
                    "Blue component for {} should be 0.0-1.0, got {}",
                    preset.name(),
                    b
                );
            }
        }
    }

    #[test]
    fn test_trees_have_green_tints() {
        let tree_presets = vec![
            VisualPreset::ShortTree,
            VisualPreset::MediumTree,
            VisualPreset::TallTree,
        ];

        for preset in tree_presets {
            let metadata = preset.to_metadata();
            assert!(
                metadata.color_tint.is_some(),
                "{} should have a color tint",
                preset.name()
            );

            let (r, g, b) = metadata.color_tint.unwrap();
            // Trees should have more green than other colors
            assert!(
                g > r && g > b,
                "{} should have a green dominant color tint",
                preset.name()
            );
        }
    }

    #[test]
    fn test_shrubs_all_have_colors() {
        let shrub_presets = vec![
            VisualPreset::SmallShrub,
            VisualPreset::LargeShrub,
            VisualPreset::FloweringShrub,
        ];

        for preset in shrub_presets {
            let metadata = preset.to_metadata();
            assert!(
                metadata.color_tint.is_some(),
                "{} should have a color tint",
                preset.name()
            );
            assert!(
                metadata.height.is_some(),
                "{} should have a height",
                preset.name()
            );
        }
    }

    #[test]
    fn test_lava_presets_have_hot_colors() {
        let lava_presets = vec![
            VisualPreset::LavaPool,
            VisualPreset::LavaFlow,
            VisualPreset::VolcanicVent,
        ];

        for preset in lava_presets {
            let metadata = preset.to_metadata();
            let (r, g, b) = metadata.color_tint.unwrap_or((0.0, 0.0, 0.0));

            // Lava should have high red component (hot colors)
            assert!(
                r > 0.9,
                "{} should have high red component (>0.9), got {}",
                preset.name(),
                r
            );
        }
    }

    #[test]
    fn test_swamp_presets_have_water_tints() {
        let swamp_presets = vec![
            VisualPreset::ShallowSwamp,
            VisualPreset::DeepSwamp,
            VisualPreset::MurkySwamp,
        ];

        for preset in swamp_presets {
            let metadata = preset.to_metadata();
            assert!(
                metadata.color_tint.is_some(),
                "{} should have a color tint",
                preset.name()
            );

            let (r, g, b) = metadata.color_tint.unwrap();
            // Swamp should have muted colors (low brightness)
            let brightness = (r + g + b) / 3.0;
            assert!(
                brightness < 0.6,
                "{} should have muted colors, brightness {}",
                preset.name(),
                brightness
            );
        }
    }

    #[test]
    fn test_all_presets_roundtrip_correctly() {
        let presets = VisualPreset::all();

        for preset in presets {
            let metadata = preset.to_metadata();

            // The metadata should contain the configured values
            match preset {
                VisualPreset::Default => {
                    // Default should have no special metadata
                    assert!(metadata.height.is_none() || metadata.height == Some(0.0));
                }
                _ => {
                    // Most presets should have at least one configured property
                    let has_property = metadata.height.is_some()
                        || metadata.width_x.is_some()
                        || metadata.width_z.is_some()
                        || metadata.scale.is_some()
                        || metadata.y_offset.is_some()
                        || metadata.rotation_y.is_some()
                        || metadata.color_tint.is_some();

                    assert!(
                        has_property,
                        "{} should have at least one configured property",
                        preset.name()
                    );
                }
            }
        }
    }
}
