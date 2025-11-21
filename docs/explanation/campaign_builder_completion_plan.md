# Campaign Builder GUI Completion Implementation Plan

## Overview

This plan addresses the incomplete Campaign Builder GUI identified in Phase 2. While the foundation is solid, several critical issues prevent effective campaign creation:

1. **Items editor has ID type mismatches** - Using `u32` instead of `ItemId` type alias
2. **Spells/Monsters editors are incomplete** - Basic CRUD works but missing validation
3. **Quests editor is a placeholder** - No functionality implemented
4. **Dialogues editor is disconnected** - Backend exists but not integrated into UI
5. **Assets manager reports false positives** - All assets marked as "unused" despite being referenced

The implementation will proceed in focused phases to systematically complete each editor.

## Current State Analysis

### Existing Infrastructure

**Working Components:**

- ✅ Campaign metadata editor (fully functional)
- ✅ Validation system with error/warning severity
- ✅ File I/O with RON serialization
- ✅ Unsaved changes tracking
- ✅ File tree browser
- ✅ Tab navigation system

**Partially Working:**

- ⚠️ Items editor - CRUD operations work but ID generation has type issues
- ⚠️ Spells editor - Basic functionality but missing advanced features
- ⚠️ Monsters editor - Similar to spells, needs enhancement
- ⚠️ Maps editor - Placeholder with basic list view

**Not Working:**

- ❌ Quests editor - Placeholder UI only, backend exists but not connected
- ❌ Dialogues editor - Backend implemented but UI shows basic list, no tree view
- ❌ Assets manager - Scans files but reference tracking non-functional

### Identified Issues

#### Issue 1: Type System Violations (CRITICAL)

**Location:** `sdk/campaign_builder/src/main.rs` throughout Items/Spells/Monsters editors

**Problem:** Using raw `u32` for IDs instead of type aliases defined in architecture:

```rust
// WRONG (current code):
pub id: u32

// RIGHT (architecture.md Section 4.6):
pub id: ItemId  // where ItemId = u32
pub id: SpellId // where SpellId = u32
pub id: MonsterId
```

**Impact:**

- Violates Golden Rule 3 (Type System Adherence)
- Breaks semantic type safety
- Makes cross-references harder to validate
- ID clash errors occur because integer arithmetic doesn't respect domain boundaries

#### Issue 2: Missing ID Uniqueness Validation

**Location:** ID generation in all editors (lines 1960, 2087, 2175, 2312, 2416, 2562)

**Problem:** ID generation uses `max().unwrap_or(0) + 1` pattern without checking for:

- Existing IDs when loading from file
- ID collisions during duplicate operations
- ID gaps after deletions

**Impact:**

- "ID clash" errors reported by user
- Data corruption risk when items with same ID exist
- Unpredictable behavior in game engine

#### Issue 3: Quest Editor Disconnected

**Location:** `sdk/campaign_builder/src/main.rs` line 2943-2975

**Problem:** Quest editor shows placeholder UI but `QuestEditorState` is fully implemented in `quest_editor.rs`:

- 681 lines of working backend code
- Quest CRUD operations ready
- Stage/objective management complete
- Validation logic present

**Impact:**

- Users cannot create or edit quests
- Quest system appears non-functional
- Phase 5 work is partially complete but invisible

#### Issue 4: Dialogue Editor Not Integrated

**Location:** `sdk/campaign_builder/src/main.rs` line 3092-3126

**Problem:** `DialogueEditorState` is implemented (738 lines) but UI only shows:

```rust
ui.label("Dialogue editor integration in progress");
// TODO: Integrate DialogueEditorWidget when UI components are ready
```

**Impact:**

- Cannot create NPC conversations
- Dialogue trees cannot be edited
- Major feature gap for campaign creators

#### Issue 5: Asset Reference Tracking Non-Functional

**Location:** `sdk/campaign_builder/src/asset_manager.rs` line 390-392

**Problem:** `AssetManager::unreferenced_assets()` returns all assets because `mark_referenced()` is never called:

```rust
pub fn unreferenced_assets(&self) -> Vec<&PathBuf> {
    self.assets.values()
        .filter(|asset| !asset.is_referenced)  // Always false
        .map(|asset| &asset.path)
        .collect()
}
```

**Impact:**

- False warnings that all assets are unused
- Cannot identify truly orphaned files
- Misleading data for campaign cleanup

#### Issue 6: Maps Editor Incomplete

**Location:** `sdk/campaign_builder/src/main.rs` line 2673-2888

**Problem:** Map editor has list view but `MapEditorState` in `map_editor.rs` is not fully utilized:

- Map preview exists but basic
- No tile editing UI
- No event placement
- No exit/entrance connections

**Impact:**

- Cannot create maps in GUI
- Must use separate `map_builder` CLI tool
- Workflow fragmentation

## Implementation Phases

### Phase 3A: Type System and ID Management Fixes (Week 1)

**Priority:** CRITICAL - Blocks all data editing functionality

#### 3A.1 Import Type Aliases

**Files:** `sdk/campaign_builder/src/main.rs`

Add type alias imports:

```rust
use antares::domain::items::ItemId;
use antares::domain::magic::SpellId;
use antares::domain::combat::MonsterId;
use antares::domain::world::MapId;
use antares::domain::quest::QuestId;
```

#### 3A.2 Fix Item Structure Type Signatures

**Files:** `sdk/campaign_builder/src/main.rs` CampaignBuilderApp

Change all Item-related fields:

- `items: Vec<Item>` - Already correct, Item uses ItemId internally
- `items_edit_buffer: Item` - Already correct
- Update ID generation to use type-safe patterns

#### 3A.3 Implement ID Uniqueness Validator

**Files:** `sdk/campaign_builder/src/main.rs`

Create new validation function:

```rust
impl CampaignBuilderApp {
    /// Check for duplicate IDs across items
    fn validate_item_ids(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for item in &self.items {
            if !seen_ids.insert(item.id) {
                errors.push(ValidationError {
                    severity: Severity::Error,
                    message: format!("Duplicate item ID: {}", item.id),
                });
            }
        }
        errors
    }

    /// Similar for spells, monsters
    fn validate_spell_ids(&self) -> Vec<ValidationError> { /* ... */ }
    fn validate_monster_ids(&self) -> Vec<ValidationError> { /* ... */ }
}
```

#### 3A.4 Enhance ID Generation Logic

**Files:** `sdk/campaign_builder/src/main.rs` multiple locations

Replace all ID generation with safe pattern:

```rust
// BEFORE:
self.items_edit_buffer.id = self.items.iter().map(|i| i.id).max().unwrap_or(0) + 1;

// AFTER:
self.items_edit_buffer.id = self.next_available_item_id();

// New helper method:
fn next_available_item_id(&self) -> ItemId {
    let max_id = self.items.iter().map(|i| i.id).max().unwrap_or(0);
    ItemId::from(max_id + 1)
}
```

Apply similar pattern for spells, monsters, maps.

#### 3A.5 Add ID Validation on Load

**Files:** `sdk/campaign_builder/src/main.rs` load functions

Update `load_items()`, `load_spells()`, `load_monsters()`:

```rust
fn load_items(&mut self) {
    // ... existing load code ...

    // Validate after load
    let id_errors = self.validate_item_ids();
    if !id_errors.is_empty() {
        self.validation_errors.extend(id_errors);
        self.status_message = format!(
            "⚠️ Loaded {} items with {} ID conflicts",
            self.items.len(),
            id_errors.len()
        );
    }
}
```

#### 3A.6 Testing Requirements

Create tests in `main.rs` test module:

- `test_item_id_uniqueness_validation` - Detect duplicate IDs
- `test_next_available_id_generation` - ID generation with gaps
- `test_load_items_with_duplicate_ids` - Error handling on load
- `test_duplicate_item_creates_unique_id` - Duplicate operation safety

#### 3A.7 Deliverables

- [ ] All ID fields use type aliases (ItemId, SpellId, MonsterId)
- [ ] ID uniqueness validation on load
- [ ] Safe ID generation helpers
- [ ] Duplicate detection in validation panel
- [ ] 4+ new tests for ID management
- [ ] Zero cargo clippy warnings

#### 3A.8 Success Criteria

- User loads "foo" campaign without ID clash errors
- Adding new items generates unique IDs
- Duplicating items creates new unique IDs
- Validation panel shows ID conflicts if present
- All quality gates pass

---

### Phase 3B: Items Editor Enhancement (Week 2)

**Priority:** HIGH - Most commonly used editor

