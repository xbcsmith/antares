// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Audio System Foundation
//!
//! This module provides the foundation for the game's audio system.
//! Currently, it stores audio configuration as a Bevy resource for future use.
//!
//! The AudioSettings resource contains volume levels for different audio channels
//! (master, music, sfx, ambient) and an enabled flag. These settings are derived
//! from the campaign's AudioConfig but are not yet applied to actual audio playback.
//!
//! # Future Implementation
//!
//! When audio playback is implemented, systems should:
//! - Query the AudioSettings resource to get current volume levels
//! - Scale audio playback volumes by multiplying channel volume × master volume
//! - Check the `enabled` flag before playing any audio
//! - React to changes in AudioSettings for runtime volume adjustments
//!
//! # Examples
//!
//! ```rust
//! use antares::game::systems::audio::{AudioPlugin, AudioSettings};
//! use antares::sdk::game_config::AudioConfig;
//! use bevy::prelude::*;
//!
//! # fn example() {
//! let audio_config = AudioConfig {
//!     master_volume: 0.8,
//!     music_volume: 0.6,
//!     sfx_volume: 1.0,
//!     ambient_volume: 0.5,
//!     enable_audio: true,
//! };
//!
//! let mut app = App::new();
//! app.add_plugins(AudioPlugin {
//!     config: audio_config,
//!     audio_dir: "assets/audio".to_string(),
//!     audio_manifest: None,
//!     audio_map: None,
//! });
//! # }
//! ```

use crate::domain::AudioMap;
use crate::sdk::game_config::{AudioConfig, AudioManifest};
use bevy::audio::Volume;
use bevy::prelude::*;

/// Bevy resource holding the campaign's audio manifest, if one was loaded.
///
/// `None` means no `audio.ron` was found; all SFX and music IDs are resolved
/// by filename-by-convention (the engine ID used directly as the filename stem).
///
/// Inserted by [`AudioPlugin`] at startup; available to any Bevy system that
/// needs to resolve audio paths.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::audio::AudioManifestResource;
///
/// // Default: no manifest loaded
/// let resource = AudioManifestResource::default();
/// assert!(resource.0.is_none());
/// ```
#[derive(Resource, Default, Clone)]
pub struct AudioManifestResource(pub Option<AudioManifest>);

/// Bevy resource wrapping the domain-layer [`AudioMap`] for the loaded campaign.
///
/// `None` means no `data/audio.ron` was found; all IDs are resolved by
/// filename-by-convention.  Inserted by [`AudioPlugin`] at startup.
///
/// [`AudioMapResource`] is consulted first in [`handle_audio_messages`] before
/// falling back to [`AudioManifestResource`] and then to the bare event ID.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::audio::AudioMapResource;
///
/// let resource = AudioMapResource::default();
/// assert!(resource.0.is_none());
/// ```
#[derive(Resource, Default, Clone)]
pub struct AudioMapResource(pub Option<AudioMap>);

/// Resource that holds the resolved audio directory path for the loaded campaign.
///
/// Inserted by [`AudioPlugin`] at startup.  All music track IDs and SFX IDs
/// are resolved relative to this directory.  If a track/sfx identifier has no
/// recognised audio file extension (`.ogg`, `.mp3`, `.wav`, `.flac`), `.ogg`
/// is appended automatically.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::audio::AudioPaths;
///
/// let paths = AudioPaths { audio_dir: "assets/audio".to_string() };
/// assert_eq!(paths.audio_dir, "assets/audio");
/// ```
#[derive(Resource, Clone, Debug)]
pub struct AudioPaths {
    /// Directory (relative to campaign root) where all audio files live.
    pub audio_dir: String,
}

impl Default for AudioPaths {
    fn default() -> Self {
        Self {
            audio_dir: "assets/audio".to_string(),
        }
    }
}

