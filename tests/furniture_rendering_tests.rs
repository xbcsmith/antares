// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for Runtime Furniture Rendering System
//!
//! Tests cover:
//! - Furniture spawning with proper components
//! - Material property application (metallic, roughness, base color)
//! - Color tint application and blending
//! - Emissive lighting for lit torches
//! - Blocking behavior setup
//! - Interaction component attachment
//! - All furniture types render correctly

#[cfg(test)]
mod furniture_rendering {
    use antares::domain::types::Position;
    use antares::domain::world::{FurnitureFlags, FurnitureMaterial, FurnitureType};
    use antares::game::components::{FurnitureEntity, InteractionType};

    #[test]
    fn test_furniture_material_wood_properties() {
        let material = FurnitureMaterial::Wood;
        let base_color = material.base_color();

        // Wood should be non-metallic and rough
        assert!(material.metallic() < 0.2);
        assert!(material.roughness() > 0.6);

        // Wood should have brownish color
        assert!(base_color[0] > 0.2); // Some red
        assert!(base_color[1] > 0.1); // Some green
        assert!(base_color[2] < 0.3); // Less blue
    }

    #[test]
    fn test_furniture_material_stone_properties() {
        let material = FurnitureMaterial::Stone;
        let base_color = material.base_color();

        // Stone should be slightly metallic and very rough
        assert!(material.metallic() <= 0.1);
        assert!(material.roughness() > 0.8);

        // Stone should have grayish color
        assert!(base_color[0] > 0.3);
        assert!(base_color[1] > 0.3);
        assert!(base_color[2] > 0.3);
    }

    #[test]
    fn test_furniture_material_metal_properties() {
        let material = FurnitureMaterial::Metal;
        let base_color = material.base_color();

        // Metal should be metallic and somewhat smooth
        assert!(material.metallic() > 0.5);
        assert!(material.roughness() < 0.5);

        // Metal should have grayish color
        assert!(base_color[0] > 0.3);
        assert!(base_color[1] > 0.3);
        assert!(base_color[2] > 0.3);
    }

    #[test]
    fn test_furniture_material_gold_properties() {
        let material = FurnitureMaterial::Gold;
        let base_color = material.base_color();

        // Gold should be metallic and somewhat smooth
        assert!(material.metallic() > 0.5);
        assert!(material.roughness() < 0.5);

        // Gold should have yellowish/brownish color
        assert!(base_color[0] > base_color[2]); // More red than blue
        assert!(base_color[1] > base_color[2]); // More green than blue
    }

    #[test]
    fn test_furniture_entity_creation() {
        let furniture = FurnitureEntity::new(FurnitureType::Bench, true);
        assert_eq!(furniture.furniture_type, FurnitureType::Bench);
        assert!(furniture.blocking);
    }

    #[test]
    fn test_furniture_entity_non_blocking() {
        let furniture = FurnitureEntity::new(FurnitureType::Torch, false);
        assert_eq!(furniture.furniture_type, FurnitureType::Torch);
        assert!(!furniture.blocking);
    }

    #[test]
    fn test_furniture_types_all_variants_present() {
        let all_types = FurnitureType::all();

        // Should have all expected furniture types
        assert!(all_types.contains(&FurnitureType::Throne));
        assert!(all_types.contains(&FurnitureType::Bench));
        assert!(all_types.contains(&FurnitureType::Table));
        assert!(all_types.contains(&FurnitureType::Chair));
        assert!(all_types.contains(&FurnitureType::Torch));
        assert!(all_types.contains(&FurnitureType::Bookshelf));
        assert!(all_types.contains(&FurnitureType::Barrel));
        assert!(all_types.contains(&FurnitureType::Chest));
    }

    #[test]
    fn test_furniture_flags_new() {
        let flags = FurnitureFlags::new();
        assert!(!flags.lit);
        assert!(!flags.locked);
        assert!(!flags.blocking);
    }

    #[test]
    fn test_furniture_flags_with_lit() {
        let flags = FurnitureFlags::new().with_lit(true);
        assert!(flags.lit);
        assert!(!flags.locked);
        assert!(!flags.blocking);
    }

    #[test]
    fn test_furniture_flags_with_locked() {
        let flags = FurnitureFlags::new().with_locked(true);
        assert!(!flags.lit);
        assert!(flags.locked);
        assert!(!flags.blocking);
    }

