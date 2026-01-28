// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Sprite components for tile and actor rendering
//!
//! Provides component markers for different sprite entity types:
//! - `TileSprite`: Decorative sprites for walls, floors, terrain
//! - `ActorSprite`: Character sprites (NPCs, Monsters, Recruitables)
//! - `AnimatedSprite`: Frame-based sprite animations
//!
//! # Examples
//!
//! ```
//! use antares::game::components::sprite::{ActorSprite, ActorType};
//!
//! fn spawn_npc() {
//!     let npc = ActorSprite {
//!         sheet_path: "sprites/npcs_town.png".to_string(),
//!         sprite_index: 2, // Innkeeper
//!         actor_type: ActorType::Npc,
//!     };
//! }
//! ```

use bevy::prelude::*;

/// Component for tile-based sprites (walls, floors, decorations)
///
/// # Fields
///
/// * `sheet_path` - Path to sprite sheet texture (relative to assets/)
/// * `sprite_index` - Index in texture atlas grid (0-indexed, row-major)
///
/// # Examples
///
/// ```
/// use antares::game::components::sprite::TileSprite;
///
/// let wall_sprite = TileSprite {
///     sheet_path: "sprites/walls.png".to_string(),
///     sprite_index: 5,
/// };
/// ```
#[derive(Component, Debug, Clone)]
pub struct TileSprite {
    /// Path to sprite sheet texture
    pub sheet_path: String,
    /// Index in texture atlas grid (0-indexed, row-major)
    pub sprite_index: u32,
}

/// Component for actor sprites (NPCs, Monsters, Recruitables)
///
/// # Fields
///
/// * `sheet_path` - Path to sprite sheet texture
/// * `sprite_index` - Index in texture atlas grid
/// * `actor_type` - Type of actor (used for systems filtering)
///
/// # Examples
///
/// ```
/// use antares::game::components::sprite::{ActorSprite, ActorType};
///
/// let npc = ActorSprite {
///     sheet_path: "sprites/npcs_town.png".to_string(),
///     sprite_index: 0,
///     actor_type: ActorType::Npc,
/// };
/// ```
#[derive(Component, Debug, Clone)]
pub struct ActorSprite {
    /// Path to sprite sheet texture
    pub sheet_path: String,
    /// Index in texture atlas grid (0-indexed, row-major)
    pub sprite_index: u32,
    /// Type of actor (NPC, Monster, Recruitable)
    pub actor_type: ActorType,
}

/// Type of actor entity
///
/// Used for filtering systems and gameplay logic.
///
/// # Variants
///
/// * `Npc` - Non-player character (dialogue, quests)
/// * `Monster` - Enemy entity (combat)
/// * `Recruitable` - Character available for recruitment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActorType {
    /// Non-player character (dialogue, quests)
    Npc,
    /// Enemy entity (combat)
    Monster,
    /// Character available for recruitment
    Recruitable,
}

/// Component for animated sprites
///
/// # Fields
///
/// * `frames` - Frame indices in animation sequence
/// * `fps` - Frames per second (animation speed)
/// * `looping` - Whether animation repeats
/// * `current_frame` - Current frame index (internal state)
/// * `timer` - Time accumulator for frame advancement
///
/// # Examples
///
/// ```
/// use antares::game::components::sprite::AnimatedSprite;
///
/// let water_anim = AnimatedSprite {
///     frames: vec![0, 1, 2, 3],
///     fps: 8.0,
///     looping: true,
///     current_frame: 0,
///     timer: 0.0,
/// };
/// ```
#[derive(Component, Debug, Clone)]
pub struct AnimatedSprite {
    /// Frame indices in animation sequence
    pub frames: Vec<u32>,

    /// Frames per second
    pub fps: f32,

    /// Whether animation loops
    pub looping: bool,

    /// Current frame index in `frames` vector
    pub current_frame: usize,

    /// Time accumulator for frame advancement (in seconds)
    pub timer: f32,
}

impl AnimatedSprite {
    /// Creates a new animated sprite
    ///
    /// # Arguments
    ///
    /// * `frames` - Frame indices in animation sequence
    /// * `fps` - Frames per second
    /// * `looping` - Whether animation loops
    ///
    /// # Returns
    ///
    /// New AnimatedSprite with timer at 0.0 and current_frame at 0
    pub fn new(frames: Vec<u32>, fps: f32, looping: bool) -> Self {
        Self {
            frames,
            fps,
            looping,
            current_frame: 0,
            timer: 0.0,
        }
    }

