use bevy_ecs::prelude::*;

#[derive(Event)]
pub struct CloseRequestedEvent {
    pub window_id: winit::window::WindowId,
}
