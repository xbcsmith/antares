// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Campaign-scoped font handle resource.
//!
//! Holds resolved `Handle<Font>` values for the active campaign's custom fonts.
//! Populated by `ensure_campaign_fonts_loaded` in `hud.rs`.

use bevy::prelude::*;

/// Holds resolved `Handle<Font>` values for the active campaign's custom fonts.
///
/// This resource is populated by `ensure_campaign_fonts_loaded` at the start of
/// each campaign. Both handle fields default to `None`, which causes all text to
/// use the Bevy engine default font. Switching campaigns triggers a reload.
///
/// # Examples
///
/// ```
/// use antares::game::resources::CampaignFontHandles;
///
/// let handles = CampaignFontHandles::default();
/// assert!(handles.dialogue_font.is_none());
/// assert!(handles.game_menu_font.is_none());
/// assert!(handles.loaded_for_campaign.is_none());
/// ```
#[derive(Resource, Default)]
pub struct CampaignFontHandles {
    /// Handle to the custom dialogue font, or `None` to use the engine default.
    ///
    /// Applied to all text spawned by `spawn_dialogue_bubble` (speaker name and
    /// content). Populated from `campaign.game_config.fonts.dialogue_font`.
    pub dialogue_font: Option<Handle<Font>>,

    /// Handle to the custom game menu font, or `None` to use the engine default.
    ///
    /// Applied to all text in `spawn_main_menu`, `spawn_save_load_menu`, and
    /// `spawn_settings_menu`. Populated from `campaign.game_config.fonts.game_menu_font`.
    pub game_menu_font: Option<Handle<Font>>,

    /// The campaign id for which these handles were last loaded.
    ///
    /// When `ensure_campaign_fonts_loaded` detects that the active campaign id
    /// differs from this value, it reloads the fonts. `None` means the resource
    /// has never been populated.
    pub loaded_for_campaign: Option<String>,
}
