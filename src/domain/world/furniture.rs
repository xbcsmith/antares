// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Furniture definition and database system
//!
//! This module provides data-driven furniture support: a [`FurnitureDefinition`]
//! struct that represents a named, reusable furniture template and a
//! [`FurnitureDatabase`] that loads and indexes definitions from
//! `furniture.ron` campaign data files.
//!
//! # Architecture Reference
//!
//! See the furniture RON implementation plan in
//! `docs/explanation/furniture_as_ron_implementation_plan.md` for implementation details.
//!
//! # Examples
//!
//! ```
//! use antares::domain::world::furniture::{FurnitureDefinition, FurnitureDatabase};
//! use antares::domain::world::{FurnitureCategory, FurnitureMaterial, FurnitureType, FurnitureFlags};
//! use antares::domain::types::{FurnitureId, FurnitureMeshId};
//!
//! let mut db = FurnitureDatabase::new();
//! assert!(db.is_empty());
//!
//! let def = FurnitureDefinition {
//!     id: 1,
//!     name: "Wooden Throne".to_string(),
//!     category: FurnitureCategory::Seating,
//!     base_type: FurnitureType::Throne,
//!     material: FurnitureMaterial::Wood,
//!     scale: 1.2,
//!     color_tint: None,
//!     flags: FurnitureFlags { lit: false, locked: false, blocking: true },
//!     icon: None,
//!     tags: vec![],
//!     mesh_id: None,
//!     description: Some("A sturdy wooden throne fit for a lord.".to_string()),
//! };
//!
//! db.add(def).expect("Failed to add definition");
//! assert_eq!(db.len(), 1);
//! assert!(db.get_by_id(1).is_some());
//! ```

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::types::{FurnitureId, FurnitureMeshId};
use crate::domain::visual::creature_database::{CreatureDatabase, CreatureDatabaseError};
use crate::domain::world::types::{
    FurnitureCategory, FurnitureFlags, FurnitureMaterial, FurnitureType,
};

// ===== Error Types =====

