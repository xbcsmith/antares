// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// Each binary only uses a subset of this shared module, so some items
// will appear unused when compiled for any single binary.
#![allow(dead_code)]

//! Shared constants and helper functions for CLI editor binaries.
//!
//! This module contains common code used across multiple editor binaries
//! (`item_editor`, `class_editor`, `race_editor`) to avoid duplication.
//! Since each binary in `src/bin/` is compiled as its own crate, this
//! module is included via `#[path = "editor_common.rs"] mod editor_common;`.

/// Standard proficiency IDs recognized by the system.
///
/// These identifiers represent the canonical set of proficiency categories
/// used when defining class and race proficiency lists.
///
/// # Categories
///
/// - **Weapons**: `simple_weapon`, `martial_melee`, `martial_ranged`, `blunt_weapon`, `unarmed`
/// - **Armor**: `light_armor`, `medium_armor`, `heavy_armor`, `shield`
/// - **Magic Items**: `arcane_item`, `divine_item`
pub const STANDARD_PROFICIENCY_IDS: &[&str] = &[
    "simple_weapon",
    "martial_melee",
    "martial_ranged",
    "blunt_weapon",
    "unarmed",
    "light_armor",
    "medium_armor",
    "heavy_armor",
    "shield",
    "arcane_item",
    "divine_item",
];

/// Standard item tags used for race restrictions and item properties.
///
/// These tags are applied to items and referenced by race definitions
/// (via `incompatible_item_tags`) to enforce equipment restrictions.
///
/// # Tags
///
/// - `large_weapon` — Large/oversized weapons (restricted by small races)
/// - `two_handed` — Two-handed weapons (requires both hands)
/// - `heavy_armor` — Heavy armor pieces (restricted by small races)
/// - `elven_crafted` — Elven-crafted items (may have race restrictions)
/// - `dwarven_crafted` — Dwarven-crafted items (may have race restrictions)
/// - `requires_strength` — Items requiring high strength
pub const STANDARD_ITEM_TAGS: &[&str] = &[
    "large_weapon",
    "two_handed",
    "heavy_armor",
    "elven_crafted",
    "dwarven_crafted",
    "requires_strength",
];

/// Truncates a string to a maximum length, appending `...` if truncated.
///
/// If the string is already within `max_len`, it is returned unchanged.
/// Otherwise the string is cut and `...` is appended so that the total
/// length does not exceed `max_len`.
///
/// # Arguments
///
/// * `s` - The string slice to truncate.
/// * `max_len` - The maximum allowed length of the returned string.
///
/// # Returns
///
/// A `String` that is at most `max_len` characters long.
///
/// # Examples
///
/// ```
/// # #[path = "editor_common.rs"] mod editor_common;
/// # use editor_common::truncate;
/// assert_eq!(truncate("short", 10), "short");
/// assert_eq!(truncate("this is a very long string", 10), "this is...");
/// ```
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Filters proficiency candidates to include only standard proficiency IDs.
///
/// Each candidate string is checked against [`STANDARD_PROFICIENCY_IDS`].
/// Only candidates that match a known proficiency are included in the result.
///
/// # Arguments
///
/// * `candidates` - Slice of candidate proficiency strings.
///
/// # Returns
///
/// A `Vec<String>` containing only the valid (standard) proficiencies.
///
/// # Examples
///
/// ```
/// # #[path = "editor_common.rs"] mod editor_common;
/// # use editor_common::filter_valid_proficiencies;
/// let candidates = vec![
///     "simple_weapon".to_string(),
///     "invalid_tag".to_string(),
/// ];
/// let filtered = filter_valid_proficiencies(&candidates);
/// assert_eq!(filtered, vec!["simple_weapon".to_string()]);
/// ```
pub fn filter_valid_proficiencies(candidates: &[String]) -> Vec<String> {
    candidates
        .iter()
        .filter(|p| STANDARD_PROFICIENCY_IDS.contains(&p.as_str()))
        .cloned()
        .collect()
}

/// Filters tag candidates to include only standard item tags.
///
/// Each candidate string is checked against [`STANDARD_ITEM_TAGS`].
/// Only candidates that match a known tag are included in the result.
///
/// # Arguments
///
/// * `candidates` - Slice of candidate tag strings.
///
/// # Returns
///
/// A `Vec<String>` containing only the valid (standard) item tags.
///
/// # Examples
///
/// ```
/// # #[path = "editor_common.rs"] mod editor_common;
/// # use editor_common::filter_valid_tags;
/// let candidates = vec![
///     "large_weapon".to_string(),
///     "not_a_tag".to_string(),
/// ];
/// let filtered = filter_valid_tags(&candidates);
/// assert_eq!(filtered, vec!["large_weapon".to_string()]);
/// ```
pub fn filter_valid_tags(candidates: &[String]) -> Vec<String> {
    candidates
        .iter()
        .filter(|t| STANDARD_ITEM_TAGS.contains(&t.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("short", 10), "short");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("exactly10c", 10), "exactly10c");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("this is a very long string", 10), "this is...");
    }

    #[test]
    fn test_filter_valid_proficiencies_mixed() {
        let candidates = vec![
            "simple_weapon".to_string(),
            "invalid_tag".to_string(),
            "martial_melee".to_string(),
        ];
        let filtered = filter_valid_proficiencies(&candidates);
        assert_eq!(
            filtered,
            vec!["simple_weapon".to_string(), "martial_melee".to_string()]
        );
    }

    #[test]
    fn test_filter_valid_proficiencies_all_invalid() {
        let candidates = vec!["not_a_proficiency".to_string()];
        let filtered = filter_valid_proficiencies(&candidates);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_valid_proficiencies_empty() {
        let filtered = filter_valid_proficiencies(&[]);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_valid_tags_mixed() {
        let candidates = vec![
            "large_weapon".to_string(),
            "not_a_tag".to_string(),
            "heavy_armor".to_string(),
        ];
        let filtered = filter_valid_tags(&candidates);
        assert_eq!(
            filtered,
            vec!["large_weapon".to_string(), "heavy_armor".to_string()]
        );
    }

    #[test]
    fn test_filter_valid_tags_all_invalid() {
        let candidates = vec!["bogus".to_string()];
        let filtered = filter_valid_tags(&candidates);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_valid_tags_empty() {
        let filtered = filter_valid_tags(&[]);
        assert!(filtered.is_empty());
    }
}
