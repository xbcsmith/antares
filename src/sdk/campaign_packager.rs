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
//! See `docs/explanation/sdk_and_campaign_architecture.md` for specifications.
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

use crate::domain::path_security::validate_identifier;
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

    #[error("Unsafe campaign id: {0}")]
    UnsafeCampaignId(String),

    #[error("Archive too large: uncompressed size exceeds {limit} bytes")]
    ArchiveTooLarge {
        /// The cumulative uncompressed-size cap (in bytes) that was exceeded.
        limit: u64,
    },
}

/// Maximum cumulative uncompressed size permitted when extracting a package.
///
/// Guards against decompression-bomb archives: a small `.tar.gz` can inflate to
/// many gigabytes and exhaust disk space. Extraction is aborted with
/// [`PackageError::ArchiveTooLarge`] once the summed uncompressed entry sizes
/// exceed this cap (512 MiB), which is comfortably larger than any legitimate
/// campaign package.
const MAX_UNCOMPRESSED_BYTES: u64 = 512 * 1024 * 1024;

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

        // Open the archive for streaming extraction.
        let tar_gz = File::open(package_path)?;
        let dec = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(dec);

        // Extract to temporary location first
        let temp_dir = campaigns_dir.join(".tmp_install");
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir)?;
        }
        fs::create_dir(&temp_dir)?;

        // Extract entries one at a time, enforcing a cumulative uncompressed-size
        // cap. This prevents a crafted "decompression bomb" from filling the disk:
        // the running total is checked from each entry's header *before* the entry
        // is written, so an oversized archive is never fully extracted.
        let entries = archive
            .entries()
            .map_err(|e| PackageError::ArchiveError(format!("Failed to read archive: {}", e)))?;
        let mut total_uncompressed: u64 = 0;
        for entry in entries {
            let mut entry = entry.map_err(|e| {
                PackageError::ArchiveError(format!("Failed to read archive entry: {}", e))
            })?;
            let entry_size = entry.header().size().map_err(|e| {
                PackageError::ArchiveError(format!("Failed to read entry header: {}", e))
            })?;
            total_uncompressed = total_uncompressed.saturating_add(entry_size);
            if total_uncompressed > MAX_UNCOMPRESSED_BYTES {
                fs::remove_dir_all(&temp_dir).ok();
                return Err(PackageError::ArchiveTooLarge {
                    limit: MAX_UNCOMPRESSED_BYTES,
                });
            }
            // `unpack_in` refuses to write outside `temp_dir` (it rejects absolute
            // and `..` entry paths), so it is safe against tar-path traversal.
            entry.unpack_in(&temp_dir).map_err(|e| {
                PackageError::ArchiveError(format!("Failed to extract archive: {}", e))
            })?;
        }

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

        // Validate the campaign_id from the (untrusted) manifest before using it
        // as a directory name. This rejects path traversal (e.g. "../evil"),
        // absolute paths, and embedded separators ("a/b") that could redirect the
        // install outside the campaigns directory.
        if let Err(e) = validate_identifier(&manifest.campaign_id) {
            fs::remove_dir_all(&temp_dir).ok();
            return Err(PackageError::UnsafeCampaignId(format!(
                "'{}': {}",
                manifest.campaign_id, e
            )));
        }

        // Determine installation path
        let install_path = campaigns_dir.join(&manifest.campaign_id);

        // Defense-in-depth: ensure the resolved install path is a direct child of
        // the campaigns directory. `validate_identifier` already guarantees this,
        // but re-checking the computed path guards against any future regression.
        if install_path.parent() != Some(campaigns_dir) {
            fs::remove_dir_all(&temp_dir).ok();
            return Err(PackageError::UnsafeCampaignId(manifest.campaign_id.clone()));
        }

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

