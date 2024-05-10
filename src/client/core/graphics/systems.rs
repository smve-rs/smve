//! Bevy systems for the graphics module.

use crate::client::core::graphics::resources::{ExtractedWindows, GraphicsState};
use crate::client::core::graphics::ExtractSchedule;
use bevy_ecs::prelude::{Res, Schedules, World};
use bevy_ecs::system::ResMut;
use bevy_ecs::world::Mut;
use cfg_if::cfg_if;
use std::ops::DerefMut;
use tracing::info;
use wgpu::PresentMode;
use winit::dpi::PhysicalSize;

cfg_if! {
    if #[cfg(any(target_os = "macos", target_os = "ios"))] {
        use crate::client::core::graphics::resources::NonSendMarker;
        use bevy_ecs::system::NonSend;
    }
}

/// Creates a surface for each window created.
///
/// Runs on `Prepare`
/// use the [`GraphicsState`] to create a surface for the window, passing in the window and raw handle.
// * Fun Fact: Regarding error handling, I eventually settled on only panicking in systems and never panic in helper functions.
// *           I don't know why I did that since no matter where it panics, if it does panic the program will crash.
pub fn rp_configure_surfaces(
    // On macOS windowing operations can only happen on the main thread
    #[cfg(any(target_os = "macos", target_os = "ios"))] _non_send: Option<NonSend<NonSendMarker>>,
    mut graphics_state: ResMut<GraphicsState<'static>>,
    extracted_windows: Res<ExtractedWindows>,
) {
    for (entity, window) in extracted_windows.iter() {
        if !graphics_state.surface_states.contains_key(entity) {
            graphics_state
                .create_surface(window, *entity, &window.raw_handles)
                .unwrap_or_else(|err| {
                    panic!(
                        "Failed to create surface for window on {:?} with error {err}",
                        entity
                    )
                });

            info!("Surface created for window on {:?}", entity);
        }

        let graphics_state = graphics_state.deref_mut();
        let surface_state = graphics_state
            .surface_states
            .get_mut(entity)
            .expect("Surface state should be created above.");

        if window.size_changed {
            surface_state.resize(
                PhysicalSize::new(window.physical_width, window.physical_height),
                &graphics_state.device,
            );
        }

        if window.present_mode_changed {
            surface_state.config.present_mode = match window.vsync {
                true => PresentMode::AutoVsync,
                false => PresentMode::AutoNoVsync,
            };
            surface_state
                .surface
                .configure(&graphics_state.device, &surface_state.config);
        }
    }
}

/// Run condition for [`rp_configure_surfaces`]
///
/// This solves a deadlock occurring because [`rp_configure_surfaces`] tries to run on the main thread
/// while it is blocking.
pub fn cond_surface_needs_configuration(
    graphics_state: Res<GraphicsState<'static>>,
    extracted_windows: Res<ExtractedWindows>,
) -> bool {
    for (entity, window) in extracted_windows.iter() {
        if !graphics_state.surface_states.contains_key(entity)
            || window.size_changed
            || window.present_mode_changed
        {
            return true;
        }
    }

    false
}

/// Applies commands added from the extract schedule
///
/// Called on `ExtractCommands` to allow it to run in parallel with the main world
pub fn rec_apply_commands(render_world: &mut World) {
    render_world.resource_scope(|render_world, mut schedules: Mut<Schedules>| {
        schedules
            .get_mut(ExtractSchedule)
            .expect("ExtractSchedule should exist")
            .apply_deferred(render_world);
    });
}
