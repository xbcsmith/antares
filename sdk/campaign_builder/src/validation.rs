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
    /// NPC definitions and IDs
    NPCs,
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
            ValidationCategory::NPCs => "NPCs",
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
            ValidationCategory::NPCs,
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
            ValidationCategory::NPCs => "🧙",
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

/// Standard proficiency IDs used in the game.
///
/// These are the proficiency IDs defined in data/proficiencies.ron.
pub const STANDARD_PROFICIENCY_IDS: &[&str] = &[
    "simple_weapon",
    "martial_melee",
    "martial_ranged",
    "blunt_weapon",
    "unarmed",
    "light_armor",
    "medium_armor",
    "heavy_armor",
    "shield",
    "arcane_item",
    "divine_item",
];

/// Validates that a proficiency ID is a known standard proficiency.
///
/// # Arguments
///
/// * `proficiency_id` - The proficiency ID to validate
/// * `context` - Description of where the reference is used (e.g., "class 'Knight'")
///
/// # Returns
///
/// Returns `Some(ValidationResult)` with a warning if the proficiency_id is not
/// a standard proficiency, or `None` if the reference is valid.
///
/// # Examples
///
/// ```
/// use antares::sdk::campaign_builder::validation::validate_proficiency_id;
///
/// let result = validate_proficiency_id("martial_melee", "class 'Knight'");
/// assert!(result.is_none()); // Valid proficiency
///
/// let result = validate_proficiency_id("unknown_prof", "class 'Knight'");
/// assert!(result.is_some()); // Unknown proficiency (warning)
/// ```
pub fn validate_proficiency_id(proficiency_id: &str, context: &str) -> Option<ValidationResult> {
    if proficiency_id.is_empty() {
        return Some(ValidationResult::warning(
            ValidationCategory::Classes,
            format!("Empty proficiency ID in {}", context),
        ));
    }

    if !STANDARD_PROFICIENCY_IDS.contains(&proficiency_id) {
        Some(ValidationResult::warning(
            ValidationCategory::Classes,
            format!(
                "Unknown proficiency ID '{}' in {} (standard: {})",
                proficiency_id,
                context,
                STANDARD_PROFICIENCY_IDS.join(", ")
            ),
        ))
    } else {
        None
    }
}

/// Validates all proficiency IDs in a class definition.
///
/// # Arguments
///
/// * `class_id` - The class identifier
/// * `proficiencies` - Iterator of proficiency IDs to validate
///
/// # Returns
///
/// A vector of validation results for any invalid proficiency IDs found.
///
/// # Examples
///
/// ```
/// use antares::sdk::campaign_builder::validation::validate_class_proficiencies;
///
/// let proficiencies = vec!["martial_melee", "heavy_armor", "shield"];
/// let results = validate_class_proficiencies("knight", proficiencies.into_iter());
/// assert!(results.is_empty()); // All valid
///
/// let proficiencies = vec!["martial_melee", "unknown_prof"];
/// let results = validate_class_proficiencies("knight", proficiencies.into_iter());
/// assert_eq!(results.len(), 1); // One warning for unknown_prof
/// ```
pub fn validate_class_proficiencies<'a>(
    class_id: &str,
    proficiencies: impl Iterator<Item = &'a str>,
) -> Vec<ValidationResult> {
    let mut results = Vec::new();
    let context = format!("class '{}'", class_id);

    for prof_id in proficiencies {
        if let Some(result) = validate_proficiency_id(prof_id, &context) {
            results.push(result);
        }
    }

    results
}

