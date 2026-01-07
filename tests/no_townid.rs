// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration test to ensure no `TownId` references remain in `src/`.
//!
//! This test walks the `src/` directory and fails if it finds any literal
//! occurrences of the token `TownId`. It's intended to prevent accidental
//! re-introduction of the deprecated numeric town/inn ID type while the
//! project migrates to string-based `InnkeeperId`(s).

use std::path::Path;

#[test]
fn test_no_townid_references_in_src() -> Result<(), Box<dyn std::error::Error>> {
    // Start from the crate manifest directory to make the test robust to
    // where it's executed from when run as an integration test.
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src_dir = manifest_dir.join("src");

    assert!(
        src_dir.exists(),
        "Expected `src/` directory to exist at {}",
        src_dir.display()
    );

    let mut occurrences: Vec<String> = Vec::new();

    fn visit_dir(
        dir: &Path,
        occurrences: &mut Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                visit_dir(&path, occurrences)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                let content = std::fs::read_to_string(&path)?;
                for (idx, line) in content.lines().enumerate() {
                    if line.contains("TownId") {
                        occurrences.push(format!(
                            "{}:{}: {}",
                            path.display(),
                            idx + 1,
                            line.trim()
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    visit_dir(&src_dir, &mut occurrences)?;

    if !occurrences.is_empty() {
        panic!(
            "Found `TownId` references in source files (should be migrated to `InnkeeperId`):\n{}",
            occurrences.join("\n")
        );
    }

    Ok(())
}
