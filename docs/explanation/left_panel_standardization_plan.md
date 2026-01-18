# Left Panel Standardization Implementation Plan

## Overview

This plan standardizes the Campaign Builder UI by introducing a reusable `StandardListItem` component to ensure consistent presentation, metadata display, and context menu actions across all editor left panel lists. The standardization covers 8 editors: Items, Monsters, Spells, Classes, Conditions, Maps, Quests, and Dialogue.

## Current State Analysis

### Existing Infrastructure

**File**: `antares/sdk/campaign_builder/src/ui_helpers.rs`

- **Lines 894-905**: `ItemAction` enum with Edit, Delete, Duplicate, Export, None variants
- **Lines 911-1089**: `ActionButtons` component with builder pattern for action buttons
- **Lines 1100-1293**: `TwoColumnLayout` component for split-panel layouts
- **Lines 613-886**: `EditorToolbar` component for consistent toolbar UI
- **No existing**: Standardized list item component for left panel entries

**Current Editor Implementations**:

- All 8 editors use ad-hoc string formatting for list labels
- Items editor (L363-550): Uses emoji appending ("✨", "💀", "📜") directly in labels
- Monsters editor (L272-450): Uses inline emoji formatting with HP display
- Spells editor (L297-450): Uses school icons and level prefix in string format
- Classes, Conditions, Maps, Quests, Dialogue: Similar ad-hoc patterns
- Context menus: Currently implemented via `ActionButtons` in right panel only

### Identified Issues

1. **Inconsistent Label Formatting**: Each editor uses different string concatenation patterns
2. **No Rich Metadata Display**: Badges/metadata are plain text or emojis, not styled components
3. **Missing Context Menus**: Right-click functionality absent on list items
4. **Code Duplication**: Selection logic and action handling repeated across 8 editors
5. **No ID Display**: Item IDs not consistently shown in metadata
6. **Accessibility**: Emoji-based indicators lack proper semantic meaning

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

### Phase 10: Documentation and Final Integration

#### 10.1 Documentation Updates

**File**: `antares/docs/explanation/implementations.md`

**Task 10.1.1**: Add implementation summary

**Content to Add**:

```markdown
## Left Panel Standardization (Campaign Builder)

**Date**: [Current Date]
**Components Modified**: 8 editors, ui_helpers.rs
**Scope**: UI standardization and context menu integration

### Overview

Standardized left panel list items across all Campaign Builder editors using a reusable `StandardListItem` component pattern.

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

### Testing

- All quality gates pass (fmt, check, clippy, test)
- Manual UI testing completed for all 8 editors
- Context menus functional on all editors
- No regressions in search/filter functionality

### Benefits

- **Consistency**: Uniform appearance across all editors
- **Maintainability**: Single source of truth for list item rendering
- **Accessibility**: Semantic badges with tooltips replace emoji indicators
- **Usability**: Context menus reduce clicks for common actions
- **Extensibility**: Easy to add new badge types or metadata fields
```

**Task 10.1.2**: Update architecture documentation (if applicable)

**File**: `antares/docs/reference/architecture.md` (if UI patterns are documented)

**Action**: Add reference to `StandardListItem` pattern in UI/SDK section

#### 10.2 Final Quality Verification

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

#### 10.3 Manual Testing Verification

**Complete Testing Matrix**:

| Editor     | List Display | Badges | ID Display | Context Menu | Search/Filter | Edit | Delete | Duplicate | Export |
| ---------- | ------------ | ------ | ---------- | ------------ | ------------- | ---- | ------ | --------- | ------ |
| Items      | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Monsters   | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Spells     | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Classes    | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Conditions | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Maps       | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Quests     | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |
| Dialogue   | [ ]          | [ ]    | [ ]        | [ ]          | [ ]           | [ ]  | [ ]    | [ ]       | [ ]    |

**Testing Instructions**:

1. Open Campaign Builder application
2. For each editor, verify each column in the matrix
3. Check box only if functionality works correctly
4. Report any failures with specific steps to reproduce

#### 10.4 Deliverables

- [ ] Implementation summary added to `docs/explanation/implementations.md`
- [ ] Architecture documentation updated (if applicable)
- [ ] All quality gates pass across entire project
- [ ] Manual testing matrix 100% complete
- [ ] No regressions in existing functionality
- [ ] All 8 editors standardized and functional

#### 10.5 Success Criteria

- [ ] All code compiles without errors
- [ ] Zero clippy warnings
- [ ] All tests pass (existing + new)
- [ ] Manual testing matrix shows 100% pass rate
- [ ] Documentation updated and accurate
- [ ] Campaign Builder UI visually consistent across all editors
- [ ] Context menus work on all editors
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
10. ✅ Phase 10: Documentation and Final Integration - **END HERE**

**Rationale**:

- Phase 1 provides foundation for all editors
- Phases 2-9 can be executed sequentially (each editor is independent)
- Phase 10 finalizes and documents the entire implementation

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

**Total Estimated Implementation Time**: 8-12 hours across 10 phases
**Risk Level**: Low (incremental, well-tested approach)
**Impact**: High (consistency, maintainability, usability improvements across entire Campaign Builder)