/// Resolves a bare track/sfx identifier to a campaign-relative asset path,
/// optionally consulting an [`AudioManifest`] for a remapped filename.
///
/// # Resolution order
///
/// 1. If `manifest` is `Some` and contains a mapping for `id` via
///    [`AudioManifest::resolve_sfx`], the mapped value is used as the
///    effective filename.
/// 2. Otherwise `id` is used as the effective filename (existing behaviour).
/// 3. If the effective filename has no recognised audio extension (`.ogg`,
///    `.mp3`, `.wav`, `.flac`) then `.ogg` is appended.
/// 4. The result is `{audio_dir}/{effective_filename}`.
pub fn resolve_audio_path(audio_dir: &str, id: &str, manifest: Option<&AudioManifest>) -> String {
    // Strip any trailing slash so format! never produces "dir//file".
    let audio_dir = audio_dir.trim_end_matches('/');
    // Consult the manifest first; fall back to bare id.
    let effective_id = manifest.and_then(|m| m.resolve_sfx(id)).unwrap_or(id);
    let has_ext = effective_id.ends_with(".ogg")
        || effective_id.ends_with(".mp3")
        || effective_id.ends_with(".wav")
        || effective_id.ends_with(".flac");
    if has_ext {
        format!("{}/{}", audio_dir, effective_id)
    } else {
        format!("{}/{}.ogg", audio_dir, effective_id)
    }
}

/// Audio settings resource for runtime access
///
/// This resource holds the current audio configuration and is available
/// to all Bevy systems that need to access or modify audio settings.
///
/// All volume values are in the range 0.0-1.0, where:
/// - 0.0 = silent
/// - 1.0 = maximum volume
///
/// When playing audio, the effective volume for a sound should be calculated as:
/// `effective_volume = channel_volume * master_volume`
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::audio::AudioSettings;
/// use bevy::prelude::*;
///
/// fn example_system(audio_settings: Res<AudioSettings>) {
///     if audio_settings.enabled {
///         // Calculate effective music volume
///         let music_vol = audio_settings.music_volume * audio_settings.master_volume;
///         println!("Music playing at volume: {}", music_vol);
///     }
/// }
/// ```
#[derive(Resource, Clone, Debug)]
pub struct AudioSettings {
    /// Master volume control (0.0-1.0)
    ///
    /// This volume is multiplied with all other volumes to get the final
    /// playback volume. Set to 0.0 to mute all audio.
    pub master_volume: f32,

    /// Music channel volume (0.0-1.0)
    ///
    /// Controls background music and ambient music tracks.
    pub music_volume: f32,

    /// Sound effects channel volume (0.0-1.0)
    ///
    /// Controls gameplay sound effects like combat sounds, item pickups,
    /// footsteps, etc.
    pub sfx_volume: f32,

    /// Ambient sound channel volume (0.0-1.0)
    ///
    /// Controls environmental sounds like wind, water, crowd noise, etc.
    pub ambient_volume: f32,

    /// Audio system enabled flag
    ///
    /// When false, no audio should be played regardless of volume settings.
    /// This provides a quick way to disable all audio without changing volumes.
    pub enabled: bool,
}

impl AudioSettings {
    /// Create AudioSettings from AudioConfig
    ///
    /// # Examples
    ///
    /// ```rust
    /// use antares::game::systems::audio::AudioSettings;
    /// use antares::sdk::game_config::AudioConfig;
    ///
    /// let config = AudioConfig::default();
    /// let settings = AudioSettings::from_config(&config);
    ///
    /// assert_eq!(settings.master_volume, config.master_volume);
    /// assert_eq!(settings.enabled, config.enable_audio);
    /// ```
    pub fn from_config(config: &AudioConfig) -> Self {
        Self {
            master_volume: config.master_volume,
            music_volume: config.music_volume,
            sfx_volume: config.sfx_volume,
            ambient_volume: config.ambient_volume,
            enabled: config.enable_audio,
        }
    }

