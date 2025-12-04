// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Item Editor CLI
//!
//! Interactive command-line tool for creating and editing item definitions.
//! Supports weapons, armor, accessories, consumables, ammunition, and quest items.
//!
//! # Usage
//!
//! ```bash
//! # Create/edit items in default location
//! item_editor
//!
//! # Edit specific file
//! item_editor data/items.ron
//!
//! # Create new items file
//! item_editor campaigns/my_campaign/data/items.ron
//! ```
//!
//! # Features
//!
//! - Interactive menu-driven interface
//! - Add/edit/delete item definitions
//! - Support for all item types (weapons, armor, consumables, etc.)
//! - Class restriction configuration
//! - Bonus and enchantment support
//! - Preview item statistics
//! - Input validation
//! - Pretty-printed RON output

use antares::domain::items::{
    AccessoryData, AccessorySlot, AmmoData, AmmoType, ArmorData, AttributeType, Bonus,
    BonusAttribute, ConsumableData, ConsumableEffect, Disablement, Item, ItemType, QuestData,
    WeaponData,
};
use antares::domain::types::{DiceRoll, ItemId};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

/// Main application state
struct ItemEditor {
    items: Vec<Item>,
    file_path: PathBuf,
    modified: bool,
}

impl ItemEditor {
    /// Creates a new editor with loaded items from file
    fn load(path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let items = if path.exists() {
            println!("Loading items from: {}", path.display());
            let contents = fs::read_to_string(&path)?;
            let mut items: Vec<Item> = ron::from_str(&contents)?;
            items.sort_by(|a, b| a.id.cmp(&b.id));
            items
        } else {
            println!("File not found, starting with empty item list");
            Vec::new()
        };

        Ok(Self {
            items,
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
                "1" => self.list_items(),
                "2" => self.add_item(),
                "3" => self.edit_item(),
                "4" => self.delete_item(),
                "5" => self.preview_item(),
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
        println!("║       ANTARES ITEM EDITOR              ║");
        println!("╚════════════════════════════════════════╝");
        println!("  File: {}", self.file_path.display());
        println!(
            "  Items: {} {}",
            self.items.len(),
            if self.modified { "(modified)" } else { "" }
        );
        println!();
        println!("  [1] List Items");
        println!("  [2] Add Item");
        println!("  [3] Edit Item");
        println!("  [4] Delete Item");
        println!("  [5] Preview Item");
        println!("  [6] Save & Exit");
        println!("  [Q] Quit (discard changes)");
        println!();
    }

    /// Lists all items
    fn list_items(&self) {
        if self.items.is_empty() {
            println!("📦 No items defined yet.");
            return;
        }

        println!("\n════════════════════════════════════════");
        println!("  ITEM LIST");
        println!("════════════════════════════════════════");

        for item in &self.items {
            let type_str = match &item.item_type {
                ItemType::Weapon(_) => "Weapon",
                ItemType::Armor(_) => "Armor",
                ItemType::Accessory(_) => "Accessory",
                ItemType::Consumable(_) => "Consumable",
                ItemType::Ammo(_) => "Ammo",
                ItemType::Quest(_) => "Quest",
            };

            let magic = if item.is_magical() { "✨" } else { "" };
            let cursed = if item.is_cursed { "💀" } else { "" };

            println!(
                "  [{}] {} - {} {} {}",
                item.id, item.name, type_str, magic, cursed
            );
        }
    }

    /// Adds a new item
    fn add_item(&mut self) {
        println!("\n════════════════════════════════════════");
        println!("  ADD NEW ITEM");
        println!("════════════════════════════════════════");

        // Auto-assign ID
        let id = self.next_item_id();
        println!("  Auto-assigned ID: {}", id);

        // Get basic info
        let name = self.read_input("Item name: ").trim().to_string();
        if name.is_empty() {
            println!("❌ Name cannot be empty.");
            return;
        }

        // Select item type
        println!("\nItem Type:");
        println!("  [1] Weapon");
        println!("  [2] Armor");
        println!("  [3] Accessory");
        println!("  [4] Consumable");
        println!("  [5] Ammunition");
        println!("  [6] Quest Item");

        let type_choice = self.read_input("Type: ");
        let item_type = match type_choice.trim() {
            "1" => self.create_weapon(),
            "2" => self.create_armor(),
            "3" => self.create_accessory(),
            "4" => self.create_consumable(),
            "5" => self.create_ammo(),
            "6" => self.create_quest_item(),
            _ => {
                println!("❌ Invalid item type.");
                return;
            }
        };

        // Get costs
        let base_cost = self.read_u32("Base cost (gold): ", 0);
        let sell_cost = self.read_u32("Sell cost (gold): ", base_cost / 2);

        // Get class restrictions
        let disablements = self.select_class_restrictions();

        // Optional bonuses
        let constant_bonus = self.select_bonus("Constant bonus (passive)", true);
        let temporary_bonus = self.select_bonus("Temporary bonus (on use)", true);

        // Spell effect (optional)
        let spell_effect = self.read_optional_u16("Spell effect ID (0 for none): ");

        // Max charges
        let max_charges = if spell_effect.is_some() || temporary_bonus.is_some() {
            self.read_u16("Max charges: ", 0)
        } else {
            0
        };

        // Cursed?
        let is_cursed = self.read_bool("Is cursed? (y/n): ");

        let item = Item {
            id,
            name,
            item_type,
            base_cost,
            sell_cost,
            disablements,
            constant_bonus,
            temporary_bonus,
            spell_effect,
            max_charges,
            is_cursed,
            icon_path: None,
        };

        self.items.push(item);
        self.items.sort_by(|a, b| a.id.cmp(&b.id));
        self.modified = true;

        println!("✅ Item added successfully!");
    }

    /// Creates weapon data
    fn create_weapon(&self) -> ItemType {
        println!("\n  Weapon Configuration:");

        let damage = self.read_dice_roll("Damage dice (format: 1d8 or 2d6+1): ");
        let bonus = self.read_i8("To-hit/damage bonus: ", 0);
        let hands_required = self.read_u8("Hands required (1 or 2): ", 1);

        ItemType::Weapon(WeaponData {
            damage,
            bonus,
            hands_required,
        })
    }

    /// Creates armor data
    fn create_armor(&self) -> ItemType {
        println!("\n  Armor Configuration:");

        let ac_bonus = self.read_u8("AC bonus: ", 0);
        let weight = self.read_u8("Weight (pounds): ", 0);

        ItemType::Armor(ArmorData { ac_bonus, weight })
    }

    /// Creates accessory data
    fn create_accessory(&self) -> ItemType {
        println!("\n  Accessory Type:");
        println!("    [1] Ring");
        println!("    [2] Amulet");
        println!("    [3] Belt");
        println!("    [4] Cloak");

        let choice = self.read_input("  Slot: ");
        let slot = match choice.trim() {
            "1" => AccessorySlot::Ring,
            "2" => AccessorySlot::Amulet,
            "3" => AccessorySlot::Belt,
            "4" => AccessorySlot::Cloak,
            _ => AccessorySlot::Ring,
        };

        ItemType::Accessory(AccessoryData { slot })
    }

    /// Creates consumable data
    fn create_consumable(&self) -> ItemType {
        println!("\n  Consumable Effect:");
        println!("    [1] Heal HP");
        println!("    [2] Restore SP");
        println!("    [3] Cure Condition");
        println!("    [4] Boost Attribute");

        let choice = self.read_input("  Effect: ");
        let effect = match choice.trim() {
            "1" => {
                let amount = self.read_u16("HP to heal: ", 10);
                ConsumableEffect::HealHp(amount)
            }
            "2" => {
                let amount = self.read_u16("SP to restore: ", 10);
                ConsumableEffect::RestoreSp(amount)
            }
            "3" => {
                let flags = self.read_u8("Condition flags to clear (0-255): ", 0);
                ConsumableEffect::CureCondition(flags)
            }
            "4" => {
                let attr = self.select_attribute_type();
                let boost = self.read_i8("Boost amount: ", 1);
                ConsumableEffect::BoostAttribute(attr, boost)
            }
            _ => ConsumableEffect::HealHp(10),
        };

        let is_combat_usable = self.read_bool("Usable in combat? (y/n): ");

        ItemType::Consumable(ConsumableData {
            effect,
            is_combat_usable,
        })
    }

    /// Creates ammo data
    fn create_ammo(&self) -> ItemType {
        println!("\n  Ammunition Type:");
        println!("    [1] Arrow");
        println!("    [2] Bolt");
        println!("    [3] Stone");

        let choice = self.read_input("  Type: ");
        let ammo_type = match choice.trim() {
            "1" => AmmoType::Arrow,
            "2" => AmmoType::Bolt,
            "3" => AmmoType::Stone,
            _ => AmmoType::Arrow,
        };

        let quantity = self.read_u16("Quantity per bundle: ", 20);

        ItemType::Ammo(AmmoData {
            ammo_type,
            quantity,
        })
    }

    /// Creates quest item data
    fn create_quest_item(&self) -> ItemType {
        println!("\n  Quest Item Configuration:");

        let quest_id = self.read_input("Quest ID: ").trim().to_string();
        let is_key_item = self.read_bool("Is key item (cannot drop/sell)? (y/n): ");

        ItemType::Quest(QuestData {
            quest_id,
            is_key_item,
        })
    }

    /// Selects an attribute type
    fn select_attribute_type(&self) -> AttributeType {
        println!("    Attribute:");
        println!("      [1] Might");
        println!("      [2] Intellect");
        println!("      [3] Personality");
        println!("      [4] Endurance");
        println!("      [5] Speed");
        println!("      [6] Accuracy");
        println!("      [7] Luck");

        let choice = self.read_input("    Choice: ");
        match choice.trim() {
            "1" => AttributeType::Might,
            "2" => AttributeType::Intellect,
            "3" => AttributeType::Personality,
            "4" => AttributeType::Endurance,
            "5" => AttributeType::Speed,
            "6" => AttributeType::Accuracy,
            "7" => AttributeType::Luck,
            _ => AttributeType::Might,
        }
    }

    /// Selects class restrictions
    fn select_class_restrictions(&self) -> Disablement {
        println!("\n  Class Restrictions:");
        println!("    [1] All classes can use (0xFF)");
        println!("    [2] No classes (quest item)");
        println!("    [3] Custom selection");

        let choice = self.read_input("  Choice: ");
        match choice.trim() {
            "1" => Disablement::ALL,
            "2" => Disablement::NONE,
            "3" => self.custom_class_selection(),
            _ => Disablement::ALL,
        }
    }

    /// Custom class restriction selection
    #[allow(deprecated)]
    fn custom_class_selection(&self) -> Disablement {
        println!("\n    Select classes that CAN use this item:");

        let knight = self.read_bool("    Knight? (y/n): ");
        let paladin = self.read_bool("    Paladin? (y/n): ");
        let archer = self.read_bool("    Archer? (y/n): ");
        let cleric = self.read_bool("    Cleric? (y/n): ");
        let sorcerer = self.read_bool("    Sorcerer? (y/n): ");
        let robber = self.read_bool("    Robber? (y/n): ");
        let good = self.read_bool("    Good alignment only? (y/n): ");
        let evil = self.read_bool("    Evil alignment only? (y/n): ");

        let mut flags = 0u8;
        if knight {
            flags |= Disablement::KNIGHT;
        }
        if paladin {
            flags |= Disablement::PALADIN;
        }
        if archer {
            flags |= Disablement::ARCHER;
        }
        if cleric {
            flags |= Disablement::CLERIC;
        }
        if sorcerer {
            flags |= Disablement::SORCERER;
        }
        if robber {
            flags |= Disablement::ROBBER;
        }
        if good {
            flags |= Disablement::GOOD;
        }
        if evil {
            flags |= Disablement::EVIL;
        }

        Disablement(flags)
    }

    /// Selects a bonus (optional)
    fn select_bonus(&self, label: &str, optional: bool) -> Option<Bonus> {
        if optional {
            let add = self.read_bool(&format!("\n  Add {}? (y/n): ", label));
            if !add {
                return None;
            }
        }

        println!("\n  Bonus Attribute:");
        println!("    [1] Might");
        println!("    [2] Intellect");
        println!("    [3] Personality");
        println!("    [4] Endurance");
        println!("    [5] Speed");
        println!("    [6] Accuracy");
        println!("    [7] Luck");
        println!("    [8] Fire Resistance");
        println!("    [9] Cold Resistance");
        println!("    [10] Electricity Resistance");
        println!("    [11] Acid Resistance");
        println!("    [12] Poison Resistance");
        println!("    [13] Magic Resistance");
        println!("    [14] Armor Class");

        let choice = self.read_input("  Attribute: ");
        let attribute = match choice.trim() {
            "1" => BonusAttribute::Might,
            "2" => BonusAttribute::Intellect,
            "3" => BonusAttribute::Personality,
            "4" => BonusAttribute::Endurance,
            "5" => BonusAttribute::Speed,
            "6" => BonusAttribute::Accuracy,
            "7" => BonusAttribute::Luck,
            "8" => BonusAttribute::ResistFire,
            "9" => BonusAttribute::ResistCold,
            "10" => BonusAttribute::ResistElectricity,
            "11" => BonusAttribute::ResistAcid,
            "12" => BonusAttribute::ResistPoison,
            "13" => BonusAttribute::ResistMagic,
            "14" => BonusAttribute::ArmorClass,
            _ => BonusAttribute::Might,
        };

        let value = self.read_i8("  Value: ", 1);

        Some(Bonus { attribute, value })
    }

    /// Edits an existing item
    fn edit_item(&mut self) {
        if self.items.is_empty() {
            println!("📦 No items to edit.");
            return;
        }

        let id_str = self.read_input("Enter item ID to edit: ");
        let id: ItemId = match id_str.trim().parse() {
            Ok(id) => id,
            Err(_) => {
                println!("❌ Invalid ID.");
                return;
            }
        };

        let index = match self.items.iter().position(|item| item.id == id) {
            Some(idx) => idx,
            None => {
                println!("❌ Item with ID {} not found.", id);
                return;
            }
        };

        println!("\n  Editing: {}", self.items[index].name);
        println!("  Note: For now, delete and re-add to change item data.");
        println!("        This preserves structural integrity.");

        // Future enhancement: implement full edit mode
        println!("\n  Press Enter to return...");
        self.read_input("");
    }

    /// Deletes an item
    fn delete_item(&mut self) {
        if self.items.is_empty() {
            println!("📦 No items to delete.");
            return;
        }

        let id_str = self.read_input("Enter item ID to delete: ");
        let id: ItemId = match id_str.trim().parse() {
            Ok(id) => id,
            Err(_) => {
                println!("❌ Invalid ID.");
                return;
            }
        };

        let index = match self.items.iter().position(|item| item.id == id) {
            Some(idx) => idx,
            None => {
                println!("❌ Item with ID {} not found.", id);
                return;
            }
        };

        let item_name = self.items[index].name.clone();
        let confirm = self.read_bool(&format!("Delete \"{}\"? (y/n): ", item_name));

        if confirm {
            self.items.remove(index);
            self.modified = true;
            println!("✅ Item deleted.");
        } else {
            println!("❌ Deletion cancelled.");
        }
    }

    /// Previews an item
    fn preview_item(&self) {
        if self.items.is_empty() {
            println!("📦 No items to preview.");
            return;
        }

        let id_str = self.read_input("Enter item ID to preview: ");
        let id: ItemId = match id_str.trim().parse() {
            Ok(id) => id,
            Err(_) => {
                println!("❌ Invalid ID.");
                return;
            }
        };

        let item = match self.items.iter().find(|item| item.id == id) {
            Some(item) => item,
            None => {
                println!("❌ Item with ID {} not found.", id);
                return;
            }
        };

        println!("\n════════════════════════════════════════");
        println!("  ITEM PREVIEW");
        println!("════════════════════════════════════════");
        println!("  ID: {}", item.id);
        println!("  Name: {}", item.name);

        match &item.item_type {
            ItemType::Weapon(w) => {
                println!("  Type: Weapon");
                println!(
                    "  Damage: {}d{}+{}",
                    w.damage.count, w.damage.sides, w.damage.bonus
                );
                println!("  Bonus: {}", w.bonus);
                println!("  Hands: {}", w.hands_required);
            }
            ItemType::Armor(a) => {
                println!("  Type: Armor");
                println!("  AC Bonus: {}", a.ac_bonus);
                println!("  Weight: {} lbs", a.weight);
            }
            ItemType::Accessory(acc) => {
                println!("  Type: Accessory");
                println!("  Slot: {:?}", acc.slot);
            }
            ItemType::Consumable(c) => {
                println!("  Type: Consumable");
                println!("  Effect: {:?}", c.effect);
                println!("  Combat Usable: {}", c.is_combat_usable);
            }
            ItemType::Ammo(ammo) => {
                println!("  Type: Ammunition");
                println!("  Ammo Type: {:?}", ammo.ammo_type);
                println!("  Quantity: {}", ammo.quantity);
            }
            ItemType::Quest(q) => {
                println!("  Type: Quest Item");
                println!("  Quest ID: {}", q.quest_id);
                println!("  Key Item: {}", q.is_key_item);
            }
        }

        println!("  Base Cost: {} gp", item.base_cost);
        println!("  Sell Cost: {} gp", item.sell_cost);
        println!("  Disablement Flags: 0x{:02X}", item.disablements.0);

        if let Some(bonus) = &item.constant_bonus {
            println!("  Constant Bonus: {:?} {:+}", bonus.attribute, bonus.value);
        }
        if let Some(bonus) = &item.temporary_bonus {
            println!("  Temporary Bonus: {:?} {:+}", bonus.attribute, bonus.value);
        }
        if let Some(spell) = item.spell_effect {
            println!("  Spell Effect: 0x{:04X}", spell);
        }
        if item.max_charges > 0 {
            println!("  Max Charges: {}", item.max_charges);
        }
        if item.is_cursed {
            println!("  💀 CURSED");
        }
        if item.is_magical() {
            println!("  ✨ MAGICAL");
        }

        println!("\n  Press Enter to return...");
        self.read_input("");
    }

    /// Saves items to file
    fn save(&self) -> bool {
        println!("\n💾 Saving to {}...", self.file_path.display());

        // Create parent directory if needed
        if let Some(parent) = self.file_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                println!("❌ Failed to create directory: {}", e);
                return false;
            }
        }

        let ron_config = ron::ser::PrettyConfig::new()
            .depth_limit(4)
            .separate_tuple_members(false)
            .enumerate_arrays(false);

        let serialized = match ron::ser::to_string_pretty(&self.items, ron_config) {
            Ok(s) => s,
            Err(e) => {
                println!("❌ Serialization error: {}", e);
                return false;
            }
        };

        // Add header comment
        let contents = format!(
            "// items.ron - Item definitions\n\
             //\n\
             // Generated by item_editor\n\
             // Total items: {}\n\n{}",
            self.items.len(),
            serialized
        );

        if let Err(e) = fs::write(&self.file_path, contents) {
            println!("❌ Write error: {}", e);
            return false;
        }

        true
    }

