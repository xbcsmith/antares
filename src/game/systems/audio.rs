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
use bevy::prelude::*;

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

/// Lightweight handler that consumes audio messages and forwards them to the
/// audio subsystem (placeholder - currently logs the intent and respects
/// `AudioSettings`).
fn handle_audio_messages(
    mut music_reader: MessageReader<PlayMusic>,
    mut sfx_reader: MessageReader<PlaySfx>,
    settings: Res<AudioSettings>,
) {
    // Handle music requests
    for ev in music_reader.read() {
        if settings.enabled {
            info!(
                "Audio: PlayMusic track='{}' looped={} volume={}",
                ev.track_id,
                ev.looped,
                settings.effective_music_volume()
            );
        } else {
            debug!("Audio disabled; PlayMusic '{}' ignored", ev.track_id);
        }
    }

    // Handle SFX requests
    for ev in sfx_reader.read() {
        if settings.enabled {
            info!(
                "Audio: PlaySfx sfx='{}' volume={}",
                ev.sfx_id,
                settings.effective_sfx_volume()
            );
        } else {
            debug!("Audio disabled; PlaySfx '{}' ignored", ev.sfx_id);
        }
    }
}

pub struct AudioPlugin {
    /// Audio configuration to use for initializing AudioSettings
    pub config: AudioConfig,
}

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        let settings = AudioSettings::from_config(&self.config);

        // Insert the runtime audio settings resource, register audio messages,
        // and add a simple handler that will eventually be replaced by a real
        // playback system (Bevy Audio integration).
        app.insert_resource(settings)
            .add_message::<PlayMusic>()
            .add_message::<PlaySfx>()
            .add_systems(Update, handle_audio_messages);

        // Future: Hook up to concrete audio playback (Bevy's audio) here.
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
        });

        // Verify resource was inserted
        let settings = app.world().resource::<AudioSettings>();
        assert_eq!(settings.master_volume, 0.7);
        assert_eq!(settings.music_volume, 0.5);
        assert_eq!(settings.sfx_volume, 0.9);
        assert_eq!(settings.ambient_volume, 0.4);
        assert!(settings.enabled);
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
}
