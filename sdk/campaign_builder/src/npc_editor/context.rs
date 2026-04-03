// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Context bundle for the NPC editor's `show()` method.
//!
//! Extracted from the main NPC editor module to keep file sizes manageable.

use crate::creature_assets::CreatureAssetManager;
use antares::sdk::tool_config::DisplayConfig;
use std::path::PathBuf;

/// Context bundle for [`super::NpcEditorState::show`].
///
/// Collapses four per-call parameters so the `show()` signature stays
/// under the Clippy `too_many_arguments` limit.
///
/// # Examples
///
/// ```no_run
/// use campaign_builder::npc_editor::NpcEditorContext;
/// use antares::sdk::tool_config::DisplayConfig;
/// use std::path::PathBuf;
///
/// let dir = PathBuf::from("/campaigns/demo");
/// let cfg = NpcEditorContext {
///     campaign_dir: Some(&dir),
///     npcs_file: "data/npcs.ron",
///     display_config: &DisplayConfig::default(),
///     creature_manager: None,
/// };
/// assert_eq!(cfg.npcs_file, "data/npcs.ron");
/// ```
pub struct NpcEditorContext<'a> {
    /// Path to the open campaign root directory, or `None` if no campaign is loaded.
    pub campaign_dir: Option<&'a PathBuf>,
    /// Relative path to the NPCs data file.
    pub npcs_file: &'a str,
    /// Display configuration for layout calculations.
    pub display_config: &'a DisplayConfig,
    /// Optional creature asset manager for creature-picker support.
    pub creature_manager: Option<&'a CreatureAssetManager>,
}

impl<'a> NpcEditorContext<'a> {
    /// Returns a short debug description of this context (campaign dir and NPC file path).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use campaign_builder::npc_editor::NpcEditorContext;
    /// use antares::sdk::tool_config::DisplayConfig;
    /// use std::path::PathBuf;
    ///
    /// let dir = PathBuf::from("/campaigns/demo");
    /// let ctx = NpcEditorContext {
    ///     campaign_dir: Some(&dir),
    ///     npcs_file: "data/npcs.ron",
    ///     display_config: &DisplayConfig::default(),
    ///     creature_manager: None,
    /// };
    /// assert!(ctx.debug_info().contains("data/npcs.ron"));
    /// ```
    pub fn debug_info(&self) -> String {
        format!(
            "NpcEditorContext {{ dir: {:?}, npcs_file: {:?} }}",
            self.campaign_dir.map(|p| p.display().to_string()),
            self.npcs_file
        )
    }
}
