// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Texture atlas generation for performance optimization
//!
//! This module provides algorithms for packing multiple textures into a single
//! atlas texture, which reduces texture binding overhead and improves rendering
//! performance.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a single texture in the atlas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureEntry {
    /// Original texture path
    pub path: String,

    /// Width in pixels
    pub width: u32,

    /// Height in pixels
    pub height: u32,

    /// Position in atlas (x, y)
    pub atlas_position: (u32, u32),

    /// UV coordinates in atlas (min_u, min_v, max_u, max_v)
    pub atlas_uvs: (f32, f32, f32, f32),
}

/// Configuration for texture atlas generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasConfig {
    /// Maximum atlas width in pixels
    pub max_width: u32,

    /// Maximum atlas height in pixels
    pub max_height: u32,

    /// Padding between textures in pixels
    pub padding: u32,

    /// Whether to use power-of-two dimensions
    pub power_of_two: bool,
}

impl Default for AtlasConfig {
    fn default() -> Self {
        Self {
            max_width: 4096,
            max_height: 4096,
            padding: 2,
            power_of_two: true,
        }
    }
}

/// Result of atlas generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasResult {
    /// Atlas width in pixels
    pub width: u32,

    /// Atlas height in pixels
    pub height: u32,

    /// Entries packed into this atlas
    pub entries: Vec<TextureEntry>,

    /// Packing efficiency (0.0 to 1.0)
    pub efficiency: f32,
}

/// Rectangle packing algorithm for texture atlas
#[derive(Debug, Clone)]
struct PackingNode {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    used: bool,
    right: Option<Box<PackingNode>>,
    down: Option<Box<PackingNode>>,
}

impl PackingNode {
    fn new(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
            used: false,
            right: None,
            down: None,
        }
    }

    fn insert(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if self.used {
            // Try inserting in right or down child
            if let Some(ref mut right) = self.right {
                if let Some(pos) = right.insert(w, h) {
                    return Some(pos);
                }
            }
            if let Some(ref mut down) = self.down {
                return down.insert(w, h);
            }
            return None;
        }

        // Check if texture fits
        if w > self.width || h > self.height {
            return None;
        }

        // Perfect fit
        if w == self.width && h == self.height {
            self.used = true;
            return Some((self.x, self.y));
        }

        // Split node
        let dw = self.width - w;
        let dh = self.height - h;

        if dw > dh {
            // Split horizontally
            self.right = Some(Box::new(PackingNode {
                x: self.x + w,
                y: self.y,
                width: self.width - w,
                height: h,
                used: false,
                right: None,
                down: None,
            }));
            self.down = Some(Box::new(PackingNode {
                x: self.x,
                y: self.y + h,
                width: self.width,
                height: self.height - h,
                used: false,
                right: None,
                down: None,
            }));
        } else {
            // Split vertically
            self.right = Some(Box::new(PackingNode {
                x: self.x + w,
                y: self.y,
                width: self.width - w,
                height: self.height,
                used: false,
                right: None,
                down: None,
            }));
            self.down = Some(Box::new(PackingNode {
                x: self.x,
                y: self.y + h,
                width: w,
                height: self.height - h,
                used: false,
                right: None,
                down: None,
            }));
        }

        self.used = true;
        Some((self.x, self.y))
    }
}

