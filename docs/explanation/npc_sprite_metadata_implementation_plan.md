# NPC Sprite Metadata Implementation Plan

## Plan: NPC Sprite Metadata

**TL;DR** — Add optional per-NPC sprite metadata so NPCs can specify a `SpriteReference` (sheet + index + optional animation). Update runtime spawn logic to prefer per-NPC sprites, extend asset scanning to include referenced sprite sheets, and add editor UI to author sprite metadata. Follow required Phases 1–3 (metadata, spawn integration, editor/asset integration) plus optional Phase 4 (post-MVP enhancements).

**Steps (short):**

1. Add `sprite: Option<SpriteReference>` to `NpcDefinition` (`antares/src/domain/world/npc.rs` line ~45).
2. Propagate sprite into `ResolvedNpc` (`antares/src/domain/world/types.rs`).
3. Update `spawn_map` (`antares/src/game/systems/map.rs`) to prefer `resolved_npc.sprite` over `DEFAULT_NPC_SPRITE_PATH`.
4. Extend asset scanning in `sdk/campaign_builder/src/asset_manager.rs::scan_npcs_references` to detect sprite sheet references.
5. Add editor UI in `sdk/campaign_builder/src/npc_editor.rs` for sprite selection and persist to `npcs.ron`.
6. Update `docs/explanation/implementations.md` with implementation summary.

**Open Questions (MUST answer before proceeding to implementation):**

1. **Per-placement sprite overrides** — Should `NpcPlacement` support per-instance sprite customization?

   - **Option A**: Include now — Add `sprite_override: Option<SpriteReference>` to `NpcPlacement` in Phase 1
     - _Pros_: Complete feature, supports map-specific NPC visuals (e.g., same NPC definition with different sprites per dungeon)
     - _Cons_: Increases Phase 1 scope by ~2 hours, adds complexity to placement resolution
   - **Option B**: Defer to Phase 4 (post-MVP) — Only support definition-level sprites in initial implementation
     - _Pros_: Smaller Phase 1, faster validation, simpler initial implementation
     - _Cons_: Requires later migration for per-instance overrides, may need data migration
   - **Recommendation**: **Option B (defer)** — keeps Phase 1 focused on core metadata; can add overrides as enhancement

2. **Editor animation config complexity** — How much animation control in Campaign Builder V1?

   - **Option A**: Full config — Allow editing `SpriteAnimation` (frames, fps, loop) in editor UI
     - _Pros_: Complete authoring experience, no manual RON editing needed
     - _Cons_: Complex UI, Phase 3 scope increases by ~4 hours
   - **Option B**: Index-only — Only allow sheet + index selection, no animation parameter editing
     - _Pros_: Simpler UI, faster implementation, animations can be added manually to RON files
     - _Cons_: Campaign authors must hand-edit RON for animations
   - **Recommendation**: **Option B (index-only)** — reduces editor complexity; advanced users can edit RON directly

3. **Placeholder asset strategy** — How to provide the missing `npc_placeholder.png`?
   - **Option A**: Copy existing — Use `assets/sprites/actors/npcs_town.png` sprite 0 as placeholder (already exists, 32×48)
     - _Pros_: Reuses validated assets, no new generation needed, immediate fix
     - _Cons_: Not semantically a "placeholder" (shows actual town NPC sprite)
   - **Option B**: Generate new — Run `scripts/generate_placeholder_sprites.py` to create single-sprite placeholder sheet
     - _Pros_: Semantically correct placeholder (distinct visual), clean asset organization
     - _Cons_: Requires script execution, adds new asset to repo
   - **Recommendation**: **Option A (copy existing)** — pragmatic solution, fixes missing texture warnings immediately

---

## Overview

Add explicit sprite metadata support to NPCs so they can declare a `SpriteReference` (sheet_path, sprite_index, optional `SpriteAnimation`, optional material properties). This enables per-NPC visuals (static or animated), better asset hygiene (asset scanning detects sprite usage), and authoring via the Campaign Builder.

**Why this is needed:**

- Current behavior spawns **all NPCs** using a single `DEFAULT_NPC_SPRITE_PATH` constant. NPCs cannot opt into specific visuals without code changes.
- Aligns with sprite support plan (Phase 1 metadata + Phase 3 actor spawning + Phase 5 editor integration).
- Enables data-driven NPC appearance, supporting visual variety in campaigns without hardcoding.

---

## Current State Analysis

### Existing Infrastructure

**Domain Layer (Data Structures):**

- `SpriteReference` struct exists in `antares/src/domain/world/types.rs` (canonical representation for sprites):
  - Fields: `sheet_path: String`, `sprite_index: u32`, `animation: Option<SpriteAnimation>`, `material_properties: Option<SpriteMaterialProperties>`
- `SpriteAnimation` struct exists in same file (frame count, FPS, looping config)
- `NpcDefinition` struct exists in `antares/src/domain/world/npc.rs` (currently has `name`, `portrait_id`, etc. but NO sprite field)
- `ResolvedNpc` struct exists in `antares/src/domain/world/types.rs` (runtime representation of placed NPC)

**Application Layer (Spawning):**

- Actor spawning uses `spawn_actor_sprite` (`antares/src/game/systems/actor.rs`) — accepts `SpriteReference` parameter
- `spawn_map` function (`antares/src/game/systems/map.rs`) spawns NPCs using hardcoded `DEFAULT_NPC_SPRITE_PATH = "sprites/placeholders/npc_placeholder.png"`
- Existing spawn code creates `SpriteReference { sheet_path: DEFAULT_NPC_SPRITE_PATH.into(), sprite_index: 0, animation: None, material_properties: None }`

**SDK Layer (Editor & Asset Management):**

- Asset scanning (`sdk/campaign_builder/src/asset_manager.rs::scan_npcs_references`) currently scans NPC portrait images (via `portrait_id` field) but does NOT scan sprite sheet references
- Campaign Builder UI (`sdk/campaign_builder/src/*`) has NPC editing flows but no sprite sheet selector UI component
- Existing sprite asset helpers (`get_sprites_for_sheet`, etc.) available for UI integration

### Identified Issues

1. **No per-NPC sprite metadata** — All NPCs use identical placeholder; visual variety requires ad-hoc code changes or global default replacement
2. **Asset manager blind spot** — Sprite sheets referenced by NPCs (once metadata exists) won't be detected by `scan_npcs_references`, causing missing asset warnings
3. **Campaign Builder UX gap** — No UI workflow for selecting/previewing NPC sprites or configuring animations
4. **Missing placeholder asset** — `DEFAULT_NPC_SPRITE_PATH` references `sprites/placeholders/npc_placeholder.png` which may not exist (causes runtime warnings)

---

## Implementation Phases

### Phase 1: Core Domain Implementation (REQUIRED)

**Scope**: Add sprite metadata to `NpcDefinition` and `ResolvedNpc` domain types with full backward compatibility.

#### 1.1 Foundation Work — Architecture Verification

**MANDATORY — Consult Architecture Document FIRST (Golden Rule 1):**

Execute these verification steps BEFORE writing any code:

1. **Read** `docs/reference/architecture.md` **Section 4 (Data Structures)** — Verify `SpriteReference` fields match EXACTLY:

   - Confirm field names: `sheet_path`, `sprite_index`, `animation`, `material_properties`
   - Confirm types: `String`, `u32`, `Option<SpriteAnimation>`, `Option<SpriteMaterialProperties>`
   - **IF** any deviation required, STOP and document in `docs/explanation/` before proceeding

