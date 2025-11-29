// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use antares::domain::items::types::{
    AccessoryData, AccessorySlot, AmmoData, AmmoType, ArmorData, AttributeType, Bonus,
    BonusAttribute, ConsumableData, ConsumableEffect, Disablement, Item, ItemType, QuestData,
    WeaponData,
};
use antares::domain::types::DiceRoll;
use eframe::egui;
use std::path::PathBuf;

/// Editor mode for items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemsEditorMode {
    List,
    Add,
    Edit,
}

/// Item type filter for search
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemTypeFilter {
    Weapon,
    Armor,
    Accessory,
    Consumable,
    Ammo,
    Quest,
}

impl ItemTypeFilter {
    pub fn matches(&self, item: &Item) -> bool {
        matches!(
            (self, &item.item_type),
            (ItemTypeFilter::Weapon, ItemType::Weapon(_))
                | (ItemTypeFilter::Armor, ItemType::Armor(_))
                | (ItemTypeFilter::Accessory, ItemType::Accessory(_))
                | (ItemTypeFilter::Consumable, ItemType::Consumable(_))
                | (ItemTypeFilter::Ammo, ItemType::Ammo(_))
                | (ItemTypeFilter::Quest, ItemType::Quest(_))
        )
    }

    pub fn as_str(&self) -> &str {
        match self {
            ItemTypeFilter::Weapon => "Weapon",
            ItemTypeFilter::Armor => "Armor",
            ItemTypeFilter::Accessory => "Accessory",
            ItemTypeFilter::Consumable => "Consumable",
            ItemTypeFilter::Ammo => "Ammo",
            ItemTypeFilter::Quest => "Quest",
        }
    }

    pub fn all() -> [ItemTypeFilter; 6] {
        [
            ItemTypeFilter::Weapon,
            ItemTypeFilter::Armor,
            ItemTypeFilter::Accessory,
            ItemTypeFilter::Consumable,
            ItemTypeFilter::Ammo,
            ItemTypeFilter::Quest,
        ]
    }
}

/// State for the items editor
pub struct ItemsEditorState {
    pub mode: ItemsEditorMode,
    pub search_query: String,
    pub selected_item: Option<usize>,
    pub edit_buffer: Item,
    pub show_import_dialog: bool,
    pub import_export_buffer: String,

    // Filters
    pub filter_type: Option<ItemTypeFilter>,
    pub filter_magical: Option<bool>,
    pub filter_cursed: Option<bool>,
    pub filter_quest: Option<bool>,
}

impl Default for ItemsEditorState {
    fn default() -> Self {
        Self {
            mode: ItemsEditorMode::List,
            search_query: String::new(),
            selected_item: None,
            edit_buffer: Self::default_item(),
            show_import_dialog: false,
            import_export_buffer: String::new(),
            filter_type: None,
            filter_magical: None,
            filter_cursed: None,
            filter_quest: None,
        }
    }
}

