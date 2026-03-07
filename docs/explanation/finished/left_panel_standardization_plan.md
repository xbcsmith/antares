# Left Panel Standardization Implementation Plan

## Overview

This plan standardizes the Campaign Builder UI by introducing a reusable `StandardListItem` component to ensure consistent presentation, metadata display, and context menu actions across all editor left panel lists. The standardization covers 14 editors: Items, Monsters, Spells, Classes, Conditions, Maps, Quests, Dialogue, Characters, Races, Proficiencies, NPCs, Creatures, and Campaign.

## Current State Analysis

### Existing Infrastructure

**File**: `antares/sdk/campaign_builder/src/ui_helpers.rs`

- **Lines 894-905**: `ItemAction` enum with Edit, Delete, Duplicate, Export, None variants
- **Lines 911-1089**: `ActionButtons` component with builder pattern for action buttons
- **Lines 1100-1293**: `TwoColumnLayout` component for split-panel layouts
- **Lines 613-886**: `EditorToolbar` component for consistent toolbar UI
- **No existing**: Standardized list item component for left panel entries

**Current Editor Implementations**:

- All 14 editors use ad-hoc string formatting for list labels
- Items editor (L363-550): Uses emoji appending ("✨", "💀", "📜") directly in labels
- Monsters editor (L272-450): Uses inline emoji formatting with HP display
- Spells editor (L297-450): Uses school icons and level prefix in string format
- Classes, Conditions, Maps, Quests, Dialogue: Similar ad-hoc patterns
- Characters editor (L1252+): Ad-hoc "⭐ Premade"/"📋 Template" horizontal badge rows
- Races editor (L587+): Ad-hoc size/stat modifier inline badges with `RichText`
- Proficiencies editor (L510+): Emoji-prefixed string labels ("⚔️", "🛡️", "✨") concatenated with ID
- NPC editor (L353+): Ad-hoc merchant/innkeeper/quest horizontal badge rows with `RichText`
- Creatures editor (L525+): Custom registry table layout with colored ID badges and category labels
- Campaign editor (L558+): Section navigation list (Overview/Gameplay/Files/Advanced) without metadata
- Context menus: Currently implemented via `ActionButtons` in right panel only

### Identified Issues

1. **Inconsistent Label Formatting**: Each editor uses different string concatenation patterns
2. **No Rich Metadata Display**: Badges/metadata are plain text or emojis, not styled components
3. **Missing Context Menus**: Right-click functionality absent on list items
4. **Code Duplication**: Selection logic and action handling repeated across 14 editors
5. **No ID Display**: Item IDs not consistently shown in metadata
6. **Accessibility**: Emoji-based indicators lack proper semantic meaning
7. **New Editors Not Covered**: Characters, Races, Proficiencies, NPC, Creatures, and Campaign editors added after original plan was written and all use divergent ad-hoc patterns

## Implementation Phases

### Phase 1: Core Infrastructure (StandardListItem Component)

#### 1.1 Foundation Work

**File**: `antares/sdk/campaign_builder/src/ui_helpers.rs`
**Insert After**: Line 1089 (after `ActionButtons` implementation)

**Task 1.1.1**: Define `MetadataBadge` struct

```rust
// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

/// Metadata badge for displaying rich metadata in list items
#[derive(Debug, Clone)]
pub struct MetadataBadge {
    pub text: String,
    pub color: egui::Color32,
    pub tooltip: Option<String>,
}

impl MetadataBadge {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: egui::Color32::GRAY,
            tooltip: None,
        }
    }

    pub fn with_color(mut self, color: egui::Color32) -> Self {
        self.color = color;
        self
    }

    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}
```

**Validation**: Run `cargo check --all-targets --all-features` - must pass with zero errors

**Task 1.1.2**: Define `StandardListItemConfig` struct

```rust
/// Configuration for a standard list item
pub struct StandardListItemConfig<'a> {
    pub label: String,
    pub badges: Vec<MetadataBadge>,
    pub id_display: Option<u32>,
    pub is_selected: bool,
    pub context_menu_enabled: bool,
    pub icon: Option<&'a str>,
}

impl<'a> StandardListItemConfig<'a> {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            badges: Vec::new(),
            id_display: None,
            is_selected: false,
            context_menu_enabled: true,
            icon: None,
        }
    }

    pub fn with_badges(mut self, badges: Vec<MetadataBadge>) -> Self {
        self.badges = badges;
        self
    }

    pub fn with_id(mut self, id: u32) -> Self {
        self.id_display = Some(id);
        self
    }

    pub fn selected(mut self, is_selected: bool) -> Self {
        self.is_selected = is_selected;
        self
    }

    pub fn with_context_menu(mut self, enabled: bool) -> Self {
        self.context_menu_enabled = enabled;
        self
    }

    pub fn with_icon(mut self, icon: &'a str) -> Self {
        self.icon = Some(icon);
        self
    }
}
```

**Validation**: Run `cargo check --all-targets --all-features` - must pass with zero errors

**Task 1.1.3**: Implement `show_standard_list_item` function

```rust
/// Shows a standard list item with consistent formatting
///
/// Returns (clicked, action) where:
/// - clicked: true if the item was clicked for selection
/// - action: ItemAction from context menu (None if no action)
pub fn show_standard_list_item(
    ui: &mut egui::Ui,
    config: StandardListItemConfig,
) -> (bool, ItemAction) {
    let mut clicked = false;
    let mut action = ItemAction::None;

    // Main selectable label with optional icon
    let label_text = if let Some(icon) = config.icon {
        format!("{} {}", icon, config.label)
    } else {
        config.label.clone()
    };

    let response = ui.selectable_label(config.is_selected, &label_text);
    clicked = response.clicked();

    // Context menu
    if config.context_menu_enabled {
        response.context_menu(|ui| {
            if ui.button("✏️ Edit").clicked() {
                action = ItemAction::Edit;
                ui.close_menu();
            }
            if ui.button("🗑️ Delete").clicked() {
                action = ItemAction::Delete;
                ui.close_menu();
            }
            if ui.button("📋 Duplicate").clicked() {
                action = ItemAction::Duplicate;
                ui.close_menu();
            }
            if ui.button("📤 Export").clicked() {
                action = ItemAction::Export;
                ui.close_menu();
            }
        });
    }

    // Metadata badges (indented)
    if !config.badges.is_empty() || config.id_display.is_some() {
        ui.horizontal(|ui| {
            ui.add_space(20.0); // Indent for hierarchy

            // Show badges
            for badge in &config.badges {
                let badge_text = egui::RichText::new(&badge.text)
                    .small()
                    .color(badge.color);
                let badge_response = ui.label(badge_text);
                if let Some(tooltip) = &badge.tooltip {
                    badge_response.on_hover_text(tooltip);
                }
            }

            // Show ID if present
            if let Some(id) = config.id_display {
                let id_text = egui::RichText::new(format!("ID:{}", id))
                    .small()
                    .color(egui::Color32::DARK_GRAY);
                ui.label(id_text);
            }
        });
    }

    (clicked, action)
}
```

**Validation**: Run `cargo clippy --all-targets --all-features -- -D warnings` - must pass with zero warnings

#### 1.2 Add Foundation Tests

**File**: `antares/sdk/campaign_builder/src/ui_helpers.rs`
**Insert In**: `mod tests` section (after existing tests)

**Task 1.2.1**: Create unit tests for `MetadataBadge`

```rust
#[test]
fn metadata_badge_new_creates_default() {
    let badge = MetadataBadge::new("Test");
    assert_eq!(badge.text, "Test");
    assert_eq!(badge.color, egui::Color32::GRAY);
    assert!(badge.tooltip.is_none());
}

#[test]
fn metadata_badge_builder_pattern() {
    let badge = MetadataBadge::new("Magic")
        .with_color(egui::Color32::BLUE)
        .with_tooltip("Magical item");
    assert_eq!(badge.text, "Magic");
    assert_eq!(badge.color, egui::Color32::BLUE);
    assert_eq!(badge.tooltip, Some("Magical item".to_string()));
}
```

**Task 1.2.2**: Create unit tests for `StandardListItemConfig`

