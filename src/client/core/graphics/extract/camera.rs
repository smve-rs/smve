//! Responsible for extracting the camera into the render world

use crate::client::core::graphics::camera::components::Camera;
use crate::client::core::graphics::extract::utils::extract_component::ExtractComponentPlugin;
use bevy_app::{App, Plugin};

/// Extracts Cameras into the render world
pub struct CameraExtractPlugin;

impl Plugin for CameraExtractPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<Camera>::default());
    }
}
