//! Campaign Packager Integration Module
//!
//! This module provides UI and logic for packaging campaigns into distributable
//! .zip archives using the SDK's campaign_packager functionality.

use crate::{CampaignBuilderApp, CampaignError};
use antares::sdk::campaign_packager::{CampaignPackager, PackageError};
use std::path::PathBuf;

/// Export wizard state for multi-step packaging process
#[derive(Debug, Clone, PartialEq)]
pub enum ExportWizardStep {
    /// Initial step - validation and overview
    Validation,
    /// File selection - choose assets to include
    FileSelection,
    /// Metadata confirmation - version, author, description
    Metadata,
    /// Export settings - compression, output location
    Settings,
    /// Export in progress
    Exporting,
    /// Export complete
    Complete,
}

/// Export wizard state management
#[derive(Debug, Clone)]
pub struct ExportWizard {
    /// Current step in the wizard
    pub current_step: ExportWizardStep,
    /// Validation passed flag
    pub validation_passed: bool,
    /// Selected files to include in package
    pub selected_files: Vec<PathBuf>,
    /// Output path for .zip file
    pub output_path: Option<PathBuf>,
    /// Compression level (0-9)
    pub compression_level: u8,
    /// Include README flag
    pub include_readme: bool,
    /// Include all maps flag
    pub include_all_maps: bool,
    /// Export progress message
    pub progress_message: String,
    /// Export complete flag
    pub export_complete: bool,
    /// Export error if any
    pub export_error: Option<String>,
}

impl Default for ExportWizard {
    fn default() -> Self {
        Self {
            current_step: ExportWizardStep::Validation,
            validation_passed: false,
            selected_files: Vec::new(),
            output_path: None,
            compression_level: 6, // Default compression
            include_readme: true,
            include_all_maps: true,
            progress_message: String::new(),
            export_complete: false,
            export_error: None,
        }
    }
}

impl ExportWizard {
    /// Creates a new export wizard
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances to the next step
    pub fn next_step(&mut self) {
        self.current_step = match self.current_step {
            ExportWizardStep::Validation => ExportWizardStep::FileSelection,
            ExportWizardStep::FileSelection => ExportWizardStep::Metadata,
            ExportWizardStep::Metadata => ExportWizardStep::Settings,
            ExportWizardStep::Settings => ExportWizardStep::Exporting,
            ExportWizardStep::Exporting => ExportWizardStep::Complete,
            ExportWizardStep::Complete => ExportWizardStep::Complete,
        };
    }

    /// Goes back to the previous step
    pub fn previous_step(&mut self) {
        self.current_step = match self.current_step {
            ExportWizardStep::Validation => ExportWizardStep::Validation,
            ExportWizardStep::FileSelection => ExportWizardStep::Validation,
            ExportWizardStep::Metadata => ExportWizardStep::FileSelection,
            ExportWizardStep::Settings => ExportWizardStep::Metadata,
            ExportWizardStep::Exporting => ExportWizardStep::Settings,
            ExportWizardStep::Complete => ExportWizardStep::Settings,
        };
    }

    /// Resets the wizard to initial state
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Checks if can proceed to next step
    pub fn can_proceed(&self) -> bool {
        match self.current_step {
            ExportWizardStep::Validation => self.validation_passed,
            ExportWizardStep::FileSelection => !self.selected_files.is_empty(),
            ExportWizardStep::Metadata => true,
            ExportWizardStep::Settings => self.output_path.is_some(),
            ExportWizardStep::Exporting => false,
            ExportWizardStep::Complete => false,
        }
    }
}

/// Version increment type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionIncrement {
    /// Increment major version (1.0.0 -> 2.0.0)
    Major,
    /// Increment minor version (1.0.0 -> 1.1.0)
    Minor,
    /// Increment patch version (1.0.0 -> 1.0.1)
    Patch,
}

/// Parses a semantic version string into components
///
/// # Arguments
///
/// * `version` - Version string in format "major.minor.patch"
///
/// # Returns
///
/// Returns `Some((major, minor, patch))` if valid, `None` otherwise
pub fn parse_version(version: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return None;
    }

    let major = parts[0].parse::<u32>().ok()?;
    let minor = parts[1].parse::<u32>().ok()?;
    let patch = parts[2].parse::<u32>().ok()?;

    Some((major, minor, patch))
}

/// Increments a semantic version string
///
/// # Arguments
///
/// * `version` - Current version string
/// * `increment` - Type of increment to apply
///
/// # Returns
///
/// Returns incremented version string or original if parsing fails
///
/// # Examples
///
/// ```
/// use antares::sdk::campaign_builder::packager::{increment_version, VersionIncrement};
///
/// assert_eq!(increment_version("1.0.0", VersionIncrement::Patch), "1.0.1");
/// assert_eq!(increment_version("1.0.0", VersionIncrement::Minor), "1.1.0");
/// assert_eq!(increment_version("1.0.0", VersionIncrement::Major), "2.0.0");
/// ```
pub fn increment_version(version: &str, increment: VersionIncrement) -> String {
    if let Some((major, minor, patch)) = parse_version(version) {
        match increment {
            VersionIncrement::Major => format!("{}.0.0", major + 1),
            VersionIncrement::Minor => format!("{}.{}.0", major, minor + 1),
            VersionIncrement::Patch => format!("{}.{}.{}", major, minor, patch + 1),
        }
    } else {
        version.to_string()
    }
}