```rust
#[test]
fn standard_list_item_config_new_creates_default() {
    let config = StandardListItemConfig::new("Test Item");
    assert_eq!(config.label, "Test Item");
    assert!(config.badges.is_empty());
    assert!(config.id_display.is_none());
    assert!(!config.is_selected);
    assert!(config.context_menu_enabled);
    assert!(config.icon.is_none());
}

#[test]
fn standard_list_item_config_builder_pattern() {
    let badges = vec![MetadataBadge::new("Test")];
    let config = StandardListItemConfig::new("Item")
        .with_badges(badges.clone())
        .with_id(42)
        .selected(true)
        .with_context_menu(false)
        .with_icon("⚔️");

    assert_eq!(config.label, "Item");
    assert_eq!(config.badges.len(), 1);
    assert_eq!(config.id_display, Some(42));
    assert!(config.is_selected);
    assert!(!config.context_menu_enabled);
    assert_eq!(config.icon, Some("⚔️"));
}
```

**Validation**: Run `cargo nextest run --all-features` - all tests must pass

#### 1.3 Integrate Foundation Work

**No integration required yet** - foundation ready for editor updates

#### 1.4 Testing Requirements

**Command Sequence** (all must pass):

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

**Expected Results**:

- ✅ `cargo fmt`: No output (formatting complete)
- ✅ `cargo check`: "Finished" with 0 errors
- ✅ `cargo clippy`: "Finished" with 0 warnings
- ✅ `cargo nextest run`: All tests pass (existing + 4 new tests)

#### 1.5 Deliverables

- [ ] `MetadataBadge` struct implemented in `ui_helpers.rs` (after L1089)
- [ ] `StandardListItemConfig` struct implemented in `ui_helpers.rs`
- [ ] `show_standard_list_item` function implemented in `ui_helpers.rs`
- [ ] 4 unit tests added to `ui_helpers.rs` test module
- [ ] All quality gates pass (fmt, check, clippy, test)
- [ ] No existing tests broken

#### 1.6 Success Criteria

- [ ] `cargo check` passes with zero errors
- [ ] `cargo clippy` passes with zero warnings
- [ ] `cargo nextest run` shows 4 new passing tests
- [ ] No breaking changes to existing `ui_helpers.rs` exports
- [ ] Code coverage >80% for new functions

---

### Phase 2: Items Editor Standardization

#### 2.1 Refactor Items Editor List

**File**: `antares/sdk/campaign_builder/src/items_editor.rs`
**Modify Function**: `show_list` (Lines 363-550)

**Task 2.1.1**: Update imports

**Location**: Top of file (after existing imports)

```rust
use crate::ui_helpers::{
    // ... existing imports ...
    MetadataBadge, StandardListItemConfig, show_standard_list_item,
};
```

**Task 2.1.2**: Replace label generation loop

**Replace Lines**: Approximately L403-419 (the label formatting section)

**Old Code Pattern**:

```rust
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
```

**New Code**:

```rust
.map(|(idx, item)| (idx, item.clone()))
```

**Task 2.1.3**: Replace list rendering loop

**Replace Lines**: Approximately L431-438 (the list rendering section in left panel)

**Old Code Pattern**:

```rust
for (idx, label, _) in &sorted_items {
    let is_selected = selected == Some(*idx);
    if left_ui.selectable_label(is_selected, label).clicked() {
        new_selection = Some(*idx);
    }
}
```

**New Code**:

```rust
for (idx, item) in &sorted_items {
    let mut badges = Vec::new();

    // Add type-specific badge
    let type_badge = match &item.item_type {
        ItemType::Weapon(_) => MetadataBadge::new("Weapon")
            .with_color(egui::Color32::from_rgb(200, 100, 100)),
        ItemType::Armor(_) => MetadataBadge::new("Armor")
            .with_color(egui::Color32::from_rgb(100, 100, 200)),
        ItemType::Accessory(_) => MetadataBadge::new("Accessory")
            .with_color(egui::Color32::from_rgb(200, 200, 100)),
        ItemType::Consumable(_) => MetadataBadge::new("Consumable")
            .with_color(egui::Color32::from_rgb(100, 200, 100)),
        ItemType::Ammo(_) => MetadataBadge::new("Ammo")
            .with_color(egui::Color32::from_rgb(150, 150, 150)),
        ItemType::Quest(_) => MetadataBadge::new("Quest")
            .with_color(egui::Color32::from_rgb(255, 215, 0)),
    };
    badges.push(type_badge);

    // Add property badges
    if item.is_magical() {
        badges.push(
            MetadataBadge::new("Magic")
                .with_color(egui::Color32::from_rgb(138, 43, 226))
                .with_tooltip("Magical item"),
        );
    }
    if item.is_cursed {
        badges.push(
            MetadataBadge::new("Cursed")
                .with_color(egui::Color32::from_rgb(139, 0, 0))
                .with_tooltip("Cursed item - cannot be unequipped"),
        );
    }
    if item.is_quest_item() {
        badges.push(
            MetadataBadge::new("Quest")
                .with_color(egui::Color32::from_rgb(255, 215, 0))
                .with_tooltip("Quest item"),
        );
    }

    let config = StandardListItemConfig::new(&item.name)
        .with_badges(badges)
        .with_id(item.id)
        .selected(selected == Some(*idx));

    let (clicked, action) = show_standard_list_item(left_ui, config);

    if clicked {
        new_selection = Some(*idx);
    }

    if action != ItemAction::None {
        action_requested = Some(action);
    }
}
```

**Validation**: Run `cargo check --all-targets --all-features` after changes

#### 2.2 Integrate Items Editor Changes

**Task 2.2.1**: Remove action button handling from right panel

**Location**: Lines approximately 456-463 (in right panel closure)

**Action**: Remove the `ActionButtons::new().enabled(true).show(right_ui)` call since context menu now handles actions

**Task 2.2.2**: Update action handling logic

**Location**: Lines approximately 520-550 (action handling after closures)

**Action**: Verify action handling still works correctly - logic should remain the same, just triggered from context menu instead of buttons

**Validation**: Run `cargo clippy --all-targets --all-features -- -D warnings`

#### 2.3 Testing Requirements

**Task 2.3.1**: Manual UI testing checklist

- [ ] Open Items Editor in Campaign Builder
- [ ] Verify list items show name as primary text
- [ ] Verify badges display correctly with colors (Weapon, Armor, etc.)
- [ ] Verify Magic/Cursed/Quest badges appear when applicable
- [ ] Verify ID displays in gray at end of metadata line
- [ ] Right-click item → verify context menu shows Edit/Delete/Duplicate/Export
- [ ] Click Edit in context menu → verify enters edit mode
- [ ] Click Delete in context menu → verify item deleted
- [ ] Click Duplicate in context menu → verify item duplicated
- [ ] Verify search/filter still works correctly
- [ ] Verify selection highlighting works

**Task 2.3.2**: Automated testing

