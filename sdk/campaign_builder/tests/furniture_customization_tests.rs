// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Comprehensive tests for Phase 9: Furniture Customization & Material System
//!
//! This test module validates:
//! - Material visual properties (base_color, metallic, roughness)
//! - Color tint serialization and deserialization
//! - Furniture appearance presets
//! - Color picker UI integration
//! - Preset application logic
//! - Color range validation

use antares::domain::types::Position;
use antares::domain::world::{
    FurnitureAppearancePreset, FurnitureCategory, FurnitureFlags, FurnitureMaterial, FurnitureType,
    MapEvent,
};

// ===== Material Visual Properties Tests =====

#[test]
fn test_material_base_color_wood() {
    let color = FurnitureMaterial::Wood.base_color();
    assert_eq!(color, [0.6, 0.4, 0.2]); // Brown
                                        // Verify values are in valid range
    assert!(color.iter().all(|&c| c >= 0.0 && c <= 1.0));
}

#[test]
fn test_material_base_color_stone() {
    let color = FurnitureMaterial::Stone.base_color();
    assert_eq!(color, [0.5, 0.5, 0.5]); // Gray
    assert!(color.iter().all(|&c| c >= 0.0 && c <= 1.0));
}

#[test]
fn test_material_base_color_metal() {
    let color = FurnitureMaterial::Metal.base_color();
    assert_eq!(color, [0.7, 0.7, 0.8]); // Silver
    assert!(color.iter().all(|&c| c >= 0.0 && c <= 1.0));
}

#[test]
fn test_material_base_color_gold() {
    let color = FurnitureMaterial::Gold.base_color();
    assert_eq!(color, [1.0, 0.84, 0.0]); // Gold
    assert!(color.iter().all(|&c| c >= 0.0 && c <= 1.0));
}

#[test]
fn test_material_base_color_all_variants() {
    for material in FurnitureMaterial::all() {
        let color = material.base_color();
        // All colors should be in valid RGB range
        assert!(
            color.iter().all(|&c| c >= 0.0 && c <= 1.0),
            "Material {:?} has out-of-range color values",
            material
        );
        // All colors should have 3 components
        assert_eq!(color.len(), 3);
    }
}

#[test]
fn test_material_metallic_properties() {
    // Wood: non-metallic
    assert_eq!(FurnitureMaterial::Wood.metallic(), 0.0);
    // Stone: slightly metallic
    assert_eq!(FurnitureMaterial::Stone.metallic(), 0.1);
    // Metal: very metallic
    assert_eq!(FurnitureMaterial::Metal.metallic(), 0.9);
    // Gold: fully metallic
    assert_eq!(FurnitureMaterial::Gold.metallic(), 1.0);
}

#[test]
fn test_material_metallic_range_validity() {
    for material in FurnitureMaterial::all() {
        let metallic = material.metallic();
        assert!(
            metallic >= 0.0 && metallic <= 1.0,
            "Material {:?} has metallic value outside 0.0-1.0 range",
            material
        );
    }
}

#[test]
fn test_material_roughness_properties() {
    // Wood: rough
    assert_eq!(FurnitureMaterial::Wood.roughness(), 0.8);
    // Stone: very rough
    assert_eq!(FurnitureMaterial::Stone.roughness(), 0.9);
    // Metal: smooth
    assert_eq!(FurnitureMaterial::Metal.roughness(), 0.3);
    // Gold: very smooth/polished
    assert_eq!(FurnitureMaterial::Gold.roughness(), 0.2);
}

#[test]
fn test_material_roughness_range_validity() {
    for material in FurnitureMaterial::all() {
        let roughness = material.roughness();
        assert!(
            roughness >= 0.0 && roughness <= 1.0,
            "Material {:?} has roughness value outside 0.0-1.0 range",
            material
        );
    }
}

// ===== Color Tint Serialization Tests =====

#[test]
fn test_color_tint_none_serialization() {
    let event = MapEvent::Furniture {
        name: "Test Furniture".to_string(),
        furniture_type: FurnitureType::Bench,
        rotation_y: None,
        scale: 1.0,
        material: FurnitureMaterial::Wood,
        flags: FurnitureFlags::default(),
        color_tint: None,
    };

    // Serialize to RON
    let serialized = ron::to_string(&event).expect("Failed to serialize");
    // Deserialize back
    let deserialized: MapEvent = ron::from_str(&serialized).expect("Failed to deserialize");

    match deserialized {
        MapEvent::Furniture { color_tint, .. } => {
            assert_eq!(color_tint, None);
        }
        _ => panic!("Expected Furniture event"),
    }
}

