use bevy_ecs::prelude::*;

/// This event is only emitted when a window receives a `CloseRequested` event.
/// This may be from a user clicking the close button.
#[derive(Event)]
pub struct CloseRequestedEvent {
    pub window_id: winit::window::WindowId,
}
