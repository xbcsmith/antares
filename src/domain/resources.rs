// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Resource management system - Food, light, and rest mechanics
//!
//! This module implements party-wide resource management including food
//! consumption, light tracking, and rest/recovery mechanics.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 5.4 for complete specifications.

use crate::domain::character::Party;
use crate::domain::types::GameTime;
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur during resource management
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ResourceError {
    #[error("Party has no food remaining")]
    NoFoodRemaining,

    #[error("Party has no light remaining")]
    NoLightRemaining,

    #[error("Cannot rest - party is in combat")]
    CannotRestInCombat,

    #[error("Party is too hungry to rest")]
    TooHungryToRest,
}

// ===== Constants =====

/// Food consumption per rest period (8 hours)
pub const FOOD_PER_REST: u32 = 1;

/// Food consumption per day of travel
pub const FOOD_PER_DAY: u32 = 3;

/// Light consumption per hour in dark areas
pub const LIGHT_PER_HOUR: u32 = 1;

/// HP restored per hour of rest (percentage of max)
pub const HP_RESTORE_RATE: f32 = 0.125; // 12.5% per hour, full in 8 hours

/// SP restored per hour of rest (percentage of max)
pub const SP_RESTORE_RATE: f32 = 0.125; // 12.5% per hour, full in 8 hours

/// Hours for a full rest
pub const REST_DURATION_HOURS: u32 = 8;

// ===== Food Management =====

/// Consumes food for the party
///
/// Each party member consumes food. If there isn't enough food for
/// everyone, consumes what's available and returns an error.
///
/// # Arguments
///
/// * `party` - The party consuming food
/// * `amount_per_member` - Amount of food each member consumes
///
/// # Returns
///
/// Returns `Ok(total_consumed)` with the amount consumed, or an error
/// if there wasn't enough food.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Party, Character, Class, Race, Sex, Alignment};
/// use antares::domain::resources::consume_food;
///
/// let mut party = Party::new();
/// let character = Character::new(
///     "Hero".to_string(),
///     Race::Human,
///     Class::Knight,
///     Sex::Male,
///     Alignment::Good,
/// );
/// party.add_member(character).unwrap();
/// party.food = 10;
///
/// let consumed = consume_food(&mut party, 1).unwrap();
/// assert_eq!(consumed, 1);
/// assert_eq!(party.food, 9);
/// ```
pub fn consume_food(party: &mut Party, amount_per_member: u32) -> Result<u32, ResourceError> {
    let total_needed = amount_per_member * party.members.len() as u32;

    if party.food < total_needed {
        // Consume what we have and return error
        let _consumed = party.food;
        party.food = 0;
        return Err(ResourceError::NoFoodRemaining);
    }

    party.food = party.food.saturating_sub(total_needed);
    Ok(total_needed)
}

/// Checks if the party is starving
///
/// Returns true if the party has no food remaining.
///
/// # Examples
///
/// ```
/// use antares::domain::character::Party;
/// use antares::domain::resources::check_starvation;
///
/// let mut party = Party::new();
/// party.food = 0;
///
/// assert!(check_starvation(&party));
///
/// party.food = 5;
/// assert!(!check_starvation(&party));
/// ```
pub fn check_starvation(party: &Party) -> bool {
    party.food == 0
}

// ===== Light Management =====

/// Consumes light for the party
///
/// Decrements the party's light counter. Used when traveling through
/// dark areas like dungeons.
///
/// # Arguments
///
/// * `party` - The party consuming light
/// * `amount` - Amount of light to consume
///
/// # Returns
///
/// Returns `Ok(amount)` if successful, or an error if there's no light.
///
/// # Examples
///
/// ```
/// use antares::domain::character::Party;
/// use antares::domain::resources::consume_light;
///
/// let mut party = Party::new();
/// party.light_units = 100;
///
/// consume_light(&mut party, 10).unwrap();
/// assert_eq!(party.light_units, 90);
/// ```
pub fn consume_light(party: &mut Party, amount: u32) -> Result<u32, ResourceError> {
    if party.light_units == 0 {
        return Err(ResourceError::NoLightRemaining);
    }

    let consumed = amount.min(party.light_units as u32);
    party.light_units = party.light_units.saturating_sub(consumed as u8);
    Ok(consumed)
}

/// Checks if the party is in darkness
///
/// Returns true if the party's light has run out.
///
/// # Examples
///
/// ```
/// use antares::domain::character::Party;
/// use antares::domain::resources::is_dark;
///
/// let mut party = Party::new();
/// party.light_units = 0;
///
/// assert!(is_dark(&party));
///
/// party.light_units = 10;
/// assert!(!is_dark(&party));
/// ```
pub fn is_dark(party: &Party) -> bool {
    party.light_units == 0
}

