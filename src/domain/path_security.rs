// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Path-security utilities for validating untrusted, campaign-relative paths.
//!
//! Campaign packages, their asset registries, and save-file names are untrusted
//! input: a downloaded campaign or a crafted save can supply a `filepath`, a
//! `campaign_id`, or a save `name` that escapes the intended directory via an
//! absolute path or `..` traversal. Joining such a value onto a base directory
//! without validation yields an arbitrary-file read/write primitive.
//!
//! This module centralizes the guard logic (originally implemented ad hoc in
//! `FontConfig::validate`) so every filesystem sink that joins external input
//! onto a base directory can reuse the same, unit-tested rules.
//!
//! # Rules
//!
//! - [`validate_campaign_relative_path`] rejects empty, absolute, and
//!   `..`-containing paths, and (when both paths exist) verifies via
//!   canonicalization that the resolved path does not escape the base through
//!   symlinks.
//! - [`validate_identifier`] enforces `^[A-Za-z0-9_-]+$` for campaign/CLI IDs
//!   used as a single directory name.
//! - [`validate_filename_component`] ensures a value is a single, safe path
//!   component (no separators, no `..`, not absolute) for use as a file stem.

use std::path::{Component, Path, PathBuf};
use thiserror::Error;

/// Errors returned when an untrusted path or identifier fails validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PathSecurityError {
    /// The candidate path or identifier was empty (or whitespace-only).
    #[error("path/identifier must not be empty")]
    Empty,

    /// The candidate path was absolute; only campaign-relative paths are allowed.
    #[error("path must be relative, not absolute: {0:?}")]
    Absolute(String),

    /// The candidate path contained a parent-directory (`..`) component.
    #[error("path must not contain a parent-directory ('..') component: {0:?}")]
    ParentTraversal(String),

    /// The candidate identifier contained characters outside `[A-Za-z0-9_-]`.
    #[error("identifier must match ^[A-Za-z0-9_-]+$ (got {0:?})")]
    InvalidIdentifier(String),

    /// The candidate was expected to be a single path component but was not.
    #[error("value must be a single path component (got {0:?})")]
    NotSingleComponent(String),

    /// The resolved path escaped the base directory (e.g. via a symlink).
    #[error("resolved path {resolved:?} escapes base directory {base:?}")]
    Escapes {
        /// Canonical base directory.
        base: String,
        /// Canonical resolved path that escaped the base.
        resolved: String,
    },
}

/// Validates that `candidate` is a safe, campaign-relative path under `base` and
/// returns the joined [`PathBuf`].
///
/// The candidate must be non-empty, relative, and free of any `..` component.
/// When both `base` and the resolved path exist on disk, the resolved path is
/// canonicalized and checked to remain within the canonicalized `base` (this
/// catches symlink-based escapes).
///
/// # Errors
///
/// - [`PathSecurityError::Empty`] if `candidate` is empty or whitespace-only.
/// - [`PathSecurityError::Absolute`] if `candidate` is an absolute path or
///   contains a root/prefix component.
/// - [`PathSecurityError::ParentTraversal`] if `candidate` contains `..`.
/// - [`PathSecurityError::Escapes`] if the resolved path escapes `base`.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use antares::domain::path_security::validate_campaign_relative_path;
///
/// let base = Path::new("campaigns/tutorial");
///
/// // A normal, campaign-relative path is accepted.
/// let ok = validate_campaign_relative_path(base, "data/creatures/goblin.ron").unwrap();
/// assert!(ok.ends_with("data/creatures/goblin.ron"));
///
/// // Traversal and absolute paths are rejected.
/// assert!(validate_campaign_relative_path(base, "../../etc/passwd").is_err());
/// assert!(validate_campaign_relative_path(base, "/etc/passwd").is_err());
/// ```
pub fn validate_campaign_relative_path(
    base: &Path,
    candidate: &str,
) -> Result<PathBuf, PathSecurityError> {
    if candidate.trim().is_empty() {
        return Err(PathSecurityError::Empty);
    }

    let candidate_path = Path::new(candidate);

    if candidate_path.is_absolute() {
        return Err(PathSecurityError::Absolute(candidate.to_string()));
    }

    for component in candidate_path.components() {
        match component {
            Component::ParentDir => {
                return Err(PathSecurityError::ParentTraversal(candidate.to_string()));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(PathSecurityError::Absolute(candidate.to_string()));
            }
            Component::Normal(_) | Component::CurDir => {}
        }
    }

    let resolved = base.join(candidate_path);

    // Defense-in-depth: when both paths exist, verify the canonical resolved
    // path is still contained within the canonical base. This catches escapes
    // that route through a symlink inside the campaign directory. When the
    // resolved path does not yet exist (or the base is not canonicalizable),
    // the lexical checks above already guarantee containment.
    if let (Ok(canonical_base), Ok(canonical_resolved)) =
        (base.canonicalize(), resolved.canonicalize())
    {
        if !canonical_resolved.starts_with(&canonical_base) {
            return Err(PathSecurityError::Escapes {
                base: canonical_base.display().to_string(),
                resolved: canonical_resolved.display().to_string(),
            });
        }
    }

    Ok(resolved)
}

