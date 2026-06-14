// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Generate a normal map from `assets/textures/trees/bark.png`.
//!
//! Reads the bark diffuse texture, converts it to a per-pixel grayscale height
//! field using ITU-R BT.709 luminance weights, applies a 3×3 Sobel kernel to
//! estimate the surface gradient, and writes the result as an RGB normal map to:
//! - `assets/textures/trees/bark_normal.png`
//! - `campaigns/tutorial/assets/textures/trees/bark_normal.png`
//!
//! The Z channel always encodes the un-scaled surface normal pointing out of the
//! page, giving the classic blue-ish tint associated with normal maps.

use image::{DynamicImage, ImageBuffer, Rgb};
use std::path::Path;
use thiserror::Error;

// ── Error type ────────────────────────────────────────────────────────────────

/// Errors that can occur while generating or saving the normal map.
#[derive(Error, Debug)]
pub enum GeneratorError {
    /// An I/O error occurred (e.g. directory creation or file copy).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The `image` crate failed to open, decode, or encode a PNG.
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
}

// ── Constants ─────────────────────────────────────────────────────────────────

/// Path to the source bark diffuse texture, relative to `CARGO_MANIFEST_DIR`.
pub const BARK_TEXTURE_PATH: &str = "assets/textures/trees/bark.png";

/// Path to write the generated normal map, relative to `CARGO_MANIFEST_DIR`.
pub const BARK_NORMAL_OUTPUT_PATH: &str = "assets/textures/trees/bark_normal.png";

/// Divisor applied to Sobel gradients before normalisation.
///
/// Larger values produce a flatter (less bumpy) surface; smaller values
/// exaggerate the surface relief.  4.0 gives a moderate bumpiness for bark.
pub const SOBEL_SCALE: f32 = 4.0;

/// ITU-R BT.709 luminance weight for the red channel.
pub const LUM_R: f32 = 0.2126;

/// ITU-R BT.709 luminance weight for the green channel.
pub const LUM_G: f32 = 0.7152;

/// ITU-R BT.709 luminance weight for the blue channel.
pub const LUM_B: f32 = 0.0722;

// ── Helper functions ──────────────────────────────────────────────────────────

/// Samples the grayscale height of `pixels` at `(x, y)` with clamp-to-edge.
///
/// Out-of-bounds coordinates are clamped to `[0, width-1]` / `[0, height-1]`
/// rather than wrapping, which avoids seam artefacts at texture borders.
///
/// # Arguments
/// * `pixels` – Flat row-major slice of grayscale `f32` values in `[0, 1]`.
/// * `width`  – Image width in pixels.
/// * `height` – Image height in pixels.
/// * `x`      – Column index (may be negative or ≥ width).
/// * `y`      – Row index (may be negative or ≥ height).
///
/// # Returns
/// The grayscale value at the clamped coordinate.
pub fn sample_gray(pixels: &[f32], width: u32, height: u32, x: i32, y: i32) -> f32 {
    let cx = x.clamp(0, width as i32 - 1) as u32;
    let cy = y.clamp(0, height as i32 - 1) as u32;
    pixels[(cy * width + cx) as usize]
}

/// Converts a `DynamicImage` to a normal map using a 3×3 Sobel filter.
///
/// The pipeline is:
/// 1. Convert each pixel to a grayscale height value using BT.709 luminance
///    weights.
/// 2. For each pixel, convolve the 3×3 neighbourhood with the Sobel X and Y
///    kernels to obtain the surface gradient (dx, dy).
/// 3. Scale the gradient by [`SOBEL_SCALE`] and compute the normalised surface
///    normal `(−dx, −dy, 1).normalize()`.
/// 4. Remap each component from `[−1, 1]` to `[0, 255]`.
/// 5. Pack as `Rgb<u8>` where R = dx, G = dy, B = Z.
///
/// # Arguments
/// * `img` – Source image; any bit-depth or colour model is accepted because
///   it is first converted to `Rgb8` internally.
///
/// # Returns
/// An `ImageBuffer<Rgb<u8>, Vec<u8>>` of the same dimensions as `img`.
pub fn generate_normal_map(img: &DynamicImage) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let rgb = img.to_rgb8();
    let width = rgb.width();
    let height = rgb.height();

    // Build the grayscale height field.
    let gray: Vec<f32> = rgb
        .pixels()
        .map(|p| {
            let Rgb([r, g, b]) = *p;
            LUM_R * (r as f32 / 255.0) + LUM_G * (g as f32 / 255.0) + LUM_B * (b as f32 / 255.0)
        })
        .collect();

    let mut out = ImageBuffer::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let xi = x as i32;
            let yi = y as i32;

            // Sobel X kernel: [[-1,0,1],[-2,0,2],[-1,0,1]]
            let dx = -sample_gray(&gray, width, height, xi - 1, yi - 1)
                + sample_gray(&gray, width, height, xi + 1, yi - 1)
                - 2.0 * sample_gray(&gray, width, height, xi - 1, yi)
                + 2.0 * sample_gray(&gray, width, height, xi + 1, yi)
                - sample_gray(&gray, width, height, xi - 1, yi + 1)
                + sample_gray(&gray, width, height, xi + 1, yi + 1);

            // Sobel Y kernel: [[-1,-2,-1],[0,0,0],[1,2,1]]
            let dy = -sample_gray(&gray, width, height, xi - 1, yi - 1)
                - 2.0 * sample_gray(&gray, width, height, xi, yi - 1)
                - sample_gray(&gray, width, height, xi + 1, yi - 1)
                + sample_gray(&gray, width, height, xi - 1, yi + 1)
                + 2.0 * sample_gray(&gray, width, height, xi, yi + 1)
                + sample_gray(&gray, width, height, xi + 1, yi + 1);

            // Scale and build the normal vector, then normalise.
            let nx = dx / SOBEL_SCALE;
            let ny = dy / SOBEL_SCALE;
            let nz = 1.0_f32;
            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            let (nx, ny, nz) = (nx / len, ny / len, nz / len);

            // Remap [-1, 1] → [0, 255].
            let remap =
                |v: f32| -> u8 { ((v + 1.0) * 0.5 * 255.0).round().clamp(0.0, 255.0) as u8 };

            out.put_pixel(x, y, Rgb([remap(nx), remap(ny), remap(nz)]));
        }
    }

    out
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() -> Result<(), GeneratorError> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    // Load the source bark texture.
    let source_path = Path::new(manifest_dir).join(BARK_TEXTURE_PATH);
    let img = image::open(&source_path)?;

    // Generate the normal map.
    let normal_map = generate_normal_map(&img);

    // Write to the primary assets location.
    let assets_path = Path::new(manifest_dir).join(BARK_NORMAL_OUTPUT_PATH);
    if let Some(parent) = assets_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    normal_map.save(&assets_path)?;
    println!("Generated: {}", assets_path.display());

    // Copy to the campaign assets directory.
    let campaign_path = Path::new(manifest_dir)
        .join("campaigns/tutorial/assets/textures/trees")
        .join("bark_normal.png");
    if let Some(parent) = campaign_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(&assets_path, &campaign_path)?;
    println!("Copied: {}", campaign_path.display());

    Ok(())
}
