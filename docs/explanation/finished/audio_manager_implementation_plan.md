# Audio Manager Implementation Plan

## Overview

Add a campaign-level audio mapping layer to Antares so campaign authors can
point any well-known engine event ID (e.g. `"combat_hit"`) at an
arbitrarily-named audio file without touching engine source code.  This
requires a new `AudioMap` domain struct, a `data/audio.ron` data file, a
mapping resource in the game's Bevy audio pipeline, and a new **Audio** tab in
the SDK Campaign Builder with an import panel, SFX/music mapping tables, volume
preview sliders, and a file preview button.

The current audio pipeline is fully operational (Bevy `PlaySfx`/`PlayMusic`
messages, `AudioPlugin`, `resolve_audio_path`) — this plan layers a mapping
indirection on top without breaking any existing filename-by-convention
behaviour.

---

## Current State Analysis

### Existing Infrastructure

| Component | Location | Status |
|-----------|----------|--------|
| `AudioPlugin` / `AudioSettings` / `AudioPaths` | `src/game/systems/audio.rs` | ✅ Complete |
| `PlaySfx { sfx_id }` / `PlayMusic { track_id }` messages | `src/game/systems/audio.rs` | ✅ Complete |
| `resolve_audio_path(audio_dir, id)` | `src/game/systems/audio.rs` | ✅ Complete |
| `AudioConfig` (volume sliders in `config.ron`) | `src/sdk/game_config.rs` | ✅ Complete |
| Config Editor audio sliders | `sdk/.../config_editor.rs` | ✅ Complete |
| `CampaignAssets::audio` dir (`assets/audio`) | `src/sdk/campaign_loader.rs` | ✅ Complete |

### Hardcoded Engine Event IDs (all in `src/game/systems/combat.rs`)

| Kind  | Engine Event ID      | Current resolved path                       |
|-------|----------------------|---------------------------------------------|
| SFX   | `combat_hit`         | `assets/audio/combat_hit.ogg`               |
| SFX   | `combat_miss`        | `assets/audio/combat_miss.ogg`              |
| SFX   | `combat_heal`        | `assets/audio/combat_heal.ogg`              |
| SFX   | `spell_fizzle`       | `assets/audio/spell_fizzle.ogg`             |
| SFX   | `victory_fanfare`    | `assets/audio/victory_fanfare.ogg`          |
| Music | `combat_theme`       | `assets/audio/combat_theme.ogg`             |
| Music | `exploration_theme`  | `assets/audio/exploration_theme.ogg`        |

### Identified Issues

1. **No mapping layer** — `sfx_id` / `track_id` IS the filename.  Authors cannot
   use an arbitrarily-named file; they must rename their asset to match the engine ID.
2. **No `audio.ron` data file** — `CampaignMetadata` has no `audio_file` field;
   `CampaignData` has no `audio` field.
3. **No Audio tab in the SDK** — `EditorTab` has no `Audio` variant.
4. **Preview requires OS or in-process audio** — the SDK is an eframe app with no
   Bevy runtime; preview needs a separate playback mechanism (e.g. `rodio`).

---

## Implementation Phases

### Phase 1: Domain — `AudioMap` Struct and `audio.ron` Format

Define the data model first.  No behavior changes in this phase.

#### 1.1 New `src/domain/audio.rs` Module

Create `src/domain/audio.rs` with:

- `pub struct AudioMap` — the top-level serialisable type for `audio.ron`:
  - `pub sfx: BTreeMap<String, String>` — engine SFX event ID → audio filename
    (with or without extension; `resolve_audio_path` handles extension appending)
  - `pub music: BTreeMap<String, String>` — music track ID → audio filename
- `impl AudioMap`:
  - `pub fn resolve_sfx(&self, engine_id: &str) -> Option<&str>` — looks up a
    SFX event ID; returns the mapped filename or `None` (caller falls back to the
    ID-as-filename convention)
  - `pub fn resolve_music(&self, engine_id: &str) -> Option<&str>` — same for
    music tracks
  - `pub fn default_sfx_ids() -> &'static [&'static str]` — returns the seven
    well-known engine event IDs listed in the table above; used by the SDK to
    pre-populate the mapping table
- Derive `Debug, Clone, Default, Serialize, Deserialize` on `AudioMap`

Add `pub mod audio;` and `pub use audio::AudioMap;` to `src/domain/mod.rs`.

#### 1.2 Add `audio_file` to `CampaignMetadata` in `src/sdk/campaign_loader.rs`

Following the existing pattern for `furniture_file`, `landscape_file`, etc.:

- Add `#[serde(default = "default_audio_file_path")] pub audio_file: String` to
  `CampaignMetadata`.
- Add `fn default_audio_file_path() -> String { "data/audio.ron".to_string() }`.
- Add `pub audio: String` to `CampaignData` with the same default.
- Propagate `audio: metadata.audio_file` in `TryFrom<CampaignMetadata> for Campaign`
  and `impl Campaign::load`.

No existing campaign RON files change — `#[serde(default)]` ensures they
deserialise without the field.

#### 1.3 Testing Requirements

- Unit tests for `AudioMap`:
  - `test_resolve_sfx_returns_mapped_filename_when_present`
  - `test_resolve_sfx_returns_none_when_absent`
  - `test_resolve_music_returns_mapped_filename_when_present`
  - `test_default_sfx_ids_contains_all_seven_engine_events`
- Doctest on `AudioMap` showing RON round-trip (serialize + deserialize).
- `test_campaign_metadata_audio_file_defaults_to_data_audio_ron` — ensures
  `CampaignMetadata` without the field deserialises to `"data/audio.ron"`.

#### 1.4 Deliverables

- [ ] `src/domain/audio.rs` created with `AudioMap` struct
- [ ] `AudioMap` exported from `src/domain/mod.rs`
- [ ] `CampaignMetadata::audio_file` field added (serde default `"data/audio.ron"`)
- [ ] `CampaignData::audio` field added
- [ ] All new tests pass

#### 1.5 Success Criteria

`cargo check --all-targets --all-features` clean; existing campaign RON files
continue to load without modification.

---

### Phase 2: Game Integration — Load and Apply `AudioMap`

Wire the mapping into the Bevy audio pipeline.  Backward compatibility with the
filename-by-convention approach is fully preserved throughout.

#### 2.1 `AudioMapResource` Bevy Resource

In `src/game/systems/audio.rs`, add:

```
pub struct AudioMapResource(pub Option<AudioMap>);
```

`AudioPlugin` already receives `config` and `audio_dir` at startup.  Add an
`audio_map: Option<AudioMap>` field to `AudioPlugin` (default `None`).  In
`AudioPlugin::build()`, insert `AudioMapResource(self.audio_map.clone())`.

#### 2.2 Update `handle_audio_messages`

Before calling `resolve_audio_path` for each `PlaySfx` and `PlayMusic` event,
query `AudioMapResource` and substitute the mapped ID if one exists:

- SFX: if `audio_map.resolve_sfx(&ev.sfx_id)` returns `Some(mapped)`, use
  `mapped` as the path argument to `resolve_audio_path`; otherwise use `ev.sfx_id`.
- Music: same pattern with `resolve_music`.

This means a campaign with no `audio.ron` behaves exactly as today.

#### 2.3 Load `audio.ron` at Game Startup in `src/bin/antares.rs`

After loading the campaign (where `campaign.assets.audio` is already used for
`audio_dir`), attempt to load `{campaign_root}/{campaign.data.audio}` as RON
into an `AudioMap`.  Missing file is not an error — log at debug level and pass
`None`.  Pass the loaded `Option<AudioMap>` into `AudioPlugin`.

#### 2.4 Testing Requirements

- `test_audio_map_resource_resolves_sfx_through_mapping` — Bevy app test: insert
  an `AudioMapResource` with a custom SFX mapping, fire a `PlaySfx` message, and
  assert the spawned `AudioPlayer` uses the mapped filename.
- `test_audio_map_resource_falls_back_to_id_when_no_mapping` — same but with
  `AudioMapResource(None)`, confirms the old filename-by-convention path is taken.
- `test_audio_plugin_with_audio_map_inserts_resource`.

#### 2.5 Deliverables

- [ ] `AudioMapResource` added to `src/game/systems/audio.rs`
- [ ] `AudioPlugin::audio_map` field added
- [ ] `handle_audio_messages` consults `AudioMapResource` before resolving path
- [ ] `src/bin/antares.rs` loads `audio.ron` and passes map to `AudioPlugin`
- [ ] All new and existing audio tests pass

#### 2.6 Success Criteria

A campaign with `data/audio.ron` mapping `"combat_hit"` to `"sword_clang.ogg"`
causes the game to resolve the path as `assets/audio/sword_clang.ogg` instead of
`assets/audio/combat_hit.ogg`.  Campaigns without `audio.ron` are unaffected.

---

### Phase 3: SDK Data Model — `AudioEditorState`

Build the SDK-side state management for the Audio tab before touching any UI.

