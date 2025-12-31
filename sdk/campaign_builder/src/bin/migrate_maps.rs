// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Migration tool to remove event_trigger from map RON files
//!
//! This tool processes RON map files and removes all `event_trigger` fields from tiles.
//! The event system now uses the canonical `Map.events` HashMap exclusively.
//!
//! # Usage
//!
//! ```bash
//! cargo run --bin migrate_maps -- <map_file_path>
//! ```
//!
//! # Examples
//!
//! Migrate a single map:
//! ```bash
//! cargo run --bin migrate_maps -- ../../campaigns/tutorial/data/maps/map_1.ron
//! ```
//!
//! Migrate all maps in a directory:
//! ```bash
//! for f in ../../campaigns/tutorial/data/maps/*.ron; do
//!     cargo run --bin migrate_maps -- "$f"
//! done
//! ```

use clap::Parser;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to map RON file to migrate
    map_file: PathBuf,

    /// Skip backup creation (not recommended)
    #[arg(long)]
    no_backup: bool,

    /// Dry run - show changes without writing
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if !args.map_file.exists() {
        eprintln!("Error: File does not exist: {}", args.map_file.display());
        std::process::exit(1);
    }

    if args.map_file.extension().and_then(|s| s.to_str()) != Some("ron") {
        eprintln!("Error: File must have .ron extension");
        std::process::exit(1);
    }

    println!("Migrating map file: {}", args.map_file.display());

    // Read file
    let content = fs::read_to_string(&args.map_file)?;

    // Count event_trigger occurrences before migration
    let event_trigger_count = content
        .lines()
        .filter(|line| line.trim().starts_with("event_trigger:"))
        .count();

    if event_trigger_count == 0 {
        println!("✓ File already migrated (no event_trigger fields found)");
        return Ok(());
    }

    println!(
        "Found {} event_trigger fields to remove",
        event_trigger_count
    );

    // Remove event_trigger lines
    let migrated = content
        .lines()
        .filter(|line| !line.trim().starts_with("event_trigger:"))
        .collect::<Vec<_>>()
        .join("\n");

    // Add final newline if original had one
    let migrated = if content.ends_with('\n') {
        format!("{}\n", migrated)
    } else {
        migrated
    };

    if args.dry_run {
        println!("\n--- DRY RUN MODE ---");
        println!("Would remove {} lines", event_trigger_count);
        println!("Original size: {} bytes", content.len());
        println!("Migrated size: {} bytes", migrated.len());
        println!("Size reduction: {} bytes", content.len() - migrated.len());
        return Ok(());
    }

    // Create backup unless explicitly skipped
    if !args.no_backup {
        let backup_path = args.map_file.with_extension("ron.backup");
        fs::copy(&args.map_file, &backup_path)?;
        println!("✓ Backup created: {}", backup_path.display());
    }

    // Write migrated content
    fs::write(&args.map_file, migrated)?;
    println!("✓ Migration complete!");
    println!("  Removed {} event_trigger fields", event_trigger_count);
    println!(
        "  Size reduction: {} bytes",
        content.len() - fs::read_to_string(&args.map_file)?.len()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_migration_removes_event_trigger_lines() {
        let tmpdir = tempdir().expect("Failed to create tempdir");
        let map_file = tmpdir.path().join("test_map.ron");

        // Create a test map file with event_trigger fields
        let test_content = r#"Map(
    id: 1,
    name: "Test Map",
    tiles: [
        Tile(
            x: 0,
            y: 0,
            terrain: Grass,
            event_trigger: None,
        ),
        Tile(
            x: 1,
            y: 0,
            terrain: Grass,
            event_trigger: Some(42),
        ),
    ],
    events: [],
)
"#;
        fs::write(&map_file, test_content).expect("Failed to write test file");

        // Run migration (simulated - we'll do it directly)
        let content = fs::read_to_string(&map_file).unwrap();
        let migrated = content
            .lines()
            .filter(|line| !line.trim().starts_with("event_trigger:"))
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(&map_file, migrated).unwrap();

        // Verify no event_trigger lines remain
        let result = fs::read_to_string(&map_file).unwrap();
        assert!(!result.contains("event_trigger:"));
        assert!(result.contains("Tile("));
        assert!(result.contains("x: 0,"));
        assert!(result.contains("y: 0,"));
    }

    #[test]
    fn test_migration_preserves_other_content() {
        let tmpdir = tempdir().expect("Failed to create tempdir");
        let map_file = tmpdir.path().join("test_map.ron");

        let test_content = r#"Map(
    id: 1,
    name: "Test Map",
    description: "A test map",
    width: 10,
    height: 10,
    tiles: [
        Tile(
            x: 0,
            y: 0,
            terrain: Grass,
            event_trigger: None,
            blocked: false,
        ),
    ],
    events: [
        MapEvent(
            position: Position(x: 5, y: 5),
            event_type: Text("Hello"),
        ),
    ],
)
"#;
        fs::write(&map_file, test_content).expect("Failed to write test file");

        // Run migration
        let content = fs::read_to_string(&map_file).unwrap();
        let migrated = content
            .lines()
            .filter(|line| !line.trim().starts_with("event_trigger:"))
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(&map_file, migrated).unwrap();

        // Verify structure preserved
        let result = fs::read_to_string(&map_file).unwrap();
        assert!(!result.contains("event_trigger:"));
        assert!(result.contains("id: 1,"));
        assert!(result.contains("name: \"Test Map\","));
        assert!(result.contains("description: \"A test map\","));
        assert!(result.contains("events: ["));
        assert!(result.contains("position: Position(x: 5, y: 5)"));
    }
}