    /// Calculate effective volume for a channel
    ///
    /// Returns the final volume to use for audio playback by multiplying
    /// the channel volume with the master volume.
    ///
    /// # Arguments
    ///
    /// * `channel_volume` - The volume of the specific channel (music, sfx, or ambient)
    ///
    /// # Returns
    ///
    /// The effective volume (0.0-1.0) to use for playback
    ///
    /// # Examples
    ///
    /// ```rust
    /// use antares::game::systems::audio::AudioSettings;
    /// use antares::sdk::game_config::AudioConfig;
    ///
    /// let config = AudioConfig {
    ///     master_volume: 0.5,
    ///     music_volume: 0.8,
    ///     sfx_volume: 1.0,
    ///     ambient_volume: 0.6,
    ///     enable_audio: true,
    /// };
    ///
    /// let settings = AudioSettings::from_config(&config);
    ///
    /// // Music plays at 0.5 * 0.8 = 0.4
    /// assert_eq!(settings.effective_volume(settings.music_volume), 0.4);
    ///
    /// // SFX plays at 0.5 * 1.0 = 0.5
    /// assert_eq!(settings.effective_volume(settings.sfx_volume), 0.5);
    /// ```
    pub fn effective_volume(&self, channel_volume: f32) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        channel_volume * self.master_volume
    }

    /// Get effective music volume
    ///
    /// # Examples
    ///
    /// ```rust
    /// use antares::game::systems::audio::AudioSettings;
    /// use antares::sdk::game_config::AudioConfig;
    ///
    /// let config = AudioConfig {
    ///     master_volume: 0.5,
    ///     music_volume: 0.8,
    ///     sfx_volume: 1.0,
    ///     ambient_volume: 0.6,
    ///     enable_audio: true,
    /// };
    ///
    /// let settings = AudioSettings::from_config(&config);
    /// assert_eq!(settings.effective_music_volume(), 0.4);
    /// ```
    pub fn effective_music_volume(&self) -> f32 {
        self.effective_volume(self.music_volume)
    }

    /// Get effective SFX volume
    ///
    /// # Examples
    ///
    /// ```rust
    /// use antares::game::systems::audio::AudioSettings;
    /// use antares::sdk::game_config::AudioConfig;
    ///
    /// let config = AudioConfig {
    ///     master_volume: 0.5,
    ///     music_volume: 0.8,
    ///     sfx_volume: 1.0,
    ///     ambient_volume: 0.6,
    ///     enable_audio: true,
    /// };
    ///
    /// let settings = AudioSettings::from_config(&config);
    /// assert_eq!(settings.effective_sfx_volume(), 0.5);
    /// ```
    pub fn effective_sfx_volume(&self) -> f32 {
        self.effective_volume(self.sfx_volume)
    }

    /// Get effective ambient volume
    ///
    /// # Examples
    ///
    /// ```rust
    /// use antares::game::systems::audio::AudioSettings;
    /// use antares::sdk::game_config::AudioConfig;
    ///
    /// let config = AudioConfig {
    ///     master_volume: 0.5,
    ///     music_volume: 0.8,
    ///     sfx_volume: 1.0,
    ///     ambient_volume: 0.6,
    ///     enable_audio: true,
    /// };
    ///
    /// let settings = AudioSettings::from_config(&config);
    /// assert_eq!(settings.effective_ambient_volume(), 0.3);
    /// ```
    pub fn effective_ambient_volume(&self) -> f32 {
        self.effective_volume(self.ambient_volume)
    }
}

/// Audio system plugin
///
/// This plugin initializes the audio system by creating the AudioSettings
/// resource from the provided AudioConfig. The resource is then available
/// to all game systems that need to access or modify audio settings.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::audio::AudioPlugin;
/// use antares::sdk::game_config::AudioConfig;
/// use bevy::prelude::*;
///
/// # fn example() {
/// let audio_config = AudioConfig::default();
///
/// let mut app = App::new();
/// app.add_plugins(AudioPlugin {
///     config: audio_config,
///     audio_dir: "assets/audio".to_string(),
///     audio_manifest: None,
/// });
/// # }
/// ```
/// Message to request background music playback.
///
/// Other systems can listen for this message and perform appropriate track
/// changes (e.g., start combat music on combat start, restore exploration
/// music on combat end).
#[derive(Message, Clone)]
pub struct PlayMusic {
    /// Identifier for the music track (campaign-defined string)
    pub track_id: String,
    /// Whether the track should loop
    pub looped: bool,
}

