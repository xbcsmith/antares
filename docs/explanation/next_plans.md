# Next Plans

## Generic

### Consolidate and Enforce All ID Range Allocations

The project has accumulated two parallel numeric ID spaces (game-data entity IDs
and visual/mesh registry IDs) across eight registries, but there is no single
authoritative definition of where each space starts and ends.  Constants exist
for only one registry (`LANDSCAPE_MESH_ID_MIN = 11000`).  The object mesh
registry still uses string keys instead of numbers.  Creature custom IDs (4000+)
have no declared upper bound and theoretically bleed into the item mesh range
(9000+) if a campaign adds enough creatures.

The full current state is documented in
`docs/reference/monster_creature_mapping_reference.md` — ID Space Allocation
section.  The implementation work below makes the codebase match that document.

#### Step 1 — Define named constants for every range boundary

In `src/domain/types.rs` (or a new `src/domain/id_ranges.rs`), add one constant
per registry boundary:

```rust
// Visual mesh registry boundaries
pub const CREATURE_MONSTER_ID_MIN: u32    = 1;
pub const CREATURE_MONSTER_ID_MAX: u32    = 999;
pub const CREATURE_NPC_ID_MIN: u32        = 1000;
pub const CREATURE_NPC_ID_MAX: u32        = 1999;
pub const CREATURE_TEMPLATE_ID_MIN: u32   = 2000;
pub const CREATURE_TEMPLATE_ID_MAX: u32   = 2999;
pub const CREATURE_VARIANT_ID_MIN: u32    = 3000;
pub const CREATURE_VARIANT_ID_MAX: u32    = 3999;
pub const CREATURE_CUSTOM_ID_MIN: u32     = 4000;
pub const CREATURE_CUSTOM_ID_MAX: u32     = 8999;  // NEW — cap before item mesh range
pub const ITEM_MESH_ID_MIN: u32           = 9000;
pub const ITEM_MESH_ID_MAX: u32           = 9999;
pub const FURNITURE_MESH_ID_MIN: u32      = 10000;
pub const FURNITURE_MESH_ID_MAX: u32      = 10999;
pub const LANDSCAPE_MESH_ID_MIN: u32      = 11000; // already exists
pub const LANDSCAPE_MESH_ID_MAX: u32      = 11999;
pub const OBJECT_MESH_ID_MIN: u32         = 12000; // new — was string-keyed
pub const OBJECT_MESH_ID_MAX: u32         = 12999;
```

- Replace all hardcoded range literals in `CreatureIdManager`, the importers,
  and `obj_importer.rs` with these constants
- `CreatureCategory::Custom` should now use `CREATURE_CUSTOM_ID_MIN..=CREATURE_CUSTOM_ID_MAX`
  (upper-bounded) rather than `4000..u32::MAX`

#### Step 2 — Add `DialogueId` type alias

Dialogues currently use a bare `u32`.  Add `pub type DialogueId = u32;` in
`src/domain/types.rs` and replace raw `u32` usages in the dialogue domain and
data loading code.  No data files change.

#### Step 3 — Add SDK range-boundary validation to mesh importers

Each importer export path (`ExportType::Item`, `ExportType::Furniture`,
`ExportType::Landscape`, `ExportType::ObjectMesh`) should validate that the
assigned ID falls within its declared range before writing the registry.  Return
a `ValidationResult::Error` with a clear message if it would overflow into an
adjacent range.

#### Step 4 — Update `architecture.md` section 4.6

Replace the current sparse code block in section 4.6 with a prose + table
summary that mirrors the table now in
`docs/reference/monster_creature_mapping_reference.md`, referencing the new
constants.  This is the single authoritative written statement of what every
numeric ID means.

#### Step 5 — Fix `object_mesh_registry.ron` (dependency)

This step depends on the separate
**Refactor `object_mesh_registry.ron` to ID-Based Format** plan in this file.
Complete that refactor first; this step just ensures the resulting numeric IDs
use `OBJECT_MESH_ID_MIN` as their base and that the object mesh importer
validates against the 12000–12999 range.

