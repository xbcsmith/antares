// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Race Editor CLI
//!
//! Interactive command-line tool for creating and editing character race definitions.
//! Supports loading, editing, and saving race data in RON format.
//!
//! # Usage
//!
//! ```bash
//! # Create/edit races in default location
//! race_editor
//!
//! # Edit specific file
//! race_editor data/races.ron
//!
//! # Create new races file
//! race_editor campaigns/my_campaign/data/races.ron
//! ```
//!
//! # Features
//!
//! - Interactive menu-driven interface
//! - Add/edit/delete race definitions
//! - Configure stat modifiers
//! - Set resistances and special abilities
//! - Input validation
//! - Pretty-printed RON output

use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

/// Type alias for race identifiers
pub type RaceId = String;

/// Complete definition of a character race
///
/// This structure contains all mechanical properties of a race,
/// loaded from external data files to support modding and campaigns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaceDefinition {
    /// Unique identifier (e.g., "human", "elf")
    pub id: RaceId,

    /// Display name (e.g., "Human", "Elf")
    pub name: String,

    /// Stat modifiers applied to base stats
    pub stat_modifiers: StatModifiers,

    /// Elemental resistance modifiers
    pub resistances: Resistances,

    /// Special racial abilities
    pub special_abilities: Vec<String>,

    /// Bitflag for item disablement checking
    pub disablement_bit: u8,
}

/// Stat modifiers for a race
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StatModifiers {
    pub might: i8,
    pub intellect: i8,
    pub personality: i8,
    pub endurance: i8,
    pub speed: i8,
    pub accuracy: i8,
    pub luck: i8,
}

/// Elemental resistance modifiers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Resistances {
    pub fire: i8,
    pub cold: i8,
    pub electricity: i8,
    pub poison: i8,
    pub energy: i8,
}

/// Main application state
struct RaceEditor {
    races: Vec<RaceDefinition>,
    file_path: PathBuf,
    modified: bool,
}

impl RaceEditor {
    /// Creates a new editor with loaded races from file
    fn load(path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let races = if path.exists() {
            println!("Loading races from: {}", path.display());
            let contents = std::fs::read_to_string(&path)?;
            let mut vec: Vec<RaceDefinition> = ron::from_str(&contents)?;
            vec.sort_by(|a, b| a.id.cmp(&b.id));
            vec
        } else {
            println!("File not found, starting with empty race list");
            Vec::new()
        };

        Ok(Self {
            races,
            file_path: path,
            modified: false,
        })
    }

    /// Main menu loop
    fn run(&mut self) {
        loop {
            self.show_menu();

            let choice = self.read_input("Choice: ");

            match choice.trim() {
                "1" => self.list_races(),
                "2" => self.add_race(),
                "3" => self.edit_race(),
                "4" => self.delete_race(),
                "5" => self.preview_race(),
                "6" => {
                    if self.save() {
                        println!("✅ Saved successfully. Exiting.");
                        break;
                    }
                }
                "q" | "Q" => {
                    if self.confirm_exit() {
                        break;
                    }
                }
                _ => println!("❌ Invalid choice. Please try again."),
            }

            println!(); // Blank line between operations
        }
    }

    /// Displays the main menu
    fn show_menu(&self) {
        println!("\n╔════════════════════════════════════════╗");
        println!("║       ANTARES RACE EDITOR              ║");
        println!("╚════════════════════════════════════════╝");
        println!("  File: {}", self.file_path.display());
        println!("  Races: {}", self.races.len());
        if self.modified {
            println!("  Status: ⚠️  UNSAVED CHANGES");
        } else {
            println!("  Status: ✅ Saved");
        }
        println!("\n┌────────────────────────────────────────┐");
        println!("│  1. List all races                     │");
        println!("│  2. Add new race                       │");
        println!("│  3. Edit existing race                 │");
        println!("│  4. Delete race                        │");
        println!("│  5. Preview race                       │");
        println!("│  6. Save and exit                      │");
        println!("│  q. Quit (without saving)              │");
        println!("└────────────────────────────────────────┘");
    }

    /// Lists all races
    fn list_races(&self) {
        if self.races.is_empty() {
            println!("\n📋 No races defined yet.");
            return;
        }

        println!("\n┌─────┬──────────────┬────────────────┬───────────────────────┐");
        println!("│ Idx │ ID           │ Name           │ Stat Modifiers        │");
        println!("├─────┼──────────────┼────────────────┼───────────────────────┤");

        for (idx, race) in self.races.iter().enumerate() {
            let stat_summary = format!(
                "M{:+} I{:+} P{:+} E{:+}",
                race.stat_modifiers.might,
                race.stat_modifiers.intellect,
                race.stat_modifiers.personality,
                race.stat_modifiers.endurance
            );

            println!(
                "│ {:3} │ {:12} │ {:14} │ {:21} │",
                idx,
                truncate(&race.id, 12),
                truncate(&race.name, 14),
                truncate(&stat_summary, 21)
            );
        }

        println!("└─────┴──────────────┴────────────────┴───────────────────────┘");
    }