/// Errors that can occur when working with furniture definitions and mesh registries.
#[derive(Error, Debug)]
pub enum FurnitureDatabaseError {
    /// File could not be read from disk
    #[error("Failed to read furniture data file: {0}")]
    ReadError(#[from] std::io::Error),

    /// RON parsing failed
    #[error("Failed to parse furniture RON data: {0}")]
    ParseError(#[from] ron::error::SpannedError),

    /// A definition with the given ID was not found
    #[error("Furniture ID {0} not found in database")]
    NotFound(FurnitureId),

    /// An entry with the same ID already exists
    #[error("Duplicate furniture ID {0} detected")]
    DuplicateId(FurnitureId),

    /// Furniture mesh registry or one of its asset files failed to load
    #[error("Failed to load furniture mesh registry: {0}")]
    MeshRegistryError(#[from] CreatureDatabaseError),
}

// ===== FurnitureDefinition =====

/// A named, reusable furniture template loaded from `furniture.ron`
///
/// Each definition provides default property values (material, scale, color
/// tint, flags) for a furniture placement.  Map authors can reference a
/// definition by its `id` field in `MapEvent::Furniture` rather than
/// specifying every property inline.
///
/// # Examples
///
/// ```
/// use antares::domain::world::furniture::FurnitureDefinition;
/// use antares::domain::world::{FurnitureCategory, FurnitureMaterial, FurnitureType, FurnitureFlags};
///
/// let def = FurnitureDefinition {
///     id: 7,
///     name: "Wooden Torch".to_string(),
///     category: FurnitureCategory::Lighting,
///     base_type: FurnitureType::Torch,
///     material: FurnitureMaterial::Wood,
///     scale: 1.0,
///     color_tint: Some([1.0, 0.6, 0.2]),
///     flags: FurnitureFlags { lit: true, locked: false, blocking: false },
///     icon: Some("🔥".to_string()),
///     tags: vec!["light".to_string(), "fire".to_string()],
///     mesh_id: None,
///     description: Some("A wall-mounted wooden torch.".to_string()),
/// };
///
/// assert_eq!(def.id, 7);
/// assert!(def.flags.lit);
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FurnitureDefinition {
    /// Unique furniture definition identifier within a campaign
    pub id: FurnitureId,

    /// Human-readable display name (e.g. "Iron-Bound Dungeon Door", "Royal Throne")
    pub name: String,

    /// Furniture category used for palette filtering in the map editor
    pub category: FurnitureCategory,

    /// Procedural mesh template to use when `mesh_id` is `None`
    pub base_type: FurnitureType,

    /// Default material applied when spawning this furniture
    pub material: FurnitureMaterial,

    /// Default scale multiplier (typically 0.5–2.0)
    pub scale: f32,

    /// Optional default color tint override (RGB, 0.0–1.0 per channel)
    ///
    /// When `None`, the material's own `base_color()` is used.
    #[serde(default)]
    pub color_tint: Option<[f32; 3]>,

    /// Default behaviour flags (lit, locked, blocking)
    #[serde(default)]
    pub flags: FurnitureFlags,

    /// Optional emoji icon override shown in the map editor palette
    ///
    /// Falls back to `FurnitureType::icon()` when `None`.
    #[serde(default)]
    pub icon: Option<String>,

    /// Free-form tags for editor filtering (e.g. `["dungeon", "boss"]`)
    #[serde(default)]
    pub tags: Vec<String>,

    /// Optional custom mesh from `furniture_mesh_registry.ron`
    ///
    /// When `Some(id)`, the rendering system loads the RON mesh asset
    /// referenced by `FurnitureMeshDatabase` instead of calling the
    /// procedural `spawn_*` function.
    #[serde(default)]
    pub mesh_id: Option<FurnitureMeshId>,

    /// Optional flavor text displayed in the editor and any in-game inspect UI
    #[serde(default)]
    pub description: Option<String>,
}

impl FurnitureDefinition {
    /// Returns the display icon for this definition
    ///
    /// Uses the explicit `icon` override when set; otherwise falls back to
    /// the emoji defined by `FurnitureType::icon()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::FurnitureDefinition;
    /// use antares::domain::world::{FurnitureCategory, FurnitureMaterial, FurnitureType, FurnitureFlags};
    ///
    /// let def = FurnitureDefinition {
    ///     id: 1,
    ///     name: "Throne".to_string(),
    ///     category: FurnitureCategory::Seating,
    ///     base_type: FurnitureType::Throne,
    ///     material: FurnitureMaterial::Wood,
    ///     scale: 1.0,
    ///     color_tint: None,
    ///     flags: FurnitureFlags::default(),
    ///     icon: Some("🪑".to_string()),
    ///     tags: vec![],
    ///     mesh_id: None,
    ///     description: None,
    /// };
    ///
    /// assert_eq!(def.display_icon(), "🪑");
    /// ```
    pub fn display_icon(&self) -> &str {
        self.icon
            .as_deref()
            .unwrap_or_else(|| self.base_type.icon())
    }

    /// Returns `true` if this definition uses a custom OBJ-imported mesh
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::FurnitureDefinition;
    /// use antares::domain::world::{FurnitureCategory, FurnitureMaterial, FurnitureType, FurnitureFlags};
    ///
    /// let def = FurnitureDefinition {
    ///     id: 1,
    ///     name: "Custom Table".to_string(),
    ///     category: FurnitureCategory::Utility,
    ///     base_type: FurnitureType::Table,
    ///     material: FurnitureMaterial::Wood,
    ///     scale: 1.0,
    ///     color_tint: None,
    ///     flags: FurnitureFlags::default(),
    ///     icon: None,
    ///     tags: vec![],
    ///     mesh_id: Some(10001),
    ///     description: None,
    /// };
    ///
    /// assert!(def.has_custom_mesh());
    /// ```
    pub fn has_custom_mesh(&self) -> bool {
        self.mesh_id.is_some()
    }
}

// ===== FurnitureDatabase =====

/// In-memory index of all [`FurnitureDefinition`] entries for a campaign
///
/// Loaded from `data/furniture.ron` by the campaign loader.  Provides
/// O(1) lookup by ID and linear scans by category, base type, or name.
///
/// # Examples
///
/// ```
/// use antares::domain::world::furniture::{FurnitureDatabase, FurnitureDefinition};
/// use antares::domain::world::{FurnitureCategory, FurnitureMaterial, FurnitureType, FurnitureFlags};
///
/// let mut db = FurnitureDatabase::new();
///
/// let def = FurnitureDefinition {
///     id: 1,
///     name: "Bench".to_string(),
///     category: FurnitureCategory::Seating,
///     base_type: FurnitureType::Bench,
///     material: FurnitureMaterial::Wood,
///     scale: 1.0,
///     color_tint: None,
///     flags: FurnitureFlags::default(),
///     icon: None,
///     tags: vec![],
///     mesh_id: None,
///     description: None,
/// };
///
/// db.add(def).unwrap();
/// assert_eq!(db.len(), 1);
/// ```
#[derive(Debug, Clone, Default)]
pub struct FurnitureDatabase {
    /// Definitions indexed by `FurnitureId` for O(1) lookup
    items: HashMap<FurnitureId, FurnitureDefinition>,
}

crate::impl_ron_database!(
    FurnitureDatabase,
    entity: FurnitureDefinition,
    key: FurnitureId,
    error: FurnitureDatabaseError,
    field: items,
    id_of: |d: &FurnitureDefinition| d.id,
    dup_err: FurnitureDatabaseError::DuplicateId,
    read_err: FurnitureDatabaseError::ReadError,
    parse_err: FurnitureDatabaseError::ParseError,
);

/// Database of custom furniture meshes loaded from `furniture_mesh_registry.ron`
///
/// Furniture mesh assets reuse the same underlying [`crate::domain::visual::CreatureDefinition`]
/// format as creature and item mesh assets. This wrapper keeps furniture mesh
/// concerns separate from creature visuals while reusing the validated registry loader.
///
/// # Examples
///
/// ```no_run
/// use antares::domain::world::furniture::FurnitureMeshDatabase;
/// use std::path::Path;
///
/// let db = FurnitureMeshDatabase::load_from_registry(
///     Path::new("data/test_campaign/data/furniture_mesh_registry.ron"),
///     Path::new("data/test_campaign"),
/// );
///
/// // Missing fixture data in some environments is fine; this is just a usage example.
/// let _ = db;
/// ```
#[derive(Debug, Clone, Default)]
pub struct FurnitureMeshDatabase {
    /// Wrapped creature-style registry database for furniture mesh assets
    inner: CreatureDatabase,
}

impl FurnitureMeshDatabase {
    /// Creates a new, empty furniture mesh database.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::FurnitureMeshDatabase;
    ///
    /// let db = FurnitureMeshDatabase::new();
    /// assert!(db.is_empty());
    /// assert_eq!(db.count(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            inner: CreatureDatabase::new(),
        }
    }

    /// Loads a furniture mesh database from a registry RON file.
    ///
    /// The registry uses the same `CreatureReference` format as creature and
    /// item mesh registries. Each entry points at a `CreatureDefinition` RON
    /// file under the campaign root.
    ///
    /// # Arguments
    ///
    /// * `registry_path` - Path to `furniture_mesh_registry.ron`
    /// * `campaign_root` - Root campaign directory used to resolve relative asset paths
    ///
    /// # Errors
    ///
    /// Returns an error if the registry file or any referenced mesh asset file
    /// cannot be read, parsed, or validated.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::domain::world::furniture::FurnitureMeshDatabase;
    /// use std::path::Path;
    ///
    /// let db = FurnitureMeshDatabase::load_from_registry(
    ///     Path::new("data/test_campaign/data/furniture_mesh_registry.ron"),
    ///     Path::new("data/test_campaign"),
    /// ).unwrap();
    /// assert!(!db.is_empty());
    /// ```
    pub fn load_from_registry(
        registry_path: &Path,
        campaign_root: &Path,
    ) -> Result<Self, CreatureDatabaseError> {
        let inner = CreatureDatabase::load_from_registry(registry_path, campaign_root)?;
        Ok(Self { inner })
    }

    /// Returns the underlying creature-style mesh database.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::FurnitureMeshDatabase;
    ///
    /// let db = FurnitureMeshDatabase::new();
    /// assert!(db.as_creature_database().is_empty());
    /// ```
    pub fn as_creature_database(&self) -> &CreatureDatabase {
        &self.inner
    }

    /// Returns `true` when no furniture mesh entries are loaded.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::FurnitureMeshDatabase;
    ///
    /// let db = FurnitureMeshDatabase::new();
    /// assert!(db.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the number of registered furniture mesh entries.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::FurnitureMeshDatabase;
    ///
    /// let db = FurnitureMeshDatabase::new();
    /// assert_eq!(db.count(), 0);
    /// ```
    pub fn count(&self) -> usize {
        self.inner.count()
    }

    /// Returns `true` if a mesh with the given furniture mesh ID exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::FurnitureMeshDatabase;
    ///
    /// let db = FurnitureMeshDatabase::new();
    /// assert!(!db.has_mesh(10001));
    /// ```
    pub fn has_mesh(&self, id: FurnitureMeshId) -> bool {
        self.inner.has_creature(id)
    }

    /// Validates all registered furniture mesh assets.
    ///
    /// # Errors
    ///
    /// Returns a validation error if any referenced mesh asset is malformed.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::FurnitureMeshDatabase;
    ///
    /// let db = FurnitureMeshDatabase::new();
    /// assert!(db.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), CreatureDatabaseError> {
        self.inner.validate()
    }
}

