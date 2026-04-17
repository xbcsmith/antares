// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! `antares-sdk map validate` — RON map file validator.
//!
//! Migrated from `src/bin/validate_map.rs`. Exposes [`run`] as the single
//! entry point called by `src/bin/antares_sdk.rs`.
//!
//! # Subcommands
//!
//! | Subcommand | Description                                 |
//! |------------|---------------------------------------------|
//! | `validate` | Validate one or more RON map files          |
//!
//! # Dynamic ID loading
//!
//! When `--campaign-dir` is provided, valid monster and item IDs are loaded
//! at runtime from `<campaign-dir>/data/monsters.ron` and
//! `<campaign-dir>/data/items.ron`. If the flag is omitted, ID validation is
//! skipped entirely and a warning is printed to stderr.
//!
//! # Examples
//!
//! ```no_run
//! use antares::sdk::cli::map_validator::{run, MapArgs, MapSubcommand, MapValidateArgs};
//! use std::path::PathBuf;
//!
//! let args = MapArgs {
//!     command: MapSubcommand::Validate(MapValidateArgs {
//!         files: vec![PathBuf::from("campaigns/tutorial/data/maps/map_1.ron")],
//!         campaign_dir: Some(PathBuf::from("campaigns/tutorial")),
//!     }),
//! };
//! // run(args).unwrap();
//! ```

use crate::domain::items::ItemDatabase;
use crate::domain::types::Position;
use crate::domain::world::{Map, MapBlueprint, MapEvent};
use crate::sdk::database::MonsterDatabase;
use clap::{Args, Subcommand};
use std::path::{Path, PathBuf};

// ──────────────────────────────────────────────────────────────────────────────
// CLI argument structs
// ──────────────────────────────────────────────────────────────────────────────

/// Arguments for the `antares-sdk map` subcommand group.
///
/// This struct acts as the dispatcher for nested map subcommands. Pass it
/// to [`run`] to execute the chosen subcommand.
#[derive(Args, Debug)]
#[command(about = "Map creation and validation tools")]
pub struct MapArgs {
    /// The map subcommand to execute.
    #[command(subcommand)]
    pub command: MapSubcommand,
}

/// Available subcommands under `antares-sdk map`.
#[derive(Subcommand, Debug)]
pub enum MapSubcommand {
    /// Interactive map builder REPL with command history.
    Build,
    /// Validate one or more RON map files for correctness and gameplay constraints.
    Validate(MapValidateArgs),
}

/// Arguments for `antares-sdk map validate`.
///
/// Accepts one or more RON map file paths and an optional campaign directory
/// for dynamic ID validation.
#[derive(Args, Debug)]
#[command(
    about = "Validate RON map files",
    long_about = "Validates RON map files for structural integrity, content validity,\n\
                  and gameplay constraints.\n\n\
                  When --campaign-dir is provided, monster and item IDs are validated\n\
                  against the campaign's data files. Without it, ID checks are skipped."
)]
pub struct MapValidateArgs {
    /// One or more RON map files to validate.
    #[arg(value_name = "MAP_FILE", required = true)]
    pub files: Vec<PathBuf>,

    /// Campaign data directory used for dynamic monster/item ID loading.
    ///
    /// The validator will look for `<campaign-dir>/data/monsters.ron` and
    /// `<campaign-dir>/data/items.ron`. When omitted, ID validation is
    /// skipped with a warning printed to stderr.
    #[arg(short = 'c', long, value_name = "DIR")]
    pub campaign_dir: Option<PathBuf>,
}

// ──────────────────────────────────────────────────────────────────────────────
// Public entry point
// ──────────────────────────────────────────────────────────────────────────────

