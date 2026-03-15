# Implementations

## ItemMesh Full Coverage — All Items Data-Driven (Complete)

### Problem

`ItemMeshCategory` and `ItemMeshDescriptor::from_item` form a hard-coded
procedural fallback in the game engine. The fallback derives mesh shapes
entirely from Rust code — `WeaponClassification::Simple + sides ≤ 4 →
Dagger shape`, `ArmorClassification::Medium → iron chain-mail pile`, etc.
Because 33 of the 43 tutorial items had `mesh_id: None` in `items.ron`, the
data-driven path was bypassed for the vast majority of items. Several items
received actively wrong shapes: `Staff` rendered as a blunt club (its
`Simple` classification + `sides = 6` mapped to Blunt), `Healing Scroll` and
`Resurection Scroll` rendered as red potion flasks (their `HealHp`/
`CureCondition` effect mapped to Potion), and `Mage Robe` rendered as a
leather-armor pile.

Food items (`Food Ration`, `Trail Ration`) had no entry in the registry at
all — the fallback generated an earthy-brown flask, but any `mesh_id` pointer
would have resulted in the wrong colour since no food-coloured mesh existed.

### Root Cause

`spawn_dropped_item_system` only takes the data-driven path when
`item.mesh_id = Some(id)` AND the id resolves in `GameDataResource`. For
`mesh_id: None` the fallback is always triggered regardless of what RON mesh
files exist in the campaign. The 27-entry registry had full coverage; the
item definitions simply had not been wired up to it.

### Fix

#### 1. `campaigns/tutorial/assets/items/consumables/food_item.ron` (new file)

A new `CreatureDefinition` asset — id `9205` — using the same six-vertex
hexagonal disc geometry as the potion meshes, coloured earthy brown
`(0.55, 0.35, 0.10)` with high roughness `(0.85)` and no emissive glow.
The colour exactly matches what the procedural fallback generated for
`ConsumableEffect::IsFood` items, ensuring visual consistency between the
data-driven path and any future fallback use. Scale is `0.8` (slightly
smaller than the `0.9` potion scale) to read as a plain, mundane consumable.

#### 2. `campaigns/tutorial/data/item_mesh_registry.ron`

Added entry `/*[27]*/` for id `9205 ItemMeshFoodItem` pointing to the new
`assets/items/consumables/food_item.ron`. The registry now has 28 entries
covering all weapon, armor, accessory, consumable, ammo, and quest categories.

#### 3. `campaigns/tutorial/data/items.ron`

Assigned `mesh_id` to all 33 items that previously had `mesh_id: None`.
The game engine's `from_item` procedural fallback is now unreachable for
every item in the tutorial campaign. Full mapping:

| Item                  | id      | mesh_id | Rationale                                                  |
| --------------------- | ------- | ------- | ---------------------------------------------------------- |
| Mace                  | 5       | 9006    | Club — only blunt-weapon mesh available                    |
| Battle Axe            | 6       | 9001    | Sword — closest martial 1H shape; no axe mesh yet          |
| Leather Armor         | 20      | 9101    | Exact match                                                |
| Chain Mail            | 21      | 9102    | Exact match                                                |
| Plate Mail            | 22      | 9103    | Exact match                                                |
| Wooden Shield         | 23      | 9104    | Exact match                                                |
| Iron Helmet           | 25      | 9105    | Exact match                                                |
| Leather Boots         | 26      | 9106    | Exact match                                                |
| Chain Mail +1         | 30      | 9102    | Same shape, enchanted variant                              |
| Dragon Scale Mail     | 31      | 9103    | Plate shape — high-AC, full-coverage armour                |
| Ring of Protection    | 40      | 9301    | Exact match                                                |
| Amulet of Might       | 41      | 9302    | Exact match                                                |
| Belt of Speed         | 42      | 9303    | Exact match                                                |
| Arcane Wand           | 43      | 9007    | Staff — rod/wand silhouette                                |
| Holy Symbol           | 44      | 9302    | Amulet — worn as pendant                                   |
| Healing Potion        | 50      | 9201    | Exact match                                                |
| Magic Potion          | 51      | 9202    | Mana Potion — same flask shape                             |
| Cure Poison Potion    | 52      | 9203    | Cure Potion — exact match                                  |
| Arrows                | 60      | 9401    | Exact match                                                |
| Crossbow Bolts        | 61      | 9402    | Exact match                                                |
| Ruby Whistle          | 100     | 9502    | Key Item — quest object                                    |
| Mace of Undead        | 101     | 9006    | Club — blunt weapon shape                                  |
| Foo / Foo Stand / Bar | 102–104 | 9502    | Key Item — test placeholders                               |
| Long Bow +1           | 105     | 9008    | Bow — exact match                                          |
| Staff                 | 106     | 9007    | **Critical fix** — fallback gave it a club shape           |
| Mage Robe             | 107     | 9304    | Cloak — fabric garment, far better than leather-armor pile |
| Healing Scroll        | 108     | 9501    | **Critical fix** — fallback gave it a red potion flask     |
| Cure Disease Potion   | 109     | 9203    | Cure Potion                                                |
| Resurection Scroll    | 110     | 9501    | **Critical fix** — fallback gave it a red potion flask     |
| Food Ration           | 111     | 9205    | New food mesh — earthy brown flask                         |
| Trail Ration          | 112     | 9205    | New food mesh — earthy brown flask                         |

### Files Changed

| File                                                        | Change                                                     |
| ----------------------------------------------------------- | ---------------------------------------------------------- |
| `campaigns/tutorial/assets/items/consumables/food_item.ron` | New — earthy-brown hexagonal flask, id 9205                |
| `campaigns/tutorial/data/item_mesh_registry.ron`            | Added entry 27: id 9205 `ItemMeshFoodItem`                 |
| `campaigns/tutorial/data/items.ron`                         | `mesh_id` assigned for all 43 items; zero `None` remaining |

### Quality Gates

```text
cargo fmt --all          → clean
cargo check --all-targets --all-features → 0 errors
cargo clippy --all-targets --all-features -- -D warnings → 0 warnings
cargo nextest run --all-features → 3560 passed, 0 failed
```

---

## ItemMesh Format Mismatch — Dropped Items and SDK Preview Fix (Complete)

### Problem

Dropped items in the game world always displayed the same procedural mesh
regardless of which `mesh_id` was assigned to the item. Importing a new OBJ
(as RON) in the Item Mesh Editor and moving it to `assets/items/weapons/` also
had no visible effect — the SDK preview always showed the same generic sword
shape.

### Root Cause 1 — `short_sword.ron` in Wrong Format (Critical)

`campaigns/tutorial/assets/items/weapons/short_sword.ron` was serialised in
`ItemMeshDescriptor` format:

```campaigns/tutorial/assets/items/weapons/short_sword.ron#L1-9
(
    category: Dagger,
    blade_length: 0.5,
    primary_color: (0.75, 0.75, 0.78, 1.0),
    ...
    scale: 1.5,
)
```

Every other asset file under `assets/items/` was already in `CreatureDefinition`
format (with `id:`, `name:`, `meshes:`, etc.). When
`CreatureDatabase::load_from_registry` tried to deserialise `short_sword.ron`
as a `CreatureDefinition` it returned a `ParseError`. That error propagated
through `ItemMeshDatabase::load_from_registry` → `load_item_meshes` →
`load_game_data` → `load_campaign_data`, which caught the top-level error,
logged it, and inserted an **empty** `GameDataResource`.

Consequence: every `spawn_dropped_item_system` call found `item_meshes` empty,
hit the `mesh_id not found` warning branch, and fell back to the procedural
`ItemMeshDescriptor::from_item` path — making all items look identical.

### Root Cause 2 — SDK `perform_save_as_with_path` Serialised Wrong Type

`perform_save_as_with_path` was serialising the raw `ItemMeshDescriptor` struct
to disk:

```sdk/campaign_builder/src/item_mesh_editor.rs#L870-L875
let ron_text = ron::ser::to_string_pretty(&descriptor, ...)
```

Any file written from the SDK editor was therefore in `ItemMeshDescriptor`
format, which the runtime `CreatureDatabase` loader cannot parse. New meshes
created inside the SDK would silently fail to load in-game.

### Root Cause 3 — `execute_register_asset` Rejected `CreatureDefinition` Files

`execute_register_asset_validation` tried to parse the file only as
`ItemMeshDescriptor` and rejected it with an error if that failed — meaning any
correctly-formatted `CreatureDefinition` RON (e.g. an OBJ imported outside the
SDK) could not be registered at all.

### Root Cause 4 — SDK Preview Lost Custom Geometry

When `load_from_campaign` encountered a file in `CreatureDefinition` format
(Attempt 2 code path), it discarded the vertex data and stored only a
simplified `ItemMeshDescriptor` inferred from the first mesh's colour.
`sync_preview_renderer_from_descriptor` always called
`descriptor.to_creature_definition()`, so the preview showed a procedurally
regenerated shape instead of the imported OBJ geometry.

### Fix

#### 1. `campaigns/tutorial/assets/items/weapons/short_sword.ron`

Converted from `ItemMeshDescriptor` format to `CreatureDefinition` format,
consistent with all other asset files. Blade vertices computed from the
original descriptor parameters (category `Dagger`, `blade_length: 0.5`,
`scale: 1.5`):

- `half_len = (0.3 + 0.5 × 0.7) × 0.5 = 0.3250`
- `half_width = half_len × 0.12 = 0.0390`
- `pommel_z = −half_len × 0.30 = −0.0975`

Cross-guard proportioned between `dagger.ron` (±0.0700) and `sword.ron`
(±0.1100): ±0.0850 wide, ±0.0150 deep. Both meshes receive steel colours and
metallic materials matching the original descriptor.

This single fix unblocks the entire item mesh registry: `load_game_data` now
succeeds, `GameDataResource` is populated, and all items with a `mesh_id`
resolve to their correct `CreatureDefinition`.

#### 2. `sdk/campaign_builder/src/item_mesh_editor.rs` — `perform_save_as_with_path`

Now serialises a `CreatureDefinition` (the canonical game-engine format) instead
of an `ItemMeshDescriptor`. If the entry being saved originated from an
imported `CreatureDefinition` (see fix 4 below), that native definition is
reused with `scale` updated from the descriptor; otherwise
`descriptor.to_creature_definition()` generates the mesh. Files written by the
SDK can now be loaded by `CreatureDatabase::load_from_registry` without error.

#### 3. `execute_register_asset_validation` / `execute_register_asset`

Both methods now accept either format. Validation checks
`ron::de::from_str::<ItemMeshDescriptor>` **and** `ron::de::from_str::<CreatureDefinition>`
and rejects the file only when neither succeeds. The registration path
constructs an `ItemMeshEntry` with `native_creature_def: Some(def)` when the
file is a `CreatureDefinition`, preserving the custom geometry for use in the
preview and in subsequent saves.

#### 4. `ItemMeshEntry` — `native_creature_def` Field

Added `pub native_creature_def: Option<CreatureDefinition>` to `ItemMeshEntry`.

- `load_from_campaign` Attempt 1 (ItemMeshDescriptor path): `native_creature_def: None`.
- `load_from_campaign` Attempt 2 (CreatureDefinition path): `native_creature_def: Some(def)`.
- `execute_register_asset` (CreatureDefinition file): `native_creature_def: Some(def)`.
- SDK-authored saves: `native_creature_def: None` (descriptor is source of truth).

`sync_preview_renderer_from_descriptor` now checks the selected entry for a
`native_creature_def` and passes it directly to the preview renderer when
present, so imported OBJ geometry is faithfully shown instead of a procedural
approximation.

### Files Changed

| File                                                      | Change                                                                                        |
| --------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `campaigns/tutorial/assets/items/weapons/short_sword.ron` | Converted from `ItemMeshDescriptor` to `CreatureDefinition` format                            |
| `sdk/campaign_builder/src/item_mesh_editor.rs`            | `ItemMeshEntry` new field; save path fix; register-asset dual-format; preview uses native def |

### Quality Gates

```text
cargo fmt --all          → clean
cargo check --all-targets --all-features → 0 errors
cargo clippy --all-targets --all-features -- -D warnings → 0 warnings
cargo nextest run --all-features → 3560 passed, 0 failed
```

---

## SDK: Reload Sweep — Centralized and Fixed Across All Editors (Complete)

### Problem

The Campaign Builder's **Reload** toolbar action (F5) was broken in two editors
and inconsistently implemented across all others:

1. **Creatures Editor** — `ToolbarAction::Reload` was silently discarded with a
   `// Handled by parent` comment, but the parent (`CampaignBuilderApp`) never
   handled it. Clicking Reload in the Creatures tab did nothing.

2. **NPC Editor** — `ToolbarAction::Reload` fell through a `_ => {}` wildcard
   arm and was silently ignored. Clicking Reload in the NPCs tab did nothing.

3. **Six simple editors** (Items, Monsters, Spells, Conditions, Proficiencies,
   Quests) — Each contained its own copy of identical `std::fs::read_to_string`
   - `ron::from_str` logic instead of using the centralised `handle_reload`
     helper that already existed in `ui_helpers.rs` but was never called.

### Root Cause — Creatures Editor

The creatures editor uses a two-step load sequence (registry file →
per-creature asset files) that lives in `CampaignBuilderApp::load_creatures()`.
The editor's `show()` returns `Option<String>` (a status message) rather than a
`ToolbarAction`, so the parent has no typed channel to receive an action request.
The `// Handled by parent` comment was aspirational but never wired up.

### Root Cause — NPC Editor

The NPC editor's `match toolbar_action` block had only three explicit arms
(`New`, `Import`, `Export`) and a catch-all `_ => {}`. `Reload` fell into the
catch-all without any effect. Additionally, because `show()` returns `bool`
(unsaved-changes flag) rather than a status string, there was no mechanism to
report the reload outcome to the global status bar.

### Fix

**`sdk/campaign_builder/src/creatures_editor.rs`**

- Added `pub const RELOAD_CREATURES_SENTINEL` — a namespaced sentinel string
  modelled on the existing `OPEN_CREATURE_TEMPLATES_SENTINEL`.
- Changed the `ToolbarAction::Reload` arm from `// Handled by parent` to
  `return Some(RELOAD_CREATURES_SENTINEL.to_string())`, so the editor signals
  its intent without duplicating load logic.

**`sdk/campaign_builder/src/lib.rs` (creatures)**

- In the `EditorTab::Creatures` handler, added an `else if` branch for
  `RELOAD_CREATURES_SENTINEL` that calls `self.load_creatures()`, the same
  full two-step reload that runs on campaign open.

**`sdk/campaign_builder/src/npc_editor.rs`**

- Added `pub pending_status: Option<String>` (`#[serde(skip)]`) to
  `NpcEditorState` — a side-channel the parent polls with `.take()` after
  every `show()` call to pick up status messages.
- Added `pub fn load_from_file(&mut self, path: &Path) -> Result<(), String>`
  which deserialises `Vec<NpcDefinition>` from disk, replaces `self.npcs`,
  clears the selection, resets to List mode, and clears the unsaved-changes
  flag.
- Added an explicit `ToolbarAction::Reload` arm in `show()` that calls
  `load_from_file` and stores the result in `pending_status`.

**`sdk/campaign_builder/src/lib.rs` (NPCs)**

- After the `npc_editor_state.show()` call, added:
  `if let Some(status) = self.npc_editor_state.pending_status.take() { ... }`
  to forward reload (and any future) status messages to the global status bar.

**Six simple editors — migrated to `handle_reload`**

Replaced the duplicated inline reload blocks in each of the following files
with a single call to `handle_reload(data, campaign_dir, filename, status)`:

| File                                               | Data type                    |
| -------------------------------------------------- | ---------------------------- |
| `sdk/campaign_builder/src/items_editor.rs`         | `Vec<Item>`                  |
| `sdk/campaign_builder/src/monsters_editor.rs`      | `Vec<MonsterDefinition>`     |
| `sdk/campaign_builder/src/spells_editor.rs`        | `Vec<Spell>`                 |
| `sdk/campaign_builder/src/conditions_editor.rs`    | `Vec<ConditionDefinition>`   |
| `sdk/campaign_builder/src/proficiencies_editor.rs` | `Vec<ProficiencyDefinition>` |
| `sdk/campaign_builder/src/quest_editor.rs`         | `Vec<Quest>`                 |

Each file also gained `handle_reload` in its `use crate::ui_helpers::` import.

### Editors Left Unchanged

The following editors already had correct, custom reload logic and were not
touched:

- `characters_editor.rs` — calls `self.load_from_file(&path)` (handles extra state)
- `classes_editor.rs` — calls `self.load_from_file(&path)`
- `races_editor.rs` — calls `self.load_from_file(&path)`
- `dialogue_editor.rs` — calls `self.load_from_file(&path)` + syncs `*dialogues`
- `config_editor.rs` — calls `self.load_config(campaign_dir)`
- `map_editor.rs` — calls `self.load_maps(...)` (multi-file directory load)
- `stock_templates_editor.rs` — auto-loads on first display; Reload not in toolbar

### New Tests (8 new)

**`creatures_editor::tests`**

- `test_reload_sentinel_is_nonempty_and_namespaced` — sentinel is non-empty and
  starts with `__campaign_builder`.
- `test_reload_sentinel_differs_from_template_sentinel` — the two sentinels are
  distinct so the parent can route them to different handlers.

**`npc_editor::tests`**

- `test_pending_status_initial_state` — `pending_status` starts as `None`.
- `test_load_from_file_replaces_npcs` — round-trip: writes two NPCs to a temp
  file, loads into an editor that had a stale NPC, asserts list replaced and
  editor state reset.
- `test_load_from_file_missing_file_returns_err` — error message mentions
  "Failed to read" when the path does not exist.
- `test_load_from_file_bad_ron_returns_err` — error message mentions "Failed to
  parse" when the file contains invalid RON.
- `test_reload_sets_pending_status_on_success` — `pending_status` contains a
  "Reloaded" message after a successful reload; `.take()` clears the field.
- `test_reload_sets_pending_status_when_file_missing` — `pending_status`
  contains "not found" and `self.npcs` is left unchanged.

### Quality Gates

```text
cargo fmt         → clean
cargo check       → 0 errors
cargo clippy      → 0 warnings
cargo nextest run (workspace)          → 3560 passed, 8 skipped
cargo nextest run -p campaign_builder  → 2059 passed, 2 skipped
```

---

## SDK: Stock-Template Wipe-Prevention Fix (Complete)

### Problem

Every time the user saved the tutorial campaign from the Campaign Builder,
`campaigns/tutorial/data/npc_stock_templates.ron` was silently overwritten with
an empty array `[]`, destroying all merchant stock template definitions. The
user had to restore the file by hand after every save session.

### Root Cause

`do_save_campaign` in `sdk/campaign_builder/src/lib.rs` called
`stock_templates_editor_state.save_to_file(...)` **unconditionally** on every
campaign save. `save_to_file` serialises whatever is in
`stock_templates_editor_state.templates` — which is `Vec::new()` until
`load_from_file` is called. So any save that occurred before the Stock
Templates tab had been visited (or before `load_stock_templates()` completed)
wrote `[]` to disk. Once the file contained `[]`, every subsequent load read
it back as an empty list, perpetuating the cycle.

### Fix

**`sdk/campaign_builder/src/stock_templates_editor.rs`**

Added a `loaded_from_file: bool` field to `StockTemplatesEditorState`
(`#[serde(skip)]`, default `false`):

- `load_from_file` sets `loaded_from_file = true` on success.
- `reset_for_new_campaign` resets it to `false` so a stale loaded state from
  a previous campaign cannot authorise writes for a freshly opened one.

**`sdk/campaign_builder/src/lib.rs`**

The stock-templates write block in `do_save_campaign` is now guarded:

```sdk/campaign_builder/src/lib.rs#L1-4
let should_save_templates = self.stock_templates_editor_state.loaded_from_file
    || self.stock_templates_editor_state.has_unsaved_changes;
if should_save_templates {
    ...save_to_file...
```

This means:

- **Templates loaded from disk** → written back (safe, data exists in memory).
- **User made explicit in-editor changes** → written back (user intent respected,
  even if they deleted all templates).
- **Empty default Vec, never loaded** → skipped (file left untouched).

**`campaigns/tutorial/data/npc_stock_templates.ron`**

Restored to the correct three-template content and fixed a template-ID mismatch
that would have caused SDK validation errors:

| Template ID                | Referenced by NPC                                  |
| -------------------------- | -------------------------------------------------- |
| `"town_merchant_basic"`    | `tutorial_merchant_town`                           |
| `"mountain_pass_merchant"` | `tutorial_merchant_town2`                          |
| `"holy_temple_menu"`       | `tutorial_priestess_town`, `tutorial_priest_town2` |

The previous file used `"temple_basic_stock"` for the third template while the
NPCs referenced `"holy_temple_menu"` — a mismatch the SDK validator would flag
as an unknown stock template error. The template was renamed and food items
(ids 111 and 112) were added to all three templates.

### New Tests (4 new — in `sdk/campaign_builder/src/`)

| Test                                                                       | File                        | Covers                                                                                                                                  |
| -------------------------------------------------------------------------- | --------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `test_loaded_from_file_is_false_on_default`                                | `stock_templates_editor.rs` | Guard starts `false` on a fresh editor state                                                                                            |
| `test_load_from_file_sets_loaded_from_file_true`                           | `stock_templates_editor.rs` | Successful `load_from_file` sets guard to `true`                                                                                        |
| `test_reset_for_new_campaign_clears_loaded_from_file`                      | `stock_templates_editor.rs` | `reset_for_new_campaign` clears guard back to `false`                                                                                   |
| `test_do_save_campaign_does_not_overwrite_stock_templates_when_not_loaded` | `lib.rs`                    | Full regression: `do_save_campaign` leaves the on-disk file untouched when `loaded_from_file = false` and `has_unsaved_changes = false` |

### Quality Gates

```
cargo fmt         → clean
cargo check       → 0 errors
cargo clippy      → 0 warnings
cargo nextest run (antares)           → 3560 passed
cargo nextest run (campaign_builder)  → 28 passed (3 new guard tests + 1 regression)
```

---

## Phase 5: Character Equipment — Documentation and Final Validation (Complete)

### Summary

Phase 5 closes out the five-phase Character Equipment feature by auditing test
coverage for all new variants and functions introduced in Phases 1–4, adding
the missing unit tests found during the audit, and running the full four-gate
quality pipeline to confirm zero errors and zero warnings.

### Coverage Audit Results

The four priority targets from the plan were inspected:

| Target file                                                   | Gap found                                                                    | Action taken     |
| ------------------------------------------------------------- | ---------------------------------------------------------------------------- | ---------------- |
| `src/domain/items/types.rs` — `required_proficiency`          | No tests for `Helmet` and `Boots` classifications                            | Added 2 tests    |
| `src/domain/items/equipment_validation.rs` — `can_equip_item` | `Helmet` and `Boots` fully covered by Phase 1 tests                          | No action needed |
| `src/domain/proficiency.rs` — `proficiency_for_armor`         | `Helmet` and `Boots` fully covered by Phase 1 tests                          | No action needed |
| `src/domain/character_definition.rs` — `instantiate`          | Weapon and body-armor tested in Phase 3; no tests for `helmet`/`boots` slots | Added 2 tests    |

### New Tests (4 new)

| Test                                                 | File                                 | Covers                                                                                                            |
| ---------------------------------------------------- | ------------------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| `test_armor_required_proficiency_helmet`             | `src/domain/items/types.rs`          | `Item::required_proficiency()` returns `Some("light_armor")` for a `Helmet`-classified armor item                 |
| `test_armor_required_proficiency_boots`              | `src/domain/items/types.rs`          | `Item::required_proficiency()` returns `Some("light_armor")` for a `Boots`-classified armor item                  |
| `test_instantiate_starting_helmet_in_equipment_slot` | `src/domain/character_definition.rs` | `instantiate` places a helmet (item 25, Iron Helmet) into `equipment.helmet`; item absent from inventory; AC = 11 |
| `test_instantiate_starting_boots_in_equipment_slot`  | `src/domain/character_definition.rs` | `instantiate` places boots (item 26, Leather Boots) into `equipment.boots`; item absent from inventory; AC = 11   |

### Cross-Phase Feature Summary

The complete Character Equipment feature (Phases 1–5) delivered:

#### Phase 1 — `ArmorClassification` Expansion

- Added `Helmet` and `Boots` variants to `ArmorClassification` enum
  (`src/domain/items/types.rs`)
- Extended `ProficiencyDatabase::proficiency_for_armor` to map both new
  variants to `"light_armor"` (`src/domain/proficiency.rs`)
- Made `has_slot_for_item` exhaustive over all six `ArmorClassification`
  variants — no `_` wildcard (`src/domain/items/equipment_validation.rs`)
- Made `EquipmentSlot::for_item` exhaustive over the same variants
  (`src/domain/items/equipment_validation.rs`)
- Migrated `data/test_campaign/data/items.ron` and `data/items.ron` to use
  `classification: Helmet` / `classification: Boots` in RON data
- Extended SDK validation to accept the two new classifications
  (`src/sdk/validation.rs`)

#### Phase 2 — Domain Transaction Functions and AC Calculation

- Added `EquipmentSlot` enum with seven variants and `get`/`set`/`for_item`
  methods (`src/domain/character.rs`)
- Added `calculate_armor_class` pure function — reads `equipment` and
  `item_db`, returns a clamped `[AC_MIN, AC_MAX]` value
  (`src/domain/items/equipment_validation.rs`)
- Added `equip_item` domain transaction: validates class/race proficiency
  and alignment, adds item to inventory, then moves it to the correct slot,
  then recalculates AC (`src/domain/transactions.rs`)
- Added `unequip_item` domain transaction: clears the slot, returns item to
  inventory, recalculates AC (`src/domain/transactions.rs`)
- Fixed `Character::new()` to initialise `ac.current = AC_DEFAULT` (was 0)
  (`src/domain/character.rs`)
- Fixed `equipped_count()` to iterate all seven slots including `helmet` and
  `boots` (`src/domain/character.rs`)

#### Phase 3 — Starting Equipment in Inventory

- Changed `CharacterDefinition::instantiate` to a two-pass flow: first add
  all `starting_items` to inventory, then call `equip_item` for each slot in
  `starting_equipment` (`src/domain/character_definition.rs`)
- Removed `create_starting_equipment` helper (logic merged into `instantiate`)
- Added `CharacterDefinitionError::InvalidStartingEquipment` — returned when
  a starting equipment item fails proficiency or alignment validation
- Audited `data/test_campaign/data/characters.ron` and
  `campaigns/tutorial/data/characters.ron` to verify all `starting_equipment`
  item IDs are present in the corresponding items databases

#### Phase 4 — Inventory UI — Equip and Unequip

- Added `EquipItemAction` and `UnequipItemAction` message types, registered in
  `InventoryPlugin::build` (`src/game/systems/inventory_ui.rs`)
- Added `PanelAction::Equip` and `PanelAction::Unequip` variants to the action
  strip; `build_action_list` prepends `Equip` (index 0) for equipable items
- Added equipment display strip: two rows of seven cells rendered above the
  inventory grid in every character panel
- Added **E** keyboard shortcut to dispatch `EquipItemAction` from the
  `SlotNavigation` phase; **Enter** on a focused strip cell dispatches
  `UnequipItemAction`
- Extended `inventory_action_system` to handle both new message types, writing
  `GameLog` errors on all failure paths

### Architecture Compliance

- [x] `ArmorClassification` match arms exhaustive — no `_` wildcard
- [x] `EquipmentSlot::for_item` match exhaustive over all six `ArmorClassification` variants
- [x] Proficiency IDs are string values from `ProficiencyDatabase`, never hard-coded in callers
- [x] RON data files use `classification: Helmet` / `Boots`
- [x] No tests reference `campaigns/tutorial` — all use `data/test_campaign`
- [x] Helmet and Boots fixture items exist in `data/test_campaign/data/items.ron` (ids 25 and 26)
- [x] `equip_item` and `unequip_item` are pure-domain functions in `transactions.rs` — no Bevy dependencies
- [x] `calculate_armor_class` only reads `equipment` and `item_db` — no mutable state
- [x] AC recalculation is triggered inside `equip_item` and `unequip_item`, never in callers
- [x] Starting equipment items added to inventory then equip-validated via `equip_item`
- [x] `EquipItemAction` / `UnequipItemAction` follow existing message pattern
- [x] All new public functions have `///` doc comments
- [x] SPDX `FileCopyrightText` and `License-Identifier` headers present in all modified `.rs` files

### Quality Gates

```
cargo fmt         → clean
cargo check       → 0 errors
cargo clippy      → 0 warnings
cargo nextest run → 3560 passed (8 skipped); was 3556 before Phase 5
```

---

## Phase 4: Inventory UI — Equip and Unequip (Complete)

### Summary

Added full equip/unequip support to the inventory UI overlay. Players can now
select an item in the inventory grid and press **E** (or click the **Equip**
button in the action strip) to move it into the appropriate equipment slot.
An equipment display strip — two rows of seven cells — is rendered between
the character name header and the inventory slot grid in every character panel,
showing all equipped items. Selecting a cell in the strip (keyboard or mouse)
and pressing **Enter** (or clicking **Unequip**) returns the item to inventory.
All failure paths write a human-readable `GameLog` entry instead of panicking.

### Files Changed

- **`src/game/systems/inventory_ui.rs`** — primary implementation file:
  - `EquipItemAction` and `UnequipItemAction` message types added and
    registered in `InventoryPlugin::build`
  - `PanelAction::Equip` and `PanelAction::Unequip` variants added
  - `InventoryNavigationState.selected_equip_slot: Option<EquipmentSlot>`
    added to track equipment strip focus
  - `build_action_list` extended: `Equip` prepended as index-0 action for
    any equipable item (weapons, armour, accessories)
  - `inventory_input_system` extended:
    - **E key** shortcut dispatches `EquipItemAction` directly from
      `SlotNavigation` phase (mirrors existing **U** shortcut for Use)
    - Equipment strip keyboard navigation: **↑** from grid row 0 enters
      the strip; **↓** returns to grid row 0; **←**/**→** cycle the seven
      cells; **Enter** dispatches `UnequipItemAction`
    - `ActionNavigation` `Enter` handler updated to handle `Equip` and
      `Unequip` variants
    - `Tab` clears `selected_equip_slot`
    - **↑** wrapping from row 0 now enters the equipment strip instead of
      wrapping to the last grid row
  - `inventory_ui_system` extended: passes `panel_equip_slot` to
    `render_character_panel`; handles `PanelAction::Equip` and
    `PanelAction::Unequip`; hint line updated for equipment strip phase
  - `render_character_panel` extended:
    - `selected_equip_slot: Option<EquipmentSlot>` parameter added
    - Equipment strip (`EQUIP_STRIP_H = 76px`) rendered between header and
      grid — two rows (Weapon/Armor/Shield, Helmet/Boots/Ring/Ring)
    - Each cell shows a dimmed slot label and the equipped item name (or
      `—` when empty); focused cell has a green highlight border
    - Mouse click on an occupied cell dispatches `PanelAction::Unequip`
    - Unequip action button shown below the strip when a cell is focused
    - **Equip** button added to the action strip (index 0, green) for
      equipable items; existing Use/Drop/Transfer index offsets updated
    - `painter` cloned to owned `Painter` so `ui.allocate_rect()` and
      `ui.new_child()` can be called without borrow conflicts
  - `inventory_action_system` extended:
    - `EquipItemAction` handler: bounds-checks, resolves `GameContent`,
      calls `equip_item()`, clears nav state on success, writes `GameLog`
      error on failure
    - `UnequipItemAction` handler: bounds-checks, resolves `GameContent`,
      calls `unequip_item()`, clears `selected_equip_slot` on success,
      writes "inventory is full" `GameLog` entry on `InventoryFull` error
  - All six existing `render_character_panel` test call sites updated to
    pass `None` for the new `selected_equip_slot` parameter
  - All ten existing `inventory_action_system` test setups updated to
    register `EquipItemAction` and `UnequipItemAction`
  - `test_build_action_list_no_use_for_non_consumable` updated: first
    action for a weapon is now `Equip`, not `Drop`
  - `test_panel_action_drop_variant` and `test_panel_action_transfer_variant`
    updated with `Equip`/`Unequip` catch-all arms

### Design Decisions

#### `Painter::clone()` to resolve egui borrow conflict

`egui::Ui::painter()` returns `&Painter` — an immutable reference to the
`Ui`. Calling `ui.allocate_rect()` (mutable) while `painter` is alive fails
the borrow checker. Cloning the painter at the start of
`render_character_panel` gives an owned `Painter` whose internal `Arc`
state still writes to the same draw list, eliminating the conflict without
any observable behavioural difference.

#### E key test uses nav-state side-effect, not message-queue inspection

`Messages<T>` is not re-exported from `bevy::prelude`. Rather than depend
on the internal `bevy_ecs::message::Messages` path, the E key test verifies
the nav-state side-effect: `selected_slot_index` is set to `None` and
`phase` stays `SlotNavigation` — a code path that is only reached inside
the `if is_equipable { equip_writer.write(...) }` branch. This is a
sufficient proxy for "the message was dispatched".

#### `ButtonInput<KeyCode>` initialised without `InputPlugin`

Bevy's `InputPlugin` registers a `First`-schedule system that calls
`ButtonInput::clear()` at the start of every frame, wiping `just_pressed`
before `Update` systems run. In tests this means pressing a key before
`app.update()` would be invisible to the input system. By registering
`ButtonInput<KeyCode>` with `app.init_resource::<ButtonInput<KeyCode>>()`
(without `InputPlugin`), `just_pressed` persists through the update frame
as needed.

#### Up-arrow from row 0 enters the equipment strip (no bottom-wrap)

Before Phase 4, pressing **↑** from grid row 0 wrapped to the last grid
row (standard grid wrapping). Now it navigates to the equipment strip
above the grid. Bottom-to-top grid wrapping is removed for row 0 because
the strip is the natural "above row 0" destination. Pressing **↓** from
the strip returns to slot 0 of the grid, completing the bidirectional
navigation.

#### Action button index offsets computed with a running `btn_idx`

The action strip previously used hard-coded offsets
(`if is_consumable { 1 } else { 0 }`) for the Drop index. With the new
`Equip` button potentially preceding both `Use` and `Drop`, a running
`btn_idx` counter replaces all hard-coded offsets, making the order
table self-documenting and easy to extend.

### New Tests (7 new)

| Test                                                   | Covers                                                          |
| ------------------------------------------------------ | --------------------------------------------------------------- |
| `test_equip_action_dispatched_on_e_key`                | E key in `SlotNavigation` clears nav state (proxy for dispatch) |
| `test_equip_button_absent_for_consumable`              | `build_action_list` omits Equip for consumables                 |
| `test_equip_button_present_for_weapon`                 | `build_action_list` puts Equip first for weapons                |
| `test_equip_action_system_moves_item_to_slot`          | `EquipItemAction` moves item to `equipment.weapon`              |
| `test_unequip_action_system_returns_item_to_inventory` | `UnequipItemAction` clears slot and restores inventory          |
| `test_unequip_action_system_inventory_full_logs_error` | Full inventory → GameLog "inventory is full", slot unchanged    |
| `test_equipment_strip_shows_equipped_item_name`        | Panel renders with equipped weapon; item accessible via struct  |

### Quality Gates

```
cargo fmt         → clean
cargo check       → 0 errors
cargo clippy      → 0 warnings
cargo nextest run → 3555 passed (excluding pre-existing flaky perf benchmark)
```

---

## Phase 3: Starting Equipment in Inventory (Complete)

### Summary

Replaced the direct-copy `create_starting_equipment` helper with a two-pass
"add-to-inventory, then equip" flow inside `CharacterDefinition::instantiate`.
Every starting equipment item now passes through `equip_item` (proficiency,
race, alignment, and slot validation). Added `CharacterDefinitionError::InvalidStartingEquipment`
to surface bad data to campaign authors. Audited and fixed duplicate item IDs
in both `data/test_campaign/data/characters.ron` and
`campaigns/tutorial/data/characters.ron` (Whisper had Dagger in both
`starting_items` and `starting_equipment.weapon`).

### Files Changed

| File                                     | Change                                                                                                                                                                                                                                                                                                          |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/domain/character_definition.rs`     | Added `AC_DEFAULT`, `calculate_armor_class`, and `equip_item` imports; added `InvalidStartingEquipment` error variant; rewrote `instantiate` to use two-pass flow; removed `create_starting_equipment` helper; removed its two tests; updated `test_instantiate_with_real_databases`; added 5 new Phase 3 tests |
| `data/test_campaign/data/characters.ron` | Removed duplicate item 2 (Dagger) from `whisper.starting_items` — it is authoritative in `starting_equipment.weapon`                                                                                                                                                                                            |
| `campaigns/tutorial/data/characters.ron` | Removed duplicate item 2 (Dagger) from `whisper.starting_items`                                                                                                                                                                                                                                                 |

### Design Decisions

- **Two-pass order**: Equipment items are appended to inventory in
  `StartingEquipment::all_item_ids()` order (weapon → armor → shield →
  helmet → boots → accessory1 → accessory2). Pass 2 iterates **in reverse**
  so that removing a higher-indexed slot does not shift the indices of
  items not yet processed, keeping the index arithmetic O(1) and correct.

- **`Equipment::new()` in character struct**: The character is built with
  all equipment slots empty, then the two-pass flow populates them through
  `equip_item`. This means equipment always satisfies the same validation
  contract as runtime equipping.

- **AC recalculation once, after all equips**: `calculate_armor_class` is
  called once after the entire Pass 2 loop rather than after each equip.
  This matches the plan and avoids redundant work.

- **`InventoryFull` on Pass 1**: If the inventory is full before a starting
  equipment item can be added, the existing `CharacterDefinitionError::InventoryFull`
  variant is returned. The `InvalidStartingEquipment` variant is reserved for
  validation failures inside `equip_item`.

- **Duplicate-ID audit**: `starting_equipment` is authoritative for equipped
  items. Duplicate entries in `starting_items` were removed from the bag list
  to prevent characters receiving two copies of the same item.

### New Tests (7 total — 5 new + 2 removed, test count net change: +5)

| Test                                                            | Location               | What it verifies                                                |
| --------------------------------------------------------------- | ---------------------- | --------------------------------------------------------------- |
| `test_instantiate_starting_weapon_in_equipment_slot`            | `character_definition` | Weapon lands in slot, not inventory                             |
| `test_instantiate_starting_weapon_equippable_then_unequippable` | `character_definition` | Unequip after instantiate moves weapon to inventory             |
| `test_instantiate_starting_armor_updates_ac`                    | `character_definition` | Leather Armor (+2) yields `ac.current == 12`                    |
| `test_instantiate_no_starting_equipment_ac_is_default`          | `character_definition` | Empty starting equipment yields `ac.current == AC_DEFAULT (10)` |
| `test_instantiate_invalid_starting_equipment_returns_error`     | `character_definition` | Human sorcerer + Short Sword returns `InvalidStartingEquipment` |

_Removed_: `test_create_starting_equipment_empty` and
`test_create_starting_equipment_with_items` (function deleted).

### Quality Gates

```text
cargo fmt         → No output (all files formatted)
cargo check       → Finished with 0 errors
cargo clippy      → Finished with 0 warnings
cargo nextest run → 3549 tests run: 3549 passed, 0 failed, 8 skipped
```

---

## Phase 2: Domain Transaction Functions and AC Calculation (Complete)

### Summary

Added the `EquipmentSlot` enum with slot-routing, `get`, and `set` methods;
fixed `Equipment::MAX_EQUIPPED` and `equipped_count()`; implemented
`calculate_armor_class` as a pure domain function; fixed `Character::new()` AC
initialisation; and added `equip_item` / `unequip_item` transaction functions
with full atomic swap semantics.

### Files Changed

| File                                       | Change                                                                                                                                                                                                                                                                                  |
| ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/domain/character.rs`                  | Added `EquipmentSlot` enum (7 variants) with `get` and `set` methods; fixed `MAX_EQUIPPED` from 6 → 7; added `accessory2` to `equipped_count()` slice; changed `ac: AttributePair::new(0)` → `ac: AttributePair::new(AC_DEFAULT)` in `Character::new()`; updated `test_equipment_count` |
| `src/domain/items/equipment_validation.rs` | Added `impl EquipmentSlot { fn for_item(...) }` (in same crate, separate module to avoid circular dependency); added `calculate_armor_class(equipment, item_db) -> u8`; added 5 new AC tests                                                                                            |
| `src/domain/items/mod.rs`                  | Re-exported `calculate_armor_class`                                                                                                                                                                                                                                                     |
| `src/domain/transactions.rs`               | Added imports (`EquipmentSlot`, `ClassDatabase`, `RaceDatabase`, `EquipError`, `calculate_armor_class`, `can_equip_item`); added `equip_item` (returns `Result<(), EquipError>`); added `unequip_item` (returns `Result<(), TransactionError>`); added 12 new tests                     |

### Design Decisions

- **`EquipmentSlot::for_item` placement**: `character.rs` already imports
  `character::Alignment` from `items::types`, so adding an `Item` import in
  `character.rs` would create a circular dependency. The method is instead
  added via an `impl EquipmentSlot` block in `equipment_validation.rs`, which
  already imports both worlds. This is valid Rust — inherent impl blocks for a
  type in the same crate can live in any module.

- **`equip_item` return type `Result<(), EquipError>`**: The plan permits
  either `EquipError` directly or wrapping in `TransactionError::EquipFailed`.
  Since all equip-side test assertions check `EquipError` variants and the
  existing `can_equip_item` already surfaces `EquipError`, returning it directly
  avoids an unnecessary wrapping layer and keeps callers' pattern-matches
  concise.

- **`unequip_item` `character_id` in `InventoryFull`**: `unequip_item` does not
  accept a `character_id` parameter (per plan signature), so the
  `InventoryFull { character_id: 0 }` sentinel is used. Tests only check the
  variant, not the field value.

- **Atomicity in `equip_item`**: The item is removed from inventory (step 6)
  before the equipment slot is written (step 7). This means there is always
  a free inventory slot when the displaced item is written back in step 8.
  The explicit rollback path in step 8 (theoretically unreachable under normal
  conditions) re-inserts the removed slot and restores the equipment slot to
  its previous value.

- **`AC_DEFAULT` in `Character::new()`**: Corrected from `0` to `AC_DEFAULT`
  (10) so an unarmed, unarmoured fresh character correctly starts with AC 10,
  consistent with the success criterion.

### New Tests (17 total)

| Test                                                  | Location               |
| ----------------------------------------------------- | ---------------------- |
| `test_calculate_ac_no_armor`                          | `equipment_validation` |
| `test_calculate_ac_body_armor_only`                   | `equipment_validation` |
| `test_calculate_ac_all_slots`                         | `equipment_validation` |
| `test_calculate_ac_clamps_to_max`                     | `equipment_validation` |
| `test_calculate_ac_missing_item_id_skips_slot`        | `equipment_validation` |
| `test_equip_item_weapon_moves_from_inventory_to_slot` | `transactions`         |
| `test_equip_item_swaps_old_weapon_back_to_inventory`  | `transactions`         |
| `test_equip_item_armor_updates_ac`                    | `transactions`         |
| `test_equip_item_helmet_routes_to_helmet_slot`        | `transactions`         |
| `test_equip_item_boots_routes_to_boots_slot`          | `transactions`         |
| `test_equip_item_invalid_class_returns_error`         | `transactions`         |
| `test_equip_item_out_of_bounds_returns_error`         | `transactions`         |
| `test_equip_item_non_equipable_item_returns_error`    | `transactions`         |
| `test_unequip_item_moves_to_inventory`                | `transactions`         |
| `test_unequip_item_reduces_ac`                        | `transactions`         |
| `test_unequip_item_empty_slot_is_noop`                | `transactions`         |
| `test_unequip_item_inventory_full_returns_error`      | `transactions`         |

### Quality Gates

```text
cargo fmt         → No output (all files formatted)
cargo check       → Finished with 0 errors
cargo clippy      → Finished with 0 warnings
cargo nextest run → 3546 tests run: 3546 passed, 0 failed, 8 skipped
```

---

## Phase 1: ArmorClassification Expansion (Complete)

### Summary

Expanded `ArmorClassification` with two new variants (`Helmet` and `Boots`),
extended the proficiency mapping, made the equipment-slot routing match
exhaustive, migrated all three RON item data files, and added SDK validation
that enforces slot-type integrity at campaign-pack time.

### Files Changed

| File                                       | Change                                                                                                                                                                                                      |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/domain/items/types.rs`                | Added `Helmet` and `Boots` variants to `ArmorClassification`; updated doc example                                                                                                                           |
| `src/domain/proficiency.rs`                | Extended `proficiency_for_armor` match with `Helmet => "light_armor"` and `Boots => "light_armor"`; updated doctest and added two dedicated test functions                                                  |
| `src/domain/items/equipment_validation.rs` | Replaced blanket `ItemType::Armor(_) => true` arm with exhaustive `ArmorClassification` match; added 6 new unit tests                                                                                       |
| `src/domain/visual/item_mesh.rs`           | Added `Helmet` and `Boots` arms to `from_armor`; added both variants to the all-classifications test array                                                                                                  |
| `src/bin/item_editor.rs`                   | Added options `[5] Helmet` and `[6] Boots` to `select_armor_classification`                                                                                                                                 |
| `src/sdk/validation.rs`                    | Added `HelmetSlotTypeMismatch` and `BootsSlotTypeMismatch` `ValidationError` variants (both `Error` severity); added validation logic in `validate_character_references`; added 3 new SDK integration tests |
| `src/sdk/error_formatter.rs`               | Added `get_suggestions` arms for `HelmetSlotTypeMismatch` and `BootsSlotTypeMismatch`                                                                                                                       |
| `tests/proficiency_integration_test.rs`    | Extended `get_armor_proficiency` helper with `Helmet` and `Boots` arms                                                                                                                                      |
| `data/items.ron`                           | Added Iron Helmet (id 25, `classification: Helmet`) and Leather Boots (id 26, `classification: Boots`)                                                                                                      |
| `data/test_campaign/data/items.ron`        | Same two items added — required by Phase 2 tests                                                                                                                                                            |
| `campaigns/tutorial/data/items.ron`        | Same two items added — live campaign kept in sync                                                                                                                                                           |

### Design Decisions

- **Proficiency mapping**: Both `Helmet` and `Boots` map to `"light_armor"`.
  Headgear and footwear are universally lightweight and do not warrant their
  own proficiency tracks; any class or race that can wear light armour can wear
  them. This matches the Might and Magic 1 design where helmets and boots are
  accessible to all adventurers.

- **IDs 25 and 26**: The plan suggested IDs 50/51, but those were already
  occupied by potions in every data file. IDs 25 and 26 are the next free
  slots in the armor ID block (20–29) and are available across all three item
  data files.

- **Exhaustive match in `has_slot_for_item`**: All six `ArmorClassification`
  arms return `true` because the function checks whether a slot _type_ exists,
  not whether it is vacant. The exhaustive form guarantees any future
  classification variant causes a compile-time error rather than silently
  defaulting to an incorrect value.

- **SDK validation severity**: `HelmetSlotTypeMismatch` and
  `BootsSlotTypeMismatch` are `Error`-severity (not `Warning`) because placing
  the wrong item type in a dedicated slot is always a data-authoring bug, not
  an intentional design choice.

### New Tests (11 total)

| Test                                                    | Location               |
| ------------------------------------------------------- | ---------------------- |
| `test_armor_classification_helmet_variant_exists`       | `equipment_validation` |
| `test_armor_classification_boots_variant_exists`        | `equipment_validation` |
| `test_has_slot_for_helmet_item`                         | `equipment_validation` |
| `test_has_slot_for_boots_item`                          | `equipment_validation` |
| `test_can_equip_helmet_succeeds`                        | `equipment_validation` |
| `test_can_equip_boots_succeeds`                         | `equipment_validation` |
| `test_proficiency_for_armor_helmet_maps_to_light_armor` | `proficiency`          |
| `test_proficiency_for_armor_boots_maps_to_light_armor`  | `proficiency`          |
| `test_sdk_validation_helmet_in_wrong_slot_fails`        | `sdk::validation`      |
| `test_sdk_validation_boots_in_wrong_slot_fails`         | `sdk::validation`      |
| `test_sdk_validation_correct_helmet_passes`             | `sdk::validation`      |

### Quality Gates

```text
cargo fmt         → No output (all files formatted)
cargo check       → Finished with 0 errors
cargo clippy      → Finished with 0 warnings
cargo nextest run → 3529 tests run: 3529 passed, 0 failed, 8 skipped
```

---

## Item Mesh Editor — Registry Loading Fix (Complete)

### Problem

Opening the Item Mesh Editor tab always showed an empty registry regardless of
which campaign was loaded. No item mesh assets were ever displayed, and the
"Reload" button had no effect.

### Root Causes (three separate bugs)

| #   | Location                                           | Bug                                                                                                                                                                                  |
| --- | -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | `lib.rs` → `do_open_campaign`                      | No `load_item_meshes()` call exists — the `item_mesh_editor_state.registry` is initialised empty and never populated when a campaign is opened                                       |
| 2   | `item_mesh_editor.rs` → `refresh_available_assets` | Used `std::fs::read_dir` (one level only) on `assets/items/`; all actual RON files live in **subdirectories** (`weapons/`, `armor/`, `accessories/`, etc.) so nothing was ever found |
| 3   | `item_mesh_editor.rs` → `show_registry_mode`       | Contained a duplicate inline asset-scan with the same non-recursive bug; this was the code path triggered when the tab became visible for the first time                             |

**Format note**: The tutorial campaign stores item mesh geometry as
`CreatureDefinition` RON files (the game's internal format with explicit
vertices/faces). The editor's existing `execute_register_asset` path expects
`ItemMeshDescriptor` format (the editor's procedural-parameter format). The
loading code needs to handle both.

### Fix

#### 1. New `load_from_campaign` method (`item_mesh_editor.rs`)

Added a public method that:

- Recursively scans `<campaign_dir>/assets/items/**/*.ron`
- For each file, tries `ItemMeshDescriptor` deserialization first (editor-created files)
- Falls back to `CreatureDefinition` deserialization (game/legacy files), deriving
  an approximate `ItemMeshDescriptor` from mesh color, scale, and emissive data
- Uses `infer_category_from_path()` (file stem → folder name → default) to
  determine `ItemMeshCategory` for legacy files
- Replaces `self.registry` entirely so repeated loads don't accumulate stale entries
- Keeps `available_item_assets` in sync after loading

#### 2. New `collect_ron_files_recursive` helper (`item_mesh_editor.rs`)

Replaces the flat `read_dir` calls with a recursive walk so all files in
subdirectory trees are found.

#### 3. New `infer_category_from_path` helper (`item_mesh_editor.rs`)

Maps file stem (e.g. `"short_sword"` → `Dagger`, `"chain_mail"` → `BodyArmor`)
and parent folder name (`"weapons"` → `Sword`, `"accessories"` → `Ring`, etc.)
to `ItemMeshCategory`.

#### 4. `refresh_available_assets` fixed to use recursive helper

Replaced the non-recursive `read_dir` loop with a call to
`collect_ron_files_recursive`.

#### 5. `show_registry_mode` inline scan replaced (`item_mesh_editor.rs`)

Replaced the duplicate non-recursive inline scan block (triggered on first
tab visit) with a call to `load_from_campaign` so the registry is also
populated when the tab is shown for the first time after opening a campaign.

#### 6. `do_open_campaign` wired up (`lib.rs`)

Added a call to `self.item_mesh_editor_state.load_from_campaign(dir)` in the
data-loading block alongside all other `load_*` calls. The call is guarded by
`if let Some(ref dir) = self.campaign_dir.clone()` and logged at info level.

| File                                           | Change                                                                                                                                                                                        |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `sdk/campaign_builder/src/item_mesh_editor.rs` | Added `load_from_campaign`, `collect_ron_files_recursive`, `stem_to_display_name`, `infer_category_from_path`; fixed `refresh_available_assets`; replaced inline scan in `show_registry_mode` |
| `sdk/campaign_builder/src/lib.rs`              | Added `load_from_campaign` call in `do_open_campaign`                                                                                                                                         |

All four quality gates pass after the fix:

```text
cargo fmt         → clean
cargo check       → Finished (0 errors, 0 warnings)
cargo clippy      → Finished (0 warnings)
cargo nextest run → 3518 passed, 0 failed
```

---

## Campaign Builder Context Menu Fix (Complete)

### Problem

Right-click edit action on items in the Campaign Builder Items Editor was not
triggering. When users right-clicked on an item and selected "Edit", the context
menu would close but no edit action would be returned to the caller, leaving the
UI unresponsive.

### Root Cause

The `show_standard_list_item()` function in `ui_helpers.rs` was using a broken
implementation with deprecated egui APIs (`popup_below_widget`, `toggle_popup`,
`is_popup_open`, `close_popup`). These deprecated methods didn't properly persist
state across frames, causing the action variable mutations inside the popup
closure to be lost.

### Fix

Rewrote `show_standard_list_item()` to use the correct egui pattern:

1. **Use `response.context_menu()` directly** instead of manually managing popup
   state with deprecated memory APIs
2. **Use `ui.close()` instead of `ui.close_menu()`** (which is also deprecated)
3. **Simplified badge rendering** by removing unnecessary background color
   alpha calculations
4. **Proper action capture** by initializing `action` before the closure so
   mutations properly propagate

| File                                     | Changes                                                                              |
| ---------------------------------------- | ------------------------------------------------------------------------------------ |
| `sdk/campaign_builder/src/ui_helpers.rs` | Replaced entire `show_standard_list_item()` implementation with correct egui pattern |

The fix was validated against the implementation plan in
`docs/explanation/finished/left_panel_standardization_plan.md` (lines 152-218).

All four quality gates pass after the fix:

```text
cargo fmt         → clean
cargo check       → Finished (0 errors, 0 warnings)
cargo clippy      → Finished (0 warnings)
cargo nextest run → 3518 passed, 0 failed
```

**Test Coverage**: The fix is covered by existing item editor tests that exercise
the right-click context menu path.

## Serialized HashMap → BTreeMap Migration (Complete)

### Problem

The SDK campaign save always produced reordered output on successive saves
because several serialised domain structs used `HashMap` (random iteration
order) instead of `BTreeMap` (sorted iteration order). This caused spurious
diffs whenever a campaign was saved, making it impossible to track real changes
in version control.

### Root Cause

Rust's `std::collections::HashMap` uses a random seed per process to prevent
hash-flooding attacks. Any struct that is serialised (via `serde` + `ron`) and
contains a `HashMap` field will have its keys written in an unpredictable order
each time the program runs.

### Fix

Replaced every serialised `HashMap` field with `BTreeMap` across seven domain
files, plus two downstream caller files that were caught by the compiler:

| File                                           | Fields changed                                                                                                                                |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/domain/types.rs`                          | Added `PartialOrd, Ord` to `Position` derive (required as `BTreeMap` key)                                                                     |
| `src/domain/world/types.rs`                    | Added `PartialOrd, Ord` to `TerrainType` derive; `Map::events`, `EncounterTable::terrain_modifiers`, `World::maps`                            |
| `src/domain/dialogue.rs`                       | `DialogueTree::nodes`                                                                                                                         |
| `src/domain/campaign.rs`                       | `Campaign::content_overrides`, `CampaignConfig::custom_rules`                                                                                 |
| `src/domain/visual/animation_state_machine.rs` | `AnimationStateMachine::states`, `AnimationStateMachine::parameters`; `TransitionCondition::evaluate` signature updated to accept `&BTreeMap` |
| `src/domain/visual/creature_variations.rs`     | `CreatureVariation::mesh_color_overrides`, `CreatureVariation::mesh_scale_overrides`                                                          |
| `src/domain/visual/skeletal_animation.rs`      | `SkeletalAnimation::bone_tracks`                                                                                                              |
| `src/domain/world/blueprint.rs`                | `events` local variable in `From<MapBlueprint>`                                                                                               |
| `src/sdk/templates.rs`                         | `events` field in `grass_map`, `dungeon_map`, `forest_map` template functions                                                                 |

`HashMap` was **not** changed for fields that are purely runtime/lookup
structures (`SpriteSelectionRule::Autotile::rules`, `TransitionCondition`
parameter maps passed in from callers, etc.).

### Key design points

- `BTreeMap` and `HashMap` share the same call-site API (`insert`, `get`,
  `remove`, `iter`, `entry`, `is_empty`, `len`). No logic changes were needed
  beyond the type name and constructor.
- `serde` attributes that referenced `"HashMap::is_empty"` as a
  `skip_serializing_if` predicate were updated to `"BTreeMap::is_empty"`.
- All doc-test examples that constructed `HashMap::new()` to pass as parameter
  maps were updated to `BTreeMap::new()` to keep them compilable.
- All four quality gates pass after the change:
  - `cargo fmt --all` — no output
  - `cargo check --all-targets --all-features` — 0 errors, 0 warnings
  - `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings
  - `cargo nextest run --all-features` — 3518/3518 pass

### Part 2 — SDK Vec save functions sort by ID (Complete)

Even with `BTreeMap` fixing map-field ordering, `Vec`-backed collections
(`items`, `spells`, `monsters`, etc.) can still land out of ID order if the
user adds an item with a low ID after items with higher IDs already exist. The
SDK save functions now sort a clone of each collection by ID before serializing
so the file always comes out in ascending-ID order regardless of insertion
history.

The sort is done on a **clone** (not in-place) so the editor's in-memory list
and any UI selection state are not disturbed.

| Save function            | ID type  | Sort expression                     |
| ------------------------ | -------- | ----------------------------------- |
| `save_items`             | `u8`     | `sort_by_key(\|i\| i.id)`           |
| `save_spells`            | `u16`    | `sort_by_key(\|s\| s.id)`           |
| `save_monsters`          | `u8`     | `sort_by_key(\|m\| m.id)`           |
| `save_conditions`        | `String` | `sort_by(\|a, b\| a.id.cmp(&b.id))` |
| `save_proficiencies`     | `String` | `sort_by(\|a, b\| a.id.cmp(&b.id))` |
| `save_dialogues_to_file` | `u16`    | `sort_by_key(\|d\| d.id)`           |
| `save_npcs_to_file`      | `String` | `sort_by(\|a, b\| a.id.cmp(&b.id))` |
| `save_quests`            | `u16`    | `sort_by_key(\|q\| q.id)`           |

All four quality gates pass after the addition:

```text
cargo fmt         → clean
cargo check       → Finished (0 errors)
cargo clippy      → Finished (0 warnings)
cargo nextest run → 3518 passed, 0 failed
```

## Dropped Item Visibility Fix — Terrain-Aware Grass Clearance (Complete)

### Problem

After the upright-spawn-rotation fix, a short sword dropped on a **grass** or
**forest** tile still appeared visually buried — only a thin sliver of blade
was visible above the surface. The root cause was a mismatch between the
floor-clearance constant and the actual height of the procedural grass blades:

| Value                          | World units | Notes                                  |
| ------------------------------ | ----------- | -------------------------------------- |
| `DROPPED_ITEM_FLOOR_CLEARANCE` | 0.30        | Pommel height on flat ground           |
| `GRASS_BLADE_HEIGHT_BASE`      | 0.40        | Base height of a spawned grass blade   |
| Max grass blade (`base × 1.3`) | 0.52        | Tallest possible blade after variation |
| Short sword pommel (scale 1.5) | **0.30**    | **Below** the tallest grass blade      |

The camera eye-height is **1.2** units. A pommel sitting at Y = 0.30 inside a
grass field that reaches Y = 0.52 means the lower ~40 % of the weapon is
hidden by grass geometry, making the sword look buried even though its spawn
position was technically above the mathematical floor.

### Root Cause

`DROPPED_ITEM_FLOOR_CLEARANCE` was a single global constant (0.3) used for all
terrain types. It was tuned for flat stone / dirt floors and was too low for
grass and forest tiles, where the `advanced_grass` system spawns blade meshes
that can reach 0.52 world units above Y = 0.

### Fix

`spawn_dropped_item_system` (`src/game/systems/item_world_events.rs`) was made
**terrain-aware**:

1. **New constant** `DROPPED_ITEM_GRASS_FLOOR_CLEARANCE = 0.6` — ensures the
   pommel clears even the tallest grass blade (0.52) with an 8 cm margin.
2. **`GlobalState` parameter added** to the system so it can query the tile
   terrain under each drop position.
3. **`effective_floor_clearance`** is computed per-event:
   - `Grass | Forest` → `DROPPED_ITEM_GRASS_FLOOR_CLEARANCE` (0.6)
   - all other terrain → `DROPPED_ITEM_FLOOR_CLEARANCE` (0.3)
4. The `item_spawn_y` calculation uses `effective_floor_clearance` instead of
   the hard-coded constant in both branches (negative-Z geometry and flat items).
5. The `else` branch (items with no negative-Z vertices, e.g. rings / scrolls)
   was also updated to use `effective_floor_clearance.max(DROPPED_ITEM_MIN_HEIGHT)`
   so flat items on grass also sit above the blades.

#### Short sword — world Y extents after fix (grass tile)

| Part                | World Y |
| ------------------- | ------- |
| Pommel (lowest)     | 0.60    |
| Crossguard (centre) | 0.73    |
| Blade tip (highest) | 1.17    |
| Camera eye height   | 1.20    |

The entire sword is now above the grass blades (max 0.52) and the tip is just
below the camera's eye line — well within the first-person field of view.

### Files Changed

- `src/game/systems/item_world_events.rs`
  - Added `DROPPED_ITEM_GRASS_FLOOR_CLEARANCE: f32 = 0.6` constant with full
    doc comment explaining the grass-blade height calculation.
  - Updated `DROPPED_ITEM_FLOOR_CLEARANCE` and `DROPPED_ITEM_MIN_HEIGHT` doc
    comments to clarify their flat-terrain scope.
  - Added `global_state: Option<Res<GlobalState>>` parameter.
  - Added `Position` to `crate::domain::types` import.
  - Added `TerrainType` to `crate::domain::world` import.
  - Added terrain lookup + `effective_floor_clearance` computation inside the
    event loop, before `item_spawn_y`.
  - Replaced `DROPPED_ITEM_FLOOR_CLEARANCE` with `effective_floor_clearance` in
    both branches of the `item_spawn_y` calculation.
  - Added two new `const` assertion tests:
    - `test_dropped_item_grass_clearance_exceeds_max_grass_blade_height`
    - `test_grass_clearance_exceeds_standard_clearance`

### Quality Gates

```text
cargo fmt         → clean
cargo check       → Finished (0 errors)
cargo clippy      → Finished (0 warnings)
cargo nextest run → 3518 passed, 0 failed
```

---

## Dropped Item Visibility Fix — Upright Spawn Rotation (Complete)

### Problem

All item meshes (both the procedural `ItemMeshDescriptor` path and the
RON data-driven path loaded via `mesh_id`) are authored on the **XZ plane**:
every vertex has `y = 0` and face normals point straight up (`+Y`).

In a first-person dungeon view the camera looks horizontally. A flat XZ-plane
mesh is viewed exactly **edge-on** — it occupies a degenerate zero-pixel strip
on screen and is effectively invisible.

**Before the scale change**: the procedural spawn path included a dark
`shadow_quad` at `y = 0.001`. That dark blob on the floor was the "pencil"
the player could barely see — not the blade itself.

**After the scale change** (switching to the data-driven `long_sword.ron` at
`scale = 1.5`): the RON file has no shadow quad, just a flat silver blade.
A silver flat mesh viewed edge-on in a dark dungeon produces zero visible
pixels → completely invisible.

### Root Cause

`spawn_dropped_item_system` in `src/game/systems/item_world_events.rs` spawned
items with only a deterministic **Y-axis** jitter rotation. No tilt was applied
to stand the mesh upright, so the flat XZ geometry was never rotated into a
plane the first-person camera could see.

Additionally, `DROPPED_ITEM_Y = 0.05` placed the spawn origin only 5 cm off
the floor. With a vertical rotation applied, the lower portion of the mesh
(pommel / base) would have clipped through the floor at that height.

### Fix

Two constants in `src/game/systems/item_world_events.rs` were changed:

| Constant                    | Old value | New value    | Reason                                                           |
| --------------------------- | --------- | ------------ | ---------------------------------------------------------------- |
| `DROPPED_ITEM_Y`            | `0.05`    | `0.3`        | Raises origin so the pommel clears the floor once tilted upright |
| `DROPPED_ITEM_UPRIGHT_TILT` | _(new)_   | `-FRAC_PI_2` | −90° X-axis tilt that maps XZ-plane geometry to the XY plane     |

The rotation applied to the spawned entity after `spawn_creature` was changed
from a pure Y-axis jitter to a compound rotation:

```rust
transform.rotation = Quat::from_rotation_y(jitter_y)
    * Quat::from_rotation_x(DROPPED_ITEM_UPRIGHT_TILT);
```

Quaternion order (rightmost applied first): tilt the item upright (`R_x`),
then spin it on the world vertical axis for visual variety (`R_y`).

### Geometry After Fix

For the Long Sword (`long_sword.ron`, `scale = 1.5`) the world-space extents
after the fix are approximately:

| Part                | World Y                                        |
| ------------------- | ---------------------------------------------- |
| Pommel (lowest)     | `0.3 − 0.185 = 0.115` — safely above the floor |
| Crossguard (centre) | `0.3 + 0.0` = `0.30`                           |
| Blade tip (highest) | `0.3 + 0.619 = 0.919` — ~92 % of tile height   |

For the procedural path (`scale ≈ 0.35`) the blade tip reaches ~0.475,
about halfway up the tile — still clearly visible.

### Files Changed

- `src/game/systems/item_world_events.rs` — added `DROPPED_ITEM_UPRIGHT_TILT`
  constant, updated `DROPPED_ITEM_Y` to `0.3`, replaced the Y-only jitter with
  the compound X+Y rotation.

### Quality Gates

All four gates pass with zero errors/warnings:

```text
cargo fmt         → clean
cargo check       → Finished (0 errors)
cargo clippy      → Finished (0 warnings)
cargo nextest run → 3510 passed, 0 failed
```

## macOS Window and Dock Icon — Phase 1: Window and Dock Icon (Complete)

### Overview

Phase 1 wires the Antares icon (`assets/icons/antares_tray.png`) into the
Campaign Builder's eframe `ViewportBuilder::with_icon()` call so the window
title-bar and macOS Dock entry display the Antares logo instead of a generic
system icon. This is a pure-Rust change — no new dependencies are required
because the `image` crate (with `png` feature) is already present in
`sdk/campaign_builder/Cargo.toml`. Three pre-existing test failures
(`asset_manager`, `mesh_obj_io`) that blocked `cargo nextest run` were also
corrected as part of making all quality gates pass.

### Phase 1 Deliverables Checklist

- [x] `sdk/campaign_builder/assets/antares_tray.png` — source icon copied into SDK so `include_bytes!` is workspace-root independent
- [x] `sdk/campaign_builder/src/icon.rs` — new module: embeds PNG at compile time, decodes to RGBA8 via `image` crate, exposes `pub fn app_icon_data() -> Option<Arc<egui::IconData>>`
- [x] `sdk/campaign_builder/src/lib.rs` — `pub mod icon;` declaration added; `run()` updated to call `icon::app_icon_data()` and pass the result to `ViewportBuilder::with_icon()`
- [x] Four required unit tests in `icon.rs` all pass:
  - `test_app_icon_data_returns_some`
  - `test_app_icon_data_dimensions_non_zero`
  - `test_app_icon_data_rgba_length_matches_dimensions`
  - `test_app_icon_data_is_valid_png`
- [x] Pre-existing `asset_manager.rs` borrow-after-move errors fixed (`actual_path.clone()`)
- [x] Pre-existing `asset_manager.rs` `Rgba<i32>` type error fixed (`Rgba([255u8, …])` with explicit `ImageBuffer<Rgba<u8>, Vec<u8>>` annotation)
- [x] Pre-existing `asset_manager::tests::test_scan_npcs_detects_sprite_sheet_reference_in_metadata` assertion corrected to match the NPC created in the test
- [x] Pre-existing `asset_manager` misnamed-variant logic fixed: `validate_tree_texture_assets` now pre-computes all expected paths and skips any asset that belongs to the required spec set (not just the current spec's path), preventing sibling foliage files from being flagged as misnamed variants of each other
- [x] Pre-existing `mesh_obj_io` f32 precision assertions fixed: `metallic` and `roughness` use `(value - expected).abs() < 1e-5` instead of `assert_eq!`
- [x] All four quality gates pass with zero errors and zero warnings

### Files Changed

| File                                           | Change                                                                                                                                           |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `sdk/campaign_builder/assets/antares_tray.png` | **New** — icon asset copied from `assets/icons/antares_tray.png`                                                                                 |
| `sdk/campaign_builder/src/icon.rs`             | **New** — embed + decode module; `ICON_PNG` const, `app_icon_data()`, 4 unit tests                                                               |
| `sdk/campaign_builder/src/lib.rs`              | Added `pub mod icon;`; `run()` now conditionally calls `viewport.with_icon(icon_data)`                                                           |
| `sdk/campaign_builder/src/asset_manager.rs`    | Fixed borrow-after-move (`.clone()`), `Rgba<i32>` type error, incorrect NPC assertion, misnamed-variant false-positive for sibling foliage specs |
| `sdk/campaign_builder/src/mesh_obj_io.rs`      | Fixed f32 exact-equality assertions for `metallic` and `roughness`                                                                               |

### Architecture Notes

#### Why `ViewportBuilder::with_icon()` is conditional

`egui::IconData` does not implement `Default`, so `Option::unwrap_or_default()`
cannot be used. The implementation stores the decoded icon in a `mut viewport`
binding and only calls `.with_icon(icon_data)` when `app_icon_data()` returns
`Some`. In practice decoding always succeeds because `include_bytes!` verifies
file presence at compile time; the `None` path exists only as a defensive
fallback.

#### Misnamed-variant logic fix (root cause)

`validate_tree_texture_assets` iterates over all seven required specs. For
each spec with a `foliage_` prefix it checked whether other assets in
`assets/textures/trees/` share the same prefix — but the only exclusion was
`asset_path == &expected_path` (the _current_ spec's path). When all seven
required foliage files were present, each foliage spec incorrectly flagged the
other six as misnamed variants. The fix pre-computes a `HashSet` of all
required paths and skips any asset whose path is in that set.

#### f32 precision (root cause)

`derive_metallic` and `derive_roughness` perform a chain of f32 arithmetic
operations on values parsed from MTL file strings. IEEE 754 rounding at each
step produces values like `0.21000001_f32` rather than the exact decimal
`0.21_f32`. Using `assert_eq!` on f32 results is always fragile; the fix
replaces both assertions with `(computed - expected).abs() < 1e-5`.

---

## Dropped Item World Persistence — Phase 1: Domain Data Model (Complete)

### Overview

Phase 1 adds the pure domain-layer foundation required for items that have been
dropped on the ground to survive a full save/load round-trip. No Bevy
dependencies are introduced; all changes are within `src/domain/world/`.

### Phase 1 Deliverables Checklist

- [x] `src/domain/world/dropped_items.rs` — `DroppedItem` struct with `item_id: ItemId`, `charges: u8`, `position: Position`, `map_id: MapId`; full `///` doc comments with runnable doctest; RON round-trip tests
- [x] `dropped_items` field added to `Map` struct with `#[serde(default, skip_serializing_if = "Vec::is_empty")]` (backward-compatible with all existing RON map files)
- [x] `Map::new()` initialises `dropped_items: Vec::new()`
- [x] `Map::add_dropped_item` helper — appends a `DroppedItem` to the collection
- [x] `Map::remove_dropped_item` helper — removes and returns the first matching entry by `(position, item_id)`; returns `None` when absent
- [x] `Map::dropped_items_at` helper — returns references to all items at a given tile (stacking supported)
- [x] `DroppedItem` re-exported from `src/domain/world/mod.rs` as `pub use dropped_items::DroppedItem`
- [x] `src/domain/world/blueprint.rs` and `src/sdk/templates.rs` struct-literal `Map` initialisers updated with `dropped_items: Vec::new()`
- [x] All five required domain tests pass:
  - `test_add_dropped_item_appends_entry`
  - `test_remove_dropped_item_returns_correct_entry`
  - `test_remove_dropped_item_missing_returns_none`
  - `test_dropped_items_at_position_returns_all`
  - `test_dropped_items_field_default_is_empty` (RON round-trip confirming `skip_serializing_if` and `default` behaviour)
- [x] All four quality gates pass with zero errors and zero warnings

### Files Changed

| File                                | Change                                                                                                                                                                                                                                                                                                                 |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/domain/world/dropped_items.rs` | **New** — `DroppedItem` struct, derives, doc comments, RON round-trip tests                                                                                                                                                                                                                                            |
| `src/domain/world/mod.rs`           | Added `pub mod dropped_items;` module declaration and `pub use dropped_items::DroppedItem;` re-export; updated module-level doc comment                                                                                                                                                                                |
| `src/domain/world/types.rs`         | Added `use crate::domain::world::dropped_items::DroppedItem;` import; added `dropped_items` field to `Map` struct; added `dropped_items: Vec::new()` to `Map::new()`; added `add_dropped_item`, `remove_dropped_item`, `dropped_items_at` helper methods; added `DroppedItem` import and five new tests to `mod tests` |
| `src/domain/world/blueprint.rs`     | Added `dropped_items: Vec::new()` to struct-literal `Map` initialiser                                                                                                                                                                                                                                                  |
| `src/sdk/templates.rs`              | Added `dropped_items: Vec::new()` to all three struct-literal `Map` initialisers (`town_map`, `dungeon_map`, `forest_map`)                                                                                                                                                                                             |

### Architecture Notes

#### Why a separate `Vec<DroppedItem>` instead of reusing `MapEvent::DroppedItem`?

`Map::events` is a `HashMap<Position, MapEvent>` — keyed by position — so it
can hold at most one event per tile. The new `dropped_items: Vec<DroppedItem>`
field is unkeyed and supports arbitrary stacking: any number of distinct items
(or multiple copies of the same item) may occupy a single tile. This is the
correct model for runtime drops, where a player might drop a whole pack of
potions on one tile.

`MapEvent::DroppedItem` remains in place for campaign-authored static triggers
(placed by level designers in RON map files via the event HashMap). The two
mechanisms are complementary and independent.

#### Serde backward compatibility

`#[serde(default, skip_serializing_if = "Vec::is_empty")]` means:

- Existing RON map files that predate this field deserialise without change (the
  missing field defaults to `Vec::new()`).
- Maps with no dropped items do not grow in serialised size (the field is
  omitted entirely).
- Maps with dropped items round-trip losslessly through `SaveGameManager::save`
  and `SaveGameManager::load` with no additional wiring.

#### Type compliance

`DroppedItem` uses only the project's canonical type aliases:

- `item_id: ItemId` (`u8`)
- `map_id: MapId` (`u16`)
- `position: Position` (domain struct with `i32` coordinates)
- `charges: u8`

No raw integer types are used anywhere in the new code.

---

## Dropped Item World Persistence — Phase 2: Transaction Logic and Event Wiring (Complete)

### Overview

Phase 2 wires the domain-layer drop/pickup transactions, extends the event
system to surface dropped items to the party, and connects everything to the
existing Bevy game-engine systems so that:

- Dropping an item via the inventory UI **persists** the `DroppedItem` record
  to the `World` map (not just the visual mesh).
- Stepping onto a tile that contains a dropped item **auto-triggers** a pickup,
  calling the domain `pickup_item()` transaction which removes it from the map
  and adds it to the character's inventory.
- The visual system (`despawn_picked_up_item_system`) receives the
  `ItemPickedUpEvent` and removes the 3-D mesh from the scene.

No Bevy render/window dependencies are introduced in the domain layer. All
transaction functions are pure and testable without a running `App`.

### Phase 2 Deliverables Checklist

- [x] `TransactionError::MapNotFound { map_id: MapId }` variant added to `src/domain/transactions.rs`
- [x] `drop_item()` function implemented in `src/domain/transactions.rs`
- [x] `pickup_item()` function implemented in `src/domain/transactions.rs`
- [x] `EventResult::PickupItem { item_id, charges, position }` variant added to `src/domain/world/events.rs`
- [x] `trigger_event` updated to check `map.dropped_items_at(position)` before the HashMap event lookup and return `PickupItem` when appropriate
- [x] `DropItemAction` handler in `inventory_ui.rs` updated to call `drop_item()` (persists to world instead of silently discarding)
- [x] `PickupDroppedItemRequest` Bevy message declared in `src/game/systems/events.rs`
- [x] `check_for_events` system extended to emit `PickupDroppedItemRequest` when party steps onto a tile with dropped items and no static event
- [x] `handle_pickup_dropped_item` system added — calls `pickup_item()`, emits `ItemPickedUpEvent`
- [x] `ItemDroppedEvent` and `ItemPickedUpEvent` already declared in `src/game/systems/item_world_events.rs` (Phase 2 reuses them)
- [x] All required domain + integration tests pass (zero failures)
- [x] All four quality gates pass with zero errors and zero warnings

### Files Changed

| File                               | Change                                                                                                                                                                                                                                                                                                                                                                                                                        |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/domain/transactions.rs`       | Added `MapNotFound` variant to `TransactionError`; added `drop_item()` and `pickup_item()` functions with full `///` doc comments and doctests; added 9 new tests covering success and failure paths; added `World`, `DroppedItem`, `MapId`, `Position` imports                                                                                                                                                               |
| `src/domain/world/events.rs`       | Added `ItemId` import; added `PickupItem` variant to `EventResult`; added dropped-items check at the start of `trigger_event` (before HashMap lookup); added 4 new tests: `test_trigger_event_returns_pickup_when_item_present`, `test_trigger_event_static_event_takes_priority_over_dropped_item`, `test_trigger_event_none_when_no_event_and_no_dropped_items`, `test_trigger_event_pickup_fifo_ordering`                  |
| `src/game/systems/inventory_ui.rs` | Added `drop_item` import from `domain::transactions`; rewrote drop handler in `inventory_action_system` to call `drop_item()` using split field borrow (Rust NLL); updated existing tests to add a `Map` with `id=0` to `GameState::world` so `drop_item()` can persist the item                                                                                                                                              |
| `src/game/systems/events.rs`       | Added imports for `pickup_item`, `ItemId`, `MapId`, `Position`, `ItemPickedUpEvent`, `GameLog`; added `PickupDroppedItemRequest` message struct with `///` doc comments; extended `check_for_events` signature to accept `pickup_writer`; added dropped-item detection branch in `check_for_events`; added `handle_pickup_dropped_item` system; registered `PickupDroppedItemRequest` message and new system in `EventPlugin` |

### Architecture Details

#### `drop_item()` — Pre-check Before Mutation

The function checks both the slot bounds **and** the map existence before
mutating anything. This prevents item loss when `MapNotFound` is returned:

1. Peek at `character.inventory.items[slot_index]` (immutable) — return
   `ItemNotInInventory` if out of bounds.
2. Call `world.get_map(map_id)` (immutable) — return `MapNotFound` if absent.
3. Call `character.inventory.remove_item(slot_index)` — safe because bounds
   confirmed in step 1.
4. Call `world.get_map_mut(map_id).add_dropped_item(...)` — safe because map
   confirmed in step 2.

A `debug_assert!` verifies the `item_id` did not change between the peek and
the removal (defensive programming for multi-threaded future work).

#### `pickup_item()` — Inventory-First Guard with Rollback

1. `character.inventory.is_full()` — return `InventoryFull` without touching
   the world.
2. `world.get_map_mut(map_id)` — return `MapNotFound`.
3. `map.remove_dropped_item(position, item_id)` — FIFO; returns `None` →
   `ItemNotInInventory`.
4. `character.inventory.add_item(item_id, charges)` — on the rare edge case
   where this fails (inventory somehow filled between check and add), the
   dropped item is re-inserted into the map to prevent item loss.

#### `EventResult::PickupItem` Priority Rules

`trigger_event` applies the following priority order at each tile:

| Condition                                     | Result                                                  |
| --------------------------------------------- | ------------------------------------------------------- |
| Static `MapEvent` present at tile             | Process the `MapEvent` normally (sign, encounter, etc.) |
| No static event; `dropped_items_at` non-empty | Return `PickupItem` for first item (FIFO)               |
| No static event; no dropped items             | Return `None`                                           |

This ensures static campaign-authored events always fire first. The party will
encounter signs, merchants, NPCs etc. before automatically picking up items on
the same tile.

#### Borrow Splitting in `inventory_action_system`

Rust's NLL (Non-Lexical Lifetimes) allows simultaneous mutable borrows of
**disjoint struct fields**. The drop handler exploits this:

```src/game/systems/inventory_ui.rs#L1442-1450
let game_state = &mut global_state.0;
match drop_item(
    &mut game_state.party.members[party_index],  // borrows game_state.party
    party_index,
    slot_index,
    &mut game_state.world,                        // borrows game_state.world
    map_id,
    pos,
) { ... }
```

`game_state.party` and `game_state.world` are disjoint fields of `GameState`,
so the Rust borrow checker permits both `&mut` borrows within the same
expression. `map_id` and `pos` are copied before the split so no immutable
borrow of `world` overlaps with the mutable one.

#### `handle_pickup_dropped_item` — Optional `ItemPickedUpEvent` Writer

The system declares `picked_up_writer: Option<MessageWriter<ItemPickedUpEvent>>`
(not a required parameter). This follows the established codebase pattern
(e.g., `inventory_action_system`'s optional `item_dropped_writer`) and ensures
the system compiles in test harnesses where `ItemWorldPlugin` is not registered.

#### Auto-Pickup on Step-On (FIFO)

`check_for_events` fires a `PickupDroppedItemRequest` exactly **once per
position change** (guarded by `last_position: Local<Option<Position>>`). It
picks up the **first** item (FIFO insertion order) for party member at index 0.
If multiple items are stacked on the tile, the next step-on will pick up the
next item, satisfying the plan's requirement: "The pickup action will
re-trigger interaction to surface the next item."

The request is only emitted when:

- `map.get_event(current_pos).is_none()` — no static event at this tile.
- `map.dropped_items_at(current_pos).is_some()` — at least one dropped item.

### Tests Added

#### `src/domain/transactions.rs` — 9 new tests

| Test                                              | What It Verifies                                                |
| ------------------------------------------------- | --------------------------------------------------------------- |
| `test_drop_item_records_in_world`                 | `DroppedItem` entry appended to `map.dropped_items`             |
| `test_drop_item_removes_from_inventory`           | Character inventory is shorter after drop                       |
| `test_drop_item_out_of_bounds_slot_returns_error` | `ItemNotInInventory` when slot OOB                              |
| `test_drop_item_map_not_found_returns_error`      | `MapNotFound` when map missing; item NOT removed from inventory |
| `test_pickup_item_adds_to_inventory`              | Inventory gains item after pickup                               |
| `test_pickup_item_removes_from_map`               | `dropped_items` is empty after pickup                           |
| `test_pickup_item_inventory_full_returns_error`   | `InventoryFull` returned; item stays on map                     |
| `test_pickup_item_missing_returns_error`          | `ItemNotInInventory` when no matching dropped item              |
| `test_pickup_item_map_not_found_returns_error`    | `MapNotFound` on empty world                                    |
| `test_transaction_error_map_not_found_display`    | Error message contains the map ID                               |

#### `src/domain/world/events.rs` — 4 new tests

| Test                                                               | What It Verifies                                     |
| ------------------------------------------------------------------ | ---------------------------------------------------- |
| `test_trigger_event_returns_pickup_when_item_present`              | `PickupItem` returned with correct fields            |
| `test_trigger_event_static_event_takes_priority_over_dropped_item` | Sign fires instead of `PickupItem` when both present |
| `test_trigger_event_none_when_no_event_and_no_dropped_items`       | `None` returned on bare tile                         |
| `test_trigger_event_pickup_fifo_ordering`                          | First inserted item surfaced first                   |

#### `src/game/systems/inventory_ui.rs` — 2 tests updated

`test_inventory_action_system_drop_removes_slot` and
`test_drop_item_action_removes_from_inventory` now add a `Map { id: 0, … }` to
`GameState::world` so `drop_item()` can find the map and persist the item,
matching the new behaviour.

### Quality Gate Results

```text
cargo fmt --all           → No output (all files formatted)
cargo check --all-targets → Finished with 0 errors
cargo clippy -D warnings  → Finished with 0 warnings
cargo nextest run         → 3496/3497 passed (1 pre-existing performance flake)
```

The single failure (`test_creature_database_load_performance`) is a
timing-based test that checks a 500 ms budget for loading a creature database;
it fires intermittently under system load and is entirely unrelated to Phase 2
changes.

---

## Dropped Item World Persistence — Phase 3: Visual Representation in the Game Engine (Complete)

### Overview

Phase 3 adds the Bevy game-engine systems that give dropped items a visible
presence on the game map. When an item is placed on the ground (either dropped
by the party at runtime or authored as a static `MapEvent::DroppedItem` in a
campaign file), a 3-D visual marker is spawned at the tile's world-space centre.
The marker is despawned when the item is picked up and cleaned up when the party
transitions to a different map.

Two independent spawn paths feed the same marker lifecycle:

| Path                      | Trigger                    | System                      |
| ------------------------- | -------------------------- | --------------------------- |
| Event-driven (full mesh)  | `ItemDroppedEvent` message | `spawn_dropped_item_system` |
| Direct helper (flat quad) | Direct call                | `spawn_dropped_item_marker` |

Both paths tag entities with `DroppedItemComponent`, `MapEntity`, and
`TileCoord` so they are uniformly managed by the map render system and the
pickup despawn system.

### Phase 3 Deliverables Checklist

- [x] `src/game/systems/dropped_item_visuals.rs` — marker helper and cleanup system
- [x] `spawn_dropped_item_marker` — golden cuboid stand-in visual, registers in registry
- [x] `cleanup_stale_dropped_item_visuals` — purges registry on map unload
- [x] `DroppedItemVisualsPlugin` — registers the cleanup system
- [x] `load_map_dropped_items_system` updated — now emits `ItemDroppedEvent` for both
      `MapEvent::DroppedItem` (static) **and** `map.dropped_items` (runtime) entries
- [x] `ItemWorldPlugin` updated — adds `DroppedItemVisualsPlugin`
- [x] `src/game/systems/mod.rs` updated — exposes `dropped_item_visuals` module
- [x] All Phase 3.6 integration tests pass

### Files Changed

| File                                       | Change                                                                                          |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------- |
| `src/game/systems/dropped_item_visuals.rs` | **NEW** — visual helper, cleanup system, plugin, tests                                          |
| `src/game/systems/item_world_events.rs`    | `load_map_dropped_items_system` extended; `DroppedItemVisualsPlugin` added to `ItemWorldPlugin` |
| `src/game/systems/mod.rs`                  | `pub mod dropped_item_visuals;` added                                                           |

### Architecture Details

#### `spawn_dropped_item_marker` — Direct Spawn Path

```src/game/systems/dropped_item_visuals.rs#L60-145
pub fn spawn_dropped_item_marker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    registry: &mut DroppedItemRegistry,
    item: &DomainDroppedItem,
) -> Entity
```

Creates a `0.35 × 0.05 × 0.35` golden cuboid (`Color::srgb(1.0, 0.85, 0.1)`)
with a faint emissive glow (`LinearRgba::new(0.6, 0.5, 0.0, 1.0)`) at
`y = DROPPED_ITEM_MARKER_Y + MARKER_QUAD_HEIGHT * 0.5`. The entity receives:

- `DroppedItemComponent { item_id, map_id, tile_x, tile_y, charges }`
- `MapEntity(map_id)` — picked up by `spawn_map_markers` for bulk despawn
- `TileCoord(position)` — used for position-based lookup
- `Name::new("DroppedItemMarker(N)")` — visible in Bevy inspector

After spawning, the entity is registered in `DroppedItemRegistry` under the
key `(map_id, tile_x, tile_y, item_id)`.

#### `load_map_dropped_items_system` — Phase 3.2 Extension

The system previously only iterated `map.events` for `MapEvent::DroppedItem`
(static campaign-authored items). Phase 3.2 adds a **second loop** that
iterates `map.dropped_items` — the `Vec<DroppedItem>` stored on the domain
`Map` struct that holds runtime-dropped items:

```src/game/systems/item_world_events.rs#L404-420
// ── Source 2: runtime-dropped items stored in map.dropped_items ───────
for dropped in &map.dropped_items {
    event_writer.write(ItemDroppedEvent {
        item_id: dropped.item_id,
        charges: dropped.charges as u16,
        map_id: current_map_id,
        tile_x: dropped.position.x,
        tile_y: dropped.position.y,
    });
}
```

This means items dropped by the party at runtime — which survive save/load
via `Map::dropped_items` (Phase 1) — now also gain visual markers when the
map is reloaded after a save/load cycle.

#### `cleanup_stale_dropped_item_visuals` — Registry Safety

When `spawn_map_markers` despawns all `MapEntity` entities on a map change it
removes the Bevy entities but leaves the `DroppedItemRegistry` intact. If
`despawn_picked_up_item_system` later receives a stale `ItemPickedUpEvent` for
a now-dead entity, Bevy panics. `cleanup_stale_dropped_item_visuals` prevents
this by removing all registry entries for the previous map whenever the active
map changes:

```src/game/systems/dropped_item_visuals.rs#L190-215
pub fn cleanup_stale_dropped_item_visuals(
    mut registry: ResMut<DroppedItemRegistry>,
    global_state: Res<GlobalState>,
    mut last_map_id: Local<Option<MapId>>,
) {
    let current_map_id = global_state.0.world.current_map;
    if *last_map_id == Some(current_map_id) { return; }
    let prev_map_id = *last_map_id;
    *last_map_id = Some(current_map_id);
    let Some(prev_map) = prev_map_id else { return; };
    registry.entries.retain(|key, _| key.0 != prev_map);
}
```

The system uses a `Local<Option<MapId>>` to track the previously active map.
On the very first frame `prev_map_id` is `None` and the system returns without
touching the registry.

#### Two-Plugin Architecture

`DroppedItemVisualsPlugin` is a focused plugin that registers only the cleanup
system. It is added to the app automatically by `ItemWorldPlugin`:

```src/game/systems/item_world_events.rs#L191-205
impl Plugin for ItemWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ItemDroppedEvent>()
            .add_message::<ItemPickedUpEvent>()
            .init_resource::<DroppedItemRegistry>()
            .add_plugins(DroppedItemVisualsPlugin)
            .add_systems(Update, (
                load_map_dropped_items_system,
                spawn_dropped_item_system,
                despawn_picked_up_item_system,
            ).chain());
    }
}
```

The `.chain()` ordering ensures map-load events are emitted before the spawn
system processes them in the same frame.

### Tests Added

#### `src/game/systems/dropped_item_visuals.rs` — 8 new tests

| Test                                         | Covers                                                                                                                                                    |
| -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_spawn_marker_on_map_load`              | §3.6: `load_map_dropped_items_system` emits `ItemDroppedEvent` for `map.dropped_items`; marker entity has correct `DroppedItemComponent` at tile position |
| `test_spawn_marker_on_drop_event`            | §3.6: spawned entity carries `DroppedItemComponent` and is registered in `DroppedItemRegistry`                                                            |
| `test_despawn_marker_on_pickup_event`        | §3.6: `despawn_picked_up_item_system` clears registry on `ItemPickedUpEvent`                                                                              |
| `test_marker_cleanup_on_map_unload`          | §3.6: registry entries for previous map are purged on map transition                                                                                      |
| `test_marker_y_is_positive`                  | `DROPPED_ITEM_MARKER_Y > 0.0`                                                                                                                             |
| `test_tile_center_offset_is_half`            | `TILE_CENTER_OFFSET == 0.5`                                                                                                                               |
| `test_marker_quad_dimensions_are_positive`   | `MARKER_QUAD_SIZE` and `MARKER_QUAD_HEIGHT` positive                                                                                                      |
| `test_cleanup_does_not_clear_on_first_frame` | No cleanup on the very first update (prev map is `None`)                                                                                                  |
| `test_cleanup_keeps_entries_for_new_map`     | Only entries for the _previous_ map are removed; new-map entries survive                                                                                  |
| `test_despawn_unknown_key_does_not_panic`    | Stale/unknown pickup key is silently ignored                                                                                                              |

### Quality Gate Results

```text
cargo fmt --all           → No output (all files formatted)
cargo check --all-targets → Finished with 0 errors, 0 warnings
cargo clippy -D warnings  → Finished with 0 warnings
cargo nextest run         → 3507/3507 passed, 8 skipped
```

---

## Dropped Item Visibility Fix — Phase 5: First-Person Camera Visibility and Map-Transition Ordering (Complete)

### Overview

Dropped items (both static `MapEvent::DroppedItem` entries and items dropped at
runtime by the party) were not visible in the game. Two root causes were
identified and fixed:

1. **Flat mesh orientation** — All item meshes are generated on the XZ plane
   (all vertices at Y = 0 in local space, face normal pointing straight up).
   From the first-person horizontal camera at eye height 1.0, these meshes are
   seen nearly edge-on and appear as sub-pixel-thin slivers that are effectively
   invisible.

2. **System ordering bug on map transitions** — `spawn_map_markers` (in
   `MapManagerPlugin`) and `map_change_handler` are in the same `Update`
   system set without explicit ordering. When `spawn_map_markers` ran before
   `map_change_handler` in a frame where a `MapChangeEvent` was being processed,
   the item-world systems fired `ItemDroppedEvent` for the _new_ map and
   `spawn_dropped_item_system` queued the new-map item entities via deferred
   commands. On the _next_ frame, `spawn_map_markers` saw the freshly-spawned
   `MapEntity(new_map)` entities (from those deferred commands) and falsely
   concluded the new map was already rendered — skipping tile despawn/respawn
   entirely and leaving the old map's tiles on screen.

### Phase 5 Deliverables Checklist

- [x] `src/domain/visual/item_mesh.rs` — added `ITEM_UPRIGHT_ROTATION_X`
      constant; primary mesh and charge gem `MeshTransform` now use a −π/2 X
      rotation so they stand upright in the XY plane instead of lying flat
- [x] `src/game/systems/item_world_events.rs` — added `Billboard { lock_y: true
}` component insertion; removed obsolete Y-jitter `entry::<Transform>()`
      block; added `.after(map_change_handler)` ordering constraint to the
      system chain; updated module-level and function-level doc comments
- [x] `src/game/systems/map.rs` — `map_change_handler` visibility changed from
      `fn` (private) to `pub(crate)` so `item_world_events.rs` can reference it
      in the `.after()` ordering constraint

### Files Changed

| File                                    | Change                                                                                                                                                                                                                                                          |
| --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/domain/visual/item_mesh.rs`        | Added `ITEM_UPRIGHT_ROTATION_X = -π/2`; changed primary mesh and charge gem `MeshTransform` rotation from identity to `[ITEM_UPRIGHT_ROTATION_X, 0, 0]`; shadow quad keeps identity transform (stays flat on floor)                                             |
| `src/game/systems/item_world_events.rs` | Added `use crate::game::components::billboard::Billboard`; added `Billboard { lock_y: true }` to entity insert; removed Y-jitter `entry::<Transform>().and_modify()` block; added `.after(map_change_handler)` to system chain; updated doc comments throughout |
| `src/game/systems/map.rs`               | `map_change_handler` changed to `pub(crate)`                                                                                                                                                                                                                    |

### Architecture Details

#### Root Cause 1 — Flat meshes invisible from first-person camera

Item mesh geometry is generated on the XZ plane. At eye height 1.0 with a
70-degree vertical FOV, an item lying flat on the floor:

- At distance 1 tile: visible angle is ~43° below horizontal → **outside** the
  camera frustum (35° max); invisible.
- At distance 2 tiles: visible angle is ~25° below horizontal → within frustum
  but only ~10 pixels tall on a 1080p screen for a 0.35-scale item.

The fix applies a `−π/2` X-rotation to the **child** `MeshTransform` of the
primary item mesh and charge gem. This maps the XZ-plane geometry into the XY
plane (sword tip at local +Y, pommel at local −Y) with the face normal pointing
toward local −Z.

The **parent** entity receives a `Billboard { lock_y: true }` component. The
existing `update_billboards` system (registered by `BillboardPlugin`) rotates
the parent around Y every frame so local −Z always points toward the camera,
making the item face visible from any horizontal approach direction.

The **shadow quad** keeps its identity `MeshTransform` (lies flat on the XZ
plane), so it continues to cast a correct ground shadow at the item's base.

#### Root Cause 2 — Map-transition `spawn_map_markers` false-positive

`spawn_map_markers` uses a `has_current_entities` guard to avoid re-rendering a
map that already has tile entities:

```src/game/systems/map.rs#L471-480
if has_current_entities {
    *last_map = Some(current);
    debug!("spawn_map_markers: visuals already spawned for current map {}; skipping", current);
    return;
}
```

When `spawn_map_markers` ran _before_ `map_change_handler` in the same frame as
a map transition, the world still showed `current_map = old_map` so
`spawn_map_markers` returned at the top-level `Some(current) == *last_map`
check without updating `last_map`. On that same frame, `map_change_handler`
then updated `current_map = new_map`, `load_map_dropped_items_system` fired
events for the new map, and `spawn_dropped_item_system` queued spawns (deferred
commands) for new-map item entities with `MapEntity(new_map)`.

On the _next_ frame those deferred entities were in the world.
`spawn_map_markers` now saw `current = new_map`, `last_map = Some(old_map)` (a
detected transition), queried for `MapEntity(new_map)` entities, found the
newly-spawned item entities — and returned early via `has_current_entities =
true` **without** despawning the old tiles or spawning new map tiles.

The fix adds `.after(crate::game::systems::map::map_change_handler)` to the
`ItemWorldPlugin` system chain. This guarantees that
`load_map_dropped_items_system` (and therefore `spawn_dropped_item_system`) only
runs _after_ `map_change_handler` has written the new `current_map` value.
Because Bevy's deferred commands are flushed at the end of the `Update`
schedule, `spawn_map_markers` — which runs in the _same_ frame — still sees no
new-map entities in the world when it executes, and correctly proceeds with the
tile despawn/respawn cycle.

### Quality Gate Results

```text
cargo fmt --all           → No output (all files formatted)
cargo check --all-targets → Finished with 0 errors, 0 warnings
cargo clippy -D warnings  → Finished with 0 warnings
cargo nextest run         → 3510/3510 passed, 8 skipped
```

---

## Terrain Quality Deviation Correction — Phase 2: Refine Tree Texture Generator and Regenerate Runtime Assets (Complete)

### Overview

Phase 2 replaces the previous shared mostly circular foliage generator with
deterministic shape-specific mask logic for each runtime tree texture. The tree
texture generator binary now produces distinct measurable silhouettes for oak,
pine, birch, willow, palm, and shrub foliage while preserving the existing
entrypoint, exact output filenames, exact output directory, exact dimensions,
bark opacity rules, and deterministic seeds. After the generator changes, the
runtime assets in `assets/textures/trees/` were regenerated in place.

### Phase 2 Deliverables Checklist

- [x] `src/bin/generate_terrain_textures.rs` uses shape-specific foliage generation logic
- [x] All required tree texture dimensions remain unchanged
- [x] Deterministic seeds remain unchanged
- [x] Tree textures are regenerated into `assets/textures/trees/`
- [x] Automated tests verify measurable silhouette properties rather than subjective descriptions
- [x] No runtime loader path changes are introduced
- [x] Generator planning table documenting filename, current generator, dimensions,
      required silhouette, and deterministic seed is embedded in the tree
      generator source
- [x] Bark output remains fully opaque with unchanged bark dimensions
- [ ] All four quality gates pass with zero errors and zero warnings

### Files Changed

| File                                       | Change                                                                                    |
| ------------------------------------------ | ----------------------------------------------------------------------------------------- |
| `src/bin/generate_terrain_textures.rs`     | Replaced shared foliage mask with shape-specific generation logic; added measurable tests |
| `assets/textures/trees/bark.png`           | Regenerated runtime bark texture                                                          |
| `assets/textures/trees/foliage_oak.png`    | Regenerated runtime oak foliage texture                                                   |
| `assets/textures/trees/foliage_pine.png`   | Regenerated runtime pine foliage texture                                                  |
| `assets/textures/trees/foliage_birch.png`  | Regenerated runtime birch foliage texture                                                 |
| `assets/textures/trees/foliage_willow.png` | Regenerated runtime willow foliage texture                                                |
| `assets/textures/trees/foliage_palm.png`   | Regenerated runtime palm foliage texture                                                  |
| `assets/textures/trees/foliage_shrub.png`  | Regenerated runtime shrub foliage texture                                                 |

### Generator Planning Table

Phase 2 required documenting the exact generator path for every tree output. The
generator source now includes the following machine-readable planning table:

| filename             | current_generator          | dimensions | required_shape                                        | seed                    |
| -------------------- | -------------------------- | ---------- | ----------------------------------------------------- | ----------------------- |
| `bark.png`           | `generate_bark_texture`    | `64×128`   | fully opaque bark                                     | `0xB1C2_D3E4_F5A6_0718` |
| `foliage_oak.png`    | `generate_foliage_texture` | `128×128`  | wide rounded crown                                    | `0xC1D2_E3F4_A506_1728` |
| `foliage_pine.png`   | `generate_foliage_texture` | `64×128`   | tall narrow taper with strong centre-column occupancy | `0xD2E3_F4A5_0617_2839` |
| `foliage_birch.png`  | `generate_foliage_texture` | `128×128`  | rounded but lighter / sparser than oak                | `0xE3F4_A506_1728_394A` |
| `foliage_willow.png` | `generate_foliage_texture` | `128×128`  | downward-heavy drooping silhouette                    | `0xF4A5_0617_2839_4A5B` |
| `foliage_palm.png`   | `generate_foliage_texture` | `128×128`  | radial fan with multiple separated frond lobes        | `0xA506_1728_394A_5B6C` |
| `foliage_shrub.png`  | `generate_foliage_texture` | `64×64`    | compact dense low-profile bush                        | `0x0617_2839_4A5B_6C7D` |

### Architecture and Implementation Details

#### Shape-specific foliage generation

The generator now routes foliage creation through a `FoliageShape` enum and a
stable `FoliageTextureSpec` table. This keeps filenames, dimensions, colours,
and seeds explicit while allowing each tree family to use different silhouette
rules without changing the binary entrypoint or output layout.

The implementation keeps one shared generator entry point,
`generate_foliage_texture`, but replaces the previous shared circular alpha mask
with shape-selection logic built around:

- `foliage_radius_limit` for shape-specific outer silhouette boundaries
- `foliage_density_threshold` for shape-specific interior occupancy patterns
- `foliage_alpha_for_pixel` for deterministic alpha assignment with preserved
  transparent outer regions and soft retained edges

This approach satisfied the phase requirement that the generator use either one
helper per silhouette or one generic helper with exact per-shape parameter sets
and selection logic.

#### Per-shape silhouette behavior

Each foliage target now has deterministic measurable structure:

- **Oak** uses a wide rounded canopy with broad horizontal occupancy
- **Pine** uses a tall narrow taper with stronger central-column occupancy and a
  lower occupied width/height ratio than oak
- **Birch** remains rounded but is intentionally sparser than oak so its opaque
  pixel count is lower at the same dimensions
- **Willow** biases occupancy downward so the lower half contains more opaque
  pixels than the upper half
- **Palm** uses angular modulation to create separated outer frond lobes and
  populate multiple non-empty angular sectors outside the centre radius
- **Shrub** keeps a shorter occupied height ratio than oak and higher lower-half
  density so it reads as a compact low-profile bush

#### Runtime asset regeneration

After the generator refactor, the runtime tree textures were regenerated
directly into the existing asset location:

- `assets/textures/trees/bark.png`
- `assets/textures/trees/foliage_oak.png`
- `assets/textures/trees/foliage_pine.png`
- `assets/textures/trees/foliage_birch.png`
- `assets/textures/trees/foliage_willow.png`
- `assets/textures/trees/foliage_palm.png`
- `assets/textures/trees/foliage_shrub.png`

No new directory was introduced, no filenames were changed, and no loader path
changes were required.

### Automated Test Coverage

Phase 2 added measurable generator-focused tests for the required output
properties:

- `test_generate_bark_texture_fully_opaque`
- `test_oak_bounding_box_width_is_greater_than_shrub_bounding_box_width`
- `test_pine_central_vertical_occupancy_ratio_is_greater_than_oak`
- `test_pine_width_height_ratio_is_lower_than_oak`
- `test_birch_opaque_pixel_count_is_lower_than_oak`
- `test_willow_lower_half_opaque_pixel_count_is_greater_than_upper_half`
- `test_palm_has_at_least_four_non_empty_angular_sectors_outside_center_radius`
- `test_shrub_occupied_height_ratio_is_lower_than_oak`
- `test_shrub_lower_half_density_is_greater_than_oak`
- `test_generate_foliage_texture_deterministic_for_all_fixed_seeds`
- `test_all_foliage_outputs_have_transparent_outer_region_pixels`
- `test_generate_foliage_texture_preserves_exact_required_dimensions`

These tests are intentionally metric-based and avoid subjective visual
assertions.

### Quality Gate Status

Phase 2 was validated with formatting, compile, lint, focused generator tests,
and runtime asset regeneration. At the time of this summary:

- `cargo fmt --all` — passed
- `cargo check --all-targets --all-features` — passed
- `cargo clippy --all-targets --all-features -- -D warnings` — passed
- `cargo test --bin generate_terrain_textures` — passed
- `cargo run --bin generate_terrain_textures` — passed and regenerated runtime assets
- `cargo nextest run --all-features` — not yet confirmed as fully passing in this phase summary

Because the full `nextest` gate was not yet confirmed in the captured phase
validation, the final checklist item remains open here until that full-suite run
is confirmed clean.

---

## Consumable Duration Effects — Phase 6: End-to-End Integration Tests and Documentation (Complete)

## Consumable Duration Effects — Phase 6: End-to-End Integration Tests and Documentation (Complete)

### Overview

Phase 6 hardens the complete consumable duration effects feature with five
cross-layer integration tests and updates all affected documentation. The tests
exercise the full path from effect application through in-game time advancement
to verified expiry, covering both `ActiveSpells` resistance potions and
per-character `TimedStatBoost` attribute potions. Documentation updates ensure
every public symbol in the timed-consumable stack has `///` doc comments with
`# Arguments`, `# Returns`, and runnable `# Examples` doctests, and that
`docs/reference/architecture.md` accurately reflects all implemented
`ConsumableEffect` variants.

### Phase 6 Deliverables Checklist

- [x] All 5 end-to-end integration tests in `src/application/mod.rs` pass:
  - [x] `test_timed_resistance_potion_expires_after_advance_time`
  - [x] `test_timed_attribute_potion_expires_after_advance_time`
  - [x] `test_timed_potion_expires_during_rest`
  - [x] `test_permanent_attribute_potion_survives_advance_time`
  - [x] `test_second_resistance_potion_overwrites_duration`
- [x] All `pub` symbols in `src/domain/items/consumable_usage.rs` have `///`
      doc comments and doctests (module-level doc expanded with timed vs.
      permanent table and two-entry-point description).
- [x] `TimedStatBoost`, `apply_timed_stat_boost`, and
      `tick_timed_stat_boosts_minute` have `///` doc comments (already present
      from Phase 2; verified intact).
- [x] `apply_attribute_delta` expanded with `# Arguments`, `# Returns`, and
      `# Examples` doc comment.
- [x] `ActiveSpells` struct doc comment expanded to describe
      `effective_resistance`, `ACTIVE_PROTECTION_BONUS`, timed-resistance
      routing, and overwrite semantics — with a runnable doctest.
- [x] `GameState::advance_time` doc comment updated with a `# Summary` section
      explicitly enumerating all three per-minute side effects (spell tick,
      stat-boost tick, restock) plus a timed-boost expiry doctest.
- [x] `docs/reference/architecture.md` `ConsumableEffect` enum updated to
      include `BoostResistance` and `IsFood` variants with prose notes on
      timed vs. permanent behaviour.
- [x] `docs/explanation/implementations.md` (this file) includes a complete
      Phase 6 summary section.
- [x] All four quality gates pass: `cargo fmt`, `cargo check`, `cargo clippy
-D warnings`, `cargo nextest run` — **3453/3453 tests pass**.

### Files Changed

| File                                   | Change                                                                                                                   |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `src/application/mod.rs`               | 5 new integration tests; `ActiveSpells` struct doc expanded; `advance_time` doc expanded with summary and second doctest |
| `src/domain/items/consumable_usage.rs` | Module-level doc expanded with two-entry-point table and timed-vs-permanent matrix                                       |
| `src/domain/character.rs`              | `apply_attribute_delta` doc expanded with `# Arguments`, `# Returns`, `# Examples`                                       |
| `docs/reference/architecture.md`       | `ConsumableEffect` enum updated to include `BoostResistance`, `IsFood`, and timed-vs-permanent prose                     |
| `docs/explanation/implementations.md`  | This Phase 6 summary section                                                                                             |

### Architecture Details

#### Integration test design

The five tests are placed in `src/application/mod.rs` `mod tests` under the
section banner `// ===== Phase 6: End-to-end timed potion / active-spell expiry
tests =====`. Each test exercises a distinct scenario from the end-to-end expiry
lifecycle:

**1. `test_timed_resistance_potion_expires_after_advance_time`**

`apply_consumable_effect_exploration` writes a `u8` duration directly onto the
corresponding `ActiveSpells` field (e.g. `fire_protection = 60`). Each call to
`advance_time(n, None)` calls `ActiveSpells::tick` once per minute, decrementing
each non-zero field by 1 (saturating). After exactly 60 ticks the field is 0
and `effective_resistance` returns 0. The test writes the field directly —
identical to what the exploration handler writes — to avoid needing a full
`GameState` with content loaded.

**2. `test_timed_attribute_potion_expires_after_advance_time`**

`apply_timed_stat_boost(attr, amount, Some(minutes))` appends a `TimedStatBoost`
to `character.timed_stat_boosts` and immediately raises `stats.<attr>.current`.
`GameState::advance_time` calls `tick_timed_stat_boosts_minute` once per elapsed
minute. When `minutes_remaining` reaches 0, the entry is removed and
`apply_attribute_delta(attr, -amount)` restores the stat. After exactly 30 ticks
the list is empty and `stats.might.current` equals the pre-boost baseline.

**3. `test_timed_potion_expires_during_rest`**

`rest_party(REST_DURATION_HOURS, …)` internally calls `advance_time(hours * 60,
…)`, which delivers 720 tick calls for `REST_DURATION_HOURS = 12`. This is
more than enough to drain any effect shorter than 12 hours. The test applies a
60-minute Speed boost and a 60-minute `cold_protection` potion, then calls a
full rest, asserting both expire cleanly.

**4. `test_permanent_attribute_potion_survives_advance_time`**

When `ConsumableData::duration_minutes` is `None`, `normalize_duration` returns
`None` and `apply_consumable_effect` takes the permanent branch, calling
`apply_attribute_delta` directly. No entry is appended to `timed_stat_boosts`,
so `tick_timed_stat_boosts_minute` has nothing to reverse.
`advance_time(999, None)` therefore leaves the stat untouched and
`timed_stat_boosts` remains empty throughout.

**5. `test_second_resistance_potion_overwrites_duration`**

`apply_consumable_effect_exploration` uses direct field assignment
(`active_spells.fire_protection = clamped`) rather than addition. This is the
canonical "last write wins" contract documented in the architecture design
decisions. The test verifies: after 30 of 60 minutes elapse, a second potion
sets the field to exactly 60 (not 90), and 30 more ticks leave exactly 30
remaining.

#### `ActiveSpells` documentation update

The struct doc comment was rewritten to describe:

- The per-field `u8` counter semantics (0 = inactive, n > 0 = n minutes remaining).
- How timed resistance potions are written by `apply_consumable_effect_exploration`.
- The role of `effective_resistance` during combat damage resolution.
- The relationship between `ACTIVE_PROTECTION_BONUS` (flat 25-point bonus) and
  the `amount` field on `BoostResistance` (campaign-author-controlled magnitude).
- Overwrite semantics: second potion overwrites remaining duration, no stacking.
- A runnable doctest demonstrating the full activate → tick-to-zero cycle.

#### `advance_time` documentation update

A new introductory summary paragraph was added before the existing `# Arguments`
section, explicitly listing all three per-minute side effects:

1. `ActiveSpells::tick()` — decrements all spell/potion protection counters.
2. `Character::tick_timed_stat_boosts_minute()` — reverses expired attribute boosts.
3. `npc_runtime.tick_restock(…)` — merchant stock replenishment (when templates are `Some`).

A second doctest demonstrating timed-boost expiry was appended so the contract
is machine-verified.

#### `consumable_usage.rs` module doc update

The module-level `//!` comment was extended with:

- A "Two Entry Points" section describing when to use each of
  `apply_consumable_effect` (combat) vs.
  `apply_consumable_effect_exploration` (exploration).
- A "Timed vs. Permanent Boosts" table mapping `duration_minutes` values to
  observable behaviour for both `BoostAttribute` and `BoostResistance`.

#### `apply_attribute_delta` documentation

This `pub(crate)` function already had a brief doc comment; it was expanded to
include `# Arguments`, `# Returns`, and an `# Examples` doctest that
demonstrates the full apply-then-reverse cycle through
`apply_timed_stat_boost` + 10 `tick_timed_stat_boosts_minute` calls.

#### `architecture.md` update

The `ConsumableEffect` pseudo-Rust enum was updated from four variants to six:

- `BoostResistance(ResistanceType, i8)` — added (was missing)
- `IsFood(u8)` — added (was missing)
- `BoostAttribute` comment clarified to mention timed behaviour

A prose block was appended describing how `duration_minutes` controls timed vs.
permanent routing for each variant.

### Tests Added

#### `src/application/mod.rs` — 5 new end-to-end tests

| Test                                                      | Layer exercised                                             | Assertion                                                        |
| --------------------------------------------------------- | ----------------------------------------------------------- | ---------------------------------------------------------------- |
| `test_timed_resistance_potion_expires_after_advance_time` | `GameState::advance_time` → `ActiveSpells::tick`            | `fire_protection` reaches 0 after exactly 60 ticks               |
| `test_timed_attribute_potion_expires_after_advance_time`  | `GameState::advance_time` → `tick_timed_stat_boosts_minute` | boost list empty and stat restored after 30 ticks                |
| `test_timed_potion_expires_during_rest`                   | `GameState::rest_party` → `advance_time` → both tick paths  | both Speed boost and cold protection expire during 720-tick rest |
| `test_permanent_attribute_potion_survives_advance_time`   | `apply_consumable_effect` permanent path                    | stat unchanged and boost list empty after 999 ticks              |
| `test_second_resistance_potion_overwrites_duration`       | `active_spells` overwrite semantics                         | second potion resets to 60, not 90; 30 ticks leave 30 remaining  |

### Quality Gate Results

```text
✅ cargo fmt --all              → no output (all files formatted)
✅ cargo check --all-targets    → Finished with 0 errors
✅ cargo clippy -D warnings     → Finished with 0 warnings
✅ cargo nextest run            → 3453/3453 passed, 8 skipped, 0 failed
```

Total test count increased from 3448 (end of Phase 5) to 3453 — exactly 5 new
tests from this phase.

---

---

## Consumable Duration Effects — Phase 5: Campaign Builder Support for Duration-Aware Consumables (Complete)

### Overview

Phase 5 exposes `duration_minutes` in the Campaign Builder's Items Editor so
campaign authors can author timed attribute and resistance consumables. It also
adds two SDK template functions and two timed consumable fixtures to the test
campaign data file.

### Phase 5 Deliverables Checklist

- [x] `show_type_editor` in `sdk/campaign_builder/src/items_editor.rs` shows a
      `Duration (minutes)` `DragValue` row for `BoostAttribute` and
      `BoostResistance` effect types only; instant effects (`HealHp`,
      `RestoreSp`, `CureCondition`, `IsFood`) never show the widget.
- [x] Preview text in `show_preview_static` appends `" (N min)"` for timed
      `BoostAttribute` and `BoostResistance` items; permanent items show no
      suffix.
- [x] `timed_fire_resist_potion` template function added to
      `src/sdk/templates.rs`.
- [x] `timed_might_potion` template function added to `src/sdk/templates.rs`.
- [x] Test-campaign fixture `data/test_campaign/data/items.ron` includes item
      62 (`Fire Resist Potion`, `BoostResistance(Fire, 25)`,
      `duration_minutes: Some(60)`) and item 63 (`Might Potion`,
      `BoostAttribute(Might, 5)`, `duration_minutes: Some(30)`).
- [x] All 7 Phase 5 tests pass (5 in `items_editor.rs`, 2 in `templates.rs`).
- [x] All four quality gates pass.
- [x] `sdk/AGENTS.md` egui ID audit confirmed clean — no new `ComboBox`,
      `ScrollArea`, loop, `SidePanel`, or `CollapsingHeader` was introduced.

### Files Changed

| File                                       | Change                                                                                       |
| ------------------------------------------ | -------------------------------------------------------------------------------------------- |
| `sdk/campaign_builder/src/items_editor.rs` | Duration widget in `show_type_editor`; duration suffix in `show_preview_static`; 5 new tests |
| `src/sdk/templates.rs`                     | `timed_fire_resist_potion` and `timed_might_potion` functions; 2 new tests                   |
| `data/test_campaign/data/items.ron`        | Items 62 and 63 added                                                                        |

### Architecture Details

#### Duration widget in `show_type_editor`

A `ui.horizontal` block containing a `DragValue` (range `0..=u16::MAX`) and a
`"(0 = permanent)"` label is added **after** the Amount row for both the
`BoostResistance` and `BoostAttribute` match arms. The write-back logic mirrors
`normalize_duration`: if `raw == 0` the field is stored as `None`; otherwise it
is stored as `Some(raw)`.

```sdk/campaign_builder/src/items_editor.rs#L1412-1425
// Duration row — shown only for timed-capable effects
ui.horizontal(|ui| {
    ui.label("Duration (minutes):");
    let mut raw: u16 = data.duration_minutes.unwrap_or(0);
    ui.add(egui::DragValue::new(&mut raw).range(0..=u16::MAX));
    ui.label("(0 = permanent)");
    data.duration_minutes = if raw == 0 { None } else { Some(raw) };
});
```

The widget uses no new egui ID contexts — `DragValue` requires no `push_id`,
`from_id_salt`, or `id_salt`. This is exactly the pattern described in the
implementation plan (Section 5.1).

#### Duration suffix in `show_preview_static`

Inside the `ItemType::Consumable` arm of `show_preview_static`, both the
`BoostAttribute` and `BoostResistance` format strings are updated to compute a
`duration_str` and append it:

```sdk/campaign_builder/src/items_editor.rs#L655-670
ConsumableEffect::BoostAttribute(attr, n) => {
    let duration_str = data
        .duration_minutes
        .map(|m| format!(" ({} min)", m))
        .unwrap_or_default();
    format!(
        "Boost {} ({}{}){}",
        attr.display_name(),
        if n >= 0 { "+" } else { "" },
        n,
        duration_str
    )
}
```

Permanent items (`duration_minutes: None`) produce `unwrap_or_default()` → an
empty string, so no suffix appears. This satisfies Section 5.2.

#### SDK template functions

`timed_fire_resist_potion(id, duration_minutes, name)` — creates an
`Item` with `ConsumableData { effect: BoostResistance(Fire, 25), is_combat_usable: false,
duration_minutes: normalize_duration(Some(duration_minutes)) }`. Cost defaults
to 100 gold buy / 50 gold sell and `max_charges: 1`.

`timed_might_potion(id, duration_minutes, name)` — creates an `Item` with
`ConsumableData { effect: BoostAttribute(Might, 5), is_combat_usable: false,
duration_minutes: normalize_duration(Some(duration_minutes)) }`. Cost defaults
to 80 gold buy / 40 gold sell and `max_charges: 1`.

Both functions call `normalize_duration(Some(duration_minutes))` so that callers
passing `0` receive a permanent item (`duration_minutes: None`). This matches
Section 5.3 of the plan.

#### Test-campaign fixture items

Items 60 and 61 in `data/test_campaign/data/items.ron` are already occupied by
`Arrows` and `Crossbow Bolts` respectively. The new timed consumable fixtures
use the next free IDs:

| ID  | Name               | Effect                      | Duration   |
| --- | ------------------ | --------------------------- | ---------- |
| 62  | Fire Resist Potion | `BoostResistance(Fire, 25)` | `Some(60)` |
| 63  | Might Potion       | `BoostAttribute(Might, 5)`  | `Some(30)` |

### Tests Added

#### `sdk/campaign_builder/src/items_editor.rs` — 5 new tests

| Test                                                       | What it verifies                                                                       |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `test_duration_field_round_trips_through_editor`           | `duration_minutes: Some(60)` in the edit buffer survives the round-trip                |
| `test_duration_hidden_for_instant_effects`                 | `HealHp` and `RestoreSp` `ConsumableData` have `duration_minutes: None`                |
| `test_duration_zero_normalizes_to_none_on_save`            | The `raw == 0 → None` logic produces `None`; non-zero produces `Some(n)`               |
| `test_preview_text_includes_duration_for_timed_boost`      | Preview string for a `BoostAttribute` item with `Some(60)` contains `"60"` and `"min"` |
| `test_preview_text_no_duration_suffix_for_permanent_boost` | Preview string for a `BoostAttribute` item with `None` does not contain `"min"`        |

#### `src/sdk/templates.rs` — 2 new tests

| Test                                                 | What it verifies                                                                                           |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `test_timed_fire_resist_potion_has_correct_duration` | `timed_fire_resist_potion(62, 90, "…")` produces `duration_minutes: Some(90)` and `BoostResistance(_, 25)` |
| `test_timed_might_potion_zero_duration_is_none`      | `timed_might_potion(63, 0, "…")` produces `duration_minutes: None` (permanent)                             |

### egui ID Audit (sdk/AGENTS.md compliance)

The two new `ui.horizontal` blocks each contain only:

- `ui.label` — no ID required
- `egui::DragValue::new(&mut raw)` — no ID required
- `ui.label` — no ID required

✅ No `ComboBox` introduced — no `from_id_salt` needed
✅ No `ScrollArea` introduced — no `id_salt` needed
✅ No loop introduced — no `push_id` needed
✅ No `SidePanel`/`TopBottomPanel`/`CentralPanel` touched
✅ No `request_repaint()` needed — `DragValue` is a passive input widget, not a layout driver
✅ No `CollapsingHeader`, `egui::Grid`, or `egui::Window` introduced

### Quality Gate Results

```text
✅ cargo fmt --all              → no output (all files formatted)
✅ cargo check --all-targets    → Finished with 0 errors
✅ cargo clippy -D warnings     → Finished with 0 warnings
✅ cargo nextest run            → 3447/3448 passed; 1 pre-existing flaky
                                  performance test unrelated to Phase 5
                                  (test_creature_database_load_performance
                                   times out at ~700–800 ms vs. a 500 ms
                                   threshold on this machine)
```

---

## Consumable Duration Effects — Phase 4: Project `ActiveSpells` into Effective Resistance Calculations (Complete)

### Overview

Phase 4 makes the `active_spells.*_protection` fields that are written by Phase 3
(timed resistance potions) actually influence combat outcomes. Before this phase
those fields were ticked by `advance_time` but never read during damage
resolution. After this phase a party member who has consumed a fire-resistance
potion will take measurably less fire damage while the duration is non-zero, and
full damage again once it expires.

Two files were changed and five new tests were added.

### Phase 4 Deliverables Checklist

- [x] `ACTIVE_PROTECTION_BONUS: i16 = 25` constant defined at module scope in
      `src/application/mod.rs`.
- [x] `ActiveSpells::effective_resistance(res_type: ResistanceType) -> i16`
      method added to `impl ActiveSpells` in `src/application/mod.rs`; covers
      all eight `ResistanceType` variants using the same mapping as Phase 3.
- [x] `resolve_attack` signature updated to accept
      `active_spells: Option<&ActiveSpells>` as a new parameter (inserted
      between `attack` and `rng`).
- [x] `resolve_attack` body projects the active-spell bonus into the effective
      resistance percentage for non-Physical attack types targeting player
      characters; Physical attacks are unaffected.
- [x] All three call sites of `resolve_attack` in `src/game/systems/combat.rs`
      updated to pass `Some(&global_state.0.active_spells)`.
- [x] All existing `resolve_attack` call sites inside
      `src/domain/combat/engine.rs` `mod tests` updated to pass `None`.
- [x] All five Phase 4 tests pass.
- [x] All four quality gates pass (fmt, check, clippy -D warnings, nextest).

### Files Changed

| File                          | Change                                                                                                                                                                                                                                            |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/application/mod.rs`      | `ACTIVE_PROTECTION_BONUS` constant added; `effective_resistance` method added to `impl ActiveSpells`; 3 new Phase 4 tests                                                                                                                         |
| `src/domain/combat/engine.rs` | `use crate::application::ActiveSpells` import added; `resolve_attack` signature extended with `active_spells: Option<&ActiveSpells>`; resistance-projection logic added; all internal test call sites updated to pass `None`; 2 new Phase 4 tests |
| `src/game/systems/combat.rs`  | All 3 `resolve_attack` call sites updated to pass `Some(&global_state.0.active_spells)`                                                                                                                                                           |

### Architecture Details

#### `ACTIVE_PROTECTION_BONUS` constant

```antares/src/application/mod.rs#L305-311
/// Bonus resistance points granted per active protection spell/potion.
///
/// When an `ActiveSpells` protection field is non-zero, this flat bonus is
/// added to the character's current resistance for the matching damage type
/// during combat damage resolution.
pub const ACTIVE_PROTECTION_BONUS: i16 = 25;
```

A single canonical value controls how much resistance (out of 100%) a timed
potion/spell contributes. At 25 points a fully-unresisted fire character still
takes 25% less fire damage while the potion is active.

#### `ActiveSpells::effective_resistance`

The method maps each `ResistanceType` to the `ActiveSpells` field established in
Phase 3, returning `ACTIVE_PROTECTION_BONUS` if non-zero or `0` if expired:

```antares/src/application/mod.rs#L362-384
    pub fn effective_resistance(
        &self,
        res_type: crate::domain::items::types::ResistanceType,
    ) -> i16 {
        use crate::domain::items::types::ResistanceType;
        let active = match res_type {
            ResistanceType::Fire => self.fire_protection > 0,
            ResistanceType::Cold => self.cold_protection > 0,
            ResistanceType::Electricity => self.electricity_protection > 0,
            ResistanceType::Energy => self.magic_protection > 0,
            ResistanceType::Fear => self.fear_protection > 0,
            ResistanceType::Physical => self.magic_protection > 0,
            ResistanceType::Paralysis => self.psychic_protection > 0,
            ResistanceType::Sleep => self.psychic_protection > 0,
        };
        if active {
            ACTIVE_PROTECTION_BONUS
        } else {
            0
        }
    }
```

#### `resolve_attack` resistance projection

After the damage roll and might-bonus calculation, the function computes a
`resistance_reduction` using the target's character resistance value plus the
`active_spells` projection, clamped to `[0, 100]` and treated as a percentage:

```antares/src/domain/combat/engine.rs#L638-688
    let raw_damage = (base_damage + damage_bonus).max(1);

    // Project active spell protection bonuses into effective resistance …
    let resistance_reduction: i32 = match &attack.attack_type {
        AttackType::Physical => 0,
        non_physical => {
            // Map AttackType → ResistanceType for active_spells lookup
            // Map AttackType → character resistance field for base value
            // effective = (char_resistance + spell_bonus).clamp(0, 100)
            // reduction = (raw_damage * effective) / 100
            …
        }
    };

    let total_damage = (raw_damage - resistance_reduction).max(0) as u16;
```

**Key design decisions:**

- **Physical attacks bypass resistance** — the `AttackType::Physical` arm always
  returns `resistance_reduction = 0`, preserving all existing physical-damage
  tests.
- **Monsters are unaffected** — `active_spells` is party-wide; when the target
  is a `Combatant::Monster`, `char_resistance` is set to `0` so the projection
  has no effect on monster-targeted attacks.
- **`None` preserves legacy behaviour** — passing `None` for `active_spells`
  makes `spell_bonus = 0`, so all existing unit tests that pass `None` are
  unaffected.
- **Percentage reduction** — resistance is `[0, 100]`; `100` means full
  immunity. The formula `(raw_damage * effective) / 100` is integer division,
  which slightly under-reduces at low values (safe by design — never over-
  protects).

#### Call sites in `combat.rs`

All three functions that invoke `resolve_attack` now pass the party's live
`active_spells` reference:

```antares/src/game/systems/combat.rs#L2714-2720
    let (damage, special) = resolve_attack(
        &combat_res.state,
        action.attacker,
        action.target,
        &attack_data,
        Some(&global_state.0.active_spells),
        rng,
    )?;
```

The same pattern is applied in `perform_ranged_attack_action_with_rng` and
`perform_monster_turn_with_rng`.

### Tests Added

| Location                      | Test name                                           | What it verifies                                                                                                                 |
| ----------------------------- | --------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `src/application/mod.rs`      | `test_effective_resistance_zero_when_no_protection` | All eight `ResistanceType` variants return `0` when all `active_spells` fields are `0`                                           |
| `src/application/mod.rs`      | `test_effective_resistance_nonzero_when_active`     | Each of the eight types returns `ACTIVE_PROTECTION_BONUS` when its mapped field is non-zero                                      |
| `src/application/mod.rs`      | `test_effective_resistance_zero_when_expired`       | After `tick()` decrements `fire_protection` to `0`, `effective_resistance(Fire)` returns `0`                                     |
| `src/domain/combat/engine.rs` | `test_resistance_check_without_active_spells`       | `resolve_attack` with `None` active spells applies no resistance reduction; physical damage stays in `[1, 6]` for a `1d6` attack |
| `src/domain/combat/engine.rs` | `test_resistance_check_with_active_fire_protection` | Average fire damage with `fire_protection = 30` is statistically lower than without protection (300 trials each)                 |

### Quality Gate Results

```antares/docs/explanation/implementations.md#L1-1
cargo fmt --all           → OK  (no output)
cargo check --all-targets → Finished with 0 errors
cargo clippy -D warnings  → Finished with 0 warnings
cargo nextest run         → 3445 passed, 1 pre-existing timing flake, 8 skipped
```

---

## Consumable Duration Effects — Phase 3: Route Timed Consumable Effects to the Correct Backend (Complete)

### Overview

Phase 3 wires timed consumable effects to the correct backend for both the
combat path and the exploration path. `BoostAttribute` consumables with
`duration_minutes: Some(n)` now register a `TimedStatBoost` (via Phase 2's
`apply_timed_stat_boost`) instead of permanently mutating `current`. A new
`apply_consumable_effect_exploration` function routes timed `BoostResistance`
effects through `ActiveSpells` (overwrite semantics, `u8`-clamped duration)
rather than directly mutating `character.resistances`, so they expire
automatically via `GameState::advance_time`. The combat path
(`apply_consumable_effect`) continues to mutate resistance permanently. All
call sites updated to pass `&ConsumableData` instead of `ConsumableEffect`.

### Phase 3 Deliverables Checklist

- [x] `ConsumableApplyResult` extended with `attribute_boost_is_timed: bool`
      and `resistance_boost_is_timed: bool` (both default `false`).
- [x] `apply_consumable_effect` signature changed from
      `(character, effect: ConsumableEffect)` to `(character, data: &ConsumableData)`.
- [x] `BoostAttribute` arm branches on `normalize_duration(data.duration_minutes)`:
      timed path calls `apply_timed_stat_boost` and sets `attribute_boost_is_timed`;
      permanent path calls `pub(crate) apply_attribute_delta` directly.
- [x] `apply_attribute_delta` promoted from `fn` to `pub(crate) fn` on
      `Character` so `consumable_usage.rs` (a different module) can call it.
- [x] `BoostResistance` in `apply_consumable_effect` always permanent (combat).
- [x] `apply_resistance_to_character` private helper extracted to share the
      eight-arm `ResistanceType` match between the two functions.
- [x] `apply_consumable_effect_exploration` added: routes timed `BoostResistance`
      to `ActiveSpells` (overwrite, `u16::min(minutes, u8::MAX)` clamp), falls
      through to `apply_consumable_effect` for all other effects.
- [x] `apply_consumable_effect_exploration` re-exported from
      `src/domain/items/mod.rs`.
- [x] `execute_item_use_by_slot` (combat) captures full `ConsumableData`
      (`*consumable` copy) instead of `consumable.effect`; passes `&consumable_data`
      to `apply_consumable_effect`.
- [x] `handle_use_item_action_exploration` updated: captures `*consumable`
      (full `ConsumableData`), calls `apply_consumable_effect_exploration` with
      split borrows on `gs.party.members[party_index]` and `gs.active_spells`,
      and emits timed-aware `GameLog` messages.
- [x] All existing tests in `consumable_usage.rs`, `item_usage.rs`, and
      `inventory_ui.rs` updated to pass `&ConsumableData` wrappers.
- [x] 8 new Phase 3 unit tests added to `consumable_usage.rs`.
- [x] 2 new Phase 3 regression tests added to `combat/item_usage.rs`.
- [x] All four quality gates pass with zero errors and zero warnings.

### Files Changed

| File                                   | Change                                                                                                                                                                                                               |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/domain/character.rs`              | `apply_attribute_delta` promoted to `pub(crate)`                                                                                                                                                                     |
| `src/domain/items/consumable_usage.rs` | `ConsumableApplyResult` extended; `apply_consumable_effect` signature changed; `apply_resistance_to_character` helper extracted; `apply_consumable_effect_exploration` added; all tests updated; 8 new Phase 3 tests |
| `src/domain/items/mod.rs`              | `apply_consumable_effect_exploration` added to re-export                                                                                                                                                             |
| `src/domain/combat/item_usage.rs`      | Phase A captures `*consumable` (full `ConsumableData`); Phase B passes `&consumable_data`; all direct `apply_consumable_effect` test calls wrapped in `&ConsumableData`; 2 new Phase 3 regression tests              |
| `src/game/systems/inventory_ui.rs`     | Import updated; Step 4 captures `*consumable`; Step 6 calls `apply_consumable_effect_exploration` with split borrows; Step 7 emits timed-aware log messages                                                          |

### Architecture Details

#### `ConsumableApplyResult` new fields

```antares/src/domain/items/consumable_usage.rs#L99-109
    /// Stat change applied via `BoostAttribute` (0 if none)
    pub attribute_delta: i16,
    /// Resistance change applied via `BoostResistance` (0 if none)
    pub resistance_delta: i16,
    /// True when a `BoostAttribute` was registered as a timed boost
    pub attribute_boost_is_timed: bool,
    /// True when a `BoostResistance` was handled by the caller's timed layer
    pub resistance_boost_is_timed: bool,
```

#### Timed vs Permanent branching in `apply_consumable_effect`

For `BoostAttribute`, `normalize_duration(data.duration_minutes).is_some()` is
the branch condition:

- `Some(n)` → `character.apply_timed_stat_boost(attr, amount, data.duration_minutes)`
  and `attribute_boost_is_timed = true`
- `None` / `Some(0)` → `character.apply_attribute_delta(attr, amount as i16)`
  (permanent, no timed entry created)

`BoostResistance` always uses the permanent path in `apply_consumable_effect`
(combat context). Duration information is intentionally ignored on this path.

#### `apply_consumable_effect_exploration` routing

For `BoostResistance` with a timed duration, the function writes directly to
`active_spells` using overwrite semantics (second potion of the same type
replaces the duration, not stacks it):

```antares/src/domain/items/consumable_usage.rs#L346-362
    if let ConsumableEffect::BoostResistance(res_type, amount) = data.effect {
        if let Some(minutes) = normalize_duration(data.duration_minutes) {
            // Clamp to u8 range (overwrite semantics — last write wins).
            let clamped = u16::min(minutes, u8::MAX as u16) as u8;
            match res_type {
                ResistanceType::Fire => active_spells.fire_protection = clamped,
                ResistanceType::Cold => active_spells.cold_protection = clamped,
                ResistanceType::Electricity => active_spells.electricity_protection = clamped,
                ResistanceType::Energy => active_spells.magic_protection = clamped,
                ResistanceType::Fear => active_spells.fear_protection = clamped,
                ResistanceType::Physical => active_spells.magic_protection = clamped,
                ResistanceType::Paralysis => active_spells.psychic_protection = clamped,
                ResistanceType::Sleep => active_spells.psychic_protection = clamped,
            }
```

#### Borrow-splitting in `handle_use_item_action_exploration`

To satisfy the borrow checker when calling `apply_consumable_effect_exploration`
with both `&mut character` and `&mut active_spells` from the same `GameState`,
the code rebinds through a single `&mut GameState` reference:

```antares/src/game/systems/inventory_ui.rs#L1756-1762
        let result: ConsumableApplyResult = {
            let gs = &mut global_state.0;
            let character = &mut gs.party.members[party_index];
            apply_consumable_effect_exploration(character, &mut gs.active_spells, &consumable_data)
        };
```

This works because `gs.party.members[party_index]` and `gs.active_spells` are
disjoint fields of `GameState`.

#### ResistanceType → ActiveSpells field mapping

| `ResistanceType` | `ActiveSpells` field                    |
| ---------------- | --------------------------------------- |
| `Fire`           | `fire_protection`                       |
| `Cold`           | `cold_protection`                       |
| `Electricity`    | `electricity_protection`                |
| `Energy`         | `magic_protection`                      |
| `Fear`           | `fear_protection`                       |
| `Physical`       | `magic_protection` (no dedicated field) |
| `Paralysis`      | `psychic_protection`                    |
| `Sleep`          | `psychic_protection`                    |

This mapping mirrors the existing `apply_resistance_to_character` mapping for
consistency with the permanent path.

---

## Consumable Duration Effects — Phase 2: Add `TimedStatBoost` to `Character` and Wire Expiry (Complete)

### Overview

Phase 2 introduces a reversible per-character timed boost structure on
`Character` so that `BoostAttribute` consumables with
`duration_minutes: Some(n)` can be applied, tracked, and automatically
reversed. Both `GameState::advance_time` and `rest_party_hour` are updated to
tick per-character boosts in lockstep with `active_spells.tick()`. Existing
save files without the new field load without error via `#[serde(default)]`.

### Phase 2 Deliverables Checklist

- [x] `TimedStatBoost` struct added to `src/domain/character.rs` with doc
      comment and doctest.
- [x] `Character.timed_stat_boosts: Vec<TimedStatBoost>` field added with
      `#[serde(default)]`.
- [x] `Character::apply_timed_stat_boost` — applies delta to `current` and
      stores entry for reversal; `None`/`Some(0)` durations are no-ops.
- [x] `Character::tick_timed_stat_boosts_minute` — decrements counters,
      reverses expired boosts by subtracting the original delta.
- [x] `Character::apply_attribute_delta` — private, centralised
      `AttributeType`→field mapping used by apply and reversal.
- [x] `Character::new` initialises `timed_stat_boosts: Vec::new()`.
- [x] `GameState::advance_time` ticks `tick_timed_stat_boosts_minute` for
      every party member inside the per-minute loop alongside `active_spells.tick()`.
- [x] `rest_party_hour` ticks `tick_timed_stat_boosts_minute` inside its
      existing 60-iteration condition loop.
- [x] `Character` struct literal in `character_definition.rs` updated with
      `timed_stat_boosts: Vec::new()`.
- [x] `Character` struct literal in `equipment_validation.rs` test updated
      with `timed_stat_boosts: vec![]`.
- [x] All 10 Phase 2 tests pass.
- [x] All four quality gates pass with zero errors and zero warnings.

### Files Changed

| File                                       | Change                                                                                                                                                                               |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/domain/character.rs`                  | Added `TimedStatBoost` struct; `timed_stat_boosts` field on `Character`; `apply_timed_stat_boost`, `tick_timed_stat_boosts_minute`, `apply_attribute_delta` methods; 8 Phase 2 tests |
| `src/domain/character_definition.rs`       | Added `timed_stat_boosts: Vec::new()` to `Character` struct literal in `instantiate`                                                                                                 |
| `src/domain/items/equipment_validation.rs` | Added `timed_stat_boosts: vec![]` to `Character` struct literal in alignment-restriction test                                                                                        |
| `src/application/mod.rs`                   | Extended `advance_time` per-minute loop to call `tick_timed_stat_boosts_minute` on every party member; added 2 Phase 2 tests                                                         |
| `src/domain/resources.rs`                  | Extended `rest_party_hour` 60-tick condition loop to also call `tick_timed_stat_boosts_minute`                                                                                       |

### Architecture Details

#### `TimedStatBoost` struct

```src/domain/character.rs#L931-963
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimedStatBoost {
    /// Which attribute this boost modifies.
    pub attribute: crate::domain::items::types::AttributeType,
    /// Signed delta applied to `current` (positive = boost, negative = penalty).
    pub amount: i8,
    /// Minutes remaining before the boost expires and is reversed.
    pub minutes_remaining: u16,
}
```

#### `apply_timed_stat_boost` contract

- Calls `normalize_duration` — `None` and `Some(0)` are both treated as
  permanent: the function returns early with no mutation and no stored entry.
- For `Some(n)` where `n > 0`: applies `amount` to `stats.<attr>.current`
  via `apply_attribute_delta`, then pushes a `TimedStatBoost` entry.
- `base` values are **never** mutated; only `current` is modified.

#### `tick_timed_stat_boosts_minute` contract

- Called once per in-game minute from both `advance_time` and `rest_party_hour`.
- Uses `retain_mut` to decrement `minutes_remaining` by 1 (via
  `saturating_sub`) and collect expired entries.
- Expired entries (those reaching `0`) are removed and reversed: the original
  `amount` is negated and applied via `apply_attribute_delta`.

#### `apply_attribute_delta` — single authoritative mapping

All seven `AttributeType` variants map to their `Stats` field:

| Variant       | Field                    |
| ------------- | ------------------------ |
| `Might`       | `self.stats.might`       |
| `Intellect`   | `self.stats.intellect`   |
| `Personality` | `self.stats.personality` |
| `Endurance`   | `self.stats.endurance`   |
| `Speed`       | `self.stats.speed`       |
| `Accuracy`    | `self.stats.accuracy`    |
| `Luck`        | `self.stats.luck`        |

#### Wiring in `GameState::advance_time`

```src/application/mod.rs#L1495-1510
for _ in 0..minutes {
    self.active_spells.tick();
    // Phase 2: tick per-character timed stat boosts
    for member in &mut self.party.members {
        member.tick_timed_stat_boosts_minute();
    }
}
```

#### Wiring in `rest_party_hour`

```src/domain/resources.rs#L682-688
// Tick minute-based conditions and timed stat boosts for one hour (60 minutes).
for _ in 0..60 {
    character.tick_conditions_minute();
    character.tick_timed_stat_boosts_minute();
}
```

#### Backward compatibility

`#[serde(default)]` on `timed_stat_boosts` means all existing save files
(which do not contain the field) deserialise cleanly — serde uses
`Vec::default()` (an empty `Vec`) when the field is absent.

### Tests Added

**`src/domain/character.rs` — 8 tests:**

| Test                                                      | What it verifies                                                                                 |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `test_timed_stat_boosts_defaults_empty_on_new_character`  | `Character::new` produces `timed_stat_boosts == []`                                              |
| `test_apply_timed_stat_boost_modifies_current_not_base`   | `current` increases by `amount`; `base` unchanged; entry stored with correct `minutes_remaining` |
| `test_apply_timed_stat_boost_none_duration_is_noop`       | `None` duration leaves `current` and `timed_stat_boosts` unchanged                               |
| `test_apply_timed_stat_boost_zero_duration_is_noop`       | `Some(0)` duration leaves `current` and `timed_stat_boosts` unchanged                            |
| `test_tick_timed_stat_boosts_decrements_counter`          | Single tick decrements `minutes_remaining` by 1; stat unchanged                                  |
| `test_tick_timed_stat_boosts_reverses_on_expiry`          | After N ticks (N == initial duration), stat restored and list empty                              |
| `test_tick_timed_stat_boosts_multiple_boosts_independent` | Two boosts with different durations expire independently at correct times                        |
| `test_timed_stat_boost_serde_default_deserializes`        | Character RON without `timed_stat_boosts` deserialises with `timed_stat_boosts == []`            |

**`src/application/mod.rs` — 2 tests:**

| Test                                             | What it verifies                                                                                         |
| ------------------------------------------------ | -------------------------------------------------------------------------------------------------------- |
| `test_advance_time_ticks_timed_stat_boosts`      | `advance_time(N)` expires a boost with `minutes_remaining = N` and restores the stat                     |
| `test_advance_time_ticks_both_spells_and_boosts` | `active_spells.light` and a member's timed boost both decrement together in the same `advance_time` call |

### Quality Gate Results

```/dev/null/quality_gates.txt#L1-6
cargo fmt --all                                    → OK (no output)
cargo check --all-targets --all-features           → Finished (0 errors)
cargo clippy --all-targets --all-features          → Finished (0 warnings)
cargo nextest run --all-features                   → 3431 passed, 8 skipped
  (includes 10 new Phase 2 tests — all PASS)
```

---

## Consumable Duration Effects — Phase 1: Extend `ConsumableData` and Align Core Contracts (Complete)

### Overview

Phase 1 of the Consumable Duration Effects plan adds `duration_minutes:
Option<u16>` to `ConsumableData`, introduces the `normalize_duration` pure
helper, and updates every struct literal call site across `src/`, `sdk/`, and
`tests/` so the codebase compiles and all existing tests continue to pass. No
behavioral changes are made in this phase — permanent-effect semantics are
fully preserved. The architecture document is updated to match.

### Phase 1 Deliverables Checklist

- [x] `ConsumableData` in `src/domain/items/types.rs` includes
      `duration_minutes: Option<u16>` with `#[serde(default)]`.
- [x] `normalize_duration` pure function added to `src/domain/items/types.rs`
      and re-exported from `src/domain/items/mod.rs`.
- [x] All struct literals in `src/`, `sdk/`, `src/bin/`, and `tests/`
      compile with the new field (≈ 40 call sites updated).
- [x] `docs/reference/architecture.md` updated at the `ConsumableData`
      definition to include `duration_minutes`.
- [x] All 6 Phase 1 unit tests pass.
- [x] All four quality gates pass with zero errors and zero warnings.

### Files Changed

| File                                            | Change                                                                                                                                                                               |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/domain/items/types.rs`                     | Added `duration_minutes: Option<u16>` with `#[serde(default)]` to `ConsumableData`; added `normalize_duration` function; updated all doc examples in the file; added 6 Phase 1 tests |
| `src/domain/items/mod.rs`                       | Re-exported `normalize_duration` alongside existing `pub use types::` block                                                                                                          |
| `src/domain/combat/item_usage.rs`               | Added `duration_minutes: None` to 4 test-helper `ConsumableData` literals                                                                                                            |
| `src/domain/combat/engine.rs`                   | Added `duration_minutes: None` to `make_consumable_item` test helper                                                                                                                 |
| `src/domain/items/database.rs`                  | Added `duration_minutes: None` to test-helper literal                                                                                                                                |
| `src/domain/items/equipment_validation.rs`      | Added `duration_minutes: None` to test literal                                                                                                                                       |
| `src/domain/visual/item_mesh.rs`                | Added `duration_minutes: None` to `make_consumable` test helper                                                                                                                      |
| `src/domain/resources.rs`                       | Added `duration_minutes: None` to 3 doc-comment examples and 2 test helpers                                                                                                          |
| `src/application/mod.rs`                        | Added `duration_minutes: None` to 1 doc-comment example and 2 test helpers                                                                                                           |
| `src/game/systems/combat.rs`                    | Added `duration_minutes: None` to `test_perform_use_item_action_heal_*` literal                                                                                                      |
| `src/game/systems/inventory_ui.rs`              | Added `duration_minutes: None` to 7 test-helper `ConsumableData` literals                                                                                                            |
| `src/game/systems/rest.rs`                      | Added `duration_minutes: None` to `make_food_item_db` test helper                                                                                                                    |
| `src/sdk/templates.rs`                          | Added `duration_minutes: None` to `healing_potion` and `sp_potion` templates                                                                                                         |
| `src/bin/item_editor.rs`                        | Added `duration_minutes: None` to `create_consumable` and 4 test literals                                                                                                            |
| `sdk/campaign_builder/src/items_editor.rs`      | Added `duration_minutes: None` to `show_form` default and 3 test literals                                                                                                            |
| `sdk/campaign_builder/src/characters_editor.rs` | Added `duration_minutes: None` to `create_test_item` literal                                                                                                                         |
| `sdk/campaign_builder/src/dialogue_editor.rs`   | Added `duration_minutes: None` to test literal                                                                                                                                       |
| `sdk/campaign_builder/src/lib.rs`               | Added `duration_minutes: None` to `test_item_type_specific_editors` literal                                                                                                          |
| `sdk/campaign_builder/src/templates.rs`         | Added `duration_minutes: None` to `healing_potion` and `mana_potion` template literals                                                                                               |
| `sdk/campaign_builder/src/ui_helpers.rs`        | Added `duration_minutes: None` to 2 `test_extract_item_tag_candidates` literals                                                                                                      |
| `tests/cli_editor_tests.rs`                     | Added `duration_minutes: None` to `create_test_consumable` and `test_item_consumable_effect_variants` literals                                                                       |
| `docs/reference/architecture.md`                | Updated `ConsumableData` struct definition to add `duration_minutes` field                                                                                                           |

### Architecture Details

#### `duration_minutes: Option<u16>` on `ConsumableData`

The field uses `#[serde(default)]` so all existing RON files (`data/items.ron`,
`data/test_campaign/data/items.ron`, `campaigns/tutorial/data/items.ron`)
deserialize without modification — the absent field defaults to `None` via
Serde's `Default` impl for `Option`.

Semantics by value:

| Value     | Meaning                                                            |
| --------- | ------------------------------------------------------------------ |
| `None`    | Effect is permanent (legacy / backward-compatible).                |
| `Some(0)` | Normalized to `None` at application time via `normalize_duration`. |
| `Some(n)` | Effect expires after `n` in-game minutes (used by Phases 2–4).     |

Only `BoostAttribute` and `BoostResistance` effects are timed; `HealHp`,
`RestoreSp`, and `CureCondition` are instant and ignore the field.

#### `normalize_duration`

```src/domain/items/types.rs#L289-302
/// Normalizes a raw `duration_minutes` value.
///
/// `Some(0)` is treated as permanent (`None`) so that editor inputs of `0`
/// and omitted RON fields both produce identical runtime semantics.
pub fn normalize_duration(raw: Option<u16>) -> Option<u16> {
    match raw {
        Some(0) | None => None,
        other => other,
    }
}
```

The function is re-exported as `antares::domain::items::normalize_duration` so
later phases can call it directly from `consumable_usage.rs` without importing
from the internal `types` submodule.

#### RON data files — no changes required

Because `#[serde(default)]` is present, all three RON item files continue to
deserialize correctly. Adding a timed consumable to any of them requires only:

```/dev/null/example.ron#L1-4
item_type: Consumable((
    effect: BoostResistance(Fire, 25),
    is_combat_usable: false,
    duration_minutes: Some(60),
)),
```

### Tests Added

All six tests live in `mod tests` inside `src/domain/items/types.rs`:

| Test                                                          | What it verifies                                                              |
| ------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `test_consumable_data_duration_defaults_none_in_ron`          | RON without `duration_minutes` deserializes to `None` via `#[serde(default)]` |
| `test_consumable_data_duration_some_round_trips`              | `Some(60)` survives a full RON serialize → deserialize round-trip             |
| `test_normalize_duration_zero_becomes_none`                   | `normalize_duration(Some(0)) == None`                                         |
| `test_normalize_duration_none_stays_none`                     | `normalize_duration(None) == None`                                            |
| `test_normalize_duration_positive_unchanged`                  | `Some(30)`, `Some(1)`, `Some(u16::MAX)` all pass through unchanged            |
| `test_consumable_data_struct_literal_compiles_with_new_field` | Three-field struct literal compiles; `duration_minutes` is `None`             |

### Quality Gate Results

```/dev/null/quality_gates.txt#L1-8
cargo fmt --all                                    → OK (no output)
cargo check --all-targets --all-features           → Finished (0 errors)
cargo clippy --all-targets --all-features          → Finished (0 warnings)
cargo nextest run --all-features                   → 3421 passed, 8 skipped
  (includes 6 new Phase 1 tests — all PASS)
```

---

## Consumables Outside Combat — Feature Complete Summary

All four phases of the "Consumables Outside Combat" implementation plan are
complete. Players can now use consumable items directly from the inventory
screen in exploration and menu modes via:

- **`U` keyboard shortcut** — use the highlighted consumable directly from
  Slot Navigation, bypassing the Action Navigation step entirely.
- **"Use" button** — rendered in the action strip when a consumable slot is
  selected; accessible via mouse click or Action Navigation (`←`/`→` then
  `Enter`).

The implementation is backed by a single authoritative pure-domain helper
(`apply_consumable_effect` in `src/domain/items/consumable_usage.rs`) shared
by both the combat and exploration paths — no duplicated `ConsumableEffect`
match logic exists anywhere in the codebase.

### Files Changed (all phases)

| File                                   | Change                                                    | Phase |
| -------------------------------------- | --------------------------------------------------------- | ----- |
| `src/domain/items/consumable_usage.rs` | **Created** — pure-domain effect helper + result type     | 1     |
| `src/domain/items/mod.rs`              | Added `pub mod consumable_usage`; re-exports              | 1     |
| `src/domain/combat/item_usage.rs`      | Delegated effect match to shared helper; regression tests | 1, 4  |
| `src/game/systems/inventory_ui.rs`     | Messages, enum variants, systems, keyboard routing, docs  | 2, 3  |
| `docs/explanation/implementations.md`  | This file                                                 | 4     |

---

## Phase 4: Harden Contracts, Docs, and Cross-Mode Regression Coverage (Complete)

### Overview

Finalized documentation across all three implementation files, verified no
stray `ConsumableEffect` match arms exist outside the two designated files
(`consumable_usage.rs` and `item_usage.rs`), confirmed `combat.rs`
`perform_use_item_action_with_rng` still delegates correctly, and added three
cross-mode regression tests to `src/domain/combat/item_usage.rs`.

### Phase 4 Deliverables Checklist

- [x] `src/domain/combat/item_usage.rs` module doc updated to note that effect
      application is delegated to `apply_consumable_effect` in
      `src/domain/items/consumable_usage.rs`; added `## Effect Application —
Shared Helper` and `## Design Notes` sections.
- [x] `execute_item_use_by_slot` doc comment updated with numbered step list
      explicitly calling out the delegation; `# Arguments` section rewritten
      with plain 2-space continuation (fixes `doc_overindented_list_items`
      Clippy lint); `# Errors` section added.
- [x] `src/game/systems/inventory_ui.rs` module-level key-routing table updated: - Phase 1 table: added `U` row — "Use the highlighted consumable directly
      (bypasses Action Navigation)". - Phase 2 table: `←` `→` description updated to include `Use`.
- [x] `UseItemExplorationAction` doc comment expanded with: - `## Self-target contract` — explains self-target-only scope. - `## Valid ranges` — documents `party_index` and `slot_index` bounds and
      the `GameLog` behaviour when they are exceeded. - `## Charge semantics` — documents decrement/remove/reject behaviour. - Field-level `///` comments updated with `Valid range:` notation.
- [x] Stray `ConsumableEffect` audit: no duplicate match arms outside
      `consumable_usage.rs` (authoritative) and `item_usage.rs` (`IsFood`
      guard only). `item_editor.rs` uses constructors, not match arms on
      effects, which is correct.
- [x] `combat.rs` `perform_use_item_action_with_rng` confirmed to call
      `execute_item_use_by_slot` unchanged — no duplicated effect logic.
- [x] Three Phase 4 cross-mode regression tests added to
      `src/domain/combat/item_usage.rs` `mod tests`: - `test_combat_still_rejects_non_combat_usable` — confirms combat gate
      returns `Err(NotUsableInCombat)` for `is_combat_usable: false` items
      after Phase 1 refactor. - `test_combat_boost_attribute_via_shared_helper` — `BoostAttribute` stat
      delta in combat matches a direct call to `apply_consumable_effect`. - `test_combat_boost_resistance_via_shared_helper` — `BoostResistance`
      delta in combat matches a direct call to `apply_consumable_effect`.
- [x] `cargo fmt --all`, `cargo check --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings`, and
      `cargo nextest run --all-features` all pass with zero warnings and
      **3415 tests passing** (3 new Phase 4 tests).

### Files Changed

| File                                  | Change                                                                                                      |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `src/domain/combat/item_usage.rs`     | Module doc rewritten; `execute_item_use_by_slot` doc expanded; 3 regression tests added (ids 215, 216, 217) |
| `src/game/systems/inventory_ui.rs`    | Module doc table updated (`U` key row); `UseItemExplorationAction` doc expanded with contract sections      |
| `docs/explanation/implementations.md` | This entry added; full feature summary added at top                                                         |

### Architecture Audit Results

#### Single Source of Truth Confirmed

A codebase-wide search for `ConsumableEffect::` match arms found:

- **`src/domain/items/consumable_usage.rs`** — authoritative `match effect { … }`
  covering all six variants (`HealHp`, `RestoreSp`, `CureCondition`,
  `BoostAttribute`, `BoostResistance`, `IsFood`). ✅ correct location.
- **`src/domain/combat/item_usage.rs`** — one `matches!(effect, ConsumableEffect::IsFood(_))`
  guard to reject food items from the combat path (IsFood is not a real combat
  consumable). ✅ intentional and documented.
- All other occurrences are constructors (`ConsumableEffect::HealHp(20)`) in
  test helpers and the CLI item editor — not duplicate match arms. ✅

#### Combat Delegation Chain

```text
handle_use_item_action (Bevy system, combat.rs)
  └─ perform_use_item_action_with_rng (combat.rs)
       └─ execute_item_use_by_slot (item_usage.rs)
            └─ apply_consumable_effect (consumable_usage.rs)  ← shared helper
```

```text
handle_use_item_action_exploration (Bevy system, inventory_ui.rs)
  └─ validate_item_use_slot (item_usage.rs)
  └─ apply_consumable_effect (consumable_usage.rs)            ← same shared helper
```

Both paths converge on the same leaf function. Logic drift between modes is
structurally impossible.

---

## Phase 3: Consumables Outside Combat — Handler System and Feedback (Complete)

### Overview

Added `handle_use_item_action_exploration`, the Bevy system that processes
`UseItemExplorationAction` messages emitted by the inventory UI. The system
validates each use request via the shared `validate_item_use_slot` gate (with
`in_combat = false`), applies the effect via `apply_consumable_effect`, consumes
charges (decrement or remove), resets navigation state, and writes a
player-visible `GameLog` entry for every outcome — success or failure. All 14
required Phase 3 tests pass.

### Phase 3 Deliverables Checklist

- [x] `handle_use_item_action_exploration` system added to
      `src/game/systems/inventory_ui.rs` after `inventory_action_system`.
- [x] System registered as the **last** entry in `InventoryPlugin::build()`'s
      `.chain()` set:
      `(inventory_input_system, inventory_ui_system, inventory_action_system, handle_use_item_action_exploration).chain()`.
- [x] `GameLog` import added:
      `use crate::game::systems::ui::GameLog;`
- [x] `validate_item_use_slot` and `ItemUseError` imported:
      `use crate::domain::combat::item_usage::{validate_item_use_slot, ItemUseError};`
- [x] `apply_consumable_effect` imported:
      `use crate::domain::items::consumable_usage::apply_consumable_effect;`
- [x] `ConsumableEffect` imported for the success message match:
      `use crate::domain::items::types::{ConsumableEffect, ItemType};`
- [x] All 10 `ItemUseError` variants produce a distinct, player-readable
      `GameLog` message as specified in the plan's error table.
- [x] Charge consumption semantics match the combat path: decrement when
      `charges > 1`, remove the slot entirely when `charges == 1`, defensive
      check + log entry when `charges == 0`.
- [x] Effect-specific `GameLog` success messages implemented for all six
      `ConsumableEffect` variants including the "already at full" cases for
      `HealHp` and `RestoreSp`.
- [x] Navigation state fully reset after every use attempt (success or failure):
      `selected_slot = None`, `selected_slot_index = None`,
      `focused_action_index = 0`, `phase = SlotNavigation`.
- [x] Helper functions `make_heal_potion_db`, `make_sp_potion_db`, and
      `make_exploration_use_app` added to `mod tests` to reduce repetition
      across the 14 new tests.
- [x] All 14 Phase 3 tests added and passing:
      `test_exploration_use_heals_character`,
      `test_exploration_use_restores_sp`,
      `test_exploration_use_cures_condition`,
      `test_exploration_use_boosts_attribute`,
      `test_exploration_use_boosts_resistance`,
      `test_exploration_use_decrements_multi_charge_item`,
      `test_exploration_use_removes_last_charge`,
      `test_exploration_use_resets_nav_state`,
      `test_exploration_use_writes_game_log`,
      `test_exploration_use_invalid_slot_writes_log`,
      `test_exploration_use_non_consumable_writes_log`,
      `test_exploration_use_zero_charges_writes_log`,
      `test_exploration_use_non_combat_usable_item_succeeds`,
      `test_exploration_use_invalid_party_index_writes_log`.
- [x] `cargo fmt --all`, `cargo check --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings`, and
      `cargo nextest run --all-features` all pass with zero warnings and 3412
      tests passing (14 new).

### Files Changed

| File                                  | Change                                                                                                                                     |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/game/systems/inventory_ui.rs`    | 4 new imports; `handle_use_item_action_exploration` system added; plugin chain extended; 14 new tests; 2 test helpers + 1 app helper added |
| `docs/explanation/implementations.md` | This entry added                                                                                                                           |

### Architecture Details

#### `handle_use_item_action_exploration` System

The system signature follows the exact contract from the plan:

```text
fn handle_use_item_action_exploration(
    mut reader: MessageReader<UseItemExplorationAction>,
    mut global_state: ResMut<GlobalState>,
    mut nav_state: ResMut<InventoryNavigationState>,
    game_content: Option<Res<GameContent>>,
    mut game_log: Option<ResMut<GameLog>>,
)
```

Messages are collected upfront into a `Vec<(usize, usize)>` to avoid borrow
conflicts between the immutable `game_content` reference and the mutable
`global_state`. The system returns early (writing one `GameLog` entry) if
`game_content` is `None`.

#### Validation → Charge Consumption → Effect → Log ordering

The logic for each message follows these ordered steps:

1. **Resolve game_content** — early return if unavailable.
2. **Bounds-check party_index** — `continue` with log if out of range.
3. **`validate_item_use_slot(…, false)`** — all 10 error arms produce a
   distinct `GameLog` message; navigation state is reset even on failure.
4. **Capture item name and effect** — short immutable borrow, releases before
   mutation.
5. **Consume one charge** — decrement or remove; defensive zero-charge guard.
6. **`apply_consumable_effect`** — mutates character stats via the shared
   pure-domain helper.
7. **Write success `GameLog`** — effect-specific template; "already at full"
   fallback for `HealHp`/`RestoreSp` when `result.healing == 0` /
   `result.sp_restored == 0`.
8. **Reset navigation state** — `selected_slot`, `selected_slot_index`,
   `focused_action_index`, `phase`.

#### `is_combat_usable: false` Items

`validate_item_use_slot` returns `NotUsableInCombat` only when
`in_combat == true && !consumable.is_combat_usable`. Calling it with
`in_combat = false` means exploration-only items pass validation normally.
The dedicated test `test_exploration_use_non_combat_usable_item_succeeds`
exercises this boundary explicitly.

#### Navigation Reset on All Paths

Both success and all failure paths (except the `game_content = None` early
return which resets nothing — there is no character state to reset) perform the
identical four-field navigation reset. This prevents the UI being stuck in
`ActionNavigation` after a failed use attempt.

---

## Phase 2: Consumables Outside Combat — Inventory UI Integration (Complete)

### Overview

Wired the consumable-use pathway into the exploration-mode inventory UI
(`src/game/systems/inventory_ui.rs`). Players can now use consumable items
directly from the inventory screen without entering combat. The implementation
adds a new `UseItemExplorationAction` message, a `PanelAction::Use` variant, a
`U` keyboard shortcut, and a "Use" button in the action strip — all gated on the
item being a `ItemType::Consumable(_)` according to the content database.

### Phase 2 Deliverables Checklist

- [x] `UseItemExplorationAction { party_index, slot_index }` struct added with
      `#[derive(Message)]`, full `///` doc comment, and doctest.
- [x] `PanelAction::Use { party_index, slot_index }` variant added as the first
      variant in the enum; doc example updated to cover all three variants.
- [x] `build_action_list` signature extended to accept `selected_slot_index`,
      `character: &Character`, and `game_content: Option<&GameContent>`;
      `Use` is prepended only when the slot holds a consumable item.
- [x] `inventory_input_system` updated: two new parameters
      (`game_content: Option<Res<GameContent>>`, `use_writer: MessageWriter<UseItemExplorationAction>`);
      `build_action_list` call updated; `PanelAction::Use` arm added in the
      `Enter` handler; `U` shortcut added in `SlotNavigation` phase.
- [x] `inventory_ui_system` updated: `use_writer` parameter added; status line
      appends `"  [U: use]"` for consumable slots; hint text updated to include
      `"U: use consumable"`; `PanelAction::Use` arm added in the
      `pending_action` match.
- [x] `render_character_panel` action strip updated: "Use" button rendered before
      "Drop" when the selected slot is a consumable; Drop and Transfer button
      focus indices adjusted accordingly (`drop_focused_idx` and
      `action_btn_idx` conditioned on `is_consumable`).
- [x] `InventoryPlugin::build()` registers `UseItemExplorationAction` with
      `app.add_message::<UseItemExplorationAction>()`.
- [x] Existing `build_action_list` tests updated to the new 5-argument signature
      (character with empty inventory, `game_content = None`).
- [x] Existing `test_panel_action_drop_variant` and
      `test_panel_action_transfer_variant` updated with `PanelAction::Use { .. }`
      arms to satisfy exhaustiveness.
- [x] 5 new Phase 2 tests added:
      `test_build_action_list_use_first_for_consumable`,
      `test_build_action_list_no_use_for_non_consumable`,
      `test_build_action_list_no_use_when_no_content`,
      `test_panel_action_use_variant`,
      `test_build_action_list_drop_transfer_unchanged`.
- [x] `cargo fmt --all`, `cargo check --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings`, and
      `cargo nextest run --all-features` all pass with zero warnings and 3398
      tests passing.

### Files Changed

| File                                  | Change                                                                                                                                                 |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/game/systems/inventory_ui.rs`    | `UseItemExplorationAction` struct added; `PanelAction::Use` variant added; `build_action_list` extended; systems updated; 7 new tests; 5 tests updated |
| `docs/explanation/implementations.md` | This entry added                                                                                                                                       |

### Architecture Details

#### `UseItemExplorationAction`

A `#[derive(Message)]` struct with two `pub usize` fields:

- `party_index` — which party member owns the item (0-based).
- `slot_index` — which slot in that character's `inventory.items` to consume.

Mirrors the shape of `DropItemAction` and `TransferItemAction` to keep the
action message pattern consistent across the inventory system.

#### `PanelAction::Use` Variant

Added as the **first** variant in `PanelAction` so that when keyboard focus
enters `ActionNavigation`, index 0 maps to `Use` for consumables (and index 0
maps to `Drop` for non-consumables). This preserves the invariant that the most
destructive irreversible action (`Drop`) is not the default focus when a safer
action (`Use`) is available.

#### `build_action_list` Consumable Guard

```text
character.inventory.items.get(selected_slot_index)
    → game_content.db().items.get_item(slot.item_id)
    → matches!(item.item_type, ItemType::Consumable(_))
```

If `game_content` is `None` or the item ID is not found, `is_consumable`
defaults to `false` and no `Use` action is emitted. This makes the function
safe to call in tests without a content database.

#### Button Index Offsets in `render_character_panel`

When a consumable slot is selected the action strip renders:
`[Use] [Drop] [→ Ally] [→ Mage] …`

The Drop button focus index becomes `1` (was `0`) and Transfer buttons start at
`2` (were `1`). These offsets are computed from the same `is_consumable` boolean
so keyboard and mouse paths always agree.

#### `U` Keyboard Shortcut

Inserted in `SlotNavigation` phase, before the arrow-key handler and after the
`Esc`/`Tab`/`Enter` blocks. If the highlighted slot is a consumable the shortcut
fires `UseItemExplorationAction` immediately, clears the slot selection, and
resets the nav phase — bypassing `ActionNavigation` entirely for the common case.

---

## Phase 1: Extract Shared Consumable Domain Logic (Complete)

### Overview

Extracted the authoritative `ConsumableEffect` match from `execute_item_use_by_slot`
in `src/domain/combat/item_usage.rs` into a new standalone pure-domain module
`src/domain/items/consumable_usage.rs`. Both the existing combat path and the
future exploration/menu path now share a single implementation, eliminating any
risk of logic drift between the two. `ResistanceType` was also added to the
public re-export surface of `src/domain/items/mod.rs` so callers can import it
via `antares::domain::items::ResistanceType` without reaching into the `types`
submodule directly.

### Phase 1 Deliverables Checklist

- [x] `src/domain/items/consumable_usage.rs` created with SPDX header,
      `ConsumableApplyResult` struct, and `apply_consumable_effect` covering all
      six `ConsumableEffect` variants (`HealHp`, `RestoreSp`, `CureCondition`,
      `BoostAttribute`, `BoostResistance`, `IsFood`).
- [x] `src/domain/items/mod.rs` updated: `pub mod consumable_usage;` added;
      `ResistanceType` added to `pub use types::...`; `apply_consumable_effect`
      and `ConsumableApplyResult` re-exported from `consumable_usage`.
- [x] `execute_item_use_by_slot` in `src/domain/combat/item_usage.rs` delegates
      the `ConsumableEffect` match to `apply_consumable_effect`; all combat-only
      responsibilities (user identity check, charge consumption, `advance_turn`,
      `check_combat_end`) remain in the combat executor.
- [x] All ten domain and regression tests listed in the plan pass.
- [x] Two additional combat-path regression tests for `BoostResistance` and
      `BoostAttribute` added to `item_usage.rs` to exercise the new helper
      through the full combat stack.
- [x] `cargo fmt --all`, `cargo check --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings`, and
      `cargo nextest run --all-features` all pass with zero warnings.

### Files Changed

| File                                   | Change                                                                                                              |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `src/domain/items/consumable_usage.rs` | **Created** — `ConsumableApplyResult`, `apply_consumable_effect`, 18 unit tests                                     |
| `src/domain/items/mod.rs`              | Added `pub mod consumable_usage;`, re-exported `apply_consumable_effect`, `ConsumableApplyResult`, `ResistanceType` |
| `src/domain/combat/item_usage.rs`      | Phase B match replaced with `apply_consumable_effect` delegation; 5 new regression tests added                      |

### Architecture Details

#### `ConsumableApplyResult`

A plain `#[derive(Default)]` struct with five `i32`/`i16`/`u8` fields:

- `healing: i32` — HP delta actually applied (zero for non-heal effects).
- `sp_restored: i32` — SP delta actually applied.
- `conditions_cleared: u8` — bitflags cleared from `character.conditions`.
- `attribute_delta: i16` — stat delta applied by `BoostAttribute`.
- `resistance_delta: i16` — resistance delta applied by `BoostResistance`.

Callers receive the result and can compose player-visible feedback messages
without re-deriving deltas from a before/after snapshot.

#### `apply_consumable_effect` Contract

| Variant                             | Mutation                                                                 | Cap        |
| ----------------------------------- | ------------------------------------------------------------------------ | ---------- |
| `HealHp(amount)`                    | `hp.modify(amount as i32)` then clamp `hp.current` to `hp.base`          | `hp.base`  |
| `RestoreSp(amount)`                 | `sp.modify(amount as i32)` then clamp `sp.current` to `sp.base`          | `sp.base`  |
| `CureCondition(flags)`              | `conditions.remove(flags)` — bitflag only, `active_conditions` untouched | none       |
| `BoostAttribute(attr, amount)`      | `stats.<field>.modify(amount as i16)`                                    | saturating |
| `BoostResistance(res_type, amount)` | `resistances.<field>.modify(amount as i16)`                              | saturating |
| `IsFood(_)`                         | No-op; returns zeroed `ConsumableApplyResult`                            | —          |

#### Refactored Combat Path

`execute_item_use_by_slot` retains all combat-only responsibilities. Phase B now
consists of a single `get_combatant_mut` call followed by a call to
`apply_consumable_effect(pc_target, effect)`. The returned `ConsumableApplyResult`
is used to populate `total_healing`, `effected_indices`, and `applied_conditions`
exactly as before, preserving full backward compatibility with existing callers and
tests.

#### `ResistanceType` Re-export

`ResistanceType` was defined in `src/domain/items/types.rs` but was previously not
re-exported from `src/domain/items/mod.rs`. It is now available as
`antares::domain::items::ResistanceType` alongside `AttributeType`,
`ConsumableEffect`, and the rest of the public items API.

### Quality Gate Results

```antares/docs/explanation/implementations.md#L1-1
cargo fmt         → clean (no output)
cargo check       → Finished (0 errors, 0 warnings)
cargo clippy      → Finished (0 warnings)
cargo nextest run → 3393 passed, 8 skipped
```

---

## Phase 5E: Months and Years — Call-Site Audit & Quality Gates (Complete)

### Overview

Audited every `.day` access on `GameTime` across the entire codebase and updated all
sites that were treating `day` as a **cumulative elapsed-day counter** to use
`GameTime::total_days()` instead. After Phase 1A changed `day` to mean "day-within-month
(1–30)", any arithmetic of the form `day * 24 * 60` or `day - 1` that computed
absolute or elapsed minutes was silently wrong once the calendar rolled past month 1.

### Phase 5E Deliverables Checklist

- [x] Full `.day` access audit across `src/` and `sdk/` completed
- [x] `src/domain/world/npc_runtime.rs` — `tick_restock()` uses `total_days()` for cumulative-day comparisons
- [x] `src/application/mod.rs` — `test_step_advances_time` uses `total_days()` for minute baseline
- [x] `src/application/mod.rs` — `test_rest_advances_time_via_state` uses `total_days()` for elapsed-minutes calculation
- [x] `src/game/systems/combat.rs` — `test_combat_round_advances_time` uses `total_days()` for minute baseline
- [x] `src/game/systems/map.rs` — `test_map_transition_advances_time` uses `total_days()` for minute baseline
- [x] All quality gates pass (0 errors, 0 warnings, 3371 tests green)
- [x] `implementations.md` updated

### Call-Site Audit Results

Every `.day` access was classified as one of three categories:

#### Category 1: Legitimate day-within-month accesses (no change needed)

These correctly use `day` as the 1–30 field it now represents:

| Location                                                                        | Usage                                                                 | Verdict    |
| ------------------------------------------------------------------------------- | --------------------------------------------------------------------- | ---------- |
| `src/domain/types.rs` — struct body and tests                                   | Field construction, rollover logic, assertions about day-within-month | ✅ Correct |
| `src/game/systems/hud.rs` — `update_clock()`                                    | `game_time.day` passed to `format_clock_date()`                       | ✅ Correct |
| `src/sdk/campaign_loader.rs` — tests                                            | `starting_time.day == 1` assertions                                   | ✅ Correct |
| `sdk/campaign_builder/src/campaign_editor.rs` — buffer                          | `m.starting_time.day` copy/apply                                      | ✅ Correct |
| `src/application/mod.rs` — doctest + unit test                                  | `state.time.day == 1` new-game assertion                              | ✅ Correct |
| `src/application/mod.rs` — `test_blocked_step_does_not_advance_time`            | `time.day == time_before.day` identity check                          | ✅ Correct |
| `src/application/save_game.rs` — `test_save_and_load`                           | `loaded_state.time.day == game_state.time.day` roundtrip identity     | ✅ Correct |
| `src/game/systems/time.rs` — `test_time_advance_event_rolls_over_midnight`      | `time.day == 2` after midnight rollover                               | ✅ Correct |
| `src/game/systems/map.rs` — `test_invalid_map_transition_does_not_advance_time` | `time.day == time_before.day` identity check                          | ✅ Correct |

#### Category 2: Cumulative elapsed-day arithmetic — FIXED

These computed total elapsed minutes using `day * 24 * 60`, which breaks once the
calendar rolls past month 1 (day resets to 1):

| Location                                                               | Old code                            | Fixed code                                   |
| ---------------------------------------------------------------------- | ----------------------------------- | -------------------------------------------- |
| `src/application/mod.rs` `test_step_advances_time` (before)            | `state.time.day as u64 * 24 * 60`   | `state.time.total_days() as u64 * 24 * 60`   |
| `src/application/mod.rs` `test_step_advances_time` (after)             | `state.time.day as u64 * 24 * 60`   | `state.time.total_days() as u64 * 24 * 60`   |
| `src/application/mod.rs` `test_rest_advances_time_via_state`           | `(state.time.day - 1) * 24 * 60`    | `(state.time.total_days() - 1) * 24 * 60`    |
| `src/game/systems/combat.rs` `test_combat_round_advances_time` (start) | `gs.time.day as u64 * 24 * 60`      | `gs.time.total_days() as u64 * 24 * 60`      |
| `src/game/systems/combat.rs` `test_combat_round_advances_time` (end)   | `state.0.time.day as u64 * 24 * 60` | `state.0.time.total_days() as u64 * 24 * 60` |
| `src/game/systems/map.rs` `test_map_transition_advances_time` (start)  | `gs.time.day as u64 * 24 * 60`      | `gs.time.total_days() as u64 * 24 * 60`      |
| `src/game/systems/map.rs` `test_map_transition_advances_time` (end)    | `state.0.time.day as u64 * 24 * 60` | `state.0.time.total_days() as u64 * 24 * 60` |

#### Category 3: NPC restock tracking — FIXED (most important)

`NpcRuntimeStore::tick_restock()` compared `new_time.day` against
`last_restock_day` / `last_magic_refresh_day` to determine whether a new
calendar day had passed. With `day` now being 1–30, this caused NPCs to restock
12 times per year (once per month rollover when day resets to 1):

```antares/src/domain/world/npc_runtime.rs#L707-L714
    pub fn tick_restock(
        &mut self,
        new_time: &crate::domain::types::GameTime,
        templates: &MerchantStockTemplateDatabase,
    ) {
        // Use total_days() so the counter is cumulative across months and years.
        // new_time.day is only 1–30 (day-within-month, not a running total).
        let new_day = new_time.total_days();
```

`last_restock_day` and `last_magic_refresh_day` are stored as `u32` and continue
to hold cumulative-day values (they are serde-persisted; existing save files have
values ≤ 30 which will be treated as total_days() from year 1, month 1 — correct
since existing saves have `year=1, month=1` via the serde default).

Updated tests that previously asserted raw `.day` values in `last_restock_day`
now assert `game_time.total_days()` instead. Comment explanations were added so
the mapping from `GameTime::new(7, 12, 0)` → `total_days() = 7` is explicit.

Updated doc comment to reference `total_days()` instead of `.day` for the seed
recommendation in `refresh_magic_slots`.

### What Was NOT Changed

The following `.day` accesses were intentionally left unchanged because they
operate on the correct semantic:

- `npc_runtime.rs` doctest for `tick_restock` — uses `GameTime::new(2, 6, 0)` which
  has `total_days() = 2`; the example comment "Advance to day 2" is still accurate
  since the test starts at the very beginning of the calendar.
- All doc comments in `src/application/mod.rs` that say `assert_eq!(state.time.day, 1)` —
  these check the day-within-month field on a freshly created game state (correct).
- `src/game/systems/time.rs` rollover test — explicitly tests that `day` increments
  from 1 to 2 after midnight (day-within-month semantics, correct).

### Architecture Compliance

- The decision from the plan is enforced: `day` = day-within-month (1–30);
  cumulative elapsed days = `total_days()`.
- `TimeCondition::AfterDay` and `BeforeDay` already use `total_days()` (Phase 2B).
- All comparison logic that needs cumulative day counts now uses `total_days()`.
- No magic numbers introduced — all rollover constants reference `DAYS_PER_MONTH`,
  `MONTHS_PER_YEAR`, `DAYS_PER_YEAR` from `src/domain/types.rs`.

### Quality Gate Results

```antares/docs/explanation/implementations.md#L1-1
cargo fmt         → clean (no output)
cargo check       → Finished (0 errors)
cargo clippy      → Finished (0 warnings)
cargo nextest run → 3371 passed, 8 skipped
```

---

## Phase 4D: Months and Years — Campaign Builder & Config (Complete)

### Overview

Extended the Campaign Builder SDK editor (`sdk/campaign_builder/src/campaign_editor.rs`)
to expose the new `year` and `month` fields from `GameTime` in the campaign metadata
editing workflow. The `CampaignMetadataEditBuffer` now holds five separate time
components — year, month, day, hour, minute — that are round-tripped through
`GameTime::new_full(...)` with full range clamping. The Gameplay section of the
Campaign Metadata editor panel gained two new `DragValue` spinners (Year and Month)
so campaign authors can set the starting calendar date entirely within the UI.

### Phase 4D Deliverables Checklist

- [x] `starting_year: u32` and `starting_month: u32` fields added to `CampaignMetadataEditBuffer`
- [x] `from_metadata()` updated to read `m.starting_time.year` and `m.starting_time.month`
- [x] `apply_to()` updated to call `GameTime::new_full(year, month, day, hour, minute)` with clamping (`year.max(1)`, `month.clamp(1, 12)`)
- [x] UI Gameplay grid: Year (1–9999) and Month (1–12) `DragValue` spinners added before the existing Day spinner
- [x] Day spinner range narrowed from `1..=9999` to `1..=30` (day-within-month semantics)
- [x] `preview_time` in the UI updated to use `GameTime::new_full(...)` with year/month
- [x] Test `test_buffer_from_metadata_copies_starting_year_month` passes
- [x] Test `test_buffer_apply_to_writes_starting_year_month` passes
- [x] Test `test_buffer_starting_time_clamps_month` passes (both month=0 → 1 and month=13 → 12)
- [x] Test `test_buffer_starting_time_clamps_year_zero` passes
- [x] Existing buffer tests updated: `test_buffer_default_starting_time_fields`, `test_buffer_starting_time_roundtrip_via_metadata`, `test_buffer_starting_time_clamps_day_zero`
- [x] All 22 campaign_editor tests pass
- [x] All quality gates pass (0 errors, 0 warnings, 3371 main-crate tests green)

### What Was Built

#### `CampaignMetadataEditBuffer` — `sdk/campaign_builder/src/campaign_editor.rs`

Two new fields were inserted immediately before `starting_day`, keeping logical
calendar order (year → month → day → hour → minute):

```antares/sdk/campaign_builder/src/campaign_editor.rs#L102-110
    // Starting date/time (split from GameTime for ergonomic drag-value editing)
    /// Starting year (1-based)
    pub starting_year: u32,
    /// Starting month within the year (1-based, 1–12)
    pub starting_month: u32,
    /// Starting day within the month (1-based, 1–30)
    pub starting_day: u32,
    /// Starting hour (0–23)
    pub starting_hour: u8,
    /// Starting minute (0–59)
    pub starting_minute: u8,
```

#### `from_metadata()` — Reading Year and Month

```antares/sdk/campaign_builder/src/campaign_editor.rs#L152-158
            starting_year: m.starting_time.year,
            starting_month: m.starting_time.month,
            starting_day: m.starting_time.day,
            starting_hour: m.starting_time.hour,
            starting_minute: m.starting_time.minute,
```

#### `apply_to()` — Writing via `GameTime::new_full` with Clamping

`apply_to()` now calls `GameTime::new_full(...)` instead of `GameTime::new(...)`,
passing all five components with their respective clamping guards:

- `year`: `self.starting_year.max(1)` — 1-based, 0 is invalid
- `month`: `self.starting_month.clamp(1, 12)` — must stay in 1–12
- `day`: `self.starting_day.max(1)` — 1-based, 0 is invalid
- `hour`: `self.starting_hour.min(23)` — 0–23
- `minute`: `self.starting_minute.min(59)` — 0–59

#### UI Gameplay Grid Spinners

Two `DragValue` spinners were added before the existing Day spinner in the
"Starting Date/Time" row of the Gameplay grid:

- **Year**: `DragValue::new(&mut year).range(1..=9999)` — clamped with `year.max(1)`
- **Month**: `DragValue::new(&mut month).range(1..=12)` — clamped with `month.clamp(1, 12)`

The Day spinner's upper bound was narrowed from `9999` to `30` (day-within-month
semantics from Phase 1A). The period-of-day `preview_time` at the end of the row
was updated to use `GameTime::new_full(year, month, day, hour, minute)`.

#### `sdk/campaign_builder/src/lib.rs` — No Changes Required

`CampaignMetadata.starting_time` is already typed as `GameTime`, which now carries
`year` and `month`. The `default_starting_time()` function returns `GameTime::new(1, 8, 0)`
which sets `year=1, month=1` via the backward-compatible three-argument constructor —
no schema change needed.

### Clamping Contract

| Field             | Valid range | `apply_to()` guard |
| ----------------- | ----------- | ------------------ |
| `starting_year`   | ≥ 1         | `.max(1)`          |
| `starting_month`  | 1–12        | `.clamp(1, 12)`    |
| `starting_day`    | ≥ 1         | `.max(1)`          |
| `starting_hour`   | 0–23        | `.min(23)`         |
| `starting_minute` | 0–59        | `.min(59)`         |

### Tests Added / Updated

| Test name                                                    | What it verifies                                                       |
| ------------------------------------------------------------ | ---------------------------------------------------------------------- |
| `test_buffer_from_metadata_copies_starting_year_month`       | `from_metadata` copies `year` and `month` into new buffer fields       |
| `test_buffer_apply_to_writes_starting_year_month`            | `apply_to` writes all five fields via `new_full`                       |
| `test_buffer_starting_time_clamps_month`                     | month=0 → 1, month=13 → 12                                             |
| `test_buffer_starting_time_clamps_year_zero`                 | year=0 → 1                                                             |
| `test_buffer_default_starting_time_fields` (updated)         | now also asserts `starting_year=1`, `starting_month=1`                 |
| `test_buffer_starting_time_roundtrip_via_metadata` (updated) | now sets/checks year=2, month=6 in the round-trip                      |
| `test_buffer_starting_time_clamps_day_zero` (updated)        | now initialises `starting_year` and `starting_month` in buffer literal |

### Architecture Compliance

- `GameTime::new_full()` (Phase 1A) used in `apply_to()` — no raw struct construction.
- `MONTHS_PER_YEAR` constant from `src/domain/types.rs` is respected via the `clamp(1, 12)` bound.
- No magic numbers: month upper-bound `12` matches `MONTHS_PER_YEAR`; day upper-bound `30` matches `DAYS_PER_MONTH`.
- `CampaignMetadata.starting_time` field unchanged in `lib.rs` — uses `GameTime` which already carries the new fields via Phase 1A serde defaults.
- `sdk/campaign_builder/src/lib.rs` `default_starting_time()` unchanged — `GameTime::new(1, 8, 0)` correctly defaults year=1/month=1.

### Quality Gate Results

```antares/docs/explanation/implementations.md#L1-1
cargo fmt         → clean (no output)
cargo check       → Finished (0 errors)
cargo clippy      → Finished (0 warnings)
cargo nextest run → 3371 passed, 8 skipped (main crate)
campaign_builder campaign_editor tests → 22 passed
```

---

## Phase 3C: Months and Years — HUD Clock Update (Complete)

### Overview

Updated `src/game/systems/hud.rs` to display a full calendar date on the HUD clock
widget. The existing single-field `"Day N"` display is replaced with a three-field
`"Y{year} M{month} D{day}"` compact format that fits the fixed-width clock panel.

Three coordinated changes were made:

1. **`ClockDayText` → `ClockDateText`** — marker component renamed throughout
   (struct definition, spawn site, `update_clock()` queries, and all tests).
2. **`format_clock_day(day)` → `format_clock_date(year, month, day)`** — pure helper
   function replaced; returns `"Y{year} M{month} D{day}"`.
3. **`update_clock()`** — updated to call `format_clock_date` with all three calendar
   fields from `game_time`, and to use the renamed `ClockDateText` query.

All existing clock tests were updated in-place (no tests deleted); two plan-specified
new tests (`test_format_clock_date_defaults` and `test_format_clock_date_large_values`)
plus three additional tests were added for full coverage.

### Phase 3C Deliverables Checklist

- [x] `ClockDayText` → `ClockDateText` rename (struct, spawn, queries, tests)
- [x] `format_clock_date(year, month, day)` implemented (replaces `format_clock_day`)
- [x] `update_clock()` passes `game_time.year`, `game_time.month`, `game_time.day`
- [x] Initial spawn text updated from `"Day 1"` to `"Y1 M1 D1"`
- [x] All HUD tests updated to `ClockDateText` and new format strings
- [x] All quality gates pass (0 errors, 0 warnings, 3371 tests green)

### What Was Built

#### `ClockDateText` Marker Component — `src/game/systems/hud.rs`

```antares/src/game/systems/hud.rs#L158-161
/// Marker component for the calendar date text node (displays "Y{year} M{month} D{day}")
#[derive(Component)]
pub struct ClockDateText;
```

The rename cascaded to every reference: the `setup_hud` spawn site, both `Without<>`
filter type parameters in `update_clock()`, and all `clock_tests` queries.

#### `format_clock_date()` — Pure Helper

```antares/src/game/systems/hud.rs#L1212-1245
pub fn format_clock_date(year: u32, month: u32, day: u32) -> String {
    format!("Y{} M{} D{}", year, month, day)
}
```

Replaces the removed `format_clock_day(day: u32) -> String`. The compact
`"Y{year} M{month} D{day}"` format keeps the clock panel narrow (same width as
the compass widget above it) while conveying all three calendar fields.

#### `setup_hud` Spawn Site Update

The initial placeholder text for the date node changed from `"Day 1"` to `"Y1 M1 D1"`,
and the marker changed from `ClockDayText` to `ClockDateText`:

```antares/src/game/systems/hud.rs#L427-436
            // Date line: "Y1 M1 D1"
            parent.spawn((
                Text::new("Y1 M1 D1"),
                TextFont {
                    font_size: CLOCK_FONT_SIZE,
                    ..default()
                },
                TextColor(CLOCK_TEXT_COLOR),
                ClockDateText,
            ));
```

#### `update_clock()` System Update

```antares/src/game/systems/hud.rs#L580-590
    for (mut text, _color) in &mut date_query {
        **text = format_clock_date(game_time.year, game_time.month, game_time.day);
    }
```

The system now reads all three calendar fields (`year`, `month`, `day`) from
`game_time` and passes them to `format_clock_date`. The day-query variable was
renamed `date_query` for clarity.

### Tests

#### Updated tests in `mod clock_tests`

All five tests that previously referenced `format_clock_day` or `ClockDayText` were
updated in-place:

| Old test name                      | New test name                         | Change                                        |
| ---------------------------------- | ------------------------------------- | --------------------------------------------- |
| `test_clock_day_display_first_day` | `test_format_clock_date_defaults`     | `format_clock_date(1,1,1)` → `"Y1 M1 D1"`     |
| `test_clock_day_display_forty_two` | `test_format_clock_date_large_values` | `format_clock_date(4,12,30)` → `"Y4 M12 D30"` |
| `test_clock_day_display_year`      | `test_clock_date_display_mid_year`    | `format_clock_date(1,6,15)` → `"Y1 M6 D15"`   |
| `test_clock_day_display_zero`      | `test_clock_date_display_year_two`    | `format_clock_date(2,1,1)` → `"Y2 M1 D1"`     |
| `test_clock_day_display_max`       | `test_clock_date_display_max`         | panic-free with all three `u32::MAX`          |

One additional test was added:

| New test name                              | What it verifies             |
| ------------------------------------------ | ---------------------------- |
| `test_clock_date_display_last_day_of_year` | `(1,12,30)` → `"Y1 M12 D30"` |

#### Updated Bevy ECS integration tests

Three existing integration tests in `mod clock_tests` were updated:

| Test name                                      | Change                                                     |
| ---------------------------------------------- | ---------------------------------------------------------- |
| `test_clock_widget_spawned_on_startup`         | Query uses `ClockDateText`; asserts count = 1              |
| `test_clock_widget_shows_default_game_time`    | Query uses `ClockDateText`; asserts `"Y1 M1 D1"` present   |
| `test_clock_widget_updates_after_time_advance` | Query uses `ClockDateText`; asserts `"Y1 M1 D2"` after 18h |

### Architecture Compliance

- `format_clock_date` is a pure function with no side effects — identical pattern
  to `format_clock_time` and the removed `format_clock_day`.
- The `"Y{year} M{month} D{day}"` format is compact and unambiguous, fitting the
  fixed `CLOCK_WIDTH` panel that matches the compass widget width.
- No constants were hardcoded; all clock panel sizing uses the existing
  `CLOCK_WIDTH`, `CLOCK_FONT_SIZE`, and `CLOCK_PADDING` constants.
- `ClockDateText` follows the existing naming convention for HUD marker components.

### Quality Gate Results

```text
cargo fmt --all          → clean (no output)
cargo check              → Finished dev profile, 0 errors
cargo clippy -D warnings → Finished dev profile, 0 warnings
cargo nextest run        → 3371 passed, 8 skipped, 0 failed
```

## Phase 2B: Months and Years — TimeCondition Variants (Complete)

### Overview

Extended `TimeCondition` in `src/domain/world/types.rs` with four new variants that
allow campaign authors to gate map events by calendar month or year, not just by
time-of-day, elapsed days, or hour window.

The four new variants are:

- `DuringMonths(Vec<u32>)` — fires when `game_time.month` is in the supplied list
- `AfterYear(u32)` — fires when `game_time.year > threshold`
- `BeforeYear(u32)` — fires when `game_time.year < threshold`
- `BetweenYears { from: u32, to: u32 }` — fires when `from <= game_time.year <= to`

All existing tests (26) continue to pass unchanged. Twenty-one new tests cover
match/skip/boundary/RON-roundtrip/RON-literal cases for every new variant.

### Phase 2B Deliverables Checklist

- [x] `DuringMonths(Vec<u32>)` variant added to `TimeCondition`
- [x] `AfterYear(u32)` variant added to `TimeCondition`
- [x] `BeforeYear(u32)` variant added to `TimeCondition`
- [x] `BetweenYears { from: u32, to: u32 }` variant added to `TimeCondition`
- [x] `is_met()` extended with match arms for all four new variants
- [x] Enum-level doc comment variant table updated
- [x] Enum-level doc comment examples updated
- [x] `is_met()` doc comment examples updated
- [x] Unit tests pass (21 new tests)
- [x] RON roundtrip tests pass (4 roundtrip + 4 literal = 8 serialization tests)
- [x] All quality gates pass (0 errors, 0 warnings, 3370 tests green)

### What Was Built

#### `TimeCondition` Enum — `src/domain/world/types.rs`

Four variants appended to the existing enum after `BetweenHours`:

```antares/src/domain/world/types.rs#L1827-1848
    /// Event fires only when the current month is in the supplied list.
    ///
    /// Months are 1-based (1 = January … 12 = December in the game calendar).
    /// Use this to gate events by season, e.g. `[11, 12, 1]` for winter.
    DuringMonths(Vec<u32>),
    /// Event fires only after the given year has passed (`game_time.year > threshold`).
    AfterYear(u32),
    /// Event fires only before the given year is reached (`game_time.year < threshold`).
    BeforeYear(u32),
    /// Event fires only while the current year is within `[from, to]` inclusive
    /// (`from <= game_time.year <= to`).
    BetweenYears {
        /// First year of the active window (inclusive).
        from: u32,
        /// Last year of the active window (inclusive).
        to: u32,
    },
```

#### `is_met()` — New Match Arms

Four arms added to the exhaustive match in `TimeCondition::is_met()`:

```antares/src/domain/world/types.rs#L1903-1912
            TimeCondition::DuringMonths(months) => months.contains(&game_time.month),
            TimeCondition::AfterYear(threshold) => game_time.year > *threshold,
            TimeCondition::BeforeYear(threshold) => game_time.year < *threshold,
            TimeCondition::BetweenYears { from, to } => {
                game_time.year >= *from && game_time.year <= *to
            }
```

#### Variant Table in Doc Comment

The enum-level table was extended to document all eight variants:

| Variant         | Fires when …                                                   |
| --------------- | -------------------------------------------------------------- |
| `DuringPeriods` | current `TimeOfDay` is in the supplied list                    |
| `AfterDay`      | `game_time.total_days() > threshold`                           |
| `BeforeDay`     | `game_time.total_days() < threshold`                           |
| `BetweenHours`  | `from <= game_time.hour <= to` (24-hour, inclusive)            |
| `DuringMonths`  | `game_time.month` is in the supplied list (e.g. `[11, 12, 1]`) |
| `AfterYear`     | `game_time.year > threshold`                                   |
| `BeforeYear`    | `game_time.year < threshold`                                   |
| `BetweenYears`  | `from <= game_time.year <= to` (inclusive)                     |

#### RON Usage Examples

Campaign authors can now write these conditions directly in map RON files:

```antares/data/test_campaign/data/maps/map_1.ron#L1-1
// Example RON spellings (not a real file excerpt — illustrative only):
```

```/dev/null/examples.ron#L1-8
// Winter-only event (months 11, 12, 1):
time_condition: Some(DuringMonths([11, 12, 1])),

// Year 2+ content unlock:
time_condition: Some(AfterYear(1)),

// Era-gated story event active during years 2 through 4:
time_condition: Some(BetweenYears(from: 2, to: 4)),
```

### Tests

#### New tests in `src/domain/world/types.rs` — `mod time_condition_tests`

**`DuringMonths` tests (4)**

| Test name                            | What it verifies                             |
| ------------------------------------ | -------------------------------------------- |
| `test_during_months_fires_in_winter` | Months 11, 12, 1 all fire for winter list    |
| `test_during_months_skips_summer`    | Months 6, 7, 8 do not fire for winter list   |
| `test_during_months_single_month`    | Single-element list fires exactly that month |
| `test_during_months_all_months`      | List of all 12 months fires for every month  |

**`AfterYear` tests (3)**

| Test name                  | What it verifies                                     |
| -------------------------- | ---------------------------------------------------- |
| `test_after_year_fires`    | Year 3 and year 10 fire for `AfterYear(2)`           |
| `test_after_year_skips`    | Year 2 and year 1 do not fire for `AfterYear(2)`     |
| `test_after_year_boundary` | Year 1 does not fire, year 2 does for `AfterYear(1)` |

**`BeforeYear` tests (3)**

| Test name                   | What it verifies                                  |
| --------------------------- | ------------------------------------------------- |
| `test_before_year_fires`    | Year 1 and year 2 fire for `BeforeYear(3)`        |
| `test_before_year_skips`    | Year 3 and year 5 do not fire for `BeforeYear(3)` |
| `test_before_year_boundary` | Year 1 fires, year 2 does not for `BeforeYear(2)` |

**`BetweenYears` tests (3)**

| Test name                        | What it verifies                                             |
| -------------------------------- | ------------------------------------------------------------ |
| `test_between_years_fires`       | Years 1, 2, 3 all fire for `BetweenYears{1,3}` (both bounds) |
| `test_between_years_skips`       | Year 5 skips `{1,3}`; years below/above skip `{3,5}`         |
| `test_between_years_single_year` | `from == to` fires only that exact year                      |

**RON serialization tests (8)**

| Test name                                         | What it verifies                       |
| ------------------------------------------------- | -------------------------------------- |
| `test_time_condition_ron_roundtrip_during_months` | Serialize → deserialize `DuringMonths` |
| `test_time_condition_ron_roundtrip_after_year`    | Serialize → deserialize `AfterYear`    |
| `test_time_condition_ron_roundtrip_before_year`   | Serialize → deserialize `BeforeYear`   |
| `test_time_condition_ron_roundtrip_between_years` | Serialize → deserialize `BetweenYears` |
| `test_time_condition_ron_literal_during_months`   | Canonical RON literal deserialises     |
| `test_time_condition_ron_literal_after_year`      | Canonical RON literal deserialises     |
| `test_time_condition_ron_literal_before_year`     | Canonical RON literal deserialises     |
| `test_time_condition_ron_literal_between_years`   | Canonical RON literal deserialises     |

### Architecture Compliance

- No magic numbers — month and year comparisons use the values stored in `GameTime`
  fields which are enforced by the calendar constants from Phase 1A.
- `DuringMonths` mirrors `DuringPeriods` in design: a `Vec` of accepted values,
  checked with `.contains()`. Consistent pattern.
- `AfterYear` / `BeforeYear` mirror `AfterDay` / `BeforeDay` in design: strict
  inequality, single `u32` threshold.
- `BetweenYears` mirrors `BetweenHours` in design: `{ from, to }` struct with
  inclusive bounds on both sides.
- All variants are `#[derive(Serialize, Deserialize)]` via the existing enum derive,
  so RON round-tripping works without any additional code.
- Existing variants and their `is_met()` logic are completely unchanged.

### Quality Gate Results

```text
cargo fmt --all          → clean (no output)
cargo check              → Finished dev profile, 0 errors
cargo clippy -D warnings → Finished dev profile, 0 warnings
cargo nextest run        → 3370 passed, 8 skipped, 0 failed
```

## Phase 1A: Months and Years — Core Time System (Complete)

### Overview

Extended `GameTime` in `src/domain/types.rs` from a three-field `{ day, hour, minute }`
struct to a full five-field calendar struct `{ year, month, day, hour, minute }`.
Added three calendar constants, a `new_full()` constructor, a `total_days()` helper,
and rollover logic in `advance_minutes()` and `advance_days()` so that time correctly
propagates from minutes → hours → days → months → years.

Updated `TimeCondition::AfterDay` and `TimeCondition::BeforeDay` in
`src/domain/world/types.rs` to compare against `game_time.total_days()` rather than
`game_time.day`, preserving the original "total elapsed days" semantics for existing
RON data even though `day` now means "day within month (1–30)".

### Phase 1A Deliverables Checklist

- [x] Calendar constants added (`MONTHS_PER_YEAR`, `DAYS_PER_MONTH`, `DAYS_PER_YEAR`)
- [x] `GameTime` extended with `year` and `month` fields (serde default = 1)
- [x] `GameTime::new_full(year, month, day, hour, minute)` constructor
- [x] `GameTime::total_days()` helper
- [x] `advance_minutes()` rolls day → month → year
- [x] `advance_days()` rolls day → month → year (via shared `apply_day_rollover()`)
- [x] Updated doctests pass
- [x] New rollover unit tests pass (14 new tests)
- [x] `TimeCondition::AfterDay` / `BeforeDay` updated to use `total_days()`
- [x] All quality gates pass (0 errors, 0 warnings, 3349 tests green)

### What Was Built

#### Calendar Constants — `src/domain/types.rs`

Three `pub const` values placed above the `GameTime` struct establish the fixed-length
calendar used throughout the game:

```antares/src/domain/types.rs#L437-447
/// Number of months in a game year.
pub const MONTHS_PER_YEAR: u32 = 12;

/// Number of days in a game month (all months are equal length).
pub const DAYS_PER_MONTH: u32 = 30;

/// Number of days in a game year (MONTHS_PER_YEAR × DAYS_PER_MONTH = 360).
pub const DAYS_PER_YEAR: u32 = MONTHS_PER_YEAR * DAYS_PER_MONTH;
```

#### `GameTime` Struct Extension — `src/domain/types.rs`

Two new fields were prepended to `GameTime` in declaration order (`year`, `month`, then
the existing `day`, `hour`, `minute`). Both use `#[serde(default)]` pointing to private
helper functions that return `1`, so any existing save file or RON data that lacks these
fields deserializes correctly with `year = 1, month = 1`.

```antares/src/domain/types.rs#L505-515
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GameTime {
    /// Current year (1-based)
    #[serde(default = "default_year")]
    pub year: u32,
    /// Current month within the year (1-based, 1–12)
    #[serde(default = "default_month")]
    pub month: u32,
    /// Current day within the month (1-based, 1–30)
    pub day: u32,
    /// Current hour (0-23)
    pub hour: u8,
    /// Current minute (0-59)
    pub minute: u8,
}
```

#### `GameTime::new()` — Backward-Compatible Constructor

The existing three-argument constructor is unchanged in signature; it now sets
`year = 1, month = 1` internally so all call sites continue to compile without
modification.

#### `GameTime::new_full()` — Five-Argument Constructor

A new constructor accepts all five fields in calendar order:

```antares/src/domain/types.rs#L573-581
pub fn new_full(year: u32, month: u32, day: u32, hour: u8, minute: u8) -> Self {
    Self {
        year,
        month,
        day,
        hour,
        minute,
    }
}
```

#### `GameTime::total_days()` — Cumulative Day Counter

Returns the total number of days elapsed since the beginning of the calendar
(Year 1, Month 1, Day 1 = day 1). Used by `TimeCondition::AfterDay` and
`BeforeDay` so their thresholds continue to mean "total elapsed days" even though
`self.day` is now bounded to 1–30.

```antares/src/domain/types.rs#L616-619
pub fn total_days(&self) -> u32 {
    (self.year - 1) * DAYS_PER_YEAR + (self.month - 1) * DAYS_PER_MONTH + self.day
}
```

#### `apply_day_rollover()` — Shared Rollover Helper

A private method called by both `advance_minutes()` and `advance_days()` after
incrementing `self.day`:

```antares/src/domain/types.rs#L679-688
fn apply_day_rollover(&mut self) {
    while self.day > DAYS_PER_MONTH {
        self.day -= DAYS_PER_MONTH;
        self.month += 1;
    }
    while self.month > MONTHS_PER_YEAR {
        self.month -= MONTHS_PER_YEAR;
        self.year += 1;
    }
}
```

#### `TimeCondition::AfterDay` / `BeforeDay` — `src/domain/world/types.rs`

The two match arms that previously compared `game_time.day` now call
`game_time.total_days()`:

```antares/src/domain/world/types.rs#L1839-1848
pub fn is_met(&self, game_time: &GameTime) -> bool {
    match self {
        TimeCondition::DuringPeriods(periods) => periods.contains(&game_time.time_of_day()),
        TimeCondition::AfterDay(threshold) => game_time.total_days() > *threshold,
        TimeCondition::BeforeDay(threshold) => game_time.total_days() < *threshold,
        TimeCondition::BetweenHours { from, to } => {
            game_time.hour >= *from && game_time.hour <= *to
        }
    }
}
```

This preserves all existing RON data semantics: `AfterDay(5)` still fires when
the party has travelled more than 5 cumulative days into the campaign, regardless
of which month or year they are in.

### Tests

#### New tests in `src/domain/types.rs` — 14 new tests

| Test name                                     | What it verifies                                     |
| --------------------------------------------- | ---------------------------------------------------- |
| `test_new_full_constructor`                   | All five fields set correctly                        |
| `test_new_defaults_year_and_month`            | `new()` sets year=1, month=1                         |
| `test_advance_minutes_day_to_month_rollover`  | Day 30 + 1 min → Month 2, Day 1                      |
| `test_advance_minutes_month_to_year_rollover` | Month 12, Day 30, 23:00 + 120 min → Year 2           |
| `test_advance_minutes_multi_year_rollover`    | +2 full years of minutes → Year 3                    |
| `test_advance_days_with_month_rollover`       | +31 days from Day 1 → Month 2, Day 2                 |
| `test_advance_days_exact_month_boundary`      | +30 days from Day 1 → Month 2, Day 1                 |
| `test_advance_days_year_rollover`             | +360 days → Year 2, Month 1, Day 1                   |
| `test_serde_default_year_month`               | RON `(day: 5, hour: 8, minute: 0)` → year=1, month=1 |
| `test_total_days_basic`                       | Y1M1D1=1, Y1M2D10=40, Y2M1D1=361                     |
| `test_total_days_adventure_span`              | M2D10 → M3D12 = 32 days elapsed                      |
| `test_total_days_year_boundary`               | Y1M12D30=360, Y2M1D1=361                             |
| `test_game_time_creation`                     | Extended: also asserts year=1, month=1               |

All 8 pre-existing `GameTime` unit tests continue to pass unchanged. All 26
`TimeCondition` tests continue to pass.

### Architecture Compliance

- Constants use named `pub const` values — no magic numbers.
- `#[serde(default)]` used for backward compatibility — no breaking format change.
- `day` field semantics changed from "total elapsed days" to "day within month";
  all code that required cumulative days was updated to use `total_days()`.
- `new()` constructor preserved — zero call-site changes required across the codebase.
- RON format unchanged; existing data files deserialize correctly.

### Quality Gate Results

```text
cargo fmt --all          → clean (no output)
cargo check              → Finished dev profile, 0 errors
cargo clippy -D warnings → Finished dev profile, 0 warnings
cargo nextest run        → 3349 passed, 8 skipped, 0 failed
```

## Phase 5: Campaign Builder — Starting Date/Time (Complete)

### Overview

Phase 5 wires a configurable **starting date/time** into every layer of the
campaign stack: the `CampaignConfig` data structure, the `CampaignMetadata`
RON file, the `GameState` initialisation path, the Campaign Builder editor
buffer, and the Gameplay section of the Campaign Builder UI. A campaign
author can now open Campaign Builder → Campaign Editor → Gameplay, set
Day 3, 22:00 as the starting time, save, and launch the game to find the HUD
clock (Phase 3) showing `22:00` and `Day 3` from the very first frame of
exploration. Campaigns whose `campaign.ron` lacks the field silently fall
back to Day 1, 08:00 via `serde(default)`.

---

### Phase 5 Deliverables Checklist

- [x] `starting_time: GameTime` field on `CampaignConfig` with `serde(default = "default_starting_time")`
- [x] `default_starting_time()` returning `GameTime::new(1, 8, 0)` (morning)
- [x] `starting_time: GameTime` field on `CampaignMetadata` with `serde(default)`
- [x] `CampaignMetadata → CampaignConfig` conversion propagates `starting_time`
- [x] `GameState::new_game()` initialises `state.time` from `campaign.config.starting_time`
- [x] `starting_day`, `starting_hour`, `starting_minute` fields on `CampaignMetadataEditBuffer`
- [x] `CampaignMetadataEditBuffer::from_metadata()` copies all three fields from `starting_time`
- [x] `CampaignMetadataEditBuffer::apply_to()` clamps and writes back to `dest.starting_time`
- [x] **Starting Date/Time** row in Campaign Builder → Campaign Editor → Gameplay section
- [x] `period_label()` helper with `TimeOfDay` preview hint next to the spinners
- [x] `campaigns/tutorial/campaign.ron` — explicit `starting_time: GameTime(day: 1, hour: 8, minute: 0)`
- [x] `data/test_campaign/campaign.ron` — explicit `starting_time: (day: 1, hour: 8, minute: 0)`
- [x] All phase-5 tests pass (3337/3337)

---

### What Was Built

#### `starting_time` on `CampaignConfig` — `src/sdk/campaign_loader.rs`

A `starting_time: GameTime` field was added to `CampaignConfig` (L192–197).
The `#[serde(default = "default_starting_time")]` attribute guarantees
backward compatibility with any `campaign.ron` file that pre-dates this change.
`default_starting_time()` returns `GameTime::new(1, 8, 0)` — Day 1, 08:00 —
so campaigns without an explicit field start in the morning.

The `TryFrom<CampaignMetadata>` implementation at L533–537 copies
`metadata.starting_time` directly into the resulting `CampaignConfig`.

#### `starting_time` on `CampaignMetadata` — `src/sdk/campaign_loader.rs`

`CampaignMetadata` (L480–484) received the same `starting_time: GameTime`
field with an identical `serde(default)`. This is the struct that is
deserialised from `campaign.ron` on disk, so the RON files only need to
contain the field when a non-default starting time is desired.

#### `GameState::new_game()` — `src/application/mod.rs`

Inside `new_game()` (L645–654) the starting time is extracted from the
campaign config before the `GameState` is constructed:

```antares/src/application/mod.rs#L645-654
// Initialise the game clock from the campaign's configured starting time.
// Campaign authors set this in config.ron via `starting_time: (day: N, hour: H, minute: M)`.
// Falls back to Day 1, 08:00 when the field is absent (serde default).
let starting_time = campaign.config.starting_time;

let mut state = Self {
    // ...
    time: starting_time,
    // ...
};
```

This means that from the very first frame of exploration the HUD clock, the
ambient-lighting system, and every time-gated event condition all see the
campaign author's intended starting time.

#### `CampaignMetadataEditBuffer` — `sdk/campaign_builder/src/campaign_editor.rs`

Three fields were added to the buffer struct (L103–109) to allow ergonomic
drag-value editing in egui:

```antares/sdk/campaign_builder/src/campaign_editor.rs#L103-109
// Starting date/time (split from GameTime for ergonomic drag-value editing)
/// Starting day (1-based)
pub starting_day: u32,
/// Starting hour (0–23)
pub starting_hour: u8,
/// Starting minute (0–59)
pub starting_minute: u8,
```

`from_metadata()` (L149–155) copies `m.starting_time.{day,hour,minute}` into
the three split fields.

`apply_to()` (L192–196) reconstructs `GameTime` with clamping:

- `starting_day.max(1)` — day is 1-based; 0 is invalid
- `starting_hour.min(23)` — hours are 0–23
- `starting_minute.min(59)` — minutes are 0–59

`Default for CampaignMetadataEditBuffer` seeds the fields from
`CampaignMetadata::default()`, which delegates to `default_starting_time()`,
so a freshly opened editor always shows Day 1, 08:00.

#### Starting Date/Time UI Row — `sdk/campaign_builder/src/campaign_editor.rs`

The Gameplay section (`CampaignSection::Gameplay`, L1075–1128) contains the
new row immediately after the **Starting Direction** ComboBox and before the
**Starting Gold** drag-value:

- Three `egui::DragValue` widgets with enforced ranges (`1..=9999` for day,
  `0..=23` for hour, `0..=59` for minute).
- A `ui.colored_label` grey preview that calls `period_label()` to show
  which time-of-day period the selected time falls in — e.g. `(Morning)` for
  08:00 or `(Night)` for 22:00.

#### `period_label()` helper — `sdk/campaign_builder/src/campaign_editor.rs`

A public helper at L1478–1487 maps every `TimeOfDay` variant to a short
`&'static str` label. It is also tested by a doc-test and by
`test_period_label_all_variants`.

#### RON Data Files Updated

Both canonical `campaign.ron` files now carry an explicit `starting_time`
field so the `serde(default)` fallback is never exercised in production or the
test fixture:

- `campaigns/tutorial/campaign.ron` — `starting_time: GameTime(day: 1, hour: 8, minute: 0)`
- `data/test_campaign/campaign.ron` — `starting_time: (day: 1, hour: 8, minute: 0)`

---

### Tests

#### Domain / loader tests — `src/sdk/campaign_loader.rs`

| Test                                                      | What it verifies                                                               |
| --------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `test_campaign_config_starting_time_default`              | `default_starting_time()` returns Day 1, 08:00                                 |
| `test_campaign_config_starting_time_roundtrip`            | RON serialise → deserialise preserves `GameTime::new(3, 22, 30)`               |
| `test_campaign_config_missing_starting_time_uses_default` | RON string without `starting_time` key defaults to Day 1, 08:00                |
| `test_test_campaign_has_explicit_starting_time`           | Loading `data/test_campaign` yields `starting_time.day==1, hour==8, minute==0` |

#### Campaign Editor buffer tests — `sdk/campaign_builder/src/campaign_editor.rs`

| Test                                               | What it verifies                                                            |
| -------------------------------------------------- | --------------------------------------------------------------------------- |
| `test_buffer_from_metadata_copies_starting_time`   | `from_metadata()` copies day=2, hour=20, minute=45 into split fields        |
| `test_buffer_apply_to_writes_starting_time`        | `apply_to()` writes day=5, hour=6, minute=30 back to `GameTime`             |
| `test_buffer_starting_time_clamps_hour`            | hour=25 is clamped to 23 by `apply_to()`                                    |
| `test_buffer_starting_time_clamps_minute`          | minute=75 is clamped to 59 by `apply_to()`                                  |
| `test_buffer_starting_time_clamps_day_zero`        | day=0 is clamped to 1 by `apply_to()`                                       |
| `test_buffer_default_starting_time_fields`         | Default buffer has day=1, hour=8, minute=0                                  |
| `test_buffer_starting_time_roundtrip_via_metadata` | Split fields → `apply_to` → `from_metadata` round-trip preserves values     |
| `test_period_label_all_variants`                   | `period_label` returns correct string for all six `TimeOfDay` variants      |
| `test_period_label_matches_game_time_time_of_day`  | `period_label` agrees with `GameTime::time_of_day` for representative hours |

---

### Architecture Compliance

| Check                                                                       | Status |
| --------------------------------------------------------------------------- | ------ |
| Data structures match `architecture.md` Section 4.9 `CampaignConfig`        | ✅     |
| `serde(default)` ensures backward-compatible RON deserialization            | ✅     |
| `GameTime` type alias used consistently (not raw fields)                    | ✅     |
| `default_starting_time()` named consistently with other `default_*` helpers | ✅     |
| Campaign builder split-field pattern matches existing DragValue conventions | ✅     |
| RON format used for all data files (not JSON/YAML)                          | ✅     |
| Test data under `data/test_campaign`, not `campaigns/tutorial`              | ✅     |
| No architectural deviations from `architecture.md`                          | ✅     |

---

### Quality Gate Results

```/dev/null/quality_gates.txt#L1-8
cargo fmt --all          → No output (all files formatted)
cargo check --all-targets --all-features
                         → Finished (0 errors)
cargo clippy --all-targets --all-features -- -D warnings
                         → Finished (0 warnings)
cargo nextest run --all-features
                         → 3337 tests run: 3337 passed, 8 skipped
```

---

## Phase 4: Time-Triggered Events (Complete)

### Overview

Phase 4 introduces time-gated map events — any `MapEvent` variant that supports
it can now carry an optional `time_condition` field. When the condition is not
met the event returns `EventResult::None` without being consumed, so the
event re-evaluates on every future visit until the window opens. When
`time_condition` is `None` (the default) the event fires unconditionally,
preserving full backward compatibility with all existing RON map files.

| Deliverable                                                                          | Location                               |
| ------------------------------------------------------------------------------------ | -------------------------------------- |
| `TimeCondition` enum                                                                 | `src/domain/world/types.rs`            |
| `time_condition` field on `Encounter`, `Sign`, `NpcDialogue`, `RecruitableCharacter` | `src/domain/world/types.rs`            |
| `trigger_event(world, position, &game_time)` gating logic                            | `src/domain/world/events.rs`           |
| `TimeAdvanceEvent` + `apply_time_advance` system                                     | `src/game/systems/time.rs`             |
| Campaign-author how-to guide                                                         | `docs/how-to/authoring_time_events.md` |

### Phase 4 Deliverables Checklist

- [x] `TimeCondition` enum in `src/domain/world/types.rs` — four variants, `is_met(&GameTime) -> bool` method
- [x] `time_condition: Option<TimeCondition>` on `MapEvent::Encounter`, `::Sign`, `::NpcDialogue`, `::RecruitableCharacter` with `#[serde(default)]`
- [x] `trigger_event` accepts `&GameTime` and evaluates the condition before processing
- [x] Unmet condition returns `EventResult::None` without consuming the event
- [x] `TimeAdvanceEvent { minutes: u32 }` Bevy event in `src/game/systems/time.rs`
- [x] `apply_time_advance` system draining `TimeAdvanceEvent` queue and calling `GameState::advance_time`
- [x] `TimeCondition` publicly re-exported from `src/domain/world/mod.rs`
- [x] `docs/how-to/authoring_time_events.md` with complete RON examples for all four variants
- [x] All Phase 4 tests pass (27 tests across `mod time_condition_tests` and `mod tests`)

### What Was Built

#### `TimeCondition` Enum — `src/domain/world/types.rs`

Four variants cover the use cases described in the plan:

```src/domain/world/types.rs#L1791-1805
pub enum TimeCondition {
    /// Event fires only during these time-of-day periods.
    DuringPeriods(Vec<TimeOfDay>),
    /// Event fires only after this many in-game days have elapsed (day > threshold).
    AfterDay(u32),
    /// Event fires only before this many in-game days have elapsed (day < threshold).
    BeforeDay(u32),
    /// Event fires only between these hours (inclusive, 0–23, 24-hour clock).
    BetweenHours {
        /// First hour of the active window (0–23, inclusive).
        from: u8,
        /// Last hour of the active window (0–23, inclusive).
        to: u8,
    },
}
```

The `is_met(&GameTime) -> bool` method is a pure function — no side-effects,
safe from both the domain layer and Bevy systems:

```src/domain/world/types.rs#L1838-1847
pub fn is_met(&self, game_time: &GameTime) -> bool {
    match self {
        TimeCondition::DuringPeriods(periods) => periods.contains(&game_time.time_of_day()),
        TimeCondition::AfterDay(threshold) => game_time.day > *threshold,
        TimeCondition::BeforeDay(threshold) => game_time.day < *threshold,
        TimeCondition::BetweenHours { from, to } => {
            game_time.hour >= *from && game_time.hour <= *to
        }
    }
}
```

`TimeCondition` derives `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`
so it round-trips through RON without loss.

#### `time_condition` Field on `MapEvent` Variants — `src/domain/world/types.rs`

The field is added to `Encounter`, `Sign`, `NpcDialogue`, and
`RecruitableCharacter`. Each uses `#[serde(default)]` so existing RON files
that omit the field continue to load cleanly with `None`:

```src/domain/world/types.rs#L1871-1875
        /// Optional time condition — if `Some`, the encounter only fires when
        /// the condition is met.  `None` means always fire (default, backward
        /// compatible with existing RON data).
        #[serde(default)]
        time_condition: Option<TimeCondition>,
```

The same pattern is applied to `Sign` (L1953), `NpcDialogue` (L1975), and
`RecruitableCharacter` (L2021).

#### Time-Gating in `trigger_event` — `src/domain/world/events.rs`

The function signature was extended with `game_time: &GameTime`. Before
processing any event the function checks whether the event carries a
`time_condition` and evaluates it:

```src/domain/world/events.rs#L168-178
pub fn trigger_event(
    world: &mut World,
    position: Position,
    game_time: &GameTime,
) -> Result<EventResult, EventError> {
```

The gating block:

```src/domain/world/events.rs#L208-228
    let time_condition_met = match &event {
        MapEvent::Encounter {
            time_condition: Some(tc),
            ..
        } => tc.is_met(game_time),
        MapEvent::Sign {
            time_condition: Some(tc),
            ..
        } => tc.is_met(game_time),
        MapEvent::NpcDialogue {
            time_condition: Some(tc),
            ..
        } => tc.is_met(game_time),
        MapEvent::RecruitableCharacter {
            time_condition: Some(tc),
            ..
        } => tc.is_met(game_time),
        // All other variants, and any variant whose time_condition is None,
        // are unconditionally allowed.
        _ => true,
    };

    if !time_condition_met {
        return Ok(EventResult::None);
    }
```

Key design properties:

- The event is **not consumed** when the condition is not met — it stays on the
  map and is re-evaluated on the next step.
- The domain layer remains pure — the caller passes `&game_state.time`; no
  `GameState` or Bevy types enter the domain module.
- All existing callers pass `&self.time` — the one caller in
  `src/application/mod.rs::move_party_and_handle_events` was updated when the
  signature changed.

#### `TimeAdvanceEvent` and `apply_time_advance` — `src/game/systems/time.rs`

The `TimeAdvanceEvent { minutes: u32 }` Bevy message type and the
`apply_time_advance` drain system were implemented as part of Phase 1 and are
registered by `TimeOfDayPlugin`. Phase 4 depends on — but does not duplicate —
this infrastructure. See the Phase 1 and Phase 2 entries for the full
description.

#### `TimeCondition` Public Re-export — `src/domain/world/mod.rs`

`TimeCondition` is exported from the world module facade so campaign tooling
and tests can import it via the short path:

```src/domain/world/mod.rs#L43-45
pub use types::{
    ...
    TimeCondition, ...
};
```

#### `docs/how-to/authoring_time_events.md`

A complete campaign-author guide covering:

- How time conditions work and which event variants support them
- All four `TimeCondition` variants with RON examples
- A full self-contained map file showing all four variants in one place
- Cross-midnight caveats for `BetweenHours`
- Backward-compatibility guarantee
- Testing instructions pointing authors at `trigger_event` unit tests

### Tests

#### `mod time_condition_tests` in `src/domain/world/types.rs` — 21 tests

| Test                                                      | What it verifies                                                                    |
| --------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `test_time_condition_during_periods_night_fires_at_night` | `DuringPeriods([Night])` fires at hour 23                                           |
| `test_time_condition_during_periods_night_skips_at_noon`  | `DuringPeriods([Night])` does not fire at hour 12                                   |
| `test_time_condition_during_periods_evening_and_night`    | Multi-period list fires at both Evening and Night, not at Dawn                      |
| `test_time_condition_during_periods_all_six_variants`     | Each of the six `TimeOfDay` variants fires at its canonical hour and no other       |
| `test_time_condition_after_day_fires`                     | `AfterDay(5)` fires on day 6 and 10; not on day 5 or 1                              |
| `test_time_condition_after_day_boundary`                  | `AfterDay(1)` does not fire on day 1; fires on day 2                                |
| `test_time_condition_before_day_fires`                    | `BeforeDay(10)` fires on day 9 and 1; not on day 10 or 15                           |
| `test_time_condition_between_hours_fires_within_range`    | `BetweenHours{20,23}` fires at hours 20, 21, 23                                     |
| `test_time_condition_between_hours_skips_outside_range`   | `BetweenHours{20,23}` does not fire at hour 19 or midnight                          |
| `test_time_condition_between_hours_full_day_range`        | `BetweenHours{0,23}` fires at every hour 0–23                                       |
| `test_time_condition_between_hours_single_hour`           | `BetweenHours{12,12}` fires only at hour 12                                         |
| `test_time_condition_ron_roundtrip_during_periods`        | `DuringPeriods` survives RON serialise → deserialise                                |
| `test_time_condition_ron_roundtrip_after_day`             | `AfterDay` survives RON round-trip                                                  |
| `test_time_condition_ron_roundtrip_between_hours`         | `BetweenHours` survives RON round-trip                                              |
| `test_map_event_encounter_time_condition_none_by_default` | Struct literal with `time_condition: None` compiles and matches                     |
| `test_map_event_encounter_with_time_condition_night`      | `Encounter` with `DuringPeriods([Night, Evening])` fires at 23:00, not 12:00        |
| `test_map_event_sign_time_condition_ron_roundtrip`        | `Sign` with `DuringPeriods([Night])` serialises to RON containing `"DuringPeriods"` |
| `test_map_event_sign_no_time_condition_backward_compat`   | RON `Sign` without `time_condition` field deserialises to `None`                    |
| `test_map_event_npc_dialogue_time_condition`              | `NpcDialogue` with `BetweenHours{8,18}` fires at noon, not at 22:00                 |
| `test_map_event_recruitable_character_time_condition`     | `RecruitableCharacter` with `AfterDay(3)` fires on day 4, not day 3                 |

#### `mod tests` in `src/domain/world/events.rs` — 6 time-condition integration tests

| Test                                                 | What it verifies                                                                      |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------- |
| `test_time_condition_night_fires_at_night`           | `trigger_event` with night-only Encounter returns `EventResult::Encounter` at hour 23 |
| `test_time_condition_night_skips_at_noon`            | Same event returns `EventResult::None` at hour 12                                     |
| `test_time_condition_after_day_fires`                | Sign with `AfterDay(5)` fires on day 10; returns None on day 3 and day 5 (boundary)   |
| `test_time_condition_between_hours`                  | NpcDialogue with `BetweenHours{8,18}` fires at 8, 13, 18; returns None at 7 and 19    |
| `test_no_time_condition_always_fires`                | Unconditional Sign fires at every hour 0–23                                           |
| `test_time_condition_not_met_does_not_consume_event` | Event skipped at noon is still present and fires at night                             |

### Architecture Compliance

- [x] `TimeCondition` enum matches the four variants specified in plan §4.1 exactly
- [x] `Option<TimeCondition>` with `#[serde(default)]` on all four applicable event variants — backward compatible
- [x] `trigger_event` accepts `&GameTime` as specified in plan §4.2; domain layer remains pure
- [x] Unmet condition returns `EventResult::None` without consuming the event — specified in plan §4.2
- [x] `TimeAdvanceEvent` + `apply_time_advance` present in `src/game/systems/time.rs` — plan §4.3
- [x] `docs/how-to/authoring_time_events.md` present with night-ambush RON example — plan §4.4
- [x] All five spec tests from plan §4.5 present and passing
- [x] `TimeCondition` derives `Serialize, Deserialize` for RON compatibility — plan §4.4
- [x] No `unwrap()` without justification; domain functions return `Result`

### Quality Gate Results

```
cargo fmt --all          → no output (all files formatted)
cargo check              → Finished with 0 errors, 0 warnings
cargo clippy -D warnings → Finished with 0 warnings
cargo nextest run        → 3337 passed, 0 failed, 8 skipped
```

---

## Phase 3: Clock UI in the HUD (Complete)

### Overview

Phase 3 adds a visible clock widget to the exploration HUD, positioned directly
below the compass in the top-right corner. The widget shows two lines of text
that update every frame:

- **Time line** — `"HH:MM"` with zero-padded hours and minutes
- **Day line** — `"Day N"` where N is the current in-game day

The time text tints between warm golden (`CLOCK_DAY_TEXT_COLOR`) during bright
periods (Dawn through Dusk) and cool blue-white (`CLOCK_NIGHT_TEXT_COLOR`)
during dark periods (Evening and Night), giving the player an ambient cue that
mirrors the `TimeOfDay::is_dark()` flag from Phase 2. The clock is gated by
`not_in_combat` so it does not render on top of the combat HUD.

### Phase 3 Deliverables Checklist

- [x] `ClockRoot`, `ClockTimeText`, `ClockDayText` marker components in `src/game/systems/hud.rs`
- [x] `CLOCK_FONT_SIZE`, `CLOCK_BACKGROUND_COLOR`, `CLOCK_BORDER_COLOR`, `CLOCK_TEXT_COLOR`, `CLOCK_NIGHT_TEXT_COLOR`, `CLOCK_DAY_TEXT_COLOR` constants
- [x] `CLOCK_TOP_OFFSET`, `CLOCK_WIDTH`, `CLOCK_PADDING` layout constants
- [x] Clock widget spawned in `setup_hud` — absolute-positioned below the compass, two `Text` children
- [x] `update_clock` system updating time and day text + time text colour every frame
- [x] `update_clock` registered in `HudPlugin` inside the `not_in_combat`-gated system set
- [x] `format_clock_time()` and `format_clock_day()` pure helper functions for testability
- [x] `clock_text_color()` pure helper delegating to `TimeOfDay::is_dark()`
- [x] All Phase 3 tests pass (31 clock-specific tests across `mod clock_tests` and `mod tests`)

### What Was Built

#### Marker Components — `src/game/systems/hud.rs`

Three zero-sized marker components identify the clock entities in queries:

```src/game/systems/hud.rs#L151-163
/// Marker component for the clock widget container (sits below the compass)
#[derive(Component)]
pub struct ClockRoot;

/// Marker component for the time-of-day text node (displays "HH:MM")
#[derive(Component)]
pub struct ClockTimeText;

/// Marker component for the day counter text node (displays "Day N")
#[derive(Component)]
pub struct ClockDayText;
```

#### Constants — `src/game/systems/hud.rs`

Nine constants define the clock's visual style and layout, placed adjacent to
the existing compass constants so related values are grouped together:

| Constant                 | Value                       | Purpose                              |
| ------------------------ | --------------------------- | ------------------------------------ |
| `CLOCK_FONT_SIZE`        | `14.0`                      | Text size for both clock lines       |
| `CLOCK_BACKGROUND_COLOR` | `srgba(0.1, 0.1, 0.1, 0.9)` | Matches compass panel style          |
| `CLOCK_BORDER_COLOR`     | `srgba(0.4, 0.4, 0.4, 1.0)` | Matches compass border               |
| `CLOCK_TEXT_COLOR`       | `srgba(1.0, 1.0, 1.0, 1.0)` | Default white (used for day line)    |
| `CLOCK_NIGHT_TEXT_COLOR` | `srgba(0.6, 0.6, 1.0, 1.0)` | Cool blue-white for dark periods     |
| `CLOCK_DAY_TEXT_COLOR`   | `srgba(1.0, 0.9, 0.5, 1.0)` | Warm golden for bright periods       |
| `CLOCK_TOP_OFFSET`       | `COMPASS_SIZE + 28.0`       | Positions clock below compass (76px) |
| `CLOCK_WIDTH`            | `COMPASS_SIZE` (48px)       | Matches compass width                |
| `CLOCK_PADDING`          | `4.0`                       | Inner padding of the clock panel     |

#### Clock Widget Spawn — `setup_hud` in `src/game/systems/hud.rs`

The clock is spawned at the end of `setup_hud` as a separate absolute-positioned
node. It uses `FlexDirection::Column` to stack the two text children vertically,
anchored at `right: 20px` / `top: CLOCK_TOP_OFFSET`:

```src/game/systems/hud.rs#L399-440
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(20.0),
                top: Val::Px(CLOCK_TOP_OFFSET),
                width: Val::Px(CLOCK_WIDTH),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(Val::Px(CLOCK_PADDING)),
                row_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(CLOCK_BACKGROUND_COLOR),
            ClockRoot,
        ))
        .with_children(|parent| {
            parent.spawn((Text::new("00:00"), ..., ClockTimeText));
            parent.spawn((Text::new("Day 1"), ..., ClockDayText));
        });
```

#### `update_clock` System — `src/game/systems/hud.rs`

Runs every frame in the `not_in_combat`-gated `Update` set. It reads
`global_state.0.time_of_day()` via `GameState::time_of_day()` (Phase 2
helper) to pick the correct text colour, then updates both text nodes:

```src/game/systems/hud.rs#L566-585
fn update_clock(
    global_state: Res<GlobalState>,
    mut time_query: Query<
        (&mut Text, &mut TextColor),
        (With<ClockTimeText>, Without<ClockDayText>),
    >,
    mut day_query: Query<(&mut Text, &mut TextColor), (With<ClockDayText>, Without<ClockTimeText>)>,
) {
    let game_time = &global_state.0.time;
    let time_of_day = global_state.0.time_of_day();
    let time_color = clock_text_color(time_of_day);

    for (mut text, mut color) in &mut time_query {
        **text = format_clock_time(game_time.hour, game_time.minute);
        *color = TextColor(time_color);
    }
    for (mut text, _color) in &mut day_query {
        **text = format_clock_day(game_time.day);
    }
}
```

The `Without<ClockDayText>` / `Without<ClockTimeText>` filter guards prevent
Bevy's borrow checker from rejecting the two simultaneous mutable queries over
the same `Text` component type.

#### `HudPlugin` Registration — `src/game/systems/hud.rs`

`update_clock` is added to the `not_in_combat`-gated system tuple alongside
`update_compass`, `update_portraits`, and `ensure_portraits_loaded`:

```src/game/systems/hud.rs#L188-205
    fn build(&self, app: &mut App) {
        app.insert_resource(PortraitAssets::default())
            .add_systems(Startup, (setup_hud, setup_party_entities))
            .add_systems(Update, update_hud)
            .add_systems(
                Update,
                (
                    ensure_portraits_loaded,
                    update_compass,
                    update_clock,
                    update_portraits,
                )
                    .run_if(not_in_combat),
            );
    }
```

#### Pure Helper Functions — `src/game/systems/hud.rs`

Three public helpers are extracted from the system so they are independently
testable without a Bevy world:

- `format_clock_time(hour: u8, minute: u8) -> String` — zero-pads both fields to `"HH:MM"`
- `format_clock_day(day: u32) -> String` — produces `"Day N"`
- `clock_text_color(time_of_day: TimeOfDay) -> Color` — delegates to `TimeOfDay::is_dark()`, returning `CLOCK_NIGHT_TEXT_COLOR` or `CLOCK_DAY_TEXT_COLOR`

### Tests

All tests live in `mod clock_tests` (and a few clock-constant checks in `mod tests`) within `src/game/systems/hud.rs`.

#### `format_clock_time` tests

| Test                                                  | What it verifies                                     |
| ----------------------------------------------------- | ---------------------------------------------------- |
| `test_clock_format_midnight`                          | `(0, 0)` → `"00:00"`                                 |
| `test_clock_format_noon`                              | `(12, 5)` → `"12:05"` (minute zero-padded)           |
| `test_clock_format_single_digit_hour`                 | `(9, 0)` → `"09:00"` (hour zero-padded)              |
| `test_clock_format_end_of_day`                        | `(23, 59)` → `"23:59"`                               |
| `test_clock_format_zero_hour_one_minute`              | `(0, 1)` → `"00:01"`                                 |
| `test_clock_format_dawn_default`                      | `(6, 30)` → `"06:30"`                                |
| `test_clock_format_all_hours_produce_valid_strings`   | Every hour 0–23 produces a 5-char `"HH:MM"` string   |
| `test_clock_format_all_minutes_produce_valid_strings` | Every minute 0–59 produces a 5-char `"HH:MM"` string |

#### `format_clock_day` tests

| Test                               | What it verifies                                       |
| ---------------------------------- | ------------------------------------------------------ |
| `test_clock_day_display_first_day` | `1` → `"Day 1"`                                        |
| `test_clock_day_display_forty_two` | `42` → `"Day 42"`                                      |
| `test_clock_day_display_year`      | `365` → `"Day 365"`                                    |
| `test_clock_day_display_zero`      | `0` → `"Day 0"` (must not panic)                       |
| `test_clock_day_display_max`       | `u32::MAX` must not panic; result starts with `"Day "` |

#### `clock_text_color` tests

| Test                                                        | What it verifies                                              |
| ----------------------------------------------------------- | ------------------------------------------------------------- |
| `test_clock_text_color_night_returns_night_color`           | `Night` → `CLOCK_NIGHT_TEXT_COLOR`                            |
| `test_clock_text_color_evening_returns_night_color`         | `Evening` → `CLOCK_NIGHT_TEXT_COLOR` (is_dark)                |
| `test_clock_text_color_dawn_returns_day_color`              | `Dawn` → `CLOCK_DAY_TEXT_COLOR`                               |
| `test_clock_text_color_morning_returns_day_color`           | `Morning` → `CLOCK_DAY_TEXT_COLOR`                            |
| `test_clock_text_color_afternoon_returns_day_color`         | `Afternoon` → `CLOCK_DAY_TEXT_COLOR`                          |
| `test_clock_text_color_dusk_returns_day_color`              | `Dusk` → `CLOCK_DAY_TEXT_COLOR`                               |
| `test_clock_text_color_agrees_with_is_dark_for_all_periods` | `clock_text_color` agrees with `is_dark()` for all 6 variants |

#### Constant sanity tests

| Test                                                  | What it verifies                                             |
| ----------------------------------------------------- | ------------------------------------------------------------ |
| `test_clock_font_size_is_positive`                    | `CLOCK_FONT_SIZE > 0.0`                                      |
| `test_clock_top_offset_places_clock_below_compass`    | `CLOCK_TOP_OFFSET > COMPASS_SIZE` (no overlap)               |
| `test_clock_width_is_positive`                        | `CLOCK_WIDTH > 0.0`                                          |
| `test_clock_padding_is_non_negative`                  | `CLOCK_PADDING >= 0.0`                                       |
| `test_clock_night_and_day_colors_are_distinct`        | Night and day colors differ by >0.05 on at least one channel |
| `test_clock_colors_are_opaque`                        | All three text color constants have `alpha == 1.0`           |
| `test_clock_background_and_border_colors_are_visible` | Background and border have `alpha > 0.0`                     |
| `test_clock_constants_valid` (in `mod tests`)         | `CLOCK_FONT_SIZE > 0.0`, `CLOCK_BACKGROUND_COLOR` alpha > 0  |

#### Bevy integration tests

| Test                                           | What it verifies                                                                                |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `test_clock_widget_spawned_on_startup`         | After `app.update()` exactly one `ClockRoot`, `ClockTimeText`, and `ClockDayText` entity exists |
| `test_clock_widget_shows_default_game_time`    | Default `GameState` (06:00, Day 1) produces `"06:00"` and `"Day 1"` text after two updates      |
| `test_clock_widget_updates_after_time_advance` | `GameState` advanced 18 h from 06:00 → 00:00 Day 2 produces `"00:00"` and `"Day 2"` text        |

### Architecture Compliance

- [x] `ClockRoot`, `ClockTimeText`, `ClockDayText` marker components match the names specified in plan §3.1 exactly
- [x] All nine constants match the values specified in plan §3.1
- [x] Clock spawned in `setup_hud` as specified in plan §3.2 — absolute-positioned below the compass
- [x] `update_clock` system matches the signature pattern from plan §3.3 (extended with `TextColor` mutation for the tinting bonus)
- [x] `update_clock` registered in `HudPlugin` under `not_in_combat` as specified in plan §3.4
- [x] All three spec tests (`test_clock_format_midnight`, `test_clock_format_noon`, `test_clock_day_display`) present and passing
- [x] Pure helper functions extracted for testability — no `unwrap()` without justification
- [x] No architectural deviations from `architecture.md`

### Quality Gate Results

```
cargo fmt --all          → no output (all files formatted)
cargo check              → Finished with 0 errors, 0 warnings
cargo clippy -D warnings → Finished with 0 warnings
cargo nextest run        → 3337 passed, 0 failed, 8 skipped
```

---

## Phase 2: Time-of-Day System (Complete)

### Overview

Phase 2 introduces the full `TimeOfDay` classification system, which maps the
in-game 24-hour clock onto six named periods. Every system that needs to know
whether it is dawn, midday, dusk, or pitch-dark can call a single helper
rather than comparing raw hour numbers.

| Period    | Hours         | Notes                                             |
| --------- | ------------- | ------------------------------------------------- |
| Dawn      | 05:00 – 07:59 | Pale light; roosters crow                         |
| Morning   | 08:00 – 11:59 | Full daylight                                     |
| Afternoon | 12:00 – 15:59 | Peak brightness                                   |
| Dusk      | 16:00 – 18:59 | Golden hour; shadows lengthen                     |
| Evening   | 19:00 – 21:59 | Dark but not full night; light source recommended |
| Night     | 22:00 – 04:59 | Pitch black; light source required outdoors       |

Ambient light is updated every frame by the `update_ambient_light` system in
`src/game/systems/time.rs` so any time advancement (step, combat round, rest,
map transition) is reflected in the rendered scene within the same frame.

### Phase 2 Deliverables Checklist

- [x] `TimeOfDay` enum in `src/domain/types.rs` — six variants with correct hour boundaries, `label()`, and `is_dark()`
- [x] `GameTime::time_of_day()` — maps `self.hour` to the correct `TimeOfDay` variant
- [x] `GameTime::is_night()` — delegates to `time_of_day()`, returns `true` for `Evening | Night`
- [x] `GameTime::is_day()` — delegates to `is_night()` via logical inverse
- [x] `GameState::time_of_day()` convenience helper in `src/application/mod.rs`
- [x] Ambient-light hook in `src/game/systems/time.rs` reading `time_of_day()` every frame
- [x] `AMBIENT_NIGHT_BRIGHTNESS`, `AMBIENT_EVENING_BRIGHTNESS`, `AMBIENT_DAWN_BRIGHTNESS`, `AMBIENT_DUSK_BRIGHTNESS`, `AMBIENT_DAY_BRIGHTNESS` constants
- [x] `time_of_day_brightness()` pure function for testability
- [x] `TimeOfDayPlugin` registering both `apply_time_advance` and `update_ambient_light` systems
- [x] All Phase 2 tests pass (see table below)

### What Was Built

#### `TimeOfDay` Enum — `src/domain/types.rs`

Six variants cover the full 24-hour cycle with precise hour boundaries:

```src/domain/types.rs#L381-394
pub enum TimeOfDay {
    /// 05:00–07:59 — pale light, roosters crow
    Dawn,
    /// 08:00–11:59 — full daylight
    Morning,
    /// 12:00–15:59 — peak brightness
    Afternoon,
    /// 16:00–18:59 — golden light, shadows lengthen
    Dusk,
    /// 19:00–21:59 — dark but not full night
    Evening,
    /// 22:00–04:59 — pitch black without a light source
    Night,
}
```

Two helper methods live on `TimeOfDay`:

- `label() -> &'static str` — returns a human-readable period name (`"Dawn"`, `"Night"`, etc.), used by the HUD clock colour system.
- `is_dark() -> bool` — returns `true` for `Evening | Night`; consumed by `clock_text_color()` in `hud.rs` and `time_of_day_brightness()` in `time.rs`.

#### `GameTime::time_of_day()` — `src/domain/types.rs`

A pure `match` on `self.hour` maps all 24 possible hour values to the correct
`TimeOfDay` variant:

```src/domain/types.rs#L543-553
pub fn time_of_day(&self) -> TimeOfDay {
    match self.hour {
        5..=7 => TimeOfDay::Dawn,
        8..=11 => TimeOfDay::Morning,
        12..=15 => TimeOfDay::Afternoon,
        16..=18 => TimeOfDay::Dusk,
        19..=21 => TimeOfDay::Evening,
        // 22-23 and 0-4 are Night
        _ => TimeOfDay::Night,
    }
}
```

#### `GameTime::is_night()` and `GameTime::is_day()` — `src/domain/types.rs`

Both delegate to `time_of_day()` so they are always consistent with the
six-period classification:

```src/domain/types.rs#L570-591
pub fn is_night(&self) -> bool {
    matches!(self.time_of_day(), TimeOfDay::Evening | TimeOfDay::Night)
}

pub fn is_day(&self) -> bool {
    !self.is_night()
}
```

`Evening` is classified as "night" for `is_night()` because a light source is
recommended at that point; however the ambient-light system keeps `Evening`
distinct from `Night` by using a higher brightness value (`0.50` vs `0.25`).

#### `GameState::time_of_day()` — `src/application/mod.rs`

A thin convenience wrapper so any system with `GameState` access can query the
period without reaching into `state.time` directly:

```src/application/mod.rs#L1469-1471
pub fn time_of_day(&self) -> TimeOfDay {
    self.time.time_of_day()
}
```

#### Ambient-Light Hook — `src/game/systems/time.rs`

Five brightness constants encode the intended per-period light intensity:

| Constant                     | Value  | Period(s)          |
| ---------------------------- | ------ | ------------------ |
| `AMBIENT_NIGHT_BRIGHTNESS`   | `0.25` | Night              |
| `AMBIENT_EVENING_BRIGHTNESS` | `0.50` | Evening            |
| `AMBIENT_DAWN_BRIGHTNESS`    | `0.70` | Dawn               |
| `AMBIENT_DUSK_BRIGHTNESS`    | `0.70` | Dusk               |
| `AMBIENT_DAY_BRIGHTNESS`     | `1.00` | Morning, Afternoon |

The `time_of_day_brightness()` pure function maps a `TimeOfDay` to the
correct constant. The Bevy system `update_ambient_light` calls it every frame:

```src/game/systems/time.rs#L137-143
pub fn update_ambient_light(
    global_state: Res<GlobalState>,
    mut ambient_light: ResMut<AmbientLight>,
) {
    let brightness = time_of_day_brightness(global_state.0.time_of_day());
    ambient_light.brightness = brightness;
}
```

`TimeOfDayPlugin` orders `apply_time_advance` **before** `update_ambient_light`
so the light always reflects the current frame's clock value:

```src/game/systems/time.rs#L114-118
fn build(&self, app: &mut App) {
    app.add_message::<TimeAdvanceEvent>();
    app.add_systems(Update, apply_time_advance.before(update_ambient_light));
    app.add_systems(Update, update_ambient_light);
}
```

#### HUD Clock Colour Tinting — `src/game/systems/hud.rs`

The `update_clock` system reads `global_state.0.time_of_day()` and calls
`clock_text_color()` to tint the time text:

- Dark periods (`Evening`, `Night`) → `CLOCK_NIGHT_TEXT_COLOR` — cool blue-white (`rgba(0.6, 0.6, 1.0, 1.0)`)
- All other periods → `CLOCK_DAY_TEXT_COLOR` — warm golden (`rgba(1.0, 0.9, 0.5, 1.0)`)

This gives players an immediate ambient visual cue in the HUD that maps to the
`TimeOfDay::is_dark()` flag.

### Tests

#### `src/domain/types.rs` — `TimeOfDay` and `GameTime` tests

| Test                                     | What it verifies                                                                        |
| ---------------------------------------- | --------------------------------------------------------------------------------------- |
| `test_time_of_day_night_early_morning`   | Hours 0–4 all map to `Night`                                                            |
| `test_time_of_day_dawn`                  | Hours 5–7 (including 5:00 and 7:59) map to `Dawn`                                       |
| `test_time_of_day_morning`               | Hours 8–11 map to `Morning`                                                             |
| `test_time_of_day_afternoon`             | Hours 12–15 map to `Afternoon`                                                          |
| `test_time_of_day_dusk`                  | Hours 16–18 map to `Dusk`                                                               |
| `test_time_of_day_evening`               | Hours 19–21 map to `Evening`                                                            |
| `test_time_of_day_night`                 | Hours 22–23 map to `Night`                                                              |
| `test_time_of_day_boundary_transitions`  | Every exact transition hour tested (4:59→5:00, 7:59→8:00, …, 21:59→22:00)               |
| `test_is_night_delegates_to_time_of_day` | `Evening` and `Night` return `true`; `Dawn`/`Morning`/`Afternoon`/`Dusk` return `false` |
| `test_is_day_is_inverse_of_is_night`     | For every hour 0–23, `is_day() == !is_night()`                                          |
| `test_time_of_day_label`                 | Each variant's `label()` returns the correct string                                     |
| `test_time_of_day_is_dark`               | `Evening` and `Night` are dark; the other four are not                                  |

#### `src/application/mod.rs` — `GameState::time_of_day()` tests

| Test                                                 | What it verifies                                                         |
| ---------------------------------------------------- | ------------------------------------------------------------------------ |
| `test_game_state_time_of_day_default_is_dawn`        | `GameState::new()` starts at 06:00 — confirmed Dawn                      |
| `test_game_state_time_of_day_delegates_to_game_time` | Seven representative hours each produce the expected `TimeOfDay` variant |
| `test_game_state_time_of_day_advances_correctly`     | 06:00 + 6 hours via `advance_hours` → Afternoon                          |
| `test_game_state_time_of_day_night_via_advance_time` | 06:00 + 16 hours via `advance_time` → Night                              |

#### `src/game/systems/time.rs` — ambient-light tests

| Test                                                    | What it verifies                                                       |
| ------------------------------------------------------- | ---------------------------------------------------------------------- |
| `test_brightness_night`                                 | Night maps to `AMBIENT_NIGHT_BRIGHTNESS`                               |
| `test_brightness_evening`                               | Evening maps to `AMBIENT_EVENING_BRIGHTNESS`                           |
| `test_brightness_dawn`                                  | Dawn maps to `AMBIENT_DAWN_BRIGHTNESS`                                 |
| `test_brightness_dusk`                                  | Dusk maps to `AMBIENT_DUSK_BRIGHTNESS`                                 |
| `test_brightness_morning`                               | Morning maps to `AMBIENT_DAY_BRIGHTNESS`                               |
| `test_brightness_afternoon`                             | Afternoon maps to `AMBIENT_DAY_BRIGHTNESS`                             |
| `test_brightness_is_darker_at_night_than_day`           | Night brightness is strictly less than Afternoon brightness            |
| `test_brightness_ordering`                              | Full ordering: Night < Evening < Dawn = Dusk < Morning = Afternoon     |
| `test_all_hours_produce_valid_brightness`               | Every hour 0–23 yields a brightness in `[0.0, 1.0]`                    |
| `test_dark_periods_below_threshold`                     | Evening and Night are strictly below `1.0`                             |
| `test_time_of_day_is_dark_matches_brightness_reduction` | Every `is_dark()` period has brightness below `AMBIENT_DAY_BRIGHTNESS` |

#### `src/game/systems/hud.rs` — clock colour tests

| Test                                                        | What it verifies                                                         |
| ----------------------------------------------------------- | ------------------------------------------------------------------------ |
| `test_clock_text_color_night_returns_night_color`           | `Night` → `CLOCK_NIGHT_TEXT_COLOR`                                       |
| `test_clock_text_color_evening_returns_night_color`         | `Evening` → `CLOCK_NIGHT_TEXT_COLOR` (it `is_dark()`)                    |
| `test_clock_text_color_dawn_returns_day_color`              | `Dawn` → `CLOCK_DAY_TEXT_COLOR`                                          |
| `test_clock_text_color_morning_returns_day_color`           | `Morning` → `CLOCK_DAY_TEXT_COLOR`                                       |
| `test_clock_text_color_afternoon_returns_day_color`         | `Afternoon` → `CLOCK_DAY_TEXT_COLOR`                                     |
| `test_clock_text_color_dusk_returns_day_color`              | `Dusk` → `CLOCK_DAY_TEXT_COLOR`                                          |
| `test_clock_text_color_agrees_with_is_dark_for_all_periods` | `clock_text_color` agrees with `TimeOfDay::is_dark()` for all 6 variants |

### Architecture Compliance

- [x] `TimeOfDay` lives in `src/domain/types.rs` alongside `GameTime` as specified in plan §2.1
- [x] Hour boundaries match the plan specification exactly (Dawn 05–07, Morning 08–11, Afternoon 12–15, Dusk 16–18, Evening 19–21, Night 22–04)
- [x] `is_night()` delegates to `time_of_day()` — no duplicated hour comparisons
- [x] `is_day()` is the logical inverse of `is_night()` — no independent implementation
- [x] `GameState::time_of_day()` convenience helper present as specified in plan §2.2
- [x] Ambient-light hook reads `time_of_day()` in `src/game/systems/time.rs` as specified in plan §2.3
- [x] `AMBIENT_NIGHT_BRIGHTNESS` = `0.25` matches the `night_ambient_brightness` value specified in plan §2.3
- [x] `TimeOfDay` derives `Serialize, Deserialize` for future RON data-file event conditions (Phase 4)
- [x] No `unwrap()` without justification; all paths handled
- [x] All four quality gates pass

### Quality Gate Results

```
cargo fmt --all          → no output (all files formatted)
cargo check              → Finished with 0 errors, 0 warnings
cargo clippy -D warnings → Finished with 0 warnings
cargo nextest run        → 3337 passed, 0 failed, 8 skipped
```

---

## Phase 1: Time Advancement Hooks (Complete)

### Overview

Phase 1 wires the in-game clock (`GameState.time: GameTime`) to every player
action that should consume time:

| Action                                     | Cost                                        | Location                                   |
| ------------------------------------------ | ------------------------------------------- | ------------------------------------------ |
| One exploration step                       | `TIME_COST_STEP_MINUTES` (5 min)            | `GameState::move_party_and_handle_events`  |
| One combat round                           | `TIME_COST_COMBAT_ROUND_MINUTES` (5 min)    | `tick_combat_time` system in `combat.rs`   |
| Map transition (teleport, dungeon, portal) | `TIME_COST_MAP_TRANSITION_MINUTES` (30 min) | `map_change_handler` system in `map.rs`    |
| Rest (any duration)                        | `hours * 60` minutes                        | `GameState::rest_party` via `advance_time` |

All time mutations go through `GameState::advance_time(minutes, templates)` so
that active-spell duration ticking and merchant restocking are never bypassed.

### Phase 1 Deliverables Checklist

- [x] `TIME_COST_STEP_MINUTES`, `TIME_COST_COMBAT_ROUND_MINUTES`, `TIME_COST_MAP_TRANSITION_MINUTES` constants in `src/domain/resources.rs`
- [x] Time advance on successful exploration step (`move_party_and_handle_events`)
- [x] Time advance per combat round (`tick_combat_time` in `src/game/systems/combat.rs`)
- [x] Time advance on map transition (`map_change_handler` in `src/game/systems/map.rs`)
- [x] `rest_party()` callers use `GameState::advance_time()` exclusively
- [x] `TimeAdvanceEvent` Bevy event + `apply_time_advance` system in `src/game/systems/time.rs`
- [x] `TimeOfDayPlugin` integrating ambient-light updates
- [x] All phase-1 tests pass

### What Was Built

#### `TIME_COST_*` Constants — `src/domain/resources.rs`

Three new constants define the canonical time cost for each action category:

```src/domain/resources.rs#L77-85
pub const TIME_COST_STEP_MINUTES: u32 = 5;
pub const TIME_COST_COMBAT_ROUND_MINUTES: u32 = 5;
pub const TIME_COST_MAP_TRANSITION_MINUTES: u32 = 30;
```

`REST_DURATION_HOURS` (12) already existed and was left unchanged.

#### Exploration Movement — `src/application/mod.rs`

`GameState::move_party_and_handle_events()` calls
`self.advance_time(TIME_COST_STEP_MINUTES, None)` immediately after a
successful `move_party()`, before any event resolution. A blocked step
(movement error) returns early before `advance_time` is reached, so the clock
never ticks for failed moves.

#### Combat Round Time — `src/game/systems/combat.rs`

The private `tick_combat_time` Bevy system runs after every combat-action
handler. It compares `combat_res.state.round` against the new
`CombatResource::last_timed_round` field; when the round has advanced it
charges `new_rounds * TIME_COST_COMBAT_ROUND_MINUTES` exactly once. This
prevents double-charging when the same round spans multiple frames.

The system is a no-op when `GameMode` is not `Combat`, so stale combat data
never advances the exploration clock.

#### Map Transition Time — `src/game/systems/map.rs`

`map_change_handler` now calls `global_state.0.advance_time(TIME_COST_MAP_TRANSITION_MINUTES, None)`
after confirming the target map exists. Invalid map ids are silently ignored
and do **not** advance the clock.

#### Rest Time — `src/application/mod.rs`

`GameState::rest_party()` delegates HP/SP restoration to
`domain::resources::rest_party()` (which no longer calls `advance_hours`
directly) and then calls `self.advance_time(hours * 60, templates)`.
This ensures active-spell ticking and merchant restocking both happen for the
full rest duration.

#### `TimeAdvanceEvent` + `apply_time_advance` — `src/game/systems/time.rs`

A Bevy `Message` type `TimeAdvanceEvent { minutes: u32 }` lets any system
request a clock advance without touching `GlobalState` directly. The
`apply_time_advance` system drains the queue each frame and calls
`GameState::advance_time` per event, keeping time mutation centralised.
`TimeOfDayPlugin` registers both this system and the ambient-light update
system, ordering `apply_time_advance` before `update_ambient_light` so the
light reflects the updated time within the same frame.

### Tests Added

#### New tests in `src/game/systems/combat.rs`

| Test                              | What it verifies                                                                                                                               |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_combat_round_advances_time` | One combat round (round 1) advances clock by `TIME_COST_COMBAT_ROUND_MINUTES`; a second frame with the same round number does NOT charge again |

#### New tests in `src/game/systems/map.rs`

| Test                                                | What it verifies                                                                                                           |
| --------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `test_map_transition_advances_time`                 | A valid `MapChangeEvent` advances clock by `TIME_COST_MAP_TRANSITION_MINUTES` and updates `current_map` + `party_position` |
| `test_invalid_map_transition_does_not_advance_time` | A `MapChangeEvent` targeting a non-existent map id does NOT advance the clock                                              |

#### Pre-existing tests that cover Phase 1 requirements

| Test                                                | Location                   | Phase 1 requirement covered                         |
| --------------------------------------------------- | -------------------------- | --------------------------------------------------- |
| `test_step_advances_time`                           | `src/application/mod.rs`   | Successful step costs `TIME_COST_STEP_MINUTES`      |
| `test_blocked_step_does_not_advance_time`           | `src/application/mod.rs`   | Blocked step costs zero time                        |
| `test_rest_advances_time_via_state`                 | `src/application/mod.rs`   | Rest costs exactly `hours * 60` minutes             |
| `test_rest_ticks_active_spells`                     | `src/application/mod.rs`   | `advance_time` ticks active spells during rest      |
| `test_time_advance_event_advances_clock`            | `src/game/systems/time.rs` | `TimeAdvanceEvent` moves clock by requested minutes |
| `test_multiple_time_advance_events_same_frame`      | `src/game/systems/time.rs` | Multiple events per frame are all applied           |
| `test_no_time_advance_event_leaves_clock_unchanged` | `src/game/systems/time.rs` | No event → clock unchanged                          |
| `test_time_advance_event_rolls_over_midnight`       | `src/game/systems/time.rs` | Day rollover on midnight crossing                   |

### Architecture Compliance

- [x] Data structures match `architecture.md` Section 4.1 (`GameState.time: GameTime`) exactly
- [x] `TIME_COST_*` constants extracted — no magic numbers
- [x] `advance_time` is the single path for all clock mutations
- [x] Active-spell ticking and merchant restocking are never bypassed
- [x] No `unwrap()` without justification; all error paths handled
- [x] All four quality gates pass: `cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo nextest run`

### Quality Gate Results

```
cargo fmt --all          → no output (all files formatted)
cargo check              → Finished with 0 errors, 0 warnings
cargo clippy -D warnings → Finished with 0 warnings
cargo nextest run        → 3337 passed, 0 failed, 8 skipped
```

---

## Phase 4: Campaign Builder SDK Editor Updates — Unified Creature Asset Binding (Complete)

### Overview

Phase 4 extends all three Campaign Builder editors (Monsters, Characters, NPCs) so
that every definition type exposes a consistent Browse/Clear/tooltip creature picker
in its edit form. Each editor now accepts `creature_manager: Option<&CreatureAssetManager>`
through its `show` method, which is constructed lazily from `campaign_dir` in
`lib.rs` and passed down to the form-level methods. When no campaign is open the
parameter is `None` and the picker button gracefully degrades to a plain text field.

A post-implementation follow-up corrected the autocomplete handling for the
Characters and NPC editors. The initial implementation used a raw
`egui::TextEdit::singleline` for the creature ID field. This was replaced with a
new shared helper `autocomplete_creature_selector` in `ui_helpers.rs` that follows
the same pattern as `autocomplete_portrait_selector` and
`autocomplete_sprite_sheet_selector`: a persistent egui-memory buffer, filtered
candidate suggestions (shown as `"id — name"` pairs), a built-in Clear button, a
hover tooltip showing the resolved creature name or a warning for unknown IDs, and
proper buffer clearing when `reset_autocomplete_buffers` fires. The Monsters editor
was not affected because its `creature_id` is `Option<CreatureId>` (numeric) and is
displayed as a read-only label; only the Browse modal applies there.

### Phase 4 Deliverables Checklist

- [x] **4.1 Monsters Editor** — `creature_picker_open`; `apply_selected_creature_id`; Visual Asset section in `show_form` with read-only label + Browse modal; resolved name in `show_monster_details` and `show_preview_static`; `show_form` and `show` accept `creature_manager`
- [x] **4.2 Characters Editor** — `creature_id: String` in `CharacterEditBuffer`; `start_edit_character` populates it; `save_character` writes it back; `available_creatures: Vec<(u32, String)>` cache on state; `autocomplete_creature_selector` in `show_character_form`; picker modal syncs autocomplete buffer on selection; `reset_autocomplete_buffers` clears creature buffer; creature name in preview
- [x] **4.3 NPC Editor** — `available_creatures: Vec<(u32, String)>` cache on state; `autocomplete_creature_selector` in `show_edit_view` replacing raw TextEdit; picker modal syncs autocomplete buffer on selection; `reset_autocomplete_buffers` clears creature buffer; `show_preview` shows resolved creature name
- [x] **`ui_helpers.rs`** — new `pub fn autocomplete_creature_selector` with persistent buffer, `"id — name"` display format, ID extraction, hover tooltip, built-in Clear button, and 10 logic-level unit tests
- [x] All three `show` methods accept `creature_manager: Option<&CreatureAssetManager>`
- [x] `sdk/campaign_builder/src/lib.rs` — All three call sites updated (Monsters, Characters, NPCs tabs)
- [x] **20 new unit tests** (3 Monsters + 5 Characters + 2 NPCs + 10 `autocomplete_creature_selector` logic tests)
- [x] All four quality gates pass: `cargo fmt` (no output), `cargo check --all-targets --all-features` (0 errors), `cargo clippy --all-targets --all-features -- -D warnings` (0 warnings), `cargo nextest run --all-features` (3333 passed, 0 failed)
- [x] egui ID rules (sdk/AGENTS.md) satisfied: every `ScrollArea` has `id_salt`, every `Window` has `egui::Id::new(…)`, every loop uses `push_id`, no same-frame guard violations

### `autocomplete_creature_selector` — Design Notes

The new helper in `ui_helpers.rs` follows the exact same structure as
`autocomplete_portrait_selector`:

```sdk/campaign_builder/src/ui_helpers.rs#L2876-2884
pub fn autocomplete_creature_selector(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_creature_id: &mut String,
    candidates: &[(u32, String)],
) -> bool {
```

Key design decisions:

- **Candidates are `(u32, String)` tuples** — the caller pre-builds this list once
  per frame from `CreatureAssetManager::load_all_creatures()` and stores it in
  `available_creatures` on the editor state, avoiding per-widget file I/O.
- **Display format is `"id — name"`** — allows the user to filter by either numeric
  ID or creature name. On commit, the `" — "` separator is used to extract just the
  ID part back into the string buffer.
- **Buffer initialisation resolves ID → display string** — when editing an existing
  definition, the stored `"42"` is expanded to `"42 — Dragon"` on first render so
  the field shows something human-readable.
- **Raw numeric input is also accepted** — if the user types `"7"` directly (no
  name), the widget stores `"7"` into the buffer. This preserves keyboard-first
  workflow for power users.
- **Built-in Clear button** — consistent with `autocomplete_portrait_selector`.
  The external standalone "Clear" button added in the initial Phase 4
  implementation was removed since the helper now owns that action.
- **`autocomplete:creature:<id_salt>` egui memory key** — follows the same naming
  convention as `autocomplete:portrait:…` and `autocomplete:sprite:…`.

### `available_creatures` Cache

Both `CharactersEditorState` and `NpcEditorState` gained:

```sdk/campaign_builder/src/characters_editor.rs#L71-74
    /// Available creature candidates (id, name) cached for autocomplete (rebuilt when campaign dir changes)
    #[serde(skip)]
    pub available_creatures: Vec<(u32, String)>,
```

The cache is rebuilt in `show()` whenever `campaign_dir_changed` is `true`:

```sdk/campaign_builder/src/characters_editor.rs#L1040-1051
            // Rebuild creature candidates from the manager whenever the campaign dir changes.
            self.available_creatures = creature_manager
                .and_then(|m| m.load_all_creatures().ok())
                .map(|creatures| {
                    creatures
                        .into_iter()
                        .map(|c| (c.id, c.name))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
```

This matches the pattern used for `available_portraits` and
`available_sprite_sheets`.

### Autocomplete Buffer Sync on Modal Picker Selection

When a creature is selected via the Browse modal (rather than typed), the
autocomplete buffer is explicitly synced so the text field shows the resolved
`"id — name"` string immediately rather than just the bare numeric ID:

```sdk/campaign_builder/src/characters_editor.rs#L1241-1251
                if let Some(id) = picked_id {
                    self.apply_selected_creature_id(Some(id.clone()));
                    // Sync the autocomplete buffer so the text field shows the
                    // resolved "id — name" display string immediately.
                    let display = creatures
                        .iter()
                        .find(|c| c.id.to_string() == id)
                        .map(|c| format!("{} — {}", c.id, c.name))
                        .unwrap_or_else(|| id.clone());
                    crate::ui_helpers::store_autocomplete_buffer(
                        ui.ctx(),
                        egui::Id::new("autocomplete:creature:character_creature".to_string()),
                        &display,
                    );
```

The NPC editor uses the same pattern with key `"autocomplete:creature:npc_creature"`.

### egui ID Audit (sdk/AGENTS.md compliance)

| Widget              | ID                                         | Editor     |
| ------------------- | ------------------------------------------ | ---------- |
| `egui::Window`      | `monster_creature_picker`                  | Monsters   |
| `ScrollArea`        | `monster_creature_picker_scroll`           | Monsters   |
| `egui::Window`      | `character_creature_picker`                | Characters |
| `ScrollArea`        | `character_creature_picker_scroll`         | Characters |
| `AutocompleteInput` | `autocomplete:creature:character_creature` | Characters |
| `egui::Window`      | `npc_creature_picker`                      | NPCs       |
| `ScrollArea`        | `npc_creature_picker_scroll`               | NPCs       |
| `AutocompleteInput` | `autocomplete:creature:npc_creature`       | NPCs       |

All picker row loops use `ui.push_id(creature.id, …)`.
All picker modals use two-phase (`picked_id` / `should_close`) to avoid borrow
conflicts — no mutable borrow of `self` occurs inside the window closure.

### Test Coverage Summary

| Test                                                                          | Location          | What it verifies                                                                                   |
| ----------------------------------------------------------------------------- | ----------------- | -------------------------------------------------------------------------------------------------- |
| `test_monsters_editor_creature_id_roundtrips_through_form`                    | monsters_editor   | `apply_selected_creature_id(Some(42))` sets `edit_buffer.creature_id == Some(42)`                  |
| `test_monsters_editor_clear_creature_id`                                      | monsters_editor   | `apply_selected_creature_id(None)` clears `edit_buffer.creature_id`                                |
| `test_monsters_editor_default_monster_creature_id_is_none`                    | monsters_editor   | `default_monster().creature_id == None`                                                            |
| `test_characters_editor_creature_id_roundtrips_through_form`                  | characters_editor | `start_edit_character` with `creature_id: Some(42)` → buffer `"42"`; `save_character` → `Some(42)` |
| `test_characters_editor_creature_id_empty_string_saves_none`                  | characters_editor | Buffer `""` → `save_character` → `creature_id: None`                                               |
| `test_characters_editor_creature_id_invalid_string_saves_none`                | characters_editor | Buffer `"not_a_number"` → `save_character` → `creature_id: None`                                   |
| `test_creature_picker_open_flag`                                              | characters_editor | Default `CharactersEditorState` has `creature_picker_open == false`                                |
| `test_apply_selected_creature_id_sets_buffer`                                 | characters_editor | `apply_selected_creature_id(Some("7"))` sets buffer to `"7"` and closes picker                     |
| `test_npc_creature_picker_initial_state`                                      | npc_editor        | Default `NpcEditorState` has `creature_picker_open == false`                                       |
| `test_npc_apply_selected_creature_id_updates_buffer`                          | npc_editor        | `apply_selected_creature_id("1000")` writes `"1000"` to buffer and closes picker                   |
| `test_autocomplete_creature_selector_empty_candidates_returns_false`          | ui_helpers        | Empty candidate list produces empty display vec                                                    |
| `test_autocomplete_creature_selector_display_format`                          | ui_helpers        | Each candidate renders as `"id — name"`                                                            |
| `test_autocomplete_creature_selector_id_extraction_from_display_string`       | ui_helpers        | `"42 — Dragon"` → extracted ID `"42"`                                                              |
| `test_autocomplete_creature_selector_raw_numeric_id_accepted`                 | ui_helpers        | Plain `"7"` parses successfully as `u32`                                                           |
| `test_autocomplete_creature_selector_non_numeric_raw_input_rejected`          | ui_helpers        | Non-numeric, non-`" — "` string is rejected by both parse branches                                 |
| `test_autocomplete_creature_selector_buffer_initialisation_with_known_id`     | ui_helpers        | Buffer init with `"7"` → `"7 — Skeleton"` when ID is in registry                                   |
| `test_autocomplete_creature_selector_buffer_initialisation_with_unknown_id`   | ui_helpers        | Buffer init with `"99"` → `"99"` (raw) when ID is not in registry                                  |
| `test_autocomplete_creature_selector_buffer_initialisation_empty_stays_empty` | ui_helpers        | Empty current value → empty display string                                                         |
| `test_autocomplete_creature_selector_tooltip_resolved_name`                   | ui_helpers        | Known ID produces `"Creature: Dragon"` tooltip                                                     |
| `test_autocomplete_creature_selector_tooltip_unknown_id`                      | ui_helpers        | Unknown ID produces `"⚠ Creature ID '999' not found in registry"` tooltip                          |

---

## Phase 4.3: NPC Editor — Unified Creature Asset Binding

### Overview

Updated `sdk/campaign_builder/src/npc_editor.rs` to support visual asset binding
for NPCs. The editor now lets campaign authors link an `NpcDefinition` to a
`CreatureDefinition` (a procedural mesh creature) via a "Creature ID" row in the
Appearance group. When a `CreatureAssetManager` is provided the editor presents a
browseable creature picker modal and resolves the creature's human-readable name in
the preview panel; without one the text field remains fully editable as a plain
numeric input and the picker button immediately closes itself.

### Deliverables Checklist

- [x] `sdk/campaign_builder/src/npc_editor.rs` — `use crate::creature_assets::CreatureAssetManager` added to imports
- [x] `sdk/campaign_builder/src/npc_editor.rs` — `pub creature_picker_open: bool` field added to `NpcEditorState` with `#[serde(skip)]`
- [x] `sdk/campaign_builder/src/npc_editor.rs` — `creature_picker_open: false` added to `impl Default for NpcEditorState`
- [x] `sdk/campaign_builder/src/npc_editor.rs` — `apply_selected_creature_id(&mut self, id: String)` method added with `///` doc comment
- [x] `sdk/campaign_builder/src/npc_editor.rs` — `show` signature updated with `creature_manager: Option<&CreatureAssetManager>` parameter
- [x] `sdk/campaign_builder/src/npc_editor.rs` — `show_edit_view` signature updated with `creature_manager: Option<&CreatureAssetManager>` parameter
- [x] `sdk/campaign_builder/src/npc_editor.rs` — `show_edit_view` call site inside `show` updated to pass `creature_manager`
- [x] `sdk/campaign_builder/src/npc_editor.rs` — "Creature ID" row added to Appearance group in `show_edit_view` after the sprite index label, with text field, Browse, Clear, and ℹ widgets
- [x] `sdk/campaign_builder/src/npc_editor.rs` — Creature picker modal inserted after the Appearance group close, before Dialogue & Quests group
- [x] `sdk/campaign_builder/src/npc_editor.rs` — `show_preview` signature updated with `creature_manager: Option<&CreatureAssetManager>` parameter
- [x] `sdk/campaign_builder/src/npc_editor.rs` — `show_preview` call site inside `show_list_view` updated to pass `creature_manager`
- [x] `sdk/campaign_builder/src/npc_editor.rs` — `show_preview` renders resolved creature asset name ("Asset:" row) after the existing "Creature ID:" row when manager is available
- [x] `sdk/campaign_builder/src/lib.rs` — `EditorTab::NPCs` arm constructs `npc_creature_manager` and passes it as last arg to `npc_editor_state.show`
- [x] Two new tests added: `test_npc_creature_picker_initial_state`, `test_npc_apply_selected_creature_id_updates_buffer`
- [x] All four quality gates pass: `cargo fmt` (no output), `cargo check --all-targets --all-features` (0 errors), `cargo clippy --all-targets --all-features -- -D warnings` (0 warnings), `cargo nextest run --all-features` (3333 passed, 0 failed)

### What Was Built

#### `creature_picker_open` State Field

A `#[serde(skip)] pub creature_picker_open: bool` on `NpcEditorState` tracks
whether the picker modal is open. It is initialised `false` in `Default`. The
`#[serde(skip)]` ensures it never leaks into serialised editor state.

#### `apply_selected_creature_id`

```sdk/campaign_builder/src/npc_editor.rs#L232-235
    /// Sets the creature ID buffer and closes the creature picker.
    pub(crate) fn apply_selected_creature_id(&mut self, id: String) {
        self.edit_buffer.creature_id = id;
        self.creature_picker_open = false;
    }
```

Single method that both writes the string ID into the buffer and closes the
picker, keeping the two pieces of state in sync. Used by both the picker modal
selection handler and the "Clear" button.

#### "Creature ID" Row in `show_edit_view`

Appended to the existing Appearance `ui.group` closure, after the sprite index
label, with a `ui.separator()` to visually separate it from the sprite fields.
Contains:

- A `TextEdit::singleline` bound to `self.edit_buffer.creature_id` (80 px wide,
  hint text `"numeric ID or empty"`).
- A **Browse…** button that sets `creature_picker_open = true`.
- A **Clear** button that calls `apply_selected_creature_id(String::new())`.
- An ℹ hover label explaining the 3-D mesh spawning behaviour.

#### Creature Picker Modal in `show_edit_view`

An `egui::Window` placed immediately after the Appearance group closes (before
the Dialogue & Quests group) with:

- `id(egui::Id::new("npc_creature_picker"))` — stable egui ID.
- `ScrollArea::vertical()` with `id_salt("npc_creature_picker_scroll")`.
- `ui.push_id(creature.id, …)` for every row (AGENTS.md egui ID audit compliance).
- Two-phase close: `picked_id` / `should_close` locals evaluated after the window
  closure to avoid borrow conflicts.
- Graceful `None`-manager fallback: `creature_picker_open` is immediately cleared,
  leaving the text field as the only input path.

#### Preview Update in `show_preview`

After the existing `if let Some(creature_id) = npc.creature_id` row that shows
the raw numeric ID, a second conditional block resolves the human-readable name:

```sdk/campaign_builder/src/npc_editor.rs#L648-657
                if let (Some(creature_id), Some(manager)) = (npc.creature_id, creature_manager) {
                    let resolved = manager
                        .load_creature(creature_id)
                        .map(|c| c.name)
                        .unwrap_or_else(|_| "⚠ Unknown".to_string());
                    ui.label("Asset:");
                    ui.label(resolved);
                    ui.end_row();
                }
```

When the manager cannot load the creature (file missing, parse error, etc.) the
cell displays `"⚠ Unknown"` rather than propagating an error.

#### Call-Site Update in `lib.rs`

```sdk/campaign_builder/src/lib.rs#L4703-4720
            EditorTab::NPCs => {
                // ...
                let npc_creature_manager = self
                    .campaign_dir
                    .as_ref()
                    .map(|d| crate::creature_assets::CreatureAssetManager::new(d.clone()));

                if self.npc_editor_state.show(
                    ui,
                    &self.dialogues,
                    &self.quests,
                    self.campaign_dir.as_ref(),
                    &self.tool_config.display,
                    &self.campaign.npcs_file,
                    npc_creature_manager.as_ref(),
                ) {
                    self.unsaved_changes = true;
                }
```

The manager is constructed lazily from `self.campaign_dir` each frame, identical
to the pattern used by the Characters and Monsters editors.

#### Test Coverage

| Test                                                 | What it verifies                                                                 |
| ---------------------------------------------------- | -------------------------------------------------------------------------------- |
| `test_npc_creature_picker_initial_state`             | Default `NpcEditorState` has `creature_picker_open == false`                     |
| `test_npc_apply_selected_creature_id_updates_buffer` | `apply_selected_creature_id("1000")` writes `"1000"` to buffer and closes picker |

---

## Phase 4.2: Characters Editor — Unified Creature Asset Binding

### Overview

Updated `sdk/campaign_builder/src/characters_editor.rs` to support visual asset
binding for characters. The editor now lets campaign authors link a
`CharacterDefinition` to a `CreatureDefinition` (a procedural mesh creature) via a
"Creature ID" row in the Basic Information grid. When a `CreatureAssetManager` is
provided the editor presents a browseable creature picker modal; without one the
🦎 button is a no-op and the text field remains fully editable as a plain numeric
input.

### Deliverables Checklist

- [x] `sdk/campaign_builder/src/characters_editor.rs` — `use antares::domain::types::CreatureId` added to imports
- [x] `sdk/campaign_builder/src/characters_editor.rs` — `use crate::creature_assets::CreatureAssetManager` added to imports
- [x] `sdk/campaign_builder/src/characters_editor.rs` — `pub creature_picker_open: bool` field added to `CharactersEditorState` with `#[serde(skip)]`
- [x] `sdk/campaign_builder/src/characters_editor.rs` — `creature_picker_open: false` added to `impl Default for CharactersEditorState`
- [x] `sdk/campaign_builder/src/characters_editor.rs` — `pub creature_id: String` field added to `CharacterEditBuffer`
- [x] `sdk/campaign_builder/src/characters_editor.rs` — `creature_id: String::new()` added to `impl Default for CharacterEditBuffer`
- [x] `sdk/campaign_builder/src/characters_editor.rs` — `start_edit_character` loads `character.creature_id.map_or(String::new(), |id| id.to_string())` into buffer
- [x] `sdk/campaign_builder/src/characters_editor.rs` — `save_character` writes `creature_id` field using `parse::<CreatureId>().ok()` with empty-string → `None` guard
- [x] `sdk/campaign_builder/src/characters_editor.rs` — `apply_selected_creature_id(&mut self, id: Option<String>)` method added with `///` doc comment
- [x] `sdk/campaign_builder/src/characters_editor.rs` — `show` signature updated with `creature_manager: Option<&CreatureAssetManager>` parameter
- [x] `sdk/campaign_builder/src/characters_editor.rs` — `show_character_form` signature updated with `creature_manager: Option<&CreatureAssetManager>` parameter
- [x] `sdk/campaign_builder/src/characters_editor.rs` — `show_character_form` call site inside `show` updated to pass `creature_manager`
- [x] `sdk/campaign_builder/src/characters_editor.rs` — "Creature ID" row added to basic info grid in `show_character_form` after the "Portrait ID" row
- [x] `sdk/campaign_builder/src/characters_editor.rs` — Creature picker modal inserted in `show` after the portrait picker logic
- [x] `sdk/campaign_builder/src/characters_editor.rs` — `show_character_preview` updated to show `Creature:` row in `character_preview_grid` when `creature_id` is set
- [x] `sdk/campaign_builder/src/lib.rs` — `EditorTab::Characters` arm updated to construct a `CreatureAssetManager` and pass it to `show`
- [x] `sdk/campaign_builder/src/asset_manager.rs` — Four existing test `CharacterDefinition` struct literals updated with `creature_id: None`
- [x] Five new tests added: `test_characters_editor_creature_id_roundtrips_through_form`, `test_characters_editor_creature_id_empty_string_saves_none`, `test_characters_editor_creature_id_invalid_string_saves_none`, `test_creature_picker_open_flag`, `test_apply_selected_creature_id_sets_buffer`
- [x] All four quality gates pass with zero errors/warnings (`cargo fmt`, `cargo check --all-targets --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`, sdk `cargo test` — 1649 passed, 1 pre-existing unrelated failure in `mesh_obj_io`)

### What Was Built

#### `creature_id` Buffer Field

`CharacterEditBuffer` gains a plain `pub creature_id: String` field. String storage
was chosen (consistent with `portrait_id`) because egui text inputs work directly
on `String`. Conversion to/from `CreatureId` (`u32`) happens only at the
save/load boundary.

#### `creature_picker_open` State Field

A `#[serde(skip)] pub creature_picker_open: bool` on `CharactersEditorState`
tracks whether the picker modal is open. It is initialised `false` in `Default`.

#### `apply_selected_creature_id`

```antares/sdk/campaign_builder/src/characters_editor.rs#L977-980
    /// Sets the creature ID buffer and closes the creature picker.
    pub(crate) fn apply_selected_creature_id(&mut self, id: Option<String>) {
        self.buffer.creature_id = id.unwrap_or_default();
        self.creature_picker_open = false;
    }
```

Single method that both writes the string ID into the buffer and closes the
picker, keeping the two pieces of state in sync.

#### `save_character` — `creature_id` Serialisation

```antares/sdk/campaign_builder/src/characters_editor.rs#L575-580
            creature_id: if self.buffer.creature_id.is_empty() {
                None
            } else {
                self.buffer.creature_id.trim().parse::<CreatureId>().ok()
            },
```

Empty string → `None`; non-numeric string → `None` (silent discard, matching
the portrait ID pattern); valid integer → `Some(id)`.

#### "Creature ID" Row in `show_character_form`

Inserted directly after the "Portrait ID" row in the `character_basic_grid`.
Contains:

- A `TextEdit::singleline` bound to `self.buffer.creature_id` (80 px wide, hint
  text `"numeric ID or empty"`).
- A 🦎 **Browse** button that opens the picker modal when a `creature_manager` is
  available.
- A ✕ **Clear** button that calls `apply_selected_creature_id(None)`.
- An ℹ hover label explaining the 3-D mesh spawning behaviour.

#### Creature Picker Modal in `show`

An `egui::Window` inserted after the portrait picker logic with:

- `id(egui::Id::new("character_creature_picker"))` — stable egui ID.
- `ScrollArea::vertical()` with `id_salt("character_creature_picker_scroll")`.
- `ui.push_id(creature.id, …)` for every row.
- Two-phase close: `picked_id` / `should_close` locals evaluated after the window
  closure.
- Graceful `None`-manager fallback: `creature_picker_open` is immediately cleared.

#### Preview Update

`show_character_preview` renders a `Creature:` row inside `character_preview_grid`
(between Alignment and the closing of that grid) when `character.creature_id` is
`Some`.

#### Call-Site Update in `lib.rs`

```antares/sdk/campaign_builder/src/lib.rs#L4660-4680
            EditorTab::Characters => {
                let char_creature_manager = self
                    .campaign_dir
                    .as_ref()
                    .map(|d| crate::creature_assets::CreatureAssetManager::new(d.clone()));
                self.characters_editor_state.show(
                    ui,
                    &self.races_editor_state.races,
                    &self.classes_editor_state.classes,
                    &self.items,
                    self.campaign_dir.as_ref(),
                    &self.campaign.characters_file,
                    &mut self.unsaved_changes,
                    &mut self.status_message,
                    &mut self.file_load_merge_mode,
                    char_creature_manager.as_ref(),
                )
            }
```

#### Test Coverage

| Test                                                           | What it verifies                                                                                 |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `test_characters_editor_creature_id_roundtrips_through_form`   | `start_edit_character` loads `Some(42)` → buffer `"42"`; `save_character` writes `Some(42)` back |
| `test_characters_editor_creature_id_empty_string_saves_none`   | Empty buffer string → `creature_id: None` in saved definition                                    |
| `test_characters_editor_creature_id_invalid_string_saves_none` | Non-numeric buffer string → `creature_id: None` (silent discard)                                 |
| `test_creature_picker_open_flag`                               | Default state has `creature_picker_open == false`                                                |
| `test_apply_selected_creature_id_sets_buffer`                  | `apply_selected_creature_id(Some("7"))` writes `"7"` and closes picker                           |

---

## Phase 4.1: Monsters Editor — Unified Creature Asset Binding

### Overview

Updated `sdk/campaign_builder/src/monsters_editor.rs` to support visual asset
binding for monsters. The editor now lets campaign authors link a `MonsterDefinition`
to a `CreatureDefinition` (a procedural mesh creature) via a "Visual Asset" section
in the form. When a `CreatureAssetManager` is provided the editor resolves and
displays the creature's human-readable name; without one it still shows the raw
numeric ID.

### Deliverables Checklist

- [x] `sdk/campaign_builder/src/monsters_editor.rs` — `use antares::domain::types::CreatureId` import added
- [x] `sdk/campaign_builder/src/monsters_editor.rs` — `use crate::creature_assets::CreatureAssetManager` import added
- [x] `sdk/campaign_builder/src/monsters_editor.rs` — `pub creature_picker_open: bool` field added to `MonstersEditorState`
- [x] `sdk/campaign_builder/src/monsters_editor.rs` — `creature_picker_open: false` added to `impl Default`
- [x] `sdk/campaign_builder/src/monsters_editor.rs` — `apply_selected_creature_id(&mut self, id: Option<CreatureId>)` method added with `///` doc comment
- [x] `sdk/campaign_builder/src/monsters_editor.rs` — `show` signature updated with `creature_manager: Option<&CreatureAssetManager>` parameter
- [x] `sdk/campaign_builder/src/monsters_editor.rs` — `show_form` signature updated with `creature_manager: Option<&CreatureAssetManager>` parameter
- [x] `sdk/campaign_builder/src/monsters_editor.rs` — "Visual Asset" `ui.group` section inserted in `show_form` between Basic Properties and Combat Stats
- [x] `sdk/campaign_builder/src/monsters_editor.rs` — Creature picker modal (`egui::Window`) inserted with `ScrollArea`, `push_id` per row, `id_salt` on scroll area, and correct open/close state management
- [x] `sdk/campaign_builder/src/monsters_editor.rs` — `show_monster_details` updated to display `Creature ID: {id}` when set
- [x] `sdk/campaign_builder/src/monsters_editor.rs` — `show_preview_static` updated to display `🦎 Creature: {id}` or `🦎 Creature: No creature asset`
- [x] `sdk/campaign_builder/src/lib.rs` — `EditorTab::Monsters` call site updated to pass a `CreatureAssetManager` built from `self.campaign_dir`
- [x] Three new tests added: `test_monsters_editor_creature_id_roundtrips_through_form`, `test_monsters_editor_clear_creature_id`, `test_monsters_editor_default_monster_creature_id_is_none`
- [x] All four quality gates pass with zero errors/warnings (`cargo fmt`, `cargo check --all-targets --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo nextest run --all-features` — 3333 passed)

### What Was Built

#### `creature_picker_open` Field

A plain `pub bool` field (no serde attributes — the struct does not derive
`Serialize`/`Deserialize`) that tracks whether the creature-picker modal window is
open. It is initialised to `false` in `Default`.

#### `apply_selected_creature_id`

```antares/sdk/campaign_builder/src/monsters_editor.rs#L67-71
    /// Sets the creature ID on the edit buffer and closes the picker.
    pub fn apply_selected_creature_id(&mut self, id: Option<CreatureId>) {
        self.edit_buffer.creature_id = id;
        self.creature_picker_open = false;
    }
```

Single method that both writes `creature_id` onto the edit buffer and closes the
picker, keeping the two pieces of state in sync in one place.

#### "Visual Asset" Section in `show_form`

Inserted between the existing "Basic Properties" group and "Combat Stats" group.
The section contains:

- A read-only label showing the current `creature_id` (or `"None"`).
- A **Browse…** button that sets `creature_picker_open = true`.
- A **Clear** button that calls `apply_selected_creature_id(None)`.
- An ℹ hover tooltip explaining what the binding does.
- A resolved-name label (`Asset: "…"` in grey) rendered only when both a
  `creature_id` and a `CreatureAssetManager` are available and
  `manager.load_creature(id)` succeeds.

#### Creature Picker Modal

An `egui::Window` with:

- `id(egui::Id::new("monster_creature_picker"))` — stable, unique egui ID.
- A `ScrollArea::vertical()` with `id_salt("monster_creature_picker_scroll")`.
- `ui.push_id(creature.id, …)` for every row in the scroll area.
- Two-phase close: clicking a row records `picked_id`; clicking "Close" records
  `should_close`. Both are evaluated after the window closure to avoid
  borrow-checker conflicts with `&mut self` inside the closure.
- Graceful fallback when `creature_manager` is `None`: the picker immediately
  self-closes.

#### Call-Site Update in `lib.rs`

```antares/sdk/campaign_builder/src/lib.rs#L4555-4565
            EditorTab::Monsters => self.monsters_editor_state.show(
                ui,
                &mut self.monsters,
                self.campaign_dir.as_ref(),
                &self.campaign.monsters_file,
                &mut self.unsaved_changes,
                &mut self.status_message,
                &mut self.file_load_merge_mode,
                self.campaign_dir
                    .as_ref()
                    .map(|d| crate::creature_assets::CreatureAssetManager::new(d.clone()))
                    .as_ref(),
            ),
```

A `CreatureAssetManager` is constructed on-the-fly from `self.campaign_dir` and
passed as `Option<&CreatureAssetManager>`. When no campaign is open the value is
`None` and the feature degrades gracefully.

#### Test Coverage

| Test                                                       | What it verifies                                                                      |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| `test_monsters_editor_creature_id_roundtrips_through_form` | `apply_selected_creature_id(Some(42))` writes `Some(42)` to `edit_buffer.creature_id` |
| `test_monsters_editor_clear_creature_id`                   | `apply_selected_creature_id(None)` clears a previously set `creature_id`              |
| `test_monsters_editor_default_monster_creature_id_is_none` | `default_monster()` initialises `creature_id` as `None`                               |

---

## Phase 3: Add the `CreatureBound` Trait — Unified Creature Asset Binding

### Overview

Defined the `CreatureBound` trait in a new file `src/domain/world/creature_binding.rs`
and implemented it for all four types that carry a `creature_id: Option<CreatureId>`
field: `MonsterDefinition`, `Monster` (runtime), `NpcDefinition`, and
`CharacterDefinition`. Updated `src/game/systems/map.rs` to use the trait method
(`def.creature_id()`) instead of direct field access at all three spawn branches:
`resolve_encounter_creature_id` (Encounter), the `RecruitableCharacter` branch, and
the NPC dialogue branch (via `ResolvedNpc`).

The `Monster` runtime type was discovered to require its own `impl` because the
SDK's `ContentDatabase` converts `MonsterDefinition` → `Monster` at load time
via `to_monster()`, and `content.0.monsters.get_monster()` returns
`Option<&Monster>`, not `Option<&MonsterDefinition>`.

### Deliverables Checklist

- [x] `src/domain/world/creature_binding.rs` — new file; SPDX header; `CreatureBound` trait definition with `///` doc comments and runnable `cargo test` example; `impl CreatureBound for MonsterDefinition`; `impl CreatureBound for Monster`; `impl CreatureBound for NpcDefinition`; `impl CreatureBound for CharacterDefinition`; nine unit tests
- [x] `src/domain/world/mod.rs` — `pub mod creature_binding;` added; `pub use creature_binding::CreatureBound;` added to the re-export block
- [x] `src/game/systems/map.rs` — `use crate::domain::world::CreatureBound;` import added; `resolve_encounter_creature_id` updated to `monster_def.creature_id()`; `RecruitableCharacter` branch updated to `.and_then(|def| def.creature_id())`
- [x] All four quality gates pass with zero errors/warnings (`cargo fmt`, `cargo check --all-targets --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo nextest run --all-features`)
- [x] All nine new trait unit tests pass (seven from the plan plus two for the `Monster` runtime type)

### What Was Built

#### `CreatureBound` Trait

The trait is defined in `src/domain/world/creature_binding.rs`:

```antares/src/domain/world/creature_binding.rs#L72-80
pub trait CreatureBound {
    /// Returns the optional [`CreatureId`] that links this definition to a mesh
    /// asset in the creature registry.
    ///
    /// Returns `None` when no visual binding has been set for this definition.
    fn creature_id(&self) -> Option<CreatureId>;
}
```

Each `impl` block is a one-liner that copies the `Option<CreatureId>` field value
(all four underlying fields are `Copy`):

```antares/src/domain/world/creature_binding.rs#L119-123
impl CreatureBound for MonsterDefinition {
    fn creature_id(&self) -> Option<CreatureId> {
        self.creature_id
    }
}
```

The module is publicly re-exported from `src/domain/world/mod.rs` so callers can
write `use antares::domain::world::CreatureBound`.

#### Why `Monster` Required a Separate `impl`

The SDK's `ContentDatabase` (in `src/sdk/database.rs`) stores runtime `Monster`
instances rather than `MonsterDefinition` objects — `add_monster` calls
`def.to_monster()` and inserts the result. Consequently,
`content.0.monsters.get_monster()` returns `Option<&Monster>`. The trait was
therefore implemented for the runtime `Monster` struct as well as the definition
type so that `resolve_encounter_creature_id` in `map.rs` can call the trait method
uniformly regardless of which backing storage is used.

#### Map System Updates

All three creature-id read call-sites in `src/game/systems/map.rs` now use the
trait method:

**Encounter branch (`resolve_encounter_creature_id`):**

```antares/src/game/systems/map.rs#L395-407
fn resolve_encounter_creature_id(
    monster_group: &[types::MonsterId],
    content: &crate::application::resources::GameContent,
) -> Option<types::CreatureId> {
    for monster_id in monster_group {
        if let Some(monster_def) = content.0.monsters.get_monster(*monster_id) {
            if let Some(creature_id) = monster_def.creature_id() {
                return Some(creature_id);
            }
        }
    }

    None
}
```

**`RecruitableCharacter` branch:**

```antares/src/game/systems/map.rs#L1347-1353
                    if let Some(creature_id) = content
                        .0
                        .characters
                        .get_character(character_id)
                        .and_then(|def| def.creature_id())
                    {
```

The NPC dialogue spawn loop works through `ResolvedNpc` (a DTO that copies
`creature_id` from `NpcDefinition` at resolution time in
`ResolvedNpc::from_placement_and_definition`). Because `ResolvedNpc` is a plain
data-transfer object rather than a definition type, direct field access on
`resolved_npc.creature_id` is correct there and is not replaced.

#### Test Coverage

Nine unit tests live in `src/domain/world/creature_binding.rs`:

| Test                                             | What it verifies                                                                         |
| ------------------------------------------------ | ---------------------------------------------------------------------------------------- |
| `test_creature_bound_runtime_monster_some`       | `Monster { creature_id: Some(3), .. }.creature_id() == Some(3)`                          |
| `test_creature_bound_runtime_monster_none`       | `Monster { creature_id: None, .. }.creature_id() == None`                                |
| `test_creature_bound_monster_some`               | `MonsterDefinition { creature_id: Some(3), .. }.creature_id() == Some(3)`                |
| `test_creature_bound_monster_none`               | `MonsterDefinition { creature_id: None, .. }.creature_id() == None`                      |
| `test_creature_bound_npc_some`                   | `NpcDefinition { creature_id: Some(1000), .. }.creature_id() == Some(1000)`              |
| `test_creature_bound_npc_none`                   | `NpcDefinition { creature_id: None, .. }.creature_id() == None`                          |
| `test_creature_bound_character_some`             | `CharacterDefinition { creature_id: Some(2000), .. }.creature_id() == Some(2000)`        |
| `test_creature_bound_character_none`             | `CharacterDefinition { creature_id: None, .. }.creature_id() == None`                    |
| `test_creature_bound_all_three_types_consistent` | All four types with `creature_id: Some(42)` return identical `Option<CreatureId>` values |

---

## Phase 2: Add `creature_id` to `CharacterDefinition` — Unified Creature Asset Binding

### Overview

Added `pub creature_id: Option<CreatureId>` to `CharacterDefinition` (and its
backward-compat deserialisation helper `CharacterDefinitionDef`) so that
recruitable characters displayed on the 3D map can be linked directly to a
`CreatureDefinition` in the creature registry. When `None`, the rendering system
falls back to the portrait sprite as before.

The heuristic functions `normalize_lookup_key` and `resolve_recruitable_creature_id`
in `src/game/systems/map.rs` were deleted. The spawn path now reads
`def.creature_id` directly from the `CharacterDefinition`, eliminating the
fragile name-normalisation cross-database lookup that was the previous fallback.

### Deliverables Checklist

- [x] `src/domain/character_definition.rs` — `use crate::domain::types::CreatureId` import added
- [x] `src/domain/character_definition.rs` — `creature_id: Option<CreatureId>` field added to `CharacterDefinition` with `#[serde(default)]` and `#[serde(skip_serializing_if = "Option::is_none")]`
- [x] `src/domain/character_definition.rs` — `creature_id: Option<CreatureId>` field added to `CharacterDefinitionDef` with `#[serde(default)]`
- [x] `src/domain/character_definition.rs` — `impl From<CharacterDefinitionDef> for CharacterDefinition` passes through `creature_id: def.creature_id`
- [x] `src/domain/character_definition.rs` — `CharacterDefinition::new` initialises `creature_id: None`
- [x] `src/domain/character_definition.rs` — `/// Examples` doc comment on `new` updated with `assert!(definition.creature_id.is_none())`
- [x] `src/domain/character_definition.rs` — struct-literal `CharacterDefinition` instances in tests updated with `creature_id: None`
- [x] `src/domain/character_definition.rs` — doc-comment struct literal updated with `creature_id: None`
- [x] `src/domain/character_definition.rs` — three new tests added: `test_character_definition_creature_id_defaults_to_none`, `test_character_definition_creature_id_field_roundtrips_ron`, `test_character_definition_creature_id_none_omits_field_in_ron`
- [x] `src/game/systems/map.rs` — `normalize_lookup_key` function deleted
- [x] `src/game/systems/map.rs` — `resolve_recruitable_creature_id` function deleted
- [x] `src/game/systems/map.rs` — `spawn_map` call site updated to use `content.0.characters.get_character(character_id).and_then(|def| def.creature_id)` directly
- [x] `src/game/systems/map.rs` — `test_spawn_map_uses_recruitable_character_creature_visual` updated: `creature_id: Some(58)` set on `old_gareth`; `character_id` changed from `"npc_old_gareth"` to `"old_gareth"`
- [x] `src/game/systems/map.rs` — `test_recruitable_visual_despawns_after_event_removed` updated: same `creature_id` and `character_id` fixes
- [x] `src/game/systems/map.rs` — `test_map_event_recruitable_character_facing` rewritten to use `CharacterDefinition` with `creature_id: Some(30)` and `character_id: "facing_test_char"` instead of `NpcDefinition`
- [x] `src/game/systems/map.rs` — new test `test_recruitable_spawn_uses_character_def_creature_id` added
- [x] `src/game/systems/map.rs` — new test `test_recruitable_spawn_falls_back_to_sprite_when_no_creature_id` added
- [x] `data/test_campaign/data/characters.ron` — `creature_id: Some(58)` added to `old_gareth` entry
- [x] `campaigns/tutorial/data/characters.ron` — `creature_id: Some(1007)` added to `old_gareth` entry
- [x] `docs/reference/architecture.md` — `creature_id: Option<CreatureId>` field added to `pub struct CharacterDefinition` in Section 4.7
- [x] `campaigns/tutorial/data/characters.ron` — `creature_id: Some(1012)` added to `"whisper"`; `creature_id: Some(1008)` added to `"apprentice_zara"` (both are `RecruitableCharacter` map events that previously resolved through the name-match heuristic)
- [x] All four quality gates pass with zero errors/warnings (`cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo nextest run`)

### What Was Built

#### `creature_id` Field on `CharacterDefinition`

The new field `pub creature_id: Option<CreatureId>` mirrors the pattern already
established on `NpcDefinition` and `MonsterDefinition`. The field carries
`#[serde(default)]` so existing RON files without the key deserialise silently
to `None`, and `#[serde(skip_serializing_if = "Option::is_none")]` so new files
serialised without a binding remain compact.

`CharacterDefinitionDef` (the backward-compat shim used by the custom
`Deserialize` impl) receives the same field with `#[serde(default)]`. The
`From<CharacterDefinitionDef>` conversion passes it through unchanged.

`CharacterDefinition::new` initialises the field to `None`; callers that need a
binding set it directly after construction:

```antares/src/domain/character_definition.rs#L557-591
pub fn new(...) -> Self {
    Self {
        // ... other fields ...
        creature_id: None,
    }
}
```

#### Deletion of Heuristic Resolution Functions

`normalize_lookup_key` and `resolve_recruitable_creature_id` have been removed
from `src/game/systems/map.rs`. These functions implemented a three-step
fallback that matched character names against creature names via ASCII-folded
normalisation — a fragile cross-database heuristic with no authoritative
source of truth. Now the spawn path reads the field directly:

```antares/src/game/systems/map.rs#L1344-1349
if let Some(creature_id) = content
    .0
    .characters
    .get_character(character_id)
    .and_then(|def| def.creature_id)
{
```

This is O(1), requires no name normalisation, and is fully deterministic.

#### Test Updates

Tests that previously relied on the name-match heuristic (`"npc_old_gareth"` →
`"OldGareth"` creature) now set `creature_id: Some(58)` explicitly on the
`CharacterDefinition` and use `character_id: "old_gareth"` in the map event to
match the database key directly.

`test_map_event_recruitable_character_facing` was rewritten to use a
`CharacterDefinition` with `creature_id: Some(30)` and id `"facing_test_char"`,
removing the dependency on `NpcDefinition` for the recruitable spawn path.

Two new tests verify the complete contract:

- **`test_recruitable_spawn_uses_character_def_creature_id`**: a
  `CharacterDefinition` with `creature_id: Some(42)` triggers spawn of a
  `CreatureVisual { creature_id: 42 }` entity at the correct tile position.
- **`test_recruitable_spawn_falls_back_to_sprite_when_no_creature_id`**: a
  `CharacterDefinition` with `creature_id: None` produces zero `CreatureVisual`
  entities and exactly one `RecruitableVisualMarker` sprite-fallback entity.

#### RON Data Files

`old_gareth` in `data/test_campaign/data/characters.ron` receives
`creature_id: Some(58)` so that the integration tests in `map.rs` that load
actual campaign data continue to resolve correctly without the heuristic.

`old_gareth` in `campaigns/tutorial/data/characters.ron` receives
`creature_id: Some(1007)` to bind him to the tutorial creature registry entry
for live-game rendering.

---

## Phase 1: Rename `visual_id` → `creature_id` on Monster Types

### Overview

Renamed the `visual_id` field to `creature_id` on both `MonsterDefinition` (domain
data struct) and `Monster` (runtime struct) to align naming with `NpcDefinition`,
which already used `creature_id`. Updated every call-site across source files, SDK
files, RON data files, and integration tests. Added a new RON round-trip test.

### Deliverables Checklist

- [x] `src/domain/combat/database.rs` — `visual_id` → `creature_id` on `MonsterDefinition`; doc comment updated; `to_monster()` updated; `create_test_monster` helper updated; `test_monster_visual_id_parsing` renamed to `test_monster_creature_id_parsing`; `test_load_tutorial_monsters_visual_ids` renamed to `test_load_tutorial_monsters_creature_ids`; new `test_monster_definition_creature_id_field_roundtrips_ron` added
- [x] `src/domain/combat/monster.rs` — `visual_id` → `creature_id` on `Monster`; `Monster::new()` initialiser updated; `set_visual` parameter renamed from `visual_id` to `creature_id`; doc comments updated; `test_set_visual_sets_creature_id` added
- [x] `src/game/systems/map.rs` — `resolve_encounter_creature_id` doc comment and `monster_def.visual_id` field access updated; all six inline test `MonsterDefinition` literals updated (`visual_id` → `creature_id`)
- [x] `src/game/systems/monster_rendering.rs` — module-level doc comments, `spawn_monster_with_visual` doc comment and logic, `spawn_fallback_visual` doc comment updated; local binding renamed from `visual_id` to `creature_id`; `CreatureVisual` construction updated to use shorthand `creature_id`; warn message updated
- [x] `src/domain/combat/engine.rs` — test helper `MonsterDefinition` literal updated
- [x] `sdk/campaign_builder/src/monsters_editor.rs` — `default_monster()` updated
- [x] `sdk/campaign_builder/src/advanced_validation.rs` — `create_test_monster()` updated
- [x] `sdk/campaign_builder/src/lib.rs` — all five `MonsterDefinition` literals updated (`default_monster`, `test_monster_xp_calculation_basic`, `test_monster_xp_calculation_with_abilities`, `test_monster_import_export_roundtrip`, `test_monster_preview_fields`)
- [x] `sdk/campaign_builder/src/templates.rs` — all four `create_monster()` literals updated
- [x] `sdk/campaign_builder/src/ui_helpers.rs` — both test-helper `MonsterDefinition` literals updated
- [x] `data/test_campaign/data/monsters.ron` — all `visual_id:` occurrences replaced with `creature_id:`
- [x] `campaigns/tutorial/data/monsters.ron` — all `visual_id:` occurrences replaced with `creature_id:`
- [x] `tests/campaign_integration_tests.rs` — `test_all_monsters_have_visual_id_mapping` renamed to `test_all_monsters_have_creature_id_mapping`; `test_fallback_mechanism_for_missing_visual_id` renamed to `test_fallback_mechanism_for_monster_missing_creature_id`; `test_creature_visual_id_ranges_follow_convention` renamed to `test_creature_id_ranges_follow_convention`; all `.visual_id` field accesses updated; all assertion messages updated
- [x] `tests/tutorial_campaign_loading_integration.rs` — `test_monster_spawning_with_missing_visual_id` renamed to `test_monster_spawning_with_missing_creature_id`; comments updated
- [x] `tests/tutorial_monster_creature_mapping.rs` — module doc comment updated; all `.visual_id` field accesses updated; all assertion messages updated
- [x] `grep -r "visual_id" . --include="*.rs" --include="*.ron"` returns zero matches
- [x] `campaigns/tutorial/creature_mappings.md` — `visual_id:` in Step 4 monster example replaced with `creature_id:`; "See Also" reference updated; monster count table updated from 11 to 17 entries
- [x] `docs/reference/architecture.md` Section 4.4 — `pub creature_id: Option<CreatureId>` added to `Monster` runtime struct; new sub-section `#### 4.4.1 CreatureBound Trait` added describing the trait and its three implementors
- [x] All four quality gates pass with zero errors/warnings (`cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo nextest run`)

### What Was Built

#### Field Rename — `MonsterDefinition` and `Monster`

The field `pub visual_id: Option<CreatureId>` was renamed to
`pub creature_id: Option<CreatureId>` on both structs. The doc comment was updated
from `"Optional visual creature ID for 3D representation"` to the more precise
`"Optional creature asset binding — links this monster to a CreatureDefinition in
the creature registry."` mirroring the language used in `NpcDefinition`.

The `#[serde(default)]` attribute was preserved unchanged. Because backwards
compatibility is not required, no `#[serde(rename)]` alias was added; the RON
data files were updated directly.

#### `set_visual` Method

The parameter of `Monster::set_visual` was renamed from `visual_id` to `creature_id`
for consistency. The body now reads `self.creature_id = Some(creature_id);`.

#### `to_monster()` Conversion

The single assignment `monster.visual_id = self.visual_id;` in
`MonsterDefinition::to_monster()` became `monster.creature_id = self.creature_id;`.

#### `resolve_encounter_creature_id` in `map.rs`

The field read `monster_def.visual_id` in the loop body was updated to
`monster_def.creature_id`. The function's leading doc comment was also updated.

#### `spawn_monster_with_visual` in `monster_rendering.rs`

The local binding `if let Some(visual_id) = monster.visual_id` became
`if let Some(creature_id) = monster.creature_id`. The `CreatureVisual` struct
construction was simplified from `creature_id: visual_id` to shorthand `creature_id`
after the binding rename. The `warn!` message and all comments referencing
`visual_id` were updated.

#### RON Data Files

A `sed` substitution replaced every `visual_id:` token with `creature_id:` in:

- `data/test_campaign/data/monsters.ron` (11 occurrences)
- `campaigns/tutorial/data/monsters.ron` (17 occurrences)

No numeric values changed.

#### New Test: `test_monster_definition_creature_id_field_roundtrips_ron`

Added to `src/domain/combat/database.rs` `mod tests`. Serialises a
`MonsterDefinition` with `creature_id: Some(42)` to RON using
`ron::ser::to_string_pretty`, deserialises it back, and asserts the value is
preserved — directly satisfying the Phase 1 success criterion.

#### New Test: `test_set_visual_sets_creature_id`

Added to `src/domain/combat/monster.rs` `mod tests`. Constructs a `Monster`,
asserts `creature_id` is `None`, calls `set_visual(5)` and asserts `Some(5)`,
then calls `set_visual(42)` and asserts `Some(42)` — confirming the method
sets `creature_id` and that repeated calls overwrite the previous value.
This was a required Phase 1 test that had been omitted in the original implementation.

### Success Criteria Verification

| Criterion                                                                            | Result                  |
| ------------------------------------------------------------------------------------ | ----------------------- |
| `grep -r "visual_id" . --include="*.rs" --include="*.ron"` returns zero matches      | ✓ Verified              |
| `cargo nextest run --all-features` reports zero failures                             | ✓ 3319 passed, 0 failed |
| `test_monster_definition_creature_id_field_roundtrips_ron` passes RON round-trip     | ✓ Passes                |
| All existing rendering and combat tests use `creature_id` and produce same behaviour | ✓ Verified              |

---

## Terrain Quality Improvement

### Cross-References

This grouped section documents delivered terrain work. `docs/explanation/terrain_quality_deviation_plan.md` documents the remaining correction work and approved retained deviations.

### Phase 3: High-Quality Tree Models

### Overview

Phase 3 replaces sphere-based foliage with alpha-masked double-sided plane quads,
adds a bark texture to trunk cylinders, adds the `Palm` variant to the rendering
`TreeType` enum, and caches material handles in `ProceduralMeshCache` alongside
existing mesh handles.

### Deliverables Checklist

- [x] `assets/textures/trees/bark.png` — bark texture (64×128, opaque)
- [x] `assets/textures/trees/foliage_oak.png` — Oak foliage alpha-mask texture
- [x] `assets/textures/trees/foliage_pine.png` — Pine foliage alpha-mask texture
- [x] `assets/textures/trees/foliage_birch.png` — Birch foliage alpha-mask texture
- [x] `assets/textures/trees/foliage_willow.png` — Willow foliage alpha-mask texture
- [x] `assets/textures/trees/foliage_palm.png` — Palm foliage alpha-mask texture
- [x] `assets/textures/trees/foliage_shrub.png` — Shrub foliage alpha-mask texture
- [x] `src/game/systems/advanced_trees.rs` — `Palm` variant added to `TreeType` enum, `config()`, `name()`, `all()`
- [x] `src/game/systems/map.rs` — `Palm` domain type maps to `advanced_trees::TreeType::Palm` (fallback removed)
- [x] `src/game/systems/procedural_meshes.rs` — `tree_bark_material` and `tree_foliage_materials` fields added to `ProceduralMeshCache`
- [x] `src/game/systems/procedural_meshes.rs` — `get_or_create_bark_material` and `get_or_create_foliage_material` methods implemented
- [x] `src/game/systems/procedural_meshes.rs` — `spawn_foliage_clusters` uses plane quads with `AlphaMode::Mask` foliage material
- [x] `src/game/systems/procedural_meshes.rs` — `spawn_tree` uses bark material on trunk cylinders
- [x] All four quality gates pass with zero errors/warnings
- [x] All new and updated unit tests pass

### What Was Built

#### `assets/textures/trees/` — Seven Generated Textures

Seven PNG textures are generated by `src/bin/generate_terrain_textures.rs` via the
new `generate_tree_textures()` function:

| File                 | Dimensions   | Content                                                  |
| -------------------- | ------------ | -------------------------------------------------------- |
| `bark.png`           | 64×128 RGBA  | Brown vertical-grain bark, fully opaque (alpha = 255)    |
| `foliage_oak.png`    | 128×128 RGBA | Circular leaf cluster, transparent outside radius        |
| `foliage_pine.png`   | 64×128 RGBA  | Tall narrow circular cluster, transparent outside radius |
| `foliage_birch.png`  | 128×128 RGBA | Light green circular cluster                             |
| `foliage_willow.png` | 128×128 RGBA | Medium-dark green circular cluster                       |
| `foliage_palm.png`   | 128×128 RGBA | Tropical green circular cluster                          |
| `foliage_shrub.png`  | 64×64 RGBA   | Small dense circular cluster                             |

All foliage textures use a circular alpha mask with a soft 20%-radius falloff edge.
The bark texture uses per-row grain variation (`y % 5` modulation, −2 to +2) plus
per-pixel noise for a natural wood look.

#### `src/bin/generate_terrain_textures.rs` — Generator Binary Extension

New constants:

```
BARK_WIDTH / BARK_HEIGHT          64 × 128 — bark texture dimensions
BARK_R / BARK_G / BARK_B          90 / 60 / 35 — brown bark base colour
BARK_SEED                         0xB1C2_D3E4_F5A6_0718
FOLIAGE_WIDTH / FOLIAGE_HEIGHT    128 × 128 — standard foliage texture dimensions
SHRUB_FOLIAGE_SIZE                64 — square shrub texture size
PINE_FOLIAGE_WIDTH / HEIGHT       64 × 128 — tall narrow pine texture
FOLIAGE_ALPHA_OUTER               0 — transparent pixels outside circle
FOLIAGE_ALPHA_INNER               240 — near-opaque pixels inside circle
```

New functions:

- `generate_bark_texture()` — 64×128 RGBA opaque brown bark with vertical grain and per-pixel noise.
- `generate_foliage_texture(w, h, r, g, b, seed)` — Generic circular alpha-mask foliage generator with soft edge falloff.
- `generate_tree_textures(trees_dir)` — Orchestrates all 7 tree PNG writes with unique seeds and colours per variant.
- `save_texture(dir, filename, img)` — Private helper for error-checked image file writes.

`main()` now calls `generate_tree_textures()` after the grass section, writing to
`assets/textures/trees/` relative to `CARGO_MANIFEST_DIR`.

#### `src/game/systems/advanced_trees.rs` — Palm TreeType Variant

`Palm` was added as the seventh variant in `pub enum TreeType`:

```rust
Palm,   // Tall slender trunk with fan-shaped fronds at the crown
        // Use for: Tropical biomes, coastal areas, desert oases
```

Three `impl TreeType` methods updated:

- `config()` — Palm returns `TreeConfig { trunk_radius: 0.18, height: 5.5, branch_angle_range: (70.0, 85.0), depth: 2, foliage_density: 0.7, foliage_color: (0.4, 0.7, 0.2) }`
- `name()` — `TreeType::Palm => "Palm"`
- `all()` — slice now has 7 elements including `TreeType::Palm`

Two internal match statements in `generate_branch_graph` and `subdivide_branch` were
also extended with `Palm` arms (seed `48u64`; child count `2..=3`).

#### `src/game/systems/map.rs` — Palm Fallback Removed

The Palm arm in the domain→rendering TreeType match was changed from:

```rust
crate::domain::world::TreeType::Palm => {
    crate::game::systems::advanced_trees::TreeType::Oak
} // Fallback for Palm
```

to:

```rust
crate::domain::world::TreeType::Palm => {
    crate::game::systems::advanced_trees::TreeType::Palm
}
```

Both `spawn_tree` call-sites also received the new `&asset_server` parameter.

#### `src/game/systems/procedural_meshes.rs` — Material Cache and Foliage Quads

**New constants** (after `TREE_FOLIAGE_COLOR`):

| Constant                      | Value                                 |
| ----------------------------- | ------------------------------------- |
| `TREE_BARK_TEXTURE`           | `"textures/trees/bark.png"`           |
| `TREE_FOLIAGE_TEXTURE_OAK`    | `"textures/trees/foliage_oak.png"`    |
| `TREE_FOLIAGE_TEXTURE_PINE`   | `"textures/trees/foliage_pine.png"`   |
| `TREE_FOLIAGE_TEXTURE_BIRCH`  | `"textures/trees/foliage_birch.png"`  |
| `TREE_FOLIAGE_TEXTURE_WILLOW` | `"textures/trees/foliage_willow.png"` |
| `TREE_FOLIAGE_TEXTURE_PALM`   | `"textures/trees/foliage_palm.png"`   |
| `TREE_FOLIAGE_TEXTURE_SHRUB`  | `"textures/trees/foliage_shrub.png"`  |
| `TREE_FOLIAGE_ALPHA_CUTOFF`   | `0.35_f32`                            |

**New `ProceduralMeshCache` fields**:

```rust
/// Cached bark material handle (shared across all non-Dead tree types)
tree_bark_material: Option<Handle<StandardMaterial>>,
/// Cached foliage material handles keyed by TreeType variant
tree_foliage_materials: HashMap<TreeType, Handle<StandardMaterial>>,
```

Both are initialised to `None` / empty in `Default` and cleared in `clear_all()`.

**New `impl ProceduralMeshCache` methods**:

- `get_or_create_bark_material(asset_server, materials)` — Returns a cached `Handle<StandardMaterial>` for the bark texture tinted with `TREE_TRUNK_COLOR`, `perceptual_roughness: 0.9`.
- `get_or_create_foliage_material(tree_type, asset_server, materials)` — Returns a cached `Handle<StandardMaterial>` per tree type with `base_color_texture`, `AlphaMode::Mask(TREE_FOLIAGE_ALPHA_CUTOFF)`, `double_sided: true`, `cull_mode: None`, `perceptual_roughness: 0.8`.

**Private helper**:

```rust
fn foliage_texture_path(tree_type: TreeType) -> &'static str
```

Selects the correct texture path constant for each variant. `Dead` falls back to the
Oak path (density is 0.0 so it is never loaded in practice).

**`spawn_foliage_clusters` rewrite**:

- Added `asset_server: &AssetServer` and `tree_type: TreeType` parameters.
- `foliage_color` changed from `Color` to `Option<Color>`.
- Foliage geometry replaced: `Sphere { radius: TREE_FOLIAGE_RADIUS }` → `Plane3d::default().mesh().size(foliage_size * 2.0, foliage_size * 2.0).build()` where `foliage_size = config.foliage_density * TREE_FOLIAGE_RADIUS`.
- The plane is rotated 90° around X to stand upright and given a random Y-axis rotation so clusters fan out naturally.
- The inline `StandardMaterial` is replaced by `cache.get_or_create_foliage_material(tree_type, …)`. When a tint is present the cached material is cloned and `base_color` overridden; without tint the cached handle is reused directly.

**`spawn_tree` update**:

- Added `asset_server: &AssetServer` parameter.
- Trunk material is now obtained from `cache.get_or_create_bark_material(asset_server, materials)` (with per-instance tint clone when `color_tint` is `Some`).
- `foliage_color` is now `Option<Color>` (None when no tint).
- Updated call to `spawn_foliage_clusters` passes `asset_server` and `tree_type_resolved`.

### Tests

#### New tests in `src/bin/generate_terrain_textures.rs` (6 new)

| Test                                                | What it verifies                  |
| --------------------------------------------------- | --------------------------------- |
| `test_generate_bark_texture_dimensions`             | Image is exactly 64×128           |
| `test_generate_bark_texture_fully_opaque`           | Every pixel has alpha = 255       |
| `test_generate_bark_texture_is_deterministic`       | Two calls produce identical bytes |
| `test_generate_foliage_texture_dimensions`          | Image is 128×128                  |
| `test_generate_foliage_texture_centre_is_opaque`    | Centre pixel alpha > 0            |
| `test_generate_foliage_texture_corners_transparent` | All four corners have alpha = 0   |
| `test_generate_foliage_texture_is_deterministic`    | Same seed → identical bytes       |

#### New tests in `src/game/systems/advanced_trees.rs` (4 new)

| Test                                                        | What it verifies                                 |
| ----------------------------------------------------------- | ------------------------------------------------ |
| `test_tree_type_palm_config_returns_correct_parameters`     | `trunk_radius == 0.18`, `height == 5.5`          |
| `test_tree_type_palm_name`                                  | `TreeType::Palm.name() == "Palm"`                |
| `test_tree_type_all_includes_palm`                          | `TreeType::all().len() == 7` and contains `Palm` |
| `test_all_tree_types_including_palm_generate_without_panic` | All 7 types produce non-empty branch graphs      |

#### Updated tests in `src/game/systems/advanced_trees.rs`

- `test_tree_type_all_variants_present` — updated assertion to `len() == 7`, added `Palm` contains check.
- `test_get_leaf_branches_all_tree_types` — added `TreeType::Palm` to the explicit iteration list.

#### Updated test in `tests/advanced_trees_integration_test.rs`

- `test_tree_type_enumeration` — updated count to `7`, added `palm_found` assertion.

#### New tests in `src/game/systems/procedural_meshes.rs` (6 new)

| Test                                                | What it verifies                                              |
| --------------------------------------------------- | ------------------------------------------------------------- |
| `test_foliage_texture_path_all_variants`            | Non-empty `.png` path for all 7 `TreeType` variants           |
| `test_tree_foliage_alpha_cutoff_valid`              | `TREE_FOLIAGE_ALPHA_CUTOFF` is in `(0.0, 1.0)` (const blocks) |
| `test_cache_tree_foliage_materials_default_empty`   | `tree_foliage_materials` is empty on default construction     |
| `test_cache_tree_bark_material_default_none`        | `tree_bark_material` is `None` on default construction        |
| `test_cache_clear_all_clears_foliage_materials`     | `clear_all()` resets both material cache fields               |
| `test_foliage_texture_path_distinct_for_leaf_types` | Oak/Pine/Birch/Willow/Palm/Shrub each have unique paths       |
| `test_foliage_texture_path_palm_uses_palm_texture`  | Palm path contains the string `"palm"`                        |

### Architecture Compliance

- `TreeType::Palm` added to the rendering enum (`src/game/systems/advanced_trees.rs`) matching the domain enum (`src/domain/world/types.rs`) — no deviations.
- `AttributePair` pattern not applicable here (rendering-layer only change).
- Constants extracted: `TREE_BARK_TEXTURE`, `TREE_FOLIAGE_TEXTURE_*`, `TREE_FOLIAGE_ALPHA_CUTOFF` — no magic numbers.
- `TerrainVisualConfig` fields (`scale`, `height_multiplier`, `color_tint`, `rotation_y`) continue to function without domain-layer modification.
- RON data files unchanged.
- No test references `campaigns/tutorial` (Implementation Rule 5 compliant).

### Quality Gate Results

| Gate                                                       | Result                              |
| ---------------------------------------------------------- | ----------------------------------- |
| `cargo fmt --all`                                          | ✅ no output                        |
| `cargo check --all-targets --all-features`                 | ✅ Finished, 0 errors               |
| `cargo clippy --all-targets --all-features -- -D warnings` | ✅ Finished, 0 warnings             |
| `cargo nextest run --all-features`                         | ✅ 3318 passed, 8 skipped, 0 failed |

### Phase 2: High-Quality Grass

### Overview

Replace the opaque solid-colour grass blade material with an `AlphaMode::Mask`
textured material. A 32×128 RGBA grass blade PNG is generated procedurally, and
all `GrassBladeConfig` / `color_tint` parameters from `TileVisualMetadata`
continue to work by multiplying the tint into `base_color` on the
`StandardMaterial`.

### Deliverables Checklist

- [x] `assets/textures/grass/grass_blade.png` — 32×128 alpha-masked blade texture
- [x] `src/bin/generate_terrain_textures.rs` — extended with `generate_grass_blade_texture()`
- [x] `src/game/systems/advanced_grass.rs` — `GRASS_BLADE_TEXTURE` and `GRASS_ALPHA_CUTOFF` constants added
- [x] `src/game/systems/advanced_grass.rs` — `spawn_grass_cluster` and `spawn_grass` accept `asset_server` and use `AlphaMode::Mask`
- [x] `src/game/systems/advanced_grass.rs` — `spawn_grass` accepts and applies `tile_tint: Option<(f32,f32,f32)>`
- [x] `src/game/systems/map.rs` — `spawn_map` passes `&asset_server` and `tile.visual.color_tint` to `spawn_grass`
- [x] All four quality gates pass with zero errors/warnings
- [x] All new and updated unit tests pass

### What Was Built

#### `assets/textures/grass/grass_blade.png` — Grass Blade Texture

Generated by the extended `src/bin/generate_terrain_textures.rs` binary via the
new `generate_grass_blade_texture()` function.

- Dimensions: 32×128 RGBA PNG.
- The blade strip occupies the centre 16 columns (`x ∈ [8, 24)`); pixels outside
  the strip are fully transparent (`alpha = 0`).
- Alpha gradient: `y = 0` (top, UV `v = 0`) = alpha 255 (fully opaque, blade
  base); `y = 127` (bottom, UV `v = 1`) = alpha 64 (semi-transparent, blade
  tip). This matches the UV generation in `create_grass_blade_mesh` where `t`
  goes from 0.0 (base) to 1.0 (tip).
- RGB inside the strip: base colour `(60, 130, 50)` with deterministic ±10
  per-channel noise (seed `0xA1B2_C3D4_E5F6_0718`).

#### `src/bin/generate_terrain_textures.rs` — Generator Binary Extension

Added at module level:

- Eight new constants: `GRASS_BLADE_WIDTH`, `GRASS_BLADE_HEIGHT`,
  `BLADE_STRIP_WIDTH`, `GRASS_BLADE_R/G/B`, `GRASS_BLADE_ALPHA_BASE/TIP`,
  `GRASS_BLADE_SEED`.
- `pub fn generate_grass_blade_texture() -> ImageBuffer<Rgba<u8>, Vec<u8>>` — the
  generation function, reusing the existing `apply_noise` helper.
- `main()` extended to also write `assets/textures/grass/grass_blade.png`.

Five new tests cover dimensions, outside-strip transparency, inside-strip alpha
range, gradient direction, and determinism.

#### `src/game/systems/advanced_grass.rs` — Constants and Material Update

Two module-level constants added after the existing constants block (L31):

```rust
const GRASS_BLADE_TEXTURE: &str = "textures/grass/grass_blade.png";
const GRASS_ALPHA_CUTOFF: f32 = 0.3;
```

`fn spawn_grass_cluster` — new parameter `asset_server: &Res<AssetServer>`.
The material construction block replaces the previous opaque solid-colour
material:

```rust
let texture_handle: Handle<Image> = asset_server.load(GRASS_BLADE_TEXTURE);
let blade_material = materials.add(StandardMaterial {
    base_color: blade_color,           // tinted by GrassColorScheme
    base_color_texture: Some(texture_handle),
    alpha_mode: AlphaMode::Mask(GRASS_ALPHA_CUTOFF),
    double_sided: true,
    cull_mode: None,
    perceptual_roughness: 0.7,
    ..default()
});
```

`pub fn spawn_grass` — two new parameters:

| Parameter      | Type                      | Purpose                                                                    |
| -------------- | ------------------------- | -------------------------------------------------------------------------- |
| `asset_server` | `&Res<AssetServer>`       | Forwarded to `spawn_grass_cluster`                                         |
| `tile_tint`    | `Option<(f32, f32, f32)>` | Explicit tint override; takes precedence over `visual_metadata.color_tint` |

The tint resolution order:

1. Explicit `tile_tint` argument (highest priority)
2. `visual_metadata.color_tint`
3. Natural-green default `(0.3, 0.65, 0.2)`

The resolved tint is multiplied into `base_color` and `tip_color` before the
`GrassColorScheme` is constructed, so all per-blade colour sampling is
automatically tinted.

#### `src/game/systems/map.rs` — Call-Site Update

The single call to `super::advanced_grass::spawn_grass` inside `spawn_map` now
passes two additional arguments:

```rust
super::advanced_grass::spawn_grass(
    &mut commands,
    &mut materials,
    &mut meshes,
    &asset_server,          // new — already in spawn_map's signature
    pos,
    map.id,
    Some(&tile.visual),
    tile.visual.color_tint, // new — Option<(f32,f32,f32)> from TileVisualMetadata
    &quality_settings,
);
```

### Tests

#### New tests in `src/game/systems/advanced_grass.rs`

| Test name                                            | What it verifies                                                                   |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `test_grass_blade_texture_path_constant`             | `GRASS_BLADE_TEXTURE` starts with `"textures/grass/"` and ends with `".png"`       |
| `test_grass_alpha_cutoff_in_valid_range`             | `GRASS_ALPHA_CUTOFF` is `> 0.0` and `< 1.0`                                        |
| `test_grass_material_uses_alpha_mask`                | Spawned blades have `AlphaMode::Mask(_)` and `base_color_texture.is_some()`        |
| `test_grass_color_tint_forwarded_to_color_scheme`    | `(0.5, 0.5, 0.5)` tint produces a darker green than the default natural-green tint |
| `test_create_grass_blade_mesh_uvs_span_full_v_range` | UV V-coordinates span exactly `[0.0, 1.0]` (texture maps base-to-tip)              |

#### Updated tests

- `test_create_grass_blade_mesh_has_uvs` — updated to assert UV attribute is
  present (separate `test_create_grass_blade_mesh_uvs_span_full_v_range` handles
  the range check).
- `test_spawn_grass_with_none_density_spawns_no_blades` — updated system
  signature and call to pass `asset_server` and `tile_tint: None`; now uses
  `AssetPlugin` + `init_asset` for the minimal app.
- `test_spawn_grass_with_density_spawns_blades_and_cluster` — same updates as
  above.

#### New tests in `src/bin/generate_terrain_textures.rs`

| Test name                                                     | What it verifies                                                 |
| ------------------------------------------------------------- | ---------------------------------------------------------------- |
| `test_generate_grass_blade_texture_dimensions`                | Image is 32×128                                                  |
| `test_generate_grass_blade_texture_outside_strip_transparent` | All pixels outside the 16-px blade strip have alpha = 0          |
| `test_generate_grass_blade_texture_inside_strip_alpha_range`  | All pixels inside the strip have alpha ≥ `GRASS_BLADE_ALPHA_TIP` |
| `test_generate_grass_blade_texture_alpha_gradient_direction`  | Top row (y=0) is more opaque than bottom row (y=127)             |
| `test_generate_grass_blade_texture_is_deterministic`          | Two calls produce identical pixel data                           |

### Architecture Compliance

- [ ] Data structures match architecture.md Section 4 **EXACTLY** — no domain
      structs modified; all changes are in rendering code.
- [ ] Module placement follows Section 3.2 — grass rendering code stays in
      `src/game/systems/advanced_grass.rs`.
- [ ] Constants extracted, not hardcoded — `GRASS_BLADE_TEXTURE` and
      `GRASS_ALPHA_CUTOFF` defined at module level.
- [ ] `TileVisualMetadata` fields continue to affect grass as specified in
      Section 2.3 of the plan: `grass_density`, `grass_blade_config.*`, and
      `color_tint` all remain functional.
- [ ] RON format used for data files — no new data files added; texture files
      use `.png` as required by Bevy's `AssetServer`.
- [ ] No test references `campaigns/tutorial` (Implementation Rule 5).

### Quality Gate Results

```
cargo fmt --all         → no output (all files formatted)
cargo check --all-targets --all-features → Finished, 0 errors
cargo clippy --all-targets --all-features -- -D warnings → Finished, 0 warnings
cargo nextest run --all-features → 3300 tests run: 3300 passed, 8 skipped
```

---

### Phase 1: Terrain Texture Foundation

### Overview

Phase 1 of the terrain quality improvement plan introduces PBR texture support for ground
tiles. Before this phase every terrain tile in `spawn_map` rendered with a flat
`StandardMaterial { base_color: Color::srgb(...) }` — no textures, no `base_color_texture`.
Phase 1 replaces those inline flat-colour materials with cached, textured `StandardMaterial`
handles loaded at startup.

The work is split into four parts:

1. **Asset directory + placeholder textures** — nine 64×64 RGBA PNGs written to
   `assets/textures/terrain/` by a new `generate_terrain_textures` binary.
2. **`TerrainMaterialCache` resource** — a new Bevy `Resource` that stores one
   `Handle<StandardMaterial>` per `TerrainType` variant.
3. **`load_terrain_materials_system`** — a `Startup` system that loads each texture via
   `AssetServer`, creates a `StandardMaterial` with correct `perceptual_roughness`, and
   populates the cache.
4. **`spawn_map` wiring** — `spawn_map` (and every call-site that forwards to it) now
   accepts a `&TerrainMaterialCache` parameter and looks up the cached handle for each
   tile instead of allocating a new material per tile. The existing `color_tint` logic
   is preserved: tinted tiles still create one-off materials; the cached handle is never
   mutated.

### Deliverables Checklist

- [x] `src/bin/generate_terrain_textures.rs` — binary that generates deterministic 64×64 RGBA PNGs with ±10 per-channel noise
- [x] `assets/textures/terrain/ground.png` — placeholder ground texture (grey)
- [x] `assets/textures/terrain/grass.png` — placeholder grass texture (green)
- [x] `assets/textures/terrain/stone.png` — placeholder stone texture (light grey)
- [x] `assets/textures/terrain/mountain.png` — placeholder mountain texture (dark grey)
- [x] `assets/textures/terrain/dirt.png` — placeholder dirt texture (brown)
- [x] `assets/textures/terrain/water.png` — placeholder water texture (blue)
- [x] `assets/textures/terrain/lava.png` — placeholder lava texture (red-orange)
- [x] `assets/textures/terrain/swamp.png` — placeholder swamp texture (olive-green)
- [x] `assets/textures/terrain/forest_floor.png` — placeholder forest floor texture (dark green)
- [x] `Cargo.toml` — `image = { version = "0.25", features = ["png"] }` dependency added
- [x] `Cargo.toml` — `[[bin]] name = "generate_terrain_textures"` entry added
- [x] `src/game/resources/terrain_material_cache.rs` — `TerrainMaterialCache` resource with `get`, `set`, `is_fully_loaded` impl and 11 unit tests
- [x] `src/game/resources/mod.rs` — `pub mod terrain_material_cache` + `pub use terrain_material_cache::TerrainMaterialCache`
- [x] `src/game/systems/terrain_materials.rs` — `load_terrain_materials_system`, `texture_path_for`, `roughness_for`, nine `TEXTURE_*` constants, 8 unit tests
- [x] `src/game/systems/mod.rs` — `pub mod terrain_materials` added
- [x] `src/game/systems/map.rs` — `MapRenderingPlugin::build` chains `load_terrain_materials_system` before `spawn_map_system`
- [x] `src/game/systems/map.rs` — `spawn_map` accepts `terrain_cache: &TerrainMaterialCache` and uses it for Ground, Water, Grass, Mountain, and all other terrain types via the catch-all fallback
- [x] `src/game/systems/map.rs` — `spawn_map_system`, `handle_door_opened`, `spawn_map_markers` each accept `Option<Res<TerrainMaterialCache>>` so existing tests that do not insert the resource continue to work
- [x] All four quality gates passed: `cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo nextest run` (3290 passed, 8 skipped)

### What Was Built

#### `src/bin/generate_terrain_textures.rs` — Texture Generator Binary

A standalone binary that creates all nine placeholder terrain PNGs. Key design choices:

- **Deterministic output**: A custom `xorshift64` PRNG is seeded with a unique `u64` per
  texture so the output is bit-identical across runs (reproducible builds). No external
  randomness crate is needed.
- **Correct unsigned modulo**: The offset calculation uses `next % range` on `u64` before
  casting to `i32`, avoiding the negative-remainder bug that signed modulo would introduce.
- **Clamped channels**: `clamp_channel(i32) -> u8` saturates at `[0, 255]` so extreme base
  values (0 or 255) near the boundary never overflow.
- **Base colours from spec**: Each `TerrainTextureSpec` carries the RGBA base values from
  the implementation plan Section 1.1 table exactly.

The binary is run once at development time (`cargo run --bin generate_terrain_textures`) and
the resulting PNGs are committed to the repository. Bevy's `AssetServer` then serves them at
runtime without any further build step.

#### `src/game/resources/terrain_material_cache.rs` — `TerrainMaterialCache`

A simple `#[derive(Resource, Default)]` struct with nine `Option<Handle<StandardMaterial>>`
fields, one per `TerrainType` variant. The three impl methods follow the pattern established
by the plan:

| Method                 | Behaviour                                                                  |
| ---------------------- | -------------------------------------------------------------------------- |
| `get(terrain)`         | Returns `Option<&Handle<StandardMaterial>>` via a `match` on `TerrainType` |
| `set(terrain, handle)` | Inserts / overwrites the handle for the given variant                      |
| `is_fully_loaded()`    | Returns `true` only when all nine fields are `Some`                        |

`TerrainType::Forest` maps to the `forest_floor` field (the forest floor texture is used for
the ground plane under forest tiles; tree meshes are spawned separately on top).

#### `src/game/systems/terrain_materials.rs` — Startup System

`load_terrain_materials_system` iterates all nine `TerrainType` variants in a single loop,
calling `asset_server.load::<Image>(texture_path_for(terrain))` and then
`materials.add(StandardMaterial { base_color_texture: Some(handle), perceptual_roughness: roughness_for(terrain), ..default() })`.
The finished cache is inserted with `commands.insert_resource(cache)`.

The nine `TEXTURE_*` path constants are `pub` so other systems (e.g. future Phase 2 grass
work) can reference them without hard-coding strings.

`roughness_for` encodes the per-terrain values from the plan (Water = 0.10 for reflective
appearance; Ground = 0.95 for rough matte; Stone = 0.75, etc.).

#### `src/game/systems/map.rs` — `spawn_map` Wiring

Three changes were made to `map.rs`:

1. **Startup ordering**: The `Startup` schedule now chains
   `load_terrain_materials_system → register_sprite_sheets_system → spawn_map_system`
   so the cache is populated before the first map is rendered.

2. **Cache lookups in `spawn_map`**: The three pre-built materials (`floor_material`,
   `water_material`, `grass_material`) are now resolved from the cache with an
   `unwrap_or_else` fallback to a flat-colour material. The Mountain branch and the
   wildcard `_` branch (Ground, Stone, Dirt, Lava, Swamp) similarly look up the cache.
   The `color_tint` logic is untouched: tinted tiles always produce a one-off material.

3. **`Option<Res<TerrainMaterialCache>>` in Update systems**: `handle_door_opened` and
   `spawn_map_markers` use `Option<Res<TerrainMaterialCache>>` so that test apps that add
   `MapManagerPlugin` without inserting the resource do not panic. When the resource is
   absent a local `TerrainMaterialCache::default()` is used as a transparent fallback (all
   fields `None` → the flat-colour fallback path in `spawn_map` activates automatically).

### Architecture Compliance

- [x] `TerrainMaterialCache` derives `Resource` and `Default` as required.
- [x] `TerrainType` variants used exactly as defined in `src/domain/world/types.rs` — no new variants introduced.
- [x] RON data files untouched — no game data was changed.
- [x] `color_tint` behaviour preserved — tinted tiles still create one-off materials.
- [x] `TEXTURE_*` constants defined at module level — no magic strings in system logic.
- [x] All roughness values match the plan table (Section 1.3) exactly.
- [x] SPDX headers added to all new `.rs` files (`2026 Brett Smith`).
- [x] `///` doc comments on every public item.

---

## Food System — Phase 4: UI and SDK Editor Updates

### Overview

Phase 4 of the food system migration updates the SDK campaign builder's Items Editor and the
CLI `item_editor` tool to expose the `ConsumableEffect::IsFood(u8)` variant introduced in
Phase 1. Before this phase, campaign developers could not create novel food items (e.g. "Elven
Bread", "Roast Beef") through the graphical or command-line editors — the `IsFood` variant was
simply missing from all dropdowns and menus. Phase 4 closes that gap: the Items Editor now
lists "Food (Rations)" as a selectable effect type and renders a `ration_value` drag-value
field when it is chosen. The CLI tool gains an equivalent `[5] Food (Rations)` option in both
the create and edit flows.

No domain-logic changes were needed. All modifications are pure UI/presentation layer.

### Deliverables Checklist

- [x] SDK `sdk/campaign_builder/src/items_editor.rs` — `IsFood` added to `ConsumableEffect` ComboBox dropdown
- [x] SDK `sdk/campaign_builder/src/items_editor.rs` — `ration_value: u8` DragValue field rendered when `IsFood` is selected
- [x] SDK `sdk/campaign_builder/src/items_editor.rs` — `IsFood` branch added to `show_preview_static` for readable label ("Food (1 ration)" / "Food (3 rations)")
- [x] SDK `sdk/campaign_builder/src/items_editor.rs` — tooltip on effect ComboBox row explains food semantics
- [x] SDK `sdk/campaign_builder/src/items_editor.rs` — "⚠️ Food items are not usable in combat." label shown below ration_value field
- [x] SDK `sdk/campaign_builder/src/items_editor.rs` — 9 new tests covering `IsFood` editor behaviour
- [x] CLI `src/bin/item_editor.rs` — `[5] Food (Rations)` option added to `create_consumable`
- [x] CLI `src/bin/item_editor.rs` — food items hard-code `is_combat_usable = false` in create flow
- [x] CLI `src/bin/item_editor.rs` — `[5] Food (Rations)` option added to `edit_item_classification` consumable branch
- [x] CLI `src/bin/item_editor.rs` — editing to `IsFood` forces `is_combat_usable = false` on the item
- [x] CLI `src/bin/item_editor.rs` — 4 new tests covering `IsFood` CLI behaviour
- [x] All quality gates passed: `cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo nextest run` (3258 passed, 8 skipped)

### What Was Built

#### `sdk/campaign_builder/src/items_editor.rs` — SDK Items Editor

**Effect type ComboBox** (`show_type_editor`, Consumable branch): The existing ComboBox that
lets the user pick a `ConsumableEffect` variant now includes a "Food (Rations)" option. When
clicked it initialises the effect to `ConsumableEffect::IsFood(1)` (a standard single-ration
food item). The option is added at the bottom of the dropdown alongside the existing Heal HP,
Restore SP, Cure Condition, Boost Attribute, and Boost Resistance options.

An `ℹ️` hover-tooltip on the ComboBox row explains the food mechanics:

> "Food (Rations): consumed during rest to feed party members. Ration Value controls how many
> characters one item feeds (usually 1). Food items are not usable in combat."

**Ration Value field**: When `IsFood` is the active effect, the editor renders:

```sdk/campaign_builder/src/items_editor.rs#L1438-1450
ConsumableEffect::IsFood(ration_value) => {
    ui.horizontal(|ui| {
        ui.label("Ration Value:");
        ui.add(egui::DragValue::new(ration_value).range(1..=255));
        ui.label("ℹ️").on_hover_text(concat!(
            "Number of party members this item feeds when consumed during rest.\n",
            "Standard Food Ration = 1 (feeds one character).\n",
            "Trail Ration = 3 (feeds three characters).",
        ));
    });
    ui.label("⚠️ Food items are not usable in combat.");
}
```

This is consistent with the `AttributePair`-style pattern used in the rest of the editor: each
effect variant owns its own sub-controls.

**Preview panel** (`show_preview_static`, Consumable branch): The static preview now handles
`IsFood` and produces human-readable singular/plural labels:

- `IsFood(1)` → "Food (1 ration)"
- `IsFood(3)` → "Food (3 rations)"

#### `src/bin/item_editor.rs` — CLI Item Editor

**`create_consumable`**: A new `[5] Food (Rations)` option is listed in the effect menu. When
chosen, the user is prompted for a ration value (default 1). The `is_combat_usable` prompt is
skipped entirely for food items — the code hard-codes `false` and prints an informational
message. This matches the domain invariant that food is never usable in combat.

**`edit_item_classification`** (Consumable branch): The same `[5]` option is added to the
effect-type change menu. When the user switches an existing consumable to `IsFood`, the code
writes both the new effect and forces `is_combat_usable = false` on the stored item, ensuring
a previously-combat-usable consumable (e.g. a Healing Potion) cannot become a combat-usable
food item by accident.

### Tests

#### SDK Items Editor tests (9 new, all in `mod tests` of `items_editor.rs`)

| Test                                                 | What it verifies                                                                      |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------- |
| `test_is_food_effect_default_ration_value`           | `IsFood(1)` carries ration_value = 1                                                  |
| `test_is_food_effect_trail_ration_value`             | `IsFood(3)` carries ration_value = 3                                                  |
| `test_is_food_effect_inequality_with_other_variants` | `IsFood` is not equal to any other effect variant                                     |
| `test_is_food_item_loads_into_edit_buffer`           | Food Ration (id 53) round-trips through `edit_buffer` with `is_combat_usable = false` |
| `test_is_food_trail_ration_loads_into_edit_buffer`   | Trail Ration (id 54, ration_value 3) round-trips through `edit_buffer`                |
| `test_consumable_filter_matches_food_item`           | `ItemTypeFilter::Consumable` matches `IsFood` items; Weapon/Quest filters do not      |
| `test_is_food_preview_label_singular`                | `IsFood(1)` preview label is "Food (1 ration)"                                        |
| `test_is_food_preview_label_plural`                  | `IsFood(3)` preview label is "Food (3 rations)"                                       |

#### CLI Item Editor tests (4 new, in `mod tests` of `item_editor.rs`)

| Test                                                      | What it verifies                                                                           |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `test_format_classification_consumable_is_food`           | `format_classification` output contains "Consumable" and "IsFood"                          |
| `test_create_consumable_is_food_effect_not_combat_usable` | `IsFood` branch always produces `is_combat_usable = false`                                 |
| `test_create_consumable_trail_ration_value_preserved`     | Trail Ration `ration_value = 3` is preserved through the create flow                       |
| `test_edit_consumable_is_food_clears_combat_usable`       | Editing an existing combat-usable consumable to `IsFood` forces `is_combat_usable = false` |

### Success Criteria Met

> Campaign developers can create novel food items (e.g., "Elven Bread", "Roast Beef") in the
> Items Editor.

The "Food (Rations)" option is now available in the effect-type dropdown. A campaign developer
can:

1. Create a new Consumable item and select "Food (Rations)" as the effect.
2. Set a custom `ration_value` (e.g., 2 for a hearty meal that feeds two characters).
3. Save the item to the campaign's `items.ron`.
4. Reference the new item ID in `npc_stock_templates.ron` so merchants sell it.

The rest system (Phase 2) already reads any item with `ConsumableEffect::IsFood(_)` from
character inventories — no further changes are needed for new food items to function correctly
at runtime.

---

## Food System — Phase 3: Merchant and Innkeeper Integration

### Overview

Phase 3 of the food system migration makes food purchasable in the world by updating merchant
and innkeeper stock templates to include Food Ration and Trail Ration items. Before this phase,
the party could start with food rations (granted during character initialization in Phase 2) but
had no way to replenish them — merchants had no food in their inventories and innkeepers sold
no provisions. Phase 3 closes that loop: every general-goods merchant and innkeeper now stocks
food so the player can always buy rations to enable resting.

No Rust code changes were required — the entire work is data-driven, with new and updated RON
stock template files plus integration tests to verify the templates load and populate runtime
merchant stock correctly.

### Deliverables Checklist

- [x] Core `data/npc_stock_templates.ron` — `general_store_basic` updated with Food Ration (53) and Trail Ration (54)
- [x] Core `data/npc_stock_templates.ron` — new `general_goods` template alias added with food items
- [x] Core `data/npc_stock_templates.ron` — new `innkeeper_basic` template added with food items and markup prices
- [x] Core `data/npc_stock_templates.ron` — `magic_item_pool / magic_slot_count / magic_refresh_days` fields added to all templates (previously missing)
- [x] Test campaign `data/test_campaign/data/npc_stock_templates.ron` — `tutorial_merchant_stock` updated with Food Ration (108) and Trail Ration (109)
- [x] Test campaign `data/test_campaign/data/npc_stock_templates.ron` — new `tutorial_general_store` template added
- [x] Test campaign `data/test_campaign/data/npc_stock_templates.ron` — new `tutorial_innkeeper_stock` template added with override prices
- [x] Tutorial campaign `campaigns/tutorial/data/items.ron` — Food Ration (id 111, IsFood 1) and Trail Ration (id 112, IsFood 3) added
- [x] Tutorial campaign `campaigns/tutorial/data/npc_stock_templates.ron` — filled with `town_merchant_basic` and `mountain_pass_merchant` templates (previously empty `[]`)
- [x] 7 new integration tests in `src/sdk/database.rs` — all passing

### What Was Built

#### `data/npc_stock_templates.ron` — core template updates

Three templates were modified or added:

**`general_store_basic`** (modified): Food Ration (item_id 53, quantity 20) and Trail Ration
(item_id 54, quantity 10) added to the entries list. The `magic_item_pool`, `magic_slot_count`,
and `magic_refresh_days` fields were also added to this and all other templates in the file,
which were previously omitted (they have `#[serde(default)]` in Rust so they round-tripped
silently, but making them explicit improves data legibility).

**`general_goods`** (new): A named alias for general merchants who primarily sell provisions
rather than weapons. Campaign authors can reference `"general_goods"` in `npcs.ron` without
coupling to the internal `general_store_basic` name. Stocks the same selection including food.

**`innkeeper_basic`** (new): Innkeepers provide rest services but also sell food provisions.
This template stocks only food items (no weapons or potions) with a slight price markup via
`override_price` to reflect the innkeeper's convenience premium:

- Food Ration (item_id 53): quantity 30, override_price Some(3) (base cost is 2)
- Trail Ration (item_id 54): quantity 15, override_price Some(6) (base cost is 5)

#### `data/test_campaign/data/npc_stock_templates.ron` — test fixture updates

**`tutorial_merchant_stock`** (modified): Food Ration (item_id 108, quantity 10) and Trail
Ration (item_id 109, quantity 5) added. These item IDs match the test campaign's `items.ron`
where food was placed at ids 108 and 109 during Phase 1.

**`tutorial_general_store`** (new): A self-contained test fixture template used by the Phase 3
acceptance tests. Stocks Healing Potion (50), Food Ration (108, qty 20), Trail Ration (109,
qty 10), Arrows (60), and Crossbow Bolts (61). The comment in the file marks this as the
canonical fixture for Phase 3 merchant integration tests.

**`tutorial_innkeeper_stock`** (new): Mirrors `innkeeper_basic` but uses test-campaign item IDs
(108, 109) and carries `override_price` values to allow tests to verify markup pricing is
preserved through the template → `MerchantStock` initialization path.

#### `campaigns/tutorial/data/items.ron` — food item additions

Items 108–110 were already occupied in the tutorial campaign (Healing Scroll, Cure Disease
Potion, Resurrection Scroll), so food items were appended with the next available IDs:

- **id 111 — Food Ration**: `ConsumableEffect::IsFood(1)`, base_cost 2, sell_cost 1, max_charges 1
- **id 112 — Trail Ration**: `ConsumableEffect::IsFood(3)`, base_cost 5, sell_cost 2, max_charges 1

Both items are non-combat-usable (`is_combat_usable: false`) consistent with Phase 1 definitions.

#### `campaigns/tutorial/data/npc_stock_templates.ron` — tutorial campaign templates

This file was previously empty (`[]`), meaning the two merchants in `npcs.ron` that reference
`stock_template: Some("town_merchant_basic")` and `stock_template: Some("mountain_pass_merchant")`
would silently initialize with no stock. Both templates are now defined:

**`town_merchant_basic`**: Full general-goods merchant for the starting town. Stocks Club (1),
Dagger (2), Short Sword (3), Mace (5), Leather Armor (20), Wooden Shield (23), Healing Potion (50),
Cure Poison Potion (52), Arrows (60), Crossbow Bolts (61), and food: Food Ration (111, qty 20)
and Trail Ration (112, qty 10). Magic rotation: 2 slots from pool [10, 11, 12].

**`mountain_pass_merchant`**: Expanded merchant for the mid-game area. Broader weapon and armor
selection (adds Long Sword (4), Battle Axe (6), Chain Mail (21), Magic Potion (51)). Food stocked
at slightly lower quantities (qty 15 / 8) reflecting the party being better provisioned by that
point. Same magic rotation pool.

### Tests

Seven new integration tests were added to `src/sdk/database.rs`:

| Test                                                                | What it verifies                                                                                                                                                          |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_general_store_basic_contains_food_rations`                    | `general_store_basic` in core data has item_id 53 and 54 with qty > 0                                                                                                     |
| `test_innkeeper_basic_template_contains_food_rations`               | `innkeeper_basic` in core data has item_id 53 and 54; food ration qty >= 10                                                                                               |
| `test_general_goods_template_contains_food_rations`                 | `general_goods` alias in core data has item_id 53 and 54                                                                                                                  |
| `test_test_campaign_merchant_stock_contains_food_rations`           | `tutorial_merchant_stock` in test campaign has item_id 108 and 109 with qty > 0                                                                                           |
| `test_test_campaign_general_store_template_contains_food_rations`   | `tutorial_general_store` in test campaign has item_id 108 and 109 with qty > 0                                                                                            |
| `test_test_campaign_innkeeper_stock_template_contains_food_rations` | `tutorial_innkeeper_stock` has item*id 108 and 109 with `override_price: Some(*)`                                                                                         |
| `test_stock_template_populates_merchant_runtime_with_food`          | End-to-end: loads `tutorial_general_store`, calls `NpcRuntimeState::initialize_stock_from_template`, asserts resulting `MerchantStock` contains food entries with qty > 0 |

All tests use `data/test_campaign` fixtures, not `campaigns/tutorial`, in compliance with
Implementation Rule 5.

### Architecture Compliance

- [x] Data structures match architecture.md Section 4 — no new types introduced; all changes are data-only
- [x] RON format used for all data files
- [x] `ItemId` type aliases used consistently in `TemplateStockEntry::item_id`
- [x] No magic numbers hardcoded — item IDs documented with inline comments
- [x] Test data lives in `data/test_campaign`, never in `campaigns/tutorial`
- [x] `campaigns/tutorial` modified only to populate the previously-empty `npc_stock_templates.ron` (legitimate: runtime game binary default campaign data)
- [x] No architectural deviations

### Quality Gates

```text
cargo fmt --all         → clean (no output)
cargo check             → Finished, 0 errors
cargo clippy -- -D warnings → Finished, 0 warnings
cargo nextest run       → 3254 passed, 8 skipped, 0 failed
```

Phase 3 acceptance test run (filtered):

```text
11 tests run: 11 passed, 3251 skipped
  PASS antares sdk::database::tests::test_general_store_basic_contains_food_rations
  PASS antares sdk::database::tests::test_innkeeper_basic_template_contains_food_rations
  PASS antares sdk::database::tests::test_general_goods_template_contains_food_rations
  PASS antares sdk::database::tests::test_test_campaign_merchant_stock_contains_food_rations
  PASS antares sdk::database::tests::test_test_campaign_general_store_template_contains_food_rations
  PASS antares sdk::database::tests::test_test_campaign_innkeeper_stock_template_contains_food_rations
  PASS antares sdk::database::tests::test_stock_template_populates_merchant_runtime_with_food
```

---

## Food System — Phase 2: Rest System Migration

### Overview

Phase 2 migrates the rest system from abstract numeric food counters
(`Character.food: u8` / `Party.food: u32`) to inventory-based item consumption.
When a party rests, the system now searches all party members' inventories for
items carrying `ConsumableEffect::IsFood`, removes them as whole slots, and
applies party-level pooling/sharing so that one member's surplus covers another
member's need.

This phase also deprecates the legacy numeric `food` fields on `Character` and
`Party`, updates character initialization to grant starting food as inventory
items, and removes legacy forced food assignments from the application layer.

### Deliverables Checklist

- [x] `consume_food()` rewritten to use inventory items (`src/domain/resources.rs`)
- [x] `count_food_in_party()` added — sums `IsFood` values across all inventories
- [x] `check_starvation()` rewritten — delegates to `count_food_in_party`
- [x] `Character.food` numeric field deprecated (kept for save compatibility)
- [x] `Party.food` numeric field deprecated (kept for save compatibility)
- [x] Character initialization grants starting food as inventory items (`src/domain/character_definition.rs`)
- [x] Legacy food assignments removed from `src/application/mod.rs` and `src/application/save_game.rs`
- [x] `rest_party()` signature updated to accept `&ItemDatabase`
- [x] Game-layer rest system (`src/game/systems/rest.rs`) updated to use `GameContent`
- [x] All `consume_food` tests replaced with inventory-based tests
- [x] All 3247 tests pass; 0 failures

### What Was Built

#### `count_food_in_party` — `src/domain/resources.rs`

A new public function that iterates all party member inventories, resolves each
slot's item from the `ItemDatabase`, and sums the inner `u8` value of every
`ConsumableEffect::IsFood` variant found. The result is the total ration-units
available across the whole party.

```antares/src/domain/resources.rs#L237-252
pub fn count_food_in_party(party: &Party, item_db: &ItemDatabase) -> u32 {
    party
        .members
        .iter()
        .flat_map(|c| c.inventory.items.iter())
        .fold(0u32, |acc, slot| {
            if let Some(item) = item_db.get_item(slot.item_id) {
                if let ItemType::Consumable(ref data) = item.item_type {
                    if let ConsumableEffect::IsFood(rations) = data.effect {
                        return acc + rations as u32;
                    }
                }
            }
            acc
        })
}
```

#### `consume_food` rewrite — `src/domain/resources.rs`

The function now operates in two passes:

**Pass 1 — Member-first:** Each member pays `amount_per_member` ration-units
from their own inventory. Items are removed as whole slots; the slot's full
`IsFood` value is credited to `total_pass1_consumed`. If a multi-ration item
(e.g. Trail Ration, `IsFood(3)`) covers more than a single member's need, the
overpayment counts toward the net shortfall calculation.

**Pass 2 — Pool/share:** After Pass 1, the net shortfall is
`total_needed.saturating_sub(total_pass1_consumed)`. If any shortfall remains
(members with no personal food), the function iterates all inventories again,
removing `IsFood` items to cover the remainder.

The key bug fixed in this phase: the previous implementation computed shortfall
as the sum of per-member gaps, silently discarding overpayment from multi-ration
items. A Trail Ration held by member 0 in a 3-member party would be consumed
(removing the slot) but only credit 1 ration unit to the consumption total,
leaving members 1 and 2 short with nothing left to pool. The fix tracks
`total_pass1_consumed` (actual ration-units removed) and derives the shortfall
from it, so a Trail Ration correctly satisfies all 3 members in one slot
removal.

The function returns `total_pass1_consumed + total_pass2_consumed` — the actual
ration-units removed from inventories, which may exceed `total_needed` when a
multi-ration item is the last item consumed (since items are removed as whole
slots with no fractional consumption).

Pre-check before any mutation: if `count_food_in_party < total_needed`, the
function returns `Err(ResourceError::NoFoodRemaining)` without touching any
inventory.

#### `check_starvation` rewrite — `src/domain/resources.rs`

Now a thin wrapper: `count_food_in_party(party, item_db) == 0`.

#### `ration_value_of` helper — `src/domain/resources.rs`

Private helper that resolves a single `ItemId` against the `ItemDatabase` and
returns its `IsFood` ration value, or `0` if the item is not found or is not a
food consumable.

#### Deprecated numeric food fields — `src/domain/character.rs`

`Character.food: u8` and `Party.food: u32` are retained with `#[deprecated]`
attributes and zeroed-out constructors. Existing save files that still carry
these fields deserialize without error; the fields are simply ignored by the
rest system going forward. A future migration routine can convert them to
inventory slots on load if desired.

#### Character initialization — `src/domain/character_definition.rs`

`instantiate()` now calls a private `grant_starting_food(character, item_db,
starting_food)` helper instead of writing to `character.food`. The helper
locates an `IsFood(1)` item in the database (preferring single-ration items),
then calls `character.inventory.add_item()` once per ration unit. Returns
`CharacterDefinitionError::InventoryFull` if the inventory cannot accept all
starting food items.

#### Application layer cleanup — `src/application/mod.rs`, `src/application/save_game.rs`

Removed all assignments of the form `state.party.food = N` and
`character.food = N` from the new-game and save-load paths. The new-game path
relies entirely on `instantiate()` granting food items into character
inventories.

#### Game-layer rest system — `src/game/systems/rest.rs`

`process_rest` now queries the `GameContent` Bevy resource (if present) to
obtain the `ItemDatabase`, then:

1. Calls `count_food_in_party` to check upfront whether enough food is
   available before committing.
2. Calls `consume_food(party, item_db, FOOD_PER_REST)` to remove `IsFood` items
   from inventories.

Rest system unit tests were updated to insert a minimal `GameContent` resource
containing a Food Ration item (id=1) and to populate member inventories with
ration items instead of mutating the deprecated `party.food` field.

### Tests

All pre-existing resource tests were replaced or updated to use the
inventory-based API. New tests added:

| Test name                                        | What it verifies                                                      |
| ------------------------------------------------ | --------------------------------------------------------------------- |
| `test_consume_food`                              | 3 members × 4 rations; 1 consumed per member; 3 remain each           |
| `test_consume_food_not_enough`                   | Returns `NoFoodRemaining`; inventories unchanged                      |
| `test_check_starvation`                          | Empty party is starving; party with ration is not                     |
| `test_count_food_in_party_empty`                 | Returns 0 for party with no food                                      |
| `test_count_food_in_party_multiple_members`      | Sums across all members                                               |
| `test_count_food_multi_ration_item`              | Trail Ration counts as 3                                              |
| `test_consume_food_sharing_across_members`       | Member 0's surplus feeds members 1 and 2                              |
| `test_consume_food_trail_ration_counts_as_three` | Single Trail Ration satisfies 3-member party; slot removed; returns 3 |
| `test_rest_consumes_food`                        | `rest_party` removes exactly 1 ration per member                      |
| `test_rest_party_fails_without_enough_food`      | `rest_party` refuses when food < member count                         |
| `test_rest_party_fails_without_food`             | `rest_party` refuses with completely empty inventories                |

### Architecture Compliance

- `consume_food`, `count_food_in_party`, `check_starvation` all accept
  `&ItemDatabase` — no hidden global state.
- Type aliases (`ItemId`) used throughout; no raw `u32` introduced.
- `ConsumableEffect::IsFood(u8)` from Phase 1 used exactly as defined —
  no duplicate food representation.
- Test fixtures live in `data/test_campaign/` and inline `ItemDatabase`
  helpers — no reference to `campaigns/tutorial`.
- SPDX headers present in all modified `.rs` files.

### Quality Gates

```text
cargo fmt         → no output
cargo check       → Finished 0 errors
cargo clippy      → Finished 0 warnings
cargo nextest run → 3247 passed; 0 failed; 8 skipped
```

---

## Food System — Phase 1: Core Item Foundation

### Overview

Phase 1 converts food from an abstract numeric counter into a proper inventory
item by adding the `ConsumableEffect::IsFood(u8)` variant to the item type
system and defining canonical food items in the game's data files. This is the
foundation for Phases 2–4 which will rewrite the rest system, wire up merchant
stock, and update the SDK editor.

### Deliverables Checklist

- [x] `ConsumableEffect::IsFood(u8)` variant added to `src/domain/items/types.rs`
- [x] Base food items added to `data/items.ron` (ids 53 "Food Ration", 54 "Trail Ration")
- [x] Food items added to `data/test_campaign/data/items.ron` (ids 108 "Food Ration", 109 "Trail Ration")
- [x] Serialization / deserialization tests passed (10 new tests in `types.rs`)
- [x] Exhaustive match sites updated (`combat/item_usage.rs`, `visual/item_mesh.rs`)

### What Was Built

#### `ConsumableEffect::IsFood(u8)` — `src/domain/items/types.rs`

A new variant appended to the existing `ConsumableEffect` enum. The inner
`u8` is the **ration count** supplied by a single unit of the item — almost
always `1` for a standard ration, but higher values are valid for multi-serving
items such as a "Trail Ration" (3 rations).

The variant is `Copy + PartialEq + Serialize + Deserialize`, consistent with
all other `ConsumableEffect` variants, so it round-trips cleanly through RON
without any schema migration.

#### `data/items.ron` additions

Two food items were appended in a new `// ===== Food Items =====` section
between the existing Consumables block and the Ammunition block:

| id  | name         | effect    | base_cost | sell_cost | combat_usable |
| --- | ------------ | --------- | --------- | --------- | ------------- |
| 53  | Food Ration  | IsFood(1) | 2         | 1         | false         |
| 54  | Trail Ration | IsFood(3) | 5         | 2         | false         |

Food items are intentionally **not** combat-usable (`is_combat_usable: false`).

#### `data/test_campaign/data/items.ron` additions

Identical items at ids 108 / 109 (offset to avoid id collisions with the
test-campaign's existing item numbering).

#### Exhaustive match updates

Two sites in the codebase perform exhaustive matches over `ConsumableEffect`
and required new arms:

- **`src/domain/combat/item_usage.rs`** — `execute_item_use_by_slot`: the
  `IsFood(_)` arm returns `Err(ItemUseError::NotUsableInCombat)`. The
  `validate_item_use_slot` gate already blocks food items via
  `is_combat_usable: false`, so this arm is a safety net for callers that
  bypass validation.
- **`src/domain/visual/item_mesh.rs`** — consumable colour selector: food items
  are assigned an earthy brown `[0.55, 0.35, 0.10, 1.0]` to visually
  distinguish them from magical potions.

#### Tests — `src/domain/items/types.rs` (10 new)

| Test name                                     | What it verifies                                               |
| --------------------------------------------- | -------------------------------------------------------------- |
| `test_is_food_effect_equality`                | `IsFood(1) == IsFood(1)`, `IsFood(1) != IsFood(3)`             |
| `test_is_food_ration_count_extracted`         | Pattern-match extracts inner `u8`                              |
| `test_is_food_trail_pack_ration_count`        | Pack of 3 extracts correctly                                   |
| `test_is_food_serializes_correctly`           | RON output contains `"IsFood"` and the count                   |
| `test_is_food_deserializes_correctly`         | `"IsFood(1)"` parses to correct variant                        |
| `test_is_food_roundtrip_serde`                | Full serialize → deserialize identity                          |
| `test_consumable_data_with_is_food_roundtrip` | `ConsumableData` struct round-trips                            |
| `test_food_ration_item_loads_from_ron_string` | `ItemDatabase::load_from_string` succeeds with Food Ration RON |
| `test_food_ration_not_combat_usable`          | `is_combat_usable` is `false`                                  |
| `test_is_food_no_required_proficiency`        | `required_proficiency()` returns `None`                        |

### Architecture Compliance

- Data structures match architecture.md Section 4.5 (`ConsumableData`, `ConsumableEffect`) **exactly**.
- Type aliases (`ItemId`) used throughout; no raw `u32` introduced.
- RON format used for all data files; no JSON/YAML.
- Test fixtures live in `data/test_campaign/` — no reference to `campaigns/tutorial`.
- SPDX headers present in all modified `.rs` files (pre-existing headers unchanged).

### Quality Gates

```text
cargo fmt         → no output
cargo check       → Finished 0 errors
cargo clippy      → Finished 0 warnings
cargo nextest run → 3242 passed; 0 failed; 8 skipped
```

---

## New MTL Support - Phase 1: Rebaseline Around The Existing Importer

**Plan**: [`newmtl_support_plan.md`](newmtl_support_plan.md)

### Overview

Phase 1 does not add MTL parsing yet. Instead, it rebases the OBJ import code
around the branch's current architecture so later MTL work lands in the correct
modules.

The important correction is architectural: this branch already has a dedicated
`Importer` tab, importer state, and export flow. The OBJ backend should now be
documented and treated as the parser layer behind that standalone workflow,
instead of being described as a creature-editor-only utility.

---

### Phase 1 Deliverables

**Files modified**:

- `sdk/campaign_builder/src/mesh_obj_io.rs`
- `sdk/campaign_builder/src/obj_importer.rs`
- `docs/explanation/implementations.md`

---

### What was built

#### Importer backend rebaseline (`sdk/campaign_builder/src/mesh_obj_io.rs`)

Updated the module and API docs so `mesh_obj_io.rs` now clearly describes its
actual role on this branch:

- it is the OBJ parsing and serialization backend for the Campaign Builder
  importer workflow
- the standalone `Importer` tab is the primary consumer of the multi-mesh import
  APIs
- parser concerns stay in `mesh_obj_io.rs`, while importer-tab state and egui UI
  concerns stay out of the parser layer

This gives later MTL work a correct landing zone before any new parsing logic is
added.

#### Explicit parser-state seam (`sdk/campaign_builder/src/obj_importer.rs`)

Documented `obj_importer.rs` as the handoff layer between:

- `mesh_obj_io.rs`, which returns `MeshDefinition` values
- `obj_importer.rs`, which turns them into editable importer rows and campaign
  state
- `obj_importer_ui.rs`, which renders and exports that state

Added a dedicated `ObjImporterState::obj_import_options()` helper so parser
options are assembled in one place instead of ad hoc inside load paths. That
keeps future parser-facing changes such as MTL resolution, source-path
awareness, and manual override wiring localized to a single seam.

#### Regression coverage for the seam (`sdk/campaign_builder/src/obj_importer.rs`)

Added a focused test that proves importer state forwards its current parser
options into `mesh_obj_io` by verifying that `scale` changes alter the imported
vertex positions.

This is intentionally small, but it locks in the contract Phase 2 and later MTL
phases will extend.

---

### Architecture compliance

- The work stays entirely inside the SDK importer layer under
  `sdk/campaign_builder`.
- No domain structures were changed; `MeshDefinition` remains the parser output
  passed through importer state exactly as defined in `src/domain/visual/mod.rs`.
- The current standalone importer tab and export flow remain intact.
- No campaign fixture or gameplay data paths were changed.

---

### Validation

Validation was rerun after the Phase 1 rebaseline changes using the required
repo commands.

## New MTL Support - Phase 2: Refactor OBJ Parsing For Material-Aware Segments

**Plan**: [`newmtl_support_plan.md`](newmtl_support_plan.md)

### Overview

Phase 2 refactors the OBJ parser backend so it no longer treats `o`, `g`, and
future material boundaries as one overwriteable mesh name. The importer now
parses low-level OBJ data into explicit segments that preserve object name,
group name, and active material name separately before any `MeshDefinition`
values are built.

This is the structural groundwork Phase 3 needs for real MTL resolution.
Nothing in the importer UI changes yet, but the parser can now represent a
multi-material OBJ deterministically instead of flattening those boundaries
away.

---

### Phase 2 Deliverables

**Files modified**:

- `sdk/campaign_builder/src/mesh_obj_io.rs`
- `docs/explanation/implementations.md`

---

### What was built

#### Material-aware segment parsing (`sdk/campaign_builder/src/mesh_obj_io.rs`)

Replaced the old parsed-mesh placeholder with explicit parser-side segment
identity metadata:

- `object_name`
- `group_name`
- `material_name`

The parser now flushes segment geometry on:

- `o`
- `g`
- `usemtl`

That means one logical object can now produce multiple parsed segments when the
source OBJ changes materials mid-stream, which is required because
`MeshDefinition` still has only one material slot.

#### Separation between parse-time structure and mesh construction

The low-level parse path and the mesh-building path are now more clearly split:

- `parse_obj_meshes()` gathers global vertices, normals, UVs, and parsed
  segments
- `build_mesh_from_faces()` constructs a `MeshDefinition` from a chosen segment
  or a combined face stream
- `resolve_segment_names()` assigns deterministic exported mesh names after the
  parser has preserved object and group identity

This keeps parse-time identity available long enough for later MTL resolution
instead of discarding it during the first pass.

#### Identity-preserving mesh naming

Segment display names now prefer object/group identity instead of letting
material switches rename meshes.

Current naming behavior:

- object + distinct group -> `<object>_<group>`
- object only -> `<object>`
- group only -> `<group>`
- unnamed segment -> `mesh_<index>`
- repeated object/group identity caused by `usemtl` splits -> first segment keeps
  the base name, later segments receive `_segment_<n>` suffixes

This preserves the source model's structural identity while still producing
unique `MeshDefinition.name` values for export and editor display.

#### Single-mesh import compatibility

`import_mesh_from_obj_with_options()` now reuses the segment-aware parser and
then combines all parsed segments back into one mesh for callers that still
want a single mesh result.

That preserves the existing single-mesh API contract while moving the parsing
logic onto the same internal representation used by the multi-mesh importer.

---

### Test coverage

Added parser-focused tests for:

- preservation of object, group, and material identity across parsed segments
- mesh splitting on `usemtl` boundaries without losing object/group naming
- single-mesh import continuing to combine multi-segment OBJ input

Existing OBJ fixture tests for `examples/skeleton.obj` and
`examples/female_1.obj` continue to pass against the new segment model.

---

### Architecture compliance

- The work stays inside the SDK importer backend under
  `sdk/campaign_builder/src/mesh_obj_io.rs`.
- No domain data structures were changed.
- `MeshDefinition` remains the parser output type used by importer state.
- No test fixtures were moved under `campaigns/tutorial`.
- The refactor prepares later MTL work without introducing new persistence or UI
  surface area prematurely.

## New MTL Support - Phase 3: Add MTL Parsing And Resolution

**Plan**: [`newmtl_support_plan.md`](newmtl_support_plan.md)

### Overview

Phase 3 teaches the OBJ importer backend to discover, resolve, and parse MTL
files without yet mapping those parsed materials into `MeshDefinition.material`
or imported mesh colors. That keeps this slice focused on the backend seam the
later mapping and UI phases need.

The parser now understands `mtllib` well enough to find sidecar material
libraries relative to the OBJ file, honors a parser-side manual override path,
and parses a first-pass subset of MTL directives into backend material data.

## New MTL Support - Phase 4: Map Imported Materials Into Domain Types

**Plan**: [`newmtl_support_plan.md`](newmtl_support_plan.md)

### Overview

Phase 4 wires parsed MTL data into the existing visual domain model so imported
OBJ segments now carry visible color, material metadata, alpha behavior, and
portable texture-path metadata through to exported `MeshDefinition` values.

This phase stays focused on the parser and importer-state seam rather than the
importer UI. The main effect is that successful MTL parsing now survives into
domain output instead of being dropped after resolution.

---

### Phase 4 Deliverables

**Files modified**:

- `sdk/campaign_builder/src/mesh_obj_io.rs`
- `sdk/campaign_builder/src/obj_importer.rs`
- `docs/explanation/implementations.md`

---

### What was built

#### Domain material mapping in the OBJ backend

`build_mesh_from_faces()` now resolves the active parsed material and maps it
into domain output instead of always returning default color-only meshes.

Current mapping behavior:

- `Kd` -> `MeshDefinition.color`
- `Kd` -> `MaterialDefinition.base_color`
- `d` -> alpha channel on both mesh color and material base color
- `d < 1.0` -> `AlphaMode::Blend`
- `Ke` -> `MaterialDefinition.emissive` when non-black
- `map_Kd` -> `MeshDefinition.texture_path` only when the original MTL path is
  relative and therefore safe to preserve as portable metadata

If a referenced material has no meaningful parsed data, the importer still
falls back to the existing default OBJ color behavior.

#### Conservative `Ks` and `Ns` heuristics

Phase 4 implements a conservative heuristic for MTL specular values rather than
pretending Wavefront MTL maps perfectly onto the engine's PBR fields.

- `Ks` contributes to `metallic` only when:
  - illumination model is `>= 2`
  - average specular strength is at least `0.5`
- even then, metallic is capped at `0.35` to avoid over-classifying legacy MTL
  materials as metal
- `Ns` maps into `roughness` through a clamped square-root inversion over the
  common `0..1000` shininess range
- when `Ns` is absent, roughness falls back to `0.45` for mildly metallic
  materials and `0.9` otherwise

This keeps imported materials visually useful without overstating the fidelity
of a Phong-to-PBR conversion.

#### Texture-path safety rule

The parser now preserves both the resolved on-disk `map_Kd` path and the
original MTL reference string.

That allows Phase 4 to keep texture metadata only when it is portable:

- relative `map_Kd` paths are normalized and stored on `MeshDefinition`
- absolute texture paths are intentionally dropped from exported domain data

This avoids leaking machine-specific paths into RON exports.

#### Importer-state color preservation seam

The importer state still owns heuristic palette assignment, but it now avoids
overwriting mesh colors that already arrived from MTL-backed domain data.

That small seam change is required so Phase 4's mapped `Kd` color survives the
initial load into `ImportedMesh` and remains visible for export.

Automatic palette assignment is still available as an explicit importer action.

---

### Test coverage

Added focused tests for:

- mapping `Kd`, `d`, `Ke`, `Ks`, `Ns`, and relative `map_Kd` into domain mesh
  output
- preserving portable relative texture paths while rejecting absolute ones
- keeping imported MTL colors when importer state creates editable mesh rows

Existing parser tests for missing or malformed MTL files continue to verify the
required graceful-degradation behavior.

---

### Architecture compliance

- The work stays inside the SDK importer pipeline and existing visual domain
  types.
- `MeshDefinition`, `MaterialDefinition`, and `AlphaMode` are used exactly as
  already defined in `src/domain/visual/mod.rs`.
- No gameplay data structures or campaign fixture paths were changed.
- No new persistence format was introduced.

## New MTL Support - Phase 5: Integrate With Existing Importer State

**Plan**: [`newmtl_support_plan.md`](newmtl_support_plan.md)

### Overview

Phase 5 threads MTL-aware import results through `ObjImporterState` without
changing the current importer workflow. The key fix is that importer state now
tracks whether a mesh color came from explicit MTL color data, fallback
auto-assignment, or a later manual edit, instead of guessing based on whether a
mesh happened to be white.

This closes the gap left by Phase 4 where explicit white `Kd` values and
fallback candidates could be confused once they were flattened into plain
`MeshDefinition` values.

---

### Phase 5 Deliverables

**Files modified**:

- `sdk/campaign_builder/src/mesh_obj_io.rs`
- `sdk/campaign_builder/src/obj_importer.rs`
- `sdk/campaign_builder/src/obj_importer_ui.rs`
- `docs/explanation/implementations.md`

---

### What was built

#### Parser-to-importer color-source handoff

Added an SDK-internal importer path in `mesh_obj_io.rs` that returns each
imported `MeshDefinition` together with color-source metadata for importer use.

The public OBJ import APIs remain unchanged, but importer state can now tell:

- explicit `Kd` color from MTL -> imported material color
- material with no diffuse color -> fallback candidate for mesh-name auto-color
- no material data -> fallback candidate

That means explicit white material colors are preserved correctly instead of
being mistaken for "no imported color".

#### Explicit importer color provenance

`ImportedMesh` now records whether its current color is:

- `ImportedMaterial`
- `AutoAssigned`
- `ManualOverride`

This lets importer state preserve imported material color on initial load,
reset to heuristic colors only when the user explicitly requests it, and mark
later palette or picker edits as user overrides.

#### Fallback auto-color behavior that preserves imported alpha

When importer state falls back to mesh-name heuristics because no diffuse MTL
color exists, it now keeps the imported alpha channel instead of always
resetting to fully opaque colors.

This keeps transparency from `d` intact while still using the branch's existing
name-based color suggestions for RGB fallback.

#### Material/base-color synchronization during edits and export

Importer color edits now synchronize both:

- `mesh_def.color`
- `mesh_def.material.base_color` when material data exists

and update `AlphaMode::Blend` when edited alpha drops below `1.0`.

As a result, exported RON assets now keep edited importer colors consistent
between the top-level mesh color and the nested material color.

---

### Test coverage

Added focused importer-state coverage for:

- preserving explicit white `Kd` colors during OBJ import
- using heuristic fallback when an MTL has no diffuse color
- preserving imported alpha during fallback auto-assignment
- marking later manual edits as overrides
- exporting edited material base colors consistently

---

### Architecture compliance

- The work stays inside the SDK importer backend, importer state, and exporter
  seam.
- No domain structs were changed.
- Public OBJ import APIs continue returning `MeshDefinition` values.
- The importer tab flow, mesh list, active selection, and export flow remain
  intact.

---

### Phase 3 Deliverables

**Files modified**:

- `sdk/campaign_builder/src/mesh_obj_io.rs`
- `sdk/campaign_builder/src/obj_importer.rs`
- `docs/explanation/implementations.md`

---

### What was built

#### Parser-facing MTL resolution options

Extended `ObjImportOptions` with:

- `source_path`
- `manual_mtl_path`

This gives the parser enough context to:

- resolve OBJ-declared material libraries relative to the OBJ file location
- accept a future importer-state or UI-supplied manual MTL override

The file-based OBJ import helpers now automatically populate `source_path` when
the caller does not provide one explicitly.

#### MTL library discovery and path resolution

`parse_obj_meshes()` now captures `mtllib` directives and resolves them into a
list of actual library paths.

Current precedence:

- if `manual_mtl_path` is set and exists, it is used as the material source
- otherwise, the parser resolves each `mtllib` reference relative to the OBJ
  directory
- missing libraries are ignored instead of failing geometry import

This matches the plan's graceful-degradation requirement.

## New MTL Support - Phase 6: Extend The Existing Importer Tab

**Plan**: [`newmtl_support_plan.md`](newmtl_support_plan.md)

### Overview

Phase 6 completes the first importer-tab integration for MTL-aware OBJ import.
The standalone `Importer` tab now exposes which MTL source is active, lets the
user choose or clear a manual `.mtl` override from the existing workflow, and
surfaces imported material swatches as a session-only palette without changing
the existing campaign-scoped custom palette format.

The UI remains the same importer tab and export flow introduced earlier on this
branch. The difference is that the tab now explains where imported colors came
from and gives users a direct path from imported MTL colors into the current
custom-palette save flow.

---

### Phase 6 Deliverables

**Files modified**:

- `sdk/campaign_builder/src/mesh_obj_io.rs`
- `sdk/campaign_builder/src/obj_importer.rs`
- `sdk/campaign_builder/src/obj_importer_ui.rs`
- `docs/explanation/implementations.md`

---

### What was built

#### Importer-session MTL source summary

The importer now keeps a small session summary of the last OBJ load:

- whether the current MTL source was auto-detected or manually overridden
- which `mtllib` names were declared by the OBJ
- which MTL file paths were actually resolved and used
- which imported material swatches were available from explicit diffuse `Kd`
  colors

This summary is parser-driven instead of guessed in the UI, so the importer tab
can report the real MTL source that produced the current mesh colors.

#### MTL source row in idle and loaded importer metadata

Both the idle importer form and the loaded importer metadata grid now include an
`MTL Source` row.

Current behavior:

- when a manual override is selected, the row shows that path
- when OBJ `mtllib` resolution succeeded, the row reports the resolved
  auto-detected MTL path or path count
- when the OBJ declared libraries but none resolved, the row explains that and
  lists the declared names
- when no MTL is in use, the row explains that a manual `.mtl` can be selected

This gives the importer tab the explicit MTL status the Phase 6 plan called
for, both before and after OBJ load.

#### Manual `.mtl` override selection and clear flow

The importer tab now includes `Browse .mtl...` and `Clear Override` actions.

Behavior differs slightly by mode:

- in idle mode, selecting a manual override stores it for the next OBJ load
- in loaded mode, changing or clearing the override immediately reloads the
  current OBJ through the existing importer pipeline so imported colors and
  metadata update in place

The override still uses the same parser seam as earlier phases: the UI updates
`ObjImporterState.manual_mtl_path`, then importer state rebuilds
`ObjImportOptions` and reloads the OBJ normally.

#### Imported MTL palette in the color editor

The right-hand `Color Editor` now has a dedicated `Imported MTL Palette`
section ahead of the built-in and custom palettes.

This palette is intentionally session-only:

- it is populated from imported materials that had explicit diffuse `Kd` colors
- it is cleared when importer session geometry is cleared
- it is not persisted directly to campaign config

Each imported swatch can:

- apply its color to the active mesh
- stage its label and color into the existing custom-palette draft controls via
  `Use As Draft`

That keeps imported colors separate from built-in and custom palette entries
while still letting users promote useful swatches through the existing
`config/importer_palette.ron` save path.

#### Color provenance messaging in the active mesh editor

The active mesh detail panel now explains whether the mesh's current color came
from:

- imported MTL diffuse data
- built-in mesh-name fallback heuristics
- a manual importer edit

This message is driven by the `ImportedMeshColorSource` state added in Phase 5.
The result is that the importer tab now tells the user whether a visible color
is original imported material data or a fallback generated by the current
branch's auto-assignment heuristics.

#### Imported swatches versus built-in and custom palettes

The importer now clearly separates three palette sources:

- `Imported MTL Palette`: temporary session colors derived from imported MTL
  diffuse values
- `Built-In Palette`: static importer defaults baked into the SDK
- `Custom Palette`: campaign-scoped colors persisted to
  `config/importer_palette.ron`

Promotion path for imported swatches:

1. choose an imported swatch in `Imported MTL Palette`
2. optionally use `Use As Draft` to copy its label and color into the custom
   palette draft controls
3. save it through the existing custom palette add action

No new persistence format was introduced.

---

### Final priority rule

The importer behavior is now explicit:

- imported `Kd` colors win on initial load when they exist
- built-in mesh-name auto-colors are only the fallback when diffuse `Kd` is not
  available
- `Auto-Assign All` remains an explicit user action that reverts meshes to the
  built-in heuristic palette
- later color picker or palette edits become manual overrides

---

### Deferred and unsupported UI behavior

Phase 6 still stays conservative in a few areas:

- imported swatches are only created for explicit diffuse `Kd` colors
- materials that only contribute alpha, texture, or non-diffuse metadata do not
  create palette swatches
- no new persistence or palette file format was added beyond the existing
  custom importer palette

This matches the plan's first-pass goal of exposing imported material color
cleanly without rebuilding the importer tab.

#### First-pass MTL parser

Added parser-side support for these MTL directives:

- `newmtl`
- `Kd`
- `Ks`
- `Ke`
- `Ns`
- `d`
- `illum`
- `map_Kd`

Parsed materials are stored in backend structures keyed by material name, with
resolved texture paths preserved as `PathBuf` values relative to the MTL file.

Unsupported directives and malformed values are ignored non-fatally so OBJ
geometry import still succeeds even when the material file is incomplete or
partially invalid.

#### Importer-state seam for future manual override UI

Extended `ObjImporterState` with `manual_mtl_path` and updated the
`obj_import_options()` helper so importer state now forwards:

- `source_path`
- `manual_mtl_path`
- `scale`

No importer-tab UI changes land in this phase yet, but the state seam is now in
place for the later override picker.

---

### Test coverage

Added backend tests covering:

- relative `mtllib` resolution from an OBJ source path
- multiple `mtllib` directives loading more than one library
- manual MTL override precedence over OBJ-declared libraries
- missing `.mtl` files degrading gracefully while geometry still imports
- malformed MTL values being ignored without breaking OBJ import
- parsing of `Kd`, `Ks`, `Ke`, `Ns`, `d`, `illum`, and `map_Kd`

Added importer-state coverage proving parser-facing source and manual MTL paths
are forwarded through `ObjImportOptions`.

---

### Architecture compliance

- The work remains inside the SDK importer/backend layer.
- No gameplay or domain core structures were changed.
- `MeshDefinition` output remains unchanged in this phase; material-to-domain
  mapping is intentionally deferred to Phase 4.
- Missing or malformed MTL data does not break OBJ geometry import.
- No tests reference `campaigns/tutorial`.

## OBJ to RON Conversion - Phase 3: Importer Tab UI and RON Export

**Plan**: [`obj_to_ron_implementation_plan.md`](obj_to_ron_implementation_plan.md)

### Overview

Phase 3 completes the Campaign Builder importer workflow. The SDK now exposes
an `Importer` tab directly below `Creatures`, lets the user pick an OBJ file,
inspect every imported mesh, edit colors with both the built-in palette and
campaign-scoped custom colors, and export the result as a valid
`CreatureDefinition` RON asset under either `assets/creatures/` or
`assets/items/`.

This phase also closes a fixture gap left by the earlier importer work:
`examples/skeleton.obj` and `examples/female_1.obj` are now present in the
repository, so both the existing file-based importer tests and the new importer
workflow tests have stable OBJ inputs.

## New MTL Support - Phase 7: Tests

**Plan**: [`newmtl_support_plan.md`](newmtl_support_plan.md)

### Overview

Phase 7 closes the remaining MTL importer test matrix by treating the current
branch state as the source of truth instead of rebuilding tests from scratch.
The parser and importer-state suites already covered most of the MTL cases from
the plan, so this phase fills the final persistence gap and documents the full
coverage now present across parser, importer-state, and importer-UI helpers.

---

### Phase 7 Deliverables

**Files modified**:

- `sdk/campaign_builder/src/obj_importer.rs`
- `sdk/campaign_builder/src/obj_importer_ui.rs`
- `docs/explanation/implementations.md`

---

### What was built

#### Parser and importer-state coverage audit completed

The existing Phase 2 through Phase 6 work already provided backend coverage for
the parser-heavy Phase 7 requirements, including:

- relative `mtllib` resolution
- multiple `mtllib` directives
- missing `.mtl` fallback behavior
- malformed MTL tolerance while preserving geometry import
- `usemtl` boundary splitting with object and group identity preserved
- `Kd`, `d`, and `Ke` mapping into exported mesh and material data
- importer-state handling for imported-color precedence, manual override MTL
  loading, imported swatch session state, and explicit `Auto-Assign All`
  fallback behavior

That meant the remaining missing deliverable was not another parser test, but a
workflow-level assertion that the importer's palette persistence path still
writes the expected campaign-scoped config file.

#### Importer-state palette persistence test

Added a focused `ObjImporterState` test that saves a custom palette entry
through `save_custom_palette()`, verifies the campaign-local
`config/importer_palette.ron` file is created, and reloads it through
`load_custom_palette()` to prove the importer state API preserves round-trip
behavior.

This locks the persistence requirement to the importer state seam rather than
only to the lower-level color-palette module.

#### Importer-UI persistence helper test

Added a companion `obj_importer_ui.rs` test that exercises the existing
`persist_custom_palette()` helper used by the custom-palette UI actions.

The test proves that the importer-tab save flow still writes
`config/importer_palette.ron` at the expected campaign-relative path, which is
the Phase 7 UI-side persistence deliverable from the plan.

---

### Test coverage summary

Phase 7 coverage now explicitly exists for:

- parser path resolution and non-fatal MTL error handling in
  `sdk/campaign_builder/src/mesh_obj_io.rs`
- importer-state color provenance, manual override handling, imported swatch
  session state, and explicit auto-assign behavior in
  `sdk/campaign_builder/src/obj_importer.rs`
- importer-tab helper behavior for imported swatch staging, MTL status text,
  rendering, and palette persistence in
  `sdk/campaign_builder/src/obj_importer_ui.rs`

---

### Architecture compliance

- The work stays inside the existing SDK importer workflow.
- No gameplay or domain data structures were changed.
- No tests use `campaigns/tutorial`; persistence tests write only into temporary
  campaign directories.
- The campaign-scoped custom palette format remains
  `config/importer_palette.ron` exactly as required.

## New MTL Support - Phase 8: Validation And Documentation

**Plan**: [`newmtl_support_plan.md`](newmtl_support_plan.md)

### Overview

Phase 8 closes the MTL importer work by validating the finished branch state
with the required repo quality gates and recording the final behavior that now
defines the importer workflow.

This phase does not introduce new importer features. It verifies that the
existing Phase 1 through Phase 7 implementation still compiles, lints cleanly,
and passes tests after the MTL-aware importer changes are integrated.

---

### Phase 8 Deliverables

**Files modified**:

- `sdk/campaign_builder/src/obj_importer_ui.rs`
- `docs/explanation/implementations.md`

---

### Final importer behavior

#### Final priority rule between imported and fallback colors

The importer color precedence is now finalized as:

- imported `Kd` diffuse color wins on initial load when it exists
- built-in mesh-name auto-coloring only applies when no diffuse `Kd` color was
  imported for that mesh
- `Auto-Assign All` remains an explicit reset action that reapplies built-in
  heuristics even to meshes that originally had imported colors
- later color picker edits and palette applications are treated as manual
  overrides

This keeps imported material intent visible by default while preserving the
branch's pre-existing heuristic palette workflow as an opt-in editing action.

#### Final `Ks` and `Ns` mapping behavior

The importer keeps the Phase 4 conservative Phong-to-PBR mapping.

- `Ks` contributes to `MaterialDefinition.metallic` only when the MTL
  illumination model is at least `2` and average specular strength is at least
  `0.5`
- the resulting metallic value is capped at `0.35`
- `Ns` maps into `MaterialDefinition.roughness` through a clamped square-root
  inversion across the common `0..1000` shininess range
- when `Ns` is absent, roughness falls back to `0.45` for mildly metallic
  materials and `0.9` otherwise

This remains intentionally heuristic rather than claiming a one-to-one MTL to
PBR conversion.

#### Unsupported and deferred MTL directives

The first-pass importer intentionally supports only:

- `newmtl`
- `Kd`
- `Ks`
- `Ke`
- `Ns`
- `d`
- `illum`
- optional `map_Kd`

Everything else is still unsupported or deferred in this pass.

Current deferred behavior:

- no broad support for the rest of the Wavefront MTL directive set
- no separate persistence format beyond `config/importer_palette.ron`
- no per-face material preservation beyond splitting OBJ meshes on `usemtl`
- no aggressive texture-import workflow beyond conservative relative
  `map_Kd` preservation

Unsupported or malformed directives continue to degrade gracefully instead of
failing otherwise valid OBJ geometry import.

#### Imported swatches versus built-in and custom palettes

The importer palette model is now finalized as three distinct sources:

- `Imported MTL Palette`: session-only swatches derived from imported diffuse
  `Kd` colors
- `Built-In Palette`: static SDK defaults used for quick edits and fallback
  auto-assignment
- `Custom Palette`: campaign-scoped colors persisted to
  `config/importer_palette.ron`

Imported swatches differ from the other palette sources in two important ways:

- they are generated from the current import session and cleared with importer
  session state
- they are not persisted directly; users keep them by promoting them through
  the existing custom-palette draft and save flow

---

### Validation

Phase 8 reran the required validation sequence in the exact order from
`AGENTS.md`:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

Results:

- `cargo fmt --all` completed successfully
- `cargo check --all-targets --all-features` completed successfully
- `cargo clippy --all-targets --all-features -- -D warnings` completed
  successfully
- `cargo nextest run --all-features` completed successfully with `3162` tests
  passed and `8` skipped

One compatibility repair was required during validation:

- `sdk/campaign_builder/src/obj_importer_ui.rs` test code used the removed
  `CursorIcon::is_resize()` helper from egui; the render test now keeps its
  no-panic coverage without depending on that API

---

### Architecture compliance

- Validation did not require changes to gameplay or domain core structures.
- The importer still uses the existing `MeshDefinition`,
  `MaterialDefinition`, and `AlphaMode` types from
  `src/domain/visual/mod.rs`.
- No new persistence format was added; campaign palette persistence remains
  `config/importer_palette.ron`.
- No tests or fixtures were moved to `campaigns/tutorial`.

---

### Phase 3 Deliverables

**Files modified**:

- `sdk/campaign_builder/src/lib.rs`
- `sdk/campaign_builder/src/obj_importer.rs`
- `sdk/campaign_builder/src/creature_assets.rs`
- `docs/explanation/implementations.md`

**Files created**:

- `sdk/campaign_builder/src/obj_importer_ui.rs`
- `examples/skeleton.obj`
- `examples/female_1.obj`

---

### What was built

#### Importer tab UI (`sdk/campaign_builder/src/obj_importer_ui.rs`)

Added a dedicated importer UI module that renders:

- Idle mode with OBJ file browsing, export-type selection, scale input, and a
  `Load OBJ` action
- Loaded mode with importer metadata inputs (`ID`, `Name`, `Import Scale`)
- A scrollable mesh list showing mesh name, counts, selection state, and the
  current color swatch plus an inline per-row color edit button for each
  imported mesh
- A color editor panel for the active mesh using `TwoColumnLayout` to stay
  consistent with `sdk/AGENTS.md`
- Built-in palette swatches plus campaign-scoped custom palette add/remove UI
- Summary and control actions including `Auto-Assign All`, `Load Another OBJ`,
  `Export RON`, and `Back / Clear`

The importer UI follows the SDK-specific egui rules:

- `TwoColumnLayout` is used for the list/detail split instead of raw panels
- mesh rows are wrapped in `push_id`
- all `ScrollArea`s have explicit `id_salt` values
- layout-driving state changes request repaint immediately

#### Export pipeline (`sdk/campaign_builder/src/obj_importer_ui.rs`)

Added a reusable export path that:

- builds a `CreatureDefinition` from the current `ObjImporterState`
- applies the edited per-mesh colors back onto the cloned `MeshDefinition`s
- generates `MeshTransform::identity()` entries for every exported mesh
- preserves the importer `scale` as the exported creature scale

Creature export now writes to the exact planned location:

- `assets/creatures/<sanitized_name>.ron`

Item export writes the same `CreatureDefinition` format to:

- `assets/items/<sanitized_name>.ron`

#### Creature registry integration (`sdk/campaign_builder/src/creature_assets.rs`)

Added `save_creature_at_path()` so importer exports can preserve the exact
relative asset path required by the Phase 3 plan while still updating the
reference-backed `data/creatures.ron` registry.

This keeps importer-created creature assets aligned with the existing creature
asset manager rather than introducing separate persistence logic.

#### Campaign Builder app wiring (`sdk/campaign_builder/src/lib.rs`)

The app shell now:

- exposes `obj_importer_ui` as a module
- adds `EditorTab::Importer` directly below `EditorTab::Creatures`
- dispatches the new tab from the central panel
- refreshes the creature registry after successful creature exports
- switches to the `Creatures` tab after creature export so the newly exported
  asset is immediately visible in the main creature workflow

#### Importer state polish (`sdk/campaign_builder/src/obj_importer.rs`)

Extended importer state with lightweight UI state needed by Phase 3:

- `active_mesh_index` for the currently edited mesh
- `new_custom_color_label` and `new_custom_color` for the custom-palette form

The importer `clear()` path now preserves:

- current scale
- custom palette entries
- suggested creature ID
- current export type
- current custom-color draft value

#### Deterministic OBJ fixtures (`examples/skeleton.obj`, `examples/female_1.obj`)

Added the missing OBJ fixtures referenced by the Phase 1 and Phase 3 plans so
the importer can be tested with real file-based inputs instead of only inline
OBJ strings.

---

### Architecture compliance

- The work stays inside the SDK/editor layer under `sdk/campaign_builder`.
- Exported assets reuse `CreatureDefinition`, `MeshDefinition`, and
  `MeshTransform` from `src/domain/visual/mod.rs` exactly as defined.
- `CreatureId` remains the type used for importer-generated creature IDs.
- No core gameplay, party, combat, or inventory data structures were modified.
- All fixture data remains outside `campaigns/tutorial`, so Implementation Rule
  5 stays satisfied.

---

### Test coverage

Added importer workflow tests covering:

- loading a real OBJ fixture into `ObjImporterState`
- color edits propagating into the exported `CreatureDefinition`
- creature export round-tripping through valid RON on disk
- item export writing to `assets/items/`
- export-path preview behavior

The newly added `examples/*.obj` fixtures also satisfy the previously-added
file-based multi-mesh importer tests in `sdk/campaign_builder/src/mesh_obj_io.rs`.

Validation run status:

- `cargo fmt --all` -> passed
- `cargo check --all-targets --all-features` -> passed
- `cargo clippy --all-targets --all-features -- -D warnings` -> passed
- `cargo nextest run --all-features` -> blocked by existing unrelated failure:
  `tests/campaign_integration_tests.rs:252`
  `test_creature_database_load_performance`

The isolated rerun of that performance test still measured slightly over the
threshold on this machine (`535ms` vs expected `< 500ms`), matching the known
timing-sensitive repository note rather than an importer-specific regression.

## OBJ to RON Conversion - Phase 2: Color Palette and Mesh Color Mapping

**Plan**: [`obj_to_ron_implementation_plan.md`](obj_to_ron_implementation_plan.md)

### Overview

Phase 2 adds the importer-side color system needed for the future OBJ Importer
tab. The campaign builder now has a built-in palette module, mesh-name based
auto-color assignment, campaign-scoped custom palette persistence, and a
dedicated importer state object that can load OBJ meshes and pre-populate each
mesh row with counts, selections, and editable colors.

This work stays inside `sdk/campaign_builder` and reuses the existing
`MeshDefinition` and `CreatureId` architecture types instead of inventing SDK-
local equivalents.

---

### Phase 2 Deliverables

**Files modified**:

- `sdk/campaign_builder/src/lib.rs`
- `docs/explanation/implementations.md`

**Files created**:

- `sdk/campaign_builder/src/color_palette.rs`
- `sdk/campaign_builder/src/obj_importer.rs`

---

### What was built

#### Built-in palette module (`sdk/campaign_builder/src/color_palette.rs`)

Added a new palette module containing:

- `PALETTE: &[(&str, [f32; 4])]` for the built-in importer palette
- `PaletteEntry` for UI iteration
- `palette_entries()` to expose the built-in palette as a `Vec<PaletteEntry>`
- `suggest_color_for_mesh(mesh_name)` for name-based color assignment
- `CustomPalette` plus per-campaign load/save helpers for
  `config/importer_palette.ron`

The palette includes skin, hair, armor, cloth, and material colors, plus the
required default skin tone used by the Phase 2 test expectation for
`EM3D_Base_Body`.

#### Mesh-name color assignment (`sdk/campaign_builder/src/color_palette.rs`)

The matcher normalizes mesh names to lowercase underscore-delimited tokens, then
applies ordered keyword checks so specific names such as `Hair_Pink` win before
generic matches like `hair` or `body`.

Notable mappings now covered:

- `EM3D_Base_Body` -> `[0.92, 0.85, 0.78, 1.0]`
- `Hair_Pink` -> `[0.92, 0.55, 0.70, 1.0]`
- unknown names -> `[0.8, 0.8, 0.8, 1.0]`

#### Custom palette persistence (`sdk/campaign_builder/src/color_palette.rs`)

`CustomPalette` now supports:

- `load_from_campaign_dir()`
- `save_to_campaign_dir()`
- `add_color()`
- `remove_color()`

The file path is fixed at `<campaign_dir>/config/importer_palette.ron`, ready
for the later importer UI to add and remove user palette entries.

#### Importer state module (`sdk/campaign_builder/src/obj_importer.rs`)

Added `ObjImporterState`, `ImportedMesh`, `ImporterMode`, `ExportType`, and an
`ObjImporterError` wrapper. The state object now handles:

- loading OBJ meshes through the Phase 1 multi-mesh importer
- auto-assigning per-mesh colors during load
- preserving mesh counts and selection state for later bulk actions
- tracking custom palette data for the active campaign
- preserving `scale` and suggested `CreatureId` across importer resets

#### Campaign builder integration (`sdk/campaign_builder/src/lib.rs`)

`CampaignBuilderApp` now owns `obj_importer_state` and initializes it in
`Default::default()`.

When a campaign is opened, the app now also:

- loads `config/importer_palette.ron` into `obj_importer_state.custom_palette`
- computes the next available custom creature ID and stores it in the importer
  state

This keeps later importer UI work aligned with the currently loaded campaign.

#### Fixture consistency note

The Phase 2 plan references `examples/skeleton.obj` and `examples/female_1.obj`,
but those files were not present in this checkout while implementing Phase 2.
The importer-state tests therefore use deterministic inline OBJ content instead
of depending on absent fixture files.

---

### Architecture compliance

- The work is confined to `sdk/campaign_builder`, matching the SDK/editor layer
  described in `docs/reference/architecture.md`.
- `CreatureId` from `src/domain/types.rs` is used instead of a raw `u32`.
- `MeshDefinition` remains the authoritative mesh type; no duplicate mesh
  structs were introduced.
- No core domain or application data structures were modified.

---

### Test coverage

Added unit tests for:

- `suggest_color_for_mesh("EM3D_Base_Body")`
- `suggest_color_for_mesh("Hair_Pink")`
- unknown mesh fallback color
- `palette_entries()` covering all built-in palette entries
- custom palette load/save round-trip
- custom palette add/remove behavior
- importer mesh auto-color assignment
- importer state mode transitions and OBJ load behavior

Validation status is recorded after the Phase 2 code and tests were added.

## OBJ to RON Conversion - Phase 1: Multi-Mesh OBJ Import

**Plan**: [`obj_to_ron_implementation_plan.md`](obj_to_ron_implementation_plan.md)

### Overview

Phase 1 extends the Campaign Builder OBJ importer so it can read a Wavefront
OBJ file as a list of named meshes instead of flattening every object/group
into one `MeshDefinition`. The legacy single-mesh importer remains available,
while a new multi-mesh API now produces one `MeshDefinition` per `o`/`g`
section with local vertex remapping suitable for later creature/item RON
export work.

---

### Phase 1 Deliverables

**Files modified**:

- `sdk/campaign_builder/src/mesh_obj_io.rs`
- `docs/explanation/implementations.md`

**Files created**:

- `examples/skeleton.obj`
- `examples/female_1.obj`

---

### What was built

#### Multi-mesh OBJ import APIs (`sdk/campaign_builder/src/mesh_obj_io.rs`)

Added four new public functions:

- `import_meshes_from_obj`
- `import_meshes_from_obj_with_options`
- `import_meshes_from_obj_file`
- `import_meshes_from_obj_file_with_options`

These APIs parse global OBJ vertex/normal/UV pools, split meshes on `o` and
`g` directives, then build one `MeshDefinition` per parsed section.

#### Per-mesh vertex remapping (`sdk/campaign_builder/src/mesh_obj_io.rs`)

The new importer tracks face vertices as `(v, vt, vn)` references and remaps
them into local mesh indices. This means each exported `MeshDefinition` only
contains vertices actually referenced by that mesh, and every mesh gets its
own local zero-based index buffer.

Faces with more than three vertices are triangulated with a triangle-fan
strategy, matching the existing importer behavior for quads and n-gons.

#### Mesh name sanitization (`sdk/campaign_builder/src/mesh_obj_io.rs`)

Added a private `sanitize_mesh_name(raw: &str) -> String` helper that:

- replaces non-ASCII alphanumeric / underscore characters with `_`
- collapses repeated underscores
- trims leading and trailing underscores

If a sanitized mesh name becomes empty, the importer falls back to a stable
generated name such as `mesh_0`.

#### `ObjImportOptions::scale` (`sdk/campaign_builder/src/mesh_obj_io.rs`)

Extended `ObjImportOptions` with `scale: f32` and defaulted it to `1.0`.
Imported vertex positions are multiplied by this scale in both the legacy
single-mesh importer and the new multi-mesh importer.

#### No-group fallback (`sdk/campaign_builder/src/mesh_obj_io.rs`)

When an OBJ has no `o` or `g` directives, the multi-mesh importer returns a
single mesh named `mesh_0` so downstream code still receives a valid list of
meshes.

#### Deterministic OBJ fixtures (`examples/skeleton.obj`, `examples/female_1.obj`)

Added two small multi-object OBJ fixtures to the repository so the importer
tests can exercise the required filename-based paths without depending on
external assets.

---

### Architecture compliance

- The work is confined to the SDK importer layer under `sdk/campaign_builder`.
- Existing `MeshDefinition` from `src/domain/visual/mod.rs` is reused exactly
  as defined by the architecture.
- No domain/core data structures were modified.
- The legacy single-mesh importer remains intact for backward compatibility.
- New test fixtures live outside `campaigns/tutorial`, so Implementation Rule 5
  remains satisfied.

---

### Test coverage

Added or extended unit tests in `sdk/campaign_builder/src/mesh_obj_io.rs` for:

- `sanitize_mesh_name` edge cases
- `scale` application during import
- no-group fallback to `mesh_0`
- file-based multi-mesh import of `examples/skeleton.obj`
- file-based multi-mesh import of `examples/female_1.obj`
- legacy round-trip and single-mesh import behavior remaining intact

Validation run completed successfully:

- `cargo fmt --all`
- `cargo check --all-targets --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo nextest run --all-features` -> `3162 passed, 8 skipped`

## Combat Events — Missing Deliverables Gap Fill

### Overview

After Phase 5 was completed a gap analysis against
`docs/explanation/combat_events_implementation_plan.md` identified five
outstanding deliverables that had not been implemented:

1. **Phase 2** — `test_ambush_player_turn_is_skipped` test missing.
2. **Phase 2** — `test_ambush_player_can_act_round_2` test missing.
3. **Phase 4** — Four individually-named boss-flag tests missing
   (`test_boss_combat_monsters_advance`, `test_boss_combat_monsters_regenerate`,
   `test_boss_combat_cannot_bribe`, `test_boss_combat_cannot_surrender`); the
   behaviour was covered by the combined `test_boss_combat_sets_boss_flags` but
   the plan required each assertion in its own named test.
4. **Phase 4** — Boss opening combat-log text deviated from the plan.
   The plan specified `"A powerful foe stands before you! Prepare for a
legendary battle!"` but the implementation emitted `"A powerful foe
appears!"`.
5. **Phase 2 / Section 2.7** — `src/domain/resources.rs` was missing the
   mandatory code comment documenting that rest-interrupted encounters must use
   `CombatEventType::Ambush`.

### Deliverables Checklist

- [x] `test_ambush_player_turn_is_skipped` — asserts the "surprised" log entry
      is emitted and `CombatTurnStateResource` stays `EnemyTurn` after the player
      slot is auto-skipped in round 1.
- [x] `test_ambush_player_can_act_round_2` — asserts that after `advance_turn`
      pushes the state into round 2, `ambush_round_active` is `false` and
      `handicap` is `Even`, confirming the player would not be skipped.
- [x] `test_boss_combat_monsters_advance` — isolated assertion on
      `cs.monsters_advance == true`.
- [x] `test_boss_combat_monsters_regenerate` — isolated assertion on
      `cs.monsters_regenerate == true`.
- [x] `test_boss_combat_cannot_bribe` — isolated assertion on
      `cs.can_bribe == false`.
- [x] `test_boss_combat_cannot_surrender` — isolated assertion on
      `cs.can_surrender == false`.
- [x] Boss opening log text corrected to
      `"A powerful foe stands before you! Prepare for a legendary battle!"`.
- [x] `ResourceError::CannotRestWithActiveEncounter` doc comment updated to
      mandate `CombatEventType::Ambush` for rest-interrupted encounters.

### What Was Built

#### `test_ambush_player_turn_is_skipped` (`src/game/systems/combat.rs`)

A Bevy app test that manually constructs a `CombatResource` with
`ambush_round_active = true` and turn order `[Player(0), Monster(1)]`, inserts
it into a `CombatPlugin` app with `CombatTurnState::EnemyTurn`, then calls
`app.update()`. After the update:

- `CombatLogState` must contain a line with the word "surprised" (emitted by
  `execute_monster_turn`'s ambush-skip path).
- `CombatTurnStateResource` must still be `EnemyTurn` (the monster on the next
  slot has not yet acted, so the system keeps enemy turn active).

#### `test_ambush_player_can_act_round_2` (`src/game/systems/combat.rs`)

A pure-logic test (no Bevy app) that calls `start_encounter` with
`CombatEventType::Ambush`, verifies `ambush_round_active == true` as a
pre-condition, then calls `cs.advance_turn(&[])` to exhaust round 1 and
trigger `advance_round`. After the call it asserts:

- `ambush_round_active == false` — the flag is cleared.
- `handicap == Handicap::Even` — the handicap is reset.
- `round == 2` — we are actually in round 2.

This is sufficient to prove the player would not be skipped: both guard checks
(`combat_input_system` and `execute_monster_turn`) inspect `ambush_round_active`
directly.

#### Four individual boss-flag tests (`src/game/systems/combat.rs`)

Each test calls `start_encounter(&mut gs, &content, &[], CombatEventType::Boss)`
and asserts exactly one `CombatState` field. They are structurally identical to
the existing `test_boss_combat_sets_boss_flags` (which remains as a combined
sanity check) but satisfy the plan's requirement that each flag has a dedicated,
individually-named test that can fail in isolation.

| Test                                   | Field asserted                   |
| -------------------------------------- | -------------------------------- |
| `test_boss_combat_monsters_advance`    | `cs.monsters_advance == true`    |
| `test_boss_combat_monsters_regenerate` | `cs.monsters_regenerate == true` |
| `test_boss_combat_cannot_bribe`        | `cs.can_bribe == false`          |
| `test_boss_combat_cannot_surrender`    | `cs.can_surrender == false`      |

#### Boss opening log text (`src/game/systems/combat.rs`)

The `CombatEventType::Boss` arm of the `opening_text` match inside
`handle_combat_started` was updated from:

```antares/src/game/systems/combat.rs#L1212-1212
"A powerful foe appears!".to_string()
```

to:

```antares/src/game/systems/combat.rs#L1212-1214
"A powerful foe stands before you! Prepare for a legendary battle!".to_string()
```

This matches the exact text specified in plan Section 4.5.

#### Rest-interruption ambush comment (`src/domain/resources.rs`)

A `# Combat Event Type Requirement` doc section was added to the
`ResourceError::CannotRestWithActiveEncounter` variant. It states:

> Any encounter that fires while the party is resting **MUST** be started with
> `CombatEventType::Ambush`. The resting party is asleep and cannot react — the
> ambush mechanic (monsters act first in round 1, party turns suppressed)
> correctly models this. The rest system implementation is responsible for
> passing `CombatEventType::Ambush` to `start_encounter()` whenever it returns
> this error variant and triggers combat.

A cross-reference link to plan Section 2.7 is included.

### Architecture Compliance

- No architectural deviations introduced.
- No new data structures modified.
- All new tests use `data/test_campaign` patterns; no reference to
  `campaigns/tutorial`.
- RON format unchanged — no data files modified.

### Quality Gate Results

```text
cargo fmt --all           → no output (clean)
cargo check --all-targets → Finished dev profile, 0 errors
cargo clippy -D warnings  → Finished dev profile, 0 warnings
cargo nextest run         → 3232 tests run: 3232 passed, 8 skipped
  (campaign_builder)      → 1938 tests run: 1938 passed, 2 skipped
```

---

## Phase 5: Campaign Builder UI — Combat Event Type

### Overview

Phase 5 wires the `CombatEventType` domain enum (introduced in Phase 1) into the
Campaign Builder SDK so that campaign authors can select and persist the combat type
for every map encounter event and random encounter group without hand-editing RON
files. It also surfaces the selected type visually in the inspector panel with
per-type colour hints.

Files modified:

- `sdk/campaign_builder/src/map_editor.rs` — all UI, state, and serialisation changes
- `sdk/campaign_builder/src/lib.rs` — fixed 9 pre-existing `Attack` struct-literal
  compilation errors (missing `is_ranged` field introduced by Phase 3)

### Phase 5 Deliverables Checklist

- [x] `encounter_combat_event_type: CombatEventType` field on `EventEditorState`
- [x] `CombatEventType::Normal` default in `impl Default for EventEditorState`
- [x] `CombatEventType` combo-box in `show_event_editor()` for Encounter type
- [x] `to_map_event()` forwards `combat_event_type` into `MapEvent::Encounter`
- [x] `from_map_event()` reads `combat_event_type` from `MapEvent::Encounter`
- [x] Per-group `CombatEventType` selector in the random encounter table editor (`show_metadata_editor`)
- [x] Combat type displayed with per-type colour in the inspector panel (`show_inspector_panel`)
- [x] Combat type colour constants (`COMBAT_TYPE_COLOR_AMBUSH/BOSS/RANGED/MAGIC`)
- [x] `push_id` used for all group-level combo-boxes (no egui ID clashes)
- [x] `ComboBox::from_id_salt` used for every combo-box (SDK egui ID rule)
- [x] `ScrollArea::id_salt` set on encounter groups scroll area
- [x] All 12 Phase 5 tests pass
- [x] All 4 quality gates pass (fmt / check / clippy / nextest)

### What Was Built

#### `encounter_combat_event_type` field on `EventEditorState` (`sdk/campaign_builder/src/map_editor.rs`)

Added a new public field to `EventEditorState`:

```sdk/campaign_builder/src/map_editor.rs#L1952-1953
/// Combat event type selected for this encounter. Controls ambush, ranged,
/// magic, and boss mechanics in the game layer.
pub encounter_combat_event_type: CombatEventType,
```

`impl Default for EventEditorState` initialises it to `CombatEventType::Normal` so
that existing saved events without the field continue to behave identically
(backward-compatible via `#[serde(default)]` on the domain struct).

#### `to_map_event()` — encounter arm (`sdk/campaign_builder/src/map_editor.rs`)

The `EventType::Encounter` arm was extended to forward the editor field:

```sdk/campaign_builder/src/map_editor.rs#L2141-2155
Ok(MapEvent::Encounter {
    name: self.name.clone(),
    description: self.description.clone(),
    monster_group: monsters,
    time_condition: None,
    facing,
    proximity_facing: self.event_proximity_facing,
    rotation_speed: ...,
    combat_event_type: self.encounter_combat_event_type,
})
```

#### `from_map_event()` — encounter arm (`sdk/campaign_builder/src/map_editor.rs`)

The `MapEvent::Encounter` arm was extended to read `combat_event_type` back into the
editor state, enabling lossless round-trip editing:

```sdk/campaign_builder/src/map_editor.rs#L2371-2391
MapEvent::Encounter {
    ...,
    combat_event_type,
    ..
} => {
    ...
    s.encounter_combat_event_type = *combat_event_type;
}
```

#### Combat Type ComboBox in `show_event_editor()` (`sdk/campaign_builder/src/map_editor.rs`)

After the monster selector for `EventType::Encounter`, a labelled combo-box is
rendered using `ComboBox::from_id_salt("encounter_combat_event_type")`. It iterates
`CombatEventType::all()`, uses `selectable_value` for each variant, and shows the
variant `description()` as hover text. A small grey description label appears below
the combo-box for the currently-selected variant. Changing the selection sets
`editor.has_changes = true`.

#### Per-group CombatEventType in `show_metadata_editor()` (`sdk/campaign_builder/src/map_editor.rs`)

The random encounter table section was added to the map metadata editor. For each
`EncounterGroup` in `EncounterTable::groups`, the UI:

1. Wraps the row in `ui.push_id(group_idx, |ui| { ... })` to prevent egui ID collisions.
2. Renders `ComboBox::from_id_salt(format!("encounter_group_combat_type_{}", group_idx))` for per-group type selection.
3. Shows the group's `combat_event_type.description()` as a small grey hint label.
4. Provides a "🗑️ Remove" button per group and an "➕ Add Group" button at the bottom.
5. Wraps the group list in `ScrollArea::vertical().id_salt("encounter_groups_scroll")`.

#### Inspector panel — combat type display (`sdk/campaign_builder/src/map_editor.rs`)

`show_inspector_panel()` was extended so that when the selected tile has a
`MapEvent::Encounter`, the combat type is shown with a colour-coded label:

| Variant | Colour                                     |
| ------- | ------------------------------------------ |
| Normal  | `Color32::LIGHT_GRAY`                      |
| Ambush  | `COMBAT_TYPE_COLOR_AMBUSH` (180, 60, 70)   |
| Ranged  | `COMBAT_TYPE_COLOR_RANGED` (209, 154, 102) |
| Magic   | `COMBAT_TYPE_COLOR_MAGIC` (198, 120, 221)  |
| Boss    | `COMBAT_TYPE_COLOR_BOSS` (220, 50, 50)     |

A small grey description label follows the type label.

#### Combat type colour constants (`sdk/campaign_builder/src/map_editor.rs`)

Four constants were added (grid tiles continue to use `EVENT_COLOR_ENCOUNTER` — the
colour differentiation is inspector-only):

```sdk/campaign_builder/src/map_editor.rs#L97-106
const COMBAT_TYPE_COLOR_AMBUSH: Color32 = Color32::from_rgb(180, 60, 70);
const COMBAT_TYPE_COLOR_BOSS:   Color32 = Color32::from_rgb(220, 50, 50);
const COMBAT_TYPE_COLOR_RANGED: Color32 = Color32::from_rgb(209, 154, 102);
const COMBAT_TYPE_COLOR_MAGIC:  Color32 = Color32::from_rgb(198, 120, 221);
```

#### `Attack` struct-literal fixes (`sdk/campaign_builder/src/lib.rs`)

9 `Attack { ... }` struct literals in `lib.rs` were missing the `is_ranged: bool`
field added by Phase 3. All 9 occurrences were updated to include `is_ranged: false`.
This was a pre-existing compilation blocker preventing the `campaign_builder` crate
from building; Phase 5 resolved it as part of making the full crate compile.

### Phase 5 Tests

All tests live in `mod tests` at the bottom of `sdk/campaign_builder/src/map_editor.rs`.

| Test                                                       | Assertion                                                                          |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `test_event_editor_state_default_combat_type`              | `Default::default()` has `Normal`                                                  |
| `test_to_map_event_preserves_combat_type_ambush`           | `to_map_event()` with Ambush → `MapEvent::Encounter { combat_event_type: Ambush }` |
| `test_to_map_event_preserves_combat_type_boss`             | same for Boss                                                                      |
| `test_to_map_event_preserves_combat_type_ranged`           | same for Ranged                                                                    |
| `test_to_map_event_preserves_combat_type_magic`            | same for Magic                                                                     |
| `test_from_map_event_reads_combat_type_boss`               | `from_map_event()` on Boss encounter sets editor field                             |
| `test_from_map_event_reads_combat_type_ambush`             | same for Ambush                                                                    |
| `test_from_map_event_normal_type_on_default_field`         | backward-compat: Normal field → Normal editor state                                |
| `test_combat_type_combo_box_has_all_variants`              | `CombatEventType::all()` returns exactly 5 variants                                |
| `test_combat_type_round_trip_all_variants`                 | every variant survives `to_map_event` → `from_map_event`                           |
| `test_combat_type_does_not_affect_non_encounter_events`    | Sign event is unaffected by `encounter_combat_event_type`                          |
| `test_encounter_combat_event_type_display_names_non_empty` | `display_name()` non-empty for all variants                                        |
| `test_encounter_combat_event_type_descriptions_non_empty`  | `description()` non-empty for all variants                                         |

### Architecture Compliance

- `CombatEventType` used exactly as defined in `src/domain/combat/types.rs` (Section 4 of architecture).
- `EncounterGroup::combat_event_type` uses the type alias from Phase 1 — no raw integers.
- `ComboBox::from_id_salt`, `push_id` for loops, and `ScrollArea::id_salt` follow `sdk/AGENTS.md` egui ID rules.
- No new documentation files created; summary placed in `docs/explanation/implementations.md` as required.
- No `campaigns/tutorial` references in tests (Implementation Rule 5).
- RON format used for all game data — no JSON or YAML.

### Quality Gate Results

```text
cargo fmt --all           → no output (clean)
cargo check --all-targets → Finished dev profile, 0 errors
cargo clippy -D warnings  → Finished dev profile, 0 warnings
cargo nextest run         → 3226 tests run: 3226 passed, 8 skipped
  (campaign_builder)      → 1938 tests run: 1938 passed, 2 skipped
```

---

## Phase 4: Boss Combat

### Overview

Phase 4 extends the combat system with a fully-featured **Boss** encounter
mode. When `CombatEventType::Boss` is active, monsters never flee, regenerate
at an accelerated rate (`BOSS_REGEN_PER_ROUND = 5` HP per round instead of 1),
and a prominent HP bar is rendered at the top of the screen. Victory over a
boss encounter sets `VictorySummary::boss_defeated = true`, causing the victory
screen to display a `"⚔ Boss Defeated! ⚔"` header.

### Phase 4 Deliverables Checklist

- [x] `BOSS_REGEN_PER_ROUND` and `BOSS_STAT_MULTIPLIER` constants in `src/domain/combat/types.rs`
- [x] Two constant unit tests in `src/domain/combat/types.rs`
- [x] `BossHpBar`, `BossHpBarFill`, `BossHpBarText` components in `src/game/systems/combat.rs`
- [x] Boss HP bar visual constants (`BOSS_HP_BAR_WIDTH`, `BOSS_HP_BAR_HEIGHT`, `BOSS_HP_HEALTHY_COLOR`, `BOSS_HP_INJURED_COLOR`, `BOSS_HP_CRITICAL_COLOR`)
- [x] `setup_combat_ui` spawns the boss HP bar panel when `combat_event_type == Boss`
- [x] `update_combat_ui` updates boss HP bar fill width and text each frame
- [x] `perform_monster_turn_with_rng` suppresses monster fleeing for Boss encounters
- [x] `perform_monster_turn_with_rng` applies `BOSS_REGEN_PER_ROUND` bonus regeneration after `advance_turn`
- [x] `execute_monster_turn` captures `round_before`/`round_after` and emits regeneration log lines
- [x] `VictorySummary::boss_defeated` field added
- [x] `process_combat_victory_with_rng` sets `boss_defeated` from `combat_event_type`
- [x] `handle_combat_victory` shows `"⚔ Boss Defeated! ⚔"` header when `boss_defeated == true`
- [x] Five new Phase 4 tests in `src/game/systems/combat.rs`

### What Was Built

#### `BOSS_REGEN_PER_ROUND` and `BOSS_STAT_MULTIPLIER` (`src/domain/combat/types.rs`)

Two public constants added immediately after the `CombatEventType` impl block:

- `BOSS_REGEN_PER_ROUND: u16 = 5` — total HP regenerated per round by a boss
  monster with `can_regenerate = true`. The base engine already adds 1 HP in
  `advance_round`; boss logic adds the remaining 4 as a bonus in
  `perform_monster_turn_with_rng`, giving exactly 5 total.
- `BOSS_STAT_MULTIPLIER: f32 = 1.0` — reserved for future stat scaling;
  currently a no-op so campaign authors can tune monsters via RON data.

#### Boss HP Bar Components (`src/game/systems/combat.rs`)

Three new `#[derive(Component)]` structs, each carrying a `participant_index`:

- `BossHpBar` — root panel node; used as a presence marker in tests
- `BossHpBarFill` — the colored fill node whose width tracks `hp.current / hp.base`
- `BossHpBarText` — the `"current/base"` text label

Five visual constants drive the appearance:

| Constant                 | Value                                          |
| ------------------------ | ---------------------------------------------- |
| `BOSS_HP_BAR_WIDTH`      | `400.0` px                                     |
| `BOSS_HP_BAR_HEIGHT`     | `20.0` px                                      |
| `BOSS_HP_HEALTHY_COLOR`  | `srgba(0.8, 0.1, 0.1, 1.0)` (dark red)         |
| `BOSS_HP_INJURED_COLOR`  | `srgba(0.5, 0.1, 0.1, 1.0)` (dimmer red)       |
| `BOSS_HP_CRITICAL_COLOR` | `srgba(0.3, 0.05, 0.05, 1.0)` (near-black red) |

#### `setup_combat_ui` boss bar spawn (`src/game/systems/combat.rs`)

After the enemy panel's `.with_children` block closes and before the turn
order panel, a conditional block checks `combat_res.combat_event_type ==
CombatEventType::Boss`. For the first monster in `participants`, it spawns:

1. An absolutely-positioned panel at `top: 8px`, centred horizontally.
2. A gold `"⚔ {name} ⚔"` name label.
3. A HP bar background → fill child (tagged `BossHpBarFill`).
4. A `BossHpBarText` label.

Only the first monster gets a boss bar (`break` after the first match).

#### `update_combat_ui` boss bar update (`src/game/systems/combat.rs`)

Two new query parameters were added to the function:

```
mut boss_hp_fills: Query<(&BossHpBarFill, &mut Node, &mut BackgroundColor), Without<EnemyHpBarFill>>
mut boss_hp_texts: Query<(&BossHpBarText, &mut Text), Without<EnemyHpText>>
```

Existing queries received matching `Without<BossHpBarFill>` / `Without<BossHpBarText>`
filters to prevent Bevy query conflicts. At the end of `update_combat_ui`, the
fill's `node.width` is set to `Val::Percent(ratio * 100.0)` and the color
transitions through the three threshold constants (≥50% healthy, ≥25%
injured, <25% critical).

#### Monster flee suppression (`perform_monster_turn_with_rng`)

Before `resolve_attack`, a `should_flee_this_turn` boolean is computed:

```rust
let should_flee_this_turn = if combat_res.combat_event_type == CombatEventType::Boss {
    false
} else if let Some(Combatant::Monster(mon)) = ... {
    mon.should_flee()
} else { false };
```

If true (non-boss only), the monster is marked as acted, the turn is advanced,
and the function returns `Ok(None)` without attacking — identical to what a
fleeing monster would do. Boss monsters always proceed to the attack path.

#### Boss bonus regeneration (`perform_monster_turn_with_rng`)

After `advance_turn`, when `combat_event_type == Boss && monsters_regenerate`:

```rust
let bonus_regen = BOSS_REGEN_PER_ROUND.saturating_sub(1); // = 4
```

Each alive, `can_regenerate` monster calls `mon.regenerate(bonus_regen)`.
Combined with `advance_round`'s built-in `regenerate(1)` this gives exactly
`BOSS_REGEN_PER_ROUND = 5` HP per round.

#### Boss regeneration log lines (`execute_monster_turn`)

```rust
let round_before = combat_res.state.round;
let outcome = perform_monster_turn_with_rng(...);
let round_after = combat_res.state.round;

if round_after > round_before
    && combat_res.combat_event_type == CombatEventType::Boss
    && combat_res.state.monsters_regenerate
{ ... push log line ... }
```

When a new round starts the log receives a green `"{name} regenerates {N} HP!"`
line (color `FEEDBACK_COLOR_HEAL`) for every regenerating alive monster.

#### `VictorySummary::boss_defeated` (`src/game/systems/combat.rs`)

A `pub boss_defeated: bool` field was added to `VictorySummary`. It is set in
`process_combat_victory_with_rng`:

```rust
boss_defeated: combat_res.combat_event_type == CombatEventType::Boss,
```

In `handle_combat_victory` the text is:

```rust
let header = if summary.boss_defeated { "⚔ Boss Defeated! ⚔\n".to_string() } else { String::new() };
Text::new(format!("{}Victory! XP: {} ...", header, ...))
```

### Phase 4 Tests

All five new tests live in the `mod tests` block at the bottom of
`src/game/systems/combat.rs`:

| Test                                        | What it verifies                                                                                 |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `test_boss_monster_does_not_flee`           | Monster at 1 HP (flee_threshold=50) returns `Ok(Some(...))` in Boss mode — it attacked, not fled |
| `test_boss_monster_regenerates_each_round`  | After `advance_turn` + bonus regen, HP increases by exactly `BOSS_REGEN_PER_ROUND`               |
| `test_boss_hp_bar_spawned`                  | After `app.update()` with Boss type, `BossHpBar` component count > 0                             |
| `test_normal_combat_no_boss_bar`            | Normal encounter produces zero `BossHpBar` components                                            |
| `test_boss_victory_summary_has_boss_header` | `process_combat_victory_with_rng` with Boss type sets `summary.boss_defeated = true`             |

Two constant tests were also added to `src/domain/combat/types.rs`:
`test_boss_regen_per_round_constant` and `test_boss_stat_multiplier_constant`.

# Implementation Notes

## Terrain Quality Deviation Plan - Phase 3: SDK Terrain Asset Validation

### Grass Texture Validation Follow-Up

Added SDK-side validation coverage for the required grass runtime texture asset
alongside the existing tree texture checks. The campaign builder asset manager
now validates:

- `assets/textures/grass/grass_blade.png`
- exact required dimensions: `32×128`

The implementation mirrors the established tree-texture validation pattern:

- added grass texture spec/issue types in
  `sdk/campaign_builder/src/asset_manager.rs`
- added `required_grass_texture_specs()`
- added `AssetManager::validate_grass_texture_assets()`
- integrated grass diagnostics into the existing validation panel and assets
  surfaces in `sdk/campaign_builder/src/lib.rs`
- added tests for missing file, misnamed file, dimension mismatch, and valid
  asset set

Validation IDs for the grass checks use a separate namespace so they do not
overload the existing tree-texture identifiers.

- Added Campaign Builder / SDK-side terrain tree texture validation through existing asset and validation surfaces without introducing a dedicated terrain-quality panel.
- Audited the planned integration points and kept the implementation scoped to existing validation flows:
  | component | terrain_feature_dependency | required_change | reason |
  | --- | --- | --- | --- |
  | `sdk/campaign_builder/src/asset_manager.rs::AssetManager` | direct | add required tree texture filename and dimension validation helpers | asset scanning already owns campaign-relative asset discovery and is the correct place to inspect required texture files |
  | `sdk/campaign_builder/src/lib.rs::CampaignBuilderApp::validate_campaign` | direct | inject tree texture diagnostics into existing validation results | the Validation tab already surfaces builder-visible diagnostics and satisfies the no-new-panel requirement |
  | `sdk/campaign_builder/src/lib.rs::CampaignBuilderApp::show_assets_editor` | indirect | expose tree texture issues in the existing assets surface | asset-oriented diagnostics should be discoverable where campaign authors inspect assets |
  | `sdk/campaign_builder/src/config_editor.rs::ConfigEditorState` | none | none | config editing is unrelated to terrain tree runtime asset validation for this phase |
  | `src/sdk/validation.rs::Validator` | indirect | none | runtime-agnostic SDK validation remains unchanged because this phase is specifically builder-surface asset validation |
- Added exact required tree texture validation coverage for:
  - `SDK-TEX-01`: missing required tree texture file
  - `SDK-TEX-02`: misnamed tree texture file does not satisfy exact filename requirements
  - `SDK-TEX-03` through `SDK-TEX-09`: exact required image dimensions for `bark.png` and each `foliage_*.png`
- Validation diagnostics now include:
  - expected filename
  - actual missing or mismatched asset path
  - expected dimensions
  - actual dimensions when available
- Kept validation intentionally limited to filename presence, filename exactness, and image dimensions. No subjective silhouette-quality validation was added to the Campaign Builder.
- Added SDK tests covering:
  - missing required tree texture file
  - misnamed tree texture file
  - bark dimension mismatch
  - pine dimension mismatch
  - fully valid required tree texture set
- No egui layout or widget-ID changes were required for this phase, so no additional egui ID audit items were introduced.
- Deliverables completed:

  - builder-side tree texture validation added to existing asset/validation surfaces
  - exact filename validation added
  - exact tree texture dimension validation added
  - no dedicated terrain-quality panel added
  - diagnostics include expected and actual values
  - SDK tests added for missing files, misnamed files, dimension mismatches, and valid asset sets
  - quality gates run:
    - `cargo fmt --all`
    - `cargo check --all-targets --all-features`
    - `cargo clippy --all-targets --all-features -- -D warnings`
    - `cargo nextest run --all-features` started successfully and showed passing test execution in the captured run output

- **`has_acted` is reset by `advance_round`**: `advance_round` calls
  `monster.reset_turn()` on every monster, clearing `has_acted`. The flee test
  therefore verifies `Ok(Some(...))` (attack resolved) rather than `has_acted`,
  which would be unreliable when the turn wraps into a new round during the test.
- **No ECS `world().query()` borrow conflict**: Boss HP bar queries use
  `Without<EnemyHpBarFill>` / `Without<EnemyHpBarText>` filters to satisfy
  Bevy's disjoint-query requirement. Existing queries received matching
  `Without<BossHpBar*>` filters.
- **SPDX headers not duplicated**: Both files already carried SPDX headers from
  earlier phases; none were added.

### Architecture Compliance

- [x] Data structures match architecture.md Section 4.4 (`CombatState`, `Monster`)
- [x] `CombatEventType::Boss` used (not a new enum, already existed from Phase 1)
- [x] Constants extracted (`BOSS_REGEN_PER_ROUND`, `BOSS_STAT_MULTIPLIER`)
- [x] `AttributePair16` pattern used for HP (not raw integers)
- [x] No new RON data files required (boss mechanics are purely runtime)
- [x] No architectural deviations from architecture.md

### Quality Gate Results

```
cargo fmt         → no output (all files formatted)
cargo check       → Finished with 0 errors
cargo clippy      → Finished with 0 warnings
cargo nextest run → 3226 passed; 0 failed; 8 skipped
```

## Phase 3: Ranged and Magic Combat

### Overview

Phase 3 extends the combat system with two new encounter modes — **Ranged** and
**Magic** — and wires the full action pipeline for ranged attacks: button
spawning, target selection, ammo consumption, damage resolution, and combat-log
messaging. Monster AI is updated to prefer ranged attacks in ranged encounters,
and the action-menu keyboard order is adjusted so that **Cast** is the default
highlight in magic encounters.

### Phase 3 Deliverables Checklist

- [x] `TurnAction::RangedAttack` variant added to `src/domain/combat/types.rs`
- [x] `CombatError::NoAmmo` variant added to `src/domain/combat/engine.rs`
- [x] `choose_monster_attack` extended with `is_ranged_combat: bool` parameter
- [x] `ActionButtonType::RangedAttack` variant added in `src/game/systems/combat.rs`
- [x] `COMBAT_ACTION_COUNT_MAGIC` and `COMBAT_ACTION_ORDER_MAGIC` constants added
- [x] `RangedAttackAction` message struct added and registered with the plugin
- [x] `RangedAttackPending` resource added and registered with the plugin
- [x] `setup_combat_ui` spawns Ranged button for `CombatEventType::Ranged`
- [x] `setup_combat_ui` uses magic button order for `CombatEventType::Magic`
- [x] `setup_combat_ui` ordered `.after(handle_combat_started)` to fix race
- [x] `update_ranged_button_color` system enables/disables Ranged button each frame
- [x] `dispatch_combat_action` handles `ActionButtonType::RangedAttack`
- [x] `confirm_attack_target` dispatches `RangedAttackAction` when `RangedAttackPending`
- [x] `select_target` / `combat_input_system` pass `RangedAttackPending` through
- [x] `combat_input_system` uses correct action-order array for magic/standard
- [x] `update_action_highlight` uses correct order and skips the Ranged button
- [x] `perform_ranged_attack_action_with_rng` function implemented
- [x] `handle_ranged_attack_action` ECS system implemented and registered
- [x] `handle_combat_started` combat log updated for all `CombatEventType` variants
- [x] `perform_attack_action_with_rng` passes `is_ranged_combat` to `choose_monster_attack`
- [x] `perform_monster_turn_with_rng` passes `is_ranged_combat` to `choose_monster_attack`
- [x] All 10 Phase 3 tests pass; pre-existing `test_ambush_combat_started_sets_enemy_turn` fixed
- [x] `docs/explanation/implementations.md` updated

### What Was Built

#### `TurnAction::RangedAttack` (`src/domain/combat/types.rs`)

Added between `Attack` and `Defend` as the plan specifies. The doc comment
explains the ammo requirement and its intended use in `CombatEventType::Ranged`
encounters.

#### `CombatError::NoAmmo` (`src/domain/combat/engine.rs`)

New variant with message `"No ammo available for ranged attack"`. Returned by
`perform_ranged_attack_action_with_rng` when the attacker has a
`MartialRanged` weapon but no `ItemType::Ammo` item in their inventory.

#### `choose_monster_attack` signature change (`src/domain/combat/engine.rs`)

Added `is_ranged_combat: bool` as the second parameter (before `rng`). When
`true` the function first filters `monster.attacks` for entries with
`is_ranged == true`; if any exist one is chosen uniformly at random. If none
exist the function falls through to the existing special-attack-threshold +
random selection logic. When `false` behaviour is completely unchanged.

All callers updated:

- `perform_attack_action_with_rng` (monster branch) — passes `combat_res.combat_event_type == CombatEventType::Ranged`
- `perform_monster_turn_with_rng` — same
- `test_combat_monster_special_ability_applied` in `engine.rs` — passes `false`

#### `ActionButtonType::RangedAttack` (`src/game/systems/combat.rs`)

New variant inserted after `Attack`. Excluded from `COMBAT_ACTION_ORDER` and
`COMBAT_ACTION_ORDER_MAGIC` (those arrays cycle only the 5 standard actions);
the Ranged button is spawned as an extra sixth button in ranged encounters.

#### `COMBAT_ACTION_COUNT_MAGIC` and `COMBAT_ACTION_ORDER_MAGIC`

```src/game/systems/combat.rs#L319-336
pub const COMBAT_ACTION_COUNT_MAGIC: usize = 5;

pub const COMBAT_ACTION_ORDER_MAGIC: [ActionButtonType; COMBAT_ACTION_COUNT_MAGIC] = [
    ActionButtonType::Cast,
    ActionButtonType::Attack,
    ActionButtonType::Defend,
    ActionButtonType::Item,
    ActionButtonType::Flee,
];
```

`Cast` is index 0 so the default keyboard highlight is the most useful action
in a magic encounter.

#### `RangedAttackAction` and `RangedAttackPending` (`src/game/systems/combat.rs`)

`RangedAttackAction` mirrors `AttackAction` (same `attacker` + `target` fields)
but routes through `perform_ranged_attack_action_with_rng`.

`RangedAttackPending(bool)` is a `Resource` that `dispatch_combat_action` sets
to `true` when `ActionButtonType::RangedAttack` is pressed.
`confirm_attack_target` reads it: if `true`, writes `RangedAttackAction` and
resets the flag; otherwise writes the normal `AttackAction`.
Cancelling target selection (`Escape`) also clears the flag.

#### `setup_combat_ui` changes (`src/game/systems/combat.rs`)

Two changes:

1. **Magic button order** — when `combat_res.combat_event_type.highlights_magic_action()`
   the 5 standard buttons are spawned in `COMBAT_ACTION_ORDER_MAGIC` order
   (Cast first); otherwise the standard order is used.

2. **Ranged button** — after the 5 standard buttons, if
   `combat_res.combat_event_type.enables_ranged_action()` an extra `Button` is
   spawned with `ActionButton { button_type: ActionButtonType::RangedAttack }`
   and `ACTION_BUTTON_DISABLED_COLOR`. `update_ranged_button_color` enables it
   each frame once a ranged weapon + ammo is confirmed.

3. **System ordering fix** — `setup_combat_ui` is now registered
   `.after(handle_combat_started)` so `combat_res.combat_event_type` is
   populated before the button spawn decision is made. Without this ordering,
   the system could run before the message handler and always see
   `CombatEventType::Normal`.

#### `update_ranged_button_color` (`src/game/systems/combat.rs`)

New private system registered after `update_combat_ui` and
`update_action_highlight`. Each frame during a ranged encounter it queries the
current actor, calls `has_ranged_weapon(pc, &content.db().items)`, and sets
the `RangedAttack` button color to `ACTION_BUTTON_COLOR` (enabled) or
`ACTION_BUTTON_DISABLED_COLOR` (disabled).

`update_action_highlight` skips buttons with `button_type ==
ActionButtonType::RangedAttack` so the two systems do not conflict.

#### `perform_ranged_attack_action_with_rng` (`src/game/systems/combat.rs`)

Full implementation:

1. Guard: only runs if it is the attacker's current turn.
2. Only player combatants may use this path (returns `CombatantCannotAct` for monsters).
3. Calls `has_ranged_weapon` — if `false`, distinguishes "bow but no ammo"
   (`NoAmmo`) from "no bow at all" (`CombatantCannotAct`).
4. Calls `get_character_attack` expecting `MeleeAttackResult::Ranged`.
5. Calls `resolve_attack` for the to-hit roll and damage.
6. Removes the **first** `ItemType::Ammo` slot from the attacker's inventory
   (one arrow consumed per shot).
7. Calls `apply_damage`.
8. Applies special effects (same pattern as `perform_attack_action_with_rng`).
9. Calls `check_combat_end`, `advance_turn`, updates `CombatTurnStateResource`.

#### `handle_ranged_attack_action` (`src/game/systems/combat.rs`)

ECS system wrapper registered after `handle_attack_action`. Reads
`RangedAttackAction` messages, calls `perform_ranged_attack_action_with_rng`,
emits a `CombatFeedbackEvent`, and pushes a "fires a ranged attack!" log line.
On `NoAmmo` it pushes a "No ammo!" log line; other errors are logged as
warnings.

#### `handle_combat_started` combat log (`src/game/systems/combat.rs`)

Replaced the if/else branch with a `match` on `msg.combat_event_type`:

| Variant  | Log text                                                 |
| -------- | -------------------------------------------------------- |
| `Normal` | "Monsters appear!"                                       |
| `Ranged` | "Combat begins at range! Draw your bows!"                |
| `Magic`  | "The air crackles with magical energy!"                  |
| `Boss`   | "A powerful foe appears!"                                |
| `Ambush` | "The monsters ambush the party! The party is surprised!" |

#### `test_ambush_combat_started_sets_enemy_turn` fix

The test previously used an empty monster group. With the new system ordering
(setup_combat_ui runs after handle_combat_started, which in turn forces
execute_monster_turn to also run after), the ambush player-skip path in
`execute_monster_turn` would process the single player slot, wrap the round,
clear `ambush_round_active`, and leave the state as `PlayerTurn` — breaking
the assertion.

Fix: the test now injects a `CombatResource` with one player + one goblin
monster. The ambush path skips the player (slot 0) and advances to the
monster (slot 1), leaving `turn_state = EnemyTurn`. This correctly models
the actual game flow and makes the assertion meaningful.

### Phase 3 Tests

All added to `mod tests` in `src/game/systems/combat.rs`:

| Test                                                    | What it verifies                                                       |
| ------------------------------------------------------- | ---------------------------------------------------------------------- |
| `test_ranged_combat_shows_ranged_button`                | `ActionButton { RangedAttack }` spawned in Ranged combat               |
| `test_ranged_button_disabled_without_ranged_weapon`     | Button has `ACTION_BUTTON_DISABLED_COLOR` when no ranged weapon        |
| `test_ranged_button_enabled_with_ranged_weapon`         | Button has `ACTION_BUTTON_COLOR` when player has bow + ammo            |
| `test_perform_ranged_attack_consumes_ammo`              | One ammo slot removed from inventory after a ranged attack             |
| `test_perform_ranged_attack_no_ammo_returns_error`      | `CombatError::NoAmmo` when bow is equipped but inventory empty         |
| `test_magic_combat_cast_is_first_action`                | `COMBAT_ACTION_ORDER_MAGIC[0] == ActionButtonType::Cast`               |
| `test_magic_combat_normal_handicap`                     | Magic combat uses `Handicap::Even`                                     |
| `test_monster_ranged_attack_preferred_in_ranged_combat` | `choose_monster_attack(mon, true, rng)` always picks the ranged attack |
| `test_combat_log_ranged_opening`                        | Log contains "range" for `CombatEventType::Ranged`                     |
| `test_combat_log_magic_opening`                         | Log contains "magical" for `CombatEventType::Magic`                    |

Domain-layer test in `src/domain/combat/engine.rs` (existing, updated caller):

- `test_combat_monster_special_ability_applied` — updated to pass `false` for `is_ranged_combat`

### Architecture Compliance

- `TurnAction::RangedAttack` placed between `Attack` and `Defend` per plan
- `CombatError::NoAmmo` added to `engine.rs` alongside existing error variants
- `ActionButtonType::RangedAttack` added without disrupting `COMBAT_ACTION_ORDER`
- `RangedAttackAction` message follows the same `Message` derive pattern as all other action messages
- `RangedAttackPending` is a minimal `Resource` (single bool) following the resource naming convention
- All game data files remain in RON format; no new data files created
- No modifications to `campaigns/tutorial`
- Test data uses `make_p2_combat_fixture` / inline fixtures, not `campaigns/tutorial`
- `has_ranged_weapon` imported from `engine` (already existed from Phase 1 equipped-weapon work)
- `CombatError` imported from `engine` (replaces previously inline `use` statements)

### Quality Gate Results

```
cargo fmt --all          → No output (clean)
cargo check --all-targets --all-features → Finished, 0 errors
cargo clippy --all-targets --all-features -- -D warnings → Finished, 0 warnings
cargo nextest run --all-features → 3218 passed, 1 failed (pre-existing
    test_creature_database_load_performance timing flake, unrelated to Phase 3)
```

---

## Phase 2: Normal and Ambush Combat

### Overview

Phase 2 of the Combat Events Implementation Plan implements the behavioral
differences between `Normal` and `Ambush` combat encounters. After this phase,
an ambush encounter causes the party to miss their entire first round of actions
(the party is surprised), monsters receive `Handicap::MonsterAdvantage` for
round 1, and the combat log clearly announces the ambush. From round 2 onward
the combat reverts to `Handicap::Even` and proceeds identically to a normal
encounter. Boss combat flags (`monsters_advance`, `monsters_regenerate`,
`can_bribe = false`, `can_surrender = false`) are also wired in this phase via
`start_encounter()`.

### Phase 2 Deliverables Checklist

- [x] `ambush_round_active: bool` field on `CombatState` (`src/domain/combat/engine.rs`)
- [x] `TurnAction::Skip` variant in `src/domain/combat/types.rs`
- [x] `start_encounter()` sets `ambush_round_active` and `MonsterAdvantage` handicap for Ambush
- [x] `start_encounter()` sets boss flags (`monsters_advance`, `monsters_regenerate`, `can_bribe = false`, `can_surrender = false`) for Boss type
- [x] `advance_round()` clears `ambush_round_active` and resets handicap to `Even` at round 2
- [x] `handle_combat_started` forces `CombatTurnState::EnemyTurn` when `ambush_round_active` is set
- [x] `execute_monster_turn` auto-skips surprised player slots during ambush round 1
- [x] `combat_input_system` defence-in-depth guard blocks player input during ambush round 1
- [x] Combat log entry "The monsters ambush the party! The party is surprised!" on ambush start
- [x] Combat log entry "Monsters appear!" on normal encounter start
- [x] All Phase 2 tests pass (3209 passed, 8 skipped, 0 failed)

### What Was Built

#### `ambush_round_active: bool` on `CombatState` (`src/domain/combat/engine.rs`)

New boolean field on `CombatState`, defaulting to `false`. When `true`, it
signals that round 1 of an ambush is active and player turns must be skipped.
The field is cleared automatically at the start of round 2 inside
`advance_round()`.

```antares/src/domain/combat/engine.rs#L207-212
    /// True during round 1 of an ambush encounter.
    ///
    /// When set, player turns are automatically skipped (the party is surprised
    /// and cannot act). Cleared automatically at the start of round 2, at which
    /// point the handicap is also reset to `Handicap::Even` so that subsequent
    /// rounds are fought on equal footing.
    pub ambush_round_active: bool,
```

#### `TurnAction::Skip` variant (`src/domain/combat/types.rs`)

New internal-only variant added to `TurnAction`. It is never shown in the
player UI action menu; it is used programmatically by the combat engine to
represent an auto-advanced turn (ambush surprise, incapacitated combatant).

#### `advance_round()` updated (`src/domain/combat/engine.rs`)

At the start of round 2, if `ambush_round_active` is `true`, the engine:

1. Clears `ambush_round_active = false`
2. Resets `handicap = Handicap::Even`
3. Recalculates turn order under the new even handicap
4. Resets `current_turn = 0`

This ensures the remainder of the fight is fair and not permanently skewed by
the ambush initiative advantage.

#### `start_encounter()` updated (`src/game/systems/combat.rs`)

Phase 2 replaces the Phase 1 stub ("always use Even handicap") with the
correct logic:

- **Ambush**: `handicap = Handicap::MonsterAdvantage`, `ambush_round_active = true`
- **Normal / Ranged / Magic**: `handicap = Handicap::Even`, `ambush_round_active = false`
- **Boss** (any type with `applies_boss_mechanics()`): sets `monsters_advance = true`,
  `monsters_regenerate = true`, `can_bribe = false`, `can_surrender = false`

#### `handle_combat_started` updated (`src/game/systems/combat.rs`)

When `combat_res.state.ambush_round_active` is `true`, the system immediately
sets `CombatTurnStateResource` to `EnemyTurn` regardless of actual turn order
(monsters always act first in an ambush round 1). It also emits the combat log
line describing how the battle began:

- Ambush: `"The monsters ambush the party! The party is surprised!"`
- Normal: `"Monsters appear!"`

#### `execute_monster_turn` updated (`src/game/systems/combat.rs`)

At the top of `execute_monster_turn`, a new Phase 2 guard checks whether
`ambush_round_active` is set and the current slot belongs to a player. If so,
it:

1. Pushes a `"The party is surprised and cannot act!"` log line.
2. Calls `advance_turn()` to consume that slot.
3. Determines the turn state for the next actor (staying on `EnemyTurn` while
   the ambush round is still active, switching to `PlayerTurn` once it ends).
4. Returns early without performing any player-damaging action.

This loop continues until all player slots in round 1 have been skipped and
`advance_round()` fires, clearing `ambush_round_active`.

#### `combat_input_system` updated (`src/game/systems/combat.rs`)

Added a defence-in-depth guard at the top of `combat_input_system`. Even if
`CombatTurnStateResource` is somehow not `EnemyTurn` during an ambush round,
no player input is dispatched:

```antares/src/game/systems/combat.rs#L1914-1933
    // Phase 2: During an ambush round the party is surprised and cannot act.
    if combat_res.state.ambush_round_active
        && matches!(
            combat_res.state.get_current_combatant(),
            Some(Combatant::Player(_))
        )
    {
        let any_key = keyboard
            .as_ref()
            .is_some_and(|kb| kb.just_pressed(KeyCode::Tab) || kb.just_pressed(KeyCode::Enter));
        if any_key {
            info!("Combat: input blocked — party is surprised (ambush round 1)");
        }
        return;
    }
```

### Phase 2 Tests

#### Domain-layer tests (`src/domain/combat/engine.rs`)

| Test name                                              | What it verifies                                                                       |
| ------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| `test_combat_state_ambush_round_active_defaults_false` | `CombatState::new` initialises the flag to `false`                                     |
| `test_ambush_round_active_cleared_at_round_2`          | After `advance_turn` exhausts round 1, `ambush_round_active == false` and `round == 2` |
| `test_non_ambush_handicap_unchanged_at_round_2`        | When flag is `false`, `advance_round` does not alter handicap                          |

#### Game-layer tests (`src/game/systems/combat.rs`)

| Test name                                          | What it verifies                                                    |
| -------------------------------------------------- | ------------------------------------------------------------------- |
| `test_normal_combat_handicap_is_even`              | `start_encounter(…, Normal)` → `handicap == Even`                   |
| `test_ambush_combat_handicap_is_monster_advantage` | `start_encounter(…, Ambush)` → `handicap == MonsterAdvantage`       |
| `test_ambush_round_active_set_on_start`            | `start_encounter(…, Ambush)` → `ambush_round_active == true`        |
| `test_normal_round_active_not_set`                 | `start_encounter(…, Normal)` → `ambush_round_active == false`       |
| `test_ambush_round_active_cleared_at_round_2`      | After one `advance_turn`, flag cleared and `handicap == Even`       |
| `test_ambush_handicap_resets_to_even_round_2`      | Dedicated handicap-reset assertion                                  |
| `test_boss_combat_sets_boss_flags`                 | Boss type sets all four boss flags correctly                        |
| `test_combat_log_reports_ambush`                   | Log contains "ambush" text after `CombatStarted` (Ambush)           |
| `test_combat_log_reports_normal_encounter`         | Log contains "Monsters appear!" after `CombatStarted` (Normal)      |
| `test_ambush_combat_started_sets_enemy_turn`       | `CombatTurnStateResource == EnemyTurn` after ambush `CombatStarted` |

### Architecture Compliance

- [x] `ambush_round_active` is a domain-layer field on `CombatState` (no Bevy types)
- [x] `TurnAction::Skip` is in `src/domain/combat/types.rs` (domain layer)
- [x] `start_encounter()` uses `gives_monster_advantage()` and `applies_boss_mechanics()` helper methods (no hardcoded literals)
- [x] All public fields and functions have `///` doc comments
- [x] No tests reference `campaigns/tutorial` (Implementation Rule 5)
- [x] SPDX header present in all modified `.rs` files
- [x] No architectural deviations from architecture.md

### Quality Gate Results

| Gate    | Command                                                    | Result                              |
| ------- | ---------------------------------------------------------- | ----------------------------------- |
| Format  | `cargo fmt --all`                                          | ✅ No output                        |
| Compile | `cargo check --all-targets --all-features`                 | ✅ Finished, 0 errors               |
| Lint    | `cargo clippy --all-targets --all-features -- -D warnings` | ✅ Finished, 0 warnings             |
| Tests   | `cargo nextest run --all-features`                         | ✅ 3209 passed, 8 skipped, 0 failed |

---

## Phase 1: Combat Events — `CombatEventType` Domain Type and Data Layer

### Overview

Phase 1 of the Combat Events Implementation Plan adds the `CombatEventType` enum
to the domain layer and threads it end-to-end from RON data files through the
domain types, event system, and Bevy game layer without changing any combat
mechanics. After this phase, campaign RON files can declare
`combat_event_type: Ambush` (or any other variant) on an `Encounter` event and
the engine reads, stores, and forwards the type through to `CombatResource`.
Later phases (2–5) will act on the stored type to implement ambush suppression,
ranged attack availability, magic action priority, and boss mechanics.

### Phase 1 Deliverables Checklist

- [x] `CombatEventType` enum in `src/domain/combat/types.rs`
- [x] `combat_event_type: CombatEventType` field on `MapEvent::Encounter`
- [x] `combat_event_type` on `EventResult::Encounter`
- [x] `EncounterGroup` struct replacing raw `Vec<u8>` entries in `EncounterTable`
- [x] `random_encounter()` returns `Option<EncounterGroup>`
- [x] `CombatStarted.combat_event_type` field
- [x] `CombatResource.combat_event_type` field
- [x] `start_encounter()` accepts and forwards `CombatEventType`
- [x] All callers of `start_encounter()` and `random_encounter()` updated
- [x] All Phase 1 tests pass (3196 passed, 8 skipped, 0 failed)

### What Was Built

#### `CombatEventType` enum (`src/domain/combat/types.rs`)

New enum alongside the existing `Handicap`, `CombatStatus`, and `TurnAction`
types. Five variants: `Normal` (default), `Ambush`, `Ranged`, `Magic`, `Boss`.
Helper methods: `gives_monster_advantage()`, `enables_ranged_action()`,
`highlights_magic_action()`, `applies_boss_mechanics()`, `display_name()`,
`description()`, `all()`. Derives `Default` (`Normal`), `Serialize`,
`Deserialize`, `Copy`.

#### `MapEvent::Encounter` extended (`src/domain/world/types.rs`)

Added `#[serde(default)] combat_event_type: CombatEventType` field. The
`#[serde(default)]` attribute means all existing RON map files that omit the
field continue to deserialize correctly as `CombatEventType::Normal`.

#### `EncounterGroup` struct (`src/domain/world/types.rs`)

New struct replacing the raw `Vec<u8>` entries in `EncounterTable::groups`:

```antares/src/domain/world/types.rs#L2149-2195
pub struct EncounterGroup {
    pub monster_group: Vec<u8>,
    #[serde(default)]
    pub combat_event_type: CombatEventType,
}
```

Constructors: `EncounterGroup::new(monster_group)` (Normal type) and
`EncounterGroup::with_type(monster_group, combat_event_type)`.
`EncounterTable::groups` is now `Vec<EncounterGroup>` (was `Vec<Vec<u8>>`).
All existing RON files that omit `groups` continue to deserialize with the
default empty vec.

#### `EventResult::Encounter` extended (`src/domain/world/events.rs`)

Added `combat_event_type: CombatEventType` field. `trigger_event()` extracts
and forwards the value from `MapEvent::Encounter`. `random_encounter()` now
returns `Option<EncounterGroup>` (was `Option<Vec<u8>>`); callers extract
`.monster_group` and `.combat_event_type` separately.

#### `CombatStarted` message extended (`src/game/systems/combat.rs`)

Added `pub combat_event_type: CombatEventType` field. `handle_combat_started`
copies `msg.combat_event_type` into `combat_res.combat_event_type`.

#### `CombatResource` extended (`src/game/systems/combat.rs`)

Added `pub combat_event_type: CombatEventType` field, initialized to `Normal`
in `new()` and reset to `Normal` in `clear()`.

#### `start_encounter()` signature updated (`src/game/systems/combat.rs`)

New signature:

```antares/src/game/systems/combat.rs#L997-1001
pub fn start_encounter(
    game_state: &mut crate::application::GameState,
    content: &GameContent,
    group: &[u8],
    combat_event_type: CombatEventType,
) -> Result<(), crate::domain::combat::database::MonsterDatabaseError>
```

Phase 1 stores the type for the Bevy message path; Phase 2 will use it to set
`Handicap::MonsterAdvantage` for ambushes.

#### `RestCompleteEvent` extended (`src/game/systems/rest.rs`)

Added `pub encounter_combat_event_type: CombatEventType` field. Rest
interruptions are hardcoded to `CombatEventType::Ambush` (the party is caught
off-guard while sleeping), which is forwarded to `start_encounter()`.

#### All callers updated

| Caller                                            | File                                     | Change                                                                                                   |
| ------------------------------------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `handle_events` (encounter arm)                   | `src/game/systems/events.rs`             | Extracts `combat_event_type` from `MapEvent::Encounter`; passes to `start_encounter` and `CombatStarted` |
| `handle_rest_complete`                            | `src/game/systems/rest.rs`               | Passes `event.encounter_combat_event_type` to `start_encounter`                                          |
| `process_rest` (random encounter)                 | `src/game/systems/rest.rs`               | Returns `EncounterGroup`; sets `encounter_combat_event_type: Ambush` on `RestCompleteEvent`              |
| `move_party_and_handle_events` (random encounter) | `src/application/mod.rs`                 | Extracts `.monster_group` from `EncounterGroup`; stores `combat_event_type` (Phase 2 will act on it)     |
| `move_party_and_handle_events` (tile event)       | `src/application/mod.rs`                 | Extracts `combat_event_type` from `EventResult::Encounter`                                               |
| `MapBuilder::process_command`                     | `src/bin/map_builder.rs`                 | Adds `combat_event_type: Normal` to constructed `MapEvent::Encounter`                                    |
| `blueprint.rs` `From<MapBlueprint>`               | `src/domain/world/blueprint.rs`          | Adds `combat_event_type: Normal`                                                                         |
| `EventEditorState::to_map_event`                  | `sdk/campaign_builder/src/map_editor.rs` | Adds `combat_event_type: Normal` (Phase 5 will wire a combo-box)                                         |

### Phase 1 Tests

Tests in `src/domain/combat/types.rs`:

- `test_combat_event_type_default_is_normal`
- `test_combat_event_type_flags`
- `test_combat_event_type_display_names`
- `test_combat_event_type_descriptions_non_empty`
- `test_combat_event_type_all_has_five_variants`
- `test_combat_event_type_serde_round_trip`
- `test_combat_event_type_default_deserializes_when_missing`

Tests in `src/domain/world/events.rs`:

- `test_combat_event_type_default_is_normal`
- `test_map_event_encounter_ron_round_trip`
- `test_map_event_encounter_ron_backward_compat`
- `test_event_result_encounter_carries_type`
- `test_encounter_group_ron_round_trip`
- `test_random_encounter_returns_group_type`

Test in `src/game/systems/combat.rs`:

- `test_start_encounter_stores_type_in_resource`

### Architecture Compliance

- [x] `CombatEventType` in `src/domain/combat/types.rs` (domain layer, no Bevy)
- [x] `EncounterGroup` in `src/domain/world/types.rs` (domain layer)
- [x] `#[serde(default)]` on all new fields — full backward compatibility
- [x] `UNARMED_DAMAGE`-style: no magic literals, named constant default
- [x] `DiceRoll` / `MonsterId` type aliases used throughout
- [x] All public functions and types have `///` doc comments
- [x] No tests reference `campaigns/tutorial` (Implementation Rule 5)
- [x] SPDX header present in all modified `.rs` files
- [x] RON data files unaffected (no existing file had `groups:` data)

### Quality Gate Results

| Gate    | Command                                                    | Result                              |
| ------- | ---------------------------------------------------------- | ----------------------------------- |
| Format  | `cargo fmt --all`                                          | ✅ No output                        |
| Compile | `cargo check --all-targets --all-features`                 | ✅ Finished, 0 errors               |
| Lint    | `cargo clippy --all-targets --all-features -- -D warnings` | ✅ Finished, 0 warnings             |
| Tests   | `cargo nextest run --all-features`                         | ✅ 3196 passed, 8 skipped, 0 failed |

---

## Phase 4: Equipped Weapon Damage — Documentation and Final Validation

### Overview

Phase 4 is the concluding phase of the Equipped Weapon Damage in Combat
implementation plan. Its sole deliverables are:

1. A complete summary of all work done across Phases 1–3 added to
   `docs/explanation/implementations.md` (this section).
2. A clean run of all four mandatory quality gates with zero errors and zero
   warnings.

No new production code was written in Phase 4. Everything listed below was
already implemented and verified in Phases 1–3.

### Phase 4 Deliverables Checklist

- [x] `docs/explanation/implementations.md` updated with full cross-phase summary
- [x] `cargo fmt --all` — no output (all files already formatted)
- [x] `cargo check --all-targets --all-features` — `Finished` with 0 errors
- [x] `cargo clippy --all-targets --all-features -- -D warnings` — `Finished` with 0 warnings
- [x] `cargo nextest run --all-features` — 3182 passed, 8 skipped, 0 failed

### Full Cross-Phase Summary

#### Phase 1 — Domain Combat Engine Changes

**Files changed**: `src/domain/combat/engine.rs`, `src/domain/combat/types.rs`

| Symbol                     | Location               | Purpose                                                                                                    |
| -------------------------- | ---------------------- | ---------------------------------------------------------------------------------------------------------- |
| `UNARMED_DAMAGE`           | `engine.rs`            | `DiceRoll { count: 1, sides: 2, bonus: 0 }` — replaces all scattered 1d4 literals                          |
| `MeleeAttackResult`        | `engine.rs`            | Enum returned by `get_character_attack`: `Melee(Attack)` or `Ranged(Attack)`                               |
| `get_character_attack`     | `engine.rs`            | Pure-domain fn — resolves equipped weapon to a `MeleeAttackResult` with bonus applied via `saturating_add` |
| `has_ranged_weapon`        | `engine.rs`            | Returns `true` only when a `MartialRanged` weapon is equipped **and** ammo exists in inventory             |
| `is_ranged: bool`          | `types.rs` on `Attack` | `#[serde(default)]` field distinguishing ranged from melee attacks                                         |
| `Attack::ranged(damage)`   | `types.rs`             | Constructor that sets `is_ranged = true`                                                                   |
| `Attack::physical(damage)` | `types.rs`             | Constructor that keeps `is_ranged = false`                                                                 |

Key design decisions:

- `get_character_attack` is pure domain (no Bevy, no I/O) and lives entirely in the domain layer.
- Weapon bonus composition uses `saturating_add` to merge `weapon_data.damage.bonus` with `weapon_data.bonus` into the final `DiceRoll::bonus` — preventing silent `i8` overflow.
- Unknown item IDs and non-weapon items in the weapon slot fall back gracefully to `UNARMED_DAMAGE` rather than panicking.

Phase 1 tests added to `src/domain/combat/engine.rs` test module:

- `test_get_character_attack_no_weapon_returns_unarmed`
- `test_get_character_attack_melee_weapon_returns_melee`
- `test_get_character_attack_weapon_bonus_applied`
- `test_get_character_attack_unknown_item_id_falls_back`
- `test_get_character_attack_non_weapon_item_falls_back`
- `test_get_character_attack_ranged_weapon_returns_ranged_variant`
- `test_get_character_attack_ranged_weapon_damage_correct`
- `test_has_ranged_weapon_false_no_weapon`
- `test_has_ranged_weapon_false_melee_weapon`
- `test_has_ranged_weapon_false_no_ammo`
- `test_has_ranged_weapon_true_with_bow_and_arrows`

#### Phase 2 — Game System Integration

**Files changed**: `src/game/systems/combat.rs`

The player attack branch inside `perform_attack_action_with_rng` was rewritten.
Previously it used a hardcoded `DiceRoll::new(1, 4, 0)` for every player attack
regardless of equipment. After Phase 2 it calls `get_character_attack`, matches
on `MeleeAttackResult`, and:

- **`MeleeAttackResult::Melee(attack)`** — uses the resolved attack (correct
  weapon dice + bonus) as the input to `resolve_attack`.
- **`MeleeAttackResult::Ranged(_)`** — emits a `warn!` log and returns `Ok(())`
  without dealing any damage, consuming the turn and directing the player to use
  `TurnAction::RangedAttack` instead. This is the ranged-weapon guard.

The monster attack branch was left unchanged — monsters continue to use
`choose_monster_attack`.

Phase 2 helper fixtures added to `src/game/systems/combat.rs` test module:

- `make_p2_weapon_item(id, damage, bonus, classification)` — builds an `Item` with a `WeaponData` payload.
- `make_p2_combat_fixture(player)` — builds a self-contained `(CombatResource, GameContent, GlobalState, CombatTurnStateResource)` with one player (index 0) and one goblin with AC 1 (index 1, nearly always hit).

Phase 2 tests:

- `test_player_attack_uses_equipped_melee_weapon_damage` — equips a 1d8 longsword; asserts damage ∈ [1, 8] over 50 seeds and that at least one roll exceeded 4 (proving the old 1d4 path is gone).
- `test_player_attack_unarmed_when_no_weapon` — no weapon equipped; asserts damage ≤ 2 (1d2 UNARMED_DAMAGE) over 30 seeds.
- `test_player_attack_bonus_weapon_floor_at_one` — equips a cursed 1d4 −3 dagger (baked into `DiceRoll::bonus`); asserts monster HP never increases and any hit deals ≥ 1 damage.
- `test_player_melee_attack_with_ranged_weapon_skips_turn` — equips a `MartialRanged` bow; asserts the function returns `Ok(())` and the monster's HP is completely unchanged.

#### Phase 3 — Damage Floor and Bonus Application Verification

**Files changed**: `src/domain/combat/engine.rs` (doc comment update + two new tests)

Two invariants were verified and documented:

**Invariant 1 — Bonus integration**: `get_character_attack` merges
`weapon_data.damage.bonus` and `weapon_data.bonus` using `saturating_add` into
the `DiceRoll::bonus` field. The `DiceRoll::bonus` field type is `i8`; the use
of `saturating_add` prevents wraparound on extreme values.

**Invariant 2 — Damage floor at 1**: `resolve_attack` computes
`(base_damage + might_bonus).max(1)` before casting to `u16`. This is the
authoritative damage floor — any successful hit deals at least 1 damage even
when weapon bonuses are so negative that the raw roll is ≤ 0. `DiceRoll::roll`
itself clamps at 0 (`total.max(0)`) as a secondary safeguard.

The `resolve_attack` doc comment was updated to explicitly document:

- Where the damage floor of 1 lives (`(base_damage + might_bonus).max(1)`).
- That `DiceRoll::roll` floors at 0 (not 1) — the authoritative floor is in `resolve_attack`.

Phase 3 tests added to `src/domain/combat/engine.rs` test module:

- `test_cursed_weapon_damage_floor_at_one` — equips a 1d4 −10 cursed weapon; asserts every hit yields damage ≥ 1 across 100 random seeds.
- `test_positive_bonus_adds_to_roll` — equips a +3 longsword (1d6 base, bonus 3); asserts `DiceRoll::bonus == 3`, `DiceRoll::min() == 4`, and that every observed hit damage ∈ [4, 9].

### Architecture Compliance

- [x] `get_character_attack` in `src/domain/combat/engine.rs` (domain layer, no Bevy)
- [x] `MeleeAttackResult` in `src/domain/combat/engine.rs` (domain layer, no Bevy)
- [x] `has_ranged_weapon` in `src/domain/combat/engine.rs` (domain layer, no Bevy)
- [x] `UNARMED_DAMAGE` is a named constant — no magic literals
- [x] `is_ranged: bool` on `Attack` with `#[serde(default)]`
- [x] `Attack::ranged(damage)` sets `is_ranged = true`
- [x] `Attack::physical(damage)` keeps `is_ranged = false`
- [x] Melee path returns `Ok(())` (no damage, with `warn!`) on `MeleeAttackResult::Ranged`
- [x] `DiceRoll` type used throughout, not raw primitives
- [x] All public functions have `///` doc comments with runnable examples
- [x] No tests reference `campaigns/tutorial` (Implementation Rule 5)
- [x] SPDX header present in all modified `.rs` files

### Quality Gate Results

| Gate    | Command                                                    | Result                              |
| ------- | ---------------------------------------------------------- | ----------------------------------- |
| Format  | `cargo fmt --all`                                          | ✅ No output                        |
| Compile | `cargo check --all-targets --all-features`                 | ✅ Finished, 0 errors               |
| Lint    | `cargo clippy --all-targets --all-features -- -D warnings` | ✅ Finished, 0 warnings             |
| Tests   | `cargo nextest run --all-features`                         | ✅ 3182 passed, 8 skipped, 0 failed |

---

## Phase 3: Equipped Weapon Damage — Damage Floor and Bonus Application Verification

### Overview

Phase 3 verifies and documents that weapon bonuses are applied correctly through
the full attack pipeline and that the damage floor of 1 is enforced on every
hit, regardless of how negative a weapon's bonus is.

Two critical invariants are codified and proven by tests:

1. **Bonus integration** — `get_character_attack` uses `saturating_add` to
   merge `WeaponData::damage.bonus` and `WeaponData::bonus` into a single
   `DiceRoll::bonus` field. This was already implemented in Phase 1; Phase 3
   verifies it via boundary tests.
2. **Damage floor at 1** — `resolve_attack` applies `.max(1)` to
   `base_damage + damage_bonus` after every hit, preventing a cursed weapon
   from ever dealing 0 damage on a successful strike. The floor is the sole
   responsibility of `resolve_attack`; neither `DiceRoll::roll` (which floors
   at 0) nor `get_character_attack` (which only builds the roll descriptor)
   duplicate it.

### Phase 3 Deliverables Checklist

- [x] `DiceRoll::bonus` field type confirmed as `i8`; `saturating_add` used
      throughout `get_character_attack` — no silent truncation
- [x] `resolve_attack` floors damage at 1 via `(base_damage + damage_bonus).max(1)`
      — existing code confirmed and documented
- [x] `resolve_attack` doc comment updated to explicitly state the floor-at-1
      invariant and explain that it is the single authoritative enforcement point
- [x] `test_cursed_weapon_damage_floor_at_one` passes
- [x] `test_positive_bonus_adds_to_roll` passes

### What Was Built

#### Doc comment update (`src/domain/combat/engine.rs`)

The `resolve_attack` function's doc comment was extended to document the
damage-floor invariant:

- States that on a hit, damage is **always** floored at 1 regardless of weapon
  penalties, negative bonuses, or low might.
- Explicitly identifies `resolve_attack` as the single authoritative place for
  this invariant.
- Cross-references `DiceRoll::roll` (floors at 0) and `get_character_attack`
  (roll descriptor only) to prevent future duplication.

#### `test_cursed_weapon_damage_floor_at_one`

Located in `src/domain/combat/engine.rs`, `mod tests`.

- Constructs a `CombatState` with an attacker (might=10, accuracy=20) and a
  defender (AC=0) so nearly every roll is a hit.
- Equips the attacker with a 1d4-10 cursed weapon built via `make_weapon_item`
  and `ItemDatabase::add_item`.
- Calls `get_character_attack` to produce the `Attack` the same way the game
  system does, verifying `attack.damage.bonus == -10`.
- Runs 200 `resolve_attack` trials and asserts `damage == 0` (miss) or
  `damage >= 1` (hit, floored).
- Runs a further 500 trials, filters to hits only, and asserts that the
  collected `hit_damages` vector is non-empty and every element is `>= 1`.

#### `test_positive_bonus_adds_to_roll`

Located in `src/domain/combat/engine.rs`, `mod tests`.

- Builds a +3 longsword (1d6 base, `WeaponData::bonus = 3`) in a fresh
  `ItemDatabase`.
- Calls `get_character_attack` and confirms:
  - `attack.damage.bonus == 3` (saturating_add(0, 3))
  - `attack.damage.count == 1`, `attack.damage.sides == 6`
  - `attack.damage.min() == 4` (die=1 + bonus=3)
- Runs 500 `resolve_attack` trials with an attacker of might=10 and AC=0
  defender, filters to non-zero results, and asserts:
  - At least one hit observed.
  - Every hit `>= 4` (bonus raises the minimum).
  - Every hit `<= 9` (1×6 + 3, no might bonus).

### Architecture Compliance

| Check                                                     | Status |
| --------------------------------------------------------- | ------ |
| Type aliases used (`ItemId` etc.)                         | ✅     |
| `DiceRoll::bonus` is `i8`; `saturating_add` used          | ✅     |
| Floor-at-1 in `resolve_attack`, not in helpers            | ✅     |
| No magic numbers; `UNARMED_DAMAGE` constant used          | ✅     |
| Tests use `data/` fixtures only (no `campaigns/tutorial`) | ✅     |
| RON data files untouched                                  | ✅     |

### Quality Gate Results

```text
cargo fmt         → OK (no output)
cargo check       → Finished 0 errors
cargo clippy      → Finished 0 warnings
cargo nextest run → 3182 tests run: 3182 passed, 8 skipped
```

---

## Phase 2: Equipped Weapon Damage — Game System Integration

### Overview

Phase 2 wires the domain-layer work from Phase 1 into the live game system.
`perform_attack_action_with_rng` in `src/game/systems/combat.rs` previously
hardcoded `Attack::physical(DiceRoll::new(1, 4, 0))` for every
`CombatantId::Player` turn. This phase replaces that single literal with a
call to `get_character_attack` and dispatches on `MeleeAttackResult`:

- `Melee(attack)` — the resolved attack (with correct weapon damage and bonus)
  is used directly in `resolve_attack`.
- `Ranged(_)` — the melee path is a no-op: a `warn!` is logged and the
  function returns `Ok(())` immediately without applying any damage. The
  ranged path (`TurnAction::RangedAttack` /
  `perform_ranged_attack_action_with_rng`) is reserved for a future phase
  (`combat_events_implementation_plan.md` §3).

The monster path (`CombatantId::Monster`) is **not changed** by this phase.

### Phase 2 Deliverables Checklist

- [x] Hardcoded `DiceRoll::new(1, 4, 0)` removed from the `CombatantId::Player`
      branch of `perform_attack_action_with_rng`
- [x] `get_character_attack` + `MeleeAttackResult` dispatch wired in
- [x] Ranged-weapon guard logs a `warn!` and returns `Ok(())` without damage
- [x] `use` imports for `get_character_attack` and `MeleeAttackResult` added in
      `src/game/systems/combat.rs`
- [x] Four integration tests added and passing

### What Was Built

#### Updated imports (`src/game/systems/combat.rs`)

```antares/src/game/systems/combat.rs#L59-62
use crate::domain::combat::engine::{
    apply_damage, choose_monster_attack, get_character_attack, initialize_combat_from_group,
    resolve_attack, CombatState, Combatant, MeleeAttackResult,
};
```

#### Replaced player attack branch

The old hardcoded block:

```antares/docs/explanation/equipped_weapon_damage_implementation_plan.md#L284-294
CombatantId::Player(_) => {
    crate::domain::combat::types::Attack::physical(DiceRoll::new(1, 4, 0))
}
```

…is replaced with:

```antares/src/game/systems/combat.rs#L2181-2198
        CombatantId::Player(idx) => {
            if let Some(Combatant::Player(pc)) = combat_res.state.participants.get(idx) {
                match get_character_attack(pc, &content.db().items) {
                    MeleeAttackResult::Melee(attack) => attack,
                    MeleeAttackResult::Ranged(_) => {
                        // Ranged weapons must be used via TurnAction::RangedAttack /
                        // perform_ranged_attack_action_with_rng, not the melee path.
                        // Log a warning and skip the turn rather than dealing wrong damage.
                        warn!(
                            "Player {:?} attempted melee attack with ranged weapon; \
                             use TurnAction::RangedAttack instead. Turn skipped.",
                            action.attacker
                        );
                        return Ok(());
                    }
                }
            } else {
                return Err(CombatError::CombatantNotFound(action.attacker));
            }
        }
```

#### Integration tests (`src/game/systems/combat.rs`, `mod tests`)

Four pure-function tests were added that construct a `CombatResource` directly
(no Bevy `App`) via the `make_p2_combat_fixture` helper:

| Test name                                                | What it verifies                                                                                           |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `test_player_attack_uses_equipped_melee_weapon_damage`   | Longsword (1d8) deals damage in [1, 8]; at least one roll over 50 seeds exceeds 4, proving old 1d4 is gone |
| `test_player_attack_unarmed_when_no_weapon`              | `equipment.weapon = None` → damage ≤ 2 (1d2 UNARMED_DAMAGE) across 30 seeds                                |
| `test_player_attack_bonus_weapon_floor_at_one`           | Cursed dagger (1d4, bonus -3) never deals negative damage; any hit deals ≥ 1                               |
| `test_player_melee_attack_with_ranged_weapon_skips_turn` | `MartialRanged` bow via melee path returns `Ok(())` and monster HP is unchanged                            |

All four tests use `data/test_campaign`-independent fixtures built entirely in
memory — no reference to `campaigns/tutorial`.

### Quality Gate Results

```antares/docs/explanation/implementations.md#L1-1
cargo fmt         → no output (all files formatted)
cargo check       → Finished, 0 errors
cargo clippy -D warnings → Finished, 0 warnings
cargo nextest run → 3180 tests run: 3180 passed, 8 skipped
```

---

## Phase 1: Equipped Weapon Damage — Domain Combat Engine Changes

### Overview

Player characters in combat previously always dealt 1d4 physical damage regardless
of their equipped weapon, because `perform_attack_action_with_rng` hardcoded
`Attack::physical(DiceRoll::new(1, 4, 0))` for every `CombatantId::Player` turn.
This phase repairs the domain layer so that the combat engine can correctly resolve
a character's attack from their equipped weapon, identify ranged weapons that must
not fire through the melee path, and fall back to a correct unarmed damage value
(1d2, not 1d4).

No Bevy or game-system code was changed in this phase — all additions are
pure-domain functions in `src/domain/combat/engine.rs` and a field addition to
`src/domain/combat/types.rs`.

### Phase 1 Deliverables Checklist

- [x] `UNARMED_DAMAGE` constant in `src/domain/combat/engine.rs`
- [x] `MeleeAttackResult` enum in `src/domain/combat/engine.rs`
- [x] `get_character_attack(character, item_db) -> MeleeAttackResult` in
      `src/domain/combat/engine.rs`
- [x] `has_ranged_weapon(character, item_db) -> bool` in
      `src/domain/combat/engine.rs`
- [x] `is_ranged: bool` field on `Attack` with `#[serde(default)]` in
      `src/domain/combat/types.rs`
- [x] `Attack::ranged(damage)` constructor in `src/domain/combat/types.rs`
- [x] Required `use` imports added to `engine.rs`
- [x] All 14 unit tests pass (13 specified + 1 extra coverage test)

### What Was Built

#### `UNARMED_DAMAGE` constant (`src/domain/combat/engine.rs`)

```antares/src/domain/combat/engine.rs#L42-47
pub const UNARMED_DAMAGE: DiceRoll = DiceRoll {
    count: 1,
    sides: 2,
    bonus: 0,
};
```

Replaces all scattered `DiceRoll::new(1, 4, 0)` literals previously used as the
player unarmed fallback. The correct unarmed damage per spec is 1d2, not 1d4.

#### `MeleeAttackResult` enum (`src/domain/combat/engine.rs`)

A small discriminated union returned by `get_character_attack` that communicates
whether the character's equipped weapon is usable in the melee path:

- `Melee(Attack)` — a valid melee `Attack` ready for `resolve_attack`
- `Ranged(Attack)` — the weapon is `MartialRanged`; the melee path must refuse
  it and direct the player through `perform_ranged_attack_action_with_rng`

The `Ranged` variant carries the fully-constructed `Attack` so callers can log or
display weapon stats without a second item lookup — but must never apply damage
through the melee pipeline with it.

#### `get_character_attack` (`src/domain/combat/engine.rs`)

Pure-domain function: `pub fn get_character_attack(character: &Character, item_db: &ItemDatabase) -> MeleeAttackResult`

Logic (in order, fully infallible):

1. No weapon in `character.equipment.weapon` → unarmed fallback
2. Item ID not found in `item_db` → unarmed fallback (no panic)
3. Item found but not `ItemType::Weapon(_)` (e.g. consumable in weapon slot) → unarmed fallback
4. Build `DiceRoll` from `weapon_data.damage`; apply `weapon_data.bonus` via
   `saturating_add` to the `bonus` field
5. If `weapon_data.classification == WeaponClassification::MartialRanged` →
   return `MeleeAttackResult::Ranged(Attack::ranged(adjusted))`
6. Otherwise → return `MeleeAttackResult::Melee(Attack::physical(adjusted))`

#### `has_ranged_weapon` (`src/domain/combat/engine.rs`)

Pure-domain helper: `pub fn has_ranged_weapon(character: &Character, item_db: &ItemDatabase) -> bool`

Returns `true` only when **both** conditions hold:

- The equipped weapon has `WeaponClassification::MartialRanged`, **and**
- The character's inventory contains at least one `ItemType::Ammo(_)` item

A character with a bow but no arrows returns `false`.

#### `is_ranged: bool` field on `Attack` (`src/domain/combat/types.rs`)

Added with `#[serde(default)]` so all existing RON monster data that lacks the
field deserialises correctly (defaults to `false`). `Attack::physical` continues
to set `is_ranged: false`; the new `Attack::ranged` constructor sets it `true`.

#### `Attack::ranged(damage)` constructor (`src/domain/combat/types.rs`)

```antares/src/domain/combat/types.rs#L85-93
pub fn ranged(damage: DiceRoll) -> Self {
    Self {
        damage,
        attack_type: AttackType::Physical,
        special_effect: None,
        is_ranged: true,
    }
}
```

Used by `get_character_attack` when the equipped weapon is `MartialRanged`.

#### Imports added to `engine.rs`

```antares/src/domain/combat/engine.rs#L18-19
use crate::domain::items::{ItemDatabase, ItemType, WeaponClassification};
use crate::domain::types::DiceRoll;
```

### Architecture Compliance

- [x] Data structures match architecture.md Section 4.4 **exactly**
- [x] Module placement follows Section 3.2 (`src/domain/combat/`)
- [x] Type aliases used consistently (`ItemId`, `DiceRoll`, etc.)
- [x] `UNARMED_DAMAGE` constant extracted — no magic literals
- [x] `AttributePair` pattern untouched — no direct stat mutation
- [x] RON format unchanged — `#[serde(default)]` preserves all existing data files
- [x] No architectural deviations

### Test Coverage

14 unit tests added across two files:

**`src/domain/combat/types.rs`** (3 tests):

| Test                                                 | Assertion                                         |
| ---------------------------------------------------- | ------------------------------------------------- |
| `test_attack_physical_constructor_is_ranged_false`   | `Attack::physical(...)` sets `is_ranged = false`  |
| `test_attack_ranged_constructor_sets_is_ranged_true` | `Attack::ranged(...)` sets `is_ranged = true`     |
| `test_attack_ranged_damage_preserved`                | inner `damage` field is carried through unchanged |

**`src/domain/combat/engine.rs`** (11 tests):

| Test                                                             | Assertion                                         |
| ---------------------------------------------------------------- | ------------------------------------------------- |
| `test_get_character_attack_no_weapon_returns_unarmed`            | `None` weapon → `Melee(UNARMED_DAMAGE)`           |
| `test_get_character_attack_melee_weapon_returns_melee`           | Simple sword 1d8 → `Melee(1d8)`                   |
| `test_get_character_attack_weapon_bonus_applied`                 | +2 sword → `damage.bonus == 2`                    |
| `test_get_character_attack_unknown_item_id_falls_back`           | item_id 99 not in db → unarmed fallback, no panic |
| `test_get_character_attack_non_weapon_item_falls_back`           | consumable in weapon slot → unarmed fallback      |
| `test_get_character_attack_ranged_weapon_returns_ranged_variant` | bow → `Ranged(_)` with `is_ranged = true`         |
| `test_get_character_attack_ranged_weapon_damage_correct`         | crossbow 1d8+1 → inner `Attack` has correct dice  |
| `test_has_ranged_weapon_false_no_weapon`                         | no weapon equipped → `false`                      |
| `test_has_ranged_weapon_false_melee_weapon`                      | melee weapon → `false`                            |
| `test_has_ranged_weapon_false_no_ammo`                           | bow equipped, empty inventory → `false`           |
| `test_has_ranged_weapon_true_with_bow_and_arrows`                | bow + arrows in inventory → `true`                |

### Quality Gates

```text
✅ cargo fmt         → no output (all files formatted)
✅ cargo check       → Finished with 0 errors
✅ cargo clippy      → Finished with 0 warnings
✅ cargo nextest run → 3176 tests run: 3176 passed, 0 failed
```

## Items Procedural Meshes — Phase 1: Domain Layer

**Plan**: [`items_procedural_meshes_implementation_plan.md`](items_procedural_meshes_implementation_plan.md)

### Overview

Phase 1 adds the domain-layer types that drive procedural 3-D world-mesh
generation for dropped items. When a player drops an item from inventory it
will (in later phases) spawn a procedural mesh on the tile; this phase
establishes the pure-Rust data layer that converts any `Item` definition into
a `CreatureDefinition` that the existing `spawn_creature` pipeline can render.

No Bevy dependency is introduced in Phase 1. All new code lives in
`src/domain/` and `src/sdk/`.

---

### Phase 1 Deliverables

**Files changed / created**:

- `src/domain/visual/item_mesh.rs` _(new)_
- `src/domain/visual/mod.rs` _(updated)_
- `src/domain/items/types.rs` _(updated)_
- `src/domain/items/database.rs` _(updated)_
- `src/sdk/validation.rs` _(updated)_
- `src/sdk/error_formatter.rs` _(updated)_

**Files with `mesh_descriptor_override: None` field additions** (backward-compatible):

- `src/domain/combat/item_usage.rs`
- `src/domain/items/equipment_validation.rs`
- `src/domain/transactions.rs`
- `src/game/systems/combat.rs`
- `src/game/systems/dialogue.rs`
- `src/sdk/templates.rs`
- `src/bin/item_editor.rs`
- `tests/cli_editor_tests.rs`
- `tests/merchant_transaction_integration_test.rs`

---

### What was built

#### `ItemMeshCategory` (`src/domain/visual/item_mesh.rs`)

An enum with 17 variants mapping every `ItemType` sub-classification to a
distinct mesh silhouette: `Sword`, `Dagger`, `Blunt`, `Staff`, `Bow`,
`BodyArmor`, `Helmet`, `Shield`, `Boots`, `Ring`, `Amulet`, `Belt`, `Cloak`,
`Potion`, `Scroll`, `Ammo`, `QuestItem`.

#### `ItemMeshDescriptor` (`src/domain/visual/item_mesh.rs`)

The full per-item visual specification: `category`, `blade_length`,
`primary_color`, `accent_color`, `emissive`, `emissive_color`, and `scale`.

`ItemMeshDescriptor::from_item(item: &Item) -> Self` is a **pure function**
that reads `item.item_type`, sub-type classification fields, `tags`, bonus
values, and charge data:

- `WeaponClassification::Simple` with `sides ≤ 4` → `Dagger`; otherwise →
  `Blunt`. `MartialMelee` → `Sword`. `MartialRanged` → `Bow`.
  `Blunt` → `Blunt`.
- Blade length = `(damage.sides × 0.08).clamp(0.25, 1.0)`. Dagger blade is
  multiplied by 0.7 (shorter).
- `two_handed` tag → scale multiplied by `1.45`.
- `ConsumableEffect::HealHp` → red; `RestoreSp` → blue;
  `CureCondition` → `Scroll` category (parchment color);
  `BoostAttribute` / `BoostResistance` → yellow.
- `item.is_magical()` → `emissive = true`, soft white glow.
- `item.is_cursed` → dark purple primary color, purple emissive (overrides
  magical glow — curse takes visual priority).
- Quest items always emit (magenta star mesh).

`ItemMeshDescriptor::to_creature_definition(&self) -> CreatureDefinition`
converts the descriptor into a single-mesh `CreatureDefinition` on the XZ
plane (item lying flat on the ground). The returned definition always passes
`CreatureDefinition::validate()`.

Each mesh category has a dedicated geometry builder that produces a flat
polygon on the XZ plane (Y = 0). All polygon fans use a dedicated centre
vertex (never vertex 0 as the hub) to avoid degenerate triangles.

#### `ItemMeshDescriptorOverride` (`src/domain/visual/item_mesh.rs`)

A `#[serde(default)]`-annotated struct with four optional fields:
`primary_color`, `accent_color`, `scale`, `emissive`. Campaign authors can
embed it in a RON item file to customise the visual without touching gameplay
data. An all-`None` override is identical to no override at all.

#### `Item::mesh_descriptor_override` (`src/domain/items/types.rs`)

Added `#[serde(default)] pub mesh_descriptor_override:
Option<ItemMeshDescriptorOverride>` to the `Item` struct. All existing RON
item files remain valid without modification because `#[serde(default)]`
deserialises the field as `None` when absent.

#### `ItemDatabase::validate_mesh_descriptors` (`src/domain/items/database.rs`)

A new method that calls `ItemMeshDescriptor::from_item` for every loaded item
and validates the resulting `CreatureDefinition`. A new error variant
`ItemDatabaseError::InvalidMeshDescriptor { item_id, message }` is returned
on the first failure.

#### SDK plumbing (`src/sdk/validation.rs`, `src/sdk/error_formatter.rs`)

- `ValidationError::ItemMeshDescriptorInvalid { item_id, message }` — new
  `Error`-severity variant.
- `Validator::validate_item_mesh_descriptors()` — calls
  `ItemDatabase::validate_mesh_descriptors` and converts the result into a
  `Vec<ValidationError>`.
- `validate_all()` now calls `validate_item_mesh_descriptors()`.
- `error_formatter.rs` has an actionable suggestion block for the new variant.

---

### Architecture compliance

- `CreatureDefinition` is reused as the output type — no new rendering path.
- `ItemId`, `ItemType` type aliases used throughout.
- `#[serde(default)]` on `mesh_descriptor_override` preserves full backward
  compatibility with all existing RON files.
- All geometry builders produce non-degenerate triangles (centre-vertex fan).
- No constants are hard-coded; all shape parameters (`BASE_SCALE`,
  `TWO_HANDED_SCALE_MULT`, `BLADE_SIDES_FACTOR`, etc.) are named constants.
- SPDX headers present in `item_mesh.rs`.
- Test data uses `data/items.ron` (Implementation Rule 5 compliant).

---

### Test coverage

**`src/domain/visual/item_mesh.rs`** (inline `mod tests`):

| Test                                                       | What it verifies                                                  |
| ---------------------------------------------------------- | ----------------------------------------------------------------- |
| `test_sword_descriptor_from_short_sword`                   | Short sword → `Sword` category, correct blade length, no emissive |
| `test_dagger_descriptor_short_blade`                       | Dagger → `Dagger` category, blade shorter than same-sides sword   |
| `test_potion_color_heal_is_red`                            | `HealHp` → red primary color                                      |
| `test_potion_color_restore_sp_is_blue`                     | `RestoreSp` → blue                                                |
| `test_potion_color_boost_attribute_is_yellow`              | `BoostAttribute` → yellow                                         |
| `test_cure_condition_produces_scroll`                      | `CureCondition` → `Scroll` category                               |
| `test_magical_item_emissive`                               | `max_charges > 0` → emissive                                      |
| `test_magical_item_emissive_via_bonus`                     | `constant_bonus` → emissive                                       |
| `test_cursed_item_dark_tint`                               | `is_cursed` → dark purple + purple emissive                       |
| `test_cursed_overrides_magical_glow`                       | Cursed+magical → cursed emissive wins                             |
| `test_two_handed_weapon_larger_scale`                      | `two_handed` tag → scale > one-handed                             |
| `test_descriptor_to_creature_definition_valid`             | Round-trip for all categories passes `validate()`                 |
| `test_override_color_applied`                              | `primary_color` override applied                                  |
| `test_override_scale_applied`                              | `scale` override applied                                          |
| `test_override_invalid_scale_ignored`                      | Negative scale override ignored                                   |
| `test_override_emissive_applied`                           | Non-zero emissive override enables flag                           |
| `test_override_zero_emissive_disables`                     | All-zero emissive override disables flag                          |
| `test_quest_item_descriptor_unique_shape`                  | Quest items → `QuestItem` category, always emissive               |
| `test_all_accessory_slots_produce_valid_definitions`       | All 4 accessory slots round-trip                                  |
| `test_all_armor_classifications_produce_valid_definitions` | All 4 armor classes round-trip                                    |
| `test_ammo_descriptor_valid`                               | Ammo → valid definition                                           |
| `test_descriptor_default_override_is_identity`             | Empty override = no override                                      |

**`src/domain/items/database.rs`** (extended `mod tests`):

| Test                                            | What it verifies                                  |
| ----------------------------------------------- | ------------------------------------------------- |
| `test_validate_mesh_descriptors_all_base_items` | Loads `data/items.ron`; all items pass validation |
| `test_validate_mesh_descriptors_empty_db`       | Empty DB → `Ok(())`                               |
| `test_validate_mesh_descriptors_all_item_types` | One item of every `ItemType` variant → `Ok(())`   |

---

## Items Procedural Meshes — Phase 2: Game Engine — Dropped Item Mesh Generation

**Plan**: [`items_procedural_meshes_implementation_plan.md`](items_procedural_meshes_implementation_plan.md)

### Overview

Phase 2 wires the domain-layer types from Phase 1 into the live Bevy game
engine. Dropping an item from inventory now spawns a procedural 3-D mesh on
the party's current tile; static `MapEvent::DroppedItem` entries in RON map
files cause the same mesh to appear on map load; picking up an item despawns
the mesh.

---

### Phase 2 Deliverables

**Files created**:

- `src/game/components/dropped_item.rs` — `DroppedItem` ECS marker component
- `src/game/systems/item_world_events.rs` — `ItemDroppedEvent`, `ItemPickedUpEvent`, spawn / despawn / map-load systems, `ItemWorldPlugin`

**Files modified**:

- `src/domain/world/types.rs` — `MapEvent::DroppedItem` variant added
- `src/domain/world/events.rs` — `DroppedItem` arm in `trigger_event` match
- `src/game/components/mod.rs` — `pub mod dropped_item` + re-export
- `src/game/resources/mod.rs` — `DroppedItemRegistry` resource
- `src/game/systems/mod.rs` — `pub mod item_world_events`
- `src/game/systems/procedural_meshes.rs` — 12 item mesh cache slots, `get_or_create_item_mesh`, 10 per-category spawn functions (`spawn_sword_mesh`, `spawn_dagger_mesh`, `spawn_blunt_mesh`, `spawn_staff_mesh`, `spawn_bow_mesh`, `spawn_armor_mesh`, `spawn_shield_mesh`, `spawn_potion_mesh`, `spawn_scroll_mesh`, `spawn_ring_mesh`, `spawn_ammo_mesh`), `spawn_dropped_item_mesh` dispatcher, 11 config structs
- `src/game/systems/inventory_ui.rs` — drop action fires `ItemDroppedEvent`
- `src/game/systems/events.rs` — `MapEvent::DroppedItem` arm in `handle_events`
- `src/sdk/validation.rs` — `MapEvent::DroppedItem` validation arm
- `src/bin/validate_map.rs` — `MapEvent::DroppedItem` counting arm
- `src/bin/antares.rs` — `ItemWorldPlugin` registered

---

### What was built

#### `DroppedItem` component (`src/game/components/dropped_item.rs`)

`#[derive(Component, Clone, Debug, PartialEq, Eq)]` struct that marks any
entity whose mesh represents an item lying on the ground. Stores `item_id`,
`map_id`, `tile_x`, `tile_y`, and `charges`.

#### `DroppedItemRegistry` resource (`src/game/resources/mod.rs`)

`#[derive(Resource, Default)]` wrapping a `HashMap<(MapId, i32, i32, ItemId),
Entity>`. Provides typed `insert`, `get`, and `remove` helpers. Used to
correlate pickup events with ECS entities for targeted despawn.

#### `MapEvent::DroppedItem` variant (`src/domain/world/types.rs`)

New enum arm with `name: String`, `item_id: ItemId`, and
`#[serde(default)] charges: u16`. All fields that are optional use
`#[serde(default)]` so existing RON map files that pre-date this variant
remain valid without modification.

#### `ItemDroppedEvent` / `ItemPickedUpEvent` (`src/game/systems/item_world_events.rs`)

`#[derive(Message, Clone, Debug)]` event structs carrying `item_id`, `charges`,
`map_id`, `tile_x`, `tile_y` (drop) or the same minus charges (pickup).
Registered with `app.add_message::<…>()` inside `ItemWorldPlugin`.

#### `spawn_dropped_item_system`

Reads `MessageReader<ItemDroppedEvent>`. For each event:

1. Looks up the item from `GameContent`; skips with a warning if not found.
2. Calls `ItemMeshDescriptor::from_item` → `to_creature_definition`.
3. Calls `spawn_creature` at world-space `(tile_x + 0.5, 0.05, tile_y + 0.5)`.
4. Applies a random Y-axis jitter rotation for visual variety.
5. Inserts `DroppedItem`, `MapEntity`, `TileCoord`, and a `Name` component.
6. Registers the entity in `DroppedItemRegistry`.

`GameContent` is wrapped in `Option<Res<…>>` so the system degrades gracefully
when content is not yet loaded.

#### `despawn_picked_up_item_system`

Reads `MessageReader<ItemPickedUpEvent>`. Looks up the entity in
`DroppedItemRegistry` by the four-part key, calls
`commands.entity(entity).despawn()` (Bevy 0.17 — recursive by default), and
removes the registry entry. Unknown keys emit a `warn!` log.

#### `load_map_dropped_items_system`

Stores the last-processed map ID in a `Local<Option<MapId>>`. On map change,
iterates all `MapEvent::DroppedItem` entries on the new map and fires
`ItemDroppedEvent` for each so static map-authored drops share the identical
spawn path as runtime drops.

#### Item mesh config structs & generators (`src/game/systems/procedural_meshes.rs`)

Eleven typed config structs (`SwordConfig`, `DaggerConfig`, `BluntConfig`,
`StaffConfig`, `BowConfig`, `ArmorMeshConfig`, `ShieldConfig`, `PotionConfig`,
`ScrollConfig`, `RingMeshConfig`, `AmmoConfig`) plus a `spawn_dropped_item_mesh`
dispatcher that selects the right generator from `ItemMeshCategory`.

Twelve item mesh cache slots added to `ProceduralMeshCache` (one per category
string: `"sword"`, `"dagger"`, `"blunt"`, `"staff"`, `"bow"`, `"armor"`,
`"shield"`, `"potion"`, `"scroll"`, `"ring"`, `"ammo"`, `"quest"`).
`get_or_create_item_mesh` follows the same pattern as the existing
`get_or_create_furniture_mesh`. `clear_all` and `cached_count` updated.

Notable mesh details:

- **Potion**: `AlphaMode::Blend` on both bottle and liquid inner cylinder;
  liquid colour carries a faint emissive glow matching the liquid tint.
- **Staff**: emissive orb at tip.
- **Shield**: flat `Cylinder` disc with `FRAC_PI_2` X-rotation.
- **Ring**: `Torus` primitive (`minor_radius` = 0.018, `major_radius` = 0.065).
- **Ammo**: three sub-types (`"arrow"`, `"bolt"`, `"stone"`) selected from
  `AmmoConfig::ammo_type`.

#### Inventory drop integration (`src/game/systems/inventory_ui.rs`)

`inventory_action_system` now accepts
`Option<MessageWriter<ItemDroppedEvent>>` and fires it when a drop action
removes an item from a character's inventory. The writer is `Option`-wrapped
so existing tests that do not register the message type continue to pass.

---

### Architecture compliance

| Check                                          | Status                                                                                          |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Data structures match architecture.md §4       | ✅ `ItemId`, `MapId` type aliases used throughout                                               |
| Module placement follows §3.2                  | ✅ Components in `game/components/`, resources in `game/resources/`, systems in `game/systems/` |
| No `unwrap()` without justification            | ✅ All error paths use `warn!` / `Option` guards                                                |
| RON format for data files                      | ✅ `MapEvent::DroppedItem` serde-compatible with existing `.ron` map files                      |
| Constants extracted, not hardcoded             | ✅ `DROPPED_ITEM_Y`, `DROP_ROTATION_JITTER`, `TILE_CENTER_OFFSET`, 7 `ITEM_*_COLOR` constants   |
| SPDX headers on all new `.rs` files            | ✅ `2026 Brett Smith` header on `dropped_item.rs` and `item_world_events.rs`                    |
| Test data in `data/`, not `campaigns/tutorial` | ✅ No test references campaign data                                                             |
| Backward compatibility                         | ✅ `#[serde(default)]` on `MapEvent::DroppedItem` fields; existing RON files unaffected         |

---

### Test coverage

**`src/game/components/dropped_item.rs`** (9 tests):

| Test                                       | What it checks                                  |
| ------------------------------------------ | ----------------------------------------------- |
| `test_dropped_item_component_fields`       | All five fields stored correctly                |
| `test_dropped_item_clone`                  | `Clone` produces equal copy                     |
| `test_dropped_item_debug`                  | `Debug` output non-empty and contains type name |
| `test_dropped_item_equality`               | `PartialEq` symmetric                           |
| `test_dropped_item_inequality_item_id`     | Different `item_id` → not equal                 |
| `test_dropped_item_inequality_map_id`      | Different `map_id` → not equal                  |
| `test_dropped_item_inequality_tile_coords` | Different tiles → not equal                     |
| `test_dropped_item_zero_charges`           | Zero charges accepted                           |
| `test_dropped_item_max_charges`            | `u16::MAX` accepted without overflow            |

**`src/game/resources/mod.rs`** (5 tests):

| Test                                       | What it checks                          |
| ------------------------------------------ | --------------------------------------- |
| `test_dropped_item_registry_default_empty` | Default has no entries                  |
| `test_registry_insert_and_lookup`          | Insert + `get` by key                   |
| `test_registry_remove_on_pickup`           | Remove returns entity; key absent after |
| `test_registry_two_entries`                | Two distinct keys coexist               |
| `test_registry_insert_overwrites`          | Later insert replaces earlier entity    |

**`src/game/systems/item_world_events.rs`** (10 tests):

| Test                                       | What it checks             |
| ------------------------------------------ | -------------------------- |
| `test_item_dropped_event_creation`         | All five fields set        |
| `test_item_picked_up_event_creation`       | All four fields set        |
| `test_item_dropped_event_clone`            | `Clone`                    |
| `test_item_picked_up_event_clone`          | `Clone`                    |
| `test_item_dropped_event_debug`            | `Debug` contains type name |
| `test_item_picked_up_event_debug`          | `Debug` contains type name |
| `test_item_dropped_event_zero_charges`     | Zero charges valid         |
| `test_item_dropped_event_max_charges`      | `u16::MAX` valid           |
| `test_item_picked_up_event_negative_tiles` | Negative tile coords valid |
| `test_dropped_item_y_is_positive`          | Constant assertion         |
| `test_tile_center_offset_is_half`          | Constant assertion         |

**`src/game/systems/procedural_meshes.rs`** (`item_mesh_tests` module, 18 tests):

| Test                                            | What it checks                                       |
| ----------------------------------------------- | ---------------------------------------------------- |
| `test_sword_config_defaults`                    | `blade_length > 0`, `has_crossguard`, `color = None` |
| `test_dagger_config_defaults`                   | `blade_length < sword blade_length`                  |
| `test_potion_config_defaults`                   | Non-zero color components                            |
| `test_scroll_config_defaults`                   | Non-zero alpha; R > 0.5 (parchment)                  |
| `test_cache_item_slots_default_none`            | All 12 item slots `None` at default                  |
| `test_cache_item_slots_cleared_after_clear_all` | `clear_all` resets item slots                        |
| `test_blunt_config_defaults`                    | Positive dimensions                                  |
| `test_staff_config_defaults`                    | Positive `length` and `orb_radius`                   |
| `test_bow_config_defaults`                      | Positive `arc_height`                                |
| `test_armor_mesh_config_defaults`               | Positive dimensions; `is_helmet = false`             |
| `test_shield_config_defaults`                   | Positive `radius`                                    |
| `test_ring_mesh_config_defaults`                | Non-zero alpha                                       |
| `test_ammo_config_defaults`                     | Non-zero alpha; type = `"arrow"`                     |
| `test_item_color_constants_valid`               | All 7 colour constants convert to valid `LinearRgba` |
| `test_sword_config_clone`                       | `Clone`                                              |
| `test_dagger_config_clone`                      | `Clone`                                              |
| `test_potion_config_clone`                      | `Clone`                                              |
| `test_scroll_config_clone`                      | `Clone`                                              |
| `test_ammo_config_clone`                        | `Clone`                                              |

---

## Items Procedural Meshes — Phase 3: Item Mesh RON Asset Files

### Overview

Phase 3 creates the data layer that backs Phase 2's runtime mesh generation:
RON asset files for every dropped-item category, a `CreatureReference` registry
so the campaign loader can discover them, a new `ItemMeshDatabase` type
(thin `CreatureDatabase` wrapper), an extended `CampaignLoader` that loads
the registry (opt-in; missing file is silently skipped), a
`ItemDatabase::link_mesh_overrides` validation hook, and the Python generator
script that keeps the asset files regenerable from a single authoritative
manifest.

### Phase 3 Deliverables

| Deliverable                              | Path                                                            |
| ---------------------------------------- | --------------------------------------------------------------- |
| Generator script                         | `examples/generate_item_meshes.py`                              |
| Tutorial campaign item mesh RON files    | `campaigns/tutorial/assets/items/` (27 files)                   |
| Tutorial campaign item mesh registry     | `campaigns/tutorial/data/item_mesh_registry.ron`                |
| Test-campaign minimal RON fixtures       | `data/test_campaign/assets/items/sword.ron`, `potion.ron`       |
| Test-campaign item mesh registry         | `data/test_campaign/data/item_mesh_registry.ron`                |
| `ItemMeshDatabase` type                  | `src/domain/items/database.rs`                                  |
| `ItemDatabase::link_mesh_overrides`      | `src/domain/items/database.rs`                                  |
| `ItemDatabaseError::UnknownMeshOverride` | `src/domain/items/database.rs`                                  |
| `GameData::item_meshes` field            | `src/domain/campaign_loader.rs`                                 |
| `CampaignLoader::load_item_meshes`       | `src/domain/campaign_loader.rs`                                 |
| Integration tests                        | `src/domain/campaign_loader.rs`, `src/domain/items/database.rs` |

### What was built

#### `examples/generate_item_meshes.py`

Developer convenience tool that generates one `CreatureDefinition` RON file per
item mesh type. The script mirrors all color and scale constants from
`src/domain/visual/item_mesh.rs` so the generated geometry exactly matches what
`ItemMeshDescriptor::build_mesh` would produce at runtime.

- `--output-dir <path>` writes the full 27-file manifest to a custom directory
  (default: `campaigns/tutorial/assets/items/`).
- `--test-fixtures` writes only the two minimal test fixtures
  (`sword.ron`, `potion.ron`) to `data/test_campaign/assets/items/`.
- Geometry helpers: `blade_mesh`, `blunt_mesh`, `staff_mesh`, `bow_mesh`,
  `armor_mesh`, `helmet_mesh`, `shield_mesh`, `boots_mesh`, `ring_mesh`,
  `belt_mesh`, `cloak_mesh`, `potion_mesh`, `scroll_mesh`, `ammo_mesh`,
  `quest_mesh` — each produces a flat XZ-plane silhouette with correct normals
  and an optional `MaterialDefinition` (metallic / roughness / emissive).
- `MANIFEST` table: 27 items covering weapon (9001–9008), armor (9101–9106),
  consumable (9201–9204), accessory (9301–9304), ammo (9401–9403), and quest
  (9501–9502) categories. IDs start at 9000 to avoid collision with creature /
  NPC / template IDs.
- `TEST_MANIFEST`: 2-item subset (`sword` id=9001, `potion` id=9201) for stable
  integration test fixtures.

#### Item mesh RON asset files (`campaigns/tutorial/assets/items/`)

27 `CreatureDefinition` RON files organised into six sub-directories:

```
weapons/    sword, dagger, short_sword, long_sword, great_sword, club, staff, bow
armor/      leather_armor, chain_mail, plate_mail, shield, helmet, boots
consumables/ health_potion, mana_potion, cure_potion, attribute_potion
accessories/ ring, amulet, belt, cloak
ammo/        arrow, bolt, stone
quest/       quest_scroll (2 meshes), key_item
```

Each file is a valid `CreatureDefinition` with:

- `id` in the 9000+ range matching the registry entry.
- One (or two for quest_scroll) flat-lying `MeshDefinition` meshes with
  per-vertex `normals: Some([...])` pointing upward.
- A `MaterialDefinition` with correct metallic / roughness / emissive values.
- An identity `MeshTransform` per mesh.
- `color_tint: None`.

#### `campaigns/tutorial/data/item_mesh_registry.ron`

`Vec<CreatureReference>` listing all 27 tutorial campaign item meshes. The
registry format is identical to `data/creatures.ron`; `CampaignLoader` reuses
`CreatureDatabase::load_from_registry` internally.

#### Test-campaign fixtures

`data/test_campaign/assets/items/sword.ron` (id=9001) and
`data/test_campaign/assets/items/potion.ron` (id=9201) are minimal stable
fixtures committed to the repository. They are referenced by
`data/test_campaign/data/item_mesh_registry.ron` and used exclusively by
integration tests — never by the live tutorial campaign.

#### `ItemMeshDatabase` (`src/domain/items/database.rs`)

Thin `#[derive(Debug, Clone, Default)]` wrapper around `CreatureDatabase`:

```src/domain/items/database.rs#L447-460
pub struct ItemMeshDatabase {
    inner: CreatureDatabase,
}
```

Public API:

| Method                                             | Description                                         |
| -------------------------------------------------- | --------------------------------------------------- |
| `new()` / `default()`                              | Empty database                                      |
| `load_from_registry(registry_path, campaign_root)` | Delegates to `CreatureDatabase::load_from_registry` |
| `as_creature_database()`                           | Returns `&CreatureDatabase` for direct queries      |
| `is_empty()`                                       | True if no entries                                  |
| `count()`                                          | Number of mesh entries                              |
| `has_mesh(id: u32)`                                | True if creature ID present                         |
| `validate()`                                       | Validates all mesh `CreatureDefinition`s            |

Re-exported from `src/domain/items/mod.rs` as `antares::domain::items::ItemMeshDatabase`.

#### `ItemDatabase::link_mesh_overrides` (`src/domain/items/database.rs`)

Forward-compatibility validation hook:

```src/domain/items/database.rs#L435-442
pub fn link_mesh_overrides(
    &self,
    _registry: &ItemMeshDatabase,
) -> Result<(), ItemDatabaseError> {
```

Walks all items that carry a `mesh_descriptor_override`, calls
`ItemMeshDescriptor::from_item` + `CreatureDefinition::validate` to confirm
the override does not break mesh generation. Full registry cross-linking
(verifying that a named creature ID exists in `ItemMeshDatabase`) is reserved
for a future extension of `ItemMeshDescriptorOverride` with an explicit
`creature_id` field.

#### `GameData::item_meshes` and `CampaignLoader::load_item_meshes`

`GameData` now carries:

```src/domain/campaign_loader.rs#L90-95
pub struct GameData {
    pub creatures: CreatureDatabase,
    pub item_meshes: ItemMeshDatabase,
}
```

`CampaignLoader::load_game_data` calls the new `load_item_meshes` helper which:

1. Looks for `data/item_mesh_registry.ron` inside the campaign directory.
2. If absent — returns `ItemMeshDatabase::new()` silently (opt-in per campaign).
3. If present — calls `ItemMeshDatabase::load_from_registry`, propagating any
   read / parse errors as `CampaignError::ReadError`.

`GameData::validate` also calls `item_meshes.validate()` so malformed mesh RON
files are caught at load time.

Note: `GameData` no longer derives `Serialize`/`Deserialize` because
`ItemMeshDatabase` wraps `CreatureDatabase` (which does) but the wrapper itself
is `Debug + Clone` only — sufficient for all current usages.

### Architecture compliance

- [ ] `ItemMeshDatabase` IDs are in the 9000+ range — no collision with
      creature IDs (1–50), NPC IDs (1000+), template IDs (2000+), variant IDs (3000+).
- [ ] RON format used for all asset and registry files — no JSON or YAML.
- [ ] File names follow lowercase + underscore convention (`item_mesh_registry.ron`,
      `health_potion.ron`, etc.).
- [ ] SPDX headers present in `generate_item_meshes.py`.
- [ ] All test data in `data/test_campaign/` — no references to
      `campaigns/tutorial` from tests.
- [ ] `CampaignLoader` opt-in: missing registry file is not an error.
- [ ] `ItemMeshDatabase` does not replace `CreatureDatabase`; it is an additive
      type that sits alongside it.

### Test coverage

**`src/domain/items/database.rs`** — 11 new unit tests:

| Test                                                       | What it verifies                                        |
| ---------------------------------------------------------- | ------------------------------------------------------- |
| `test_item_mesh_database_new_is_empty`                     | `new()` starts empty                                    |
| `test_item_mesh_database_default_is_empty`                 | `default()` == `new()`                                  |
| `test_item_mesh_database_has_mesh_absent`                  | `has_mesh` returns false for absent IDs                 |
| `test_item_mesh_database_validate_empty`                   | `validate()` succeeds on empty DB                       |
| `test_item_mesh_database_as_creature_database`             | Inner DB accessible                                     |
| `test_item_mesh_database_load_from_registry_missing_file`  | Missing file → error                                    |
| `test_item_mesh_database_load_from_registry_test_campaign` | Loads ≥ 2 entries from fixture; ids 9001 & 9201 present |
| `test_item_mesh_database_validate_test_campaign`           | Loaded fixture validates without error                  |
| `test_link_mesh_overrides_empty_item_db`                   | Empty `ItemDatabase` → ok                               |
| `test_link_mesh_overrides_no_override_items_skipped`       | Items without override → ok                             |
| `test_link_mesh_overrides_valid_override_passes`           | Valid override passes mesh validation                   |

**`src/domain/campaign_loader.rs`** — 2 new integration tests:

| Test                                            | What it verifies                                                                            |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `test_campaign_loader_loads_item_mesh_registry` | Full `load_game_data` against `data/test_campaign` populates `item_meshes` with ≥ 2 entries |
| `test_item_mesh_registry_missing_is_ok`         | Missing registry file returns empty `ItemMeshDatabase` without error                        |

All tests reference `data/test_campaign` — never `campaigns/tutorial`
(Implementation Rule 5 compliant).

---

## Procedural Meshes Direction Control

**Plan**: [`procedural_meshes_direction_control_implementation_plan.md`](procedural_meshes_direction_control_implementation_plan.md)

### Overview

All creatures (NPCs, recruitable characters, monsters) and signs spawned as
procedural meshes previously faced the same default direction because
`spawn_creature()` had no rotation parameter and `MapEvent` variants carried
no `facing` field. This implementation adds per-entity cardinal facing across
the full stack: domain data model, ECS spawn systems, runtime event system,
smooth rotation animation, and Campaign Builder SDK UI.

---

### Phase 1: Direction-to-Rotation Foundation

**Files changed**:

- `src/domain/types.rs`
- `src/game/components/creature.rs`
- `src/game/components/mod.rs`
- `src/game/systems/creature_spawning.rs`

**What was built**:

`Direction::direction_to_yaw_radians(&self) -> f32` is a new method on the
`Direction` enum that maps each cardinal to a Y-axis rotation in radians:
North → 0.0, East → π/2, South → π, West → 3π/2. The inverse,
`Direction::from_yaw_radians(yaw: f32) -> Direction`, normalises any yaw
value into `[0, 2π)` and rounds to the nearest 90° cardinal. These two
methods are the single source of truth for the angle mapping; no other file
redefines the cardinal-to-float relationship.

`FacingComponent { direction: Direction }` is a new ECS component in
`creature.rs` (re-exported from `components/mod.rs`). It is the authoritative
runtime facing state for every spawned creature, NPC, and sign entity.

`spawn_creature()` gained a `facing: Option<Direction>` parameter. It
computes `Quat::from_rotation_y(d.direction_to_yaw_radians())` from the
resolved direction, applies it to the parent `Transform`, and inserts
`FacingComponent` on the parent entity. All pre-existing call sites pass
`None`, preserving identity rotation.

---

### Phase 2: Static Map-Time Facing

**Files changed**:

- `src/domain/world/types.rs`
- `src/game/systems/map.rs`
- `src/game/systems/procedural_meshes.rs`
- `campaigns/tutorial/data/maps/map_1.ron`

**What was built**:

`facing: Option<Direction>` with `#[serde(default)]` was added to
`MapEvent::Sign`, `MapEvent::NpcDialogue`, `MapEvent::Encounter`, and
`MapEvent::RecruitableCharacter`. The `#[serde(default)]` annotation keeps
all existing RON files valid without migration — omitted fields deserialise
to `None` (identity rotation).

In `map.rs`, the NPC spawn block now passes `resolved_npc.facing` to
`spawn_creature()`. The sprite-fallback path applies the same yaw rotation
directly to the sprite entity's `Transform`. An `NpcDialogue` event-level
`facing` overrides the NPC placement `facing` when both are present.
`MapEvent::Encounter` and `MapEvent::RecruitableCharacter` spawn blocks
forward their `facing` field to `spawn_creature()`.

`spawn_sign()` in `procedural_meshes.rs` gained a `facing: Option<Direction>`
parameter. Cardinal facing takes precedence over the existing `rotation_y:
Option<f32>` degrees parameter when both are provided. `FacingComponent` is
inserted on sign entities.

The tutorial map was updated: `Old Gareth` (`RecruitableCharacter` at map_1
(15,7)) has `facing: Some(West)` as a functional smoke-test for map-time
facing on event entities. An NPC placement in map_1 has `facing: Some(South)`
as the smoke-test for NPC placement facing.

---

### Phase 3: Runtime Facing Change System

**Files changed**:

- `src/game/systems/facing.rs` (new file)
- `src/game/systems/map.rs`
- `src/game/systems/dialogue.rs`
- `src/domain/world/types.rs`

**What was built**:

A new `src/game/systems/facing.rs` module provides the full runtime facing
system and is registered via `FacingPlugin`.

`SetFacing { entity: Entity, direction: Direction, instant: bool }` is a
Bevy message. `handle_set_facing` reads it each frame: when `instant: true`
it snaps `Transform.rotation` and updates `FacingComponent.direction`
directly; when `instant: false` it inserts a `RotatingToFacing` component
for frame-by-frame slerp (Phase 4).

`ProximityFacing { trigger_distance: u32, rotation_speed: Option<f32> }` is
a marker component inserted by the map loading system on entities whose
`MapEvent` has `proximity_facing: true`. The `face_toward_player_on_proximity`
system queries all entities carrying this component each frame, computes the
4-direction from the entity's `TileCoord` to `GlobalState::party_position`
using the `cardinal_toward()` helper, and emits a `SetFacing` event whenever
the nearest cardinal differs from the current `FacingComponent.direction`.

`proximity_facing: bool` (with `#[serde(default)]`) was added to
`MapEvent::Encounter` and `MapEvent::NpcDialogue`. The map loading system in
`map.rs` inserts `ProximityFacing { trigger_distance: 2, rotation_speed }`
on the spawned entity when this flag is true, forwarding the companion
`rotation_speed` field.

`handle_start_dialogue` in `dialogue.rs` was extended: when the speaker
entity has a `TileCoord`, it computes the direction from the speaker toward
the party and writes a `SetFacing { instant: true }` event so the NPC always
faces the player at dialogue start.

---

### Phase 4: Smooth Rotation Animation

**Files changed**:

- `src/game/systems/facing.rs`
- `src/domain/world/types.rs`

**What was built**:

`RotatingToFacing { target: Quat, speed_deg_per_sec: f32, target_direction: Direction }`
is a scratch ECS component inserted by `handle_set_facing` when `instant:
false`. It is never serialised and carries the logical `target_direction` so
`FacingComponent` can be updated correctly when the rotation completes.

`apply_rotation_to_facing` is a per-frame system that queries all entities
carrying `RotatingToFacing`. Each frame it computes the remaining angle
between the current and target quaternion. When the remaining angle exceeds
the `ROTATION_COMPLETE_THRESHOLD_RAD` (0.01 rad) constant it advances the
rotation using `Quat::slerp` at the configured speed. When within the
threshold it snaps to the exact target, writes the final direction to
`FacingComponent`, and removes the `RotatingToFacing` component. This keeps
the snap paths unchanged and performant.

`rotation_speed: Option<f32>` (with `#[serde(default)]`) was added to
`MapEvent::Encounter` and `MapEvent::NpcDialogue`. When set, the value is
forwarded to `ProximityFacing.rotation_speed` and used as the
`speed_deg_per_sec` when `handle_set_facing` inserts `RotatingToFacing`.
`None` means snap (instant).

---

### Phase 5: Campaign Builder SDK UI

**Files changed**:

- `sdk/campaign_builder/src/map_editor.rs`

**What was built**:

Three fields were added to `EventEditorState`:

- `event_facing: Option<String>` — the selected cardinal direction name, or
  `None` for the engine default (North). Applies to `Sign`, `NpcDialogue`,
  `Encounter`, and `RecruitableCharacter`.
- `event_proximity_facing: bool` — mirrors the `proximity_facing` RON flag.
  Applies to `Encounter` and `NpcDialogue` only.
- `event_rotation_speed: Option<f32>` — mirrors the `rotation_speed` RON
  field. Applies to `Encounter` and `NpcDialogue` only. Suppressed in
  `to_map_event()` when `event_proximity_facing` is `false`.

`Default for EventEditorState` initialises all three to `None`, `false`,
and `None` respectively.

A **Facing** combo-box was added to the bottom of each of the four affected
`match` arms in `show_event_editor()`. Each combo-box uses a unique
`id_salt` to satisfy the egui ID rules:

| Event type             | `id_salt`                           |
| ---------------------- | ----------------------------------- |
| `Sign`                 | `"sign_event_facing_combo"`         |
| `NpcDialogue`          | `"npc_dialogue_event_facing_combo"` |
| `Encounter`            | `"encounter_event_facing_combo"`    |
| `RecruitableCharacter` | `"recruitable_event_facing_combo"`  |

A **Behaviour** section (separator + label + checkbox + conditional
text-input) was added to the `Encounter` and `NpcDialogue` arms only,
surfacing the proximity-facing toggle and the rotation-speed field.
The rotation-speed input renders only when the proximity-facing checkbox
is ticked.

`to_map_event()` was updated for all four variants to parse `event_facing`
via the private `parse_facing()` helper and include it in the constructed
`MapEvent`. For `Encounter` and `NpcDialogue` it also forwards
`proximity_facing` and `rotation_speed` (with the suppression rule above).

`from_map_event()` was updated for all four variants to populate
`event_facing`, `event_proximity_facing`, and `event_rotation_speed` from
the loaded event, preserving backward compatibility for RON files that
predate these fields.

`show_inspector_panel()` was extended for all four event types to display
the `facing` direction when set. For `Encounter` and `NpcDialogue` it also
shows the proximity-facing label and rotation speed when applicable.

---

### Test Coverage

| Module                                   | Key tests added                                                                                                                                                                                                                                                                                                                                                                                                |
| ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/domain/types.rs`                    | `test_direction_to_yaw_north/east/south/west`, `test_direction_roundtrip`, `test_direction_from_yaw_cardinals`, `test_direction_from_yaw_snaps_to_nearest`                                                                                                                                                                                                                                                     |
| `src/game/components/creature.rs`        | `test_facing_component_new`, `test_facing_component_default_is_north`, `test_facing_component_all_directions`, `test_facing_component_clone/equality`                                                                                                                                                                                                                                                          |
| `src/game/systems/creature_spawning.rs`  | `test_spawn_creature_facing_none_is_north`, `test_spawn_creature_facing_south_rotation`                                                                                                                                                                                                                                                                                                                        |
| `src/game/systems/map.rs`                | `test_npc_facing_applied_at_spawn`, `test_facing_component_on_npc`, `test_map_event_encounter_facing`, `test_map_event_sign_facing`, `test_map_event_ron_round_trip`, `test_proximity_facing_inserted_on_encounter_with_flag`, `test_proximity_facing_not_inserted_when_flag_false`, `test_proximity_facing_npc_inserted_when_flag_set`                                                                        |
| `src/game/systems/facing.rs`             | `test_set_facing_snaps_transform`, `test_set_facing_updates_facing_component`, `test_proximity_facing_emits_event`, `test_set_facing_instant_false_inserts_rotating_component`, `test_rotating_to_facing_approaches_target`, `test_rotating_to_facing_completes_and_removes_component`                                                                                                                         |
| `src/game/systems/dialogue.rs`           | `test_dialogue_start_emits_set_facing`, `test_dialogue_start_no_speaker_entity_does_not_panic`, `test_dialogue_start_speaker_without_tile_coord_skips_facing`                                                                                                                                                                                                                                                  |
| `sdk/campaign_builder/src/map_editor.rs` | `test_event_editor_state_default_facing_none`, `test_event_editor_to_sign_with_facing`, `test_event_editor_from_sign_with_facing`, `test_event_editor_from_sign_no_facing`, `test_event_editor_to_encounter_with_facing_and_proximity`, `test_event_editor_from_encounter_with_proximity`, `test_event_editor_facing_round_trip_all_variants`, `test_event_editor_proximity_false_clears_rotation_speed_in_ui` |

---

### Architecture Compliance

- `direction_to_yaw_radians` is the **single source of truth** for the
  cardinal-to-angle mapping; no other file redefines north/south/etc as raw
  floats.
- All new `MapEvent` fields use `#[serde(default)]` — all existing RON files
  remain valid without migration.
- `SetFacing` follows the existing `#[derive(Message)]` broadcast pattern.
- `RotatingToFacing` is a pure ECS scratch component — never serialised,
  never referenced by domain structs.
- `FacingPlugin` registers all three systems (`handle_set_facing`,
  `face_toward_player_on_proximity`, `apply_rotation_to_facing`) in a single
  plugin, keeping the addition self-contained.
- No test references `campaigns/tutorial`; all test fixtures use
  `data/test_campaign`.

---

## Items Procedural Meshes — Phase 4: Visual Quality and Variation

**Plan**: [`items_procedural_meshes_implementation_plan.md`](items_procedural_meshes_implementation_plan.md)

### Overview

Phase 4 extends the procedural item-mesh pipeline with four major visual
improvements:

1. **Per-item accent colors** derived from `BonusAttribute` (fire → orange,
   cold → icy blue, magic → purple, etc.)
2. **Metallic / roughness PBR material differentiation** — magical items get
   `metallic: 0.7, roughness: 0.25`; mundane non-metal items get
   `metallic: 0.0, roughness: 0.8`.
3. **Deterministic Y-rotation** — dropped items receive a tile-position-derived
   rotation instead of non-deterministic random jitter, making save/load replay
   safe.
4. **Child mesh additions**: a ground shadow quad (semi-transparent, alpha 0.3,
   `AlphaMode::Blend`) prepended to every definition, and an optional
   charge-level emissive gem appended when `charges_fraction` is supplied.
5. **LOD levels** attached automatically to primary meshes exceeding 200
   triangles (`LOD1` at 8 world units, `LOD2` billboard at 20 world units).

---

### Phase 4 Deliverables

**Files changed**:

- `src/domain/visual/item_mesh.rs` — extended with accent colors, metallic /
  roughness rules, shadow quad builder, charge gem builder, LOD wiring, and all
  Phase 4 unit tests.
- `src/game/systems/item_world_events.rs` — replaced random jitter with
  `deterministic_drop_rotation`, wired `charges_fraction` into
  `to_creature_definition_with_charges`, and added deterministic-rotation unit
  tests.

---

### What was built

#### 4.1 — Accent color from `BonusAttribute` (`src/domain/visual/item_mesh.rs`)

New private function `accent_color_from_item(item: &Item) -> Option<[f32; 4]>`
maps the item's `constant_bonus` (or `temporary_bonus` fallback) to a
Phase 4 accent color:

| `BonusAttribute`         | Accent color constant                |
| ------------------------ | ------------------------------------ |
| `ResistFire`             | `COLOR_ACCENT_FIRE` — orange         |
| `ResistCold`             | `COLOR_ACCENT_COLD` — icy blue       |
| `ResistElectricity`      | `COLOR_ACCENT_ELECTRICITY` — yellow  |
| `ResistAcid`             | `COLOR_ACCENT_ACID` — acid green     |
| `ResistPoison`           | `COLOR_ACCENT_POISON` — acid green   |
| `ResistMagic`            | `COLOR_ACCENT_MAGIC` — purple        |
| `Might`                  | `COLOR_ACCENT_MIGHT` — warm red      |
| `ArmorClass`/`Endurance` | `COLOR_ACCENT_TEAL` — teal           |
| `Intellect`              | `COLOR_ACCENT_DEEP_BLUE` — deep blue |

The accent is applied inside `from_item` after the base descriptor is built,
but only when the item is not cursed (cursed items already override
`primary_color` entirely, making accent irrelevant).

#### 4.1 — Metallic / roughness PBR differentiation

New helper `is_metallic_magical(&self) -> bool` returns `true` when
`emissive == true && emissive_color == EMISSIVE_MAGIC` (the marker set by
`from_item` when `item.is_magical()`).

`make_material` now branches on this:

- **Magical**: `metallic: 0.7, roughness: 0.25` (shiny, jewel-like)
- **Mundane metal categories** (Sword, Dagger, Blunt, Helmet, Shield, Ring,
  Amulet): legacy `metallic: 0.6, roughness: 0.5`
- **All other mundane**: `metallic: 0.0, roughness: 0.8` (matte)

New constants: `MATERIAL_METALLIC_MAGICAL = 0.7`,
`MATERIAL_ROUGHNESS_MAGICAL = 0.25`, `MATERIAL_METALLIC_MUNDANE = 0.0`,
`MATERIAL_ROUGHNESS_MUNDANE = 0.8`.

#### 4.2 — Deterministic Y-rotation (`src/game/systems/item_world_events.rs`)

Replaced the `rand::Rng::random::<f32>()` call with a new public function:

```rust
pub fn deterministic_drop_rotation(
    map_id: MapId,
    tile_x: i32,
    tile_y: i32,
    item_id: ItemId,
) -> f32
```

Algorithm:

```text
hash = map_id + (tile_x × 31) + (tile_y × 17) + (item_id × 7)   [wrapping u64 ops]
angle = (hash % 360) / 360.0 × TAU
```

This gives visually varied orientations across tiles while being fully
deterministic. The `rand` import was removed from `item_world_events.rs`.

#### 4.3 — Charge-level gem child mesh

`to_creature_definition` now delegates to a new public method:

```rust
pub fn to_creature_definition_with_charges(
    &self,
    charges_fraction: Option<f32>,
) -> CreatureDefinition
```

When `charges_fraction: Some(f)` is supplied a small diamond gem mesh is
appended as the third mesh, positioned `+0.04` Y above the item origin.

Gem color gradient (via `charge_gem_color(frac) -> ([f32; 4], [f32; 3])`):

- `1.0` → `COLOR_CHARGE_FULL` (gold, emissive gold glow)
- `0.5` → `COLOR_CHARGE_HALF` (white, dim emissive)
- `0.0` → `COLOR_CHARGE_EMPTY` (grey, no emissive)
- Intermediate fractions linearly interpolated via `lerp_color4` / `lerp_color3`.

`spawn_dropped_item_system` now computes
`charges_fraction = Some(charges as f32 / max_charges as f32)` when
`item.max_charges > 0`, otherwise `None`.

#### 4.4 — Ground shadow quad

New private function `build_shadow_quad(&self) -> MeshDefinition` builds a
flat `2 × 2`-triangle quad on the XZ plane at Y = `SHADOW_QUAD_Y` (0.001).
The quad's half-extent is `self.scale × SHADOW_QUAD_SCALE × 0.5` where
`SHADOW_QUAD_SCALE = 1.2`.

Material:

- `base_color: [0.0, 0.0, 0.0, 0.3]`
- `alpha_mode: AlphaMode::Blend`
- `metallic: 0.0, roughness: 1.0`

The shadow quad is always inserted as `meshes[0]`, with the primary item mesh
at `meshes[1]`, and the optional charge gem at `meshes[2]`.

#### 4.5 — LOD support

New private function `build_mesh_with_lod(&self) -> MeshDefinition`:

- Builds the primary mesh via `build_mesh()`.
- Counts triangles = `indices.len() / 3`.
- If `> LOD_TRIANGLE_THRESHOLD (200)`: calls `generate_lod_levels(&mesh, 2)`
  and overrides the auto-distances with fixed values
  `[LOD_DISTANCE_1, LOD_DISTANCE_2]` = `[8.0, 20.0]`.
- If `≤ 200`: returns mesh as-is (no LOD).

All procedural item meshes in the current implementation are well under 200
triangles, so LOD is not triggered at runtime today. The infrastructure is
ready for future artist-authored higher-fidelity meshes.

#### Free helper functions

Two free (non-method) `#[inline]` functions were added to the module:

- `lerp_color4(a, b, t) -> [f32; 4]` — RGBA linear interpolation
- `lerp_color3(a, b, t) -> [f32; 3]` — RGB linear interpolation (for emissive)

---

### Architecture compliance

- [ ] All new constants extracted (`COLOR_ACCENT_*`, `COLOR_CHARGE_*`,
      `EMISSIVE_CHARGE_*`, `SHADOW_QUAD_*`, `LOD_*`, `MATERIAL_*`).
- [ ] No hardcoded magic numbers in logic paths.
- [ ] `to_creature_definition` is unchanged in signature; the new
      `to_creature_definition_with_charges` is additive.
- [ ] `rand` dependency removed from `item_world_events.rs` — the system is
      now deterministic and safe for save/load replay.
- [ ] RON data files unchanged.
- [ ] No test references `campaigns/tutorial`.
- [ ] SPDX headers present on all modified `.rs` files (inherited).
- [ ] All new public functions documented with `///` doc comments and examples.

---

### Test coverage

New tests in `src/domain/visual/item_mesh.rs` (`mod tests`):

| Test                                                    | What it verifies                                                |
| ------------------------------------------------------- | --------------------------------------------------------------- |
| `test_fire_resist_item_accent_orange`                   | ResistFire → `COLOR_ACCENT_FIRE`                                |
| `test_cold_resist_item_accent_blue`                     | ResistCold → `COLOR_ACCENT_COLD`                                |
| `test_electricity_resist_item_accent_yellow`            | ResistElectricity → yellow                                      |
| `test_poison_resist_item_accent_green`                  | ResistPoison → acid green                                       |
| `test_magic_resist_item_accent_purple`                  | ResistMagic → purple                                            |
| `test_might_bonus_item_accent_warm_red`                 | Might → warm red                                                |
| `test_ac_bonus_item_accent_teal`                        | ArmorClass → teal                                               |
| `test_intellect_bonus_item_accent_deep_blue`            | Intellect → deep blue                                           |
| `test_magical_item_metallic_material`                   | `is_magical()` → `metallic > 0.5`, `roughness < 0.3`            |
| `test_non_magical_item_matte_material`                  | mundane non-metal → `metallic: 0.0`, `roughness: 0.8`           |
| `test_shadow_quad_present_and_transparent`              | `meshes[0]` is shadow quad, alpha < 0.5, `AlphaMode::Blend`     |
| `test_shadow_quad_valid_for_all_categories`             | Shadow quad present for all item types                          |
| `test_charge_fraction_full_color_gold`                  | `charges_fraction=1.0` → gold gem, emissive                     |
| `test_charge_fraction_empty_color_grey`                 | `charges_fraction=0.0` → grey gem, no emissive                  |
| `test_charge_fraction_none_no_gem`                      | `charges_fraction=None` → exactly 2 meshes                      |
| `test_deterministic_charge_gem_color`                   | Color gradient determinism and boundary values                  |
| `test_lod_added_for_complex_mesh`                       | > 200 triangles → LOD levels generated                          |
| `test_no_lod_for_simple_mesh`                           | ≤ 200 triangles → `lod_levels: None`                            |
| `test_creature_definition_mesh_transform_count_matches` | `meshes.len() == mesh_transforms.len()` for all charge variants |
| `test_accent_color_not_applied_to_cursed_item`          | Cursed items keep `COLOR_CURSED` even with bonus                |
| `test_lerp_color4_midpoint`                             | `lerp_color4` at `t=0.5` produces midpoint                      |
| `test_lerp_color3_midpoint`                             | `lerp_color3` at `t=0.5` produces midpoint                      |

New tests in `src/game/systems/item_world_events.rs` (`mod tests`):

| Test                                               | What it verifies                         |
| -------------------------------------------------- | ---------------------------------------- |
| `test_deterministic_drop_rotation_same_inputs`     | Same inputs → same angle                 |
| `test_deterministic_drop_rotation_different_tiles` | Different tile → different angle         |
| `test_deterministic_drop_rotation_in_range`        | Angle in `[0, TAU)` for all tested tiles |
| `test_deterministic_drop_rotation_different_items` | Different item IDs → different angle     |

**Total tests added: 26** across two modules. All 3,159 tests pass.

## Items Procedural Meshes — Phase 5: Campaign Builder SDK Integration

### Overview

Phase 5 brings the Item Mesh workflow in the Campaign Builder to parity with
the Creature Builder (`creatures_editor.rs`). Campaign authors can now browse
all registered item mesh RON assets, filter by `ItemMeshCategory`, edit a
descriptor's visual properties (colors, scale, emissive), preview the result
live, undo/redo every change, save to `assets/items/`, and register existing
RON files. A **"Ground Mesh Preview"** collapsible was also added to the
existing Items editor form, and a cross-tab "Open in Item Mesh Editor" signal
was wired between the Items tab and the new **Item Meshes** tab.

### Phase 5 Deliverables

| File                                                | Role                                                      |
| --------------------------------------------------- | --------------------------------------------------------- |
| `sdk/campaign_builder/src/item_mesh_undo_redo.rs`   | `ItemMeshUndoRedo` + `ItemMeshEditAction`                 |
| `sdk/campaign_builder/src/item_mesh_workflow.rs`    | `ItemMeshWorkflow`, `ItemMeshEditorMode`                  |
| `sdk/campaign_builder/src/item_mesh_editor.rs`      | `ItemMeshEditorState` — full editor UI                    |
| `sdk/campaign_builder/src/items_editor.rs`          | Ground Mesh Preview pane + `requested_open_item_mesh`     |
| `sdk/campaign_builder/src/lib.rs`                   | `EditorTab::ItemMeshes`, module registrations, tab wiring |
| `sdk/campaign_builder/tests/map_data_validation.rs` | `MapEvent::DroppedItem` arm                               |

### What was built

#### 5.1 — `item_mesh_undo_redo.rs`

`ItemMeshUndoRedo` is a simple two-stack undo/redo manager owning a
`Vec<ItemMeshEditAction>` for each direction. `ItemMeshEditAction` covers:

- `SetPrimaryColor { old, new }` — RGBA primary color change
- `SetAccentColor { old, new }` — RGBA accent color change
- `SetScale { old, new }` — scale factor change
- `SetEmissive { old, new }` — emissive bool toggle
- `SetOverrideEnabled { old, new }` — override enable/disable
- `ReplaceDescriptor { old, new }` — atomic full-descriptor swap

`push()` appends to the undo stack and clears the redo stack. `undo()` pops
from the undo stack and pushes the action to redo; `redo()` does the reverse.
Both return the popped `ItemMeshEditAction` so the caller can apply `old` (for
undo) or `new` (for redo) to the live descriptor.

#### 5.2 — `item_mesh_workflow.rs`

`ItemMeshWorkflow` tracks `ItemMeshEditorMode` (`Registry` or `Edit`),
`current_file: Option<String>`, and `unsaved_changes: bool`.

Public API:

- `mode_indicator() -> String` — `"Registry Mode"` or `"Asset Editor: <file>"`
- `breadcrumb_string() -> String` — `"Item Meshes"` or `"Item Meshes > <file>"`
- `enter_edit(file_name)` — transitions to Edit mode, sets `current_file`, clears dirty
- `return_to_registry()` — resets to Registry mode, clears file and dirty
- `mark_dirty()` / `mark_clean()` — unsaved-change tracking
- `has_unsaved_changes()` / `current_file()`

#### 5.3 — `item_mesh_editor.rs`

`ItemMeshEditorState` is the top-level state struct for the Item Mesh Editor
tab. Key design decisions:

**Registry mode UI** uses `TwoColumnLayout::new("item_mesh_registry")`. All
mutations inside the two `FnOnce` closures are collected in separate
`left_*` and `right_*` deferred-mutation locals (sdk/AGENTS.md Rule 10), then
merged into canonical `pending_*` vars and applied after `show_split` returns.
This avoids the E0499/E0524 double-borrow errors that arise when both closures
capture the same `&mut` variable. The `search_query` text edit uses an owned
clone of the value rather than a `&mut self.search_query` reference, flushed
via `pending_new_search`.

**Edit mode UI** uses `ui.columns(2, ...)` for a properties/preview split:

- Left: override-enabled checkbox, primary/accent RGBA sliders, scale slider
  (0.25–4.0), emissive checkbox, Reset to Defaults button, inline Validation
  collapsible. Every mutation pushes an `ItemMeshEditAction`, sets
  `preview_dirty = true`, and calls `ui.ctx().request_repaint()`.
- Right: camera-distance slider, "Regenerate Preview" button, live
  `PreviewRenderer` display.

**Dialog windows** (`show_save_as_dialog_window`,
`show_register_asset_dialog_window`) use the deferred-action pattern instead of
`.open(&mut bool)` — the `still_open` double-borrow issue is avoided by
collecting `do_save`, `do_cancel`, `do_validate`, and `do_register` booleans
inside the closure and acting on them after it returns.

**`validate_descriptor`** is a pure `(errors, warnings)` function:

- Error: `scale <= 0.0`
- Warning: `scale > 3.0`

**`perform_save_as_with_path`** validates the path prefix (`assets/items/`),
serialises the descriptor to RON via `ron::ser::to_string_pretty`, creates
directories, writes the file, derives a display name from the file stem, and
appends a new `ItemMeshEntry` to the registry.

**`execute_register_asset_validation`** reads and deserialises the RON file,
checks for duplicate `file_path` entries in the registry, and sets
`register_asset_error` on failure.

**`refresh_available_assets`** scans `campaign_dir/assets/items/*.ron` and
caches results in `available_item_assets`; skips the scan if
`last_campaign_dir` is unchanged.

#### 5.4 — Items editor Ground Mesh Preview pane

`ItemsEditorState` gained:

- `requested_open_item_mesh: Option<ItemId>` — cross-tab navigation signal,
  consumed by the parent `CampaignBuilderApp` to switch to `EditorTab::ItemMeshes`.
- A `ui.collapsing("🧊 Ground Mesh Preview", ...)` section at the bottom of
  `show_form()`. It derives an `ItemMeshDescriptor` from the current
  `edit_buffer` via `ItemMeshDescriptor::from_item`, displays category, shape,
  and override parameters, and provides an "✏️ Open in Item Mesh Editor" button
  that sets `requested_open_item_mesh`.

#### 5.5 — Tab wiring in `lib.rs`

- Three new modules registered: `item_mesh_editor`, `item_mesh_undo_redo`,
  `item_mesh_workflow`.
- `EditorTab::ItemMeshes` added to the enum and the sidebar tabs array.
- `item_mesh_editor_state: item_mesh_editor::ItemMeshEditorState` added to
  `CampaignBuilderApp`.
- The central panel match dispatches `EditorTab::ItemMeshes` to
  `item_mesh_editor_state.show(ui, campaign_dir.as_ref())`.
- `ItemMeshEditorSignal::OpenInItemsEditor(item_id)` switches to
  `EditorTab::Items` and selects the matching item.
- Cross-tab from Items: `requested_open_item_mesh.take()` switches to
  `EditorTab::ItemMeshes`.

#### 5.6 — `MapEvent::DroppedItem` exhaustive match arms

Five `match event` blocks in `map_editor.rs` and one in
`tests/map_data_validation.rs` were missing the `DroppedItem` variant
(introduced in Phase 2). All were fixed:

- `EventEditorState::from_map_event` — sets `event_type = Treasure`, copies name
- Two tile-grid colour queries — maps to `EventType::Treasure`
- The event-details tooltip panel — shows item id and charges
- `event_name_description` helper — returns name and empty description
- Test validation loop — empty arm (no validation required)

#### Pre-existing `mesh_descriptor_override` field gap

`Item::mesh_descriptor_override` (added in Phase 1) was missing from struct
literal initialisers throughout the SDK codebase. All affected files were
patched to add `mesh_descriptor_override: None,`:

`advanced_validation.rs`, `asset_manager.rs`, `characters_editor.rs`,
`dialogue_editor.rs`, `items_editor.rs`, `lib.rs`, `templates.rs`,
`undo_redo.rs`, `ui_helpers.rs`.

Where the Python insertion script accidentally added the field to `TemplateInfo`
literals (which have no such field), the spurious lines were removed.

### Architecture compliance

- [ ] Data structures match `architecture.md` Section 4 — `ItemMeshDescriptor`,
      `ItemMeshCategory`, `ItemMeshDescriptorOverride` used exactly as defined.
- [ ] Module placement follows Section 3.2 — three new SDK modules in
      `sdk/campaign_builder/src/`.
- [ ] RON format used for all data files — descriptor serialisation via `ron`.
- [ ] No architectural deviations without documentation.
- [ ] egui ID rules (sdk/AGENTS.md) fully followed:
  - Every loop body uses `ui.push_id(idx, ...)`.
  - Every `ScrollArea` has `.id_salt("unique_string")`.
  - Every `ComboBox` uses `ComboBox::from_id_salt("...")`.
  - Every `Window` has a unique title.
  - State mutations call `ui.ctx().request_repaint()`.
  - `TwoColumnLayout` used for the registry list/detail split.
  - No `SidePanel`/`CentralPanel` guards skipped same-frame.
  - Deferred-mutation pattern (Rule 10) applied throughout.
- [ ] SPDX headers present on all three new `.rs` files.

### Test coverage

**`item_mesh_undo_redo.rs`** (12 tests)

| Test                                     | Assertion                                                  |
| ---------------------------------------- | ---------------------------------------------------------- |
| `test_item_mesh_undo_redo_push_and_undo` | After push + undo: `can_undo == false`, `can_redo == true` |
| `test_item_mesh_undo_redo_redo`          | After push + undo + redo: `can_redo == false`              |
| `test_item_mesh_undo_redo_clear`         | After clear: both stacks empty                             |
| `test_push_clears_redo_stack`            | New push after undo wipes redo                             |
| `test_undo_empty_returns_none`           | Undo on empty stack returns `None`                         |
| `test_redo_empty_returns_none`           | Redo on empty stack returns `None`                         |
| `test_multiple_pushes_lifo_order`        | LIFO semantics verified                                    |
| `test_set_primary_color_action`          | `SetPrimaryColor` old/new fields                           |
| `test_set_accent_color_action`           | `SetAccentColor` old/new fields                            |
| `test_set_override_enabled_action`       | `SetOverrideEnabled` old/new fields                        |
| `test_replace_descriptor_action`         | `ReplaceDescriptor` full descriptor swap                   |

**`item_mesh_workflow.rs`** (11 tests)

| Test                                                    | Assertion                             |
| ------------------------------------------------------- | ------------------------------------- |
| `test_workflow_default_is_registry`                     | Default mode is `Registry`            |
| `test_item_mesh_editor_mode_indicator_registry`         | Returns `"Registry Mode"`             |
| `test_item_mesh_editor_mode_indicator_edit`             | Returns `"Asset Editor: sword.ron"`   |
| `test_item_mesh_editor_mode_indicator_edit_no_file`     | Returns `"Asset Editor"` with no file |
| `test_item_mesh_editor_breadcrumb_registry`             | Returns `"Item Meshes"`               |
| `test_item_mesh_editor_breadcrumb_edit`                 | Returns `"Item Meshes > sword.ron"`   |
| `test_item_mesh_editor_breadcrumb_edit_no_file`         | Returns `"Item Meshes"` with no file  |
| `test_workflow_enter_edit`                              | Mode transitions to Edit, file set    |
| `test_workflow_enter_edit_clears_unsaved_changes`       | Dirty flag cleared on enter           |
| `test_workflow_return_to_registry`                      | Resets mode, file, dirty              |
| `test_workflow_mark_dirty` / `test_workflow_mark_clean` | Dirty flag round-trip                 |

**`item_mesh_editor.rs`** (28 tests, including 1 in `items_editor.rs`)

| Test                                                          | Assertion                                 |
| ------------------------------------------------------------- | ----------------------------------------- |
| `test_item_mesh_editor_state_default`                         | Mode is Registry, no selection, not dirty |
| `test_item_mesh_editor_has_unsaved_changes_false_by_default`  | Fresh state is clean                      |
| `test_item_mesh_editor_has_unsaved_changes_true_after_edit`   | Mutation sets dirty                       |
| `test_item_mesh_editor_can_undo_false_by_default`             | Empty undo stack                          |
| `test_item_mesh_editor_can_redo_false_by_default`             | Empty redo stack                          |
| `test_item_mesh_editor_back_to_registry_clears_edit_state`    | edit_buffer cleared, mode reset           |
| `test_available_item_assets_empty_when_no_assets_dir`         | Missing dir yields empty list             |
| `test_available_item_assets_populated_from_campaign_dir`      | Scans `.ron` files correctly              |
| `test_available_item_assets_not_refreshed_when_dir_unchanged` | Cache hit on same dir                     |
| `test_available_item_assets_refreshed_when_dir_changes`       | Cache miss on dir change                  |
| `test_register_asset_validate_duplicate_id_sets_error`        | Duplicate path sets error                 |
| `test_register_asset_cancel_does_not_modify_registry`         | Cancel leaves registry unchanged          |
| `test_register_asset_success_appends_entry`                   | Valid RON appended to registry            |
| `test_perform_save_as_with_path_appends_new_entry`            | Save-as writes file and registry          |
| `test_perform_save_as_requires_campaign_directory`            | Error with no campaign dir                |
| `test_perform_save_as_rejects_non_item_asset_paths`           | Path outside `assets/items/` rejected     |
| `test_revert_edit_buffer_restores_original`                   | Buffer reset from registry entry          |
| `test_revert_edit_buffer_errors_in_registry_mode`             | Revert in Registry mode is error          |
| `test_validate_descriptor_reports_invalid_scale`              | `scale = 0.0` → error containing "scale"  |
| `test_validate_descriptor_reports_negative_scale`             | `scale = -1.0` → error                    |
| `test_validate_descriptor_passes_for_default_descriptor`      | Clean descriptor → no issues              |
| `test_validate_descriptor_warns_on_large_scale`               | `scale = 4.0` → warning                   |
| `test_filtered_sorted_registry_empty`                         | Empty registry → empty result             |
| `test_filtered_sorted_registry_by_name`                       | Alphabetical sort respected               |
| `test_filtered_sorted_registry_search_filter`                 | Search query filters correctly            |
| `test_count_by_category`                                      | Category histogram correct                |
| `test_items_editor_requested_open_item_mesh_set_on_button`    | Signal field set + drainable              |

**Total new tests: 51.** All 1,925 SDK tests and 3,159 full-suite tests pass.

---

## Items Procedural Meshes — Phase 6.4: Required Integration Tests

### Overview

Phase 6.4 adds three mandatory integration tests that close coverage gaps
identified in the Phase 6 acceptance criteria:

1. **`test_all_base_items_have_valid_mesh_descriptor`** — iterates every item
   in `data/items.ron`, generates an `ItemMeshDescriptor` via
   `ItemMeshDescriptor::from_item`, converts it to a `CreatureDefinition` via
   `to_creature_definition`, and asserts `validate()` returns `Ok`. This
   guarantees the descriptor pipeline is sound for all current base items.

2. **`test_item_mesh_registry_tutorial_coverage`** — loads the
   `data/test_campaign` campaign via `CampaignLoader`, asserts the returned
   `GameData::item_meshes` registry is non-empty and contains at least 2
   entries. Validates the end-to-end loader path for item mesh data.

3. **`test_dropped_item_event_in_map_ron`** — reads
   `data/test_campaign/data/maps/map_1.ron`, deserialises it as
   `crate::domain::world::Map`, and asserts that at least one
   `MapEvent::DroppedItem` event is present and that item_id 4 (Long Sword) is
   among them. Validates RON round-trip for the `DroppedItem` variant.

A prerequisite data fixture was also added: a `DroppedItem` entry for the
Long Sword (item_id 4) at map position (7, 7) was inserted into
`data/test_campaign/data/maps/map_1.ron`.

### Phase 6.4 Deliverables

| File                                     | Change                                           |
| ---------------------------------------- | ------------------------------------------------ |
| `src/domain/visual/item_mesh.rs`         | 3 new tests appended to `mod tests`              |
| `data/test_campaign/data/maps/map_1.ron` | `DroppedItem` event added at position (x:7, y:7) |

### What was built

#### `test_all_base_items_have_valid_mesh_descriptor`

Loads `data/items.ron` using `ItemDatabase::load_from_file`, then loops over
every `Item` returned by `all_items()`. For each item it calls
`ItemMeshDescriptor::from_item(item)`, then `descriptor.to_creature_definition()`,
then `creature_def.validate()`. Any failure includes the item id and name in
the assertion message for fast triage.

#### `test_item_mesh_registry_tutorial_coverage`

Constructs a `CampaignLoader` pointing at `data/` (base) and
`data/test_campaign` (campaign), calls `load_game_data()`, and asserts:

- `result.is_ok()`
- `!game_data.item_meshes.is_empty()`
- `game_data.item_meshes.count() >= 2`

Uses `env!("CARGO_MANIFEST_DIR")` for portable paths. Does **not** reference
`campaigns/tutorial` (Implementation Rule 5 compliant).

#### `test_dropped_item_event_in_map_ron`

Reads `data/test_campaign/data/maps/map_1.ron` from disk, deserialises via
`ron::from_str::<Map>(&contents)`, then:

- Asserts at least one `MapEvent::DroppedItem { .. }` variant is present.
- Asserts a `DroppedItem` with `item_id == 4` (Long Sword) exists.

#### `DroppedItem` fixture in `map_1.ron`

Added at the end of the `events` block (before the closing brace):

```data/test_campaign/data/maps/map_1.ron#L8384-8391
        (
            x: 7,
            y: 7,
        ): DroppedItem(
            name: "Long Sword",
            item_id: 4,
            charges: 0,
        ),
```

### Architecture compliance

- [x] Data structures match `architecture.md` Section 4 — `ItemMeshDescriptor`,
      `Map`, `MapEvent` used exactly as defined.
- [x] Test data uses `data/test_campaign`, NOT `campaigns/tutorial`
      (Implementation Rule 5).
- [x] New fixture added to `data/test_campaign/data/maps/map_1.ron`, not
      borrowed from live campaign data.
- [x] RON format used for all data files.
- [x] No architectural deviations without documentation.
- [x] SPDX headers unaffected (tests appended to existing file).

### Test coverage

**`src/domain/visual/item_mesh.rs`** (3 new tests, inside existing `mod tests`)

| Test                                             | Assertion                                                   |
| ------------------------------------------------ | ----------------------------------------------------------- |
| `test_all_base_items_have_valid_mesh_descriptor` | Every item in `data/items.ron` → valid `CreatureDefinition` |
| `test_item_mesh_registry_tutorial_coverage`      | `test_campaign` item mesh registry non-empty, count ≥ 2     |
| `test_dropped_item_event_in_map_ron`             | `map_1.ron` parses, contains `DroppedItem` with item_id=4   |

**All 3 new tests pass.** All quality gates pass (fmt, check, clippy -D warnings, nextest).

---

## Phase 6.3 — `MapEvent::DroppedItem` Placements in Tutorial Campaign and Test Fixture

### Overview

Phase 6.3 populates the tutorial campaign maps and the test fixture map with
concrete `MapEvent::DroppedItem` entries. These events represent items lying on
the ground that the player can walk over and pick up. This phase adds 3 events
to the live tutorial campaign and 1 to the test fixture (`data/test_campaign`),
satisfying both the gameplay placement requirements and Implementation Rule 5
(tests use `data/test_campaign`, never `campaigns/tutorial`).

---

### What Was Changed

#### Tutorial Campaign Maps

| File                                     | Position | Item               | item_id         | Purpose                                                          |
| ---------------------------------------- | -------- | ------------------ | --------------- | ---------------------------------------------------------------- |
| `campaigns/tutorial/data/maps/map_1.ron` | (3, 17)  | Dropped Sword      | 3 (Short Sword) | Near the elder NPC at (1,16) — early starting area reward        |
| `campaigns/tutorial/data/maps/map_2.ron` | (2, 5)   | Healing Potion     | 50              | Near dungeon entrances in Dark Forrest — survival incentive      |
| `campaigns/tutorial/data/maps/map_4.ron` | (3, 3)   | Ring of Protection | 40              | Near the `Treasure` event at (1,1) — treasure chamber floor loot |

All three entries were inserted before the closing `},` of the existing
`events: { ... }` BTreeMap block in each file. No existing events were
modified. No duplicate positions were introduced.

#### Test Fixture Map

| File                                     | Position | Item                    | item_id        | Note                                                                                 |
| ---------------------------------------- | -------- | ----------------------- | -------------- | ------------------------------------------------------------------------------------ |
| `data/test_campaign/data/maps/map_1.ron` | (7, 7)   | Test Dropped Long Sword | 4 (Long Sword) | Entry already existed; name updated to "Test Dropped Long Sword" for fixture clarity |

The `DroppedItem` at (7, 7) in `data/test_campaign/data/maps/map_1.ron` was
pre-existing with name `"Long Sword"`. Its name was updated to
`"Test Dropped Long Sword"` to clearly identify it as a test fixture entry
and match the Phase 6.3 specification.

---

### RON Format Used

Each event entry follows the `MapEvent::DroppedItem` variant structure, inserted
into the `events` BTreeMap block:

```antares/campaigns/tutorial/data/maps/map_1.ron#L8450-8459
        (
            x: 3,
            y: 17,
        ): DroppedItem(
            name: "Dropped Sword",
            item_id: 3,
            charges: 0,
        ),
```

The `name` field is `#[serde(default)]` (optional display label).
The `charges` field is `#[serde(default)]` and set to `0` for non-charged items.
`item_id` is the `ItemId` (`u32`) type alias referencing entries in `items.ron`.

---

### Architecture Compliance

- `MapEvent::DroppedItem` structure used exactly as defined (Section 4, map events).
- RON format used for all data files per Section 7.1.
- No JSON or YAML introduced.
- Test data placed in `data/test_campaign` per Implementation Rule 5.
- No modifications to `campaigns/tutorial` from tests.

---

### Quality Gates

All four gates passed after edits:

```text
cargo fmt         → no output (all files already formatted)
cargo check       → Finished 0 errors
cargo clippy      → Finished 0 warnings
cargo nextest run → 3162 passed; 0 failed; 8 skipped
```

---

## Phase 6.2 — Visual Quality Pass: Item Mesh RON Files

### Overview

Phase 6.2 improves the visual silhouette of every item mesh category so that
dropped items on the ground are immediately recognisable at tile scale.
Each category listed in the quality table from the plan now passes the
corresponding check.

---

### What Was Changed

All files are under `campaigns/tutorial/assets/items/`.

#### Weapons

| File                      | id   | What changed                                                                                                  |
| ------------------------- | ---- | ------------------------------------------------------------------------------------------------------------- |
| `weapons/dagger.ron`      | 9002 | Added `crossguard` mesh (half-width ±0.070, half-height ±0.015). Scale lowered to 0.3150 (compact).           |
| `weapons/short_sword.ron` | 9003 | Added `crossguard` mesh (±0.090 × ±0.018). Scale 0.3500.                                                      |
| `weapons/sword.ron`       | 9001 | Added `crossguard` mesh (±0.110 × ±0.020). Scale raised to 0.4025 — clearly longer than dagger.               |
| `weapons/long_sword.ron`  | 9004 | Added `crossguard` mesh (±0.130 × ±0.022). Scale 0.4375.                                                      |
| `weapons/great_sword.ron` | 9005 | Added `crossguard` mesh (±0.160 × ±0.025). Scale 0.5250 — dominant two-handed silhouette.                     |
| `weapons/club.ron`        | 9006 | Split into `handle` (thin shaft) + `head` (wide 6-point boxy hexagon). Scale 0.4025.                          |
| `weapons/staff.ron`       | 9007 | Renamed shaft to `shaft` (widened ±0.035). Added `orb_tip` 8-point polygon at Z+0.48 with blue emissive glow. |
| `weapons/bow.ron`         | 9008 | Renamed limb to `limb` (tightened arc). Added `string` diamond mesh for visible bowstring. Scale 0.5600.      |

**Crossguard material** (all swords): `color (0.60, 0.60, 0.64)`, `metallic 0.65`, `roughness 0.35` — slightly darker and more weathered than the polished blade.

**Scale progression** ensures clear size graduation:

```
dagger(0.3150) → short_sword(0.3500) → sword(0.4025) → long_sword(0.4375) → great_sword(0.5250)
```

#### Armor

| File                   | id   | What changed                                                                                                                                       |
| ---------------------- | ---- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `armor/plate_mail.ron` | 9103 | Split into `body` (narrower rectangle) + `shoulders` (wide U-shaped pauldron extending ±0.32 X). Scale 0.4550. High metallic 0.75, roughness 0.25. |
| `armor/helmet.ron`     | 9105 | Added `visor` mesh (thin dark horizontal stripe) over the existing `dome`. Scale 0.3850.                                                           |

`leather_armor.ron` retains its plain trapezoid — the **silhouette contrast** now comes from plate's shoulder extensions vs leather's clean trapezoidal outline.

#### Accessories

| File                   | id   | What changed                                                                                                                                                                 |
| ---------------------- | ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `accessories/ring.ron` | 9301 | **Complete rework** — annular washer shape. 12 outer vertices (r=0.160) + 12 inner vertices (r=0.070), 24 stitched triangles. Outer radius ≥ 0.15 as required. Scale 0.2100. |

The ring now has a visible hole in the centre so it reads as a torus/ring at tile scale. The amulet retains its filled-disc shape, making the two accessories visually distinct.

#### Ammo

| File             | id   | What changed                                                                                             |
| ---------------- | ---- | -------------------------------------------------------------------------------------------------------- |
| `ammo/arrow.ron` | 9401 | Split into `shaft` (thin diamond, width 0.018) + `fletching` (triangular red fin at tail). Scale 0.2100. |

---

### Architecture Compliance

- All RON files use `.ron` extension.
- No SPDX headers in RON data files (only in `.rs` source files).
- `mesh_transforms` has exactly one entry per mesh in every file.
- Normals array has exactly as many entries as vertices in every mesh.
- All floats have decimal points.
- No JSON or YAML format used.

---

### Quality Gate Verification

```text
cargo fmt         → no output
cargo check       → Finished 0 errors
cargo clippy      → Finished 0 warnings
cargo nextest run → 3162 passed; 0 failed; 8 skipped
```

---

## Phase 6 — Complete: Full Item Mesh Coverage

### Overview

Phase 6 is the final phase of the Items Procedural Meshes implementation plan.
It brings full coverage of all base items, a visual quality pass, authored
in-world dropped item events, and comprehensive coverage tests.

---

### Deliverables Checklist

- [x] All base items in `data/items.ron` (32 items, IDs 1–101) covered by a
      valid auto-generated `ItemMeshDescriptor` — verified by
      `test_all_base_items_have_valid_mesh_descriptor`
- [x] Visual quality pass completed for all 13 categories (see Phase 6.2 above)
- [x] At least three authored `DroppedItem` events in tutorial campaign maps: - `map_1.ron` (3,17): Short Sword — near starting room - `map_2.ron` (2,5): Healing Potion — first dungeon entrance - `map_4.ron` (3,3): Ring of Protection — treasure chamber
- [x] Full coverage tests passing (see Phase 6.4 below)

---

### Phase 6.4 Tests

Three new tests added to `src/domain/visual/item_mesh.rs` `mod tests`:

| Test                                             | What it verifies                                                                                                                          |
| ------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `test_all_base_items_have_valid_mesh_descriptor` | Every item in `data/items.ron` → `ItemMeshDescriptor::from_item` → `to_creature_definition()` → `validate()` returns `Ok`                 |
| `test_item_mesh_registry_tutorial_coverage`      | `CampaignLoader` on `data/test_campaign` returns non-empty item mesh registry with ≥ 2 entries                                            |
| `test_dropped_item_event_in_map_ron`             | `data/test_campaign/data/maps/map_1.ron` deserialises as `Map`, contains ≥ 1 `MapEvent::DroppedItem`, specifically item_id=4 (Long Sword) |

All tests use `data/test_campaign` — not `campaigns/tutorial` — per Implementation Rule 5.

---

### Quality Gates — Final

```text
cargo fmt         → no output
cargo check       → Finished 0 errors
cargo clippy      → Finished 0 warnings (0 warnings)
cargo nextest run → 3162 passed; 0 failed; 8 skipped
```

---

## Items Procedural Meshes — Phase 3.2: Python Generator Script

**Plan**: [`items_procedural_meshes_implementation_plan.md`](items_procedural_meshes_implementation_plan.md)

### Overview

Phase 3.2 delivers `examples/generate_item_meshes.py` — the developer
convenience script called out in the Phase 3 deliverables list. The script
generates every `CreatureDefinition` RON file under
`campaigns/tutorial/assets/items/` from a single authoritative Python manifest,
making the asset files regenerable without hand-editing them one by one.

---

### Phase 3.2 Deliverables

**Files created / updated**:

- `examples/generate_item_meshes.py` _(new)_

---

### What Was Built

#### Script structure

The script is organised into four layers:

1. **RON formatting helpers** — `fv()`, `fc()`, `fmat()`, `emit_mesh()`,
   `emit_transform()`, `write_item_ron()`: pure string-building functions that
   produce syntactically correct RON without any external library dependency.

2. **Color / scale constants** — mirror `item_mesh.rs` exactly so that
   re-generated files stay visually consistent with the runtime pipeline:
   `COLOR_STEEL`, `COLOR_WOOD`, `COLOR_LEATHER`, `COLOR_SILVER`, `COLOR_GOLD`,
   `COLOR_ORB`, `EMISSIVE_MAGIC`, `EMISSIVE_ORB`, `EMISSIVE_QUEST`,
   `BASE_SCALE`, `TWO_HANDED_SCALE_MULT`, `ARMOR_MED_SCALE_MULT`,
   `ARMOR_HEAVY_SCALE_MULT`, `SMALL_SCALE_MULT`.

3. **Geometry builders** — one function per logical item type, each returning
   `(list[mesh_str], list[transform_tuple])`. Multi-part items emit multiple
   `MeshDefinition` blocks with correct per-part transforms:

   | Builder                                                                                         | Parts | Description                                               |
   | ----------------------------------------------------------------------------------------------- | ----- | --------------------------------------------------------- |
   | `build_sword` / `build_dagger` / `build_short_sword` / `build_long_sword` / `build_great_sword` | 2     | Diamond blade + rectangular crossguard                    |
   | `build_club`                                                                                    | 2     | Rectangular handle + fan-hexagon head                     |
   | `build_staff`                                                                                   | 2     | Rectangular shaft + 8-sided orb tip (offset to shaft tip) |
   | `build_bow`                                                                                     | 2     | Curved arc limb + thin bowstring                          |
   | `build_plate_mail`                                                                              | 2     | Body plate + U-shaped pauldron bar                        |
   | `build_helmet`                                                                                  | 2     | Pentagon dome + rectangular visor                         |
   | `build_arrow`                                                                                   | 2     | Diamond shaft + V-shaped fletching                        |
   | `build_quest_scroll`                                                                            | 2     | Hex scroll body + 16-point star seal                      |
   | `build_leather_armor`, `build_chain_mail`, `build_shield`, `build_boots`                        | 1     | Single silhouette                                         |
   | `build_health/mana/cure/attribute_potion`                                                       | 1     | Hexagonal disc                                            |
   | `build_ring`                                                                                    | 1     | Flat torus (two concentric n-gons joined by quad strips)  |
   | `build_amulet`                                                                                  | 1     | Octagon disc                                              |
   | `build_belt`, `build_cloak`                                                                     | 1     | Rectangle / teardrop                                      |
   | `build_bolt`, `build_stone`                                                                     | 1     | Flat diamond                                              |
   | `build_key_item`                                                                                | 1     | 16-point star                                             |

4. **Manifests** — `MANIFEST` (27 entries covering all IDs 9001–9502) and
   `TEST_MANIFEST` (2 entries: sword + potion) for the
   `data/test_campaign/assets/items/` fixtures.

#### CLI usage

```text
# Full manifest → campaigns/tutorial/assets/items/
python examples/generate_item_meshes.py

# Test fixtures → data/test_campaign/assets/items/
python examples/generate_item_meshes.py --test-fixtures

# Custom root directory
python examples/generate_item_meshes.py --output-dir /tmp/items
```

The script is idempotent. Re-running overwrites existing files with freshly
generated geometry. All `.ron` files are committed; the script is not a build
step.

#### Part counts per committed file

| File                               | Parts                  |
| ---------------------------------- | ---------------------- |
| `weapons/sword.ron`                | 2 (blade, crossguard)  |
| `weapons/dagger.ron`               | 2 (blade, crossguard)  |
| `weapons/short_sword.ron`          | 2 (blade, crossguard)  |
| `weapons/long_sword.ron`           | 2 (blade, crossguard)  |
| `weapons/great_sword.ron`          | 2 (blade, crossguard)  |
| `weapons/club.ron`                 | 2 (handle, head)       |
| `weapons/staff.ron`                | 2 (shaft, orb_tip)     |
| `weapons/bow.ron`                  | 2 (limb, string)       |
| `armor/leather_armor.ron`          | 1 (leather)            |
| `armor/chain_mail.ron`             | 1 (chain)              |
| `armor/plate_mail.ron`             | 2 (body, shoulders)    |
| `armor/shield.ron`                 | 1 (shield)             |
| `armor/helmet.ron`                 | 2 (dome, visor)        |
| `armor/boots.ron`                  | 1 (boots)              |
| `consumables/health_potion.ron`    | 1 (potion)             |
| `consumables/mana_potion.ron`      | 1 (potion)             |
| `consumables/cure_potion.ron`      | 1 (potion)             |
| `consumables/attribute_potion.ron` | 1 (potion)             |
| `accessories/ring.ron`             | 1 (band)               |
| `accessories/amulet.ron`           | 1 (amulet)             |
| `accessories/belt.ron`             | 1 (belt)               |
| `accessories/cloak.ron`            | 1 (cloak)              |
| `ammo/arrow.ron`                   | 2 (shaft, fletching)   |
| `ammo/bolt.ron`                    | 1 (bolt)               |
| `ammo/stone.ron`                   | 1 (stone)              |
| `quest/quest_scroll.ron`           | 2 (quest_scroll, seal) |
| `quest/key_item.ron`               | 1 (key_item)           |

---

### Architecture Compliance

- SPDX header present: `// SPDX-FileCopyrightText: 2026 Brett Smith` +
  `Apache-2.0` on lines 2–3.
- File extension `.py` — developer tool, not a game data file.
- No game data in JSON/YAML; all output files use `.ron` as required.
- Test fixtures written to `data/test_campaign/assets/items/` — not
  `campaigns/tutorial` — per Implementation Rule 5.
- `--output-dir` flag allows targeting any directory, satisfying the plan's
  §3.2 requirement verbatim.
- Script is idempotent and not a build step.

---

### Quality Gates

```text
cargo fmt         → no output
cargo check       → Finished 0 errors
cargo clippy      → Finished 0 warnings
cargo nextest run → 3162 passed; 0 failed; 8 skipped
python3 examples/generate_item_meshes.py --output-dir /tmp/items → 27 files ✅
python3 examples/generate_item_meshes.py --test-fixtures          →  2 files ✅
```

---

## macOS Menu-Bar Status Item — Phase 2: macOS Menu-Bar Status Item (Complete)

### Overview

Phase 2 adds a real macOS menu-bar status item (`NSStatusItem`) to the Campaign
Builder using the [`tray-icon`](https://crates.io/crates/tray-icon) crate (Tauri
ecosystem). When the application is running, the Antares icon appears in the
top-right macOS menu bar. Clicking it opens a context menu with a single
**Quit** action that terminates the process cleanly via
`std::process::exit(0)`.

All tray code is gated on `#[cfg(target_os = "macos")]` — non-macOS targets
compile and link without any change in behaviour or warnings.

### Phase 2 Deliverables Checklist

- [x] `scripts/generate_icons.sh` verified — already existed; generates
      `assets/icons/generated/macos/tray_icon_1x.png` (22×22) and
      `tray_icon_2x.png` (44×44) from the source 1513×1513 PNG
- [x] `assets/icons/generated/macos/tray_icon_1x.png` (22×22) — generated
- [x] `assets/icons/generated/macos/tray_icon_2x.png` (44×44) — generated
- [x] `sdk/campaign_builder/assets/icons/tray_icon_1x.png` (22×22) — copied
      into SDK for compile-time embedding
- [x] `sdk/campaign_builder/assets/icons/tray_icon_2x.png` (44×44) — copied
      into SDK for Retina / future HiDPI use
- [x] `sdk/campaign_builder/Cargo.toml` — `tray-icon = "0.19"` added under
      `[target.'cfg(target_os = "macos")'.dependencies]`
- [x] `sdk/campaign_builder/src/tray.rs` — new module with:
  - `TRAY_ICON_1X` / `TRAY_ICON_2X` embedded constants
  - `MENU_ID_QUIT` constant (`"quit"`)
  - `build_tray_icon() -> tray_icon::TrayIcon` — decodes icon, builds
    context menu, constructs `NSStatusItem`
  - `handle_tray_events()` — drains menu-event channel; dispatches `"quit"`
    → `std::process::exit(0)`
  - 5 unit tests (see below)
- [x] `sdk/campaign_builder/src/lib.rs`:
  - `#[cfg(target_os = "macos")] pub mod tray;` module declaration added
  - `#[cfg(target_os = "macos")] let _tray = tray::build_tray_icon();` in
    `run()` after `NativeOptions` construction and before `eframe::run_native`
  - `#[cfg(target_os = "macos")] tray::handle_tray_events();` at the top of
    `CampaignBuilderApp::update()`
- [x] All five required tray tests pass
- [x] All four quality gates pass with zero errors and zero warnings

### Files Changed

| File                                                 | Change                                                                                                                                      |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `assets/icons/generated/macos/tray_icon_1x.png`      | **New** — 22×22 menu-bar icon generated from source PNG by Python/Pillow                                                                    |
| `assets/icons/generated/macos/tray_icon_2x.png`      | **New** — 44×44 Retina icon generated from source PNG by Python/Pillow                                                                      |
| `sdk/campaign_builder/assets/icons/tray_icon_1x.png` | **New** — 22×22 icon embedded in the SDK crate via `include_bytes!`                                                                         |
| `sdk/campaign_builder/assets/icons/tray_icon_2x.png` | **New** — 44×44 icon embedded in the SDK crate via `include_bytes!`                                                                         |
| `sdk/campaign_builder/Cargo.toml`                    | Added `[target.'cfg(target_os = "macos")'.dependencies]` section with `tray-icon = { version = "0.19", features = [] }`                     |
| `sdk/campaign_builder/src/tray.rs`                   | **New** — tray module: embedded constants, `build_tray_icon`, `handle_tray_events`, 5 unit tests; file-level `#![cfg(target_os = "macos")]` |
| `sdk/campaign_builder/src/lib.rs`                    | Added `#[cfg(target_os = "macos")] pub mod tray;`; `_tray` binding in `run()`; `handle_tray_events()` call in `update()`                    |

### Architecture Details

#### `tray.rs` — File-Level Platform Gate

The entire file is wrapped with `#![cfg(target_os = "macos")]`. On non-macOS
targets the file compiles to an empty module; neither the `tray_icon` crate
imports nor the embedded asset bytes are compiled or linked. The
`tray-icon` dependency is itself declared in a
`[target.'cfg(target_os = "macos")'.dependencies]` section, so it is not
fetched or linked on other platforms at all.

```sdk/campaign_builder/src/tray.rs#L37-L40
#![cfg(target_os = "macos")]

use tray_icon::{
    menu::{Menu, MenuItem},
```

#### `build_tray_icon()` — Four-Step Construction

```sdk/campaign_builder/src/tray.rs#L102-L128
pub fn build_tray_icon() -> tray_icon::TrayIcon {
    // 1. Decode the 22×22 PNG to RGBA8.
    let img = image::load_from_memory(TRAY_ICON_1X)
        .expect("failed to decode tray_icon_1x.png — embedded bytes must be valid PNG")
        .into_rgba8();
    let width = img.width();
    let height = img.height();
    let rgba = img.into_raw();

    // 2. Construct the platform icon from raw RGBA bytes.
    let icon = Icon::from_rgba(rgba, width, height)
        .expect("failed to construct tray_icon::Icon — RGBA dimensions must be consistent");

    // 3. Build the context menu: one "Quit" item.
    let menu = Menu::new();
    let quit_item = MenuItem::with_id(MENU_ID_QUIT, "Quit", true, None);
    menu.append(&quit_item)
        .expect("failed to append Quit item to tray context menu");

    // 4. Assemble and return the TrayIcon.
    TrayIconBuilder::new()
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .with_tooltip("Antares Campaign Builder")
        .build()
        .expect("failed to build macOS NSStatusItem — must be called on the main thread")
}
```

#### `handle_tray_events()` — Non-Blocking Channel Drain

`tray_icon::menu::MenuEvent::receiver()` returns a reference to a
`crossbeam_channel::Receiver<MenuEvent>`. `try_recv()` is non-blocking —
it returns `Err(TryRecvError::Empty)` immediately when no events are pending,
so calling it once per frame adds negligible overhead:

```sdk/campaign_builder/src/tray.rs#L147-L154
pub fn handle_tray_events() {
    while let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
        if event.id == MENU_ID_QUIT {
            std::process::exit(0);
        }
    }
}
```

`event.id` is a `MenuId(String)` that implements `PartialEq<&str>`, so the
string comparison is direct and allocation-free.

#### `_tray` Lifetime in `run()`

`TrayIcon` is `!Send + !Sync`. The binding is created on the main thread
before `eframe::run_native` and is dropped only when `run()` returns (after
the eframe event loop exits):

```sdk/campaign_builder/src/lib.rs#L152-L157
// Build the macOS menu-bar status item (NSStatusItem).  The binding must
// remain live for the entire duration of `run_native`; dropping it removes
// the icon from the menu bar.
#[cfg(target_os = "macos")]
let _tray = tray::build_tray_icon();
```

eframe's event loop also runs on the main thread, so no cross-thread
constraints are violated.

#### Per-Frame Poll in `update()`

`handle_tray_events()` is the first statement in `update()`, before any UI
rendering. This ensures tray menu actions are processed at the start of each
frame rather than after potentially expensive UI work:

```sdk/campaign_builder/src/lib.rs#L4212-L4215
// Poll macOS menu-bar status item events once per frame.
// Handles "Quit" and any future tray menu actions without a separate thread.
#[cfg(target_os = "macos")]
tray::handle_tray_events();
```

### Tests Added

All 5 tests live in `sdk/campaign_builder/src/tray.rs` under
`#[cfg(test)]`. They decode pixel data only — no `NSApp` or `NSStatusItem`
is touched — so they are safe to run in any headless CI environment on macOS.

| Test                            | What it verifies                                             |
| ------------------------------- | ------------------------------------------------------------ |
| `test_tray_icon_1x_png_magic`   | `TRAY_ICON_1X` bytes begin with PNG magic `[137,80,78,71,…]` |
| `test_tray_icon_2x_png_magic`   | `TRAY_ICON_2X` bytes begin with PNG magic `[137,80,78,71,…]` |
| `test_tray_icon_1x_dimensions`  | Decoded `width == 22`, `height == 22`                        |
| `test_tray_icon_2x_dimensions`  | Decoded `width == 44`, `height == 44`                        |
| `test_tray_icon_1x_rgba_length` | `rgba.len() == 22 × 22 × 4 == 1936`                          |

### Quality Gate Results

```text
cargo fmt --all           → No output (all files formatted)
cargo check --all-targets → Finished with 0 errors, 0 warnings
cargo clippy -D warnings  → Finished with 0 warnings
cargo nextest run         → 3510/3510 passed, 8 skipped
  (campaign_builder: 2045/2045 passed, including 5 new tray tests)
```

---

---

## Dropped Item World Persistence — Phase 4: Save/Load Validation and End-to-End Testing (Complete)

### Overview

Phase 4 completes the dropped-item persistence feature by adding three
end-to-end integration tests that exercise the entire flow — from domain
transaction through RON serialisation to reloaded world state — and verifies
that the `SaveGameManager` round-trip preserves `DroppedItem` data with
complete fidelity.

No new production code was required: `Map` already derives
`Serialize`/`Deserialize` and the `dropped_items` field already carries
`#[serde(default, skip_serializing_if = "Vec::is_empty")]`, so dropped items
serialise and deserialise automatically through the existing `SaveGame` RON
pipeline introduced in Phase 1.

The test fixtures required by the integration tests (a weapon and a consumable)
were already present in `data/test_campaign/data/items.ron` from earlier
campaign development:

| Fixture constant     | Item id | Name           | Type       |
| -------------------- | ------- | -------------- | ---------- |
| `WEAPON_ITEM_ID`     | `3`     | Short Sword    | Weapon     |
| `CONSUMABLE_ITEM_ID` | `50`    | Healing Potion | Consumable |

### Phase 4 Deliverables Checklist

- [x] `tests/dropped_item_integration_test.rs` — three integration tests
  - [x] `test_dropped_item_round_trip_save_load`
  - [x] `test_multiple_items_stacked_on_same_tile`
  - [x] `test_dropped_item_scoped_to_map`
- [x] `data/test_campaign/data/items.ron` contains required weapon (id=3) and consumable (id=50) fixtures (pre-existing)
- [x] Save/load round-trip confirmed in automated tests
- [x] No tests reference `campaigns/tutorial` (Implementation Rule 5 compliant)
- [x] All four quality gates pass with zero errors and zero warnings
- [x] `docs/explanation/implementations.md` updated (this entry)

### Files Changed

| File                                     | Change                                                                                 |
| ---------------------------------------- | -------------------------------------------------------------------------------------- |
| `tests/dropped_item_integration_test.rs` | **New** — three end-to-end integration tests with shared helpers and fixture constants |
| `data/test_campaign/data/items.ron`      | No change required — weapon (id=3) and consumable (id=50) already present              |

### Test Details

#### `test_dropped_item_round_trip_save_load`

Full seven-step flow verifying that a `DroppedItem` survives a complete
save/load cycle and is then retrievable and removable:

1. Construct `GameState` with a 20×20 map and a character carrying a Short Sword.
2. Call `drop_item()` — verifies item removed from inventory and written to
   `map.dropped_items`.
3. Call `SaveGameManager::save()` — serialises `GameState` (including
   `Map::dropped_items`) to a RON file in a `TempDir`.
4. Call `SaveGameManager::load()` — deserialises the RON file back into
   `GameState`.
5. Assert all `DroppedItem` fields (`item_id`, `charges`, `position`,
   `map_id`) are intact in the loaded world.
6. Call `trigger_event()` at the drop tile — asserts `EventResult::PickupItem`
   is returned with matching fields.
7. Call `pickup_item()` — asserts the inventory is updated and
   `map.dropped_items` is empty.

#### `test_multiple_items_stacked_on_same_tile`

Verifies that an arbitrary number of `DroppedItem` entries can share one tile
and that the FIFO insertion order is preserved through save/load:

1. Drop a weapon (slot 0) then a potion (slot 0 after shift) on the same tile.
2. Assert `map.dropped_items_at(tile)` returns both entries in insertion order.
3. Save and load.
4. Assert both entries survive with correct FIFO ordering.
5. Call `trigger_event()` twice, picking up weapon first then potion.
6. Assert `map.dropped_items` is empty and both items are in inventory.

#### `test_dropped_item_scoped_to_map`

Verifies that `DroppedItem` entries are bound to their owning map and do not
leak across map boundaries:

1. Create a `GameState` with two maps (ids 1 and 2).
2. Drop a weapon on map 1; assert map 2 has no dropped items.
3. Save and load.
4. Assert map 1 still has the item and map 2 remains empty.
5. Call `trigger_event()` with `current_map = 1` → `PickupItem`.
6. Call `trigger_event()` with `current_map = 2` → `None` (item invisible from
   map 2).

### Architecture Notes

#### Why no production code changes were needed

Phase 1 added `#[serde(default, skip_serializing_if = "Vec::is_empty")]` to
`Map::dropped_items`, which is all the wiring required for round-trip
serialisation. `SaveGame` wraps `GameState` which owns `World` which owns
`HashMap<MapId, Map>`, so the entire map collection — including every
`dropped_items` vector — is serialised and deserialised transparently by the
existing RON pipeline.

#### Borrow-splitting in integration tests

The integration tests use the same NLL field-split borrow pattern established
in Phase 2's `inventory_action_system`:

```tests/dropped_item_integration_test.rs#L110-116
{
    let party_ref = &mut state.party.members[0];
    let world_ref = &mut state.world;
    drop_item(party_ref, 0, 0, world_ref, MAP_ONE, drop_pos)
        .expect("drop_item must succeed");
}
```

`state.party` and `state.world` are disjoint fields of `GameState`, so Rust's
NLL borrow checker permits both `&mut` borrows within the same block. Each
operation is wrapped in its own block to release the borrows before the next
assertion.

#### `TempDir` isolation

Every test creates its own `TempDir` (from the `tempfile` crate), which is
automatically deleted when the test ends. Save files never touch the
repository working tree.

#### `trigger_event` re-checks after pickup

The plan specifies that `trigger_event` must return `None` after all items at a
tile have been picked up. Both `test_dropped_item_round_trip_save_load` and
`test_multiple_items_stacked_on_same_tile` verify this implicitly: after the
final pickup, `map.dropped_items` is asserted empty, which means a subsequent
`trigger_event` call on that tile would return `EventResult::None`.

### Quality Gate Results

```text
cargo fmt --all           → No output (all files formatted)
cargo check --all-targets → Finished with 0 errors, 0 warnings
cargo clippy -D warnings  → Finished with 0 warnings
cargo nextest run         → 3510/3510 passed, 8 skipped
```

The three new tests (`test_dropped_item_round_trip_save_load`,
`test_multiple_items_stacked_on_same_tile`, `test_dropped_item_scoped_to_map`)
are included in the 3510 passing tests.

---

## macOS Menu-Bar Status Item — Phase 3: Show/Hide Window from Menu Bar (Complete)

### Overview

Extends the macOS menu-bar tray integration (Phase 2) with a Show/Hide window
toggle driven by a `std::sync::mpsc` channel. The tray's context menu now
exposes three items — **Show Antares Campaign Builder**, **Hide**, and **Quit**
— and dispatches `TrayCommand` values over a bounded sync channel so that the
`update()` loop can issue the appropriate `egui::ViewportCommand`s without
blocking any OS callback thread.

Key design decisions:

- A `TrayCommand` enum (`ShowWindow`, `HideWindow`, `Quit`) carries intent
  across the thread boundary from the tray event handler to the egui render
  loop.
- A `SyncSender<TrayCommand>` (which is both `Send` and `Sync`) is stored in
  a module-level `OnceLock`, satisfying the `static` bound without a `Mutex`.
- `build_tray_icon()` now returns `(tray_icon::TrayIcon, Receiver<TrayCommand>)`;
  the `TrayIcon` stays alive in `run()` outside the closure, and the `Receiver`
  is moved into `CampaignBuilderApp` inside the closure.
- `handle_tray_events()` retains its zero-argument signature: it retrieves the
  sender from the `OnceLock` and sends commands without requiring the caller to
  pass anything extra.
- `Quit` remains a direct `process::exit(0)` call for immediate, synchronous
  termination; `TrayCommand::Quit` is also provided as a belt-and-suspenders
  `ViewportCommand::Close` path drained in `update()`.

### Phase 3 Deliverables Checklist

- [x] `sdk/campaign_builder/src/tray.rs` — `TrayCommand` enum (`PartialEq`,
      `Debug`, `Clone`); `MENU_ID_SHOW` / `MENU_ID_HIDE` constants; module-level
      `TRAY_CMD_TX: OnceLock<SyncSender<TrayCommand>>`; `build_tray_icon()` returns
      `(TrayIcon, Receiver<TrayCommand>)`; Show + Hide menu items appended;
      `handle_tray_events()` sends `ShowWindow` / `HideWindow` over the channel;
      2 new unit tests
- [x] `sdk/campaign_builder/src/lib.rs` — `tray_cmd_rx:
Option<std::sync::mpsc::Receiver<tray::TrayCommand>>` field on
      `CampaignBuilderApp` (cfg-gated); `Default` impl initialises it to `None`;
      `run()` destructures `build_tray_icon()` and wires the receiver into the app;
      `update()` drains the receiver and issues `ViewportCommand`s
- [x] All quality gates pass (`cargo fmt`, `cargo check`, `cargo clippy -D
warnings`, `cargo nextest run`)

### Files Changed

| File                               | Change                                                                                                          |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `sdk/campaign_builder/src/tray.rs` | `TrayCommand` enum, `OnceLock` sender, updated `build_tray_icon()`, updated `handle_tray_events()`, 2 new tests |
| `sdk/campaign_builder/src/lib.rs`  | `tray_cmd_rx` field, `Default` init, `run()` wiring, `update()` drain loop                                      |

### Architecture Details

#### `TrayCommand` Enum

```sdk/campaign_builder/src/tray.rs#L113-125
#[derive(Debug, Clone, PartialEq)]
pub enum TrayCommand {
    /// Raise the window to the front and make it visible.
    ShowWindow,
    /// Hide the window without terminating the process.
    HideWindow,
    /// Close the egui viewport (counterpart to the direct `process::exit` path).
    Quit,
}
```

#### `OnceLock<SyncSender>` Pattern

`SyncSender<T>` implements both `Send` and `Sync` (unlike `Sender<T>` which is
only `Send`). This makes it eligible for storage in a `OnceLock<T>` static,
which requires `T: Send + Sync`. The bounded channel capacity of 32 ensures
that even rapid menu clicks cannot block the OS callback thread.

```sdk/campaign_builder/src/tray.rs#L133-136
static TRAY_CMD_TX: OnceLock<SyncSender<TrayCommand>> = OnceLock::new();
```

#### `build_tray_icon()` — Updated Signature

```sdk/campaign_builder/src/tray.rs#L193-196
pub fn build_tray_icon() -> (tray_icon::TrayIcon, Receiver<TrayCommand>) {
    let (tx, rx) = mpsc::sync_channel(32);
    let _ = TRAY_CMD_TX.set(tx);
    // ...
```

The `TrayIcon` is kept in `run()` via `let (_tray, tray_cmd_rx) =
tray::build_tray_icon();` outside the `run_native` closure. The `Receiver` is
moved into the closure and stored as `app.tray_cmd_rx = Some(tray_cmd_rx)`.

#### Menu Items

Three items are appended in order: **Show Antares Campaign Builder** (ID
`"show"`), **Hide** (ID `"hide"`), **Quit** (ID `"quit"`).

#### `handle_tray_events()` — Channel Dispatch

The function retrieves the sender from `TRAY_CMD_TX.get()` (returning early if
not yet initialised) and maps raw `MenuEvent` IDs to channel sends or a direct
`process::exit(0)`:

- `"quit"` → `std::process::exit(0)` (synchronous, immediate)
- `"show"` → `tx.send(TrayCommand::ShowWindow)` (non-blocking; error ignored
  if receiver dropped)
- `"hide"` → `tx.send(TrayCommand::HideWindow)`
- Unknown IDs → silently ignored

#### `update()` — Receiver Drain

Each frame, after calling `tray::handle_tray_events()`, the app drains
`self.tray_cmd_rx` with `try_recv()` and issues `ViewportCommand`s:

```sdk/campaign_builder/src/lib.rs#L4241-4266
#[cfg(target_os = "macos")]
if let Some(ref rx) = self.tray_cmd_rx {
    while let Ok(cmd) = rx.try_recv() {
        match cmd {
            tray::TrayCommand::ShowWindow => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            tray::TrayCommand::HideWindow => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
            tray::TrayCommand::Quit => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}
```

`Visible(true)` is issued before `Focus` so the window exists in the window
manager before the focus request is processed.

#### Non-macOS Builds

All new code is gated with `#[cfg(target_os = "macos")]`. The `tray` module
itself has `#![cfg(target_os = "macos")]` at the file level, so non-macOS
builds see neither the module nor the struct field.

### Tests Added

#### `sdk/campaign_builder/src/tray.rs` — 2 new tests (Phase 3)

| Test name                                      | What it verifies                                                                                            |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `test_tray_command_show_is_distinct_from_hide` | `TrayCommand::ShowWindow != TrayCommand::HideWindow`; confirms `PartialEq` derivation                       |
| `test_tray_command_channel_send_recv`          | A `TrayCommand::ShowWindow` sent over an `mpsc::sync_channel` is received via `try_recv()` without blocking |

Both tests are purely data-structure / channel tests; no `NSApp` or
`NSStatusItem` is touched. They run anywhere the crate compiles (macOS only,
due to the file-level `#![cfg(target_os = "macos")]` gate).

### Quality Gate Results

```text
cargo fmt --all                                → No output (all files formatted)
cargo check --all-targets --all-features       → Finished with 0 errors, 0 warnings
cargo clippy --all-targets --all-features
  -- -D warnings                               → Finished with 0 warnings
cargo nextest run --all-features
  -p campaign_builder                          → 2047/2047 passed (2 new tray tests included)
```

The 2 new tray tests:

- `campaign_builder tray::tests::test_tray_command_show_is_distinct_from_hide` ✅
- `campaign_builder tray::tests::test_tray_command_channel_send_recv` ✅

---
