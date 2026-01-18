# SDK Campaign Builder: Autocomplete Integration Implementation Plan

## Document Update Summary

**Last Updated**: 2025-01-XX

**Major Revision**: Extended Phase 2 to cover ALL Campaign Builder editors

**What Changed**:

- Added 4 new editor integrations to Phase 2 (originally 5, now 9 total)
- Extended Phase 2.6-2.9 to cover: Races Editor, Quest Editor, Dialogue Editor, and Map Event additional references
- Updated deliverables from 5 integrations to 9 complete editor integrations
- Added proficiency, item tag, special ability, map, NPC, and quest reference autocomplete
- Comprehensive coverage now includes all entity types: Monster, Item, Condition, Proficiency, Map, NPC, Quest

**New Sections Added**:

- **Phase 2.6**: Proficiency Reference in Races Editor
- **Phase 2.7**: Quest Objectives Entity References (Monster/Item/Map/NPC)
- **Phase 2.8**: Dialogue Tree Entity References (Quest/Item)
- **Phase 2.9**: Map Event Additional References (Map/NPC)

**Impact**: ~40% more work in Phase 2, but ensures complete coverage across the entire Campaign Builder SDK. No half-implementations or missing editors.

---

## Overview

This plan outlines the integration of the `egui_autocomplete` library into the Campaign Builder SDK to improve data entry efficiency and validity across ALL editors. The implementation provides context-aware suggestions for numeric ranges, string restrictions, and entity references (monsters, items, spells, conditions, maps, NPCs, quests, proficiencies) through reusable autocomplete widgets.

**Key Goals:**

- Reduce manual ID lookup and typing errors across all 9 editor contexts
- Make data constraints visible during entry (proficiency lists, valid item tags, entity references)
- Improve search efficiency in large entity lists (items, monsters, maps, NPCs)
- Maintain consistency with existing UI patterns
- Enable cross-editor reference validation (quests→monsters, dialogue→quests, etc.)
- Provide complete coverage for all Campaign Builder editors

## Current State Analysis

### Existing Infrastructure

| Component            | File Path                                       | Status     | Notes                                                           |
| -------------------- | ----------------------------------------------- | ---------- | --------------------------------------------------------------- |
| Campaign Builder     | `sdk/campaign_builder/`                         | ✅ Active  | Uses `egui` 0.33, `eframe` 0.33                                 |
| UI Helpers           | `sdk/campaign_builder/src/ui_helpers.rs`        | ✅ Active  | Centralized UI components (toolbars, layouts, attribute inputs) |
| Monsters Editor      | `sdk/campaign_builder/src/monsters_editor.rs`   | ✅ Active  | Uses standard `TextEdit` and `DragValue` widgets                |
| Items Editor         | `sdk/campaign_builder/src/items_editor.rs`      | ✅ Active  | Basic item management interface                                 |
| Classes Editor       | `sdk/campaign_builder/src/classes_editor.rs`    | ✅ Active  | Manages class definitions with starting items & proficiencies   |
| Characters Editor    | `sdk/campaign_builder/src/characters_editor.rs` | ✅ Active  | Character creation and equipment                                |
| Races Editor         | `sdk/campaign_builder/src/races_editor.rs`      | ✅ Active  | Race definitions with proficiencies and item restrictions       |
| Spells Editor        | `sdk/campaign_builder/src/spells_editor.rs`     | ✅ Active  | Spell definitions with conditions                               |
| Map Editor           | `sdk/campaign_builder/src/map_editor.rs`        | ✅ Active  | Map events, traps, teleports, encounters, NPCs                  |
| Quest Editor         | `sdk/campaign_builder/src/quest_editor.rs`      | ✅ Active  | Quest objectives with monster/item/map/NPC references           |
| Dialogue Editor      | `sdk/campaign_builder/src/dialogue_editor.rs`   | ✅ Active  | Dialogue trees with quest and item references                   |
| Conditions Editor    | `sdk/campaign_builder/src/conditions_editor.rs` | ✅ Active  | Status condition definitions                                    |
| Data Files           | `data/*.ron`                                    | ✅ Active  | RON format for items, spells, monsters, conditions              |
| Autocomplete Library | N/A                                             | ❌ Missing | `egui_autocomplete` not yet integrated                          |