/// Run the `map` subcommand group with the given arguments.
///
/// Dispatches to the appropriate handler based on [`MapSubcommand`].
///
/// # Errors
///
/// Returns `Err` if argument resolution fails. Validation failures are
/// signalled via [`std::process::exit(1)`] to preserve identical exit-code
/// semantics with the original `validate_map` binary.
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::cli::map_validator::{run, MapArgs, MapSubcommand, MapValidateArgs};
/// use std::path::PathBuf;
///
/// let args = MapArgs {
///     command: MapSubcommand::Validate(MapValidateArgs {
///         files: vec![PathBuf::from("campaigns/tutorial/data/maps/map_1.ron")],
///         campaign_dir: None,
///     }),
/// };
/// // run(args).unwrap();
/// ```
pub fn run(args: MapArgs) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        MapSubcommand::Build => crate::sdk::cli::map_builder::run_build(),
        MapSubcommand::Validate(v) => run_validate(v),
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Validate subcommand implementation
// ──────────────────────────────────────────────────────────────────────────────

/// Execute `antares-sdk map validate`.
///
/// Exits the process with code 1 if any file fails validation.
fn run_validate(args: MapValidateArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Load IDs dynamically when campaign-dir is provided; skip otherwise.
    let (valid_monster_ids, valid_item_ids) = load_ids(args.campaign_dir.as_deref());

    let mut total_files: usize = 0;
    let mut valid_files: usize = 0;
    let mut failed_files: usize = 0;

    for file_path in &args.files {
        total_files += 1;
        println!("\n{:=<70}", "");
        println!("Validating: {}", file_path.display());
        println!("{:-<70}", "");

        match validate_map_file(
            file_path,
            valid_monster_ids.as_deref(),
            valid_item_ids.as_deref(),
        ) {
            Ok(map) => {
                valid_files += 1;
                println!("✅ VALID");
                print_map_summary(&map);
            }
            Err(errors) => {
                failed_files += 1;
                println!("❌ INVALID ({} errors)", errors.len());
                println!("\nErrors:");
                for error in errors {
                    println!("  • {}", error);
                }
            }
        }
    }

    println!("\n{:=<70}", "");
    println!("Validation Summary:");
    println!("  Total files: {}", total_files);
    println!("  Valid:       {}", valid_files);
    println!("  Failed:      {}", failed_files);
    println!("{:=<70}", "");

    if failed_files > 0 {
        std::process::exit(1);
    }

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Dynamic ID loading
// ──────────────────────────────────────────────────────────────────────────────

/// Load valid monster and item IDs from the campaign data directory.
///
/// Returns `(None, None)` with a warning when `campaign_dir` is `None`.
/// Returns `(Some(ids), Some(ids))` on success, falling back to an empty
/// `Vec` (which prints a warning) when the individual data files are missing
/// or malformed.
fn load_ids(campaign_dir: Option<&Path>) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
    let Some(dir) = campaign_dir else {
        eprintln!(
            "Warning: --campaign-dir not provided; \
             monster and item ID validation will be skipped."
        );
        return (None, None);
    };

    let monster_ids = load_monster_ids_from(dir);
    let item_ids = load_item_ids_from(dir);
    (Some(monster_ids), Some(item_ids))
}

/// Load valid monster IDs from `<campaign_dir>/data/monsters.ron`.
///
/// Returns an empty `Vec` (with a warning) when the file cannot be loaded.
fn load_monster_ids_from(campaign_dir: &Path) -> Vec<u8> {
    let path = campaign_dir.join("data").join("monsters.ron");

    match MonsterDatabase::load_from_file(&path) {
        Ok(db) => {
            let ids = db.all_monsters();
            if ids.is_empty() {
                eprintln!(
                    "Warning: {} loaded but contains no monsters; ID validation may be incomplete.",
                    path.display()
                );
            } else {
                println!("Loaded {} monster IDs from {}", ids.len(), path.display());
            }
            ids
        }
        Err(e) => {
            eprintln!(
                "Warning: Could not load {}: {}. Monster ID validation will be skipped.",
                path.display(),
                e
            );
            Vec::new()
        }
    }
}