/// Validates all proficiency IDs in a race definition.
///
/// # Arguments
///
/// * `race_id` - The race identifier
/// * `proficiencies` - Iterator of proficiency IDs to validate
///
/// # Returns
///
/// A vector of validation results for any invalid proficiency IDs found.
///
/// # Examples
///
/// ```
/// use antares::sdk::campaign_builder::validation::validate_race_proficiencies;
///
/// let proficiencies = vec!["martial_ranged"];
/// let results = validate_race_proficiencies("elf", proficiencies.into_iter());
/// assert!(results.is_empty()); // All valid
/// ```
pub fn validate_race_proficiencies<'a>(
    race_id: &str,
    proficiencies: impl Iterator<Item = &'a str>,
) -> Vec<ValidationResult> {
    let mut results = Vec::new();
    let context = format!("race '{}'", race_id);

    for prof_id in proficiencies {
        if let Some(result) = validate_proficiency_id(prof_id, &context) {
            results.push(result);
        }
    }

    results
}

/// Standard item tags used in the game.
///
/// These are the conventional tags that races can mark as incompatible.
pub const STANDARD_ITEM_TAGS: &[&str] = &[
    "large_weapon",
    "two_handed",
    "heavy_armor",
    "elven_crafted",
    "dwarven_crafted",
    "requires_strength",
];

/// Validates that an item tag is a known standard tag.
///
/// # Arguments
///
/// * `tag` - The tag to validate
/// * `context` - Description of where the tag is used
///
/// # Returns
///
/// Returns `Some(ValidationResult)` with an info message if the tag is not
/// a standard tag, or `None` if the tag is recognized.
pub fn validate_item_tag(tag: &str, context: &str) -> Option<ValidationResult> {
    if tag.is_empty() {
        return Some(ValidationResult::warning(
            ValidationCategory::Items,
            format!("Empty tag in {}", context),
        ));
    }

    if !STANDARD_ITEM_TAGS.contains(&tag) {
        Some(ValidationResult::info(
            ValidationCategory::Items,
            format!(
                "Custom tag '{}' in {} (standard: {})",
                tag,
                context,
                STANDARD_ITEM_TAGS.join(", ")
            ),
        ))
    } else {
        None
    }
}

/// Validates all item tags for a race's incompatible_item_tags.
///
/// # Arguments
///
/// * `race_id` - The race identifier
/// * `tags` - Iterator of tag strings to validate
///
/// # Returns
///
/// A vector of validation results for any non-standard tags found.
pub fn validate_race_incompatible_tags<'a>(
    race_id: &str,
    tags: impl Iterator<Item = &'a str>,
) -> Vec<ValidationResult> {
    let mut results = Vec::new();
    let context = format!("race '{}' incompatible_item_tags", race_id);

    for tag in tags {
        if let Some(result) = validate_item_tag(tag, &context) {
            results.push(result);
        }
    }

    results
}

/// Validates all tags for an item.
///
/// # Arguments
///
/// * `item_id` - The item identifier
/// * `item_name` - The item name for display
/// * `tags` - Iterator of tag strings to validate
///
/// # Returns
///
/// A vector of validation results for any non-standard tags found.
pub fn validate_item_tags<'a>(
    item_id: u8,
    item_name: &str,
    tags: impl Iterator<Item = &'a str>,
) -> Vec<ValidationResult> {
    let mut results = Vec::new();
    let context = format!("item '{}' (ID: {})", item_name, item_id);

    for tag in tags {
        if let Some(result) = validate_item_tag(tag, &context) {
            results.push(result);
        }
    }

    results
}

/// Validates that a weapon has a classification set.
///
/// # Arguments
///
/// * `item_id` - The item identifier
/// * `item_name` - The item name for display
/// * `has_classification` - Whether the weapon has a non-default classification
///
/// # Returns
///
/// Returns a validation result if classification should be reviewed.
pub fn validate_weapon_classification(
    item_id: u8,
    item_name: &str,
    classification: &str,
) -> Option<ValidationResult> {
    // Simple is the default, so it's fine if set explicitly or by default
    // We just provide info that classification affects proficiency requirements
    Some(ValidationResult::info(
        ValidationCategory::Items,
        format!(
            "Weapon '{}' (ID: {}) classification: {} (requires {} proficiency)",
            item_name,
            item_id,
            classification,
            match classification {
                "Simple" => "simple_weapon",
                "MartialMelee" => "martial_melee",
                "MartialRanged" => "martial_ranged",
                "Blunt" => "blunt_weapon",
                "Unarmed" => "unarmed",
                _ => "unknown",
            }
        ),
    ))
}