impl FurnitureDatabase {
    /// Creates an empty furniture database
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::FurnitureDatabase;
    ///
    /// let db = FurnitureDatabase::new();
    /// assert!(db.is_empty());
    /// assert_eq!(db.len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    /// Adds a definition to the database
    ///
    /// # Errors
    ///
    /// Returns [`FurnitureDatabaseError::DuplicateId`] if a definition with
    /// the same `id` already exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::{FurnitureDatabase, FurnitureDefinition};
    /// use antares::domain::world::{FurnitureCategory, FurnitureMaterial, FurnitureType, FurnitureFlags};
    ///
    /// let mut db = FurnitureDatabase::new();
    ///
    /// let def = FurnitureDefinition {
    ///     id: 1,
    ///     name: "Chair".to_string(),
    ///     category: FurnitureCategory::Seating,
    ///     base_type: FurnitureType::Chair,
    ///     material: FurnitureMaterial::Wood,
    ///     scale: 1.0,
    ///     color_tint: None,
    ///     flags: FurnitureFlags::default(),
    ///     icon: None,
    ///     tags: vec![],
    ///     mesh_id: None,
    ///     description: None,
    /// };
    ///
    /// assert!(db.add(def.clone()).is_ok());
    /// assert!(db.add(def).is_err()); // duplicate
    /// ```
    pub fn add(&mut self, def: FurnitureDefinition) -> Result<(), FurnitureDatabaseError> {
        if self.items.contains_key(&def.id) {
            return Err(FurnitureDatabaseError::DuplicateId(def.id));
        }
        self.items.insert(def.id, def);
        Ok(())
    }

