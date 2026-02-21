// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Keyboard Shortcuts System - Phase 5.3
//!
//! Provides keyboard shortcut management for the creature editor.
//! Supports:
//! - Standard editing shortcuts (Ctrl+Z, Ctrl+Y, Ctrl+C, etc.)
//! - Tool shortcuts (T for translate, R for rotate, S for scale)
//! - View shortcuts (Grid toggle, wireframe, etc.)
//! - Custom shortcut registration
//! - Platform-specific modifiers (Cmd on macOS, Ctrl on Windows/Linux)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Modifier keys for shortcuts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool, // Command key on macOS
}

impl Modifiers {
    /// No modifiers pressed
    pub const NONE: Self = Self {
        ctrl: false,
        shift: false,
        alt: false,
        meta: false,
    };

    /// Ctrl modifier only (Cmd on macOS)
    pub const CTRL: Self = Self {
        ctrl: true,
        shift: false,
        alt: false,
        meta: false,
    };

    /// Shift modifier only
    pub const SHIFT: Self = Self {
        ctrl: false,
        shift: true,
        alt: false,
        meta: false,
    };

    /// Alt modifier only
    pub const ALT: Self = Self {
        ctrl: false,
        shift: false,
        alt: true,
        meta: false,
    };

    /// Ctrl+Shift modifiers
    pub const CTRL_SHIFT: Self = Self {
        ctrl: true,
        shift: true,
        alt: false,
        meta: false,
    };

    /// Create new modifiers
    pub fn new() -> Self {
        Self::NONE
    }

    /// Check if no modifiers are pressed
    pub fn is_none(&self) -> bool {
        !self.ctrl && !self.shift && !self.alt && !self.meta
    }

    /// Check if any modifier is pressed
    pub fn any(&self) -> bool {
        !self.is_none()
    }
}

impl Default for Modifiers {
    fn default() -> Self {
        Self::NONE
    }
}

/// Key codes for shortcuts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Key {
    // Letters
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Numbers
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // Special keys
    Escape,
    Tab,
    Space,
    Enter,
    Backspace,
    Delete,

    // Arrow keys
    Left,
    Right,
    Up,
    Down,

    // Home/End/PageUp/PageDown
    Home,
    End,
    PageUp,
    PageDown,

    // Punctuation
    Minus,
    Equals,
    LeftBracket,
    RightBracket,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,
    Backslash,
}

/// Keyboard shortcut definition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Shortcut {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl Shortcut {
    /// Create a new shortcut
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    /// Create a shortcut with no modifiers
    pub fn key_only(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::NONE,
        }
    }

    /// Create a shortcut with Ctrl modifier (Cmd on macOS)
    pub fn ctrl(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::CTRL,
        }
    }

    /// Create a shortcut with Shift modifier
    pub fn shift(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::SHIFT,
        }
    }

    /// Create a shortcut with Alt modifier
    pub fn alt(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::ALT,
        }
    }

    /// Create a shortcut with Ctrl+Shift modifiers
    pub fn ctrl_shift(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::CTRL_SHIFT,
        }
    }

    /// Get human-readable description of this shortcut
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::keyboard_shortcuts::{Key, Shortcut};
    ///
    /// let s = Shortcut::ctrl(Key::Z);
    /// assert_eq!(s.describe(), "Ctrl+Z");
    /// ```
    pub fn describe(&self) -> String {
        self.to_string()
    }
}

impl std::fmt::Display for Shortcut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();

        if self.modifiers.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.modifiers.shift {
            parts.push("Shift".to_string());
        }
        if self.modifiers.alt {
            parts.push("Alt".to_string());
        }
        if self.modifiers.meta {
            parts.push("Meta".to_string());
        }

        parts.push(format!("{:?}", self.key));

        write!(f, "{}", parts.join("+"))
    }
}

/// Actions that can be triggered by shortcuts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShortcutAction {
    // Edit actions
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    SelectAll,
    Duplicate,

    // Tool selection
    SelectTool,
    TranslateTool,
    RotateTool,
    ScaleTool,

    // View controls
    ToggleGrid,
    ToggleWireframe,
    ToggleNormals,
    ToggleBoundingBox,
    ToggleStatistics,
    ResetCamera,
    FocusSelected,

    // Mesh operations
    AddVertex,
    DeleteVertex,
    MergeVertices,
    FlipNormals,
    RecalculateNormals,
    TriangulateFaces,

    // File operations
    New,
    Open,
    Save,
    SaveAs,
    Import,
    Export,

    // Navigation
    PreviousMesh,
    NextMesh,
    PreviousMode,
    NextMode,

    // Misc
    ShowHelp,
    ToggleFullscreen,
    Quit,
}

