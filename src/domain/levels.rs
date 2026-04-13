// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Level database — per-class XP threshold tables loaded from `levels.ron`
//!
//! This module provides [`LevelDatabase`], a container for explicit XP
//! thresholds per character class. When a class has an entry in the database,
//! functions like [`crate::domain::progression::experience_for_level_class`]
//! use those thresholds instead of the default formula
//! (`base_xp * (level - 1)^xp_multiplier`).
//!
//! If a class is absent from the database the formula fallback is used
//! transparently — no configuration change is needed at the call site.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 12.5 and
//! `docs/explanation/level_up_plan.md` Phase 1 for full specifications.
//!
//! # Data File Format
//!
//! `levels.ron` uses a struct-wrapper format (not a plain list):
//!
//! ```text
//! (
//!     entries: [
//!         (
//!             class_id: "knight",
//!             thresholds: [0, 1200, 3000, 6000],
//!         ),
//!     ],
//! )
//! ```
//!
//! Classes absent from the file fall back to the XP formula configured in
//! `CampaignConfig`. An empty or missing `levels.ron` is valid and means
//! "use formula for every class".

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur when working with the level database
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum LevelError {
    /// File could not be read from disk
    #[error("Failed to load level database: {0}")]
    LoadError(String),

    /// RON content could not be parsed
    #[error("Failed to parse level data: {0}")]
    ParseError(String),

    /// Requested class ID was not found in the database
    #[error("Class not found in level database: {0}")]
    ClassNotFound(String),
}

// ===== ClassLevelThresholds =====

/// XP thresholds for a single character class
///
/// `thresholds` is a flat vector indexed by `(level - 1)`:
/// - `thresholds[0]` is always `0` (level 1 requires 0 XP)
/// - `thresholds[1]` is the total XP required to reach level 2
/// - …up to 200 entries
///
/// When a character's level exceeds the end of the vector, the last
/// *delta* (difference between the final two entries) is repeated for each
/// extra level — this gives a smooth, unbounded extension beyond the
/// explicitly defined levels.
///
/// # Examples
///
/// ```
/// use antares::domain::levels::ClassLevelThresholds;
///
/// let t = ClassLevelThresholds {
///     class_id: "knight".to_string(),
///     thresholds: vec![0, 1200, 3000, 6000],
/// };
///
/// assert_eq!(t.xp_for_level(1), 0);
/// assert_eq!(t.xp_for_level(2), 1200);
/// assert_eq!(t.xp_for_level(4), 6000);
/// // Beyond table: last delta = 6000 - 3000 = 3000
/// assert_eq!(t.xp_for_level(5), 9000);
/// assert_eq!(t.xp_for_level(6), 12000);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassLevelThresholds {
    /// Class identifier — must match an `id` in `classes.ron`
    pub class_id: String,

    /// XP required to *be at* each level, indexed by `(level - 1)`.
    ///
    /// - `thresholds[0]` → level 1 (always 0)
    /// - `thresholds[N]` → level N+1
    pub thresholds: Vec<u64>,
}

impl ClassLevelThresholds {
    /// Returns the total XP required to reach `level` for this class.
    ///
    /// - Level 1 always returns `0`.
    /// - Levels within the explicit table return the stored value.
    /// - Levels beyond the table extrapolate by repeating the last stored
    ///   delta (the difference between the final two entries). If the table
    ///   has only one entry the delta is `0` (flat cap).
    ///
    /// # Arguments
    ///
    /// * `level` - Target character level (1-based)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::levels::ClassLevelThresholds;
    ///
    /// let t = ClassLevelThresholds {
    ///     class_id: "sorcerer".to_string(),
    ///     thresholds: vec![0, 800, 2000, 4000],
    /// };
    ///
    /// // Within table
    /// assert_eq!(t.xp_for_level(1), 0);
    /// assert_eq!(t.xp_for_level(2), 800);
    /// assert_eq!(t.xp_for_level(4), 4000);
    ///
    /// // Cap behaviour: last delta = 4000 - 2000 = 2000
    /// assert_eq!(t.xp_for_level(5), 6000);
    /// assert_eq!(t.xp_for_level(6), 8000);
    /// ```
    pub fn xp_for_level(&self, level: u32) -> u64 {
        if level <= 1 {
            return 0;
        }

        let idx = (level - 1) as usize; // 0-based index into thresholds
        let len = self.thresholds.len();

        if len == 0 {
            return 0;
        }

        if idx < len {
            self.thresholds[idx]
        } else {
            // Extrapolate: repeat the last delta for each level beyond the table.
            let last = self.thresholds[len - 1];
            let second_to_last = if len >= 2 {
                self.thresholds[len - 2]
            } else {
                0
            };
            let last_delta = last.saturating_sub(second_to_last);
            let extra_levels = (idx - len + 1) as u64;
            last.saturating_add(last_delta.saturating_mul(extra_levels))
        }
    }
}

