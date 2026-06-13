// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Instanced grass vertex shader.
//!
//! Vertex buffer 0 (step Vertex):  position, normal, UV, vertex color — the
//! standard Bevy mesh attributes for a grass clump card mesh.
//!
//! Vertex buffer 1 (step Instance): per-instance transform and wind phase.
//! Each instance corresponds to one grass clump drawn at a specific world
//! location.
//!
//! Wind paths (identical to `grass.wgsl`):
//!   0 = None  — static geometry
//!   1 = Sine  — world-coherent sinusoidal sway
//!   2 = Perlin — noise-texture-driven spatially-varying gusts
//!
//! Bind groups:
//!   @group(0) — view (camera, globals) — set by Bevy's `SetMeshViewBindGroup`
//!   @group(1) — mesh-view binding array (lights, shadow maps) — Bevy
//!   @group(2) — mesh uniforms (per-object transform, identity for batches)
//!   @group(3) — grass wind uniforms + noise texture (this file)

#import bevy_pbr::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

// ── Per-instance vertex attributes ────────────────────────────────────────────
// Each attribute occupies a distinct @location that does not collide with the
// standard mesh attributes (0 = position, 1 = normal, 2 = uv, 5 = color).

struct GrassInstance {
    /// World-space position of this clump.
    @location(8)  i_position:  vec3<f32>,
    /// Wind phase offset for staggered animation across neighbouring clumps.
    @location(9)  i_phase:     f32,
    /// Ground-surface normal for this clump location (default: up).
    @location(10) i_normal:    vec3<f32>,
    /// Uniform scale applied to the clump mesh before placing it.
    @location(11) i_scale:     f32,
    /// Y-axis rotation applied to the clump mesh (radians).
    @location(12) i_rotation_y: f32,
}

// ── Wind bind group ────────────────────────────────────────────────────────────
// @group(3): placed after Bevy's three standard groups (view, mesh-view-array,
// mesh-uniforms) that `SpecializedMeshPipeline::specialize` establishes.

struct GrassWindUniform {
    strength:     f32,
    frequency:    f32,
    direction:    vec2<f32>,
    wind_system:  u32,   // 0 = None, 1 = Sine, 2 = Perlin
    perlin_scale: f32,
    _pad:         vec2<f32>,
}

@group(3) @binding(0) var<uniform> wind: GrassWindUniform;
@group(3) @binding(1) var wind_noise:   texture_2d<f32>;
@group(3) @binding(2) var wind_sampler: sampler;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Rotate a vector around the Y axis by `angle` radians.
fn rotate_y(v: vec3<f32>, angle: f32) -> vec3<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return vec3<f32>(v.x * c - v.z * s, v.y, v.x * s + v.z * c);
}

// ── Vertex entry point ────────────────────────────────────────────────────────

@vertex
fn vertex(vertex_no_morph: Vertex, instance: GrassInstance) -> VertexOutput {
    var out: VertexOutput;

    // --- Apply per-instance scale and Y-axis rotation to the blade geometry.
    //     The clump card mesh is in local space centred at the origin; we
    //     scale, rotate, then place it at instance.i_position in world space.
    let local_scaled = vertex_no_morph.position * instance.i_scale;
    let local_rotated = rotate_y(local_scaled, instance.i_rotation_y);
    var world_pos = vec4<f32>(local_rotated + instance.i_position, 1.0);

    // --- Wind sway.
    //     `vertex_no_morph.uv.y` is 0 at the blade base and 1 at the tip.
    //     Squaring it anchors the base while the tip sways at full amplitude.
    var sway = 0.0f;
    if wind.wind_system == 1u {
        // Sine: world-coherent phase keeps adjacent blades aligned.
        let t = globals.time * wind.frequency
            + world_pos.x * 0.17
            + world_pos.z * 0.13
            + instance.i_phase;
        sway = wind.strength * vertex_no_morph.uv.y * vertex_no_morph.uv.y * sin(t);
    } else if wind.wind_system == 2u {
        // Perlin: scroll a tiling noise texture through the world.
        // textureSampleLevel is required in the vertex stage (no implicit LOD).
        let scrolled_uv = world_pos.xz / wind.perlin_scale
            + wind.direction * globals.time * wind.frequency;
        let noise = textureSampleLevel(wind_noise, wind_sampler, scrolled_uv, 0.0).r;
        sway = wind.strength * vertex_no_morph.uv.y * vertex_no_morph.uv.y
            * (noise * 2.0 - 1.0 + instance.i_phase * 0.3);
    }
    // wind.wind_system == 0  →  sway = 0.0 (no displacement)

    world_pos.x += sway * wind.direction.x;
    world_pos.z += sway * wind.direction.y;

    // Recompute clip position from the (potentially displaced) world position.
    out.world_position = world_pos;
    out.position = position_world_to_clip(world_pos.xyz);

    // Rotate the vertex normal the same way as the position so that lighting
    // (if enabled) stays correct after per-instance rotation.
    let rotated_normal = rotate_y(vertex_no_morph.normal, instance.i_rotation_y);
    out.world_normal = rotated_normal;

    out.uv = vertex_no_morph.uv;

#ifdef VERTEX_COLORS
    out.color = vertex_no_morph.color;
#endif

    return out;
}