    /// Returns a reference to the definition with the given ID, or `None`
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::{FurnitureDatabase, FurnitureDefinition};
    /// use antares::domain::world::{FurnitureCategory, FurnitureMaterial, FurnitureType, FurnitureFlags};
    ///
    /// let mut db = FurnitureDatabase::new();
    /// let def = FurnitureDefinition {
    ///     id: 42,
    ///     name: "Mystery Chair".to_string(),
    ///     category: FurnitureCategory::Seating,
    ///     base_type: FurnitureType::Chair,
    ///     material: FurnitureMaterial::Stone,
    ///     scale: 1.0,
    ///     color_tint: None,
    ///     flags: FurnitureFlags::default(),
    ///     icon: None,
    ///     tags: vec![],
    ///     mesh_id: None,
    ///     description: None,
    /// };
    /// db.add(def).unwrap();
    ///
    /// assert!(db.get_by_id(42).is_some());
    /// assert!(db.get_by_id(99).is_none());
    /// ```
    pub fn get_by_id(&self, id: FurnitureId) -> Option<&FurnitureDefinition> {
        self.items.get(&id)
    }

    /// Returns the first definition whose `name` field matches (case-sensitive),
    /// or `None`
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::{FurnitureDatabase, FurnitureDefinition};
    /// use antares::domain::world::{FurnitureCategory, FurnitureMaterial, FurnitureType, FurnitureFlags};
    ///
    /// let mut db = FurnitureDatabase::new();
    /// let def = FurnitureDefinition {
    ///     id: 5,
    ///     name: "Golden Throne".to_string(),
    ///     category: FurnitureCategory::Seating,
    ///     base_type: FurnitureType::Throne,
    ///     material: FurnitureMaterial::Gold,
    ///     scale: 1.5,
    ///     color_tint: None,
    ///     flags: FurnitureFlags::default(),
    ///     icon: None,
    ///     tags: vec![],
    ///     mesh_id: None,
    ///     description: None,
    /// };
    /// db.add(def).unwrap();
    ///
    /// assert!(db.get_by_name("Golden Throne").is_some());
    /// assert!(db.get_by_name("golden throne").is_none()); // case-sensitive
    /// ```
    pub fn get_by_name(&self, name: &str) -> Option<&FurnitureDefinition> {
        self.items.values().find(|d| d.name == name)
    }

