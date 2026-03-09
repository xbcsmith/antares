# Unified Creature Asset Binding Implementation Plan

<!-- SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com> -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

## Overview

Standardise the creature-visual binding field to `creature_id: Option<CreatureId>`
across all three definition types (`MonsterDefinition`, `NpcDefinition`,
`CharacterDefinition`) and remove the fragile runtime name-match heuristic that
currently backs `CharacterDefinition`. The plan is split into four self-contained
phases that each compile, pass all tests, and ship a meaningful improvement
independently.

- **Phase 1** renames `MonsterDefinition.visual_id` → `creature_id` in the domain
  and runtime structs, updates every call-site, RON data file, and test.
- **Phase 2** adds `creature_id: Option<CreatureId>` to `CharacterDefinition`,
  populates it in the data files, and replaces `resolve_recruitable_creature_id`
  with a direct field lookup in `map.rs`.
- **Phase 3** adds the optional `CreatureBound` trait that makes rendering-system
  lookups type-generic and validates the full three-way symmetry.
- **Phase 4** updates all three Campaign Builder SDK editors — Monsters, Characters,
  and NPCs — so that every definition type exposes a consistent Browse/Clear/tooltip
  creature picker in its edit form.

Backwards compatibility is explicitly **not** required. Every `monsters.ron` file
must be updated before the build will pass; `cargo check` catches any missed file
at deserialisation time.

---

## Current State Analysis

### Existing Infrastructure

| Component | File | Current Behaviour |
|---|---|---|
| `MonsterDefinition.visual_id` | `src/domain/combat/database.rs` L118–122 | `Option<CreatureId>` with `#[serde(default)]`; name diverges from NPC |
| `Monster.visual_id` (runtime) | `src/domain/combat/monster.rs` L331–335 | Same `Option<CreatureId>`; set via `set_visual(visual_id)` L402 |
| `NpcDefinition.creature_id` | `src/domain/world/npc.rs` L145 | `Option<CreatureId>` — already the target field name |
| `CharacterDefinition` | `src/domain/character_definition.rs` L341–434 | **No visual field at all** |
| `resolve_recruitable_creature_id` | `src/game/systems/map.rs` L406–437 | Three-step fallback: NPC proxy → id strip → name-match heuristic |
| `resolve_encounter_creature_id` | `src/game/systems/map.rs` L441–455 | Reads `monster_def.visual_id` directly — needs field name update |
| `CreatureAssetManager` | `sdk/campaign_builder/src/creature_assets.rs` L53 | Manages registry and per-file loading; exposes `load_all_creatures()`, `list_creatures()`, `next_creature_id()` |
| Monsters Editor `show_form` | `sdk/campaign_builder/src/monsters_editor.rs` L745 | `visual_id` **not exposed** in the Add/Edit form at all |
| NPC Editor `show_edit_view` | `sdk/campaign_builder/src/npc_editor.rs` ~L730 | `creature_id` buffer exists and `save_npc` parses it; **no UI widget** in edit form |
| Characters Editor `show_character_form` | `sdk/campaign_builder/src/characters_editor.rs` ~L1723 | `CharacterEditBuffer` has no `creature_id` field; portrait picker pattern exists as model |

### Identified Issues

1. **Naming inconsistency**: `MonsterDefinition` uses `visual_id`; `NpcDefinition` uses
   `creature_id`. Identical concept, two field names. Every system that reads either must
   branch on the definition type.

2. **`CharacterDefinition` has no visual binding field**: `resolve_recruitable_creature_id`
   (`map.rs` L406–437) compensates with a three-step fallback that includes a
   normalised name-match against `CreatureDefinition.name`. A campaign author who
   names a character `"Gareth the Old"` and a creature `"old gareth"` silently gets
   a placeholder sprite with only a log warning.

3. **`visual_id` is absent from the Monsters Editor UI**: `show_form` in
   `monsters_editor.rs` renders Identity, Combat Stats, Attributes, Resistances,
   Abilities, Attacks, and Loot sections but has no row for the visual/creature
   field. Campaign authors cannot assign a creature asset to a monster without
   hand-editing the RON file.

4. **`creature_id` is absent from the NPC Editor UI**: The field exists in
   `NpcEditBuffer` and is parsed in `save_npc`, but no widget renders it in
   `show_edit_view`. The preview panel does show a read-only label for it, but
   there is no way to set it through the UI.

5. **Monster `set_visual` method uses the old parameter name**: `fn set_visual(&mut
   self, visual_id: CreatureId)` at `monster.rs` L402 will need the parameter
   renamed to `creature_id` for consistency after the field rename.

6. **Scattered `visual_id` constructions in SDK and tests**: `lib.rs`, `templates.rs`,
   `ui_helpers.rs`, `advanced_validation.rs`, and many inline test helpers inside
   `map.rs` and `database.rs` all construct `MonsterDefinition` with `visual_id:
   None` or `visual_id: Some(...)`. All must be updated in Phase 1.