    /// Adds a new race
    fn add_race(&mut self) {
        println!("\n╔════════════════════════════════════════╗");
        println!("║        ADD NEW RACE                    ║");
        println!("╚════════════════════════════════════════╝");

        let id = loop {
            let input = self.read_input("Race ID (lowercase, e.g., 'halfelf'): ");
            let trimmed = input.trim();

            if trimmed.is_empty() {
                println!("❌ ID cannot be empty");
                continue;
            }

            if self.races.iter().any(|r| r.id == trimmed) {
                println!("❌ Race ID '{}' already exists", trimmed);
                continue;
            }

            if !trimmed.chars().all(|c| c.is_alphanumeric() || c == '_') {
                println!("❌ ID must contain only letters, numbers, and underscores");
                continue;
            }

            break trimmed.to_string();
        };

        let name = self.read_input("Display Name (e.g., 'Half-Elf'): ");

        let stat_modifiers = self.input_stat_modifiers();
        let resistances = self.input_resistances();
        let disablement_bit = self.get_next_disablement_bit();
        let special_abilities = self.input_special_abilities();

        let race_def = RaceDefinition {
            id: id.clone(),
            name: name.trim().to_string(),
            stat_modifiers,
            resistances,
            special_abilities,
            disablement_bit,
        };

        self.races.push(race_def);
        self.races.sort_by(|a, b| a.id.cmp(&b.id));
        self.modified = true;

        println!("✅ Race '{}' created successfully!", id);
    }

    /// Edits an existing race
    fn edit_race(&mut self) {
        if self.races.is_empty() {
            println!("❌ No races to edit.");
            return;
        }

        self.list_races();

        let idx = loop {
            let input = self.read_input("\nEnter race index to edit (or 'c' to cancel): ");
            if input.trim().eq_ignore_ascii_case("c") {
                return;
            }

            match input.trim().parse::<usize>() {
                Ok(i) if i < self.races.len() => break i,
                _ => println!("❌ Invalid index. Please try again."),
            }
        };

        let race = &self.races[idx];
        println!("\n╔════════════════════════════════════════╗");
        println!("║        EDIT RACE: {:21} ║", truncate(&race.name, 21));
        println!("╚════════════════════════════════════════╝");

        println!("\nWhat would you like to edit?");
        println!("  1. Display Name (currently: {})", race.name);
        println!("  2. Stat Modifiers");
        println!("  3. Resistances");
        println!(
            "  4. Special Abilities (currently: {})",
            race.special_abilities.len()
        );
        println!("  c. Cancel");

        let choice = self.read_input("\nChoice: ");

        match choice.trim() {
            "1" => {
                let new_name = self.read_input("New display name: ");
                self.races[idx].name = new_name.trim().to_string();
                self.modified = true;
                println!("✅ Name updated");
            }
            "2" => {
                let stat_modifiers = self.input_stat_modifiers();
                self.races[idx].stat_modifiers = stat_modifiers;
                self.modified = true;
                println!("✅ Stat modifiers updated");
            }
            "3" => {
                let resistances = self.input_resistances();
                self.races[idx].resistances = resistances;
                self.modified = true;
                println!("✅ Resistances updated");
            }
            "4" => {
                let abilities = self.input_special_abilities();
                self.races[idx].special_abilities = abilities;
                self.modified = true;
                println!("✅ Special abilities updated");
            }
            "c" | "C" => println!("Cancelled"),
            _ => println!("❌ Invalid choice"),
        }
    }

    /// Deletes a race
    fn delete_race(&mut self) {
        if self.races.is_empty() {
            println!("❌ No races to delete.");
            return;
        }

        self.list_races();

        let idx = loop {
            let input = self.read_input("\nEnter race index to delete (or 'c' to cancel): ");
            if input.trim().eq_ignore_ascii_case("c") {
                return;
            }

            match input.trim().parse::<usize>() {
                Ok(i) if i < self.races.len() => break i,
                _ => println!("❌ Invalid index. Please try again."),
            }
        };

        let race = &self.races[idx];
        let confirm = self.read_input(&format!(
            "⚠️  Delete race '{}'? This cannot be undone! (yes/no): ",
            race.name
        ));

        if confirm.trim().eq_ignore_ascii_case("yes") {
            let removed = self.races.remove(idx);
            self.modified = true;
            println!("✅ Deleted race '{}'", removed.name);
        } else {
            println!("Cancelled");
        }
    }

