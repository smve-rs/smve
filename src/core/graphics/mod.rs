//! Rendering code for the engine.
//!
//! This module contains the [`GraphicsPlugin`] which is responsible for initializing rendering with [`wgpu`](https://docs.rs/wgpu/latest/wgpu/index.html).

use crate::core::graphics::resources::GraphicsState;
use crate::core::graphics::systems::{u_create_surface, u_destroy_surface, u_resize};
use crate::core::window::WindowPlugin;
use bevy_app::{App, Plugin, Update};

mod adapter_selection_utils;
pub mod camera;
pub mod resources;
mod systems;

/// Responsible for initializing rendering with wgpu.
///
/// This plugin initializes the graphics state and adds the necessary systems to create and destroy surfaces.
///
/// # Examples
///
/// * Creates a primary window with default settings, initializes the graphics state and creates a surface for the primary window.
/// ```rust
/// App::new().add_plugin(GraphicsPlugin).run();
/// ```
/// * Applies custom parameters to the WindowPlugin.
/// ```rust
/// App::new()
///     .add_plugins((
///         WindowPlugin {
///             primary_window: Some(Window {
///                 title: "New Title".to_string(),
///                 ..Default::default()
///             }),
///             ..Default::default()
///         },
///         GraphicsPlugin,
///     ))
///     .run();
/// ```
///
pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WindowPlugin>() {
            app.add_plugins(WindowPlugin::default());
        }

        app.insert_resource(pollster::block_on(GraphicsState::new()));
        app.add_systems(Update, u_create_surface);
        app.add_systems(Update, u_resize);
        app.add_systems(Update, u_destroy_surface);
    }
}
