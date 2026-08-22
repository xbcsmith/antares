// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Unified object mesh registry — `ObjectMeshDatabase`.
//!
//! `ObjectMeshDatabase` is the replacement for the split
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
//! `object_mesh_registry.ron` uses a flat array of `ObjectMeshEntry` records:
//!
//! ```ron
//! [
//!     (
//!         id: 12001,
//!         name: "Barred Passage",
//!         filepath: "assets/meshes/objects/barred_door.ron",
//!     ),
//! ]
//! ```
//!
//! Each `filepath` is a path **relative to the campaign root** pointing at a
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

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::path_security::validate_campaign_relative_path;
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

    /// The registry file could not be written.
    #[error("Failed to write object mesh registry: {0}")]
    WriteError(String),
}

impl From<CreatureDatabaseError> for ObjectMeshError {
    fn from(e: CreatureDatabaseError) -> Self {
        ObjectMeshError::ValidationError(e.to_string())
    }
}

/// A single entry in an [`ObjectMeshRegistryFile`].
///
/// Each entry pairs a numeric `id` and a human-readable `name` with the
/// campaign-relative path to a `CreatureDefinition` RON asset file.
/// `id` is the stable key used by [`ObjectMeshDatabase`] when keying resolved
/// meshes (as `id.to_string()`), while `name` is for editor display only.
///
/// # Examples
///
/// ```
/// use antares::domain::world::object_mesh::ObjectMeshEntry;
///
/// let entry = ObjectMeshEntry {
///     id: 12001,
///     name: "Barred Door".to_string(),
///     filepath: "assets/meshes/objects/barred_door.ron".to_string(),
/// };
/// assert_eq!(entry.id, 12001);
/// assert_eq!(entry.name, "Barred Door");
/// assert_eq!(entry.filepath, "assets/meshes/objects/barred_door.ron");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectMeshEntry {
    /// Numeric identifier for this mesh, unique within a registry.
    pub id: u32,
    /// Human-readable display name for this mesh entry (editor use only).
    pub name: String,
    /// Campaign-relative path to the `CreatureDefinition` RON asset file.
    pub filepath: String,
}

/// Editable, round-trippable representation of `object_mesh_registry.ron`.
///
/// This is the **write-capable** counterpart to [`ObjectMeshDatabase`].
/// `ObjectMeshDatabase` is read-only and eagerly resolves every entry to a
/// full [`CreatureDefinition`] for runtime lookups; it has no add, rename,
/// remove, or save methods because it isn't meant for editing.
/// `ObjectMeshRegistryFile` keeps the registry in its raw entry list so
/// the Campaign Builder SDK can add, rename, and remove entries and write
/// the result back to disk without resolving (or even reading) every
/// referenced asset on every edit.
///
/// Entries are kept sorted by `id` after every [`upsert`](Self::upsert).
///
/// # Examples
///
/// ```
/// use antares::domain::world::object_mesh::ObjectMeshRegistryFile;
///
/// let mut registry = ObjectMeshRegistryFile::default();
/// registry.upsert(12001, "Barred Door", "assets/meshes/objects/barred_door.ron");
/// assert_eq!(registry.entries[0].name, "Barred Door");
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObjectMeshRegistryFile {
    /// Ordered list of mesh registry entries, sorted ascending by `id`.
    pub entries: Vec<ObjectMeshEntry>,
}

