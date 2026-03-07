// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Undo/Redo stack for Item Mesh Editor actions.
//!
//! This module provides a simple, action-based undo/redo stack tailored to the
//! Item Mesh Editor. Unlike the trait-based [`crate::creature_undo_redo`]
//! system, this implementation stores concrete enum variants that the caller
//! applies directly — keeping the editor logic self-contained.
//!
//! # Design
//!
//! - [`ItemMeshUndoRedo`] holds two `Vec<ItemMeshEditAction>` stacks.
//! - [`push`](ItemMeshUndoRedo::push) appends to the undo stack and clears the
//!   redo stack (a new edit invalidates any forward history).
//! - [`undo`](ItemMeshUndoRedo::undo) pops from the undo stack, copies the
//!   action onto the redo stack, and returns the action so the caller can
//!   apply the `old` value to revert state.
//! - [`redo`](ItemMeshUndoRedo::redo) pops from the redo stack, copies the
//!   action onto the undo stack, and returns the action so the caller can
//!   re-apply the `new` value.
//!
//! # Examples
//!
//! ```
//! use campaign_builder::item_mesh_undo_redo::{ItemMeshUndoRedo, ItemMeshEditAction};
//!
//! let mut stack = ItemMeshUndoRedo::new();
//! stack.push(ItemMeshEditAction::SetScale { old: 1.0, new: 2.0 });
//!
//! assert!(stack.can_undo());
//! assert!(!stack.can_redo());
//!
//! let action = stack.undo().unwrap();
//! assert!(!stack.can_undo());
//! assert!(stack.can_redo());
//! ```

use antares::domain::visual::item_mesh::ItemMeshDescriptor;

// ─────────────────────────────────────────────────────────────────────────────
// ItemMeshEditAction
// ─────────────────────────────────────────────────────────────────────────────

