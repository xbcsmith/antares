// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! GPU instancing pipeline for grass rendering.
//!
//! Connects the existing [`GrassInstanceBatch`] infrastructure to an actual
//! render pass so all clumps within a (mesh, material, map) group share a
//! single indexed draw call, dramatically reducing entity count and CPU
//! overhead on grass-dense maps.
//!
//! # Architecture
//!
//! The pipeline follows Bevy 0.17's `custom_shader_instancing` example:
//!
//! 1. [`GrassInstanceBatch`] is extracted to the render world via
//!    [`ExtractComponentPlugin`].
//! 2. [`prepare_grass_instance_buffers`] uploads per-instance data to a GPU
//!    [`Buffer`] with [`BufferUsages::VERTEX`].
//! 3. [`prepare_grass_wind_bind_group`] creates or refreshes the wind uniform
//!    bind group from extracted wind resources.
//! 4. [`queue_grass_instanced`] queues each batch into the [`Opaque3d`] render
//!    phase using [`BinnedRenderPhaseType::NonMesh`].
//! 5. [`DrawGrassInstanced`] sets the pipeline, view bind group, mesh bind
//!    group, wind bind group, and calls `draw_indexed`.
//!
//! # Render modes
//!
//! The [`GrassRenderMode`] resource gates which path is active:
//! - [`GrassRenderMode::PerEntity`]: Phase-6 `ExtendedMaterial` path — each
//!   clump is a distinct `Mesh3d`/`MeshMaterial3d` entity.
//! - [`GrassRenderMode::Instanced`] (default): this module — batch entities
//!   without per-clump render components; `GrassInstanceBatch` entities carry
//!   the `Mesh3d` needed for GPU buffer allocation.

use bevy::core_pipeline::core_3d::{Opaque3d, Opaque3dBatchSetKey, Opaque3dBinKey};
use bevy::ecs::{
    component::Tick,
    query::ROQueryItem,
    system::{lifetimeless::*, SystemParamItem},
};
use bevy::mesh::{MeshVertexBufferLayoutRef, VertexBufferLayout};
use bevy::pbr::{
    MeshPipeline, MeshPipelineKey, RenderMeshInstances, SetMeshBindGroup, SetMeshViewBindGroup,
    SetMeshViewBindingArrayBindGroup,
};
use bevy::prelude::*;
use bevy::render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    mesh::{allocator::MeshAllocator, RenderMesh, RenderMeshBufferInfo},
    render_asset::RenderAssets,
    render_phase::{
        AddRenderCommand, BinnedRenderPhaseType, DrawFunctions, InputUniformIndex, PhaseItem,
        RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
        ViewBinnedRenderPhases,
    },
    render_resource::{
        BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry, BindingResource,
        BindingType, BufferBindingType, BufferInitDescriptor, BufferUsages, PipelineCache,
        RenderPipelineDescriptor, SamplerBindingType, ShaderStages, SpecializedMeshPipeline,
        SpecializedMeshPipelineError, SpecializedMeshPipelines, TextureSampleType,
        TextureViewDimension, VertexAttribute, VertexFormat, VertexStepMode,
    },
    renderer::RenderDevice,
    sync_world::MainEntity,
    view::{ExtractedView, NoIndirectDrawing},
    Render, RenderApp, RenderStartup, RenderSystems,
};
use bytemuck::{Pod, Zeroable};
use std::num::NonZeroU64;

use crate::domain::world;
use crate::game::resources::WindConfig;
use crate::game::systems::advanced_grass::{
    GrassInstanceBatch, GrassWindUniform, WindNoiseTexture,
};

// ── Render mode resource ───────────────────────────────────────────────────────

/// Marker resource inserted into the main world only when the Bevy `RenderApp`
/// sub-application is present.
///
/// [`build_grass_instance_batches_system`] checks for this resource before
/// spawning [`Mesh3d`] components on batch entities.  Without a `RenderApp`,
/// `Mesh3d` triggers a render-world sync hook that panics on a missing
/// `PendingSyncEntity` resource — this guard prevents that in test environments
/// that use `MinimalPlugins`.
#[derive(Resource, Default)]
pub struct GrassRenderWorldAvailable;

