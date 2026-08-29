// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Terrain definition and database support.
//!
//! Terrain data represents the fundamental ground type of each map tile.
//! Terrain definitions are loaded from `data/terrain.ron` into a
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

    /// Merges all definitions from `other` into this database.
    ///
    /// For each entry in `other`:
    /// - If `self` already has a definition with that ID, it is **replaced**.
    /// - If the ID is new, it is **added**.
    ///
    /// This is the mechanism by which a campaign `terrain.ron` can override a
    /// built-in terrain definition or extend the database with new entries.
    /// ID validation is skipped during merge; callers are responsible for
    /// ensuring `other` contains only valid IDs (>= [`TERRAIN_ID_MIN`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::terrain::{
    ///     TerrainDatabase, TerrainDefinition, TerrainMeshStyle, TerrainVegetation,
    ///     builtin_terrain_db, TERRAIN_GRASS,
    /// };
    ///
    /// let mut base = builtin_terrain_db();
    /// let mut campaign = TerrainDatabase::new();
    /// campaign.add(TerrainDefinition {
    ///     id: TERRAIN_GRASS,
    ///     name: "Lush Grass".to_string(),
    ///     texture_path: "assets/textures/terrain/lush_grass.png".to_string(),
    ///     roughness: 0.95,
    ///     mesh_style: TerrainMeshStyle::Flat,
    ///     vegetation: TerrainVegetation::GrassCover,
    ///     blocked: false,
    ///     height: 0.0,
    ///     color: [0.1, 0.7, 0.1],
    /// }).unwrap();
    ///
    /// base.merge(campaign);
    ///
    /// // The built-in Grass is now replaced by the campaign's "Lush Grass"
    /// assert_eq!(base.get_by_id(TERRAIN_GRASS).unwrap().name, "Lush Grass");
    /// ```
    pub fn merge(&mut self, other: TerrainDatabase) {
        for (id, def) in other.items {
            self.items.insert(id, def);
        }
    }
}

// ===== Built-in terrain ID constants =====

/// Built-in terrain ID for Ground (flat, walkable).
pub const TERRAIN_GROUND: TerrainId = 13_000;
/// Built-in terrain ID for Grass (flat, walkable, supports grass cover vegetation).
pub const TERRAIN_GRASS: TerrainId = 13_001;
/// Built-in terrain ID for Water (water mesh, blocks movement by default).
pub const TERRAIN_WATER: TerrainId = 13_002;
/// Built-in terrain ID for Lava (flat, walkable, damages party).
pub const TERRAIN_LAVA: TerrainId = 13_003;
/// Built-in terrain ID for Swamp (flat, walkable, slows movement).
pub const TERRAIN_SWAMP: TerrainId = 13_004;
/// Built-in terrain ID for Stone (flat, walkable stone floor).
pub const TERRAIN_STONE: TerrainId = 13_005;
/// Built-in terrain ID for Dirt (flat, walkable dirt path).
pub const TERRAIN_DIRT: TerrainId = 13_006;
/// Built-in terrain ID for Forest (flat, walkable, supports full forest vegetation).
pub const TERRAIN_FOREST: TerrainId = 13_007;
/// Built-in terrain ID for Mountain (mountain mesh, blocks movement by default).
pub const TERRAIN_MOUNTAIN: TerrainId = 13_008;
/// Built-in terrain ID for Sand (flat, walkable).
pub const TERRAIN_SAND: TerrainId = 13_009;
/// Built-in terrain ID for Snow (flat, walkable).
pub const TERRAIN_SNOW: TerrainId = 13_010;
/// Built-in terrain ID for Ice (flat, walkable).
pub const TERRAIN_ICE: TerrainId = 13_011;

/// Short-name aliases for the twelve built-in terrain IDs.
///
/// These are re-exports of the top-level `TERRAIN_*` constants with shorter
/// names for ergonomic use in engine code and tests.
///
/// # Examples
///
/// ```
/// use antares::domain::world::terrain::builtin;
///
/// assert_eq!(builtin::GROUND, 13_000);
/// assert_eq!(builtin::WATER, 13_002);
/// assert!(builtin::WATER_BLOCKED);
/// ```
pub mod builtin {
    use super::*;