/// Validates that an armor has a classification set.
///
/// # Arguments
///
/// * `item_id` - The item identifier
/// * `item_name` - The item name for display
/// * `classification` - The armor classification string
///
/// # Returns
///
/// Returns a validation result with info about the proficiency requirement.
pub fn validate_armor_classification(
    item_id: u8,
    item_name: &str,
    classification: &str,
) -> Option<ValidationResult> {
    Some(ValidationResult::info(
        ValidationCategory::Items,
        format!(
            "Armor '{}' (ID: {}) classification: {} (requires {} proficiency)",
            item_name,
            item_id,
            classification,
            match classification {
                "Light" => "light_armor",
                "Medium" => "medium_armor",
                "Heavy" => "heavy_armor",
                "Shield" => "shield",
                _ => "unknown",
            }
        ),
    ))
}

#[cfg(test)]
/// Validates NPC placement references
///
/// Checks if NPC placement references a valid NPC ID from the NPC database.
///
/// # Arguments
///
/// * `npc_id` - The NPC ID from the placement
/// * `available_npc_ids` - Set of valid NPC IDs from the NPC database
///
/// # Returns
///
/// Returns `Ok(())` if valid, or an error message
pub fn validate_npc_placement_reference(
    npc_id: &str,
    available_npc_ids: &std::collections::HashSet<String>,
) -> Result<(), String> {
    if npc_id.is_empty() {
        return Err("NPC ID cannot be empty".to_string());
    }

    if !available_npc_ids.contains(npc_id) {
        return Err(format!(
            "NPC placement references unknown NPC ID: '{}'",
            npc_id
        ));
    }

    Ok(())
}

/// Validates NPC dialogue ID reference
///
/// Checks if NPC's dialogue_id references a valid dialogue from the dialogue database.
///
/// # Arguments
///
/// * `dialogue_id` - The dialogue ID from the NPC definition
/// * `available_dialogue_ids` - Set of valid dialogue IDs
///
/// # Returns
///
/// Returns `Ok(())` if valid, or an error message
pub fn validate_npc_dialogue_reference(
    dialogue_id: Option<u16>,
    available_dialogue_ids: &std::collections::HashSet<u16>,
) -> Result<(), String> {
    if let Some(id) = dialogue_id {
        if !available_dialogue_ids.contains(&id) {
            return Err(format!("NPC references unknown dialogue ID: {}", id));
        }
    }
    Ok(())
}