2. **Read** `docs/reference/architecture.md` **Section 3.2 (Module Structure)** — Confirm module placement:

   - Verify `npc.rs` belongs in `antares/src/domain/world/` (domain layer)
   - Verify `types.rs` belongs in `antares/src/domain/world/` (domain layer)
   - **IF** creating new modules, verify against Section 3.2 allowed structure

3. **Read** `docs/reference/architecture.md` **Section 7.1 (Data Formats)** — Confirm RON format usage:

   - Verify `npcs.ron` uses RON format (not JSON/YAML)
   - Confirm serialization uses `#[serde(default)]` for optional fields

4. **Read** `docs/reference/architecture.md` **Section 4.6 (Type Aliases)** — Verify no raw type usage:

   - Confirm `sprite_index` uses `u32` (standard index type, no alias defined)
   - Verify no `usize` or raw integer types where aliases exist (e.g., `ItemId`, `SpellId`)

5. **Verify existing constant usage** (`antares/src/game/systems/map.rs`):
   - Confirm `DEFAULT_NPC_SPRITE_PATH` is defined as constant (not magic string)
   - Note current value: `"sprites/placeholders/npc_placeholder.png"`

**Checklist (complete before proceeding to 1.2):**

- [ ] Architecture Section 4 reviewed — `SpriteReference` structure confirmed
- [ ] Architecture Section 3.2 reviewed — module placement confirmed
- [ ] Architecture Section 7.1 reviewed — RON format confirmed
- [ ] Architecture Section 4.6 reviewed — type alias usage confirmed
- [ ] `DEFAULT_NPC_SPRITE_PATH` constant confirmed in `map.rs`

#### Phase 1 Execution Sequence

**Execute in this EXACT order (do not skip or reorder steps):**

**Step 1.2.1**: Modify `NpcDefinition` struct (file: `antares/src/domain/world/npc.rs`, struct definition ~line 15)

**Step 1.2.2**: Modify `ResolvedNpc` struct (file: `antares/src/domain/world/types.rs`)

**Step 1.2.3**: Write unit tests (4 tests total)

**Step 1.2.4**: Run quality gates (all must pass before claiming Phase 1 complete)

**Step 1.2.5**: Verify Phase 1 success criteria checklist

#### 1.2 Add Foundation Functionality

**Step 1.2.1 — Modify `NpcDefinition` struct:**

- **File**: `antares/src/domain/world/npc.rs`
- **Symbol**: `NpcDefinition` (struct definition starts ~line 15)
- **Location**: Add field after `portrait_id` field (currently ~line 45)

**Actions:**

1. Add field declaration:

   ````rust
   /// Optional sprite reference for this NPC's visual representation.
   ///
   /// When `Some`, the NPC will use the specified sprite sheet and index.
   /// When `None`, falls back to `DEFAULT_NPC_SPRITE_PATH` (placeholder).
   ///
   /// Backward compatibility: Old RON files without this field will deserialize
   /// with `sprite = None` via `#[serde(default)]`.
   ///
   /// # Examples
   ///
   /// ```ron
   /// NpcDefinition(
   ///     name: "Town Guard",
   ///     portrait_id: Some(5),
   ///     sprite: Some(SpriteReference(
   ///         sheet_path: "sprites/actors/npcs_town.png",
   ///         sprite_index: 3,
   ///         animation: None,
   ///         material_properties: None,
   ///     )),
   /// )
   /// ```
   #[serde(default)]
   pub sprite: Option<SpriteReference>,
   ````

2. Add builder method (after existing `new` method, do NOT modify `new` signature):

   ````rust
   /// Sets the sprite reference for this NPC (builder pattern).
   ///
   /// # Arguments
   ///
   /// * `sprite` - The sprite reference to use for this NPC
   ///
   /// # Returns
   ///
   /// Self with sprite field set
   ///
   /// # Examples
   ///
   /// ```
   /// use antares::domain::world::npc::NpcDefinition;
   /// use antares::domain::world::types::SpriteReference;
   ///
   /// let sprite = SpriteReference {
   ///     sheet_path: "sprites/actors/npcs_town.png".to_string(),
   ///     sprite_index: 2,
   ///     animation: None,
   ///     material_properties: None,
   /// };
   ///
   /// let npc = NpcDefinition::new("Guard")
   ///     .with_sprite(sprite);
   /// ```
   pub fn with_sprite(mut self, sprite: SpriteReference) -> Self {
       self.sprite = Some(sprite);
       self
   }
   ````

3. **IMPORTANT**: Do NOT modify `NpcDefinition::new` signature (backward compatibility requirement)

4. **IF creating new `.rs` files** (not applicable here, modifying existing), add SPDX headers:
   ```rust
   // SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
   // SPDX-License-Identifier: Apache-2.0
   ```
   (Existing file modifications do NOT need headers added)

**Step 1.2.2 — Modify `ResolvedNpc` struct:**

- **File**: `antares/src/domain/world/types.rs`
- **Symbol**: `ResolvedNpc` (struct definition)

**Actions:**

1. Add field to `ResolvedNpc` struct:

   ```rust
   /// Optional sprite reference from NPC definition.
   /// When `Some`, runtime spawning prefers this over default placeholder.
   pub sprite: Option<SpriteReference>,
   ```

2. Modify `ResolvedNpc::from_placement_and_definition` function:

   - Locate the function that constructs `ResolvedNpc` from placement + definition
   - Add line copying sprite field:
     ```rust
     sprite: definition.sprite.clone(),
     ```
   - **Note**: Use `.clone()` because `SpriteReference` may not be `Copy` (verify derive attributes)

3. Ensure backward compatibility:
   - When `definition.sprite` is `None`, `resolved_npc.sprite` is `None` (fallback to `DEFAULT_NPC_SPRITE_PATH` happens in spawn logic)

#### 1.3 Integrate Foundation Work

**No additional integration needed** — Phase 1 is pure domain layer changes. Integration happens in Phase 2 (runtime spawn logic).

**Verification**:

- `NpcDefinition` and `ResolvedNpc` serialize/deserialize correctly
- No breaking changes to existing code (all changes are additive)

#### 1.4 Testing Requirements

**Add 4 unit tests in `#[cfg(test)] mod tests` blocks:**

**Test 1: `test_npc_definition_serializes_with_sprite_field_present`**

- **File**: `antares/src/domain/world/npc.rs` (in `#[cfg(test)] mod tests` at end of file)
- **Purpose**: Verify `NpcDefinition` with `sprite` serializes to RON and deserializes correctly
- **Setup**:
  ```rust
  let sprite = SpriteReference {
      sheet_path: "sprites/test/custom.png".to_string(),
      sprite_index: 42,
      animation: None,
      material_properties: None,
  };
  let npc = NpcDefinition::new("Test NPC").with_sprite(sprite.clone());
  ```
- **Actions**:
  1. Serialize to RON string: `let ron_str = ron::to_string(&npc).unwrap();`
  2. Deserialize back: `let deserialized: NpcDefinition = ron::from_str(&ron_str).unwrap();`
- **Assertions**:
  ```rust
  assert!(deserialized.sprite.is_some());
  assert_eq!(deserialized.sprite.as_ref().unwrap().sheet_path, "sprites/test/custom.png");
  assert_eq!(deserialized.sprite.as_ref().unwrap().sprite_index, 42);
  ```