7. **`CreatureBound` trait does not exist**: There is no shared trait or convention
   that allows rendering-system code to call a single method on any definition type
   to retrieve the creature binding. Phase 3 adds this.

---

## Implementation Phases

---

### Phase 1: Rename `visual_id` → `creature_id` on Monster Types

Rename the field on `MonsterDefinition` and the runtime `Monster` struct, update
every call-site in source code and tests, update both RON data files, and verify
all quality gates pass before touching anything in Phase 2.

#### 1.1 Foundation Work — Domain and Runtime Structs

**File to modify**: `src/domain/combat/database.rs`

- Rename `pub visual_id: Option<CreatureId>` → `pub creature_id: Option<CreatureId>`
  at L118–122.
- Update the `#[serde(default)]` attribute — keep it, field name change is
  sufficient (no rename alias needed; backwards compatibility is not required).
- Update the doc comment from `"Optional visual creature ID"` to
  `"Optional creature asset binding — links this monster to a `CreatureDefinition`
   in the creature registry."`.
- In `to_monster()` at L184–188: rename `monster.visual_id = self.visual_id;`
  → `monster.creature_id = self.creature_id;`.
- Update the `MonsterDefinition` doc-comment example (L82–83, L156–160) to use
  `creature_id: None`.
- Update the `MonsterDatabase` doc-comment example (L350–354) to use `creature_id: None`.
- Update the inline test helper `create_test_monster` (L439–443) to use `creature_id: None`.
- Update `test_monster_visual_id_parsing` (L530–542):
  - Rename to `test_monster_creature_id_parsing`.
  - Replace all `monster.visual_id` references with `monster.creature_id`.
- Update `test_load_tutorial_monsters_visual_ids` (L543–598):
  - Rename to `test_load_tutorial_monsters_creature_ids`.
  - Replace all `.visual_id` references with `.creature_id`.
  - Update assertion messages from `"incorrect visual_id"` → `"incorrect creature_id"`.

**File to modify**: `src/domain/combat/monster.rs`

- Rename `pub visual_id: Option<CreatureId>` → `pub creature_id: Option<CreatureId>`
  at L331–335.
- Update the doc comment to match the wording used in `database.rs`.
- In `Monster::new()` at L371–375: rename `visual_id: None` → `creature_id: None`.
- Rename `pub fn set_visual(&mut self, visual_id: CreatureId)` at L402–404:
  - New signature: `pub fn set_visual(&mut self, creature_id: CreatureId)`.
  - Body: `self.creature_id = Some(creature_id);`.
  - Update the doc-comment example to use `creature_id` instead of `visual_id`.
- Update the `Monster` doc-comment example (L285–286) to use `creature_id: None`.

#### 1.2 Update Rendering System

**File to modify**: `src/game/systems/map.rs`

- In `resolve_encounter_creature_id` (L441–455): rename all `monster_def.visual_id`
  references to `monster_def.creature_id`.
- Update the function's leading doc comment ("Uses the first monster entry that has
  a configured `visual_id`") to say `creature_id`.
- Update every inline test in `mod tests` that constructs a `MonsterDefinition`
  with `visual_id: Some(...)` or `visual_id: None`:
  - `test_resolve_encounter_creature_id_returns_first_visual_match` (L1911–1915)
  - `test_resolve_encounter_creature_id_skips_monsters_without_visuals` (L1946–1973)
  - `test_map_event_encounter_facing` (L2734–2738)
  - `test_proximity_facing_inserted_on_encounter_with_flag` (L3027–3031)
  - `test_proximity_facing_not_inserted_when_flag_false` (L3100–3104)
  - `test_proximity_facing_rotation_speed_forwarded_on_encounter` (L3428–3432)

**File to modify**: `src/game/systems/monster_rendering.rs`

- Rename all `monster.visual_id` references to `monster.creature_id`.
  Specifically:
  - L151–162: `if let Some(visual_id) = monster.visual_id` →
    `if let Some(creature_id) = monster.creature_id`.
  - L168–171: `creature_id: visual_id` → `creature_id` (binding name already
    matches after rename; verify no variable shadowing issue).
  - L176–179: Update the `warn!` message from `"has invalid visual_id {}"` to
    `"has invalid creature_id {}"`.
  - L182–185: Update the `"// No visual_id"` comment to `"// No creature_id"`.
  - L12–18: Update module-level doc comments from `visual_id` to `creature_id`.
  - L92–93: Update the function doc comment.

**File to modify**: `src/domain/combat/engine.rs`

- Find the inline `MonsterDefinition` construction in the test at L2085–2089
  and rename `visual_id: None` → `creature_id: None`.

#### 1.3 Update SDK Campaign Builder

**File to modify**: `sdk/campaign_builder/src/monsters_editor.rs`

