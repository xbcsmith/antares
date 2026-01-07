// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Test Utilities Module
//!
//! This module provides pattern matching helpers and source code scanning utilities
//! for testing the campaign builder's code quality and pattern compliance.
//!
//! # Features
//!
//! - **Source Code Scanning**: Read and analyze Rust source files
//! - **Pattern Matching**: Regex-based pattern detection for code analysis
//! - **Compliance Testing**: Verify editors follow standard patterns
//! - **ComboBox ID Verification**: Ensure proper ID salt usage to prevent conflicts
//!
//! # Example
//!
//! ```no_run
//! use campaign_builder::test_utils::{
//!     scan_source_files,
//!     PatternMatcher,
//!     collect_combobox_id_salts,
//! };
//!
//! // Scan for ComboBox usage and collect ID salts
//! let matcher = PatternMatcher::combobox_id_salt();
//! let files = scan_source_files("src/");
//! for file in files {
//!     let matches = matcher.find_matches(&file.content);
//!     let salts = collect_combobox_id_salts(&file);
//!     // Process matches and salts...
//! }
//! ```

use regex::Regex;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

// =============================================================================
// Source File Types
// =============================================================================

/// Represents a source file with its path and content.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// Path to the source file
    pub path: PathBuf,
    /// Content of the source file
    pub content: String,
    /// File name without extension
    pub name: String,
}

impl SourceFile {
    /// Creates a new SourceFile from a path and content.
    pub fn new(path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        let path = path.into();
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        Self {
            path,
            content: content.into(),
            name,
        }
    }

    /// Returns the number of lines in the file.
    pub fn line_count(&self) -> usize {
        self.content.lines().count()
    }

    /// Returns true if the file contains the given pattern (case-insensitive).
    pub fn contains_pattern(&self, pattern: &str) -> bool {
        self.content
            .to_lowercase()
            .contains(&pattern.to_lowercase())
    }
}

/// Result of a pattern match within a source file.
#[derive(Debug, Clone)]
pub struct PatternMatch {
    /// The line number where the match was found (1-based)
    pub line_number: usize,
    /// The matched text
    pub matched_text: String,
    /// The full line containing the match
    pub line_content: String,
    /// Capture groups from the regex, if any
    pub captures: Vec<String>,
}

// =============================================================================
// Pattern Matcher
// =============================================================================

/// A reusable pattern matcher for source code analysis.
///
/// PatternMatcher wraps a regex pattern and provides convenience methods
/// for finding and analyzing matches in source code.
#[derive(Debug, Clone)]
pub struct PatternMatcher {
    /// The compiled regex pattern
    pattern: Regex,
    /// Human-readable description of what this pattern matches
    pub description: String,
}

impl PatternMatcher {
    /// Creates a new PatternMatcher from a regex pattern string.
    ///
    /// # Panics
    ///
    /// Panics if the regex pattern is invalid.
    pub fn new(pattern: &str, description: &str) -> Self {
        Self {
            pattern: Regex::new(pattern).expect("Invalid regex pattern"),
            description: description.to_string(),
        }
    }

    /// Creates a matcher for ComboBox::from_id_salt usage.
    ///
    /// This pattern matches proper ComboBox instantiation that uses
    /// id_salt for unique identification.
    pub fn combobox_id_salt() -> Self {
        Self::new(
            r#"ComboBox::from_id_salt\s*\(\s*["']([^"']+)["']\s*\)"#,
            "ComboBox using from_id_salt pattern",
        )
    }

    /// Creates a matcher for improper ComboBox::from_label usage.
    ///
    /// This pattern detects ComboBox usage that may cause ID conflicts.
    pub fn combobox_from_label() -> Self {
        Self::new(
            r"ComboBox::from_label\s*\(",
            "ComboBox using from_label (potential ID conflict)",
        )
    }

    /// Creates a matcher for pub fn show method signatures.
    pub fn pub_fn_show() -> Self {
        Self::new(r"pub\s+fn\s+show\s*\(", "Public show function signature")
    }

    /// Creates a matcher for pub fn new method signatures.
    pub fn pub_fn_new() -> Self {
        Self::new(r"pub\s+fn\s+new\s*\(", "Public new function signature")
    }

    /// Creates a matcher for editor state struct definitions.
    pub fn editor_state_struct() -> Self {
        Self::new(
            r"pub\s+struct\s+(\w+EditorState)\s*\{",
            "Editor state struct definition",
        )
    }

    /// Creates a matcher for toolbar button patterns.
    pub fn toolbar_button() -> Self {
        Self::new(
            r#"ui\.button\s*\(\s*["']([➕✏️🗑️📥📤💾🔄][^"']*)["']\s*\)"#,
            "Toolbar button with emoji",
        )
    }