    /// Gets the frame per second duration
    ///
    /// # Returns
    ///
    /// Time in seconds for one frame (1.0 / fps)
    pub fn frame_duration(&self) -> f32 {
        1.0 / self.fps
    }

    /// Advances animation by delta time
    ///
    /// # Arguments
    ///
    /// * `delta` - Delta time in seconds
    ///
    /// # Returns
    ///
    /// true if animation finished (only if not looping)
    pub fn advance(&mut self, delta: f32) -> bool {
        if self.frames.is_empty() {
            return false;
        }

        self.timer += delta;
        let frame_duration = self.frame_duration();

        if self.timer >= frame_duration {
            self.timer -= frame_duration;
            self.current_frame += 1;

            if self.current_frame >= self.frames.len() {
                if self.looping {
                    self.current_frame = 0;
                    return false;
                } else {
                    self.current_frame = self.frames.len() - 1;
                    return true;
                }
            }
        }

        false
    }

    /// Gets the current frame index
    ///
    /// # Returns
    ///
    /// Frame index from the `frames` vector
    pub fn current_sprite_index(&self) -> u32 {
        if self.frames.is_empty() {
            0
        } else {
            self.frames[self.current_frame]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_sprite_creation() {
        let sprite = TileSprite {
            sheet_path: "sprites/walls.png".to_string(),
            sprite_index: 5,
        };
        assert_eq!(sprite.sheet_path, "sprites/walls.png");
        assert_eq!(sprite.sprite_index, 5);
    }

    #[test]
    fn test_actor_sprite_creation() {
        let sprite = ActorSprite {
            sheet_path: "sprites/npcs.png".to_string(),
            sprite_index: 2,
            actor_type: ActorType::Npc,
        };
        assert_eq!(sprite.sheet_path, "sprites/npcs.png");
        assert_eq!(sprite.sprite_index, 2);
        assert_eq!(sprite.actor_type, ActorType::Npc);
    }

    #[test]
    fn test_actor_type_variants() {
        assert_eq!(ActorType::Npc, ActorType::Npc);
        assert_ne!(ActorType::Npc, ActorType::Monster);
        assert_ne!(ActorType::Monster, ActorType::Recruitable);
    }

    #[test]
    fn test_animated_sprite_new() {
        let anim = AnimatedSprite::new(vec![0, 1, 2, 3], 8.0, true);
        assert_eq!(anim.frames, vec![0, 1, 2, 3]);
        assert_eq!(anim.fps, 8.0);
        assert!(anim.looping);
        assert_eq!(anim.current_frame, 0);
        assert_eq!(anim.timer, 0.0);
    }

    #[test]
    fn test_animated_sprite_frame_duration() {
        let anim = AnimatedSprite::new(vec![0, 1], 8.0, true);
        assert!((anim.frame_duration() - 0.125).abs() < 0.0001);
    }

    #[test]
    fn test_animated_sprite_advance_looping() {
        let mut anim = AnimatedSprite::new(vec![0, 1, 2], 2.0, true);
        assert_eq!(anim.current_frame, 0);

        // Advance to frame 1 (0.5 seconds)
        let finished = anim.advance(0.5);
        assert!(!finished);
        assert_eq!(anim.current_frame, 1);

        // Advance to frame 2
        let finished = anim.advance(0.5);
        assert!(!finished);
        assert_eq!(anim.current_frame, 2);

        // Advance past end - should loop
        let finished = anim.advance(0.5);
        assert!(!finished);
        assert_eq!(anim.current_frame, 0);
    }

    #[test]
    fn test_animated_sprite_advance_non_looping() {
        let mut anim = AnimatedSprite::new(vec![0, 1, 2], 2.0, false);

        // Advance through all frames
        anim.advance(0.5); // frame 1
        anim.advance(0.5); // frame 2
        let finished = anim.advance(0.5); // past end

        assert!(finished);
        assert_eq!(anim.current_frame, 2); // Stays on last frame
    }

    #[test]
    fn test_animated_sprite_current_sprite_index() {
        let mut anim = AnimatedSprite::new(vec![10, 20, 30], 2.0, true);
        assert_eq!(anim.current_sprite_index(), 10);

        anim.advance(0.5);
        assert_eq!(anim.current_sprite_index(), 20);

        anim.advance(0.5);
        assert_eq!(anim.current_sprite_index(), 30);
    }

    #[test]
    fn test_animated_sprite_empty_frames() {
        let mut anim = AnimatedSprite::new(vec![], 8.0, true);
        let finished = anim.advance(1.0);
        assert!(!finished);
        assert_eq!(anim.current_sprite_index(), 0);
    }
}