    /// Previews a race with detailed information
    fn preview_race(&self) {
        if self.races.is_empty() {
            println!("❌ No races to preview.");
            return;
        }

        self.list_races();

        let idx = loop {
            let input = self.read_input("\nEnter race index to preview (or 'c' to cancel): ");
            if input.trim().eq_ignore_ascii_case("c") {
                return;
            }

            match input.trim().parse::<usize>() {
                Ok(i) if i < self.races.len() => break i,
                _ => println!("❌ Invalid index. Please try again."),
            }
        };

        let race = &self.races[idx];

        println!("\n╔════════════════════════════════════════╗");
        println!("║        RACE PREVIEW                    ║");
        println!("╚════════════════════════════════════════╝");
        println!("\n  ID: {}", race.id);
        println!("  Name: {}", race.name);

        println!("\n  Stat Modifiers:");
        println!("    Might:       {:+3}", race.stat_modifiers.might);
        println!("    Intellect:   {:+3}", race.stat_modifiers.intellect);
        println!("    Personality: {:+3}", race.stat_modifiers.personality);
        println!("    Endurance:   {:+3}", race.stat_modifiers.endurance);
        println!("    Speed:       {:+3}", race.stat_modifiers.speed);
        println!("    Accuracy:    {:+3}", race.stat_modifiers.accuracy);
        println!("    Luck:        {:+3}", race.stat_modifiers.luck);

        println!("\n  Resistances:");
        println!("    Fire:        {:+3}%", race.resistances.fire);
        println!("    Cold:        {:+3}%", race.resistances.cold);
        println!("    Electricity: {:+3}%", race.resistances.electricity);
        println!("    Poison:      {:+3}%", race.resistances.poison);
        println!("    Energy:      {:+3}%", race.resistances.energy);

        println!(
            "\n  Disablement Bit: {} (mask: 0b{:08b})",
            race.disablement_bit,
            1 << race.disablement_bit
        );

        if race.special_abilities.is_empty() {
            println!("\n  Special Abilities: None");
        } else {
            println!("\n  Special Abilities:");
            for ability in &race.special_abilities {
                println!("    - {}", ability);
            }
        }

        // Show sample character with average stats
        println!("\n  Sample Starting Character (base stats 10):");
        println!("    Might:       {}", 10 + race.stat_modifiers.might);
        println!("    Intellect:   {}", 10 + race.stat_modifiers.intellect);
        println!("    Personality: {}", 10 + race.stat_modifiers.personality);
        println!("    Endurance:   {}", 10 + race.stat_modifiers.endurance);
        println!("    Speed:       {}", 10 + race.stat_modifiers.speed);
        println!("    Accuracy:    {}", 10 + race.stat_modifiers.accuracy);
        println!("    Luck:        {}", 10 + race.stat_modifiers.luck);
    }