impl ItemsEditorState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn default_item() -> Item {
        Item {
            id: 0,
            name: "New Item".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 6, 0),
                bonus: 0,
                hands_required: 1,
            }),
            base_cost: 10,
            sell_cost: 5,
            is_cursed: false,
            disablements: Disablement(0),
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            icon_path: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        items: &mut Vec<Item>,
        campaign_dir: Option<&PathBuf>,
        items_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        file_load_merge_mode: &mut bool,
    ) {
        ui.heading("⚔️ Items Editor");
        ui.add_space(5.0);

        // Top toolbar
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            if ui.text_edit_singleline(&mut self.search_query).changed() {
                self.selected_item = None;
            }
            ui.separator();

            if ui.button("➕ Add Item").clicked() {
                self.mode = ItemsEditorMode::Add;
                self.edit_buffer = Self::default_item();
                // Calculate next ID
                let next_id = items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
                self.edit_buffer.id = next_id;
            }

            if ui.button("🔄 Reload").clicked() {
                // Reload logic
                if let Some(dir) = campaign_dir {
                    let path = dir.join(items_file);
                    if path.exists() {
                        match std::fs::read_to_string(&path) {
                            Ok(contents) => match ron::from_str::<Vec<Item>>(&contents) {
                                Ok(loaded_items) => {
                                    *items = loaded_items;
                                    *status_message =
                                        format!("Loaded items from: {}", path.display());
                                }
                                Err(e) => *status_message = format!("Failed to parse items: {}", e),
                            },
                            Err(e) => *status_message = format!("Failed to read items: {}", e),
                        }
                    }
                }
            }

            if ui.button("📥 Import").clicked() {
                self.show_import_dialog = true;
                self.import_export_buffer.clear();
            }

            ui.separator();

            // File I/O buttons
            if ui.button("📂 Load from File").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("RON", &["ron"])
                    .pick_file()
                {
                    let load_result = std::fs::read_to_string(&path).and_then(|contents| {
                        ron::from_str::<Vec<Item>>(&contents)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    });

                    match load_result {
                        Ok(loaded_items) => {
                            if *file_load_merge_mode {
                                // Merge: update existing, add new
                                for item in loaded_items {
                                    if let Some(existing) =
                                        items.iter_mut().find(|i| i.id == item.id)
                                    {
                                        *existing = item;
                                    } else {
                                        items.push(item);
                                    }
                                }
                            } else {
                                // Replace: clear and load
                                *items = loaded_items;
                            }
                            *unsaved_changes = true;
                            *status_message = format!("Loaded items from: {}", path.display());
                        }
                        Err(e) => {
                            *status_message = format!("Failed to load items: {}", e);
                        }
                    }
                }
            }

            ui.checkbox(file_load_merge_mode, "Merge");
            ui.label(if *file_load_merge_mode {
                "(adds to existing)"
            } else {
                "(replaces all)"
            });

            if ui.button("💾 Save to File").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("items.ron")
                    .add_filter("RON", &["ron"])
                    .save_file()
                {
                    match ron::ser::to_string_pretty(items, Default::default()) {
                        Ok(contents) => match std::fs::write(&path, contents) {
                            Ok(_) => {
                                *status_message = format!("Saved items to: {}", path.display());
                            }
                            Err(e) => {
                                *status_message = format!("Failed to save items: {}", e);
                            }
                        },
                        Err(e) => {
                            *status_message = format!("Failed to serialize items: {}", e);
                        }
                    }
                }
            }

            ui.separator();
            ui.label(format!("Total: {}", items.len()));
        });

        // Filter toolbar
        ui.horizontal(|ui| {
            ui.label("Filters:");

            egui::ComboBox::from_id_salt("item_type_filter")
                .selected_text(
                    self.filter_type
                        .map(|f| f.as_str().to_string())
                        .unwrap_or_else(|| "All Types".to_string()),
                )
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.filter_type.is_none(), "All Types")
                        .clicked()
                    {
                        self.filter_type = None;
                    }
                    for filter in ItemTypeFilter::all() {
                        if ui
                            .selectable_value(&mut self.filter_type, Some(filter), filter.as_str())
                            .clicked()
                        {}
                    }
                });

            ui.separator();

            if ui
                .selectable_label(self.filter_magical == Some(true), "✨ Magical")
                .clicked()
            {
                self.filter_magical = if self.filter_magical == Some(true) {
                    None
                } else {
                    Some(true)
                };
            }

            if ui
                .selectable_label(self.filter_cursed == Some(true), "💀 Cursed")
                .clicked()
            {
                self.filter_cursed = if self.filter_cursed == Some(true) {
                    None
                } else {
                    Some(true)
                };
            }

            if ui
                .selectable_label(self.filter_quest == Some(true), "📜 Quest")
                .clicked()
            {
                self.filter_quest = if self.filter_quest == Some(true) {
                    None
                } else {
                    Some(true)
                };
            }

            ui.separator();

            if ui.button("🔄 Clear Filters").clicked() {
                self.filter_type = None;
                self.filter_magical = None;
                self.filter_cursed = None;
                self.filter_quest = None;
            }
        });

        ui.separator();

        match self.mode {
            ItemsEditorMode::List => self.show_list(
                ui,
                items,
                unsaved_changes,
                status_message,
                campaign_dir,
                items_file,
            ),
            ItemsEditorMode::Add | ItemsEditorMode::Edit => self.show_form(
                ui,
                items,
                unsaved_changes,
                status_message,
                campaign_dir,
                items_file,
            ),
        }

        if self.show_import_dialog {
            self.show_import_dialog(
                ui.ctx(),
                items,
                unsaved_changes,
                status_message,
                campaign_dir,
                items_file,
            );
        }
    }

    fn show_list(
        &mut self,
        ui: &mut egui::Ui,
        items: &mut Vec<Item>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        items_file: &str,
    ) {
        let search_lower = self.search_query.to_lowercase();
        let mut filtered_items: Vec<(usize, String)> = items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if !search_lower.is_empty() && !item.name.to_lowercase().contains(&search_lower) {
                    return false;
                }
                if let Some(type_filter) = self.filter_type {
                    if !type_filter.matches(item) {
                        return false;
                    }
                }
                if let Some(magical) = self.filter_magical {
                    if item.is_magical() != magical {
                        return false;
                    }
                }
                if let Some(cursed) = self.filter_cursed {
                    if item.is_cursed != cursed {
                        return false;
                    }
                }
                if let Some(quest) = self.filter_quest {
                    if item.is_quest_item() != quest {
                        return false;
                    }
                }
                true
            })
            .map(|(idx, item)| {
                let mut label = format!("{}: {}", item.id, item.name);
                if item.is_magical() {
                    label.push_str(" ✨");
                }
                if item.is_cursed {
                    label.push_str(" 💀");
                }
                if item.is_quest_item() {
                    label.push_str(" 📜");
                }
                (idx, label)
            })
            .collect();

        filtered_items.sort_by_key(|(idx, _)| items[*idx].id);

        let selected = self.selected_item;
        let mut new_selection = selected;
        let mut action: Option<(usize, &str)> = None;

        ui.horizontal(|ui| {
            let height = ui.available_height();

            ui.vertical(|ui| {
                ui.set_width(300.0);
                ui.set_height(height);

                ui.heading("Items");
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("items_list_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        for (idx, label) in &filtered_items {
                            let is_selected = selected == Some(*idx);
                            if ui.selectable_label(is_selected, label).clicked() {
                                new_selection = Some(*idx);
                            }
                        }

                        if filtered_items.is_empty() {
                            ui.label("No items found");
                        }
                    });
            });

            ui.separator();

            ui.vertical(|ui| {
                ui.set_height(height);
                ui.set_min_width(ui.available_width());
                if let Some(idx) = selected {
                    if idx < items.len() {
                        let item = items[idx].clone();

                        ui.heading(&item.name);
                        ui.separator();

                        ui.horizontal(|ui| {
                            if ui.button("✏️ Edit").clicked() {
                                action = Some((idx, "edit"));
                            }
                            if ui.button("🗑️ Delete").clicked() {
                                action = Some((idx, "delete"));
                            }
                            if ui.button("📋 Duplicate").clicked() {
                                action = Some((idx, "duplicate"));
                            }
                            if ui.button("📤 Export").clicked() {
                                action = Some((idx, "export"));
                            }
                        });

                        ui.separator();
                        self.show_preview(ui, &item);
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label("Select an item to view details");
                    });
                }
            });
        });

        self.selected_item = new_selection;

        if let Some((idx, cmd)) = action {
            match cmd {
                "edit" => {
                    self.mode = ItemsEditorMode::Edit;
                    self.edit_buffer = items[idx].clone();
                }
                "delete" => {
                    items.remove(idx);
                    self.selected_item = None;
                    self.save_items(
                        items,
                        campaign_dir,
                        items_file,
                        unsaved_changes,
                        status_message,
                    );
                }
                "duplicate" => {
                    let mut new_item = items[idx].clone();
                    let next_id = items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
                    new_item.id = next_id;
                    new_item.name = format!("{} (Copy)", new_item.name);
                    items.push(new_item);
                    self.save_items(
                        items,
                        campaign_dir,
                        items_file,
                        unsaved_changes,
                        status_message,
                    );
                }
                "export" => {
                    if let Ok(ron_str) =
                        ron::ser::to_string_pretty(&items[idx], ron::ser::PrettyConfig::default())
                    {
                        self.import_export_buffer = ron_str;
                        self.show_import_dialog = true;
                        *status_message = "Item exported to clipboard dialog".to_string();
                    } else {
                        *status_message = "Failed to export item".to_string();
                    }
                }
                _ => {}
            }
        }
    }

    fn show_preview(&self, ui: &mut egui::Ui, item: &Item) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.heading("Basic Info");
                    ui.label(format!("ID: {}", item.id));
                    ui.label(format!("Base Cost: {} gold", item.base_cost));
                    ui.label(format!("Sell Cost: {} gold", item.sell_cost));

                    let mut flags = Vec::new();
                    if item.is_magical() {
                        flags.push("✨ Magical");
                    }
                    if item.is_cursed {
                        flags.push("💀 Cursed");
                    }
                    if item.is_quest_item() {
                        flags.push("📜 Quest Item");
                    }
                    if !flags.is_empty() {
                        ui.label(flags.join(" "));
                    }
                });

                ui.add_space(5.0);

                ui.group(|ui| {
                    ui.heading("Item Type");
                    match &item.item_type {
                        ItemType::Weapon(data) => {
                            ui.label("⚔️ Weapon");
                            ui.label(format!("  Damage: {:?}", data.damage));
                            ui.label(format!("  Bonus: {}", data.bonus));
                            ui.label(format!("  Hands: {}", data.hands_required));
                        }
                        ItemType::Armor(data) => {
                            ui.label("🛡️ Armor");
                            ui.label(format!("  AC Bonus: +{}", data.ac_bonus));
                            ui.label(format!("  Weight: {} lbs", data.weight));
                        }
                        ItemType::Accessory(data) => {
                            ui.label("💍 Accessory");
                            ui.label(format!("  Slot: {:?}", data.slot));
                        }
                        ItemType::Consumable(data) => {
                            ui.label("🧪 Consumable");
                            ui.label(format!("  Effect: {:?}", data.effect));
                            ui.label(format!("  Combat Use: {}", data.is_combat_usable));
                        }
                        ItemType::Ammo(data) => {
                            ui.label("🏹 Ammunition");
                            ui.label(format!("  Type: {:?}", data.ammo_type));
                            ui.label(format!("  Quantity: {}", data.quantity));
                        }
                        ItemType::Quest(data) => {
                            ui.label("📜 Quest Item");
                            ui.label(format!("  Quest: {}", data.quest_id));
                            ui.label(format!("  Key Item: {}", data.is_key_item));
                        }
                    }
                });

                ui.add_space(5.0);

                ui.group(|ui| {
                    ui.heading("Class Restrictions");
                    self.show_disablement_display(ui, item.disablements);
                });

                if item.constant_bonus.is_some()
                    || item.temporary_bonus.is_some()
                    || item.spell_effect.is_some()
                {
                    ui.add_space(5.0);
                    ui.group(|ui| {
                        ui.heading("Magical Effects");

                        if let Some(bonus) = item.constant_bonus {
                            ui.label(format!("Constant: {:?} {:+}", bonus.attribute, bonus.value));
                        }

                        if let Some(bonus) = item.temporary_bonus {
                            ui.label(format!(
                                "Temporary: {:?} {:+}",
                                bonus.attribute, bonus.value
                            ));
                        }

                        if let Some(spell_id) = item.spell_effect {
                            ui.label(format!("Spell Effect: ID {}", spell_id));
                        }

                        if item.max_charges > 0 {
                            ui.label(format!("Max Charges: {}", item.max_charges));
                        }
                    });
                }
            });
    }

    fn show_disablement_display(&self, ui: &mut egui::Ui, disablement: Disablement) {
        ui.horizontal_wrapped(|ui| {
            let classes = [
                (Disablement::KNIGHT, "Knight"),
                (Disablement::PALADIN, "Paladin"),
                (Disablement::ARCHER, "Archer"),
                (Disablement::CLERIC, "Cleric"),
                (Disablement::SORCERER, "Sorcerer"),
                (Disablement::ROBBER, "Robber"),
            ];

            for (flag, name) in &classes {
                let can_use = disablement.can_use_class(*flag);
                if can_use {
                    ui.label(format!("✓ {}", name));
                } else {
                    ui.label(format!("✗ {}", name));
                }
            }
        });

        ui.horizontal(|ui| {
            if disablement.good_only() {
                ui.label("☀️ Good Only");
            }
            if disablement.evil_only() {
                ui.label("🌙 Evil Only");
            }
            if !disablement.good_only() && !disablement.evil_only() {
                ui.label("⚖️ Any Alignment");
            }
        });
    }

    fn show_import_dialog(
        &mut self,
        ctx: &egui::Context,
        items: &mut Vec<Item>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        items_file: &str,
    ) {
        let mut open = self.show_import_dialog;

        egui::Window::new("Import/Export Item")
            .open(&mut open)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.heading("Item RON Data");
                ui.separator();

                ui.label("Paste RON data to import, or copy exported data:");
                let text_edit = egui::TextEdit::multiline(&mut self.import_export_buffer)
                    .desired_rows(15)
                    .code_editor();
                ui.add(text_edit);

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("📥 Import").clicked() {
                        match ron::from_str::<Item>(&self.import_export_buffer) {
                            Ok(mut item) => {
                                let next_id = items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
                                item.id = next_id;
                                items.push(item);
                                self.save_items(
                                    items,
                                    campaign_dir,
                                    items_file,
                                    unsaved_changes,
                                    status_message,
                                );
                                *status_message = "Item imported successfully".to_string();
                                self.show_import_dialog = false;
                            }
                            Err(e) => {
                                *status_message = format!("Import failed: {}", e);
                            }
                        }
                    }

                    if ui.button("📋 Copy to Clipboard").clicked() {
                        ui.ctx().copy_text(self.import_export_buffer.clone());
                        *status_message = "Copied to clipboard".to_string();
                    }

                    if ui.button("❌ Close").clicked() {
                        self.show_import_dialog = false;
                    }
                });
            });

        self.show_import_dialog = open;
    }

    fn show_form(
        &mut self,
        ui: &mut egui::Ui,
        items: &mut Vec<Item>,
        unsaved_changes: &mut bool,
        status_message: &mut String,
        campaign_dir: Option<&PathBuf>,
        items_file: &str,
    ) {
        let is_add = self.mode == ItemsEditorMode::Add;
        ui.heading(if is_add { "Add New Item" } else { "Edit Item" });
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.heading("Basic Properties");

                    ui.horizontal(|ui| {
                        ui.label("ID:");
                        ui.add_enabled(
                            false,
                            egui::TextEdit::singleline(&mut self.edit_buffer.id.to_string()),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.edit_buffer.name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Base Cost:");
                        ui.add(egui::DragValue::new(&mut self.edit_buffer.base_cost).speed(1.0));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Sell Cost:");
                        ui.add(egui::DragValue::new(&mut self.edit_buffer.sell_cost).speed(1.0));
                    });

                    ui.checkbox(&mut self.edit_buffer.is_cursed, "Cursed");

                    ui.horizontal(|ui| {
                        ui.label("Max Charges:");
                        ui.add(egui::DragValue::new(&mut self.edit_buffer.max_charges).speed(1.0));
                    });
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Item Type");

                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(self.edit_buffer.is_weapon(), "⚔️ Weapon")
                            .clicked()
                        {
                            self.edit_buffer.item_type = ItemType::Weapon(WeaponData {
                                damage: DiceRoll::new(1, 6, 0),
                                bonus: 0,
                                hands_required: 1,
                            });
                        }
                        if ui
                            .selectable_label(self.edit_buffer.is_armor(), "🛡️ Armor")
                            .clicked()
                        {
                            self.edit_buffer.item_type = ItemType::Armor(ArmorData {
                                ac_bonus: 0,
                                weight: 0,
                            });
                        }
                        if ui
                            .selectable_label(self.edit_buffer.is_accessory(), "💍 Accessory")
                            .clicked()
                        {
                            self.edit_buffer.item_type = ItemType::Accessory(AccessoryData {
                                slot: AccessorySlot::Ring,
                            });
                        }
                        if ui
                            .selectable_label(self.edit_buffer.is_consumable(), "🧪 Consumable")
                            .clicked()
                        {
                            self.edit_buffer.item_type = ItemType::Consumable(ConsumableData {
                                effect: ConsumableEffect::HealHp(10),
                                is_combat_usable: true,
                            });
                        }
                        if ui
                            .selectable_label(self.edit_buffer.is_ammo(), "🏹 Ammo")
                            .clicked()
                        {
                            self.edit_buffer.item_type = ItemType::Ammo(AmmoData {
                                ammo_type: AmmoType::Arrow,
                                quantity: 20,
                            });
                        }
                        if ui
                            .selectable_label(self.edit_buffer.is_quest_item(), "📜 Quest")
                            .clicked()
                        {
                            self.edit_buffer.item_type = ItemType::Quest(QuestData {
                                quest_id: String::new(),
                                is_key_item: false,
                            });
                        }
                    });

                    ui.separator();
                    self.show_type_editor(ui);
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.heading("Class Restrictions");
                    self.show_disablement_editor(ui);
                });

                ui.add_space(10.0);
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("💾 Save").clicked() {
                        if is_add {
                            items.push(self.edit_buffer.clone());
                        } else if let Some(idx) = self.selected_item {
                            if idx < items.len() {
                                items[idx] = self.edit_buffer.clone();
                            }
                        }
                        self.save_items(
                            items,
                            campaign_dir,
                            items_file,
                            unsaved_changes,
                            status_message,
                        );
                        self.mode = ItemsEditorMode::List;
                        *status_message = "Item saved".to_string();
                    }

                    if ui.button("❌ Cancel").clicked() {
                        self.mode = ItemsEditorMode::List;
                    }
                });
            });
    }

    fn show_type_editor(&mut self, ui: &mut egui::Ui) {
        match &mut self.edit_buffer.item_type {
            ItemType::Weapon(data) => {
                ui.label("⚔️ Weapon Properties");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Damage Dice:");
                    ui.add(
                        egui::DragValue::new(&mut data.damage.count)
                            .range(1..=10)
                            .prefix("Count: "),
                    );
                    ui.label("d");
                    ui.add(
                        egui::DragValue::new(&mut data.damage.sides)
                            .range(1..=100)
                            .prefix("Sides: "),
                    );
                    ui.label("+");
                    ui.add(
                        egui::DragValue::new(&mut data.damage.bonus)
                            .range(-100..=100)
                            .prefix("Bonus: "),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("To-Hit/Damage Bonus:");
                    ui.add(egui::DragValue::new(&mut data.bonus).range(-10..=10));
                });

                ui.horizontal(|ui| {
                    ui.label("Hands Required:");
                    ui.add(egui::DragValue::new(&mut data.hands_required).range(1..=2));
                });
            }
            ItemType::Armor(data) => {
                ui.label("🛡️ Armor Properties");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("AC Bonus:");
                    ui.add(egui::DragValue::new(&mut data.ac_bonus).range(0..=20));
                });

                ui.horizontal(|ui| {
                    ui.label("Weight (lbs):");
                    ui.add(egui::DragValue::new(&mut data.weight).range(0..=255));
                });
            }
            ItemType::Accessory(data) => {
                ui.label("💍 Accessory Properties");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Equipment Slot:");
                    egui::ComboBox::from_id_salt("accessory_slot")
                        .selected_text(format!("{:?}", data.slot))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut data.slot, AccessorySlot::Ring, "Ring");
                            ui.selectable_value(&mut data.slot, AccessorySlot::Amulet, "Amulet");
                            ui.selectable_value(&mut data.slot, AccessorySlot::Belt, "Belt");
                            ui.selectable_value(&mut data.slot, AccessorySlot::Cloak, "Cloak");
                        });
                });
            }
            ItemType::Consumable(data) => {
                ui.label("🧪 Consumable Properties");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Effect Type:");

                    let effect_str = match &data.effect {
                        ConsumableEffect::HealHp(_) => "Heal HP",
                        ConsumableEffect::RestoreSp(_) => "Restore SP",
                        ConsumableEffect::CureCondition(_) => "Cure Condition",
                        ConsumableEffect::BoostAttribute(_, _) => "Boost Attribute",
                    };

                    egui::ComboBox::from_id_salt("consumable_effect")
                        .selected_text(effect_str)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(
                                    matches!(data.effect, ConsumableEffect::HealHp(_)),
                                    "Heal HP",
                                )
                                .clicked()
                            {
                                data.effect = ConsumableEffect::HealHp(10);
                            }
                            if ui
                                .selectable_label(
                                    matches!(data.effect, ConsumableEffect::RestoreSp(_)),
                                    "Restore SP",
                                )
                                .clicked()
                            {
                                data.effect = ConsumableEffect::RestoreSp(10);
                            }
                            if ui
                                .selectable_label(
                                    matches!(data.effect, ConsumableEffect::CureCondition(_)),
                                    "Cure Condition",
                                )
                                .clicked()
                            {
                                data.effect = ConsumableEffect::CureCondition(0);
                            }
                            if ui
                                .selectable_label(
                                    matches!(data.effect, ConsumableEffect::BoostAttribute(_, _)),
                                    "Boost Attribute",
                                )
                                .clicked()
                            {
                                data.effect =
                                    ConsumableEffect::BoostAttribute(AttributeType::Might, 1);
                            }
                        });
                });

                match &mut data.effect {
                    ConsumableEffect::HealHp(amount) | ConsumableEffect::RestoreSp(amount) => {
                        ui.horizontal(|ui| {
                            ui.label("Amount:");
                            ui.add(egui::DragValue::new(amount).range(1..=1000));
                        });
                    }
                    ConsumableEffect::CureCondition(flags) => {
                        ui.horizontal(|ui| {
                            ui.label("Condition Flags:");
                            ui.add(egui::DragValue::new(flags).range(0..=255));
                        });
                    }
                    ConsumableEffect::BoostAttribute(attr_type, value) => {
                        ui.horizontal(|ui| {
                            ui.label("Attribute:");
                            egui::ComboBox::from_id_salt("boost_attribute")
                                .selected_text(format!("{:?}", attr_type))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(attr_type, AttributeType::Might, "Might");
                                    ui.selectable_value(
                                        attr_type,
                                        AttributeType::Intellect,
                                        "Intellect",
                                    );
                                    ui.selectable_value(
                                        attr_type,
                                        AttributeType::Personality,
                                        "Personality",
                                    );
                                    ui.selectable_value(
                                        attr_type,
                                        AttributeType::Endurance,
                                        "Endurance",
                                    );
                                    ui.selectable_value(attr_type, AttributeType::Speed, "Speed");
                                    ui.selectable_value(
                                        attr_type,
                                        AttributeType::Accuracy,
                                        "Accuracy",
                                    );
                                    ui.selectable_value(attr_type, AttributeType::Luck, "Luck");
                                });
                        });
                        ui.horizontal(|ui| {
                            ui.label("Boost Amount:");
                            ui.add(egui::DragValue::new(value).range(-10..=10));
                        });
                    }
                }

                ui.checkbox(&mut data.is_combat_usable, "Usable in Combat");
            }
            ItemType::Ammo(data) => {
                ui.label("🏹 Ammunition Properties");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Ammo Type:");
                    egui::ComboBox::from_id_salt("ammo_type")
                        .selected_text(format!("{:?}", data.ammo_type))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut data.ammo_type, AmmoType::Arrow, "Arrow");
                            ui.selectable_value(&mut data.ammo_type, AmmoType::Bolt, "Bolt");
                            ui.selectable_value(&mut data.ammo_type, AmmoType::Stone, "Stone");
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Quantity:");
                    ui.add(egui::DragValue::new(&mut data.quantity).range(1..=1000));
                });
            }
            ItemType::Quest(data) => {
                ui.label("📜 Quest Item Properties");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Quest ID:");
                    ui.text_edit_singleline(&mut data.quest_id);
                });

                ui.checkbox(&mut data.is_key_item, "Key Item (Cannot drop/sell)");
            }
        }
    }

    fn show_disablement_editor(&mut self, ui: &mut egui::Ui) {
        let mut flags = self.edit_buffer.disablements.0;

        ui.label("Usable by:");
        ui.horizontal_wrapped(|ui| {
            let classes = [
                (Disablement::KNIGHT, "Knight"),
                (Disablement::PALADIN, "Paladin"),
                (Disablement::ARCHER, "Archer"),
                (Disablement::CLERIC, "Cleric"),
                (Disablement::SORCERER, "Sorcerer"),
                (Disablement::ROBBER, "Robber"),
            ];

            for (flag, name) in &classes {
                let mut enabled = (flags & flag) != 0;
                if ui.checkbox(&mut enabled, *name).changed() {
                    if enabled {
                        flags |= flag;
                    } else {
                        flags &= !flag;
                    }
                }
            }
        });

        ui.separator();
        ui.label("Alignment:");
        ui.horizontal(|ui| {
            let mut good = (flags & Disablement::GOOD) != 0;
            let mut evil = (flags & Disablement::EVIL) != 0;

            if ui.checkbox(&mut good, "☀️ Good Only").changed() {
                if good {
                    flags |= Disablement::GOOD;
                    flags &= !Disablement::EVIL;
                } else {
                    flags &= !Disablement::GOOD;
                }
            }

            if ui.checkbox(&mut evil, "🌙 Evil Only").changed() {
                if evil {
                    flags |= Disablement::EVIL;
                    flags &= !Disablement::GOOD;
                } else {
                    flags &= !Disablement::EVIL;
                }
            }
        });

        self.edit_buffer.disablements.0 = flags;

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("✓ All Classes").clicked() {
                self.edit_buffer.disablements.0 = 0b0011_1111;
            }
            if ui.button("✗ None").clicked() {
                self.edit_buffer.disablements.0 = 0;
            }
        });
    }

    fn save_items(
        &self,
        items: &Vec<Item>,
        campaign_dir: Option<&PathBuf>,
        items_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        if let Some(dir) = campaign_dir {
            let items_path = dir.join(items_file);

            if let Some(parent) = items_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            let ron_config = ron::ser::PrettyConfig::new()
                .struct_names(false)
                .enumerate_arrays(false);

            match ron::ser::to_string_pretty(items, ron_config) {
                Ok(contents) => match std::fs::write(&items_path, contents) {
                    Ok(_) => {
                        *unsaved_changes = true;
                        // *status_message = format!("Saved items to: {}", items_path.display());
                        // Note: We don't necessarily want to set status message here as it might be an autosave
                    }
                    Err(e) => {
                        *status_message = format!("Failed to write items file: {}", e);
                    }
                },
                Err(e) => {
                    *status_message = format!("Failed to serialize items: {}", e);
                }
            }
        }
    }
}
