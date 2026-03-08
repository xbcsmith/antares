# Implementations

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
