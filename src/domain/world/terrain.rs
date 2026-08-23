// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Terrain definition and database support.
//!
//! Terrain data represents the fundamental ground type of each map tile.
//! Unlike the closed [`TerrainType`](crate::domain::world::TerrainType) enum,
//! terrain definitions are loaded from `data/terrain.ron` into a
//! [`TerrainDatabase`] registry, allowing campaign authors to introduce
//! arbitrary terrain types — including Sand, Snow, and Ice — without engine
//! code changes.
//!
//! Built-in terrain definitions (Ground, Grass, Water, Lava, Swamp, Stone,
//! Dirt, Forest, Mountain, Sand, Snow, Ice) occupy IDs `13000`–`13011`.
//! Campaign-defined custom terrain starts at `13100` by convention.
//!
//! The design mirrors [`LandscapeDatabase`](crate::domain::world::LandscapeDatabase):
//! small closed *style* enums (`TerrainMeshStyle`, `TerrainVegetation`) capture
//! the finite set of rendering shapes, while the open `TerrainDefinition` struct
//! carries per-entry authored content.
//!
//! # Examples
//!
//! ```
//! use antares::domain::types::{TerrainId, TERRAIN_ID_MIN};
//! use antares::domain::world::terrain::{
//!     TerrainDatabase, TerrainDefinition, TerrainMeshStyle, TerrainVegetation,
//! };
//!
//! let mut db = TerrainDatabase::new();
//! db.add(TerrainDefinition {
//!     id: TERRAIN_ID_MIN,
//!     name: "Ground".to_string(),
//!     texture_path: "assets/textures/terrain/ground.png".to_string(),
//!     roughness: 0.8,
//!     mesh_style: TerrainMeshStyle::Flat,
//!     vegetation: TerrainVegetation::None,
//!     blocked: false,
//!     height: 0.0,
//!     color: [0.6, 0.5, 0.4],
//! })
//! .unwrap();
//!
//! assert!(db.has_definition(TERRAIN_ID_MIN));
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::types::{TerrainId, TERRAIN_ID_MIN};

// ===== Errors =====

/// Errors that can occur when working with terrain definitions and databases.
///
/// # Examples
///
/// ```
/// use antares::domain::world::terrain::TerrainDatabaseError;
///
/// let error = TerrainDatabaseError::NotFound(13000);
/// assert!(error.to_string().contains("13000"));
/// ```
#[derive(Error, Debug)]
pub enum TerrainDatabaseError {
    /// File could not be read from disk.
    #[error("Failed to read terrain data file: {0}")]
    ReadError(#[from] std::io::Error),

    /// RON parsing failed.
    #[error("Failed to parse terrain RON data: {0}")]
    ParseError(#[from] ron::error::SpannedError),

    /// An entry with the same ID already exists.
    #[error("Duplicate terrain ID {0} detected")]
    DuplicateId(TerrainId),

    /// A definition with the given ID was not found.
    #[error("Terrain ID {0} not found in database")]
    NotFound(TerrainId),

    /// A terrain definition used an ID below the minimum allowed value.
    #[error("Terrain ID {id} is invalid; IDs must be >= {min}")]
    InvalidTerrainId {
        /// Invalid terrain definition ID.
        id: TerrainId,
        /// Minimum valid terrain definition ID.
        min: TerrainId,
    },
}

// ===== TerrainMeshStyle =====

/// Controls which mesh-spawn branch is selected for a terrain tile.
///
/// `Flat` is the default and covers the majority of terrain types. `Water` and
/// `Mountain` select the existing specialized mesh paths already present in the
/// map renderer.
///
/// # Examples
///
/// ```
/// use antares::domain::world::terrain::TerrainMeshStyle;
///
/// let style = TerrainMeshStyle::default();
/// assert_eq!(style, TerrainMeshStyle::Flat);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TerrainMeshStyle {
    /// Standard flat ground plane mesh (default for most terrain types).
    #[default]
    Flat,
    /// Animated water surface mesh.
    Water,
    /// Elevated, non-traversable mountain mesh.
    Mountain,
}

impl TerrainMeshStyle {
    /// Returns all mesh styles in definition order.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::terrain::TerrainMeshStyle;
    ///
    /// assert!(TerrainMeshStyle::all().contains(&TerrainMeshStyle::Water));
    /// ```
    pub const fn all() -> &'static [TerrainMeshStyle] {
        &[
            TerrainMeshStyle::Flat,
            TerrainMeshStyle::Water,
            TerrainMeshStyle::Mountain,
        ]
    }
}

