// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Creature asset persistence for the campaign builder.
//!
//! Active persistence model:
//! - `data/creatures.ron` stores `Vec<CreatureReference>` (registry).
//! - `assets/creatures/*.ron` stores one `CreatureDefinition` per file.
//!
//! Legacy compatibility guard:
//! - If `data/creatures.ron` is detected as inline `Vec<CreatureDefinition>`,
//!   operations return `LegacyInlineRegistryDetected` until migrated.

use antares::domain::types::CreatureId;
use antares::domain::visual::{CreatureDefinition, CreatureReference};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during creature asset operations
#[derive(Error, Debug)]
pub enum CreatureAssetError {
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    #[error("RON serialization error: {0}")]
    SerializationError(#[from] ron::Error),

    #[error("RON deserialization error: {0}")]
    DeserializationError(String),

    #[error("Creature '{0}' not found")]
    CreatureNotFound(String),

    #[error("Campaign directory not found: {0:?}")]
    CampaignNotFound(PathBuf),

    #[error("Creature '{0}' already exists")]
    CreatureExists(String),

    #[error(
        "Legacy creature registry format detected in data/creatures.ron ({count} inline definitions). Run migrate_legacy_inline_registry() first"
    )]
    LegacyInlineRegistryDetected { count: usize },
}

/// Manages creature asset files within a campaign
///
/// Provides high-level operations for creature file management including
/// save, load, list, delete, and duplicate functionality.
#[derive(Debug, Clone)]
pub struct CreatureAssetManager {
    campaign_dir: PathBuf,
}

impl CreatureAssetManager {
    /// Creates a creature asset manager rooted at a campaign directory.
    pub fn new(campaign_dir: PathBuf) -> Self {
        Self { campaign_dir }
    }

    fn data_dir(&self) -> PathBuf {
        self.campaign_dir.join("data")
    }

    fn registry_file(&self) -> PathBuf {
        self.data_dir().join("creatures.ron")
    }

    fn creature_assets_dir(&self) -> PathBuf {
        self.campaign_dir.join("assets").join("creatures")
    }

    fn ensure_reference_storage_dirs(&self) -> Result<(), CreatureAssetError> {
        fs::create_dir_all(self.data_dir())?;
        fs::create_dir_all(self.creature_assets_dir())?;
        Ok(())
    }

    fn creature_asset_path(&self, relative_path: &str) -> PathBuf {
        self.campaign_dir.join(relative_path)
    }

    fn default_filepath_for_creature(creature: &CreatureDefinition) -> String {
        let base = sanitize_creature_filename(&creature.name);
        let file_stem = if base.is_empty() {
            format!("creature_{}", creature.id)
        } else {
            base
        };
        format!("assets/creatures/{}.ron", file_stem)
    }

    fn allocate_unique_filepath(
        &self,
        creature: &CreatureDefinition,
        registry: &[CreatureReference],
    ) -> String {
        let preferred = Self::default_filepath_for_creature(creature);
        if !registry.iter().any(|r| r.filepath == preferred) {
            return preferred;
        }

        let base = sanitize_creature_filename(&creature.name);
        let stem = if base.is_empty() {
            format!("creature_{}", creature.id)
        } else {
            base
        };

        let with_id = format!("assets/creatures/{}_{}.ron", stem, creature.id);
        if !registry.iter().any(|r| r.filepath == with_id) {
            return with_id;
        }

        for n in 1..=1000 {
            let candidate = format!("assets/creatures/{}_{}_{}.ron", stem, creature.id, n);
            if !registry.iter().any(|r| r.filepath == candidate) {
                return candidate;
            }
        }

        format!("assets/creatures/creature_{}_fallback.ron", creature.id)
    }

    fn load_registry_references(&self) -> Result<Vec<CreatureReference>, CreatureAssetError> {
        let registry_file = self.registry_file();
        if !registry_file.exists() {
            return Ok(Vec::new());
        }

        let contents = fs::read_to_string(&registry_file)?;
        if contents.trim().is_empty() {
            return Ok(Vec::new());
        }

        if let Ok(references) = ron::from_str::<Vec<CreatureReference>>(&contents) {
            return Ok(references);
        }

        if let Ok(legacy_inline) = ron::from_str::<Vec<CreatureDefinition>>(&contents) {
            return Err(CreatureAssetError::LegacyInlineRegistryDetected {
                count: legacy_inline.len(),
            });
        }

        ron::from_str::<Vec<CreatureReference>>(&contents)
            .map_err(|e| CreatureAssetError::DeserializationError(e.to_string()))
    }

