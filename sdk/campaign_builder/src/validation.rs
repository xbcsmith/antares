// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Validation Module
//!
//! This module provides structured types and utilities for campaign validation.
//! It defines categories for validation results, severity levels, and a unified
//! result type that enables consistent display in the validation panel.
//!
//! # Components
//!
//! - [`ValidationCategory`] - Categories for grouping validation results
//! - [`ValidationSeverity`] - Severity levels (Error, Warning, Info, Passed)
//! - [`ValidationResult`] - Structured validation result with category, severity, message

use std::fmt;
use std::path::PathBuf;

/// Categories for grouping validation results.
///
/// These categories correspond to the different data types and configuration
/// sections in a campaign, allowing validation results to be organized and
/// displayed in a logical manner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationCategory {
    /// Campaign metadata (id, name, author, version, etc.)
    Metadata,
    /// Campaign configuration (party size, starting level, etc.)
    Configuration,
    /// File paths and file system validation
    FilePaths,
    /// Item definitions and IDs
    Items,
    /// Spell definitions and IDs
    Spells,
    /// Monster definitions and IDs
    Monsters,
    /// Map definitions and IDs
    Maps,
    /// Condition definitions and IDs
    Conditions,
    /// Quest definitions and IDs
    Quests,
    /// Dialogue definitions and IDs
    Dialogues,
    /// Class definitions
    Classes,
    /// Race definitions
    Races,
    /// Character definitions
    Characters,
    /// Asset files (images, sounds, etc.)
    Assets,
}

impl fmt::Display for ValidationCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl ValidationCategory {
    /// Returns the display name for the category.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::campaign_builder::validation::ValidationCategory;
    ///
    /// assert_eq!(ValidationCategory::Metadata.display_name(), "Metadata");
    /// assert_eq!(ValidationCategory::Items.display_name(), "Items");
    /// ```
    pub fn display_name(&self) -> &'static str {
        match self {
            ValidationCategory::Metadata => "Metadata",
            ValidationCategory::Configuration => "Configuration",
            ValidationCategory::FilePaths => "File Paths",
            ValidationCategory::Items => "Items",
            ValidationCategory::Spells => "Spells",
            ValidationCategory::Monsters => "Monsters",
            ValidationCategory::Maps => "Maps",
            ValidationCategory::Conditions => "Conditions",
            ValidationCategory::Quests => "Quests",
            ValidationCategory::Dialogues => "Dialogues",
            ValidationCategory::Classes => "Classes",
            ValidationCategory::Races => "Races",
            ValidationCategory::Characters => "Characters",
            ValidationCategory::Assets => "Assets",
        }
    }

    /// Returns all validation categories in display order.
    ///
    /// Categories are ordered logically: metadata/config first, then data types,
    /// then assets.
    pub fn all() -> Vec<ValidationCategory> {
        vec![
            ValidationCategory::Metadata,
            ValidationCategory::Configuration,
            ValidationCategory::FilePaths,
            ValidationCategory::Items,
            ValidationCategory::Spells,
            ValidationCategory::Monsters,
            ValidationCategory::Maps,
            ValidationCategory::Conditions,
            ValidationCategory::Quests,
            ValidationCategory::Dialogues,
            ValidationCategory::Classes,
            ValidationCategory::Races,
            ValidationCategory::Characters,
            ValidationCategory::Assets,
        ]
    }

    /// Returns the icon for this category.
    pub fn icon(&self) -> &'static str {
        match self {
            ValidationCategory::Metadata => "📋",
            ValidationCategory::Configuration => "⚙️",
            ValidationCategory::FilePaths => "📁",
            ValidationCategory::Items => "🎒",
            ValidationCategory::Spells => "✨",
            ValidationCategory::Monsters => "👹",
            ValidationCategory::Maps => "🗺️",
            ValidationCategory::Conditions => "💀",
            ValidationCategory::Quests => "📜",
            ValidationCategory::Dialogues => "💬",
            ValidationCategory::Classes => "⚔️",
            ValidationCategory::Races => "👤",
            ValidationCategory::Characters => "🧑",
            ValidationCategory::Assets => "📦",
        }
    }
}