/// Message to request a one-shot sound effect.
///
/// Systems that handle SFX should respect `AudioSettings` (volume/enabled).
#[derive(Message, Clone)]
pub struct PlaySfx {
    /// Identifier for the sound effect to play
    pub sfx_id: String,
}

/// Tracks the currently playing background music entity.
///
/// When a new `PlayMusic` message arrives, the old music entity (if any)
/// is despawned and replaced.
#[derive(Resource, Default)]
pub struct CurrentMusicTrack {
    /// Entity playing the current music, if any
    pub entity: Option<Entity>,
    /// Track ID of the currently playing music
    pub track_id: Option<String>,
}

/// Marker component for one-shot SFX entities.
///
/// Added alongside `AudioPlayer` so cleanup systems can identify
/// audio entities spawned by the audio subsystem.
#[derive(Component)]
pub struct SfxMarker;

/// Handles audio messages by spawning Bevy audio entities.
///
/// Listens for `PlayMusic` and `PlaySfx` messages and spawns
/// appropriate audio entities with the correct playback settings
/// and volume levels derived from `AudioSettings`.
#[allow(clippy::too_many_arguments)]
fn handle_audio_messages(
    mut music_reader: MessageReader<PlayMusic>,
    mut sfx_reader: MessageReader<PlaySfx>,
    settings: Res<AudioSettings>,
    paths: Res<AudioPaths>,
    manifest: Res<AudioManifestResource>,
    audio_map: Res<AudioMapResource>,
    asset_server: Option<Res<AssetServer>>,
    mut commands: Commands,
    mut current_music: ResMut<CurrentMusicTrack>,
) {
    // Handle music requests
    for ev in music_reader.read() {
        if !settings.enabled {
            debug!("Audio disabled; PlayMusic '{}' ignored", ev.track_id);
            continue;
        }

        let Some(ref server) = asset_server else {
            debug!(
                "No AssetServer available; PlayMusic '{}' deferred",
                ev.track_id
            );
            continue;
        };

        // Despawn the old music entity if one exists
        if let Some(old_entity) = current_music.entity.take() {
            commands.entity(old_entity).despawn();
        }

        let volume = settings.effective_music_volume();
        // Domain-layer map takes priority; SDK manifest is the fallback.
        let effective_track_id: String = audio_map
            .0
            .as_ref()
            .and_then(|m| m.resolve_music(&ev.track_id))
            .map(str::to_owned)
            .or_else(|| {
                manifest
                    .0
                    .as_ref()
                    .and_then(|m| m.resolve_music(&ev.track_id))
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| ev.track_id.clone());
        let asset_path = resolve_audio_path(&paths.audio_dir, &effective_track_id, None);
        let handle: Handle<AudioSource> = server.load(asset_path.clone());

        let playback = if ev.looped {
            PlaybackSettings::LOOP
        } else {
            PlaybackSettings::REMOVE
        };
        let playback = playback.with_volume(Volume::Linear(volume));

        let entity = commands
            .spawn((AudioPlayer::<AudioSource>::new(handle), playback))
            .id();

        current_music.entity = Some(entity);
        current_music.track_id = Some(ev.track_id.clone());

        info!(
            "Audio: Playing music '{}' path='{}' looped={} volume={:.2}",
            ev.track_id, asset_path, ev.looped, volume
        );
    }

    // Handle SFX requests
    for ev in sfx_reader.read() {
        if !settings.enabled {
            debug!("Audio disabled; PlaySfx '{}' ignored", ev.sfx_id);
            continue;
        }

        let Some(ref server) = asset_server else {
            debug!("No AssetServer available; PlaySfx '{}' deferred", ev.sfx_id);
            continue;
        };

        let volume = settings.effective_sfx_volume();
        // Domain-layer map takes priority; SDK manifest is the fallback.
        let effective_sfx_id: String = audio_map
            .0
            .as_ref()
            .and_then(|m| m.resolve_sfx(&ev.sfx_id))
            .map(str::to_owned)
            .or_else(|| {
                manifest
                    .0
                    .as_ref()
                    .and_then(|m| m.resolve_sfx(&ev.sfx_id))
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| ev.sfx_id.clone());
        let asset_path = resolve_audio_path(&paths.audio_dir, &effective_sfx_id, None);
        let handle: Handle<AudioSource> = server.load(asset_path.clone());

        let playback = PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume));

        commands.spawn((AudioPlayer::<AudioSource>::new(handle), playback, SfxMarker));

        info!(
            "Audio: Playing SFX '{}' path='{}' volume={:.2}",
            ev.sfx_id, asset_path, volume
        );
    }
}

