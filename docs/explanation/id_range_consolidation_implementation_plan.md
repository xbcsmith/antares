# ID Range Consolidation Implementation Plan

## Overview

The project maintains two parallel numeric ID spaces — game-data entity IDs and
visual/mesh registry IDs — across eight registries, but has no single
authoritative definition of where each space starts and ends.  Only one registry
has named range constants (`LANDSCAPE_MESH_ID_MIN = 11_000`).  The remaining
boundaries are either absent, locally redefined inside individual functions, or
outright wrong (`ITEM_MESH_ID_MIN` is hardcoded `4000` in `obj_importer_ui.rs`
instead of the correct `9000`).  This plan consolidates all boundaries into
`src/domain/types.rs`, wires them through the SDK, adds importer validation, and
updates `architecture.md` section 4.6 to match.

The authoritative current-state reference is
`docs/reference/monster_creature_mapping_reference.md` — ID Space Allocation
section.

---

## Current State Analysis

### Existing Infrastructure

| Symbol | Location | Value | Status |
|--------|----------|-------|--------|
| `LANDSCAPE_MESH_ID_MIN` | `src/domain/types.rs` L119 | `11_000` | ✅ Correct |
| `LANDSCAPE_ID_MIN` | `src/domain/types.rs` L92 | `1` | ✅ Correct |
| `DialogueId` | `src/domain/dialogue.rs` L22 | `u16` | ✅ Exists — **not** in `types.rs` |
| `FURNITURE_MESH_ID_MIN` | `sdk/.../obj_importer_ui.rs` L553 | `10001` (local `const`) | ⚠️ Not exported |
| `ITEM_MESH_ID_MIN` | `sdk/.../obj_importer_ui.rs` L595 | `4000` (local `const`) | ❌ **Bug** — should be `9000` |
| `creature_id` default | `sdk/.../obj_importer.rs` L497 | `4000` (literal) | ⚠️ No constant |
| `furniture_id` default | `sdk/.../obj_importer.rs` L498 | `10001` (literal) | ⚠️ No constant |
| `CreatureCategory::Custom` range | `sdk/.../creature_id_manager.rs` L94 | `4000..u32::MAX` | ❌ No upper bound — bleeds into item mesh space (9000+) |

### Identified Issues

1. **Bug**: `suggest_next_item_mesh_id_from_dir` in `obj_importer_ui.rs` starts item
   mesh IDs at `4000` instead of `9000`.  This causes item mesh IDs to collide with
   the Custom creature range.
2. **Missing constants**: No named constants for item mesh (9000), furniture mesh
   (10000/10001), object mesh (12000), or any `*_MAX` boundary — even for
   landscape.
3. **Unbounded Custom range**: `CreatureCategory::Custom` uses `4000..u32::MAX`,
   allowing IDs to silently overflow into the item mesh (9000+), furniture (10000+),
   and landscape (11000+) ranges.
4. **`DialogueId` not in `types.rs`**: `pub type DialogueId = u16` lives in
   `src/domain/dialogue.rs`, not consolidated with the other type aliases in
   `src/domain/types.rs`.
5. **No importer range validation**: Importers do not check that an exported ID
   falls within its registry's declared range before writing.
6. **Architecture.md section 4.6** is a sparse code block — missing the full ID
   table now documented in the reference file.

---

## Implementation Phases

### Phase 1: Define All Range Constants in `src/domain/types.rs`

This is the foundation phase.  All later phases depend on these constants being
exported from the game crate.

#### 1.1 Foundation Work

Add the following constants to `src/domain/types.rs`, grouped below the existing
`LANDSCAPE_MESH_ID_MIN` constant.  Add `///` doc comments and `# Examples` blocks
for each constant per the coding standards.

**Creature visual ID boundaries** (replace hardcoded range literals everywhere):

- `CREATURE_MONSTER_ID_MIN: u32 = 1`
- `CREATURE_MONSTER_ID_MAX: u32 = 999`
- `CREATURE_NPC_ID_MIN: u32 = 1000`
- `CREATURE_NPC_ID_MAX: u32 = 1999`
- `CREATURE_TEMPLATE_ID_MIN: u32 = 2000`
- `CREATURE_TEMPLATE_ID_MAX: u32 = 2999`
- `CREATURE_VARIANT_ID_MIN: u32 = 3000`
- `CREATURE_VARIANT_ID_MAX: u32 = 3999`
- `CREATURE_CUSTOM_ID_MIN: u32 = 4000`
- `CREATURE_CUSTOM_ID_MAX: u32 = 8999`  ← **new upper bound, not previously enforced**

**Mesh registry ID boundaries** (three registries currently have no constants):

