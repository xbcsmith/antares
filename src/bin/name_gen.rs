// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Antares Fantasy Character Name Generator CLI
//!
//! Command-line tool for generating fantasy character names for NPCs and characters.
//! Supports multiple themes including star-themed names inspired by Antares and Arcturus.

use antares::sdk::name_generator::{NameGenerator, NameTheme};
use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ThemeArg {
    /// General fantasy names
    Fantasy,
    /// Star-themed names based on real star names
    Star,
    /// Antares-themed (red supergiant, rival to Mars, heart of Scorpius)
    Antares,
    /// Arcturus-themed (guardian of the bear, northern star)
    Arcturus,
}

impl From<ThemeArg> for NameTheme {
    fn from(arg: ThemeArg) -> Self {
        match arg {
            ThemeArg::Fantasy => NameTheme::Fantasy,
            ThemeArg::Star => NameTheme::Star,
            ThemeArg::Antares => NameTheme::Antares,
            ThemeArg::Arcturus => NameTheme::Arcturus,
        }
    }
}

#[derive(Parser)]
#[command(
    name = "antares-name-gen",
    about = "Generate fantasy character names for the Antares game",
    long_about = "Generates fantasy names for NPCs and characters with star-themed options\n\
                  inspired by Antares (the red supergiant 'rival to Mars') and Arcturus\n\
                  (the 'Guardian of the Bear')",
    version
)]
struct Args {
    /// Number of names to generate
    #[arg(short, long, default_value = "5")]
    number: usize,

    /// Theme for name generation
    #[arg(short, long, value_enum, default_value = "fantasy")]
    theme: ThemeArg,

    /// Include lore descriptions with each name
    #[arg(short, long)]
    lore: bool,

    /// Suppress header output (names only)
    #[arg(short, long)]
    quiet: bool,
}

fn main() {
    let args = Args::parse();

    let generator = NameGenerator::new();
    let theme: NameTheme = args.theme.into();

    // Print header unless quiet mode
    if !args.quiet {
        print_header(&args);
    }

    // Generate and display names
    if args.lore {
        let entries = generator.generate_multiple_with_lore(args.number, theme);
        for (i, (name, lore)) in entries.iter().enumerate() {
            if args.quiet {
                println!("{}", name);
            } else {
                println!("{}. {}", i + 1, name);
                println!("   {}\n", lore);
            }
        }
    } else {
        let names = generator.generate_multiple(args.number, theme);
        for (i, name) in names.iter().enumerate() {
            if args.quiet {
                println!("{}", name);
            } else {
                println!("{}. {}", i + 1, name);
            }
        }
    }
}

fn print_header(args: &Args) {
    let (title, description) = match args.theme {
        ThemeArg::Antares => (
            "ANTARES CHARACTER NAMES",
            "Theme: Red Supergiant | Rival to Mars | Heart of Scorpius",
        ),
        ThemeArg::Arcturus => (
            "ARCTURUS CHARACTER NAMES",
            "Theme: Guardian of the Bear | Bright Northern Star",
        ),
        ThemeArg::Star => (
            "STAR-THEMED CHARACTER NAMES",
            "Theme: Celestial Bodies & Constellations",
        ),
        ThemeArg::Fantasy => ("FANTASY CHARACTER NAMES", "Theme: General Fantasy"),
    };

    println!("\n=== {} ===", title);
    println!("{}", description);
    println!("Provider: Random Generation\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_arg_conversion() {
        let fantasy: NameTheme = ThemeArg::Fantasy.into();
        assert_eq!(fantasy, NameTheme::Fantasy);

        let star: NameTheme = ThemeArg::Star.into();
        assert_eq!(star, NameTheme::Star);

        let antares: NameTheme = ThemeArg::Antares.into();
        assert_eq!(antares, NameTheme::Antares);

        let arcturus: NameTheme = ThemeArg::Arcturus.into();
        assert_eq!(arcturus, NameTheme::Arcturus);
    }
}
