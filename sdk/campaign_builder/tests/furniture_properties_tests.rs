// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Tests for Phase 8: Furniture Properties Extension, Categorization, and Editor UI
//!
//! This module contains comprehensive tests for:
//! - FurnitureMaterial enum variants and properties
//! - FurnitureFlags struct and flag behavior
//! - FurnitureCategory enum and categorization system
//! - Enhanced property editor UI controls
//! - Furniture palette with filtering
//! - Serialization/deserialization of new properties

#[cfg(test)]
mod furniture_properties_tests {
    use antares::domain::types::Position;
    use antares::domain::world::{
        FurnitureCategory, FurnitureFlags, FurnitureMaterial, FurnitureType, MapEvent,
    };
    use campaign_builder::map_editor::{EventEditorState, EventType};

    // ===== FurnitureMaterial Tests =====

    /// Test FurnitureMaterial enum variants
    #[test]
    fn test_furniture_material_enum_variants() {
        let materials = FurnitureMaterial::all();

        assert_eq!(materials.len(), 4, "Should have 4 material variants");

        // Verify all expected materials are present
        let material_names: Vec<&str> = materials.iter().map(|m| m.name()).collect();
        assert!(material_names.contains(&"Wood"));
        assert!(material_names.contains(&"Stone"));
        assert!(material_names.contains(&"Metal"));
        assert!(material_names.contains(&"Gold"));
    }

    /// Test FurnitureMaterial default is Wood
    #[test]
    fn test_furniture_material_default() {
        let default_material = FurnitureMaterial::default();
        assert_eq!(default_material, FurnitureMaterial::Wood);
        assert_eq!(default_material.name(), "Wood");
    }

    /// Test each FurnitureMaterial has a name
    #[test]
    fn test_furniture_material_names() {
        let test_cases = vec![
            (FurnitureMaterial::Wood, "Wood"),
            (FurnitureMaterial::Stone, "Stone"),
            (FurnitureMaterial::Metal, "Metal"),
            (FurnitureMaterial::Gold, "Gold"),
        ];

        for (material, expected_name) in test_cases {
            assert_eq!(material.name(), expected_name);
        }
    }

    // ===== FurnitureFlags Tests =====

    /// Test FurnitureFlags default is all false
    #[test]
    fn test_furniture_flags_default() {
        let flags = FurnitureFlags::default();

        assert!(!flags.lit);
        assert!(!flags.locked);
        assert!(!flags.blocking);
    }

    /// Test FurnitureFlags::new() creates empty flags
    #[test]
    fn test_furniture_flags_new() {
        let flags = FurnitureFlags::new();

        assert!(!flags.lit);
        assert!(!flags.locked);
        assert!(!flags.blocking);
    }

    /// Test FurnitureFlags builder methods
    #[test]
    fn test_furniture_flags_builder_methods() {
        let flags = FurnitureFlags::new()
            .with_lit(true)
            .with_locked(true)
            .with_blocking(true);

        assert!(flags.lit);
        assert!(flags.locked);
        assert!(flags.blocking);
    }

    /// Test FurnitureFlags can be individually set
    #[test]
    fn test_furniture_flags_individual_settings() {
        let mut flags = FurnitureFlags {
            lit: true,
            ..Default::default()
        };

        assert!(flags.lit);
        assert!(!flags.locked);
        assert!(!flags.blocking);

        flags.locked = true;
        assert!(flags.lit);
        assert!(flags.locked);
        assert!(!flags.blocking);

        flags.blocking = true;
        assert!(flags.lit);
        assert!(flags.locked);
        assert!(flags.blocking);
    }

    // ===== FurnitureCategory Tests =====

    /// Test FurnitureCategory enum variants
    #[test]
    fn test_furniture_category_enum_variants() {
        let categories = FurnitureCategory::all();

        assert_eq!(categories.len(), 5, "Should have 5 category variants");

        let category_names: Vec<&str> = categories.iter().map(|c| c.name()).collect();
        assert!(category_names.contains(&"Seating"));
        assert!(category_names.contains(&"Storage"));
        assert!(category_names.contains(&"Decoration"));
        assert!(category_names.contains(&"Lighting"));
        assert!(category_names.contains(&"Utility"));
    }

