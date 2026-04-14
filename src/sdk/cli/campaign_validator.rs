// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! `antares-sdk campaign` — Campaign-level validation tools.
//!
//! Migrated from `src/bin/campaign_validator.rs`. Exposes [`run`] as the
//! single entry point called by `src/bin/antares_sdk.rs`.
//!
//! # Subcommands
//!
//! | Subcommand | Description                               |
//! |------------|-------------------------------------------|
//! | `validate` | Validate one or all campaign directories  |
//!
//! # Examples
//!
//! ```no_run
//! use antares::sdk::cli::campaign_validator::{
//!     run, CampaignArgs, CampaignSubcommand, CampaignValidateArgs,
//! };
//! use std::path::PathBuf;
//!
//! let args = CampaignArgs {
//!     command: CampaignSubcommand::Validate(CampaignValidateArgs {
//!         campaign: Some(PathBuf::from("campaigns/tutorial")),
//!         all: false,
//!         campaigns_dir: PathBuf::from("campaigns"),
//!         verbose: false,
//!         json: false,
//!         errors_only: false,
//!     }),
//! };
//! // run(args).unwrap();
//! ```

use crate::sdk::campaign_loader::{Campaign, CampaignLoader, ValidationReport};
use crate::sdk::dialogue_editor::validate_dialogue;
use crate::sdk::quest_editor::validate_quest;
use crate::sdk::validation::Validator;
use clap::{Args, Subcommand};
use std::path::PathBuf;

/// Arguments for the `antares-sdk campaign` subcommand group.
///
/// This struct acts as the dispatcher for nested campaign subcommands. Pass it
/// to [`run`] to execute the chosen subcommand.
#[derive(Args, Debug)]
#[command(about = "Campaign-level validation tools")]
pub struct CampaignArgs {
    /// The campaign subcommand to execute.
    #[command(subcommand)]
    pub command: CampaignSubcommand,
}

/// Available subcommands under `antares-sdk campaign`.
#[derive(Subcommand, Debug)]
pub enum CampaignSubcommand {
    /// Validate a campaign directory for correctness and completeness.
    ///
    /// Supply a single `CAMPAIGN_DIR` to validate one campaign, or use
    /// `--all` to validate every campaign found in `--campaigns-dir`.
    Validate(CampaignValidateArgs),
}

/// Arguments for `antares-sdk campaign validate`.
#[derive(Args, Debug)]
#[command(
    about = "Validate an Antares campaign",
    long_about = "Validates campaign structure, metadata, content references, and data integrity.\n\
                  Supply a single CAMPAIGN_DIR, or use --all to validate every campaign in a directory."
)]
pub struct CampaignValidateArgs {
    /// Campaign directory to validate.
    #[arg(value_name = "CAMPAIGN_DIR")]
    pub campaign: Option<PathBuf>,

    /// Validate all campaigns found in `--campaigns-dir`.
    #[arg(short, long)]
    pub all: bool,

    /// Root directory that contains all campaigns (used with `--all`).
    #[arg(short = 'd', long, default_value = "campaigns")]
    pub campaigns_dir: PathBuf,

    /// Verbose output — print detailed progress and content statistics.
    #[arg(short, long)]
    pub verbose: bool,

    /// Emit results as JSON instead of human-readable text.
    #[arg(short, long)]
    pub json: bool,

    /// Only show errors; suppress warnings from the output.
    #[arg(short = 'e', long)]
    pub errors_only: bool,
}

/// Run the `campaign` subcommand with the given arguments.
///
/// Dispatches to the appropriate handler based on the chosen
/// [`CampaignSubcommand`].
///
/// # Errors
///
/// Returns `Err` when arguments are structurally invalid (e.g. neither
/// `--all` nor a campaign path is provided). Validation failures are
/// signalled via [`std::process::exit(1)`] to preserve identical exit-code
/// behaviour with the original standalone `campaign_validator` binary.
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::cli::campaign_validator::{
///     run, CampaignArgs, CampaignSubcommand, CampaignValidateArgs,
/// };
/// use std::path::PathBuf;
///
/// let args = CampaignArgs {
///     command: CampaignSubcommand::Validate(CampaignValidateArgs {
///         campaign: Some(PathBuf::from("campaigns/tutorial")),
///         all: false,
///         campaigns_dir: PathBuf::from("campaigns"),
///         verbose: false,
///         json: false,
///         errors_only: false,
///     }),
/// };
/// // run(args).unwrap();
/// ```
pub fn run(args: CampaignArgs) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        CampaignSubcommand::Validate(validate_args) => run_validate(validate_args),
    }
}

