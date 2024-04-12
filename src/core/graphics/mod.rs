//! Rendering code for the engine.
//!
//! This module contains the [`GraphicsPlugin`] which is responsible for initializing rendering with [`wgpu`](https://docs.rs/wgpu/latest/wgpu/index.html).

use crate::core::graphics::resources::GraphicsState;
use crate::core::graphics::systems::{u_create_surface, u_destroy_surface, u_resize};
use crate::core::window::WindowPlugin;
use bevy_app::{App, Plugin, Update};

mod adapter_selection_utils;
pub mod resources;
mod systems;

/// Responsible for initializing rendering with wgpu.
///
/// This plugin initializes the graphics state and adds the necessary systems to create and destroy surfaces.
///
/// # Notes
/// This plugin is added automatically when using the [`WindowPlugin`](crate::core::window::WindowPlugin).
///
/// # Examples
///
/// The following App will initialize some wgpu objects but will not create any surfaces.
/// ```rust
/// App::new().add_plugin(GraphicsPlugin).run();
/// ```
///
/// To use the plugin with the [`WindowPlugin`](crate::core::window::WindowPlugin) you can do the following:
/// ```rust
/// App::new().add_plugins(WindowPlugin::default()).run();
/// ```
/// This creates a primary window with default settings, initializes the graphics state and creates a surface for the primary window.
pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WindowPlugin::default());

        // TODO: Perhaps do this asynchronously instead of blocking?
        // By implementing ready() to check if the async process is done.
        app.insert_resource(pollster::block_on(GraphicsState::new()));
        app.add_systems(Update, u_create_surface);
        app.add_systems(Update, u_resize);
        app.add_systems(Update, u_destroy_surface);
    }
}