/// Controls which grass render path is active.
///
/// Set to [`GrassRenderMode::Instanced`] (the default) to enable the Phase-7
/// GPU instancing pipeline.  Set to [`GrassRenderMode::PerEntity`] to fall
/// back to the Phase-6 `ExtendedMaterial` path where each clump is a separate
/// `Mesh3d`/`MeshMaterial3d` entity.
///
/// Both paths MUST NOT run simultaneously — the resource gate enforces this.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::grass_instancing::GrassRenderMode;
///
/// assert_eq!(GrassRenderMode::default(), GrassRenderMode::Instanced);
/// ```
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GrassRenderMode {
    /// Phase-6 path: each grass clump is a separate `Mesh3d` entity.
    PerEntity,
    /// Phase-7 path (default): GPU instancing via `GrassInstanceBatch`.
    #[default]
    Instanced,
}

// ── Per-instance GPU data ─────────────────────────────────────────────────────

/// Per-instance data for vertex buffer 1 of the instanced grass pipeline.
///
/// This struct is tightly packed to 48 bytes so that `bytemuck::cast_slice`
/// can upload a `&[GrassInstanceGpu]` directly to a GPU vertex buffer.
///
/// # Vertex attribute locations
///
/// | Field        | `@location` | Format           | Offset |
/// |---|---|---|---|
/// | `position`   | 8           | `Float32x3`     | 0      |
/// | `phase`      | 9           | `Float32`       | 12     |
/// | `normal`     | 10          | `Float32x3`     | 16     |
/// | `scale`      | 11          | `Float32`       | 28     |
/// | `rotation_y` | 12          | `Float32`       | 32     |
/// | `_pad`       | —           | padding          | 36     |
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::grass_instancing::GrassInstanceGpu;
///
/// let inst = GrassInstanceGpu {
///     position: [1.0, 0.0, 2.0],
///     phase: 0.5,
///     normal: [0.0, 1.0, 0.0],
///     scale: 1.0,
///     rotation_y: 0.0,
///     _pad: [0.0; 3],
/// };
/// assert_eq!(std::mem::size_of::<GrassInstanceGpu>(), 48);
/// ```
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GrassInstanceGpu {
    /// World-space XYZ of this clump.
    pub position: [f32; 3],
    /// Wind phase offset for staggered animation.
    pub phase: f32,
    /// Ground-surface normal (default: [0, 1, 0]).
    pub normal: [f32; 3],
    /// Uniform scale applied to the clump mesh.
    pub scale: f32,
    /// Y-axis rotation of the clump mesh in radians.
    pub rotation_y: f32,
    /// Alignment padding.
    pub _pad: [f32; 3],
}

impl From<&world::InstanceData> for GrassInstanceGpu {
    fn from(data: &world::InstanceData) -> Self {
        // Derive a per-clump wind phase from the world position so that
        // neighbouring blades don't all sway in perfect synchrony.
        let phase =
            (data.position[0] * 73.0 + data.position[2] * 47.0).sin() * std::f32::consts::PI;
        Self {
            position: data.position,
            phase,
            normal: [0.0, 1.0, 0.0],
            scale: data.scale,
            rotation_y: data.rotation_y,
            _pad: [0.0; 3],
        }
    }
}

// ── Vertex buffer layout constants ────────────────────────────────────────────

/// Byte stride of one [`GrassInstanceGpu`] in the instance vertex buffer.
pub const GRASS_INSTANCE_STRIDE: u64 = std::mem::size_of::<GrassInstanceGpu>() as u64;

/// Starting `@location` for instance vertex attributes.
///
/// The standard Bevy mesh attributes occupy locations 0–7; instance data
/// starts at 8 to guarantee no collision regardless of which mesh features
/// are active.
pub const GRASS_INSTANCE_ATTR_OFFSET: u32 = 8;

// ── ExtractComponent for GrassInstanceBatch ───────────────────────────────────

// We implement ExtractComponent manually so we can clone the component into
// the render world without re-deriving the full component.
impl ExtractComponent for GrassInstanceBatch {
    type QueryData = &'static GrassInstanceBatch;
    type QueryFilter = ();
    type Out = GrassInstanceBatch;

    fn extract_component(item: ROQueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(GrassInstanceBatch {
            mesh: item.mesh.clone(),
            material: item.material.clone(),
            instances: item.instances.clone(),
        })
    }
}

// ── Render-world GPU buffer component ─────────────────────────────────────────