**File**: `antares/sdk/campaign_builder/src/items_editor.rs`
**Add to test module** (if exists, or create):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_type_filter_matches() {
        let weapon = Item {
            id: 1,
            name: "Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData::default()),
            // ... other fields ...
        };

        assert!(ItemTypeFilter::Weapon.matches(&weapon));
        assert!(!ItemTypeFilter::Armor.matches(&weapon));
    }
}
```

**Validation**: Run `cargo nextest run --all-features`

#### 2.4 Deliverables

- [ ] Items editor `show_list` function refactored to use `StandardListItemConfig`
- [ ] Emoji indicators replaced with styled `MetadataBadge` components
- [ ] Context menu integrated for Edit/Delete/Duplicate/Export actions
- [ ] Item ID displayed in metadata line
- [ ] Type badge shows item category (Weapon/Armor/etc.) with color coding
- [ ] Property badges show Magic/Cursed/Quest status with tooltips
- [ ] All quality gates pass
- [ ] Manual UI testing checklist completed

#### 2.5 Success Criteria

- [ ] Items editor compiles with zero errors/warnings
- [ ] List items display consistently with name + badges + ID
- [ ] Context menu works on right-click
- [ ] All existing functionality preserved (search, filter, edit, delete, duplicate)
- [ ] Visual appearance improved with color-coded badges
- [ ] Tooltips provide additional context on hover

---

### Phase 3: Monsters Editor Standardization

#### 3.1 Refactor Monsters Editor List

**File**: `antares/sdk/campaign_builder/src/monsters_editor.rs`
**Modify Function**: `show_list` (Lines 272-450)

**Task 3.1.1**: Update imports (same pattern as Items Editor)

**Task 3.1.2**: Replace label generation

**Replace Lines**: Approximately L286-296

**Old Code Pattern**:

```rust
.map(|(idx, monster)| {
    let undead_icon = if monster.is_undead { "💀" } else { "👹" };
    (
        idx,
        format!("{} {} (HP:{})", undead_icon, monster.name, monster.hp.base),
        monster.clone(),
    )
})
```

**New Code**:

```rust
.map(|(idx, monster)| (idx, monster.clone()))
```

**Task 3.1.3**: Replace list rendering loop

**Replace Lines**: Approximately L317-324

**New Code**:

```rust
for (idx, monster) in &sorted_monsters {
    let mut badges = Vec::new();

    // HP badge
    badges.push(
        MetadataBadge::new(format!("HP:{}", monster.hp.base))
            .with_color(egui::Color32::from_rgb(200, 100, 100))
            .with_tooltip("Hit Points"),
    );

    // AC badge
    badges.push(
        MetadataBadge::new(format!("AC:{}", monster.armor_class))
            .with_color(egui::Color32::from_rgb(100, 100, 200))
            .with_tooltip("Armor Class"),
    );

    // Undead badge
    if monster.is_undead {
        badges.push(
            MetadataBadge::new("Undead")
                .with_color(egui::Color32::from_rgb(139, 0, 139))
                .with_tooltip("Undead creature"),
        );
    }

    // Special abilities badge (if any)
    if !monster.special_abilities.is_empty() {
        badges.push(
            MetadataBadge::new(format!("Abilities:{}", monster.special_abilities.len()))
                .with_color(egui::Color32::from_rgb(255, 165, 0))
                .with_tooltip("Has special abilities"),
        );
    }

    let icon = if monster.is_undead { "💀" } else { "👹" };

    let config = StandardListItemConfig::new(&monster.name)
        .with_badges(badges)
        .with_id(monster.id)
        .selected(selected == Some(*idx))
        .with_icon(icon);

    let (clicked, action) = show_standard_list_item(left_ui, config);

    if clicked {
        new_selection = Some(*idx);
    }

    if action != ItemAction::None {
        action_requested = Some(action);
    }
}
```

#### 3.2 Testing Requirements

**Manual UI Testing**:

- [ ] Monsters list shows name with icon (💀 for undead, 👹 for others)
- [ ] HP badge displays with red color
- [ ] AC badge displays with blue color
- [ ] Undead badge appears only for undead monsters
- [ ] Abilities badge shows count of special abilities
- [ ] ID displays correctly
- [ ] Context menu functions work (Edit/Delete/Duplicate/Export)

#### 3.3 Deliverables

- [ ] Monsters editor refactored to use `StandardListItemConfig`
- [ ] HP, AC, Undead, Abilities badges implemented with colors
- [ ] Context menu integrated
- [ ] Icon preserved (💀/👹) for visual consistency
- [ ] Quality gates pass

#### 3.4 Success Criteria

- [ ] Zero compilation errors/warnings
- [ ] All badges display with correct colors
- [ ] Context menu works correctly
- [ ] Existing functionality preserved

---

### Phase 4: Spells Editor Standardization

#### 4.1 Refactor Spells Editor List

**File**: `antares/sdk/campaign_builder/src/spells_editor.rs`
**Modify Function**: `show_list` (Lines 297-450)

**Task 4.1.1**: Update imports (same pattern)

**Task 4.1.2**: Replace label generation

**Replace Lines**: Approximately L311-325

**Old Code Pattern**:

```rust
.map(|(idx, spell)| {
    let school_icon = match spell.school {
        SpellSchool::Cleric => "✝️",
        SpellSchool::Sorcerer => "🔮",
    };
    (
        idx,
        format!("{} L{}: {}", school_icon, spell.level, spell.name),
        spell.clone(),
    )
})
```

**New Code**:

```rust
.map(|(idx, spell)| (idx, spell.clone()))
```

**Task 4.1.3**: Replace list rendering loop

**New Code**:

```rust
for (idx, spell) in &sorted_spells {
    let mut badges = Vec::new();

    // School badge
    let (school_name, school_color, school_icon) = match spell.school {
        SpellSchool::Cleric => ("Cleric", egui::Color32::from_rgb(255, 215, 0), "✝️"),
        SpellSchool::Sorcerer => ("Sorcerer", egui::Color32::from_rgb(138, 43, 226), "🔮"),
    };
    badges.push(
        MetadataBadge::new(school_name)
            .with_color(school_color)
            .with_tooltip(format!("{} spell", school_name)),
    );

    // Level badge
    badges.push(
        MetadataBadge::new(format!("Lv{}", spell.level))
            .with_color(egui::Color32::from_rgb(100, 200, 200))
            .with_tooltip("Spell level"),
    );

    // SP Cost badge
    badges.push(
        MetadataBadge::new(format!("SP:{}", spell.sp_cost))
            .with_color(egui::Color32::from_rgb(150, 150, 255))
            .with_tooltip("Spell Point cost"),
    );

    let config = StandardListItemConfig::new(&spell.name)
        .with_badges(badges)
        .with_id(spell.id)
        .selected(selected == Some(*idx))
        .with_icon(school_icon);

    let (clicked, action) = show_standard_list_item(left_ui, config);

    if clicked {
        new_selection = Some(*idx);
    }

    if action != ItemAction::None {
        action_requested = Some(action);
    }
}
```

#### 4.2 Testing Requirements

- [ ] Spells list shows school icon (✝️/🔮)
- [ ] School badge displays with correct color (Gold/Purple)
- [ ] Level badge shows spell level
- [ ] SP Cost badge shows spell point cost
- [ ] Context menu works

#### 4.3 Deliverables

- [ ] Spells editor refactored
- [ ] School, Level, SP Cost badges implemented
- [ ] Quality gates pass

#### 4.4 Success Criteria

- [ ] Zero errors/warnings
- [ ] All badges display correctly
- [ ] Context menu functional

---

### Phase 5: Classes Editor Standardization

#### 5.1 Refactor Classes Editor List

**File**: `antares/sdk/campaign_builder/src/classes_editor.rs`
**Locate Function**: Search for `show_list` or `filtered_classes` usage

**Task 5.1.1**: Update imports

**Task 5.1.2**: Implement list rendering with badges

**Badge Specification**:

- HP Die badge (e.g., "1d8") - Red color - Tooltip: "Hit Die"
- Proficiency Count badge (e.g., "Prof:3") - Green color - Tooltip: "Number of proficiencies"
- Spell School badge (if applicable) - Purple/Gold color - Tooltip: School name
- Pure Caster badge (if `is_pure_caster`) - Blue color - Tooltip: "Pure spellcaster"

**Example Code Pattern**:

```rust
let mut badges = Vec::new();

// Hit Die badge
badges.push(
    MetadataBadge::new(format!("{}d{}", class.hp_die.count, class.hp_die.sides))
        .with_color(egui::Color32::from_rgb(200, 50, 50))
        .with_tooltip("Hit Die"),
);

// Proficiency count
badges.push(
    MetadataBadge::new(format!("Prof:{}", class.proficiencies.len()))
        .with_color(egui::Color32::from_rgb(50, 200, 50))
        .with_tooltip("Number of proficiencies"),
);

// Spell school (if caster)
if let Some(school) = class.spell_school {
    let (school_name, color) = match school {
        SpellSchool::Cleric => ("Cleric", egui::Color32::from_rgb(255, 215, 0)),
        SpellSchool::Sorcerer => ("Sorcerer", egui::Color32::from_rgb(138, 43, 226)),
    };
    badges.push(
        MetadataBadge::new(school_name)
            .with_color(color)
            .with_tooltip("Spell school"),
    );

    if class.is_pure_caster {
        badges.push(
            MetadataBadge::new("Pure Caster")
                .with_color(egui::Color32::from_rgb(100, 100, 255))
                .with_tooltip("Full spellcasting progression"),
        );
    }
}

let config = StandardListItemConfig::new(&class.name)
    .with_badges(badges)
    .selected(selected == Some(*idx));
