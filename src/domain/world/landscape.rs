// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Landscape definition, placement, and mesh registry support.
//!
//! Landscape data represents static environmental decoration such as trees,
//! shrubs, brush, rocks, ground cover, and ruins. Unlike furniture, landscape
//! objects are usually non-interactable and are placed many times on outdoor
//! maps. Unlike [`TileVisualMetadata`](crate::domain::world::TileVisualMetadata),
//! landscape placements are authored objects that can share a tile and can use
//! imported mesh assets.
//!
//! # Examples
//!
//! ```
//! use antares::domain::types::Position;
//! use antares::domain::world::landscape::{
//!     LandscapeCategory, LandscapeDatabase, LandscapeDefinition, LandscapeFlags,
//!     LandscapePlacement,
//! };
//!
//! let mut db = LandscapeDatabase::new();
//! db.add(LandscapeDefinition {
//!     id: 1,
//!     name: "Oak Tree".to_string(),
//!     category: LandscapeCategory::Tree,
//!     default_scale: 1.0,
//!     color_tint: None,
//!     flags: LandscapeFlags::default(),
//!     icon: Some("🌳".to_string()),
//!     tags: vec!["tree".to_string()],
//!     mesh_id: None,
//!     description: None,
//! })
//! .unwrap();
//!
//! let placement = LandscapePlacement::new(1, Position::new(4, 5));
//! assert_eq!(placement.landscape_id, 1);
//! assert!(db.has_definition(1));
//! ```

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::types::{LandscapeId, LandscapeMeshId, Position};
use crate::domain::visual::creature_database::{CreatureDatabase, CreatureDatabaseError};

/// Errors that can occur when working with landscape definitions and mesh registries.
#[derive(Error, Debug)]
pub enum LandscapeDatabaseError {
    /// File could not be read from disk.
    #[error("Failed to read landscape data file: {0}")]
    ReadError(#[from] std::io::Error),

    /// RON parsing failed.
    #[error("Failed to parse landscape RON data: {0}")]
    ParseError(#[from] ron::error::SpannedError),

    /// A definition with the given ID was not found.
    #[error("Landscape ID {0} not found in database")]
    NotFound(LandscapeId),

    /// An entry with the same ID already exists.
    #[error("Duplicate landscape ID {0} detected")]
    DuplicateId(LandscapeId),

    /// A landscape definition referenced a mesh ID that is not registered.
    #[error("Landscape ID {landscape_id} references missing landscape mesh ID {mesh_id}")]
    MissingMeshReference {
        /// Landscape definition containing the bad reference.
        landscape_id: LandscapeId,
        /// Missing mesh ID.
        mesh_id: LandscapeMeshId,
    },

    /// A map placement referenced a landscape definition that does not exist.
    #[error("Landscape placement at ({x}, {y}) references missing landscape ID {landscape_id}")]
    MissingPlacementDefinition {
        /// Missing landscape definition ID.
        landscape_id: LandscapeId,
        /// Placement x coordinate.
        x: i32,
        /// Placement y coordinate.
        y: i32,
    },

    /// A map placement is outside the map bounds.
    #[error("Landscape placement for ID {landscape_id} at ({x}, {y}) is outside map bounds {width}x{height}")]
    PlacementOutOfBounds {
        /// Landscape definition ID used by the placement.
        landscape_id: LandscapeId,
        /// Placement x coordinate.
        x: i32,
        /// Placement y coordinate.
        y: i32,
        /// Map width.
        width: u32,
        /// Map height.
        height: u32,
    },

    /// Landscape mesh registry or one of its asset files failed to load.
    #[error("Failed to load landscape mesh registry: {0}")]
    MeshRegistryError(#[from] CreatureDatabaseError),
}

/// Broad palette grouping for reusable landscape definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LandscapeCategory {
    /// Trees with trunks and canopies, such as oak, pine, palm, willow, or dead trees.
    #[default]
    Tree,
    /// Low woody plants, bushes, and hedges.
    Shrub,
    /// Dense brush or thickets.
    Brush,
    /// Boulders, rock clusters, crystals, and similar mineral props.
    Rock,
    /// Tall grass clumps or authored grass meshes.
    Grass,
    /// Non-blocking small ground-cover details such as flowers or moss.
    GroundCover,
    /// Broken columns, wall fragments, and outdoor ruins.
    Ruin,
    /// Campaign-specific landscape that does not fit a standard category.
    Custom,
}

impl LandscapeCategory {
    /// Returns all categories in the display order used by SDK controls.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::landscape::LandscapeCategory;
    ///
    /// assert!(LandscapeCategory::all().contains(&LandscapeCategory::Tree));
    /// ```
    pub const fn all() -> &'static [LandscapeCategory] {
        &[
            LandscapeCategory::Tree,
            LandscapeCategory::Shrub,
            LandscapeCategory::Brush,
            LandscapeCategory::Rock,
            LandscapeCategory::Grass,
            LandscapeCategory::GroundCover,
            LandscapeCategory::Ruin,
            LandscapeCategory::Custom,
        ]
    }

    /// Returns the human-readable category name.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::landscape::LandscapeCategory;
    ///
    /// assert_eq!(LandscapeCategory::GroundCover.name(), "Ground Cover");
    /// ```
    pub const fn name(self) -> &'static str {
        match self {
            LandscapeCategory::Tree => "Tree",
            LandscapeCategory::Shrub => "Shrub",
            LandscapeCategory::Brush => "Brush",
            LandscapeCategory::Rock => "Rock",
            LandscapeCategory::Grass => "Grass",
            LandscapeCategory::GroundCover => "Ground Cover",
            LandscapeCategory::Ruin => "Ruin",
            LandscapeCategory::Custom => "Custom",
        }
    }

