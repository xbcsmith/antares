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
    AccessoryData, AccessorySlot, AlignmentRestriction, AmmoData, AmmoType, ArmorClassification,
    ArmorData, AttributeType, Bonus, BonusAttribute, ConsumableData, ConsumableEffect, Item,
    ItemType, MagicItemClassification, QuestData, WeaponClassification, WeaponData,
};
use antares::domain::types::{DiceRoll, ItemId};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

/// Standard item tags used for race restrictions and item properties
const STANDARD_ITEM_TAGS: &[&str] = &[
    "large_weapon",
    "two_handed",
    "heavy_armor",
    "elven_crafted",
    "dwarven_crafted",
    "requires_strength",
];

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

        // Item tags (proficiency system now handles restrictions)
        let tags = self.input_item_tags();

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

        // Alignment restriction
        let alignment_restriction = self.select_alignment_restriction();

        #[allow(deprecated)]
        let item = Item {
            id,
            name,
            item_type,
            base_cost,
            sell_cost,

            alignment_restriction,
            constant_bonus,
            temporary_bonus,
            spell_effect,
            max_charges,
            is_cursed,
            icon_path: None,
            tags,
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
        let classification = self.select_weapon_classification();

        ItemType::Weapon(WeaponData {
            damage,
            bonus,
            hands_required,
            classification,
        })
    }

    /// Creates armor data
    fn create_armor(&self) -> ItemType {
        println!("\n  Armor Configuration:");

        let ac_bonus = self.read_u8("AC bonus: ", 0);
        let weight = self.read_u8("Weight (pounds): ", 0);
        let classification = self.select_armor_classification();

        ItemType::Armor(ArmorData {
            ac_bonus,
            weight,
            classification,
        })
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

        let classification = self.select_magic_item_classification();

        ItemType::Accessory(AccessoryData {
            slot,
            classification,
        })
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

    // Disablement functions removed - proficiency system now handles restrictions

    /// Selects weapon classification
    fn select_weapon_classification(&self) -> WeaponClassification {
        println!("\n  Weapon Classification:");
        println!("    [1] Simple (clubs, daggers, staffs - anyone can use)");
        println!("    [2] Martial Melee (swords, axes - fighters)");
        println!("    [3] Martial Ranged (bows, crossbows - archers)");
        println!("    [4] Blunt (maces, hammers - clerics)");
        println!("    [5] Unarmed (fists, martial arts)");

        let choice = self.read_input("  Classification: ");
        match choice.trim() {
            "1" => WeaponClassification::Simple,
            "2" => WeaponClassification::MartialMelee,
            "3" => WeaponClassification::MartialRanged,
            "4" => WeaponClassification::Blunt,
            "5" => WeaponClassification::Unarmed,
            _ => WeaponClassification::Simple,
        }
    }

    /// Selects armor classification
    fn select_armor_classification(&self) -> ArmorClassification {
        println!("\n  Armor Classification:");
        println!("    [1] Light (leather, padded)");
        println!("    [2] Medium (chain mail, scale)");
        println!("    [3] Heavy (plate mail, full plate)");
        println!("    [4] Shield (all shield types)");

        let choice = self.read_input("  Classification: ");
        match choice.trim() {
            "1" => ArmorClassification::Light,
            "2" => ArmorClassification::Medium,
            "3" => ArmorClassification::Heavy,
            "4" => ArmorClassification::Shield,
            _ => ArmorClassification::Light,
        }
    }

    /// Selects magic item classification (for accessories)
    fn select_magic_item_classification(&self) -> Option<MagicItemClassification> {
        println!("\n  Magic Item Classification:");
        println!("    [1] None (mundane accessory)");
        println!("    [2] Arcane (wands, arcane scrolls - sorcerers)");
        println!("    [3] Divine (holy symbols, divine scrolls - clerics)");
        println!("    [4] Universal (potions, rings - anyone)");

        let choice = self.read_input("  Classification: ");
        match choice.trim() {
            "1" => None,
            "2" => Some(MagicItemClassification::Arcane),
            "3" => Some(MagicItemClassification::Divine),
            "4" => Some(MagicItemClassification::Universal),
            _ => None,
        }
    }

    /// Selects alignment restriction
    fn select_alignment_restriction(&self) -> Option<AlignmentRestriction> {
        println!("\n  Alignment Restriction:");
        println!("    [1] None (any alignment can use)");
        println!("    [2] Good only");
        println!("    [3] Evil only");

        let choice = self.read_input("  Restriction: ");
        match choice.trim() {
            "1" => None,
            "2" => Some(AlignmentRestriction::GoodOnly),
            "3" => Some(AlignmentRestriction::EvilOnly),
            _ => None,
        }
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

    /// Inputs item tags with validation
    fn input_item_tags(&self) -> Vec<String> {
        println!("\n╔════════════════════════════════════════╗");
        println!("║        ITEM TAGS SELECTION             ║");
        println!("╚════════════════════════════════════════╝");
        println!("\nStandard Item Tags:");
        println!("  • large_weapon       - Large/oversized weapons (restricted by small races)");
        println!("  • two_handed         - Two-handed weapons (requires both hands)");
        println!("  • heavy_armor        - Heavy armor pieces (restricted by small races)");
        println!("  • elven_crafted      - Elven-crafted items (may have race restrictions)");
        println!("  • dwarven_crafted    - Dwarven-crafted items (may have race restrictions)");
        println!("  • requires_strength  - Items requiring high strength");

        println!("\n📝 Tags are used for race restrictions (incompatible_item_tags).");
        println!("   Example: A halfling with 'large_weapon' incompatible cannot use items tagged 'large_weapon'.");
        println!("\nEnter item tags (one per line, or leave empty):");
        println!("   Example: large_weapon");

        let tags = self.input_multistring_values("", "Tag: ");

        if tags.is_empty() {
            return Vec::new();
        }

        // Validate tags (standard first; confirm unknown tags)
        let mut valid_tags = filter_valid_tags(&tags);
        for tag in &tags {
            if valid_tags.contains(tag) {
                continue;
            }
            println!("⚠️  Warning: '{}' is not a standard item tag", tag);
            println!("   Standard tags: {}", STANDARD_ITEM_TAGS.join(", "));
            let confirm = self.read_input(&format!("   Include '{}' anyway? (y/n): ", tag));
            if confirm.trim().eq_ignore_ascii_case("y") {
                valid_tags.push(tag.clone());
            }
        }

        if !valid_tags.is_empty() {
            println!("✅ Added tags: {}", valid_tags.join(", "));
        }

        valid_tags
    }

    /// Edits an existing item
    fn edit_item(&mut self) {
        if self.items.is_empty() {
            println!("📦 No items to edit.");
            return;
        }

        self.list_items();

        let idx = loop {
            let input = self.read_input("\nEnter item index to edit (or 'c' to cancel): ");
            if input.trim().eq_ignore_ascii_case("c") {
                return;
            }

            match input.trim().parse::<usize>() {
                Ok(i) if i < self.items.len() => break i,
                _ => println!("❌ Invalid index. Please try again."),
            }
        };

        loop {
            let item = &self.items[idx];
            println!("\n╔════════════════════════════════════════╗");
            println!(
                "║        EDIT ITEM: {:19} ║",
                format!(
                    "{:19}",
                    if item.name.len() > 19 {
                        &item.name[..19]
                    } else {
                        &item.name
                    }
                )
            );
            println!("╚════════════════════════════════════════╝");

            println!("\nWhat would you like to edit?");
            println!("  1. Name (currently: {})", item.name);
            println!("  2. Base Cost (currently: {}g)", item.base_cost);
            println!("  3. Sell Cost (currently: {}g)", item.sell_cost);
            println!(
                "  4. Classification (currently: {})",
                self.format_classification(&item.item_type)
            );
            println!(
                "  5. Tags (currently: {})",
                if item.tags.is_empty() {
                    "None".to_string()
                } else {
                    item.tags.join(", ")
                }
            );
            println!(
                "  6. Alignment Restriction (currently: {})",
                match item.alignment_restriction {
                    Some(AlignmentRestriction::GoodOnly) => "Good Only",
                    Some(AlignmentRestriction::EvilOnly) => "Evil Only",
                    None => "None",
                }
            );
            println!("  7. Max Charges (currently: {})", item.max_charges);
            println!("  8. Cursed Status (currently: {})", item.is_cursed);
            println!("  s. Save and return");
            println!("  c. Cancel (discard changes)");

            let choice = self.read_input("\nChoice: ");

            match choice.trim() {
                "1" => {
                    let new_name = self.read_input("New name: ");
                    if !new_name.trim().is_empty() {
                        self.items[idx].name = new_name.trim().to_string();
                        self.modified = true;
                        println!("✅ Name updated");
                    } else {
                        println!("❌ Name cannot be empty");
                    }
                }
                "2" => {
                    let cost = self.read_u32("New base cost (gold): ", item.base_cost);
                    self.items[idx].base_cost = cost;
                    self.modified = true;
                    println!("✅ Base cost updated");
                }
                "3" => {
                    let cost = self.read_u32("New sell cost (gold): ", item.sell_cost);
                    self.items[idx].sell_cost = cost;
                    self.modified = true;
                    println!("✅ Sell cost updated");
                }
                "4" => {
                    self.edit_item_classification(idx);
                }
                "5" => {
                    let tags = self.input_item_tags();
                    self.items[idx].tags = tags;
                    self.modified = true;
                    println!("✅ Tags updated");
                }
                "6" => {
                    let restriction = self.select_alignment_restriction();
                    self.items[idx].alignment_restriction = restriction;
                    self.modified = true;
                    println!("✅ Alignment restriction updated");
                }
                "7" => {
                    let charges =
                        self.read_u16("New max charges (0 for non-magical): ", item.max_charges);
                    self.items[idx].max_charges = charges;
                    self.modified = true;
                    println!("✅ Max charges updated");
                }
                "8" => {
                    let cursed = self.read_bool("Is cursed? (y/n): ");
                    self.items[idx].is_cursed = cursed;
                    self.modified = true;
                    println!("✅ Cursed status updated");
                }
                "s" | "S" => {
                    println!("✅ Changes saved");
                    return;
                }
                "c" | "C" => {
                    let confirm = self.read_input("⚠️  Discard all changes? (yes/no): ");
                    if confirm.trim().eq_ignore_ascii_case("yes") {
                        println!("Changes discarded");
                        return;
                    }
                }
                _ => println!("❌ Invalid choice"),
            }
        }
    }

    /// Edit item classification (type-specific properties)
    fn edit_item_classification(&mut self, idx: usize) {
        let item = &self.items[idx];

        println!("\n╔════════════════════════════════════════╗");
        println!("║        EDIT CLASSIFICATION             ║");
        println!("╚════════════════════════════════════════╝");

        match &item.item_type {
            ItemType::Weapon(data) => {
                println!("\nCurrent weapon classification: {:?}", data.classification);
                println!(
                    "Current damage: {}d{}+{}",
                    data.damage.count, data.damage.sides, data.damage.bonus
                );
                println!("Current bonus: {}", data.bonus);
                println!("Current hands required: {}", data.hands_required);

                println!("\nWhat would you like to edit?");
                println!("  1. Weapon Classification");
                println!("  2. Damage");
                println!("  3. Bonus");
                println!("  4. Hands Required");
                println!("  c. Cancel");

                let choice = self.read_input("\nChoice: ");
                match choice.trim() {
                    "1" => {
                        let classification = self.select_weapon_classification();
                        if let ItemType::Weapon(ref mut weapon_data) = self.items[idx].item_type {
                            weapon_data.classification = classification;
                            self.modified = true;
                            println!("✅ Weapon classification updated");
                        }
                    }
                    "2" => {
                        let damage = self.read_dice_roll("New damage (e.g., 1d8+2): ");
                        if let ItemType::Weapon(ref mut weapon_data) = self.items[idx].item_type {
                            weapon_data.damage = damage;
                            self.modified = true;
                            println!("✅ Damage updated");
                        }
                    }
                    "3" => {
                        let bonus = self.read_i8("New bonus (can be negative): ", data.bonus);
                        if let ItemType::Weapon(ref mut weapon_data) = self.items[idx].item_type {
                            weapon_data.bonus = bonus;
                            self.modified = true;
                            println!("✅ Bonus updated");
                        }
                    }
                    "4" => {
                        let hands = self.read_u8("New hands required (1-2): ", data.hands_required);
                        if (1..=2).contains(&hands) {
                            if let ItemType::Weapon(ref mut weapon_data) = self.items[idx].item_type
                            {
                                weapon_data.hands_required = hands;
                                self.modified = true;
                                println!("✅ Hands required updated");
                            }
                        } else {
                            println!("❌ Hands required must be 1 or 2");
                        }
                    }
                    "c" | "C" => println!("Cancelled"),
                    _ => println!("❌ Invalid choice"),
                }
            }
            ItemType::Armor(data) => {
                println!("\nCurrent armor classification: {:?}", data.classification);
                println!("Current AC bonus: {}", data.ac_bonus);

                println!("\nWhat would you like to edit?");
                println!("  1. Armor Classification");
                println!("  2. AC Bonus");
                println!("  c. Cancel");

                let choice = self.read_input("\nChoice: ");
                match choice.trim() {
                    "1" => {
                        let classification = self.select_armor_classification();
                        if let ItemType::Armor(ref mut armor_data) = self.items[idx].item_type {
                            armor_data.classification = classification;
                            self.modified = true;
                            println!("✅ Armor classification updated");
                        }
                    }
                    "2" => {
                        let ac_bonus = self.read_u8("New AC bonus: ", data.ac_bonus);
                        if let ItemType::Armor(ref mut armor_data) = self.items[idx].item_type {
                            armor_data.ac_bonus = ac_bonus;
                            self.modified = true;
                            println!("✅ AC bonus updated");
                        }
                    }
                    "c" | "C" => println!("Cancelled"),
                    _ => println!("❌ Invalid choice"),
                }
            }
            ItemType::Accessory(data) => {
                println!("\nCurrent slot: {:?}", data.slot);
                println!("Current classification: {:?}", data.classification);

                println!("\nWhat would you like to edit?");
                println!("  1. Accessory Slot");
                println!("  2. Magic Item Classification");
                println!("  c. Cancel");

                let choice = self.read_input("\nChoice: ");
                match choice.trim() {
                    "1" => {
                        println!("\nSelect accessory slot:");
                        println!("  1. Ring");
                        println!("  2. Amulet");
                        println!("  3. Belt");
                        println!("  4. Cloak");

                        let slot_choice = self.read_input("Choice: ");
                        let slot = match slot_choice.trim() {
                            "1" => AccessorySlot::Ring,
                            "2" => AccessorySlot::Amulet,
                            "3" => AccessorySlot::Belt,
                            "4" => AccessorySlot::Cloak,
                            _ => {
                                println!("❌ Invalid choice");
                                return;
                            }
                        };

                        if let ItemType::Accessory(ref mut acc_data) = self.items[idx].item_type {
                            acc_data.slot = slot;
                            self.modified = true;
                            println!("✅ Accessory slot updated");
                        }
                    }
                    "2" => {
                        let classification = self.select_magic_item_classification();
                        if let ItemType::Accessory(ref mut acc_data) = self.items[idx].item_type {
                            acc_data.classification = classification;
                            self.modified = true;
                            println!("✅ Magic item classification updated");
                        }
                    }
                    "c" | "C" => println!("Cancelled"),
                    _ => println!("❌ Invalid choice"),
                }
            }
            ItemType::Consumable(data) => {
                println!("\nCurrent effect: {:?}", data.effect);
                println!("Current combat usable: {}", data.is_combat_usable);

                println!("\nWhat would you like to edit?");
                println!("  1. Effect Type");
                println!("  2. Combat Usable");
                println!("  c. Cancel");

                let choice = self.read_input("\nChoice: ");
                match choice.trim() {
                    "1" => {
                        println!("\nSelect consumable effect:");
                        println!("  1. Heal HP (specify amount)");
                        println!("  2. Restore SP (specify amount)");
                        println!("  3. Cure Condition (specify condition flags)");
                        println!("  4. Boost Attribute");

                        let effect_choice = self.read_input("Choice: ");
                        let effect = match effect_choice.trim() {
                            "1" => {
                                let amount = self.read_u16("HP to heal: ", 20);
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
                                println!("\nSelect attribute:");
                                println!("  1. Might");
                                println!("  2. Intellect");
                                println!("  3. Personality");
                                println!("  4. Endurance");
                                println!("  5. Speed");
                                println!("  6. Accuracy");
                                println!("  7. Luck");
                                let attr_choice = self.read_input("Choice: ");
                                let attr = match attr_choice.trim() {
                                    "1" => AttributeType::Might,
                                    "2" => AttributeType::Intellect,
                                    "3" => AttributeType::Personality,
                                    "4" => AttributeType::Endurance,
                                    "5" => AttributeType::Speed,
                                    "6" => AttributeType::Accuracy,
                                    "7" => AttributeType::Luck,
                                    _ => {
                                        println!("❌ Invalid choice");
                                        return;
                                    }
                                };
                                let boost = self.read_i8("Boost amount (can be negative): ", 1);
                                ConsumableEffect::BoostAttribute(attr, boost)
                            }
                            _ => {
                                println!("❌ Invalid choice");
                                return;
                            }
                        };

                        if let ItemType::Consumable(ref mut cons_data) = self.items[idx].item_type {
                            cons_data.effect = effect;
                            self.modified = true;
                            println!("✅ Consumable effect updated");
                        }
                    }
                    "2" => {
                        let usable = self.read_bool("Combat usable? (y/n): ");
                        if let ItemType::Consumable(ref mut cons_data) = self.items[idx].item_type {
                            cons_data.is_combat_usable = usable;
                            self.modified = true;
                            println!("✅ Combat usable updated");
                        }
                    }
                    "c" | "C" => println!("Cancelled"),
                    _ => println!("❌ Invalid choice"),
                }
            }
            ItemType::Ammo(data) => {
                println!("\nCurrent ammo type: {:?}", data.ammo_type);
                println!("Current quantity: {}", data.quantity);

                println!("\nWhat would you like to edit?");
                println!("  1. Ammo Type");
                println!("  2. Quantity");
                println!("  c. Cancel");

                let choice = self.read_input("\nChoice: ");
                match choice.trim() {
                    "1" => {
                        println!("\nSelect ammo type:");
                        println!("  1. Arrow");
                        println!("  2. Bolt");
                        println!("  3. Stone");

                        let type_choice = self.read_input("Choice: ");
                        let ammo_type = match type_choice.trim() {
                            "1" => AmmoType::Arrow,
                            "2" => AmmoType::Bolt,
                            "3" => AmmoType::Stone,
                            _ => {
                                println!("❌ Invalid choice");
                                return;
                            }
                        };

                        if let ItemType::Ammo(ref mut ammo_data) = self.items[idx].item_type {
                            ammo_data.ammo_type = ammo_type;
                            self.modified = true;
                            println!("✅ Ammo type updated");
                        }
                    }
                    "2" => {
                        let quantity = self.read_u16("New quantity: ", data.quantity);
                        if let ItemType::Ammo(ref mut ammo_data) = self.items[idx].item_type {
                            ammo_data.quantity = quantity;
                            self.modified = true;
                            println!("✅ Quantity updated");
                        }
                    }
                    "c" | "C" => println!("Cancelled"),
                    _ => println!("❌ Invalid choice"),
                }
            }
            ItemType::Quest(_) => {
                println!("\nQuest items have no editable classification properties.");
                println!("Press Enter to continue...");
                self.read_input("");
            }
        }
    }

    /// Format classification for display
    fn format_classification(&self, item_type: &ItemType) -> String {
        match item_type {
            ItemType::Weapon(data) => format!("Weapon - {:?}", data.classification),
            ItemType::Armor(data) => format!("Armor - {:?}", data.classification),
            ItemType::Accessory(data) => {
                format!("Accessory - {:?}/{:?}", data.slot, data.classification)
            }
            ItemType::Consumable(data) => format!("Consumable - {:?}", data.effect),
            ItemType::Ammo(data) => format!("Ammo - {:?}", data.ammo_type),
            ItemType::Quest(_) => "Quest Item".to_string(),
        }
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

        // Show alignment restriction
        match &item.alignment_restriction {
            Some(AlignmentRestriction::GoodOnly) => println!("  Alignment: Good Only"),
            Some(AlignmentRestriction::EvilOnly) => println!("  Alignment: Evil Only"),
            None => println!("  Alignment: Any"),
        }

        // Show tags
        if !item.tags.is_empty() {
            println!("  Tags: {}", item.tags.join(", "));
        }

        // Show derived proficiency requirement
        if let Some(prof_id) = item.required_proficiency() {
            println!("  ⚔️  Required Proficiency: {}", prof_id);
        }

        // Disablement system removed - proficiency system now handles restrictions

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

/// Filters tags to include only standard item tags
fn filter_valid_tags(candidates: &[String]) -> Vec<String> {
    candidates
        .iter()
        .filter(|t| STANDARD_ITEM_TAGS.contains(&t.as_str()))
        .cloned()
        .collect()
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
    #[allow(deprecated)]
    fn test_next_item_id_with_items() {
        let editor = ItemEditor {
            items: vec![
                Item {
                    id: 1,
                    name: "Test Sword".to_string(),
                    item_type: ItemType::Weapon(WeaponData {
                        damage: DiceRoll::new(1, 8, 0),
                        bonus: 0,
                        hands_required: 1,
                        classification: WeaponClassification::MartialMelee,
                    }),
                    base_cost: 100,
                    sell_cost: 50,

                    alignment_restriction: None,
                    constant_bonus: None,
                    temporary_bonus: None,
                    spell_effect: None,
                    max_charges: 0,
                    is_cursed: false,
                    icon_path: None,
                    tags: vec![],
                },
                Item {
                    id: 5,
                    name: "Item 5".to_string(),
                    item_type: ItemType::Armor(ArmorData {
                        ac_bonus: 5,
                        weight: 20,
                        classification: ArmorClassification::Medium,
                    }),
                    base_cost: 50,
                    sell_cost: 25,

                    alignment_restriction: None,
                    constant_bonus: None,
                    temporary_bonus: None,
                    spell_effect: None,
                    max_charges: 0,
                    is_cursed: false,
                    icon_path: None,
                    tags: vec![],
                },
            ],
            file_path: PathBuf::from("test.ron"),
            modified: false,
        };

        assert_eq!(editor.next_item_id(), 6);
    }

    // Disablement-related tests removed - proficiency system now handles restrictions

    #[test]
    fn test_format_classification_weapon() {
        let editor = ItemEditor {
            items: vec![],
            file_path: PathBuf::from("test.ron"),
            modified: false,
        };

        let weapon_type = ItemType::Weapon(WeaponData {
            damage: DiceRoll::new(1, 8, 0),
            bonus: 0,
            hands_required: 1,
            classification: WeaponClassification::MartialMelee,
        });

        let result = editor.format_classification(&weapon_type);
        assert!(result.contains("Weapon"));
        assert!(result.contains("MartialMelee"));
    }

    #[test]
    fn test_format_classification_armor() {
        let editor = ItemEditor {
            items: vec![],
            file_path: PathBuf::from("test.ron"),
            modified: false,
        };

        let armor_type = ItemType::Armor(ArmorData {
            ac_bonus: 5,
            weight: 20,
            classification: ArmorClassification::Heavy,
        });

        let result = editor.format_classification(&armor_type);
        assert!(result.contains("Armor"));
        assert!(result.contains("Heavy"));
    }

    #[test]
    fn test_format_classification_accessory() {
        let editor = ItemEditor {
            items: vec![],
            file_path: PathBuf::from("test.ron"),
            modified: false,
        };

        let accessory_type = ItemType::Accessory(AccessoryData {
            slot: AccessorySlot::Ring,
            classification: Some(MagicItemClassification::Arcane),
        });

        let result = editor.format_classification(&accessory_type);
        assert!(result.contains("Accessory"));
        assert!(result.contains("Ring"));
        assert!(result.contains("Arcane"));
    }

    #[test]
    fn test_format_classification_consumable() {
        let editor = ItemEditor {
            items: vec![],
            file_path: PathBuf::from("test.ron"),
            modified: false,
        };

        let consumable_type = ItemType::Consumable(ConsumableData {
            effect: ConsumableEffect::HealHp(20),
            is_combat_usable: true,
        });

        let result = editor.format_classification(&consumable_type);
        assert!(result.contains("Consumable"));
        assert!(result.contains("HealHp"));
    }

    #[test]
    fn test_format_classification_ammo() {
        let editor = ItemEditor {
            items: vec![],
            file_path: PathBuf::from("test.ron"),
            modified: false,
        };

        let ammo_type = ItemType::Ammo(AmmoData {
            ammo_type: AmmoType::Arrow,
            quantity: 20,
        });

        let result = editor.format_classification(&ammo_type);
        assert!(result.contains("Ammo"));
        assert!(result.contains("Arrow"));
    }

    #[test]
    fn test_format_classification_quest() {
        let editor = ItemEditor {
            items: vec![],
            file_path: PathBuf::from("test.ron"),
            modified: false,
        };

        let quest_type = ItemType::Quest(QuestData {
            quest_id: "test_quest".to_string(),
            is_key_item: true,
        });

        let result = editor.format_classification(&quest_type);
        assert_eq!(result, "Quest Item");
    }

    #[test]
    #[allow(deprecated)]
    fn test_item_with_alignment_restriction() {
        let item = Item {
            id: 1,
            name: "Holy Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 8, 0),
                bonus: 2,
                hands_required: 1,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 500,
            sell_cost: 250,

            alignment_restriction: Some(AlignmentRestriction::GoodOnly),
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
        };

        assert_eq!(
            item.alignment_restriction,
            Some(AlignmentRestriction::GoodOnly)
        );
        assert_eq!(item.name, "Holy Sword");
    }

    #[test]
    #[allow(deprecated)]
    fn test_item_with_tags() {
        let item = Item {
            id: 1,
            name: "Great Axe".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(2, 6, 0),
                bonus: 0,
                hands_required: 2,
                classification: WeaponClassification::MartialMelee,
            }),
            base_cost: 150,
            sell_cost: 75,

            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec!["two_handed".to_string(), "large_weapon".to_string()],
        };

        assert_eq!(item.tags.len(), 2);
        assert!(item.tags.contains(&"two_handed".to_string()));
        assert!(item.tags.contains(&"large_weapon".to_string()));
    }

    #[test]
    #[allow(deprecated)]
    fn test_item_cursed() {
        let item = Item {
            id: 1,
            name: "Cursed Amulet".to_string(),
            item_type: ItemType::Accessory(AccessoryData {
                slot: AccessorySlot::Amulet,
                classification: None,
            }),
            base_cost: 100,
            sell_cost: 0,

            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: true,
            icon_path: None,
            tags: vec![],
        };

        assert!(item.is_cursed);
        assert!(item.is_accessory());
    }

    #[test]
    #[allow(deprecated)]
    fn test_item_with_charges() {
        let item = Item {
            id: 1,
            name: "Wand of Fireballs".to_string(),
            item_type: ItemType::Accessory(AccessoryData {
                slot: AccessorySlot::Ring,
                classification: Some(MagicItemClassification::Arcane),
            }),
            base_cost: 500,
            sell_cost: 250,

            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: Some(1), // Spell ID 1
            max_charges: 10,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
        };

        assert_eq!(item.max_charges, 10);
        assert_eq!(item.spell_effect, Some(1));
        assert!(item.is_accessory());
    }

    #[test]
    fn test_filter_valid_tags() {
        let candidates = vec![
            "large_weapon".to_string(),
            "not_a_tag".to_string(),
            "heavy_armor".to_string(),
        ];
        let filtered = filter_valid_tags(&candidates);
        assert_eq!(
            filtered,
            vec!["large_weapon".to_string(), "heavy_armor".to_string()]
        );
    }
}
