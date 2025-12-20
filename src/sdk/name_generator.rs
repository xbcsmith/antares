// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Fantasy Character Name Generator for Campaign Builder SDK
//!
//! Generates fantasy names for NPCs and characters with star-themed options
//! inspired by Antares (the red supergiant "rival to Mars") and Arcturus
//! (the "Guardian of the Bear").
//!
//! # Architecture Reference
//!
//! This module is part of the SDK layer (Section 3.2) and provides content
//! generation utilities for the Campaign Builder tools.
//!
//! # Examples
//!
//! ```
//! use antares::sdk::name_generator::{NameGenerator, NameTheme};
//!
//! // Generate a star-themed name
//! let generator = NameGenerator::new();
//! let name = generator.generate(NameTheme::Star);
//! println!("Generated name: {}", name);
//!
//! // Generate with a title
//! let name_with_title = generator.generate_with_title(NameTheme::Antares);
//! println!("Hero: {}", name_with_title);
//!
//! // Generate with lore
//! let (name, lore) = generator.generate_with_lore(NameTheme::Arcturus);
//! println!("{}: {}", name, lore);
//!
//! // Generate multiple names
//! let names = generator.generate_multiple(5, NameTheme::Fantasy);
//! for name in names {
//!     println!("- {}", name);
//! }
//! ```

use rand::prelude::*;

/// Themes for name generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameTheme {
    /// General fantasy names
    Fantasy,
    /// Star-themed names based on real star names
    Star,
    /// Antares-themed (red supergiant, rival to Mars, heart of Scorpius)
    Antares,
    /// Arcturus-themed (guardian of the bear, northern star)
    Arcturus,
}

/// Fantasy character name generator
///
/// Generates thematic names suitable for NPCs and characters in the Antares game.
/// Supports multiple themes including star-based names inspired by celestial bodies.
///
/// # Thread Safety
///
/// This generator is `Send` and `Sync` but uses thread-local RNG internally.
/// Each thread will have its own randomness state.
#[derive(Debug, Clone)]
pub struct NameGenerator {
    // Future: could add custom word lists here
}

impl Default for NameGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl NameGenerator {
    /// Creates a new name generator
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::name_generator::NameGenerator;
    ///
    /// let generator = NameGenerator::new();
    /// ```
    pub fn new() -> Self {
        Self {}
    }

    /// Generates a single name based on the given theme
    ///
    /// # Arguments
    ///
    /// * `theme` - The naming theme to use
    ///
    /// # Returns
    ///
    /// A generated name without a title
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::name_generator::{NameGenerator, NameTheme};
    ///
    /// let generator = NameGenerator::new();
    /// let name = generator.generate(NameTheme::Star);
    /// assert!(!name.is_empty());
    /// ```
    pub fn generate(&self, theme: NameTheme) -> String {
        let mut rng = rand::rng();

        match theme {
            NameTheme::Fantasy => Self::generate_fantasy_name(&mut rng),
            NameTheme::Star => Self::generate_star_name(&mut rng),
            NameTheme::Antares => Self::generate_antares_name(&mut rng),
            NameTheme::Arcturus => Self::generate_arcturus_name(&mut rng),
        }
    }

    /// Generates a name with a 40% chance of including a title
    ///
    /// # Arguments
    ///
    /// * `theme` - The naming theme to use
    ///
    /// # Returns
    ///
    /// A generated name, possibly with a title like "the Brave" or "Starborn"
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::name_generator::{NameGenerator, NameTheme};
    ///
    /// let generator = NameGenerator::new();
    /// let name = generator.generate_with_title(NameTheme::Antares);
    /// assert!(!name.is_empty());
    /// ```
    pub fn generate_with_title(&self, theme: NameTheme) -> String {
        let mut rng = rand::rng();
        let base_name = self.generate(theme);

        // 40% chance to add a title
        if rng.random_bool(0.4) {
            let title = TITLES.choose(&mut rng).unwrap();
            format!("{} {}", base_name, title)
        } else {
            base_name
        }
    }

    /// Generates a name with accompanying lore description
    ///
    /// # Arguments
    ///
    /// * `theme` - The naming theme to use
    ///
    /// # Returns
    ///
    /// A tuple of (name, lore_description)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::name_generator::{NameGenerator, NameTheme};
    ///
    /// let generator = NameGenerator::new();
    /// let (name, lore) = generator.generate_with_lore(NameTheme::Arcturus);
    /// assert!(!name.is_empty());
    /// assert!(!lore.is_empty());
    /// ```
    pub fn generate_with_lore(&self, theme: NameTheme) -> (String, String) {
        let name = self.generate_with_title(theme);
        let lore = Self::generate_lore(&name, theme);
        (name, lore)
    }

