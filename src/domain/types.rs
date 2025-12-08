// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Core type aliases and supporting types
//!
//! This module defines the fundamental types used throughout the game,
//! including type aliases for IDs and supporting structures like Position,
//! Direction, DiceRoll, and GameTime.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.6 for complete specifications.

use rand::Rng;
use serde::{Deserialize, Serialize};

// Re-export GameMode from application layer for convenience
pub use crate::application::GameMode;

// ===== Type Aliases =====

/// Item identifier
pub type ItemId = u8;

/// Spell identifier (high byte = school, low byte = spell number)
pub type SpellId = u16;

/// Monster identifier
pub type MonsterId = u8;

/// Map identifier
pub type MapId = u16;

/// Character identifier (index in roster or party)
pub type CharacterId = usize;

/// Town identifier
pub type TownId = u8;

/// Event identifier
pub type EventId = u16;

/// Race identifier (e.g., "human", "elf", "dwarf")
pub type RaceId = String;

// ===== Position =====

/// 2D position on a map
///
/// Uses i32 coordinates to allow negative values during calculations.
///
/// # Examples
///
/// ```
/// use antares::domain::types::Position;
///
/// let start = Position { x: 5, y: 10 };
/// let offset = Position { x: -2, y: 3 };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    /// Creates a new position
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::Position;
    ///
    /// let pos = Position::new(10, 20);
    /// assert_eq!(pos.x, 10);
    /// assert_eq!(pos.y, 20);
    /// ```
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Calculates Manhattan distance to another position
    pub fn manhattan_distance(&self, other: &Position) -> u32 {
        ((self.x - other.x).abs() + (self.y - other.y).abs()) as u32
    }
}

// ===== Direction =====

/// Cardinal directions for movement and facing
///
/// # Examples
///
/// ```
/// use antares::domain::types::{Direction, Position};
///
/// let dir = Direction::North;
/// let pos = Position::new(5, 5);
/// let new_pos = dir.forward(pos);
/// assert_eq!(new_pos, Position::new(5, 4));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    /// Turns 90 degrees to the left
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::Direction;
    ///
    /// let dir = Direction::North;
    /// assert_eq!(dir.turn_left(), Direction::West);
    /// ```
    pub fn turn_left(&self) -> Direction {
        match self {
            Direction::North => Direction::West,
            Direction::West => Direction::South,
            Direction::South => Direction::East,
            Direction::East => Direction::North,
        }
    }

    /// Turns 90 degrees to the right
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::Direction;
    ///
    /// let dir = Direction::North;
    /// assert_eq!(dir.turn_right(), Direction::East);
    /// ```
    pub fn turn_right(&self) -> Direction {
        match self {
            Direction::North => Direction::East,
            Direction::East => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
        }
    }

    /// Returns the position one step forward in this direction
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::{Direction, Position};
    ///
    /// let pos = Position::new(5, 5);
    /// let new_pos = Direction::North.forward(pos);
    /// assert_eq!(new_pos, Position::new(5, 4));
    /// ```
    pub fn forward(&self, pos: Position) -> Position {
        match self {
            Direction::North => Position {
                x: pos.x,
                y: pos.y - 1,
            },
            Direction::East => Position {
                x: pos.x + 1,
                y: pos.y,
            },
            Direction::South => Position {
                x: pos.x,
                y: pos.y + 1,
            },
            Direction::West => Position {
                x: pos.x - 1,
                y: pos.y,
            },
        }
    }
}

// ===== DiceRoll =====

/// Dice roll specification (e.g., 2d6+3)
///
/// Represents a dice roll in RPG notation: XdY+Z
/// - X = number of dice (`count`)
/// - Y = die size (`sides`)
/// - Z = fixed bonus/penalty (`bonus`)
///
/// # Examples
///
/// ```
/// use antares::domain::types::DiceRoll;
/// use rand::rng;
///
/// let roll = DiceRoll::new(2, 6, 3); // 2d6+3
/// let mut rng = rng();
/// let result = roll.roll(&mut rng);
/// assert!(result >= 5 && result <= 15); // Min: 2+3, Max: 12+3
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiceRoll {
    /// Number of dice to roll
    pub count: u8,
    /// Die size (d4, d6, d8, d10, d12, d20, etc.)
    pub sides: u8,
    /// Fixed bonus or penalty added to the total
    pub bonus: i8,
}