impl ObjectMeshRegistryFile {
    /// Loads an `ObjectMeshRegistryFile` from a RON file at `path`.
    ///
    /// Unlike [`ObjectMeshDatabase::load_from_registry`], this does **not**
    /// resolve referenced assets — it only parses the entry list itself.
    ///
    /// This function does **not** check whether `path` exists before
    /// reading. Callers (e.g. the Campaign Builder's `load_objects`, per
    /// `sdk/AGENTS.md` Rule 13) own the `path.exists()` guard and decide what
    /// an absent file means for their use case — that decision is kept out
    /// of this domain type on purpose.
    ///
    /// The registry file must use the array-of-entries format:
    ///
    /// ```ron
    /// [
    ///     (id: 12001, name: "Barred Passage", filepath: "assets/meshes/objects/barred_door.ron"),
    /// ]
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ObjectMeshError::ReadError`] if `path` cannot be read (this
    /// includes a non-existent file), and [`ObjectMeshError::ParseError`] if
    /// the contents are not valid RON or do not match the expected
    /// `Vec<ObjectMeshEntry>` shape.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::object_mesh::ObjectMeshRegistryFile;
    /// use std::path::Path;
    ///
    /// let result = ObjectMeshRegistryFile::load(Path::new("/nonexistent/path.ron"));
    /// assert!(result.is_err());
    /// ```
    pub fn load(path: &Path) -> Result<Self, ObjectMeshError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ObjectMeshError::ReadError(e.to_string()))?;
        let entries = ron::from_str::<Vec<ObjectMeshEntry>>(&content)
            .map_err(|e| ObjectMeshError::ParseError(e.to_string()))?;
        Ok(Self { entries })
    }

    /// Serializes this registry to RON and writes it to `path`.
    ///
    /// Creates any missing parent directories before writing.
    /// The on-disk format is a plain RON array of [`ObjectMeshEntry`] records —
    /// no named-struct wrapper.
    ///
    /// # Errors
    ///
    /// Returns [`ObjectMeshError::WriteError`] if the parent directories
    /// cannot be created, the data cannot be serialized, or the file cannot
    /// be written.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::object_mesh::ObjectMeshRegistryFile;
    ///
    /// let mut registry = ObjectMeshRegistryFile::default();
    /// registry.upsert(12001, "oak_tree", "assets/meshes/objects/oak_tree.ron");
    /// let tmp = std::env::temp_dir().join("object_mesh_registry_file_doctest_save.ron");
    /// registry.save(&tmp).unwrap();
    /// assert!(tmp.exists());
    /// std::fs::remove_file(&tmp).unwrap();
    /// ```
    pub fn save(&self, path: &Path) -> Result<(), ObjectMeshError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ObjectMeshError::WriteError(e.to_string()))?;
        }
        let content = ron::ser::to_string_pretty(&self.entries, ron::ser::PrettyConfig::new())
            .map_err(|e| ObjectMeshError::WriteError(e.to_string()))?;
        std::fs::write(path, content).map_err(|e| ObjectMeshError::WriteError(e.to_string()))
    }

    /// Inserts a new entry with `id`, `name`, and `filepath`, or replaces the
    /// `name` and `filepath` of an existing entry whose `id` matches.
    ///
    /// Entries are kept sorted by `id` after every call.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::object_mesh::ObjectMeshRegistryFile;
    ///
    /// let mut registry = ObjectMeshRegistryFile::default();
    /// registry.upsert(12001, "barred_door", "assets/meshes/objects/barred_door.ron");
    /// registry.upsert(12001, "barred_door", "assets/meshes/objects/barred_door_v2.ron");
    /// assert_eq!(registry.entries.len(), 1);
    /// assert_eq!(registry.entries[0].filepath, "assets/meshes/objects/barred_door_v2.ron");
    /// ```
    pub fn upsert(&mut self, id: u32, name: &str, filepath: &str) {
        if let Some(existing) = self.entries.iter_mut().find(|e| e.id == id) {
            existing.name = name.to_string();
            existing.filepath = filepath.to_string();
        } else {
            self.entries.push(ObjectMeshEntry {
                id,
                name: name.to_string(),
                filepath: filepath.to_string(),
            });
            self.entries.sort_by_key(|e| e.id);
        }
    }

    /// Renames an existing entry by `id`, updating its `name` field.
    ///
    /// Returns `true` if an entry with `id` was found and renamed.
    /// Returns `false`, leaving the list unchanged, if no entry with `id` exists.
    /// Does not check whether a collision with another entry's `name` would
    /// result — uniqueness validation is the caller's responsibility.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::object_mesh::ObjectMeshRegistryFile;
    ///
    /// let mut registry = ObjectMeshRegistryFile::default();
    /// registry.upsert(12001, "oak_tree", "assets/meshes/objects/oak_tree.ron");
    /// assert!(registry.rename(12001, "ancient_oak"));
    /// assert_eq!(registry.entries[0].name, "ancient_oak");
    /// assert_eq!(registry.entries[0].id, 12001);
    /// assert!(!registry.rename(99999, "whatever"));
    /// ```
    pub fn rename(&mut self, id: u32, new_name: &str) -> bool {
        match self.entries.iter_mut().find(|e| e.id == id) {
            Some(entry) => {
                entry.name = new_name.to_string();
                true
            }
            None => false,
        }
    }

    /// Removes the entry with the given `id` and returns it, or `None` if absent.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::object_mesh::ObjectMeshRegistryFile;
    ///
    /// let mut registry = ObjectMeshRegistryFile::default();
    /// registry.upsert(12001, "oak_tree", "assets/meshes/objects/oak_tree.ron");
    /// let removed = registry.remove(12001);
    /// assert!(removed.is_some());
    /// let entry = removed.unwrap();
    /// assert_eq!(entry.id, 12001);
    /// assert_eq!(entry.name, "oak_tree");
    /// assert!(registry.remove(12001).is_none());
    /// ```
    pub fn remove(&mut self, id: u32) -> Option<ObjectMeshEntry> {
        if let Some(pos) = self.entries.iter().position(|e| e.id == id) {
            Some(self.entries.remove(pos))
        } else {
            None
        }
    }
}