    /// Returns all definitions belonging to the specified category
    ///
    /// The returned slice is not guaranteed to be in any particular order.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::{FurnitureDatabase, FurnitureDefinition};
    /// use antares::domain::world::{FurnitureCategory, FurnitureMaterial, FurnitureType, FurnitureFlags};
    ///
    /// let mut db = FurnitureDatabase::new();
    /// for id in 1u32..=3 {
    ///     db.add(FurnitureDefinition {
    ///         id,
    ///         name: format!("Chair {}", id),
    ///         category: FurnitureCategory::Seating,
    ///         base_type: FurnitureType::Chair,
    ///         material: FurnitureMaterial::Wood,
    ///         scale: 1.0,
    ///         color_tint: None,
    ///         flags: FurnitureFlags::default(),
    ///         icon: None,
    ///         tags: vec![],
    ///         mesh_id: None,
    ///         description: None,
    ///     }).unwrap();
    /// }
    ///
    /// let seating = db.get_by_category(FurnitureCategory::Seating);
    /// assert_eq!(seating.len(), 3);
    ///
    /// let storage = db.get_by_category(FurnitureCategory::Storage);
    /// assert_eq!(storage.len(), 0);
    /// ```
    pub fn get_by_category(&self, cat: FurnitureCategory) -> Vec<&FurnitureDefinition> {
        self.items.values().filter(|d| d.category == cat).collect()
    }

    /// Returns all definitions that use the specified procedural mesh base type
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::{FurnitureDatabase, FurnitureDefinition};
    /// use antares::domain::world::{FurnitureCategory, FurnitureMaterial, FurnitureType, FurnitureFlags};
    ///
    /// let mut db = FurnitureDatabase::new();
    /// db.add(FurnitureDefinition {
    ///     id: 1,
    ///     name: "Wooden Throne".to_string(),
    ///     category: FurnitureCategory::Seating,
    ///     base_type: FurnitureType::Throne,
    ///     material: FurnitureMaterial::Wood,
    ///     scale: 1.2,
    ///     color_tint: None,
    ///     flags: FurnitureFlags::default(),
    ///     icon: None,
    ///     tags: vec![],
    ///     mesh_id: None,
    ///     description: None,
    /// }).unwrap();
    /// db.add(FurnitureDefinition {
    ///     id: 2,
    ///     name: "Golden Throne".to_string(),
    ///     category: FurnitureCategory::Seating,
    ///     base_type: FurnitureType::Throne,
    ///     material: FurnitureMaterial::Gold,
    ///     scale: 1.5,
    ///     color_tint: None,
    ///     flags: FurnitureFlags::default(),
    ///     icon: None,
    ///     tags: vec![],
    ///     mesh_id: None,
    ///     description: None,
    /// }).unwrap();
    ///
    /// assert_eq!(db.get_by_base_type(FurnitureType::Throne).len(), 2);
    /// assert_eq!(db.get_by_base_type(FurnitureType::Bench).len(), 0);
    /// ```
    pub fn get_by_base_type(&self, t: FurnitureType) -> Vec<&FurnitureDefinition> {
        self.items.values().filter(|d| d.base_type == t).collect()
    }

    /// Returns all definitions in the database
    ///
    /// The returned Vec is not guaranteed to be in any particular order.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::FurnitureDatabase;
    ///
    /// let db = FurnitureDatabase::new();
    /// assert!(db.all_definitions().is_empty());
    /// ```
    pub fn all_definitions(&self) -> Vec<&FurnitureDefinition> {
        self.items.values().collect()
    }