- `ITEM_MESH_ID_MIN: u32 = 9000`
- `ITEM_MESH_ID_MAX: u32 = 9999`
- `FURNITURE_MESH_ID_MIN: u32 = 10000`
- `FURNITURE_MESH_ID_MAX: u32 = 10999`
- `LANDSCAPE_MESH_ID_MAX: u32 = 11999`  ← completes the existing `LANDSCAPE_MESH_ID_MIN`
- `OBJECT_MESH_ID_MIN: u32 = 12000`
- `OBJECT_MESH_ID_MAX: u32 = 12999`

> **Note on furniture**: The existing code defaults `furniture_id` to `10001`
> (i.e. `FURNITURE_MESH_ID_MIN + 1`), treating `10000` as a reserved sentinel.
> `FURNITURE_MESH_ID_MIN` is defined as `10000` (clean range start, consistent
> with all other ranges).  All downstream code that previously used the bare
> literal `10001` is updated to `FURNITURE_MESH_ID_MIN + 1` explicitly.

#### 1.2 Consolidate `DialogueId` into `types.rs`

`pub type DialogueId = u16` currently lives in `src/domain/dialogue.rs`.
Move the definition to `src/domain/types.rs` alongside the other type aliases,
**changing the underlying type to `u32`**, then re-export it from
`src/domain/dialogue.rs` with `pub use crate::domain::types::DialogueId;` to
preserve all downstream imports.

The type change from `u16` to `u32` requires:
- Update all `as DialogueId` casts and any `u16`-specific operations in
  `src/domain/dialogue.rs`, `src/application/dialogue.rs`, and the SDK dialogue
  editors to use `u32`-compatible equivalents.
- RON data files store dialogue IDs as plain integers; RON deserialises these
  without a type tag, so **no data file changes are needed** — existing `1`–`1004`
  values deserialise correctly into `u32`.
- Update doctests that cast `as DialogueId` where the cast now widens to `u32`.

#### 1.3 Integration

Run a project-wide `grep` for `use.*domain::dialogue::DialogueId` and
`as DialogueId` after the re-export is in place, verify all callers compile,
and confirm no `u16`-specific operations (`u16::MAX`, narrow arithmetic) remain.

#### 1.4 Testing Requirements

- Unit tests for every new constant asserting its exact value
  (`test_{CONST_NAME}_has_expected_value` naming convention).
- Verify `CREATURE_CUSTOM_ID_MAX < ITEM_MESH_ID_MIN` in a static-assertion-style
  test.
- Verify `LANDSCAPE_MESH_ID_MIN <= LANDSCAPE_MESH_ID_MAX` and similar ordering
  checks for each adjacent pair of ranges.
- Doctest for `DialogueId` showing a cast to `u32` and comparison.
- Test asserting `DialogueId::MAX == u32::MAX` (i.e. the alias is `u32`, not `u16`).

#### 1.5 Deliverables

- [ ] All `CREATURE_*`, `ITEM_MESH_*`, `FURNITURE_MESH_*`, `LANDSCAPE_MESH_*_MAX`,
      and `OBJECT_MESH_*` constants exported from `src/domain/types.rs`