#### 3B.1 Add ItemType-Specific Editors

**Files:** `sdk/campaign_builder/src/main.rs` `show_items_form()`

Expand form to handle all ItemType variants:

- Weapon editor: damage dice, attack bonus, weapon class
- Armor editor: AC bonus, armor class
- Consumable editor: effect, charges, consumption behavior
- Accessory editor: stat modifiers, special effects

Current code has placeholder:

```rust
// Type-specific editors would go here
ui.label(format!("Type: {:?}", self.items_edit_buffer.item_type));
```

Replace with:

```rust
ui.collapsing("Item Type Details", |ui| {
    match &mut self.items_edit_buffer.item_type {
        ItemType::Weapon(weapon_data) => self.show_weapon_editor(ui, weapon_data),
        ItemType::Armor(armor_data) => self.show_armor_editor(ui, armor_data),
        ItemType::Consumable(consumable_data) => self.show_consumable_editor(ui, consumable_data),
        ItemType::Accessory(accessory_data) => self.show_accessory_editor(ui, accessory_data),
        ItemType::QuestItem => ui.label("Quest items have no additional properties"),
    }
});
```

#### 3B.2 Implement Disablement Editor

**Files:** `sdk/campaign_builder/src/main.rs`

Add UI for class/alignment restrictions:

```rust
fn show_disablement_editor(&mut self, ui: &mut egui::Ui, disablement: &mut Disablement) {
    ui.label("Class Restrictions:");
    ui.checkbox(&mut disablement.knight_disabled, "Knight cannot use");
    ui.checkbox(&mut disablement.paladin_disabled, "Paladin cannot use");
    // ... all classes

    ui.separator();
    ui.label("Alignment Restrictions:");
    // Alignment checkboxes
}
```

#### 3B.3 Add Item Preview Panel

**Files:** `sdk/campaign_builder/src/main.rs` `show_items_list()`

Enhance right panel details to show:

- All item properties formatted
- Calculated values (effective damage, total AC bonus)
- Visual indicators for special properties (cursed, magical)
- Disablement restrictions summary

#### 3B.4 Implement Item Search Filters

**Files:** `sdk/campaign_builder/src/main.rs`

Add filter dropdowns above search:

```rust
ui.horizontal(|ui| {
    ui.label("Filter by type:");
    egui::ComboBox::from_label("")
        .selected_text(format!("{:?}", self.items_filter_type))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut self.items_filter_type, None, "All Types");
            ui.selectable_value(&mut self.items_filter_type, Some(ItemType::Weapon), "Weapons");
            // ... etc
        });
});
```

#### 3B.5 Add Item Import/Export

**Files:** `sdk/campaign_builder/src/main.rs`

Add buttons to toolbar:

- Import from another campaign's items.ron
- Export selected items to file
- Bulk operations (delete, modify stats)

#### 3B.6 Testing Requirements

- `test_weapon_editor_updates_data`
- `test_armor_editor_updates_data`
- `test_disablement_restrictions_toggle`
- `test_item_search_filter_by_type`
- `test_item_import_validates_ids`

#### 3B.7 Deliverables

- [ ] Full item type editors (weapon, armor, consumable, accessory)
- [ ] Disablement restriction UI
- [ ] Enhanced preview panel
- [ ] Search filters by type
- [ ] Import/export utilities
- [ ] 5+ new tests

#### 3B.8 Success Criteria

- Can create complete weapon with all properties
- Can set class restrictions on items
- Can filter item list by type
- Can export items for reuse in other campaigns

---

### Phase 3C: Spells and Monsters Editor Enhancement (Week 3)

**Priority:** HIGH - Core gameplay data

#### 3C.1 Enhance Spell Editor

**Files:** `sdk/campaign_builder/src/main.rs` `show_spells_form()`

Add missing fields:

- `context: SpellContext` (Exploration, Combat, Both)
- `target: SpellTarget` (Self, Single, Group, All)
- Effect description (multiline text)
- Visual/audio effect references

Current form only has: name, school, level, costs, description.

#### 3C.2 Add Spell Filtering by School/Level

**Files:** `sdk/campaign_builder/src/main.rs` `show_spells_list()`

Add filter controls:

```rust
ui.horizontal(|ui| {
    ui.label("School:");
    for school in [SpellSchool::Cleric, SpellSchool::Sorcerer] {
        if ui.selectable_label(self.spell_school_filter == Some(school),
                              format!("{:?}", school)).clicked() {
            self.spell_school_filter = Some(school);
        }
    }

    ui.separator();
    ui.label("Level:");
    for level in 1..=7 {
        if ui.selectable_label(self.spell_level_filter == Some(level),
                              format!("{}", level)).clicked() {
            self.spell_level_filter = Some(level);
        }
    }
});
```

#### 3C.3 Enhance Monster Editor

**Files:** `sdk/campaign_builder/src/main.rs` `show_monsters_form()`

Add missing sections:

- Attack editor (multiple attacks, attack types, damage dice)
- Resistances editor (magic, physical, elemental)
- Special abilities (regeneration rate, advance distance)
- AI behavior flags

Current form has basic stats but attacks are hardcoded in `default_monster()`.

#### 3C.4 Implement Monster Attack Editor

**Files:** `sdk/campaign_builder/src/main.rs`

Create attack management UI:

```rust
fn show_monster_attacks_editor(&mut self, ui: &mut egui::Ui, monster: &mut MonsterDefinition) {
    ui.collapsing("Attacks", |ui| {
        for (idx, attack) in monster.attacks.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.label(format!("Attack {}", idx + 1));

                egui::ComboBox::from_id(format!("attack_type_{}", idx))
                    .selected_text(format!("{:?}", attack.attack_type))
                    .show_ui(ui, |ui| {
                        // Attack type selection
                    });

                // Damage dice editor
                ui.horizontal(|ui| {
                    ui.label("Damage:");
                    ui.add(egui::DragValue::new(&mut attack.damage.count));
                    ui.label("d");
                    ui.add(egui::DragValue::new(&mut attack.damage.sides));
                });

                if ui.button("🗑️ Remove").clicked() {
                    // Mark for removal
                }
            });
        }

        if ui.button("➕ Add Attack").clicked() {
            monster.attacks.push(Attack::default());
        }
    });
}
```

#### 3C.5 Add Loot Table Editor

**Files:** `sdk/campaign_builder/src/main.rs`

Enhance loot editing beyond current min/max:

- Item drop chances (specific items)
- Equipment drop percentages
- Treasure tier selection
- Special loot flags

#### 3C.6 Add Monster Stat Calculator

**Files:** `sdk/campaign_builder/src/main.rs`

Helper tool that suggests stats based on:

- Challenge rating
- Party level
- Encounter type (boss, minion, standard)

#### 3C.7 Testing Requirements

- `test_spell_context_target_editing`
- `test_spell_filtering_by_school`
- `test_monster_attack_add_remove`
- `test_monster_resistances_editor`
- `test_loot_table_validation`

#### 3C.8 Deliverables

- [ ] Complete spell editor with context/target
- [ ] Spell filtering by school and level
- [ ] Monster attack editor (add/remove/edit)
- [ ] Monster resistances editor
- [ ] Enhanced loot table editor
- [ ] 5+ new tests

#### 3C.9 Success Criteria

- Can create spell with all properties (context, target, effects)
- Can filter spells by school and level
- Can add multiple attacks to monsters
- Can configure monster resistances and special abilities

---

### Phase 4A: Quest Editor Integration (Week 4)

**Priority:** HIGH - Backend exists, needs UI wiring

#### 4A.1 Replace Placeholder Quest UI

**Files:** `sdk/campaign_builder/src/main.rs` `show_quests_editor()`

Remove placeholder and integrate `QuestEditorState`:

```rust
fn show_quests_editor(&mut self, ui: &mut egui::Ui) {
    ui.heading("📜 Quests Editor");
    ui.add_space(5.0);

    // Top toolbar (same pattern as items/spells)
    ui.horizontal(|ui| {
        ui.label("🔍 Search:");
        if ui.text_edit_singleline(&mut self.quest_editor_state.search_filter).changed() {
            // Filter updates handled by backend
        }

        if ui.button("➕ Add Quest").clicked() {
            self.quest_editor_state.start_new_quest();
        }

        if ui.button("🔄 Reload").clicked() {
            self.quest_editor_state.load_quests(&self.campaign_dir);
        }

        ui.label(format!("Total: {}", self.quest_editor_state.quests.len()));
    });

    ui.separator();

    match self.quest_editor_state.mode {
        QuestEditorMode::List => self.show_quest_list(ui),
        QuestEditorMode::Creating | QuestEditorMode::Editing => self.show_quest_form(ui),
    }
}
```

