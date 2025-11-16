// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Campaign packager module
//!
//! This module provides tools for packaging campaigns for distribution and
//! installing campaign packages. Campaigns can be exported as tar.gz archives
//! with metadata and checksums for validation.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sdk_and_campaign_architecture.md` Phase 7 for specifications.
//!
//! # Examples
//!
//! ```no_run
//! use antares::sdk::campaign_packager::CampaignPackager;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let packager = CampaignPackager::new();
//!
//! // Package a campaign
//! packager.package_campaign("campaigns/my_campaign", "my_campaign_v1.0.0.tar.gz")?;
//!
//! // Install a campaign package
//! packager.install_package("my_campaign_v1.0.0.tar.gz", "campaigns")?;
//! # Ok(())
//! # }
//! ```

use crate::sdk::campaign_loader::Campaign;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur when packaging or installing campaigns
#[derive(Error, Debug)]
pub enum PackageError {
    #[error("Campaign not found: {0}")]
    CampaignNotFound(String),

    #[error("Package file not found: {0}")]
    PackageNotFound(String),

    #[error("Invalid package format: {0}")]
    InvalidFormat(String),

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Campaign already exists: {0}")]
    CampaignExists(String),

    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    #[error("Archive error: {0}")]
    ArchiveError(String),

    #[error("Metadata error: {0}")]
    MetadataError(String),
}

// ===== Package Metadata =====

/// Package metadata stored in manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    /// Package format version
    pub version: String,

    /// Campaign ID
    pub campaign_id: String,

    /// Campaign name
    pub campaign_name: String,

    /// Campaign version
    pub campaign_version: String,

    /// Package creation timestamp
    pub created_at: String,

    /// List of files with checksums
    pub files: Vec<FileEntry>,

    /// Total package size in bytes
    pub total_size: u64,
}

/// File entry with checksum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Relative path within campaign
    pub path: String,

    /// SHA-256 checksum
    pub checksum: String,

    /// File size in bytes
    pub size: u64,
}

impl PackageManifest {
    /// Creates a new package manifest
    pub fn new(campaign: &Campaign) -> Self {
        let now = chrono::Utc::now();
        Self {
            version: "1.0".to_string(),
            campaign_id: campaign.id.clone(),
            campaign_name: campaign.name.clone(),
            campaign_version: campaign.version.clone(),
            created_at: now.to_rfc3339(),
            files: Vec::new(),
            total_size: 0,
        }
    }

    /// Adds a file entry to the manifest
    pub fn add_file(&mut self, path: String, checksum: String, size: u64) {
        self.files.push(FileEntry {
            path,
            checksum,
            size,
        });
        self.total_size += size;
    }
}

// ===== Campaign Packager =====

/// Campaign packager for export and installation
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::campaign_packager::CampaignPackager;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let packager = CampaignPackager::new();
///
/// // Package campaign
/// let package_info = packager.package_campaign(
///     "campaigns/my_campaign",
///     "my_campaign_v1.0.0.tar.gz"
/// )?;
///
/// println!("Created package: {} bytes", package_info.total_size);
/// # Ok(())
/// # }
/// ```
pub struct CampaignPackager {
    /// Compression level (0-9, default: 6)
    compression_level: u32,
}

impl CampaignPackager {
    /// Creates a new campaign packager with default compression
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::campaign_packager::CampaignPackager;
    ///
    /// let packager = CampaignPackager::new();
    /// ```
    pub fn new() -> Self {
        Self {
            compression_level: 6,
        }
    }

    /// Creates a packager with custom compression level (0-9)
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::campaign_packager::CampaignPackager;
    ///
    /// // Maximum compression
    /// let packager = CampaignPackager::with_compression(9);
    /// ```
    pub fn with_compression(level: u32) -> Self {
        Self {
            compression_level: level.min(9),
        }
    }

