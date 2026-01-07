# CharacterDefinition AttributePair Migration Plan

## Overview

This plan migrates `CharacterDefinition` to store `AttributePair`/`AttributePair16` for stats instead of plain `u8`/`u16` values. This enables base+current value tracking for character template stats, providing consistency with the runtime `Character` type.

**Scope**: Game engine domain types, serialization, instantiation, SDK editors, campaign data files, tests, and documentation.

## Current State Analysis

### Existing Infrastructure

| Component              | Current Type                  | Location                                                                                                                        |
| ---------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `BaseStats`            | Uses plain `u8` values        | [character_definition.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/character_definition.rs#L295-L310) |
| `Stats` (runtime)      | Uses `AttributePair`          | [character.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/character.rs#L288-L304)                       |
| `hp_base`/`hp_current` | Separate `Option<u16>` fields | [character_definition.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/character_definition.rs#L447-L454) |
| SDK editor             | Edits plain `u8` stat values  | [characters_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/characters_editor.rs)   |
| Campaign data          | `base_stats.(might: 15, ...)` | [characters.ron](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/campaigns/tutorial/data/characters.ron#L9-L17)         |

### Problem Statement

1. **Inconsistency**: `CharacterDefinition.base_stats` uses plain values; runtime `Character.stats` uses `AttributePair`
2. **Limited flexibility**: Cannot define pre-buffed/debuffed template characters
3. **Redundant HP fields**: `hp_base` and `hp_current` are separate optional fields instead of unified `AttributePair16`

---

## Implementation Phases

### Phase 1: Domain Type Changes

#### 1.1 Deprecate `BaseStats`, Use `Stats` Directly

**File**: [character_definition.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/character_definition.rs)

Replace `BaseStats` struct with direct use of `Stats` (which already uses `AttributePair`):

- Mark `BaseStats` as `#[deprecated]` initially for migration period
- Change `CharacterDefinition.base_stats: BaseStats` → `base_stats: Stats`
- Update `CharacterDefinition::new()` to initialize with `Stats::new()`
- Update `validate()` to work with `Stats` type

#### 1.2 Consolidate HP Fields

Replace `hp_base: Option<u16>` and `hp_current: Option<u16>` with:

- `hp_override: Option<AttributePair16>` — unified base+current override
- Maintain backward compatibility via custom deserialization

#### 1.3 Add Migration Serde Support

Add custom deserialization to support both formats:

```rust
// Old format (backward compatible)
base_stats: (might: 15, intellect: 10, ...)

// New format (explicit base+current)
base_stats: (might: (base: 15, current: 15), ...)
```

The existing `AttributePairDef` enum (line 197) already supports this via `#[serde(untagged)]`.

#### 1.4 Update `instantiate()` Method

Modify `CharacterDefinition::instantiate()` (line 718) to:

- Copy `base_stats` directly to `Character.stats` instead of calling `BaseStats.to_stats()`
- Apply race modifiers to the copied stats
- Use `hp_override.base` and `hp_override.current` if present

#### 1.5 Testing Requirements

- Unit tests for `Stats` serialization/deserialization (both formats)
- Unit tests for `hp_override` migration
- Update existing `BaseStats` tests → `Stats` tests
- Test backward compatibility with old RON format

#### 1.6 Deliverables

- [ ] `CharacterDefinition.base_stats` changed to `Stats` type
- [ ] `hp_base`/`hp_current` replaced with `hp_override: Option<AttributePair16>`
- [ ] Backward-compatible deserialization
- [ ] Updated unit tests

#### 1.7 Success Criteria

- `cargo check --all-targets --all-features` passes
- All existing tests pass (backward compatibility verified)
- New stats deserialization tests pass

---

### Phase 2: Campaign Data Migration ✅ COMPLETED

#### 2.1 Migrate Tutorial Campaign

**File**: [characters.ron](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/campaigns/tutorial/data/characters.ron)

Convert 9 character definitions from old to new format. Example:

```diff
-base_stats: (
-    might: 15,
-    intellect: 10,
-),
+base_stats: (
+    might: 15,  // Shorthand: expands to (base: 15, current: 15)
+    intellect: 10,
+),
```

> [!NOTE]
> No actual syntax change needed for most entries since `AttributePair` accepts simple values via `untagged` deserialization. Only update if explicit `current` differs from `base`.

#### 2.2 Migrate Core Data

**File**: `data/characters.ron` (if exists)

Apply same migration as tutorial campaign.

#### 2.3 Testing Requirements

- Integration test loading migrated RON files
- Validation test ensuring instantiated characters have correct stats

#### 2.4 Deliverables

- [x] `campaigns/tutorial/data/characters.ron` verified compatible (9 characters)
- [x] `data/characters.ron` verified compatible (6 characters)
- [x] Integration tests pass (9 new tests, all passing)
- [x] Example formats file created (`data/examples/character_definition_formats.ron`)
- [x] Content author guide created (`docs/how-to/character_definition_ron_format.md`)

#### 2.5 Phase 2 Results

**Status**: ✅ COMPLETED

**Key Findings**:

- All existing campaign data loads without modification (backward compatibility confirmed)
- Tutorial campaign: 9 characters using simple format + old `hp_base` → works perfectly
- Core data: 6 characters using simple format (no HP override) → works perfectly
- All 15 campaign characters instantiate successfully with correct stats and HP

**Tests Added**:

1. `test_phase2_tutorial_campaign_loads()` - verifies 9 tutorial characters load
2. `test_phase2_tutorial_campaign_hp_override()` - verifies `hp_base` → `hp_override` conversion
3. `test_phase2_tutorial_campaign_stats_format()` - verifies simple stats format
4. `test_phase2_core_campaign_loads()` - verifies 6 core characters load
5. `test_phase2_core_campaign_stats_format()` - verifies core stats deserialization
6. `test_phase2_campaign_instantiation()` - verifies instantiation with correct values
7. `test_phase2_all_tutorial_characters_instantiate()` - validates all 9 tutorial characters
8. `test_phase2_all_core_characters_instantiate()` - validates all 6 core characters
9. `test_phase2_stats_roundtrip_preserves_format()` - verifies simple/full format roundtrip
10. `test_phase2_example_formats_file_loads()` - verifies example file with all formats

**Documentation Created**:

- `data/examples/character_definition_formats.ron` - comprehensive examples of all supported formats
- `docs/how-to/character_definition_ron_format.md` - content author guide with best practices
- Updated `docs/explanation/implementations.md` with Phase 2 completion summary

**Test Results**: 1,152 tests executed, 1,152 passed ✅

---

### Phase 3: SDK Updates

#### 3.1 Update Characters Editor

**File**: [characters_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/characters_editor.rs)

Modify `CharacterEditBuffer` and stats editing UI:

- Show both `base` and `current` stat fields by default for all stats
- Default behavior: when creating new character, current = base
- Users can explicitly set current ≠ base for pre-buffed/debuffed templates

#### 3.2 Update Validation

**File**: `sdk/campaign_builder/src/validation.rs`

Add validation for:

- Missing `base` or `current` values → **Error** (required fields)
- `current > base` → **Error** (indicates invalid data, buffs should be applied at runtime)
- HP `current > base` → **Error**

#### 3.3 Update Asset Manager

**File**: [asset_manager.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/asset_manager.rs)

Update character loading/saving to use new types.

#### 3.4 Testing Requirements

- Unit tests for `CharacterEditBuffer` with new stats types
- Integration tests for save/load round-trip

#### 3.5 Deliverables

- [ ] `CharacterEditBuffer` uses `Stats` instead of `BaseStats`
- [ ] UI supports base+current editing (advanced mode)
- [ ] Validation rules updated
- [ ] Asset manager updated

---

### Phase 4: Documentation and Cleanup

#### 4.1 Update Architecture Documentation

**File**: `docs/reference/architecture.md`

Update `CharacterDefinition` documentation to reflect `Stats` usage.

#### 4.2 Remove Deprecated Types

After migration verification:

- Remove `BaseStats` struct
- Remove deprecated field aliases
- Clean up migration serde helpers if no longer needed

#### 4.3 Update Lesson Learned

**File**: `docs/explanation/lessons_learned.md`

Document the migration pattern for future reference.

#### 4.4 Deliverables

- [ ] Architecture docs updated
- [ ] `BaseStats` removed (Phase 4 only, after verification)
- [ ] Lessons learned documented

---

## Timing Recommendation

> [!IMPORTANT] > **Implement BEFORE [game_engine_fixes_implementation_plan.md](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/docs/explanation/game_engine_fixes_implementation_plan.md)**

### Rationale

1. **Reduces conflicts**: Game engine fixes modify `input.rs`, `events.rs`, and `hud.rs` — not character definition types. However, both plans touch test infrastructure.

2. **Foundation first**: The AttributePair change is a domain-layer change. Game engine fixes are application-layer changes. Following the standard dependency order (domain → application → game) is cleaner.

3. **Test stability**: Character definition tests will be modified. Running game engine fix tests after this migration ensures they test the correct types.

4. **Independent scope**: Game engine fixes (doors, NPCs, signs, teleports, HUD) don't depend on how stats are stored. They can proceed unaffected after this migration completes.

### Alternative: After Game Engine Fixes

If you prefer to see gameplay improvements first:

- **Pro**: Visible progress faster (doors work, NPCs interact)
- **Con**: Potential test merge conflicts later
- **Con**: May need to revisit character-related tests twice

---

## Verification Plan

### Automated Tests

```bash
# Run full test suite
cargo test --workspace --all-features

# Run character-specific tests
cargo test -p antares --lib character
cargo test -p antares --lib character_definition

# Run SDK tests
cargo test -p campaign_builder --lib

# Run integration tests
cargo test --test data_validation_tests
cargo test --test phase14_campaign_integration_test
```

### Manual Verification

1. Load Campaign Builder SDK: `cargo run -p campaign_builder`
2. Open tutorial campaign
3. Navigate to Characters tab
4. Verify existing characters display correctly
5. Edit a character's stats
6. Save and verify `characters.ron` format

---

## File Summary

| File                                            | Action | Description                                             |
| ----------------------------------------------- | ------ | ------------------------------------------------------- |
| `src/domain/character_definition.rs`            | Modify | Replace `BaseStats` with `Stats`, consolidate HP fields |
| `src/domain/mod.rs`                             | Modify | Update exports                                          |
| `campaigns/tutorial/data/characters.ron`        | Verify | Ensure backward compatibility                           |
| `sdk/campaign_builder/src/characters_editor.rs` | Modify | Update to use `Stats` type                              |
| `sdk/campaign_builder/src/asset_manager.rs`     | Modify | Update character loading                                |
| `sdk/campaign_builder/src/validation.rs`        | Modify | Add stat validation rules                               |
| `docs/reference/architecture.md`                | Modify | Update CharacterDefinition docs                         |
| `docs/explanation/next_plans.md`                | Modify | Mark this item complete                                 |

## Dependencies

- **None blocking**: This change is foundational and has no prerequisites
- **Blocked by this**: None identified

## Design Decisions (Resolved)

| Question                | Decision                                                               |
| ----------------------- | ---------------------------------------------------------------------- |
| Current stat editing UI | Show base+current fields **by default** for all stats                  |
| Validation strictness   | Missing base/current is an **error**; `current > base` is an **error** |
