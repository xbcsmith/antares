// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Party management operations for recruiting, dismissing, and swapping characters
//!
//! This module provides centralized party management logic that maintains consistency
//! between the active party and the character roster. All operations ensure that
//! character locations are properly tracked and party size constraints are enforced.
//!
//! # Core Operations
//!
//! - `recruit_to_party`: Move character from roster (inn/map) to active party
//! - `dismiss_to_inn`: Move character from party to inn storage
//! - `swap_party_member`: Atomically swap party member with roster character
//!
//! # Examples
//!
//! ```
//! use antares::domain::party_manager::PartyManager;
//! use antares::domain::character::{Party, Roster, Character, CharacterLocation, Sex, Alignment, Stats};
//!
//! let mut party = Party::new();
//! let mut roster = Roster::new();
//!
//! // Add a character to the roster at an inn
//! let char1 = Character::new("Hero", 1, 1, Sex::Male, Alignment::Good, Stats::new(10, 10, 10, 10, 10, 10, 10));
//! roster.add_character(char1, CharacterLocation::AtInn(1)).unwrap();
//!
//! // Recruit to party
//! PartyManager::recruit_to_party(&mut party, &mut roster, 0).unwrap();
//! assert_eq!(party.size(), 1);
//! ```

use crate::domain::character::{Character, CharacterError, CharacterLocation, Party, Roster};
use crate::domain::types::TownId;
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur during party management operations
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PartyManagementError {
    #[error("Party is full (max {0} members)")]
    PartyFull(usize),

    #[error("Party must have at least 1 member")]
    PartyEmpty,

    #[error("Character not found in roster at index {0}")]
    CharacterNotFound(usize),

    #[error("Character is already in party")]
    AlreadyInParty,

    #[error("Character is not at current inn (location: {0:?})")]
    NotAtInn(CharacterLocation),

    #[error("Invalid party index {0} (party size: {1})")]
    InvalidPartyIndex(usize, usize),

    #[error("Invalid roster index {0} (roster size: {1})")]
    InvalidRosterIndex(usize, usize),

    #[error("Character error: {0}")]
    CharacterError(#[from] CharacterError),
}

// ===== Party Manager =====

/// Central manager for party operations
///
/// All party management operations go through this struct to ensure
/// consistency between party state and roster location tracking.
pub struct PartyManager;

impl PartyManager {
    /// Moves character from roster to active party
    ///
    /// Verifies that the party is not full and that the character is not already
    /// in the party. The character must be located at an inn or on a map to be recruited.
    ///
    /// # Arguments
    ///
    /// * `party` - Mutable reference to the active party
    /// * `roster` - Mutable reference to the character roster
    /// * `roster_index` - Index of character in roster to recruit
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if recruitment succeeds, otherwise returns an error explaining
    /// why the operation failed.
    ///
    /// # Errors
    ///
    /// - `PartyManagementError::PartyFull` if party is at maximum size
    /// - `PartyManagementError::CharacterNotFound` if roster_index is invalid
    /// - `PartyManagementError::AlreadyInParty` if character is already in party
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::party_manager::PartyManager;
    /// use antares::domain::character::{Party, Roster, Character, CharacterLocation, Sex, Alignment, Stats};
    ///
    /// let mut party = Party::new();
    /// let mut roster = Roster::new();
    ///
    /// let char1 = Character::new("Warrior", 1, 1, Sex::Male, Alignment::Good, Stats::new(15, 10, 10, 10, 10, 10, 10));
    /// roster.add_character(char1, CharacterLocation::AtInn(1)).unwrap();
    ///
    /// // Recruit the warrior
    /// let result = PartyManager::recruit_to_party(&mut party, &mut roster, 0);
    /// assert!(result.is_ok());
    /// assert_eq!(party.size(), 1);
    /// ```
    pub fn recruit_to_party(
        party: &mut Party,
        roster: &mut Roster,
        roster_index: usize,
    ) -> Result<(), PartyManagementError> {
        // Validate party is not full
        if party.is_full() {
            return Err(PartyManagementError::PartyFull(Party::MAX_MEMBERS));
        }

        // Validate roster index
        if roster_index >= roster.characters.len() {
            return Err(PartyManagementError::InvalidRosterIndex(
                roster_index,
                roster.characters.len(),
            ));
        }

        // Check character location - must not already be in party
        let location = roster.character_locations[roster_index];
        if location == CharacterLocation::InParty {
            return Err(PartyManagementError::AlreadyInParty);
        }

        // Clone character from roster and add to party
        let character = roster.characters[roster_index].clone();
        party.add_member(character)?;

        // Update roster location to InParty
        roster.update_location(roster_index, CharacterLocation::InParty)?;

        Ok(())
    }

    /// Moves character from party to inn storage
    ///
    /// Removes a character from the active party and places them at the specified inn.
    /// Enforces that the party must have at least one member remaining.
    ///
    /// # Arguments
    ///
    /// * `party` - Mutable reference to the active party
    /// * `roster` - Mutable reference to the character roster
    /// * `party_index` - Index of character in party to dismiss (0-5)
    /// * `inn_id` - Town/inn ID where character will be stored
    ///
    /// # Returns
    ///
    /// Returns the dismissed character if successful, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// - `PartyManagementError::PartyEmpty` if dismissing would leave party empty
    /// - `PartyManagementError::InvalidPartyIndex` if party_index is out of bounds
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::party_manager::PartyManager;
    /// use antares::domain::character::{Party, Roster, Character, CharacterLocation, Sex, Alignment, Stats};
    ///
    /// let mut party = Party::new();
    /// let mut roster = Roster::new();
    ///
    /// // Add two characters to roster and recruit both
    /// let char1 = Character::new("Warrior", 1, 1, Sex::Male, Alignment::Good, Stats::new(15, 10, 10, 10, 10, 10, 10));
    /// let char2 = Character::new("Mage", 2, 2, Sex::Female, Alignment::Good, Stats::new(8, 16, 10, 10, 10, 10, 10));
    /// roster.add_character(char1, CharacterLocation::AtInn(1)).unwrap();
    /// roster.add_character(char2, CharacterLocation::AtInn(1)).unwrap();
    ///
    /// PartyManager::recruit_to_party(&mut party, &mut roster, 0).unwrap();
    /// PartyManager::recruit_to_party(&mut party, &mut roster, 1).unwrap();
    ///
    /// // Dismiss the first character to inn 2
    /// let dismissed = PartyManager::dismiss_to_inn(&mut party, &mut roster, 0, 2).unwrap();
    /// assert_eq!(dismissed.name, "Warrior");
    /// assert_eq!(party.size(), 1);
    /// ```
    pub fn dismiss_to_inn(
        party: &mut Party,
        roster: &mut Roster,
        party_index: usize,
        inn_id: TownId,
    ) -> Result<Character, PartyManagementError> {
        // Enforce minimum party size
        if party.size() <= 1 {
            return Err(PartyManagementError::PartyEmpty);
        }

        // Validate party index
        if party_index >= party.size() {
            return Err(PartyManagementError::InvalidPartyIndex(
                party_index,
                party.size(),
            ));
        }

        // Find the character in the roster by matching the party member
        // In the current implementation, we need to find which roster index
        // corresponds to this party member by checking InParty locations
        let party_chars_in_roster: Vec<usize> = roster
            .character_locations
            .iter()
            .enumerate()
            .filter_map(|(idx, loc)| {
                if *loc == CharacterLocation::InParty {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();

        // The party_index corresponds to the nth character marked InParty
        if party_index >= party_chars_in_roster.len() {
            return Err(PartyManagementError::InvalidPartyIndex(
                party_index,
                party.size(),
            ));
        }

        let roster_index = party_chars_in_roster[party_index];

        // Remove from party
        let character =
            party
                .remove_member(party_index)
                .ok_or(PartyManagementError::InvalidPartyIndex(
                    party_index,
                    party.size(),
                ))?;

        // Update roster location to AtInn
        roster.update_location(roster_index, CharacterLocation::AtInn(inn_id))?;

        Ok(character)
    }

    /// Atomically swaps a party member with a roster character
    ///
    /// This operation removes one character from the party and adds another in a single
    /// atomic operation. This ensures the party never becomes empty during the swap.
    ///
    /// # Arguments
    ///
    /// * `party` - Mutable reference to the active party
    /// * `roster` - Mutable reference to the character roster
    /// * `party_index` - Index of character in party to remove (0-5)
    /// * `roster_index` - Index of character in roster to add
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if swap succeeds, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// - `PartyManagementError::InvalidPartyIndex` if party_index is out of bounds
    /// - `PartyManagementError::InvalidRosterIndex` if roster_index is out of bounds
    /// - `PartyManagementError::AlreadyInParty` if roster character is already in party
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::party_manager::PartyManager;
    /// use antares::domain::character::{Party, Roster, Character, CharacterLocation, Sex, Alignment, Stats};
    ///
    /// let mut party = Party::new();
    /// let mut roster = Roster::new();
    ///
    /// // Add characters to roster
    /// let char1 = Character::new("Warrior", 1, 1, Sex::Male, Alignment::Good, Stats::new(15, 10, 10, 10, 10, 10, 10));
    /// let char2 = Character::new("Mage", 2, 2, Sex::Female, Alignment::Good, Stats::new(8, 16, 10, 10, 10, 10, 10));
    /// roster.add_character(char1, CharacterLocation::AtInn(1)).unwrap();
    /// roster.add_character(char2, CharacterLocation::AtInn(1)).unwrap();
    ///
    /// // Recruit warrior
    /// PartyManager::recruit_to_party(&mut party, &mut roster, 0).unwrap();
    ///
    /// // Swap warrior with mage
    /// PartyManager::swap_party_member(&mut party, &mut roster, 0, 1).unwrap();
    /// assert_eq!(party.members[0].name, "Mage");
    /// ```
    pub fn swap_party_member(
        party: &mut Party,
        roster: &mut Roster,
        party_index: usize,
        roster_index: usize,
    ) -> Result<(), PartyManagementError> {
        // Validate indices
        if party_index >= party.size() {
            return Err(PartyManagementError::InvalidPartyIndex(
                party_index,
                party.size(),
            ));
        }

        if roster_index >= roster.characters.len() {
            return Err(PartyManagementError::InvalidRosterIndex(
                roster_index,
                roster.characters.len(),
            ));
        }

        // Check that roster character is not already in party
        let roster_location = roster.character_locations[roster_index];
        if roster_location == CharacterLocation::InParty {
            return Err(PartyManagementError::AlreadyInParty);
        }

        // Find the roster index of the party member being swapped out
        let party_chars_in_roster: Vec<usize> = roster
            .character_locations
            .iter()
            .enumerate()
            .filter_map(|(idx, loc)| {
                if *loc == CharacterLocation::InParty {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();

        if party_index >= party_chars_in_roster.len() {
            return Err(PartyManagementError::InvalidPartyIndex(
                party_index,
                party.size(),
            ));
        }

        let party_member_roster_index = party_chars_in_roster[party_index];

        // Store the location where the removed party member will go
        // (preserve their previous location if it was an inn, otherwise use inn 1)
        let dismissed_location = match roster_location {
            CharacterLocation::AtInn(inn_id) => CharacterLocation::AtInn(inn_id),
            CharacterLocation::OnMap(map_id) => CharacterLocation::OnMap(map_id),
            CharacterLocation::InParty => CharacterLocation::AtInn(1), // Fallback (shouldn't happen)
        };

        // Perform atomic swap:
        // 1. Clone new character from roster
        let new_character = roster.characters[roster_index].clone();

        // 2. Replace party member (swap in place)
        party.members[party_index] = new_character;

        // 3. Update locations in roster
        roster.update_location(party_member_roster_index, dismissed_location)?;
        roster.update_location(roster_index, CharacterLocation::InParty)?;

        Ok(())
    }

    /// Validates whether a character can be recruited to the party
    ///
    /// Checks party size constraints and character location.
    ///
    /// # Arguments
    ///
    /// * `party` - Reference to the active party
    /// * `roster` - Reference to the character roster
    /// * `roster_index` - Index of character to validate
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if recruitment is valid, otherwise returns an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::party_manager::PartyManager;
    /// use antares::domain::character::{Party, Roster, Character, CharacterLocation, Sex, Alignment, Stats};
    ///
    /// let party = Party::new();
    /// let mut roster = Roster::new();
    ///
    /// let char1 = Character::new("Hero", 1, 1, Sex::Male, Alignment::Good, Stats::new(10, 10, 10, 10, 10, 10, 10));
    /// roster.add_character(char1, CharacterLocation::AtInn(1)).unwrap();
    ///
    /// let result = PartyManager::can_recruit(&party, &roster, 0);
    /// assert!(result.is_ok());
    /// ```
    pub fn can_recruit(
        party: &Party,
        roster: &Roster,
        roster_index: usize,
    ) -> Result<(), PartyManagementError> {
        // Check party size
        if party.is_full() {
            return Err(PartyManagementError::PartyFull(Party::MAX_MEMBERS));
        }

        // Check roster index validity
        if roster_index >= roster.characters.len() {
            return Err(PartyManagementError::InvalidRosterIndex(
                roster_index,
                roster.characters.len(),
            ));
        }

        // Check location
        let location = roster.character_locations[roster_index];
        if location == CharacterLocation::InParty {
            return Err(PartyManagementError::AlreadyInParty);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::character::{Alignment, Sex};

    fn create_test_character(name: &str, race_id: &str, class_id: &str) -> Character {
        Character::new(
            name.to_string(),
            race_id.to_string(),
            class_id.to_string(),
            Sex::Male,
            Alignment::Good,
        )
    }

    #[test]
    fn test_recruit_to_party_success() {
        let mut party = Party::new();
        let mut roster = Roster::new();

        let char1 = create_test_character("Warrior", "human", "knight");
        roster
            .add_character(char1, CharacterLocation::AtInn(1))
            .unwrap();

        let result = PartyManager::recruit_to_party(&mut party, &mut roster, 0);
        assert!(result.is_ok());
        assert_eq!(party.size(), 1);
        assert_eq!(party.members[0].name, "Warrior");
        assert_eq!(roster.character_locations[0], CharacterLocation::InParty);
    }

    #[test]
    fn test_recruit_to_party_when_full() {
        let mut party = Party::new();
        let mut roster = Roster::new();

        // Fill party to max
        for i in 0..Party::MAX_MEMBERS {
            let character = create_test_character(&format!("Char{}", i), "human", "knight");
            roster
                .add_character(character.clone(), CharacterLocation::AtInn(1))
                .unwrap();
            PartyManager::recruit_to_party(&mut party, &mut roster, i).unwrap();
        }

        // Add one more to roster
        let extra_char = create_test_character("Extra", "human", "knight");
        roster
            .add_character(extra_char, CharacterLocation::AtInn(1))
            .unwrap();

        // Try to recruit when party is full
        let result = PartyManager::recruit_to_party(&mut party, &mut roster, Party::MAX_MEMBERS);
        assert!(matches!(result, Err(PartyManagementError::PartyFull(6))));
    }

    #[test]
    fn test_recruit_already_in_party() {
        let mut party = Party::new();
        let mut roster = Roster::new();

        let char1 = create_test_character("Warrior", "human", "knight");
        roster
            .add_character(char1, CharacterLocation::AtInn(1))
            .unwrap();

        // Recruit once
        PartyManager::recruit_to_party(&mut party, &mut roster, 0).unwrap();

        // Try to recruit again
        let result = PartyManager::recruit_to_party(&mut party, &mut roster, 0);
        assert!(matches!(result, Err(PartyManagementError::AlreadyInParty)));
    }

    #[test]
    fn test_recruit_invalid_roster_index() {
        let mut party = Party::new();
        let mut roster = Roster::new();

        let result = PartyManager::recruit_to_party(&mut party, &mut roster, 0);
        assert!(matches!(
            result,
            Err(PartyManagementError::InvalidRosterIndex(0, 0))
        ));
    }

    #[test]
    fn test_dismiss_to_inn_success() {
        let mut party = Party::new();
        let mut roster = Roster::new();

        // Add two characters
        let char1 = create_test_character("Warrior", "human", "knight");
        let char2 = create_test_character("Mage", "elf", "mage");
        roster
            .add_character(char1, CharacterLocation::AtInn(1))
            .unwrap();
        roster
            .add_character(char2, CharacterLocation::AtInn(1))
            .unwrap();

        // Recruit both
        PartyManager::recruit_to_party(&mut party, &mut roster, 0).unwrap();
        PartyManager::recruit_to_party(&mut party, &mut roster, 1).unwrap();

        // Dismiss first to inn 2
        let result = PartyManager::dismiss_to_inn(&mut party, &mut roster, 0, 2);
        assert!(result.is_ok());
        let dismissed = result.unwrap();
        assert_eq!(dismissed.name, "Warrior");
        assert_eq!(party.size(), 1);
        assert_eq!(roster.character_locations[0], CharacterLocation::AtInn(2));
    }

    #[test]
    fn test_dismiss_last_member_fails() {
        let mut party = Party::new();
        let mut roster = Roster::new();

        let char1 = create_test_character("Warrior", "human", "knight");
        roster
            .add_character(char1, CharacterLocation::AtInn(1))
            .unwrap();

        PartyManager::recruit_to_party(&mut party, &mut roster, 0).unwrap();

        // Try to dismiss the only party member
        let result = PartyManager::dismiss_to_inn(&mut party, &mut roster, 0, 1);
        assert!(matches!(result, Err(PartyManagementError::PartyEmpty)));
    }

    #[test]
    fn test_dismiss_invalid_party_index() {
        let mut party = Party::new();
        let mut roster = Roster::new();

        let char1 = create_test_character("Warrior", "human", "knight");
        let char2 = create_test_character("Mage", "elf", "mage");
        roster
            .add_character(char1, CharacterLocation::AtInn(1))
            .unwrap();
        roster
            .add_character(char2, CharacterLocation::AtInn(1))
            .unwrap();

        PartyManager::recruit_to_party(&mut party, &mut roster, 0).unwrap();
        PartyManager::recruit_to_party(&mut party, &mut roster, 1).unwrap();

        // Try to dismiss index 5 when only 2 members
        let result = PartyManager::dismiss_to_inn(&mut party, &mut roster, 5, 1);
        assert!(matches!(
            result,
            Err(PartyManagementError::InvalidPartyIndex(5, 2))
        ));
    }

    #[test]
    fn test_swap_party_member_atomic() {
        let mut party = Party::new();
        let mut roster = Roster::new();

        // Add characters
        let char1 = create_test_character("Warrior", "human", "knight");
        let char2 = create_test_character("Mage", "elf", "mage");
        roster
            .add_character(char1, CharacterLocation::AtInn(1))
            .unwrap();
        roster
            .add_character(char2, CharacterLocation::AtInn(1))
            .unwrap();

        // Recruit warrior
        PartyManager::recruit_to_party(&mut party, &mut roster, 0).unwrap();
        assert_eq!(party.members[0].name, "Warrior");

        // Swap warrior with mage
        let result = PartyManager::swap_party_member(&mut party, &mut roster, 0, 1);
        assert!(result.is_ok());

        // Verify swap
        assert_eq!(party.size(), 1);
        assert_eq!(party.members[0].name, "Mage");
        assert_eq!(roster.character_locations[0], CharacterLocation::AtInn(1));
        assert_eq!(roster.character_locations[1], CharacterLocation::InParty);
    }

    #[test]
    fn test_swap_invalid_party_index() {
        let mut party = Party::new();
        let mut roster = Roster::new();

        let char1 = create_test_character("Warrior", "human", "knight");
        let char2 = create_test_character("Mage", "elf", "mage");
        roster
            .add_character(char1, CharacterLocation::AtInn(1))
            .unwrap();
        roster
            .add_character(char2, CharacterLocation::AtInn(1))
            .unwrap();

        PartyManager::recruit_to_party(&mut party, &mut roster, 0).unwrap();

        // Try to swap invalid party index
        let result = PartyManager::swap_party_member(&mut party, &mut roster, 5, 1);
        assert!(matches!(
            result,
            Err(PartyManagementError::InvalidPartyIndex(5, 1))
        ));
    }

    #[test]
    fn test_swap_invalid_roster_index() {
        let mut party = Party::new();
        let mut roster = Roster::new();

        let char1 = create_test_character("Warrior", "human", "knight");
        roster
            .add_character(char1, CharacterLocation::AtInn(1))
            .unwrap();

        PartyManager::recruit_to_party(&mut party, &mut roster, 0).unwrap();

        // Try to swap with invalid roster index
        let result = PartyManager::swap_party_member(&mut party, &mut roster, 0, 10);
        assert!(matches!(
            result,
            Err(PartyManagementError::InvalidRosterIndex(10, 1))
        ));
    }

    #[test]
    fn test_swap_already_in_party() {
        let mut party = Party::new();
        let mut roster = Roster::new();

        let char1 = create_test_character("Warrior", "human", "knight");
        let char2 = create_test_character("Mage", "elf", "mage");
        roster
            .add_character(char1, CharacterLocation::AtInn(1))
            .unwrap();
        roster
            .add_character(char2, CharacterLocation::AtInn(1))
            .unwrap();

        PartyManager::recruit_to_party(&mut party, &mut roster, 0).unwrap();
        PartyManager::recruit_to_party(&mut party, &mut roster, 1).unwrap();

        // Try to swap party member with another party member
        let result = PartyManager::swap_party_member(&mut party, &mut roster, 0, 1);
        assert!(matches!(result, Err(PartyManagementError::AlreadyInParty)));
    }

    #[test]
    fn test_location_tracking_consistency() {
        let mut party = Party::new();
        let mut roster = Roster::new();

        // Add multiple characters
        for i in 0..4 {
            let character = create_test_character(&format!("Char{}", i), "human", "knight");
            roster
                .add_character(character, CharacterLocation::AtInn(1))
                .unwrap();
        }

        // Recruit first two
        PartyManager::recruit_to_party(&mut party, &mut roster, 0).unwrap();
        PartyManager::recruit_to_party(&mut party, &mut roster, 1).unwrap();

        // Verify locations
        assert_eq!(roster.character_locations[0], CharacterLocation::InParty);
        assert_eq!(roster.character_locations[1], CharacterLocation::InParty);
        assert_eq!(roster.character_locations[2], CharacterLocation::AtInn(1));
        assert_eq!(roster.character_locations[3], CharacterLocation::AtInn(1));

        // Verify party count
        let party_count = roster
            .character_locations
            .iter()
            .filter(|loc| **loc == CharacterLocation::InParty)
            .count();
        assert_eq!(party_count, 2);
        assert_eq!(party.size(), 2);
    }

    #[test]
    fn test_can_recruit_validation() {
        let party = Party::new();
        let mut roster = Roster::new();

        let char1 = create_test_character("Warrior", "human", "knight");
        roster
            .add_character(char1, CharacterLocation::AtInn(1))
            .unwrap();

        // Should be able to recruit
        let result = PartyManager::can_recruit(&party, &roster, 0);
        assert!(result.is_ok());

        // Invalid index
        let result = PartyManager::can_recruit(&party, &roster, 10);
        assert!(matches!(
            result,
            Err(PartyManagementError::InvalidRosterIndex(10, 1))
        ));
    }

    #[test]
    fn test_can_recruit_party_full() {
        let mut party = Party::new();
        let mut roster = Roster::new();

        // Fill party
        for i in 0..Party::MAX_MEMBERS {
            let character = create_test_character(&format!("Char{}", i), "human", "knight");
            roster
                .add_character(character.clone(), CharacterLocation::AtInn(1))
                .unwrap();
            PartyManager::recruit_to_party(&mut party, &mut roster, i).unwrap();
        }

        // Add one more to roster
        let extra = create_test_character("Extra", "human", "knight");
        roster
            .add_character(extra, CharacterLocation::AtInn(1))
            .unwrap();

        // Should not be able to recruit
        let result = PartyManager::can_recruit(&party, &roster, Party::MAX_MEMBERS);
        assert!(matches!(result, Err(PartyManagementError::PartyFull(6))));
    }

    #[test]
    fn test_recruit_from_map_location() {
        let mut party = Party::new();
        let mut roster = Roster::new();

        let char1 = create_test_character("NPC", "human", "knight");
        roster
            .add_character(char1, CharacterLocation::OnMap(5))
            .unwrap();

        let result = PartyManager::recruit_to_party(&mut party, &mut roster, 0);
        assert!(result.is_ok());
        assert_eq!(party.size(), 1);
        assert_eq!(roster.character_locations[0], CharacterLocation::InParty);
    }

    #[test]
    fn test_swap_preserves_map_location() {
        let mut party = Party::new();
        let mut roster = Roster::new();

        // Party member at inn 1
        let char1 = create_test_character("Warrior", "human", "knight");
        roster
            .add_character(char1, CharacterLocation::AtInn(1))
            .unwrap();

        // NPC on map 5
        let char2 = create_test_character("NPC", "elf", "mage");
        roster
            .add_character(char2, CharacterLocation::OnMap(5))
            .unwrap();

        // Recruit warrior
        PartyManager::recruit_to_party(&mut party, &mut roster, 0).unwrap();

        // Swap warrior with NPC
        let result = PartyManager::swap_party_member(&mut party, &mut roster, 0, 1);
        assert!(result.is_ok());

        // Warrior should go to map 5 (where NPC was)
        assert_eq!(roster.character_locations[0], CharacterLocation::OnMap(5));
        // NPC should be in party
        assert_eq!(roster.character_locations[1], CharacterLocation::InParty);
    }
}
