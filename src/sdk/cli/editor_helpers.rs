// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared constants and helper functions for SDK CLI editor modules.
//!
//! This module provides common I/O helpers and validation constants shared
//! across the `class_editor`, `race_editor`, and `item_editor` subcommands.
//! It replaces the `editor_common.rs` file that was previously included via
//! `#[path]` in each standalone bin target.

use std::io::{self, Write};

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
/// # Examples
///
/// ```
/// use antares::sdk::cli::editor_helpers::truncate;
///
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
/// # Examples
///
/// ```
/// use antares::sdk::cli::editor_helpers::filter_valid_proficiencies;
///
/// let candidates = vec!["simple_weapon".to_string(), "invalid_tag".to_string()];
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
/// # Examples
///
/// ```
/// use antares::sdk::cli::editor_helpers::filter_valid_tags;
///
/// let candidates = vec!["large_weapon".to_string(), "not_a_tag".to_string()];
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

/// Prints `prompt` to stdout, flushes, then reads one line from stdin.
///
/// The trailing newline from `read_line` is included in the returned string;
/// callers should call `.trim()` when comparing or storing the value.
///
/// # Panics
///
/// Panics if writing to stdout or reading from stdin fails — both of which
/// are unrecoverable errors in an interactive CLI tool.
pub fn read_line(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().expect("failed to flush stdout");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("failed to read line from stdin");
    input
}

/// Reads multiple string values interactively, one per line.
///
/// If `prompt` is non-empty it is printed before the loop starts.
/// The loop prints an instruction banner, then calls [`read_line`] with
/// `label` repeatedly, collecting non-empty trimmed values until the
/// user presses Enter on a blank line.
///
/// # Examples
///
/// This function reads from `stdin` so it cannot be tested in a simple
/// doc-test, but the unit tests in this module exercise it indirectly.
pub fn input_multistring_values(prompt: &str, label: &str) -> Vec<String> {
    if !prompt.is_empty() {
        println!("\n{}", prompt);
    }
    println!("(Enter values one per line. Press Enter on an empty line to finish.)");
    let mut values: Vec<String> = Vec::new();
    loop {
        let input = read_line(label);
        let trimmed = input.trim();
        if trimmed.is_empty() {
            break;
        }
        values.push(trimmed.to_string());
    }
    values
}

// ──────────────────────────────────────────────────────────────────────────────
// Test-only helpers
// ──────────────────────────────────────────────────────────────────────────────

/// Parses a multi-line string into a `Vec<String>` the same way the
/// interactive `input_multistring_values` loop would — trimming each
/// line and ignoring blank lines.
///
/// Used in unit tests across multiple editor modules.
#[cfg(test)]
pub fn parse_multistring_input(input: &str) -> Vec<String> {
    input
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
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
    fn test_truncate_very_small_max_len() {
        // max_len < 3: saturating_sub gives 0, so prefix is empty
        let result = truncate("hello", 2);
        assert_eq!(result, "...");
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
        assert!(filter_valid_proficiencies(&candidates).is_empty());
    }

    #[test]
    fn test_filter_valid_proficiencies_empty_input() {
        assert!(filter_valid_proficiencies(&[]).is_empty());
    }

    #[test]
    fn test_filter_valid_proficiencies_all_valid() {
        let candidates: Vec<String> = STANDARD_PROFICIENCY_IDS
            .iter()
            .map(|s| s.to_string())
            .collect();
        let filtered = filter_valid_proficiencies(&candidates);
        assert_eq!(filtered.len(), STANDARD_PROFICIENCY_IDS.len());
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
        assert!(filter_valid_tags(&candidates).is_empty());
    }

    #[test]
    fn test_filter_valid_tags_empty_input() {
        assert!(filter_valid_tags(&[]).is_empty());
    }

    #[test]
    fn test_parse_multistring_input_basic() {
        let s = "alpha\nbeta\n\n  gamma  \n";
        let parsed = parse_multistring_input(s);
        assert_eq!(
            parsed,
            vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]
        );
    }

    #[test]
    fn test_parse_multistring_input_empty() {
        assert!(parse_multistring_input("").is_empty());
    }

    #[test]
    fn test_parse_multistring_input_whitespace_only() {
        let s = "\n   \n  \n";
        assert!(parse_multistring_input(s).is_empty());
    }

    #[test]
    fn test_parse_multistring_input_with_whitespace() {
        let s = "\n   \n  one  \n two \n\nthree\n";
        let parsed = parse_multistring_input(s);
        assert_eq!(
            parsed,
            vec!["one".to_string(), "two".to_string(), "three".to_string()]
        );
    }
}
