// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Class Editor CLI
//!
//! Interactive command-line tool for creating and editing character class definitions.
//! Supports loading, editing, and saving class data in RON format.
//!
//! # Usage
//!
//! ```bash
//! # Create/edit classes in default location
//! class_editor
//!
//! # Edit specific file
//! class_editor data/classes.ron
//!
//! # Create new classes file
//! class_editor campaigns/my_campaign/data/classes.ron
//! ```
//!
//! # Features
//!
//! - Interactive menu-driven interface
//! - Add/edit/delete class definitions
//! - Preview class statistics
//! - Input validation
//! - Pretty-printed RON output

use antares::domain::classes::{ClassDatabase, ClassDefinition, SpellSchool, SpellStat};
use antares::domain::types::DiceRoll;
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

/// Main application state
struct ClassEditor {
    classes: Vec<ClassDefinition>,
    file_path: PathBuf,
    modified: bool,
}

impl ClassEditor {
    /// Creates a new editor with loaded classes from file
    fn load(path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let classes = if path.exists() {
            println!("Loading classes from: {}", path.display());
            let db = ClassDatabase::load_from_file(&path)?;
            let mut vec: Vec<ClassDefinition> = db.all_classes().cloned().collect();
            vec.sort_by(|a, b| a.id.cmp(&b.id));
            vec
        } else {
            println!("File not found, starting with empty class list");
            Vec::new()
        };

        Ok(Self {
            classes,
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
                "1" => self.list_classes(),
                "2" => self.add_class(),
                "3" => self.edit_class(),
                "4" => self.delete_class(),
                "5" => self.preview_class(),
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
        println!("║       ANTARES CLASS EDITOR             ║");
        println!("╚════════════════════════════════════════╝");
        println!("  File: {}", self.file_path.display());
        println!("  Classes: {}", self.classes.len());
        if self.modified {
            println!("  Status: ⚠️  UNSAVED CHANGES");
        } else {
            println!("  Status: ✅ Saved");
        }
        println!("\n┌────────────────────────────────────────┐");
        println!("│  1. List all classes                   │");
        println!("│  2. Add new class                      │");
        println!("│  3. Edit existing class                │");
        println!("│  4. Delete class                       │");
        println!("│  5. Preview class                      │");
        println!("│  6. Save and exit                      │");
        println!("│  q. Quit (without saving)              │");
        println!("└────────────────────────────────────────┘");
    }

    /// Lists all classes
    fn list_classes(&self) {
        if self.classes.is_empty() {
            println!("\n📋 No classes defined yet.");
            return;
        }

        println!("\n┌─────┬──────────────┬────────────────┬────────┬─────────────┐");
        println!("│ Idx │ ID           │ Name           │ HP Die │ Spells      │");
        println!("├─────┼──────────────┼────────────────┼────────┼─────────────┤");

        for (idx, class) in self.classes.iter().enumerate() {
            let spell_info = if let Some(school) = class.spell_school {
                format!(
                    "{:?} {}",
                    school,
                    if class.is_pure_caster {
                        "Full"
                    } else {
                        "Hybrid"
                    }
                )
            } else {
                "None".to_string()
            };

            println!(
                "│ {:3} │ {:12} │ {:14} │ {:6} │ {:11} │",
                idx,
                truncate(&class.id, 12),
                truncate(&class.name, 14),
                format!("{}d{}", class.hp_die.count, class.hp_die.sides),
                truncate(&spell_info, 11)
            );
        }

        println!("└─────┴──────────────┴────────────────┴────────┴─────────────┘");
    }

    /// Adds a new class
    fn add_class(&mut self) {
        println!("\n╔════════════════════════════════════════╗");
        println!("║        ADD NEW CLASS                   ║");
        println!("╚════════════════════════════════════════╝");

        let id = loop {
            let input = self.read_input("Class ID (lowercase, e.g., 'barbarian'): ");
            let trimmed = input.trim();

            if trimmed.is_empty() {
                println!("❌ ID cannot be empty");
                continue;
            }

            if self.classes.iter().any(|c| c.id == trimmed) {
                println!("❌ Class ID '{}' already exists", trimmed);
                continue;
            }

            if !trimmed.chars().all(|c| c.is_alphanumeric() || c == '_') {
                println!("❌ ID must contain only letters, numbers, and underscores");
                continue;
            }

            break trimmed.to_string();
        };

        let name = self.read_input("Display Name (e.g., 'Barbarian'): ");

        let hp_die = self.select_hp_die();
        let (spell_school, is_pure_caster, spell_stat) = self.select_spell_access();
        let special_abilities = self.input_special_abilities();
        let proficiencies = self.input_proficiencies();

        let class_def = ClassDefinition {
            id: id.clone(),
            name: name.trim().to_string(),
            description: String::new(),
            hp_die,
            spell_school,
            is_pure_caster,
            spell_stat,

            special_abilities,
            starting_weapon_id: None,
            starting_armor_id: None,
            starting_items: Vec::new(),
            proficiencies,
        };

        self.classes.push(class_def);
        self.classes.sort_by(|a, b| a.id.cmp(&b.id));
        self.modified = true;

        println!("✅ Class '{}' created successfully!", id);
    }

    /// Edits an existing class
    fn edit_class(&mut self) {
        if self.classes.is_empty() {
            println!("❌ No classes to edit.");
            return;
        }

        self.list_classes();

        let idx = loop {
            let input = self.read_input("\nEnter class index to edit (or 'c' to cancel): ");
            if input.trim().eq_ignore_ascii_case("c") {
                return;
            }

            match input.trim().parse::<usize>() {
                Ok(i) if i < self.classes.len() => break i,
                _ => println!("❌ Invalid index. Please try again."),
            }
        };

        let class = &self.classes[idx];
        println!("\n╔════════════════════════════════════════╗");
        println!("║        EDIT CLASS: {:19} ║", truncate(&class.name, 19));
        println!("╚════════════════════════════════════════╝");

        println!("\nWhat would you like to edit?");
        println!("  1. Display Name (currently: {})", class.name);
        println!(
            "  2. HP Die (currently: {}d{})",
            class.hp_die.count, class.hp_die.sides
        );
        println!(
            "  3. Spell Access (currently: {})",
            if class.spell_school.is_some() {
                format!("{:?}", class.spell_school.unwrap())
            } else {
                "None".to_string()
            }
        );
        println!(
            "  4. Special Abilities (currently: {})",
            class.special_abilities.len()
        );
        println!(
            "  5. Proficiencies (currently: {})",
            if class.proficiencies.is_empty() {
                "None".to_string()
            } else {
                class.proficiencies.join(", ")
            }
        );
        println!("  c. Cancel");

        let choice = self.read_input("\nChoice: ");

        match choice.trim() {
            "1" => {
                let new_name = self.read_input("New display name: ");
                self.classes[idx].name = new_name.trim().to_string();
                self.modified = true;
                println!("✅ Name updated");
            }
            "2" => {
                let hp_die = self.select_hp_die();
                self.classes[idx].hp_die = hp_die;
                self.modified = true;
                println!("✅ HP die updated");
            }
            "3" => {
                let (spell_school, is_pure_caster, spell_stat) = self.select_spell_access();
                self.classes[idx].spell_school = spell_school;
                self.classes[idx].is_pure_caster = is_pure_caster;
                self.classes[idx].spell_stat = spell_stat;
                self.modified = true;
                println!("✅ Spell access updated");
            }
            "4" => {
                let abilities = self.input_special_abilities();
                self.classes[idx].special_abilities = abilities;
                self.modified = true;
                println!("✅ Special abilities updated");
            }
            "5" => {
                let proficiencies = self.input_proficiencies();
                self.classes[idx].proficiencies = proficiencies;
                self.modified = true;
                println!("✅ Proficiencies updated");
            }
            "c" | "C" => println!("Cancelled"),
            _ => println!("❌ Invalid choice"),
        }
    }

    /// Deletes a class
    fn delete_class(&mut self) {
        if self.classes.is_empty() {
            println!("❌ No classes to delete.");
            return;
        }

        self.list_classes();

        let idx = loop {
            let input = self.read_input("\nEnter class index to delete (or 'c' to cancel): ");
            if input.trim().eq_ignore_ascii_case("c") {
                return;
            }

            match input.trim().parse::<usize>() {
                Ok(i) if i < self.classes.len() => break i,
                _ => println!("❌ Invalid index. Please try again."),
            }
        };

        let class = &self.classes[idx];
        let confirm = self.read_input(&format!(
            "⚠️  Delete class '{}'? This cannot be undone! (yes/no): ",
            class.name
        ));

        if confirm.trim().eq_ignore_ascii_case("yes") {
            let removed = self.classes.remove(idx);
            self.modified = true;
            println!("✅ Deleted class '{}'", removed.name);
        } else {
            println!("Cancelled");
        }
    }

    /// Previews a class with sample stats
    fn preview_class(&self) {
        if self.classes.is_empty() {
            println!("❌ No classes to preview.");
            return;
        }

        self.list_classes();

        let idx = loop {
            let input = self.read_input("\nEnter class index to preview (or 'c' to cancel): ");
            if input.trim().eq_ignore_ascii_case("c") {
                return;
            }

            match input.trim().parse::<usize>() {
                Ok(i) if i < self.classes.len() => break i,
                _ => println!("❌ Invalid index. Please try again."),
            }
        };

        let class = &self.classes[idx];

        println!("\n╔════════════════════════════════════════╗");
        println!("║        CLASS PREVIEW                   ║");
        println!("╚════════════════════════════════════════╝");
        println!("  ID: {}", class.id);
        println!("  Name: {}", class.name);
        println!("  HP Die: {}d{}", class.hp_die.count, class.hp_die.sides);
        println!(
            "  Spell School: {}",
            class
                .spell_school
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "None".to_string())
        );
        println!("  Pure Caster: {}", class.is_pure_caster);
        println!(
            "  Spell Stat: {}",
            class
                .spell_stat
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "N/A".to_string())
        );
        if class.special_abilities.is_empty() {
            println!("  Special Abilities: None");
        } else {
            println!("  Special Abilities:");
            for ability in &class.special_abilities {
                println!("    - {}", ability);
            }
        }

        println!("\n  Sample HP progression (assuming average rolls):");
        let avg_hp = (class.hp_die.sides + 1) as f32 / 2.0;
        for level in [1, 5, 10, 15, 20] {
            let estimated_hp = (avg_hp * level as f32) as u32;
            println!("    Level {}: ~{} HP", level, estimated_hp);
        }
    }

