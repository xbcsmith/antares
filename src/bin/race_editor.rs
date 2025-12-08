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

use antares::domain::races::{RaceDefinition, Resistances, SizeCategory, StatModifiers};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

/// Standard proficiency IDs recognized by the system
const STANDARD_PROFICIENCY_IDS: &[&str] = &[
    "simple_weapon",
    "martial_melee",
    "martial_ranged",
    "blunt_weapon",
    "unarmed",
    "light_armor",
    "medium_armor",
    "heavy_armor",
    "shield",
    "arcane_item",
    "divine_item",
];

/// Standard item tags used for race restrictions
const STANDARD_ITEM_TAGS: &[&str] = &[
    "large_weapon",
    "two_handed",
    "heavy_armor",
    "elven_crafted",
    "dwarven_crafted",
    "requires_strength",
];

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
                        println!("Saved successfully. Exiting.");
                        break;
                    }
                }
                "q" | "Q" => {
                    if self.confirm_exit() {
                        break;
                    }
                }
                _ => println!("Invalid choice. Please try again."),
            }

            println!(); // Blank line between operations
        }
    }

    /// Displays the main menu
    fn show_menu(&self) {
        println!("\n========================================");
        println!("       ANTARES RACE EDITOR              ");
        println!("========================================");
        println!("  File: {}", self.file_path.display());
        println!("  Races: {}", self.races.len());
        if self.modified {
            println!("  Status: UNSAVED CHANGES");
        } else {
            println!("  Status: Saved");
        }
        println!("\n----------------------------------------");
        println!("  1. List all races");
        println!("  2. Add new race");
        println!("  3. Edit existing race");
        println!("  4. Delete race");
        println!("  5. Preview race");
        println!("  6. Save and exit");
        println!("  q. Quit (without saving)");
        println!("----------------------------------------");
    }

    /// Lists all races
    fn list_races(&self) {
        if self.races.is_empty() {
            println!("\nNo races defined yet.");
            return;
        }

        println!("\n+-----+--------------+----------------+-----------------------+--------+");
        println!("| Idx | ID           | Name           | Stat Modifiers        | Size   |");
        println!("+-----+--------------+----------------+-----------------------+--------+");

        for (idx, race) in self.races.iter().enumerate() {
            let stat_summary = format!(
                "M{:+} I{:+} P{:+} E{:+}",
                race.stat_modifiers.might,
                race.stat_modifiers.intellect,
                race.stat_modifiers.personality,
                race.stat_modifiers.endurance
            );

            let size_str = match race.size {
                SizeCategory::Small => "Small",
                SizeCategory::Medium => "Medium",
                SizeCategory::Large => "Large",
            };

            println!(
                "| {:3} | {:12} | {:14} | {:21} | {:6} |",
                idx,
                truncate(&race.id, 12),
                truncate(&race.name, 14),
                truncate(&stat_summary, 21),
                size_str
            );
        }

        println!("+-----+--------------+----------------+-----------------------+--------+");
    }

    /// Adds a new race
    fn add_race(&mut self) {
        println!("\n========================================");
        println!("        ADD NEW RACE                    ");
        println!("========================================");

        let id = loop {
            let input = self.read_input("Race ID (lowercase, e.g., 'halfelf'): ");
            let trimmed = input.trim();

            if trimmed.is_empty() {
                println!("ID cannot be empty");
                continue;
            }

            if self.races.iter().any(|r| r.id == trimmed) {
                println!("Race ID '{}' already exists", trimmed);
                continue;
            }

            if !trimmed.chars().all(|c| c.is_alphanumeric() || c == '_') {
                println!("ID must contain only letters, numbers, and underscores");
                continue;
            }

            break trimmed.to_string();
        };

        let name = self.read_input("Display Name (e.g., 'Half-Elf'): ");
        let description = self.read_input("Description: ");
        let stat_modifiers = self.input_stat_modifiers();
        let resistances = self.input_resistances();
        let size = self.input_size_category();

        let special_abilities = self.input_special_abilities();
        let proficiencies = self.input_proficiencies();
        let incompatible_item_tags = self.input_incompatible_tags();

        let race_def = RaceDefinition {
            id: id.clone(),
            name: name.trim().to_string(),
            description: description.trim().to_string(),
            stat_modifiers,
            resistances,
            special_abilities,
            size,
            proficiencies,
            incompatible_item_tags,
        };

        self.races.push(race_def);
        self.races.sort_by(|a, b| a.id.cmp(&b.id));
        self.modified = true;

        println!("Race '{}' created successfully!", id);
    }

    /// Edits an existing race
    fn edit_race(&mut self) {
        if self.races.is_empty() {
            println!("No races to edit.");
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
                _ => println!("Invalid index. Please try again."),
            }
        };

        let race = &self.races[idx];
        println!("\n========================================");
        println!("        EDIT RACE: {:21}", truncate(&race.name, 21));
        println!("========================================");

        println!("\nWhat would you like to edit?");
        println!("  1. Display Name (currently: {})", race.name);
        println!("  2. Description");
        println!("  3. Stat Modifiers");
        println!("  4. Resistances");
        println!("  5. Size Category (currently: {:?})", race.size);
        println!(
            "  6. Special Abilities (currently: {})",
            race.special_abilities.len()
        );
        println!(
            "  7. Proficiencies (currently: {})",
            if race.proficiencies.is_empty() {
                "None".to_string()
            } else {
                race.proficiencies.join(", ")
            }
        );
        println!(
            "  8. Incompatible Item Tags (currently: {})",
            if race.incompatible_item_tags.is_empty() {
                "None".to_string()
            } else {
                race.incompatible_item_tags.join(", ")
            }
        );
        println!("  c. Cancel");

        let choice = self.read_input("\nChoice: ");

        match choice.trim() {
            "1" => {
                let new_name = self.read_input("New display name: ");
                self.races[idx].name = new_name.trim().to_string();
                self.modified = true;
                println!("Name updated");
            }
            "2" => {
                let new_desc = self.read_input("New description: ");
                self.races[idx].description = new_desc.trim().to_string();
                self.modified = true;
                println!("Description updated");
            }
            "3" => {
                let stat_modifiers = self.input_stat_modifiers();
                self.races[idx].stat_modifiers = stat_modifiers;
                self.modified = true;
                println!("Stat modifiers updated");
            }
            "4" => {
                let resistances = self.input_resistances();
                self.races[idx].resistances = resistances;
                self.modified = true;
                println!("Resistances updated");
            }
            "5" => {
                let size = self.input_size_category();
                self.races[idx].size = size;
                self.modified = true;
                println!("Size category updated");
            }
            "6" => {
                let abilities = self.input_special_abilities();
                self.races[idx].special_abilities = abilities;
                self.modified = true;
                println!("Special abilities updated");
            }
            "7" => {
                let proficiencies = self.input_proficiencies();
                self.races[idx].proficiencies = proficiencies;
                self.modified = true;
                println!("Proficiencies updated");
            }
            "8" => {
                let incompatible_tags = self.input_incompatible_tags();
                self.races[idx].incompatible_item_tags = incompatible_tags;
                self.modified = true;
                println!("Incompatible item tags updated");
            }
            "c" | "C" => println!("Cancelled"),
            _ => println!("Invalid choice"),
        }
    }

    /// Deletes a race
    fn delete_race(&mut self) {
        if self.races.is_empty() {
            println!("No races to delete.");
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
                _ => println!("Invalid index. Please try again."),
            }
        };

        let race = &self.races[idx];
        let confirm = self.read_input(&format!(
            "Delete race '{}'? This cannot be undone! (yes/no): ",
            race.name
        ));

        if confirm.trim().eq_ignore_ascii_case("yes") {
            let removed = self.races.remove(idx);
            self.modified = true;
            println!("Deleted race '{}'", removed.name);
        } else {
            println!("Cancelled");
        }
    }

    /// Previews a race with detailed information
    fn preview_race(&self) {
        if self.races.is_empty() {
            println!("No races to preview.");
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
                _ => println!("Invalid index. Please try again."),
            }
        };

        let race = &self.races[idx];

        println!("\n========================================");
        println!("        RACE PREVIEW                    ");
        println!("========================================");
        println!("\n  ID: {}", race.id);
        println!("  Name: {}", race.name);
        println!("  Description: {}", race.description);
        println!("  Size: {:?}", race.size);

        println!("\n  Stat Modifiers:");
        println!("    Might:       {:+3}", race.stat_modifiers.might);
        println!("    Intellect:   {:+3}", race.stat_modifiers.intellect);
        println!("    Personality: {:+3}", race.stat_modifiers.personality);
        println!("    Endurance:   {:+3}", race.stat_modifiers.endurance);
        println!("    Speed:       {:+3}", race.stat_modifiers.speed);
        println!("    Accuracy:    {:+3}", race.stat_modifiers.accuracy);
        println!("    Luck:        {:+3}", race.stat_modifiers.luck);

        println!("\n  Resistances:");
        println!("    Magic:       {:3}%", race.resistances.magic);
        println!("    Fire:        {:3}%", race.resistances.fire);
        println!("    Cold:        {:3}%", race.resistances.cold);
        println!("    Electricity: {:3}%", race.resistances.electricity);
        println!("    Acid:        {:3}%", race.resistances.acid);
        println!("    Fear:        {:3}%", race.resistances.fear);
        println!("    Poison:      {:3}%", race.resistances.poison);
        println!("    Psychic:     {:3}%", race.resistances.psychic);

        // Disablement system removed - proficiency system now handles restrictions

        if race.special_abilities.is_empty() {
            println!("\n  Special Abilities: None");
        } else {
            println!("\n  Special Abilities:");
            for ability in &race.special_abilities {
                println!("    - {}", ability);
            }
        }

        if race.proficiencies.is_empty() {
            println!("\n  Proficiencies: None");
        } else {
            println!("\n  Proficiencies:");
            for prof in &race.proficiencies {
                println!("    - {}", prof);
            }
        }

        if race.incompatible_item_tags.is_empty() {
            println!("\n  Incompatible Item Tags: None");
        } else {
            println!("\n  Incompatible Item Tags:");
            for tag in &race.incompatible_item_tags {
                println!("    - {}", tag);
            }
        }

        // Show sample character with average stats
        println!("\n  Sample Starting Character (base stats 10):");
        println!(
            "    Might:       {}",
            10i16 + race.stat_modifiers.might as i16
        );
        println!(
            "    Intellect:   {}",
            10i16 + race.stat_modifiers.intellect as i16
        );
        println!(
            "    Personality: {}",
            10i16 + race.stat_modifiers.personality as i16
        );
        println!(
            "    Endurance:   {}",
            10i16 + race.stat_modifiers.endurance as i16
        );
        println!(
            "    Speed:       {}",
            10i16 + race.stat_modifiers.speed as i16
        );
        println!(
            "    Accuracy:    {}",
            10i16 + race.stat_modifiers.accuracy as i16
        );
        println!(
            "    Luck:        {}",
            10i16 + race.stat_modifiers.luck as i16
        );
    }

    /// Saves races to file
    fn save(&mut self) -> bool {
        print!("Saving to {}... ", self.file_path.display());
        io::stdout().flush().unwrap();

        // Ensure parent directory exists
        if let Some(parent) = self.file_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                println!("\nFailed to create directory: {}", e);
                return false;
            }
        }

        // Serialize to RON
        let ron_config = ron::ser::PrettyConfig::new()
            .struct_names(false)
            .separate_tuple_members(true)
            .enumerate_arrays(false);

        let ron_string = match ron::ser::to_string_pretty(&self.races, ron_config) {
            Ok(s) => s,
            Err(e) => {
                println!("\nSerialization error: {}", e);
                return false;
            }
        };

        // Write to file
        if let Err(e) = std::fs::write(&self.file_path, ron_string) {
            println!("\nFailed to write file: {}", e);
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

        let response = self.read_input("You have unsaved changes. Quit anyway? (yes/no): ");
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
        println!("\nStat Modifiers (range: -10 to +10):");
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
                Ok(val) if (-10..=10).contains(&val) => return val,
                Ok(_) => println!("    Value must be between -10 and +10"),
                Err(_) => println!("    Invalid number"),
            }
        }
    }

    /// Inputs resistances
    fn input_resistances(&self) -> Resistances {
        println!("\nElemental Resistances (range: 0 to 100):");
        println!("  Enter resistance % for each element (press Enter for 0)");

        Resistances {
            magic: self.read_resistance("Magic"),
            fire: self.read_resistance("Fire"),
            cold: self.read_resistance("Cold"),
            electricity: self.read_resistance("Electricity"),
            acid: self.read_resistance("Acid"),
            fear: self.read_resistance("Fear"),
            poison: self.read_resistance("Poison"),
            psychic: self.read_resistance("Psychic"),
        }
    }

    /// Reads a single resistance value
    fn read_resistance(&self, element: &str) -> u8 {
        loop {
            let input = self.read_input(&format!("  {}: ", element));
            let trimmed = input.trim();

            if trimmed.is_empty() {
                return 0;
            }

            match trimmed.parse::<u8>() {
                Ok(val) if val <= 100 => return val,
                Ok(_) => println!("    Value must be between 0 and 100"),
                Err(_) => println!("    Invalid number"),
            }
        }
    }

    /// Inputs size category
    fn input_size_category(&self) -> SizeCategory {
        println!("\nSize Category:");
        println!("  1. Small (gnomes, halflings)");
        println!("  2. Medium (humans, elves, dwarves)");
        println!("  3. Large (half-giants, ogres)");

        loop {
            let input = self.read_input("Choice (1-3, default 2): ");
            let trimmed = input.trim();

            if trimmed.is_empty() {
                return SizeCategory::Medium;
            }

            match trimmed {
                "1" => return SizeCategory::Small,
                "2" => return SizeCategory::Medium,
                "3" => return SizeCategory::Large,
                _ => println!("  Invalid choice. Enter 1, 2, or 3."),
            }
        }
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

    /// Inputs proficiencies with validation
    fn input_proficiencies(&self) -> Vec<String> {
        println!("\n========================================");
        println!("        PROFICIENCY SELECTION           ");
        println!("========================================");
        println!("\nStandard Proficiencies:");
        println!("  Weapons:");
        println!("    • simple_weapon      - Simple weapons (daggers, clubs)");
        println!("    • martial_melee      - Martial melee weapons (swords, axes)");
        println!("    • martial_ranged     - Martial ranged weapons (longbows, crossbows)");
        println!("    • blunt_weapon       - Blunt weapons (maces, flails)");
        println!("    • unarmed            - Unarmed combat");
        println!("\n  Armor:");
        println!("    • light_armor        - Light armor (leather, padded)");
        println!("    • medium_armor       - Medium armor (chainmail, scale)");
        println!("    • heavy_armor        - Heavy armor (plate, full plate)");
        println!("    • shield             - Shields");
        println!("\n  Magic Items:");
        println!("    • arcane_item        - Arcane magic items (wands, staves)");
        println!("    • divine_item        - Divine magic items (holy symbols, relics)");

        println!("\nEnter proficiencies (comma-separated, or leave empty):");
        println!("  Example: simple_weapon,light_armor,shield");

        let input = self.read_input("Proficiencies: ");
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return Vec::new();
        }

        let proficiencies: Vec<String> = trimmed
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Validate proficiencies
        let mut valid_proficiencies = Vec::new();
        for prof in proficiencies {
            if STANDARD_PROFICIENCY_IDS.contains(&prof.as_str()) {
                valid_proficiencies.push(prof);
            } else {
                println!("⚠️  Warning: '{}' is not a standard proficiency ID", prof);
                println!("   Standard IDs: {}", STANDARD_PROFICIENCY_IDS.join(", "));
                let confirm = self.read_input(&format!("   Include '{}' anyway? (y/n): ", prof));
                if confirm.trim().eq_ignore_ascii_case("y") {
                    valid_proficiencies.push(prof);
                }
            }
        }

        if !valid_proficiencies.is_empty() {
            println!("✅ Added proficiencies: {}", valid_proficiencies.join(", "));
        }

        valid_proficiencies
    }

    /// Inputs incompatible item tags with validation
    fn input_incompatible_tags(&self) -> Vec<String> {
        println!("\n========================================");
        println!("   INCOMPATIBLE ITEM TAGS SELECTION     ");
        println!("========================================");
        println!("\nStandard Item Tags:");
        println!("  • large_weapon       - Large/oversized weapons");
        println!("  • two_handed         - Two-handed weapons");
        println!("  • heavy_armor        - Heavy armor pieces");
        println!("  • elven_crafted      - Elven-crafted items");
        println!("  • dwarven_crafted    - Dwarven-crafted items");
        println!("  • requires_strength  - Items requiring high strength");

        println!("\nRaces with incompatible tags cannot use items with those tags.");
        println!("Example: A halfling might have 'large_weapon,heavy_armor' incompatible.");
        println!("\nEnter incompatible tags (comma-separated, or leave empty):");

        let input = self.read_input("Incompatible Tags: ");
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return Vec::new();
        }

        let tags: Vec<String> = trimmed
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Validate tags
        let mut valid_tags = Vec::new();
        for tag in tags {
            if STANDARD_ITEM_TAGS.contains(&tag.as_str()) {
                valid_tags.push(tag);
            } else {
                println!("⚠️  Warning: '{}' is not a standard item tag", tag);
                println!("   Standard tags: {}", STANDARD_ITEM_TAGS.join(", "));
                let confirm = self.read_input(&format!("   Include '{}' anyway? (y/n): ", tag));
                if confirm.trim().eq_ignore_ascii_case("y") {
                    valid_tags.push(tag);
                }
            }
        }

        if !valid_tags.is_empty() {
            println!("✅ Added incompatible tags: {}", valid_tags.join(", "));
        }

        valid_tags
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

    println!("========================================");
    println!("    ANTARES RACE EDITOR v0.2.0          ");
    println!("========================================");

    let mut editor = match RaceEditor::load(file_path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to load races: {}", e);
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
        assert_eq!(resistances.magic, 0);
        assert_eq!(resistances.fire, 0);
        assert_eq!(resistances.cold, 0);
        assert_eq!(resistances.electricity, 0);
        assert_eq!(resistances.acid, 0);
        assert_eq!(resistances.fear, 0);
        assert_eq!(resistances.poison, 0);
        assert_eq!(resistances.psychic, 0);
    }

    #[test]
    fn test_size_category_default() {
        let size: SizeCategory = SizeCategory::default();
        assert_eq!(size, SizeCategory::Medium);
    }
}