/// Validates NPC quest ID references
///
/// Checks if NPC's quest_ids reference valid quests from the quest database.
///
/// # Arguments
///
/// * `quest_ids` - The quest IDs from the NPC definition
/// * `available_quest_ids` - Set of valid quest IDs
///
/// # Returns
///
/// Returns `Ok(())` if all valid, or an error message with the first invalid ID
pub fn validate_npc_quest_references(
    quest_ids: &[u32],
    available_quest_ids: &std::collections::HashSet<u32>,
) -> Result<(), String> {
    for quest_id in quest_ids {
        if !available_quest_ids.contains(quest_id) {
            return Err(format!("NPC references unknown quest ID: {}", quest_id));
        }
    }
    Ok(())
}

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
        assert!(results[0].is_error());
    }

    #[test]
    fn test_validate_character_references_invalid_race() {
        let characters = vec![("Hero", "knight", "invalid_race")];
        let classes = vec!["knight".to_string()];
        let races = vec!["human".to_string()];

        let results = validate_character_references(characters.into_iter(), &classes, &races);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_error());
    }

    #[test]
    fn test_validate_character_references_both_invalid() {
        let characters = vec![("Hero", "invalid_class", "invalid_race")];
        let classes = vec!["knight".to_string()];
        let races = vec!["human".to_string()];

        let results = validate_character_references(characters.into_iter(), &classes, &races);
        assert_eq!(results.len(), 2);
    }

    // Proficiency validation tests

    #[test]
    fn test_validate_proficiency_id_valid() {
        let result = validate_proficiency_id("martial_melee", "class 'Knight'");
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_proficiency_id_invalid() {
        let result = validate_proficiency_id("unknown_prof", "class 'Knight'");
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.is_warning());
        assert!(result.message.contains("unknown_prof"));
    }

    #[test]
    fn test_validate_proficiency_id_empty() {
        let result = validate_proficiency_id("", "class 'Knight'");
        assert!(result.is_some());
        assert!(result.unwrap().message.contains("Empty proficiency ID"));
    }

    #[test]
    fn test_validate_class_proficiencies_all_valid() {
        let proficiencies = vec!["martial_melee", "heavy_armor", "shield"];
        let results = validate_class_proficiencies("knight", proficiencies.into_iter());
        assert!(results.is_empty());
    }

    #[test]
    fn test_validate_class_proficiencies_with_invalid() {
        let proficiencies = vec!["martial_melee", "unknown_prof", "heavy_armor"];
        let results = validate_class_proficiencies("knight", proficiencies.into_iter());
        assert_eq!(results.len(), 1);
        assert!(results[0].message.contains("unknown_prof"));
    }

    #[test]
    fn test_validate_race_proficiencies_valid() {
        let proficiencies = vec!["martial_ranged"];
        let results = validate_race_proficiencies("elf", proficiencies.into_iter());
        assert!(results.is_empty());
    }

    #[test]
    fn test_validate_race_proficiencies_with_invalid() {
        let proficiencies = vec!["bow_mastery"];
        let results = validate_race_proficiencies("elf", proficiencies.into_iter());
        assert_eq!(results.len(), 1);
        assert!(results[0].message.contains("bow_mastery"));
    }

    // Item tag validation tests

    #[test]
    fn test_validate_item_tag_valid() {
        let result = validate_item_tag("large_weapon", "item 'Greatsword'");
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_item_tag_custom() {
        let result = validate_item_tag("custom_tag", "item 'Special Item'");
        assert!(result.is_some());
        // Custom tags are info, not warnings
        let result = result.unwrap();
        assert_eq!(result.severity, ValidationSeverity::Info);
    }

    #[test]
    fn test_validate_item_tag_empty() {
        let result = validate_item_tag("", "item 'Test'");
        assert!(result.is_some());
        assert!(result.unwrap().is_warning());
    }

    #[test]
    fn test_validate_race_incompatible_tags_valid() {
        let tags = vec!["large_weapon", "heavy_armor"];
        let results = validate_race_incompatible_tags("gnome", tags.into_iter());
        assert!(results.is_empty());
    }

    #[test]
    fn test_validate_race_incompatible_tags_custom() {
        let tags = vec!["large_weapon", "gnome_unfriendly"];
        let results = validate_race_incompatible_tags("gnome", tags.into_iter());
        assert_eq!(results.len(), 1);
        assert!(results[0].message.contains("gnome_unfriendly"));
    }

    #[test]
    fn test_validate_item_tags_valid() {
        let tags = vec!["two_handed", "large_weapon"];
        let results = validate_item_tags(1, "Greatsword", tags.into_iter());
        assert!(results.is_empty());
    }

    #[test]
    fn test_validate_item_tags_with_custom() {
        let tags = vec!["two_handed", "magical_aura"];
        let results = validate_item_tags(1, "Magic Sword", tags.into_iter());
        assert_eq!(results.len(), 1);
        assert!(results[0].message.contains("magical_aura"));
    }

    #[test]
    fn test_validate_weapon_classification() {
        let result = validate_weapon_classification(1, "Longsword", "MartialMelee");
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.message.contains("martial_melee"));
    }

    #[test]
    fn test_validate_armor_classification() {
        let result = validate_armor_classification(10, "Plate Mail", "Heavy");
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.message.contains("heavy_armor"));
    }

    #[test]
    fn test_standard_proficiency_ids_complete() {
        // Ensure all expected proficiencies are in the list
        assert!(STANDARD_PROFICIENCY_IDS.contains(&"simple_weapon"));
        assert!(STANDARD_PROFICIENCY_IDS.contains(&"martial_melee"));
        assert!(STANDARD_PROFICIENCY_IDS.contains(&"martial_ranged"));
        assert!(STANDARD_PROFICIENCY_IDS.contains(&"blunt_weapon"));
        assert!(STANDARD_PROFICIENCY_IDS.contains(&"unarmed"));
        assert!(STANDARD_PROFICIENCY_IDS.contains(&"light_armor"));
        assert!(STANDARD_PROFICIENCY_IDS.contains(&"medium_armor"));
        assert!(STANDARD_PROFICIENCY_IDS.contains(&"heavy_armor"));
        assert!(STANDARD_PROFICIENCY_IDS.contains(&"shield"));
        assert!(STANDARD_PROFICIENCY_IDS.contains(&"arcane_item"));
        assert!(STANDARD_PROFICIENCY_IDS.contains(&"divine_item"));
    }

    #[test]
    fn test_standard_item_tags_complete() {
        // Ensure all expected tags are in the list
        assert!(STANDARD_ITEM_TAGS.contains(&"large_weapon"));
        assert!(STANDARD_ITEM_TAGS.contains(&"two_handed"));
        assert!(STANDARD_ITEM_TAGS.contains(&"heavy_armor"));
        assert!(STANDARD_ITEM_TAGS.contains(&"elven_crafted"));
        assert!(STANDARD_ITEM_TAGS.contains(&"dwarven_crafted"));
        assert!(STANDARD_ITEM_TAGS.contains(&"requires_strength"));
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

    #[test]
    fn test_validate_npc_placement_reference_valid() {
        let mut available = std::collections::HashSet::new();
        available.insert("merchant_bob".to_string());
        available.insert("innkeeper_mary".to_string());

        assert!(validate_npc_placement_reference("merchant_bob", &available).is_ok());
    }

    #[test]
    fn test_validate_npc_placement_reference_invalid() {
        let mut available = std::collections::HashSet::new();
        available.insert("merchant_bob".to_string());

        let result = validate_npc_placement_reference("unknown_npc", &available);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown NPC ID"));
    }

    #[test]
    fn test_validate_npc_placement_reference_empty() {
        let available = std::collections::HashSet::new();
        let result = validate_npc_placement_reference("", &available);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_npc_dialogue_reference_valid() {
        let mut available = std::collections::HashSet::new();
        available.insert(1);
        available.insert(5);

        assert!(validate_npc_dialogue_reference(Some(1), &available).is_ok());
        assert!(validate_npc_dialogue_reference(None, &available).is_ok());
    }

    #[test]
    fn test_validate_npc_dialogue_reference_invalid() {
        let mut available = std::collections::HashSet::new();
        available.insert(1);

        let result = validate_npc_dialogue_reference(Some(99), &available);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown dialogue ID"));
    }

    #[test]
    fn test_validate_npc_quest_references_valid() {
        let mut available = std::collections::HashSet::new();
        available.insert(1);
        available.insert(2);
        available.insert(3);

        assert!(validate_npc_quest_references(&[1, 2], &available).is_ok());
        assert!(validate_npc_quest_references(&[], &available).is_ok());
    }

    #[test]
    fn test_validate_npc_quest_references_invalid() {
        let mut available = std::collections::HashSet::new();
        available.insert(1);
        available.insert(2);

        let result = validate_npc_quest_references(&[1, 99], &available);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown quest ID: 99"));
    }

    #[test]
    fn test_validate_npc_quest_references_multiple_invalid() {
        let mut available = std::collections::HashSet::new();
        available.insert(1);

        // Should fail on first invalid quest
        let result = validate_npc_quest_references(&[99, 100], &available);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown quest ID"));
    }
}