// ===== TerrainVegetation =====

/// Controls which vegetation decoration is spawned on top of a terrain tile.
///
/// `GrassCover` triggers grass-blade decoration. `Forest` triggers both
/// grass-blade decoration and tree/shrub placement via `vegetation_placement.rs`.
///
/// # Examples
///
/// ```
/// use antares::domain::world::terrain::TerrainVegetation;
///
/// let veg = TerrainVegetation::default();
/// assert_eq!(veg, TerrainVegetation::None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TerrainVegetation {
    /// No vegetation decoration (default).
    #[default]
    None,
    /// Grass-blade ground cover spawned on this terrain type.
    GrassCover,
    /// Full forest decoration: grass blades plus tree and shrub placement.
    Forest,
}

impl TerrainVegetation {
    /// Returns all vegetation variants in definition order.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::terrain::TerrainVegetation;
    ///
    /// assert!(TerrainVegetation::all().contains(&TerrainVegetation::Forest));
    /// ```
    pub const fn all() -> &'static [TerrainVegetation] {
        &[
            TerrainVegetation::None,
            TerrainVegetation::GrassCover,
            TerrainVegetation::Forest,
        ]
    }
}

// ===== TerrainDefinition =====

/// A reusable terrain template loaded from `data/terrain.ron`.
///
/// Each definition describes one terrain type: its visual properties
/// (`texture_path`, `roughness`, `color`), its rendering shape
/// (`mesh_style`), the vegetation it spawns (`vegetation`), and whether
/// tiles of this type block party movement by default (`blocked`).
///
/// The `mesh_style`, `vegetation`, `blocked`, and `height` fields are all
/// optional in RON (they default to `Flat`, `None`, `false`, and `0.0`
/// respectively), so a minimal entry needs only `id`, `name`,
/// `texture_path`, `roughness`, and `color`.
///
/// # Examples
///
/// ```
/// use antares::domain::types::TERRAIN_ID_MIN;
/// use antares::domain::world::terrain::{
///     TerrainDefinition, TerrainMeshStyle, TerrainVegetation,
/// };
///
/// let ground = TerrainDefinition {
///     id: TERRAIN_ID_MIN,
///     name: "Ground".to_string(),
///     texture_path: "assets/textures/terrain/ground.png".to_string(),
///     roughness: 0.8,
///     mesh_style: TerrainMeshStyle::Flat,
///     vegetation: TerrainVegetation::None,
///     blocked: false,
///     height: 0.0,
///     color: [0.6, 0.5, 0.4],
/// };
/// assert_eq!(ground.name, "Ground");
/// assert!(!ground.blocked);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainDefinition {
    /// Unique terrain definition identifier; must be >= [`TERRAIN_ID_MIN`].
    pub id: TerrainId,
    /// Human-readable display name (e.g. `"Grass"`, `"Lava"`).
    pub name: String,
    /// Relative path to the diffuse texture, prefixed with `assets/`.
    pub texture_path: String,
    /// PBR roughness value in `[0.0, 1.0]`.
    pub roughness: f32,
    /// Mesh-spawn branch used by the map renderer for this terrain type.
    #[serde(default)]
    pub mesh_style: TerrainMeshStyle,
    /// Vegetation decoration spawned on tiles of this terrain type.
    #[serde(default)]
    pub vegetation: TerrainVegetation,
    /// Whether tiles of this terrain type block party movement by default.
    ///
    /// Note: individual `Tile.blocked` fields are independently mutable at
    /// runtime (doors, spells, etc.) and take precedence over this value.
    #[serde(default)]
    pub blocked: bool,
    /// Effective height offset in world units applied to the tile mesh.
    #[serde(default)]
    pub height: f32,
    /// Automap RGB color used when rendering the tile on the mini-map.
    pub color: [f32; 3],
}

// ===== TerrainDatabase =====