/// Severity level for validation results.
///
/// Severity determines how the result is displayed and whether it blocks
/// certain operations (e.g., test play requires no errors).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationSeverity {
    /// Critical error that must be fixed
    Error,
    /// Warning that should be addressed but doesn't block operations
    Warning,
    /// Informational message
    Info,
    /// Validation check passed successfully
    Passed,
}

impl ValidationSeverity {
    /// Returns the display icon for the severity.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::campaign_builder::validation::ValidationSeverity;
    ///
    /// assert_eq!(ValidationSeverity::Error.icon(), "❌");
    /// assert_eq!(ValidationSeverity::Warning.icon(), "⚠️");
    /// assert_eq!(ValidationSeverity::Passed.icon(), "✅");
    /// ```
    pub fn icon(&self) -> &'static str {
        match self {
            ValidationSeverity::Error => "❌",
            ValidationSeverity::Warning => "⚠️",
            ValidationSeverity::Info => "ℹ️",
            ValidationSeverity::Passed => "✅",
        }
    }

    /// Returns the color for displaying this severity.
    ///
    /// Returns an `egui::Color32` appropriate for the severity level.
    pub fn color(&self) -> eframe::egui::Color32 {
        match self {
            ValidationSeverity::Error => eframe::egui::Color32::from_rgb(255, 80, 80),
            ValidationSeverity::Warning => eframe::egui::Color32::from_rgb(255, 180, 0),
            ValidationSeverity::Info => eframe::egui::Color32::from_rgb(100, 180, 255),
            ValidationSeverity::Passed => eframe::egui::Color32::from_rgb(80, 200, 80),
        }
    }

    /// Returns the display name for the severity.
    pub fn display_name(&self) -> &'static str {
        match self {
            ValidationSeverity::Error => "Error",
            ValidationSeverity::Warning => "Warning",
            ValidationSeverity::Info => "Info",
            ValidationSeverity::Passed => "Passed",
        }
    }
}

/// A structured validation result.
///
/// This type captures all information needed to display a validation result
/// in the UI, including its category, severity, message, and optional file path.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationResult {
    /// The category this result belongs to
    pub category: ValidationCategory,
    /// The severity of the result
    pub severity: ValidationSeverity,
    /// The human-readable message
    pub message: String,
    /// Optional file path associated with the result
    pub file_path: Option<PathBuf>,
}

impl ValidationResult {
    /// Creates a new validation result.
    ///
    /// # Arguments
    ///
    /// * `category` - The category for grouping
    /// * `severity` - The severity level
    /// * `message` - Human-readable description
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::campaign_builder::validation::{
    ///     ValidationCategory, ValidationSeverity, ValidationResult
    /// };
    ///
    /// let result = ValidationResult::new(
    ///     ValidationCategory::Metadata,
    ///     ValidationSeverity::Error,
    ///     "Campaign ID is required",
    /// );
    /// assert_eq!(result.severity, ValidationSeverity::Error);
    /// ```
    pub fn new(
        category: ValidationCategory,
        severity: ValidationSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            category,
            severity,
            message: message.into(),
            file_path: None,
        }
    }

    /// Creates an error result.
    ///
    /// # Arguments
    ///
    /// * `category` - The category for grouping
    /// * `message` - Human-readable description
    pub fn error(category: ValidationCategory, message: impl Into<String>) -> Self {
        Self::new(category, ValidationSeverity::Error, message)
    }

    /// Creates a warning result.
    ///
    /// # Arguments
    ///
    /// * `category` - The category for grouping
    /// * `message` - Human-readable description
    pub fn warning(category: ValidationCategory, message: impl Into<String>) -> Self {
        Self::new(category, ValidationSeverity::Warning, message)
    }

    /// Creates an info result.
    ///
    /// # Arguments
    ///
    /// * `category` - The category for grouping
    /// * `message` - Human-readable description
    pub fn info(category: ValidationCategory, message: impl Into<String>) -> Self {
        Self::new(category, ValidationSeverity::Info, message)
    }

