// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared editor context passed to all editor `show()` methods.
//!
//! [`EditorContext`] bundles the five parameters that every editor `show()` method
//! previously received individually, collapsing them into a single `&mut` reference so
//! that method signatures stay under five parameters and the
//! `#[allow(clippy::too_many_arguments)]` suppressions can be removed.
//!
//! # Design rationale
//!
//! Almost every editor `show()` method accepted the following five parameters:
//!
//! | Parameter              | Type                  | Purpose                                      |
//! |------------------------|-----------------------|----------------------------------------------|
//! | `campaign_dir`         | `Option<&PathBuf>`    | Resolve absolute paths for load/save         |
//! | `data_file`            | `&str`                | Relative path of the data file (e.g. `"data/items.ron"`) |
//! | `unsaved_changes`      | `&mut bool`           | Mark the campaign dirty after any mutation   |
//! | `status_message`       | `&mut String`         | One-line feedback shown in the status bar    |
//! | `file_load_merge_mode` | `&mut bool`           | Whether file-load merges or replaces         |
//!
//! Collapsing these into [`EditorContext`] reduces most `show()` signatures from
//! 8–10 parameters to 3–5.

use std::path::PathBuf;

/// Shared mutable context passed to every editor `show()` method.
///
/// Bundles five commonly-threaded parameters so that editor method signatures
/// stay under the Clippy `too_many_arguments` threshold.
///
/// # Usage
///
/// Construct an `EditorContext` in the frame update loop and pass a `&mut`
/// reference to each editor's `show()` call:
///
/// ```no_run
/// use campaign_builder::editor_context::EditorContext;
/// use std::path::PathBuf;
///
/// let dir = PathBuf::from("/campaigns/my_campaign");
/// let mut unsaved = false;
/// let mut status = String::new();
/// let mut merge_mode = true;
///
/// let mut ctx = EditorContext::new(
///     Some(&dir),
///     "data/items.ron",
///     &mut unsaved,
///     &mut status,
///     &mut merge_mode,
/// );
///
/// // editor.show(ui, &mut items, &mut ctx);
/// assert!(!*ctx.unsaved_changes);
/// ```
pub struct EditorContext<'a> {
    /// Path to the open campaign's root directory, or `None` if no campaign is loaded.
    ///
    /// Editors join this path with [`data_file`][Self::data_file] to obtain the
    /// absolute path for file operations.
    pub campaign_dir: Option<&'a PathBuf>,

    /// Relative path to the data file managed by this editor instance.
    ///
    /// Examples: `"data/items.ron"`, `"data/spells.ron"`, `"data/maps/"`.
    pub data_file: &'a str,

    /// Set to `true` whenever the editor makes any change that has not yet been saved.
    ///
    /// Editors that mutate campaign data **must** set `*ctx.unsaved_changes = true`.
    pub unsaved_changes: &'a mut bool,

    /// One-line status bar message updated by editor operations.
    ///
    /// Overwritten with human-readable feedback after every save, load, or error.
    pub status_message: &'a mut String,

    /// Controls how file-load operations behave.
    ///
    /// * `true`  – merge loaded records into the existing list by ID (no data loss).
    /// * `false` – replace the entire list with the loaded data.
    ///
    /// Note: some editors manage their own internal `file_load_merge_mode` flag and
    /// do not read this field; it is present for API uniformity.
    pub file_load_merge_mode: &'a mut bool,
}

impl<'a> EditorContext<'a> {
    /// Constructs a new `EditorContext` from individual references.
    ///
    /// # Arguments
    ///
    /// * `campaign_dir`         – path to the open campaign root, or `None`
    /// * `data_file`            – relative data-file path for this editor
    /// * `unsaved_changes`      – dirty flag to set on every mutation
    /// * `status_message`       – status bar text to update after operations
    /// * `file_load_merge_mode` – whether file-load should merge (`true`) or replace
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::editor_context::EditorContext;
    /// use std::path::PathBuf;
    ///
    /// let dir = PathBuf::from("/campaigns/demo");
    /// let mut unsaved = false;
    /// let mut status = String::new();
    /// let mut merge = true;
    ///
    /// let ctx = EditorContext::new(
    ///     Some(&dir),
    ///     "data/items.ron",
    ///     &mut unsaved,
    ///     &mut status,
    ///     &mut merge,
    /// );
    ///
    /// assert_eq!(ctx.data_file, "data/items.ron");
    /// ```
    pub fn new(
        campaign_dir: Option<&'a PathBuf>,
        data_file: &'a str,
        unsaved_changes: &'a mut bool,
        status_message: &'a mut String,
        file_load_merge_mode: &'a mut bool,
    ) -> Self {
        Self {
            campaign_dir,
            data_file,
            unsaved_changes,
            status_message,
            file_load_merge_mode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_context_new_sets_all_fields() {
        let dir = PathBuf::from("/campaigns/test");
        let mut unsaved = false;
        let mut status = String::new();
        let mut merge = true;

        let ctx = EditorContext::new(
            Some(&dir),
            "data/items.ron",
            &mut unsaved,
            &mut status,
            &mut merge,
        );

        assert_eq!(ctx.campaign_dir, Some(&dir));
        assert_eq!(ctx.data_file, "data/items.ron");
        assert!(!*ctx.unsaved_changes);
        assert!(ctx.status_message.is_empty());
        assert!(*ctx.file_load_merge_mode);
    }

    #[test]
    fn test_editor_context_no_campaign_dir() {
        let mut unsaved = false;
        let mut status = String::from("initial");
        let mut merge = false;

        let ctx = EditorContext::new(
            None,
            "data/spells.ron",
            &mut unsaved,
            &mut status,
            &mut merge,
        );

        assert!(ctx.campaign_dir.is_none());
        assert_eq!(ctx.data_file, "data/spells.ron");
        assert!(!*ctx.file_load_merge_mode);
    }

    #[test]
    fn test_editor_context_mutation_through_reference() {
        let dir = PathBuf::from("/campaigns/mutation_test");
        let mut unsaved = false;
        let mut status = String::new();
        let mut merge = false;

        {
            let ctx = EditorContext::new(
                Some(&dir),
                "data/monsters.ron",
                &mut unsaved,
                &mut status,
                &mut merge,
            );

            // Simulate what an editor does when it saves successfully.
            *ctx.unsaved_changes = true;
            *ctx.status_message = "Saved monsters".to_string();
            *ctx.file_load_merge_mode = true;
        }

        assert!(unsaved);
        assert_eq!(status, "Saved monsters");
        assert!(merge);
    }

    #[test]
    fn test_editor_context_struct_literal_construction() {
        // Verify that struct-literal construction (used in lib.rs call sites) works.
        let dir = PathBuf::from("/campaigns/literal");
        let mut unsaved = false;
        let mut status = String::new();
        let mut merge = true;

        let ctx = EditorContext {
            campaign_dir: Some(&dir),
            data_file: "data/quests.ron",
            unsaved_changes: &mut unsaved,
            status_message: &mut status,
            file_load_merge_mode: &mut merge,
        };

        assert_eq!(ctx.data_file, "data/quests.ron");
        assert!(*ctx.file_load_merge_mode);
    }
}