    /// Returns the number of definitions in the database
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::FurnitureDatabase;
    ///
    /// let db = FurnitureDatabase::new();
    /// assert_eq!(db.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the database contains no definitions
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::FurnitureDatabase;
    ///
    /// let db = FurnitureDatabase::new();
    /// assert!(db.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns `true` if a definition with the given ID exists
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::furniture::FurnitureDatabase;
    ///
    /// let db = FurnitureDatabase::new();
    /// assert!(!db.has_definition(1));
    /// ```
    pub fn has_definition(&self, id: FurnitureId) -> bool {
        self.items.contains_key(&id)
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;

    fn make_def(id: FurnitureId, name: &str) -> FurnitureDefinition {
        FurnitureDefinition {
            id,
            name: name.to_string(),
            category: FurnitureCategory::Seating,
            base_type: FurnitureType::Chair,
            material: FurnitureMaterial::Wood,
            scale: 1.0,
            color_tint: None,
            flags: FurnitureFlags::default(),
            icon: None,
            tags: vec![],
            mesh_id: None,
            description: None,
        }
    }

    // ----- FurnitureDefinition tests -----

    #[test]
    fn test_furniture_definition_display_icon_override() {
        let def = FurnitureDefinition {
            icon: Some("🏛️".to_string()),
            ..make_def(1, "Grand Chair")
        };
        assert_eq!(def.display_icon(), "🏛️");
    }

    #[test]
    fn test_furniture_definition_display_icon_fallback() {
        let def = FurnitureDefinition {
            icon: None,
            base_type: FurnitureType::Torch,
            ..make_def(1, "Torch")
        };
        // Fallback must match FurnitureType::Torch.icon()
        assert_eq!(def.display_icon(), FurnitureType::Torch.icon());
    }

    #[test]
    fn test_furniture_definition_has_custom_mesh_true() {
        let def = FurnitureDefinition {
            mesh_id: Some(10001),
            ..make_def(1, "Custom Table")
        };
        assert!(def.has_custom_mesh());
    }

    #[test]
    fn test_furniture_definition_has_custom_mesh_false() {
        let def = make_def(1, "Plain Chair");
        assert!(!def.has_custom_mesh());
    }

    #[test]
    fn test_furniture_definition_ron_roundtrip() {
        let original = FurnitureDefinition {
            id: 7,
            name: "Wooden Torch".to_string(),
            category: FurnitureCategory::Lighting,
            base_type: FurnitureType::Torch,
            material: FurnitureMaterial::Wood,
            scale: 1.0,
            color_tint: Some([1.0, 0.6, 0.2]),
            flags: FurnitureFlags {
                lit: true,
                locked: false,
                blocking: false,
            },
            icon: None,
            tags: vec!["fire".to_string(), "light".to_string()],
            mesh_id: None,
            description: Some("A wall-mounted wooden torch.".to_string()),
        };

        let serialized = ron::to_string(&original).expect("serialization failed");
        let deserialized: FurnitureDefinition =
            ron::from_str(&serialized).expect("deserialization failed");

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_furniture_definition_ron_roundtrip_with_mesh_id() {
        let original = FurnitureDefinition {
            id: 42,
            name: "Custom Oak Table".to_string(),
            category: FurnitureCategory::Utility,
            base_type: FurnitureType::Table,
            material: FurnitureMaterial::Wood,
            scale: 1.1,
            color_tint: None,
            flags: FurnitureFlags {
                lit: false,
                locked: false,
                blocking: true,
            },
            icon: Some("🪵".to_string()),
            tags: vec!["custom".to_string()],
            mesh_id: Some(10001),
            description: None,
        };

        let serialized = ron::to_string(&original).expect("serialization failed");
        let deserialized: FurnitureDefinition =
            ron::from_str(&serialized).expect("deserialization failed");

        assert_eq!(original, deserialized);
        assert_eq!(deserialized.mesh_id, Some(10001));
    }

    // ----- FurnitureDatabase construction tests -----

    #[test]
    fn test_new_database_is_empty() {
        let db = FurnitureDatabase::new();
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);
    }

    #[test]
    fn test_default_database_is_empty() {
        let db = FurnitureDatabase::default();
        assert!(db.is_empty());
    }

    // ----- add() tests -----

    #[test]
    fn test_add_single_definition() {
        let mut db = FurnitureDatabase::new();
        let def = make_def(1, "Chair");
        assert!(db.add(def).is_ok());
        assert_eq!(db.len(), 1);
        assert!(!db.is_empty());
    }

    #[test]
    fn test_add_multiple_definitions() {
        let mut db = FurnitureDatabase::new();
        for i in 1u32..=5 {
            assert!(db.add(make_def(i, &format!("Furniture {}", i))).is_ok());
        }
        assert_eq!(db.len(), 5);
    }

    #[test]
    fn test_add_duplicate_id_returns_error() {
        let mut db = FurnitureDatabase::new();
        db.add(make_def(1, "Chair A")).unwrap();

        let result = db.add(make_def(1, "Chair B"));
        assert!(result.is_err());

        match result.unwrap_err() {
            FurnitureDatabaseError::DuplicateId(id) => assert_eq!(id, 1),
            e => panic!("Expected DuplicateId, got {:?}", e),
        }
    }

    // ----- get_by_id() tests -----

    #[test]
    fn test_get_by_id_found() {
        let mut db = FurnitureDatabase::new();
        db.add(make_def(10, "Throne")).unwrap();

        let result = db.get_by_id(10);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "Throne");
    }