    /// Creates a matcher for EditorToolbar usage.
    pub fn editor_toolbar_usage() -> Self {
        Self::new(r"EditorToolbar::new\s*\(", "EditorToolbar component usage")
    }

    /// Creates a matcher for ActionButtons usage.
    pub fn action_buttons_usage() -> Self {
        Self::new(r"ActionButtons::new\s*\(", "ActionButtons component usage")
    }

    /// Creates a matcher for TwoColumnLayout usage.
    pub fn two_column_layout_usage() -> Self {
        Self::new(
            r"TwoColumnLayout::new\s*\(",
            "TwoColumnLayout component usage",
        )
    }

    /// Creates a matcher for #[test] annotations.
    pub fn test_annotation() -> Self {
        Self::new(r"#\[test\]", "Test annotation")
    }

    /// Creates a matcher for #[cfg(test)] modules.
    pub fn test_module() -> Self {
        Self::new(r"#\[cfg\(test\)\]", "Test configuration module")
    }

    /// Finds all matches of the pattern in the given content.
    pub fn find_matches(&self, content: &str) -> Vec<PatternMatch> {
        let mut matches = Vec::new();
        for (line_idx, line) in content.lines().enumerate() {
            for cap in self.pattern.captures_iter(line) {
                let matched_text = cap
                    .get(0)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                let captures: Vec<String> = cap
                    .iter()
                    .skip(1) // Skip the full match
                    .filter_map(|m| m.map(|m| m.as_str().to_string()))
                    .collect();
                matches.push(PatternMatch {
                    line_number: line_idx + 1,
                    matched_text,
                    line_content: line.to_string(),
                    captures,
                });
            }
        }
        matches
    }

    /// Returns true if the pattern matches anywhere in the content.
    pub fn matches(&self, content: &str) -> bool {
        self.pattern.is_match(content)
    }

    /// Counts the number of matches in the content.
    pub fn count_matches(&self, content: &str) -> usize {
        content
            .lines()
            .map(|line| self.pattern.find_iter(line).count())
            .sum()
    }
}

// =============================================================================
// Source Scanning Functions
// =============================================================================

/// Scans all Rust source files in the given directory.
///
/// This function reads all `.rs` files in the specified directory and returns
/// them as `SourceFile` instances. It does not recurse into subdirectories.
///
/// # Example
///
/// ```no_run
/// use campaign_builder::test_utils::scan_source_files;
///
/// let files = scan_source_files("sdk/campaign_builder/src");
/// for file in files {
///     println!("Found: {}", file.name);
/// }
/// ```
pub fn scan_source_files(dir: impl AsRef<Path>) -> Vec<SourceFile> {
    let dir = dir.as_ref();
    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "rs") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    files.push(SourceFile::new(path, content));
                }
            }
        }
    }

    files.sort_by(|a, b| a.name.cmp(&b.name));
    files
}

/// Scans for a specific file by name in the given directory.
pub fn find_source_file(dir: impl AsRef<Path>, filename: &str) -> Option<SourceFile> {
    scan_source_files(dir)
        .into_iter()
        .find(|f| f.name == filename || f.path.file_name().is_some_and(|n| n == filename))
}

// =============================================================================
// Compliance Checking
// =============================================================================

/// Result of a compliance check on an editor file.
#[derive(Debug, Clone)]
pub struct EditorComplianceResult {
    /// Name of the editor file
    pub editor_name: String,
    /// Whether the editor has a pub fn show method
    pub has_show_method: bool,
    /// Whether the editor has a pub fn new method
    pub has_new_method: bool,
    /// Whether the editor has an EditorState struct
    pub has_state_struct: bool,
    /// Whether the editor uses EditorToolbar
    pub uses_toolbar: bool,
    /// Whether the editor uses ActionButtons
    pub uses_action_buttons: bool,
    /// Whether the editor uses TwoColumnLayout
    pub uses_two_column_layout: bool,
    /// Whether the editor has tests
    pub has_tests: bool,
    /// ComboBox from_id_salt usage count
    pub combobox_id_salt_count: usize,
    /// ComboBox from_label usage count (should be 0)
    pub combobox_from_label_count: usize,
    /// List of issues found
    pub issues: Vec<String>,
}

impl EditorComplianceResult {
    /// Returns true if the editor passes all compliance checks.
    pub fn is_compliant(&self) -> bool {
        self.issues.is_empty() && self.combobox_from_label_count == 0
    }