- `default_monster()` at L88–92: rename `visual_id: None` → `creature_id: None`.
  (The `edit_buffer` field stores a raw `MonsterDefinition`, so this is the only
  change needed here; the UI field will be added in Phase 4.)

**File to modify**: `sdk/campaign_builder/src/advanced_validation.rs`

- `create_test_monster()` at L780–784: rename `visual_id: Some(id)` →
  `creature_id: Some(id)`.

**File to modify**: `sdk/campaign_builder/src/lib.rs`

- All `MonsterDefinition` struct literals that contain `visual_id:`:
  - `default_monster()` at L1022–1026: `visual_id: None` → `creature_id: None`.
  - `test_monster_xp_calculation_basic` at L9383–9387: same.
  - `test_monster_xp_calculation_with_abilities` at L9437–9441: same.
  - `test_monster_import_export_roundtrip` at L9486–9490: same.
  - `test_monster_preview_fields` at L9540–9544: same.

**File to modify**: `sdk/campaign_builder/src/templates.rs`

- `create_monster()` — all four `MonsterDefinition` struct literals (L439–443,
  L467–471, L495–499, L526–530): rename `visual_id: None` → `creature_id: None`.

**File to modify**: `sdk/campaign_builder/src/ui_helpers.rs`

- Test helpers that construct `MonsterDefinition` (L5831–5835, L6487–6491):
  rename `visual_id: None` → `creature_id: None`.

#### 1.4 Update RON Data Files

**File to modify**: `data/test_campaign/data/monsters.ron`

- Replace every occurrence of `visual_id:` with `creature_id:` using a simple
  text substitution. No numeric values change.

**File to modify**: `campaigns/tutorial/data/monsters.ron`

- Same substitution: every `visual_id:` → `creature_id:`.

Search the entire repository for any remaining `.ron` files that contain `visual_id:`
and apply the same substitution:

```bash
grep -r "visual_id:" data/ campaigns/ --include="*.ron" -l
```

Apply the rename to every file that appears in the output.

#### 1.5 Update Integration Tests

**File to modify**: `tests/campaign_integration_tests.rs`

- `test_all_monsters_have_visual_id_mapping` at L99–109:
  - Rename to `test_all_monsters_have_creature_id_mapping`.
  - Replace all `.visual_id` field accesses with `.creature_id`.
  - Update assertion messages.

#### 1.6 Testing Requirements

Run the full quality gate sequence after every edit:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

Add or verify the following unit tests in `src/domain/combat/database.rs` inside
`mod tests`:

| Test name | What it verifies |
|---|---|
| `test_monster_creature_id_parsing` | Setting and reading `creature_id` on a `MonsterDefinition`; `None` is valid |
| `test_monster_definition_creature_id_field_roundtrips_ron` | Serialise `MonsterDefinition` with `creature_id: Some(42)` to RON; deserialise; assert value survives |
| `test_load_tutorial_monsters_creature_ids` | All monsters in `data/test_campaign/data/monsters.ron` have expected `creature_id` values |

Add in `src/domain/combat/monster.rs`:

| Test name | What it verifies |
|---|---|
| `test_set_visual_sets_creature_id` | `monster.set_visual(5)` sets `creature_id` to `Some(5)` |

#### 1.7 Deliverables

- [ ] `src/domain/combat/database.rs` — `visual_id` → `creature_id` on `MonsterDefinition`; `to_monster()` updated; all doc examples and tests updated
- [ ] `src/domain/combat/monster.rs` — `visual_id` → `creature_id` on `Monster`; `set_visual` parameter renamed
- [ ] `src/game/systems/map.rs` — `resolve_encounter_creature_id` reads `monster_def.creature_id`; all inline tests updated
- [ ] `src/game/systems/monster_rendering.rs` — all `visual_id` references → `creature_id`
- [ ] `src/domain/combat/engine.rs` — test helper updated
- [ ] `sdk/campaign_builder/src/monsters_editor.rs` — `default_monster()` updated
- [ ] `sdk/campaign_builder/src/advanced_validation.rs` — `create_test_monster()` updated
- [ ] `sdk/campaign_builder/src/lib.rs` — all `MonsterDefinition` literals updated
- [ ] `sdk/campaign_builder/src/templates.rs` — all `create_monster()` literals updated
- [ ] `sdk/campaign_builder/src/ui_helpers.rs` — test helpers updated
- [ ] `data/test_campaign/data/monsters.ron` — all `visual_id:` → `creature_id:`
- [ ] `campaigns/tutorial/data/monsters.ron` — all `visual_id:` → `creature_id:`
- [ ] Any other `.ron` files containing `visual_id:` updated
- [ ] `tests/campaign_integration_tests.rs` — test renamed and updated
- [ ] The word `visual_id` no longer appears anywhere in the repository (verified by `grep -r visual_id . --include="*.rs" --include="*.ron"`)
- [ ] All four quality gates pass with zero errors/warnings

