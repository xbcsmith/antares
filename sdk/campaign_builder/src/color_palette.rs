// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Built-in and user-defined color palette support for the OBJ importer.
//!
//! This module ports the mesh-color palette needed by the OBJ-to-RON importer
//! into Rust and provides mesh-name based auto-assignment helpers.
//!
//! # Examples
//!
//! ```
//! use campaign_builder::color_palette::{palette_entries, suggest_color_for_mesh};
//!
//! let entries = palette_entries();
//! assert!(!entries.is_empty());
//! assert_eq!(suggest_color_for_mesh("Hair_Pink"), [0.92, 0.55, 0.70, 1.0]);
//! ```

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Default fallback mesh color used when no keyword match is found.
pub const DEFAULT_MESH_COLOR: [f32; 4] = [0.8, 0.8, 0.8, 1.0];

/// Default humanoid skin tone used for generic body-part mesh names.
pub const DEFAULT_SKIN_COLOR: [f32; 4] = [0.92, 0.85, 0.78, 1.0];

/// Built-in palette used by the OBJ importer UI.
pub const PALETTE: &[(&str, [f32; 4])] = &[
    ("skin_default", DEFAULT_SKIN_COLOR),
    ("skin_fair", [0.90, 0.80, 0.74, 1.0]),
    ("skin_tan", [0.78, 0.63, 0.50, 1.0]),
    ("skin_dark", [0.40, 0.28, 0.20, 1.0]),
    ("skin_olive", [0.65, 0.58, 0.42, 1.0]),
    ("skin_orc", [0.36, 0.50, 0.28, 1.0]),
    ("skin_orc_grey", [0.42, 0.52, 0.40, 1.0]),
    ("skin_tiefling_crimson", [0.72, 0.18, 0.15, 1.0]),
    ("skin_tiefling_indigo", [0.28, 0.20, 0.55, 1.0]),
    ("skin_tiefling_blue", [0.15, 0.22, 0.55, 1.0]),
    ("skin_halfling", [0.85, 0.74, 0.62, 1.0]),
    ("skin_gnome", [0.82, 0.72, 0.60, 1.0]),
    ("skin_gnome_tan", [0.72, 0.60, 0.45, 1.0]),
    ("skin_dwarf", [0.80, 0.65, 0.55, 1.0]),
    ("skin_elf", [0.92, 0.86, 0.80, 1.0]),
    ("skin_elf_pale", [0.96, 0.92, 0.88, 1.0]),
    ("scales_bronze", [0.72, 0.52, 0.25, 1.0]),
    ("scales_copper", [0.75, 0.45, 0.20, 1.0]),
    ("hair_red", [0.70, 0.20, 0.10, 1.0]),
    ("hair_auburn", [0.55, 0.25, 0.12, 1.0]),
    ("hair_orange", [0.82, 0.40, 0.08, 1.0]),
    ("hair_blonde", [0.85, 0.75, 0.35, 1.0]),
    ("hair_silver", [0.85, 0.85, 0.90, 1.0]),
    ("hair_platinum", [0.95, 0.95, 0.98, 1.0]),
    ("hair_black", [0.12, 0.10, 0.10, 1.0]),
    ("hair_dark", [0.22, 0.17, 0.13, 1.0]),
    ("hair_brown", [0.42, 0.28, 0.16, 1.0]),
    ("hair_chestnut", [0.50, 0.28, 0.14, 1.0]),
    ("hair_ginger", [0.78, 0.38, 0.12, 1.0]),
    ("hair_white", [0.96, 0.96, 0.96, 1.0]),
    ("hair_pink", [0.92, 0.55, 0.70, 1.0]),
    ("hair_purple", [0.42, 0.20, 0.55, 1.0]),
    ("plate_steel", [0.72, 0.74, 0.76, 1.0]),
    ("plate_gold", [0.82, 0.72, 0.28, 1.0]),
    ("plate_mithril", [0.80, 0.88, 0.95, 1.0]),
    ("leather_dark", [0.28, 0.22, 0.16, 1.0]),
    ("leather_brown", [0.48, 0.35, 0.22, 1.0]),
    ("leather_green", [0.28, 0.42, 0.22, 1.0]),
    ("leather_black", [0.14, 0.12, 0.12, 1.0]),
    ("chainmail", [0.58, 0.60, 0.62, 1.0]),
    ("fur_brown", [0.50, 0.38, 0.24, 1.0]),
    ("fur_white", [0.88, 0.85, 0.80, 1.0]),
    ("cloth_crimson", [0.72, 0.12, 0.12, 1.0]),
    ("cloth_gold", [0.75, 0.62, 0.18, 1.0]),
    ("cloth_blue", [0.18, 0.28, 0.62, 1.0]),
    ("cloth_deepblue", [0.12, 0.18, 0.48, 1.0]),
    ("cloth_violet", [0.38, 0.14, 0.58, 1.0]),
    ("cloth_dark", [0.14, 0.12, 0.18, 1.0]),
    ("cloth_white", [0.94, 0.94, 0.96, 1.0]),
    ("cloth_green", [0.22, 0.48, 0.24, 1.0]),
    ("cloth_dawn", [0.88, 0.68, 0.55, 1.0]),
    ("cloth_silver", [0.76, 0.78, 0.82, 1.0]),
    ("robe_patchwork", [0.42, 0.32, 0.48, 1.0]),
    ("iron", [0.52, 0.52, 0.55, 1.0]),
    ("gold_metal", [0.88, 0.78, 0.30, 1.0]),
    ("wood_dark", [0.32, 0.22, 0.12, 1.0]),
    ("wood_light", [0.62, 0.48, 0.30, 1.0]),
    ("horn_dark", [0.22, 0.18, 0.14, 1.0]),
    ("obsidian", [0.12, 0.10, 0.14, 1.0]),
    ("orb_blue", [0.50, 0.78, 1.00, 0.85]),
    ("orb_purple", [0.65, 0.35, 0.90, 0.85]),
    ("orb_green", [0.35, 0.85, 0.45, 0.85]),
    ("glowing_white", [1.00, 1.00, 0.90, 1.0]),
    ("glowing_gold", [1.00, 0.88, 0.40, 1.0]),
    ("glowing_red", [1.00, 0.30, 0.20, 1.0]),
    ("glowing_purple", [0.78, 0.40, 1.00, 1.0]),
    ("tusk_ivory", [0.95, 0.90, 0.80, 1.0]),
    ("bone_white", [0.92, 0.90, 0.84, 1.0]),
    ("shadow", [0.10, 0.08, 0.14, 0.80]),
];