/// In-memory index of all [`TerrainDefinition`] entries for a campaign.
///
/// Definitions are keyed by [`TerrainId`] for O(1) lookup. The database is
/// populated either via [`TerrainDatabase::load_from_string`] /
/// [`TerrainDatabase::load_from_file`] (generated by
/// [`crate::impl_ron_database!`]) or by calling [`TerrainDatabase::add`]
/// directly.
///
/// # Examples
///
/// ```
/// use antares::domain::types::TERRAIN_ID_MIN;
/// use antares::domain::world::terrain::{
///     TerrainDatabase, TerrainDefinition, TerrainMeshStyle, TerrainVegetation,
/// };
///
/// let mut db = TerrainDatabase::new();
/// db.add(TerrainDefinition {
///     id: TERRAIN_ID_MIN,
///     name: "Ground".to_string(),
///     texture_path: "assets/textures/terrain/ground.png".to_string(),
///     roughness: 0.8,
///     mesh_style: TerrainMeshStyle::Flat,
///     vegetation: TerrainVegetation::None,
///     blocked: false,
///     height: 0.0,
///     color: [0.6, 0.5, 0.4],
/// })
/// .unwrap();
///
/// assert_eq!(db.len(), 1);
/// assert!(!db.is_empty());
/// assert!(db.get_by_name("Ground").is_some());
/// ```
#[derive(Debug, Clone, Default)]
pub struct TerrainDatabase {
    /// Definitions indexed by [`TerrainId`] for O(1) lookup.
    items: HashMap<TerrainId, TerrainDefinition>,
}

crate::impl_ron_database!(
    TerrainDatabase,
    entity: TerrainDefinition,
    key: TerrainId,
    error: TerrainDatabaseError,
    field: items,
    id_of: |d: &TerrainDefinition| d.id,
    dup_err: TerrainDatabaseError::DuplicateId,
    read_err: TerrainDatabaseError::ReadError,
    parse_err: TerrainDatabaseError::ParseError,
    post_load: |db: &TerrainDatabase| db.validate_definition_ids(),
);

impl TerrainDatabase {
    fn validate_definition_ids(&self) -> Result<(), TerrainDatabaseError> {
        for def in self.items.values() {
            if def.id < TERRAIN_ID_MIN {
                return Err(TerrainDatabaseError::InvalidTerrainId {
                    id: def.id,
                    min: TERRAIN_ID_MIN,
                });
            }
        }
        Ok(())
    }

    /// Creates an empty terrain database.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::terrain::TerrainDatabase;
    ///
    /// let db = TerrainDatabase::new();
    /// assert!(db.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    /// Adds a definition to the database.
    ///
    /// # Errors
    ///
    /// Returns [`TerrainDatabaseError::InvalidTerrainId`] if `def.id` is below
    /// [`TERRAIN_ID_MIN`].
    ///
    /// Returns [`TerrainDatabaseError::DuplicateId`] if a definition with the
    /// same `id` already exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::TERRAIN_ID_MIN;
    /// use antares::domain::world::terrain::{
    ///     TerrainDatabase, TerrainDefinition, TerrainMeshStyle, TerrainVegetation,
    /// };
    ///
    /// let mut db = TerrainDatabase::new();
    /// let def = TerrainDefinition {
    ///     id: TERRAIN_ID_MIN,
    ///     name: "Ground".to_string(),
    ///     texture_path: "assets/textures/terrain/ground.png".to_string(),
    ///     roughness: 0.8,
    ///     mesh_style: TerrainMeshStyle::Flat,
    ///     vegetation: TerrainVegetation::None,
    ///     blocked: false,
    ///     height: 0.0,
    ///     color: [0.6, 0.5, 0.4],
    /// };
    /// assert!(db.add(def).is_ok());
    /// ```
    pub fn add(&mut self, def: TerrainDefinition) -> Result<(), TerrainDatabaseError> {
        if def.id < TERRAIN_ID_MIN {
            return Err(TerrainDatabaseError::InvalidTerrainId {
                id: def.id,
                min: TERRAIN_ID_MIN,
            });
        }
        if self.items.contains_key(&def.id) {
            return Err(TerrainDatabaseError::DuplicateId(def.id));
        }
        self.items.insert(def.id, def);
        Ok(())
    }

    /// Returns a reference to the definition with the given ID, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::terrain::TerrainDatabase;
    ///
    /// let db = TerrainDatabase::new();
    /// assert!(db.get_by_id(13000).is_none());
    /// ```
    pub fn get_by_id(&self, id: TerrainId) -> Option<&TerrainDefinition> {
        self.items.get(&id)
    }