#### Affected files (expected)

| File | Change |
|------|--------|
| `src/domain/types.rs` | Add all range constants; add `DialogueId` alias |
| `sdk/campaign_builder/src/creature_id_manager.rs` | Use constants; cap `Custom` range at 8999 |
| `sdk/campaign_builder/src/obj_importer.rs` | Use constants for ID defaults |
| `sdk/campaign_builder/src/campaign_io.rs` | Use constants in importer sync |
| `docs/reference/architecture.md` | Update section 4.6 with full ID table |
| `docs/reference/monster_creature_mapping_reference.md` | Already updated (this session) |

Write a plan with a phased approach to update the ID for the game and sdk. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [ID Range Consolidation Implementation Plan](./id_range_consolidation_implementation_plan.md)

---

### Campaign Builder Audio Manager

The SDK Campaign Builder needs a dedicated Audio section where campaign authors
can import sound effect and music files, and remap the engine's built-in event
IDs to custom audio files.  This pairs with a new per-campaign `audio.ron` data
file (see **Game Engine / Audio RON and SFX Mapping** below).

#### Background — current audio system

As of now the audio pipeline has two layers:

- **Volume control only** — `AudioConfig` in `config.ron` exposes four sliders
  (`master_volume`, `music_volume`, `sfx_volume`, `ambient_volume`) and an
  `enable_audio` flag.  The SDK config editor already surfaces these.
- **Filename-by-convention** — combat and spell systems emit hardcoded SFX ID
  strings.  `resolve_audio_path` appends `.ogg` and looks for the file in
  `assets/audio/`.  There is no mapping layer; the ID **is** the filename.

  | Combat event        | Hardcoded SFX ID    | Resolved path                        |
  |---------------------|---------------------|--------------------------------------|
  | Attack hit          | `combat_hit`        | `assets/audio/combat_hit.ogg`        |
  | Attack / spell miss | `combat_miss`       | `assets/audio/combat_miss.ogg`       |
  | Item / spell heal   | `combat_heal`       | `assets/audio/combat_heal.ogg`       |
  | Spell fizzle        | `spell_fizzle`      | `assets/audio/spell_fizzle.ogg`      |
  | Victory fanfare     | `victory_fanfare`   | `assets/audio/victory_fanfare.ogg`   |

  To swap a sound today a campaign author must name their file to match the
  hardcoded ID.  There is no way to point `combat_hit` at an arbitrarily-named
  file without editing engine source code.

#### What needs to be built (SDK side)

- **Audio tab** in the Campaign Builder sidebar (alongside Assets, Maps, etc.)
- **Import panel** — drag-and-drop or file-picker to bring `.ogg`, `.mp3`,
  `.wav`, `.flac` files into `assets/audio/` inside the campaign directory;
  mirrors the existing asset importer pattern
- **SFX Mapping table** — an editable table that lists every well-known engine
  SFX ID on the left and lets the author choose any imported audio file on the
  right; persisted to `audio.ron`
- **Music track list** — similar table for named music tracks (e.g. `combat`,
  `exploration`, `town`)
- **Preview button** — plays the selected file through Bevy's audio so authors
  can audition sounds without launching the full game
- **Volume preview sliders** — mirror the `AudioConfig` sliders so authors can
  hear changes in context

#### Out of scope for now

- Per-monster or per-weapon sound overrides (layer on top once basic mapping
  exists)
- Waveform visualisation
- Audio encoding / conversion tooling

Write a plan with a phased approach to add audio features to game and sdk. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Audio Manager Implementation Plan](./audio_manager_implementation_plan.md)

---

### Fix Skill Trainer SDK — Dialogue ID Visibility, Detail Panel, and Template Bug

#### Background — current state

The NPC editor has two trainer checkboxes:

| Checkbox | `NpcDefinition` field | Triggers |
|---|---|---|
| `🎓 Is Trainer` | `is_trainer` | `OpenTraining` action; character level-up for gold |
| `🧠 Is Skill Trainer` | `is_skill_trainer` | `OpenSkillTraining` action; pay gold to raise a skill rank by 1 |

When a box is ticked, the SDK auto-generates a template dialogue tree and
stores its id in the NPC's single `dialogue_id` field.  Checking both boxes on
the same NPC is untested and likely broken (they fight over `dialogue_id`).

**Problems:**

1. **No dialogue ID control** — the edit panel does not show the auto-assigned
   dialogue ID anywhere near the checkbox.  There is no way to point the trainer
   at an *existing* hand-authored dialogue without using the raw data editor.
2. **Template bug** — the `ensure_skill_trainer_dialogue_for_npc` SDK function
   hard-codes `npc_id: "npc_1"` in the generated `OpenSkillTraining` action
   instead of using the actual NPC ID.  Any NPC whose dialogue was auto-created
   will silently fail at runtime (tracked separately as a Campaign/Story fix).
3. **Detail panel is silent** — `show_npc_preview` (`portrait_picker.rs`)
   renders no trainer or skill trainer information.  There is no quick way to
   see from the list view whether an NPC is a trainer, which dialogue it uses,
   or which skills it offers.

#### Step 1 — Fix the SDK dialogue template generator

In `sdk/campaign_builder/src/dialogue_editor.rs`,
`ensure_skill_trainer_dialogue_for_npc` and `ensure_trainer_dialogue_for_npc`:

- Replace the hard-coded `npc_id: "npc_1"` (and any equivalent level-trainer
  placeholder) with `npc.id.clone()` so every generated `OpenSkillTraining` /
  `OpenTraining` action contains the correct NPC ID
- Add a "Repair" path: if an existing dialogue already has the SDK-managed
  action node but with the wrong NPC ID, update it in-place
- Add tests: `test_ensure_skill_trainer_dialogue_uses_npc_id`,
  `test_ensure_trainer_dialogue_uses_npc_id`

#### Step 2 — Add dialogue ID display and picker to the edit panel

In `sdk/campaign_builder/src/npc_editor/mod.rs`, inside both the
`is_trainer` and `is_skill_trainer` expanded sections (after the status label):

- Add a read-only label showing the currently assigned dialogue ID
  (or `"(none)"` if unset)
- Add a **ComboBox** (`from_id_salt("npc_trainer_dialogue_picker")` /
  `"npc_skill_trainer_dialogue_picker"`) listing all available dialogues by
  `"<id>: <name>"`.  When the author picks one, write its id into
  `self.edit_buffer.dialogue_id` and set `needs_save = true`.
- Keep the existing "Create trainer dialogue" / "Repair" buttons; they become
  the convenience path, not the only path
- Add a small hint: `"Dialogue must contain an OpenTraining action for this NPC"`
  (or `OpenSkillTraining` for skill trainer) so the author knows what the
  dialogue needs

Example layout when `is_skill_trainer` is expanded:

```
[x] 🧠 Is Skill Trainer
    Skill trainer dialogue valid           ← coloured status label
    Dialogue: [1002: Lost Ranger Skill Trainer Dialogue ▾]  ← ComboBox
    Trainable Skills: stealth, athletics, tracking, disarm_traps
    [Create skill trainer dialogue]  [Repair]  [Remove branch]
    hint text...
```

#### Step 3 — Update the NPC detail / preview panel

In `sdk/campaign_builder/src/npc_editor/portrait_picker.rs`,
`show_npc_preview`:

- In the role badge row (where `🏪 Merchant`, `🛏️ Innkeeper`, `✝ Priest` are
  shown) add `🎓 Trainer` and `🧠 Skill Trainer` badges when the flags are set
- Add a **"Trainer"** section (when `npc.is_trainer`):
  - Dialogue ID (or `"no dialogue assigned"` in red)
  - Fee base / multiplier summary
