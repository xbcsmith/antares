// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Cursor-based linear history for mesh editors.
//!
//! This module provides [`LinearHistory<Op>`], a simple, bounded undo/redo
//! container that tracks a *position* cursor within a flat `Vec` of
//! operations. It is designed for mesh-editing workflows where each
//! operation carries both the "before" and "after" state (e.g.,
//! [`crate::mesh_vertex_editor::VertexOperation`] or
//! [`crate::mesh_index_editor::IndexOperation`]).
//!
//! ## Semantics
//!
//! * [`push`](LinearHistory::push) — truncates any forward (redo) history,
//!   appends the new operation, advances the cursor, and evicts the oldest
//!   entry when `max_history` is exceeded.
//! * [`undo`](LinearHistory::undo) — decrements the cursor and returns a
//!   clone of the operation at the new position. The caller reads the
//!   operation's "old" fields to revert mesh state.
//! * [`redo`](LinearHistory::redo) — returns a clone of the operation at the
//!   current cursor position, then advances the cursor. The caller reads the
//!   operation's "new" fields to re-apply the change.
//!
//! ## Examples
//!
//! ```
//! use campaign_builder::linear_history::LinearHistory;
//!
//! #[derive(Clone, Debug, PartialEq)]
//! struct Op { label: &'static str }
//!
//! let mut h: LinearHistory<Op> = LinearHistory::with_default_max();
//! h.push(Op { label: "move" });
//! h.push(Op { label: "scale" });
//!
//! assert!(h.can_undo());
//! let undone = h.undo().unwrap();
//! assert_eq!(undone.label, "scale");
//!
//! let redone = h.redo().unwrap();
//! assert_eq!(redone.label, "scale");
//! ```

/// Default maximum number of operations retained in a [`LinearHistory`].
pub const DEFAULT_MAX_HISTORY: usize = 100;

/// A cursor-based, bounded linear history for mesh-editor operations.
///
/// The history stores operations as a flat `Vec<Op>` and keeps a `position`
/// cursor that separates the "undo-able" region (indices `0..position`) from
/// the "redo-able" region (indices `position..len`).
///
/// # Type Parameters
///
/// * `Op` – The operation type. Must implement [`Clone`] so that callers
///   receive owned values from [`undo`](Self::undo) and [`redo`](Self::redo).
///
/// # Examples
///
/// ```
/// use campaign_builder::linear_history::{LinearHistory, DEFAULT_MAX_HISTORY};
///
/// #[derive(Clone)]
/// struct Edit { value: i32 }
///
/// let mut h = LinearHistory::new(DEFAULT_MAX_HISTORY);
/// h.push(Edit { value: 1 });
/// h.push(Edit { value: 2 });
///
/// assert_eq!(h.len(), 2);
/// assert!(h.can_undo());
/// assert!(!h.can_redo());
/// ```
#[derive(Debug, Clone)]
pub struct LinearHistory<Op: Clone> {
    /// All stored operations; index 0 is the oldest.
    history: Vec<Op>,
    /// Cursor separating undo-able from redo-able operations.
    /// Invariant: `position <= history.len()`.
    position: usize,
    /// Maximum number of operations retained in `history`.
    max_history: usize,
}

