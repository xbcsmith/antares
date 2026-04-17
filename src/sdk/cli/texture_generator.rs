// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! `antares-sdk textures generate` — Placeholder terrain texture generator.
//!
//! Migrated from `src/bin/generate_terrain_textures.rs`. Exposes [`run`] as
//! the single entry point called by `src/bin/antares_sdk.rs`.
//!
//! This module has **no antares library imports** — all generation logic is
//! self-contained and depends only on the `image` crate and the standard
//! library.
//!
//! # Subcommands
//!
//! | Subcommand | Description                                  |
//! |------------|----------------------------------------------|
//! | `generate` | Generate all placeholder terrain textures    |
//!
//! # Output layout
//!
//! All output paths are relative to `--output-dir` (default: `assets/textures`
//! relative to the current working directory):
//!
//! ```text
//! <output-dir>/
//!   terrain/     ← 9 terrain PNGs (64×64)
//!   grass/       ← grass_blade.png (32×128)
//!   trees/       ← bark.png + 6 foliage PNGs
//! ```
//!
//! # Examples
//!
//! ```no_run
//! use antares::sdk::cli::texture_generator::{
//!     run, TexturesArgs, TexturesSubcommand, TexturesGenerateArgs,
//! };
//! use std::path::PathBuf;
//!
//! let args = TexturesArgs {
//!     command: TexturesSubcommand::Generate(TexturesGenerateArgs {
//!         output_dir: PathBuf::from("/tmp/textures"),
//!     }),
//! };
//! // run(args).unwrap();
//! ```

use clap::{Args, Subcommand};
use image::{ImageBuffer, Rgba};
use std::f32::consts::PI;
use std::path::{Path, PathBuf};

// ──────────────────────────────────────────────────────────────────────────────
// CLI argument structs
// ──────────────────────────────────────────────────────────────────────────────

/// Arguments for the `antares-sdk textures` subcommand group.
///
/// Acts as the dispatcher for nested texture subcommands. Pass it to [`run`]
/// to execute the chosen subcommand.
#[derive(Args, Debug)]
#[command(about = "Generate placeholder terrain and tree textures")]
pub struct TexturesArgs {
    /// The textures subcommand to execute.
    #[command(subcommand)]
    pub command: TexturesSubcommand,
}

/// Available subcommands under `antares-sdk textures`.
#[derive(Subcommand, Debug)]
pub enum TexturesSubcommand {
    /// Generate all placeholder terrain, grass, and tree textures.
    Generate(TexturesGenerateArgs),
}

/// Arguments for `antares-sdk textures generate`.
#[derive(Args, Debug)]
#[command(
    about = "Generate placeholder terrain textures",
    long_about = "Generates deterministic placeholder textures for terrain tiles, grass, and trees.\n\n\
                  Output sub-directories (`terrain/`, `grass/`, `trees/`) are created inside\n\
                  the specified --output-dir. Existing files are overwritten."
)]
pub struct TexturesGenerateArgs {
    /// Root directory where texture sub-directories will be written.
    ///
    /// Defaults to `assets/textures` relative to the current working
    /// directory. Sub-directories `terrain/`, `grass/`, and `trees/` are
    /// created automatically.
    #[arg(short, long, default_value = "assets/textures", value_name = "DIR")]
    pub output_dir: PathBuf,
}

// ──────────────────────────────────────────────────────────────────────────────
// Public entry point
// ──────────────────────────────────────────────────────────────────────────────

/// Run the `textures` subcommand group with the given arguments.
///
/// Dispatches to the appropriate handler based on [`TexturesSubcommand`].
///
/// # Errors
///
/// Returns `Err` if a required directory cannot be created. Individual file
/// write failures exit the process with code 1 to give clear shell-script
/// feedback.
///
/// # Examples
///
/// ```no_run
/// use antares::sdk::cli::texture_generator::{
///     run, TexturesArgs, TexturesSubcommand, TexturesGenerateArgs,
/// };
/// use std::path::PathBuf;
///
/// let args = TexturesArgs {
///     command: TexturesSubcommand::Generate(TexturesGenerateArgs {
///         output_dir: PathBuf::from("/tmp/textures"),
///     }),
/// };
/// // run(args).unwrap();
/// ```
pub fn run(args: TexturesArgs) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        TexturesSubcommand::Generate(g) => run_generate(g),
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Generate subcommand implementation
// ──────────────────────────────────────────────────────────────────────────────

