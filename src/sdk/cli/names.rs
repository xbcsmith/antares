// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! `antares-sdk names` — Fantasy character name generator.
//!
//! Migrated from `src/bin/name_gen.rs`. Exposes [`run`] as the single entry
//! point called by `src/bin/antares_sdk.rs`.

use crate::sdk::name_generator::{NameGenerator, NameTheme};
use clap::{Args, ValueEnum};

/// Theme selection for the name generator.
///
/// Each variant maps to the corresponding [`NameTheme`] used by
/// [`NameGenerator`].
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ThemeArg {
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

/// Arguments for the `antares-sdk names` subcommand.
#[derive(Args, Debug)]
#[command(
    about = "Generate fantasy character names for the Antares game",
    long_about = "Generates fantasy names for NPCs and characters with star-themed options\n\
                  inspired by Antares (the red supergiant 'rival to Mars') and Arcturus\n\
                  (the 'Guardian of the Bear')"
)]
pub struct NamesArgs {
    /// Number of names to generate
    #[arg(short, long, default_value = "5")]
    pub number: usize,

    /// Theme for name generation
    #[arg(short, long, value_enum, default_value = "fantasy")]
    pub theme: ThemeArg,

    /// Include lore descriptions with each name
    #[arg(short, long)]
    pub lore: bool,

    /// Suppress header output (names only)
    #[arg(short, long)]
    pub quiet: bool,
}

/// Run the name generator with the provided arguments.
///
/// Prints generated names (and optional lore) to stdout. The header is
/// suppressed when `args.quiet` is `true`.
///
/// # Errors
///
/// This function is infallible in the current implementation but returns
/// `Result` to maintain a uniform entry-point signature across all
/// `antares-sdk` subcommands and to allow future error propagation without
/// a breaking signature change.
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::cli::names::{run, NamesArgs, ThemeArg};
///
/// let args = NamesArgs {
///     number: 3,
///     theme: ThemeArg::Fantasy,
///     lore: false,
///     quiet: true,
/// };
/// run(args).expect("name generation should succeed");
/// ```
pub fn run(args: NamesArgs) -> Result<(), Box<dyn std::error::Error>> {
    let generator = NameGenerator::new();
    let theme: NameTheme = args.theme.into();

    if !args.quiet {
        print_header(&args);
    }

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

    Ok(())
}

/// Print a themed header to stdout.
///
/// Only called when `args.quiet` is `false`.
fn print_header(args: &NamesArgs) {
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

    /// Every `ThemeArg` variant must convert to the matching `NameTheme`.
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

    /// `run` must return `Ok` for all theme variants with a non-zero count.
    #[test]
    fn test_run_returns_ok_for_all_themes() {
        for theme in [
            ThemeArg::Fantasy,
            ThemeArg::Star,
            ThemeArg::Antares,
            ThemeArg::Arcturus,
        ] {
            let args = NamesArgs {
                number: 1,
                theme,
                lore: false,
                quiet: true,
            };
            assert!(run(args).is_ok(), "run failed for theme {:?}", theme);
        }
    }

    /// `run` with `lore = true` must also return `Ok`.
    #[test]
    fn test_run_with_lore_returns_ok() {
        let args = NamesArgs {
            number: 2,
            theme: ThemeArg::Fantasy,
            lore: true,
            quiet: true,
        };
        assert!(run(args).is_ok());
    }

    /// `run` with `number = 0` must return `Ok` (no output, no panic).
    #[test]
    fn test_run_zero_names_returns_ok() {
        let args = NamesArgs {
            number: 0,
            theme: ThemeArg::Fantasy,
            lore: false,
            quiet: true,
        };
        assert!(run(args).is_ok());
    }
}
