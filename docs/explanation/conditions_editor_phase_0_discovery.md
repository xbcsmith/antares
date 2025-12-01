# Phase 0: Discovery & Preparation — Conditions Editor Refactor

Author: Antares Dev Team (implemented by engineer)
Date: 2025-11-XX

## Summary

This Phase 0 document summarizes the results of the initial discovery and preparation work for the Conditions Editor refactor (Toolbar, Effects Editor, File I/O). It collects findings from a code audit, identifies integration points and tests, documents existing data and examples, captures reusable editor components, and provides a prioritized plan of follow-up actions for Phase 1 and beyond.

## Objective

- Audit the current codebase and editor usage for `ConditionEffect` and `ConditionDefinition`.
- Identify all external and runtime dependencies that must be respected by the editor.
- Produce a list of reusable UI components and patterns that can be copied from other editors (spells/items/monsters).
- Produce sample test data (RON) covering normal cases and edge cases.
- Define success criteria for moving to Phase 1 (Core Implementation).

## Scope

- The focus is limited to the campaign builder Conditions Editor in `sdk/campaign_builder`. No core domain types will be modified as part of Phase 0.
- This phase does not implement UI nor change behavior; only documentation, planning, and sample data are delivered.

## High-level Findings

Code Audit — Where ConditionEffect is defined and used

- Core type definition: `ConditionEffect`, `ConditionDefinition`, `ActiveCondition`.

```antares/src/domain/conditions.rs#L1-200
(pub enum ConditionEffect { ... })
```

- Runtime application:
  - The game uses `ActiveCondition` and `ConditionDefinition` in character/monster logic:
    - Damage-over-time and Heal-over-time are applied by the combat engine and spell effects.

```antares/src/domain/magic/spell_effects.rs#L1-220
(pub fn apply_condition_dot_effects(...) { ... })
```

- Character and Monster attribute modifications from conditions are computed at runtime:

```antares/src/domain/character.rs#L700-840
(pub fn get_condition_modifier(...) { ... })
```

```antares/src/domain/combat/monster.rs#L430-500
(pub fn get_condition_modifier(...) { ... })
```

- Tests cover DOT/HOT and effect application in the combat engine:

```antares/src/domain/combat/engine.rs#L800-940
(mod tests { fn test_dot_effects_application() { ... } })
```

UI/Editor code

- Conditions editor (current, basic):

```antares/sdk/campaign_builder/src/conditions_editor.rs#L1-400
(render_conditions_editor(...) — read-only `effects` list)
```

- Spells Editor provides a well-structured `show()` pattern and a dice UI used in DOT/HOT entries:

```antares/sdk/campaign_builder/src/spells_editor.rs#L580-680
(Shows Dice UI pattern: count/sides/bonus using DragValue elements)
```

- Items/Monsters Editors provide the toolbar, import/export/merge, and file dialogs used across the SDK:

```antares/sdk/campaign_builder/src/items_editor.rs#L1-200
(sdk/campaign_builder/src/monsters_editor.rs#L1-200)
```

Campaign & Data Files

- Example campaign RON referencing conditions:

```antares/campaigns/tutorial/campaign.ron#L1-40
(conditions_file: "data/conditions.ron")
```

- Example conditions.ron content used by the tutorial campaign (sample):

```antares/campaigns/tutorial/data/conditions.ron#L1-200
([
    (id: "blind", name: "Blind", effects: [StatusEffect("blind"), AttributeModifier(...)]),
    (id: "sleep", ...),
    (id: "bless", ...),
    (id: "poison", ...),
])
```

## Key Observations & Constraints

- Runtime-only fields:

  - `ActiveCondition.magnitude` is purely a runtime multiplier (default 1.0).
    - This field MUST NOT be added to `ConditionDefinition` in RON or be persisted in the editor's saved data.
    - The editor may offer a simulation preview using a sample magnitude, but this is separate from saved data.

- `ConditionEffect` variants are the canonical types editors must support:

  - `AttributeModifier { attribute: String, value: i16 }`
  - `StatusEffect(String)`
  - `DamageOverTime { damage: DiceRoll, element: String }`
  - `HealOverTime { amount: DiceRoll }`

- Attribute & Element naming:

  - Attributes used in `AttributeModifier` are currently strings; the editor should present a ComboBox with valid attribute names:
    - Stats: `might`, `intellect`, `personality`, `endurance`, `speed`, `accuracy`, `luck`
    - Resistances: `magic`, `fire`, `cold`, `electricity`, `acid`, `fear`, `poison`, `psychic` (domain-defined)
  - Elements used with DOT effects are currently arbitrary strings (e.g., `fire`, `poison`); a recommended list should be used for UI convenience and validation.