#[test]
fn test_color_tint_some_serialization() {
    let expected_tint = [1.0, 0.6, 0.2];
    let event = MapEvent::Furniture {
        name: "Torch".to_string(),
        furniture_type: FurnitureType::Torch,
        rotation_y: None,
        scale: 1.0,
        material: FurnitureMaterial::Wood,
        flags: FurnitureFlags::default(),
        color_tint: Some(expected_tint),
    };

    let serialized = ron::to_string(&event).expect("Failed to serialize");
    let deserialized: MapEvent = ron::from_str(&serialized).expect("Failed to deserialize");

    match deserialized {
        MapEvent::Furniture { color_tint, .. } => {
            assert_eq!(color_tint, Some(expected_tint));
        }
        _ => panic!("Expected Furniture event"),
    }
}

#[test]
fn test_color_tint_roundtrip_zero_values() {
    let tint = [0.0, 0.0, 0.0]; // Black
    let event = MapEvent::Furniture {
        name: "Dark Furniture".to_string(),
        furniture_type: FurnitureType::Bench,
        rotation_y: None,
        scale: 1.0,
        material: FurnitureMaterial::Stone,
        flags: FurnitureFlags::default(),
        color_tint: Some(tint),
    };

    let serialized = ron::to_string(&event).expect("Failed to serialize");
    let deserialized: MapEvent = ron::from_str(&serialized).expect("Failed to deserialize");

    match deserialized {
        MapEvent::Furniture { color_tint, .. } => {
            assert_eq!(color_tint, Some(tint));
        }
        _ => panic!("Expected Furniture event"),
    }
}

#[test]
fn test_color_tint_roundtrip_max_values() {
    let tint = [1.0, 1.0, 1.0]; // White
    let event = MapEvent::Furniture {
        name: "Light Furniture".to_string(),
        furniture_type: FurnitureType::Torch,
        rotation_y: None,
        scale: 1.0,
        material: FurnitureMaterial::Gold,
        flags: FurnitureFlags::default(),
        color_tint: Some(tint),
    };

    let serialized = ron::to_string(&event).expect("Failed to serialize");
    let deserialized: MapEvent = ron::from_str(&serialized).expect("Failed to deserialize");

    match deserialized {
        MapEvent::Furniture { color_tint, .. } => {
            assert_eq!(color_tint, Some(tint));
        }
        _ => panic!("Expected Furniture event"),
    }
}

#[test]
fn test_color_tint_range_validation() {
    // Test that color values are expected to be in 0.0-1.0 range
    let valid_tints = vec![
        [0.0, 0.0, 0.0],
        [0.5, 0.5, 0.5],
        [1.0, 1.0, 1.0],
        [1.0, 0.6, 0.2], // Orange
        [0.6, 0.8, 1.0], // Blue
    ];

    for tint in valid_tints {
        let event = MapEvent::Furniture {
            name: "Test".to_string(),
            furniture_type: FurnitureType::Chair,
            rotation_y: None,
            scale: 1.0,
            material: FurnitureMaterial::Wood,
            flags: FurnitureFlags::default(),
            color_tint: Some(tint),
        };

        let serialized = ron::to_string(&event).expect("Failed to serialize");
        let deserialized: MapEvent = ron::from_str(&serialized).expect("Failed to deserialize");

        match deserialized {
            MapEvent::Furniture { color_tint, .. } => {
                assert_eq!(color_tint, Some(tint));
                // Verify each component is in valid range
                for &component in &tint {
                    assert!(component >= 0.0 && component <= 1.0);
                }
            }
            _ => panic!("Expected Furniture event"),
        }
    }
}

// ===== Furniture Appearance Preset Tests =====

#[test]
fn test_furniture_appearance_preset_struct() {
    let preset = FurnitureAppearancePreset {
        name: "Test Preset",
        material: FurnitureMaterial::Gold,
        scale: 1.5,
        color_tint: Some([1.0, 0.84, 0.0]),
    };

    assert_eq!(preset.name, "Test Preset");
    assert_eq!(preset.material, FurnitureMaterial::Gold);
    assert_eq!(preset.scale, 1.5);
    assert_eq!(preset.color_tint, Some([1.0, 0.84, 0.0]));
}

#[test]
fn test_appearance_presets_throne_count() {
    let presets = FurnitureType::Throne.default_presets();
    assert!(presets.len() >= 3, "Throne should have at least 3 presets");
}

