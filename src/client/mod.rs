//! This is the plugin that contains all code related to the client.
//! It includes things such as cameras for now.

use bevy_app::{App, Plugin, Startup};

use crate::client::camera::systems::s_spawn_camera;
use crate::client::core::graphics::camera::CameraPlugin;

mod camera;
pub mod core;

/// Plugin that contains everything the game uses.
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<CameraPlugin>() {
            app.add_plugins(CameraPlugin);
        }

        app.add_systems(Startup, s_spawn_camera);
    }
}