#### 4A.2 Implement Quest List View

**Files:** `sdk/campaign_builder/src/main.rs`

Create list/detail split view:

```rust
fn show_quest_list(&mut self, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        // Left: Quest list
        ui.vertical(|ui| {
            ui.set_width(300.0);
            ui.heading("Quests");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                let filtered = self.quest_editor_state.filtered_quests();
                for (idx, quest) in filtered.iter().enumerate() {
                    let icon = if quest.is_main_quest { "⭐" } else { "📜" };
                    let label = format!("{} {} (Lv{})", icon, quest.name, quest.min_level);

                    if ui.selectable_label(
                        self.quest_editor_state.selected_quest == Some(idx),
                        label
                    ).clicked() {
                        self.quest_editor_state.selected_quest = Some(idx);
                    }
                }
            });
        });

        ui.separator();

        // Right: Quest details
        ui.vertical(|ui| {
            if let Some(idx) = self.quest_editor_state.selected_quest {
                self.show_quest_details(ui, idx);
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.label("Select a quest to view details");
                });
            }
        });
    });
}
```

#### 4A.3 Implement Quest Form Editor

**Files:** `sdk/campaign_builder/src/main.rs`

Create quest editing form:

```rust
fn show_quest_form(&mut self, ui: &mut egui::Ui) {
    let buffer = &mut self.quest_editor_state.quest_buffer;

    ui.heading(if self.quest_editor_state.mode == QuestEditorMode::Creating {
        "Create New Quest"
    } else {
        "Edit Quest"
    });

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Basic properties
        ui.horizontal(|ui| {
            ui.label("ID:");
            ui.text_edit_singleline(&mut buffer.id);
        });

        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut buffer.name);
        });

        ui.label("Description:");
        ui.text_edit_multiline(&mut buffer.description);

        // Level range
        ui.horizontal(|ui| {
            ui.label("Min Level:");
            ui.add(egui::Slider::new(&mut buffer.min_level, 1..=20));
            ui.label("Max Level:");
            ui.add(egui::Slider::new(&mut buffer.max_level, 1..=20));
        });

        ui.checkbox(&mut buffer.repeatable, "Repeatable");
        ui.checkbox(&mut buffer.is_main_quest, "Main Quest");

        ui.separator();

        // Quest giver info
        ui.collapsing("Quest Giver", |ui| {
            ui.horizontal(|ui| {
                ui.label("NPC ID:");
                ui.text_edit_singleline(&mut buffer.quest_giver_npc);
            });
            ui.horizontal(|ui| {
                ui.label("Map:");
                ui.text_edit_singleline(&mut buffer.quest_giver_map);
            });
            ui.horizontal(|ui| {
                ui.label("Position:");
                ui.add(egui::DragValue::new(&mut buffer.quest_giver_x));
                ui.label(",");
                ui.add(egui::DragValue::new(&mut buffer.quest_giver_y));
            });
        });

        ui.separator();

        // Stages section
        self.show_quest_stages_editor(ui);

        ui.separator();

        // Save/Cancel
        ui.horizontal(|ui| {
            if ui.button("💾 Save").clicked() {
                if let Err(e) = self.quest_editor_state.save_quest() {
                    self.status_message = format!("Failed to save quest: {}", e);
                } else {
                    self.status_message = "Quest saved".to_string();
                }
            }

            if ui.button("❌ Cancel").clicked() {
                self.quest_editor_state.cancel_edit();
            }
        });
    });
}
```

#### 4A.4 Implement Quest Stages Editor

**Files:** `sdk/campaign_builder/src/main.rs`

Create collapsible stages UI:

```rust
fn show_quest_stages_editor(&mut self, ui: &mut egui::Ui) {
    ui.collapsing("Quest Stages", |ui| {
        let stage_count = self.quest_editor_state.quest_buffer.stages.len();

        for stage_idx in 0..stage_count {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Stage {}", stage_idx + 1));
                    if ui.button("🗑️").clicked() {
                        // Mark for deletion
                    }
                });

                // Stage details would go here
                self.show_quest_objectives_editor(ui, stage_idx);
            });
        }

        if ui.button("➕ Add Stage").clicked() {
            self.quest_editor_state.add_stage();
        }
    });
}

fn show_quest_objectives_editor(&mut self, ui: &mut egui::Ui, stage_idx: usize) {
    ui.collapsing("Objectives", |ui| {
        // Objective list and add/edit UI
        if ui.button("➕ Add Objective").clicked() {
            self.quest_editor_state.add_objective(stage_idx);
        }
    });
}
```

#### 4A.5 Add Quest Validation Display

**Files:** `sdk/campaign_builder/src/main.rs`

Show validation errors in quest form:

```rust
// After quest form, before save button
if !self.quest_editor_state.validation_errors.is_empty() {
    ui.separator();
    ui.colored_label(egui::Color32::RED, "⚠️ Validation Errors:");
    for error in &self.quest_editor_state.validation_errors {
        ui.label(format!("  • {}", error));
    }
}
```

#### 4A.6 Implement Quest Save/Load Integration

**Files:** `sdk/campaign_builder/src/main.rs`

Wire up quest persistence:

```rust
impl CampaignBuilderApp {
    fn load_quests(&mut self) {
        if let Some(ref dir) = self.campaign_dir {
            let quests_path = dir.join(&self.campaign.quests_file);
            if let Err(e) = self.quest_editor_state.load_quests(&quests_path) {
                self.status_message = format!("Failed to load quests: {}", e);
            }
        }
    }

    fn save_quests(&mut self) -> Result<(), String> {
        if let Some(ref dir) = self.campaign_dir {
            let quests_path = dir.join(&self.campaign.quests_file);
            // Save quest_editor_state.quests to RON
            // Similar pattern to save_items()
        }
        Ok(())
    }
}
```

Call `load_quests()` in `do_open_campaign()` and quest reload button.

#### 4A.7 Testing Requirements

- `test_quest_editor_initialization`
- `test_quest_create_and_save`
- `test_quest_stage_add_remove`
- `test_quest_validation_errors`
- `test_quest_search_filter`

#### 4A.8 Deliverables

- [ ] Quest list view with search/filter
- [ ] Quest form editor with all fields
- [ ] Quest stages editor
- [ ] Quest objectives editor (basic)
- [ ] Quest validation display
- [ ] Quest save/load integration
- [ ] 5+ new tests

#### 4A.9 Success Criteria

- Can view existing quests from quests.ron
- Can create new quest with name, description, levels
- Can add stages to quest
- Can save quest to file
- Quest validation shows errors before save

---

### Phase 4B: Dialogue Editor Integration (Week 5)

**Priority:** HIGH - Backend exists, needs tree visualization

#### 4B.1 Replace Placeholder Dialogue UI

**Files:** `sdk/campaign_builder/src/main.rs` `show_dialogues_editor()`

Similar pattern to quests - replace basic list with full editor:

```rust
fn show_dialogues_editor(&mut self, ui: &mut egui::Ui) {
    ui.heading("💬 Dialogues Editor");
    ui.add_space(5.0);

    ui.horizontal(|ui| {
        ui.label("🔍 Search:");
        ui.text_edit_singleline(&mut self.dialogue_editor_state.search_filter);

        if ui.button("➕ Add Dialogue").clicked() {
            self.dialogue_editor_state.start_new_dialogue();
        }

        if ui.button("🔄 Reload").clicked() {
            self.load_dialogues();
        }

        ui.label(format!("Total: {}", self.dialogue_editor_state.dialogues.len()));
    });

    ui.separator();

    match self.dialogue_editor_state.mode {
        DialogueEditorMode::List => self.show_dialogue_list(ui),
        DialogueEditorMode::Creating | DialogueEditorMode::Editing => self.show_dialogue_form(ui),
    }
}
```

#### 4B.2 Implement Dialogue List View

**Files:** `sdk/campaign_builder/src/main.rs`

