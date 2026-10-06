# Audio RON and SFX Mapping Implementation Plan

## Overview

Introduce a per-campaign `audio.ron` file and an `AudioManifest` type so
campaign authors can remap built-in engine SFX/music event IDs to
arbitrarily-named audio files.  The manifest is loaded at campaign startup as a
Bevy resource; `resolve_audio_path` is updated to consult it before falling back
to the existing filename-by-convention path.  All existing campaigns that lack
`audio.ron` continue to work unchanged.

This is the **game-engine side** of the audio feature.  The SDK Campaign Builder
UI (Audio tab, import panel, mapping tables) is tracked in
`docs/explanation/audio_manager_implementation_plan.md`, which depends on the
`AudioManifest` type defined here.

> **Naming note**: The companion SDK plan uses `AudioMap`.  These names must be
> reconciled before implementation begins.  `AudioManifest` is preferred because
> it is more specific and sits alongside `AudioConfig` in `src/sdk/game_config.rs`.
> The SDK plan should be updated to use `AudioManifest`, `sfx_mappings`, and
> `music_tracks` to match this plan.

---

## Current State Analysis

### Existing Infrastructure

| Symbol | Location | Notes |
|--------|----------|-------|
| `AudioPlugin { config, audio_dir }` | `src/game/systems/audio.rs` | Bevy plugin; inserts `AudioSettings` + `AudioPaths` |
| `AudioPaths { audio_dir: String }` | `src/game/systems/audio.rs` | Bevy resource; holds the campaign audio directory |
| `resolve_audio_path(audio_dir, id)` | `src/game/systems/audio.rs` | Appends `.ogg` if no extension |
| `handle_audio_messages` | `src/game/systems/audio.rs` | Reads `PlaySfx`/`PlayMusic`; calls `resolve_audio_path` |
| `AudioConfig` (volume sliders) | `src/sdk/game_config.rs` | Lives in `GameConfig`; loaded from `config.ron` |
| `GameConfig::load_or_default` | `src/sdk/game_config.rs` | Pattern for loading root-level config files |
| `Campaign::load()` | `src/sdk/campaign_loader.rs` | Loads `campaign.ron` then `config.ron` separately |
| `Campaign::game_config` | `src/sdk/campaign_loader.rs` | `#[serde(skip)]` field populated post-load |

### Hardcoded Engine Event IDs (`src/game/systems/combat.rs`)

| Kind | Engine ID | Callsite |
|------|-----------|---------|
| SFX | `combat_hit` | `handle_attack_action` (×3), `handle_cast_spell_action` |
| SFX | `combat_miss` | `handle_attack_action`, `handle_cast_spell_action` |
| SFX | `combat_heal` | `handle_use_item_action` (×2) |
| SFX | `spell_fizzle` | `handle_cast_spell_action` |
| SFX | `victory_fanfare` | `handle_combat_victory` |
| Music | `combat_theme` | `handle_combat_started` |
| Music | `exploration_theme` | `handle_combat_victory` |

### Identified Issues

1. **No manifest layer**: SFX/music IDs are resolved directly as filenames; no
   way to use custom-named assets without renaming them.
2. **`audio.ron` does not exist**: `Campaign` has no field or load path for it.
3. **`resolve_audio_path` is not manifest-aware**: hardcoded to use `id` as the
   path stem.
4. **`AudioPlugin` cannot accept a manifest**: its constructor only takes
   `AudioConfig` and `audio_dir`.

---

## Implementation Phases

### Phase 1: Define `AudioManifest` in `src/sdk/game_config.rs`

Place the new type alongside `AudioConfig` so all audio-related SDK config lives
in one file.

#### 1.1 `AudioManifest` Struct

Add `pub struct AudioManifest` in `src/sdk/game_config.rs` immediately after
`AudioConfig`:

- `pub sfx_mappings: BTreeMap<String, String>` — engine SFX event ID →
  audio filename (relative to `audio_dir`; extension optional)
- `pub music_tracks: BTreeMap<String, String>` — music track ID → audio
  filename (relative to `audio_dir`; extension optional)
- Derive `Debug, Clone, Default, Serialize, Deserialize, PartialEq`

`impl AudioManifest`:

- `pub fn resolve_sfx(&self, engine_id: &str) -> Option<&str>` — returns the
  mapped value if the key exists
- `pub fn resolve_music(&self, track_id: &str) -> Option<&str>` — same for
  music tracks
- `pub fn well_known_sfx_ids() -> &'static [&'static str]` — returns the five
  hardcoded SFX engine IDs from the table above (used by the SDK to seed the
  mapping table)