    /// Returns a score representing the compliance level (0-100).
    pub fn compliance_score(&self) -> u32 {
        let mut score = 0u32;

        if self.has_show_method {
            score += 20;
        }
        if self.has_new_method {
            score += 10;
        }
        if self.has_state_struct {
            score += 15;
        }
        if self.uses_toolbar {
            score += 15;
        }
        if self.uses_action_buttons {
            score += 10;
        }
        if self.uses_two_column_layout {
            score += 10;
        }
        if self.has_tests {
            score += 10;
        }
        if self.combobox_from_label_count == 0 {
            score += 10;
        }

        score.min(100)
    }
}

/// Checks an editor file for pattern compliance.
pub fn check_editor_compliance(file: &SourceFile) -> EditorComplianceResult {
    let content = &file.content;
    let mut issues = Vec::new();

    // Pattern matchers
    let show_matcher = PatternMatcher::pub_fn_show();
    let new_matcher = PatternMatcher::pub_fn_new();
    let state_matcher = PatternMatcher::editor_state_struct();
    let toolbar_matcher = PatternMatcher::editor_toolbar_usage();
    let action_matcher = PatternMatcher::action_buttons_usage();
    let layout_matcher = PatternMatcher::two_column_layout_usage();
    let test_matcher = PatternMatcher::test_annotation();
    let id_salt_matcher = PatternMatcher::combobox_id_salt();
    let from_label_matcher = PatternMatcher::combobox_from_label();

    let has_show = show_matcher.matches(content);
    let has_new = new_matcher.matches(content);
    let has_state = state_matcher.matches(content);
    let uses_toolbar = toolbar_matcher.matches(content);
    let uses_action = action_matcher.matches(content);
    let uses_layout = layout_matcher.matches(content);
    let has_tests = test_matcher.matches(content);
    let id_salt_count = id_salt_matcher.count_matches(content);
    let from_label_count = from_label_matcher.count_matches(content);

    // Check for issues
    if !has_show && file.name.ends_with("_editor") {
        issues.push(format!("{}: Missing pub fn show method", file.name));
    }

    if !has_new && file.name.ends_with("_editor") {
        issues.push(format!("{}: Missing pub fn new method", file.name));
    }

    if from_label_count > 0 {
        issues.push(format!(
            "{}: Uses ComboBox::from_label ({} occurrences) - should use from_id_salt",
            file.name, from_label_count
        ));
    }

    if !has_tests && file.name.ends_with("_editor") {
        issues.push(format!("{}: Missing test coverage", file.name));
    }

    EditorComplianceResult {
        editor_name: file.name.clone(),
        has_show_method: has_show,
        has_new_method: has_new,
        has_state_struct: has_state,
        uses_toolbar,
        uses_action_buttons: uses_action,
        uses_two_column_layout: uses_layout,
        has_tests,
        combobox_id_salt_count: id_salt_count,
        combobox_from_label_count: from_label_count,
        issues,
    }
}

/// Checks all editor files in a directory for compliance.
pub fn check_all_editors_compliance(
    dir: impl AsRef<Path>,
) -> HashMap<String, EditorComplianceResult> {
    let files = scan_source_files(dir);
    let mut results = HashMap::new();

    for file in files {
        // Only check files that are editors (have "editor" in the name)
        if file.name.contains("editor") {
            let result = check_editor_compliance(&file);
            results.insert(file.name.clone(), result);
        }
    }

    results
}

// =============================================================================
// ComboBox ID Verification
// =============================================================================

/// Verifies that all ComboBox usages in a file use from_id_salt.
///
/// Returns a list of violations (files/lines using from_label instead).
pub fn verify_combobox_id_salt_usage(file: &SourceFile) -> Vec<PatternMatch> {
    let from_label_matcher = PatternMatcher::combobox_from_label();
    from_label_matcher.find_matches(&file.content)
}

/// Collects all ComboBox ID salts used in a file.
///
/// This is useful for detecting potential ID conflicts where the same
/// salt is used in multiple places.
pub fn collect_combobox_id_salts(file: &SourceFile) -> Vec<String> {
    let id_salt_matcher = PatternMatcher::combobox_id_salt();
    id_salt_matcher
        .find_matches(&file.content)
        .into_iter()
        .filter_map(|m| m.captures.first().cloned())
        .collect()
}

/// Checks for duplicate ComboBox ID salts across all files.
pub fn find_duplicate_combobox_ids(files: &[SourceFile]) -> HashMap<String, Vec<(String, usize)>> {
    let mut all_ids: HashMap<String, Vec<(String, usize)>> = HashMap::new();

    for file in files {
        let matcher = PatternMatcher::combobox_id_salt();
        for m in matcher.find_matches(&file.content) {
            if let Some(id) = m.captures.first() {
                all_ids
                    .entry(id.clone())
                    .or_default()
                    .push((file.name.clone(), m.line_number));
            }
        }
    }

    // Keep only duplicates
    all_ids.retain(|_, locations| locations.len() > 1);
    all_ids
}

