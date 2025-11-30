# Conditions Editor Refactor Plan

## Plan: Conditions Editor Refactor (Toolbar, Effects Editor, File I/O)

TL;DR
- Bring `sdk/campaign_builder/src/conditions_editor.rs` to parity with other editor modules (`items`, `monsters`, `spells`) by:
  - Adding consistent toolbar features (Search, Add, Import/Export, Load from File, Save),
  - Implementing full editing for `ConditionDefinition` fields (duration, icon_id, id uniqueness),
  - Implementing a complete per-effect editor for all `ConditionEffect` variants,
  - Adding file IO, validation checks, a preview, tests, and documentation.

**Top-level Steps**
1. Add toolbar and file IO (Load/Save/Import/Export).
2. Implement editing for `ConditionDefinition` core fields (`default_duration`, `icon_id`, etc.).
3. Add `effects` list UI with add/remove/reorder and effect type selection.
4. Implement robust per-effect editors (attributes, DOT/ HOT, status effects).
5. Add validation, preview, tests, and finalize docs; update `main.rs` and docs.

**Open Questions**
1. Should attribute selection be restricted to Stats attributes (e.g., might/intellect) or remain free text?
2. Icon picking: should the editor list campaign icons or accept raw `icon_id` strings?
3. How should `ConditionId` auto-generation work? slug-based `condition_<n>` or fully user-defined strings?
4. Should element strings be validated against a known list (`"fire"`, `"cold"`, `"electricity"`, `"poison"`, etc.) or free-form?
5. Should conditions support custom metadata fields (tags, categories, creator info)?
6. Should there be condition "templates" or a library system for common conditions?
7. How to handle multi-language support for name/description fields?
8. What bounds should be enforced on DiceRoll fields (count, sides, bonus)?

---

## Implementation Plan

### Overview
This plan details the phased approach to refactor and enhance the Conditions Editor in the Campaign Builder SDK. The goal is to give designers and modders consistent editing ability for `ConditionDefinition` including multi-type `ConditionEffect` editing, with the same usability and file I/O features as other editors.

### Current State Analysis

#### Existing Infrastructure
- Domain definitions in `src/domain/conditions.rs`:
  - `ConditionDefinition { id, name, description, effects, default_duration, icon_id }`.
  - `ConditionEffect` variants: `AttributeModifier`, `StatusEffect`, `DamageOverTime`, `HealOverTime`.
  - `ConditionDuration` enum: `Instant`, `Rounds`, `Minutes`, `Permanent`.
- UI patterns exist in `sdk/campaign_builder/src/items_editor.rs`, `spells_editor.rs`, and `monsters_editor.rs`.
- Shared layout helpers in `sdk/campaign_builder/src/ui_helpers.rs`.

#### Identified Issues
- The Conditions Editor currently:
  - Does not support editing `effects` (read-only list).
  - Lacks toolbar actions (Import, Load, Save, Export, Duplicate/Delete).
  - Lacks `default_duration` and `icon_id` editing.
  - Lacks per-variant `ConditionEffect` editors.
  - Not integrated into a `show()` pattern consistent with other editors that provide `ctx` and `status_message`.

---

## Implementation Phases

### Phase 0: Discovery & Preparation
**Objective:** Audit existing code, document current state, and prepare for refactoring.

1. Code Audit
   - Document all `ConditionEffect` usage across the codebase (combat, spell_effects, character stats).
   - List all existing campaign `conditions.ron` files and their structure.
   - Take screenshots of current conditions editor UI for before/after comparison.
   - Review how `ActiveCondition.magnitude` is used (runtime vs. editor).

2. Integration Analysis
   - Map out how conditions integrate with spell system (`Spell.applied_conditions`).
   - Identify any save game dependencies or existing conditions "in use".
   - Document the relationship between `ConditionDefinition` and actual gameplay.

3. Prepare Shared Components
   - Verify that dice editor from `spells_editor.rs` is reusable/portable.
   - Create element/attribute constant lists if validation is needed.
   - Identify common validation patterns from other editors.

