// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Tests for Furniture Event Editor functionality
//!
//! This module contains comprehensive tests for Phase 7: Campaign Builder SDK -
//! Furniture & Props Event Editor, verifying:
//! - FurnitureType enum integration
//! - Furniture event serialization/deserialization
//! - Event editor state management for furniture events
//! - Rotation and other furniture-specific properties

#[cfg(test)]
mod furniture_editor_tests {
    use antares::domain::types::Position;
    use antares::domain::world::{FurnitureFlags, FurnitureMaterial, FurnitureType, MapEvent};
    use campaign_builder::map_editor::{EventEditorState, EventType};

    /// Test that all FurnitureType variants are present
    #[test]
    fn test_furniture_type_all_variants() {
        let all_types = FurnitureType::all();

        assert!(
            all_types.len() >= 8,
            "Should have at least 8 furniture types"
        );

        // Verify all expected types are present
        let type_names: Vec<&str> = all_types.iter().map(|t| t.name()).collect();
        assert!(type_names.contains(&"Throne"));
        assert!(type_names.contains(&"Bench"));
        assert!(type_names.contains(&"Table"));
        assert!(type_names.contains(&"Chair"));
        assert!(type_names.contains(&"Torch"));
        assert!(type_names.contains(&"Bookshelf"));
        assert!(type_names.contains(&"Barrel"));
        assert!(type_names.contains(&"Chest"));
    }

    /// Test that each FurnitureType has a unique icon and name
    #[test]
    fn test_furniture_type_icons() {
        let all_types = FurnitureType::all();

        for furniture_type in all_types {
            let name = furniture_type.name();
            let icon = furniture_type.icon();

            // Each should have non-empty name and icon
            assert!(
                !name.is_empty(),
                "Furniture type should have non-empty name"
            );
            assert!(
                !icon.is_empty(),
                "Furniture type should have non-empty icon"
            );

            // Icons should be emoji or unicode (just check they're not ASCII letters)
            assert!(
                !icon.chars().all(|c| c.is_ascii_alphanumeric()),
                "Icon for {} should be emoji/unicode, not ASCII",
                name
            );
        }
    }

    /// Test FurnitureType can be used in EventType enum
    #[test]
    fn test_furniture_event_type_variant() {
        let event_type = EventType::Furniture;

        assert_eq!(event_type.name(), "Furniture");
        assert_eq!(event_type.icon(), "🪑");

        // Verify it's in the all() list
        assert!(
            EventType::all().contains(&event_type),
            "Furniture should be in EventType::all()"
        );
    }

    /// Test furniture event serialization (to_map_event)
    #[test]
    fn test_furniture_event_serialization() {
        let editor = EventEditorState {
            event_type: EventType::Furniture,
            name: "Ornate Throne".to_string(),
            description: "A golden throne".to_string(),
            furniture_type: FurnitureType::Throne,
            furniture_rotation_y: "45".to_string(),
            ..Default::default()
        };

        let result = editor.to_map_event();

        assert!(result.is_ok());
        match result.unwrap() {
            MapEvent::Furniture {
                name,
                furniture_type,
                rotation_y,
                ..
            } => {
                assert_eq!(name, "Ornate Throne");
                assert_eq!(furniture_type, FurnitureType::Throne);
                assert_eq!(rotation_y, Some(45.0));
            }
            _ => panic!("Expected MapEvent::Furniture"),
        }
    }

    /// Test furniture event deserialization (from_map_event)
    #[test]
    fn test_furniture_event_deserialization() {
        let event = MapEvent::Furniture {
            name: "Simple Bench".to_string(),
            furniture_type: FurnitureType::Bench,
            rotation_y: Some(90.0),
            scale: 1.0,
            material: FurnitureMaterial::Wood,
            flags: FurnitureFlags::new(),
            color_tint: None,
        };

        let position = Position::new(5, 7);
        let editor = EventEditorState::from_map_event(position, &event);

        assert_eq!(editor.event_type, EventType::Furniture);
        assert_eq!(editor.name, "Simple Bench");
        assert_eq!(editor.furniture_type, FurnitureType::Bench);
        assert_eq!(editor.furniture_rotation_y, "90");
        assert_eq!(editor.position, position);
    }

    /// Test furniture event with no rotation
    #[test]
    fn test_furniture_event_without_rotation() {
        let event = MapEvent::Furniture {
            name: "Simple Chair".to_string(),
            furniture_type: FurnitureType::Chair,
            rotation_y: None,
            scale: 1.0,
            material: FurnitureMaterial::Wood,
            flags: FurnitureFlags::new(),
            color_tint: None,
        };

        let editor = EventEditorState::from_map_event(Position::new(0, 0), &event);

        assert_eq!(editor.event_type, EventType::Furniture);
        assert_eq!(editor.furniture_type, FurnitureType::Chair);
        assert_eq!(editor.furniture_rotation_y, "");
    }

    /// Test rotation range validation (0-360 degrees)
    #[test]
    fn test_furniture_rotation_range() {
        // Test minimum rotation (0 degrees)
        let mut editor = EventEditorState {
            event_type: EventType::Furniture,
            furniture_rotation_y: "0".to_string(),
            ..Default::default()
        };

        let result = editor.to_map_event();
        assert!(result.is_ok());

        // Test maximum rotation (360 degrees)
        editor.furniture_rotation_y = "360".to_string();
        let result = editor.to_map_event();
        assert!(result.is_ok());

        // Test intermediate rotation
        editor.furniture_rotation_y = "180.5".to_string();
        let result = editor.to_map_event();
        assert!(result.is_ok());
    }