    /// Package a campaign for distribution
    ///
    /// Creates a .tar.gz archive containing the campaign files and a manifest
    /// with checksums for validation.
    ///
    /// # Arguments
    ///
    /// * `campaign_path` - Path to the campaign directory
    /// * `output_path` - Path where the package should be created
    ///
    /// # Returns
    ///
    /// Returns `PackageManifest` with package metadata
    ///
    /// # Errors
    ///
    /// Returns `PackageError` if:
    /// - Campaign directory doesn't exist
    /// - Cannot create output file
    /// - Cannot read campaign files
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::sdk::campaign_packager::CampaignPackager;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let packager = CampaignPackager::new();
    /// let manifest = packager.package_campaign(
    ///     "campaigns/example",
    ///     "example_v1.0.0.tar.gz"
    /// )?;
    /// println!("Package created with {} files", manifest.files.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn package_campaign<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        campaign_path: P,
        output_path: Q,
    ) -> Result<PackageManifest, PackageError> {
        let campaign_path = campaign_path.as_ref();
        let output_path = output_path.as_ref();

        // Verify campaign exists
        if !campaign_path.exists() {
            return Err(PackageError::CampaignNotFound(
                campaign_path.display().to_string(),
            ));
        }

        // Load campaign metadata
        let campaign = Campaign::load(campaign_path)
            .map_err(|e| PackageError::MetadataError(e.to_string()))?;

        // Create manifest
        let mut manifest = PackageManifest::new(&campaign);

        // Create tar.gz archive
        let tar_gz = File::create(output_path)?;
        let enc = GzEncoder::new(tar_gz, Compression::new(self.compression_level));
        let mut tar = Builder::new(enc);

        // Add files to archive with checksums
        Self::add_directory_to_archive(&mut tar, campaign_path, campaign_path, &mut manifest)?;

        // Write manifest to archive
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| PackageError::MetadataError(e.to_string()))?;
        let manifest_bytes = manifest_json.as_bytes();
        let mut header = tar::Header::new_gnu();
        header.set_size(manifest_bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, "MANIFEST.json", manifest_bytes)?;

        // Finalize archive
        tar.finish()?;

        Ok(manifest)
    }

    /// Install a campaign package
    ///
    /// Extracts a campaign package to the specified campaigns directory,
    /// validates checksums, and ensures the campaign is properly installed.
    ///
    /// # Arguments
    ///
    /// * `package_path` - Path to the .tar.gz package file
    /// * `campaigns_dir` - Directory where campaigns are installed
    ///
    /// # Returns
    ///
    /// Returns the installed campaign's directory path
    ///
    /// # Errors
    ///
    /// Returns `PackageError` if:
    /// - Package file doesn't exist
    /// - Package format is invalid
    /// - Checksum validation fails
    /// - Campaign already exists (unless overwrite is enabled)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::sdk::campaign_packager::CampaignPackager;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let packager = CampaignPackager::new();
    /// let installed_path = packager.install_package(
    ///     "example_v1.0.0.tar.gz",
    ///     "campaigns"
    /// )?;
    /// println!("Installed to: {}", installed_path.display());
    /// # Ok(())
    /// # }
    /// ```
    pub fn install_package<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        package_path: P,
        campaigns_dir: Q,
    ) -> Result<PathBuf, PackageError> {
        let package_path = package_path.as_ref();
        let campaigns_dir = campaigns_dir.as_ref();

        // Verify package exists
        if !package_path.exists() {
            return Err(PackageError::PackageNotFound(
                package_path.display().to_string(),
            ));
        }

        // Create campaigns directory if it doesn't exist
        fs::create_dir_all(campaigns_dir)?;

        // Open and extract archive
        let tar_gz = File::open(package_path)?;
        let dec = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(dec);

        // Extract to temporary location first
        let temp_dir = campaigns_dir.join(".tmp_install");
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir)?;
        }
        fs::create_dir(&temp_dir)?;

        // Extract all files
        archive
            .unpack(&temp_dir)
            .map_err(|e| PackageError::ArchiveError(format!("Failed to extract archive: {}", e)))?;

        // Load and validate manifest
        let manifest_path = temp_dir.join("MANIFEST.json");
        if !manifest_path.exists() {
            fs::remove_dir_all(&temp_dir)?;
            return Err(PackageError::InvalidFormat(
                "Missing MANIFEST.json".to_string(),
            ));
        }

        let manifest_content = fs::read_to_string(&manifest_path)?;
        let manifest: PackageManifest = serde_json::from_str(&manifest_content)
            .map_err(|e| PackageError::MetadataError(e.to_string()))?;

        // Validate checksums
        for file_entry in &manifest.files {
            let file_path = temp_dir.join(&file_entry.path);
            if file_path.exists() {
                let actual_checksum = calculate_checksum(&file_path)?;
                if actual_checksum != file_entry.checksum {
                    fs::remove_dir_all(&temp_dir)?;
                    return Err(PackageError::ChecksumMismatch {
                        expected: file_entry.checksum.clone(),
                        actual: actual_checksum,
                    });
                }
            }
        }

        // Determine installation path
        let install_path = campaigns_dir.join(&manifest.campaign_id);

        // Check if campaign already exists
        if install_path.exists() {
            fs::remove_dir_all(&temp_dir)?;
            return Err(PackageError::CampaignExists(manifest.campaign_id.clone()));
        }

        // Move from temp to final location
        fs::rename(&temp_dir, &install_path)?;

        // Remove manifest file from installed campaign
        let installed_manifest = install_path.join("MANIFEST.json");
        if installed_manifest.exists() {
            fs::remove_file(installed_manifest)?;
        }

        Ok(install_path)
    }

    /// Helper to add directory contents to tar archive recursively
    fn add_directory_to_archive(
        tar: &mut Builder<GzEncoder<File>>,
        dir: &Path,
        base_path: &Path,
        manifest: &mut PackageManifest,
    ) -> Result<(), PackageError> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let relative_path = path
                .strip_prefix(base_path)
                .map_err(|e| PackageError::ArchiveError(e.to_string()))?;

            // Skip hidden files and certain directories
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                if name_str.starts_with('.') || name_str == "target" || name_str == "node_modules" {
                    continue;
                }
            }

            if path.is_dir() {
                // Recursively add subdirectory
                Self::add_directory_to_archive(tar, &path, base_path, manifest)?;
            } else {
                // Add file to archive
                let mut file = File::open(&path)?;
                let metadata = file.metadata()?;
                let size = metadata.len();

                // Calculate checksum
                let checksum = calculate_checksum(&path)?;

                // Add to tar
                tar.append_file(relative_path, &mut file)
                    .map_err(|e| PackageError::ArchiveError(e.to_string()))?;

                // Add to manifest
                manifest.add_file(relative_path.to_string_lossy().to_string(), checksum, size);
            }
        }

        Ok(())
    }
}

