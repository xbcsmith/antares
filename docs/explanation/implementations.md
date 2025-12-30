## Plan: Portrait IDs as Strings

TL;DR: Require portrait identifiers to be explicit strings (filename stems). Update domain types, HUD asset lookups, campaign data, and campaign validation to use and enforce string keys. This simplifies asset management and ensures unambiguous, filesystem-driven portrait matching.

**Steps (4 steps):**

1. Change domain types in [file](antares/src/domain/character_definition.rs) and [file](antares/src/domain/character.rs): convert `portrait_id` to `String` (`CharacterDefinition::portrait_id`, `Character::portrait_id`).
2. Simplify HUD logic in [file](antares/src/game/systems/hud.rs): remove numeric mapping and index portraits only by normalized filename stems (`PortraitAssets.handles_by_name`); lookups use `character.portrait_id` string key first then fallback to normalized `character.name`.
3. Require campaign data changes: update sample campaigns (e.g. `campaigns/tutorial/data/characters.ron`) and add validation (in `sdk/campaign_builder` / campaign loader) to reject non-string `portrait_id`.
4. Update tests and docs: adjust unit tests to use string keys, add new tests for name-key lookup + validation, and document the new format in `docs/reference` and `docs/how-to`.

**Decisions:**

1. Strict enforcement: Numeric `portrait_id` values will be rejected with a hard error during campaign validation. Campaign data MUST provide `portrait_id` as a string (filename stem); migration helpers or warnings are out-of-scope for this change.

2. Normalization: Portrait keys are normalized by lowercasing and replacing spaces with underscores when indexing and looking up assets (e.g., `"Sir Lancelot"` -> `"sir_lancelot"`).

3. Default value: When omitted, `portrait_id` defaults to an empty string (`""`) to indicate no portrait. The legacy `"0"` value is no longer used.

---

# Portrait IDs as Strings Implementation Plan

## Overview

Replace numeric portrait identifiers with explicit string identifiers (matching filename stems). Campaign authors will provide portrait keys as strings (example: `portrait_id: "kira"`) and the engine will match files in `assets/portraits/` by normalized stem. Validation will require string usage and will error on numeric form to avoid ambiguity.

## Current State Analysis

### Existing Infrastructure

- Domain types:
  - `CharacterDefinition::portrait_id: u8` ([file](antares/src/domain/character_definition.rs))
  - `Character::portrait_id: u8` ([file](antares/src/domain/character.rs))
- HUD / UI:
  - `PortraitAssets` currently includes `handles_by_id: HashMap<u8, Handle<Image>>` and `handles_by_name: HashMap<String, Handle<Image>>` ([file](antares/src/game/systems/hud.rs)).
  - `ensure_portraits_loaded` parses filenames and optionally indexes numeric stems.
  - `update_portraits` tries numeric lookup then name lookup.
- Campaign data:
  - `campaigns/tutorial/data/characters.ron` uses numeric `portrait_id` values.
- Tooling: Campaign editor exists under `sdk/campaign_builder` and currently allows/assumes `portrait_id` as strings in editor buffers, but validation is not strict.

### Identified Issues

- Mixed numeric/string handling adds complexity and ambiguity.
- Many characters default to numeric `0`, leading to identical placeholders.
- Lack of explicit validation means old numeric data silently works (or is partially tolerated); user wants to require explicit string format.

## Implementation Phases

### Phase 1: Core Implementation

#### 1.1 Foundation Work

- Change `CharacterDefinition::portrait_id` from `u8` -> `String` in [file](antares/src/domain/character_definition.rs) and update `CharacterDefinition::new` default.
- Change `Character::portrait_id` from `u8` -> `String` in [file](antares/src/domain/character.rs) and update `Character::new` default.
- Add/adjust model documentation comments to describe the new requirement.

#### 1.2 Add Foundation Functionality

- Update `PortraitAssets` in [file](antares/src/game/systems/hud.rs) to remove `handles_by_id` and only use `handles_by_name: HashMap<String, Handle<Image>>`.
- Update `ensure_portraits_loaded`:
  - Always index files by normalized stem (lowercase + underscores).
  - Do not attempt numeric parsing or special numeric mapping.
- Update `update_portraits`:
  - Use `character.portrait_id` (normalized) as first lookup key in `handles_by_name`.
  - Fallback to normalized `character.name` if no `portrait_id` key is found.
- Add debug logging around asset scanning and lookup for observability.

#### 1.3 Integrate Foundation Work

- Update all code that previously relied on numeric portrait indices.
- Remove or repurpose any helper maps or code paths used solely for numeric handling.
- Ensure `CharacterDefinition` deserialization expects strings (strict), so numeric values in campaign files will cause validation error.

#### 1.4 Testing Requirements

- Add unit tests for `ensure_portraits_loaded` to confirm indexing by normalized name keys.
- Add unit tests for `update_portraits` verifying lookup precedence and fallback.
- Update existing tests that use numeric literals (e.g., `portrait_id: 1`) to use string keys (e.g., `portrait_id: "1".to_string()` or more meaningful names).
- Add validation tests asserting that numeric `portrait_id` values in campaign RON fail validation (explicit error).

#### 1.5 Deliverables

- [] `CharacterDefinition` and `Character` updated to use `String`.
- [] HUD asset loading updated to name-only indexing.
- [] Validation logic added to campaign loader/editor to reject numeric `portrait_id`.
- [] Tests updated and new tests added.
- [] Documentation updated in `docs/reference` and sample campaigns updated.

#### 1.6 Success Criteria

- All unit tests pass.
- Engine fails campaign validation for any campaign that uses numeric `portrait_id`.
- Updated tutorial campaign (example) uses string portrait keys and HUD displays portraits accordingly.

### Phase 2: Campaign & Tooling Updates

#### 2.1 Feature Work

- Update the tutorial campaign `campaigns/tutorial/data/characters.ron` as the canonical example to use string portrait IDs.
- Update the `sdk/campaign_builder` editor UI to present/enforce a string input for portrait keys.

#### 2.2 Integrate Feature

- Add a validation routine in campaign loading/publishing to check:
  - `portrait_id` must be a non-empty string when present.
  - A matching file exists in `assets/portraits/` for `portrait_id`, or emit a clear validation error.

#### 2.3 Configuration Updates

- Update developer docs (new doc in `docs/how-to/portrait_naming.md`) describing:
  - required filename rules,
  - normalization policy (lowercase + underscores),
  - example entries and sample RON snippets.

#### 2.4 Testing requirements

- Integration test: Load a sample campaign with string portrait IDs and ensure HUD portraits render.
- Validation tests: Ensure campaigns with numeric `portrait_id` values raise validation errors.

#### 2.5 Deliverables

- [] Tutorial campaign updated to string keys.
- [] Campaign editor validation enforced in `sdk/campaign_builder`.
- [] Documentation and examples updated.

#### 2.6 Success Criteria

- Campaigns with string `portrait_id` load and display portraits correctly.
- Campaigns with numeric `portrait_id` fail validation with clear guidance to users.

---

This is a draft plan for review. I will NOT begin implementation until you confirm the plan and answer the open questions:

1. Strictly reject numeric `portrait_id` during validation? (Yes/No)
2. Confirm normalization: lowercase + underscores? (Yes/No)
3. Default `Character::portrait_id` preference: empty `""` or legacy `"0"`? (Empty / `"0"`)

Please review and confirm. Once confirmed I will produce an ordered checklist of concrete PR-sized tasks and testing steps for implementation.
