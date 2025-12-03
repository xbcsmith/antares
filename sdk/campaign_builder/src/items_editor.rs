// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use crate::ui_helpers::{ActionButtons, EditorToolbar, ItemAction, ToolbarAction, TwoColumnLayout};
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

        // Use shared EditorToolbar component
        let toolbar_action = EditorToolbar::new("Items")
            .with_search(&mut self.search_query)
            .with_merge_mode(file_load_merge_mode)
            .with_total_count(items.len())
            .with_id_salt("items_toolbar")
            .show(ui);

        // Handle toolbar actions
        match toolbar_action {
            ToolbarAction::New => {
                self.mode = ItemsEditorMode::Add;
                self.edit_buffer = Self::default_item();
                let next_id = items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
                self.edit_buffer.id = next_id;
                *unsaved_changes = true;
            }
            ToolbarAction::Save => {
                self.save_items(
                    items,
                    campaign_dir,
                    items_file,
                    unsaved_changes,
                    status_message,
                );
            }
            ToolbarAction::Load => {
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
            ToolbarAction::Import => {
                self.show_import_dialog = true;
                self.import_export_buffer.clear();
            }
            ToolbarAction::Export => {
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
            ToolbarAction::Reload => {
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
                    } else {
                        *status_message = "Items file does not exist".to_string();
                    }
                }
            }
            ToolbarAction::None => {}
        }

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

        // Build filtered list snapshot to avoid borrow conflicts in closures
        let filtered_items: Vec<(usize, String, Item)> = items
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
                (idx, label, item.clone())
            })
            .collect();

        // Sort by ID
        let mut sorted_items = filtered_items;
        sorted_items.sort_by_key(|(idx, _, _)| items[*idx].id);

        let selected = self.selected_item;
        let mut new_selection = selected;
        let mut action_requested: Option<ItemAction> = None;

        // Use shared TwoColumnLayout component
        TwoColumnLayout::new("items").show_split(
            ui,
            |left_ui| {
                // Left panel: Items list
                left_ui.heading("Items");
                left_ui.separator();

                for (idx, label, _) in &sorted_items {
                    let is_selected = selected == Some(*idx);
                    if left_ui.selectable_label(is_selected, label).clicked() {
                        new_selection = Some(*idx);
                    }
                }

                if sorted_items.is_empty() {
                    left_ui.label("No items found");
                }
            },
            |right_ui| {
                // Right panel: Detail view
                if let Some(idx) = selected {
                    if let Some((_, _, item)) = sorted_items.iter().find(|(i, _, _)| *i == idx) {
                        right_ui.heading(&item.name);
                        right_ui.separator();

                        // Use shared ActionButtons component
                        let action = ActionButtons::new().enabled(true).show(right_ui);
                        if action != ItemAction::None {
                            action_requested = Some(action);
                        }

                        right_ui.separator();
                        Self::show_preview_static(right_ui, item);
                    } else {
                        right_ui.vertical_centered(|ui| {
                            ui.add_space(100.0);
                            ui.label("Select an item to view details");
                        });
                    }
                } else {
                    right_ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label("Select an item to view details");
                    });
                }
            },
        );

        // Apply selection change after closures
        self.selected_item = new_selection;

        // Handle action button clicks after closures
        if let Some(action) = action_requested {
            match action {
                ItemAction::Edit => {
                    if let Some(idx) = self.selected_item {
                        if idx < items.len() {
                            self.mode = ItemsEditorMode::Edit;
                            self.edit_buffer = items[idx].clone();
                        }
                    }
                }
                ItemAction::Delete => {
                    if let Some(idx) = self.selected_item {
                        if idx < items.len() {
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
                    }
                }
                ItemAction::Duplicate => {
                    if let Some(idx) = self.selected_item {
                        if idx < items.len() {
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
                    }
                }
                ItemAction::Export => {
                    if let Some(idx) = self.selected_item {
                        if idx < items.len() {
                            if let Ok(ron_str) = ron::ser::to_string_pretty(
                                &items[idx],
                                ron::ser::PrettyConfig::default(),
                            ) {
                                self.import_export_buffer = ron_str;
                                self.show_import_dialog = true;
                                *status_message = "Item exported to clipboard dialog".to_string();
                            } else {
                                *status_message = "Failed to export item".to_string();
                            }
                        }
                    }
                }
                ItemAction::None => {}
            }
        }
    }

    /// Static preview method that doesn't require self
    fn show_preview_static(ui: &mut egui::Ui, item: &Item) {
        let panel_height = crate::ui_helpers::compute_panel_height(
            ui,
            crate::ui_helpers::DEFAULT_PANEL_MIN_HEIGHT,
        );

        egui::ScrollArea::vertical()
            .max_height(panel_height)
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
                    Self::show_disablement_display_static(ui, item.disablements);
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

    /// Static disablement display that doesn't require self
    fn show_disablement_display_static(ui: &mut egui::Ui, disablement: Disablement) {
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
                ui.label("Weapon Properties:");
                ui.horizontal(|ui| {
                    ui.label("Damage Dice:");
                    ui.add(egui::DragValue::new(&mut data.damage.count).range(1..=10));
                    ui.label("d");
                    ui.add(egui::DragValue::new(&mut data.damage.sides).range(1..=20));
                    ui.label("+");
                    ui.add(egui::DragValue::new(&mut data.damage.bonus).range(-10..=20));
                });
                ui.horizontal(|ui| {
                    ui.label("Attack Bonus:");
                    ui.add(egui::DragValue::new(&mut data.bonus).range(-5..=10));
                });
                ui.horizontal(|ui| {
                    ui.label("Hands Required:");
                    ui.add(egui::DragValue::new(&mut data.hands_required).range(1..=2));
                });
            }
            ItemType::Armor(data) => {
                ui.label("Armor Properties:");
                ui.horizontal(|ui| {
                    ui.label("AC Bonus:");
                    ui.add(egui::DragValue::new(&mut data.ac_bonus).range(0..=20));
                });
                ui.horizontal(|ui| {
                    ui.label("Weight:");
                    ui.add(egui::DragValue::new(&mut data.weight).range(0..=100));
                });
            }
            ItemType::Accessory(data) => {
                ui.label("Accessory Properties:");
                ui.horizontal(|ui| {
                    ui.label("Slot:");
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
                ui.label("Consumable Properties:");
                ui.checkbox(&mut data.is_combat_usable, "Usable in Combat");

                ui.horizontal(|ui| {
                    ui.label("Effect:");
                    let effect_type = match &data.effect {
                        ConsumableEffect::HealHp(_) => "Heal HP",
                        ConsumableEffect::RestoreSp(_) => "Restore SP",
                        ConsumableEffect::CureCondition(_) => "Cure Condition",
                        ConsumableEffect::BoostAttribute(_, _) => "Boost Attribute",
                    };
                    egui::ComboBox::from_id_salt("consumable_effect")
                        .selected_text(effect_type)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(effect_type == "Heal HP", "Heal HP")
                                .clicked()
                            {
                                data.effect = ConsumableEffect::HealHp(10);
                            }
                            if ui
                                .selectable_label(effect_type == "Restore SP", "Restore SP")
                                .clicked()
                            {
                                data.effect = ConsumableEffect::RestoreSp(10);
                            }
                            if ui
                                .selectable_label(effect_type == "Cure Condition", "Cure Condition")
                                .clicked()
                            {
                                data.effect = ConsumableEffect::CureCondition(0xFF);
                            }
                            if ui
                                .selectable_label(
                                    effect_type == "Boost Attribute",
                                    "Boost Attribute",
                                )
                                .clicked()
                            {
                                data.effect =
                                    ConsumableEffect::BoostAttribute(AttributeType::Might, 1);
                            }
                        });
                });

                // Edit effect value
                match &mut data.effect {
                    ConsumableEffect::HealHp(amount) => {
                        ui.horizontal(|ui| {
                            ui.label("Amount:");
                            ui.add(egui::DragValue::new(amount).range(1..=999));
                        });
                    }
                    ConsumableEffect::RestoreSp(amount) => {
                        ui.horizontal(|ui| {
                            ui.label("Amount:");
                            ui.add(egui::DragValue::new(amount).range(1..=999));
                        });
                    }
                    ConsumableEffect::CureCondition(flags) => {
                        ui.horizontal(|ui| {
                            ui.label("Condition Flags:");
                            ui.add(egui::DragValue::new(flags).range(0..=255));
                        });
                    }
                    ConsumableEffect::BoostAttribute(attr_type, amount) => {
                        ui.horizontal(|ui| {
                            ui.label("Attribute:");
                            egui::ComboBox::from_id_salt("boost_attr_type")
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
                            ui.label("Amount:");
                            ui.add(egui::DragValue::new(amount).range(-128..=127));
                        });
                    }
                }
            }
            ItemType::Ammo(data) => {
                ui.label("Ammunition Properties:");
                ui.horizontal(|ui| {
                    ui.label("Type:");
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
                    ui.add(egui::DragValue::new(&mut data.quantity).range(1..=99));
                });
            }
            ItemType::Quest(data) => {
                ui.label("Quest Item Properties:");
                ui.horizontal(|ui| {
                    ui.label("Quest ID:");
                    ui.text_edit_singleline(&mut data.quest_id);
                });
                ui.checkbox(&mut data.is_key_item, "Key Item");
            }
        }
    }

    fn show_disablement_editor(&mut self, ui: &mut egui::Ui) {
        let disablement = &mut self.edit_buffer.disablements;

        ui.label("Classes that CAN use this item:");
        ui.horizontal(|ui| {
            let classes = [
                (Disablement::KNIGHT, "Knight"),
                (Disablement::PALADIN, "Paladin"),
                (Disablement::ARCHER, "Archer"),
                (Disablement::CLERIC, "Cleric"),
                (Disablement::SORCERER, "Sorcerer"),
                (Disablement::ROBBER, "Robber"),
            ];

            for (flag, name) in &classes {
                let mut can_use = disablement.can_use_class(*flag);
                if ui.checkbox(&mut can_use, *name).changed() {
                    if can_use {
                        disablement.0 |= *flag;
                    } else {
                        disablement.0 &= !*flag;
                    }
                }
            }
        });

        ui.separator();
        ui.label("Alignment Restriction:");
        ui.horizontal(|ui| {
            let mut good_only = disablement.good_only();
            let mut evil_only = disablement.evil_only();

            if ui
                .radio_value(&mut good_only, false, "Any Alignment")
                .changed()
            {
                disablement.0 &= !(Disablement::GOOD | Disablement::EVIL);
            }
            if ui.radio_value(&mut good_only, true, "Good Only").changed() {
                disablement.0 |= Disablement::GOOD;
                disablement.0 &= !Disablement::EVIL;
            }
            if ui.radio_value(&mut evil_only, true, "Evil Only").changed() {
                disablement.0 &= !Disablement::GOOD;
                disablement.0 |= Disablement::EVIL;
            }
        });
    }

    fn save_items(
        &self,
        items: &[Item],
        campaign_dir: Option<&PathBuf>,
        items_file: &str,
        unsaved_changes: &mut bool,
        status_message: &mut String,
    ) {
        if let Some(dir) = campaign_dir {
            let items_path = dir.join(items_file);

            // Create parent directories if necessary
            if let Some(parent) = items_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    *status_message = format!("Failed to create directory: {}", e);
                    return;
                }
            }

            match ron::ser::to_string_pretty(items, Default::default()) {
                Ok(contents) => match std::fs::write(&items_path, contents) {
                    Ok(_) => {
                        *unsaved_changes = true;
                        *status_message = format!("Auto-saved items to: {}", items_path.display());
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
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ItemsEditorState Tests
    // =========================================================================

    #[test]
    fn test_items_editor_state_new() {
        let state = ItemsEditorState::new();
        assert_eq!(state.mode, ItemsEditorMode::List);
        assert!(state.search_query.is_empty());
        assert!(state.selected_item.is_none());
        assert!(!state.show_import_dialog);
        assert!(state.import_export_buffer.is_empty());
    }

    #[test]
    fn test_items_editor_state_default() {
        let state = ItemsEditorState::default();
        assert_eq!(state.mode, ItemsEditorMode::List);
        assert!(state.filter_type.is_none());
        assert!(state.filter_magical.is_none());
        assert!(state.filter_cursed.is_none());
        assert!(state.filter_quest.is_none());
    }

    #[test]
    fn test_default_item_creation() {
        let item = ItemsEditorState::default_item();
        assert_eq!(item.id, 0);
        assert_eq!(item.name, "New Item");
        assert_eq!(item.base_cost, 10);
        assert_eq!(item.sell_cost, 5);
        assert!(!item.is_cursed);
        assert_eq!(item.max_charges, 0);
        assert!(matches!(item.item_type, ItemType::Weapon(_)));
    }

    // =========================================================================
    // ItemsEditorMode Tests
    // =========================================================================

    #[test]
    fn test_items_editor_mode_variants() {
        assert_eq!(ItemsEditorMode::List, ItemsEditorMode::List);
        assert_eq!(ItemsEditorMode::Add, ItemsEditorMode::Add);
        assert_eq!(ItemsEditorMode::Edit, ItemsEditorMode::Edit);
        assert_ne!(ItemsEditorMode::List, ItemsEditorMode::Add);
    }

    // =========================================================================
    // ItemTypeFilter Tests
    // =========================================================================

    #[test]
    fn test_item_type_filter_as_str() {
        assert_eq!(ItemTypeFilter::Weapon.as_str(), "Weapon");
        assert_eq!(ItemTypeFilter::Armor.as_str(), "Armor");
        assert_eq!(ItemTypeFilter::Accessory.as_str(), "Accessory");
        assert_eq!(ItemTypeFilter::Consumable.as_str(), "Consumable");
        assert_eq!(ItemTypeFilter::Ammo.as_str(), "Ammo");
        assert_eq!(ItemTypeFilter::Quest.as_str(), "Quest");
    }

    #[test]
    fn test_item_type_filter_all() {
        let filters = ItemTypeFilter::all();
        assert_eq!(filters.len(), 6);
        assert!(filters.contains(&ItemTypeFilter::Weapon));
        assert!(filters.contains(&ItemTypeFilter::Armor));
        assert!(filters.contains(&ItemTypeFilter::Accessory));
        assert!(filters.contains(&ItemTypeFilter::Consumable));
        assert!(filters.contains(&ItemTypeFilter::Ammo));
        assert!(filters.contains(&ItemTypeFilter::Quest));
    }

    #[test]
    fn test_item_type_filter_matches_weapon() {
        let weapon_item = Item {
            id: 1,
            name: "Sword".to_string(),
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
        };

        assert!(ItemTypeFilter::Weapon.matches(&weapon_item));
        assert!(!ItemTypeFilter::Armor.matches(&weapon_item));
        assert!(!ItemTypeFilter::Quest.matches(&weapon_item));
    }

    #[test]
    fn test_item_type_filter_matches_armor() {
        let armor_item = Item {
            id: 2,
            name: "Plate".to_string(),
            item_type: ItemType::Armor(ArmorData {
                ac_bonus: 5,
                weight: 50,
            }),
            base_cost: 100,
            sell_cost: 50,
            is_cursed: false,
            disablements: Disablement(0),
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            icon_path: None,
        };

        assert!(ItemTypeFilter::Armor.matches(&armor_item));
        assert!(!ItemTypeFilter::Weapon.matches(&armor_item));
    }

    #[test]
    fn test_item_type_filter_matches_quest() {
        let quest_item = Item {
            id: 3,
            name: "Magic Key".to_string(),
            item_type: ItemType::Quest(QuestData {
                quest_id: "quest_1".to_string(),
                is_key_item: true,
            }),
            base_cost: 0,
            sell_cost: 0,
            is_cursed: false,
            disablements: Disablement(0),
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            icon_path: None,
        };

        assert!(ItemTypeFilter::Quest.matches(&quest_item));
        assert!(!ItemTypeFilter::Weapon.matches(&quest_item));
    }

    // =========================================================================
    // Editor State Transitions Tests
    // =========================================================================

    #[test]
    fn test_editor_mode_transitions() {
        let mut state = ItemsEditorState::new();
        assert_eq!(state.mode, ItemsEditorMode::List);

        state.mode = ItemsEditorMode::Add;
        assert_eq!(state.mode, ItemsEditorMode::Add);

        state.mode = ItemsEditorMode::Edit;
        assert_eq!(state.mode, ItemsEditorMode::Edit);

        state.mode = ItemsEditorMode::List;
        assert_eq!(state.mode, ItemsEditorMode::List);
    }

    #[test]
    fn test_selected_item_handling() {
        let mut state = ItemsEditorState::new();
        assert!(state.selected_item.is_none());

        state.selected_item = Some(0);
        assert_eq!(state.selected_item, Some(0));

        state.selected_item = Some(5);
        assert_eq!(state.selected_item, Some(5));

        state.selected_item = None;
        assert!(state.selected_item.is_none());
    }

    #[test]
    fn test_filter_combinations() {
        let mut state = ItemsEditorState::new();

        // Set multiple filters
        state.filter_type = Some(ItemTypeFilter::Weapon);
        state.filter_magical = Some(true);
        state.filter_cursed = Some(false);

        assert_eq!(state.filter_type, Some(ItemTypeFilter::Weapon));
        assert_eq!(state.filter_magical, Some(true));
        assert_eq!(state.filter_cursed, Some(false));
        assert!(state.filter_quest.is_none());
    }
}
