// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Unified Antares SDK Command-Line Interface
//!
//! Provides a single `antares-sdk` binary with subcommands for all developer
//! and content-creation tools. Each subcommand is implemented in
//! `src/sdk/cli/` and exposed here via a thin `clap` dispatch layer.
//!
//! # Usage
//!
//! ```text
//! antares-sdk names --theme fantasy --number 5
//! antares-sdk names --theme star --number 10 --lore
//! antares-sdk campaign validate campaigns/tutorial
//! antares-sdk campaign validate --all
//! antares-sdk campaign validate --all -d campaigns/
//! ```

use antares::sdk::cli;
use clap::{Parser, Subcommand};

/// Antares RPG SDK command-line tools.
///
/// Consolidates all developer and content-creation utilities for the Antares
/// game engine into a single binary with consistent subcommand UX.
#[derive(Parser)]
#[command(
    name = "antares-sdk",
    about = "Antares RPG SDK command-line tools",
    long_about = "Developer and content-creation tools for the Antares RPG game engine.\n\n\
                  Use subcommands to generate names, validate campaigns, and more.\n\n\
                  Run `antares-sdk <COMMAND> --help` for subcommand-specific help.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Top-level subcommands dispatched by `antares-sdk`.
#[derive(Subcommand)]
enum Commands {
    /// Generate fantasy character names for NPCs and characters.
    Names(cli::names::NamesArgs),
    /// Campaign-level validation tools.
    Campaign(cli::campaign_validator::CampaignArgs),
    /// Interactive class definition editor (REPL).
    Class(cli::class_editor::ClassArgs),
    /// Interactive item definition editor (REPL).
    Item(cli::item_editor::ItemArgs),
    /// Map creation and validation tools.
    Map(cli::map_validator::MapArgs),
    /// Interactive race definition editor (REPL).
    Race(cli::race_editor::RaceArgs),
    /// Generate placeholder terrain and tree textures.
    Textures(cli::texture_generator::TexturesArgs),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Names(args) => cli::names::run(args),
        Commands::Campaign(args) => cli::campaign_validator::run(args),
        Commands::Class(args) => cli::class_editor::run(args),
        Commands::Item(args) => cli::item_editor::run(args),
        Commands::Map(args) => cli::map_validator::run(args),
        Commands::Race(args) => cli::race_editor::run(args),
        Commands::Textures(args) => cli::texture_generator::run(args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// Verify the `antares-sdk --help` path works without panicking.
    ///
    /// This exercises the same code path that clap traverses when the user
    /// runs `antares-sdk --help`: it builds the full command tree, populates
    /// all metadata, and verifies every registered subcommand is reachable.
    #[test]
    fn test_antares_sdk_help_renders_without_panic() {
        let cmd = Cli::command();
        assert_eq!(cmd.get_name(), "antares-sdk");

        let subcommand_names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(
            subcommand_names.contains(&"names"),
            "names subcommand must be registered; found: {:?}",
            subcommand_names
        );
        assert!(
            subcommand_names.contains(&"campaign"),
            "campaign subcommand must be registered; found: {:?}",
            subcommand_names
        );
        assert!(
            subcommand_names.contains(&"class"),
            "class subcommand must be registered; found: {:?}",
            subcommand_names
        );
        assert!(
            subcommand_names.contains(&"item"),
            "item subcommand must be registered; found: {:?}",
            subcommand_names
        );
        assert!(
            subcommand_names.contains(&"map"),
            "map subcommand must be registered; found: {:?}",
            subcommand_names
        );
        assert!(
            subcommand_names.contains(&"race"),
            "race subcommand must be registered; found: {:?}",
            subcommand_names
        );
        assert!(
            subcommand_names.contains(&"textures"),
            "textures subcommand must be registered; found: {:?}",
            subcommand_names
        );
    }

    /// `antares-sdk names` with no extra flags must parse successfully using
    /// clap's default values.
    #[test]
    fn test_cli_parses_names_with_defaults() {
        let result = Cli::try_parse_from(["antares-sdk", "names"]);
        assert!(
            result.is_ok(),
            "should parse 'names' with defaults: {:?}",
            result.err()
        );
        match result.unwrap().command {
            Commands::Names(args) => {
                assert_eq!(args.number, 5, "default --number should be 5");
                assert!(!args.lore, "default --lore should be false");
                assert!(!args.quiet, "default --quiet should be false");
            }
            _ => panic!("expected Names command"),
        }
    }

    /// `antares-sdk names --number 10 --theme star --lore --quiet` must parse
    /// all flags correctly.
    #[test]
    fn test_cli_parses_names_with_all_flags() {
        let result = Cli::try_parse_from([
            "antares-sdk",
            "names",
            "--number",
            "10",
            "--theme",
            "star",
            "--lore",
            "--quiet",
        ]);
        assert!(
            result.is_ok(),
            "should parse full names flags: {:?}",
            result.err()
        );
        match result.unwrap().command {
            Commands::Names(args) => {
                assert_eq!(args.number, 10);
                assert!(args.lore);
                assert!(args.quiet);
            }
            _ => panic!("expected Names command"),
        }
    }

    /// `antares-sdk campaign validate campaigns/tutorial` must parse the
    /// campaign path correctly.
    #[test]
    fn test_cli_parses_campaign_validate_with_path() {
        use antares::sdk::cli::campaign_validator::CampaignSubcommand;
        use std::path::PathBuf;

        let result =
            Cli::try_parse_from(["antares-sdk", "campaign", "validate", "campaigns/tutorial"]);
        assert!(
            result.is_ok(),
            "should parse campaign validate with path: {:?}",
            result.err()
        );
        match result.unwrap().command {
            Commands::Campaign(args) => match args.command {
                CampaignSubcommand::Validate(v) => {
                    assert_eq!(v.campaign, Some(PathBuf::from("campaigns/tutorial")));
                    assert!(!v.all);
                    assert!(!v.verbose);
                    assert!(!v.json);
                    assert!(!v.errors_only);
                }
            },
            _ => panic!("expected Campaign command"),
        }
    }

    /// `antares-sdk campaign validate --all` must set the `all` flag.
    #[test]
    fn test_cli_parses_campaign_validate_all() {
        use antares::sdk::cli::campaign_validator::CampaignSubcommand;

        let result = Cli::try_parse_from(["antares-sdk", "campaign", "validate", "--all"]);
        assert!(
            result.is_ok(),
            "should parse campaign validate --all: {:?}",
            result.err()
        );
        match result.unwrap().command {
            Commands::Campaign(args) => match args.command {
                CampaignSubcommand::Validate(v) => {
                    assert!(v.all);
                    assert!(v.campaign.is_none());
                }
            },
            _ => panic!("expected Campaign command"),
        }
    }

    /// `antares-sdk map validate map_1.ron` must parse the file path and
    /// default to no `--campaign-dir`.
    #[test]
    fn test_cli_parses_map_validate_with_file() {
        use antares::sdk::cli::map_validator::MapSubcommand;
        use std::path::PathBuf;

        let result = Cli::try_parse_from(["antares-sdk", "map", "validate", "map_1.ron"]);
        assert!(
            result.is_ok(),
            "should parse map validate with file: {:?}",
            result.err()
        );
        match result.unwrap().command {
            Commands::Map(args) => match args.command {
                MapSubcommand::Validate(v) => {
                    assert_eq!(v.files, vec![PathBuf::from("map_1.ron")]);
                    assert!(v.campaign_dir.is_none());
                }
                MapSubcommand::Build => panic!("expected Validate subcommand, not Build"),
            },
            _ => panic!("expected Map command"),
        }
    }

    /// `antares-sdk map validate --campaign-dir campaigns/tutorial map_1.ron`
    /// must set `campaign_dir`.
    #[test]
    fn test_cli_parses_map_validate_with_campaign_dir() {
        use antares::sdk::cli::map_validator::MapSubcommand;
        use std::path::PathBuf;

        let result = Cli::try_parse_from([
            "antares-sdk",
            "map",
            "validate",
            "--campaign-dir",
            "campaigns/tutorial",
            "map_1.ron",
            "map_2.ron",
        ]);
        assert!(
            result.is_ok(),
            "should parse map validate with campaign-dir: {:?}",
            result.err()
        );
        match result.unwrap().command {
            Commands::Map(args) => match args.command {
                MapSubcommand::Validate(v) => {
                    assert_eq!(v.campaign_dir, Some(PathBuf::from("campaigns/tutorial")));
                    assert_eq!(v.files.len(), 2);
                }
                MapSubcommand::Build => panic!("expected Validate subcommand, not Build"),
            },
            _ => panic!("expected Map command"),
        }
    }

    /// `antares-sdk textures generate` must parse with the default output dir.
    #[test]
    fn test_cli_parses_textures_generate_with_defaults() {
        use antares::sdk::cli::texture_generator::TexturesSubcommand;
        use std::path::PathBuf;

        let result = Cli::try_parse_from(["antares-sdk", "textures", "generate"]);
        assert!(
            result.is_ok(),
            "should parse textures generate: {:?}",
            result.err()
        );
        match result.unwrap().command {
            Commands::Textures(args) => match args.command {
                TexturesSubcommand::Generate(g) => {
                    assert_eq!(g.output_dir, PathBuf::from("assets/textures"));
                }
            },
            _ => panic!("expected Textures command"),
        }
    }

    /// `antares-sdk class` with no args must parse using the default file path.
    #[test]
    fn test_cli_parses_class_with_default_file() {
        use std::path::PathBuf;

        let result = Cli::try_parse_from(["antares-sdk", "class"]);
        assert!(
            result.is_ok(),
            "should parse 'class' with defaults: {:?}",
            result.err()
        );
        match result.unwrap().command {
            Commands::Class(args) => {
                assert_eq!(args.file, PathBuf::from("data/classes.ron"));
            }
            _ => panic!("expected Class command"),
        }
    }

    /// `antares-sdk class path/to/classes.ron` must use the provided path.
    #[test]
    fn test_cli_parses_class_with_explicit_file() {
        use std::path::PathBuf;

        let result = Cli::try_parse_from(["antares-sdk", "class", "custom/classes.ron"]);
        assert!(
            result.is_ok(),
            "should parse 'class' with explicit file: {:?}",
            result.err()
        );
        match result.unwrap().command {
            Commands::Class(args) => {
                assert_eq!(args.file, PathBuf::from("custom/classes.ron"));
            }
            _ => panic!("expected Class command"),
        }
    }

    /// `antares-sdk race` with no args must parse using the default file path.
    #[test]
    fn test_cli_parses_race_with_default_file() {
        use std::path::PathBuf;

        let result = Cli::try_parse_from(["antares-sdk", "race"]);
        assert!(
            result.is_ok(),
            "should parse 'race' with defaults: {:?}",
            result.err()
        );
        match result.unwrap().command {
            Commands::Race(args) => {
                assert_eq!(args.file, PathBuf::from("data/races.ron"));
            }
            _ => panic!("expected Race command"),
        }
    }

    /// `antares-sdk item` with no args must parse using the default file path.
    #[test]
    fn test_cli_parses_item_with_default_file() {
        use std::path::PathBuf;

        let result = Cli::try_parse_from(["antares-sdk", "item"]);
        assert!(
            result.is_ok(),
            "should parse 'item' with defaults: {:?}",
            result.err()
        );
        match result.unwrap().command {
            Commands::Item(args) => {
                assert_eq!(args.file, PathBuf::from("data/items.ron"));
            }
            _ => panic!("expected Item command"),
        }
    }

    /// `antares-sdk map build` must parse as the Build sub-subcommand with no
    /// additional arguments.
    #[test]
    fn test_cli_parses_map_build() {
        use antares::sdk::cli::map_validator::MapSubcommand;

        let result = Cli::try_parse_from(["antares-sdk", "map", "build"]);
        assert!(
            result.is_ok(),
            "should parse 'map build': {:?}",
            result.err()
        );
        match result.unwrap().command {
            Commands::Map(args) => match args.command {
                MapSubcommand::Build => {}
                _ => panic!("expected Map Build subcommand"),
            },
            _ => panic!("expected Map command"),
        }
    }

    /// `antares-sdk textures generate --output-dir /tmp/out` must override
    /// the default output directory.
    #[test]
    fn test_cli_parses_textures_generate_with_output_dir() {
        use antares::sdk::cli::texture_generator::TexturesSubcommand;
        use std::path::PathBuf;

        let result = Cli::try_parse_from([
            "antares-sdk",
            "textures",
            "generate",
            "--output-dir",
            "/tmp/out",
        ]);
        assert!(
            result.is_ok(),
            "should parse textures generate --output-dir: {:?}",
            result.err()
        );
        match result.unwrap().command {
            Commands::Textures(args) => match args.command {
                TexturesSubcommand::Generate(g) => {
                    assert_eq!(g.output_dir, PathBuf::from("/tmp/out"));
                }
            },
            _ => panic!("expected Textures command"),
        }
    }

    /// `antares-sdk campaign validate --verbose --json --errors-only` must
    /// parse all optional flags.
    #[test]
    fn test_cli_parses_campaign_validate_optional_flags() {
        use antares::sdk::cli::campaign_validator::CampaignSubcommand;

        let result = Cli::try_parse_from([
            "antares-sdk",
            "campaign",
            "validate",
            "--all",
            "--verbose",
            "--json",
            "--errors-only",
        ]);
        assert!(
            result.is_ok(),
            "should parse optional campaign flags: {:?}",
            result.err()
        );
        match result.unwrap().command {
            Commands::Campaign(args) => match args.command {
                CampaignSubcommand::Validate(v) => {
                    assert!(v.verbose);
                    assert!(v.json);
                    assert!(v.errors_only);
                }
            },
            _ => panic!("expected Campaign command"),
        }
    }
}
