use bevy_ecs::event::EventReader;
use bevy_ecs::system::{NonSend, NonSendMut, Query};
use log::info;
use crate::core::graphics::resources::GraphicsState;
use crate::core::window::components::Window;
use crate::core::window::events::{CloseRequestedEvent, WindowCreatedEvent, WindowResizedEvent};
use crate::core::window::resources::WinitWindows;

pub fn u_create_surface(
    mut window_created_event: EventReader<WindowCreatedEvent>,
    winit_windows: NonSend<WinitWindows>,
    mut graphics_state: NonSendMut<GraphicsState>,
    query: Query<&Window>,
) {
    for event in window_created_event.read() {
        let window = winit_windows.windows[&event.window_id].clone();
        let window_entity = winit_windows.window_to_entity[&event.window_id];
        let window_component = query.get(window_entity)
            .unwrap_or_else(|_| panic!("No Window component found on entity {:?}!", window_entity));
        pollster::block_on(graphics_state.create_surface(window, window_component));
        info!("Surface created for window on {:?}", window_entity);
    }
}

pub fn u_resize(
    mut window_resized_event: EventReader<WindowResizedEvent>,
    mut graphics_state: NonSendMut<GraphicsState>,
) {
    for event in window_resized_event.read() {
        let graphics_state = &mut *graphics_state;
        let surface_state = graphics_state.surface_states.get_mut(&event.window_id);
        if let Some(surface_state) = surface_state {
            surface_state.resize(event.new_inner_size, &graphics_state.device);
        }
    }
}

pub fn u_destroy_surface(
    mut close_requested_event: EventReader<CloseRequestedEvent>,
    mut graphics_state: NonSendMut<GraphicsState>
) {
    for event in close_requested_event.read() {
        graphics_state.destroy_surface(event.window_id);
    }
}