// ===== LevelDatabase =====

/// Top-level container loaded from `levels.ron`
///
/// Holds explicit per-class XP threshold tables. Classes absent from the
/// database transparently fall back to the XP formula configured in
/// `CampaignConfig` (see [`crate::domain::progression::experience_for_level`]).
///
/// # Construction
///
/// Use [`LevelDatabase::load_from_file`] or [`LevelDatabase::load_from_string`]
/// to construct from RON data. Both methods build the internal lookup index
/// automatically after parsing.
///
/// # Examples
///
/// ```
/// use antares::domain::levels::LevelDatabase;
///
/// let ron = r#"(
///     entries: [
///         (class_id: "knight", thresholds: [0, 1200, 3000, 6000]),
///     ],
/// )"#;
///
/// let db = LevelDatabase::load_from_string(ron).unwrap();
/// assert_eq!(db.threshold_for_class("knight", 2), Some(1200));
/// // Absent class signals formula fallback
/// assert_eq!(db.threshold_for_class("sorcerer", 2), None);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelDatabase {
    /// Serialised list of per-class threshold tables.
    ///
    /// This field is the canonical storage; the `index` below is derived
    /// from it and is rebuilt after every load.
    pub entries: Vec<ClassLevelThresholds>,

    /// Internal `class_id → index-into-entries` lookup map.
    ///
    /// Not written to or read from the RON file. Always rebuilt by
    /// [`rebuild_index`][LevelDatabase::rebuild_index] after deserialisation.
    #[serde(skip)]
    index: HashMap<String, usize>,
}

impl LevelDatabase {
    /// Creates an empty level database (no class entries, formula used for all).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::levels::LevelDatabase;
    ///
    /// let db = LevelDatabase::new();
    /// assert!(db.entries.is_empty());
    /// assert!(db.threshold_for_class("knight", 2).is_none());
    /// ```
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Rebuilds the internal `class_id → index` lookup from `self.entries`.
    ///
    /// Called automatically by [`load_from_string`][LevelDatabase::load_from_string]
    /// and [`load_from_file`][LevelDatabase::load_from_file] after RON
    /// deserialisation. Only the last entry wins when duplicate `class_id`
    /// values are present (the last one shadows earlier ones).
    fn rebuild_index(&mut self) {
        self.index.clear();
        self.index.reserve(self.entries.len());
        for (i, entry) in self.entries.iter().enumerate() {
            self.index.insert(entry.class_id.clone(), i);
        }
    }

    /// Returns the [`ClassLevelThresholds`] for `class_id`, if present.
    ///
    /// Returns `None` if the class is absent from the database. Callers
    /// should treat `None` as "use formula fallback" rather than an error.
    ///
    /// # Arguments
    ///
    /// * `class_id` - The class identifier to look up (e.g. `"knight"`)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::levels::LevelDatabase;
    ///
    /// let ron = r#"(entries: [(class_id: "knight", thresholds: [0, 1200])])"#;
    /// let db = LevelDatabase::load_from_string(ron).unwrap();
    ///
    /// assert!(db.get("knight").is_some());
    /// assert!(db.get("unknown_class").is_none());
    /// ```
    pub fn get(&self, class_id: &str) -> Option<&ClassLevelThresholds> {
        self.index.get(class_id).map(|&i| &self.entries[i])
    }