impl<Op: Clone> LinearHistory<Op> {
    /// Creates a new, empty history with the specified maximum size.
    ///
    /// When `max_history` is `usize::MAX` the history is effectively
    /// unbounded (the capacity condition `len > usize::MAX` can never be
    /// satisfied).
    ///
    /// # Arguments
    ///
    /// * `max_history` – Maximum number of operations to retain.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::linear_history::LinearHistory;
    ///
    /// let h: LinearHistory<String> = LinearHistory::new(50);
    /// assert!(h.is_empty());
    /// assert_eq!(h.len(), 0);
    /// ```
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::new(),
            position: 0,
            max_history,
        }
    }

    /// Creates a new, empty history using [`DEFAULT_MAX_HISTORY`] as the cap.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::linear_history::{LinearHistory, DEFAULT_MAX_HISTORY};
    ///
    /// let h: LinearHistory<String> = LinearHistory::with_default_max();
    /// assert!(h.is_empty());
    /// ```
    pub fn with_default_max() -> Self {
        Self::new(DEFAULT_MAX_HISTORY)
    }

    /// Appends a new operation, truncating any forward (redo) history.
    ///
    /// Steps performed:
    /// 1. Truncate `history` to `position` (discards all redo-able entries).
    /// 2. Push `op`.
    /// 3. Advance `position` to `history.len()`.
    /// 4. If `history.len() > max_history`, evict the oldest entry and
    ///    decrement `position` to keep the invariant.
    ///
    /// # Arguments
    ///
    /// * `op` – The operation to record.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::linear_history::LinearHistory;
    ///
    /// #[derive(Clone)]
    /// struct Op(i32);
    ///
    /// let mut h = LinearHistory::new(3);
    /// h.push(Op(1));
    /// h.push(Op(2));
    /// h.push(Op(3));
    /// h.push(Op(4)); // Op(1) evicted
    ///
    /// assert_eq!(h.len(), 3);
    /// assert!(!h.can_redo());
    /// ```
    pub fn push(&mut self, op: Op) {
        // Discard forward (redo) history.
        self.history.truncate(self.position);
        self.history.push(op);
        self.position = self.history.len();

        // Enforce maximum size by evicting the oldest entry.
        if self.history.len() > self.max_history {
            self.history.remove(0);
            self.position -= 1;
        }
    }

    /// Decrements the cursor and returns a clone of the operation just below it.
    ///
    /// The caller should read the operation's "old" fields to revert mesh
    /// state to what it was before that operation was applied.
    ///
    /// Returns `None` if there is nothing to undo (cursor is at the start).
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::linear_history::LinearHistory;
    ///
    /// #[derive(Clone, Debug, PartialEq)]
    /// struct Op { label: &'static str }
    ///
    /// let mut h = LinearHistory::with_default_max();
    /// h.push(Op { label: "translate" });
    ///
    /// let op = h.undo().unwrap();
    /// assert_eq!(op.label, "translate");
    /// assert!(!h.can_undo());
    /// assert!(h.can_redo());
    /// ```
    pub fn undo(&mut self) -> Option<Op> {
        if self.position == 0 {
            return None;
        }
        self.position -= 1;
        Some(self.history[self.position].clone())
    }

    /// Returns a clone of the operation at the cursor, then advances the cursor.
    ///
    /// The caller should read the operation's "new" fields to re-apply the
    /// change to mesh state.
    ///
    /// Returns `None` if there is nothing to redo (cursor is at the end).
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::linear_history::LinearHistory;
    ///
    /// #[derive(Clone, Debug, PartialEq)]
    /// struct Op { label: &'static str }
    ///
    /// let mut h = LinearHistory::with_default_max();
    /// h.push(Op { label: "scale" });
    /// h.undo();
    ///
    /// let op = h.redo().unwrap();
    /// assert_eq!(op.label, "scale");
    /// assert!(h.can_undo());
    /// assert!(!h.can_redo());
    /// ```
    pub fn redo(&mut self) -> Option<Op> {
        if self.position >= self.history.len() {
            return None;
        }
        let op = self.history[self.position].clone();
        self.position += 1;
        Some(op)
    }

    /// Returns `true` if there is at least one operation that can be undone.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::linear_history::LinearHistory;
    ///
    /// #[derive(Clone)]
    /// struct Op;
    ///
    /// let mut h = LinearHistory::with_default_max();
    /// assert!(!h.can_undo());
    /// h.push(Op);
    /// assert!(h.can_undo());
    /// ```
    pub fn can_undo(&self) -> bool {
        self.position > 0
    }

    /// Returns `true` if there is at least one operation that can be redone.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::linear_history::LinearHistory;
    ///
    /// #[derive(Clone)]
    /// struct Op;
    ///
    /// let mut h = LinearHistory::with_default_max();
    /// h.push(Op);
    /// assert!(!h.can_redo());
    /// h.undo();
    /// assert!(h.can_redo());
    /// ```
    pub fn can_redo(&self) -> bool {
        self.position < self.history.len()
    }

    /// Clears all stored operations and resets the cursor to zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::linear_history::LinearHistory;
    ///
    /// #[derive(Clone)]
    /// struct Op;
    ///
    /// let mut h = LinearHistory::with_default_max();
    /// h.push(Op);
    /// h.clear();
    /// assert!(h.is_empty());
    /// assert!(!h.can_undo());
    /// assert!(!h.can_redo());
    /// ```
    pub fn clear(&mut self) {
        self.history.clear();
        self.position = 0;
    }

    /// Returns the total number of operations stored (both undo-able and redo-able).
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::linear_history::LinearHistory;
    ///
    /// #[derive(Clone)]
    /// struct Op;
    ///
    /// let mut h = LinearHistory::with_default_max();
    /// assert_eq!(h.len(), 0);
    /// h.push(Op);
    /// h.push(Op);
    /// assert_eq!(h.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Returns `true` if no operations are stored.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::linear_history::LinearHistory;
    ///
    /// #[derive(Clone)]
    /// struct Op;
    ///
    /// let mut h = LinearHistory::with_default_max();
    /// assert!(h.is_empty());
    /// h.push(Op);
    /// assert!(!h.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct Op {
        value: i32,
    }

    fn op(value: i32) -> Op {
        Op { value }
    }

    // ── construction ─────────────────────────────────────────────────────────

    #[test]
    fn test_new_is_empty() {
        let h: LinearHistory<Op> = LinearHistory::new(10);
        assert!(h.is_empty());
        assert_eq!(h.len(), 0);
        assert!(!h.can_undo());
        assert!(!h.can_redo());
    }

    #[test]
    fn test_with_default_max_is_empty() {
        let h: LinearHistory<Op> = LinearHistory::with_default_max();
        assert!(h.is_empty());
    }

    // ── push ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_push_single() {
        let mut h = LinearHistory::with_default_max();
        h.push(op(1));
        assert_eq!(h.len(), 1);
        assert!(!h.is_empty());
        assert!(h.can_undo());
        assert!(!h.can_redo());
    }

    #[test]
    fn test_push_multiple_advances_position() {
        let mut h = LinearHistory::with_default_max();
        h.push(op(1));
        h.push(op(2));
        h.push(op(3));
        assert_eq!(h.len(), 3);
        assert!(h.can_undo());
        assert!(!h.can_redo());
    }

    #[test]
    fn test_push_after_undo_truncates_redo() {
        let mut h = LinearHistory::with_default_max();
        h.push(op(1));
        h.push(op(2));
        h.push(op(3));

        // Undo twice: cursor is now at 1, two entries are redo-able.
        h.undo();
        h.undo();
        assert!(h.can_redo());

        // Pushing now must discard ops 2 and 3.
        h.push(op(42));
        assert!(!h.can_redo(), "push must truncate forward history");
        assert_eq!(h.len(), 2); // op(1) and op(42)
    }

    #[test]
    fn test_push_enforces_max_history() {
        let mut h = LinearHistory::new(3);
        h.push(op(1));
        h.push(op(2));
        h.push(op(3));
        h.push(op(4)); // op(1) should be evicted
        assert_eq!(h.len(), 3);
        assert!(!h.can_redo());
        assert!(h.can_undo());
        // The oldest remaining entry is op(2); undo 3 times to reach it.
        let a = h.undo().unwrap();
        let b = h.undo().unwrap();
        let c = h.undo().unwrap();
        assert_eq!(a.value, 4);
        assert_eq!(b.value, 3);
        assert_eq!(c.value, 2);
        assert!(!h.can_undo());
    }

    #[test]
    fn test_push_with_max_history_one() {
        let mut h = LinearHistory::new(1);
        h.push(op(10));
        h.push(op(20)); // op(10) evicted
        assert_eq!(h.len(), 1);
        let a = h.undo().unwrap();
        assert_eq!(a.value, 20);
        assert!(!h.can_undo());
    }

    // ── undo ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_undo_empty_returns_none() {
        let mut h: LinearHistory<Op> = LinearHistory::with_default_max();
        assert!(h.undo().is_none());
    }

    #[test]
    fn test_undo_returns_correct_op() {
        let mut h = LinearHistory::with_default_max();
        h.push(op(7));
        let result = h.undo().unwrap();
        assert_eq!(result.value, 7);
    }

    #[test]
    fn test_undo_decrements_position() {
        let mut h = LinearHistory::with_default_max();
        h.push(op(1));
        h.push(op(2));
        h.push(op(3));

        let r3 = h.undo().unwrap();
        assert_eq!(r3.value, 3);
        assert!(h.can_undo());
        assert!(h.can_redo());

        let r2 = h.undo().unwrap();
        assert_eq!(r2.value, 2);

        let r1 = h.undo().unwrap();
        assert_eq!(r1.value, 1);

        assert!(!h.can_undo());
        assert!(h.can_redo());
    }

    #[test]
    fn test_undo_to_zero_then_none() {
        let mut h = LinearHistory::with_default_max();
        h.push(op(1));
        h.undo();
        assert!(h.undo().is_none());
    }

    // ── redo ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_redo_empty_returns_none() {
        let mut h: LinearHistory<Op> = LinearHistory::with_default_max();
        assert!(h.redo().is_none());
    }

    #[test]
    fn test_redo_after_undo_returns_correct_op() {
        let mut h = LinearHistory::with_default_max();
        h.push(op(5));
        h.undo();
        let result = h.redo().unwrap();
        assert_eq!(result.value, 5);
    }

    #[test]
    fn test_redo_advances_position() {
        let mut h = LinearHistory::with_default_max();
        h.push(op(1));
        h.push(op(2));
        h.push(op(3));

        h.undo();
        h.undo();
        h.undo();

        assert!(!h.can_undo());

        let r1 = h.redo().unwrap();
        assert_eq!(r1.value, 1);
        let r2 = h.redo().unwrap();
        assert_eq!(r2.value, 2);
        let r3 = h.redo().unwrap();
        assert_eq!(r3.value, 3);

        assert!(h.can_undo());
        assert!(!h.can_redo());
        assert!(h.redo().is_none());
    }

    // ── full undo/redo cycle ──────────────────────────────────────────────────

    #[test]
    fn test_full_undo_redo_cycle() {
        let mut h = LinearHistory::with_default_max();
        h.push(op(1));
        h.push(op(2));

        let u2 = h.undo().unwrap();
        assert_eq!(u2.value, 2);
        assert!(h.can_redo());

        let u1 = h.undo().unwrap();
        assert_eq!(u1.value, 1);
        assert!(!h.can_undo());
        assert!(h.can_redo());

        let r1 = h.redo().unwrap();
        assert_eq!(r1.value, 1);
        let r2 = h.redo().unwrap();
        assert_eq!(r2.value, 2);
        assert!(!h.can_redo());
        assert!(h.can_undo());
    }

    // ── can_undo / can_redo ───────────────────────────────────────────────────

    #[test]
    fn test_can_undo_false_when_empty() {
        let h: LinearHistory<Op> = LinearHistory::with_default_max();
        assert!(!h.can_undo());
    }

    #[test]
    fn test_can_redo_false_when_at_end() {
        let mut h = LinearHistory::with_default_max();
        h.push(op(1));
        assert!(!h.can_redo());
    }

    #[test]
    fn test_can_redo_true_after_undo() {
        let mut h = LinearHistory::with_default_max();
        h.push(op(1));
        h.undo();
        assert!(h.can_redo());
    }

    // ── clear ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_clear_empties_history() {
        let mut h = LinearHistory::with_default_max();
        h.push(op(1));
        h.push(op(2));
        h.clear();
        assert!(h.is_empty());
        assert_eq!(h.len(), 0);
        assert!(!h.can_undo());
        assert!(!h.can_redo());
    }

    #[test]
    fn test_clear_resets_cursor() {
        let mut h = LinearHistory::with_default_max();
        h.push(op(1));
        h.undo();
        h.clear();
        // After clearing, undo and redo return None.
        let mut h2 = h;
        assert!(h2.undo().is_none());
        assert!(h2.redo().is_none());
    }

    #[test]
    fn test_push_after_clear_works() {
        let mut h = LinearHistory::with_default_max();
        h.push(op(1));
        h.clear();
        h.push(op(99));
        assert_eq!(h.len(), 1);
        let r = h.undo().unwrap();
        assert_eq!(r.value, 99);
    }

    // ── len / is_empty ────────────────────────────────────────────────────────

    #[test]
    fn test_len_tracks_pushes() {
        let mut h = LinearHistory::with_default_max();
        for i in 0..5 {
            h.push(op(i));
            assert_eq!(h.len(), (i + 1) as usize);
        }
    }

    #[test]
    fn test_len_after_undo_unchanged() {
        let mut h = LinearHistory::with_default_max();
        h.push(op(1));
        h.push(op(2));
        h.undo(); // len stays 2; only cursor moved
        assert_eq!(h.len(), 2);
    }

    #[test]
    fn test_len_after_push_following_undo_truncates() {
        let mut h = LinearHistory::with_default_max();
        h.push(op(1));
        h.push(op(2));
        h.undo(); // position = 1, len = 2
        h.push(op(3)); // truncates op(2), then pushes op(3) → len = 2
        assert_eq!(h.len(), 2);
    }

    // ── max_history boundary ──────────────────────────────────────────────────

    #[test]
    fn test_max_history_zero_discards_all() {
        let mut h = LinearHistory::new(0);
        h.push(op(1));
        // len = 1 > 0, so oldest evicted → len = 0
        assert_eq!(h.len(), 0);
        assert!(h.is_empty());
        assert!(h.undo().is_none());
        assert!(h.redo().is_none());
    }

    #[test]
    fn test_max_history_large_no_eviction() {
        let mut h = LinearHistory::new(usize::MAX);
        for i in 0..200 {
            h.push(op(i));
        }
        assert_eq!(h.len(), 200);
    }

    #[test]
    fn test_push_at_max_evicts_then_allows_undo() {
        let mut h = LinearHistory::new(2);
        h.push(op(10));
        h.push(op(20));
        h.push(op(30)); // op(10) evicted → history = [20, 30], position = 2

        assert_eq!(h.len(), 2);
        assert!(h.can_undo());
        assert!(!h.can_redo());

        let r1 = h.undo().unwrap();
        assert_eq!(r1.value, 30);
        let r2 = h.undo().unwrap();
        assert_eq!(r2.value, 20);
        assert!(!h.can_undo());
    }
}