    fn write_registry_references(
        &self,
        references: &[CreatureReference],
    ) -> Result<(), CreatureAssetError> {
        self.ensure_reference_storage_dirs()?;

        let pretty = ron::ser::PrettyConfig::new()
            .struct_names(false)
            .enumerate_arrays(true)
            .separate_tuple_members(true)
            .depth_limit(2);
        let content = ron::ser::to_string_pretty(references, pretty)?;
        fs::write(self.registry_file(), content)?;
        Ok(())
    }

    fn write_creature_asset(
        &self,
        relative_path: &str,
        creature: &CreatureDefinition,
    ) -> Result<(), CreatureAssetError> {
        self.ensure_reference_storage_dirs()?;
        let asset_path = self.creature_asset_path(relative_path);
        if let Some(parent) = asset_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = ron::ser::to_string_pretty(creature, ron::ser::PrettyConfig::new())?;
        fs::write(asset_path, content)?;
        Ok(())
    }

    fn read_creature_asset(
        &self,
        reference: &CreatureReference,
    ) -> Result<CreatureDefinition, CreatureAssetError> {
        let asset_path = self.creature_asset_path(&reference.filepath);
        let content = fs::read_to_string(&asset_path).map_err(|e| {
            CreatureAssetError::DeserializationError(format!(
                "Failed to read creature asset {}: {}",
                asset_path.display(),
                e
            ))
        })?;

        let mut creature = ron::from_str::<CreatureDefinition>(&content)
            .map_err(|e| CreatureAssetError::DeserializationError(e.to_string()))?;

        // Registry metadata is authoritative for reference-backed loads.
        creature.id = reference.id;
        creature.name = reference.name.clone();

        Ok(creature)
    }

    /// Converts a legacy inline `Vec<CreatureDefinition>` registry into the
    /// current reference-backed format.
    pub fn migrate_legacy_inline_registry(&self) -> Result<usize, CreatureAssetError> {
        let registry_file = self.registry_file();
        if !registry_file.exists() {
            return Ok(0);
        }

        let contents = fs::read_to_string(&registry_file)?;
        if contents.trim().is_empty() {
            return Ok(0);
        }

        if ron::from_str::<Vec<CreatureReference>>(&contents).is_ok() {
            return Ok(0);
        }

        let legacy_creatures = ron::from_str::<Vec<CreatureDefinition>>(&contents)
            .map_err(|e| CreatureAssetError::DeserializationError(e.to_string()))?;

        self.ensure_reference_storage_dirs()?;

        let mut references = Vec::with_capacity(legacy_creatures.len());
        for creature in legacy_creatures {
            let filepath = self.allocate_unique_filepath(&creature, &references);
            self.write_creature_asset(&filepath, &creature)?;
            references.push(CreatureReference {
                id: creature.id,
                name: creature.name,
                filepath,
            });
        }

        self.write_registry_references(&references)?;
        Ok(references.len())
    }

    /// Saves or updates a creature in the reference-backed registry.
    pub fn save_creature(&self, creature: &CreatureDefinition) -> Result<(), CreatureAssetError> {
        let mut references = self.load_registry_references()?;

        let existing_index = references.iter().position(|r| r.id == creature.id);
        let filepath = if let Some(idx) = existing_index {
            references[idx].filepath.clone()
        } else {
            self.allocate_unique_filepath(creature, &references)
        };

        self.write_creature_asset(&filepath, creature)?;

        if let Some(idx) = existing_index {
            references[idx].name = creature.name.clone();
            references[idx].filepath = filepath;
        } else {
            references.push(CreatureReference {
                id: creature.id,
                name: creature.name.clone(),
                filepath,
            });
        }

        references.sort_by_key(|r| r.id);
        self.write_registry_references(&references)
    }

    /// Loads a creature by ID from the reference-backed registry.
    pub fn load_creature(
        &self,
        creature_id: CreatureId,
    ) -> Result<CreatureDefinition, CreatureAssetError> {
        let references = self.load_registry_references()?;
        let reference = references
            .iter()
            .find(|r| r.id == creature_id)
            .ok_or_else(|| CreatureAssetError::CreatureNotFound(creature_id.to_string()))?;

        self.read_creature_asset(reference)
    }

    /// Loads all creatures in registry order.
    pub fn load_all_creatures(&self) -> Result<Vec<CreatureDefinition>, CreatureAssetError> {
        let references = self.load_registry_references()?;
        let mut creatures = Vec::with_capacity(references.len());
        for reference in &references {
            creatures.push(self.read_creature_asset(reference)?);
        }
        Ok(creatures)
    }

    /// Lists creature names from the registry.
    pub fn list_creatures(&self) -> Result<Vec<String>, CreatureAssetError> {
        let references = self.load_registry_references()?;
        Ok(references.into_iter().map(|r| r.name).collect())
    }