4. Test Data Preparation
   - Create diverse example conditions for testing (DOT, HOT, status effects, attribute modifiers).
   - Prepare edge case data (empty effects, maximum values, special characters in IDs).

5. Deliverables & Success Criteria
   - Documentation of current state and dependencies.
   - Reusable component identification complete.
   - Test data created and validated.

---

### Phase 1: Core Implementation (Toolbar & File I/O)
**Objective:** Add toolbar (Search/Add/Import/Load/Save), import/export dialogs, load/save helpers, and integrate with `main.rs`.

1. Foundation Work
   - Update `sdk/campaign_builder/src/conditions_editor.rs`:
     - Add `show_import_dialog: bool`, `import_export_buffer: String` to `ConditionsEditorState`.
     - Provide a `pub fn show(...)` method that mirrors other editor `show` signatures (accepts `ui`, `ctx`, `campaign_dir`, `conditions_file`, `unsaved_changes`, `status_message`, `file_load_merge_mode`).
     - Convert the current `render_conditions_editor` into a private internal function used by the new `show`.
   - Implement `fn show_import_dialog(ctx: &egui::Context, ...)` mirroring `items_editor` import behavior.

2. Add Foundation Functionality
   - Add a top toolbar with:
     - `🔍 Search:` (existing `search_filter` moved into toolbar),
     - `➕ Add Condition` (create `default_condition` buffer),
     - `📥 Import` (open import dialog),
     - `📂 Load from File` (RON file load),
     - `💾 Save to File` (RON file save),
     - `📋 Export` (copy RON to clipboard for selected condition).
   - Implement `load_conditions` / `save_conditions` functions using RON patterns used by other editors.

3. Integrate Foundation Work
   - Update `sdk/campaign_builder/src/main.rs`:
     - Replace `conditions_editor::render_conditions_editor(...)` with `conditions_editor::ConditionsEditorState::show(...)` and pass required params (campaign_dir, file paths, unsaved changes flag, status_message).
   - Add `show_import_dialog` invocation pattern similar to `items_editor.rs`.

4. Test Requirements
   - Unit tests: load/save roundtrip of a small `conditions.ron`.
   - Manual tests: ensure Import/Export, Load/Save work and `unsaved_changes` toggles.

5. Error Handling
   - Add user-facing error messages for RON parsing failures during import/load.
   - Implement validation feedback for file format issues.
   - Add recovery options (keep existing data, retry, cancel).

6. Deliverables & Success Criteria
   - Toolbar and file I/O present and functioning; show() method integrated into main UI.
   - Save and load from campaign path match other editors' behavior.
   - Error handling provides clear feedback without data loss.

---

### Phase 2: ConditionDefinition Core Editing Fields
**Objective:** Implement editing UI for all top-level `ConditionDefinition` fields (duration, icon, description, id/name).

1. Feature Work
   - Add UI controls to edit:
     - `default_duration` with `ComboBox` or similar (Instant / Rounds(n) / Minutes(n) / Permanent).
     - `icon_id` as a text field and optionally a dropdown if image assets are integrated later.
   - Ensure ID edit validation (e.g., required, no duplicates).

2. Integrate Feature
   - Update detail pane to include these fields alongside `name` and `description`.
   - Add `ConditionsEditorState::default_condition()` helper to create a default condition template.

3. Test Requirements
   - Add tests verifying `default_duration` and `icon_id` fields persist through round-trip save/load.

4. Magnitude Consideration
   - Document that `ActiveCondition.magnitude` is runtime-only (not in editor).
   - Clarify that magnitude defaults to 1.0 when conditions are applied.

5. Deliverables & Success Criteria
   - Fields present, editable, and persistent via RON save/load.
   - Duration editor supports all variants (Instant, Rounds, Minutes, Permanent).

---

### Phase 3: Effects List & Basic Effect Editing
**Objective:** Add an effects list to the condition detail pane and allow adding/removing/reordering.

1. Feature Work
   - Add `effects` list UI in the condition detail pane with per-row actions: `✏️ Edit`, `🗑️ Delete`, `⬆️`, `⬇️`, `📋 Duplicate`.
   - Add `➕ Add Effect` control that opens a small add dialog with a `ComboBox` of effect types.
   - Create a temporary `EffectEditBuffer` structure used to edit an effect before commit.

