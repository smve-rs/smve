use bevy_ecs::prelude::*;

/// This event is only emitted when a window receives a `CloseRequested` event.
/// This may be from a user clicking the close button.
#[derive(Event)]
pub struct CloseRequestedEvent {
    pub window_id: winit::window::WindowId,
}

#[derive(Event)]
pub struct WindowResizedEvent {
    pub window_id: winit::window::WindowId,
    pub new_inner_size: winit::dpi::PhysicalSize<u32>,
}

#[derive(Event)]
pub struct WindowCreatedEvent {
    pub window_id: winit::window::WindowId,
}