/// Execute the `validate` subcommand.
///
/// Exits the process with code 1 if validation finds errors. This preserves
/// the behaviour of the original standalone binary and ensures shell scripts
/// can detect failures with `$?`.
fn run_validate(args: CampaignValidateArgs) -> Result<(), Box<dyn std::error::Error>> {
    if args.all {
        validate_all_campaigns(&args);
    } else if let Some(ref campaign_path) = args.campaign {
        validate_single_campaign(campaign_path, &args);
    } else {
        eprintln!("Error: Must specify campaign path or --all flag");
        eprintln!("Usage: antares-sdk campaign validate <CAMPAIGN_DIR>");
        eprintln!("   or: antares-sdk campaign validate --all");
        std::process::exit(1);
    }
    Ok(())
}

/// Validate every campaign found in `args.campaigns_dir` and print a summary.
///
/// Exits with code 1 if any campaign contains errors.
fn validate_all_campaigns(args: &CampaignValidateArgs) {
    let loader = CampaignLoader::new(&args.campaigns_dir);

    let campaigns = match loader.list_campaigns() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error listing campaigns: {}", e);
            std::process::exit(1);
        }
    };

    if campaigns.is_empty() {
        println!("No campaigns found in {}", args.campaigns_dir.display());
        return;
    }

    println!("Validating {} campaigns...\n", campaigns.len());

    let mut total_errors = 0;
    let mut total_warnings = 0;
    let mut valid_count = 0;

    for info in &campaigns {
        if args.verbose {
            println!("=== Campaign: {} ===", info.name);
        } else {
            print!("Validating {}... ", info.name);
        }

        let report = validate_campaign_comprehensive(&info.path, args.verbose);

        if report.is_valid {
            valid_count += 1;
            if !args.verbose {
                println!("✓ VALID");
            }
        } else if !args.verbose {
            println!("✗ INVALID");
        }

        total_errors += report.errors.len();
        total_warnings += report.warnings.len();

        if args.verbose {
            print_report(&report, args.errors_only);
            println!();
        }
    }

    println!("\n=== Summary ===");
    println!("Total campaigns: {}", campaigns.len());
    println!("Valid: {}", valid_count);
    println!("Invalid: {}", campaigns.len() - valid_count);
    println!("Total errors: {}", total_errors);
    println!("Total warnings: {}", total_warnings);

    if total_errors > 0 {
        std::process::exit(1);
    }
}

/// Validate a single campaign at `path` and print a report.
///
/// Exits with code 1 if the campaign is invalid.
fn validate_single_campaign(path: &PathBuf, args: &CampaignValidateArgs) {
    let report = validate_campaign_comprehensive(path, args.verbose);

    if args.json {
        print_json_report(&report);
    } else {
        print_report(&report, args.errors_only);
    }

    if !report.is_valid {
        std::process::exit(1);
    }
}