    /// Generates multiple names at once
    ///
    /// # Arguments
    ///
    /// * `count` - Number of names to generate
    /// * `theme` - The naming theme to use
    ///
    /// # Returns
    ///
    /// A vector of generated names
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::name_generator::{NameGenerator, NameTheme};
    ///
    /// let generator = NameGenerator::new();
    /// let names = generator.generate_multiple(5, NameTheme::Star);
    /// assert_eq!(names.len(), 5);
    /// ```
    pub fn generate_multiple(&self, count: usize, theme: NameTheme) -> Vec<String> {
        (0..count)
            .map(|_| self.generate_with_title(theme))
            .collect()
    }

    /// Generates multiple names with lore descriptions
    ///
    /// # Arguments
    ///
    /// * `count` - Number of names to generate
    /// * `theme` - The naming theme to use
    ///
    /// # Returns
    ///
    /// A vector of (name, lore_description) tuples
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::name_generator::{NameGenerator, NameTheme};
    ///
    /// let generator = NameGenerator::new();
    /// let entries = generator.generate_multiple_with_lore(3, NameTheme::Antares);
    /// assert_eq!(entries.len(), 3);
    /// for (name, lore) in entries {
    ///     assert!(!name.is_empty());
    ///     assert!(!lore.is_empty());
    /// }
    /// ```
    pub fn generate_multiple_with_lore(
        &self,
        count: usize,
        theme: NameTheme,
    ) -> Vec<(String, String)> {
        (0..count).map(|_| self.generate_with_lore(theme)).collect()
    }

    // Private generation methods

    fn generate_star_name(rng: &mut impl Rng) -> String {
        let prefix = STAR_PREFIXES.choose(rng).unwrap();
        let suffix = STAR_SUFFIXES.choose(rng).unwrap();
        format!("{}{}", prefix, suffix)
    }

    fn generate_fantasy_name(rng: &mut impl Rng) -> String {
        let prefix = FANTASY_PREFIXES.choose(rng).unwrap();
        let suffix = FANTASY_SUFFIXES.choose(rng).unwrap();
        format!("{}{}", prefix, suffix)
    }

    fn generate_antares_name(rng: &mut impl Rng) -> String {
        let theme = ANTARES_THEMES.choose(rng).unwrap();
        let suffix = STAR_SUFFIXES.choose(rng).unwrap();

        // Create compound names with Antares lore
        let patterns = [
            format!("Antar{}", suffix),
            format!(
                "{}{}",
                theme,
                ["ion", "us", "is", "as"].choose(rng).unwrap()
            ),
            format!("Red{}", theme),
            format!("{}heart", theme),
            format!("Scorpi{}", suffix),
        ];

        patterns.choose(rng).unwrap().clone()
    }

    fn generate_arcturus_name(rng: &mut impl Rng) -> String {
        let theme = ARCTURUS_THEMES.choose(rng).unwrap();
        let suffix = STAR_SUFFIXES.choose(rng).unwrap();

        // Create compound names with Arcturus lore
        let patterns = [
            format!("Arctur{}", suffix),
            format!(
                "{}{}",
                theme,
                ["ion", "us", "is", "ar"].choose(rng).unwrap()
            ),
            format!("Bear{}", theme),
            format!("{}star", theme),
            format!("North{}", suffix),
        ];

        patterns.choose(rng).unwrap().clone()
    }

    fn generate_lore(name: &str, theme: NameTheme) -> String {
        let mut rng = rand::rng();

        let templates = match theme {
            NameTheme::Antares => ANTARES_LORE_TEMPLATES,
            NameTheme::Arcturus => ARCTURUS_LORE_TEMPLATES,
            NameTheme::Star => STAR_LORE_TEMPLATES,
            NameTheme::Fantasy => FANTASY_LORE_TEMPLATES,
        };

        let template = templates.choose(&mut rng).unwrap();
        template.replace("{name}", name)
    }
}

// Star-themed name components inspired by real star names
static STAR_PREFIXES: &[&str] = &[
    "Antar",
    "Arc",
    "Sirius",
    "Vega",
    "Rigel",
    "Altair",
    "Betel",
    "Deneb",
    "Pollux",
    "Castor",
    "Spica",
    "Aldeb",
    "Regul",
    "Capell",
    "Procy",
    "Acrux",
    "Hadar",
    "Mira",
    "Algol",
    "Fomalhaut",
    "Mimosa",
    "Bellatrix",
    "Shaula",
    "Elnath",
    "Alnilam",
];

static STAR_SUFFIXES: &[&str] = &[
    "es", "is", "us", "on", "as", "ax", "an", "ar", "ian", "ius", "or", "ix", "el", "iel", "eth",
    "ath", "yn", "os", "ux", "ara",
];