/// Render-world component holding the uploaded instance vertex buffer.
///
/// Created each frame by [`prepare_grass_instance_buffers`] from the
/// extracted [`GrassInstanceBatch`] component.
#[derive(Component)]
pub struct GrassInstanceBuffer {
    /// GPU vertex buffer containing all per-instance data.
    pub buffer: bevy::render::render_resource::Buffer,
    /// Number of instances (draw call `instance_count`).
    pub count: u32,
}

// ── Wind bind group resource ───────────────────────────────────────────────────

/// Render-world resource holding the grass wind bind group.
///
/// Contains the wind uniform buffer, noise texture, and noise sampler bound
/// to @group(3) of the instanced grass pipeline.
#[derive(Resource, Default)]
pub struct GrassWindBindGroupResource {
    /// The GPU bind group, or `None` if the pipeline is not yet ready.
    pub bind_group: Option<BindGroup>,
}

// ── Pipeline ──────────────────────────────────────────────────────────────────

/// Asset path for the instanced grass vertex shader.
const GRASS_INSTANCED_SHADER_PATH: &str = "assets/shaders/grass_instanced.wgsl";

/// Custom mesh pipeline for GPU-instanced grass.
///
/// Wraps Bevy's [`MeshPipeline`] to inherit the standard view and mesh bind
/// group layouts, then appends:
/// - A second vertex buffer (step mode Instance) for per-clump data.
/// - A fourth bind group layout for wind uniforms and the Perlin noise texture.
#[derive(Resource)]
pub struct GrassInstancedPipeline {
    /// Base Bevy mesh pipeline (provides vertex layout + bind group layouts 0–2).
    pub mesh_pipeline: MeshPipeline,
    /// Bind group layout for the wind uniform + noise texture (@group(3)).
    pub wind_bind_group_layout: BindGroupLayout,
    /// Handle for `assets/shaders/grass_instanced.wgsl`.
    pub shader: Handle<Shader>,
}

impl SpecializedMeshPipeline for GrassInstancedPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        // Build the base descriptor from MeshPipeline. This gives us the
        // correct vertex buffer 0 layout for the grass mesh attributes
        // (position, normal, UV, color) and the three standard bind group
        // layouts (view, mesh-view-array, mesh-uniforms).
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;

        // Override only the VERTEX shader — our custom shader defines a
        // @vertex entry point that applies per-instance transforms and wind
        // displacement.  The fragment stage is left as-is so the standard
        // PBR fragment shader handles lighting; overriding the fragment with
        // a shader that has no @fragment entry point causes a wgpu validation
        // error ("no entry point was found").
        descriptor.vertex.shader = self.shader.clone();

        // Append vertex buffer 1 — per-instance data (VertexStepMode::Instance).
        descriptor.vertex.buffers.push(VertexBufferLayout {
            array_stride: GRASS_INSTANCE_STRIDE,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: GRASS_INSTANCE_ATTR_OFFSET, // i_position
                },
                VertexAttribute {
                    format: VertexFormat::Float32,
                    offset: 12,
                    shader_location: GRASS_INSTANCE_ATTR_OFFSET + 1, // i_phase
                },
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 16,
                    shader_location: GRASS_INSTANCE_ATTR_OFFSET + 2, // i_normal
                },
                VertexAttribute {
                    format: VertexFormat::Float32,
                    offset: 28,
                    shader_location: GRASS_INSTANCE_ATTR_OFFSET + 3, // i_scale
                },
                VertexAttribute {
                    format: VertexFormat::Float32,
                    offset: 32,
                    shader_location: GRASS_INSTANCE_ATTR_OFFSET + 4, // i_rotation_y
                },
            ],
        });

        // Append the wind bind group layout at index 3 (after the three
        // layouts that MeshPipeline::specialize inserts at indices 0–2).
        descriptor.layout.push(self.wind_bind_group_layout.clone());

        Ok(descriptor)
    }
}

// ── SetGrassWindBindGroup render command ──────────────────────────────────────

/// Render command that binds the grass wind uniform to @group(I).
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::grass_instancing::SetGrassWindBindGroup;
/// // Used as part of the DrawGrassInstanced tuple type alias.
/// ```
pub struct SetGrassWindBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetGrassWindBindGroup<I> {
    type Param = SRes<GrassWindBindGroupResource>;
    type ViewQuery = ();
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, ()>,
        _entity: Option<ROQueryItem<'w, '_, ()>>,
        wind_bg: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(bind_group) = &wind_bg.into_inner().bind_group else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(I, bind_group, &[]);
        RenderCommandResult::Success
    }
}