```rust
fn show_dialogue_list(&mut self, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        // Left: Dialogue list
        ui.vertical(|ui| {
            ui.set_width(300.0);
            ui.heading("Dialogues");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                let filtered = self.dialogue_editor_state.filtered_dialogues();
                for (idx, dialogue) in filtered.iter().enumerate() {
                    let label = format!("💬 {} ({} nodes)",
                                      dialogue.name,
                                      dialogue.nodes.len());

                    if ui.selectable_label(
                        self.dialogue_editor_state.selected_dialogue == Some(idx),
                        label
                    ).clicked() {
                        self.dialogue_editor_state.selected_dialogue = Some(idx);
                    }
                }
            });
        });

        ui.separator();

        // Right: Dialogue preview/tree
        ui.vertical(|ui| {
            if let Some(idx) = self.dialogue_editor_state.selected_dialogue {
                self.show_dialogue_tree_preview(ui, idx);
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.label("Select a dialogue to view tree");
                });
            }
        });
    });
}
```

#### 4B.3 Implement Dialogue Form Editor

**Files:** `sdk/campaign_builder/src/main.rs`

```rust
fn show_dialogue_form(&mut self, ui: &mut egui::Ui) {
    let buffer = &mut self.dialogue_editor_state.dialogue_buffer;

    ui.heading("Edit Dialogue");

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("ID:");
            ui.text_edit_singleline(&mut buffer.id);
        });

        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut buffer.name);
        });

        ui.horizontal(|ui| {
            ui.label("Speaker:");
            ui.text_edit_singleline(&mut buffer.speaker_name);
        });

        ui.checkbox(&mut buffer.repeatable, "Repeatable");

        ui.horizontal(|ui| {
            ui.label("Associated Quest:");
            ui.text_edit_singleline(&mut buffer.associated_quest);
        });

        ui.separator();

        // Dialogue nodes tree
        self.show_dialogue_nodes_editor(ui);

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("💾 Save").clicked() {
                if let Err(e) = self.dialogue_editor_state.save_dialogue() {
                    self.status_message = format!("Failed to save dialogue: {}", e);
                } else {
                    self.status_message = "Dialogue saved".to_string();
                }
            }

            if ui.button("❌ Cancel").clicked() {
                self.dialogue_editor_state.cancel_edit();
            }
        });
    });
}
```

#### 4B.4 Implement Dialogue Node Tree Editor

**Files:** `sdk/campaign_builder/src/main.rs`

Create hierarchical node editor:

```rust
fn show_dialogue_nodes_editor(&mut self, ui: &mut egui::Ui) {
    ui.collapsing("Dialogue Nodes", |ui| {
        // Show root node
        if let Some(root_idx) = self.dialogue_editor_state.selected_dialogue {
            self.show_dialogue_node_recursive(ui, root_idx, 0);
        }

        if ui.button("➕ Add Node").clicked() {
            self.dialogue_editor_state.add_node();
        }
    });
}

fn show_dialogue_node_recursive(&mut self, ui: &mut egui::Ui, node_idx: usize, depth: usize) {
    let indent = depth as f32 * 20.0;

    ui.horizontal(|ui| {
        ui.add_space(indent);

        // Node display
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Node {}", node_idx));
                if ui.button("✏️").clicked() {
                    self.dialogue_editor_state.selected_node = Some(node_idx);
                }
                if ui.button("🗑️").clicked() {
                    // Delete node
                }
            });

            // Show node text preview (first 50 chars)
            // Show choices
        });
    });

    // Recursively show child nodes
}
```

#### 4B.5 Add Condition/Action Editors

**Files:** `sdk/campaign_builder/src/main.rs`

Integrate condition and action buffer editors:

```rust
fn show_node_conditions_editor(&mut self, ui: &mut egui::Ui) {
    ui.collapsing("Conditions", |ui| {
        let buffer = &mut self.dialogue_editor_state.condition_buffer;

        egui::ComboBox::from_label("Condition Type")
            .selected_text(buffer.condition_type.as_str())
            .show_ui(ui, |ui| {
                for cond_type in [
                    ConditionType::HasQuest,
                    ConditionType::CompletedQuest,
                    ConditionType::HasItem,
                    // ... etc
                ] {
                    ui.selectable_value(&mut buffer.condition_type, cond_type, cond_type.as_str());
                }
            });

        // Show fields relevant to selected condition type
        match buffer.condition_type {
            ConditionType::HasQuest | ConditionType::CompletedQuest => {
                ui.text_edit_singleline(&mut buffer.quest_id);
            },
            ConditionType::HasItem => {
                ui.horizontal(|ui| {
                    ui.label("Item ID:");
                    ui.add(egui::DragValue::new(&mut buffer.item_id));
                    ui.label("Quantity:");
                    ui.add(egui::DragValue::new(&mut buffer.item_quantity));
                });
            },
            // ... other types
        }

        if ui.button("➕ Add Condition").clicked() {
            // Build condition from buffer and add to current node
        }
    });
}

fn show_node_actions_editor(&mut self, ui: &mut egui::Ui) {
    // Similar pattern for actions
}
```

#### 4B.6 Implement Dialogue Tree Visualization

**Files:** `sdk/campaign_builder/src/main.rs`

Create visual tree preview:

```rust
fn show_dialogue_tree_preview(&self, ui: &mut egui::Ui, dialogue_idx: usize) {
    ui.heading("Dialogue Tree");
    ui.separator();

    egui::ScrollArea::both().show(ui, |ui| {
        // ASCII-art style tree or simple indented list
        let preview = self.dialogue_editor_state.get_dialogue_preview(dialogue_idx);
        ui.monospace(&preview);
    });

    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("✏️ Edit").clicked() {
            self.dialogue_editor_state.start_edit_dialogue(dialogue_idx);
        }
        if ui.button("🗑️ Delete").clicked() {
            self.dialogue_editor_state.delete_dialogue(dialogue_idx);
        }
    });
}
```

#### 4B.7 Testing Requirements

- `test_dialogue_editor_initialization`
- `test_dialogue_create_and_save`
- `test_dialogue_node_add_remove`
- `test_dialogue_condition_creation`
- `test_dialogue_action_creation`

#### 4B.8 Deliverables

- [ ] Dialogue list view with search
- [ ] Dialogue form editor with metadata
- [ ] Dialogue node tree editor
- [ ] Condition editor with all types
- [ ] Action editor with all types
- [ ] Dialogue tree visualization
- [ ] 5+ new tests

#### 4B.9 Success Criteria

- Can view existing dialogues
- Can create new dialogue with nodes
- Can add choices to nodes
- Can set conditions on choices
- Can add actions to nodes
- Tree visualization shows structure

---

### Phase 5: Asset Manager Reference Tracking (Week 6)

**Priority:** MEDIUM - Usability feature

#### 5.1 Implement Asset Reference Scanner

**Files:** `sdk/campaign_builder/src/asset_manager.rs`

Add method to scan data files for asset references:

```rust
impl AssetManager {
    /// Scan campaign data files and mark referenced assets
    pub fn scan_references(&mut self, campaign_dir: &Path) -> Result<(), String> {
        // Reset all references
        for asset in self.assets.values_mut() {
            asset.is_referenced = false;
        }

        // Scan items.ron for image/icon references
        self.scan_items_references(campaign_dir)?;

        // Scan maps for tileset references
        self.scan_map_references(campaign_dir)?;

        // Scan dialogue for portrait references
        self.scan_dialogue_references(campaign_dir)?;

        // Scan for music/sound references
        self.scan_audio_references(campaign_dir)?;

        Ok(())
    }

    fn scan_items_references(&mut self, campaign_dir: &Path) -> Result<(), String> {
        let items_path = campaign_dir.join("data/items.ron");
        if !items_path.exists() {
            return Ok(());
        }

        let contents = std::fs::read_to_string(&items_path)
            .map_err(|e| format!("Failed to read items: {}", e))?;

        // Find all asset paths in the file (basic string search)
        for asset_path in self.assets.keys() {
            if let Some(file_name) = asset_path.file_name() {
                if contents.contains(file_name.to_str().unwrap_or("")) {
                    if let Some(asset) = self.assets.get_mut(asset_path) {
                        asset.is_referenced = true;
                    }
                }
            }
        }

        Ok(())
    }

    fn scan_map_references(&mut self, campaign_dir: &Path) -> Result<(), String> {
        // Similar pattern for maps
        Ok(())
    }

    // ... other scan methods
}
```

#### 5.2 Add Reference Scanning to UI

**Files:** `sdk/campaign_builder/src/main.rs` `show_assets_editor()`

Add button to trigger scan:

```rust
ui.horizontal(|ui| {
    ui.label(format!("Total Assets: {}", manager.asset_count()));
    ui.separator();
    if ui.button("🔄 Refresh").clicked() {
        if let Err(e) = manager.scan_directory() {
            self.status_message = format!("Failed to refresh: {}", e);
        }
    }
    if ui.button("🔍 Scan References").clicked() {
        if let Some(ref dir) = self.campaign_dir {
            if let Err(e) = manager.scan_references(dir) {
                self.status_message = format!("Failed to scan references: {}", e);
            } else {
                self.status_message = "Reference scan complete".to_string();
            }
        }
    }
});
```

