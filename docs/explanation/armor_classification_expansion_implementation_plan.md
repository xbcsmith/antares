# ArmorClassification Expansion Implementation Plan

## Overview

The `ArmorClassification` enum in `src/domain/items/types.rs` currently has
four variants (`Light`, `Medium`, `Heavy`, `Shield`). The `Equipment` struct
has `helmet` and `boots` slots but no corresponding `ArmorClassification`
values, so slot routing falls through to item `tags` as a workaround. This plan
adds `Helmet` and `Boots` variants, routes them through the proficiency and
equip validation layers, migrates all RON item data, and extends AC calculation
to sum contributions from all four armor-bearing slots (body, shield, helmet,
boots).

Because the existing RON data relies on `tags` for helmet/boot routing and this
change is not backward compatible, all data files must be migrated atomically
in the same phase as the enum change.

## Current State Analysis

### Existing Infrastructure

- `ArmorClassification` in [src/domain/items/types.rs](../../src/domain/items/types.rs)
  has variants `Light`, `Medium`, `Heavy`, `Shield`.
- `ArmorData` carries `ac_bonus: u8`, `weight: u16`, `classification: ArmorClassification`.
- `ProficiencyDatabase::proficiency_for_armor` in
  [src/domain/proficiency.rs](../../src/domain/proficiency.rs) maps the four
  existing variants to string IDs (`"light_armor"`, `"medium_armor"`,
  `"heavy_armor"`, `"shield"`).
- `has_slot_for_item` in
  [src/domain/items/equipment_validation.rs](../../src/domain/items/equipment_validation.rs)
  matches `ItemType::Armor(_)` and returns `true` unconditionally — it does not
  distinguish which `equipment` slot to target.
- `Equipment` in [src/domain/character.rs](../../src/domain/character.rs) has
  `weapon`, `armor`, `shield`, `helmet`, `boots`, `accessory1`, `accessory2`
  fields. No existing code maps `ArmorClassification` to the `helmet` or `boots`
  slots.
- `Character.ac: AttributePair` stores the character's current/base AC but
  there is no function that computes effective AC from equipped armor pieces.
  Combat reads `c.ac.current` directly (set at character creation / service
  application).
- `src/sdk/validation.rs` contains equipment validation but has no checks
  specific to `Helmet` or `Boots` slot integrity.
- `data/items.ron` and `campaigns/tutorial/data/items.ron` are the live RON
  data files. No helmet or boot items were found in `data/items.ron` in the
  current search, but the workaround using `tags` must be audited.

### Identified Issues

1. No `Helmet` or `Boots` variant on `ArmorClassification` — slot routing is a
   tag-based workaround.
2. `has_slot_for_item` is not slot-specific: any `Armor` item routes to the body
   armor slot; helmets and boots would overwrite it.
3. `proficiency_for_armor` has an exhaustive match that will cause a compile
   error once new variants are added — this must be extended before the enum
   change compiles.
4. AC calculation does not sum `equipment.helmet` and `equipment.boots`
   contributions — only `equipment.armor` (and `equipment.shield` if at all)
   is consulted.
5. `data/items.ron` and `data/test_campaign/data/items.ron` must be updated
   atomically with the code change.

---

## Implementation Phases

### Phase 1: Enum and Proficiency Layer

Extend `ArmorClassification` and its proficiency mapping. This phase must
compile cleanly before any downstream code is changed, and the data migration
must happen in the same commit.

#### 1.1 Add Variants to `ArmorClassification`

In [src/domain/items/types.rs](../../src/domain/items/types.rs), add two
variants:

```
/// Helmet/headgear (maps to equipment.helmet)
Helmet,
/// Boots/footwear (maps to equipment.boots)
Boots,
```

The `#[default]` remains on `Light`. Update all doc examples in the enum to
include `Helmet` and `Boots` so doctests still compile.

#### 1.2 Extend `proficiency_for_armor` in `src/domain/proficiency.rs`

Add the two new arms to the exhaustive match in
[src/domain/proficiency.rs](../../src/domain/proficiency.rs):

```
ArmorClassification::Helmet => "light_armor".to_string(),
ArmorClassification::Boots  => "light_armor".to_string(),
```

Rationale: helmets and boots share proficiency requirements with light armor in
the classic RPG tradition. If dedicated proficiency IDs (`"helmet"` / `"boots"`)
are preferred in the future, this is the single place to change. Update the doc
comment and doctest to cover the new variants.

#### 1.3 RON Data Migration (Atomic with Phase 1)

Audit `data/items.ron`, `data/test_campaign/data/items.ron`, and
`campaigns/tutorial/data/items.ron` for items that currently use `tags` such as
`"helmet"` or `"boots"` to indicate slot. Change those items to use
`classification: Helmet` or `classification: Boots` inside their
`Armor(ArmorData { ... })` block. Remove the workaround tags from migrated
items.

Because no helmet/boot items were found in `data/items.ron` during research,
the migration may only affect `campaigns/tutorial/data/items.ron`. Verify
before committing.

