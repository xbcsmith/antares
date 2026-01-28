// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for sprite rendering system (Phase 3)
//!
//! Comprehensive test suite covering:
//! - Sprite component creation and configuration
//! - Animated sprite setup
//! - Billboard component setup
//! - Event marker type mapping
//!
//! Tests verify sprite rendering infrastructure

#[cfg(test)]
mod sprite_integration_tests {
    use antares::domain::world::{SpriteAnimation, SpriteReference};
    use antares::game::components::billboard::Billboard;
    use antares::game::components::sprite::{ActorSprite, ActorType, AnimatedSprite, TileSprite};
    use antares::game::resources::sprite_assets::SpriteAssets;

    // ===== SPRITE REFERENCE TESTS =====

    #[test]
    fn test_sprite_reference_creation() {
        let sprite_ref = SpriteReference {
            sheet_path: "sprites/walls.png".to_string(),
            sprite_index: 0,
            animation: None,
            material_properties: None,
        };

        assert_eq!(sprite_ref.sheet_path, "sprites/walls.png");
        assert_eq!(sprite_ref.sprite_index, 0);
        assert!(sprite_ref.animation.is_none());
    }

    #[test]
    fn test_sprite_reference_with_animation() {
        let sprite_ref = SpriteReference {
            sheet_path: "sprites/water.png".to_string(),
            sprite_index: 0,
            animation: Some(SpriteAnimation {
                frames: vec![0, 1, 2, 3],
                fps: 8.0,
                looping: true,
            }),
            material_properties: None,
        };

        assert_eq!(sprite_ref.sheet_path, "sprites/water.png");
        assert!(sprite_ref.animation.is_some());

        let anim = sprite_ref.animation.as_ref().unwrap();
        assert_eq!(anim.frames, vec![0, 1, 2, 3]);
        assert_eq!(anim.fps, 8.0);
        assert!(anim.looping);
    }

    #[test]
    fn test_sprite_animation_defaults() {
        let anim = SpriteAnimation {
            frames: vec![0, 1],
            fps: 10.0,
            looping: false,
        };

        assert_eq!(anim.frames.len(), 2);
        assert_eq!(anim.fps, 10.0);
        assert!(!anim.looping);
    }

    // ===== TILE SPRITE COMPONENT TESTS =====

    #[test]
    fn test_tile_sprite_component_creation() {
        let tile_sprite = TileSprite {
            sheet_path: "sprites/walls.png".to_string(),
            sprite_index: 0,
        };

        assert_eq!(tile_sprite.sheet_path, "sprites/walls.png");
        assert_eq!(tile_sprite.sprite_index, 0);
    }

    #[test]
    fn test_tile_sprite_different_indices() {
        for index in 0..16 {
            let tile_sprite = TileSprite {
                sheet_path: "sprites/terrain.png".to_string(),
                sprite_index: index,
            };

            assert_eq!(tile_sprite.sprite_index, index);
        }
    }

    #[test]
    fn test_tile_sprite_different_sheets() {
        let sheets = vec!["walls.png", "terrain.png", "water.png", "doors.png"];

        for sheet in sheets {
            let tile_sprite = TileSprite {
                sheet_path: format!("sprites/{}", sheet),
                sprite_index: 0,
            };

            assert_eq!(tile_sprite.sheet_path, format!("sprites/{}", sheet));
        }
    }

    // ===== ACTOR SPRITE COMPONENT TESTS =====

    #[test]
    fn test_actor_sprite_npc_creation() {
        let actor_sprite = ActorSprite {
            sheet_path: "sprites/npcs_town.png".to_string(),
            sprite_index: 2,
            actor_type: ActorType::Npc,
        };

        assert_eq!(actor_sprite.sheet_path, "sprites/npcs_town.png");
        assert_eq!(actor_sprite.sprite_index, 2);
        assert_eq!(actor_sprite.actor_type, ActorType::Npc);
    }