2. Integrate & Reuse
   - Implement an in-place or modal `render_effect_list` subcomponent to manage effect lists.
   - Use `ui_helpers` and height/width constants for consistent layout.

3. Test Requirements
   - Unit tests for adding/removing/reordering effects (data-level tests).
   - Round-trip test verifying effects persisted.

4. Deliverables & Success Criteria
   - The editor can add multiple effects and the `ConditionDefinition` is updated accordingly.

---

### Phase 4: Per-Variant Effect Editor Implementation & UX Polish
**Objective:** Implement rich editors for each `ConditionEffect` variant and finalize UX.

1. Feature Work (per `ConditionEffect` variant)
   - `AttributeModifier`:
     - `ComboBox` choose the attribute (suggest using Stats names as default).
     - `DragValue` to edit the signed integer effect value.
   - `StatusEffect`:
     - `Text` input for the status tag name.
   - `DamageOverTime`:
     - Dice editor: `count`, `sides`, `bonus` (re-using `spells_editor` patterns),
     - `element` string/ComboBox.
   - `HealOverTime`:
     - Dice editor for `amount`.

2. Validation & UX
   - Enforce `ConditionId` uniqueness in the editor; show inline feedback or disable save until unique or allow user override.
   - Add effect reorder, duplicate and undo/cancel edits.
   - Add a text preview and, optionally, a sample `ActiveCondition` application preview panel.

3. Tests
   - Validation tests for invalid Dice values and duplicate IDs.
   - Editor tests for editing and saving each effect type.

4. Spell Integration
   - Add UI to view which spells reference this condition ("Used by" panel).
   - Provide warnings when deleting/modifying conditions referenced by spells.
   - Consider adding quick-link navigation to spell editor.

5. Performance Considerations
   - Implement virtual scrolling for large condition lists (100+).
   - Optimize search to handle many conditions efficiently.
   - Test with realistic campaign data volumes.

6. Deliverables & Success Criteria
   - Per-variant fields are functional and saved; no crash on edge-cases.
   - Integration with spell system is clear and functional.
   - Performance is acceptable with large datasets.

---

### Phase 5: Polishing, Tests & Documentation
**Objective:** Add tests, finalize documentation, and ensure code quality.

1. Tests
   - Integration tests for `Import / Export` flow.
   - Additional unit tests for effect editors and behavior.
   - RON serialization roundtrip tests across domain and UI layer.

2. Documentation
   - Add `docs/explanation/conditions_editor_refactor_plan.md` (this document).
   - Update `docs/explanation/implementations.md` with the new refactor summary.
   - Add `How-To` guidance for using the editor if needed.

3. Final QA & Delivery
   - Run recommended commands before completion:
     - `cargo fmt --all`
     - `cargo check --all-targets --all-features`
     - `cargo clippy --all-targets --all-features -- -D warnings`
     - `cargo test --all-features`
   - Final UI verification steps: open the editor, add new conditions, add multiple effects, run import/export, save/load, and confirm data integrity.

4. Data Migration & Compatibility
   - Document any `ConditionDefinition` structure changes.
   - Provide migration scripts or guidance for existing campaigns.
   - Test backward compatibility with existing campaign data.

5. Accessibility & Usability Enhancements
   - Add keyboard shortcuts consistent with other editors.
   - Provide tooltips for complex fields (status effects, elements).
   - Create example templates or starter conditions for common use cases.

6. Success Metrics
   - Specific manual test: Create a poison DOT condition in under 2 minutes.
   - Performance benchmark: Load campaign with 100+ conditions in under 1 second.
   - User acceptance: Designer can create all condition types without documentation.

7. Deliverables & Success Criteria
   - The `conditions_editor` is fully featured and consistent with other editors:
     - Toolbar and file operations implemented;
     - Full `ConditionDefinition` editing (incl. `default_duration`, `icon_id`);
     - Per-effect editing with validation;
     - Documentation and tests in place;
     - Migration guide available for existing campaigns.

---