### Identified Issues

1. **Manual ID Entry**: Referencing other entities (e.g., Drop Item ID in monsters, Equipment IDs in classes, Monster IDs in quest objectives, Map IDs in teleports) requires manual lookup in separate files and manual typing of numeric IDs.
2. **Opaque Constraints**: Numeric ranges (0-255 for stats) and string format rules (proficiency IDs, item tags) are not visible or enforced during data entry.
3. **Inefficient Search**: Selecting from large entity lists (100+ items, 50+ monsters, multiple maps, many NPCs) requires scrolling through ComboBox or manual search bars.
4. **Error-Prone Workflow**: Typos in IDs or names result in validation errors only after save/reload cycle, affecting multiple editors (Races, Quests, Dialogue, Map Events).
5. **No Cross-Reference Validation**: Cannot verify if referenced entity (Item, Monster, Spell, Map, NPC, Quest, Proficiency) actually exists in campaign data during entry.
6. **Inconsistent UX**: Some editors use ComboBox, others use TextEdit, creating confusion about which fields support selection vs. manual entry.

## Implementation Phases

### Phase 1: Core Integration & Reusable Widget

**Goal**: Add `egui_autocomplete` dependency and create reusable autocomplete widget in centralized UI helpers.

#### 1.1 Dependency Addition

**File**: `sdk/campaign_builder/Cargo.toml`

**Action**:

- Add `egui_autocomplete = "0.7"` to `[dependencies]` section (compatible with `egui` 0.33)
- Verify no version conflicts with existing `egui` and `eframe` dependencies

**Validation Command**:

```bash
cd sdk/campaign_builder
cargo check --all-targets --all-features
```

**Expected**: Zero compilation errors, dependency resolves successfully.

#### 1.2 Create Autocomplete Widget Wrapper

**File**: `sdk/campaign_builder/src/ui_helpers.rs`

**Action**:

- Add SPDX copyright header if creating new sections
- Define `pub struct AutocompleteInput<'a>` with fields:
  - `id_salt: &'a str` (unique widget identifier)
  - `candidates: &'a [String]` (suggestion list)
  - `placeholder: Option<&'a str>` (hint text)
- Implement `new(id_salt: &'a str, candidates: &'a [String]) -> Self`
- Implement `with_placeholder(mut self, placeholder: &'a str) -> Self` (builder pattern)
- Implement `show(&self, ui: &mut egui::Ui, text: &mut String) -> egui::Response`
  - Wrap `egui_autocomplete::AutoCompleteTextEdit`
  - Configure case-insensitive filtering
  - Return response for chaining

**Code Pattern**:

````rust
/// Autocomplete text input with dropdown suggestions
///
/// # Examples
///
/// ```
/// use campaign_builder::ui_helpers::AutocompleteInput;
///
/// let candidates = vec!["Goblin".to_string(), "Orc".to_string()];
/// let mut input = String::new();
/// AutocompleteInput::new("monster_select", &candidates)
///     .with_placeholder("Type monster name...")
///     .show(ui, &mut input);
/// ```
pub struct AutocompleteInput<'a> { /* ... */ }
````

#### 1.3 Integration with Existing UI Patterns

**File**: `sdk/campaign_builder/src/ui_helpers.rs`

**Action**:

- Ensure `AutocompleteInput` follows same naming convention as `AttributePairInput`
- Use consistent spacing constants (`DEFAULT_LEFT_COLUMN_WIDTH`, etc.)
- Add doc comments with runnable examples (tested by `cargo test --doc`)

#### 1.4 Testing Requirements

**Test Type**: Manual + Integration

**Test File**: `sdk/campaign_builder/src/test_utils.rs` or inline test harness

**Test Cases**:

1. **Rendering**: Widget displays without panic
2. **Filtering**: Typing "go" filters to candidates starting with "go" (case-insensitive)
3. **Selection**: Clicking suggestion updates `text` parameter
4. **Empty Candidates**: Empty list shows no dropdown
5. **No Match**: Typing text with no matches shows empty dropdown

**Manual Test**:

- Add test window in `sdk/campaign_builder/src/test_play.rs`
- Create sample candidate list: `["Goblin", "Orc", "Dragon", "Skeleton"]`
- Verify dropdown appears, filters correctly, and selection updates text field

**Validation Commands**:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

#### 1.5 Deliverables

- [ ] `egui_autocomplete` dependency added to `Cargo.toml`
- [ ] `AutocompleteInput` struct implemented in `ui_helpers.rs`
- [ ] Doc comments with examples added
- [ ] Manual test harness created and verified
- [ ] All quality checks pass (fmt, check, clippy, test)

#### 1.6 Success Criteria

- `cargo check` compiles without errors
- `AutocompleteInput::show()` renders dropdown with suggestions
- Typing filters candidates correctly (case-insensitive)
- Selecting suggestion updates text buffer
- Widget follows existing UI helper patterns and documentation standards

---

### Phase 2: Entity Reference Autocomplete

**Goal**: Replace manual ID entry and basic dropdowns with autocomplete for entity references (Monsters, Items, Conditions).

#### 2.1 Monster Reference in Monsters Editor

**File**: `sdk/campaign_builder/src/monsters_editor.rs`

**Target Fields**:

- `MonsterDefinition.name` (primary identification)
- Special attacks that reference other monsters (e.g., "Summon Goblin")

**Action**:

- Add `monster_name_candidates: Vec<String>` to `MonstersEditorState`
- Populate from loaded `MonsterDefinition` list on state initialization
- Replace `TextEdit` for monster name search with `AutocompleteInput`
- Map selected name back to `MonsterDefinition` index for editing

**Code Location**: `MonstersEditorState::default()` initialization, monster list rendering logic

**Data Source**: Existing `Vec<MonsterDefinition>` in editor state

#### 2.2 Item Reference in Classes Editor

**File**: `sdk/campaign_builder/src/classes_editor.rs`

**Target Fields**:

- `Class.starting_items: Vec<ItemId>` (u8/u16)
- `Class.starting_equipment.weapon: Option<ItemId>`
- `Class.starting_equipment.armor: Option<ItemId>`
- Other equipment slots (shield, helmet, gloves, boots, accessory)

**Action**:

- Add `item_candidates: Vec<(String, ItemId)>` to `ClassesEditorState`
  - Tuple maps display name to numeric ID
- Load from `data/items.ron` on editor initialization
- Replace ComboBox with `AutocompleteInput` for item selection
- Implement mapping: selected name → lookup `ItemId` → add to `starting_items` Vec
- Display format: `"{name} (ID: {id})"` for clarity

**Data Source**: Parse `data/items.ron` or load from `AssetManager`

**Validation**: Verify selected `ItemId` exists in loaded item list before adding

#### 2.3 Item Reference in Characters Editor

**File**: `sdk/campaign_builder/src/characters_editor.rs`

**Target Fields**:

- `Character.inventory: Vec<ItemId>`
- `Character.equipment.weapon`, `armor`, etc.

**Action**:

- Reuse `item_candidates` loading logic from Classes Editor
- Add "Add Item" button that opens autocomplete dialog
- Replace equipment slot dropdowns with autocomplete for faster search

**Shared Code**: Extract item candidate loading into `ui_helpers.rs` or `asset_manager.rs` helper function

#### 2.4 Condition Reference in Spells Editor

**File**: `sdk/campaign_builder/src/spells_editor.rs`

**Target Fields**:

- `Spell.applied_conditions: Vec<ConditionId>` (e.g., "Poison", "Sleep", "Paralysis")

**Action**:

- Add `condition_candidates: Vec<(String, ConditionId)>` to `SpellsEditorState`
- Load from `data/conditions.ron` or existing condition definitions
- Replace manual condition entry with `AutocompleteInput`
- Display format: `"{name}"` (condition names are descriptive)