    /// Confirms exit without saving
    fn confirm_exit(&self) -> bool {
        if self.modified {
            println!("\n⚠️  You have unsaved changes!");
            self.read_bool("Discard changes and exit? (y/n): ")
        } else {
            true
        }
    }

    /// Gets next available item ID
    fn next_item_id(&self) -> ItemId {
        self.items.iter().map(|item| item.id).max().unwrap_or(0) + 1
    }

    // ===== Input Helpers =====

    fn read_input(&self, prompt: &str) -> String {
        print!("{}", prompt);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input
    }

    fn read_u8(&self, prompt: &str, default: u8) -> u8 {
        let input = self.read_input(prompt);
        input.trim().parse().unwrap_or(default)
    }

    fn read_u16(&self, prompt: &str, default: u16) -> u16 {
        let input = self.read_input(prompt);
        input.trim().parse().unwrap_or(default)
    }

    fn read_u32(&self, prompt: &str, default: u32) -> u32 {
        let input = self.read_input(prompt);
        input.trim().parse().unwrap_or(default)
    }

    fn read_i8(&self, prompt: &str, default: i8) -> i8 {
        let input = self.read_input(prompt);
        input.trim().parse().unwrap_or(default)
    }

    fn read_optional_u16(&self, prompt: &str) -> Option<u16> {
        let input = self.read_input(prompt);
        match input.trim().parse::<u16>() {
            Ok(0) => None,
            Ok(val) => Some(val),
            Err(_) => None,
        }
    }

