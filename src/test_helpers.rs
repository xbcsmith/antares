// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared test helpers for constructing game entities.
//!
//! All functions in this module are gated behind `#[cfg(test)]` and provide
//! convenient factories for [`Character`] and related types used across many
//! test modules.
//!
//! # Usage
//!
//! Import the factory you need in your test module:
//!
//! ```rust,ignore
//! use crate::test_helpers::factories::test_character;
//!
//! let hero = test_character("Hero");
//! ```

#[cfg(test)]
pub mod factories {
    use crate::domain::character::{Alignment, Character, Sex};

    /// Creates a basic test character with the given name.
    ///
    /// Uses `"human"` race, `"knight"` class, [`Sex::Male`], [`Alignment::Good`].
    ///
    /// # Arguments
    ///
    /// * `name` - The display name for the character
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::test_helpers::factories::test_character;
    ///
    /// let hero = test_character("Sir Lancelot");
    /// assert_eq!(hero.name, "Sir Lancelot");
    /// assert_eq!(hero.race_id, "human");
    /// assert_eq!(hero.class_id, "knight");
    /// ```
    pub fn test_character(name: &str) -> Character {
        Character::new(
            name.to_string(),
            "human".to_string(),
            "knight".to_string(),
            Sex::Male,
            Alignment::Good,
        )
    }

    /// Creates a test character with the specified class.
    ///
    /// Uses `"human"` race, [`Sex::Male`], [`Alignment::Good`].
    ///
    /// # Arguments
    ///
    /// * `name` - The display name for the character
    /// * `class_id` - The class identifier (e.g. `"knight"`, `"cleric"`, `"sorcerer"`)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::test_helpers::factories::test_character_with_class;
    ///
    /// let mage = test_character_with_class("Gandalf", "sorcerer");
    /// assert_eq!(mage.class_id, "sorcerer");
    /// ```
    pub fn test_character_with_class(name: &str, class_id: &str) -> Character {
        Character::new(
            name.to_string(),
            "human".to_string(),
            class_id.to_string(),
            Sex::Male,
            Alignment::Good,
        )
    }

    /// Creates a test character with specified race and class.
    ///
    /// Uses [`Sex::Male`] and [`Alignment::Good`].
    ///
    /// # Arguments
    ///
    /// * `name` - The display name for the character
    /// * `race_id` - The race identifier (e.g. `"human"`, `"elf"`, `"gnome"`)
    /// * `class_id` - The class identifier (e.g. `"knight"`, `"cleric"`, `"mage"`)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::test_helpers::factories::test_character_with_race_class;
    ///
    /// let elf_mage = test_character_with_race_class("Elrond", "elf", "mage");
    /// assert_eq!(elf_mage.race_id, "elf");
    /// assert_eq!(elf_mage.class_id, "mage");
    /// ```
    pub fn test_character_with_race_class(name: &str, race_id: &str, class_id: &str) -> Character {
        Character::new(
            name.to_string(),
            race_id.to_string(),
            class_id.to_string(),
            Sex::Male,
            Alignment::Good,
        )
    }

    /// Creates a dead test character (HP set to 0).
    ///
    /// Useful for testing death-related logic such as resurrection,
    /// experience denial, and condition checks.
    ///
    /// # Arguments
    ///
    /// * `name` - The display name for the character
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::test_helpers::factories::test_dead_character;
    ///
    /// let corpse = test_dead_character("Fallen Hero");
    /// assert_eq!(corpse.hp.current, 0);
    /// ```
    pub fn test_dead_character(name: &str) -> Character {
        let mut c = test_character(name);
        c.hp.current = 0;
        c
    }
}