- [ ] `DialogueId` defined in `src/domain/types.rs`; re-exported from `src/domain/dialogue.rs`
- [ ] All constants have `///` doc comments with `# Examples` doctests
- [ ] Unit tests for all new constants pass
- [ ] `cargo check --all-targets --all-features` clean
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` clean

#### 1.6 Success Criteria

`grep -r "10001\|10000\|9000\|4000" src/domain/types.rs` shows only the new
constant definitions — zero bare numeric literals remain for these values.

---

### Phase 2: Fix the `ITEM_MESH_ID_MIN` Bug and Wire Constants into the SDK

This phase eliminates the concrete collision bug and replaces all remaining bare
literals in the SDK with the constants defined in Phase 1.

#### 2.1 Fix the Item Mesh ID Bug

In `sdk/campaign_builder/src/obj_importer_ui.rs`,
`suggest_next_item_mesh_id_from_dir` has a locally-declared
`const ITEM_MESH_ID_MIN: CreatureId = 4000`.  Replace it with
`antares::domain::types::ITEM_MESH_ID_MIN` (value `9000`).

Update the three tests that assert `suggest_next_item_mesh_id_from_dir` returns
`4000` — they should now assert `9000`.

#### 2.2 Replace Bare Literals with Constants

| File | Current literal | Replacement constant |
|------|-----------------|---------------------|
| `sdk/.../obj_importer_ui.rs` L553 | `10001` (local `FURNITURE_MESH_ID_MIN`) | `antares::domain::types::FURNITURE_MESH_ID_MIN + 1` |
| `sdk/.../obj_importer.rs` L497 | `4000` (`creature_id` default) | `antares::domain::types::CREATURE_CUSTOM_ID_MIN` |
| `sdk/.../obj_importer.rs` L498 | `10001` (`furniture_id` default) | `antares::domain::types::FURNITURE_MESH_ID_MIN + 1` |
| `sdk/.../creature_id_manager.rs` L94 | `4000..u32::MAX` | `CREATURE_CUSTOM_ID_MIN..=CREATURE_CUSTOM_ID_MAX` (see §2.3) |

#### 2.3 Cap `CreatureCategory::Custom` Upper Bound

In `sdk/campaign_builder/src/creature_id_manager.rs`, `CreatureCategory::id_range`
returns `4000..u32::MAX` for `Custom`.  Change it to
`CREATURE_CUSTOM_ID_MIN..=(CREATURE_CUSTOM_ID_MAX + 1)` (exclusive upper bound to
keep the `Range<u32>` type) using the imported constants.

Also update `CreatureCategory::from_id`: the wildcard arm `_ =>
CreatureCategory::Custom` implicitly catches IDs ≥ 9000.  Add an explicit match
arm for `4000..=8999 => CreatureCategory::Custom` and keep the wildcard as a
fallthrough that still returns `Custom` (existing behavior preserved, but the
intent is now explicit).

Update the existing test `test_category_id_range` (L709–716): the assertion
`CreatureCategory::Custom.id_range().contains(&10000)` should now **fail** and
must be replaced with an assertion that `10000` is **not** in the Custom range.

#### 2.4 Testing Requirements

- Existing `suggest_next_item_mesh_id_*` tests updated to assert `9000` baseline.
- New test: `test_creature_custom_range_does_not_overlap_item_mesh_range` —
  asserts `CREATURE_CUSTOM_ID_MAX < ITEM_MESH_ID_MIN`.
- All modified tests pass; no new test failures introduced.

#### 2.5 Deliverables

- [ ] `suggest_next_item_mesh_id_from_dir` returns `9000` for empty registry
- [ ] `obj_importer` defaults use named constants (no bare `4000` or `10001` literals)
- [ ] `CreatureCategory::Custom` range capped at `CREATURE_CUSTOM_ID_MAX` (8999)
- [ ] All tests pass

#### 2.6 Success Criteria

`grep -rn "4000\|10001" sdk/campaign_builder/src/` returns zero hits inside
function bodies (constants-only file excluded).

---

### Phase 3: Add SDK Range-Boundary Validation to Mesh Importers

This phase adds proactive validation so the importer rejects an export before it
can write an out-of-range ID to a registry.

#### 3.1 Validation Work

In `sdk/campaign_builder/src/obj_importer_ui.rs`, inside
`export_state_to_campaign` (and `export_state_to_campaign_with_landscape_file`),
add an ID range check immediately before the registry write for each
`ExportType`:

| `ExportType` | ID field | Valid range |
|--------------|----------|-------------|
| `ExportType::Item` | `state.creature_id` | `ITEM_MESH_ID_MIN..=ITEM_MESH_ID_MAX` |
| `ExportType::Furniture` | `state.furniture_id` | `FURNITURE_MESH_ID_MIN..=FURNITURE_MESH_ID_MAX` |
| `ExportType::Landscape` | `state.landscape_mesh_id` | `LANDSCAPE_MESH_ID_MIN..=LANDSCAPE_MESH_ID_MAX` |
| `ExportType::ObjectMesh` | (pending Phase 4) | `OBJECT_MESH_ID_MIN..=OBJECT_MESH_ID_MAX` |

Return `Err(ObjImporterExportError::IdOutOfRange { id, export_type, range })` (a
new variant) if the check fails.  Display the error in the importer UI status bar
via the existing `state.status_message` / `state.is_error` fields.

`ExportType::Creature` delegates to `CreatureIdManager::validate_id`, which
already performs range checking — no change needed there.

#### 3.2 Update Error Type

Add variant to `ObjImporterExportError` in `obj_importer_ui.rs`:

```
IdOutOfRange { id: u32, export_type: ExportType, valid_range: std::ops::RangeInclusive<u32> }
```

The `Display` impl should produce a message like:
`"Item mesh ID 9999 is out of range 9000..=9999"`.

#### 3.3 Testing Requirements

- One test per `ExportType` variant asserting that an out-of-range ID returns
  `Err(ObjImporterExportError::IdOutOfRange { … })`.
- One test per variant asserting that a boundary-edge ID (e.g. `ITEM_MESH_ID_MAX`)
  succeeds.
- Tests follow `test_{function}_{condition}_{expected}` naming.

#### 3.4 Deliverables

- [ ] `ObjImporterExportError::IdOutOfRange` variant added
- [ ] Range check applied to `Item`, `Furniture`, and `Landscape` export paths
- [ ] All new validation tests pass
- [ ] `cargo clippy` clean

#### 3.5 Success Criteria

Attempting to export a furniture mesh with ID `11000` (landscape range) produces
an `Err` with a descriptive message and sets `is_error = true` in importer state.

---

### Phase 4: Update `architecture.md` Section 4.6

This phase makes the architecture document the single authoritative written
statement of every numeric ID range.

#### 4.1 Documentation Work

In `docs/reference/architecture.md` section 4.6 ("Supporting Types"), replace the
current sparse code block with:

1. A prose paragraph explaining the two domains (game-data IDs vs. visual/mesh
   registry IDs) and where to find the authoritative constants
   (`src/domain/types.rs`).
2. **Table: Visual / Mesh Registry IDs** — mirrors the table in
   `docs/reference/monster_creature_mapping_reference.md` ID Space Allocation
   section, listing range, registry file, Rust constant name, and status.
3. **Table: Game-Data Entity IDs** — mirrors the existing table in the reference
   doc (Items, Monsters, Spells, Maps, Dialogues, etc.).
4. Keep the existing type-alias code block but add `DialogueId` and the new
   constants to it.

Cross-link section 4.6 ↔ `docs/reference/monster_creature_mapping_reference.md`
with explicit `See also` lines in both directions.

#### 4.2 Deliverables

- [ ] Section 4.6 contains full Visual/Mesh Registry ID table
- [ ] Section 4.6 contains full Game-Data Entity ID table
- [ ] Type-alias code block includes `DialogueId` and new mesh constants
- [ ] Cross-links to `monster_creature_mapping_reference.md` added
- [ ] `docs/explanation/implementations.md` updated with a summary of this work

#### 4.3 Success Criteria

A developer can determine the correct ID range for any registry by reading
architecture.md section 4.6 alone, without consulting any other file.

---

### Phase 5: Object Mesh Registry Validation (Deferred — Depends on Separate Plan)

This phase is **blocked** on the separate
**"Refactor `object_mesh_registry.ron` to ID-Based Format"** plan completing
first.  It is listed here for completeness only.

Once `object_mesh_registry.ron` is converted from string keys to numeric IDs in
the 12000–12999 range:

#### 5.1 Work

- Add `ExportType::ObjectMesh` range validation in `export_state_to_campaign`
  (same pattern as Phase 3) using `OBJECT_MESH_ID_MIN..=OBJECT_MESH_ID_MAX`.
- Confirm the object mesh importer assigns IDs starting at `OBJECT_MESH_ID_MIN`.

#### 5.2 Deliverables

- [ ] Object mesh export validates against 12000–12999
- [ ] Object mesh registry refactor plan completed first
- [ ] All tests pass

---

## Affected Files Summary

| File | Phase | Change |
|------|-------|--------|
| `src/domain/types.rs` | 1 | Add all `CREATURE_*`, `ITEM_MESH_*`, `FURNITURE_MESH_*`, `LANDSCAPE_MESH_ID_MAX`, `OBJECT_MESH_*` constants; add `DialogueId` |
| `src/domain/dialogue.rs` | 1 | Re-export `DialogueId` from `types.rs`; remove local definition |
| `sdk/campaign_builder/src/obj_importer_ui.rs` | 2, 3 | Fix `ITEM_MESH_ID_MIN` bug; replace local `FURNITURE_MESH_ID_MIN`; add export range validation; new `IdOutOfRange` error variant |
| `sdk/campaign_builder/src/obj_importer.rs` | 2 | Replace bare `4000` and `10001` defaults with constants |
| `sdk/campaign_builder/src/creature_id_manager.rs` | 2 | Cap `Custom` range at `CREATURE_CUSTOM_ID_MAX`; use constants in `id_range` and `from_id` |
| `sdk/campaign_builder/src/campaign_io.rs` | 2 | Replace any remaining bare ID literals in importer sync with constants |
| `docs/reference/architecture.md` | 4 | Update section 4.6 with full ID tables and type-alias block |
| `docs/explanation/implementations.md` | 4 | Add implementation summary |

---

## Decisions Log

| Question | Decision |
|----------|----------|
| `FURNITURE_MESH_ID_MIN` value | `10000` — clean range start; downstream code uses `FURNITURE_MESH_ID_MIN + 1` for first assignment |
| `DialogueId` underlying type | `u32` — moved to `types.rs`; re-exported from `dialogue.rs`; no RON data changes needed |
| Phase ordering | As written — Phase 1 → 2 → 3 → 4 → 5 |
