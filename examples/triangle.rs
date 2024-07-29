//! A simple application that uses the triangle renderer to render a triangle.

use bevy::math::Vec3A;
use bevy::prelude::*;
use bevy::render::primitives::Aabb;
use bevy::window::WindowResolution;
use smve::render::components::Triangle;
use smve::render::RenderPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "SMve".into(),
                resolution: WindowResolution::new(800.0, 600.0),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.0, 0.6328125, 0.92578125)))
        .add_plugins(RenderPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands
        .spawn(SpatialBundle {
            visibility: Visibility::Visible,
            transform: Transform::IDENTITY,
            ..default()
        })
        .insert(Aabb {
            center: Vec3A::ZERO,
            half_extents: Vec3A::splat(0.5),
        })
        .insert(Triangle);

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