/// Load valid item IDs from `<campaign_dir>/data/items.ron`.
///
/// Returns an empty `Vec` (with a warning) when the file cannot be loaded.
fn load_item_ids_from(campaign_dir: &Path) -> Vec<u8> {
    let path = campaign_dir.join("data").join("items.ron");

    match ItemDatabase::load_from_file(&path) {
        Ok(db) => {
            let ids: Vec<u8> = db.all_items().iter().map(|item| item.id).collect();
            if ids.is_empty() {
                eprintln!(
                    "Warning: {} loaded but contains no items; ID validation may be incomplete.",
                    path.display()
                );
            } else {
                println!("Loaded {} item IDs from {}", ids.len(), path.display());
            }
            ids
        }
        Err(e) => {
            eprintln!(
                "Warning: Could not load {}: {}. Item ID validation will be skipped.",
                path.display(),
                e
            );
            Vec::new()
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Core validation logic
// ──────────────────────────────────────────────────────────────────────────────

/// Validate a single RON map file.
///
/// Returns the parsed [`Map`] on success or a `Vec` of error strings when
/// validation fails. `valid_monster_ids` and `valid_item_ids` are `None` when
/// the respective ID checks should be skipped.
fn validate_map_file(
    file_path: &Path,
    valid_monster_ids: Option<&[u8]>,
    valid_item_ids: Option<&[u8]>,
) -> Result<Map, Vec<String>> {
    let mut errors = Vec::new();

    // Read file
    let contents = match std::fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            errors.push(format!("Failed to read file: {}", e));
            return Err(errors);
        }
    };

    // Parse RON
    let blueprint: MapBlueprint = match ron::from_str(&contents) {
        Ok(bp) => bp,
        Err(e) => {
            errors.push(format!("Failed to parse RON: {}", e));
            return Err(errors);
        }
    };

    let map: Map = blueprint.into();

    // Validate structure
    validate_structure(&map, &mut errors);

    // Validate content (optional ID checks)
    validate_content(&map, &mut errors, valid_monster_ids, valid_item_ids);

    // Validate gameplay constraints
    validate_gameplay(&map, &mut errors);

    if errors.is_empty() {
        Ok(map)
    } else {
        Err(errors)
    }
}

/// Validate map structure: dimensions and tile array integrity.
fn validate_structure(map: &Map, errors: &mut Vec<String>) {
    if map.id == 0 {
        errors.push("Map ID must be non-zero (valid range: 1-65535)".to_string());
    }

    if map.width == 0 || map.height == 0 {
        errors.push(format!(
            "Map dimensions must be > 0 (got {}x{})",
            map.width, map.height
        ));
    }

    if map.width > 256 || map.height > 256 {
        errors.push(format!(
            "Map dimensions exceed maximum (got {}x{}, max 256x256)",
            map.width, map.height
        ));
    }

    let expected_tiles = (map.width * map.height) as usize;
    if map.tiles.len() != expected_tiles {
        errors.push(format!(
            "Tile count mismatch: expected {}, found {}",
            expected_tiles,
            map.tiles.len()
        ));
    }
}

/// Validate map content: event positions and optional ID checks.
///
/// When `valid_monster_ids` or `valid_item_ids` is `None`, the respective
/// ID validation step is silently skipped (the caller already printed a
/// warning when IDs could not be loaded).
fn validate_content(
    map: &Map,
    errors: &mut Vec<String>,
    valid_monster_ids: Option<&[u8]>,
    valid_item_ids: Option<&[u8]>,
) {
    for (pos, event) in &map.events {
        // Position bounds check
        if !is_position_valid(pos, map.width, map.height) {
            errors.push(format!(
                "Event at ({}, {}) out of bounds (map: {}x{})",
                pos.x, pos.y, map.width, map.height
            ));
        }

        // Monster ID validation (only when IDs were loaded)
        if let (MapEvent::Encounter { monster_group, .. }, Some(ids)) = (event, valid_monster_ids) {
            for &monster_id in monster_group {
                if !ids.contains(&monster_id) {
                    errors.push(format!(
                        "Invalid Monster ID {} at ({}, {}). Valid IDs: {:?}",
                        monster_id, pos.x, pos.y, ids
                    ));
                }
            }
        }

        // Item ID validation (only when IDs were loaded)
        if let (MapEvent::Treasure { loot, .. }, Some(ids)) = (event, valid_item_ids) {
            for &item_id in loot {
                if !ids.contains(&item_id) {
                    errors.push(format!(
                        "Invalid Item ID {} at ({}, {}). Check data/items.ron",
                        item_id, pos.x, pos.y
                    ));
                }
            }
        }
    }

    // NPC placement bounds checks
    for placement in &map.npc_placements {
        if !is_position_valid(&placement.position, map.width, map.height) {
            errors.push(format!(
                "NPC placement '{}' at ({}, {}) out of bounds (map: {}x{})",
                placement.npc_id, placement.position.x, placement.position.y, map.width, map.height
            ));
        }
    }
}

/// Validate gameplay constraints: overlapping events and empty maps.
fn validate_gameplay(map: &Map, errors: &mut Vec<String>) {
    let mut occupied_positions = std::collections::HashSet::new();

    for pos in map.events.keys() {
        if !occupied_positions.insert((pos.x, pos.y)) {
            errors.push(format!(
                "Multiple events at same position ({}, {})",
                pos.x, pos.y
            ));
        }
    }

    for placement in &map.npc_placements {
        let pos_tuple = (placement.position.x, placement.position.y);
        if occupied_positions.contains(&pos_tuple) {
            errors.push(format!(
                "NPC placement '{}' overlaps with event at ({}, {})",
                placement.npc_id, placement.position.x, placement.position.y
            ));
        }
    }

    if map.events.is_empty() && map.npc_placements.is_empty() {
        errors.push("Map has no events or NPC placements (empty map)".to_string());
    }
}

/// Return `true` if `pos` falls within the map bounds.
fn is_position_valid(pos: &Position, width: u32, height: u32) -> bool {
    pos.x >= 0 && pos.x < width as i32 && pos.y >= 0 && pos.y < height as i32
}

// ──────────────────────────────────────────────────────────────────────────────
// Map summary printer
// ──────────────────────────────────────────────────────────────────────────────

/// Print a human-readable summary of a validated map.
///
/// Uses a separate named counter for **every** [`MapEvent`] variant so that
/// no event type is silently lumped into an unrelated bucket.
fn print_map_summary(map: &Map) {
    println!("\nMap Summary:");
    println!("  ID:          {}", map.id);
    println!("  Size:        {}x{}", map.width, map.height);
    println!("  Total tiles: {}", map.width * map.height);
    println!("  Events:      {}", map.events.len());
    println!("  NPC placements: {}", map.npc_placements.len());

    // One named counter per MapEvent variant — no implicit lumping.
    let mut encounters: u32 = 0;
    let mut treasures: u32 = 0;
    let mut teleports: u32 = 0;
    let mut traps: u32 = 0;
    let mut signs: u32 = 0;
    let mut npc_dialogues: u32 = 0;
    let mut recruitable_characters: u32 = 0;
    let mut inns: u32 = 0;
    let mut furniture: u32 = 0;
    let mut containers: u32 = 0;
    let mut dropped_items: u32 = 0;
    let mut locked_doors: u32 = 0;
    let mut locked_containers: u32 = 0;

    for event in map.events.values() {
        match event {
            MapEvent::Encounter { .. } => encounters += 1,
            MapEvent::Treasure { .. } => treasures += 1,
            MapEvent::Teleport { .. } => teleports += 1,
            MapEvent::Trap { .. } => traps += 1,
            MapEvent::Sign { .. } => signs += 1,
            MapEvent::NpcDialogue { .. } => npc_dialogues += 1,
            MapEvent::RecruitableCharacter { .. } => recruitable_characters += 1,
            MapEvent::EnterInn { .. } => inns += 1,
            MapEvent::Furniture { .. } => furniture += 1,
            MapEvent::Container { .. } => containers += 1,
            MapEvent::DroppedItem { .. } => dropped_items += 1,
            MapEvent::LockedDoor { .. } => locked_doors += 1,
            MapEvent::LockedContainer { .. } => locked_containers += 1,
        }
    }

    if !map.events.is_empty() {
        println!("\n  Event Breakdown:");
        print_count("Encounters", encounters);
        print_count("Treasures", treasures);
        print_count("Teleports", teleports);
        print_count("Traps", traps);
        print_count("Signs", signs);
        print_count("NPC Dialogues", npc_dialogues);
        print_count("Recruitable Characters", recruitable_characters);
        print_count("Inn Entrances", inns);
        print_count("Furniture", furniture);
        print_count("Containers", containers);
        print_count("Dropped Items", dropped_items);
        print_count("Locked Doors", locked_doors);
        print_count("Locked Containers", locked_containers);
    }

    if !map.npc_placements.is_empty() {
        println!("\n  NPC Placements:");
        for placement in &map.npc_placements {
            println!(
                "    - {} at ({}, {})",
                placement.npc_id, placement.position.x, placement.position.y
            );
        }
    }
}

/// Print a single event-type count line, but only when the count is non-zero.
#[inline]
fn print_count(label: &str, count: u32) {
    if count > 0 {
        println!("    - {}: {}", label, count);
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    // ── CLI parsing ────────────────────────────────────────────────────────────

    /// A `MapArgs` wrapping `MapSubcommand::Validate` must be constructable
    /// from the [`Args`] derived impl — this exercises the public API.
    #[test]
    fn test_map_args_validate_subcommand_fields() {
        let args = MapArgs {
            command: MapSubcommand::Validate(MapValidateArgs {
                files: vec![PathBuf::from("map_1.ron"), PathBuf::from("map_2.ron")],
                campaign_dir: Some(PathBuf::from("campaigns/tutorial")),
            }),
        };
        match args.command {
            MapSubcommand::Build => panic!("expected Validate, got Build"),
            MapSubcommand::Validate(v) => {
                assert_eq!(v.files.len(), 2);
                assert_eq!(v.campaign_dir, Some(PathBuf::from("campaigns/tutorial")));
            }
        }
    }

    /// `antares-sdk map validate map_1.ron` must parse correctly without
    /// `--campaign-dir`.
    #[test]
    fn test_map_validate_args_without_campaign_dir() {
        let args = MapValidateArgs {
            files: vec![PathBuf::from("map_1.ron")],
            campaign_dir: None,
        };
        assert!(args.campaign_dir.is_none());
        assert_eq!(args.files, vec![PathBuf::from("map_1.ron")]);
    }

    /// `--campaign-dir` flag must be parsed into `campaign_dir`.
    #[test]
    fn test_map_validate_args_with_campaign_dir() {
        let args = MapValidateArgs {
            files: vec![PathBuf::from("map_1.ron")],
            campaign_dir: Some(PathBuf::from("campaigns/tutorial")),
        };
        assert_eq!(args.campaign_dir, Some(PathBuf::from("campaigns/tutorial")));
        assert_eq!(args.files, vec![PathBuf::from("map_1.ron")]);
    }

    /// Multiple file arguments must all be captured.
    #[test]
    fn test_map_validate_args_multiple_files() {
        let args = MapValidateArgs {
            files: vec![
                PathBuf::from("map_1.ron"),
                PathBuf::from("map_2.ron"),
                PathBuf::from("map_3.ron"),
            ],
            campaign_dir: None,
        };
        assert_eq!(args.files.len(), 3);
        assert!(args.campaign_dir.is_none());
    }

    // ── load_ids ───────────────────────────────────────────────────────────────

    /// When `campaign_dir` is `None`, `load_ids` must return `(None, None)`.
    #[test]
    fn test_load_ids_without_campaign_dir_returns_none() {
        let (monsters, items) = load_ids(None);
        assert!(
            monsters.is_none(),
            "monster IDs should be None when no dir given"
        );
        assert!(items.is_none(), "item IDs should be None when no dir given");
    }

    /// When `campaign_dir` points to a directory without data files, both
    /// vecs must be present but empty (not `None`).
    #[test]
    fn test_load_ids_with_missing_data_files_returns_empty_vecs() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let (monsters, items) = load_ids(Some(tmp.path()));
        // We get Some(vec) because a dir was provided, even though files are missing.
        assert!(
            monsters.is_some(),
            "monster IDs should be Some when dir given"
        );
        assert!(items.is_some(), "item IDs should be Some when dir given");
        assert!(
            monsters.unwrap().is_empty(),
            "monster IDs should be empty when file missing"
        );
        assert!(
            items.unwrap().is_empty(),
            "item IDs should be empty when file missing"
        );
    }

    /// When `campaign_dir` points to the test campaign fixture, both vecs
    /// must be non-empty.
    #[test]
    fn test_load_ids_with_test_campaign_returns_ids() {
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let campaign = manifest.join("data/test_campaign");

        if !campaign.exists() {
            // Skip gracefully if the fixture is not present.
            return;
        }

        let (monsters, items) = load_ids(Some(&campaign));
        let monsters = monsters.expect("monster IDs should be Some");
        let items = items.expect("item IDs should be Some");
        assert!(!monsters.is_empty(), "test campaign should have monsters");
        assert!(!items.is_empty(), "test campaign should have items");
    }

    // ── validate_structure ────────────────────────────────────────────────────

    /// A map with ID 0 must produce an error.
    #[test]
    fn test_validate_structure_zero_id() {
        let mut map = make_minimal_map();
        map.id = 0;
        let mut errors = Vec::new();
        validate_structure(&map, &mut errors);
        assert!(
            errors.iter().any(|e| e.contains("Map ID must be non-zero")),
            "expected zero-ID error; got: {:?}",
            errors
        );
    }

    /// A map with zero width must produce an error.
    #[test]
    fn test_validate_structure_zero_width() {
        let mut map = make_minimal_map();
        map.width = 0;
        // Tile count will also be wrong, but the dimension check fires first.
        let mut errors = Vec::new();
        validate_structure(&map, &mut errors);
        assert!(
            errors.iter().any(|e| e.contains("dimensions must be > 0")),
            "expected zero-dimension error; got: {:?}",
            errors
        );
    }

    /// A map that exceeds 256×256 must produce an error.
    #[test]
    fn test_validate_structure_oversized_map() {
        let mut map = make_minimal_map();
        map.width = 300;
        map.height = 300;
        map.tiles = (0..(300 * 300))
            .map(|i| {
                let xi = i % 300;
                let yi = i / 300;
                crate::domain::world::Tile::new(
                    xi,
                    yi,
                    crate::domain::world::TerrainType::Grass,
                    crate::domain::world::WallType::None,
                )
            })
            .collect();
        let mut errors = Vec::new();
        validate_structure(&map, &mut errors);
        assert!(
            errors.iter().any(|e| e.contains("exceed maximum")),
            "expected oversized error; got: {:?}",
            errors
        );
    }

    /// A valid minimal map must produce no structural errors.
    #[test]
    fn test_validate_structure_valid_map_no_errors() {
        let map = make_minimal_map();
        let mut errors = Vec::new();
        validate_structure(&map, &mut errors);
        assert!(
            errors.is_empty(),
            "expected no errors for valid map; got: {:?}",
            errors
        );
    }

    // ── validate_content ──────────────────────────────────────────────────────

    /// An event at a position outside the map bounds must produce an error.
    #[test]
    fn test_validate_content_event_out_of_bounds() {
        let mut map = make_minimal_map();
        map.events.insert(
            Position { x: 99, y: 99 },
            MapEvent::Sign {
                name: String::new(),
                description: String::new(),
                text: "oob".into(),
                time_condition: None,
                facing: None,
            },
        );
        let mut errors = Vec::new();
        validate_content(&map, &mut errors, None, None);
        assert!(
            errors.iter().any(|e| e.contains("out of bounds")),
            "expected out-of-bounds error; got: {:?}",
            errors
        );
    }

    /// An encounter with an invalid monster ID must produce an error when IDs
    /// are provided.
    #[test]
    fn test_validate_content_invalid_monster_id() {
        let mut map = make_minimal_map();
        map.events.insert(
            Position { x: 0, y: 0 },
            MapEvent::Encounter {
                name: String::new(),
                description: String::new(),
                monster_group: vec![99],
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
                combat_event_type: Default::default(),
            },
        );
        let valid_ids: &[u8] = &[1, 2, 3];
        let mut errors = Vec::new();
        validate_content(&map, &mut errors, Some(valid_ids), None);
        assert!(
            errors.iter().any(|e| e.contains("Invalid Monster ID")),
            "expected invalid monster ID error; got: {:?}",
            errors
        );
    }

    /// When `valid_monster_ids` is `None`, invalid monster IDs must NOT
    /// produce an error (ID validation is skipped).
    #[test]
    fn test_validate_content_skips_monster_id_check_when_ids_none() {
        let mut map = make_minimal_map();
        map.events.insert(
            Position { x: 0, y: 0 },
            MapEvent::Encounter {
                name: String::new(),
                description: String::new(),
                monster_group: vec![255],
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
                combat_event_type: Default::default(),
            },
        );
        let mut errors = Vec::new();
        validate_content(&map, &mut errors, None, None);
        // No invalid-monster-ID error because IDs are None (skipped).
        assert!(
            errors.iter().all(|e| !e.contains("Invalid Monster ID")),
            "monster ID check should be skipped when ids are None; got: {:?}",
            errors
        );
    }

    /// An item ID outside the valid set must produce an error when IDs are
    /// provided.
    #[test]
    fn test_validate_content_invalid_item_id() {
        let mut map = make_minimal_map();
        map.events.insert(
            Position { x: 0, y: 0 },
            MapEvent::Treasure {
                name: String::new(),
                description: String::new(),
                loot: vec![200],
            },
        );
        let valid_ids: &[u8] = &[1, 2, 3];
        let mut errors = Vec::new();
        validate_content(&map, &mut errors, None, Some(valid_ids));
        assert!(
            errors.iter().any(|e| e.contains("Invalid Item ID")),
            "expected invalid item ID error; got: {:?}",
            errors
        );
    }

    // ── validate_gameplay ─────────────────────────────────────────────────────

    /// An empty map (no events, no NPC placements) must produce a warning
    /// error.
    #[test]
    fn test_validate_gameplay_empty_map_error() {
        let map = make_minimal_map();
        let mut errors = Vec::new();
        validate_gameplay(&map, &mut errors);
        assert!(
            errors.iter().any(|e| e.contains("empty map")),
            "expected empty-map error; got: {:?}",
            errors
        );
    }

    /// A map with at least one event must NOT produce the empty-map error.
    #[test]
    fn test_validate_gameplay_non_empty_map_no_error() {
        let mut map = make_minimal_map();
        map.events.insert(
            Position { x: 0, y: 0 },
            MapEvent::Sign {
                name: String::new(),
                description: String::new(),
                text: "hello".into(),
                time_condition: None,
                facing: None,
            },
        );
        let mut errors = Vec::new();
        validate_gameplay(&map, &mut errors);
        assert!(
            errors.iter().all(|e| !e.contains("empty map")),
            "non-empty map should not raise empty-map error; got: {:?}",
            errors
        );
    }

    // ── print_map_summary (smoke) ──────────────────────────────────────────────

    /// `print_map_summary` must not panic on a map containing one of every
    /// event variant.
    #[test]
    fn test_print_map_summary_does_not_panic_for_all_event_variants() {
        let map = make_map_with_all_event_variants();
        // Just ensure this doesn't panic.
        print_map_summary(&map);
    }

    // ── is_position_valid ─────────────────────────────────────────────────────

    #[test]
    fn test_is_position_valid_inside_bounds() {
        assert!(is_position_valid(&Position { x: 0, y: 0 }, 10, 10));
        assert!(is_position_valid(&Position { x: 9, y: 9 }, 10, 10));
        assert!(is_position_valid(&Position { x: 5, y: 5 }, 10, 10));
    }

    #[test]
    fn test_is_position_valid_outside_bounds() {
        assert!(!is_position_valid(&Position { x: 10, y: 0 }, 10, 10));
        assert!(!is_position_valid(&Position { x: 0, y: 10 }, 10, 10));
        assert!(!is_position_valid(&Position { x: -1, y: 0 }, 10, 10));
        assert!(!is_position_valid(&Position { x: 0, y: -1 }, 10, 10));
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    /// Build the smallest valid map: 2×2, 4 tiles, non-zero ID, no events.
    fn make_minimal_map() -> Map {
        use crate::domain::world::{TerrainType, Tile, WallType};
        use std::collections::BTreeMap;

        let tiles = (0..4)
            .map(|i| Tile::new(i % 2, i / 2, TerrainType::Grass, WallType::None))
            .collect();

        Map {
            id: 1,
            width: 2,
            height: 2,
            name: "Test Map".to_string(),
            description: String::new(),
            tiles,
            events: BTreeMap::new(),
            encounter_table: None,
            allow_random_encounters: false,
            npc_placements: Vec::new(),
            dropped_items: Vec::new(),
            lock_states: std::collections::HashMap::new(),
        }
    }

    /// Build a map that contains one instance of every MapEvent variant, all
    /// at valid (distinct) positions within a 16×16 grid.
    fn make_map_with_all_event_variants() -> Map {
        use crate::domain::world::{TerrainType, Tile, WallType};
        use std::collections::BTreeMap;

        let size: u32 = 16;
        let tiles: Vec<Tile> = (0..(size * size) as usize)
            .map(|i| {
                let xi = (i as u32 % size) as i32;
                let yi = (i as u32 / size) as i32;
                Tile::new(xi, yi, TerrainType::Grass, WallType::None)
            })
            .collect();

        let mut events: BTreeMap<Position, MapEvent> = BTreeMap::new();
        let mut x: i32 = 0;
        let mut next_pos = || {
            let pos = Position { x, y: 0 };
            x += 1;
            pos
        };

        events.insert(
            next_pos(),
            MapEvent::Encounter {
                name: String::new(),
                description: String::new(),
                monster_group: vec![],
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
                combat_event_type: Default::default(),
            },
        );
        events.insert(
            next_pos(),
            MapEvent::Treasure {
                name: String::new(),
                description: String::new(),
                loot: vec![],
            },
        );
        events.insert(
            next_pos(),
            MapEvent::Teleport {
                name: String::new(),
                description: String::new(),
                destination: Position { x: 0, y: 0 },
                map_id: 1,
            },
        );
        events.insert(
            next_pos(),
            MapEvent::Trap {
                name: String::new(),
                description: String::new(),
                damage: 5,
                effect: None,
            },
        );
        events.insert(
            next_pos(),
            MapEvent::Sign {
                name: String::new(),
                description: String::new(),
                text: "A sign".into(),
                time_condition: None,
                facing: None,
            },
        );
        events.insert(
            next_pos(),
            MapEvent::NpcDialogue {
                name: String::new(),
                description: String::new(),
                npc_id: "npc_1".into(),
                time_condition: None,
                facing: None,
                proximity_facing: false,
                rotation_speed: None,
            },
        );
        events.insert(
            next_pos(),
            MapEvent::RecruitableCharacter {
                name: String::new(),
                description: String::new(),
                character_id: "char_1".into(),
                dialogue_id: None,
                time_condition: None,
                facing: None,
            },
        );
        events.insert(
            next_pos(),
            MapEvent::EnterInn {
                name: String::new(),
                description: String::new(),
                innkeeper_id: "innkeeper_1".into(),
            },
        );
        events.insert(
            next_pos(),
            MapEvent::Furniture {
                name: String::new(),
                furniture_id: None,
                furniture_type: crate::domain::world::FurnitureType::Table,
                rotation_y: None,
                scale: 1.0,
                material: Default::default(),
                flags: Default::default(),
                color_tint: None,
                key_item_id: None,
            },
        );
        events.insert(
            next_pos(),
            MapEvent::Container {
                id: "chest_1".into(),
                name: String::new(),
                description: String::new(),
                items: vec![],
                gold: 0,
                gems: 0,
            },
        );
        events.insert(
            next_pos(),
            MapEvent::DroppedItem {
                name: String::new(),
                item_id: 1,
                charges: 0,
            },
        );
        events.insert(
            next_pos(),
            MapEvent::LockedDoor {
                name: String::new(),
                lock_id: "door_1".into(),
                key_item_id: None,
                initial_trap_chance: 0,
            },
        );
        events.insert(
            next_pos(),
            MapEvent::LockedContainer {
                name: String::new(),
                lock_id: "chest_locked_1".into(),
                key_item_id: None,
                items: vec![],
                initial_trap_chance: 0,
            },
        );

        Map {
            id: 1,
            width: size,
            height: size,
            name: "All Events Map".to_string(),
            description: String::new(),
            tiles,
            events,
            encounter_table: None,
            allow_random_encounters: false,
            npc_placements: Vec::new(),
            dropped_items: Vec::new(),
            lock_states: std::collections::HashMap::new(),
        }
    }
}