/// Run all five validation passes against the campaign at `path`.
///
/// Returns a [`ValidationReport`] summarising all errors and warnings
/// discovered. Never panics; load failures are captured as errors in the
/// report.
fn validate_campaign_comprehensive(path: &PathBuf, verbose: bool) -> ValidationReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Load campaign
    let campaign = match Campaign::load(path) {
        Ok(c) => c,
        Err(e) => {
            return ValidationReport {
                is_valid: false,
                errors: vec![format!("Failed to load campaign: {}", e)],
                warnings: Vec::new(),
            };
        }
    };

    if verbose {
        println!("Campaign: {} v{}", campaign.name, campaign.version);
        println!("Author: {}", campaign.author);
        println!("Engine: {}", campaign.engine_version);
    }

    // 1. Structure validation
    if verbose {
        println!("\n[1/5] Validating campaign structure...");
    }
    let structure_errors = campaign.validate_structure();
    errors.extend(structure_errors);

    // 2. Load content database
    if verbose {
        println!("[2/5] Loading content database...");
    }
    let db = match campaign.load_content() {
        Ok(db) => {
            if verbose {
                let stats = db.stats();
                println!("  Classes: {}", stats.class_count);
                println!("  Races: {}", stats.race_count);
                println!("  Items: {}", stats.item_count);
                println!("  Monsters: {}", stats.monster_count);
                println!("  Spells: {}", stats.spell_count);
                println!("  Maps: {}", stats.map_count);
                println!("  Quests: {}", stats.quest_count);
                println!("  Dialogues: {}", stats.dialogue_count);
            }
            db
        }
        Err(e) => {
            errors.push(format!("Failed to load content: {}", e));
            return ValidationReport {
                is_valid: false,
                errors,
                warnings,
            };
        }
    };

    let stats = db.stats();

    // Check for empty / missing content
    if stats.map_count == 0 {
        errors.push("No maps defined - campaign cannot be played".to_string());
    }
    if stats.class_count == 0 {
        warnings.push("No classes defined".to_string());
    }
    if stats.item_count == 0 {
        warnings.push("No items defined".to_string());
    }

    // 3. Validate cross-references
    if verbose {
        println!("[3/5] Validating cross-references...");
    }
    let validator = Validator::new(&db);
    match validator.validate_all() {
        Ok(validation_errors) => {
            for error in validation_errors {
                match error.severity() {
                    crate::sdk::validation::Severity::Error => {
                        errors.push(error.to_string());
                    }
                    crate::sdk::validation::Severity::Warning => {
                        warnings.push(error.to_string());
                    }
                    crate::sdk::validation::Severity::Info => {
                        // Info messages are not surfaced in validation reports.
                    }
                }
            }
        }
        Err(e) => {
            errors.push(format!("Validation failed: {}", e));
        }
    }

    // 4. Validate quests
    if verbose {
        println!("[4/5] Validating quests...");
    }
    for quest_id in db.quests.all_quests() {
        if let Some(quest) = db.quests.get_quest(quest_id) {
            let quest_errors = validate_quest(quest, &db);
            for error in quest_errors {
                errors.push(format!("Quest {}: {}", quest_id, error));
            }
        }
    }

    // 5. Validate dialogues
    if verbose {
        println!("[5/5] Validating dialogues...");
    }
    for dialogue_id in db.dialogues.all_dialogues() {
        if let Some(dialogue) = db.dialogues.get_dialogue(dialogue_id) {
            let dialogue_errors = validate_dialogue(dialogue, &db);
            for error in dialogue_errors {
                errors.push(format!("Dialogue {}: {}", dialogue_id, error));
            }
        }
    }

    ValidationReport {
        is_valid: errors.is_empty(),
        errors,
        warnings,
    }
}

/// Print a human-readable validation report to stdout.
fn print_report(report: &ValidationReport, errors_only: bool) {
    if report.is_valid {
        println!("\n✓ Campaign is VALID");
    } else {
        println!("\n✗ Campaign is INVALID");
    }

    if !report.errors.is_empty() {
        println!("\nErrors ({}):", report.errors.len());
        for (i, error) in report.errors.iter().enumerate() {
            println!("  {}. {}", i + 1, error);
        }
    }

    if !errors_only && !report.warnings.is_empty() {
        println!("\nWarnings ({}):", report.warnings.len());
        for (i, warning) in report.warnings.iter().enumerate() {
            println!("  {}. {}", i + 1, warning);
        }
    }

    if report.is_valid && report.warnings.is_empty() {
        println!("\nNo issues found!");
    }
}

