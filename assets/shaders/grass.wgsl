// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Grass wind vertex shader for `ExtendedMaterial<StandardMaterial, GrassWindExtension>`.
//!
//! Displaces grass blade vertices along `wind.direction` using one of three
//! wind algorithms selected by `wind.wind_system`:
//!   0 = None   → no displacement
//!   1 = Sine   → sinusoidal sway (world-coherent phase)
//!   2 = Perlin → noise-texture-driven gusts

#import bevy_pbr::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

// ── Wind extension bindings ────────────────────────────────────────────────────
// StandardMaterial occupies the low binding indices of @group(2); extension
// bindings start at 100 to avoid collisions.

struct GrassWindUniform {
    strength:     f32,
    frequency:    f32,
    direction:    vec2<f32>,
    wind_system:  u32,    // 0 = None, 1 = Sine, 2 = Perlin
    perlin_scale: f32,
    _pad:         vec2<f32>,
}

@group(2) @binding(100) var<uniform> wind: GrassWindUniform;
@group(2) @binding(101) var wind_noise: texture_2d<f32>;
@group(2) @binding(102) var wind_sampler: sampler;

// ── Vertex entry point ─────────────────────────────────────────────────────────

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
    var world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex_no_morph.position, 1.0),
    );

    // Compute per-vertex sway based on the active wind system.
    // `vertex_no_morph.uv.y` encodes blade height 0→1; squaring it anchors
    // the base while the tip sways at full amplitude.
    var sway = 0.0f;

    if wind.wind_system == 1u {
        // Sine wind: world-coherent phase ensures adjacent blades align.
        let t = globals.time * wind.frequency
            + world_position.x * 0.17
            + world_position.z * 0.13;
        sway = wind.strength * vertex_no_morph.uv.y * vertex_no_morph.uv.y * sin(t);

    } else if wind.wind_system == 2u {
        // Perlin wind: scroll a noise texture through the world.
        // `textureSampleLevel` is required in vertex stage (no implicit LOD).
        let scrolled_uv = world_position.xz / wind.perlin_scale
            + wind.direction * globals.time * wind.frequency;
        let noise = textureSampleLevel(wind_noise, wind_sampler, scrolled_uv, 0.0).r;
        sway = wind.strength * vertex_no_morph.uv.y * vertex_no_morph.uv.y * (noise * 2.0 - 1.0);
    }
    // wind.wind_system == 0  →  sway = 0.0 (no displacement)

    // Apply sway along the configured wind direction.
    world_position.x += sway * wind.direction.x;
    world_position.z += sway * wind.direction.y;

    // Recompute clip-space position from the displaced world position.
    // Displacing `world_position` alone does not move geometry — the clip
    // position must be recomputed explicitly.
    out.world_position = world_position;
    out.position = position_world_to_clip(world_position.xyz);

    // Forward standard vertex attributes.
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex_no_morph.normal,
        vertex_no_morph.instance_index,
    );
    out.uv = vertex_no_morph.uv;

#ifdef VERTEX_COLORS
    out.color = vertex_no_morph.color;
#endif

    return out;
}