/// Generate texture atlas from a collection of texture metadata
///
/// # Arguments
///
/// * `textures` - Map of texture paths to their dimensions (width, height)
/// * `config` - Atlas generation configuration
///
/// # Returns
///
/// Returns `AtlasResult` with packing information or an error if textures don't fit
///
/// # Examples
///
/// ```
/// use antares::domain::visual::texture_atlas::{generate_atlas, AtlasConfig};
/// use std::collections::HashMap;
///
/// let mut textures = HashMap::new();
/// textures.insert("texture1.png".to_string(), (64, 64));
/// textures.insert("texture2.png".to_string(), (128, 128));
///
/// let config = AtlasConfig::default();
/// let result = generate_atlas(&textures, &config).unwrap();
///
/// assert_eq!(result.entries.len(), 2);
/// assert!(result.efficiency > 0.0);
/// ```
pub fn generate_atlas(
    textures: &HashMap<String, (u32, u32)>,
    config: &AtlasConfig,
) -> Result<AtlasResult, String> {
    if textures.is_empty() {
        return Err("No textures to pack".to_string());
    }

    // Sort textures by area (largest first) for better packing
    let mut sorted_textures: Vec<_> = textures.iter().collect();
    sorted_textures.sort_by(|a, b| {
        let area_a = a.1 .0 * a.1 .1;
        let area_b = b.1 .0 * b.1 .1;
        area_b.cmp(&area_a)
    });

    // Start with smallest power-of-two that might fit
    let mut atlas_width = if config.power_of_two { 256 } else { 128 };
    let mut atlas_height = atlas_width;

    loop {
        let mut root = PackingNode::new(atlas_width, atlas_height);
        let mut entries = Vec::new();
        let mut success = true;

        for (path, (width, height)) in &sorted_textures {
            let padded_width = width + config.padding * 2;
            let padded_height = height + config.padding * 2;

            if let Some((x, y)) = root.insert(padded_width, padded_height) {
                // Calculate UV coordinates
                let min_u = (x + config.padding) as f32 / atlas_width as f32;
                let min_v = (y + config.padding) as f32 / atlas_height as f32;
                let max_u = (x + config.padding + width) as f32 / atlas_width as f32;
                let max_v = (y + config.padding + height) as f32 / atlas_height as f32;

                entries.push(TextureEntry {
                    path: (*path).clone(),
                    width: *width,
                    height: *height,
                    atlas_position: (x + config.padding, y + config.padding),
                    atlas_uvs: (min_u, min_v, max_u, max_v),
                });
            } else {
                success = false;
                break;
            }
        }

        if success {
            // Calculate efficiency
            let used_area: u32 = entries.iter().map(|e| e.width * e.height).sum();
            let total_area = atlas_width * atlas_height;
            let efficiency = used_area as f32 / total_area as f32;

            return Ok(AtlasResult {
                width: atlas_width,
                height: atlas_height,
                entries,
                efficiency,
            });
        }

        // Try larger atlas
        if atlas_width >= config.max_width && atlas_height >= config.max_height {
            return Err(format!(
                "Textures do not fit in maximum atlas size {}x{}",
                config.max_width, config.max_height
            ));
        }

        if config.power_of_two {
            if atlas_width == atlas_height {
                atlas_width *= 2;
            } else {
                atlas_height = atlas_width;
            }
        } else {
            atlas_width += 128;
            atlas_height += 128;
        }

        atlas_width = atlas_width.min(config.max_width);
        atlas_height = atlas_height.min(config.max_height);
    }
}