/// Computes the cumulative uncompressed size declared by a `.tar.gz` package's
/// tar entry headers.
///
/// This reads only the tar entry *headers* (the declared per-entry size), not
/// the entry bodies, and sums them. It underpins the decompression-bomb guard in
/// [`CampaignPackager::install_package`], and is exposed as a standalone helper
/// so the size-summing logic can be unit-tested directly against a fixture
/// archive.
///
/// # Arguments
///
/// * `package_path` - Path to the `.tar.gz` package file to inspect
///
/// # Returns
///
/// The sum of all entry header sizes, saturating at [`u64::MAX`].
///
/// # Errors
///
/// Returns [`PackageError::IoError`] if the package cannot be opened, and
/// [`PackageError::ArchiveError`] if the archive or an entry header cannot be
/// read.
#[cfg(test)]
fn total_uncompressed_size(package_path: &Path) -> Result<u64, PackageError> {
    let tar_gz = File::open(package_path)?;
    let dec = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(dec);

    let entries = archive
        .entries()
        .map_err(|e| PackageError::ArchiveError(e.to_string()))?;

    let mut total: u64 = 0;
    for entry in entries {
        let entry = entry.map_err(|e| PackageError::ArchiveError(e.to_string()))?;
        let size = entry
            .header()
            .size()
            .map_err(|e| PackageError::ArchiveError(e.to_string()))?;
        total = total.saturating_add(size);
    }
    Ok(total)
}

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
    Ok(hash.iter().map(|byte| format!("{byte:02x}")).collect())
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
        let campaign = crate::sdk::campaign_loader::test_fixtures::make_test_campaign();

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
        let campaign = crate::sdk::campaign_loader::test_fixtures::make_test_campaign();

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

    /// Recursively copy a directory (helper for tests)
    fn copy_dir_all(src: &std::path::Path, dest: &std::path::Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dest)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let dest_path = dest.join(entry.file_name());
            if path.is_dir() {
                copy_dir_all(&path, &dest_path)?;
            } else {
                std::fs::copy(&path, &dest_path)?;
            }
        }
        Ok(())
    }

    #[test]
    fn test_package_and_install_preserves_vec_fields() -> Result<(), Box<dyn std::error::Error>> {
        use crate::domain::world::MapEvent;
        use std::path::PathBuf;
        use tempfile::tempdir;

        // Source: test campaign provided in repo under data/test_campaign
        let src_campaign = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/test_campaign");
        assert!(
            src_campaign.exists(),
            "expected test campaign to exist at {:?}",
            src_campaign
        );

        // Create a temporary directory and copy the campaign there
        let tmp_dir = tempdir()?;
        let campaign_dir = tmp_dir.path().join("tutorial");
        copy_dir_all(&src_campaign, &campaign_dir)?;

        // Create an output package path
        let out_pkg = tmp_dir.path().join("tutorial_pkg.tar.gz");

        // Use the packager to create a package
        let packager = CampaignPackager::new();
        let _manifest = packager.package_campaign(&campaign_dir, &out_pkg)?;

        // Install the package to a fresh install directory
        let install_dir = tmp_dir.path().join("installed");
        std::fs::create_dir_all(&install_dir)?;
        let installed_path = packager.install_package(&out_pkg, &install_dir)?;

        // Load the installed campaign and its content database
        let campaign = crate::sdk::campaign_loader::Campaign::load(&installed_path)?;
        let db = campaign.load_content()?;

        // Verify: At least one map contains an Encounter event with a non-empty monster_group Vec
        let mut found_encounter = false;
        for map_id in db.maps.all_maps() {
            if let Some(map) = db.maps.get_map(map_id) {
                for event in map.events.values() {
                    if let MapEvent::Encounter { monster_group, .. } = event {
                        if !monster_group.is_empty() {
                            found_encounter = true;
                            break;
                        }
                    }
                }
            }
            if found_encounter {
                break;
            }
        }

        assert!(
            found_encounter,
            "Encounter with non-empty monster_group not found after pack/install"
        );

        Ok(())
    }

    /// Builds a minimal valid `.tar.gz` package containing only a `MANIFEST.json`
    /// whose `campaign_id` is set to `campaign_id` (and no data files, so
    /// checksum validation is trivially satisfied). Used to exercise
    /// `install_package`'s campaign_id validation.
    fn build_package_with_campaign_id(
        out_path: &std::path::Path,
        campaign_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let manifest = PackageManifest {
            version: "1.0".to_string(),
            campaign_id: campaign_id.to_string(),
            campaign_name: "Malicious".to_string(),
            campaign_version: "1.0.0".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            files: Vec::new(),
            total_size: 0,
        };

        let tar_gz = File::create(out_path)?;
        let enc = GzEncoder::new(tar_gz, Compression::new(6));
        let mut tar = Builder::new(enc);

        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        let manifest_bytes = manifest_json.as_bytes();
        let mut header = tar::Header::new_gnu();
        header.set_size(manifest_bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, "MANIFEST.json", manifest_bytes)?;
        tar.finish()?;
        Ok(())
    }

    #[test]
    fn test_install_rejects_malicious_campaign_id() -> Result<(), Box<dyn std::error::Error>> {
        use tempfile::tempdir;

        let tmp = tempdir()?;
        let campaigns_dir = tmp.path().join("campaigns");
        std::fs::create_dir_all(&campaigns_dir)?;
        let packager = CampaignPackager::new();

        // Traversal, absolute path, and embedded-separator ids must all be rejected.
        for bad in ["../evil", "/abs/evil", "a/b"] {
            let pkg = tmp.path().join("bad.tar.gz");
            build_package_with_campaign_id(&pkg, bad)?;

            let result = packager.install_package(&pkg, &campaigns_dir);
            assert!(
                matches!(result, Err(PackageError::UnsafeCampaignId(_))),
                "expected UnsafeCampaignId for {:?}, got {:?}",
                bad,
                result
            );

            // The temp extraction dir must be cleaned up and nothing installed.
            assert!(
                !campaigns_dir.join(".tmp_install").exists(),
                "temp install dir left behind for {:?}",
                bad
            );
            std::fs::remove_file(&pkg).ok();
        }

        // No campaign directory (or anything else) should have been written under
        // the campaigns directory.
        let entries: Vec<_> = std::fs::read_dir(&campaigns_dir)?.collect::<Result<_, _>>()?;
        assert!(
            entries.is_empty(),
            "campaigns_dir should be empty, found {} entries",
            entries.len()
        );
        Ok(())
    }

    #[test]
    fn test_total_uncompressed_size_sums_entry_headers() -> Result<(), Box<dyn std::error::Error>> {
        use tempfile::tempdir;

        let tmp = tempdir()?;
        let pkg = tmp.path().join("sized.tar.gz");

        let tar_gz = File::create(&pkg)?;
        let enc = GzEncoder::new(tar_gz, Compression::new(6));
        let mut tar = Builder::new(enc);

        let a: &[u8] = b"hello world"; // 11 bytes
        let b = vec![0u8; 1000]; // 1000 bytes
        for (name, data) in [("a.txt", a), ("b.bin", b.as_slice())] {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, name, data)?;
        }
        tar.finish()?;
        // Finalize the gzip stream explicitly: dropping the encoder without
        // finishing can truncate the gzip footer, causing an "unexpected end of
        // file" when the archive is read back.
        let enc = tar.into_inner()?;
        enc.finish()?;

        let total = total_uncompressed_size(&pkg)?;
        assert_eq!(total, 11 + 1000);
        Ok(())
    }

    /// Builds a tiny archive whose single entry header *claims* `claimed_size`
    /// uncompressed bytes but is followed immediately by the end-of-archive
    /// marker (no body). This models a decompression bomb: the declared
    /// uncompressed size is enormous while the archive itself is a few hundred
    /// bytes.
    fn build_bomb_header_archive(
        out_path: &std::path::Path,
        claimed_size: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::io::Write;

        let mut header = tar::Header::new_gnu();
        header.set_path("MANIFEST.json")?;
        header.set_size(claimed_size);
        header.set_mode(0o644);
        header.set_cksum();

        let tar_gz = File::create(out_path)?;
        let mut enc = GzEncoder::new(tar_gz, Compression::new(6));
        enc.write_all(header.as_bytes())?;
        // Two zero blocks terminate the tar archive.
        enc.write_all(&[0u8; 512])?;
        enc.write_all(&[0u8; 512])?;
        enc.finish()?;
        Ok(())
    }

    #[test]
    fn test_install_rejects_decompression_bomb() -> Result<(), Box<dyn std::error::Error>> {
        use tempfile::tempdir;

        let tmp = tempdir()?;
        let campaigns_dir = tmp.path().join("campaigns");
        std::fs::create_dir_all(&campaigns_dir)?;
        let pkg = tmp.path().join("bomb.tar.gz");

        // Claim a size just over the 512 MiB cap; the running total is checked
        // from the header before the (nonexistent) body is read.
        build_bomb_header_archive(&pkg, MAX_UNCOMPRESSED_BYTES + 1)?;

        let packager = CampaignPackager::new();
        let result = packager.install_package(&pkg, &campaigns_dir);
        assert!(
            matches!(
                result,
                Err(PackageError::ArchiveTooLarge { limit }) if limit == MAX_UNCOMPRESSED_BYTES
            ),
            "expected ArchiveTooLarge, got {:?}",
            result
        );

        // The temp extraction dir must be cleaned up.
        assert!(!campaigns_dir.join(".tmp_install").exists());
        Ok(())
    }
}