/// Unified, string-keyed mesh database for all interactive objects.
///
/// Aggregates mesh assets from:
/// - `object_mesh_registry.ron` (primary, keyed by `entry.id.to_string()`)
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
    /// Maps mesh ID string key to the human-readable registry name.
    ///
    /// Populated from `ObjectMeshEntry::name` during `load_from_registry`.
    /// Also populated during `merge_landscape` and `merge_furniture` using
    /// `CreatureDefinition::name` as a fallback.
    names: HashMap<String, String>,
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
            names: HashMap::new(),
        }
    }

    /// Loads an `ObjectMeshDatabase` from `object_mesh_registry.ron`.
    ///
    /// Delegates parsing to [`ObjectMeshRegistryFile::load`], which expects
    /// the array-of-entries format.
    ///
    /// Each entry is keyed by `entry.id.to_string()` (e.g. `"12001"`).
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
        let registry = ObjectMeshRegistryFile::load(registry_path)?;

        let mut db = Self::new();

        for entry in registry.entries {
            let key = entry.id.to_string();
            let filepath = &entry.filepath;

            // Reject untrusted registry paths that are empty, absolute, or
            // attempt `..` traversal (or symlink escape) out of the campaign.
            let asset_path =
                validate_campaign_relative_path(campaign_root, filepath).map_err(|e| {
                    ObjectMeshError::AssetReadError {
                        path: filepath.clone(),
                        reason: e.to_string(),
                    }
                })?;
            let asset_content = std::fs::read_to_string(&asset_path).map_err(|e| {
                ObjectMeshError::AssetReadError {
                    path: filepath.clone(),
                    reason: e.to_string(),
                }
            })?;

            let creature: CreatureDefinition = ron::from_str(&asset_content)
                .map_err(|e| ObjectMeshError::ParseError(format!("'{}': {}", filepath, e)))?;

            db.names.insert(key.clone(), entry.name.clone());
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
            self.meshes
                .entry(key.clone())
                .or_insert_with(|| creature.clone());
            self.names
                .entry(key)
                .or_insert_with(|| creature.name.clone());
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
            self.meshes
                .entry(key.clone())
                .or_insert_with(|| creature.clone());
            self.names
                .entry(key)
                .or_insert_with(|| creature.name.clone());
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

    /// Returns all registered mesh IDs paired with their human-readable names.
    ///
    /// Returns `(id_string, name)` tuples where `id_string` is the string key
    /// (e.g. `"12001"`) and `name` is the display name from the registry
    /// (e.g. `"Ironbound Treasure Chest"`).
    ///
    /// Order is unspecified; callers should sort as needed.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::object_mesh::ObjectMeshDatabase;
    ///
    /// let db = ObjectMeshDatabase::new();
    /// assert!(db.all_mesh_ids_with_names().is_empty());
    /// ```
    pub fn all_mesh_ids_with_names(&self) -> Vec<(String, String)> {
        self.names
            .iter()
            .map(|(id, name)| (id.clone(), name.clone()))
            .collect()
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

        // Write a minimal CreatureDefinition asset file.
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

        // Write the registry using the new write-capable type.
        let registry_path = tmp.path().join("data/object_mesh_registry.ron");
        std::fs::create_dir_all(tmp.path().join("data")).unwrap();
        let mut reg = ObjectMeshRegistryFile::default();
        reg.upsert(12001, "TestChest", "assets/meshes/objects/test_chest.ron");
        reg.save(&registry_path).unwrap();

        let db = ObjectMeshDatabase::load_from_registry(&registry_path, tmp.path()).unwrap();
        assert_eq!(db.count(), 1);
        assert!(db.has_mesh("12001"));
        assert!(db.lookup("12001").is_some());
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
                br#"[
    (
        id: 0,
        name: "ghost_mesh",
        filepath: "assets/meshes/ghost.ron",
    ),
]"#,
            )
            .unwrap();

        let result = ObjectMeshDatabase::load_from_registry(&registry_path, tmp.path());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("ghost.ron") || msg.contains("ghost_mesh"));
    }

    #[test]
    fn test_load_from_registry_rejects_parent_traversal_filepath() {
        use std::io::Write;

        // Registry whose asset filepath attempts `..` traversal out of the
        // campaign root must be rejected before any asset read is attempted.
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("data")).unwrap();
        let registry_path = tmp.path().join("data/object_mesh_registry.ron");
        std::fs::File::create(&registry_path)
            .unwrap()
            .write_all(
                br#"[
    (
        id: 0,
        name: "escape",
        filepath: "../../etc/passwd",
    ),
]"#,
            )
            .unwrap();

        let result = ObjectMeshDatabase::load_from_registry(&registry_path, tmp.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ObjectMeshError::AssetReadError { .. }));
        assert!(
            err.to_string().contains("../../etc/passwd"),
            "expected rejected path in error, got: {err}"
        );
    }

    #[test]
    fn test_primary_entry_not_overwritten_by_merge() {
        use std::io::Write;

        let tmp = tempfile::TempDir::new().unwrap();
        let asset_dir = tmp.path().join("assets/meshes");
        std::fs::create_dir_all(&asset_dir).unwrap();

        // Asset with name "PrimaryEntry" — should survive a landscape merge.
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
        let mut reg = ObjectMeshRegistryFile::default();
        reg.upsert(12001, "PrimaryEntry", "assets/meshes/primary.ron");
        reg.save(&registry_path).unwrap();

        let mut db = ObjectMeshDatabase::load_from_registry(&registry_path, tmp.path()).unwrap();

        // Build an empty landscape mesh database and try to merge — should not overwrite.
        let landscape = LandscapeMeshDatabase::new();
        db.merge_landscape(&landscape);

        let entry = db.lookup("12001").unwrap();
        assert_eq!(entry.name, "PrimaryEntry");
    }

    #[test]
    fn test_object_mesh_registry_file_round_trip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let registry_path = tmp.path().join("data/object_mesh_registry.ron");

        let mut registry = ObjectMeshRegistryFile::default();
        registry.upsert(
            12001,
            "barred_door",
            "assets/meshes/objects/barred_door.ron",
        );
        registry.upsert(12002, "oak_tree", "assets/meshes/objects/oak_tree.ron");
        registry.save(&registry_path).unwrap();

        let loaded = ObjectMeshRegistryFile::load(&registry_path).unwrap();
        assert_eq!(loaded.entries.len(), 2);

        let e1 = loaded.entries.iter().find(|e| e.id == 12001).unwrap();
        assert_eq!(e1.name, "barred_door");
        assert_eq!(e1.filepath, "assets/meshes/objects/barred_door.ron");

        let e2 = loaded.entries.iter().find(|e| e.id == 12002).unwrap();
        assert_eq!(e2.name, "oak_tree");
        assert_eq!(e2.filepath, "assets/meshes/objects/oak_tree.ron");
    }

    #[test]
    fn test_object_mesh_registry_file_rename() {
        let mut registry = ObjectMeshRegistryFile::default();
        registry.upsert(12001, "oak_tree", "assets/meshes/objects/oak_tree.ron");

        assert!(registry.rename(12001, "ancient_oak"));
        assert_eq!(registry.entries[0].name, "ancient_oak");
        assert_eq!(registry.entries[0].id, 12001);

        let before_entries: Vec<ObjectMeshEntry> = registry.entries.clone();
        assert!(!registry.rename(99999, "whatever"));
        assert_eq!(registry.entries.len(), before_entries.len());
        assert_eq!(registry.entries, before_entries);
    }

    #[test]
    fn test_object_mesh_registry_file_remove() {
        let mut registry = ObjectMeshRegistryFile::default();
        registry.upsert(12001, "oak_tree", "assets/meshes/objects/oak_tree.ron");

        let removed = registry.remove(12001);
        assert!(removed.is_some());
        let entry = removed.unwrap();
        assert_eq!(entry.id, 12001);
        assert_eq!(entry.name, "oak_tree");

        assert!(registry.remove(12001).is_none());
        assert!(registry.remove(99999).is_none());
    }

    #[test]
    fn test_load_legacy_format_returns_error_after_removal() {
        use std::io::Write;

        let tmp = tempfile::TempDir::new().unwrap();
        let registry_path = tmp.path().join("object_mesh_registry.ron");
        std::fs::File::create(&registry_path)
            .unwrap()
            .write_all(
                br#"ObjectMeshRegistry(
    meshes: {
        "barred_passage": "assets/meshes/objects/barred_door.ron",
    }
)"#,
            )
            .unwrap();

        let result = ObjectMeshRegistryFile::load(&registry_path);
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ObjectMeshError::ParseError(_)),
            "expected ParseError for legacy named-struct format"
        );
    }

    #[test]
    fn test_object_mesh_registry_file_load_test_campaign_fixture() {
        let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data/test_campaign/data/object_mesh_registry.ron");
        let loaded = ObjectMeshRegistryFile::load(&fixture_path).unwrap();
        assert_eq!(
            loaded.entries.len(),
            6,
            "expected 6 entries in test campaign fixture"
        );
        assert_eq!(
            loaded.entries[0].id, 12001,
            "first entry id should be 12001"
        );
    }

    #[test]
    fn test_campaign_loader_object_meshes_keyed_by_numeric_id() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let registry_path = manifest_dir.join("data/test_campaign/data/object_mesh_registry.ron");
        let campaign_root = manifest_dir.join("data/test_campaign");

        let db = ObjectMeshDatabase::load_from_registry(&registry_path, &campaign_root)
            .expect("test campaign object mesh registry must load");

        assert!(
            db.has_mesh("12001"),
            "numeric id key '12001' must be present"
        );
        assert!(
            !db.has_mesh("oak_tree"),
            "legacy string key 'oak_tree' must not be present"
        );
    }

    #[test]
    fn test_object_mesh_registry_file_interop_with_object_mesh_database() {
        use std::io::Write;

        let tmp = tempfile::TempDir::new().unwrap();

        // Write a minimal CreatureDefinition asset file.
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

        // Write the registry using the new write-capable type.
        let registry_path = tmp.path().join("data/object_mesh_registry.ron");
        let mut registry = ObjectMeshRegistryFile::default();
        registry.upsert(12001, "test_chest", "assets/meshes/objects/test_chest.ron");
        registry.save(&registry_path).unwrap();

        // Load it back with the read-only loader used by the game runtime.
        let db = ObjectMeshDatabase::load_from_registry(&registry_path, tmp.path()).unwrap();
        assert!(db.has_mesh("12001"));
        assert_eq!(db.lookup("12001").unwrap().name, "TestChest");
    }

    #[test]
    fn test_object_mesh_registry_file_load_nonexistent_path_errors() {
        let result =
            ObjectMeshRegistryFile::load(Path::new("/nonexistent/object_mesh_registry.ron"));
        assert!(result.is_err());
    }

    #[test]
    fn test_object_mesh_registry_file_load_garbage_contents_errors() {
        use std::io::Write;

        let tmp = tempfile::TempDir::new().unwrap();
        let registry_path = tmp.path().join("object_mesh_registry.ron");
        std::fs::File::create(&registry_path)
            .unwrap()
            .write_all(b"not valid ron {{{")
            .unwrap();

        let result = ObjectMeshRegistryFile::load(&registry_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_all_mesh_ids_with_names_round_trip() {
        use std::io::Write;

        let tmp = tempfile::TempDir::new().unwrap();
        let asset_dir = tmp.path().join("assets/meshes/objects");
        std::fs::create_dir_all(&asset_dir).unwrap();
        std::fs::File::create(asset_dir.join("chest.ron"))
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

        let registry_path = tmp.path().join("data/object_mesh_registry.ron");
        std::fs::create_dir_all(tmp.path().join("data")).unwrap();
        let mut reg = ObjectMeshRegistryFile::default();
        reg.upsert(
            12001,
            "Ironbound Treasure Chest",
            "assets/meshes/objects/chest.ron",
        );
        reg.save(&registry_path).unwrap();

        let db = ObjectMeshDatabase::load_from_registry(&registry_path, tmp.path()).unwrap();
        let pairs = db.all_mesh_ids_with_names();

        assert_eq!(pairs.len(), 1);
        let (id, name) = &pairs[0];
        assert_eq!(id, "12001");
        assert_eq!(name, "Ironbound Treasure Chest");
    }
}