#### 3.1 New `sdk/campaign_builder/src/audio_editor.rs`

Create `audio_editor.rs` with:

- `pub struct SfxMappingRow` — `engine_id: String`, `mapped_file: String`
- `pub struct MusicMappingRow` — `track_id: String`, `mapped_file: String`
- `pub struct AudioEditorState`:
  - `pub sfx_mappings: Vec<SfxMappingRow>`
  - `pub music_mappings: Vec<MusicMappingRow>`
  - `pub imported_files: Vec<String>` — filenames present in `assets/audio/`
    within the open campaign dir (refreshed on load/import)
  - `pub selected_file: Option<usize>` — index into `imported_files` for preview
  - `pub preview_volume: f32` — standalone volume for the preview player (0.0–1.0)
  - `pub status_message: String`
  - `pub unsaved_changes: bool`
- `impl AudioEditorState`:
  - `pub fn new() -> Self` — pre-populates `sfx_mappings` and `music_mappings`
    with one row per engine event ID from `AudioMap::default_sfx_ids()`, with
    empty `mapped_file` strings
  - `pub fn load_from_map(&mut self, map: &AudioMap)` — merges a loaded
    `AudioMap` into the rows (preserves engine ID order; fills `mapped_file`
    from the map)
  - `pub fn to_audio_map(&self) -> AudioMap` — serialises non-empty rows back
    into an `AudioMap`
  - `pub fn refresh_imported_files(&mut self, campaign_dir: &Path)` — scans
    `assets/audio/` and repopulates `imported_files`
  - `pub fn import_audio_file(&mut self, src: &Path, campaign_dir: &Path) ->
    Result<(), AudioImportError>` — copies `src` into
    `campaign_dir/assets/audio/`, adds the filename to `imported_files`, sets
    `unsaved_changes = true`

- `pub enum AudioImportError` (thiserror):
  - `UnsupportedFormat(String)` — file is not `.ogg`, `.mp3`, `.wav`, or `.flac`
  - `CopyFailed(String)`
  - `NoCampaignDir`

Add `pub mod audio_editor;` to `sdk/campaign_builder/src/lib.rs`.
Add `pub audio_editor_state: audio_editor::AudioEditorState` to `EditorRegistry`
in `sdk/campaign_builder/src/editor_state.rs` and initialise it in
`impl Default for EditorRegistry`.

#### 3.2 Load / Save Integration in `campaign_io.rs`

Add `pub fn load_audio(&mut self)` and `pub fn save_audio(&mut self) ->
Result<(), CampaignIoError>` to `CampaignBuilderApp`, following the pattern of
`load_furniture` / `save_furniture`.  Missing `audio.ron` is not an error
(opt-in feature).  Call `load_audio()` from the campaign open path.

#### 3.3 Testing Requirements

- `test_audio_editor_state_new_contains_all_engine_sfx_ids`
- `test_load_from_map_fills_mapped_file_fields`
- `test_to_audio_map_skips_rows_with_empty_mapped_file`
- `test_import_audio_file_copies_file_and_adds_to_imported_list`
- `test_import_audio_file_rejects_unsupported_extension`
- `test_refresh_imported_files_scans_directory`

#### 3.4 Deliverables

- [ ] `sdk/campaign_builder/src/audio_editor.rs` created
- [ ] `AudioEditorState` added to `EditorRegistry`
- [ ] `load_audio()` / `save_audio()` added to `CampaignBuilderApp`
- [ ] All new tests pass

#### 3.5 Success Criteria

Opening a campaign with `data/audio.ron` populates `AudioEditorState` correctly.
Opening a campaign without `audio.ron` leaves the state at defaults (all rows
with empty `mapped_file`).  Saving produces a valid `audio.ron` that deserialises
back to an equivalent `AudioMap`.

---

### Phase 4: SDK UI — Audio Tab

Implement the Campaign Builder Audio tab following Implementation Rules 5a/5b
(egui ID audit + multi-column layout via `allocate_ui`).

#### 4.1 Add `EditorTab::Audio` to the Enum

In `sdk/campaign_builder/src/lib.rs`:

- Add `Audio` variant to `pub enum EditorTab` (place after `Config`, near the
  asset-management tabs).
- Add `EditorTab::Audio => "Audio"` to `impl EditorTab::name()`.
- Insert `EditorTab::Audio` in the sidebar `tabs` array at the same position.
- Add `EditorTab::Audio => self.show_audio_editor(ui)` to the central panel
  match.

#### 4.2 Implement `show_audio_editor` in a New `audio_editor_ui.rs` or Inline