// ===== Rest and Recovery =====

/// Rests the party, restoring HP and SP
///
/// The party rests for the specified number of hours. During rest:
/// - HP and SP are gradually restored
/// - Food is consumed
/// - Time advances
///
/// # Arguments
///
/// * `party` - The party resting (will be modified)
/// * `game_time` - Current game time (will be advanced)
/// * `hours` - Number of hours to rest
///
/// # Returns
///
/// Returns `Ok(())` if successful, or an error if the party cannot rest.
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Party, Character, Class, Race, Sex, Alignment};
/// use antares::domain::resources::rest_party;
/// use antares::domain::types::GameTime;
///
/// let mut party = Party::new();
/// let mut character = Character::new(
///     "Hero".to_string(),
///     Race::Human,
///     Class::Knight,
///     Sex::Male,
///     Alignment::Good,
/// );
/// character.hp.current = 5;
/// character.hp.base = 20;
/// party.add_member(character).unwrap();
/// party.food = 10;
///
/// let mut time = GameTime::new(1, 0, 0);
/// rest_party(&mut party, &mut time, 8).unwrap();
///
/// assert_eq!(party.members[0].hp.current, 20); // Fully healed
/// assert_eq!(time.hour, 8); // Time advanced
/// ```
pub fn rest_party(
    party: &mut Party,
    game_time: &mut GameTime,
    hours: u32,
) -> Result<(), ResourceError> {
    // Check if party has food
    if party.food == 0 {
        return Err(ResourceError::TooHungryToRest);
    }

    // Consume food (1 per full 8-hour rest)
    let food_needed = (hours as f32 / REST_DURATION_HOURS as f32).ceil() as u32;
    if food_needed > 0 {
        let _ = consume_food(party, food_needed); // Consume what we can
    }

    // Restore HP and SP for each party member
    for character in &mut party.members {
        // Skip dead/unconscious characters
        if character.conditions.is_fatal() || character.conditions.is_unconscious() {
            continue;
        }

        // Restore HP
        let hp_to_restore = (character.hp.base as f32 * HP_RESTORE_RATE * hours as f32) as u16;
        character.hp.current = (character.hp.current + hp_to_restore).min(character.hp.base);

        // Restore SP
        let sp_to_restore = (character.sp.base as f32 * SP_RESTORE_RATE * hours as f32) as u16;
        character.sp.current = (character.sp.current + sp_to_restore).min(character.sp.base);
    }

    // Advance time
    game_time.advance_hours(hours);

    Ok(())
}

