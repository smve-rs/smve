//! Contains the components related to the camera

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::Component;
use macros::ExtractComponent;
use wgpu::Color;

/// A component representing a camera and its settings.
///
/// Not exhaustive at the moment, but it will be expanded with more fields later on.
// TODO: Derive default and make primary window the default render target
#[derive(Component, Clone, ExtractComponent)]
pub struct Camera {
    /// Where the camera renders to
    ///
    /// Only supports rendering to windows for now, but will eventually support rendering to
    /// textures.
    ///
    /// # See Also
    /// [`CameraRenderTarget`]
    pub render_target: CameraRenderTarget,
    /// How the camera should clear
    ///
    /// # See Also
    /// [`CameraClearBehaviour`]
    pub clear_behaviour: CameraClearBehaviour,
}

/// Where a camera renders to.
///
/// Will eventually support rendering to textures.
#[non_exhaustive]
#[derive(Clone, Default)]
pub enum CameraRenderTarget {
    /// Rendering to a window
    Window(Entity),
    /// Ignores the camera when rendering
    #[default]
    None,
}

/// How a camera clears the render target.
#[derive(Clone)]
pub enum CameraClearBehaviour {
    /// Do not clear the target at the start of the frame
    DontClear,
    /// Clears the target with the supplied color
    Color(Color),
}

impl Default for CameraClearBehaviour {
    fn default() -> Self {
        CameraClearBehaviour::Color(Color::BLACK)
    }
}