    /// Returns the XP threshold for `(class_id, level)` when an explicit
    /// table exists; returns `None` to signal that the caller should use the
    /// formula fallback.
    ///
    /// # Arguments
    ///
    /// * `class_id` - The class identifier
    /// * `level`    - The target level (1-based)
    ///
    /// # Returns
    ///
    /// - `Some(xp)` when the class has an explicit entry in the database.
    /// - `None` when the class is absent (caller should use formula).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::levels::LevelDatabase;
    ///
    /// let ron = r#"(
    ///     entries: [
    ///         (class_id: "knight", thresholds: [0, 1200, 3000, 6000]),
    ///     ],
    /// )"#;
    ///
    /// let db = LevelDatabase::load_from_string(ron).unwrap();
    ///
    /// // Level 1 always requires 0 XP
    /// assert_eq!(db.threshold_for_class("knight", 1), Some(0));
    /// // Explicit table value
    /// assert_eq!(db.threshold_for_class("knight", 2), Some(1200));
    /// // Unknown class → None (caller uses formula)
    /// assert_eq!(db.threshold_for_class("sorcerer", 2), None);
    /// ```
    pub fn threshold_for_class(&self, class_id: &str, level: u32) -> Option<u64> {
        self.get(class_id).map(|t| t.xp_for_level(level))
    }

    /// Deserialises a [`LevelDatabase`] from a RON string.
    ///
    /// The RON must use the struct-wrapper format:
    ///
    /// ```text
    /// (
    ///     entries: [
    ///         (class_id: "knight", thresholds: [0, 1000, 2500]),
    ///     ],
    /// )
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`LevelError::ParseError`] if the RON is malformed.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::levels::LevelDatabase;
    ///
    /// let ron = r#"(entries: [(class_id: "knight", thresholds: [0, 1200, 3000])])"#;
    /// let db = LevelDatabase::load_from_string(ron).unwrap();
    ///
    /// assert_eq!(db.entries.len(), 1);
    /// assert_eq!(db.threshold_for_class("knight", 2), Some(1200));
    /// assert_eq!(db.threshold_for_class("knight", 3), Some(3000));
    /// ```
    pub fn load_from_string(data: &str) -> Result<Self, LevelError> {
        let mut db: LevelDatabase =
            ron::from_str(data).map_err(|e| LevelError::ParseError(e.to_string()))?;
        db.rebuild_index();
        Ok(db)
    }

    /// Loads a [`LevelDatabase`] from a RON file on disk.
    ///
    /// # Errors
    ///
    /// Returns [`LevelError::LoadError`] if the file cannot be read, or
    /// [`LevelError::ParseError`] if the RON content is malformed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::domain::levels::LevelDatabase;
    /// use std::path::Path;
    ///
    /// let db = LevelDatabase::load_from_file(
    ///     Path::new("data/test_campaign/data/levels.ron")
    /// ).unwrap();
    ///
    /// assert!(db.entries.len() >= 1);
    /// ```
    pub fn load_from_file(path: &Path) -> Result<Self, LevelError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| LevelError::LoadError(format!("{}: {}", path.display(), e)))?;
        Self::load_from_string(&content)
    }
}

