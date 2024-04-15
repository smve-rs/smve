use bevy_ecs::prelude::Component;
use wgpu::SurfaceTexture;

#[derive(Component)]
pub struct SurfaceTextureComponent(pub Option<SurfaceTexture>);