    /// Saves races to file
    fn save(&mut self) -> bool {
        print!("💾 Saving to {}... ", self.file_path.display());
        io::stdout().flush().unwrap();

        // Ensure parent directory exists
        if let Some(parent) = self.file_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                println!("\n❌ Failed to create directory: {}", e);
                return false;
            }
        }

        // Serialize to RON
        let ron_config = ron::ser::PrettyConfig::new()
            .struct_names(true)
            .separate_tuple_members(true)
            .enumerate_arrays(true);

        let ron_string = match ron::ser::to_string_pretty(&self.races, ron_config) {
            Ok(s) => s,
            Err(e) => {
                println!("\n❌ Serialization error: {}", e);
                return false;
            }
        };

        // Write to file
        if let Err(e) = std::fs::write(&self.file_path, ron_string) {
            println!("\n❌ Failed to write file: {}", e);
            return false;
        }

        self.modified = false;
        println!("Done!");
        true
    }

    /// Confirms exit with unsaved changes
    fn confirm_exit(&self) -> bool {
        if !self.modified {
            return true;
        }

        let response = self.read_input("⚠️  You have unsaved changes. Quit anyway? (yes/no): ");
        response.trim().eq_ignore_ascii_case("yes")
    }

    // ===== Input Helpers =====

    /// Reads a line of input with a prompt
    fn read_input(&self, prompt: &str) -> String {
        print!("{}", prompt);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input
    }

    /// Inputs stat modifiers
    fn input_stat_modifiers(&self) -> StatModifiers {
        println!("\nStat Modifiers (range: -5 to +5):");
        println!("  Enter modifiers for each stat (press Enter for 0)");

        StatModifiers {
            might: self.read_stat_modifier("Might"),
            intellect: self.read_stat_modifier("Intellect"),
            personality: self.read_stat_modifier("Personality"),
            endurance: self.read_stat_modifier("Endurance"),
            speed: self.read_stat_modifier("Speed"),
            accuracy: self.read_stat_modifier("Accuracy"),
            luck: self.read_stat_modifier("Luck"),
        }
    }

    /// Reads a single stat modifier
    fn read_stat_modifier(&self, stat_name: &str) -> i8 {
        loop {
            let input = self.read_input(&format!("  {}: ", stat_name));
            let trimmed = input.trim();

            if trimmed.is_empty() {
                return 0;
            }

            match trimmed.parse::<i8>() {
                Ok(val) if (-5..=5).contains(&val) => return val,
                Ok(_) => println!("    ❌ Value must be between -5 and +5"),
                Err(_) => println!("    ❌ Invalid number"),
            }
        }
    }

    /// Inputs resistances
    fn input_resistances(&self) -> Resistances {
        println!("\nElemental Resistances (range: -50 to +50):");
        println!("  Enter resistance % for each element (press Enter for 0)");

        Resistances {
            fire: self.read_resistance("Fire"),
            cold: self.read_resistance("Cold"),
            electricity: self.read_resistance("Electricity"),
            poison: self.read_resistance("Poison"),
            energy: self.read_resistance("Energy"),
        }
    }

    /// Reads a single resistance value
    fn read_resistance(&self, element: &str) -> i8 {
        loop {
            let input = self.read_input(&format!("  {}: ", element));
            let trimmed = input.trim();

            if trimmed.is_empty() {
                return 0;
            }

            match trimmed.parse::<i8>() {
                Ok(val) if (-50..=50).contains(&val) => return val,
                Ok(_) => println!("    ❌ Value must be between -50 and +50"),
                Err(_) => println!("    ❌ Invalid number"),
            }
        }
    }

    /// Gets the next available disablement bit
    fn get_next_disablement_bit(&self) -> u8 {
        let mut used_bits = [false; 8];

        for race in &self.races {
            if (race.disablement_bit as usize) < 8 {
                used_bits[race.disablement_bit as usize] = true;
            }
        }

        for (idx, &used) in used_bits.iter().enumerate() {
            if !used {
                return idx as u8;
            }
        }

        // If all bits used, find the next available
        self.races.len() as u8 % 8
    }

    /// Inputs special abilities
    fn input_special_abilities(&self) -> Vec<String> {
        println!("\nSpecial Abilities (comma-separated, or leave empty):");
        println!("  Examples: infravision, magic_resistance, detect_secret_doors");

        let input = self.read_input("Abilities: ");
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return Vec::new();
        }

        trimmed
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

// ===== Helper Functions =====

/// Truncates a string to a maximum length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

// ===== Main Entry Point =====

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let file_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("data/races.ron")
    };

    println!("╔════════════════════════════════════════╗");
    println!("║    ANTARES RACE EDITOR v0.1.0          ║");
    println!("╚════════════════════════════════════════╝");

    let mut editor = match RaceEditor::load(file_path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("❌ Failed to load races: {}", e);
            eprintln!("   Check that the file path is correct and the file is valid RON format.");
            process::exit(1);
        }
    };

    editor.run();

    println!("\nThank you for using Antares Race Editor!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a very long string", 10), "this is...");
        assert_eq!(truncate("exactly10c", 10), "exactly10c");
    }

    #[test]
    fn test_get_next_disablement_bit_empty() {
        let editor = RaceEditor {
            races: Vec::new(),
            file_path: PathBuf::from("test.ron"),
            modified: false,
        };

        assert_eq!(editor.get_next_disablement_bit(), 0);
    }

    #[test]
    fn test_get_next_disablement_bit_sequential() {
        let races = vec![
            RaceDefinition {
                id: "human".to_string(),
                name: "Human".to_string(),
                stat_modifiers: StatModifiers::default(),
                resistances: Resistances::default(),
                special_abilities: vec![],
                disablement_bit: 0,
            },
            RaceDefinition {
                id: "elf".to_string(),
                name: "Elf".to_string(),
                stat_modifiers: StatModifiers::default(),
                resistances: Resistances::default(),
                special_abilities: vec![],
                disablement_bit: 1,
            },
        ];

        let editor = RaceEditor {
            races,
            file_path: PathBuf::from("test.ron"),
            modified: false,
        };

        assert_eq!(editor.get_next_disablement_bit(), 2);
    }

    #[test]
    fn test_stat_modifiers_default() {
        let modifiers = StatModifiers::default();
        assert_eq!(modifiers.might, 0);
        assert_eq!(modifiers.intellect, 0);
        assert_eq!(modifiers.personality, 0);
        assert_eq!(modifiers.endurance, 0);
        assert_eq!(modifiers.speed, 0);
        assert_eq!(modifiers.accuracy, 0);
        assert_eq!(modifiers.luck, 0);
    }

    #[test]
    fn test_resistances_default() {
        let resistances = Resistances::default();
        assert_eq!(resistances.fire, 0);
        assert_eq!(resistances.cold, 0);
        assert_eq!(resistances.electricity, 0);
        assert_eq!(resistances.poison, 0);
        assert_eq!(resistances.energy, 0);
    }
}
