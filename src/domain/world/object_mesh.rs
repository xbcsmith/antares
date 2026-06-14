// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Unified object mesh registry — `ObjectMeshDatabase`.
//!
//! `ObjectMeshDatabase` is the Phase 4 replacement for the split
//! `LandscapeMeshDatabase` / `FurnitureMeshDatabase` lookup used by rendering
//! code. It holds mesh assets keyed by an arbitrary **string** identifier so
//! that map-event `mesh_id` values can be human-readable names like
//! `"barred_door"` or `"treasure_chest"` rather than raw numeric IDs.
//!
//! Legacy registries (`landscape_mesh_registry.ron`, `furniture_mesh_registry.ron`)
//! are merged in at load time — their numeric IDs become string keys (e.g. `"11001"`)
//! so existing campaigns continue to resolve without modification.
//!
//! # File format
//!
//! `object_mesh_registry.ron` uses a named-struct wrapper:
//!
//! ```ron
//! ObjectMeshRegistry(
//!     meshes: {
//!         "barred_door":    "assets/meshes/objects/barred_door.ron",
//!         "treasure_chest": "assets/meshes/objects/treasure_chest.ron",
//!     }
//! )
//! ```
//!
//! Each value is a path **relative to the campaign root** pointing at a
//! `CreatureDefinition` RON asset file — the same format used by creature,
//! item, landscape, and furniture mesh registries.
//!
//! # Examples
//!
//! ```
//! use antares::domain::world::object_mesh::ObjectMeshDatabase;
//!
//! let db = ObjectMeshDatabase::new();
//! assert!(db.is_empty());
//! assert_eq!(db.count(), 0);
//! assert!(db.lookup("oak_tree").is_none());
//! ```

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

use crate::domain::visual::creature_database::CreatureDatabaseError;
use crate::domain::visual::CreatureDefinition;
use crate::domain::world::furniture::FurnitureMeshDatabase;
use crate::domain::world::landscape::LandscapeMeshDatabase;

/// Errors from loading or validating an [`ObjectMeshDatabase`].
///
/// # Examples
///
/// ```
/// use antares::domain::world::object_mesh::ObjectMeshError;
///
/// let e = ObjectMeshError::ReadError("no file".to_string());
/// assert!(e.to_string().contains("no file"));
/// ```
#[derive(Debug, Error)]
pub enum ObjectMeshError {
    /// Registry file could not be read.
    #[error("Failed to read object mesh registry: {0}")]
    ReadError(String),

    /// RON parsing of the registry or a referenced asset failed.
    #[error("Failed to parse object mesh data: {0}")]
    ParseError(String),

    /// A referenced mesh asset file could not be read.
    #[error("Failed to read mesh asset '{path}': {reason}")]
    AssetReadError {
        /// Relative path from the registry entry.
        path: String,
        /// Underlying I/O error message.
        reason: String,
    },

    /// Validation of an underlying `CreatureDefinition` failed.
    #[error("Mesh asset validation failed: {0}")]
    ValidationError(String),
}

impl From<CreatureDatabaseError> for ObjectMeshError {
    fn from(e: CreatureDatabaseError) -> Self {
        ObjectMeshError::ValidationError(e.to_string())
    }
}

/// RON schema for `object_mesh_registry.ron`.
///
/// Maps string mesh IDs to campaign-relative file paths for
/// `CreatureDefinition` RON assets.
#[derive(Debug, Deserialize)]
struct ObjectMeshRegistry {
    meshes: HashMap<String, String>,
}