impl Default for LevelDatabase {
    fn default() -> Self {
        Self::new()
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;

    // ---- ClassLevelThresholds tests ----

    #[test]
    fn test_xp_for_level_1_always_zero() {
        let t = ClassLevelThresholds {
            class_id: "knight".to_string(),
            thresholds: vec![0, 1000, 2500, 5000],
        };
        assert_eq!(t.xp_for_level(1), 0);
    }

    #[test]
    fn test_xp_for_level_0_returns_zero() {
        // Level 0 is treated as ≤ 1
        let t = ClassLevelThresholds {
            class_id: "knight".to_string(),
            thresholds: vec![0, 1000, 2500],
        };
        assert_eq!(t.xp_for_level(0), 0);
    }

    #[test]
    fn test_xp_for_level_within_table() {
        let t = ClassLevelThresholds {
            class_id: "knight".to_string(),
            thresholds: vec![0, 1200, 3000, 6000],
        };
        assert_eq!(t.xp_for_level(2), 1200);
        assert_eq!(t.xp_for_level(3), 3000);
        assert_eq!(t.xp_for_level(4), 6000);
    }

    #[test]
    fn test_xp_for_level_cap_behaviour_repeats_last_delta() {
        // Table covers levels 1-4; last delta = 6000 - 3000 = 3000
        let t = ClassLevelThresholds {
            class_id: "knight".to_string(),
            thresholds: vec![0, 1200, 3000, 6000],
        };
        assert_eq!(t.xp_for_level(5), 9000); // 6000 + 3000 * 1
        assert_eq!(t.xp_for_level(6), 12000); // 6000 + 3000 * 2
        assert_eq!(t.xp_for_level(10), 24000); // 6000 + 3000 * 6
    }

    #[test]
    fn test_xp_for_level_empty_table_returns_zero() {
        let t = ClassLevelThresholds {
            class_id: "empty".to_string(),
            thresholds: vec![],
        };
        assert_eq!(t.xp_for_level(1), 0);
        assert_eq!(t.xp_for_level(5), 0);
        assert_eq!(t.xp_for_level(200), 0);
    }

    #[test]
    fn test_xp_for_level_single_entry_flat_cap() {
        // Only one entry [0]. delta = 0 - 0 = 0 → flat cap at 0
        let t = ClassLevelThresholds {
            class_id: "flat".to_string(),
            thresholds: vec![0],
        };
        assert_eq!(t.xp_for_level(1), 0);
        assert_eq!(t.xp_for_level(5), 0);
        assert_eq!(t.xp_for_level(200), 0);
    }

    #[test]
    fn test_xp_for_level_two_entries_cap() {
        // thresholds = [0, 1000]; len = 2; last delta = 1000 - 0 = 1000
        let t = ClassLevelThresholds {
            class_id: "tiny".to_string(),
            thresholds: vec![0, 1000],
        };
        assert_eq!(t.xp_for_level(2), 1000);
        assert_eq!(t.xp_for_level(3), 2000); // 1000 + 1000*1
        assert_eq!(t.xp_for_level(4), 3000); // 1000 + 1000*2
    }

    #[test]
    fn test_xp_for_level_200_boundary() {
        // Make sure we don't overflow or panic at the maximum supported level
        let t = ClassLevelThresholds {
            class_id: "knight".to_string(),
            thresholds: vec![0, 1200, 3000, 6000],
        };
        // Should not panic; value should be finite and increasing
        let xp_199 = t.xp_for_level(199);
        let xp_200 = t.xp_for_level(200);
        assert!(xp_200 >= xp_199);
    }

    // ---- LevelDatabase construction tests ----

    #[test]
    fn test_level_database_new_is_empty() {
        let db = LevelDatabase::new();
        assert!(db.entries.is_empty());
    }

    #[test]
    fn test_level_database_default_is_empty() {
        let db = LevelDatabase::default();
        assert!(db.entries.is_empty());
    }

    // ---- LevelDatabase::get tests ----

    #[test]
    fn test_get_existing_class() {
        let ron = r#"(entries: [(class_id: "knight", thresholds: [0, 1200, 3000])])"#;
        let db = LevelDatabase::load_from_string(ron).unwrap();

        let entry = db.get("knight");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().class_id, "knight");
    }

    #[test]
    fn test_get_nonexistent_class_returns_none() {
        let ron = r#"(entries: [(class_id: "knight", thresholds: [0, 1200])])"#;
        let db = LevelDatabase::load_from_string(ron).unwrap();

        assert!(db.get("sorcerer").is_none());
        assert!(db.get("").is_none());
    }

    #[test]
    fn test_get_on_empty_database_returns_none() {
        let db = LevelDatabase::new();
        assert!(db.get("knight").is_none());
    }

    // ---- LevelDatabase::threshold_for_class tests ----

    #[test]
    fn test_threshold_for_class_found_returns_some() {
        let ron = r#"(entries: [(class_id: "knight", thresholds: [0, 1200, 3000, 6000])])"#;
        let db = LevelDatabase::load_from_string(ron).unwrap();

        assert_eq!(db.threshold_for_class("knight", 1), Some(0));
        assert_eq!(db.threshold_for_class("knight", 2), Some(1200));
        assert_eq!(db.threshold_for_class("knight", 4), Some(6000));
    }

    #[test]
    fn test_threshold_for_class_not_found_returns_none() {
        let ron = r#"(entries: [(class_id: "knight", thresholds: [0, 1200])])"#;
        let db = LevelDatabase::load_from_string(ron).unwrap();

        // Unknown class → caller should use formula
        assert_eq!(db.threshold_for_class("sorcerer", 2), None);
        assert_eq!(db.threshold_for_class("unknown_class", 5), None);
    }

    #[test]
    fn test_threshold_for_class_empty_database_returns_none() {
        let db = LevelDatabase::new();
        assert_eq!(db.threshold_for_class("knight", 2), None);
    }