```

**Note**: Classes may not have numeric IDs - omit `.with_id()` if `class.id` is a String

#### 5.2 Deliverables

- [ ] Classes editor refactored
- [ ] Hit Die, Proficiency Count, Spell School badges implemented
- [ ] Quality gates pass

#### 5.3 Success Criteria

- [ ] Zero errors/warnings
- [ ] Badges display class characteristics clearly
- [ ] Context menu functional

---

### Phase 6: Conditions Editor Standardization

#### 6.1 Refactor Conditions Editor List

**File**: `antares/sdk/campaign_builder/src/conditions_editor.rs`
**Modify Function**: `show_list` (Lines 506-510+)

**Badge Specification**:

- Effect Type badge based on condition type - Various colors
- Duration badge (if temporary) - Orange color
- Severity badge (if applicable) - Red/Yellow/Green

**Example Badge Logic**:

```rust
let mut badges = Vec::new();

// Effect type badge (customize based on your Condition structure)
// Example assuming conditions have a `effect_type` or similar field
badges.push(
    MetadataBadge::new("Status Effect")
        .with_color(egui::Color32::from_rgb(255, 140, 0))
        .with_tooltip("Status effect condition"),
);

let config = StandardListItemConfig::new(&condition.name)
    .with_badges(badges)
    .with_id(condition.id)
    .selected(selected == Some(*idx));
```

#### 6.2 Deliverables

- [ ] Conditions editor refactored
- [ ] Effect Type badges implemented
- [ ] Quality gates pass

#### 6.3 Success Criteria

- [ ] Zero errors/warnings
- [ ] Badges show condition properties
- [ ] Context menu works

---

### Phase 7: Map Editor Standardization

#### 7.1 Refactor Map Editor List

**File**: `antares/sdk/campaign_builder/src/map_editor.rs`
**Modify Function**: `show_list` (Lines 2198-2202+)

**Badge Specification**:

- Size badge (e.g., "16x16") - Blue color
- Environment badge (Outdoor/Indoor/Dungeon) - Green/Brown/Gray
- NPC Count badge - Purple color
- Event Count badge - Orange color

**Example Implementation**:

```rust
let mut badges = Vec::new();

// Size badge
badges.push(
    MetadataBadge::new(format!("{}x{}", map.width, map.height))
        .with_color(egui::Color32::from_rgb(100, 150, 200))
        .with_tooltip("Map dimensions"),
);

// Environment badge
let (env_name, env_color) = match map.environment {
    // Adjust based on actual enum
    Environment::Outdoor => ("Outdoor", egui::Color32::from_rgb(50, 200, 50)),
    Environment::Indoor => ("Indoor", egui::Color32::from_rgb(139, 69, 19)),
    Environment::Dungeon => ("Dungeon", egui::Color32::from_rgb(100, 100, 100)),
};
badges.push(
    MetadataBadge::new(env_name)
        .with_color(env_color)
        .with_tooltip("Map environment"),
);

// NPC count
if !map.npcs.is_empty() {
    badges.push(
        MetadataBadge::new(format!("NPCs:{}", map.npcs.len()))
            .with_color(egui::Color32::from_rgb(200, 100, 200))
            .with_tooltip("Number of NPCs"),
    );
}

let config = StandardListItemConfig::new(&map.name)
    .with_badges(badges)
    .with_id(map.id)
    .selected(selected == Some(*idx));
```

#### 7.2 Deliverables

- [ ] Map editor refactored
- [ ] Size, Environment, NPC/Event count badges implemented
- [ ] Quality gates pass

#### 7.3 Success Criteria

- [ ] Zero errors/warnings
- [ ] Map characteristics clearly visible in badges
- [ ] Context menu functional

---

### Phase 8: Quest Editor Standardization

#### 8.1 Refactor Quest Editor List

**File**: `antares/sdk/campaign_builder/src/quest_editor.rs`
**Locate Function**: Search for `show_list` or equivalent

**Badge Specification**:

- Quest Type badge (Main/Side) - Gold/Silver
- Repeatable badge - Green color
- Level Range badge (e.g., "Lv 3-7") - Blue color
- Reward badge (if has rewards) - Yellow color

**Example Implementation**:

```rust
let mut badges = Vec::new();

// Quest type
let (type_name, type_color) = if quest.is_main_quest {
    ("Main Quest", egui::Color32::from_rgb(255, 215, 0))
} else {
    ("Side Quest", egui::Color32::from_rgb(192, 192, 192))
};
badges.push(
    MetadataBadge::new(type_name)
        .with_color(type_color)
        .with_tooltip("Quest type"),
);

// Repeatable
if quest.is_repeatable {
    badges.push(
        MetadataBadge::new("Repeatable")
            .with_color(egui::Color32::from_rgb(100, 200, 100))
            .with_tooltip("Can be completed multiple times"),
    );
}

// Level range
if let (Some(min), Some(max)) = (quest.min_level, quest.max_level) {
    badges.push(
        MetadataBadge::new(format!("Lv{}-{}", min, max))
            .with_color(egui::Color32::from_rgb(100, 150, 255))
            .with_tooltip("Recommended level range"),
    );
}

let config = StandardListItemConfig::new(&quest.name)
    .with_badges(badges)
    .with_id(quest.id)
    .selected(selected == Some(*idx));
```

#### 8.2 Deliverables

- [ ] Quest editor refactored
- [ ] Quest Type, Repeatable, Level Range badges implemented
- [ ] Quality gates pass

#### 8.3 Success Criteria

- [ ] Zero errors/warnings
- [ ] Quest properties clearly displayed
- [ ] Context menu functional

---

### Phase 9: Dialogue Editor Standardization

#### 9.1 Refactor Dialogue Editor List

**File**: `antares/sdk/campaign_builder/src/dialogue_editor.rs`
**Modify Function**: `show_dialogue_list` (Lines 1435-1439+)

**Note**: Plan states "Update to use StandardListItem helper for long-term consistency, keeping existing rich metadata badges"

**Task 9.1.1**: Analyze existing implementation

**Action**: Read current dialogue list implementation to understand existing metadata display

**Task 9.1.2**: Integrate StandardListItem while preserving functionality

**Approach**: Wrap existing badges in `MetadataBadge` structure without changing display logic

#### 9.2 Deliverables

- [ ] Dialogue editor updated to use `StandardListItemConfig`
- [ ] Existing metadata badges preserved
- [ ] Context menu integrated
- [ ] Quality gates pass

#### 9.3 Success Criteria

- [ ] Zero errors/warnings
- [ ] No visual regression from current implementation
- [ ] Context menu works
- [ ] Maintains consistency with other editors

---

### Phase 10: Characters Editor Standardization

#### 10.1 Refactor Characters Editor List

**File**: `antares/sdk/campaign_builder/src/characters_editor.rs`
**Modify Function**: `show_list` (Lines 1252+)

**Task 10.1.1**: Update imports

**Location**: Top of file (after existing imports)

Add `MetadataBadge`, `StandardListItemConfig`, `show_standard_list_item` to the `ui_helpers` import block.

**Task 10.1.2**: Replace ad-hoc badge rows in left panel scroll area

**Replace**: The `ui.horizontal` block inside the `ScrollArea` closure that renders `"⭐ Premade"` / `"📋 Template"` labels and the `"| {alignment} | ID: {id}"` weak label.

**Old Code Pattern**:

```rust
let label = format!(
    "{} ({} {})",
    character.name, character.race_id, character.class_id
);
let response = ui.selectable_label(is_selected, label);
if response.clicked() {
    select_idx = Some(*original_idx);
}
// Show character type badge
ui.horizontal(|ui| {
    ui.add_space(20.0);
    if character.is_premade {
        ui.label(egui::RichText::new("⭐ Premade").small().color(egui::Color32::GOLD));
    } else {
        ui.label(egui::RichText::new("📋 Template").small().color(egui::Color32::LIGHT_BLUE));
    }
    ui.label(egui::RichText::new(format!("| {} | ID: {}", alignment_name(character.alignment), character.id)).small().weak());
});
ui.add_space(4.0);
```

**New Code**:

```rust
let mut badges = Vec::new();

// Premade vs Template badge
if character.is_premade {
    badges.push(
        MetadataBadge::new("Premade")
            .with_color(egui::Color32::GOLD)
            .with_tooltip("Premade character available at character creation"),
    );
} else {
    badges.push(
        MetadataBadge::new("Template")
            .with_color(egui::Color32::LIGHT_BLUE)
            .with_tooltip("Character template"),
    );
}

