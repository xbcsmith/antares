// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Enhanced error formatting with actionable suggestions
//!
//! Provides user-friendly error messages with context, suggestions, and
//! examples to help SDK tool users quickly understand and fix issues.
//!
//! # Examples
//!
//! ```
//! use antares::sdk::error_formatter::{ErrorFormatter, ErrorContext};
//! use antares::sdk::validation::ValidationError;
//! use antares::domain::types::ItemId;
//!
//! let error = ValidationError::MissingItem {
//!     context: "Monster loot table".to_string(),
//!     item_id: ItemId::from(99),
//! };
//!
//! let context = ErrorContext {
//!     file_path: Some("data/monsters.ron".into()),
//!     line_number: Some(45),
//!     available_ids: vec![1, 2, 3, 10, 11],
//! };
//!
//! let formatter = ErrorFormatter::new(true); // colored output
//! let formatted = formatter.format_validation_error(&error, Some(&context));
//! println!("{}", formatted);
//! ```

use crate::sdk::validation::ValidationError;

use std::path::PathBuf;

// ===== Constants =====

const ERROR_COLOR: &str = "\x1b[31m"; // Red
const WARNING_COLOR: &str = "\x1b[33m"; // Yellow
const INFO_COLOR: &str = "\x1b[36m"; // Cyan
const SUCCESS_COLOR: &str = "\x1b[32m"; // Green
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";

// ===== Error Context =====

/// Additional context for error formatting
#[derive(Debug, Clone, Default)]
pub struct ErrorContext {
    /// File where error occurred
    pub file_path: Option<PathBuf>,

    /// Line number in file
    pub line_number: Option<usize>,

    /// List of available/valid IDs (for reference errors)
    pub available_ids: Vec<u32>,
}

// ===== Error Formatter =====

/// Formats errors with color, context, and suggestions
pub struct ErrorFormatter {
    use_color: bool,
}

impl ErrorFormatter {
    /// Creates a new error formatter
    ///
    /// # Arguments
    ///
    /// * `use_color` - Whether to use ANSI color codes
    pub fn new(use_color: bool) -> Self {
        Self { use_color }
    }

    /// Formats a validation error with enhanced context
    pub fn format_validation_error(
        &self,
        error: &ValidationError,
        context: Option<&ErrorContext>,
    ) -> String {
        let mut output = String::new();

        // Header
        self.append_header(&mut output, error);

        // Context (file/line)
        if let Some(ctx) = context {
            self.append_context(&mut output, ctx);
        }

        // Error details
        self.append_error_details(&mut output, error);

        // Suggestions
        self.append_suggestions(&mut output, error, context);

        output
    }

    /// Formats multiple errors as a report
    pub fn format_error_report(&self, errors: &[ValidationError]) -> String {
        let mut output = String::new();

        let error_count = errors.iter().filter(|e| e.is_error()).count();
        let warning_count = errors.iter().filter(|e| e.is_warning()).count();

        // Summary header
        output.push_str(&self.colorize(BOLD, "Validation Report\n"));
        output.push_str("─────────────────\n");

        if error_count > 0 {
            output.push_str(&self.colorize(ERROR_COLOR, &format!("✗ {} errors\n", error_count)));
        }

        if warning_count > 0 {
            output.push_str(
                &self.colorize(WARNING_COLOR, &format!("⚠ {} warnings\n", warning_count)),
            );
        }

        output.push('\n');

        // Individual errors
        for (i, error) in errors.iter().enumerate() {
            output.push_str(&format!("{}. ", i + 1));
            output.push_str(&self.format_validation_error(error, None));
            output.push('\n');
        }

        output
    }

    // === Private Helper Methods ===

    fn append_header(&self, output: &mut String, error: &ValidationError) {
        let (symbol, color) = if error.is_error() {
            ("✗", ERROR_COLOR)
        } else if error.is_warning() {
            ("⚠", WARNING_COLOR)
        } else {
            ("ℹ", INFO_COLOR)
        };

        let severity = format!("{:?}", error.severity());
        output.push_str(&self.colorize(color, &format!("{} {} ", symbol, severity.to_uppercase())));
    }

    fn append_context(&self, output: &mut String, context: &ErrorContext) {
        if let Some(ref path) = context.file_path {
            output.push_str(&self.colorize(DIM, "in "));
            output.push_str(&format!("{}", path.display()));

            if let Some(line) = context.line_number {
                output.push_str(&self.colorize(DIM, &format!(":{}", line)));
            }

            output.push('\n');
        }
    }

    fn append_error_details(&self, output: &mut String, error: &ValidationError) {
        output.push_str(&format!("{}\n", error));
    }