// ── DrawGrassInstancedInner render command ────────────────────────────────────

/// The final draw step of the instanced grass pipeline.
///
/// Reads the mesh vertex/index buffers from the [`MeshAllocator`] and the
/// instance buffer from the [`GrassInstanceBuffer`] component, then issues a
/// single `draw_indexed(0..index_count, 0, 0..instance_count)` call.
pub struct DrawGrassInstancedInner;

impl<P: PhaseItem> RenderCommand<P> for DrawGrassInstancedInner {
    type Param = (
        SRes<RenderAssets<RenderMesh>>,
        SRes<RenderMeshInstances>,
        SRes<MeshAllocator>,
    );
    type ViewQuery = ();
    type ItemQuery = Read<GrassInstanceBuffer>;

    fn render<'w>(
        item: &P,
        _view: ROQueryItem<'w, '_, ()>,
        instance_buffer: Option<ROQueryItem<'w, '_, Read<GrassInstanceBuffer>>>,
        (meshes, render_mesh_instances, mesh_allocator): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mesh_allocator = mesh_allocator.into_inner();

        let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(item.main_entity())
        else {
            return RenderCommandResult::Skip;
        };
        let Some(gpu_mesh) = meshes.into_inner().get(mesh_instance.mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };
        let Some(instance_buffer) = instance_buffer else {
            return RenderCommandResult::Skip;
        };
        if instance_buffer.count == 0 {
            return RenderCommandResult::Skip;
        }
        let Some(vertex_buffer_slice) =
            mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id)
        else {
            return RenderCommandResult::Skip;
        };

        pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

        match &gpu_mesh.buffer_info {
            RenderMeshBufferInfo::Indexed {
                index_format,
                count,
            } => {
                let Some(index_buffer_slice) =
                    mesh_allocator.mesh_index_slice(&mesh_instance.mesh_asset_id)
                else {
                    return RenderCommandResult::Skip;
                };

                pass.set_index_buffer(index_buffer_slice.buffer.slice(..), 0, *index_format);
                pass.draw_indexed(
                    index_buffer_slice.range.start..(index_buffer_slice.range.start + count),
                    vertex_buffer_slice.range.start as i32,
                    0..instance_buffer.count,
                );
            }
            RenderMeshBufferInfo::NonIndexed => {
                pass.draw(vertex_buffer_slice.range.clone(), 0..instance_buffer.count);
            }
        }

        RenderCommandResult::Success
    }
}

// ── DrawGrassInstanced type alias ─────────────────────────────────────────────

/// The full render-command chain for instanced grass.
///
/// Executed once per [`GrassInstanceBatch`] entity in the [`Opaque3d`] phase.
pub type DrawGrassInstanced = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshViewBindingArrayBindGroup<1>,
    SetMeshBindGroup<2>,
    SetGrassWindBindGroup<3>,
    DrawGrassInstancedInner,
);

// ── Systems ───────────────────────────────────────────────────────────────────