**Data Source**: Parse `data/conditions.ron` or in-memory condition registry

#### 2.5 Condition Reference in Map Editor (Traps)

**File**: `sdk/campaign_builder/src/map_editor.rs`

**Target Fields**:

- `MapEvent::Trap.effect: Option<String>` (condition name or effect description)

**Action**:

- Reuse `condition_candidates` from Spells Editor
- Add autocomplete for trap effect selection
- Allow freeform text entry (effects may be custom, not just conditions)

**Validation**: Warn if entered effect doesn't match known condition (soft validation)

#### 2.6 Proficiency Reference in Races Editor

**File**: `sdk/campaign_builder/src/races_editor.rs`

**Target Fields**:

- `RaceDefinition.proficiencies: Vec<ProficiencyId>` (weapon/armor proficiencies granted by race)
- `RaceDefinition.incompatible_item_tags: Vec<String>` (item tags the race cannot use)
- `RaceDefinition.special_abilities: Vec<String>` (racial abilities like "infravision")

**Action**:

- Add `proficiency_candidates: Vec<(String, ProficiencyId)>` to `RacesEditorState`
- Load from `ProficiencyDatabase` or parse from `ProficiencyId` enum variants
- Add `item_tag_candidates: Vec<String>` loaded from all unique item tags in campaign
- Add `special_ability_candidates: Vec<String>` for known racial abilities
- Replace `searchable_selector_multi` with `AutocompleteInput` for better UX
- Display format for proficiencies: `"{proficiency_name}"` (e.g., "Sword", "Heavy Armor")
- Display format for item tags: `"{tag_name}"` (e.g., "Heavy", "Metal", "TwoHanded")

**Data Source**:

- Proficiencies: `antares::domain::proficiency::ProficiencyDatabase`
- Item tags: Extract from loaded `Item` definitions
- Special abilities: Define constant list or load from configuration

**Validation**: Verify selected ProficiencyId exists in ProficiencyDatabase before adding

#### 2.7 Quest Objectives Entity References

**File**: `sdk/campaign_builder/src/quest_editor.rs`

**Target Fields**:

- `ObjectiveEditBuffer.monster_id: String` (kill monster objectives)
- `ObjectiveEditBuffer.item_id: String` (collect/deliver item objectives)
- `ObjectiveEditBuffer.map_id: String` (visit location objectives)
- `ObjectiveEditBuffer.npc_id: String` (talk to NPC objectives)
- `DialogueEditBuffer.associated_quest: String` (quest ID reference)

**Action**:

- Add `monster_candidates: Vec<(String, MonsterId)>` to `QuestEditorState`
- Add `item_candidates: Vec<(String, ItemId)>` to `QuestEditorState`
- Add `map_candidates: Vec<(String, MapId)>` to `QuestEditorState`
- Add `npc_candidates: Vec<(String, String)>` to `QuestEditorState` (NPC ID is String type)
- Load candidates from respective data sources on editor initialization
- Replace `TextEdit` for entity ID fields with `AutocompleteInput`
- Display format: `"{entity_name} (ID: {id})"` for all entity types
- Map selected name → lookup entity ID → update buffer field

**Data Source**:

- Monsters: Existing `Vec<MonsterDefinition>` in campaign
- Items: Parse `data/items.ron` or load from `AssetManager`
- Maps: Existing `Vec<Map>` in campaign
- NPCs: Extract from all map NPC definitions

**Validation**: Verify selected entity ID exists in loaded data before saving objective

**Note**: Quest editor has multiple objective types (Kill, Collect, Visit, TalkTo), so autocomplete must be context-aware based on selected `objective_type`.

#### 2.8 Dialogue Tree Entity References

**File**: `sdk/campaign_builder/src/dialogue_editor.rs`

**Target Fields**:

- `DialogueEditBuffer.associated_quest: String` (quest ID reference)
- `ConditionEditBuffer` - likely contains item/quest references for dialogue conditions
- `ActionEditBuffer` - likely contains item/quest references for dialogue actions

**Action**:

- Add `quest_candidates: Vec<(String, QuestId)>` to `DialogueEditorState`
- Add `item_candidates: Vec<(String, ItemId)>` to `DialogueEditorState` (for condition/action references)
- Load from respective data sources on editor initialization
- Replace `TextEdit` for quest/item reference fields with `AutocompleteInput`
- Display format: `"{quest_name} (ID: {id})"` or `"{item_name} (ID: {id})"`

**Data Source**:

- Quests: Load from `Vec<Quest>` in campaign
- Items: Reuse item loading logic from Classes/Characters editors

**Validation**: Verify selected quest/item ID exists before saving dialogue tree

**Note**: The dialogue editor state already tracks `available_dialogue_ids`, `available_quest_ids`, and `available_item_ids` fields, so these should be converted to autocomplete candidate lists.

#### 2.9 Map Event Additional References

**File**: `sdk/campaign_builder/src/map_editor.rs`

**Target Fields**:

- `EventEditorState.teleport_map_id: String` (destination map for teleport events)
- `EventEditorState.npc_id: String` (NPC reference for dialogue events)
- `EventEditorState.encounter_monsters: Vec<MonsterId>` (already has query field)
- `EventEditorState.treasure_items: Vec<ItemId>` (already has query field)

**Action**:

- Add `map_candidates: Vec<(String, MapId)>` to `MapsEditorState` or `MapEditorState`
- Add `npc_candidates: Vec<(String, String)>` extracted from all NPCs in all maps
- Replace `TextEdit` for `teleport_map_id` with `AutocompleteInput`
- Replace `TextEdit` for `npc_id` with `AutocompleteInput`
- **Note**: `encounter_monsters` and `treasure_items` already have `*_query` fields, so they may already use searchable_selector_multi - upgrade to AutocompleteInput if appropriate
- Display format for maps: `"{map_name} (ID: {id})"`
- Display format for NPCs: `"{npc_name} (ID: {npc_id})"`

**Data Source**:

- Maps: Load from campaign's `Vec<Map>`
- NPCs: Extract from `Map.npcs` across all maps in campaign

**Validation**: Verify teleport destination map exists; warn if NPC ID not found

#### 2.10 Integration Testing

**Test Type**: Integration + Manual

**Test Cases**:

1. **Item Selection in Classes**: Add starting item via autocomplete, verify ItemId saved correctly
2. **Condition Selection in Spells**: Add condition via autocomplete, verify ConditionId in applied_conditions
3. **Monster Name Search**: Type partial monster name, verify filtering works across 20+ monsters
4. **Proficiency Selection in Races**: Add proficiency via autocomplete, verify ProficiencyId in races.proficiencies
5. **Quest Objective References**: Create "Kill 5 Goblins" objective via monster autocomplete, verify MonsterId saved
6. **Dialogue Quest References**: Associate dialogue with quest via autocomplete, verify QuestId saved
7. **Map Teleport**: Create teleport event via map autocomplete, verify destination MapId saved
8. **Cross-File Consistency**: Reference Item ID in Classes, verify it exists in Items editor

**Manual Test Workflow**:

1. Create new Class in Classes Editor
2. Add starting item "Longsword" via autocomplete
3. Save class definition
4. Create new Quest in Quest Editor
5. Add objective "Kill 5 Goblins" using monster autocomplete
6. Create new Race in Races Editor
7. Add proficiency "Sword" using autocomplete
8. Reload Campaign Builder
9. Verify all references persist correctly across editors

**Validation Commands**:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

#### 2.11 Deliverables

- [ ] Monster name autocomplete in Monsters Editor
- [ ] Item reference autocomplete in Classes Editor (starting_items, equipment)
- [ ] Item reference autocomplete in Characters Editor (inventory, equipment)
- [ ] Condition reference autocomplete in Spells Editor (applied_conditions)
- [ ] Condition/effect autocomplete in Map Editor (traps)
- [ ] Proficiency/item tag/special ability autocomplete in Races Editor
- [ ] Monster/item/map/NPC autocomplete in Quest Editor (objectives)
- [ ] Quest/item autocomplete in Dialogue Editor (conditions, actions)
- [ ] Map/NPC autocomplete in Map Editor (teleport events, NPC dialogue events)
- [ ] Candidate loading logic extracted to reusable helper functions
- [ ] Integration tests verify ID mapping correctness across all 9 editor integrations
- [ ] All quality checks pass