    #[test]
    fn test_get_by_id_not_found() {
        let db = FurnitureDatabase::new();
        assert!(db.get_by_id(999).is_none());
    }

    // ----- get_by_name() tests -----

    #[test]
    fn test_get_by_name_found() {
        let mut db = FurnitureDatabase::new();
        db.add(make_def(1, "Golden Throne")).unwrap();

        assert!(db.get_by_name("Golden Throne").is_some());
    }

    #[test]
    fn test_get_by_name_not_found() {
        let mut db = FurnitureDatabase::new();
        db.add(make_def(1, "Throne")).unwrap();

        assert!(db.get_by_name("throne").is_none()); // case-sensitive
        assert!(db.get_by_name("Bench").is_none());
    }

    // ----- get_by_category() tests -----

    #[test]
    fn test_get_by_category_returns_matching() {
        let mut db = FurnitureDatabase::new();
        db.add(FurnitureDefinition {
            id: 1,
            category: FurnitureCategory::Seating,
            ..make_def(1, "Chair")
        })
        .unwrap();
        db.add(FurnitureDefinition {
            id: 2,
            category: FurnitureCategory::Storage,
            base_type: FurnitureType::Chest,
            ..make_def(2, "Chest")
        })
        .unwrap();
        db.add(FurnitureDefinition {
            id: 3,
            category: FurnitureCategory::Seating,
            ..make_def(3, "Bench")
        })
        .unwrap();

        let seating = db.get_by_category(FurnitureCategory::Seating);
        assert_eq!(seating.len(), 2);

        let storage = db.get_by_category(FurnitureCategory::Storage);
        assert_eq!(storage.len(), 1);

        let lighting = db.get_by_category(FurnitureCategory::Lighting);
        assert!(lighting.is_empty());
    }

    #[test]
    fn test_get_by_category_passage() {
        let mut db = FurnitureDatabase::new();
        db.add(FurnitureDefinition {
            id: 1,
            name: "Stone Arch".to_string(),
            category: FurnitureCategory::Passage,
            base_type: FurnitureType::Bench, // placeholder base
            material: FurnitureMaterial::Stone,
            scale: 1.5,
            color_tint: None,
            flags: FurnitureFlags::default(),
            icon: Some("🚪".to_string()),
            tags: vec!["passage".to_string()],
            mesh_id: None,
            description: Some("A stone archway.".to_string()),
        })
        .unwrap();

        let passages = db.get_by_category(FurnitureCategory::Passage);
        assert_eq!(passages.len(), 1);
        assert_eq!(passages[0].name, "Stone Arch");
    }

    // ----- get_by_base_type() tests -----

    #[test]
    fn test_get_by_base_type_returns_matching() {
        let mut db = FurnitureDatabase::new();
        db.add(FurnitureDefinition {
            id: 1,
            base_type: FurnitureType::Throne,
            ..make_def(1, "Wooden Throne")
        })
        .unwrap();
        db.add(FurnitureDefinition {
            id: 2,
            base_type: FurnitureType::Throne,
            material: FurnitureMaterial::Gold,
            ..make_def(2, "Golden Throne")
        })
        .unwrap();
        db.add(FurnitureDefinition {
            id: 3,
            base_type: FurnitureType::Bench,
            ..make_def(3, "Bench")
        })
        .unwrap();

        assert_eq!(db.get_by_base_type(FurnitureType::Throne).len(), 2);
        assert_eq!(db.get_by_base_type(FurnitureType::Bench).len(), 1);
        assert_eq!(db.get_by_base_type(FurnitureType::Table).len(), 0);
    }

    // ----- all_definitions() tests -----