    #[test]
    fn test_threshold_for_class_cap_behaviour_via_database() {
        // last delta = 6000 - 3000 = 3000; level 5 = 6000 + 3000 = 9000
        let ron = r#"(entries: [(class_id: "knight", thresholds: [0, 1200, 3000, 6000])])"#;
        let db = LevelDatabase::load_from_string(ron).unwrap();

        assert_eq!(db.threshold_for_class("knight", 5), Some(9000));
    }

    // ---- LevelDatabase::load_from_string tests ----

    #[test]
    fn test_load_from_string_valid_single_class() {
        let ron = r#"(
            entries: [
                (class_id: "knight", thresholds: [0, 1200, 3000, 6000]),
            ],
        )"#;
        let db = LevelDatabase::load_from_string(ron).unwrap();
        assert_eq!(db.entries.len(), 1);
        assert_eq!(db.threshold_for_class("knight", 2), Some(1200));
    }

    #[test]
    fn test_load_from_string_multiple_classes() {
        let ron = r#"(
            entries: [
                (class_id: "knight",   thresholds: [0, 1200, 3000]),
                (class_id: "sorcerer", thresholds: [0,  800, 2000]),
            ],
        )"#;
        let db = LevelDatabase::load_from_string(ron).unwrap();

        assert_eq!(db.entries.len(), 2);
        assert_eq!(db.threshold_for_class("knight", 2), Some(1200));
        assert_eq!(db.threshold_for_class("sorcerer", 2), Some(800));
    }

    #[test]
    fn test_load_from_string_empty_entries() {
        let ron = r#"(entries: [])"#;
        let db = LevelDatabase::load_from_string(ron).unwrap();
        assert!(db.entries.is_empty());
        assert_eq!(db.threshold_for_class("knight", 2), None);
    }

    #[test]
    fn test_load_from_string_invalid_ron_returns_parse_error() {
        let result = LevelDatabase::load_from_string("this is not ron }{{{");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LevelError::ParseError(_)));
    }

    #[test]
    fn test_load_from_string_wrong_structure_returns_parse_error() {
        // A plain list instead of the expected struct wrapper
        let result = LevelDatabase::load_from_string("[(class_id: \"knight\", thresholds: [])]");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LevelError::ParseError(_)));
    }

    // ---- LevelDatabase::load_from_file tests ----

    #[test]
    fn test_load_from_file_nonexistent_returns_load_error() {
        let result = LevelDatabase::load_from_file(Path::new("does/not/exist/levels.ron"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LevelError::LoadError(_)));
    }

    #[test]
    fn test_load_from_file_fixture() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path =
            std::path::PathBuf::from(manifest_dir).join("data/test_campaign/data/levels.ron");

        let db = LevelDatabase::load_from_file(&path).unwrap();

        // Fixture has at least one entry
        assert!(
            !db.entries.is_empty(),
            "Expected at least one entry in levels.ron fixture"
        );

        // Knight entry must be present and differ from formula (level 2 = 1200, not 1000)
        assert_eq!(
            db.threshold_for_class("knight", 2),
            Some(1200),
            "Knight level 2 threshold should be 1200 in the fixture"
        );

        // Sorcerer entry must be present
        assert!(
            db.threshold_for_class("sorcerer", 2).is_some(),
            "Sorcerer should have an entry in the fixture"
        );

        // Unknown class → None
        assert_eq!(
            db.threshold_for_class("unknown_class", 2),
            None,
            "Unknown class should return None"
        );
    }

    // ---- Round-trip serialisation tests ----

    #[test]
    fn test_roundtrip_single_class() {
        let original = LevelDatabase {
            entries: vec![ClassLevelThresholds {
                class_id: "knight".to_string(),
                thresholds: vec![0, 1200, 3000, 6000, 10000, 15000],
            }],
            index: HashMap::new(),
        };

        // Serialize to RON
        let serialized = ron::to_string(&original).unwrap();

        // Deserialize back
        let restored = LevelDatabase::load_from_string(&serialized).unwrap();

        // Verify round-trip fidelity
        assert_eq!(restored.entries.len(), 1);
        assert_eq!(restored.entries[0].class_id, "knight");
        assert_eq!(
            restored.entries[0].thresholds,
            vec![0, 1200, 3000, 6000, 10000, 15000]
        );
        assert_eq!(restored.threshold_for_class("knight", 2), Some(1200));
        assert_eq!(restored.threshold_for_class("knight", 6), Some(15000));
    }

    #[test]
    fn test_roundtrip_multiple_classes() {
        let ron = r#"(
            entries: [
                (class_id: "knight",   thresholds: [0, 1200, 3000, 6000]),
                (class_id: "sorcerer", thresholds: [0,  800, 2000, 4000]),
                (class_id: "cleric",   thresholds: [0, 1000, 2500, 5000]),
            ],
        )"#;

        let db = LevelDatabase::load_from_string(ron).unwrap();
        let serialized = ron::to_string(&db).unwrap();
        let restored = LevelDatabase::load_from_string(&serialized).unwrap();

        assert_eq!(restored.entries.len(), 3);
        assert_eq!(restored.threshold_for_class("knight", 2), Some(1200));
        assert_eq!(restored.threshold_for_class("sorcerer", 2), Some(800));
        assert_eq!(restored.threshold_for_class("cleric", 2), Some(1000));
    }

    #[test]
    fn test_roundtrip_empty_database() {
        let db = LevelDatabase::new();
        let serialized = ron::to_string(&db).unwrap();
        let restored = LevelDatabase::load_from_string(&serialized).unwrap();
        assert!(restored.entries.is_empty());
    }

    // ---- Index rebuild verification ----

    #[test]
    fn test_index_rebuilt_after_load_from_string() {
        let ron = r#"(
            entries: [
                (class_id: "paladin", thresholds: [0, 1500, 3500]),
                (class_id: "robber",  thresholds: [0,  900, 2200]),
            ],
        )"#;
        let db = LevelDatabase::load_from_string(ron).unwrap();

        // Both classes should be findable via the index
        assert!(db.get("paladin").is_some());
        assert!(db.get("robber").is_some());
        assert!(db.get("archer").is_none());
    }

    // ---- LevelError display tests ----

    #[test]
    fn test_level_error_display_load_error() {
        let err = LevelError::LoadError("file not found".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Failed to load level database"));
        assert!(display.contains("file not found"));
    }

    #[test]
    fn test_level_error_display_parse_error() {
        let err = LevelError::ParseError("unexpected token".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Failed to parse level data"));
        assert!(display.contains("unexpected token"));
    }

    #[test]
    fn test_level_error_display_class_not_found() {
        let err = LevelError::ClassNotFound("wizard".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Class not found in level database"));
        assert!(display.contains("wizard"));
    }

    // ---- Boundary / edge case tests ----

    #[test]
    fn test_threshold_for_class_level_200() {
        let ron = r#"(entries: [(class_id: "knight", thresholds: [0, 1200, 3000, 6000])])"#;
        let db = LevelDatabase::load_from_string(ron).unwrap();

        // Should not panic and should be strictly increasing
        let xp_199 = db.threshold_for_class("knight", 199).unwrap();
        let xp_200 = db.threshold_for_class("knight", 200).unwrap();
        assert!(
            xp_200 > xp_199,
            "XP at level 200 should be > XP at level 199"
        );
    }

    #[test]
    fn test_threshold_strictly_increasing_after_cap() {
        // Verify cap extrapolation produces monotonically increasing values
        let t = ClassLevelThresholds {
            class_id: "archer".to_string(),
            thresholds: vec![0, 1000, 2500, 5000, 8500],
        };
        let mut prev = t.xp_for_level(1);
        for lvl in 2..=20 {
            let cur = t.xp_for_level(lvl);
            assert!(
                cur >= prev,
                "xp_for_level({}) = {} < xp_for_level({}) = {}",
                lvl,
                cur,
                lvl - 1,
                prev
            );
            prev = cur;
        }
    }

    #[test]
    fn test_level_database_two_classes_independent() {
        // Verify that two classes stored independently return correct values
        let ron = r#"(
            entries: [
                (class_id: "knight",   thresholds: [0, 1200, 3000, 6000]),
                (class_id: "sorcerer", thresholds: [0,  800, 2000, 4000]),
            ],
        )"#;
        let db = LevelDatabase::load_from_string(ron).unwrap();

        // Knight level 3 = 3000, Sorcerer level 3 = 2000 — must not cross-contaminate
        assert_ne!(
            db.threshold_for_class("knight", 3),
            db.threshold_for_class("sorcerer", 3)
        );
        assert_eq!(db.threshold_for_class("knight", 3), Some(3000));
        assert_eq!(db.threshold_for_class("sorcerer", 3), Some(2000));
    }
}
