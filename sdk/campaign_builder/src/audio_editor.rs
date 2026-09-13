// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Audio editor state for the Campaign Builder SDK.
//!
//! [`AudioEditorState`] manages in-memory representation of the audio mapping
//! table that is serialised as `data/audio.ron`. It provides methods to load
//! from and save to [`antares::domain::AudioMap`], refresh the list of audio
//! files present in the campaign's `assets/audio/` directory, and import new
//! audio files.

use antares::domain::AudioMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// One row in the SFX mapping table.
///
/// `engine_id` is a well-known engine event identifier (e.g. `"combat_hit"`).
/// `mapped_file` is the audio filename the campaign maps it to; empty string
/// means "use filename-by-convention".
#[derive(Debug, Clone, PartialEq)]
pub struct SfxMappingRow {
    /// Well-known engine SFX event ID.
    pub engine_id: String,
    /// Audio filename mapped to this ID, relative to `assets/audio/`.
    /// Empty string = filename-by-convention (no override).
    pub mapped_file: String,
}

/// One row in the music-track mapping table.
///
/// `track_id` is a well-known engine music track identifier (e.g. `"combat_theme"`).
/// `mapped_file` is the audio filename the campaign maps it to; empty string
/// means "use filename-by-convention".
#[derive(Debug, Clone, PartialEq)]
pub struct MusicMappingRow {
    /// Well-known engine music track ID.
    pub track_id: String,
    /// Audio filename mapped to this ID, relative to `assets/audio/`.
    /// Empty string = filename-by-convention (no override).
    pub mapped_file: String,
}

/// Errors that can occur when importing an audio file into a campaign.
#[derive(Debug, Error)]
pub enum AudioImportError {
    /// The source file has an unsupported extension (must be `.ogg`, `.mp3`, `.wav`, or `.flac`).
    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),

    /// The file could not be copied to the campaign audio directory.
    #[error("Failed to copy file: {0}")]
    CopyFailed(String),

    /// No campaign directory is set; cannot determine the destination.
    #[error("No campaign directory set")]
    NoCampaignDir,
}

/// All transient state for the Audio tab in the Campaign Builder.
///
/// Holds the mapping rows for SFX events and music tracks, the list of audio
/// files discovered in `assets/audio/`, and bookkeeping flags.
#[derive(Debug)]
pub struct AudioEditorState {
    /// SFX event ID → filename mapping rows.
    pub sfx_mappings: Vec<SfxMappingRow>,

    /// Music track ID → filename mapping rows.
    pub music_mappings: Vec<MusicMappingRow>,

    /// Filenames found in `assets/audio/` (refreshed on load and import).
    pub imported_files: Vec<String>,

    /// Index into `imported_files` for the preview player (`None` = nothing selected).
    pub selected_file: Option<usize>,

    /// Preview playback volume (0.0–1.0).
    pub preview_volume: f32,

    /// One-line status message shown in the Audio tab footer.
    pub status_message: String,

    /// `true` when the in-memory state differs from the last saved `data/audio.ron`.
    pub unsaved_changes: bool,

    /// Set to `true` when `data/audio.ron` was successfully loaded from disk.
    ///
    /// Guards [`save_audio`](crate::campaign_io) against overwriting a valid
    /// `data/audio.ron` with a default-empty map when the file was never read
    /// during this session (SDK Rule 17).
    pub loaded_from_file: bool,
}