/// Serialise a validation report as pretty-printed JSON to stdout.
fn print_json_report(report: &ValidationReport) {
    let json = serde_json::json!({
        "is_valid": report.is_valid,
        "errors": report.errors,
        "warnings": report.warnings,
        "error_count": report.errors.len(),
        "warning_count": report.warnings.len(),
    });

    // SAFETY: the Value is constructed from primitives (bool, Vec<String>,
    // usize) that are always JSON-serialisable.
    let output = serde_json::to_string_pretty(&json)
        .expect("serde_json Value built from primitives must serialise");
    println!("{}", output);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A valid empty report has no errors and no warnings.
    #[test]
    fn test_validation_report_valid() {
        let report = ValidationReport {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        assert!(report.is_valid);
        assert_eq!(report.errors.len(), 0);
        assert_eq!(report.warnings.len(), 0);
    }

    /// A report with errors must be marked invalid.
    #[test]
    fn test_validation_report_with_errors() {
        let report = ValidationReport {
            is_valid: false,
            errors: vec![
                "Missing monster ID: 42".to_string(),
                "Duplicate item ID: sword".to_string(),
            ],
            warnings: vec!["Low monster density".to_string()],
        };

        assert!(!report.is_valid);
        assert_eq!(report.errors.len(), 2);
        assert_eq!(report.warnings.len(), 1);
    }

    /// A report may be valid while still carrying warnings.
    #[test]
    fn test_validation_report_warnings_only() {
        let report = ValidationReport {
            is_valid: true,
            errors: Vec::new(),
            warnings: vec!["Balance warning: Level 1 area has high-level monsters".to_string()],
        };

        assert!(report.is_valid);
        assert_eq!(report.errors.len(), 0);
        assert_eq!(report.warnings.len(), 1);
    }

    /// The JSON output structure must contain all required fields.
    #[test]
    fn test_print_json_report_structure() {
        let report = ValidationReport {
            is_valid: false,
            errors: vec!["Test error".to_string()],
            warnings: vec!["Test warning".to_string()],
        };

        let json = serde_json::json!({
            "is_valid": report.is_valid,
            "errors": report.errors,
            "warnings": report.warnings,
            "error_count": report.errors.len(),
            "warning_count": report.warnings.len(),
        });

        assert_eq!(json["is_valid"], false);
        assert_eq!(json["error_count"], 1);
        assert_eq!(json["warning_count"], 1);
        assert_eq!(json["errors"][0], "Test error");
        assert_eq!(json["warnings"][0], "Test warning");
    }

    /// An empty report is valid.
    #[test]
    fn test_empty_report_is_valid() {
        let report = ValidationReport {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        assert!(report.is_valid);
        assert!(report.errors.is_empty());
        assert!(report.warnings.is_empty());
    }

    /// Multiple errors are preserved in order.
    #[test]
    fn test_report_with_multiple_errors() {
        let errors = vec![
            "Error 1".to_string(),
            "Error 2".to_string(),
            "Error 3".to_string(),
        ];

        let report = ValidationReport {
            is_valid: false,
            errors: errors.clone(),
            warnings: Vec::new(),
        };

        assert!(!report.is_valid);
        assert_eq!(report.errors.len(), 3);
        assert_eq!(report.errors, errors);
    }

    /// The JSON report must serialise without error and contain all keys.
    #[test]
    fn test_json_output_format() {
        let report = ValidationReport {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        let json = serde_json::json!({
            "is_valid": report.is_valid,
            "errors": report.errors,
            "warnings": report.warnings,
            "error_count": report.errors.len(),
            "warning_count": report.warnings.len(),
        });

        // Verify JSON can be serialised
        let json_string = serde_json::to_string(&json);
        assert!(json_string.is_ok());

        // Verify all expected keys are present
        assert!(json.is_object());
        assert!(json.get("is_valid").is_some());
        assert!(json.get("errors").is_some());
        assert!(json.get("warnings").is_some());
        assert!(json.get("error_count").is_some());
        assert!(json.get("warning_count").is_some());
    }

    /// `CampaignArgs` must have its `command` field set to a `Validate` variant.
    #[test]
    fn test_campaign_args_validate_subcommand_fields() {
        let validate_args = CampaignValidateArgs {
            campaign: None,
            all: true,
            campaigns_dir: PathBuf::from("campaigns"),
            verbose: false,
            json: false,
            errors_only: false,
        };

        // Verify the subcommand is constructible and the fields are accessible.
        let args = CampaignArgs {
            command: CampaignSubcommand::Validate(validate_args),
        };

        match args.command {
            CampaignSubcommand::Validate(ref v) => {
                assert!(v.all);
                assert_eq!(v.campaigns_dir, PathBuf::from("campaigns"));
                assert!(!v.verbose);
                assert!(!v.json);
                assert!(!v.errors_only);
            }
        }
    }

    /// `CampaignValidateArgs` with a specific campaign path stores it correctly.
    #[test]
    fn test_campaign_validate_args_with_path() {
        let args = CampaignValidateArgs {
            campaign: Some(PathBuf::from("campaigns/tutorial")),
            all: false,
            campaigns_dir: PathBuf::from("campaigns"),
            verbose: true,
            json: true,
            errors_only: false,
        };

        assert_eq!(args.campaign, Some(PathBuf::from("campaigns/tutorial")));
        assert!(!args.all);
        assert!(args.verbose);
        assert!(args.json);
    }
}