    /// Test category names
    #[test]
    fn test_furniture_category_names() {
        let test_cases = vec![
            (FurnitureCategory::Seating, "Seating"),
            (FurnitureCategory::Storage, "Storage"),
            (FurnitureCategory::Decoration, "Decoration"),
            (FurnitureCategory::Lighting, "Lighting"),
            (FurnitureCategory::Utility, "Utility"),
        ];

        for (category, expected_name) in test_cases {
            assert_eq!(category.name(), expected_name);
        }
    }

    // ===== FurnitureType Categorization Tests =====

    /// Test FurnitureType returns correct category
    #[test]
    fn test_furniture_type_category_assignment() {
        // Seating
        assert_eq!(FurnitureType::Throne.category(), FurnitureCategory::Seating);
        assert_eq!(FurnitureType::Bench.category(), FurnitureCategory::Seating);
        assert_eq!(FurnitureType::Chair.category(), FurnitureCategory::Seating);

        // Storage
        assert_eq!(FurnitureType::Chest.category(), FurnitureCategory::Storage);
        assert_eq!(FurnitureType::Barrel.category(), FurnitureCategory::Storage);
        assert_eq!(
            FurnitureType::Bookshelf.category(),
            FurnitureCategory::Storage
        );

        // Lighting
        assert_eq!(FurnitureType::Torch.category(), FurnitureCategory::Lighting);

        // Utility
        assert_eq!(FurnitureType::Table.category(), FurnitureCategory::Utility);
    }

    /// Test all furniture types are categorized
    #[test]
    fn test_all_furniture_types_categorized() {
        for furniture_type in FurnitureType::all() {
            let category = furniture_type.category();
            // Just verify it doesn't panic and returns a valid category
            let _name = category.name();
            assert!(!_name.is_empty());
        }
    }

    /// Test furniture palette filtering by category
    #[test]
    fn test_furniture_palette_filtering() {
        let seating = vec![
            FurnitureType::Throne,
            FurnitureType::Bench,
            FurnitureType::Chair,
        ];
        let storage = vec![
            FurnitureType::Chest,
            FurnitureType::Barrel,
            FurnitureType::Bookshelf,
        ];
        let lighting = vec![FurnitureType::Torch];
        let utility = vec![FurnitureType::Table];

        // Verify categorization is consistent
        for furniture_type in &seating {
            assert_eq!(furniture_type.category(), FurnitureCategory::Seating);
        }
        for furniture_type in &storage {
            assert_eq!(furniture_type.category(), FurnitureCategory::Storage);
        }
        for furniture_type in &lighting {
            assert_eq!(furniture_type.category(), FurnitureCategory::Lighting);
        }
        for furniture_type in &utility {
            assert_eq!(furniture_type.category(), FurnitureCategory::Utility);
        }
    }

    // ===== Furniture Properties Editor Tests =====

    /// Test scale property in EventEditorState
    #[test]
    fn test_furniture_scale_property() {
        let mut editor = EventEditorState::default();

        // Default scale
        assert_eq!(editor.furniture_scale, 1.0);

        // Set to valid ranges
        editor.furniture_scale = 0.5;
        assert_eq!(editor.furniture_scale, 0.5);

        editor.furniture_scale = 2.0;
        assert_eq!(editor.furniture_scale, 2.0);

        editor.furniture_scale = 1.5;
        assert_eq!(editor.furniture_scale, 1.5);
    }

