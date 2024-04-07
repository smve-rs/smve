//! Bevy events for windowing.

use bevy_ecs::prelude::*;

/// This event is only emitted when a window receives a `CloseRequested` event.
/// This may be from a user clicking the close button.
#[derive(Event)]
pub struct CloseRequestedEvent {
    /// The window that received the close request
    pub window_id: winit::window::WindowId,
}

/// This event is emitted when a window is resized.
#[derive(Event)]
pub struct WindowResizedEvent {
    /// The entity with the window that was resized
    pub entity: Entity,
    /// The new logical width of the window
    pub new_width: f64,
    /// The new logical height of the window
    pub new_height: f64,
}

/// This event is emitted when a window is created.
#[derive(Event)]
pub struct WindowCreatedEvent {
    /// The window that was created
    pub window_id: winit::window::WindowId,
}