#### 1.4 Testing Requirements

Tests in [src/domain/items/types.rs](../../src/domain/items/types.rs) and
[src/domain/proficiency.rs](../../src/domain/proficiency.rs):

- `test_armor_classification_helmet_variant_exists` — construct `ArmorClassification::Helmet`; assert `!= Light`.
- `test_armor_classification_boots_variant_exists` — same for `Boots`.
- `test_proficiency_for_armor_helmet_maps_to_light_armor` — assert `proficiency_for_armor(Helmet) == "light_armor"`.
- `test_proficiency_for_armor_boots_maps_to_light_armor` — assert `proficiency_for_armor(Boots) == "light_armor"`.
- Update existing `test_armor_required_proficiency_light/heavy/shield` doctests to remain valid.

#### 1.5 Deliverables

- [ ] `Helmet` and `Boots` variants added to `ArmorClassification`
- [ ] `proficiency_for_armor` extended for both variants
- [ ] RON data files migrated (tags replaced with classification)
- [ ] All four new tests pass

#### 1.6 Success Criteria

`cargo check --all-targets --all-features` and
`cargo nextest run --all-features` pass with zero errors/warnings. The two new
variants are fully recognized by the type system and the proficiency layer.

---

### Phase 2: Equipment Slot Routing

Update `has_slot_for_item` and the equip logic to route `Helmet` and `Boots`
items to the correct `Equipment` fields.

#### 2.1 Update `has_slot_for_item` in `equipment_validation.rs`

In [src/domain/items/equipment_validation.rs](../../src/domain/items/equipment_validation.rs),
the current `ItemType::Armor(_) => true` arm is slot-agnostic. Replace it with:

```
ItemType::Armor(data) => match data.classification {
    ArmorClassification::Light
    | ArmorClassification::Medium
    | ArmorClassification::Heavy    => true,   // equipment.armor slot
    ArmorClassification::Shield     => true,   // equipment.shield slot
    ArmorClassification::Helmet     => true,   // equipment.helmet slot
    ArmorClassification::Boots      => true,   // equipment.boots slot
},
```

All arms return `true` here because `has_slot_for_item` checks whether a slot
*type* exists (not whether it is empty — that is handled by `do_equip`). The
value of this change is making the classification-to-slot mapping explicit and
ensuring the exhaustive match catches any future variants at compile time.

#### 2.2 Update `do_equip` Slot Resolution

When `do_equip` is implemented in [src/domain/transactions.rs](../../src/domain/transactions.rs),
it must resolve the target slot from `ArmorClassification` rather than from
item `tags`:

| `ArmorClassification` | Target field          |
|-----------------------|-----------------------|
| `Light / Medium / Heavy` | `equipment.armor`  |
| `Shield`              | `equipment.shield`    |
| `Helmet`              | `equipment.helmet`    |
| `Boots`               | `equipment.boots`     |

If `do_equip` already exists, update the match arm. If it does not yet exist,
include this routing table in its initial implementation. Do not add a `tags`
fallback path.

#### 2.3 Update SDK Validation

In [src/sdk/validation.rs](../../src/sdk/validation.rs), add validation rules:

- An item with `classification: Helmet` in its `ArmorData` must have
  `ItemType::Armor` and no conflicting tags that imply a body-armor slot.
- An item with `classification: Boots` must similarly not conflict.
- A character definition that has `equipment.helmet = Some(item_id)` must
  reference an item with `classification: Helmet`.
- A character definition that has `equipment.boots = Some(item_id)` must
  reference an item with `classification: Boots`.

These checks prevent invalid data from reaching the game engine.

#### 2.4 Testing Requirements

All tests use `data/test_campaign` fixtures.

- `test_has_slot_for_helmet_item` — create an `Armor(Helmet)` item; assert
  `has_slot_for_item` returns `true`.
- `test_has_slot_for_boots_item` — same for `Boots`.
- `test_can_equip_helmet_succeeds` — full `can_equip_item` call with a `Helmet`
  item and a character with matching proficiency; assert `Ok(true)`.
- `test_can_equip_boots_succeeds` — same for `Boots`.
- `test_validation_helmet_in_wrong_slot_fails` — SDK validation: character with
  a `Boots` item in `equipment.helmet`; assert validation error.

#### 2.5 Deliverables

- [ ] `has_slot_for_item` updated with exhaustive classification match
- [ ] `do_equip` routes by classification (not tags)
- [ ] `src/sdk/validation.rs` enforces helmet/boot slot integrity
- [ ] All five new tests pass

#### 2.6 Success Criteria

A character definition that equips a `Helmet`-classified item in
`equipment.helmet` passes SDK validation. Attempting to equip a `Helmet` item
into `equipment.armor` fails validation. All tests pass.

---

### Phase 3: AC Calculation

Add a function that computes effective AC from all equipped armor slots. Wire
it into character creation and any point where `ac.current` is set from
equipment.

#### 3.1 Add `calculate_armor_class` Function