    /// Returns the first definition whose `name` field matches case-sensitively.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::TERRAIN_ID_MIN;
    /// use antares::domain::world::terrain::{
    ///     TerrainDatabase, TerrainDefinition, TerrainMeshStyle, TerrainVegetation,
    /// };
    ///
    /// let mut db = TerrainDatabase::new();
    /// db.add(TerrainDefinition {
    ///     id: TERRAIN_ID_MIN,
    ///     name: "Ground".to_string(),
    ///     texture_path: "assets/textures/terrain/ground.png".to_string(),
    ///     roughness: 0.8,
    ///     mesh_style: TerrainMeshStyle::Flat,
    ///     vegetation: TerrainVegetation::None,
    ///     blocked: false,
    ///     height: 0.0,
    ///     color: [0.6, 0.5, 0.4],
    /// }).unwrap();
    /// assert_eq!(db.get_by_name("Ground").unwrap().id, TERRAIN_ID_MIN);
    /// ```
    pub fn get_by_name(&self, name: &str) -> Option<&TerrainDefinition> {
        self.items.values().find(|d| d.name == name)
    }

    /// Returns all definitions in the database.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::terrain::TerrainDatabase;
    ///
    /// let db = TerrainDatabase::new();
    /// assert!(db.all_definitions().is_empty());
    /// ```
    pub fn all_definitions(&self) -> Vec<&TerrainDefinition> {
        self.items.values().collect()
    }

    /// Returns the number of definitions in the database.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::terrain::TerrainDatabase;
    ///
    /// let db = TerrainDatabase::new();
    /// assert_eq!(db.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the database contains no definitions.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::terrain::TerrainDatabase;
    ///
    /// assert!(TerrainDatabase::new().is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns `true` if a definition with the given ID exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::terrain::TerrainDatabase;
    ///
    /// let db = TerrainDatabase::new();
    /// assert!(!db.has_definition(13000));
    /// ```
    pub fn has_definition(&self, id: TerrainId) -> bool {
        self.items.contains_key(&id)
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;

    fn make_definition(id: TerrainId, name: &str) -> TerrainDefinition {
        TerrainDefinition {
            id,
            name: name.to_string(),
            texture_path: "assets/textures/terrain/ground.png".to_string(),
            roughness: 0.8,
            mesh_style: TerrainMeshStyle::Flat,
            vegetation: TerrainVegetation::None,
            blocked: false,
            height: 0.0,
            color: [0.6, 0.5, 0.4],
        }
    }

    #[test]
    fn test_terrain_database_add_and_lookup() {
        let mut db = TerrainDatabase::new();
        db.add(make_definition(TERRAIN_ID_MIN, "Ground")).unwrap();

        assert_eq!(db.len(), 1);
        assert!(!db.is_empty());
        assert!(db.has_definition(TERRAIN_ID_MIN));
        assert!(db.get_by_id(TERRAIN_ID_MIN).is_some());
        assert!(db.get_by_name("Ground").is_some());
        assert_eq!(db.all_definitions().len(), 1);
    }

    #[test]
    fn test_terrain_database_rejects_duplicate_id() {
        let mut db = TerrainDatabase::new();
        db.add(make_definition(TERRAIN_ID_MIN, "Ground")).unwrap();

        assert!(matches!(
            db.add(make_definition(TERRAIN_ID_MIN, "Dirt")),
            Err(TerrainDatabaseError::DuplicateId(id)) if id == TERRAIN_ID_MIN
        ));
    }

    #[test]
    fn test_terrain_database_rejects_id_below_min() {
        let mut db = TerrainDatabase::new();

        assert!(matches!(
            db.add(make_definition(0, "Invalid")),
            Err(TerrainDatabaseError::InvalidTerrainId {
                id: 0,
                min: TERRAIN_ID_MIN
            })
        ));

        // Also test an ID just below the minimum
        assert!(matches!(
            db.add(make_definition(TERRAIN_ID_MIN - 1, "AlsoInvalid")),
            Err(TerrainDatabaseError::InvalidTerrainId {
                id,
                min: TERRAIN_ID_MIN
            }) if id == TERRAIN_ID_MIN - 1
        ));
    }

    #[test]
    fn test_terrain_definition_ron_roundtrip_preserves_all_fields() {
        let definition = TerrainDefinition {
            id: 13007,
            name: "Forest".to_string(),
            texture_path: "assets/textures/terrain/forest.png".to_string(),
            roughness: 0.9,
            mesh_style: TerrainMeshStyle::Flat,
            vegetation: TerrainVegetation::Forest,
            blocked: false,
            height: 0.1,
            color: [0.1, 0.4, 0.1],
        };

        let ron = ron::to_string(&definition).unwrap();
        let roundtrip: TerrainDefinition = ron::from_str(&ron).unwrap();

        assert_eq!(roundtrip, definition);
    }

    #[test]
    fn test_terrain_mesh_style_and_vegetation_ron_roundtrip_all_variants() {
        for style in TerrainMeshStyle::all() {
            let ron = ron::to_string(style).unwrap();
            let roundtrip: TerrainMeshStyle = ron::from_str(&ron).unwrap();
            assert_eq!(
                roundtrip, *style,
                "TerrainMeshStyle::{style:?} must round-trip through RON"
            );
        }

        for veg in TerrainVegetation::all() {
            let ron = ron::to_string(veg).unwrap();
            let roundtrip: TerrainVegetation = ron::from_str(&ron).unwrap();
            assert_eq!(
                roundtrip, *veg,
                "TerrainVegetation::{veg:?} must round-trip through RON"
            );
        }
    }

    #[test]
    fn test_terrain_definition_minimal_ron_defaults_optional_fields() {
        let ron = r#"[
            (
                id: 13000,
                name: "Ground",
                texture_path: "assets/textures/terrain/ground.png",
                roughness: 0.8,
                color: (0.6, 0.5, 0.4),
            ),
        ]"#;

        let db = TerrainDatabase::load_from_string(ron).unwrap();
        let def = db.get_by_id(13000).unwrap();

        assert_eq!(def.mesh_style, TerrainMeshStyle::Flat);
        assert_eq!(def.vegetation, TerrainVegetation::None);
        assert!(!def.blocked);
        assert_eq!(def.height, 0.0);
    }