    fn read_bool(&self, prompt: &str) -> bool {
        let input = self.read_input(prompt);
        matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
    }

    fn read_dice_roll(&self, prompt: &str) -> DiceRoll {
        let input = self.read_input(prompt);
        let input = input.trim();

        // Parse format: 1d8, 2d6+1, etc.
        if let Some((count_str, rest)) = input.split_once('d') {
            let count: u8 = count_str.parse().unwrap_or(1);

            if let Some((sides_str, bonus_str)) = rest.split_once('+') {
                let sides: u8 = sides_str.parse().unwrap_or(6);
                let bonus: i8 = bonus_str.parse().unwrap_or(0);
                return DiceRoll::new(count, sides, bonus);
            } else if let Some((sides_str, bonus_str)) = rest.split_once('-') {
                let sides: u8 = sides_str.parse().unwrap_or(6);
                let bonus: i8 = bonus_str.parse::<i8>().unwrap_or(0);
                return DiceRoll::new(count, sides, -bonus);
            } else {
                let sides: u8 = rest.parse().unwrap_or(6);
                return DiceRoll::new(count, sides, 0);
            }
        }

        // Default
        DiceRoll::new(1, 6, 0)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let file_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("data/items.ron")
    };

    let mut editor = match ItemEditor::load(file_path) {
        Ok(editor) => editor,
        Err(e) => {
            eprintln!("❌ Failed to load items: {}", e);
            process::exit(1);
        }
    };