#### 2.12 Success Criteria

- User can type partial entity name and see dropdown of valid matches in all 9 editor contexts
- Selecting entity correctly maps display name to typed ID (ItemId, ConditionId, MonsterId, MapId, ProficiencyId, QuestId)
- Changes persist correctly in RON data files after save/load cycle
- No invalid entity references created (validation catches missing IDs)
- Autocomplete responds within 100ms for lists up to 200 items
- Quest objectives dynamically show relevant autocomplete based on objective type (Kill→monsters, Collect→items, Visit→maps)
- Cross-editor references work correctly (Quest references Monster, Dialogue references Quest, etc.)

---

### Phase 3: Validation, Constraints & Polish

**Goal**: Add validation feedback, numeric constraints, and proficiency/enum autocomplete for non-entity fields.

#### 3.1 Numeric Field Validation (Non-Autocomplete)

**Files**: `sdk/campaign_builder/src/map_editor.rs`, `monsters_editor.rs`

**Target Fields**:

- `Trap.damage: u16` (0-65535)
- `MonsterDefinition.stats.might: AttributePair` (base: 0-255, current: 0-255)

**Action**:

- Ensure numeric fields use `egui::DragValue` with `.clamp_range(0..=255)` or appropriate range
- Add tooltip on hover: "Valid range: 0-255"
- DO NOT use autocomplete for pure numeric entry (autocomplete is for text/references)

**Optional Enhancement**: Provide "Preset Values" button group for common damage values (10, 25, 50, 100) as quick-select

**Validation**: `cargo clippy` should warn on unbounded numeric inputs

#### 3.2 String Restriction Autocomplete (Proficiencies in Classes)

**File**: `sdk/campaign_builder/src/classes_editor.rs`

**Target Fields**:

- `Class.proficiencies: Vec<ProficiencyId>` (enum: SimpleWeapon, MartialWeapon, HeavyArmor, etc.)

**Action**:

- Reuse proficiency autocomplete logic from Phase 2.6 (Races Editor)
- Define `PROFICIENCY_CANDIDATES: &[&str]` constant with valid proficiency IDs (if not already done)
- Use `AutocompleteInput` to suggest valid proficiency strings
- Validate on save: reject unknown proficiency IDs with clear error message

**Data Source**: Hardcoded list or parse from `ProficiencyId` enum variants (if accessible)

**Validation Error**: `"Unknown proficiency: 'invalid_name'. Valid options: [...]"`

**Note**: This was originally planned for Phase 3 but now shares implementation with Phase 2.6 Races Editor proficiency autocomplete.

#### 3.3 Error Handling & User Feedback

**Files**: All editors using autocomplete

**Action**:

- If selected entity ID does not exist in loaded data, show warning label:
  - `ui.colored_label(egui::Color32::YELLOW, "⚠ Item ID 42 not found in campaign data")`
- Add "Refresh Candidates" button to reload entity lists after importing new data
- Log autocomplete errors to console: `log::warn!("Failed to load item candidates: {}", e)`

**Error Patterns** (follow `thiserror`):

```rust
#[derive(Error, Debug)]
pub enum AutocompleteError {
    #[error("Failed to load entity candidates: {0}")]
    LoadError(String),

    #[error("Invalid entity ID: {0}")]
    InvalidId(String),
}
```

#### 3.4 Performance Optimization

**Target**: Autocomplete response time < 100ms for 200+ candidates

**Actions**:

- Cache candidate lists (don't regenerate on every frame)
- Invalidate cache only on data import/add/delete operations
- Use `egui_autocomplete` built-in filtering (already optimized)
- Profile with `cargo flamegraph` if performance issues occur

**Measurement**: Add debug timing logs during candidate loading

#### 3.5 Integration Testing

**Test Type**: Integration + Performance

**Test Cases**:

1. **Invalid ID Warning**: Reference non-existent ItemId, verify warning appears
2. **Proficiency Validation**: Enter invalid proficiency string, verify save error
3. **Cache Invalidation**: Add new monster, verify autocomplete candidates update
4. **Performance**: Load 200-item candidate list, verify dropdown renders in < 100ms
5. **Edge Case - Empty Data**: Load campaign with no items, verify autocomplete shows "(No items loaded)"

**Validation Commands**:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

#### 3.6 Deliverables

- [ ] Numeric fields use `DragValue` with range constraints (not autocomplete)
- [ ] Proficiency autocomplete in Classes Editor
- [ ] Error feedback for invalid entity references (warning labels)
- [ ] Candidate caching implemented (regenerate only on data changes)
- [ ] Performance profiling confirms < 100ms response time
- [ ] Edge case tests pass (empty data, invalid IDs, cache invalidation)
- [ ] All quality checks pass

#### 3.7 Success Criteria

- Invalid entity references show clear warning messages
- Numeric fields prevent out-of-range values via clamping
- Proficiency autocomplete suggests only valid enum values
- Autocomplete remains responsive with 200+ candidates
- User receives immediate feedback for validation errors (no silent failures)

---

### Phase 4: Documentation & Final Validation

**Goal**: Update documentation and verify full integration compliance.

#### 4.1 Update Implementation Documentation

**File**: `docs/explanation/implementations.md`

**Action**:

- Add section: "SDK Campaign Builder: Autocomplete Integration"
- Document:
  - Integrated `egui_autocomplete` library
  - Created reusable `AutocompleteInput` widget in `ui_helpers.rs`
  - Replaced manual ID entry in Monsters, Items, Classes, Characters, Spells, Map editors
  - Added entity reference validation with warning feedback
  - Implemented candidate caching for performance
- Include code examples of `AutocompleteInput` usage
- Note testing approach and validation results

**Template**:

```markdown
## SDK Campaign Builder: Autocomplete Integration (Phase X)

Integrated `egui_autocomplete` to improve data entry efficiency across all Campaign Builder editors...

**Changes**:

- Added `AutocompleteInput` widget in `ui_helpers.rs`
- Entity reference autocomplete in 9 editor contexts:
  - Monsters Editor: Monster name search
  - Items Editor: (self-references, if applicable)
  - Classes Editor: Item references, proficiencies
  - Characters Editor: Item references (inventory, equipment)
  - Spells Editor: Condition references
  - Map Editor: Condition references (traps), map references (teleports), NPC references, monster references (encounters), item references (treasure)
  - Races Editor: Proficiency references, item tag references, special ability references
  - Quest Editor: Monster/item/map/NPC references in objectives, quest references
  - Dialogue Editor: Quest/item references in conditions/actions
- Validation warnings for invalid entity IDs
- Performance: < 100ms response time for 200+ candidates

**Testing**: All quality checks pass, integration tests verify ID mapping correctness across all editor contexts.
```

#### 4.2 Update Module Documentation

**Files**: `sdk/campaign_builder/src/ui_helpers.rs`, editor files

**Action**:

- Add `AutocompleteInput` to module-level doc comment in `ui_helpers.rs`
- Update editor doc comments to mention autocomplete usage
- Ensure all public functions have doc comments with examples

**Validation**: `cargo doc --open` should show complete documentation with examples

#### 4.3 Architecture Compliance Verification

**Action**:

- Verify `AutocompleteInput` follows `AttributePairInput` patterns (consistent API)
- Confirm no architectural deviations introduced
- Check that entity ID types (`ItemId`, `MonsterId`, `ConditionId`) are used correctly (not raw `u8`/`u16`)

**Reference**: `docs/reference/architecture.md` Section 4 (Data Structures)

#### 4.4 Final Quality Gate

**Validation Commands** (ALL MUST PASS):

```bash
cd sdk/campaign_builder

# 1. Format code
cargo fmt --all

# 2. Compile check
cargo check --all-targets --all-features

# 3. Lint (zero warnings)
cargo clippy --all-targets --all-features -- -D warnings

# 4. Run tests
cargo test --all-features

# 5. Generate documentation
cargo doc --no-deps --open

# 6. Manual test in Campaign Builder GUI
cargo run --bin campaign-builder
```

**Expected Results**:

- ✅ `cargo fmt` - No output (all files formatted)
- ✅ `cargo check` - "Finished" with 0 errors
- ✅ `cargo clippy` - "Finished" with 0 warnings
- ✅ `cargo test` - All tests pass
- ✅ `cargo doc` - Documentation builds, `AutocompleteInput` appears in docs
- ✅ Manual test - Autocomplete works in at least 3 different editors

#### 4.5 Deliverables

- [ ] `docs/explanation/implementations.md` updated with autocomplete integration summary
- [ ] Module doc comments updated in `ui_helpers.rs`
- [ ] Editor doc comments reference autocomplete usage
- [ ] All quality checks pass (fmt, check, clippy, test, doc)
- [ ] Manual GUI testing confirms autocomplete works in production build
- [ ] Architecture compliance verified (no deviations from architecture.md)

#### 4.6 Success Criteria

- All cargo validation commands pass with zero errors/warnings
- Documentation accurately describes autocomplete integration
- Manual testing confirms autocomplete improves workflow efficiency
- No regressions in existing editor functionality
- Code follows AGENTS.md standards (SPDX headers, error handling, testing)

---

## Rollback Strategy

If critical issues arise during integration:

1. **Phase 1 Rollback**: Remove `egui_autocomplete` from `Cargo.toml`, delete `AutocompleteInput` from `ui_helpers.rs`
2. **Phase 2 Rollback**: Revert editor changes, restore original `TextEdit`/`ComboBox` widgets
3. **Phase 3 Rollback**: Remove validation warnings and caching logic

**Validation After Rollback**: Run full quality gate to ensure no partial changes remain.

---

## Open Questions

1. **Version Compatibility**: Is `egui_autocomplete` 0.7 definitively compatible with `egui` 0.33? (Verify on crates.io before Phase 1.1)
2. **Data Loading**: Should candidate loading be synchronous (blocking UI) or asynchronous (background thread)? Recommend synchronous for Phase 2, evaluate async if performance issues.
3. **Custom Filtering**: Should autocomplete support fuzzy matching (e.g., "dmg" matches "damage spell") or only prefix matching? Recommend prefix-only for Phase 2, add fuzzy as Phase 5 enhancement.
4. **Accessibility**: Does `egui_autocomplete` support keyboard navigation (arrow keys, Enter to select)? Verify in Phase 1.3 testing.

---

## Summary

This plan integrates autocomplete into the Campaign Builder SDK through four focused phases:

**Phase 1**: Core infrastructure (egui_autocomplete dependency, reusable AutocompleteInput widget)

**Phase 2**: Entity reference autocomplete across 9 editor contexts:

- Monsters Editor: Monster name search and references
- Classes Editor: Item references, proficiencies
- Characters Editor: Item references (inventory, equipment)
- Spells Editor: Condition references
- Map Editor: Condition references (traps), map references (teleports), NPC references, monster references (encounters), item references (treasure)
- Races Editor: Proficiency references, item tag references, special ability references
- Quest Editor: Monster/item/map/NPC references in objectives
- Dialogue Editor: Quest/item references in conditions/actions
- Map Events: Additional map and NPC reference autocomplete

**Phase 3**: Validation, error handling, performance optimization, and polish

**Phase 4**: Documentation and final validation

**Coverage**: This plan now covers ALL editors in the Campaign Builder SDK that require entity reference autocomplete, ensuring a complete and consistent implementation across the entire application. The comprehensive scope includes monster, item, condition, proficiency, map, NPC, and quest references across all editing contexts.

The phased approach allows incremental testing and rollback while delivering immediate value to campaign designers.
Each phase includes specific deliverables, testing requirements, and success criteria to ensure quality and compliance with AGENTS.md standards.