- `pub fn well_known_music_ids() -> &'static [&'static str]` — returns the two
  hardcoded music track IDs

Re-export `AudioManifest` from `src/sdk/mod.rs` if that module re-exports other
SDK types.

#### 1.2 RON Format

The canonical `audio.ron` format is:

```
// campaigns/<name>/audio.ron
AudioManifest(
    sfx_mappings: {
        "combat_hit":      "sounds/heavy_impact.wav",
        "combat_miss":     "sounds/whoosh.ogg",
        "combat_heal":     "sounds/heal_chime.ogg",
        "spell_fizzle":    "sounds/fizzle.ogg",
        "victory_fanfare": "sounds/victory.mp3",
    },
    music_tracks: {
        "combat":      "music/battle_theme.ogg",
        "exploration": "music/overworld.ogg",
        "town":        "music/town_ambient.ogg",
    },
)
```

Values are relative to the campaign's `audio_dir` (`assets/audio/` by default).
Existing `resolve_audio_path` extension-handling logic applies to mapped values
just as it does to bare IDs.

#### 1.3 Testing Requirements

- `test_audio_manifest_resolve_sfx_returns_mapped_value_when_present`
- `test_audio_manifest_resolve_sfx_returns_none_when_absent`
- `test_audio_manifest_resolve_music_returns_mapped_value_when_present`
- `test_audio_manifest_resolve_music_returns_none_when_absent`
- `test_audio_manifest_well_known_sfx_ids_contains_five_entries` — asserts all
  five hardcoded SFX IDs are present
- `test_audio_manifest_well_known_music_ids_contains_two_entries`
- Doctest on `AudioManifest` showing RON round-trip using `ron::to_string` /
  `ron::from_str`
- `test_audio_manifest_default_is_empty_maps` — `AudioManifest::default()` has
  no entries in either map

#### 1.4 Deliverables

- [ ] `AudioManifest` struct in `src/sdk/game_config.rs` with full `///` doc
      comments and examples
- [ ] All Phase 1 tests pass
- [ ] `cargo check --all-targets --all-features` clean

#### 1.5 Success Criteria

`ron::from_str::<AudioManifest>(...)` parses the canonical RON block above
without error; round-trip through `ron::to_string` and back produces an
equivalent value.

---

### Phase 2: Campaign Loader — Load `audio.ron` at Startup

Load `audio.ron` from the campaign root using the same optional-file pattern as
`config.ron`.  Missing file is not an error.

#### 2.1 Add `audio_manifest` Field to `Campaign`

In `src/sdk/campaign_loader.rs`, add to `pub struct Campaign`:

```
/// Loaded audio manifest (from `audio.ron` at campaign root).
/// `None` if the file does not exist (full backwards compatibility).
#[serde(skip)]
pub audio_manifest: Option<AudioManifest>,
```

The `#[serde(skip)]` matches the pattern of `game_config`.

#### 2.2 Load in `Campaign::load()`

After the `config.ron` load block (both the `CampaignMetadata` and `Campaign`
paths), add:

```
let audio_path = path.join("audio.ron");
campaign.audio_manifest = if audio_path.exists() {
    let contents = std::fs::read_to_string(&audio_path)
        .map_err(|e| CampaignLoadError::MetadataError(
            format!("Failed to read audio.ron: {}", e)
        ))?;
    Some(ron::from_str::<AudioManifest>(&contents)
        .map_err(|e| CampaignLoadError::MetadataError(
            format!("Failed to parse audio.ron: {}", e)
        ))?)
} else {
    tracing::debug!("No audio.ron found at {:?}; using filename-by-convention", audio_path);
    None
};
```

This mirrors exactly how `GameConfig::load_or_default` is called but uses an
explicit `Option` rather than a fallback default, so callers can distinguish
"no file" from "file with empty mappings".

#### 2.3 Update `CampaignLoader` (if distinct from `Campaign::load`)

If a `CampaignLoader` struct wraps `Campaign::load`, ensure it propagates
`audio_manifest` unchanged.  Verify by grepping for `CampaignLoader::new` and
`CampaignLoader::load_campaign`.

#### 2.4 Testing Requirements

- `test_campaign_load_without_audio_ron_sets_manifest_to_none` — loads
  `data/test_campaign` (which does not yet have `audio.ron`); asserts
  `campaign.audio_manifest.is_none()`.
- `test_campaign_load_with_valid_audio_ron_populates_manifest` — adds a
  temporary `audio.ron` beside the test campaign fixture; asserts mappings
  loaded correctly.