/// Execute `antares-sdk textures generate`.
///
/// Writes terrain, grass, and tree textures into sub-directories of
/// `args.output_dir`. Exits with code 1 on any file write failure.
fn run_generate(args: TexturesGenerateArgs) -> Result<(), Box<dyn std::error::Error>> {
    let base = &args.output_dir;

    // ── Terrain ──────────────────────────────────────────────────────────────
    let terrain_dir = base.join("terrain");
    std::fs::create_dir_all(&terrain_dir).map_err(|e| {
        format!(
            "Could not create directory '{}': {e}",
            terrain_dir.display()
        )
    })?;

    println!("Writing terrain textures to: {}", terrain_dir.display());
    for spec in TERRAIN_SPECS {
        let path = terrain_dir.join(spec.filename);
        let img = generate_texture(spec);
        match img.save(&path) {
            Ok(()) => println!("  ✓  {}", spec.filename),
            Err(e) => {
                eprintln!("ERROR: Failed to write '{}': {e}", path.display());
                std::process::exit(1);
            }
        }
    }
    println!("Done. {} terrain textures written.", TERRAIN_SPECS.len());

    // ── Grass ─────────────────────────────────────────────────────────────────
    let grass_dir = base.join("grass");
    std::fs::create_dir_all(&grass_dir)
        .map_err(|e| format!("Could not create directory '{}': {e}", grass_dir.display()))?;

    println!("Writing grass textures to: {}", grass_dir.display());
    let blade_path = grass_dir.join("grass_blade.png");
    let blade_img = generate_grass_blade_texture();
    match blade_img.save(&blade_path) {
        Ok(()) => println!("  ✓  grass_blade.png"),
        Err(e) => {
            eprintln!("ERROR: Failed to write 'grass_blade.png': {e}");
            std::process::exit(1);
        }
    }
    println!("Done. 1 grass texture written.");

    // ── Trees ─────────────────────────────────────────────────────────────────
    let trees_dir = base.join("trees");
    println!("Writing tree textures to: {}", trees_dir.display());
    generate_tree_textures(&trees_dir);

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Grass blade texture constants
// ──────────────────────────────────────────────────────────────────────────────

/// Width of the grass blade texture in pixels.
const GRASS_BLADE_WIDTH: u32 = 32;
/// Height of the grass blade texture in pixels.
const GRASS_BLADE_HEIGHT: u32 = 128;
/// Width of the visible blade strip in pixels (centred in the image).
const BLADE_STRIP_WIDTH: u32 = 16;
/// Base red channel for the grass blade colour.
const GRASS_BLADE_R: u8 = 60;
/// Base green channel for the grass blade colour.
const GRASS_BLADE_G: u8 = 130;
/// Base blue channel for the grass blade colour.
const GRASS_BLADE_B: u8 = 50;
/// Alpha at the very base of the blade (fully opaque).
const GRASS_BLADE_ALPHA_BASE: u8 = 255;
/// Alpha at the very tip of the blade (semi-transparent).
const GRASS_BLADE_ALPHA_TIP: u8 = 64;
/// Deterministic noise seed for the grass blade texture.
const GRASS_BLADE_SEED: u64 = 0xA1B2_C3D4_E5F6_0718;

// ──────────────────────────────────────────────────────────────────────────────
// Tree texture constants
// ──────────────────────────────────────────────────────────────────────────────

/// Width of bark texture in pixels.
const BARK_WIDTH: u32 = 64;
/// Height of bark texture in pixels.
const BARK_HEIGHT: u32 = 128;
/// Bark base brown colour — red channel.
const BARK_R: u8 = 90;
/// Bark base brown colour — green channel.
const BARK_G: u8 = 60;
/// Bark base brown colour — blue channel.
const BARK_B: u8 = 35;
/// Bark noise seed.
const BARK_SEED: u64 = 0xB1C2_D3E4_F5A6_0718;

/// Width of foliage textures in pixels.
const FOLIAGE_WIDTH: u32 = 128;
/// Height of foliage textures in pixels.
const FOLIAGE_HEIGHT: u32 = 128;
/// Width/height of shrub foliage texture (smaller).
const SHRUB_FOLIAGE_SIZE: u32 = 64;
/// Width of the pine foliage texture.
const PINE_FOLIAGE_WIDTH: u32 = 64;
/// Height of the pine foliage texture.
const PINE_FOLIAGE_HEIGHT: u32 = 128;
/// Alpha for fully transparent foliage pixels.
const FOLIAGE_ALPHA_OUTER: u8 = 0;
/// Alpha for fully opaque foliage pixels.
const FOLIAGE_ALPHA_INNER: u8 = 240;
/// Alpha used at soft mask edges for retained semi-opaque leaf pixels.
const FOLIAGE_ALPHA_EDGE_MIN: u8 = 48;

/// Bark output filename.
const BARK_FILENAME: &str = "bark.png";

// ──────────────────────────────────────────────────────────────────────────────
// Terrain constants
// ──────────────────────────────────────────────────────────────────────────────

/// Output image width in pixels.
const IMAGE_WIDTH: u32 = 64;
/// Output image height in pixels.
const IMAGE_HEIGHT: u32 = 64;
/// Maximum per-channel noise magnitude (±NOISE_RANGE).
const NOISE_RANGE: i32 = 10;

// ──────────────────────────────────────────────────────────────────────────────
// Shape selection enum
// ──────────────────────────────────────────────────────────────────────────────

/// Shape selection for deterministic foliage mask generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoliageShape {
    /// Wide rounded crown.
    Oak,
    /// Tall narrow conical taper.
    Pine,
    /// Rounded but lighter / sparser than oak.
    Birch,
    /// Downward-heavy drooping silhouette.
    Willow,
    /// Radial fan with multiple separated frond lobes.
    Palm,
    /// Compact dense low-profile bush.
    Shrub,
}