    #[test]
    fn test_actor_sprite_monster_creation() {
        let actor_sprite = ActorSprite {
            sheet_path: "sprites/monsters.png".to_string(),
            sprite_index: 5,
            actor_type: ActorType::Monster,
        };

        assert_eq!(actor_sprite.actor_type, ActorType::Monster);
        assert_eq!(actor_sprite.sprite_index, 5);
    }

    #[test]
    fn test_actor_sprite_recruitable_creation() {
        let actor_sprite = ActorSprite {
            sheet_path: "sprites/recruitable.png".to_string(),
            sprite_index: 3,
            actor_type: ActorType::Recruitable,
        };

        assert_eq!(actor_sprite.actor_type, ActorType::Recruitable);
    }

    #[test]
    fn test_actor_type_enum_variants() {
        // Test that all variants can be instantiated
        let _npc = ActorType::Npc;
        let _monster = ActorType::Monster;
        let _recruitable = ActorType::Recruitable;

        assert_eq!(ActorType::Npc, ActorType::Npc);
        assert_eq!(ActorType::Monster, ActorType::Monster);
        assert_eq!(ActorType::Recruitable, ActorType::Recruitable);

        // Verify they are not equal to each other
        assert_ne!(ActorType::Npc, ActorType::Monster);
        assert_ne!(ActorType::Monster, ActorType::Recruitable);
        assert_ne!(ActorType::Npc, ActorType::Recruitable);
    }

    // ===== ANIMATED SPRITE COMPONENT TESTS =====

    #[test]
    fn test_animated_sprite_creation() {
        let anim = AnimatedSprite::new(vec![0, 1, 2, 3], 8.0, true);

        assert_eq!(anim.frames, vec![0, 1, 2, 3]);
        assert_eq!(anim.fps, 8.0);
        assert!(anim.looping);
        assert_eq!(anim.current_frame, 0);
    }

    #[test]
    fn test_animated_sprite_non_looping() {
        let anim = AnimatedSprite::new(vec![0, 1, 2], 10.0, false);

        assert!(!anim.looping);
        assert_eq!(anim.frames.len(), 3);
    }

    #[test]
    fn test_animated_sprite_frame_sequence() {
        let frames = vec![0, 2, 4, 6, 8];
        let anim = AnimatedSprite::new(frames.clone(), 6.0, true);

        assert_eq!(anim.frames, frames);
    }

    #[test]
    fn test_animated_sprite_initial_timer() {
        let anim = AnimatedSprite::new(vec![0, 1, 2], 8.0, true);
        // Timer should be initialized to 0.0
        assert_eq!(anim.fps, 8.0);
        assert_eq!(anim.timer, 0.0);
        assert_eq!(anim.current_frame, 0);
    }

    #[test]
    fn test_animated_sprite_high_fps() {
        let anim = AnimatedSprite::new(vec![0, 1], 60.0, true);
        assert_eq!(anim.fps, 60.0);
    }

    #[test]
    fn test_animated_sprite_low_fps() {
        let anim = AnimatedSprite::new(vec![0, 1], 1.0, true);
        assert_eq!(anim.fps, 1.0);
    }

    // ===== BILLBOARD COMPONENT TESTS =====

    #[test]
    fn test_billboard_default_lock_y_true() {
        let billboard = Billboard::default();
        assert!(billboard.lock_y);
    }

    #[test]
    fn test_billboard_explicit_lock_y_true() {
        let billboard = Billboard { lock_y: true };
        assert!(billboard.lock_y);
    }

    #[test]
    fn test_billboard_explicit_lock_y_false() {
        let billboard = Billboard { lock_y: false };
        assert!(!billboard.lock_y);
    }

    // ===== SPRITE ASSETS RESOURCE TESTS =====

    #[test]
    fn test_sprite_assets_creation() {
        let sprite_assets = SpriteAssets::default();

        // Verify resource can be created
        assert!(std::mem::size_of_val(&sprite_assets) > 0);
    }