Create `pub fn show_audio_editor(app: &mut CampaignBuilderApp, ui: &mut egui::Ui)`
(or as a method, matching the existing pattern for `show_metadata_editor`).

**Layout (three columns, `allocate_ui` pattern — Rule 6):**

```
┌─ Title bar: "🔊 Audio" [Tab] Scan   [ESC] hints right-aligned ─────────────┐
│                                                                              │
│ ┌─ Left 180px ──────┐ ┌─ Center (fill) ──────┐ ┌─ Right 260px ──────────┐ │
│ │ Imported Files    │ │ SFX Mappings          │ │ Music Tracks           │ │
│ │ ─────────────     │ │ ──────────────────    │ │ ──────────────────     │ │
│ │ [list of .ogg     │ │ Engine ID | File      │ │ Track ID | File        │ │
│ │  .mp3 .wav files] │ │ combat_hit | [picker] │ │ combat_theme | [p]     │ │
│ │                   │ │ combat_miss| [picker] │ │ exploration_theme|[p]  │ │
│ │ [Import File...]  │ │ ...                   │ │ ...                    │ │
│ │ [▶ Preview]       │ │ [+ Custom Row]        │ │ [+ Custom Track]       │ │
│ │ Volume: ──O──     │ │                       │ │                        │ │
│ └───────────────────┘ └───────────────────────┘ └────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Left column — Imported Files:**
- Scrollable list of `imported_files` entries; each row uses `ui.push_id(idx, …)`
- Clicking a row sets `selected_file = Some(idx)` and `request_repaint()`
- "Import File…" button — opens `rfd::FileDialog::new()` filtered to
  `["ogg", "mp3", "wav", "flac"]`; on selection calls
  `audio_editor_state.import_audio_file(path, campaign_dir)`; refreshes file list
- "▶ Preview" button — enabled only when `selected_file.is_some()`; triggers
  preview playback (Phase 5; shows "Not yet available" label until then)
- Preview volume slider: `egui::Slider::new(&mut state.preview_volume, 0.0..=1.0)`

**Center column — SFX Mappings:**
- `egui::ScrollArea::vertical().id_salt("audio_sfx_scroll").auto_shrink([true, false])`
- Each row: engine ID label (fixed-width) + `egui::ComboBox::from_id_salt(format!("sfx_combo_{i}"), …)` populated from `imported_files` + an optional clear button
- "Add Custom SFX Row" button appends a blank `SfxMappingRow`
- Any change sets `unsaved_changes = true` and calls `request_repaint()`

**Right column — Music Tracks:**
- Same pattern as center column but for `music_mappings`
- `egui::ScrollArea::vertical().id_salt("audio_music_scroll").auto_shrink([true, false])`
- Each row: `egui::ComboBox::from_id_salt(format!("music_combo_{i}"), …)`

**Volume preview sliders (bottom of right column or inline):**
- Mirror `AudioConfig` master/music/sfx/ambient sliders from the Config tab
- Read from `self.editor_registry.config_editor_state.game_config.audio`
- Changes save to `config.ron` via the existing config save path; do not
  duplicate state

**Save integration:**
- When `unsaved_changes` is true, the tab button shows a `*` indicator
  (match the pattern from other editors)
- On campaign save, `save_audio()` is called alongside other saves

#### 4.3 Testing Requirements

- `test_audio_tab_exists_in_editor_tab_enum`
- `test_audio_tab_appears_in_sidebar_tabs_array`
- `test_show_audio_editor_does_not_panic_with_empty_state` — uses a minimal
  `egui::Context` to call `show_audio_editor` without a real window; asserts no
  panic.
- `test_import_audio_file_invalid_extension_returns_error`

#### 4.4 Deliverables

- [ ] `EditorTab::Audio` added to enum, sidebar, and central panel match
- [ ] `show_audio_editor` renders without panic on an empty / loaded state
- [ ] Import button wired to `import_audio_file`
- [ ] SFX and music mapping tables render all engine event IDs
- [ ] Volume sliders displayed in right column
- [ ] ComboBoxes use `from_id_salt` (egui ID audit Rule 5a)
- [ ] Every loop body uses `push_id` (egui ID audit Rule 5a)
- [ ] Multi-column layout uses `allocate_ui` (Rule 6)
- [ ] All tests pass

#### 4.5 Success Criteria

A campaign author can open the Audio tab, import an `.ogg` file, map it to
`combat_hit`, save the campaign, and find a valid `data/audio.ron` on disk that
the game loads and applies correctly (verified via Phase 2 integration).

---

### Phase 5: Audio File Preview

Enable in-SDK playback of imported audio files so authors can audition sounds
without launching the full game.

#### 5.1 Add `rodio` Dependency to `sdk/campaign_builder/Cargo.toml`

Add `rodio = { version = "0.19", default-features = false, features = ["wav",
"ogg", "mp3", "flac"] }` to the SDK crate's `[dependencies]`.

This is a dev/SDK-only dependency — it does not affect the game binary.

#### 5.2 `AudioPreviewPlayer` in `audio_editor.rs`

Add `pub struct AudioPreviewPlayer` wrapping a `rodio::Sink` behind an
`Option<…>`.  Methods:

- `pub fn play(&mut self, path: &Path, volume: f32) -> Result<(), PreviewError>`
  — stops any currently playing audio, opens the file, decodes via rodio, sets
  sink volume, appends the source.
- `pub fn stop(&mut self)` — stops playback and drops the sink.
- `pub fn is_playing(&self) -> bool`

Add `preview_player: AudioPreviewPlayer` to `AudioEditorState`.

The "▶ Preview" button (Phase 4) calls `preview_player.play(path, volume)`.
When `is_playing()` is true, the button label changes to "⏹ Stop" and clicking
calls `preview_player.stop()`.

#### 5.3 Handling `request_repaint`

Rodio plays audio on a background thread; the egui frame will not refresh
automatically when playback ends.  Use `egui_ctx.request_repaint_after(
Duration::from_millis(500))` while `is_playing()` to keep the button state
current.

#### 5.4 Testing Requirements

- `test_preview_player_is_not_playing_by_default`
- `test_preview_player_stop_is_safe_when_not_playing`
- Integration test: `test_preview_player_plays_valid_wav_without_panic` —
  generates a minimal in-memory WAV (1 second of silence) and calls `play()`.

#### 5.5 Deliverables

- [ ] `rodio` added to `sdk/campaign_builder/Cargo.toml`
- [ ] `AudioPreviewPlayer` implemented in `audio_editor.rs`
- [ ] Preview button in Audio tab wired to `AudioPreviewPlayer`
- [ ] Button label toggles between ▶ Play and ⏹ Stop
- [ ] `request_repaint_after` keeps UI responsive during playback
- [ ] All preview tests pass

#### 5.6 Success Criteria

Selecting an `.ogg` file in the Audio tab and clicking ▶ Preview causes the file
to play through the system audio output.  Clicking ⏹ Stop halts playback.

---

## Affected Files Summary

| File | Phase | Change |
|------|-------|--------|
| `src/domain/audio.rs` | 1 | **New** — `AudioMap` struct |
| `src/domain/mod.rs` | 1 | Export `audio::AudioMap` |
| `src/sdk/campaign_loader.rs` | 1 | Add `audio_file` to `CampaignMetadata`; `audio` to `CampaignData` |
| `src/game/systems/audio.rs` | 2 | Add `AudioMapResource`, `AudioPlugin::audio_map`; update `handle_audio_messages` |
| `src/bin/antares.rs` | 2 | Load `audio.ron`; pass to `AudioPlugin` |
| `sdk/campaign_builder/src/audio_editor.rs` | 3, 5 | **New** — `AudioEditorState`, `AudioPreviewPlayer` |
| `sdk/campaign_builder/src/editor_state.rs` | 3 | Add `audio_editor_state` to `EditorRegistry` |
| `sdk/campaign_builder/src/campaign_io.rs` | 3 | Add `load_audio()` / `save_audio()` |
| `sdk/campaign_builder/src/lib.rs` | 4 | Add `EditorTab::Audio`; sidebar entry; central panel arm |
| `sdk/campaign_builder/Cargo.toml` | 5 | Add `rodio` dependency |
| `docs/explanation/implementations.md` | 4 | Add implementation summary |
| `data/test_campaign/data/audio.ron` | 3 | **New** test fixture — minimal `AudioMap` for integration tests |

---

## Decisions Log

| Question | Decision |
|----------|----------|
| Preview mechanism | `rodio` in SDK process (not OS open command, not Bevy) — gives volume control and stop button |
| `audio.ron` opt-in? | Yes — missing file is not an error; maps default to empty (ID-as-filename convention preserved) |
| Where custom SFX rows live | Rows with non-empty `engine_id` and `mapped_file` are written; blank rows are silently dropped on save |
| Music track IDs to seed | The two hardcoded engine track IDs: `combat_theme`, `exploration_theme` |
