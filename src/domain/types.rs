// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Core type aliases and supporting types
//!
//! This module defines the fundamental types used throughout the game,
//! including type aliases for IDs and supporting structures like Position,
//! Direction, DiceRoll, GameTime, and TimeOfDay.
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

/// Innkeeper NPC identifier (references NpcId with is_innkeeper=true)
pub type InnkeeperId = String;

/// Event identifier
pub type EventId = u16;

/// Race identifier (e.g., "human", "elf", "dwarf")
pub type RaceId = String;

/// Creature identifier (for visual/mesh definitions)
pub type CreatureId = u32;

/// Mesh identifier (for individual meshes within a creature)
pub type MeshId = u32;

/// Furniture definition identifier
///
/// Used to reference `FurnitureDefinition` entries in `FurnitureDatabase`.
/// IDs are assigned per-campaign starting at 1.
pub type FurnitureId = u32;

/// Furniture mesh identifier (for custom OBJ-imported meshes)
///
/// References entries in `furniture_mesh_registry.ron`.
/// Conventionally in the 10000+ range to avoid collisions with creature IDs.
pub type FurnitureMeshId = u32;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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

    /// Converts this cardinal direction to a yaw angle in radians
    ///
    /// The yaw is measured clockwise when viewed from above (positive Y axis),
    /// matching Bevy's `Quat::from_rotation_y` convention:
    ///
    /// | Direction | Yaw (radians)          |
    /// |-----------|------------------------|
    /// | North     | 0.0                    |
    /// | East      | π/2  (FRAC_PI_2)       |
    /// | South     | π    (PI)              |
    /// | West      | 3π/2 (3 * FRAC_PI_2)  |
    ///
    /// Use the result directly with `Quat::from_rotation_y(dir.direction_to_yaw_radians())`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::Direction;
    /// use std::f32::consts::{FRAC_PI_2, PI};
    ///
    /// assert_eq!(Direction::North.direction_to_yaw_radians(), 0.0);
    /// assert!((Direction::East.direction_to_yaw_radians() - FRAC_PI_2).abs() < 1e-6);
    /// assert!((Direction::South.direction_to_yaw_radians() - PI).abs() < 1e-6);
    /// assert!((Direction::West.direction_to_yaw_radians() - 3.0 * FRAC_PI_2).abs() < 1e-6);
    /// ```
    pub fn direction_to_yaw_radians(&self) -> f32 {
        match self {
            Direction::North => 0.0,
            Direction::East => std::f32::consts::FRAC_PI_2,
            Direction::South => std::f32::consts::PI,
            Direction::West => 3.0 * std::f32::consts::FRAC_PI_2,
        }
    }

    /// Constructs the nearest cardinal `Direction` from a yaw angle in radians
    ///
    /// Normalises the input to `[0, 2π)` then rounds to the closest 90° cardinal.
    /// This is the inverse of [`Direction::direction_to_yaw_radians`].
    ///
    /// | Yaw range              | Result  |
    /// |------------------------|---------|
    /// | \[-π/4,  π/4)          | North   |
    /// | \[ π/4,  3π/4)         | East    |
    /// | \[3π/4,  5π/4)         | South   |
    /// | \[5π/4,  7π/4)         | West    |
    /// | \[7π/4,  2π) or wrap   | North   |
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::Direction;
    /// use std::f32::consts::{FRAC_PI_2, PI};
    ///
    /// assert_eq!(Direction::from_yaw_radians(0.0), Direction::North);
    /// assert_eq!(Direction::from_yaw_radians(FRAC_PI_2), Direction::East);
    /// assert_eq!(Direction::from_yaw_radians(PI), Direction::South);
    /// assert_eq!(Direction::from_yaw_radians(3.0 * FRAC_PI_2), Direction::West);
    /// // Values between cardinals snap to the nearest one
    /// assert_eq!(Direction::from_yaw_radians(FRAC_PI_2 / 2.0 - 0.01), Direction::North);
    /// assert_eq!(Direction::from_yaw_radians(FRAC_PI_2 / 2.0 + 0.01), Direction::East);
    /// ```
    pub fn from_yaw_radians(yaw: f32) -> Direction {
        use std::f32::consts::{FRAC_PI_2, TAU};
        // Normalise to [0, 2π)
        let normalised = ((yaw % TAU) + TAU) % TAU;
        // Each quadrant spans π/2 radians; offset by π/4 so each cardinal is centred
        let sector = ((normalised + FRAC_PI_2 / 2.0) / FRAC_PI_2) as u32 % 4;
        match sector {
            0 => Direction::North,
            1 => Direction::East,
            2 => Direction::South,
            3 => Direction::West,
            _ => Direction::North,
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

// ===== TimeOfDay =====

/// Categorised period of the day for event gating and visual effects.
///
/// Used to gate map events by time, drive ambient lighting changes,
/// and provide a human-readable time period to the HUD clock.
///
/// Hour boundaries (24-hour clock):
/// - Dawn:      05:00 – 07:59
/// - Morning:   08:00 – 11:59
/// - Afternoon: 12:00 – 15:59
/// - Dusk:      16:00 – 18:59
/// - Evening:   19:00 – 21:59
/// - Night:     22:00 – 04:59
///
/// # Examples
///
/// ```
/// use antares::domain::types::{GameTime, TimeOfDay};
///
/// assert_eq!(GameTime::new(1, 6, 0).time_of_day(), TimeOfDay::Dawn);
/// assert_eq!(GameTime::new(1, 10, 0).time_of_day(), TimeOfDay::Morning);
/// assert_eq!(GameTime::new(1, 14, 0).time_of_day(), TimeOfDay::Afternoon);
/// assert_eq!(GameTime::new(1, 17, 0).time_of_day(), TimeOfDay::Dusk);
/// assert_eq!(GameTime::new(1, 20, 0).time_of_day(), TimeOfDay::Evening);
/// assert_eq!(GameTime::new(1, 23, 0).time_of_day(), TimeOfDay::Night);
/// assert_eq!(GameTime::new(1, 2, 0).time_of_day(), TimeOfDay::Night);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeOfDay {
    /// 05:00–07:59 — pale light, roosters crow
    Dawn,
    /// 08:00–11:59 — full daylight
    Morning,
    /// 12:00–15:59 — peak brightness
    Afternoon,
    /// 16:00–18:59 — golden light, shadows lengthen
    Dusk,
    /// 19:00–21:59 — dark but not full night
    Evening,
    /// 22:00–04:59 — pitch black without a light source
    Night,
}

impl TimeOfDay {
    /// Returns a short human-readable label for the period.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::TimeOfDay;
    ///
    /// assert_eq!(TimeOfDay::Dawn.label(), "Dawn");
    /// assert_eq!(TimeOfDay::Night.label(), "Night");
    /// ```
    pub fn label(&self) -> &'static str {
        match self {
            TimeOfDay::Dawn => "Dawn",
            TimeOfDay::Morning => "Morning",
            TimeOfDay::Afternoon => "Afternoon",
            TimeOfDay::Dusk => "Dusk",
            TimeOfDay::Evening => "Evening",
            TimeOfDay::Night => "Night",
        }
    }

    /// Returns `true` for periods where ambient darkness applies
    /// (Evening and Night require a light source outdoors).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::TimeOfDay;
    ///
    /// assert!(TimeOfDay::Night.is_dark());
    /// assert!(TimeOfDay::Evening.is_dark());
    /// assert!(!TimeOfDay::Dawn.is_dark());
    /// assert!(!TimeOfDay::Morning.is_dark());
    /// assert!(!TimeOfDay::Afternoon.is_dark());
    /// assert!(!TimeOfDay::Dusk.is_dark());
    /// ```
    pub fn is_dark(&self) -> bool {
        matches!(self, TimeOfDay::Evening | TimeOfDay::Night)
    }
}