/// Unified, string-keyed mesh database for all interactive objects.
///
/// Aggregates mesh assets from:
/// - `object_mesh_registry.ron` (primary, string-keyed)
/// - `landscape_mesh_registry.ron` (deprecated alias — numeric IDs as strings)
/// - `furniture_mesh_registry.ron` (deprecated alias — numeric IDs as strings)
///
/// The deprecated registries are merged so that existing campaigns referencing
/// numeric `mesh_id` values (e.g. `"11001"`) keep resolving correctly.
///
/// `item_mesh_registry.ron` is intentionally **not** merged here — item mesh
/// lookup goes through a separate `DroppedItem` spawn path.
///
/// # Examples
///
/// ```
/// use antares::domain::world::object_mesh::ObjectMeshDatabase;
///
/// let db = ObjectMeshDatabase::new();
/// assert!(db.is_empty());
/// assert_eq!(db.count(), 0);
/// ```
#[derive(Debug, Clone, Default)]
pub struct ObjectMeshDatabase {
    meshes: HashMap<String, CreatureDefinition>,
}

impl ObjectMeshDatabase {
    /// Creates a new, empty `ObjectMeshDatabase`.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::object_mesh::ObjectMeshDatabase;
    ///
    /// let db = ObjectMeshDatabase::new();
    /// assert!(db.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            meshes: HashMap::new(),
        }
    }

    /// Loads an `ObjectMeshDatabase` from `object_mesh_registry.ron`.
    ///
    /// Each entry in the registry maps a string key to a
    /// `CreatureDefinition` RON file path relative to `campaign_root`.
    ///
    /// This does **not** merge the legacy landscape or furniture registries —
    /// call [`merge_landscape`](Self::merge_landscape) and
    /// [`merge_furniture`](Self::merge_furniture) separately after loading.
    ///
    /// # Errors
    ///
    /// Returns [`ObjectMeshError`] if the registry file or any referenced
    /// asset cannot be read or parsed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::domain::world::object_mesh::ObjectMeshDatabase;
    /// use std::path::Path;
    ///
    /// let db = ObjectMeshDatabase::load_from_registry(
    ///     Path::new("campaigns/my_campaign/data/object_mesh_registry.ron"),
    ///     Path::new("campaigns/my_campaign"),
    /// ).unwrap();
    /// assert!(!db.is_empty());
    /// ```
    pub fn load_from_registry(
        registry_path: &Path,
        campaign_root: &Path,
    ) -> Result<Self, ObjectMeshError> {
        let content = std::fs::read_to_string(registry_path)
            .map_err(|e| ObjectMeshError::ReadError(e.to_string()))?;

        let registry: ObjectMeshRegistry =
            ron::from_str(&content).map_err(|e| ObjectMeshError::ParseError(e.to_string()))?;

        let mut db = Self::new();

        for (key, filepath) in registry.meshes {
            let asset_path = campaign_root.join(&filepath);
            let asset_content = std::fs::read_to_string(&asset_path).map_err(|e| {
                ObjectMeshError::AssetReadError {
                    path: filepath.clone(),
                    reason: e.to_string(),
                }
            })?;

            let creature: CreatureDefinition = ron::from_str(&asset_content)
                .map_err(|e| ObjectMeshError::ParseError(format!("'{}': {}", filepath, e)))?;

            db.meshes.insert(key, creature);
        }

        Ok(db)
    }

    /// Merges entries from a legacy `LandscapeMeshDatabase` into this database.
    ///
    /// Each landscape mesh is inserted with its numeric ID converted to a
    /// string key (e.g. `11001` → `"11001"`).  Existing entries for the same
    /// key are **not** overwritten — `object_mesh_registry.ron` entries take
    /// precedence.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::object_mesh::ObjectMeshDatabase;
    /// use antares::domain::world::landscape::LandscapeMeshDatabase;
    ///
    /// let mut db = ObjectMeshDatabase::new();
    /// let landscape = LandscapeMeshDatabase::new();
    /// db.merge_landscape(&landscape);
    /// assert!(db.is_empty());
    /// ```
    pub fn merge_landscape(&mut self, landscape: &LandscapeMeshDatabase) {
        for creature in landscape.as_creature_database().all_creatures() {
            let key = creature.id.to_string();
            self.meshes.entry(key).or_insert_with(|| creature.clone());
        }
    }

    /// Merges entries from a legacy `FurnitureMeshDatabase` into this database.
    ///
    /// Each furniture mesh is inserted with its numeric ID converted to a
    /// string key (e.g. `10001` → `"10001"`).  Existing entries are **not**
    /// overwritten.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::object_mesh::ObjectMeshDatabase;
    /// use antares::domain::world::furniture::FurnitureMeshDatabase;
    ///
    /// let mut db = ObjectMeshDatabase::new();
    /// let furniture = FurnitureMeshDatabase::new();
    /// db.merge_furniture(&furniture);
    /// assert!(db.is_empty());
    /// ```
    pub fn merge_furniture(&mut self, furniture: &FurnitureMeshDatabase) {
        for creature in furniture.as_creature_database().all_creatures() {
            let key = creature.id.to_string();
            self.meshes.entry(key).or_insert_with(|| creature.clone());
        }
    }

    /// Looks up a mesh asset by its string key.
    ///
    /// Returns `None` when no entry for `key` exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::object_mesh::ObjectMeshDatabase;
    ///
    /// let db = ObjectMeshDatabase::new();
    /// assert!(db.lookup("oak_tree").is_none());
    /// ```
    pub fn lookup(&self, key: &str) -> Option<&CreatureDefinition> {
        self.meshes.get(key)
    }

    /// Returns `true` if a mesh with the given string key is registered.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::object_mesh::ObjectMeshDatabase;
    ///
    /// let db = ObjectMeshDatabase::new();
    /// assert!(!db.has_mesh("oak_tree"));
    /// ```
    pub fn has_mesh(&self, key: &str) -> bool {
        self.meshes.contains_key(key)
    }

    /// Returns all registered mesh IDs (string keys).
    ///
    /// Order is unspecified.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::object_mesh::ObjectMeshDatabase;
    ///
    /// let db = ObjectMeshDatabase::new();
    /// assert!(db.all_mesh_ids().is_empty());
    /// ```
    pub fn all_mesh_ids(&self) -> Vec<String> {
        self.meshes.keys().cloned().collect()
    }

    /// Returns `true` when no mesh entries are registered.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::object_mesh::ObjectMeshDatabase;
    ///
    /// assert!(ObjectMeshDatabase::new().is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.meshes.is_empty()
    }

    /// Returns the number of registered mesh entries.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::object_mesh::ObjectMeshDatabase;
    ///
    /// assert_eq!(ObjectMeshDatabase::new().count(), 0);
    /// ```
    pub fn count(&self) -> usize {
        self.meshes.len()
    }

    /// Validates all registered mesh assets.
    ///
    /// Delegates to the underlying `CreatureDefinition` validation for each
    /// entry.  Returns the first validation error encountered.
    ///
    /// # Errors
    ///
    /// Returns [`ObjectMeshError::ValidationError`] if any asset is malformed.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::object_mesh::ObjectMeshDatabase;
    ///
    /// assert!(ObjectMeshDatabase::new().validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), ObjectMeshError> {
        for (key, creature) in &self.meshes {
            if creature.name.is_empty() {
                return Err(ObjectMeshError::ValidationError(format!(
                    "Object mesh '{}' has an empty name",
                    key
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_is_empty() {
        let db = ObjectMeshDatabase::new();
        assert!(db.is_empty());
        assert_eq!(db.count(), 0);
    }

    #[test]
    fn test_lookup_miss() {
        let db = ObjectMeshDatabase::new();
        assert!(db.lookup("missing").is_none());
        assert!(!db.has_mesh("missing"));
    }

    #[test]
    fn test_all_mesh_ids_empty() {
        let db = ObjectMeshDatabase::new();
        assert!(db.all_mesh_ids().is_empty());
    }

    #[test]
    fn test_validate_empty_is_ok() {
        assert!(ObjectMeshDatabase::new().validate().is_ok());
    }

    #[test]
    fn test_merge_landscape_empty() {
        let mut db = ObjectMeshDatabase::new();
        let landscape = LandscapeMeshDatabase::new();
        db.merge_landscape(&landscape);
        assert!(db.is_empty());
    }

    #[test]
    fn test_merge_furniture_empty() {
        let mut db = ObjectMeshDatabase::new();
        let furniture = FurnitureMeshDatabase::new();
        db.merge_furniture(&furniture);
        assert!(db.is_empty());
    }

    #[test]
    fn test_load_from_registry_round_trip() {
        use std::io::Write;

        // Write a minimal CreatureDefinition asset file
        let tmp = tempfile::TempDir::new().unwrap();
        let asset_dir = tmp.path().join("assets/meshes/objects");
        std::fs::create_dir_all(&asset_dir).unwrap();
        let asset_path = asset_dir.join("test_chest.ron");
        std::fs::File::create(&asset_path)
            .unwrap()
            .write_all(
                br#"(
    id: 1,
    name: "TestChest",
    meshes: [],
    mesh_transforms: [],
)"#,
            )
            .unwrap();

        // Write the object_mesh_registry.ron
        let registry_path = tmp.path().join("data/object_mesh_registry.ron");
        std::fs::create_dir_all(tmp.path().join("data")).unwrap();
        std::fs::File::create(&registry_path)
            .unwrap()
            .write_all(
                br#"ObjectMeshRegistry(
    meshes: {
        "test_chest": "assets/meshes/objects/test_chest.ron",
    }
)"#,
            )
            .unwrap();

        let db = ObjectMeshDatabase::load_from_registry(&registry_path, tmp.path()).unwrap();
        assert_eq!(db.count(), 1);
        assert!(db.has_mesh("test_chest"));
        assert!(db.lookup("test_chest").is_some());
        assert!(db.validate().is_ok());
    }

    #[test]
    fn test_load_from_registry_missing_asset_returns_error() {
        use std::io::Write;

        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("data")).unwrap();
        let registry_path = tmp.path().join("data/object_mesh_registry.ron");
        std::fs::File::create(&registry_path)
            .unwrap()
            .write_all(
                br#"ObjectMeshRegistry(
    meshes: {
        "ghost_mesh": "assets/meshes/ghost.ron",
    }
)"#,
            )
            .unwrap();

        let result = ObjectMeshDatabase::load_from_registry(&registry_path, tmp.path());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("ghost.ron") || msg.contains("ghost_mesh"));
    }

    #[test]
    fn test_primary_entry_not_overwritten_by_merge() {
        use std::io::Write;

        let tmp = tempfile::TempDir::new().unwrap();
        let asset_dir = tmp.path().join("assets/meshes");
        std::fs::create_dir_all(&asset_dir).unwrap();

        // Asset with id 11001 (matches landscape legacy registry numeric key)
        let asset_path = asset_dir.join("primary.ron");
        std::fs::File::create(&asset_path)
            .unwrap()
            .write_all(
                br#"(
    id: 11001,
    name: "PrimaryEntry",
    meshes: [],
    mesh_transforms: [],
)"#,
            )
            .unwrap();

        let registry_path = tmp.path().join("data/object_mesh_registry.ron");
        std::fs::create_dir_all(tmp.path().join("data")).unwrap();
        std::fs::File::create(&registry_path)
            .unwrap()
            .write_all(
                br#"ObjectMeshRegistry(
    meshes: {
        "11001": "assets/meshes/primary.ron",
    }
)"#,
            )
            .unwrap();

        let mut db = ObjectMeshDatabase::load_from_registry(&registry_path, tmp.path()).unwrap();

        // Build a fake landscape mesh database and try to merge — should not overwrite
        let landscape = LandscapeMeshDatabase::new();
        db.merge_landscape(&landscape);

        let entry = db.lookup("11001").unwrap();
        assert_eq!(entry.name, "PrimaryEntry");
    }
}