/// Initialises the [`GrassInstancedPipeline`] resource in the render world.
///
/// Called once at [`RenderStartup`]; must run before map spawn so the pipeline
/// is ready when grass is first queued.
pub fn init_grass_instanced_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mesh_pipeline: Res<MeshPipeline>,
    render_device: Res<RenderDevice>,
) {
    let shader = asset_server.load(GRASS_INSTANCED_SHADER_PATH);

    // @group(3) bind group layout: wind uniform buffer + noise texture + sampler.
    let wind_bind_group_layout = render_device.create_bind_group_layout(
        "grass_wind_bind_group_layout",
        &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    // GrassWindUniform is 32 bytes:
                    // strength(4) + frequency(4) + direction(8) +
                    // wind_system(4) + perlin_scale(4) + _pad(8) = 32
                    min_binding_size: NonZeroU64::new(32),
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Texture {
                    multisampled: false,
                    view_dimension: TextureViewDimension::D2,
                    sample_type: TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
    );

    commands.insert_resource(GrassInstancedPipeline {
        mesh_pipeline: mesh_pipeline.clone(),
        wind_bind_group_layout,
        shader,
    });
}

/// Uploads per-instance data from each [`GrassInstanceBatch`] to a GPU buffer.
///
/// Runs in [`RenderSystems::PrepareResources`].  The resulting
/// [`GrassInstanceBuffer`] component is read by [`DrawGrassInstancedInner`].
pub fn prepare_grass_instance_buffers(
    mut commands: Commands,
    query: Query<(Entity, &GrassInstanceBatch)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, batch) in &query {
        if batch.instances.is_empty() {
            continue;
        }

        let gpu_instances: Vec<GrassInstanceGpu> =
            batch.instances.iter().map(GrassInstanceGpu::from).collect();

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("grass_instance_buffer"),
            contents: bytemuck::cast_slice(gpu_instances.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        commands.entity(entity).insert(GrassInstanceBuffer {
            buffer,
            count: gpu_instances.len() as u32,
        });
    }
}

/// Creates or refreshes the grass wind bind group in the render world.
///
/// Runs in [`RenderSystems::PrepareBindGroups`].  Reads the extracted
/// [`WindConfig`] and [`WindNoiseTexture`] resources; falls back gracefully
/// when they are absent (static grass).
pub fn prepare_grass_wind_bind_group(
    mut wind_bg: ResMut<GrassWindBindGroupResource>,
    pipeline: Option<Res<GrassInstancedPipeline>>,
    wind_config: Option<Res<WindConfig>>,
    noise_texture: Option<Res<WindNoiseTexture>>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<bevy::render::texture::GpuImage>>,
    samplers: Res<bevy::render::texture::FallbackImage>,
) {
    let Some(pipeline) = pipeline else {
        return;
    };

    // Build the wind uniform bytes (32 bytes matching GrassWindUniform WGSL).
    let wind_uniform = wind_config
        .as_deref()
        .map(|wc| GrassWindUniform::from_config(&wc.0))
        .unwrap_or_default();

    // Pack the GrassWindUniform struct into raw bytes for the GPU buffer.
    let wind_bytes: [f32; 8] = [
        wind_uniform.strength,
        wind_uniform.frequency,
        wind_uniform.direction.x,
        wind_uniform.direction.y,
        f32::from_bits(wind_uniform.wind_system),
        wind_uniform.perlin_scale,
        0.0, // _pad.x
        0.0, // _pad.y
    ];

    let wind_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("grass_wind_uniform_buffer"),
        contents: bytemuck::cast_slice(&wind_bytes),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    // Get the noise texture GPU image; fall back to the 1×1 white fallback.
    let noise_gpu = noise_texture.as_ref().and_then(|nt| images.get(&nt.0));

    let (noise_view, noise_sampler) = if let Some(gpu) = noise_gpu {
        (&*gpu.texture_view, &*gpu.sampler)
    } else {
        (&*samplers.d2.texture_view, &*samplers.d2.sampler)
    };

    let bind_group = render_device.create_bind_group(
        "grass_wind_bind_group",
        &pipeline.wind_bind_group_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: wind_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::TextureView(noise_view),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::Sampler(noise_sampler),
            },
        ],
    );

    wind_bg.bind_group = Some(bind_group);
}

/// Queues each [`GrassInstanceBatch`] entity into the [`Opaque3d`] render phase.
///
/// Uses [`BinnedRenderPhaseType::NonMesh`] to bypass Bevy's GPU preprocessing
/// machinery; the custom instanced draw command handles everything directly.
///
/// Runs in [`RenderSystems::QueueMeshes`].
#[allow(clippy::too_many_arguments)]
pub fn queue_grass_instanced(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    pipeline: Option<Res<GrassInstancedPipeline>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<GrassInstancedPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    batches: Query<(Entity, &MainEntity), With<GrassInstanceBatch>>,
    mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    views: Query<(Entity, &ExtractedView, &Msaa, Option<&NoIndirectDrawing>)>,
    mut next_tick: Local<Tick>,
) {
    let Some(pipeline) = pipeline else {
        return;
    };

    let draw_function = opaque_draw_functions.read().id::<DrawGrassInstanced>();

    for (_view_entity, view, msaa, _no_indirect) in &views {
        let Some(opaque_phase) = opaque_render_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples());
        let view_key = msaa_key | MeshPipelineKey::from_hdr(view.hdr);

        for (entity, main_entity) in &batches {
            // Retrieve the mesh from RenderMeshInstances (the batch entity
            // has Mesh3d so it is registered there).
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*main_entity)
            else {
                continue;
            };
            let Some(gpu_mesh) = meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };

            let pipeline_key =
                view_key | MeshPipelineKey::from_primitive_topology(gpu_mesh.primitive_topology());

            let Ok(pipeline_id) =
                pipelines.specialize(&pipeline_cache, &pipeline, pipeline_key, &gpu_mesh.layout)
            else {
                continue;
            };

            // Bump change tick so Bevy doesn't skip rebuilding the bin.
            let this_tick = next_tick.get() + 1;
            next_tick.set(this_tick);

            opaque_phase.add(
                Opaque3dBatchSetKey {
                    pipeline: pipeline_id,
                    draw_function,
                    material_bind_group_index: None,
                    lightmap_slab: None,
                    vertex_slab: default(),
                    index_slab: None,
                },
                Opaque3dBinKey {
                    asset_id: mesh_instance.mesh_asset_id.untyped(),
                },
                (entity, *main_entity),
                InputUniformIndex::default(),
                BinnedRenderPhaseType::NonMesh,
                *next_tick,
            );
        }
    }
}