    /// Ground terrain ID (flat, walkable).
    pub const GROUND: TerrainId = TERRAIN_GROUND;
    /// Grass terrain ID (flat, walkable, grass-cover vegetation).
    pub const GRASS: TerrainId = TERRAIN_GRASS;
    /// Water terrain ID (water mesh, blocked by default).
    pub const WATER: TerrainId = TERRAIN_WATER;
    /// Lava terrain ID (flat, walkable, damages party).
    pub const LAVA: TerrainId = TERRAIN_LAVA;
    /// Swamp terrain ID (flat, walkable, slows movement).
    pub const SWAMP: TerrainId = TERRAIN_SWAMP;
    /// Stone terrain ID (flat, walkable).
    pub const STONE: TerrainId = TERRAIN_STONE;
    /// Dirt terrain ID (flat, walkable).
    pub const DIRT: TerrainId = TERRAIN_DIRT;
    /// Forest terrain ID (flat, walkable, forest vegetation).
    pub const FOREST: TerrainId = TERRAIN_FOREST;
    /// Mountain terrain ID (mountain mesh, blocked by default).
    pub const MOUNTAIN: TerrainId = TERRAIN_MOUNTAIN;
    /// Sand terrain ID (flat, walkable).
    pub const SAND: TerrainId = TERRAIN_SAND;
    /// Snow terrain ID (flat, walkable).
    pub const SNOW: TerrainId = TERRAIN_SNOW;
    /// Ice terrain ID (flat, walkable, near-water slipperiness).
    pub const ICE: TerrainId = TERRAIN_ICE;

    /// `true` because Water blocks movement by default.
    pub const WATER_BLOCKED: bool = true;
    /// `true` because Mountain blocks movement by default.
    pub const MOUNTAIN_BLOCKED: bool = true;
}

/// Returns a [`TerrainDatabase`] pre-populated with all twelve built-in terrain
/// definitions (Ground through Ice, IDs 13000–13011).
///
/// Use this in tests and SDK tooling that need a database but have not yet
/// loaded a campaign's `terrain.ron`.
///
/// # Examples
///
/// ```
/// use antares::domain::world::terrain::{builtin_terrain_db, TERRAIN_GROUND, TERRAIN_WATER};
///
/// let db = builtin_terrain_db();
/// assert!(db.has_definition(TERRAIN_GROUND));
/// assert!(db.get_by_id(TERRAIN_WATER).unwrap().blocked);
/// ```
// SAFETY: Built-in terrain definitions carry hardcoded IDs >= TERRAIN_ID_MIN that are
// guaranteed unique by construction. `add` cannot fail for these entries.
#[allow(clippy::expect_used)]
pub fn builtin_terrain_db() -> TerrainDatabase {
    let mut db = TerrainDatabase::new();
    for def in builtin_terrain_definitions() {
        db.add(def)
            .expect("built-in terrain IDs are unique and valid");
    }
    db
}