#[test]
fn test_appearance_presets_throne_wooden() {
    let presets = FurnitureType::Throne.default_presets();
    let wooden = presets
        .iter()
        .find(|p| p.name == "Wooden Throne")
        .expect("Wooden Throne preset not found");

    assert_eq!(wooden.material, FurnitureMaterial::Wood);
    assert_eq!(wooden.scale, 1.2);
    assert_eq!(wooden.color_tint, None);
}

#[test]
fn test_appearance_presets_throne_stone() {
    let presets = FurnitureType::Throne.default_presets();
    let stone = presets
        .iter()
        .find(|p| p.name == "Stone Throne")
        .expect("Stone Throne preset not found");

    assert_eq!(stone.material, FurnitureMaterial::Stone);
    assert_eq!(stone.scale, 1.3);
    assert_eq!(stone.color_tint, None);
}

#[test]
fn test_appearance_presets_throne_golden() {
    let presets = FurnitureType::Throne.default_presets();
    let golden = presets
        .iter()
        .find(|p| p.name == "Golden Throne")
        .expect("Golden Throne preset not found");

    assert_eq!(golden.material, FurnitureMaterial::Gold);
    assert_eq!(golden.scale, 1.5);
    assert_eq!(golden.color_tint, None);
}

#[test]
fn test_appearance_presets_torch_count() {
    let presets = FurnitureType::Torch.default_presets();
    assert!(presets.len() >= 2, "Torch should have at least 2 presets");
}

#[test]
fn test_appearance_presets_torch_wooden() {
    let presets = FurnitureType::Torch.default_presets();
    let wooden = presets
        .iter()
        .find(|p| p.name == "Wooden Torch")
        .expect("Wooden Torch preset not found");

    assert_eq!(wooden.material, FurnitureMaterial::Wood);
    assert_eq!(wooden.scale, 1.0);
    assert_eq!(wooden.color_tint, Some([1.0, 0.6, 0.2])); // Orange flame
}

#[test]
fn test_appearance_presets_torch_metal_sconce() {
    let presets = FurnitureType::Torch.default_presets();
    let metal = presets
        .iter()
        .find(|p| p.name == "Metal Sconce")
        .expect("Metal Sconce preset not found");

    assert_eq!(metal.material, FurnitureMaterial::Metal);
    assert_eq!(metal.scale, 0.8);
    assert_eq!(metal.color_tint, Some([0.6, 0.8, 1.0])); // Blue flame
}

#[test]
fn test_appearance_presets_other_types_default() {
    let other_types = vec![
        FurnitureType::Bench,
        FurnitureType::Table,
        FurnitureType::Chair,
        FurnitureType::Bookshelf,
        FurnitureType::Barrel,
        FurnitureType::Chest,
    ];

    for furniture_type in other_types {
        let presets = furniture_type.default_presets();
        assert!(
            presets.len() >= 1,
            "{:?} should have at least 1 preset",
            furniture_type
        );

        // Default preset should be present
        let default_preset = presets.iter().find(|p| p.name == "Default");
        assert!(
            default_preset.is_some(),
            "{:?} should have a 'Default' preset",
            furniture_type
        );
    }
}

#[test]
fn test_preset_all_presets_have_valid_scales() {
    for furniture_type in FurnitureType::all() {
        let presets = furniture_type.default_presets();
        for preset in presets {
            assert!(
                preset.scale > 0.0 && preset.scale <= 2.0,
                "Preset '{}' has invalid scale {}",
                preset.name,
                preset.scale
            );
        }
    }
}

#[test]
fn test_preset_all_presets_have_valid_materials() {
    for furniture_type in FurnitureType::all() {
        let presets = furniture_type.default_presets();
        for preset in presets {
            // Just accessing material to ensure it's a valid variant
            let _name = preset.material.name();
            assert!(!_name.is_empty());
        }
    }
}

#[test]
fn test_preset_tint_values_in_valid_range() {
    for furniture_type in FurnitureType::all() {
        let presets = furniture_type.default_presets();
        for preset in presets {
            if let Some(tint) = preset.color_tint {
                for component in &tint {
                    assert!(
                        *component >= 0.0 && *component <= 1.0,
                        "Preset '{}' has out-of-range color component {}",
                        preset.name,
                        component
                    );
                }
            }
        }
    }
}

// ===== Furniture Properties Round-Trip Tests =====