- Validation patterns already exist in other editors:

  - Items/Monsters/Spells editors include:
    - Top toolbar (Search / Add / Import / Load / Save / Export / Merge)
    - `show_import_dialog` pattern with `import_export_buffer` usage
    - `unsaved_changes` flag toggling after actions
    - `rfd::FileDialog` + `ron` parsing/serialization + robust error reporting to `status_message`
    - `Duplicate` and `Delete` UI buttons with confirmation dialogs

- File I/O behavior:
  - `load_conditions()` and `save_conditions()` patterns exist in `sdk/campaign_builder/src/main.rs` and should be reused.
  - Merge behavior (default loaded vs merged with existing data) is useful and exists already.

## Reusable Components & Patterns

- Dice editor (from Spells editor): `DiceRoll` editing via `egui::DragValue` for
  `count`, `sides`, `bonus`. Use identical UI & validation for DOT/HOT.

```antares/sdk/campaign_builder/src/spells_editor.rs#L600-666
(Example dice editor snippet)
```

- Top toolbar and file I/O:
  - Add toolbar with Search, Add Condition, Import, Load from File, Save to File, Export (copy).
  - Use `rfd::FileDialog` and `ron::from_str` & `ron::ser::to_string_pretty`.
  - Support 'Merge' vs 'Replace' behavior for Load.

```antares/sdk/campaign_builder/src/items_editor.rs#L1-200
(Shows toolbar & load/save implementation)
```

- UI patterns:
  - `show()` function signature consistent with other editors:
    - Accepts `ui`, `ctx`, `campaign_dir`, `conditions_file`, `unsaved_changes`, `status_message`, `file_load_merge_mode`.
  - Use `ConditionsEditorState::show()` instead of module-level `render_` function.
  - Re-use `show_import_dialog(ctx, ...)` pattern from other editors.

## Test Data & Edge Cases

- Core test data (RON) to be prepared and committed to `data/` or the test suite:
  - Basic status & attribute modifiers (already present in `conditions.ron`)
  - DOT & HOT examples (fire, poison, regeneration)
  - Complex multi-effect condition with multiple `ConditionEffect` variants
- Edge case RON examples (for validation & error handling):
  - `empty_effects` (effects: [])
  - `max_values` (big dice counts/sides, large attribute modifier absolute values)
  - `special_chars_in_id` (IDs with spaces or unicode — must be validated by editor)
  - `duplicate_id` cases
  - `invalid_ron` content to ensure error messages & recovery options
- Include tests:
  - Unit tests for serialization + round-trip (RON -> ConditionDefinition -> RON)
  - Editor tests: Add + Edit + Save + Import/Export roundtrips
  - Validation tests: duplicate ID detection and dice/value bounds

## Deliverables for Phase 0 (complete)

- Documentation of current state and dependencies (this file).
- Identification of reusable components:
  - Dice editor (spells editor)
  - Toolbar & file I/O patterns (items/monsters editors)
- An initial set of sample RON test data (examples in `data/conditions.ron` already present).
- A clear migration and integration checklist for the refactor's Phase 1.

## Success Criteria (Phase 0)

- A thorough list of all code locations where `ConditionEffect` is defined and used (domain & runtime).
- Confirmation that `ActiveCondition.magnitude` is runtime-only and should not be serialized by editor changes.
- Reusable UI components and patterns identified and documented with implementation references.
- A set of sample test RON snippets prepared for Phase 1 tests.

## Risks & Observations

- `ConditionEffect` directly uses strings for attributes & elements — string typos can lead to runtime bugs. Consider a domain-level `Attribute` enum in the medium-term (this is out of scope for the refactor).
- Changing the domain shape of `ConditionEffect` will require migration & potential manual RON updates. If the editor implements new fields, we must ensure backward compatibility.
- Editor validation rules must not be too strict — keep an escape/hard override for modders with valid reasons to use custom strings.
- If other code path relies on `ActiveCondition.magnitude` being default 1.0, test coverage must explicitly ensure no change in behavior (editor must not change default application behavior).

## Phase 1 Preparation — Implementation task list (prioritized)

The following items have been identified and vetted to move into Phase 1:

1. Convert `render_conditions_editor` to `ConditionsEditorState::show`:

   - Add `show_import_dialog: bool`, `import_export_buffer: String` to `ConditionsEditorState`.
   - Add `show()` signature consistent with other editors (include `ui`, `ctx`, `campaign_dir`, `conditions_file`, `unsaved_changes`, `status_message`, `file_load_merge_mode`).

2. Implement top toolbar with:

   - Search input moved into toolbar,
   - Add Condition (creates a default condition edit buffer),
   - Import dialog (`show_import_dialog`) with `import_export_buffer`,
   - Load from File / Save to File (RON),
   - Export condition (copies RON to clipboard),
   - Duplicate & Delete (with confirmation).

3. Implement `load_conditions` & `save_conditions` helper functions using the `ron` patterns found in other editors:

   - Merge behavior for loading,
   - Validation errors surfaced in `status_message` and `unsaved_changes` flags.

