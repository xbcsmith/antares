// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Antares - Main Game Binary
//!
//! Turn-based RPG inspired by Might and Magic 1.
//!
//! # Usage
//!
//! ```bash
//! # Start new game with core content
//! antares
//!
//! # Start new game with specific campaign
//! antares --campaign tutorial
//!
//! # List available campaigns
//! antares --list-campaigns
//!
//! # Validate campaign before playing
//! antares --validate-campaign my_campaign
//!
//! # Continue last saved game
//! antares --continue
//!
//! # Load specific save file
//! antares --load my_save
//! ```

use antares::application::save_game::SaveGameManager;
use antares::application::GameState;
use antares::sdk::campaign_loader::CampaignLoader;
use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::path::PathBuf;

/// Antares - A classic turn-based RPG
#[derive(Parser, Debug)]
#[command(name = "antares")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Campaign to load
    #[arg(short, long)]
    campaign: Option<String>,

    /// List available campaigns
    #[arg(long)]
    list_campaigns: bool,

    /// Validate campaign without starting game
    #[arg(long)]
    validate_campaign: Option<String>,

    /// Continue from last save
    #[arg(long)]
    continue_game: bool,

    /// Load specific save file
    #[arg(short, long)]
    load: Option<String>,

    /// Campaigns directory (default: "campaigns")
    #[arg(long, default_value = "campaigns")]
    campaigns_dir: PathBuf,

    /// Saves directory (default: "saves")
    #[arg(long, default_value = "saves")]
    saves_dir: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Handle list campaigns command
    if cli.list_campaigns {
        return list_campaigns(&cli.campaigns_dir);
    }

    // Handle validate campaign command
    if let Some(campaign_id) = cli.validate_campaign {
        return validate_campaign(&cli.campaigns_dir, &campaign_id);
    }

    // Initialize save game manager
    let save_manager = SaveGameManager::new(&cli.saves_dir)?;

    // Initialize game state
    let game_state = if cli.continue_game {
        // Load last save
        load_last_save(&save_manager)?
    } else if let Some(save_name) = cli.load {
        // Load specific save
        println!("Loading save: {}", save_name);
        save_manager.load(&save_name)?
    } else if let Some(campaign_id) = cli.campaign {
        // Start new game with campaign
        println!("Loading campaign: {}", campaign_id);
        let campaign_loader = CampaignLoader::new(&cli.campaigns_dir);
        let campaign = campaign_loader.load_campaign(&campaign_id)?;
        println!("Campaign loaded: {} v{}", campaign.name, campaign.version);
        println!("Author: {}", campaign.author);
        println!("{}", campaign.description);
        println!();
        GameState::new_game(campaign)
    } else {
        // Start new game with core content
        println!("Starting new game with core content");
        GameState::new()
    };

    // Run game loop
    run_game(game_state, save_manager)?;

    Ok(())
}

/// Lists all available campaigns
fn list_campaigns(campaigns_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Available Campaigns:");
    println!();

    let campaign_loader = CampaignLoader::new(campaigns_dir);
    let campaigns = campaign_loader.list_campaigns()?;

    if campaigns.is_empty() {
        println!("  No campaigns found in {:?}", campaigns_dir);
        println!();
        println!("  Create campaigns using the Campaign Builder tool:");
        println!("  $ cargo run --bin campaign_builder");
        return Ok(());
    }

    for campaign in campaigns {
        println!(
            "  {} - {} v{}",
            campaign.id, campaign.name, campaign.version
        );
        println!("    Author: {}", campaign.author);
        println!("    {}", campaign.description);
        println!();
    }

    Ok(())
}

/// Validates a campaign
fn validate_campaign(
    campaigns_dir: &PathBuf,
    campaign_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Validating campaign: {}", campaign_id);
    println!();

    let campaign_loader = CampaignLoader::new(campaigns_dir);
    let report = campaign_loader.validate_campaign(campaign_id)?;

    println!("Campaign: {}", campaign_id);
    println!();

    // Display validation results
    println!("Validation Results:");
    println!("  Errors: {}", report.errors.len());
    println!("  Warnings: {}", report.warnings.len());
    println!();

    if !report.errors.is_empty() {
        println!("Errors:");
        for error in &report.errors {
            println!("  - {}", error);
        }
        println!();
    }

    if !report.warnings.is_empty() {
        println!("Warnings:");
        for warning in &report.warnings {
            println!("  - {}", warning);
        }
        println!();
    }

    if report.errors.is_empty() {
        println!("✓ Campaign is valid!");
        println!();
        println!("To play this campaign:");
        println!("  $ antares --campaign {}", campaign_id);
        Ok(())
    } else {
        Err("Campaign validation failed".into())
    }
}

