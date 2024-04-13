//! Systems to spawn and manage cameras in game

use crate::core::graphics::camera::components::{Camera, CameraClearBehaviour, CameraRenderTarget};
use crate::core::window::components::PrimaryWindow;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{Query, With};
use bevy_ecs::system::Commands;
use wgpu::Color;

pub fn s_spawn_camera(mut commands: Commands, query: Query<Entity, With<PrimaryWindow>>) {
    if let Ok(entity) = query.get_single() {
        commands.spawn(Camera {
            render_target: CameraRenderTarget::Window(entity),
            clear_behaviour: CameraClearBehaviour::Color(Color::BLUE),
        });
    }
}