// Fantasy name components for general use
static FANTASY_PREFIXES: &[&str] = &[
    "Thal", "Kor", "Mal", "Vel", "Dra", "Zar", "Fen", "Bor", "Grim", "Thur", "Eld", "Mor", "Kael",
    "Rav", "Syl", "Lyr", "Aren", "Bran", "Cal", "Dor",
];

static FANTASY_SUFFIXES: &[&str] = &[
    "ion", "ric", "wen", "dor", "mir", "thor", "las", "rin", "win", "dan", "dir", "eth", "ros",
    "nar", "gar", "var", "kin", "lin", "tor", "mon",
];

// Antares-specific lore elements (red supergiant, scorpion constellation, "rival to Mars")
static ANTARES_THEMES: &[&str] = &[
    "Scorpius",
    "Crimson",
    "Ruby",
    "Mars",
    "Warrior",
    "Heart",
    "Flame",
    "Blood",
    "Supergiant",
    "Shadow",
    "Eclipse",
    "Nebula",
    "Void",
    "Chaos",
];

// Arcturus-specific lore elements (guardian, bear, bright northern star)
static ARCTURUS_THEMES: &[&str] = &[
    "Guardian",
    "Bear",
    "Sentinel",
    "Watcher",
    "Shield",
    "Protector",
    "North",
    "Bright",
    "Guide",
    "Keeper",
    "Ancient",
    "Wise",
    "Elder",
];

// Character titles for flavor
static TITLES: &[&str] = &[
    "the Brave",
    "the Wise",
    "the Swift",
    "the Strong",
    "the Cunning",
    "the Eternal",
    "the Wanderer",
    "the Guardian",
    "the Shadow",
    "the Light",
    "Starborn",
    "Skyseeker",
    "Voidwalker",
    "Flamekeeper",
    "Stormcaller",
];

// Lore templates
static ANTARES_LORE_TEMPLATES: &[&str] = &[
    "{name} bears the crimson mark of Antares, the great red supergiant that rivals Mars in the night sky.",
    "Born under the scorpion's heart, {name} carries the fierce legacy of Antares within.",
    "{name} channels the ancient power of Antares, the dying star that blazes with supergiant fury.",
    "Marked by the red star Antares, {name} walks the path between light and shadow.",
    "The blood-red light of Antares illuminated {name}'s birth, granting them the strength of the cosmos.",
];

static ARCTURUS_LORE_TEMPLATES: &[&str] = &[
    "{name} serves as guardian, following the ancient path of Arcturus, the bear's sentinel.",
    "Like Arcturus the bright watcher, {name} stands vigilant against the encroaching darkness.",
    "{name} draws wisdom from Arcturus, the ancient northern star that guides lost travelers.",
    "Blessed by the guardian star Arcturus, {name} protects those who cannot protect themselves.",
    "The constellation of the bear watches over {name}, granting the wisdom of Arcturus.",
];

static STAR_LORE_TEMPLATES: &[&str] = &[
    "{name} was born beneath a constellation of unusual brightness, marking them for greatness.",
    "The stars themselves whispered {name}'s destiny on the night of their birth.",
    "{name} navigates by starlight, their fate intertwined with the celestial dance above.",
    "On the night {name} entered the world, a new star appeared in the heavens.",
    "The ancient astronomers foretold {name}'s coming when the stars aligned in the pattern of destiny.",
];