// ── Plugin ────────────────────────────────────────────────────────────────────

/// Bevy plugin that wires up all GPU instancing systems for grass.
///
/// Added by [`MapRenderingPlugin`]; registers [`GrassRenderMode`],
/// the pipeline, the extraction component, and the prepare/queue systems.
///
/// # Examples
///
/// ```rust
/// use antares::game::systems::grass_instancing::GrassInstancingPlugin;
/// use bevy::prelude::App;
///
/// let mut app = App::new();
/// // GrassInstancingPlugin is added by MapRenderingPlugin automatically.
/// // app.add_plugins(GrassInstancingPlugin);
/// let _ = app; // silence unused warning
/// ```
pub struct GrassInstancingPlugin;

impl Plugin for GrassInstancingPlugin {
    fn build(&self, app: &mut App) {
        // Register GrassRenderMode (default: Instanced) in the main world.
        app.init_resource::<GrassRenderMode>();

        // Check if a render sub-app is present before borrowing it mutably.
        let has_render_app = app.get_sub_app(RenderApp).is_some();

        if has_render_app {
            // ExtractComponentPlugin always adds SyncComponentPlugin<GrassInstanceBatch>
            // which registers a component hook that accesses PendingSyncEntity.
            // That resource only exists when RenderApp is present, so we must
            // gate this plugin on has_render_app to avoid panics in test
            // environments that use MinimalPlugins.
            app.add_plugins(ExtractComponentPlugin::<GrassInstanceBatch>::default());

            // Mark the main world as render-capable so batch-building can safely
            // spawn Mesh3d entities.  Mesh3d also registers a sync hook; the
            // guard prevents that panic in MinimalPlugins environments.
            app.init_resource::<GrassRenderWorldAvailable>();

            let render_app = app.sub_app_mut(RenderApp);
            render_app
                .add_render_command::<Opaque3d, DrawGrassInstanced>()
                .init_resource::<SpecializedMeshPipelines<GrassInstancedPipeline>>()
                .init_resource::<GrassWindBindGroupResource>()
                .add_systems(RenderStartup, init_grass_instanced_pipeline)
                .add_systems(
                    Render,
                    (
                        prepare_grass_instance_buffers.in_set(RenderSystems::PrepareResources),
                        prepare_grass_wind_bind_group.in_set(RenderSystems::PrepareBindGroups),
                        queue_grass_instanced.in_set(RenderSystems::QueueMeshes),
                    ),
                );
        }
        // No RenderApp (e.g. unit-test with MinimalPlugins):
        //   - ExtractComponentPlugin is NOT added (avoids SyncComponentPlugin panic)
        //   - GrassRenderWorldAvailable is absent (batch-building skips Mesh3d)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::world;

    /// `GrassInstanceGpu` must be exactly 48 bytes so `bytemuck::cast_slice`
    /// produces the correct GPU vertex buffer layout.
    #[test]
    fn test_grass_instance_gpu_size_is_48_bytes() {
        assert_eq!(
            std::mem::size_of::<GrassInstanceGpu>(),
            48,
            "GrassInstanceGpu must be 48 bytes to match the vertex buffer stride"
        );
    }

    /// `GrassInstanceGpu` must be aligned to 4 bytes (the minimum for GPU
    /// vertex attribute reads).
    #[test]
    fn test_grass_instance_gpu_alignment() {
        assert!(
            std::mem::align_of::<GrassInstanceGpu>() >= 4,
            "GrassInstanceGpu must be at least 4-byte aligned"
        );
    }

    /// Converting from `world::InstanceData` must preserve position, scale,
    /// and rotation_y, and produce a valid phase.
    #[test]
    fn test_grass_instance_gpu_from_instance_data() {
        let data = world::InstanceData {
            position: [3.0, 0.0, 7.0],
            scale: 1.25,
            rotation_y: 0.5,
        };
        let gpu = GrassInstanceGpu::from(&data);
        assert_eq!(gpu.position, [3.0, 0.0, 7.0]);
        assert_eq!(gpu.scale, 1.25);
        assert_eq!(gpu.rotation_y, 0.5);
        // phase is a derived value; we only check it's finite
        assert!(gpu.phase.is_finite(), "phase must be finite");
        // normal defaults to up
        assert_eq!(gpu.normal, [0.0, 1.0, 0.0]);
    }

    /// Two identical `InstanceData` values must produce the same phase (
    /// the derivation is deterministic).
    #[test]
    fn test_grass_instance_gpu_phase_is_deterministic() {
        let data = world::InstanceData {
            position: [5.0, 0.0, 9.0],
            scale: 1.0,
            rotation_y: 0.0,
        };
        let a = GrassInstanceGpu::from(&data);
        let b = GrassInstanceGpu::from(&data);
        assert_eq!(a.phase, b.phase, "phase derivation must be deterministic");
    }

    /// Two clumps at different positions must produce different phases so that
    /// adjacent blades don't sway in synchrony.
    #[test]
    fn test_grass_instance_gpu_phase_varies_with_position() {
        let a = GrassInstanceGpu::from(&world::InstanceData {
            position: [1.0, 0.0, 1.0],
            scale: 1.0,
            rotation_y: 0.0,
        });
        let b = GrassInstanceGpu::from(&world::InstanceData {
            position: [2.0, 0.0, 3.0],
            scale: 1.0,
            rotation_y: 0.0,
        });
        assert_ne!(
            a.phase, b.phase,
            "different positions should produce different wind phases"
        );
    }

    /// Default [`GrassRenderMode`] must be [`GrassRenderMode::Instanced`].
    #[test]
    fn test_grass_render_mode_default_is_instanced() {
        assert_eq!(GrassRenderMode::default(), GrassRenderMode::Instanced);
    }

    /// `GRASS_INSTANCE_STRIDE` must equal the size of one `GrassInstanceGpu`.
    #[test]
    fn test_grass_instance_stride_matches_struct_size() {
        assert_eq!(
            GRASS_INSTANCE_STRIDE as usize,
            std::mem::size_of::<GrassInstanceGpu>(),
            "GRASS_INSTANCE_STRIDE must equal size_of::<GrassInstanceGpu>()"
        );
    }

    /// `GRASS_INSTANCE_ATTR_OFFSET` must be high enough that no instance
    /// attribute location conflicts with the mesh's vertex attributes (0–7).
    #[test]
    fn test_grass_instance_attr_offset_does_not_collide() {
        // Validate at compile time.
        const _: () = assert!(
            crate::game::systems::grass_instancing::GRASS_INSTANCE_ATTR_OFFSET >= 8,
            "instance attributes must start at location >= 8",
        );
    }

    /// `bytemuck::cast_slice` must succeed on a slice of `GrassInstanceGpu`.
    #[test]
    fn test_grass_instance_gpu_bytemuck_cast_slice_succeeds() {
        let instances = vec![
            GrassInstanceGpu {
                position: [1.0, 0.0, 2.0],
                phase: 0.5,
                normal: [0.0, 1.0, 0.0],
                scale: 1.0,
                rotation_y: 0.0,
                _pad: [0.0; 3],
            },
            GrassInstanceGpu {
                position: [3.0, 0.0, 4.0],
                phase: 1.0,
                normal: [0.0, 1.0, 0.0],
                scale: 1.2,
                rotation_y: 1.57,
                _pad: [0.0; 3],
            },
        ];
        let bytes: &[u8] = bytemuck::cast_slice(instances.as_slice());
        assert_eq!(bytes.len(), 2 * 48, "two instances should be 96 bytes");
    }
}
