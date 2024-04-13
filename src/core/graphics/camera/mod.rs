//! Contains camera related functionality including the [`CameraPlugin`]

pub mod components;

use bevy_app::{App, Plugin};

/// Plugin containing functionality to do with a camera.
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, _app: &mut App) {}
}