static FANTASY_LORE_TEMPLATES: &[&str] = &[
    "{name} emerged from the mists of legend to forge their own destiny.",
    "Tales of {name}'s exploits spread across the realm like wildfire.",
    "{name} seeks glory and adventure in the uncharted territories of Antares.",
    "The bards sing of {name}, whose name will echo through the ages.",
    "Against all odds, {name} rose from obscurity to become a legend.",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_star_name() {
        let generator = NameGenerator::new();
        let name = generator.generate(NameTheme::Star);
        assert!(!name.is_empty());
        assert!(name.len() > 2);
    }

    #[test]
    fn test_generate_fantasy_name() {
        let generator = NameGenerator::new();
        let name = generator.generate(NameTheme::Fantasy);
        assert!(!name.is_empty());
        assert!(name.len() > 2);
    }

    #[test]
    fn test_generate_antares_name() {
        let generator = NameGenerator::new();
        let name = generator.generate(NameTheme::Antares);
        assert!(!name.is_empty());
        assert!(name.len() > 2);
    }

    #[test]
    fn test_generate_arcturus_name() {
        let generator = NameGenerator::new();
        let name = generator.generate(NameTheme::Arcturus);
        assert!(!name.is_empty());
        assert!(name.len() > 2);
    }

    #[test]
    fn test_generate_with_title() {
        let generator = NameGenerator::new();
        let name = generator.generate_with_title(NameTheme::Star);
        assert!(!name.is_empty());
    }

    #[test]
    fn test_generate_with_lore() {
        let generator = NameGenerator::new();
        let (name, lore) = generator.generate_with_lore(NameTheme::Antares);
        assert!(!name.is_empty());
        assert!(!lore.is_empty());
        assert!(lore.contains(&name));
    }

    #[test]
    fn test_generate_multiple() {
        let generator = NameGenerator::new();
        let names = generator.generate_multiple(10, NameTheme::Fantasy);
        assert_eq!(names.len(), 10);

        for name in names {
            assert!(!name.is_empty());
        }
    }

    #[test]
    fn test_generate_multiple_with_lore() {
        let generator = NameGenerator::new();
        let entries = generator.generate_multiple_with_lore(5, NameTheme::Arcturus);
        assert_eq!(entries.len(), 5);

        for (name, lore) in entries {
            assert!(!name.is_empty());
            assert!(!lore.is_empty());
            assert!(lore.contains(&name));
        }
    }

    #[test]
    fn test_all_themes_generate_valid_names() {
        let generator = NameGenerator::new();
        let themes = [
            NameTheme::Fantasy,
            NameTheme::Star,
            NameTheme::Antares,
            NameTheme::Arcturus,
        ];

        for theme in themes {
            let name = generator.generate(theme);
            assert!(!name.is_empty(), "Theme {:?} produced empty name", theme);
        }
    }

    #[test]
    fn test_name_diversity() {
        let generator = NameGenerator::new();
        let mut names = std::collections::HashSet::new();

        // Generate 100 names, should have good variety
        for _ in 0..100 {
            names.insert(generator.generate(NameTheme::Star));
        }

        // Should have at least 50 unique names out of 100
        assert!(
            names.len() >= 50,
            "Name diversity too low: only {} unique names out of 100",
            names.len()
        );
    }

    #[test]
    fn test_lore_contains_name() {
        let generator = NameGenerator::new();

        for theme in [
            NameTheme::Fantasy,
            NameTheme::Star,
            NameTheme::Antares,
            NameTheme::Arcturus,
        ] {
            let (name, lore) = generator.generate_with_lore(theme);
            assert!(
                lore.contains(&name),
                "Lore '{}' should contain name '{}'",
                lore,
                name
            );
        }
    }

    #[test]
    fn test_star_name_components() {
        let generator = NameGenerator::new();
        let mut has_prefix = false;
        let mut has_suffix = false;

        // Generate several names and check they use known components
        for _ in 0..20 {
            let name = generator.generate(NameTheme::Star);

            for prefix in STAR_PREFIXES {
                if name.starts_with(prefix) {
                    has_prefix = true;
                }
            }

            for suffix in STAR_SUFFIXES {
                if name.ends_with(suffix) {
                    has_suffix = true;
                }
            }
        }

        assert!(has_prefix, "Should use star prefixes");
        assert!(has_suffix, "Should use star suffixes");
    }

    #[test]
    fn test_antares_theme_in_names() {
        let generator = NameGenerator::new();
        let mut found_theme = false;

        // Generate several Antares names
        for _ in 0..20 {
            let name = generator.generate(NameTheme::Antares);

            // Check if name contains Antares-specific elements
            if name.contains("Antar")
                || name.contains("Scorpi")
                || name.contains("Red")
                || name.contains("Crimson")
                || name.contains("Mars")
            {
                found_theme = true;
                break;
            }
        }

        assert!(
            found_theme,
            "Antares names should contain thematic elements"
        );
    }

    #[test]
    fn test_arcturus_theme_in_names() {
        let generator = NameGenerator::new();
        let mut found_theme = false;

        // Generate several Arcturus names
        for _ in 0..20 {
            let name = generator.generate(NameTheme::Arcturus);

            // Check if name contains Arcturus-specific elements
            if name.contains("Arctur")
                || name.contains("Bear")
                || name.contains("North")
                || name.contains("Guardian")
                || name.contains("Sentinel")
            {
                found_theme = true;
                break;
            }
        }

        assert!(
            found_theme,
            "Arcturus names should contain thematic elements"
        );
    }

    #[test]
    fn test_zero_count_generates_empty_vec() {
        let generator = NameGenerator::new();
        let names = generator.generate_multiple(0, NameTheme::Fantasy);
        assert!(names.is_empty());
    }

    #[test]
    fn test_large_batch_generation() {
        let generator = NameGenerator::new();
        let names = generator.generate_multiple(1000, NameTheme::Star);
        assert_eq!(names.len(), 1000);
    }
}