- Add a **"Skill Trainer"** section (when `npc.is_skill_trainer`):
  - Dialogue ID (or `"no dialogue assigned"` in red)
  - Skills list: one bullet per `trainable_skill_ids` entry (or `"(none)"`)
  - Max rank override if set

#### Step 4 — Tests

- `portrait_picker`: `test_show_npc_preview_shows_skill_trainer_badge` —
  `is_skill_trainer: true` → skill trainer section rendered
- `portrait_picker`: `test_show_npc_preview_shows_trainer_badge`
- `npc_editor`: `test_edit_panel_skill_trainer_shows_dialogue_combobox`
- `npc_editor`: `test_edit_panel_trainer_shows_dialogue_combobox`
- All new egui tests must use `push_id`/`id_salt` on every scroll area and
  combo box per `sdk/AGENTS.md` Rule 13

#### Affected files (expected)

| File | Change |
|------|--------|
| `sdk/campaign_builder/src/dialogue_editor.rs` | Fix NPC ID in generated action; add repair path |
| `sdk/campaign_builder/src/npc_editor/mod.rs` | Add dialogue ComboBox to trainer and skill trainer sections |
| `sdk/campaign_builder/src/npc_editor/portrait_picker.rs` | Add trainer/skill trainer badges and detail sections |


Write a plan with a phased approach to update the training dialogues and sdk. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Fix Skill Trainer SDK Implementation Plan](./fix_skill_trainer_SDK_implementation_plan.md)

---

### Custom Font Editor

The SDK Campaign Builder needs a way to specify custom fonts for campaigns.

## Campaign / Story

---

### Party Member Dialogue — Talk Button on Character Sheet

Once a character is recruited, there is currently no way to speak with them.
This plan adds a lightweight in-party conversation system through the existing
character sheet UI.

#### Step 1 — Add `party_dialogue_id` to the character data

- Add an optional `party_dialogue_id: Option<u32>` field to the character
  domain struct and to each premade entry in `characters.ron`
- Assign ids in the 200-series range (201–208 for the eight premade characters)

#### Step 2 — Create party dialogue trees (ids 201–208)

- One short dialogue per character (2–4 nodes) that reflects their current
  story context
- Content drawn from the lore files in
  `campaigns/tutorial/assets/characters/lore/`
- Characters comment on where the party is in the story — something different
  before and after reaching the Astronomer's Temple, for example
- `repeatable: true` so the player can check in at any time

#### Step 3 — Wire a Talk button to the character sheet

- Add a "Talk" button to the character sheet UI (already exists in
  `src/game/systems/`)
- On press: look up the active character's `party_dialogue_id`, open the
  dialogue system with that id
- If `party_dialogue_id` is `None`, show a brief fallback line ("They have
  nothing to say right now.")

#### Out of scope for now

- State-conditional dialogue branches (different text per act) — can be layered
  on top once the basic system exists
- Location-reactive banter (Option C) — separate feature, tracked separately

Write a plan with a phased approach to add a Talk button. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Party Memeber Talk Button Implementation Plan](./party_member_talk_button_implementation_plan.md)

---

We should be able to configure any NPC to have the combat triggered switch entity not just Eonir. The Eonir work should be split out for the tutorial campaign once the Generic NPC Combat Trigger work is done. Rework the plan so we have a generic Feature and a example for the tutorial.

### Eonir the Still — Act IV Boss Fight (Option A)

Eonir is currently an NPC with dialogue (`tutorial_lich_eonir`). To make him
fightable at the end of Act IV while keeping the persuade/fight fork, we use
the classic separate-entity approach: the NPC handles all dialogue and the
`lich_king_eonir` creature handles the boss combat.

#### Step 1 — Add the Lich King creature entry

- Add `lich_king_eonir` to `campaigns/tutorial/data/creatures.ron`
- Give him boss-tier stats befitting a lich who has been still for centuries:
  high endurance, high magic resistance, low speed (he rarely moves)
