use crate::components::Triangle;
use bevy_asset::{AssetId, AssetServer, Handle};
use bevy_core_pipeline::core_3d::{Opaque3d, Opaque3dBinKey, CORE_3D_DEPTH_FORMAT};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::World;
use bevy_ecs::query::{ROQueryItem, With};
use bevy_ecs::system::lifetimeless::SRes;
use bevy_ecs::system::{Commands, Query, Res, ResMut, Resource, SystemParamItem};
use bevy_ecs::world::FromWorld;
use bevy_math::{vec3, Vec3};
use bevy_render::mesh::Mesh;
use bevy_render::prelude::Shader;
use bevy_render::render_phase::{
    BinnedRenderPhaseType, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
    SetItemPipeline, TrackedRenderPass, ViewBinnedRenderPhases,
};
use bevy_render::render_resource::{
    BufferUsages, ColorTargetState, ColorWrites, CompareFunction, DepthStencilState, FragmentState,
    IndexFormat, MultisampleState, PipelineCache, RawBufferVec, RenderPipelineDescriptor,
    SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};
use bevy_render::renderer::{RenderDevice, RenderQueue};
use bevy_render::texture::BevyDefault;
use bevy_render::view::{ExtractedView, Msaa, VisibleEntities};
use bytemuck::{Pod, Zeroable};

#[derive(Resource)]
pub struct TrianglePipeline {
    shader: Handle<Shader>,
}

pub struct DrawTrianglePhaseItem;

impl<P> RenderCommand<P> for DrawTrianglePhaseItem
where
    P: PhaseItem,
{
    type Param = SRes<TrianglePhaseItemBuffers>;
    type ViewQuery = ();
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, Self::ItemQuery>>,
        param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let param = param.into_inner();

        pass.set_vertex_buffer(0, param.vertices.buffer().unwrap().slice(..));

        pass.set_index_buffer(
            param.indices.buffer().unwrap().slice(..),
            0,
            IndexFormat::Uint32,
        );

        pass.draw_indexed(0..3, 0, 0..1);

        RenderCommandResult::Success
    }
}

#[derive(Resource)]
pub struct TrianglePhaseItemBuffers {
    vertices: RawBufferVec<Vertex>,
    indices: RawBufferVec<u32>,
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Vertex {
    position: Vec3,
    pad0: u32, // Padding
    color: Vec3,
    pad1: u32, // Padding
}

impl Vertex {
    const fn new(position: Vec3, color: Vec3) -> Vertex {
        Vertex {
            position,
            color,
            pad0: 0,
            pad1: 0,
        }
    }
}

pub type DrawTriangleCommands = (SetItemPipeline, DrawTrianglePhaseItem);

pub type WithTriangle = With<Triangle>;

static VERTICES: [Vertex; 3] = [
    Vertex::new(vec3(-0.5, -0.5, 0.5), vec3(1.0, 0.0, 0.0)),
    Vertex::new(vec3(0.5, -0.5, 0.5), vec3(0.0, 1.0, 0.0)),
    Vertex::new(vec3(0.0, 0.5, 0.5), vec3(0.0, 0.0, 1.0)),
];

pub fn prepare_triangle_phase_item_buffers(mut commands: Commands<'_, '_>) {
    commands.init_resource::<TrianglePhaseItemBuffers>();
}

pub fn queue_triangle_phase_item(
    pipeline_cache: Res<'_, PipelineCache>,
    custom_phase_pipeline: Res<'_, TrianglePipeline>,
    msaa: Res<'_, Msaa>,
    mut opaque_render_phases: ResMut<'_, ViewBinnedRenderPhases<Opaque3d>>,
    opaque_draw_functions: Res<'_, DrawFunctions<Opaque3d>>,
    mut specialized_render_pipelines: ResMut<'_, SpecializedRenderPipelines<TrianglePipeline>>,
    views: Query<'_, '_, (Entity, &VisibleEntities), With<ExtractedView>>,
) {
    let draw_triangle_phase_item = opaque_draw_functions.read().id::<DrawTriangleCommands>();

    for (view_entity, view_visible_entities) in views.iter() {
        let Some(opaque_phase) = opaque_render_phases.get_mut(&view_entity) else {
            continue;
        };

        for &entity in view_visible_entities.get::<WithTriangle>().iter() {
            let pipeline_id = specialized_render_pipelines.specialize(
                &pipeline_cache,
                &custom_phase_pipeline,
                *msaa,
            );

            opaque_phase.add(
                Opaque3dBinKey {
                    draw_function: draw_triangle_phase_item,
                    pipeline: pipeline_id,
                    asset_id: AssetId::<Mesh>::invalid().untyped(),
                    material_bind_group_id: None,
                    lightmap_image: None,
                },
                entity,
                BinnedRenderPhaseType::NonMesh,
            );
        }
    }
}

impl FromWorld for TrianglePhaseItemBuffers {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();

        let mut vbo = RawBufferVec::new(BufferUsages::VERTEX);
        let mut ibo = RawBufferVec::new(BufferUsages::INDEX);

        for vertex in &VERTICES {
            vbo.push(*vertex);
        }
        for index in 0..3 {
            ibo.push(index);
        }

        vbo.write_buffer(render_device, render_queue);
        ibo.write_buffer(render_device, render_queue);

        TrianglePhaseItemBuffers {
            vertices: vbo,
            indices: ibo,
        }
    }
}

impl FromWorld for TrianglePipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource_mut::<AssetServer>();

        let handle = asset_server.add(Shader::from_wgsl(include_str!("triangle.wgsl"), file!()));

        TrianglePipeline { shader: handle }
    }
}

impl SpecializedRenderPipeline for TrianglePipeline {
    type Key = Msaa;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("Triangle Render Pipeline".into()),
            layout: vec![],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: self.shader.clone(),

                shader_defs: vec![],
                entry_point: "vertex".into(),
                buffers: vec![VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 16,
                            shader_location: 1,
                        },
                    ],
                }],
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: MultisampleState {
                count: key.samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        }
    }
}
