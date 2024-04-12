//! Bevy systems for the graphics module.

use crate::core::graphics::resources::GraphicsState;
use crate::core::window::components::{RawHandleWrapper, Window};
use crate::core::window::events::{CloseRequestedEvent, WindowCreatedEvent, WindowResizedEvent};
use crate::core::window::resources::WinitWindows;
use bevy_ecs::event::EventReader;
use bevy_ecs::system::{NonSend, Query, ResMut};
use log::info;

/// Creates a surface for each window created.
///
/// Runs on `Update` when a [`WindowCreatedEvent`] is received,
/// use the [`GraphicsState`] to create a surface for the window, passing in the window and raw handle.
// * Fun Fact: Regarding error handling, I eventually settled on only panicking in systems and never panic in helper functions.
// *           I don't know why I did that since no matter where it panics, if it does panic the program will crash.
pub fn u_create_surface(
    mut window_created_event: EventReader<WindowCreatedEvent>,
    winit_windows: NonSend<WinitWindows>,
    mut graphics_state: ResMut<GraphicsState<'static>>,
    query: Query<(&Window, &RawHandleWrapper)>,
) {
    for event in window_created_event.read() {
        let window = winit_windows
            .windows
            .get(&event.window_id)
            // * Fun Fact: I used to always use unwrap here. I eventually learned to use unwrap_or_else to provide a custom panic message.
            .unwrap_or_else(|| panic!("Window {:?} not found!", event.window_id));
        let window_entity = winit_windows.window_to_entity[&event.window_id];
        let (window_component, raw_window_handle) = query
            .get(window_entity)
            .unwrap_or_else(|_| panic!("No Window component found on entity {:?}!", window_entity));

        graphics_state
            .create_surface(window, window_component, raw_window_handle)
            .unwrap_or_else(|err| {
                panic!(
                    "Failed to create surface for window on {:?} with error {err}",
                    window_entity
                )
            });

        info!("Surface created for window on {:?}", window_entity);
    }
}

/// Resizes the surface for each window that has a resized event.
///
/// Runs on `Update` when a [`WindowResizedEvent`] is received,
pub fn u_resize(
    mut window_resized_event: EventReader<WindowResizedEvent>,
    winit_windows: NonSend<WinitWindows>,
    query: Query<&Window>,
    mut graphics_state: ResMut<GraphicsState<'static>>,
) {
    for event in window_resized_event.read() {
        let graphics_state = &mut *graphics_state;

        let window_id = winit_windows
            .entity_to_window
            .get(&event.entity)
            .expect("Window should exist");

        let window = query
            .get(event.entity)
            .expect("Window component should exist");

        let surface_state = graphics_state.surface_states.get_mut(window_id);
        if let Some(surface_state) = surface_state {
            surface_state.resize(window.resolution.physical_size(), &graphics_state.device);
        }
    }
}

/// Destroys a surface for each window that has a close requested event.
///
/// Runs on `Update` when a [`CloseRequestedEvent`] is received,
/// use the [`GraphicsState`] to destroy the surface for the window.
pub fn u_destroy_surface(
    mut close_requested_event: EventReader<CloseRequestedEvent>,
    mut graphics_state: ResMut<GraphicsState<'static>>,
) {
    for event in close_requested_event.read() {
        graphics_state.destroy_surface(event.window_id);
    }
}