impl AudioEditorState {
    /// Creates a new `AudioEditorState` pre-populated with one row per well-known
    /// engine event ID.
    ///
    /// SFX rows are seeded from [`antares::sdk::game_config::AudioManifest::well_known_sfx_ids`]
    /// and music rows from [`antares::sdk::game_config::AudioManifest::well_known_music_ids`].
    /// All `mapped_file` fields start as empty strings (filename-by-convention applies
    /// for all IDs until overridden by the campaign author).
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::audio_editor::AudioEditorState;
    ///
    /// let state = AudioEditorState::new();
    /// assert!(!state.sfx_mappings.is_empty());
    /// assert!(!state.music_mappings.is_empty());
    /// assert!(state.sfx_mappings.iter().all(|r| r.mapped_file.is_empty()));
    /// assert!(state.music_mappings.iter().all(|r| r.mapped_file.is_empty()));
    /// ```
    pub fn new() -> Self {
        let sfx_mappings = antares::sdk::game_config::AudioManifest::well_known_sfx_ids()
            .iter()
            .map(|id| SfxMappingRow {
                engine_id: id.to_string(),
                mapped_file: String::new(),
            })
            .collect();

        let music_mappings = antares::sdk::game_config::AudioManifest::well_known_music_ids()
            .iter()
            .map(|id| MusicMappingRow {
                track_id: id.to_string(),
                mapped_file: String::new(),
            })
            .collect();

        Self {
            sfx_mappings,
            music_mappings,
            imported_files: Vec::new(),
            selected_file: None,
            preview_volume: 1.0,
            status_message: String::new(),
            unsaved_changes: false,
            loaded_from_file: false,
        }
    }

    /// Populates mapping rows from an [`AudioMap`] loaded from `data/audio.ron`.
    ///
    /// For each [`SfxMappingRow`], the `engine_id` is looked up in `map.sfx`; if
    /// found, `mapped_file` is set to the stored filename; otherwise it is cleared.
    /// The same logic applies to each [`MusicMappingRow`] against `map.music`.
    ///
    /// # Arguments
    ///
    /// * `map` – The [`AudioMap`] to load from, typically deserialised from
    ///   `data/audio.ron`.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::audio_editor::AudioEditorState;
    /// use antares::domain::AudioMap;
    ///
    /// let mut state = AudioEditorState::new();
    /// let mut map = AudioMap::default();
    /// map.sfx.insert("combat_hit".to_string(), "sounds/impact.ogg".to_string());
    ///
    /// state.load_from_map(&map);
    ///
    /// let row = state.sfx_mappings.iter().find(|r| r.engine_id == "combat_hit").unwrap();
    /// assert_eq!(row.mapped_file, "sounds/impact.ogg");
    /// ```
    pub fn load_from_map(&mut self, map: &AudioMap) {
        for row in &mut self.sfx_mappings {
            row.mapped_file = map.sfx.get(&row.engine_id).cloned().unwrap_or_default();
        }
        for row in &mut self.music_mappings {
            row.mapped_file = map.music.get(&row.track_id).cloned().unwrap_or_default();
        }
    }

    /// Converts the current mapping rows into an [`AudioMap`] ready for serialisation.
    ///
    /// Only rows whose `mapped_file` is non-empty are included. Rows with an
    /// empty `mapped_file` use filename-by-convention and produce no entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::audio_editor::AudioEditorState;
    ///
    /// let mut state = AudioEditorState::new();
    /// state.sfx_mappings[0].mapped_file = "sounds/hit.ogg".to_string();
    ///
    /// let map = state.to_audio_map();
    /// assert_eq!(map.sfx.len(), 1);
    /// assert!(map.music.is_empty());
    /// ```
    pub fn to_audio_map(&self) -> AudioMap {
        let mut map = AudioMap::default();
        for row in &self.sfx_mappings {
            if !row.mapped_file.is_empty() {
                map.sfx
                    .insert(row.engine_id.clone(), row.mapped_file.clone());
            }
        }
        for row in &self.music_mappings {
            if !row.mapped_file.is_empty() {
                map.music
                    .insert(row.track_id.clone(), row.mapped_file.clone());
            }
        }
        map
    }