    /// Test scale range (0.5-2.0)
    #[test]
    fn test_furniture_scale_range() {
        let mut editor = EventEditorState {
            event_type: EventType::Furniture,
            ..Default::default()
        };

        // Minimum scale
        editor.furniture_scale = 0.5;
        let event = editor.to_map_event().unwrap();
        match event {
            MapEvent::Furniture { scale, .. } => {
                assert_eq!(scale, 0.5);
            }
            _ => panic!("Expected MapEvent::Furniture"),
        }

        // Maximum scale
        editor.furniture_scale = 2.0;
        editor.event_type = EventType::Furniture;
        let event = editor.to_map_event().unwrap();
        match event {
            MapEvent::Furniture { scale, .. } => {
                assert_eq!(scale, 2.0);
            }
            _ => panic!("Expected MapEvent::Furniture"),
        }
    }

    /// Test material property in EventEditorState
    #[test]
    fn test_furniture_material_property_editor() {
        let mut editor = EventEditorState::default();

        // Default material
        assert_eq!(editor.furniture_material, FurnitureMaterial::Wood);

        // Set to different materials
        editor.furniture_material = FurnitureMaterial::Stone;
        assert_eq!(editor.furniture_material, FurnitureMaterial::Stone);

        editor.furniture_material = FurnitureMaterial::Metal;
        assert_eq!(editor.furniture_material, FurnitureMaterial::Metal);

        editor.furniture_material = FurnitureMaterial::Gold;
        assert_eq!(editor.furniture_material, FurnitureMaterial::Gold);
    }

    /// Test lit flag for torches
    #[test]
    fn test_torch_lit_flag() {
        let mut editor = EventEditorState {
            event_type: EventType::Furniture,
            furniture_type: FurnitureType::Torch,
            name: "Test Torch".to_string(),
            ..Default::default()
        };

        // Default: not lit
        assert!(!editor.furniture_lit);

        // Set lit
        editor.furniture_lit = true;
        let event = editor.to_map_event().unwrap();

        match event {
            MapEvent::Furniture { flags, .. } => {
                assert!(flags.lit);
            }
            _ => panic!("Expected MapEvent::Furniture"),
        }
    }

    /// Test lit flag serialization for torch
    #[test]
    fn test_torch_lit_flag_serialization() {
        let event = MapEvent::Furniture {
            name: "Lit Torch".to_string(),
            furniture_type: FurnitureType::Torch,
            rotation_y: Some(0.0),
            scale: 1.0,
            material: FurnitureMaterial::Metal,
            flags: FurnitureFlags {
                lit: true,
                locked: false,
                blocking: false,
            },
            color_tint: None,
        };

        // Verify serialization through editor state
        let editor = EventEditorState::from_map_event(Position::new(0, 0), &event);
        assert!(editor.furniture_lit);

        let recovered = editor.to_map_event().unwrap();
        match recovered {
            MapEvent::Furniture { flags, .. } => {
                assert!(flags.lit);
            }
            _ => panic!("Expected MapEvent::Furniture"),
        }
    }

    /// Test locked flag for chests
    #[test]
    fn test_chest_locked_flag() {
        let mut editor = EventEditorState {
            event_type: EventType::Furniture,
            furniture_type: FurnitureType::Chest,
            ..Default::default()
        };

        // Default: not locked
        assert!(!editor.furniture_locked);

        // Set locked
        editor.furniture_locked = true;
        let event = editor.to_map_event().unwrap();

        match event {
            MapEvent::Furniture { flags, .. } => {
                assert!(flags.locked);
            }
            _ => panic!("Expected MapEvent::Furniture"),
        }
    }

    /// Test locked flag serialization for chest
    #[test]
    fn test_chest_locked_flag_serialization() {
        let event = MapEvent::Furniture {
            name: "Locked Chest".to_string(),
            furniture_type: FurnitureType::Chest,
            rotation_y: Some(0.0),
            scale: 1.0,
            material: FurnitureMaterial::Wood,
            flags: FurnitureFlags {
                lit: false,
                locked: true,
                blocking: false,
            },
            color_tint: None,
        };

        let editor = EventEditorState::from_map_event(Position::new(5, 5), &event);
        assert!(editor.furniture_locked);

        let recovered = editor.to_map_event().unwrap();
        match recovered {
            MapEvent::Furniture { flags, .. } => {
                assert!(flags.locked);
            }
            _ => panic!("Expected MapEvent::Furniture"),
        }
    }

