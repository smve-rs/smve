//! Components for rendering

use bevy_ecs::prelude::Component;
use wgpu::SurfaceTexture;

/// Wrapper around surface texture
/// stores an [`Option`] because we will take out the surface texture value when we present it
#[derive(Component)]
pub struct SurfaceTextureComponent(pub Option<SurfaceTexture>);