    /// Scans `campaign_dir/assets/audio/` and repopulates [`Self::imported_files`]
    /// with the filenames of supported audio files found there.
    ///
    /// Supported extensions are `.ogg`, `.mp3`, `.wav`, and `.flac` (checked
    /// case-insensitively). The resulting list is sorted alphabetically.
    /// If the directory does not exist, the list is cleared without error.
    ///
    /// # Arguments
    ///
    /// * `campaign_dir` – Root directory of the campaign (the directory that
    ///   contains `campaign.ron`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::audio_editor::AudioEditorState;
    /// use std::path::Path;
    ///
    /// let mut state = AudioEditorState::new();
    /// state.refresh_imported_files(Path::new("campaigns/my_campaign"));
    /// println!("Found {} audio files", state.imported_files.len());
    /// ```
    pub fn refresh_imported_files(&mut self, campaign_dir: &Path) {
        self.imported_files.clear();
        let audio_dir: PathBuf = campaign_dir.join("assets/audio");
        if let Ok(entries) = fs::read_dir(&audio_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase())
                    .unwrap_or_default();
                if matches!(ext.as_str(), "ogg" | "mp3" | "wav" | "flac") {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        self.imported_files.push(name.to_string());
                    }
                }
            }
        }
        self.imported_files.sort();
    }

    /// Copies an audio file into the campaign's `assets/audio/` directory and
    /// registers it in [`Self::imported_files`].
    ///
    /// The source file must have a supported extension (`.ogg`, `.mp3`, `.wav`,
    /// or `.flac`); any other extension returns
    /// [`AudioImportError::UnsupportedFormat`]. The destination directory is
    /// created if it does not already exist. After a successful copy,
    /// [`Self::unsaved_changes`] is set to `true`.
    ///
    /// # Arguments
    ///
    /// * `src` – Path to the audio file to import.
    /// * `campaign_dir` – Root directory of the campaign.
    ///
    /// # Errors
    ///
    /// Returns [`AudioImportError::UnsupportedFormat`] if the source extension is
    /// not recognised, or [`AudioImportError::CopyFailed`] if the directory could
    /// not be created or the file could not be copied.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::audio_editor::AudioEditorState;
    /// use std::path::Path;
    ///
    /// let mut state = AudioEditorState::new();
    /// state
    ///     .import_audio_file(Path::new("/tmp/effect.ogg"), Path::new("campaigns/my_campaign"))
    ///     .expect("import succeeded");
    /// assert!(state.imported_files.contains(&"effect.ogg".to_string()));
    /// ```
    pub fn import_audio_file(
        &mut self,
        src: &Path,
        campaign_dir: &Path,
    ) -> Result<(), AudioImportError> {
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if !matches!(ext.as_str(), "ogg" | "mp3" | "wav" | "flac") {
            return Err(AudioImportError::UnsupportedFormat(format!(
                "{}: must be .ogg, .mp3, .wav, or .flac",
                src.display()
            )));
        }

        let audio_dir: PathBuf = campaign_dir.join("assets/audio");
        fs::create_dir_all(&audio_dir).map_err(|e| AudioImportError::CopyFailed(e.to_string()))?;

        let filename = src
            .file_name()
            .ok_or_else(|| AudioImportError::CopyFailed("Invalid source path".to_string()))?;

        let dest = audio_dir.join(filename);
        fs::copy(src, &dest).map_err(|e| AudioImportError::CopyFailed(e.to_string()))?;

        let filename_str = filename.to_string_lossy().to_string();
        if !self.imported_files.contains(&filename_str) {
            self.imported_files.push(filename_str);
            self.imported_files.sort();
        }
        self.unsaved_changes = true;
        Ok(())
    }
}

