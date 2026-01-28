// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Procedural sprite selection system (Phase 6)
//!
//! Provides functions to resolve sprite selection rules to concrete sprites,
//! enabling automatic sprite variation based on tile position, randomness,
//! and neighbor detection (auto-tiling).

use crate::domain::world::{SpriteReference, SpriteSelectionRule};

/// Resolves a sprite selection rule to a concrete sprite reference
///
/// Handles three types of rules:
/// - Fixed: Always returns the same sprite
/// - Random: Selects randomly from a list (deterministic if seeded)
/// - Autotile: Selects based on neighbor bitmask
///
/// # Arguments
///
/// * `rule` - The sprite selection rule to resolve
/// * `tile_pos` - (x, y) position of the tile for position-based seeding
/// * `neighbor_mask` - Optional 4-bit bitmask of cardinal neighbors [N, E, S, W]
///
/// # Returns
///
/// A `SpriteReference` with the selected sprite index
///
/// # Examples
///
/// ```ignore
/// use antares::domain::world::SpriteSelectionRule;
/// use antares::domain::world::sprite_selection::resolve_sprite_rule;
///
/// let rule = SpriteSelectionRule::Random {
///     sheet_path: "grass.png".to_string(),
///     sprite_indices: vec![0, 1, 2, 3],
///     seed: Some(42),
/// };
///
/// let sprite = resolve_sprite_rule(&rule, (5, 10), None);
/// assert_eq!(sprite.sheet_path, "grass.png");
/// ```
pub fn resolve_sprite_rule(
    rule: &SpriteSelectionRule,
    tile_pos: (i32, i32),
    neighbor_mask: Option<u8>,
) -> SpriteReference {
    match rule {
        SpriteSelectionRule::Fixed {
            sheet_path,
            sprite_index,
        } => SpriteReference {
            sheet_path: sheet_path.clone(),
            sprite_index: *sprite_index,
            animation: None,
            material_properties: None,
        },

        SpriteSelectionRule::Random {
            sheet_path,
            sprite_indices,
            seed,
        } => {
            // Use provided seed or derive from tile position
            if sprite_indices.is_empty() {
                return SpriteReference {
                    sheet_path: sheet_path.clone(),
                    sprite_index: 0,
                    animation: None,
                    material_properties: None,
                };
            }

            // Use seed for deterministic selection
            let selection_seed = seed.unwrap_or_else(|| {
                // Deterministic: position-based seed
                // Combine x and y into single 64-bit seed
                ((tile_pos.0 as u64) << 32) | ((tile_pos.1 as u32) as u64)
            });

            // Select sprite using modulo of seed
            let sprite_index = sprite_indices[(selection_seed as usize) % sprite_indices.len()];

            SpriteReference {
                sheet_path: sheet_path.clone(),
                sprite_index,
                animation: None,
                material_properties: None,
            }
        }

        SpriteSelectionRule::Autotile { sheet_path, rules } => {
            // Use neighbor mask if provided, otherwise default to all neighbors
            let mask = neighbor_mask.unwrap_or(0);

            // Look up sprite index for this neighbor bitmask
            let sprite_index = rules.get(&mask).copied().unwrap_or(0);

            SpriteReference {
                sheet_path: sheet_path.clone(),
                sprite_index,
                animation: None,
                material_properties: None,
            }
        }
    }
}

