//! Systems to spawn and manage cameras in game

use crate::core::graphics::camera::components::{Camera, CameraClearBehaviour};
use bevy_ecs::system::Commands;
use wgpu::Color;

pub fn s_spawn_camera(mut commands: Commands) {
    commands.spawn(Camera {
        clear_behaviour: CameraClearBehaviour::Color(Color {
            // Windows blue
            r: 0.0,
            g: 0.6328125,
            b: 0.92578125,
            a: 1.0
        }),
        ..Default::default()
    });
}