    /// Test invalid rotation handling
    #[test]
    fn test_furniture_invalid_rotation() {
        let editor = EventEditorState {
            event_type: EventType::Furniture,
            furniture_rotation_y: "invalid".to_string(),
            ..Default::default()
        };

        let result = editor.to_map_event();

        // Invalid rotation should be treated as None
        assert!(result.is_ok());
        match result.unwrap() {
            MapEvent::Furniture { rotation_y, .. } => {
                assert_eq!(rotation_y, None);
            }
            _ => panic!("Expected MapEvent::Furniture"),
        }
    }

    /// Test different furniture types
    #[test]
    fn test_furniture_type_variants() {
        let test_cases = vec![
            (FurnitureType::Throne, "Throne"),
            (FurnitureType::Bench, "Bench"),
            (FurnitureType::Table, "Table"),
            (FurnitureType::Chair, "Chair"),
            (FurnitureType::Torch, "Torch"),
            (FurnitureType::Bookshelf, "Bookshelf"),
            (FurnitureType::Barrel, "Barrel"),
            (FurnitureType::Chest, "Chest"),
        ];

        for (furniture_type, expected_name) in test_cases {
            assert_eq!(furniture_type.name(), expected_name);

            // Test roundtrip
            let editor = EventEditorState {
                furniture_type,
                ..Default::default()
            };
            assert_eq!(editor.furniture_type, furniture_type);
        }
    }

    /// Test furniture event at different map positions
    #[test]
    fn test_furniture_event_placement() {
        let positions = vec![
            Position::new(0, 0),
            Position::new(5, 5),
            Position::new(20, 15),
            Position::new(99, 99),
        ];

        for pos in positions {
            let event = MapEvent::Furniture {
                name: "Test Furniture".to_string(),
                furniture_type: FurnitureType::Throne,
                rotation_y: Some(45.0),
                scale: 1.0,
                material: FurnitureMaterial::Wood,
                flags: FurnitureFlags::new(),
                color_tint: None,
            };

            let editor = EventEditorState::from_map_event(pos, &event);
            assert_eq!(editor.position, pos);
            assert_eq!(editor.event_type, EventType::Furniture);
        }
    }

    /// Test furniture event editing workflow
    #[test]
    fn test_furniture_event_editing() {
        // Create initial event
        let initial_event = MapEvent::Furniture {
            name: "Old Bench".to_string(),
            furniture_type: FurnitureType::Bench,
            rotation_y: Some(0.0),
            scale: 1.0,
            material: FurnitureMaterial::Wood,
            flags: FurnitureFlags::new(),
            color_tint: None,
        };

        // Load into editor
        let mut editor = EventEditorState::from_map_event(Position::new(3, 4), &initial_event);

        // Verify initial state
        assert_eq!(editor.name, "Old Bench");
        assert_eq!(editor.furniture_type, FurnitureType::Bench);
        assert_eq!(editor.furniture_rotation_y, "0");

        // Edit the event
        editor.name = "New Bench".to_string();
        editor.furniture_type = FurnitureType::Chair;
        editor.furniture_rotation_y = "45".to_string();

        // Convert back to MapEvent
        let updated_event = editor.to_map_event().unwrap();

        match updated_event {
            MapEvent::Furniture {
                name,
                furniture_type,
                rotation_y,
                ..
            } => {
                assert_eq!(name, "New Bench");
                assert_eq!(furniture_type, FurnitureType::Chair);
                assert_eq!(rotation_y, Some(45.0));
            }
            _ => panic!("Expected MapEvent::Furniture"),
        }
    }

    /// Test empty name handling for furniture events
    #[test]
    fn test_furniture_event_empty_name() {
        let editor = EventEditorState {
            event_type: EventType::Furniture,
            name: String::new(),
            furniture_type: FurnitureType::Torch,
            ..Default::default()
        };

        // Empty name should still be allowed (defaults behavior)
        let result = editor.to_map_event();
        assert!(result.is_ok());
    }

    /// Test EventType::Furniture properties
    #[test]
    fn test_furniture_event_type_properties() {
        let furniture_event_type = EventType::Furniture;

        // Check name
        assert_eq!(furniture_event_type.name(), "Furniture");

        // Check icon
        assert_eq!(furniture_event_type.icon(), "🪑");

        // Check color (should have a valid color)
        let _color = furniture_event_type.color();
        // Just verify it doesn't panic

        // Verify it's in all()
        let all = EventType::all();
        assert!(all.contains(&furniture_event_type));
    }

    /// Test multiple furniture events with different configurations
    #[test]
    fn test_multiple_furniture_configurations() {
        let configs = vec![
            (FurnitureType::Throne, 45.0),
            (FurnitureType::Torch, 0.0),
            (FurnitureType::Chest, 180.0),
            (FurnitureType::Bookshelf, 90.0),
        ];

        for (ftype, rotation) in configs {
            let editor = EventEditorState {
                event_type: EventType::Furniture,
                furniture_type: ftype,
                furniture_rotation_y: rotation.to_string(),
                name: format!("Test {}", ftype.name()),
                ..Default::default()
            };

            let event = editor.to_map_event().unwrap();

            match event {
                MapEvent::Furniture {
                    furniture_type,
                    rotation_y,
                    ..
                } => {
                    assert_eq!(furniture_type, ftype);
                    assert_eq!(rotation_y, Some(rotation));
                }
                _ => panic!("Expected MapEvent::Furniture"),
            }
        }
    }
}
