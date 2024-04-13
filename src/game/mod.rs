//! This is the plugin that contains all code related to the game.
//! It includes things such as cameras for now.

mod camera;

use crate::core::graphics::camera::CameraPlugin;
use crate::game::camera::systems::s_spawn_camera;
use bevy_app::{App, Plugin, Startup};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<CameraPlugin>() {
            app.add_plugins(CameraPlugin);
        }

        app.add_systems(Startup, s_spawn_camera);
    }
}