- `test_campaign_load_with_malformed_audio_ron_returns_error` — provides a
  syntactically invalid `audio.ron`; asserts `Err(CampaignLoadError::MetadataError)`.

All tests use `data/test_campaign`, never `campaigns/tutorial`
(Implementation Rule 5).

#### 2.5 Deliverables

- [ ] `Campaign::audio_manifest: Option<AudioManifest>` field added
- [ ] `Campaign::load()` loads and parses `audio.ron` when present
- [ ] Missing `audio.ron` is debug-logged and stored as `None`
- [ ] All Phase 2 tests pass

#### 2.6 Success Criteria

`cargo nextest run --all-features` passes all existing campaign-loader tests
unmodified.  New tests confirm optional loading in both present/absent cases.

---

### Phase 3: Bevy Integration — `AudioManifestResource` and Updated `resolve_audio_path`

Wire the loaded manifest into the audio pipeline.

#### 3.1 Add `AudioManifestResource` Bevy Resource

In `src/game/systems/audio.rs`, add:

```rust
/// Bevy resource holding the campaign's audio manifest, if one was loaded.
/// `None` means filename-by-convention is used for all IDs.
#[derive(Resource, Default, Clone)]
pub struct AudioManifestResource(pub Option<AudioManifest>);
```

#### 3.2 Update `AudioPlugin`

Add `pub audio_manifest: Option<AudioManifest>` field to `AudioPlugin`.  In
`AudioPlugin::build()`, insert `AudioManifestResource(self.audio_manifest.clone())`:

```rust
app.insert_resource(settings)
   .insert_resource(paths)
   .insert_resource(AudioManifestResource(self.audio_manifest.clone()))
   .init_resource::<CurrentMusicTrack>()
   ...
```

#### 3.3 Update `resolve_audio_path`

Change the signature to accept an `Option<&AudioManifest>` and perform the
lookup before constructing the path:

```
pub fn resolve_audio_path(
    audio_dir: &str,
    id: &str,
    manifest: Option<&AudioManifest>,
) -> String
```

Resolution order:
1. If `manifest.is_some()` and `manifest.resolve_sfx(id).is_some()`, use the
   mapped value as the effective `id` (extension handling still applies).
2. Otherwise, use `id` as the effective filename (current behaviour).
3. Combine with `audio_dir` as before.

`resolve_music` works the same way — callers pass the same manifest for both
SFX and music resolution; the distinction is which lookup method is called.

**Caller update**: every call site of `resolve_audio_path` must be updated to
pass the third argument.  There are currently two call sites: the two `info!`
log lines and the two `let asset_path = …` lines inside `handle_audio_messages`.
Both are in `src/game/systems/audio.rs`.

#### 3.4 Update `handle_audio_messages`

Add `manifest: Res<AudioManifestResource>` to the system parameter list.
Pass `manifest.0.as_ref()` to `resolve_audio_path` for both music and SFX
resolution:

```rust
// Music
let effective_id = manifest.0.as_ref()
    .and_then(|m| m.resolve_music(&ev.track_id))
    .unwrap_or(&ev.track_id);
let asset_path = resolve_audio_path(&paths.audio_dir, effective_id, None);
// (manifest already consulted above; pass None to avoid double-lookup)
```

#### 3.5 Update `src/bin/antares.rs`

After loading the campaign, pass `campaign.audio_manifest.clone()` into
`AudioPlugin`:

```rust
.add_plugins(antares::game::systems::audio::AudioPlugin {
    config: audio_config,
    audio_dir,
    audio_manifest: campaign.audio_manifest.clone(),
})
```

#### 3.6 Testing Requirements

- `test_resolve_audio_path_with_manifest_uses_mapped_value` — passes an
  `AudioManifest` with `"combat_hit" → "sounds/heavy_impact.wav"` and asserts
  the result is `"assets/audio/sounds/heavy_impact.wav"`.
- `test_resolve_audio_path_without_manifest_falls_back_to_id` — passes `None`
  and asserts the old `{audio_dir}/{id}.ogg` result.
- `test_resolve_audio_path_with_manifest_falls_back_when_id_not_in_map` —
  manifest present but ID absent; asserts filename-by-convention result.
- `test_resolve_audio_path_manifest_mapped_value_with_extension_preserved` —
  manifest value already has `.mp3`; asserts no `.ogg` appended.
- Update all five existing `test_resolve_audio_path_*` tests to pass `None` as
  the third argument.
- `test_audio_plugin_with_manifest_inserts_resource` — Bevy app test; asserts
  `AudioManifestResource` present after plugin build.

#### 3.7 Deliverables