- **Expected**: Test passes, round-trip preserves sprite field

**Test 2: `test_npc_definition_deserializes_without_sprite_field_defaults_none`**

- **File**: `antares/src/domain/world/npc.rs` tests
- **Purpose**: Verify backward compatibility — old RON without `sprite` field deserializes successfully
- **Setup** (RON string without sprite field):
  ```rust
  let ron_str = r#"
  NpcDefinition(
      name: "Old NPC",
      portrait_id: Some(5),
  )
  "#;
  ```
- **Actions**:
  ```rust
  let npc: NpcDefinition = ron::from_str(ron_str).unwrap();
  ```
- **Assertions**:
  ```rust
  assert!(npc.sprite.is_none());
  assert_eq!(npc.name, "Old NPC");
  ```
- **Expected**: Deserialization succeeds, `sprite` defaults to `None` via `#[serde(default)]`

**Test 3: `test_resolved_npc_from_placement_copies_sprite_field_when_present`**

- **File**: `antares/src/domain/world/types.rs` tests
- **Purpose**: Verify `ResolvedNpc::from_placement_and_definition` copies sprite from definition
- **Setup**:
  ```rust
  let sprite = SpriteReference {
      sheet_path: "sprites/actors/knight.png".to_string(),
      sprite_index: 7,
      animation: None,
      material_properties: None,
  };
  let definition = NpcDefinition::new("Knight").with_sprite(sprite.clone());
  let placement = NpcPlacement::new(/* appropriate params */);
  ```
- **Actions**:
  ```rust
  let resolved = ResolvedNpc::from_placement_and_definition(&placement, &definition);
  ```
- **Assertions**:
  ```rust
  assert!(resolved.sprite.is_some());
  assert_eq!(resolved.sprite.as_ref().unwrap().sheet_path, "sprites/actors/knight.png");
  assert_eq!(resolved.sprite.as_ref().unwrap().sprite_index, 7);
  ```
- **Expected**: Test passes, sprite field copied correctly

**Test 4: `test_resolved_npc_from_placement_sprite_none_when_definition_none`**

- **File**: `antares/src/domain/world/types.rs` tests
- **Purpose**: Verify resolved NPC has `sprite = None` when definition has no sprite
- **Setup**:
  ```rust
  let definition = NpcDefinition::new("Generic NPC"); // no sprite set
  let placement = NpcPlacement::new(/* appropriate params */);
  ```
- **Actions**:
  ```rust
  let resolved = ResolvedNpc::from_placement_and_definition(&placement, &definition);
  ```
- **Assertions**:
  ```rust
  assert!(resolved.sprite.is_none());
  ```
- **Expected**: Test passes, `None` propagates correctly

**Update existing test** (if it exists):

- Find `test_npc_definition_serialization_defaults` or similar
- Add assertion: `assert!(npc.sprite.is_none());` to verify default deserialization

#### 1.5 Deliverables (Phase 1)

**Complete these in order (check off as you finish each):**

- [ ] **1.5.1**: Architecture.md verification checklist (Section 1.1) completed
- [ ] **1.5.2**: Add `pub sprite: Option<SpriteReference>` field to `NpcDefinition` struct (after `portrait_id` field, ~line 45)
- [ ] **1.5.3**: Add `#[serde(default)]` attribute to `sprite` field
- [ ] **1.5.4**: Add doc comment to `sprite` field (include semantics, backward-compat guarantee, RON example)
- [ ] **1.5.5**: Add `pub fn with_sprite(mut self, sprite: SpriteReference) -> Self` builder method
- [ ] **1.5.6**: Add doc comment with example to `with_sprite` method
- [ ] **1.5.7**: Add `pub sprite: Option<SpriteReference>` field to `ResolvedNpc` struct
- [ ] **1.5.8**: Add doc comment to `ResolvedNpc.sprite` field
- [ ] **1.5.9**: Modify `ResolvedNpc::from_placement_and_definition` to copy `definition.sprite` field
- [ ] **1.5.10**: Add test `test_npc_definition_serializes_with_sprite_field_present`
- [ ] **1.5.11**: Add test `test_npc_definition_deserializes_without_sprite_field_defaults_none`
- [ ] **1.5.12**: Add test `test_resolved_npc_from_placement_copies_sprite_field_when_present`
- [ ] **1.5.13**: Add test `test_resolved_npc_from_placement_sprite_none_when_definition_none`
- [ ] **1.5.14**: Run `cargo fmt --all` (verify no diffs after formatting)
- [ ] **1.5.15**: Run `cargo check --all-targets --all-features` (verify 0 errors)
- [ ] **1.5.16**: Run `cargo clippy --all-targets --all-features -- -D warnings` (verify 0 warnings)
- [ ] **1.5.17**: Run `cargo nextest run --all-features` (verify all tests pass)
- [ ] **1.5.18**: Verify Phase 1 Success Criteria (Section 1.6)

#### 1.6 Success Criteria (Phase 1)

**ALL of the following must be TRUE before proceeding to Phase 2:**

**Compilation & Quality Gates:**

- [ ] `cargo fmt --all` produces no diffs (all code formatted)
- [ ] `cargo check --all-targets --all-features` exits with 0 errors
  - Expected output: `Finished dev [unoptimized + debuginfo] target(s) in X.XXs`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` exits with 0 warnings
  - Expected output: `Finished dev [unoptimized + debuginfo] target(s) in X.XXs`
- [ ] `cargo nextest run --all-features` shows all tests passing
  - Expected output pattern: `test result: ok. X passed; 0 failed; 0 ignored`

**Test Results:**

- [ ] `test_npc_definition_serializes_with_sprite_field_present` — PASSED
- [ ] `test_npc_definition_deserializes_without_sprite_field_defaults_none` — PASSED
- [ ] `test_resolved_npc_from_placement_copies_sprite_field_when_present` — PASSED
- [ ] `test_resolved_npc_from_placement_sprite_none_when_definition_none` — PASSED

**Backward Compatibility:**

- [ ] Existing `NpcDefinition::new` signature unchanged (no breaking changes)
- [ ] RON files without `sprite` field deserialize successfully
- [ ] Deserialized NPCs without sprite have `sprite = None` (verified by Test 2)

**Code Quality:**

- [ ] No `unwrap()` calls without justification comments
- [ ] All public items (`sprite` field, `with_sprite` method) have `///` doc comments
- [ ] Doc comments include examples (verified in Steps 1.5.4 and 1.5.6)

**IF ANY criterion fails**: STOP, fix the issue, re-run quality gates, then retry checklist.

---

### Phase 2: Runtime Integration & Asset Scanning (REQUIRED)

**Scope**: Update map spawning to use per-NPC sprites, extend asset manager to scan sprite references, add integration tests.

#### Phase 2 Execution Sequence

**Execute in this EXACT order:**

**Step 2.1.1**: Modify `spawn_map` to prefer `resolved_npc.sprite`

**Step 2.1.2**: Verify `spawn_actor_sprite` handles `SpriteReference` fields correctly

**Step 2.2.1**: Add integration test for custom sprite spawning

**Step 2.2.2**: Update existing test for default sprite fallback

**Step 2.2.3**: Modify `scan_npcs_references` to scan sprite sheets

**Step 2.2.4**: Add asset manager test

**Step 2.3.1**: (Optional) Configure placeholder asset

**Step 2.4.1**: Run quality gates

**Step 2.4.2**: Verify Phase 2 success criteria