    /// Creates a passed result.
    ///
    /// # Arguments
    ///
    /// * `category` - The category for grouping
    /// * `message` - Human-readable description
    pub fn passed(category: ValidationCategory, message: impl Into<String>) -> Self {
        Self::new(category, ValidationSeverity::Passed, message)
    }

    /// Adds a file path to the result.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path associated with this result
    pub fn with_file_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    /// Returns true if this is an error.
    pub fn is_error(&self) -> bool {
        self.severity == ValidationSeverity::Error
    }

    /// Returns true if this is a warning.
    pub fn is_warning(&self) -> bool {
        self.severity == ValidationSeverity::Warning
    }

    /// Returns true if this is a passed check.
    pub fn is_passed(&self) -> bool {
        self.severity == ValidationSeverity::Passed
    }
}

/// Summary statistics for validation results.
#[derive(Debug, Clone, Default)]
pub struct ValidationSummary {
    /// Number of errors
    pub error_count: usize,
    /// Number of warnings
    pub warning_count: usize,
    /// Number of info messages
    pub info_count: usize,
    /// Number of passed checks
    pub passed_count: usize,
}

impl ValidationSummary {
    /// Creates a summary from a list of validation results.
    ///
    /// # Arguments
    ///
    /// * `results` - Slice of validation results to summarize
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::campaign_builder::validation::{
    ///     ValidationCategory, ValidationResult, ValidationSummary
    /// };
    ///
    /// let results = vec![
    ///     ValidationResult::error(ValidationCategory::Metadata, "Error 1"),
    ///     ValidationResult::warning(ValidationCategory::Items, "Warning 1"),
    ///     ValidationResult::passed(ValidationCategory::Spells, "All spells valid"),
    /// ];
    ///
    /// let summary = ValidationSummary::from_results(&results);
    /// assert_eq!(summary.error_count, 1);
    /// assert_eq!(summary.warning_count, 1);
    /// assert_eq!(summary.passed_count, 1);
    /// ```
    pub fn from_results(results: &[ValidationResult]) -> Self {
        let mut summary = Self::default();
        for result in results {
            match result.severity {
                ValidationSeverity::Error => summary.error_count += 1,
                ValidationSeverity::Warning => summary.warning_count += 1,
                ValidationSeverity::Info => summary.info_count += 1,
                ValidationSeverity::Passed => summary.passed_count += 1,
            }
        }
        summary
    }

    /// Returns total number of results.
    pub fn total(&self) -> usize {
        self.error_count + self.warning_count + self.info_count + self.passed_count
    }

    /// Returns true if there are no errors.
    pub fn has_no_errors(&self) -> bool {
        self.error_count == 0
    }

    /// Returns true if all checks passed with no errors or warnings.
    pub fn all_passed(&self) -> bool {
        self.error_count == 0 && self.warning_count == 0
    }
}

/// Groups validation results by category.
///
/// # Arguments
///
/// * `results` - Slice of validation results to group
///
/// # Returns
///
/// A vector of tuples containing (category, results for that category),
/// ordered by category display order.
///
/// # Examples
///
/// ```
/// use antares::sdk::campaign_builder::validation::{
///     ValidationCategory, ValidationResult, group_results_by_category
/// };
///
/// let results = vec![
///     ValidationResult::error(ValidationCategory::Items, "Item error"),
///     ValidationResult::error(ValidationCategory::Metadata, "Metadata error"),
///     ValidationResult::warning(ValidationCategory::Items, "Item warning"),
/// ];
///
/// let grouped = group_results_by_category(&results);
/// // Results are grouped by category in display order
/// ```
pub fn group_results_by_category(
    results: &[ValidationResult],
) -> Vec<(ValidationCategory, Vec<&ValidationResult>)> {
    let mut grouped: Vec<(ValidationCategory, Vec<&ValidationResult>)> = Vec::new();

    for category in ValidationCategory::all() {
        let category_results: Vec<&ValidationResult> =
            results.iter().filter(|r| r.category == category).collect();

        if !category_results.is_empty() {
            grouped.push((category, category_results));
        }
    }

    grouped
}

/// Counts results by category.
///
/// # Arguments
///
/// * `results` - Slice of validation results to count
/// * `category` - The category to count
///
/// # Returns
///
/// Number of results in the specified category.
pub fn count_by_category(results: &[ValidationResult], category: ValidationCategory) -> usize {
    results.iter().filter(|r| r.category == category).count()
}

/// Counts errors by category.
///
/// # Arguments
///
/// * `results` - Slice of validation results to count
/// * `category` - The category to count
///
/// # Returns
///
/// Number of errors in the specified category.
pub fn count_errors_by_category(
    results: &[ValidationResult],
    category: ValidationCategory,
) -> usize {
    results
        .iter()
        .filter(|r| r.category == category && r.is_error())
        .count()
}

/// Validates that a class_id reference exists in the available classes.
///
/// # Arguments
///
/// * `class_id` - The class ID to validate
/// * `available_classes` - Slice of valid class IDs
/// * `context` - Description of where the reference is used (e.g., "character 'Hero'")
///
/// # Returns
///
/// Returns `Some(ValidationResult)` with an error if the class_id is invalid,
/// or `None` if the reference is valid.
///
/// # Examples
///
/// ```
/// use antares::sdk::campaign_builder::validation::validate_class_id_reference;
///
/// let classes = vec!["knight".to_string(), "sorcerer".to_string()];
/// let result = validate_class_id_reference("knight", &classes, "character 'Hero'");
/// assert!(result.is_none()); // Valid reference
///
/// let result = validate_class_id_reference("invalid_class", &classes, "character 'Hero'");
/// assert!(result.is_some()); // Invalid reference
/// ```
pub fn validate_class_id_reference(
    class_id: &str,
    available_classes: &[String],
    context: &str,
) -> Option<ValidationResult> {
    if class_id.is_empty() {
        return Some(ValidationResult::error(
            ValidationCategory::Characters,
            format!("Empty class_id in {}", context),
        ));
    }

    if !available_classes.iter().any(|c| c == class_id) {
        Some(ValidationResult::error(
            ValidationCategory::Characters,
            format!(
                "Invalid class_id '{}' in {} (available: {})",
                class_id,
                context,
                if available_classes.is_empty() {
                    "none loaded".to_string()
                } else {
                    available_classes.join(", ")
                }
            ),
        ))
    } else {
        None
    }
}

/// Validates that a race_id reference exists in the available races.
///
/// # Arguments
///
/// * `race_id` - The race ID to validate
/// * `available_races` - Slice of valid race IDs
/// * `context` - Description of where the reference is used (e.g., "character 'Hero'")
///
/// # Returns
///
/// Returns `Some(ValidationResult)` with an error if the race_id is invalid,
/// or `None` if the reference is valid.
///
/// # Examples
///
/// ```
/// use antares::sdk::campaign_builder::validation::validate_race_id_reference;
///
/// let races = vec!["human".to_string(), "elf".to_string()];
/// let result = validate_race_id_reference("human", &races, "character 'Hero'");
/// assert!(result.is_none()); // Valid reference
///
/// let result = validate_race_id_reference("invalid_race", &races, "character 'Hero'");
/// assert!(result.is_some()); // Invalid reference
/// ```
pub fn validate_race_id_reference(
    race_id: &str,
    available_races: &[String],
    context: &str,
) -> Option<ValidationResult> {
    if race_id.is_empty() {
        return Some(ValidationResult::error(
            ValidationCategory::Characters,
            format!("Empty race_id in {}", context),
        ));
    }

    if !available_races.iter().any(|r| r == race_id) {
        Some(ValidationResult::error(
            ValidationCategory::Characters,
            format!(
                "Invalid race_id '{}' in {} (available: {})",
                race_id,
                context,
                if available_races.is_empty() {
                    "none loaded".to_string()
                } else {
                    available_races.join(", ")
                }
            ),
        ))
    } else {
        None
    }
}

/// Validates all class and race references in a collection of characters.
///
/// # Arguments
///
/// * `characters` - Iterator of (character_name, class_id, race_id) tuples
/// * `available_classes` - Slice of valid class IDs
/// * `available_races` - Slice of valid race IDs
///
/// # Returns
///
/// A vector of validation results for any invalid references found.
///
/// # Examples
///
/// ```
/// use antares::sdk::campaign_builder::validation::validate_character_references;
///
/// let characters = vec![
///     ("Hero".to_string(), "knight".to_string(), "human".to_string()),
///     ("Mage".to_string(), "sorcerer".to_string(), "elf".to_string()),
/// ];
/// let classes = vec!["knight".to_string(), "sorcerer".to_string()];
/// let races = vec!["human".to_string(), "elf".to_string()];
///
/// let results = validate_character_references(
///     characters.iter().map(|(n, c, r)| (n.as_str(), c.as_str(), r.as_str())),
///     &classes,
///     &races,
/// );
/// assert!(results.is_empty()); // All valid
/// ```
pub fn validate_character_references<'a>(
    characters: impl Iterator<Item = (&'a str, &'a str, &'a str)>,
    available_classes: &[String],
    available_races: &[String],
) -> Vec<ValidationResult> {
    let mut results = Vec::new();

    for (name, class_id, race_id) in characters {
        let context = format!("character '{}'", name);

        if let Some(result) = validate_class_id_reference(class_id, available_classes, &context) {
            results.push(result);
        }

        if let Some(result) = validate_race_id_reference(race_id, available_races, &context) {
            results.push(result);
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_class_id_reference_valid() {
        let classes = vec!["knight".to_string(), "sorcerer".to_string()];
        let result = validate_class_id_reference("knight", &classes, "test character");
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_class_id_reference_invalid() {
        let classes = vec!["knight".to_string(), "sorcerer".to_string()];
        let result = validate_class_id_reference("invalid", &classes, "test character");
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.is_error());
        assert!(result.message.contains("invalid"));
    }

    #[test]
    fn test_validate_class_id_reference_empty() {
        let classes = vec!["knight".to_string()];
        let result = validate_class_id_reference("", &classes, "test character");
        assert!(result.is_some());
        assert!(result.unwrap().message.contains("Empty class_id"));
    }

    #[test]
    fn test_validate_race_id_reference_valid() {
        let races = vec!["human".to_string(), "elf".to_string()];
        let result = validate_race_id_reference("human", &races, "test character");
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_race_id_reference_invalid() {
        let races = vec!["human".to_string(), "elf".to_string()];
        let result = validate_race_id_reference("invalid", &races, "test character");
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.is_error());
        assert!(result.message.contains("invalid"));
    }

    #[test]
    fn test_validate_race_id_reference_empty() {
        let races = vec!["human".to_string()];
        let result = validate_race_id_reference("", &races, "test character");
        assert!(result.is_some());
        assert!(result.unwrap().message.contains("Empty race_id"));
    }

    #[test]
    fn test_validate_character_references_all_valid() {
        let characters = vec![("Hero", "knight", "human"), ("Mage", "sorcerer", "elf")];
        let classes = vec!["knight".to_string(), "sorcerer".to_string()];
        let races = vec!["human".to_string(), "elf".to_string()];

        let results = validate_character_references(characters.into_iter(), &classes, &races);
        assert!(results.is_empty());
    }

    #[test]
    fn test_validate_character_references_invalid_class() {
        let characters = vec![("Hero", "invalid_class", "human")];
        let classes = vec!["knight".to_string()];
        let races = vec!["human".to_string()];

        let results = validate_character_references(characters.into_iter(), &classes, &races);
        assert_eq!(results.len(), 1);
        assert!(results[0].message.contains("invalid_class"));
    }

    #[test]
    fn test_validate_character_references_invalid_race() {
        let characters = vec![("Hero", "knight", "invalid_race")];
        let classes = vec!["knight".to_string()];
        let races = vec!["human".to_string()];

        let results = validate_character_references(characters.into_iter(), &classes, &races);
        assert_eq!(results.len(), 1);
        assert!(results[0].message.contains("invalid_race"));
    }

    #[test]
    fn test_validate_character_references_both_invalid() {
        let characters = vec![("Hero", "bad_class", "bad_race")];
        let classes = vec!["knight".to_string()];
        let races = vec!["human".to_string()];

        let results = validate_character_references(characters.into_iter(), &classes, &races);
        assert_eq!(results.len(), 2); // One for class, one for race
    }

    #[test]
    fn test_validation_category_display_name() {
        assert_eq!(ValidationCategory::Metadata.display_name(), "Metadata");
        assert_eq!(ValidationCategory::Items.display_name(), "Items");
        assert_eq!(ValidationCategory::FilePaths.display_name(), "File Paths");
    }

    #[test]
    fn test_validation_category_all() {
        let all = ValidationCategory::all();
        assert!(!all.is_empty());
        assert!(all.contains(&ValidationCategory::Metadata));
        assert!(all.contains(&ValidationCategory::Items));
        assert!(all.contains(&ValidationCategory::Assets));
    }

    #[test]
    fn test_validation_category_icon() {
        assert_eq!(ValidationCategory::Metadata.icon(), "📋");
        assert_eq!(ValidationCategory::Items.icon(), "🎒");
        assert_eq!(ValidationCategory::Spells.icon(), "✨");
    }

    #[test]
    fn test_validation_severity_icon() {
        assert_eq!(ValidationSeverity::Error.icon(), "❌");
        assert_eq!(ValidationSeverity::Warning.icon(), "⚠️");
        assert_eq!(ValidationSeverity::Info.icon(), "ℹ️");
        assert_eq!(ValidationSeverity::Passed.icon(), "✅");
    }

    #[test]
    fn test_validation_severity_display_name() {
        assert_eq!(ValidationSeverity::Error.display_name(), "Error");
        assert_eq!(ValidationSeverity::Warning.display_name(), "Warning");
        assert_eq!(ValidationSeverity::Passed.display_name(), "Passed");
    }

    #[test]
    fn test_validation_result_new() {
        let result = ValidationResult::new(
            ValidationCategory::Metadata,
            ValidationSeverity::Error,
            "Test error message",
        );

        assert_eq!(result.category, ValidationCategory::Metadata);
        assert_eq!(result.severity, ValidationSeverity::Error);
        assert_eq!(result.message, "Test error message");
        assert!(result.file_path.is_none());
    }

    #[test]
    fn test_validation_result_error() {
        let result = ValidationResult::error(ValidationCategory::Items, "Duplicate item ID");

        assert_eq!(result.category, ValidationCategory::Items);
        assert!(result.is_error());
        assert!(!result.is_warning());
        assert!(!result.is_passed());
    }

    #[test]
    fn test_validation_result_warning() {
        let result = ValidationResult::warning(ValidationCategory::Metadata, "Author recommended");

        assert_eq!(result.category, ValidationCategory::Metadata);
        assert!(result.is_warning());
        assert!(!result.is_error());
    }

    #[test]
    fn test_validation_result_passed() {
        let result = ValidationResult::passed(ValidationCategory::Spells, "All spells valid");

        assert!(result.is_passed());
        assert!(!result.is_error());
        assert!(!result.is_warning());
    }

    #[test]
    fn test_validation_result_with_file_path() {
        let result = ValidationResult::error(ValidationCategory::FilePaths, "File not found")
            .with_file_path("data/items.ron");

        assert!(result.file_path.is_some());
        assert_eq!(result.file_path.unwrap(), PathBuf::from("data/items.ron"));
    }

    #[test]
    fn test_validation_summary_from_results() {
        let results = vec![
            ValidationResult::error(ValidationCategory::Metadata, "Error 1"),
            ValidationResult::error(ValidationCategory::Items, "Error 2"),
            ValidationResult::warning(ValidationCategory::Spells, "Warning 1"),
            ValidationResult::info(ValidationCategory::Configuration, "Info 1"),
            ValidationResult::passed(ValidationCategory::Monsters, "Passed 1"),
        ];

        let summary = ValidationSummary::from_results(&results);

        assert_eq!(summary.error_count, 2);
        assert_eq!(summary.warning_count, 1);
        assert_eq!(summary.info_count, 1);
        assert_eq!(summary.passed_count, 1);
        assert_eq!(summary.total(), 5);
    }

    #[test]
    fn test_validation_summary_has_no_errors() {
        let results = vec![
            ValidationResult::warning(ValidationCategory::Metadata, "Warning"),
            ValidationResult::passed(ValidationCategory::Items, "Passed"),
        ];

        let summary = ValidationSummary::from_results(&results);
        assert!(summary.has_no_errors());
    }

    #[test]
    fn test_validation_summary_all_passed() {
        let results = vec![
            ValidationResult::passed(ValidationCategory::Metadata, "Passed 1"),
            ValidationResult::passed(ValidationCategory::Items, "Passed 2"),
        ];

        let summary = ValidationSummary::from_results(&results);
        assert!(summary.all_passed());

        // Add a warning - should not be "all passed"
        let results_with_warning = vec![
            ValidationResult::passed(ValidationCategory::Metadata, "Passed"),
            ValidationResult::warning(ValidationCategory::Items, "Warning"),
        ];

        let summary2 = ValidationSummary::from_results(&results_with_warning);
        assert!(!summary2.all_passed());
        assert!(summary2.has_no_errors());
    }

    #[test]
    fn test_group_results_by_category() {
        let results = vec![
            ValidationResult::error(ValidationCategory::Items, "Item error 1"),
            ValidationResult::error(ValidationCategory::Metadata, "Metadata error"),
            ValidationResult::warning(ValidationCategory::Items, "Item warning"),
            ValidationResult::passed(ValidationCategory::Spells, "Spells OK"),
        ];

        let grouped = group_results_by_category(&results);

        // Should have 3 categories with results
        assert_eq!(grouped.len(), 3);

        // Find metadata group
        let metadata_group = grouped
            .iter()
            .find(|(cat, _)| *cat == ValidationCategory::Metadata);
        assert!(metadata_group.is_some());
        assert_eq!(metadata_group.unwrap().1.len(), 1);

        // Find items group
        let items_group = grouped
            .iter()
            .find(|(cat, _)| *cat == ValidationCategory::Items);
        assert!(items_group.is_some());
        assert_eq!(items_group.unwrap().1.len(), 2);
    }

    #[test]
    fn test_count_by_category() {
        let results = vec![
            ValidationResult::error(ValidationCategory::Items, "Error 1"),
            ValidationResult::error(ValidationCategory::Items, "Error 2"),
            ValidationResult::warning(ValidationCategory::Items, "Warning"),
            ValidationResult::error(ValidationCategory::Metadata, "Metadata error"),
        ];

        assert_eq!(count_by_category(&results, ValidationCategory::Items), 3);
        assert_eq!(count_by_category(&results, ValidationCategory::Metadata), 1);
        assert_eq!(count_by_category(&results, ValidationCategory::Spells), 0);
    }

    #[test]
    fn test_count_errors_by_category() {
        let results = vec![
            ValidationResult::error(ValidationCategory::Items, "Error 1"),
            ValidationResult::error(ValidationCategory::Items, "Error 2"),
            ValidationResult::warning(ValidationCategory::Items, "Warning"),
            ValidationResult::error(ValidationCategory::Metadata, "Metadata error"),
        ];

        assert_eq!(
            count_errors_by_category(&results, ValidationCategory::Items),
            2
        );
        assert_eq!(
            count_errors_by_category(&results, ValidationCategory::Metadata),
            1
        );
    }

    #[test]
    fn test_validation_summary_empty() {
        let results: Vec<ValidationResult> = vec![];
        let summary = ValidationSummary::from_results(&results);

        assert_eq!(summary.error_count, 0);
        assert_eq!(summary.warning_count, 0);
        assert_eq!(summary.info_count, 0);
        assert_eq!(summary.passed_count, 0);
        assert_eq!(summary.total(), 0);
        assert!(summary.has_no_errors());
        assert!(summary.all_passed());
    }
}