- [ ] `AudioManifestResource` added to `src/game/systems/audio.rs`
- [ ] `AudioPlugin::audio_manifest` field added
- [ ] `resolve_audio_path` accepts `Option<&AudioManifest>` (third parameter)
- [ ] All five existing `test_resolve_audio_path_*` tests updated to pass `None`
- [ ] `handle_audio_messages` queries `AudioManifestResource`
- [ ] `src/bin/antares.rs` passes manifest to `AudioPlugin`
- [ ] All new and existing audio tests pass

#### 3.8 Success Criteria

With a campaign `audio.ron` mapping `"combat_hit"` to `"sounds/hit.wav"`, the
game resolves the path to `assets/audio/sounds/hit.wav`.  Without `audio.ron`,
the path is `assets/audio/combat_hit.ogg` (unchanged from today).

---

### Phase 4: Test Fixtures and Integration Tests

Add the `data/test_campaign/audio.ron` fixture and write loader/resolver
integration tests.

#### 4.1 Add `data/test_campaign/audio.ron`

Create `data/test_campaign/audio.ron` at the campaign root (alongside
`data/test_campaign/campaign.ron` and `data/test_campaign/config.ron`) with a
minimal but valid manifest that remaps at least one SFX and one music track:

```ron
AudioManifest(
    sfx_mappings: {
        "combat_hit": "test_hit.ogg",
    },
    music_tracks: {
        "combat_theme": "test_battle.ogg",
    },
)
```

This file is NOT added to `campaigns/tutorial/` (Implementation Rule 5).

#### 4.2 Integration Tests

Add to `src/sdk/campaign_loader.rs` tests (or a dedicated integration test in
`tests/`):

- `test_campaign_load_test_campaign_populates_audio_manifest` — loads
  `data/test_campaign` via `Campaign::load()`; asserts
  `audio_manifest.is_some()` and that `resolve_sfx("combat_hit")` returns
  `Some("test_hit.ogg")`.
- `test_audio_manifest_missing_file_gives_none` — loads `data/test_campaign`
  after temporarily renaming `audio.ron`; asserts `audio_manifest.is_none()`.
  (Alternatively: use a second fixture dir without `audio.ron`.)

#### 4.3 Update `campaigns/config.template.ron` Documentation

In the campaign template file (if it exists), add a comment block documenting
that `audio.ron` is an optional root-level companion file for SFX/music
remapping, with a link to the full format description.

#### 4.4 Deliverables

- [ ] `data/test_campaign/audio.ron` created with minimal valid manifest
- [ ] Integration tests for present/absent manifest pass
- [ ] `config.template.ron` updated with `audio.ron` documentation comment
- [ ] `docs/explanation/implementations.md` updated with implementation summary
- [ ] `cargo nextest run --all-features` fully green

#### 4.5 Success Criteria

Zero references to `campaigns/tutorial` in any new or modified test.  All
quality gates pass: `cargo fmt --all`, `cargo check`, `cargo clippy -- -D warnings`,
`cargo nextest run --all-features`.

---

## Affected Files Summary

| File | Phase | Change |
|------|-------|--------|
| `src/sdk/game_config.rs` | 1 | Add `AudioManifest` struct, `resolve_sfx`, `resolve_music`, `well_known_*` methods |
| `src/sdk/campaign_loader.rs` | 2 | Add `Campaign::audio_manifest`; load `audio.ron` in `Campaign::load()` |
| `src/game/systems/audio.rs` | 3 | Add `AudioManifestResource`; update `AudioPlugin`; update `resolve_audio_path` signature; update `handle_audio_messages` |
| `src/bin/antares.rs` | 3 | Pass `campaign.audio_manifest` to `AudioPlugin` |
| `data/test_campaign/audio.ron` | 4 | **New** — minimal manifest fixture |
| `docs/explanation/implementations.md` | 4 | Implementation summary |

---

## Decisions Log

| Question | Decision |
|----------|----------|
| Where does `AudioManifest` live? | `src/sdk/game_config.rs` alongside `AudioConfig` |
| Is `audio.ron` in `data/` or campaign root? | Campaign root (parallel to `config.ron`), not in `data/` subdirectory |
| Missing `audio.ron` behaviour | `None` (debug log only); filename-by-convention path fully preserved |
| `resolve_audio_path` signature | Add third parameter `Option<&AudioManifest>`; all existing callers pass `None` |
| Music vs SFX lookup in `handle_audio_messages` | Caller resolves via manifest before passing to `resolve_audio_path`; avoids double-dispatch |
| Volume settings location | Remain in `config.ron` / `AudioConfig`; `audio.ron` is content-only (file mappings) |