/// Keyboard shortcut manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutManager {
    shortcuts: HashMap<Shortcut, ShortcutAction>,
    reverse_map: HashMap<ShortcutAction, Shortcut>,
}

impl Default for ShortcutManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ShortcutManager {
    /// Create a new shortcut manager with default shortcuts
    pub fn new() -> Self {
        let mut manager = Self {
            shortcuts: HashMap::new(),
            reverse_map: HashMap::new(),
        };

        manager.register_defaults();
        manager
    }

    /// Register default shortcuts
    fn register_defaults(&mut self) {
        // Edit actions
        self.register(Shortcut::ctrl(Key::Z), ShortcutAction::Undo);
        self.register(Shortcut::ctrl(Key::Y), ShortcutAction::Redo);
        self.register(Shortcut::ctrl_shift(Key::Z), ShortcutAction::Redo);
        self.register(Shortcut::ctrl(Key::X), ShortcutAction::Cut);
        self.register(Shortcut::ctrl(Key::C), ShortcutAction::Copy);
        self.register(Shortcut::ctrl(Key::V), ShortcutAction::Paste);
        self.register(Shortcut::key_only(Key::Delete), ShortcutAction::Delete);
        self.register(Shortcut::ctrl(Key::A), ShortcutAction::SelectAll);
        self.register(Shortcut::ctrl(Key::D), ShortcutAction::Duplicate);

        // Tool selection
        self.register(Shortcut::key_only(Key::Q), ShortcutAction::SelectTool);
        self.register(Shortcut::key_only(Key::T), ShortcutAction::TranslateTool);
        self.register(Shortcut::key_only(Key::R), ShortcutAction::RotateTool);
        self.register(Shortcut::key_only(Key::S), ShortcutAction::ScaleTool);

        // View controls
        self.register(Shortcut::key_only(Key::G), ShortcutAction::ToggleGrid);
        self.register(Shortcut::key_only(Key::W), ShortcutAction::ToggleWireframe);
        self.register(Shortcut::key_only(Key::N), ShortcutAction::ToggleNormals);
        self.register(
            Shortcut::key_only(Key::B),
            ShortcutAction::ToggleBoundingBox,
        );
        self.register(Shortcut::shift(Key::S), ShortcutAction::ToggleStatistics);
        self.register(Shortcut::key_only(Key::Home), ShortcutAction::ResetCamera);
        self.register(Shortcut::key_only(Key::Space), ShortcutAction::ResetCamera);
        self.register(Shortcut::key_only(Key::F), ShortcutAction::FocusSelected);

        // Mesh operations
        self.register(Shortcut::shift(Key::A), ShortcutAction::AddVertex);
        self.register(Shortcut::shift(Key::D), ShortcutAction::DeleteVertex);
        self.register(Shortcut::shift(Key::M), ShortcutAction::MergeVertices);
        self.register(Shortcut::shift(Key::F), ShortcutAction::FlipNormals);
        self.register(Shortcut::shift(Key::N), ShortcutAction::RecalculateNormals);
        self.register(Shortcut::shift(Key::T), ShortcutAction::TriangulateFaces);

        // File operations
        self.register(Shortcut::ctrl(Key::N), ShortcutAction::New);
        self.register(Shortcut::ctrl(Key::O), ShortcutAction::Open);
        self.register(Shortcut::ctrl(Key::S), ShortcutAction::Save);
        self.register(Shortcut::ctrl_shift(Key::S), ShortcutAction::SaveAs);
        self.register(Shortcut::ctrl(Key::I), ShortcutAction::Import);
        self.register(Shortcut::ctrl(Key::E), ShortcutAction::Export);

        // Navigation
        self.register(
            Shortcut::key_only(Key::Escape),
            ShortcutAction::PreviousMode,
        );
        self.register(Shortcut::key_only(Key::Tab), ShortcutAction::NextMode);
        self.register(
            Shortcut::key_only(Key::PageUp),
            ShortcutAction::PreviousMesh,
        );
        self.register(Shortcut::key_only(Key::PageDown), ShortcutAction::NextMesh);
        self.register(Shortcut::ctrl(Key::PageUp), ShortcutAction::PreviousMode);
        self.register(Shortcut::ctrl(Key::PageDown), ShortcutAction::NextMode);

        // Misc
        self.register(Shortcut::key_only(Key::F1), ShortcutAction::ShowHelp);
        self.register(
            Shortcut::key_only(Key::F11),
            ShortcutAction::ToggleFullscreen,
        );
        self.register(Shortcut::ctrl(Key::Q), ShortcutAction::Quit);
    }