    fn append_suggestions(
        &self,
        output: &mut String,
        error: &ValidationError,
        context: Option<&ErrorContext>,
    ) {
        let suggestions = self.get_suggestions(error, context);

        if suggestions.is_empty() {
            return;
        }

        output.push('\n');
        output.push_str(&self.colorize(BOLD, "Suggestions:\n"));

        for suggestion in suggestions {
            output.push_str(&format!("  • {}\n", suggestion));
        }
    }

    fn get_suggestions(
        &self,
        error: &ValidationError,
        context: Option<&ErrorContext>,
    ) -> Vec<String> {
        match error {
            ValidationError::MissingClass {
                class_id,
                context: _,
            } => {
                let mut suggestions = vec![
                    format!("Run 'class_editor' to create class with ID '{class_id}'"),
                    format!("Check that 'data/classes.ron' contains class '{class_id}'"),
                ];

                if let Some(ctx) = context {
                    if !ctx.available_ids.is_empty() {
                        suggestions.push(format!(
                            "Available class IDs: {}",
                            self.format_id_list(&ctx.available_ids)
                        ));
                    }
                }

                suggestions
            }

            ValidationError::MissingRace { race_id, .. } => {
                vec![
                    format!("Run 'race_editor' to create race with ID '{race_id}'"),
                    format!("Check that 'data/races.ron' contains race '{race_id}'"),
                    "Use 'campaign_validator --verbose' to see all available races".to_string(),
                ]
            }

            ValidationError::MissingItem {
                item_id,
                context: _,
            } => {
                let mut suggestions = vec![
                    format!("Run 'item_editor' to create item with ID {item_id}"),
                    format!("Check that 'data/items.ron' contains item {item_id}"),
                ];

                if let Some(context) = context {
                    if !context.available_ids.is_empty() {
                        let similar =
                            self.find_similar_id(u32::from(*item_id), &context.available_ids);
                        if let Some(similar_id) = similar {
                            suggestions.insert(
                                0,
                                format!(
                                    "Did you mean item ID {}? (similar to {})",
                                    similar_id, item_id
                                ),
                            );
                        }
                    }
                }

                suggestions
            }

            ValidationError::MissingMonster { monster_id, map } => {
                vec![
                    format!("Add monster {monster_id} to 'data/monsters.ron'"),
                    format!("Or remove reference from map '{map}'"),
                    "Use 'campaign_validator --list-monsters' to see all defined monsters"
                        .to_string(),
                ]
            }

            ValidationError::MissingSpell { spell_id, .. } => {
                vec![
                    format!("Add spell {spell_id} to 'data/spells.ron'"),
                    "Spell definitions are loaded from the spells data file".to_string(),
                    format!("Check that spell {spell_id} is not a typo"),
                ]
            }

            ValidationError::DisconnectedMap { map_id } => {
                vec![
                    format!("Add a connection to map {map_id} from another map"),
                    "Use 'map_builder' to add exits/entrances".to_string(),
                    "Disconnected maps are unreachable by players".to_string(),
                    "Consider: Is this map meant to be a starting location?".to_string(),
                ]
            }

            ValidationError::DuplicateId { entity_type, id } => {
                vec![
                    format!("Each {entity_type} must have a unique ID"),
                    format!("Change one of the '{id}' IDs to a different value"),
                    "Run the validator with --fix-ids to auto-renumber (coming soon)".to_string(),
                ]
            }

            ValidationError::BalanceWarning { .. } => {
                vec![
                    "This is a balance suggestion, not a critical error".to_string(),
                    "Review the values and adjust if desired".to_string(),
                    "Use --no-balance-checks to skip balance validation".to_string(),
                ]
            }
        }
    }

    fn find_similar_id(&self, target: u32, available: &[u32]) -> Option<u32> {
        // Find closest ID by numeric distance
        available
            .iter()
            .min_by_key(|&&id| id.abs_diff(target))
            .copied()
    }

    fn format_id_list(&self, ids: &[u32]) -> String {
        if ids.is_empty() {
            return "none".to_string();
        }

        if ids.len() <= 5 {
            ids.iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            format!("{}, {} (and {} more)", ids[0], ids[1], ids.len() - 2)
        }
    }

    fn colorize(&self, color: &str, text: &str) -> String {
        if self.use_color {
            format!("{}{}{}", color, text, RESET)
        } else {
            text.to_string()
        }
    }
}

// ===== Progress Reporter =====

/// Reports progress during long operations
pub struct ProgressReporter {
    use_color: bool,
    current_step: usize,
    total_steps: usize,
}

impl ProgressReporter {
    /// Creates a new progress reporter
    pub fn new(total_steps: usize, use_color: bool) -> Self {
        Self {
            use_color,
            current_step: 0,
            total_steps,
        }
    }

    /// Reports progress for a step
    pub fn step(&mut self, message: &str) {
        self.current_step += 1;
        let percentage = (self.current_step * 100) / self.total_steps;

        let prefix = if self.use_color {
            format!("{}[{:3}%]{}", DIM, percentage, RESET)
        } else {
            format!("[{:3}%]", percentage)
        };

        println!("{} {}", prefix, message);
    }