    /// Returns the default icon used when a definition has no explicit icon.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::landscape::LandscapeCategory;
    ///
    /// assert_eq!(LandscapeCategory::Rock.default_icon(), "🪨");
    /// ```
    pub const fn default_icon(self) -> &'static str {
        match self {
            LandscapeCategory::Tree => "🌳",
            LandscapeCategory::Shrub => "🌿",
            LandscapeCategory::Brush => "🌾",
            LandscapeCategory::Rock => "🪨",
            LandscapeCategory::Grass => "☘️",
            LandscapeCategory::GroundCover => "🌱",
            LandscapeCategory::Ruin => "🏛️",
            LandscapeCategory::Custom => "◇",
        }
    }
}

/// Default behavior flags for a landscape definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LandscapeFlags {
    /// Whether the landscape object blocks party movement when map systems opt into collision.
    #[serde(default)]
    pub blocking: bool,
}

/// A reusable landscape template loaded from `data/landscape.ron`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandscapeDefinition {
    /// Unique landscape definition identifier within a campaign.
    pub id: LandscapeId,
    /// Human-readable display name.
    pub name: String,
    /// Palette category used by the importer and map editor.
    pub category: LandscapeCategory,
    /// Default scale multiplier applied to placements without overrides.
    pub default_scale: f32,
    /// Optional default RGB tint applied multiplicatively to the mesh material.
    #[serde(default)]
    pub color_tint: Option<[f32; 3]>,
    /// Default behavior flags.
    #[serde(default)]
    pub flags: LandscapeFlags,
    /// Optional emoji/icon override for SDK palettes.
    #[serde(default)]
    pub icon: Option<String>,
    /// Free-form tags for SDK filtering.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional custom mesh from `landscape_mesh_registry.ron`.
    #[serde(default)]
    pub mesh_id: Option<LandscapeMeshId>,
    /// Optional flavor text displayed in editor previews.
    #[serde(default)]
    pub description: Option<String>,
}

impl LandscapeDefinition {
    /// Returns the icon shown in SDK palettes.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::landscape::{LandscapeCategory, LandscapeDefinition, LandscapeFlags};
    ///
    /// let def = LandscapeDefinition {
    ///     id: 1,
    ///     name: "Boulder".to_string(),
    ///     category: LandscapeCategory::Rock,
    ///     default_scale: 1.0,
    ///     color_tint: None,
    ///     flags: LandscapeFlags::default(),
    ///     icon: None,
    ///     tags: vec![],
    ///     mesh_id: None,
    ///     description: None,
    /// };
    /// assert_eq!(def.display_icon(), "🪨");
    /// ```
    pub fn display_icon(&self) -> &str {
        self.icon
            .as_deref()
            .unwrap_or_else(|| self.category.default_icon())
    }