#### 5.3 Add Asset Usage Context Display

**Files:** `sdk/campaign_builder/src/main.rs`

When showing asset details, show where it's referenced:

```rust
for (path, asset) in manager.assets() {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(format!("📄 {}", path.display()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if asset.is_referenced {
                    ui.colored_label(egui::Color32::GREEN, "✓ Used");
                } else {
                    ui.colored_label(egui::Color32::YELLOW, "⚠ Unused");
                }
                ui.label(asset.size_string());
                ui.label(asset.asset_type.display_name());
            });
        });

        // If we tracked usage details, show them here
        if let Some(ref usage) = asset.used_in {
            ui.label(format!("Used in: {}", usage.join(", ")));
        }
    });
}
```

#### 5.4 Add Asset Cleanup Utility

**Files:** `sdk/campaign_builder/src/asset_manager.rs`

```rust
impl AssetManager {
    /// Get list of assets not referenced anywhere
    pub fn get_cleanup_candidates(&self) -> Vec<&PathBuf> {
        self.unreferenced_assets()
    }

    /// Delete unreferenced assets (with confirmation)
    pub fn cleanup_unused(&mut self) -> Result<usize, String> {
        let unused = self.unreferenced_assets();
        let mut deleted = 0;

        for path in unused {
            if let Err(e) = std::fs::remove_file(path) {
                return Err(format!("Failed to delete {}: {}", path.display(), e));
            }
            self.remove_asset(path)?;
            deleted += 1;
        }

        Ok(deleted)
    }
}
```

Add cleanup button to UI with confirmation dialog.

#### 5.5 Testing Requirements

- `test_asset_reference_scanning`
- `test_mark_referenced_updates_flag`
- `test_scan_items_finds_references`
- `test_scan_maps_finds_tilesets`
- `test_unreferenced_assets_list_accuracy`

#### 5.6 Deliverables

- [ ] Asset reference scanning implementation
- [ ] UI button to trigger reference scan
- [ ] Visual indication of used vs unused assets
- [ ] Asset cleanup utility (with confirmation)
- [ ] 5+ new tests

#### 5.7 Success Criteria

- After scanning references, only truly unused assets marked as "Unused"
- Can identify which data files reference each asset
- Can safely delete unreferenced assets
- False positive rate < 5%

---

### Phase 6: Maps Editor Enhancement (Week 7-8)

**Priority:** MEDIUM - Complex feature, can use external tool

#### 6.1 Integrate Map Preview

**Files:** `sdk/campaign_builder/src/main.rs` `show_map_preview()`

Enhance existing preview to show:

- Tile grid with colors for tile types
- Event markers
- Entry/exit points
- NPC locations

Current preview (line 2891-2940) is basic. Enhance with proper rendering.

#### 6.2 Add Basic Tile Painting

**Files:** `sdk/campaign_builder/src/map_editor.rs`

Extend `MapEditorState` with:

- Tile palette selection
- Click-to-paint tile changes
- Brush size selector
- Tile type filters

#### 6.3 Add Event Placement Tool

**Files:** `sdk/campaign_builder/src/map_editor.rs`

```rust
impl MapEditorState {
    pub fn add_event_at_position(&mut self, x: u32, y: u32, event_type: EventType) {
        // Add event to map at coordinates
    }

    pub fn show_event_editor(&self, ui: &mut egui::Ui) {
        // Event configuration UI
    }
}
```

#### 6.4 Map Metadata Editor

**Files:** `sdk/campaign_builder/src/main.rs`

Add form for map properties:

- Map ID, name, description
- Outdoor vs indoor flag
- Light level
- Music track
- Random encounter settings

#### 6.5 Testing Requirements

- `test_map_preview_renders`
- `test_tile_painting_updates_map`
- `test_event_placement`
- `test_map_metadata_editing`

#### 6.6 Deliverables

- [ ] Enhanced map preview with tile colors
- [ ] Basic tile painting
- [ ] Event placement tool
- [ ] Map metadata editor
- [ ] 4+ new tests

#### 6.7 Success Criteria

- Can visualize map layout in preview
- Can place events on map
- Can edit map metadata
- Can save map changes
- (Note: Full map editor is complex - this is MVP integration)

---

### Phase 7: Quest and Dialogue Editor Refinements (Week 9)

**Priority:** MEDIUM - Enhance existing editors with missing CRUD operations

**Status:** Backend COMPLETED (2025-01-25), UI Integration IN PROGRESS

**Rationale:** Phases 4A and 4B implemented creation and viewing, but lack full editing and deletion capabilities for nested structures (quest objectives, dialogue nodes/choices).

#### 7.1 Quest Objective Editing (Backend - ✅ COMPLETED)

**Files:** `sdk/campaign_builder/src/quest_editor.rs`

**Status:** Backend implementation complete with full test coverage.

**Implemented Methods:**

- ✅ `edit_objective(quest_idx, stage_idx, objective_idx)` - Loads objective into edit buffer
- ✅ `save_objective(quest_idx, stage_idx, objective_idx)` - Saves changes to existing objective
- ✅ `delete_objective(quest_idx, stage_idx, objective_idx)` - Removes objective from stage

**Implementation Details:**

- Supports all 7 objective types (KillMonsters, CollectItems, ReachLocation, TalkToNpc, DeliverItem, EscortNpc, CustomFlag)
- Properly populates `ObjectiveEditBuffer` with all fields based on objective type
- Validates parsed values (IDs, quantities, coordinates) before saving
- Updates `has_unsaved_changes` flag on modifications
- Clears selection state on delete

**Tests Added:** 3 tests (edit, delete, save)

#### 7.2 Quest Stage Editing (Backend - ✅ COMPLETED)

**Files:** `sdk/campaign_builder/src/quest_editor.rs`

**Status:** Backend implementation complete with full test coverage.

**Implemented Methods:**

- ✅ `edit_stage(quest_idx, stage_idx)` - Loads stage into edit buffer
- ✅ `save_stage(quest_idx, stage_idx)` - Saves changes to existing stage
- ✅ `delete_stage(quest_idx, stage_idx)` - Removes stage from quest

**Implementation Details:**

- Populates `StageEditBuffer` with stage number, name, description, and `require_all_objectives` flag
- In-place updates maintain stage structure and objectives
- Delete operation properly maintains quest integrity
- Handles selection state clearing

**Tests Added:** 2 tests (edit, delete)

#### 7.3 Dialogue Node Editing (Backend - ✅ COMPLETED)

**Files:** `sdk/campaign_builder/src/dialogue_editor.rs`

**Status:** Backend implementation complete with full test coverage.

**Implemented Methods:**

- ✅ `edit_node(dialogue_idx, node_id)` - Loads node into edit buffer
- ✅ `save_node(dialogue_idx, node_id)` - Saves changes to existing node
- ✅ `delete_node(dialogue_idx, node_id)` - Removes node from dialogue tree

**Implementation Details:**

- Populates `NodeEditBuffer` with node ID, text, speaker override, and terminal flag
- Prevents deletion of root node (returns error)
- Updates node in dialogue's HashMap structure
- Handles `Option<String>` for speaker override properly

**Tests Added:** 3 tests (edit, delete, save)

#### 7.4 Dialogue Choice Editing (Backend - ✅ COMPLETED)

**Files:** `sdk/campaign_builder/src/dialogue_editor.rs`

**Status:** Backend implementation complete with full test coverage.

**Implemented Methods:**

- ✅ `edit_choice(dialogue_idx, node_id, choice_idx)` - Loads choice into edit buffer
- ✅ `save_choice(dialogue_idx, node_id, choice_idx)` - Saves changes to existing choice
- ✅ `delete_choice(dialogue_idx, node_id, choice_idx)` - Removes choice from node

**Implementation Details:**

- Populates `ChoiceEditBuffer` with choice text, target node ID, and ends_dialogue flag
- Handles `Option<NodeId>` for target nodes (None means ends dialogue)
- Parses target node string and validates before saving
- Updates choice in node's choices vector

**Tests Added:** 3 tests (edit, delete, save)

#### 7.5 Orphaned Content Detection (Backend - ✅ COMPLETED)

**Files:** `sdk/campaign_builder/src/quest_editor.rs`, `sdk/campaign_builder/src/dialogue_editor.rs`