## File & Symbol Modifications (Reference)

- `sdk/campaign_builder/src/conditions_editor.rs`
  - `struct ConditionsEditorState`:
    - Add `show_import_dialog: bool`, `import_export_buffer: String`.
  - Add `pub fn show(...)` matching patterns in `items_editor.rs`.
  - Add: `show_import_dialog`, `save_conditions`, `load_conditions`.
  - Add: effect UI helpers: `render_effect_list`, `render_effect_editor`, `EffectEditBuffer`.
  - Reuse `ui_helpers::DEFAULT_LEFT_COLUMN_WIDTH` and `compute_panel_height`.

- `sdk/campaign_builder/src/main.rs`
  - Replace `render_conditions_editor(...)` call in `EditorTab::Conditions` to `conditions_editor::ConditionsEditorState::show(...)`.
  - Ensure the `main.rs` passes `campaign_dir`, `conditions_file`, `unsaved_changes`, `status_message`, `file_load_merge_mode`, and `ctx` like other editors.

- Optional new file:
  - `sdk/campaign_builder/src/conditions/effects_editor.rs` (optional to break out effect editor UI code for maintainability).

---

## Additional State Fields Required

```rust
// Add to ConditionsEditorState
pub struct ConditionsEditorState {
    // ... existing fields ...
    pub show_import_dialog: bool,
    pub import_export_buffer: String,
    pub duplicate_dialog_open: bool,
    pub delete_confirmation_open: bool,
    pub selected_for_delete: Option<ConditionId>,
    pub effect_edit_buffer: Option<EffectEditBuffer>,
}

// New helper struct for editing effects
pub struct EffectEditBuffer {
    pub effect_type: ConditionEffectType, // Enum: AttributeMod, Status, DOT, HOT
    pub temp_data: EffectData, // Variant-specific temp fields
}
```

---

## Recommended Implementation Order Summary
0. Complete discovery and preparation (Phase 0).
1. Implement toolbar and file IO (Phase 1).
2. Add fields for condition core properties (Phase 2).
3. Add effects list management (Phase 3).
4. Implement per-variant effect editors and UX polish (Phase 4).
5. Add tests and documentation (Phase 5).

---

If you'd like, I can now:
- Break Phase 1 into a PR-sized checklist (small functions to create in `main.rs` and `conditions_editor.rs`).
- Or create a follow-up file that maps the change list into test cases and example data (RON snippet).
- Or create a stub branch/PR checklist with the exact lines or helper functions to add for each step.

Which would you prefer next?

---

## Appendix A: Example RON Snippets

### Basic Status Effect
```ron
(
    id: "stunned",
    name: "Stunned",
    description: "Cannot act for several rounds.",
    effects: [StatusEffect("stunned")],
    default_duration: Rounds(3),
    icon_id: Some("icon_stunned"),
)
```

### Attribute Modifier (Buff)
```ron
(
    id: "giants_strength",
    name: "Giant's Strength",
    description: "Increases might temporarily.",
    effects: [
        AttributeModifier(
            attribute: "might",
            value: 5,
        ),
    ],
    default_duration: Minutes(10),
    icon_id: Some("icon_strength"),
)
```

### Damage Over Time
```ron
(
    id: "burning",
    name: "Burning",
    description: "Takes fire damage each round.",
    effects: [
        DamageOverTime(
            damage: (count: 2, sides: 6, bonus: 0),
            element: "fire",
        ),
    ],
    default_duration: Rounds(5),
    icon_id: Some("icon_fire"),
)
```

### Heal Over Time
```ron
(
    id: "regeneration",
    name: "Regeneration",
    description: "Heals gradually over time.",
    effects: [
        HealOverTime(
            amount: (count: 1, sides: 8, bonus: 2),
        ),
    ],
    default_duration: Rounds(10),
    icon_id: Some("icon_regen"),
)
```