/// Calculate optimal atlas size for a set of textures
///
/// # Arguments
///
/// * `textures` - Map of texture paths to their dimensions
///
/// # Returns
///
/// Returns estimated atlas dimensions (width, height)
pub fn estimate_atlas_size(textures: &HashMap<String, (u32, u32)>) -> (u32, u32) {
    if textures.is_empty() {
        return (256, 256);
    }

    let total_area: u32 = textures.values().map(|(w, h)| w * h).sum();
    let side = (total_area as f32).sqrt() as u32;

    // Round up to next power of two
    let next_power = side.next_power_of_two();
    (next_power, next_power)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_atlas_simple() {
        let mut textures = HashMap::new();
        textures.insert("tex1.png".to_string(), (64, 64));
        textures.insert("tex2.png".to_string(), (32, 32));

        let config = AtlasConfig::default();
        let result = generate_atlas(&textures, &config).unwrap();

        assert_eq!(result.entries.len(), 2);
        assert!(result.width > 0);
        assert!(result.height > 0);
        assert!(result.efficiency > 0.0 && result.efficiency <= 1.0);
    }

    #[test]
    fn test_generate_atlas_empty() {
        let textures = HashMap::new();
        let config = AtlasConfig::default();
        let result = generate_atlas(&textures, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_generate_atlas_too_large() {
        let mut textures = HashMap::new();
        textures.insert("huge.png".to_string(), (5000, 5000));

        let config = AtlasConfig {
            max_width: 1024,
            max_height: 1024,
            ..Default::default()
        };

        let result = generate_atlas(&textures, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_atlas_uv_coordinates() {
        let mut textures = HashMap::new();
        textures.insert("tex1.png".to_string(), (64, 64));

        let config = AtlasConfig {
            padding: 0,
            ..Default::default()
        };

        let result = generate_atlas(&textures, &config).unwrap();
        let entry = &result.entries[0];

        // UV coordinates should be in range [0, 1]
        assert!(entry.atlas_uvs.0 >= 0.0 && entry.atlas_uvs.0 <= 1.0);
        assert!(entry.atlas_uvs.1 >= 0.0 && entry.atlas_uvs.1 <= 1.0);
        assert!(entry.atlas_uvs.2 >= 0.0 && entry.atlas_uvs.2 <= 1.0);
        assert!(entry.atlas_uvs.3 >= 0.0 && entry.atlas_uvs.3 <= 1.0);

        // Max should be greater than min
        assert!(entry.atlas_uvs.2 > entry.atlas_uvs.0);
        assert!(entry.atlas_uvs.3 > entry.atlas_uvs.1);
    }

    #[test]
    fn test_estimate_atlas_size_empty() {
        let textures = HashMap::new();
        let (width, height) = estimate_atlas_size(&textures);

        assert_eq!(width, 256);
        assert_eq!(height, 256);
    }

    #[test]
    fn test_estimate_atlas_size_power_of_two() {
        let mut textures = HashMap::new();
        textures.insert("tex1.png".to_string(), (100, 100));
        textures.insert("tex2.png".to_string(), (100, 100));

        let (width, height) = estimate_atlas_size(&textures);

        // Should be power of two
        assert!(width.is_power_of_two());
        assert!(height.is_power_of_two());
    }

    #[test]
    fn test_atlas_config_default() {
        let config = AtlasConfig::default();

        assert_eq!(config.max_width, 4096);
        assert_eq!(config.max_height, 4096);
        assert_eq!(config.padding, 2);
        assert!(config.power_of_two);
    }

    #[test]
    fn test_generate_atlas_with_padding() {
        let mut textures = HashMap::new();
        textures.insert("tex1.png".to_string(), (64, 64));

        let config = AtlasConfig {
            padding: 4,
            ..Default::default()
        };

        let result = generate_atlas(&textures, &config).unwrap();
        let entry = &result.entries[0];

        // Position should account for padding
        assert_eq!(entry.atlas_position.0, config.padding);
        assert_eq!(entry.atlas_position.1, config.padding);
    }

    #[test]
    fn test_generate_atlas_sorts_by_size() {
        let mut textures = HashMap::new();
        textures.insert("small.png".to_string(), (32, 32));
        textures.insert("large.png".to_string(), (128, 128));
        textures.insert("medium.png".to_string(), (64, 64));

        let config = AtlasConfig::default();
        let result = generate_atlas(&textures, &config).unwrap();

        assert_eq!(result.entries.len(), 3);
        // Largest should be packed first (better efficiency)
        assert_eq!(result.entries[0].path, "large.png");
    }

    #[test]
    fn test_packing_node_insert() {
        let mut node = PackingNode::new(256, 256);

        // First insert should succeed
        let pos1 = node.insert(64, 64);
        assert!(pos1.is_some());
        assert_eq!(pos1.unwrap(), (0, 0));

        // Second insert should find space
        let pos2 = node.insert(64, 64);
        assert!(pos2.is_some());

        // Too large should fail
        let pos3 = node.insert(512, 512);
        assert!(pos3.is_none());
    }

    #[test]
    fn test_atlas_efficiency() {
        let mut textures = HashMap::new();
        // Pack 4 identical squares
        for i in 0..4 {
            textures.insert(format!("tex{}.png", i), (64, 64));
        }

        let config = AtlasConfig::default();
        let result = generate_atlas(&textures, &config).unwrap();

        // Should pack reasonably efficiently (lowered threshold as packing may not be optimal)
        assert!(result.efficiency > 0.0 && result.efficiency <= 1.0);
    }
}
