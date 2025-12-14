# SDK Campaign Builder: Autocomplete Integration Plan

## Overview
This plan outlines the integration of the `egui_autocomplete` library into the Campaign Builder SDK. The goal is to improve data entry efficiency and validity by providing context-aware suggestions for numeric ranges, string restrictions, and entity references (e.g., selecting monsters, items).

## Current State Analysis

### Existing Infrastructure
| Component | Location | Status |
|-----------|----------|--------|
| `Campaign Builder` | `sdk/campaign_builder` | ✅ Uses `egui` 0.33 |
| `UI Helpers` | `sdk/campaign_builder/src/ui_helpers.rs` | ✅ Centralized UI components exist |
| `Monsters Editor` | `sdk/campaign_builder/src/monsters_editor.rs` | ✅ Uses standard `TextEdit` / `DragValue` |
| Autocomplete | N/A | ❌ Missing |

### Identified Issues
1.  **Manual ID Entry**: Referencing other entities (e.g., Drop Item ID) requires manual lookup and typing.
2.  **Opaque Constraints**: Numeric ranges and string format rules are not visible during entry.
3.  **Inefficient Search**: Selecting from large lists often involves scrolling or separate search bars.

## Implementation Phases

### Phase 1: Core Integration & UI Helper
**Goal**: Add dependency and create a reusable Autocomplete widget in `ui_helpers.rs`.

#### 1.1 Add Dependency
**File**: `sdk/campaign_builder/Cargo.toml`
*   Add `egui_autocomplete = "X.Y"` (ensure compatibility with `egui` 0.33).
*   Run `cargo build` to verify resolution.

#### 1.2 Create Autocomplete Wrapper
**File**: `sdk/campaign_builder/src/ui_helpers.rs`
*   Define `pub struct AutocompleteInput<'a>`.
*   Implement `new(id_salt: &'a str, candidates: &'a [String])`.
*   Implement `show(&self, ui, text: &mut String)`.
*   Logic: Wrap `egui_autocomplete::AutoCompleteTextEdit`.

#### 1.3 Testing & Delivery
*   Create a simple test window in `sdk/campaign_builder/src/test_play.rs` (or similar manual test harness) to verify the widget renders and filters suggestions.

### Phase 2: Reference Autocomplete (Entities & IDs)
**Goal**: Replace manual ID/Name entry and simple dropdowns with autocomplete for relations.

#### 2.1 Monster Reference
**File**: `sdk/campaign_builder/src/monsters_editor.rs`
*   **Target**: "Summon Monster" or similar fields.
*   **Action**: Use `AutocompleteInput` to select Monsters by Name/ID.

#### 2.2 Item Reference (Classes & Characters)
**File**: `sdk/campaign_builder/src/classes_editor.rs`, `sdk/campaign_builder/src/characters_editor.rs`
*   **Target**: 
    *   `starting_items` (Vec<ItemId>)
    *   `starting_equipment` fields (Weapon, Armor, etc.)
*   **Action**: Replace current ComboBox implementation with `AutocompleteInput` sourcing from `items.ron`.
    *   *Constraint*: Must map selected Name back to `ItemId` (u8/u16).

#### 2.3 Condition Reference (Traps & Spells)
**File**: `sdk/campaign_builder/src/map_editor.rs` (Traps), `sdk/campaign_builder/src/spells_editor.rs` (Spells)
*   **Target**: 
    *   `MapEvent::Trap.effect` (Option<String>)
    *   `Spell.applied_conditions` (Vec<ConditionId>)
*   **Action**: 
    *   Load candidates from `conditions.ron`.
    *   Use `AutocompleteInput` to suggest Condition IDs (e.g., "Poison", "Sleep").

### Phase 3: Validation & Numeric Constraints
**Goal**: Ensure data validity during entry.

#### 3.1 Numeric Autocomplete & Validation
**File**: `sdk/campaign_builder/src/ui_helpers.rs`, `sdk/campaign_builder/src/map_editor.rs`
*   **Target**: Numeric fields like `Damage`.
*   **Action**:
    *   Ensure `damage` fields (e.g., in Traps) use `egui::DragValue` (numeric) processing, NOT string parsing.
    *   Optional: Provide "Presets" via autocomplete (e.g., "10", "25", "50", "100") if useful, but primarily enforce numeric types.

#### 3.2 String Restrictions & Proficiencies
**File**: `sdk/campaign_builder/src/classes_editor.rs`
*   **Target**: `proficiencies` (Vec<ProficiencyId>).
*   **Action**: Suggest valid proficiency IDs (`simple_weapon`, `heavy_armor`) from a defined source (`proficiencies.ron` or enum).

## Success Criteria
1.  **Dependency**: `egui_autocomplete` compiles and links.
2.  **Usability**: User can type partial string and see dropdown of valid options.
3.  **Integration**: At least one Editor (e.g., Monsters or Items) uses the new widget effectively.
