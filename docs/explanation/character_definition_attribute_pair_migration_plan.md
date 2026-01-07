# CharacterDefinition AttributePair Migration Plan - ✅ COMPLETED

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

### Phase 3: SDK Updates ✅ COMPLETED

**Status**: ✅ COMPLETED
**Date**: 2025-01-XX
**Effort**: ~4 hours
**Documentation**: [phase3_sdk_updates_completed.md](phase3_sdk_updates_completed.md)

#### 3.1 Update Characters Editor

**File**: [characters_editor.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/characters_editor.rs)

Modify `CharacterEditBuffer` and stats editing UI:

- ✅ Show both `base` and `current` stat fields by default for all stats
- ✅ Default behavior: when creating new character, current = base
- ✅ Users can explicitly set current ≠ base for pre-buffed/debuffed templates
- ✅ Updated `CharacterEditBuffer` with separate base/current fields for all stats
- ✅ Updated UI to 6-column grid showing Base/Current pairs

#### 3.2 Update Validation

**File**: `sdk/campaign_builder/src/validation.rs`

Add validation for:

- ✅ Missing `base` or `current` values → **Error** (required fields)
- ✅ `current > base` → **Error** (indicates invalid data, buffs should be applied at runtime)
- ✅ HP `current > base` → **Error**

#### 3.3 Update Asset Manager

**File**: [asset_manager.rs](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/asset_manager.rs)

✅ Updated character loading/saving to use new types.

#### 3.4 Testing Requirements

- ✅ Unit tests for `CharacterEditBuffer` with new stats types
- ✅ Integration tests for save/load round-trip
- ✅ All 882 SDK tests updated and passing

#### 3.5 Deliverables

- ✅ `CharacterEditBuffer` uses `Stats` instead of `BaseStats`
- ✅ UI supports base+current editing (6-column grid layout)
- ✅ Validation rules updated (current ≤ base enforced)
- ✅ Asset manager updated
- ✅ Display trait implemented for AttributePair types
- ✅ All deprecated types removed from SDK
- ✅ HP override uses AttributePair16

**Test Results**: 1,152 tests executed, 1,152 passed ✅

---

### Phase 4: Documentation and Cleanup - ✅ COMPLETED

#### 4.1 Update Architecture Documentation

**File**: `docs/reference/architecture.md`

Update `CharacterDefinition` documentation to reflect `Stats` usage.

**Status**: ✅ COMPLETE

- Removed deprecated `BaseStats` struct documentation
- Updated `CharacterDefinition` to show `Stats` and `hp_override: Option<AttributePair16>`
- Updated `instantiate()` flow to reflect AttributePair usage
- Added documentation for backward-compatible formats

#### 4.2 Remove Deprecated Types

After migration verification:

- Remove `BaseStats` struct
- Remove deprecated field aliases
- Clean up migration serde helpers if no longer needed

**Status**: ✅ COMPLETE

- `BaseStats` struct removed from `src/domain/character_definition.rs`
- All deprecated `BaseStats` tests removed (4 tests)
- Migration helper `CharacterDefinitionDef` retained for backward compatibility (to be removed in future after extended verification period)

#### 4.3 Update Lesson Learned

**File**: `docs/explanation/lessons_learned.md`

Document the migration pattern for future reference.

**Status**: ✅ COMPLETE

- Added new section "5. AttributePair Migration Pattern"
- Documented 4-phase migration strategy
- Provided complete implementation example with backward compatibility
- Updated existing CharacterDefinition example to use `Stats` and `hp_override`
- Documented validation considerations and key benefits

#### 4.4 Deliverables

- [x] Architecture docs updated
- [x] `BaseStats` removed from codebase
- [x] Lessons learned documented
- [x] Migration pattern documented for future reference

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

## Verification Plan - ✅ COMPLETED

All verification steps completed successfully:

### Quality Gates ✅

```bash
✅ cargo fmt --all                                      # Passed
✅ cargo check --all-targets --all-features             # Passed
✅ cargo clippy --all-targets --all-features -- -D warnings  # Passed (0 warnings)
✅ cargo nextest run --all-features                     # Passed (1,148/1,148 tests)
```

### Architecture Compliance ✅

- [x] Data structures match architecture.md Section 4 definitions
- [x] Module placement follows Section 3.2 structure
- [x] Type aliases used consistently (ItemId, SpellId, CharacterDefinitionId, etc.)
- [x] Constants extracted, not hardcoded
- [x] AttributePair pattern used for modifiable stats
- [x] RON format used for data files
- [x] No architectural deviations without documentation

### Backward Compatibility ✅

- [x] Existing campaign RON files load correctly
- [x] Simple format (numbers) supported for stats
- [x] Full format (base/current pairs) supported for stats
- [x] Legacy hp_base/hp_current fields converted correctly
- [x] Migration helper CharacterDefinitionDef working as expected

---

## Final Summary

**Migration Completed**: 2025-01-24

**All Phases Complete**:

- Phase 1: Domain layer migrated to Stats with AttributePair
- Phase 2: Campaign data verified compatible, no changes required
- Phase 3: SDK (Campaign Builder) updated with editor support for base/current
- Phase 4: Deprecated code removed, documentation updated

**Test Results**:

- Total tests: 1,148
- Passed: 1,148 (100%)
- Failed: 0
- Clippy warnings: 0

**Backward Compatibility**: Maintained via CharacterDefinitionDef migration helper (to be removed after verification period)

**Documentation**:

- [x] Architecture.md updated
- [x] Lessons learned documented
- [x] Migration pattern documented for future reference
- [x] Phase 4 completion summary created

**Next Steps**: Monitor for issues during verification period before removing CharacterDefinitionDef migration helper in a future release.

---

## Original Verification Plan

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