/// A built-in palette entry surfaced to the importer UI.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaletteEntry {
    /// Human-readable palette label.
    pub label: &'static str,
    /// RGBA color value in 0.0-1.0 space.
    pub color: [f32; 4],
}

/// User-defined palette colors persisted per campaign.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CustomPalette {
    /// User-provided label/color pairs.
    pub colors: Vec<(String, [f32; 4])>,
}

/// Errors that can occur while loading or saving importer palettes.
#[derive(Debug, Error)]
pub enum PaletteError {
    /// File I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to serialize the palette.
    #[error("RON serialization error: {0}")]
    Serialization(#[from] ron::Error),

    /// Failed to deserialize the palette.
    #[error("RON deserialization error: {0}")]
    Deserialization(#[from] ron::error::SpannedError),
}

/// Returns the built-in palette as UI-friendly entries.
pub fn palette_entries() -> Vec<PaletteEntry> {
    PALETTE
        .iter()
        .map(|(label, color)| PaletteEntry {
            label,
            color: *color,
        })
        .collect()
}

/// Returns the color registered for a palette label.
pub fn palette_color(label: &str) -> Option<[f32; 4]> {
    PALETTE
        .iter()
        .find(|(entry_label, _)| *entry_label == label)
        .map(|(_, color)| *color)
}

/// Suggests a color for a mesh based on its name.
pub fn suggest_color_for_mesh(mesh_name: &str) -> [f32; 4] {
    let normalized = normalize_mesh_name(mesh_name);
    if normalized.is_empty() {
        return DEFAULT_MESH_COLOR;
    }

    // Order matters here: specific matches should win before generic body or material matches.
    if matches_keyword_group(
        &normalized,
        &["hair_pink", "pink_hair", "pigtail", "ponytail_pink"],
    ) {
        return palette_color("hair_pink").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["hair_purple", "purple_hair"]) {
        return palette_color("hair_purple").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["hair_white", "white_hair"]) {
        return palette_color("hair_white").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["hair_silver", "silver_hair"]) {
        return palette_color("hair_silver").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["hair_platinum", "platinum_hair"]) {
        return palette_color("hair_platinum").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["hair_blonde", "blonde_hair"]) {
        return palette_color("hair_blonde").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["hair_auburn", "auburn_hair"]) {
        return palette_color("hair_auburn").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["hair_brown", "brown_hair"]) {
        return palette_color("hair_brown").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["hair_black", "black_hair"]) {
        return palette_color("hair_black").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["hair_red", "red_hair"]) {
        return palette_color("hair_red").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["hair_ginger", "ginger_hair"]) {
        return palette_color("hair_ginger").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["hair_orange", "orange_hair"]) {
        return palette_color("hair_orange").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("hair") {
        return palette_color("hair_brown").unwrap_or(DEFAULT_MESH_COLOR);
    }

    if matches_keyword_group(&normalized, &["orc_grey", "orcgray", "grey_orc"]) {
        return palette_color("skin_orc_grey").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["orc", "goblin", "ogre"]) {
        return palette_color("skin_orc").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(
        &normalized,
        &["tiefling_crimson", "crimson_tiefling", "demon_red"],
    ) {
        return palette_color("skin_tiefling_crimson").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["tiefling_indigo", "indigo_tiefling"]) {
        return palette_color("skin_tiefling_indigo").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["tiefling_blue", "blue_tiefling"]) {
        return palette_color("skin_tiefling_blue").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["elf_pale", "pale_elf"]) {
        return palette_color("skin_elf_pale").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("elf") {
        return palette_color("skin_elf").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("dwarf") {
        return palette_color("skin_dwarf").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["gnome_tan", "tan_gnome"]) {
        return palette_color("skin_gnome_tan").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("gnome") {
        return palette_color("skin_gnome").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("halfling") {
        return palette_color("skin_halfling").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["skin_dark", "dark_skin"]) {
        return palette_color("skin_dark").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["skin_tan", "tan_skin"]) {
        return palette_color("skin_tan").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["skin_fair", "fair_skin"]) {
        return palette_color("skin_fair").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("body")
        || normalized.contains("skin")
        || normalized.contains("head")
        || normalized.contains("face")
        || normalized.contains("ear")
        || normalized.contains("arm")
        || normalized.contains("hand")
        || normalized.contains("leg")
        || normalized.contains("foot")
        || normalized.contains("torso")
        || normalized.contains("neck")
        || normalized.contains("finger")
    {
        return palette_color("skin_default").unwrap_or(DEFAULT_MESH_COLOR);
    }

    if matches_keyword_group(&normalized, &["plate_gold", "gold_armor", "gold_armour"]) {
        return palette_color("plate_gold").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["plate_mithril", "mithril"]) {
        return palette_color("plate_mithril").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(
        &normalized,
        &["plate", "armor", "armour", "helmet", "pauldron", "greave"],
    ) {
        return palette_color("plate_steel").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("chain") {
        return palette_color("chainmail").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["leather_black", "black_leather"]) {
        return palette_color("leather_black").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["leather_green", "green_leather"]) {
        return palette_color("leather_green").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["leather_dark", "dark_leather"]) {
        return palette_color("leather_dark").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("leather") || normalized.contains("belt") || normalized.contains("boot")
    {
        return palette_color("leather_brown").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(
        &normalized,
        &["cloth_crimson", "crimson_cloth", "red_cloth", "cape_red"],
    ) {
        return palette_color("cloth_crimson").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["cloth_gold", "gold_cloth", "trim_gold"]) {
        return palette_color("cloth_gold").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["cloth_deepblue", "deepblue_cloth"]) {
        return palette_color("cloth_deepblue").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(
        &normalized,
        &["cloth_blue", "robe_blue", "cloak_blue", "dress_blue"],
    ) {
        return palette_color("cloth_blue").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(
        &normalized,
        &["cloth_violet", "violet_cloth", "robe_violet"],
    ) {
        return palette_color("cloth_violet").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["cloth_white", "robe_white", "dress_white"]) {
        return palette_color("cloth_white").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["cloth_green", "robe_green", "cloak_green"]) {
        return palette_color("cloth_green").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["cloth_silver", "silver_cloth"]) {
        return palette_color("cloth_silver").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["dawn", "cloth_dawn"]) {
        return palette_color("cloth_dawn").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["patchwork", "robe_patchwork"]) {
        return palette_color("robe_patchwork").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("cloth")
        || normalized.contains("robe")
        || normalized.contains("cloak")
        || normalized.contains("dress")
    {
        return palette_color("cloth_dark").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["fur_white", "white_fur"]) {
        return palette_color("fur_white").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("fur") {
        return palette_color("fur_brown").unwrap_or(DEFAULT_MESH_COLOR);
    }

    if matches_keyword_group(&normalized, &["gold_metal", "gold_metallic", "gold_trim"]) {
        return palette_color("gold_metal").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("gold") {
        return palette_color("glowing_gold").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("iron")
        || normalized.contains("steel")
        || normalized.contains("blade")
        || normalized.contains("sword")
    {
        return palette_color("iron").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(&normalized, &["wood_dark", "dark_wood"]) {
        return palette_color("wood_dark").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("wood") || normalized.contains("staff") || normalized.contains("handle")
    {
        return palette_color("wood_light").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("horn") || normalized.contains("antler") {
        return palette_color("horn_dark").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("bone") || normalized.contains("skull") || normalized.contains("rib") {
        return palette_color("bone_white").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("tusk") || normalized.contains("ivory") {
        return palette_color("tusk_ivory").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("obsidian") || normalized.contains("shadow") {
        return palette_color("obsidian").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(
        &normalized,
        &["orb_blue", "blue_orb", "eye_blue", "blue_eye"],
    ) {
        return palette_color("orb_blue").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(
        &normalized,
        &["orb_green", "green_orb", "eye_green", "green_eye"],
    ) {
        return palette_color("orb_green").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if matches_keyword_group(
        &normalized,
        &["orb_purple", "purple_orb", "eye_purple", "purple_eye"],
    ) {
        return palette_color("orb_purple").unwrap_or(DEFAULT_MESH_COLOR);
    }
    if normalized.contains("glow") {
        return palette_color("glowing_white").unwrap_or(DEFAULT_MESH_COLOR);
    }

    DEFAULT_MESH_COLOR
}

impl CustomPalette {
    /// Returns the on-disk path for the importer palette file.
    pub fn file_path_for_campaign_dir(campaign_dir: &Path) -> PathBuf {
        campaign_dir.join("config").join("importer_palette.ron")
    }

    /// Loads the custom palette for the given campaign directory.
    pub fn load_from_campaign_dir(campaign_dir: &Path) -> Result<Self, PaletteError> {
        let path = Self::file_path_for_campaign_dir(campaign_dir);
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(path)?;
        if contents.trim().is_empty() {
            return Ok(Self::default());
        }

        Ok(ron::from_str(&contents)?)
    }

    /// Saves the custom palette for the given campaign directory.
    pub fn save_to_campaign_dir(&self, campaign_dir: &Path) -> Result<(), PaletteError> {
        let path = Self::file_path_for_campaign_dir(campaign_dir);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())?;
        fs::write(path, contents)?;
        Ok(())
    }

    /// Adds or replaces a custom palette entry.
    pub fn add_color(&mut self, label: impl Into<String>, color: [f32; 4]) {
        let label = label.into();
        if let Some(existing) = self
            .colors
            .iter_mut()
            .find(|(existing_label, _)| *existing_label == label)
        {
            existing.1 = color;
            return;
        }

        self.colors.push((label, color));
    }

    /// Removes a custom palette entry by label.
    pub fn remove_color(&mut self, label: &str) -> bool {
        if let Some(index) = self
            .colors
            .iter()
            .position(|(existing_label, _)| existing_label == label)
        {
            self.colors.remove(index);
            return true;
        }

        false
    }
}

fn normalize_mesh_name(mesh_name: &str) -> String {
    let mut normalized = String::with_capacity(mesh_name.len());
    let mut previous_was_separator = false;

    for ch in mesh_name.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            normalized.push('_');
            previous_was_separator = true;
        }
    }

    normalized.trim_matches('_').to_string()
}

fn matches_keyword_group(mesh_name: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| mesh_name.contains(keyword))
}

#[cfg(test)]
mod tests {
    use super::{
        palette_entries, suggest_color_for_mesh, CustomPalette, DEFAULT_MESH_COLOR,
        DEFAULT_SKIN_COLOR, PALETTE,
    };
    use tempfile::tempdir;

    #[test]
    fn test_suggest_color_for_mesh_returns_default_skin_for_em3d_body() {
        assert_eq!(suggest_color_for_mesh("EM3D_Base_Body"), DEFAULT_SKIN_COLOR);
    }

    #[test]
    fn test_suggest_color_for_mesh_returns_hair_pink() {
        assert_eq!(suggest_color_for_mesh("Hair_Pink"), [0.92, 0.55, 0.70, 1.0]);
    }

    #[test]
    fn test_suggest_color_for_mesh_returns_default_for_unknown_mesh() {
        assert_eq!(suggest_color_for_mesh("unknown_xyz"), DEFAULT_MESH_COLOR);
    }

    #[test]
    fn test_palette_entries_cover_builtin_palette() {
        let entries = palette_entries();
        assert_eq!(entries.len(), PALETTE.len());

        for ((expected_label, expected_color), entry) in PALETTE.iter().zip(entries.iter()) {
            assert_eq!(entry.label, *expected_label);
            assert_eq!(entry.color, *expected_color);
        }
    }

    #[test]
    fn test_custom_palette_round_trip_for_campaign_dir() {
        let temp_dir = tempdir().unwrap();
        let campaign_dir = temp_dir.path();
        let mut palette = CustomPalette::default();
        palette.add_color("favorite_teal", [0.1, 0.7, 0.7, 1.0]);
        palette.add_color("favorite_gold", [0.9, 0.8, 0.2, 1.0]);

        palette.save_to_campaign_dir(campaign_dir).unwrap();

        let loaded = CustomPalette::load_from_campaign_dir(campaign_dir).unwrap();
        assert_eq!(loaded, palette);
    }

    #[test]
    fn test_custom_palette_remove_color_returns_true_when_present() {
        let mut palette = CustomPalette::default();
        palette.add_color("favorite_teal", [0.1, 0.7, 0.7, 1.0]);

        assert!(palette.remove_color("favorite_teal"));
        assert!(!palette.remove_color("favorite_teal"));
    }
}