#### 1.8 Success Criteria

- `grep -r "visual_id" . --include="*.rs" --include="*.ron"` returns zero matches.
- `cargo nextest run --all-features` reports zero failures.
- `test_monster_definition_creature_id_field_roundtrips_ron` passes: a
  `MonsterDefinition` with `creature_id: Some(42)` round-trips through RON
  serialisation without data loss.
- All existing rendering and combat tests that previously used `visual_id` now
  use `creature_id` and produce the same behaviour.

---

### Phase 2: Add `creature_id` to `CharacterDefinition` and Delete the Heuristic

Add `pub creature_id: Option<CreatureId>` to `CharacterDefinition`, populate the
field in the data files, populate the campaign tutorial data, and replace
`resolve_recruitable_creature_id` with a single-line direct field read in
`map.rs`.

#### 2.1 Foundation Work — Domain Struct

**File to modify**: `src/domain/character_definition.rs`

Add the new field to `CharacterDefinition` (L341–434), after `portrait_id`:

```rust
/// Optional creature asset binding — links this character to a
/// `CreatureDefinition` in the creature registry for 3D map rendering.
/// When `None`, the rendering system falls back to the portrait sprite.
#[serde(default)]
#[serde(skip_serializing_if = "Option::is_none")]
pub creature_id: Option<CreatureId>,
```

The `#[serde(default)]` annotation ensures that existing RON files that omit the
field deserialise without error. `skip_serializing_if = "Option::is_none"` keeps
the RON compact — characters without a visual binding do not emit `creature_id: None`.

Import `CreatureId` at the top of the file:

```rust
use crate::domain::types::CreatureId;
```

Update the `architecture.md` `CharacterDefinition` table in Section 4.7 to add a
`creature_id: Option<CreatureId>` row with the annotation
`"#[serde(default)] — None = sprite fallback"`.

#### 2.2 Delete the Heuristic and Simplify `map.rs`

**File to modify**: `src/game/systems/map.rs`

1. **Delete `normalize_lookup_key`** (L392–398) — it is only used by
   `resolve_recruitable_creature_id`. Deleting both removes dead code.

2. **Delete `resolve_recruitable_creature_id`** (L406–437) in its entirety.

3. In `fn spawn_map`, find the `RecruitableCharacter` spawn branch (around L1395):

   Before:
   ```rust
   if let Some(creature_id) =
       resolve_recruitable_creature_id(character_id, &content)
   ```

   After (direct field lookup):
   ```rust
   if let Some(creature_id) = content
       .0
       .characters
       .get_character(character_id)
       .and_then(|def| def.creature_id)
   ```

4. Update the `RecruitableCharacter` branch fallback path: when `creature_id` is
   `None`, the existing sprite-placeholder logic already runs via the `else`
   branch — no change needed there.

5. Remove the `// Uses the NPC definition path` comment from
   `test_map_event_recruitable_character_facing` (L2947–2957) — the test should
   be updated to set `creature_id` directly on a `CharacterDefinition` rather than
   proxying through an `NpcDefinition`.

#### 2.3 Populate Campaign Data Files

Pre-made characters in the tutorial campaign that have a 3D creature representation
must have their `creature_id` set in the RON file. For any character entry that
previously resolved through the NPC proxy or name-match path, the value that was
previously returned by `resolve_recruitable_creature_id` must now be set
explicitly.

**File to modify**: `campaigns/tutorial/data/characters.ron`

For each character entry that previously resolved through the heuristic, add:

```ron
creature_id: Some(<id>),
```

with the numeric `CreatureId` value that was being returned by the now-deleted
heuristic. Characters with no 3D representation simply omit the field (the
`#[serde(default)]` handles this).

**File to modify**: `data/test_campaign/data/characters.ron`

Apply the same additions for any test fixture characters that need a visual binding.
If no test fixture characters currently have a creature binding via the heuristic,
add at least one entry with `creature_id: Some(N)` so the new round-trip test has
data to exercise.

#### 2.4 Testing Requirements

Run the full quality gate sequence.

New tests to add in `src/domain/character_definition.rs` inside `mod tests`:

| Test name | What it verifies |
|---|---|
| `test_character_definition_creature_id_defaults_to_none` | `CharacterDefinition` constructed without the field (or with `#[serde(default)]`) produces `creature_id: None` |
| `test_character_definition_creature_id_field_roundtrips_ron` | Serialise with `creature_id: Some(7)`; deserialise; assert `Some(7)` survives |
| `test_character_definition_creature_id_none_omits_field_in_ron` | Serialise with `creature_id: None`; assert the output string does not contain `"creature_id"` |

New tests to add in `src/game/systems/map.rs` inside `mod tests`:

| Test name | What it verifies |
|---|---|
| `test_recruitable_spawn_uses_character_def_creature_id` | Build a minimal `App` with a `CharacterDefinition` that has `creature_id: Some(N)` and a matching `CreatureDefinition`; trigger a `RecruitableCharacter` map event; assert a `CreatureVisual { creature_id: N }` entity is spawned |
| `test_recruitable_spawn_falls_back_to_sprite_when_no_creature_id` | Same setup but `creature_id: None`; assert no `CreatureVisual` is spawned and a sprite entity is present instead |

Verify that the existing test `test_map_event_recruitable_character_facing` still
passes after the refactor (it must be updated to set `creature_id` on a
`CharacterDefinition` rather than via `NpcDefinition`).

#### 2.5 Deliverables

- [ ] `src/domain/character_definition.rs` — `pub creature_id: Option<CreatureId>` added with `#[serde(default)]` and `skip_serializing_if`
- [ ] `docs/reference/architecture.md` — Section 4.7 `CharacterDefinition` table updated with `creature_id` row
- [ ] `src/game/systems/map.rs` — `normalize_lookup_key` deleted; `resolve_recruitable_creature_id` deleted; `RecruitableCharacter` spawn branch uses direct field lookup
- [ ] `campaigns/tutorial/data/characters.ron` — `creature_id: Some(N)` added for all characters that previously resolved through the heuristic
- [ ] `data/test_campaign/data/characters.ron` — at least one character with `creature_id: Some(N)` for round-trip testing
- [ ] `test_map_event_recruitable_character_facing` — updated to set `creature_id` on `CharacterDefinition` directly
- [ ] All four quality gates pass with zero errors/warnings
- [ ] All new unit tests pass

#### 2.6 Success Criteria

- `grep -r "resolve_recruitable_creature_id" . --include="*.rs"` returns zero matches.
- `grep -r "normalize_lookup_key" . --include="*.rs"` returns zero matches.
- `CharacterDefinition` has a `creature_id` field that round-trips through RON.
- A `RecruitableCharacter` map event with a `CharacterDefinition.creature_id: Some(N)`
  spawns a `CreatureVisual { creature_id: N }` entity — confirmed by unit test.
- All existing recruitable-character tests continue to pass.

---

### Phase 3: Add the `CreatureBound` Trait

Define the `CreatureBound` trait in a new file in the domain layer, implement it
for all three definition types, and update the rendering system call-sites to use
the trait method instead of direct field access. This is the "Optional but
Recommended" item from the spec; by making it a mandatory phase it prevents future
drift.

#### 3.1 Foundation Work — New Trait File

**File to create**: `src/domain/world/creature_binding.rs`

Add SPDX header.

Define the trait:

```rust
use crate::domain::types::CreatureId;

/// Implemented by any definition type that may carry a reference to a
/// `CreatureDefinition` in the creature registry.
///
/// # Examples
///
/// ```
/// use antares::domain::world::creature_binding::CreatureBound;
/// use antares::domain::combat::database::MonsterDefinition;
///
/// let def = MonsterDefinition { /* ... */ creature_id: Some(7), /* ... */ };
/// assert_eq!(def.creature_id(), Some(7));
/// ```
pub trait CreatureBound {
    /// Returns the optional `CreatureId` that links this definition to a mesh
    /// asset in the creature registry. Returns `None` when no visual binding
    /// has been set.
    fn creature_id(&self) -> Option<CreatureId>;
}
```

Implement the trait for all three definition types in the same file:

- `impl CreatureBound for MonsterDefinition { fn creature_id(&self) -> Option<CreatureId> { self.creature_id } }`
- `impl CreatureBound for NpcDefinition { fn creature_id(&self) -> Option<CreatureId> { self.creature_id } }`
- `impl CreatureBound for CharacterDefinition { fn creature_id(&self) -> Option<CreatureId> { self.creature_id } }`

Add the necessary `use` imports for all three definition types at the top of the
file.

**File to modify**: `src/domain/world/mod.rs`

- Add `pub mod creature_binding;`
- Add `pub use creature_binding::CreatureBound;` to the `pub use` block so
  callers can write `use antares::domain::world::CreatureBound`.

#### 3.2 Integrate Trait into Rendering System

**File to modify**: `src/game/systems/map.rs`

In `resolve_encounter_creature_id` (now reads `monster_def.creature_id` directly
after Phase 1), add a use import:

```rust
use crate::domain::world::CreatureBound;
```

Rewrite the inner body to use the trait method for clarity and consistency:

```rust
for monster_id in monster_group {
    if let Some(monster_def) = content.0.monsters.get_monster(*monster_id) {
        if let Some(id) = monster_def.creature_id() {
            return Some(id);
        }
    }
}
```

In the `RecruitableCharacter` branch (updated in Phase 2):

```rust
if let Some(creature_id) = content
    .0
    .characters
    .get_character(character_id)
    .and_then(|def| def.creature_id())