Add to [src/domain/items/equipment_validation.rs](../../src/domain/items/equipment_validation.rs)
(or a new `src/domain/items/ac.rs` submodule):

```rust
pub fn calculate_armor_class(
    equipment: &Equipment,
    item_db: &ItemDatabase,
) -> u8
```

Logic:
1. Start with `AC_DEFAULT` (10, defined in `src/domain/character.rs`).
2. For each of `equipment.armor`, `equipment.shield`, `equipment.helmet`,
   `equipment.boots`: if `Some(item_id)`, look up the item in `item_db`. If it
   is `ItemType::Armor(data)`, add `data.ac_bonus` to the running total.
3. Clamp the result to `[AC_MIN, AC_MAX]` (0–30).
4. Return the clamped value.

Accessory items with `BonusAttribute::ArmorClass` in `constant_bonus`/
`temporary_bonus` are out of scope for this plan.

#### 3.2 Wire `calculate_armor_class` at Equip / Unequip Time

Wherever `equipment` is mutated by the equip or unequip operation, call
`calculate_armor_class` and write the result to `character.ac.current`. Do
not modify `character.ac.base` — that is the unarmored base set by character
creation.

If `do_equip` is not yet fully implemented, add a note in the implementation
plan comment that `calculate_armor_class` must be called after any equip/
unequip operation.

#### 3.3 Testing Requirements

- `test_calculate_ac_no_armor` — empty `equipment`; assert result equals
  `AC_DEFAULT` (10).
- `test_calculate_ac_body_armor_only` — equip a +4 leather (body armor) only;
  assert result = 10 + 4 = 14.
- `test_calculate_ac_all_slots` — equip +4 body armor, +2 shield, +1 helmet,
  +1 boots; assert result = 10 + 4 + 2 + 1 + 1 = 18.
- `test_calculate_ac_clamps_to_max` — equip extremely high-AC items; assert
  result ≤ `AC_MAX` (30).
- `test_calculate_ac_missing_item_id_skips_slot` — equip a non-existent
  `item_id` in `equipment.helmet`; assert no panic, slot contribution is 0.

#### 3.4 Deliverables

- [ ] `calculate_armor_class(equipment, item_db) -> u8` implemented
- [ ] `character.ac.current` updated whenever equipment changes
- [ ] All five new tests pass

#### 3.5 Success Criteria

A character equipped with items in all four armor-bearing slots has an `ac.current`
that correctly sums all `ac_bonus` contributions. An unarmed character has
`ac.current == AC_DEFAULT`. Result is always in `[0, 30]`.

---

### Phase 4: Documentation and Final Validation

#### 4.1 Update `docs/explanation/implementations.md`

Add a section describing the `ArmorClassification` expansion: which variants
were added, how slot routing changed, how AC is now calculated, and which data
files were migrated.

#### 4.2 Verify Test Coverage for All `ArmorClassification` Tests

Update (or add) tests that previously only covered `Light/Medium/Heavy/Shield`
to also cover `Helmet` and `Boots` in:

- [src/domain/items/types.rs](../../src/domain/items/types.rs) — `required_proficiency` tests
- [src/domain/items/equipment_validation.rs](../../src/domain/items/equipment_validation.rs) — `can_equip_item` tests
- [src/domain/proficiency.rs](../../src/domain/proficiency.rs) — `proficiency_for_armor` tests

#### 4.3 Final Quality Gate Run

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

All four must pass with zero errors and zero warnings.

#### 4.4 Deliverables

- [ ] All existing `ArmorClassification` tests updated to cover new variants
- [ ] `docs/explanation/implementations.md` updated
- [ ] All quality gates pass

---

## Sequence Summary

| Phase | Core Output | Key Files |
|-------|-------------|-----------|
| 1 | `Helmet`/`Boots` variants + proficiency mapping + RON data migration | `src/domain/items/types.rs`, `src/domain/proficiency.rs`, `data/*.ron` |
| 2 | Slot routing in `has_slot_for_item`, `do_equip`, SDK validation | `src/domain/items/equipment_validation.rs`, `src/domain/transactions.rs`, `src/sdk/validation.rs` |
| 3 | `calculate_armor_class` function, `ac.current` wired at equip time | `src/domain/items/equipment_validation.rs`, `src/domain/character.rs` |
| 4 | Doc updates, exhaustive test coverage, quality gates | `docs/explanation/implementations.md` |

## Architecture Compliance Checklist

- [ ] `ArmorClassification` match arms are exhaustive (no `_` wildcard)
- [ ] Proficiency IDs are string constants from `ProficiencyDatabase`, not literals in callers
- [ ] RON data files use `classification: Helmet`/`Boots`, not `tags` workarounds
- [ ] No tests reference `campaigns/tutorial` (Implementation Rule 5)
- [ ] New fixture items for helmet/boots added to `data/test_campaign/data/items.ron`
- [ ] All public functions have `///` doc comments with doctests
- [ ] SPDX header present in all modified `.rs` files
- [ ] `docs/explanation/implementations.md` updated after completion