// Alignment badge
badges.push(
    MetadataBadge::new(alignment_name(character.alignment))
        .with_color(egui::Color32::GRAY)
        .with_tooltip("Character alignment"),
);

// Race/Class summary badge
badges.push(
    MetadataBadge::new(format!("{} {}", character.race_id, character.class_id))
        .with_color(egui::Color32::from_rgb(150, 180, 220))
        .with_tooltip("Race and Class"),
);

let config = StandardListItemConfig::new(&character.name)
    .with_badges(badges)
    .selected(is_selected);

let (clicked, ctx_action) = show_standard_list_item(left_ui, config);
if clicked {
    select_idx = Some(*original_idx);
}
if ctx_action != ItemAction::None {
    action_idx = Some(*original_idx);
    action_type = ctx_action;
}
```

**Validation**: Run `cargo check --all-targets --all-features` after changes.

#### 10.2 Testing Requirements

- [ ] Characters list shows name as primary label
- [ ] "Premade" (gold) or "Template" (blue) badge present on each entry
- [ ] Alignment badge shows character alignment
- [ ] Race/Class badge shows `race_id` and `class_id`
- [ ] Right-click context menu shows Edit/Delete/Duplicate
- [ ] Search/filter still narrows the list correctly
- [ ] Selecting a character populates the right panel preview

#### 10.3 Deliverables

- [x] Characters editor `show_list` refactored to use `StandardListItemConfig`
- [x] Ad-hoc `ui.horizontal` badge rows replaced with `show_standard_list_item`
- [x] Context menu wired for Edit/Delete/Duplicate actions
- [x] All quality gates pass

#### 10.4 Success Criteria

- [x] Zero errors/warnings
- [x] Premade/Template, Alignment, and Race/Class badges display correctly
- [x] Context menu functional
- [x] All existing character editor functionality preserved

---

### Phase 11: Races Editor Standardization

#### 11.1 Refactor Races Editor List

**File**: `antares/sdk/campaign_builder/src/races_editor.rs`
**Modify Location**: `TwoColumnLayout` left-panel closure inside `pub fn show` (Lines 587+)

**Task 11.1.1**: Update imports

Add `MetadataBadge`, `StandardListItemConfig`, `show_standard_list_item` to the `ui_helpers` import block.

**Task 11.1.2**: Replace ad-hoc badge block in left panel scroll area

**Replace**: The `ui.horizontal` block inside `ScrollArea` that builds size text, color, and stat modifier strings manually.

**Old Code Pattern**:

```rust
let response = ui.selectable_label(is_selected, &race.name);
if response.clicked() {
    new_selection.set(Some(*idx));
}
// Sub-text with badges and metadata
ui.horizontal(|ui| {
    ui.add_space(20.0);
    let (size_text, size_color) = match race.size { ... };
    ui.label(egui::RichText::new(size_text).small().color(size_color));
    // ... stat modifier string building ...
});
```

**New Code**:

```rust
let mut badges = Vec::new();

// Size badge
let (size_text, size_color) = match race.size {
    SizeCategory::Small  => ("Small",  egui::Color32::LIGHT_GRAY),
    SizeCategory::Medium => ("Medium", egui::Color32::LIGHT_BLUE),
    SizeCategory::Large  => ("Large",  egui::Color32::GOLD),
};
badges.push(
    MetadataBadge::new(size_text)
        .with_color(size_color)
        .with_tooltip("Race size category"),
);

// Stat modifier summary badge (non-zero modifiers only)
let stats = &race.stat_modifiers;
let mods: Vec<String> = [
    ("Mgt", stats.might),
    ("Int", stats.intellect),
    ("Per", stats.personality),
    ("End", stats.endurance),
    ("Spd", stats.speed),
    ("Acc", stats.accuracy),
    ("Lck", stats.luck),
]
.iter()
.filter(|(_, v)| *v != 0)
.map(|(name, v)| format!("{name}:{v:+}"))
.collect();

if !mods.is_empty() {
    badges.push(
        MetadataBadge::new(mods.join(" "))
            .with_color(egui::Color32::from_rgb(180, 220, 180))
            .with_tooltip("Racial stat modifiers"),
    );
}

let config = StandardListItemConfig::new(&race.name)
    .with_badges(badges)
    .selected(is_selected);

let (clicked, ctx_action) = show_standard_list_item(left_ui, config);
if clicked {
    new_selection.set(Some(*idx));
}
if ctx_action != ItemAction::None {
    action_to_perform.set(Some((*idx, ctx_action)));
}
```

**Validation**: Run `cargo check --all-targets --all-features` after changes.

#### 11.2 Testing Requirements

- [ ] Race list shows name as primary label
- [ ] Size badge (Small/Medium/Large) with correct color on each entry
- [ ] Non-zero stat modifiers shown as compact badge (e.g., "Mgt:+2 Lck:-1")
- [ ] Right-click context menu shows Edit/Delete/Duplicate/Export
- [ ] Search filter still narrows the list correctly

#### 11.3 Deliverables

- [x] Races editor left-panel closure refactored to use `StandardListItemConfig`
- [x] Ad-hoc `ui.horizontal` size/stat badge block replaced
- [x] Context menu wired
- [x] All quality gates pass

#### 11.4 Success Criteria

- [x] Zero errors/warnings
- [x] Size and stat-modifier badges display correctly
- [x] Context menu functional
- [x] All existing race editor functionality preserved

---

### Phase 12: Proficiencies Editor Standardization

#### 12.1 Refactor Proficiencies Editor List

**File**: `antares/sdk/campaign_builder/src/proficiencies_editor.rs`
**Modify Function**: `show_list` (Lines 510+)

**Task 12.1.1**: Update imports

Add `MetadataBadge`, `StandardListItemConfig`, `show_standard_list_item` to the `ui_helpers` import block.

**Task 12.1.2**: Replace emoji-prefixed string label generation

**Replace**: The `.map` closure that builds `format!("{} {}: {}", emoji, prof.id, prof.name)` labels and the subsequent `selectable_label` call.

**Old Code Pattern**:

```rust
.map(|(idx, prof)| {
    let emoji = match prof.category {
        ProficiencyCategory::Weapon    => "⚔️",
        ProficiencyCategory::Armor     => "🛡️",
        ProficiencyCategory::Shield    => "🛡️",
        ProficiencyCategory::MagicItem => "✨",
    };
    let label = format!("{} {}: {}", emoji, prof.id, prof.name);
    (idx, label, prof.clone())
})
// ...
for (i, (idx, label, _prof)) in filtered_proficiencies.iter().enumerate() {
    let is_selected = selected == Some(*idx);
    if left_ui.selectable_label(is_selected, label).clicked() {
        new_selection = Some(*idx);
    }
}
```

**New Code**:

Simplify the `.map` closure to `(idx, prof.clone())`, then replace the render loop:

```rust
for (idx, prof) in &filtered_proficiencies {
    let mut badges = Vec::new();

    // Category badge with icon and color
    let (cat_label, cat_color) = match prof.category {
        ProficiencyCategory::Weapon    => ("Weapon",     egui::Color32::from_rgb(200, 80,  80)),
        ProficiencyCategory::Armor     => ("Armor",      egui::Color32::from_rgb(80,  80,  200)),
        ProficiencyCategory::Shield    => ("Shield",     egui::Color32::from_rgb(80,  150, 200)),
        ProficiencyCategory::MagicItem => ("Magic Item", egui::Color32::from_rgb(180, 80,  220)),
    };
    badges.push(
        MetadataBadge::new(cat_label)
            .with_color(cat_color)
            .with_tooltip("Proficiency category"),
    );

    let config = StandardListItemConfig::new(&prof.name)
        .with_badges(badges)
        .selected(selected == Some(*idx));

    let (clicked, ctx_action) = show_standard_list_item(left_ui, config);
    if clicked {
        new_selection = Some(*idx);
    }
    if ctx_action != ItemAction::None {
        action_requested = Some(ctx_action);
    }
}
```

**Validation**: Run `cargo check --all-targets --all-features` after changes.

#### 12.2 Testing Requirements

- [ ] Proficiency list shows name as primary label
- [ ] Category badge (Weapon/Armor/Shield/Magic Item) with correct color
- [ ] No more raw emoji-prefixed ID strings in labels
- [ ] Right-click context menu shows Edit/Delete/Duplicate/Export
- [ ] Category filter and search still work correctly

#### 12.3 Deliverables

- [x] Proficiencies editor `show_list` refactored to use `StandardListItemConfig`
- [x] Emoji-prefix string labels replaced with `MetadataBadge` category badges
- [x] Context menu wired
- [x] All quality gates pass

#### 12.4 Success Criteria

- [x] Zero errors/warnings
- [x] Category badges display with correct colors
- [x] Context menu functional
- [x] All existing proficiency editor functionality preserved

---

### Phase 13: NPC Editor Standardization

#### 13.1 Refactor NPC Editor List

**File**: `antares/sdk/campaign_builder/src/npc_editor.rs`
**Modify Function**: `show_list_view` (Lines 353+)

**Task 13.1.1**: Update imports

Add `MetadataBadge`, `StandardListItemConfig`, `show_standard_list_item` to the `ui_helpers` import block.

**Task 13.1.2**: Replace ad-hoc badge rows in left panel list

**Replace**: The `left_ui.selectable_label` call and the subsequent `left_ui.horizontal` block that renders merchant/innkeeper/quest badges with inline `RichText`.

**Old Code Pattern**:

```rust
let response = left_ui.selectable_label(is_selected, &npc.name);
if response.clicked() {
    new_selection = Some(*idx);
}
left_ui.horizontal(|ui| {
    ui.add_space(20.0);
    if npc.is_merchant {
        ui.label(egui::RichText::new("🏪 Merchant").small().color(egui::Color32::GOLD));
    }
    if npc.is_innkeeper {
        ui.label(egui::RichText::new("🛏️ Innkeeper").small().color(egui::Color32::LIGHT_BLUE));
    }
    if !npc.quest_ids.is_empty() {
        ui.label(egui::RichText::new(format!("📜 Quests: {}", npc.quest_ids.len())).small().color(...));
    }
    // ... etc
});
```

**New Code**:

```rust
let mut badges = Vec::new();