```

In the `NpcDialogue` / NPC branch, update the inline `npc_def.creature_id` access
to `npc_def.creature_id()` for consistency.

#### 3.3 Testing Requirements

Run the full quality gate sequence.

Tests to add in `src/domain/world/creature_binding.rs` inside `mod tests`:

| Test name | What it verifies |
|---|---|
| `test_creature_bound_monster_some` | `MonsterDefinition { creature_id: Some(3), .. }.creature_id() == Some(3)` |
| `test_creature_bound_monster_none` | `MonsterDefinition { creature_id: None, .. }.creature_id() == None` |
| `test_creature_bound_npc_some` | `NpcDefinition { creature_id: Some(1000), .. }.creature_id() == Some(1000)` |
| `test_creature_bound_npc_none` | `NpcDefinition { creature_id: None, .. }.creature_id() == None` |
| `test_creature_bound_character_some` | `CharacterDefinition { creature_id: Some(2000), .. }.creature_id() == Some(2000)` |
| `test_creature_bound_character_none` | `CharacterDefinition { creature_id: None, .. }.creature_id() == None` |
| `test_creature_bound_all_three_types_consistent` | Given all three definitions with the same `creature_id` value, the trait method returns identical `Option<CreatureId>` |

#### 3.4 Deliverables

- [ ] `src/domain/world/creature_binding.rs` — `CreatureBound` trait + three `impl` blocks + seven unit tests; SPDX header
- [ ] `src/domain/world/mod.rs` — `pub mod creature_binding` and `pub use creature_binding::CreatureBound`
- [ ] `src/game/systems/map.rs` — all three spawn branches use `def.creature_id()` via the trait; import added
- [ ] All four quality gates pass with zero errors/warnings
- [ ] All seven new trait unit tests pass

#### 3.5 Success Criteria

- `cargo nextest run --all-features` reports zero failures.
- The trait method `creature_id()` is callable on all three definition types via
  a generic bound `T: CreatureBound`, verified by the consistency test.
- All three spawn branches in `spawn_map` (`Encounter`, `NpcDialogue`,
  `RecruitableCharacter`) use the trait method rather than direct field access.

---

### Phase 4: Campaign Builder SDK Editor Updates

Update all three Campaign Builder editors so that every definition type exposes a
consistent Browse/Clear/tooltip creature picker in its edit form. Each sub-section
is independent but they share the same picker pattern modelled after the existing
portrait picker in `characters_editor.rs`.

The portrait picker pattern to follow is:
- State flag `*_picker_open: bool` on the editor state struct (serialised as
  `#[serde(skip)]`).
- `apply_selected_*` helper that writes the chosen ID to the buffer and clears
  the flag.
- A modal `egui::Window` shown when the flag is `true`, populated by loading
  creature names via `CreatureAssetManager::load_all_creatures()`.
- In the form: a read-only display of the current value, a Browse button that
  sets the flag, and a Clear button that sets the buffer to empty string.

#### 4.1 Feature Work — Monsters Editor Creature Picker

**File to modify**: `sdk/campaign_builder/src/monsters_editor.rs`

The `edit_buffer` is a raw `MonsterDefinition` (no separate `MonsterEditBuffer`
struct). Because `MonsterDefinition.creature_id` is `Option<CreatureId>` (not a
`String`), the display and editing approach is:

1. Add `creature_picker_open: bool` to `MonstersEditorState` (L25–35) with
   `#[serde(skip)]` and `Default = false`.

2. Add `apply_selected_creature_id(&mut self, id: Option<CreatureId>)` method:
   sets `self.edit_buffer.creature_id = id` and clears `creature_picker_open`.

3. In `show_form` (L745), add a **"Visual Asset"** section (a `ui.group` block)
   between the Identity section and the Combat Stats section:

   ```
   ┌─ Visual Asset ──────────────────────────────────────────────────────────┐
   │  Creature ID:  [ 7 (read-only label) ]  [ Browse… ]  [ Clear ]  [ℹ️]   │
   │  Resolved:     "Goblin" (looked up from registry)                       │
   └─────────────────────────────────────────────────────────────────────────┘
   ```

   - Use `ui.label` to show the current numeric ID (or "None" when absent).
   - The **Browse…** button opens the creature picker modal:
     - `egui::Window::new("Select Creature").id(egui::Id::new("monster_creature_picker"))`.
     - Contains a `ScrollArea` with `id_salt("monster_creature_picker_scroll")`.
     - Populated by `creature_manager.load_all_creatures()` (passed in as a
       parameter to `show_form`); shows `id — name` pairs as selectable rows.
     - On row click: `self.apply_selected_creature_id(Some(selected_id))`.
   - The **Clear** button calls `self.apply_selected_creature_id(None)`.
   - The **ℹ️** tooltip text (set via `.on_hover_text()`):
     `"Links this monster to a procedural mesh creature definition. When set,
      the monster spawns as a 3-D creature mesh on the map instead of a
      sprite placeholder."`.
   - The resolved creature name is shown via a `ui.label` in dim colour when
     the registry contains a matching entry.