- Assign regeneration and any special attacks (gravity, shadow barrier) that
  match the `eonir.md` combat description
- Use `creature_id: 1021` — the same ID already on the `tutorial_lich_eonir`
  NPC entry so the two are paired by convention

#### Step 2 — Add a combat dialogue branch to Eonir's dialogue

- In the dialogue file for `dialogue_id: 1003`, add a branch that represents
  the party refusing to negotiate (or failing the persuasion check)
- The combat branch sets an outcome flag, e.g. `eonir_combat_triggered`, that
  the map/event system can react to
- The peaceful branch sets `eonir_relic_returned` and resolves the quest

#### Step 3 — Map event: NPC → monster swap

- On the Frostspire Peaks map, add an event that watches for
  `eonir_combat_triggered`
- When the flag fires: despawn `tutorial_lich_eonir` NPC, spawn
  `lich_king_eonir` monster at the same tile, begin combat
- After combat ends (party wins): set `eonir_defeated`, remove the spawn
  point, trigger the relic-return quest step

#### Step 4 — Post-combat quest resolution

- Both the peaceful path (`eonir_relic_returned`) and the combat path
  (`eonir_defeated`) should converge on the same Act V quest step: the party
  carries the jade icosahedron back to Jyeshtha
- The NPC `tutorial_lich_eonir` should not reappear after either outcome

#### Out of scope for now

- Evil-playthrough support (attacking friendly NPCs, faction hostility) — this
  is tracked as a future Option B/C item
- Multiple boss phases — can be added to `lich_king_eonir` later without
  touching the NPC or dialogue layers

Write a plan with a phased approach to add the combat triggered NPC fight feature. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [NPC Combat Trigger Implementation Plan](./npc_combat_trigger_implementation_plan.md)

---

## Game Engine

### Audio RON and SFX Mapping

Introduce a per-campaign `audio.ron` file and extend the engine audio system so
that campaign authors can remap built-in SFX event IDs to arbitrary audio files
without renaming files or touching engine source code.

#### Step 1 — Define `audio.ron` data structure

Add a new `SfxMap` / `AudioManifest` type (exact name TBD, follow architecture
naming conventions) in `src/sdk/` (or wherever `AudioConfig` lives):

```ron
// campaigns/<name>/audio.ron
AudioManifest(
    // Remap well-known engine SFX IDs to campaign-specific files.
    // Values may include or omit the extension; bare names default to .ogg.
    sfx_mappings: {
        "combat_hit":      "sounds/heavy_impact.wav",
        "combat_miss":     "sounds/whoosh.ogg",
        "combat_heal":     "sounds/heal_chime.ogg",
        "spell_fizzle":    "sounds/fizzle.ogg",
        "victory_fanfare": "sounds/victory.mp3",
    },

    // Named music tracks (key = logical name, value = file inside assets/audio/).
    music_tracks: {
        "combat":      "music/battle_theme.ogg",
        "exploration": "music/overworld.ogg",
        "town":        "music/town_ambient.ogg",
    },
)
```

- File paths in the manifest are relative to the campaign's `assets/audio/`
  directory (same root `AudioPaths.audio_dir` already uses)
- Any ID not present in the manifest falls back to the existing
  filename-by-convention behaviour so old campaigns keep working

#### Step 2 — Load `audio.ron` at campaign startup

- Add `CampaignLoader` support for `audio.ron` (optional file; missing =
  empty manifest, full backwards compat)
- Insert the loaded manifest as a Bevy `Resource` (`AudioManifest`)

#### Step 3 — Update `resolve_audio_path` to consult the manifest

- Change the function signature to accept `Option<&AudioManifest>`
- If the manifest has an entry for the given ID, use that path; otherwise fall
  back to `{audio_dir}/{id}.ogg`
- Update callers in `handle_audio_messages` to pass the resource

#### Step 4 — Update `AudioConfig` (optional)