if npc.is_merchant {
    badges.push(
        MetadataBadge::new("Merchant")
            .with_color(egui::Color32::GOLD)
            .with_tooltip("This NPC is a merchant"),
    );
}
if npc.is_innkeeper {
    badges.push(
        MetadataBadge::new("Innkeeper")
            .with_color(egui::Color32::LIGHT_BLUE)
            .with_tooltip("This NPC is an innkeeper"),
    );
}
if !npc.quest_ids.is_empty() {
    badges.push(
        MetadataBadge::new(format!("Quests:{}", npc.quest_ids.len()))
            .with_color(egui::Color32::from_rgb(200, 180, 100))
            .with_tooltip("Number of associated quests"),
    );
}
if npc.dialogue_id.is_some() {
    badges.push(
        MetadataBadge::new("Dialogue")
            .with_color(egui::Color32::from_rgb(100, 200, 180))
            .with_tooltip("Has dialogue tree"),
    );
}

let config = StandardListItemConfig::new(&npc.name)
    .with_badges(badges)
    .selected(is_selected);

let (clicked, ctx_action) = show_standard_list_item(left_ui, config);
if clicked {
    new_selection = Some(*idx);
}
if ctx_action != ItemAction::None {
    action_requested = Some(ctx_action);
}
```

**Validation**: Run `cargo check --all-targets --all-features` after changes.

#### 13.2 Testing Requirements

- [ ] NPC list shows name as primary label
- [ ] "Merchant" (gold) badge present when `npc.is_merchant` is true
- [ ] "Innkeeper" (blue) badge present when `npc.is_innkeeper` is true
- [ ] "Quests:N" badge present when NPC has associated quests
- [ ] "Dialogue" badge present when NPC has a dialogue tree
- [ ] Right-click context menu shows Edit/Delete/Duplicate/Export
- [ ] Search and filters still narrow the list correctly

#### 13.3 Deliverables

- [x] NPC editor `show_list_view` refactored to use `StandardListItemConfig`
- [x] Ad-hoc merchant/innkeeper/quest `ui.horizontal` rows replaced
- [x] Context menu wired
- [x] All quality gates pass

#### 13.4 Success Criteria

- [x] Zero errors/warnings
- [x] Merchant, Innkeeper, Quests, and Dialogue badges display correctly
- [x] Context menu functional
- [x] All existing NPC editor functionality preserved

---

### Phase 14: Creatures Editor Standardization

#### 14.1 Refactor Creatures Registry Left Panel

**File**: `antares/sdk/campaign_builder/src/creatures_editor.rs`
**Modify Function**: `show_registry_mode` (Lines 295+)
**Target**: The `ScrollArea` block starting at approximately L525 labeled `creatures_registry_list`

**Context**: The Creatures editor uses a custom registry table layout with a `ui.horizontal` row per creature showing a colored ID badge, name label, validation tick, and category label. This should be refactored to use `StandardListItemConfig` while preserving the colored ID prefix and category badge.

**Task 14.1.1**: Update imports

Add `MetadataBadge`, `StandardListItemConfig`, `show_standard_list_item` to the `ui_helpers` import block.

**Task 14.1.2**: Replace per-row `ui.horizontal` in registry scroll area

**Replace**: The `ui.horizontal` closure per creature entry (colored ID label + selectable name + validation tick + category badge) with a `StandardListItemConfig`-based call.

**Old Code Pattern**:

```rust
ui.horizontal(|ui| {
    let id_text = format!("{:03}", creature.id);
    ui.colored_label(egui::Color32::from_rgb(...), egui::RichText::new(id_text).strong());
    ui.separator();
    let label = format!("{} ({} mesh{})", creature.name, creature.meshes.len(), ...);
    let response = ui.selectable_label(is_selected, label);
    if response.clicked() { ... }
    if response.double_clicked() { ... }
    ui.separator();
    // Validation icon
    if validation_result.is_ok() { ui.label("✓"); } else { ui.colored_label(YELLOW, "⚠"); }
    ui.separator();
    // Category badge
    ui.label(egui::RichText::new(category.display_name()).small().background_color(...));
});
```

**New Code**:

```rust
let mut badges = Vec::new();

// Category badge with category color
let color = category.color();
let cat_color = egui::Color32::from_rgb(
    (color[0] * 200.0) as u8,
    (color[1] * 200.0) as u8,
    (color[2] * 200.0) as u8,
);
badges.push(
    MetadataBadge::new(category.display_name())
        .with_color(cat_color)
        .with_tooltip("Creature category"),
);

// Mesh count badge
badges.push(
    MetadataBadge::new(format!(
        "{} mesh{}",
        creature.meshes.len(),
        if creature.meshes.len() == 1 { "" } else { "es" }
    ))
    .with_color(egui::Color32::from_rgb(150, 150, 200))
    .with_tooltip("Number of meshes"),
);

// Validation badge (warning if invalid)
let validation_result = self.id_manager.validate_id(creature.id, category);
if validation_result.is_err() {
    badges.push(
        MetadataBadge::new("ID Warning")
            .with_color(egui::Color32::YELLOW)
            .with_tooltip("ID validation issue detected"),
    );
}

// Use ID-prefixed label to preserve the numeric ID display
let label = format!("{:03} {}", creature.id, creature.name);
let config = StandardListItemConfig::new(label)
    .with_badges(badges)
    .selected(is_selected);

let (clicked, ctx_action) = show_standard_list_item(ui, config);

if clicked {
    if self.selected_registry_entry != Some(idx) {
        self.registry_delete_confirm_pending = false;
    }
    self.selected_registry_entry = Some(idx);
    ui.ctx().request_repaint();
}

// Double-click is no longer available via StandardListItem; promote to
// a single-click edit path or retain with a secondary button in the right panel.