// ===== Calendar Constants =====

/// Number of months in a game year.
pub const MONTHS_PER_YEAR: u32 = 12;

/// Number of days in a game month (all months are equal length).
pub const DAYS_PER_MONTH: u32 = 30;

/// Number of days in a game year (MONTHS_PER_YEAR × DAYS_PER_MONTH = 360).
pub const DAYS_PER_YEAR: u32 = MONTHS_PER_YEAR * DAYS_PER_MONTH;

// ===== Serde default helpers =====

fn default_year() -> u32 {
    1
}

fn default_month() -> u32 {
    1
}

// ===== GameTime =====

/// Game time tracking
///
/// Tracks the in-game calendar date and time. Years, months, days, hours, and
/// minutes advance as the party travels, rests, and performs actions.
///
/// The calendar uses a simplified fixed-length structure:
/// - 12 months per year ([`MONTHS_PER_YEAR`])
/// - 30 days per month ([`DAYS_PER_MONTH`])
/// - 360 days per year ([`DAYS_PER_YEAR`])
///
/// All fields are 1-based for `year`, `month`, and `day`; `hour` is 0–23;
/// `minute` is 0–59.
///
/// # Backward Compatibility
///
/// The `year` and `month` fields use `#[serde(default)]` so existing save
/// files and RON data that only contain `day`, `hour`, and `minute` will
/// deserialize with `year = 1, month = 1`.
///
/// # Examples
///
/// ```
/// use antares::domain::types::GameTime;
///
/// let mut time = GameTime::new(1, 0, 0); // Year 1, Month 1, Day 1, midnight
/// time.advance_minutes(90); // Advance 1 hour 30 minutes
/// assert_eq!(time.hour, 1);
/// assert_eq!(time.minute, 30);
///
/// // Full constructor
/// let full = GameTime::new_full(2, 6, 15, 8, 30);
/// assert_eq!(full.year, 2);
/// assert_eq!(full.month, 6);
/// assert_eq!(full.day, 15);
/// assert_eq!(full.hour, 8);
/// assert_eq!(full.minute, 30);
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GameTime {
    /// Current year (1-based)
    #[serde(default = "default_year")]
    pub year: u32,
    /// Current month within the year (1-based, 1–12)
    #[serde(default = "default_month")]
    pub month: u32,
    /// Current day within the month (1-based, 1–30)
    pub day: u32,
    /// Current hour (0-23)
    pub hour: u8,
    /// Current minute (0-59)
    pub minute: u8,
}