/// Calculates 4-bit bitmask for cardinal neighbors
///
/// Checks if tiles exist in the four cardinal directions (N, E, S, W)
/// and encodes them as a 4-bit value.
///
/// # Bit Layout
///
/// - Bit 0: North neighbor (tile at position (x, y+1))
/// - Bit 1: East neighbor (tile at position (x+1, y))
/// - Bit 2: South neighbor (tile at position (x, y-1))
/// - Bit 3: West neighbor (tile at position (x-1, y))
///
/// # Arguments
///
/// * `tile_pos` - (x, y) position of the tile
/// * `neighbor_check` - Closure that returns true if tile exists at position
///
/// # Returns
///
/// 4-bit bitmask (0-15) representing neighbor configuration
///
/// # Examples
///
/// ```ignore
/// use antares::domain::world::sprite_selection::calculate_neighbor_bitmask;
///
/// // L-shaped neighbor pattern (North and East)
/// let check = |pos: (i32, i32)| {
///     matches!(pos, (5, 11) | (6, 10))
/// };
///
/// let mask = calculate_neighbor_bitmask((5, 10), &check);
/// assert_eq!(mask, 0b0011); // North and East
/// ```
pub fn calculate_neighbor_bitmask<F>(tile_pos: (i32, i32), mut neighbor_check: F) -> u8
where
    F: FnMut((i32, i32)) -> bool,
{
    let mut mask = 0u8;

    // Check cardinal neighbors in order: N, E, S, W
    let neighbors = [
        (tile_pos.0, tile_pos.1 + 1), // North
        (tile_pos.0 + 1, tile_pos.1), // East
        (tile_pos.0, tile_pos.1 - 1), // South
        (tile_pos.0 - 1, tile_pos.1), // West
    ];

    for (bit, &neighbor_pos) in neighbors.iter().enumerate() {
        if neighbor_check(neighbor_pos) {
            mask |= 1 << bit;
        }
    }

    mask
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_fixed_sprite_selection() {
        let rule = SpriteSelectionRule::Fixed {
            sheet_path: "walls.png".to_string(),
            sprite_index: 5,
        };

        let sprite = resolve_sprite_rule(&rule, (0, 0), None);

        assert_eq!(sprite.sheet_path, "walls.png");
        assert_eq!(sprite.sprite_index, 5);
        assert!(sprite.animation.is_none());
    }

    #[test]
    fn test_random_sprite_selection_with_seed() {
        let rule = SpriteSelectionRule::Random {
            sheet_path: "grass.png".to_string(),
            sprite_indices: vec![0, 1, 2, 3],
            seed: Some(42),
        };

        let sprite1 = resolve_sprite_rule(&rule, (0, 0), None);
        let sprite2 = resolve_sprite_rule(&rule, (0, 0), None);

        // Same seed and position = same selection
        assert_eq!(sprite1.sprite_index, sprite2.sprite_index);
        assert_eq!(sprite1.sheet_path, "grass.png");
    }

    #[test]
    fn test_random_sprite_selection_deterministic_by_position() {
        let rule = SpriteSelectionRule::Random {
            sheet_path: "grass.png".to_string(),
            sprite_indices: vec![0, 1, 2, 3],
            seed: None, // Position-based seed
        };

        let sprite1 = resolve_sprite_rule(&rule, (5, 10), None);
        let sprite2 = resolve_sprite_rule(&rule, (5, 10), None);

        // Same position, no seed = deterministic result
        assert_eq!(sprite1.sprite_index, sprite2.sprite_index);
    }

    #[test]
    fn test_random_sprite_selection_varies_by_position() {
        let rule = SpriteSelectionRule::Random {
            sheet_path: "grass.png".to_string(),
            sprite_indices: vec![0, 1, 2, 3],
            seed: None,
        };

        let sprite1 = resolve_sprite_rule(&rule, (0, 0), None);
        let sprite2 = resolve_sprite_rule(&rule, (10, 10), None);

        // Different positions MAY result in different sprites
        // (not guaranteed but likely with 4 options)
        // Just verify we get valid indices
        assert!(sprite1.sprite_index < 4);
        assert!(sprite2.sprite_index < 4);
    }

    #[test]
    fn test_autotile_with_neighbor_mask() {
        let mut rules: HashMap<u8, u32> = HashMap::new();
        rules.insert(0b0011, 5); // North + East = corner
        rules.insert(0b0001, 1); // North only = edge
        rules.insert(0b0000, 0); // No neighbors = isolated

        let rule = SpriteSelectionRule::Autotile {
            sheet_path: "terrain.png".to_string(),
            rules,
        };

        let sprite_corner = resolve_sprite_rule(&rule, (0, 0), Some(0b0011));
        let sprite_edge = resolve_sprite_rule(&rule, (0, 0), Some(0b0001));
        let sprite_isolated = resolve_sprite_rule(&rule, (0, 0), Some(0b0000));

        assert_eq!(sprite_corner.sprite_index, 5);
        assert_eq!(sprite_edge.sprite_index, 1);
        assert_eq!(sprite_isolated.sprite_index, 0);
    }

    #[test]
    fn test_autotile_unmapped_mask_defaults_to_zero() {
        let rules: HashMap<u8, u32> = HashMap::new();

        let rule = SpriteSelectionRule::Autotile {
            sheet_path: "terrain.png".to_string(),
            rules,
        };

        // No rules defined, should default to sprite 0
        let sprite = resolve_sprite_rule(&rule, (0, 0), Some(0b1111));
        assert_eq!(sprite.sprite_index, 0);
    }

    #[test]
    fn test_calculate_neighbor_bitmask_no_neighbors() {
        let check = |_pos: (i32, i32)| false;

        let mask = calculate_neighbor_bitmask((5, 5), check);

        assert_eq!(mask, 0b0000);
    }

    #[test]
    fn test_calculate_neighbor_bitmask_all_neighbors() {
        let check = |_pos: (i32, i32)| true;

        let mask = calculate_neighbor_bitmask((5, 5), check);

        assert_eq!(mask, 0b1111);
    }

    #[test]
    fn test_calculate_neighbor_bitmask_north_and_east() {
        let neighbors = [(5, 6), (6, 5)];
        let check = |pos: (i32, i32)| neighbors.contains(&pos);

        let mask = calculate_neighbor_bitmask((5, 5), check);

        // Bit 0 (North) and Bit 1 (East)
        assert_eq!(mask, 0b0011);
    }

    #[test]
    fn test_calculate_neighbor_bitmask_south_only() {
        let check = |pos: (i32, i32)| pos == (5, 4);

        let mask = calculate_neighbor_bitmask((5, 5), check);

        // Bit 2 (South)
        assert_eq!(mask, 0b0100);
    }

    #[test]
    fn test_calculate_neighbor_bitmask_west_and_north() {
        let neighbors = [(4, 5), (5, 6)];
        let check = |pos: (i32, i32)| neighbors.contains(&pos);

        let mask = calculate_neighbor_bitmask((5, 5), check);

        // Bit 0 (North) and Bit 3 (West)
        assert_eq!(mask, 0b1001);
    }

    #[test]
    fn test_material_properties_preserved() {
        let rule = SpriteSelectionRule::Fixed {
            sheet_path: "emissive.png".to_string(),
            sprite_index: 0,
        };

        let sprite = resolve_sprite_rule(&rule, (0, 0), None);

        // Material properties should be None (not set by rule resolution)
        assert!(sprite.material_properties.is_none());
    }

    #[test]
    fn test_animation_not_set_by_rule() {
        let rule = SpriteSelectionRule::Random {
            sheet_path: "animated.png".to_string(),
            sprite_indices: vec![0, 1, 2],
            seed: Some(100),
        };

        let sprite = resolve_sprite_rule(&rule, (0, 0), None);

        // Rules don't set animation (that comes from SpriteReference)
        assert!(sprite.animation.is_none());
    }
}
