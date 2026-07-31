// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! End-to-end integration tests for the Export / Package Campaign wizard.
//!
//! These tests drive the [`ExportWizard`] state machine headlessly (no egui)
//! against the stable `data/test_campaign` fixture (per Implementation Rule 5;
//! never `campaigns/tutorial`). They exercise the full step progression and the
//! real packaging engine (`antares::sdk::campaign_packager::CampaignPackager`)
//! via [`ExportWizard::run_export`].

use std::path::PathBuf;

use campaign_builder::packager::{
    increment_version, ExportWizard, ExportWizardStep, VersionIncrement,
};

/// Returns the path to the stable `data/test_campaign` fixture directory.
fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/test_campaign")
}

#[test]
fn test_export_wizard_full_flow_end_to_end() {
    let fixture = fixture_dir();
    assert!(
        fixture.join("campaign.ron").exists(),
        "test fixture campaign.ron must exist at {}",
        fixture.display()
    );

    // Step 1: Validation.
    let mut wizard = ExportWizard::new();
    assert_eq!(wizard.current_step, ExportWizardStep::Validation);
    assert!(!wizard.can_proceed());

    wizard.validation_passed = true;
    assert!(wizard.can_proceed());
    wizard.next_step();
    assert_eq!(wizard.current_step, ExportWizardStep::FileSelection);

    // Step 2: File selection — populate from the campaign directory.
    wizard.populate_files_from_campaign(&fixture);
    assert!(
        !wizard.selected_files.is_empty(),
        "populate_files_from_campaign should list top-level entries"
    );
    assert!(wizard.can_proceed());
    wizard.next_step();
    assert_eq!(wizard.current_step, ExportWizardStep::Metadata);

    // Step 3: Metadata (always proceedable) -> Step 4: Settings.
    assert!(wizard.can_proceed());
    wizard.next_step();
    assert_eq!(wizard.current_step, ExportWizardStep::Settings);

    // Step 4: Settings — an output path is required to proceed.
    assert!(!wizard.can_proceed());
    let temp = tempfile::TempDir::new().expect("create temp dir");
    let output = temp.path().join("test_campaign_export.tar.gz");
    wizard.output_path = Some(output.clone());
    wizard.compression_level = 4;
    assert!(wizard.can_proceed());

    // Step 5: advance into the Exporting step, where the real export runs.
    wizard.next_step();
    assert_eq!(wizard.current_step, ExportWizardStep::Exporting);

    // Run the real export.
    let manifest = wizard
        .run_export(&fixture)
        .expect("run_export should succeed against the fixture campaign");

    assert!(manifest.total_size > 0, "manifest total_size should be > 0");
    assert!(
        !manifest.files.is_empty(),
        "manifest should include packaged files"
    );
    assert!(
        output.exists(),
        "the .tar.gz output file should exist on disk"
    );
    assert!(wizard.export_complete);
    assert!(wizard.export_error.is_none());

    // Advance to Complete after a successful export.
    wizard.next_step();
    assert_eq!(wizard.current_step, ExportWizardStep::Complete);
}

#[test]
fn test_run_export_without_output_path_returns_err() {
    let mut wizard = ExportWizard::new();
    assert!(wizard.output_path.is_none());

    let result = wizard.run_export(&fixture_dir());
    assert!(
        result.is_err(),
        "run_export must fail without an output path"
    );
    assert!(
        wizard.export_error.is_some(),
        "export_error should be populated on failure"
    );
    assert!(!wizard.export_complete);
}

#[test]
fn test_run_export_invalid_campaign_dir_returns_err() {
    let mut wizard = ExportWizard::new();
    let temp = tempfile::TempDir::new().expect("create temp dir");
    wizard.output_path = Some(temp.path().join("out.tar.gz"));

    // A directory that does not contain a valid campaign.
    let bogus = temp.path().join("does_not_exist");
    let result = wizard.run_export(&bogus);
    assert!(
        result.is_err(),
        "run_export must fail for a missing campaign dir"
    );
    assert!(wizard.export_error.is_some());
    assert!(!wizard.export_complete);
}

#[test]
fn test_populate_files_from_campaign_is_idempotent() {
    let fixture = fixture_dir();
    let mut wizard = ExportWizard::new();

    wizard.populate_files_from_campaign(&fixture);
    let first = wizard.selected_files.clone();
    assert!(!first.is_empty());

    // Calling again should not accumulate duplicates.
    wizard.populate_files_from_campaign(&fixture);
    assert_eq!(
        wizard.selected_files, first,
        "repeated population should be idempotent"
    );

    // The fixture's campaign.ron should be among the top-level entries.
    assert!(
        wizard
            .selected_files
            .iter()
            .any(|p| p.file_name().and_then(|n| n.to_str()) == Some("campaign.ron")),
        "campaign.ron should be listed among top-level entries"
    );
}

#[test]
fn test_populate_files_from_missing_dir_leaves_empty() {
    let mut wizard = ExportWizard::new();
    wizard.populate_files_from_campaign(&PathBuf::from("/nonexistent/campaign/path"));
    assert!(
        wizard.selected_files.is_empty(),
        "a missing directory should leave the selection empty"
    );
}

#[test]
fn test_increment_version_integration_with_wizard_flow() {
    // The Metadata step offers version bumps via increment_version; verify the
    // semantics the UI relies on.
    assert_eq!(increment_version("1.2.3", VersionIncrement::Patch), "1.2.4");
    assert_eq!(increment_version("1.2.3", VersionIncrement::Minor), "1.3.0");
    assert_eq!(increment_version("1.2.3", VersionIncrement::Major), "2.0.0");
}