    /// Deletes a creature and removes its registry entry.
    pub fn delete_creature(&self, creature_id: CreatureId) -> Result<(), CreatureAssetError> {
        let mut references = self.load_registry_references()?;

        let idx = references
            .iter()
            .position(|r| r.id == creature_id)
            .ok_or_else(|| CreatureAssetError::CreatureNotFound(creature_id.to_string()))?;

        let removed = references.remove(idx);
        self.write_registry_references(&references)?;

        let asset_path = self.creature_asset_path(&removed.filepath);
        if asset_path.exists() {
            fs::remove_file(asset_path)?;
        }

        Ok(())
    }

    /// Duplicates a creature into a new creature ID and name.
    pub fn duplicate_creature(
        &self,
        source_id: CreatureId,
        new_id: CreatureId,
        new_name: String,
    ) -> Result<CreatureDefinition, CreatureAssetError> {
        let mut duplicate = self.load_creature(source_id)?;
        if self.has_creature(new_id)? {
            return Err(CreatureAssetError::CreatureExists(new_id.to_string()));
        }

        duplicate.id = new_id;
        duplicate.name = new_name;
        self.save_creature(&duplicate)?;

        Ok(duplicate)
    }

    /// Returns true if the creature ID exists in the registry.
    pub fn has_creature(&self, creature_id: CreatureId) -> Result<bool, CreatureAssetError> {
        let references = self.load_registry_references()?;
        Ok(references.iter().any(|r| r.id == creature_id))
    }

    /// Returns the next available creature ID.
    pub fn next_creature_id(&self) -> Result<CreatureId, CreatureAssetError> {
        let references = self.load_registry_references()?;
        let max_id = references.iter().map(|r| r.id).max().unwrap_or(0);
        Ok(max_id + 1)
    }
}