#### 2.1 Feature Work — Spawn Logic

**Step 2.1.1 — Modify `spawn_map` function:**

- **File**: `antares/src/game/systems/map.rs`
- **Symbol**: `spawn_map` (function that spawns NPCs during map loading)
- **Current behavior**: Creates `SpriteReference` with `DEFAULT_NPC_SPRITE_PATH` for all NPCs

**Actions:**

1. Locate NPC spawning loop (iterates `resolved_npcs`)
2. Find line creating `SpriteReference` with `DEFAULT_NPC_SPRITE_PATH`
3. Replace with conditional logic:
   ```rust
   // Prefer per-NPC sprite if defined, otherwise use default placeholder
   let sprite_ref = resolved_npc.sprite.clone().unwrap_or_else(|| {
       SpriteReference {
           sheet_path: DEFAULT_NPC_SPRITE_PATH.into(),
           sprite_index: 0,
           animation: None,
           material_properties: None,
       }
   });
   ```
4. Pass `sprite_ref` to `spawn_actor_sprite` call

**Step 2.1.2 — Verify `spawn_actor_sprite` handling:**

- **File**: `antares/src/game/systems/actor.rs`
- **Symbol**: `spawn_actor_sprite`
- **Action**: READ ONLY — verify function accepts `SpriteReference` and correctly applies:
  - `sheet_path` (texture lookup)
  - `sprite_index` (atlas index)
  - `animation` field (if `Some`, apply animation component)
  - `material_properties` (if `Some`, apply material overrides)
- **Expected**: No changes needed (existing implementation handles all fields)

#### 2.2 Integrate Feature — Testing

**Step 2.2.1 — Add integration test for custom sprite:**

**Test: `test_spawn_map_prefers_resolved_npc_sprite_over_default`**

- **File**: `antares/src/game/systems/map.rs` (in `#[cfg(test)] mod tests`)
- **Purpose**: Verify `spawn_map` uses `resolved_npc.sprite` when present

**Setup:**

```rust
// Create NPC definition with custom sprite
let sprite = SpriteReference {
    sheet_path: "sprites/test/custom_npc.png".to_string(),
    sprite_index: 42,
    animation: None,
    material_properties: None,
};
let npc_def = NpcDefinition::new("Custom NPC").with_sprite(sprite.clone());

// Create placement and resolve
let placement = NpcPlacement::new(/* params */);
let resolved_npc = ResolvedNpc::from_placement_and_definition(&placement, &npc_def);

// Create minimal map with this NPC
let map = /* construct test map with resolved_npc */;
```

**Actions:**

```rust
// Spawn the map
spawn_map(&mut world, &map, &sprite_assets, &audio);

// Query spawned entity (implementation-specific entity lookup)
let spawned_entity = /* find entity spawned for this NPC */;
let actor_sprite_component = world.get::<ActorSprite>(spawned_entity).unwrap();
```

**Assertions:**

```rust
assert_eq!(actor_sprite_component.sheet_path, "sprites/test/custom_npc.png");
assert_eq!(actor_sprite_component.sprite_index, 42);
assert!(actor_sprite_component.animation.is_none());
```

**Expected**: Test passes, custom sprite used instead of `DEFAULT_NPC_SPRITE_PATH`

**Step 2.2.2 — Update existing default sprite test:**

**Test: `test_spawn_map_spawns_actor_sprite_for_npc` (existing test, if present)**

- **File**: `antares/src/game/systems/map.rs` tests
- **Action**: Modify to ensure backward compatibility verification

**Add/Update assertions:**

```rust
// When NPC has no sprite defined, should use DEFAULT_NPC_SPRITE_PATH
let npc_without_sprite = NpcDefinition::new("Generic NPC"); // no sprite set
// ... setup placement, map, spawn ...

assert_eq!(spawned_sprite.sheet_path, DEFAULT_NPC_SPRITE_PATH);
assert_eq!(spawned_sprite.sprite_index, 0);
```

**Expected**: Test passes, default placeholder used when `sprite = None`

**Step 2.2.3 — Modify asset manager scanning:**

- **File**: `sdk/campaign_builder/src/asset_manager.rs`
- **Symbol**: `scan_npcs_references` (function that scans NPC asset references)
- **Current behavior**: Scans `portrait_id` field for portrait images

**Actions:**

1. Locate loop iterating NPCs (likely iterates `npc_definitions`)
2. After portrait scanning logic, add sprite sheet scanning:
   ```rust
   // Scan sprite sheet references
   if let Some(ref sprite) = npc.sprite {
       // Mark sprite sheet as referenced
       let sprite_sheet_path = &sprite.sheet_path;
       referenced_assets.insert(sprite_sheet_path.clone());
       // Or use whatever collection/API the asset manager uses for tracking
   }
   ```

**Step 2.2.4 — Add asset manager test:**

**Test: `test_scan_npcs_detects_sprite_sheet_reference_in_metadata`**

- **File**: `sdk/campaign_builder/src/asset_manager.rs` (in `#[cfg(test)] mod tests`)
- **Purpose**: Verify asset scanner detects sprite sheets referenced in NPC definitions

**Setup:**

```rust
let sprite = SpriteReference {
    sheet_path: "assets/sprites/actors/test_npc.png".to_string(),
    sprite_index: 5,
    animation: None,
    material_properties: None,
};
let npc = NpcDefinition::new("Test NPC").with_sprite(sprite);
let npcs = vec![npc];
```

**Actions:**

```rust
let mut asset_manager = AssetManager::new();
asset_manager.scan_npcs_references(&npcs);
let referenced = asset_manager.get_referenced_assets(); // or equivalent API
```

**Assertions:**

```rust
assert!(referenced.contains("assets/sprites/actors/test_npc.png"));
```

**Expected**: Test passes, sprite sheet marked as referenced

#### 2.3 Configuration Updates

**Step 2.3.1 — Placeholder asset (conditional on Open Question 3 answer):**

**IF Open Question 3 = Option A (copy existing):**

- **Action**: Copy `assets/sprites/actors/npcs_town.png` to `assets/sprites/placeholders/npc_placeholder.png`
- **Command**: `cp assets/sprites/actors/npcs_town.png assets/sprites/placeholders/npc_placeholder.png`
- **OR** create symlink: `ln -s ../actors/npcs_town.png assets/sprites/placeholders/npc_placeholder.png`

**IF Open Question 3 = Option B (generate new):**

- **Action**: Run placeholder generation script
- **Command**: `python scripts/generate_placeholder_sprites.py --output assets/sprites/placeholders/npc_placeholder.png --size 32x48 --type npc`
- **Verify**: File exists at `assets/sprites/placeholders/npc_placeholder.png`

**Optional — Update `data/sprite_sheets.ron`:**

- IF placeholder sheet needs registration in sprite configuration:
  ```ron
  SpriteSheetConfig(
      path: "sprites/placeholders/npc_placeholder.png",
      tile_size: (32, 48),
      columns: 1,
      rows: 1,
  )
  ```
- Add to `data/sprite_sheets.ron` if required by `SpriteAssets::register_config`

#### 2.4 Testing Requirements

**Step 2.4.1 — Run quality gates:**

```bash
# Format code
cargo fmt --all

# Check compilation
cargo check --all-targets --all-features

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
cargo nextest run --all-features
```

**Expected output:**