// =============================================================================
// Test Assertion Helpers
// =============================================================================

/// Asserts that a pattern exists in the source file.
///
/// # Panics
///
/// Panics if the pattern is not found.
pub fn assert_pattern_exists(file: &SourceFile, matcher: &PatternMatcher) {
    assert!(
        matcher.matches(&file.content),
        "Pattern '{}' not found in {}\nDescription: {}",
        matcher.pattern,
        file.name,
        matcher.description
    );
}

/// Asserts that a pattern does not exist in the source file.
///
/// # Panics
///
/// Panics if the pattern is found.
pub fn assert_pattern_absent(file: &SourceFile, matcher: &PatternMatcher) {
    let matches = matcher.find_matches(&file.content);
    assert!(
        matches.is_empty(),
        "Pattern '{}' should not exist in {} but found {} occurrences:\n{}\nDescription: {}",
        matcher.pattern,
        file.name,
        matches.len(),
        matches
            .iter()
            .map(|m| format!("  Line {}: {}", m.line_number, m.line_content.trim()))
            .collect::<Vec<_>>()
            .join("\n"),
        matcher.description
    );
}

/// Asserts that a file passes all compliance checks.
///
/// # Panics
///
/// Panics if the file has compliance issues.
pub fn assert_editor_compliant(file: &SourceFile) {
    let result = check_editor_compliance(file);
    assert!(
        result.is_compliant(),
        "Editor '{}' is not compliant:\n{}",
        file.name,
        result.issues.join("\n")
    );
}