/// A single reversible edit action in the Item Mesh Editor.
///
/// Each variant stores both the `old` (pre-edit) value and the `new`
/// (post-edit) value so that the caller can apply either direction without
/// extra bookkeeping.
///
/// # Examples
///
/// ```
/// use campaign_builder::item_mesh_undo_redo::ItemMeshEditAction;
///
/// let action = ItemMeshEditAction::SetScale { old: 1.0, new: 1.5 };
/// match action {
///     ItemMeshEditAction::SetScale { old, new } => {
///         assert_eq!(old, 1.0);
///         assert_eq!(new, 1.5);
///     }
///     _ => panic!("unexpected variant"),
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ItemMeshEditAction {
    /// Primary RGBA color was changed.
    SetPrimaryColor {
        /// Color before the change `[r, g, b, a]`
        old: [f32; 4],
        /// Color after the change `[r, g, b, a]`
        new: [f32; 4],
    },

    /// Accent RGBA color was changed.
    SetAccentColor {
        /// Color before the change `[r, g, b, a]`
        old: [f32; 4],
        /// Color after the change `[r, g, b, a]`
        new: [f32; 4],
    },

    /// World-space scale was changed.
    SetScale {
        /// Scale before the change
        old: f32,
        /// Scale after the change
        new: f32,
    },

    /// Emissive flag was toggled.
    SetEmissive {
        /// Flag value before the change
        old: bool,
        /// Flag value after the change
        new: bool,
    },

    /// Override-enabled flag was toggled.
    SetOverrideEnabled {
        /// Flag value before the change
        old: bool,
        /// Flag value after the change
        new: bool,
    },

    /// The entire descriptor was replaced (e.g. "Reset to Defaults").
    ReplaceDescriptor {
        /// Descriptor before the replacement
        old: ItemMeshDescriptor,
        /// Descriptor after the replacement
        new: ItemMeshDescriptor,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// ItemMeshUndoRedo
// ─────────────────────────────────────────────────────────────────────────────

/// Undo/redo stack for Item Mesh Editor operations.
///
/// Stores a list of [`ItemMeshEditAction`]s that have been performed (undo
/// stack) and a list of actions that have been undone and can be re-applied
/// (redo stack).
///
/// # Examples
///
/// ```
/// use campaign_builder::item_mesh_undo_redo::{ItemMeshUndoRedo, ItemMeshEditAction};
///
/// let mut mgr = ItemMeshUndoRedo::new();
/// assert!(!mgr.can_undo());
/// assert!(!mgr.can_redo());
///
/// mgr.push(ItemMeshEditAction::SetEmissive { old: false, new: true });
/// assert!(mgr.can_undo());
///
/// let action = mgr.undo().unwrap();
/// // Caller reads `action.old` (false) to revert.
/// assert!(!mgr.can_undo());
/// assert!(mgr.can_redo());
/// ```
#[derive(Debug, Default)]
pub struct ItemMeshUndoRedo {
    undo_stack: Vec<ItemMeshEditAction>,
    redo_stack: Vec<ItemMeshEditAction>,
}

impl ItemMeshUndoRedo {
    /// Creates an empty undo/redo stack.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_undo_redo::ItemMeshUndoRedo;
    ///
    /// let mgr = ItemMeshUndoRedo::new();
    /// assert!(!mgr.can_undo());
    /// assert!(!mgr.can_redo());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes a new action onto the undo stack and clears the redo stack.
    ///
    /// Any forward history (redo actions) is discarded because a new edit
    /// branches the timeline.
    ///
    /// # Arguments
    ///
    /// * `action` — The edit action that was just performed.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_undo_redo::{ItemMeshUndoRedo, ItemMeshEditAction};
    ///
    /// let mut mgr = ItemMeshUndoRedo::new();
    /// mgr.push(ItemMeshEditAction::SetScale { old: 1.0, new: 2.0 });
    /// assert_eq!(mgr.undo_count(), 1);
    /// assert_eq!(mgr.redo_count(), 0);
    /// ```
    pub fn push(&mut self, action: ItemMeshEditAction) {
        self.undo_stack.push(action);
        self.redo_stack.clear();
    }

    /// Undoes the most recent action.
    ///
    /// Pops the top action from the undo stack, pushes it onto the redo stack
    /// so it can be re-applied later, then returns the action. The caller is
    /// responsible for reading the `old` field of the returned action and
    /// restoring state accordingly.
    ///
    /// Returns `None` if the undo stack is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_undo_redo::{ItemMeshUndoRedo, ItemMeshEditAction};
    ///
    /// let mut mgr = ItemMeshUndoRedo::new();
    /// mgr.push(ItemMeshEditAction::SetScale { old: 1.0, new: 2.0 });
    ///
    /// let action = mgr.undo().unwrap();
    /// // To revert: apply action's `old` value (1.0).
    /// assert!(!mgr.can_undo());
    /// assert!(mgr.can_redo());
    /// ```
    pub fn undo(&mut self) -> Option<ItemMeshEditAction> {
        let action = self.undo_stack.pop()?;
        self.redo_stack.push(action.clone());
        Some(action)
    }

    /// Re-applies the most recently undone action.
    ///
    /// Pops the top action from the redo stack, pushes it back onto the undo
    /// stack, and returns the action. The caller is responsible for reading the
    /// `new` field of the returned action and re-applying state accordingly.
    ///
    /// Returns `None` if the redo stack is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_undo_redo::{ItemMeshUndoRedo, ItemMeshEditAction};
    ///
    /// let mut mgr = ItemMeshUndoRedo::new();
    /// mgr.push(ItemMeshEditAction::SetScale { old: 1.0, new: 2.0 });
    /// mgr.undo();
    ///
    /// let action = mgr.redo().unwrap();
    /// // To re-apply: apply action's `new` value (2.0).
    /// assert!(mgr.can_undo());
    /// assert!(!mgr.can_redo());
    /// ```
    pub fn redo(&mut self) -> Option<ItemMeshEditAction> {
        let action = self.redo_stack.pop()?;
        self.undo_stack.push(action.clone());
        Some(action)
    }

    /// Returns `true` if there is at least one action that can be undone.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_undo_redo::{ItemMeshUndoRedo, ItemMeshEditAction};
    ///
    /// let mut mgr = ItemMeshUndoRedo::new();
    /// assert!(!mgr.can_undo());
    /// mgr.push(ItemMeshEditAction::SetEmissive { old: false, new: true });
    /// assert!(mgr.can_undo());
    /// ```
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns `true` if there is at least one action that can be redone.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_undo_redo::{ItemMeshUndoRedo, ItemMeshEditAction};
    ///
    /// let mut mgr = ItemMeshUndoRedo::new();
    /// mgr.push(ItemMeshEditAction::SetEmissive { old: false, new: true });
    /// mgr.undo();
    /// assert!(mgr.can_redo());
    /// ```
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clears both the undo and redo stacks.
    ///
    /// Call this when loading a new asset or when the user explicitly resets
    /// history.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_undo_redo::{ItemMeshUndoRedo, ItemMeshEditAction};
    ///
    /// let mut mgr = ItemMeshUndoRedo::new();
    /// mgr.push(ItemMeshEditAction::SetScale { old: 1.0, new: 2.0 });
    /// mgr.clear();
    /// assert!(!mgr.can_undo());
    /// assert!(!mgr.can_redo());
    /// ```
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Returns the number of actions currently on the undo stack.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_undo_redo::{ItemMeshUndoRedo, ItemMeshEditAction};
    ///
    /// let mut mgr = ItemMeshUndoRedo::new();
    /// assert_eq!(mgr.undo_count(), 0);
    /// mgr.push(ItemMeshEditAction::SetScale { old: 1.0, new: 2.0 });
    /// assert_eq!(mgr.undo_count(), 1);
    /// ```
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Returns the number of actions currently on the redo stack.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::item_mesh_undo_redo::{ItemMeshUndoRedo, ItemMeshEditAction};
    ///
    /// let mut mgr = ItemMeshUndoRedo::new();
    /// mgr.push(ItemMeshEditAction::SetScale { old: 1.0, new: 2.0 });
    /// mgr.undo();
    /// assert_eq!(mgr.redo_count(), 1);
    /// ```
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a `SetScale` action.
    fn scale_action(old: f32, new: f32) -> ItemMeshEditAction {
        ItemMeshEditAction::SetScale { old, new }
    }

    // ── push / undo ───────────────────────────────────────────────────────────

    /// Pushing an action and then undoing it should leave the undo stack empty
    /// and put the action on the redo stack.
    #[test]
    fn test_item_mesh_undo_redo_push_and_undo() {
        let mut mgr = ItemMeshUndoRedo::new();
        mgr.push(scale_action(1.0, 2.0));

        assert!(mgr.can_undo(), "should be undoable after push");
        assert!(!mgr.can_redo(), "redo stack should be empty after push");

        let action = mgr.undo().expect("undo should return the action");
        assert!(
            matches!(action, ItemMeshEditAction::SetScale { old, new } if old == 1.0 && new == 2.0),
            "returned action should match what was pushed"
        );
        assert!(
            !mgr.can_undo(),
            "undo stack should be empty after single undo"
        );
        assert!(mgr.can_redo(), "redo stack should be non-empty after undo");
        assert_eq!(mgr.undo_count(), 0);
        assert_eq!(mgr.redo_count(), 1);
    }

    // ── redo ─────────────────────────────────────────────────────────────────

    /// After push → undo → redo the redo stack should be empty again and the
    /// undo stack should contain the action once more.
    #[test]
    fn test_item_mesh_undo_redo_redo() {
        let mut mgr = ItemMeshUndoRedo::new();
        mgr.push(scale_action(1.0, 3.0));
        mgr.undo();

        assert!(mgr.can_redo(), "redo should be available after undo");

        let action = mgr.redo().expect("redo should return the action");
        assert!(
            matches!(action, ItemMeshEditAction::SetScale { old, new } if old == 1.0 && new == 3.0),
            "redo should return the original action"
        );
        assert!(mgr.can_undo(), "undo stack should be restored after redo");
        assert!(!mgr.can_redo(), "redo stack should be empty after redo");
        assert_eq!(mgr.undo_count(), 1);
        assert_eq!(mgr.redo_count(), 0);
    }

    // ── clear ─────────────────────────────────────────────────────────────────

    /// Clearing should empty both stacks regardless of their contents.
    #[test]
    fn test_item_mesh_undo_redo_clear() {
        let mut mgr = ItemMeshUndoRedo::new();
        mgr.push(scale_action(1.0, 2.0));
        mgr.push(ItemMeshEditAction::SetEmissive {
            old: false,
            new: true,
        });

        assert_eq!(mgr.undo_count(), 2, "two actions pushed");
        mgr.undo(); // moves one to redo stack
        assert_eq!(mgr.redo_count(), 1, "one action on redo stack");

        mgr.clear();
        assert_eq!(
            mgr.undo_count(),
            0,
            "undo stack should be empty after clear"
        );
        assert_eq!(
            mgr.redo_count(),
            0,
            "redo stack should be empty after clear"
        );
        assert!(!mgr.can_undo());
        assert!(!mgr.can_redo());
    }

    // ── push discards redo history ────────────────────────────────────────────

    /// Pushing a new action after an undo should clear the redo stack.
    #[test]
    fn test_push_clears_redo_stack() {
        let mut mgr = ItemMeshUndoRedo::new();
        mgr.push(scale_action(1.0, 2.0));
        mgr.undo();
        assert!(mgr.can_redo());

        // Push a new action — redo history must be discarded.
        mgr.push(scale_action(1.0, 3.0));
        assert!(!mgr.can_redo(), "new push should discard redo history");
        assert_eq!(mgr.redo_count(), 0);
    }

    // ── empty undo/redo returns None ──────────────────────────────────────────

    /// `undo` on an empty stack should return `None`.
    #[test]
    fn test_undo_empty_returns_none() {
        let mut mgr = ItemMeshUndoRedo::new();
        assert!(mgr.undo().is_none());
    }

    /// `redo` on an empty stack should return `None`.
    #[test]
    fn test_redo_empty_returns_none() {
        let mut mgr = ItemMeshUndoRedo::new();
        assert!(mgr.redo().is_none());
    }

    // ── multiple actions ──────────────────────────────────────────────────────

    /// Multiple pushes should accumulate on the undo stack in LIFO order.
    #[test]
    fn test_multiple_pushes_lifo_order() {
        let mut mgr = ItemMeshUndoRedo::new();
        mgr.push(scale_action(1.0, 2.0));
        mgr.push(scale_action(2.0, 3.0));
        mgr.push(scale_action(3.0, 4.0));

        assert_eq!(mgr.undo_count(), 3);

        // Undo returns last pushed first.
        let a3 = mgr.undo().unwrap();
        assert!(
            matches!(a3, ItemMeshEditAction::SetScale { old, new } if old == 3.0 && new == 4.0)
        );

        let a2 = mgr.undo().unwrap();
        assert!(
            matches!(a2, ItemMeshEditAction::SetScale { old, new } if old == 2.0 && new == 3.0)
        );

        let a1 = mgr.undo().unwrap();
        assert!(
            matches!(a1, ItemMeshEditAction::SetScale { old, new } if old == 1.0 && new == 2.0)
        );

        assert!(mgr.undo().is_none());
    }

    // ── SetPrimaryColor variant ───────────────────────────────────────────────

    #[test]
    fn test_set_primary_color_action() {
        let mut mgr = ItemMeshUndoRedo::new();
        let old_color = [1.0_f32, 0.0, 0.0, 1.0];
        let new_color = [0.0_f32, 1.0, 0.0, 1.0];
        mgr.push(ItemMeshEditAction::SetPrimaryColor {
            old: old_color,
            new: new_color,
        });

        let action = mgr.undo().unwrap();
        assert!(
            matches!(action, ItemMeshEditAction::SetPrimaryColor { old, new } if old == old_color && new == new_color)
        );
    }

    // ── SetAccentColor variant ────────────────────────────────────────────────

    #[test]
    fn test_set_accent_color_action() {
        let mut mgr = ItemMeshUndoRedo::new();
        mgr.push(ItemMeshEditAction::SetAccentColor {
            old: [0.2, 0.2, 0.2, 1.0],
            new: [0.8, 0.4, 0.1, 1.0],
        });
        assert_eq!(mgr.undo_count(), 1);
    }

    // ── SetOverrideEnabled variant ────────────────────────────────────────────

    #[test]
    fn test_set_override_enabled_action() {
        let mut mgr = ItemMeshUndoRedo::new();
        mgr.push(ItemMeshEditAction::SetOverrideEnabled {
            old: false,
            new: true,
        });
        let action = mgr.undo().unwrap();
        assert!(
            matches!(action, ItemMeshEditAction::SetOverrideEnabled { old, new } if !old && new)
        );
    }

    // ── ReplaceDescriptor variant ─────────────────────────────────────────────

    #[test]
    fn test_replace_descriptor_action() {
        use antares::domain::visual::item_mesh::ItemMeshCategory;

        let old_desc = ItemMeshDescriptor {
            category: ItemMeshCategory::Sword,
            blade_length: 0.5,
            primary_color: [0.7, 0.7, 0.8, 1.0],
            accent_color: [0.5, 0.3, 0.1, 1.0],
            emissive: false,
            emissive_color: [0.0, 0.0, 0.0],
            scale: 1.0,
        };
        let new_desc = ItemMeshDescriptor {
            scale: 2.0,
            ..old_desc.clone()
        };

        let mut mgr = ItemMeshUndoRedo::new();
        mgr.push(ItemMeshEditAction::ReplaceDescriptor {
            old: old_desc.clone(),
            new: new_desc.clone(),
        });

        let action = mgr.undo().unwrap();
        match action {
            ItemMeshEditAction::ReplaceDescriptor { old, new } => {
                assert_eq!(old, old_desc);
                assert_eq!(new, new_desc);
            }
            _ => panic!("expected ReplaceDescriptor"),
        }

        // Redo returns same action.
        let redo_action = mgr.redo().unwrap();
        assert!(matches!(
            redo_action,
            ItemMeshEditAction::ReplaceDescriptor { .. }
        ));
    }
}