    /// Returns `true` when this definition references an imported mesh.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::landscape::{LandscapeCategory, LandscapeDefinition, LandscapeFlags};
    ///
    /// let def = LandscapeDefinition {
    ///     id: 1,
    ///     name: "Imported Oak".to_string(),
    ///     category: LandscapeCategory::Tree,
    ///     default_scale: 1.0,
    ///     color_tint: None,
    ///     flags: LandscapeFlags::default(),
    ///     icon: None,
    ///     tags: vec![],
    ///     mesh_id: Some(11001),
    ///     description: None,
    /// };
    /// assert!(def.has_custom_mesh());
    /// ```
    pub fn has_custom_mesh(&self) -> bool {
        self.mesh_id.is_some()
    }
}

/// A single authored landscape instance on a map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandscapePlacement {
    /// Reusable landscape definition to spawn.
    pub landscape_id: LandscapeId,
    /// Tile position of the placement.
    pub position: Position,
    /// Optional sub-tile X/Z offset from the tile center, in world units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<[f32; 2]>,
    /// Optional vertical offset from the ground plane.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_offset: Option<f32>,
    /// Optional rotation around the Y axis in degrees.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_y: Option<f32>,
    /// Optional scale override. When absent, the definition's default scale is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f32>,
    /// Optional RGB tint override applied multiplicatively to the mesh material.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_tint: Option<[f32; 3]>,
    /// Optional blocking override. When absent, the definition's flags are used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocking: Option<bool>,
}

impl LandscapePlacement {
    /// Creates a placement with default transform overrides.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::Position;
    /// use antares::domain::world::landscape::LandscapePlacement;
    ///
    /// let placement = LandscapePlacement::new(1, Position::new(2, 3));
    /// assert_eq!(placement.landscape_id, 1);
    /// assert_eq!(placement.position, Position::new(2, 3));
    /// assert_eq!(placement.scale, None);
    /// ```
    pub fn new(landscape_id: LandscapeId, position: Position) -> Self {
        Self {
            landscape_id,
            position,
            offset: None,
            y_offset: None,
            rotation_y: None,
            scale: None,
            color_tint: None,
            blocking: None,
        }
    }

    /// Returns the effective scale for this placement.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::Position;
    /// use antares::domain::world::landscape::LandscapePlacement;
    ///
    /// let mut placement = LandscapePlacement::new(1, Position::new(0, 0));
    /// assert_eq!(placement.effective_scale(1.5), 1.5);
    /// placement.scale = Some(2.0);
    /// assert_eq!(placement.effective_scale(1.5), 2.0);
    /// ```
    pub fn effective_scale(&self, definition_scale: f32) -> f32 {
        self.scale.unwrap_or(definition_scale)
    }

    /// Returns the effective blocking flag for this placement.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::Position;
    /// use antares::domain::world::landscape::LandscapePlacement;
    ///
    /// let mut placement = LandscapePlacement::new(1, Position::new(0, 0));
    /// assert!(placement.effective_blocking(true));
    /// placement.blocking = Some(false);
    /// assert!(!placement.effective_blocking(true));
    /// ```
    pub fn effective_blocking(&self, definition_blocking: bool) -> bool {
        self.blocking.unwrap_or(definition_blocking)
    }
}

/// In-memory index of all [`LandscapeDefinition`] entries for a campaign.
#[derive(Debug, Clone, Default)]
pub struct LandscapeDatabase {
    /// Definitions indexed by `LandscapeId` for O(1) lookup.
    items: HashMap<LandscapeId, LandscapeDefinition>,
}

crate::impl_ron_database!(
    LandscapeDatabase,
    entity: LandscapeDefinition,
    key: LandscapeId,
    error: LandscapeDatabaseError,
    field: items,
    id_of: |d: &LandscapeDefinition| d.id,
    dup_err: LandscapeDatabaseError::DuplicateId,
    read_err: LandscapeDatabaseError::ReadError,
    parse_err: LandscapeDatabaseError::ParseError,
);

impl LandscapeDatabase {
    /// Creates an empty landscape database.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::landscape::LandscapeDatabase;
    ///
    /// let db = LandscapeDatabase::new();
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
    /// Returns [`LandscapeDatabaseError::DuplicateId`] if a definition with the
    /// same `id` already exists.
    pub fn add(&mut self, def: LandscapeDefinition) -> Result<(), LandscapeDatabaseError> {
        if self.items.contains_key(&def.id) {
            return Err(LandscapeDatabaseError::DuplicateId(def.id));
        }
        self.items.insert(def.id, def);
        Ok(())
    }

