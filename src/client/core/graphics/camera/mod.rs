//! Contains camera related functionality including the [`CameraPlugin`]

use bevy_app::{App, Plugin};

pub mod components;

/// Plugin containing functionality to do with a camera.
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, _app: &mut App) {}
}