    /// Test blocking flag applies to all furniture
    #[test]
    fn test_furniture_blocking_flag() {
        for furniture_type in FurnitureType::all() {
            let mut editor = EventEditorState {
                event_type: EventType::Furniture,
                furniture_type: *furniture_type,
                ..Default::default()
            };

            // Default: not blocking
            assert!(!editor.furniture_blocking);

            // Set blocking
            editor.furniture_blocking = true;
            let event = editor.to_map_event().unwrap();

            match event {
                MapEvent::Furniture { flags, .. } => {
                    assert!(flags.blocking);
                }
                _ => panic!("Expected MapEvent::Furniture"),
            }
        }
    }

    // ===== Round-trip Tests =====

    /// Test furniture properties round-trip serialization
    #[test]
    fn test_furniture_properties_roundtrip() {
        let original_event = MapEvent::Furniture {
            name: "Test Throne".to_string(),
            furniture_type: FurnitureType::Throne,
            rotation_y: Some(45.0),
            scale: 1.5,
            material: FurnitureMaterial::Gold,
            flags: FurnitureFlags {
                lit: false,
                locked: false,
                blocking: true,
            },
            color_tint: None,
        };

        // Convert to editor state
        let editor = EventEditorState::from_map_event(Position::new(10, 10), &original_event);

        // Verify editor has all properties
        assert_eq!(editor.furniture_scale, 1.5);
        assert_eq!(editor.furniture_material, FurnitureMaterial::Gold);
        assert!(!editor.furniture_lit);
        assert!(!editor.furniture_locked);
        assert!(editor.furniture_blocking);

        // Convert back to event
        let recovered = editor.to_map_event().unwrap();

        // Verify recovered event matches original
        match recovered {
            MapEvent::Furniture {
                name,
                furniture_type,
                rotation_y,
                scale,
                material,
                flags,
                ..
            } => {
                assert_eq!(name, "Test Throne");
                assert_eq!(furniture_type, FurnitureType::Throne);
                assert_eq!(rotation_y, Some(45.0));
                assert_eq!(scale, 1.5);
                assert_eq!(material, FurnitureMaterial::Gold);
                assert!(!flags.lit);
                assert!(!flags.locked);
                assert!(flags.blocking);
            }
            _ => panic!("Expected MapEvent::Furniture"),
        }
    }

    /// Test complex furniture property combinations
    #[test]
    fn test_complex_furniture_configurations() {
        let configurations = vec![
            (
                FurnitureType::Torch,
                FurnitureMaterial::Metal,
                1.0,
                true,
                false,
                false,
            ),
            (
                FurnitureType::Chest,
                FurnitureMaterial::Wood,
                0.8,
                false,
                true,
                true,
            ),
            (
                FurnitureType::Throne,
                FurnitureMaterial::Gold,
                1.2,
                false,
                false,
                true,
            ),
            (
                FurnitureType::Table,
                FurnitureMaterial::Stone,
                1.5,
                false,
                false,
                false,
            ),
        ];

        for (furniture_type, material, scale, lit, locked, blocking) in configurations {
            let editor = EventEditorState {
                event_type: EventType::Furniture,
                furniture_type,
                furniture_material: material,
                furniture_scale: scale,
                furniture_lit: lit,
                furniture_locked: locked,
                furniture_blocking: blocking,
                ..Default::default()
            };

            let event = editor.to_map_event().unwrap();

            match event {
                MapEvent::Furniture {
                    furniture_type: ft,
                    material: m,
                    scale: s,
                    flags,
                    ..
                } => {
                    assert_eq!(ft, furniture_type);
                    assert_eq!(m, material);
                    assert_eq!(s, scale);
                    assert_eq!(flags.lit, lit);
                    assert_eq!(flags.locked, locked);
                    assert_eq!(flags.blocking, blocking);
                }
                _ => panic!("Expected MapEvent::Furniture"),
            }
        }
    }
}