#[test]
fn test_furniture_properties_roundtrip_basic() {
    let original = MapEvent::Furniture {
        name: "Test Furniture".to_string(),
        furniture_type: FurnitureType::Throne,
        rotation_y: Some(45.0),
        scale: 1.2,
        material: FurnitureMaterial::Gold,
        flags: FurnitureFlags {
            lit: false,
            locked: false,
            blocking: true,
        },
        color_tint: None,
    };

    let serialized = ron::to_string(&original).expect("Failed to serialize");
    let deserialized: MapEvent = ron::from_str(&serialized).expect("Failed to deserialize");

    match deserialized {
        MapEvent::Furniture {
            name,
            furniture_type,
            rotation_y,
            scale,
            material,
            flags,
            color_tint,
        } => {
            assert_eq!(name, "Test Furniture");
            assert_eq!(furniture_type, FurnitureType::Throne);
            assert_eq!(rotation_y, Some(45.0));
            assert_eq!(scale, 1.2);
            assert_eq!(material, FurnitureMaterial::Gold);
            assert_eq!(flags.lit, false);
            assert_eq!(flags.locked, false);
            assert_eq!(flags.blocking, true);
            assert_eq!(color_tint, None);
        }
        _ => panic!("Expected Furniture event"),
    }
}

#[test]
fn test_furniture_properties_roundtrip_with_color_tint() {
    let original = MapEvent::Furniture {
        name: "Colored Torch".to_string(),
        furniture_type: FurnitureType::Torch,
        rotation_y: Some(90.0),
        scale: 0.9,
        material: FurnitureMaterial::Metal,
        flags: FurnitureFlags {
            lit: true,
            locked: false,
            blocking: false,
        },
        color_tint: Some([0.8, 0.4, 0.1]),
    };

    let serialized = ron::to_string(&original).expect("Failed to serialize");
    let deserialized: MapEvent = ron::from_str(&serialized).expect("Failed to deserialize");

    match deserialized {
        MapEvent::Furniture {
            name,
            furniture_type,
            rotation_y,
            scale,
            material,
            flags,
            color_tint,
        } => {
            assert_eq!(name, "Colored Torch");
            assert_eq!(furniture_type, FurnitureType::Torch);
            assert_eq!(rotation_y, Some(90.0));
            assert_eq!(scale, 0.9);
            assert_eq!(material, FurnitureMaterial::Metal);
            assert_eq!(flags.lit, true);
            assert_eq!(flags.locked, false);
            assert_eq!(flags.blocking, false);
            assert_eq!(color_tint, Some([0.8, 0.4, 0.1]));
        }
        _ => panic!("Expected Furniture event"),
    }
}

#[test]
fn test_furniture_properties_roundtrip_all_flags() {
    let original = MapEvent::Furniture {
        name: "Locked Lit Blocking Chest".to_string(),
        furniture_type: FurnitureType::Chest,
        rotation_y: Some(180.0),
        scale: 1.1,
        material: FurnitureMaterial::Wood,
        flags: FurnitureFlags {
            lit: true,
            locked: true,
            blocking: true,
        },
        color_tint: Some([0.5, 0.5, 0.5]),
    };

    let serialized = ron::to_string(&original).expect("Failed to serialize");
    let deserialized: MapEvent = ron::from_str(&serialized).expect("Failed to deserialize");

    match deserialized {
        MapEvent::Furniture {
            flags, color_tint, ..
        } => {
            assert_eq!(flags.lit, true);
            assert_eq!(flags.locked, true);
            assert_eq!(flags.blocking, true);
            assert_eq!(color_tint, Some([0.5, 0.5, 0.5]));
        }
        _ => panic!("Expected Furniture event"),
    }
}

// ===== Category Assignment Tests =====

#[test]
fn test_furniture_category_assignment_seating() {
    assert_eq!(FurnitureType::Throne.category(), FurnitureCategory::Seating);
    assert_eq!(FurnitureType::Bench.category(), FurnitureCategory::Seating);
    assert_eq!(FurnitureType::Chair.category(), FurnitureCategory::Seating);
}

#[test]
fn test_furniture_category_assignment_storage() {
    assert_eq!(FurnitureType::Chest.category(), FurnitureCategory::Storage);
    assert_eq!(FurnitureType::Barrel.category(), FurnitureCategory::Storage);
    assert_eq!(
        FurnitureType::Bookshelf.category(),
        FurnitureCategory::Storage
    );
}

#[test]
fn test_furniture_category_assignment_lighting() {
    assert_eq!(FurnitureType::Torch.category(), FurnitureCategory::Lighting);
}