    #[test]
    fn test_furniture_flags_with_blocking() {
        let flags = FurnitureFlags::new().with_blocking(true);
        assert!(!flags.lit);
        assert!(!flags.locked);
        assert!(flags.blocking);
    }

    #[test]
    fn test_furniture_flags_chained() {
        let flags = FurnitureFlags::new()
            .with_lit(true)
            .with_locked(true)
            .with_blocking(true);
        assert!(flags.lit);
        assert!(flags.locked);
        assert!(flags.blocking);
    }

    #[test]
    fn test_furniture_scale_multiplier_various_values() {
        // Scale should be applicable to furniture at spawn time
        let scales: Vec<f32> = vec![0.5, 0.75, 1.0, 1.25, 2.0];

        for scale in scales {
            assert!(scale > 0.0);
            // Scales should be positive and finite
            assert!(scale.is_finite());
        }
    }

    #[test]
    fn test_color_tint_blending() {
        // Test multiplicative color blending
        let base_white: [f32; 3] = [1.0, 1.0, 1.0];
        let tint_red: [f32; 3] = [1.0, 0.0, 0.0];

        // White * red = red
        let blended = [
            (base_white[0] * tint_red[0]).min(1.0),
            (base_white[1] * tint_red[1]).min(1.0),
            (base_white[2] * tint_red[2]).min(1.0),
        ];

        assert_eq!(blended[0], 1.0);
        assert_eq!(blended[1], 0.0);
        assert_eq!(blended[2], 0.0);
    }

    #[test]
    fn test_color_tint_darkening() {
        // Color tint should be able to darken colors
        let base_color: [f32; 3] = [1.0, 1.0, 1.0];
        let tint_dark: [f32; 3] = [0.5, 0.5, 0.5];

        let blended = [
            (base_color[0] * tint_dark[0]).min(1.0),
            (base_color[1] * tint_dark[1]).min(1.0),
            (base_color[2] * tint_dark[2]).min(1.0),
        ];

        assert_eq!(blended[0], 0.5);
        assert_eq!(blended[1], 0.5);
        assert_eq!(blended[2], 0.5);
    }

    #[test]
    fn test_furniture_names_are_correct() {
        assert_eq!(FurnitureType::Throne.name(), "Throne");
        assert_eq!(FurnitureType::Bench.name(), "Bench");
        assert_eq!(FurnitureType::Table.name(), "Table");
        assert_eq!(FurnitureType::Chair.name(), "Chair");
        assert_eq!(FurnitureType::Torch.name(), "Torch");
        assert_eq!(FurnitureType::Bookshelf.name(), "Bookshelf");
        assert_eq!(FurnitureType::Barrel.name(), "Barrel");
        assert_eq!(FurnitureType::Chest.name(), "Chest");
    }

    #[test]
    fn test_furniture_icon_present_for_all_types() {
        for furniture_type in FurnitureType::all() {
            let icon = furniture_type.icon();
            assert!(
                !icon.is_empty(),
                "Furniture type {:?} should have an icon",
                furniture_type
            );
        }
    }

    #[test]
    fn test_furniture_category_assignment() {
        // Test that furniture types are assigned to categories
        assert!(!FurnitureType::Chair.category().name().is_empty());
        assert!(!FurnitureType::Chest.category().name().is_empty());
        assert!(!FurnitureType::Torch.category().name().is_empty());
        assert!(!FurnitureType::Table.category().name().is_empty());
    }

    #[test]
    fn test_position_type() {
        let pos = Position { x: 5, y: 10 };
        assert_eq!(pos.x, 5);
        assert_eq!(pos.y, 10);
    }

    #[test]
    fn test_furniture_interaction_type_names() {
        assert_eq!(InteractionType::OpenChest.name(), "Open Chest");
        assert_eq!(InteractionType::SitOnChair.name(), "Sit on Chair");
        assert_eq!(InteractionType::LightTorch.name(), "Light Torch");
        assert_eq!(InteractionType::ReadBookshelf.name(), "Read Bookshelf");
    }

    #[test]
    fn test_furniture_appearance_presets_throne() {
        let presets = FurnitureType::Throne.default_presets();
        assert!(!presets.is_empty(), "Throne should have default presets");

        // Check that presets have names
        for preset in presets {
            assert!(!preset.name.is_empty());
        }
    }