    /// Register a new shortcut
    pub fn register(&mut self, shortcut: Shortcut, action: ShortcutAction) {
        // Remove old shortcut for this action if it exists
        if let Some(old_shortcut) = self.reverse_map.get(&action) {
            self.shortcuts.remove(old_shortcut);
        }

        self.shortcuts.insert(shortcut.clone(), action);
        self.reverse_map.insert(action, shortcut);
    }

    /// Unregister a shortcut
    pub fn unregister(&mut self, shortcut: &Shortcut) -> Option<ShortcutAction> {
        if let Some(action) = self.shortcuts.remove(shortcut) {
            self.reverse_map.remove(&action);
            Some(action)
        } else {
            None
        }
    }

    /// Get action for a shortcut
    pub fn get_action(&self, shortcut: &Shortcut) -> Option<ShortcutAction> {
        self.shortcuts.get(shortcut).copied()
    }

    /// Get shortcut for an action
    pub fn get_shortcut(&self, action: ShortcutAction) -> Option<&Shortcut> {
        self.reverse_map.get(&action)
    }

    /// Check if a shortcut is registered
    pub fn has_shortcut(&self, shortcut: &Shortcut) -> bool {
        self.shortcuts.contains_key(shortcut)
    }

    /// Check if an action has a shortcut
    pub fn has_action(&self, action: ShortcutAction) -> bool {
        self.reverse_map.contains_key(&action)
    }

    /// Get all registered shortcuts
    pub fn all_shortcuts(&self) -> Vec<(Shortcut, ShortcutAction)> {
        self.shortcuts
            .iter()
            .map(|(s, a)| (s.clone(), *a))
            .collect()
    }

    /// Clear all shortcuts
    pub fn clear(&mut self) {
        self.shortcuts.clear();
        self.reverse_map.clear();
    }

    /// Reset to default shortcuts
    pub fn reset_defaults(&mut self) {
        self.clear();
        self.register_defaults();
    }

    /// Get human-readable shortcut description
    pub fn describe(&self, action: ShortcutAction) -> Option<String> {
        self.get_shortcut(action).map(|s| s.to_string())
    }