    /// Saves classes to file
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

        let ron_string = match ron::ser::to_string_pretty(&self.classes, ron_config) {
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

    /// Inputs multiple string values (one per line).
    ///
    /// This helper allows entering multiple values in an interactive CLI:
    ///  - Prompts with `label` repeatedly
    ///  - Pressing Enter on a blank line finishes input
    fn input_multistring_values(&self, prompt: &str, label: &str) -> Vec<String> {
        if !prompt.is_empty() {
            println!("\n{}", prompt);
        }
        println!("(Enter values one per line. Press Enter on an empty line to finish.)");

        let mut values: Vec<String> = Vec::new();
        loop {
            let input = self.read_input(label);
            let trimmed = input.trim();
            if trimmed.is_empty() {
                break;
            }
            values.push(trimmed.to_string());
        }
        values
    }

    /// Selects HP die from menu
    fn select_hp_die(&self) -> DiceRoll {
        println!("\nHP Gain Die:");
        println!("  1. 1d4  (Weak - Sorcerer)");
        println!("  2. 1d6  (Low - Cleric, Robber)");
        println!("  3. 1d8  (Medium - Paladin, Archer)");
        println!("  4. 1d10 (High - Knight)");
        println!("  5. 1d12 (Very High - Barbarian)");

        loop {
            let choice = self.read_input("Choice (1-5): ");
            match choice.trim() {
                "1" => return DiceRoll::new(1, 4, 0),
                "2" => return DiceRoll::new(1, 6, 0),
                "3" => return DiceRoll::new(1, 8, 0),
                "4" => return DiceRoll::new(1, 10, 0),
                "5" => return DiceRoll::new(1, 12, 0),
                _ => println!("❌ Invalid choice. Please enter 1-5."),
            }
        }
    }

    /// Selects spell access
    fn select_spell_access(&self) -> (Option<SpellSchool>, bool, Option<SpellStat>) {
        println!("\nSpell Access:");
        println!("  1. None (Warrior classes)");
        println!("  2. Cleric - Full (Cleric)");
        println!("  3. Sorcerer - Full (Sorcerer)");
        println!("  4. Cleric - Hybrid (Paladin)");
        println!("  5. Sorcerer - Hybrid (Archer)");

        loop {
            let choice = self.read_input("Choice (1-5): ");
            match choice.trim() {
                "1" => return (None, false, None),
                "2" => {
                    return (
                        Some(SpellSchool::Cleric),
                        true,
                        Some(SpellStat::Personality),
                    )
                }
                "3" => {
                    return (
                        Some(SpellSchool::Sorcerer),
                        true,
                        Some(SpellStat::Intellect),
                    )
                }
                "4" => {
                    return (
                        Some(SpellSchool::Cleric),
                        false,
                        Some(SpellStat::Personality),
                    )
                }
                "5" => {
                    return (
                        Some(SpellSchool::Sorcerer),
                        false,
                        Some(SpellStat::Intellect),
                    )
                }
                _ => println!("❌ Invalid choice. Please enter 1-5."),
            }
        }
    }

    /// Inputs special abilities (one per line; blank line to finish)
    fn input_special_abilities(&self) -> Vec<String> {
        println!(
            "\nSpecial Abilities (enter one per line; press Enter on an empty line to finish):"
        );
        println!("  Examples: backstab, disarm_trap, multiple_attacks, turn_undead");

        self.input_multistring_values("Special Abilities:", "Ability: ")
    }

    /// Inputs proficiencies with validation (one per line)
    fn input_proficiencies(&self) -> Vec<String> {
        println!("\n╔════════════════════════════════════════╗");
        println!("║        PROFICIENCY SELECTION           ║");
        println!("╚════════════════════════════════════════╝");
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

        println!("\nEnter proficiencies (one per line; leave empty to finish):");
        println!("  Example: simple_weapon");

        let candidates = self.input_multistring_values("", "Proficiency: ");

        // Validate proficiencies and warn about invalid ones
        let valid_proficiencies = filter_valid_proficiencies(&candidates);
        for prof in &candidates {
            if !valid_proficiencies.contains(prof) {
                println!("⚠ Invalid proficiency '{}'; ignoring", prof);
            }
        }

        if !valid_proficiencies.is_empty() {
            println!("✅ Added proficiencies: {}", valid_proficiencies.join(", "));
        }

        valid_proficiencies
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

// NOTE: `parse_multistring_input` is a helper used only in unit tests.
// It has been moved into the `#[cfg(test)] mod tests` module below so it is only
// compiled during test builds and won't trigger dead-code warnings in release builds.

/// Filter valid proficiencies from a list of candidate strings.
///
/// This function is a pure helper that checks whether each provided string
/// exists in the `STANDARD_PROFICIENCY_IDS` set and returns the subset that
/// are valid proficiency IDs.
///
/// # Arguments
///
/// * `candidates` - Slice of candidate proficiency strings.
///
/// # Returns
///
/// A `Vec<String>` containing only the valid proficiencies.
///
/// # Examples
///
/// ```
/// let candidates = vec![
///     "simple_weapon".to_string(),
///     "invalid_tag".to_string(),
/// ];
/// let filtered = filter_valid_proficiencies(&candidates);
/// assert_eq!(filtered, vec!["simple_weapon".to_string()]);
/// ```
fn filter_valid_proficiencies(candidates: &[String]) -> Vec<String> {
    candidates
        .iter()
        .filter(|p| STANDARD_PROFICIENCY_IDS.contains(&p.as_str()))
        .cloned()
        .collect()
}

// ===== Main Entry Point =====

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let file_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("data/classes.ron")
    };

    println!("╔════════════════════════════════════════╗");
    println!("║    ANTARES CLASS EDITOR v0.1.0         ║");
    println!("╚════════════════════════════════════════╝");

    let mut editor = match ClassEditor::load(file_path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("❌ Failed to load classes: {}", e);
            eprintln!("   Check that the file path is correct and the file is valid RON format.");
            process::exit(1);
        }
    };