fn sanitize_creature_filename(name: &str) -> String {
    let mut sanitized = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();

    while sanitized.contains("__") {
        sanitized = sanitized.replace("__", "_");
    }

    sanitized.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use antares::domain::visual::MeshTransform;
    use std::collections::HashSet;
    use tempfile::TempDir;

    fn create_test_creature(id: CreatureId, name: &str) -> CreatureDefinition {
        CreatureDefinition {
            id,
            name: name.to_string(),
            meshes: vec![],
            mesh_transforms: vec![],
            scale: 1.0,
            color_tint: None,
        }
    }

    fn read_registry(campaign_dir: &Path) -> Vec<CreatureReference> {
        let path = campaign_dir.join("data/creatures.ron");
        let content = fs::read_to_string(path).unwrap();
        ron::from_str::<Vec<CreatureReference>>(&content).unwrap()
    }

    #[test]
    fn test_save_and_load_creature_uses_reference_backed_files() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        let creature = create_test_creature(1, "Test Creature");
        manager.save_creature(&creature).unwrap();

        let registry = read_registry(temp_dir.path());
        assert_eq!(registry.len(), 1);
        assert_eq!(registry[0].id, 1);
        assert_eq!(registry[0].name, "Test Creature");

        let asset_file = temp_dir.path().join(&registry[0].filepath);
        assert!(asset_file.exists(), "creature asset file should be written");

        let loaded = manager.load_creature(1).unwrap();
        assert_eq!(loaded.id, 1);
        assert_eq!(loaded.name, "Test Creature");
    }

    #[test]
    fn test_round_trip_multiple_creatures_and_registry_integrity() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        manager
            .save_creature(&create_test_creature(1, "Goblin"))
            .unwrap();
        manager
            .save_creature(&create_test_creature(2, "Orc"))
            .unwrap();
        manager
            .save_creature(&create_test_creature(3, "Troll"))
            .unwrap();

        let loaded = manager.load_all_creatures().unwrap();
        assert_eq!(loaded.len(), 3);

        let ids: HashSet<CreatureId> = loaded.into_iter().map(|c| c.id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
        assert!(ids.contains(&3));

        let registry = read_registry(temp_dir.path());
        assert_eq!(registry.len(), 3);
        for reference in &registry {
            assert!(
                temp_dir.path().join(&reference.filepath).exists(),
                "asset path from registry must exist: {}",
                reference.filepath
            );
        }
    }

    #[test]
    fn test_delete_creature_removes_registry_entry_and_asset_file() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        manager
            .save_creature(&create_test_creature(1, "Goblin"))
            .unwrap();
        let registry_before = read_registry(temp_dir.path());
        let asset = temp_dir.path().join(&registry_before[0].filepath);
        assert!(asset.exists());

        manager.delete_creature(1).unwrap();

        let registry_after = read_registry(temp_dir.path());
        assert!(registry_after.is_empty());
        assert!(!asset.exists());
    }

    #[test]
    fn test_duplicate_creature_creates_new_registry_entry_and_asset_file() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        manager
            .save_creature(&create_test_creature(7, "Original"))
            .unwrap();

        let duplicate = manager
            .duplicate_creature(7, 8, "Original Copy".to_string())
            .unwrap();
        assert_eq!(duplicate.id, 8);
        assert_eq!(duplicate.name, "Original Copy");

        let registry = read_registry(temp_dir.path());
        assert_eq!(registry.len(), 2);
        assert!(registry.iter().any(|r| r.id == 7));
        assert!(registry.iter().any(|r| r.id == 8));
    }

    #[test]
    fn test_legacy_inline_registry_detection_guard() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());
        fs::create_dir_all(temp_dir.path().join("data")).unwrap();

        let legacy = vec![create_test_creature(1, "Legacy Goblin")];
        let legacy_content =
            ron::ser::to_string_pretty(&legacy, ron::ser::PrettyConfig::new()).unwrap();
        fs::write(temp_dir.path().join("data/creatures.ron"), legacy_content).unwrap();

        let result = manager.load_all_creatures();
        assert!(matches!(
            result,
            Err(CreatureAssetError::LegacyInlineRegistryDetected { count: 1 })
        ));
    }

    #[test]
    fn test_migrate_legacy_inline_registry_to_reference_model() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());
        fs::create_dir_all(temp_dir.path().join("data")).unwrap();

        let legacy = vec![
            create_test_creature(1, "Legacy Goblin"),
            create_test_creature(2, "Legacy Orc"),
        ];
        let legacy_content =
            ron::ser::to_string_pretty(&legacy, ron::ser::PrettyConfig::new()).unwrap();
        fs::write(temp_dir.path().join("data/creatures.ron"), legacy_content).unwrap();

        let migrated = manager.migrate_legacy_inline_registry().unwrap();
        assert_eq!(migrated, 2);

        let loaded = manager.load_all_creatures().unwrap();
        assert_eq!(loaded.len(), 2);

        let registry = read_registry(temp_dir.path());
        assert_eq!(registry.len(), 2);
        assert!(
            temp_dir.path().join(&registry[0].filepath).exists(),
            "first migrated asset file should exist"
        );
        assert!(
            temp_dir.path().join(&registry[1].filepath).exists(),
            "second migrated asset file should exist"
        );
    }

    #[test]
    fn test_next_creature_id_and_has_creature_with_reference_registry() {
        let temp_dir = TempDir::new().unwrap();
        let manager = CreatureAssetManager::new(temp_dir.path().to_path_buf());

        assert_eq!(manager.next_creature_id().unwrap(), 1);
        assert!(!manager.has_creature(1).unwrap());

        manager
            .save_creature(&create_test_creature(5, "Five"))
            .unwrap();
        manager
            .save_creature(&create_test_creature(11, "Eleven"))
            .unwrap();

        assert!(manager.has_creature(5).unwrap());
        assert!(manager.has_creature(11).unwrap());
        assert!(!manager.has_creature(4).unwrap());
        assert_eq!(manager.next_creature_id().unwrap(), 12);
    }

    #[test]
    fn test_load_all_creatures_supports_shared_asset_filepath_aliasing() {
        let temp_dir = TempDir::new().unwrap();
        let campaign_dir = temp_dir.path();
        let manager = CreatureAssetManager::new(campaign_dir.to_path_buf());

        fs::create_dir_all(campaign_dir.join("data")).unwrap();
        fs::create_dir_all(campaign_dir.join("assets/creatures")).unwrap();

        let shared_creature = create_test_creature(12, "Wolf");
        let shared_content =
            ron::ser::to_string_pretty(&shared_creature, ron::ser::PrettyConfig::new()).unwrap();
        fs::write(
            campaign_dir.join("assets/creatures/wolf.ron"),
            shared_content,
        )
        .unwrap();

        let references = vec![
            CreatureReference {
                id: 4,
                name: "DireWolf".to_string(),
                filepath: "assets/creatures/wolf.ron".to_string(),
            },
            CreatureReference {
                id: 5,
                name: "DireWolfLeader".to_string(),
                filepath: "assets/creatures/wolf.ron".to_string(),
            },
            CreatureReference {
                id: 12,
                name: "Wolf".to_string(),
                filepath: "assets/creatures/wolf.ron".to_string(),
            },
        ];

        let registry_content =
            ron::ser::to_string_pretty(&references, ron::ser::PrettyConfig::new()).unwrap();
        fs::write(campaign_dir.join("data/creatures.ron"), registry_content).unwrap();

        let creatures = manager.load_all_creatures().unwrap();
        assert_eq!(creatures.len(), 3);
        assert!(creatures.iter().any(|c| c.id == 4 && c.name == "DireWolf"));
        assert!(creatures
            .iter()
            .any(|c| c.id == 5 && c.name == "DireWolfLeader"));
        assert!(creatures.iter().any(|c| c.id == 12 && c.name == "Wolf"));
    }
}