### Complex Multi-Effect Condition
```ron
(
    id: "poisoned_weakness",
    name: "Poisoned Weakness",
    description: "Suffers poison damage and reduced strength.",
    effects: [
        DamageOverTime(
            damage: (count: 1, sides: 4, bonus: 0),
            element: "poison",
        ),
        AttributeModifier(
            attribute: "might",
            value: -3,
        ),
        StatusEffect("poisoned"),
    ],
    default_duration: Permanent,
    icon_id: Some("icon_poison_debuff"),
)
```

---

## Appendix B: UI Layout Mockup (ASCII)

```
┌─────────────────────────────────────────────────────────────────┐
│ Conditions Editor                                      [X]       │
├─────────────────────────────────────────────────────────────────┤
│ 🔍 Search: [________] ➕Add  📥Import  📂Load  💾Save  📋Export  │
├──────────────────┬──────────────────────────────────────────────┤
│ Conditions       │ Edit Condition: "Poison"                     │
│ ┌──────────────┐ │ ┌──────────────────────────────────────────┐ │
│ │ 🔍 [filter]  │ │ │ ID: poison (read-only)                   │ │
│ ├──────────────┤ │ │ Name: [Poison_____________]              │ │
│ │ • Blind      │ │ │ Description:                             │ │
│ │ ► Poison     │ │ │ [Takes damage over time from poison___] │ │
│ │ • Sleep      │ │ │                                          │ │
│ │ • Bless      │ │ │ Duration: [Permanent ▼]                 │ │
│ │              │ │ │ Icon: [icon_poison___________]           │ │
│ │              │ │ │                                          │ │
│ │              │ │ │ Effects:                                 │ │
│ │              │ │ │ ┌──────────────────────────────────────┐ │ │
│ │              │ │ │ │ 1. DOT: 1d4 poison  [✏️][🗑️][⬆️][⬇️] │ │ │
│ │              │ │ │ │    ➕ Add Effect                     │ │ │
│ │              │ │ │ └──────────────────────────────────────┘ │ │
│ │              │ │ └──────────────────────────────────────────┘ │
│ └──────────────┘ │                                              │
└──────────────────┴──────────────────────────────────────────────┘
```

---

## Appendix C: Migration Guide for Existing Campaigns

If you're upgrading an existing campaign after this refactor:

1. **Backup First**: Always backup your `data/conditions.ron` before editing.

2. **Field Additions**: If new fields are added to `ConditionDefinition`:
   - Old files will use default values for missing fields
   - `icon_id` defaults to `None`
   - No manual migration needed

3. **Effect Validation**: After upgrading:
   - Load conditions in editor
   - Review any warnings about invalid effect data
   - Re-save to apply new validation rules

4. **Spell References**:
   - Check that spell `applied_conditions` still reference valid IDs
   - Use the "Used by" panel to see which spells reference each condition

5. **Breaking Changes** (if any):
   - If `ConditionEffect` enum changes, manual RON edits may be required
   - Check documentation for specific upgrade notes

6. **CI/CD Integration**:
   - Consider adding automated validation of `conditions.ron` in your build pipeline
   - Example: `cargo test --test validate_campaign_data`

---

## Appendix D: Known Element and Attribute Constants

### Standard Elements (for DOT effects)
- `"fire"` - Fire damage
- `"cold"` - Cold/ice damage
- `"electricity"` - Lightning damage
- `"poison"` - Poison damage
- `"acid"` - Acid damage
- `"psychic"` - Mental/psychic damage
- `"energy"` - Generic magical energy
- `"physical"` - Physical damage

### Standard Attributes (for AttributeModifier)
- `"might"` - Physical strength
- `"intellect"` - Mental acuity
- `"personality"` - Charisma/willpower
- `"endurance"` - Constitution/stamina
- `"speed"` - Agility/reflexes
- `"accuracy"` - Hit chance
- `"luck"` - Random event modifier

### Standard Status Effects
- `"blind"` - Cannot see, reduced accuracy
- `"asleep"` - Unconscious, cannot act
- `"stunned"` - Dazed, cannot act
- `"paralyzed"` - Cannot move or act
- `"silenced"` - Cannot cast spells
- `"confused"` - Random actions
- `"poisoned"` - Visual indicator for poison DOT

**Note**: These are conventions, not hard restrictions. The editor allows free-form strings but will suggest these standard values.
