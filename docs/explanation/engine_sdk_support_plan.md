# Engine Support for SDK Data Changes Implementation Plan

## Overview

This plan documents the changes required to support new SDK data additions in the Antares project and outlines a phased implementation to integrate those changes safely into the engine. The SDK additions include:

- `Item.icon_path` support for assets (icons),
- `Condition.icon_id` support,
- `ConsumableEffect` schema updates, specifically:
  - Replace `CureCondition(u8)` with `CureCondition(Vec<ConditionId>)`,
  - Add `duration` control for `BoostAttribute` effects via `ConsumableData.duration`.

The goal is to ensure the engine handles these new data fields with complete runtime behavior, including item usage in both combat and exploration contexts, and asset management for icons.

## Assumptions & Decisions (confirmed)

- `CureCondition` is no longer a bit-index or bit mask, but an explicit `Vec<ConditionId>`.
- `BoostAttribute` uses `ConsumableData.duration` (Option<ConditionDuration>). If unset, the engine uses a sensible default (e.g., `Rounds(5)` or `Minutes(10)` depending on context).
- The engine will implement and own an `EngineAssetManager` to load and cache assets (images, icons) referenced by `icon_path` and `icon_id`. This allows consistent runtime rendering and efficient asset re-use.

## High-Level Changes

- Domain model updates:
  - `src/domain/items/types.rs`: modify `ConsumableEffect::CureCondition` and add `ConsumableData.duration`.
  - Data files: update entries under `data/` and `campaigns/*/data/` to the new schema.
- Core engine logic:
  - Add a `Character::use_item` API that applies `ConsumableEffect` semantics.
  - Add `apply_item_effect` to process `ConsumableEffect` cases.
- Integration:
  - Ensure `SpellResult` effects are applied consistently in combat and exploration:
    - Ensure `apply_spell_conditions_to_character` and `apply_spell_conditions_to_monster` are used by combat systems when a spell is cast.
  - Implement `CombatState::apply_spell_result` or equivalent to apply damage/healing and conditions.
- Asset management:
  - Implement `src/engine/assets.rs` for loading and caching icons.
- Editor & SDK:
  - Update `sdk/campaign_builder/src/items_editor.rs` and other editors for new RON fields and UI.

## Current State & Gap Analysis

### Existing
- Domain types cover `Item`, `ConditionDefinition`, `ActiveCondition`, `Spell`, and `SpellResult`.
- Helpers exist: `apply_condition_dot_effects`, `apply_spell_conditions_to_*`.
- `SpellResult` is produced by `cast_spell` but is not always consumed by the engine to update the world or combat participants.

### Gaps
- No explicit `use_item` runtime API; consumables are not applied automatically by the engine.
- `ConsumableEffect::CureCondition(u8)` legacy representation is ambiguous and not implemented against `ConditionDefinition`.
- Items with `icon_path` are stored in data but there is no centralized runtime asset manager to load images for UI or debugging.
- `BoostAttribute` temporary effect duration is not defined at the consumable level.
- Integration between `SpellResult` and combat application must be standardized.

## Implementation Phases

Each phase includes: (a) specific file/symbol tasks, (b) tests to add, and (c) deliverables and acceptance criteria.

---

### Phase 1 — Schema & Domain Updates (Core foundation)

1. File & Model Updates
   - `src/domain/items/types.rs`
     - Change `ConsumableEffect::CureCondition(u8)` to `ConsumableEffect::CureCondition(Vec<ConditionId>)`.
     - Add `duration: Option<crate::domain::conditions::ConditionDuration>` to `ConsumableData`.
   - Update `src/domain/items/database.rs` parsing to decode the new structure.
2. SDK / Test Data Update
   - Update `data/items.ron`, `campaigns/*/data/items.ron` to use `CureCondition(["poison"])`.
   - Add `duration` attribute to the `ConsumableData` as appropriate.
3. Tests
   - RON parsing tests for new `CureCondition(Vec)` and `ConsumableData.duration`.
   - Editor tests for saving/loading new fields.
4. Deliverables
   - New domain types compile and parse sample RON files.
5. Success Criteria
   - No compile errors.
   - Sample campaign items parse into `Item` objects with valid `ConsumableData` fields.

---

### Phase 2 — Item Usage & Consumable Effect Application

1. New Engine API
   - Add `pub fn use_item(&mut self, slot_idx: usize, ctx: &EngineContext) -> Result<SpellResult, CharacterError>` to `src/domain/character.rs`:
     - Validate `is_combat_usable` and resource (charges).
     - Call into `apply_item_effect` helper.
   - Add `apply_item_effect(...)` to apply consumable data semantics.
2. Consumable Effects Implementation (in `apply_item_effect`):
   - `HealHp(u16)` → `self.hp.modify(amount as i32)` or equivalent safe modifier.
   - `RestoreSp(u16)` → `self.sp.modify(amount as i32)`.
   - `CureCondition(Vec<ConditionId>)`:
     - For each `ConditionId`, call `Character::remove_active_condition_by_id`.
     - Define `remove_active_condition_by_id` in `src/domain/character.rs`.
   - `BoostAttribute(AttributeType, i8)`:
     - Create a new `ActiveCondition` with `ConditionEffect::AttributeModifier` and set duration to `ConsumableData.duration`, or a default if `None`.
     - Add the `ActiveCondition` via `Character::add_condition`.
   - `Item.spell_effect: Option<SpellId>`:
     - Resolve `Spell` and call `cast_spell`, then return the `SpellResult` for the caller to apply.
3. Tests
   - Unit tests: `test_use_item_heal`, `test_use_item_restore_sp`, `test_use_item_cure_condition_by_id`, `test_use_item_boost_attribute_duration`.
   - Integration tests: item use in combat & exploration flows.