#[test]
fn test_furniture_category_assignment_utility() {
    assert_eq!(FurnitureType::Table.category(), FurnitureCategory::Utility);
}

#[test]
fn test_all_furniture_types_have_category() {
    for furniture_type in FurnitureType::all() {
        let category = furniture_type.category();
        // Just verify it returns something valid
        let _name = category.name();
        assert!(!_name.is_empty());
    }
}

// ===== Color Preview Validation Tests =====

#[test]
fn test_color_preview_conversion_black() {
    let color_values = [0.0, 0.0, 0.0];
    let r = (color_values[0] * 255.0) as u8;
    let g = (color_values[1] * 255.0) as u8;
    let b = (color_values[2] * 255.0) as u8;

    assert_eq!(r, 0);
    assert_eq!(g, 0);
    assert_eq!(b, 0);
}

#[test]
fn test_color_preview_conversion_white() {
    let color_values = [1.0, 1.0, 1.0];
    let r = (color_values[0] * 255.0) as u8;
    let g = (color_values[1] * 255.0) as u8;
    let b = (color_values[2] * 255.0) as u8;

    assert_eq!(r, 255);
    assert_eq!(g, 255);
    assert_eq!(b, 255);
}

#[test]
fn test_color_preview_conversion_orange_flame() {
    let color_values = [1.0, 0.6, 0.2];
    let r = (color_values[0] * 255.0) as u8;
    let g = (color_values[1] * 255.0) as u8;
    let b = (color_values[2] * 255.0) as u8;

    assert_eq!(r, 255);
    assert_eq!(g, 153); // 0.6 * 255
    assert_eq!(b, 51); // 0.2 * 255
}

#[test]
fn test_color_preview_conversion_blue_flame() {
    let color_values = [0.6, 0.8, 1.0];
    let r = (color_values[0] * 255.0) as u8;
    let g = (color_values[1] * 255.0) as u8;
    let b = (color_values[2] * 255.0) as u8;

    assert_eq!(r, 153); // 0.6 * 255
    assert_eq!(g, 204); // 0.8 * 255
    assert_eq!(b, 255);
}

// ===== Integration Tests =====

#[test]
fn test_furniture_event_with_all_phase9_features() {
    // Create a furniture event using Phase 9 features
    let event = MapEvent::Furniture {
        name: "Golden Throne with Custom Glow".to_string(),
        furniture_type: FurnitureType::Throne,
        rotation_y: Some(45.0),
        scale: 1.5,
        material: FurnitureMaterial::Gold,
        flags: FurnitureFlags {
            lit: false,
            locked: false,
            blocking: true,
        },
        color_tint: Some([1.0, 0.9, 0.5]), // Warm golden glow
    };

    // Verify all properties
    match event {
        MapEvent::Furniture {
            name,
            furniture_type,
            rotation_y,
            scale,
            material,
            flags,
            color_tint,
        } => {
            assert_eq!(name, "Golden Throne with Custom Glow");
            assert_eq!(furniture_type, FurnitureType::Throne);
            assert_eq!(rotation_y, Some(45.0));
            assert_eq!(scale, 1.5);
            assert_eq!(material, FurnitureMaterial::Gold);
            assert_eq!(flags.blocking, true);
            assert_eq!(color_tint, Some([1.0, 0.9, 0.5]));

            // Verify material properties are accessible
            let base_color = material.base_color();
            assert_eq!(base_color, [1.0, 0.84, 0.0]);
            assert_eq!(material.metallic(), 1.0);
            assert_eq!(material.roughness(), 0.2);
        }
        _ => panic!("Expected Furniture event"),
    }
}

#[test]
fn test_preset_application_creates_valid_event() {
    // Simulate applying a preset
    let preset = FurnitureType::Torch.default_presets()[0].clone();

    let event = MapEvent::Furniture {
        name: "Torch with Applied Preset".to_string(),
        furniture_type: FurnitureType::Torch,
        rotation_y: None,
        scale: preset.scale,
        material: preset.material,
        flags: FurnitureFlags::default(),
        color_tint: preset.color_tint,
    };

    let serialized = ron::to_string(&event).expect("Failed to serialize");
    let deserialized: MapEvent = ron::from_str(&serialized).expect("Failed to deserialize");

    match deserialized {
        MapEvent::Furniture {
            scale,
            material,
            color_tint,
            ..
        } => {
            assert_eq!(scale, preset.scale);
            assert_eq!(material, preset.material);
            assert_eq!(color_tint, preset.color_tint);
        }
        _ => panic!("Expected Furniture event"),
    }
}