    /// Reports successful completion
    pub fn success(&self, message: &str) {
        if self.use_color {
            println!("{}✓ {}{}", SUCCESS_COLOR, message, RESET);
        } else {
            println!("✓ {}", message);
        }
    }

    /// Reports an error
    pub fn error(&self, message: &str) {
        if self.use_color {
            eprintln!("{}✗ {}{}", ERROR_COLOR, message, RESET);
        } else {
            eprintln!("✗ {}", message);
        }
    }
}

// ===== Helper Functions =====

/// Creates a helpful error message for file not found errors
pub fn format_file_not_found(path: &str, suggestions: &[&str]) -> String {
    let mut msg = format!("File not found: {}\n", path);
    msg.push_str("\nSuggestions:\n");
    for suggestion in suggestions {
        msg.push_str(&format!("  • {}\n", suggestion));
    }
    msg
}

/// Creates a helpful error message for parsing errors
pub fn format_parse_error(file: &str, line: usize, column: usize, message: &str) -> String {
    format!(
        "Parse error in {}:{}:{}\n  {}\n\nHint: Check RON syntax - common issues:\n  • Missing commas between fields\n  • Unmatched parentheses or brackets\n  • Incorrect string quotes",
        file, line, column, message
    )
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::ItemId;

    #[test]
    fn test_formatter_without_color() {
        let formatter = ErrorFormatter::new(false);
        let error = ValidationError::MissingItem {
            context: "Test".to_string(),
            item_id: ItemId::from(99),
        };

        let formatted = formatter.format_validation_error(&error, None);
        assert!(formatted.contains("99"));
        assert!(formatted.contains("Suggestions"));
        // Should not contain ANSI codes
        assert!(!formatted.contains("\x1b["));
    }

    #[test]
    fn test_formatter_with_context() {
        let formatter = ErrorFormatter::new(false);
        let error = ValidationError::MissingItem {
            context: "Test".to_string(),
            item_id: ItemId::from(99),
        };

        let context = ErrorContext {
            file_path: Some("test.ron".into()),
            line_number: Some(42),
            available_ids: vec![1, 2, 3],
        };

        let formatted = formatter.format_validation_error(&error, Some(&context));
        assert!(formatted.contains("test.ron"));
        assert!(formatted.contains("42"));
        assert!(formatted.contains("similar"));
    }

    #[test]
    fn test_find_similar_id() {
        let formatter = ErrorFormatter::new(false);

        let available = vec![10, 20, 30, 40];
        assert_eq!(formatter.find_similar_id(25, &available), Some(20));
        assert_eq!(formatter.find_similar_id(15, &available), Some(10));
        assert_eq!(formatter.find_similar_id(50, &available), Some(40));
    }

    #[test]
    fn test_format_id_list() {
        let formatter = ErrorFormatter::new(false);

        assert_eq!(formatter.format_id_list(&[]), "none");
        assert_eq!(formatter.format_id_list(&[1]), "1");
        assert_eq!(formatter.format_id_list(&[1, 2, 3]), "1, 2, 3");
        assert_eq!(
            formatter.format_id_list(&[1, 2, 3, 4, 5, 6, 7]),
            "1, 2 (and 5 more)"
        );
    }

    #[test]
    fn test_progress_reporter() {
        let mut reporter = ProgressReporter::new(3, false);
        reporter.step("Step 1");
        reporter.step("Step 2");
        reporter.step("Step 3");
        reporter.success("Complete");
        // Just testing it doesn't panic
    }

    #[test]
    fn test_format_file_not_found() {
        let msg = format_file_not_found("missing.ron", &["Check the file path", "Create the file"]);
        assert!(msg.contains("missing.ron"));
        assert!(msg.contains("Suggestions"));
        assert!(msg.contains("Check the file path"));
    }

    #[test]
    fn test_format_parse_error() {
        let msg = format_parse_error("test.ron", 10, 5, "unexpected token");
        assert!(msg.contains("test.ron:10:5"));
        assert!(msg.contains("unexpected token"));
        assert!(msg.contains("RON syntax"));
    }

    #[test]
    fn test_error_report() {
        let formatter = ErrorFormatter::new(false);
        let errors = vec![
            ValidationError::MissingItem {
                context: "Test 1".to_string(),
                item_id: ItemId::from(1),
            },
            ValidationError::MissingItem {
                context: "Test 2".to_string(),
                item_id: ItemId::from(2),
            },
        ];

        let report = formatter.format_error_report(&errors);
        assert!(report.contains("Validation Report"));
        assert!(report.contains("2 errors"));
        assert!(report.contains("1."));
        assert!(report.contains("2."));
    }
}