/// Bevy plugin that initialises the audio subsystem for a loaded campaign.
///
/// Insert via `app.add_plugins(AudioPlugin { ... })` after loading a campaign.
/// The plugin inserts [`AudioSettings`], [`AudioPaths`],
/// [`AudioManifestResource`], and [`AudioMapResource`] as Bevy resources and registers the
/// [`handle_audio_messages`] system on the [`Update`] schedule.
///
/// # Fields
///
/// * `config` — [`AudioConfig`] controlling volume levels (from `config.ron`)
/// * `audio_dir` — campaign-relative directory for audio assets (e.g. `"assets/audio"`)
/// * `audio_manifest` — optional [`AudioManifest`] loaded from `audio.ron`;
///   `None` means filename-by-convention applies for all IDs
/// * `audio_map` — optional domain-layer [`AudioMap`] loaded from `data/audio.ron`;
///   consulted before `audio_manifest` when resolving IDs
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::audio::AudioPlugin;
/// use antares::sdk::game_config::AudioConfig;
/// use bevy::prelude::*;
///
/// # fn example() {
/// let audio_config = AudioConfig::default();
///
/// let mut app = App::new();
/// app.add_plugins(AudioPlugin {
///     config: audio_config,
///     audio_dir: "assets/audio".to_string(),
///     audio_manifest: None,
///     audio_map: None,
/// });
/// # }
/// ```
pub struct AudioPlugin {
    /// Audio configuration to use for initializing AudioSettings
    pub config: AudioConfig,
    /// Campaign-relative directory where audio files are stored.
    ///
    /// Typically `"assets/audio"` (the default).  Passed through to the
    /// [`AudioPaths`] resource so [`handle_audio_messages`] can build
    /// correct relative asset paths from bare track/sfx identifiers.
    pub audio_dir: String,
    /// Campaign audio manifest, if `audio.ron` was present when the campaign loaded.
    ///
    /// `None` is valid — campaigns without `audio.ron` use filename-by-convention.
    pub audio_manifest: Option<AudioManifest>,
    /// Domain-layer audio map loaded from `data/audio.ron`, if present.
    ///
    /// `None` is valid — campaigns without `data/audio.ron` fall back first to
    /// [`AudioManifestResource`] then to filename-by-convention.
    pub audio_map: Option<AudioMap>,
}

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        let settings = AudioSettings::from_config(&self.config);
        let paths = AudioPaths {
            audio_dir: self.audio_dir.clone(),
        };

        app.insert_resource(settings)
            .insert_resource(paths)
            .insert_resource(AudioManifestResource(self.audio_manifest.clone()))
            .insert_resource(AudioMapResource(self.audio_map.clone()))
            .init_resource::<CurrentMusicTrack>()
            .add_message::<PlayMusic>()
            .add_message::<PlaySfx>()
            .add_systems(Update, handle_audio_messages);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_settings_from_config() {
        let config = AudioConfig {
            master_volume: 0.8,
            music_volume: 0.6,
            sfx_volume: 1.0,
            ambient_volume: 0.5,
            enable_audio: true,
        };

        let settings = AudioSettings::from_config(&config);

        assert_eq!(settings.master_volume, 0.8);
        assert_eq!(settings.music_volume, 0.6);
        assert_eq!(settings.sfx_volume, 1.0);
        assert_eq!(settings.ambient_volume, 0.5);
        assert!(settings.enabled);
    }

    #[test]
    fn test_audio_settings_from_default_config() {
        let config = AudioConfig::default();
        let settings = AudioSettings::from_config(&config);

        assert_eq!(settings.master_volume, 0.8);
        assert_eq!(settings.music_volume, 0.6);
        assert_eq!(settings.sfx_volume, 1.0);
        assert_eq!(settings.ambient_volume, 0.5);
        assert!(settings.enabled);
    }

    #[test]
    fn test_audio_disabled_when_config_false() {
        let config = AudioConfig {
            master_volume: 1.0,
            music_volume: 1.0,
            sfx_volume: 1.0,
            ambient_volume: 1.0,
            enable_audio: false,
        };

        let settings = AudioSettings::from_config(&config);

        assert!(!settings.enabled);
        assert_eq!(settings.effective_music_volume(), 0.0);
        assert_eq!(settings.effective_sfx_volume(), 0.0);
        assert_eq!(settings.effective_ambient_volume(), 0.0);
    }

    #[test]
    fn test_effective_volume_calculation() {
        let config = AudioConfig {
            master_volume: 0.5,
            music_volume: 0.8,
            sfx_volume: 1.0,
            ambient_volume: 0.6,
            enable_audio: true,
        };

        let settings = AudioSettings::from_config(&config);

        // Music: 0.5 * 0.8 = 0.4
        assert_eq!(settings.effective_music_volume(), 0.4);

        // SFX: 0.5 * 1.0 = 0.5
        assert_eq!(settings.effective_sfx_volume(), 0.5);

        // Ambient: 0.5 * 0.6 = 0.3
        assert_eq!(settings.effective_ambient_volume(), 0.3);
    }

    #[test]
    fn test_effective_volume_when_disabled() {
        let config = AudioConfig {
            master_volume: 1.0,
            music_volume: 1.0,
            sfx_volume: 1.0,
            ambient_volume: 1.0,
            enable_audio: false,
        };

        let settings = AudioSettings::from_config(&config);

        assert_eq!(settings.effective_volume(0.8), 0.0);
        assert_eq!(settings.effective_music_volume(), 0.0);
        assert_eq!(settings.effective_sfx_volume(), 0.0);
        assert_eq!(settings.effective_ambient_volume(), 0.0);
    }

    #[test]
    fn test_effective_volume_zero_master() {
        let config = AudioConfig {
            master_volume: 0.0,
            music_volume: 1.0,
            sfx_volume: 1.0,
            ambient_volume: 1.0,
            enable_audio: true,
        };

        let settings = AudioSettings::from_config(&config);

        assert_eq!(settings.effective_music_volume(), 0.0);
        assert_eq!(settings.effective_sfx_volume(), 0.0);
        assert_eq!(settings.effective_ambient_volume(), 0.0);
    }

    #[test]
    fn test_effective_volume_max_values() {
        let config = AudioConfig {
            master_volume: 1.0,
            music_volume: 1.0,
            sfx_volume: 1.0,
            ambient_volume: 1.0,
            enable_audio: true,
        };

        let settings = AudioSettings::from_config(&config);

        assert_eq!(settings.effective_music_volume(), 1.0);
        assert_eq!(settings.effective_sfx_volume(), 1.0);
        assert_eq!(settings.effective_ambient_volume(), 1.0);
    }

    #[test]
    fn test_audio_plugin_inserts_resource() {
        let config = AudioConfig {
            master_volume: 0.7,
            music_volume: 0.5,
            sfx_volume: 0.9,
            ambient_volume: 0.4,
            enable_audio: true,
        };

        let mut app = App::new();
        app.add_plugins(AudioPlugin {
            config: config.clone(),
            audio_dir: "assets/audio".to_string(),
            audio_manifest: None,
            audio_map: None,
        });

        // Verify resources were inserted
        let settings = app.world().resource::<AudioSettings>();
        assert_eq!(settings.master_volume, 0.7);
        assert_eq!(settings.music_volume, 0.5);
        assert_eq!(settings.sfx_volume, 0.9);
        assert_eq!(settings.ambient_volume, 0.4);
        assert!(settings.enabled);

        // Verify CurrentMusicTrack resource was initialized
        let music_track = app.world().resource::<CurrentMusicTrack>();
        assert!(music_track.entity.is_none());
        assert!(music_track.track_id.is_none());
    }

    #[test]
    fn test_audio_settings_debug_output() {
        let config = AudioConfig::default();
        let settings = AudioSettings::from_config(&config);

        // Verify Debug trait is implemented and produces output
        let debug_output = format!("{:?}", settings);
        assert!(debug_output.contains("AudioSettings"));
        assert!(debug_output.contains("master_volume"));
    }

    #[test]
    fn test_audio_settings_clone() {
        let config = AudioConfig::default();
        let settings = AudioSettings::from_config(&config);
        let cloned = settings.clone();

        assert_eq!(cloned.master_volume, settings.master_volume);
        assert_eq!(cloned.music_volume, settings.music_volume);
        assert_eq!(cloned.sfx_volume, settings.sfx_volume);
        assert_eq!(cloned.ambient_volume, settings.ambient_volume);
        assert_eq!(cloned.enabled, settings.enabled);
    }

    #[test]
    fn test_current_music_track_default() {
        let track = CurrentMusicTrack::default();
        assert!(track.entity.is_none());
        assert!(track.track_id.is_none());
    }

    /// `resolve_audio_path` must produce a clean path with no double-slash,
    /// even when `audio_dir` has a trailing slash.
    #[test]
    fn test_resolve_audio_path_no_ext() {
        assert_eq!(
            resolve_audio_path("assets/audio", "combat_theme", None),
            "assets/audio/combat_theme.ogg"
        );
    }

    #[test]
    fn test_resolve_audio_path_with_ogg_ext() {
        assert_eq!(
            resolve_audio_path("assets/audio", "combat_theme.ogg", None),
            "assets/audio/combat_theme.ogg"
        );
    }

    #[test]
    fn test_resolve_audio_path_with_mp3_ext() {
        assert_eq!(
            resolve_audio_path("assets/audio", "fanfare.mp3", None),
            "assets/audio/fanfare.mp3"
        );
    }

    #[test]
    fn test_resolve_audio_path_trailing_slash_stripped() {
        // A trailing slash in audio_dir must NOT produce a double-slash "//" in the result.
        assert_eq!(
            resolve_audio_path("assets/audio/", "combat_theme", None),
            "assets/audio/combat_theme.ogg",
            "trailing slash in audio_dir must be stripped before joining"
        );
        assert_eq!(
            resolve_audio_path("assets/audio/", "ambient.ogg", None),
            "assets/audio/ambient.ogg",
            "trailing slash must not produce double-slash with an explicit ext"
        );
        // Multiple trailing slashes should also be normalised.
        assert_eq!(
            resolve_audio_path("assets/audio///", "fanfare.mp3", None),
            "assets/audio/fanfare.mp3",
            "multiple trailing slashes must all be stripped"
        );
    }

    // ── AudioManifest-aware resolve_audio_path tests ─────────────────────────────────────────

    #[test]
    fn test_resolve_audio_path_with_manifest_uses_mapped_value() {
        // When the manifest has a mapping for the id, the mapped filename is used.
        use crate::sdk::game_config::AudioManifest;
        let mut manifest = AudioManifest::default();
        manifest.sfx_mappings.insert(
            "combat_hit".to_string(),
            "sounds/heavy_impact.wav".to_string(),
        );
        assert_eq!(
            resolve_audio_path("assets/audio", "combat_hit", Some(&manifest)),
            "assets/audio/sounds/heavy_impact.wav",
            "mapped value with extension must be used verbatim"
        );
    }

    #[test]
    fn test_resolve_audio_path_without_manifest_falls_back_to_id() {
        // None manifest: id is used as-is with .ogg appended (old behaviour).
        assert_eq!(
            resolve_audio_path("assets/audio", "combat_hit", None),
            "assets/audio/combat_hit.ogg",
            "None manifest must fall back to id-based filename"
        );
    }

    #[test]
    fn test_resolve_audio_path_with_manifest_falls_back_when_id_not_in_map() {
        // Manifest present but the id has no mapping: filename-by-convention applies.
        use crate::sdk::game_config::AudioManifest;
        let manifest = AudioManifest::default(); // empty maps
        assert_eq!(
            resolve_audio_path("assets/audio", "combat_hit", Some(&manifest)),
            "assets/audio/combat_hit.ogg",
            "unmapped id with non-None manifest must still fall back to convention"
        );
    }

    #[test]
    fn test_resolve_audio_path_manifest_mapped_value_with_extension_preserved() {
        // When the mapped value already has an extension, .ogg must NOT be appended.
        use crate::sdk::game_config::AudioManifest;
        let mut manifest = AudioManifest::default();
        manifest.sfx_mappings.insert(
            "victory_fanfare".to_string(),
            "sounds/victory.mp3".to_string(),
        );
        assert_eq!(
            resolve_audio_path("assets/audio", "victory_fanfare", Some(&manifest)),
            "assets/audio/sounds/victory.mp3",
            "mapped value with .mp3 extension must not have .ogg appended"
        );
    }

    #[test]
    fn test_audio_plugin_with_manifest_inserts_resource() {
        // AudioPlugin with a non-None manifest must insert a populated AudioManifestResource.
        use crate::sdk::game_config::{AudioConfig, AudioManifest};
        let mut manifest = AudioManifest::default();
        manifest
            .sfx_mappings
            .insert("combat_hit".to_string(), "sounds/impact.ogg".to_string());

        let mut app = App::new();
        app.add_plugins(AudioPlugin {
            config: AudioConfig::default(),
            audio_dir: "assets/audio".to_string(),
            audio_manifest: Some(manifest.clone()),
            audio_map: None,
        });

        let resource = app.world().resource::<AudioManifestResource>();
        let inner = resource
            .0
            .as_ref()
            .expect("AudioManifestResource must be Some");
        assert_eq!(
            inner.resolve_sfx("combat_hit"),
            Some("sounds/impact.ogg"),
            "manifest resource must contain the manifest passed to AudioPlugin"
        );
    }

    // ── AudioMapResource tests ─────────────────────────────────────────────────────

    #[test]
    fn test_audio_map_resource_resolves_sfx_through_mapping() {
        // AudioMapResource with a mapping must resolve the SFX ID to the mapped filename.
        use std::collections::BTreeMap;
        let mut sfx = BTreeMap::new();
        sfx.insert("combat_hit".to_string(), "sounds/impact.ogg".to_string());
        let map = AudioMap {
            sfx,
            music: BTreeMap::new(),
        };
        let resource = AudioMapResource(Some(map));

        assert_eq!(
            resource.0.as_ref().expect("Some").resolve_sfx("combat_hit"),
            Some("sounds/impact.ogg"),
            "mapped SFX ID must resolve to the configured filename"
        );
        assert_eq!(
            resource
                .0
                .as_ref()
                .expect("Some")
                .resolve_sfx("combat_miss"),
            None,
            "unmapped SFX ID must return None"
        );
    }

    #[test]
    fn test_audio_map_resource_falls_back_to_id_when_no_mapping() {
        // AudioMapResource(None) holds no map; callers must fall back to the bare event ID.
        let resource = AudioMapResource(None);
        assert!(
            resource.0.is_none(),
            "AudioMapResource(None) must have no inner map"
        );
    }

    #[test]
    fn test_audio_plugin_with_audio_map_inserts_resource() {
        // AudioPlugin with audio_map: Some(...) must insert a populated AudioMapResource.
        use std::collections::BTreeMap;
        let mut sfx = BTreeMap::new();
        sfx.insert("combat_hit".to_string(), "sounds/impact.ogg".to_string());
        let map = AudioMap {
            sfx,
            music: BTreeMap::new(),
        };

        let mut app = App::new();
        app.add_plugins(AudioPlugin {
            config: AudioConfig::default(),
            audio_dir: "assets/audio".to_string(),
            audio_manifest: None,
            audio_map: Some(map),
        });

        let resource = app.world().resource::<AudioMapResource>();
        let inner = resource
            .0
            .as_ref()
            .expect("AudioMapResource must be Some when audio_map is provided");
        assert_eq!(
            inner.resolve_sfx("combat_hit"),
            Some("sounds/impact.ogg"),
            "AudioMapResource must contain the map passed to AudioPlugin"
        );
    }
}