    #[test]
    fn test_all_definitions_returns_all() {
        let mut db = FurnitureDatabase::new();
        for i in 1u32..=3 {
            db.add(make_def(i, &format!("Item {}", i))).unwrap();
        }

        let all = db.all_definitions();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_all_definitions_empty() {
        let db = FurnitureDatabase::new();
        assert!(db.all_definitions().is_empty());
    }

    // ----- has_definition() tests -----

    #[test]
    fn test_has_definition_true() {
        let mut db = FurnitureDatabase::new();
        db.add(make_def(7, "Torch")).unwrap();
        assert!(db.has_definition(7));
    }

    #[test]
    fn test_has_definition_false() {
        let db = FurnitureDatabase::new();
        assert!(!db.has_definition(7));
    }

    // ----- load_from_string() tests -----

    #[test]
    fn test_load_from_ron_string_single_entry() {
        let ron_data = r#"[
            (
                id: 1,
                name: "Wooden Bench",
                category: Seating,
                base_type: Bench,
                material: Wood,
                scale: 1.0,
                description: Some("A simple bench."),
            ),
        ]"#;

        let db = FurnitureDatabase::load_from_string(ron_data).expect("load failed");
        assert_eq!(db.len(), 1);

        let def = db.get_by_id(1).expect("id 1 not found");
        assert_eq!(def.name, "Wooden Bench");
        assert_eq!(def.category, FurnitureCategory::Seating);
        assert_eq!(def.base_type, FurnitureType::Bench);
        assert_eq!(def.material, FurnitureMaterial::Wood);
        assert!((def.scale - 1.0).abs() < f32::EPSILON);
        assert_eq!(def.description, Some("A simple bench.".to_string()));
    }

    #[test]
    fn test_load_from_ron_string_multiple_entries() {
        let ron_data = r#"[
            (
                id: 1,
                name: "Chair",
                category: Seating,
                base_type: Chair,
                material: Wood,
                scale: 1.0,
            ),
            (
                id: 2,
                name: "Torch",
                category: Lighting,
                base_type: Torch,
                material: Wood,
                scale: 1.0,
                color_tint: Some((1.0, 0.6, 0.2)),
                flags: (lit: true, locked: false, blocking: false),
            ),
        ]"#;

        let db = FurnitureDatabase::load_from_string(ron_data).expect("load failed");
        assert_eq!(db.len(), 2);

        let torch = db.get_by_id(2).expect("torch not found");
        assert_eq!(torch.color_tint, Some([1.0, 0.6, 0.2_f32]));
        assert!(torch.flags.lit);
    }

    #[test]
    fn test_load_from_ron_string_duplicate_id_fails() {
        let ron_data = r#"[
            (
                id: 1,
                name: "Chair A",
                category: Seating,
                base_type: Chair,
                material: Wood,
                scale: 1.0,
            ),
            (
                id: 1,
                name: "Chair B",
                category: Seating,
                base_type: Chair,
                material: Stone,
                scale: 1.0,
            ),
        ]"#;

        let result = FurnitureDatabase::load_from_string(ron_data);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FurnitureDatabaseError::DuplicateId(1)
        ));
    }

    #[test]
    fn test_load_from_ron_string_invalid_ron_fails() {
        let ron_data = "this is not valid ron [[[";
        let result = FurnitureDatabase::load_from_string(ron_data);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FurnitureDatabaseError::ParseError(_)
        ));
    }

    #[test]
    fn test_load_from_ron_string_empty_list() {
        let ron_data = "[]";
        let db = FurnitureDatabase::load_from_string(ron_data).expect("load failed");
        assert!(db.is_empty());
    }

    // ----- load_from_file() (test_campaign fixture) tests -----

    #[test]
    fn test_load_from_file_test_campaign() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path =
            std::path::PathBuf::from(manifest_dir).join("data/test_campaign/data/furniture.ron");

        let db = FurnitureDatabase::load_from_file(&path)
            .expect("failed to load data/test_campaign/data/furniture.ron");

        // At least the 11 seed entries from FurnitureType::default_presets()
        assert!(
            db.len() >= 11,
            "Expected at least 11 furniture definitions, got {}",
            db.len()
        );

        // Must have entries for every FurnitureType
        for ft in FurnitureType::all() {
            assert!(
                !db.get_by_base_type(*ft).is_empty(),
                "Expected at least one definition for base_type {:?}",
                ft
            );
        }
    }

    #[test]
    fn test_load_from_file_missing_is_error() {
        let result = FurnitureDatabase::load_from_file("nonexistent/path/furniture.ron");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FurnitureDatabaseError::ReadError(_)
        ));
    }

    // ----- FurnitureId type alias tests -----

    #[test]
    fn test_furniture_id_type_alias_works() {
        let id: FurnitureId = 42;
        let mut db = FurnitureDatabase::new();
        db.add(make_def(id, "Test")).unwrap();
        assert!(db.get_by_id(id).is_some());
    }

    #[test]
    fn test_furniture_mesh_id_in_definition() {
        let mesh_id: FurnitureMeshId = 10001;
        let def = FurnitureDefinition {
            mesh_id: Some(mesh_id),
            ..make_def(1, "Custom")
        };
        assert_eq!(def.mesh_id, Some(10001));
        assert!(def.has_custom_mesh());
    }
}