4. Update `show_monster_details` and `show_preview_static` to show the resolved
   creature name (or "No creature asset") alongside the numeric ID.

5. Update `show_form`'s signature to accept `creature_manager: Option<&CreatureAssetManager>`
   so the picker list can be populated. Pass it through from `pub fn show`.

6. Update `pub fn show` to accept `creature_manager: Option<&CreatureAssetManager>`
   and forward it to `show_form`. Update all call-sites of `show` in the parent
   `CampaignBuilderApp` to pass the manager.

#### 4.2 Feature Work — Characters Editor Creature Picker

**File to modify**: `sdk/campaign_builder/src/characters_editor.rs`

Three coordinated changes:

**A — Buffer field:**
Add `pub creature_id: String` to `CharacterEditBuffer` (L100–146) after
`portrait_id`:

```rust
/// Creature ID for procedural mesh rendering (empty string = None)
pub creature_id: String,
```

Add `creature_id: String::new()` to `impl Default for CharacterEditBuffer` (L148).

**B — `start_edit_character` population** (L229–283):
Add the following line inside the `CharacterEditBuffer { .. }` literal:

```rust
creature_id: character.creature_id
    .map_or(String::new(), |id| id.to_string()),
```

**C — `save_character` write-back** (L285–560):
Add creature_id to the `CharacterDefinition { .. }` literal:

```rust
creature_id: if self.buffer.creature_id.is_empty() {
    None
} else {
    self.buffer.creature_id.trim().parse::<CreatureId>().ok()
},
```

**D — Edit form UI** (in `show_character_form` after the portrait row, ~L1723):

Add a "Creature ID" row to the Visual section, following the same Browse/Clear/tooltip
pattern as the portrait picker:

- Add `creature_picker_open: bool` to `CharactersEditorState` with `#[serde(skip)]`.
- Add `apply_selected_creature_id(&mut self, id: Option<String>)` helper.
- In `show_character_form`, add after the Portrait row:

  ```
  Creature ID: [ 7 ]  [ Browse… ]  [ Clear ]  [ℹ️]
  Resolved:    "Goblin"
  ```

  Same modal window pattern as the Monsters Editor picker, using
  `egui::Id::new("character_creature_picker")` and
  `ScrollArea` with `id_salt("character_creature_picker_scroll")`.

**E — Preview and list view:**
In the character list view and preview panel, display the resolved creature name
(or "No creature asset") alongside the portrait.

#### 4.3 Feature Work — NPC Editor Creature Picker Upgrade

**File to modify**: `sdk/campaign_builder/src/npc_editor.rs`

The NPC editor already has `creature_id: String` in `NpcEditBuffer` (L162) and
parses it in `save_npc` (L1528–1532). No structural changes to the buffer or save
logic are needed. Two UI changes are required:

1. **Add the Browse/Clear picker to `show_edit_view`**: The field currently has
   zero UI exposure. Add a "Creature ID" row to the edit form's Visual section
   (near the `sprite_sheet` / `sprite_index` rows):

   - Add `creature_picker_open: bool` to `NpcEditorState` with `#[serde(skip)]`.
   - Add `apply_selected_creature_id(&mut self, id: String)` helper.
   - In `show_edit_view`, add after the sprite index row:

     ```
     Creature ID: [ 7 (text input) ]  [ Browse… ]  [ Clear ]  [ℹ️]
     ```

     Keep the raw `TextEdit::singleline` for keyboard-first users but add the
     Browse button that opens the modal picker:
     `egui::Window::new("Select Creature").id(egui::Id::new("npc_creature_picker"))`.
     `ScrollArea` with `id_salt("npc_creature_picker_scroll")`.

2. **Upgrade `show_preview`**: Currently shows a plain `ui.label(creature_id.to_string())`
   at L628–634. Replace with a two-line display:

   ```
   Creature ID: 1000
   Asset:       "Village Elder" (or "⚠ Unknown" if not in registry)
   ```

   Pass the resolved name via `creature_manager: Option<&CreatureAssetManager>`.

#### 4.4 Configuration Updates