/// Applies starvation damage to the party
///
/// When the party is out of food, each member takes damage over time.
/// This should be called periodically (e.g., every hour or day).
///
/// # Arguments
///
/// * `party` - The party suffering from starvation
/// * `damage_per_member` - HP damage to apply to each member
///
/// # Examples
///
/// ```
/// use antares::domain::character::{Party, Character, Class, Race, Sex, Alignment};
/// use antares::domain::resources::apply_starvation_damage;
///
/// let mut party = Party::new();
/// let mut character = Character::new(
///     "Hero".to_string(),
///     Race::Human,
///     Class::Knight,
///     Sex::Male,
///     Alignment::Good,
/// );
/// character.hp.current = 20;
/// party.add_member(character).unwrap();
///
/// apply_starvation_damage(&mut party, 5);
/// assert_eq!(party.members[0].hp.current, 15);
/// ```
pub fn apply_starvation_damage(party: &mut Party, damage_per_member: u16) {
    for character in &mut party.members {
        // Skip already dead characters
        if character.conditions.is_fatal() {
            continue;
        }

        character.hp.current = character.hp.current.saturating_sub(damage_per_member);

        // If HP drops to 0, character dies
        if character.hp.current == 0 {
            character
                .conditions
                .add(crate::domain::character::Condition::DEAD);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, Character, Class, Race, Sex};

    fn create_test_party() -> Party {
        let mut party = Party::new();
        for i in 0..3 {
            let mut character = Character::new(
                format!("Hero{}", i),
                Race::Human,
                Class::Knight,
                Sex::Male,
                Alignment::Good,
            );
            character.hp.base = 20;
            character.hp.current = 20;
            character.sp.base = 10;
            character.sp.current = 10;
            party.add_member(character).unwrap();
        }
        party
    }

    #[test]
    fn test_consume_food() {
        let mut party = create_test_party();
        party.food = 10;

        let consumed = consume_food(&mut party, 1).unwrap();
        assert_eq!(consumed, 3); // 3 party members
        assert_eq!(party.food, 7);
    }

    #[test]
    fn test_consume_food_not_enough() {
        let mut party = create_test_party();
        party.food = 2; // Not enough for 3 members

        let result = consume_food(&mut party, 1);
        assert!(matches!(result, Err(ResourceError::NoFoodRemaining)));
        assert_eq!(party.food, 0);
    }

    #[test]
    fn test_check_starvation() {
        let mut party = create_test_party();
        party.food = 0;

        assert!(check_starvation(&party));

        party.food = 5;
        assert!(!check_starvation(&party));
    }

    #[test]
    fn test_consume_light() {
        let mut party = create_test_party();
        party.light_units = 100;

        consume_light(&mut party, 10).unwrap();
        assert_eq!(party.light_units, 90);

        consume_light(&mut party, 50).unwrap();
        assert_eq!(party.light_units, 40);
    }

    #[test]
    fn test_consume_light_when_dark() {
        let mut party = create_test_party();
        party.light_units = 0;

        let result = consume_light(&mut party, 10);
        assert!(matches!(result, Err(ResourceError::NoLightRemaining)));
    }

    #[test]
    fn test_is_dark() {
        let mut party = create_test_party();
        party.light_units = 0;

        assert!(is_dark(&party));

        party.light_units = 1;
        assert!(!is_dark(&party));
    }

    #[test]
    fn test_rest_restores_hp() {
        let mut party = create_test_party();
        party.members[0].hp.current = 10; // Half HP
        party.food = 10;

        let mut time = GameTime::new(1, 0, 0);
        rest_party(&mut party, &mut time, 8).unwrap();

        // After 8 hours, should be fully healed
        assert_eq!(party.members[0].hp.current, 20);
    }

    #[test]
    fn test_rest_restores_sp() {
        let mut party = create_test_party();
        party.members[0].sp.current = 5; // Half SP
        party.food = 10;

        let mut time = GameTime::new(1, 0, 0);
        rest_party(&mut party, &mut time, 8).unwrap();

        // After 8 hours, should be fully restored
        assert_eq!(party.members[0].sp.current, 10);
    }

    #[test]
    fn test_rest_advances_time() {
        let mut party = create_test_party();
        party.food = 10;

        let mut time = GameTime::new(1, 6, 0);
        rest_party(&mut party, &mut time, 8).unwrap();

        assert_eq!(time.day, 1);
        assert_eq!(time.hour, 14);
    }

    #[test]
    fn test_rest_consumes_food() {
        let mut party = create_test_party();
        party.food = 10;

        let mut time = GameTime::new(1, 0, 0);
        rest_party(&mut party, &mut time, 8).unwrap();

        // 3 party members * 1 food = 3 food consumed
        assert_eq!(party.food, 7);
    }

    #[test]
    fn test_rest_without_food() {
        let mut party = create_test_party();
        party.food = 0;

        let mut time = GameTime::new(1, 0, 0);
        let result = rest_party(&mut party, &mut time, 8);

        assert!(matches!(result, Err(ResourceError::TooHungryToRest)));
    }

    #[test]
    fn test_rest_partial_hours() {
        let mut party = create_test_party();
        party.members[0].hp.current = 10; // Half HP
        party.food = 10;

        let mut time = GameTime::new(1, 0, 0);
        rest_party(&mut party, &mut time, 4).unwrap(); // 4 hours rest

        // 4 hours * 12.5% per hour = 50% restoration = 10 HP restored
        // Starting at 10 HP, should be fully healed (10 + 10 = 20)
        assert_eq!(party.members[0].hp.current, 20);
    }

    #[test]
    fn test_apply_starvation_damage() {
        let mut party = create_test_party();
        party.members[0].hp.current = 20;

        apply_starvation_damage(&mut party, 5);
        assert_eq!(party.members[0].hp.current, 15);

        apply_starvation_damage(&mut party, 5);
        assert_eq!(party.members[0].hp.current, 10);
    }

    #[test]
    fn test_starvation_kills_character() {
        let mut party = create_test_party();
        party.members[0].hp.current = 3;

        apply_starvation_damage(&mut party, 5);

        assert_eq!(party.members[0].hp.current, 0);
        assert!(party.members[0].conditions.is_fatal());
    }

    #[test]
    fn test_dead_character_skipped_in_rest() {
        let mut party = create_test_party();
        party.members[0].hp.current = 0;
        party.members[0]
            .conditions
            .add(crate::domain::character::Condition::DEAD);
        party.food = 10;

        let initial_hp = party.members[0].hp.current;
        let mut time = GameTime::new(1, 0, 0);
        rest_party(&mut party, &mut time, 8).unwrap();

        // Dead character should not heal
        assert_eq!(party.members[0].hp.current, initial_hp);
    }
}
