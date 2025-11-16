// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Campaign Validator CLI
//!
//! Comprehensive validation tool for Antares campaigns. Validates campaign
//! structure, metadata, content references, and data integrity.
//!
//! # Usage
//!
//! ```bash
//! # Validate a campaign
//! campaign_validator campaigns/my_campaign
//!
//! # Validate all campaigns in a directory
//! campaign_validator --all campaigns/
//!
//! # Verbose output
//! campaign_validator -v campaigns/my_campaign
//!
//! # JSON output
//! campaign_validator --json campaigns/my_campaign
//! ```

use antares::sdk::campaign_loader::{Campaign, CampaignLoader, ValidationReport};
use antares::sdk::dialogue_editor::validate_dialogue;
use antares::sdk::quest_editor::validate_quest;
use antares::sdk::validation::Validator;
use clap::Parser;
use std::path::PathBuf;
use std::process;

#[derive(Parser, Debug)]
#[command(name = "campaign_validator")]
#[command(about = "Validate Antares campaigns", long_about = None)]
struct Args {
    /// Campaign directory to validate
    #[arg(value_name = "CAMPAIGN_DIR")]
    campaign: Option<PathBuf>,

    /// Validate all campaigns in directory
    #[arg(short, long)]
    all: bool,

    /// Campaigns directory (when using --all)
    #[arg(short = 'd', long, default_value = "campaigns")]
    campaigns_dir: PathBuf,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Output as JSON
    #[arg(short, long)]
    json: bool,

    /// Only show errors (hide warnings)
    #[arg(short = 'e', long)]
    errors_only: bool,
}

fn main() {
    let args = Args::parse();

    if args.all {
        validate_all_campaigns(&args);
    } else if let Some(ref campaign_path) = args.campaign {
        validate_single_campaign(campaign_path, &args);
    } else {
        eprintln!("Error: Must specify campaign path or --all flag");
        eprintln!("Usage: campaign_validator <CAMPAIGN_DIR> or campaign_validator --all");
        process::exit(1);
    }
}

fn validate_all_campaigns(args: &Args) {
    let loader = CampaignLoader::new(&args.campaigns_dir);

    let campaigns = match loader.list_campaigns() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error listing campaigns: {}", e);
            process::exit(1);
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
        process::exit(1);
    }
}

fn validate_single_campaign(path: &PathBuf, args: &Args) {
    let report = validate_campaign_comprehensive(path, args.verbose);

    if args.json {
        print_json_report(&report);
    } else {
        print_report(&report, args.errors_only);
    }

    if !report.is_valid {
        process::exit(1);
    }
}

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

    // Check for empty content
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
                    antares::sdk::validation::Severity::Error => {
                        errors.push(error.to_string());
                    }
                    antares::sdk::validation::Severity::Warning => {
                        warnings.push(error.to_string());
                    }
                    antares::sdk::validation::Severity::Info => {
                        // Info messages are not errors or warnings, skip them
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

fn print_json_report(report: &ValidationReport) {
    let json = serde_json::json!({
        "is_valid": report.is_valid,
        "errors": report.errors,
        "warnings": report.warnings,
        "error_count": report.errors.len(),
        "warning_count": report.warnings.len(),
    });

    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}