/// Asserts that no ComboBox uses from_label.
///
/// # Panics
///
/// Panics if any from_label usage is found.
pub fn assert_no_combobox_from_label(files: &[SourceFile]) {
    let matcher = PatternMatcher::combobox_from_label();
    let mut violations = Vec::new();

    for file in files {
        let matches = matcher.find_matches(&file.content);
        for m in matches {
            violations.push(format!(
                "{}:{}: {}",
                file.name,
                m.line_number,
                m.line_content.trim()
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ComboBox::from_label usage found (should use from_id_salt):\n{}",
        violations.join("\n")
    );
}

// =============================================================================
// Test Summary Generation
// =============================================================================

/// Summary of compliance checking across all editors.
#[derive(Debug, Clone, Default)]
pub struct ComplianceSummary {
    /// Total number of editors checked
    pub total_editors: usize,
    /// Number of fully compliant editors
    pub compliant_editors: usize,
    /// Total issues found
    pub total_issues: usize,
    /// Total ComboBox from_label violations
    pub from_label_violations: usize,
    /// Average compliance score
    pub average_score: f32,
    /// List of all issues
    pub all_issues: Vec<String>,
}

impl ComplianceSummary {
    /// Creates a summary from compliance results.
    pub fn from_results(results: &HashMap<String, EditorComplianceResult>) -> Self {
        let total_editors = results.len();
        let compliant_editors = results.values().filter(|r| r.is_compliant()).count();
        let total_issues: usize = results.values().map(|r| r.issues.len()).sum();
        let from_label_violations: usize =
            results.values().map(|r| r.combobox_from_label_count).sum();
        let average_score = if total_editors > 0 {
            results
                .values()
                .map(|r| r.compliance_score() as f32)
                .sum::<f32>()
                / total_editors as f32
        } else {
            0.0
        };
        let all_issues: Vec<String> = results.values().flat_map(|r| r.issues.clone()).collect();

        Self {
            total_editors,
            compliant_editors,
            total_issues,
            from_label_violations,
            average_score,
            all_issues,
        }
    }
}

impl fmt::Display for ComplianceSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Compliance Summary:\n\
             - Total Editors: {}\n\
             - Compliant: {} ({:.1}%)\n\
             - Total Issues: {}\n\
             - from_label Violations: {}\n\
             - Average Score: {:.1}/100",
            self.total_editors,
            self.compliant_editors,
            if self.total_editors > 0 {
                (self.compliant_editors as f32 / self.total_editors as f32) * 100.0
            } else {
                0.0
            },
            self.total_issues,
            self.from_label_violations,
            self.average_score
        )
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // SourceFile Tests
    // =========================================================================

    #[test]
    fn test_source_file_new() {
        let file = SourceFile::new("test.rs", "fn main() {}");
        assert_eq!(file.name, "test");
        assert_eq!(file.content, "fn main() {}");
    }

    #[test]
    fn test_source_file_line_count() {
        let file = SourceFile::new("test.rs", "line1\nline2\nline3");
        assert_eq!(file.line_count(), 3);
    }

    #[test]
    fn test_source_file_contains_pattern() {
        let file = SourceFile::new("test.rs", "fn main() { ComboBox::from_id_salt(\"test\") }");
        assert!(file.contains_pattern("combobox"));
        assert!(file.contains_pattern("COMBOBOX")); // case insensitive
        assert!(!file.contains_pattern("nonexistent"));
    }

    // =========================================================================
    // PatternMatcher Tests
    // =========================================================================

    #[test]
    fn test_pattern_matcher_combobox_id_salt() {
        let matcher = PatternMatcher::combobox_id_salt();
        let content = r#"egui::ComboBox::from_id_salt("my_combo")"#;
        assert!(matcher.matches(content));

        let matches = matcher.find_matches(content);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].captures[0], "my_combo");
    }

    #[test]
    fn test_pattern_matcher_combobox_from_label() {
        let matcher = PatternMatcher::combobox_from_label();
        let content = r#"ComboBox::from_label("bad")"#;
        assert!(matcher.matches(content));
    }

    #[test]
    fn test_pattern_matcher_pub_fn_show() {
        let matcher = PatternMatcher::pub_fn_show();
        assert!(matcher.matches("pub fn show(&mut self, ui: &mut Ui)"));
        assert!(matcher.matches("    pub fn show("));
        assert!(!matcher.matches("fn show("));
        assert!(!matcher.matches("pub fn show_items("));
    }

    #[test]
    fn test_pattern_matcher_pub_fn_new() {
        let matcher = PatternMatcher::pub_fn_new();
        assert!(matcher.matches("pub fn new() -> Self"));
        assert!(!matcher.matches("fn new()"));
    }

    #[test]
    fn test_pattern_matcher_count_matches() {
        let matcher = PatternMatcher::combobox_id_salt();
        let content = r#"
            ComboBox::from_id_salt("first")
            ComboBox::from_id_salt("second")
            ComboBox::from_id_salt("third")
        "#;
        assert_eq!(matcher.count_matches(content), 3);
    }

    #[test]
    fn test_pattern_matcher_editor_state_struct() {
        let matcher = PatternMatcher::editor_state_struct();
        assert!(matcher.matches("pub struct ItemsEditorState {"));
        assert!(!matcher.matches("struct ItemsEditorState {")); // not pub
    }

    // =========================================================================
    // Compliance Tests
    // =========================================================================

    #[test]
    fn test_editor_compliance_result_score() {
        let result = EditorComplianceResult {
            editor_name: "test_editor".to_string(),
            has_show_method: true,
            has_new_method: true,
            has_state_struct: true,
            uses_toolbar: true,
            uses_action_buttons: true,
            uses_two_column_layout: true,
            has_tests: true,
            combobox_id_salt_count: 5,
            combobox_from_label_count: 0,
            issues: vec![],
        };
        assert_eq!(result.compliance_score(), 100);
        assert!(result.is_compliant());
    }

    #[test]
    fn test_editor_compliance_result_partial_score() {
        let result = EditorComplianceResult {
            editor_name: "test_editor".to_string(),
            has_show_method: true,
            has_new_method: false,
            has_state_struct: false,
            uses_toolbar: true,
            uses_action_buttons: false,
            uses_two_column_layout: false,
            has_tests: true,
            combobox_id_salt_count: 2,
            combobox_from_label_count: 0,
            issues: vec![],
        };
        // 20 (show) + 15 (toolbar) + 10 (tests) + 10 (no from_label) = 55
        assert_eq!(result.compliance_score(), 55);
    }

    #[test]
    fn test_check_editor_compliance_basic() {
        let content = r#"
            pub struct ItemsEditorState {
                items: Vec<Item>,
            }

            impl ItemsEditorState {
                pub fn new() -> Self {
                    Self { items: vec![] }
                }

                pub fn show(&mut self, ui: &mut egui::Ui) {
                    EditorToolbar::new("Items").show(ui);
                    ActionButtons::new().show(ui);
                    TwoColumnLayout::new("items").show_split(ui, |left_ui| {}, |right_ui| {});
                }
            }

            #[cfg(test)]
            mod tests {
                #[test]
                fn test_something() {}
            }
        "#;

        let file = SourceFile::new("items_editor.rs", content);
        let result = check_editor_compliance(&file);

        assert!(result.has_show_method);
        assert!(result.has_new_method);
        assert!(result.has_state_struct);
        assert!(result.uses_toolbar);
        assert!(result.uses_action_buttons);
        assert!(result.uses_two_column_layout);
        assert!(result.has_tests);
        assert_eq!(result.combobox_from_label_count, 0);
    }

    #[test]
    fn test_check_editor_compliance_with_violations() {
        let content = r#"
            impl SomeEditor {
                pub fn show(&mut self, ui: &mut egui::Ui) {
                    egui::ComboBox::from_label("Bad")
                        .show(ui);
                }
            }
        "#;

        let file = SourceFile::new("some_editor.rs", content);
        let result = check_editor_compliance(&file);

        assert_eq!(result.combobox_from_label_count, 1);
        assert!(!result.is_compliant());
        assert!(result.issues.iter().any(|i| i.contains("from_label")));
    }

    // =========================================================================
    // ComboBox ID Tests
    // =========================================================================

    #[test]
    fn test_collect_combobox_id_salts() {
        let content = r#"
            ComboBox::from_id_salt("combo_a").show(ui);
            ComboBox::from_id_salt("combo_b").show(ui);
        "#;
        let file = SourceFile::new("test.rs", content);
        let salts = collect_combobox_id_salts(&file);

        assert_eq!(salts.len(), 2);
        assert!(salts.contains(&"combo_a".to_string()));
        assert!(salts.contains(&"combo_b".to_string()));
    }

    #[test]
    fn test_find_duplicate_combobox_ids() {
        let file1 = SourceFile::new("editor1.rs", r#"ComboBox::from_id_salt("shared_id")"#);
        let file2 = SourceFile::new("editor2.rs", r#"ComboBox::from_id_salt("shared_id")"#);
        let file3 = SourceFile::new("editor3.rs", r#"ComboBox::from_id_salt("unique_id")"#);

        let files = vec![file1, file2, file3];
        let duplicates = find_duplicate_combobox_ids(&files);

        assert_eq!(duplicates.len(), 1);
        assert!(duplicates.contains_key("shared_id"));
        assert_eq!(duplicates["shared_id"].len(), 2);
    }

    // =========================================================================
    // Summary Tests
    // =========================================================================

    #[test]
    fn test_compliance_summary_from_results() {
        let mut results = HashMap::new();
        results.insert(
            "editor1".to_string(),
            EditorComplianceResult {
                editor_name: "editor1".to_string(),
                has_show_method: true,
                has_new_method: true,
                has_state_struct: true,
                uses_toolbar: true,
                uses_action_buttons: true,
                uses_two_column_layout: true,
                has_tests: true,
                combobox_id_salt_count: 1,
                combobox_from_label_count: 0,
                issues: vec![],
            },
        );
        results.insert(
            "editor2".to_string(),
            EditorComplianceResult {
                editor_name: "editor2".to_string(),
                has_show_method: true,
                has_new_method: false,
                has_state_struct: false,
                uses_toolbar: false,
                uses_action_buttons: false,
                uses_two_column_layout: false,
                has_tests: false,
                combobox_id_salt_count: 0,
                combobox_from_label_count: 1,
                issues: vec!["Missing tests".to_string()],
            },
        );

        let summary = ComplianceSummary::from_results(&results);

        assert_eq!(summary.total_editors, 2);
        assert_eq!(summary.compliant_editors, 1);
        assert_eq!(summary.total_issues, 1);
        assert_eq!(summary.from_label_violations, 1);
    }

    #[test]
    fn test_compliance_summary_to_string() {
        let summary = ComplianceSummary {
            total_editors: 10,
            compliant_editors: 8,
            total_issues: 3,
            from_label_violations: 1,
            average_score: 85.5,
            all_issues: vec![],
        };

        let output = summary.to_string();
        assert!(output.contains("Total Editors: 10"));
        assert!(output.contains("Compliant: 8"));
        assert!(output.contains("80.0%"));
    }

    // =========================================================================
    // Assertion Helper Tests
    // =========================================================================

    #[test]
    fn test_assert_pattern_exists_success() {
        let file = SourceFile::new("test.rs", "pub fn show(&self) {}");
        let matcher = PatternMatcher::pub_fn_show();
        assert_pattern_exists(&file, &matcher); // Should not panic
    }

    #[test]
    #[should_panic(expected = "not found")]
    fn test_assert_pattern_exists_failure() {
        let file = SourceFile::new("test.rs", "fn private_show(&self) {}");
        let matcher = PatternMatcher::pub_fn_show();
        assert_pattern_exists(&file, &matcher);
    }

    #[test]
    fn test_assert_pattern_absent_success() {
        let file = SourceFile::new("test.rs", "ComboBox::from_id_salt(\"good\")");
        let matcher = PatternMatcher::combobox_from_label();
        assert_pattern_absent(&file, &matcher); // Should not panic
    }

    #[test]
    #[should_panic(expected = "should not exist")]
    fn test_assert_pattern_absent_failure() {
        let file = SourceFile::new("test.rs", "ComboBox::from_label(\"bad\")");
        let matcher = PatternMatcher::combobox_from_label();
        assert_pattern_absent(&file, &matcher);
    }

    #[test]
    fn test_assert_no_combobox_from_label_success() {
        let files = vec![
            SourceFile::new("a.rs", "ComboBox::from_id_salt(\"ok\")"),
            SourceFile::new("b.rs", "no combo boxes here"),
        ];
        assert_no_combobox_from_label(&files); // Should not panic
    }

    #[test]
    #[should_panic(expected = "from_label usage found")]
    fn test_assert_no_combobox_from_label_failure() {
        let files = vec![SourceFile::new("bad.rs", "ComboBox::from_label(\"bad\")")];
        assert_no_combobox_from_label(&files);
    }

    // =========================================================================
    // Editor Pattern Compliance Integration Tests
    // =========================================================================

    /// Creates mock source representing an editor that uses all shared patterns
    fn create_compliant_editor_source(name: &str) -> SourceFile {
        let content = format!(
            r#"
            pub struct {}EditorState {{
                mode: EditorMode,
                search_filter: String,
            }}

            impl {}EditorState {{
                pub fn new() -> Self {{
                    Self::default()
                }}

                pub fn show(&mut self, ui: &mut egui::Ui) {{
                    let toolbar_action = EditorToolbar::new("{}")
                        .with_search(&mut self.search_filter)
                        .show(ui);

                    TwoColumnLayout::new("{}").show_split(ui, |left_ui| {{}}, |right_ui| {{
                        let action = ActionButtons::new().enabled(true).show(right_ui);
                    }});
                }}
            }}

            #[cfg(test)]
            mod tests {{
                #[test]
                fn test_editor_state_new() {{}}
            }}
            "#,
            name,
            name,
            name.to_lowercase(),
            name.to_lowercase()
        );
        SourceFile::new(format!("{}_editor.rs", name.to_lowercase()), &content)
    }

    /// Creates mock source representing an editor missing some patterns
    fn create_partial_editor_source(name: &str) -> SourceFile {
        let content = format!(
            r#"
            pub struct {}EditorState {{
                mode: EditorMode,
            }}

            impl {}EditorState {{
                pub fn new() -> Self {{
                    Self::default()
                }}

                pub fn show(&mut self, ui: &mut egui::Ui) {{
                    // Missing EditorToolbar
                    // Missing TwoColumnLayout
                    // Missing ActionButtons
                }}
            }}
            "#,
            name, name
        );
        SourceFile::new(format!("{}_editor.rs", name.to_lowercase()), &content)
    }

    #[test]
    fn test_compliant_editor_passes_all_checks() {
        let file = create_compliant_editor_source("Items");
        let result = check_editor_compliance(&file);

        assert!(result.has_show_method, "Should have show method");
        assert!(result.has_new_method, "Should have new method");
        assert!(result.has_state_struct, "Should have state struct");
        assert!(result.uses_toolbar, "Should use EditorToolbar");
        assert!(result.uses_action_buttons, "Should use ActionButtons");
        assert!(result.uses_two_column_layout, "Should use TwoColumnLayout");
        assert!(result.has_tests, "Should have tests");
        assert_eq!(result.combobox_from_label_count, 0);
        assert!(
            result.compliance_score() >= 90,
            "Compliant editor should score >= 90"
        );
    }

    #[test]
    fn test_partial_editor_detects_missing_patterns() {
        let file = create_partial_editor_source("Broken");
        let result = check_editor_compliance(&file);

        assert!(result.has_show_method, "Should have show method");
        assert!(result.has_new_method, "Should have new method");
        assert!(result.has_state_struct, "Should have state struct");
        assert!(!result.uses_toolbar, "Should NOT use EditorToolbar");
        assert!(!result.uses_action_buttons, "Should NOT use ActionButtons");
        assert!(
            !result.uses_two_column_layout,
            "Should NOT use TwoColumnLayout"
        );
        assert!(!result.has_tests, "Should NOT have tests");
        assert!(
            result.compliance_score() < 70,
            "Partial editor should score < 70"
        );
    }

    #[test]
    fn test_compliance_summary_with_mixed_editors() {
        let compliant = create_compliant_editor_source("Good");
        let partial = create_partial_editor_source("Partial");

        let mut results = HashMap::new();
        results.insert(
            "good_editor".to_string(),
            check_editor_compliance(&compliant),
        );
        results.insert(
            "partial_editor".to_string(),
            check_editor_compliance(&partial),
        );

        let summary = ComplianceSummary::from_results(&results);

        assert_eq!(summary.total_editors, 2);
        // At least one should be compliant (the good one)
        assert!(summary.compliant_editors >= 1);
    }

    #[test]
    fn test_editor_toolbar_pattern_detection() {
        let with_toolbar = SourceFile::new(
            "test_editor.rs",
            r#"
            let action = EditorToolbar::new("Test")
                .with_search(&mut self.query)
                .show(ui);
            "#,
        );

        let without_toolbar = SourceFile::new(
            "test_editor.rs",
            r#"
            ui.horizontal(|ui| {
                if ui.button("New").clicked() {}
            });
            "#,
        );

        let matcher = PatternMatcher::editor_toolbar_usage();
        assert!(matcher.matches(&with_toolbar.content));
        assert!(!matcher.matches(&without_toolbar.content));
    }

    #[test]
    fn test_action_buttons_pattern_detection() {
        let with_buttons = SourceFile::new(
            "test_editor.rs",
            r#"
            let action = ActionButtons::new()
                .enabled(true)
                .show(ui);
            "#,
        );

        let without_buttons = SourceFile::new(
            "test_editor.rs",
            r#"
            if ui.button("Edit").clicked() {}
            if ui.button("Delete").clicked() {}
            "#,
        );

        let matcher = PatternMatcher::action_buttons_usage();
        assert!(matcher.matches(&with_buttons.content));
        assert!(!matcher.matches(&without_buttons.content));
    }

    #[test]
    fn test_two_column_layout_pattern_detection() {
        let with_layout = SourceFile::new(
            "test_editor.rs",
            r#"
            TwoColumnLayout::new("items").show_split(ui, |left_ui| {}, |right_ui| {});
            "#,
        );

        let without_layout = SourceFile::new(
            "test_editor.rs",
            r#"
            ui.columns(2, |cols| {});
            "#,
        );

        let matcher = PatternMatcher::two_column_layout_usage();
        assert!(matcher.matches(&with_layout.content));
        assert!(!matcher.matches(&without_layout.content));
    }

    #[test]
    fn test_all_standard_editors_have_required_structure() {
        // Test that standard editor naming patterns are detected
        let editor_names = [
            "items",
            "spells",
            "monsters",
            "conditions",
            "quests",
            "classes",
            "dialogues",
            "maps",
        ];

        for name in editor_names {
            let file = create_compliant_editor_source(&capitalize_first(name));
            let result = check_editor_compliance(&file);

            assert!(
                result.has_show_method,
                "{}_editor should have show method",
                name
            );
            assert!(
                result.has_new_method,
                "{}_editor should have new method",
                name
            );
            assert!(
                result.has_state_struct,
                "{}_editor should have state struct",
                name
            );
        }
    }

    /// Helper to capitalize first letter
    fn capitalize_first(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }

    #[test]
    fn test_compliance_score_calculation() {
        // Test score calculation with known values
        let result = EditorComplianceResult {
            editor_name: "test_editor".to_string(),
            has_show_method: true,        // 20 points
            has_new_method: true,         // 10 points
            has_state_struct: true,       // 15 points
            uses_toolbar: true,           // 15 points
            uses_action_buttons: true,    // 10 points
            uses_two_column_layout: true, // 10 points
            has_tests: true,              // 10 points
            combobox_id_salt_count: 5,
            combobox_from_label_count: 0, // 10 points (no violations)
            issues: vec![],
        };

        assert_eq!(result.compliance_score(), 100);
        assert!(result.is_compliant());
    }

    #[test]
    fn test_compliance_score_with_missing_elements() {
        let result = EditorComplianceResult {
            editor_name: "test_editor".to_string(),
            has_show_method: true,         // 20 points
            has_new_method: false,         // 0 points
            has_state_struct: true,        // 15 points
            uses_toolbar: false,           // 0 points
            uses_action_buttons: true,     // 10 points
            uses_two_column_layout: false, // 0 points
            has_tests: false,              // 0 points
            combobox_id_salt_count: 0,
            combobox_from_label_count: 2, // 0 points (has violations)
            issues: vec!["Issue 1".to_string()],
        };

        // 20 + 0 + 15 + 0 + 10 + 0 + 0 + 0 = 45
        assert_eq!(result.compliance_score(), 45);
        assert!(!result.is_compliant());
    }
}