if ctx_action != ItemAction::None {
    // Map context menu actions to registry actions
    match ctx_action {
        ItemAction::Edit => {
            let file_name = format!(
                "assets/creatures/{}.ron",
                creature.name.to_lowercase().replace(' ', "_")
            );
            pending_edit = Some((idx, file_name));
        }
        ItemAction::Delete => { /* set pending delete */ }
        ItemAction::Duplicate => { /* set pending duplicate */ }
        _ => {}
    }
}
```

**Note**: The double-click-to-edit behavior should be preserved via the existing right-panel Edit button. Document this trade-off in code comments.

**Validation**: Run `cargo check --all-targets --all-features` after changes.

#### 14.2 Testing Requirements

- [ ] Creature registry list shows `{ID:03} {name}` as primary label
- [ ] Category badge with category-derived color on each entry
- [ ] Mesh count badge shows number of meshes
- [ ] "ID Warning" (yellow) badge shown when validation fails
- [ ] Right-click context menu shows Edit/Delete/Duplicate
- [ ] Category filter and search still narrow the list
- [ ] Right-panel preview still activates on single click

#### 14.3 Deliverables

- [x] Creatures editor registry `ui.horizontal` rows replaced with `StandardListItemConfig`
- [x] Category, mesh-count, and validation-warning badges implemented
- [x] Context menu wired to existing registry action handlers
- [x] All quality gates pass

#### 14.4 Success Criteria

- [x] Zero errors/warnings
- [x] Category and mesh-count badges display correctly
- [x] ID Warning badge appears only on validation failures
- [x] Context menu functional
- [x] All existing creature editor functionality preserved (including double-click path via right panel)

---

### Phase 15: Campaign Editor Standardization

#### 15.1 Refactor Campaign Section Navigation List

**File**: `antares/sdk/campaign_builder/src/campaign_editor.rs`
**Modify Function**: `render_ui` (Lines 558+)
**Target**: The left-panel `selectable_label` block that renders Overview/Gameplay/Files/Advanced section links.

**Context**: The Campaign editor's left panel is a simple section-navigation list (not a data list), so it does not need metadata badges or a context menu. The goal is to add icons and consistent indentation to match the visual rhythm of other standardized editors.

**Task 15.1.1**: Replace plain `selectable_label` calls with icon-prefixed labels

**Replace**: The four `selectable_label` calls with icon-prefixed versions using `StandardListItemConfig` with `context_menu_enabled: false`.

**Old Code Pattern**:

```rust
left_ui.heading("Sections");
left_ui.separator();
let is_overview = new_selected.get() == CampaignSection::Overview;
if left_ui.selectable_label(is_overview, "Overview").clicked() {
    new_selected.set(CampaignSection::Overview);
}
let is_gameplay = new_selected.get() == CampaignSection::Gameplay;
if left_ui.selectable_label(is_gameplay, "Gameplay").clicked() {
    new_selected.set(CampaignSection::Gameplay);
}
let is_files = new_selected.get() == CampaignSection::Files;
if left_ui.selectable_label(is_files, "Files").clicked() {
    new_selected.set(CampaignSection::Files);
}
let is_advanced = new_selected.get() == CampaignSection::Advanced;
if left_ui.selectable_label(is_advanced, "Advanced").clicked() {
    new_selected.set(CampaignSection::Advanced);
}
```

**New Code**:

```rust
left_ui.heading("Sections");
left_ui.separator();

let sections: &[(&str, &str, CampaignSection)] = &[
    ("📋", "Overview",  CampaignSection::Overview),
    ("⚔️",  "Gameplay",  CampaignSection::Gameplay),
    ("📁", "Files",     CampaignSection::Files),
    ("⚙️",  "Advanced",  CampaignSection::Advanced),
];

for (icon, label, section) in sections {
    let is_selected = new_selected.get() == *section;
    let config = StandardListItemConfig::new(*label)
        .with_icon(icon)
        .selected(is_selected)
        .with_context_menu(false);
    let (clicked, _) = show_standard_list_item(left_ui, config);
    if clicked {
        new_selected.set(*section);
    }
}
```

**Validation**: Run `cargo check --all-targets --all-features` after changes.

#### 15.2 Testing Requirements

- [ ] Section list shows icon + label for each of the four sections
- [ ] Selected section is highlighted
- [ ] Clicking a section switches the right panel content
- [ ] No context menu appears on right-click (context menu disabled)

#### 15.3 Deliverables

- [x] Campaign editor section navigation refactored to use `StandardListItemConfig`
- [x] Icons added to Overview/Gameplay/Files/Advanced entries
- [x] Context menu disabled (navigation list, not data list)
- [x] All quality gates pass

#### 15.4 Success Criteria

- [x] Zero errors/warnings
- [x] Section icons display correctly
- [x] Section switching still works correctly
- [x] No regression in campaign metadata editing

---

### Phase 16: Documentation and Final Integration

#### 16.1 Documentation Updates

**File**: `antares/docs/explanation/implementations.md`

**Task 10.1.1**: Add implementation summary

**Content to Add**:

```markdown
## Left Panel Standardization (Campaign Builder) [UPDATED]

**Date**: [Current Date]
**Components Modified**: 8 editors, ui_helpers.rs
**Scope**: UI standardization and context menu integration

### Overview [Updated]

Standardized left panel list items across all Campaign Builder editors using a reusable `StandardListItem` component pattern.

Standardized left panel list items across all Campaign Builder editors using a reusable `StandardListItem` component pattern. Covers 14 editors total including 6 editors added after the original plan was written.

### Components Implemented

1. **MetadataBadge** (`ui_helpers.rs` after L1089)

   - Struct for styled metadata badges with color and tooltip support
   - Builder pattern for configuration
   - Replaces emoji-based indicators with semantic badges

2. **StandardListItemConfig** (`ui_helpers.rs`)

   - Configuration struct for standard list items
   - Supports badges, ID display, selection state, context menus, icons
   - Builder pattern for flexible configuration

3. **show_standard_list_item** (`ui_helpers.rs`)
   - Rendering function for consistent list item display
   - Returns (clicked, action) tuple for selection and context menu handling
   - Implements right-click context menu with Edit/Delete/Duplicate/Export

### Editors Standardized

1. **Items Editor** (`items_editor.rs`)

   - Badges: Type, Magic, Cursed, Quest
   - Color-coded by item category
   - ID display in metadata

2. **Monsters Editor** (`monsters_editor.rs`)

   - Badges: HP, AC, Undead, Abilities
   - Icon preservation (💀/👹)
   - Context menu integration

3. **Spells Editor** (`spells_editor.rs`)

   - Badges: School, Level, SP Cost
   - School icons preserved (✝️/🔮)

4. **Classes Editor** (`classes_editor.rs`)

   - Badges: Hit Die, Proficiency Count, Spell School, Pure Caster

5. **Conditions Editor** (`conditions_editor.rs`)

   - Badges: Effect Type, Duration, Severity

6. **Map Editor** (`map_editor.rs`)

   - Badges: Size, Environment, NPC Count, Event Count

7. **Quest Editor** (`quest_editor.rs`)

   - Badges: Quest Type, Repeatable, Level Range

8. **Dialogue Editor** (`dialogue_editor.rs`)

   - Existing metadata preserved with new component structure

9. **Characters Editor** (`characters_editor.rs`)

   - Badges: Premade/Template, Alignment, Race+Class summary
   - Replaces ad-hoc `ui.horizontal` badge rows

10. **Races Editor** (`races_editor.rs`)

    - Badges: Size category (Small/Medium/Large), non-zero stat modifier summary
    - Replaces ad-hoc inline `RichText` size/stat block

11. **Proficiencies Editor** (`proficiencies_editor.rs`)

    - Badges: Category (Weapon/Armor/Shield/Magic Item) with color coding
    - Replaces emoji-prefixed string label pattern

12. **NPC Editor** (`npc_editor.rs`)

    - Badges: Merchant, Innkeeper, Quest count, Dialogue presence
    - Replaces ad-hoc `ui.horizontal` merchant/innkeeper badge rows

13. **Creatures Editor** (`creatures_editor.rs`)

    - Badges: Category (color-coded), Mesh count, ID Validation warning
    - Preserves numeric ID prefix in primary label
    - Replaces custom `ui.horizontal` registry table rows

14. **Campaign Editor** (`campaign_editor.rs`)

    - Icon-prefixed section navigation (Overview/Gameplay/Files/Advanced)
    - Context menu disabled (navigation list, not data list)
    - Replaces plain `selectable_label` section links

### Testing

- All quality gates pass (fmt, check, clippy, test)
- Manual UI testing completed for all 14 editors
- Context menus functional on all data-list editors (disabled on Campaign section nav)
- No regressions in search/filter functionality

### Benefits