**Status:** Backend implementation complete with full test coverage.

**Quest Editor Implementation:**

```rust
impl QuestEditorState {
    pub fn find_orphaned_objectives(&self) -> Vec<(QuestId, u8)> {
        // Returns (quest_id, stage_number) for stages with no objectives
    }
}
```

- Iterates through all quests and stages
- Returns tuples of quest ID and stage number for empty stages

**Dialogue Editor Implementation:**

```rust
impl DialogueEditorState {
    pub fn find_unreachable_nodes(&self) -> Vec<(DialogueId, Vec<NodeId>)> {
        // Returns (dialogue_id, unreachable_node_ids) for each dialogue
    }
}
```

- Uses breadth-first search (BFS) from root node
- Tracks all reachable nodes via choice target references
- Returns sorted list of unreachable node IDs per dialogue

**Tests Added:** 2 tests (orphaned objectives, unreachable nodes)

#### 7.6 UI Integration for Quest Editor (TODO)

**Files:** `sdk/campaign_builder/src/main.rs`

**Required Changes:**

Enhance `show_quest_stages_editor` (around line 5036):

- Add Edit and Delete buttons for each stage
- Show selected stage in edit mode with inline form
- Add confirmation dialog for stage deletion
- Wire up `edit_stage()`, `save_stage()`, and `delete_stage()` methods
- Display stage details in collapsible sections

Add new `show_quest_objectives_editor` function:

- Display objectives list for selected stage
- Add Edit button per objective (opens inline editor or modal)
- Add Delete button with confirmation
- Wire up `edit_objective()`, `save_objective()`, `delete_objective()` methods
- Show objective type-specific fields in edit mode
- Integrate into quest form editor UI

UI Pattern Example:

```
Stage 1: Find the Cave
  [Edit] [Delete]

  Objectives:
    • Kill 5 Goblins [Edit] [Delete]
    • Collect 3 Herbs [Edit] [Delete]
    [+ Add Objective]
```

#### 7.7 UI Integration for Dialogue Editor (TODO)

**Files:** `sdk/campaign_builder/src/main.rs`

**Required Changes:**

Enhance `show_dialogue_nodes_editor` (around line 5640):

- Add Edit and Delete buttons for each node
- Implement node editing modal or inline form
- Add confirmation dialog for node deletion
- Wire up `edit_node()`, `save_node()`, `delete_node()` methods
- Show root node indicator (prevent deletion)

Add choice editing to node display:

- Display choices with Edit and Delete buttons
- Add inline choice editor or modal
- Wire up `edit_choice()`, `save_choice()`, `delete_choice()` methods
- Show target node validation

UI Pattern Example:

```
Node 1 (ROOT)
  Text: "Hello, traveler!"
  [Edit] [Cannot Delete - Root Node]

  Choices:
    1. "Tell me about the quest" → Node 2 [Edit] [Delete]
    2. "Goodbye" → Ends Dialogue [Edit] [Delete]
    [+ Add Choice]
```

#### 7.8 Orphaned Content Validation Display (TODO)

**Files:** `sdk/campaign_builder/src/main.rs`

**Required Changes:**

Add validation panel to Quest Editor:

- Call `find_orphaned_objectives()` during validation
- Display warning badge with count of orphaned stages
- Add "Show Orphaned Content" button
- Create dialog/panel showing:
  - Quest ID and Stage number
  - "Navigate to Stage" button
  - "Delete Empty Stage" button

Add validation panel to Dialogue Editor:

- Call `find_unreachable_nodes()` during validation
- Display warning badge with count of unreachable nodes
- Add "Show Unreachable Nodes" button
- Create dialog/panel showing:
  - Dialogue ID and Node IDs
  - "Navigate to Node" button
  - "Delete Node" button (with cascade warning)

UI Pattern Example:

```
⚠️ Validation Issues (2)

Orphaned Quest Stages:
  • Quest 5, Stage 2: No objectives [Navigate] [Delete Stage]

Unreachable Dialogue Nodes:
  • Dialogue 3, Nodes: 8, 12, 15 [Show Details]
```

#### 7.9 Testing Requirements

**Backend Tests (✅ COMPLETED):**

- ✅ `test_edit_quest_objective` - Verifies objective editing loads correct data
- ✅ `test_delete_quest_objective` - Confirms objectives can be removed
- ✅ `test_save_edited_objective` - Validates changes are persisted correctly
- ✅ `test_edit_stage` - Verifies stage data loads correctly
- ✅ `test_delete_stage` - Confirms stages can be removed
- ✅ `test_edit_node` - Verifies node editing populates buffer correctly
- ✅ `test_delete_node` - Confirms root node cannot be deleted
- ✅ `test_save_edited_node` - Validates node changes persist correctly
- ✅ `test_edit_choice` - Verifies choice editing loads correct values
- ✅ `test_delete_choice` - Confirms choices can be removed
- ✅ `test_save_edited_choice` - Validates choice modifications are saved
- ✅ `test_find_orphaned_objectives` - Verifies detection of empty stages
- ✅ `test_find_unreachable_nodes` - Confirms unreachable nodes are found

**UI Integration Tests (TODO):**

- Manual testing of edit/delete buttons in quest editor
- Manual testing of edit/delete buttons in dialogue editor
- Manual testing of orphaned content display
- Verification of confirmation dialogs
- Test that unsaved changes are tracked correctly

#### 7.10 Deliverables

**Backend (✅ COMPLETED):**

- ✅ Quest objective editing methods (edit, save, delete)
- ✅ Quest stage editing methods (edit, save, delete)
- ✅ Dialogue node editing methods (edit, save, delete)
- ✅ Dialogue choice editing methods (edit, save, delete)
- ✅ Orphaned content detection for both editors
- ✅ 16 new tests covering CRUD operations
- ✅ Full documentation in `docs/explanation/implementations.md`

**UI Integration (TODO):**

- [ ] Quest stage editor with Edit/Delete buttons
- [ ] Quest objective editor with Edit/Delete buttons
- [ ] Dialogue node editor with Edit/Delete buttons
- [ ] Dialogue choice editor with Edit/Delete buttons
- [ ] Orphaned content validation display
- [ ] Confirmation dialogs for destructive operations
- [ ] Navigate to orphaned content functionality

#### 7.11 Success Criteria

**Backend (✅ ACHIEVED):**

- ✅ Can edit existing quest objectives and stages (methods implemented)
- ✅ Can delete quest content (methods implemented)
- ✅ Can edit existing dialogue nodes and choices (methods implemented)
- ✅ Can delete dialogue content with root node protection (methods implemented)
- ✅ Orphaned content is detected and reported (algorithms implemented)
- ✅ All CRUD operations properly validated
- ✅ All 262 campaign_builder tests pass

**UI Integration (PENDING REVIEW):**

- [ ] User can click Edit button on quest stages/objectives
- [ ] User can click Delete button with confirmation prompt
- [ ] User can click Edit button on dialogue nodes/choices
- [ ] User can click Delete button with appropriate warnings
- [ ] Orphaned content warnings visible in UI
- [ ] User can navigate to orphaned content
- [ ] All operations update unsaved changes flag
- [ ] No data loss on edit/delete operations

---

### Phase 8: Asset System Enhancements (Week 10)

**Priority:** LOW - Quality of life improvements to asset management

**Rationale:** Phase 5 implemented reference tracking using name-based heuristics. This phase adds explicit asset path fields to domain types for accurate tracking and extends asset management features.

#### 8.1 Add Asset Path Fields to Domain Types

**Files:** `antares/src/domain/items/types.rs`, `antares/src/domain/quest.rs`, `antares/src/domain/dialogue.rs`

**CRITICAL:** This modifies core data structures - requires architecture approval.

**Item struct enhancement:**

```rust
pub struct Item {
    // ... existing fields ...

    /// Optional icon/sprite asset path (relative to campaign assets directory)
    pub icon_path: Option<String>,
}
```

**Quest struct enhancement:**

```rust
pub struct Quest {
    // ... existing fields ...

    /// Optional quest icon asset path
    pub icon_path: Option<String>,
}
```

**DialogueTree struct enhancement:**

```rust
pub struct DialogueTree {
    // ... existing fields ...

    /// Optional portrait image for primary speaker
    pub speaker_portrait: Option<String>,
}
```

**Monster struct enhancement:**

```rust
pub struct Monster {
    // ... existing fields ...

    /// Optional sprite/portrait asset path
    pub sprite_path: Option<String>,
}
```