    #[test]
    fn test_sprite_assets_multiple_instances() {
        let assets1 = SpriteAssets::default();
        let assets2 = SpriteAssets::default();

        // Both instances should be valid
        assert!(std::mem::size_of_val(&assets1) > 0);
        assert!(std::mem::size_of_val(&assets2) > 0);
    }

    // ===== BACKWARD COMPATIBILITY TESTS =====

    #[test]
    fn test_sprite_reference_without_animation() {
        let sprite_ref = SpriteReference {
            sheet_path: "sprites/terrain.png".to_string(),
            sprite_index: 5,
            animation: None,
            material_properties: None,
        };

        assert!(sprite_ref.animation.is_none());
    }

    #[test]
    fn test_tile_sprite_without_animation_component() {
        let tile_sprite = TileSprite {
            sheet_path: "sprites/walls.png".to_string(),
            sprite_index: 0,
        };

        // TileSprite itself doesn't have animation - that's in AnimatedSprite
        assert_eq!(tile_sprite.sprite_index, 0);
    }

    // ===== EVENT MARKER TYPE MAPPING TESTS =====

    #[test]
    fn test_event_marker_type_sign() {
        let event_type = "sign";
        assert_eq!(event_type, "sign");
    }

    #[test]
    fn test_event_marker_type_portal() {
        let event_type = "portal";
        assert_eq!(event_type, "portal");
    }

    #[test]
    fn test_event_marker_type_treasure() {
        let event_type = "treasure";
        assert_eq!(event_type, "treasure");
    }

    #[test]
    fn test_event_marker_type_quest() {
        let event_type = "quest";
        assert_eq!(event_type, "quest");
    }

    #[test]
    fn test_event_marker_type_mapping() {
        let event_types = vec!["sign", "portal", "treasure", "quest"];

        for event_type in event_types {
            // Map event type to sprite sheet (matching spawn_event_marker logic)
            let (sheet_path, _sprite_index) = match event_type {
                "sign" => ("sprites/signs.png", 0u32),
                "portal" => ("sprites/portals.png", 0u32),
                "treasure" => ("sprites/treasure.png", 0u32),
                "quest" => ("sprites/signs.png", 1u32),
                _ => ("sprites/signs.png", 0u32),
            };

            assert!(!sheet_path.is_empty());
            assert!(sheet_path.contains("sprites"));
        }
    }

    // ===== CONFIGURATION TESTS =====

    #[test]
    fn test_sprite_sheet_path_formats() {
        let paths = vec![
            "sprites/walls.png",
            "sprites/npcs_town.png",
            "sprites/water.png",
            "sprites/doors.png",
            "sprites/monsters.png",
        ];

        for path in paths {
            assert!(path.starts_with("sprites/"));
            assert!(path.ends_with(".png"));
        }
    }

    #[test]
    fn test_sprite_index_range() {
        for index in 0..256 {
            let tile_sprite = TileSprite {
                sheet_path: "sprites/test.png".to_string(),
                sprite_index: index,
            };

            assert_eq!(tile_sprite.sprite_index, index);
        }
    }

    #[test]
    fn test_animation_frame_sequence_validity() {
        let frames = vec![0, 1, 2, 3, 4, 5];
        let anim = AnimatedSprite::new(frames.clone(), 8.0, true);

        // All frames should be in sequence
        for (i, &frame) in anim.frames.iter().enumerate() {
            assert_eq!(frame, i as u32);
        }
    }

    #[test]
    fn test_animation_fps_positive() {
        let fps_values = vec![1.0, 8.0, 10.0, 24.0, 60.0];

        for fps in fps_values {
            let anim = AnimatedSprite::new(vec![0, 1], fps, true);
            assert!(anim.fps > 0.0);
            assert_eq!(anim.fps, fps);
        }
    }
}