/// Loads the most recent save game
fn load_last_save(save_manager: &SaveGameManager) -> Result<GameState, Box<dyn std::error::Error>> {
    let saves = save_manager.list_saves()?;

    if saves.is_empty() {
        return Err(
            "No saved games found. Start a new game with --campaign or without flags.".into(),
        );
    }

    // For simplicity, load the first save (alphabetically)
    // TODO: Track last played save or sort by modification time
    let save_name = &saves[0];
    println!("Loading last save: {}", save_name);
    Ok(save_manager.load(save_name)?)
}

/// Main game loop (simplified for Phase 14)
fn run_game(
    mut game_state: GameState,
    save_manager: SaveGameManager,
) -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("========================================");
    println!("           ANTARES RPG                  ");
    println!("========================================");
    println!();

    if let Some(ref campaign) = game_state.campaign {
        println!("Campaign: {} v{}", campaign.name, campaign.version);
    } else {
        println!("Core Content Mode");
    }

    println!();
    println!("Party Gold: {}", game_state.party.gold);
    println!("Party Food: {}", game_state.party.food);
    println!("Game Mode: {:?}", game_state.mode);
    println!(
        "Day: {}, Time: {}:{:02}",
        game_state.time.day, game_state.time.hour, game_state.time.minute
    );
    println!();

    // Interactive menu system
    let mut rl = DefaultEditor::new()?;

    println!("Available commands:");
    println!("  status  - Show game status");
    println!("  save    - Save game");
    println!("  load    - Load game");
    println!("  quit    - Quit game");
    println!();

    loop {
        let readline = rl.readline("antares> ");

        match readline {
            Ok(line) => {
                let line = line.trim();

                match line {
                    "status" | "s" => {
                        show_status(&game_state);
                    }
                    "save" => {
                        print!("Enter save name: ");
                        let save_line = rl.readline("")?;
                        let save_name = save_line.trim();
                        match save_manager.save(save_name, &game_state) {
                            Ok(_) => println!("Game saved: {}", save_name),
                            Err(e) => println!("Failed to save: {}", e),
                        }
                    }
                    "load" => {
                        let saves = save_manager.list_saves()?;
                        if saves.is_empty() {
                            println!("No saved games found");
                            continue;
                        }
                        println!("Available saves:");
                        for (i, save) in saves.iter().enumerate() {
                            println!("  {}. {}", i + 1, save);
                        }
                        print!("Enter save number: ");
                        let load_line = rl.readline("")?;
                        if let Ok(index) = load_line.trim().parse::<usize>() {
                            if index > 0 && index <= saves.len() {
                                let save_name = &saves[index - 1];
                                match save_manager.load(save_name) {
                                    Ok(loaded_state) => {
                                        game_state = loaded_state;
                                        println!("Game loaded: {}", save_name);
                                    }
                                    Err(e) => println!("Failed to load: {}", e),
                                }
                            } else {
                                println!("Invalid save number");
                            }
                        }
                    }
                    "quit" | "q" | "exit" => {
                        println!("Thanks for playing Antares!");
                        break;
                    }
                    "" => continue,
                    _ => {
                        println!("Unknown command: {}", line);
                        println!("Type 'status', 'save', 'load', or 'quit'");
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("^D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}

/// Shows current game status
fn show_status(game_state: &GameState) {
    println!();
    println!("=== Game Status ===");

    if let Some(ref campaign) = game_state.campaign {
        println!("Campaign: {} v{}", campaign.name, campaign.version);
        println!("Author: {}", campaign.author);
    } else {
        println!("Mode: Core Content");
    }

    println!();
    println!("Game Mode: {:?}", game_state.mode);
    println!(
        "Day: {}, Time: {}:{:02}",
        game_state.time.day, game_state.time.hour, game_state.time.minute
    );
    println!();
    println!("Party:");
    println!("  Members: {}", game_state.party.members.len());
    println!("  Gold: {}", game_state.party.gold);
    println!("  Food: {}", game_state.party.food);
    println!("  Gems: {}", game_state.party.gems);
    println!();
    println!("Roster:");
    println!("  Total Characters: {}", game_state.roster.characters.len());
    println!();
}