impl GameTime {
    /// Creates a new `GameTime` at the specified day, hour, and minute.
    ///
    /// Sets `year = 1` and `month = 1` for backward compatibility with
    /// existing call sites. Use [`GameTime::new_full`] to specify year and
    /// month explicitly.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::GameTime;
    ///
    /// let time = GameTime::new(1, 12, 30); // Year 1, Month 1, Day 1, 12:30 PM
    /// assert_eq!(time.year, 1);
    /// assert_eq!(time.month, 1);
    /// assert_eq!(time.day, 1);
    /// assert_eq!(time.hour, 12);
    /// assert_eq!(time.minute, 30);
    /// ```
    pub fn new(day: u32, hour: u8, minute: u8) -> Self {
        Self {
            year: 1,
            month: 1,
            day,
            hour,
            minute,
        }
    }

    /// Creates a new `GameTime` with all five calendar and clock fields set.
    ///
    /// # Arguments
    ///
    /// * `year`   - 1-based year (e.g. `1` for the first in-game year)
    /// * `month`  - 1-based month within the year (1–12)
    /// * `day`    - 1-based day within the month (1–30)
    /// * `hour`   - Hour of day (0–23)
    /// * `minute` - Minute of hour (0–59)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::GameTime;
    ///
    /// let time = GameTime::new_full(3, 11, 20, 14, 45);
    /// assert_eq!(time.year, 3);
    /// assert_eq!(time.month, 11);
    /// assert_eq!(time.day, 20);
    /// assert_eq!(time.hour, 14);
    /// assert_eq!(time.minute, 45);
    /// ```
    pub fn new_full(year: u32, month: u32, day: u32, hour: u8, minute: u8) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
        }
    }

    /// Returns the total cumulative days elapsed since the very beginning of
    /// the calendar (Year 1, Month 1, Day 1).
    ///
    /// This is the canonical way to compare two points in time by day count
    /// and is used by [`crate::domain::world::TimeCondition::AfterDay`] and
    /// [`crate::domain::world::TimeCondition::BeforeDay`] so that their
    /// thresholds continue to mean "total elapsed days" even after `day` was
    /// narrowed to mean "day within month".
    ///
    /// # Formula
    ///
    /// `(year - 1) * DAYS_PER_YEAR + (month - 1) * DAYS_PER_MONTH + day`
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::GameTime;
    ///
    /// // Year 1, Month 1, Day 1 → day 1
    /// assert_eq!(GameTime::new(1, 0, 0).total_days(), 1);
    ///
    /// // Year 1, Month 2, Day 10 → 30 + 10 = 40
    /// assert_eq!(GameTime::new_full(1, 2, 10, 0, 0).total_days(), 40);
    ///
    /// // Year 2, Month 1, Day 1 → 360 + 1 = 361
    /// assert_eq!(GameTime::new_full(2, 1, 1, 0, 0).total_days(), 361);
    /// ```
    pub fn total_days(&self) -> u32 {
        (self.year - 1) * DAYS_PER_YEAR + (self.month - 1) * DAYS_PER_MONTH + self.day
    }

    /// Advances time by the specified number of minutes.
    ///
    /// Automatically handles rollover into hours, days, months, and years.
    ///
    /// | Boundary           | Roll-over rule                          |
    /// |--------------------|-----------------------------------------|
    /// | 60 minutes         | → increment hour                        |
    /// | 24 hours           | → increment day                         |
    /// | `DAYS_PER_MONTH`   | → increment month, reset day to 1       |
    /// | `MONTHS_PER_YEAR`  | → increment year, reset month to 1      |
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
        self.apply_day_rollover();
    }

    /// Advances time by the specified number of hours.
    pub fn advance_hours(&mut self, hours: u32) {
        self.advance_minutes(hours * 60);
    }

    /// Advances time by the specified number of days.
    ///
    /// Handles rollover into months and years via the same logic as
    /// [`GameTime::advance_minutes`].
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::GameTime;
    ///
    /// // Advancing 31 days from Month 1, Day 1 should land on Month 2, Day 1
    /// let mut time = GameTime::new(1, 0, 0);
    /// time.advance_days(31);
    /// assert_eq!(time.month, 2);
    /// assert_eq!(time.day, 1);
    /// ```
    pub fn advance_days(&mut self, days: u32) {
        self.day += days;
        self.apply_day_rollover();
    }

    /// Applies month and year rollover after `self.day` has been incremented.
    ///
    /// Called internally by [`advance_minutes`] and [`advance_days`].
    fn apply_day_rollover(&mut self) {
        while self.day > DAYS_PER_MONTH {
            self.day -= DAYS_PER_MONTH;
            self.month += 1;
        }
        while self.month > MONTHS_PER_YEAR {
            self.month -= MONTHS_PER_YEAR;
            self.year += 1;
        }
    }

    /// Returns the current [`TimeOfDay`] period based on the hour.
    ///
    /// | Period    | Hours         |
    /// |-----------|---------------|
    /// | Dawn      | 05:00 – 07:59 |
    /// | Morning   | 08:00 – 11:59 |
    /// | Afternoon | 12:00 – 15:59 |
    /// | Dusk      | 16:00 – 18:59 |
    /// | Evening   | 19:00 – 21:59 |
    /// | Night     | 22:00 – 04:59 |
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::{GameTime, TimeOfDay};
    ///
    /// assert_eq!(GameTime::new(1, 5, 0).time_of_day(), TimeOfDay::Dawn);
    /// assert_eq!(GameTime::new(1, 8, 0).time_of_day(), TimeOfDay::Morning);
    /// assert_eq!(GameTime::new(1, 12, 0).time_of_day(), TimeOfDay::Afternoon);
    /// assert_eq!(GameTime::new(1, 16, 0).time_of_day(), TimeOfDay::Dusk);
    /// assert_eq!(GameTime::new(1, 19, 0).time_of_day(), TimeOfDay::Evening);
    /// assert_eq!(GameTime::new(1, 22, 0).time_of_day(), TimeOfDay::Night);
    /// assert_eq!(GameTime::new(1, 3, 0).time_of_day(), TimeOfDay::Night);
    /// ```
    pub fn time_of_day(&self) -> TimeOfDay {
        match self.hour {
            5..=7 => TimeOfDay::Dawn,
            8..=11 => TimeOfDay::Morning,
            12..=15 => TimeOfDay::Afternoon,
            16..=18 => TimeOfDay::Dusk,
            19..=21 => TimeOfDay::Evening,
            // 22-23 and 0-4 are Night
            _ => TimeOfDay::Night,
        }
    }

    /// Returns `true` if it is currently nighttime (Evening or Night period).
    ///
    /// Delegates to [`GameTime::time_of_day`] for consistency with the
    /// [`TimeOfDay`] system.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::GameTime;
    ///
    /// assert!(GameTime::new(1, 20, 0).is_night()); // Evening
    /// assert!(GameTime::new(1, 23, 0).is_night()); // Night
    /// assert!(GameTime::new(1, 3, 0).is_night());  // Night (early morning)
    /// assert!(!GameTime::new(1, 12, 0).is_night()); // Afternoon
    /// ```
    pub fn is_night(&self) -> bool {
        matches!(self.time_of_day(), TimeOfDay::Evening | TimeOfDay::Night)
    }

    /// Returns `true` if it is currently daytime (Dawn through Dusk).
    ///
    /// Delegates to [`GameTime::time_of_day`] for consistency with the
    /// [`TimeOfDay`] system.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::GameTime;
    ///
    /// assert!(GameTime::new(1, 6, 0).is_day());   // Dawn
    /// assert!(GameTime::new(1, 14, 0).is_day());  // Afternoon
    /// assert!(GameTime::new(1, 17, 0).is_day());  // Dusk
    /// assert!(!GameTime::new(1, 22, 0).is_day()); // Night
    /// ```
    pub fn is_day(&self) -> bool {
        !self.is_night()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rng;

    // ===== TimeOfDay Tests =====

    #[test]
    fn test_time_of_day_night_early_morning() {
        // 00:00–04:59 is Night
        assert_eq!(GameTime::new(1, 0, 0).time_of_day(), TimeOfDay::Night);
        assert_eq!(GameTime::new(1, 1, 0).time_of_day(), TimeOfDay::Night);
        assert_eq!(GameTime::new(1, 4, 59).time_of_day(), TimeOfDay::Night);
    }

    #[test]
    fn test_time_of_day_dawn() {
        // 05:00–07:59 is Dawn
        assert_eq!(GameTime::new(1, 5, 0).time_of_day(), TimeOfDay::Dawn);
        assert_eq!(GameTime::new(1, 6, 30).time_of_day(), TimeOfDay::Dawn);
        assert_eq!(GameTime::new(1, 7, 59).time_of_day(), TimeOfDay::Dawn);
    }

    #[test]
    fn test_time_of_day_morning() {
        // 08:00–11:59 is Morning
        assert_eq!(GameTime::new(1, 8, 0).time_of_day(), TimeOfDay::Morning);
        assert_eq!(GameTime::new(1, 10, 0).time_of_day(), TimeOfDay::Morning);
        assert_eq!(GameTime::new(1, 11, 59).time_of_day(), TimeOfDay::Morning);
    }

    #[test]
    fn test_time_of_day_afternoon() {
        // 12:00–15:59 is Afternoon
        assert_eq!(GameTime::new(1, 12, 0).time_of_day(), TimeOfDay::Afternoon);
        assert_eq!(GameTime::new(1, 14, 0).time_of_day(), TimeOfDay::Afternoon);
        assert_eq!(GameTime::new(1, 15, 59).time_of_day(), TimeOfDay::Afternoon);
    }

    #[test]
    fn test_time_of_day_dusk() {
        // 16:00–18:59 is Dusk
        assert_eq!(GameTime::new(1, 16, 0).time_of_day(), TimeOfDay::Dusk);
        assert_eq!(GameTime::new(1, 17, 30).time_of_day(), TimeOfDay::Dusk);
        assert_eq!(GameTime::new(1, 18, 59).time_of_day(), TimeOfDay::Dusk);
    }

    #[test]
    fn test_time_of_day_evening() {
        // 19:00–21:59 is Evening
        assert_eq!(GameTime::new(1, 19, 0).time_of_day(), TimeOfDay::Evening);
        assert_eq!(GameTime::new(1, 20, 30).time_of_day(), TimeOfDay::Evening);
        assert_eq!(GameTime::new(1, 21, 59).time_of_day(), TimeOfDay::Evening);
    }

    #[test]
    fn test_time_of_day_night() {
        // 22:00–23:59 is Night
        assert_eq!(GameTime::new(1, 22, 0).time_of_day(), TimeOfDay::Night);
        assert_eq!(GameTime::new(1, 23, 59).time_of_day(), TimeOfDay::Night);
    }

    #[test]
    fn test_time_of_day_boundary_transitions() {
        // Test exact boundary hours
        assert_eq!(GameTime::new(1, 4, 59).time_of_day(), TimeOfDay::Night);
        assert_eq!(GameTime::new(1, 5, 0).time_of_day(), TimeOfDay::Dawn);
        assert_eq!(GameTime::new(1, 7, 59).time_of_day(), TimeOfDay::Dawn);
        assert_eq!(GameTime::new(1, 8, 0).time_of_day(), TimeOfDay::Morning);
        assert_eq!(GameTime::new(1, 11, 59).time_of_day(), TimeOfDay::Morning);
        assert_eq!(GameTime::new(1, 12, 0).time_of_day(), TimeOfDay::Afternoon);
        assert_eq!(GameTime::new(1, 15, 59).time_of_day(), TimeOfDay::Afternoon);
        assert_eq!(GameTime::new(1, 16, 0).time_of_day(), TimeOfDay::Dusk);
        assert_eq!(GameTime::new(1, 18, 59).time_of_day(), TimeOfDay::Dusk);
        assert_eq!(GameTime::new(1, 19, 0).time_of_day(), TimeOfDay::Evening);
        assert_eq!(GameTime::new(1, 21, 59).time_of_day(), TimeOfDay::Evening);
        assert_eq!(GameTime::new(1, 22, 0).time_of_day(), TimeOfDay::Night);
    }

    #[test]
    fn test_is_night_delegates_to_time_of_day() {
        // Evening (19–21) should be night
        assert!(GameTime::new(1, 19, 0).is_night());
        assert!(GameTime::new(1, 21, 59).is_night());
        // Night (22–04) should be night
        assert!(GameTime::new(1, 22, 0).is_night());
        assert!(GameTime::new(1, 23, 59).is_night());
        assert!(GameTime::new(1, 0, 0).is_night());
        assert!(GameTime::new(1, 4, 59).is_night());
        // Dawn/Morning/Afternoon/Dusk should NOT be night
        assert!(!GameTime::new(1, 5, 0).is_night());
        assert!(!GameTime::new(1, 8, 0).is_night());
        assert!(!GameTime::new(1, 12, 0).is_night());
        assert!(!GameTime::new(1, 16, 0).is_night());
    }

    #[test]
    fn test_is_day_is_inverse_of_is_night() {
        for hour in 0u8..24 {
            let t = GameTime::new(1, hour, 0);
            assert_eq!(t.is_day(), !t.is_night(), "hour {} mismatch", hour);
        }
    }

    #[test]
    fn test_time_of_day_label() {
        assert_eq!(TimeOfDay::Dawn.label(), "Dawn");
        assert_eq!(TimeOfDay::Morning.label(), "Morning");
        assert_eq!(TimeOfDay::Afternoon.label(), "Afternoon");
        assert_eq!(TimeOfDay::Dusk.label(), "Dusk");
        assert_eq!(TimeOfDay::Evening.label(), "Evening");
        assert_eq!(TimeOfDay::Night.label(), "Night");
    }

    #[test]
    fn test_time_of_day_is_dark() {
        assert!(TimeOfDay::Evening.is_dark());
        assert!(TimeOfDay::Night.is_dark());
        assert!(!TimeOfDay::Dawn.is_dark());
        assert!(!TimeOfDay::Morning.is_dark());
        assert!(!TimeOfDay::Afternoon.is_dark());
        assert!(!TimeOfDay::Dusk.is_dark());
    }

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

    // ===== Direction yaw conversion tests =====

    #[test]
    fn test_direction_to_yaw_north() {
        assert_eq!(
            Direction::North.direction_to_yaw_radians(),
            0.0,
            "North should map to 0 radians"
        );
    }

    #[test]
    fn test_direction_to_yaw_east() {
        let yaw = Direction::East.direction_to_yaw_radians();
        let expected = std::f32::consts::FRAC_PI_2;
        assert!(
            (yaw - expected).abs() < 1e-6,
            "East should map to π/2 radians, got {}",
            yaw
        );
    }

    #[test]
    fn test_direction_to_yaw_south() {
        let yaw = Direction::South.direction_to_yaw_radians();
        let expected = std::f32::consts::PI;
        assert!(
            (yaw - expected).abs() < 1e-6,
            "South should map to π radians, got {}",
            yaw
        );
    }

    #[test]
    fn test_direction_to_yaw_west() {
        let yaw = Direction::West.direction_to_yaw_radians();
        let expected = 3.0 * std::f32::consts::FRAC_PI_2;
        assert!(
            (yaw - expected).abs() < 1e-6,
            "West should map to 3π/2 radians, got {}",
            yaw
        );
    }

    #[test]
    fn test_direction_roundtrip() {
        // Every cardinal direction must survive a round-trip through yaw conversion
        let directions = [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ];
        for dir in &directions {
            let yaw = dir.direction_to_yaw_radians();
            let recovered = Direction::from_yaw_radians(yaw);
            assert_eq!(
                recovered, *dir,
                "Round-trip failed for {:?}: yaw={}, recovered={:?}",
                dir, yaw, recovered
            );
        }
    }

    #[test]
    fn test_direction_from_yaw_cardinals() {
        use std::f32::consts::{FRAC_PI_2, PI};
        assert_eq!(Direction::from_yaw_radians(0.0), Direction::North);
        assert_eq!(Direction::from_yaw_radians(FRAC_PI_2), Direction::East);
        assert_eq!(Direction::from_yaw_radians(PI), Direction::South);
        assert_eq!(
            Direction::from_yaw_radians(3.0 * FRAC_PI_2),
            Direction::West
        );
    }

    #[test]
    fn test_direction_from_yaw_snaps_to_nearest() {
        use std::f32::consts::FRAC_PI_2;
        // Just below the North/East boundary → still North
        assert_eq!(
            Direction::from_yaw_radians(FRAC_PI_2 / 2.0 - 0.01),
            Direction::North
        );
        // Just above the North/East boundary → East
        assert_eq!(
            Direction::from_yaw_radians(FRAC_PI_2 / 2.0 + 0.01),
            Direction::East
        );
    }

    #[test]
    fn test_direction_from_yaw_normalises_negative() {
        // -π/2 is equivalent to 3π/2 (West)
        assert_eq!(
            Direction::from_yaw_radians(-std::f32::consts::FRAC_PI_2),
            Direction::West
        );
    }

    #[test]
    fn test_direction_from_yaw_normalises_above_two_pi() {
        // 2π + π/2 should behave the same as π/2 (East)
        assert_eq!(
            Direction::from_yaw_radians(std::f32::consts::TAU + std::f32::consts::FRAC_PI_2),
            Direction::East
        );
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
        assert_eq!(time.year, 1);
        assert_eq!(time.month, 1);
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
        // 20:00 (Evening) → night
        let time = GameTime::new(1, 20, 0);
        assert!(time.is_night());
        assert!(!time.is_day());

        // 3:00 (Night) → night
        let time = GameTime::new(1, 3, 0);
        assert!(time.is_night());

        // 12:00 (Afternoon) → day
        let time = GameTime::new(1, 12, 0);
        assert!(!time.is_night());
        assert!(time.is_day());
    }

    // ===== GameTime Calendar / Rollover Tests =====

    #[test]
    fn test_new_full_constructor() {
        let time = GameTime::new_full(3, 11, 20, 14, 45);
        assert_eq!(time.year, 3);
        assert_eq!(time.month, 11);
        assert_eq!(time.day, 20);
        assert_eq!(time.hour, 14);
        assert_eq!(time.minute, 45);
    }

    #[test]
    fn test_advance_minutes_day_to_month_rollover() {
        // Start at Month 1, Day 30, 23:59 — one minute rolls into Month 2
        let mut time = GameTime::new_full(1, 1, 30, 23, 59);
        time.advance_minutes(1);
        assert_eq!(time.year, 1);
        assert_eq!(time.month, 2);
        assert_eq!(time.day, 1);
        assert_eq!(time.hour, 0);
        assert_eq!(time.minute, 0);
    }

    #[test]
    fn test_advance_minutes_month_to_year_rollover() {
        // Start at Month 12, Day 30, 23:00 — advance 120 minutes → crosses midnight → Year 2
        let mut time = GameTime::new_full(1, 12, 30, 23, 0);
        time.advance_minutes(120);
        assert_eq!(time.year, 2);
        assert_eq!(time.month, 1);
        assert_eq!(time.day, 1);
        assert_eq!(time.hour, 1);
        assert_eq!(time.minute, 0);
    }

    #[test]
    fn test_advance_minutes_multi_year_rollover() {
        // Start at Year 1, Month 1, Day 1, 00:00
        // Advance 2 full years (2 × 360 × 24 × 60 minutes)
        let mut time = GameTime::new_full(1, 1, 1, 0, 0);
        let two_years_in_minutes = 2 * DAYS_PER_YEAR * 24 * 60;
        time.advance_minutes(two_years_in_minutes);
        assert_eq!(time.year, 3);
        assert_eq!(time.month, 1);
        assert_eq!(time.day, 1);
        assert_eq!(time.hour, 0);
        assert_eq!(time.minute, 0);
    }

    #[test]
    fn test_advance_days_with_month_rollover() {
        // Advancing 31 days from Month 1, Day 1 → Month 2, Day 2
        let mut time = GameTime::new_full(1, 1, 1, 0, 0);
        time.advance_days(31);
        assert_eq!(time.year, 1);
        assert_eq!(time.month, 2);
        assert_eq!(time.day, 2);
        assert_eq!(time.hour, 0);
        assert_eq!(time.minute, 0);
    }

    #[test]
    fn test_advance_days_exact_month_boundary() {
        // Advancing 30 days from Month 1, Day 1 → Month 2, Day 1
        let mut time = GameTime::new_full(1, 1, 1, 6, 0);
        time.advance_days(30);
        assert_eq!(time.year, 1);
        assert_eq!(time.month, 2);
        assert_eq!(time.day, 1);
        assert_eq!(time.hour, 6);
    }

    #[test]
    fn test_advance_days_year_rollover() {
        // Advancing 360 days from start lands on Year 2 Month 1 Day 1
        let mut time = GameTime::new_full(1, 1, 1, 0, 0);
        time.advance_days(360);
        assert_eq!(time.year, 2);
        assert_eq!(time.month, 1);
        assert_eq!(time.day, 1);
    }

    #[test]
    fn test_serde_default_year_month() {
        // RON data without year/month fields should deserialize with year=1, month=1
        let ron_str = "(day: 5, hour: 8, minute: 0)";
        let time: GameTime = ron::from_str(ron_str).expect("RON deserialize failed");
        assert_eq!(time.year, 1, "year should default to 1");
        assert_eq!(time.month, 1, "month should default to 1");
        assert_eq!(time.day, 5);
        assert_eq!(time.hour, 8);
        assert_eq!(time.minute, 0);
    }

    #[test]
    fn test_total_days_basic() {
        // Year 1, Month 1, Day 1 → total 1
        assert_eq!(GameTime::new_full(1, 1, 1, 0, 0).total_days(), 1);

        // Year 1, Month 2, Day 10 → (0 * 360) + (1 * 30) + 10 = 40
        assert_eq!(GameTime::new_full(1, 2, 10, 0, 0).total_days(), 40);

        // Year 2, Month 1, Day 1 → (1 * 360) + (0 * 30) + 1 = 361
        assert_eq!(GameTime::new_full(2, 1, 1, 0, 0).total_days(), 361);
    }

    #[test]
    fn test_total_days_adventure_span() {
        // Month 2 Day 10 to Month 3 Day 12 = 32 days elapsed
        let start = GameTime::new_full(1, 2, 10, 0, 0);
        let end = GameTime::new_full(1, 3, 12, 0, 0);
        assert_eq!(end.total_days() - start.total_days(), 32);
    }

    #[test]
    fn test_total_days_year_boundary() {
        // Last day of year 1 → 360
        assert_eq!(GameTime::new_full(1, 12, 30, 0, 0).total_days(), 360);
        // First day of year 2 → 361
        assert_eq!(GameTime::new_full(2, 1, 1, 0, 0).total_days(), 361);
    }

    #[test]
    fn test_new_defaults_year_and_month() {
        // GameTime::new() must set year=1 and month=1 for backward compat
        let time = GameTime::new(15, 8, 30);
        assert_eq!(time.year, 1);
        assert_eq!(time.month, 1);
        assert_eq!(time.day, 15);
    }
}
