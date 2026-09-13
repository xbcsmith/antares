// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Domain-layer type for campaign audio configuration.
//!
//! [`AudioMap`] maps well-known engine event IDs to campaign-specific audio
//! filenames. It is the in-memory representation of `data/audio.ron` inside a
//! campaign directory.
//!
//! Campaigns that do not ship a `data/audio.ron` file continue to work: the
//! engine falls back to its filename-by-convention behaviour, and an
//! [`AudioMap`] constructed via [`Default`] simply returns `None` for every
//! lookup.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Maps engine event IDs to campaign-specific audio filenames.
///
/// `sfx` holds sound-effect bindings (engine event ID → filename) and `music`
/// holds music-track bindings (track ID → filename).  Both maps are optional in
/// the RON file; missing keys are treated as empty maps.
///
/// # Examples
///
/// RON round-trip:
///
/// ```
/// use antares::domain::AudioMap;
///
/// let ron_src = r#"(
///     sfx: {
///         "combat_hit": "sounds/impact.ogg",
///     },
///     music: {
///         "combat_theme": "music/battle.ogg",
///     },
/// )"#;
///
/// let map: AudioMap = ron::from_str(ron_src).expect("valid RON");
///
/// // Known SFX ID resolves to its filename.
/// assert_eq!(map.resolve_sfx("combat_hit"), Some("sounds/impact.ogg"));
///
/// // Unknown ID returns None.
/// assert_eq!(map.resolve_sfx("unknown_event"), None);
///
/// // Serialise and round-trip.
/// let serialised = ron::to_string(&map).expect("serialisable");
/// let map2: AudioMap = ron::from_str(&serialised).expect("round-trip");
/// assert_eq!(map, map2);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AudioMap {
    /// Engine SFX event ID → audio filename.
    #[serde(default)]
    pub sfx: BTreeMap<String, String>,

    /// Music track ID → audio filename.
    #[serde(default)]
    pub music: BTreeMap<String, String>,
}

impl AudioMap {
    /// Returns the audio filename bound to the given engine SFX event ID, or
    /// `None` if the campaign has not defined a mapping for it.
    ///
    /// # Arguments
    ///
    /// * `engine_id` – The well-known engine event identifier (e.g. `"combat_hit"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::AudioMap;
    /// use std::collections::BTreeMap;
    ///
    /// let mut map = AudioMap::default();
    /// map.sfx.insert("combat_hit".into(), "sounds/impact.ogg".into());
    ///
    /// assert_eq!(map.resolve_sfx("combat_hit"), Some("sounds/impact.ogg"));
    /// assert_eq!(map.resolve_sfx("combat_miss"), None);
    /// ```
    pub fn resolve_sfx(&self, engine_id: &str) -> Option<&str> {
        self.sfx.get(engine_id).map(String::as_str)
    }

    /// Returns the audio filename bound to the given music track ID, or `None`
    /// if the campaign has not defined a mapping for it.
    ///
    /// # Arguments
    ///
    /// * `track_id` – The well-known music track identifier (e.g. `"combat_theme"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::AudioMap;
    /// use std::collections::BTreeMap;
    ///
    /// let mut map = AudioMap::default();
    /// map.music.insert("combat_theme".into(), "music/battle.ogg".into());
    ///
    /// assert_eq!(map.resolve_music("combat_theme"), Some("music/battle.ogg"));
    /// assert_eq!(map.resolve_music("exploration_theme"), None);
    /// ```
    pub fn resolve_music(&self, track_id: &str) -> Option<&str> {
        self.music.get(track_id).map(String::as_str)
    }

    /// Returns all seven well-known engine event IDs recognised by the audio
    /// system: five SFX events and two music tracks.
    ///
    /// Campaigns may define filenames for any subset of these IDs in their
    /// `data/audio.ron`.  IDs absent from the map fall back to
    /// filename-by-convention.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::AudioMap;
    ///
    /// let ids = AudioMap::default_sfx_ids();
    /// assert_eq!(ids.len(), 7);
    /// assert!(ids.contains(&"combat_hit"));
    /// assert!(ids.contains(&"combat_theme"));
    /// ```
    pub fn default_sfx_ids() -> &'static [&'static str] {
        &[
            "combat_hit",
            "combat_miss",
            "combat_heal",
            "spell_fizzle",
            "victory_fanfare",
            "combat_theme",
            "exploration_theme",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_sfx_returns_mapped_filename_when_present() {
        let mut map = AudioMap::default();
        map.sfx
            .insert("combat_hit".into(), "sounds/impact.ogg".into());

        assert_eq!(map.resolve_sfx("combat_hit"), Some("sounds/impact.ogg"));
    }

    #[test]
    fn test_resolve_sfx_returns_none_when_absent() {
        let map = AudioMap::default();

        assert_eq!(map.resolve_sfx("combat_hit"), None);
    }

    #[test]
    fn test_resolve_music_returns_mapped_filename_when_present() {
        let mut map = AudioMap::default();
        map.music
            .insert("combat_theme".into(), "music/battle.ogg".into());

        assert_eq!(map.resolve_music("combat_theme"), Some("music/battle.ogg"));
    }

    #[test]
    fn test_default_sfx_ids_contains_all_seven_engine_events() {
        let ids = AudioMap::default_sfx_ids();

        assert_eq!(ids.len(), 7);

        let expected = [
            "combat_hit",
            "combat_miss",
            "combat_heal",
            "spell_fizzle",
            "victory_fanfare",
            "combat_theme",
            "exploration_theme",
        ];

        for id in &expected {
            assert!(
                ids.contains(id),
                "expected engine event ID '{id}' not found in default_sfx_ids()"
            );
        }
    }
}