```
cargo fmt --all
  → (no output = success)

cargo check --all-targets --all-features
  → Finished dev [unoptimized + debuginfo] target(s) in X.XXs

cargo clippy --all-targets --all-features -- -D warnings
  → Finished dev [unoptimized + debuginfo] target(s) in X.XXs
  → (0 warnings)

cargo nextest run --all-features
  → test result: ok. X passed; 0 failed; 0 ignored
```

**IF any command fails**: STOP, fix issues, re-run from `cargo fmt`.

#### 2.5 Deliverables (Phase 2)

- [ ] **2.5.1**: Modify `spawn_map` to prefer `resolved_npc.sprite` over `DEFAULT_NPC_SPRITE_PATH`
- [ ] **2.5.2**: Verify `spawn_actor_sprite` handles all `SpriteReference` fields (read-only verification)
- [ ] **2.5.3**: Add test `test_spawn_map_prefers_resolved_npc_sprite_over_default`
- [ ] **2.5.4**: Update test `test_spawn_map_spawns_actor_sprite_for_npc` for default fallback
- [ ] **2.5.5**: Modify `scan_npcs_references` to scan `npc.sprite.sheet_path`
- [ ] **2.5.6**: Add test `test_scan_npcs_detects_sprite_sheet_reference_in_metadata`
- [ ] **2.5.7**: (Optional) Add/copy placeholder asset per Open Question 3 answer
- [ ] **2.5.8**: Run `cargo fmt --all` (0 diffs)
- [ ] **2.5.9**: Run `cargo check --all-targets --all-features` (0 errors)
- [ ] **2.5.10**: Run `cargo clippy --all-targets --all-features -- -D warnings` (0 warnings)
- [ ] **2.5.11**: Run `cargo nextest run --all-features` (all tests pass)
- [ ] **2.5.12**: Verify Phase 2 Success Criteria (Section 2.6)

#### 2.6 Success Criteria (Phase 2)

**ALL must be TRUE:**

**Compilation & Quality Gates:**

- [ ] All 4 cargo commands pass (fmt, check, clippy, nextest run)
- [ ] No new compiler errors introduced
- [ ] No new clippy warnings introduced
- [ ] No test regressions (all existing tests still pass)

**Test Results:**

- [ ] `test_spawn_map_prefers_resolved_npc_sprite_over_default` — PASSED
- [ ] `test_spawn_map_spawns_actor_sprite_for_npc` — PASSED (default fallback works)
- [ ] `test_scan_npcs_detects_sprite_sheet_reference_in_metadata` — PASSED

**Runtime Behavior:**

- [ ] NPCs with `sprite` metadata spawn with custom sprite sheet/index
- [ ] NPCs without `sprite` metadata spawn with `DEFAULT_NPC_SPRITE_PATH` (backward compatible)
- [ ] Asset manager detects sprite sheet references in NPC definitions

**IF placeholder asset added:**

- [ ] File exists at `assets/sprites/placeholders/npc_placeholder.png`
- [ ] No missing texture warnings in logs when spawning default NPCs

---

### Phase 3: Editor Integration & Documentation (REQUIRED)

**Scope**: Add Campaign Builder UI for sprite selection, persist to RON, update documentation.

#### Phase 3 Execution Sequence

**Execute in this EXACT order:**

**Step 3.1.1**: Add sprite selection UI to NPC editor

**Step 3.1.2**: Add sprite persistence to save/load logic

**Step 3.1.3**: Add editor test for save/load round-trip

**Step 3.2.1**: Update `docs/explanation/implementations.md`

**Step 3.2.2**: (Optional) Update sprite support plan with NPC usage notes

**Step 3.3.1**: Run quality gates

**Step 3.4.1**: Verify Phase 3 success criteria

#### 3.1 Feature Work — Campaign Builder UI

**Step 3.1.1 — Add sprite selection UI:**

- **File**: `sdk/campaign_builder/src/npc_editor.rs`
- **Location**: NPC editor form/panel (where portrait, name, etc. are edited)

**Actions:**

1. Add UI section after portrait selector (or appropriate location):

   - **Label**: "Sprite Sheet" or "Visual Appearance"
   - **Dropdown/Picker**: List available sprite sheets (use existing `get_sprite_sheets()` helper or equivalent)
   - **Sprite Index Input**: Numeric input for sprite index (0-based, range 0-255 typical)
   - **(Optional, if Open Question 2 = A)**: Animation config UI (frames, FPS, loop toggle)

2. **IF Open Question 2 = B (index-only, RECOMMENDED):**

   - Only show sheet picker + index input
   - No animation UI (users can edit RON manually for animations)

3. Bind UI controls to `NpcDefinition.sprite` field:

   ```rust
   // When sheet/index selected:
   if let (Some(sheet_path), Some(index)) = (selected_sheet, sprite_index_input) {
       let sprite = SpriteReference {
           sheet_path: sheet_path.clone(),
           sprite_index: index,
           animation: None, // or from UI if Open Question 2 = A
           material_properties: None,
       };
       npc_definition.sprite = Some(sprite);
   } else {
       npc_definition.sprite = None; // Clear if deselected
   }
   ```

4. **(Optional)** Add sprite preview:
   - Use existing sprite preview component (if available)
   - Show selected sprite from sheet at given index

**Step 3.1.2 — Add persistence:**

- **File**: `sdk/campaign_builder/src/npc_editor.rs` (save/load functions)
- **Actions**:
  1. Verify save function serializes `NpcDefinition` to `npcs.ron` (should be automatic via serde)
  2. Verify load function deserializes `npcs.ron` and populates UI fields (including new `sprite` field)
  3. No code changes needed if using standard serde serialization (field is already `#[serde(default)]`)

**Step 3.1.3 — Add editor test:**

**Test: `test_npc_editor_roundtrip_preserves_sprite_metadata`**

- **File**: `sdk/campaign_builder/src/npc_editor.rs` (in `#[cfg(test)] mod tests`)
- **Purpose**: Verify save → load round-trip preserves sprite field

**Setup:**

```rust
let sprite = SpriteReference {
    sheet_path: "sprites/actors/wizard.png".to_string(),
    sprite_index: 12,
    animation: None,
    material_properties: None,
};
let npc = NpcDefinition::new("Wizard NPC").with_sprite(sprite.clone());
```

**Actions:**

```rust
// Save to temporary file
let temp_path = "/tmp/test_npcs.ron";
save_npc_definitions(&[npc.clone()], temp_path).unwrap();

// Load back
let loaded = load_npc_definitions(temp_path).unwrap();
let loaded_npc = &loaded[0];
```

**Assertions:**

```rust
assert!(loaded_npc.sprite.is_some());
assert_eq!(loaded_npc.sprite.as_ref().unwrap().sheet_path, "sprites/actors/wizard.png");
assert_eq!(loaded_npc.sprite.as_ref().unwrap().sprite_index, 12);
```

**Expected**: Test passes, sprite metadata preserved through save/load cycle

#### 3.2 Integrate Feature — Documentation

**Step 3.2.1 — Update implementations.md:**

- **File**: `docs/explanation/implementations.md`
- **Location**: Add new section under "Game Content Systems" heading (after "Sprite Asset Management" section if present, otherwise at end)
- **Section title**: `### NPC Sprite Metadata System`

**Required content structure:**

````markdown
### NPC Sprite Metadata System

**Purpose**: Enables per-NPC sprite customization via `SpriteReference` metadata in NPC definitions. NPCs can specify custom sprite sheets, indices, and animations instead of using the global `DEFAULT_NPC_SPRITE_PATH` placeholder.