- Consider whether any volume overrides or metadata belong in `audio.ron`
  vs `config.ron`; keep volume settings in `config.ron` for now, content
  (file mappings) in `audio.ron`

#### Step 5 — Add to `data/test_campaign/`

- Add a minimal `audio.ron` (or explicitly test the missing-file fallback) to
  the test campaign fixture so the loader and resolver have coverage
- All new tests MUST use `data/test_campaign`, never `campaigns/tutorial`
  (Implementation Rule 5)

#### Affected files (expected)

| File | Change |
|------|--------|
| `src/sdk/game_config.rs` | Add `AudioManifest` struct + serde impl |
| `src/game/systems/audio.rs` | Update `resolve_audio_path`, `AudioPaths`, `handle_audio_messages` |
| `src/sdk/campaign_loader.rs` | Load optional `audio.ron` |
| `data/test_campaign/audio.ron` | New test fixture |
| `campaigns/config.template.ron` | Document that `audio.ron` is a separate file |

Write a plan with a phased approach to add the audio config feature to the game. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Audio RON SFX mapping Implementation Plan](./audio_ron_sfx_mapping_implementation_plan.md)

---

### Refactor `object_mesh_registry.ron` to ID-Based Format

The `object_mesh_registry.ron` file uses a bespoke string-keyed map format
that is inconsistent with every other mesh registry in the campaign and
blocks legitimate use cases (e.g. two distinct variants of the same mesh
cannot be registered because the key is the name and names collide).

#### Current (broken) format

```ron
// data/object_mesh_registry.ron  ← TODAY
(
    meshes: {
        "ancient_treasure_chest": "assets/furniture/seating/ancient_treasure_chest.ron",
        "barred_passage":         "assets/meshes/objects/barred_door.ron",
        "ironbound_treasure_chest": "assets/furniture/storage/ironbound_treasure_chest.ron",
        "weathered_wooden_hutch": "assets/furniture/storage/weathered_wooden_hutch.ron",
    },
)
```

Problems:
- String keys are **unique by name** — impossible to register two meshes that
  share the same concept (e.g. `ancient_treasure_chest_open` vs
  `ancient_treasure_chest_closed`)
- Differs structurally from every other registry; readers and the SDK must
  handle a one-off deserialisation path
- The string key has no canonical meaning — callers cobble together ad-hoc
  conventions for what the key represents

#### Target format (matches all other registries)

```ron
// data/object_mesh_registry.ron  ← TARGET
[
    (
        id: 12001,
        name: "Ancient Treasure Chest",
        filepath: "assets/furniture/seating/ancient_treasure_chest.ron",
    ),
    (
        id: 12002,
        name: "Barred Passage",
        filepath: "assets/meshes/objects/barred_door.ron",
    ),
    (
        id: 12003,
        name: "Ironbound Treasure Chest",
        filepath: "assets/furniture/storage/ironbound_treasure_chest.ron",
    ),
    (
        id: 12004,
        name: "Weathered Wooden Hutch",
        filepath: "assets/furniture/storage/weathered_wooden_hutch.ron",
    ),
]
```

ID range: **12000–12999** (object meshes).  Chosen to be above landscape
(11000-range) and furniture (10000-range) and below future expansion space.

#### Step 1 — Update the data file

- Convert `campaigns/tutorial/data/object_mesh_registry.ron` to the array
  format above, assigning ids starting at 12001
- Do the same for `data/test_campaign/data/object_mesh_registry.ron` if it
  exists, or create it in the new format

#### Step 2 — Replace the Rust types in `object_mesh.rs`

- Add a plain `ObjectMeshEntry { id: u32, name: String, filepath: String }`
  struct (mirrors `LandscapeMeshEntry`, `FurnitureMeshEntry`, etc.)
- Replace the private `ObjectMeshRegistry { meshes: HashMap<String,String> }`
  with a plain `Vec<ObjectMeshEntry>` (or a thin newtype)