impl FoliageShape {
    /// Returns the canonical output filename for this foliage shape.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::sdk::cli::texture_generator::FoliageShape;
    /// assert_eq!(FoliageShape::Oak.filename(), "foliage_oak.png");
    /// ```
    pub const fn filename(self) -> &'static str {
        match self {
            Self::Oak => "foliage_oak.png",
            Self::Pine => "foliage_pine.png",
            Self::Birch => "foliage_birch.png",
            Self::Willow => "foliage_willow.png",
            Self::Palm => "foliage_palm.png",
            Self::Shrub => "foliage_shrub.png",
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Spec types
// ──────────────────────────────────────────────────────────────────────────────

/// Describes one foliage texture to generate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoliageTextureSpec {
    /// Output filename (placed in `trees/` sub-directory).
    pub filename: &'static str,
    /// Shape-specific mask selection.
    pub shape: FoliageShape,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Base red channel value.
    pub r: u8,
    /// Base green channel value.
    pub g: u8,
    /// Base blue channel value.
    pub b: u8,
    /// Deterministic seed.
    pub seed: u64,
}

/// Describes one terrain texture to generate.
struct TerrainTextureSpec {
    /// Output filename (placed in `terrain/` sub-directory).
    filename: &'static str,
    /// Base red channel value (0–255).
    r: u8,
    /// Base green channel value (0–255).
    g: u8,
    /// Base blue channel value (0–255).
    b: u8,
    /// Alpha channel value (always 255 for terrain tiles).
    a: u8,
    /// Deterministic noise seed — unique per texture.
    seed: u64,
}

// ──────────────────────────────────────────────────────────────────────────────
// Static specs
// ──────────────────────────────────────────────────────────────────────────────

const TERRAIN_SPECS: &[TerrainTextureSpec] = &[
    TerrainTextureSpec {
        filename: "ground.png",
        r: 100,
        g: 95,
        b: 85,
        a: 255,
        seed: 0x1A2B_3C4D_5E6F_7081,
    },
    TerrainTextureSpec {
        filename: "grass.png",
        r: 65,
        g: 120,
        b: 50,
        a: 255,
        seed: 0x2B3C_4D5E_6F70_8192,
    },
    TerrainTextureSpec {
        filename: "stone.png",
        r: 130,
        g: 130,
        b: 135,
        a: 255,
        seed: 0x3C4D_5E6F_7081_92A3,
    },
    TerrainTextureSpec {
        filename: "mountain.png",
        r: 90,
        g: 88,
        b: 90,
        a: 255,
        seed: 0x4D5E_6F70_8192_A3B4,
    },
    TerrainTextureSpec {
        filename: "dirt.png",
        r: 110,
        g: 80,
        b: 55,
        a: 255,
        seed: 0x5E6F_7081_92A3_B4C5,
    },
    TerrainTextureSpec {
        filename: "water.png",
        r: 55,
        g: 105,
        b: 200,
        a: 255,
        seed: 0x6F70_8192_A3B4_C5D6,
    },
    TerrainTextureSpec {
        filename: "lava.png",
        r: 210,
        g: 75,
        b: 50,
        a: 255,
        seed: 0x7081_92A3_B4C5_D6E7,
    },
    TerrainTextureSpec {
        filename: "swamp.png",
        r: 88,
        g: 100,
        b: 55,
        a: 255,
        seed: 0x8192_A3B4_C5D6_E7F8,
    },
    TerrainTextureSpec {
        filename: "forest_floor.png",
        r: 50,
        g: 95,
        b: 40,
        a: 255,
        seed: 0x92A3_B4C5_D6E7_F809,
    },
];

/// All foliage output specs. Filenames, dimensions, and seeds must remain fixed.
const FOLIAGE_SPECS: &[FoliageTextureSpec] = &[
    FoliageTextureSpec {
        filename: "foliage_oak.png",
        shape: FoliageShape::Oak,
        width: FOLIAGE_WIDTH,
        height: FOLIAGE_HEIGHT,
        r: 45,
        g: 110,
        b: 35,
        seed: 0xC1D2_E3F4_A506_1728,
    },
    FoliageTextureSpec {
        filename: "foliage_pine.png",
        shape: FoliageShape::Pine,
        width: PINE_FOLIAGE_WIDTH,
        height: PINE_FOLIAGE_HEIGHT,
        r: 20,
        g: 75,
        b: 25,
        seed: 0xD2E3_F4A5_0617_2839,
    },
    FoliageTextureSpec {
        filename: "foliage_birch.png",
        shape: FoliageShape::Birch,
        width: FOLIAGE_WIDTH,
        height: FOLIAGE_HEIGHT,
        r: 65,
        g: 130,
        b: 55,
        seed: 0xE3F4_A506_1728_394A,
    },
    FoliageTextureSpec {
        filename: "foliage_willow.png",
        shape: FoliageShape::Willow,
        width: FOLIAGE_WIDTH,
        height: FOLIAGE_HEIGHT,
        r: 50,
        g: 100,
        b: 40,
        seed: 0xF4A5_0617_2839_4A5B,
    },
    FoliageTextureSpec {
        filename: "foliage_palm.png",
        shape: FoliageShape::Palm,
        width: FOLIAGE_WIDTH,
        height: FOLIAGE_HEIGHT,
        r: 55,
        g: 125,
        b: 30,
        seed: 0xA506_1728_394A_5B6C,
    },
    FoliageTextureSpec {
        filename: "foliage_shrub.png",
        shape: FoliageShape::Shrub,
        width: SHRUB_FOLIAGE_SIZE,
        height: SHRUB_FOLIAGE_SIZE,
        r: 40,
        g: 90,
        b: 30,
        seed: 0x0617_2839_4A5B_6C7D,
    },
];

// ──────────────────────────────────────────────────────────────────────────────
// PRNG and noise helpers
// ──────────────────────────────────────────────────────────────────────────────

/// Minimal xorshift64 PRNG — deterministic, no external crate needed.
fn xorshift64(state: u64) -> u64 {
    let mut x = state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

/// Clamps `value` into `[0, 255]` and returns a `u8`.
fn clamp_channel(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

/// Applies ±[`NOISE_RANGE`] noise to `base` using the PRNG state.
///
/// Returns `(noisy_value, next_state)`.
fn apply_noise(base: u8, state: u64) -> (u8, u64) {
    let next = xorshift64(state);
    let range = (NOISE_RANGE * 2 + 1) as u64;
    let offset = (next % range) as i32 - NOISE_RANGE;
    (clamp_channel(base as i32 + offset), next)
}

/// Returns a deterministic `f32` in `[0.0, 1.0]` and the next state.
fn next_unit_f32(state: u64) -> (f32, u64) {
    let next = xorshift64(state);
    let value = ((next >> 40) as u32) as f32 / ((1_u32 << 24) - 1) as f32;
    (value.clamp(0.0, 1.0), next)
}

/// Returns the image-space centre in pixels.
fn image_center(width: u32, height: u32) -> (f32, f32) {
    (width as f32 / 2.0, height as f32 / 2.0)
}

/// Returns a normalized radial distance from the image centre.
fn normalized_radius(x: u32, y: u32, width: u32, height: u32) -> f32 {
    let (cx, cy) = image_center(width, height);
    let dx = x as f32 + 0.5 - cx;
    let dy = y as f32 + 0.5 - cy;
    let rx = width as f32 / 2.0;
    let ry = height as f32 / 2.0;
    ((dx / rx).powi(2) + (dy / ry).powi(2)).sqrt()
}

/// Returns a normalized x distance from the centre in roughly `[0, 1]`.
fn normalized_abs_x(x: u32, width: u32) -> f32 {
    let cx = width as f32 / 2.0;
    ((x as f32 + 0.5) - cx).abs() / cx.max(1.0)
}

/// Returns a normalized y position from top to bottom in `[0, 1]`.
fn normalized_y(y: u32, height: u32) -> f32 {
    y as f32 / (height.saturating_sub(1)).max(1) as f32
}

/// Returns the angle of a pixel relative to the image centre in `[0, 2π)`.
fn pixel_angle(x: u32, y: u32, width: u32, height: u32) -> f32 {
    let (cx, cy) = image_center(width, height);
    let dx = x as f32 + 0.5 - cx;
    let dy = y as f32 + 0.5 - cy;
    let angle = dy.atan2(dx);
    if angle < 0.0 {
        angle + 2.0 * PI
    } else {
        angle
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Foliage shape maths
// ──────────────────────────────────────────────────────────────────────────────

/// Returns the shape-dependent nominal radius threshold for a foliage pixel.
fn foliage_radius_limit(shape: FoliageShape, x: u32, y: u32, width: u32, height: u32) -> f32 {
    let ny = normalized_y(y, height);
    let nx = normalized_abs_x(x, width);
    let angle = pixel_angle(x, y, width, height);

    match shape {
        FoliageShape::Oak => {
            let vertical_bias = if ny < 0.20 {
                0.70 + ny * 0.45
            } else if ny > 0.88 {
                0.78 + (1.0 - ny) * 0.25
            } else {
                0.98 - (ny - 0.52).abs() * 0.18
            };
            let horizontal_bias = 1.04 - nx.powf(1.6) * 0.14;
            (vertical_bias * horizontal_bias).clamp(0.58, 1.02)
        }
        FoliageShape::Pine => {
            let from_top = ny;
            let trunk_clearance = if from_top < 0.10 {
                0.22 + from_top * 0.55
            } else {
                0.32 + from_top * 0.58
            };
            let edge_taper = 1.0 - nx.powf(0.9) * 0.30;
            (trunk_clearance * edge_taper).clamp(0.18, 0.84)
        }
        FoliageShape::Birch => {
            let vertical_bias = 0.84 - (ny - 0.47).abs() * 0.18;
            let horizontal_bias = 0.92 - nx.powf(1.3) * 0.08;
            (vertical_bias * horizontal_bias).clamp(0.46, 0.90)
        }
        FoliageShape::Willow => {
            let bottom_heaviness = 0.62 + ny * 0.34;
            let side_drape = 0.90 + nx.powf(0.7) * 0.11;
            let top_trim = if ny < 0.16 { 0.58 + ny * 0.85 } else { 1.0 };
            (bottom_heaviness * side_drape * top_trim).clamp(0.34, 0.99)
        }
        FoliageShape::Palm => {
            let sector_wave = ((angle * 5.0).sin() * 0.5 + 0.5) * 0.26;
            let vertical_fan = if ny < 0.28 {
                0.46 + ny * 1.00
            } else if ny < 0.60 {
                0.78 + (0.60 - ny).abs() * 0.08
            } else {
                0.70 - (ny - 0.60) * 0.18
            };
            (vertical_fan + sector_wave).clamp(0.22, 0.98)
        }
        FoliageShape::Shrub => {
            let dome = if ny < 0.20 {
                0.30 + ny * 0.75
            } else if ny < 0.45 {
                0.54 + ny * 0.55
            } else {
                0.84 - (ny - 0.45) * 0.55
            };
            let side_trim = 0.98 - nx.powf(1.8) * 0.14;
            (dome * side_trim).clamp(0.26, 0.86)
        }
    }
}

/// Returns a density threshold used to keep or discard pixels inside a shape.
fn foliage_density_threshold(shape: FoliageShape, x: u32, y: u32, width: u32, height: u32) -> f32 {
    let ny = normalized_y(y, height);
    let nx = normalized_abs_x(x, width);
    let angle = pixel_angle(x, y, width, height);

    match shape {
        FoliageShape::Oak => {
            let lower_support = 0.70 - (ny - 0.55).abs() * 0.18;
            let edge_fade = nx.powf(1.4) * 0.08;
            (lower_support - edge_fade).clamp(0.40, 0.76)
        }
        FoliageShape::Pine => {
            let centre_bias = 0.34 + nx.powf(0.8) * 0.38;
            let lower_fill = if ny > 0.68 { 0.04 } else { 0.0 };
            (centre_bias + lower_fill).clamp(0.28, 0.82)
        }
        FoliageShape::Birch => {
            let sparse_bias = 0.60 + nx.powf(1.2) * 0.05 + (ny - 0.50).abs() * 0.06;
            sparse_bias.clamp(0.48, 0.78)
        }
        FoliageShape::Willow => {
            let droop_fill = 0.58 - ny * 0.12 + nx.powf(0.6) * 0.05;
            droop_fill.clamp(0.36, 0.72)
        }
        FoliageShape::Palm => {
            let lobe_phase = ((angle * 5.0).sin() * 0.5 + 0.5) * 0.24;
            let centre_gap = 0.62 + lobe_phase;
            centre_gap.clamp(0.42, 0.90)
        }
        FoliageShape::Shrub => {
            let dense_base = 0.26 + (1.0 - ny) * 0.18 + nx.powf(1.2) * 0.03;
            dense_base.clamp(0.14, 0.42)
        }
    }
}

/// Returns the foliage alpha for one pixel based on the selected shape.
fn foliage_alpha_for_pixel(
    shape: FoliageShape,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    noise_sample: f32,
) -> u8 {
    let radius = normalized_radius(x, y, width, height);
    let radius_limit = foliage_radius_limit(shape, x, y, width, height);

    if radius > radius_limit {
        return FOLIAGE_ALPHA_OUTER;
    }

    let threshold = foliage_density_threshold(shape, x, y, width, height);
    if noise_sample < threshold {
        return FOLIAGE_ALPHA_OUTER;
    }

    let softness = ((radius_limit - radius) / radius_limit.max(0.001)).clamp(0.0, 1.0);
    let alpha = if softness < 0.08 {
        let t = softness / 0.08;
        FOLIAGE_ALPHA_EDGE_MIN as f32 + t * (FOLIAGE_ALPHA_INNER - FOLIAGE_ALPHA_EDGE_MIN) as f32
    } else {
        FOLIAGE_ALPHA_INNER as f32
    };

    alpha.round() as u8
}

// ──────────────────────────────────────────────────────────────────────────────
// Texture generation functions
// ──────────────────────────────────────────────────────────────────────────────

/// Generates a single 64×64 RGBA texture for the given [`TerrainTextureSpec`].
fn generate_texture(spec: &TerrainTextureSpec) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(IMAGE_WIDTH, IMAGE_HEIGHT);
    let mut state = spec.seed;

    for y in 0..IMAGE_HEIGHT {
        for x in 0..IMAGE_WIDTH {
            let (r, s1) = apply_noise(spec.r, state);
            let (g, s2) = apply_noise(spec.g, s1);
            let (b, s3) = apply_noise(spec.b, s2);
            state = s3;
            img.put_pixel(x, y, Rgba([r, g, b, spec.a]));
        }
    }

    img
}

/// Generates a 64×128 RGBA bark texture for tree trunks.
///
/// The texture is fully opaque (alpha = 255) with a brown base colour and
/// deterministic vertical-grain noise applied per pixel.
pub fn generate_bark_texture() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(BARK_WIDTH, BARK_HEIGHT);
    let mut state = BARK_SEED;

    for y in 0..BARK_HEIGHT {
        let grain = ((y % 5) as i32) - 2;
        for x in 0..BARK_WIDTH {
            let r_base = clamp_channel(BARK_R as i32 + grain);
            let g_base = clamp_channel(BARK_G as i32 + grain);
            let (r, s1) = apply_noise(r_base, state);
            let (g, s2) = apply_noise(g_base, s1);
            let (b, s3) = apply_noise(BARK_B, s2);
            state = s3;
            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }

    img
}

/// Generates an RGBA foliage texture using deterministic shape-specific mask logic.
///
/// # Arguments
///
/// * `shape`  - The target foliage silhouette to generate
/// * `width`  - Image width in pixels
/// * `height` - Image height in pixels
/// * `base_r` - Base red channel for foliage colour
/// * `base_g` - Base green channel for foliage colour
/// * `base_b` - Base blue channel for foliage colour
/// * `seed`   - Deterministic seed value
///
/// # Examples
///
/// ```
/// use antares::sdk::cli::texture_generator::{generate_foliage_texture, FoliageShape};
///
/// let img = generate_foliage_texture(FoliageShape::Oak, 128, 128, 45, 110, 35, 0xC1D2_E3F4_A506_1728);
/// assert_eq!(img.width(), 128);
/// assert_eq!(img.height(), 128);
/// ```
pub fn generate_foliage_texture(
    shape: FoliageShape,
    width: u32,
    height: u32,
    base_r: u8,
    base_g: u8,
    base_b: u8,
    seed: u64,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(width, height);
    let mut state = seed;

    for y in 0..height {
        for x in 0..width {
            let (mask_noise, s1) = next_unit_f32(state);
            let alpha = foliage_alpha_for_pixel(shape, x, y, width, height, mask_noise);

            if alpha == FOLIAGE_ALPHA_OUTER {
                state = s1;
                img.put_pixel(x, y, Rgba([0, 0, 0, FOLIAGE_ALPHA_OUTER]));
                continue;
            }

            let (r, s2) = apply_noise(base_r, s1);
            let (g, s3) = apply_noise(base_g, s2);
            let (b, s4) = apply_noise(base_b, s3);
            state = s4;
            img.put_pixel(x, y, Rgba([r, g, b, alpha]));
        }
    }

    img
}

/// Generates a 32×128 RGBA grass blade texture with a transparent background.
///
/// The blade occupies the centre [`BLADE_STRIP_WIDTH`] columns and fades from
/// [`GRASS_BLADE_ALPHA_BASE`] at the bottom row to [`GRASS_BLADE_ALPHA_TIP`]
/// at the top row.
pub fn generate_grass_blade_texture() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(GRASS_BLADE_WIDTH, GRASS_BLADE_HEIGHT);
    let mut state = GRASS_BLADE_SEED;

    let blade_left = (GRASS_BLADE_WIDTH - BLADE_STRIP_WIDTH) / 2;
    let blade_right = blade_left + BLADE_STRIP_WIDTH;

    for y in 0..GRASS_BLADE_HEIGHT {
        let t = y as f32 / (GRASS_BLADE_HEIGHT - 1) as f32;
        let alpha_f = GRASS_BLADE_ALPHA_BASE as f32
            + t * (GRASS_BLADE_ALPHA_TIP as f32 - GRASS_BLADE_ALPHA_BASE as f32);
        let alpha = alpha_f.round() as u8;

        for x in 0..GRASS_BLADE_WIDTH {
            if x >= blade_left && x < blade_right {
                let (r, s1) = apply_noise(GRASS_BLADE_R, state);
                let (g, s2) = apply_noise(GRASS_BLADE_G, s1);
                let (b, s3) = apply_noise(GRASS_BLADE_B, s2);
                state = s3;
                img.put_pixel(x, y, Rgba([r, g, b, alpha]));
            } else {
                img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }

    img
}

/// Generates all tree texture assets and writes them to `trees_dir`.
///
/// # Textures generated
///
/// | File                | Dimensions | Content                          |
/// |---------------------|------------|----------------------------------|
/// | `bark.png`          | 64×128     | Brown vertical-grain bark        |
/// | `foliage_oak.png`   | 128×128    | Wide rounded crown               |
/// | `foliage_pine.png`  | 64×128     | Tall narrow taper                |
/// | `foliage_birch.png` | 128×128    | Light rounded sparse crown       |
/// | `foliage_willow.png`| 128×128    | Downward drooping canopy         |
/// | `foliage_palm.png`  | 128×128    | Radial separated fronds          |
/// | `foliage_shrub.png` | 64×64      | Compact low-profile bush         |
pub fn generate_tree_textures(trees_dir: &Path) {
    std::fs::create_dir_all(trees_dir).unwrap_or_else(|e| {
        eprintln!(
            "ERROR: Could not create directory '{}': {e}",
            trees_dir.display()
        );
        std::process::exit(1);
    });

    let bark = generate_bark_texture();
    save_texture(trees_dir, BARK_FILENAME, &bark);

    for spec in FOLIAGE_SPECS {
        let foliage = generate_foliage_texture(
            spec.shape,
            spec.width,
            spec.height,
            spec.r,
            spec.g,
            spec.b,
            spec.seed,
        );
        save_texture(trees_dir, spec.filename, &foliage);
    }

    println!(
        "Done. {} tree textures written to: {}",
        FOLIAGE_SPECS.len() + 1,
        trees_dir.display()
    );
}

/// Saves an image buffer to a file, exiting the process on error.
fn save_texture(dir: &Path, filename: &str, img: &ImageBuffer<Rgba<u8>, Vec<u8>>) {
    let path = dir.join(filename);
    match img.save(&path) {
        Ok(()) => println!("  ✓  {filename}"),
        Err(e) => {
            eprintln!("ERROR: Failed to write '{filename}': {e}");
            std::process::exit(1);
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── CLI parsing ────────────────────────────────────────────────────────────

    /// Verify `TexturesGenerateArgs` default `output_dir` value parses from
    /// an empty args list.
    #[test]
    fn test_textures_generate_args_default_output_dir() {
        // Smoke-test that the struct is constructable with a custom path.
        let args = TexturesGenerateArgs {
            output_dir: PathBuf::from("assets/textures"),
        };
        assert_eq!(args.output_dir, PathBuf::from("assets/textures"));

        // Verify the command builds without panicking.
        let cmd = TexturesArgs::augment_args(clap::Command::new("textures"));
        assert_eq!(cmd.get_name(), "textures");
    }

    /// `--output-dir /tmp/test` must parse into the `output_dir` field.
    #[test]
    fn test_textures_generate_args_custom_output_dir() {
        let args = TexturesGenerateArgs {
            output_dir: PathBuf::from("/tmp/test_textures"),
        };
        assert_eq!(args.output_dir, PathBuf::from("/tmp/test_textures"));
    }

    /// `TexturesArgs` with a `Generate` subcommand must be constructable.
    #[test]
    fn test_textures_args_generate_subcommand() {
        let args = TexturesArgs {
            command: TexturesSubcommand::Generate(TexturesGenerateArgs {
                output_dir: PathBuf::from("/tmp/out"),
            }),
        };
        match args.command {
            TexturesSubcommand::Generate(g) => {
                assert_eq!(g.output_dir, PathBuf::from("/tmp/out"));
            }
        }
    }

    // ── PRNG helpers ───────────────────────────────────────────────────────────

    #[test]
    fn test_xorshift64_advances_state() {
        let seed = 0xDEAD_BEEF_CAFE_1234_u64;
        let next = xorshift64(seed);
        assert_ne!(next, seed, "xorshift64 must change the state");
    }

    #[test]
    fn test_xorshift64_is_deterministic() {
        let seed = 0x1122_3344_5566_7788_u64;
        assert_eq!(xorshift64(seed), xorshift64(seed));
    }

    // ── Channel clamping ───────────────────────────────────────────────────────

    #[test]
    fn test_clamp_channel_low() {
        assert_eq!(clamp_channel(-1), 0);
        assert_eq!(clamp_channel(i32::MIN), 0);
    }

    #[test]
    fn test_clamp_channel_high() {
        assert_eq!(clamp_channel(256), 255);
        assert_eq!(clamp_channel(i32::MAX), 255);
    }

    #[test]
    fn test_clamp_channel_valid_range() {
        for v in 0..=255_i32 {
            assert_eq!(clamp_channel(v), v as u8);
        }
    }

    // ── Noise ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_apply_noise_stays_in_bounds() {
        let state = 0xABCD_EF01_2345_6789_u64;
        for base in [0_u8, 10, 128, 245, 255] {
            let (noisy, _) = apply_noise(base, state);
            let lo = (base as i32 - NOISE_RANGE).max(0) as u8;
            let hi = (base as i32 + NOISE_RANGE).min(255) as u8;
            assert!(
                noisy >= lo && noisy <= hi,
                "apply_noise({base}) = {noisy} is outside [{lo}, {hi}]"
            );
        }
    }

    #[test]
    fn test_apply_noise_advances_state() {
        let state = 0x1111_2222_3333_4444_u64;
        let (_, next) = apply_noise(128, state);
        assert_ne!(next, state);
    }

    // ── Terrain specs ──────────────────────────────────────────────────────────

    #[test]
    fn test_terrain_specs_unique_seeds() {
        let seeds: Vec<u64> = TERRAIN_SPECS.iter().map(|s| s.seed).collect();
        let mut sorted = seeds.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            seeds.len(),
            "Duplicate seeds found in TERRAIN_SPECS"
        );
    }

    #[test]
    fn test_terrain_specs_unique_filenames() {
        let mut names: Vec<&str> = TERRAIN_SPECS.iter().map(|s| s.filename).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), TERRAIN_SPECS.len());
    }

    #[test]
    fn test_terrain_specs_count() {
        assert_eq!(TERRAIN_SPECS.len(), 9, "Expected exactly 9 terrain specs");
    }

    #[test]
    fn test_terrain_specs_all_opaque() {
        for spec in TERRAIN_SPECS {
            assert_eq!(spec.a, 255, "Spec '{}' must be fully opaque", spec.filename);
        }
    }

    // ── Grass blade ───────────────────────────────────────────────────────────

    #[test]
    fn test_generate_grass_blade_texture_dimensions() {
        let img = generate_grass_blade_texture();
        assert_eq!(img.width(), GRASS_BLADE_WIDTH);
        assert_eq!(img.height(), GRASS_BLADE_HEIGHT);
    }

    #[test]
    fn test_generate_grass_blade_texture_outside_strip_transparent() {
        let img = generate_grass_blade_texture();
        let blade_left = (GRASS_BLADE_WIDTH - BLADE_STRIP_WIDTH) / 2;
        let blade_right = blade_left + BLADE_STRIP_WIDTH;
        for y in 0..GRASS_BLADE_HEIGHT {
            for x in 0..GRASS_BLADE_WIDTH {
                let pixel = img.get_pixel(x, y);
                if x < blade_left || x >= blade_right {
                    assert_eq!(
                        pixel[3], 0,
                        "pixel ({x},{y}) outside strip must be transparent"
                    );
                }
            }
        }
    }

    #[test]
    fn test_generate_grass_blade_texture_inside_strip_alpha_range() {
        let img = generate_grass_blade_texture();
        let blade_left = (GRASS_BLADE_WIDTH - BLADE_STRIP_WIDTH) / 2;
        let blade_right = blade_left + BLADE_STRIP_WIDTH;
        for y in 0..GRASS_BLADE_HEIGHT {
            for x in blade_left..blade_right {
                let alpha = img.get_pixel(x, y)[3];
                assert!(
                    alpha >= GRASS_BLADE_ALPHA_TIP,
                    "blade alpha {alpha} at ({x},{y}) must be >= {GRASS_BLADE_ALPHA_TIP}"
                );
            }
        }
    }

    #[test]
    fn test_generate_grass_blade_texture_alpha_gradient_direction() {
        let img = generate_grass_blade_texture();
        let mid_x = GRASS_BLADE_WIDTH / 2;
        // y=0 corresponds to t=0 → ALPHA_BASE (255, blade root/base, fully opaque).
        // y=H-1 corresponds to t=1 → ALPHA_TIP  (64,  blade tip, semi-transparent).
        let root_alpha = img.get_pixel(mid_x, 0)[3];
        let tip_alpha = img.get_pixel(mid_x, GRASS_BLADE_HEIGHT - 1)[3];
        assert!(
            root_alpha > tip_alpha,
            "blade root (y=0, alpha={root_alpha}) must be more opaque than tip (y=H-1, alpha={tip_alpha})"
        );
    }

    #[test]
    fn test_generate_grass_blade_texture_is_deterministic() {
        let img1 = generate_grass_blade_texture();
        let img2 = generate_grass_blade_texture();
        assert_eq!(
            img1.as_raw(),
            img2.as_raw(),
            "grass blade texture must be deterministic"
        );
    }

    // ── Terrain texture ───────────────────────────────────────────────────────

    #[test]
    fn test_generate_texture_dimensions() {
        let spec = &TERRAIN_SPECS[0];
        let img = generate_texture(spec);
        assert_eq!(img.width(), IMAGE_WIDTH);
        assert_eq!(img.height(), IMAGE_HEIGHT);
    }

    #[test]
    fn test_generate_texture_pixels_in_bounds() {
        for spec in TERRAIN_SPECS {
            let img = generate_texture(spec);
            for pixel in img.pixels() {
                let lo = (spec.r as i32 - NOISE_RANGE).max(0) as u8;
                let hi = (spec.r as i32 + NOISE_RANGE).min(255) as u8;
                assert!(
                    pixel[0] >= lo && pixel[0] <= hi,
                    "R={} out of [{lo},{hi}] for {}",
                    pixel[0],
                    spec.filename
                );
            }
        }
    }

    #[test]
    fn test_generate_texture_is_deterministic() {
        let spec = &TERRAIN_SPECS[0];
        let img1 = generate_texture(spec);
        let img2 = generate_texture(spec);
        assert_eq!(img1.as_raw(), img2.as_raw());
    }

    #[test]
    fn test_generate_texture_different_seeds_differ() {
        let img1 = generate_texture(&TERRAIN_SPECS[0]);
        let img2 = generate_texture(&TERRAIN_SPECS[1]);
        assert_ne!(img1.as_raw(), img2.as_raw());
    }

    // ── Bark texture ──────────────────────────────────────────────────────────

    #[test]
    fn test_generate_bark_texture_dimensions() {
        let img = generate_bark_texture();
        assert_eq!(img.width(), BARK_WIDTH);
        assert_eq!(img.height(), BARK_HEIGHT);
    }

    #[test]
    fn test_generate_bark_texture_fully_opaque() {
        let img = generate_bark_texture();
        for pixel in img.pixels() {
            assert_eq!(pixel[3], 255, "bark pixels must be fully opaque");
        }
    }

    #[test]
    fn test_generate_bark_texture_is_deterministic() {
        let img1 = generate_bark_texture();
        let img2 = generate_bark_texture();
        assert_eq!(img1.as_raw(), img2.as_raw());
    }

    // ── Foliage specs ──────────────────────────────────────────────────────────

    #[test]
    fn test_foliage_specs_use_expected_filenames_dimensions_and_seeds() {
        let expected = [
            (
                "foliage_oak.png",
                FOLIAGE_WIDTH,
                FOLIAGE_HEIGHT,
                0xC1D2_E3F4_A506_1728_u64,
            ),
            (
                "foliage_pine.png",
                PINE_FOLIAGE_WIDTH,
                PINE_FOLIAGE_HEIGHT,
                0xD2E3_F4A5_0617_2839_u64,
            ),
            (
                "foliage_birch.png",
                FOLIAGE_WIDTH,
                FOLIAGE_HEIGHT,
                0xE3F4_A506_1728_394A_u64,
            ),
            (
                "foliage_willow.png",
                FOLIAGE_WIDTH,
                FOLIAGE_HEIGHT,
                0xF4A5_0617_2839_4A5B_u64,
            ),
            (
                "foliage_palm.png",
                FOLIAGE_WIDTH,
                FOLIAGE_HEIGHT,
                0xA506_1728_394A_5B6C_u64,
            ),
            (
                "foliage_shrub.png",
                SHRUB_FOLIAGE_SIZE,
                SHRUB_FOLIAGE_SIZE,
                0x0617_2839_4A5B_6C7D_u64,
            ),
        ];
        assert_eq!(FOLIAGE_SPECS.len(), expected.len());
        for (spec, (filename, w, h, seed)) in FOLIAGE_SPECS.iter().zip(expected.iter()) {
            assert_eq!(spec.filename, *filename);
            assert_eq!(spec.width, *w);
            assert_eq!(spec.height, *h);
            assert_eq!(spec.seed, *seed);
        }
    }

    // ── Foliage texture ───────────────────────────────────────────────────────

    fn foliage_image(spec: &FoliageTextureSpec) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        generate_foliage_texture(
            spec.shape,
            spec.width,
            spec.height,
            spec.r,
            spec.g,
            spec.b,
            spec.seed,
        )
    }

    fn find_spec(shape: FoliageShape) -> &'static FoliageTextureSpec {
        FOLIAGE_SPECS
            .iter()
            .find(|s| s.shape == shape)
            .expect("spec must exist")
    }

    fn opaque_pixel_count(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> usize {
        img.pixels().filter(|p| p[3] > 0).count()
    }

    fn bounding_box(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Option<(u32, u32, u32, u32)> {
        let mut min_x = img.width();
        let mut min_y = img.height();
        let mut max_x = 0_u32;
        let mut max_y = 0_u32;
        let mut found = false;
        for (x, y, pixel) in img.enumerate_pixels() {
            if pixel[3] > 0 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                found = true;
            }
        }
        found.then_some((min_x, min_y, max_x, max_y))
    }

    fn occupied_width(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> u32 {
        let (min_x, _, max_x, _) = bounding_box(img).expect("non-empty foliage image");
        max_x - min_x + 1
    }

    fn occupied_height(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> u32 {
        let (_, min_y, _, max_y) = bounding_box(img).expect("non-empty foliage image");
        max_y - min_y + 1
    }

    fn occupied_width_height_ratio(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> f32 {
        occupied_width(img) as f32 / occupied_height(img) as f32
    }

    fn occupied_height_ratio(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> f32 {
        occupied_height(img) as f32 / img.height() as f32
    }

    fn lower_half_opaque_count(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> usize {
        let start_y = img.height() / 2;
        (start_y..img.height())
            .flat_map(|y| (0..img.width()).map(move |x| (x, y)))
            .filter(|(x, y)| img.get_pixel(*x, *y)[3] > 0)
            .count()
    }

    fn upper_half_opaque_count(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> usize {
        (0..(img.height() / 2))
            .flat_map(|y| (0..img.width()).map(move |x| (x, y)))
            .filter(|(x, y)| img.get_pixel(*x, *y)[3] > 0)
            .count()
    }

    fn central_vertical_occupancy_ratio(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> f32 {
        let centre_x = img.width() / 2;
        let left = centre_x.saturating_sub(img.width().max(1) / 8);
        let right = (centre_x + img.width().max(1) / 8).min(img.width() - 1);
        let mut opaque = 0usize;
        let mut total = 0usize;
        for y in 0..img.height() {
            for x in left..=right {
                total += 1;
                if img.get_pixel(x, y)[3] > 0 {
                    opaque += 1;
                }
            }
        }
        opaque as f32 / total.max(1) as f32
    }

    fn transparent_outer_region_pixels(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> usize {
        let (cx, cy) = image_center(img.width(), img.height());
        let max_radius = (img.width().min(img.height()) as f32 / 2.0).max(1.0);
        let threshold = max_radius * 0.85;
        img.enumerate_pixels()
            .filter(|(x, y, pixel)| {
                let dx = *x as f32 + 0.5 - cx;
                let dy = *y as f32 + 0.5 - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                dist >= threshold && pixel[3] == 0
            })
            .count()
    }

    fn non_empty_angular_sectors_outside_center(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> usize {
        const SECTOR_COUNT: usize = 8;
        const OUTER_RADIUS_FRACTION: f32 = 0.28;

        let mut sectors = [false; SECTOR_COUNT];
        let (cx, cy) = image_center(img.width(), img.height());
        let max_radius = (img.width().min(img.height()) as f32 / 2.0).max(1.0);
        let min_outer_radius = max_radius * OUTER_RADIUS_FRACTION;

        for (x, y, pixel) in img.enumerate_pixels() {
            if pixel[3] == 0 {
                continue;
            }
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let radius = (dx * dx + dy * dy).sqrt();
            if radius < min_outer_radius {
                continue;
            }
            let angle = if dy.atan2(dx) < 0.0 {
                dy.atan2(dx) + 2.0 * PI
            } else {
                dy.atan2(dx)
            };
            let sector =
                ((angle / (2.0 * PI)) * SECTOR_COUNT as f32).floor() as usize % SECTOR_COUNT;
            sectors[sector] = true;
        }

        sectors.iter().filter(|filled| **filled).count()
    }

    #[test]
    fn test_generate_foliage_texture_dimensions_for_all_shapes() {
        for spec in FOLIAGE_SPECS {
            let img = foliage_image(spec);
            assert_eq!(img.width(), spec.width, "{} width", spec.filename);
            assert_eq!(img.height(), spec.height, "{} height", spec.filename);
        }
    }

    #[test]
    fn test_generate_foliage_texture_deterministic_for_all_fixed_seeds() {
        for spec in FOLIAGE_SPECS {
            let img1 = foliage_image(spec);
            let img2 = foliage_image(spec);
            assert_eq!(
                img1.as_raw(),
                img2.as_raw(),
                "{} must be deterministic",
                spec.filename
            );
        }
    }

    #[test]
    fn test_all_foliage_outputs_have_transparent_outer_region_pixels() {
        for spec in FOLIAGE_SPECS {
            let img = foliage_image(spec);
            let transparent = transparent_outer_region_pixels(&img);
            assert!(
                transparent > 0,
                "{} must have transparent outer pixels",
                spec.filename
            );
        }
    }

    #[test]
    fn test_oak_bounding_box_width_is_greater_than_shrub_bounding_box_width() {
        let oak = foliage_image(find_spec(FoliageShape::Oak));
        let shrub = foliage_image(find_spec(FoliageShape::Shrub));
        assert!(
            occupied_width(&oak) > occupied_width(&shrub),
            "oak occupied width {} must exceed shrub {}",
            occupied_width(&oak),
            occupied_width(&shrub)
        );
    }

    #[test]
    fn test_pine_central_vertical_occupancy_ratio_is_greater_than_oak() {
        let pine = foliage_image(find_spec(FoliageShape::Pine));
        let oak = foliage_image(find_spec(FoliageShape::Oak));
        assert!(
            central_vertical_occupancy_ratio(&pine) > central_vertical_occupancy_ratio(&oak),
            "pine centre occupancy {} must exceed oak {}",
            central_vertical_occupancy_ratio(&pine),
            central_vertical_occupancy_ratio(&oak)
        );
    }

    #[test]
    fn test_pine_width_height_ratio_is_lower_than_oak() {
        let pine = foliage_image(find_spec(FoliageShape::Pine));
        let oak = foliage_image(find_spec(FoliageShape::Oak));
        assert!(
            occupied_width_height_ratio(&pine) < occupied_width_height_ratio(&oak),
            "pine w/h ratio {} must be lower than oak {}",
            occupied_width_height_ratio(&pine),
            occupied_width_height_ratio(&oak)
        );
    }

    #[test]
    fn test_birch_opaque_pixel_count_is_lower_than_oak() {
        let birch = foliage_image(find_spec(FoliageShape::Birch));
        let oak = foliage_image(find_spec(FoliageShape::Oak));
        // Birch and oak share the same canvas size so the comparison is fair.
        assert!(
            opaque_pixel_count(&birch) < opaque_pixel_count(&oak),
            "birch opaque {} must be less than oak {}",
            opaque_pixel_count(&birch),
            opaque_pixel_count(&oak)
        );
    }

    #[test]
    fn test_willow_lower_half_opaque_pixel_count_is_greater_than_upper_half() {
        let willow = foliage_image(find_spec(FoliageShape::Willow));
        assert!(
            lower_half_opaque_count(&willow) > upper_half_opaque_count(&willow),
            "willow lower half {} must exceed upper {}",
            lower_half_opaque_count(&willow),
            upper_half_opaque_count(&willow)
        );
    }

    #[test]
    fn test_palm_has_at_least_four_non_empty_angular_sectors_outside_center_radius() {
        let palm = foliage_image(find_spec(FoliageShape::Palm));
        let sectors = non_empty_angular_sectors_outside_center(&palm);
        assert!(
            sectors >= 4,
            "palm must fill ≥4 angular sectors; got {sectors}"
        );
    }

    #[test]
    fn test_shrub_occupied_height_ratio_is_lower_than_oak() {
        let shrub = foliage_image(find_spec(FoliageShape::Shrub));
        let oak = foliage_image(find_spec(FoliageShape::Oak));
        assert!(
            occupied_height_ratio(&shrub) < occupied_height_ratio(&oak),
            "shrub height ratio {} must be less than oak {}",
            occupied_height_ratio(&shrub),
            occupied_height_ratio(&oak)
        );
    }

    #[test]
    fn test_shrub_lower_half_density_is_greater_than_oak() {
        let shrub = foliage_image(find_spec(FoliageShape::Shrub));
        let oak_full = foliage_image(find_spec(FoliageShape::Oak));
        // Shrub is 64×64, oak is 128×128; compare ratios to total pixels.
        let shrub_total = (shrub.width() * shrub.height()) as usize;
        let oak_total = (oak_full.width() * oak_full.height()) as usize;
        let shrub_ratio = lower_half_opaque_count(&shrub) as f32 / shrub_total as f32;
        let oak_ratio = lower_half_opaque_count(&oak_full) as f32 / oak_total as f32;
        assert!(
            shrub_ratio > oak_ratio,
            "shrub lower density {shrub_ratio:.4} must exceed oak {oak_ratio:.4}"
        );
    }

    #[test]
    fn test_generate_foliage_texture_preserves_exact_required_dimensions() {
        let expected_dimensions = [
            (FoliageShape::Oak, FOLIAGE_WIDTH, FOLIAGE_HEIGHT),
            (FoliageShape::Pine, PINE_FOLIAGE_WIDTH, PINE_FOLIAGE_HEIGHT),
            (FoliageShape::Birch, FOLIAGE_WIDTH, FOLIAGE_HEIGHT),
            (FoliageShape::Willow, FOLIAGE_WIDTH, FOLIAGE_HEIGHT),
            (FoliageShape::Palm, FOLIAGE_WIDTH, FOLIAGE_HEIGHT),
            (FoliageShape::Shrub, SHRUB_FOLIAGE_SIZE, SHRUB_FOLIAGE_SIZE),
        ];
        for (shape, w, h) in expected_dimensions {
            let spec = find_spec(shape);
            let img = foliage_image(spec);
            assert_eq!(img.width(), w, "{:?} width", shape);
            assert_eq!(img.height(), h, "{:?} height", shape);
        }
    }

    // ── run_generate writes files to a temp dir ───────────────────────────────

    /// `run_generate` must create all expected files under the specified
    /// output directory without panicking.
    #[test]
    fn test_run_generate_writes_expected_files() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let base = tmp.path().to_path_buf();

        let args = TexturesGenerateArgs {
            output_dir: base.clone(),
        };
        run_generate(args).expect("run_generate should succeed");

        // Terrain
        let terrain_dir = base.join("terrain");
        for spec in TERRAIN_SPECS {
            assert!(
                terrain_dir.join(spec.filename).exists(),
                "missing terrain file: {}",
                spec.filename
            );
        }

        // Grass
        assert!(
            base.join("grass").join("grass_blade.png").exists(),
            "missing grass_blade.png"
        );

        // Trees
        let trees_dir = base.join("trees");
        assert!(trees_dir.join(BARK_FILENAME).exists(), "missing bark.png");
        for spec in FOLIAGE_SPECS {
            assert!(
                trees_dir.join(spec.filename).exists(),
                "missing foliage file: {}",
                spec.filename
            );
        }
    }
}