impl Default for CampaignPackager {
    fn default() -> Self {
        Self::new()
    }
}

// ===== Helper Functions =====

/// Calculate SHA-256 checksum of a file
fn calculate_checksum<P: AsRef<Path>>(path: P) -> Result<String, PackageError> {
    use sha2::{Digest, Sha256};

    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packager_creation() {
        let packager = CampaignPackager::new();
        assert_eq!(packager.compression_level, 6);
    }

    #[test]
    fn test_packager_custom_compression() {
        let packager = CampaignPackager::with_compression(9);
        assert_eq!(packager.compression_level, 9);

        // Test clamping
        let packager = CampaignPackager::with_compression(100);
        assert_eq!(packager.compression_level, 9);
    }

    #[test]
    fn test_package_manifest_creation() {
        let campaign = Campaign {
            id: "test".to_string(),
            name: "Test Campaign".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Test".to_string(),
            engine_version: "0.1.0".to_string(),
            required_features: Vec::new(),
            config: crate::sdk::campaign_loader::CampaignConfig {
                starting_map: 1,
                starting_position: crate::domain::types::Position::new(0, 0),
                starting_direction: crate::domain::types::Direction::North,
                starting_gold: 100,
                starting_food: 50,
                max_party_size: 6,
                max_roster_size: 20,
                difficulty: crate::sdk::campaign_loader::Difficulty::Normal,
                permadeath: false,
                allow_multiclassing: false,
                starting_level: 1,
                max_level: 20,
            },
            data: crate::sdk::campaign_loader::CampaignData {
                items: "data/items.ron".to_string(),
                spells: "data/spells.ron".to_string(),
                monsters: "data/monsters.ron".to_string(),
                classes: "data/classes.ron".to_string(),
                races: "data/races.ron".to_string(),
                maps: "data/maps".to_string(),
                quests: "data/quests.ron".to_string(),
                dialogues: "data/dialogues.ron".to_string(),
            },
            assets: crate::sdk::campaign_loader::CampaignAssets {
                tilesets: "assets/tilesets".to_string(),
                music: "assets/music".to_string(),
                sounds: "assets/sounds".to_string(),
                images: "assets/images".to_string(),
            },
            root_path: PathBuf::new(),
        };

        let manifest = PackageManifest::new(&campaign);
        assert_eq!(manifest.campaign_id, "test");
        assert_eq!(manifest.campaign_name, "Test Campaign");
        assert_eq!(manifest.campaign_version, "1.0.0");
        assert_eq!(manifest.version, "1.0");
        assert_eq!(manifest.files.len(), 0);
        assert_eq!(manifest.total_size, 0);
    }

    #[test]
    fn test_manifest_add_file() {
        let campaign = Campaign {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Test".to_string(),
            engine_version: "0.1.0".to_string(),
            required_features: Vec::new(),
            config: crate::sdk::campaign_loader::CampaignConfig {
                starting_map: 1,
                starting_position: crate::domain::types::Position::new(0, 0),
                starting_direction: crate::domain::types::Direction::North,
                starting_gold: 100,
                starting_food: 50,
                max_party_size: 6,
                max_roster_size: 20,
                difficulty: crate::sdk::campaign_loader::Difficulty::Normal,
                permadeath: false,
                allow_multiclassing: false,
                starting_level: 1,
                max_level: 20,
            },
            data: crate::sdk::campaign_loader::CampaignData {
                items: "data/items.ron".to_string(),
                spells: "data/spells.ron".to_string(),
                monsters: "data/monsters.ron".to_string(),
                classes: "data/classes.ron".to_string(),
                races: "data/races.ron".to_string(),
                maps: "data/maps".to_string(),
                quests: "data/quests.ron".to_string(),
                dialogues: "data/dialogues.ron".to_string(),
            },
            assets: crate::sdk::campaign_loader::CampaignAssets {
                tilesets: "assets/tilesets".to_string(),
                music: "assets/music".to_string(),
                sounds: "assets/sounds".to_string(),
                images: "assets/images".to_string(),
            },
            root_path: PathBuf::new(),
        };

        let mut manifest = PackageManifest::new(&campaign);
        manifest.add_file("test.txt".to_string(), "abc123".to_string(), 100);

        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.total_size, 100);
        assert_eq!(manifest.files[0].path, "test.txt");
        assert_eq!(manifest.files[0].checksum, "abc123");
        assert_eq!(manifest.files[0].size, 100);
    }

    #[test]
    fn test_packager_default() {
        let packager = CampaignPackager::default();
        assert_eq!(packager.compression_level, 6);
    }
}