- Replace `ObjectMeshRegistryFile { meshes: BTreeMap<String,String> }` with
  `ObjectMeshRegistryFile { entries: Vec<ObjectMeshEntry> }` — the `BTreeMap`
  was only needed to preserve key-sorted output; the array format is
  inherently ordered and naturally sorted by id
- Update `upsert`, `rename`, `remove` on `ObjectMeshRegistryFile` to operate
  on the `Vec` by id instead of by string key
- Update `ObjectMeshDatabase::load_from_registry` to iterate
  `Vec<ObjectMeshEntry>` and key each resolved mesh by its numeric id
  converted to a string (`"12001"`) — the same convention already used when
  merging landscape and furniture registries (they already convert their
  numeric ids to string keys, so lookup is already uniform)

#### Step 3 — Update callers in the engine

- `src/domain/campaign_loader.rs` — no signature change expected, but verify
  the deserialization path works with the new format and remove the `ron::Value`
  workaround in `ObjectMeshRegistryFile::load` (the struct-name mismatch it
  worked around goes away once the file is an array, not a named struct)
- Any map events that store `mesh_id` as a string name (e.g.
  `"ancient_treasure_chest"`) must be migrated to the numeric id string
  (`"12001"`); audit `data/maps/map_*.ron` for string-keyed object mesh
  references and update them

#### Step 4 — Update the SDK (Campaign Builder)

- `sdk/campaign_builder/src/campaign_io.rs` — update `load_objects` and
  `save_objects` to iterate `registry.entries` (Vec) instead of
  `registry.meshes` (BTreeMap)
- `sdk/campaign_builder/src/map_editor.rs` — `EventEditorState::treasure_mesh_id`
  is currently a free-form String (the old string key).  Change the map event
  editor's mesh picker to a dropdown populated from the numeric id list so
  authors pick by name and the id is stored — same UX as the furniture picker
- `sdk/campaign_builder/src/objects_editor.rs` — update `ObjectEntry` to carry
  `id: u32` and `name: String` in addition to `file_path`; the Objects editor
  tab should display id and name and allow editing the name field
- Update all SDK tests that construct `ObjectMeshRegistryFile` directly (they
  call `registry.upsert(key, path)` today) to use the new entry-based API

#### Step 5 — Tests and test fixtures

- All new and updated tests MUST use `data/test_campaign`, never
  `campaigns/tutorial` (Implementation Rule 5)
- Add or update `data/test_campaign/data/object_mesh_registry.ron` in the
  new array format
- Cover: round-trip load/save, `upsert`/`remove` by id, lookup by numeric-id
  string key, migration fallback if any old-format file is encountered

#### Affected files (expected)

| File | Change |
|------|--------|
| `campaigns/tutorial/data/object_mesh_registry.ron` | Convert to array format; assign ids 12001+ |
| `data/test_campaign/data/object_mesh_registry.ron` | Create / update in new format |
| `src/domain/world/object_mesh.rs` | New `ObjectMeshEntry`; replace `HashMap`/`BTreeMap` types |
| `src/domain/campaign_loader.rs` | Verify load path; drop `ron::Value` workaround |
| `campaigns/tutorial/data/maps/map_*.ron` | Migrate any string-keyed mesh refs to numeric ids |
| `sdk/campaign_builder/src/campaign_io.rs` | Use `entries` Vec in load/save |
| `sdk/campaign_builder/src/map_editor.rs` | Switch mesh picker to id-based dropdown |
| `sdk/campaign_builder/src/objects_editor.rs` | Carry `id`+`name` on `ObjectEntry` |

Write a plan with a phased approach to refactor the object mesh registry. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Refactor Object Mesh Registry RON to ID-Based Format](./object_mesh_registry_refactor_implementation_plan.md)

---

### Game Tray Icon Implementation Plan

We need to add a tray icon for the game like the ones we added for the SDK.

✅ PLAN WRITTEN - [Game Tray Icon Implementation Plan](./game_tray_icon_implementation_plan.md)

## Bugs

Portraits should support jpg images.
