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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Names(args) => cli::names::run(args),
        Commands::Campaign(args) => cli::campaign_validator::run(args),
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