**Affected Files**:

- `antares/src/domain/world/npc.rs` — Added `sprite: Option<SpriteReference>` field to `NpcDefinition`
- `antares/src/domain/world/types.rs` — Added `sprite` field to `ResolvedNpc`, propagated from definition
- `antares/src/game/systems/map.rs` — Modified `spawn_map` to prefer per-NPC sprite over default
- `sdk/campaign_builder/src/asset_manager.rs` — Extended `scan_npcs_references` to detect sprite sheet usage
- `sdk/campaign_builder/src/npc_editor.rs` — Added sprite selection UI to NPC editor

**Data Structure Changes**:

`NpcDefinition` (domain layer):

```rust
pub struct NpcDefinition {
    pub name: String,
    pub portrait_id: Option<u32>,
    #[serde(default)]
    pub sprite: Option<SpriteReference>, // NEW FIELD
    // ... other fields
}
```
````

`ResolvedNpc` (domain layer):

```rust
pub struct ResolvedNpc {
    pub sprite: Option<SpriteReference>, // NEW FIELD
    // ... other fields
}
```

**Usage Example** (RON format):

```ron
// npcs.ron
[
    NpcDefinition(
        name: "Town Guard",
        portrait_id: Some(5),
        sprite: Some(SpriteReference(
            sheet_path: "sprites/actors/npcs_town.png",
            sprite_index: 3,
            animation: None,
            material_properties: None,
        )),
    ),
    NpcDefinition(
        name: "Generic Villager",
        portrait_id: Some(10),
        // No sprite field — uses DEFAULT_NPC_SPRITE_PATH placeholder
    ),
]
```

**Behavior**:

- When `sprite` is `Some`: Runtime spawns NPC using specified sheet/index
- When `sprite` is `None`: Runtime falls back to `DEFAULT_NPC_SPRITE_PATH` (backward compatible)

**Testing**:

- `test_npc_definition_serializes_with_sprite_field_present` — Domain serialization
- `test_npc_definition_deserializes_without_sprite_field_defaults_none` — Backward compatibility
- `test_resolved_npc_from_placement_copies_sprite_field_when_present` — Sprite propagation
- `test_spawn_map_prefers_resolved_npc_sprite_over_default` — Runtime spawn behavior
- `test_scan_npcs_detects_sprite_sheet_reference_in_metadata` — Asset scanning
- `test_npc_editor_roundtrip_preserves_sprite_metadata` — Editor persistence

**Backward Compatibility**:

- Existing `npcs.ron` files without `sprite` field continue to work (serde default = `None`)
- Existing NPCs without sprite metadata spawn with placeholder as before
- No breaking changes to `NpcDefinition::new` constructor signature

````

**Step 3.2.2 — (Optional) Update sprite support plan:**

- **File**: `docs/explanation/sprite_support_implementation_plan.md` (if exists)
- **Action**: Add reference to NPC sprite usage in Phase 3 or Phase 5 notes
- **Content**: Brief mention that NPCs support `SpriteReference` metadata per this implementation

#### 3.3 Configuration Updates

**No additional configuration needed** — covered in Phase 2 (placeholder asset).

#### 3.4 Testing Requirements

**Step 3.3.1 — Run quality gates:**

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
````

**Expected**: All commands pass (see Phase 2.4.1 for expected output format).

**Step 3.4.1 — Verify Phase 3 success criteria (next section)**

#### 3.5 Deliverables (Phase 3)

- [ ] **3.5.1**: Add sprite sheet picker UI to `npc_editor.rs`
- [ ] **3.5.2**: Add sprite index input to `npc_editor.rs`
- [ ] **3.5.3**: (Optional, if Open Question 2 = A) Add animation config UI
- [ ] **3.5.4**: Bind UI controls to `NpcDefinition.sprite` field
- [ ] **3.5.5**: Verify save/load functions handle `sprite` field (automatic via serde)
- [ ] **3.5.6**: Add test `test_npc_editor_roundtrip_preserves_sprite_metadata`
- [ ] **3.5.7**: Update `docs/explanation/implementations.md` (add NPC Sprite Metadata section)
- [ ] **3.5.8**: (Optional) Update `docs/explanation/sprite_support_implementation_plan.md`
- [ ] **3.5.9**: Run `cargo fmt --all` (0 diffs)
- [ ] **3.5.10**: Run `cargo check --all-targets --all-features` (0 errors)
- [ ] **3.5.11**: Run `cargo clippy --all-targets --all-features -- -D warnings` (0 warnings)
- [ ] **3.5.12**: Run `cargo nextest run --all-features` (all tests pass)
- [ ] **3.5.13**: Verify Phase 3 Success Criteria (Section 3.6)

#### 3.6 Success Criteria (Phase 3)

**ALL must be TRUE:**

**Compilation & Quality Gates:**

- [ ] All 4 cargo commands pass (fmt, check, clippy, nextest run)
- [ ] No compiler errors
- [ ] No clippy warnings
- [ ] All tests passing (including new editor test)

**Test Results:**

- [ ] `test_npc_editor_roundtrip_preserves_sprite_metadata` — PASSED

**Editor Functionality:**

- [ ] NPC editor UI displays sprite sheet picker
- [ ] NPC editor UI displays sprite index input
- [ ] Selecting sheet + index updates `NpcDefinition.sprite` field
- [ ] Saving NPC definition persists `sprite` to `npcs.ron`
- [ ] Loading NPC definition populates UI with saved `sprite` values

**Documentation:**

- [ ] `docs/explanation/implementations.md` updated with NPC Sprite Metadata section
- [ ] Documentation includes purpose, affected files, data structure changes, usage example, testing, backward compatibility

**Post-Implementation Verification (not automatable, optional):**

- Campaign Builder opens without errors
- NPC editor allows sprite selection and saves correctly
- Loading a campaign with NPC sprite metadata displays correct sprites in-game

---

### Phase 3.7: Architecture Compliance Verification (MANDATORY)

**Execute this checklist BEFORE claiming Phase 3 complete. This is a BLOCKING step.**

**Purpose**: Verify NO architectural drift introduced during implementation.

**MANDATORY Checklist (all must pass):**

**Data Structure Compliance:**

- [ ] `NpcDefinition` structure matches `docs/reference/architecture.md` Section 4 exactly (no unauthorized fields added beyond `sprite`)
- [ ] `ResolvedNpc` structure matches architecture.md Section 4 exactly
- [ ] `SpriteReference` structure unchanged from architecture.md definition
- [ ] No modifications to core domain types beyond documented changes

**Module Structure Compliance:**

- [ ] No new modules created outside `docs/reference/architecture.md` Section 3.2 structure
- [ ] `npc.rs` remains in `antares/src/domain/world/` (domain layer)
- [ ] `types.rs` remains in `antares/src/domain/world/` (domain layer)
- [ ] `map.rs` remains in `antares/src/game/systems/` (application layer)
- [ ] `npc_editor.rs` remains in `sdk/campaign_builder/src/` (SDK layer)

**Type System Compliance:**

- [ ] All type aliases used correctly (`ItemId`, `SpellId`, etc. where applicable)
- [ ] No raw `u32` or `usize` types used where type aliases exist
- [ ] `sprite_index` uses `u32` (no type alias defined for sprite indices, per architecture)
- [ ] No introduction of `String` where path type aliases should be used (if any)

**Constants & Magic Numbers:**