impl DiceRoll {
    /// Creates a new dice roll specification
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::DiceRoll;
    ///
    /// let damage = DiceRoll::new(1, 8, 2); // 1d8+2
    /// ```
    pub fn new(count: u8, sides: u8, bonus: i8) -> Self {
        Self {
            count,
            sides,
            bonus,
        }
    }

    /// Rolls the dice and returns the result
    ///
    /// The result is clamped to a minimum of 0 (negative results become 0).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::DiceRoll;
    /// use rand::rng;
    ///
    /// let roll = DiceRoll::new(3, 6, 0); // 3d6
    /// let mut rng = rng();
    /// let result = roll.roll(&mut rng);
    /// assert!(result >= 3 && result <= 18);
    /// ```
    pub fn roll(&self, rng: &mut impl Rng) -> i32 {
        let mut total = self.bonus as i32;
        for _ in 0..self.count {
            total += rng.random_range(1..=self.sides as i32);
        }
        total.max(0)
    }

    /// Returns the minimum possible result
    pub fn min(&self) -> i32 {
        (self.count as i32 + self.bonus as i32).max(0)
    }

    /// Returns the maximum possible result
    pub fn max(&self) -> i32 {
        (self.count as i32 * self.sides as i32 + self.bonus as i32).max(0)
    }

    /// Returns the average result (rounded down)
    pub fn average(&self) -> i32 {
        let avg_per_die = (self.sides as f32 + 1.0) / 2.0;
        ((self.count as f32 * avg_per_die) as i32 + self.bonus as i32).max(0)
    }
}

// ===== GameTime =====

/// Game time tracking
///
/// Tracks the in-game date and time. Days, hours, and minutes advance
/// as the party travels, rests, and performs actions.
///
/// # Examples
///
/// ```
/// use antares::domain::types::GameTime;
///
/// let mut time = GameTime::new(1, 0, 0); // Day 1, midnight
/// time.advance_minutes(90); // Advance 1 hour 30 minutes
/// assert_eq!(time.hour, 1);
/// assert_eq!(time.minute, 30);
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GameTime {
    /// Current day (1-based)
    pub day: u32,
    /// Current hour (0-23)
    pub hour: u8,
    /// Current minute (0-59)
    pub minute: u8,
}

impl GameTime {
    /// Creates a new GameTime at the specified day, hour, and minute
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::GameTime;
    ///
    /// let time = GameTime::new(1, 12, 30); // Day 1, 12:30 PM
    /// assert_eq!(time.day, 1);
    /// assert_eq!(time.hour, 12);
    /// assert_eq!(time.minute, 30);
    /// ```
    pub fn new(day: u32, hour: u8, minute: u8) -> Self {
        Self { day, hour, minute }
    }

    /// Advances time by the specified number of minutes
    ///
    /// Automatically handles overflow into hours and days.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::GameTime;
    ///
    /// let mut time = GameTime::new(1, 23, 30);
    /// time.advance_minutes(45); // Should roll over to next day
    /// assert_eq!(time.day, 2);
    /// assert_eq!(time.hour, 0);
    /// assert_eq!(time.minute, 15);
    /// ```
    pub fn advance_minutes(&mut self, minutes: u32) {
        self.minute += (minutes % 60) as u8;
        let hours = minutes / 60 + (self.minute / 60) as u32;
        self.minute %= 60;

        self.hour += (hours % 24) as u8;
        let days = hours / 24 + (self.hour / 24) as u32;
        self.hour %= 24;

        self.day += days;
    }

    /// Advances time by the specified number of hours
    pub fn advance_hours(&mut self, hours: u32) {
        self.advance_minutes(hours * 60);
    }

    /// Advances time by the specified number of days
    pub fn advance_days(&mut self, days: u32) {
        self.day += days;
    }

    /// Returns true if it is currently nighttime (6 PM to 6 AM)
    pub fn is_night(&self) -> bool {
        self.hour >= 18 || self.hour < 6
    }

