//! Resources for rendering

use std::ops::{Deref, DerefMut};

use bevy_ecs::system::Resource;
use wgpu::CommandEncoder;

/// Wraps around the command encoder
#[derive(Resource)]
pub struct CommandEncoderWrapper(pub CommandEncoder);

impl Deref for CommandEncoderWrapper {
    type Target = CommandEncoder;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CommandEncoderWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