impl Default for AudioEditorState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_editor_state_new_contains_all_engine_sfx_ids() {
        let state = AudioEditorState::new();
        let sfx_ids: Vec<&str> = state
            .sfx_mappings
            .iter()
            .map(|r| r.engine_id.as_str())
            .collect();
        for id in antares::sdk::game_config::AudioManifest::well_known_sfx_ids() {
            assert!(
                sfx_ids.contains(id),
                "sfx_mappings missing engine ID: {}",
                id
            );
        }
        let music_ids: Vec<&str> = state
            .music_mappings
            .iter()
            .map(|r| r.track_id.as_str())
            .collect();
        for id in antares::sdk::game_config::AudioManifest::well_known_music_ids() {
            assert!(
                music_ids.contains(id),
                "music_mappings missing track ID: {}",
                id
            );
        }
        // Total: 7 well-known engine event IDs (5 SFX + 2 music)
        assert_eq!(state.sfx_mappings.len() + state.music_mappings.len(), 7);
    }

    #[test]
    fn test_load_from_map_fills_mapped_file_fields() {
        let mut state = AudioEditorState::new();
        let mut map = AudioMap::default();
        map.sfx
            .insert("combat_hit".to_string(), "sounds/impact.ogg".to_string());
        map.music
            .insert("combat_theme".to_string(), "music/battle.ogg".to_string());

        state.load_from_map(&map);

        let sfx_row = state
            .sfx_mappings
            .iter()
            .find(|r| r.engine_id == "combat_hit")
            .expect("row");
        assert_eq!(sfx_row.mapped_file, "sounds/impact.ogg");

        let music_row = state
            .music_mappings
            .iter()
            .find(|r| r.track_id == "combat_theme")
            .expect("row");
        assert_eq!(music_row.mapped_file, "music/battle.ogg");

        // Unmapped IDs stay empty
        let miss_row = state
            .sfx_mappings
            .iter()
            .find(|r| r.engine_id == "combat_miss")
            .expect("row");
        assert!(miss_row.mapped_file.is_empty());
    }

    #[test]
    fn test_to_audio_map_skips_rows_with_empty_mapped_file() {
        let mut state = AudioEditorState::new();
        // Set one SFX mapping, leave all others empty
        state.sfx_mappings[0].mapped_file = "sounds/hit.ogg".to_string();

        let map = state.to_audio_map();
        assert_eq!(map.sfx.len(), 1, "only the non-empty row should appear");
        assert!(map.music.is_empty(), "no music overrides were set");
    }

    #[test]
    fn test_import_audio_file_copies_file_and_adds_to_imported_list() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let campaign_dir = tmp.path();

        // Create a source .ogg file
        let src_dir = tempfile::tempdir().expect("src tempdir");
        let src = src_dir.path().join("effect.ogg");
        std::fs::write(&src, b"fake ogg data").expect("write src");

        let mut state = AudioEditorState::new();
        state.import_audio_file(&src, campaign_dir).expect("import");

        assert!(state.imported_files.contains(&"effect.ogg".to_string()));
        assert!(state.unsaved_changes);
        assert!(campaign_dir.join("assets/audio/effect.ogg").exists());
    }

    #[test]
    fn test_import_audio_file_rejects_unsupported_extension() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("track.txt");
        std::fs::write(&src, b"not audio").expect("write");

        let mut state = AudioEditorState::new();
        let result = state.import_audio_file(&src, tmp.path());
        assert!(matches!(
            result,
            Err(AudioImportError::UnsupportedFormat(_))
        ));
    }

    #[test]
    fn test_refresh_imported_files_scans_directory() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let audio_dir = tmp.path().join("assets/audio");
        std::fs::create_dir_all(&audio_dir).expect("create dir");
        std::fs::write(audio_dir.join("hit.ogg"), b"fake").expect("write");
        std::fs::write(audio_dir.join("music.mp3"), b"fake").expect("write");
        std::fs::write(audio_dir.join("not_audio.txt"), b"text").expect("write");

        let mut state = AudioEditorState::new();
        state.refresh_imported_files(tmp.path());

        assert!(state.imported_files.contains(&"hit.ogg".to_string()));
        assert!(state.imported_files.contains(&"music.mp3".to_string()));
        assert!(!state.imported_files.contains(&"not_audio.txt".to_string()));
        assert_eq!(state.imported_files.len(), 2);
    }
}