#### 8.2 Update Asset Scanner to Use Explicit Paths

**Files:** `sdk/campaign_builder/src/asset_manager.rs`

**Replace heuristic scanning with direct field checks:**

```rust
fn scan_items_references(&mut self, items: &[Item]) {
    for item in items {
        if let Some(ref icon_path) = item.icon_path {
            let path = PathBuf::from(icon_path);
            if let Some(asset) = self.assets.get_mut(&path) {
                asset.is_referenced = true;
                asset.references.push(AssetReference::Item {
                    id: item.id,
                    name: item.name.clone(),
                });
            }
        }
    }
}
```

Similar updates for quests, dialogues, and monsters.

#### 8.3 Add Asset Picker Widget

**Files:** `sdk/campaign_builder/src/asset_manager.rs`, `sdk/campaign_builder/src/main.rs`

**New widget for selecting assets in editors:**

```rust
impl AssetManager {
    pub fn show_asset_picker(
        &self,
        ui: &mut egui::Ui,
        selected_path: &mut Option<String>,
        filter_type: Option<AssetType>,
    ) -> bool {
        // Returns true if selection changed
        // Shows filterable list of assets with thumbnails (if image)
        // Double-click or Select button confirms choice
    }
}
```

**Integration points:**

- Item editor: Pick icon when creating/editing items
- Quest editor: Pick quest icon
- Dialogue editor: Pick speaker portrait
- Monster editor: Pick sprite

#### 8.4 Add Asset Preview Panel

**Files:** `sdk/campaign_builder/src/main.rs` (show_assets_editor)

**Enhancement to asset list:**

- Image assets show thumbnail preview (64x64 or 128x128)
- Audio assets show duration and format info
- Click asset to show full-size preview in side panel
- Preview panel shows:
  - Full image (if image asset)
  - Metadata (size, modified date, dimensions)
  - Reference list (already implemented)
  - Actions: Open in External Editor, Replace File, Delete

#### 8.5 Add Asset Import with Auto-Categorization

**Files:** `sdk/campaign_builder/src/main.rs` (show_assets_editor)

**Import wizard:**

- "Import Assets" button opens file dialog
- Multi-select support for batch import
- Auto-detects asset type from extension
- Suggests target directory based on type
- Option to override suggested directory
- Progress bar for batch imports
- Summary report: X files imported, Y skipped (duplicates)

#### 8.6 Add Asset Replacement

**Files:** `sdk/campaign_builder/src/asset_manager.rs`

**Method:**

```rust
impl AssetManager {
    pub fn replace_asset(
        &mut self,
        old_path: &Path,
        new_source_path: &Path,
    ) -> Result<(), std::io::Error> {
        // Replace file on disk
        // Update metadata
        // Preserve references
    }
}
```

**Use case:** Update an icon without changing references in items/quests.

#### 8.7 Testing Requirements

- `test_item_with_icon_path` - Verify Item serialization with icon_path field
- `test_quest_with_icon_path` - Verify Quest serialization with icon_path
- `test_dialogue_with_portrait` - Verify DialogueTree with speaker_portrait
- `test_scan_references_uses_explicit_paths` - Verify scanner uses fields not heuristics
- `test_asset_picker_filters_by_type` - Verify asset picker filtering
- `test_asset_replacement_preserves_references` - Verify replace doesn't break references
- `test_batch_asset_import` - Verify multi-file import

#### 8.8 Deliverables

- [ ] Asset path fields added to Item, Quest, DialogueTree, Monster structs
- [ ] Asset scanner updated to use explicit paths
- [ ] Asset picker widget implemented
- [ ] Asset preview panel with thumbnails
- [ ] Asset import wizard with batch support
- [ ] Asset replacement functionality
- [ ] Migration guide for existing campaigns (optional icon_path = None)
- [ ] 7+ new tests covering asset enhancements

#### 8.9 Success Criteria

- Domain types have optional asset path fields
- Asset scanning uses explicit paths (100% accurate)
- Can pick assets from UI when editing items/quests/dialogues
- Can preview image assets with thumbnails
- Can import multiple assets in one operation
- Can replace assets without breaking references
- Backward compatible with existing campaigns (None values)
- All quality gates pass (fmt, check, clippy, test)

#### 8.10 Migration Considerations

**Backward Compatibility:**

- All new asset path fields are `Option<String>` (None = no asset)
- Existing campaigns load without errors (fields default to None)
- Phase 5's heuristic scanning still works as fallback for None values

**Migration Path:**

1. Load existing campaign
2. Run Phase 5 heuristic scanner to populate references
3. Use "Asset Suggestions" tool to auto-populate icon_path fields based on references
4. Designer reviews and confirms suggestions
5. Save campaign with explicit paths

---

## Testing Strategy

### Unit Tests

Each phase must add:

- **Minimum 4 tests per phase**
- Cover success cases, failure cases, edge cases
- Use descriptive names: `test_{feature}_{condition}_{expected}`

### Integration Tests

Add to `sdk/campaign_builder/tests/`:

- `integration_test_campaign_workflow.rs` - Full create/edit/save cycle
- `integration_test_data_persistence.rs` - Save/load data integrity
- `integration_test_validation.rs` - End-to-end validation

### Manual Testing Checklist

After each phase, verify:

- [ ] All cargo quality gates pass (fmt, check, clippy, test)
- [ ] No new warnings introduced
- [ ] UI remains responsive (<100ms per frame)
- [ ] File I/O doesn't corrupt existing campaigns
- [ ] Validation errors are clear and actionable

### Regression Prevention

- [ ] Re-test previous phase features after each new phase
- [ ] Keep test campaign in `sdk/campaign_builder/test_data/`
- [ ] Document known issues in `KNOWN_ISSUES.md`

---

## Documentation Updates

### Required Documentation

After each phase, update:

1. **`sdk/campaign_builder/README.md`**

   - Move completed features from "Coming in Phase X" to "Implemented"
   - Update feature list with new capabilities
   - Add screenshots if GUI changed significantly

2. **`docs/explanation/implementations.md`**

   - Add summary of phase completion
   - Note any deviations from plan
   - Document design decisions

3. **`sdk/campaign_builder/QUICKSTART.md`**
   - Update with new workflows
   - Add keyboard shortcuts
   - Include troubleshooting for new features

### Code Documentation

- Every new public function must have `///` doc comments
- Include `# Examples` in doc comments where applicable
- Document complex algorithms with inline comments

---

## Success Metrics

### Phase 3 Success (Weeks 1-3)

- [ ] Zero ID clash errors when loading campaigns
- [ ] Items editor supports all item types (weapon, armor, etc.)
- [ ] Spells editor has context/target fields
- [ ] Monsters editor has attack editor
- [ ] All quality gates pass
- [ ] 15+ new tests added (5 per phase)

### Phase 4 Success (Weeks 4-5)

- [ ] Quest editor fully functional (create, edit, delete)
- [ ] Can add stages and objectives to quests
- [ ] Dialogue editor shows tree structure
- [ ] Can create dialogues with choices and conditions
- [ ] All quality gates pass
- [ ] 10+ new tests added

### Phase 5-6 Success (Weeks 6-8)

- [ ] Asset manager accurately tracks references
- [ ] Can identify unused assets for cleanup
- [ ] Map editor shows preview and allows basic editing
- [ ] Can place events on maps
- [ ] All quality gates pass
- [ ] 10+ new tests added

### Overall Completion Criteria

- [ ] User can create complete campaign without external tools
- [ ] All data editors functional (items, spells, monsters, quests, dialogues)
- [ ] No critical bugs (data corruption, crashes, ID conflicts)
- [ ] Test coverage > 80%
- [ ] Documentation complete and accurate
- [ ] Zero cargo clippy warnings
- [ ] Campaign Builder README updated to "Phase 6 Complete"

---

## Risk Mitigation

### Risk 1: Type System Refactoring Breaks Existing Code

**Mitigation:**

- Make changes incrementally (one editor at a time)
- Run tests after each change
- Keep backup of working state
- Use git branches for each phase

### Risk 2: UI Becomes Sluggish with Large Data Sets

**Mitigation:**

- Implement pagination for lists (show 50 items at a time)
- Add virtual scrolling for large lists
- Profile with campaigns containing 500+ items/monsters
- Cache filtered results

### Risk 3: Backend/UI Integration Complexity

**Mitigation:**

- Follow existing patterns (items/spells editors as templates)
- Quest and Dialogue backends already have clean APIs
- Test each integration point individually
- Add integration tests before UI work

