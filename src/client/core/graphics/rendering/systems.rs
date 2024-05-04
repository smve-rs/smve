//! Contains wgpu code for rendering

use std::ops::DerefMut;

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::Query;
use bevy_ecs::system::{Commands, Res, ResMut, SystemState};
use bevy_ecs::world::World;
use log::{error, warn};
use wgpu::{CommandEncoderDescriptor, SurfaceError};

use crate::client::core::graphics::camera::components::Camera;
use crate::client::core::graphics::rendering::components::SurfaceTextureComponent;
use crate::client::core::graphics::rendering::resources::CommandEncoderWrapper;
use crate::client::core::graphics::rendering::utils::begin_render_pass;
use crate::client::core::graphics::resources::{ExtractedWindows, GraphicsState};

/// Begins the render pass through the command encoder
///
/// Called on `PreQueue`
pub fn rpq_begin_render_passes(
    cameras: Query<(Entity, &Camera)>,
    extracted_windows: Res<ExtractedWindows>,
    mut graphics_state: ResMut<GraphicsState<'static>>,
    mut command_encoder: ResMut<CommandEncoderWrapper>,
    mut commands: Commands,
) {
    for (entity, camera) in cameras.iter() {
        let Some(render_window) = camera
            .render_target
            .get_window_entity(extracted_windows.primary)
        else {
            continue;
        };

        let graphics_state = graphics_state.deref_mut();

        if let Some(surface_state) = graphics_state.surface_states.get_mut(&render_window) {
            match begin_render_pass(
                format!("{render_window:?}").as_str(),
                &surface_state.surface,
                command_encoder.deref_mut(),
                &camera.clear_behaviour,
            ) {
                Ok(surface_texture) => {
                    commands
                        .entity(entity)
                        .insert(SurfaceTextureComponent(Some(surface_texture)));
                }
                Err(SurfaceError::Lost) => {
                    surface_state.resize(surface_state.size, &graphics_state.device);
                }
                Err(SurfaceError::OutOfMemory) => {
                    panic!("Out of memory!");
                }
                Err(e) => {
                    error!("Surface error! {}", e);
                }
            }
        } else {
            warn!(
                "No surface associated with window {render_window:?}, skipping camera {entity:?}"
            );
        }
    }
}

/// Creates the command encoder
///
/// Called on `Prepare`
pub fn rp_create_command_encoder(
    graphics_state: Res<GraphicsState<'static>>,
    mut commands: Commands,
) {
    let encoder = graphics_state
        .device
        .create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

    commands.insert_resource(CommandEncoderWrapper(encoder));
}

/// Submits the command buffer
///
/// Called on `FinishQueue`
pub fn rfq_finish_queue(world: &mut World, params: &mut SystemState<Res<GraphicsState<'static>>>) {
    let command_encoder = world
        .remove_resource::<CommandEncoderWrapper>()
        .expect("Command encoder should exist");
    params
        .get(world)
        .queue
        .submit(std::iter::once(command_encoder.0.finish()));
    params.apply(world);
}

/// Presents the surface texture
///
/// Called on `Render`
pub fn rr_render(mut query: Query<&mut SurfaceTextureComponent>) {
    for mut output in query.iter_mut() {
        let output = std::mem::take(&mut output.0).expect("Surface texture should not be None");
        output.present();
    }
}