/// Returns all twelve built-in [`TerrainDefinition`] entries as a `Vec`.
///
/// Prefer [`builtin_terrain_db`] when you need a ready-to-query database.
pub fn builtin_terrain_definitions() -> Vec<TerrainDefinition> {
    vec![
        TerrainDefinition {
            id: TERRAIN_GROUND,
            name: "Ground".to_string(),
            texture_path: "assets/textures/terrain/ground.png".to_string(),
            roughness: 0.95,
            mesh_style: TerrainMeshStyle::Flat,
            vegetation: TerrainVegetation::None,
            blocked: false,
            height: 0.0,
            color: [0.6_f32, 0.5, 0.4],
        },
        TerrainDefinition {
            id: TERRAIN_GRASS,
            name: "Grass".to_string(),
            texture_path: "assets/textures/terrain/grass.png".to_string(),
            roughness: 0.90,
            mesh_style: TerrainMeshStyle::Flat,
            vegetation: TerrainVegetation::GrassCover,
            blocked: false,
            height: 0.0,
            color: [0.3_f32, 0.6, 0.2],
        },
        TerrainDefinition {
            id: TERRAIN_WATER,
            name: "Water".to_string(),
            texture_path: "assets/textures/terrain/water.png".to_string(),
            roughness: 0.10,
            mesh_style: TerrainMeshStyle::Water,
            vegetation: TerrainVegetation::None,
            blocked: true,
            height: 0.0,
            color: [0.1_f32, 0.3, 0.8],
        },
        TerrainDefinition {
            id: TERRAIN_LAVA,
            name: "Lava".to_string(),
            texture_path: "assets/textures/terrain/lava.png".to_string(),
            roughness: 0.60,
            mesh_style: TerrainMeshStyle::Flat,
            vegetation: TerrainVegetation::None,
            blocked: false,
            height: 0.0,
            color: [0.8_f32, 0.3, 0.2],
        },
        TerrainDefinition {
            id: TERRAIN_SWAMP,
            name: "Swamp".to_string(),
            texture_path: "assets/textures/terrain/swamp.png".to_string(),
            roughness: 0.88,
            mesh_style: TerrainMeshStyle::Flat,
            vegetation: TerrainVegetation::None,
            blocked: false,
            height: 0.0,
            color: [0.35_f32, 0.3, 0.2],
        },
        TerrainDefinition {
            id: TERRAIN_STONE,
            name: "Stone".to_string(),
            texture_path: "assets/textures/terrain/stone.png".to_string(),
            roughness: 0.75,
            mesh_style: TerrainMeshStyle::Flat,
            vegetation: TerrainVegetation::None,
            blocked: false,
            height: 0.0,
            color: [0.5_f32, 0.5, 0.5],
        },
        TerrainDefinition {
            id: TERRAIN_DIRT,
            name: "Dirt".to_string(),
            texture_path: "assets/textures/terrain/dirt.png".to_string(),
            roughness: 0.92,
            mesh_style: TerrainMeshStyle::Flat,
            vegetation: TerrainVegetation::None,
            blocked: false,
            height: 0.0,
            color: [0.55_f32, 0.4, 0.25],
        },
        TerrainDefinition {
            id: TERRAIN_FOREST,
            name: "Forest".to_string(),
            texture_path: "assets/textures/terrain/forest_floor.png".to_string(),
            roughness: 0.90,
            mesh_style: TerrainMeshStyle::Flat,
            vegetation: TerrainVegetation::Forest,
            blocked: false,
            height: 2.2,
            color: [0.1_f32, 0.4, 0.1],
        },
        TerrainDefinition {
            id: TERRAIN_MOUNTAIN,
            name: "Mountain".to_string(),
            texture_path: "assets/textures/terrain/mountain.png".to_string(),
            roughness: 0.85,
            mesh_style: TerrainMeshStyle::Mountain,
            vegetation: TerrainVegetation::None,
            blocked: true,
            height: 3.0,
            color: [0.4_f32, 0.4, 0.4],
        },
        TerrainDefinition {
            id: TERRAIN_SAND,
            name: "Sand".to_string(),
            texture_path: "assets/textures/terrain/sand.png".to_string(),
            roughness: 0.85,
            mesh_style: TerrainMeshStyle::Flat,
            vegetation: TerrainVegetation::None,
            blocked: false,
            height: 0.0,
            color: [0.9_f32, 0.85, 0.6],
        },
        TerrainDefinition {
            id: TERRAIN_SNOW,
            name: "Snow".to_string(),
            texture_path: "assets/textures/terrain/snow.png".to_string(),
            roughness: 0.30,
            mesh_style: TerrainMeshStyle::Flat,
            vegetation: TerrainVegetation::None,
            blocked: false,
            height: 0.0,
            color: [0.95_f32, 0.95, 1.0],
        },
        TerrainDefinition {
            id: TERRAIN_ICE,
            name: "Ice".to_string(),
            texture_path: "assets/textures/terrain/ice.png".to_string(),
            roughness: 0.05,
            mesh_style: TerrainMeshStyle::Flat,
            vegetation: TerrainVegetation::None,
            blocked: false,
            height: 0.0,
            color: [0.8_f32, 0.9, 1.0],
        },
    ]
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

    #[test]
    fn test_builtin_terrain_db_has_all_twelve_entries() {
        let db = builtin_terrain_db();
        assert_eq!(db.len(), 12);
        for id in [
            TERRAIN_GROUND,
            TERRAIN_GRASS,
            TERRAIN_WATER,
            TERRAIN_LAVA,
            TERRAIN_SWAMP,
            TERRAIN_STONE,
            TERRAIN_DIRT,
            TERRAIN_FOREST,
            TERRAIN_MOUNTAIN,
            TERRAIN_SAND,
            TERRAIN_SNOW,
            TERRAIN_ICE,
        ] {
            assert!(db.has_definition(id), "missing built-in terrain ID {id}");
        }
    }

    #[test]
    fn test_builtin_terrain_db_water_and_mountain_are_blocked() {
        let db = builtin_terrain_db();
        assert!(db.get_by_id(TERRAIN_WATER).unwrap().blocked);
        assert!(db.get_by_id(TERRAIN_MOUNTAIN).unwrap().blocked);
        assert!(!db.get_by_id(TERRAIN_GROUND).unwrap().blocked);
        assert!(!db.get_by_id(TERRAIN_GRASS).unwrap().blocked);
    }

    #[test]
    fn test_terrain_new_seeds_blocked_from_db() {
        use crate::domain::world::{Tile, WallType};
        let db = builtin_terrain_db();
        let water = Tile::new(0, 0, TERRAIN_WATER, WallType::None, &db);
        assert!(water.blocked);
        let mountain = Tile::new(0, 0, TERRAIN_MOUNTAIN, WallType::None, &db);
        assert!(mountain.blocked);
        let grass = Tile::new(0, 0, TERRAIN_GRASS, WallType::None, &db);
        assert!(!grass.blocked);
        // blocked is still mutable
        let mut tile = Tile::new(0, 0, TERRAIN_WATER, WallType::None, &db);
        tile.blocked = false;
        assert!(!tile.blocked);
    }

    #[test]
    fn test_load_default_terrain_ron() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let terrain_path = std::path::Path::new(manifest_dir).join("data/terrain.ron");
        let db = TerrainDatabase::load_from_file(&terrain_path)
            .expect("data/terrain.ron should parse without error");
        assert_eq!(
            db.len(),
            12,
            "data/terrain.ron must contain exactly 12 entries"
        );
        // Verify all 12 built-in IDs are present
        for id in [
            TERRAIN_GROUND,
            TERRAIN_GRASS,
            TERRAIN_WATER,
            TERRAIN_LAVA,
            TERRAIN_SWAMP,
            TERRAIN_STONE,
            TERRAIN_DIRT,
            TERRAIN_FOREST,
            TERRAIN_MOUNTAIN,
            TERRAIN_SAND,
            TERRAIN_SNOW,
            TERRAIN_ICE,
        ] {
            assert!(
                db.has_definition(id),
                "data/terrain.ron is missing built-in terrain ID {id}"
            );
        }
        // Water and Mountain are blocked
        assert!(
            db.get_by_id(TERRAIN_WATER).unwrap().blocked,
            "Water must be blocked"
        );
        assert!(
            db.get_by_id(TERRAIN_MOUNTAIN).unwrap().blocked,
            "Mountain must be blocked"
        );
    }

    #[test]
    fn test_terrain_database_merge_campaign_override() {
        // Start with the 12 built-ins
        let mut base = builtin_terrain_db();
        assert_eq!(base.len(), 12);

        // Build a "campaign" database:
        // - ID 13001 (Grass) overrides the built-in with a custom name
        // - ID 13100 adds a completely new entry
        let mut campaign = TerrainDatabase::new();
        campaign
            .add(TerrainDefinition {
                id: 13_001,
                name: "Lush Meadow".to_string(),
                texture_path: "assets/textures/terrain/lush_meadow.png".to_string(),
                roughness: 0.92,
                mesh_style: TerrainMeshStyle::Flat,
                vegetation: TerrainVegetation::GrassCover,
                blocked: false,
                height: 0.0,
                color: [0.1, 0.8, 0.1],
            })
            .unwrap();
        campaign
            .add(TerrainDefinition {
                id: 13_100,
                name: "Volcanic Ash".to_string(),
                texture_path: "assets/textures/terrain/volcanic_ash.png".to_string(),
                roughness: 0.70,
                mesh_style: TerrainMeshStyle::Flat,
                vegetation: TerrainVegetation::None,
                blocked: false,
                height: 0.0,
                color: [0.2, 0.2, 0.2],
            })
            .unwrap();

        base.merge(campaign);

        // 12 built-ins + 1 new = 13 total (override replaces, not adds)
        assert_eq!(base.len(), 13);
        // Built-in Grass (13001) is now overridden
        assert_eq!(
            base.get_by_id(13_001).unwrap().name,
            "Lush Meadow",
            "campaign override of ID 13001 should replace built-in"
        );
        // New custom entry (13100) was added
        assert!(
            base.has_definition(13_100),
            "new campaign entry 13100 should be present"
        );
        assert_eq!(base.get_by_id(13_100).unwrap().name, "Volcanic Ash");
        // Other built-ins remain untouched
        assert_eq!(base.get_by_id(TERRAIN_GROUND).unwrap().name, "Ground");
        assert!(base.get_by_id(TERRAIN_WATER).unwrap().blocked);
    }
}
