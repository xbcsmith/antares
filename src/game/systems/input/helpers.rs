// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Pure helper functions for the input system.

use crate::domain::types::Position;

/// Returns all 8 adjacent positions around a given position.
///
/// Returns tiles in clockwise order starting from North:
/// N, NE, E, SE, S, SW, W, NW
///
/// # Arguments
///
/// * `position` - The center position
///
/// # Returns
///
/// Array of 8 `Position` values representing adjacent tiles
///
/// # Examples
///
/// ```
/// use antares::domain::types::Position;
/// use antares::game::systems::input::get_adjacent_positions;
///
/// let adjacent = get_adjacent_positions(Position::new(5, 5));
///
/// assert_eq!(adjacent[0], Position::new(5, 4));
/// assert_eq!(adjacent[2], Position::new(6, 5));
/// assert_eq!(adjacent.len(), 8);
/// ```
pub fn get_adjacent_positions(position: Position) -> [Position; 8] {
    [
        Position::new(position.x, position.y - 1),     // North
        Position::new(position.x + 1, position.y - 1), // NorthEast
        Position::new(position.x + 1, position.y),     // East
        Position::new(position.x + 1, position.y + 1), // SouthEast
        Position::new(position.x, position.y + 1),     // South
        Position::new(position.x - 1, position.y + 1), // SouthWest
        Position::new(position.x - 1, position.y),     // West
        Position::new(position.x - 1, position.y - 1), // NorthWest
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adjacent_positions_count() {
        let center = Position::new(5, 5);
        let adjacent = get_adjacent_positions(center);
        assert_eq!(adjacent.len(), 8);
    }

    #[test]
    fn test_adjacent_positions_north() {
        let center = Position::new(5, 5);
        let adjacent = get_adjacent_positions(center);
        assert_eq!(adjacent[0], Position::new(5, 4));
    }

    #[test]
    fn test_adjacent_positions_east() {
        let center = Position::new(5, 5);
        let adjacent = get_adjacent_positions(center);
        assert_eq!(adjacent[2], Position::new(6, 5));
    }

    #[test]
    fn test_npc_interaction_adjacent_positions() {
        let center = Position::new(5, 5);
        let adjacent = get_adjacent_positions(center);

        assert!(adjacent.contains(&Position::new(5, 4))); // North
        assert!(adjacent.contains(&Position::new(6, 4))); // NorthEast
        assert!(adjacent.contains(&Position::new(6, 5))); // East
        assert!(adjacent.contains(&Position::new(6, 6))); // SouthEast
        assert!(adjacent.contains(&Position::new(5, 6))); // South
        assert!(adjacent.contains(&Position::new(4, 6))); // SouthWest
        assert!(adjacent.contains(&Position::new(4, 5))); // West
        assert!(adjacent.contains(&Position::new(4, 4))); // NorthWest
    }
}