    /// Returns a reference to the definition with the given ID, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::landscape::LandscapeDatabase;
    ///
    /// let db = LandscapeDatabase::new();
    /// assert!(db.get_by_id(1).is_none());
    /// ```
    pub fn get_by_id(&self, id: LandscapeId) -> Option<&LandscapeDefinition> {
        self.items.get(&id)
    }

    /// Returns the first definition whose `name` field matches case-sensitively.
    pub fn get_by_name(&self, name: &str) -> Option<&LandscapeDefinition> {
        self.items.values().find(|d| d.name == name)
    }

    /// Returns all definitions belonging to the specified category.
    pub fn get_by_category(&self, category: LandscapeCategory) -> Vec<&LandscapeDefinition> {
        self.items
            .values()
            .filter(|d| d.category == category)
            .collect()
    }

    /// Returns all definitions in the database.
    pub fn all_definitions(&self) -> Vec<&LandscapeDefinition> {
        self.items.values().collect()
    }

    /// Returns the number of definitions in the database.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the database contains no definitions.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns `true` if a definition with the given ID exists.
    pub fn has_definition(&self, id: LandscapeId) -> bool {
        self.items.contains_key(&id)
    }

    /// Validates every definition-to-mesh reference.
    ///
    /// # Errors
    ///
    /// Returns [`LandscapeDatabaseError::MissingMeshReference`] if a definition
    /// references a mesh that is not registered.
    pub fn validate_mesh_references(
        &self,
        meshes: &LandscapeMeshDatabase,
    ) -> Result<(), LandscapeDatabaseError> {
        for def in self.items.values() {
            if let Some(mesh_id) = def.mesh_id {
                if !meshes.has_mesh(mesh_id) {
                    return Err(LandscapeDatabaseError::MissingMeshReference {
                        landscape_id: def.id,
                        mesh_id,
                    });
                }
            }
        }

        Ok(())
    }

    /// Validates that all placements reference known definitions and are in bounds.
    ///
    /// # Errors
    ///
    /// Returns a landscape validation error for the first invalid placement.
    pub fn validate_placements(
        &self,
        placements: &[LandscapePlacement],
        width: u32,
        height: u32,
    ) -> Result<(), LandscapeDatabaseError> {
        for placement in placements {
            if !self.has_definition(placement.landscape_id) {
                return Err(LandscapeDatabaseError::MissingPlacementDefinition {
                    landscape_id: placement.landscape_id,
                    x: placement.position.x,
                    y: placement.position.y,
                });
            }

            if placement.position.x < 0
                || placement.position.y < 0
                || placement.position.x >= width as i32
                || placement.position.y >= height as i32
            {
                return Err(LandscapeDatabaseError::PlacementOutOfBounds {
                    landscape_id: placement.landscape_id,
                    x: placement.position.x,
                    y: placement.position.y,
                    width,
                    height,
                });
            }
        }

        Ok(())
    }
}

/// Database of custom landscape meshes loaded from `landscape_mesh_registry.ron`.
///
/// Landscape mesh assets reuse the same underlying
/// [`crate::domain::visual::CreatureDefinition`] format as creature, item, and
/// furniture mesh assets. This wrapper keeps landscape mesh concerns separate
/// while reusing the validated registry loader.
#[derive(Debug, Clone, Default)]
pub struct LandscapeMeshDatabase {
    /// Wrapped creature-style registry database for landscape mesh assets.
    inner: CreatureDatabase,
}