- [ ] `DEFAULT_NPC_SPRITE_PATH` constant used (not hardcoded string)
- [ ] No magic numbers introduced (sprite indices use variables/params, not hardcoded values)
- [ ] All constants defined at module level or appropriate scope

**Data Format Compliance:**

- [ ] RON format used for `npcs.ron` (NOT JSON/YAML)
- [ ] `#[serde(default)]` used for all optional fields
- [ ] No breaking serialization format changes

**Layer Boundary Compliance:**

- [ ] Domain layer (`npc.rs`, `types.rs`) has NO dependencies on infrastructure layer
- [ ] Domain layer has NO dependencies on application layer
- [ ] Application layer (`map.rs`, `actor.rs`) can depend on domain layer (allowed)
- [ ] SDK layer can depend on domain layer (allowed)

**Dependency Compliance:**

- [ ] `cargo check` passes (verifies no circular dependencies)
- [ ] No new crate dependencies added without justification
- [ ] Existing dependencies unchanged

**AttributePair Pattern (if applicable):**

- [ ] No stats modified in this feature (N/A for NPC sprite metadata)
- [ ] IF stats were modified, `AttributePair` pattern used correctly

**IF ANY item fails:**

1. Document the deviation in `docs/explanation/architectural_decisions.md`
2. Get user approval before proceeding
3. Re-run this checklist after approval

**IF ALL items pass**: Proceed to Phase 4 (optional) or declare implementation complete.

---

### Phase 4: Optional Enhancements (POST-MVP, NOT REQUIRED FOR COMPLETION)

**IMPORTANT**: This phase is **NOT required for MVP**. Implement only if explicitly requested by user.

**Possible improvements after core rollout (Phases 1-3):**

#### 4.1 Per-Placement Sprite Overrides

**IF Open Question 1 = A (include overrides):**

- Add `sprite_override: Option<SpriteReference>` to `NpcPlacement` struct
- Modify `ResolvedNpc::from_placement_and_definition` to prefer placement override over definition sprite
- Precedence: `placement.sprite_override` → `definition.sprite` → `DEFAULT_NPC_SPRITE_PATH`
- Update tests to verify override behavior
- Add editor UI for per-placement sprite customization

**Estimated effort**: +2-4 hours

#### 4.2 Animated Placeholder

- Create multi-frame placeholder sprite sheet (e.g., 4-frame idle animation)
- Add default `SpriteAnimation` config (frames: 4, fps: 8, loop: true)
- Update `DEFAULT_NPC_SPRITE_PATH` logic to include default animation
- Benefits: NPCs without sprite metadata have basic animation instead of static sprite

**Estimated effort**: +1-2 hours

#### 4.3 Performance Benchmarking

- Add benchmark: `benchmark_billboard_system_100_npcs_custom_sprites`
- Compare performance: all default sprites vs. all custom sprites vs. mixed
- Identify any performance regressions from per-NPC sprite lookups
- Optimize if needed (e.g., sprite reference caching)

**Estimated effort**: +2-3 hours

#### 4.4 Advanced Editor Features

- Batch sprite assignment UI (select multiple NPCs, assign same sprite)
- Sprite preview grid (visual picker instead of index input)
- Animation preview in editor (show animated sprite when animation configured)
- Sprite search/filter (find sprites by name/tag)

**Estimated effort**: +6-12 hours depending on scope

**To enable Phase 4**: User must explicitly request specific enhancements after Phase 3 completion.

---

## Files / Symbols to Modify (Complete Reference Table)

| File                                                     | Symbols                                      | Action                                                                                              | Line Location (approx)         | Tests to Add/Modify                                                                                                                      |
| -------------------------------------------------------- | -------------------------------------------- | --------------------------------------------------------------------------------------------------- | ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- | ----------------- | -------------------------------------------------------------------------------------------------------------- |
| `antares/src/domain/world/npc.rs`                        | `NpcDefinition` struct                       | Add `#[serde(default)] pub sprite: Option<SpriteReference>` field                                   | After `portrait_id` (~line 45) | `test_npc_definition_serializes_with_sprite_field_present`, `test_npc_definition_deserializes_without_sprite_field_defaults_none`        |
| `antares/src/domain/world/npc.rs`                        | `NpcDefinition` impl                         | Add `pub fn with_sprite(mut self, sprite: SpriteReference) -> Self` builder                         | After `new` method             | (covered by serialization tests)                                                                                                         |
| `antares/src/domain/world/types.rs`                      | `ResolvedNpc` struct                         | Add `pub sprite: Option<SpriteReference>` field                                                     | Struct definition              | `test_resolved_npc_from_placement_copies_sprite_field_when_present`, `test_resolved_npc_from_placement_sprite_none_when_definition_none` |
| `antares/src/domain/world/types.rs`                      | `ResolvedNpc::from_placement_and_definition` | Add `sprite: definition.sprite.clone(),` to struct construction                                     | Function body                  | (covered by above tests)                                                                                                                 |
| `antares/src/game/systems/map.rs`                        | `spawn_map` function                         | Prefer `resolved_npc.sprite.clone().unwrap_or_else(                                                 |                                | default)`                                                                                                                                | NPC spawning loop | `test_spawn_map_prefers_resolved_npc_sprite_over_default`, update `test_spawn_map_spawns_actor_sprite_for_npc` |
| `antares/src/game/systems/actor.rs`                      | `spawn_actor_sprite`                         | (Read-only verification, no changes)                                                                | N/A                            | (existing tests)                                                                                                                         |
| `sdk/campaign_builder/src/asset_manager.rs`              | `scan_npcs_references`                       | Add `if let Some(ref sprite) = npc.sprite { referenced_assets.insert(sprite.sheet_path.clone()); }` | NPC iteration loop             | `test_scan_npcs_detects_sprite_sheet_reference_in_metadata`                                                                              |
| `sdk/campaign_builder/src/npc_editor.rs`                 | NPC editor UI                                | Add sprite sheet picker + index input + binding to `sprite` field                                   | Editor form/panel              | `test_npc_editor_roundtrip_preserves_sprite_metadata`                                                                                    |
| `docs/explanation/implementations.md`                    | (documentation)                              | Add "NPC Sprite Metadata System" section under "Game Content Systems"                               | After sprite section           | N/A                                                                                                                                      |
| `docs/explanation/sprite_support_implementation_plan.md` | (documentation, optional)                    | Add reference to NPC sprite usage                                                                   | Phase 3/5 notes                | N/A                                                                                                                                      |

---

## Validation Criteria (Fully Automatable)

**ALL criteria must pass before declaring implementation complete.**

### Unit Tests (must all PASS)

**Domain Layer:**

- [ ] `test_npc_definition_serializes_with_sprite_field_present` — PASSED
- [ ] `test_npc_definition_deserializes_without_sprite_field_defaults_none` — PASSED
- [ ] `test_resolved_npc_from_placement_copies_sprite_field_when_present` — PASSED
- [ ] `test_resolved_npc_from_placement_sprite_none_when_definition_none` — PASSED

**Application Layer:**

- [ ] `test_spawn_map_prefers_resolved_npc_sprite_over_default` — PASSED
- [ ] `test_spawn_map_spawns_actor_sprite_for_npc` — PASSED (updated for default fallback)

**SDK Layer:**

- [ ] `test_scan_npcs_detects_sprite_sheet_reference_in_metadata` — PASSED
- [ ] `test_npc_editor_roundtrip_preserves_sprite_metadata` — PASSED