/// Validates that `id` is a safe identifier usable as a single directory name.
///
/// Enforces `^[A-Za-z0-9_-]+$`: non-empty, ASCII alphanumerics plus `_` and `-`.
/// This is stricter than [`validate_campaign_relative_path`] and is intended for
/// values such as a campaign ID that are joined onto a base directory as a
/// single path segment.
///
/// # Errors
///
/// - [`PathSecurityError::Empty`] if `id` is empty.
/// - [`PathSecurityError::InvalidIdentifier`] if `id` contains any other character.
///
/// # Examples
///
/// ```
/// use antares::domain::path_security::validate_identifier;
///
/// assert!(validate_identifier("tutorial").is_ok());
/// assert!(validate_identifier("my-campaign_01").is_ok());
/// assert!(validate_identifier("../evil").is_err());
/// assert!(validate_identifier("a/b").is_err());
/// assert!(validate_identifier("").is_err());
/// ```
pub fn validate_identifier(id: &str) -> Result<(), PathSecurityError> {
    if id.is_empty() {
        return Err(PathSecurityError::Empty);
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(PathSecurityError::InvalidIdentifier(id.to_string()));
    }
    Ok(())
}

/// Validates that `name` is a single, safe path component (a file stem).
///
/// Rejects empty values, absolute paths, `..` traversal, and any value that
/// resolves to more than one path component (i.e. contains a separator). This
/// is intended for save-file names that are formatted into `"{name}.ron"` and
/// joined onto a saves directory.
///
/// # Errors
///
/// - [`PathSecurityError::Empty`] if `name` is empty or whitespace-only.
/// - [`PathSecurityError::ParentTraversal`] if `name` is `..`.
/// - [`PathSecurityError::NotSingleComponent`] / [`PathSecurityError::Absolute`]
///   if `name` contains a separator or is absolute.
///
/// # Examples
///
/// ```
/// use antares::domain::path_security::validate_filename_component;
///
/// assert!(validate_filename_component("save_20260101_120000").is_ok());
/// assert!(validate_filename_component("../escape").is_err());
/// assert!(validate_filename_component("sub/dir").is_err());
/// assert!(validate_filename_component("/abs").is_err());
/// ```
pub fn validate_filename_component(name: &str) -> Result<(), PathSecurityError> {
    if name.trim().is_empty() {
        return Err(PathSecurityError::Empty);
    }
    let path = Path::new(name);
    let mut components = path.components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        (Some(Component::ParentDir), _) => {
            Err(PathSecurityError::ParentTraversal(name.to_string()))
        }
        (Some(Component::RootDir), _) | (Some(Component::Prefix(_)), _) => {
            Err(PathSecurityError::Absolute(name.to_string()))
        }
        _ => Err(PathSecurityError::NotSingleComponent(name.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_campaign_relative_path_accepts_normal() {
        let base = Path::new("campaigns/tutorial");
        let resolved = validate_campaign_relative_path(base, "data/creatures/goblin.ron").unwrap();
        assert!(resolved.ends_with("data/creatures/goblin.ron"));
        assert!(resolved.starts_with(base));
    }

    #[test]
    fn test_validate_campaign_relative_path_rejects_parent_traversal() {
        let base = Path::new("campaigns/tutorial");
        assert_eq!(
            validate_campaign_relative_path(base, "../../etc/passwd"),
            Err(PathSecurityError::ParentTraversal(
                "../../etc/passwd".to_string()
            ))
        );
    }

    #[test]
    fn test_validate_campaign_relative_path_rejects_embedded_parent() {
        let base = Path::new("campaigns/tutorial");
        assert_eq!(
            validate_campaign_relative_path(base, "data/../../secret"),
            Err(PathSecurityError::ParentTraversal(
                "data/../../secret".to_string()
            ))
        );
    }

    #[test]
    fn test_validate_campaign_relative_path_rejects_absolute() {
        let base = Path::new("campaigns/tutorial");
        assert_eq!(
            validate_campaign_relative_path(base, "/etc/passwd"),
            Err(PathSecurityError::Absolute("/etc/passwd".to_string()))
        );
    }

    #[test]
    fn test_validate_campaign_relative_path_rejects_empty() {
        let base = Path::new("campaigns/tutorial");
        assert_eq!(
            validate_campaign_relative_path(base, "   "),
            Err(PathSecurityError::Empty)
        );
    }

    #[test]
    fn test_validate_identifier_accepts_valid() {
        assert!(validate_identifier("tutorial").is_ok());
        assert!(validate_identifier("my-campaign_01").is_ok());
        assert!(validate_identifier("ABC123").is_ok());
    }

    #[test]
    fn test_validate_identifier_rejects_traversal_and_separators() {
        assert_eq!(
            validate_identifier("../evil"),
            Err(PathSecurityError::InvalidIdentifier("../evil".to_string()))
        );
        assert_eq!(
            validate_identifier("a/b"),
            Err(PathSecurityError::InvalidIdentifier("a/b".to_string()))
        );
        assert_eq!(validate_identifier(""), Err(PathSecurityError::Empty));
    }

    #[test]
    fn test_validate_filename_component_accepts_timestamp_name() {
        assert!(validate_filename_component("save_20260101_120000").is_ok());
    }

    #[test]
    fn test_validate_filename_component_rejects_traversal_and_separators() {
        assert_eq!(
            validate_filename_component("../escape"),
            Err(PathSecurityError::ParentTraversal("../escape".to_string()))
        );
        assert_eq!(
            validate_filename_component("sub/dir"),
            Err(PathSecurityError::NotSingleComponent("sub/dir".to_string()))
        );
        assert_eq!(
            validate_filename_component("/abs"),
            Err(PathSecurityError::Absolute("/abs".to_string()))
        );
        assert_eq!(
            validate_filename_component(""),
            Err(PathSecurityError::Empty)
        );
    }
}