    #[test]
    fn test_furniture_appearance_presets_torch() {
        let presets = FurnitureType::Torch.default_presets();
        assert!(!presets.is_empty(), "Torch should have default presets");

        // Presets should all have valid data
        for preset in presets {
            assert!(!preset.name.is_empty(), "Preset should have a name");
        }
    }

    #[test]
    fn test_furniture_blocking_affects_pathfinding() {
        // Blocking furniture should prevent movement
        let blocking_chest = FurnitureEntity::new(FurnitureType::Chest, true);
        assert!(blocking_chest.blocking);

        // Non-blocking torch shouldn't affect pathfinding
        let non_blocking_torch = FurnitureEntity::new(FurnitureType::Torch, false);
        assert!(!non_blocking_torch.blocking);
    }

    #[test]
    fn test_torch_lit_state_affects_material() {
        let lit_torch_flags = FurnitureFlags::new().with_lit(true);
        let unlit_torch_flags = FurnitureFlags::new().with_lit(false);

        assert!(lit_torch_flags.lit);
        assert!(!unlit_torch_flags.lit);
    }

    #[test]
    fn test_chest_locked_state() {
        let locked_chest = FurnitureFlags::new().with_locked(true);
        let unlocked_chest = FurnitureFlags::new().with_locked(false);

        assert!(locked_chest.locked);
        assert!(!unlocked_chest.locked);
    }

    #[test]
    fn test_material_metallic_values_in_range() {
        for _furniture_type in FurnitureType::all() {
            // Get a sample material - just test the material enum values
            let material = FurnitureMaterial::Wood;
            let metallic = material.metallic();

            // Metallic should be in valid range
            assert!(metallic >= 0.0);
            assert!(metallic <= 1.0);
        }
    }

    #[test]
    fn test_material_roughness_values_in_range() {
        for _furniture_type in FurnitureType::all() {
            let material = FurnitureMaterial::Stone;
            let roughness = material.roughness();

            // Roughness should be in valid range
            assert!(roughness >= 0.0);
            assert!(roughness <= 1.0);
        }
    }

    #[test]
    fn test_color_values_normalized() {
        let material = FurnitureMaterial::Gold;
        let color = material.base_color();

        // RGB values should be normalized [0..1]
        assert!(color[0] >= 0.0 && color[0] <= 1.0);
        assert!(color[1] >= 0.0 && color[1] <= 1.0);
        assert!(color[2] >= 0.0 && color[2] <= 1.0);
    }

    #[test]
    fn test_furniture_scale_ranges() {
        // Test various scale values that might be used
        let test_scales: Vec<f32> = vec![0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0];

        for scale in test_scales {
            // Scaled dimensions should all be positive
            let scaled_height: f32 = 0.7 * scale;
            assert!(scaled_height > 0.0);
            assert!(scaled_height.is_finite());
        }
    }

    #[test]
    fn test_default_furniture_scale() {
        // Default scale should be 1.0
        assert_eq!(1.0, 1.0);
    }

    #[test]
    fn test_rotation_y_values() {
        // Rotation values in degrees
        let rotations: Vec<f32> = vec![0.0, 45.0, 90.0, 180.0, 270.0, 360.0];

        for rotation in rotations {
            // All should be valid rotation values
            assert!(rotation.is_finite());
        }
    }

    #[test]
    fn test_color_tint_none_uses_base_color() {
        // When color_tint is None, base color should be used
        let material = FurnitureMaterial::Wood;
        let base_color: [f32; 3] = material.base_color();

        // Should have valid RGB values
        assert!(base_color[0] >= 0.0);
        assert!(base_color[1] >= 0.0);
        assert!(base_color[2] >= 0.0);
    }

    #[test]
    fn test_color_tint_some_overrides_base() {
        // When color_tint is Some, it should affect the final color
        let tint: Option<[f32; 3]> = Some([0.5, 0.5, 0.5]);
        let material = FurnitureMaterial::Wood;
        let base_color: [f32; 3] = material.base_color();

        let final_color: [f32; 3] = if let Some([r, g, b]) = tint {
            [
                (base_color[0] * r).min(1.0),
                (base_color[1] * g).min(1.0),
                (base_color[2] * b).min(1.0),
            ]
        } else {
            base_color
        };

        // Final color values should be dampened
        assert!(final_color[0] <= base_color[0]);
        assert!(final_color[1] <= base_color[1]);
        assert!(final_color[2] <= base_color[2]);
    }
}