4. Add `EffectEditBuffer` structure & per-variant editors:

   - `AttributeModifier` editor (attribute selection, value),
   - `StatusEffect` editor (text input for a status tag),
   - `DamageOverTime` editor (re-use dice UI + element selection),
   - `HealOverTime` editor (re-use dice UI),
   - Add the per-effect actions: Edit, Delete, Reorder, Duplicate.

5. Add ID uniqueness and basic validation:

   - Hook into `CampaignBuilderApp::validate_*_ids` style pattern to add `validate_condition_ids`.
   - Inline feedback in the editor form (ID required, uniqueness validation).

6. Add tests:

   - RON load/save roundtrip unit tests.
   - Editor unit tests: Add, edit, reorder, duplicate, delete effect actions.
   - Validation tests for dice bounds and duplicate IDs.

7. UX & Help:
   - Include a small 'preview' panel: show what a sample `ActiveCondition` application would look like (using preview `magnitude` slider).
   - Provide a small help tip next to attribute ComboBox listing valid attributes per domain.

## Appendix A — Example RON Snippets (for quick copy)

- Basic status / attribute modifier / DOT / HOT (from tutorial)

```antares/data/conditions.ron#L1-200
[
    (
        id: "blind",
        name: "Blind",
        description: "Reduces accuracy and perception.",
        effects: [
            StatusEffect("blind"),
            AttributeModifier(attribute: "accuracy", value: -5),
        ],
        default_duration: Rounds(5),
        icon_id: Some("icon_blind"),
    ),
    (
        id: "poison",
        name: "Poison",
        description: "Takes damage over time.",
        effects: [
            DamageOverTime(damage: (count: 1, sides: 4, bonus: 0), element: "poison"),
        ],
        default_duration: Permanent,
        icon_id: Some("icon_poison"),
    ),
]
```

- Edge case: empty effects

```/dev/null/example.ron#L1-32
[
    (
        id: "empty_effects",
        name: "Empty Effects",
        description: "No effects; used to test UI and validation.",
        effects: [],
        default_duration: Rounds(1),
        icon_id: None,
    ),
]
```

## Appendix B — Important File References (phase 1 integration list)

- Domain:

  - `antares/src/domain/conditions.rs` — `ConditionDefinition`, `ConditionEffect`, `ActiveCondition`
  - `antares/src/domain/character.rs` — `Character::get_condition_modifier`, `Character::has_status_effect`
  - `antares/src/domain/combat/monster.rs` — `Monster::get_condition_modifier`, `Monster::has_status_effect`
  - `antares/src/domain/magic/spell_effects.rs` — `apply_spell_conditions_to_character`, `apply_condition_dot_effects`

- SDK Editor & Main:

  - `antares/sdk/campaign_builder/src/conditions_editor.rs`
  - `antares/sdk/campaign_builder/src/spells_editor.rs` — dice editing, patterns
  - `antares/sdk/campaign_builder/src/items_editor.rs` — top toolbar & file I/O patterns
  - `antares/sdk/campaign_builder/src/monsters_editor.rs` — import/load/save patterns
  - `antares/sdk/campaign_builder/src/main.rs` — `load_conditions`, `save_conditions`, `CampaignMetadata::conditions_file` default

- Data:
  - `antares/campaigns/tutorial/data/conditions.ron`

## Next Steps & Acceptance Criteria for Phase 1

- `ConditionsEditorState::show` method implemented and integrated in `sdk/campaign_builder/src/main.rs`.
- `ConditionsEditorState` has import/export fields and import dialog behavior consistent with other editors.
- Top toolbar is present and fully functional (Search, Add, Import, Load, Save, Export).
- Per-variant `ConditionEffect` editors are implemented and can create/edit effects in the UI.
- `load_conditions` and `save_conditions` round-trip tested with `campaigns/tutorial/data/conditions.ron`.
- Validation tests: duplicate ID detection, dice limits and attribute selection coverage.
- The editor must follow the 'unsaved_changes' and 'status_message' patterns used by other editors.
- `cargo check` & `cargo test` pass with new tests included.

## Notes for Future Phases & Non-Goals

- Rewriting domain types to use enums for a `Attribute` or `Element` is out-of-scope for the Conditions Editor refactor and will be deferred to a future domain-centered change, if necessary.
- This phase does not add migration tools to convert a changed domain `ConditionEffect` enum to new forms. If domain changes happen later, a migration guide must be created.

## Contact & Ownership

- For any clarifications or if any domain type changes are required, engage with the core domain team. Any domain-level changes require updating the architecture documentation and developer sign-off.

## Concluding Remarks

Phase 0 has produced a clear, actionable plan and all necessary discovery artifacts to proceed confidently into Phase 1 (Core Implementation). The next step is to implement the toolbar and file I/O foundations, then proceed to the deeper per-variant effect editing.