### Risk 4: Asset Reference Scanning False Negatives

**Mitigation:**

- Start with conservative scanning (mark as used if in doubt)
- Add manual "Mark as Used" button
- Don't auto-delete, only suggest cleanup
- Log scanning decisions for debugging

---

## Dependencies and Prerequisites

### Technical Dependencies

- Rust 1.70+ (already met)
- egui 0.29 (already used)
- All dependencies already in `Cargo.toml`

### Knowledge Prerequisites

- Understanding of antares data structures (Section 4 of architecture.md)
- Familiarity with egui immediate-mode UI patterns
- RON serialization format
- Understanding of Golden Rules (AGENTS.md)

### Blockers

None identified. All required backend code exists:

- `quest_editor.rs` - 681 lines, ready
- `dialogue_editor.rs` - 738 lines, ready
- `asset_manager.rs` - 413 lines, mostly ready
- `map_editor.rs` - Exists, needs enhancement

---

## Open Questions

1. **Quest Objective Types**: Should we support custom objective types beyond the 7 defined? Answer: No, use CustomFlag for special cases.

2. **Dialogue Tree Visualization**: ASCII art or graphical node editor? Answer: Start with ASCII preview, graphical in future phase.

3. **Map Editor Scope**: Full tileset editor or basic placement only? Answer: Basic placement, use external `map_builder` for complex editing.

4. **Asset Import**: Support drag-and-drop from file system? Answer: Future enhancement, not in this plan.

5. **Undo/Redo**: When to implement? Answer: Phase 3A should add undo/redo manager integration for all editors.

---

## Implementation Order Summary

**Priority Order (by week):**

1. **Week 1**: Phase 3A (Type System + ID Management) - CRITICAL
2. **Week 2**: Phase 3B (Items Editor Enhancement) - HIGH
3. **Week 3**: Phase 3C (Spells/Monsters Enhancement) - HIGH
4. **Week 4**: Phase 4A (Quest Editor Integration) - HIGH
5. **Week 5**: Phase 4B (Dialogue Editor Integration) - HIGH
6. **Week 6**: Phase 5 (Asset Manager Reference Tracking) - MEDIUM
7. **Weeks 7-8**: Phase 6 (Maps Editor Enhancement) - ✅ **COMPLETED** (2025-01-25)
8. **Week 9**: Phase 7 (Quest/Dialogue Editor Refinements) - 🔨 **BACKEND COMPLETED** (2025-01-25), UI Integration TODO
9. **Week 10**: Phase 8 (Asset System Enhancements) - ⏸️ **PAUSED** (Pending game engine review)

**Critical Path:**
Phase 3A must complete before any other work (fixes foundation)
→ Phases 3B/3C can run in parallel
→ Phases 4A/4B can run in parallel after Phase 3
→ Phase 5 builds on Phase 4 (needs quest/dialogue data)
→ ✅ Phase 6 COMPLETED (maps editor with preview, painting, events, metadata)
→ 🔨 Phase 7 BACKEND COMPLETED (full CRUD for quest/dialogue editors, orphaned content detection)
→ Phase 7 UI Integration required before Phase 8
→ ⏸️ Phase 8 PAUSED (awaiting game engine architecture review before domain modifications)

**Parallel Work Opportunities:**

- Phases 3B and 3C (both editor enhancements)
- Phases 4A and 4B (both editor integrations)
- Phase 6 can be done anytime after Phase 3A (independent)

---

## Future Work: Game Engine Enhancements

**Note**: This plan focuses exclusively on Campaign Builder GUI completion. Game engine enhancements (such as 3D tile-based rendering) are **out of scope** for this document.

**Current Status (2025-01-25):**

✅ **Phase 6 COMPLETED**: Map editor with enhanced preview, tile painting, event placement, metadata editing
✅ **Phase 7 Backend COMPLETED**: Full CRUD operations for quest objectives, stages, dialogue nodes, choices; orphaned content detection
🔨 **Phase 7 UI Integration IN PROGRESS**: Wire up backend methods to UI with edit/delete buttons and validation displays
⏸️ **Phase 8 PAUSED**: Awaiting game engine architecture review before implementing asset path fields in domain types

**Recommended Timing for Game Engine Work:**

Before proceeding with **Phase 8** (Asset System Enhancements), the campaign builder has reached a functionally complete state with:

✅ All content authoring tools operational (items, spells, monsters, quests, dialogues)
✅ Map editor with tile painting, event placement, and metadata editing
✅ Asset management system
✅ Full CRUD backend operations for all campaign data (quest/dialogue editing complete)

**Why wait until after Phase 6-7?**

1. **Complete data structures**: Map editor completion reveals all data requirements for engine
2. **Asset pipeline clarity**: Understanding asset management informs 3D asset pipeline design
3. **Event system ready**: Quest/dialogue systems define event triggers engine must support
4. **Parallel work possible**: Campaign builder functional, allowing content creation during engine development
5. **Clear requirements**: All editor features expose what engine must render/execute

**Suggested Next Steps (Current State):**

- **Option A (Complete Phase 7 UI)**: Finish Phase 7 UI integration to enable full quest/dialogue editing in GUI, then review game engine before Phase 8
- **Option B (Engine Review First)**: Review and plan game engine architecture now, then decide if Phase 8 asset system enhancements are still needed or should be redesigned
- **Option C (Parallel)**: Complete Phase 7 UI integration while conducting game engine planning in parallel, defer Phase 8 decision until engine requirements are clear

**Recommended**: **Option B** - Review game engine architecture before Phase 8, as Phase 8 requires domain type modifications that may conflict with engine design decisions.

**Engine Planning Document:**

Create a separate implementation plan: `docs/explanation/game_engine_3d_enhancement_plan.md`

This should address:

- Rendering stack selection (Bevy, wgpu, rend3, custom)
- 3D tile representation approach (voxel, isometric 3D, true 3D tiles)
- Camera and movement systems (first-person, third-person, isometric)
- Asset pipeline (2D to 3D transition strategy)
- Performance targets and optimization strategy
- Backward compatibility with existing 2D map data

**Campaign Builder remains the priority** until Phases 6-7 are complete. Engine enhancements should not block content authoring tool development.

---

## Appendix: Code Patterns to Follow

### Pattern 1: Editor Toolbar

```rust
ui.horizontal(|ui| {
    ui.label("🔍 Search:");
    ui.text_edit_singleline(&mut self.search_field);
    ui.separator();
    if ui.button("➕ Add").clicked() {
        self.mode = EditorMode::Add;
    }
    if ui.button("🔄 Reload").clicked() {
        self.load_data();
    }
    ui.separator();
    ui.label(format!("Total: {}", self.items.len()));
});
```

### Pattern 2: List/Detail Split View

```rust
ui.horizontal(|ui| {
    // Left panel
    ui.vertical(|ui| {
        ui.set_width(300.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            // List items
        });
    });

    ui.separator();

    // Right panel
    ui.vertical(|ui| {
        // Details view
    });
});
```

### Pattern 3: ID Generation

```rust
fn next_available_id<T>(&self, items: &[T], id_fn: impl Fn(&T) -> u32) -> u32 {
    items.iter().map(id_fn).max().unwrap_or(0) + 1
}

// Usage:
let new_id = self.next_available_id(&self.items, |i| i.id);
```

### Pattern 4: Validation Display

```rust
if !self.validation_errors.is_empty() {
    ui.colored_label(egui::Color32::RED, "⚠️ Validation Errors:");
    for error in &self.validation_errors {
        ui.horizontal(|ui| {
            ui.label(error.severity.icon());
            ui.label(&error.message);
        });
    }
}
```

---

## Conclusion

This plan provides a systematic approach to completing the Campaign Builder GUI. By following the phased implementation, starting with critical type system fixes and progressing through each editor, the Campaign Builder will become a fully functional tool for creating Antares campaigns.

The plan adheres to:

- ✅ AGENTS.md Golden Rules (especially Type System Adherence)
- ✅ Architecture.md data structure definitions
- ✅ Existing code patterns in campaign_builder
- ✅ Quality gate requirements
- ✅ Test coverage standards

**Estimated completion: 10 weeks (one developer, full-time)**

**Risk levels:**

- **Phases 3-6: LOW** - All backend code exists, only UI integration required
- **Phase 7: MEDIUM** - Requires careful handling of cascading deletes and reference integrity
- **Phase 8: HIGH** - Modifies core domain types, requires architecture approval and migration strategy

---

_Document version: 1.0_
_Created: 2024_
_Status: Ready for implementation_