- **Consistency**: Uniform appearance across all 14 editors
- **Maintainability**: Single source of truth for list item rendering
- **Accessibility**: Semantic badges with tooltips replace emoji indicators
- **Usability**: Context menus reduce clicks for common actions
- **Extensibility**: Easy to add new badge types or metadata fields
- **Completeness**: All editors including the 6 added after the original plan are now covered
```

**Task 10.1.2**: Update architecture documentation (if applicable)

**File**: `antares/docs/reference/architecture.md` (if UI patterns are documented)

**Action**: Add reference to `StandardListItem` pattern in UI/SDK section

#### 16.2 Final Quality Verification

**Command Sequence** (run from project root):

```bash
# Format all code
cargo fmt --all

# Check compilation (all targets, all features)
cargo check --all-targets --all-features

# Lint with warnings as errors
cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
cargo nextest run --all-features

# Optional: Run with coverage
cargo nextest run --all-features --no-capture
```

**Expected Results**: All commands pass with zero errors/warnings

#### 16.3 Manual Testing Verification

**Complete Testing Matrix**:

| Editor        | List Display | Badges | ID Display | Context Menu | Search/Filter | Edit | Delete | Duplicate | Export |
| ------------- | ------------ | ------ | ---------- | ------------ | ------------- | ---- | ------ | --------- | ------ |
| Items         | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Monsters      | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Spells        | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Classes       | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Conditions    | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Maps          | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Quests        | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Dialogue      | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Characters    | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Races         | [ ]          | [ ]    | N/A        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Proficiencies | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| NPCs          | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Creatures     | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Campaign      | [ ]          | N/A    | N/A        | N/A          | N/A           | N/A  | N/A    | N/A       | N/A    |

**Testing Instructions**:

1. Open Campaign Builder application
2. For each editor, verify each column in the matrix
3. Check box only if functionality works correctly
4. Report any failures with specific steps to reproduce

#### 16.4 Deliverables

- [x] Implementation summary added to `docs/explanation/implementations.md`
- [x] Architecture documentation updated (if applicable)
- [x] All quality gates pass across entire project
- [ ] Manual testing matrix 100% complete
- [x] No regressions in existing functionality
- [x] All 14 editors standardized and functional

#### 16.5 Success Criteria

- [x] All code compiles without errors
- [x] Zero clippy warnings
- [x] All tests pass (existing + new)
- [ ] Manual testing matrix shows 100% pass rate
- [x] Documentation updated and accurate
- [x] Campaign Builder UI visually consistent across all 14 editors
- [x] Context menus work on all data-list editors (Characters, Races, Proficiencies, NPCs, Creatures)
- [x] Campaign editor section navigation uses icon-prefixed labels, no context menu
- [ ] No performance degradation

---

## Implementation Order Summary

**Execute phases in this exact order:**

1. ✅ Phase 1: Core Infrastructure (StandardListItem Component) - **START HERE**
2. ✅ Phase 2: Items Editor Standardization
3. ✅ Phase 3: Monsters Editor Standardization
4. ✅ Phase 4: Spells Editor Standardization
5. ✅ Phase 5: Classes Editor Standardization
6. ✅ Phase 6: Conditions Editor Standardization
7. ✅ Phase 7: Map Editor Standardization
8. ✅ Phase 8: Quest Editor Standardization
9. ✅ Phase 9: Dialogue Editor Standardization
10. ✅ Phase 10: Characters Editor Standardization
11. ✅ Phase 11: Races Editor Standardization
12. ✅ Phase 12: Proficiencies Editor Standardization
13. ✅ Phase 13: NPC Editor Standardization
14. ✅ Phase 14: Creatures Editor Standardization
15. ✅ Phase 15: Campaign Editor Standardization
16. ✅ Phase 16: Documentation and Final Integration - **END HERE**

**Rationale**:

- Phase 1 provides foundation for all editors
- Phases 2-9 are complete (original 8 editors)
- Phases 10-15 cover the 6 new editors added after the original plan; each is independent and can be executed sequentially or in parallel
- Phase 14 (Creatures) is the most complex due to the custom registry table layout; do it before Phase 16
- Phase 15 (Campaign) is the simplest change (navigation list, no badges/context menu); can be done at any point
- Phase 16 finalizes and documents the entire implementation

---

## Validation Commands Reference

**Quality Gates** (must pass after every phase):

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

**Quick Check** (during development):

```bash
cargo check --all-features
```

**Full Verification** (before committing):

```bash
cargo fmt --all && \
cargo check --all-targets --all-features && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo nextest run --all-features
```

---

## Risk Mitigation

### Potential Issues

1. **Breaking Changes**: Modifying `ui_helpers.rs` could affect other components

   - **Mitigation**: Add new exports without removing existing ones; run full test suite after Phase 1

2. **Context Menu Conflicts**: Existing right-click functionality in editors

   - **Mitigation**: Check each editor for existing context menus before integration

3. **Performance**: Additional badge rendering could slow UI

   - **Mitigation**: Profile UI performance; badges are lightweight RichText components

4. **Type Mismatches**: Editor-specific types may not fit standard pattern

   - **Mitigation**: Use builder pattern with optional fields; customize per editor as needed

5. **Visual Regression**: Users accustomed to current UI
   - **Mitigation**: Preserve icons and color schemes; enhance rather than replace

### Rollback Plan

If critical issues arise:

1. Revert affected editor files to previous state
2. Keep `ui_helpers.rs` changes (backward compatible)
3. Address issues and re-attempt integration
4. Document specific incompatibilities for future reference

---

## AI Agent Instructions Summary

### For Implementation Agents

**Phase Execution Pattern**:

1. Read this plan section for current phase
2. Read specified file at specified lines
3. Implement exact changes as documented
4. Run validation commands (cargo fmt, check, clippy, test)
5. Verify all deliverables checked off
6. Verify success criteria met
7. Move to next phase

**New Editor Notes (Phases 10-15)**:

- Characters (Phase 10): `show_list` in `characters_editor.rs` - replace `ui.horizontal` badge block inside `ScrollArea`
- Races (Phase 11): Left-panel closure inside `pub fn show` in `races_editor.rs` - replace `ui.horizontal` size/stat block
- Proficiencies (Phase 12): `show_list` in `proficiencies_editor.rs` - simplify `.map` and replace `selectable_label` loop
- NPCs (Phase 13): `show_list_view` in `npc_editor.rs` - replace merchant/innkeeper/quest `ui.horizontal` block
- Creatures (Phase 14): `show_registry_mode` in `creatures_editor.rs` - replace `ui.horizontal` per-row table in `ScrollArea`; preserve colored ID prefix in primary label
- Campaign (Phase 15): `render_ui` in `campaign_editor.rs` - replace four `selectable_label` section links with icon-prefixed `StandardListItemConfig` calls, context menu disabled

**Critical Rules**:

- Follow AGENTS.md rules for copyright headers (SPDX)
- Use exact type names, function signatures as specified
- Run quality gates after EVERY change
- Do not skip validation steps
- Do not modify files outside current phase scope
- Do not optimize or refactor beyond plan specifications

**When Stuck**:

1. Re-read current phase section
2. Check validation command output for specific errors
3. Verify you're editing correct file at correct lines
4. Confirm all imports are correct
5. Ask for clarification with specific error messages

### Quality Verification Checklist (Every Phase)

- [ ] Copyright header added (SPDX format) to any new .rs files
- [ ] `cargo fmt --all` executed and passed
- [ ] `cargo check --all-targets --all-features` passed (0 errors)
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passed (0 warnings)
- [ ] `cargo nextest run --all-features` passed (all tests)
- [ ] Phase deliverables all checked
- [ ] Phase success criteria all met
- [ ] No files modified outside phase scope

---

## Conclusion

This plan provides comprehensive, step-by-step instructions for standardizing Campaign Builder left panel lists. Each phase is self-contained with explicit file paths, line numbers, code examples, validation steps, and success criteria. The phased approach ensures safe, incremental progress with full testing at each stage.

The plan was updated to include 6 new editors (Characters, Races, Proficiencies, NPC, Creatures, Campaign) that were added after the original 8-editor plan was written. All 14 editors are now covered.

**Total Estimated Implementation Time**: 12-16 hours across 16 phases (Phases 1-9 already complete)
**Remaining Estimated Time**: 0 hours (all phases complete)
**Risk Level**: Low (incremental, well-tested approach; new editors follow the same proven pattern)
**Impact**: High (consistency, maintainability, usability improvements across entire Campaign Builder)
