//! This crate contains all voxel rendering code. It does NOT contain any representations of
//! voxels like Chunks, allowing for the usage of a different rendering engine.

pub mod components;
mod render;

use crate::components::Triangle;
use crate::render::draw::{
    prepare_triangle_phase_item_buffers, queue_triangle_phase_item, DrawTriangleCommands,
    TrianglePipeline, WithTriangle,
};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_core_pipeline::core_3d::Opaque3d;
use bevy_ecs::schedule::IntoSystemConfigs;
use bevy_render::extract_component::ExtractComponentPlugin;
use bevy_render::render_phase::AddRenderCommand;
use bevy_render::render_resource::SpecializedRenderPipelines;
use bevy_render::view::VisibilitySystems;
use bevy_render::{view, Render, RenderApp, RenderSet};

/// This plugin contains logic to do with rendering voxels.
///
/// TODO: Improve documentation once functionality is semi-complete.
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<Triangle>::default())
            .add_systems(
                PostUpdate,
                view::check_visibility::<WithTriangle>.in_set(VisibilitySystems::CheckVisibility),
            );

        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<TrianglePipeline>()
            .init_resource::<SpecializedRenderPipelines<TrianglePipeline>>()
            .add_render_command::<Opaque3d, DrawTriangleCommands>()
            .add_systems(
                Render,
                prepare_triangle_phase_item_buffers.in_set(RenderSet::Prepare),
            )
            .add_systems(Render, queue_triangle_phase_item.in_set(RenderSet::Queue));
    }
}