    editor.run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_item_id_empty() {
        let editor = ItemEditor {
            items: vec![],
            file_path: PathBuf::from("test.ron"),
            modified: false,
        };

        assert_eq!(editor.next_item_id(), 1);
    }

    #[test]
    fn test_next_item_id_with_items() {
        let editor = ItemEditor {
            items: vec![
                Item {
                    id: 1,
                    name: "Item 1".to_string(),
                    item_type: ItemType::Weapon(WeaponData {
                        damage: DiceRoll::new(1, 6, 0),
                        bonus: 0,
                        hands_required: 1,
                    }),
                    base_cost: 10,
                    sell_cost: 5,
                    disablements: Disablement::ALL,
                    constant_bonus: None,
                    temporary_bonus: None,
                    spell_effect: None,
                    max_charges: 0,
                    is_cursed: false,
                    icon_path: None,
                },
                Item {
                    id: 5,
                    name: "Item 5".to_string(),
                    item_type: ItemType::Armor(ArmorData {
                        ac_bonus: 5,
                        weight: 20,
                    }),
                    base_cost: 50,
                    sell_cost: 25,
                    disablements: Disablement::ALL,
                    constant_bonus: None,
                    temporary_bonus: None,
                    spell_effect: None,
                    max_charges: 0,
                    is_cursed: false,
                    icon_path: None,
                },
            ],
            file_path: PathBuf::from("test.ron"),
            modified: false,
        };

        assert_eq!(editor.next_item_id(), 6);
    }

    #[test]
    #[allow(deprecated)]
    fn test_custom_class_selection_all_flags() {
        // This test validates the bit flags are correctly set
        let flags = Disablement::KNIGHT
            | Disablement::PALADIN
            | Disablement::ARCHER
            | Disablement::CLERIC
            | Disablement::SORCERER
            | Disablement::ROBBER;

        let dis = Disablement(flags);
        assert!(dis.can_use_class(Disablement::KNIGHT));
        assert!(dis.can_use_class(Disablement::PALADIN));
        assert!(dis.can_use_class(Disablement::ARCHER));
        assert!(dis.can_use_class(Disablement::CLERIC));
        assert!(dis.can_use_class(Disablement::SORCERER));
        assert!(dis.can_use_class(Disablement::ROBBER));
        assert!(!dis.good_only());
        assert!(!dis.evil_only());
    }

    #[test]
    fn test_disablement_all() {
        let dis = Disablement::ALL;
        assert_eq!(dis.0, 0xFF);
    }

    #[test]
    fn test_disablement_none() {
        let dis = Disablement::NONE;
        assert_eq!(dis.0, 0x00);
    }
}