    #[test]
    fn test_terrain_database_load_from_string_rejects_id_below_min() {
        let ron = r#"[
            (
                id: 100,
                name: "Invalid",
                texture_path: "assets/textures/terrain/invalid.png",
                roughness: 0.5,
                color: (0.5, 0.5, 0.5),
            ),
        ]"#;

        assert!(matches!(
            TerrainDatabase::load_from_string(ron),
            Err(TerrainDatabaseError::InvalidTerrainId {
                id: 100,
                min: TERRAIN_ID_MIN
            })
        ));
    }

    #[test]
    fn test_terrain_database_load_from_string_full_entry() {
        let ron = r#"[
            (
                id: 13002,
                name: "Water",
                texture_path: "assets/textures/terrain/water.png",
                roughness: 0.1,
                mesh_style: Water,
                vegetation: None,
                blocked: true,
                height: 0.0,
                color: (0.1, 0.3, 0.8),
            ),
        ]"#;

        let db = TerrainDatabase::load_from_string(ron).unwrap();
        let def = db.get_by_id(13002).unwrap();

        assert_eq!(def.name, "Water");
        assert_eq!(def.mesh_style, TerrainMeshStyle::Water);
        assert!(def.blocked);
    }

    #[test]
    fn test_terrain_database_get_by_id_missing_returns_none() {
        let db = TerrainDatabase::new();
        assert!(db.get_by_id(13000).is_none());
    }

    #[test]
    fn test_terrain_database_get_by_name_missing_returns_none() {
        let db = TerrainDatabase::new();
        assert!(db.get_by_name("Nonexistent").is_none());
    }

    #[test]
    fn test_terrain_mesh_style_default_is_flat() {
        assert_eq!(TerrainMeshStyle::default(), TerrainMeshStyle::Flat);
    }

    #[test]
    fn test_terrain_vegetation_default_is_none() {
        assert_eq!(TerrainVegetation::default(), TerrainVegetation::None);
    }

    #[test]
    fn test_terrain_database_not_found_error_message() {
        let err = TerrainDatabaseError::NotFound(13042);
        assert!(err.to_string().contains("13042"));
    }
}