impl CampaignBuilderApp {
    /// Exports the current campaign as a distributable package
    ///
    /// # Arguments
    ///
    /// * `output_path` - Path where the .zip file will be created
    /// * `compression_level` - Compression level (0-9)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if successful, or an error
    ///
    /// # Errors
    ///
    /// Returns `CampaignError::NoPath` if campaign_dir is not set
    /// Returns `CampaignError::Io` for filesystem errors
    pub fn export_campaign(
        &mut self,
        output_path: PathBuf,
        compression_level: u8,
    ) -> Result<(), CampaignError> {
        // Ensure campaign is saved
        self.save_campaign()?;

        // Get campaign directory
        let campaign_dir = self
            .campaign_dir
            .as_ref()
            .ok_or(CampaignError::NoPath)?
            .clone();

        // Create packager with specified compression
        let packager = CampaignPackager::with_compression(compression_level as u32);

        // Package the campaign
        packager
            .package_campaign(&campaign_dir, &output_path)
            .map_err(|e| match e {
                PackageError::IoError(io_err) => CampaignError::Io(io_err),
                PackageError::CampaignNotFound(path) => CampaignError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Campaign not found: {}", path),
                )),
                PackageError::ArchiveError(msg) => {
                    CampaignError::Io(std::io::Error::other(format!("Archive error: {}", msg)))
                }
                _ => CampaignError::Io(std::io::Error::other("Package error")),
            })?;

        self.status_message = format!("Campaign exported to {:?}", output_path);
        Ok(())
    }

    /// Imports a campaign from a .zip package
    ///
    /// # Arguments
    ///
    /// * `package_path` - Path to the .zip file
    /// * `install_dir` - Directory where campaign will be installed
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if successful, or an error
    ///
    /// # Errors
    ///
    /// Returns `CampaignError::Io` for filesystem or archive errors
    pub fn import_campaign(
        &mut self,
        package_path: PathBuf,
        install_dir: PathBuf,
    ) -> Result<(), CampaignError> {
        let packager = CampaignPackager::new();

        packager
            .install_package(&package_path, &install_dir)
            .map_err(|e| match e {
                PackageError::IoError(io_err) => CampaignError::Io(io_err),
                PackageError::PackageNotFound(path) => CampaignError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Package not found: {}", path),
                )),
                PackageError::ArchiveError(msg) => {
                    CampaignError::Io(std::io::Error::other(format!("Archive error: {}", msg)))
                }
                _ => CampaignError::Io(std::io::Error::other("Import error")),
            })?;

        self.status_message = format!(
            "Campaign imported from {:?} to {:?}",
            package_path, install_dir
        );
        Ok(())
    }

    /// Validates if campaign is ready for export
    ///
    /// # Returns
    ///
    /// Returns `true` if campaign can be exported, `false` otherwise
    pub fn can_export_campaign(&self) -> bool {
        // Must have a campaign directory
        if self.campaign_dir.is_none() {
            return false;
        }

        // Must have no critical validation errors
        let has_critical_errors = self
            .validation_errors
            .iter()
            .any(|e| matches!(e.severity, crate::Severity::Error));

        !has_critical_errors
    }

    /// Gets a list of files to include in the package
    ///
    /// # Returns
    ///
    /// Returns a vector of relative file paths within the campaign directory
    pub fn get_package_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();

        // Core data files
        files.push(PathBuf::from("campaign.ron"));
        files.push(PathBuf::from(&self.campaign.items_file));
        files.push(PathBuf::from(&self.campaign.spells_file));
        files.push(PathBuf::from(&self.campaign.monsters_file));
        files.push(PathBuf::from(&self.campaign.classes_file));
        files.push(PathBuf::from(&self.campaign.races_file));
        files.push(PathBuf::from(&self.campaign.quests_file));
        files.push(PathBuf::from(&self.campaign.dialogue_file));

        // Maps directory (all .ron files)
        if let Some(ref campaign_dir) = self.campaign_dir {
            let maps_dir = campaign_dir.join(&self.campaign.maps_dir);
            if maps_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&maps_dir) {
                    for entry in entries.flatten() {
                        if let Some(ext) = entry.path().extension() {
                            if ext == "ron" {
                                if let Some(name) = entry.path().file_name() {
                                    files.push(PathBuf::from(&self.campaign.maps_dir).join(name));
                                }
                            }
                        }
                    }
                }
            }
        }

        // README if exists
        if let Some(ref campaign_dir) = self.campaign_dir {
            let readme = campaign_dir.join("README.md");
            if readme.exists() {
                files.push(PathBuf::from("README.md"));
            }
        }

        files
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_wizard_creation() {
        let wizard = ExportWizard::new();
        assert_eq!(wizard.current_step, ExportWizardStep::Validation);
        assert!(!wizard.validation_passed);
        assert_eq!(wizard.compression_level, 6);
    }

    #[test]
    fn test_export_wizard_step_progression() {
        let mut wizard = ExportWizard::new();

        wizard.next_step();
        assert_eq!(wizard.current_step, ExportWizardStep::FileSelection);

        wizard.next_step();
        assert_eq!(wizard.current_step, ExportWizardStep::Metadata);

        wizard.next_step();
        assert_eq!(wizard.current_step, ExportWizardStep::Settings);

        wizard.previous_step();
        assert_eq!(wizard.current_step, ExportWizardStep::Metadata);
    }

    #[test]
    fn test_export_wizard_can_proceed() {
        let mut wizard = ExportWizard::new();
        assert!(!wizard.can_proceed()); // No validation yet

        wizard.validation_passed = true;
        assert!(wizard.can_proceed());

        wizard.next_step(); // FileSelection
        assert!(!wizard.can_proceed()); // No files selected

        wizard.selected_files.push(PathBuf::from("test.ron"));
        assert!(wizard.can_proceed());
    }

    #[test]
    fn test_export_wizard_reset() {
        let mut wizard = ExportWizard::new();
        wizard.next_step();
        wizard.validation_passed = true;
        wizard.selected_files.push(PathBuf::from("test.ron"));

        wizard.reset();
        assert_eq!(wizard.current_step, ExportWizardStep::Validation);
        assert!(!wizard.validation_passed);
        assert!(wizard.selected_files.is_empty());
    }

    #[test]
    fn test_parse_version_valid() {
        assert_eq!(parse_version("1.0.0"), Some((1, 0, 0)));
        assert_eq!(parse_version("2.5.3"), Some((2, 5, 3)));
        assert_eq!(parse_version("10.20.30"), Some((10, 20, 30)));
    }

    #[test]
    fn test_parse_version_invalid() {
        assert_eq!(parse_version("1.0"), None);
        assert_eq!(parse_version("1.0.0.0"), None);
        assert_eq!(parse_version("a.b.c"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn test_increment_version_patch() {
        assert_eq!(increment_version("1.0.0", VersionIncrement::Patch), "1.0.1");
        assert_eq!(increment_version("1.2.3", VersionIncrement::Patch), "1.2.4");
    }

    #[test]
    fn test_increment_version_minor() {
        assert_eq!(increment_version("1.0.0", VersionIncrement::Minor), "1.1.0");
        assert_eq!(increment_version("1.2.3", VersionIncrement::Minor), "1.3.0");
    }

    #[test]
    fn test_increment_version_major() {
        assert_eq!(increment_version("1.0.0", VersionIncrement::Major), "2.0.0");
        assert_eq!(increment_version("1.2.3", VersionIncrement::Major), "2.0.0");
    }

    #[test]
    fn test_increment_version_invalid() {
        // Should return original string if parsing fails
        assert_eq!(
            increment_version("invalid", VersionIncrement::Patch),
            "invalid"
        );
        assert_eq!(increment_version("1.0", VersionIncrement::Minor), "1.0");
    }

    #[test]
    fn test_version_increment_enum() {
        assert_eq!(VersionIncrement::Major, VersionIncrement::Major);
        assert_ne!(VersionIncrement::Major, VersionIncrement::Minor);
    }

    #[test]
    fn test_export_wizard_step_equality() {
        assert_eq!(ExportWizardStep::Validation, ExportWizardStep::Validation);
        assert_ne!(
            ExportWizardStep::Validation,
            ExportWizardStep::FileSelection
        );
    }

    #[test]
    fn test_export_wizard_compression_level() {
        let mut wizard = ExportWizard::new();
        assert_eq!(wizard.compression_level, 6);

        wizard.compression_level = 9;
        assert_eq!(wizard.compression_level, 9);
    }

    #[test]
    fn test_export_wizard_flags() {
        let mut wizard = ExportWizard::new();
        assert!(wizard.include_readme);
        assert!(wizard.include_all_maps);

        wizard.include_readme = false;
        wizard.include_all_maps = false;

        assert!(!wizard.include_readme);
        assert!(!wizard.include_all_maps);
    }

    #[test]
    fn test_export_wizard_output_path() {
        let mut wizard = ExportWizard::new();
        assert_eq!(wizard.output_path, None);

        wizard.output_path = Some(PathBuf::from("/tmp/test.zip"));
        assert!(wizard.output_path.is_some());
    }

    #[test]
    fn test_export_wizard_error_handling() {
        let mut wizard = ExportWizard::new();
        assert_eq!(wizard.export_error, None);

        wizard.export_error = Some("Test error".to_string());
        assert!(wizard.export_error.is_some());
    }
}
