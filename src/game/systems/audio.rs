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
//! });
//! # }
//! ```

use crate::sdk::game_config::AudioConfig;
use bevy::audio::Volume;
use bevy::prelude::*;

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

/// Resolves a bare track/sfx identifier to a campaign-relative asset path.
///
/// If `id` already ends with a recognised audio extension (`.ogg`, `.mp3`,
/// `.wav`, `.flac`) the extension is preserved.  Otherwise `.ogg` is appended
/// as the project-standard audio format.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::audio::resolve_audio_path;
///
/// assert_eq!(resolve_audio_path("assets/audio", "combat_theme"),
///            "assets/audio/combat_theme.ogg");
/// assert_eq!(resolve_audio_path("assets/audio", "combat_theme.ogg"),
///            "assets/audio/combat_theme.ogg");
/// assert_eq!(resolve_audio_path("assets/audio", "fanfare.mp3"),
///            "assets/audio/fanfare.mp3");
/// ```
pub fn resolve_audio_path(audio_dir: &str, id: &str) -> String {
    let has_ext = id.ends_with(".ogg")
        || id.ends_with(".mp3")
        || id.ends_with(".wav")
        || id.ends_with(".flac");
    if has_ext {
        format!("{}/{}", audio_dir, id)
    } else {
        format!("{}/{}.ogg", audio_dir, id)
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
fn handle_audio_messages(
    mut music_reader: MessageReader<PlayMusic>,
    mut sfx_reader: MessageReader<PlaySfx>,
    settings: Res<AudioSettings>,
    paths: Res<AudioPaths>,
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
        let asset_path = resolve_audio_path(&paths.audio_dir, &ev.track_id);
        let handle: Handle<AudioSource> = server.load(asset_path);

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
            ev.track_id,
            resolve_audio_path(&paths.audio_dir, &ev.track_id),
            ev.looped,
            volume
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
        let asset_path = resolve_audio_path(&paths.audio_dir, &ev.sfx_id);
        let handle: Handle<AudioSource> = server.load(asset_path);

        let playback = PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume));

        commands.spawn((AudioPlayer::<AudioSource>::new(handle), playback, SfxMarker));

        info!(
            "Audio: Playing SFX '{}' path='{}' volume={:.2}",
            ev.sfx_id,
            resolve_audio_path(&paths.audio_dir, &ev.sfx_id),
            volume
        );
    }
}

pub struct AudioPlugin {
    /// Audio configuration to use for initializing AudioSettings
    pub config: AudioConfig,
    /// Campaign-relative directory where audio files are stored.
    ///
    /// Typically `"assets/audio"` (the default).  Passed through to the
    /// [`AudioPaths`] resource so [`handle_audio_messages`] can build
    /// correct relative asset paths from bare track/sfx identifiers.
    pub audio_dir: String,
}

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        let settings = AudioSettings::from_config(&self.config);
        let paths = AudioPaths {
            audio_dir: self.audio_dir.clone(),
        };

        app.insert_resource(settings)
            .insert_resource(paths)
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
}
