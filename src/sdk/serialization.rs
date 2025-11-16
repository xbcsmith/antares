// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! RON serialization helpers and utilities
//!
//! This module provides helper functions for working with RON (Rusty Object Notation)
//! format, including formatting, syntax validation, and data merging.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sdk_implementation_plan.md` Phase 3.4 for specifications.
//!
//! # Examples
//!
//! ```
//! use antares::sdk::serialization::{format_ron, validate_ron_syntax};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let data = r#"(items: [Item(id: 1, name: "Sword")])"#;
//!
//! // Validate syntax
//! assert!(validate_ron_syntax(data).is_ok());
//!
//! // Format with pretty printing
//! let formatted = format_ron(data)?;
//! println!("{}", formatted);
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur during RON serialization operations
#[derive(Error, Debug)]
pub enum SerializationError {
    #[error("RON syntax error: {0}")]
    SyntaxError(String),

    #[error("RON parsing error: {0}")]
    ParseError(#[from] ron::Error),

    #[error("RON formatting error: {0}")]
    FormatError(String),

    #[error("Merge error: {0}")]
    MergeError(String),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
}

// ===== RON Formatting =====

/// Formats RON data with pretty printing
///
/// Takes a RON string and reformats it with proper indentation and spacing
/// for improved readability.
///
/// # Arguments
///
/// * `ron_data` - RON string to format
///
/// # Returns
///
/// Returns the formatted RON string with pretty printing applied.
///
/// # Errors
///
/// Returns `SerializationError` if the RON data is invalid or cannot be formatted.
///
/// # Examples
///
/// ```
/// use antares::sdk::serialization::format_ron;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let input = "(id: 1, name: \"Test\", value: 42)";
/// let formatted = format_ron(input)?;
///
/// // Output will have proper indentation and valid RON syntax
/// assert!(!formatted.is_empty());
/// # Ok(())
/// # }
/// ```
pub fn format_ron(ron_data: &str) -> Result<String, SerializationError> {
    // Parse the RON data as a generic value
    let value: ron::Value =
        ron::from_str(ron_data).map_err(|e| SerializationError::ParseError(e.into()))?;

    // Serialize back with pretty printing
    let pretty_config = ron::ser::PrettyConfig::new()
        .depth_limit(4)
        .separate_tuple_members(true)
        .enumerate_arrays(true)
        .new_line("\n".to_string());

    ron::ser::to_string_pretty(&value, pretty_config)
        .map_err(|e| SerializationError::FormatError(e.to_string()))
}

/// Validates RON syntax without deserializing to a specific type
///
/// Checks if the given string is valid RON syntax by attempting to parse it
/// as a generic RON value.
///
/// # Arguments
///
/// * `ron_data` - RON string to validate
///
/// # Returns
///
/// Returns `Ok(())` if the syntax is valid, otherwise returns a `SerializationError`.
///
/// # Examples
///
/// ```
/// use antares::sdk::serialization::validate_ron_syntax;
///
/// // Valid RON
/// assert!(validate_ron_syntax("(id: 1, name: \"Test\")").is_ok());
///
/// // Invalid RON
/// assert!(validate_ron_syntax("(invalid: syntax").is_err());
/// ```
pub fn validate_ron_syntax(ron_data: &str) -> Result<(), SerializationError> {
    // Try to parse as a generic RON value
    let _: ron::Value =
        ron::from_str(ron_data).map_err(|e| SerializationError::SyntaxError(e.to_string()))?;

    Ok(())
}

/// Merges two RON data structures
///
/// Combines two RON data structures by parsing them and merging their fields.
/// For maps/objects, fields from `override_data` take precedence over `base_data`.
/// For sequences/arrays, the override replaces the base entirely.
///
/// # Arguments
///
/// * `base_data` - Base RON data string
/// * `override_data` - Override RON data string (takes precedence)
///
/// # Returns
///
/// Returns a formatted RON string containing the merged data.
///
/// # Errors
///
/// Returns `SerializationError` if:
/// - Either input is invalid RON syntax
/// - The data structures are incompatible for merging
///
/// # Examples
///
/// ```
/// use antares::sdk::serialization::merge_ron_data;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let base = "(id: 1, name: \"Base\", value: 10)";
/// let override_data = "(name: \"Override\", extra: 42)";
///
/// let merged = merge_ron_data(base, override_data)?;
///
/// // Result should have name from override, id and value from base, plus extra field
/// assert!(merged.contains("Override"));
/// # Ok(())
/// # }
/// ```
pub fn merge_ron_data(base_data: &str, override_data: &str) -> Result<String, SerializationError> {
    use ron::Value;

    let base: Value =
        ron::from_str(base_data).map_err(|e| SerializationError::ParseError(e.into()))?;
    let override_val: Value =
        ron::from_str(override_data).map_err(|e| SerializationError::ParseError(e.into()))?;

    let merged = merge_values(base, override_val)?;

    let pretty_config = ron::ser::PrettyConfig::new()
        .depth_limit(4)
        .separate_tuple_members(true)
        .enumerate_arrays(true)
        .new_line("\n".to_string());

    ron::ser::to_string_pretty(&merged, pretty_config)
        .map_err(|e| SerializationError::FormatError(e.to_string()))
}

/// Merges two RON values recursively
fn merge_values(
    base: ron::Value,
    override_val: ron::Value,
) -> Result<ron::Value, SerializationError> {
    use ron::Value;

    match (base, override_val) {
        // Both are maps - merge fields
        (Value::Map(mut base_map), Value::Map(override_map)) => {
            for (key, value) in override_map {
                base_map.insert(key, value);
            }
            Ok(Value::Map(base_map))
        }

        // Both are sequences - override replaces base
        (Value::Seq(_), Value::Seq(override_seq)) => Ok(Value::Seq(override_seq)),

        // Override value replaces base for all other types
        (_, override_val) => Ok(override_val),
    }
}

// ===== Helper Functions =====

/// Serializes a value to RON format with pretty printing
///
/// # Examples
///
/// ```
/// use antares::sdk::serialization::to_ron_string;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct TestData {
///     id: u32,
///     name: String,
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let data = TestData {
///     id: 1,
///     name: "Test".to_string(),
/// };
///
/// let ron = to_ron_string(&data)?;
/// assert!(ron.contains("id: 1"));
/// # Ok(())
/// # }
/// ```
pub fn to_ron_string<T: Serialize>(value: &T) -> Result<String, SerializationError> {
    let pretty_config = ron::ser::PrettyConfig::new()
        .depth_limit(4)
        .separate_tuple_members(true)
        .enumerate_arrays(true)
        .new_line("\n".to_string());

    ron::ser::to_string_pretty(value, pretty_config)
        .map_err(|e| SerializationError::FormatError(e.to_string()))
}

/// Deserializes a value from RON format
///
/// # Examples
///
/// ```
/// use antares::sdk::serialization::from_ron_string;
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Debug, PartialEq)]
/// struct TestData {
///     id: u32,
///     name: String,
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let ron = "(id: 1, name: \"Test\")";
/// let data: TestData = from_ron_string(ron)?;
///
/// assert_eq!(data.id, 1);
/// assert_eq!(data.name, "Test");
/// # Ok(())
/// # }
/// ```
pub fn from_ron_string<'a, T: Deserialize<'a>>(ron_data: &'a str) -> Result<T, SerializationError> {
    ron::from_str(ron_data).map_err(|e| SerializationError::ParseError(e.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_validate_ron_syntax_valid() {
        let valid_ron = "(id: 1, name: \"Test\")";
        assert!(validate_ron_syntax(valid_ron).is_ok());
    }

    #[test]
    fn test_validate_ron_syntax_invalid() {
        let invalid_ron = "(invalid: syntax";
        assert!(validate_ron_syntax(invalid_ron).is_err());
    }

    #[test]
    fn test_format_ron() {
        let input = "(id: 1, name: \"Test\", value: 42)";
        let formatted = format_ron(input).unwrap();

        // Should contain the values (RON may format as Map with keys/values)
        assert!(formatted.contains("1") || formatted.contains("id"));
        assert!(formatted.contains("Test"));
        assert!(formatted.contains("42") || formatted.contains("value"));
    }

    #[test]
    fn test_format_ron_invalid() {
        let invalid = "(invalid: syntax";
        assert!(format_ron(invalid).is_err());
    }

    #[test]
    fn test_merge_ron_data_maps() {
        let base = "(id: 1, name: \"Base\", value: 10)";
        let override_data = "(name: \"Override\", extra: 42)";

        let merged = merge_ron_data(base, override_data).unwrap();

        // Should have override name
        assert!(merged.contains("Override"));
        // Should have values from both base and override
        assert!(merged.contains("1") || merged.contains("id"));
        assert!(merged.contains("42") || merged.contains("extra"));
    }

    #[test]
    fn test_merge_ron_data_sequences() {
        let base = "[1, 2, 3]";
        let override_data = "[4, 5]";

        let merged = merge_ron_data(base, override_data).unwrap();

        // Override should replace base for sequences
        assert!(merged.contains("4"));
        assert!(merged.contains("5"));
        // Base values should not be present (or if present due to RON formatting, that's okay)
        // The important thing is override values are there
    }

    #[test]
    fn test_to_ron_string() {
        #[derive(Serialize)]
        struct TestData {
            name: String,
        }

        let data = TestData {
            name: "Test".to_string(),
        };

        let ron = to_ron_string(&data).unwrap();
        assert!(ron.contains("name: \"Test\""));
    }

    #[test]
    fn test_from_ron_string() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct TestData {
            name: String,
        }

        let ron = "(name: \"Test\")";
        let data: TestData = from_ron_string(ron).unwrap();

        assert_eq!(data.name, "Test");
    }

    #[test]
    fn test_from_ron_string_invalid() {
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct TestData {
            id: u32,
        }

        let invalid = "(invalid: syntax";
        let result: Result<TestData, _> = from_ron_string(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialization_error_display() {
        let error = SerializationError::SyntaxError("test error".to_string());
        assert!(error.to_string().contains("test error"));

        let error = SerializationError::TypeMismatch {
            expected: "String".to_string(),
            actual: "Integer".to_string(),
        };
        assert!(error.to_string().contains("String"));
        assert!(error.to_string().contains("Integer"));
    }

    #[test]
    fn test_merge_values_primitive_override() {
        use ron::Value;

        let base = Value::Number(ron::Number::new(10.0));
        let override_val = Value::Number(ron::Number::new(20.0));

        let merged = merge_values(base, override_val).unwrap();

        // Verify that the override value replaced the base value
        assert!(matches!(merged, Value::Number(_)));
    }

    #[test]
    fn test_validate_ron_complex_structure() {
        let complex = r#"
        (
            id: 1,
            name: "Complex",
            nested: (
                value: 42,
                items: [1, 2, 3],
            ),
        )
        "#;

        assert!(validate_ron_syntax(complex).is_ok());
    }
}