**File to modify** (all three editors' `pub fn show` signatures):

All three `show` methods must accept `creature_manager: Option<&CreatureAssetManager>`
so the picker can enumerate creatures from the registry without loading them at
widget-render time.

Pre-load the creature list once per frame in the parent `CampaignBuilderApp` (or
the editor's `show` method itself if it already has access to the campaign dir)
and pass it down. Avoid loading inside the inner form render loop to prevent
per-frame file I/O.

#### 4.5 Testing Requirements

Run the full quality gate sequence.

**Monsters editor** — add in `monsters_editor.rs` inside `mod tests`:

| Test name | What it verifies |
|---|---|
| `test_monsters_editor_creature_id_roundtrips_through_form` | Set `edit_buffer.creature_id = Some(42)`, call `apply_selected_creature_id(Some(42))`, assert `edit_buffer.creature_id == Some(42)` |
| `test_monsters_editor_clear_creature_id` | After `apply_selected_creature_id(None)`, assert `edit_buffer.creature_id == None` |
| `test_monsters_editor_default_monster_creature_id_is_none` | `default_monster().creature_id == None` |

**Characters editor** — add in `characters_editor.rs` inside `mod tests`:

| Test name | What it verifies |
|---|---|
| `test_characters_editor_creature_id_roundtrips_through_form` | `start_edit_character` with `creature_id: Some(42)` → buffer `"42"`; `save_character` → `Some(42)` |
| `test_characters_editor_creature_id_empty_string_saves_none` | Buffer `""` → `save_character` → `creature_id: None` |
| `test_characters_editor_creature_id_invalid_string_saves_none` | Buffer `"not_a_number"` → `save_character` → `creature_id: None` |
| `test_creature_picker_open_flag` | Toggle `creature_picker_open`; assert initial state is `false` |
| `test_apply_selected_creature_id_sets_buffer` | `apply_selected_creature_id(Some("7".to_string()))` sets buffer to `"7"` and closes picker |

**NPC editor** — verify that the following existing tests still pass unchanged (they
already cover `creature_id` round-trip):

- `test_start_edit_npc_populates_sprite_fields`
- `test_save_npc_edit_mode`

Add in `npc_editor.rs`:

| Test name | What it verifies |
|---|---|
| `test_npc_creature_picker_initial_state` | `creature_picker_open` starts `false` |
| `test_npc_apply_selected_creature_id_updates_buffer` | `apply_selected_creature_id("1000".to_string())` sets buffer and closes picker |

Verify that `sdk/AGENTS.md` egui ID rules are satisfied for all new UI widgets:
every `ScrollArea` has `id_salt`, every `Window` has a unique `id`, no same-frame
guards wrap panels, every layout-state mutation calls `request_repaint()` where
applicable.

#### 4.6 Deliverables

- [ ] `sdk/campaign_builder/src/monsters_editor.rs` — `creature_picker_open` state; `apply_selected_creature_id`; Visual Asset section in `show_form`; resolved name in `show_monster_details` and `show_preview_static`; `show_form` and `show` accept `creature_manager`
- [ ] `sdk/campaign_builder/src/characters_editor.rs` — `creature_id: String` in `CharacterEditBuffer`; `start_edit_character` populates it; `save_character` writes it; Visual section in `show_character_form` with Browse/Clear/tooltip; creature name in list view and preview
- [ ] `sdk/campaign_builder/src/npc_editor.rs` — Browse/Clear creature picker row added to `show_edit_view`; `show_preview` upgraded to show resolved name; `creature_picker_open` state
- [ ] All three `show` methods accept `creature_manager: Option<&CreatureAssetManager>`
- [ ] All four quality gates pass with zero errors/warnings
- [ ] All new editor unit tests pass; all existing NPC editor tests continue to pass

#### 4.7 Success Criteria

- **Monsters Editor**: `show_form` exposes a working "Creature ID" Visual Asset
  section with Browse/Clear that round-trips `creature_id` correctly through
  save and reload (verified by `test_monsters_editor_creature_id_roundtrips_through_form`).
- **Characters Editor**: `CharacterEditBuffer.creature_id`, `start_edit_character`,
  and `save_character` all participate in the round-trip; the edit form exposes a
  Browse/Clear picker in the Visual section.
- **NPC Editor**: the creature_id field now has a visible edit widget in the form
  (no longer zero-UI-exposure); `show_preview` shows the resolved creature name.
- All egui `id_salt` / `Id::new` rules from `sdk/AGENTS.md` are satisfied.
- `cargo nextest run --all-features` reports zero failures.

---

## Documentation

After all four phases pass quality gates, update
`docs/explanation/implementations.md` by prepending a new section:

```
## Unified Creature Asset Binding

### Phase 1: Rename visual_id → creature_id on Monster Types
### Phase 2: Add creature_id to CharacterDefinition
### Phase 3: CreatureBound Trait
### Phase 4: Campaign Builder SDK Editor Updates
```

Each section must list:

- Files created / modified
- Deliverables checklist (copied and checked from above)
- A one-paragraph summary of what changed and why

Also update `docs/reference/architecture.md`:

- Section 4.7 (`CharacterDefinition`): add `creature_id` field row.
- Any table that previously listed `visual_id` for `MonsterDefinition`: update
  to `creature_id`.
- Add a short section describing the `CreatureBound` trait and its three
  implementors.