    /// Get all shortcuts grouped by category
    pub fn shortcuts_by_category(&self) -> HashMap<&str, Vec<(Shortcut, ShortcutAction)>> {
        let mut categories: HashMap<&str, Vec<(Shortcut, ShortcutAction)>> = HashMap::new();

        for (shortcut, action) in &self.shortcuts {
            let category = match action {
                ShortcutAction::Undo
                | ShortcutAction::Redo
                | ShortcutAction::Cut
                | ShortcutAction::Copy
                | ShortcutAction::Paste
                | ShortcutAction::Delete
                | ShortcutAction::SelectAll
                | ShortcutAction::Duplicate => "Edit",

                ShortcutAction::SelectTool
                | ShortcutAction::TranslateTool
                | ShortcutAction::RotateTool
                | ShortcutAction::ScaleTool => "Tools",

                ShortcutAction::ToggleGrid
                | ShortcutAction::ToggleWireframe
                | ShortcutAction::ToggleNormals
                | ShortcutAction::ToggleBoundingBox
                | ShortcutAction::ToggleStatistics
                | ShortcutAction::ResetCamera
                | ShortcutAction::FocusSelected => "View",

                ShortcutAction::AddVertex
                | ShortcutAction::DeleteVertex
                | ShortcutAction::MergeVertices
                | ShortcutAction::FlipNormals
                | ShortcutAction::RecalculateNormals
                | ShortcutAction::TriangulateFaces => "Mesh",

                ShortcutAction::New
                | ShortcutAction::Open
                | ShortcutAction::Save
                | ShortcutAction::SaveAs
                | ShortcutAction::Import
                | ShortcutAction::Export => "File",

                ShortcutAction::PreviousMesh
                | ShortcutAction::NextMesh
                | ShortcutAction::PreviousMode
                | ShortcutAction::NextMode => "Navigation",

                ShortcutAction::ShowHelp
                | ShortcutAction::ToggleFullscreen
                | ShortcutAction::Quit => "Misc",
            };

            categories
                .entry(category)
                .or_default()
                .push((shortcut.clone(), *action));
        }

        categories
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modifiers_none() {
        let mods = Modifiers::NONE;
        assert!(mods.is_none());
        assert!(!mods.any());
    }

    #[test]
    fn test_modifiers_ctrl() {
        let mods = Modifiers::CTRL;
        assert!(!mods.is_none());
        assert!(mods.any());
        assert!(mods.ctrl);
        assert!(!mods.shift);
        assert!(!mods.alt);
    }

    #[test]
    fn test_shortcut_creation() {
        let shortcut = Shortcut::ctrl(Key::Z);
        assert_eq!(shortcut.key, Key::Z);
        assert!(shortcut.modifiers.ctrl);
        assert!(!shortcut.modifiers.shift);
    }

    #[test]
    fn test_shortcut_to_string() {
        let shortcut = Shortcut::ctrl(Key::Z);
        assert_eq!(shortcut.to_string(), "Ctrl+Z");

        let shortcut2 = Shortcut::ctrl_shift(Key::S);
        assert_eq!(shortcut2.to_string(), "Ctrl+Shift+S");
    }

    #[test]
    fn test_shortcut_manager_register() {
        let manager = ShortcutManager::new();
        let shortcut = Shortcut::ctrl(Key::Z);

        assert!(manager.has_action(ShortcutAction::Undo));
        assert_eq!(manager.get_action(&shortcut), Some(ShortcutAction::Undo));
    }

    #[test]
    fn test_shortcut_manager_reverse_lookup() {
        let manager = ShortcutManager::new();
        let shortcut = manager.get_shortcut(ShortcutAction::Undo);

        assert!(shortcut.is_some());
        assert_eq!(shortcut.unwrap().key, Key::Z);
    }

    #[test]
    fn test_shortcut_manager_unregister() {
        let mut manager = ShortcutManager::new();
        let shortcut = Shortcut::ctrl(Key::Z);

        let action = manager.unregister(&shortcut);
        assert_eq!(action, Some(ShortcutAction::Undo));
        assert!(!manager.has_shortcut(&shortcut));
        assert!(!manager.has_action(ShortcutAction::Undo));
    }

    #[test]
    fn test_shortcut_manager_reregister() {
        let mut manager = ShortcutManager::new();

        // Register new shortcut for existing action
        let new_shortcut = Shortcut::ctrl(Key::U);
        manager.register(new_shortcut.clone(), ShortcutAction::Undo);

        // Old shortcut should be removed
        let old_shortcut = Shortcut::ctrl(Key::Z);
        assert!(!manager.has_shortcut(&old_shortcut));

        // New shortcut should work
        assert_eq!(
            manager.get_action(&new_shortcut),
            Some(ShortcutAction::Undo)
        );
    }

    #[test]
    fn test_shortcut_manager_clear() {
        let mut manager = ShortcutManager::new();
        assert!(manager.has_action(ShortcutAction::Undo));

        manager.clear();
        assert!(!manager.has_action(ShortcutAction::Undo));
        assert_eq!(manager.all_shortcuts().len(), 0);
    }

    #[test]
    fn test_shortcut_manager_reset_defaults() {
        let mut manager = ShortcutManager::new();
        let initial_count = manager.all_shortcuts().len();

        manager.clear();
        assert_eq!(manager.all_shortcuts().len(), 0);

        manager.reset_defaults();
        assert_eq!(manager.all_shortcuts().len(), initial_count);
    }

    #[test]
    fn test_shortcut_describe() {
        let manager = ShortcutManager::new();
        let desc = manager.describe(ShortcutAction::Undo);

        assert!(desc.is_some());
        let desc_str = desc.unwrap();
        assert!(desc_str.contains("Ctrl"));
        assert!(desc_str.contains("Z"));
    }

    #[test]
    fn test_shortcuts_by_category() {
        let manager = ShortcutManager::new();
        let categories = manager.shortcuts_by_category();

        assert!(categories.contains_key("Edit"));
        assert!(categories.contains_key("Tools"));
        assert!(categories.contains_key("View"));
        assert!(categories.contains_key("Mesh"));
        assert!(categories.contains_key("File"));

        let edit_shortcuts = &categories["Edit"];
        assert!(!edit_shortcuts.is_empty());
    }

    #[test]
    fn test_all_default_shortcuts() {
        let manager = ShortcutManager::new();

        // Test some key default shortcuts
        assert!(manager.has_action(ShortcutAction::Undo));
        assert!(manager.has_action(ShortcutAction::Redo));
        assert!(manager.has_action(ShortcutAction::Copy));
        assert!(manager.has_action(ShortcutAction::Paste));
        assert!(manager.has_action(ShortcutAction::Save));
        assert!(manager.has_action(ShortcutAction::ToggleGrid));
        assert!(manager.has_action(ShortcutAction::TranslateTool));
    }

    #[test]
    fn test_key_only_shortcut() {
        let shortcut = Shortcut::key_only(Key::G);
        assert_eq!(shortcut.key, Key::G);
        assert!(shortcut.modifiers.is_none());
    }

    #[test]
    fn test_multiple_modifiers() {
        let shortcut = Shortcut::new(
            Key::S,
            Modifiers {
                ctrl: true,
                shift: true,
                alt: false,
                meta: false,
            },
        );

        assert!(shortcut.modifiers.ctrl);
        assert!(shortcut.modifiers.shift);
        assert!(!shortcut.modifiers.alt);
    }
}