impl LandscapeMeshDatabase {
    /// Creates a new, empty landscape mesh database.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::landscape::LandscapeMeshDatabase;
    ///
    /// let db = LandscapeMeshDatabase::new();
    /// assert!(db.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            inner: CreatureDatabase::new(),
        }
    }

    /// Loads a landscape mesh database from a registry RON file.
    ///
    /// The registry uses the same `CreatureReference` format as creature,
    /// item, and furniture mesh registries. Each entry points at a
    /// `CreatureDefinition` RON file under the campaign root.
    ///
    /// # Errors
    ///
    /// Returns an error if the registry file or any referenced mesh asset file
    /// cannot be read, parsed, or validated.
    pub fn load_from_registry(
        registry_path: &Path,
        campaign_root: &Path,
    ) -> Result<Self, CreatureDatabaseError> {
        let inner = CreatureDatabase::load_from_registry(registry_path, campaign_root)?;
        Ok(Self { inner })
    }

    /// Returns the underlying creature-style mesh database.
    pub fn as_creature_database(&self) -> &CreatureDatabase {
        &self.inner
    }

    /// Returns `true` when no landscape mesh entries are loaded.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the number of registered landscape mesh entries.
    pub fn count(&self) -> usize {
        self.inner.count()
    }

    /// Returns `true` if a mesh with the given landscape mesh ID exists.
    pub fn has_mesh(&self, id: LandscapeMeshId) -> bool {
        self.inner.has_creature(id)
    }

    /// Returns a registered mesh asset by landscape mesh ID.
    pub fn get_mesh(
        &self,
        id: LandscapeMeshId,
    ) -> Option<&crate::domain::visual::CreatureDefinition> {
        self.inner.get_creature(id)
    }

    /// Validates all registered landscape mesh assets.
    ///
    /// # Errors
    ///
    /// Returns a validation error if any referenced mesh asset is malformed.
    pub fn validate(&self) -> Result<(), CreatureDatabaseError> {
        self.inner.validate()
    }

    /// Validates mesh texture paths and optional on-disk texture existence.
    ///
    /// Mesh texture paths must start with `assets/`. When `campaign_root` is
    /// provided, every texture path must also exist below that root.
    ///
    /// # Errors
    ///
    /// Returns a validation error for the first invalid texture path.
    pub fn validate_texture_paths(
        &self,
        campaign_root: Option<&Path>,
    ) -> Result<(), CreatureDatabaseError> {
        for creature in self.inner.all_creatures() {
            for mesh in &creature.meshes {
                if let Some(texture_path) = &mesh.texture_path {
                    let mesh_name = mesh.name.as_deref().unwrap_or("<unnamed>");
                    if !texture_path.starts_with("assets/") {
                        return Err(CreatureDatabaseError::ValidationError(
                            creature.id,
                            format!(
                                "Landscape mesh '{}' (ID {}, part '{}') texture path '{}' must start with 'assets/'",
                                creature.name, creature.id, mesh_name, texture_path
                            ),
                        ));
                    }

                    if let Some(root) = campaign_root {
                        let absolute = root.join(texture_path);
                        if !absolute.exists() {
                            return Err(CreatureDatabaseError::ValidationError(
                                creature.id,
                                format!(
                                    "Landscape mesh '{}' (ID {}, part '{}') texture '{}' does not exist under campaign root '{}'",
                                    creature.name,
                                    creature.id,
                                    mesh_name,
                                    texture_path,
                                    root.display()
                                ),
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::visual::{
        AlphaMode, CreatureDefinition, CreatureReference, MaterialDefinition, MeshDefinition,
        MeshTransform,
    };
    use std::fs;
    use tempfile::TempDir;

    fn make_definition(id: LandscapeId, name: &str) -> LandscapeDefinition {
        LandscapeDefinition {
            id,
            name: name.to_string(),
            category: LandscapeCategory::Tree,
            default_scale: 1.0,
            color_tint: None,
            flags: LandscapeFlags::default(),
            icon: None,
            tags: vec![],
            mesh_id: None,
            description: None,
        }
    }

    fn write_mesh_registry_fixture(
        root: &std::path::Path,
        texture_path: &str,
    ) -> std::path::PathBuf {
        let mesh_dir = root.join("assets/meshes/landscape");
        fs::create_dir_all(&mesh_dir).unwrap();
        let data_dir = root.join("data");
        fs::create_dir_all(&data_dir).unwrap();

        let mesh = MeshDefinition {
            name: Some("leaf_card".to_string()),
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            normals: Some(vec![[0.0, 0.0, 1.0]; 3]),
            uvs: Some(vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]),
            color: [1.0, 1.0, 1.0, 1.0],
            lod_levels: None,
            lod_distances: None,
            material: Some(MaterialDefinition {
                base_color: [1.0, 1.0, 1.0, 1.0],
                metallic: 0.0,
                roughness: 0.8,
                emissive: None,
                alpha_mode: AlphaMode::Mask,
            }),
            texture_path: Some(texture_path.to_string()),
        };
        let creature = CreatureDefinition {
            id: 11042,
            name: "Fixture Landscape Mesh".to_string(),
            meshes: vec![mesh],
            mesh_transforms: vec![MeshTransform::identity()],
            scale: 1.0,
            color_tint: None,
        };
        let mesh_path = mesh_dir.join("fixture.ron");
        fs::write(
            &mesh_path,
            ron::ser::to_string_pretty(&creature, ron::ser::PrettyConfig::new()).unwrap(),
        )
        .unwrap();

        let registry = vec![CreatureReference {
            id: 11042,
            name: "Fixture Landscape Mesh".to_string(),
            filepath: "assets/meshes/landscape/fixture.ron".to_string(),
        }];
        let registry_path = data_dir.join("landscape_mesh_registry.ron");
        fs::write(
            &registry_path,
            ron::ser::to_string_pretty(&registry, ron::ser::PrettyConfig::new()).unwrap(),
        )
        .unwrap();
        registry_path
    }

    #[test]
    fn test_landscape_category_names_and_icons() {
        assert_eq!(LandscapeCategory::Tree.name(), "Tree");
        assert_eq!(LandscapeCategory::GroundCover.name(), "Ground Cover");
        assert_eq!(LandscapeCategory::Rock.default_icon(), "🪨");
        assert!(LandscapeCategory::all().contains(&LandscapeCategory::Custom));
    }

    #[test]
    fn test_landscape_definition_display_icon_override() {
        let def = LandscapeDefinition {
            icon: Some("🌴".to_string()),
            ..make_definition(1, "Palm")
        };

        assert_eq!(def.display_icon(), "🌴");
    }

    #[test]
    fn test_landscape_definition_display_icon_fallback() {
        let def = LandscapeDefinition {
            category: LandscapeCategory::Brush,
            ..make_definition(1, "Brush")
        };

        assert_eq!(def.display_icon(), LandscapeCategory::Brush.default_icon());
    }

    #[test]
    fn test_landscape_definition_has_custom_mesh() {
        let def = LandscapeDefinition {
            mesh_id: Some(11001),
            ..make_definition(1, "Oak")
        };

        assert!(def.has_custom_mesh());
    }

    #[test]
    fn test_landscape_placement_defaults() {
        let placement = LandscapePlacement::new(1, Position::new(3, 4));

        assert_eq!(placement.landscape_id, 1);
        assert_eq!(placement.position, Position::new(3, 4));
        assert_eq!(placement.effective_scale(1.25), 1.25);
        assert!(!placement.effective_blocking(false));
    }

    #[test]
    fn test_landscape_placement_overrides() {
        let mut placement = LandscapePlacement::new(1, Position::new(3, 4));
        placement.scale = Some(2.0);
        placement.blocking = Some(false);

        assert_eq!(placement.effective_scale(1.25), 2.0);
        assert!(!placement.effective_blocking(true));
    }

    #[test]
    fn test_landscape_database_add_and_lookup() {
        let mut db = LandscapeDatabase::new();
        db.add(make_definition(1, "Oak")).unwrap();

        assert_eq!(db.len(), 1);
        assert!(db.has_definition(1));
        assert!(db.get_by_id(1).is_some());
        assert!(db.get_by_name("Oak").is_some());
        assert_eq!(db.get_by_category(LandscapeCategory::Tree).len(), 1);
    }

    #[test]
    fn test_landscape_database_rejects_duplicate_id() {
        let mut db = LandscapeDatabase::new();
        db.add(make_definition(1, "Oak")).unwrap();

        assert!(matches!(
            db.add(make_definition(1, "Pine")),
            Err(LandscapeDatabaseError::DuplicateId(1))
        ));
    }

    #[test]
    fn test_landscape_database_load_from_string() {
        let ron = r#"[
            (
                id: 1,
                name: "Oak Tree",
                category: Tree,
                default_scale: 1.0,
                color_tint: None,
                flags: (blocking: false),
                icon: Some("🌳"),
                tags: ["tree"],
                mesh_id: None,
                description: None,
            ),
        ]"#;

        let db = LandscapeDatabase::load_from_string(ron).unwrap();
        assert!(db.has_definition(1));
    }

    #[test]
    fn test_landscape_placement_ron_roundtrip() {
        let placement = LandscapePlacement {
            landscape_id: 1,
            position: Position::new(2, 3),
            offset: Some([0.1, -0.2]),
            y_offset: Some(0.05),
            rotation_y: Some(45.0),
            scale: Some(1.2),
            color_tint: Some([0.8, 0.9, 1.0]),
            blocking: Some(true),
        };

        let ron = ron::to_string(&placement).unwrap();
        let roundtrip: LandscapePlacement = ron::from_str(&ron).unwrap();
        assert_eq!(roundtrip, placement);
    }

    #[test]
    fn test_validate_placements_rejects_missing_definition() {
        let db = LandscapeDatabase::new();
        let placements = [LandscapePlacement::new(99, Position::new(1, 1))];

        assert!(matches!(
            db.validate_placements(&placements, 10, 10),
            Err(LandscapeDatabaseError::MissingPlacementDefinition { .. })
        ));
    }

    #[test]
    fn test_validate_placements_rejects_out_of_bounds() {
        let mut db = LandscapeDatabase::new();
        db.add(make_definition(1, "Oak")).unwrap();
        let placements = [LandscapePlacement::new(1, Position::new(10, 1))];

        assert!(matches!(
            db.validate_placements(&placements, 10, 10),
            Err(LandscapeDatabaseError::PlacementOutOfBounds { .. })
        ));
    }

    #[test]
    fn test_validate_texture_paths_rejects_non_assets_prefix_with_context() {
        let temp = TempDir::new().unwrap();
        let registry_path = write_mesh_registry_fixture(temp.path(), "textures/trees/leaf.png");
        let db = LandscapeMeshDatabase::load_from_registry(&registry_path, temp.path()).unwrap();

        let error = db.validate_texture_paths(None).unwrap_err().to_string();

        assert!(error.contains("Fixture Landscape Mesh"));
        assert!(error.contains("leaf_card"));
        assert!(error.contains("textures/trees/leaf.png"));
        assert!(error.contains("assets/"));
    }

    #[test]
    fn test_validate_texture_paths_reports_missing_texture_with_context() {
        let temp = TempDir::new().unwrap();
        let registry_path =
            write_mesh_registry_fixture(temp.path(), "assets/textures/trees/missing.png");
        let db = LandscapeMeshDatabase::load_from_registry(&registry_path, temp.path()).unwrap();

        let error = db
            .validate_texture_paths(Some(temp.path()))
            .unwrap_err()
            .to_string();

        assert!(error.contains("Fixture Landscape Mesh"));
        assert!(error.contains("leaf_card"));
        assert!(error.contains("assets/textures/trees/missing.png"));
        assert!(error.contains(&temp.path().display().to_string()));
    }

    #[test]
    fn test_validate_texture_paths_accepts_valid_assets_texture() {
        let temp = TempDir::new().unwrap();
        let texture_path = temp.path().join("assets/textures/trees/leaf.png");
        fs::create_dir_all(texture_path.parent().unwrap()).unwrap();
        fs::write(&texture_path, b"fake png bytes").unwrap();
        let registry_path =
            write_mesh_registry_fixture(temp.path(), "assets/textures/trees/leaf.png");
        let db = LandscapeMeshDatabase::load_from_registry(&registry_path, temp.path()).unwrap();

        db.validate_texture_paths(Some(temp.path())).unwrap();
    }

    #[test]
    fn test_test_campaign_phase1_landscape_mesh_fixture_integrity() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/test_campaign");
        let registry_path = root.join("data/landscape_mesh_registry.ron");
        let db = LandscapeMeshDatabase::load_from_registry(&registry_path, &root).unwrap();

        assert_eq!(db.count(), 5);
        db.validate_texture_paths(Some(&root)).unwrap();

        let mut mesh_count = 0;
        for creature in db.as_creature_database().all_creatures() {
            assert!(
                !creature.meshes.is_empty(),
                "{} has no meshes",
                creature.name
            );
            for mesh in &creature.meshes {
                mesh_count += 1;
                assert!(
                    !mesh.vertices.is_empty(),
                    "{} has no vertices",
                    creature.name
                );
                assert!(!mesh.indices.is_empty(), "{} has no indices", creature.name);
                if let Some(normals) = &mesh.normals {
                    assert_eq!(normals.len(), mesh.vertices.len());
                }
                if let Some(uvs) = &mesh.uvs {
                    assert_eq!(uvs.len(), mesh.vertices.len());
                }
                assert!(mesh.material.is_some(), "{} has no material", creature.name);
                let texture_path = mesh
                    .texture_path
                    .as_deref()
                    .expect("landscape fixture texture");
                assert!(texture_path.starts_with("assets/"));
                assert!(
                    root.join(texture_path).exists(),
                    "missing texture {texture_path}"
                );
                if texture_path.contains("foliage_") {
                    assert_eq!(
                        mesh.material.as_ref().unwrap().alpha_mode,
                        AlphaMode::Mask,
                        "foliage texture {texture_path} should use alpha masking"
                    );
                }
            }
        }
        assert!(mesh_count >= 5);
    }
}