### Quality Gates (all commands must exit with 0)

**Formatting:**

```bash
cargo fmt --all
```

**Expected**: No output (no diffs = success)

**Compilation:**

```bash
cargo check --all-targets --all-features
```

**Expected**:

```
Finished dev [unoptimized + debuginfo] target(s) in X.XXs
```

**(0 errors)**

**Linting:**

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

**Expected**:

```
Finished dev [unoptimized + debuginfo] target(s) in X.XXs
```

**(0 warnings)**

**Testing:**

```bash
cargo nextest run --all-features
```

**Expected**:

```
Running tests...
test result: ok. X passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in X.XXs
```

**(All tests pass, including new tests listed above)**

### Backward Compatibility (must all be TRUE)

- [ ] RON files without `sprite` field deserialize successfully (verified by `test_npc_definition_deserializes_without_sprite_field_defaults_none`)
- [ ] Deserialized NPCs without sprite have `sprite = None` (serde default behavior)
- [ ] Runtime behavior falls back to `DEFAULT_NPC_SPRITE_PATH` when `sprite = None` (verified by updated spawn test)
- [ ] No breaking changes to `NpcDefinition::new` signature (existing code compiles without modifications)

### Architecture Compliance (from Phase 3.7 checklist)

- [ ] Data structures match architecture.md Section 4 EXACTLY (no unauthorized changes)
- [ ] Module placement follows Section 3.2 structure (no new modules outside defined structure)
- [ ] Type aliases used correctly (no raw types where aliases defined)
- [ ] Constants used instead of magic numbers (`DEFAULT_NPC_SPRITE_PATH`)
- [ ] RON format used for game data (`npcs.ron`)
- [ ] No architectural drift (all deviations documented and approved)
- [ ] No circular dependencies (`cargo check` passes)
- [ ] Layer boundaries respected (domain has no infrastructure dependencies)

### Documentation Compliance

- [ ] Markdown filenames use `lowercase_with_underscores.md` (not CamelCase, not kebab-case)
- [ ] Updated file is `docs/explanation/implementations.md` (correct Diataxis category)
- [ ] No emojis in documentation
- [ ] All code blocks in docs specify language (e.g., `rust, `ron, ```bash)
- [ ] Documentation includes: purpose, affected files, data structure changes, usage example, testing, backward compatibility

### Code Quality (additional checks)

- [ ] No `unwrap()` calls without justification comments
- [ ] All public items have `///` doc comments with examples
- [ ] SPDX headers added to new `.rs` files (N/A for this implementation — only modifying existing files)
- [ ] No `TODO` or `FIXME` comments left unresolved
- [ ] All warnings addressed (clippy 0 warnings enforced)

**IF ANY validation criterion fails**: STOP, fix the issue, re-run quality gates, then retry checklist.

**WHEN ALL criteria pass**: Implementation is complete and ready for user review.

---

## Risk Assessment & Mitigations

### Risk 1: Breaking Deserialization for Old Campaigns

**Risk Level**: HIGH (would break existing campaigns)

**Mitigation**:

- Use `#[serde(default)]` attribute on `sprite` field
- Add explicit backward compatibility test (`test_npc_definition_deserializes_without_sprite_field_defaults_none`)
- Verify old RON files deserialize with `sprite = None`
- No changes to required fields (all changes are additive)

**Verification**: Run backward-compat test against real campaign `npcs.ron` file (if available)

### Risk 2: Editor Complexity Scope Creep

**Risk Level**: MEDIUM (could delay Phase 3)

**Mitigation**:

- Implement index-only selection in V1 (defer full animation UI to Phase 4)
- Limit initial UI to: sheet picker + index input only
- Advanced features (animation config, batch assignment) deferred to optional Phase 4
- Use existing UI components where possible (sprite sheet picker already exists for other editors)

**Verification**: Strictly follow Phase 3 scope (do not add features beyond index-only selection unless Open Question 2 = A)

### Risk 3: Missing Assets at Runtime Cause Warnings

**Risk Level**: LOW (cosmetic issue, not functional break)

**Mitigation**:

- Address placeholder asset in Phase 2 (Open Question 3 resolution)
- Option A: Copy existing `npcs_town.png` to placeholder location (immediate fix)
- Option B: Generate new placeholder via script (cleaner but requires script execution)
- Add asset existence validation in asset manager (detect missing sprite sheets at campaign load)

**Verification**: Run game after Phase 2 completion, check logs for missing texture warnings

### Risk 4: Performance Regression from Per-NPC Sprite Lookups

**Risk Level**: LOW (unlikely with small NPC counts, <100 typical)

**Mitigation**:

- Existing `spawn_actor_sprite` already performs texture lookups (no new overhead)
- `SpriteReference.clone()` is cheap (small struct, mostly string clone)
- Defer performance optimization to Phase 4 (benchmark first, optimize if needed)

**Verification**: Manual testing with campaign containing 50+ NPCs (no noticeable lag expected)

---

## Work Estimate (Revised)

**Time estimates per phase (for experienced Rust developer):**

| Phase                  | Description                          | Estimated Time  | Includes                                                      |
| ---------------------- | ------------------------------------ | --------------- | ------------------------------------------------------------- |
| **Phase 1**            | Core domain implementation           | **2-3 hours**   | Add fields, tests, verify compilation                         |
| **Phase 2**            | Runtime integration & asset scanning | **3-5 hours**   | Spawn logic, asset manager, integration tests                 |
| **Phase 3**            | Editor UI & documentation            | **4-8 hours**   | UI components, save/load, docs (varies by UI complexity)      |
| **Phase 3.7**          | Architecture compliance verification | **0.5-1 hour**  | Checklist review, verification                                |
| **Total (Phases 1-3)** | **Required for MVP**                 | **10-17 hours** | End-to-end implementation                                     |
| **Phase 4**            | Optional enhancements                | **2-12 hours**  | Per-placement overrides, animation, benchmarks (if requested) |

**Variability factors:**

- Editor UI complexity (index-only vs. full animation config)
- Existing test infrastructure (may reduce test writing time)
- Familiarity with codebase (first-time contributors may take longer)

**Recommendation**: Budget 2 days (16 hours) for full Phases 1-3 implementation including testing and documentation.

---

## Next Steps (For User)

**BEFORE IMPLEMENTATION CAN BEGIN:**

1. **Answer Open Questions** (required for plan finalization):

   - **Question 1**: Per-placement overrides — Option A (include now) or Option B (defer to Phase 4)?
   - **Question 2**: Editor animation config — Option A (full config) or Option B (index-only)?
   - **Question 3**: Placeholder asset — Option A (copy existing) or Option B (generate new)?

2. **Approve This Plan** (or request revisions):

   - Review all phases, deliverables, validation criteria
   - Confirm scope is appropriate for goals
   - Request changes if needed

3. **After Approval**:
   - Implementation can begin following the phased execution sequence
   - Each phase must complete ALL deliverables and pass ALL success criteria before proceeding to next phase
   - Quality gates enforced at end of each phase (no exceptions)

**Once you answer Open Questions and approve this plan:**

- Implementation can proceed immediately
- Follow phases in order: 1 → 2 → 3 → 3.7 (compliance) → (optional) 4
- Refer to this document for exact steps, test names, file locations, and validation criteria

---

**Plan Status**: ✅ READY FOR REVIEW (awaiting user answers to Open Questions and approval)

**Last Updated**: 2025 (comprehensive revision per review feedback)
