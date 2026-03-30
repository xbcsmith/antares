// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Domain-level validation error types
//!
//! This module provides a unified `ValidationError` enum that replaces ad-hoc
//! `Result<(), String>` returns across domain validation functions. All
//! validation helpers in `domain::visual`, `domain::dialogue`, `domain::races`,
//! `application::quests`, and `application::resources` should return
//! `Result<(), ValidationError>` instead of `Result<(), String>`.
//!
//! # Variant Selection Guide
//!
//! | Situation | Variant |
//! |---|---|
//! | A referenced ID (node, bone, state) doesn't exist | `MissingReference` |
//! | A required string field is empty | `EmptyField` |
//! | A numeric value is outside its allowed range | `OutOfRange` |
//! | Two parallel collections differ in length | `CountMismatch` |
//! | A float is NaN or infinite | `NotFinite` |
//! | A structural invariant is broken (cycles, self-parents) | `Structural` |
//! | A child element failed its own validation | `Nested` |
//! | A looked-up entity (quest, NPC, map) was not found | `NotFound` |
//! | A domain precondition blocks the operation | `PreconditionFailed` |
//! | The caller lacks a required resource (gold, gems) | `InsufficientResources` |

use thiserror::Error;

/// Errors produced by domain-level validation of data definitions and
/// precondition checks on domain operations (quest start, resurrection, etc.).
///
/// Each variant carries a human-readable message that is displayed via the
/// `Display` impl derived by `thiserror`.
///
/// # Examples
///
/// ```
/// use antares::domain::validation::ValidationError;
///
/// let err = ValidationError::MissingReference(
///     "Node 0 choice 2 references non-existent node 99".to_string(),
/// );
/// assert!(err.to_string().contains("non-existent node 99"));
/// ```
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// A reference to a named entity (node, state, bone, etc.) that does not
    /// exist in the parent structure.
    #[error("{0}")]
    MissingReference(String),

    /// A required field is empty (e.g. animation name, blend parameter).
    #[error("{0}")]
    EmptyField(String),

    /// A numeric value is outside its valid range (e.g. resistance > 100,
    /// negative duration, weight outside \[0, 1\]).
    #[error("{0}")]
    OutOfRange(String),

    /// Two parallel collections that must match in length do not
    /// (e.g. mesh count vs. transform count).
    #[error("{0}")]
    CountMismatch(String),

    /// A floating-point value is NaN or infinite where a finite value is
    /// required (vertex coordinates, UV coordinates, normals).
    #[error("{0}")]
    NotFinite(String),

    /// A structural invariant is violated (circular bone references,
    /// degenerate triangles, self-parent bones, etc.).
    #[error("{0}")]
    Structural(String),

    /// A child element failed its own validation. The `context` string
    /// identifies which child (e.g. `"Mesh 2"`), and `source` carries the
    /// underlying error.
    #[error("{context}: {source}")]
    Nested {
        /// Human-readable label for the child element that failed.
        context: String,
        /// The validation error from the child element.
        source: Box<ValidationError>,
    },

    /// A looked-up entity (quest, NPC, map, condition) was not found in the
    /// content database.
    #[error("{0}")]
    NotFound(String),

    /// A domain precondition blocks the requested operation (e.g. permadeath
    /// prevents resurrection, character is not dead).
    #[error("{0}")]
    PreconditionFailed(String),

    /// The party lacks a required resource to complete the operation
    /// (gold, gems, etc.).
    #[error("{0}")]
    InsufficientResources(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_reference_display() {
        let err = ValidationError::MissingReference("Root node 5 does not exist".to_string());
        assert_eq!(err.to_string(), "Root node 5 does not exist");
    }

    #[test]
    fn test_empty_field_display() {
        let err = ValidationError::EmptyField("Animation name cannot be empty".to_string());
        assert_eq!(err.to_string(), "Animation name cannot be empty");
    }

    #[test]
    fn test_out_of_range_display() {
        let err =
            ValidationError::OutOfRange("Magic resistance 150 exceeds maximum of 100".to_string());
        assert!(err.to_string().contains("150"));
    }

    #[test]
    fn test_count_mismatch_display() {
        let err = ValidationError::CountMismatch(
            "Mesh count (3) must match transform count (2)".to_string(),
        );
        assert!(err.to_string().contains("3"));
    }

    #[test]
    fn test_not_finite_display() {
        let err =
            ValidationError::NotFinite("Vertex 0 coordinate 1 is not finite: NaN".to_string());
        assert!(err.to_string().contains("NaN"));
    }

    #[test]
    fn test_structural_display() {
        let err =
            ValidationError::Structural("Bone 'spine' has circular parent reference".to_string());
        assert!(err.to_string().contains("spine"));
    }

    #[test]
    fn test_nested_display() {
        let inner = ValidationError::OutOfRange("Scale must be positive, got -1".to_string());
        let outer = ValidationError::Nested {
            context: "Mesh 2".to_string(),
            source: Box::new(inner),
        };
        assert_eq!(outer.to_string(), "Mesh 2: Scale must be positive, got -1");
    }

    #[test]
    fn test_not_found_display() {
        let err = ValidationError::NotFound("Quest 42 not found in content database".to_string());
        assert!(err.to_string().contains("42"));
    }

    #[test]
    fn test_precondition_failed_display() {
        let err = ValidationError::PreconditionFailed(
            "Resurrection is not allowed in this campaign (permadeath enabled).".to_string(),
        );
        assert!(err.to_string().contains("permadeath"));
    }

    #[test]
    fn test_insufficient_resources_display() {
        let err = ValidationError::InsufficientResources(
            "Insufficient gold: resurrection costs 100 gold but the party only has 50".to_string(),
        );
        assert!(err.to_string().contains("100"));
    }

    #[test]
    fn test_variants_are_not_equal() {
        let a = ValidationError::EmptyField("x".to_string());
        let b = ValidationError::OutOfRange("x".to_string());
        assert_ne!(a, b);
    }

    #[test]
    fn test_same_variant_same_message_equal() {
        let a = ValidationError::NotFound("gone".to_string());
        let b = ValidationError::NotFound("gone".to_string());
        assert_eq!(a, b);
    }

    #[test]
    fn test_clone() {
        let err = ValidationError::Structural("cycle detected".to_string());
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_debug_format() {
        let err = ValidationError::EmptyField("name".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("EmptyField"));
    }

    #[test]
    fn test_nested_source_preserved() {
        let inner = ValidationError::MissingReference("bone 99 not found".to_string());
        let outer = ValidationError::Nested {
            context: "Track 3".to_string(),
            source: Box::new(inner.clone()),
        };
        if let ValidationError::Nested { source, .. } = &outer {
            assert_eq!(source.as_ref(), &inner);
        } else {
            panic!("Expected Nested variant");
        }
    }
}