    /// Returns true if it is currently daytime (6 AM to 6 PM)
    pub fn is_day(&self) -> bool {
        !self.is_night()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rng;

    #[test]
    fn test_position_creation() {
        let pos = Position::new(10, 20);
        assert_eq!(pos.x, 10);
        assert_eq!(pos.y, 20);
    }

    #[test]
    fn test_position_manhattan_distance() {
        let pos1 = Position::new(0, 0);
        let pos2 = Position::new(3, 4);
        assert_eq!(pos1.manhattan_distance(&pos2), 7);
    }

    #[test]
    fn test_direction_turn_left() {
        assert_eq!(Direction::North.turn_left(), Direction::West);
        assert_eq!(Direction::West.turn_left(), Direction::South);
        assert_eq!(Direction::South.turn_left(), Direction::East);
        assert_eq!(Direction::East.turn_left(), Direction::North);
    }

    #[test]
    fn test_direction_turn_right() {
        assert_eq!(Direction::North.turn_right(), Direction::East);
        assert_eq!(Direction::East.turn_right(), Direction::South);
        assert_eq!(Direction::South.turn_right(), Direction::West);
        assert_eq!(Direction::West.turn_right(), Direction::North);
    }

    #[test]
    fn test_direction_forward() {
        let pos = Position::new(5, 5);
        assert_eq!(Direction::North.forward(pos), Position::new(5, 4));
        assert_eq!(Direction::East.forward(pos), Position::new(6, 5));
        assert_eq!(Direction::South.forward(pos), Position::new(5, 6));
        assert_eq!(Direction::West.forward(pos), Position::new(4, 5));
    }

    #[test]
    fn test_dice_roll_creation() {
        let roll = DiceRoll::new(2, 6, 3);
        assert_eq!(roll.count, 2);
        assert_eq!(roll.sides, 6);
        assert_eq!(roll.bonus, 3);
    }

    #[test]
    fn test_dice_roll() {
        let roll = DiceRoll::new(2, 6, 0);
        let mut rng = rng();

        // Test multiple rolls to ensure they're in valid range
        for _ in 0..100 {
            let result = roll.roll(&mut rng);
            assert!((2..=12).contains(&result));
        }
    }

    #[test]
    fn test_dice_roll_min_max_average() {
        let roll = DiceRoll::new(3, 6, 2); // 3d6+2
        assert_eq!(roll.min(), 5); // 3+2
        assert_eq!(roll.max(), 20); // 18+2
        assert_eq!(roll.average(), 12); // (3*3.5)+2 = 12.5 -> 12
    }

    #[test]
    fn test_dice_roll_negative_bonus() {
        let roll = DiceRoll::new(1, 6, -10);
        let mut rng = rng();
        let result = roll.roll(&mut rng);
        assert_eq!(result, 0); // Clamped to minimum of 0
    }

    #[test]
    fn test_game_time_creation() {
        let time = GameTime::new(5, 14, 30);
        assert_eq!(time.day, 5);
        assert_eq!(time.hour, 14);
        assert_eq!(time.minute, 30);
    }

    #[test]
    fn test_game_time_advance_minutes() {
        let mut time = GameTime::new(1, 0, 0);
        time.advance_minutes(30);
        assert_eq!(time.day, 1);
        assert_eq!(time.hour, 0);
        assert_eq!(time.minute, 30);
    }

    #[test]
    fn test_game_time_advance_hours() {
        let mut time = GameTime::new(1, 10, 0);
        time.advance_hours(5);
        assert_eq!(time.day, 1);
        assert_eq!(time.hour, 15);
        assert_eq!(time.minute, 0);
    }

    #[test]
    fn test_game_time_advance_with_overflow() {
        let mut time = GameTime::new(1, 23, 30);
        time.advance_minutes(45); // Should overflow to next day
        assert_eq!(time.day, 2);
        assert_eq!(time.hour, 0);
        assert_eq!(time.minute, 15);
    }

    #[test]
    fn test_game_time_advance_multiple_days() {
        let mut time = GameTime::new(1, 12, 0);
        time.advance_days(5);
        assert_eq!(time.day, 6);
        assert_eq!(time.hour, 12);
        assert_eq!(time.minute, 0);
    }

    #[test]
    fn test_game_time_is_night() {
        let mut time = GameTime::new(1, 20, 0); // 8 PM
        assert!(time.is_night());
        assert!(!time.is_day());

        time = GameTime::new(1, 3, 0); // 3 AM
        assert!(time.is_night());

        time = GameTime::new(1, 12, 0); // Noon
        assert!(!time.is_night());
        assert!(time.is_day());
    }
}