4. Deliverables
   - `use_item` & `apply_item_effect` implemented; unit tests added.
5. Success Criteria
   - Items perform semantic behaviors (heal, restore SP, cure conditions, create active conditions).
   - `Item.spell_effect` results in `SpellResult`.

---

### Phase 3 — Spell Result, Combat & Exploration Integration

1. New Combat Hooks
   - `src/domain/combat/engine.rs`:
     - Add `pub fn apply_spell_result(&mut self, caster_id: CombatantId, result: SpellResult, condition_defs: &[ConditionDefinition])`.
     - Add `pub fn use_item_in_combat(&mut self, user: CombatantId, slot_idx: usize)` wrapping `Character::use_item`.
   - `apply_spell_result` should:
     - Apply damage/healing to `affected_targets`.
     - Use `apply_spell_conditions_to_character` / `apply_spell_conditions_to_monster` for each `applied_condition`.
     - Return a list of per-target effects for logging.
2. Exploration Integration
   - Add a small exploration routine that applies `SpellResult` contents to party or NPCs by reusing the logic for noncombat contexts.
3. Tests
   - `test_apply_spell_result_in_combat`: confirm damage/HOT/DOT and condition application.
   - `test_item_use_in_combat` flows test.
4. Deliverables
   - `apply_spell_result` hook with thorough coverage of `SpellResult` semantics.
5. Success Criteria
   - Spell result application is deterministic and consistent in both combat & exploration.

---

### Phase 4 — Engine Asset Manager & Icons

1. Implement `EngineAssetManager`
   - New file: `src/engine/assets.rs`.
   - Public API:
     - `pub struct EngineAssetManager { ... }`
     - `pub fn load_icon(&mut self, path: &str) -> Result<AssetHandle, AssetError>`
     - `pub fn get_icon(&self, handle: &AssetHandle) -> Option<&IconTexture>`
   - Asset caching: in-memory cache keyed by path to avoid reloading.
2. Integrate with data fields
   - Make `Item.icon_path` and `Condition.icon_id` available to the `EngineAssetManager`.
   - Provide a small helper from the engine to query `EngineAssetManager` to retrieve icons for UI.
3. Tests
   - Unit tests: caching semantics, missing-file behavior.
   - If we have a small headless UI mock, test that an icon handle renders or is retrievable.
4. Deliverables
   - Asset manager + unit tests.
5. Success Criteria
   - Engine loads assets properly, caches them, and gracefully handles missing assets.

---

### Phase 5 — Validation, Documentation & QA

1. Test Coverage
   - Ensure unit tests and integration tests for each new feature:
     - `use_item` tests,
     - `apply_item_effect` tests for each `ConsumableEffect`,
     - `apply_spell_result` tests,
     - Asset manager tests.
   - Integration tests for sample campaign usage of new RON consumables.
2. Documentation
   - Update `docs/reference/architecture.md`:
     - Document `ConsumableEffect::CureCondition(Vec<ConditionId>)`.
     - Document `ConsumableData.duration`.
     - Note how `BoostAttribute` is represented and applied.
     - Update item asset fields (`icon_path`) semantics.
   - Update SDK docs & editors:
     - `sdk/campaign_builder/src/items_editor.rs` for the UI change (condition multi-select & duration field).
3. Migration & Tools
   - Create a small, optional migration tool/script under `sdk` or `tools`:
     - Convert `CureCondition(4)` to `CureCondition(["poison"])` for existing campaigns.
   - Add editor validation & warnings for missing `ConditionId`s or invalid `icon_path`.
4. Success Criteria
   - Test suite passes for the updated modules.
   - `cargo check`, `clippy`, `fmt` pass.
   - Editor saves human-readable `CureCondition` RON format.
   - Asset manager loads icons successfully when available.

---

## Developer Checklist (PR-sized task list)
- PR 1 (Small, safe): Domain type & RON parsing updates for `ConsumableEffect::CureCondition` and `ConsumableData.duration`.
- PR 2: Update sample RON files & SDK editor `items_editor` UI to handle the new consumable format and `duration`.
- PR 3: Implement `Character::use_item` and `apply_item_effect` + unit tests.
- PR 4: Implement `CombatState::apply_spell_result`, `use_item_in_combat`, and combat integration tests.
- PR 5: Implement `EngineAssetManager`: load & caching + unit tests; integrate with `Item.icon_path`.
- PR 6: Migration tool + sample campaign sweeps + final documentation updates.
- PR 7: Add coverage tests & refactors following the changes if necessary.

---

## Migration Notes
- Provide a script or editor "Import" path for old-style `CureCondition(4)`→`CureCondition(["poison"])` conversion. This is optional but helpful for campaign migration if there are user campaigns that rely on legacy formats.
- For in-game consumables with `BoostAttribute` that previously had no duration, the SDK should now ask the user to choose a duration on creation — if missing in legacy, default the item to a sensible duration or require a user-specified duration.

---

## Risks & Mitigations
- Risk: Editor or data files with legacy formats: Mitigation: Provide import tool and editor import warnings.
- Risk: Multiple conflicting condition names: Mitigation: Validate `ConditionId` exists at save/import time; warn the author.
- Risk: Asset load failure: Mitigation: `AssetManager` should fallback gracefully and render a default placeholder icon instead of failing.
- Risk: Breaking game rules: Mitigation: Unit/integration tests and QA pass with scenario-based tests for consumables and spells.

---

## Final Notes & Next Steps
- If you confirm the plan, I will break the plan into PR-sized tasks (the “Developer Checklist”) and prepare PR descriptions, tests, and the initial migration scripts.
- We will also update the `docs/explanation/implementations.md` (summary) once individual phases are completed and PRs merged — the plan file will be the primary, detailed reference.