    editor.run();

    println!("\nThank you for using Antares Class Editor!");
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test-only helper for parsing newline-separated inputs. Defined inside the
    // tests module so it is only compiled for tests and doesn't cause dead-code
    // warnings during normal builds.
    fn parse_multistring_input(input: &str) -> Vec<String> {
        input
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect()
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a very long string", 10), "this is...");
        assert_eq!(truncate("exactly10c", 10), "exactly10c");
    }

    #[test]
    fn test_parse_multistring_input_basic() {
        let input = "alpha\nbeta\n\n  gamma  \n";
        let parsed = parse_multistring_input(input);
        assert_eq!(
            parsed,
            vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]
        );
    }

    #[test]
    fn test_parse_multistring_input_empty() {
        let input = "";
        let parsed = parse_multistring_input(input);
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_parse_multistring_input_with_whitespace() {
        let input = "\n   \n   one  \n two \n\nthree\n";
        let parsed = parse_multistring_input(input);
        assert_eq!(
            parsed,
            vec!["one".to_string(), "two".to_string(), "three".to_string()]
        );
    }

    #[test]
    fn test_filter_valid_proficiencies() {
        let candidates = vec![
            "simple_weapon".to_string(),
            "invalid_tag".to_string(),
            "martial_melee".to_string(),
        ];
        let filtered = filter_valid_proficiencies(&candidates);
        assert_eq!(
            filtered,
            vec!["simple_weapon".to_string(), "martial_melee".to_string()]
        );

        // Ensure unknown entries are removed
        let candidates2 = vec!["not_a_proficiency".to_string()];
        let filtered2 = filter_valid_proficiencies(&candidates2);
        assert!(filtered2.is_empty());
    }
}
